//! Keeping the game directory and the highscore table between sessions of real play (851-3164).
//!
//! The game only ever sees its virtual directory and table ([`gm::Host`]), which a run starts from and
//! savestates carry, so the simulation stays deterministic. Persisting them is the frontend's job, at the
//! boundary: [`Persistence::open`] lays a [`Store`]'s saved files and table over the host the game boots
//! with, and [`Persistence::sync`], called after each tick, writes back what the game changed. The game
//! changes a file when it closes it (or opens it for writing, which empties it, or deletes it), and the
//! table on `highscore_add`/`highscore_clear`, so between those `sync` writes nothing.
//!
//! The frontend doesn't run the game yet (it previews rooms), so nothing calls this so far: the game loop
//! should open a [`Persistence`] on a store (`DirStore::default_dir()` natively) before booting with
//! `game::Config::default()`, and `sync` after every tick.
//!
//! Replays and the oracle never use a store: they start from the replay's files ([`gm::Host::from_replay`]).
//!
//! Stores: [`DirStore`] natively (a per-user data directory), `LocalStorage` on the web.

use std::collections::BTreeMap;
use std::io;

use gm::{GmString, HIGHSCORE_ENTRIES, Host};

/// What a store holds.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Saved {
    /// Game directory files by path (`/`-separated, as in [`Host::files`]).
    pub files: BTreeMap<String, Vec<u8>>,
    /// The highscore table, if one was saved.
    pub highscores: Option<Vec<(GmString, f64)>>,
}

/// Where the game directory and highscore table are kept between sessions.
pub trait Store {
    fn load(&self) -> io::Result<Saved>;
    fn write_file(&mut self, path: &str, contents: &[u8]) -> io::Result<()>;
    fn delete_file(&mut self, path: &str) -> io::Result<()>;
    fn write_highscores(&mut self, table: &[(GmString, f64)]) -> io::Result<()>;
}

/// A [`Store`] kept in step with a running game's host.
pub struct Persistence<S> {
    store: S,
    /// The host's files and table as the store last saw them.
    files: BTreeMap<String, GmString>,
    highscores: Vec<(GmString, f64)>,
}

impl<S: Store> Persistence<S> {
    /// Lays the store's files over `host`'s (the shipped `.cfg` files when booting with
    /// `game::Config::default()`) and takes its highscore table, if it has one.
    pub fn open(store: S, host: &mut Host) -> io::Result<Persistence<S>> {
        let saved = store.load()?;
        for (path, contents) in saved.files {
            // Only paths inside the game directory; anything else in the store isn't the game's.
            if gm::is_game_key(&path) {
                host.files.insert(path, contents.into());
            }
        }
        if let Some(table) = saved.highscores
            && table.len() == HIGHSCORE_ENTRIES
        {
            host.highscores = table;
        }
        Ok(Persistence {
            store,
            files: host.files.clone(),
            highscores: host.highscores.clone(),
        })
    }

    /// Writes what the game changed in `host` since the last call. A change the store fails to take
    /// doesn't hold up the others; it's tried again next time, and the first error is returned.
    pub fn sync(&mut self, host: &Host) -> io::Result<()> {
        let mut result = Ok(());
        let mut keep = |r: io::Result<()>| {
            let ok = r.is_ok();
            if result.is_ok() {
                result = r;
            }
            ok
        };
        for (path, contents) in &host.files {
            // A file's contents stay shared until the game rewrites it, so this is mostly a pointer check.
            let same = self.files.get(path).is_some_and(|old| {
                std::ptr::eq(old.as_bytes(), contents.as_bytes()) || old == contents
            });
            // Savestates can't hold paths outside the directory either; this keeps the store safe anyway.
            if !same
                && gm::is_game_key(path)
                && keep(self.store.write_file(path, contents.as_bytes()))
            {
                self.files.insert(path.clone(), contents.clone());
            }
        }
        let deleted: Vec<String> = self
            .files
            .keys()
            .filter(|p| !host.files.contains_key(*p))
            .cloned()
            .collect();
        for path in deleted {
            if keep(self.store.delete_file(&path)) {
                self.files.remove(&path);
            }
        }
        if self.highscores != host.highscores && keep(self.store.write_highscores(&host.highscores))
        {
            self.highscores = host.highscores.clone();
        }
        result
    }
}

/// The highscore table as text: one `value<TAB>name` line per entry, the value as Rust prints an `f64`
/// (exact when read back) and the name's bytes in hex (a name can hold any byte, line ends included).
pub fn highscores_to_text(table: &[(GmString, f64)]) -> String {
    table
        .iter()
        .map(|(name, value)| {
            let hex: String = name.as_bytes().iter().map(|b| format!("{b:02x}")).collect();
            format!("{value}\t{hex}\n")
        })
        .collect()
}

/// The inverse of [`highscores_to_text`]; `None` if the text isn't a table.
pub fn highscores_from_text(text: &str) -> Option<Vec<(GmString, f64)>> {
    let unhex = |hex: &str| -> Option<Vec<u8>> {
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok())
            .collect()
    };
    text.lines()
        .map(|line| {
            let (value, name) = line.split_once('\t')?;
            Some((unhex(name)?.into(), value.parse().ok()?))
        })
        .collect()
}

#[cfg(not(target_arch = "wasm32"))]
pub use dir::DirStore;

#[cfg(not(target_arch = "wasm32"))]
mod dir {
    use super::*;
    use std::path::{Path, PathBuf};

    /// A directory on disk: the game directory's files under `files/`, as the game names them, and the
    /// highscore table in `highscores.txt`.
    pub struct DirStore {
        root: PathBuf,
    }

    impl DirStore {
        pub fn new(root: impl Into<PathBuf>) -> DirStore {
            DirStore { root: root.into() }
        }

        /// The per-user data directory: `~/Library/Application Support/Spelunky Classic` on macOS,
        /// `%APPDATA%\Spelunky Classic` on Windows, `$XDG_DATA_HOME/spelunky-classic` (by default
        /// `~/.local/share/spelunky-classic`) elsewhere. `None` without a home directory.
        pub fn default_dir() -> Option<PathBuf> {
            let env = |k| {
                std::env::var_os(k)
                    .filter(|v| !v.is_empty())
                    .map(PathBuf::from)
            };
            if cfg!(target_os = "macos") {
                Some(env("HOME")?.join("Library/Application Support/Spelunky Classic"))
            } else if cfg!(windows) {
                Some(env("APPDATA")?.join("Spelunky Classic"))
            } else {
                let data =
                    env("XDG_DATA_HOME").or_else(|| Some(env("HOME")?.join(".local/share")))?;
                Some(data.join("spelunky-classic"))
            }
        }

        fn file(&self, path: &str) -> PathBuf {
            self.root.join("files").join(path)
        }

        /// Writes through a temporary file outside `files/` (the game has names of its own ending in
        /// `.tmp`), so a crash never leaves a half-written one.
        fn write(&self, to: &Path, contents: &[u8]) -> io::Result<()> {
            if let Some(dir) = to.parent() {
                std::fs::create_dir_all(dir)?;
            }
            let tmp = self.root.join("writing");
            std::fs::write(&tmp, contents)?;
            std::fs::rename(&tmp, to)
        }
    }

    fn walk(dir: &Path, prefix: &str, out: &mut BTreeMap<String, Vec<u8>>) -> io::Result<()> {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e),
        };
        for entry in entries {
            let entry = entry?;
            // Names the game couldn't have written (not valid Unicode) aren't its files.
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            let path = format!("{prefix}{name}");
            if entry.file_type()?.is_dir() {
                walk(&entry.path(), &format!("{path}/"), out)?;
            } else {
                out.insert(path, std::fs::read(entry.path())?);
            }
        }
        Ok(())
    }

    impl Store for DirStore {
        fn load(&self) -> io::Result<Saved> {
            let mut files = BTreeMap::new();
            walk(&self.root.join("files"), "", &mut files)?;
            let highscores = match std::fs::read_to_string(self.root.join("highscores.txt")) {
                Ok(text) => highscores_from_text(&text),
                Err(e) if e.kind() == io::ErrorKind::NotFound => None,
                Err(e) => return Err(e),
            };
            Ok(Saved { files, highscores })
        }

        fn write_file(&mut self, path: &str, contents: &[u8]) -> io::Result<()> {
            self.write(&self.file(path), contents)
        }

        fn delete_file(&mut self, path: &str) -> io::Result<()> {
            match std::fs::remove_file(self.file(path)) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
                _ => Ok(()),
            }
        }

        fn write_highscores(&mut self, table: &[(GmString, f64)]) -> io::Result<()> {
            let text = highscores_to_text(table);
            self.write(&self.root.join("highscores.txt"), text.as_bytes())
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use web::LocalStorage;

#[cfg(target_arch = "wasm32")]
mod web {
    use super::*;

    const FILES: &str = "spelunky/files/";
    const HIGHSCORES: &str = "spelunky/highscores";

    /// The browser's `localStorage`: each file under `spelunky/files/<path>`, its bytes as Latin-1 chars,
    /// and the table under `spelunky/highscores`.
    pub struct LocalStorage {
        storage: web_sys::Storage,
    }

    fn js(e: wasm_bindgen::JsValue) -> io::Error {
        io::Error::other(format!("localStorage: {e:?}"))
    }

    impl LocalStorage {
        /// The window's `localStorage`, if the browser allows it.
        pub fn new() -> io::Result<LocalStorage> {
            let storage = web_sys::window()
                .ok_or_else(|| io::Error::other("no window"))?
                .local_storage()
                .map_err(js)?
                .ok_or_else(|| io::Error::other("localStorage is unavailable"))?;
            Ok(LocalStorage { storage })
        }
    }

    impl Store for LocalStorage {
        fn load(&self) -> io::Result<Saved> {
            let mut saved = Saved::default();
            for i in 0..self.storage.length().map_err(js)? {
                let Some(key) = self.storage.key(i).map_err(js)? else {
                    continue;
                };
                let Some(value) = self.storage.get_item(&key).map_err(js)? else {
                    continue;
                };
                if let Some(path) = key.strip_prefix(FILES)
                    && let Some(bytes) = gm::string::from_latin1(&value)
                {
                    saved.files.insert(path.to_string(), bytes);
                } else if key == HIGHSCORES {
                    saved.highscores = highscores_from_text(&value);
                }
            }
            Ok(saved)
        }

        fn write_file(&mut self, path: &str, contents: &[u8]) -> io::Result<()> {
            self.storage
                .set_item(&format!("{FILES}{path}"), &gm::string::latin1(contents))
                .map_err(js)
        }

        fn delete_file(&mut self, path: &str) -> io::Result<()> {
            self.storage
                .remove_item(&format!("{FILES}{path}"))
                .map_err(js)
        }

        fn write_highscores(&mut self, table: &[(GmString, f64)]) -> io::Result<()> {
            self.storage
                .set_item(HIGHSCORES, &highscores_to_text(table))
                .map_err(js)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A store in memory that counts its writes.
    #[derive(Default)]
    struct Memory {
        saved: Saved,
        writes: usize,
    }

    impl Store for Memory {
        fn load(&self) -> io::Result<Saved> {
            Ok(self.saved.clone())
        }
        fn write_file(&mut self, path: &str, contents: &[u8]) -> io::Result<()> {
            self.writes += 1;
            self.saved.files.insert(path.into(), contents.into());
            Ok(())
        }
        fn delete_file(&mut self, path: &str) -> io::Result<()> {
            self.writes += 1;
            self.saved.files.remove(path);
            Ok(())
        }
        fn write_highscores(&mut self, table: &[(GmString, f64)]) -> io::Result<()> {
            self.writes += 1;
            self.saved.highscores = Some(table.to_vec());
            Ok(())
        }
    }

    #[test]
    fn the_store_follows_the_game() {
        let mut store = Memory::default();
        store
            .saved
            .files
            .insert("stats.txt".into(), b"old".to_vec());
        store.saved.files.insert("../outside".into(), b"x".to_vec());
        store
            .saved
            .files
            .insert("levels/\u{e9}.lvl".into(), b"x".to_vec());
        let mut host = Host::default();
        let mut p = Persistence::open(store, &mut host).unwrap();
        // The saved stats join the shipped config files; a path the game can't name doesn't.
        assert_eq!(host.files["stats.txt"].as_bytes(), b"old");
        assert!(host.files.contains_key("keys.cfg"));
        assert!(!host.files.contains_key("../outside"));
        // A level the game named `levels\é.lvl` (Latin-1 0xE9) comes back under the name it wrote.
        assert_eq!(host.file_text_open_read(b"levels\\\xe9.lvl"), Ok(1));
        host.file_text_close(1).unwrap();
        p.sync(&host).unwrap();
        assert_eq!(p.store.writes, 0);

        // The game rewrites the stats and a level, deletes the stats, and adds a highscore.
        let f = host.file_text_open_write(b"levels\\a.lvl").unwrap();
        host.file_text_write_string(f, b"level").unwrap();
        host.file_text_close(f).unwrap();
        host.file_delete(b"stats.txt");
        host.highscore_add("me".into(), 5.0);
        p.sync(&host).unwrap();
        let saved = &p.store.saved;
        assert_eq!(saved.files["levels/a.lvl"], b"level");
        assert!(!saved.files.contains_key("stats.txt"));
        assert_eq!(saved.highscores.as_deref(), Some(&host.highscores[..]));
        assert_eq!(p.store.writes, 3);
        p.sync(&host).unwrap();
        assert_eq!(p.store.writes, 3);

        // The next session starts where this one stopped.
        let mut next = Host::default();
        let store = std::mem::take(&mut p.store);
        Persistence::open(store, &mut next).unwrap();
        assert_eq!(next.files, host.files);
        assert_eq!(next.highscores, host.highscores);
    }

    #[test]
    fn highscores_round_trip_through_text() {
        let mut table = Host::fresh_highscores();
        table[0] = (GmString::from(&b"\xe9\tx\r\ny"[..]), 9_000_001.25);
        table[1].1 = 1e300;
        let text = highscores_to_text(&table);
        assert_eq!(highscores_from_text(&text), Some(table));
        assert_eq!(highscores_from_text("x"), None);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn a_directory_store_round_trips() {
        let root = std::env::temp_dir().join(format!("spelunky-save-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut store = DirStore::new(&root);
        assert_eq!(store.load().unwrap(), Saved::default());
        store.write_file("stats.txt", b"a\r\nb").unwrap();
        store.write_file("levels/test.tmp", b"t").unwrap();
        store.write_file("levels/x.lvl", b"x").unwrap();
        store.write_file("gone.txt", b"").unwrap();
        store.delete_file("gone.txt").unwrap();
        store.delete_file("never.txt").unwrap();
        let table = Host::fresh_highscores();
        store.write_highscores(&table).unwrap();
        let saved = store.load().unwrap();
        assert_eq!(saved.files.len(), 3);
        assert_eq!(saved.files["levels/test.tmp"], b"t");
        assert_eq!(saved.files["stats.txt"], b"a\r\nb");
        assert_eq!(saved.files["levels/x.lvl"], b"x");
        assert_eq!(saved.highscores, Some(table));
        std::fs::remove_dir_all(&root).unwrap();
    }
}
