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
//! Input goes through the game's path: GM8 key codes and replay events (`app::input`). `--record OUT.txt`
//! saves the session as a replay, and `--replay IN.txt` plays one back instead of reading the devices
//! (with `--screenshot`, headlessly, shooting the last frame). A gamepad connected at startup is GM's
//! joystick 1 and makes the recording declare `joystick pov`; `--no-gamepad` keeps the session, and its
//! recording, keyboard-only, as the oracle needs.

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
    use app::preview::RoomPreview;
    use app::render::Renderer;
    use app::textures::Textures;
    use app::window::{App, Game, Input, UserEvent};
    use assets::Bundle;
    use draw::Frame;
    use std::path::{Path, PathBuf};
    use trace::replay::{Button, Event, InputState, Replay, ReplayHeader};
    use winit::event_loop::EventLoop;

    struct Args {
        assets: PathBuf,
        room: String,
        view: Option<(i32, i32)>,
        screenshot: Option<PathBuf>,
        scale: Option<u32>,
        record: Option<PathBuf>,
        replay: Option<PathBuf>,
        gamepad: bool,
    }

    /// Largest `--scale`: 16x a 320x240 view is already 5120x3840.
    const MAX_SCALE: u32 = 16;

    fn parse_args() -> Result<Args, String> {
        let mut args = Args {
            assets: PathBuf::from("assets"),
            room: "rTutorial".into(),
            view: None,
            screenshot: None,
            scale: None,
            record: None,
            replay: None,
            gamepad: true,
        };
        let mut it = std::env::args().skip(1);
        while let Some(flag) = it.next() {
            let mut value = || it.next().ok_or_else(|| format!("{flag} needs a value"));
            match flag.as_str() {
                "--assets" => args.assets = value()?.into(),
                "--room" => args.room = value()?,
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
                         [--record OUT.txt | --replay IN.txt] [--no-gamepad]"
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
            let speed = if held(16) { 16 } else { 4 };
            let axis = |neg, pos| (i32::from(held(pos)) - i32::from(held(neg))) * speed;
            let (dx, dy) = (axis(37, 39), axis(38, 40));
            let view = self.room.view;
            self.room.move_view(view.x + dx, view.y + dy);
        }
    }

    fn read_replay(path: &Path) -> Result<Replay, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Replay::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Whether a gamepad is connected at startup. Some platforms report pads only through events, so this
    /// gives them a moment.
    #[cfg(feature = "gamepad")]
    fn gamepad_connected() -> bool {
        let Some(mut pads) = app::input::Gamepads::new() else {
            return false;
        };
        for _ in 0..10 {
            if pads.poll().is_some() {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        false
    }

    #[cfg(not(feature = "gamepad"))]
    fn gamepad_connected() -> bool {
        false
    }

    pub fn run() -> Result<(), String> {
        let args = parse_args()?;
        let (bundle, textures) = load(&args.assets)?;
        let mut room = RoomPreview::new(&bundle, &args.room)?;
        if let Some((x, y)) = args.view {
            room.move_view(x, y);
        }
        let mut preview = Preview {
            room,
            held: InputState::default(),
        };
        let replay = args.replay.as_deref().map(read_replay).transpose()?;
        if let Some(out) = &args.screenshot {
            if let Some(replay) = &replay {
                for frame in replay.inputs() {
                    let events: Vec<Event> = frame.events.iter().map(|&(_, e)| e).collect();
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
            None => {
                let mut header = ReplayHeader::default();
                if args.gamepad && gamepad_connected() {
                    header.joystick = Some(app::input::PAD_JOYSTICK);
                }
                Input::Live(Replay::new(header))
            }
        };
        let event_loop = EventLoop::<UserEvent>::with_user_event()
            .build()
            .map_err(|e| e.to_string())?;
        let title = format!("Spelunky - {}", args.room);
        let mut app = App::new(
            preview,
            textures,
            &title,
            size,
            event_loop.create_proxy(),
            input,
        );
        event_loop.run_app(&mut app).map_err(|e| e.to_string())?;
        if let Some(e) = app.error() {
            return Err(e.to_owned());
        }
        // The end state, to compare a recording with its playback.
        let view = app.game().room.view;
        println!("{} frames, view at {},{}", app.ticks(), view.x, view.y);
        if let (Some(path), Some(recording)) = (&args.record, app.into_recording()) {
            std::fs::write(path, recording.to_text())
                .map_err(|e| format!("{}: {e}", path.display()))?;
            println!("recorded {}", path.display());
        }
        Ok(())
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
