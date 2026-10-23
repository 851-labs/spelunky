//! The game's own host state, which `gm` keeps in `gm::Host::ext` (a `gm::Extension`): the port's model of
//! `supersound.dll`, which the `SS_*` overrides call. Kept in the world rather than beside it because the
//! overrides only get the world, and so that cloning the world (a savestate) clones it too.

use std::any::Any;
use std::sync::Arc;

use audio::{Mode, SoundBank, SuperSound};
use gm::{Extension, World};

/// What the game keeps in [`gm::Host::ext`].
#[derive(Clone, Debug)]
pub struct HostState {
    /// supersound.dll and its GML wrappers. Its sound files are shared between clones.
    pub sound: SuperSound,
}

impl HostState {
    pub fn new(sounds: Arc<SoundBank>, mode: Mode) -> HostState {
        HostState {
            sound: SuperSound::new(sounds, mode),
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
    fn clone_box(&self) -> Box<dyn Extension> {
        Box::new(self.clone())
    }

    /// Whether the states are the same, as savestates see them: not the shared bank, nor the commands
    /// not taken yet, which `SuperSound`'s `==` compares.
    fn eq_dyn(&self, other: &dyn Extension) -> bool {
        (other as &dyn Any)
            .downcast_ref::<HostState>()
            .is_some_and(|o| o.save() == self.save())
    }

    /// The audio model's state: mode, handles, positions, the paused list. Not the sound files, which
    /// aren't state, nor the pending commands, which are output (a player resyncs after loading).
    fn save(&self) -> Vec<u8> {
        // Serializing plain data into a Vec can't fail.
        postcard::to_allocvec(&self.sound).expect("postcard encoding into a Vec")
    }

    fn load(&mut self, bytes: &[u8]) -> Result<(), postcard::Error> {
        let (mut sound, rest): (SuperSound, _) = postcard::take_from_bytes(bytes)?;
        if !rest.is_empty() {
            return Err(postcard::Error::DeserializeBadEncoding);
        }
        sound.set_bank(self.sound.model().bank().clone());
        self.sound = sound;
        Ok(())
    }
}
