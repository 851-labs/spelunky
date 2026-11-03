//! Reports which objects a set of replays creates, by gameplay ticket: the coverage `fuzz-sweep` reports for
//! its random runs, for any replays (by default every committed one, `fidelity/replays/`). With `--lines`, which
//! lines of the GML they run instead ([`harness::lines`], 851-3775).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};

use harness::fingerprint::Suite;
use harness::fuzz::{Coverage, coverage, object_folders, object_table};
use harness::lines::{LineReport, report as line_report, universe};
use harness::{RunOptions, load_assets, run_replay};
use tools::{Arg, Args, write_stdout};
use trace::Replay;

const USAGE: &str = "\
usage: replay-coverage [options] [<replay.txt>...]

Plays each replay on the port (no oracle) and reports, in Markdown, which objects they created, by
gameplay ticket: active (in or near the view on some frame), only deactivated, or never. Without
replays, it reads every fidelity/replays/*.txt. An object that is created and destroyed within one
step isn't seen, and a level's first frame, before oLevel first deactivates what's out of view,
doesn't count as active.

options:
  --by-object     also list, for each object created and each room visited, the replays that did it,
                  and how often the player died in each replay
  --lines         report GML line coverage instead: of every line of the game's and the settings
                  program's generated code (crates/game/src/generated, crates/config/src/generated), the
                  ones the replays ran, and each unit (event action or script) none of whose lines ran.
                  Without replays, every committed one: fidelity/replays/ and fidelity/settings/
  --missed        with --lines, also list each partly run unit's lines that didn't run, with their GML
  --by-line <u>   with --lines, list the replays that ran each line of the units whose names start with u
                  (oSnake/, scrInit)
  --jobs <n>      replays played at once (default 4)
  --assets <dir>  the asset bundle (default: assets in the repo)
  -h, --help

exit status: 0 every replay ran to its end, 1 some replay stopped on an error, 2 bad usage or I/O";

struct Options {
    replays: Vec<PathBuf>,
    by_object: bool,
    lines: bool,
    missed: bool,
    by_line: Option<String>,
    jobs: usize,
    assets: PathBuf,
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Option<Options>, String> {
    let mut o = Options {
        replays: Vec::new(),
        by_object: false,
        lines: false,
        missed: false,
        by_line: None,
        jobs: 4,
        assets: repo_root().join("assets"),
    };
    let mut args = Args::new(args);
    while let Some(arg) = args.next() {
        let flag = match arg {
            Arg::Positional(p) => {
                o.replays.push(PathBuf::from(p));
                continue;
            }
            Arg::Flag(flag) => flag,
        };
        match flag.name.as_str() {
            "-h" | "--help" => {
                args.switch(&flag)?;
                return Ok(None);
            }
            "--by-object" => {
                args.switch(&flag)?;
                o.by_object = true;
            }
            "--lines" => {
                args.switch(&flag)?;
                o.lines = true;
            }
            "--missed" => {
                args.switch(&flag)?;
                o.missed = true;
            }
            "--by-line" => o.by_line = Some(args.value(&flag)?),
            "--jobs" => o.jobs = args.number::<usize>(&flag)?.max(1),
            "--assets" => o.assets = args.value(&flag)?.into(),
            f => return Err(format!("unknown option {f}")),
        }
    }
    if (o.missed || o.by_line.is_some()) && !o.lines {
        return Err("--missed and --by-line are for --lines".into());
    }
    Ok(Some(o))
}

/// Every `*.txt` in `dir`, sorted.
fn replays_in(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "txt"))
        .collect();
    out.sort();
    Ok(out)
}

/// The replays' names in the report: their file stems, or their paths where two stems are the same.
fn names(paths: &[PathBuf]) -> Vec<String> {
    let stem = |p: &Path| {
        p.file_stem().map_or_else(
            || p.display().to_string(),
            |s| s.to_string_lossy().into_owned(),
        )
    };
    paths
        .iter()
        .map(|p| {
            let s = stem(p);
            if paths.iter().filter(|q| stem(q) == s).count() > 1 {
                p.display().to_string()
            } else {
                s
            }
        })
        .collect()
}

/// The report: totals, any errors, the table, and with `by_object` each object's replays.
fn report(
    runs: &[(String, Coverage)],
    folders: &BTreeMap<String, String>,
    by_object: bool,
) -> String {
    let mut out = String::new();
    let frames: u64 = runs.iter().map(|(_, c)| c.frames).sum();
    let _ = writeln!(out, "{} replays, {frames} frames.\n", runs.len());
    for (n, c) in runs {
        if let Some(e) = &c.error {
            let _ = writeln!(out, "- `{n}` stopped: {e}");
        }
    }
    if runs.iter().any(|(_, c)| c.error.is_some()) {
        out.push('\n');
    }
    let active: BTreeSet<&str> = runs
        .iter()
        .flat_map(|(_, c)| c.active.iter().map(String::as_str))
        .collect();
    let existed: BTreeSet<&str> = runs
        .iter()
        .flat_map(|(_, c)| c.existed.iter().map(String::as_str))
        .collect();
    out.push_str(&object_table(&active, &existed, folders));
    if by_object {
        let _ = writeln!(out, "\nThe replays that created each object:\n");
        for o in &existed {
            let names: Vec<&str> = runs
                .iter()
                .filter(|(_, c)| c.existed.contains(*o))
                .map(|(n, _)| n.as_str())
                .collect();
            let _ = writeln!(out, "- {o}: {}", names.join(" "));
        }
        let rooms: BTreeSet<&str> = runs
            .iter()
            .flat_map(|(_, c)| c.rooms.iter().map(String::as_str))
            .collect();
        let _ = writeln!(out, "\nThe replays that visited each room:\n");
        for r in rooms {
            let names: Vec<&str> = runs
                .iter()
                .filter(|(_, c)| c.rooms.contains(r))
                .map(|(n, _)| n.as_str())
                .collect();
            let _ = writeln!(out, "- {r}: {}", names.join(" "));
        }
        let deaths: Vec<String> = runs
            .iter()
            .filter(|(_, c)| c.deaths > 0)
            .map(|(n, c)| format!("{n} ({})", c.deaths))
            .collect();
        let _ = writeln!(
            out,
            "\nThe replays in which the player died (deaths): {}",
            deaths.join(" ")
        );
    }
    out
}

fn run() -> Result<bool, String> {
    let Some(mut o) =
        parse_args(std::env::args().skip(1)).map_err(|e| format!("{e}\n\n{USAGE}"))?
    else {
        write_stdout(&format!("{USAGE}\n"))?;
        return Ok(true);
    };
    if o.lines {
        return lines(&o);
    }
    if o.replays.is_empty() {
        o.replays = replays_in(&repo_root().join("fidelity/replays"))?;
    }
    let replays = o
        .replays
        .iter()
        .zip(names(&o.replays))
        .map(|(p, name)| {
            let text = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
            let replay = Replay::parse(&text).map_err(|e| format!("{}: {e}", p.display()))?;
            Ok((name, replay))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let assets = load_assets(&o.assets)?;
    let next = AtomicUsize::new(0);
    let runs = Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..o.jobs.min(replays.len()) {
            s.spawn(|| {
                while let Some((n, replay)) = replays.get(next.fetch_add(1, Ordering::Relaxed)) {
                    let c = coverage(replay, assets.clone());
                    // A panicking replay mustn't lose the others' results.
                    let mut runs = runs.lock().unwrap_or_else(PoisonError::into_inner);
                    eprintln!("[{}/{}] {n}", runs.len() + 1, replays.len());
                    runs.push((n.clone(), c));
                }
            });
        }
    });
    let mut runs = runs.into_inner().unwrap_or_else(PoisonError::into_inner);
    runs.sort_by(|a, b| a.0.cmp(&b.0));
    let inventory = std::fs::read_to_string(repo_root().join("docs/source-inventory.md"))
        .map_err(|e| format!("docs/source-inventory.md: {e}"))?;
    write_stdout(&report(&runs, &object_folders(&inventory), o.by_object))?;
    Ok(runs.iter().all(|(_, c)| c.error.is_none()))
}

/// The lines a replay ran: the game's or (`settings`) the settings program's.
struct LinesOf {
    name: String,
    settings: bool,
    lines: BTreeSet<(String, u32)>,
    error: Option<String>,
}

/// `--lines`: GML line coverage.
fn lines(o: &Options) -> Result<bool, String> {
    let repo = std::fs::canonicalize(repo_root()).map_err(|e| format!("the repository: {e}"))?;
    let paths = if o.replays.is_empty() {
        let mut all = replays_in(&repo.join("fidelity/replays"))?;
        all.extend(replays_in(&repo.join("fidelity/settings"))?);
        all
    } else {
        o.replays.clone()
    };
    let replays = paths
        .iter()
        .zip(names(&paths))
        .map(|(p, name)| {
            let text = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
            let replay = Replay::parse(&text).map_err(|e| format!("{}: {e}", p.display()))?;
            // The settings program's replays are fidelity/settings/'s.
            let settings = Suite::of(&repo, p).is_ok_and(|s| s.settings);
            Ok((name, settings, replay))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let game = load_assets(&o.assets)?;
    let config = if replays.iter().any(|r| r.1) {
        Some(load_assets(&o.assets.join("config"))?)
    } else {
        None
    };
    let next = AtomicUsize::new(0);
    let runs = Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..o.jobs.min(replays.len()) {
            s.spawn(|| {
                while let Some((n, settings, replay)) =
                    replays.get(next.fetch_add(1, Ordering::Relaxed))
                {
                    let assets = if *settings {
                        config.clone().expect("loaded")
                    } else {
                        game.clone()
                    };
                    let opts = RunOptions {
                        settings: *settings,
                        lines: true,
                        ..RunOptions::default()
                    };
                    let (lines, error) = match run_replay(replay, assets, &opts) {
                        Ok(summary) => (
                            (summary.lines.iter().flatten())
                                .map(|&(unit, line)| (unit.to_owned(), line))
                                .collect(),
                            summary.error.map(|e| e.message),
                        ),
                        Err(e) => (BTreeSet::new(), Some(e)),
                    };
                    let mut runs = runs.lock().unwrap_or_else(PoisonError::into_inner);
                    eprintln!("[{}/{}] {n}", runs.len() + 1, replays.len());
                    runs.push(LinesOf {
                        name: n.clone(),
                        settings: *settings,
                        lines,
                        error,
                    });
                }
            });
        }
    });
    let mut runs = runs.into_inner().unwrap_or_else(PoisonError::into_inner);
    runs.sort_by(|a, b| a.name.cmp(&b.name));
    let frames: u64 = replays.iter().map(|r| r.2.len()).sum();
    let mut out = format!("{} replays, {frames} frames.\n", replays.len());
    for (settings, title, dir) in [
        (false, "The game", "crates/game/src/generated"),
        (true, "The settings program", "crates/config/src/generated"),
    ] {
        let mine: Vec<&LinesOf> = runs.iter().filter(|r| r.settings == settings).collect();
        if mine.is_empty() {
            continue;
        }
        let all = universe(&repo.join(dir))?;
        let run: BTreeSet<(String, u32)> =
            mine.iter().flat_map(|r| r.lines.iter().cloned()).collect();
        let r = line_report(&all, &run);
        out += &lines_section(title, &r, o.missed);
        if let Some(prefix) = &o.by_line {
            out += &by_line(&all, &mine, prefix);
        }
    }
    for r in runs.iter().filter(|r| r.error.is_some()) {
        let _ = writeln!(
            out,
            "\n{} stopped: {}",
            r.name,
            r.error.as_deref().unwrap_or_default()
        );
    }
    write_stdout(&out)?;
    Ok(runs.iter().all(|r| r.error.is_none()))
}

/// A program's line coverage, in Markdown.
fn lines_section(title: &str, r: &LineReport, missed: bool) -> String {
    let pct = |a: usize, b: usize| {
        if b == 0 {
            0.0
        } else {
            100.0 * a as f64 / b as f64
        }
    };
    let mut out = format!(
        "\n### {title}\n\n{} of {} GML lines ran ({:.1}%), in {} units (event actions and scripts): {} never ran, {} \
         partly.\n\nNever ran:\n\n",
        r.lines_run,
        r.lines,
        pct(r.lines_run, r.lines),
        r.units,
        r.never.len(),
        r.partly.len()
    );
    for (unit, n) in &r.never {
        let _ = writeln!(
            out,
            "- `{unit}` ({n} line{})",
            if *n == 1 { "" } else { "s" }
        );
    }
    if missed {
        out += "\nPartly ran, with the lines that didn't:\n";
        for (unit, n, lines) in &r.partly {
            let _ = writeln!(out, "\n- `{unit}`: {} of {n} lines ran", n - lines.len());
            for (line, gml) in lines {
                let _ = writeln!(out, "  - {line}: `{}`", gml.replace('`', "'"));
            }
        }
    }
    out
}

/// `--by-line`: for each line of the units whose names start with `prefix`, the replays that ran it.
fn by_line(
    all: &BTreeMap<String, harness::lines::UnitLines>,
    runs: &[&LinesOf],
    prefix: &str,
) -> String {
    let mut out = format!("\nThe replays that ran each line of `{prefix}*`:\n");
    for (unit, lines) in all
        .range(prefix.to_owned()..)
        .take_while(|(u, _)| u.starts_with(prefix))
    {
        let _ = writeln!(out, "\n- `{unit}`");
        for (line, gml) in lines {
            let key = (unit.clone(), *line);
            let who: Vec<&str> = runs
                .iter()
                .filter(|r| r.lines.contains(&key))
                .map(|r| r.name.as_str())
                .collect();
            let who = if who.is_empty() {
                "none".to_owned()
            } else {
                who.join(" ")
            };
            let _ = writeln!(out, "  - {line} `{}`: {who}", gml.replace('`', "'"));
        }
    }
    out
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            eprintln!("replay-coverage: {e}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments() {
        let parse = |a: &[&str]| parse_args(a.iter().map(|s| s.to_string()));
        let o = parse(&["a.txt", "--jobs", "0", "--by-object", "b.txt"])
            .unwrap()
            .unwrap();
        assert_eq!(o.replays, [PathBuf::from("a.txt"), PathBuf::from("b.txt")]);
        assert_eq!((o.jobs, o.by_object), (1, true));
        assert!(parse(&["--help"]).unwrap().is_none());
        assert!(parse(&["--frames", "3"]).is_err());
    }

    #[test]
    fn names_by_stem_unless_they_collide() {
        let paths = ["a/x.txt", "b/x.txt", "c/y.txt"].map(PathBuf::from);
        assert_eq!(names(&paths), ["a/x.txt", "b/x.txt", "y"]);
    }

    #[test]
    fn reports_objects_by_replay() {
        let folders: BTreeMap<String, String> = [("oGold", "Blocks"), ("oHeart", "Effects")]
            .map(|(o, f)| (o.into(), f.into()))
            .into();
        let run = |existed: &[&str], active: &[&str]| Coverage {
            frames: 10,
            existed: existed.iter().map(|s| s.to_string()).collect(),
            active: active.iter().map(|s| s.to_string()).collect(),
            rooms: ["rLevel".to_string()].into(),
            deaths: u32::from(existed.len() == active.len()),
            ..Coverage::default()
        };
        let runs = [
            ("a".to_string(), run(&["oGold"], &[])),
            ("b".to_string(), run(&["oGold"], &["oGold"])),
        ];
        let text = report(&runs, &folders, true);
        assert!(text.starts_with("2 replays, 20 frames."), "{text}");
        assert!(
            text.contains("| 851-3146 | terrain | 1 of 1: oGold |  |  |"),
            "{text}"
        );
        assert!(
            text.contains("| 851-3158 | effects | 0 of 1:  |  | oHeart |"),
            "{text}"
        );
        assert!(text.contains("- oGold: a b\n"), "{text}");
        assert!(text.contains("- rLevel: a b\n"), "{text}");
        assert!(text.contains("died (deaths): b (1)\n"), "{text}");
    }
}
