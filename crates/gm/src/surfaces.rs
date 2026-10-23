//! Surfaces, blend modes and the screen built-ins (851-3160). OpenGMK `gml/kernel.rs` (`surface_*`,
//! `draw_surface*`, `draw_set_blend_mode*`, `draw_clear`, `screen_refresh`, `action_fullscreen`) and
//! `render/opengl.rs` (`set_target`, `reset_target`).
//!
//! Surfaces are logical here: an id and a size. Their pixels live only in the renderer, which runs the
//! recorded [`draw::Command`]s. GML sees the ids (`surface_create`, `surface_exists`), so they are kept, and
//! answer the same, whether or not a frame is attached. So does the drawing state the commands change
//! (target, projection, blend mode, in [`World::draw_state`]), which outlives a step: `oScreen` leaves its
//! `screen` surface as the target at every end step for the draw stage and the next begin step.

use crate::error::{Error, R};
use crate::world::World;
use draw::{BlendMode, Command, SurfaceId, Target};

impl World {
    /// A surface's size, if `id` is one.
    pub fn surface_size(&self, id: i32) -> Option<(u32, u32)> {
        let slot = usize::try_from(id).ok()?;
        self.draw_settings.surfaces.get(slot).copied().flatten()
    }

    /// The surfaces that exist, by id.
    pub fn surface_sizes(&self) -> Vec<draw::SurfaceSize> {
        let slots = self.draw_settings.surfaces.iter().enumerate();
        slots
            .filter_map(|(id, s)| {
                s.map(|(width, height)| draw::SurfaceSize {
                    id: SurfaceId(id as u32),
                    width,
                    height,
                })
            })
            .collect()
    }

    /// `surface_create(w, h)`: a surface with the lowest free id, from 0 (OpenGMK's `HandleList::put`), so
    /// `oScreen`'s `pSurf`, `screen` and `darkSurf` are 0, 1 and 2. On a working machine it never fails;
    /// sizes under 1 aren't supported (GM8 would fail in Direct3D, and the game never asks).
    pub fn surface_create(&mut self, w: i32, h: i32) -> R<i32> {
        let (Ok(width @ 1..), Ok(height @ 1..)) = (u32::try_from(w), u32::try_from(h)) else {
            return Err(Error::Unsupported("surface_create with a size under 1"));
        };
        let slots = &mut self.draw_settings.surfaces;
        let id = match slots.iter().position(Option::is_none) {
            Some(free) => free,
            None => {
                slots.push(None);
                slots.len() - 1
            }
        };
        slots[id] = Some((width, height));
        self.push_draw(Command::CreateSurface {
            id: SurfaceId(id as u32),
            width,
            height,
        });
        Ok(id as i32)
    }

    /// `surface_exists(id)`.
    pub fn surface_exists(&self, id: i32) -> bool {
        self.surface_size(id).is_some()
    }

    /// `surface_free(id)`. Freeing the current target first resets the target to the screen. An id that
    /// isn't a surface does nothing.
    pub fn surface_free(&mut self, id: i32) {
        if id >= 0 && self.draw_state.target == Target::Surface(SurfaceId(id as u32)) {
            self.surface_reset_target();
        }
        if self.surface_exists(id) {
            self.draw_settings.surfaces[id as usize] = None;
            self.push_draw(Command::FreeSurface(SurfaceId(id as u32)));
        }
    }

    /// `surface_set_target(id)`: draw into the surface, with the projection reset to the whole surface
    /// (OpenGMK's renderer `set_target`). An id that isn't a surface does nothing, so the target stays.
    pub fn surface_set_target(&mut self, id: i32) {
        if self.surface_exists(id) {
            self.apply_draw(Command::SetTarget(Target::Surface(SurfaceId(id as u32))));
        }
    }

    /// `surface_reset_target()`: draw into the back buffer again, with the projection reset to the whole
    /// back buffer (OpenGMK's renderer `reset_target`), even if it already was the target. A view's port
    /// only applies again from the next draw stage.
    pub fn surface_reset_target(&mut self) {
        self.apply_draw(Command::SetTarget(Target::Screen));
    }

    /// `draw_surface(id, x, y)`: unscaled, white and opaque, whatever `draw_set_color`/`draw_set_alpha`
    /// say. An id that isn't a surface draws nothing.
    pub fn draw_surface(&mut self, id: i32, x: f64, y: f64) {
        self.draw_surface_scaled(id, x, y, |_, _| (1.0, 1.0));
    }

    /// `draw_surface_stretched(id, x, y, w, h)`: `draw_surface` scaled to `w` x `h` pixels.
    pub fn draw_surface_stretched(&mut self, id: i32, x: f64, y: f64, w: f64, h: f64) {
        self.draw_surface_scaled(id, x, y, |sw, sh| (w / f64::from(sw), h / f64::from(sh)));
    }

    fn draw_surface_scaled(
        &mut self,
        id: i32,
        x: f64,
        y: f64,
        scale: impl Fn(u32, u32) -> (f64, f64),
    ) {
        let Some((w, h)) = self.surface_size(id) else {
            return;
        };
        let (xscale, yscale) = scale(w, h);
        self.push_draw(draw::SurfaceDraw {
            surface: SurfaceId(id as u32),
            x,
            y,
            xscale,
            yscale,
            blend: draw::Color::WHITE,
            alpha: 1.0,
        });
    }

    /// `draw_set_blend_mode(mode)`: `bm_normal`, `bm_add`, `bm_max`, `bm_subtract`; any other is `bm_normal`.
    /// It stays until changed, across steps and targets.
    pub fn draw_set_blend_mode(&mut self, mode: i32) {
        self.apply_draw(Command::SetBlend(BlendMode::from_gml(mode)));
    }

    /// `draw_set_blend_mode_ext(src, dest)`, with `bm_*` factors (an unknown one is `bm_zero`).
    pub fn draw_set_blend_mode_ext(&mut self, src: i32, dest: i32) {
        self.apply_draw(Command::SetBlend(BlendMode::from_gml_ext(src, dest)));
    }

    /// `draw_clear(color)`: fills the target with `color`, opaque, without blending.
    pub fn draw_clear(&mut self, color: i32) {
        self.push_draw(Command::Clear {
            color: draw::Color(color as u32 & 0xFF_FF_FF),
            alpha: 1.0,
        });
    }

    /// `screen_redraw()`: the whole draw stage, draw events included, on the same path as the step's own
    /// (runtime.md section 1), into whatever the target is; then, like `screen_refresh`, it shows the back
    /// buffer. Each call runs every draw event once more and records what they draw once.
    pub fn screen_redraw(&mut self) -> R<()> {
        self.redraw()?;
        self.screen_refresh();
        Ok(())
    }

    /// `screen_refresh()`: shows the back buffer now. The picture shown is the back buffer as the tick
    /// leaves it (nothing in the game draws to it after `oScreen`'s refresh), so this records nothing; it
    /// only makes the tick count as shown even if it ends with a room change pending
    /// ([`presents`](Self::presents)).
    pub fn screen_refresh(&mut self) {
        self.presented = true;
    }

    /// `action_fullscreen(action)`: 1 windowed, 2 fullscreen, anything else switches. Nothing in GML reads
    /// it back; a frontend can ([`fullscreen`](Self::fullscreen)). OpenGMK ignores it.
    pub fn action_fullscreen(&mut self, action: i32) {
        let s = &mut self.draw_settings;
        s.fullscreen = match action {
            1 => false,
            2 => true,
            _ => !s.fullscreen,
        };
    }

    /// Whether `action_fullscreen` asked for fullscreen.
    pub fn fullscreen(&self) -> bool {
        self.draw_settings.fullscreen
    }
}
