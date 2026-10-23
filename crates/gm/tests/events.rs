//! Running handlers: `with`/`other`, `event_inherited`, `instance_create` and `instance_destroy`.

mod common;

use common::*;
use gm::*;

const ID: Var = Builtin::Id.var();

fn handle(g: &World, n: i32) -> Inst {
    g.find(FIRST_ID + n - 1).unwrap()
}

/// Runs `e` for instance `n` (numbered from 1) with `other` = itself, as the frame loop does.
fn run(g: &mut World, n: i32, e: Event) {
    let h = handle(g, n);
    g.run_event(h, h, e).unwrap();
}

#[test]
fn with_targets_objects_with_descendants_all_ids_self_and_other() {
    fn ctl_step(g: &mut World, cx: &mut Ctx) -> R<()> {
        let me = cx.this;
        let mut out = Vec::new();
        for (name, target) in [
            ("p", 0.0),
            ("c", 1.0),
            ("all", -3.0),
            ("noone", -4.0),
            ("global", -5.0),
            ("id4", f64::from(FIRST_ID + 3)),
            ("self", -1.0),
            ("other", -2.0),
            ("missing", f64::from(FIRST_ID + 50)),
        ] {
            let ids = with_ids(g, cx, target)?;
            assert_eq!((cx.this, cx.other), (me, me));
            out.push(format!("{name}:{ids:?}"));
        }
        log(g, &out.join(" "));
        Ok(())
    }
    let prog = build(|b| {
        let p = b.push_object(ObjectInfo::new("p"));
        let c = b.push_object(child("c", p));
        b.push_object(child("gc", c));
        b.push_object(ObjectInfo::new("other"));
        let ctl = b.push_object(ObjectInfo::new("ctl"));
        b.event(ctl, Event::Step, ctl_step);
    });
    let [p, c, gc, other, ctl] = [0, 1, 2, 3, 4].map(ObjectId);
    let mut g = world(prog, &[c, p, other, gc, c, ctl]);
    run(&mut g, 6, Event::Step);
    // An object includes its descendants, in creation order; `all` is the draw order.
    assert_eq!(
        take_log(&mut g),
        "p:[1, 2, 4, 5] c:[1, 4, 5] all:[1, 2, 3, 4, 5, 6] noone:[] global:[] id4:[4] self:[6] \
         other:[6] missing:[]"
    );
}

#[test]
fn other_is_the_partner_in_collisions_and_the_outer_self_in_with() {
    fn hit(g: &mut World, cx: &mut Ctx) -> R<()> {
        let (me, them) = (tag(g, cx.this), tag(g, cx.other));
        log(g, &format!("{me}<-{them}"));
        let mut it = g.with_iter(cx, &Value::Real(-2.0))?;
        let outer = (cx.this, cx.other);
        while let Some(h) = it.next(g) {
            (cx.this, cx.other) = (h, outer.0);
            let s = format!("with:{}/{}", tag(g, cx.this), tag(g, cx.other));
            log(g, &s);
        }
        (cx.this, cx.other) = outer;
        Ok(())
    }
    let prog = build(|b| {
        let a = b.push_object(ObjectInfo::new("a"));
        let t = b.push_object(ObjectInfo::new("t"));
        b.push_object(child("u", t));
        b.event(a, Event::Collision(t), hit);
    });
    let mut g = world(prog, &[ObjectId(0), ObjectId(2)]);
    let (a, u) = (handle(&g, 1), handle(&g, 2));
    // u is a t by inheritance; it has no handler of its own for a.
    let h = *g
        .program()
        .collision_handler(ObjectId(0), ObjectId(2))
        .unwrap();
    assert!(
        g.program()
            .collision_handler(ObjectId(2), ObjectId(0))
            .is_none()
    );
    g.run_handler(a, u, h).unwrap();
    assert_eq!(take_log(&mut g), "a1<-u2 with:u2/a1");
}

#[test]
fn event_inherited_runs_the_defining_objects_parent_handler() {
    fn say(g: &mut World, cx: &Ctx, what: &str) {
        let s = format!("{what}@{}", tag(g, cx.this));
        log(g, &s);
    }
    fn base(g: &mut World, cx: &mut Ctx) -> R<()> {
        say(g, cx, "base");
        Ok(())
    }
    fn mid(g: &mut World, cx: &mut Ctx) -> R<()> {
        say(g, cx, "mid");
        g.event_inherited(cx)
    }
    fn leaf(g: &mut World, cx: &mut Ctx) -> R<()> {
        say(g, cx, "leaf");
        g.event_inherited(cx)?;
        say(g, cx, "after");
        Ok(())
    }
    let prog = build(|b| {
        let o_base = b.push_object(ObjectInfo::new("base"));
        let o_mid = b.push_object(child("mid", o_base));
        let o_leaf = b.push_object(child("leaf", o_mid));
        let o_skip = b.push_object(child("skip", o_mid));
        let o_deep = b.push_object(child("deep", o_skip));
        b.event(o_base, Event::Step, base)
            .event(o_mid, Event::Step, mid)
            .event(o_leaf, Event::Step, leaf)
            .event(o_deep, Event::Step, leaf);
    });
    let mut g = world(prog, &[ObjectId(2), ObjectId(3), ObjectId(4)]);
    for n in 1..=3 {
        run(&mut g, n, Event::Step);
    }
    // skip defines no step, so it runs mid's. deep's inherited call starts at skip, which resolves to mid.
    assert_eq!(
        take_log(&mut g),
        "leaf@leaf1 mid@leaf1 base@leaf1 after@leaf1 \
         mid@skip2 base@skip2 \
         leaf@deep3 mid@deep3 base@deep3 after@deep3"
    );
}

#[test]
fn instance_create_runs_the_create_event_at_once() {
    fn spawn(g: &mut World, cx: &mut Ctx) -> R<()> {
        let id = g.instance_create(5.0, 6.0, 1)?;
        log(g, &format!("made{}", id - FIRST_ID + 1));
        // The new instance is in every list already.
        let all = with_ids(g, cx, -3.0)?;
        log(g, &format!("all:{all:?}"));
        Ok(())
    }
    fn create(g: &mut World, cx: &mut Ctx) -> R<()> {
        note(g, cx)?;
        // In a create event, `other` is the new instance itself.
        assert_eq!(cx.other, cx.this);
        let x = g.get(cx.this, Builtin::X.var())?;
        log(g, &format!("x{}", x.to_int()));
        Ok(())
    }
    let prog = build(|b| {
        let s = b.push_object(ObjectInfo::new("s"));
        let k = b.push_object(ObjectInfo::new("k"));
        b.event(s, Event::Step, spawn)
            .event(k, Event::Create, create);
    });
    let mut g = world(prog, &[ObjectId(0)]);
    run(&mut g, 1, Event::Step);
    run(&mut g, 1, Event::Step);
    assert_eq!(
        take_log(&mut g),
        "k101.create x5 made101 all:[1, 101] k102.create x5 made102 all:[1, 101, 102]"
    );
    assert_eq!(g.instance_create(0.0, 0.0, 7), Err(Error::NoSuchObject(7)));
}

#[test]
fn destroying_inside_with_skips_the_destroyed_and_keeps_running() {
    const HP: Var = Var(FIRST_USER_VAR);
    fn create_t(g: &mut World, cx: &mut Ctx) -> R<()> {
        let id = g.get(cx.this, ID)?.to_int();
        g.set(
            cx.this,
            HP,
            Value::Real(f64::from(id - FIRST_ID + 1) * 10.0),
        )
    }
    fn ctl_step(g: &mut World, cx: &mut Ctx) -> R<()> {
        let mut it = g.with_iter(cx, &Value::Real(1.0))?;
        let outer = (cx.this, cx.other);
        while let Some(h) = it.next(g) {
            (cx.this, cx.other) = (h, outer.0);
            let me = tag(g, cx.this);
            log(g, &format!("visit:{me}"));
            if me == "t2" {
                let t3 = g.find(FIRST_ID + 2).unwrap();
                g.destroy(t3)?;
                g.instance_destroy(cx)?;
                // The destroyed instance's variables stay readable until the end-of-step cleanup.
                let hp = g.get(cx.this, HP)?.to_int();
                let exists = g.instance_exists(cx, FIRST_ID + 1);
                log(g, &format!("hp{hp} exists:{exists}"));
            }
        }
        (cx.this, cx.other) = outer;
        Ok(())
    }
    let prog = build(|b| {
        let hp = b.var("hp");
        assert_eq!(hp, HP);
        let ctl = b.push_object(ObjectInfo::new("ctl"));
        let t = b.push_object(ObjectInfo::new("t"));
        b.layout(t, &[hp]);
        b.event(ctl, Event::Step, ctl_step);
        b.event(t, Event::Create, create_t)
            .event(t, Event::Destroy, note);
    });
    let mut g = world(prog, &[ObjectId(0), ObjectId(1), ObjectId(1), ObjectId(1)]);
    let t2 = handle(&g, 2);
    run(&mut g, 1, Event::Step);
    assert_eq!(
        take_log(&mut g),
        "visit:t2 t3.destroy t2.destroy hp20 exists:false visit:t4"
    );
    assert_eq!(g.instance(t2).state(), State::Deleted);
    assert_eq!(g.find(FIRST_ID + 1), None);
    let cx = ctx(&g, handle(&g, 1));
    assert_eq!(g.instance_number(&cx, 1), 1);
    assert_eq!(g.instance_find(1, 0), FIRST_ID + 3);
    assert_eq!(g.instance_find(1, 1), -4);
    assert_eq!(g.instance_find(1, i32::MAX), -4);
}

#[test]
fn no_event_runs_while_a_room_change_is_pending() {
    let prog = build(|b| {
        let a = b.push_object(ObjectInfo::new("a"));
        b.event(a, Event::Step, note).event(a, Event::Destroy, note);
    });
    let mut g = world(prog, &[ObjectId(0)]);
    run(&mut g, 1, Event::Step);
    // `room = 5` requests a room change, like room_goto.
    g.bset(Builtin::Room.var(), 0, Value::Real(5.0)).unwrap();
    assert_eq!(g.pending(), Some(SceneChange::Room(5)));
    run(&mut g, 1, Event::Step);
    // Destroying still marks the instance, without its destroy event.
    let h = handle(&g, 1);
    g.destroy(h).unwrap();
    assert_eq!(take_log(&mut g), "a1.step");
    assert_eq!(g.instance(h).state(), State::Deleted);
}

#[test]
fn room_start_skips_create_events_of_instances_already_gone() {
    // a1's create event draws from the RNG and destroys b2, which then gets no create event (and so
    // doesn't draw); c3's create event still runs and sees the RNG state a1 left.
    fn create_a(g: &mut World, cx: &mut Ctx) -> R<()> {
        note(g, cx)?;
        g.rng().random(1.0);
        let b2 = g.find(FIRST_ID + 1).unwrap();
        g.destroy(b2)
    }
    fn create_rng(g: &mut World, cx: &mut Ctx) -> R<()> {
        note(g, cx)?;
        let seed = g.rng().seed();
        log(g, &format!("seed{seed}"));
        g.rng().random(1.0);
        Ok(())
    }
    let prog = build(|b| {
        let a = b.push_object(ObjectInfo::new("a"));
        let bb = b.push_object(ObjectInfo::new("b"));
        let c = b.push_object(ObjectInfo::new("c"));
        b.event(a, Event::Create, create_a);
        b.event(bb, Event::Create, create_rng);
        b.event(c, Event::Create, create_rng);
    });
    let mut g = world(prog, &[ObjectId(0), ObjectId(1), ObjectId(2)]);
    let mut rng = Rng::with_seed(12345);
    rng.random(1.0);
    assert_eq!(
        take_log(&mut g),
        format!("a1.create c3.create seed{}", rng.seed())
    );

    // A room placing an object that doesn't exist is an error, as in GM8's room loading.
    let room = g.room().clone();
    let bad = PlacedInstance {
        id: FIRST_ID,
        object: ObjectId(9),
        x: 0.0,
        y: 0.0,
    };
    let err = g
        .start_room(RoomStart {
            room,
            instances: vec![bad],
        })
        .unwrap_err();
    assert_eq!(err.error, Error::NoSuchObject(9));
}
