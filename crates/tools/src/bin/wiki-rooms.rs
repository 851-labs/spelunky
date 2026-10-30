//! Draws every room as the wiki's media: a picture of what the room editor placed in it, for its page in the
//! source viewer (851-3692; docs/wiki-media.md).
//!
//! ```sh
//! wiki-rooms --assets assets/ --out wiki/media/rooms
//! ```
//!
//! `scripts/wiki-media` runs it, after copying in the extractor's `rooms/<name>.json`, and it writes
//! `<name>.png` beside each: the room at one game pixel a pixel, its backgrounds, tiles and each instance's
//! sprite at frame 0, at its position. Nothing runs: no creation code, no create event, so a room that builds
//! its level in code (`rOlmec`'s `scrRoomGen5`) shows only what's placed.
//!
//! # Draw order
//!
//! As GM8 draws a room (OpenGMK `game/draw.rs`'s `draw_view`, which the port's `gm::drawing` follows and the
//! oracle confirms): the room's colour if it draws it, the background layers, then the instances and tiles
//! merged by depth, highest first (at equal depth the instance first; each list in its own order, the
//! instances in creation order, the tiles in the room's), then the foreground layers. The room editor's own
//! order isn't something the oracle can run, so this is the runner's at the room's start, with every
//! instance at its object's depth. Unlike the runner, it draws an instance whose object isn't visible, as
//! the editor does: the picture is of what's placed. An object without a sprite draws nothing (the editor
//! draws a question mark).
//!
//! # Determinism
//!
//! The same bundle gives byte-identical output: integer blending, rooms by id, fixed PNG settings.

use std::cmp::Ordering;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use assets::{Background, Bundle, Room};
use tools::{Arg, Args, WIKI_NOTICE as NOTICE};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const USAGE: &str = "usage: wiki-rooms --assets <extracted bundle dir> --out <rooms dir>";

fn main() -> ExitCode {
    match run(std::env::args().skip(1)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("wiki-rooms: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: impl IntoIterator<Item = String>) -> Result<()> {
    let mut args = Args::new(args);
    let (mut assets_dir, mut out) = (None, None);
    while let Some(arg) = args.next() {
        let flag = match arg {
            Arg::Flag(flag) => flag,
            Arg::Positional(p) => return Err(format!("unexpected argument {p:?}\n{USAGE}").into()),
        };
        match flag.name.as_str() {
            "-h" | "--help" => {
                args.switch(&flag)?;
                println!("{USAGE}");
                return Ok(());
            }
            "--assets" => assets_dir = Some(PathBuf::from(args.value(&flag)?)),
            "--out" => out = Some(PathBuf::from(args.value(&flag)?)),
            name => return Err(format!("unexpected argument {name:?}\n{USAGE}").into()),
        }
    }
    let (Some(assets_dir), Some(out)) = (assets_dir, out) else {
        return Err(USAGE.into());
    };
    let read = |path: &Path| fs::read(path).map_err(|e| format!("{}: {e}", path.display()));
    let bundle_path = assets_dir.join(assets::BUNDLE_FILE);
    let bundle = Bundle::from_bytes(&read(&bundle_path)?)
        .map_err(|e| format!("{}: {e} (run scripts/extract first)", bundle_path.display()))?;
    let mut pages = Vec::new();
    for page in &bundle.atlas_pages {
        let path = assets_dir.join(&page.file);
        pages.push(Image::from(tools::decode_png(&read(&path)?)?));
    }
    let mut backgrounds = Vec::new();
    for bg in &bundle.backgrounds {
        backgrounds.push(match bg.as_ref().and_then(|b| b.file.as_ref()) {
            Some(file) => Some(Image::from(tools::decode_png(&read(
                &assets_dir.join(file),
            )?)?)),
            None => None,
        });
    }
    let art = Art {
        bundle: &bundle,
        pages: &pages,
        backgrounds: &backgrounds,
    };
    fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut written = 0;
    for room in bundle.rooms.iter().flatten() {
        let png = encode(&art.room(room)?)?;
        let path = out.join(format!("{}.png", room.name));
        fs::write(&path, png).map_err(|e| format!("{}: {e}", path.display()))?;
        written += 1;
    }
    println!("wiki-rooms: drew {written} rooms in {}", out.display());
    Ok(())
}

/// An RGBA image.
#[derive(Debug, Clone, PartialEq)]
struct Image {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

impl From<(u32, u32, Vec<u8>)> for Image {
    fn from((width, height, rgba): (u32, u32, Vec<u8>)) -> Image {
        Image {
            width,
            height,
            rgba,
        }
    }
}

impl Image {
    fn new(width: u32, height: u32) -> Image {
        Image {
            width,
            height,
            rgba: vec![0; width as usize * height as usize * 4],
        }
    }

    fn fill(&mut self, rgba: [u8; 4]) {
        for px in self.rgba.as_chunks_mut::<4>().0 {
            *px = rgba;
        }
    }

    /// Draws the `w`x`h` region of `src` at (`sx`, `sy`) with its top-left corner at (`x`, `y`), blended
    /// over what's there (source over, in integers), scaled by whole or fractional factors by nearest
    /// neighbour, and clipped to the image.
    #[expect(
        clippy::too_many_arguments,
        reason = "a region, a place and a scale, as GM's draw functions take"
    )]
    fn draw(
        &mut self,
        src: &Image,
        sx: i32,
        sy: i32,
        w: i32,
        h: i32,
        x: i32,
        y: i32,
        scale: (f64, f64),
    ) {
        let (dw, dh) = (
            (w as f64 * scale.0).round() as i32,
            (h as f64 * scale.1).round() as i32,
        );
        for dy in 0..dh.max(0) {
            let ty = y + dy;
            if ty < 0 || ty >= self.height as i32 {
                continue;
            }
            let py = sy + (dy as f64 / scale.1) as i32;
            for dx in 0..dw.max(0) {
                let tx = x + dx;
                let px = sx + (dx as f64 / scale.0) as i32;
                if tx < 0
                    || tx >= self.width as i32
                    || px < 0
                    || py < 0
                    || px >= src.width as i32
                    || py >= src.height as i32
                {
                    continue;
                }
                let s = (py as usize * src.width as usize + px as usize) * 4;
                let d = (ty as usize * self.width as usize + tx as usize) * 4;
                let over = blend(
                    src.rgba[s..s + 4].try_into().unwrap(),
                    self.rgba[d..d + 4].try_into().unwrap(),
                );
                self.rgba[d..d + 4].copy_from_slice(&over);
            }
        }
    }
}

/// `src` over `dst`, rounding each channel to the nearest.
fn blend(src: [u8; 4], dst: [u8; 4]) -> [u8; 4] {
    let a = src[3] as u32;
    if a == 255 {
        return src;
    }
    let da = dst[3] as u32 * (255 - a);
    let out_a = a * 255 + da;
    if out_a == 0 {
        return [0; 4];
    }
    let mut out = [0; 4];
    for c in 0..3 {
        out[c] = ((src[c] as u32 * a * 255 + dst[c] as u32 * da + out_a / 2) / out_a) as u8;
    }
    out[3] = ((out_a + 127) / 255) as u8;
    out
}

/// The bundle's images: the atlas pages and each background's.
struct Art<'a> {
    bundle: &'a Bundle,
    pages: &'a [Image],
    backgrounds: &'a [Option<Image>],
}

/// What draws between the layers: an instance (by its index in the room) or a tile, at a depth.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Drawn {
    Instance(usize),
    Tile(usize),
}

/// The room's instances and tiles in the order GM8 draws them: each list sorted by depth, highest first,
/// keeping its own order at equal depths (a stable sort), then merged, a tile first only where the
/// instance's depth is less, so a tie draws the instance (OpenGMK `draw_view`, `gm::drawing`'s).
fn draw_order(instance_depths: &[i32], tile_depths: &[i32]) -> Vec<Drawn> {
    let by_depth = |depths: &[i32]| {
        let mut order: Vec<usize> = (0..depths.len()).collect();
        order.sort_by(|&a, &b| depths[b].cmp(&depths[a]));
        order
    };
    let (insts, tiles) = (by_depth(instance_depths), by_depth(tile_depths));
    let (mut i, mut t) = (0, 0);
    let mut out = Vec::with_capacity(insts.len() + tiles.len());
    while i < insts.len() || t < tiles.len() {
        let tile_first = match (insts.get(i), tiles.get(t)) {
            (Some(&a), Some(&b)) => instance_depths[a].cmp(&tile_depths[b]) == Ordering::Less,
            (None, Some(_)) => true,
            _ => false,
        };
        if tile_first {
            out.push(Drawn::Tile(tiles[t]));
            t += 1;
        } else {
            out.push(Drawn::Instance(insts[i]));
            i += 1;
        }
    }
    out
}

impl Art<'_> {
    fn background(&self, id: i32) -> Option<(&Background, &Image)> {
        let id = usize::try_from(id).ok()?;
        let bg = self.bundle.backgrounds.get(id)?.as_ref()?;
        Some((bg, self.backgrounds.get(id)?.as_ref()?))
    }

    fn room(&self, room: &Room) -> Result<Image> {
        let (w, h) = (room.width.max(0) as u32, room.height.max(0) as u32);
        let mut image = Image::new(w, h);
        if room.draw_background_color {
            // GM's 0x00BBGGRR.
            let c = room.background_color;
            image.fill([c as u8, (c >> 8) as u8, (c >> 16) as u8, 255]);
        }
        self.layers(room, &mut image, false);
        let objects = &self.bundle.objects;
        let object = |id: i32| {
            usize::try_from(id)
                .ok()
                .and_then(|i| objects.get(i)?.as_ref())
        };
        let depths: Vec<i32> = room
            .instances
            .iter()
            .map(|inst| object(inst.object).map_or(0, |o| o.depth))
            .collect();
        let tile_depths: Vec<i32> = room.tiles.iter().map(|tile| tile.depth).collect();
        for drawn in draw_order(&depths, &tile_depths) {
            match drawn {
                Drawn::Instance(i) => {
                    let inst = &room.instances[i];
                    let sprite = object(inst.object)
                        .and_then(|o| usize::try_from(o.sprite).ok())
                        .and_then(|s| self.bundle.sprites.get(s)?.as_ref());
                    let Some(sprite) = sprite else { continue };
                    let Some(frame) = sprite.frames.first() else {
                        continue;
                    };
                    let page = self
                        .pages
                        .get(frame.page as usize)
                        .ok_or("a frame on an atlas page the bundle hasn't got")?;
                    let (x, y) = (inst.x - sprite.origin_x, inst.y - sprite.origin_y);
                    image.draw(
                        page,
                        frame.x as i32,
                        frame.y as i32,
                        frame.width as i32,
                        frame.height as i32,
                        x,
                        y,
                        (1.0, 1.0),
                    );
                }
                Drawn::Tile(i) => {
                    let tile = &room.tiles[i];
                    if let Some((_, src)) = self.background(tile.background) {
                        image.draw(
                            src,
                            tile.bg_x,
                            tile.bg_y,
                            tile.width,
                            tile.height,
                            tile.x,
                            tile.y,
                            (1.0, 1.0),
                        );
                    }
                }
            }
        }
        self.layers(room, &mut image, true);
        Ok(image)
    }

    /// The room's visible background layers, or its foreground ones: each at its place, stretched to the
    /// room or tiled across and down it as the room says.
    fn layers(&self, room: &Room, image: &mut Image, foreground: bool) {
        for layer in room
            .backgrounds
            .iter()
            .filter(|b| b.visible && b.foreground == foreground)
        {
            let Some((_, src)) = self.background(layer.background) else {
                continue;
            };
            let (bw, bh) = (src.width as i32, src.height as i32);
            if bw == 0 || bh == 0 {
                continue;
            }
            let scale = if layer.stretch {
                (
                    room.width as f64 / bw as f64,
                    room.height as f64 / bh as f64,
                )
            } else {
                (1.0, 1.0)
            };
            let (tw, th) = (
                (bw as f64 * scale.0).round() as i32,
                (bh as f64 * scale.1).round() as i32,
            );
            // A tiled layer starts a whole tile left of (or above) the room's edge, at its offset.
            let xs: Vec<i32> = if layer.tile_h {
                (layer.x.rem_euclid(tw) - tw..room.width)
                    .step_by(tw as usize)
                    .collect()
            } else {
                vec![layer.x]
            };
            let ys: Vec<i32> = if layer.tile_v {
                (layer.y.rem_euclid(th) - th..room.height)
                    .step_by(th as usize)
                    .collect()
            } else {
                vec![layer.y]
            };
            for &y in &ys {
                for &x in &xs {
                    image.draw(src, 0, 0, bw, bh, x, y, scale);
                }
            }
        }
    }
}

/// A PNG of the picture, with the copyright notice as a `Copyright` text chunk, as `wiki-sprites`' are.
fn encode(image: &Image) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut encoder = tools::png_encoder(&mut out, image.width, image.height);
    encoder.add_text_chunk("Copyright".to_owned(), NOTICE.to_owned())?;
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&image.rgba)?;
    writer.finish()?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draws_by_depth_highest_first_an_instance_before_a_tile_at_its_depth() {
        // Instances at depths 0, 100, 0; tiles at 1000000, 0, 50.
        let order = draw_order(&[0, 100, 0], &[1_000_000, 0, 50]);
        use Drawn::{Instance as I, Tile as T};
        assert_eq!(order, [T(0), I(1), T(2), I(0), I(2), T(1)]);
    }

    #[test]
    fn blends_source_over() {
        assert_eq!(
            blend([10, 20, 30, 255], [200, 200, 200, 255]),
            [10, 20, 30, 255]
        );
        assert_eq!(
            blend([0, 0, 0, 0], [200, 100, 50, 255]),
            [200, 100, 50, 255]
        );
        assert_eq!(
            blend([255, 0, 0, 128], [0, 0, 255, 255]),
            [128, 0, 127, 255]
        );
        assert_eq!(blend([255, 0, 0, 128], [0, 0, 0, 0]), [255, 0, 0, 128]);
    }

    #[test]
    fn draws_a_region_clipped() {
        let mut src = Image::new(2, 2);
        src.fill([1, 2, 3, 255]);
        let mut dst = Image::new(3, 3);
        dst.draw(&src, 0, 0, 2, 2, 2, -1, (1.0, 1.0));
        let painted: Vec<bool> = dst
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| p[3] == 255)
            .collect();
        assert_eq!(
            painted,
            [false, false, true, false, false, false, false, false, false]
        );
    }
}
