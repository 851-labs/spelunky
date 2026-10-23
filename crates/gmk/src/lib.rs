//! Reader for GameMaker 8.0 `.gmk` project files.
//!
//! [`parse`] turns the bytes of a `.gmk` into a [`Gmk`]: every section of the file, typed. Resource lists
//! are indexed by GM resource id, with `None` for deleted slots, so ids in GML and in object, room and
//! action data index them directly.
//!
//! The format follows LateralGM's `GmFileReader.java` and the throwaway Python parser in
//! `tools/inventory-py/gmk_parse.py`, which was checked against Spelunky Classic 1.1's `spelunky.gmk`.
//! Only GM 8.0 files (version 800) are read; any other version is an error.
//!
//! The reader takes a byte slice and doesn't touch the filesystem, so it works on wasm. Malformed input
//! returns an [`Error`], never panics: every read is bounds-checked, counts are checked against the bytes
//! left before anything is allocated for them, zlib blocks inflate to at most [`MAX_INFLATED_BLOCK`]
//! bytes, and the resource tree nests at most [`MAX_TREE_DEPTH`] deep.
//!
//! Collision masks aren't stored in a `.gmk`; [`Sprite::collision_masks`] computes them the way GM does
//! when it builds a game (see the [`mask`] module for the rules and how they were checked).

mod error;
pub mod mask;
mod object;
mod reader;
mod resources;
mod room;
mod settings;
mod tree;

pub use error::{Error, ErrorKind};
pub use mask::{Bbox, CollisionMask, MaskError};
pub use object::{Action, ActionKind, Arg, ArgKind, Event, EventType, ExecType, Object};
pub use reader::decode_cp1252;
pub use resources::{
    Background, BboxMode, Constant, Font, GameInformation, Image, IncludeFile, MaskShape, Moment,
    Path, PathPoint, Script, Sound, Sprite, Timeline, Trigger,
};
pub use room::{Instance, Room, RoomBackground, RoomEditorSettings, Tile, View};
pub use settings::Settings;
pub use tree::TreeNode;

use reader::Reader;

/// Upper bound on the inflated size of one zlib block. Spelunky's largest is under 1 MiB.
pub const MAX_INFLATED_BLOCK: usize = 64 << 20;

/// Upper bound on resource-tree nesting.
pub const MAX_TREE_DEPTH: usize = 64;

const MAGIC: i32 = 1234321;

/// A whole GM 8.0 project.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Gmk {
    pub game_id: i32,
    pub guid: [u8; 16],
    pub settings: Settings,
    pub triggers: Vec<Option<Trigger>>,
    pub constants: Vec<Constant>,
    pub sounds: Vec<Option<Sound>>,
    pub sprites: Vec<Option<Sprite>>,
    pub backgrounds: Vec<Option<Background>>,
    pub paths: Vec<Option<Path>>,
    pub scripts: Vec<Option<Script>>,
    pub fonts: Vec<Option<Font>>,
    pub timelines: Vec<Option<Timeline>>,
    pub objects: Vec<Option<Object>>,
    pub rooms: Vec<Option<Room>>,
    /// The editor's counters for the next instance and tile ids.
    pub last_instance_id: i32,
    pub last_tile_id: i32,
    pub include_files: Vec<IncludeFile>,
    pub extension_packages: Vec<String>,
    pub game_information: GameInformation,
    pub library_creation_code: Vec<String>,
    /// Room ids in game order; the first is the starting room.
    pub room_order: Vec<i32>,
    /// The twelve top-level sections of the editor's resource tree.
    pub resource_tree: Vec<TreeNode>,
}

/// Parses a GM 8.0 `.gmk` file.
pub fn parse(data: &[u8]) -> Result<Gmk, Error> {
    let r = &mut Reader::new(data, "gmk");
    let magic = r.i32()?;
    if magic != MAGIC {
        return Err(Error {
            context: "gmk".into(),
            offset: 0,
            kind: ErrorKind::BadMagic(magic),
        });
    }
    r.version("gmk", 800)?;
    let game_id = r.i32()?;
    let mut guid = [0; 16];
    guid.copy_from_slice(r.bytes(16)?);
    let settings = settings::read(r)?;
    let triggers = resources::read_triggers(r)?;
    let constants = resources::read_constants(r)?;
    let sounds = resources::read_sounds(r)?;
    let sprites = resources::read_sprites(r)?;
    let backgrounds = resources::read_backgrounds(r)?;
    let paths = resources::read_paths(r)?;
    let scripts = resources::read_scripts(r)?;
    let fonts = resources::read_fonts(r)?;
    let timelines = resources::read_timelines(r)?;
    let objects = object::read_objects(r)?;
    let rooms = room::read_rooms(r)?;
    let last_instance_id = r.i32()?;
    let last_tile_id = r.i32()?;
    let include_files = resources::read_includes(r)?;
    r.version("extension packages", 700)?;
    let n = r.count(4)?;
    let mut extension_packages = Vec::new();
    for _ in 0..n {
        extension_packages.push(r.string()?);
    }
    let game_information = resources::read_game_information(r)?;
    r.version("library creation code", 500)?;
    let n = r.count(4)?;
    let mut library_creation_code = Vec::new();
    for _ in 0..n {
        library_creation_code.push(r.string()?);
    }
    r.version("room order", 700)?;
    let n = r.count(4)?;
    let mut room_order = Vec::new();
    for _ in 0..n {
        room_order.push(r.i32()?);
    }
    let resource_tree = tree::read_tree(r)?;
    r.finish()?;
    Ok(Gmk {
        game_id,
        guid,
        settings,
        triggers,
        constants,
        sounds,
        sprites,
        backgrounds,
        paths,
        scripts,
        fonts,
        timelines,
        objects,
        rooms,
        last_instance_id,
        last_tile_id,
        include_files,
        extension_packages,
        game_information,
        library_creation_code,
        room_order,
        resource_tree,
    })
}

/// Looks up a defined resource by id: `None` for -1, out-of-range and deleted ids.
pub fn get<T>(list: &[Option<T>], id: i32) -> Option<&T> {
    usize::try_from(id)
        .ok()
        .and_then(|i| list.get(i))
        .and_then(Option::as_ref)
}
