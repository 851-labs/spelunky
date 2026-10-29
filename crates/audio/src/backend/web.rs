//! kira's cpal backend in the browser, but holding on to its stream, for the stream's audio context: a
//! browser keeps a context silent until the page has had a user gesture, and kira's own backend has no way
//! to ask whether it plays yet, or to start it later ([`crate::backend::Player::running`],
//! [`crate::backend::Player::resume`]).

use kira::backend::cpal::Error;
use kira::backend::cpal::cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use kira::backend::cpal::cpal::{self, Device, Stream, StreamConfig};
use kira::backend::{Backend, Renderer};
use wasm_bindgen::JsValue;
use wasm_bindgen::closure::Closure;
use web_sys::{AudioContext, AudioContextState};

thread_local! {
    /// Takes a refused `resume()`'s rejection, which needs no more than the next try. One for the page's
    /// life: a pending promise can outlive the backend, whose closure it would then call after it's freed.
    static IGNORE: Closure<dyn FnMut(JsValue)> = Closure::new(|_| {});
}

pub struct WebBackend {
    /// The device and its config, until kira starts the stream (at once, in `AudioManager::new`).
    setup: Option<(Device, StreamConfig)>,
    stream: Option<Stream>,
}

impl WebBackend {
    /// Whether the browser plays the sound: its context is running. A stream without a context this knows
    /// counts as playing, so that its sounds aren't held for good.
    pub fn running(&self) -> bool {
        self.context()
            .is_none_or(|c| c.state() == AudioContextState::Running)
    }

    /// Asks the browser to play the sound, if it holds it and the page has a user gesture to grant it
    /// with: Safari passes one on the page around this frame to it too. Without one it's refused, so
    /// it isn't asked (Chrome warns in the console each time).
    pub fn resume(&self) {
        let Some(context) = self.context() else {
            return;
        };
        // Suspended, or Safari's "interrupted" (which web-sys doesn't know), where it holds a context after
        // the stream's own `resume()` outside a gesture.
        if !matches!(
            context.state(),
            AudioContextState::Running | AudioContextState::Closed
        ) && user_activation()
            && let Ok(promise) = context.resume()
        {
            IGNORE.with(|ignore| {
                let _ = promise.catch(ignore);
            });
        }
    }

    fn context(&self) -> Option<&AudioContext> {
        match self.stream.as_ref()?.as_inner() {
            cpal::platform::StreamInner::WebAudio(stream) => Some(stream.audio_context()),
            // cpal's other browser hosts (AudioWorklet, with threads), which this build doesn't enable.
            #[allow(unreachable_patterns)]
            _ => None,
        }
    }
}

/// Whether the page has a user gesture now (`navigator.userActivation.isActive`): a browser without the
/// API (Safari before 16.4) is asked anyway.
fn user_activation() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let activation = window.navigator().user_activation();
    activation.is_undefined() || activation.is_active()
}

impl Backend for WebBackend {
    type Settings = ();
    type Error = Error;

    fn setup(_: (), _internal_buffer_size: usize) -> Result<(Self, u32), Error> {
        let device = cpal::default_host()
            .default_output_device()
            .ok_or(Error::NoDefaultOutputDevice)?;
        let config = device.default_output_config()?.config();
        let sample_rate = config.sample_rate;
        Ok((
            Self {
                setup: Some((device, config)),
                stream: None,
            },
            sample_rate,
        ))
    }

    fn start(&mut self, mut renderer: Renderer) -> Result<(), Error> {
        let (device, config) = self.setup.take().expect("kira starts a backend once");
        let channels = config.channels;
        let stream = device.build_output_stream(
            config,
            move |data: &mut [f32], _| {
                renderer.on_start_processing();
                renderer.process(data, channels);
            },
            |_| {},
            None,
        )?;
        stream.play()?;
        self.stream = Some(stream);
        Ok(())
    }
}
