//! Draw lists: what the game emits each tick and the frontend renders.
//!
//! A [`Frame`] is plain data with no graphics dependency, so `gm` can build one headlessly (and tests can
//! compare them) while `app` turns it into GPU work. Commands run strictly in order against GM 8's
//! drawing state: a current target (the screen or a surface), a current view projection, and a current
//! blend mode. Everything else GM keeps as draw state (`draw_set_color`, `draw_set_alpha`,
//! `draw_set_font`) is resolved by the emitter and baked into each command, because the renderer never
//! needs to know where a colour came from.
//!
//! # Coordinates
//!
//! Positions are GM room coordinates (`f64`, like GML reals), mapped through the current view. The
//! renderer reproduces Direct3D 8's rasterisation, which is what GM 8 used: pixel centres sit on integer
//! coordinates, and GM pulls sprite, background, tile and surface quads up and left by half a pixel to
//! compensate. Rectangles, lines and circles don't get that shift, which is why `draw_rectangle(0, 0, 10,
//! 10)` fills 11x11 pixels. The emitter passes GML's values untouched; the renderer applies these rules.
//!
//! # Draw order
//!
//! The emitter is responsible for the order. For each visible view, GM 8 draws (runner behaviour,
//! confirmed against OpenGMK's `Game::draw` and `Game::draw_view` in `gm8emulator/src/game/draw.rs`):
//!
//! 1. Before the first view, the whole window is cleared to the settings' "colour outside the room".
//! 2. [`Command::SetView`] for the view, then [`Command::Clear`] with the room colour if the room draws
//!    its background colour.
//! 3. Every visible non-foreground room background, in layer order ([`Command::Background`]).
//! 4. Instances and tiles merged by depth, **highest depth first**. Only active (not deactivated)
//!    instances take part; invisible ones are skipped without running their draw event. Each instance
//!    either runs its draw event (which emits any commands) or gets the default draw, a
//!    [`Command::Sprite`] of `sprite_index` at `floor(image_index)` with its `image_*` values.
//! 5. Every visible foreground room background, in layer order.
//!
//! **Ties.** Instances live in one persistent draw-order list and tiles in another. New instances and
//! tiles are appended to the end of their list. Each time a view is drawn, each list is re-sorted in
//! place by depth, highest first, with a *stable* sort that starts from the order the previous sort
//! left. Equal depths therefore keep their previous relative order: for things whose depth never
//! changed that's creation order, but an instance whose depth changes slots in among its new peers
//! according to where it sat in the old order, not by creation time. When the next instance and the next
//! tile have the same depth, the instance is drawn first. (Sources: OpenGMK's `InstanceList::draw_sort`,
//! `TileList::draw_sort` and the merge loop in `Game::draw_view`; GM has no particle systems in this game,
//! so the third list in that loop is always empty.)
//!
//! # Surfaces and blending
//!
//! Surfaces are identified by the emitter's own ids and live until [`Command::FreeSurface`]; their
//! contents persist across frames, as in GM. [`Command::SetTarget`] resets the view to the target's full
//! size, like `surface_set_target`/`surface_reset_target`. A [`Command::SetView`] port is ignored while a
//! surface is the target, because Direct3D 8's viewport call does nothing then: the view's rectangle is
//! stretched over the whole surface. That is the behaviour `oScreen`'s view-scaling trick relies on.
//!
//! Blend factors apply to the alpha channel exactly as they do to colour (Direct3D 8 had no separate alpha
//! blend), so drawing onto a surface with `bm_normal` leaves translucent alpha behind, and the game's
//! "alpha fix" rectangle (`bm_src_color, bm_one` in black) repairs it.
//!
//! # Timing
//!
//! The game ticks at 30 Hz and emits one frame per tick. A frame holds every draw call of the tick in
//! execution order, not just the draw stage's: Spelunky's `oScreen` draws the screen surface onto the back
//! buffer by hand in its begin-step event and calls `screen_refresh()`, then the draw stage renders the next
//! picture into that surface. The window shows the back buffer as it stands after the whole frame. The
//! frontend renders each frame exactly once (surfaces make re-rendering observable) and presents the latest
//! result at display rate, with no interpolation.
//!
//! # State between frames
//!
//! GM's drawing state outlives a step: a blend mode set with `draw_set_blend_mode` stays until changed,
//! and `oScreen` leaves its surface as the target at the end of every step (end-step event) for the draw
//! stage and the next begin step to inherit. Each [`Frame`] therefore records the state it starts in
//! ([`Frame::start`], a [`DrawState`]) instead of the renderer remembering the previous frame. The emitter
//! tracks this state anyway, so frames stay self-contained: a frame renders the same way whether or not the
//! previous one was rendered, after a savestate is loaded, or when replayed from a headless trace.
//! Surfaces and the back buffer's pixels are the only things the renderer carries between frames. A frame
//! also lists the surfaces that exist when it starts ([`Frame::surfaces`]): a renderer that missed the
//! commands creating them (a loaded savestate, a frontend attached mid-run) makes them blank, so draws into
//! them aren't lost. Their old contents are, which a game redrawing its surfaces every step, like
//! Spelunky, doesn't notice.
//!
//! # Producing frames
//!
//! `gm`'s draw built-ins record into a [`DrawSink`] when one is attached and do nothing otherwise, so a
//! headless run pays nothing for drawing. [`Frame`] is the recording sink; [`Discard`] is the no-op one.
//! [`Frame::replay`] feeds a recorded frame to another sink.

/// Where draw commands go as the game runs its draw calls.
pub trait DrawSink {
    /// Starts a frame with GM's back buffer at `width` x `height`, GM's drawing state as it stands, and
    /// the surfaces that exist (see [`Frame::surfaces`]).
    fn begin(&mut self, width: u32, height: u32, start: DrawState, surfaces: &[SurfaceSize]);
    fn command(&mut self, command: Command);
}

/// A sink that drops everything (headless runs).
#[derive(Debug, Clone, Copy, Default)]
pub struct Discard;

impl DrawSink for Discard {
    fn begin(&mut self, _width: u32, _height: u32, _start: DrawState, _surfaces: &[SurfaceSize]) {}
    fn command(&mut self, _command: Command) {}
}

impl DrawSink for Frame {
    fn begin(&mut self, width: u32, height: u32, start: DrawState, surfaces: &[SurfaceSize]) {
        self.width = width;
        self.height = height;
        self.start = start;
        self.surfaces.clear();
        self.surfaces.extend_from_slice(surfaces);
        self.commands.clear();
    }

    fn command(&mut self, command: Command) {
        self.commands.push(command);
    }
}

/// GM's drawing state that carries from one command (and one step) to the next.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DrawState {
    pub target: Target,
    /// The current projection, as the last [`Command::SetTarget`] or [`Command::SetView`] left it.
    pub view: Rect,
    /// Ignored while a surface is the target (see the module docs).
    pub port: Rect,
    pub blend: BlendMode,
}

impl DrawState {
    /// GM's state at game start: drawing to a `width` x `height` back buffer, 1:1, with `bm_normal`.
    pub const fn initial(width: u32, height: u32) -> DrawState {
        let full = Rect::new(0, 0, width as i32, height as i32);
        DrawState {
            target: Target::Screen,
            view: full,
            port: full,
            blend: BlendMode::NORMAL,
        }
    }

    /// The state after running `command` from this one, for emitters that track it alongside the commands.
    /// `target_size` gives a surface's size (`None` if it doesn't exist, which makes setting it a no-op);
    /// `screen` is the back buffer's size.
    pub fn apply(
        &mut self,
        command: &Command,
        screen: (u32, u32),
        target_size: impl Fn(SurfaceId) -> Option<(u32, u32)>,
    ) {
        match command {
            Command::SetTarget(target) => {
                let size = match target {
                    Target::Screen => Some(screen),
                    Target::Surface(id) => target_size(*id),
                };
                if let Some((w, h)) = size {
                    let full = Rect::new(0, 0, w as i32, h as i32);
                    self.target = *target;
                    self.view = full;
                    self.port = full;
                }
            }
            Command::SetView { view, port } => {
                self.view = *view;
                self.port = *port;
            }
            Command::SetBlend(blend) => self.blend = *blend,
            _ => {}
        }
    }
}

/// One tick's drawing: GM's back buffer size, the drawing state the tick started in, and its commands.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    /// Size of GM's back buffer (the window's client area before any frontend scaling), in pixels. The
    /// back buffer keeps its contents between frames unless this size changes.
    pub width: u32,
    pub height: u32,
    pub start: DrawState,
    /// The surfaces that exist when the frame starts, by id.
    pub surfaces: Vec<SurfaceSize>,
    pub commands: Vec<Command>,
}

impl Default for Frame {
    fn default() -> Frame {
        Frame::new(0, 0)
    }
}

impl Frame {
    /// A frame that starts in GM's initial drawing state.
    pub fn new(width: u32, height: u32) -> Frame {
        Frame {
            width,
            height,
            start: DrawState::initial(width, height),
            surfaces: Vec::new(),
            commands: Vec::new(),
        }
    }

    pub fn push(&mut self, command: impl Into<Command>) {
        self.commands.push(command.into());
    }

    /// Feeds this frame to another sink.
    pub fn replay(&self, sink: &mut dyn DrawSink) {
        sink.begin(self.width, self.height, self.start, &self.surfaces);
        for command in &self.commands {
            sink.command(command.clone());
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// `surface_create`. Creating an id that already exists replaces it.
    CreateSurface {
        id: SurfaceId,
        width: u32,
        height: u32,
    },
    /// `surface_free`. If the surface is the current target, draws do nothing until the next
    /// [`Command::SetTarget`] (or until a surface is created under the same id, which then receives them).
    FreeSurface(SurfaceId),
    /// `surface_set_target` / `surface_reset_target`. Resets the view to the target's full size.
    SetTarget(Target),
    /// The back buffer takes a new size and loses its contents, as when a room change resizes GM's window
    /// mid-tick. The projection stays as it was (OpenGMK's `resize_framebuffer` leaves it alone). Later
    /// commands, and the next frame's [`Frame::width`]/[`Frame::height`], use the new size.
    ResizeScreen {
        width: u32,
        height: u32,
    },
    /// Sets the projection: `view` (room coordinates) maps onto `port` (target pixels). See the module
    /// docs for why `port` is ignored on surfaces. View angles are unused by the game and unsupported.
    SetView {
        view: Rect,
        port: Rect,
    },
    /// `draw_set_blend_mode` / `draw_set_blend_mode_ext`.
    SetBlend(BlendMode),
    /// `draw_clear` / `draw_clear_alpha`: fills the current port (the whole target on surfaces),
    /// replacing colour and alpha without blending.
    Clear {
        color: Color,
        alpha: f64,
    },
    Sprite(Sprite),
    Background(Background),
    Tile(Tile),
    /// `draw_rectangle`. Corners may come in any order; both are inclusive, like GM.
    Rectangle {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        color: Color,
        alpha: f64,
        outline: bool,
    },
    /// `draw_line`: one pixel wide.
    Line {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        color: Color,
        alpha: f64,
    },
    /// `draw_circle`, at GM's default precision of [`CIRCLE_PRECISION`] segments (the game never changes
    /// it).
    Circle {
        x: f64,
        y: f64,
        radius: f64,
        color: Color,
        alpha: f64,
        outline: bool,
    },
    /// `draw_text` with a sprite font, left/top aligned (the game never changes the alignment).
    Text(Text),
    /// `draw_surface`, `draw_surface_ext` and `draw_surface_stretched` (which is `_ext` with the scale
    /// set to `w / surface_width`). Drawn like a sprite with its origin at the top left.
    Surface(SurfaceDraw),
}

/// The circle precision GM 8 starts with (`draw_set_circle_precision` is never called by the game).
pub const CIRCLE_PRECISION: u32 = 24;

/// `draw_sprite_ext` and the default instance draw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sprite {
    /// Sprite resource id.
    pub sprite: u32,
    /// `floor(image_index)`. The renderer wraps it into the sprite's frame count like GM
    /// (`rem_euclid`), so the emitter passes it unreduced.
    pub frame: i32,
    pub x: f64,
    pub y: f64,
    pub xscale: f64,
    pub yscale: f64,
    /// Degrees, counter-clockwise, around the origin.
    pub angle: f64,
    pub blend: Color,
    pub alpha: f64,
}

impl Sprite {
    /// A plain `draw_sprite`: unscaled, unrotated, white, opaque.
    pub fn new(sprite: u32, frame: i32, x: f64, y: f64) -> Sprite {
        Sprite {
            sprite,
            frame,
            x,
            y,
            xscale: 1.0,
            yscale: 1.0,
            angle: 0.0,
            blend: Color::WHITE,
            alpha: 1.0,
        }
    }
}

/// A room background or foreground layer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Background {
    /// Background resource id.
    pub background: u32,
    pub x: f64,
    pub y: f64,
    /// The layer repeats from the left edge of the room past the right edge of the current view, as GM
    /// does; `tile_v` likewise downwards.
    pub tile_h: bool,
    pub tile_v: bool,
    /// For a stretched layer, `room_width / background_width` (and likewise for height).
    pub xscale: f64,
    pub yscale: f64,
    pub blend: Color,
    pub alpha: f64,
}

/// A tile: part of a background, drawn without an origin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tile {
    /// Background resource id.
    pub background: u32,
    /// The part of the background, in its pixels.
    pub left: u32,
    pub top: u32,
    pub width: u32,
    pub height: u32,
    pub x: f64,
    pub y: f64,
    pub xscale: f64,
    pub yscale: f64,
    pub blend: Color,
    pub alpha: f64,
}

/// `draw_text` with a font made by `font_add_sprite` (the game has no font resources).
#[derive(Debug, Clone, PartialEq)]
pub struct Text {
    pub font: SpriteFont,
    pub x: f64,
    pub y: f64,
    /// GM 8 strings are bytes; `#` starts a new line and `\#` draws a literal `#` (a space if the font has
    /// no `#`).
    pub text: Vec<u8>,
    pub color: Color,
    pub alpha: f64,
}

/// A font from `font_add_sprite(sprite, first, false, sep)`: one frame per character starting at byte
/// `first`. Only monospaced fonts are supported because the game only makes those (proportional fonts
/// need per-frame pixel bounds, which a draw list doesn't carry).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpriteFont {
    pub sprite: u32,
    pub first: u8,
    pub separation: i32,
}

/// Where a character of a [`Text`] goes, relative to the text's position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Glyph {
    pub frame: u32,
    pub dx: i32,
    pub dy: i32,
}

impl SpriteFont {
    /// Lays out `text` for a font sprite of `frame_count` frames of `width` x `height` pixels, following
    /// GM 8: every character advances by `width + separation`; lines are `height` apart; a byte the font
    /// doesn't cover is drawn as a space, and if the font has no space either nothing is drawn but the
    /// cursor still advances by the first character's advance. In a monospaced font that is the same
    /// `width + separation` (OpenGMK: `create_chars_from_sprite` gives every character that offset, and
    /// `draw_string` advances by `font.first`'s offset for a missing space).
    pub fn layout(&self, text: &[u8], frame_count: u32, width: u32, height: u32) -> Vec<Glyph> {
        let first = u32::from(self.first);
        // GM clamps the last character to 255.
        let last = (first + frame_count).min(256);
        let frame_of = |c: u8| {
            let c = u32::from(c);
            (frame_count > 0 && c >= first && c < last).then(|| c - first)
        };
        let advance = width as i32 + self.separation;
        let mut glyphs = Vec::with_capacity(text.len());
        let (mut dx, mut dy) = (0, 0);
        let mut i = 0;
        while i < text.len() {
            let c = text[i];
            i += 1;
            let c = match c {
                b'#' | b'\n' | b'\r' => {
                    // CRLF is one line break.
                    if c == b'\r' && text.get(i) == Some(&b'\n') {
                        i += 1;
                    }
                    dx = 0;
                    dy += height as i32;
                    continue;
                }
                b'\\' if text.get(i) == Some(&b'#') => {
                    i += 1;
                    b'#'
                }
                c => c,
            };
            // Checked after unescaping, so a `\#` in a font without `#` is a space too (OpenGMK's
            // `LineIterator` pushes a space for any character the font lacks).
            if let Some(frame) = frame_of(c).or_else(|| frame_of(b' ')) {
                glyphs.push(Glyph { frame, dx, dy });
            }
            if frame_count > 0 {
                dx += advance;
            }
        }
        glyphs
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceDraw {
    pub surface: SurfaceId,
    pub x: f64,
    pub y: f64,
    pub xscale: f64,
    pub yscale: f64,
    pub blend: Color,
    pub alpha: f64,
}

/// The emitter's handle for a surface (GM's surface id).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SurfaceId(pub u32);

/// A surface that exists, with its size in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SurfaceSize {
    pub id: SurfaceId,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Target {
    /// GM's back buffer: what the window shows.
    Screen,
    Surface(SurfaceId),
}

/// An integer rectangle: a view in room coordinates or a port in target pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }
}

/// A GM colour: `0x00BBGGRR`, as `make_color_rgb` builds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Color(pub u32);

impl Color {
    pub const WHITE: Color = Color(0xFF_FF_FF);
    pub const BLACK: Color = Color(0);

    pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color(r as u32 | (g as u32) << 8 | (b as u32) << 16)
    }

    pub const fn r(self) -> u8 {
        self.0 as u8
    }

    pub const fn g(self) -> u8 {
        (self.0 >> 8) as u8
    }

    pub const fn b(self) -> u8 {
        (self.0 >> 16) as u8
    }
}

/// A Direct3D blend factor, numbered as GM's `bm_*` constants for `draw_set_blend_mode_ext`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BlendFactor {
    Zero = 1,
    One = 2,
    SrcColor = 3,
    InvSrcColor = 4,
    SrcAlpha = 5,
    InvSrcAlpha = 6,
    DestAlpha = 7,
    InvDestAlpha = 8,
    DestColor = 9,
    InvDestColor = 10,
    SrcAlphaSaturate = 11,
}

impl BlendFactor {
    /// Converts a GML `bm_*` value; anything unknown is `bm_zero`, as in GM.
    pub fn from_gml(value: i32) -> BlendFactor {
        match value {
            2 => BlendFactor::One,
            3 => BlendFactor::SrcColor,
            4 => BlendFactor::InvSrcColor,
            5 => BlendFactor::SrcAlpha,
            6 => BlendFactor::InvSrcAlpha,
            7 => BlendFactor::DestAlpha,
            8 => BlendFactor::InvDestAlpha,
            9 => BlendFactor::DestColor,
            10 => BlendFactor::InvDestColor,
            11 => BlendFactor::SrcAlphaSaturate,
            _ => BlendFactor::Zero,
        }
    }
}

/// `result = src * src_factor + dest * dest_factor`, for colour and alpha alike.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlendMode {
    pub src: BlendFactor,
    pub dest: BlendFactor,
}

impl BlendMode {
    pub const NORMAL: BlendMode = BlendMode::new(BlendFactor::SrcAlpha, BlendFactor::InvSrcAlpha);
    pub const ADD: BlendMode = BlendMode::new(BlendFactor::SrcAlpha, BlendFactor::One);
    pub const MAX: BlendMode = BlendMode::new(BlendFactor::SrcAlpha, BlendFactor::InvSrcColor);
    pub const SUBTRACT: BlendMode = BlendMode::new(BlendFactor::Zero, BlendFactor::InvSrcColor);

    pub const fn new(src: BlendFactor, dest: BlendFactor) -> BlendMode {
        BlendMode { src, dest }
    }

    /// `draw_set_blend_mode`: `bm_add` (1), `bm_max` (2), `bm_subtract` (3), and anything else is
    /// `bm_normal`. (OpenGMK's comments swap the two names, but its factors match these.)
    pub fn from_gml(mode: i32) -> BlendMode {
        match mode {
            1 => BlendMode::ADD,
            2 => BlendMode::MAX,
            3 => BlendMode::SUBTRACT,
            _ => BlendMode::NORMAL,
        }
    }

    /// `draw_set_blend_mode_ext`.
    pub fn from_gml_ext(src: i32, dest: i32) -> BlendMode {
        BlendMode::new(BlendFactor::from_gml(src), BlendFactor::from_gml(dest))
    }
}

impl Default for BlendMode {
    fn default() -> BlendMode {
        BlendMode::NORMAL
    }
}

macro_rules! command_from {
    ($($ty:ident => $variant:ident),*) => {
        $(impl From<$ty> for Command {
            fn from(value: $ty) -> Command {
                Command::$variant(value)
            }
        })*
    };
}

command_from!(
    Sprite => Sprite,
    Background => Background,
    Tile => Tile,
    Text => Text,
    SurfaceDraw => Surface,
    BlendMode => SetBlend,
    Target => SetTarget
);

#[cfg(test)]
mod tests {
    use super::*;

    const FONT: SpriteFont = SpriteFont {
        sprite: 0,
        first: b' ',
        separation: 0,
    };

    fn layout(text: &[u8]) -> Vec<(u32, i32, i32)> {
        // 64 frames: ' ' through '_', like the game's fonts.
        FONT.layout(text, 64, 8, 8)
            .into_iter()
            .map(|g| (g.frame, g.dx, g.dy))
            .collect()
    }

    #[test]
    fn text_is_monospaced() {
        assert_eq!(layout(b"AB"), [(33, 0, 0), (34, 8, 0)]);
    }

    #[test]
    fn hash_breaks_lines_unless_escaped() {
        assert_eq!(layout(b"A#B"), [(33, 0, 0), (34, 0, 8)]);
        assert_eq!(layout(b"\\#"), [(3, 0, 0)]);
        assert_eq!(layout(b"A\r\nB"), [(33, 0, 0), (34, 0, 8)]);
    }

    #[test]
    fn an_escaped_hash_the_font_lacks_is_a_space() {
        // ' ' through '"': no '#' (nor 'A'), so both draw the space frame.
        let glyphs: Vec<_> = FONT
            .layout(b"\\#A", 3, 8, 8)
            .into_iter()
            .map(|g| (g.frame, g.dx))
            .collect();
        assert_eq!(glyphs, [(0, 0), (0, 8)]);
    }

    #[test]
    fn missing_characters_become_spaces() {
        // 'a' is past the font's last character.
        assert_eq!(layout(b"a!"), [(0, 0, 0), (1, 8, 0)]);
    }

    #[test]
    fn missing_space_only_advances() {
        let font = SpriteFont {
            first: b'A',
            ..FONT
        };
        let glyphs: Vec<_> = font.layout(b"z A", 26, 8, 8);
        assert_eq!(
            glyphs,
            [Glyph {
                frame: 0,
                dx: 16,
                dy: 0
            }]
        );
    }

    #[test]
    fn separation_widens_every_advance_including_missing_spaces() {
        let font = SpriteFont {
            first: b'A',
            separation: 3,
            ..FONT
        };
        // 'z' and ' ' aren't in the font: each advances by width + separation without drawing.
        let glyphs: Vec<_> = font
            .layout(b"Az B", 26, 8, 8)
            .into_iter()
            .map(|g| (g.frame, g.dx))
            .collect();
        assert_eq!(glyphs, [(0, 0), (1, 33)]);
        // Negative separation overlaps characters.
        let tight = SpriteFont {
            separation: -2,
            ..FONT
        };
        let glyphs: Vec<_> = tight
            .layout(b"AB", 64, 8, 8)
            .into_iter()
            .map(|g| g.dx)
            .collect();
        assert_eq!(glyphs, [0, 6]);
    }

    #[test]
    fn draw_state_follows_targets_views_and_blends() {
        let mut state = DrawState::initial(640, 480);
        let surface = |id: SurfaceId| (id == SurfaceId(1)).then_some((320, 240));
        state.apply(
            &Command::SetTarget(Target::Surface(SurfaceId(2))),
            (640, 480),
            surface,
        );
        assert_eq!(
            state,
            DrawState::initial(640, 480),
            "missing surfaces aren't targets"
        );
        state.apply(
            &Command::SetTarget(Target::Surface(SurfaceId(1))),
            (640, 480),
            surface,
        );
        state.apply(&Command::SetBlend(BlendMode::ADD), (640, 480), surface);
        assert_eq!(state.target, Target::Surface(SurfaceId(1)));
        assert_eq!(state.view, Rect::new(0, 0, 320, 240));
        assert_eq!(state.blend, BlendMode::ADD);
        // Resetting the target keeps the blend mode, as in GM.
        state.apply(&Command::SetTarget(Target::Screen), (640, 480), surface);
        assert_eq!(state.blend, BlendMode::ADD);
        assert_eq!(state.port, Rect::new(0, 0, 640, 480));
    }

    #[test]
    fn gml_blend_values() {
        assert_eq!(BlendMode::from_gml(0), BlendMode::NORMAL);
        // bm_max is 2 and bm_subtract 3; subtract never adds the source.
        assert_eq!(BlendMode::from_gml(2).src, BlendFactor::SrcAlpha);
        assert_eq!(BlendMode::from_gml(3).src, BlendFactor::Zero);
        assert_eq!(
            BlendMode::from_gml_ext(9, 1),
            BlendMode::new(BlendFactor::DestColor, BlendFactor::Zero)
        );
        assert_eq!(BlendFactor::from_gml(99), BlendFactor::Zero);
        assert_eq!(Color::rgb(1, 2, 3), Color(0x030201));
    }
}
