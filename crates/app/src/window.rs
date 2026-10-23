//! The window and its event loop: ticks the game at a fixed 30 Hz, renders each tick's frame once, and
//! presents the latest frame at display rate.
//!
//! Ticks run on a timer (`ControlFlow::WaitUntil` the next tick's deadline), not on redraws, so the game
//! keeps time while the window is minimised or hidden and the OS stops sending redraws. Redraws only
//! present.
//!
//! Each tick gets the input events that arrived since the previous one, in the replay format
//! ([`crate::input`]), or a replay's events for that frame. A live session is recorded as a replay.
//!
//! Written against winit's `ApplicationHandler` so the same code runs on the web, where nothing may block:
//! GPU setup is a future that native code blocks on and the web build spawns, reporting back through a
//! user event.

use crate::gpu::{self, Gpu};
use crate::input::Collector;
use crate::present::{Presenter, integer_fit};
use crate::render::Renderer;
use crate::textures::Textures;
use draw::Frame;
use std::sync::Arc;
use trace::replay::{Event, MouseButton, Replay};
use web_time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, KeyEvent, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Fullscreen, Window, WindowId};

/// GM's room speed: every Spelunky room runs at 30 steps a second.
pub const TICK: Duration = Duration::from_nanos(1_000_000_000 / 30);

/// Late ticks to run back to back before giving up on the backlog. Timer wake-ups are a little late now
/// and then, which a few catch-up ticks absorb. After a real stall (a debugger, a suspended laptop) the
/// game resumes from now instead of fast-forwarding, as GM 8 does: its runner waits for each step's start
/// time but never runs extra steps to make up for lost ones.
const MAX_CATCH_UP: u32 = 4;

/// What the window drives: anything that turns input into one frame per tick.
pub trait Game {
    /// Runs one tick with its input events, in order.
    fn tick(&mut self, events: &[Event], out: &mut Frame);
}

/// Where the ticks' input comes from.
pub enum Input {
    /// The keyboard, the mouse and, when the header declares a joystick, the first gamepad. Every tick's
    /// events are appended to this replay, which [`App::into_recording`] returns.
    Live(Replay),
    /// A replay's events; the window closes after its last frame.
    Replay(Replay),
}

enum Source {
    Live {
        collector: Collector,
        #[cfg(feature = "gamepad")]
        gamepads: Option<crate::input::Gamepads>,
        recording: Replay,
    },
    Replay {
        replay: Replay,
        /// Index of the next frame's first event.
        next_event: usize,
    },
}

/// Sent to the event loop when asynchronous GPU setup finishes (web only).
pub enum UserEvent {
    GpuReady(Result<(Gpu, wgpu::Surface<'static>), String>),
}

struct Graphics {
    renderer: Renderer,
    presenter: Presenter,
}

pub struct App<G> {
    game: G,
    title: String,
    /// Initial window size in logical pixels.
    size: (u32, u32),
    /// Handed to the renderer once the GPU is ready.
    textures: Option<Textures>,
    window: Option<Arc<Window>>,
    graphics: Option<Graphics>,
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    proxy: EventLoopProxy<UserEvent>,
    source: Source,
    /// Ticks run so far: the next tick's frame number.
    ticks: u64,
    /// The cursor in physical window pixels, while it's over the window.
    cursor: Option<PhysicalPosition<f64>>,
    frame: Frame,
    /// When the next tick is due; `None` until the GPU is ready.
    next_tick: Option<Instant>,
    error: Option<String>,
}

impl<G: Game> App<G> {
    /// `size` is the window's initial size in logical pixels.
    pub fn new(
        game: G,
        textures: Textures,
        title: &str,
        size: (u32, u32),
        proxy: EventLoopProxy<UserEvent>,
        input: Input,
    ) -> App<G> {
        let source = match input {
            Input::Live(recording) => {
                let joystick = recording.header.joystick.is_some();
                Source::Live {
                    collector: Collector::new(joystick),
                    #[cfg(feature = "gamepad")]
                    gamepads: joystick.then(crate::input::Gamepads::new).flatten(),
                    recording,
                }
            }
            Input::Replay(replay) => Source::Replay {
                replay,
                next_event: 0,
            },
        };
        App {
            game,
            title: title.to_owned(),
            size,
            textures: Some(textures),
            window: None,
            graphics: None,
            proxy,
            source,
            ticks: 0,
            cursor: None,
            frame: Frame::default(),
            next_tick: None,
            error: None,
        }
    }

    /// The error that ended the event loop, if any.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Ticks run so far.
    pub fn ticks(&self) -> u64 {
        self.ticks
    }

    pub fn game(&self) -> &G {
        &self.game
    }

    /// A live session's recording: every tick's events, one frame per tick run. `None` for a replay.
    pub fn into_recording(self) -> Option<Replay> {
        match self.source {
            Source::Live { recording, .. } => Some(recording),
            Source::Replay { .. } => None,
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: String) {
        self.error = Some(error);
        event_loop.exit();
    }

    fn gpu_ready(
        &mut self,
        event_loop: &ActiveEventLoop,
        gpu: Gpu,
        surface: wgpu::Surface<'static>,
    ) {
        let Some(window) = &self.window else { return };
        let size = window.inner_size();
        let presenter = match Presenter::new(
            surface,
            &gpu.adapter,
            &gpu.device,
            (size.width, size.height),
        ) {
            Ok(p) => p,
            Err(e) => return self.fail(event_loop, e),
        };
        let textures = self.textures.take().expect("the GPU becomes ready once");
        let renderer = Renderer::new(gpu.device, gpu.queue, textures);
        self.graphics = Some(Graphics {
            renderer,
            presenter,
        });
        // The first tick runs straight away.
        self.next_tick = Some(Instant::now());
    }

    /// The cursor in GM window pixels: the back buffer's, which the window shows scaled and centred.
    fn cursor_in_screen(&self) -> Option<(i32, i32)> {
        let pos = self.cursor?;
        let (screen, _) = self.graphics.as_ref()?.renderer.screen()?;
        let size = self.window.as_ref()?.inner_size();
        let (scale, (x, y)) =
            integer_fit((screen.width(), screen.height()), (size.width, size.height));
        let map = |p: f64, offset: i32| ((p - f64::from(offset)) / f64::from(scale)).floor() as i32;
        Some((map(pos.x, x), map(pos.y, y)))
    }

    /// The next tick's events (recording them when live), or `None` once a replay has run out.
    fn next_events(&mut self) -> Result<Option<Vec<Event>>, String> {
        let frame = self.ticks;
        let cursor = self.cursor_in_screen();
        match &mut self.source {
            Source::Live {
                collector,
                #[cfg(feature = "gamepad")]
                gamepads,
                recording,
            } => {
                if let Some((x, y)) = cursor {
                    collector.cursor(x, y);
                }
                #[cfg(feature = "gamepad")]
                let pad = gamepads.as_mut().and_then(|g| g.poll());
                #[cfg(not(feature = "gamepad"))]
                let pad: Option<crate::input::PadState> = None;
                let events = collector.take_tick(pad.as_ref());
                for &e in &events {
                    recording.push(frame, e).map_err(|e| e.to_string())?;
                }
                recording.set_len(frame + 1).map_err(|e| e.to_string())?;
                Ok(Some(events))
            }
            Source::Replay { replay, next_event } => {
                if frame >= replay.len() {
                    return Ok(None);
                }
                let rest = &replay.events()[*next_event..];
                let n = rest.iter().take_while(|(f, _)| *f == frame).count();
                *next_event += n;
                Ok(Some(rest[..n].iter().map(|&(_, e)| e).collect()))
            }
        }
    }

    /// Runs every tick that's due, rendering each one's frame. Returns when the next one is due; `None`
    /// before the GPU is ready and once the run is over.
    fn run_due_ticks(&mut self, event_loop: &ActiveEventLoop) -> Option<Instant> {
        self.graphics.as_ref()?;
        let mut next = self.next_tick?;
        let now = Instant::now();
        let mut ticks = 0;
        while next <= now {
            if ticks == MAX_CATCH_UP {
                next = now + TICK;
                break;
            }
            let events = match self.next_events() {
                Ok(Some(events)) => events,
                Ok(None) => {
                    event_loop.exit();
                    return None;
                }
                Err(e) => {
                    self.fail(event_loop, e);
                    return None;
                }
            };
            self.game.tick(&events, &mut self.frame);
            self.ticks += 1;
            // Every tick's frame is rendered, even ones never shown, because surfaces carry state.
            self.graphics.as_mut()?.renderer.render(&self.frame);
            next += TICK;
            ticks += 1;
        }
        self.next_tick = Some(next);
        if ticks > 0
            && let Some(window) = &self.window
        {
            window.request_redraw();
        }
        Some(next)
    }

    /// The live session's input collector; `None` while playing a replay, which ignores the devices.
    fn collector(&mut self) -> Option<&mut Collector> {
        match &mut self.source {
            Source::Live { collector, .. } => Some(collector),
            Source::Replay { .. } => None,
        }
    }
}

impl<G: Game> ApplicationHandler<UserEvent> for App<G> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title(&self.title)
            .with_inner_size(LogicalSize::new(self.size.0, self.size.1))
            .with_min_inner_size(LogicalSize::new(320, 240));
        #[cfg(target_arch = "wasm32")]
        let attributes = {
            use winit::platform::web::WindowAttributesExtWebSys;
            attributes.with_append(true)
        };
        let window = match event_loop.create_window(attributes) {
            Ok(w) => Arc::new(w),
            Err(e) => return self.fail(event_loop, format!("can't open a window: {e}")),
        };
        self.window = Some(window.clone());

        let instance = gpu::instance();
        let surface = match instance.create_surface(window) {
            Ok(s) => s,
            Err(e) => return self.fail(event_loop, format!("can't draw to the window: {e}")),
        };
        #[cfg(not(target_arch = "wasm32"))]
        match pollster::block_on(gpu::request(instance, Some(&surface))) {
            Ok(gpu) => self.gpu_ready(event_loop, gpu, surface),
            Err(e) => self.fail(event_loop, e),
        }
        #[cfg(target_arch = "wasm32")]
        {
            let proxy = self.proxy.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let result = gpu::request(instance, Some(&surface))
                    .await
                    .map(|gpu| (gpu, surface));
                let _ = proxy.send_event(UserEvent::GpuReady(result));
            });
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::GpuReady(Ok((gpu, surface))) => self.gpu_ready(event_loop, gpu, surface),
            UserEvent::GpuReady(Err(e)) => self.fail(event_loop, e),
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(g) = &mut self.graphics {
                    g.presenter
                        .resize(g.renderer.device(), (size.width, size.height));
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(g) = &mut self.graphics {
                    // Nothing to show while minimised or hidden; the next tick asks again.
                    g.presenter.present(&g.renderer);
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state,
                        repeat,
                        ..
                    },
                ..
            } => {
                let down = state == ElementState::Pressed;
                // F4 toggles fullscreen, as in the original's game settings.
                if down
                    && key == KeyCode::F4
                    && !repeat
                    && let Some(window) = &self.window
                {
                    let full = window.fullscreen().is_none();
                    window.set_fullscreen(full.then_some(Fullscreen::Borderless(None)));
                }
                if let Some(c) = self.collector() {
                    c.key(key, down);
                }
            }
            WindowEvent::CursorMoved { position, .. } => self.cursor = Some(position),
            WindowEvent::CursorLeft { .. } => self.cursor = None,
            WindowEvent::MouseInput { state, button, .. } => {
                let b = match button {
                    winit::event::MouseButton::Left => MouseButton::Left,
                    winit::event::MouseButton::Right => MouseButton::Right,
                    winit::event::MouseButton::Middle => MouseButton::Middle,
                    _ => return,
                };
                if let Some(c) = self.collector() {
                    c.mouse_button(b, state == ElementState::Pressed);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let dy = match delta {
                    MouseScrollDelta::LineDelta(_, y) => f64::from(y),
                    MouseScrollDelta::PixelDelta(p) => p.y,
                };
                if dy != 0.0
                    && let Some(c) = self.collector()
                {
                    c.wheel(dy > 0.0);
                }
            }
            WindowEvent::Focused(false) => {
                if let Some(c) = self.collector() {
                    c.focus_lost();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        match self.run_due_ticks(event_loop) {
            Some(next) => event_loop.set_control_flow(ControlFlow::WaitUntil(next)),
            None => event_loop.set_control_flow(ControlFlow::Wait),
        }
    }
}
