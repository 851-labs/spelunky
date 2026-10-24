//! Rooms (runtime.md section 10): loading one and changing rooms, the room's views, backgrounds and tiles,
//! and their built-ins. Loading follows OpenGMK `game.rs` (`load_room`); views `game/draw.rs` (`Game::draw`)
//! and `game/view.rs`; tiles `instancelist.rs` (`TileList`) and `gml/kernel.rs` (`tile_*`).

use crate::error::{Error, GmlError, R};
use crate::instance::Instance;
use crate::list::InstanceList;
use crate::math;
use crate::program::{Event, ObjectId};
use crate::world::{Assets, SceneChange, World};
use std::sync::Arc;

/// Views per room, as in GM8.
pub const VIEWS: usize = 8;

/// Background layers per room, as in GM8.
pub const BACKGROUNDS: usize = 8;

/// Values of `view_object` from here on are instance ids; below, object indices. OpenGMK `game/draw.rs`
/// (`Game::draw`).
const FIRST_INSTANCE_ID: i32 = 100_000;

/// A view: the part of the room it shows (`view_xview` ... `view_hview`), where in the window (`view_xport`
/// ... `view_hport`), and what it follows.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct View {
    pub visible: bool,
    pub xview: i32,
    pub yview: i32,
    pub wview: i32,
    pub hview: i32,
    pub xport: i32,
    pub yport: i32,
    pub wport: i32,
    pub hport: i32,
    // OpenGMK keeps the port's size unsigned; Spelunky never makes one negative, so that isn't reproduced.
    /// Degrees. Spelunky never rotates a view; only the mouse mapping honours it.
    pub angle: f64,
    /// How close the followed instance may get to the view's edges.
    pub hborder: i32,
    pub vborder: i32,
    /// The most the view moves per frame to follow; negative is unlimited.
    pub hspeed: i32,
    pub vspeed: i32,
    /// What it follows: an object index (its first instance, children included), an instance id, or
    /// nothing (`noone`).
    pub object: i32,
}

impl Default for View {
    fn default() -> View {
        View {
            visible: false,
            xview: 0,
            yview: 0,
            wview: 640,
            hview: 480,
            xport: 0,
            yport: 0,
            wport: 640,
            hport: 480,
            angle: 0.0,
            hborder: 32,
            vborder: 32,
            hspeed: -1,
            vspeed: -1,
            object: -1,
        }
    }
}

impl View {
    fn from_asset(v: &assets::View) -> View {
        View {
            visible: v.visible,
            xview: v.view_x,
            yview: v.view_y,
            wview: v.view_w,
            hview: v.view_h,
            xport: v.port_x,
            yport: v.port_y,
            wport: v.port_w,
            hport: v.port_h,
            angle: 0.0,
            hborder: v.hborder,
            vborder: v.vborder,
            hspeed: v.hspeed,
            vspeed: v.vspeed,
            object: v.follow_object,
        }
    }

    /// A room's views as it defines them; GM8 rooms always have [`VIEWS`].
    fn of_room(room: &assets::Room) -> [View; VIEWS] {
        std::array::from_fn(|i| {
            room.views
                .get(i)
                .map_or_else(View::default, View::from_asset)
        })
    }

    /// Moves the view to follow an instance at `(x, y)`, then keeps it inside the room. On each axis, GM8
    /// rounds the position, and if the border is less than half the view, moves the view (at most `speed`
    /// pixels, or all the way if `speed` is negative) just enough that the position is `border` pixels
    /// inside it; otherwise it centres the view on the position. Clamping applies the room's left/top edge
    /// first and its right/bottom edge last, so a view larger than the room sits at `room - size`, a
    /// negative position. The arithmetic wraps on overflow, as in OpenGMK's release build. OpenGMK
    /// `game/draw.rs` (`Game::draw`).
    pub fn follow(&mut self, x: f64, y: f64, room_width: i32, room_height: i32) {
        let axis = |pos: &mut i32, size: i32, border: i32, speed: i32, at: f64, room: i32| {
            let at = math::to_int(at);
            let p = *pos;
            if border < size / 2 {
                let (low, high) = (at.wrapping_sub(border), at.wrapping_add(border));
                let end = p.wrapping_add(size);
                if low < p {
                    *pos = if speed < 0 {
                        low
                    } else {
                        p.wrapping_sub(p.wrapping_sub(low).min(speed))
                    };
                } else if high > end {
                    *pos = if speed < 0 {
                        high.wrapping_sub(size)
                    } else {
                        p.wrapping_add(high.wrapping_sub(end).min(speed))
                    };
                }
            } else {
                *pos = at.wrapping_sub(size / 2);
            }
            *pos = (*pos).max(0).min(room.wrapping_sub(size));
        };
        axis(
            &mut self.xview,
            self.wview,
            self.hborder,
            self.hspeed,
            x,
            room_width,
        );
        axis(
            &mut self.yview,
            self.hview,
            self.vborder,
            self.vspeed,
            y,
            room_height,
        );
    }

    /// Whether a window pixel is in this view's port. OpenGMK `game/view.rs` (`contains_point`).
    pub fn port_contains(&self, x: i32, y: i32) -> bool {
        x >= self.xport
            && y >= self.yport
            && x < self.xport + self.wport
            && y < self.yport + self.hport
    }

    /// A window pixel in room coordinates: scaled from the port onto the view, rotated about the view's
    /// centre by its angle, rounded half to even. OpenGMK `game/view.rs` (`transform_point`).
    pub fn to_room(&self, x: i32, y: i32) -> (i32, i32) {
        let (vx, vy) = (f64::from(self.xview), f64::from(self.yview));
        let (vw, vh) = (f64::from(self.wview), f64::from(self.hview));
        let mut x = vx + vw * f64::from(x - self.xport) / f64::from(self.wport);
        let mut y = vy + vh * f64::from(y - self.yport) / f64::from(self.hport);
        // OpenGMK `view.rs` converts with `f64::to_radians`, not GML's `degtorad` (GM8's is unmeasured).
        let a = self.angle.to_radians();
        let (sin, cos) = (math::sin(a), math::cos(a));
        let (cx, cy) = (vx + vw / 2.0, vy + vh / 2.0);
        (x, y) = (x - cx, y - cy);
        (x, y) = (x * cos - y * sin, x * sin + y * cos);
        (math::to_int(x + cx), math::to_int(y + cy))
    }

    fn rect(&self) -> draw::Rect {
        draw::Rect::new(self.xview, self.yview, self.wview, self.hview)
    }

    fn port(&self) -> draw::Rect {
        draw::Rect::new(self.xport, self.yport, self.wport, self.hport)
    }
}

/// A room background or foreground layer (`background_index[n]` and friends).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Background {
    pub visible: bool,
    pub foreground: bool,
    /// The background resource, or -1.
    pub index: i32,
    pub x: f64,
    pub y: f64,
    pub tile_h: bool,
    pub tile_v: bool,
    /// Added to `x`/`y` after every draw stage.
    pub hspeed: f64,
    pub vspeed: f64,
    pub xscale: f64,
    pub yscale: f64,
    /// `0xBBGGRR`.
    pub blend: u32,
    pub alpha: f64,
}

impl Default for Background {
    fn default() -> Background {
        Background {
            visible: false,
            foreground: false,
            index: -1,
            x: 0.0,
            y: 0.0,
            tile_h: true,
            tile_v: true,
            hspeed: 0.0,
            vspeed: 0.0,
            xscale: 1.0,
            yscale: 1.0,
            blend: 0xFF_FF_FF,
            alpha: 1.0,
        }
    }
}

impl Background {
    /// A room's layer. A stretched one is scaled to the room's size, computed once at load (OpenGMK
    /// `game.rs`, room conversion: `width / background width`, or the room's width if the background
    /// doesn't exist).
    fn from_asset(
        b: &assets::RoomBackground,
        width: i32,
        height: i32,
        assets: &Assets,
    ) -> Background {
        let bg = assets.background(b.background);
        let scale = |room: i32, size: Option<u32>| match (b.stretch, size) {
            (false, _) => 1.0,
            (true, Some(size)) => f64::from(room) / f64::from(size),
            (true, None) => f64::from(room),
        };
        Background {
            visible: b.visible,
            foreground: b.foreground,
            index: b.background,
            x: f64::from(b.x),
            y: f64::from(b.y),
            tile_h: b.tile_h,
            tile_v: b.tile_v,
            hspeed: f64::from(b.hspeed),
            vspeed: f64::from(b.vspeed),
            xscale: scale(width, bg.map(|b| b.width)),
            yscale: scale(height, bg.map(|b| b.height)),
            ..Background::default()
        }
    }
}

/// A tile: part of a background drawn at a depth, among the instances.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Tile {
    pub id: i32,
    /// The background resource.
    pub background: i32,
    /// The part of the background, in its pixels.
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
    pub x: f64,
    pub y: f64,
    pub depth: f64,
    pub xscale: f64,
    pub yscale: f64,
    /// `0xBBGGRR`.
    pub blend: u32,
    pub alpha: f64,
    pub visible: bool,
}

impl Tile {
    /// A tile with GM8's defaults: unscaled, untinted, opaque, visible.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: i32,
        background: i32,
        left: i32,
        top: i32,
        width: i32,
        height: i32,
        x: f64,
        y: f64,
        depth: f64,
    ) -> Tile {
        Tile {
            id,
            background,
            left,
            top,
            width,
            height,
            x,
            y,
            depth,
            xscale: 1.0,
            yscale: 1.0,
            blend: 0xFF_FF_FF,
            alpha: 1.0,
            visible: true,
        }
    }

    fn from_asset(t: &assets::Tile) -> Tile {
        Tile::new(
            t.id,
            t.background,
            t.bg_x,
            t.bg_y,
            t.width,
            t.height,
            f64::from(t.x),
            f64::from(t.y),
            f64::from(t.depth),
        )
    }
}

/// The current room's runtime state.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RoomState {
    /// -1 before the game starts.
    pub id: i32,
    pub width: i32,
    pub height: i32,
    pub speed: i32,
    /// `background_color`, `0xBBGGRR`.
    pub background_color: u32,
    /// Whether each view is cleared to `background_color` before drawing.
    pub show_color: bool,
    pub views_enabled: bool,
    pub views: [View; VIEWS],
    pub backgrounds: [Background; BACKGROUNDS],
    /// In draw order: the order the last draw left them in (a stable sort by depth, highest first), then
    /// creation. OpenGMK `instancelist.rs` (`TileList`).
    pub tiles: Vec<Tile>,
}

impl Default for RoomState {
    fn default() -> RoomState {
        RoomState {
            id: -1,
            width: 0,
            height: 0,
            speed: 30,
            background_color: 0,
            show_color: false,
            views_enabled: false,
            views: [View::default(); VIEWS],
            backgrounds: [Background::default(); BACKGROUNDS],
            tiles: Vec::new(),
        }
    }
}

impl RoomState {
    fn new(id: i32, room: &assets::Room, views: [View; VIEWS], assets: &Assets) -> RoomState {
        RoomState {
            id,
            width: room.width,
            height: room.height,
            speed: room.speed,
            background_color: room.background_color,
            show_color: room.draw_background_color,
            views_enabled: room.views_enabled,
            views,
            backgrounds: std::array::from_fn(|i| {
                room.backgrounds
                    .get(i)
                    .map_or_else(Background::default, |b| {
                        Background::from_asset(b, room.width, room.height, assets)
                    })
            }),
            tiles: room.tiles.iter().map(Tile::from_asset).collect(),
        }
    }

    /// The window (back buffer) size for this room: the room's size, or with views on, the furthest right
    /// and bottom edges of the visible views' ports (the room's size if none is visible). OpenGMK `game.rs`
    /// (`load_room`), which refuses a negative result.
    fn window_size(&self) -> Option<(u32, u32)> {
        let (w, h) = if self.views_enabled {
            let visible = || self.views.iter().filter(|v| v.visible);
            (
                visible()
                    .map(|v| v.xport + v.wport)
                    .max()
                    .unwrap_or(self.width),
                visible()
                    .map(|v| v.yport + v.hport)
                    .max()
                    .unwrap_or(self.height),
            )
        } else {
            (self.width, self.height)
        };
        Some((u32::try_from(w).ok()?, u32::try_from(h).ok()?))
    }
}

/// Clamps a view or background array index the way GM8 does: past the end means element 0. OpenGMK
/// `gml/runtime.rs` (`get_instance_var`, `.get(i).unwrap_or(&views[0])`).
pub(crate) fn layer(index: u32, len: usize) -> usize {
    let i = index as usize;
    if i < len { i } else { 0 }
}

impl World {
    /// Starts the game in the first room of the room order, with game start events. OpenGMK `game.rs`
    /// (`Game::init` then `load_room`).
    pub fn start_game(&mut self) -> Result<(), GmlError> {
        let first = self.assets.room_order().first().copied().unwrap_or(-1);
        self.start_game_in(first)
    }

    /// [`start_game`](Self::start_game) in another room: what a replay's `room` line asks for.
    ///
    /// A room change the first room's load requests (Spelunky's `rScreenInit` calls `room_goto_next` from
    /// `oScreen`'s create event) happens here too, before the first frame, so frame 0 is a full frame of
    /// the room it leads to. That's the one place a change requested during a load isn't left for the
    /// next frame: GM8 handles it once between starting up and entering its frame loop (OpenGMK `game.rs`,
    /// `run` and `play_replay`: `init` then `handle_scene_change!`). A change *that* load requests is left
    /// pending as usual.
    pub fn start_game_in(&mut self, room: i32) -> Result<(), GmlError> {
        self.load_room(room, true)
            .and_then(|()| self.change_scene())
            .map_err(|e| self.located(e))
    }

    /// [`start_game`](Self::start_game), recording what booting draws into `frame`, as
    /// [`step_drawing`](Self::step_drawing) records a tick. That's the frame-0 draw of the room booting
    /// ends in: in Spelunky, `rIntro`'s, which fills `oScreen`'s surface for frame 0 to show.
    pub fn start_game_drawing(&mut self, frame: &mut draw::Frame) -> Result<(), GmlError> {
        self.record_drawing(frame, |w| w.start_game())
    }

    /// Performs a pending room change; called at the end of every step. OpenGMK `game.rs` (the main loop's
    /// `scene_change` match).
    pub(crate) fn change_scene(&mut self) -> R<()> {
        match self.pending {
            Some(SceneChange::Room(id)) => self.load_room(id, false),
            Some(SceneChange::End) => self.end_game(),
            None => Ok(()),
        }
    }

    /// Loads a room, in GM8's order:
    ///
    /// 1. Room end events for every instance, in draw order. A room change requested there is dropped,
    ///    but it still stops the remaining room end events, as any pending change stops events.
    /// 2. Persistent instances leave the old room in slot order, deactivated and destroyed ones too.
    /// 3. The new room's state (views, backgrounds, tiles) replaces the old; the window takes the new
    ///    size; the frame's pressed/released input ends.
    /// 4. The placed instances are added (skipping an id already present), then the persistent ones after
    ///    them. A persistent instance replaces a placed one with its id: that one is destroyed without
    ///    events.
    /// 5. Create events for the placed instances still active, in placement order: each create event can
    ///    see the instances placed after it.
    /// 6. On the first room only, game start events; then room start events (draw order).
    /// 7. If no room change is pending, the draw stage runs once ("frame 0").
    ///
    /// Spelunky has no persistent rooms and no room or instance creation code, so neither is supported.
    /// OpenGMK `game.rs` (`load_room`).
    fn load_room(&mut self, id: i32, game_start: bool) -> R<()> {
        let assets = Arc::clone(&self.assets);
        let room = assets.room(id).ok_or(Error::NoSuchRoom(id))?;
        self.pending = None;
        self.run_other_event(Event::ROOM_END)?;
        self.pending = None;
        let views = self
            .room_views
            .get(&id)
            .copied()
            .unwrap_or_else(|| View::of_room(room));
        let state = RoomState::new(id, room, views, &assets);
        // Everything that can fail is checked before anything leaves the old room, so a failed load
        // leaves the world as it was. OpenGMK fails part-way, but a GM8 error here ends the game anyway.
        let window = state.window_size().ok_or(Error::Unsupported(
            "a room whose views make a negative window",
        ))?;
        let mut seen = std::collections::BTreeSet::new();
        for p in &room.instances {
            // A repeated id is skipped below, whatever its object.
            if seen.insert(p.id) && self.program.object_by_index(p.object).is_none() {
                // OpenGMK: "Instance of invalid Object in room".
                return Err(Error::NoSuchObject(p.object));
            }
        }
        let persistent = self.insts.take_persistent();
        self.room = state;
        if window != self.window {
            self.push_draw(draw::Command::ResizeScreen {
                width: window.0,
                height: window.1,
            });
        }
        self.window = window;
        if game_start {
            // The runner opens its window at the first room's size, drawing 1:1. Later resizes keep the
            // projection the last draw left (OpenGMK's `resize_framebuffer` doesn't touch it).
            self.draw_state = draw::DrawState::initial(window.0, window.1);
        }
        self.input.end_frame();
        self.insts = InstanceList::with_objects(self.program.object_slots());
        let mut placed = Vec::with_capacity(room.instances.len());
        for p in &room.instances {
            if self.insts.by_id(p.id).is_some() {
                continue;
            }
            let (object, info) = self
                .program
                .object_by_index(p.object)
                .expect("checked above");
            let slots = self.program.layout(object).len();
            let inst = Instance::new(p.id, f64::from(p.x), f64::from(p.y), object, info, slots);
            placed.push(self.insts.insert(inst, self.program));
        }
        for inst in persistent {
            if let Some(h) = self.insts.by_id(inst.id) {
                self.insts.mark_deleted(h);
            }
            self.insts.insert(inst, self.program);
        }
        for h in placed {
            if self.insts.get(h).is_active() {
                self.run_event(h, h, Event::Create)?;
            }
        }
        if game_start {
            self.run_other_event(Event::GAME_START)?;
        }
        self.run_other_event(Event::ROOM_START)?;
        if self.pending.is_none() {
            self.redraw()?;
        }
        Ok(())
    }

    /// `game_end`: room end then game end events, after which the world stays as they left it. OpenGMK
    /// `game/events.rs` (`run_game_end_events`). The oracle then restarts the game before it exits
    /// (`docs/quirks.md` section 7).
    fn end_game(&mut self) -> R<()> {
        self.pending = None;
        self.run_other_event(Event::ROOM_END)?;
        self.run_other_event(Event::GAME_END)?;
        self.pending = Some(SceneChange::End);
        self.ended = true;
        Ok(())
    }

    /// Runs an event for every active instance in draw order (not grouped by object like the step
    /// events). OpenGMK `game/events.rs` (`run_other_event`).
    fn run_other_event(&mut self, e: Event) -> R<()> {
        let mut it = self.insts.iter_drawing();
        while let Some(h) = it.next(&self.insts) {
            self.run_event(h, h, e)?;
        }
        Ok(())
    }

    /// `room_goto(room)`. A room that doesn't exist is an error when the change happens.
    pub fn room_goto(&mut self, room: i32) {
        self.request_scene_change(SceneChange::Room(room));
    }

    /// `room_goto_next()`: the room after the current one in the room order.
    pub fn room_goto_next(&mut self) -> R<()> {
        let order = self.assets.room_order();
        let next = order
            .iter()
            .position(|&r| r == self.room.id)
            .and_then(|i| order.get(i + 1).copied())
            .ok_or(Error::EndOfRoomOrder)?;
        self.room_goto(next);
        Ok(())
    }

    /// `room_restart()`: a change to the current room. OpenGMK `gml/kernel.rs` (`room_restart`).
    pub fn room_restart(&mut self) {
        self.room_goto(self.room.id);
    }

    /// `game_end()`.
    pub fn game_end(&mut self) {
        self.request_scene_change(SceneChange::End);
    }

    /// `room_set_view(room, index, visible, xview, ..., object)`: replaces a view of a room the next time
    /// it loads, not the current room's. The view keeps its angle. An index outside 0-7 or a room that
    /// doesn't exist does nothing. OpenGMK `gml/kernel.rs` (`room_set_view`).
    pub fn room_set_view(&mut self, room: i32, index: i32, view: View) {
        let (Some(asset), Ok(index)) = (self.assets.room(room), usize::try_from(index)) else {
            return;
        };
        if index >= VIEWS {
            return;
        }
        let views = self
            .room_views
            .entry(room)
            .or_insert_with(|| View::of_room(asset));
        views[index] = View {
            angle: views[index].angle,
            ..view
        };
    }

    /// `tile_add(background, left, top, width, height, x, y, depth)`: adds a tile at the end of the draw
    /// order and returns its id. OpenGMK `gml/kernel.rs` (`tile_add`).
    #[allow(clippy::too_many_arguments)]
    pub fn tile_add(
        &mut self,
        background: i32,
        left: i32,
        top: i32,
        width: i32,
        height: i32,
        x: f64,
        y: f64,
        depth: f64,
    ) -> R<i32> {
        if self.assets.background(background).is_none() {
            return Err(Error::NoSuchBackground(background));
        }
        self.last_tile_id += 1;
        let id = self.last_tile_id;
        let tile = Tile::new(id, background, left, top, width, height, x, y, depth);
        self.room.tiles.push(tile);
        Ok(id)
    }

    /// `tile_delete(id)`. OpenGMK `gml/kernel.rs` (`tile_delete`).
    pub fn tile_delete(&mut self, id: i32) -> R<()> {
        let i = self
            .room
            .tiles
            .iter()
            .position(|t| t.id == id)
            .ok_or(Error::NoSuchTile(id))?;
        self.room.tiles.remove(i);
        Ok(())
    }

    /// `tile_layer_find(depth, x, y)`: the first tile in draw order at exactly `depth` containing the
    /// point, or -1. **GM 8.0 never finds one**: it measures the tile as `0 * width` by `0 * height` (8.1
    /// multiplies by the tile's scale instead), so `x < tile.x + 0` contradicts `x >= tile.x`. OpenGMK
    /// `gml/kernel.rs` (`tile_layer_find`, "8.1 bugfix"); `docs/quirks.md` section 5.
    pub fn tile_layer_find(&self, _depth: f64, _x: f64, _y: f64) -> i32 {
        -1
    }

    /// The mouse in room coordinates (`mouse_x`, `mouse_y`). With views on, the window pixel maps through
    /// the last visible view whose port contains it, else through the first visible view; otherwise it's
    /// used as is. OpenGMK `game.rs` (`translate_screen_to_room`).
    pub fn mouse_in_room(&self) -> (i32, i32) {
        let (x, y) = self.input.mouse();
        self.screen_to_room(x, y)
    }

    /// A window pixel in room coordinates, as [`mouse_in_room`](Self::mouse_in_room) maps the mouse.
    pub(crate) fn screen_to_room(&self, x: i32, y: i32) -> (i32, i32) {
        if !self.room.views_enabled {
            return (x, y);
        }
        let views = &self.room.views;
        views
            .iter()
            .rev()
            .find(|v| v.visible && v.port_contains(x, y))
            .or_else(|| views.iter().find(|v| v.visible))
            .map_or((x, y), |v| v.to_room(x, y))
    }

    /// Moves every visible view with a target to follow it, when views are on: the target is the first
    /// active instance of the object (or its descendants), or the active instance with that id. OpenGMK
    /// `game/draw.rs` (`Game::draw`).
    pub(crate) fn follow_views(&mut self) {
        if !self.room.views_enabled {
            return;
        }
        let (w, h) = (self.room.width, self.room.height);
        for i in 0..VIEWS {
            let view = self.room.views[i];
            if !view.visible {
                continue;
            }
            let target = if view.object < FIRST_INSTANCE_ID {
                u32::try_from(view.object).ok().and_then(|o| {
                    let mut it = self.insts.iter_ancestor(ObjectId(o));
                    it.next(&self.insts)
                })
            } else {
                self.insts.by_id(view.object)
            };
            if let Some(t) = target {
                let inst = self.insts.get(t);
                self.room.views[i].follow(inst.x, inst.y, w, h);
            }
        }
    }

    /// The view and port of view `i`, for drawing.
    pub(crate) fn view_rects(&self, i: usize) -> (draw::Rect, draw::Rect) {
        let v = &self.room.views[i];
        (v.rect(), v.port())
    }
}
