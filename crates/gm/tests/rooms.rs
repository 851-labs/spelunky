//! Rooms: loading and changing them (persistent instances, room and game events, frame 0), views, tiles
//! and backgrounds.

mod common;

use common::*;
use gm::*;
use trace::replay::Event as Input;

fn handle(g: &World, n: i32) -> Inst {
    g.find(FIRST_ID + n - 1).unwrap()
}

fn ids(g: &World) -> Vec<i32> {
    g.trace_frame()
        .instances
        .iter()
        .map(|i| i.id - FIRST_ID + 1)
        .collect()
}

/// Objects `a` (0) and persistent `p` (1), both logging create, game start, room start and room end.
fn room_program() -> &'static Program {
    build(|b| {
        let a = b.push_object(ObjectInfo::new("a"));
        let p = b.push_object(ObjectInfo {
            persistent: true,
            ..ObjectInfo::new("p")
        });
        for o in [a, p] {
            for e in [
                Event::Create,
                Event::GAME_START,
                Event::ROOM_START,
                Event::ROOM_END,
                Event::GAME_END,
            ] {
                b.event(o, e, note);
            }
        }
    })
}

#[test]
fn room_changes_run_gm8s_events_and_carry_persistent_instances() {
    let (a, p) = (ObjectId(0), ObjectId(1));
    let first = room(320, 240, &numbered(&[(a, 0, 0), (p, 0, 0)]));
    // The next room places an `a` and a `p` with the persistent instance's id.
    let next = room(
        640,
        480,
        &[(FIRST_ID + 2, a, 0, 0), (FIRST_ID + 1, p, 5, 5)],
    );
    let mut g = start(room_program(), Assets::default(), &[(0, first), (1, next)]);
    assert_eq!(
        take_log(&mut g),
        "a1.create p2.create a1.gamestart p2.gamestart a1.roomstart p2.roomstart"
    );
    g.instance_mut(handle(&g, 2)).x = 99.0;
    g.room_goto(1);
    step(&mut g);
    // Room end in the old room; the placed `p` is replaced by the persistent one without a create event;
    // no game start the second time.
    assert_eq!(
        take_log(&mut g),
        "a1.roomend p2.roomend a3.create a3.roomstart p2.roomstart"
    );
    assert_eq!((g.room().id, g.room().width), (1, 640));
    assert_eq!(ids(&g), [2, 3]);
    // Until the cleanup, the id finds the replaced instance first in draw order: nothing, as in GM8.
    assert_eq!(g.find(FIRST_ID + 1), None);
    step(&mut g);
    assert_eq!(g.with_state(State::Deleted).count(), 0);
    assert_eq!(g.instance(handle(&g, 2)).x, 99.0);
    // `room_restart` reloads the current room; the persistent instance stays.
    g.room_restart();
    step(&mut g);
    assert_eq!(
        take_log(&mut g),
        "a3.roomend p2.roomend a3.create a3.roomstart p2.roomstart"
    );
    // `room_goto_next` past the last room is an error.
    assert_eq!(g.room_goto_next(), Err(Error::EndOfRoomOrder));
    // A room that doesn't exist fails when the change happens.
    g.room_goto(7);
    assert_eq!(g.step(&[]).unwrap_err().error, Error::NoSuchRoom(7));
}

#[test]
fn persistent_instances_leave_in_slot_order() {
    let (a, p) = (ObjectId(0), ObjectId(1));
    let first = room(320, 240, &numbered(&[(a, 0, 0), (a, 0, 0)]));
    let mut g = start(
        room_program(),
        Assets::default(),
        &[(0, first), (1, room(320, 240, &[]))],
    );
    // p101 takes the slot after the two placed instances. Once a1 is gone, p102 reuses a1's slot, the
    // lowest free one, so it leaves the room first though it was created last.
    g.instance_create(0.0, 0.0, p.0 as i32).unwrap();
    let cx = ctx(&g, handle(&g, 1));
    g.instance_destroy(&cx).unwrap();
    step(&mut g);
    g.instance_create(0.0, 0.0, p.0 as i32).unwrap();
    take_log(&mut g);
    g.room_goto(1);
    step(&mut g);
    assert_eq!(
        take_log(&mut g),
        "a2.roomend p101.roomend p102.roomend p102.roomstart p101.roomstart"
    );
}

#[test]
fn game_end_runs_room_end_then_game_end_and_stops() {
    let first = room(320, 240, &numbered(&[(ObjectId(1), 0, 0)]));
    let mut g = start(room_program(), Assets::default(), &[(0, first)]);
    take_log(&mut g);
    g.game_end();
    step(&mut g);
    assert_eq!(take_log(&mut g), "p1.roomend p1.gameend");
    assert!(g.ended());
    step(&mut g);
    assert_eq!(take_log(&mut g), "");
    assert_eq!(g.frames(), 2);
}

#[test]
fn a_change_requested_while_loading_skips_frame_0_and_waits_for_the_next_step() {
    fn go(g: &mut World, _: &mut Ctx) -> R<()> {
        g.room_goto_next()
    }
    let prog = build(|b| {
        let a = b.push_object(ObjectInfo::new("a"));
        let z = b.push_object(ObjectInfo::new("z"));
        b.event(a, Event::Create, go)
            .event(a, Event::Draw, note)
            .event(z, Event::Draw, note);
    });
    let first = room(320, 240, &numbered(&[(ObjectId(0), 0, 0)]));
    let second = room(320, 240, &[(FIRST_ID + 1, ObjectId(1), 0, 0)]);
    let mut g = start(prog, Assets::default(), &[(0, first), (1, second)]);
    assert_eq!(take_log(&mut g), "");
    assert_eq!(g.pending(), Some(SceneChange::Room(1)));
    step(&mut g);
    assert_eq!(take_log(&mut g), "z2.draw");
    assert_eq!(g.room().id, 1);
}

#[test]
fn views_follow_by_gm8s_border_rule() {
    let view = View {
        visible: true,
        wview: 320,
        hview: 240,
        hborder: 64,
        vborder: 32,
        ..View::default()
    };
    let follow = |mut v: View, x: f64, y: f64, room: (i32, i32)| {
        v.follow(x, y, room.0, room.1);
        (v.xview, v.yview)
    };
    let big = (1000, 1000);
    // Inside the borders: no move. Past the right border: just enough to keep 64 px of border.
    assert_eq!(follow(view, 200.0, 100.0, big), (0, 0));
    assert_eq!(follow(view, 300.0, 230.0, big), (44, 22));
    // At most `speed` pixels a frame.
    let slow = View {
        hspeed: 5,
        vspeed: 0,
        ..view
    };
    assert_eq!(follow(slow, 300.0, 230.0, big), (5, 0));
    // Leftwards and upwards.
    let moved = View {
        xview: 100,
        yview: 100,
        ..view
    };
    assert_eq!(follow(moved, 120.0, 110.0, big), (56, 78));
    // A border of half the view or more centres it.
    let centred = View {
        hborder: 160,
        ..view
    };
    assert_eq!(follow(centred, 500.0, 100.0, big).0, 340);
    // The position rounds half to even.
    assert_eq!(follow(centred, 500.5, 100.0, big).0, 340);
    assert_eq!(follow(centred, 501.5, 100.0, big).0, 342);
    // Clamped to the room, the right edge last: a view wider than the room goes negative.
    assert_eq!(follow(view, 990.0, 990.0, big), (680, 760));
    assert_eq!(follow(view, 10.0, 10.0, (200, 100)), (-120, -140));
}

/// A room with views on and view 0 (320x240, port 640x480) following `object`.
fn view_room(object: i32, placed: &[(ObjectId, i32, i32)]) -> assets::Room {
    let view = assets::View {
        visible: true,
        view_w: 320,
        view_h: 240,
        port_w: 640,
        port_h: 480,
        hborder: 64,
        vborder: 32,
        hspeed: -1,
        vspeed: -1,
        follow_object: object,
        ..Default::default()
    };
    assets::Room {
        views_enabled: true,
        views: vec![view],
        ..room(1000, 600, &numbered(placed))
    }
}

#[test]
fn the_draw_stage_moves_views_and_traces_view_0() {
    let prog = build(|b| {
        b.push_object(ObjectInfo::new("a"));
        b.push_object(ObjectInfo::new("b"));
    });
    let placed = [(ObjectId(0), 0, 0), (ObjectId(1), 600, 400)];
    let mut g = start(prog, Assets::default(), &[(0, view_room(1, &placed))]);
    // Frame 0 already followed `b`; the window is the port's size.
    assert_eq!(
        (g.room().views[0].xview, g.room().views[0].yview),
        (344, 192)
    );
    assert_eq!(g.window_size(), (640, 480));
    let builtins = g.trace_frame().builtins.unwrap();
    let r = |n: f64| trace::Value::from(trace::Scalar::Real(trace::Real(n)));
    assert_eq!(builtins["view_xview[0]"], r(344.0));
    assert_eq!(builtins["view_wview[0]"], r(320.0));
    // Following an instance id; views move only in the draw stage.
    g.bset(Builtin::ViewObject.var(), 0, Value::from(FIRST_ID))
        .unwrap();
    assert_eq!(g.room().views[0].xview, 344);
    step(&mut g);
    assert_eq!((g.room().views[0].xview, g.room().views[0].yview), (0, 0));
    // The mouse maps from the port (2x) into the view, rounding half to even.
    g.step(&[Input::MouseMove { x: 101, y: 51 }]).unwrap();
    assert_eq!(g.mouse_in_room(), (50, 26));
    assert_eq!(g.bget(Builtin::MouseX.var(), 0), Ok(Value::from(50)));
}

#[test]
fn room_set_view_applies_from_the_next_load() {
    let prog = build(|b| {
        b.push_object(ObjectInfo::new("a"));
    });
    let r = view_room(-1, &[(ObjectId(0), 0, 0)]);
    let mut g = start(prog, Assets::default(), &[(0, r)]);
    let view = View {
        visible: true,
        wview: 160,
        hview: 120,
        wport: 480,
        hport: 360,
        ..View::default()
    };
    g.room_set_view(0, 0, view);
    g.room_set_view(0, 8, view);
    g.room_set_view(9, 0, view);
    assert_eq!(g.room().views[0].wview, 320);
    g.room_restart();
    step(&mut g);
    assert_eq!(g.room().views[0], view);
    assert_eq!(g.window_size(), (480, 360));
}

fn background(name: &str) -> assets::Background {
    assets::Background {
        name: name.into(),
        file: None,
        width: 64,
        height: 32,
        use_as_tileset: true,
        tile_width: 16,
        tile_height: 16,
        h_offset: 0,
        v_offset: 0,
        h_sep: 0,
        v_sep: 0,
    }
}

#[test]
fn tiles_and_instances_draw_merged_by_depth() {
    fn deep(g: &mut World, cx: &mut Ctx) -> R<()> {
        g.set(cx.this, Builtin::Depth.var(), Value::Real(5.0))
    }
    let prog = build(|b| {
        let a = b.push_object(ObjectInfo {
            sprite: 0,
            ..ObjectInfo::new("a")
        });
        b.event(a, Event::Create, deep);
    });
    let mut r = room(320, 240, &numbered(&[(ObjectId(0), 1, 1)]));
    r.tiles = vec![assets::Tile {
        id: 7,
        background: 0,
        width: 16,
        height: 16,
        depth: 5,
        ..Default::default()
    }];
    r.backgrounds = vec![
        assets::RoomBackground {
            visible: true,
            background: 0,
            stretch: true,
            ..Default::default()
        },
        assets::RoomBackground {
            visible: true,
            foreground: true,
            background: 0,
            hspeed: 2,
            ..Default::default()
        },
    ];
    r.draw_background_color = true;
    r.background_color = 0x123456;
    let assets = Assets::new(vec![Some(sprite("s", 1, 8))], Vec::new())
        .with_backgrounds(vec![Some(background("bg"))]);
    let mut g = start(prog, assets, &[(0, r)]);
    assert_eq!(g.room().backgrounds[0].xscale, 5.0);
    let t1 = g.tile_add(0, 16, 0, 16, 16, 2.0, 2.0, 10.0).unwrap();
    let t2 = g.tile_add(0, 32, 16, 16, 16, 3.0, 3.0, 5.0).unwrap();
    let t3 = g.tile_add(0, 48, 0, 16, 16, 4.0, 4.0, -1.0).unwrap();
    assert_eq!((t1, t2, t3), (10_000_001, 10_000_002, 10_000_003));
    assert_eq!(
        g.tile_add(3, 0, 0, 1, 1, 0.0, 0.0, 0.0),
        Err(Error::NoSuchBackground(3))
    );
    let mut frame = draw::Frame::default();
    g.step_drawing(&[], &mut frame).unwrap();
    let kinds: Vec<String> = frame
        .commands
        .iter()
        .map(|c| match c {
            draw::Command::Tile(t) => format!("tile{}", t.left),
            draw::Command::Sprite(_) => "a".into(),
            draw::Command::Background(b) if b.xscale == 5.0 => "back".into(),
            draw::Command::Background(_) => "fore".into(),
            draw::Command::Clear { color, .. } => format!("clear{:x}", color.0),
            draw::Command::SetView { .. } => "view".into(),
            c => format!("{c:?}"),
        })
        .collect();
    // Highest depth first; at equal depth the instance before the tiles, which keep their order (the
    // room's tile before the added one).
    assert_eq!(
        kinds,
        [
            "view",
            "clear123456",
            "back",
            "tile16",
            "a",
            "tile0",
            "tile32",
            "tile48",
            "fore",
            "view"
        ]
    );
    // The foreground layer scrolled after the step's draw (not after frame 0's).
    assert_eq!(g.room().backgrounds[1].x, 2.0);
    // GM 8.0's tile_layer_find never finds a tile, even at its exact position.
    assert_eq!(g.tile_layer_find(5.0, 3.0, 3.0), -1);
    g.tile_delete(t2).unwrap();
    assert_eq!(g.tile_delete(t2), Err(Error::NoSuchTile(t2)));
    assert_eq!(g.room().tiles.len(), 3);
}

#[test]
fn clones_step_identically() {
    fn walk(g: &mut World, cx: &mut Ctx) -> R<()> {
        let x = g.get(cx.this, Builtin::X.var())?.to_real();
        let r = g.rng().random(4.0);
        g.set(cx.this, Builtin::X.var(), Value::Real(x + r))?;
        if x > 40.0 {
            g.room_goto(1);
        }
        Ok(())
    }
    let prog = build(|b| {
        let a = b.push_object(ObjectInfo {
            persistent: true,
            ..ObjectInfo::new("a")
        });
        b.event(a, Event::Step, walk);
    });
    let placed = [(ObjectId(0), 0, 0)];
    let mut g = start(
        prog,
        Assets::default(),
        &[(0, view_room(0, &placed)), (1, view_room(0, &[]))],
    );
    let mut copy = g.clone();
    for _ in 0..40 {
        step(&mut g);
        step(&mut copy);
        assert_eq!(g.trace_frame(), copy.trace_frame());
    }
    assert_eq!(g.room().id, 1);
}
