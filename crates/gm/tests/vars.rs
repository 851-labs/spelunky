//! Variable storage and access: user variables (layout and overflow), arrays, globals, built-ins, and
//! `target.v`.

mod common;

use common::*;
use gm::*;

const HP: Var = Var(FIRST_USER_VAR);
const GRID: Var = Var(FIRST_USER_VAR + 1);
const STRAY: Var = Var(FIRST_USER_VAR + 2);
const SCORE: GVar = GVar(1);

fn program() -> &'static Program {
    build(|b| {
        assert_eq!(b.var("hp"), HP);
        assert_eq!(b.var("grid"), GRID);
        assert_eq!(b.var("stray"), STRAY);
        assert_eq!(b.global("score"), SCORE);
        let o = b.push_object(ObjectInfo::new("o"));
        b.layout(o, &[HP, GRID]);
        b.push_object(ObjectInfo::new("ctl"));
    })
}

fn r(x: f64) -> Value {
    Value::Real(x)
}

#[test]
fn user_variables_live_in_slots_or_overflow() {
    let mut g = world(program(), &[ObjectId(0)]);
    let h = g.find(FIRST_ID).unwrap();
    assert_eq!(g.get(h, HP), Err(Error::UnsetVar(HP)));
    g.set(h, HP, r(3.0)).unwrap();
    // Not in o's layout: stored in the overflow list, same behaviour.
    g.set(h, STRAY, Value::from("x")).unwrap();
    assert_eq!(g.get(h, HP), Ok(r(3.0)));
    assert_eq!(g.get(h, STRAY), Ok(Value::from("x")));
}

#[test]
fn arrays_follow_gm8_index_rules() {
    let mut g = world(program(), &[ObjectId(0)]);
    let h = g.find(FIRST_ID).unwrap();
    let j = ix(&r(2.6)).unwrap();
    g.set_a(h, GRID, 0, j, r(1.0)).unwrap();
    // grid[3] was set, so grid (grid[0]) and grid[1] read 0; grid[4] is unset.
    assert_eq!(g.get_a(h, GRID, 0, 3), Ok(r(1.0)));
    assert_eq!(g.get(h, GRID), Ok(r(0.0)));
    assert_eq!(g.get_a(h, GRID, 0, 1), Ok(r(0.0)));
    assert_eq!(g.get_a(h, GRID, 0, 4), Err(Error::UnsetVar(GRID)));
    // 2-D rows are separate.
    g.set_a(h, GRID, 2, 5, r(7.0)).unwrap();
    assert_eq!(g.get_a(h, GRID, 2, 4), Ok(r(0.0)));
    assert_eq!(g.get_a(h, GRID, 1, 0), Err(Error::UnsetVar(GRID)));
    assert_eq!(ix(&r(32000.0)), Err(Error::BadArrayIndex(32000)));
    // Locals have the same storage rules.
    let mut l = Local::default();
    assert_eq!(l.get(), Err(Error::UnsetLocal));
    l.set_a(0, 2, r(5.0));
    assert_eq!(l.get(), Ok(r(0.0)));
}

#[test]
fn globals() {
    let mut g = world(program(), &[]);
    assert_eq!(g.gget(SCORE), Err(Error::UnsetGlobal(SCORE)));
    g.gset(SCORE, r(10.0)).unwrap();
    g.gset_a(SCORE, 0, 1, r(11.0)).unwrap();
    assert_eq!(g.gget(SCORE), Ok(r(10.0)));
    assert_eq!(g.gget_a(SCORE, 0, 1), Ok(r(11.0)));
}

#[test]
fn builtins_have_side_effects_and_read_only_ones_refuse_writes() {
    let mut g = world(program(), &[ObjectId(0)]);
    let h = g.find(FIRST_ID).unwrap();
    let v = |b: Builtin| b.var();
    g.set(h, v(Builtin::Speed), r(4.0)).unwrap();
    g.set(h, v(Builtin::Direction), r(180.0)).unwrap();
    assert_eq!(g.get(h, v(Builtin::Hspeed)), Ok(r(-4.0)));
    assert_eq!(g.get(h, v(Builtin::Vspeed)), Ok(r(0.0)));
    g.set(h, v(Builtin::Visible), r(0.4)).unwrap();
    assert_eq!(g.get(h, v(Builtin::Visible)), Ok(r(0.0)));
    assert_eq!(g.get(h, v(Builtin::Id)), Ok(r(f64::from(FIRST_ID))));
    assert_eq!(g.get(h, v(Builtin::ObjectIndex)), Ok(r(0.0)));
    assert_eq!(
        g.set(h, v(Builtin::Id), r(1.0)),
        Err(Error::ReadOnly(v(Builtin::Id)))
    );
    // No sprite: sizes read 0, the box reads GM8's default.
    assert_eq!(g.get(h, v(Builtin::SpriteWidth)), Ok(r(0.0)));
    assert_eq!(g.get(h, v(Builtin::BboxTop)), Ok(r(-100000.0)));
    // Game-wide built-ins, also through an instance (`other.room`).
    assert_eq!(g.bget(v(Builtin::RoomWidth), 0), Ok(r(320.0)));
    assert_eq!(g.get(h, v(Builtin::Room)), Ok(r(0.0)));
    assert_eq!(g.bget(v(Builtin::InstanceCount), 0), Ok(r(1.0)));
    assert!(matches!(
        g.bget(v(Builtin::ViewXview), 0),
        Err(Error::Unsupported(_))
    ));
}

#[test]
fn target_access_reads_the_first_and_writes_all() {
    let mut g = world(program(), &[ObjectId(0), ObjectId(0), ObjectId(1)]);
    let ctl = g.find(FIRST_ID + 2).unwrap();
    let cx = ctx(&g, ctl);
    let o = r(0.0);
    assert_eq!(g.get_on(&cx, &o, HP), Err(Error::UnsetVar(HP)));
    g.set_on(&cx, &o, HP, r(5.0)).unwrap();
    let o2 = g.find(FIRST_ID + 1).unwrap();
    g.set(o2, HP, r(6.0)).unwrap();
    assert_eq!(g.get_on(&cx, &o, HP), Ok(r(5.0)));
    assert_eq!(g.get_on(&cx, &r(f64::from(FIRST_ID + 1)), HP), Ok(r(6.0)));
    // `noone.v = x` does nothing; reading it is an error.
    g.set_on(&cx, &r(-4.0), HP, r(1.0)).unwrap();
    assert_eq!(g.get_on(&cx, &r(-4.0), HP), Err(Error::UnsetVar(HP)));
    // `all.x = 0` writes every instance.
    g.set_on(&cx, &r(-3.0), Builtin::X.var(), r(0.0)).unwrap();
    assert!(g.active().all(|h| g.instance(h).x == 0.0));
    g.set_on_a(&cx, &r(-1.0), GRID, 1, 1, r(2.0)).unwrap();
    assert_eq!(g.get_on_a(&cx, &r(-1.0), GRID, 1, 1), Ok(r(2.0)));
}
