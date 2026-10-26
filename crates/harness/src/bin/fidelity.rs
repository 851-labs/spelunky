//! Runs a replay on the oracle and on the port and reports the first frame where they differ: the loop
//! every fidelity ticket works in. `scripts/fidelity` runs it.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

use harness::fingerprint::{
    Suite, all_replays, fingerprint_path, fingerprint_trace, port_mismatch, run_fingerprint, shown,
};
use harness::host::{HostReport, diff_host, host_dir};
use harness::{RunOptions, RunSummary, load_assets, run_replay};
use serde::Serialize;
use tools::{Arg, Args, check_diff_options, diff_option, diff_usage, open_trace, write_stdout};
use trace::Replay;
use trace::diff::{DiffOptions, Outcome, Report, diff_traces};

const USAGE: &str = "\
usage: fidelity [options] <replay.txt>
       fidelity --refresh-fingerprints [--jobs <n>] [--rerun-oracle] [--assets <dir>] [<replay.txt>...]

Runs the replay on the oracle (tools/oracle/run) and on the port (like replay-run), then compares the two
traces (like trace-diff, with the oracle as a and the port as b) and reports the first divergence, or
where the port stopped if it stopped before diverging. Then it compares what each run left behind outside
its trace (<trace>.host/): the game directory's new or changed files, byte for byte, and the highscore
table, value for value.

The oracle's trace is cached in the output directory and reused while the replay, --fields and the
oracle build are unchanged, so after the first run only the port runs.

With --refresh-fingerprints, it records the fingerprints of committed replays (by default all of
fidelity/replays/ and fidelity/settings/) from the oracle, with --fields, into fidelity/fingerprints/,
for fidelity-check. A fingerprint is only recorded if the port matches it too; otherwise the old one
stays and the oracle's trace is kept for scripts/fidelity. A trace the oracle made just for this is
deleted afterwards (they take GBs). --jobs runs that many oracles at once (default 6), the longest
replays first.

options:
  --out <dir>            where the traces go (default: target/fidelity/<replay name> in the repo)
  --oracle-trace <file>  compare against this oracle trace instead of running the oracle
  --rerun-oracle         run the oracle even if its cached trace is current
  --fields               dump (and compare) instance variables and globals in both runs
  --assets <dir>         the port's asset bundle (default: assets/ in the repo; assets/config/ with
                         --settings)
  --settings             run the settings program, config.exe, on both instead of the game
  --json                 print one JSON object: verdict, paths, the port's run summary, the diff report
  --refresh-fingerprints record the replays' fingerprints from the oracle (see above)
  --jobs <n>             with --refresh-fingerprints, replays recorded at once (default 6)
  -h, --help

diff options (as trace-diff's; --compare is trace-diff's --fields):
";

const EXIT: &str = "\
exit status: 0 identical, 1 diverged, the port stopped early, or the host files differ, 2 error";

struct Options {
    replay: PathBuf,
    out: Option<PathBuf>,
    oracle_trace: Option<PathBuf>,
    rerun_oracle: bool,
    fields: bool,
    settings: bool,
    assets: Option<PathBuf>,
    json: bool,
    diff: DiffOptions,
    /// `--refresh-fingerprints`, with its replays (none for all of them).
    refresh: Option<Vec<PathBuf>>,
    /// `--jobs`, for `--refresh-fingerprints`.
    jobs: Option<usize>,
}

fn usage() -> String {
    format!("{USAGE}{}\n\n{EXIT}", diff_usage("--compare"))
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Option<Options>, String> {
    let mut a = Options {
        replay: PathBuf::new(),
        out: None,
        oracle_trace: None,
        rerun_oracle: false,
        fields: false,
        settings: false,
        assets: None,
        json: false,
        diff: DiffOptions::default(),
        refresh: None,
        jobs: None,
    };
    let mut replays = Vec::new();
    let mut diff_given = false;
    let mut args = Args::new(args);
    while let Some(arg) = args.next() {
        let flag = match arg {
            Arg::Positional(path) => {
                replays.push(PathBuf::from(path));
                continue;
            }
            Arg::Flag(flag) => flag,
        };
        match flag.name.as_str() {
            "-h"
            | "--help"
            | "--json"
            | "--fields"
            | "--rerun-oracle"
            | "--settings"
            | "--refresh-fingerprints" => {
                args.switch(&flag)?;
                match flag.name.as_str() {
                    "--json" => a.json = true,
                    "--fields" => a.fields = true,
                    "--rerun-oracle" => a.rerun_oracle = true,
                    "--settings" => a.settings = true,
                    "--refresh-fingerprints" => a.refresh = Some(Vec::new()),
                    _ => return Ok(None),
                }
            }
            "--out" => a.out = Some(args.value(&flag)?.into()),
            "--oracle-trace" => a.oracle_trace = Some(args.value(&flag)?.into()),
            "--assets" => a.assets = Some(args.value(&flag)?.into()),
            "--jobs" => a.jobs = Some(args.number::<usize>(&flag)?.max(1)),
            _ if diff_option(&mut a.diff, &flag, "--compare", &mut args)? => diff_given = true,
            f => return Err(format!("unknown option {f}")),
        }
    }
    check_diff_options(&a.diff)?;
    if let Some(list) = &mut a.refresh {
        if a.out.is_some()
            || a.oracle_trace.is_some()
            || a.json
            || a.fields
            || a.settings
            || diff_given
        {
            return Err(
                "--refresh-fingerprints takes no --out, --oracle-trace, --json, --fields, --settings or diff \
                 options (it always dumps the fields, a replay's directory says which program it runs, and \
                 fingerprints match exactly)"
                    .into(),
            );
        }
        *list = replays;
        return Ok(Some(a));
    }
    if a.jobs.is_some() {
        return Err("--jobs is only for --refresh-fingerprints".into());
    }
    let [replay]: [PathBuf; 1] = replays
        .try_into()
        .map_err(|r: Vec<PathBuf>| format!("expected one replay, got {}", r.len()))?;
    a.replay = replay;
    Ok(Some(a))
}

/// The repository root, for `tools/oracle`.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// What the cached oracle trace depends on: the oracle build, the program, the replay, and `--fields`.
/// `None` if the oracle's build stamp can't be computed (the trace then isn't cached).
fn oracle_key(replay_text: &str, exe: &str, fields: bool) -> Option<String> {
    let out = Command::new("bash")
        .arg("-c")
        .arg("source tools/oracle/common.sh && build_stamp")
        .current_dir(repo_root())
        .output()
        .ok()?;
    let stamp = String::from_utf8(out.stdout).ok()?;
    let stamp = stamp.trim();
    if !out.status.success() || stamp.is_empty() {
        return None;
    }
    Some(format!(
        "oracle {stamp}\nexe {exe}\nfields {fields}\nreplay {}\n{replay_text}",
        replay_text.len()
    ))
}

/// Where the oracle's trace came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum OracleSource {
    /// `--oracle-trace`: not checked against the replay or the oracle build.
    Given,
    /// The cache, made by the same oracle build from the same replay and `--fields`.
    Cached,
    /// The oracle ran now.
    Ran,
}

/// The oracle's trace for the replay: given, cached, or made now.
fn oracle_trace(
    args: &Options,
    dir: &Path,
    replay_text: &str,
) -> Result<(PathBuf, OracleSource), String> {
    if let Some(path) = &args.oracle_trace {
        return Ok((path.clone(), OracleSource::Given));
    }
    let path = dir.join("oracle.jsonl");
    let key_path = dir.join("oracle.key");
    let exe = if args.settings {
        "config.exe"
    } else {
        "Spelunky.exe"
    };
    let key = oracle_key(replay_text, exe, args.fields);
    let cached = std::fs::read_to_string(&key_path).ok();
    // A trace from before the oracle wrote its host files has none: run it again.
    let host = host_dir(&path).exists();
    if !args.rerun_oracle && key.is_some() && cached == key && path.exists() && host {
        return Ok((path, OracleSource::Cached));
    }
    let _ = std::fs::remove_file(&key_path);
    eprintln!("fidelity: running the oracle (tools/oracle/run), about 30 s for 1,700 frames...");
    let started = Instant::now();
    let mut cmd = Command::new(repo_root().join("tools/oracle/run"));
    cmd.arg(&args.replay).arg(&path).arg("--exe").arg(exe);
    if args.fields {
        cmd.arg("--fields");
    }
    let status = cmd.status().map_err(|e| format!("tools/oracle/run: {e}"))?;
    if !status.success() {
        return Err(format!("tools/oracle/run failed ({status})"));
    }
    eprintln!(
        "fidelity: the oracle took {:.1} s",
        started.elapsed().as_secs_f64()
    );
    if let Some(key) = key {
        std::fs::write(&key_path, key).map_err(|e| format!("{}: {e}", key_path.display()))?;
    }
    Ok((path, OracleSource::Ran))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Verdict {
    /// Every frame matches, and both runs have the same frames.
    Identical,
    /// A frame both runs have differs.
    Diverged,
    /// The port matches as far as it got, then stopped on an error (usually a missing built-in).
    PortStopped,
    /// Everything compared matches, but one trace has frames the other doesn't (or none were compared).
    UnmatchedFrames,
    /// The traces match, but the runs left different files or highscore tables behind.
    HostDiffers,
}

#[derive(Serialize)]
struct Fidelity<'a> {
    verdict: Verdict,
    /// One line saying what to look at.
    summary: String,
    /// The divergent frame's first difference, as the report prints it (`oPlayer1 #1234 x: a=1.5 b=2`),
    /// and its signature: what differs without instance ids or values (`oPlayer1 x`), the same for the same
    /// divergence in another run.
    difference: Option<String>,
    signature: Option<String>,
    replay: &'a Path,
    oracle_trace: &'a Path,
    oracle_source: OracleSource,
    port_trace: &'a Path,
    port: &'a RunSummary,
    diff: &'a Report,
    /// The files and highscore tables compared, or `None` if the oracle's weren't there (a given
    /// `--oracle-trace` without its `.host` directory).
    host: Option<&'a HostReport>,
}

fn verdict(port: &RunSummary, diff: &Report, host: Option<&HostReport>) -> (Verdict, String) {
    if let Some(d) = &diff.divergence {
        let room = d.room_name_a.as_deref().unwrap_or("?");
        return (
            Verdict::Diverged,
            format!(
                "diverged at frame {} in {room}, after {} equal frames: {} difference(s)",
                d.frame, diff.equal_frames, d.total_differences
            ),
        );
    }
    if let Some(e) = &port.error {
        let matched = match diff.last_equal_frame {
            Some(f) => format!("matches the oracle up to frame {f}, then "),
            None => String::new(),
        };
        return (
            Verdict::PortStopped,
            format!("the port {matched}stopped: {}", e.message),
        );
    }
    let host_differs = host.filter(|h| !h.is_identical()).map(|h| {
        format!(
            "{} file(s) and {} highscore place(s) differ",
            h.files.len(),
            h.highscores.len()
        )
    });
    match (diff.outcome, host_differs) {
        // The port didn't fail, so it completed or the game ended, as the oracle's did if the frames match.
        (Outcome::Identical, None) => (
            Verdict::Identical,
            format!("identical: {} frames", diff.equal_frames),
        ),
        (Outcome::Identical, Some(host)) => (
            Verdict::HostDiffers,
            format!(
                "the traces match ({} equal frames), but {host}",
                diff.equal_frames
            ),
        ),
        (_, host) => (
            Verdict::UnmatchedFrames,
            format!(
                "the {} frames in both traces match, but {} frame(s) are only in the oracle's and {} only in the port's{}",
                diff.equal_frames,
                diff.frames_only_in_a.count,
                diff.frames_only_in_b.count,
                host.map_or(String::new(), |h| format!("; and {h}"))
            ),
        ),
    }
}

/// `--refresh-fingerprints`: records each replay's fingerprint from the oracle, if the port matches it.
fn refresh(args: &Options, replays: &[PathBuf]) -> Result<Verdict, String> {
    let repo = std::fs::canonicalize(repo_root()).map_err(|e| format!("the repository: {e}"))?;
    // Every path, and the assets, are checked before the first oracle run, so a typo can't stop a batch
    // halfway.
    let replays = if replays.is_empty() {
        all_replays(&repo)?
    } else {
        (replays.iter())
            .map(|p| Ok((p.clone(), Suite::of(&repo, p)?)))
            .collect::<Result<_, String>>()?
    };
    let mut assets: [Option<Arc<gm::Assets>>; 2] = [None, None];
    for (_, suite) in &replays {
        if let slot @ None = &mut assets[usize::from(suite.settings)] {
            let dir = args.assets.clone().unwrap_or_else(|| repo.join("assets"));
            let dir = if suite.settings {
                dir.join("config")
            } else {
                dir
            };
            *slot = Some(load_assets(&dir)?);
        }
    }
    // The longest first, so that one doesn't start last and hold everything up; and each once, as two
    // oracles would write the same trace.
    let mut queue = Vec::new();
    for (path, suite) in &replays {
        let canon = std::fs::canonicalize(path).map_err(|e| format!("{}: {e}", path.display()))?;
        queue.push((harness::load_replay(path)?.len(), canon, path, *suite));
    }
    queue.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    queue.dedup_by(|a, b| a.1 == b.1);
    let next = AtomicUsize::new(0);
    let failed = Mutex::new(Vec::new());
    let out = Mutex::new(Ok(()));
    std::thread::scope(|s| {
        for _ in 0..args.jobs.unwrap_or(REFRESH_JOBS).min(queue.len()) {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(&(_, _, path, suite)) = queue.get(i) else {
                        break;
                    };
                    let shown = shown(&repo, path);
                    eprintln!("fidelity: [{}/{}] {shown}", i + 1, queue.len());
                    let assets = assets[usize::from(suite.settings)]
                        .clone()
                        .expect("loaded above");
                    let line = match refresh_one(args, &repo, suite, path, &shown, assets) {
                        Ok(frames) => format!("{shown}: recorded ({frames} frames)\n"),
                        Err(e) => {
                            lock(&failed).push(shown.clone());
                            format!("{shown}: not recorded: {e}\n")
                        }
                    };
                    let mut out = lock(&out);
                    if out.is_ok() {
                        *out = write_stdout(&line);
                    }
                }
            });
        }
    });
    out.into_inner().unwrap_or_else(PoisonError::into_inner)?;
    let mut failed = failed.into_inner().unwrap_or_else(PoisonError::into_inner);
    failed.sort();
    if failed.is_empty() {
        Ok(Verdict::Identical)
    } else {
        write_stdout(&format!(
            "{} of {} fingerprints not recorded: {}\n",
            failed.len(),
            queue.len(),
            failed.join(" ")
        ))?;
        Ok(Verdict::Diverged)
    }
}

/// How many replays `--refresh-fingerprints` records at once by default: each runs an oracle (an emulator
/// and an X server in a container) and then the port, and 6 kept a 10-core Mac busy without thrashing.
const REFRESH_JOBS: usize = 6;

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Records one replay's fingerprint; the number of frames, or why not.
fn refresh_one(
    args: &Options,
    repo: &Path,
    suite: &Suite,
    path: &Path,
    shown: &str,
    assets: Arc<gm::Assets>,
) -> Result<u64, String> {
    let replay_text =
        std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let replay = Replay::parse(&replay_text).map_err(|e| format!("{}: {e}", path.display()))?;
    // The port first: it's fast, and a port that can't run shouldn't cost an oracle run.
    let (port, summary) = run_fingerprint(&replay, assets, suite.settings)?;
    let opts = Options {
        replay: path.to_path_buf(),
        out: None,
        oracle_trace: None,
        rerun_oracle: args.rerun_oracle,
        fields: true,
        settings: suite.settings,
        assets: None,
        json: false,
        diff: DiffOptions::default(),
        refresh: None,
        jobs: None,
    };
    let dir = repo
        .join("target/fidelity")
        .join(path.file_stem().unwrap_or_default());
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let (oracle_path, source) = oracle_trace(&opts, &dir, &replay_text)?;
    let oracle = fingerprint_trace(&replay, &oracle_path)?;
    if let Some(m) = port_mismatch(&oracle, &port, &summary) {
        let settings = if suite.settings { " --settings" } else { "" };
        return Err(format!(
            "the port doesn't match the oracle: {m}. For the exact frame (the oracle's trace is cached): \
             scripts/fidelity --fields{settings} {shown}"
        ));
    }
    oracle.write(&fingerprint_path(repo, suite, path))?;
    if source == OracleSource::Ran {
        // Only made for this; the fingerprint is all that's needed now. The fingerprint is recorded
        // whatever happens here, so a file that can't be deleted is only worth a warning. (There's no key
        // if the oracle's build stamp couldn't be computed.)
        let warn = |r: Result<(), String>| {
            if let Err(e) = r {
                eprintln!("fidelity: warning: {e}");
            }
        };
        warn(harness::host::remove_host(&host_dir(&oracle_path)));
        for f in [oracle_path, dir.join("oracle.key")] {
            match std::fs::remove_file(&f) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                    warn(Err(format!("{}: {e}", f.display())))
                }
                _ => {}
            }
        }
        // Unless something else is in it.
        let _ = std::fs::remove_dir(&dir);
    }
    Ok(oracle.frames)
}

fn run() -> Result<Verdict, String> {
    let Some(args) =
        parse_args(std::env::args().skip(1)).map_err(|e| format!("{e}\n\n{}", usage()))?
    else {
        write_stdout(&format!("{}\n", usage()))?;
        return Ok(Verdict::Identical);
    };
    if let Some(replays) = &args.refresh {
        return refresh(&args, replays);
    }
    let replay_text = std::fs::read_to_string(&args.replay)
        .map_err(|e| format!("{}: {e}", args.replay.display()))?;
    let replay =
        Replay::parse(&replay_text).map_err(|e| format!("{}: {e}", args.replay.display()))?;
    let dir = args.out.clone().unwrap_or_else(|| {
        let stem = args.replay.file_stem().unwrap_or_default();
        repo_root().join("target/fidelity").join(stem)
    });
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;

    // The port first: it's fast, and a bad asset path shouldn't cost an oracle run.
    let assets = load_assets(&args.assets.clone().unwrap_or_else(|| {
        repo_root().join(if args.settings {
            "assets/config"
        } else {
            "assets"
        })
    }))?;
    let port_path = dir.join("port.jsonl");
    let port = run_replay(
        &replay,
        assets,
        &RunOptions {
            trace: Some(port_path.clone()),
            fields: args.fields,
            settings: args.settings,
            ..RunOptions::default()
        },
    )?;
    let (oracle_path, oracle_source) = oracle_trace(&args, &dir, &replay_text)?;

    let oracle_str = oracle_path.to_string_lossy();
    let port_str = port_path.to_string_lossy();
    let report = diff_traces(open_trace(&oracle_str)?, open_trace(&port_str)?, &args.diff)
        .map_err(|e| {
            let path = match e.side {
                trace::diff::Side::A => &oracle_str,
                trace::diff::Side::B => &port_str,
            };
            format!("{path}: {}", e.error)
        })?;
    // The port's are only written by a run that didn't fail. They're compared even when the traces differ,
    // but the verdict only reports them when the traces match.
    let (oracle_host, port_host) = (host_dir(&oracle_path), host_dir(&port_path));
    let host = if port.error.is_none() && oracle_host.exists() {
        Some(diff_host(&oracle_host, &port_host)?)
    } else {
        None
    };
    let (verdict, summary) = verdict(&port, &report, host.as_ref());
    let out = if args.json {
        let first = report
            .divergence
            .as_ref()
            .and_then(|d| d.differences.first());
        let f = Fidelity {
            verdict,
            summary,
            difference: first.map(|d| d.describe()),
            signature: first.map(|d| d.signature()),
            replay: &args.replay,
            oracle_trace: &oracle_path,
            oracle_source,
            port_trace: &port_path,
            port: &port,
            diff: &report,
            host: host.as_ref(),
        };
        format!(
            "{}\n",
            serde_json::to_string_pretty(&f).map_err(|e| e.to_string())?
        )
    } else {
        let host = match (&host, &port.error) {
            (Some(h), _) => h.to_text(),
            (None, Some(_)) => "host: not compared (the port stopped on an error)".into(),
            (None, None) => format!("host: not compared ({} is missing)", oracle_host.display()),
        };
        format!(
            "replay:     {}\noracle (a): {}{}\nport (b):   {}: {}\n\n{}\n{host}\n\n{summary}\n",
            args.replay.display(),
            oracle_path.display(),
            match oracle_source {
                OracleSource::Given => "",
                OracleSource::Cached => " (cached)",
                OracleSource::Ran => " (ran now)",
            },
            port_path.display(),
            port.to_text(),
            report.to_text().trim_end(),
        )
    };
    write_stdout(&out)?;
    Ok(verdict)
}

fn main() -> ExitCode {
    match run() {
        Ok(Verdict::Identical) => ExitCode::SUCCESS,
        Ok(_) => ExitCode::from(1),
        Err(e) => {
            eprintln!("fidelity: {e}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Option<Options>, String> {
        parse_args(args.iter().map(|s| s.to_string()))
    }

    #[test]
    fn arguments() {
        let a = parse(&[
            "r.txt",
            "--fields",
            "--compare=x,vars.hp",
            "--ignore-fields",
            "vars.randNum",
            "--to=100",
            "--json",
            "--out",
            "o",
        ])
        .unwrap()
        .unwrap();
        assert_eq!(a.replay, PathBuf::from("r.txt"));
        assert!(a.fields && a.json && !a.rerun_oracle);
        assert_eq!(a.out, Some(PathBuf::from("o")));
        assert_eq!(a.diff.filter.fields, ["x", "vars.hp"]);
        assert_eq!(a.diff.filter.ignore_fields, ["vars.randNum"]);
        assert_eq!(a.diff.to, Some(100));
        let a = parse(&["--refresh-fingerprints", "--jobs", "0", "a", "b"])
            .unwrap()
            .unwrap();
        assert_eq!(
            a.refresh,
            Some(vec![PathBuf::from("a"), PathBuf::from("b")])
        );
        assert_eq!(a.jobs, Some(1));
        for (args, want) in [
            (&[][..], "expected one replay, got 0"),
            (
                &["a", "--from", "9", "--to", "3"],
                "--from 9 is after --to 3",
            ),
            (&["a", "--json=1"], "--json takes no value"),
            (&["a", "--nope"], "unknown option --nope"),
            (
                &["a", "--jobs", "2"],
                "--jobs is only for --refresh-fingerprints",
            ),
            (
                &["--refresh-fingerprints", "--tolerance", "1"],
                "--refresh-fingerprints takes no",
            ),
            (
                &["--refresh-fingerprints", "--fields"],
                "--refresh-fingerprints takes no",
            ),
        ] {
            let err = parse(args).err().unwrap();
            assert!(err.contains(want), "{args:?}: {err}");
        }
    }
}
