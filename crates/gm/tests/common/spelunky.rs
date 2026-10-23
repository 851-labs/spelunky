//! Real Spelunky sprites and collision masks, read from the reference `spelunky.gmk`.

use std::io::Read;
use std::sync::OnceLock;

fn gmk() -> &'static gmk::Gmk {
    static GMK: OnceLock<gmk::Gmk> = OnceLock::new();
    GMK.get_or_init(|| {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../reference/spelunky_1_1_src.zip"
        );
        let file = std::fs::File::open(path).expect("reference/spelunky_1_1_src.zip");
        let mut zip = zip::ZipArchive::new(file).unwrap();
        let mut bytes = Vec::new();
        zip.by_name("spelunky.gmk")
            .unwrap()
            .read_to_end(&mut bytes)
            .unwrap();
        gmk::parse(&bytes).expect("spelunky.gmk parses")
    })
}

/// A sprite's id in the game.
pub fn sprite_id(name: &str) -> i32 {
    gmk()
        .sprites
        .iter()
        .position(|s| s.as_ref().is_some_and(|s| s.name == name))
        .unwrap_or_else(|| panic!("no sprite {name}")) as i32
}

/// The game's sprites as the asset bundle has them, with masks (and dummy atlas rects), but only the named
/// ones: building every mask takes a while. The others are `None`.
pub fn sprites(names: &[&str]) -> Vec<Option<assets::Sprite>> {
    gmk()
        .sprites
        .iter()
        .map(|s| {
            let s = s.as_ref().filter(|s| names.contains(&s.name.as_str()))?;
            let rect = assets::AtlasRect {
                page: 0,
                x: 0,
                y: 0,
                width: s.width(),
                height: s.height(),
            };
            let masks = s.collision_masks().unwrap();
            Some(assets::Sprite {
                name: s.name.clone(),
                width: s.width(),
                height: s.height(),
                origin_x: s.origin_x,
                origin_y: s.origin_y,
                frames: vec![rect; s.frames.len()],
                shape: s.shape,
                bbox_mode: s.bbox_mode,
                alpha_tolerance: s.alpha_tolerance,
                separate_masks: s.separate_masks,
                masks: masks
                    .iter()
                    .map(assets::Mask::from_collision_mask)
                    .collect(),
            })
        })
        .collect()
}
