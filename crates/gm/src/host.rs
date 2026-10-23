//! What the original read from or wrote to the machine around it: the game directory, the highscore table
//! and the clock (runtime.md section 11). The port takes all of it as an explicit input, so a
//! run is reproducible, and keeps it in the world because the game changes it (`stats.txt`,
//! `highscore_add`), so savestates carry it.

use crate::string::GmString;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// How many entries GM 8's highscore table holds.
pub const HIGHSCORE_ENTRIES: usize = 10;

/// The machine the game runs on, as far as the game can see it (input aside).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Host {
    /// The game directory's files by relative, `/`-separated path, byte for byte. The runner puts the three
    /// `.cfg` files here (`trace::ReplayHeader::game_files`).
    /// The contents are shared, so cloning the world doesn't copy them.
    pub files: BTreeMap<String, GmString>,
    /// GM 8's highscore table, best first: always [`HIGHSCORE_ENTRIES`] `(name, value)` entries.
    pub highscores: Vec<(GmString, f64)>,
    /// The system clock when the game starts, in ns since the Unix epoch (a replay's `time`).
    pub start_time: u128,
}

impl Host {
    /// A replay's machine: its game directory (`ReplayHeader::game_files`: the three `.cfg` files plus its
    /// `file` lines), its highscore table (filled up with the fresh table's entries, as the oracle does)
    /// and its start time. Its joystick goes to the input state ([`World::connect_joystick`]).
    ///
    /// [`World::connect_joystick`]: crate::World::connect_joystick
    pub fn from_replay(h: &trace::ReplayHeader) -> Host {
        let mut highscores = Host::fresh_highscores();
        for (slot, e) in highscores.iter_mut().zip(&h.highscores) {
            *slot = (GmString::from(e.name.as_str()), e.value);
        }
        Host {
            files: h
                .game_files()
                .into_iter()
                .map(|(path, text)| (path, GmString::from(text)))
                .collect(),
            highscores,
            start_time: h.time,
        }
    }

    /// GM 8's fresh highscore table: ten `<nobody>` entries scoring 0.
    pub fn fresh_highscores() -> Vec<(GmString, f64)> {
        vec![(GmString::from("<nobody>"), 0.0); HIGHSCORE_ENTRIES]
    }
}

impl Default for Host {
    /// What the oracle starts with when a replay doesn't say otherwise: the shipped `settings.cfg`,
    /// `keys.cfg` and `gamepad.cfg`, a fresh highscore table and 2010-01-01.
    fn default() -> Self {
        Host::from_replay(&trace::ReplayHeader::default())
    }
}
