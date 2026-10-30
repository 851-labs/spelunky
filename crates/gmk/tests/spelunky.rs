//! Reads the real `spelunky.gmk` and checks it against `docs/source-inventory.md` and against the collision
//! masks baked into the released `Spelunky.exe`.

use gmk::{ActionKind, EventType, Gmk, MaskShape};
use std::io::Read;
use std::sync::OnceLock;

fn gmk_bytes() -> &'static [u8] {
    static BYTES: OnceLock<Vec<u8>> = OnceLock::new();
    BYTES.get_or_init(|| {
        let path = std::path::Path::new(
            &std::env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR: run it with cargo"),
        )
        .join("../../reference/spelunky_1_1_src.zip");
        let file = std::fs::File::open(path).expect("reference/spelunky_1_1_src.zip");
        let mut zip = zip::ZipArchive::new(file).unwrap();
        let mut entry = zip.by_name("spelunky.gmk").unwrap();
        let mut out = Vec::new();
        entry.read_to_end(&mut out).unwrap();
        out
    })
}

fn spelunky() -> &'static Gmk {
    static GMK: OnceLock<Gmk> = OnceLock::new();
    GMK.get_or_init(|| gmk::parse(gmk_bytes()).expect("spelunky.gmk parses"))
}

fn defined<T>(list: &[Option<T>]) -> impl Iterator<Item = &T> {
    list.iter().flatten()
}

#[test]
fn counts_match_the_source_inventory() {
    let g = spelunky();
    assert_eq!(gmk_bytes().len(), 749_475);
    assert_eq!(g.game_id, 834_524);

    assert_eq!((defined(&g.objects).count(), g.objects.len()), (453, 944));
    assert_eq!((defined(&g.scripts).count(), g.scripts.len()), (139, 315));
    assert_eq!((defined(&g.rooms).count(), g.rooms.len()), (30, 124));
    assert_eq!((defined(&g.sprites).count(), g.sprites.len()), (804, 1436));
    assert_eq!(
        (defined(&g.backgrounds).count(), g.backgrounds.len()),
        (29, 56)
    );
    assert_eq!((defined(&g.sounds).count(), g.sounds.len()), (0, 54));
    assert_eq!((defined(&g.fonts).count(), g.fonts.len()), (0, 1));
    assert_eq!((defined(&g.timelines).count(), g.timelines.len()), (0, 2));
    assert_eq!(g.paths.len(), 0);
    assert!(g.triggers.is_empty() && g.constants.is_empty() && g.include_files.is_empty());
    assert!(g.extension_packages.is_empty());
    // source-inventory.md says 3; the file (and the Python parser) has 7, all empty.
    assert_eq!(g.library_creation_code.len(), 7);
    assert!(g.library_creation_code.iter().all(String::is_empty));
    assert_eq!(g.room_order.len(), 30);
    assert_eq!(g.resource_tree.len(), 12);

    let frames: usize = defined(&g.sprites).map(|s| s.frames.len()).sum();
    assert_eq!(frames, 2299);
    for s in defined(&g.sprites) {
        for f in &s.frames {
            assert_eq!(
                f.bgra.len(),
                f.width as usize * f.height as usize * 4,
                "{}",
                s.name
            );
        }
    }
    for b in defined(&g.backgrounds) {
        assert_eq!(
            b.image.bgra.len(),
            b.image.width as usize * b.image.height as usize * 4,
            "{}",
            b.name
        );
    }

    let events: Vec<_> = defined(&g.objects).flat_map(|o| &o.events).collect();
    assert_eq!(events.len(), 1001);
    let actions: Vec<_> = events.iter().flat_map(|e| &e.actions).collect();
    let code = actions
        .iter()
        .filter(|a| a.kind == ActionKind::Code)
        .count();
    assert_eq!((code, actions.len() - code), (969, 350));
    assert_eq!(
        events
            .iter()
            .filter(|e| e.event_type == EventType::Create)
            .count(),
        305
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| e.event_type == EventType::Collision)
            .count(),
        123
    );

    let instances: usize = defined(&g.rooms).map(|r| r.instances.len()).sum();
    let tiles: usize = defined(&g.rooms).map(|r| r.tiles.len()).sum();
    assert_eq!((instances, tiles), (6330, 9));
    assert!(defined(&g.rooms).all(|r| r.creation_code.is_empty()));
}

#[test]
fn spot_checks_match_the_source_inventory() {
    let g = spelunky();
    let object = |name: &str| defined(&g.objects).find(|o| o.name == name).unwrap();
    let sprite_name = |id: i32| gmk::get(&g.sprites, id).map(|s| s.name.as_str());
    let object_name = |id: i32| gmk::get(&g.objects, id).map(|o| o.name.as_str());

    let player = object("oPlayer1");
    assert_eq!(object_name(player.parent), Some("oCharacter"));
    assert_eq!(sprite_name(player.sprite), Some("sStandLeft"));
    assert_eq!(player.depth, 50);
    assert_eq!(player.events.len(), 15);

    let screen = object("oScreen");
    assert!(screen.persistent && !screen.visible);
    let item = object("oItem");
    assert!(item.solid);
    assert_eq!(object("oCaveBG").depth, 99_999_999);

    let inherited = defined(&g.objects)
        .flat_map(|o| &o.events)
        .flat_map(|e| &e.actions)
        .filter(|a| a.function == "action_inherited")
        .count();
    assert_eq!(inherited, 284);

    // The DnD "execute script" action resolves its first argument to a script id.
    let exec = defined(&g.objects)
        .flat_map(|o| &o.events)
        .flat_map(|e| &e.actions)
        .find(|a| a.function == "action_execute_script")
        .unwrap();
    assert_eq!(
        (exec.library_id, exec.action_id, exec.args.len()),
        (1, 601, 6)
    );
    assert_eq!(exec.args[0].kind, gmk::ArgKind::Script);

    let start = gmk::get(&g.rooms, g.room_order[0]).unwrap();
    assert!(start.width > 0 && start.speed > 0);
    assert!(
        !g.settings.esc_ends_game && g.settings.f4_fullscreen && g.settings.freeze_on_lose_focus
    );
    assert!(g.settings.display_errors && !g.settings.treat_uninitialized_as_zero);
}

fn fnv(mut h: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}

#[test]
fn collision_masks_match_spelunky_exe() {
    let g = spelunky();
    let expected = include_str!("data/exe_mask_digests.txt");
    let expected: Vec<(&str, usize, u64)> = expected
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| {
            let mut parts = l.split(' ');
            let name = parts.next().unwrap();
            let count = parts.next().unwrap().parse().unwrap();
            let digest = u64::from_str_radix(parts.next().unwrap(), 16).unwrap();
            (name, count, digest)
        })
        .collect();
    let sprites: Vec<_> = defined(&g.sprites).collect();
    assert_eq!(sprites.len(), expected.len());

    let mut mismatches = Vec::new();
    for (sprite, (name, count, digest)) in sprites.iter().zip(&expected) {
        assert_eq!(&sprite.name, name);
        let masks = sprite.collision_masks().unwrap();
        let mut h = 0xcbf2_9ce4_8422_2325;
        for m in &masks {
            for v in [
                m.width,
                m.height,
                m.bbox.left,
                m.bbox.top,
                m.bbox.right,
                m.bbox.bottom,
            ] {
                h = fnv(h, &v.to_le_bytes());
            }
            let bytes: Vec<u8> = m.solid.iter().map(|&s| u8::from(s)).collect();
            h = fnv(h, &bytes);
        }
        if masks.len() != *count || h != *digest {
            mismatches.push(format!("{name} ({:?})", sprite.shape));
        }
    }
    assert!(
        mismatches.is_empty(),
        "{} sprites differ from Spelunky.exe: {mismatches:?}",
        mismatches.len()
    );
    // Both shapes the game uses are covered.
    assert!(sprites.iter().any(|s| s.shape == MaskShape::Precise));
    assert!(sprites.iter().any(|s| s.shape == MaskShape::Rectangle));
}

#[test]
fn corrupt_input_is_an_error_not_a_panic() {
    let bytes = gmk_bytes();
    assert!(gmk::parse(&[]).is_err());
    assert!(gmk::parse(&bytes[..bytes.len() - 1]).is_err());
    // Truncations at many points, and single-byte corruptions spread across the file.
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for _ in 0..200 {
        let at = (next() % bytes.len() as u64) as usize;
        assert!(gmk::parse(&bytes[..at]).is_err());
        let mut corrupt = bytes.to_vec();
        corrupt[at] ^= 1 << (next() % 8);
        let _ = gmk::parse(&corrupt);
    }
}
