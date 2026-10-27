//! The drawing, font, window and dialog built-ins (851-3175). OpenGMK `gml/kernel.rs` and
//! `game/draw.rs` (`draw_string`).
//!
//! GM8 keeps a drawing colour, alpha and font that GML sets and later draws read. They persist across
//! steps and room changes, so they live in the world and clone with it (savestates). The draws resolve
//! them into each [`draw::Command`] and record it only while a frame is attached
//! ([`World::step_drawing`]); headless, they still check their arguments (a missing sprite is an error
//! either way) but record nothing.
//!
//! GM8's draws convert their positions through the x87 themselves (`crate::gm8`'s drawing rules: corners stored as
//! singles, `Floor`s and `Round`s into an `Int64`), which traps on Intel past those types (851-3304). The draws
//! check that whether or not they record, so a run's state doesn't depend on whether it's drawn.
//!
//! Text is recorded as one [`draw::Text`] per `draw_text`, and the renderer lays it out with
//! [`draw::SpriteFont::layout`]: one command keeps traces small, and the layout lives in one place.
//!
//! Dialogs can't be shown in a deterministic run. The port answers as if the player dismissed each one
//! straight away (docs/quirks.md section 7): only the leftover SuperSound demo object `core`, which nothing
//! creates, calls them in play.

use crate::ctx::Ctx;
use crate::error::{Error, R};
use crate::gm8;
use crate::math;
use crate::string::GmString;
use crate::value::{Value, ValueError};
use crate::world::World;
use draw::SurfaceId;
use std::ops::RangeInclusive;

/// The characters GM's built-in font (font -1, before any `draw_set_font`) has: `draw_text` draws those and
/// skips the others (probe `draw-builtin-trap`, 851-3304).
const BUILTIN_FONT_CHARS: RangeInclusive<u32> = 32..=127;

/// GM8's drawing settings and the fonts made at run time.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct DrawSettings {
    color: draw::Color,
    alpha: f64,
    /// A font id, or -1 for GM's built-in font.
    font: i32,
    /// The game file's font slots, deleted ones included: the first `font_add_sprite` id.
    font_base: usize,
    /// One per `font_add_sprite`, from id `font_base` (the game file's own fonts are all deleted).
    fonts: Vec<draw::SpriteFont>,
    cursor: i32,
    /// The surfaces that exist (`crate::surfaces`), sorted by id.
    pub(crate) surfaces: Vec<draw::SurfaceSize>,
    /// Whether `action_fullscreen` asked for fullscreen.
    pub(crate) fullscreen: bool,
}

impl DrawSettings {
    /// The id of the first font made at run time (the game file's font slots).
    pub(crate) fn font_base(&self) -> usize {
        self.font_base
    }

    /// Whether loaded settings fit the game: fonts numbered from its `font_slots`, made from sprites it
    /// has, and the current font one of them or -1; surfaces sorted by id, each 1 to
    /// [`draw::MAX_SURFACE_SIZE`] pixels a side.
    pub(crate) fn is_consistent(&self, assets: &crate::Assets, font_slots: usize) -> bool {
        let font = usize::try_from(self.font).ok();
        self.font_base == font_slots
            && self
                .fonts
                .iter()
                .all(|f| i32::try_from(f.sprite).is_ok_and(|s| assets.sprite(s).is_some()))
            && (self.font == -1
                || font.is_some_and(|f| (font_slots..font_slots + self.fonts.len()).contains(&f)))
            && self.surfaces.windows(2).all(|p| p[0].id < p[1].id)
            && self.surfaces.iter().all(|s| {
                let size = 1..=draw::MAX_SURFACE_SIZE;
                size.contains(&s.width) && size.contains(&s.height)
            })
    }

    /// Where surface `id` is in [`surfaces`](Self::surfaces), if it exists.
    pub(crate) fn surface_index(&self, id: i32) -> Option<usize> {
        let id = SurfaceId(u32::try_from(id).ok()?);
        self.surfaces.binary_search_by_key(&id, |s| s.id).ok()
    }

    /// Surface `id`'s size, if it exists.
    pub(crate) fn surface(&self, id: i32) -> Option<(u32, u32)> {
        let s = self.surfaces[self.surface_index(id)?];
        Some((s.width, s.height))
    }
    /// GM8's start: black, opaque, the built-in font, the default cursor (OpenGMK `game.rs`,
    /// `Game::launch`), with fonts numbered after the game file's `font_slots`.
    pub(crate) fn new(font_slots: usize) -> DrawSettings {
        DrawSettings {
            color: draw::Color::BLACK,
            alpha: 1.0,
            font: -1,
            font_base: font_slots,
            fonts: Vec::new(),
            // cr_default.
            cursor: 0,
            surfaces: Vec::new(),
            fullscreen: false,
        }
    }
}

impl World {
    /// `draw_set_color`, and the `action_color` drag-and-drop action. Only the low 24 bits are kept.
    pub fn draw_set_color(&mut self, color: i32) {
        self.draw_settings.color = draw::Color(color as u32 & 0xFF_FF_FF);
    }

    /// `draw_set_alpha`. Kept as given; the renderer clamps.
    pub fn draw_set_alpha(&mut self, alpha: f64) {
        self.draw_settings.alpha = alpha;
    }

    /// `draw_set_font`. An id that isn't a font selects GM's built-in font (-1). OpenGMK `gml/kernel.rs`.
    pub fn draw_set_font(&mut self, font: i32) {
        self.draw_settings.font = if self.font(font).is_some() { font } else { -1 };
    }

    /// The colour `draw_set_color` set.
    pub fn draw_color(&self) -> draw::Color {
        self.draw_settings.color
    }

    /// The alpha `draw_set_alpha` set.
    pub fn draw_alpha(&self) -> f64 {
        self.draw_settings.alpha
    }

    /// The font `draw_set_font` set, or -1.
    pub fn draw_font(&self) -> i32 {
        self.draw_settings.font
    }

    /// A font made by `font_add_sprite`.
    pub fn font(&self, id: i32) -> Option<draw::SpriteFont> {
        let s = &self.draw_settings;
        let i = usize::try_from(id).ok()?.checked_sub(s.font_base)?;
        s.fonts.get(i).copied()
    }

    /// `font_add_sprite(sprite, first, prop, sep)`: a font with one character per frame of `sprite`,
    /// starting at byte `first` (clamped to 0..=255). Returns its id, the next after the game file's fonts
    /// and those made before. OpenGMK `gml/kernel.rs` (`font_add_sprite`).
    pub fn font_add_sprite(&mut self, sprite: i32, first: i32, prop: bool, sep: i32) -> R<i32> {
        if self.assets.sprite(sprite).is_none() {
            return Err(Error::NoSuchSprite(sprite));
        }
        if prop {
            // Proportional fonts need each frame's pixel bounds, which the draw list doesn't carry. Spelunky
            // only makes monospaced ones.
            return Err(Error::Unsupported("proportional sprite fonts"));
        }
        let s = &mut self.draw_settings;
        let id = (s.font_base + s.fonts.len()) as i32;
        s.fonts.push(draw::SpriteFont {
            sprite: sprite as u32,
            first: first.clamp(0, 255) as u8,
            separation: sep,
        });
        Ok(id)
    }

    /// `draw_text(x, y, text)`: `text` as `string()` would show it, in the drawing colour, alpha and font.
    /// With GM's built-in font (no `draw_set_font` yet, or an invalid one) nothing is recorded: the port
    /// can't draw it, and the game only does so in the unused `core` object.
    ///
    /// GM8 formats a real itself (Spelunky.exe 0x55433f), with `string()`'s rule but not its handler: its `Round`
    /// into an `Int64` traps from 2^63 instead of giving `"ERROR"`. Each character the font has then goes through
    /// the scaled texture draw ([`gm8::draw_quad_ext`]) at the text's position plus some pixels; those are far
    /// below the doubles' spacing where a corner reaches [`gm8::SINGLE_LIMIT`], so a corner traps exactly where
    /// `x` or `y` itself would (probe `draw-builtin-trap`, 851-3304).
    pub fn draw_text(&mut self, x: f64, y: f64, text: &Value) -> R<()> {
        if let Value::Real(r) = *text {
            math::round(r)?;
        }
        if (gm8::single(x).is_none() || gm8::single(y).is_none()) && self.text_draws(text) {
            return Err(ValueError::Overflow.into());
        }
        if !self.recording_draw() {
            return Ok(());
        }
        let s = &self.draw_settings;
        let Some(font) = self.font(s.font) else {
            return Ok(());
        };
        let cmd = draw::Text {
            font,
            x,
            y,
            text: text.to_gml_string().as_bytes().to_vec(),
            color: s.color,
            alpha: s.alpha,
        };
        self.push_draw(cmd);
        Ok(())
    }

    /// Whether `draw_text` draws any character of `text` in the current font ([`gm8::text_draws`]): a sprite font
    /// has its sprite's frames from its first character, to 255 at most, GM's own [`BUILTIN_FONT_CHARS`] (probe
    /// `draw-builtin-trap`, 851-3304). GM8 skips the others, where the renderer draws a space
    /// ([`draw::SpriteFont::layout`], after OpenGMK).
    fn text_draws(&self, text: &Value) -> bool {
        let chars = match self.font(self.draw_settings.font) {
            Some(f) => {
                let frames = self
                    .assets
                    .sprite(f.sprite as i32)
                    .map_or(0, |s| s.frames.len());
                if frames == 0 {
                    return false;
                }
                let first = usize::from(f.first);
                first as u32..=(first + frames - 1).min(255) as u32
            }
            None => BUILTIN_FONT_CHARS,
        };
        gm8::text_draws(text.to_gml_string().as_bytes(), chars)
    }

    /// `draw_sprite(sprite, subimg, x, y)`: unscaled, white and opaque whatever the drawing colour and
    /// alpha are. A negative `subimg` means the calling instance's `floor(image_index)`. OpenGMK
    /// `gml/kernel.rs` (`draw_sprite`). GM8 draws it through its plain texture draw ([`gm8::draw_quad`],
    /// Spelunky.exe 0x4a5a58), whose corners can trap.
    pub fn draw_sprite(&mut self, cx: &Ctx, sprite: i32, subimg: i32, x: f64, y: f64) -> R<()> {
        let white = draw::Color::WHITE.0 as i32;
        self.sprite_draw(cx, sprite, subimg, x, y, 1.0, 1.0, 0.0, white, 1.0, true)
    }

    /// `draw_sprite_ext(sprite, subimg, x, y, xscale, yscale, angle, color, alpha)`, as
    /// [`draw_sprite`](Self::draw_sprite) but with its own transform, blend colour and alpha. GM8 draws it through
    /// its scaled and rotated texture draw ([`gm8::draw_quad_ext`], Spelunky.exe 0x4a5aa4), whatever the transform.
    #[allow(clippy::too_many_arguments)] // GML's signature.
    pub fn draw_sprite_ext(
        &mut self,
        cx: &Ctx,
        sprite: i32,
        subimg: i32,
        x: f64,
        y: f64,
        xscale: f64,
        yscale: f64,
        angle: f64,
        color: i32,
        alpha: f64,
    ) -> R<()> {
        self.sprite_draw(
            cx, sprite, subimg, x, y, xscale, yscale, angle, color, alpha, false,
        )
    }

    /// A sprite draw, through the plain texture draw or the scaled and rotated one. A negative `subimg` is the
    /// calling instance's Delphi `Floor` of `image_index` (32 bits), whose `Trunc` traps from 2^63; GM8 takes it
    /// before it checks the sprite (Spelunky.exe 0x554b29, probe `draw-builtin-trap`, 851-3304). A sprite without
    /// frames draws nothing.
    #[allow(clippy::too_many_arguments)]
    fn sprite_draw(
        &mut self,
        cx: &Ctx,
        sprite: i32,
        subimg: i32,
        x: f64,
        y: f64,
        xscale: f64,
        yscale: f64,
        angle: f64,
        color: i32,
        alpha: f64,
        plain: bool,
    ) -> R<()> {
        let frame = if subimg < 0 {
            math::floor(self.insts.get(cx.this).image_index)? as i32
        } else {
            subimg
        };
        let Some(extent) = self.sprite_extent(sprite) else {
            return Err(Error::NoSuchSprite(sprite));
        };
        let Some(extent) = extent else {
            return Ok(());
        };
        if plain {
            extent.check_plain(x, y)?;
        } else {
            extent.check_ext(x, y, xscale, yscale, angle)?;
        }
        self.push_draw(draw::Sprite {
            sprite: sprite as u32,
            frame,
            x,
            y,
            xscale,
            yscale,
            angle,
            blend: draw::Color(color as u32 & 0xFF_FF_FF),
            alpha,
        });
        Ok(())
    }

    /// `draw_rectangle(x1, y1, x2, y2, outline)` in the drawing colour and alpha. GM8 takes `Floor`s and stores
    /// corners that can trap ([`gm8::draw_rectangle`]).
    pub fn draw_rectangle(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, outline: bool) -> R<()> {
        gm8::draw_rectangle(x1, y1, x2, y2).map_err(|t| match t {
            gm8::DrawTrap::Invalid => ValueError::IntOverflow,
            gm8::DrawTrap::Overflow => ValueError::Overflow,
        })?;
        let s = &self.draw_settings;
        let cmd = draw::Command::Rectangle {
            x1,
            y1,
            x2,
            y2,
            color: s.color,
            alpha: s.alpha,
            outline,
        };
        self.push_draw(cmd);
        Ok(())
    }

    /// `draw_circle(x, y, r, outline)` in the drawing colour and alpha. GM8 stores bounds and points that can trap
    /// ([`gm8::draw_circle`]).
    pub fn draw_circle(&mut self, x: f64, y: f64, radius: f64, outline: bool) -> R<()> {
        gm8::draw_circle(x, y, radius).ok_or(ValueError::Overflow)?;
        let s = &self.draw_settings;
        let cmd = draw::Command::Circle {
            x,
            y,
            radius,
            color: s.color,
            alpha: s.alpha,
            outline,
        };
        self.push_draw(cmd);
        Ok(())
    }

    /// `window_set_cursor`. Nothing in GML reads it back; a frontend can ([`World::window_cursor`]).
    pub fn window_set_cursor(&mut self, cursor: i32) {
        self.draw_settings.cursor = cursor;
    }

    /// The cursor `window_set_cursor` set: the game uses `cr_none` (-1) in play and `cr_default` (0) in
    /// the level editor.
    pub fn window_cursor(&self) -> i32 {
        self.draw_settings.cursor
    }

    /// `show_message(text)`: dismissed at once.
    pub fn show_message(&self, _text: &[u8]) {}

    /// `show_info()`: the game information window, closed at once.
    pub fn show_info(&self) {}

    /// `show_menu(items, def)`: closed without a choice, which gives `def`. GM8 returns an item index, so
    /// `def` is rounded as GM8 rounds an integer argument. Assumed: OpenGMK doesn't implement `show_menu`.
    pub fn show_menu(&self, _items: &[u8], def: i32) -> i32 {
        def
    }

    /// `get_integer(prompt, def)`: accepted as filled in, which gives `def`.
    pub fn get_integer(&self, _prompt: &[u8], def: f64) -> f64 {
        def
    }

    /// `show_error(text, abort)`: stops the game with [`Error::ShowError`], as the oracle does.
    pub fn show_error(&self, text: &[u8], _abort: bool) -> R<()> {
        Err(Error::ShowError(GmString::from(text)))
    }
}
