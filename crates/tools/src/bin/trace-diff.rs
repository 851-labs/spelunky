//! Compares two traces and reports the first frame where they diverge. See `docs/formats.md`.

use std::process::ExitCode;

use tools::{Arg, Args, check_diff_options, diff_option, diff_usage, open_trace, write_stdout};
use trace::diff::{DiffOptions, Outcome, diff_traces};

const HEAD: &str = "\
usage: trace-diff [options] <a.jsonl> <b.jsonl>

Compares two traces (JSON lines, optionally .gz; `-` reads stdin) and reports the first frame where they
differ, with the differing instances and fields and their values over the frames before. Frames are paired
by number; frames only one trace has are counted, not compared.

options:
  --json                 machine-readable report on stdout
  -h, --help
";

const EXIT: &str = "\
exit status: 0 identical, 1 diverged, frames only in one trace, or nothing compared; 2 error";

fn usage() -> String {
    format!("{HEAD}{}\n\n{EXIT}", diff_usage("--fields"))
}

struct Options {
    paths: [String; 2],
    json: bool,
    opts: DiffOptions,
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Option<Options>, String> {
    let mut opts = DiffOptions::default();
    let mut json = false;
    let mut paths = Vec::new();
    let mut args = Args::new(args);
    while let Some(arg) = args.next() {
        let flag = match arg {
            Arg::Positional(path) => {
                paths.push(path);
                continue;
            }
            Arg::Flag(flag) => flag,
        };
        match flag.name.as_str() {
            "-h" | "--help" => {
                args.switch(&flag)?;
                return Ok(None);
            }
            "--json" => {
                args.switch(&flag)?;
                json = true;
            }
            _ if diff_option(&mut opts, &flag, "--fields", &mut args)? => {}
            f => return Err(format!("unknown option {f}")),
        }
    }
    let paths: [String; 2] = paths
        .try_into()
        .map_err(|p: Vec<String>| format!("expected two traces, got {}", p.len()))?;
    check_diff_options(&opts)?;
    if paths.iter().all(|p| p == "-") {
        return Err("only one trace can be read from stdin".into());
    }
    Ok(Some(Options { paths, json, opts }))
}

fn run() -> Result<Outcome, String> {
    let Some(args) =
        parse_args(std::env::args().skip(1)).map_err(|e| format!("{e}\n\n{}", usage()))?
    else {
        write_stdout(&format!("{}\n", usage()))?;
        return Ok(Outcome::Identical);
    };
    let [a, b] = &args.paths;
    let report = diff_traces(open_trace(a)?, open_trace(b)?, &args.opts).map_err(|e| {
        let path = match e.side {
            trace::diff::Side::A => a,
            trace::diff::Side::B => b,
        };
        format!("{path}: {}", e.error)
    })?;
    let out = if args.json {
        format!("{}\n", report.to_json())
    } else {
        report.to_text()
    };
    write_stdout(&out)?;
    Ok(report.outcome)
}

fn main() -> ExitCode {
    match run() {
        Ok(Outcome::Identical) => ExitCode::SUCCESS,
        Ok(_) => ExitCode::from(1),
        Err(e) => {
            eprintln!("trace-diff: {e}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use trace::diff::Matching;

    fn parse(args: &[&str]) -> Result<Option<Options>, String> {
        parse_args(args.iter().map(|s| s.to_string()))
    }

    #[test]
    fn arguments() {
        let a = parse(&[
            "a.jsonl",
            "--json",
            "--tolerance=1e-9",
            "--match",
            "object",
            "--objects",
            "oPlayer1,oBat*",
            "--ignore-fields=vars.randNum",
            "--context",
            "5",
            "--max-diffs",
            "0",
            "--from=1300",
            "--to",
            "1699",
            "-",
        ])
        .unwrap()
        .unwrap();
        assert_eq!(a.paths, ["a.jsonl".to_string(), "-".to_string()]);
        assert!(a.json);
        assert_eq!((a.opts.from, a.opts.to), (Some(1300), Some(1699)));
        assert_eq!(
            (
                a.opts.tolerance,
                a.opts.matching,
                a.opts.context,
                a.opts.max_differences
            ),
            (1e-9, Matching::Object, 5, 0)
        );
        assert_eq!(a.opts.filter.objects, ["oPlayer1", "oBat*"]);
        assert_eq!(a.opts.filter.ignore_fields, ["vars.randNum"]);
        assert!(parse(&["--help"]).unwrap().is_none());
    }

    #[test]
    fn bad_arguments() {
        for (args, want) in [
            (&["a"][..], "expected two traces, got 1"),
            (&["a", "b", "c"], "expected two traces, got 3"),
            (&["-", "-"], "only one trace"),
            (
                &["a", "b", "--from", "9", "--to", "3"],
                "--from 9 is after --to 3",
            ),
            (&["a", "b", "--tolerance", "-1"], "bad --tolerance"),
            (&["a", "b", "--tolerance", "x"], "bad --tolerance"),
            (&["a", "b", "--match", "name"], "bad --match"),
            (&["a", "b", "--context"], "--context needs a value"),
            (&["a", "b", "--max-diffs", "-3"], "bad --max-diffs"),
            (&["a", "b", "--frobnicate"], "unknown option --frobnicate"),
            (&["a", "b", "--json=false"], "--json takes no value"),
            (&["--help=1"], "--help takes no value"),
        ] {
            let err = parse(args).err().unwrap();
            assert!(err.contains(want), "{args:?}: {err}");
        }
    }
}
