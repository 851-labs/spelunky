//! The world: everything that changes while the game runs, as one plain value (runtime.md section 2).
//! Cloning it is a savestate.

use crate::error::GmlError;
use crate::host::Host;
use crate::input::InputState;
use crate::instance::{Bbox, Instance, State};
use crate::list::{Inst, InstanceList};
use crate::program::Program;
use crate::rng::Rng;
use crate::room::{RoomState, VIEWS, View};
use crate::var::Slot;
use std::collections::BTreeMap;
use std::sync::Arc;

/// The immutable assets a world needs: sprites (frame counts, origins, collision masks), backgrounds
/// (sizes), rooms and the room order. Shared between clones.
#[derive(Clone, Debug)]
pub struct Assets {
    /// By sprite id; `None` for deleted slots.
    pub(crate) sprites: Vec<Option<assets::Sprite>>,
    /// Each sprite's collision box, precomputed: bounding boxes are tested constantly.
    mask_boxes: Vec<Bbox>,
    /// By background id; `None` for deleted slots.
    pub(crate) backgrounds: Vec<Option<assets::Background>>,
    /// By room id; `None` for deleted slots.
    pub(crate) rooms: Vec<Option<assets::Room>>,
    /// Room ids in game order; the first is where the game starts.
    room_order: Vec<i32>,
    /// The settings' "colour outside the room" (`0x00BBGGRR`): what the window is cleared to before the
    /// views are drawn.
    color_outside_room: u32,
    /// GM8's `last_tile_id` at game start: `tile_add` takes the next one.
    last_tile_id: i32,
    /// [`fingerprint`](Self::fingerprint), computed on first use.
    fingerprint: std::sync::OnceLock<u64>,
}

impl Default for Assets {
    fn default() -> Assets {
        Assets::new(Vec::new(), Vec::new())
    }
}

impl Assets {
    /// Sprites and rooms by id, `None` for deleted slots. The room order is every room in id order, and
    /// there are no backgrounds.
    pub fn new(sprites: Vec<Option<assets::Sprite>>, rooms: Vec<Option<assets::Room>>) -> Assets {
        let mask_boxes = sprites
            .iter()
            .map(|s| s.as_ref().map_or(Bbox::default(), sprite_mask_box))
            .collect();
        Assets {
            sprites,
            mask_boxes,
            backgrounds: Vec::new(),
            rooms: Vec::new(),
            room_order: Vec::new(),
            color_outside_room: 0,
            // GM8 numbers tiles from 10000001, as it numbers instances from 100001.
            last_tile_id: 10_000_000,
            fingerprint: std::sync::OnceLock::new(),
        }
        .with_rooms(rooms)
    }

    /// The parts of an extracted bundle the runtime uses.
    pub fn from_bundle(b: assets::Bundle) -> Assets {
        Assets {
            room_order: b.room_order,
            color_outside_room: b.settings.color_outside_room,
            last_tile_id: b.last_tile_id,
            ..Assets::new(b.sprites, b.rooms).with_backgrounds(b.backgrounds)
        }
    }

    /// Replaces the rooms, and the room order with every room in id order.
    pub fn with_rooms(mut self, rooms: Vec<Option<assets::Room>>) -> Assets {
        self.room_order = (0..rooms.len())
            .filter(|&i| rooms[i].is_some())
            .map(|i| i as i32)
            .collect();
        self.rooms = rooms;
        self
    }

    /// Replaces the room order.
    pub fn with_room_order(mut self, room_order: Vec<i32>) -> Assets {
        self.room_order = room_order;
        self
    }

    /// Replaces the backgrounds.
    pub fn with_backgrounds(mut self, backgrounds: Vec<Option<assets::Background>>) -> Assets {
        self.backgrounds = backgrounds;
        self
    }

    /// The sprite with this id, if it exists.
    #[inline]
    pub fn sprite(&self, id: i32) -> Option<&assets::Sprite> {
        self.sprites.get(usize::try_from(id).ok()?)?.as_ref()
    }

    /// A sprite and its collision box (the union of its frame masks' boxes), if it exists.
    #[inline]
    pub fn sprite_with_box(&self, id: i32) -> Option<(&assets::Sprite, Bbox)> {
        let i = usize::try_from(id).ok()?;
        Some((self.sprites.get(i)?.as_ref()?, self.mask_boxes[i]))
    }

    /// GM8's bounding box of an instance as if it were at (x, y): its mask sprite's (`mask_index`, else
    /// `sprite_index`), or [`NO_MASK_BBOX`](crate::NO_MASK_BBOX) without one. The one rule every bounding box
    /// in the runtime comes from.
    pub fn bbox_at(&self, inst: &Instance, x: f64, y: f64) -> Bbox {
        match self.sprite_with_box(inst.mask_sprite()) {
            Some((s, mask)) => inst.bbox_at(x, y, s.origin_x, s.origin_y, mask),
            None => crate::NO_MASK_BBOX,
        }
    }

    /// The background with this id, if it exists.
    pub fn background(&self, id: i32) -> Option<&assets::Background> {
        self.backgrounds.get(usize::try_from(id).ok()?)?.as_ref()
    }

    /// A hash of what a savestate's sprite and room ids refer to (`savestate.rs`).
    pub(crate) fn fingerprint(&self) -> u64 {
        *self
            .fingerprint
            .get_or_init(|| crate::savestate::assets_fingerprint(self))
    }

    /// The id of the room with this name, if there is one.
    pub fn room_id(&self, name: &str) -> Option<i32> {
        let i = self
            .rooms
            .iter()
            .position(|r| r.as_ref().is_some_and(|r| r.name == name))?;
        i32::try_from(i).ok()
    }

    /// The room with this id, if it exists.
    pub fn room(&self, id: i32) -> Option<&assets::Room> {
        self.rooms.get(usize::try_from(id).ok()?)?.as_ref()
    }

    /// A room's name, if it exists.
    pub fn room_name(&self, id: i32) -> Option<&str> {
        Some(&self.room(id)?.name)
    }

    /// Room ids in game order.
    pub fn room_order(&self) -> &[i32] {
        &self.room_order
    }

    pub fn color_outside_room(&self) -> u32 {
        self.color_outside_room
    }

    pub fn last_tile_id(&self) -> i32 {
        self.last_tile_id
    }
}

/// The union of a sprite's frame masks: GM8 uses one box for every frame. OpenGMK `game.rs` (sprite
/// loading: min of the lefts and tops, max of the rights and bottoms, 0 without masks).
fn sprite_mask_box(s: &assets::Sprite) -> Bbox {
    let edge = |f: fn(&assets::Bbox) -> u32, min: bool| {
        let it = s.masks.iter().map(|m| f(&m.bbox));
        let v = if min { it.min() } else { it.max() };
        v.unwrap_or(0) as i32
    };
    Bbox {
        left: edge(|b| b.left, true),
        top: edge(|b| b.top, true),
        right: edge(|b| b.right, false),
        bottom: edge(|b| b.bottom, false),
    }
}

/// A requested room change. It happens at the end of the frame ([`World::step`]); while one is pending,
/// no event runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SceneChange {
    /// `room_goto`, `room = r`, `room_goto_next`, `room_restart`.
    Room(i32),
    /// `game_end`.
    End,
}

/// The game state: instances, globals, room, RNG and input, plus a reference to the static program.
#[derive(Clone)]
pub struct World {
    pub(crate) program: &'static Program,
    pub(crate) assets: Arc<Assets>,
    pub(crate) insts: InstanceList,
    pub(crate) globals: Box<[Slot]>,
    pub(crate) room: RoomState,
    /// Views `room_set_view` gave rooms, by room id. GM8 writes them into the room resource, so they apply
    /// from the room's next load on (OpenGMK `gml/kernel.rs` `room_set_view`).
    pub(crate) room_views: BTreeMap<i32, [View; VIEWS]>,
    pub(crate) rng: Rng,
    pub(crate) input: InputState,
    /// GM8's `last_instance_id`: `instance_create` takes the next one.
    pub(crate) last_instance_id: i32,
    /// GM8's `last_tile_id`: `tile_add` takes the next one.
    pub(crate) last_tile_id: i32,
    pub(crate) pending: Option<SceneChange>,
    /// Whether `game_end` has run: the world then stays as the game end events left it.
    pub(crate) ended: bool,
    /// Whether GM8 showed the last frame's picture ([`presents`](Self::presents)).
    pub(crate) presented: bool,
    /// Frames run so far.
    pub(crate) frames: u64,
    /// The last `(unit, line)` executed, for error and divergence reports.
    pub(crate) cursor: (&'static str, u32),
    /// The exception that cut the last step short, if one did ([`World::exception`]).
    pub(crate) exception: Option<GmlError>,
    /// The size of GM8's window (its back buffer): the room's, or with views on, the extent of the visible
    /// views' ports. Set when a room loads (OpenGMK `game.rs` `load_room`).
    pub(crate) window: (u32, u32),
    /// The view being drawn (`view_current`); 0 outside the draw stage.
    pub(crate) view_current: usize,
    /// The drawing state as the last command left it, so a recorded frame can start from it
    /// (`draw::Frame::start`). Tracked whether or not drawing is recorded.
    pub(crate) draw_state: draw::DrawState,
    /// The caller's draw list while [`World::step_drawing`] runs; `None` otherwise, so clones (savestates)
    /// never copy drawing output.
    pub(crate) draw: crate::drawing::LentFrame,
    /// Colour, alpha, font and fonts (drawing settings GML sets and later draws use), the surfaces, and
    /// the fullscreen request: everything drawing-related GML can change except [`draw_state`].
    pub(crate) draw_settings: crate::draw_builtins::DrawSettings,
    /// The game directory, highscore table, clock and joystick.
    pub(crate) host: Host,
}

impl World {
    /// A world before the game starts: no room, no instances. `last_instance_id` is the game's
    /// (`gmk::Gmk::last_instance_id`); `font_slots` is how many font slots the game file has, deleted ones
    /// included (`gmk::Gmk::fonts`), because GM8 numbers the fonts `font_add_sprite` makes after them; `seed`
    /// is the RNG state the game starts with, before the first room loads. Call [`start_game`](Self::start_game) next.
    pub fn new(
        program: &'static Program,
        assets: Arc<Assets>,
        seed: i32,
        last_instance_id: i32,
        font_slots: usize,
    ) -> World {
        let last_tile_id = assets.last_tile_id();
        World {
            program,
            assets,
            insts: InstanceList::with_objects(program.object_slots()),
            globals: vec![Slot::Unset; program.global_count()].into_boxed_slice(),
            room: RoomState::default(),
            room_views: BTreeMap::new(),
            rng: Rng::with_seed(seed),
            input: InputState::default(),
            last_instance_id,
            last_tile_id,
            pending: None,
            ended: false,
            presented: false,
            frames: 0,
            cursor: ("", 0),
            exception: None,
            window: (0, 0),
            view_current: 0,
            draw_state: draw::DrawState::initial(0, 0),
            draw: Default::default(),
            draw_settings: crate::draw_builtins::DrawSettings::new(font_slots),
            host: Host::default(),
        }
    }

    /// The program this world runs.
    pub fn program(&self) -> &'static Program {
        self.program
    }

    pub fn assets(&self) -> &Assets {
        &self.assets
    }

    pub fn room(&self) -> &RoomState {
        &self.room
    }

    /// The RNG, for `random` and friends.
    pub fn rng(&mut self) -> &mut Rng {
        &mut self.rng
    }

    pub fn input(&self) -> &InputState {
        &self.input
    }

    /// The game directory, highscore table, clock and joystick.
    pub fn host(&self) -> &Host {
        &self.host
    }

    /// The host, to set it up before the game starts or for the file and highscore built-ins.
    pub fn host_mut(&mut self) -> &mut Host {
        &mut self.host
    }

    /// Frames run so far.
    pub fn frames(&self) -> u64 {
        self.frames
    }

    /// The pending room change, if any. Between steps that's only ever [`SceneChange::End`] (see
    /// [`ended`](Self::ended)) or a change requested while a room was loading (other than the first, see
    /// [`start_game_in`](Self::start_game_in)), which the next step performs.
    ///
    /// Generated events check it before each action, so it's inline.
    #[inline]
    pub fn pending(&self) -> Option<SceneChange> {
        self.pending
    }

    /// Whether the game has ended (`game_end`). Steps then do nothing, not even count.
    pub fn ended(&self) -> bool {
        self.ended
    }

    /// Requests a room change (`room_goto`, `room_restart`, `game_end`). From now until the change happens,
    /// no event runs and the frame stops after the current stage.
    pub fn request_scene_change(&mut self, change: SceneChange) {
        self.pending = Some(change);
    }

    /// The size of GM8's window, which is the size of the back buffer frames draw to.
    pub fn window_size(&self) -> (u32, u32) {
        self.window
    }

    /// Records the GML unit and line about to run, for error and divergence reports.
    #[inline(always)]
    pub fn at(&mut self, unit: &'static str, line: u32) {
        self.cursor = (unit, line);
    }

    /// The last `(unit, line)` passed to [`at`](Self::at).
    pub fn cursor(&self) -> (&'static str, u32) {
        self.cursor
    }

    pub(crate) fn located(&self, error: crate::Error) -> GmlError {
        GmlError {
            error,
            unit: self.cursor.0,
            line: self.cursor.1,
        }
    }

    /// An instance by handle. Handles stay valid until the end-of-step cleanup after the instance is
    /// destroyed.
    #[inline]
    pub fn instance(&self, h: Inst) -> &Instance {
        self.insts.get(h)
    }

    /// An instance by handle, for built-ins that change it.
    #[inline]
    pub fn instance_mut(&mut self, h: Inst) -> &mut Instance {
        self.insts.get_mut(h)
    }

    /// The active instance with this id.
    pub fn find(&self, id: i32) -> Option<Inst> {
        self.insts.by_id(id)
    }

    /// Active instances in draw order (the order of the last depth sort, then creation).
    pub fn active(&self) -> impl Iterator<Item = Inst> + '_ {
        self.with_state(State::Active)
    }

    /// Instances in draw order with this state.
    pub fn with_state(&self, state: State) -> impl Iterator<Item = Inst> + '_ {
        self.insts
            .draw_order()
            .iter()
            .copied()
            .filter(move |&h| self.insts.get(h).state() == state)
    }

    /// An instance's bounding box ([`Assets::bbox_at`]): [`NO_MASK_BBOX`](crate::NO_MASK_BBOX) without
    /// a mask sprite.
    pub fn bbox(&self, h: Inst) -> Bbox {
        let inst = self.insts.get(h);
        self.assets.bbox_at(inst, inst.x, inst.y)
    }

    /// Whether an instance has a mask sprite. Without one, region and room-boundary tests use its
    /// position as a point.
    pub fn has_mask(&self, h: Inst) -> bool {
        self.assets
            .sprite(self.insts.get(h).mask_sprite())
            .is_some()
    }

    /// The number of frames of an instance's sprite, or 0 without one.
    pub(crate) fn frame_count(&self, sprite: i32) -> usize {
        self.assets.sprite(sprite).map_or(0, |s| s.frames.len())
    }
}
