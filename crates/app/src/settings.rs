//! The settings program (`config.exe`, `crates/config`) in the window: `app --settings` natively, `?settings`
//! on the web. It runs like the game ([`Play`], on its own bundle, `assets/config/`, with the game's sounds)
//! and keeps its `.cfg` files in the same store the game reads them from ([`crate::save`]).
//!
//! When it ends having called `execute_program("Spelunky.exe", ...)` (OK with "start game on exit"
//! checked), the game starts in the same window: the port records that call instead of starting an
//! executable (`docs/quirks.md` section 7), and this is where it's honoured. The game boots on the
//! session's header (its seed, clock and joystick, so a replay of the settings program plays the game the
//! same way every time) with the game directory the settings program left, so it reads the settings just
//! written, store or no store. The settings program's sound stops, as its process would.

use crate::notice::Notice;
use crate::play::Play;
use crate::textures::Textures;
use crate::window::{Game as WindowGame, Switch};
use draw::Frame;
use trace::replay::{Event, ReplayHeader};

/// The game the settings program can start: its play (with its own store if any, not begun), its images,
/// its notice, and its window size if the window is fixed (`--scale`).
pub struct Launch {
    pub game: Play,
    pub textures: Textures,
    pub notice: Option<Notice>,
    pub window_size: Option<(u32, u32)>,
}

/// The settings program, then the game if it asks for it.
pub struct Settings {
    settings: Play,
    launch: Option<Launch>,
    game: Option<Play>,
    /// The game's images and notice, from the tick that starts it until the window takes them.
    switch: Option<Switch>,
    /// The session's header, which the game boots on too.
    header: ReplayHeader,
    /// The player has made a user gesture, so the game's sound can start straight away.
    gestured: bool,
}

impl Settings {
    /// `settings` should run `config::executable()` ([`Play::with_program`]) on the settings program's
    /// assets with the game's sounds, and keep a store if `launch`'s game does.
    pub fn new(settings: Play, launch: Launch) -> Settings {
        Settings {
            settings,
            launch: Some(launch),
            game: None,
            switch: None,
            header: ReplayHeader::default(),
            gestured: false,
        }
    }

    fn launches(&self) -> bool {
        self.settings.ended() && self.settings.game().is_some_and(config::launches_game)
    }
}

impl WindowGame for Settings {
    fn begin(&mut self, header: &ReplayHeader, out: &mut Frame) -> Result<(), String> {
        self.header = header.clone();
        self.settings.begin(header, out)
    }

    fn tick(&mut self, events: &[Event], out: &mut Frame) -> Result<(), String> {
        if let Some(game) = &mut self.game {
            return game.tick(events, out);
        }
        self.settings.tick(events, out)?;
        if self.launches()
            && let Some(Launch {
                game,
                textures,
                notice,
                window_size,
            }) = self.launch.take()
        {
            self.settings.stop_sound();
            let files = self
                .settings
                .game()
                .map(|s| s.world().host().files.clone())
                .unwrap_or_default();
            let mut game = game.with_files(files);
            // Its boot draws over the settings program's last frame.
            *out = Frame::default();
            game.begin(&self.header, out)?;
            if self.gestured {
                game.user_gesture();
            }
            self.game = Some(game);
            self.switch = Some(Switch {
                textures,
                notice,
                window_size,
            });
        }
        Ok(())
    }

    fn ended(&self) -> bool {
        match &self.game {
            Some(game) => game.ended(),
            None => self.settings.ended() && !self.launches(),
        }
    }

    fn close_button(&self) -> bool {
        true
    }

    fn fullscreen(&self) -> Option<bool> {
        self.game.as_ref().unwrap_or(&self.settings).fullscreen()
    }

    fn sizes_window(&self) -> bool {
        self.game.as_ref().unwrap_or(&self.settings).sizes_window()
    }

    fn user_gesture(&mut self) {
        self.gestured = true;
        match &mut self.game {
            Some(game) => game.user_gesture(),
            None => self.settings.user_gesture(),
        }
    }

    fn take_switch(&mut self) -> Option<Switch> {
        self.switch.take()
    }

    /// The game's rooms between runs, once it has started: the settings program itself is a few clicks
    /// the player is in the middle of.
    fn interruptible(&self) -> bool {
        self.game.as_ref().is_some_and(Play::interruptible)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::play::SoundOutput;
    use crate::save::{Saved, Store};
    use crate::textures::Layers;
    use gm::GmString;
    use std::cell::RefCell;
    use std::io;
    use std::rc::Rc;
    use std::sync::Arc;
    use trace::replay::{Button, MouseButton};
    use transpile::reference::{self, Program};

    /// One store shared by the two plays, as two `DirStore`s on one directory are.
    #[derive(Clone, Default)]
    struct Memory(Rc<RefCell<Saved>>);

    impl Store for Memory {
        fn load(&self) -> io::Result<Saved> {
            Ok(self.0.borrow().clone())
        }
        fn write_file(&mut self, path: &str, contents: &[u8]) -> io::Result<()> {
            self.0
                .borrow_mut()
                .files
                .insert(path.into(), contents.to_vec());
            Ok(())
        }
        fn delete_file(&mut self, path: &str) -> io::Result<()> {
            self.0.borrow_mut().files.remove(path);
            Ok(())
        }
        fn write_highscores(&mut self, table: &[(GmString, f64)]) -> io::Result<()> {
            self.0.borrow_mut().highscores = Some(table.to_vec());
            Ok(())
        }
    }

    fn assets(program: Program) -> Arc<gm::Assets> {
        let gmk = reference::load_program(&reference::default_path(), program);
        Arc::new(reference::stub_assets(&gmk))
    }

    fn no_textures() -> Textures {
        let empty = || Layers {
            width: 1,
            height: 1,
            layers: vec![vec![0; 4]],
        };
        Textures {
            sprite_pages: empty(),
            background_pages: empty(),
            sprites: Vec::new(),
            backgrounds: Vec::new(),
        }
    }

    fn global(game: &game::Game, name: &str) -> gm::Value {
        let v = game::global_named(game.world().program(), name).unwrap();
        game.world().gget(v).unwrap()
    }

    fn settings_then_game(store: Option<Memory>) -> Settings {
        let sounds = Arc::default();
        let mut settings = Play::new(
            assets(Program::Config),
            Arc::clone(&sounds),
            SoundOutput::Off,
        )
        .with_program(config::executable());
        let mut game = Play::new(assets(Program::Game), sounds, SoundOutput::Off);
        if let Some(store) = store {
            settings = settings.with_store(Box::new(store.clone()));
            game = game.with_store(Box::new(store));
        }
        let launch = Launch {
            game,
            textures: no_textures(),
            notice: None,
            window_size: None,
        };
        Settings::new(settings, launch)
    }

    /// Clicks the room pixel (x, y) of rInit's 2x view port, over three ticks.
    fn click(s: &mut Settings, (x, y): (i32, i32), out: &mut Frame) {
        let left = Button::Mouse(MouseButton::Left);
        let at = Event::MouseMove { x: 2 * x, y: 2 * y };
        s.tick(&[at], out).unwrap();
        s.tick(&[Event::Press(left)], out).unwrap();
        s.tick(&[Event::Release(left)], out).unwrap();
    }

    #[test]
    fn ok_saves_the_settings_and_starts_the_game_on_them() {
        let store = Memory::default();
        let mut s = settings_then_game(Some(store.clone()));
        let mut out = Frame::default();
        s.begin(&ReplayHeader::default(), &mut out).unwrap();
        s.tick(&[], &mut out).unwrap();
        // The X1 radio box and the fullscreen checkbox; "start game on exit" stays checked. Then OK.
        click(&mut s, (19, 35), &mut out);
        click(&mut s, (19, 99), &mut out);
        assert!(s.take_switch().is_none());
        click(&mut s, (150, 252), &mut out);
        assert!(!s.ended(), "the game started");
        assert!(s.take_switch().is_some(), "with its images");
        let saved = store.0.borrow().files["settings.cfg"].clone();
        assert_eq!(saved, b"1\r\n1\r\n0\r\n1\r\n1\r\n15\r\n15");
        // The game booted from the store, and scrInit read the new settings.
        let game = s.game.as_ref().and_then(Play::game).unwrap();
        assert_eq!(game.world().host().files["settings.cfg"].as_bytes(), saved);
        assert_eq!(global(game, "screenScale"), gm::Value::Real(1.0));
        assert_eq!(global(game, "fullscreen"), gm::Value::Real(1.0));
        s.tick(&[], &mut out).unwrap();
    }

    #[test]
    fn the_game_reads_the_settings_without_a_store_too() {
        // X4, then OK, with nothing saved: the game boots on the settings program's game directory.
        let mut s = settings_then_game(None);
        let mut out = Frame::default();
        s.begin(&ReplayHeader::new(5), &mut out).unwrap();
        s.tick(&[], &mut out).unwrap();
        click(&mut s, (19, 59), &mut out);
        click(&mut s, (150, 252), &mut out);
        let game = s.game.as_ref().and_then(Play::game).unwrap();
        assert_eq!(
            game.world().host().files["settings.cfg"].as_bytes(),
            b"0\r\n1\r\n0\r\n1\r\n4\r\n15\r\n15"
        );
        assert_eq!(global(game, "screenScale"), gm::Value::Real(4.0));
    }

    /// A store that holds stale files and takes no writes.
    struct ReadOnly(Saved);

    impl Store for ReadOnly {
        fn load(&self) -> io::Result<Saved> {
            Ok(self.0.clone())
        }
        fn write_file(&mut self, _: &str, _: &[u8]) -> io::Result<()> {
            Err(io::Error::other("read-only"))
        }
        fn delete_file(&mut self, _: &str) -> io::Result<()> {
            Err(io::Error::other("read-only"))
        }
        fn write_highscores(&mut self, _: &[(GmString, f64)]) -> io::Result<()> {
            Err(io::Error::other("read-only"))
        }
    }

    #[test]
    fn the_settings_just_written_win_over_a_store_that_missed_them() {
        let mut stale = Saved::default();
        stale.files.insert(
            "settings.cfg".into(),
            b"0\r\n1\r\n0\r\n1\r\n2\r\n15\r\n15".to_vec(),
        );
        let sounds = Arc::default();
        let settings = Play::new(
            assets(Program::Config),
            Arc::clone(&sounds),
            SoundOutput::Off,
        )
        .with_program(config::executable())
        .with_store(Box::new(ReadOnly(stale.clone())));
        let game = Memory::default();
        *game.0.borrow_mut() = stale;
        let game_play = Play::new(assets(Program::Game), sounds, SoundOutput::Off)
            .with_store(Box::new(game.clone()));
        let launch = Launch {
            game: game_play,
            textures: no_textures(),
            notice: None,
            window_size: None,
        };
        let mut s = Settings::new(settings, launch);
        let mut out = Frame::default();
        s.begin(&ReplayHeader::default(), &mut out).unwrap();
        s.tick(&[], &mut out).unwrap();
        click(&mut s, (19, 59), &mut out);
        click(&mut s, (150, 252), &mut out);
        let fresh = b"0\r\n1\r\n0\r\n1\r\n4\r\n15\r\n15";
        let g = s.game.as_ref().and_then(Play::game).unwrap();
        assert_eq!(g.world().host().files["settings.cfg"].as_bytes(), fresh);
        assert_eq!(global(g, "screenScale"), gm::Value::Real(4.0));
        // The game's first sync saves them.
        s.tick(&[], &mut out).unwrap();
        assert_eq!(game.0.borrow().files["settings.cfg"], fresh);
    }

    #[test]
    fn escape_quits_without_starting_the_game() {
        let mut s = settings_then_game(None);
        let mut out = Frame::default();
        s.begin(&ReplayHeader::default(), &mut out).unwrap();
        s.tick(&[Event::Press(Button::Key(27))], &mut out).unwrap();
        assert!(s.ended());
        assert!(s.game.is_none());
    }
}
