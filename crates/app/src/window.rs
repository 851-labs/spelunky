//! The window and its event loop: ticks the game at a fixed 30 Hz, renders each tick's frame once, and
//! presents the latest frame at display rate.
//!
//! Ticks run on a timer (`ControlFlow::WaitUntil` the next tick's deadline), not on redraws, so the game
//! keeps time while the window is minimised or hidden and the OS stops sending redraws. Redraws only
//! present.
//!
//! Each tick gets the input events that arrived since the previous one, in the replay format
//! ([`crate::input`]), or a replay's events for that frame. A live session can be recorded as it goes.
//!
//! Written against winit's `ApplicationHandler` so the same code runs on the web, where nothing may block:
//! GPU setup is a future that native code blocks on and the web build spawns, reporting back through a
//! user event.

use crate::gpu::{self, Gpu};
use crate::input::{Collector, Gamepads, KeyMap, PAD_JOYSTICK};
use crate::present::{Presenter, integer_fit};
use crate::render::Renderer;
use crate::textures::Textures;
use draw::Frame;
use std::sync::Arc;
use trace::replay::{Event, MouseButton, Replay, ReplayHeader};
use web_time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, KeyEvent, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy};
use winit::keyboard::{Key, KeyCode, PhysicalKey};
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
    /// Called once, before the first tick, with the run's replay header. A live session settles its header
    /// (whether it has a joystick) only then, when the platform has had time to report gamepads.
    fn begin(&mut self, _header: &ReplayHeader) {}

    /// Runs one tick with its input events, in order.
    fn tick(&mut self, events: &[Event], out: &mut Frame);
}

/// Where a live session's input is recorded. It's written as it happens, so a crash or a kill loses at
/// most the closing `frames` line, which the replay format can do without.
pub trait RecordSink {
    fn begin(&mut self, header: &ReplayHeader) -> Result<(), String>;
    /// One tick's events (often none).
    fn frame(&mut self, frame: u64, events: &[Event]) -> Result<(), String>;
    /// The session ended after `frames` ticks.
    fn finish(&mut self, frames: u64) -> Result<(), String>;
}

/// Where the ticks' input comes from.
pub enum Input {
    /// The keyboard, the mouse and the first of `gamepads`. The joystick is declared in `header` if a pad
    /// is connected by the first tick.
    Live {
        header: ReplayHeader,
        keys: KeyMap,
        gamepads: Option<Gamepads>,
        record: Option<Box<dyn RecordSink>>,
    },
    /// A replay's events; the window closes after showing its last frame.
    Replay(Replay),
}

// One per app, so the variants' size difference costs nothing.
#[allow(clippy::large_enum_variant)]
enum Source {
    Live {
        header: ReplayHeader,
        collector: Collector,
        gamepads: Option<Gamepads>,
        record: Option<Box<dyn RecordSink>>,
    },
    Replay(Replay),
}

/// Sent to the event loop from elsewhere.
pub enum UserEvent {
    /// Asynchronous GPU setup finished (web only).
    GpuReady(Result<(Gpu, wgpu::Surface<'static>), String>),
    /// Close as if the window were closed (a signal, say).
    Exit,
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
    /// When the next tick is due; `None` until the GPU is ready, and once a replay has run out.
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
            Input::Live {
                header,
                keys,
                gamepads,
                record,
            } => Source::Live {
                header,
                collector: Collector::new(keys),
                gamepads,
                record,
            },
            Input::Replay(replay) => Source::Replay(replay),
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

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: String) {
        // The web's event loop never returns to its caller, so the error goes on the page instead.
        #[cfg(target_arch = "wasm32")]
        crate::web::show_error(&error);
        self.error.get_or_insert(error);
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

    /// The next tick's events (recording them when live), or `None` once a replay has run out. The first
    /// call settles the run's header and starts the game and the recording.
    fn next_events(&mut self) -> Result<Option<Vec<Event>>, String> {
        let frame = self.ticks;
        let cursor = self.cursor_in_screen();
        match &mut self.source {
            Source::Live {
                header,
                collector,
                gamepads,
                record,
            } => {
                if frame == 0 {
                    if gamepads.as_mut().is_some_and(Gamepads::connected) {
                        header.joystick = Some(PAD_JOYSTICK);
                    }
                    collector.set_joystick(header.joystick.is_some());
                    self.game.begin(header);
                    if let Some(r) = record {
                        r.begin(header)?;
                    }
                }
                if let Some(g) = gamepads {
                    g.drain(collector);
                }
                if let Some((x, y)) = cursor {
                    collector.cursor(x, y);
                }
                let events = collector.take_tick();
                if let Some(r) = record {
                    r.frame(frame, &events)?;
                }
                Ok(Some(events))
            }
            Source::Replay(replay) => {
                if frame == 0 {
                    self.game.begin(&replay.header);
                }
                Ok((frame < replay.len()).then(|| replay.frame_events(frame).collect()))
            }
        }
    }

    /// Runs every tick that's due, rendering each one's frame. Returns when the next one is due; `None`
    /// before the GPU is ready and once the run is over.
    fn run_due_ticks(&mut self, event_loop: &ActiveEventLoop) -> Option<Instant> {
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
                    // The replay is over. Show its last frame now, not on a redraw that a minimised window
                    // never gets, and close.
                    if let Some(g) = &mut self.graphics {
                        g.presenter.present(&g.renderer);
                    }
                    self.next_tick = None;
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
            Source::Replay(_) => None,
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

        #[cfg(not(target_arch = "wasm32"))]
        {
            let instance = gpu::instance();
            let surface = match instance.create_surface(window) {
                Ok(s) => s,
                Err(e) => return self.fail(event_loop, surface_error(e)),
            };
            match pollster::block_on(gpu::request(instance, Some(&surface))) {
                Ok(gpu) => self.gpu_ready(event_loop, gpu, surface),
                Err(e) => self.fail(event_loop, e),
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowExtWebSys;
            // Keys go to the focused element: start with the canvas, not the page.
            if let Some(canvas) = window.canvas() {
                let _ = canvas.focus();
            }
            let proxy = self.proxy.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let instance = gpu::web_instance().await;
                let result = match instance.create_surface(window) {
                    Ok(surface) => gpu::request(instance, Some(&surface))
                        .await
                        .map(|gpu| (gpu, surface)),
                    Err(e) => Err(surface_error(e)),
                };
                let _ = proxy.send_event(UserEvent::GpuReady(result));
            });
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::GpuReady(Ok((gpu, surface))) => self.gpu_ready(event_loop, gpu, surface),
            UserEvent::GpuReady(Err(e)) => self.fail(event_loop, e),
            UserEvent::Exit => event_loop.exit(),
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
                    ref key_event @ KeyEvent {
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
                let typed = typed(key_event);
                if let Some(c) = self.collector() {
                    c.key(key, &typed, down);
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
                if let Some(c) = self.collector() {
                    c.wheel(dy);
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

    /// Closes the recording however the loop ends (the window, a signal, an error, the OS quitting the
    /// app), since `run_app` may not return to the caller.
    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        let ticks = self.ticks;
        if let Source::Live {
            record: Some(r), ..
        } = &mut self.source
            && ticks > 0
            && let Err(e) = r.finish(ticks)
        {
            self.error.get_or_insert(e);
        }
    }
}

fn surface_error(e: wgpu::CreateSurfaceError) -> String {
    format!("can't draw to the window: {e}")
}

/// What the layout types on a key without modifiers, for [`KeyMap::Layout`]. The web only reports what it
/// types with them, which for a shifted key is usually a symbol that falls back to the physical key.
fn typed(e: &KeyEvent) -> Key {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
        e.key_without_modifiers()
    }
    #[cfg(target_arch = "wasm32")]
    {
        e.logical_key.clone()
    }
}
