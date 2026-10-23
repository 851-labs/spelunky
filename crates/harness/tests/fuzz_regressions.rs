//! Divergences the random-input fuzzer found (851-3183), replayed on the port and checked against values
//! pinned from the oracle's trace, so `scripts/check` guards them without the oracle.

use std::sync::Arc;

use game::replay::Playback;
use trace::Replay;
use transpile::reference;

/// The game's assets as `scripts/extract` builds them, minus images and sounds. Collisions need the real
/// sprites' sizes, origins and masks, and nothing in the simulation reads pixels.
fn assets() -> Arc<gm::Assets> {
    let gmk = reference::load_gmk(&reference::default_path());
    let sprites = gmk
        .sprites
        .iter()
        .map(|s| {
            s.as_ref().map(|s| assets::Sprite {
                name: s.name.clone(),
                width: s.width(),
                height: s.height(),
                origin_x: s.origin_x,
                origin_y: s.origin_y,
                frames: vec![
                    assets::AtlasRect {
                        page: 0,
                        x: 0,
                        y: 0,
                        width: s.width(),
                        height: s.height(),
                    };
                    s.frames.len()
                ],
                shape: s.shape,
                bbox_mode: s.bbox_mode,
                alpha_tolerance: s.alpha_tolerance,
                separate_masks: s.separate_masks,
                masks: s
                    .collision_masks()
                    .unwrap()
                    .iter()
                    .map(assets::Mask::from_collision_mask)
                    .collect(),
            })
        })
        .collect();
    let backgrounds = gmk
        .backgrounds
        .iter()
        .map(|b| {
            b.as_ref().map(|b| assets::Background {
                name: b.name.clone(),
                file: None,
                width: b.image.width,
                height: b.image.height,
                use_as_tileset: b.use_as_tileset,
                tile_width: b.tile_width,
                tile_height: b.tile_height,
                h_offset: b.h_offset,
                v_offset: b.v_offset,
                h_sep: b.h_sep,
                v_sep: b.v_sep,
            })
        })
        .collect();
    Arc::new(gm::Assets::from_bundle(assets::Bundle {
        settings: gmk.settings,
        room_order: gmk.room_order,
        atlas_pages: Vec::new(),
        sprites,
        backgrounds,
        sounds: Vec::new(),
        scripts: gmk.scripts,
        objects: gmk.objects,
        rooms: gmk.rooms,
        last_tile_id: gmk.last_tile_id,
    }))
}

/// The replay's RNG seed after each of `frames`, playing it from the start.
fn seeds_after(replay: &str, frames: &[u64]) -> Vec<(u64, Option<String>, Option<i32>)> {
    let path = format!(
        "{}/../../fidelity/replays/{replay}",
        env!("CARGO_MANIFEST_DIR")
    );
    let replay = Replay::parse(&std::fs::read_to_string(path).unwrap()).unwrap();
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
