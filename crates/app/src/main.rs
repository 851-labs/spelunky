//! The frontend, natively and in the browser (`scripts/web`; see the `web` module for its URL options).
//! It plays the game, from the intro on:
//!
//! ```sh
//! scripts/cargo run --release -p app                              # play
//! scripts/cargo run --release -p app -- --record out.txt --trace out.jsonl
//! scripts/cargo run --release -p app -- --replay out.txt          # watch a replay
//! scripts/cargo run --release -p app -- --replay out.txt --frames 900 --screenshot shot.png [--scale 1]
//! scripts/cargo run --release -p app -- --replay out.txt --frames 900 --capture-from 750 --screenshot frames/
//! scripts/cargo run --release -p app -- --preview --room rTutorial  # a room's placed state, for debugging
//! ```
//!
//! `--assets DIR` points at the extracted bundle (default `assets`, made by `scripts/extract`). `--scale N`
//! (1 to 16) sets a screenshot's pixel size (default 1), or the window's size in logical pixels per GM
//! pixel (by default the window takes the size the game's rooms ask for, as GM's does).
//!
//! Play keeps the game's files (`stats.txt`, the `.cfg` files) and highscores in the per-user data
//! directory (`app::save`; `--data DIR` for another, `--no-save` for none). A replay, and a recorded
//! session, start from the replay's own files instead, so a recording plays back the same anywhere:
//! `replay-run --faithful-audio out.txt` gives the trace `--trace` wrote live. Sound plays unless `--mute`.
//!
//! Input goes through the game's path: GM8 key codes and replay events (`app::input`). Keys get the
//! layout's codes, as in GM8; `--physical-keys` uses their US positions instead. A gamepad connected by
//! the first tick is GM's joystick 1 and makes the recording declare `joystick pov`; `--no-gamepad` keeps
//! the session, and its recording, keyboard-only, as the oracle needs. `--screenshot` runs headless: the
//! replay (or no input) up to `--frames`, shooting the last picture shown. With `--capture-from F` it
//! shoots every tick's picture from frame F on into the directory `--screenshot` names, as
//! `000750.png` and so on, for `scripts/clip` to make a video of.
//!
//! The preview (`--preview`, with `--room NAME` and `--view X,Y`) shows a room's placed instances without
//! running anything; the arrow keys pan.

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    if let Err(e) = native::run() {
        eprintln!("app: {e}");
        std::process::exit(1);
    }
}

// The browser build (`scripts/web`) starts here, with wasm-bindgen's start function.
#[cfg(target_arch = "wasm32")]
fn main() {
    app::web::install_panic_hook();
    wasm_bindgen_futures::spawn_local(async {
        if let Err(e) = web::run().await {
            app::web::show_error(&e);
        }
    });
}

use app::preview::MAX_SCALE;

/// The window's starting scale for the game: `scrInit`'s default `screenScale`. The window then takes
/// the size the game's rooms ask for.
const GAME_SCALE: u32 = 3;

/// A view position, `x,y`.
fn parse_view(v: &str) -> Result<(i32, i32), String> {
    let (x, y) = v.split_once(',').ok_or("the view takes x,y")?;
    let parse = |s: &str| s.trim().parse::<i32>().map_err(|_| format!("bad view {v}"));
    Ok((parse(x)?, parse(y)?))
}

fn parse_scale(s: &str) -> Result<u32, String> {
    s.parse()
        .ok()
        .filter(|s| (1..=MAX_SCALE).contains(s))
        .ok_or(format!("the scale is a whole number from 1 to {MAX_SCALE}"))
}

/// The game's starting window size in logical pixels: its 320x240 view at `scale`.
fn game_window_size(scale: Option<u32>) -> (u32, u32) {
    let scale = scale.unwrap_or(GAME_SCALE);
    (320 * scale, 240 * scale)
}

/// The browser build: the native app's window path, with the page's URL query for its command line
/// (`?replay=URL&mute&no-gamepad&physical-keys`, or `?preview&room=rTutorial&view=0,64&scale=3`) and the
/// assets fetched from `assets/` next to the page. Play keeps its saves in `localStorage`; sound starts at
/// the first key or click, as browsers require. Recording has no file to go to yet: a [`RecordSink`] that
/// downloads or stores it is where it plugs in. There's no screenshot mode: the browser's own does that.
///
/// [`RecordSink`]: app::window::RecordSink
#[cfg(target_arch = "wasm32")]
mod web {
    use app::input::KeyMap;
    use app::notice::{Notice, TITLE};
    use app::play::{Play, SoundOutput, session_header, sound_bank};
    use app::preview::{Preview, RoomPreview, room_to_preview};
    use app::textures::Textures;
    use app::web::{Query, fetch};
    use app::window::{App, Game, Input, UserEvent};
    use assets::Bundle;
    use std::collections::HashMap;
    use std::sync::Arc;
    use trace::replay::{Replay, ReplayHeader};
    use winit::event_loop::EventLoop;
    use winit::platform::web::EventLoopExtWebSys;

    /// Where `scripts/web` puts the bundle, relative to the page.
    const ASSETS: &str = "assets";

    /// Fetches `files` from the bundle, all at once.
    async fn fetch_all<'a>(
        files: impl Iterator<Item = &'a str>,
    ) -> Result<HashMap<String, Vec<u8>>, String> {
        // Every request starts before the first is awaited, so they download together.
        let requests: Vec<_> = files
            .map(|file| (file.to_owned(), fetch(&format!("{ASSETS}/{file}"))))
            .collect();
        let mut out = HashMap::new();
        for (file, request) in requests {
            out.insert(file, request.await?);
        }
        Ok(out)
    }

    fn take(files: &mut HashMap<String, Vec<u8>>, file: &str) -> Result<Vec<u8>, String> {
        files
            .remove(file)
            .ok_or_else(|| format!("{file} wasn't fetched"))
    }

    async fn fetch_replay(url: &str) -> Result<Replay, String> {
        let text = String::from_utf8(fetch(url).await?).map_err(|e| format!("{url}: {e}"))?;
        Replay::parse(&text).map_err(|e| format!("{url}: {e}"))
    }

    pub async fn run() -> Result<(), String> {
        let query = Query::from_page()?;
        let bundle = fetch(&format!("{ASSETS}/{}", assets::BUNDLE_FILE)).await?;
        let bundle =
            Bundle::from_bytes(&bundle).map_err(|e| format!("{}: {e}", assets::BUNDLE_FILE))?;
        let replay = match query.get("replay") {
            Some(url) => Some(fetch_replay(&url).await?),
            None => None,
        };
        let preview = query.has("preview");
        // The game needs its sound files even muted: the sound model answers the game from them, and
        // without them it would answer as if they were missing, a different game from an unmuted one.
        let sound_files: Vec<&str> = if preview {
            Vec::new()
        } else {
            bundle.sounds.iter().map(|s| s.file.as_str()).collect()
        };
        let mut files = fetch_all(Textures::files(&bundle).chain(sound_files)).await?;
        let textures = Textures::load(&bundle, |file| take(&mut files, file))?;
        let event_loop = EventLoop::<UserEvent>::with_user_event()
            .build()
            .map_err(|e| e.to_string())?;
        let live = |header: ReplayHeader| {
            let keys = if query.has("physical-keys") {
                KeyMap::Physical
            } else {
                KeyMap::Layout
            };
            // The Gamepad API only reports a pad after a button press on the page, so one is usually
            // joystick 1 only if the page is reloaded with it already in use.
            Input::live(header, keys, !query.has("no-gamepad"), None)
        };
        let scale = query
            .get("scale")
            .as_deref()
            .map(super::parse_scale)
            .transpose()?;

        if preview {
            let room_name = room_to_preview(replay.as_ref(), query.get("room").as_deref());
            if replay.is_some() && query.has("room") {
                app::web::warn(&format!(
                    "room is ignored with replay, which plays in its own room ({room_name})"
                ));
            }
            // The preview borrows the bundle for as long as the page runs.
            let bundle: &'static Bundle = Box::leak(Box::new(bundle));
            let mut room = RoomPreview::new(bundle, &room_name)?;
            if let Some(view) = query.get("view") {
                let (x, y) = super::parse_view(&view)?;
                room.move_view(x, y);
            }
            let preview = Preview::new(room);
            let size = preview.window_size(scale.unwrap_or(2));
            let input = match replay {
                Some(replay) => Input::Replay(replay),
                None => live(ReplayHeader {
                    room: room_name.clone(),
                    ..ReplayHeader::default()
                }),
            };
            let title = format!("Spelunky - {room_name}");
            let proxy = event_loop.create_proxy();
            start(
                event_loop,
                App::new(preview, textures, &title, size, proxy, input),
            );
            return Ok(());
        }

        for ignored in ["room", "view"] {
            if query.has(ignored) {
                app::web::warn(&format!(
                    "{ignored} is for preview; the game starts in its intro"
                ));
            }
        }
        let notice = Notice::new(&bundle);
        let sounds = sound_bank(&bundle.sounds, |file| take(&mut files, file))?;
        let sound = if query.has("mute") {
            SoundOutput::Off
        } else {
            SoundOutput::OnGesture
        };
        let mut play = Play::new(Arc::new(gm::Assets::from_bundle(bundle)), sounds, sound);
        if scale.is_some() {
            play = play.with_fixed_window();
        }
        let input = match replay {
            Some(replay) => Input::Replay(replay),
            None => {
                match app::save::LocalStorage::new() {
                    Ok(store) => play = play.with_store(Box::new(store)),
                    Err(e) => app::web::warn(&format!("the game won't be saved: {e}")),
                }
                live(session_header())
            }
        };
        let size = super::game_window_size(scale);
        let proxy = event_loop.create_proxy();
        let app = App::new(play, textures, TITLE, size, proxy, input).with_notice(notice);
        start(event_loop, app);
        Ok(())
    }

    fn start<G: Game + 'static>(event_loop: EventLoop<UserEvent>, app: App<G>) {
        app::web::clear_status();
        event_loop.spawn_app(app);
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use app::input::KeyMap;
    use app::notice::{Notice, TITLE};
    use app::play::{Play, SoundOutput, TraceSink, session_header, sound_bank};
    use app::preview::{Preview, RoomPreview, room_to_preview};
    use app::render::Renderer;
    use app::save::DirStore;
    use app::textures::Textures;
    use app::window::{App, Game, Input, RecordSink, UserEvent};
    use assets::Bundle;
    use draw::Frame;
    use std::fs::File;
    use std::io::{BufWriter, Write};
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use trace::replay::{Event, Replay, ReplayHeader, event_line};
    use trace::{Filter, TraceHeader, TraceWriter};
    use winit::event_loop::EventLoop;

    const USAGE: &str = "usage: app [--assets DIR] [--scale N] [--record OUT.txt | --replay IN.txt] \
        [--trace OUT.jsonl] [--screenshot OUT.png [--frames N] [--capture-from F]] [--data DIR | --no-save] [--mute] \
        [--no-gamepad] [--physical-keys]\n       app --preview [--room NAME] [--view X,Y] [--scale N] \
        [--screenshot OUT.png] [--record OUT.txt | --replay IN.txt]";

    struct Args {
        assets: PathBuf,
        preview: bool,
        room: Option<String>,
        view: Option<(i32, i32)>,
        screenshot: Option<PathBuf>,
        frames: Option<u64>,
        capture_from: Option<u64>,
        scale: Option<u32>,
        record: Option<PathBuf>,
        replay: Option<PathBuf>,
        trace: Option<PathBuf>,
        data: Option<PathBuf>,
        save: bool,
        mute: bool,
        gamepad: bool,
        keys: KeyMap,
    }

    fn parse_args() -> Result<Args, String> {
        let mut args = Args {
            assets: PathBuf::from("assets"),
            preview: false,
            room: None,
            view: None,
            screenshot: None,
            frames: None,
            capture_from: None,
            scale: None,
            record: None,
            replay: None,
            trace: None,
            data: None,
            save: true,
            mute: false,
            gamepad: true,
            keys: KeyMap::Layout,
        };
        let mut it = std::env::args().skip(1);
        while let Some(flag) = it.next() {
            let mut value = || it.next().ok_or_else(|| format!("{flag} needs a value"));
            match flag.as_str() {
                "--assets" => args.assets = value()?.into(),
                "--preview" => args.preview = true,
                "--room" => args.room = Some(value()?),
                "--view" => args.view = Some(super::parse_view(&value()?)?),
                "--screenshot" => args.screenshot = Some(value()?.into()),
                "--frames" => {
                    let v = value()?;
                    args.frames = Some(v.parse().map_err(|_| format!("bad --frames {v}"))?);
                }
                "--capture-from" => {
                    let v = value()?;
                    args.capture_from =
                        Some(v.parse().map_err(|_| format!("bad --capture-from {v}"))?);
                }
                "--record" => args.record = Some(value()?.into()),
                "--replay" => args.replay = Some(value()?.into()),
                "--trace" => args.trace = Some(value()?.into()),
                "--data" => args.data = Some(value()?.into()),
                "--no-save" => args.save = false,
                "--mute" => args.mute = true,
                "--no-gamepad" => args.gamepad = false,
                "--physical-keys" => args.keys = KeyMap::Physical,
                "--scale" => args.scale = Some(super::parse_scale(&value()?)?),
                "-h" | "--help" => return Err(USAGE.into()),
                other => return Err(format!("unknown argument {other}\n{USAGE}")),
            }
        }
        if args.record.is_some() && args.replay.is_some() {
            return Err("--record and --replay don't mix".into());
        }
        if args.record.is_some() && args.screenshot.is_some() {
            return Err("--record needs the window, not --screenshot".into());
        }
        if args.preview {
            if args.trace.is_some() || args.frames.is_some() {
                return Err("--trace and --frames run the game, not --preview".into());
            }
            if args.mute || args.data.is_some() || !args.save {
                return Err("--preview has no sound or saves".into());
            }
        } else if args.room.is_some() || args.view.is_some() {
            return Err("--room and --view are for --preview".into());
        }
        if args.frames.is_some() && args.screenshot.is_none() {
            return Err("--frames is for --screenshot".into());
        }
        if args.capture_from.is_some() && (args.screenshot.is_none() || args.preview) {
            return Err("--capture-from is for a game --screenshot".into());
        }
        if args.data.is_some()
            && (args.replay.is_some()
                || args.record.is_some()
                || args.screenshot.is_some()
                || !args.save)
        {
            return Err(
                "--data is for play: a replay, a recording or a screenshot starts from \
                        its own files"
                    .into(),
            );
        }
        if !args.preview
            && args.screenshot.is_some()
            && args.replay.is_none()
            && args.frames.is_none()
        {
            return Err("--screenshot needs --replay or --frames to know when to shoot".into());
        }
        Ok(args)
    }

    fn reader(dir: &Path) -> impl Fn(&str) -> Result<Vec<u8>, String> {
        move |rel: &str| {
            let path = dir.join(rel);
            std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))
        }
    }

    fn load(dir: &Path) -> Result<(Bundle, Textures), String> {
        let read = reader(dir);
        let bundle = Bundle::from_bytes(&read(assets::BUNDLE_FILE)?)
            .map_err(|e| format!("{}: {e}", dir.join(assets::BUNDLE_FILE).display()))?;
        let textures = Textures::load(&bundle, read)?;
        Ok((bundle, textures))
    }

    fn read_replay(path: &Path) -> Result<Replay, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Replay::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Writes a live session's replay as it goes: the header, each tick's events (flushed, so a crash
    /// keeps them), and `frames N` at the end. The file is only created when the session begins, so a run
    /// that fails to start leaves an older recording there alone.
    struct RecordFile {
        path: PathBuf,
        out: Option<BufWriter<File>>,
    }

    impl RecordFile {
        fn new(path: &Path) -> RecordFile {
            RecordFile {
                path: path.to_owned(),
                out: None,
            }
        }

        fn write(&mut self, text: &str) -> Result<(), String> {
            let out = self.out.as_mut().ok_or("the recording hasn't begun")?;
            out.write_all(text.as_bytes())
                .and_then(|()| out.flush())
                .map_err(|e| format!("{}: {e}", self.path.display()))
        }
    }

    impl RecordSink for RecordFile {
        fn begin(&mut self, header: &ReplayHeader) -> Result<(), String> {
            let file =
                File::create(&self.path).map_err(|e| format!("{}: {e}", self.path.display()))?;
            self.out = Some(BufWriter::new(file));
            self.write(&header.to_text())
        }

        fn frame(&mut self, frame: u64, events: &[Event]) -> Result<(), String> {
            if events.is_empty() {
                return Ok(());
            }
            let lines: String = events.iter().map(|e| event_line(frame, e)).collect();
            self.write(&lines)
        }

        fn finish(&mut self, frames: u64) -> Result<(), String> {
            self.write(&format!("frames {frames}\n"))?;
            println!("recorded {} ({frames} frames)", self.path.display());
            Ok(())
        }
    }

    /// A trace file like `replay-run`'s, written a tick at a time. It's buffered: a crash can lose the last
    /// few lines, which the recording replays anyway.
    fn trace_file(path: &Path) -> Result<TraceSink, String> {
        let error = |e: std::io::Error| format!("{}: {e}", path.display());
        let file = File::create(path).map_err(error)?;
        let mut writer = TraceWriter::with_header(
            BufWriter::new(file),
            TraceHeader::new(Some("port")),
            Filter::default(),
        )
        .map_err(error)?;
        let path = path.to_owned();
        Ok(Box::new(move |frame| {
            writer
                .write_frame(frame)
                .map_err(|e| format!("{}: {e}", path.display()))
        }))
    }

    pub fn run() -> Result<(), String> {
        let args = parse_args()?;
        let (bundle, textures) = load(&args.assets)?;
        let replay = args.replay.as_deref().map(read_replay).transpose()?;
        let record = args
            .record
            .as_deref()
            .map(|path| Box::new(RecordFile::new(path)) as Box<dyn RecordSink>);
        if args.preview {
            return preview(&args, &bundle, textures, replay, record);
        }

        let notice = Notice::new(&bundle);
        let sounds = sound_bank(&bundle.sounds, reader(&args.assets))?;
        let headless = args.screenshot.is_some();
        let sound = if args.mute || headless {
            SoundOutput::Off
        } else {
            SoundOutput::Now
        };
        let mut play = Play::new(Arc::new(gm::Assets::from_bundle(bundle)), sounds, sound);
        if let Some(path) = &args.trace {
            play = play.with_trace(trace_file(path)?);
        }
        if args.scale.is_some() {
            play = play.with_fixed_window();
        }
        // Only a session of play keeps saves: a replay or a recording has its files in its header, and a
        // headless run leaves the player's saves alone.
        if replay.is_none() && record.is_none() && !headless && args.save {
            match args.data.clone().or_else(DirStore::default_dir) {
                Some(dir) => play = play.with_store(Box::new(DirStore::new(dir))),
                None => eprintln!("app: no home directory, so the game won't be saved"),
            }
        }

        if let Some(out) = &args.screenshot {
            return game_screenshot(play, textures, replay, &args, out);
        }
        let input = match replay {
            Some(replay) => Input::Replay(replay),
            None => Input::live(session_header(), args.keys, args.gamepad, record),
        };
        let app = run_window(
            play,
            textures,
            TITLE,
            super::game_window_size(args.scale),
            input,
            notice,
        )?;
        let ticks = app.ticks();
        let error = app.error().map(str::to_owned);
        // The end state, to compare a recording with its playback.
        if let Some(game) = app.game().game() {
            let w = game.world();
            let room = w.assets().room_name(w.room().id).unwrap_or("?");
            println!("{ticks} frames, ended in {room}");
        }
        error.map_or(Ok(()), Err)
    }

    fn run_window<G: Game>(
        game: G,
        textures: Textures,
        title: &str,
        size: (u32, u32),
        input: Input,
        notice: Option<Notice>,
    ) -> Result<App<G>, String> {
        let event_loop = EventLoop::<UserEvent>::with_user_event()
            .build()
            .map_err(|e| e.to_string())?;
        // Ctrl+C closes the window properly, so a recording gets its end.
        let proxy = event_loop.create_proxy();
        ctrlc::set_handler(move || {
            let _ = proxy.send_event(UserEvent::Exit);
        })
        .map_err(|e| format!("can't handle Ctrl+C: {e}"))?;
        let mut app = App::new(
            game,
            textures,
            title,
            size,
            event_loop.create_proxy(),
            input,
        )
        .with_notice(notice);
        event_loop.run_app(&mut app).map_err(|e| e.to_string())?;
        Ok(app)
    }

    fn preview(
        args: &Args,
        bundle: &Bundle,
        textures: Textures,
        replay: Option<Replay>,
        record: Option<Box<dyn RecordSink>>,
    ) -> Result<(), String> {
        // A recording names the room it was made in, and its playback goes there.
        let room_name = room_to_preview(replay.as_ref(), args.room.as_deref());
        if replay.is_some() && args.room.is_some() {
            eprintln!(
                "app: --room is ignored with --replay, which plays in its own room ({room_name})"
            );
        }
        let mut room = RoomPreview::new(bundle, &room_name)?;
        if let Some((x, y)) = args.view {
            room.move_view(x, y);
        }
        let mut preview = Preview::new(room);
        if let Some(out) = &args.screenshot {
            if let Some(replay) = &replay {
                for frame in 0..replay.len() {
                    let events: Vec<Event> = replay.frame_events(frame).collect();
                    preview.step(&events);
                }
            }
            let frame = preview.room.frame();
            let renderer = shoot(textures, &frame, out, args.scale.unwrap_or(1))?;
            let view = preview.room.view;
            println!(
                "{}: {} at view {},{} ({} commands, {} draw calls)",
                out.display(),
                preview.room.room().name,
                view.x,
                view.y,
                frame.commands.len(),
                renderer.draw_calls()
            );
            return Ok(());
        }
        let size = preview.window_size(args.scale.unwrap_or(2));
        let input = match replay {
            Some(replay) => Input::Replay(replay),
            None => {
                let header = ReplayHeader {
                    room: room_name.clone(),
                    ..ReplayHeader::default()
                };
                Input::live(header, args.keys, args.gamepad, record)
            }
        };
        let title = format!("Spelunky - {room_name}");
        let app = run_window(preview, textures, &title, size, input, None)?;
        let view = app.game().room.view;
        println!("{} frames, view at {},{}", app.ticks(), view.x, view.y);
        app.error().map_or(Ok(()), |e| Err(e.to_owned()))
    }

    /// Runs the game headless, on the replay's input (or none) up to `--frames` (or the replay's end),
    /// rendering every frame, and shoots the last picture shown.
    fn game_screenshot(
        mut play: Play,
        textures: Textures,
        replay: Option<Replay>,
        args: &Args,
        out: &Path,
    ) -> Result<(), String> {
        // Without a replay, the default one's: the same picture every time.
        let header = replay
            .as_ref()
            .map_or_else(ReplayHeader::default, |r| r.header.clone());
        let end = match (&replay, args.frames) {
            (Some(r), Some(n)) => n.min(r.len()),
            (Some(r), None) => r.len(),
            (None, n) => n.unwrap_or(0),
        };
        let scale = args.scale.unwrap_or(1);
        if args.capture_from.is_some() {
            std::fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
        }
        let mut renderer = headless_renderer(textures)?;
        let mut frame = Frame::default();
        play.begin(&header, &mut frame)?;
        if !frame.commands.is_empty() {
            renderer.render(&frame);
        }
        let mut events = Vec::new();
        for f in 0..end {
            if play.ended() {
                break;
            }
            events.clear();
            if let Some(r) = &replay {
                events.extend(r.frame_events(f));
            }
            play.tick(&events, &mut frame)?;
            renderer.render(&frame);
            if args.capture_from.is_some_and(|from| f >= from) {
                save_screenshot(&renderer, &out.join(format!("{f:06}.png")), scale)?;
            }
        }
        if args.capture_from.is_some() {
            return Ok(());
        }
        save_screenshot(&renderer, out, scale)?;
        let game = play.game().ok_or("the game didn't begin")?;
        let w = game.world();
        println!(
            "{}: frame {} in {}",
            out.display(),
            w.frames().saturating_sub(1),
            w.assets().room_name(w.room().id).unwrap_or("?")
        );
        Ok(())
    }

    fn shoot(
        textures: Textures,
        frame: &Frame,
        out: &Path,
        scale: u32,
    ) -> Result<Renderer, String> {
        let mut renderer = headless_renderer(textures)?;
        renderer.render(frame);
        save_screenshot(&renderer, out, scale)?;
        Ok(renderer)
    }

    /// A renderer without a window, for screenshots.
    fn headless_renderer(textures: Textures) -> Result<Renderer, String> {
        let gpu = pollster::block_on(app::gpu::request(app::gpu::instance(), None))?;
        Ok(Renderer::new(gpu.device, gpu.queue, textures))
    }

    fn save_screenshot(renderer: &Renderer, out: &Path, scale: u32) -> Result<(), String> {
        let (w, h, rgba) = renderer.read_screen()?;
        let (w, h, rgba) = upscale(w, h, &rgba, scale)?;
        write_png(out, w, h, &rgba)
    }

    /// Nearest-neighbour upscaling by a whole number, like the window's presentation.
    fn upscale(w: u32, h: u32, rgba: &[u8], scale: u32) -> Result<(u32, u32, Vec<u8>), String> {
        if scale == 1 {
            return Ok((w, h, rgba.to_vec()));
        }
        let too_big = || format!("a {w}x{h} frame at --scale {scale} is too large");
        let sw = w.checked_mul(scale).ok_or_else(too_big)?;
        let sh = h.checked_mul(scale).ok_or_else(too_big)?;
        let bytes = (sw as usize)
            .checked_mul(sh as usize)
            .and_then(|n| n.checked_mul(4))
            .ok_or_else(too_big)?;
        let mut out = Vec::with_capacity(bytes);
        for y in 0..sh {
            let row = &rgba[((y / scale) * w * 4) as usize..][..(w * 4) as usize];
            for px in row.as_chunks::<4>().0 {
                for _ in 0..scale {
                    out.extend_from_slice(px);
                }
            }
        }
        Ok((sw, sh, out))
    }

    fn write_png(path: &Path, w: u32, h: u32, rgba: &[u8]) -> Result<(), String> {
        // Opaque, like the window: GM's back buffer alpha isn't part of the picture.
        let rgb: Vec<u8> = rgba
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|&[r, g, b, _]| [r, g, b])
            .collect();
        let file = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), w, h);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
        writer.write_image_data(&rgb).map_err(|e| e.to_string())
    }
}
