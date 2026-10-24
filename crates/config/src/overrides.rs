//! The settings program's script overrides: the game's `SS_*` overrides ([`game::overrides`]), for the
//! same 23 SuperSound wrappers, which `config.gmk` carries a copy of (it plays the cave music and the jump
//! sound while a volume slider is dragged). Nothing else is overridden; `scrSaveSettings` and the config
//! reading in `oInit` stay transpiled over the virtual game directory, as in the game.

macro_rules! override_table {
    ($($name:ident,)*) => {
        /// The overridden scripts, by GML name.
        pub const OVERRIDDEN: &[&str] = &[$(stringify!($name)),*];

        /// Every script as generated code calls it: the transpiled version, unless overridden. An
        /// explicit `use` shadows the glob, so the override wins.
        pub mod scr {
            pub use crate::generated::scripts::*;
            $(pub use game::overrides::sound::$name;)*
        }
    };
}

override_table! {
    SS_Init,
    SS_Unload,
    SS_LoadSound,
    SS_FreeSound,
    SS_PlaySound,
    SS_LoopSound,
    SS_StopSound,
    SS_PauseSound,
    SS_ResumeSound,
    SS_IsSoundPlaying,
    SS_IsSoundPaused,
    SS_IsSoundLooping,
    SS_IsHandleValid,
    SS_SetSoundFreq,
    SS_SetSoundPan,
    SS_SetSoundVol,
    SS_GetSoundFreq,
    SS_GetSoundPan,
    SS_GetSoundVol,
    SS_GetSoundPosition,
    SS_SetSoundPosition,
    SS_GetSoundLength,
    SS_GetSoundBytesPerSecond,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::names::SCRIPT_NAMES;

    #[test]
    fn every_supersound_wrapper_is_the_games_override() {
        let ss: Vec<_> = SCRIPT_NAMES
            .iter()
            .filter(|s| s.starts_with("SS_"))
            .collect();
        assert_eq!(ss.len(), OVERRIDDEN.len());
        assert!(ss.iter().all(|s| OVERRIDDEN.contains(s)));
        assert_eq!(OVERRIDDEN, game::overrides::OVERRIDDEN);
    }
}
