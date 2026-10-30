//! Reading the `.gmk` files from the committed reference zip.

use std::io::Read;
use std::path::{Path, PathBuf};

/// The source zip, in this repository.
const ZIP: &str = "reference/spelunky_1_1_src.zip";

/// This repository's root, found as it runs, not as it compiles (`env!`), so the build holds no path of the
/// checkout's: one cloned into a new worktree (scripts/seed-target) needn't rebuild, nor what depends on this
/// crate. The nearest directory with the source zip above the running executable, which is in its own
/// checkout's `target/` as the path `env!` gave was its own; or else above the manifest directory of the
/// package Cargo runs (`cargo test` and `cargo run` set it: a `target/` elsewhere), or the current directory.
pub fn repo_root() -> PathBuf {
    let starts = [
        std::env::current_exe().ok(),
        std::env::var_os("CARGO_MANIFEST_DIR").map(PathBuf::from),
        std::env::current_dir().ok(),
    ];
    starts
        .iter()
        .flatten()
        .find_map(|start| start.ancestors().find(|dir| dir.join(ZIP).is_file()))
        .map(Path::to_path_buf)
        .unwrap_or_else(|| {
            panic!("no {ZIP} above this executable, CARGO_MANIFEST_DIR or the current directory")
        })
}

/// `reference/spelunky_1_1_src.zip` in this repository.
pub fn default_path() -> PathBuf {
    repo_root().join(ZIP)
}

/// The two GameMaker programs in the source zip: the game, and the settings program `config.exe` that
/// ships beside it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Program {
    /// `spelunky.gmk`, transpiled into `crates/game`.
    Game,
    /// `config.gmk`, transpiled into `crates/config`.
    Config,
}

impl Program {
    /// Both programs, in the order `scripts/transpile` generates them.
    pub const ALL: [Program; 2] = [Program::Game, Program::Config];

    /// The program's file in the source zip.
    pub fn member(self) -> &'static str {
        match self {
            Program::Game => "spelunky.gmk",
            Program::Config => "config.gmk",
        }
    }

    /// The crate its generated code goes in, under `crates/`.
    pub fn crate_name(self) -> &'static str {
        match self {
            Program::Game => "game",
            Program::Config => "config",
        }
    }

    /// The program by its crate's name (`--program game`).
    pub fn from_name(name: &str) -> Option<Program> {
        Program::ALL.into_iter().find(|p| p.crate_name() == name)
    }
}

/// Reads and parses `spelunky.gmk` from a source zip. Panics if it can't: the reference files are part of
/// the repository.
pub fn load_gmk(zip_path: &Path) -> gmk::Gmk {
    load_program(zip_path, Program::Game)
}

/// Reads and parses one of the programs from a source zip. Panics if it can't.
pub fn load_program(zip_path: &Path, program: Program) -> gmk::Gmk {
    let member = program.member();
    let file = std::fs::File::open(zip_path)
        .unwrap_or_else(|e| panic!("open {}: {e}", zip_path.display()));
    let mut zip = zip::ZipArchive::new(file).expect("read zip");
    let mut bytes = Vec::new();
    zip.by_name(member)
        .unwrap_or_else(|e| panic!("{member} in zip: {e}"))
        .read_to_end(&mut bytes)
        .unwrap_or_else(|e| panic!("read {member}: {e}"));
    gmk::parse(&bytes).unwrap_or_else(|e| panic!("{member} parses: {e}"))
}

/// A program's rooms, room order and backgrounds as `gm::Assets`, without images: each sprite is an
/// imageless, filled rectangle of its size (so collisions and mouse clicks find their instances) and
/// each background an imageless one of its size. For tests that run a program without extracted assets.
pub fn stub_assets(gmk: &gmk::Gmk) -> gm::Assets {
    let sprites = gmk
        .sprites
        .iter()
        .map(|s| {
            s.as_ref().map(|s| {
                let (w, h) = s
                    .frames
                    .first()
                    .map_or((1, 1), |f| (f.width.max(1), f.height.max(1)));
                assets::Sprite {
                    name: s.name.clone(),
                    width: w,
                    height: h,
                    origin_x: s.origin_x,
                    origin_y: s.origin_y,
                    frames: Vec::new(),
                    shape: assets::MaskShape::Rectangle,
                    bbox_mode: assets::BboxMode::FullImage,
                    alpha_tolerance: 0,
                    separate_masks: false,
                    masks: vec![assets::Mask {
                        bbox: assets::Bbox {
                            left: 0,
                            top: 0,
                            right: w - 1,
                            bottom: h - 1,
                        },
                        data: assets::MaskData::Filled,
                    }],
                }
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
    gm::Assets::new(sprites, gmk.rooms.clone())
        .with_room_order(gmk.room_order.clone())
        .with_backgrounds(backgrounds)
}
