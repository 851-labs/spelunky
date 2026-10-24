//! Makes the native apps' icons from the game's own: the icon in `spelunky.gmk`'s global game settings,
//! which GameMaker gave the original `spelunky.exe`.
//!
//! ```sh
//! icon --reference reference/ --icns AppIcon.icns --ico spelunky.ico
//! ```
//!
//! It's pixel art, so the largest image in the icon (the game's only one is 32x32) is scaled by nearest
//! neighbour, never smoothed, and only to sizes that keep its pixels square: whole multiples, and 16, half.
//! `--icns` writes a macOS icon (PNG entries, 16 to 1024 pixels), `--ico` a Windows one (PNG entries, 16
//! to 256; Windows scales its in-between sizes from those), both made with no platform tools, so any
//! machine can package for any other. `--png SIZE OUT` writes one size as a PNG. The output is the same on
//! every run.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use tools::{Arg, Args, encode_png};
use transpile::reference::{Program, load_program};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const USAGE: &str = "usage: icon --reference <dir with the reference zips> [--icns OUT.icns] [--ico OUT.ico] \
    [--png SIZE OUT.png]";

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

/// `.ico` sizes: Explorer's small icons to its largest, those the 32-pixel original scales to evenly.
const ICO: [u32; 5] = [16, 32, 64, 128, 256];

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
    let mut icns = None;
    let mut ico = None;
    let mut pngs = Vec::new();
    let mut args = Args::new(args);
    while let Some(arg) = args.next() {
        let flag = match arg {
            Arg::Positional(other) => return Err(format!("unexpected {other}\n{USAGE}").into()),
            Arg::Flag(flag) => flag,
        };
        match flag.name.as_str() {
            "--reference" => reference = Some(PathBuf::from(args.value(&flag)?)),
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
            "-h" | "--help" => return Err(USAGE.into()),
            other => return Err(format!("unknown option {other}\n{USAGE}").into()),
        }
    }
    let reference = reference.ok_or(USAGE)?;
    if icns.is_none() && ico.is_none() && pngs.is_empty() {
        return Err(format!("nothing to write\n{USAGE}").into());
    }
    let gmk = load_program(&reference.join("spelunky_1_1_src.zip"), Program::Game);
    let source = largest_image(&gmk.settings.icon)?;
    if let Some(out) = icns {
        write(&out, &icns_file(&source)?)?;
    }
    if let Some(out) = ico {
        write(&out, &ico_file(&source)?)?;
    }
    for (size, out) in pngs {
        write(&out, &png_of(&source.scaled(size))?)?;
    }
    Ok(())
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
    /// The image scaled to `size` x `size` by nearest neighbour.
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

fn png_of(image: &Image) -> Result<Vec<u8>> {
    Ok(encode_png(image.width, image.height, &image.rgba)?)
}

/// An `.icns`: the magic and the file's length, then entries of a type, their length and a PNG.
fn icns_file(source: &Image) -> Result<Vec<u8>> {
    let mut body = Vec::new();
    let entries = ICNS.iter().copied().chain([(ICNS_1024, 1024)]);
    for (kind, size) in entries {
        let png = png_of(&source.scaled(size))?;
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
        .map(|&size| Ok((size, png_of(&source.scaled(size))?)))
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
