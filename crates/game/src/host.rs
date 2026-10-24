//! The game's own host state, which `gm` keeps in `gm::Host::ext` (a `gm::Extension`): the port's model of
//! `supersound.dll`, which the `SS_*` overrides call, and the programs `execute_program` started (only the
//! settings program, `crates/config`, starts one). Kept in the world rather than beside it because the
//! overrides and built-ins only get the world, and so that cloning the world (a savestate) clones it too.

use std::any::Any;
use std::sync::Arc;

use audio::{Mode, SoundBank, SuperSound};
use gm::{Extension, ExtensionError, World};

/// The `(prog, arg)` of each `execute_program` call ([`HostState::executed`]).
pub type Executed = Vec<(String, String)>;

/// What the game keeps in [`gm::Host::ext`].
#[derive(Clone, Debug)]
pub struct HostState {
    /// supersound.dll and its GML wrappers. Its sound files are shared between clones.
    pub sound: SuperSound,
    /// Each `execute_program(prog, arg, false)` so far: `(prog, arg)`. The port starts no executables;
    /// the frontend reads this when the run ends (the settings program starts the game this way).
    pub executed: Executed,
}

impl HostState {
    pub fn new(sounds: Arc<SoundBank>, mode: Mode) -> HostState {
        HostState {
            sound: SuperSound::new(sounds, mode),
            executed: Vec::new(),
        }
    }

    /// The world's host state, if it has the game's.
    pub fn get(world: &World) -> Option<&HostState> {
        let ext: &dyn Any = world.host().ext.as_deref()?;
        ext.downcast_ref()
    }

    pub fn get_mut(world: &mut World) -> Option<&mut HostState> {
        let ext: &mut dyn Any = world.host_mut().ext.as_deref_mut()?;
        ext.downcast_mut()
    }
}

impl Extension for HostState {
    /// A copy without the commands not taken yet: they're output, for this game's player only, so a
    /// clone (a savestate) mustn't play them again.
    fn clone_box(&self) -> Box<dyn Extension> {
        let mut copy = self.clone();
        drop(copy.sound.take_commands());
        Box::new(copy)
    }

    /// Whether the states are the same, as savestates see them: not the shared bank, nor the commands
    /// not taken yet, which `SuperSound`'s `==` compares.
    fn eq_dyn(&self, other: &dyn Extension) -> bool {
        (other as &dyn Any)
            .downcast_ref::<HostState>()
            .is_some_and(|o| o.sound.same_state(&self.sound) && o.executed == self.executed)
    }

    /// The sound bank's fingerprint, then the audio model's state: mode, handles, positions, the paused
    /// list; then the programs started. Not the sound files, which aren't state, nor the pending commands,
    /// which are output (a player resyncs after loading).
    fn save(&self) -> Vec<u8> {
        let bank = self.sound.model().bank().fingerprint();
        // Serializing plain data into a Vec can't fail.
        postcard::to_allocvec(&(bank, &self.sound, &self.executed))
            .expect("postcard encoding into a Vec")
    }

    /// Rejects a state saved with other sound files (its handles name that bank's files), and one the
    /// model couldn't be in.
    fn load(&mut self, bytes: &[u8]) -> Result<(), ExtensionError> {
        let corrupt = |e: postcard::Error| ExtensionError::new(format!("corrupt audio state: {e}"));
        let ((bank, mut sound, executed), rest): ((u64, SuperSound, Executed), _) =
            postcard::take_from_bytes(bytes).map_err(corrupt)?;
        if !rest.is_empty() {
            return Err(corrupt(postcard::Error::DeserializeBadEncoding));
        }
        if !sound.is_consistent() {
            return Err(ExtensionError::new("inconsistent audio state"));
        }
        let ours = self.sound.model().bank();
        if bank != ours.fingerprint() {
            return Err(ExtensionError::new(format!(
                "the savestate was made with other sound files (this run has {})",
                ours.len()
            )));
        }
        sound.set_bank(ours.clone());
        self.sound = sound;
        self.executed = executed;
        Ok(())
    }
}
