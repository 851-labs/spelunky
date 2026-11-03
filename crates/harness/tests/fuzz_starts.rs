//! The fuzzer's starts partway through committed replays (`harness::fuzz::FROM_REPLAYS`, 851-3793): each
//! one's frames are in the room it's there for, so a soak run from it fuzzes there, and a run from one plays
//! its replay up to there.

mod common;

use common::{assets, replay};
use game::replay::Playback;
use harness::fuzz::{FROM_REPLAYS, Start, fuzz_replay};

#[test]
fn every_replay_start_is_in_its_room_throughout() {
    // Each replay played once, for all its starts.
    let mut replays: Vec<&str> = FROM_REPLAYS.iter().map(|f| f.replay).collect();
    replays.sort();
    replays.dedup();
    for name in replays {
        let starts: Vec<_> = FROM_REPLAYS.iter().filter(|f| f.replay == name).collect();
        let replay = replay(&format!("{name}.txt"));
        let end = starts.iter().map(|f| f.frames.1).max().unwrap();
        assert!(end <= replay.len(), "{name}: {end} of {}", replay.len());
        let mut p = Playback::boot(assets(), &replay).unwrap();
        while let Some(frame) = p.step().unwrap() {
            for from in starts
                .iter()
                .filter(|f| (f.frames.0..f.frames.1).contains(&frame))
            {
                let room = harness::route::room_name(p.game());
                assert_eq!(room, from.room, "{} at {frame}", from.name);
            }
            if frame + 1 >= end {
                break;
            }
        }
        for from in &starts {
            let (lo, hi) = from.frames;
            assert!(lo < hi, "{}", from.name);
            for seed in [0, 1, 7, -3, i32::MAX] {
                assert!(
                    (lo..hi).contains(&from.frame(seed)),
                    "{} seed {seed}",
                    from.name
                );
            }
        }
    }
}

#[test]
fn a_run_from_a_replay_plays_its_input_up_to_there_and_fuzzes_from_there() {
    let from = FROM_REPLAYS.iter().find(|f| f.name == "custom").unwrap();
    let start = Start::From(from);
    let original = replay(&format!("{}.txt", from.replay));
    let fuzzed = fuzz_replay(assets(), start, 4, 1, 120, None).unwrap();
    let at = from.frame(4);
    assert_eq!(start.fuzz_start(4), at + 1);
    assert_eq!(fuzzed.header, original.header);
    assert_eq!(fuzzed.len(), at + 1 + 120);
    let before = |r: &trace::Replay| {
        r.events()
            .iter()
            .filter(|(f, _)| *f < at)
            .copied()
            .collect::<Vec<_>>()
    };
    assert_eq!(before(&fuzzed), before(&original));
    assert!(
        fuzzed.events().iter().any(|(f, _)| *f > at),
        "no input after the start"
    );
    assert_eq!(Start::parse("custom").unwrap(), start);
}
