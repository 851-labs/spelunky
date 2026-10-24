//! What the integration tests that play the real game share.

use std::sync::Arc;

use transpile::reference;

/// The game's assets as `scripts/extract` builds them, minus images and sounds. Collisions need the real
/// sprites' sizes, origins and masks, and nothing in the simulation reads pixels.
pub fn assets() -> Arc<gm::Assets> {
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

/// A committed replay, `fidelity/replays/<name>`.
pub fn replay(name: &str) -> trace::Replay {
    let path = format!(
        "{}/../../fidelity/replays/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    trace::Replay::parse(&std::fs::read_to_string(path).unwrap()).unwrap()
}
