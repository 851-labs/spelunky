//! A room's placed state as a draw list, standing in for `gm` until it exists: every placed instance drawn
//! with its object's sprite at frame 0 and its object's depth, plus the room's backgrounds and tiles. It
//! follows GM 8's draw order (see the `draw` crate docs) so sprites, origins and depth order can be checked
//! against the original. Nothing runs: create events, runtime tiles and `oScreen`'s surfaces are absent.

use crate::input::{VK_DOWN, VK_LEFT, VK_RIGHT, VK_SHIFT, VK_UP};
use crate::window::Game;
use assets::{Bundle, Room};
use draw::{Background, Color, Command, Frame, Rect, Sprite, Tile};
use trace::replay::{Button, Event, InputState, Replay};

/// The room previewed when neither the user nor a replay names one.
pub const DEFAULT_ROOM: &str = "rTutorial";

/// The room a run previews: a replay's own (it was recorded there), else `room`, else [`DEFAULT_ROOM`].
pub fn room_to_preview(replay: Option<&Replay>, room: Option<&str>) -> String {
    match replay {
        Some(replay) => replay.header.room.clone(),
        None => room.unwrap_or(DEFAULT_ROOM).to_owned(),
    }
}

pub struct RoomPreview<'a> {
    bundle: &'a Bundle,
    room: &'a Room,
    /// The view in room coordinates.
    pub view: Rect,
}

impl<'a> RoomPreview<'a> {
    /// The room called `name`, at its view 0 (or the whole room if it has no views).
    pub fn new(bundle: &'a Bundle, name: &str) -> Result<RoomPreview<'a>, String> {
        let room = bundle
            .rooms
            .iter()
            .flatten()
            .find(|r| r.name == name)
            .ok_or_else(|| format!("no room named {name}"))?;
        let view = match room.views.first() {
            Some(v) if room.views_enabled && v.visible => {
                Rect::new(v.view_x, v.view_y, v.view_w, v.view_h)
            }
            _ => Rect::new(0, 0, room.width, room.height),
        };
        Ok(RoomPreview { bundle, room, view })
    }

    pub fn room(&self) -> &Room {
        self.room
    }

    /// Moves the view to (x, y), kept inside the room like GM's view following does.
    pub fn move_view(&mut self, x: i32, y: i32) {
        self.view.x = x.min(self.room.width - self.view.w).max(0);
        self.view.y = y.min(self.room.height - self.view.h).max(0);
    }

    pub fn frame(&self) -> Frame {
        let (room, view) = (self.room, self.view);
        let mut frame = Frame::new(view.w.max(1) as u32, view.h.max(1) as u32);
        let port = Rect::new(0, 0, view.w, view.h);
        if room.views_enabled {
            frame.push(Command::Clear {
                color: Color(self.bundle.settings.color_outside_room),
                alpha: 1.0,
            });
        }
        frame.push(Command::SetView { view, port });
        if room.draw_background_color {
            frame.push(Command::Clear {
                color: Color(room.background_color),
                alpha: 1.0,
            });
        }

        self.push_backgrounds(&mut frame, false);

        // An instance whose object is gone can't be drawn; its depth doesn't matter.
        let instance_depths: Vec<i32> = room
            .instances
            .iter()
            .map(|inst| self.object(inst.object).map_or(0, |o| o.depth))
            .collect();
        let tile_depths: Vec<i32> = room.tiles.iter().map(|t| t.depth).collect();
        for drawn in draw_order(&instance_depths, &tile_depths) {
            match drawn {
                Drawn::Instance(i) => self.push_instance(&mut frame, i),
                Drawn::Tile(i) => self.push_tile(&mut frame, i),
            }
        }

        self.push_backgrounds(&mut frame, true);
        // GM shows the back buffer at the end of a step.
        frame.push(Command::Present);
        frame
    }

    fn object(&self, id: i32) -> Option<&'a assets::Object> {
        self.bundle.objects.get(usize::try_from(id).ok()?)?.as_ref()
    }

    fn push_instance(&self, frame: &mut Frame, index: usize) {
        let inst = &self.room.instances[index];
        let Some(object) = self.object(inst.object) else {
            return;
        };
        // Invisible instances keep their place in the order but aren't drawn; nor is an instance with no
        // sprite (its default draw does nothing).
        if !object.visible || object.sprite < 0 {
            return;
        }
        frame.push(Sprite::new(
            object.sprite as u32,
            0,
            f64::from(inst.x),
            f64::from(inst.y),
        ));
    }

    fn push_tile(&self, frame: &mut Frame, index: usize) {
        let tile = &self.room.tiles[index];
        if tile.background < 0 {
            return;
        }
        frame.push(Tile {
            background: tile.background as u32,
            left: tile.bg_x.max(0) as u32,
            top: tile.bg_y.max(0) as u32,
            width: tile.width.max(0) as u32,
            height: tile.height.max(0) as u32,
            x: f64::from(tile.x),
            y: f64::from(tile.y),
            xscale: 1.0,
            yscale: 1.0,
            blend: Color::WHITE,
            alpha: 1.0,
        });
    }

    fn push_backgrounds(&self, frame: &mut Frame, foreground: bool) {
        for layer in &self.room.backgrounds {
            if !layer.visible || layer.foreground != foreground || layer.background < 0 {
                continue;
            }
            let Some(bg) = self
                .bundle
                .backgrounds
                .get(layer.background as usize)
                .and_then(Option::as_ref)
            else {
                continue;
            };
            let stretch = |room: i32, image: u32| {
                if layer.stretch && image > 0 {
                    f64::from(room) / f64::from(image)
                } else {
                    1.0
                }
            };
            frame.push(Background {
                background: layer.background as u32,
                x: f64::from(layer.x),
                y: f64::from(layer.y),
                tile_h: layer.tile_h,
                tile_v: layer.tile_v,
                xscale: stretch(self.room.width, bg.width),
                yscale: stretch(self.room.height, bg.height),
                blend: Color::WHITE,
                alpha: 1.0,
            });
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Drawn {
    Instance(usize),
    Tile(usize),
}

/// GM 8's draw order for freshly loaded instances and tiles, given their depths in creation order: each
/// list stably sorted by depth, highest first, then merged with the instance first on equal depth.
fn draw_order(instances: &[i32], tiles: &[i32]) -> Vec<Drawn> {
    let sorted = |depths: &[i32]| {
        let mut order: Vec<usize> = (0..depths.len()).collect();
        order.sort_by_key(|&i| std::cmp::Reverse(depths[i]));
        order
    };
    let (inst, tile) = (sorted(instances), sorted(tiles));
    let (mut i, mut t) = (0, 0);
    let mut out = Vec::with_capacity(inst.len() + tile.len());
    while i < inst.len() || t < tile.len() {
        let take_tile = match (inst.get(i), tile.get(t)) {
            (Some(&a), Some(&b)) => tiles[b] > instances[a],
            (None, _) => true,
            (_, None) => false,
        };
        if take_tile {
            out.push(Drawn::Tile(tile[t]));
            t += 1;
        } else {
            out.push(Drawn::Instance(inst[i]));
            i += 1;
        }
    }
    out
}

/// What the frontends run until the game does: a room preview whose view pans with the arrow keys (faster
/// with shift), read as GM key codes.
pub struct Preview<'a> {
    pub room: RoomPreview<'a>,
    held: InputState,
}

impl<'a> Preview<'a> {
    pub fn new(room: RoomPreview<'a>) -> Preview<'a> {
        Preview {
            room,
            held: InputState::default(),
        }
    }

    /// One tick: applies `events` and pans.
    pub fn step(&mut self, events: &[Event]) {
        for e in events {
            self.held.apply(e);
        }
        let held = |vk| self.held.is_down(Button::Key(vk));
        let speed = if held(VK_SHIFT) { 16 } else { 4 };
        let axis = |neg, pos| (i32::from(held(pos)) - i32::from(held(neg))) * speed;
        let (dx, dy) = (axis(VK_LEFT, VK_RIGHT), axis(VK_UP, VK_DOWN));
        let view = self.room.view;
        self.room.move_view(view.x + dx, view.y + dy);
    }

    /// The window's starting size in logical pixels: the view at `scale`, which is at most 16 so a
    /// room-sized view can't overflow.
    pub fn window_size(&self, scale: u32) -> (u32, u32) {
        let view = self.room.view;
        (view.w.max(1) as u32 * scale, view.h.max(1) as u32 * scale)
    }
}

impl Game for Preview<'_> {
    fn tick(&mut self, events: &[Event], out: &mut Frame) {
        self.step(events);
        *out = self.room.frame();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Drawn::{Instance as I, Tile as T};

    #[test]
    fn highest_depth_first_with_creation_order_ties() {
        assert_eq!(draw_order(&[0, 10, 0, 10], &[]), [I(1), I(3), I(0), I(2)]);
    }

    #[test]
    fn instances_beat_tiles_on_equal_depth() {
        assert_eq!(
            draw_order(&[5, 1], &[5, 3, 1000]),
            [T(2), I(0), T(0), T(1), I(1)]
        );
    }
}
