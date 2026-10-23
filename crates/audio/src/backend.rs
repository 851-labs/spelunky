//! Plays the model's [`Command`]s on the default output device, with [kira](https://docs.rs/kira).
//!
//! Why kira: it already does everything a DirectSound buffer does that the game uses (volume in
//! decibels, pan, playback rate as speed and pitch, seeking, looping, streaming Ogg) with its own mixer
//! thread, on the same cpal backend natively and in the browser (WebAudio). rodio has no exact decibel
//! volume or seek-while-streaming, and cpal alone would mean writing a resampling mixer and an Ogg
//! streamer here. The backend is a sink: it never answers the game, so a different one changes nothing
//! but how it sounds.
//!
//! Each handle is one voice, like a DirectSound buffer: playing it again cuts the last play off. Music
//! (the DLL's streamed sounds) streams from memory natively. kira can't stream on wasm, so there every
//! sound is decoded whole on its first play and cached; the browser also needs a user gesture before it
//! starts audio, which is the app's job.

use crate::format::SoundInfo;
use crate::model::{Command, Handle, normalize_path};
use kira::sound::PlaybackPosition;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
#[cfg(not(target_arch = "wasm32"))]
use kira::sound::streaming::{StreamingSoundData, StreamingSoundHandle};
use kira::{
    AudioManager, AudioManagerSettings, Decibels, DefaultBackend, Easing, Panning, PlaybackRate,
    StartTime, Tween,
};
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;

/// DirectSound applies volume, pan and frequency at once.
const INSTANT: Tween = Tween {
    start_time: StartTime::Immediate,
    duration: Duration::ZERO,
    easing: Easing::Linear,
};

#[derive(Debug)]
pub enum BackendError {
    /// No output device, or it failed to start.
    Device(String),
    /// A loaded file is missing from the player or doesn't decode.
    Decode { path: String, error: String },
    /// kira refused to play a sound (e.g. too many at once).
    Play(String),
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Device(e) => write!(f, "audio device: {e}"),
            Self::Decode { path, error } => write!(f, "{path}: {error}"),
            Self::Play(e) => write!(f, "playing a sound: {e}"),
        }
    }
}

impl std::error::Error for BackendError {}

/// A playing kira sound.
enum Voice {
    Static(StaticSoundHandle),
    #[cfg(not(target_arch = "wasm32"))]
    Stream(StreamingSoundHandle<kira::sound::FromFileError>),
}

/// Calls the same method on whichever handle a voice holds.
macro_rules! on_voice {
    ($voice:expr, $h:ident => $body:expr) => {
        match $voice {
            Voice::Static($h) => $body,
            #[cfg(not(target_arch = "wasm32"))]
            Voice::Stream($h) => $body,
        }
    };
}

/// What the player knows about a handle: its file and the parameters a new play starts with.
struct Slot {
    path: Arc<str>,
    // kira can't stream on wasm, so there every sound plays from memory.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    stream: bool,
    sample_rate: u32,
    volume: i32,
    pan: i32,
    frequency: u32,
    voice: Option<Voice>,
}

impl Slot {
    /// kira's volume: the DirectSound volume plus the pan's correction (see [`directsound_pan`]).
    fn decibels(&self) -> Decibels {
        Decibels(self.volume as f32 / 100.0 + directsound_pan(self.pan).1)
    }

    fn panning(&self) -> Panning {
        directsound_pan(self.pan).0
    }

    fn playback_rate(&self) -> PlaybackRate {
        PlaybackRate(f64::from(self.frequency) / f64::from(self.sample_rate))
    }

    fn update_voice(&mut self) {
        let (db, panning, rate) = (self.decibels(), self.panning(), self.playback_rate());
        if let Some(voice) = &mut self.voice {
            on_voice!(voice, h => {
                h.set_volume(db, INSTANT);
                h.set_panning(panning, INSTANT);
                h.set_playback_rate(rate, INSTANT);
            });
        }
    }

    fn stop(&mut self) {
        if let Some(mut voice) = self.voice.take() {
            // kira's default 10 ms fade, so a cut-off doesn't click.
            on_voice!(&mut voice, h => h.stop(Tween::default()));
        }
    }
}

/// DirectSound pans by attenuating the other channel by `|pan|` hundredths of a dB. kira pans with an
/// equal-power law that also raises the near channel (by 3 dB at the extremes). This returns the kira
/// panning with the same ratio between the channels, and the decibels to add so the near channel keeps
/// its level.
fn directsound_pan(pan: i32) -> (Panning, f32) {
    if pan == 0 {
        return (Panning::CENTER, 0.0);
    }
    // The far channel's amplitude relative to the near one's; kira's ratio is sqrt((1-x)/(1+x)).
    let ratio = 10f32.powf(-(pan.unsigned_abs() as f32) / 2000.0);
    let x = (1.0 - ratio * ratio) / (1.0 + ratio * ratio);
    let near_gain_db = 10.0 * (1.0 + x).log10();
    (Panning(x.copysign(pan as f32)), -near_gain_db)
}

/// Applies [`Command`]s to the default output device.
pub struct Player {
    manager: AudioManager<DefaultBackend>,
    files: HashMap<String, Arc<[u8]>>,
    decoded: HashMap<Arc<str>, StaticSoundData>,
    slots: BTreeMap<Handle, Slot>,
}

impl Player {
    /// Opens the default output device.
    pub fn new() -> Result<Self, BackendError> {
        let manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
            .map_err(|e| BackendError::Device(e.to_string()))?;
        Ok(Self {
            manager,
            files: HashMap::new(),
            decoded: HashMap::new(),
            slots: BTreeMap::new(),
        })
    }

    /// Makes a file available under the path the game loads it with (as in [`crate::SoundBank`]).
    pub fn add_file(&mut self, path: &str, bytes: impl Into<Arc<[u8]>>) {
        self.files.insert(normalize_path(path), bytes.into());
    }

    /// Applies a tick's commands in order. A command that fails is skipped; the first error is returned
    /// after the rest have been applied.
    pub fn apply(&mut self, commands: &[Command]) -> Result<(), BackendError> {
        let mut first_error = None;
        for command in commands {
            if let Err(e) = self.apply_one(command) {
                first_error.get_or_insert(e);
            }
        }
        first_error.map_or(Ok(()), Err)
    }

    fn apply_one(&mut self, command: &Command) -> Result<(), BackendError> {
        match *command {
            Command::Load {
                handle,
                ref path,
                stream,
            } => {
                let info =
                    SoundInfo::parse(self.file(path)?).map_err(|e| BackendError::Decode {
                        path: path.to_string(),
                        error: e.to_string(),
                    })?;
                if let Some(mut old) = self.slots.remove(&handle) {
                    old.stop();
                }
                self.slots.insert(
                    handle,
                    Slot {
                        path: path.clone(),
                        stream,
                        sample_rate: info.sample_rate,
                        volume: 0,
                        pan: 0,
                        frequency: info.sample_rate,
                        voice: None,
                    },
                );
            }
            Command::Free { handle } => {
                if let Some(mut slot) = self.slots.remove(&handle) {
                    slot.stop();
                }
            }
            Command::Play {
                handle,
                looping,
                frame,
            } => {
                let Some(slot) = self.slots.get_mut(&handle) else {
                    return Ok(());
                };
                slot.stop();
                let start = PlaybackPosition::Samples(frame as usize);
                let (db, panning, rate) = (slot.decibels(), slot.panning(), slot.playback_rate());
                let error = |e: &dyn fmt::Display| BackendError::Decode {
                    path: slot.path.to_string(),
                    error: e.to_string(),
                };
                #[cfg(not(target_arch = "wasm32"))]
                if slot.stream {
                    let bytes = self
                        .files
                        .get(slot.path.as_ref())
                        .cloned()
                        .ok_or_else(|| error(&"not added to the player"))?;
                    let mut data = StreamingSoundData::from_cursor(Cursor::new(bytes))
                        .map_err(|e| error(&e))?
                        .volume(db)
                        .panning(panning)
                        .playback_rate(rate)
                        .start_position(start);
                    if looping {
                        data = data.loop_region(..);
                    }
                    let voice = self
                        .manager
                        .play(data)
                        .map_err(|e| BackendError::Play(e.to_string()))?;
                    slot.voice = Some(Voice::Stream(voice));
                    return Ok(());
                }
                let data = match self.decoded.get(&slot.path) {
                    Some(data) => data.clone(),
                    None => {
                        let bytes = self
                            .files
                            .get(slot.path.as_ref())
                            .cloned()
                            .ok_or_else(|| error(&"not added to the player"))?;
                        let data = StaticSoundData::from_cursor(Cursor::new(bytes))
                            .map_err(|e| error(&e))?;
                        self.decoded.insert(slot.path.clone(), data.clone());
                        data
                    }
                };
                let mut data = data
                    .volume(db)
                    .panning(panning)
                    .playback_rate(rate)
                    .start_position(start);
                if looping {
                    data = data.loop_region(..);
                }
                let voice = self
                    .manager
                    .play(data)
                    .map_err(|e| BackendError::Play(e.to_string()))?;
                slot.voice = Some(Voice::Static(voice));
            }
            Command::Stop { handle } => {
                if let Some(slot) = self.slots.get_mut(&handle) {
                    slot.stop();
                }
            }
            Command::Seek { handle, frame } => {
                if let Some(slot) = self.slots.get_mut(&handle) {
                    let seconds = frame as f64 / f64::from(slot.sample_rate);
                    if let Some(voice) = &mut slot.voice {
                        on_voice!(voice, h => h.seek_to(seconds));
                    }
                }
            }
            Command::Volume { handle, millibels } => {
                if let Some(slot) = self.slots.get_mut(&handle) {
                    slot.volume = millibels;
                    slot.update_voice();
                }
            }
            Command::Pan { handle, pan } => {
                if let Some(slot) = self.slots.get_mut(&handle) {
                    slot.pan = pan;
                    slot.update_voice();
                }
            }
            Command::Frequency { handle, hz } => {
                if let Some(slot) = self.slots.get_mut(&handle) {
                    slot.frequency = hz;
                    slot.update_voice();
                }
            }
            Command::Reset => {
                for slot in self.slots.values_mut() {
                    slot.stop();
                }
                self.slots.clear();
            }
        }
        Ok(())
    }

    fn file(&self, path: &str) -> Result<&[u8], BackendError> {
        self.files
            .get(path)
            .map(|b| &b[..])
            .ok_or_else(|| BackendError::Decode {
                path: path.to_owned(),
                error: "not added to the player".to_owned(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// kira's channel gains at panning `x` (its `Frame::panned`).
    fn kira_gains(x: f32) -> (f32, f32) {
        if x == 0.0 {
            return (1.0, 1.0);
        }
        let mix = (x + 1.0) * 0.5;
        (
            (1.0 - mix).sqrt() * std::f32::consts::SQRT_2,
            mix.sqrt() * std::f32::consts::SQRT_2,
        )
    }

    #[test]
    fn pan_matches_directsound_gains() {
        for pan in [-10_000, -2000, -1, 0, 600, 10_000] {
            let (panning, db) = directsound_pan(pan);
            let (l, r) = kira_gains(panning.0);
            let scale = 10f32.powf(db / 20.0);
            // DirectSound: the far channel loses |pan| hundredths of a dB, the near one nothing.
            let far = 10f32.powf(-(pan.abs() as f32) / 2000.0);
            let (want_l, want_r) = if pan > 0 { (far, 1.0) } else { (1.0, far) };
            assert!(
                (l * scale - want_l).abs() < 1e-4,
                "pan {pan}: left {}",
                l * scale
            );
            assert!(
                (r * scale - want_r).abs() < 1e-4,
                "pan {pan}: right {}",
                r * scale
            );
        }
    }
}
