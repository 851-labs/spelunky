//! The game's 23 `SS_*` scripts, exactly as their GML wraps the DLL, for the `game` crate's overrides.
//!
//! Each wrapper first checks `is_real(handle)` and returns a fixed value without calling the DLL, then
//! passes strings to `external_call` and converts the string it gets back. That is reproduced here
//! value for value, including the scripts' inconsistent return types (`SS_GetSoundVol` returns the string
//! `"-1"` for a real handle) and the [`Mode`]: in [`Mode::OracleCompat`] the DLL is replaced by the
//! oracle's stub, which makes `SS_LoadSound` return the real 0, so every later call takes the
//! `is_real` branch.

use crate::model::{AudioModel, Command, Handle, SoundBank};
use gm::{GmString, Value, ValueError};
use std::sync::Arc;

/// Which supersound.dll answers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Mode {
    /// The real DLL, as modeled by [`AudioModel`]. For play.
    #[default]
    Faithful,
    /// OpenGMK's stub, as the reference oracle runs the game: `SS_Init` (and so `SS_Unload`) returns
    /// `"Yes"` and every other export the real 0. Nothing is loaded, nothing plays, and the queries give
    /// the wrappers' `is_real` answers. For runs diffed against the oracle.
    OracleCompat,
}

/// The `SS_*` GML scripts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Script {
    Init,
    Unload,
    LoadSound,
    PlaySound,
    LoopSound,
    StopSound,
    PauseSound,
    ResumeSound,
    FreeSound,
    SetSoundFreq,
    SetSoundPan,
    SetSoundVol,
    SetSoundPosition,
    GetSoundFreq,
    GetSoundPan,
    GetSoundVol,
    GetSoundPosition,
    GetSoundLength,
    GetSoundBytesPerSecond,
    IsSoundPlaying,
    IsSoundLooping,
    IsSoundPaused,
    IsHandleValid,
}

impl Script {
    pub const ALL: [Script; 23] = [
        Self::Init,
        Self::Unload,
        Self::LoadSound,
        Self::PlaySound,
        Self::LoopSound,
        Self::StopSound,
        Self::PauseSound,
        Self::ResumeSound,
        Self::FreeSound,
        Self::SetSoundFreq,
        Self::SetSoundPan,
        Self::SetSoundVol,
        Self::SetSoundPosition,
        Self::GetSoundFreq,
        Self::GetSoundPan,
        Self::GetSoundVol,
        Self::GetSoundPosition,
        Self::GetSoundLength,
        Self::GetSoundBytesPerSecond,
        Self::IsSoundPlaying,
        Self::IsSoundLooping,
        Self::IsSoundPaused,
        Self::IsHandleValid,
    ];

    /// The script's name in the game, e.g. `SS_PlaySound`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Init => "SS_Init",
            Self::Unload => "SS_Unload",
            Self::LoadSound => "SS_LoadSound",
            Self::PlaySound => "SS_PlaySound",
            Self::LoopSound => "SS_LoopSound",
            Self::StopSound => "SS_StopSound",
            Self::PauseSound => "SS_PauseSound",
            Self::ResumeSound => "SS_ResumeSound",
            Self::FreeSound => "SS_FreeSound",
            Self::SetSoundFreq => "SS_SetSoundFreq",
            Self::SetSoundPan => "SS_SetSoundPan",
            Self::SetSoundVol => "SS_SetSoundVol",
            Self::SetSoundPosition => "SS_SetSoundPosition",
            Self::GetSoundFreq => "SS_GetSoundFreq",
            Self::GetSoundPan => "SS_GetSoundPan",
            Self::GetSoundVol => "SS_GetSoundVol",
            Self::GetSoundPosition => "SS_GetSoundPosition",
            Self::GetSoundLength => "SS_GetSoundLength",
            Self::GetSoundBytesPerSecond => "SS_GetSoundBytesPerSecond",
            Self::IsSoundPlaying => "SS_IsSoundPlaying",
            Self::IsSoundLooping => "SS_IsSoundLooping",
            Self::IsSoundPaused => "SS_IsSoundPaused",
            Self::IsHandleValid => "SS_IsHandleValid",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.name() == name)
    }
}

/// The globals `SS_Init` assigns `external_define`'s results to, in order. The override has to set them
/// too, or a `--fields` diff against the oracle shows them missing. OpenGMK numbers externals from 0 in
/// definition order (reusing freed slots), so with no other DLL in the game they are 0 to 21 there.
/// Verify what the real GM 8 returns.
pub const EXTERNAL_GLOBALS: [&str; 22] = [
    "dll_SS_Init",
    "dll_SS_Unload",
    "dll_SS_LoadSound",
    "dll_SS_PlaySound",
    "dll_SS_LoopSound",
    "dll_SS_StopSound",
    "dll_SS_SetSoundFreq",
    "dll_SS_SetSoundPan",
    "dll_SS_SetSoundVol",
    "dll_SS_FreeSound",
    "dll_SS_IsSoundPlaying",
    "dll_SS_GetSoundPosition",
    "dll_SS_SetSoundPosition",
    "dll_SS_GetSoundLength",
    "dll_SS_GetSoundBytesPerSecond",
    "dll_SS_GetSoundVol",
    "dll_SS_GetSoundFreq",
    "dll_SS_GetSoundPan",
    "dll_SS_IsSoundLooping",
    "dll_SS_IsSoundPaused",
    "dll_SS_ResumeSound",
    "dll_SS_PauseSound",
];

/// The DLL symbols the scripts call. `SS_Unload`'s binding names the `SS_Init` symbol, so it has no
/// entry of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Export {
    Init,
    LoadSound,
    PlaySound,
    LoopSound,
    StopSound,
    PauseSound,
    ResumeSound,
    FreeSound,
    SetSoundFreq,
    SetSoundPan,
    SetSoundVol,
    SetSoundPosition,
    GetSoundFreq,
    GetSoundPan,
    GetSoundVol,
    GetSoundPosition,
    GetSoundLength,
    GetSoundBytesPerSecond,
    IsSoundPlaying,
    IsSoundLooping,
    IsSoundPaused,
}

/// supersound.dll and its GML wrappers: what the `SS_*` overrides call. Lives in the game's host state;
/// a plain value, so `Clone` is a savestate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SuperSound {
    mode: Mode,
    model: AudioModel,
}

impl SuperSound {
    pub fn new(bank: Arc<SoundBank>, mode: Mode) -> Self {
        Self {
            mode,
            model: AudioModel::new(bank),
        }
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn model(&self) -> &AudioModel {
        &self.model
    }

    /// Advances the model one game frame ([`AudioModel::tick`]).
    pub fn tick(&mut self) {
        self.model.tick();
    }

    /// This frame's commands for a player ([`AudioModel::take_commands`]). Always empty in
    /// [`Mode::OracleCompat`].
    pub fn take_commands(&mut self) -> Vec<Command> {
        self.model.take_commands()
    }

    /// Runs the GML script `script` with `args` (missing arguments read as the real 0, as in GML). Errors
    /// are the GML runtime errors the script itself would raise: only `real()` of a non-numeric string
    /// volume in `SS_SetSoundVol`.
    pub fn call(&mut self, script: Script, args: &[Value]) -> Result<Value, ValueError> {
        let arg = |i: usize| args.get(i).cloned().unwrap_or_default();
        let h = arg(0);
        let real = |v: Value| v.to_gml_real().map(Value::Real);
        // `if is_real(argument0) then return <value>`, the first line of every wrapper but SS_Init's.
        let early = |v: Value| h.is_real().then_some(v);
        Ok(match script {
            // `if external_call(...) = "No" then show_error(...)`: the DLL never says "No" here, and
            // neither does the stub. Neither script returns a value.
            Script::Init | Script::Unload => {
                self.external(Export::Init, &[]);
                Value::Real(0.0)
            }
            Script::LoadSound => match early(Value::from("0")) {
                Some(v) => v,
                None => self.external(Export::LoadSound, &[h, arg(1).to_gml_string().into()]),
            },
            Script::PlaySound | Script::LoopSound | Script::StopSound => {
                let export = match script {
                    Script::PlaySound => Export::PlaySound,
                    Script::LoopSound => Export::LoopSound,
                    _ => Export::StopSound,
                };
                match early(Value::from("0")) {
                    Some(v) => v,
                    None => self.external(export, &[h]),
                }
            }
            Script::FreeSound => match early(Value::from("0")) {
                Some(v) => v,
                None => {
                    self.external(Export::StopSound, std::slice::from_ref(&h));
                    self.external(Export::FreeSound, &[h])
                }
            },
            Script::PauseSound | Script::ResumeSound => {
                let export = if script == Script::PauseSound {
                    Export::PauseSound
                } else {
                    Export::ResumeSound
                };
                match early(Value::Real(-1.0)) {
                    Some(v) => v,
                    None => real(self.external(export, &[h]))?,
                }
            }
            Script::SetSoundFreq | Script::SetSoundPan => {
                let export = if script == Script::SetSoundFreq {
                    Export::SetSoundFreq
                } else {
                    Export::SetSoundPan
                };
                match early(Value::from("0")) {
                    Some(v) => v,
                    None => {
                        let x = arg(1);
                        let x = if x.is_string() {
                            x
                        } else {
                            x.to_gml_string().into()
                        };
                        self.external(export, &[h, x])
                    }
                }
            }
            Script::SetSoundVol => match early(Value::from("0")) {
                Some(v) => v,
                None => {
                    // The wrapper converts the game's 0..10000 to DirectSound's -10000..0.
                    let x = arg(1);
                    let x = if x.is_string() {
                        x.to_gml_real()?
                    } else {
                        x.to_real()
                    } - 10000.0;
                    self.external(
                        Export::SetSoundVol,
                        &[h, Value::Real(x).to_gml_string().into()],
                    )
                }
            },
            Script::SetSoundPosition => {
                if h.is_real() || arg(1).is_string() {
                    Value::Real(0.0)
                } else {
                    let x = arg(1).to_gml_string().into();
                    // `+1`: success is 2 and failure 1, though the script's comment promises 1 and 0.
                    let r = self.external(Export::SetSoundPosition, &[h, x]);
                    Value::Real(r.to_gml_real()? + 1.0)
                }
            }
            Script::GetSoundVol => match early(Value::from("-1")) {
                Some(v) => v,
                None => {
                    let r = self.external(Export::GetSoundVol, &[h]);
                    Value::Real(r.to_gml_real()? + 10000.0)
                }
            },
            Script::GetSoundPan => match early(Value::from("0")) {
                Some(v) => v,
                None => real(self.external(Export::GetSoundPan, &[h]))?,
            },
            Script::GetSoundFreq
            | Script::GetSoundPosition
            | Script::GetSoundLength
            | Script::GetSoundBytesPerSecond
            | Script::IsSoundPlaying
            | Script::IsSoundLooping
            | Script::IsSoundPaused => {
                let (export, if_real) = match script {
                    Script::GetSoundFreq => (Export::GetSoundFreq, -1.0),
                    Script::GetSoundPosition => (Export::GetSoundPosition, -1.0),
                    Script::GetSoundLength => (Export::GetSoundLength, 0.0),
                    Script::GetSoundBytesPerSecond => (Export::GetSoundBytesPerSecond, 0.0),
                    Script::IsSoundPlaying => (Export::IsSoundPlaying, -1.0),
                    Script::IsSoundLooping => (Export::IsSoundLooping, -1.0),
                    _ => (Export::IsSoundPaused, -1.0),
                };
                match early(Value::Real(if_real)) {
                    Some(v) => v,
                    None => real(self.external(export, &[h]))?,
                }
            }
            Script::IsHandleValid => match early(Value::Real(-1.0)) {
                Some(v) => v,
                // A `switch` on the string with cases "1", "0" and "-1" and no default: falling out of
                // it returns 0, as does the stub's real 0 (a case never matches another type).
                None => match self.external(Export::IsSoundPlaying, &[h]) {
                    Value::Str(s) if matches!(s.as_bytes(), b"1" | b"0") => Value::Real(1.0),
                    _ => Value::Real(0.0),
                },
            },
        })
    }

    /// `external_call`: every argument and result is a string (`ty_string`).
    fn external(&mut self, export: Export, args: &[Value]) -> Value {
        if self.mode == Mode::OracleCompat {
            // OpenGMK `gm8emulator/src/game/external.rs` (`should_dummy`): by symbol name.
            return if export == Export::Init {
                Value::from("Yes")
            } else {
                Value::Real(0.0)
            };
        }
        let text = |i: usize| args.get(i).map_or(&b""[..], Value::to_str);
        let h: Handle = pb_val(text(0));
        let x = pb_val(text(1));
        let m = &mut self.model;
        let flag = |b: bool| Value::from(if b { "1" } else { "0" });
        let num = |n: i64| Value::Str(GmString::from(n.to_string()));
        match export {
            Export::Init => Value::from(if m.init() { "Yes" } else { "No" }),
            Export::LoadSound => {
                // GM strings are bytes; the game's paths are ASCII.
                let path = String::from_utf8_lossy(text(0));
                match m.load_sound(&path, x) {
                    Some(h) => num(h.into()),
                    None => Value::from("No"),
                }
            }
            Export::PlaySound | Export::LoopSound | Export::StopSound | Export::FreeSound => {
                match export {
                    Export::PlaySound => m.play_sound(h),
                    Export::LoopSound => m.loop_sound(h),
                    Export::StopSound => m.stop_sound(h),
                    _ => m.free_sound(h),
                }
                Value::from("0")
            }
            Export::PauseSound => flag(m.pause_sound(h)),
            Export::ResumeSound => flag(m.resume_sound(h)),
            Export::SetSoundFreq => flag(m.set_sound_freq(h, x)),
            Export::SetSoundPan => flag(m.set_sound_pan(h, x)),
            Export::SetSoundVol => flag(m.set_sound_vol(h, x)),
            Export::SetSoundPosition => flag(m.set_sound_position(h, x)),
            Export::GetSoundFreq => m.sound_freq(h).map_or(num(-1), |f| num(f.into())),
            Export::GetSoundPan => m.sound_pan(h).map_or(num(0), |p| num(p.into())),
            // The DLL's GetSoundVol and GetSoundBytesPerSecond are stdcall with two parameters, but GM
            // passes one. Verify what that does to the stack (only the unused object `core` calls them).
            Export::GetSoundVol => m.sound_vol(h).map_or(num(-1), |v| num(v.into())),
            Export::GetSoundBytesPerSecond => m
                .sound_bytes_per_second(h)
                .map_or(num(-1), |b| num(b.into())),
            Export::GetSoundPosition => m.sound_position(h).map_or(num(-1), num),
            Export::GetSoundLength => m.sound_length(h).map_or(num(-1), num),
            Export::IsSoundPlaying => m.is_sound_playing(h).map_or(num(-1), flag),
            Export::IsSoundLooping => m.is_sound_looping(h).map_or(num(-1), flag),
            Export::IsSoundPaused => flag(m.is_sound_paused(h)),
        }
    }
}

/// PureBasic's `Val()`, which the DLL applies to every numeric argument: an optional sign and decimal
/// digits, stopping at the first other character, so `"-1333.33"` is -1333 and `"abc"` is 0. Truncated
/// to 32 bits, as the DLL passes it on. Verify how it treats leading spaces (the game never sends any).
pub fn pb_val(s: &[u8]) -> i32 {
    let s = s.trim_ascii_start();
    let (negative, digits) = match s.first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let n = digits
        .iter()
        .take_while(|b| b.is_ascii_digit())
        .fold(0i64, |n, &d| {
            n.wrapping_mul(10).wrapping_add(i64::from(d - b'0'))
        });
    (if negative { n.wrapping_neg() } else { n }) as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::{SoundInfo, wav};

    fn bank() -> Arc<SoundBank> {
        let mut bank = SoundBank::new();
        bank.insert("sound/jump.wav", &wav(44100, 1, 16, 88200))
            .unwrap();
        bank.insert_info(
            "sound/cave.ogg",
            SoundInfo {
                sample_rate: 44100,
                channels: 2,
                frame_bytes: 4,
                frames: 44100 * 60,
            },
        );
        Arc::new(bank)
    }

    fn s(x: &str) -> Value {
        Value::from(x)
    }

    fn r(x: f64) -> Value {
        Value::Real(x)
    }

    /// `oMusic`'s create event: SS_Init, then load an effect and a streamed track.
    fn booted(mode: Mode) -> (SuperSound, Value, Value) {
        let mut ss = SuperSound::new(bank(), mode);
        assert_eq!(ss.call(Script::Init, &[]), Ok(r(0.0)));
        let snd = ss
            .call(Script::LoadSound, &[s("sound/jump.wav"), r(0.0)])
            .unwrap();
        let mus = ss
            .call(Script::LoadSound, &[s("sound/cave.ogg"), r(1.0)])
            .unwrap();
        (ss, snd, mus)
    }

    /// Calls a script with a handle as its only argument.
    fn on(ss: &mut SuperSound, script: Script, h: &Value) -> Value {
        ss.call(script, std::slice::from_ref(h)).unwrap()
    }

    /// `playMusic(mus, true)` with `global.music` on.
    fn play_music(ss: &mut SuperSound, mus: &Value) {
        let playing = on(ss, Script::IsSoundPlaying, mus);
        if !playing.is_truthy() {
            on(ss, Script::LoopSound, mus);
        }
    }

    #[test]
    fn names_round_trip() {
        for script in Script::ALL {
            assert_eq!(Script::from_name(script.name()), Some(script));
        }
        assert_eq!(Script::from_name("playSound"), None);
    }

    #[test]
    fn faithful_answers() {
        let (mut ss, snd, mus) = booted(Mode::Faithful);
        assert_eq!((snd.clone(), mus.clone()), (s("1"), s("2")));
        assert_eq!(
            ss.call(Script::LoadSound, &[s("sound/nope.wav")]),
            Ok(s("No"))
        );
        let call = |ss: &mut SuperSound, script, args: &[Value]| ss.call(script, args).unwrap();

        assert_eq!(on(&mut ss, Script::IsSoundPlaying, &snd), r(0.0));
        assert_eq!(on(&mut ss, Script::PlaySound, &snd), s("0"));
        assert_eq!(on(&mut ss, Script::IsSoundPlaying, &snd), r(1.0));
        assert_eq!(on(&mut ss, Script::IsSoundLooping, &snd), r(0.0));
        assert_eq!(on(&mut ss, Script::IsHandleValid, &snd), r(1.0));

        // The music volume formula at musicVol 15: 8666.67 reaches the DLL as "-1333.33", read as -1333.
        let vol = 2000.0 + 8000.0 * (15.0 / 18.0);
        assert_eq!(
            call(&mut ss, Script::SetSoundVol, &[mus.clone(), r(vol)]),
            s("1")
        );
        assert_eq!(ss.model().sound_vol(2), Some(-1333));
        assert_eq!(on(&mut ss, Script::GetSoundVol, &mus), r(8667.0));
        // A string volume goes through real().
        assert_eq!(
            call(&mut ss, Script::SetSoundVol, &[mus.clone(), s("5000")]),
            s("1")
        );
        assert_eq!(on(&mut ss, Script::GetSoundVol, &mus), r(5000.0));
        assert_eq!(
            ss.call(Script::SetSoundVol, &[mus.clone(), s("loud")]),
            Err(ValueError::BadRealString)
        );

        assert_eq!(
            call(&mut ss, Script::SetSoundPan, &[snd.clone(), r(-10000.0)]),
            s("1")
        );
        assert_eq!(on(&mut ss, Script::GetSoundPan, &snd), r(-10000.0));

        assert_eq!(on(&mut ss, Script::GetSoundFreq, &mus), r(44100.0));
        assert_eq!(on(&mut ss, Script::GetSoundLength, &mus), r(65536.0));
        assert_eq!(on(&mut ss, Script::GetSoundLength, &snd), r(88200.0));
        assert_eq!(
            on(&mut ss, Script::GetSoundBytesPerSecond, &snd),
            r(88200.0)
        );

        ss.tick();
        assert_eq!(on(&mut ss, Script::GetSoundPosition, &snd), r(2940.0));
        assert_eq!(
            call(&mut ss, Script::SetSoundPosition, &[snd.clone(), r(100.0)]),
            r(2.0)
        );
        assert_eq!(
            call(&mut ss, Script::SetSoundPosition, &[snd.clone(), s("100")]),
            r(0.0)
        );
        assert_eq!(on(&mut ss, Script::GetSoundPosition, &snd), r(100.0));

        assert_eq!(on(&mut ss, Script::PauseSound, &snd), r(1.0));
        assert_eq!(on(&mut ss, Script::IsSoundPaused, &snd), r(1.0));
        assert_eq!(on(&mut ss, Script::IsSoundPlaying, &snd), r(0.0));
        assert_eq!(on(&mut ss, Script::ResumeSound, &snd), r(1.0));
        assert_eq!(on(&mut ss, Script::ResumeSound, &snd), r(0.0));
        assert_eq!(on(&mut ss, Script::GetSoundPosition, &snd), r(100.0));

        // Handles the DLL doesn't know.
        let bad = s("99");
        assert_eq!(on(&mut ss, Script::IsSoundPlaying, &bad), r(-1.0));
        assert_eq!(on(&mut ss, Script::IsSoundLooping, &bad), r(-1.0));
        assert_eq!(on(&mut ss, Script::IsSoundPaused, &bad), r(0.0));
        assert_eq!(on(&mut ss, Script::IsHandleValid, &bad), r(0.0));
        assert_eq!(on(&mut ss, Script::GetSoundFreq, &bad), r(-1.0));
        assert_eq!(on(&mut ss, Script::GetSoundPan, &bad), r(0.0));
        assert_eq!(on(&mut ss, Script::GetSoundVol, &bad), r(9999.0));
        assert_eq!(on(&mut ss, Script::GetSoundPosition, &bad), r(-1.0));
        assert_eq!(
            call(&mut ss, Script::SetSoundVol, &[bad.clone(), r(0.0)]),
            s("0")
        );
        assert_eq!(
            call(&mut ss, Script::SetSoundPosition, &[bad.clone(), r(0.0)]),
            r(1.0)
        );
        assert_eq!(on(&mut ss, Script::PauseSound, &bad), r(0.0));

        assert_eq!(on(&mut ss, Script::FreeSound, &snd), s("0"));
        assert_eq!(on(&mut ss, Script::IsSoundPlaying, &snd), r(-1.0));
        assert_eq!(call(&mut ss, Script::Unload, &[]), r(0.0));
        assert_eq!(call(&mut ss, Script::IsSoundPlaying, &[mus]), r(-1.0));
    }

    #[test]
    fn oracle_compat_answers() {
        let (mut ss, snd, mus) = booted(Mode::OracleCompat);
        // OpenGMK's stub returns the real 0 for SS_LoadSound...
        assert_eq!((snd.clone(), mus.clone()), (r(0.0), r(0.0)));
        // ...so every wrapper takes its `is_real` branch.
        let expected = [
            (Script::PlaySound, s("0")),
            (Script::LoopSound, s("0")),
            (Script::StopSound, s("0")),
            (Script::FreeSound, s("0")),
            (Script::PauseSound, r(-1.0)),
            (Script::ResumeSound, r(-1.0)),
            (Script::SetSoundFreq, s("0")),
            (Script::SetSoundPan, s("0")),
            (Script::SetSoundVol, s("0")),
            (Script::SetSoundPosition, r(0.0)),
            (Script::GetSoundFreq, r(-1.0)),
            (Script::GetSoundPan, s("0")),
            (Script::GetSoundVol, s("-1")),
            (Script::GetSoundPosition, r(-1.0)),
            (Script::GetSoundLength, r(0.0)),
            (Script::GetSoundBytesPerSecond, r(0.0)),
            (Script::IsSoundPlaying, r(-1.0)),
            (Script::IsSoundLooping, r(-1.0)),
            (Script::IsSoundPaused, r(-1.0)),
            (Script::IsHandleValid, r(-1.0)),
        ];
        for (script, answer) in expected {
            assert_eq!(
                ss.call(script, &[snd.clone(), r(1.0)]),
                Ok(answer),
                "{}",
                script.name()
            );
        }
        // A string handle reaches the stub, which answers the real 0 to everything but SS_Init.
        assert_eq!(ss.call(Script::IsSoundPlaying, &[s("1")]), Ok(r(0.0)));
        assert_eq!(ss.call(Script::GetSoundVol, &[s("1")]), Ok(r(10000.0)));
        assert_eq!(ss.call(Script::PlaySound, &[s("1")]), Ok(r(0.0)));
        assert_eq!(ss.call(Script::Unload, &[]), Ok(r(0.0)));
        ss.tick();
        assert!(ss.take_commands().is_empty());
        assert_eq!(ss.model().sounds().count(), 0);
    }

    #[test]
    fn play_music_restarts_only_in_oracle_compat() {
        // Faithful: the second playMusic sees the track playing and leaves it alone.
        let (mut ss, _, mus) = booted(Mode::Faithful);
        play_music(&mut ss, &mus);
        for _ in 0..30 {
            ss.tick();
        }
        play_music(&mut ss, &mus);
        assert_eq!(ss.model().sound(2).unwrap().frame, 44100);

        // OracleCompat: IsSoundPlaying is -1, which is false, so the game "starts" the music every time,
        // into a stub that does nothing.
        let (mut ss, _, mus) = booted(Mode::OracleCompat);
        let playing = on(&mut ss, Script::IsSoundPlaying, &mus);
        assert_eq!(playing, r(-1.0));
        assert!(!playing.is_truthy());
        play_music(&mut ss, &mus);
        assert!(ss.take_commands().is_empty());
    }

    #[test]
    fn scr_music_fade_lowers_the_frequency() {
        let (mut ss, _, mus) = booted(Mode::Faithful);
        play_music(&mut ss, &mus);
        ss.call(Script::SetSoundFreq, &[mus.clone(), r(44100.0)])
            .unwrap();
        for _ in 0..100 {
            let f = on(&mut ss, Script::GetSoundFreq, &mus);
            ss.call(Script::SetSoundFreq, &[mus.clone(), r(f.to_real() - 100.0)])
                .unwrap();
            ss.tick();
        }
        assert_eq!(ss.call(Script::GetSoundFreq, &[mus]), Ok(r(34100.0)));
    }

    #[test]
    fn clones_stay_in_step() {
        let (mut a, snd, mus) = booted(Mode::Faithful);
        play_music(&mut a, &mus);
        a.tick();
        let mut b = a.clone();
        for ss in [&mut a, &mut b] {
            for i in 0..90 {
                if i % 7 == 0 {
                    on(ss, Script::PlaySound, &snd);
                }
                if i == 40 {
                    on(ss, Script::PauseSound, &mus);
                }
                if i == 50 {
                    on(ss, Script::ResumeSound, &mus);
                }
                ss.tick();
            }
        }
        assert_eq!(a, b);
        assert_eq!(a.take_commands(), b.take_commands());
        assert_eq!(
            on(&mut a, Script::GetSoundPosition, &mus),
            on(&mut b, Script::GetSoundPosition, &mus)
        );
    }

    #[test]
    fn val_parses_like_purebasic() {
        assert_eq!(pb_val(b"44100"), 44100);
        assert_eq!(pb_val(b"-1333.33"), -1333);
        assert_eq!(pb_val(b"+7x"), 7);
        assert_eq!(pb_val(b"No"), 0);
        assert_eq!(pb_val(b""), 0);
        assert_eq!(pb_val(b"4294967297"), 1);
    }
}
