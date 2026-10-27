//! `fuzz::counts_as_active` against the real game: a level's first frame, before `oLevel` first deactivates
//! what's out of view, doesn't count, and every other frame does.

mod common;

use std::collections::BTreeSet;

use common::{assets, replay};
use game::replay::Playback;
use harness::fuzz::counts_as_active;
use trace::Frame;

/// The ids of every instance, active or deactivated.
fn all(f: &Frame) -> BTreeSet<i32> {
    let active = f.instances.iter().map(|i| i.id);
    active.chain(deactivated(f)).collect()
}

fn deactivated(f: &Frame) -> BTreeSet<i32> {
    f.deactivated.iter().flatten().map(|i| i.id).collect()
}

#[test]
fn a_level_counts_from_olevels_first_cull() {
    // Two custom levels (rLoadLevel). oLevel deactivates the persistent oMusic in the first, which so stays
    // behind (GM8 carries only active persistent instances, 851-3315): nothing comes into the second
    // deactivated.
    let replay = replay("custom-chain-end.txt");
    let mut p = Playback::boot(assets(), &replay).unwrap();
    let mut before = p.game().trace_with(true);
    // Each level load: its frame, how many instances came in deactivated, and every instance before it.
    let mut loads: Vec<(u64, usize, BTreeSet<i32>)> = Vec::new();
    while let Some(frame) = p.step().unwrap() {
        let now = p.game().trace_with(true);
        let level = now.instances.iter().any(|i| i.object == "oLevel");
        if before.room != now.room && level {
            assert!(!counts_as_active(&now), "frame {frame}");
            // Nothing in the new level is culled yet: only what came in deactivated.
            let earlier = all(&before);
            assert!(deactivated(&now).is_subset(&earlier), "frame {frame}");
            loads.push((frame, deactivated(&now).len(), earlier));
        } else {
            assert!(counts_as_active(&now), "frame {frame}");
            // The frame after a level loads, oLevel has culled some of what the level generated.
            if let Some((_, _, earlier)) = loads.last().filter(|(f, _, _)| f + 1 == frame) {
                assert!(!deactivated(&now).is_subset(earlier), "frame {frame}");
            }
        }
        before = now;
    }
    let loads: Vec<(u64, usize)> = loads.iter().map(|&(f, n, _)| (f, n)).collect();
    assert_eq!(loads, [(400, 0), (560, 0)]);
}
