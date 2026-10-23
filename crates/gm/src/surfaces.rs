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
use draw::{BlendMode, Command, SurfaceId, SurfaceSize, Target};

impl World {
    /// A surface's size, if `id` is one.
    pub fn surface_size(&self, id: i32) -> Option<(u32, u32)> {
        self.draw_settings.surface(id)
    }

    /// The surfaces that exist, by id.
    pub fn surface_sizes(&self) -> &[SurfaceSize] {
        &self.draw_settings.surfaces
    }

    /// `surface_create(w, h)`: a surface with the lowest free id, from 0 (OpenGMK's `HandleList::put`), so
    /// `oScreen`'s `pSurf`, `screen` and `darkSurf` are 0, 1 and 2. A size outside 1 to
    /// [`draw::MAX_SURFACE_SIZE`] is an error, as in the oracle, where OpenGL fails to make the texture and
    /// OpenGMK raises one (GM8 returned -1 when Direct3D failed, at a limit that depended on the machine).
    /// The game only asks for 320x240.
    pub fn surface_create(&mut self, w: i32, h: i32) -> R<i32> {
        let size = 1..=draw::MAX_SURFACE_SIZE as i32;
        if !size.contains(&w) || !size.contains(&h) {
            return Err(Error::SurfaceSize(w, h));
        }
        let surfaces = &mut self.draw_settings.surfaces;
        // The list is sorted by id, so the first id that doesn't match its position is the lowest free.
        let at = surfaces
            .iter()
            .enumerate()
            .position(|(i, s)| s.id.0 as usize != i)
            .unwrap_or(surfaces.len());
        let s = SurfaceSize {
            id: SurfaceId(at as u32),
            width: w as u32,
            height: h as u32,
        };
        surfaces.insert(at, s);
        self.push_draw(Command::CreateSurface {
            id: s.id,
            width: s.width,
            height: s.height,
        });
        Ok(at as i32)
    }

    /// `surface_exists(id)`.
    pub fn surface_exists(&self, id: i32) -> bool {
        self.surface_size(id).is_some()
    }

    /// `surface_free(id)`. Freeing the current target first resets the target to the screen. An id that
    /// isn't a surface does nothing.
    pub fn surface_free(&mut self, id: i32) {
        let Some(at) = self.draw_settings.surface_index(id) else {
            return;
        };
        let surface = SurfaceId(id as u32);
        if self.draw_state.target == Target::Surface(surface) {
            self.surface_reset_target();
        }
        self.draw_settings.surfaces.remove(at);
        self.push_draw(Command::FreeSurface(surface));
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
        if self.surface_exists(id) {
            self.push_surface(id, x, y, 1.0, 1.0);
        }
    }

    /// `draw_surface_stretched(id, x, y, w, h)`: `draw_surface` scaled to `w` x `h` pixels.
    pub fn draw_surface_stretched(&mut self, id: i32, x: f64, y: f64, w: f64, h: f64) {
        if let Some((sw, sh)) = self.surface_size(id) {
            self.push_surface(id, x, y, w / f64::from(sw), h / f64::from(sh));
        }
    }

    /// Records drawing surface `id`, which exists.
    fn push_surface(&mut self, id: i32, x: f64, y: f64, xscale: f64, yscale: f64) {
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

    /// `screen_refresh()`: shows the back buffer as it is now ([`draw::Command::Present`]), and makes the
    /// tick count as shown even if it then fails or ends with a room change pending
    /// ([`presents`](Self::presents)).
    pub fn screen_refresh(&mut self) {
        self.present();
    }

    /// GM8 shows the back buffer.
    pub(crate) fn present(&mut self) {
        self.presented = true;
        self.push_draw(Command::Present);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Assets, ProgramBuilder};
    use draw::{Color, Rect};
    use std::sync::Arc;

    /// A started world in an empty 320x240 room.
    fn world() -> World {
        let program = Box::leak(Box::new(ProgramBuilder::new().build()));
        let room = assets::Room {
            width: 320,
            height: 240,
            speed: 30,
            ..Default::default()
        };
        let assets = Assets::new(Vec::new(), vec![Some(room)]);
        let mut g = World::new(program, Arc::new(assets), 1, 100_000, 0);
        g.start_game().unwrap();
        g
    }

    /// The commands `f` records.
    fn recorded(g: &mut World, f: impl FnOnce(&mut World)) -> Vec<Command> {
        let mut frame = draw::Frame::default();
        g.record_drawing(&mut frame, f);
        frame.commands
    }

    fn create(id: u32, width: u32, height: u32) -> Command {
        Command::CreateSurface {
            id: SurfaceId(id),
            width,
            height,
        }
    }

    #[test]
    fn surface_ids_are_the_lowest_free_slot() {
        let mut g = world();
        let cmds = recorded(&mut g, |g| {
            assert_eq!(g.surface_create(320, 240).unwrap(), 0);
            assert_eq!(g.surface_create(16, 8).unwrap(), 1);
            assert_eq!(g.surface_create(4, 4).unwrap(), 2);
            g.surface_free(1);
            // Freeing what isn't a surface does nothing.
            g.surface_free(1);
            g.surface_free(-1);
            g.surface_free(7);
            assert!(!g.surface_exists(1));
            assert_eq!(g.surface_create(2, 2).unwrap(), 1, "a freed id is reused");
            assert_eq!(g.surface_create(3, 3).unwrap(), 3);
        });
        let sizes: Vec<_> = g
            .surface_sizes()
            .iter()
            .map(|s| (s.id.0, s.width, s.height))
            .collect();
        assert_eq!(sizes, [(0, 320, 240), (1, 2, 2), (2, 4, 4), (3, 3, 3)]);
        assert!(g.surface_exists(0) && !g.surface_exists(-1) && !g.surface_exists(4));
        assert_eq!(g.surface_size(2), Some((4, 4)));
        assert_eq!(
            cmds,
            [
                create(0, 320, 240),
                create(1, 16, 8),
                create(2, 4, 4),
                Command::FreeSurface(SurfaceId(1)),
                create(1, 2, 2),
                create(3, 3, 3),
            ]
        );
    }

    #[test]
    fn sizes_outside_the_limit_are_errors() {
        let mut g = world();
        let max = draw::MAX_SURFACE_SIZE as i32;
        assert_eq!(g.surface_create(max, 1).unwrap(), 0);
        for (w, h) in [(0, 240), (320, -1), (max + 1, 240), (320, i32::MAX)] {
            assert!(
                matches!(g.surface_create(w, h), Err(Error::SurfaceSize(..))),
                "{w}x{h}"
            );
        }
        assert_eq!(g.surface_sizes().len(), 1);
    }

    #[test]
    fn targets_switch_and_reset_the_projection() {
        let mut g = world();
        let window = Rect::new(0, 0, 320, 240);
        let cmds = recorded(&mut g, |g| {
            g.surface_create(64, 32).unwrap();
            g.surface_set_target(0);
            assert_eq!(g.draw_state().target, Target::Surface(SurfaceId(0)));
            assert_eq!(g.draw_state().view, Rect::new(0, 0, 64, 32));
            // Not a surface: the target stays.
            g.surface_set_target(5);
            assert_eq!(g.draw_state().target, Target::Surface(SurfaceId(0)));
            g.surface_reset_target();
            assert_eq!(g.draw_state().target, Target::Screen);
            assert_eq!((g.draw_state().view, g.draw_state().port), (window, window));
            // Freeing the target resets it to the screen first.
            g.surface_set_target(0);
            g.surface_free(0);
            assert_eq!(g.draw_state().target, Target::Screen);
        });
        let screen = Command::SetTarget(Target::Screen);
        let surface = Command::SetTarget(Target::Surface(SurfaceId(0)));
        assert_eq!(
            cmds[1..],
            [
                surface.clone(),
                screen.clone(),
                surface,
                screen,
                Command::FreeSurface(SurfaceId(0))
            ]
        );
    }

    #[test]
    fn surface_draws_ignore_the_drawing_colour_and_missing_surfaces() {
        let mut g = world();
        let cmds = recorded(&mut g, |g| {
            g.surface_create(320, 240).unwrap();
            g.draw_set_color(0x123456);
            g.draw_set_alpha(0.25);
            g.draw_surface(0, 3.0, 4.0);
            g.draw_surface_stretched(0, 0.0, 0.0, 960.0, 120.0);
            g.draw_surface(1, 0.0, 0.0);
            g.draw_surface_stretched(-1, 0.0, 0.0, 1.0, 1.0);
            g.draw_clear(-1);
        });
        let surface = |x, y, xscale, yscale| {
            Command::Surface(draw::SurfaceDraw {
                surface: SurfaceId(0),
                x,
                y,
                xscale,
                yscale,
                blend: Color::WHITE,
                alpha: 1.0,
            })
        };
        assert_eq!(
            cmds[1..],
            [
                surface(3.0, 4.0, 1.0, 1.0),
                surface(0.0, 0.0, 3.0, 0.5),
                // Only the colour's low 24 bits count, and the clear is opaque.
                Command::Clear {
                    color: Color::WHITE,
                    alpha: 1.0
                },
            ]
        );
    }

    #[test]
    fn recording_starts_from_the_surfaces_and_state_and_never_nests() {
        let mut g = world();
        g.surface_create(8, 8).unwrap();
        g.surface_set_target(0);
        let mut frame = draw::Frame::default();
        g.record_drawing(&mut frame, |_| {});
        assert_eq!(frame.surfaces, g.surface_sizes());
        assert_eq!(frame.start, g.draw_state());
        assert_eq!((frame.width, frame.height), (320, 240));
        // A clone made while recording doesn't record.
        let copy = g.record_drawing(&mut frame, |g| g.clone());
        assert!(!copy.recording_draw());
    }

    #[test]
    #[should_panic(expected = "already recording")]
    #[cfg_attr(target_family = "wasm", ignore)]
    fn recordings_never_nest() {
        let mut g = world();
        let mut outer = draw::Frame::default();
        g.record_drawing(&mut outer, |g| {
            g.record_drawing(&mut draw::Frame::default(), |_| {});
        });
    }
}
