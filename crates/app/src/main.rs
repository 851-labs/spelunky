//! The native frontend. Until the game runs, it previews a room's placed state:
//!
//! ```sh
//! scripts/cargo run -p app -- --room rTutorial            # a window; arrow keys pan, shift pans faster
//! scripts/cargo run -p app -- --room rTutorial --view 0,64 --screenshot out.png [--scale 3]
//! ```
//!
//! `--assets DIR` points at the extracted bundle (default `assets`, made by `scripts/extract`). `--scale N`
//! (1 to 16) sets the screenshot's pixel size (default 1) or the window's starting size in logical pixels
//! per GM pixel (default 2).
//!
//! Input goes through the game's path: GM8 key codes and replay events (`app::input`). Keys get the
//! layout's codes, as in GM8; `--physical-keys` uses their US positions instead. `--record OUT.txt` saves
//! the session as a replay while it runs (its `room` is `--room`; `--view` isn't recorded), and `--replay
//! IN.txt` plays one back in its room instead of reading the devices (with `--screenshot`, headlessly,
//! shooting the last frame). A gamepad connected by the first tick is GM's joystick 1 and makes the
//! recording declare `joystick pov`; `--no-gamepad` keeps the session, and its recording, keyboard-only,
//! as the oracle needs.

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    if let Err(e) = native::run() {
        eprintln!("app: {e}");
        std::process::exit(1);
    }
}

// The web entry point arrives with the web build (851-3133).
#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use app::input::{Gamepads, KeyMap, VK_DOWN, VK_LEFT, VK_RIGHT, VK_SHIFT, VK_UP};
    use app::preview::RoomPreview;
    use app::render::Renderer;
    use app::textures::Textures;
    use app::window::{App, Game, Input, RecordSink, UserEvent};
    use assets::Bundle;
    use draw::Frame;
    use std::fs::File;
    use std::io::{BufWriter, Write};
    use std::path::{Path, PathBuf};
    use trace::replay::{Button, Event, InputState, Replay, ReplayHeader, event_line};
    use winit::event_loop::EventLoop;

    /// The room previewed when neither `--room` nor a replay names one.
    const DEFAULT_ROOM: &str = "rTutorial";

    struct Args {
        assets: PathBuf,
        room: Option<String>,
        view: Option<(i32, i32)>,
        screenshot: Option<PathBuf>,
        scale: Option<u32>,
        record: Option<PathBuf>,
        replay: Option<PathBuf>,
        gamepad: bool,
        keys: KeyMap,
    }

    /// Largest `--scale`: 16x a 320x240 view is already 5120x3840.
    const MAX_SCALE: u32 = 16;

    fn parse_args() -> Result<Args, String> {
        let mut args = Args {
            assets: PathBuf::from("assets"),
            room: None,
            view: None,
            screenshot: None,
            scale: None,
            record: None,
            replay: None,
            gamepad: true,
            keys: KeyMap::Layout,
        };
        let mut it = std::env::args().skip(1);
        while let Some(flag) = it.next() {
            let mut value = || it.next().ok_or_else(|| format!("{flag} needs a value"));
            match flag.as_str() {
                "--assets" => args.assets = value()?.into(),
                "--room" => args.room = Some(value()?),
                "--view" => {
                    let v = value()?;
                    let (x, y) = v.split_once(',').ok_or("--view takes x,y")?;
                    let parse = |s: &str| {
                        s.trim()
                            .parse::<i32>()
                            .map_err(|_| format!("bad --view {v}"))
                    };
                    args.view = Some((parse(x)?, parse(y)?));
                }
                "--screenshot" => args.screenshot = Some(value()?.into()),
                "--record" => args.record = Some(value()?.into()),
                "--replay" => args.replay = Some(value()?.into()),
                "--no-gamepad" => args.gamepad = false,
                "--physical-keys" => args.keys = KeyMap::Physical,
                "--scale" => {
                    let scale = value()?
                        .parse()
                        .ok()
                        .filter(|s| (1..=MAX_SCALE).contains(s))
                        .ok_or(format!(
                            "--scale takes a whole number from 1 to {MAX_SCALE}"
                        ))?;
                    args.scale = Some(scale);
                }
                "-h" | "--help" => {
                    return Err(
                        "usage: app [--assets DIR] [--room NAME] [--view X,Y] [--scale N] [--screenshot OUT.png] \
                         [--record OUT.txt | --replay IN.txt] [--no-gamepad] [--physical-keys]"
                            .into(),
                    );
                }
                other => return Err(format!("unknown argument {other}")),
            }
        }
        if args.record.is_some() && args.replay.is_some() {
            return Err("--record and --replay don't mix".into());
        }
        if args.record.is_some() && args.screenshot.is_some() {
            return Err("--record needs the window, not --screenshot".into());
        }
        Ok(args)
    }

    fn load(dir: &Path) -> Result<(Bundle, Textures), String> {
        let read = |rel: &str| {
            let path = dir.join(rel);
            std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))
        };
        let bundle = Bundle::from_bytes(&read(assets::BUNDLE_FILE)?)
            .map_err(|e| format!("{}: {e}", dir.join(assets::BUNDLE_FILE).display()))?;
        let textures = Textures::load(&bundle, read)?;
        Ok((bundle, textures))
    }

    /// Pans the preview's view with the arrow keys (faster with shift), read as GM key codes.
    struct Preview<'a> {
        room: RoomPreview<'a>,
        held: InputState,
    }

    impl Game for Preview<'_> {
        fn tick(&mut self, events: &[Event], out: &mut Frame) {
            self.step(events);
            *out = self.room.frame();
        }
    }

    impl Preview<'_> {
        fn step(&mut self, events: &[Event]) {
            for e in events {
                self.held.apply(e);
            }
            let held = |vk| self.held.is_down(Button::Key(vk));
            let speed = if held(VK_SHIFT) { 16 } else { 4 };
            let axis = |neg, pos| (i32::from(held(pos)) - i32::from(held(neg))) * speed;
            let (dx, dy) = (axis(VK_LEFT, VK_RIGHT), axis(VK_UP, VK_DOWN));
            let view = self.room.view;
            self.room.move_view(view.x + dx, view.y + dy);
        }
    }

    fn read_replay(path: &Path) -> Result<Replay, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Replay::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Writes a live session's replay as it goes: the header, each tick's events (flushed, so a crash
    /// keeps them), and `frames N` at the end.
    struct RecordFile {
        path: PathBuf,
        out: BufWriter<File>,
    }

    impl RecordFile {
        fn create(path: &Path) -> Result<RecordFile, String> {
            let file = File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
            Ok(RecordFile {
                path: path.to_owned(),
                out: BufWriter::new(file),
            })
        }

        fn write(&mut self, text: &str) -> Result<(), String> {
            self.out
                .write_all(text.as_bytes())
                .and_then(|()| self.out.flush())
                .map_err(|e| format!("{}: {e}", self.path.display()))
        }
    }

    impl RecordSink for RecordFile {
        fn begin(&mut self, header: &ReplayHeader) -> Result<(), String> {
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

    pub fn run() -> Result<(), String> {
        let args = parse_args()?;
        let (bundle, textures) = load(&args.assets)?;
        let replay = args.replay.as_deref().map(read_replay).transpose()?;
        // A recording names the room it was made in, and its playback goes there.
        let room_name = match (&replay, &args.room) {
            (Some(replay), room) => {
                if room.is_some() {
                    eprintln!(
                        "app: --room is ignored with --replay, which plays in its own room ({})",
                        replay.header.room
                    );
                }
                replay.header.room.clone()
            }
            (None, room) => room.as_deref().unwrap_or(DEFAULT_ROOM).to_owned(),
        };
        let mut room = RoomPreview::new(&bundle, &room_name)?;
        if let Some((x, y)) = args.view {
            room.move_view(x, y);
        }
        let mut preview = Preview {
            room,
            held: InputState::default(),
        };
        if let Some(out) = &args.screenshot {
            if let Some(replay) = &replay {
                for frame in 0..replay.len() {
                    let events: Vec<Event> = replay.frame_events(frame).collect();
                    preview.step(&events);
                }
            }
            return screenshot(&preview.room, textures, out, args.scale.unwrap_or(1));
        }
        let scale = args.scale.unwrap_or(2);
        let view = preview.room.view;
        // Within MAX_SCALE of a room-sized view, so this can't overflow.
        let size = (view.w.max(1) as u32 * scale, view.h.max(1) as u32 * scale);
        let input = match replay {
            Some(replay) => Input::Replay(replay),
            None => Input::Live {
                header: ReplayHeader {
                    room: room_name.clone(),
                    ..ReplayHeader::default()
                },
                keys: args.keys,
                // Whether one is connected is settled at the first tick, without waiting here.
                gamepads: args.gamepad.then(Gamepads::new).flatten(),
                record: match &args.record {
                    Some(path) => Some(Box::new(RecordFile::create(path)?)),
                    None => None,
                },
            },
        };
        let event_loop = EventLoop::<UserEvent>::with_user_event()
            .build()
            .map_err(|e| e.to_string())?;
        // Ctrl+C closes the window properly, so a recording gets its end.
        let proxy = event_loop.create_proxy();
        ctrlc::set_handler(move || {
            let _ = proxy.send_event(UserEvent::Exit);
        })
        .map_err(|e| format!("can't handle Ctrl+C: {e}"))?;
        let title = format!("Spelunky - {room_name}");
        let mut app = App::new(
            preview,
            textures,
            &title,
            size,
            event_loop.create_proxy(),
            input,
        );
        event_loop.run_app(&mut app).map_err(|e| e.to_string())?;
        // The end state, to compare a recording with its playback.
        let view = app.game().room.view;
        println!("{} frames, view at {},{}", app.ticks(), view.x, view.y);
        app.error().map_or(Ok(()), |e| Err(e.to_owned()))
    }

    fn screenshot(
        preview: &RoomPreview,
        textures: Textures,
        out: &Path,
        scale: u32,
    ) -> Result<(), String> {
        let gpu = pollster::block_on(app::gpu::request(app::gpu::instance(), None))?;
        let mut renderer = Renderer::new(gpu.device, gpu.queue, textures);
        let frame = preview.frame();
        renderer.render(&frame);
        let (w, h, rgba) = renderer.read_screen()?;
        let (w, h, rgba) = upscale(w, h, &rgba, scale)?;
        write_png(out, w, h, &rgba)?;
        let view = preview.view;
        println!(
            "{}: {} at view {},{} ({} commands, {} draw calls)",
            out.display(),
            preview.room().name,
            view.x,
            view.y,
            frame.commands.len(),
            renderer.draw_calls()
        );
        Ok(())
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
