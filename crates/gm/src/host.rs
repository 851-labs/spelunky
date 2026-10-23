//! What the original read from or wrote to the machine around it: the game directory, the highscore table
//! and the clock (runtime.md section 11). The port takes all of it as an explicit input, so a
//! run is reproducible, and keeps it in the world because the game changes it (`stats.txt`,
//! `highscore_add`), so savestates carry it.
//!
//! The DLLs the game calls are part of that machine too, but `gm` can't name the port's models of them
//! (the `audio` crate depends on `gm`), so the game puts them in [`Host::ext`] as an [`Extension`].

use crate::string::GmString;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::BTreeMap;
use std::fmt;

/// Host state the game owns and `gm` can't name (Spelunky's: its model of `supersound.dll`). It lives in
/// [`Host::ext`], so it's cloned with the world and saved in its savestates, through this contract.
/// The game gets its own type back with [`Any`] downcasts (`ext.as_ref() as &dyn Any`).
pub trait Extension: Any + fmt::Debug + Send + Sync {
    /// A copy, for `Clone`ing the world: it should be as cheap as the rest of the world's clone.
    fn clone_box(&self) -> Box<dyn Extension>;
    /// Whether `other` is of the same type and holds the same state.
    fn eq_dyn(&self, other: &dyn Extension) -> bool;
    /// The state, for a savestate file.
    fn save(&self) -> Vec<u8>;
    /// Replaces the state with what [`save`](Self::save) returned, keeping what isn't state (shared,
    /// immutable data such as sound files). Rejects bytes that don't decode to a consistent state, so
    /// nothing panics on them later.
    fn load(&mut self, bytes: &[u8]) -> Result<(), ExtensionError>;
}

/// Why an [`Extension`] rejected a savestate's bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtensionError(String);

impl ExtensionError {
    pub fn new(reason: impl Into<String>) -> ExtensionError {
        ExtensionError(reason.into())
    }
}

impl fmt::Display for ExtensionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ExtensionError {}

impl Clone for Box<dyn Extension> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

impl PartialEq for dyn Extension {
    fn eq(&self, other: &Self) -> bool {
        self.eq_dyn(other)
    }
}

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
    /// The game's own host state, if it has any. Savestates hold it after the rest of the world
    /// ([`Extension::save`]), not inside the host's serde encoding.
    #[serde(skip)]
    pub ext: Option<Box<dyn Extension>>,
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
            ext: None,
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
