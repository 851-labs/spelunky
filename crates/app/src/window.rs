//! The window and its event loop: ticks the game at a fixed 30 Hz, renders each tick's frame once, and
//! presents the latest frame at display rate.
//!
//! Written against winit's `ApplicationHandler` so the same code runs on the web, where nothing may block:
//! GPU setup is a future that native code blocks on and the web build spawns, reporting back through a
//! user event.

use crate::gpu::{self, Gpu};
use crate::present::Presenter;
use crate::render::Renderer;
use crate::textures::Textures;
use draw::Frame;
use std::collections::HashSet;
use std::sync::Arc;
use web_time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, KeyEvent, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Fullscreen, Window, WindowId};

/// GM's room speed: every Spelunky room runs at 30 steps a second.
pub const TICK: Duration = Duration::from_nanos(1_000_000_000 / 30);

/// Ticks to run at most per redraw before dropping the backlog (after a stall, don't fast-forward).
const MAX_CATCH_UP: u32 = 4;

/// Keyboard state for one tick.
#[derive(Debug, Default, Clone)]
pub struct Input {
    pub held: HashSet<KeyCode>,
    /// Went down since the previous tick.
    pub pressed: HashSet<KeyCode>,
}

impl Input {
    pub fn is_held(&self, key: KeyCode) -> bool {
        self.held.contains(&key)
    }
}

/// What the window drives: anything that turns input into one frame per tick.
pub trait Game {
    fn tick(&mut self, input: &Input, out: &mut Frame);
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
    /// Handed to the renderer once the GPU is ready.
    textures: Option<Textures>,
    window: Option<Arc<Window>>,
    graphics: Option<Graphics>,
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    proxy: EventLoopProxy<UserEvent>,
    input: Input,
    frame: Frame,
    last: Option<Instant>,
    backlog: Duration,
    /// Presenting failed (minimised or hidden): wait a tick instead of spinning.
    idle: bool,
    error: Option<String>,
}

impl<G: Game> App<G> {
    pub fn new(
        game: G,
        textures: Textures,
        title: &str,
        proxy: EventLoopProxy<UserEvent>,
    ) -> App<G> {
        App {
            game,
            title: title.to_owned(),
            textures: Some(textures),
            window: None,
            graphics: None,
            proxy,
            input: Input::default(),
            frame: Frame::default(),
            last: None,
            backlog: Duration::ZERO,
            idle: false,
            error: None,
        }
    }

    /// The error that ended the event loop, if any.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
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
        window.request_redraw();
    }

    fn redraw(&mut self) {
        let Some(graphics) = &mut self.graphics else {
            return;
        };
        let now = Instant::now();
        // The first redraw runs a tick straight away.
        let elapsed = self.last.map_or(TICK, |last| now - last);
        self.last = Some(now);
        self.backlog += elapsed;
        let mut ticks = 0;
        while self.backlog >= TICK {
            if ticks == MAX_CATCH_UP {
                self.backlog = Duration::ZERO;
                break;
            }
            self.backlog -= TICK;
            self.game.tick(&self.input, &mut self.frame);
            self.input.pressed.clear();
            // Every tick's frame is rendered, even ones never shown, because surfaces carry state.
            graphics.renderer.render(&self.frame);
            ticks += 1;
        }
        let shown = graphics.presenter.present(&graphics.renderer);
        self.idle = !shown;
        if shown && let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

impl<G: Game> ApplicationHandler<UserEvent> for App<G> {
    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
        if matches!(cause, StartCause::ResumeTimeReached { .. })
            && let Some(window) = &self.window
        {
            self.idle = false;
            window.request_redraw();
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title(&self.title)
            .with_inner_size(LogicalSize::new(640, 480))
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
            WindowEvent::RedrawRequested => self.redraw(),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state,
                        repeat,
                        ..
                    },
                ..
            } => match state {
                ElementState::Pressed => {
                    // F4 toggles fullscreen, as in the original's game settings.
                    if key == KeyCode::F4
                        && !repeat
                        && let Some(window) = &self.window
                    {
                        let full = window.fullscreen().is_none();
                        window.set_fullscreen(full.then_some(Fullscreen::Borderless(None)));
                    }
                    if !repeat {
                        self.input.pressed.insert(key);
                    }
                    self.input.held.insert(key);
                }
                ElementState::Released => {
                    self.input.held.remove(&key);
                }
            },
            WindowEvent::Focused(false) => self.input.held.clear(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.idle {
            event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + TICK));
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }
}
