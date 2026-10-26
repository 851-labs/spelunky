//! The frame loop, events, `with`/`other` and deactivation, on small synthetic programs.

mod common;

use common::*;
use gm::input::KeyState;
use gm::*;
use trace::replay::{Button, MouseButton};

const ALARM: Var = Builtin::Alarm.var();
const ID: Var = Builtin::Id.var();

fn alarm(g: &World, h: Inst, n: u32) -> i32 {
    g.get_a(h, ALARM, 0, n).unwrap().to_int().unwrap()
}

fn set_alarm(g: &mut World, h: Inst, n: u32, v: f64) {
    g.set_a(h, ALARM, 0, n, Value::Real(v)).unwrap();
}

fn handle(g: &World, n: i32) -> Inst {
    g.find(FIRST_ID + n - 1).unwrap()
}

#[test]
fn stages_run_in_gm8_order() {
    fn create(g: &mut World, cx: &mut Ctx) -> R<()> {
        note(g, cx)?;
        g.set_a(cx.this, ALARM, 0, 0, Value::Real(1.0))
    }
    fn collisions(g: &mut World) -> R<()> {
        log(g, "collisions");
        Ok(())
    }
    let p = build(|b| {
        let a = b.push_object(ObjectInfo::new("a"));
        b.event(a, Event::Create, create)
            .collision_stage(collisions);
        for e in [
            Event::BeginStep,
            Event::Alarm(0),
            Event::KeyPress(90),
            Event::Step,
            Event::OUTSIDE_ROOM,
            Event::EndStep,
            Event::Draw,
            Event::ANIMATION_END,
        ] {
            b.event(a, e, note);
        }
    });
    let mut g = world(p, &[ObjectId(0)]);
    // The room load draws frame 0.
    assert_eq!(take_log(&mut g), "a1.create a1.draw");
    g.instance_mut(handle(&g, 1)).x = -50.0;
    g.step(&[InputEvent::Press(Button::Key(90))]).unwrap();
    // No sprite: animation end fires every frame (the frame count is 0).
    assert_eq!(
        take_log(&mut g),
        "a1.begin a1.alarm0 a1.press90 a1.step a1.outside collisions a1.end a1.draw a1.anim"
    );
    step(&mut g);
    assert_eq!(
        take_log(&mut g),
        "a1.begin a1.step a1.outside collisions a1.end a1.draw a1.anim"
    );
}

#[test]
fn alarms_fire_by_alarm_then_object_then_instance() {
    let p = build(|b| {
        let a = b.push_object(ObjectInfo::new("a"));
        let bb = b.push_object(ObjectInfo::new("b"));
        b.push_object(ObjectInfo::new("c"));
        for o in [a, bb] {
            b.event(o, Event::Alarm(0), note);
            b.event(o, Event::Alarm(1), note);
        }
    });
    let (a, b, c) = (ObjectId(0), ObjectId(1), ObjectId(2));
    let mut g = world(p, &[b, a, b, c]);
    let [b1, a2, b3, c4] = [1, 2, 3, 4].map(|n| handle(&g, n));
    for h in [b1, a2, b3] {
        set_alarm(&mut g, h, 0, 1.0);
        set_alarm(&mut g, h, 1, 1.0);
    }
    // 0 never fires: it's counted down to -1 first.
    set_alarm(&mut g, a2, 1, 0.0);
    set_alarm(&mut g, c4, 0, 1.0);
    step(&mut g);
    assert_eq!(
        take_log(&mut g),
        "a2.alarm0 b1.alarm0 b3.alarm0 b1.alarm1 b3.alarm1"
    );
    assert_eq!(alarm(&g, a2, 1), -1);
    assert_eq!(alarm(&g, b1, 0), 0);
    // Only holders of the alarm event count down, so c's alarm is stuck.
    assert_eq!(alarm(&g, c4, 0), 1);
    step(&mut g);
    assert_eq!(take_log(&mut g), "");
    assert_eq!(alarm(&g, b1, 0), -1);

    // Writes round half to even.
    set_alarm(&mut g, b1, 0, 2.5);
    set_alarm(&mut g, b1, 1, 3.5);
    assert_eq!((alarm(&g, b1, 0), alarm(&g, b1, 1)), (2, 4));
    assert_eq!(alarm(&g, b1, 11), -1);
}

#[test]
fn new_instances_step_this_frame_only_if_their_object_comes_later() {
    fn spawn(g: &mut World, cx: &mut Ctx) -> R<()> {
        note(g, cx)?;
        if g.frames() == 0 {
            g.instance_create(10.0, 10.0, 0)?;
            g.instance_create(10.0, 10.0, 2)?;
        }
        Ok(())
    }
    let p = build(|b| {
        let low = b.push_object(ObjectInfo::new("low"));
        let spawner = b.push_object(ObjectInfo::new("s"));
        let high = b.push_object(ObjectInfo::new("high"));
        b.event(spawner, Event::Step, spawn);
        for o in [low, high] {
            b.event(o, Event::Create, note).event(o, Event::Step, note);
        }
    });
    let mut g = world(p, &[ObjectId(1)]);
    step(&mut g);
    // Create events run inside instance_create; `high` is a later holder, so it already steps.
    assert_eq!(
        take_log(&mut g),
        "s1.step low101.create high102.create high102.step"
    );
    step(&mut g);
    assert_eq!(take_log(&mut g), "low101.step s1.step high102.step");
}

#[test]
fn an_object_loop_can_reach_an_instance_created_during_it() {
    // x1 destroys x2 and creates x3. The loop over x's instances started with a count of 2, but its second
    // step scans past the deleted x2 and finds x3. GM8 behaviour per OpenGMK (`nb_il_iter`); verify
    // against the oracle.
    fn step_x(g: &mut World, cx: &mut Ctx) -> R<()> {
        note(g, cx)?;
        if g.instance(cx.this).id == FIRST_ID && g.frames() == 0 {
            let x2 = g.find(FIRST_ID + 1).unwrap();
            g.destroy(x2)?;
            g.instance_create(10.0, 10.0, 0)?;
        }
        Ok(())
    }
    let p = build(|b| {
        let x = b.push_object(ObjectInfo::new("x"));
        b.event(x, Event::Step, step_x);
    });
    let mut g = world(p, &[ObjectId(0), ObjectId(0)]);
    step(&mut g);
    assert_eq!(take_log(&mut g), "x1.step x101.step");
}

#[test]
fn animation_end_comes_after_draw() {
    fn draw(g: &mut World, cx: &mut Ctx) -> R<()> {
        let s = format!("draw{}", g.instance(cx.this).image_index);
        log(g, &s);
        Ok(())
    }
    fn anim_end(g: &mut World, cx: &mut Ctx) -> R<()> {
        note(g, cx)?;
        g.instance_destroy(cx)
    }
    let prog = build(|b| {
        let a = b.push_object(ObjectInfo {
            sprite: 0,
            ..ObjectInfo::new("a")
        });
        b.event(a, Event::Step, note)
            .event(a, Event::Draw, draw)
            .event(a, Event::ANIMATION_END, anim_end);
    });
    let assets = Assets::new(vec![Some(sprite("s", 2, 16))], Vec::new());
    let mut g = world_with(prog, assets, &[ObjectId(0)]);
    assert_eq!(take_log(&mut g), "draw0");
    step(&mut g);
    assert_eq!(take_log(&mut g), "a1.step draw0");
    step(&mut g);
    // Drawn on its last frame, then destroyed by animation end: gone from the next frame on.
    assert_eq!(take_log(&mut g), "a1.step draw1 a1.anim");
    assert!(g.trace_frame().instances.is_empty());
    step(&mut g);
    assert_eq!(take_log(&mut g), "");
}

#[test]
fn default_drawing_is_recorded_for_visible_instances_without_draw_events() {
    let prog = build(|b| {
        let info = ObjectInfo {
            sprite: 0,
            ..ObjectInfo::new("a")
        };
        b.push_object(info.clone());
        b.push_object(ObjectInfo {
            visible: false,
            ..info
        });
    });
    let assets = Assets::new(vec![Some(sprite("s", 3, 8))], Vec::new());
    let mut g = world_with(prog, assets, &[ObjectId(0), ObjectId(1)]);
    step(&mut g);
    let mut frame = draw::Frame::default();
    g.step_drawing(&[], &mut frame).unwrap();
    // Drawn at image_index 1 (it advanced after the first frame's draw); the invisible one isn't drawn.
    // Views are off: the room is drawn 1:1, then the projection is reset to the window, and the step ends
    // by showing the back buffer.
    let want = draw::Sprite::new(0, 1, 10.0, 10.0);
    let all = draw::Rect::new(0, 0, 320, 240);
    let set_view = draw::Command::SetView {
        view: all,
        port: all,
    };
    assert_eq!(
        frame.commands,
        vec![
            set_view.clone(),
            draw::Command::Sprite(want),
            set_view,
            draw::Command::Present
        ]
    );
    assert_eq!((frame.width, frame.height), (320, 240));
    assert!(g.presents());
    // Each tick records afresh into the same buffer.
    g.step_drawing(&[], &mut frame).unwrap();
    assert_eq!(frame.commands.len(), 4);
    // Ticks without a frame record nothing.
    step(&mut g);
    assert_eq!(frame.commands.len(), 4);
}

#[test]
fn drawing_sorts_by_depth_and_rebuilds_change_object_order() {
    fn create(g: &mut World, cx: &mut Ctx) -> R<()> {
        let depth = if tag(g, cx.this) == "o2" { 10.0 } else { 0.0 };
        g.set(cx.this, Builtin::Depth.var(), Value::Real(depth))
    }
    fn report(g: &mut World, cx: &mut Ctx) -> R<()> {
        note(g, cx)?;
        let all = with_ids(g, cx, -3.0)?;
        log(g, &format!("all:{all:?}"));
        Ok(())
    }
    let prog = build(|b| {
        let o = b.push_object(ObjectInfo::new("o"));
        b.event(o, Event::Create, create)
            .event(o, Event::Step, report);
    });
    let mut g = world(prog, &[ObjectId(0), ObjectId(0)]);
    step(&mut g);
    // Frame 0's draw sorted `all` by depth, highest first; the per-object list wasn't rebuilt.
    assert_eq!(take_log(&mut g), "o1.step all:[2, 1] o2.step all:[2, 1]");
    // Any (de)activation rebuilds the per-object lists from the draw order.
    let cx = ctx(&g, handle(&g, 1));
    g.instance_deactivate_object(&cx, FIRST_ID);
    g.instance_activate_object(&cx, FIRST_ID);
    step(&mut g);
    assert_eq!(take_log(&mut g), "o2.step all:[2, 1] o1.step all:[2, 1]");
}

#[test]
fn a_room_change_stops_the_frame_after_the_stage() {
    fn go(g: &mut World, cx: &mut Ctx) -> R<()> {
        note(g, cx)?;
        // `room = 5` requests a room change like room_goto; the rest of the code still runs.
        g.bset(Builtin::Room.var(), 0, Value::Real(5.0))?;
        log(g, "after");
        Ok(())
    }
    let prog = build(|b| {
        let a = b.push_object(ObjectInfo::new("a"));
        let z = b.push_object(ObjectInfo::new("z"));
        b.event(a, Event::Step, go);
        for o in [a, z] {
            b.event(o, Event::EndStep, note).event(o, Event::Draw, note);
        }
        b.event(z, Event::Step, note);
    });
    let placed = [ObjectId(0), ObjectId(0), ObjectId(1)].map(|o| (o, 10, 10));
    let r = room(320, 240, &numbered(&placed));
    let mut g = start(prog, Assets::default(), &[(0, r.clone()), (5, r)]);
    // Frame 0 drew.
    assert_eq!(take_log(&mut g), "a1.draw a2.draw z3.draw");
    step(&mut g);
    // a2 and z3 don't step: no event runs while a change is pending. End step and draw are skipped. Then
    // the room (5, a copy of room 0) loads, which draws frame 0.
    assert_eq!(take_log(&mut g), "a1.step after a1.draw a2.draw z3.draw");
    assert_eq!(g.room().id, 5);
    assert_eq!(g.pending(), None);
    assert!(!g.presents());
    // A change requested between steps (as a create event during a room load can) is performed by the next
    // step, which runs no events first. Input still applies; the room load ends the pressed state.
    g.room_goto(0);
    g.step(&[InputEvent::Press(Button::Key(37))]).unwrap();
    assert_eq!(take_log(&mut g), "a1.draw a2.draw z3.draw");
    assert_eq!((g.room().id, g.frames()), (0, 2));
    assert!(!g.input().check(KeyState::Pressed, 37));
    assert!(g.input().check(KeyState::Held, 37));
    step(&mut g);
    assert_eq!(take_log(&mut g), "a1.step after a1.draw a2.draw z3.draw");
    assert!(!g.presents());
}

#[test]
fn deactivated_instances_are_skipped_everywhere() {
    const HP: Var = Var(FIRST_USER_VAR);
    let prog = build(|b| {
        let hp = b.var("hp");
        let o = b.push_object(ObjectInfo::new("o"));
        b.layout(o, &[hp]);
        b.event(o, Event::Step, note)
            .event(o, Event::Alarm(0), note);
        b.push_object(ObjectInfo::new("ctl"));
    });
    let mut g = world(prog, &[ObjectId(0), ObjectId(0), ObjectId(1)]);
    let [o1, o2, ctl] = [1, 2, 3].map(|n| handle(&g, n));
    for (h, hp) in [(o1, 1.0), (o2, 2.0)] {
        set_alarm(&mut g, h, 0, 2.0);
        g.set(h, HP, Value::Real(hp)).unwrap();
    }
    let mut cx = ctx(&g, ctl);
    g.instance_deactivate_object(&cx, FIRST_ID);
    step(&mut g);
    step(&mut g);
    assert_eq!(take_log(&mut g), "o2.step o2.alarm0 o2.step");
    assert_eq!(alarm(&g, o1, 0), 2, "deactivated alarms don't count down");
    assert_eq!(g.find(FIRST_ID), None);
    assert!(!g.instance_exists(&cx, FIRST_ID));
    assert_eq!(g.instance_number(&cx, 0), 1);
    assert_eq!(g.instance_number(&cx, -3), 2);
    assert_eq!(g.instance_find(0, 0), FIRST_ID + 1);
    assert_eq!(with_ids(&mut g, &mut cx, 0.0).unwrap(), vec![2]);
    assert_eq!(with_ids(&mut g, &mut cx, -3.0).unwrap(), vec![2, 3]);
    assert_eq!(
        with_ids(&mut g, &mut cx, f64::from(FIRST_ID)).unwrap(),
        Vec::<i32>::new()
    );
    assert_eq!(g.get_on(&cx, &Value::Real(0.0), HP), Ok(Value::Real(2.0)));
    assert_eq!(
        g.get_on(&cx, &Value::Real(f64::from(FIRST_ID)), HP),
        Err(Error::UnsetVar(HP))
    );
    let t = g.trace_frame();
    assert_eq!(t.instance_count, Some(2));
    let off: Vec<i32> = t.deactivated.unwrap().iter().map(|i| i.id).collect();
    assert_eq!(off, vec![FIRST_ID]);

    // Reactivated, the alarm resumes from where it froze.
    g.instance_activate_object(&cx, 0);
    step(&mut g);
    assert_eq!(take_log(&mut g), "o1.step o2.step");
    step(&mut g);
    assert_eq!(take_log(&mut g), "o1.alarm0 o1.step o2.step");

    // deactivate_all(notme) spares the caller.
    g.instance_deactivate_all(&cx, true);
    assert_eq!(g.active().collect::<Vec<_>>(), vec![ctl]);
    g.instance_activate_all();
    assert_eq!(g.active().count(), 3);
}

#[test]
fn region_deactivation_tests_boxes_or_positions() {
    let prog = build(|b| {
        b.push_object(ObjectInfo {
            sprite: 0,
            ..ObjectInfo::new("masked")
        });
        b.push_object(ObjectInfo::new("point"));
        b.push_object(ObjectInfo::new("ctl"));
    });
    let assets = Assets::new(vec![Some(sprite("s", 1, 16))], Vec::new());
    let mut g = world_with(prog, assets, &[ObjectId(0), ObjectId(1), ObjectId(2)]);
    let [masked, point, ctl] = [1, 2, 3].map(|n| handle(&g, n));
    for h in [masked, point] {
        let i = g.instance_mut(h);
        (i.x, i.y) = (100.0, 100.0);
    }
    assert_eq!(
        g.get(masked, Builtin::BboxRight.var()),
        Ok(Value::Real(115.0))
    );
    assert_eq!(
        g.get(point, Builtin::BboxRight.var()),
        Ok(Value::Real(f64::from(BBOX_DEFAULT)))
    );
    let cx = ctx(&g, ctl);
    // The box (100..115) reaches into the region; the position doesn't.
    g.instance_deactivate_region(&cx, 110.0, 110.0, 50.0, 50.0, true, false);
    assert_eq!(g.active().collect::<Vec<_>>(), vec![point, ctl]);
    // Outside means strictly past an edge: a region ending at 99 misses the box at 100...
    g.instance_activate_region(0.0, 0.0, 99.0, 99.0, true);
    assert_eq!(g.active().count(), 2);
    // ...one ending at 100 touches it.
    g.instance_activate_region(0.0, 0.0, 100.0, 100.0, true);
    assert_eq!(g.active().count(), 3);
    // `inside = false` deactivates what's outside, and `notme` spares the caller.
    g.instance_deactivate_region(&cx, 0.0, 0.0, 50.0, 50.0, false, true);
    assert_eq!(g.active().collect::<Vec<_>>(), vec![ctl]);
}

#[test]
fn clones_continue_identically() {
    const SUM: Var = Var(FIRST_USER_VAR);
    fn wander(g: &mut World, cx: &mut Ctx) -> R<()> {
        let dx = g.rng().random(4.0) - 2.0;
        let x = g.get(cx.this, Builtin::X.var())?.add(&Value::Real(dx))?;
        g.set(cx.this, Builtin::X.var(), x)?;
        let sum = g.get(cx.this, SUM).unwrap_or(Value::Real(0.0));
        g.set(
            cx.this,
            SUM,
            gm::lenient(Value::add, sum, &Value::Real(dx))?,
        )?;
        if g.rng().random(10.0) < 1.0 {
            g.instance_create(50.0, 50.0, 0)?;
        }
        if g.rng().random(12.0) < 1.0 {
            g.instance_destroy(cx)?;
        }
        Ok(())
    }
    let prog = build(|b| {
        b.var("sum");
        let r = b.push_object(ObjectInfo::new("r"));
        b.event(r, Event::Step, wander);
    });
    let mut g = world(prog, &[ObjectId(0); 5]);
    for _ in 0..20 {
        step(&mut g);
    }
    let mut copy = g.clone();
    for _ in 0..40 {
        step(&mut g);
        step(&mut copy);
        assert_eq!(g.trace_frame(), copy.trace_frame());
    }
    let t = g.trace_frame();
    assert_eq!(t.frame, 59);
    assert!(
        t.instances.iter().any(|i| i.id > LAST_ID),
        "something was created"
    );
    assert!(
        t.instances
            .iter()
            .any(|i| i.vars.as_ref().unwrap().contains_key("sum"))
    );
}

#[test]
fn motion_applies_friction_gravity_then_speed() {
    let prog = build(|b| {
        b.push_object(ObjectInfo::new("o"));
    });
    let mut g = world(prog, &[ObjectId(0)]);
    let h = handle(&g, 1);
    let v = |b: Builtin| b.var();
    g.set(h, v(Builtin::Hspeed), Value::Real(3.0)).unwrap();
    g.set(h, v(Builtin::Friction), Value::Real(1.0)).unwrap();
    g.set(h, v(Builtin::Gravity), Value::Real(0.5)).unwrap();
    step(&mut g);
    let i = g.instance(h);
    // Friction takes speed 3 to 2; gravity (down) adds 0.5 vspeed; then x and y move.
    assert_eq!((i.hspeed, i.vspeed), (2.0, 0.5));
    assert_eq!((i.x, i.y), (12.0, 10.5));
    assert_eq!((i.xprevious, i.yprevious), (10.0, 10.0));
}

#[test]
fn traces_hold_instances_variables_and_globals() {
    const HP: Var = Var(FIRST_USER_VAR);
    const GRID: Var = Var(FIRST_USER_VAR + 1);
    const STRAY: Var = Var(FIRST_USER_VAR + 2);
    const SCORE: GVar = GVar(1);
    fn create(g: &mut World, cx: &mut Ctx) -> R<()> {
        let id = g.get(cx.this, ID)?;
        g.set(cx.this, HP, id)?;
        g.set_a(cx.this, GRID, 2, 5, Value::from("a"))?;
        // Outside the layout: stored in the overflow list, dumped the same way.
        g.set(cx.this, STRAY, Value::Real(1.0))?;
        g.gset_a(SCORE, 0, 1, Value::Real(9.0))?;
        g.set_a(cx.this, ALARM, 0, 3, Value::Real(4.0))
    }
    fn die(g: &mut World, cx: &mut Ctx) -> R<()> {
        if g.instance(cx.this).id == FIRST_ID + 1 {
            g.instance_destroy(cx)?;
        }
        Ok(())
    }
    let prog = build(|b| {
        let (hp, grid) = (b.var("hp"), b.var("grid"));
        b.var("stray");
        b.global("score");
        let o = b.push_object(ObjectInfo::new("o"));
        b.layout(o, &[hp, grid]);
        b.event(o, Event::Create, create).event(o, Event::Step, die);
    });
    let mut g = world(prog, &[ObjectId(0), ObjectId(0)]);
    step(&mut g);
    let t = g.trace_frame();
    assert_eq!(t.frame, 0);
    assert_eq!((t.room, t.seed), (Some(0), Some(12345)));
    // The instance destroyed in its step was removed by the cleanup.
    assert_eq!(t.instance_count, Some(1));
    let i = &t.instances[0];
    assert_eq!((i.id, i.object.as_str()), (FIRST_ID, "o"));
    assert_eq!(i.x, Some(trace::Real(10.0)));
    // o has no alarm 3 event, so the alarm never counts down.
    assert_eq!(i.alarm(3), Some(4));
    assert_eq!(i.alarm(0), Some(-1));
    let vars = i.vars.as_ref().unwrap();
    assert_eq!(vars["hp"], trace::Value::from(f64::from(FIRST_ID)));
    assert_eq!(vars["stray"], trace::Value::from(1.0));
    let trace::Value::Array(grid) = &vars["grid"] else {
        panic!("grid is an array");
    };
    assert_eq!(grid.keys().copied().collect::<Vec<_>>(), vec![64005]);
    let globals = t.globals.unwrap();
    assert!(matches!(globals["score"], trace::Value::Array(_)));
    assert_eq!(t.deactivated, Some(Vec::new()));
}

#[test]
fn input_follows_opengmk() {
    let prog = build(|b| {
        let o = b.push_object(ObjectInfo::new("o"));
        b.event(o, Event::KeyPress(90), note);
    });
    let mut g = world(prog, &[ObjectId(0)]);
    let z = Button::Key(90);
    g.step(&[InputEvent::Press(z)]).unwrap();
    // Pressing a key that is already down reports it pressed again, like auto-repeat.
    g.step(&[InputEvent::Press(z)]).unwrap();
    step(&mut g);
    assert_eq!(take_log(&mut g), "o1.press90 o1.press90");

    // Releasing either mouse button clears mouse_button, even with the other still down.
    let (left, right) = (
        Button::Mouse(MouseButton::Left),
        Button::Mouse(MouseButton::Right),
    );
    g.step(&[InputEvent::Press(left), InputEvent::Press(right)])
        .unwrap();
    let mb = Builtin::MouseButton.var();
    assert_eq!(g.bget(mb, 0), Ok(Value::Real(2.0)));
    g.step(&[InputEvent::Release(right)]).unwrap();
    assert_eq!(g.bget(mb, 0), Ok(Value::Real(0.0)));

    // Assignments GM8 can't represent are ignored.
    let key = Builtin::KeyboardKey.var();
    g.bset(key, 0, Value::Real(300.0)).unwrap();
    assert_eq!(g.bget(key, 0), Ok(Value::Real(90.0)));
    g.bset(key, 0, Value::Real(65.0)).unwrap();
    assert_eq!(g.bget(key, 0), Ok(Value::Real(65.0)));
    let last = Builtin::MouseLastbutton.var();
    g.bset(last, 0, Value::Real(7.0)).unwrap();
    assert_eq!(g.bget(last, 0), Ok(Value::Real(2.0)));
    g.bset(last, 0, Value::Real(3.0)).unwrap();
    assert_eq!(g.bget(last, 0), Ok(Value::Real(3.0)));
}

#[test]
fn negative_friction_only_acts_on_moving_instances() {
    let prog = build(|b| {
        b.push_object(ObjectInfo::new("o"));
    });
    let mut g = world(prog, &[ObjectId(0), ObjectId(0)]);
    let (still, moving) = (handle(&g, 1), handle(&g, 2));
    for h in [still, moving] {
        g.set(h, Builtin::Friction.var(), Value::Real(-1.0))
            .unwrap();
    }
    g.set(moving, Builtin::Speed.var(), Value::Real(2.0))
        .unwrap();
    step(&mut g);
    assert_eq!(g.instance(still).speed, 0.0);
    assert_eq!(g.instance(moving).speed, 3.0);
}

#[test]
fn intersect_boundary_fires_outside_the_room_too() {
    let prog = build(|b| {
        let o = b.push_object(ObjectInfo {
            sprite: 0,
            ..ObjectInfo::new("o")
        });
        b.event(o, Event::INTERSECT_BOUNDARY, note);
    });
    let assets = Assets::new(vec![Some(sprite("s", 1, 16))], Vec::new());
    let mut g = world_with(prog, assets, &[ObjectId(0), ObjectId(0), ObjectId(0)]);
    // Inside, straddling the left edge, and entirely outside.
    for (n, x) in [(1, 100.0), (2, -8.0), (3, -100.0)] {
        let h = handle(&g, n);
        g.instance_mut(h).x = x;
    }
    step(&mut g);
    assert_eq!(take_log(&mut g), "o2.Other(1) o3.Other(1)");
}

/// What `trapper`'s step event does, from the global after the log (851-3290 tests).
const MODE: GVar = GVar(1);
const TRAP: f64 = 1.0;
const GOTO_TRAP: f64 = 2.0;
const DIV0: f64 = 3.0;

/// `floor(2^64)`: the x87's FISTP traps on Intel, unless `MODE` is 0.
fn trap_if_set(g: &mut World) -> R<f64> {
    let mode = g.gget(MODE).map_or(0.0, |v| v.to_real());
    if mode != 0.0 {
        math::floor(18_446_744_073_709_551_616.0)?;
    }
    Ok(mode)
}

fn trapper(g: &mut World, cx: &mut Ctx) -> R<()> {
    note(g, cx)?;
    let mode = g.gget(MODE).map_or(0.0, |v| v.to_real());
    g.gset(MODE, Value::Real(0.0))?;
    if mode == GOTO_TRAP {
        g.room_goto(5);
    }
    if mode == DIV0 {
        math::div(1.0, 0.0)?;
    }
    g.gset(MODE, Value::Real(mode))?;
    trap_if_set(g)?;
    Ok(())
}

#[test]
fn an_exception_after_the_start_drops_the_rest_of_the_frame() {
    let prog = build(|b| {
        assert_eq!(b.global("mode"), MODE);
        let a = b.push_object(ObjectInfo::new("a"));
        let z = b.push_object(ObjectInfo::new("z"));
        b.event(a, Event::Step, trapper)
            .event(a, Event::KeyPress(90), note);
        for e in [
            Event::Step,
            Event::EndStep,
            Event::Draw,
            Event::ANIMATION_END,
        ] {
            b.event(z, e, note);
        }
    });
    let placed = [ObjectId(0), ObjectId(1)].map(|o| (o, 10, 10));
    let r = room(320, 240, &numbered(&placed));
    let mut g = start(prog, Assets::default(), &[(0, r.clone()), (5, r)]);
    take_log(&mut g);
    for i in 1..=2 {
        g.set(handle(&g, i), Builtin::Hspeed.var(), Value::Real(1.0))
            .unwrap();
    }
    // GM8's main loop catches it with an empty handler: no error, and nothing after it runs that frame (z's step,
    // the motion, end step, draw, animation, the end of the frame's input), but the frame counts.
    g.gset(MODE, Value::Real(TRAP)).unwrap();
    g.step(&[InputEvent::Press(Button::Key(90))]).unwrap();
    assert_eq!(take_log(&mut g), "a1.press90 a1.step");
    let e = g.exception().expect("an exception");
    assert_eq!(e.error, Error::Value(ValueError::IntOverflow));
    assert!(e.error.is_exception());
    assert_eq!(g.frames(), 1);
    assert!(!g.presents());
    assert_eq!(g.instance(handle(&g, 2)).x, 10.0);
    // The next frame runs as usual, the key press still pressed (GM8 clears it at the frame's end).
    g.gset(MODE, Value::Real(0.0)).unwrap();
    step(&mut g);
    assert_eq!(g.exception(), None);
    assert_eq!(
        take_log(&mut g),
        "a1.press90 a1.step z2.step z2.end z2.draw z2.anim"
    );
    assert_eq!(g.instance(handle(&g, 2)).x, 11.0);
    // A room change requested before it stays pending: the next frame runs no events, then makes it.
    g.gset(MODE, Value::Real(GOTO_TRAP)).unwrap();
    step(&mut g);
    assert_eq!(take_log(&mut g), "a1.step");
    assert_eq!((g.room().id, g.pending()), (0, Some(SceneChange::Room(5))));
    g.gset(MODE, Value::Real(0.0)).unwrap();
    step(&mut g);
    assert_eq!(take_log(&mut g), "z2.draw");
    assert_eq!(g.room().id, 5);
    // GML's own errors still stop it, as GM8's error dialog does.
    g.gset(MODE, Value::Real(DIV0)).unwrap();
    let e = g.step(&[]).unwrap_err();
    assert_eq!(e.error, Error::Value(ValueError::DivisionByZero));
}

#[test]
fn exceptions_stop_the_game_while_it_starts_and_ends() {
    fn trap(g: &mut World, cx: &mut Ctx) -> R<()> {
        note(g, cx)?;
        trap_if_set(g)?;
        Ok(())
    }
    fn end(g: &mut World, _: &mut Ctx) -> R<()> {
        g.game_end();
        Ok(())
    }
    let prog = build(|b| {
        assert_eq!(b.global("mode"), MODE);
        let a = b.push_object(ObjectInfo::new("a"));
        b.event(a, Event::Create, trap)
            .event(a, Event::GAME_END, trap)
            .event(a, Event::Step, end);
    });
    let r = room(320, 240, &numbered(&[(ObjectId(0), 10, 10)]));
    // In the first room's create events: GM8's "Unexpected error occured when running the game."
    let mut g = World::new(
        prog,
        std::sync::Arc::new(Assets::default().with_rooms(vec![Some(r.clone())])),
        12345,
        LAST_ID,
        FONT_SLOTS,
    );
    g.gset(MODE, Value::Real(TRAP)).unwrap();
    let e = g.start_game().unwrap_err();
    assert_eq!(e.error, Error::Value(ValueError::IntOverflow));
    // In game_end's events, which GM8 runs after leaving its main loop.
    let mut g = start(prog, Assets::default(), &[(0, r)]);
    g.gset(MODE, Value::Real(TRAP)).unwrap();
    let e = g.step(&[]).unwrap_err();
    assert_eq!(e.error, Error::Value(ValueError::IntOverflow));
    assert_eq!(g.exception(), None);
}
