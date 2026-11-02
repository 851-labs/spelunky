//! F9's screenshots, natively: 1.1's game settings have "Let <F9> take a screenshot of the game" on, and
//! GM8's runner then saves `screen_save`'s picture (its back buffer, the window's size) as
//! `screenshot100.png`, or the first of `screenshot101.png`, `screenshot102.png`, ... that doesn't exist
//! yet (its key routine, Spelunky.exe 0x53dbe0). GM8 writes them in the game's folder; the port writes them
//! in its per-user data directory, beside the game's files rather than among them (`docs/quirks.md`).
//! The web build leaves F9 to the browser's own screenshots.

use crate::render::Renderer;
use std::path::{Path, PathBuf};

/// GM8's first screenshot number.
const FIRST: u32 = 100;

/// Where the next screenshot in `dir` goes: GM8's name for it.
pub fn next_path(dir: &Path) -> PathBuf {
    (FIRST..)
        .map(|n| dir.join(format!("screenshot{n}.png")))
        .find(|path| !path.exists())
        .expect("a free name")
}

/// Saves what `renderer` shows (GM's back buffer) as the next screenshot in `dir`, which it makes if
/// needed, and returns where.
pub fn take(renderer: &Renderer, dir: &Path) -> Result<PathBuf, String> {
    let (w, h, rgba) = renderer.read_screen()?;
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = next_path(dir);
    write_png(&path, w, h, &rgba)?;
    Ok(path)
}

/// Writes `rgba` rows as an opaque PNG, like the window: GM's back buffer alpha isn't part of the picture.
pub fn write_png(path: &Path, w: u32, h: u32, rgba: &[u8]) -> Result<(), String> {
    let rgb: Vec<u8> = rgba
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|&[r, g, b, _]| [r, g, b])
        .collect();
    let file = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(&rgb).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screenshots_are_numbered_from_100_skipping_those_there() {
        let dir = std::env::temp_dir().join(format!("spelunky-shots-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(next_path(&dir), dir.join("screenshot100.png"));
        write_png(&dir.join("screenshot100.png"), 1, 1, &[1, 2, 3, 255]).unwrap();
        std::fs::write(dir.join("screenshot102.png"), b"").unwrap();
        // The first free one, as GM8 counts up from 100.
        assert_eq!(next_path(&dir), dir.join("screenshot101.png"));
        let png =
            crate::textures::decode_png(&std::fs::read(dir.join("screenshot100.png")).unwrap())
                .unwrap();
        assert_eq!((png.width, png.height), (1, 1));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
