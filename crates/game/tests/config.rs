//! The default configuration is what the oracle starts with: the shipped game directory's `.cfg` files,
//! byte for byte, so every frontend boots the same game.

use std::io::Read;

#[test]
fn the_default_config_files_are_the_shipped_ones() {
    let path = transpile::reference::default_path().with_file_name("spelunky_1_1.zip");
    let mut zip = zip::ZipArchive::new(std::fs::File::open(&path).unwrap()).unwrap();
    let files = game::Config::default().host.files;
    for name in ["settings.cfg", "keys.cfg", "gamepad.cfg"] {
        let mut shipped = Vec::new();
        zip.by_name(name)
            .unwrap()
            .read_to_end(&mut shipped)
            .unwrap();
        assert_eq!(files[name].as_bytes(), shipped, "{name}");
    }
    assert_eq!(files.len(), 3);
    // A replay with no header lines starts the same way.
    let replay = trace::Replay::parse("frames 1\n").unwrap();
    assert_eq!(
        game::Config::from_replay(&replay.header).host,
        game::Config::default().host
    );
}
