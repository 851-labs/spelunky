//! The file and highscore built-ins (851-3164), over the host's virtual game directory and highscore table.
//! OpenGMK `gml/kernel.rs` (`file_*`), `gml/file.rs` (`TextHandle`) and `handleman.rs` (`HandleArray`), and
//! the oracle's own highscore table (`tools/oracle/patches/opengmk/0004-highscore-table.patch`).
//!
//! The game reads and writes `stats.txt`, the three `.cfg` files and `levels/*.lvl` with these
//! (`docs/source-inventory.md` section 7.2). They only ever see [`Host::files`], so a run is the same
//! wherever it runs; persisting the directory is up to the frontend (`app::save`).
//!
//! # Rules, as OpenGMK and the oracle implement them
//!
//! - **Paths** are relative to the game directory. `\` and `/` both separate (OpenGMK `file::to_path`
//!   turns `\` into `/`), and `.` and empty components are dropped, as the OS would resolve them. A path
//!   that leaves the directory names no file. Names are case-sensitive, as on the oracle's Linux, where
//!   GM8 on Windows isn't; the game always uses the same (lower) case for a file. Every directory exists:
//!   the game only writes to the top level and `levels/`, which the game ships with.
//! - **Handles** are 1 to 32: an open takes the lowest free one. A 33rd open file is an error.
//! - **`file_text_open_read`** of a missing file returns -1 (the game tests `if (file)`, and -1 is false).
//!   The contents are read when it opens.
//! - **`file_text_open_write`** creates or truncates the file at once. What's written appears in the
//!   directory when the file is closed (OpenGMK buffers it; the game never reads a file it's writing).
//! - **`file_text_read_string`** returns the rest of the line and stops *before* its end: before the
//!   `\r\n`, or the `\n` of a bare-LF line. A `\r` not followed by `\n` is part of the line. So reading
//!   twice without `file_text_readln` returns the empty string the second time.
//! - **`file_text_readln`** skips past the next `\n` (or to the end of the file).
//! - **`file_text_eof`** is true once no bytes are left; a trailing newline isn't skipped for you.
//!   Reading at the end returns `""`, and `readln` does nothing.
//! - **Writing**: `file_text_write_string` writes the string's bytes as they are; `file_text_writeln`
//!   writes `\r\n`, as GM8 on Windows does.
//! - **Errors**: reading or writing through a handle that isn't open, or reading a file open for writing
//!   (and the reverse), stops the game, naming the function. `file_text_close` of a handle in 1..=32
//!   that isn't open does nothing, as GM8 does (OpenGMK's comment); OpenGMK itself errors, but the game
//!   never does it.
//! - **`file_find_first(mask, attr)`** lists the files in the mask's directory whose names match its
//!   `*`/`?` pattern, ignoring case as Windows does, in Windows' (case-insensitive) order; it returns the
//!   first name (without the directory) or `""`, and `file_find_next` the next. There are no
//!   directories or read-only files to filter by `attr`. The oracle differs: OpenGMK globs the mask
//!   without converting its `\`, so on Linux the game's `levels\*.lvl` never matches anything
//!   (`docs/quirks.md`).
//! - **Highscores**: `highscore_add` inserts above the first entry with a strictly lower value, keeping
//!   ten; `highscore_value(place)` is 1-based, and -1 outside 1..=10; `highscore_clear` resets the table
//!   to ten `<nobody>` entries scoring 0.

use crate::host::{HIGHSCORE_ENTRIES, Host};
use crate::string::GmString;
use serde::{Deserialize, Serialize};
use std::fmt;

/// How many text files GM 8 has open at once: handles are 1 to 32 (OpenGMK `game.rs`, `text_files`).
pub const TEXT_FILE_SLOTS: usize = 32;

/// An open text file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) enum TextFile {
    /// Open for reading: the contents when it was opened, and how far it's been read.
    Read { contents: GmString, pos: usize },
    /// Open for writing: the file's path and what's been written so far.
    Write { path: String, written: Vec<u8> },
}

/// Why a file built-in failed. GM8 shows these as errors; the port stops the tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileError {
    /// No file is open with this handle.
    BadHandle(i32),
    /// The file is open for writing, not reading.
    NotForReading,
    /// The file is open for reading, not writing.
    NotForWriting,
    /// All [`TEXT_FILE_SLOTS`] handles are taken.
    TooManyFiles,
    /// The path leaves the game directory (`..`), so there's no file to write.
    BadPath,
}

impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileError::BadHandle(h) => write!(f, "invalid file handle {h}"),
            FileError::NotForReading => f.write_str("file is not open for reading"),
            FileError::NotForWriting => f.write_str("file is not open for writing"),
            FileError::TooManyFiles => f.write_str("handle limit reached"),
            FileError::BadPath => f.write_str("the path is outside the game directory"),
        }
    }
}

/// A GML path as a key of [`Host::files`]: `/`-separated, with no empty, `.` or `..` components. `None`
/// for a path that leaves the game directory or names it. The bytes are Latin-1 (the game's names are
/// ASCII).
pub fn game_path(path: &[u8]) -> Option<String> {
    let path: String = path.iter().map(|&b| char::from(b)).collect();
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split(['/', '\\']) {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            _ => parts.push(part),
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// Whether `name` matches a Windows `*`/`?` pattern, ignoring ASCII case.
fn wildcard_match(pattern: &[u8], name: &[u8]) -> bool {
    match pattern.split_first() {
        None => name.is_empty(),
        Some((b'*', rest)) => (0..=name.len()).any(|skip| wildcard_match(rest, &name[skip..])),
        Some((&p, rest)) => name.split_first().is_some_and(|(&c, name)| {
            (p == b'?' || p.eq_ignore_ascii_case(&c)) && wildcard_match(rest, name)
        }),
    }
}

impl Host {
    fn text_file(&mut self, handle: i32) -> Result<&mut TextFile, FileError> {
        usize::try_from(handle - 1)
            .ok()
            .and_then(|i| self.text_files.get_mut(i))
            .and_then(Option::as_mut)
            .ok_or(FileError::BadHandle(handle))
    }

    /// Reads through `handle`: the contents and position of a file open for reading.
    fn reader(&mut self, handle: i32) -> Result<(&[u8], &mut usize), FileError> {
        match self.text_file(handle)? {
            TextFile::Read { contents, pos } => Ok((contents.as_bytes(), pos)),
            TextFile::Write { .. } => Err(FileError::NotForReading),
        }
    }

    fn writer(&mut self, handle: i32) -> Result<&mut Vec<u8>, FileError> {
        match self.text_file(handle)? {
            TextFile::Write { written, .. } => Ok(written),
            TextFile::Read { .. } => Err(FileError::NotForWriting),
        }
    }

    /// Opens `file` in the lowest free slot, returning its handle.
    fn open(&mut self, file: TextFile) -> Result<i32, FileError> {
        let slot = self
            .text_files
            .iter()
            .position(Option::is_none)
            .ok_or(FileError::TooManyFiles)?;
        self.text_files[slot] = Some(file);
        Ok(slot as i32 + 1)
    }

    /// `file_exists(name)`.
    pub fn file_exists(&self, name: &[u8]) -> bool {
        game_path(name).is_some_and(|p| self.files.contains_key(&p))
    }

    /// `file_delete(name)`: nothing happens if there's no such file.
    pub fn file_delete(&mut self, name: &[u8]) {
        if let Some(p) = game_path(name) {
            self.files.remove(&p);
        }
    }

    /// `file_find_first(mask, attr)`: see the [module docs](self). `attr` only filters directories and
    /// read-only files, which the virtual directory doesn't have.
    pub fn file_find_first(&mut self, mask: &[u8], _attr: i32) -> GmString {
        self.found.clear();
        // OpenGMK: a mask ending in a separator matches nothing.
        if !matches!(mask.last(), None | Some(b'/' | b'\\')) {
            let split = mask.iter().rposition(|&b| b == b'/' || b == b'\\');
            let (dir, pattern) = match split {
                Some(i) => (game_path(&mask[..i]), &mask[i + 1..]),
                None => (Some(String::new()), mask),
            };
            if let Some(dir) = dir {
                let prefix = if dir.is_empty() { dir } else { dir + "/" };
                let mut names: Vec<&str> = self
                    .files
                    .keys()
                    .filter_map(|p| p.strip_prefix(&prefix))
                    .filter(|name| !name.contains('/'))
                    .filter(|name| wildcard_match(pattern, &latin1(name)))
                    .collect();
                // NTFS lists a directory in upper-cased order.
                names.sort_by_key(|n| (n.to_ascii_uppercase(), *n));
                self.found = names.into_iter().rev().map(|n| latin1(n).into()).collect();
            }
        }
        self.file_find_next()
    }

    /// `file_find_next()`: the next name [`file_find_first`](Self::file_find_first) found, or `""`.
    pub fn file_find_next(&mut self) -> GmString {
        self.found.pop().unwrap_or_default()
    }

    /// `file_text_open_read(name)`: a handle, or -1 if there's no such file.
    pub fn file_text_open_read(&mut self, name: &[u8]) -> Result<i32, FileError> {
        match game_path(name).and_then(|p| self.files.get(&p)) {
            Some(contents) => {
                let contents = contents.clone();
                self.open(TextFile::Read { contents, pos: 0 })
            }
            None => Ok(-1),
        }
    }

    /// `file_text_open_write(name)`: creates or truncates the file, returning a handle.
    pub fn file_text_open_write(&mut self, name: &[u8]) -> Result<i32, FileError> {
        let path = game_path(name).ok_or(FileError::BadPath)?;
        let handle = self.open(TextFile::Write {
            path: path.clone(),
            written: Vec::new(),
        })?;
        self.files.insert(path, GmString::default());
        Ok(handle)
    }

    /// `file_text_close(handle)`: what was written appears in the directory.
    pub fn file_text_close(&mut self, handle: i32) -> Result<(), FileError> {
        let slot = usize::try_from(handle - 1)
            .ok()
            .and_then(|i| self.text_files.get_mut(i))
            .ok_or(FileError::BadHandle(handle))?;
        if let Some(TextFile::Write { path, written }) = slot.take() {
            self.files.insert(path, written.into());
        }
        Ok(())
    }

    /// `file_text_eof(handle)`.
    pub fn file_text_eof(&mut self, handle: i32) -> Result<bool, FileError> {
        let (contents, pos) = self.reader(handle)?;
        Ok(*pos >= contents.len())
    }

    /// `file_text_read_string(handle)`: the rest of the line, stopping before its `\r\n` or `\n`.
    pub fn file_text_read_string(&mut self, handle: i32) -> Result<GmString, FileError> {
        let (contents, pos) = self.reader(handle)?;
        let rest = &contents[*pos..];
        let end = match rest.iter().position(|&b| b == b'\n') {
            Some(lf) if lf > 0 && rest[lf - 1] == b'\r' => lf - 1,
            Some(lf) => lf,
            None => rest.len(),
        };
        let line = GmString::from(&rest[..end]);
        *pos += end;
        Ok(line)
    }

    /// `file_text_readln(handle)`: skips past the next `\n`.
    pub fn file_text_readln(&mut self, handle: i32) -> Result<(), FileError> {
        let (contents, pos) = self.reader(handle)?;
        let rest = &contents[*pos..];
        *pos += rest
            .iter()
            .position(|&b| b == b'\n')
            .map_or(rest.len(), |lf| lf + 1);
        Ok(())
    }

    /// `file_text_write_string(handle, s)`.
    pub fn file_text_write_string(&mut self, handle: i32, s: &[u8]) -> Result<(), FileError> {
        self.writer(handle)?.extend_from_slice(s);
        Ok(())
    }

    /// `file_text_writeln(handle)`: a Windows line end.
    pub fn file_text_writeln(&mut self, handle: i32) -> Result<(), FileError> {
        self.writer(handle)?.extend_from_slice(b"\r\n");
        Ok(())
    }

    /// `highscore_add(name, value)`: inserts above the first entry with a strictly lower value (so
    /// after any equal ones) and drops the eleventh. A value no higher than the last entry's isn't added.
    pub fn highscore_add(&mut self, name: GmString, value: f64) {
        if let Some(i) = self.highscores.iter().position(|&(_, v)| value > v) {
            self.highscores.insert(i, (name, value));
            self.highscores.truncate(HIGHSCORE_ENTRIES);
        }
    }

    /// `highscore_value(place)`: the value at 1-based `place`, or -1 outside the table.
    pub fn highscore_value(&self, place: i32) -> f64 {
        usize::try_from(place - 1)
            .ok()
            .and_then(|i| self.highscores.get(i))
            .map_or(-1.0, |&(_, v)| v)
    }

    /// `highscore_clear()`.
    pub fn highscore_clear(&mut self) {
        self.highscores = Host::fresh_highscores();
    }

    /// Whether the open files are ones a host could have: reading positions within their files.
    pub(crate) fn files_are_consistent(&self) -> bool {
        self.text_files.iter().flatten().all(|f| match f {
            TextFile::Read { contents, pos } => *pos <= contents.len(),
            TextFile::Write { .. } => true,
        })
    }
}

/// A path's Latin-1 bytes (the inverse of [`game_path`]'s decoding).
fn latin1(s: &str) -> Vec<u8> {
    s.chars().map(|c| u8::try_from(c).unwrap_or(b'?')).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host_with(files: &[(&str, &str)]) -> Host {
        let mut h = Host::default();
        h.files.clear();
        for (p, c) in files {
            h.files.insert((*p).into(), (*c).into());
        }
        h
    }

    fn read_all(h: &mut Host, f: i32) -> Vec<String> {
        let mut out = Vec::new();
        while !h.file_text_eof(f).unwrap() {
            out.push(h.file_text_read_string(f).unwrap().to_string());
            h.file_text_readln(f).unwrap();
        }
        out
    }

    #[test]
    fn paths_normalize_like_the_os() {
        assert_eq!(game_path(b"stats.txt").as_deref(), Some("stats.txt"));
        assert_eq!(game_path(b"levels\\a.lvl").as_deref(), Some("levels/a.lvl"));
        assert_eq!(
            game_path(b"./levels//a.lvl").as_deref(),
            Some("levels/a.lvl")
        );
        assert_eq!(
            game_path(b"levels/../keys.cfg").as_deref(),
            Some("keys.cfg")
        );
        assert_eq!(game_path(b"../stats.txt"), None);
        assert_eq!(game_path(b""), None);
        assert_eq!(game_path(b"."), None);
    }

    #[test]
    fn lines_end_at_crlf_or_lf() {
        let mut h = host_with(&[("a", "one\r\ntwo\nthr\ree\r\n\r\nlast")]);
        let f = h.file_text_open_read(b"a").unwrap();
        assert_eq!(f, 1);
        assert_eq!(read_all(&mut h, f), ["one", "two", "thr\ree", "", "last"]);
        // At the end, reading gives "" and readln does nothing.
        assert_eq!(h.file_text_read_string(f).unwrap().as_bytes(), b"");
        h.file_text_readln(f).unwrap();
        assert!(h.file_text_eof(f).unwrap());
    }

    #[test]
    fn read_string_stops_before_the_line_end() {
        let mut h = host_with(&[("a", "x\r\ny")]);
        let f = h.file_text_open_read(b"a").unwrap();
        assert_eq!(h.file_text_read_string(f).unwrap().as_bytes(), b"x");
        // Still before the CRLF: a second read gives "", and eof is false.
        assert_eq!(h.file_text_read_string(f).unwrap().as_bytes(), b"");
        assert!(!h.file_text_eof(f).unwrap());
        h.file_text_readln(f).unwrap();
        assert_eq!(h.file_text_read_string(f).unwrap().as_bytes(), b"y");
        assert!(h.file_text_eof(f).unwrap());
    }

    #[test]
    fn a_trailing_newline_leaves_eof_false_until_readln() {
        let mut h = host_with(&[("a", "1\r\n")]);
        let f = h.file_text_open_read(b"a").unwrap();
        assert_eq!(h.file_text_read_string(f).unwrap().as_bytes(), b"1");
        assert!(!h.file_text_eof(f).unwrap());
        h.file_text_readln(f).unwrap();
        assert!(h.file_text_eof(f).unwrap());
    }

    #[test]
    fn handles_take_the_lowest_free_slot() {
        let mut h = host_with(&[("a", "")]);
        assert_eq!(h.file_text_open_read(b"missing").unwrap(), -1);
        let handles: Vec<i32> = (0..TEXT_FILE_SLOTS)
            .map(|_| h.file_text_open_read(b"a").unwrap())
            .collect();
        assert_eq!(handles, (1..=32).collect::<Vec<_>>());
        assert_eq!(h.file_text_open_read(b"a"), Err(FileError::TooManyFiles));
        h.file_text_close(5).unwrap();
        h.file_text_close(2).unwrap();
        assert_eq!(h.file_text_open_write(b"b").unwrap(), 2);
        assert_eq!(h.file_text_open_read(b"a").unwrap(), 5);
    }

    #[test]
    fn bad_handles_are_errors() {
        let mut h = host_with(&[("a", "x")]);
        assert_eq!(h.file_text_read_string(1), Err(FileError::BadHandle(1)));
        assert_eq!(h.file_text_eof(0), Err(FileError::BadHandle(0)));
        assert_eq!(h.file_text_writeln(-1), Err(FileError::BadHandle(-1)));
        // Closing an unopened handle in range is fine; out of range it's an error.
        assert_eq!(h.file_text_close(32), Ok(()));
        assert_eq!(h.file_text_close(33), Err(FileError::BadHandle(33)));
        assert_eq!(h.file_text_close(0), Err(FileError::BadHandle(0)));
        let r = h.file_text_open_read(b"a").unwrap();
        let w = h.file_text_open_write(b"b").unwrap();
        assert_eq!(h.file_text_writeln(r), Err(FileError::NotForWriting));
        assert_eq!(h.file_text_readln(w), Err(FileError::NotForReading));
        assert_eq!(h.file_text_eof(w), Err(FileError::NotForReading));
        assert_eq!(h.file_text_open_write(b".."), Err(FileError::BadPath));
    }

    #[test]
    fn writes_appear_on_close_with_crlf() {
        let mut h = host_with(&[("out.txt", "old")]);
        let f = h.file_text_open_write(b"out.txt").unwrap();
        // Truncated at once.
        assert_eq!(h.files["out.txt"].as_bytes(), b"");
        h.file_text_write_string(f, b"a").unwrap();
        h.file_text_writeln(f).unwrap();
        h.file_text_write_string(f, b"b").unwrap();
        assert_eq!(h.files["out.txt"].as_bytes(), b"");
        h.file_text_close(f).unwrap();
        assert_eq!(h.files["out.txt"].as_bytes(), b"a\r\nb");
        // The handle is free again.
        assert_eq!(h.file_text_close(f), Ok(()));
        assert_eq!(h.file_text_writeln(f), Err(FileError::BadHandle(f)));
    }

    #[test]
    fn a_reader_keeps_the_contents_it_opened() {
        let mut h = host_with(&[("a", "old")]);
        let r = h.file_text_open_read(b"a").unwrap();
        h.file_delete(b"a");
        assert!(!h.file_exists(b"a"));
        assert_eq!(h.file_text_read_string(r).unwrap().as_bytes(), b"old");
        h.file_delete(b"a");
    }

    #[test]
    fn find_lists_matching_names_in_windows_order() {
        let mut h = host_with(&[
            ("levels/b.lvl", ""),
            ("levels/A.LVL", ""),
            ("levels/c.lvl.bak", ""),
            ("levels/sub/d.lvl", ""),
            ("levels/test.tmp", ""),
            ("e.lvl", ""),
        ]);
        let mut names = vec![h.file_find_first(b"levels\\*.lvl", 0)];
        loop {
            let next = h.file_find_next();
            if next.is_empty() {
                break;
            }
            names.push(next);
        }
        let names: Vec<String> = names.iter().map(GmString::to_string).collect();
        assert_eq!(names, ["A.LVL", "b.lvl"]);
        assert_eq!(h.file_find_next().as_bytes(), b"");
        assert_eq!(h.file_find_first(b"?.lvl", 0).as_bytes(), b"e.lvl");
        assert_eq!(h.file_find_first(b"levels\\", 0).as_bytes(), b"");
        assert_eq!(h.file_find_first(b"nowhere\\*", 0).as_bytes(), b"");
    }

    #[test]
    fn the_highscore_table_keeps_ten_best_first() {
        let mut h = Host::default();
        assert_eq!(h.highscore_value(1), 0.0);
        assert_eq!(h.highscore_value(0), -1.0);
        assert_eq!(h.highscore_value(11), -1.0);
        // Equal values go after the existing ones; nothing beats a full table's last entry with less.
        h.highscore_add("a".into(), 5.0);
        h.highscore_add("b".into(), 5.0);
        h.highscore_add("c".into(), 7.0);
        let names: Vec<String> = h.highscores[..3]
            .iter()
            .map(|(n, _)| n.to_string())
            .collect();
        assert_eq!(names, ["c", "a", "b"]);
        h.highscore_add("zero".into(), 0.0);
        assert_eq!(h.highscores[3].0.as_bytes(), b"<nobody>");
        assert_eq!(h.highscores.len(), HIGHSCORE_ENTRIES);
        // The game's stats table: ten adds in descending order fill it exactly.
        h.highscore_clear();
        for i in 0..10 {
            h.highscore_add("S".into(), f64::from(9 - i) * 1e6 + 1.0);
        }
        assert_eq!(h.highscore_value(1), 9e6 + 1.0);
        assert_eq!(h.highscore_value(10), 1.0);
        h.highscore_clear();
        assert_eq!(h, Host::default());
    }
}
