//! Restarting into the new version, and rolling back if it doesn't start.
//!
//! The old version does the swap once its window has closed ([`swap_and_relaunch`]), then starts the new
//! one with [`CONFIRM_ENV`] naming a file, and waits. The new version writes that file once it's up (the
//! app: once its first frame is shown; [`confirm_started`]). If it exits first, or doesn't get there in
//! [`START_TIMEOUT`], the old version stops it, puts itself back and says so.
//!
//! The new version is on probation until then ([`on_probation`]): it leaves the work directory alone,
//! since the old version may still need what's in it.

use crate::check::Ready;
use crate::install::{Install, Kind, Staged};
use crate::state::{Failed, State};
use std::ffi::OsString;
use std::path::Path;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

/// The file a new version writes to say it started.
pub const CONFIRM_ENV: &str = "SPELUNKY_UPDATE_CONFIRM";

/// How long a new version has to start: far more than a slow machine needs to open its window.
pub const START_TIMEOUT: Duration = Duration::from_secs(60);

/// Whether this is a new version that its predecessor is waiting on.
pub fn on_probation() -> bool {
    std::env::var_os(CONFIRM_ENV).is_some()
}

/// Tells the predecessor (if one is waiting) that this version started. Call it once the app is up; more
/// calls do nothing more.
pub fn confirm_started() {
    if let Some(path) = std::env::var_os(CONFIRM_ENV) {
        let _ = std::fs::write(path, b"started\n");
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The new version is running. This one should exit.
    Started,
    /// The new version failed, for this reason, and this version is back in place.
    RolledBack(String),
}

/// What the app does once its window has closed to update: swaps in `ready` and starts it with `args`
/// ([`swap_and_relaunch`]). If the new version doesn't start, it's rolled back, the state at `state_path`
/// records why (for the player to hear about), and this version starts again with `args`.
pub fn install_and_restart(
    ready: &Ready,
    args: &[OsString],
    state_path: Option<&Path>,
    timeout: Duration,
) -> Result<Outcome, String> {
    let outcome = swap_and_relaunch(&ready.install, &ready.staged, args, timeout)?;
    if let Outcome::RolledBack(reason) = &outcome {
        if let Some(path) = state_path {
            let mut state = State::load(path);
            state.failed = Some(Failed {
                version: ready.version.to_string(),
                reason: reason.clone(),
            });
            let _ = state.save(path);
        }
        launch(&ready.install, args, None)?;
    }
    Ok(outcome)
}

/// Swaps in `staged` and starts it with `args`, waiting up to `timeout` for it to start. An error means the
/// swap itself failed, and nothing changed; a failed start is [`Outcome::RolledBack`].
pub fn swap_and_relaunch(
    install: &Install,
    staged: &Staged,
    args: &[OsString],
    timeout: Duration,
) -> Result<Outcome, String> {
    let applied = install.apply(staged)?;
    let token = install.work_dir().join("started");
    let _ = std::fs::remove_file(&token);
    let failure = match launch(install, args, Some(&token)) {
        Ok(mut child) => match watch(&mut child, &token, timeout) {
            Ok(()) => {
                install.clean_up();
                return Ok(Outcome::Started);
            }
            Err(e) => e,
        },
        Err(e) => e,
    };
    install
        .roll_back(applied)
        .map_err(|e| format!("{failure}; then putting the old version back failed: {e}"))?;
    Ok(Outcome::RolledBack(failure))
}

/// Starts the installed app with `args`, not waiting for it. With `token`, it's a new version on
/// probation.
pub fn launch(install: &Install, args: &[OsString], token: Option<&Path>) -> Result<Child, String> {
    let mut command = Command::new(&install.exe);
    command.args(args);
    match token {
        Some(token) => command.env(CONFIRM_ENV, token),
        None => command.env_remove(CONFIRM_ENV),
    };
    if install.kind == Kind::File {
        // The AppImage runtime set these for this process's mount, which goes away with it; the new
        // AppImage's runtime sets its own.
        let appdir = std::env::var_os("APPDIR");
        for var in ["APPIMAGE", "APPDIR", "ARGV0", "OWD"] {
            command.env_remove(var);
        }
        if let (Some(appdir), Some(paths)) = (appdir, std::env::var_os("LD_LIBRARY_PATH")) {
            let kept: Vec<_> = std::env::split_paths(&paths)
                .filter(|p| !p.starts_with(&appdir))
                .collect();
            match std::env::join_paths(kept) {
                Ok(p) if !p.is_empty() => command.env("LD_LIBRARY_PATH", p),
                _ => command.env_remove("LD_LIBRARY_PATH"),
            };
        }
    }
    command
        .spawn()
        .map_err(|e| format!("can't start {}: {e}", install.exe.display()))
}

/// Waits for `child` to write `token`.
fn watch(child: &mut Child, token: &Path, timeout: Duration) -> Result<(), String> {
    let started = Instant::now();
    loop {
        if token.exists() {
            return Ok(());
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                // It may have started, said so and quit in the moment between the two checks.
                if token.exists() {
                    return Ok(());
                }
                return Err(format!("the new version quit before it started ({status})"));
            }
            Ok(None) => {}
            Err(e) => return Err(format!("can't watch the new version: {e}")),
        }
        if started.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "the new version didn't start within {} seconds",
                timeout.as_secs()
            ));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
