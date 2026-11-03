//! Each frame's draw calls in the canonical form the trace's `draws` hashes and `draw_calls` lists, which the
//! oracle's patch 0041 writes too (docs/formats.md, "Draw calls"; 851-3871). It is what GML asked to draw, not
//! how a renderer carries it out: one call per `draw_text` (OpenGMK draws per glyph), a sprite's frame wrapped
//! into its frame count (as both renderers do), a font by its id, colours cut to their 24 bits.
//!
//! A call is a name, a tag and typed fields. The hash is [`trace::canon::hash`] of every call in order, each as
//! its tag (a `u64`) and its fields ([`Encoder`]: an `i32`, a real bit for bit with every NaN the same, a
//! `bool`, or length-prefixed bytes). The list writes each call as its name and fields, separated by spaces:
//! reals as Rust's shortest round trip (`NaN`, `inf`), text as a JSON string of its bytes as Latin-1.

use crate::world::World;
use draw::Command;
use std::fmt::Write as _;
use trace::canon::Encoder;

/// A call's field.
#[derive(Clone, Copy)]
enum Arg<'a> {
    Int(i32),
    Real(f64),
    Bool(bool),
    Bytes(&'a [u8]),
}

use Arg::{Bool, Int, Real};

/// A colour, cut to the 24 bits a renderer reads.
fn color(c: draw::Color) -> Arg<'static> {
    Int((c.0 & 0xFF_FF_FF) as i32)
}

fn rect(r: draw::Rect) -> [Arg<'static>; 4] {
    [Int(r.x), Int(r.y), Int(r.w), Int(r.h)]
}

impl World {
    /// The hash of `frame`'s draw calls, as the trace's `draws` holds it. `buf` is reused.
    pub fn draws_hash(&self, frame: &draw::Frame, buf: &mut Vec<u8>) -> u64 {
        buf.clear();
        let e = &mut Encoder(buf);
        for c in &frame.commands {
            self.call(c, &mut |_, tag, args| {
                e.u64(tag);
                for a in args {
                    match *a {
                        Int(n) => e.i32(n),
                        Real(r) => e.real(r),
                        Bool(b) => e.bool(b),
                        Arg::Bytes(b) => e.bytes(b),
                    }
                }
            });
        }
        trace::canon::hash(buf)
    }

    /// `frame`'s draw calls, one line each, as the trace's `draw_calls` lists them.
    pub fn draw_calls(&self, frame: &draw::Frame) -> Vec<String> {
        let mut out = Vec::with_capacity(frame.commands.len());
        for c in &frame.commands {
            self.call(c, &mut |name, _, args| {
                let mut line = name.to_string();
                for a in args {
                    line.push(' ');
                    match *a {
                        Int(n) => write!(line, "{n}").unwrap(),
                        Real(r) => write!(line, "{r:?}").unwrap(),
                        Bool(b) => write!(line, "{b}").unwrap(),
                        Arg::Bytes(b) => quoted(&mut line, b),
                    }
                }
                out.push(line);
            });
        }
        out
    }

    /// Calls `f` with the canonical call `c` is: its name, tag and fields.
    fn call(&self, c: &Command, f: &mut dyn FnMut(&'static str, u64, &[Arg])) {
        match c {
            Command::Clear { color: c, alpha } => f("clear", 1, &[color(*c), Real(*alpha)]),
            Command::SetView { view, port } => {
                let [x, y, w, h] = rect(*view);
                let [px, py, pw, ph] = rect(*port);
                f("view", 2, &[x, y, w, h, px, py, pw, ph]);
            }
            Command::SetTarget(t) => {
                let id = match t {
                    draw::Target::Screen => -1,
                    draw::Target::Surface(s) => s.0 as i32,
                };
                f("target", 3, &[Int(id)]);
            }
            Command::SetBlend(b) => f("blend", 4, &[Int(b.src as i32), Int(b.dest as i32)]),
            Command::Present => f("present", 5, &[]),
            Command::CreateSurface { id, width, height } => f(
                "surface_create",
                6,
                &[Int(id.0 as i32), Int(*width as i32), Int(*height as i32)],
            ),
            Command::FreeSurface(id) => f("surface_free", 7, &[Int(id.0 as i32)]),
            Command::ResizeScreen { width, height } => {
                f("resize", 8, &[Int(*width as i32), Int(*height as i32)]);
            }
            Command::Sprite(s) => {
                // As both renderers wrap it: a sprite without frames is never recorded.
                let frames = self
                    .assets
                    .sprite(s.sprite as i32)
                    .map_or(1, |sp| sp.frames.len().max(1));
                let frame = s.frame.rem_euclid(frames as i32);
                f(
                    "sprite",
                    9,
                    &[
                        Int(s.sprite as i32),
                        Int(frame),
                        Real(s.x),
                        Real(s.y),
                        Real(s.xscale),
                        Real(s.yscale),
                        Real(s.angle),
                        color(s.blend),
                        Real(s.alpha),
                    ],
                );
            }
            Command::Background(b) => f(
                "background",
                10,
                &[
                    Int(b.background as i32),
                    Real(b.x),
                    Real(b.y),
                    Bool(b.tile_h),
                    Bool(b.tile_v),
                    Real(b.xscale),
                    Real(b.yscale),
                    color(b.blend),
                    Real(b.alpha),
                ],
            ),
            Command::Tile(t) => f(
                "tile",
                11,
                &[
                    Int(t.background as i32),
                    Int(t.left as i32),
                    Int(t.top as i32),
                    Int(t.width as i32),
                    Int(t.height as i32),
                    Real(t.x),
                    Real(t.y),
                    Real(t.xscale),
                    Real(t.yscale),
                    color(t.blend),
                    Real(t.alpha),
                ],
            ),
            Command::Rectangle {
                x1,
                y1,
                x2,
                y2,
                color: c,
                alpha,
                outline,
            } => f(
                "rectangle",
                12,
                &[
                    Real(*x1),
                    Real(*y1),
                    Real(*x2),
                    Real(*y2),
                    color(*c),
                    Real(*alpha),
                    Bool(*outline),
                ],
            ),
            Command::Line {
                x1,
                y1,
                x2,
                y2,
                color: c,
                alpha,
            } => f(
                "line",
                13,
                &[
                    Real(*x1),
                    Real(*y1),
                    Real(*x2),
                    Real(*y2),
                    color(*c),
                    Real(*alpha),
                ],
            ),
            Command::Circle {
                x,
                y,
                radius,
                color: c,
                alpha,
                outline,
            } => f(
                "circle",
                14,
                &[
                    Real(*x),
                    Real(*y),
                    Real(*radius),
                    color(*c),
                    Real(*alpha),
                    Bool(*outline),
                ],
            ),
            Command::Text(t) => f(
                "text",
                15,
                &[
                    Int(self.font_id(&t.font)),
                    Real(t.x),
                    Real(t.y),
                    Arg::Bytes(&t.text),
                    color(t.color),
                    Real(t.alpha),
                ],
            ),
            Command::Surface(s) => f(
                "surface",
                16,
                &[
                    Int(s.surface.0 as i32),
                    Real(s.x),
                    Real(s.y),
                    Real(s.xscale),
                    Real(s.yscale),
                    color(s.blend),
                    Real(s.alpha),
                ],
            ),
        }
    }
}

/// `b` as a JSON string of its bytes as Latin-1, escaping `"`, `\` and the control characters below 0x20 (as
/// `\u00XX`), as the oracle's dump writes strings.
fn quoted(out: &mut String, b: &[u8]) {
    out.push('"');
    for &c in b {
        match c {
            b'"' => out.push_str("\\\""),
            b'\\' => out.push_str("\\\\"),
            c if c < 0x20 => write!(out, "\\u{:04x}", c).unwrap(),
            c => out.push(char::from(c)),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Assets, ProgramBuilder};
    use draw::{BlendFactor, BlendMode, Color, Rect, SurfaceId, Target};
    use std::sync::Arc;

    #[test]
    fn every_command_is_a_call() {
        let program = Box::leak(Box::new(ProgramBuilder::new().build()));
        let frame = assets::AtlasRect {
            page: 0,
            x: 0,
            y: 0,
            width: 8,
            height: 8,
        };
        let sprite = assets::Sprite {
            name: "sFont".into(),
            width: 8,
            height: 8,
            origin_x: 0,
            origin_y: 0,
            frames: vec![frame; 3],
            shape: assets::MaskShape::Rectangle,
            bbox_mode: assets::BboxMode::Automatic,
            alpha_tolerance: 0,
            separate_masks: false,
            masks: Vec::new(),
        };
        let room = assets::Room {
            name: "rTest".into(),
            width: 320,
            height: 240,
            speed: 30,
            ..Default::default()
        };
        let assets = Assets::new(vec![Some(sprite)], vec![Some(room)]);
        let mut w = World::new(program, Arc::new(assets), 1, 100_000, 0);
        w.start_game().unwrap();
        let font = w.font_add_sprite(0, 32, false, 1).unwrap();
        let mut f = draw::Frame::new(320, 240);
        let s = |frame| draw::Sprite {
            angle: 90.0,
            blend: Color(0x12_34_56),
            alpha: 0.5,
            ..draw::Sprite::new(0, frame, 1.5, -0.0)
        };
        f.commands = vec![
            Command::Clear {
                color: Color(0xff_00_00),
                alpha: 1.0,
            },
            Command::SetView {
                view: Rect::new(16, 32, 320, 240),
                port: Rect::new(0, 0, 960, 720),
            },
            Command::SetTarget(Target::Surface(SurfaceId(2))),
            Command::SetTarget(Target::Screen),
            Command::SetBlend(BlendMode::new(BlendFactor::DestColor, BlendFactor::Zero)),
            Command::Present,
            Command::CreateSurface {
                id: SurfaceId(1),
                width: 320,
                height: 240,
            },
            Command::FreeSurface(SurfaceId(1)),
            Command::ResizeScreen {
                width: 960,
                height: 720,
            },
            // Frames wrapped into the sprite's 3, as the renderers do.
            Command::Sprite(s(4)),
            Command::Sprite(s(-1)),
            Command::Background(draw::Background {
                background: 3,
                x: 0.25,
                y: f64::NAN,
                tile_h: true,
                tile_v: false,
                xscale: 2.0,
                yscale: 1.0,
                blend: Color::WHITE,
                alpha: 1.0,
            }),
            Command::Tile(draw::Tile {
                background: 3,
                left: 16,
                top: 0,
                width: 16,
                height: 16,
                x: 64.0,
                y: 80.0,
                xscale: 1.0,
                yscale: 1.0,
                blend: Color::WHITE,
                alpha: 1.0,
            }),
            Command::Rectangle {
                x1: 0.0,
                y1: 1.0,
                x2: 2.0,
                y2: 3.0,
                color: Color::BLACK,
                alpha: 0.75,
                outline: false,
            },
            Command::Line {
                x1: 0.0,
                y1: 1.0,
                x2: 2.0,
                y2: f64::INFINITY,
                color: Color::BLACK,
                alpha: 1.0,
            },
            Command::Circle {
                x: 4.0,
                y: 5.0,
                radius: 6.5,
                color: Color(0xff),
                alpha: 1.0,
                outline: true,
            },
            Command::Text(draw::Text {
                font: w.font(font).unwrap(),
                x: 8.0,
                y: 24.0,
                text: b"$1,000 \"#\xe9".to_vec(),
                color: Color::WHITE,
                alpha: 1.0,
            }),
            Command::Surface(draw::SurfaceDraw {
                surface: SurfaceId(1),
                x: 0.0,
                y: 0.0,
                xscale: 3.0,
                yscale: 3.0,
                blend: Color::WHITE,
                alpha: 1.0,
            }),
        ];
        assert_eq!(
            w.draw_calls(&f),
            [
                "clear 16711680 1.0",
                "view 16 32 320 240 0 0 960 720",
                "target 2",
                "target -1",
                "blend 9 1",
                "present",
                "surface_create 1 320 240",
                "surface_free 1",
                "resize 960 720",
                "sprite 0 1 1.5 -0.0 1.0 1.0 90.0 1193046 0.5",
                "sprite 0 2 1.5 -0.0 1.0 1.0 90.0 1193046 0.5",
                "background 3 0.25 NaN true false 2.0 1.0 16777215 1.0",
                "tile 3 16 0 16 16 64.0 80.0 1.0 1.0 16777215 1.0",
                "rectangle 0.0 1.0 2.0 3.0 0 0.75 false",
                "line 0.0 1.0 2.0 inf 0 1.0",
                "circle 4.0 5.0 6.5 255 1.0 true",
                &format!("text {font} 8.0 24.0 \"$1,000 \\\"#\u{e9}\" 16777215 1.0"),
                "surface 1 0.0 0.0 3.0 3.0 16777215 1.0",
            ]
        );
        // The oracle's patch 0041 hashes the same bytes: a change here is one there, and every fingerprint
        // recorded again.
        assert_eq!(
            format!("{:016x}", w.draws_hash(&f, &mut Vec::new())),
            "2ace1c6c25d61cba"
        );
    }

    #[test]
    fn text_is_quoted_as_the_dump_quotes_strings() {
        let mut s = String::new();
        quoted(&mut s, b"a\"b\\c\n\x1f\x7f\xe9");
        assert_eq!(s, "\"a\\\"b\\\\c\\u000a\\u001f\u{7f}\u{e9}\"");
    }
}
