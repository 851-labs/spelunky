//! Runs `wiki-sprites` on a bundle extracted from the real reference zips: the output must be
//! deterministic, and its images must be the atlas's pixels.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use assets::Bundle;
use serde_json::Value;

fn reference() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../reference")
}

fn wiki_sprites(assets: &Path, out: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_wiki-sprites"))
        .arg("--assets")
        .arg(assets)
        .arg("--out")
        .arg(out)
        .output()
        .unwrap()
}

fn read_tree(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    fs::read(&path).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

fn json(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn read_png(path: &Path) -> (u32, u32, Vec<u8>) {
    tools::decode_png(&fs::read(path).unwrap()).unwrap()
}

#[test]
fn export_is_deterministic_and_faithful() {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("wiki-sprites-test");
    let _ = fs::remove_dir_all(&tmp);
    let assets_dir = tmp.join("assets");
    let status = Command::new(env!("CARGO_BIN_EXE_extract"))
        .arg("--reference")
        .arg(reference())
        .arg("--out")
        .arg(&assets_dir)
        .status()
        .unwrap();
    assert!(status.success());

    // The second run goes over what looks like earlier output, which it must replace whole.
    let (a, b) = (tmp.join("a"), tmp.join("b"));
    fs::create_dir_all(b.join("sStale")).unwrap();
    fs::write(b.join("index.json"), "{}").unwrap();
    fs::write(b.join("sStale/sprite.json"), "{}").unwrap();
    for out in [&a, &b] {
        let run = wiki_sprites(&assets_dir, out);
        assert!(run.status.success(), "{run:?}");
    }
    let files = read_tree(&a);
    assert_eq!(files, read_tree(&b), "two runs differ");
    // A directory that isn't the exporter's is left alone.
    let other = tmp.join("other");
    fs::create_dir_all(&other).unwrap();
    fs::write(other.join("keep.txt"), "x").unwrap();
    assert!(!wiki_sprites(&assets_dir, &other).status.success());
    assert!(other.join("keep.txt").exists());

    let index = json(&a.join("index.json"));
    assert_eq!(index["room_speed"], 30);
    let bundle =
        Bundle::from_bytes(&fs::read(assets_dir.join(assets::BUNDLE_FILE)).unwrap()).unwrap();
    let sprite_count = bundle.sprites.iter().flatten().count();
    assert_eq!(index["sprites"].as_array().unwrap().len(), sprite_count);

    // oSnake: its own sprite, and the walk cycle its step event switches to.
    let snake = index["objects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["name"] == "oSnake")
        .unwrap();
    assert_eq!(snake["sprite"], "sSnakeLeft");
    let sprites: Vec<&str> = snake["sprites"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap())
        .collect();
    assert!(sprites.contains(&"sSnakeWalkL"), "{sprites:?}");

    // oBow holds its sprite still (its frames are states its code picks): known, but previewed at 15.
    let bow = json(&a.join("sBowRight/sprite.json"));
    assert_eq!(bow["fps_source"]["object"], "oBow");
    assert_eq!(bow["fps_source"]["image_speed"], 0.0);
    assert_eq!(bow["fps"], index["default_fps"]);

    // sSnakeWalkL plays at oSnake's create-event image_speed of 0.4: 12 frames a second.
    let meta = json(&a.join("sSnakeWalkL/sprite.json"));
    assert_eq!(meta["fps"], 12.0);
    assert_eq!(meta["fps_source"]["object"], "oSnake");
    assert_eq!(meta["fps_source"]["via"], "uses");
    let sprite = bundle
        .sprites
        .iter()
        .flatten()
        .find(|s| s.name == "sSnakeWalkL")
        .unwrap();
    let (w, h) = (sprite.width, sprite.height);
    let frames = meta["frames"].as_array().unwrap();
    assert_eq!(frames.len(), sprite.frames.len());
    assert!(frames.len() > 1);

    // Each frame is its atlas rect, and the upscaled copy is nearest-neighbour.
    let mut pages = BTreeMap::new();
    let (sw, sh, strip) = read_png(&a.join(meta["strip"].as_str().unwrap()));
    assert_eq!((sw, sh), (w * frames.len() as u32, h));
    for (i, (frame, rect)) in frames.iter().zip(&sprite.frames).enumerate() {
        let page = pages.entry(rect.page).or_insert_with(|| {
            read_png(&assets_dir.join(&bundle.atlas_pages[rect.page as usize].file))
        });
        let (fw, fh, pixels) = read_png(&a.join(frame["file"].as_str().unwrap()));
        let (bw, bh, big) = read_png(&a.join(frame["file_scaled"].as_str().unwrap()));
        assert_eq!((fw, fh), (w, h));
        assert_eq!((bw, bh), (w * 4, h * 4));
        for y in 0..h as usize {
            for x in 0..w as usize {
                let at = |p: &[u8], width: usize, x: usize, y: usize| {
                    p[(y * width + x) * 4..][..4].to_vec()
                };
                let px = at(&pixels, w as usize, x, y);
                let atlas = at(
                    &page.2,
                    page.0 as usize,
                    rect.x as usize + x,
                    rect.y as usize + y,
                );
                assert_eq!(px, atlas, "frame {i} ({x}, {y})");
                assert_eq!(at(&strip, sw as usize, i * w as usize + x, y), px);
                assert_eq!(at(&big, bw as usize, x * 4 + 3, y * 4 + 1), px);
            }
        }
    }

    // The preview is an endlessly looping APNG of every frame at 1/12 s each.
    let anim = fs::read(a.join(meta["anim"].as_str().unwrap())).unwrap();
    let reader = png::Decoder::new(std::io::Cursor::new(&anim))
        .read_info()
        .unwrap();
    let info = reader.info();
    let actl = info.animation_control.unwrap();
    assert_eq!(
        (actl.num_frames as usize, actl.num_plays),
        (frames.len(), 0)
    );
    let fctl = info.frame_control.unwrap();
    assert_eq!((fctl.delay_num, fctl.delay_den), (1, 12));
}
