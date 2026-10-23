//! The world: everything that changes while the game runs, as one plain value (runtime.md section 2).
//! Cloning it is a savestate.

use crate::error::{Error, GmlError};
use crate::input::InputState;
use crate::instance::{Bbox, Instance, State};
use crate::list::{Inst, InstanceList};
use crate::program::{ObjectId, Program};
use crate::rng::Rng;
use crate::var::Slot;
use std::sync::Arc;

/// The immutable assets a world needs: sprites (frame counts, origins, collision masks) and rooms. Shared
/// between clones.
#[derive(Clone, Debug, Default)]
pub struct Assets {
    /// By sprite id; `None` for deleted slots.
    sprites: Vec<Option<assets::Sprite>>,
    /// Each sprite's collision box, precomputed: bounding boxes are tested constantly.
    mask_boxes: Vec<Bbox>,
    /// By room id; `None` for deleted slots.
    rooms: Vec<Option<assets::Room>>,
}

impl Assets {
    /// Sprites and rooms by id, `None` for deleted slots.
    pub fn new(sprites: Vec<Option<assets::Sprite>>, rooms: Vec<Option<assets::Room>>) -> Assets {
        let mask_boxes = sprites
            .iter()
            .map(|s| s.as_ref().map_or(Bbox::default(), sprite_mask_box))
            .collect();
        Assets {
            sprites,
            mask_boxes,
            rooms,
        }
    }

    /// The parts of an extracted bundle the runtime uses.
    pub fn from_bundle(b: assets::Bundle) -> Assets {
        Assets::new(b.sprites, b.rooms)
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

    /// A room's name, if it exists.
    pub fn room_name(&self, id: i32) -> Option<&str> {
        Some(&self.room(id)?.name)
    }

    /// The room with this id, if it exists.
    pub fn room(&self, id: i32) -> Option<&assets::Room> {
        self.rooms.get(usize::try_from(id).ok()?)?.as_ref()
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

/// A requested room change. It takes effect after the frame (851-3128); while one is pending, no event
/// runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneChange {
    /// `room_goto`, `room = r`, `room_goto_next`.
    Room(i32),
    /// `room_restart`.
    Restart,
    /// `game_end`.
    End,
}

/// The current room's runtime state.
// TODO(851-3128): views, backgrounds, tiles, persistence.
#[derive(Clone, Debug, PartialEq)]
pub struct RoomState {
    pub id: i32,
    pub width: i32,
    pub height: i32,
    pub speed: i32,
}

/// An instance placed in a room.
#[derive(Clone, Debug, PartialEq)]
pub struct PlacedInstance {
    pub id: i32,
    pub object: ObjectId,
    pub x: f64,
    pub y: f64,
}

/// What [`World::start_room`] needs: a minimal stand-in for room loading until 851-3128.
#[derive(Clone, Debug, PartialEq)]
pub struct RoomStart {
    pub room: RoomState,
    /// In placement order.
    pub instances: Vec<PlacedInstance>,
}

/// The game state: instances, globals, room, RNG and input, plus a reference to the static program.
#[derive(Clone)]
pub struct World {
    pub(crate) program: &'static Program,
    pub(crate) assets: Arc<Assets>,
    pub(crate) insts: InstanceList,
    pub(crate) globals: Box<[Slot]>,
    pub(crate) room: RoomState,
    pub(crate) rng: Rng,
    pub(crate) input: InputState,
    /// GM8's `last_instance_id`: `instance_create` takes the next one.
    pub(crate) last_instance_id: i32,
    pub(crate) pending: Option<SceneChange>,
    /// Frames run so far.
    pub(crate) frames: u64,
    /// The last `(unit, line)` executed, for error and divergence reports.
    pub(crate) cursor: (&'static str, u32),
    /// The caller's draw list while [`World::step_drawing`] runs; `None` otherwise, so clones (savestates)
    /// never copy drawing output.
    pub(crate) draw: Option<draw::Frame>,
    /// Colour, alpha, font and fonts: drawing settings GML sets and later draws use.
    pub(crate) draw_settings: crate::draw_builtins::DrawSettings,
}

impl World {
    /// An empty world: no room, no instances. `last_instance_id` is the game's (`gmk::Gmk::last_instance_id`);
    /// `font_slots` is how many font slots the game file has, deleted ones included (`gmk::Gmk::fonts`),
    /// because GM8 numbers the fonts `font_add_sprite` makes after them; `seed` is the RNG state at the
    /// start of frame 0.
    pub fn new(
        program: &'static Program,
        assets: Arc<Assets>,
        seed: i32,
        last_instance_id: i32,
        font_slots: usize,
    ) -> World {
        World {
            program,
            assets,
            insts: InstanceList::with_objects(program.object_slots()),
            globals: vec![Slot::Unset; program.global_count()].into_boxed_slice(),
            room: RoomState {
                id: -1,
                width: 0,
                height: 0,
                speed: 30,
            },
            rng: Rng::with_seed(seed),
            input: InputState::default(),
            last_instance_id,
            pending: None,
            frames: 0,
            cursor: ("", 0),
            draw: None,
            draw_settings: crate::draw_builtins::DrawSettings::new(font_slots),
        }
    }

    /// Replaces the room and its instances, then runs the placed instances' create events in placement
    /// order (each create event can already see the instances placed after it). A minimal stand-in for
    /// GM8's room loading, which 851-3128 implements in full: room end events, persistent instances,
    /// tiles, views, game start and room start events, and the frame-0 draw are not done here.
    pub fn start_room(&mut self, start: RoomStart) -> Result<(), GmlError> {
        self.room = start.room;
        self.pending = None;
        // Room loading ends the frame's pressed/released input (OpenGMK `game.rs` `load_room` calls
        // `input.step()`).
        self.input.end_frame();
        self.insts = InstanceList::with_objects(self.program.object_slots());
        let mut placed = Vec::with_capacity(start.instances.len());
        for p in &start.instances {
            if self.insts.by_id_any(p.id).is_some() {
                continue;
            }
            // OpenGMK `load_room`: "Instance of invalid Object in room".
            let info = self
                .program
                .object(p.object)
                .ok_or_else(|| self.located(Error::NoSuchObject(p.object.0 as i32)))?;
            let slots = self.program.layout(p.object).len();
            let inst = Instance::new(p.id, p.x, p.y, p.object, info, slots);
            placed.push(self.insts.insert(inst, self.program));
        }
        for h in placed {
            // An instance an earlier create event destroyed or deactivated gets no create event.
            // OpenGMK `game.rs` (`load_room`).
            if self.insts.get(h).is_active() {
                self.run_event(h, h, crate::Event::Create)
                    .map_err(|e| self.located(e))?;
            }
        }
        Ok(())
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

    /// Frames run so far.
    pub fn frames(&self) -> u64 {
        self.frames
    }

    /// The pending room change, if any.
    pub fn pending(&self) -> Option<SceneChange> {
        self.pending
    }

    /// Requests a room change (`room_goto`, `room_restart`, `game_end`). From now until the change happens,
    /// no event runs and the frame stops after the current stage.
    pub fn request_scene_change(&mut self, change: SceneChange) {
        self.pending = Some(change);
    }

    /// Takes the pending room change, so the host can perform it (usually with
    /// [`start_room`](Self::start_room)). Like room loading, it ends the frame's pressed/released input,
    /// so a host that only takes the change doesn't leave keys pressed.
    // TODO(851-3128): replace with load_room run at the end of `step`.
    pub fn take_scene_change(&mut self) -> Option<SceneChange> {
        let change = self.pending.take();
        if change.is_some() {
            self.input.end_frame();
        }
        change
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
