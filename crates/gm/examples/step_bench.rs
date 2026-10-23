//! Frame-loop overhead: a world of 2,000 instances whose events do nothing, stepped headless.
//!
//! `scripts/cargo run --release -p gm --example step_bench [instances] [frames]`

use gm::*;
use std::sync::Arc;
use std::time::Instant;

fn nop(_: &mut World, _: &mut Ctx) -> R<()> {
    Ok(())
}

fn main() {
    let mut args = std::env::args()
        .skip(1)
        .map(|a| a.parse::<usize>().expect("a number"));
    let instances = args.next().unwrap_or(2000);
    let frames = args.next().unwrap_or(5000);

    // 40 objects in parent chains of four, like Spelunky's oDrawnSprite tree. Every object has begin step,
    // step, end step, an alarm and animation end; half have a draw event.
    let mut b = ProgramBuilder::new();
    let mut objects = Vec::new();
    for i in 0..40 {
        let parent = (i % 4 != 0).then(|| objects[i - 1]);
        let o = b.push_object(ObjectInfo {
            parent,
            sprite: 0,
            depth: (i % 7) as f64,
            ..ObjectInfo::new(format!("o{i}"))
        });
        for e in [
            Event::BeginStep,
            Event::Step,
            Event::EndStep,
            Event::Alarm(0),
            Event::ANIMATION_END,
        ] {
            b.event(o, e, nop);
        }
        if i % 2 == 0 {
            b.event(o, Event::Draw, nop);
        }
        objects.push(o);
    }
    let program: &'static Program = Box::leak(Box::new(b.build()));
    let rect = assets::AtlasRect {
        page: 0,
        x: 0,
        y: 0,
        width: 16,
        height: 16,
    };
    let sprite = assets::Sprite {
        name: "s".into(),
        width: 16,
        height: 16,
        origin_x: 0,
        origin_y: 0,
        frames: vec![rect; 4],
        shape: assets::MaskShape::Rectangle,
        bbox_mode: assets::BboxMode::FullImage,
        alpha_tolerance: 0,
        separate_masks: false,
        masks: Vec::new(),
    };
    let placed = (0..instances)
        .map(|i| assets::Instance {
            id: 100_001 + i as i32,
            object: objects[i % objects.len()].0 as i32,
            x: (i % 64 * 16) as i32,
            y: (i / 64 * 16) as i32,
            ..Default::default()
        })
        .collect();
    let room = assets::Room {
        width: 1024,
        height: 1024,
        speed: 30,
        instances: placed,
        ..Default::default()
    };
    let assets = Assets::new(vec![Some(sprite)], vec![Some(room)]);
    let mut world = World::new(program, Arc::new(assets), 0, 200_000, 0);
    world.start_game().unwrap();

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

    let start = Instant::now();
    let n = 1000;
    let clones: Vec<World> = (0..n).map(|_| world.clone()).collect();
    let secs = start.elapsed().as_secs_f64();
    drop(clones);
    println!("clone: {:.1} us", secs * 1e6 / n as f64);
}
