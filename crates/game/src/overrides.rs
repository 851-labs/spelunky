//! The script override table (`docs/design/translation.md`, "Script overrides"): scripts whose
//! transpiled version must not run, replaced by hand-written Rust with the same signature.
//!
//! The transpiler still generates every script (`generated::scripts`), so each override keeps a generated twin to
//! diff against. Generated code calls scripts through [`scr`], which re-exports the generated scripts and
//! then names each override explicitly; an explicit `use` shadows a glob, so the override wins. Overrides
//! are only for things a headless, deterministic port can't run:
//!
//! - The 23 `SS_*` scripts wrap SuperSound (`supersound.dll`) through `external_define`/`external_call`.
//!   They go to the audio model, `crates/audio` (851-3132); wiring it in is 851-3177.
//!
//! The file-backed scripts translation.md mentions (`scrReadStats`, `scrWriteStats`, the config reading in
//! `scrInit`) are *not* overridden: they only use `file_text_*`, so they stay transpiled and those
//! built-ins read and write a virtual file system (851-3164). That keeps their parsing logic, quirks
//! included, exactly the original's.

use gm::{Ctx, Error, R, Value, World};

macro_rules! override_table {
    ($($module:ident :: $name:ident,)*) => {
        /// The overridden scripts, by GML name.
        pub const OVERRIDDEN: &[&str] = &[$(stringify!($name)),*];

        /// Every script as generated code calls it (`scr::name(g, cx, &args)`): the transpiled version,
        /// unless the override table replaces it.
        pub mod scr {
            pub use crate::generated::scripts::*;
            $(pub use crate::overrides::$module::$name;)*
        }
    };
}

override_table! {
    sound::SS_Init,
    sound::SS_Unload,
    sound::SS_LoadSound,
    sound::SS_FreeSound,
    sound::SS_PlaySound,
    sound::SS_LoopSound,
    sound::SS_StopSound,
    sound::SS_PauseSound,
    sound::SS_ResumeSound,
    sound::SS_IsSoundPlaying,
    sound::SS_IsSoundPaused,
    sound::SS_IsSoundLooping,
    sound::SS_IsHandleValid,
    sound::SS_SetSoundFreq,
    sound::SS_SetSoundPan,
    sound::SS_SetSoundVol,
    sound::SS_GetSoundFreq,
    sound::SS_GetSoundPan,
    sound::SS_GetSoundVol,
    sound::SS_GetSoundPosition,
    sound::SS_SetSoundPosition,
    sound::SS_GetSoundLength,
    sound::SS_GetSoundBytesPerSecond,
}

/// The SuperSound wrappers.
#[allow(non_snake_case)]
pub mod sound {
    use super::*;

    macro_rules! placeholder {
        ($($name:ident)*) => {$(
            // TODO(851-3177): call `audio::ss::SuperSound::call` (crates/audio, 851-3132), which needs a
            // place for the SuperSound state in the world, and set the `Outcome`'s globals.
            #[doc = concat!("`", stringify!($name), "`: not wired to the audio model yet (851-3177).")]
            pub fn $name(_: &mut World, _: &Ctx, _: &[Value]) -> R<Value> {
                Err(Error::Unsupported(concat!(stringify!($name), " (851-3177)")))
            }
        )*};
    }

    placeholder! {
        SS_Init SS_Unload SS_LoadSound SS_FreeSound SS_PlaySound SS_LoopSound SS_StopSound SS_PauseSound
        SS_ResumeSound SS_IsSoundPlaying SS_IsSoundPaused SS_IsSoundLooping SS_IsHandleValid
        SS_SetSoundFreq SS_SetSoundPan SS_SetSoundVol SS_GetSoundFreq SS_GetSoundPan SS_GetSoundVol
        SS_GetSoundPosition SS_SetSoundPosition SS_GetSoundLength SS_GetSoundBytesPerSecond
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::names::SCRIPT_NAMES;

    #[test]
    fn every_override_replaces_a_script() {
        for name in OVERRIDDEN {
            assert!(SCRIPT_NAMES.contains(name), "{name} is not a script");
        }
        let ss: Vec<_> = SCRIPT_NAMES
            .iter()
            .filter(|s| s.starts_with("SS_"))
            .collect();
        assert_eq!(ss.len(), 23);
        assert!(ss.iter().all(|s| OVERRIDDEN.contains(s)));
    }
}
