//! Compares two traces and reports the first frame where they diverge. See `docs/formats.md`.

use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::process::ExitCode;

use trace::diff::{DiffOptions, Matching, Outcome, diff_traces};
use trace::{Filter, TraceReader};

const USAGE: &str = "\
usage: trace-diff [options] <a.jsonl> <b.jsonl>

Compares two traces (JSON lines, optionally .gz; `-` reads stdin) and reports the first frame where they
differ, with the differing instances and fields and their values over the frames before. Frames are paired
by number; frames only one trace has are counted, not compared.

options:
  --json                 machine-readable report on stdout
  --from <frame>         ignore frames before this one
  --to <frame>           ignore frames after this one
  --tolerance <x>        treat reals at most x apart as equal (default 0: exact)
  --match <mode>         pair instances by: auto (id, then object and creation order; default),
                         id, or object (object and creation order only)
  --objects <list>       compare only these objects (comma-separated globs, e.g. oPlayer1,oBat*)
  --ignore-objects <list>
  --fields <list>        compare only these fields (paths such as x,vars.xVel,alarms,globals.money;
                         room, seed and instance_count are always compared unless ignored)
  --ignore-fields <list>
  --context <n>          frames of history before the divergence (default 3)
  --max-diffs <n>        differences to list (default 50; 0 for all)
  -h, --help

exit status: 0 identical, 1 diverged, frames only in one trace, or nothing compared; 2 error";

struct Args {
    paths: [String; 2],
    json: bool,
    opts: DiffOptions,
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Option<Args>, String> {
    let mut opts = DiffOptions::default();
    let mut json = false;
    let mut paths = Vec::new();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let (flag, inline) = match arg.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f.to_string(), Some(v.to_string())),
            _ => (arg.clone(), None),
        };
        let mut value = || -> Result<String, String> {
            inline
                .clone()
                .or_else(|| args.next())
                .ok_or_else(|| format!("{flag} needs a value"))
        };
        match flag.as_str() {
            "-h" | "--help" => return Ok(None),
            "--json" => json = true,
            "--tolerance" => {
                let v = value()?;
                opts.tolerance = v
                    .parse()
                    .ok()
                    .filter(|t: &f64| *t >= 0.0)
                    .ok_or_else(|| format!("bad --tolerance {v:?}"))?;
            }
            "--match" => {
                let v = value()?;
                opts.matching = Matching::parse(&v)
                    .ok_or_else(|| format!("bad --match {v:?} (auto, id, object)"))?;
            }
            "--objects" => opts.filter.objects.extend(Filter::split_list(&value()?)),
            "--ignore-objects" => opts
                .filter
                .ignore_objects
                .extend(Filter::split_list(&value()?)),
            "--fields" => opts.filter.fields.extend(Filter::split_list(&value()?)),
            "--ignore-fields" => opts
                .filter
                .ignore_fields
                .extend(Filter::split_list(&value()?)),
            "--context" => {
                let v = value()?;
                opts.context = v.parse().map_err(|_| format!("bad --context {v:?}"))?;
            }
            "--max-diffs" => {
                let v = value()?;
                opts.max_differences = v.parse().map_err(|_| format!("bad --max-diffs {v:?}"))?;
            }
            "--from" => {
                let v = value()?;
                opts.from = Some(v.parse().map_err(|_| format!("bad --from {v:?}"))?);
            }
            "--to" => {
                let v = value()?;
                opts.to = Some(v.parse().map_err(|_| format!("bad --to {v:?}"))?);
            }
            f if f.starts_with('-') && f != "-" => return Err(format!("unknown option {f}")),
            _ => paths.push(arg),
        }
    }
    let paths: [String; 2] = paths
        .try_into()
        .map_err(|p: Vec<String>| format!("expected two traces, got {}", p.len()))?;
    if let (Some(from), Some(to)) = (opts.from, opts.to)
        && from > to
    {
        return Err(format!("--from {from} is after --to {to}"));
    }
    if paths.iter().all(|p| p == "-") {
        return Err("only one trace can be read from stdin".into());
    }
    Ok(Some(Args { paths, json, opts }))
}

fn open(path: &str) -> Result<TraceReader<Box<dyn BufRead>>, String> {
    let raw: Box<dyn Read> = if path == "-" {
        Box::new(io::stdin().lock())
    } else {
        Box::new(File::open(path).map_err(|e| format!("{path}: {e}"))?)
    };
    let input: Box<dyn BufRead> = if path.ends_with(".gz") {
        Box::new(BufReader::new(flate2::read::MultiGzDecoder::new(raw)))
    } else {
        Box::new(BufReader::with_capacity(1 << 20, raw))
    };
    TraceReader::new(input).map_err(|e| format!("{path}: {e}"))
}

fn run() -> Result<Outcome, String> {
    let Some(args) = parse_args(std::env::args().skip(1)).map_err(|e| format!("{e}\n\n{USAGE}"))?
    else {
        println!("{USAGE}");
        return Ok(Outcome::Identical);
    };
    let [a, b] = &args.paths;
    let report = diff_traces(open(a)?, open(b)?, &args.opts).map_err(|e| {
        let path = match e.side {
            trace::diff::Side::A => a,
            trace::diff::Side::B => b,
        };
        format!("{path}: {}", e.error)
    })?;
    if args.json {
        println!("{}", report.to_json());
    } else {
        print!("{}", report.to_text());
    }
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

    fn parse(args: &[&str]) -> Result<Option<Args>, String> {
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
        ] {
            let err = parse(args).err().unwrap();
            assert!(err.contains(want), "{args:?}: {err}");
        }
    }
}
