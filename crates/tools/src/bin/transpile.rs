//! Transpiles the game's GML into `crates/game/src/generated/` (the `transpile` crate does the work).
//!
//! ```sh
//! transpile                     # regenerate crates/game/src/generated/ (what scripts/transpile runs)
//! transpile --check             # fail if regenerating would change anything (scripts/check runs this)
//! transpile --print oSnake/ scripts/rand   # print some units: object names end in `/`, others are
//!                                          # unit-name prefixes
//! ```
//!
//! `--reference <zip>` and `--out <dir>` override the defaults (`reference/spelunky_1_1_src.zip` and
//! `crates/game/src/generated`).

use std::path::Path;
use std::process::ExitCode;

use transpile::emit::{Symbols, action_fn, emit_object, emit_unit};
use transpile::source::UnitKind;
use transpile::{Source, reference};

fn print(src: &Source, prefixes: &[String]) {
    let mut syms = Symbols::default();
    for prefix in prefixes {
        if let Some(name) = prefix.strip_suffix('/') {
            let Some(object) = src.gmk.objects.iter().flatten().find(|o| o.name == name) else {
                eprintln!("no object {name}");
                continue;
            };
            print!("{}", emit_object(src, object, &mut syms));
            continue;
        }
        for unit in src
            .units
            .iter()
            .filter(|u| u.name.starts_with(prefix.as_str()) && u.kind != UnitKind::DndExpression)
        {
            let name = match unit.name.rsplit_once('/') {
                Some(("scripts", name)) => name.to_owned(),
                _ => {
                    let (event, action) = unit
                        .name
                        .rsplit_once('/')
                        .unwrap()
                        .1
                        .split_once('#')
                        .unwrap();
                    action_fn(event, action.parse().unwrap())
                }
            };
            println!("{}", emit_unit(src, &unit.name, &name, &mut syms));
        }
    }
}

fn main() -> ExitCode {
    let mut check = false;
    let mut out = Path::new(env!("CARGO_MANIFEST_DIR")).with_file_name("game/src/generated");
    let mut zip = reference::default_path();
    let mut prefixes = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--check" => check = true,
            "--out" => out = args.next().expect("--out needs a directory").into(),
            "--reference" => zip = args.next().expect("--reference needs a zip").into(),
            // Everything after `--print` is a prefix.
            "--print" => prefixes = Some(args.by_ref().collect::<Vec<_>>()),
            _ => {
                eprintln!(
                    "usage: transpile [--check] [--out DIR] [--reference ZIP] [--print PREFIX...]"
                );
                return ExitCode::FAILURE;
            }
        }
    }
    if let Some(flag) = prefixes.iter().flatten().find(|p| p.starts_with("--")) {
        eprintln!("{flag} must come before --print");
        return ExitCode::FAILURE;
    }
    let src = Source::new(reference::load_gmk(&zip));
    if let Some(prefixes) = prefixes {
        print(&src, &prefixes);
        return ExitCode::SUCCESS;
    }
    let output = transpile::generate(&src);
    if check {
        let stale = transpile::check(&output, &out).expect("read the generated code");
        if stale.is_empty() {
            return ExitCode::SUCCESS;
        }
        eprintln!(
            "{} is out of date; run scripts/transpile and commit the result:",
            out.display()
        );
        for s in stale.iter().take(20) {
            eprintln!("  {s}");
        }
        if stale.len() > 20 {
            eprintln!("  ... and {} more", stale.len() - 20);
        }
        return ExitCode::FAILURE;
    }
    transpile::write(&output, &out).expect("write the generated code");
    let s = &output.symbols;
    if std::env::var_os("TRANSPILE_LIST_BUILTINS").is_some() {
        for (name, arities) in &s.builtins {
            println!("{name} {arities:?}");
        }
    }
    println!(
        "{} units ({} GML lines) -> {} Rust lines in {} files under {}",
        output.units,
        output.gml_lines,
        output.rust_lines,
        output.files.len(),
        out.display()
    );
    println!(
        "{} variables, {} globals, {} built-in functions, {} constants",
        s.vars.len(),
        s.gvars.len(),
        s.builtins.len(),
        s.constants.len()
    );
    ExitCode::SUCCESS
}
