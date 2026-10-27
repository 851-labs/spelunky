//! The copyright notice the Spelunky User License asks a modified version to show: drawn by the app over
//! the game's picture for the first few seconds, never into the game's back buffer or surfaces, so the
//! simulation (and what it draws) is the same with or without it.

use assets::Bundle;
use draw::{Color, Command, Rect, SpriteFont, Text};

/// The notice, one line each. The game's sprite fonts only have `' '` through `'_'`: upper case, no `©`.
pub const LINES: [&str; 2] = [
    "SPELUNKY (C) 2008, 2009 DEREK YU AND MOSSMOUTH, LLC.",
    "UNOFFICIAL MOD: A FAN-MADE PORT, NOT THE ORIGINAL GAME.",
];

/// The notice in one line, for the window title and the page.
pub const TITLE: &str =
    "Spelunky Classic (unofficial mod) - Spelunky (c) 2008, 2009 Derek Yu and Mossmouth, LLC";

/// How long the notice shows, in ticks (30 a second), and how many of those it spends fading out.
const SHOWN: u64 = 6 * 30;
const FADE: u64 = 30;

/// The notice, in the game's small font.
pub struct Notice {
    font: SpriteFont,
    /// The font sprite's frame size.
    char_size: (u32, u32),
}

impl Notice {
    /// `None` if the bundle lacks the game's small font (`sFontSmall`).
    pub fn new(bundle: &Bundle) -> Option<Notice> {
        let (font, char_size) = small_font(bundle)?;
        Some(Notice { font, char_size })
    }

    /// What to draw over the picture shown after tick `tick` (counting from 0), `size` the picture's size;
    /// `None` once the notice is gone.
    pub fn commands(&self, tick: u64, size: (u32, u32)) -> Option<Vec<Command>> {
        let fade = fade(tick, SHOWN)?;
        let longest = LINES.iter().map(|l| l.len() as u32).max().unwrap_or(0);
        // In whole GM pixels, as big as the game's own (the window is 320x240 scaled), on one line each
        // if they fit at all.
        let (w, _) = size;
        let scale = (w.max(1) / (longest * self.char_size.0 + 8).max(1)).clamp(1, game_scale(size));
        Some(self.draw(fade, size, scale))
    }

    /// The notice for a clip (`--notice-frames`, `scripts/wiki-clips`): shown for `shown` ticks instead
    /// of a session's few seconds, still fading out over its last second (all of it, if it's shorter),
    /// and in the game's own pixels, wrapped to fit, so that the picture can be scaled back down to them.
    pub fn clip_commands(&self, tick: u64, size: (u32, u32), shown: u64) -> Option<Vec<Command>> {
        Some(self.draw(fade(tick, shown)?, size, game_scale(size)))
    }

    /// The notice at `fade` (1 is opaque), in pixels `scale` times the picture's.
    fn draw(&self, fade: f64, size: (u32, u32), scale: u32) -> Vec<Command> {
        let (cw, ch) = self.char_size;
        let (w, h) = (size.0.max(1), size.1.max(1));
        let (vw, vh) = ((w / scale) as i32, (h / scale) as i32);
        let lines = wrap(&LINES, (w / scale).saturating_sub(8) / cw);
        let line_h = ch as i32 + 2;
        let band = lines.len() as i32 * line_h + 6;
        let mut out = vec![
            Command::SetView {
                view: Rect::new(0, 0, vw, vh),
                port: Rect::new(0, 0, vw * scale as i32, vh * scale as i32),
            },
            // To the edges, past them: a filled rectangle reaches one screen pixel past its far corner, not
            // one view pixel, so `vw - 1` would leave a strip uncovered when the view is scaled up.
            Command::Rectangle {
                x1: 0.0,
                y1: f64::from(vh - band),
                x2: f64::from(vw),
                y2: f64::from(vh),
                color: Color::BLACK,
                alpha: 0.7 * fade,
                outline: false,
            },
        ];
        for (i, line) in lines.iter().enumerate() {
            let x = (vw - (line.len() as u32 * cw) as i32) / 2;
            out.push(Command::Text(Text {
                font: self.font,
                x: f64::from(x.max(0)),
                y: f64::from(vh - band + 4 + i as i32 * line_h),
                text: line.as_bytes().to_vec(),
                color: Color::WHITE,
                alpha: fade,
            }));
        }
        out
    }
}

/// How opaque a notice shown for `shown` ticks is after tick `tick`, fading out over its last second;
/// `None` once it's gone.
fn fade(tick: u64, shown: u64) -> Option<f64> {
    let fade_ticks = FADE.min(shown);
    match shown.checked_sub(tick)? {
        0 => None,
        left if left < fade_ticks => Some(left as f64 / fade_ticks as f64),
        _ => Some(1.0),
    }
}

/// How many of a picture's pixels make a GM pixel: the window is 320x240 scaled.
fn game_scale((w, h): (u32, u32)) -> u32 {
    (w / 320).min(h / 240).max(1)
}

/// The game's small font, which the app's overlays write in, and its characters' size: `sFontSmall` as
/// `scrInit` adds it (`font_add_sprite(sFontSmall, ord(' '), false, 0)`). Upper case only: it has `' '`
/// through `'_'`. `None` if the bundle lacks it.
pub fn small_font(bundle: &Bundle) -> Option<(SpriteFont, (u32, u32))> {
    let (index, sprite) = bundle
        .sprites
        .iter()
        .enumerate()
        .find_map(|(i, s)| Some((i, s.as_ref().filter(|s| s.name == "sFontSmall")?)))?;
    let font = SpriteFont {
        sprite: index as u32,
        first: b' ',
        separation: 0,
    };
    Some((font, (sprite.width.max(1), sprite.height.max(1))))
}

/// `lines` broken at spaces so that none is longer than `width` characters, where a word allows.
pub fn wrap(lines: &[&str], width: u32) -> Vec<String> {
    let mut out = Vec::new();
    for line in lines {
        let mut current = String::new();
        for word in line.split(' ') {
            if !current.is_empty() && current.len() + 1 + word.len() > width as usize {
                out.push(std::mem::take(&mut current));
            }
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
        out.push(current);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notice() -> Notice {
        Notice {
            font: SpriteFont {
                sprite: 0,
                first: b' ',
                separation: 0,
            },
            char_size: (8, 8),
        }
    }

    #[test]
    fn shows_then_fades_then_goes() {
        let alpha = |tick| {
            notice().commands(tick, (960, 720)).map(|c| match c.last() {
                Some(Command::Text(t)) => t.alpha,
                _ => panic!("ends with text"),
            })
        };
        assert_eq!(alpha(0), Some(1.0));
        assert_eq!(alpha(SHOWN - FADE - 1), Some(1.0));
        assert!(alpha(SHOWN - 1).is_some_and(|a| a > 0.0 && a < 0.1));
        assert_eq!(alpha(SHOWN), None);
        assert_eq!(alpha(u64::MAX), None);
    }

    #[test]
    fn a_clip_s_notice_lasts_as_long_as_asked_in_game_pixels() {
        let alpha = |tick, shown| {
            notice()
                .clip_commands(tick, (640, 480), shown)
                .map(|c| match c.last() {
                    Some(Command::Text(t)) => t.alpha,
                    _ => panic!("ends with text"),
                })
        };
        assert_eq!(alpha(0, 60), Some(1.0));
        assert_eq!(alpha(29, 60), Some(1.0));
        assert!(alpha(59, 60).is_some_and(|a| a > 0.0 && a < 0.1));
        assert_eq!(alpha(60, 60), None);
        // Shorter than the fade: it fades all along.
        assert_eq!(alpha(0, 10), Some(1.0));
        assert!(alpha(9, 10).is_some_and(|a| a > 0.0 && a < 0.2));
        assert_eq!(alpha(0, 0), None);
        // A 3x picture gets 3x text, wrapped to the game's 320 pixels, where a session's would be 2x.
        let c = notice().clip_commands(0, (960, 720), 60).unwrap();
        let Command::SetView { view, port } = c[0] else {
            panic!("starts with the view")
        };
        assert_eq!((view.w, port.w), (320, 960));
        assert!(c.iter().all(|c| match c {
            Command::Text(t) => t.x + 8.0 * t.text.len() as f64 <= 320.0,
            _ => true,
        }));
    }

    #[test]
    fn fits_the_picture_in_whole_pixels() {
        for (size, scale) in [
            ((960, 720), 2),
            ((1920, 1440), 4),
            ((320, 240), 1),
            ((1, 1), 1),
        ] {
            let c = notice().commands(0, size).unwrap();
            let Command::SetView { view, port } = c[0] else {
                panic!("starts with the view")
            };
            assert_eq!(port.w, view.w * scale, "{size:?}");
            for c in &c {
                if let Command::Text(t) = c
                    && view.w >= 8 * 20
                {
                    assert!(t.x >= 0.0 && t.x + 8.0 * t.text.len() as f64 <= f64::from(view.w));
                }
            }
        }
    }
}
