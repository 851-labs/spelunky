//! Frames the game has run but the renderer hasn't drawn: a headless capture's frames before the first it
//! shoots (`app --capture-from`). Rendering them is most of a long lead-in's time, and most of what they draw
//! never shows, as Spelunky clears the back buffer and redraws its `screen` surface every step. But the
//! renderer carries pixels from one frame to the next (the back buffer, the surfaces and the shown picture,
//! `draw`'s "State between frames"), and some of what an early frame draws does show much later: the pause
//! screen's `pSurf`, drawn once when the game pauses, and a dark level's `darkSurf`, which nothing clears.
//! So [`Unrendered`] keeps each frame for as long as any of those pixels may depend on it, drops it once
//! none can, and renders the ones it kept, in order, when asked ([`flush`](Unrendered::flush)). The pixels
//! then come out as if every frame had been rendered.
//!
//! The game runs every frame either way, draw events included: `gm` only records the draw calls, and the
//! game never reads a pixel back, so whether a frame is rendered changes nothing in it.
//!
//! What something the renderer carries holds depends on the frames that drew into it since a command last
//! replaced it whole, and on what any surface drawn into it depended on then. The commands that replace it
//! whole are a `draw_clear` filling the whole target, a surface's creation and, for the shown picture, a
//! present. Any other draw blends with what was there, or may leave some of it, so it counts as depending on
//! it, even where its pixels happen to cover everything: that only keeps a frame that could have gone. So
//! does a frame that resizes the back buffer or a surface: the renderer makes a blank one only if its own
//! is another size, and its own may be from before frames it never saw, which resized it back. Rendering
//! the frames kept, from wherever the renderer stands, gives the same pixels as rendering them all: a frame
//! dropped only drew what a kept one replaced later.

use draw::{Command, Frame, SurfaceId, Target};
use std::collections::{BTreeMap, BTreeSet};

/// The most frames kept: then they're rendered anyway. A dark level keeps all of its frames, as its
/// `darkSurf` is never replaced whole, so this bounds the memory (a level's frame is some 800 commands,
/// 55 KB) without costing ordinary levels anything: they keep two or three.
const MAX_KEPT: usize = 256;

/// What the renderer carries from one frame to the next.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Carried {
    /// GM's back buffer.
    Screen,
    /// The picture shown: the back buffer at the last present.
    Shown,
    Surface(SurfaceId),
}

/// Frames not rendered yet, and what the renderer's pixels depend on of them.
#[derive(Default)]
pub struct Unrendered {
    /// The frames kept, by number.
    kept: BTreeMap<u64, Frame>,
    /// The next frame's number.
    next: u64,
    /// The frames not rendered yet that each carried thing's pixels depend on. Nothing listed: the renderer
    /// already has them right.
    deps: BTreeMap<Carried, BTreeSet<u64>>,
    /// The back buffer's size after the last frame pushed (`None` before any).
    screen: Option<(u32, u32)>,
    /// The surfaces after the last frame pushed, with their sizes.
    surfaces: BTreeMap<SurfaceId, (u32, u32)>,
}

impl Unrendered {
    pub fn new() -> Unrendered {
        Unrendered::default()
    }

    /// Takes the next frame instead of rendering it. It's kept for as long as the renderer's pixels may
    /// depend on it; if too many are kept, they're all rendered now, with `render`.
    pub fn push(&mut self, frame: Frame, render: impl FnMut(&Frame)) {
        let n = self.next;
        self.next += 1;
        self.follow(n, &frame);
        self.kept.insert(n, frame);
        // The frames nothing depends on any more go.
        let needed: BTreeSet<u64> = self.deps.values().flatten().copied().collect();
        self.kept.retain(|n, _| needed.contains(n));
        if self.kept.len() >= MAX_KEPT {
            self.flush(render);
        }
    }

    /// Renders the frames kept, in order, with `render` (`Renderer::render`), which then has every pixel as
    /// if it had rendered every frame pushed.
    pub fn flush(&mut self, mut render: impl FnMut(&Frame)) {
        for frame in std::mem::take(&mut self.kept).values() {
            render(frame);
        }
        for deps in self.deps.values_mut() {
            deps.clear();
        }
    }

    /// Follows frame `n` through what `Renderer::render` does with it, noting what each carried thing
    /// depends on after it.
    fn follow(&mut self, n: u64, frame: &Frame) {
        // The renderer makes the back buffer the frame's size, blank if its own is another size.
        let size = (frame.width.max(1), frame.height.max(1));
        if self.screen != Some(size) {
            self.screen = Some(size);
            self.touch(Carried::Screen, n);
        }
        // It drops the surfaces the frame doesn't list, and makes the ones it lists that it lacks, or has at
        // another size, blank.
        self.surfaces
            .retain(|id, _| frame.surfaces.iter().any(|s| s.id == *id));
        self.deps.retain(|c, _| match c {
            Carried::Surface(id) => frame.surfaces.iter().any(|s| s.id == *id),
            _ => true,
        });
        for s in &frame.surfaces {
            let size = (s.width.max(1), s.height.max(1));
            if self.surfaces.get(&s.id) != Some(&size) {
                self.surfaces.insert(s.id, size);
                self.touch(Carried::Surface(s.id), n);
            }
        }
        let mut gm = frame.start;
        for command in &frame.commands {
            match command {
                Command::CreateSurface { id, width, height } => {
                    self.surfaces
                        .insert(*id, ((*width).max(1), (*height).max(1)));
                    self.replace(Carried::Surface(*id), n);
                }
                Command::FreeSurface(id) => {
                    self.surfaces.remove(id);
                    self.deps.remove(&Carried::Surface(*id));
                }
                Command::ResizeScreen { width, height } => {
                    let size = ((*width).max(1), (*height).max(1));
                    if self.screen != Some(size) {
                        self.screen = Some(size);
                        self.touch(Carried::Screen, n);
                    }
                }
                Command::Present => {
                    let mut deps = self.deps_of(Carried::Screen).clone();
                    deps.insert(n);
                    self.deps.insert(Carried::Shown, deps);
                }
                Command::SetTarget(_) | Command::SetView { .. } | Command::SetBlend(_) => {
                    let surfaces = &self.surfaces;
                    gm.apply(command, self.screen.expect("set above"), |id| {
                        surfaces.get(&id).copied()
                    });
                }
                command => {
                    // A draw into a surface that doesn't exist draws nothing.
                    let (target, (tw, th)) = match gm.target {
                        Target::Screen => (Carried::Screen, self.screen.expect("set above")),
                        Target::Surface(id) => match self.surfaces.get(&id) {
                            Some(&size) => (Carried::Surface(id), size),
                            None => continue,
                        },
                    };
                    let whole = match gm.target {
                        // Direct3D 8 ignores the port on a surface: a clear fills it.
                        Target::Surface(_) => true,
                        Target::Screen => {
                            let p = gm.port;
                            let (tw, th) = (tw as i32, th as i32);
                            p.x <= 0
                                && p.y <= 0
                                && p.x.saturating_add(p.w) >= tw
                                && p.y.saturating_add(p.h) >= th
                        }
                    };
                    if matches!(command, Command::Clear { .. }) && whole {
                        self.replace(target, n);
                        continue;
                    }
                    if let Command::Surface(s) = command {
                        let mut drawn = self.deps_of(Carried::Surface(s.surface)).clone();
                        self.deps.entry(target).or_default().append(&mut drawn);
                    }
                    self.touch(target, n);
                }
            }
        }
    }

    /// `what` was replaced whole in frame `n`: it depends on nothing earlier.
    fn replace(&mut self, what: Carried, n: u64) {
        self.deps.insert(what, BTreeSet::from([n]));
    }

    /// Frame `n` changed `what`, which still depends on what it held.
    fn touch(&mut self, what: Carried, n: u64) {
        self.deps.entry(what).or_default().insert(n);
    }

    fn deps_of(&self, what: Carried) -> &BTreeSet<u64> {
        static NONE: BTreeSet<u64> = BTreeSet::new();
        self.deps.get(&what).unwrap_or(&NONE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use draw::{Color, SurfaceDraw, SurfaceSize};

    const SURFACE: SurfaceId = SurfaceId(1);

    /// A frame with the surface, running `commands`, then marked `mark` by the colour of its last
    /// rectangle, on the back buffer, so it's told apart from the rest.
    fn frame(mark: u32, commands: Vec<Command>) -> Frame {
        let mut frame = Frame::new(8, 8);
        frame.surfaces = vec![SurfaceSize {
            id: SURFACE,
            width: 8,
            height: 8,
        }];
        frame.commands = commands;
        frame.commands.extend([
            Command::SetTarget(Target::Screen),
            Command::Rectangle {
                x1: 0.0,
                y1: 0.0,
                x2: 1.0,
                y2: 1.0,
                color: Color(mark),
                alpha: 1.0,
                outline: false,
            },
            Command::Present,
        ]);
        frame
    }

    fn clear(target: Target) -> Vec<Command> {
        vec![
            Command::SetTarget(target),
            Command::Clear {
                color: Color::BLACK,
                alpha: 1.0,
            },
        ]
    }

    fn draw_surface() -> Command {
        Command::Surface(SurfaceDraw {
            surface: SURFACE,
            x: 0.0,
            y: 0.0,
            xscale: 1.0,
            yscale: 1.0,
            blend: Color::WHITE,
            alpha: 1.0,
        })
    }

    /// The marks of the frames rendered when `frames` are pushed, then flushed.
    fn rendered(frames: &[Frame]) -> Vec<u32> {
        let mut marks = Vec::new();
        let mut mark = |frame: &Frame| {
            marks.extend(frame.commands.iter().rev().find_map(|c| match c {
                Command::Rectangle { color, .. } => Some(color.0),
                _ => None,
            }));
        };
        let mut unrendered = Unrendered::new();
        for frame in frames {
            unrendered.push(frame.clone(), &mut mark);
        }
        unrendered.flush(&mut mark);
        marks
    }

    #[test]
    fn a_frame_whose_drawing_is_cleared_away_is_dropped() {
        let frames: Vec<Frame> = (0..5).map(|i| frame(i, clear(Target::Screen))).collect();
        // The surface was made blank in the first frame, which only that frame's rendering does.
        assert_eq!(rendered(&frames), [0, 4]);
        let frames: Vec<Frame> = (0..5)
            .map(|i| {
                let mut commands = clear(Target::Surface(SURFACE));
                commands.extend(clear(Target::Screen));
                frame(i, commands)
            })
            .collect();
        assert_eq!(rendered(&frames), [4]);
    }

    #[test]
    fn a_draw_that_blends_keeps_what_it_drew_over() {
        // Nothing clears the back buffer: each frame draws over the last.
        let frames: Vec<Frame> = (0..4).map(|i| frame(i, vec![])).collect();
        assert_eq!(rendered(&frames), [0, 1, 2, 3]);
    }

    #[test]
    fn a_clear_inside_part_of_the_screen_keeps_the_rest() {
        let mut commands = clear(Target::Screen);
        commands.insert(
            1,
            Command::SetView {
                view: draw::Rect::new(0, 0, 4, 8),
                port: draw::Rect::new(0, 0, 4, 8),
            },
        );
        let frames: Vec<Frame> = (0..3).map(|i| frame(i, commands.clone())).collect();
        assert_eq!(rendered(&frames), [0, 1, 2]);
    }

    /// `oScreen`'s pause screen: drawn into once, then shown every frame over a cleared back buffer.
    #[test]
    fn a_surface_drawn_once_keeps_its_frame_until_it_is_cleared() {
        let mut frames = vec![frame(0, clear(Target::Screen))];
        let mut paused = clear(Target::Surface(SURFACE));
        paused.push(Command::Rectangle {
            x1: 2.0,
            y1: 2.0,
            x2: 3.0,
            y2: 3.0,
            color: Color::WHITE,
            alpha: 0.5,
            outline: false,
        });
        paused.extend(clear(Target::Screen));
        frames.push(frame(1, paused));
        for i in 2..6 {
            let mut shown = clear(Target::Screen);
            shown.push(draw_surface());
            frames.push(frame(i, shown));
        }
        assert_eq!(rendered(&frames), [1, 5]);
        // Cleared again: nothing before it counts.
        frames.push(frame(6, clear(Target::Surface(SURFACE))));
        let mut shown = clear(Target::Screen);
        shown.push(draw_surface());
        frames.push(frame(7, shown));
        assert_eq!(rendered(&frames), [6, 7]);
    }

    #[test]
    fn a_surface_made_again_or_freed_depends_on_nothing_before() {
        let surface = Target::Surface(SURFACE);
        let mut drawn = vec![Command::SetTarget(surface), draw_surface()];
        drawn.extend(clear(Target::Screen));
        let mut frames = vec![frame(0, drawn.clone()), frame(1, drawn.clone())];
        let mut made = vec![Command::CreateSurface {
            id: SURFACE,
            width: 8,
            height: 8,
        }];
        made.extend(clear(Target::Screen));
        frames.push(frame(2, made));
        assert_eq!(rendered(&frames), [2]);
        // Listed at another size, then at its own again: the renderer, which never saw the first, keeps
        // what it holds.
        frames.truncate(2);
        for (i, width) in [(2, 4), (3, 8)] {
            let mut resized = frame(i, clear(Target::Screen));
            resized.surfaces[0].width = width;
            frames.push(resized);
        }
        frames.push(frame(4, vec![Command::SetTarget(surface), draw_surface()]));
        assert_eq!(rendered(&frames), [0, 1, 2, 3, 4]);
        // Freed: what it held goes with it.
        frames.truncate(2);
        let mut freed = vec![Command::FreeSurface(SURFACE)];
        freed.extend(clear(Target::Screen));
        frames.push(frame(2, freed));
        assert_eq!(rendered(&frames), [2]);
    }

    #[test]
    fn the_shown_picture_keeps_its_frame_until_the_next_present() {
        let mut frames = vec![frame(0, clear(Target::Screen))];
        // A room change: the step ends without showing the back buffer, which the next frame resizes.
        let mut unshown = frame(1, clear(Target::Screen));
        unshown.commands.pop();
        unshown.commands.push(Command::ResizeScreen {
            width: 4,
            height: 4,
        });
        frames.push(unshown);
        assert_eq!(rendered(&frames), [0, 1]);
    }

    #[test]
    fn too_many_kept_are_rendered() {
        let frames: Vec<Frame> = (0..MAX_KEPT as u32 + 10)
            .map(|i| frame(i, vec![]))
            .collect();
        let mut marks = Vec::new();
        let mut unrendered = Unrendered::new();
        for frame in &frames {
            unrendered.push(frame.clone(), |_| marks.push(()));
        }
        assert_eq!(marks.len(), MAX_KEPT);
    }
}
