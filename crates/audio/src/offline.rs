//! The game's sound as samples, without an output device: [`backend::Player`] on a kira backend that mixes
//! only when asked, a game tick at a time. The wiki's clips record their soundtrack with it
//! (`wiki-clips`, docs/wiki-media.md).
//!
//! The live player hears the same commands, applied the same way (volumes, pans, rates, loops, kira's
//! resampling and its 10 ms fade when a sound is cut off), so a recording sounds as the game does. What
//! differs is the clock: live, kira's device thread mixes whatever has arrived when the device asks, and
//! the commands land between its buffers at wall-clock times. Here every tick's commands apply, then
//! exactly a tick of samples is mixed, so a sound starts on the frame that started it and the same
//! commands always give the same samples. Music is decoded whole when it starts, rather than streamed as on
//! the device, since kira streams on a thread whose timing would decide what got mixed.

use crate::backend::{BackendError, Player};
use crate::model::{Command, SoundBank, TICKS_PER_SECOND};
use kira::backend::{Backend, Renderer};
use kira::{AudioManager, AudioManagerSettings};
use std::convert::Infallible;
use std::sync::Arc;

/// The mix's sample rate: the usual output device's (a Mac's), and the one Opus encodes at.
pub const SAMPLE_RATE: u32 = 48_000;

/// Stereo frames in a game tick, 1/30 s: a whole number at [`SAMPLE_RATE`].
pub const FRAMES_PER_TICK: usize = (SAMPLE_RATE as u64 / TICKS_PER_SECOND) as usize;

/// A kira backend with no device: it holds the renderer for [`Recorder`] to run.
pub struct Offline {
    renderer: Option<Renderer>,
}

impl Backend for Offline {
    type Settings = ();
    type Error = Infallible;

    fn setup(_: (), _internal_buffer_size: usize) -> Result<(Self, u32), Infallible> {
        Ok((Self { renderer: None }, SAMPLE_RATE))
    }

    fn start(&mut self, renderer: Renderer) -> Result<(), Infallible> {
        self.renderer = Some(renderer);
        Ok(())
    }
}

/// Mixes the model's commands, tick by tick, into interleaved stereo `f32` samples at [`SAMPLE_RATE`].
pub struct Recorder {
    player: Player<Offline>,
}

impl Recorder {
    /// A recorder of files from `bank` (the model's, with their bytes).
    pub fn new(bank: Arc<SoundBank>) -> Self {
        let manager = match AudioManager::<Offline>::new(AudioManagerSettings::default()) {
            Ok(m) => m,
            Err(never) => match never {},
        };
        Self {
            player: Player::with_manager(manager, bank, false),
        }
    }

    /// Applies commands, as [`Player::apply`] does: the game's since the last tick (or since it began),
    /// or a [`resync`](crate::AudioModel::resync).
    pub fn apply(&mut self, commands: &[Command]) -> Result<(), BackendError> {
        self.player.apply(commands)
    }

    /// Mixes the next tick, [`FRAMES_PER_TICK`] frames of left and right samples, onto `out`.
    pub fn mix_tick(&mut self, out: &mut Vec<f32>) {
        let start = out.len();
        out.resize(start + FRAMES_PER_TICK * 2, 0.0);
        let renderer = (self.player.backend_mut().renderer.as_mut())
            .expect("kira starts the backend when it makes the manager");
        // As kira's device backend does before each buffer: takes the sounds and changes sent since.
        renderer.on_start_processing();
        renderer.process(&mut out[start..], 2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::wav;

    /// Half a second of a constant 8000, 16-bit mono at 44100 Hz: a level that resampling keeps exact.
    fn tone() -> Vec<u8> {
        let frames = 22_050u32;
        let mut bytes = wav(44_100, 1, 16, frames * 2);
        let data = bytes.len() - frames as usize * 2;
        for s in bytes[data..].as_chunks_mut::<2>().0 {
            s.copy_from_slice(&8000i16.to_le_bytes());
        }
        bytes
    }

    const LEVEL: f32 = 8000.0 / 32768.0;

    fn bank() -> Arc<SoundBank> {
        let mut bank = SoundBank::new();
        bank.insert("sound/tone.wav", tone()).unwrap();
        Arc::new(bank)
    }

    fn load(bank: &SoundBank, stream: bool) -> Command {
        Command::Load {
            handle: 1,
            path: "sound/tone.wav".into(),
            stream,
            info: bank.get("sound/tone.wav").unwrap(),
        }
    }

    /// Mixes `ticks` ticks, applying `at(tick)`'s commands before each.
    fn record(ticks: usize, at: impl Fn(usize) -> Vec<Command>) -> Vec<f32> {
        let mut r = Recorder::new(bank());
        let mut out = Vec::new();
        for t in 0..ticks {
            r.apply(&at(t)).unwrap();
            r.mix_tick(&mut out);
        }
        out
    }

    fn peak(samples: &[f32]) -> f32 {
        samples.iter().fold(0.0, |m, s| m.max(s.abs()))
    }

    fn tick(samples: &[f32], t: usize) -> &[f32] {
        &samples[t * FRAMES_PER_TICK * 2..(t + 1) * FRAMES_PER_TICK * 2]
    }

    fn played_at(tick: usize, then: Vec<Command>) -> impl Fn(usize) -> Vec<Command> {
        let bank = bank();
        move |t| match t {
            0 => vec![load(&bank, false)],
            t if t == tick => vec![Command::Play {
                handle: 1,
                looping: false,
                frame: 0,
            }],
            t if t == tick + 1 => then.clone(),
            _ => vec![],
        }
    }

    #[test]
    fn the_same_commands_mix_the_same_samples() {
        let a = record(30, played_at(3, vec![]));
        let b = record(30, played_at(3, vec![]));
        assert_eq!(a.len(), 30 * FRAMES_PER_TICK * 2);
        assert!(a.iter().zip(&b).all(|(x, y)| x.to_bits() == y.to_bits()));
    }

    #[test]
    fn a_sound_starts_on_its_tick_and_ends_with_its_file() {
        let s = record(30, played_at(3, vec![]));
        assert_eq!(peak(&s[..3 * FRAMES_PER_TICK * 2]), 0.0);
        // From the tick's first frame, at the file's level.
        assert!(tick(&s, 3)[..8].iter().any(|&x| x != 0.0));
        // (kira's interpolation overshoots by a hair at the file's edges)
        assert!((peak(tick(&s, 3)) - LEVEL).abs() < 1e-3);
        // Half a second is 15 ticks.
        assert!(peak(tick(&s, 17)) > 0.2);
        assert_eq!(peak(tick(&s, 19)), 0.0);
    }

    #[test]
    fn a_stop_cuts_a_sound_off() {
        let s = record(10, played_at(3, vec![Command::Stop { handle: 1 }]));
        assert!(peak(tick(&s, 3)) > 0.2);
        // kira's 10 ms fade, then nothing.
        assert_eq!(peak(&tick(&s, 4)[FRAMES_PER_TICK..]), 0.0);
    }

    #[test]
    fn volume_and_pan_are_directsounds() {
        let s = record(
            6,
            played_at(
                3,
                vec![
                    Command::Volume {
                        handle: 1,
                        millibels: -600,
                    },
                    Command::Pan {
                        handle: 1,
                        pan: 2000,
                    },
                ],
            ),
        );
        let (left, right): (Vec<f32>, Vec<f32>) = (tick(&s, 5).as_chunks::<2>().0.iter())
            .map(|&[l, r]| (l, r))
            .unzip();
        // -6 dB on the near (right) channel, a further -20 dB on the far one.
        assert!(
            (peak(&right) / LEVEL - 0.501).abs() < 1e-3,
            "{}",
            peak(&right)
        );
        assert!(
            (peak(&left) / LEVEL - 0.0501).abs() < 1e-4,
            "{}",
            peak(&left)
        );
    }

    #[test]
    fn music_is_decoded_whole_and_loops() {
        let bank = bank();
        let s = record(40, |t| match t {
            0 => vec![
                load(&bank, true),
                Command::Play {
                    handle: 1,
                    looping: true,
                    frame: 0,
                },
            ],
            _ => vec![],
        });
        // Past the file's 15 ticks, twice.
        assert!((0..40).all(|t| peak(tick(&s, t)) > 0.2));
    }
}
