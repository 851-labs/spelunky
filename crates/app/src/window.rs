//! The window and its event loop: ticks the game at a fixed 30 Hz, renders each tick's frame once (an
//! embedded replay's unshown ones only as far as their pixels last, `crate::unrendered`), and presents the
//! latest frame at display rate.
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
use crate::notice::Notice;
use crate::present::{Presenter, window_to_content};
use crate::render::{Renderer, Surfaces};
use crate::textures::Textures;
use crate::unrendered::Unrendered;
use draw::{Command, Frame};
use std::any::Any;
use std::collections::BTreeMap;
use std::sync::Arc;
use trace::replay::{Event, MouseButton, Replay, ReplayHeader};
use web_time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, KeyEvent, MouseScrollDelta, Touch, TouchPhase, WindowEvent};
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

/// How long an embedded replay runs its unshown frames back to back before letting the page have a turn
/// ([`Embed::from`]): the page stays responsive while the clip's start comes up.
const SKIP_SLICE: Duration = Duration::from_millis(50);

/// How often an embedded replay keeps a savestate as it first plays through, in frames from the clip's
/// first ([`Embed::from`]): a seek restores the latest one before where it goes and runs the frames after it
/// unshown, at most this many, a second's. Restart, a seek to the first frame, is a restore and one frame.
const SAVESTATE_EVERY: u64 = 30;

/// The most savestates an embedded replay keeps, each a few hundred KB, and as much again on the GPU for each
/// surface the game drew into since the one before (Spelunky's frame, and on a dark level its darkness):
/// past them, every other one goes and they're kept half as often ([`Savestates::insert`]), so a long replay
/// seeks a little slower rather than filling the page's memory. A scene's clip is seconds long, well within
/// them.
const MAX_SAVESTATES: usize = 64;

/// What the window drives: anything that turns input into one frame per tick.
pub trait Game {
    /// Called once, before the first tick, with the run's replay header. A live session settles its header
    /// (whether it has a joystick) only then, when the platform has had time to report gamepads. What
    /// booting draws goes in `out` (nothing, if it stays empty), rendered before the first tick's.
    fn begin(&mut self, _header: &ReplayHeader, _out: &mut Frame) -> Result<(), String> {
        Ok(())
    }

    /// Runs one tick with its input events, in order.
    fn tick(&mut self, events: &[Event], out: &mut Frame) -> Result<(), String>;

    /// Whether the run is over (the game called `game_end`): the window closes.
    fn ended(&self) -> bool {
        false
    }

    /// Whether the game wants the window fullscreen, if it decides (GM's `action_fullscreen`).
    fn fullscreen(&self) -> Option<bool> {
        None
    }

    /// Whether the window takes the size of the picture the game shows, as GM's window does.
    fn sizes_window(&self) -> bool {
        false
    }

    /// The player pressed a key or button: a user gesture, which the browser needs before it plays
    /// sound. A key's, button's or tap's is called in the browser event's handler, where Safari wants
    /// it; the embedding page's controls' and the clip's first frame's aren't (`Embed`).
    fn user_gesture(&mut self) {}

    /// What to show from now on, after a tick that switched programs (the settings program starting
    /// the game): taken after each tick, before its frame is rendered.
    fn take_switch(&mut self) -> Option<Switch> {
        None
    }

    /// Whether the player is somewhere the app may interrupt with something of its own (the updater's
    /// prompt): between runs, not in one.
    fn interruptible(&self) -> bool {
        false
    }

    /// A savestate, to [`restore`](Self::restore) when the page embedding a replay seeks back to it: `None`
    /// if the game can't make one.
    fn savestate(&self) -> Option<Box<dyn Any>> {
        None
    }

    /// Goes back to `state`, one of this game's [`savestate`](Self::savestate)s, which stays as it is. Only
    /// while the ticks are [`unshown`](Self::unshown); at least one runs before the next is shown.
    fn restore(&mut self, _state: &dyn Any) {}

    /// The ticks from now on run without being shown (an embedded replay's way to the frame it starts or
    /// seeks to, [`Embed`]), or no longer: their sounds needn't play, and when they're over the sound
    /// picks up from what the game has playing.
    fn unshown(&mut self, _unshown: bool) {}

    /// The game's state after the last tick, hashed (`gm::World::frame_hash`), if it has one: an embedded
    /// replay reports it at the clip's first frame and through its frames ([`Playback::state`],
    /// [`Playback::states`]).
    fn frame_hash(&self) -> Option<u64> {
        None
    }
}

/// The app's own layer over the game: natively, the updater's menu and prompts (`crate::update`). Like the
/// notice it's drawn over the picture, never into the game, and the keys it takes never reach the game.
/// The frontend only gives one to a session of play, never to a replay or a recording, whose input must be
/// all the game's.
pub trait Overlay {
    /// The window is up and has shown its first frame.
    fn started(&mut self) {}

    /// After each tick, with [`Game::interruptible`]. Returns whether to close the window: to restart into
    /// an update.
    fn tick(&mut self, interruptible: bool) -> bool;

    /// What to draw over the shown picture, `size` its size, if anything.
    fn commands(&self, size: (u32, u32)) -> Option<Vec<Command>>;

    /// A key went down (not a repeat): whether the overlay takes it from the game (its repeats and its
    /// release then go nowhere).
    fn key_down(&mut self, key: KeyCode) -> bool;

    /// Whether it's showing something that takes the mouse buttons from the game too.
    fn modal(&self) -> bool;
}

/// Another program's images, its notice (shown from the switch on, for its first seconds), and the
/// window size it opens with, in logical pixels, if the window doesn't follow the program's picture
/// ([`Game::sizes_window`]).
pub struct Switch {
    pub textures: Textures,
    pub notice: Option<Notice>,
    pub window_size: Option<(u32, u32)>,
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

impl Input {
    /// A live session set up as `header` says (its joystick is settled at the first tick), with the
    /// gamepads unless `gamepad` is off.
    pub fn live(
        header: ReplayHeader,
        keys: KeyMap,
        gamepad: bool,
        record: Option<Box<dyn RecordSink>>,
    ) -> Input {
        Input::Live {
            header,
            keys,
            // Whether one is connected is settled at the first tick, without waiting here.
            gamepads: gamepad.then(Gamepads::new).flatten(),
            record,
        }
    }
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
    /// The page embedding the app asks something of it ([`App::with_embed`]).
    Control(Control),
}

/// What the page that embeds a replay (the wiki's live clips) asks of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    Pause,
    Play,
    /// Hands the game from the replay to the reader's keyboard, mouse and gamepad, where it's got to.
    TakeControl,
    /// Shows this frame of the clip ([`Playback::frame`]), paused or playing as it was, handing the game
    /// back to the replay if the reader has it.
    Seek(u64),
}

/// A replay a page drives ([`App::with_embed`]): it starts at a frame, pauses, plays and seeks when asked,
/// holds its last frame instead of closing, and hands the game to the reader on [`Control::TakeControl`].
pub struct Embed {
    /// The first frame shown. The ones before it run as fast as they go, unshown: they're how the game
    /// gets there, as a clip's video starts at its spec's `from`. So do the clip's own, once, before it
    /// goes back to show them ([`Embedded::through`]).
    pub from: u64,
    /// The reader's input once they take control: how keys map, and whether gamepads are read.
    pub keys: KeyMap,
    pub gamepad: bool,
    /// Told whenever what the page shows of the replay changes.
    pub report: Box<dyn FnMut(Playback)>,
}

/// Where an embedded replay is, for the page's controls.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Playback {
    /// The first frame ([`Embed::from`]) is showing: the page can show the replay instead of its poster.
    pub ready: bool,
    pub paused: bool,
    /// The reader has the game.
    pub live: bool,
    /// The replay ran out: it holds its last frame, which the reader can take over from.
    pub ended: bool,
    /// The clip's frame showing, from 0 at [`Embed::from`] (or being sought, [`Control::Seek`]); past the
    /// replay's end once the reader has the game.
    pub frame: u64,
    /// The clip's frames: the replay's from [`Embed::from`] on.
    pub frames: u64,
    /// The replay's frames run unshown before the clip's first frame shows: the ones before [`Embed::from`],
    /// then the clip's own, which it runs through first ([`Embedded::through`]). How far the run has got, as
    /// it goes, for the page to show over its poster: the replay's length once the clip's first frame shows,
    /// after which it stays.
    pub lead: u64,
    /// The game's state at the clip's first frame ([`Game::frame_hash`]), once the replay has run it: the
    /// page checks it's what the clip's video was rendered from, as this build could play it otherwise.
    pub state: Option<u64>,
    /// The game's states at each of the clip's frames, first to last, as one hash
    /// (`trace::canon::hash_frames`), once the replay has run them through, which it does before showing the
    /// first: the page checks them too, as a build could play the clip's own frames otherwise from the same
    /// first state.
    pub states: Option<u64>,
}

/// An embedded replay, as the app drives it.
struct Embedded {
    embed: Embed,
    /// What the page was last told.
    told: Playback,
    /// The replay's frames.
    length: u64,
    /// The first frame to show: past the replay's last while it runs the clip through ([`through`]), then
    /// [`Embed::from`], then wherever the page seeks. The ones before it run unshown.
    ///
    /// [`through`]: Self::through
    seek: u64,
    /// The clip's frame the page last asked for, until the next ticks run: of a drag's many seeks, only the
    /// last restores a savestate.
    sought: Option<u64>,
    savestates: Savestates,
    /// The replay, while the reader has the game: a seek hands the game back to it.
    replay: Option<Replay>,
    /// The game's states at the clip's frames run so far ([`Game::frame_hash`]), while the replay runs the
    /// clip through unshown before showing its first frame, keeping savestates as it goes: then the page has
    /// its states ([`Playback::states`]) before the reader sees any of it, and can go back to the video
    /// first. A few hundred frames at most, a fraction of a second on top of the frames before the clip.
    /// `None` once it has, when it seeks back to the first frame.
    through: Option<Vec<u64>>,
    /// [`Playback::state`].
    state: Option<u64>,
    /// [`Playback::states`].
    states: Option<u64>,
}

/// An embedded replay's savestates ([`Game::savestate`]), by the frame each was taken before running.
struct Savestates {
    /// The clip's first frame, [`Embed::from`]: the first savestate's, and what the others count from.
    from: u64,
    /// One every this many frames: [`SAVESTATE_EVERY`], doubled each time there are too many.
    every: u64,
    states: BTreeMap<u64, Kept>,
}

/// A savestate, and GM's surfaces from then: the game redraws some only now and then (Spelunky's pause
/// screen, when it pauses), so a restore that left them blank would show them so.
struct Kept {
    state: Box<dyn Any>,
    /// `None` without a renderer: they start blank.
    surfaces: Option<Surfaces>,
}

impl Savestates {
    fn new(from: u64) -> Savestates {
        Savestates {
            from,
            every: SAVESTATE_EVERY,
            states: BTreeMap::new(),
        }
    }

    /// Whether to keep a savestate before running `frame`: one every [`every`](Self::every) frames from the
    /// clip's first, as the replay first gets to them. None before it, as a seek never goes there.
    fn wants(&self, frame: u64) -> bool {
        frame >= self.from
            && (frame - self.from).is_multiple_of(self.every)
            && !self.states.contains_key(&frame)
    }

    /// Keeps `state`, from before `frame`. Past [`MAX_SAVESTATES`], every other one goes, and the rest are
    /// kept twice as far apart.
    fn insert(&mut self, frame: u64, kept: Kept) {
        self.states.insert(frame, kept);
        if self.states.len() > MAX_SAVESTATES {
            self.every *= 2;
            let (from, every) = (self.from, self.every);
            self.states.retain(|&f, _| (f - from).is_multiple_of(every));
        }
    }

    /// Where to run to frame `target` from, with the game before frame `at` of the replay (`None` if it's
    /// not the replay's): from there if it's on the way and no savestate is nearer, else from the latest
    /// savestate at or before `target`.
    fn start(&self, target: u64, at: Option<u64>) -> Start<'_> {
        let latest = self.states.range(..=target).next_back();
        match (at, latest) {
            (Some(at), Some((&frame, _))) if (frame..=target).contains(&at) => Start::Here,
            (Some(at), None) if at <= target => Start::Here,
            (_, Some((&frame, kept))) => Start::From(frame, kept),
            (_, None) => Start::Nowhere,
        }
    }
}

/// Where a seek runs from ([`Savestates::start`]).
enum Start<'a> {
    /// Where the game is.
    Here,
    /// A savestate, taken before this frame.
    From(u64, &'a Kept),
    /// Nowhere: the game is past the frame, and has no savestate before it.
    Nowhere,
}

struct Graphics {
    renderer: Renderer,
    presenter: Presenter,
    /// An embedded replay's unshown frames not rendered yet: only the ones whose pixels last until the
    /// renderer's are next needed get rendered (`crate::unrendered`).
    unrendered: Unrendered,
}

impl Graphics {
    /// Renders the frames left unrendered that need it, so the renderer's pixels are as if it had rendered
    /// every frame.
    fn catch_up(&mut self) {
        let renderer = &mut self.renderer;
        self.unrendered.flush(|frame| renderer.render(frame));
    }
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
    /// The app's overlay over the first seconds of the game's picture.
    notice: Option<Notice>,
    /// The tick the notice's seconds count from: 0, or the last [`Switch`].
    notice_from: u64,
    /// The picture size the window was last fitted to, for [`Game::sizes_window`].
    fitted: Option<(u32, u32)>,
    /// The fullscreen state the game last asked for.
    fullscreen: bool,
    /// The app's layer over the game, if it has one.
    overlay: Option<Box<dyn Overlay>>,
    /// Keys whose press the overlay took, so the game doesn't see their release either.
    taken: Vec<KeyCode>,
    taken_buttons: Vec<MouseButton>,
    /// The first frame has been shown.
    started: bool,
    stats: Stats,
    /// The page driving the replay, if it's embedded.
    embed: Option<Embedded>,
    /// Paused by the page: no ticks run.
    paused: bool,
    /// An embedded replay ran out and holds its last frame.
    held: bool,
    /// What the game was last told of whether the ticks are shown ([`Game::unshown`]).
    unshown: bool,
}

/// Tick timing, with `SPELUNKY_STATS=1` (natively): the tick rate and how long ticks take.
#[derive(Default)]
struct Stats {
    enabled: bool,
    since: Option<Instant>,
    ticks: u32,
    busy: Duration,
    worst: Duration,
}

impl Stats {
    fn new() -> Stats {
        #[cfg(not(target_arch = "wasm32"))]
        let enabled = std::env::var_os("SPELUNKY_STATS").is_some_and(|v| v != "0");
        #[cfg(target_arch = "wasm32")]
        let enabled = false;
        Stats {
            enabled,
            ..Stats::default()
        }
    }

    /// One tick took `busy` (stepping and rendering). Every 5 seconds, prints the rate and times.
    fn tick(&mut self, busy: Duration) {
        if !self.enabled {
            return;
        }
        let now = Instant::now();
        let since = *self.since.get_or_insert(now);
        self.ticks += 1;
        self.busy += busy;
        self.worst = self.worst.max(busy);
        let elapsed = now - since;
        if elapsed >= Duration::from_secs(5) {
            println!(
                "stats: {:.2} ticks/s, tick work {:.2} ms average, {:.2} ms worst",
                f64::from(self.ticks) / elapsed.as_secs_f64(),
                self.busy.as_secs_f64() * 1000.0 / f64::from(self.ticks),
                self.worst.as_secs_f64() * 1000.0
            );
            *self = Stats {
                enabled: true,
                since: Some(now),
                ..Stats::default()
            };
        }
    }
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
            notice: None,
            notice_from: 0,
            fitted: None,
            fullscreen: false,
            overlay: None,
            taken: Vec::new(),
            taken_buttons: Vec::new(),
            started: false,
            stats: Stats::new(),
            embed: None,
            paused: false,
            held: false,
            unshown: false,
        }
    }

    /// Lets a page drive the replay ([`Embed`]). Only a replay can be embedded; live input ignores it.
    /// The page has already heard the default (`web::run` says it as it starts).
    pub fn with_embed(mut self, embed: Embed) -> App<G> {
        if let Source::Replay(replay) = &self.source {
            self.embed = Some(Embedded {
                // The clip's frames run through to the replay's end before the first shows.
                seek: replay.len().max(embed.from),
                savestates: Savestates::new(embed.from),
                embed,
                told: Playback::default(),
                length: replay.len(),
                sought: None,
                replay: None,
                through: Some(Vec::new()),
                state: None,
                states: None,
            });
        }
        self
    }

    /// Draws `notice` over the game's picture for its first seconds.
    pub fn with_notice(mut self, notice: Option<Notice>) -> App<G> {
        self.notice = notice;
        self
    }

    /// Puts `overlay` over the game ([`Overlay`]).
    pub fn with_overlay(mut self, overlay: Option<Box<dyn Overlay>>) -> App<G> {
        self.overlay = overlay;
        self
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
        if self.error.is_none() {
            // The web's event loop never returns to its caller, so the error goes on the page instead.
            #[cfg(target_arch = "wasm32")]
            crate::web::show_error(&error);
            self.error = Some(error);
        }
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
            unrendered: Unrendered::new(),
        });
        // The first tick runs straight away.
        self.next_tick = Some(Instant::now());
    }

    /// The cursor in GM window pixels: the back buffer's, which the window shows scaled and centred.
    fn cursor_in_screen(&self) -> Option<(i32, i32)> {
        let pos = self.cursor?;
        let (screen, _) = self.graphics.as_ref()?.renderer.screen()?;
        let size = self.window.as_ref()?.inner_size();
        window_to_content(
            (pos.x, pos.y),
            (screen.width(), screen.height()),
            (size.width, size.height),
        )
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
                    self.game.begin(header, &mut self.frame)?;
                    render_boot(self.graphics.as_mut(), &self.frame);
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
                    self.game.begin(&replay.header, &mut self.frame)?;
                    render_boot(self.graphics.as_mut(), &self.frame);
                }
                Ok((frame < replay.len()).then(|| replay.frame_events(frame).collect()))
            }
        }
    }

    /// Runs every tick that's due, rendering each one's frame. Returns when the next one is due; `None`
    /// before the GPU is ready and once the run is over.
    fn run_due_ticks(&mut self, event_loop: &ActiveEventLoop) -> Option<Instant> {
        if let Some(frame) = self.embed.as_mut().and_then(|e| e.sought.take()) {
            self.seek(frame);
        }
        // Paused, nothing runs but the way to a frame the page seeks to.
        if self.paused && !self.seeking() {
            return None;
        }
        let mut next = self.next_tick?;
        let now = Instant::now();
        let mut ticks = 0;
        loop {
            let skipping = self.skipping();
            if skipping {
                if now.elapsed() >= SKIP_SLICE {
                    next = Instant::now();
                    break;
                }
            } else if self.paused {
                // The frame sought, straight away: there's no time to keep.
                if !self.seeking() {
                    break;
                }
            } else if next > now {
                break;
            } else if ticks == MAX_CATCH_UP {
                next = now + TICK;
                break;
            }
            self.set_unshown(skipping);
            self.keep_savestate();
            let started = Instant::now();
            // A game that ended takes no more input, so a recording doesn't get any either.
            let events = if self.game.ended() {
                Ok(None)
            } else {
                self.next_events()
            };
            let events = match events {
                Ok(Some(events)) => events,
                Ok(None) => {
                    // The replay is over, or the game. Show the last frame now, not on a redraw that a
                    // minimised window never gets, and close.
                    if let Some(g) = &mut self.graphics {
                        g.catch_up();
                        g.presenter.present(&g.renderer);
                    }
                    // Unless the page embeds it: it holds the frame, for the reader to take over from or seek
                    // back.
                    if self.embed.is_some() && !self.game.ended() {
                        self.held = true;
                        self.paused = true;
                        self.report();
                        return None;
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
            let ticked = self.game.tick(&events, &mut self.frame);
            // The failing tick counts, so a recording ends after its input and replays the failure.
            self.ticks += 1;
            if let Err(e) = ticked {
                self.fail(event_loop, e);
                return None;
            }
            // The clip's frames, as the replay runs them through: what the page checks its video shows.
            let ran = self.ticks - 1;
            if let Some(e) = &mut self.embed
                && let Some(hashes) = &mut e.through
                && ran >= e.embed.from
                && matches!(self.source, Source::Replay(_))
            {
                let hash = self.game.frame_hash();
                if ran == e.embed.from {
                    e.state = hash;
                }
                hashes.extend(hash);
                // The clip's last: back to its first, for the reader, once this frame's done. A game
                // without savestates (none the app embeds) can't go back, and holds this one.
                if ran + 1 == e.length {
                    if hashes.len() as u64 == e.length - e.embed.from {
                        e.states = Some(trace::canon::hash_frames(hashes));
                    }
                    e.through = None;
                    e.sought = Some(0);
                }
            }
            let g = self.graphics.as_mut()?;
            if let Some(switch) = self.game.take_switch() {
                // The old program's frames, as they were drawn with its images, though the renderer keeps
                // none of their pixels.
                g.catch_up();
                g.unrendered = Unrendered::new();
                g.renderer.set_textures(switch.textures);
                self.notice = switch.notice;
                self.notice_from = self.ticks - 1;
                self.fitted = None;
                if let (Some(window), Some((w, h))) = (&self.window, switch.window_size) {
                    let _ = window.request_inner_size(LogicalSize::new(w, h));
                }
            }
            // An unshown frame is left unrendered, until a frame shown, a savestate or a seek needs the
            // pixels that last of it: surfaces carry state, but most of what a frame draws, the next
            // clears. Unless the app draws over its picture, which only the picture rendered has.
            let notice_tick = self.ticks - 1 - self.notice_from;
            let drawn_over = self.overlay.is_some()
                || self.notice.as_ref().is_some_and(|n| n.shows(notice_tick));
            if skipping && !drawn_over {
                let renderer = &mut g.renderer;
                g.unrendered.push(std::mem::take(&mut self.frame), |frame| {
                    renderer.render(frame)
                });
            } else {
                g.catch_up();
                g.renderer.render(&self.frame);
            }
            // Nothing draws over a frame left unrendered.
            if let Some(size) = g.renderer.shown_size() {
                // One pass for both: the renderer only draws over a picture once.
                let mut over = Vec::new();
                if let Some(notice) = &self.notice {
                    over.extend(notice.commands(notice_tick, size).into_iter().flatten());
                }
                if let Some(overlay) = &mut self.overlay {
                    if overlay.tick(self.game.interruptible()) {
                        self.next_tick = None;
                        event_loop.exit();
                        return None;
                    }
                    over.extend(overlay.commands(size).into_iter().flatten());
                }
                if !over.is_empty() {
                    g.renderer.draw_over_shown(&over);
                }
            }
            self.stats.tick(started.elapsed());
            // Unshown frames don't keep time: the first shown one is a tick after the last of them.
            next = if skipping {
                Instant::now() + TICK
            } else {
                next + TICK
            };
            ticks += 1;
        }
        self.next_tick = Some(next);
        if ticks > 0 {
            self.follow_game();
            if let Some(window) = &self.window {
                window.request_redraw();
            }
            if !std::mem::replace(&mut self.started, true)
                && let Some(overlay) = &mut self.overlay
            {
                overlay.started();
            }
            self.report();
        }
        (!self.paused || self.seeking()).then_some(next)
    }

    /// Whether the next tick is one of an embedded replay's unshown frames, before [`Embed::from`] or the
    /// frame the page seeks to, or the clip's own as it first runs them through ([`Embedded::through`]).
    fn skipping(&self) -> bool {
        self.embed.as_ref().is_some_and(|e| self.ticks < e.seek)
    }

    /// Whether an embedded replay has yet to show the frame it starts or seeks to.
    fn seeking(&self) -> bool {
        !self.held && self.embed.as_ref().is_some_and(|e| self.ticks <= e.seek)
    }

    /// Whether the window may show the latest frame: not one of the unshown ones, which would flash the
    /// boot before the clip's start, or the frames on the way to one sought. A held last frame shows, even
    /// one before it.
    fn showing(&self) -> bool {
        self.held || self.embed.as_ref().is_none_or(|e| self.ticks > e.seek)
    }

    /// Tells the game whether the ticks from now on are shown, if that's changed ([`Game::unshown`]).
    fn set_unshown(&mut self, unshown: bool) {
        if unshown != self.unshown {
            self.unshown = unshown;
            self.game.unshown(unshown);
        }
    }

    /// Keeps a savestate before the next tick, if an embedded replay wants one there ([`Savestates`]).
    fn keep_savestate(&mut self) {
        let Some(e) = &mut self.embed else { return };
        if matches!(self.source, Source::Replay(_))
            && self.ticks < e.length
            && e.savestates.wants(self.ticks)
            && let Some(state) = self.game.savestate()
        {
            // As the last frame left them, as the state is from before the next.
            let surfaces = self.graphics.as_mut().map(|g| {
                g.catch_up();
                g.renderer.surfaces()
            });
            e.savestates.insert(self.ticks, Kept { state, surfaces });
        }
    }

    /// Tells the embedding page where the replay is, if that's changed.
    fn report(&mut self) {
        let showing = self.showing();
        let live = matches!(self.source, Source::Live { .. });
        let Some(e) = &mut self.embed else {
            return;
        };
        // The frame sought until it shows, so that the page's slider stays where the reader left it. The
        // reader's game runs on past the replay's end, where the slider stays, and the page needn't hear
        // of each frame.
        // Until the clip has started, it's at its first, whatever runs through.
        let ready = e.told.ready || showing;
        let frame = if showing {
            self.ticks.saturating_sub(1)
        } else if !ready {
            e.embed.from
        } else {
            e.seek
        }
        .min(e.length.saturating_sub(1));
        let now = Playback {
            // Once the clip has started, the frame it shows stays while a seek runs.
            ready,
            paused: self.paused,
            live,
            ended: self.held,
            frame: frame.saturating_sub(e.embed.from),
            frames: e.length.saturating_sub(e.embed.from),
            lead: if ready { e.length } else { self.ticks }.min(e.length),
            state: e.state,
            states: e.states,
        };
        if e.told != now {
            // The clip's first frame: the reader's click on the page's button is the gesture browsers want
            // before sound (the page lets the frame play it; Safari doesn't, and the sound waits for the
            // reader's next: `Play::play_sounds`), and the sound starts from what the game has playing now,
            // not from the unshown frames' every sound at once.
            if now.ready && !e.told.ready {
                self.game.user_gesture();
            }
            e.told = now;
            (e.embed.report)(now);
        }
    }

    /// Shows the clip's `frame` ([`Control::Seek`]): from the savestate nearest before it, or from where
    /// the game is if that's on the way, running the frames between unshown.
    fn seek(&mut self, frame: u64) {
        let Some(e) = &mut self.embed else { return };
        // The replay's last frame at the latest, which it then holds; its first shown at the earliest.
        let target = e
            .embed
            .from
            .saturating_add(frame)
            .min(e.length.saturating_sub(1))
            .max(e.embed.from);
        let at = (matches!(self.source, Source::Replay(_)) && !self.held).then_some(self.ticks);
        match e.savestates.start(target, at) {
            Start::Here => {}
            Start::From(from, kept) => {
                // The sound follows once the frame sought shows, not from the savestate first.
                self.unshown = true;
                self.game.unshown(true);
                self.game.restore(kept.state.as_ref());
                self.ticks = from;
                self.held = false;
                // GM's surfaces aren't the game's to restore, but the renderer's, kept beside it. Without
                // them they start blank, as after a savestate loaded from disk, rather than with a later
                // frame's pixels. The back buffer and the shown picture aren't restored: they stay as the
                // frames run so far left them, which are rendered as far as that needs first.
                if let Some(g) = &mut self.graphics {
                    g.catch_up();
                    match &kept.surfaces {
                        Some(surfaces) => g.renderer.restore_surfaces(surfaces),
                        None => g.renderer.forget_surfaces(),
                    }
                    // What it followed of the surfaces is gone with them.
                    g.unrendered = Unrendered::new();
                }
            }
            Start::Nowhere => return,
        }
        e.seek = target;
        // The reader's game, if they have it, goes: the replay has it again.
        if let Some(replay) = e.replay.take() {
            self.source = Source::Replay(replay);
        }
        if self.next_tick.is_some() {
            self.next_tick = Some(Instant::now());
        }
    }

    /// Does what the embedding page asks.
    fn control(&mut self, control: Control) {
        if self.embed.is_none() {
            return;
        }
        // The reader's click on the page, again (as at the clip's first frame), once it's showing.
        if self.showing() {
            self.game.user_gesture();
        }
        match control {
            Control::Pause => self.paused = true,
            // A held last frame has nothing to play on to, until the reader takes over.
            Control::Play => self.paused = self.held,
            // Not while the replay runs the clip through, which it would cut short: the page's controls wait
            // for its first frame anyway.
            Control::Seek(_) if self.embed.as_ref().is_some_and(|e| e.through.is_some()) => {}
            // Done when the ticks next run, after whatever else the page has sent by then.
            Control::Seek(frame) => {
                if let Some(e) = &mut self.embed {
                    e.sought = Some(frame);
                }
            }
            // Not while the frames before the clip's first are running, or a seek's: the reader's input
            // would run as fast as they do, unseen.
            Control::TakeControl if self.skipping() => {}
            Control::TakeControl => {
                if let (Source::Replay(replay), Some(e)) = (&self.source, &mut self.embed) {
                    // The input as the game knows it after the last frame run, which the reader's
                    // devices start from.
                    let state = self
                        .ticks
                        .checked_sub(1)
                        .and_then(|last| replay.inputs().nth(last as usize))
                        .map(|input| input.state)
                        .unwrap_or_default();
                    let header = replay.header.clone();
                    let joystick = header.joystick.is_some();
                    let live = Source::Live {
                        header,
                        collector: Collector::taking_over(e.embed.keys, joystick, state),
                        gamepads: e.embed.gamepad.then(Gamepads::new).flatten(),
                        record: None,
                    };
                    // Kept for a seek, which hands the game back to it.
                    if let Source::Replay(replay) = std::mem::replace(&mut self.source, live) {
                        e.replay = Some(replay);
                    }
                }
                self.paused = false;
                self.held = false;
                // Its keys go to the focused element, which the page left on itself until now.
                #[cfg(target_arch = "wasm32")]
                if let Some(window) = &self.window {
                    use winit::platform::web::WindowExtWebSys;
                    if let Some(canvas) = window.canvas() {
                        let _ = canvas.focus();
                    }
                }
            }
        }
        // Time starts again from now, not from when it paused.
        if !self.paused && self.next_tick.is_some() {
            self.next_tick = Some(Instant::now());
        }
        self.report();
    }

    /// Gives the window the size of the picture the game shows and the fullscreen state it asks for, when
    /// they change, as GM's runner does.
    fn follow_game(&mut self) {
        let Some(window) = &self.window else { return };
        // Not to a picture not shown yet, which the renderer may not have drawn (an embedded replay's
        // unshown frames).
        if self.game.sizes_window()
            && self.showing()
            && let Some(size) = self.graphics.as_ref().and_then(|g| g.renderer.shown_size())
            && self.fitted != Some(size)
        {
            self.fitted = Some(size);
            let _ = window.request_inner_size(LogicalSize::new(size.0, size.1));
        }
        // Only a change in what the game asks for switches the window, so F4 (which can't change the
        // game's state: that's simulation) stays in effect until the game asks again.
        if let Some(full) = self.game.fullscreen()
            && full != self.fullscreen
        {
            self.fullscreen = full;
            if window.fullscreen().is_some() != full {
                window.set_fullscreen(full.then_some(Fullscreen::Borderless(None)));
            }
        }
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
            // Keys go to the focused element: start with the canvas, not the page. Unless a page
            // embeds the replay: the reader's keys are its own until they take control.
            if self.embed.is_none()
                && let Some(canvas) = window.canvas()
            {
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
            UserEvent::Control(control) => self.control(control),
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
                if self.showing()
                    && let Some(g) = &mut self.graphics
                {
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
                if down {
                    self.game.user_gesture();
                }
                // A key belongs to whoever took its press, repeats and release included: one the overlay
                // took stays its own until released, and one held for the game when a dialog opens keeps
                // repeating for the game and is released to it, so none sticks.
                let taken = if down {
                    self.taken.contains(&key)
                        || (!repeat && self.overlay.as_mut().is_some_and(|o| o.key_down(key)))
                } else {
                    self.taken
                        .iter()
                        .position(|&k| k == key)
                        .map(|i| self.taken.swap_remove(i))
                        .is_some()
                };
                if taken {
                    if down && !self.taken.contains(&key) {
                        self.taken.push(key);
                    }
                    return;
                }
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
                if state == ElementState::Pressed {
                    self.game.user_gesture();
                }
                // As with keys: a press the overlay took keeps its release from the game.
                let pressed = state == ElementState::Pressed;
                if pressed && self.overlay.as_ref().is_some_and(|o| o.modal()) {
                    self.taken_buttons.push(b);
                    return;
                }
                if !pressed && let Some(i) = self.taken_buttons.iter().position(|&t| t == b) {
                    self.taken_buttons.swap_remove(i);
                    return;
                }
                if let Some(c) = self.collector() {
                    c.mouse_button(b, state == ElementState::Pressed);
                }
            }
            // A tap is a gesture too, though not the game's: Spelunky has no touch input. Phones count its end
            // (pointerup), desktops its start.
            WindowEvent::Touch(Touch {
                phase: TouchPhase::Started | TouchPhase::Ended,
                ..
            }) => self.game.user_gesture(),
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

/// Renders what booting drew, if anything: surfaces it filled carry into the first tick.
fn render_boot(graphics: Option<&mut Graphics>, frame: &Frame) {
    if let Some(g) = graphics
        && !frame.commands.is_empty()
    {
        g.renderer.render(frame);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Savestates of a clip from frame 270, before the frames `frames`, each holding its frame.
    fn savestates(frames: &[u64]) -> Savestates {
        let mut s = Savestates::new(270);
        for &f in frames {
            s.insert(f, kept(f));
        }
        s
    }

    /// A savestate holding its frame, with no surfaces.
    fn kept(frame: u64) -> Kept {
        Kept {
            state: Box::new(frame),
            surfaces: None,
        }
    }

    fn start(s: &Savestates, target: u64, at: Option<u64>) -> Option<Option<u64>> {
        match s.start(target, at) {
            Start::Here => Some(None),
            Start::From(frame, kept) => {
                assert_eq!(kept.state.downcast_ref::<u64>(), Some(&frame));
                Some(Some(frame))
            }
            Start::Nowhere => None,
        }
    }

    #[test]
    fn savestates_are_kept_every_second_from_the_clips_start() {
        let s = savestates(&[300]);
        let wanted: Vec<u64> = (0..400).filter(|&f| s.wants(f)).collect();
        // None before the clip, and 300 is kept already.
        assert_eq!(wanted, [270, 330, 360, 390]);
    }

    #[test]
    fn too_many_savestates_are_thinned_to_half_as_often() {
        let mut s = Savestates::new(270);
        let mut frame = 270;
        while s.states.len() < MAX_SAVESTATES {
            s.insert(frame, kept(frame));
            frame += SAVESTATE_EVERY;
        }
        assert!(!s.wants(frame - SAVESTATE_EVERY) && s.wants(frame));
        s.insert(frame, kept(frame));
        assert_eq!(s.states.len(), MAX_SAVESTATES / 2 + 1);
        assert!(
            s.states
                .keys()
                .all(|f| (f - 270) % (2 * SAVESTATE_EVERY) == 0)
        );
        // The clip's first frame's stays, for Restart, and the next is kept twice as far on.
        assert!(s.states.contains_key(&270));
        assert!(!s.wants(frame + SAVESTATE_EVERY) && s.wants(frame + 2 * SAVESTATE_EVERY));
    }

    #[test]
    fn a_seek_runs_from_the_nearest_savestate_before_it_or_from_where_the_game_is() {
        let s = savestates(&[270, 300, 330]);
        // Back: the savestate at or before it.
        assert_eq!(start(&s, 310, Some(400)), Some(Some(300)));
        assert_eq!(start(&s, 300, Some(400)), Some(Some(300)));
        // Restart: the clip's first frame.
        assert_eq!(start(&s, 270, Some(280)), Some(Some(270)));
        // Forward, a little: on from where it is.
        assert_eq!(start(&s, 320, Some(305)), Some(None));
        assert_eq!(start(&s, 320, Some(320)), Some(None));
        assert_eq!(start(&s, 320, Some(300)), Some(None));
        // Forward, past a savestate: from that, which is nearer.
        assert_eq!(start(&s, 340, Some(305)), Some(Some(330)));
        // Past where the savestates reach: from the last one, or on from where it is.
        assert_eq!(start(&s, 500, None), Some(Some(330)));
        assert_eq!(start(&s, 500, Some(450)), Some(None));
        // The frame showing is the one before `at`, so a seek to it is run again from a savestate.
        assert_eq!(start(&s, 310, Some(311)), Some(Some(300)));
    }

    #[test]
    fn a_seek_without_savestates_runs_on_or_goes_nowhere() {
        let s = Savestates::new(270);
        // On the way to the clip's start, before its first savestate.
        assert_eq!(start(&s, 280, Some(100)), Some(None));
        // A game that makes none can't go back, nor from the reader's game.
        assert_eq!(start(&s, 280, Some(300)), None);
        assert_eq!(start(&s, 280, None), None);
    }
}
