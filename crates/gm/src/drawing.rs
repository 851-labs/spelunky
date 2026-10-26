//! The draw stage, and recording what a tick draws as a `draw::Frame` (runtime.md section 11; the `draw`
//! crate's docs). The draw stage always runs, because draw events change state. What it draws is only kept
//! when the caller passes a frame to [`World::step_drawing`], so headless runs pay nothing for pixels. The
//! frame belongs to the caller and is lent to the world for the tick only, so it is reused from tick to
//! tick and cloning a world never copies drawing.
//!
//! What GM8 itself converts while drawing (`crate::gm8`'s drawing rules: the default draw's corners, the background
//! layers' `Round`s, each view's projection) can trap on Intel, so the draw stage checks it whether or not it
//! records (851-3304).

use crate::World;
use crate::error::R;
use crate::gm8;
use crate::list::Inst;
use crate::math;
use crate::program::Event;
use crate::room::VIEWS;
use crate::value::ValueError;

/// A sprite's origin and size, as GM8's texture draws take them to place its corners (851-3304).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Extent {
    origin: (f64, f64),
    size: (f64, f64),
}

impl Extent {
    /// The plain texture draw at `(x, y)`, which subtracts the origin first (Spelunky.exe 0x4a5a58): an error
    /// where a corner traps ([`gm8::draw_quad`]).
    pub(crate) fn check_plain(self, x: f64, y: f64) -> Result<(), ValueError> {
        let ((xo, yo), (w, h)) = (self.origin, self.size);
        gm8::draw_quad(x - xo, y - yo, w, h).ok_or(ValueError::Overflow)
    }

    /// The scaled and rotated texture draw at `(x, y)`, `angle` in degrees (Spelunky.exe 0x4a5aa4): an error where
    /// a corner or an offset traps ([`gm8::draw_quad_ext`]).
    pub(crate) fn check_ext(
        self,
        x: f64,
        y: f64,
        xscale: f64,
        yscale: f64,
        angle: f64,
    ) -> Result<(), ValueError> {
        let radians = if angle == 0.0 {
            0.0
        } else {
            math::draw_radians(angle)
        };
        gm8::draw_quad_ext(
            self.origin,
            (x, y),
            (xscale, yscale),
            self.size,
            radians,
            math::draw_sin_cos,
        )
        .ok_or(ValueError::Overflow)
    }
}

/// The caller's frame while a recording runs ([`World::record_drawing`]). A clone is empty, so a world
/// cloned mid-recording (a savestate) neither copies the frame nor records into it.
#[derive(Debug, Default)]
pub(crate) struct LentFrame(pub(crate) Option<draw::Frame>);

impl Clone for LentFrame {
    fn clone(&self) -> LentFrame {
        LentFrame(None)
    }
}

impl World {
    /// Adds a command to this tick's drawing, if recording. The draw built-ins call this.
    #[inline]
    pub fn push_draw(&mut self, cmd: impl Into<draw::Command>) {
        if let Some(frame) = &mut self.draw.0 {
            frame.push(cmd);
        }
    }

    /// Whether this tick's drawing is being recorded (so built-ins can skip building commands).
    #[inline]
    pub fn recording_draw(&self) -> bool {
        self.draw.0.is_some()
    }

    /// The drawing state as the last command left it (target, view, blend).
    pub fn draw_state(&self) -> draw::DrawState {
        self.draw_state
    }

    /// Whether GM8 showed the tick just run. It doesn't flip the back buffer when the frame ends with a
    /// room change pending (OpenGMK `game.rs` `Game::frame`, the final `present`), so a frontend should
    /// keep showing the previous picture, unless GML showed it itself with `screen_refresh` or
    /// `screen_redraw` (Spelunky's `oScreen` refreshes in every begin step). The tick's commands still ran:
    /// surfaces they drew into keep their contents.
    pub fn presents(&self) -> bool {
        self.presented
    }

    /// Resets a reused frame for a new tick: the window's size, starting from the current drawing state
    /// and surfaces.
    pub(crate) fn begin_draw_frame(&self, frame: &mut draw::Frame) {
        use draw::DrawSink;
        let (w, h) = self.window;
        frame.begin(w, h, self.draw_state, &self.draw_settings.surfaces);
    }

    /// Runs a state command ([`draw::Command::SetTarget`], `SetView` or `SetBlend`): tracks it in
    /// [`draw_state`](Self::draw_state) whether or not drawing is recorded, and records it.
    pub(crate) fn apply_draw(&mut self, cmd: draw::Command) {
        let settings = &self.draw_settings;
        self.draw_state.apply(&cmd, self.window, |id| {
            settings.surface(i32::try_from(id.0).ok()?)
        });
        self.push_draw(cmd);
    }

    /// Sprite `sprite`'s [`Extent`]: `None` if there's no such sprite, `Some(None)` if it has no frames, which GM8's
    /// sprite draws skip (Spelunky.exe 0x4a5a62).
    pub(crate) fn sprite_extent(&self, sprite: i32) -> Option<Option<Extent>> {
        let s = self.assets.sprite(sprite)?;
        Some((!s.frames.is_empty()).then(|| Extent {
            origin: (f64::from(s.origin_x), f64::from(s.origin_y)),
            size: (f64::from(s.width), f64::from(s.height)),
        }))
    }

    /// Sets the projection.
    fn set_view(&mut self, view: draw::Rect, port: draw::Rect) {
        self.apply_draw(draw::Command::SetView { view, port });
    }

    /// The draw stage, also `screen_redraw()`. With views on: visible views follow their targets, the
    /// window is cleared to the settings' colour outside the room, and each visible view is drawn in turn
    /// (`view_current` is its index meanwhile). With views off, the whole room is drawn once. Afterwards
    /// the projection is reset to the whole window. Each view sorts the instance list by depth, which
    /// changes iteration order everywhere until the next sort. OpenGMK `game/draw.rs` (`Game::draw`).
    pub fn redraw(&mut self) -> R<()> {
        let (w, h) = (self.room.width, self.room.height);
        if self.room.views_enabled {
            self.follow_views()?;
            if self.recording_draw() {
                let color = draw::Color(self.assets.color_outside_room());
                self.push_draw(draw::Command::Clear { color, alpha: 1.0 });
            }
            // A draw event can change a later view before it's drawn.
            for i in 0..VIEWS {
                let v = self.room.views[i];
                if v.visible {
                    self.view_current = i;
                    // The projection stores the sine and cosine of `-view_angle` in radians as singles
                    // (Spelunky.exe 0x4a2440). `fsin` and `fcos` give the angle back from 2^63, so they trap where
                    // it's past the largest single (probe `draw-stage-trap`, 851-3304).
                    if v.angle != 0.0 {
                        gm8::single(math::draw_radians(v.angle)).ok_or(ValueError::Overflow)?;
                    }
                    let (view, port) = self.view_rects(i);
                    // Taken before the view's draw events, for its foreground layers too, as GM8 does.
                    let span = gm8::view_span(v.xview, v.wview, v.hview, v.angle);
                    self.draw_view(view, port, span)?;
                }
            }
            self.view_current = 0;
        } else {
            let all = draw::Rect::new(0, 0, w, h);
            self.draw_view(all, all, (0, w))?;
        }
        let (ww, wh) = self.window;
        let window = draw::Rect::new(0, 0, ww as i32, wh as i32);
        self.set_view(window, window);
        Ok(())
    }

    /// Draws one view: the room colour if the room shows it, the background layers, instances and tiles
    /// merged by depth (highest first; at equal depth the instance first), then the foreground layers.
    /// OpenGMK `game/draw.rs` (`draw_view`). `span` is the left edge and width GM8 draws the layers over.
    fn draw_view(&mut self, view: draw::Rect, port: draw::Rect, span: (i32, i32)) -> R<()> {
        self.set_view(view, port);
        if self.recording_draw() && self.room.show_color {
            let color = draw::Color(self.room.background_color);
            self.push_draw(draw::Command::Clear { color, alpha: 1.0 });
        }
        self.draw_backgrounds(false, span)?;
        self.insts.draw_sort();
        let mut insts = self.insts.iter_drawing();
        if !self.recording_draw() {
            // Tiles change no state, so headless only the instances' draw events matter. Skipping the
            // tiles' sort is safe too: tiles are only ever appended or removed, so a stable sort by depth
            // gives the same order whether or not earlier sorts ran. Nor can a tile trap: GM8 only draws the
            // ones in the view, and nothing in GML scales one (851-3304).
            while let Some(h) = insts.next(&self.insts) {
                self.draw_instance(h)?;
            }
            return self.draw_backgrounds(true, span);
        }
        // Stable, like the instances' sort: equal depths keep their previous order.
        self.room
            .tiles
            .sort_by(|a, b| crate::list::cmp_nan_first(b.depth, a.depth));
        // OpenGMK `draw_view`'s merge: each list's next entry is fetched ahead (a tile by its position in
        // the current tile list), and once one list runs out the rest of the other is drawn without looking
        // at the first again, so a tile a draw event adds then isn't drawn until the next view. OpenGMK
        // fetches a tile's handle where the port copies the tile; they differ only if a draw event deletes
        // the tile fetched ahead, which panics in OpenGMK.
        fn next_tile(tiles: &[crate::room::Tile], pos: &mut usize) -> Option<crate::room::Tile> {
            let t = tiles.get(*pos).copied();
            *pos += 1;
            t
        }
        let mut tile_pos = 0;
        let mut inst = insts.next(&self.insts);
        let mut tile = next_tile(&self.room.tiles, &mut tile_pos);
        loop {
            match (inst, tile) {
                (None, None) => break,
                (Some(h), None) => {
                    self.draw_instance(h)?;
                    while let Some(h) = insts.next(&self.insts) {
                        self.draw_instance(h)?;
                    }
                    break;
                }
                (None, Some(t)) => {
                    self.draw_tile(&t);
                    while let Some(t) = next_tile(&self.room.tiles, &mut tile_pos) {
                        self.draw_tile(&t);
                    }
                    break;
                }
                // The tile only if the instance's depth is less: a tie or a NaN draws the instance.
                (Some(h), Some(t)) if self.insts.get(h).depth < t.depth => {
                    self.draw_tile(&t);
                    tile = next_tile(&self.room.tiles, &mut tile_pos);
                }
                (Some(h), Some(_)) => {
                    self.draw_instance(h)?;
                    inst = insts.next(&self.insts);
                }
            }
        }
        self.draw_backgrounds(true, span)
    }

    /// An instance's draw event, or without one, its sprite (the default draw). Invisible instances are
    /// skipped. OpenGMK `game/draw.rs` (`draw_instance`, `draw_instance_default`).
    fn draw_instance(&mut self, h: Inst) -> R<()> {
        let i = self.insts.get(h);
        if !i.visible {
            return Ok(());
        }
        match self.program.handler(i.object, Event::Draw) {
            Some(&handler) => self.run_handler(h, h, handler)?,
            None if let Some(extent) = self.sprite_extent(i.sprite_index) => {
                // The default draw: the sprite at floor(image_index) with the instance's transform. It
                // isn't an event, so it happens even after a draw event requested a room change; GM8 then
                // doesn't show the frame (`World::presents`). The frame is Delphi's 32-bit `Floor`, whose
                // `Trunc` traps on Intel from 2^63 (Spelunky.exe 0x4bf560; probe `draw-floor-trap`, 851-3301), so
                // it's taken even when nothing is recorded.
                let frame = math::floor(i.image_index)? as i32;
                // So are its corners (851-3304): untransformed, through the plain texture draw, otherwise the
                // scaled and rotated one (Spelunky.exe 0x4bf505). A transform that traps there traps in its
                // setter first (the box), but only after it's stored (probe `draw-default-trap`).
                if let Some(e) = extent {
                    let (xs, ys, angle) = (i.image_xscale, i.image_yscale, i.image_angle);
                    if gm8::draw_is_plain(xs, ys, angle, i.image_blend, i.image_alpha) {
                        e.check_plain(i.x, i.y)?;
                    } else {
                        e.check_ext(i.x, i.y, xs, ys, angle)?;
                    }
                }
                if !self.recording_draw() {
                    return Ok(());
                }
                let cmd = draw::Sprite {
                    sprite: i.sprite_index as u32,
                    frame,
                    x: i.x,
                    y: i.y,
                    xscale: i.image_xscale,
                    yscale: i.image_yscale,
                    angle: i.image_angle,
                    blend: draw::Color(i.image_blend as u32),
                    alpha: i.image_alpha,
                };
                self.push_draw(draw::Command::Sprite(cmd));
            }
            None => {}
        }
        Ok(())
    }

    /// A visible tile whose background exists. OpenGMK `game/draw.rs` (`draw_tile`).
    fn draw_tile(&mut self, t: &crate::room::Tile) {
        if !t.visible || self.assets.background(t.background).is_none() {
            return;
        }
        let px = |v: i32| u32::try_from(v).unwrap_or(0);
        self.push_draw(draw::Command::Tile(draw::Tile {
            background: t.background as u32,
            left: px(t.left),
            top: px(t.top),
            width: px(t.width),
            height: px(t.height),
            x: t.x,
            y: t.y,
            xscale: t.xscale,
            yscale: t.yscale,
            blend: draw::Color(t.blend),
            alpha: t.alpha,
        }));
    }

    /// The visible background (or foreground) layers whose background exists, in layer order, over the view
    /// `span` (its left edge and width). Their `Round`s can trap whether or not they're recorded
    /// ([`gm8::draw_background`], probe `draw-stage-trap`, 851-3304).
    fn draw_backgrounds(&mut self, foreground: bool, span: (i32, i32)) -> R<()> {
        for i in 0..self.room.backgrounds.len() {
            let b = self.room.backgrounds[i];
            if !b.visible || b.foreground != foreground {
                continue;
            }
            let Some(bg) = self.assets.background(b.index) else {
                continue;
            };
            let size = (f64::from(bg.width), f64::from(bg.height));
            gm8::draw_background(size, (b.x, b.y), (b.xscale, b.yscale), b.tile_h, span)
                .ok_or(ValueError::IntOverflow)?;
            self.push_draw(draw::Command::Background(draw::Background {
                background: b.index as u32,
                x: b.x,
                y: b.y,
                tile_h: b.tile_h,
                tile_v: b.tile_v,
                xscale: b.xscale,
                yscale: b.yscale,
                blend: draw::Color(b.blend),
                alpha: b.alpha,
            }));
        }
        Ok(())
    }
}
