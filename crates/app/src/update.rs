//! Updating the native app (851-3255), as the window's [`Overlay`]: the `update` crate checks, downloads,
//! swaps and rolls back; this decides when, and asks the player.
//!
//! - **When:** on launch, at most once a day, unless the player turned automatic checks off; and whenever
//!   the player asks: "Check for Updates…" in the macOS menu bar, or the updates menu F2 opens on Windows
//!   and Linux (whose window has no menu: a menu bar would take the Alt key from the game, and muda's
//!   Linux menus need GTK). Only between runs ([`Game::interruptible`](crate::window::Game::interruptible)):
//!   a check starts, and anything is shown, only on the title screen and the rooms it leads to. The
//!   frontend never gives the window an updater for a replay or a recording.
//! - **Then:** a newer version is downloaded and verified in the background, and only then offered. The
//!   player's "restart and update" closes the window; [`restart`] then swaps it in and starts it, and puts
//!   this version back if it doesn't start, which the next session tells the player about.
//!
//! The prompts are drawn over the game's picture in its own small font, like the notice, and take the
//! keys while they show: arrows choose, Enter takes, Esc closes.
//!
//! A build updates only if it's a release: built with `SPELUNKY_VERSION` (the release's version) and a real
//! release key (`update::key`). In a debug build, `SPELUNKY_VERSION`, `SPELUNKY_UPDATE_URL` and
//! `SPELUNKY_UPDATE_KEY` (a public key file) in the environment stand in for them, to try it out against a
//! local server.

use crate::notice::wrap;
use crate::window::Overlay;
use draw::{Color, Command, Rect, SpriteFont, Text};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc::{Receiver, TryRecvError};
use update::check::{self, Found, Message, Progress, Ready, Source};
use update::install::Install;
use update::state::{self, State};
use winit::keyboard::KeyCode;

/// The release's version, from the release build (`docs/updates.md`); none in a development build.
const VERSION: Option<&str> = option_env!("SPELUNKY_VERSION");

/// Where the update the player chose to restart into waits for the window to close.
pub type Handoff = Rc<RefCell<Option<Ready>>>;

const TITLE_COLOR: Color = Color::rgb(255, 216, 0);
const DIM: Color = Color::rgb(160, 160, 160);

/// What a choice in a dialog does.
#[derive(Clone, Debug, PartialEq)]
enum Choice {
    Restart,
    Later,
    Skip(String),
    Open(String),
    Check,
    ToggleAutomatic,
    Close,
}

#[derive(Clone, Debug)]
struct Dialog {
    title: String,
    lines: Vec<String>,
    choices: Vec<(String, Choice)>,
    selected: usize,
    /// It shows a check's progress, which the check's result replaces.
    checking: bool,
}

impl Dialog {
    fn new(title: &str, lines: Vec<String>, choices: Vec<(&str, Choice)>) -> Dialog {
        Dialog {
            title: title.to_owned(),
            lines,
            choices: choices
                .into_iter()
                .map(|(label, choice)| (label.to_owned(), choice))
                .collect(),
            selected: 0,
            checking: false,
        }
    }
}

/// A check in progress.
struct Running {
    messages: Receiver<Message>,
    /// The player asked for it, so it reports what it finds even if there's nothing.
    manual: bool,
}

pub struct Updater {
    /// What a check needs, or why this build can't check.
    source: Result<Source, String>,
    /// `update.json` in the data directory; `None` with `--no-save`, which keeps the state in memory.
    state_path: Option<PathBuf>,
    state: State,
    font: Option<(SpriteFont, (u32, u32))>,
    running: Option<Running>,
    /// An automatic check is due, from the first moment the player is between runs.
    auto_due: bool,
    dialog: Option<Dialog>,
    /// What to show once the player is between runs again, or the dialog showing is closed, in order.
    waiting: VecDeque<Dialog>,
    /// The download a "restart and update" installs.
    ready: Option<Ready>,
    handoff: Handoff,
    interruptible: bool,
    #[cfg(target_os = "macos")]
    menu: Option<mac::Menu>,
}

impl Updater {
    /// The updater for a session of play, keeping its state in `data` (the per-user data directory, if
    /// saving), and writing in the game's small `font`.
    pub fn new(data: Option<&Path>, font: Option<(SpriteFont, (u32, u32))>) -> (Updater, Handoff) {
        let state_path = data.map(|d| d.join("update.json"));
        let mut state = state_path.as_deref().map(State::load).unwrap_or_default();
        let source = source();
        let on_probation = update::restart::on_probation();
        if let Ok(source) = &source
            && let Ok(install) = &source.install
            && !on_probation
        {
            // What an earlier update left: its download, or the version it replaced. Not while the
            // version before this one waits to hear that it started: it may still need to roll back.
            install.clean_up();
        }
        let mut waiting = VecDeque::new();
        if let Some(failed) = state.failed.take() {
            let current = source
                .as_ref()
                .map(|s| s.current.to_string())
                .unwrap_or_default();
            waiting.push_back(Dialog::new(
                "UPDATE ROLLED BACK",
                vec![
                    format!(
                        "VERSION {} DIDN'T START, SO {current} IS BACK.",
                        failed.version
                    ),
                    failed.reason.clone(),
                ],
                vec![("OK", Choice::Close)],
            ));
            // Not offered again automatically: it would fail again.
            state.skip = Some(failed.version);
        }
        let told_of_failure = !waiting.is_empty();
        let auto_due = source.is_ok() && !on_probation && state.check_due(state::now());
        let handoff = Handoff::default();
        let updater = Updater {
            source,
            state_path,
            state,
            font,
            running: None,
            auto_due,
            dialog: None,
            waiting,
            ready: None,
            handoff: handoff.clone(),
            interruptible: false,
            #[cfg(target_os = "macos")]
            menu: None,
        };
        if told_of_failure {
            updater.save();
        }
        if let Err(e) = &updater.source {
            eprintln!("app: no updates: {e}");
        }
        (updater, handoff)
    }

    fn save(&self) {
        if let Some(path) = &self.state_path
            && let Err(e) = self.state.save(path)
        {
            eprintln!("app: {e}");
        }
    }

    fn current(&self) -> String {
        match &self.source {
            Ok(s) => s.current.to_string(),
            Err(_) => VERSION.unwrap_or("(development)").to_owned(),
        }
    }

    /// Whether the player can ask for a check now.
    fn can_check(&self) -> bool {
        self.source.is_ok() && self.interruptible && self.running.is_none() && self.dialog.is_none()
    }

    fn start_check(&mut self, manual: bool) {
        let Ok(source) = &self.source else { return };
        self.state.last_check = Some(state::now());
        self.save();
        // A version the player skipped is still offered when they ask.
        let skip = if manual {
            None
        } else {
            self.state
                .skip
                .as_deref()
                .and_then(|v| update::Version::parse(v).ok())
        };
        self.running = Some(Running {
            messages: check::spawn(source.clone(), skip),
            manual,
        });
        if manual {
            let mut checking = Dialog::new(
                "UPDATES",
                vec!["CHECKING FOR UPDATES...".into()],
                vec![("HIDE", Choice::Close)],
            );
            checking.checking = true;
            self.show(checking);
        }
    }

    /// Shows `dialog` after any already showing or waiting: now if the player is between runs, or once
    /// they are. It replaces a check's progress.
    fn show(&mut self, dialog: Dialog) {
        self.drop_progress();
        self.waiting.push_back(dialog);
        if self.interruptible && self.dialog.is_none() {
            self.dialog = self.waiting.pop_front();
        }
    }

    /// Closes the dialog showing a check's progress, wherever it is.
    fn drop_progress(&mut self) {
        if self.dialog.as_ref().is_some_and(|d| d.checking) {
            self.dialog = None;
        }
        self.waiting.retain(|d| !d.checking);
    }

    fn poll(&mut self) {
        let Some(running) = &self.running else { return };
        let manual = running.manual;
        // Everything the check sent since the last tick: only the latest progress matters.
        let mut progress = None;
        let found = loop {
            match running.messages.try_recv() {
                Ok(Message::Progress(p)) => progress = Some(p),
                Ok(Message::Done(found)) => break Some(found),
                Err(TryRecvError::Empty) => break None,
                Err(TryRecvError::Disconnected) => break Some(Err("the check stopped".into())),
            }
        };
        if let Some(Progress::Downloading { version, got, size }) = progress
            && let Some(d) = self.dialog.as_mut().filter(|d| d.checking)
        {
            d.lines = vec![format!(
                "DOWNLOADING {version}: {}%",
                got * 100 / size.max(1)
            )];
        }
        let Some(found) = found else { return };
        self.running = None;
        let current = self.current();
        let dialog = match found {
            Ok(Found::UpToDate) => manual.then(|| {
                Dialog::new(
                    "UPDATES",
                    vec![format!("YOU HAVE THE LATEST VERSION, {current}.")],
                    vec![("OK", Choice::Later)],
                )
            }),
            Ok(Found::Newer {
                version,
                notes,
                why_not,
            }) => Some(Dialog::new(
                "NEW VERSION",
                vec![
                    format!("SPELUNKY CLASSIC {version} IS OUT. YOU HAVE {current}."),
                    why_not,
                ],
                vec![
                    ("GET IT FROM THE WEBSITE", Choice::Open(notes)),
                    ("LATER", Choice::Later),
                    ("SKIP THIS VERSION", Choice::Skip(version.to_string())),
                ],
            )),
            Ok(Found::Ready(ready)) => {
                let version = ready.version.to_string();
                let notes = ready.notes.clone();
                self.ready = Some(ready);
                Some(Dialog::new(
                    "UPDATE READY",
                    vec![
                        format!(
                            "SPELUNKY CLASSIC {version} IS DOWNLOADED AND VERIFIED. YOU HAVE {current}."
                        ),
                        "YOUR SAVES, SCORES AND SETTINGS STAY AS THEY ARE.".into(),
                    ],
                    vec![
                        ("RESTART AND UPDATE", Choice::Restart),
                        ("LATER", Choice::Later),
                        ("SKIP THIS VERSION", Choice::Skip(version)),
                        ("RELEASE NOTES", Choice::Open(notes)),
                    ],
                ))
            }
            Err(e) => {
                eprintln!("app: update check: {e}");
                manual.then(|| {
                    Dialog::new(
                        "UPDATES",
                        vec!["COULDN'T CHECK FOR UPDATES:".into(), e],
                        vec![("OK", Choice::Later)],
                    )
                })
            }
        };
        match dialog {
            Some(dialog) => self.show(dialog),
            // A check that found nothing worth saying closes its progress, if the player watched it.
            None => self.drop_progress(),
        }
    }

    /// The updates menu (Windows and Linux; the macOS menu bar has the same two items).
    #[cfg_attr(target_os = "macos", allow(dead_code))]
    fn menu_dialog(&self) -> Dialog {
        let automatic = if self.state.automatic { "ON" } else { "OFF" };
        Dialog::new(
            "UPDATES",
            vec![format!("SPELUNKY CLASSIC {}", self.current())],
            vec![
                ("CHECK FOR UPDATES", Choice::Check),
                (
                    &format!("CHECK AUTOMATICALLY: {automatic}"),
                    Choice::ToggleAutomatic,
                ),
                ("CLOSE", Choice::Close),
            ],
        )
    }

    /// Does the selected choice.
    fn choose(&mut self) {
        let Some(dialog) = self.dialog.take() else {
            return;
        };
        let Some((_, choice)) = dialog.choices.get(dialog.selected).cloned() else {
            return;
        };
        match choice {
            Choice::Restart => *self.handoff.borrow_mut() = self.ready.take(),
            Choice::Later | Choice::Close => {}
            Choice::Skip(version) => {
                self.state.skip = Some(version);
                self.save();
                if let Some(ready) = self.ready.take() {
                    ready.install.clean_up();
                }
            }
            Choice::Open(url) => open_url(&url),
            Choice::Check => self.start_check(true),
            Choice::ToggleAutomatic => {
                self.toggle_automatic();
                let mut menu = self.menu_dialog();
                menu.selected = dialog.selected;
                self.dialog = Some(menu);
            }
        }
    }

    fn toggle_automatic(&mut self) {
        self.state.automatic = !self.state.automatic;
        self.save();
    }
}

impl Overlay for Updater {
    fn started(&mut self) {
        update::restart::confirm_started();
        #[cfg(target_os = "macos")]
        if self.source.is_ok() {
            self.menu = mac::Menu::new(self.state.automatic);
        }
    }

    fn tick(&mut self, interruptible: bool) -> bool {
        self.interruptible = interruptible;
        #[cfg(target_os = "macos")]
        for item in mac::chosen() {
            match item {
                mac::Item::Check if self.can_check() => self.start_check(true),
                mac::Item::Automatic => self.toggle_automatic(),
                _ => {}
            }
        }
        self.poll();
        if interruptible {
            if self.auto_due && self.running.is_none() {
                self.auto_due = false;
                self.start_check(false);
            }
            if self.dialog.is_none() {
                self.dialog = self.waiting.pop_front();
            }
        } else if let Some(dialog) = self.dialog.take() {
            // A run began under it (the game can't see the keys it takes, but it doesn't stop): it shows
            // again, first, the next time the player is between runs.
            self.waiting.push_front(dialog);
        }
        #[cfg(target_os = "macos")]
        if let Some(menu) = &self.menu {
            menu.update(self.can_check(), self.state.automatic);
        }
        // The player chose "restart and update": the window closes, and `restart` takes over.
        self.handoff.borrow().is_some()
    }

    fn commands(&self, size: (u32, u32)) -> Option<Vec<Command>> {
        let dialog = self.dialog.as_ref()?;
        let (font, (cw, ch)) = self.font?;
        let (w, h) = (size.0.max(1), size.1.max(1));
        // In whole GM pixels, as big as the game's (its 320x240 view, scaled).
        let scale = (w / 320).min(h / 240).max(1);
        let (vw, vh) = ((w / scale) as i32, (h / scale) as i32);
        let width = ((vw as u32).saturating_sub(4 * cw) / cw).clamp(8, 36);
        let mut rows: Vec<(String, Color)> =
            vec![(text(&dialog.title), TITLE_COLOR), (String::new(), DIM)];
        for line in &dialog.lines {
            for l in wrap(&[&text(line)], width) {
                rows.push((l, Color::WHITE));
            }
        }
        rows.push((String::new(), DIM));
        for (i, (label, _)) in dialog.choices.iter().enumerate() {
            let selected = i == dialog.selected;
            let marker = if selected { "> " } else { "  " };
            let color = if selected { TITLE_COLOR } else { Color::WHITE };
            rows.push((format!("{marker}{}", text(label)), color));
        }
        let chars = rows.iter().map(|(r, _)| r.len() as u32).max().unwrap_or(0);
        let line_h = ch as i32 + 3;
        let (bw, bh) = (
            (chars + 2) as i32 * cw as i32,
            rows.len() as i32 * line_h + ch as i32,
        );
        let (x0, y0) = ((vw - bw) / 2, (vh - bh) / 2);
        let mut out = vec![
            Command::SetView {
                view: Rect::new(0, 0, vw, vh),
                port: Rect::new(0, 0, vw * scale as i32, vh * scale as i32),
            },
            Command::Rectangle {
                x1: f64::from(x0),
                y1: f64::from(y0),
                x2: f64::from(x0 + bw - 1),
                y2: f64::from(y0 + bh - 1),
                color: Color::BLACK,
                alpha: 1.0,
                outline: false,
            },
            Command::Rectangle {
                x1: f64::from(x0),
                y1: f64::from(y0),
                x2: f64::from(x0 + bw - 1),
                y2: f64::from(y0 + bh - 1),
                color: TITLE_COLOR,
                alpha: 1.0,
                outline: true,
            },
        ];
        for (i, (row, color)) in rows.into_iter().enumerate() {
            out.push(Command::Text(Text {
                font,
                x: f64::from(x0 + cw as i32),
                y: f64::from(y0 + ch as i32 / 2 + 2 + i as i32 * line_h),
                text: row.into_bytes(),
                color,
                alpha: 1.0,
            }));
        }
        Some(out)
    }

    fn key_down(&mut self, key: KeyCode) -> bool {
        let Some(dialog) = &mut self.dialog else {
            // F2 opens the updates menu where there's no menu bar.
            if cfg!(not(target_os = "macos")) && key == KeyCode::F2 && self.can_check() {
                self.dialog = Some(self.menu_dialog());
                return true;
            }
            return false;
        };
        let n = dialog.choices.len().max(1);
        match key {
            KeyCode::ArrowUp | KeyCode::ArrowLeft => {
                dialog.selected = (dialog.selected + n - 1) % n
            }
            KeyCode::ArrowDown | KeyCode::ArrowRight | KeyCode::Tab => {
                dialog.selected = (dialog.selected + 1) % n
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => self.choose(),
            KeyCode::Escape => self.dialog = None,
            _ => {}
        }
        // While a dialog shows, every key is its own.
        true
    }

    fn modal(&self) -> bool {
        self.dialog.is_some()
    }
}

/// `s` as the game's font can draw it: upper case, and `?` for what it lacks (it has `' '` to `'_'`).
/// `#` is a line break in GM's text, so it's left out.
fn text(s: &str) -> String {
    s.to_uppercase()
        .chars()
        .map(|c| match c {
            '#' => ' ',
            '…' => '.',
            ' '..='_' => c,
            _ => '?',
        })
        .collect()
}

/// What a check needs, if this build can update.
fn source() -> Result<Source, String> {
    // Only a debug build takes these from the environment: a release trusts only what it was built with.
    let var = |name| {
        if cfg!(debug_assertions) {
            std::env::var(name).ok().filter(|v| !v.is_empty())
        } else {
            None
        }
    };
    let version = var("SPELUNKY_VERSION")
        .or(VERSION.map(str::to_owned))
        .ok_or("a development build (built without SPELUNKY_VERSION)")?;
    let key = match var("SPELUNKY_UPDATE_KEY") {
        Some(path) => {
            let file = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
            update::key::public_key(&file).map_err(|e| format!("{path}: {e}"))?
        }
        None => update::key::release_key().ok_or("this build has no release key")?,
    };
    let install = std::env::current_exe()
        .map_err(|e| format!("can't find the app: {e}"))
        .and_then(|exe| Install::detect(&exe, appimage(&exe).as_deref()));
    Ok(Source {
        manifest_url: var("SPELUNKY_UPDATE_URL").unwrap_or_else(|| update::MANIFEST_URL.to_owned()),
        key,
        current: update::Version::parse(&version)?,
        platform: update::platform(),
        install,
    })
}

/// The AppImage this runs from, if it does: `$APPIMAGE`, which its runtime sets, but only if this runs
/// from its mount (`$APPDIR`). A program started from another AppImage inherits that one's.
fn appimage(exe: &Path) -> Option<PathBuf> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    let image = std::env::var_os("APPIMAGE")?;
    let mount = std::env::var_os("APPDIR")?;
    exe.starts_with(&mount).then(|| PathBuf::from(image))
}

/// Opens `url` in the browser.
fn open_url(url: &str) {
    if update::manifest::check_url(url).is_err() {
        return;
    }
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(windows) {
        "explorer"
    } else {
        "xdg-open"
    };
    if let Err(e) = std::process::Command::new(opener).arg(url).spawn() {
        eprintln!("app: can't open {url}: {e}");
    }
}

/// Once the window has closed for "restart and update": swaps the update in and starts it, or, if it
/// fails to start, puts this version back and starts it again (`update::restart`). The process should exit
/// afterwards either way.
pub fn restart(ready: Ready, data: Option<&Path>) -> Result<(), String> {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let state_path = data.map(|d| d.join("update.json"));
    let outcome = update::restart::install_and_restart(
        &ready,
        &args,
        state_path.as_deref(),
        update::restart::START_TIMEOUT,
    )?;
    if let update::restart::Outcome::RolledBack(why) = outcome {
        eprintln!(
            "app: the update to {} was rolled back: {why}",
            ready.version
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An updater with a dialog up, as a check that found an update leaves it.
    fn with_dialog() -> Updater {
        let font = SpriteFont {
            sprite: 0,
            first: b' ',
            separation: 0,
        };
        let (mut updater, _) = Updater::new(None, Some((font, (8, 8))));
        updater.interruptible = true;
        updater.show(Dialog::new(
            "UPDATE READY",
            vec!["Spelunky Classic 1.1.0 is downloaded and verified. You have 1.0.0.".into()],
            vec![
                ("RESTART AND UPDATE", Choice::Restart),
                ("LATER", Choice::Later),
            ],
        ));
        updater
    }

    #[test]
    fn writes_what_the_font_has() {
        assert_eq!(text("Check for Updates… #1 ©"), "CHECK FOR UPDATES.  1 ?");
    }

    #[test]
    fn a_dialog_takes_every_key_until_closed() {
        let mut updater = with_dialog();
        assert!(updater.modal());
        assert!(updater.key_down(KeyCode::ArrowRight));
        assert!(updater.key_down(KeyCode::KeyZ));
        assert_eq!(updater.dialog.as_ref().unwrap().selected, 1);
        assert!(updater.key_down(KeyCode::Escape));
        assert!(!updater.modal());
        assert!(!updater.key_down(KeyCode::KeyZ));
        assert!(!updater.key_down(KeyCode::Escape));
    }

    #[test]
    fn a_dialog_waits_out_a_run() {
        let mut updater = with_dialog();
        assert!(!updater.tick(false));
        assert!(!updater.modal());
        assert!(!updater.key_down(KeyCode::Enter));
        assert!(!updater.tick(true));
        assert!(updater.modal());
    }

    #[test]
    fn dialogs_queue_and_a_result_replaces_its_progress() {
        let mut updater = with_dialog();
        let mut checking = Dialog::new("UPDATES", vec![], vec![("HIDE", Choice::Close)]);
        checking.checking = true;
        updater.show(checking);
        updater.show(Dialog::new("RESULT", vec![], vec![("OK", Choice::Close)]));
        // A run starts and ends: the first dialog is still first.
        updater.tick(false);
        updater.tick(true);
        let titles = |u: &Updater| {
            let mut t: Vec<String> = u.dialog.iter().map(|d| d.title.clone()).collect();
            t.extend(u.waiting.iter().map(|d| d.title.clone()));
            t
        };
        assert_eq!(titles(&updater), ["UPDATE READY", "RESULT"]);
        updater.key_down(KeyCode::Escape);
        updater.tick(true);
        assert_eq!(titles(&updater), ["RESULT"]);
    }

    #[test]
    fn a_dialog_fits_the_picture_in_whole_pixels() {
        let updater = with_dialog();
        for (size, scale) in [
            ((960, 720), 3),
            ((320, 240), 1),
            ((1920, 1080), 4),
            ((100, 100), 1),
        ] {
            let commands = updater.commands(size).unwrap();
            let Command::SetView { view, port } = commands[0] else {
                panic!("starts with the view")
            };
            assert_eq!(port.w, view.w * scale, "{size:?}");
            if view.w < 320 {
                continue;
            }
            for c in &commands {
                if let Command::Text(t) = c {
                    assert!(
                        t.x >= 0.0 && t.x + 8.0 * t.text.len() as f64 <= f64::from(view.w),
                        "{size:?}"
                    );
                }
            }
        }
    }
}

/// The macOS menu bar: winit's, with the updater's two items in the app menu.
#[cfg(target_os = "macos")]
mod mac {
    use muda::{CheckMenuItem, Menu as Bar, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};

    const CHECK: &str = "update-check";
    const AUTOMATIC: &str = "update-automatic";

    pub enum Item {
        Check,
        Automatic,
    }

    pub struct Menu {
        // Kept alive: the menu bar is the app's while it is.
        _bar: Bar,
        check: MenuItem,
        automatic: CheckMenuItem,
    }

    impl Menu {
        pub fn new(automatic: bool) -> Option<Menu> {
            let check = MenuItem::with_id(CHECK, "Check for Updates…", true, None);
            let automatic = CheckMenuItem::with_id(
                AUTOMATIC,
                "Check for Updates Automatically",
                true,
                automatic,
                None,
            );
            let separator = PredefinedMenuItem::separator;
            let app = Submenu::with_items(
                "Spelunky Classic",
                true,
                &[
                    &PredefinedMenuItem::about(None, None),
                    &separator(),
                    &check,
                    &automatic,
                    &separator(),
                    &PredefinedMenuItem::services(None),
                    &separator(),
                    &PredefinedMenuItem::hide(None),
                    &PredefinedMenuItem::hide_others(None),
                    &PredefinedMenuItem::show_all(None),
                    &separator(),
                    &PredefinedMenuItem::quit(None),
                ],
            )
            .ok()?;
            let window = Submenu::with_items(
                "Window",
                true,
                &[
                    &PredefinedMenuItem::minimize(None),
                    &PredefinedMenuItem::fullscreen(None),
                    &separator(),
                    &PredefinedMenuItem::bring_all_to_front(None),
                ],
            )
            .ok()?;
            let bar = Bar::with_items(&[&app, &window]).ok()?;
            bar.init_for_nsapp();
            window.set_as_windows_menu_for_nsapp();
            Some(Menu {
                _bar: bar,
                check,
                automatic,
            })
        }

        pub fn update(&self, can_check: bool, automatic: bool) {
            if self.check.is_enabled() != can_check {
                self.check.set_enabled(can_check);
            }
            if self.automatic.is_checked() != automatic {
                self.automatic.set_checked(automatic);
            }
        }
    }

    /// The items chosen since the last call.
    pub fn chosen() -> Vec<Item> {
        std::iter::from_fn(|| MenuEvent::receiver().try_recv().ok())
            .filter_map(|e| match e.id.0.as_str() {
                CHECK => Some(Item::Check),
                AUTOMATIC => Some(Item::Automatic),
                _ => None,
            })
            .collect()
    }
}
