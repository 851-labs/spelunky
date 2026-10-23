//! Runs a replay on the oracle and on the port and reports the first frame where they differ: the loop
//! every fidelity ticket works in. `scripts/fidelity` runs it.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Instant;

use harness::host::{HostReport, diff_host, host_dir};
use harness::{RunOptions, RunSummary, load_assets, run_replay};
use serde::Serialize;
use tools::{Arg, Args, check_diff_options, diff_option, diff_usage, open_trace, write_stdout};
use trace::Replay;
use trace::diff::{DiffOptions, Outcome, Report, diff_traces};

const USAGE: &str = "\
usage: fidelity [options] <replay.txt>

Runs the replay on the oracle (tools/oracle/run) and on the port (like replay-run), then compares the two
traces (like trace-diff, with the oracle as a and the port as b) and reports the first divergence, or
where the port stopped if it stopped before diverging. Then it compares what each run left behind outside
its trace (<trace>.host/): the game directory's new or changed files, byte for byte, and the highscore
table, value for value.

The oracle's trace is cached in the output directory and reused while the replay, --fields and the
oracle build are unchanged, so after the first run only the port runs.

options:
  --out <dir>            where the traces go (default: target/fidelity/<replay name> in the repo)
  --oracle-trace <file>  compare against this oracle trace instead of running the oracle
  --rerun-oracle         run the oracle even if its cached trace is current
  --fields               dump (and compare) instance variables and globals in both runs
  --assets <dir>         the port's asset bundle (default: assets/ in the repo)
  --json                 print one JSON object: verdict, paths, the port's run summary, the diff report
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
    assets: Option<PathBuf>,
    json: bool,
    diff: DiffOptions,
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
        assets: None,
        json: false,
        diff: DiffOptions::default(),
    };
    let mut replays = Vec::new();
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
            "-h" | "--help" | "--json" | "--fields" | "--rerun-oracle" => {
                args.switch(&flag)?;
                match flag.name.as_str() {
                    "--json" => a.json = true,
                    "--fields" => a.fields = true,
                    "--rerun-oracle" => a.rerun_oracle = true,
                    _ => return Ok(None),
                }
            }
            "--out" => a.out = Some(args.value(&flag)?.into()),
            "--oracle-trace" => a.oracle_trace = Some(args.value(&flag)?.into()),
            "--assets" => a.assets = Some(args.value(&flag)?.into()),
            _ if diff_option(&mut a.diff, &flag, "--compare", &mut args)? => {}
            f => return Err(format!("unknown option {f}")),
        }
    }
    check_diff_options(&a.diff)?;
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

/// What the cached oracle trace depends on: the oracle build, the replay, and `--fields`. `None` if the
/// oracle's build stamp can't be computed (the trace then isn't cached).
fn oracle_key(replay_text: &str, fields: bool) -> Option<String> {
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
        "oracle {stamp}\nfields {fields}\nreplay {}\n{replay_text}",
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
    let key = oracle_key(replay_text, args.fields);
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
    cmd.arg(&args.replay).arg(&path);
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

fn run() -> Result<Verdict, String> {
    let Some(args) =
        parse_args(std::env::args().skip(1)).map_err(|e| format!("{e}\n\n{}", usage()))?
    else {
        write_stdout(&format!("{}\n", usage()))?;
        return Ok(Verdict::Identical);
    };
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
    let assets = load_assets(
        &args
            .assets
            .clone()
            .unwrap_or_else(|| repo_root().join("assets")),
    )?;
    let port_path = dir.join("port.jsonl");
    let port = run_replay(
        &replay,
        assets,
        &RunOptions {
            trace: Some(port_path.clone()),
            fields: args.fields,
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
        let f = Fidelity {
            verdict,
            summary,
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
        for (args, want) in [
            (&[][..], "expected one replay, got 0"),
            (
                &["a", "--from", "9", "--to", "3"],
                "--from 9 is after --to 3",
            ),
            (&["a", "--json=1"], "--json takes no value"),
            (&["a", "--nope"], "unknown option --nope"),
        ] {
            let err = parse(args).err().unwrap();
            assert!(err.contains(want), "{args:?}: {err}");
        }
    }
}
