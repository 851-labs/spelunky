//! The pixel checks of the port's renderer (851-3872, docs/design/draw-comparison.md, layer 3): chosen frames of
//! committed replays, the oracle's picture of each committed under `fidelity/screenshots/`, and the port's
//! compared with it (`fidelity-shots`). The trace's `draws` already holds the port to what GML asks to draw, so a
//! difference here is the renderer's: the `draw` crate and the app's wgpu renderer against OpenGMK's GL on Mesa.
//!
//! `fidelity/screenshots/shots.txt` lists them, a replay a line: its path from the repository's root, the frames
//! (`F1,F2,...`), and optionally how far the pictures may differ, `differ=N` pixels by at most `step=S` in any
//! channel (both 0 by default: exact). `#` starts a comment. Each frame's picture is
//! `fidelity/screenshots/<replay name>/<frame>.png`, as the oracle shows it after that frame (its
//! `--screenshots`), at the window's size; `scripts/fidelity --refresh-screenshots` records them, and beside them
//! [`SOURCE`], the oracle build and the replay they came from, as a fingerprint records them.

use std::path::{Component, Path, PathBuf};

/// The file in a replay's picture directory naming what they came from ([`source`]).
pub const SOURCE: &str = "source.txt";

/// What [`SOURCE`] holds for pictures the oracle build `stamp` (`fingerprint::oracle_build_stamp`, whose first
/// 16 hex digits a fingerprint keeps too) made of the replay whose `fingerprint::replay_hash` is `replay`.
pub fn source(stamp: &str, replay: &str) -> String {
    format!(
        "oracle {}\nreplay {replay}\n",
        &stamp[..stamp.len().min(16)]
    )
}

/// One line of the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shots {
    /// From the repository's root.
    pub replay: PathBuf,
    /// Sorted, each once.
    pub frames: Vec<u64>,
    /// How far the port's picture may be from the oracle's.
    pub tolerance: Tolerance,
}

/// How many pixels may differ, and by how much in any channel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tolerance {
    pub differ: u64,
    pub step: u8,
}

impl Shots {
    /// The replay's name: its file's, without `.txt`.
    pub fn name(&self) -> String {
        self.replay
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    }

    /// The directory of its pictures, under `dir` (`fidelity/screenshots`).
    pub fn dir(&self, dir: &Path) -> PathBuf {
        dir.join(self.name())
    }

    /// Whether it plays the settings program (a replay of `fidelity/settings/`).
    pub fn settings(&self) -> bool {
        self.replay.starts_with("fidelity/settings")
    }
}

/// Where the list and the pictures are.
pub fn screenshots_dir(repo: &Path) -> PathBuf {
    repo.join("fidelity/screenshots")
}

/// Reads the list.
pub fn read_list(repo: &Path) -> Result<Vec<Shots>, String> {
    let path = screenshots_dir(repo).join("shots.txt");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse_list(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Parses the list.
pub fn parse_list(text: &str) -> Result<Vec<Shots>, String> {
    let mut out: Vec<Shots> = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let at = |e: String| format!("line {}: {e}", n + 1);
        let line = line.split('#').next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        let mut words = line.split_whitespace();
        let word = words.next().expect("not empty");
        if Path::new(word).is_absolute() {
            return Err(at(format!("{word}: a path from the repository's root")));
        }
        // As written, but for any `./`, so `settings()` sees where it is.
        let replay: PathBuf = (Path::new(word).components())
            .filter(|c| !matches!(c, Component::CurDir))
            .collect();
        let frames = words.next().ok_or_else(|| at("no frames".into()))?;
        let mut frames = (frames.split(','))
            .map(|f| f.parse().map_err(|_| at(format!("bad frame {f:?}"))))
            .collect::<Result<Vec<u64>, _>>()?;
        frames.sort_unstable();
        frames.dedup();
        let mut tolerance = Tolerance::default();
        for w in words {
            match w.split_once('=') {
                Some(("differ", v)) => {
                    tolerance.differ = v.parse().map_err(|_| at(format!("bad {w}")))?;
                }
                Some(("step", v)) => {
                    tolerance.step = v.parse().map_err(|_| at(format!("bad {w}")))?
                }
                _ => return Err(at(format!("unknown {w:?} (differ=N or step=S)"))),
            }
        }
        if out
            .iter()
            .any(|s| s.replay.file_stem() == replay.file_stem())
        {
            return Err(at(format!(
                "{} again: its pictures' directory is its name",
                replay.display()
            )));
        }
        out.push(Shots {
            replay,
            frames,
            tolerance,
        });
    }
    Ok(out)
}

/// The lines of `list` for `replays` (by name; all of them if none), each once.
pub fn select(list: Vec<Shots>, replays: &[PathBuf]) -> Result<Vec<Shots>, String> {
    if replays.is_empty() {
        return Ok(list);
    }
    let mut names: Vec<String> = (replays.iter())
        .map(|p| {
            p.file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names.dedup();
    if let Some(n) = names.iter().find(|n| !list.iter().any(|s| s.name() == **n)) {
        return Err(format!("{n} isn't in fidelity/screenshots/shots.txt"));
    }
    Ok(list
        .into_iter()
        .filter(|s| names.contains(&s.name()))
        .collect())
}

/// A picture: width, height and RGBA rows, top first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Reads an 8-bit RGB or RGBA PNG: the port writes RGB, the oracle RGBA, opaque (`tools::decode_png` takes only
/// RGBA).
pub fn read_png(path: &Path) -> Result<Picture, String> {
    let at = |e: String| format!("{}: {e}", path.display());
    let file = std::fs::File::open(path).map_err(|e| at(e.to_string()))?;
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .map_err(|e| at(e.to_string()))?;
    let mut buf = vec![
        0;
        reader
            .output_buffer_size()
            .ok_or_else(|| at("too large".into()))?
    ];
    let info = reader.next_frame(&mut buf).map_err(|e| at(e.to_string()))?;
    buf.truncate(info.buffer_size());
    let rgba = match (info.color_type, info.bit_depth) {
        (png::ColorType::Rgba, png::BitDepth::Eight) => buf,
        (png::ColorType::Rgb, png::BitDepth::Eight) => buf
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|&[r, g, b]| [r, g, b, 255])
            .collect(),
        other => return Err(at(format!("{other:?}, not 8-bit RGB or RGBA"))),
    };
    Ok(Picture {
        width: info.width,
        height: info.height,
        rgba,
    })
}

/// Writes a picture as an RGBA PNG.
pub fn write_png(path: &Path, p: &Picture) -> Result<(), String> {
    let bytes = tools::encode_png(p.width, p.height, &p.rgba)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// How two pictures of a frame differ, alpha aside (the window shows none).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Difference {
    /// The pixels that differ.
    pub pixels: u64,
    /// The most any channel of one differs by.
    pub step: u8,
    /// The box they're in: left, top, right, bottom (inclusive).
    pub bounds: Option<(u32, u32, u32, u32)>,
}

impl Difference {
    pub fn within(&self, t: Tolerance) -> bool {
        self.pixels <= t.differ && (self.pixels == 0 || self.step <= t.step)
    }
}

/// How `b` differs from `a`, which are the same size; and a picture of where: `b`'s pixels, dimmed where they're
/// the same, magenta where they aren't.
pub fn compare(a: &Picture, b: &Picture) -> Result<(Difference, Picture), String> {
    if (a.width, a.height) != (b.width, b.height) {
        return Err(format!(
            "the oracle's picture is {}x{}, the port's {}x{}",
            a.width, a.height, b.width, b.height
        ));
    }
    let mut d = Difference::default();
    let mut map = Vec::with_capacity(b.rgba.len());
    let w = a.width as usize;
    for (i, (pa, pb)) in a
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .zip(b.rgba.as_chunks::<4>().0)
        .enumerate()
    {
        let step = (0..3).map(|c| pa[c].abs_diff(pb[c])).max().unwrap_or(0);
        if step == 0 {
            map.extend(pb[..3].iter().map(|&c| c / 3));
            map.push(255);
            continue;
        }
        map.extend_from_slice(&[255, 0, 255, 255]);
        let (x, y) = ((i % w) as u32, (i / w) as u32);
        d.pixels += 1;
        d.step = d.step.max(step);
        d.bounds = Some(match d.bounds {
            None => (x, y, x, y),
            Some((l, t, r, bo)) => (l.min(x), t.min(y), r.max(x), bo.max(y)),
        });
    }
    let map = Picture {
        width: b.width,
        height: b.height,
        rgba: map,
    };
    Ok((d, map))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list() {
        let list = parse_list(
            "# the HUD\nfidelity/replays/a.txt 30,10,10 # two\n\n./fidelity/settings/b.txt 5 differ=12 step=2\n",
        )
        .unwrap();
        assert_eq!(list[1].replay, Path::new("fidelity/settings/b.txt"));
        assert_eq!(list[1].name(), "b");
        let some = select(list.clone(), &["b.txt".into(), "x/b.txt".into()]).unwrap();
        assert_eq!(some, [list[1].clone()]);
        assert!(
            select(list.clone(), &["c.txt".into()])
                .unwrap_err()
                .contains("c isn't")
        );
        assert_eq!(list[0].frames, [10, 30]);
        assert_eq!(list[0].tolerance, Tolerance::default());
        assert!(!list[0].settings() && list[1].settings());
        assert_eq!(
            list[1].tolerance,
            Tolerance {
                differ: 12,
                step: 2
            }
        );
        for (bad, want) in [
            ("a.txt", "no frames"),
            ("a.txt 1,x", "bad frame"),
            ("a.txt 1 near=3", "unknown"),
            ("a.txt 1\nb/a.txt 2", "again"),
            ("/a.txt 1", "from the repository's root"),
        ] {
            let e = parse_list(bad).unwrap_err();
            assert!(e.contains(want), "{bad}: {e}");
        }
    }

    #[test]
    fn pictures_differ_by_pixels_and_steps() {
        let pic = |px: &[[u8; 4]]| Picture {
            width: 2,
            height: 2,
            rgba: px.concat(),
        };
        let a = pic(&[[0, 0, 0, 255], [10, 10, 10, 255], [0; 4], [5, 5, 5, 0]]);
        // Alpha doesn't count.
        let b = pic(&[[0, 0, 0, 255], [10, 12, 10, 255], [0; 4], [5, 5, 5, 255]]);
        let (d, map) = compare(&a, &b).unwrap();
        assert_eq!(
            d,
            Difference {
                pixels: 1,
                step: 2,
                bounds: Some((1, 0, 1, 0))
            }
        );
        assert_eq!(&map.rgba[4..8], &[255, 0, 255, 255]);
        assert!(!d.within(Tolerance::default()));
        assert!(!d.within(Tolerance { differ: 1, step: 1 }));
        assert!(d.within(Tolerance { differ: 1, step: 2 }));
        assert!(compare(&a, &Picture { width: 1, ..b }).is_err());
    }
}
