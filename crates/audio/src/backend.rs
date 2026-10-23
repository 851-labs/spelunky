//! Plays the model's [`Command`]s on the default output device, with [kira](https://docs.rs/kira).
//!
//! Why kira: a DirectSound buffer's controls that the game uses map straight onto it (volume in
//! decibels, pan, playback rate as speed and pitch, seeking, loop regions, streaming Ogg from memory),
//! it mixes on its own thread, and it runs on the same cpal backend natively and in the browser
//! (WebAudio). rodio could do the job with more glue (amplitude volume, no loop regions or tweens), and
//! cpal alone would mean writing a resampling mixer and an Ogg streamer here. The backend is only a
//! sink: it never answers the game, so swapping it changes how the game sounds and nothing else.
//!
//! Each handle is one voice, like a DirectSound buffer: playing it again cuts the last play off. Effects
//! are decoded when they're loaded. Music (the DLL's streamed sounds) streams from memory natively;
//! kira can't stream on wasm, so there a track is decoded whole each time it starts, which stalls that
//! frame and holds the decoded track while it plays. The browser also needs a user gesture before it
//! starts audio, which is the app's job.

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

fn decode_error(path: &str) -> impl Fn(&dyn fmt::Display) -> BackendError + '_ {
    move |e| BackendError::Decode {
        path: path.to_owned(),
        error: e.to_string(),
    }
}

fn play_error(e: impl fmt::Display) -> BackendError {
    BackendError::Play(e.to_string())
}

/// A playing kira sound.
enum Voice {
    Static(StaticSoundHandle),
    #[cfg(not(target_arch = "wasm32"))]
    Stream(StreamingSoundHandle<kira::sound::FromFileError>),
}

/// Runs the same code on whichever handle a voice holds.
macro_rules! on_voice {
    ($voice:expr, $h:ident => $body:expr) => {
        match $voice {
            Voice::Static($h) => $body,
            #[cfg(not(target_arch = "wasm32"))]
            Voice::Stream($h) => $body,
        }
    };
}

/// Sets up kira sound data (static or streaming: their builders share these methods) to play a slot
/// from `frame`, once or looping.
macro_rules! configured {
    ($data:expr, $slot:expr, $frame:expr, $looping:expr) => {{
        let (db, panning) = $slot.volume_and_panning();
        let data = $data
            .volume(db)
            .panning(panning)
            .playback_rate($slot.playback_rate())
            .start_position(PlaybackPosition::Samples($frame as usize));
        if $looping { data.loop_region(..) } else { data }
    }};
}

/// What the player knows about a handle: its file and the parameters a new play starts with.
struct Slot {
    path: Arc<str>,
    stream: bool,
    sample_rate: u32,
    volume: i32,
    pan: i32,
    frequency: u32,
    voice: Option<Voice>,
}

impl Slot {
    /// kira's volume and panning for the DirectSound volume and pan (see [`directsound_pan`]).
    fn volume_and_panning(&self) -> (Decibels, Panning) {
        let (panning, correction) = directsound_pan(self.pan);
        (Decibels(self.volume as f32 / 100.0 + correction), panning)
    }

    fn playback_rate(&self) -> PlaybackRate {
        PlaybackRate(f64::from(self.frequency) / f64::from(self.sample_rate))
    }

    /// Sends the volume, and with `pan` the panning, to a playing voice.
    fn update_volume(&mut self, pan: bool) {
        let (db, panning) = self.volume_and_panning();
        if let Some(voice) = &mut self.voice {
            on_voice!(voice, h => {
                h.set_volume(db, INSTANT);
                if pan {
                    h.set_panning(panning, INSTANT);
                }
            });
        }
    }

    fn update_rate(&mut self) {
        let rate = self.playback_rate();
        if let Some(voice) = &mut self.voice {
            on_voice!(voice, h => h.set_playback_rate(rate, INSTANT));
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
    /// Decoded effects, by path.
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
                info,
            } => {
                if let Some(mut old) = self.slots.remove(&handle) {
                    old.stop();
                }
                let bytes = self.file(path)?;
                // Decode effects now rather than on their first play, in the middle of the game.
                if !stream && !self.decoded.contains_key(path) {
                    let data = StaticSoundData::from_cursor(Cursor::new(bytes))
                        .map_err(|e| decode_error(path)(&e))?;
                    self.decoded.insert(path.clone(), data);
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
                let bytes = self.files.get(slot.path.as_ref()).cloned();
                let bytes =
                    || bytes.ok_or_else(|| decode_error(&slot.path)(&"not added to the player"));
                let voice = match slot.stream {
                    #[cfg(not(target_arch = "wasm32"))]
                    true => {
                        let data = StreamingSoundData::from_cursor(Cursor::new(bytes()?))
                            .map_err(|e| decode_error(&slot.path)(&e))?;
                        let data = configured!(data, slot, frame, looping);
                        Voice::Stream(self.manager.play(data).map_err(play_error)?)
                    }
                    _ => {
                        let data = match self.decoded.get(&slot.path) {
                            Some(data) => data.clone(),
                            // wasm's music: decoded for this play only.
                            None => StaticSoundData::from_cursor(Cursor::new(bytes()?))
                                .map_err(|e| decode_error(&slot.path)(&e))?,
                        };
                        let data = configured!(data, slot, frame, looping);
                        Voice::Static(self.manager.play(data).map_err(play_error)?)
                    }
                };
                slot.voice = Some(voice);
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
                    slot.update_volume(false);
                }
            }
            Command::Pan { handle, pan } => {
                if let Some(slot) = self.slots.get_mut(&handle) {
                    slot.pan = pan;
                    // The pan's correction is part of the volume.
                    slot.update_volume(true);
                }
            }
            Command::Frequency { handle, hz } => {
                if let Some(slot) = self.slots.get_mut(&handle) {
                    slot.frequency = hz;
                    slot.update_rate();
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

    fn file(&self, path: &str) -> Result<Arc<[u8]>, BackendError> {
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| decode_error(path)(&"not added to the player"))
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
