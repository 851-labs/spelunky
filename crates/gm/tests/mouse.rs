//! The mouse stage (frame stage 6): per-instance button, enter and leave events for the instances under
//! the cursor, then the global button events and the wheel. OpenGMK `game/events.rs` (`run_mouse_events`).

mod common;

use common::*;
use gm::*;
use trace::replay::{Button, Event as InputEvent, MouseButton};

const LEFT: Button = Button::Mouse(MouseButton::Left);
const RIGHT: Button = Button::Mouse(MouseButton::Right);

/// Every mouse sub-event the stage runs, but 58 (global middle release).
const SUBS: [u8; 24] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 50, 51, 52, 53, 54, 55, 56, 57, 60, 61,
];

/// Logs `tag.sub`.
fn mouse(g: &mut World, cx: &mut Ctx) -> R<()> {
    let Event::Mouse(sub) = cx.current_event() else {
        unreachable!("mouse events only");
    };
    let s = format!("{}.{sub}", tag(g, cx.this));
    log(g, &s);
    Ok(())
}

/// Objects `a` and `b` with every mouse event in `subs`, and `c` without any, all with the 8x8 sprite.
fn program(subs: &[u8]) -> &'static Program {
    build(|b| {
        for name in ["a", "b", "c"] {
            b.push_object(ObjectInfo {
                sprite: 0,
                ..ObjectInfo::new(name)
            });
        }
        for o in [ObjectId(0), ObjectId(1)] {
            for &sub in subs {
                b.event(o, Event::Mouse(sub), mouse);
            }
        }
    })
}

fn assets() -> Assets {
    Assets::new(vec![Some(sprite("s", 1, 8))], Vec::new())
}

/// A 320x240 room, views off, with these instances numbered from 1 in placement order.
fn world_at(subs: &[u8], placed: &[(ObjectId, i32, i32)]) -> World {
    let mut g = start(
        program(subs),
        assets(),
        &[(0, room(320, 240, &numbered(placed)))],
    );
    take_log(&mut g);
    g
}

const A: ObjectId = ObjectId(0);
const B: ObjectId = ObjectId(1);

#[test]
fn button_events_run_by_holder_then_list_order_for_the_instances_under_the_cursor() {
    // b1 and a2 overlap at (4, 4); a3 is elsewhere, and a4 overlaps too. a's instances lead (lower id).
    let mut g = world_at(&[0, 4], &[(B, 0, 0), (A, 2, 2), (A, 100, 100), (A, 4, 4)]);
    g.step(&[
        InputEvent::MouseMove { x: 5, y: 5 },
        InputEvent::Press(LEFT),
    ])
    .unwrap();
    assert_eq!(take_log(&mut g), "a2.0 a4.0 b1.0 a2.4 a4.4 b1.4");
    // Held, no longer pressed.
    step(&mut g);
    assert_eq!(take_log(&mut g), "a2.0 a4.0 b1.0");
}

#[test]
fn the_point_test_is_inclusive_of_the_bbox_and_misses_outside() {
    let mut g = world_at(&[3], &[(A, 10, 10)]);
    for (x, y, hit) in [
        (10, 10, true),
        (17, 17, true),
        (18, 10, false),
        (9, 17, false),
    ] {
        g.step(&[InputEvent::MouseMove { x, y }]).unwrap();
        let want = if hit { "a1.3" } else { "" };
        assert_eq!(take_log(&mut g), want, "({x}, {y})");
    }
}

#[test]
fn enter_and_leave_compare_with_where_the_last_frame_left_the_cursor() {
    let mut g = world_at(&[10, 11], &[(A, 10, 10), (B, 14, 10)]);
    // From (0, 0) into a1 only.
    g.step(&[InputEvent::MouseMove { x: 12, y: 12 }]).unwrap();
    assert_eq!(take_log(&mut g), "a1.10");
    // Still: nothing.
    step(&mut g);
    assert_eq!(take_log(&mut g), "");
    // Into the overlap: b2 entered; then out of a1 and into b2's right part.
    g.step(&[InputEvent::MouseMove { x: 15, y: 12 }]).unwrap();
    assert_eq!(take_log(&mut g), "b2.10");
    g.step(&[InputEvent::MouseMove { x: 20, y: 12 }]).unwrap();
    assert_eq!(take_log(&mut g), "a1.11");
    // A move that leaves and comes back within one frame is no move at all.
    g.step(&[
        InputEvent::MouseMove { x: 100, y: 100 },
        InputEvent::MouseMove { x: 20, y: 12 },
    ])
    .unwrap();
    assert_eq!(take_log(&mut g), "");
    // Leaving everything; enter runs before leave, so a jump from b2 into a1 logs a1 first.
    g.step(&[InputEvent::MouseMove { x: 11, y: 11 }]).unwrap();
    assert_eq!(take_log(&mut g), "a1.10 b2.11");
}

#[test]
fn a_click_within_one_frame_is_pressed_and_released_but_never_held() {
    let mut g = world_at(&[0, 3, 4, 7, 50, 53, 12, 56, 13], &[(A, 0, 0)]);
    g.step(&[
        InputEvent::MouseMove { x: 1, y: 1 },
        InputEvent::Press(LEFT),
        InputEvent::Release(LEFT),
    ])
    .unwrap();
    // No button is down, so "no button" (3) runs too. Each global press and release is followed by GM 5's
    // any-button 12 and 13.
    assert_eq!(take_log(&mut g), "a1.3 a1.4 a1.7 a1.53 a1.12 a1.56 a1.13");
}

#[test]
fn global_events_run_wherever_the_cursor_is_and_12_runs_once_per_button() {
    let mut g = world_at(&SUBS, &[(A, 0, 0)]);
    g.step(&[
        InputEvent::MouseMove { x: 200, y: 200 },
        InputEvent::Press(LEFT),
        InputEvent::Press(RIGHT),
    ])
    .unwrap();
    // The cursor left a1 from (0, 0); then the globals, though nothing is under the cursor.
    assert_eq!(
        take_log(&mut g),
        "a1.11 a1.50 a1.51 a1.53 a1.12 a1.54 a1.12"
    );
}

#[test]
fn both_wheel_directions_can_fire_in_one_frame_and_last_one_frame() {
    let mut g = world_at(&[60, 61], &[(A, 0, 0), (B, 0, 0)]);
    g.step(&[InputEvent::WheelDown, InputEvent::WheelUp])
        .unwrap();
    assert_eq!(take_log(&mut g), "a1.60 b2.60 a1.61 b2.61");
    step(&mut g);
    assert_eq!(take_log(&mut g), "");
}

#[test]
fn deactivated_instances_get_no_mouse_events() {
    let mut g = world_at(&[3, 60], &[(A, 0, 0), (A, 0, 0)]);
    let h = g.find(FIRST_ID).unwrap();
    let cx = ctx(&g, h);
    g.instance_deactivate_object(&cx, FIRST_ID);
    g.step(&[InputEvent::MouseMove { x: 1, y: 1 }, InputEvent::WheelUp])
        .unwrap();
    assert_eq!(take_log(&mut g), "a2.3 a2.60");
}

#[test]
fn the_cursor_maps_through_the_view_into_the_room() {
    let view = assets::View {
        visible: true,
        view_x: 100,
        view_w: 320,
        view_h: 240,
        port_w: 640,
        port_h: 480,
        follow_object: -1,
        ..Default::default()
    };
    let r = assets::Room {
        views_enabled: true,
        views: vec![view],
        ..room(1000, 600, &numbered(&[(A, 102, 2), (A, 0, 0)]))
    };
    let mut g = start(program(&[3, 10, 11]), assets(), &[(0, r)]);
    take_log(&mut g);
    // Window (4, 4) is room (102, 2): over a1, not a2 (which the raw window point would hit). The cursor
    // started at window (0, 0), room (100, 0), outside a1, so a1 is entered.
    g.step(&[InputEvent::MouseMove { x: 4, y: 4 }]).unwrap();
    assert_eq!(take_log(&mut g), "a1.3 a1.10");
}
