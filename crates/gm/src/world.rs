//! The world: everything that changes while the game runs, as one plain value (runtime.md section 2).
//! Cloning it is a savestate.

use crate::drawing::DrawList;
use crate::error::{Error, GmlError, R};
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

    /// A room's name, if it exists.
    pub fn room_name(&self, id: i32) -> Option<&str> {
        let room = self.rooms.get(usize::try_from(id).ok()?)?.as_ref()?;
        Some(&room.name)
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

/// Runs a whole frame stage. The collision stage is pluggable until 851-3127 provides GM8's.
pub type StageFn = fn(&mut World) -> R<()>;

fn no_collisions(_: &mut World) -> R<()> {
    Ok(())
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
    pub(crate) collision_stage: StageFn,
    pub(crate) draw: DrawList,
}

impl World {
    /// An empty world: no room, no instances. `last_instance_id` is the game's (`gmk::Gmk::last_instance_id`);
    /// `seed` is the RNG state at the start of frame 0.
    pub fn new(
        program: &'static Program,
        assets: Arc<Assets>,
        seed: i32,
        last_instance_id: i32,
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
            collision_stage: no_collisions,
            draw: DrawList::default(),
        }
    }

    /// Replaces the room and its instances, then runs the placed instances' create events in placement
    /// order (each create event can already see the instances placed after it). A minimal stand-in for
    /// GM8's room loading, which 851-3128 implements in full: room end events, persistent instances,
    /// tiles, views, game start and room start events, and the frame-0 draw are not done here.
    pub fn start_room(&mut self, start: RoomStart) -> Result<(), GmlError> {
        self.room = start.room;
        self.pending = None;
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

    /// Takes the pending room change, so the host can perform it. Room loading clears the frame's
    /// pressed/released input (OpenGMK `game.rs` `load_room` calls `input.step()`), and so does this.
    // TODO(851-3128): replace with load_room run at the end of `step`.
    pub fn take_scene_change(&mut self) -> Option<SceneChange> {
        let change = self.pending.take();
        if change.is_some() {
            self.input.end_frame();
        }
        change
    }

    /// Replaces the collision stage (stage 10 of the frame). The default runs no collision events.
    pub fn set_collision_stage(&mut self, f: StageFn) {
        self.collision_stage = f;
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

    /// An instance's bounding box, or `None` if it has no mask sprite (then GM8 tests its position as a
    /// point).
    pub fn bbox(&self, h: Inst) -> Option<Bbox> {
        let inst = self.insts.get(h);
        let (sprite, mask) = self.assets.sprite_with_box(inst.mask_sprite())?;
        Some(inst.bbox_for(sprite.origin_x, sprite.origin_y, mask))
    }

    /// The number of frames of an instance's sprite, or 0 without one.
    pub(crate) fn frame_count(&self, sprite: i32) -> usize {
        self.assets.sprite(sprite).map_or(0, |s| s.frames.len())
    }
}
