//! The real game playing a replay, interrupted by a savestate on disk, plays exactly like an uninterrupted
//! run. Built-ins the port doesn't have yet stop the game early (today, while booting), so this checks
//! whatever prefix runs, and that both runs stop at the same place for the same reason. It gets stronger
//! as the port does.
//!
//! NOTE: until the game boots (851-3177, then 851-3128), no frame runs, so this compares two empty traces
//! and proves nothing about savestates yet; `gm/tests/savestate.rs` is the real proof meanwhile. It
//! starts checking real frames, with no change here, as soon as the game gets past booting.

use std::io::Cursor;
use std::sync::Arc;

use game::Game;
use game::replay::{Playback, RunError};
use trace::diff::{DiffOptions, Outcome, diff_traces};
use trace::{Filter, Replay, TraceReader, TraceWriter};
use transpile::reference;

fn assets() -> Arc<gm::Assets> {
    let gmk = reference::load_gmk(&reference::default_path());
    // Rooms and the room order only: no sprites or backgrounds. Runtime tiles (rIntro's walls) need
    // backgrounds, but without sprites booting stops in rScreenInit (scrInit's font_add_sprite), before them.
    Arc::new(
        gm::Assets::new(vec![None; gmk.sprites.len()], gmk.rooms).with_room_order(gmk.room_order),
    )
}

/// Steps until the replay ends or an error, writing a trace line per frame.
fn play(p: &mut Playback, out: &mut TraceWriter<Vec<u8>>) -> Option<RunError> {
    loop {
        match p.step() {
            Ok(Some(_)) => out.write_frame(&p.game().trace()).unwrap(),
            Ok(None) => return None,
            Err(e) => return Some(e),
        }
    }
}

#[test]
fn a_savestate_on_disk_resumes_like_an_uninterrupted_run() {
    let (save_at, end) = (60, 150);
    let replay = Replay::parse(&format!(
        "seed 12345\nframes {end}\n40 hold left 30\n70 press Z\n75 release Z\n"
    ))
    .unwrap();
    let assets = assets();

    let mut full_trace = TraceWriter::new(Vec::new(), Filter::default());
    let full_error = match Playback::boot(assets.clone(), &replay) {
        Ok(mut p) => play(&mut p, &mut full_trace),
        Err(e) => Some(e),
    };

    let mut first_half = TraceWriter::new(Vec::new(), Filter::default());
    let mut second_half = TraceWriter::new(Vec::new(), Filter::default());
    let split_error = match Playback::boot(assets.clone(), &replay) {
        Ok(mut p) => {
            p.stop_before(save_at);
            match play(&mut p, &mut first_half) {
                Some(e) => Some(e),
                None => {
                    let bytes = p.into_game().save_state();
                    let game =
                        Game::load_state(assets.clone(), Default::default(), &bytes).unwrap();
                    assert_eq!(game.save_state(), bytes);
                    let mut p = Playback::resume(game, &replay);
                    assert_eq!(p.next_frame(), save_at);
                    play(&mut p, &mut second_half)
                }
            }
        }
        Err(e) => Some(e),
    };
    assert_eq!(split_error, full_error);
    if let Some(e) = &full_error {
        eprintln!("the replay stops at {e}");
    }

    let mut split = first_half.into_inner();
    split.extend(second_half.into_inner());
    let read = |t: Vec<u8>| TraceReader::new(Cursor::new(t)).unwrap();
    let full = full_trace.into_inner();
    let lines = full.iter().filter(|&&b| b == b'\n').count() as u64;
    let report = diff_traces(read(full), read(split), &DiffOptions::default()).unwrap();
    if lines == 0 {
        assert_eq!(report.outcome, Outcome::NothingCompared);
    } else {
        assert_eq!(report.outcome, Outcome::Identical, "{}", report.to_text());
        assert_eq!(report.equal_frames, lines);
    }
}
