//! What the command-line tools share: reading traces, diff options, and argument helpers
//! (`trace-diff` here, `replay-run` and `fidelity` in the `harness` crate), PNG encoding and decoding,
//! and safe file names.

use std::collections::BTreeSet;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Write};

use trace::diff::{DiffOptions, Matching};
use trace::{Filter, TraceReader};

/// Opens a trace (JSON lines, `.gz` too; `-` is stdin).
pub fn open_trace(path: &str) -> Result<TraceReader<Box<dyn BufRead>>, String> {
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

/// The diff options `trace-diff` and `fidelity` share, as usage lines, with `fields_flag` as the field
/// filter's flag (see [`diff_option`]).
pub fn diff_usage(fields_flag: &str) -> String {
    let fields = format!("{fields_flag} <list>");
    let options = [
        ("--from <frame>", "ignore frames before this one"),
        ("--to <frame>", "ignore frames after this one"),
        (
            "--tolerance <x>",
            "treat reals at most x apart as equal (default 0: exact)",
        ),
        (
            "--match <mode>",
            "pair instances by: auto (id, then object and creation order; default),",
        ),
        ("", "id, or object (object and creation order only)"),
        (
            "--objects <list>",
            "compare only these objects (comma-separated globs, e.g. oPlayer1,oBat*)",
        ),
        ("--ignore-objects <list>", ""),
        (
            &fields,
            "compare only these fields (paths such as x,vars.xVel,alarms,globals.money;",
        ),
        (
            "",
            "room, seed and instance_count are always compared unless ignored)",
        ),
        ("--ignore-fields <list>", ""),
        (
            "--context <n>",
            "frames of history before the divergence (default 3)",
        ),
        (
            "--max-diffs <n>",
            "differences to list (default 50; 0 for all)",
        ),
    ];
    let lines: Vec<String> = options
        .iter()
        .map(|(flag, what)| format!("  {flag:<23}{what}").trim_end().to_string())
        .collect();
    lines.join("\n")
}

/// Applies `flag` if it's a diff option, taking its value from `args`. `fields_flag` is the flag for the
/// field filter (`--fields` in `trace-diff`; `fidelity` uses `--fields` for the dump switch, as the oracle
/// does, and `--compare` for the filter). Returns `false` if `flag` isn't a diff option.
pub fn diff_option(
    opts: &mut DiffOptions,
    flag: &Flag,
    fields_flag: &str,
    args: &mut Args,
) -> Result<bool, String> {
    match flag.name.as_str() {
        "--tolerance" => {
            let v = args.value(flag)?;
            opts.tolerance = v
                .parse()
                .ok()
                .filter(|t: &f64| *t >= 0.0)
                .ok_or_else(|| format!("bad --tolerance {v:?}"))?;
        }
        "--match" => {
            let v = args.value(flag)?;
            opts.matching = Matching::parse(&v)
                .ok_or_else(|| format!("bad --match {v:?} (auto, id, object)"))?;
        }
        "--objects" => opts.filter.objects.extend(args.list(flag)?),
        "--ignore-objects" => opts.filter.ignore_objects.extend(args.list(flag)?),
        f if f == fields_flag => opts.filter.fields.extend(args.list(flag)?),
        "--ignore-fields" => opts.filter.ignore_fields.extend(args.list(flag)?),
        "--context" => opts.context = args.number(flag)?,
        "--max-diffs" => opts.max_differences = args.number(flag)?,
        "--from" => opts.from = Some(args.number(flag)?),
        "--to" => opts.to = Some(args.number(flag)?),
        _ => return Ok(false),
    }
    Ok(true)
}

/// Checks the diff options together.
pub fn check_diff_options(opts: &DiffOptions) -> Result<(), String> {
    match (opts.from, opts.to) {
        (Some(from), Some(to)) if from > to => Err(format!("--from {from} is after --to {to}")),
        _ => Ok(()),
    }
}

/// Command-line arguments, read one at a time: flags (`--name`, `--name=value`, `-h`) and positional
/// arguments (`-` is one: stdin). Each tool matches on [`Flag::name`] and then asks for what the flag
/// takes, so every tool treats values the same way: `--name value` or `--name=value` for options, and a
/// value given to a switch (`--json=0`) is an error.
pub struct Args {
    args: std::vec::IntoIter<String>,
}

/// One argument.
pub enum Arg {
    Flag(Flag),
    Positional(String),
}

/// A flag, with the value it was given inline (`--name=value`), if any.
pub struct Flag {
    pub name: String,
    inline: Option<String>,
}

impl Args {
    pub fn new(args: impl IntoIterator<Item = String>) -> Args {
        Args {
            args: args.into_iter().collect::<Vec<_>>().into_iter(),
        }
    }

    /// The next argument.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<Arg> {
        let arg = self.args.next()?;
        if arg == "-" || !arg.starts_with('-') {
            return Some(Arg::Positional(arg));
        }
        let (name, inline) = match arg.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f.to_string(), Some(v.to_string())),
            _ => (arg, None),
        };
        Some(Arg::Flag(Flag { name, inline }))
    }

    /// Accepts `flag` as a switch: it takes no value.
    pub fn switch(&self, flag: &Flag) -> Result<(), String> {
        match flag.inline {
            Some(_) => Err(format!("{} takes no value", flag.name)),
            None => Ok(()),
        }
    }

    /// `flag`'s value: inline, or the next argument.
    pub fn value(&mut self, flag: &Flag) -> Result<String, String> {
        flag.inline
            .clone()
            .or_else(|| self.args.next())
            .ok_or_else(|| format!("{} needs a value", flag.name))
    }

    /// `flag`'s value as a number.
    pub fn number<T: std::str::FromStr>(&mut self, flag: &Flag) -> Result<T, String> {
        let v = self.value(flag)?;
        v.parse().map_err(|_| format!("bad {} {v:?}", flag.name))
    }

    /// `flag`'s value as a comma-separated list.
    pub fn list(&mut self, flag: &Flag) -> Result<Vec<String>, String> {
        Ok(Filter::split_list(&self.value(flag)?))
    }
}

/// An RGBA image as a PNG, the same bytes every time: 8 bits a channel, fixed settings (the extractor's
/// atlases and backgrounds, and `icon`'s icons).
pub fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, png::EncodingError> {
    let mut out = Vec::new();
    let mut writer = png_encoder(&mut out, width, height).write_header()?;
    writer.write_image_data(rgba)?;
    writer.finish()?;
    Ok(out)
}

/// A PNG encoder with [`encode_png`]'s fixed settings, for images it can't write in one call (`wiki-sprites`'
/// animations and text chunks).
pub fn png_encoder<W: Write>(out: W, width: u32, height: u32) -> png::Encoder<'static, W> {
    let mut encoder = png::Encoder::new(out, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::High);
    encoder
}

/// Decodes an 8-bit RGBA PNG (what [`encode_png`] writes) into its width, height and pixels.
pub fn decode_png(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let mut reader = png::Decoder::new(io::Cursor::new(bytes))
        .read_info()
        .map_err(|e| e.to_string())?;
    let size = reader
        .output_buffer_size()
        .ok_or("PNG too large to decode")?;
    let mut buf = vec![0; size];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    if (info.color_type, info.bit_depth) != (png::ColorType::Rgba, png::BitDepth::Eight) {
        return Err(format!(
            "expected 8-bit RGBA, got {:?} {:?}",
            info.color_type, info.bit_depth
        ));
    }
    buf.truncate(info.buffer_size());
    Ok((info.width, info.height, buf))
}

/// Writes to stdout without `print!`'s panic on a closed pipe (`... | head -1`). A reader that stopped
/// early isn't an error: the exit status still says what happened.
pub fn write_stdout(text: &str) -> Result<(), String> {
    let mut out = io::stdout().lock();
    match out.write_all(text.as_bytes()).and_then(|()| out.flush()) {
        Err(e) if e.kind() != io::ErrorKind::BrokenPipe => Err(format!("writing output: {e}")),
        _ => Ok(()),
    }
}

/// Hands out file-system-safe names, unique within one directory.
#[derive(Default)]
pub struct UniqueNames(BTreeSet<String>);

impl UniqueNames {
    pub fn get(&mut self, name: &str, id: usize) -> String {
        let mut safe: String = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        // Add the id (and then a counter) until the name is free, so a duplicate (or, on
        // case-insensitive file systems, a case-only duplicate) can't overwrite another resource's file.
        let base = safe.clone();
        let mut attempt = 0;
        while safe.is_empty() || !self.0.insert(safe.to_ascii_lowercase()) {
            safe = match attempt {
                0 => format!("{base}__{id}"),
                n => format!("{base}__{id}_{n}"),
            };
            attempt += 1;
        }
        safe
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_names_are_safe_and_distinct() {
        let mut n = UniqueNames::default();
        assert_eq!(n.get("oPlayer1", 0), "oPlayer1");
        assert_eq!(n.get("oplayer1", 1), "oplayer1__1");
        assert_eq!(n.get("../x y", 2), "___x_y");
        assert_eq!(n.get("", 3), "__3");
        // A fallback name that's already taken moves on instead of being handed out twice.
        let mut n = UniqueNames::default();
        assert_eq!(n.get("foo__2", 0), "foo__2");
        assert_eq!(n.get("foo", 1), "foo");
        assert_eq!(n.get("foo", 2), "foo__2_1");
    }
}
