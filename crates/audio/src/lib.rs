//! supersound.dll for the port: all of Spelunky 1.1's sound effects and music.
//!
//! The game plays every sound through `supersound.dll` ("Super Sound System" by tsg1zzn), via 23 `SS_*`
//! GML scripts (`docs/source-inventory.md` §7.1). The port replaces those scripts with overrides that
//! call this crate. It has two parts:
//!
//! - **A deterministic model** ([`ss::SuperSound`] over [`model::AudioModel`]): headless, `Clone`,
//!   wasm-compatible. It tracks every handle (loaded, playing, paused, looping, volume, pan, frequency,
//!   position) and advances positions by game frames, 30 per second, from each file's length and sample
//!   rate ([`format`]). **Every answer the game can see comes from the model**, never from a device, so
//!   headless runs, replays and windowed runs behave identically. Each frame it also emits a list of
//!   [`model::Command`]s for a player.
//! - **A playback backend** ([`backend::Player`], feature `backend`): applies the commands to a real
//!   output device with [kira](https://docs.rs/kira), on macOS (and Windows and Linux) and in the browser.
//!
//! ```text
//! game override SS_PlaySound(h) ─► SuperSound::call ─► AudioModel ─► answers (to GML)
//!                                                          └──────► commands ─► backend::Player ─► device
//! ```
//!
//! An override returns [`Outcome::value`](ss::Outcome) to GML and sets every global in
//! [`Outcome::globals`](ss::Outcome): `SS_Init`'s 22 `global.dll_SS_*` ids. The model and the player
//! share one [`SoundBank`], which holds the files, so they can't disagree about which exist.
//!
//! # Modes
//!
//! The reference oracle (OpenGMK) can't load the DLL, so it stubs it: `SS_Init` returns `"Yes"` and every
//! other export the real 0 (`tools/oracle/README.md`). Then `SS_LoadSound` returns 0, so the game's sound
//! handles are reals, and every wrapper script returns early on its `if is_real(argument0)` line. The
//! oracle's game never plays a sound, and its queries answer differently from the real game's:
//!
//! | Script | Real DLL ([`Mode::Faithful`](ss::Mode::Faithful)) | Oracle ([`Mode::OracleCompat`](ss::Mode::OracleCompat)) |
//! |---|---|---|
//! | `SS_LoadSound` | handle string, `"No"` if missing | `0` |
//! | `SS_IsSoundPlaying`, `SS_IsSoundLooping` | `1` / `0` (`-1` bad handle) | `-1` |
//! | `SS_IsSoundPaused` | `1` / `0` | `-1` |
//! | `SS_GetSoundFreq` | Hz (`-1` bad handle) | `-1` |
//! | `SS_GetSoundVol` | `0..10000` | `"-1"` (a string) |
//! | `SS_GetSoundPan` | `-10000..10000` | `"0"` (a string) |
//! | `SS_GetSoundPosition` | bytes | `-1` |
//! | `SS_GetSoundLength`, `SS_GetSoundBytesPerSecond` | bytes, bytes/s | `0` |
//! | `SS_SetSoundPosition` | `2` (`1` bad handle) | `0` |
//! | `SS_PauseSound`, `SS_ResumeSound` | `1` / `0` | `-1` |
//! | `SS_Play/Loop/Stop/FreeSound`, `SS_Set*` | `"0"`, `"1"` | `"0"` |
//!
//! [`ss::Mode::Faithful`] (the default) models the real DLL, for play. [`ss::Mode::OracleCompat`]
//! reproduces the stub exactly, for runs diffed against the oracle: it's silent and emits no commands.
//!
//! ## What depends on the answers
//!
//! Every branch on an audio query only decides whether to make another audio call, so **no instance or
//! game variable depends on the mode**. The only game-visible difference is the 62 handle globals
//! (`global.snd*`, `global.mus*`), which are 0 in the oracle and handle strings in the real game. The
//! branches, for the record:
//!
//! - `playMusic(mus, loop)`: `if (global.music and not SS_IsSoundPlaying(mus))` loop or play it. Faithful
//!   leaves a track that's already playing alone; the oracle "starts" it every time. Called from
//!   `startMusic` (`oLevel` create, the sun/moon/stars rooms), `oGame` (M toggles music), `oTitle`,
//!   `oOlmec`, `oEndCustom`, `oPDummy2`, `oCamel` and `oSlider1`.
//! - One-at-a-time effects, `if (not SS_IsSoundPlaying(snd)) playSound(snd)`: `oOlmec` and `oLavaSpray`
//!   (`sndFlame`), `oYetiKing` (`sndYetiYell`), `moveTo` (`sndPush`, pushing blocks).
//! - The bow: `oPlayer1` step and `scrFireBow` stop `sndBowPull` only if it's playing.
//! - Pause: `oScreen` (begin step, F1) mutes each playing track on pause and restores its volume on
//!   unpause or death, `if (SS_IsSoundPlaying(mus)) SS_SetSoundVol(...)`.
//! - `scrMusicFade` (`oLevel` step, 100 frames once `oGame` sets `musicFade`): sets the level's track to
//!   `SS_GetSoundFreq(mus) - 100`, slowing 44100 Hz to 34100 Hz. In the oracle the query returns -1 and
//!   the set is skipped.
//! - The SuperSound demo object `core` uses every query, but nothing creates it.
//!
//! # supersound.dll's semantics
//!
//! Read from the DLL's disassembly (it's PureBasic: its `Sound` library over DirectSound, plus libvorbis)
//! and the GML wrappers. "Verify" marks what's inferred rather than observed; [`model`] has the details.
//!
//! Confirmed:
//!
//! - **Units are DirectSound's.** Volume is attenuation in hundredths of a dB, -10000..0; the wrapper
//!   maps the game's 0..10000 onto it by subtracting 10000 (so music at `musicVol` 15, 8667, is
//!   -13.3 dB). Pan is -10000 (left) to 10000 (right), attenuating the other channel. Frequency is the
//!   playback rate in Hz (speed and pitch). Every file is 44100 Hz but `letsexplore.wav` (unused).
//! - **Arguments** reach the DLL as GML `string()`s and are read with PureBasic's `Val()` (integer part),
//!   then passed to DirectSound unchecked; DirectSound rejects out-of-range values and the DLL reports
//!   success anyway.
//! - **Play and loop restart** a sound from the start; **stop rewinds** it. A sound is one DirectSound
//!   buffer: playing it again cuts it off.
//! - **Playing** means DirectSound reports `DSBSTATUS_PLAYING`. **Looping** means it plays with
//!   `DSBSTATUS_LOOPING`, which a playing stream always does.
//! - **Pause** remembers the position and looping flag in a list, then stops the sound; **resume**
//!   plays it again and moves it back, and does nothing for a sound that isn't paused. Play and stop
//!   forget a pause. A paused stream resumes looping, from the start of the file with its ring cursor
//!   moved. (Only `core` pauses.)
//! - **Positions and lengths are bytes** of the PCM: a static sound's buffer offset and size, or for a
//!   streamed sound (the music) the cursor in and size of a 64 KiB ring buffer. Bytes per second is the
//!   format's, ignoring the frequency.
//! - **`SS_Unload` calls `SS_Init`** (copy-paste bug in `SS_Init.gml`), which makes PureBasic start a new
//!   sound table: every handle stops working.
//! - `SS_LoadSound` returns `"No"`, not `"0"` as its wrapper says, for a missing file.
//!
//! Assumed (verify against the real game):
//!
//! - Time: the real DLL plays in wall-clock time, the model in game frames at exactly 30 per second.
//!   They agree as long as the game holds 30 fps.
//! - DirectSound's limits (frequency up to 200000 Hz), that a non-looping buffer reads position 0 after
//!   it ends, and that a position is rounded down to a sample frame.
//! - A non-looping stream stops when its ring cursor reaches the half-buffer boundary after the end of
//!   the file (read from the refill thread; the real thread polls every 10 ms, so it's a little later).
//! - Handles: the DLL returns PureBasic object addresses, the model numbers from 1. Only stored by the
//!   game, never inspected.
//!
//! # Example
//!
//! `scripts/cargo run -p audio --example play --features backend` loads a sound effect and two music tracks,
//! plays the effect and cross-fades between the tracks through the GML scripts, as the game would.

pub mod format;
pub mod model;
pub mod ss;

#[cfg(feature = "backend")]
pub mod backend;

pub use format::{FormatError, SoundInfo};
pub use model::{AudioModel, Command, Handle, SoundBank, SoundState, TICKS_PER_SECOND};
pub use ss::{EXTERNAL_GLOBALS, Mode, Outcome, Script, SuperSound};
