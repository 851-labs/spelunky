//! The real game in the window: [`Play`] runs a [`game::Game`] as the window's [`Game`], one step per
//! tick, plays its sounds, keeps its saves and can write its trace.
//!
//! A run starts from a replay header, as the oracle's do: a session of play from a fresh one
//! ([`session_header`]: a random seed and the real clock, like GM 8 at startup), a replay from its own.
//! A session of play also lays the save ([`crate::save`]) over the header's files; a recorded session and a
//! replay never do, so the recording holds everything its playback needs.
//!
//! The simulation never sees the devices: input arrives as replay events, the sound model answers every
//! `SS_*` query, and the device player only follows its commands ([`audio::backend`]). So a recording
//! replays headless (`replay-run --faithful-audio`) to the same trace as the live session.

use crate::save::{Persistence, Store};
use crate::window::Game as WindowGame;
use audio::backend::Player;
use draw::Frame;
use game::audio::{Mode, SoundBank};
use game::{Config, Executable, Game};
use std::collections::BTreeMap;
use std::sync::Arc;
use trace::replay::{Event, ReplayHeader};

/// Spelunky's rooms outside a run: the boot, the intro, the title, the highscores hall and the credits.
/// The challenge rooms (`rSun`, `rMoon`, `rStars`), the tutorial and the level editor are play, key and
/// joystick setup (`rKeyConfig`, `rJoyConfig`) are the player in the middle of choosing keys (`oKeyConfig`
/// takes whatever key comes next), and everything else is a run.
const BETWEEN_RUNS: [&str; 6] = [
    "rScreenInit",
    "rIntro",
    "rCredits1",
    "rCredits2",
    "rTitle",
    "rHighscores",
];

/// Takes each tick's trace line (`--trace`).
pub type TraceSink = Box<dyn FnMut(&trace::Frame) -> Result<(), String>>;

/// When the game's sounds reach the speakers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundOutput {
    /// Never: headless runs, or `--mute`.
    Off,
    /// From the start (natively).
    Now,
    /// From the player's first key or button press: browsers only start audio after a user gesture.
    OnGesture,
}

/// The game, as the window runs it.
pub struct Play {
    /// Spelunky, or the settings program (`crates/config`).
    exe: Executable,
    /// `exe` is Spelunky.
    is_spelunky: bool,
    /// The game directory to boot on instead of the header's ([`with_files`](Self::with_files)).
    files: Option<BTreeMap<String, gm::GmString>>,
    assets: Arc<gm::Assets>,
    sounds: Arc<SoundBank>,
    /// Laid over the game's files when it begins, then kept in step.
    store: Option<Box<dyn Store>>,
    persistence: Option<Persistence<Box<dyn Store>>>,
    trace: Option<TraceSink>,
    sound: SoundOutput,
    player: Option<Player>,
    game: Option<Game>,
    /// Problems that would repeat every tick are reported once.
    warned_save: bool,
    warned_audio: bool,
    /// The window keeps the size it was opened with instead of the game's.
    fixed_window: bool,
}

impl Play {
    /// The game on `assets`, with its sound files. It boots at the first tick ([`WindowGame::begin`]).
    pub fn new(assets: Arc<gm::Assets>, sounds: Arc<SoundBank>, sound: SoundOutput) -> Play {
        Play {
            exe: game::spelunky(),
            is_spelunky: true,
            files: None,
            assets,
            sounds,
            store: None,
            persistence: None,
            trace: None,
            sound,
            player: None,
            game: None,
            warned_save: false,
            warned_audio: false,
            fixed_window: false,
        }
    }

    /// Runs another program than Spelunky on `assets` (theirs, given to [`new`](Self::new)): the settings
    /// program, `config::executable()`. Its sounds are the game's.
    pub fn with_program(mut self, exe: Executable) -> Play {
        self.exe = exe;
        self.is_spelunky = false;
        self
    }

    /// Boots on these game directory files instead of the replay header's and the store's: the files
    /// the settings program left, for the game it starts.
    pub fn with_files(mut self, files: BTreeMap<String, gm::GmString>) -> Play {
        self.files = Some(files);
        self
    }

    /// Stops the sound for good: the settings program's, once it has started the game.
    pub fn stop_sound(&mut self) {
        self.player = None;
        self.sound = SoundOutput::Off;
    }

    /// Keeps the game directory and highscores in `store` between sessions. Only for sessions of play:
    /// a replay or a recording starts from its header's files.
    pub fn with_store(mut self, store: Box<dyn Store>) -> Play {
        self.store = Some(store);
        self
    }

    /// Hands every tick's trace line to `sink`, as `replay-run` writes them.
    pub fn with_trace(mut self, sink: TraceSink) -> Play {
        self.trace = Some(sink);
        self
    }

    /// Keeps the window at the size it opens with (`--scale`), rather than the size the game's rooms ask
    /// for.
    pub fn with_fixed_window(mut self) -> Play {
        self.fixed_window = true;
        self
    }

    /// The game, once it has begun.
    pub fn game(&self) -> Option<&Game> {
        self.game.as_ref()
    }

    fn open_player(&mut self) {
        // One try: a machine without an output device plays silently.
        self.sound = SoundOutput::Off;
        match Player::new(self.sounds.clone()) {
            Ok(mut player) => {
                // Whatever the game loaded or started before now, once: the resync covers the commands
                // not yet taken, which would otherwise load and start those sounds a second time.
                if let Some(game) = &mut self.game {
                    drop(game.take_audio_commands());
                    if let Err(e) = player.apply(&game.audio().model().resync()) {
                        warn(&format!("sound: {e}"));
                    }
                }
                self.player = Some(player);
            }
            Err(e) => warn(&format!("no sound: {e}")),
        }
    }

    /// Hands the game's sound commands since the last call to the player.
    fn play_sounds(&mut self) {
        let Some(game) = &mut self.game else { return };
        let commands = game.take_audio_commands();
        if let Some(player) = &mut self.player
            && let Err(e) = player.apply(&commands)
            && !std::mem::replace(&mut self.warned_audio, true)
        {
            warn(&format!(
                "sound: {e} (further sound errors aren't reported)"
            ));
        }
    }
}

/// The game's sound files (the bundle's `sounds`), keyed as the game loads them (`sound/jump.wav`), with
/// `read` giving a bundle file's bytes. A file the model can't read is left out with a warning: the DLL
/// couldn't load it either, so `SS_LoadSound` answers `"No"` for it (as `replay-run` does).
pub fn sound_bank(
    sounds: &[assets::SoundFile],
    mut read: impl FnMut(&str) -> Result<Vec<u8>, String>,
) -> Result<Arc<SoundBank>, String> {
    let mut bank = SoundBank::new();
    for s in sounds {
        if let Err(e) = bank.insert(&format!("sound/{}", s.name), read(&s.file)?) {
            warn(&format!(
                "{}: {e}; SS_LoadSound will answer \"No\" for it",
                s.file
            ));
        }
    }
    Ok(Arc::new(bank))
}

/// A new session's header: the shipped files, a random seed and the real clock, as GM 8 starts.
pub fn session_header() -> ReplayHeader {
    let now = web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    // Any seed will do; GM 8 takes one from the clock too.
    let seed = (now ^ (now >> 32) ^ (now >> 64)) as u32 as i32;
    ReplayHeader {
        time: now,
        ..ReplayHeader::new(seed)
    }
}

/// A GML error, located like `replay-run` reports it.
fn describe(game: &Game, frame: Option<u64>, e: &gm::GmlError) -> String {
    let w = game.world();
    let at = match frame {
        Some(f) => format!("frame {f}"),
        None => "booting (before frame 0)".into(),
    };
    let room = w.assets().room_name(w.room().id).unwrap_or("?");
    format!("{at} in {room}: {}", game.describe(e))
}

impl WindowGame for Play {
    fn begin(&mut self, header: &ReplayHeader, out: &mut Frame) -> Result<(), String> {
        let mut config = Config::from_replay(header);
        // The real DLL, for play. No game variable depends on the mode, only the sound handle globals
        // (`audio` crate docs), so traces still line up with `replay-run --faithful-audio`'s.
        config.audio = Mode::Faithful;
        config.sounds = self.sounds.clone();
        if let Some(store) = self.store.take() {
            match Persistence::open(store, &mut config.host) {
                Ok(p) => self.persistence = Some(p),
                Err(e) => warn(&format!(
                    "can't read the saved game, starting from the shipped files: {e}"
                )),
            }
        }
        // After the store's: these are newer (the settings program's, which may have failed to reach
        // the store). The first sync then saves whatever of them the store lacks.
        if let Some(files) = self.files.take() {
            config.host.files = files;
        }
        let mut game = Game::of(self.exe, self.assets.clone(), config);
        let room = game.start_room(&header.room)?;
        game.start_at(room, out)
            .map_err(|e| describe(&game, None, &e))?;
        self.game = Some(game);
        if self.sound == SoundOutput::Now {
            self.open_player();
        }
        self.play_sounds();
        Ok(())
    }

    fn tick(&mut self, events: &[Event], out: &mut Frame) -> Result<(), String> {
        let game = self
            .game
            .as_mut()
            .ok_or("the game ticked before it began")?;
        let frame = game.world().frames();
        let stepped = game.step_drawing(events, out);
        // Saved even when the step fails: what the game wrote before the error is the player's progress.
        if let Some(p) = &mut self.persistence
            && let Err(e) = p.sync(game.world().host())
            && !std::mem::replace(&mut self.warned_save, true)
        {
            warn(&format!(
                "can't save the game: {e} (it's tried again after every change)"
            ));
        }
        stepped.map_err(|e| describe(game, Some(frame), &e))?;
        if let Some(sink) = &mut self.trace {
            sink(&game.trace_with(false))?;
        }
        self.play_sounds();
        Ok(())
    }

    fn ended(&self) -> bool {
        self.game.as_ref().is_some_and(Game::ended)
    }

    fn fullscreen(&self) -> Option<bool> {
        Some(self.game.as_ref()?.world().fullscreen())
    }

    fn sizes_window(&self) -> bool {
        !self.fixed_window
    }

    fn user_gesture(&mut self) {
        if self.sound == SoundOutput::OnGesture {
            self.open_player();
        }
    }

    fn interruptible(&self) -> bool {
        self.is_spelunky
            && self.game.as_ref().is_some_and(|game| {
                let w = game.world();
                w.assets()
                    .room_name(w.room().id)
                    .is_some_and(|room| BETWEEN_RUNS.contains(&room))
            })
    }
}

fn warn(message: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    eprintln!("app: {message}");
    #[cfg(target_arch = "wasm32")]
    crate::web::warn(message);
}
