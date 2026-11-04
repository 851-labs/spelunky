//! Makes icons from the game's own pictures: by default the icon in `spelunky.gmk`'s global game settings,
//! which GameMaker gave the original `spelunky.exe` (the native apps'), or with `--sprite NAME` a sprite's
//! first frame cropped to its visible pixels (spelunky.ai's favicon: `scripts/site-icons`).
//!
//! ```sh
//! icon --reference reference/ --icns AppIcon.icns --ico spelunky.ico
//! icon --reference reference/ --sprite sStandLeft --ico favicon.ico --svg icon.svg \
//!     --touch-icon 4a2a1a apple-touch-icon.png
//! ```
//!
//! It's pixel art, so the largest image in the icon (the game's only one is 32x32), or the sprite (the
//! Spelunker's visible pixels are 12x16), is scaled by nearest neighbour, never smoothed, and only by
//! factors that keep its pixels square: to each size, the largest whole factor that fits it, centred on
//! transparency where it doesn't fill it, and for the icon's 16, half.
//! `--icns` writes a macOS icon (PNG entries, 16 to 1024 pixels), `--ico` a Windows one (PNG entries, 16
//! to 256; Windows scales its in-between sizes from those), both made with no platform tools, so any
//! machine can package for any other. `--png SIZE OUT` writes one size as a PNG. `--svg OUT` writes the
//! image, squared with transparency, as an SVG of one square per pixel, which a browser draws crisply at any
//! size. `--touch-icon RRGGBB OUT` writes Apple's touch icon: 180 pixels and opaque, as iOS fills
//! transparency with black, so the image on that colour, scaled by the largest whole factor that keeps it
//! clear of iOS's rounded corners (180 isn't a multiple of 16 or 32). The output is the same on every run.
//!
//! Every PNG it writes (alone or in an `.icns` or `.ico`) carries the Spelunky notice in a `Copyright` text
//! chunk, and the SVG in a comment, as the wiki's media do (`docs/wiki-license.md`): they're the game's art.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use tools::{Arg, Args, WIKI_NOTICE, bgra_to_rgba, png_encoder};
use transpile::reference::{Program, load_program};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const USAGE: &str = "usage: icon --reference <dir with the reference zips> [--sprite NAME] [--icns OUT.icns] \
    [--ico OUT.ico] [--png SIZE OUT.png] [--svg OUT.svg] [--touch-icon RRGGBB OUT.png]";

/// `.icns` entries: a type and its size in pixels. Each is a PNG.
const ICNS: [(&[u8; 4], u32); 10] = [
    (b"icp4", 16),
    (b"icp5", 32),
    (b"ic11", 32),
    (b"icp6", 64),
    (b"ic12", 64),
    (b"ic07", 128),
    (b"ic08", 256),
    (b"ic13", 256),
    (b"ic09", 512),
    (b"ic14", 512),
];
/// The 1024-pixel entry (512@2x), last.
const ICNS_1024: &[u8; 4] = b"ic10";

/// `.ico` sizes: Explorer's small icons to its largest, those the 32-pixel original (and the 16-pixel-tall
/// Spelunker) scales to evenly.
const ICO: [u32; 5] = [16, 32, 64, 128, 256];

/// Apple's touch icon: its size for the iPhone's home screen (iOS scales it for the others), and the radius
/// of the corners iOS rounds it to, a little more than their 40 or so, as iOS's curves are continuous, not
/// circular, so they begin further along the edges.
const TOUCH_ICON: u32 = 180;
const TOUCH_ICON_CORNER: u32 = 45;

fn main() -> ExitCode {
    match run(std::env::args().skip(1)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("icon: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: impl IntoIterator<Item = String>) -> Result<()> {
    let mut reference = None;
    let mut sprite = None;
    let mut icns = None;
    let mut ico = None;
    let mut pngs = Vec::new();
    let mut svg = None;
    let mut touch_icon = None;
    let mut args = Args::new(args);
    while let Some(arg) = args.next() {
        let flag = match arg {
            Arg::Positional(other) => return Err(format!("unexpected {other}\n{USAGE}").into()),
            Arg::Flag(flag) => flag,
        };
        match flag.name.as_str() {
            "--reference" => reference = Some(PathBuf::from(args.value(&flag)?)),
            "--sprite" => sprite = Some(args.value(&flag)?),
            "--icns" => icns = Some(PathBuf::from(args.value(&flag)?)),
            "--ico" => ico = Some(PathBuf::from(args.value(&flag)?)),
            "--png" => {
                let size: u32 = args.number(&flag)?;
                if !(1..=1024).contains(&size) {
                    return Err(format!("bad --png size {size}").into());
                }
                let Some(Arg::Positional(out)) = args.next() else {
                    return Err("--png needs a size and a file".into());
                };
                pngs.push((size, PathBuf::from(out)));
            }
            "--svg" => svg = Some(PathBuf::from(args.value(&flag)?)),
            "--touch-icon" => {
                let colour = args.value(&flag)?;
                let background = parse_colour(&colour)
                    .ok_or_else(|| format!("bad --touch-icon colour {colour} (RRGGBB)"))?;
                let Some(Arg::Positional(out)) = args.next() else {
                    return Err("--touch-icon needs a colour and a file".into());
                };
                touch_icon = Some((background, PathBuf::from(out)));
            }
            "-h" | "--help" => return Err(USAGE.into()),
            other => return Err(format!("unknown option {other}\n{USAGE}").into()),
        }
    }
    let reference = reference.ok_or(USAGE)?;
    if icns.is_none() && ico.is_none() && pngs.is_empty() && svg.is_none() && touch_icon.is_none() {
        return Err(format!("nothing to write\n{USAGE}").into());
    }
    let gmk = load_program(&reference.join("spelunky_1_1_src.zip"), Program::Game);
    let source = match sprite {
        Some(name) => sprite_image(&gmk, &name)?,
        None => largest_image(&gmk.settings.icon)?,
    };
    if let Some(out) = icns {
        write(&out, &icns_file(&source)?)?;
    }
    if let Some(out) = ico {
        write(&out, &ico_file(&source)?)?;
    }
    for (size, out) in pngs {
        write(&out, &png_of(&source.at(size)?)?)?;
    }
    if let Some(out) = svg {
        let side = source.width.max(source.height);
        write(&out, svg_of(&source.framed(side, 0)?).as_bytes())?;
    }
    if let Some((background, out)) = touch_icon {
        write(&out, &png_of(&touch_icon_of(&source, background)?)?)?;
    }
    Ok(())
}

fn parse_colour(hex: &str) -> Option<[u8; 3]> {
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    Some([byte(0)?, byte(2)?, byte(4)?])
}

fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()).into())
}

/// A square-or-not RGBA image.
struct Image {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

impl Image {
    /// The image at `size` x `size`: scaled by the largest whole factor that fits and centred on transparency
    /// (`framed`), or, smaller than a square image, by nearest neighbour (the game's 32-pixel icon's 16).
    fn at(&self, size: u32) -> Result<Image> {
        if size < self.width.max(self.height) && self.width == self.height {
            Ok(self.scaled(size))
        } else {
            self.framed(size, 0)
        }
    }

    /// A square image scaled to `size` x `size` by nearest neighbour.
    fn scaled(&self, size: u32) -> Image {
        let mut rgba = Vec::with_capacity((size * size * 4) as usize);
        for y in 0..size {
            let sy = (y * self.height / size) as usize;
            for x in 0..size {
                let sx = (x * self.width / size) as usize;
                let i = (sy * self.width as usize + sx) * 4;
                rgba.extend_from_slice(&self.rgba[i..i + 4]);
            }
        }
        Image {
            width: size,
            height: size,
            rgba,
        }
    }

    /// The image on a transparent `size` x `size` canvas, scaled by the largest whole factor that leaves at
    /// least `margin` pixels each side, and centred.
    fn framed(&self, size: u32, margin: u32) -> Result<Image> {
        let factor = size.saturating_sub(2 * margin) / self.width.max(self.height);
        if factor == 0 {
            return Err(format!(
                "a {}x{} image doesn't fit {size} pixels",
                self.width, self.height
            )
            .into());
        }
        let (left, top) = (
            (size - self.width * factor) / 2,
            (size - self.height * factor) / 2,
        );
        let mut rgba = vec![0; (size * size * 4) as usize];
        for y in 0..self.height * factor {
            for x in 0..self.width * factor {
                let src = (((y / factor) * self.width + x / factor) * 4) as usize;
                let dst = (((top + y) * size + left + x) * 4) as usize;
                rgba[dst..dst + 4].copy_from_slice(&self.rgba[src..src + 4]);
            }
        }
        Ok(Image {
            width: size,
            height: size,
            rgba,
        })
    }

    /// The image over `background`, opaque.
    fn on(&self, background: [u8; 3]) -> Image {
        let mut rgba = Vec::with_capacity(self.rgba.len());
        for pixel in self.rgba.as_chunks::<4>().0 {
            let alpha = u32::from(pixel[3]);
            for c in 0..3 {
                let over = u32::from(pixel[c]) * alpha + u32::from(background[c]) * (255 - alpha);
                rgba.push(((over + 127) / 255) as u8);
            }
            rgba.push(255);
        }
        Image {
            width: self.width,
            height: self.height,
            rgba,
        }
    }

    /// Whether every visible pixel's centre is inside the square's corners rounded to `radius`.
    fn clear_of_corners(&self, radius: u32) -> bool {
        // In half pixels, so the centres are whole.
        let (side, radius) = (i64::from(2 * self.width), i64::from(2 * radius));
        let past = |at: i64| (radius - at).max(at - (side - radius)).max(0);
        (0..self.height).all(|y| {
            (0..self.width).all(|x| {
                let visible = self.rgba[((y * self.width + x) * 4 + 3) as usize] != 0;
                let (dx, dy) = (past(i64::from(2 * x + 1)), past(i64::from(2 * y + 1)));
                !visible || dx * dx + dy * dy <= radius * radius
            })
        })
    }

    /// The image cropped to its visible pixels (those not wholly transparent), or none if it has none.
    fn cropped(&self) -> Option<Image> {
        let visible = |x: u32, y: u32| self.rgba[((y * self.width + x) * 4 + 3) as usize] != 0;
        let (mut left, mut top, mut right, mut bottom) = (self.width, self.height, 0, 0);
        for y in 0..self.height {
            for x in 0..self.width {
                if visible(x, y) {
                    (left, top) = (left.min(x), top.min(y));
                    (right, bottom) = (right.max(x + 1), bottom.max(y + 1));
                }
            }
        }
        if left >= right {
            return None;
        }
        let mut rgba = Vec::with_capacity(((right - left) * (bottom - top) * 4) as usize);
        for y in top..bottom {
            let row = ((y * self.width + left) * 4) as usize;
            rgba.extend_from_slice(&self.rgba[row..row + ((right - left) * 4) as usize]);
        }
        Some(Image {
            width: right - left,
            height: bottom - top,
            rgba,
        })
    }
}

/// Apple's touch icon of `source`: on `background`, as large as it can be while iOS's rounded corners clip none
/// of it (the Spelunker's 12x16 is 11 times as large, 132x176, its hat and feet narrow enough to clear them).
fn touch_icon_of(source: &Image, background: [u8; 3]) -> Result<Image> {
    let mut margin = 0;
    loop {
        // framed fails once the margin leaves no room, so this ends.
        let framed = source.framed(TOUCH_ICON, margin)?;
        if framed.clear_of_corners(TOUCH_ICON_CORNER) {
            return Ok(framed.on(background));
        }
        margin += 1;
    }
}

/// The first frame of the sprite called `name`, cropped to its visible pixels, so the icon draws the
/// character and not the frame's margins (the Spelunker's 16x16 has 2 empty columns each side).
fn sprite_image(gmk: &gmk::Gmk, name: &str) -> Result<Image> {
    let sprite = gmk
        .sprites
        .iter()
        .flatten()
        .find(|sprite| sprite.name == name)
        .ok_or_else(|| format!("no sprite {name}"))?;
    let frame = sprite
        .frames
        .first()
        .ok_or_else(|| format!("{name} has no frames"))?;
    Image {
        width: frame.width,
        height: frame.height,
        rgba: bgra_to_rgba(&frame.bgra),
    }
    .cropped()
    .ok_or_else(|| format!("{name}'s first frame has no visible pixels").into())
}

fn u16_at(b: &[u8], at: usize) -> Result<u16> {
    let bytes = b.get(at..at + 2).ok_or("the icon is truncated")?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn u32_at(b: &[u8], at: usize) -> Result<u32> {
    let bytes = b.get(at..at + 4).ok_or("the icon is truncated")?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

/// The ICO's largest image (by area, then colour depth), decoded.
fn largest_image(ico: &[u8]) -> Result<Image> {
    if u16_at(ico, 0)? != 0 || u16_at(ico, 2)? != 1 {
        return Err("the game's icon isn't an ICO".into());
    }
    let count = u16_at(ico, 4)? as usize;
    let mut best: Option<(u64, u16, &[u8])> = None;
    for i in 0..count {
        let entry = 6 + i * 16;
        // 0 means 256.
        let side = |v: u8| if v == 0 { 256 } else { u64::from(v) };
        let w = side(*ico.get(entry).ok_or("the icon is truncated")?);
        let h = side(*ico.get(entry + 1).ok_or("the icon is truncated")?);
        let bpp = u16_at(ico, entry + 6)?;
        let size = u32_at(ico, entry + 8)? as usize;
        let offset = u32_at(ico, entry + 12)? as usize;
        let data = ico
            .get(offset..offset + size)
            .ok_or("an icon image is out of bounds")?;
        if best.is_none_or(|(area, depth, _)| (w * h, bpp) > (area, depth)) {
            best = Some((w * h, bpp, data));
        }
    }
    let (_, _, data) = best.ok_or("the game's icon has no images")?;
    if data.starts_with(b"\x89PNG") {
        decode_png(data)
    } else {
        decode_dib(data)
    }
}

fn decode_png(data: &[u8]) -> Result<Image> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(data));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::ALPHA);
    let mut reader = decoder.read_info()?;
    let size = reader
        .output_buffer_size()
        .ok_or("the icon's PNG is too large")?;
    let mut buf = vec![0; size];
    let info = reader.next_frame(&mut buf)?;
    if info.bit_depth != png::BitDepth::Eight || info.color_type != png::ColorType::Rgba {
        return Err("the icon's PNG isn't 8-bit".into());
    }
    buf.truncate(info.buffer_size());
    Ok(Image {
        width: info.width,
        height: info.height,
        rgba: buf,
    })
}

/// A BMP image as an ICO holds it: a `BITMAPINFOHEADER` whose height counts the colour bitmap and the
/// 1-bit transparency mask after it, both bottom-up, rows padded to 4 bytes.
fn decode_dib(data: &[u8]) -> Result<Image> {
    let header = u32_at(data, 0)? as usize;
    let width = u32_at(data, 4)? as usize;
    let height = u32_at(data, 8)? as usize / 2;
    let bpp = u16_at(data, 14)? as usize;
    if !matches!(bpp, 1 | 4 | 8 | 24 | 32) || width == 0 || height == 0 || width > 256 {
        return Err(format!("unsupported icon bitmap ({width}x{height}, {bpp} bits)").into());
    }
    let palette_len = match u32_at(data, 32)? {
        0 if bpp <= 8 => 1 << bpp,
        n => n as usize,
    };
    let palette_at = header;
    let pixels_at = palette_at + if bpp <= 8 { palette_len * 4 } else { 0 };
    let stride = (width * bpp).div_ceil(32) * 4;
    let mask_at = pixels_at + stride * height;
    let mask_stride = width.div_ceil(32) * 4;
    let byte = |at: usize| data.get(at).copied().ok_or("the icon bitmap is truncated");
    let mut rgba = vec![0; width * height * 4];
    // Only 32-bit images carry alpha; the others rely on the mask.
    let mut any_alpha = false;
    for y in 0..height {
        let row = pixels_at + (height - 1 - y) * stride;
        for x in 0..width {
            let (b, g, r, a) = match bpp {
                32 => (
                    byte(row + x * 4)?,
                    byte(row + x * 4 + 1)?,
                    byte(row + x * 4 + 2)?,
                    byte(row + x * 4 + 3)?,
                ),
                24 => (
                    byte(row + x * 3)?,
                    byte(row + x * 3 + 1)?,
                    byte(row + x * 3 + 2)?,
                    255,
                ),
                _ => {
                    let bit = x * bpp;
                    let shift = 8 - bpp - bit % 8;
                    let index = (byte(row + bit / 8)? as usize >> shift) & ((1 << bpp) - 1);
                    let p = palette_at + index * 4;
                    (byte(p)?, byte(p + 1)?, byte(p + 2)?, 255)
                }
            };
            any_alpha |= bpp == 32 && a != 0;
            rgba[(y * width + x) * 4..][..4].copy_from_slice(&[r, g, b, a]);
        }
    }
    if !any_alpha {
        for y in 0..height {
            let row = mask_at + (height - 1 - y) * mask_stride;
            for x in 0..width {
                let transparent = byte(row + x / 8)? >> (7 - x % 8) & 1 == 1;
                rgba[(y * width + x) * 4 + 3] = if transparent { 0 } else { 255 };
            }
        }
    }
    Ok(Image {
        width: width as u32,
        height: height as u32,
        rgba,
    })
}

/// The image as a PNG, with the notice.
fn png_of(image: &Image) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut encoder = png_encoder(&mut out, image.width, image.height);
    encoder.add_text_chunk("Copyright".to_owned(), WIKI_NOTICE.to_owned())?;
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&image.rgba)?;
    writer.finish()?;
    Ok(out)
}

/// An SVG of the image, a square per pixel: each colour's a path of the rows' runs of it, so a browser draws
/// it crisply at any size (`crispEdges`: no anti-aliasing at the squares' edges).
fn svg_of(image: &Image) -> String {
    let (width, height) = (image.width as usize, image.height as usize);
    let pixel = |x: usize, y: usize| -> [u8; 4] {
        let i = (y * width + x) * 4;
        image.rgba[i..i + 4].try_into().expect("four bytes")
    };
    // By colour, so the output is the same on every run.
    let mut paths: BTreeMap<[u8; 4], String> = BTreeMap::new();
    for y in 0..height {
        let mut x = 0;
        while x < width {
            let colour = pixel(x, y);
            let start = x;
            while x < width && pixel(x, y) == colour {
                x += 1;
            }
            if colour[3] != 0 {
                let run = x - start;
                let path = paths.entry(colour).or_default();
                write!(path, "M{start} {y}h{run}v1h-{run}z").expect("writing to a String");
            }
        }
    }
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" \
         viewBox=\"0 0 {width} {height}\" shape-rendering=\"crispEdges\">\n<!-- {WIKI_NOTICE} -->\n"
    );
    for ([r, g, b, a], path) in paths {
        let opacity = if a == 255 {
            String::new()
        } else {
            format!(" fill-opacity=\"{:.3}\"", f64::from(a) / 255.0)
        };
        writeln!(
            out,
            "<path fill=\"#{r:02x}{g:02x}{b:02x}\"{opacity} d=\"{path}\"/>"
        )
        .expect("writing to a String");
    }
    out.push_str("</svg>\n");
    out
}

/// An `.icns`: the magic and the file's length, then entries of a type, their length and a PNG.
fn icns_file(source: &Image) -> Result<Vec<u8>> {
    let mut body = Vec::new();
    let entries = ICNS.iter().copied().chain([(ICNS_1024, 1024)]);
    for (kind, size) in entries {
        let png = png_of(&source.at(size)?)?;
        body.extend_from_slice(kind);
        body.extend_from_slice(&(png.len() as u32 + 8).to_be_bytes());
        body.extend_from_slice(&png);
    }
    let mut out = b"icns".to_vec();
    out.extend_from_slice(&(body.len() as u32 + 8).to_be_bytes());
    out.extend_from_slice(&body);
    Ok(out)
}

/// An `.ico` of PNG images (Windows Vista on).
fn ico_file(source: &Image) -> Result<Vec<u8>> {
    let pngs = ICO
        .iter()
        .map(|&size| Ok((size, png_of(&source.at(size)?)?)))
        .collect::<Result<Vec<_>>>()?;
    let mut out = Vec::new();
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&(pngs.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * pngs.len();
    for (size, png) in &pngs {
        // 256 is written as 0.
        let side = (*size % 256) as u8;
        out.extend_from_slice(&[side, side, 0, 0]);
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&(png.len() as u32).to_le_bytes());
        out.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += png.len();
    }
    for (_, png) in pngs {
        out.extend_from_slice(&png);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    // A 2x2 image: red and transparent on top, two half-transparent blues below.
    fn image() -> Image {
        Image {
            width: 2,
            height: 2,
            rgba: vec![255, 0, 0, 255, 0, 0, 0, 0, 0, 0, 255, 128, 0, 0, 255, 128],
        }
    }

    #[test]
    fn svg_runs_each_colour() {
        assert_eq!(
            svg_of(&image()),
            format!(
                "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"2\" height=\"2\" viewBox=\"0 0 2 2\" \
             shape-rendering=\"crispEdges\">\n\
             <!-- {WIKI_NOTICE} -->\n\
             <path fill=\"#0000ff\" fill-opacity=\"0.502\" d=\"M0 1h2v1h-2z\"/>\n\
             <path fill=\"#ff0000\" d=\"M0 0h1v1h-1z\"/>\n\
             </svg>\n"
            )
        );
        // A comment can't hold "--".
        assert!(!WIKI_NOTICE.contains("--"));
    }

    #[test]
    fn pngs_carry_the_notice() {
        let png = png_of(&image()).unwrap();
        let info = png::Decoder::new(std::io::Cursor::new(&png))
            .read_info()
            .unwrap();
        let chunks = &info.info().uncompressed_latin1_text;
        assert_eq!(chunks.len(), 1);
        assert_eq!(
            (chunks[0].keyword.as_str(), chunks[0].text.as_str()),
            ("Copyright", WIKI_NOTICE)
        );
        assert_eq!(decode_png(&png).unwrap().rgba, image().rgba);
    }

    #[test]
    fn framed_scales_by_whole_factors_and_centres() {
        // 9 pixels with a margin of 1 leaves 7: a factor of 3, so a 6-pixel image, at 1 from the top left.
        let framed = image().framed(9, 1).unwrap().on([0, 255, 0]);
        let at = |x: u32, y: u32| framed.rgba[((y * 9 + x) * 4) as usize..][..4].to_vec();
        assert_eq!(at(0, 0), [0, 255, 0, 255]);
        assert_eq!(at(1, 1), [255, 0, 0, 255]);
        assert_eq!(at(3, 3), [255, 0, 0, 255]);
        assert_eq!(at(4, 1), [0, 255, 0, 255]);
        assert_eq!(at(1, 4), [0, 127, 128, 255]);
        assert_eq!(at(7, 7), [0, 255, 0, 255]);
        assert!(image().framed(3, 1).is_err());
    }

    #[test]
    fn framed_on_transparency_keeps_the_pixels() {
        // A 2x1 image (red, then half-transparent blue) in 5 pixels: a factor of 2, so 4x2 at (0, 1),
        // transparency around it, and the blue as it was, not on anything.
        let wide = Image {
            width: 2,
            height: 1,
            rgba: vec![255, 0, 0, 255, 0, 0, 255, 128],
        };
        let framed = wide.framed(5, 0).unwrap();
        let at = |x: u32, y: u32| framed.rgba[((y * 5 + x) * 4) as usize..][..4].to_vec();
        assert_eq!(at(0, 0), [0, 0, 0, 0]);
        assert_eq!(at(0, 1), [255, 0, 0, 255]);
        assert_eq!(at(1, 2), [255, 0, 0, 255]);
        assert_eq!(at(2, 1), [0, 0, 255, 128]);
        assert_eq!(at(3, 2), [0, 0, 255, 128]);
        assert_eq!(at(4, 1), [0, 0, 0, 0]);
        assert_eq!(at(0, 3), [0, 0, 0, 0]);
    }

    #[test]
    fn at_fits_by_whole_factors_and_halves_squares() {
        let wide = Image {
            width: 2,
            height: 1,
            rgba: vec![255, 0, 0, 255, 0, 0, 255, 255],
        };
        // 2x1 at 5: framed's 4x2 in a 5x5 square.
        let fitted = wide.at(5).unwrap();
        assert_eq!((fitted.width, fitted.height), (5, 5));
        assert_eq!(fitted.rgba, wide.framed(5, 0).unwrap().rgba);
        // Smaller than a wide image can't keep its pixels square.
        assert!(wide.at(1).is_err());
        // A square one halves by nearest neighbour.
        let half = image().at(1).unwrap();
        assert_eq!((half.width, half.rgba), (1, vec![255, 0, 0, 255]));
    }

    #[test]
    fn corners() {
        // A 10x10 square with its corners rounded to 4: a pixel at (0, 0) is outside them (its centre 4.9
        // from the corner's, at (4, 4)), at (1, 1) or along an edge inside.
        let one_pixel = |x: u32, y: u32| {
            let mut rgba = vec![0; 10 * 10 * 4];
            rgba[((y * 10 + x) * 4 + 3) as usize] = 255;
            Image {
                width: 10,
                height: 10,
                rgba,
            }
        };
        assert!(!one_pixel(0, 0).clear_of_corners(4));
        assert!(!one_pixel(9, 9).clear_of_corners(4));
        assert!(!one_pixel(0, 9).clear_of_corners(4));
        assert!(one_pixel(1, 1).clear_of_corners(4));
        assert!(one_pixel(0, 5).clear_of_corners(4));
        assert!(one_pixel(5, 9).clear_of_corners(4));
        // A square filling it isn't clear; framed down to 6 it is (from (2, 2) to (7, 7)).
        let square = Image {
            width: 2,
            height: 2,
            rgba: vec![255; 16],
        };
        assert!(!square.framed(10, 0).unwrap().clear_of_corners(4));
        assert!(square.framed(10, 2).unwrap().clear_of_corners(4));
    }

    #[test]
    fn cropped_to_the_visible_pixels() {
        // 3x3, visible only at (1, 0) and (2, 1): a 2x2 crop.
        let mut rgba = vec![0; 3 * 3 * 4];
        rgba[4..][..4].copy_from_slice(&[1, 2, 3, 255]);
        rgba[((3 + 2) * 4)..][..4].copy_from_slice(&[4, 5, 6, 1]);
        let image = Image {
            width: 3,
            height: 3,
            rgba,
        };
        let crop = image.cropped().unwrap();
        assert_eq!((crop.width, crop.height), (2, 2));
        assert_eq!(
            crop.rgba,
            [[1, 2, 3, 255], [0; 4], [0; 4], [4, 5, 6, 1]].concat()
        );
        let empty = Image {
            width: 2,
            height: 2,
            rgba: vec![0; 16],
        };
        assert!(empty.cropped().is_none());
    }

    #[test]
    fn colours() {
        assert_eq!(parse_colour("f0c040"), Some([0xf0, 0xc0, 0x40]));
        assert_eq!(parse_colour("#f0c040"), None);
        assert_eq!(parse_colour("f0c04"), None);
    }
}
