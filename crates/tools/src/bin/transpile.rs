//! Transpiles the GML of the source zip's two programs, the game (`spelunky.gmk`) and the settings program
//! (`config.gmk`), into `crates/game/src/generated/` and `crates/config/src/generated/` (the `transpile`
//! crate does the work).
//!
//! ```sh
//! transpile                     # regenerate both crates' generated/ (what scripts/transpile runs)
//! transpile --check             # fail if regenerating would change anything (scripts/check runs this)
//! transpile --program config    # only one program: `game` or `config`
//! transpile --print oSnake/ scripts/rand   # print some units of one program (the game by default):
//!                                          # object names end in `/`, others are unit-name prefixes
//! ```
//!
//! `--reference <zip>` and `--out <dir>` override the defaults (`reference/spelunky_1_1_src.zip` and
//! `crates/<program>/src/generated`); `--out` needs `--program`.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use transpile::Source;
use transpile::reference::{self, Program};

const USAGE: &str = "usage: transpile [--check] [--program game|config] [--out DIR] [--reference ZIP] \
                     [--print PREFIX...]";

fn main() -> ExitCode {
    let mut check = false;
    let mut out = None;
    let mut program = None;
    let mut zip: Option<PathBuf> = None;
    let mut prefixes = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--check" => check = true,
            "--out" => out = Some(PathBuf::from(args.next().expect("--out needs a directory"))),
            "--program" => {
                let name = args.next().expect("--program needs a name");
                let Some(p) = Program::from_name(&name) else {
                    eprintln!("no program {name}: `game` or `config`");
                    return ExitCode::FAILURE;
                };
                program = Some(p);
            }
            "--reference" => zip = Some(args.next().expect("--reference needs a zip").into()),
            // Everything after `--print` is a prefix.
            "--print" => prefixes = Some(args.by_ref().collect::<Vec<_>>()),
            _ => {
                eprintln!("{USAGE}");
                return ExitCode::FAILURE;
            }
        }
    }
    if let Some(flag) = prefixes.iter().flatten().find(|p| p.starts_with("--")) {
        eprintln!("{flag} must come before --print");
        return ExitCode::FAILURE;
    }
    if out.is_some() && program.is_none() {
        eprintln!("--out needs --program");
        return ExitCode::FAILURE;
    }
    // The default only now, so --print, or --out with --program, needs no repository around a --reference
    // ZIP (writing the crates' generated code in place does).
    let zip = zip.unwrap_or_else(reference::default_path);
    if let Some(prefixes) = prefixes {
        let src = Source::new(reference::load_program(
            &zip,
            program.unwrap_or(Program::Game),
        ));
        return match transpile::print(&src, &prefixes) {
            Ok(code) => {
                print!("{code}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::FAILURE
            }
        };
    }
    let programs = program.map_or(Program::ALL.to_vec(), |p| vec![p]);
    let mut ok = true;
    for p in programs {
        let dir = out.clone().unwrap_or_else(|| {
            reference::repo_root()
                .join("crates")
                .join(p.crate_name())
                .join("src/generated")
        });
        let src = Source::new(reference::load_program(&zip, p));
        ok &= run(&src, &dir, check);
    }
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Regenerates (or with `check`, checks) one program's generated code in `out`. False if it's stale.
fn run(src: &Source, out: &Path, check: bool) -> bool {
    let output = transpile::generate(src);
    if check {
        let stale = transpile::check(&output, out).expect("read the generated code");
        if stale.is_empty() {
            return true;
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
        return false;
    }
    transpile::write(&output, out).expect("write the generated code");
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
    true
}
