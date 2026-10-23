//! Writes a random-input replay for fidelity fuzzing (`harness::fuzz`). `fuzz-sweep` runs many of them.

use std::path::PathBuf;
use std::process::ExitCode;

use harness::fuzz::{Start, fuzz_replay};
use harness::load_assets;
use tools::{Arg, Args, write_stdout};

const USAGE: &str = "\
usage: fuzz-replay [options] [<out.txt>]

Writes a replay (docs/formats.md) that takes a returning player into a level (the first, as
tools/oracle/examples/returning.txt does but skipping the intro, or a shortcut's), and then plays with
random input from the fuzz seed: held directions, jumps, whipping, bombs, ropes, ladders and doors, running, picking up and
throwing, item switches and pauses, leaning toward the exit. The fuzzer plays the port while it writes, to
see where the player and the exit are, so the replay depends on the seeds, the length and the port.
Without <out.txt>, the replay goes to stdout.

options:
  --start <where> mines (default; the first level), or jungle, ice or temple (levels 5, 9 and 13, through
                  the title's shortcut doors, with the Tunnel Man paid off)
  --seed <n>      the game's RNG seed (default 12345)
  --fuzz <n>      the fuzz seed (default 1)
  --frames <n>    frames of random input after the prelude (default 3000)
  --assets <dir>  the asset bundle (default: assets)
  -h, --help";

struct Options {
    start: Start,
    seed: i32,
    fuzz: u64,
    frames: u64,
    assets: PathBuf,
    out: Option<PathBuf>,
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Option<Options>, String> {
    let mut o = Options {
        start: Start::Mines,
        seed: 12345,
        fuzz: 1,
        frames: 3000,
        assets: PathBuf::from("assets"),
        out: None,
    };
    let mut paths = Vec::new();
    let mut args = Args::new(args);
    while let Some(arg) = args.next() {
        let flag = match arg {
            Arg::Positional(p) => {
                paths.push(PathBuf::from(p));
                continue;
            }
            Arg::Flag(flag) => flag,
        };
        match flag.name.as_str() {
            "-h" | "--help" => {
                args.switch(&flag)?;
                return Ok(None);
            }
            "--start" => o.start = Start::parse(&args.value(&flag)?)?,
            "--seed" => o.seed = args.number(&flag)?,
            "--fuzz" => o.fuzz = args.number(&flag)?,
            "--frames" => o.frames = args.number(&flag)?,
            "--assets" => o.assets = args.value(&flag)?.into(),
            f => return Err(format!("unknown option {f}")),
        }
    }
    if paths.len() > 1 {
        return Err("expected at most one output path".into());
    }
    o.out = paths.pop();
    Ok(Some(o))
}

fn run() -> Result<(), String> {
    let Some(o) = parse_args(std::env::args().skip(1)).map_err(|e| format!("{e}\n\n{USAGE}"))?
    else {
        return write_stdout(&format!("{USAGE}\n"));
    };
    let replay = fuzz_replay(load_assets(&o.assets)?, o.start, o.seed, o.fuzz, o.frames)?;
    let text = format!(
        "# fuzz-replay --start {} --seed {} --fuzz {} --frames {}: random input from frame {}\n{}",
        o.start.name(),
        o.seed,
        o.fuzz,
        o.frames,
        o.start.fuzz_start(),
        replay.to_text()
    );
    match &o.out {
        Some(path) => std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display())),
        None => write_stdout(&text),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("fuzz-replay: {e}");
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
        let o = parse(&[
            "--start", "ice", "--seed", "-7", "--fuzz", "3", "--frames", "10", "o.txt",
        ])
        .unwrap()
        .unwrap();
        assert_eq!((o.start, o.seed, o.fuzz, o.frames), (Start::Ice, -7, 3, 10));
        assert!(parse(&["--start", "moon"]).is_err());
        assert_eq!(o.out, Some(PathBuf::from("o.txt")));
        assert!(parse(&["a", "b"]).is_err());
        assert!(parse(&["--help"]).unwrap().is_none());
    }
}
