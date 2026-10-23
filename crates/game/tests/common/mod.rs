//! Shared by the game's integration tests.

/// Every sprite slot of the game file as an empty sprite: no frames or masks, but there, so `scrInit`'s
/// `font_add_sprite` calls (which only check the sprite exists) work without extracted assets.
pub fn stub_sprites(gmk: &gmk::Gmk) -> Vec<Option<assets::Sprite>> {
    gmk.sprites
        .iter()
        .map(|s| {
            s.as_ref().map(|s| assets::Sprite {
                name: s.name.clone(),
                width: 0,
                height: 0,
                origin_x: 0,
                origin_y: 0,
                frames: Vec::new(),
                shape: assets::MaskShape::Rectangle,
                bbox_mode: assets::BboxMode::FullImage,
                alpha_tolerance: 0,
                separate_masks: false,
                masks: Vec::new(),
            })
        })
        .collect()
}

/// Every background of the game file by size, with no image, so runtime tiles work (`scrSetupWalls` adds them
/// while `rIntro` loads at boot).
pub fn stub_backgrounds(gmk: &gmk::Gmk) -> Vec<Option<assets::Background>> {
    gmk.backgrounds
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
        .collect()
}
