//! Sounds, sprites, backgrounds, paths, scripts, fonts, timelines, triggers, constants, include files and
//! game information.

use crate::error::ErrorKind;
use crate::object::{Action, read_actions};
use crate::reader::{Reader, Result};

/// Reads a section of zlib-compressed resources. Each block starts with an "exists" flag (deleted slots
/// keep their id), then the name and a last-changed timestamp, then `body`.
pub(crate) fn read_section<T>(
    r: &mut Reader<'_>,
    kind: &'static str,
    section_version: i32,
    mut body: impl FnMut(&mut Reader<'_>, String) -> Result<T>,
) -> Result<Vec<Option<T>>> {
    r.version(kind, section_version)?;
    let n = r.count(4)?;
    let mut out = Vec::new();
    for i in 0..n {
        let block = r.inflate(format!("{kind}[{i}]"))?;
        let s = &mut block.reader();
        out.push(if s.bool()? {
            let name = s.string()?;
            s.f64()?; // last changed
            Some(body(s, name)?)
        } else {
            None
        });
        s.finish()?;
    }
    Ok(out)
}

/// A 32-bit BGRA image, `width * height * 4` bytes, rows top to bottom.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
}

impl Image {
    /// Alpha of the pixel at (x, y). The caller keeps (x, y) inside the image.
    pub fn alpha(&self, x: u32, y: u32) -> u8 {
        self.bgra[(y as usize * self.width as usize + x as usize) * 4 + 3]
    }
}

fn read_image(r: &mut Reader<'_>) -> Result<Image> {
    r.version("image", 800)?;
    let width = r.u32()?;
    let height = r.u32()?;
    if width == 0 || height == 0 {
        // GM writes no length or data for an empty image.
        return Ok(Image {
            width,
            height,
            bgra: Vec::new(),
        });
    }
    let data = r.blob()?;
    // u32 * u32 * 4 can overflow u64, so check rather than wrap.
    let expected = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|n| n.checked_mul(4));
    if expected != Some(data.len() as u64) {
        return Err(r.err(ErrorKind::BadImageSize {
            width,
            height,
            len: data.len(),
        }));
    }
    Ok(Image {
        width,
        height,
        bgra: data.to_vec(),
    })
}

fn enum_value<T>(
    r: &Reader<'_>,
    what: &'static str,
    value: i32,
    f: impl Fn(i32) -> Option<T>,
) -> Result<T> {
    f(value).ok_or_else(|| r.err(ErrorKind::InvalidEnum { what, value }))
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Sound {
    pub name: String,
    /// 0 normal, 1 background music, 2 3D, 3 multimedia player.
    pub kind: i32,
    pub file_type: String,
    pub file_name: String,
    pub data: Option<Vec<u8>>,
    /// Bit flags: chorus, echo, flanger, gargle, reverb.
    pub effects: i32,
    pub volume: f64,
    pub pan: f64,
    pub preload: bool,
}

pub(crate) fn read_sounds(r: &mut Reader<'_>) -> Result<Vec<Option<Sound>>> {
    read_section(r, "sound", 800, |s, name| {
        s.version("sound", 800)?;
        Ok(Sound {
            name,
            kind: s.i32()?,
            file_type: s.string()?,
            file_name: s.string()?,
            data: s.optional_blob()?,
            effects: s.i32()?,
            volume: s.f64()?,
            pan: s.f64()?,
            preload: s.bool()?,
        })
    })
}

/// A sprite's collision shape ("Modify Mask" in the GM 8.0 sprite editor).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MaskShape {
    Precise,
    Rectangle,
    Disk,
    Diamond,
}

/// How a sprite's bounding box is chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BboxMode {
    /// The tight box around pixels whose alpha is above the tolerance.
    Automatic,
    FullImage,
    /// The stored `bbox_*` values.
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Sprite {
    pub name: String,
    pub origin_x: i32,
    pub origin_y: i32,
    pub frames: Vec<Image>,
    pub shape: MaskShape,
    /// Pixels with alpha strictly above this collide. GM's editor limits it to 0..=255.
    pub alpha_tolerance: i32,
    pub separate_masks: bool,
    pub bbox_mode: BboxMode,
    /// The editor's stored box (inclusive). Only meaningful as input when `bbox_mode` is `Manual`.
    pub bbox_left: i32,
    pub bbox_right: i32,
    pub bbox_bottom: i32,
    pub bbox_top: i32,
}

impl Sprite {
    /// Width of the first frame (GM 8.0 requires all frames to share a size).
    pub fn width(&self) -> u32 {
        self.frames.first().map_or(0, |f| f.width)
    }

    pub fn height(&self) -> u32 {
        self.frames.first().map_or(0, |f| f.height)
    }
}

pub(crate) fn read_sprites(r: &mut Reader<'_>) -> Result<Vec<Option<Sprite>>> {
    read_section(r, "sprite", 800, |s, name| {
        s.version("sprite", 800)?;
        let origin_x = s.i32()?;
        let origin_y = s.i32()?;
        let n = s.count(12)?;
        let mut frames = Vec::new();
        for _ in 0..n {
            frames.push(read_image(s)?);
        }
        let shape = s.i32()?;
        let shape = enum_value(s, "mask shape", shape, |v| match v {
            0 => Some(MaskShape::Precise),
            1 => Some(MaskShape::Rectangle),
            2 => Some(MaskShape::Disk),
            3 => Some(MaskShape::Diamond),
            _ => None,
        })?;
        let alpha_tolerance = s.i32()?;
        let separate_masks = s.bool()?;
        let bbox_mode = s.i32()?;
        let bbox_mode = enum_value(s, "bbox mode", bbox_mode, |v| match v {
            0 => Some(BboxMode::Automatic),
            1 => Some(BboxMode::FullImage),
            2 => Some(BboxMode::Manual),
            _ => None,
        })?;
        Ok(Sprite {
            name,
            origin_x,
            origin_y,
            frames,
            shape,
            alpha_tolerance,
            separate_masks,
            bbox_mode,
            bbox_left: s.i32()?,
            bbox_right: s.i32()?,
            bbox_bottom: s.i32()?,
            bbox_top: s.i32()?,
        })
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Background {
    pub name: String,
    pub use_as_tileset: bool,
    pub tile_width: i32,
    pub tile_height: i32,
    pub h_offset: i32,
    pub v_offset: i32,
    pub h_sep: i32,
    pub v_sep: i32,
    pub image: Image,
}

pub(crate) fn read_backgrounds(r: &mut Reader<'_>) -> Result<Vec<Option<Background>>> {
    read_section(r, "background", 800, |s, name| {
        s.version("background", 710)?;
        Ok(Background {
            name,
            use_as_tileset: s.bool()?,
            tile_width: s.i32()?,
            tile_height: s.i32()?,
            h_offset: s.i32()?,
            v_offset: s.i32()?,
            h_sep: s.i32()?,
            v_sep: s.i32()?,
            image: read_image(s)?,
        })
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PathPoint {
    pub x: f64,
    pub y: f64,
    pub speed: f64,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Path {
    pub name: String,
    pub smooth: bool,
    pub closed: bool,
    pub precision: i32,
    pub background_room: i32,
    pub snap_x: i32,
    pub snap_y: i32,
    pub points: Vec<PathPoint>,
}

pub(crate) fn read_paths(r: &mut Reader<'_>) -> Result<Vec<Option<Path>>> {
    read_section(r, "path", 800, |s, name| {
        s.version("path", 530)?;
        let smooth = s.bool()?;
        let closed = s.bool()?;
        let precision = s.i32()?;
        let background_room = s.i32()?;
        let snap_x = s.i32()?;
        let snap_y = s.i32()?;
        let n = s.count(24)?;
        let mut points = Vec::new();
        for _ in 0..n {
            points.push(PathPoint {
                x: s.f64()?,
                y: s.f64()?,
                speed: s.f64()?,
            });
        }
        Ok(Path {
            name,
            smooth,
            closed,
            precision,
            background_room,
            snap_x,
            snap_y,
            points,
        })
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Script {
    pub name: String,
    /// GML source, verbatim (CRLF line endings, as the editor saved it).
    pub code: String,
}

pub(crate) fn read_scripts(r: &mut Reader<'_>) -> Result<Vec<Option<Script>>> {
    read_section(r, "script", 800, |s, name| {
        s.version("script", 800)?;
        Ok(Script {
            name,
            code: s.string()?,
        })
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Font {
    pub name: String,
    pub font_name: String,
    pub size: i32,
    pub bold: bool,
    pub italic: bool,
    pub range_min: u16,
    pub charset: u8,
    pub antialias: u8,
    pub range_max: i32,
}

pub(crate) fn read_fonts(r: &mut Reader<'_>) -> Result<Vec<Option<Font>>> {
    read_section(r, "font", 800, |s, name| {
        s.version("font", 800)?;
        Ok(Font {
            name,
            font_name: s.string()?,
            size: s.i32()?,
            bold: s.bool()?,
            italic: s.bool()?,
            range_min: s.u16()?,
            charset: s.u8()?,
            antialias: s.u8()?,
            range_max: s.i32()?,
        })
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Moment {
    pub step: i32,
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Timeline {
    pub name: String,
    pub moments: Vec<Moment>,
}

pub(crate) fn read_timelines(r: &mut Reader<'_>) -> Result<Vec<Option<Timeline>>> {
    read_section(r, "timeline", 800, |s, name| {
        s.version("timeline", 500)?;
        let n = s.count(12)?;
        let mut moments = Vec::new();
        for _ in 0..n {
            moments.push(Moment {
                step: s.i32()?,
                actions: read_actions(s)?,
            });
        }
        Ok(Timeline { name, moments })
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Trigger {
    pub name: String,
    pub condition: String,
    pub check_moment: i32,
    pub constant_name: String,
}

pub(crate) fn read_triggers(r: &mut Reader<'_>) -> Result<Vec<Option<Trigger>>> {
    r.version("triggers", 800)?;
    let n = r.count(4)?;
    let mut out = Vec::new();
    for i in 0..n {
        let block = r.inflate(format!("trigger[{i}]"))?;
        let s = &mut block.reader();
        out.push(if s.bool()? {
            s.version("trigger", 800)?;
            Some(Trigger {
                name: s.string()?,
                condition: s.string()?,
                check_moment: s.i32()?,
                constant_name: s.string()?,
            })
        } else {
            None
        });
        s.finish()?;
    }
    r.f64()?; // last changed
    Ok(out)
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Constant {
    pub name: String,
    pub value: String,
}

pub(crate) fn read_constants(r: &mut Reader<'_>) -> Result<Vec<Constant>> {
    r.version("constants", 800)?;
    let n = r.count(8)?;
    let mut out = Vec::new();
    for _ in 0..n {
        out.push(Constant {
            name: r.string()?,
            value: r.string()?,
        });
    }
    r.f64()?; // last changed
    Ok(out)
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IncludeFile {
    pub file_name: String,
    pub file_path: String,
    pub original: bool,
    pub size: i32,
    pub data: Option<Vec<u8>>,
    pub export: i32,
    pub export_folder: String,
    pub overwrite: bool,
    pub free_memory: bool,
    pub remove_at_end: bool,
}

pub(crate) fn read_includes(r: &mut Reader<'_>) -> Result<Vec<IncludeFile>> {
    r.version("include files", 800)?;
    let n = r.count(4)?;
    let mut out = Vec::new();
    for i in 0..n {
        let block = r.inflate(format!("include[{i}]"))?;
        let s = &mut block.reader();
        s.f64()?; // last changed
        s.version("include file", 800)?;
        out.push(IncludeFile {
            file_name: s.string()?,
            file_path: s.string()?,
            original: s.bool()?,
            size: s.i32()?,
            data: s.optional_blob()?,
            export: s.i32()?,
            export_folder: s.string()?,
            overwrite: s.bool()?,
            free_memory: s.bool()?,
            remove_at_end: s.bool()?,
        });
        s.finish()?;
    }
    Ok(out)
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GameInformation {
    pub background_color: i32,
    pub separate_window: bool,
    pub caption: String,
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
    pub show_border: bool,
    pub allow_resize: bool,
    pub stay_on_top: bool,
    pub pause_game: bool,
    pub text_rtf: String,
}

pub(crate) fn read_game_information(r: &mut Reader<'_>) -> Result<GameInformation> {
    r.version("game information", 800)?;
    let block = r.inflate("game information")?;
    let s = &mut block.reader();
    let mut info = GameInformation {
        background_color: s.i32()?,
        separate_window: s.bool()?,
        caption: s.string()?,
        left: s.i32()?,
        top: s.i32()?,
        width: s.i32()?,
        height: s.i32()?,
        show_border: s.bool()?,
        allow_resize: s.bool()?,
        stay_on_top: s.bool()?,
        pause_game: s.bool()?,
        text_rtf: String::new(),
    };
    s.f64()?; // last changed
    info.text_rtf = s.string()?;
    s.finish()?;
    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image_bytes(width: u32, height: u32, len: u32) -> Vec<u8> {
        [800, width, height, len]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect()
    }

    #[test]
    fn image_size_mismatch_is_an_error_even_when_the_size_overflows() {
        for (w, h) in [(u32::MAX, u32::MAX), (1 << 31, 1 << 31), (2, 2)] {
            let bytes = image_bytes(w, h, 0);
            let err = read_image(&mut Reader::new(&bytes, "t")).unwrap_err();
            assert!(
                matches!(err.kind, ErrorKind::BadImageSize { .. }),
                "{w}x{h}"
            );
        }
        let mut bytes = image_bytes(1, 1, 4);
        bytes.extend([1, 2, 3, 4]);
        assert_eq!(
            read_image(&mut Reader::new(&bytes, "t")).unwrap().bgra,
            [1, 2, 3, 4]
        );
    }
}
