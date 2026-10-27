//! Runs the extractor on the real reference zips: the output must be deterministic and must round-trip
//! the source data.

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use assets::{Bundle, MaskData};

fn reference() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../reference")
}

fn extract(out: &Path) {
    let status = Command::new(env!("CARGO_BIN_EXE_extract"))
        .arg("--reference")
        .arg(reference())
        .arg("--out")
        .arg(out)
        .status()
        .unwrap();
    assert!(status.success());
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

fn zip_entry(zip: &str, name: &str) -> Vec<u8> {
    let mut zip = zip::ZipArchive::new(fs::File::open(reference().join(zip)).unwrap()).unwrap();
    let mut out = Vec::new();
    zip.by_name(name).unwrap().read_to_end(&mut out).unwrap();
    out
}

fn decode_png(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    tools::decode_png(bytes).unwrap()
}

#[test]
fn extraction_is_deterministic_and_faithful() {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("extract-test");
    let (a, b) = (tmp.join("a"), tmp.join("b"));
    let _ = fs::remove_dir_all(&tmp);
    // A stale file in an owned directory must not survive a run; unrelated files must.
    fs::create_dir_all(a.join("gml")).unwrap();
    fs::write(a.join("gml/stale.gml"), "x").unwrap();
    fs::write(a.join("keep.txt"), "x").unwrap();
    extract(&a);
    extract(&b);
    assert!(!a.join("gml/stale.gml").exists());
    fs::remove_file(a.join("keep.txt")).unwrap();

    let files = read_tree(&a);
    assert_eq!(files, read_tree(&b), "two runs differ");
    // Running again over existing output changes nothing either.
    extract(&a);
    assert_eq!(files, read_tree(&a));

    let bundle_bytes = &files[Path::new(assets::BUNDLE_FILE)];
    // postcard isn't self-describing, so pin the encoding. When this changes, check whether the shape of
    // the bundle (or a gmk type inside it) changed; if so, bump assets::FORMAT_VERSION. Then update this.
    let digest = bundle_bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3)
    });
    assert_eq!(
        (assets::FORMAT_VERSION, format!("{digest:016x}")),
        (2, "73e2f03beeab308a".to_owned()),
        "bundle.bin encoding changed"
    );
    let bundle = Bundle::from_bytes(bundle_bytes).unwrap();
    let gmk = gmk::parse(&zip_entry("spelunky_1_1_src.zip", "spelunky.gmk")).unwrap();
    assert_eq!(bundle.objects, gmk.objects);
    assert_eq!(bundle.rooms, gmk.rooms);
    assert_eq!(bundle.scripts, gmk.scripts);
    assert_eq!(bundle.room_order, gmk.room_order);
    assert_eq!(bundle.last_tile_id, gmk.last_tile_id);
    assert_eq!(bundle.objects.iter().flatten().count(), 453);
    assert_eq!(bundle.rooms.iter().flatten().count(), 30);
    assert_eq!(bundle.scripts.iter().flatten().count(), 139);
    assert_eq!(bundle.backgrounds.iter().flatten().count(), 29);
    assert_eq!(
        files
            .keys()
            .filter(|p| p.starts_with("gml/scripts"))
            .count(),
        139
    );

    // Sounds are byte-for-byte copies.
    assert!(bundle.sounds.len() > 60);
    for sound in &bundle.sounds {
        let expected = zip_entry("spelunky_1_1.zip", &format!("sound/{}", sound.name));
        assert!(files[Path::new(&sound.file)] == expected, "{}", sound.name);
    }

    // Every frame's pixels sit in the atlas exactly, with a transparent gutter, and every mask matches
    // what gmk computes.
    let pages: Vec<_> = bundle
        .atlas_pages
        .iter()
        .map(|p| decode_png(&files[Path::new(&p.file)]))
        .collect();
    let mut frames = 0;
    for (s, b) in gmk.sprites.iter().zip(&bundle.sprites) {
        let (Some(s), Some(b)) = (s, b) else {
            assert!(s.is_none() && b.is_none());
            continue;
        };
        assert_eq!(
            (b.origin_x, b.origin_y, b.frames.len()),
            (s.origin_x, s.origin_y, s.frames.len())
        );
        for (frame, rect) in s.frames.iter().zip(&b.frames) {
            frames += 1;
            let (pw, _, pixels) = &pages[rect.page as usize];
            let at = |x: u32, y: u32| {
                let i = (y as usize * *pw as usize + x as usize) * 4;
                &pixels[i..i + 4]
            };
            for y in 0..frame.height {
                for x in 0..frame.width {
                    let i = (y as usize * frame.width as usize + x as usize) * 4;
                    let [bl, g, r, al] = frame.bgra[i..i + 4] else {
                        unreachable!()
                    };
                    assert_eq!(
                        at(rect.x + x, rect.y + y),
                        [r, g, bl, al],
                        "{} ({x}, {y})",
                        s.name
                    );
                }
            }
            for x in rect.x - 1..=rect.x + rect.width {
                assert_eq!(at(x, rect.y - 1)[3], 0);
                assert_eq!(at(x, rect.y + rect.height)[3], 0);
            }
            for y in rect.y - 1..=rect.y + rect.height {
                assert_eq!(at(rect.x - 1, y)[3], 0);
                assert_eq!(at(rect.x + rect.width, y)[3], 0);
            }
        }
        let masks = s.collision_masks().unwrap();
        assert_eq!(masks.len(), b.masks.len());
        for (m, packed) in masks.iter().zip(&b.masks) {
            assert_eq!(m.bbox, packed.bbox);
            if s.shape == gmk::MaskShape::Rectangle && !m.bbox.is_empty() {
                assert_eq!(packed.data, MaskData::Filled);
            }
            for y in 0..m.height {
                for x in 0..m.width {
                    assert_eq!(packed.get(x, y), m.get(x, y), "{} ({x}, {y})", s.name);
                }
            }
        }
    }
    assert_eq!(frames, 2299);

    for b in bundle.backgrounds.iter().flatten() {
        let (w, h, _) = decode_png(&files[Path::new(b.file.as_ref().unwrap())]);
        assert_eq!((w, h), (b.width, b.height));
    }
}
