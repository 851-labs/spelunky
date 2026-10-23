//! Collision masks, the collision built-ins and the collision event stage, with real Spelunky sprites
//! where they show the rule (`sBrick`, `sStandLeft`, `sRock`, `sArrowRight`, `sRunLeft`).

mod common;
#[path = "common/spelunky.rs"]
mod spelunky;

use common::*;
use gm::*;
use std::sync::Arc;

const NOONE: i32 = -4;
const X: Var = Builtin::X.var();
const Y: Var = Builtin::Y.var();

const SPRITES: [&str; 5] = ["sBrick", "sStandLeft", "sRock", "sArrowRight", "sRunLeft"];

/// Objects 0 to 4 use the sprites above, in that order: brick, player, rock, arrow, runner. Object 5,
/// `block`, is the brick's parent; 6, `ctl`, has no sprite.
fn program() -> &'static Program {
    build(|b| {
        let block = ObjectId(5);
        for (name, sprite) in ["brick", "player", "rock", "arrow", "runner"]
            .into_iter()
            .zip(SPRITES)
        {
            b.push_object(ObjectInfo {
                sprite: spelunky::sprite_id(sprite),
                parent: (name == "brick").then_some(block),
                ..ObjectInfo::new(name)
            });
        }
        assert_eq!(b.push_object(ObjectInfo::new("block")), block);
        b.push_object(ObjectInfo::new("ctl"));
    })
}

const BRICK: ObjectId = ObjectId(0);
const PLAYER: ObjectId = ObjectId(1);
const ROCK: ObjectId = ObjectId(2);
const ARROW: ObjectId = ObjectId(3);
const RUNNER: ObjectId = ObjectId(4);
const BLOCK: ObjectId = ObjectId(5);
const CTL: ObjectId = ObjectId(6);

/// A room with these instances (ids from [`FIRST_ID`]), then an instance of the program's last object
/// (`ctl`, without a sprite) to run queries as.
fn room_with(p: &'static Program, assets: Assets, placed: &[(ObjectId, f64, f64)]) -> World {
    room_from(p, assets, FIRST_ID, placed)
}

/// [`room_with`], numbering the instances from `first_id`.
fn room_from(
    p: &'static Program,
    assets: Assets,
    first_id: i32,
    placed: &[(ObjectId, f64, f64)],
) -> World {
    let mut w = World::new(p, Arc::new(assets), 1, LAST_ID);
    let ctl = ObjectId(p.object_slots() as u32 - 1);
    let instances = placed
        .iter()
        .chain([(ctl, 0.0, 0.0)].iter())
        .enumerate()
        .map(|(i, &(object, x, y))| PlacedInstance {
            id: first_id + i as i32,
            object,
            x,
            y,
        })
        .collect();
    let room = RoomState {
        id: 0,
        width: 640,
        height: 480,
        speed: 30,
    };
    w.start_room(RoomStart { room, instances }).unwrap();
    w
}

fn assets() -> Assets {
    Assets::new(spelunky::sprites(&SPRITES), Vec::new())
}

fn room(placed: &[(ObjectId, f64, f64)]) -> World {
    room_with(program(), assets(), placed)
}

/// The instance placed `n`th (from 1).
fn nth(g: &World, n: i32) -> Inst {
    g.find(FIRST_ID + n - 1).unwrap()
}

/// A context running as the `ctl` instance, placed last.
fn as_ctl(g: &World) -> Ctx {
    let h = g.active().last().unwrap();
    assert_eq!(g.instance(h).object, CTL);
    ctx(g, h)
}

/// `collision_point` against `obj` from `ctl`, as the 1-based placement number it finds (0 for noone).
fn point(g: &World, x: f64, y: f64, obj: i32, prec: bool) -> i32 {
    let cx = as_ctl(g);
    number(g.collision_point(&cx, x, y, obj, prec, false))
}

fn number(id: i32) -> i32 {
    if id == NOONE { 0 } else { id - FIRST_ID + 1 }
}

fn bbox(g: &World, n: i32) -> (i32, i32, i32, i32) {
    let b = g.bbox(nth(g, n));
    (b.left, b.top, b.right, b.bottom)
}

fn set(g: &mut World, n: i32, b: Builtin, v: f64) {
    let h = nth(g, n);
    g.set(h, b.var(), Value::Real(v)).unwrap();
}

#[test]
fn bounding_boxes_include_both_edges_and_compare_reals() {
    let g = room(&[(BRICK, 32.0, 48.0)]);
    let b = BRICK.0 as i32;
    assert_eq!(bbox(&g, 1), (32, 48, 47, 63));
    for (x, y, hit) in [
        (32.0, 48.0, true),
        (47.0, 63.0, true),
        (31.9, 50.0, false),
        // The point isn't rounded for the box test: 47.5 is past the right edge.
        (47.5, 50.0, false),
        (40.0, 63.01, false),
    ] {
        assert_eq!(point(&g, x, y, b, false) == 1, hit, "({x}, {y})");
    }
    // The precise test rounds the point half to even, after the box test passed.
    assert_eq!(point(&g, 46.5, 50.0, b, true), 1);
    // The parent finds its children.
    assert_eq!(point(&g, 40.0, 50.0, BLOCK.0 as i32, false), 1);
    assert_eq!(point(&g, 40.0, 50.0, PLAYER.0 as i32, false), 0);
}

#[test]
fn precise_tests_read_the_mask_and_round_positions() {
    // sRock: 8x8, origin (4, 4), mask box (1, 1)-(7, 7). Its top row is `..####..`.
    let mut g = room(&[(ROCK, 100.0, 100.0)]);
    let r = ROCK.0 as i32;
    assert_eq!(bbox(&g, 1), (97, 97, 103, 103));
    // Room x = 100 - 4 + mask x. The box's corner is transparent.
    assert_eq!(point(&g, 97.0, 97.0, r, false), 1);
    assert_eq!(point(&g, 97.0, 97.0, r, true), 0);
    assert_eq!(point(&g, 98.0, 97.0, r, true), 1);
    // 97.5 rounds to 98 (half to even), 98.5 to 98 as well.
    assert_eq!(point(&g, 97.5, 97.0, r, true), 1);
    assert_eq!(point(&g, 98.5, 97.2, r, true), 1);
    assert_eq!(point(&g, 101.0, 97.0, r, true), 1);
    assert_eq!(point(&g, 102.0, 97.0, r, true), 0);
    // The instance's position rounds too: at x = 100.5 the mask is placed from 100, but the box from
    // 100.5 - 4 + 1 = 97.5, which rounds to 98.
    set(&mut g, 1, Builtin::X, 100.5);
    assert_eq!(bbox(&g, 1), (98, 97, 104, 103));
    assert_eq!(point(&g, 98.0, 97.0, r, true), 1);
    assert_eq!(point(&g, 102.0, 97.0, r, true), 0);
    // -1 is false for prec (the game passes it 58 times): that's the caller converting with `is_truthy`.
    assert!(!Value::Real(-1.0).is_truthy());
}

#[test]
fn per_frame_masks_follow_image_index() {
    // sRunLeft: frame 0's mask spans columns 1-14, frame 1's 1-13; the box is their union.
    let mut g = room(&[(RUNNER, 100.0, 100.0)]);
    let r = RUNNER.0 as i32;
    assert_eq!(bbox(&g, 1), (93, 92, 106, 107));
    assert_eq!(point(&g, 106.0, 100.0, r, true), 1);
    set(&mut g, 1, Builtin::ImageIndex, 1.7);
    assert_eq!(point(&g, 106.0, 100.0, r, false), 1);
    assert_eq!(point(&g, 106.0, 100.0, r, true), 0);
    // Frame 7 is frame 1 (modulo the frame count).
    set(&mut g, 1, Builtin::ImageIndex, 7.0);
    assert_eq!(point(&g, 106.0, 100.0, r, true), 0);
}

#[test]
fn flipped_scaled_and_masked_instances() {
    // sStandLeft: 16x16, origin (8, 8), mask columns 2-13. Flipped, the box is one pixel wider on the
    // left than the mask (the quirk in `Instance::bbox_at`), and the mask is mirrored about x.
    let mut g = room(&[(PLAYER, 100.0, 100.0), (ROCK, 200.0, 200.0)]);
    let p = PLAYER.0 as i32;
    assert_eq!(bbox(&g, 1), (94, 92, 105, 107));
    set(&mut g, 1, Builtin::ImageXscale, -1.0);
    assert_eq!(bbox(&g, 1), (93, 92, 106, 107));
    // Mask x = floor(8 + (x - 100) / -1).
    assert_eq!(point(&g, 93.0, 100.0, p, false), 1);
    assert_eq!(point(&g, 93.0, 100.0, p, true), 0);
    assert_eq!(point(&g, 94.0, 100.0, p, true), 0);
    assert_eq!(point(&g, 95.0, 100.0, p, true), 1);
    assert_eq!(point(&g, 106.0, 100.0, p, true), 1);

    // Scaled 2x: mask x = floor(4 + (x - 200) / 2).
    let r = ROCK.0 as i32;
    set(&mut g, 2, Builtin::ImageXscale, 2.0);
    assert_eq!(bbox(&g, 2), (194, 197, 207, 203));
    assert_eq!(point(&g, 195.0, 197.0, r, true), 0);
    assert_eq!(point(&g, 196.0, 197.0, r, true), 2);
    assert_eq!(point(&g, 203.0, 197.0, r, true), 2);
    assert_eq!(point(&g, 204.0, 197.0, r, true), 0);

    // mask_index replaces the sprite's mask: the rock now collides as a brick (origin (0, 0)).
    let brick = spelunky::sprite_id("sBrick");
    set(&mut g, 2, Builtin::ImageXscale, 1.0);
    set(&mut g, 2, Builtin::MaskIndex, f64::from(brick));
    assert_eq!(bbox(&g, 2), (200, 200, 215, 215));
    assert_eq!(point(&g, 215.0, 215.0, r, true), 2);
    assert_eq!(point(&g, 199.0, 200.0, r, false), 0);
}

#[test]
fn rotated_instances_rotate_box_and_mask() {
    // sArrowRight: 8x8, origin (4, 4), mask rows 3-7, pointing right. At 90 degrees it points up.
    let mut g = room(&[(ARROW, 100.0, 100.0)]);
    let a = ARROW.0 as i32;
    set(&mut g, 1, Builtin::ImageAngle, 90.0);
    assert_eq!(bbox(&g, 1), (99, 97, 103, 104));
    // Mask (x, y) = floor(4 + dx cos - dy sin, 4 + dx sin + dy cos), with sin 1 and cos 6.1e-17 (which
    // vanishes in these sums). The tip, mask (7, 4), is at (100, 97).
    assert_eq!(point(&g, 100.0, 97.0, a, true), 1);
    // The notch between the fletching, mask (2, 3), is in the box but transparent; (1, 3) is solid.
    assert_eq!(point(&g, 99.0, 102.0, a, false), 1);
    assert_eq!(point(&g, 99.0, 102.0, a, true), 0);
    assert_eq!(point(&g, 99.0, 103.0, a, true), 1);
    // The tail: mask (0, 4) is solid, (0, 7) isn't.
    assert_eq!(point(&g, 100.0, 104.0, a, true), 1);
    assert_eq!(point(&g, 103.0, 104.0, a, true), 0);
}

#[test]
fn queries_return_the_first_match_in_list_order() {
    // Three bricks on top of each other, and a rock.
    let mut g = room(&[
        (BRICK, 0.0, 0.0),
        (ROCK, 4.0, 4.0),
        (BRICK, 0.0, 0.0),
        (BRICK, 0.0, 0.0),
    ]);
    let cx = as_ctl(&g);
    let b = BRICK.0 as i32;
    assert_eq!(point(&g, 4.0, 4.0, b, false), 1);
    assert_eq!(point(&g, 4.0, 4.0, -3, false), 1);
    // Deactivated instances don't exist for collisions.
    g.instance_deactivate_object(&cx, FIRST_ID);
    assert_eq!(point(&g, 4.0, 4.0, b, false), 3);
    assert_eq!(point(&g, 4.0, 4.0, -3, false), 2);
    // An instance id finds only that instance, if active.
    assert_eq!(point(&g, 4.0, 4.0, FIRST_ID + 3, false), 4);
    assert_eq!(point(&g, 4.0, 4.0, FIRST_ID, false), 0);
    g.instance_activate_object(&cx, FIRST_ID);
    // Drawing sorts `all` by depth; the per-object lists follow at the next rebuild.
    set(&mut g, 4, Builtin::Depth, 5.0);
    g.redraw().unwrap();
    assert_eq!(point(&g, 4.0, 4.0, -3, false), 4);
    assert_eq!(point(&g, 4.0, 4.0, b, false), 1);
    g.instance_activate_all();
    assert_eq!(point(&g, 4.0, 4.0, b, false), 4);
    // `self`, `other` and `noone` find nothing, even where the caller is.
    let me = nth(&g, 1);
    let mine = ctx(&g, me);
    assert_eq!(g.collision_point(&mine, 4.0, 4.0, -1, false, false), NOONE);
    assert_eq!(g.collision_point(&mine, 4.0, 4.0, -2, false, false), NOONE);
    // `notme` skips the caller.
    assert_eq!(
        number(g.collision_point(&mine, 4.0, 4.0, b, false, false)),
        4
    );
    let fourth = ctx(&g, nth(&g, 4));
    assert_eq!(
        number(g.collision_point(&fourth, 4.0, 4.0, b, false, true)),
        1
    );
    // instance_position is precise and has no `notme`.
    assert_eq!(number(g.instance_position(4.0, 4.0, -3)), 4);
    assert_eq!(number(g.instance_position(1.0, 1.0, ROCK.0 as i32)), 0);
    assert_eq!(number(g.instance_position(4.0, 2.0, ROCK.0 as i32)), 2);
}

#[test]
fn rectangles_take_corners_in_any_order() {
    let g = room(&[(ROCK, 100.0, 100.0)]);
    let cx = as_ctl(&g);
    let r = ROCK.0 as i32;
    let rect = |g: &World, x1, y1, x2, y2, prec| {
        number(g.collision_rectangle(&cx, x1, y1, x2, y2, r, prec, false))
    };
    // Touching the box's corner pixel (97, 97), which is transparent.
    assert_eq!(rect(&g, 90, 90, 97, 97, false), 1);
    assert_eq!(rect(&g, 97, 97, 90, 90, false), 1);
    assert_eq!(rect(&g, 90, 90, 97, 97, true), 0);
    assert_eq!(rect(&g, 90, 90, 96, 110, false), 0);
    // Reaching (98, 97), which is solid.
    assert_eq!(rect(&g, 98, 90, 90, 97, true), 1);
}

#[test]
fn lines_use_gm8s_own_rules() {
    let g = room(&[(ROCK, 100.0, 100.0)]);
    let cx = as_ctl(&g);
    let r = ROCK.0 as i32;
    let line = |g: &World, x1, y1, x2, y2, prec| {
        number(g.collision_line(&cx, x1, y1, x2, y2, r, prec, false))
    };
    // Along the top row, from the left: the box is hit; the mask from x = 98.
    assert_eq!(line(&g, 80.0, 97.0, 97.0, 97.0, false), 1);
    assert_eq!(line(&g, 80.0, 97.0, 97.0, 97.0, true), 0);
    assert_eq!(line(&g, 80.0, 97.0, 98.0, 97.0, true), 1);
    // The box counts as one pixel wider for lines: 103.5 is past its right edge (103), but a line
    // starting there still meets it unless precise.
    assert_eq!(line(&g, 103.5, 90.0, 103.5, 110.0, false), 1);
    assert_eq!(line(&g, 104.0, 90.0, 104.0, 110.0, false), 0);
    // A vertical line through the solid middle.
    assert_eq!(line(&g, 100.0, 90.0, 100.0, 110.0, true), 1);
}

#[test]
fn place_queries_test_self_at_another_position() {
    // A player (box 94-105 x 92-107) and a brick to its right at x = 110.
    let g = room(&[
        (PLAYER, 100.0, 100.0),
        (BRICK, 110.0, 92.0),
        (PLAYER, 100.0, 100.0),
    ]);
    let me = ctx(&g, nth(&g, 1));
    let b = BRICK.0 as i32;
    assert_eq!(number(g.instance_place(&me, 100.0, 100.0, b)), 0);
    assert_eq!(number(g.instance_place(&me, 104.0, 100.0, b)), 0);
    // At 104.5 the box reaches 110 (109.5 rounds to even), but the mask is placed from 104 and ends at
    // 109. At 105 both reach the brick.
    assert_eq!(number(g.instance_place(&me, 104.5, 100.0, b)), 0);
    assert_eq!(number(g.instance_place(&me, 105.0, 100.0, b)), 2);
    assert_eq!(number(g.instance_place(&me, 105.0, 100.0, -3)), 2);
    // Never itself; another instance of its object is found.
    assert_eq!(number(g.instance_place(&me, 100.0, 100.0, -3)), 3);
    assert_eq!(number(g.instance_place(&me, 100.0, 100.0, -2)), 0);
    // The instance didn't move.
    assert_eq!(g.get(me.this, X).unwrap(), Value::Real(100.0));
    assert!(g.place_meeting(&me, 105.0, 100.0, b));
    assert!(!g.place_meeting(&me, 100.0, 100.0, b));
    assert!(!g.place_meeting(&me, 100.0, 100.0, -1));
    // place_meeting tests `other` (here the brick), unlike the other queries.
    let mut with_other = me.clone();
    with_other.other = nth(&g, 2);
    assert!(g.place_meeting(&with_other, 105.0, 100.0, -2));
    assert!(!g.place_meeting(&with_other, 100.0, 100.0, -2));
    // Instances without a mask never collide with anything.
    let ctl = as_ctl(&g);
    assert_eq!(number(g.instance_place(&ctl, 100.0, 100.0, -3)), 0);
}

#[test]
fn distances_are_between_bounding_boxes() {
    let g = room(&[
        (PLAYER, 100.0, 100.0),
        (BRICK, 120.0, 92.0),
        (BRICK, 120.0, 120.0),
    ]);
    let me = ctx(&g, nth(&g, 1));
    let b = BRICK.0 as i32;
    // Player box 94-105 x 92-107. The first brick is 15 pixels right; the second 15 right and 13 down.
    assert_eq!(g.distance_to_object(&me, FIRST_ID + 1), 15.0);
    assert_eq!(
        g.distance_to_object(&me, FIRST_ID + 2),
        (15.0f64 * 15.0 + 13.0 * 13.0).sqrt()
    );
    assert_eq!(g.distance_to_object(&me, b), 15.0);
    assert_eq!(g.distance_to_object(&me, BLOCK.0 as i32), 15.0);
    assert_eq!(g.distance_to_object(&me, -1), 1_000_000.0);
    assert_eq!(g.distance_to_object(&me, PLAYER.0 as i32), 1_000_000.0);
    assert_eq!(g.distance_to_object(&me, FIRST_ID), 1_000_000.0);
    // `all` includes the mask-less ctl, whose box is the pixel (-100000, -100000).
    assert_eq!(g.distance_to_object(&me, -3), 15.0);
    let ctl = as_ctl(&g);
    let far = g.distance_to_object(&ctl, FIRST_ID);
    assert_eq!(
        far,
        (100_094.0f64 * 100_094.0 + 100_092.0 * 100_092.0).sqrt()
    );
    assert_eq!(g.distance_to_point(&me, 110.0, 100.0), 5.0);
    assert_eq!(g.distance_to_point(&me, 100.0, 100.0), 0.0);
    assert_eq!(
        g.distance_to_point(&me, 90.5, 88.0),
        (3.5f64 * 3.5 + 16.0).sqrt()
    );
}

#[test]
fn distance_to_object_reads_100000_as_an_object_and_measures_a_deactivated_other() {
    // The brick is instance 100000: collision queries find it by id, distance_to_object looks for
    // object 100000 (there is none).
    let mut g = room_from(
        program(),
        assets(),
        FIRST_ID - 1,
        &[(BRICK, 120.0, 92.0), (PLAYER, 100.0, 100.0)],
    );
    let (brick, player) = (g.find(FIRST_ID - 1).unwrap(), g.find(FIRST_ID).unwrap());
    let mut me = ctx(&g, player);
    assert_eq!(
        g.collision_point(&me, 120.0, 92.0, FIRST_ID - 1, false, false),
        FIRST_ID - 1
    );
    assert_eq!(g.distance_to_object(&me, FIRST_ID - 1), 1_000_000.0);
    // `other` is measured even when deactivated; by id it isn't found.
    g.instance_deactivate_object(&me, FIRST_ID - 1);
    me.other = brick;
    assert_eq!(g.distance_to_object(&me, -2), 15.0);
    assert_eq!(g.distance_to_object(&me, BRICK.0 as i32), 1_000_000.0);
}

#[test]
fn distance_gaps_wrap_in_32_bits() {
    // Boxes 3e9 pixels apart: the i32 gap wraps negative, and as the only nonzero axis it is the distance,
    // so it beats a brick 85 pixels away.
    let g = room(&[
        (BRICK, 2e9, 0.0),
        (BRICK, 2e9 + 100.0, 0.0),
        (BRICK, -1e9, 0.0),
    ]);
    let me = ctx(&g, nth(&g, 1));
    let wrapped = f64::from((2_000_000_000i32).wrapping_sub(-1_000_000_000 + 15));
    assert!(wrapped < 0.0);
    assert_eq!(g.distance_to_object(&me, FIRST_ID + 1), 85.0);
    assert_eq!(g.distance_to_object(&me, FIRST_ID + 2), wrapped);
    assert_eq!(g.distance_to_object(&me, BRICK.0 as i32), wrapped);
}

#[test]
fn move_snap_rounds_half_to_even() {
    let mut g = room(&[(BRICK, 13.5, 20.0)]);
    let me = ctx(&g, nth(&g, 1));
    g.move_snap(&me, 16.0, 8.0);
    // 13.5 / 16 rounds to 1; 20 / 8 = 2.5 rounds to 2.
    assert_eq!(g.get(me.this, X).unwrap(), Value::Real(16.0));
    assert_eq!(g.get(me.this, Y).unwrap(), Value::Real(16.0));
    g.move_snap(&me, 0.0, -1.0);
    assert_eq!(g.get(me.this, X).unwrap(), Value::Real(16.0));
}

// The collision event stage, with synthetic 8x8 sprites.

/// Logs `me<-other@x` and returns.
fn hit(g: &mut World, cx: &mut Ctx) -> R<()> {
    let x = g.get(cx.this, X)?.to_real();
    let s = format!("{}<-{}@{x}", tag(g, cx.this), tag(g, cx.other));
    log(g, &s);
    Ok(())
}

fn hit_and_destroy_other(g: &mut World, cx: &mut Ctx) -> R<()> {
    hit(g, cx)?;
    g.destroy(cx.other)
}

fn hit_and_destroy_self(g: &mut World, cx: &mut Ctx) -> R<()> {
    hit(g, cx)?;
    g.destroy(cx.this)
}

fn synthetic(placed: &[(ObjectId, f64, f64)], f: impl FnOnce(&mut ProgramBuilder)) -> World {
    let p = build(|b| {
        f(b);
        b.push_object(ObjectInfo::new("ctl"));
    });
    room_with(
        p,
        Assets::new(vec![Some(sprite("s", 1, 8))], Vec::new()),
        placed,
    )
}

fn with_sprite(name: &str) -> ObjectInfo {
    ObjectInfo {
        sprite: 0,
        ..ObjectInfo::new(name)
    }
}

#[test]
fn collision_events_run_per_instance_across_its_targets_both_ways() {
    let [a, b, c] = [0, 1, 2].map(ObjectId);
    let mut g = synthetic(
        &[
            (c, 4.0, 0.0),
            (a, 0.0, 0.0),
            (b, 2.0, 0.0),
            (a, 100.0, 0.0),
            (a, 6.0, 0.0),
        ],
        |p| {
            for name in ["a", "b", "c"] {
                p.push_object(with_sprite(name));
            }
            // c defines the a-c event, so the pair is (a, c) and a's instances lead; a has none for c.
            p.event(a, Event::Collision(b), hit)
                .event(b, Event::Collision(a), hit)
                .event(c, Event::Collision(a), hit);
        },
    );
    step(&mut g);
    // The pairs are a with [b, c]. For each instance of a (a2, a4, a5), each of those objects' instances:
    // a2 meets b3 (both events run, a2's first), then c1 (only c has an event); a4 is too far.
    assert_eq!(
        take_log(&mut g),
        "a2<-b3@0 b3<-a2@2 c1<-a2@4 a5<-b3@6 b3<-a5@2 c1<-a5@4"
    );
}

#[test]
fn a_destroyed_instance_meets_nothing_more() {
    let [a, b] = [0, 1].map(ObjectId);
    let mut g = synthetic(
        &[(a, 0.0, 0.0), (b, 2.0, 0.0), (b, 4.0, 0.0), (a, 1.0, 0.0)],
        |p| {
            p.push_object(with_sprite("a"));
            p.push_object(with_sprite("b"));
            p.event(a, Event::Collision(b), hit_and_destroy_self).event(
                b,
                Event::Collision(a),
                hit_and_destroy_other,
            );
        },
    );
    step(&mut g);
    // a1 destroys itself meeting b2; b2's event still runs (and destroys a1 again), then a1 is done and
    // never meets b3. The same goes for a4.
    assert_eq!(take_log(&mut g), "a1<-b2@0 b2<-a1@2 a4<-b2@1 b2<-a4@2");
}

#[test]
fn solids_move_back_before_the_events_and_on_again_after() {
    let [mover, wall] = [0, 1].map(ObjectId);
    let mut g = synthetic(&[(mover, 0.0, 0.0), (wall, 14.0, 0.0)], |p| {
        p.push_object(with_sprite("mover"));
        p.push_object(ObjectInfo {
            solid: true,
            ..with_sprite("wall")
        });
        p.event(mover, Event::Collision(wall), hit);
    });
    let m = nth(&g, 1);
    g.set(m, Builtin::Hspeed.var(), Value::Real(3.0)).unwrap();
    step(&mut g);
    step(&mut g);
    assert_eq!(take_log(&mut g), "");
    // Third frame: 6 -> 9 meets the wall (14-21). It goes back to 6 for the event, then on to 9 again,
    // still colliding, so back to 6.
    step(&mut g);
    assert_eq!(take_log(&mut g), "mover1<-wall2@6");
    assert_eq!(g.get(m, X).unwrap(), Value::Real(6.0));
    // Stopped by the wall, it meets it every frame without getting further.
    step(&mut g);
    assert_eq!(take_log(&mut g), "mover1<-wall2@6");
    assert_eq!(g.get(m, X).unwrap(), Value::Real(6.0));
}

#[test]
fn solids_keep_the_reapplied_move_when_free() {
    fn bounce(g: &mut World, cx: &mut Ctx) -> R<()> {
        hit(g, cx)?;
        g.set(cx.this, Builtin::Hspeed.var(), Value::Real(-3.0))
    }
    let [mover, wall] = [0, 1].map(ObjectId);
    let mut g = synthetic(&[(mover, 6.0, 0.0), (wall, 14.0, 0.0)], |p| {
        p.push_object(with_sprite("mover"));
        p.push_object(ObjectInfo {
            solid: true,
            ..with_sprite("wall")
        });
        p.event(mover, Event::Collision(wall), bounce);
    });
    let m = nth(&g, 1);
    g.set(m, Builtin::Hspeed.var(), Value::Real(3.0)).unwrap();
    step(&mut g);
    // Back to 6 for the event, which reverses; the re-applied move takes it to 3, free of the wall.
    assert_eq!(take_log(&mut g), "mover1<-wall2@6");
    assert_eq!(g.get(m, X).unwrap(), Value::Real(3.0));
}
