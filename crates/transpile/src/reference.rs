//! Reading the `.gmk` files from the committed reference zip.

use std::io::Read;
use std::path::{Path, PathBuf};

/// `reference/spelunky_1_1_src.zip` in this repository.
pub fn default_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../reference/spelunky_1_1_src.zip")
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
