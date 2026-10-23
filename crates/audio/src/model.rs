//! supersound.dll's state, simulated: every handle the game loaded, what it's doing, and where it is.
//!
//! The methods mirror the DLL's exports (`SS_PlaySound` is [`AudioModel::play_sound`], and so on) after
//! their string arguments have been parsed, and return what the export would report. [`crate::ss`] adds
//! the string conversions and the GML wrapper scripts on top. What each export does was read from the
//! DLL's disassembly; see the crate docs for the details and what's still unconfirmed.
//!
//! Time only moves in [`AudioModel::tick`]: once per game frame, 1/30 s. Positions are kept in 1/30ths
//! of a sample frame, so a tick at frequency `f` advances exactly `f` units and nothing is ever rounded.

use crate::format::{FormatError, SoundInfo};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Game frames per second (every Spelunky room runs at `room_speed` 30). The model's clock.
pub const TICKS_PER_SECOND: u64 = 30;

/// Size of a streamed sound's DirectSound ring buffer. The DLL refills one half while the other plays,
/// and reports the ring's cursor and size as the sound's position and length. Verify: read from the
/// disassembly (`0x10000` stored as the stream's buffer size), not observed.
pub const STREAM_BUFFER_BYTES: u32 = 0x10000;

const STREAM_HALF_BYTES: u64 = STREAM_BUFFER_BYTES as u64 / 2;

/// DirectSound's limits (`dsound.h`). Out-of-range values are rejected with `DSERR_INVALIDPARAM` and
/// leave the buffer unchanged; the DLL ignores the error and reports success.
pub mod limits {
    /// Volume is attenuation in hundredths of a decibel: `DSBVOLUME_MIN..=DSBVOLUME_MAX`.
    pub const VOLUME_MIN: i32 = -10_000;
    pub const VOLUME_MAX: i32 = 0;
    /// Pan attenuates the other channel, in hundredths of a decibel: `DSBPAN_LEFT..=DSBPAN_RIGHT`.
    pub const PAN_LEFT: i32 = -10_000;
    pub const PAN_RIGHT: i32 = 10_000;
    /// Playback frequency in Hz: `DSBFREQUENCY_MIN..=DSBFREQUENCY_MAX`, or 0 (`DSBFREQUENCY_ORIGINAL`) for
    /// the file's own rate. Verify the maximum: DirectX 7 headers say 100000, DirectX 8 and later 200000.
    pub const FREQUENCY_MIN: i32 = 100;
    pub const FREQUENCY_MAX: i32 = 200_000;
}

/// A sound handle as the DLL parses it: the PureBasic sound number its `SS_LoadSound` returned. The DLL
/// returns heap addresses; the model numbers sounds from 1. The game only stores and passes them back.
pub type Handle = i32;

/// Sound files by the path the game loads them with (`sound/jump.wav`), relative to the game directory.
/// Lookups ignore ASCII case and accept `\` for `/`, as Windows does.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SoundBank {
    files: BTreeMap<String, SoundInfo>,
}

impl SoundBank {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a file from its bytes.
    pub fn insert(&mut self, path: &str, bytes: &[u8]) -> Result<(), FormatError> {
        self.insert_info(path, SoundInfo::parse(bytes)?);
        Ok(())
    }

    pub fn insert_info(&mut self, path: &str, info: SoundInfo) {
        self.files.insert(normalize_path(path), info);
    }

    pub fn get(&self, path: &str) -> Option<SoundInfo> {
        self.files.get(&normalize_path(path)).copied()
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

/// The key a path is stored under: ASCII lowercase, `/` separators, no leading `./`.
pub fn normalize_path(path: &str) -> String {
    let mut p = path.replace('\\', "/").to_ascii_lowercase();
    while let Some(rest) = p.strip_prefix("./") {
        p = rest.to_owned();
    }
    p
}

/// What a player has to do to sound like the model. Each tick's commands come in the order the game
/// made the calls; [`AudioModel::take_commands`] drains them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// `handle` now names this file (a key of the [`SoundBank`], normalized). `stream` is the DLL's
    /// streaming flag, which the game sets for its music.
    Load {
        handle: Handle,
        path: Arc<str>,
        stream: bool,
    },
    /// Stop and forget `handle`.
    Free { handle: Handle },
    /// (Re)start `handle` from sample frame `frame` of its file, once or looping, with its current volume,
    /// pan and frequency.
    Play {
        handle: Handle,
        looping: bool,
        frame: u64,
    },
    /// Stop `handle`, whether the game asked or it reached its end.
    Stop { handle: Handle },
    /// Jump a playing sound to sample frame `frame` of its file.
    Seek { handle: Handle, frame: u64 },
    /// DirectSound volume in hundredths of a decibel, `-10000..=0`.
    Volume { handle: Handle, millibels: i32 },
    /// DirectSound pan, `-10000..=10000`: how many hundredths of a decibel the other channel loses.
    Pan { handle: Handle, pan: i32 },
    /// Playback frequency in Hz: speed and pitch, relative to the file's own rate.
    Frequency { handle: Handle, hz: u32 },
    /// Stop everything and forget every handle (the DLL was initialized again, or a resync follows).
    Reset,
}

/// A snapshot of one sound, for players, tools and tests.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SoundState {
    pub path: Arc<str>,
    pub info: SoundInfo,
    pub stream: bool,
    /// `Some(looping)` while playing.
    pub playing: Option<bool>,
    /// Sample frames played since the start of the file (or, for a stream, since it was started).
    pub frame: u64,
    pub volume: i32,
    pub pan: i32,
    pub frequency: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Sound {
    path: Arc<str>,
    info: SoundInfo,
    stream: bool,
    /// `Some(looping)` while the buffer plays. `looping` is the flag the DLL passed to PureBasic's
    /// `PlaySound`; a stream's DirectSound buffer always loops (it's a ring), whatever the flag.
    playing: Option<bool>,
    /// In 1/30ths of a sample frame. A static sound's offset into its buffer; a stream's total since it
    /// started, of which the ring cursor is the remainder.
    pos: u64,
    volume: i32,
    pan: i32,
    frequency: u32,
}

impl Sound {
    fn frame(&self) -> u64 {
        self.pos / TICKS_PER_SECOND
    }

    fn frame_bytes(&self) -> u64 {
        u64::from(self.info.frame_bytes)
    }

    /// The DirectSound buffer's size in bytes.
    fn buffer_bytes(&self) -> u64 {
        if self.stream {
            u64::from(STREAM_BUFFER_BYTES)
        } else {
            self.info.bytes()
        }
    }

    /// DirectSound's play cursor in bytes.
    fn cursor(&self) -> u64 {
        let bytes = self.frame() * self.frame_bytes();
        if self.stream {
            bytes % u64::from(STREAM_BUFFER_BYTES)
        } else {
            bytes
        }
    }

    /// The sample frame of the file that's playing, for players.
    fn file_frame(&self) -> u64 {
        let frame = self.frame();
        match (self.stream, self.playing) {
            (true, Some(true)) if self.info.frames > 0 => frame % self.info.frames,
            (true, _) => frame.min(self.info.frames),
            (false, _) => frame,
        }
    }

    /// Where playback ends, in model units, or `None` if it never does.
    ///
    /// A static sound ends with its buffer. A stream that doesn't loop is stopped by the DLL's refill
    /// thread when the cursor enters the first half it filled with no data at all, so the end of the file
    /// plays out, padded with silence to the next half-buffer boundary: `ceil(bytes / half) * half`.
    /// Verify: read from the refill thread's disassembly, and the real thread polls every 10 ms, so it
    /// stops a little later than this.
    fn end(&self) -> Option<u64> {
        match self.playing {
            None | Some(true) => None,
            Some(false) if self.stream => {
                let bytes = self.info.bytes().div_ceil(STREAM_HALF_BYTES) * STREAM_HALF_BYTES;
                Some(bytes / self.frame_bytes() * TICKS_PER_SECOND)
            }
            Some(false) => Some(self.info.frames * TICKS_PER_SECOND),
        }
    }

    fn state(&self) -> SoundState {
        SoundState {
            path: self.path.clone(),
            info: self.info,
            stream: self.stream,
            playing: self.playing,
            frame: self.frame(),
            volume: self.volume,
            pan: self.pan,
            frequency: self.frequency,
        }
    }
}

/// An entry of the DLL's list of paused sounds. `SS_PauseSound` stops the sound and remembers where it
/// was; `SS_ResumeSound` plays it again and moves it back there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Paused {
    handle: Handle,
    /// What `SS_GetSoundPosition` said, in bytes.
    position: i32,
    /// What `SS_IsSoundLooping` said: 1 or 0.
    looping: i32,
}

/// supersound.dll's state. A plain value: `Clone` is a savestate, and two equal models given the same
/// calls give the same answers and commands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioModel {
    bank: Arc<SoundBank>,
    initialized: bool,
    sounds: BTreeMap<Handle, Sound>,
    next_handle: Handle,
    paused: Vec<Paused>,
    commands: Vec<Command>,
}

impl AudioModel {
    pub fn new(bank: Arc<SoundBank>) -> Self {
        Self {
            bank,
            initialized: false,
            sounds: BTreeMap::new(),
            next_handle: 1,
            paused: Vec::new(),
            commands: Vec::new(),
        }
    }

    pub fn bank(&self) -> &Arc<SoundBank> {
        &self.bank
    }

    /// Advances every playing sound by one game frame. Call once per frame, after the frame's events.
    pub fn tick(&mut self) {
        for (&handle, s) in &mut self.sounds {
            let Some(looping) = s.playing else { continue };
            s.pos += u64::from(s.frequency);
            if let Some(end) = s.end() {
                if s.pos >= end {
                    s.playing = None;
                    // A static buffer that played to its end reads as position 0 (verify); a stream's ring
                    // cursor stays where the refill thread stopped it.
                    s.pos = if s.stream { end } else { 0 };
                    self.commands.push(Command::Stop { handle });
                }
            } else if looping && !s.stream {
                let len = s.info.frames * TICKS_PER_SECOND;
                if len == 0 {
                    s.pos = 0;
                } else {
                    s.pos %= len;
                }
            }
        }
    }

    /// The commands since the last call, oldest first.
    pub fn take_commands(&mut self) -> Vec<Command> {
        std::mem::take(&mut self.commands)
    }

    /// Commands that bring a fresh player (or one that missed commands, e.g. after loading a savestate)
    /// to the model's current state.
    pub fn resync(&self) -> Vec<Command> {
        let mut out = vec![Command::Reset];
        for (&handle, s) in &self.sounds {
            out.push(Command::Load {
                handle,
                path: s.path.clone(),
                stream: s.stream,
            });
            out.push(Command::Volume {
                handle,
                millibels: s.volume,
            });
            out.push(Command::Pan { handle, pan: s.pan });
            out.push(Command::Frequency {
                handle,
                hz: s.frequency,
            });
            if let Some(looping) = s.playing {
                out.push(Command::Play {
                    handle,
                    looping,
                    frame: s.file_frame(),
                });
            }
        }
        out
    }

    /// Every loaded sound, by handle.
    pub fn sounds(&self) -> impl Iterator<Item = (Handle, SoundState)> + '_ {
        self.sounds.iter().map(|(&h, s)| (h, s.state()))
    }

    pub fn sound(&self, handle: Handle) -> Option<SoundState> {
        self.sounds.get(&handle).map(Sound::state)
    }

    // ---- Exports ------------------------------------------------------------------------------------

    /// `SS_Init`: PureBasic's `InitSound`. Always succeeds here (the DLL says `"Yes"`).
    ///
    /// `InitSound` makes a new, empty sound table, so a second call invalidates every handle. The game
    /// does that at the very end: `SS_Unload` is bound to the `SS_Init` symbol (a copy-paste bug).
    pub fn init(&mut self) -> bool {
        if self.initialized {
            self.sounds.clear();
            self.commands.push(Command::Reset);
        }
        self.initialized = true;
        true
    }

    /// `SS_LoadSound(file, flags)`: `None` if the file isn't in the bank (the DLL says `"No"`). Bit 0 of
    /// `flags` streams the file. Loading before `SS_Init` fails (the DLL would crash).
    pub fn load_sound(&mut self, path: &str, flags: i32) -> Option<Handle> {
        if !self.initialized {
            return None;
        }
        let info = self.bank.get(path)?;
        let handle = self.next_handle;
        self.next_handle += 1;
        let path: Arc<str> = normalize_path(path).into();
        let stream = flags & 1 != 0;
        self.sounds.insert(
            handle,
            Sound {
                path: path.clone(),
                info,
                stream,
                playing: None,
                pos: 0,
                volume: limits::VOLUME_MAX,
                pan: 0,
                frequency: info.sample_rate,
            },
        );
        self.commands.push(Command::Load {
            handle,
            path,
            stream,
        });
        Some(handle)
    }

    /// `SS_FreeSound`. The GML wrapper stops the sound first.
    pub fn free_sound(&mut self, handle: Handle) {
        if self.sounds.remove(&handle).is_some() {
            self.commands.push(Command::Free { handle });
        }
    }

    /// `SS_PlaySound`: forgets a pause, then plays once from the start (restarting if it was playing).
    pub fn play_sound(&mut self, handle: Handle) {
        self.forget_pause(handle);
        self.pb_play(handle, false);
    }

    /// `SS_LoopSound`: forgets a pause, then loops from the start (restarting if it was playing).
    pub fn loop_sound(&mut self, handle: Handle) {
        self.forget_pause(handle);
        self.pb_play(handle, true);
    }

    /// `SS_StopSound`: forgets a pause, stops, and rewinds.
    pub fn stop_sound(&mut self, handle: Handle) {
        self.forget_pause(handle);
        self.pb_stop(handle);
    }

    /// `SS_PauseSound`: remembers the position and whether it loops, then stops (and rewinds) the sound.
    /// False for an unknown or already paused handle. Pausing a stopped sound works too.
    pub fn pause_sound(&mut self, handle: Handle) -> bool {
        if !self.sounds.contains_key(&handle) || self.is_sound_paused(handle) {
            return false;
        }
        let position = self.sound_position(handle).unwrap_or(-1) as i32;
        let looping = i32::from(self.is_sound_looping(handle) == Some(true));
        self.paused.push(Paused {
            handle,
            position,
            looping,
        });
        self.pb_stop(handle);
        true
    }

    /// `SS_ResumeSound`: plays a paused sound from the start, looping if it looped, then moves it to where
    /// it was paused. False if it isn't paused: unlike its GML wrapper's comment says, a stopped sound
    /// doesn't start.
    ///
    /// A stream "resumes" from the start of the file, with its ring cursor moved: the DLL's own quirk.
    pub fn resume_sound(&mut self, handle: Handle) -> bool {
        let Some(i) = self.paused.iter().position(|p| p.handle == handle) else {
            return false;
        };
        let p = self.paused[i];
        self.pb_play(handle, p.looping != 0);
        self.set_sound_position(handle, p.position);
        self.paused.remove(i);
        true
    }

    /// `SS_SetSoundFreq`: false for an unknown handle. 0 restores the file's rate; other values outside
    /// DirectSound's range are ignored.
    pub fn set_sound_freq(&mut self, handle: Handle, hz: i32) -> bool {
        let Some(s) = self.sounds.get_mut(&handle) else {
            return false;
        };
        let hz = match hz {
            0 => s.info.sample_rate,
            limits::FREQUENCY_MIN..=limits::FREQUENCY_MAX => hz as u32,
            _ => return true,
        };
        s.frequency = hz;
        self.commands.push(Command::Frequency { handle, hz });
        true
    }

    /// `SS_SetSoundPan`: false for an unknown handle; out-of-range values are ignored.
    pub fn set_sound_pan(&mut self, handle: Handle, pan: i32) -> bool {
        let Some(s) = self.sounds.get_mut(&handle) else {
            return false;
        };
        if (limits::PAN_LEFT..=limits::PAN_RIGHT).contains(&pan) {
            s.pan = pan;
            self.commands.push(Command::Pan { handle, pan });
        }
        true
    }

    /// `SS_SetSoundVol`, in DirectSound's units (the GML wrapper subtracts 10000): false for an unknown
    /// handle; out-of-range values are ignored.
    pub fn set_sound_vol(&mut self, handle: Handle, millibels: i32) -> bool {
        let Some(s) = self.sounds.get_mut(&handle) else {
            return false;
        };
        if (limits::VOLUME_MIN..=limits::VOLUME_MAX).contains(&millibels) {
            s.volume = millibels;
            self.commands.push(Command::Volume { handle, millibels });
        }
        true
    }

    /// `SS_SetSoundPosition`, in bytes: false for an unknown handle. Positions outside the buffer are
    /// ignored; a stream's position is a cursor into its ring buffer. Verify that DirectSound rounds a
    /// position inside a sample frame down.
    pub fn set_sound_position(&mut self, handle: Handle, bytes: i32) -> bool {
        let Some(s) = self.sounds.get_mut(&handle) else {
            return false;
        };
        // DirectSound takes a DWORD, so a negative position is a huge one.
        let p = u64::from(bytes as u32);
        if p < s.buffer_bytes() {
            let fb = s.frame_bytes();
            let target = if s.stream {
                let played = s.frame() * fb;
                played - played % u64::from(STREAM_BUFFER_BYTES) + p
            } else {
                p
            };
            s.pos = target / fb * TICKS_PER_SECOND;
            if s.playing.is_some() {
                let frame = s.file_frame();
                self.commands.push(Command::Seek { handle, frame });
            }
        }
        true
    }

    /// `SS_GetSoundFreq` in Hz.
    pub fn sound_freq(&self, handle: Handle) -> Option<u32> {
        self.sounds.get(&handle).map(|s| s.frequency)
    }

    /// `SS_GetSoundPan`.
    pub fn sound_pan(&self, handle: Handle) -> Option<i32> {
        self.sounds.get(&handle).map(|s| s.pan)
    }

    /// `SS_GetSoundVol`, in DirectSound's units (the GML wrapper adds 10000).
    pub fn sound_vol(&self, handle: Handle) -> Option<i32> {
        self.sounds.get(&handle).map(|s| s.volume)
    }

    /// `SS_IsSoundPlaying`. A paused sound is stopped, so it isn't playing.
    pub fn is_sound_playing(&self, handle: Handle) -> Option<bool> {
        self.sounds.get(&handle).map(|s| s.playing.is_some())
    }

    /// `SS_IsSoundLooping`: whether DirectSound plays the buffer looped. A playing stream always does.
    pub fn is_sound_looping(&self, handle: Handle) -> Option<bool> {
        self.sounds
            .get(&handle)
            .map(|s| s.playing.is_some_and(|looping| looping || s.stream))
    }

    /// `SS_IsSoundPaused`. Doesn't check the handle.
    pub fn is_sound_paused(&self, handle: Handle) -> bool {
        self.paused.iter().any(|p| p.handle == handle)
    }

    /// `SS_GetSoundPosition` in bytes: where it was paused if it's paused (checked before the handle),
    /// else the play cursor.
    pub fn sound_position(&self, handle: Handle) -> Option<i64> {
        if let Some(p) = self.paused.iter().find(|p| p.handle == handle) {
            return Some(i64::from(p.position));
        }
        self.sounds.get(&handle).map(|s| s.cursor() as i64)
    }

    /// `SS_GetSoundLength`: the buffer's size in bytes ([`STREAM_BUFFER_BYTES`] for a stream).
    pub fn sound_length(&self, handle: Handle) -> Option<i64> {
        self.sounds.get(&handle).map(|s| s.buffer_bytes() as i64)
    }

    /// `SS_GetSoundBytesPerSecond`: the buffer format's `nAvgBytesPerSec`, which ignores the frequency.
    pub fn sound_bytes_per_second(&self, handle: Handle) -> Option<u32> {
        self.sounds.get(&handle).map(|s| s.info.bytes_per_second())
    }

    // ---- PureBasic's sound library ------------------------------------------------------------------

    fn forget_pause(&mut self, handle: Handle) {
        self.paused.retain(|p| p.handle != handle);
    }

    /// `PlaySound(#Sound, flags)`: stop, rewind, play.
    fn pb_play(&mut self, handle: Handle, looping: bool) {
        if let Some(s) = self.sounds.get_mut(&handle) {
            s.playing = Some(looping);
            s.pos = 0;
            self.commands.push(Command::Play {
                handle,
                looping,
                frame: 0,
            });
        }
    }

    /// `StopSound(#Sound)`: stop and rewind.
    fn pb_stop(&mut self, handle: Handle) {
        if let Some(s) = self.sounds.get_mut(&handle) {
            s.pos = 0;
            if s.playing.take().is_some() {
                self.commands.push(Command::Stop { handle });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One second of 16-bit mono at 44100 Hz (88200 bytes), and a stereo stream of 100000 frames.
    fn model() -> AudioModel {
        let mut bank = SoundBank::new();
        let one_second = SoundInfo {
            sample_rate: 44100,
            channels: 1,
            frame_bytes: 2,
            frames: 44100,
        };
        bank.insert_info("sound/beep.wav", one_second);
        bank.insert_info(
            "sound/song.ogg",
            SoundInfo {
                sample_rate: 44100,
                channels: 2,
                frame_bytes: 4,
                frames: 100_000,
            },
        );
        let mut m = AudioModel::new(Arc::new(bank));
        assert!(m.init());
        m
    }

    fn ticks(m: &mut AudioModel, n: u32) {
        for _ in 0..n {
            m.tick();
        }
    }

    #[test]
    fn loads_by_case_insensitive_path() {
        let mut m = model();
        assert_eq!(m.load_sound("SOUND\\Beep.WAV", 0), Some(1));
        assert_eq!(m.load_sound("sound/missing.wav", 0), None);
        assert_eq!(m.load_sound("./sound/song.ogg", 1), Some(2));
        assert_eq!(m.sound(1).unwrap().path.as_ref(), "sound/beep.wav");
    }

    #[test]
    fn load_before_init_fails() {
        let mut m = AudioModel::new(model().bank().clone());
        assert_eq!(m.load_sound("sound/beep.wav", 0), None);
    }

    #[test]
    fn position_advances_with_ticks_and_frequency() {
        let mut m = model();
        let h = m.load_sound("sound/beep.wav", 0).unwrap();
        m.play_sound(h);
        assert_eq!(m.is_sound_playing(h), Some(true));
        assert_eq!(m.sound_position(h), Some(0));
        ticks(&mut m, 15);
        // 15/30 s of 44100 frames of 2 bytes.
        assert_eq!(m.sound_position(h), Some(44100));
        m.set_sound_freq(h, 22050);
        ticks(&mut m, 2);
        assert_eq!(m.sound_position(h), Some(44100 + 2 * 735 * 2));
        // An odd frequency carries the fraction over: 3 ticks at 1001 Hz are 100.1 frames.
        m.set_sound_freq(h, 1001);
        let before = m.sound_position(h).unwrap();
        ticks(&mut m, 3);
        assert_eq!(m.sound_position(h), Some(before + 100 * 2));
        ticks(&mut m, 27);
        assert_eq!(m.sound_position(h), Some(before + 1001 * 2));
    }

    #[test]
    fn a_static_sound_stops_at_its_end_and_rewinds() {
        let mut m = model();
        let h = m.load_sound("sound/beep.wav", 0).unwrap();
        m.play_sound(h);
        m.take_commands();
        ticks(&mut m, 29);
        assert_eq!(m.is_sound_playing(h), Some(true));
        m.tick();
        assert_eq!(m.is_sound_playing(h), Some(false));
        assert_eq!(m.sound_position(h), Some(0));
        assert_eq!(m.take_commands(), vec![Command::Stop { handle: h }]);
    }

    #[test]
    fn a_looping_sound_wraps() {
        let mut m = model();
        let h = m.load_sound("sound/beep.wav", 0).unwrap();
        m.loop_sound(h);
        assert_eq!(m.is_sound_looping(h), Some(true));
        ticks(&mut m, 45);
        assert_eq!(m.is_sound_playing(h), Some(true));
        assert_eq!(m.sound_position(h), Some(44100));
        // Playing it again restarts it once.
        m.play_sound(h);
        assert_eq!(m.sound_position(h), Some(0));
        assert_eq!(m.is_sound_looping(h), Some(false));
    }

    #[test]
    fn pause_and_resume() {
        let mut m = model();
        let h = m.load_sound("sound/beep.wav", 0).unwrap();
        m.loop_sound(h);
        ticks(&mut m, 10);
        assert!(m.pause_sound(h));
        assert!(!m.pause_sound(h), "already paused");
        assert!(m.is_sound_paused(h));
        assert_eq!(m.is_sound_playing(h), Some(false));
        assert_eq!(m.sound_position(h), Some(29400));
        ticks(&mut m, 10);
        assert_eq!(m.sound_position(h), Some(29400));
        assert!(m.resume_sound(h));
        assert!(!m.is_sound_paused(h));
        assert_eq!(m.is_sound_looping(h), Some(true));
        assert_eq!(m.sound_position(h), Some(29400));
        ticks(&mut m, 10);
        assert_eq!(m.sound_position(h), Some(58800));
        // Resuming something that isn't paused does nothing.
        m.stop_sound(h);
        assert!(!m.resume_sound(h));
        assert_eq!(m.is_sound_playing(h), Some(false));
    }

    #[test]
    fn play_and_stop_forget_a_pause() {
        let mut m = model();
        let h = m.load_sound("sound/beep.wav", 0).unwrap();
        m.play_sound(h);
        ticks(&mut m, 3);
        m.pause_sound(h);
        m.stop_sound(h);
        assert!(!m.is_sound_paused(h));
        assert_eq!(m.sound_position(h), Some(0));
        m.pause_sound(h);
        m.play_sound(h);
        assert!(!m.is_sound_paused(h));
        assert_eq!(m.is_sound_playing(h), Some(true));
    }

    #[test]
    fn stop_rewinds() {
        let mut m = model();
        let h = m.load_sound("sound/beep.wav", 0).unwrap();
        m.loop_sound(h);
        ticks(&mut m, 5);
        m.stop_sound(h);
        assert_eq!(m.is_sound_playing(h), Some(false));
        assert_eq!(m.is_sound_looping(h), Some(false));
        assert_eq!(m.sound_position(h), Some(0));
        ticks(&mut m, 5);
        assert_eq!(m.sound_position(h), Some(0));
    }

    #[test]
    fn streams_report_their_ring_buffer() {
        let mut m = model();
        let h = m.load_sound("sound/song.ogg", 1).unwrap();
        assert_eq!(m.sound_length(h), Some(65536));
        assert_eq!(m.sound_bytes_per_second(h), Some(176400));
        m.play_sound(h);
        // Played once, but the ring buffer loops.
        assert_eq!(m.is_sound_looping(h), Some(true));
        ticks(&mut m, 1);
        assert_eq!(m.sound_position(h), Some(1470 * 4));
        ticks(&mut m, 11);
        // 17640 frames of 4 bytes is 70560 bytes, past the ring's end.
        assert_eq!(m.sound_position(h), Some(70560 - 65536));
    }

    #[test]
    fn a_stream_stops_at_the_half_buffer_after_its_end() {
        let mut m = model();
        let h = m.load_sound("sound/song.ogg", 1).unwrap();
        m.play_sound(h);
        // 400000 bytes round up to 13 halves of 32768: 425984 bytes, 106496 frames, 72.4 ticks.
        ticks(&mut m, 72);
        assert_eq!(m.is_sound_playing(h), Some(true));
        assert_eq!(m.sound(h).unwrap().frame, 105840);
        m.tick();
        assert_eq!(m.is_sound_playing(h), Some(false));
        // Looping, it never stops.
        m.loop_sound(h);
        ticks(&mut m, 1000);
        assert_eq!(m.is_sound_playing(h), Some(true));
    }

    #[test]
    fn setters_ignore_values_directsound_rejects() {
        let mut m = model();
        let h = m.load_sound("sound/beep.wav", 0).unwrap();
        assert!(m.set_sound_vol(h, -1333));
        assert!(m.set_sound_vol(h, 1));
        assert_eq!(m.sound_vol(h), Some(-1333));
        assert!(m.set_sound_pan(h, -10_001));
        assert!(m.set_sound_pan(h, 10_000));
        assert_eq!(m.sound_pan(h), Some(10_000));
        assert!(m.set_sound_freq(h, 99));
        assert_eq!(m.sound_freq(h), Some(44100));
        assert!(m.set_sound_freq(h, 34100));
        assert_eq!(m.sound_freq(h), Some(34100));
        assert!(m.set_sound_freq(h, 0));
        assert_eq!(m.sound_freq(h), Some(44100));
        assert!(m.set_sound_position(h, 88200), "returns success...");
        assert_eq!(m.sound_position(h), Some(0), "...but ignores the end");
        assert!(m.set_sound_position(h, 1001));
        assert_eq!(m.sound_position(h), Some(1000));
    }

    #[test]
    fn unknown_handles() {
        let mut m = model();
        assert_eq!(m.is_sound_playing(7), None);
        assert_eq!(m.sound_position(7), None);
        assert!(!m.set_sound_vol(7, 0));
        assert!(!m.pause_sound(7));
        m.play_sound(7);
        assert!(m.take_commands().is_empty());
    }

    #[test]
    fn init_again_invalidates_every_handle() {
        let mut m = model();
        let h = m.load_sound("sound/beep.wav", 0).unwrap();
        m.take_commands();
        assert!(m.init());
        assert_eq!(m.is_sound_playing(h), None);
        assert_eq!(m.take_commands(), vec![Command::Reset]);
        assert_eq!(m.load_sound("sound/beep.wav", 0), Some(h + 1));
    }

    #[test]
    fn commands_follow_the_calls() {
        let mut m = model();
        let h = m.load_sound("sound/song.ogg", 1).unwrap();
        m.set_sound_vol(h, -500);
        m.loop_sound(h);
        m.tick();
        m.set_sound_position(h, 400);
        let path: Arc<str> = "sound/song.ogg".into();
        assert_eq!(
            m.take_commands(),
            vec![
                Command::Load {
                    handle: h,
                    path: path.clone(),
                    stream: true
                },
                Command::Volume {
                    handle: h,
                    millibels: -500
                },
                Command::Play {
                    handle: h,
                    looping: true,
                    frame: 0
                },
                Command::Seek {
                    handle: h,
                    frame: 100
                },
            ]
        );
        assert!(m.take_commands().is_empty());
        assert_eq!(
            m.resync(),
            vec![
                Command::Reset,
                Command::Load {
                    handle: h,
                    path,
                    stream: true
                },
                Command::Volume {
                    handle: h,
                    millibels: -500
                },
                Command::Pan { handle: h, pan: 0 },
                Command::Frequency {
                    handle: h,
                    hz: 44100
                },
                Command::Play {
                    handle: h,
                    looping: true,
                    frame: 100
                },
            ]
        );
    }

    #[test]
    fn clones_stay_in_step() {
        let mut a = model();
        let s = a.load_sound("sound/beep.wav", 0).unwrap();
        let m = a.load_sound("sound/song.ogg", 1).unwrap();
        a.loop_sound(m);
        a.play_sound(s);
        ticks(&mut a, 7);
        let mut b = a.clone();
        for x in [&mut a, &mut b] {
            x.set_sound_freq(m, 30000);
            ticks(x, 40);
            x.pause_sound(m);
            ticks(x, 3);
            x.resume_sound(m);
            x.play_sound(s);
            ticks(x, 100);
        }
        assert_eq!(a, b);
        assert_eq!(a.take_commands(), b.take_commands());
        assert_eq!(a.sound_position(m), b.sound_position(m));
    }
}
