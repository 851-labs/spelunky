//! Runs many random-input replays (`harness::fuzz`) through `scripts/fidelity`, minimizes the divergent ones,
//! and reports what the runs covered.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use harness::fuzz::{
    Coverage, Directed, Start, coverage, fuzz_replay, object_folders, object_table, truncate,
};
use harness::load_assets;
use serde::Serialize;
use tools::{Arg, Args, write_stdout};
use trace::Replay;

const USAGE: &str = "\
usage: fuzz-sweep [options]

Writes a fuzz replay (fuzz-replay) for every pair of game seed and fuzz seed, runs it through
scripts/fidelity, and collects the verdicts. A divergent replay is minimized: cut off a little after its
first divergent frame and run again, to confirm it still diverges there. Afterwards it reports what the
runs covered, from the port playing them: objects, rooms, levels, deaths and held items, by gameplay
ticket. Traces are deleted after each run (they're large), unless --keep.

<out>/results.jsonl gets a line per run as it finishes, <out>/coverage.md the report, and
<out>/replays/ the replays (<start>-s<seed>-f<fuzz>.txt, and -min.txt for a minimized one). A line of
results.jsonl has the run's verdict and first divergent frame, its first difference (`difference`), what
that difference is without instance ids or values (`signature`, the same for the same divergence in
another run), and the command that runs it alone (`reproduce`). Each run's fidelity report
(`fidelity --json`) is <out>/traces/<name>/fidelity.json.

options:
  --seeds <list>   game seeds: 1-10, or 1,5,9, or both (default 1-10)
  --fuzz <list>    fuzz seeds, the same way (default 1)
  --starts <list>  where runs start, taken in turn: mines, jungle, ice, temple (default all four)
  --frames <n>     frames of random input per run (default 3000)
  --jobs <n>       runs at once (default 3); each oracle run is its own container
  --fields         compare instance variables and globals too (much slower, and bigger traces)
  --out <dir>      default: target/fuzz in the repo
  --keep           keep the traces of divergent runs
  --directed       goal-directed runs (fuzz-replay --directed), named goal-<start>-s<seed>-f<fuzz>
  --port-only      don't run the oracle: only write the replays and report their coverage
  --no-build       run the fidelity binary beside this one instead of scripts/fidelity (which builds it)
  --assets <dir>   the asset bundle (default: assets in the repo)
  -h, --help

exit status: 0 every run identical (or not compared), 1 some run wasn't, 2 error";

/// The verdict of a run with --port-only.
const SKIPPED: &str = "skipped";

/// How far past the first divergent frame a minimized replay runs.
const MINIMIZED_TAIL: u64 = 30;

struct Options {
    seeds: Vec<i32>,
    fuzz: Vec<u64>,
    starts: Vec<Start>,
    frames: u64,
    jobs: usize,
    fields: bool,
    out: PathBuf,
    keep: bool,
    port_only: bool,
    directed: bool,
    assets: PathBuf,
    /// `--no-build`: the fidelity binary beside this one, not `scripts/fidelity`.
    no_build: bool,
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `1-3,7` as [1, 2, 3, 7]: each number once, in order of appearance.
fn parse_list<T: TryFrom<i64>>(s: &str) -> Result<Vec<T>, String> {
    let num = |v: &str| {
        v.trim()
            .parse::<i64>()
            .map_err(|_| format!("bad number {v} in {s}"))
    };
    let mut out: Vec<i64> = Vec::new();
    for part in s.split(',') {
        // A range's dash, not a leading minus sign.
        let dash = part.char_indices().skip(1).find(|&(_, c)| c == '-');
        let (lo, hi) = match dash.map(|(i, _)| i) {
            Some(i) => (num(&part[..i])?, num(&part[i + 1..])?),
            None => (num(part)?, num(part)?),
        };
        if hi < lo || hi - lo >= 100_000 {
            return Err(format!("{part} is empty or too long a range"));
        }
        for v in lo..=hi {
            if !out.contains(&v) {
                out.push(v);
            }
        }
    }
    out.into_iter()
        .map(|v| T::try_from(v).map_err(|_| format!("{v} is out of range")))
        .collect()
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Option<Options>, String> {
    let mut o = Options {
        seeds: (1..=10).collect(),
        fuzz: vec![1],
        starts: Start::ALL.to_vec(),
        frames: 3000,
        jobs: 3,
        fields: false,
        out: repo_root().join("target/fuzz"),
        keep: false,
        port_only: false,
        directed: false,
        assets: repo_root().join("assets"),
        no_build: false,
    };
    let mut args = Args::new(args);
    while let Some(arg) = args.next() {
        let flag = match arg {
            Arg::Positional(p) => return Err(format!("unexpected argument {p}")),
            Arg::Flag(flag) => flag,
        };
        match flag.name.as_str() {
            "-h" | "--help" | "--fields" | "--keep" | "--port-only" | "--directed"
            | "--no-build" => {
                args.switch(&flag)?;
                match flag.name.as_str() {
                    "--fields" => o.fields = true,
                    "--no-build" => o.no_build = true,
                    "--port-only" => o.port_only = true,
                    "--directed" => o.directed = true,
                    "--keep" => o.keep = true,
                    _ => return Ok(None),
                }
            }
            "--seeds" => o.seeds = parse_list(&args.value(&flag)?)?,
            "--fuzz" => o.fuzz = parse_list(&args.value(&flag)?)?,
            "--starts" => {
                o.starts = args
                    .list(&flag)?
                    .iter()
                    .map(|s| Start::parse(s))
                    .collect::<Result<_, _>>()?
            }
            "--frames" => o.frames = args.number(&flag)?,
            "--jobs" => o.jobs = args.number::<usize>(&flag)?.max(1),
            "--out" => o.out = args.value(&flag)?.into(),
            "--assets" => o.assets = args.value(&flag)?.into(),
            f => return Err(format!("unknown option {f}")),
        }
    }
    if o.starts.is_empty() {
        return Err("--starts is empty".into());
    }
    Ok(Some(o))
}

/// One run's fidelity verdict, from `fidelity --json`.
#[derive(Clone, Debug, Default, Serialize)]
struct Verdict {
    verdict: String,
    summary: String,
    /// The first divergent frame.
    frame: Option<u64>,
    room: Option<String>,
    /// The divergent frame's first difference, `oPlayer1 #1234 x: a=1.5 b=2` (the oracle is a).
    difference: Option<String>,
    /// What went wrong without instance ids or values, which differ from run to run: the first
    /// difference's signature (`oPlayer1 x`, trace::diff), or the verdict if the traces didn't diverge.
    signature: Option<String>,
}

/// A divergent run's replay, cut off a little after the divergence, and run again.
#[derive(Serialize)]
struct Minimized {
    replay: PathBuf,
    verdict: Verdict,
    /// Whether it diverged on the same frame as the full run.
    reproduces: bool,
}

impl Minimized {
    fn describe(&self) -> String {
        format!(
            "{}: {}{}",
            self.replay
                .file_name()
                .unwrap_or_default()
                .to_string_lossy(),
            self.verdict.summary,
            if self.reproduces {
                ""
            } else {
                " (DOESN'T REPRODUCE the full run's divergence)"
            }
        )
    }
}

#[derive(Serialize)]
struct RunResult {
    name: String,
    start: Start,
    seed: i32,
    fuzz: u64,
    frames: u64,
    seconds: f64,
    #[serde(flatten)]
    verdict: Verdict,
    /// The command that runs just this one again (fuzz-sweep), from a checkout of the same commit.
    reproduce: String,
    minimized: Option<Minimized>,
    coverage: Coverage,
}

/// Runs `scripts/fidelity --json` on a replay (or the fidelity binary, with `--no-build`), with its traces
/// and its report, fidelity.json, in `dir`.
fn fidelity(o: &Options, replay: &Path, dir: &Path) -> Result<Verdict, String> {
    let mut cmd = if o.no_build {
        let exe = std::env::current_exe().map_err(|e| format!("fuzz-sweep's own path: {e}"))?;
        Command::new(exe.with_file_name("fidelity"))
    } else {
        Command::new(repo_root().join("scripts/fidelity"))
    };
    cmd.arg("--json").arg("--assets").arg(&o.assets);
    cmd.arg("--out").arg(dir).arg(replay);
    if o.fields {
        cmd.arg("--fields");
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let log = std::fs::File::create(dir.join("fidelity.log")).map_err(|e| e.to_string())?;
    let out = cmd
        .stderr(Stdio::from(log))
        .output()
        .map_err(|e| format!("fidelity: {e}"))?;
    if out.status.code() == Some(2) || out.stdout.is_empty() {
        let log = std::fs::read_to_string(dir.join("fidelity.log")).unwrap_or_default();
        return Err(format!(
            "fidelity {} failed: {}",
            replay.display(),
            log.lines().last().unwrap_or("")
        ));
    }
    let report = dir.join("fidelity.json");
    std::fs::write(&report, &out.stdout).map_err(|e| format!("{}: {e}", report.display()))?;
    let json: serde_json::Value =
        serde_json::from_slice(&out.stdout).map_err(|e| format!("fidelity's JSON: {e}"))?;
    let divergence = &json["diff"]["divergence"];
    let verdict = json["verdict"].as_str().unwrap_or("?").to_string();
    // fidelity's signature, or for a run that didn't diverge, how it differs.
    let signature = match json["signature"].as_str() {
        Some(signature) => Some(signature.to_string()),
        None if verdict == "identical" => None,
        None => Some(verdict.replace('_', " ")),
    };
    Ok(Verdict {
        verdict,
        summary: json["summary"].as_str().unwrap_or("").to_string(),
        frame: divergence["frame"].as_u64(),
        room: divergence["room_name_a"].as_str().map(str::to_string),
        difference: json["difference"].as_str().map(str::to_string),
        signature,
    })
}

fn remove_traces(dir: &Path) {
    for f in ["oracle.jsonl", "port.jsonl", "oracle.key"] {
        let _ = std::fs::remove_file(dir.join(f));
    }
}

struct Job {
    directed: bool,
    start: Start,
    seed: i32,
    fuzz: u64,
}

impl Job {
    fn name(&self) -> String {
        let goal = if self.directed { "goal-" } else { "" };
        format!("{goal}{}-s{}-f{}", self.start.name(), self.seed, self.fuzz)
    }
}

fn run_one(o: &Options, assets: &Arc<gm::Assets>, job: &Job) -> Result<RunResult, String> {
    let started = Instant::now();
    let name = job.name();
    let replays = o.out.join("replays");
    let replay = fuzz_replay(
        assets.clone(),
        job.start,
        job.seed,
        job.fuzz,
        o.frames,
        o.directed.then(Directed::default),
    )?;
    let path = replays.join(format!("{name}.txt"));
    write_replay(
        &path,
        &replay,
        &format!(
            "fuzz-replay --start {}{} --seed {} --fuzz {} --frames {}",
            job.start.name(),
            if job.directed { " --directed" } else { "" },
            job.seed,
            job.fuzz,
            o.frames
        ),
    )?;
    let dir = o.out.join("traces").join(&name);
    let verdict = if o.port_only {
        Verdict {
            verdict: SKIPPED.into(),
            summary: "not compared (--port-only)".into(),
            ..Verdict::default()
        }
    } else {
        fidelity(o, &path, &dir)?
    };
    let mut minimized = None;
    if let Some(frame) = verdict.frame {
        if !o.keep {
            remove_traces(&dir);
        }
        let len = (frame + MINIMIZED_TAIL).min(replay.len());
        let min_path = replays.join(format!("{name}-min.txt"));
        write_replay(
            &min_path,
            &truncate(&replay, len),
            &format!(
                "{name}.txt cut off {MINIMIZED_TAIL} frames after its first divergence (frame {frame})"
            ),
        )?;
        let min_dir = o.out.join("traces").join(format!("{name}-min"));
        // A failure here mustn't lose the full run's divergence.
        let v = fidelity(o, &min_path, &min_dir).unwrap_or_else(|e| Verdict {
            verdict: "error".into(),
            summary: e,
            ..Verdict::default()
        });
        if !o.keep {
            remove_traces(&min_dir);
        }
        minimized = Some(Minimized {
            reproduces: v.frame == Some(frame),
            replay: min_path,
            verdict: v,
        });
    } else if !o.port_only {
        remove_traces(&dir);
    }
    let coverage = coverage(&replay, assets.clone());
    let reproduce = format!(
        "scripts/cargo run --release -p harness --bin fuzz-sweep --{} --starts {} --seeds {} --fuzz {} \
         --frames {}{}",
        if job.directed { " --directed" } else { "" },
        job.start.name(),
        job.seed,
        job.fuzz,
        o.frames,
        if o.fields { " --fields" } else { "" },
    );
    Ok(RunResult {
        name,
        start: job.start,
        seed: job.seed,
        fuzz: job.fuzz,
        frames: replay.len(),
        seconds: started.elapsed().as_secs_f64(),
        verdict,
        reproduce,
        minimized,
        coverage,
    })
}

fn write_replay(path: &Path, replay: &Replay, comment: &str) -> Result<(), String> {
    std::fs::write(path, format!("# {comment}\n{}", replay.to_text()))
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// The coverage report, in Markdown.
fn report(results: &[RunResult], errors: &[String], inventory: &str) -> String {
    let folders = object_folders(inventory);
    let mut out = String::new();
    let frames: u64 = results.iter().map(|r| r.frames).sum();
    let identical = results
        .iter()
        .filter(|r| r.verdict.verdict == "identical")
        .count();
    let skipped = results
        .iter()
        .filter(|r| r.verdict.verdict == SKIPPED)
        .count();
    let _ = writeln!(
        out,
        "{} runs, {frames} frames: {identical} identical, {} not, {skipped} not compared, {} failed to \
         run.\n",
        results.len() + errors.len(),
        results.len() - identical - skipped,
        errors.len()
    );
    for e in errors {
        let _ = writeln!(out, "- failed: {e}");
    }
    for r in results
        .iter()
        .filter(|r| !matches!(r.verdict.verdict.as_str(), "identical" | SKIPPED))
    {
        let min = match &r.minimized {
            Some(m) => format!("; minimized to {}", m.describe()),
            None => String::new(),
        };
        let _ = writeln!(out, "- `{}`: {}{min}", r.name, r.verdict.summary);
    }

    let mut levels: BTreeMap<i64, usize> = BTreeMap::new();
    let mut rooms: BTreeMap<&str, usize> = BTreeMap::new();
    let (mut active, mut existed, mut held) = (BTreeSet::new(), BTreeSet::new(), BTreeSet::new());
    let mut deaths = 0;
    for r in results {
        *levels.entry(r.coverage.max_level as i64).or_default() += 1;
        for room in &r.coverage.rooms {
            *rooms.entry(room).or_default() += 1;
        }
        active.extend(r.coverage.active.iter().map(String::as_str));
        existed.extend(r.coverage.existed.iter().map(String::as_str));
        held.extend(r.coverage.held.iter().map(String::as_str));
        deaths += r.coverage.deaths;
    }
    let _ = writeln!(out, "\n## Coverage\n");
    let _ = writeln!(
        out,
        "Deepest level per run (level: runs): {}  ",
        levels
            .iter()
            .map(|(l, n)| format!("{l}: {n}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let _ = writeln!(
        out,
        "Rooms (room: runs): {}  ",
        rooms
            .iter()
            .map(|(r, n)| format!("{r}: {n}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let _ = writeln!(out, "Deaths: {deaths}  ");
    let _ = writeln!(
        out,
        "Held by the player: {}\n",
        held.iter().copied().collect::<Vec<_>>().join(", ")
    );
    out.push_str(&object_table(&active, &existed, &folders));
    out
}

fn run() -> Result<bool, String> {
    let Some(o) = parse_args(std::env::args().skip(1)).map_err(|e| format!("{e}\n\n{USAGE}"))?
    else {
        write_stdout(&format!("{USAGE}\n"))?;
        return Ok(true);
    };
    std::fs::create_dir_all(o.out.join("replays"))
        .map_err(|e| format!("{}: {e}", o.out.display()))?;
    let assets = load_assets(&o.assets)?;
    let jobs: Vec<Job> = o
        .seeds
        .iter()
        .flat_map(|&seed| o.fuzz.iter().map(move |&fuzz| (seed, fuzz)))
        .enumerate()
        .map(|(i, (seed, fuzz))| Job {
            start: o.starts[i % o.starts.len()],
            seed,
            fuzz,
            directed: o.directed,
        })
        .collect();
    let results_path = o.out.join("results.jsonl");
    let results_file = Mutex::new(
        std::fs::File::create(&results_path)
            .map_err(|e| format!("{}: {e}", results_path.display()))?,
    );
    let next = AtomicUsize::new(0);
    let results = Mutex::new(Vec::new());
    let errors = Mutex::new(Vec::new());
    let started = Instant::now();
    std::thread::scope(|s| {
        for _ in 0..o.jobs.min(jobs.len()) {
            s.spawn(|| {
                while let Some(job) = jobs.get(next.fetch_add(1, Ordering::Relaxed)) {
                    match run_one(&o, &assets, job) {
                        Ok(r) => {
                            let line = serde_json::to_string(&r).unwrap_or_default();
                            let _ = writeln!(results_file.lock().unwrap(), "{line}");
                            let mut rs = results.lock().unwrap();
                            eprintln!(
                                "[{}/{}] {} ({:.0} s): {}{}",
                                rs.len() + errors.lock().unwrap().len() + 1,
                                jobs.len(),
                                r.name,
                                r.seconds,
                                r.verdict.summary,
                                match &r.minimized {
                                    Some(m) => format!("; minimized to {}", m.describe()),
                                    None => String::new(),
                                }
                            );
                            rs.push(r);
                        }
                        Err(e) => {
                            eprintln!("{}: {e}", job.name());
                            errors.lock().unwrap().push(format!("{}: {e}", job.name()));
                        }
                    }
                }
            });
        }
    });
    let mut results = results.into_inner().unwrap();
    results.sort_by_key(|r| (r.seed, r.fuzz));
    let inventory = std::fs::read_to_string(repo_root().join("docs/source-inventory.md"))
        .map_err(|e| format!("docs/source-inventory.md: {e}"))?;
    let errors = errors.into_inner().unwrap();
    let text = report(&results, &errors, &inventory);
    let report_path = o.out.join("coverage.md");
    std::fs::write(&report_path, &text).map_err(|e| format!("{}: {e}", report_path.display()))?;
    eprintln!(
        "fuzz-sweep: {} runs in {:.0} s; results in {}, the report in {}",
        results.len(),
        started.elapsed().as_secs_f64(),
        results_path.display(),
        report_path.display()
    );
    let first = text.split("\n## Coverage").next().unwrap_or("");
    write_stdout(&format!("{first}\n"))?;
    Ok(errors.is_empty()
        && results
            .iter()
            .all(|r| matches!(r.verdict.verdict.as_str(), "identical" | SKIPPED)))
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            eprintln!("fuzz-sweep: {e}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists() {
        assert_eq!(parse_list::<i32>("1-3,7,2").unwrap(), [1, 2, 3, 7]);
        assert_eq!(parse_list::<i32>("-2--1,-5").unwrap(), [-2, -1, -5]);
        assert_eq!(parse_list::<u64>("4").unwrap(), [4]);
        assert!(parse_list::<u64>("3-1").is_err());
        assert!(parse_list::<u64>("0-1000000").is_err());
        assert!(parse_list::<u64>("-1").is_err());
        assert!(parse_list::<i32>("2147483648").is_err());
        assert!(parse_list::<u64>("x").is_err());
    }

    #[test]
    fn arguments() {
        let parse = |a: &[&str]| parse_args(a.iter().map(|s| s.to_string()));
        let o = parse(&[
            "--seeds",
            "1-2",
            "--fuzz",
            "5,6",
            "--starts",
            "ice,mines",
            "--jobs",
            "0",
        ])
        .unwrap()
        .unwrap();
        assert_eq!((o.seeds, o.fuzz, o.jobs), (vec![1, 2], vec![5, 6], 1));
        assert_eq!(o.starts, [Start::Ice, Start::Mines]);
        assert!(parse(&["--starts", "moon"]).is_err());
        assert!(parse(&["stray"]).is_err());
    }
}
