//! Collision-heavy frames: a level-like room of 2,000 instances, mostly 16x16 blocks, where 200 walkers
//! each make four `collision_point` calls and one `collision_rectangle` per step (1,000 queries per
//! tick), and 20 "sight" beams lying on the floors have a collision event with every block.
//!
//! `scripts/cargo run --release -p gm --example collision_bench [frames]`

use gm::*;
use std::sync::Arc;
use std::time::Instant;

const NOONE: i32 = -4;
const BLOCK: ObjectId = ObjectId(0);
const BRICK: ObjectId = ObjectId(1);
const WALKER: ObjectId = ObjectId(2);
const SIGHT: ObjectId = ObjectId(3);

fn walker_step(g: &mut World, cx: &mut Ctx) -> R<()> {
    let block = BLOCK.0 as i32;
    let (x, y, hspeed) = {
        let i = g.instance(cx.this);
        (i.x, i.y, i.hspeed)
    };
    let dir = if hspeed < 0.0 { -1.0 } else { 1.0 };
    // Ground under both feet, a wall ahead (precise), and the ceiling: like the game's isCollision*.
    let ground = g.collision_point(cx, x + 2.0, y + 16.0, block, false, true)? != NOONE
        || g.collision_point(cx, x + 13.0, y + 16.0, block, false, true)? != NOONE;
    let wall = g.collision_point(cx, x + 8.0 + 9.0 * dir, y + 8.0, block, true, true)? != NOONE;
    let ceiling = g.collision_point(cx, x + 8.0, y - 1.0, block, false, true)? != NOONE;
    let crowd = g.collision_rectangle(
        cx,
        x as i32 - 8,
        y as i32,
        x as i32 + 24,
        y as i32 + 15,
        WALKER.0 as i32,
        false,
        true,
    )?;
    if wall || !ground || ceiling || crowd != NOONE && dir > 0.0 {
        g.set(cx.this, Builtin::Hspeed.var(), Value::Real(-hspeed))?;
    }
    Ok(())
}

fn sight_hit(g: &mut World, cx: &mut Ctx) -> R<()> {
    // Write a variable through `World::set`, as generated code does.
    let x = g.instance(cx.other).x;
    g.set(cx.this, Builtin::ImageIndex.var(), Value::Real(x))
}

fn main() {
    let frames = std::env::args()
        .nth(1)
        .map_or(2000, |a| a.parse::<usize>().expect("a number"));
    let mut b = ProgramBuilder::new();
    let block = b.push_object(ObjectInfo {
        sprite: 0,
        ..ObjectInfo::new("block")
    });
    let brick = b.push_object(ObjectInfo {
        sprite: 0,
        parent: Some(block),
        ..ObjectInfo::new("brick")
    });
    let walker = b.push_object(ObjectInfo {
        sprite: 1,
        ..ObjectInfo::new("walker")
    });
    let sight = b.push_object(ObjectInfo {
        sprite: 2,
        ..ObjectInfo::new("sight")
    });
    assert_eq!([block, brick, walker, sight], [BLOCK, BRICK, WALKER, SIGHT]);
    b.event(walker, Event::Step, walker_step);
    b.event(sight, Event::Collision(block), sight_hit);
    let program: &'static Program = Box::leak(Box::new(b.build()));

    // A full 16x16 block, a 16x16 walker with a precise (round) mask, and a 64x8 sight beam.
    let rect = assets::AtlasRect {
        page: 0,
        x: 0,
        y: 0,
        width: 16,
        height: 16,
    };
    let sprite = |name: &str, w: u32, h: u32, solid: &dyn Fn(u32, u32) -> bool| {
        let mut bits = vec![0u8; (w * h).div_ceil(8) as usize];
        for y in 0..h {
            for x in 0..w {
                if solid(x, y) {
                    let i = (y * w + x) as usize;
                    bits[i / 8] |= 1 << (i % 8);
                }
            }
        }
        assets::Sprite {
            name: name.into(),
            width: w,
            height: h,
            origin_x: 0,
            origin_y: 0,
            frames: vec![rect; 1],
            shape: assets::MaskShape::Precise,
            bbox_mode: assets::BboxMode::Automatic,
            alpha_tolerance: 0,
            separate_masks: false,
            masks: vec![assets::Mask {
                bbox: assets::Bbox {
                    left: 0,
                    top: 0,
                    right: w - 1,
                    bottom: h - 1,
                },
                data: assets::MaskData::Bits(bits),
            }],
        }
    };
    let sprites = vec![
        Some(sprite("block", 16, 16, &|_, _| true)),
        Some(sprite("walker", 16, 16, &|x, y| {
            (x as i32 * 2 - 15).pow(2) + (y as i32 * 2 - 15).pow(2) <= 256
        })),
        Some(sprite("sight", 64, 8, &|_, _| true)),
    ];
    // A 1,024 x 1,024 room: floors two tiles thick with gaps every 8 tiles, and walls: 1,780 bricks.
    // Walkers and sights on the floors.
    let mut placed = Vec::new();
    let mut add = |object: ObjectId, x: i32, y: i32| {
        placed.push(assets::Instance {
            id: 100_001 + placed.len() as i32,
            object: object.0 as i32,
            x,
            y,
            ..Default::default()
        })
    };
    let mut bricks = 0;
    'rows: for row in 0..64 {
        for col in 0..64 {
            let floor = (row % 4 == 3 || row % 4 == 0) && col % 8 != 4;
            let wall = col == 0 || col == 63;
            if floor || wall {
                add(BRICK, col * 16, row * 16);
                bricks += 1;
                if bricks == 1780 {
                    break 'rows;
                }
            }
        }
    }
    for i in 0..200 {
        add(WALKER, 32 + (i % 25) * 38, (i / 25 * 4 + 2) * 16);
    }
    for i in 0..20 {
        add(SIGHT, 40 + i * 47, (i % 14 * 4 + 2) * 16 + 12);
    }
    let instances = placed.len();
    let room = assets::Room {
        width: 1024,
        height: 1024,
        speed: 30,
        instances: placed,
        ..Default::default()
    };
    let assets = Assets::new(sprites, vec![Some(room)]);
    let mut world = World::new(program, Arc::new(assets), 0, 200_000, 0);
    world.start_game().unwrap();
    let walkers: Vec<Inst> = world
        .active()
        .filter(|&h| world.instance(h).object == WALKER)
        .collect();
    for h in walkers {
        world
            .set(h, Builtin::Hspeed.var(), Value::Real(1.5))
            .unwrap();
    }

    let start = Instant::now();
    for _ in 0..frames {
        world.step(&[]).unwrap();
    }
    let secs = start.elapsed().as_secs_f64();
    println!(
        "{instances} instances, {frames} frames: {:.0} ticks/s ({:.1} us/tick)",
        frames as f64 / secs,
        secs * 1e6 / frames as f64
    );
    // A checksum of where the walkers ended up, to compare implementations.
    let sum: f64 = world.active().map(|h| world.instance(h).x).sum();
    println!("checksum {sum}");

    // A clone drops the spatial index (savestates stay cheap); its first query rebuilds it.
    let n = 200;
    let start = Instant::now();
    for _ in 0..n {
        let w = world.clone();
        let h = w.active().next().unwrap();
        let cx = Ctx::event(h, h, Event::Step, w.instance(h).object);
        w.collision_point(&cx, 100.0, 100.0, BLOCK.0 as i32, false, false)
            .unwrap();
    }
    let secs = start.elapsed().as_secs_f64();
    println!("clone + first query: {:.1} us", secs * 1e6 / n as f64);
}
