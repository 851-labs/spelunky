//! The native app's last word: the error it stops with, or its panic, on stderr as ever, and in a message
//! box where nobody would see stderr.
//!
//! Windows' release build is a GUI program (`main.rs`), so opened from Explorer it has no stderr at all, and
//! a macOS app opened from Finder has `/dev/null` for one. Without a dialog, a game that can't start (no
//! Direct3D 12 or Vulkan adapter, the assets moved, a panic) looks to the player as if nothing happened.
//! So when stderr goes nowhere, the message goes in a native message box too: `MessageBoxW` on Windows,
//! an `NSAlert` on macOS. Elsewhere (Linux, with no toolkit in the app to draw one), and whenever stderr
//! goes somewhere (a terminal, a pipe, a file), only stderr has it.
//!
//! A dialog never opens for a headless run or a replay (`--screenshot`, `--frames`, `--capture-from`,
//! `--replay`): those are automation's (the release smoke tests, the fidelity tools), and a dialog nobody
//! can click away would hang it.
//!
//! `SPELUNKY_DIALOG_LOG=FILE` writes the dialog's text to FILE in place of showing it, and treats stderr
//! as going nowhere: CI's runners always have one, so that's how the smoke tests
//! (`scripts/release/smoke.py`) see what a player would. Without a dialog (Linux) it writes nothing.

use std::ffi::OsString;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// The dialog's title.
const TITLE: &str = "Spelunky Classic";

/// The flags that are only for a headless run or a replay, and so mean nobody is there to see a dialog.
const AUTOMATED: &[&str] = &["--screenshot", "--frames", "--capture-from", "--replay"];

/// The exit code of a failed run, and of a panic (Rust's own, which a panicking `main` exits with).
const FAILED: i32 = 1;
const PANICKED: i32 = 101;

/// Why a run stopped: what stderr shows (and a dialog would), and the exit code.
#[derive(Debug, PartialEq)]
struct Failure {
    message: String,
    code: i32,
}

/// Runs the app and returns its exit code, reporting its error or main-thread panic on stderr and, where
/// stderr goes nowhere, in a dialog. `args` is the command line, without the program, to tell an automated
/// run by: it's read raw because the error may be that it doesn't parse.
pub fn run(args: &[String], app: impl FnOnce() -> Result<(), String>) -> i32 {
    let Err(failure) = catch(app) else {
        return 0;
    };
    // A panic's message is on stderr already, from the default hook.
    if failure.code != PANICKED {
        eprintln!("{}", failure.message);
    }
    if !HAS_DIALOG {
        return failure.code;
    }
    match dialog(args, std::env::var_os("SPELUNKY_DIALOG_LOG"), stderr_unseen) {
        Some(Dialog::Show) => show(&failure.message),
        Some(Dialog::Log(log)) => log_dialog(&log, &failure.message),
        None => {}
    }
    failure.code
}

/// Whether there's a dialog to show the error in. Linux has none: the app has no toolkit to draw one
/// with, and a desktop's own (zenity, kdialog) may not be there.
const HAS_DIALOG: bool = cfg!(any(windows, target_os = "macos"));

#[derive(Debug, PartialEq)]
enum Dialog {
    Show,
    /// `SPELUNKY_DIALOG_LOG`'s file.
    Log(PathBuf),
}

/// Whether a run with `args` gets a dialog, given `SPELUNKY_DIALOG_LOG`.
fn dialog(
    args: &[String],
    log: Option<OsString>,
    stderr_unseen: impl FnOnce() -> bool,
) -> Option<Dialog> {
    if args.iter().any(|a| AUTOMATED.contains(&a.as_str())) {
        None
    } else if let Some(log) = log {
        Some(Dialog::Log(log.into()))
    } else {
        stderr_unseen().then_some(Dialog::Show)
    }
}

/// Runs `app`, giving its error (as `app: ERROR`, the way the console has always shown it) or, if it
/// panics on this thread, the panic's message and Rust's panic exit code.
///
/// The panic is caught and the dialog shown after the unwind, not from the panic hook: by then the event
/// loop and its windows are gone, whereas a dialog's modal loop inside the hook could run the windows'
/// handlers again in the middle of the panic. Other threads' panics (sound, the update check) don't stop
/// the game, so they're left to the default hook alone.
fn catch(app: impl FnOnce() -> Result<(), String>) -> Result<(), Failure> {
    let message = Arc::new(Mutex::new(None));
    let this_thread = std::thread::current().id();
    let default_hook = panic::take_hook();
    let record = Arc::clone(&message);
    panic::set_hook(Box::new(move |info| {
        default_hook(info);
        let thread = std::thread::current();
        if thread.id() == this_thread {
            // What the default hook printed, but its note about RUST_BACKTRACE.
            let name = thread.name().unwrap_or("<unnamed>");
            *record.lock().unwrap_or_else(|e| e.into_inner()) =
                Some(format!("thread '{name}' {info}"));
        }
    }));
    match panic::catch_unwind(AssertUnwindSafe(app)) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(Failure {
            message: format!("app: {e}"),
            code: FAILED,
        }),
        Err(_) => {
            let message = message.lock().unwrap_or_else(|e| e.into_inner()).take();
            Err(Failure {
                message: message.unwrap_or_else(|| "app: panicked".into()),
                code: PANICKED,
            })
        }
    }
}

/// The dialog's text for `SPELUNKY_DIALOG_LOG`: the title, then the message.
fn log_dialog(log: &Path, message: &str) {
    if let Err(e) = std::fs::write(log, format!("{TITLE}\n{message}\n")) {
        eprintln!("app: {}: {e}", log.display());
    }
}

/// Whether stderr goes nowhere anyone sees: a Windows GUI program started without one (from Explorer,
/// or a terminal that didn't redirect it) has a null handle, and a debug build's console never is.
#[cfg(windows)]
fn stderr_unseen() -> bool {
    use std::os::windows::io::AsRawHandle;
    std::io::stderr().as_raw_handle().is_null()
}

/// Whether stderr goes nowhere anyone sees: launchd gives an app opened from Finder (or by `open`)
/// `/dev/null`, and the unified log only has what goes through `os_log`. Closed is nowhere too.
#[cfg(target_os = "macos")]
fn stderr_unseen() -> bool {
    use std::os::fd::AsFd;
    use std::os::unix::fs::MetadataExt;
    let Ok(stderr) = std::io::stderr().as_fd().try_clone_to_owned() else {
        return true;
    };
    let (Ok(stderr), Ok(null)) = (
        std::fs::File::from(stderr).metadata(),
        std::fs::metadata("/dev/null"),
    ) else {
        return false;
    };
    (stderr.dev(), stderr.ino()) == (null.dev(), null.ino())
}

// No dialog here (`HAS_DIALOG`).
#[cfg(not(any(windows, target_os = "macos")))]
fn stderr_unseen() -> bool {
    false
}

#[cfg(windows)]
fn show(message: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MB_ICONERROR, MB_OK, MB_SETFOREGROUND, MessageBoxW,
    };
    let wide = |s: &str| s.encode_utf16().chain([0]).collect::<Vec<u16>>();
    let (text, title) = (wide(message), wide(TITLE));
    // windows-sys only has the raw binding, and nothing that wraps MessageBoxW is worth a dependency.
    // Sound: both strings are NUL-terminated UTF-16 that outlive the call, and a null owner window is
    // allowed (the box is then the thread's own top-level window).
    #[allow(unsafe_code)]
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR | MB_SETFOREGROUND,
        );
    }
}

#[cfg(target_os = "macos")]
fn show(message: &str) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSAlert, NSAlertStyle, NSApplication, NSApplicationActivationPolicy};
    use objc2_foundation::NSString;
    // AppKit only runs on the main thread, which is where `run` is called from.
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    // A failure before the window opened leaves an app that isn't in the Dock or in front yet, and an
    // alert behind every other window.
    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
    // `activate` is macOS 14's; the game runs on 11.
    #[allow(deprecated)]
    app.activateIgnoringOtherApps(true);
    let alert = NSAlert::new(mtm);
    alert.setAlertStyle(NSAlertStyle::Critical);
    alert.setMessageText(&NSString::from_str(TITLE));
    alert.setInformativeText(&NSString::from_str(message));
    alert.runModal();
}

#[cfg(not(any(windows, target_os = "macos")))]
fn show(_message: &str) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_error_is_what_the_console_shows() {
        assert_eq!(catch(|| Ok(())), Ok(()));
        assert_eq!(
            catch(|| Err("assets/bundle.bin: not found".into())),
            Err(Failure {
                message: "app: assets/bundle.bin: not found".into(),
                code: 1,
            })
        );
    }

    #[test]
    fn a_panic_is_caught_with_its_message() {
        let failure = catch(|| panic!("no adapter")).unwrap_err();
        assert_eq!(failure.code, 101);
        assert!(
            failure.message.starts_with("thread '"),
            "{}",
            failure.message
        );
        assert!(
            failure.message.contains("panicked at "),
            "{}",
            failure.message
        );
        assert!(
            failure.message.ends_with(":\nno adapter"),
            "{}",
            failure.message
        );
    }

    #[test]
    fn another_threads_panic_is_not_this_ones() {
        let failure = catch(|| {
            let _ = std::thread::spawn(|| panic!("the sound thread")).join();
            panic!("the main thread")
        })
        .unwrap_err();
        assert!(
            failure.message.ends_with("the main thread"),
            "{}",
            failure.message
        );
    }

    #[test]
    fn a_dialog_only_where_stderr_goes_nowhere() {
        let args = |a: &[&str]| a.iter().map(|&a| a.to_owned()).collect::<Vec<_>>();
        let play = args(&[]);
        assert_eq!(dialog(&play, None, || true), Some(Dialog::Show));
        assert_eq!(dialog(&play, None, || false), None);
        let log = Some(OsString::from("dialog.txt"));
        assert_eq!(
            dialog(&play, log.clone(), || false),
            Some(Dialog::Log("dialog.txt".into()))
        );
        // A dialog nobody clicks away would hang automation, even where it would be logged.
        for automated in [
            args(&["--replay", "r.txt"]),
            args(&["--settings", "--frames", "60", "--screenshot", "s.png"]),
            args(&["--frames", "bad"]),
        ] {
            assert_eq!(dialog(&automated, None, || true), None, "{automated:?}");
            assert_eq!(
                dialog(&automated, log.clone(), || true),
                None,
                "{automated:?}"
            );
        }
    }
}
