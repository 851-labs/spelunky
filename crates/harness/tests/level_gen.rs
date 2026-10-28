//! Level generation on the real game, across seeds: what a single replay's story can't pin, since it's about
//! the levels that are never made.

mod common;

use std::collections::HashSet;

use common::assets;
use game::replay::Playback;
use gm::State;
use harness::fuzz::Start;
use harness::route::is_a;

/// How many open tiles lie in front of the arrow trap at (`x`, `y`) before a solid one, looking `dx` (-16 or
/// 16) along its row, up to 4.
fn open_in_front(solid: &HashSet<(i32, i32)>, (x, y): (i32, i32), dx: i32) -> i32 {
    (1..=4)
        .find(|n| solid.contains(&(x + dx * n, y)))
        .map_or(4, |n| n - 1)
}

#[test]
fn a_right_facing_arrow_trap_never_has_only_two_open_blocks_in_front() {
    // scrEntityGen makes a block a left-facing trap if `collision_rectangle(x-32, y, x-1, y+15, oSolid)` is
    // clear, two blocks, but a right-facing one only if `collision_rectangle(x+16, y, x+48, y+15, oSolid)`
    // is, which takes in the first pixel column of a third (docs/quirks.md §1). So a left-facing trap can
    // face a wall two blocks off and a right-facing one never does.
    let assets = assets();
    let (mut left, mut left_two, mut right, mut right_two) = (0, 0, 0, 0);
    for seed in 0..32 {
        // Mines level 1, through the START door: it loads on frame 565.
        let replay = Start::Mines.entry(seed, 566);
        let mut p = Playback::boot(assets.clone(), &replay).unwrap();
        while p.step().unwrap().is_some() {}
        let w = p.game().world();
        assert_eq!(
            w.assets().room_name(w.room().id),
            Some("rLevel"),
            "seed {seed}"
        );
        let prog = w.program();
        let mut solid = HashSet::new();
        let mut traps = Vec::new();
        for h in w.active().chain(w.with_state(State::Inactive)) {
            let i = w.instance(h);
            let at = (i.x as i32, i.y as i32);
            if is_a(prog, i.object, "oSolid") {
                solid.insert(at);
            }
            let name = prog.object(i.object).map_or("", |o| o.name.as_str());
            if name == "oArrowTrapLeft" || name == "oArrowTrapRight" {
                traps.push((name == "oArrowTrapLeft", at));
            }
        }
        for (faces_left, at) in traps {
            if faces_left {
                left += 1;
                left_two += i32::from(open_in_front(&solid, at, -16) == 2);
            } else {
                right += 1;
                right_two += i32::from(open_in_front(&solid, at, 16) == 2);
            }
        }
    }
    assert_eq!(
        (left, left_two, right, right_two),
        (48, 8, 34, 0),
        "left-facing traps (with two open blocks in front), right-facing ones"
    );
}
