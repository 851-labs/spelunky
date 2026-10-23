//! Runs a replay on the port, headless and at full speed, and writes its trace: the port's side of
//! `tools/oracle/run`. See `docs/formats.md`.

use std::path::PathBuf;
use std::process::ExitCode;

use harness::{RunOptions, load_assets, load_replay, run_replay};
use tools::{Arg, Args, write_stdout};

const USAGE: &str = "\
usage: replay-run [options] <replay.txt> [<out.jsonl>]

Plays a replay (docs/formats.md) on the port with no window, as fast as it runs, and writes a trace line
after every frame, numbered and laid out like the oracle's (tools/oracle/run), so trace-diff can compare
them. Without <out.jsonl>, nothing is written (for --profile or savestates).

options:
  --assets <dir>         the asset bundle (default: assets, from scripts/extract)
  --fields               also dump every instance's variables and the globals, like the oracle's --fields
  --frames <n>           stop before frame n (default: the replay's length)
  --savestate-out <file> write a savestate after the last frame, or with --at, before frame n runs (an
                         older file there is deleted first, even if the run stops before)
  --at <n>
  --savestate-in <file>  start from a savestate instead of booting; the trace starts at its frame
  --profile              report headless ticks per second (stepping only) on stderr
  --faithful-audio       model the sound DLL as the real game has it, with the bundle's sounds; by
                         default the sound scripts answer like the oracle's stub, so traces compare
  --json                 print the run summary as JSON on stdout
  -h, --help

On a GML error the trace holds every frame before the failing one, and replay-run reports the frame, the
GML unit and line, and the missing built-in if that's the cause.

exit status: 0 the replay ran to its end (or the game ended), 1 a GML error stopped it or the savestate
asked for wasn't written (the run stopped before its frame), 2 bad usage or I/O";

struct Options {
    replay: PathBuf,
    assets: PathBuf,
    run: RunOptions,
    json: bool,
    faithful_audio: bool,
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Option<Options>, String> {
    let mut paths = Vec::new();
    let mut assets = PathBuf::from("assets");
    let mut run = RunOptions::default();
    let mut json = false;
    let mut args = Args::new(args);
    while let Some(arg) = args.next() {
        let flag = match arg {
            Arg::Positional(path) => {
                paths.push(PathBuf::from(path));
                continue;
            }
            Arg::Flag(flag) => flag,
        };
        match flag.name.as_str() {
            "-h" | "--help" | "--fields" | "--profile" | "--json" => {
                args.switch(&flag)?;
                match flag.name.as_str() {
                    "--fields" => run.fields = true,
                    "--profile" => run.profile = true,
                    "--json" => json = true,
                    _ => return Ok(None),
                }
            }
            "--assets" => assets = args.value(&flag)?.into(),
            "--frames" => run.frames = Some(args.number(&flag)?),
            "--at" => run.savestate_at = Some(args.number(&flag)?),
            "--savestate-out" => run.savestate_out = Some(args.value(&flag)?.into()),
            "--savestate-in" => run.savestate_in = Some(args.value(&flag)?.into()),
            f => return Err(format!("unknown option {f}")),
        }
    }
    if run.savestate_at.is_some() && run.savestate_out.is_none() {
        return Err("--at needs --savestate-out".into());
    }
    let mut paths = paths.into_iter();
    let replay = paths.next().ok_or("expected a replay")?;
    run.trace = paths.next();
    if paths.next().is_some() {
        return Err("expected a replay and at most one output trace".into());
    }
    Ok(Some(Options {
        replay,
        assets,
        run,
        json,
        faithful_audio,
    }))
}

fn run() -> Result<bool, String> {
    let Some(args) = parse_args(std::env::args().skip(1)).map_err(|e| format!("{e}\n\n{USAGE}"))?
    else {
        write_stdout(&format!("{USAGE}\n"))?;
        return Ok(true);
    };
    let replay = load_replay(&args.replay)?;
    let bundle = load_bundle(&args.assets)?;
    let mut run = args.run;
    if args.faithful_audio {
        run.sounds = Some(load_sounds(&args.assets, &bundle.sounds)?);
    }
    let assets = std::sync::Arc::new(gm::Assets::from_bundle(bundle));
    let summary = run_replay(&replay, assets, &run)?;
    eprintln!("replay-run: {}", summary.to_text());
    if args.json {
        let json = serde_json::to_string(&summary).map_err(|e| e.to_string())?;
        write_stdout(&format!("{json}\n"))?;
    }
    Ok(summary.succeeded())
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            eprintln!("replay-run: {e}");
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
            "--frames=100",
            "--savestate-out",
            "s.bin",
            "--at",
            "40",
            "out.jsonl",
            "--assets",
            "a",
            "--profile",
        ])
        .unwrap()
        .unwrap();
        assert_eq!(a.replay, PathBuf::from("r.txt"));
        assert_eq!(a.run.trace, Some(PathBuf::from("out.jsonl")));
        assert_eq!(a.assets, PathBuf::from("a"));
        assert!(a.run.fields && a.run.profile && !a.json);
        assert_eq!((a.run.frames, a.run.savestate_at), (Some(100), Some(40)));
        assert_eq!(a.run.savestate_out, Some(PathBuf::from("s.bin")));
        assert!(parse(&["--help"]).unwrap().is_none());
    }

    #[test]
    fn bad_arguments() {
        for (args, want) in [
            (&[][..], "expected a replay"),
            (&["a", "b", "c"], "at most one output trace"),
            (&["a", "--frames", "x"], "bad --frames"),
            (&["a", "--at", "3"], "--at needs --savestate-out"),
            (&["a", "--fields=1"], "--fields takes no value"),
            (&["a", "--json=0"], "--json takes no value"),
            (&["a", "--frobnicate"], "unknown option --frobnicate"),
        ] {
            let err = parse(args).err().unwrap();
            assert!(err.contains(want), "{args:?}: {err}");
        }
    }
}
