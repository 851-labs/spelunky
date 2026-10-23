//! Reading `spelunky.gmk` from the committed reference zip.

use std::io::Read;
use std::path::{Path, PathBuf};

/// `reference/spelunky_1_1_src.zip` in this repository.
pub fn default_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../reference/spelunky_1_1_src.zip")
}

/// Reads and parses `spelunky.gmk` from a source zip. Panics if it can't: the reference files are part of
/// the repository.
pub fn load_gmk(zip_path: &Path) -> gmk::Gmk {
    let file = std::fs::File::open(zip_path)
        .unwrap_or_else(|e| panic!("open {}: {e}", zip_path.display()));
    let mut zip = zip::ZipArchive::new(file).expect("read zip");
    let mut bytes = Vec::new();
    zip.by_name("spelunky.gmk")
        .expect("spelunky.gmk in zip")
        .read_to_end(&mut bytes)
        .expect("read spelunky.gmk");
    gmk::parse(&bytes).expect("spelunky.gmk parses")
}
