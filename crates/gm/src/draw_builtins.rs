//! The drawing, font, window and dialog built-ins (851-3175). OpenGMK `gml/kernel.rs` and
//! `game/draw.rs` (`draw_string`).
//!
//! GM8 keeps a drawing colour, alpha and font that GML sets and later draws read. They persist across
//! steps and room changes, so they live in the world and clone with it (savestates). The draws resolve
//! them into each [`draw::Command`] and record it only while a frame is attached
//! ([`World::step_drawing`]); headless, they still check their arguments (a missing sprite is an error
//! either way) but record nothing.
//!
//! Text is recorded as one [`draw::Text`] per `draw_text`, and the renderer lays it out with
//! [`draw::SpriteFont::layout`]: one command keeps traces small, and the layout lives in one place.
//!
//! Dialogs can't be shown in a deterministic run. The port answers as if the player dismissed each one
//! straight away (docs/quirks.md section 7): only the leftover SuperSound demo object `core`, which nothing
//! creates, calls them in play.

use crate::ctx::Ctx;
use crate::error::{Error, R};
use crate::string::GmString;
use crate::value::Value;
use crate::world::World;

/// GM8's drawing settings and the fonts made at run time.
#[derive(Clone, Debug, PartialEq)]
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
}

impl Default for DrawSettings {
    /// GM8's start: black, opaque, the built-in font, the default cursor. OpenGMK `game.rs` (`Game::launch`).
    fn default() -> DrawSettings {
        DrawSettings {
            color: draw::Color::BLACK,
            alpha: 1.0,
            font: -1,
            font_base: 0,
            fonts: Vec::new(),
            // cr_default.
            cursor: 0,
        }
    }
}

impl World {
    /// Declares how many font slots the game file has, deleted ones included (Spelunky's has one,
    /// deleted). GM8 numbers the fonts `font_add_sprite` makes after them, so call this before the game
    /// runs; the generated game does, from `FONT_SLOTS`.
    pub fn set_font_slots(&mut self, slots: usize) {
        debug_assert!(
            self.draw_settings.fonts.is_empty(),
            "font slots set after fonts were made"
        );
        self.draw_settings.font_base = slots;
    }

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
    pub fn draw_text(&mut self, x: f64, y: f64, text: &Value) {
        if !self.recording_draw() {
            return;
        }
        let s = &self.draw_settings;
        let Some(font) = self.font(s.font) else {
            return;
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
    }

    /// `draw_sprite(sprite, subimg, x, y)`: unscaled, white and opaque whatever the drawing colour and
    /// alpha are. A negative `subimg` means the calling instance's `floor(image_index)`. OpenGMK
    /// `gml/kernel.rs` (`draw_sprite`).
    pub fn draw_sprite(&mut self, cx: &Ctx, sprite: i32, subimg: i32, x: f64, y: f64) -> R<()> {
        self.draw_sprite_ext(
            cx,
            sprite,
            subimg,
            x,
            y,
            1.0,
            1.0,
            0.0,
            draw::Color::WHITE.0 as i32,
            1.0,
        )
    }

    /// `draw_sprite_ext(sprite, subimg, x, y, xscale, yscale, angle, color, alpha)`, as
    /// [`draw_sprite`](Self::draw_sprite) but with its own transform, blend colour and alpha.
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
        let Some(s) = self.assets.sprite(sprite) else {
            return Err(Error::NoSuchSprite(sprite));
        };
        // A sprite without frames draws nothing.
        if !self.recording_draw() || s.frames.is_empty() {
            return Ok(());
        }
        let frame = if subimg < 0 {
            self.insts.get(cx.this).image_index.floor() as i64 as i32
        } else {
            subimg
        };
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

    /// `draw_rectangle(x1, y1, x2, y2, outline)` in the drawing colour and alpha.
    pub fn draw_rectangle(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, outline: bool) {
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
    }

    /// `draw_circle(x, y, r, outline)` in the drawing colour and alpha.
    pub fn draw_circle(&mut self, x: f64, y: f64, radius: f64, outline: bool) {
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

    /// `show_menu(items, def)`: closed without a choice, which gives `def`.
    pub fn show_menu(&self, _items: &[u8], def: f64) -> f64 {
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
