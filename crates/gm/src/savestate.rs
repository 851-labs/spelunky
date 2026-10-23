//! Savestates on disk (runtime.md section 11). In memory, a savestate is just a clone of the world; this
//! turns one into bytes and back.
//!
//! # Format
//!
//! `SPLKSAVE`, a little-endian `u32` format version ([`SAVESTATE_VERSION`]), a little-endian `u64`
//! fingerprint of the program and assets, then the state as [postcard](https://docs.rs/postcard) (serde
//! derives on the runtime's types; reals as their exact bits). Postcard because the asset bundle already
//! uses it: compact, fast, and it works on wasm.
//!
//! The state is everything that affects the simulation: instances (every field, user variables included)
//! and all the instance lists in their current order, globals, the room (views, backgrounds, tiles), the
//! views `room_set_view` gave other rooms, the RNG, input (held keys and this frame's presses),
//! `last_instance_id` and `last_tile_id`, the pending room change, whether the game ended, the frame
//! counter, the window size, the drawing state and settings (target, view, blend; colour, alpha, font,
//! fonts made at run time, cursor), and the host (files, highscores, and the game's [`Extension`] as its own
//! bytes: for Spelunky, the audio model's state). Left out, because they aren't state: the program and
//! assets (shared, re-attached on load and checked against the fingerprint), the collision index (a cache,
//! rebuilt on demand, as a clone does), the GML cursor (only for error reports) and the draw list (only lent
//! during a tick).
//!
//! Bump [`SAVESTATE_VERSION`] whenever the encoding of any saved type changes.

use crate::draw_builtins::DrawSettings;
use crate::host::{Extension, ExtensionError, HIGHSCORE_ENTRIES, Host};
use crate::input::InputState;
use crate::list::InstanceList;
use crate::program::{ObjectId, Program};
use crate::rng::Rng;
use crate::room::{RoomState, VIEWS, View};
use crate::var::Slot;
use crate::world::{Assets, SceneChange, World};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

const MAGIC: [u8; 8] = *b"SPLKSAVE";

/// The savestate format version. Savestates of another version are rejected.
pub const SAVESTATE_VERSION: u32 = 3;

/// Why a savestate can't be loaded.
#[derive(Debug)]
pub enum SavestateError {
    /// Not a savestate.
    BadMagic,
    /// A savestate of another format version.
    WrongVersion(u32),
    /// Made with another program or other assets (a different build of the game).
    WrongProgram,
    /// Corrupt or truncated.
    Decode(postcard::Error),
    /// The game's host state ([`Host::ext`]) is missing, unexpected, or rejected its bytes.
    Extension(ExtensionError),
}

impl fmt::Display for SavestateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SavestateError::BadMagic => write!(f, "not a savestate"),
            SavestateError::WrongVersion(v) => write!(
                f,
                "savestate format version {v}, but this build reads version {SAVESTATE_VERSION}"
            ),
            SavestateError::WrongProgram => write!(
                f,
                "the savestate was made by a different build of the game (program or assets differ)"
            ),
            SavestateError::Decode(e) => write!(f, "corrupt savestate: {e}"),
            SavestateError::Extension(e) => write!(f, "bad savestate: {e}"),
        }
    }
}

impl std::error::Error for SavestateError {}

/// The saved part of a [`World`]. Borrowed when saving, owned when loading.
#[derive(Serialize, Deserialize)]
struct Saved<'a> {
    insts: Cow<'a, InstanceList>,
    globals: Cow<'a, [Slot]>,
    room: Cow<'a, RoomState>,
    room_views: Cow<'a, BTreeMap<i32, [View; VIEWS]>>,
    rng: Rng,
    input: Cow<'a, InputState>,
    last_instance_id: i32,
    last_tile_id: i32,
    pending: Option<SceneChange>,
    ended: bool,
    presented: bool,
    frames: u64,
    window: (u32, u32),
    view_current: usize,
    draw_state: draw::DrawState,
    draw_settings: Cow<'a, DrawSettings>,
    host: Cow<'a, Host>,
    /// [`Host::ext`], as [`Extension::save`] encoded it.
    ext: Option<Vec<u8>>,
}

impl World {
    /// The world as a savestate file (see the [module docs](self) for what it holds).
    pub fn save_state(&self) -> Vec<u8> {
        // Destructured so that a new field can't be forgotten: it has to be saved or skipped on purpose.
        let World {
            program,
            assets,
            insts,
            globals,
            room,
            room_views,
            rng,
            input,
            last_instance_id,
            last_tile_id,
            pending,
            ended,
            presented,
            frames,
            window,
            view_current,
            draw_state,
            draw_settings,
            host,
            // Where GML last ran: only for error reports, and set again before any code runs.
            cursor: _,
            // Only lent to the world during `step_drawing`, so always `None` here.
            draw: _,
        } = self;
        let saved = Saved {
            insts: Cow::Borrowed(insts),
            globals: Cow::Borrowed(globals),
            room: Cow::Borrowed(room),
            room_views: Cow::Borrowed(room_views),
            rng: *rng,
            input: Cow::Borrowed(input),
            last_instance_id: *last_instance_id,
            last_tile_id: *last_tile_id,
            pending: *pending,
            ended: *ended,
            presented: *presented,
            frames: *frames,
            window: *window,
            view_current: *view_current,
            draw_state: *draw_state,
            draw_settings: Cow::Borrowed(draw_settings),
            host: Cow::Borrowed(host),
            ext: host.ext.as_ref().map(|e| e.save()),
        };
        let mut out = MAGIC.to_vec();
        out.extend_from_slice(&SAVESTATE_VERSION.to_le_bytes());
        let print = fingerprint(program, assets, draw_settings.font_base());
        out.extend_from_slice(&print.to_le_bytes());
        // Serializing plain data into a Vec can't fail.
        postcard::to_extend(&saved, out).expect("postcard encoding into a Vec")
    }

    /// A world from [`save_state`](Self::save_state)'s bytes, running `program` with `assets` and
    /// `font_slots` (as [`World::new`] takes them), which must be the ones it was saved with. Bytes that
    /// decode but don't describe a consistent world are rejected as [`SavestateError::Decode`], so nothing
    /// panics on them later. For a world without a [`Host::ext`]; see
    /// [`load_state_with`](Self::load_state_with).
    pub fn load_state(
        program: &'static Program,
        assets: Arc<Assets>,
        font_slots: usize,
        bytes: &[u8],
    ) -> Result<World, SavestateError> {
        World::load_state_with(program, assets, font_slots, bytes, None)
    }

    /// [`load_state`](Self::load_state) for a world with a [`Host::ext`]: `ext` is a fresh one, holding
    /// what isn't state, that [`Extension::load`] fills in. The savestate must have one exactly if `ext`
    /// is given.
    pub fn load_state_with(
        program: &'static Program,
        assets: Arc<Assets>,
        font_slots: usize,
        bytes: &[u8],
        ext: Option<Box<dyn Extension>>,
    ) -> Result<World, SavestateError> {
        let corrupt = || SavestateError::Decode(postcard::Error::DeserializeBadEncoding);
        let rest = bytes.strip_prefix(&MAGIC).ok_or(SavestateError::BadMagic)?;
        let (version, rest) = rest
            .split_first_chunk::<4>()
            .ok_or(SavestateError::BadMagic)?;
        let version = u32::from_le_bytes(*version);
        if version != SAVESTATE_VERSION {
            return Err(SavestateError::WrongVersion(version));
        }
        let (print, rest) = rest.split_first_chunk::<8>().ok_or(SavestateError::Decode(
            postcard::Error::DeserializeUnexpectedEnd,
        ))?;
        if u64::from_le_bytes(*print) != fingerprint(program, &assets, font_slots) {
            return Err(SavestateError::WrongProgram);
        }
        let (saved, rest): (Saved<'static>, _) =
            postcard::take_from_bytes(rest).map_err(SavestateError::Decode)?;
        if !rest.is_empty() || !saved.is_consistent(program, &assets, font_slots) {
            return Err(corrupt());
        }
        let ext = match (ext, saved.ext) {
            (Some(mut ext), Some(bytes)) => {
                ext.load(&bytes).map_err(SavestateError::Extension)?;
                Some(ext)
            }
            (None, None) => None,
            (Some(_), None) => {
                return Err(SavestateError::Extension(ExtensionError::new(
                    "the savestate has no game host state",
                )));
            }
            (None, Some(_)) => {
                return Err(SavestateError::Extension(ExtensionError::new(
                    "the savestate has a game host state, but the loader none",
                )));
            }
        };
        let mut host = saved.host.into_owned();
        host.ext = ext;
        Ok(World {
            program,
            assets,
            insts: saved.insts.into_owned(),
            globals: saved.globals.into_owned().into_boxed_slice(),
            room: saved.room.into_owned(),
            room_views: saved.room_views.into_owned(),
            rng: saved.rng,
            input: saved.input.into_owned(),
            last_instance_id: saved.last_instance_id,
            last_tile_id: saved.last_tile_id,
            pending: saved.pending,
            ended: saved.ended,
            presented: saved.presented,
            frames: saved.frames,
            window: saved.window,
            view_current: saved.view_current,
            draw_state: saved.draw_state,
            draw_settings: saved.draw_settings.into_owned(),
            host,
            cursor: ("", 0),
            draw: Default::default(),
        })
    }
}

impl Saved<'_> {
    /// Whether the decoded state is one a world could be in: every id and index it holds is in range
    /// (instance lists, globals, the room, fonts) and the highscore table is full.
    fn is_consistent(&self, program: &Program, assets: &Assets, font_slots: usize) -> bool {
        self.globals.len() == program.global_count()
            && self.insts.is_consistent(program)
            && self.host.highscores.len() == HIGHSCORE_ENTRIES
            // -1 before the first room.
            && (self.room.id == -1 || assets.room(self.room.id).is_some())
            && self.room.backgrounds.iter().all(|b| b.index == -1 || assets.background(b.index).is_some())
            && self.room.tiles.iter().all(|t| assets.background(t.background).is_some())
            && self.view_current < VIEWS
            && self.draw_settings.is_consistent(assets, font_slots)
            && match self.draw_state.target {
                draw::Target::Screen => true,
                draw::Target::Surface(id) => i32::try_from(id.0)
                    .is_ok_and(|id| self.draw_settings.surface(id).is_some()),
            }
    }
}

/// FNV-1a over length-prefixed items in tagged, counted sections, so nothing can move from one item or
/// section to another and hash the same. The same on every platform and every run.
struct Fnv(u64);

impl Fnv {
    fn new() -> Fnv {
        Fnv(0xcbf2_9ce4_8422_2325)
    }

    fn bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0 ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    fn number(&mut self, n: u64) {
        self.bytes(&n.to_le_bytes());
    }

    fn item(&mut self, bytes: &[u8]) {
        self.number(bytes.len() as u64);
        self.bytes(bytes);
    }

    fn section(&mut self, tag: &str, count: usize) {
        self.item(tag.as_bytes());
        self.number(count as u64);
    }
}

/// What a savestate's ids mean: the program's (cached in it), the assets' (cached in them), and where
/// fonts made at run time are numbered from. A savestate only makes sense with the same ones.
fn fingerprint(program: &Program, assets: &Assets, font_slots: usize) -> u64 {
    let mut h = Fnv::new();
    h.number(program.fingerprint());
    h.number(assets.fingerprint());
    h.section("font_slots", font_slots);
    h.0
}

/// Object names, parents and variable layouts, user variable and global names.
pub(crate) fn program_fingerprint(program: &Program) -> u64 {
    let mut h = Fnv::new();
    h.section("objects", program.object_slots());
    for o in 0..program.object_slots() {
        let o = ObjectId(o as u32);
        h.item(program.object(o).map_or(b"\0", |i| i.name.as_bytes()));
        let parents: Vec<u8> = program
            .parents(o)
            .iter()
            .flat_map(|p| p.0.to_le_bytes())
            .collect();
        h.item(&parents);
        let layout: Vec<u8> = (program.layout(o).vars())
            .flat_map(|(v, slot)| [v.0.to_le_bytes(), (slot as u16).to_le_bytes()])
            .flatten()
            .collect();
        h.item(&layout);
    }
    h.section("vars", program.user_var_count());
    for v in 0..program.user_var_count() {
        h.item(
            program
                .var_name(crate::Var(crate::FIRST_USER_VAR + v as u16))
                .as_bytes(),
        );
    }
    h.section("globals", program.global_count());
    for g in 0..program.global_count() {
        h.item(program.global_name(crate::GVar(g as u16)).as_bytes());
    }
    h.0
}

/// Sprite names and frame counts (fonts are made from sprites, one character per frame), background and
/// room names.
pub(crate) fn assets_fingerprint(assets: &Assets) -> u64 {
    let mut h = Fnv::new();
    h.section("sprites", assets.sprites.len());
    for s in &assets.sprites {
        h.item(s.as_ref().map_or(b"\0", |s| s.name.as_bytes()));
        h.number(s.as_ref().map_or(0, |s| s.frames.len() as u64));
    }
    h.section("backgrounds", assets.backgrounds.len());
    for b in &assets.backgrounds {
        h.item(b.as_ref().map_or(b"\0", |b| b.name.as_bytes()));
    }
    h.section("rooms", assets.rooms.len());
    for r in &assets.rooms {
        h.item(r.as_ref().map_or(b"\0", |r| r.name.as_bytes()));
    }
    h.0
}
