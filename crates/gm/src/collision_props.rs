//! Property tests: the spatial index (`index.rs`) and the collision stage's shortcut through it give
//! exactly what linear scans give, on random worlds that change between queries in every way the index
//! has to follow (moves, sprite, mask, scale and angle changes, creation, destruction, cleanup,
//! (de)activation, depth sorts, clones).

use crate::collision::{Body, Query, bbox_distance, bodies_collide};
use crate::ctx::Ctx;
use crate::error::R;
use crate::kernel::{ALL, FIRST_INSTANCE_ID, NOONE, OTHER, SELF};
use crate::list::{Inst, InstIter};
use crate::program::{Event, ObjectId, ObjectInfo, Program, ProgramBuilder};
use crate::value::{Value, ValueError};
use crate::var::{Builtin, GVar};
use crate::world::{Assets, World};
use std::sync::Arc;

/// A small deterministic generator (64-bit LCG), the same on every platform.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    /// 0..n.
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    fn chance(&mut self, percent: u64) -> bool {
        self.below(100) < percent
    }

    /// A coordinate: mostly in and around a 320x240 room, sometimes fractional, far away or NaN.
    fn coord(&mut self) -> f64 {
        match self.below(40) {
            0 => f64::NAN,
            1 => 1e7,
            2 => -3e5,
            3..=9 => self.below(400) as f64 - 40.0 + self.below(4) as f64 * 0.25,
            _ => self.below(360) as f64 - 20.0,
        }
    }

    fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.below(xs.len() as u64) as usize]
    }
}

const LOG: GVar = GVar(0);
// Objects: 0 base; 1 a (child of base); 2 b (child of a); 3 c; 4 d (no sprite); 5 wall (solid).
const OBJECTS: u32 = 6;

fn meet(g: &mut World, cx: &mut Ctx) -> R<()> {
    let (me, them) = (g.instance(cx.this).id, g.instance(cx.other).id);
    let log = g.gget(LOG)?.to_gml_string().to_string();
    g.gset(LOG, Value::from(format!("{log} {me}-{them}").as_str()))?;
    // Change the world mid-stage in the ways that matter to the loops.
    match (me + them) % 7 {
        0 => g.destroy(cx.other)?,
        1 => {
            let x = g.instance(cx.this).x + 5.0;
            g.set(cx.this, Builtin::X.var(), Value::Real(x))?;
        }
        2 => {
            let (x, y) = (g.instance(cx.this).x, g.instance(cx.this).y);
            g.instance_create(x + 3.0, y, (me % 4) + 1)?;
        }
        3 => g.instance_deactivate_object(cx, them),
        4 => g.set(cx.other, Builtin::Y.var(), Value::Real(f64::from(me % 50)))?,
        _ => {}
    }
    Ok(())
}

/// The test program, leaked as worlds need (`gm` keeps no globals). Each test builds it once.
fn program() -> &'static Program {
    Box::leak(Box::new({
        let mut b = ProgramBuilder::new();
        b.global("log");
        let base = b.push_object(ObjectInfo::new("base"));
        let a = b.push_object(ObjectInfo {
            sprite: 0,
            parent: Some(base),
            ..ObjectInfo::new("a")
        });
        let bb = b.push_object(ObjectInfo {
            sprite: 1,
            parent: Some(a),
            ..ObjectInfo::new("b")
        });
        let c = b.push_object(ObjectInfo {
            sprite: 2,
            ..ObjectInfo::new("c")
        });
        b.push_object(ObjectInfo::new("d"));
        let wall = b.push_object(ObjectInfo {
            sprite: 0,
            solid: true,
            ..ObjectInfo::new("wall")
        });
        b.event(a, Event::Collision(c), meet)
            .event(c, Event::Collision(bb), meet)
            .event(bb, Event::Collision(bb), meet)
            .event(wall, Event::Collision(base), meet);
        b.build()
    }))
}

/// A sprite with random masks: per-frame or shared, some pixels set, origin anywhere in it.
fn sprite(r: &mut Lcg, w: u32, h: u32, frames: usize) -> assets::Sprite {
    let separate = frames > 1;
    let masks = (0..if separate { frames } else { 1 })
        .map(|_| {
            let (l, t) = (
                r.below(u64::from(w)) as u32 / 2,
                r.below(u64::from(h)) as u32 / 2,
            );
            let (rt, bt) = (w - 1 - r.below(u64::from(w / 2)) as u32, h - 1);
            let n = ((rt - l + 1) * (bt - t + 1)) as usize;
            let mut bits = vec![0u8; n.div_ceil(8)];
            for i in 0..n {
                if r.chance(70) {
                    bits[i / 8] |= 1 << (i % 8);
                }
            }
            assets::Mask {
                bbox: assets::Bbox {
                    left: l,
                    top: t,
                    right: rt,
                    bottom: bt,
                },
                data: assets::MaskData::Bits(bits),
            }
        })
        .collect();
    let rect = assets::AtlasRect {
        page: 0,
        x: 0,
        y: 0,
        width: w,
        height: h,
    };
    assets::Sprite {
        name: "s".into(),
        width: w,
        height: h,
        origin_x: r.below(u64::from(w)) as i32 - 2,
        origin_y: r.below(u64::from(h)) as i32,
        frames: vec![rect; frames],
        shape: assets::MaskShape::Precise,
        bbox_mode: assets::BboxMode::Automatic,
        alpha_tolerance: 0,
        separate_masks: separate,
        masks,
    }
}

/// A world with up to 70 random instances placed in the top left `width` x `height` pixels.
fn world(r: &mut Lcg, p: &'static Program, width: u64, height: u64) -> World {
    let sprites = vec![
        Some(sprite(r, 16, 16, 1)),
        Some(sprite(r, 8, 12, 3)),
        Some(sprite(r, 40, 6, 2)),
    ];
    let instances = (0..r.below(70) as i32)
        .map(|i| assets::Instance {
            id: 100_001 + i,
            object: r.below(u64::from(OBJECTS)) as i32,
            x: r.below(width) as i32,
            y: r.below(height) as i32,
            ..Default::default()
        })
        .collect();
    let room = assets::Room {
        width: 320,
        height: 240,
        speed: 30,
        instances,
        ..Default::default()
    };
    let assets = Assets::new(sprites, vec![Some(room)]);
    let mut w = World::new(p, Arc::new(assets), 1, 200_000, 0);
    w.start_game().unwrap();
    w.gset(LOG, Value::from("")).unwrap();
    w
}

/// Every instance in the slab, in draw order.
fn all(w: &World) -> Vec<Inst> {
    w.insts.draw_order().to_vec()
}

fn random_inst(r: &mut Lcg, w: &World) -> Option<Inst> {
    let v = all(w);
    (!v.is_empty()).then(|| v[r.below(v.len() as u64) as usize])
}

/// `instance_create`, which a far or NaN position stops with the box's trap (`Error::CreationTrap`).
fn create(w: &mut World, x: f64, y: f64, o: i32) {
    match w.instance_create(x, y, o) {
        Ok(_) | Err(crate::Error::CreationTrap(_)) => {}
        Err(e) => panic!("{e}"),
    }
}

/// One random change to the world.
fn mutate(r: &mut Lcg, w: &mut World) {
    let Some(h) = random_inst(r, w) else {
        create(w, r.coord(), r.coord(), 1);
        return;
    };
    let cx = Ctx::event(h, h, Event::Step, w.instance(h).object);
    // A setter's write is stored even where the box then traps (a far or NaN position).
    let set = |w: &mut World, b: Builtin, v: f64| {
        if let Err(e) = w.set(h, b.var(), Value::Real(v)) {
            assert!(e.is_exception(), "{e}");
        }
    };
    match r.below(16) {
        0 => set(w, Builtin::X, r.coord()),
        1 => set(w, Builtin::Y, r.coord()),
        2 => w.instance_mut(h).x += 1.5,
        3 => set(
            w,
            Builtin::ImageXscale,
            r.pick(&[1.0, -1.0, 2.0, 0.5, -1.5, 0.0]),
        ),
        4 => set(w, Builtin::ImageYscale, r.pick(&[1.0, -1.0, 3.0, 0.25])),
        5 => set(
            w,
            Builtin::ImageAngle,
            r.pick(&[0.0, 90.0, 45.0, 180.0, 300.5]),
        ),
        6 => set(w, Builtin::SpriteIndex, r.pick(&[-1.0, 0.0, 1.0, 2.0, 7.0])),
        7 => set(w, Builtin::MaskIndex, r.pick(&[-1.0, 0.0, 1.0, 2.0])),
        // 2^63 traps in a precise test's `Floor` (851-3305).
        8 => set(
            w,
            Builtin::ImageIndex,
            r.pick(&[0.0, 1.5, 2.0, -1.0, 5.0, 9_223_372_036_854_775_808.0]),
        ),
        9 => {
            let o = r.below(u64::from(OBJECTS)) as i32;
            create(w, r.coord(), r.coord(), o);
        }
        10 => w.destroy(h).unwrap(),
        11 => w.insts.remove_deleted(w.program),
        12 => {
            let o = r.below(u64::from(OBJECTS) + 2) as i32;
            if r.chance(50) {
                w.instance_deactivate_object(&cx, o);
            } else {
                w.instance_activate_object(&cx, o);
            }
        }
        13 => {
            let (x, y) = (r.below(320) as f64, r.below(240) as f64);
            if r.chance(50) {
                w.instance_deactivate_region(&cx, x, y, 80.0, 60.0, r.chance(50), r.chance(50));
            } else {
                w.instance_activate_region(x, y, 120.0, 90.0, true);
            }
        }
        14 => {
            set(w, Builtin::Depth, r.below(5) as f64);
            // The default draw's `Floor` traps on an `image_index` of 2^63.
            if let Err(e) = w.redraw() {
                assert!(e.is_exception(), "{e}");
            }
        }
        _ => *w = w.clone(),
    }
}

/// An instance's body with its box computed afresh, not from the index.
fn fresh(w: &World, h: Inst) -> Body {
    let i = w.instance(h);
    Body::new(i, i.x, i.y, w.assets.bbox_at(i, i.x, i.y))
}

/// The reference: `find_instance_with` as OpenGMK writes it, scanning the lists, stopping at a trap as GM8 does.
fn naive(w: &World, target: i32, q: &Query, skip: Option<Inst>) -> R<Option<Inst>> {
    let stop = |h: Inst| -> Option<Result<Inst, ValueError>> {
        if Some(h) == skip {
            return None;
        }
        q.hits(&w.assets, &fresh(w, h))
            .map(|hit| hit.then_some(h))
            .transpose()
    };
    let first = |mut it: InstIter| std::iter::from_fn(|| it.next(&w.insts)).find_map(stop);
    let found = match target {
        ALL => first(w.insts.iter_drawing()),
        t if t < 0 => None,
        t if t < FIRST_INSTANCE_ID => first(w.insts.iter_ancestor(ObjectId(t as u32))),
        id => w.insts.by_id(id).and_then(stop),
    };
    Ok(found.transpose()?)
}

fn random_target(r: &mut Lcg, w: &World) -> i32 {
    match r.below(10) {
        0 => ALL,
        1 => r.pick(&[SELF, OTHER, NOONE]),
        2 | 3 => random_inst(r, w).map_or(NOONE, |h| w.instance(h).id),
        _ => r.below(u64::from(OBJECTS) + 1) as i32,
    }
}

fn random_query(r: &mut Lcg, w: &World, me: Inst) -> Query {
    let precise = r.chance(50);
    match r.below(4) {
        0 => Query::Point {
            x: r.coord(),
            y: r.coord(),
            precise,
        },
        1 => {
            let c = |r: &mut Lcg| r.below(400) as i32 - 40;
            let big = if r.chance(10) { 2000 } else { 0 };
            Query::rect(c(r), c(r), c(r) + big, c(r), precise)
        }
        2 => Query::Line {
            x1: r.coord(),
            y1: r.coord(),
            x2: r.coord(),
            y2: r.coord(),
            precise,
        },
        _ => {
            let i = w.instance(me);
            let (x, y) = (r.coord(), r.coord());
            Query::Body(Body::new(i, x, y, w.assets.bbox_at(i, x, y)))
        }
    }
}

#[test]
fn indexed_queries_find_what_linear_scans_find() {
    let mut r = Lcg(0x5eed_3127);
    let (mut checked, mut trapped) = (0, 0);
    let p = program();
    for _ in 0..60 {
        let mut w = world(&mut r, p, 240, 240);
        for _ in 0..120 {
            mutate(&mut r, &mut w);
            let Some(me) = random_inst(&mut r, &w) else {
                continue;
            };
            for _ in 0..4 {
                let target = random_target(&mut r, &w);
                let q = random_query(&mut r, &w, me);
                let skip = r.chance(50).then_some(me);
                let want = naive(&w, target, &q, skip);
                let got = w.find_hit(target, &q, skip);
                assert_eq!(got, want, "target {target}, {q:?}, skip {skip:?}");
                checked += 1;
                trapped += usize::from(got.is_err());
            }
            // The cached boxes are the fresh ones.
            let ix = w.index();
            for h in all(&w) {
                assert_eq!(ix.bbox(h), w.bbox(h));
            }
            drop(ix);
            // distance_to_object reads the cache.
            let cx = Ctx::event(me, me, Event::Step, w.instance(me).object);
            let target = random_target(&mut r, &w);
            let got = w.distance_to_object(&cx, target);
            let want = naive_distance(&w, me, target);
            assert_eq!(got.to_bits(), want.to_bits(), "distance to {target}");
        }
    }
    assert!(checked > 20_000);
    // Some scans stop at a trap (an `image_index` of 2^63), which the index must find first too.
    assert!(trapped > 10, "{trapped} traps");
}

/// `distance_to_object` with fresh boxes, for `all`, objects and ids.
fn naive_distance(w: &World, me: Inst, target: i32) -> f64 {
    let b = |h| w.bbox(h);
    let to = |h| bbox_distance(&b(me), &b(h));
    let nearest = |mut it: InstIter| {
        std::iter::from_fn(|| it.next(&w.insts))
            .filter(|&h| h != me)
            .map(to)
            .fold(1_000_000.0, |best: f64, d| if d < best { d } else { best })
    };
    match target {
        OTHER => 1_000_000.0,
        ALL => nearest(w.insts.iter_drawing()),
        o if o <= FIRST_INSTANCE_ID => {
            u32::try_from(o).map_or(1_000_000.0, |o| nearest(w.insts.iter_ancestor(ObjectId(o))))
        }
        id => w
            .insts
            .by_id(id)
            .filter(|&h| h != me)
            .map_or(1_000_000.0, to),
    }
}

/// The collision stage as OpenGMK writes it: every pair of instances tested, with fresh boxes.
fn naive_collisions(w: &mut World) -> R<()> {
    let program = w.program;
    let collide = |w: &World, i: Inst, j: Inst| -> R<bool> {
        Ok(i != j && bodies_collide(&w.assets, &fresh(w, i), &fresh(w, j))?)
    };
    for (a, bs) in program.collision_pairs() {
        let mut it = w.insts.iter_object(a);
        'instance: while let Some(i) = it.next(&w.insts) {
            for &b in bs {
                let mut targets = w.insts.iter_object(b);
                while let Some(j) = targets.next(&w.insts) {
                    if !collide(w, i, j)? {
                        continue;
                    }
                    let solid = |w: &World| w.instance(i).solid || w.instance(j).solid;
                    // Through the position setter, whose box traps at a NaN position as the stage's does.
                    let back = |w: &mut World, h: Inst| {
                        let inst = w.instance(h);
                        w.set_position(h, inst.xprevious, inst.yprevious)
                    };
                    if solid(w) {
                        back(w, i)?;
                        back(w, j)?;
                    }
                    if let Some(&h) = program.collision_handler(a, b) {
                        w.run_handler(i, j, h)?;
                    }
                    if let Some(&h) = program.collision_handler(b, a) {
                        w.run_handler(j, i, h)?;
                    }
                    if solid(w) {
                        w.apply_speeds(i)?;
                        w.apply_speeds(j)?;
                        let active = |h| w.instance(h).is_active();
                        if active(i) && active(j) && collide(w, i, j)? {
                            back(w, i)?;
                            back(w, j)?;
                        }
                    }
                    if !w.instance(i).is_active() {
                        continue 'instance;
                    }
                }
            }
        }
    }
    Ok(())
}

/// Everything the stage can change: the log, and every instance's id, state and position.
fn snapshot(w: &World) -> (String, Vec<(i32, crate::State, u64, u64)>) {
    let log = w.gget(LOG).unwrap().to_gml_string().to_string();
    let insts = all(w)
        .into_iter()
        .map(|h| {
            let i = w.instance(h);
            (i.id, i.state(), i.x.to_bits(), i.y.to_bits())
        })
        .collect();
    (log, insts)
}

/// Runs the collision stage both ways on `worlds` random worlds whose instances lie in the top left
/// `width` x `height` pixels, checks they agree, and returns how many meetings there were.
fn compare_stages(seed: u64, worlds: usize, width: u64, height: u64) -> usize {
    let mut r = Lcg(seed);
    let p = program();
    let mut meetings = 0;
    for _ in 0..worlds {
        let mut w = world(&mut r, p, width, height);
        // Some motion for the solid rule.
        for h in all(&w) {
            if r.chance(30) {
                let s = r.below(7) as f64 - 3.0;
                w.set(h, Builtin::Hspeed.var(), Value::Real(s)).unwrap();
            }
            let i = w.instance_mut(h);
            (i.xprevious, i.yprevious) = (i.x - 2.0, i.y);
        }
        for _ in 0..r.below(20) {
            mutate(&mut r, &mut w);
        }
        let mut plain = w.clone();
        // A move back to a NaN position traps, in both alike.
        assert_eq!(w.run_collisions(), naive_collisions(&mut plain));
        let (got, want) = (snapshot(&w), snapshot(&plain));
        assert_eq!(got, want);
        meetings += got.0.matches('-').count();
    }
    meetings
}

#[test]
fn the_collision_stage_meets_what_the_plain_loops_meet() {
    // Crowded: lots of meetings, so lots of changes mid-stage.
    let meetings = compare_stages(0xc011_1de5, 150, 64, 64);
    assert!(meetings > 1000, "only {meetings} meetings");
    // Spread over the room: long target lists where most instances are far apart, so the stage jumps
    // over most of each list.
    let meetings = compare_stages(0x5ca7_7e7d, 300, 320, 240);
    assert!(meetings > 50, "only {meetings} meetings");
}
