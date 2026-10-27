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
    g.draw_rectangle(0.0, 0.0, 320.0, 240.0, false).unwrap();
    g.draw_set_color(0xFF_FF_FF);
    g.draw_circle(160.0, 120.0, 32.0, false).unwrap();
    g.surface_set_target(SCREEN);
    g.draw_set_blend_mode_ext(BM_DEST_COLOR, BM_ZERO);
    g.draw_surface(DARK_SURF, 0.0, 0.0).unwrap();
    g.draw_set_blend_mode_ext(BM_SRC_COLOR, BM_ONE);
    g.draw_set_color(0);
    g.draw_rectangle(0.0, 0.0, 320.0, 240.0, false).unwrap();
    g.draw_set_blend_mode(0);
    g.surface_reset_target();
    g.draw_set_blend_mode_ext(BM_ONE, BM_ZERO);
    g.draw_surface_stretched(SCREEN, 0.0, 0.0, 640.0, 480.0)
        .unwrap();
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
    g.draw_rectangle(0.0, 0.0, 8.0, 8.0, false).unwrap();
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
    // Shown twice: by `screen_refresh` after the blit, and at the end of the step.
    let presents: Vec<_> = (0..frame.commands.len())
        .filter(|&i| frame.commands[i] == Command::Present)
        .collect();
    assert_eq!(presents, [blit + 2, frame.commands.len() - 1]);
}

#[test]
fn screen_refresh_shows_a_tick_that_ends_with_a_room_change_or_fails() {
    fn leave(g: &mut World, _: &mut Ctx) -> R<()> {
        g.room_goto(0);
        Ok(())
    }
    fn fail(_: &mut World, _: &mut Ctx) -> R<()> {
        Err(Error::Unsupported("test"))
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
        let failing = b.push_object(ObjectInfo::new("failing"));
        b.event(failing, Event::BeginStep, refresh);
        b.event(failing, Event::Step, fail);
    });
    let mut g = world(p, &[ObjectId(0)]);
    let mut frame = draw::Frame::default();
    g.step_drawing(&[], &mut frame).unwrap();
    assert!(g.presents(), "refreshed in begin step");
    // Only the refresh shows; the new room's first draw comes after it.
    let presents = frame.commands.iter().filter(|&c| *c == Command::Present);
    assert_eq!(
        (frame.commands[0].clone(), presents.count()),
        (Command::Present, 1)
    );
    let mut g = world(p, &[ObjectId(2)]);
    assert!(g.step(&[]).is_err());
    assert!(g.presents(), "refreshed before failing");
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
        assert_eq!(drawn.surface_sizes(), headless.surface_sizes());
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
