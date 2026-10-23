//! Surfaces, blend modes and the screen built-ins (851-3160), on a synthetic `oScreen`: GM8's rules as
//! OpenGMK implements them (`gml/kernel.rs`, `render/opengl.rs`).

mod common;

use common::*;
use draw::{BlendFactor, BlendMode, Color, Command, Rect, SurfaceId, SurfaceSize, Target};
use gm::*;

/// `oScreen`'s surfaces, as its create event makes them.
const P_SURF: i32 = 0;
const SCREEN: i32 = 1;
const DARK_SURF: i32 = 2;

/// Global 1: whether the screen object pauses (draws the room into `pSurf` with `screen_redraw`) this step.
const PAUSE: GVar = GVar(1);

// bm_* factors.
const BM_ZERO: i32 = 1;
const BM_ONE: i32 = 2;
const BM_SRC_COLOR: i32 = 3;
const BM_DEST_COLOR: i32 = 9;

/// `oScreen`'s create event: its three surfaces, `screen` the target and cleared.
fn screen_create(g: &mut World, _: &mut Ctx) -> R<()> {
    for id in [P_SURF, SCREEN, DARK_SURF] {
        assert_eq!(g.surface_create(320, 240)?, id);
    }
    g.surface_set_target(SCREEN);
    g.draw_clear(0);
    Ok(())
}

/// `oScreen`'s begin step, cut down: the pause redraw, darkness multiplied in from `darkSurf`, the alpha
/// fix, and `screen` blitted 2x onto the back buffer with `bm_one, bm_zero`.
fn screen_begin_step(g: &mut World, _: &mut Ctx) -> R<()> {
    if g.gget(PAUSE)?.is_truthy() {
        g.surface_set_target(P_SURF);
        g.screen_redraw()?;
    }
    g.surface_reset_target();
    g.draw_clear(0);
    g.surface_set_target(DARK_SURF);
    g.draw_set_color(0);
    g.draw_rectangle(0.0, 0.0, 320.0, 240.0, false);
    g.draw_set_color(0xFF_FF_FF);
    g.draw_circle(160.0, 120.0, 32.0, false);
    g.surface_set_target(SCREEN);
    g.draw_set_blend_mode_ext(BM_DEST_COLOR, BM_ZERO);
    g.draw_surface(DARK_SURF, 0.0, 0.0);
    g.draw_set_blend_mode_ext(BM_SRC_COLOR, BM_ONE);
    g.draw_set_color(0);
    g.draw_rectangle(0.0, 0.0, 320.0, 240.0, false);
    g.draw_set_blend_mode(0);
    g.surface_reset_target();
    g.draw_set_blend_mode_ext(BM_ONE, BM_ZERO);
    g.draw_surface_stretched(SCREEN, 0.0, 0.0, 640.0, 480.0);
    g.draw_set_blend_mode(0);
    g.screen_refresh();
    Ok(())
}

/// `oScreen`'s end step: everything the draw stage draws goes into `screen`.
fn screen_end_step(g: &mut World, _: &mut Ctx) -> R<()> {
    g.surface_set_target(SCREEN);
    Ok(())
}

/// A room instance's draw event: logs and draws a rectangle.
fn thing_draw(g: &mut World, _: &mut Ctx) -> R<()> {
    log(g, "draw");
    g.draw_rectangle(0.0, 0.0, 8.0, 8.0, false);
    Ok(())
}

/// The synthetic `oScreen` and one drawn instance, started in a 320x240 room without views.
fn screen_world() -> World {
    let p = build(|b| {
        assert_eq!(b.global("pause"), PAUSE);
        let screen = b.push_object(ObjectInfo::new("oScreen"));
        b.event(screen, Event::Create, screen_create);
        b.event(screen, Event::BeginStep, screen_begin_step);
        b.event(screen, Event::EndStep, screen_end_step);
        let thing = b.push_object(ObjectInfo::new("thing"));
        b.event(thing, Event::Draw, thing_draw);
    });
    let mut g = world(p, &[ObjectId(0), ObjectId(1)]);
    g.gset(PAUSE, Value::Real(0.0)).unwrap();
    take_log(&mut g);
    g
}

fn surface_world() -> World {
    let p = build(|b| {
        b.push_object(ObjectInfo::new("a"));
    });
    world(p, &[])
}

fn sizes(g: &World) -> Vec<(u32, u32, u32)> {
    let sizes = g.surface_sizes();
    sizes.map(|s| (s.id.0, s.width, s.height)).collect()
}

/// The commands `f` records.
fn recorded(g: &mut World, f: impl FnOnce(&mut World)) -> Vec<Command> {
    let mut frame = draw::Frame::default();
    g.record_drawing(&mut frame, f);
    frame.commands
}

#[test]
fn surface_ids_are_the_lowest_free_slot() {
    let mut g = surface_world();
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
    assert_eq!(sizes(&g), [(0, 320, 240), (1, 2, 2), (2, 4, 4), (3, 3, 3)]);
    assert!(g.surface_exists(0) && !g.surface_exists(-1) && !g.surface_exists(4));
    assert_eq!(g.surface_size(2), Some((4, 4)));
    let create = |id, width, height| Command::CreateSurface {
        id: SurfaceId(id),
        width,
        height,
    };
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
    assert!(matches!(
        g.surface_create(0, 240),
        Err(Error::Unsupported(_))
    ));
}

#[test]
fn targets_switch_and_reset_the_projection() {
    let mut g = surface_world();
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
    let mut g = surface_world();
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
fn blend_modes_persist_across_targets_and_steps() {
    let mut g = surface_world();
    g.surface_create(8, 8).unwrap();
    g.surface_set_target(0);
    g.draw_set_blend_mode_ext(BM_DEST_COLOR, BM_ZERO);
    let multiply = BlendMode::new(BlendFactor::DestColor, BlendFactor::Zero);
    g.surface_reset_target();
    assert_eq!(g.draw_state().blend, multiply);
    step(&mut g);
    let mut frame = draw::Frame::default();
    g.step_drawing(&[], &mut frame).unwrap();
    assert_eq!(
        frame.start.blend, multiply,
        "a frame starts with the blend mode"
    );
    assert_eq!(g.draw_state().blend, multiply);
    // Unknown modes are bm_normal; the four named ones.
    for (mode, blend) in [
        (1, BlendMode::ADD),
        (2, BlendMode::MAX),
        (3, BlendMode::SUBTRACT),
        (9, BlendMode::NORMAL),
        (0, BlendMode::NORMAL),
    ] {
        g.draw_set_blend_mode(mode);
        assert_eq!(g.draw_state().blend, blend, "mode {mode}");
    }
}

#[test]
fn screen_redraw_runs_each_draw_event_once_more_into_the_target() {
    let mut g = screen_world();
    let mut frame = draw::Frame::default();
    g.step_drawing(&[], &mut frame).unwrap();
    assert_eq!(take_log(&mut g), "draw", "the draw stage alone");
    g.gset(PAUSE, Value::Real(1.0)).unwrap();
    g.step_drawing(&[], &mut frame).unwrap();
    assert_eq!(
        take_log(&mut g),
        "draw draw",
        "screen_redraw, then the draw stage"
    );
    // Each run records the rectangle once: into pSurf (screen_redraw), then into screen (the draw stage).
    let mut target = frame.start.target;
    let mut rects = Vec::new();
    for c in &frame.commands {
        match c {
            Command::SetTarget(t) => target = *t,
            Command::Rectangle { x2: 8.0, .. } => rects.push(target),
            _ => {}
        }
    }
    let surface = |id| Target::Surface(SurfaceId(id));
    assert_eq!(rects, [surface(P_SURF as u32), surface(SCREEN as u32)]);
    // The redraw leaves the projection on the whole window (ignored on pSurf), and pSurf the target.
    let start = frame
        .commands
        .iter()
        .position(|c| *c == Command::SetTarget(surface(P_SURF as u32)))
        .unwrap();
    assert_eq!(
        frame.commands[start + 1..start + 3],
        [
            Command::SetView {
                view: Rect::new(0, 0, 320, 240),
                port: Rect::new(0, 0, 320, 240)
            },
            Command::Rectangle {
                x1: 0.0,
                y1: 0.0,
                x2: 8.0,
                y2: 8.0,
                color: Color::BLACK,
                alpha: 1.0,
                outline: false
            },
        ]
    );
}

#[test]
fn the_screen_object_composes_a_frame_and_leaves_its_surface_the_target() {
    let mut g = screen_world();
    let mut frame = draw::Frame::default();
    g.step_drawing(&[], &mut frame).unwrap();
    let surfaces: Vec<_> = [P_SURF, SCREEN, DARK_SURF]
        .map(|id| SurfaceSize {
            id: SurfaceId(id as u32),
            width: 320,
            height: 240,
        })
        .into();
    assert_eq!(frame.surfaces, surfaces);
    assert_eq!(
        frame.start.target,
        Target::Surface(SurfaceId(SCREEN as u32))
    );
    assert_eq!(
        g.draw_state().target,
        Target::Surface(SurfaceId(SCREEN as u32))
    );
    assert_eq!(g.draw_state().blend, BlendMode::NORMAL);
    assert!(g.presents());
    // The blit: `bm_one, bm_zero`, 2x, onto the back buffer.
    let blit = frame
        .commands
        .iter()
        .position(|c| matches!(c, Command::Surface(s) if s.xscale == 2.0))
        .unwrap();
    assert_eq!(
        frame.commands[blit - 2..blit],
        [
            Command::SetTarget(Target::Screen),
            Command::SetBlend(BlendMode::new(BlendFactor::One, BlendFactor::Zero)),
        ]
    );
}

#[test]
fn screen_refresh_shows_a_tick_that_ends_with_a_room_change() {
    fn leave(g: &mut World, _: &mut Ctx) -> R<()> {
        g.room_goto(0);
        Ok(())
    }
    fn refresh(g: &mut World, _: &mut Ctx) -> R<()> {
        g.screen_refresh();
        Ok(())
    }
    let p = build(|b| {
        let a = b.push_object(ObjectInfo::new("a"));
        b.event(a, Event::BeginStep, refresh);
        b.event(a, Event::Step, leave);
        let quiet = b.push_object(ObjectInfo::new("quiet"));
        b.event(quiet, Event::Step, leave);
    });
    let mut g = world(p, &[ObjectId(0)]);
    step(&mut g);
    assert!(g.presents(), "refreshed in begin step");
    let mut g = world(p, &[ObjectId(1)]);
    step(&mut g);
    assert!(
        !g.presents(),
        "no refresh, and the frame ends with a room change"
    );
}

#[test]
fn headless_runs_track_the_same_surfaces_and_state() {
    let mut drawn = screen_world();
    let mut headless = drawn.clone();
    let mut frame = draw::Frame::default();
    for pause in [0.0, 1.0, 0.0] {
        for g in [&mut drawn, &mut headless] {
            g.gset(PAUSE, Value::Real(pause)).unwrap();
            g.action_fullscreen(0);
        }
        drawn.step_drawing(&[], &mut frame).unwrap();
        headless.step(&[]).unwrap();
        assert!(!frame.commands.is_empty());
        assert!(drawn.surface_sizes().eq(headless.surface_sizes()));
        assert_eq!(drawn.draw_state(), headless.draw_state());
        assert_eq!(drawn.presents(), headless.presents());
        assert_eq!(drawn.fullscreen(), headless.fullscreen());
        assert_eq!(take_log(&mut drawn), take_log(&mut headless));
        assert_eq!(drawn.trace_frame(), headless.trace_frame());
    }
}

#[test]
fn fullscreen_follows_action_fullscreen() {
    let mut g = surface_world();
    assert!(!g.fullscreen());
    g.action_fullscreen(2);
    assert!(g.fullscreen());
    g.action_fullscreen(1);
    assert!(!g.fullscreen());
    g.action_fullscreen(0);
    assert!(g.fullscreen(), "anything else switches");
    g.action_fullscreen(5);
    assert!(!g.fullscreen());
}
