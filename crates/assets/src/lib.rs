//! The asset bundle that `tools`' `extract` binary writes and the game loads.
//!
//! A bundle is a directory:
//!
//! | Path | Contents |
//! |---|---|
//! | `bundle.bin` | Everything but pixels and audio: a [`Bundle`], encoded by [`Bundle::to_bytes`] |
//! | `atlas/<n>.png` | Sprite frames packed into RGBA pages ([`AtlasPage`], [`AtlasRect`]) |
//! | `backgrounds/<name>.png` | One RGBA image per background, unpacked so it can tile and stretch |
//! | `sounds/<file>` | The game's `sound/` files, byte for byte |
//! | `gml/` | Every script and event as a readable `.gml` file. For people; the game doesn't read it |
//!
//! `bundle.bin` is [postcard](https://docs.rs/postcard) (a compact serde encoding, `no_std`, so it decodes
//! on wasm with no extra work) behind an 8-byte magic and a format version. It holds GM's own data
//! types from the `gmk` crate (objects with their full action lists, rooms, scripts, settings) plus the
//! bundle's sprite and background records. Resource lists are indexed by GM resource id with `None` for
//! deleted slots, so ids found in GML, object and room data index them directly. Decoding takes a few
//! milliseconds; the images are PNG because browsers and the `png` crate both decode it quickly.
//!
//! Bump [`FORMAT_VERSION`] whenever these types or the `gmk` types they include change shape: postcard
//! isn't self-describing, and the version check is what turns a stale bundle into a clear error.

use serde::{Deserialize, Serialize};
use std::fmt;

pub use gmk::{
    Action, ActionKind, Arg, ArgKind, Bbox, BboxMode, Event, EventType, ExecType, MaskShape,
};
pub use gmk::{Instance, Object, Room, RoomBackground, Script, Settings, Tile, View};

/// Name of the metadata file inside a bundle directory.
pub const BUNDLE_FILE: &str = "bundle.bin";

pub const MAGIC: [u8; 8] = *b"SPLKASET";

pub const FORMAT_VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bundle {
    pub settings: Settings,
    /// Room ids in game order; the first is the starting room.
    pub room_order: Vec<i32>,
    pub atlas_pages: Vec<AtlasPage>,
    pub sprites: Vec<Option<Sprite>>,
    pub backgrounds: Vec<Option<Background>>,
    /// Sorted by name.
    pub sounds: Vec<SoundFile>,
    /// GML verbatim (CRLF line endings, as GM saved it).
    pub scripts: Vec<Option<Script>>,
    pub objects: Vec<Option<Object>>,
    pub rooms: Vec<Option<Room>>,
    /// GM's last tile id: `tile_add` numbers new tiles from the next one.
    pub last_tile_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AtlasPage {
    /// Path relative to the bundle directory.
    pub file: String,
    pub width: u32,
    pub height: u32,
}

/// Where a frame sits in the atlas. Every frame has at least one fully transparent pixel of gutter
/// around it, so nearest-neighbour sampling at the edges never picks up a neighbour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AtlasRect {
    pub page: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sprite {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub origin_x: i32,
    pub origin_y: i32,
    pub frames: Vec<AtlasRect>,
    /// The mask settings the masks were built from (informational; the masks are what count).
    pub shape: MaskShape,
    pub bbox_mode: BboxMode,
    pub alpha_tolerance: i32,
    /// Whether `masks` has one mask per frame. If not, it has one mask shared by every frame.
    pub separate_masks: bool,
    pub masks: Vec<Mask>,
}

impl Sprite {
    /// The mask for a frame, following GM's shared-mask rule. `None` only for a sprite without frames.
    pub fn mask(&self, frame: usize) -> Option<&Mask> {
        if self.separate_masks {
            self.masks.get(frame)
        } else {
            self.masks.first()
        }
    }
}

/// A collision mask, as GM 8.0 stores it in a compiled game but packed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mask {
    /// Inclusive. `left > right` (GM's box for a frame with no solid pixels) means empty.
    pub bbox: Bbox,
    pub data: MaskData,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaskData {
    /// Every pixel in the box is solid (all rectangle masks, and precise masks that happen to be full).
    Filled,
    /// One bit per pixel of the box, row-major with no row padding, least significant bit first:
    /// pixel (x, y) is bit `(y - top) * box_width + (x - left)`.
    Bits(Vec<u8>),
}

impl Mask {
    /// Packs a mask from `gmk`, which never has solid pixels outside its box.
    pub fn from_collision_mask(m: &gmk::CollisionMask) -> Mask {
        let b = m.bbox;
        if b.is_empty() {
            return Mask {
                bbox: b,
                data: MaskData::Bits(Vec::new()),
            };
        }
        let mut bits = Vec::new();
        let mut filled = true;
        let mut i = 0usize;
        for y in b.top..=b.bottom {
            for x in b.left..=b.right {
                let solid = m.get(x, y);
                filled &= solid;
                if i.is_multiple_of(8) {
                    bits.push(0);
                }
                if solid {
                    bits[i / 8] |= 1 << (i % 8);
                }
                i += 1;
            }
        }
        Mask {
            bbox: b,
            data: if filled {
                MaskData::Filled
            } else {
                MaskData::Bits(bits)
            },
        }
    }

    /// Whether pixel (x, y) of the frame is solid.
    pub fn get(&self, x: u32, y: u32) -> bool {
        let b = self.bbox;
        if !b.contains(x, y) {
            return false;
        }
        match &self.data {
            MaskData::Filled => true,
            MaskData::Bits(bits) => {
                let i =
                    (y - b.top) as usize * (b.right - b.left + 1) as usize + (x - b.left) as usize;
                bits.get(i / 8)
                    .is_some_and(|byte| byte & (1 << (i % 8)) != 0)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Background {
    pub name: String,
    /// Path relative to the bundle directory, or `None` for an empty (0-pixel) image.
    pub file: Option<String>,
    pub width: u32,
    pub height: u32,
    pub use_as_tileset: bool,
    pub tile_width: i32,
    pub tile_height: i32,
    pub h_offset: i32,
    pub v_offset: i32,
    pub h_sep: i32,
    pub v_sep: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoundFile {
    /// The name the game passes to its sound DLL, relative to `sound/` (e.g. `jump.wav`).
    pub name: String,
    /// Path relative to the bundle directory.
    pub file: String,
}

#[derive(Debug)]
pub enum Error {
    BadMagic,
    /// The bundle was written by a different version of the extractor; re-run `scripts/extract`.
    WrongVersion(u32),
    Decode(postcard::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::BadMagic => write!(f, "not an asset bundle"),
            Error::WrongVersion(v) => {
                write!(
                    f,
                    "asset bundle format {v}, expected {FORMAT_VERSION}; re-run scripts/extract"
                )
            }
            Error::Decode(e) => write!(f, "corrupt asset bundle: {e}"),
        }
    }
}

impl std::error::Error for Error {}

impl Bundle {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = MAGIC.to_vec();
        out.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        // Serializing plain data into a Vec can't fail.
        postcard::to_extend(self, out).expect("postcard encoding into a Vec")
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Bundle, Error> {
        let rest = bytes.strip_prefix(&MAGIC).ok_or(Error::BadMagic)?;
        let (version, rest) = rest.split_first_chunk::<4>().ok_or(Error::BadMagic)?;
        let version = u32::from_le_bytes(*version);
        if version != FORMAT_VERSION {
            return Err(Error::WrongVersion(version));
        }
        let (bundle, rest) = postcard::take_from_bytes(rest).map_err(Error::Decode)?;
        if !rest.is_empty() {
            return Err(Error::Decode(postcard::Error::DeserializeBadEncoding));
        }
        Ok(bundle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_packing_round_trips() {
        let (w, h) = (5, 4);
        let mut solid = vec![false; w * h];
        for (x, y) in [(1, 1), (3, 1), (2, 2), (3, 3)] {
            solid[y * w + x] = true;
        }
        let m = gmk::CollisionMask {
            width: w as u32,
            height: h as u32,
            bbox: Bbox {
                left: 1,
                top: 1,
                right: 3,
                bottom: 3,
            },
            solid: solid.clone(),
        };
        let packed = Mask::from_collision_mask(&m);
        assert!(matches!(&packed.data, MaskData::Bits(b) if b.len() == 2));
        for y in 0..h {
            for x in 0..w {
                assert_eq!(
                    packed.get(x as u32, y as u32),
                    solid[y * w + x],
                    "({x}, {y})"
                );
            }
        }
    }

    #[test]
    fn full_and_empty_masks_pack_small() {
        let full = gmk::CollisionMask {
            width: 2,
            height: 2,
            bbox: Bbox {
                left: 0,
                top: 0,
                right: 1,
                bottom: 1,
            },
            solid: vec![true; 4],
        };
        assert_eq!(Mask::from_collision_mask(&full).data, MaskData::Filled);
        let empty = gmk::CollisionMask {
            width: 2,
            height: 2,
            bbox: Bbox::empty(2, 2),
            solid: vec![false; 4],
        };
        let packed = Mask::from_collision_mask(&empty);
        assert_eq!(packed.data, MaskData::Bits(Vec::new()));
        assert!(!packed.get(0, 0) && !packed.get(1, 1));
    }

    #[test]
    fn bad_headers_are_errors() {
        assert!(matches!(Bundle::from_bytes(b"nope"), Err(Error::BadMagic)));
        let mut bytes = MAGIC.to_vec();
        bytes.extend_from_slice(&99u32.to_le_bytes());
        assert!(matches!(
            Bundle::from_bytes(&bytes),
            Err(Error::WrongVersion(99))
        ));
        bytes.truncate(8);
        bytes.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        bytes.push(0xFF);
        assert!(matches!(Bundle::from_bytes(&bytes), Err(Error::Decode(_))));
    }
}
