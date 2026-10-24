//! Divergences the random-input fuzzer found (851-3183), replayed on the port and checked against values
//! pinned from the oracle's trace, so `scripts/check` guards them without the oracle.

mod common;

use common::{assets, replay};
use game::replay::Playback;

/// The replay's RNG seed after each of `frames`, playing it from the start.
fn seeds_after(name: &str, frames: &[u64]) -> Vec<(u64, Option<String>, Option<i32>)> {
    let replay = replay(name);
    let mut p = Playback::boot(assets(), &replay).unwrap();
    let mut out = Vec::new();
    while let Some(frame) = p.step().unwrap() {
        if frames.contains(&frame) {
            let t = p.game().trace();
            out.push((frame, t.room_name, t.seed));
        }
        if frame >= *frames.iter().max().unwrap() {
            break;
        }
    }
    out
}

#[test]
fn a_game_over_draws_nothing_after_the_room_change() {
    // Attack on the game over screen makes action 2 of oPlayer1's step go to rHighscores on frame 1333.
    // GM8 ends the event there, so action 4's random draw doesn't happen and the high score room's bricks
    // (`oBrick` draws twice each when created) start from the frame's first draw. The seeds are the
    // oracle's (`tools/oracle/run`, the `seed` of those frames' lines); the port that ran action 4 had
    // 1672352712 on frame 1333, one draw further along.
    assert_eq!(
        seeds_after("fuzz-room-change-ends-event.txt", &[1332, 1333]),
        [
            (1332, Some("rLevel2".into()), Some(-793257433)),
            (1333, Some("rHighscores".into()), Some(-657887653)),
        ]
    );
}
