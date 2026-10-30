//! Helpers for building small synthetic programs with hand-written event functions.

#![allow(dead_code)]

use gm::*;
use std::sync::Arc;

/// Every test program's first global: a space-separated log the event functions append to.
pub const LOG: GVar = GVar(0);

/// Instance ids start here (GM8's first room-placed id is 100001).
pub const FIRST_ID: i32 = 100_001;

/// `instance_create` ids start after this.
pub const LAST_ID: i32 = 100_100;

/// Font slots in the game file, as in Spelunky's (one, deleted): `font_add_sprite` numbers from 1.
pub const FONT_SLOTS: usize = 1;

/// Builds a program whose global 0 is the log, and leaks it: worlds hold a `&'static Program`.
pub fn build(f: impl FnOnce(&mut ProgramBuilder)) -> &'static Program {
    let mut b = ProgramBuilder::new();
    assert_eq!(b.global("log"), LOG);
    f(&mut b);
    Box::leak(Box::new(b.build()))
}

pub fn child(name: &str, parent: ObjectId) -> ObjectInfo {
    ObjectInfo {
        parent: Some(parent),
        ..ObjectInfo::new(name)
    }
}

/// A sprite with `frames` frames, `size` pixels square, a full rectangle mask and origin (0, 0).
pub fn sprite(name: &str, frames: usize, size: u32) -> assets::Sprite {
    let rect = assets::AtlasRect {
        page: 0,
        x: 0,
        y: 0,
        width: size,
        height: size,
    };
    assets::Sprite {
        name: name.into(),
        width: size,
        height: size,
        origin_x: 0,
        origin_y: 0,
        frames: vec![rect; frames],
        shape: assets::MaskShape::Rectangle,
        bbox_mode: assets::BboxMode::FullImage,
        alpha_tolerance: 0,
        separate_masks: false,
        masks: vec![assets::Mask {
            bbox: assets::Bbox {
                left: 0,
                top: 0,
                right: size - 1,
                bottom: size - 1,
            },
            data: assets::MaskData::Filled,
        }],
    }
}

/// A 320x240 room holding one instance of each object, in order, at (10, 10), with ids from
/// [`FIRST_ID`]. The game has started there: create events and the frame-0 draw have run.
pub fn world(p: &'static Program, placed: &[ObjectId]) -> World {
    world_with(p, Assets::default(), placed)
}

/// [`world`] with these assets; its room replaces their rooms.
pub fn world_with(p: &'static Program, assets: Assets, placed: &[ObjectId]) -> World {
    let instances: Vec<_> = placed
        .iter()
        .enumerate()
        .map(|(i, &object)| (FIRST_ID + i as i32, object, 10, 10))
        .collect();
    let assets = assets.with_rooms(vec![Some(room(320, 240, &instances))]);
    let mut w = World::new(p, Arc::new(assets), 12345, LAST_ID, FONT_SLOTS);
    w.start_game().unwrap();
    w
}

/// Starts a game in these `(id, room)` rooms: the lowest id first.
pub fn start(p: &'static Program, assets: Assets, rooms: &[(usize, assets::Room)]) -> World {
    let mut all = vec![None; rooms.iter().map(|r| r.0 + 1).max().unwrap_or(0)];
    for (id, r) in rooms {
        all[*id] = Some(r.clone());
    }
    let mut w = World::new(
        p,
        Arc::new(assets.with_rooms(all)),
        12345,
        LAST_ID,
        FONT_SLOTS,
    );
    w.start_game().unwrap();
    w
}

/// `(object, x, y)` placements numbered from [`FIRST_ID`].
pub fn numbered(placed: &[(ObjectId, i32, i32)]) -> Vec<(i32, ObjectId, i32, i32)> {
    placed
        .iter()
        .enumerate()
        .map(|(i, &(o, x, y))| (FIRST_ID + i as i32, o, x, y))
        .collect()
}

/// A room with views off and these `(id, object, x, y)` instances, in placement order.
pub fn room(width: i32, height: i32, placed: &[(i32, ObjectId, i32, i32)]) -> assets::Room {
    assets::Room {
        name: format!("room{width}x{height}"),
        width,
        height,
        speed: 30,
        instances: placed
            .iter()
            .map(|&(id, object, x, y)| assets::Instance {
                x,
                y,
                object: object.0 as i32,
                id,
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    }
}

/// Appends to the log.
pub fn log(g: &mut World, s: &str) {
    let mut text = g
        .gget(LOG)
        .map(|v| v.to_gml_string().to_string())
        .unwrap_or_default();
    if !text.is_empty() {
        text.push(' ');
    }
    text.push_str(s);
    g.gset(LOG, Value::from(text.as_str())).unwrap();
}

/// Returns the log and clears it.
pub fn take_log(g: &mut World) -> String {
    let s = g
        .gget(LOG)
        .map(|v| v.to_gml_string().to_string())
        .unwrap_or_default();
    g.gset(LOG, Value::from("")).unwrap();
    s
}

/// An instance as `name` plus its id's offset from the first id, e.g. `a1`. Created instances are
/// numbered from 100.
pub fn tag(g: &World, h: Inst) -> String {
    let i = g.instance(h);
    let name = &g.program().object(i.object).unwrap().name;
    format!("{name}{}", i.id - FIRST_ID + 1)
}

pub fn event_name(e: Event) -> String {
    match e {
        Event::Create => "create".into(),
        Event::Destroy => "destroy".into(),
        Event::Alarm(n) => format!("alarm{n}"),
        Event::BeginStep => "begin".into(),
        Event::Step => "step".into(),
        Event::EndStep => "end".into(),
        Event::Draw => "draw".into(),
        Event::Other(0) => "outside".into(),
        Event::Other(2) => "gamestart".into(),
        Event::Other(3) => "gameend".into(),
        Event::Other(4) => "roomstart".into(),
        Event::Other(5) => "roomend".into(),
        Event::Other(7) => "anim".into(),
        Event::Other(30) => "close".into(),
        Event::KeyPress(k) => format!("press{k}"),
        Event::Collision(o) => format!("hit{}", o.0),
        e => format!("{e:?}"),
    }
}

/// An event function that logs `tag.event`.
pub fn note(g: &mut World, cx: &mut Ctx) -> R<()> {
    let s = format!("{}.{}", tag(g, cx.this), event_name(cx.current_event()));
    log(g, &s);
    Ok(())
}

/// Runs one frame without input.
pub fn step(g: &mut World) {
    g.step(&[]).unwrap();
}

/// A context for calling built-ins from test code, as if `h` were running its step event.
pub fn ctx(g: &World, h: Inst) -> Ctx {
    Ctx::event(h, h, Event::Step, g.instance(h).object)
}

/// The ids `with (target)` visits, checking `other` is the outer `self` inside and both are restored.
pub fn with_ids(g: &mut World, cx: &mut Ctx, target: f64) -> R<Vec<i32>> {
    let mut ids = Vec::new();
    let mut it = g.with_iter(cx, &Value::Real(target))?;
    let outer = (cx.this, cx.other);
    while let Some(h) = it.next(g) {
        (cx.this, cx.other) = (h, outer.0);
        ids.push(g.get(cx.this, Builtin::Id.var())?.to_int()? - FIRST_ID + 1);
        assert_eq!(cx.other, outer.0);
    }
    (cx.this, cx.other) = outer;
    Ok(ids)
}
