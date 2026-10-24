//! A stand-in for the app in the end-to-end tests (`tests/e2e.rs`): it checks, downloads, swaps and
//! restarts through the same calls as `app::update`, but its version comes from beside it, so copies of
//! this one binary are the "older" and "newer" builds:
//!
//! - an AppImage's version is a `FIXTURE-VERSION=...` line appended to the file (on Linux: macOS won't
//!   run a signed binary with something appended);
//! - otherwise it's `version.txt` beside the executable, or in the bundle's `Contents/Resources`.
//!
//! A version followed by `broken` quits before it starts; by `hangs`, never starts.
//!
//! ```sh
//! update-fixture version
//! update-fixture update MANIFEST-URL PUBLIC-KEY-FILE PLATFORM DATA-DIR REPORT TIMEOUT-SECS [--dir]
//! update-fixture report REPORT
//! ```
//!
//! `update` is the app's "restart and update" (with the player saying yes); the version it starts runs
//! `report`, which appends its version to REPORT and confirms it started. `--dir` makes it a directory
//! install even on macOS.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;
use update::check::{self, Found, Source};
use update::install::{Install, Kind};
use update::restart::{self, Outcome};
use update::state::State;

fn main() {
    match run() {
        Ok(()) => {}
        Err(e) => {
            println!("error: {e}");
            std::process::exit(2);
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let appimage = std::env::var_os("APPIMAGE").map(PathBuf::from);
    let (version, how) = version(&exe, appimage.as_deref())?;
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["version"] => {
            println!("{version}");
            Ok(())
        }
        ["report", report] => {
            match how.as_deref() {
                Some("broken") => std::process::exit(3),
                Some("hangs") => loop {
                    std::thread::sleep(Duration::from_secs(1));
                },
                _ => {}
            }
            let mut line = std::fs::read_to_string(report).unwrap_or_default();
            line.push_str(&format!("{version}\n"));
            std::fs::write(report, line).map_err(|e| format!("{report}: {e}"))?;
            restart::confirm_started();
            Ok(())
        }
        [
            "update",
            url,
            key_file,
            platform,
            data,
            report,
            timeout,
            rest @ ..,
        ] => {
            let key = std::fs::read_to_string(key_file).map_err(|e| format!("{key_file}: {e}"))?;
            let mut install = Install::detect(&exe, appimage.as_deref());
            if rest == ["--dir"] {
                install = Ok(Install {
                    kind: Kind::Dir,
                    target: exe.parent().unwrap().to_owned(),
                    exe: exe.clone(),
                });
            }
            let source = Source {
                manifest_url: url.to_string(),
                key: update::key::public_key(&key)?,
                current: update::Version::parse(&version)?,
                platform: platform.to_string(),
                install,
            };
            let state_path = Path::new(data).join("update.json");
            match check::check(&source, None, |_| {})? {
                Found::UpToDate => println!("up to date"),
                Found::Newer {
                    version, why_not, ..
                } => println!("newer {version}: {why_not}"),
                Found::Ready(ready) => {
                    let args: Vec<OsString> = vec!["report".into(), report.into()];
                    let timeout = Duration::from_secs(timeout.parse().map_err(|_| "bad timeout")?);
                    match restart::install_and_restart(&ready, &args, Some(&state_path), timeout)? {
                        Outcome::Started => println!("updated to {}", ready.version),
                        Outcome::RolledBack(why) => {
                            let failed = State::load(&state_path).failed.map(|f| f.version);
                            println!("rolled back from {failed:?}: {why}");
                        }
                    }
                }
            }
            Ok(())
        }
        _ => Err("usage: see src/bin/update-fixture.rs".into()),
    }
}

/// The version, and what follows it (`broken`, `hangs`).
fn version(exe: &Path, appimage: Option<&Path>) -> Result<(String, Option<String>), String> {
    // An AppImage is its own file, which the version it restarts into runs as without $APPIMAGE (the real
    // runtime sets it again; this stand-in has no runtime).
    let file = appimage.unwrap_or(exe);
    let bytes = std::fs::read(file).map_err(|e| format!("{}: {e}", file.display()))?;
    let tail = String::from_utf8_lossy(&bytes[bytes.len().saturating_sub(64)..]).into_owned();
    let text = match tail.rsplit_once("FIXTURE-VERSION=") {
        Some((_, version)) => version.to_owned(),
        None if appimage.is_some() => return Err("the AppImage has no FIXTURE-VERSION line".into()),
        None => {
            let dir = exe.parent().unwrap();
            let beside = dir.join("version.txt");
            let path = if beside.exists() {
                beside
            } else {
                dir.join("../Resources/version.txt")
            };
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?
        }
    };
    let mut words = text.split_whitespace();
    let version = words.next().ok_or("no version")?.to_owned();
    Ok((version, words.next().map(str::to_owned)))
}
