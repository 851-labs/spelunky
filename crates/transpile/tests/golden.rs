//! Golden outputs for representative units, so a transpiler change shows up as a readable diff here (the
//! full output under `crates/game/src/generated/` changes too, but is too big to review). Run with
//! `UPDATE_GOLDEN=1` to rewrite the files after an intended change.
//!
//! - `oPlayer1_step.rs`: the player's step event (actions, comments skipped, event composition).
//! - `characterStepEvent.rs`: the core of player physics, the densest script in the game.
//! - `oSnake.rs`, `oArrowTrapRight.rs`: the objects of the translation prototypes
//!   (`docs/design/prototypes/`).
//! - `oScreen_create.rs`, `oBlood.rs`: drag-and-drop actions (questions, fullscreen, inherited events,
//!   set variable).

use std::path::Path;
use std::sync::OnceLock;

use transpile::Source;
use transpile::emit::{Symbols, action_fn, emit_event, emit_unit};
use transpile::source::event_name;

fn spelunky() -> &'static Source {
    static SRC: OnceLock<Source> = OnceLock::new();
    SRC.get_or_init(Source::load)
}

/// An object's code actions and event functions, for the events whose name passes `filter`.
fn object(name: &str, filter: impl Fn(&str) -> bool) -> String {
    let src = spelunky();
    let mut syms = Symbols::default();
    let object = src
        .gmk
        .objects
        .iter()
        .flatten()
        .find(|o| o.name == name)
        .expect("object exists");
    let mut out = String::new();
    for event in &object.events {
        let ev = event_name(&src.gmk, event);
        if !filter(&ev) {
            continue;
        }
        for (i, action) in event.actions.iter().enumerate() {
            if action.code().is_some() {
                let unit = format!("{name}/{ev}#{i}");
                out += &emit_unit(src, &unit, &action_fn(&ev, i), &mut syms);
                out.push('\n');
            }
        }
        out += &emit_event(src, object, event, &mut syms);
        out.push('\n');
    }
    out
}

fn script(name: &str) -> String {
    emit_unit(
        spelunky(),
        &format!("scripts/{name}"),
        name,
        &mut Symbols::default(),
    )
}

fn golden(file: &str, actual: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(file);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e}; run with UPDATE_GOLDEN=1", path.display()));
    if expected != actual {
        let line = expected
            .lines()
            .zip(actual.lines())
            .position(|(a, b)| a != b)
            .unwrap_or(expected.lines().count().min(actual.lines().count()));
        panic!(
            "{file} differs from the golden output from line {} (run with UPDATE_GOLDEN=1 if intended):\n  \
             expected: {:?}\n  actual:   {:?}",
            line + 1,
            expected.lines().nth(line),
            actual.lines().nth(line)
        );
    }
}

#[test]
fn player_step() {
    golden("oPlayer1_step.rs", &object("oPlayer1", |e| e == "step"));
}

#[test]
fn character_step_event() {
    golden("characterStepEvent.rs", &script("characterStepEvent"));
}

#[test]
fn snake() {
    golden("oSnake.rs", &object("oSnake", |_| true));
}

#[test]
fn arrow_trap() {
    golden("oArrowTrapRight.rs", &object("oArrowTrapRight", |_| true));
}

#[test]
fn drag_and_drop() {
    golden("oScreen_create.rs", &object("oScreen", |e| e == "create"));
    golden("oBlood.rs", &object("oBlood", |_| true));
}
