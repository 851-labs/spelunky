//! `run_replay`'s file handling: savestates asked for but not reached, and bad input caught before any
//! file is touched. The real game stops while booting today (851-3177), which is exactly the "run stopped
//! before the savestate" case.

use std::path::PathBuf;
use std::sync::Arc;

use harness::{RunOptions, RunOutcome, run_replay};
use transpile::reference;

fn assets() -> Arc<gm::Assets> {
    let gmk = reference::load_gmk(&reference::default_path());
    // Rooms and the room order only: no sprites or backgrounds. Runtime tiles (rIntro's walls) need
    // backgrounds, but without sprites booting stops in rScreenInit (scrInit's font_add_sprite), before them.
    Arc::new(
        gm::Assets::new(vec![None; gmk.sprites.len()], gmk.rooms).with_room_order(gmk.room_order),
    )
}

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("harness-run-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

#[test]
fn a_savestate_the_run_never_reaches_is_reported_and_no_stale_file_is_left() {
    let replay = trace::Replay::parse("frames 20\n").unwrap();
    let assets = assets();
    for at in [None, Some(5)] {
        let out = temp(&format!("stale-{at:?}.bin"));
        std::fs::write(&out, b"from an earlier run").unwrap();
        let summary = run_replay(
            &replay,
            assets.clone(),
            &RunOptions {
                savestate_out: Some(out.clone()),
                savestate_at: at,
                ..RunOptions::default()
            },
        )
        .unwrap();
        let saved = summary.savestate.as_ref().unwrap();
        if summary.outcome == RunOutcome::Error {
            // Today: the game stops while booting.
            assert!(!saved.written);
            assert!(!out.exists(), "the stale savestate is gone");
            let reason = saved.reason.as_deref().unwrap();
            assert!(reason.contains("a GML error stopped the run"), "{reason}");
            assert!(!summary.succeeded());
        } else {
            assert!(saved.written && out.exists());
        }
    }
}

#[test]
fn bad_input_is_caught_before_the_trace_is_created() {
    let replay = trace::Replay::parse("frames 20\n").unwrap();
    let trace = temp("never-created.jsonl");
    let _ = std::fs::remove_file(&trace);
    let bad = temp("bad-savestate.bin");
    std::fs::write(&bad, b"not a savestate").unwrap();
    let e = run_replay(
        &replay,
        assets(),
        &RunOptions {
            trace: Some(trace.clone()),
            savestate_in: Some(bad),
            ..RunOptions::default()
        },
    )
    .unwrap_err();
    assert!(e.contains("not a savestate"), "{e}");
    assert!(!trace.exists());
}
