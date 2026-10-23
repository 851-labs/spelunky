//! The GML-to-Rust transpiler (851-3171): translates every script, code action and drag-and-drop action
//! of Spelunky Classic 1.1 into Rust against the `gm` runtime, and writes the result to
//! `crates/game/src/generated/`. `docs/design/translation.md` explains why the game is transpiled and which GM8
//! rules the translation reproduces; `docs/design/runtime.md` section 12 is the API it targets.
//!
//! The output is committed. `scripts/transpile` regenerates it, and `scripts/check` fails if regenerating
//! would change it (see [`check`]).
//!
//! Every generated statement carries its GML unit and line (`g.at("oSnake/step#1", 33)`) and the GML as a
//! comment, so a runtime error or trace divergence points at the line of GML to read.

pub mod emit;
pub mod generate;
mod layout;
pub mod reference;
pub mod source;

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::Path;

pub use generate::{Output, generate};
pub use source::Source;

/// Whether a file is the generator's (its first line is [`generate::HEADER`]'s).
fn is_generated(path: &Path) -> bool {
    let first = generate::HEADER.lines().next().expect("a header");
    fs::read_to_string(path).is_ok_and(|s| s.lines().next() == Some(first))
}

/// The generated `.rs` files under `dir`, as paths relative to it. Hand-written files are never
/// included, so pointing `--out` at the wrong directory can't delete them.
fn existing(dir: &Path) -> io::Result<BTreeSet<String>> {
    let mut out = BTreeSet::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let entries = match fs::read_dir(&d) {
            Ok(e) => e,
            Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e),
        };
        for entry in entries {
            let path = entry?.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") && is_generated(&path) {
                let rel = path.strip_prefix(dir).expect("under dir");
                out.insert(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    Ok(out)
}

/// Writes the output to `dir`, removing generated files that are no longer produced. Files whose contents
/// are unchanged aren't rewritten, so incremental builds and editors don't see them change.
pub fn write(out: &Output, dir: &Path) -> io::Result<()> {
    for stale in existing(dir)?.difference(&out.files.keys().cloned().collect()) {
        fs::remove_file(dir.join(stale))?;
    }
    for (rel, text) in &out.files {
        let path = dir.join(rel);
        if fs::read_to_string(&path).is_ok_and(|old| old == *text) {
            continue;
        }
        fs::create_dir_all(path.parent().expect("a file in a directory"))?;
        fs::write(&path, text)?;
    }
    Ok(())
}

/// The files under `dir` that differ from the output: changed, missing or no longer generated. Empty
/// when the committed code is up to date.
pub fn check(out: &Output, dir: &Path) -> io::Result<Vec<String>> {
    let mut stale = Vec::new();
    for rel in existing(dir)? {
        if !out.files.contains_key(&rel) {
            stale.push(format!("{rel} (no longer generated)"));
        }
    }
    for (rel, text) in &out.files {
        match fs::read_to_string(dir.join(rel)) {
            Ok(old) if old == *text => {}
            Ok(_) => stale.push(format!("{rel} (changed)")),
            Err(_) => stale.push(format!("{rel} (missing)")),
        }
    }
    Ok(stale)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_replaces_only_generated_files() {
        let dir = std::env::temp_dir().join(format!("transpile-write-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("objects")).unwrap();
        let header = generate::HEADER;
        fs::write(dir.join("lib.rs"), "// hand-written\n").unwrap();
        fs::write(dir.join("objects/old.rs"), format!("{header}// stale\n")).unwrap();
        let out = Output {
            files: [("objects/new.rs".to_owned(), format!("{header}// new\n"))].into(),
            units: 0,
            gml_lines: 0,
            rust_lines: 0,
            symbols: emit::Symbols::default(),
        };
        assert_eq!(check(&out, &dir).unwrap().len(), 2);
        write(&out, &dir).unwrap();
        assert!(dir.join("lib.rs").exists(), "hand-written files stay");
        assert!(
            !dir.join("objects/old.rs").exists(),
            "stale generated files go"
        );
        assert!(check(&out, &dir).unwrap().is_empty());
        fs::remove_dir_all(&dir).unwrap();
    }
}
