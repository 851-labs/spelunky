//! Plays the model's [`Command`]s on the default output device, with [kira](https://docs.rs/kira); or, on
//! [`crate::offline`]'s backend, mixes them into samples without one.
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
//! starts audio, which is the app's job: [`Player::running`] says whether it has, and [`Player::resume`]
//! asks again (there kira runs on `web::WebBackend`, which keeps the stream's audio context). The offline
//! mixer decodes music the same way, since kira streams
//! on a thread of its own, whose timing would decide what it mixes.

use crate::model::{Command, Handle, SoundBank};
#[cfg(feature = "backend")]
use kira::AudioManagerSettings;
use kira::backend::Backend;
use kira::sound::PlaybackPosition;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
#[cfg(not(target_arch = "wasm32"))]
use kira::sound::streaming::{StreamingSoundData, StreamingSoundHandle};
use kira::{AudioManager, Decibels, Easing, Panning, PlaybackRate, StartTime, Tween};
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;

#[cfg(all(feature = "backend", target_arch = "wasm32"))]
mod web;

/// The device's kira backend: in the browser, one that can tell whether the sound plays yet.
#[cfg(all(feature = "backend", target_arch = "wasm32"))]
type DeviceBackend = web::WebBackend;
#[cfg(not(all(feature = "backend", target_arch = "wasm32")))]
type DeviceBackend = kira::DefaultBackend;

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

fn decode_error(path: &str, e: impl fmt::Display) -> BackendError {
    BackendError::Decode {
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
    ($data:expr, $slot:expr, $start:expr) => {{
        let (db, panning) = $slot.volume_and_panning();
        let data = $data
            .volume(db)
            .panning(panning)
            .playback_rate($slot.playback_rate())
            .start_position(PlaybackPosition::Samples($start.frame as usize));
        if $start.looping {
            data.loop_region(..)
        } else {
            data
        }
    }};
}

/// A play the model asked for this tick, started once the tick's other commands are in.
#[derive(Clone, Copy)]
struct Start {
    looping: bool,
    frame: u64,
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
    start: Option<Start>,
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

    /// Stops the voice, and forgets a play not started yet.
    fn stop(&mut self) {
        self.start = None;
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
// Only shapes what the device plays, never game state, so platform float math is fine (clippy.toml).
#[allow(clippy::disallowed_methods)]
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

/// Applies [`Command`]s to the default output device, or to another kira backend (`B`).
pub struct Player<B: Backend = DeviceBackend> {
    manager: AudioManager<B>,
    /// The model's bank: where the bytes of every file it loads come from.
    bank: Arc<SoundBank>,
    /// Decoded effects, by path.
    decoded: HashMap<Arc<str>, StaticSoundData>,
    slots: BTreeMap<Handle, Slot>,
    /// Music streams from memory as it plays, rather than being decoded whole when it starts.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    streams: bool,
}

#[cfg(feature = "backend")]
impl Player {
    /// Opens the default output device, to play files from `bank` (the model's, with their bytes:
    /// [`SoundBank::insert`]).
    pub fn new(bank: Arc<SoundBank>) -> Result<Self, BackendError> {
        let manager = AudioManager::<DeviceBackend>::new(AudioManagerSettings::default())
            .map_err(|e| BackendError::Device(e.to_string()))?;
        // kira can't stream on wasm.
        Ok(Self::with_manager(
            manager,
            bank,
            cfg!(not(target_arch = "wasm32")),
        ))
    }

    /// Whether its sound reaches the device. A browser holds it until the page has had a user gesture:
    /// meanwhile the commands would pile up in kira and all sound at once when it starts.
    pub fn running(&mut self) -> bool {
        #[cfg(target_arch = "wasm32")]
        return self.manager.backend_mut().running();
        #[cfg(not(target_arch = "wasm32"))]
        true
    }

    /// Asks the browser for the sound again, if it holds it and the page has a user gesture to grant it
    /// with (a key, click or tap in the last moment, in this frame or, for Safari, the page around it).
    pub fn resume(&mut self) {
        #[cfg(target_arch = "wasm32")]
        self.manager.backend_mut().resume();
    }
}

impl<B: Backend> Player<B> {
    /// A player on `manager`'s backend, which streams music if `streams` (natively only).
    pub(crate) fn with_manager(
        manager: AudioManager<B>,
        bank: Arc<SoundBank>,
        streams: bool,
    ) -> Self {
        Self {
            manager,
            bank,
            decoded: HashMap::new(),
            slots: BTreeMap::new(),
            streams,
        }
    }

    /// The kira backend, for [`crate::offline`] to render with.
    pub(crate) fn backend_mut(&mut self) -> &mut B {
        self.manager.backend_mut()
    }

    /// Applies a tick's commands. Plays start after the tick's other commands, so a sound the game
    /// starts and then sets up in the same frame (`playMusic`, then `SS_SetSoundVol`) starts with its
    /// final volume and frequency, not a burst of the old ones. A command that fails is skipped; the first
    /// error is returned after the rest have been applied.
    pub fn apply(&mut self, commands: &[Command]) -> Result<(), BackendError> {
        let mut first_error = None;
        for command in commands {
            if let Err(e) = self.apply_one(command) {
                first_error.get_or_insert(e);
            }
        }
        let starts: Vec<Handle> = self
            .slots
            .iter()
            .filter(|(_, s)| s.start.is_some())
            .map(|(&h, _)| h)
            .collect();
        for handle in starts {
            if let Err(e) = self.start(handle) {
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
                        .map_err(|e| decode_error(path, e))?;
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
                        start: None,
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
                if let Some(slot) = self.slots.get_mut(&handle) {
                    // The last play is cut off now; the new one starts at the end of the tick.
                    slot.stop();
                    slot.start = Some(Start { looping, frame });
                }
            }
            Command::Stop { handle } => {
                if let Some(slot) = self.slots.get_mut(&handle) {
                    slot.stop();
                }
            }
            Command::Seek { handle, frame } => {
                if let Some(slot) = self.slots.get_mut(&handle) {
                    if let Some(start) = &mut slot.start {
                        start.frame = frame;
                    }
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

    /// Starts a slot's pending play with its current parameters.
    fn start(&mut self, handle: Handle) -> Result<(), BackendError> {
        let Some(slot) = self.slots.get(&handle) else {
            return Ok(());
        };
        enum Source {
            Decoded(Box<StaticSoundData>),
            Bytes(Arc<[u8]>),
        }
        let path = slot.path.clone();
        // Effects were decoded at load; anything else (music) needs its bytes.
        let source = match self.decoded.get(&path) {
            Some(data) => Source::Decoded(Box::new(data.clone())),
            None => Source::Bytes(self.file(&path)?),
        };
        let slot = self.slots.get_mut(&handle).expect("looked up above");
        let Some(start) = slot.start.take() else {
            return Ok(());
        };
        let voice = match source {
            Source::Decoded(data) => {
                let data = configured!(*data, slot, start);
                Voice::Static(self.manager.play(data).map_err(play_error)?)
            }
            #[cfg(not(target_arch = "wasm32"))]
            Source::Bytes(bytes) if slot.stream && self.streams => {
                let data = StreamingSoundData::from_cursor(Cursor::new(bytes))
                    .map_err(|e| decode_error(&path, e))?;
                let data = configured!(data, slot, start);
                Voice::Stream(self.manager.play(data).map_err(play_error)?)
            }
            // Music on wasm or offline: decoded for this play only.
            Source::Bytes(bytes) => {
                let data = StaticSoundData::from_cursor(Cursor::new(bytes))
                    .map_err(|e| decode_error(&path, e))?;
                let data = configured!(data, slot, start);
                Voice::Static(self.manager.play(data).map_err(play_error)?)
            }
        };
        slot.voice = Some(voice);
        Ok(())
    }

    fn file(&self, path: &str) -> Result<Arc<[u8]>, BackendError> {
        self.bank
            .bytes(path)
            .cloned()
            .ok_or_else(|| decode_error(path, "not in the sound bank, or added without its bytes"))
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
    #[allow(clippy::disallowed_methods)] // Checks output gains, not game state.
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
