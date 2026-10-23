//! What a run leaves behind outside its trace (docs/formats.md, "Host files"): the game directory's files that
//! differ from the shipped game's, and GM8's highscore table. The oracle (`tools/oracle/run`) and the port
//! (`replay-run`) both write them to `<trace>.host/`, and `fidelity` compares the two.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use trace::Real;

/// The directory a run's host files go in: the trace's path with its extension replaced by `host`
/// (`port.jsonl` gives `port.host/`), or with `.host` added if that's its extension already, so the
/// directory is never the trace. `tools/oracle/run` names the oracle's the same way.
pub fn host_dir(trace: &Path) -> PathBuf {
    if trace.extension().is_some_and(|e| e == "host") {
        let mut dir = trace.as_os_str().to_owned();
        dir.push(".host");
        return dir.into();
    }
    trace.with_extension("host")
}

/// The game directory's files are under `files/`, by their relative paths.
const FILES: &str = "files";
/// The highscore table, one `{"name":...,"value":...}` line per entry, best first.
const HIGHSCORES: &str = "highscores.jsonl";

/// Writes `host` to `dir`, replacing what was there: the files that differ from (or aren't in) the shipped
/// game, byte for byte, and the highscore table, in the oracle's format (its patch 0009), so the two
/// directories are byte-identical when the runs agree.
pub fn write_host(host: &gm::Host, dir: &Path) -> Result<(), String> {
    let err = |p: &Path, e: std::io::Error| format!("{}: {e}", p.display());
    remove_host(dir)?;
    let shipped = trace::ReplayHeader::default().game_files();
    let files = dir.join(FILES);
    std::fs::create_dir_all(&files).map_err(|e| err(&files, e))?;
    for (path, contents) in &host.files {
        if shipped.get(path).map(String::as_bytes) == Some(contents.as_bytes()) {
            continue;
        }
        // `Host::files` keys are checked to stay inside the game directory (`gm::files::is_game_key`).
        let to = files.join(path);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).map_err(|e| err(parent, e))?;
        }
        std::fs::write(&to, contents.as_bytes()).map_err(|e| err(&to, e))?;
    }
    let mut table = String::new();
    for (name, value) in &host.highscores {
        // GM strings are Latin-1 here; the oracle decodes with the game's code page, the same for ASCII
        // (every name the game writes).
        let entry = Entry {
            name: name.as_bytes().iter().map(|&b| char::from(b)).collect(),
            value: Real(*value),
        };
        table += &serde_json::to_string(&entry).map_err(|e| e.to_string())?;
        table.push('\n');
    }
    let path = dir.join(HIGHSCORES);
    std::fs::write(&path, table).map_err(|e| err(&path, e))
}

/// Deletes `dir` if it exists, so an older run's files never pass for this one's.
pub fn remove_host(dir: &Path) -> Result<(), String> {
    match std::fs::remove_dir_all(dir) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            Err(format!("{}: {e}", dir.display()))
        }
        _ => Ok(()),
    }
}

/// A highscore table entry, a line of `highscores.jsonl`. [`Real`] reads and writes the value as the
/// oracle's dump does (non-finite ones as `"NaN"`, `"inf"` and `"-inf"`) and compares it bit for bit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub name: String,
    pub value: Real,
}

/// The highscore table in `dir`, best first.
fn read_highscores(dir: &Path) -> Result<Vec<Entry>, String> {
    let path = dir.join(HIGHSCORES);
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    text.lines()
        .enumerate()
        .map(|(i, line)| {
            serde_json::from_str(line).map_err(|e| format!("{}:{}: {e}", path.display(), i + 1))
        })
        .collect()
}

/// The files under `dir`'s `files/`, by `/`-separated relative path.
fn read_files(dir: &Path) -> Result<std::collections::BTreeMap<String, Vec<u8>>, String> {
    fn walk(
        root: &Path,
        dir: &Path,
        out: &mut std::collections::BTreeMap<String, Vec<u8>>,
    ) -> Result<(), String> {
        let err = |e: std::io::Error| format!("{}: {e}", dir.display());
        for entry in std::fs::read_dir(dir).map_err(err)? {
            let path = entry.map_err(err)?.path();
            if path.is_dir() {
                walk(root, &path, out)?;
            } else {
                let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
                let rel = path.strip_prefix(root).unwrap_or(&path);
                let key = rel
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                out.insert(key, bytes);
            }
        }
        Ok(())
    }
    let root = dir.join(FILES);
    let mut out = std::collections::BTreeMap::new();
    walk(&root, &root, &mut out)?;
    Ok(out)
}

/// A file the two runs left behind differently.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FileMismatch {
    pub path: String,
    /// Its size on each side; `None` where that run left the shipped file (or none) there.
    pub oracle_bytes: Option<usize>,
    pub port_bytes: Option<usize>,
    /// The offset of the first differing byte, when both have the file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_difference: Option<usize>,
}

/// A place in the highscore table where the two runs differ.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HighscoreMismatch {
    /// 1-based, as `highscore_value` counts.
    pub place: usize,
    pub oracle: Option<Entry>,
    pub port: Option<Entry>,
}

/// How the files and highscore table the two runs left behind compare.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct HostReport {
    /// Files either run added or changed (the union).
    pub files_compared: usize,
    pub files: Vec<FileMismatch>,
    pub highscores: Vec<HighscoreMismatch>,
}

impl HostReport {
    pub fn is_identical(&self) -> bool {
        self.files.is_empty() && self.highscores.is_empty()
    }

    /// For a terminal: one line, then a line per mismatch.
    pub fn to_text(&self) -> String {
        let mut s = format!(
            "host: {} file(s) written or changed, {} differ; {} highscore place(s) differ",
            self.files_compared,
            self.files.len(),
            self.highscores.len()
        );
        let size =
            |n: Option<usize>| n.map_or("the shipped one or none".into(), |n| format!("{n} bytes"));
        for f in &self.files {
            let _ = write!(
                s,
                "\n  {}: oracle {}, port {}",
                f.path,
                size(f.oracle_bytes),
                size(f.port_bytes)
            );
            if let Some(at) = f.first_difference {
                let _ = write!(s, ", first differing byte at {at}");
            }
        }
        let entry = |e: &Option<Entry>| match e {
            Some(Entry { name, value }) => format!("{name:?} {value}"),
            None => "none".into(),
        };
        for h in &self.highscores {
            let _ = write!(
                s,
                "\n  highscore {}: oracle {}, port {}",
                h.place,
                entry(&h.oracle),
                entry(&h.port)
            );
        }
        s
    }
}

/// Compares the host files in `oracle` and `port` (directories [`write_host`] or the oracle wrote): files
/// byte for byte, the table by name and value, bit for bit.
pub fn diff_host(oracle: &Path, port: &Path) -> Result<HostReport, String> {
    let (a, b) = (read_files(oracle)?, read_files(port)?);
    let mut report = HostReport::default();
    let paths: std::collections::BTreeSet<&String> = a.keys().chain(b.keys()).collect();
    report.files_compared = paths.len();
    for path in paths {
        let (x, y) = (a.get(path), b.get(path));
        if x == y {
            continue;
        }
        report.files.push(FileMismatch {
            path: path.clone(),
            oracle_bytes: x.map(Vec::len),
            port_bytes: y.map(Vec::len),
            first_difference: x.zip(y).map(|(x, y)| {
                x.iter()
                    .zip(y)
                    .position(|(p, q)| p != q)
                    .unwrap_or(x.len().min(y.len()))
            }),
        });
    }
    let (a, b) = (read_highscores(oracle)?, read_highscores(port)?);
    for place in 0..a.len().max(b.len()) {
        let (x, y) = (a.get(place), b.get(place));
        if x != y {
            report.highscores.push(HighscoreMismatch {
                place: place + 1,
                oracle: x.cloned(),
                port: y.cloned(),
            });
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gm::GmString;

    fn tmp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("harness-host-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn host_dir_replaces_the_extension() {
        assert_eq!(
            host_dir(Path::new("a/port.jsonl")),
            Path::new("a/port.host")
        );
        assert_eq!(host_dir(Path::new("a/trace")), Path::new("a/trace.host"));
        assert_eq!(host_dir(Path::new("a/t.host")), Path::new("a/t.host.host"));
    }

    #[test]
    fn writes_what_differs_from_the_shipped_game_and_the_table() {
        let mut host = gm::Host::default();
        host.files
            .insert("stats.txt".into(), GmString::from("SPELUNKY\r\n\"x\""));
        host.files
            .insert("levels/a.lvl".into(), GmString::from("1"));
        host.highscores[0] = (GmString::from("MONEY \"q\""), 9000100.0);
        host.highscores[1] = (GmString::from("TIME"), f64::INFINITY);
        let dir = tmp("write");
        write_host(&host, &dir).unwrap();
        let files = read_files(&dir).unwrap();
        // The shipped .cfg files aren't written.
        assert_eq!(
            files.keys().collect::<Vec<_>>(),
            ["levels/a.lvl", "stats.txt"]
        );
        assert_eq!(files["stats.txt"], b"SPELUNKY\r\n\"x\"");
        let table = std::fs::read_to_string(dir.join(HIGHSCORES)).unwrap();
        let lines: Vec<&str> = table.lines().collect();
        // As the oracle writes them (patch 0009).
        assert_eq!(lines[0], r#"{"name":"MONEY \"q\"","value":9000100.0}"#);
        assert_eq!(lines[1], r#"{"name":"TIME","value":"inf"}"#);
        assert_eq!(lines[2], r#"{"name":"<nobody>","value":0.0}"#);
        assert_eq!(lines.len(), 10);
        let read = read_highscores(&dir).unwrap();
        assert_eq!(read[0].name, "MONEY \"q\"");
        assert_eq!(read[0].value, Real(9000100.0));
        assert_eq!(read[1].value, Real(f64::INFINITY));
        assert!(diff_host(&dir, &dir).unwrap().is_identical());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn reports_files_and_places_that_differ() {
        let mut a = gm::Host::default();
        a.files.insert("stats.txt".into(), GmString::from("abc"));
        a.highscores[2].1 = 1.0;
        let mut b = gm::Host::default();
        b.files.insert("stats.txt".into(), GmString::from("abd!"));
        b.files.insert("keys.cfg".into(), GmString::from("1"));
        b.highscores[2].1 = 1.0 + f64::EPSILON;
        let (da, db) = (tmp("a"), tmp("b"));
        write_host(&a, &da).unwrap();
        write_host(&b, &db).unwrap();
        let r = diff_host(&da, &db).unwrap();
        assert_eq!(r.files_compared, 2);
        assert_eq!(
            r.files,
            [
                FileMismatch {
                    path: "keys.cfg".into(),
                    oracle_bytes: None,
                    port_bytes: Some(1),
                    first_difference: None,
                },
                FileMismatch {
                    path: "stats.txt".into(),
                    oracle_bytes: Some(3),
                    port_bytes: Some(4),
                    first_difference: Some(2),
                },
            ]
        );
        assert_eq!(r.highscores.len(), 1);
        assert_eq!(r.highscores[0].place, 3);
        assert!(r.to_text().contains("highscore 3"));
        std::fs::remove_dir_all(&da).unwrap();
        std::fs::remove_dir_all(&db).unwrap();
    }
}
