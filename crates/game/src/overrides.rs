//! The script override table (`docs/design/translation.md`, "Script overrides"): scripts whose
//! transpiled version must not run, replaced by hand-written Rust with the same signature.
//!
//! The transpiler still generates every script (`generated::scripts`), so each override keeps a generated twin to
//! diff against. Generated code calls scripts through [`scr`], which re-exports the generated scripts and
//! then names each override explicitly; an explicit `use` shadows a glob, so the override wins. Overrides
//! are only for things a headless, deterministic port can't run:
//!
//! - The 23 `SS_*` scripts wrap SuperSound (`supersound.dll`) through `external_define`/`external_call`.
//!   They call the audio model, `crates/audio`, in the world's [`HostState`](crate::HostState).
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

/// The SuperSound wrappers: each runs [`audio::SuperSound::call`] on the world's audio model, which models
/// the GML wrapper and the DLL together.
#[allow(non_snake_case)]
pub mod sound {
    use super::*;
    use crate::HostState;
    use audio::{EXTERNAL_GLOBALS, Script};

    /// Runs `script` on the world's audio model and sets the globals it assigns. The caller's cursor is
    /// left alone unless the script fails.
    ///
    /// The globals are the program's own (`HostState::dll_globals`, resolved by name when the world is
    /// made), not generated ids, so the settings program (`crates/config`), whose globals are numbered
    /// differently, shares these overrides. A program without them fails before the model changes.
    fn call(g: &mut World, unit: &'static str, script: Script, args: &[Value]) -> R<Value> {
        let host = HostState::get_mut(g).ok_or(Error::Unsupported(
            "SS_* scripts in a world without the game's host state",
        ))?;
        // Only `SS_Init` sets globals.
        let ids = host.dll_globals;
        if script == Script::Init && ids.is_none() {
            g.at(unit, 0);
            return Err(Error::Unsupported(
                "SS_Init in a program without supersound.dll's globals",
            ));
        }
        let out = match host.sound.call(script, args) {
            Ok(out) => out,
            Err(e) => {
                // The only GML error a wrapper raises: `real(argument1)` of a non-numeric string, on
                // `SS_SetSoundVol`'s line 15. Reported there, as the transpiled script would.
                let line = if script == Script::SetSoundVol { 15 } else { 0 };
                g.at(unit, line);
                return Err(e.into());
            }
        };
        // Only `SS_Init` sets globals: all of `EXTERNAL_GLOBALS`, in order.
        debug_assert!(out.globals.is_empty() || out.globals == EXTERNAL_GLOBALS);
        if let Some(ids) = ids {
            for (&(_, x), id) in out.globals.iter().zip(ids) {
                g.gset(id, Value::Real(x))?;
            }
        }
        Ok(out.value)
    }

    macro_rules! wire {
        ($($name:ident => $script:ident,)*) => {
            /// Each override's script, to check the table against [`Script::name`].
            #[cfg(test)]
            pub(super) const WIRED: &[(&str, Script)] = &[$((stringify!($name), Script::$script)),*];

            $(
                #[doc = concat!("`", stringify!($name), "`: [`Script::", stringify!($script), "`] on the audio model.")]
                pub fn $name(g: &mut World, _: &Ctx, args: &[Value]) -> R<Value> {
                    call(g, concat!("scripts/", stringify!($name)), Script::$script, args)
                }
            )*
        };
    }

    wire! {
        SS_Init => Init,
        SS_Unload => Unload,
        SS_LoadSound => LoadSound,
        SS_FreeSound => FreeSound,
        SS_PlaySound => PlaySound,
        SS_LoopSound => LoopSound,
        SS_StopSound => StopSound,
        SS_PauseSound => PauseSound,
        SS_ResumeSound => ResumeSound,
        SS_IsSoundPlaying => IsSoundPlaying,
        SS_IsSoundPaused => IsSoundPaused,
        SS_IsSoundLooping => IsSoundLooping,
        SS_IsHandleValid => IsHandleValid,
        SS_SetSoundFreq => SetSoundFreq,
        SS_SetSoundPan => SetSoundPan,
        SS_SetSoundVol => SetSoundVol,
        SS_GetSoundFreq => GetSoundFreq,
        SS_GetSoundPan => GetSoundPan,
        SS_GetSoundVol => GetSoundVol,
        SS_GetSoundPosition => GetSoundPosition,
        SS_SetSoundPosition => SetSoundPosition,
        SS_GetSoundLength => GetSoundLength,
        SS_GetSoundBytesPerSecond => GetSoundBytesPerSecond,
    }
}

#[cfg(test)]
mod tests {
    use super::sound::WIRED;
    use super::*;
    use crate::generated::names::SCRIPT_NAMES;
    use crate::{Config, Game, HostState, program};
    use audio::{EXTERNAL_GLOBALS, Mode, Script, SoundBank, SoundInfo};
    use gm::{Event, Inst, ObjectId};
    use std::sync::Arc;

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

    #[test]
    fn each_override_calls_its_own_script() {
        assert_eq!(WIRED.len(), Script::ALL.len());
        for &(name, script) in WIRED {
            assert_eq!(script.name(), name);
        }
    }

    fn sounds() -> Arc<SoundBank> {
        let mut bank = SoundBank::new();
        bank.insert_info(
            "sound/jump.wav",
            SoundInfo {
                sample_rate: 44100,
                channels: 1,
                frame_bytes: 2,
                frames: 44100,
            },
        );
        Arc::new(bank)
    }

    fn game(mode: Mode) -> Game {
        let config = Config {
            audio: mode,
            sounds: sounds(),
            ..Config::default()
        };
        Game::new(Arc::new(gm::Assets::default()), config)
    }

    /// Calls an override as generated code does.
    fn run(
        game: &mut Game,
        f: fn(&mut World, &Ctx, &[Value]) -> R<Value>,
        args: &[Value],
    ) -> Value {
        let cx = Ctx::event(Inst(0), Inst(0), Event::Create, ObjectId(0));
        f(&mut game.world, &cx, args).unwrap()
    }

    /// Calls an override with a handle as its only argument.
    fn on(game: &mut Game, f: fn(&mut World, &Ctx, &[Value]) -> R<Value>, h: &Value) -> Value {
        run(game, f, std::slice::from_ref(h))
    }

    fn s(x: &str) -> Value {
        Value::from(x)
    }

    fn r(x: f64) -> Value {
        Value::Real(x)
    }

    #[test]
    fn ss_init_sets_the_external_define_globals() {
        for mode in [Mode::Faithful, Mode::OracleCompat] {
            let mut game = game(mode);
            assert_eq!(run(&mut game, scr::SS_Init, &[]), r(0.0));
            for &(name, x) in &EXTERNAL_GLOBALS {
                let id = crate::global_named(program(), name).unwrap();
                assert_eq!(game.world.gget(id), Ok(r(x)), "{mode:?}");
            }
        }
    }

    #[test]
    fn faithful_overrides_play_on_the_model() {
        let mut game = game(Mode::Faithful);
        run(&mut game, scr::SS_Init, &[]);
        let snd = run(&mut game, scr::SS_LoadSound, &[s("sound/jump.wav"), r(0.0)]);
        assert!(!snd.is_real(), "{snd:?}");
        assert_eq!(
            run(&mut game, scr::SS_LoadSound, &[s("sound/none.wav"), r(0.0)]),
            s("No")
        );
        assert_eq!(on(&mut game, scr::SS_IsSoundPlaying, &snd), r(0.0));
        on(&mut game, scr::SS_PlaySound, &snd);
        assert_eq!(on(&mut game, scr::SS_IsSoundPlaying, &snd), r(1.0));
        assert_eq!(on(&mut game, scr::SS_GetSoundFreq, &snd), r(44100.0));
        assert!(!game.take_audio_commands().is_empty());
        assert!(game.take_audio_commands().is_empty());
    }

    #[test]
    fn oracle_compat_overrides_answer_like_the_stub() {
        let mut game = game(Mode::OracleCompat);
        run(&mut game, scr::SS_Init, &[]);
        let snd = run(&mut game, scr::SS_LoadSound, &[s("sound/jump.wav"), r(0.0)]);
        assert_eq!(snd, r(0.0));
        on(&mut game, scr::SS_PlaySound, &snd);
        assert_eq!(on(&mut game, scr::SS_IsSoundPlaying, &snd), r(-1.0));
        assert_eq!(on(&mut game, scr::SS_GetSoundVol, &snd), s("-1"));
        assert!(game.take_audio_commands().is_empty());
    }

    #[test]
    fn savestates_hold_the_audio_model() {
        let mut game = game(Mode::Faithful);
        run(&mut game, scr::SS_Init, &[]);
        let snd = run(&mut game, scr::SS_LoadSound, &[s("sound/jump.wav"), r(0.0)]);
        let other = run(&mut game, scr::SS_LoadSound, &[s("sound/jump.wav"), r(0.0)]);
        on(&mut game, scr::SS_LoopSound, &snd);
        run(&mut game, scr::SS_SetSoundVol, &[snd.clone(), r(5000.0)]);
        on(&mut game, scr::SS_PlaySound, &other);
        on(&mut game, scr::SS_PauseSound, &other);
        for _ in 0..7 {
            HostState::get_mut(&mut game.world).unwrap().sound.tick();
        }
        game.take_audio_commands();

        let bytes = game.save_state();
        let assets = Arc::new(gm::Assets::default());
        let mut loaded = Game::load_state(assets.clone(), sounds(), &bytes).unwrap();
        assert_eq!(loaded.audio(), game.audio());
        assert_eq!(loaded.save_state(), bytes);
        let position = on(&mut game, scr::SS_GetSoundPosition, &snd);
        assert_ne!(position, r(0.0));
        assert_eq!(on(&mut loaded, scr::SS_GetSoundPosition, &snd), position);
        assert_eq!(on(&mut loaded, scr::SS_IsSoundPaused, &other), r(1.0));
        // The bank came back: loading still finds the files.
        let third = run(
            &mut loaded,
            scr::SS_LoadSound,
            &[s("sound/jump.wav"), r(0.0)],
        );
        assert!(!third.is_real());

        // The mode is part of the state, whatever the loader's default.
        let stub = game_with_init(Mode::OracleCompat).save_state();
        let loaded = Game::load_state(assets.clone(), sounds(), &stub).unwrap();
        assert_eq!(loaded.audio().mode(), Mode::OracleCompat);

        // A loader without the game's host state rejects it, and so does one with other sound files.
        assert!(
            World::load_state(
                program(),
                assets.clone(),
                crate::generated::FONT_SLOTS,
                &bytes
            )
            .is_err()
        );
        assert!(Game::load_state(assets, Arc::default(), &bytes).is_err());
    }

    #[test]
    fn loading_rejects_audio_states_the_model_cant_be_in() {
        use gm::Extension;
        // A `HostState` as `save` encodes it: the bank's fingerprint, then an initialized Faithful
        // model (variant 0) with no sounds, the given next handle, and nothing paused; no programs started.
        let encode = |next_handle: i32| {
            let fingerprint = sounds().fingerprint();
            postcard::to_allocvec(&(fingerprint, 0u32, true, 0u32, next_handle, 0u32, 0u32))
                .unwrap()
        };
        let mut h = HostState::new(sounds(), Mode::Faithful, program());
        h.load(&encode(1)).unwrap();
        assert!(
            h.sound
                .same_state(&game_with_init(Mode::Faithful).audio().clone())
        );
        // The next load would overflow.
        assert!(h.load(&encode(i32::MAX)).is_err());
        assert!(h.load(&encode(0)).is_err());
    }

    #[test]
    fn clones_start_without_pending_commands() {
        let mut game = game(Mode::Faithful);
        run(&mut game, scr::SS_Init, &[]);
        let snd = run(&mut game, scr::SS_LoadSound, &[s("sound/jump.wav"), r(0.0)]);
        on(&mut game, scr::SS_PlaySound, &snd);
        let mut copy = game.clone();
        assert!(copy.take_audio_commands().is_empty());
        assert!(!game.take_audio_commands().is_empty());
        assert!(
            HostState::get(&copy.world).unwrap() as &dyn gm::Extension
                == HostState::get(&game.world).unwrap() as &dyn gm::Extension
        );
    }

    #[test]
    fn audio_stops_with_the_game() {
        let mut game = game(Mode::Faithful);
        run(&mut game, scr::SS_Init, &[]);
        let snd = run(&mut game, scr::SS_LoadSound, &[s("sound/jump.wav"), r(0.0)]);
        on(&mut game, scr::SS_LoopSound, &snd);
        game.world.game_end();
        let before = game.audio().clone();
        game.step(&[]).unwrap();
        assert!(game.audio().same_state(&before));
    }

    #[test]
    fn overrides_keep_the_callers_cursor_and_locate_their_errors() {
        let mut game = game(Mode::Faithful);
        run(&mut game, scr::SS_Init, &[]);
        let snd = run(&mut game, scr::SS_LoadSound, &[s("sound/jump.wav"), r(0.0)]);
        game.world.at("oCaller", 7);
        on(&mut game, scr::SS_PlaySound, &snd);
        assert_eq!(game.world.cursor(), ("oCaller", 7));
        let cx = Ctx::event(Inst(0), Inst(0), Event::Create, ObjectId(0));
        assert!(scr::SS_SetSoundVol(&mut game.world, &cx, &[snd, s("loud")]).is_err());
        assert_eq!(game.world.cursor(), ("scripts/SS_SetSoundVol", 15));
    }

    fn game_with_init(mode: Mode) -> Game {
        let mut game = game(mode);
        run(&mut game, scr::SS_Init, &[]);
        game
    }
}
