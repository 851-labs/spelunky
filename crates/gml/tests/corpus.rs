//! Snapshot tests over `tests/corpus/*.gml`: each file must parse, and its canonical printing must match
//! the committed `.printed` file. Run with `GML_BLESS=1` to rewrite the snapshots after an intended change.

use std::fs;
use std::path::Path;

use gml::visit::clear_spans;

#[test]
fn corpus_snapshots() {
    let dir = Path::new(
        &std::env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR: run it with cargo"),
    )
    .join("tests/corpus");
    let bless = std::env::var_os("GML_BLESS").is_some();
    let mut paths: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "gml"))
        .collect();
    paths.sort();
    assert!(paths.len() >= 5, "corpus is missing");

    for path in paths {
        let source = fs::read_to_string(&path).unwrap();
        let mut ast = gml::parse_program(&source)
            .unwrap_or_else(|e| panic!("{}: {}", path.display(), e.display(&source)));
        let printed = gml::print::program(&ast);

        let mut again = gml::parse_program(&printed).unwrap();
        clear_spans(&mut ast);
        clear_spans(&mut again);
        assert_eq!(ast, again, "{} does not round-trip", path.display());

        let snapshot = path.with_extension("printed");
        if bless {
            fs::write(&snapshot, &printed).unwrap();
        } else {
            let expected = fs::read_to_string(&snapshot)
                .unwrap_or_else(|_| panic!("missing {}; run with GML_BLESS=1", snapshot.display()));
            assert_eq!(
                printed,
                expected,
                "{} changed; run with GML_BLESS=1 if intended",
                path.display()
            );
        }
    }
}
