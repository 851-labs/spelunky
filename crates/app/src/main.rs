//! The native frontend. Until the game runs, it previews a room's placed state:
//!
//! ```sh
//! scripts/cargo run -p app -- --room rTutorial            # a window; arrow keys pan, shift pans faster
//! scripts/cargo run -p app -- --room rTutorial --view 0,64 --screenshot out.png [--scale 3]
//! ```
//!
//! `--assets DIR` points at the extracted bundle (default `assets`, made by `scripts/extract`).

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
    use winit::event_loop::EventLoop;
    use winit::keyboard::KeyCode;

    struct Args {
        assets: PathBuf,
        room: String,
        view: Option<(i32, i32)>,
        screenshot: Option<PathBuf>,
        scale: u32,
    }

    fn parse_args() -> Result<Args, String> {
        let mut args = Args {
            assets: PathBuf::from("assets"),
            room: "rTutorial".into(),
            view: None,
            screenshot: None,
            scale: 1,
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
                "--scale" => {
                    args.scale = value()?
                        .parse()
                        .ok()
                        .filter(|&s| s >= 1)
                        .ok_or("--scale takes a whole number from 1")?;
                }
                "-h" | "--help" => {
                    return Err(
                        "usage: app [--assets DIR] [--room NAME] [--view X,Y] [--screenshot OUT.png [--scale N]]"
                            .into(),
                    );
                }
                other => return Err(format!("unknown argument {other}")),
            }
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

    /// Pans the preview's view with the arrow keys.
    struct Preview<'a>(RoomPreview<'a>);

    impl Game for Preview<'_> {
        fn tick(&mut self, input: &Input, out: &mut Frame) {
            let speed = if input.is_held(KeyCode::ShiftLeft) || input.is_held(KeyCode::ShiftRight) {
                16
            } else {
                4
            };
            let axis =
                |neg, pos| (i32::from(input.is_held(pos)) - i32::from(input.is_held(neg))) * speed;
            let dx = axis(KeyCode::ArrowLeft, KeyCode::ArrowRight);
            let dy = axis(KeyCode::ArrowUp, KeyCode::ArrowDown);
            let view = self.0.view;
            self.0.move_view(view.x + dx, view.y + dy);
            self.0.frame().replay(out);
        }
    }

    pub fn run() -> Result<(), String> {
        let args = parse_args()?;
        let (bundle, textures) = load(&args.assets)?;
        let mut preview = RoomPreview::new(&bundle, &args.room)?;
        if let Some((x, y)) = args.view {
            preview.move_view(x, y);
        }
        match &args.screenshot {
            Some(out) => screenshot(&preview, textures, out, args.scale),
            None => {
                let event_loop = EventLoop::<UserEvent>::with_user_event()
                    .build()
                    .map_err(|e| e.to_string())?;
                let title = format!("Spelunky - {}", args.room);
                let mut app = App::new(
                    Preview(preview),
                    textures,
                    &title,
                    event_loop.create_proxy(),
                );
                event_loop.run_app(&mut app).map_err(|e| e.to_string())?;
                app.error().map_or(Ok(()), |e| Err(e.to_owned()))
            }
        }
    }

    fn screenshot(
        preview: &RoomPreview,
        textures: Textures,
        out: &Path,
        scale: u32,
    ) -> Result<(), String> {
        let gpu = pollster::block_on(app::gpu::request(app::gpu::instance(), None))?;
        let mut renderer = Renderer::new(gpu.device, gpu.queue, textures);
        let mut frame = Frame::default();
        preview.frame().replay(&mut frame);
        renderer.render(&frame);
        let (w, h, rgba) = renderer.read_screen()?;
        let (w, h, rgba) = upscale(w, h, &rgba, scale);
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
    fn upscale(w: u32, h: u32, rgba: &[u8], scale: u32) -> (u32, u32, Vec<u8>) {
        if scale == 1 {
            return (w, h, rgba.to_vec());
        }
        let (sw, sh) = (w * scale, h * scale);
        let mut out = Vec::with_capacity((sw * sh * 4) as usize);
        for y in 0..sh {
            let row = &rgba[((y / scale) * w * 4) as usize..][..(w * 4) as usize];
            for px in row.as_chunks::<4>().0 {
                for _ in 0..scale {
                    out.extend_from_slice(px);
                }
            }
        }
        (sw, sh, out)
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
