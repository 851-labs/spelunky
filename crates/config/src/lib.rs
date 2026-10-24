//! Spelunky Classic 1.1's settings program, `config.exe` (`config.gmk` in the source zip), on the `gm`
//! runtime: the window that sets the resolution, graphics, controls and volumes, writes `settings.cfg`,
//! `keys.cfg` and `gamepad.cfg`, and can start the game when it closes.
//!
//! Like the game, its GML is transpiled (`generated/`, by `crates/transpile`; regenerate with
//! `scripts/transpile`, never edit by hand). Everything else is the game's: it's a GameMaker 8 program on
//! the same runner, calling the same built-ins and the same `supersound.dll`, so it runs as a
//! [`game::Game`] of its own program table ([`executable`], [`new`]). The only built-in it adds is
//! `execute_program` ([`builtins`]), which the port records instead of starting an executable
//! (`docs/quirks.md` section 7).

pub mod builtins;
#[rustfmt::skip]
mod generated;
pub mod overrides;
mod rt;

use std::sync::{Arc, LazyLock};

use game::{Config, Executable, Game};
use gm::Program;

static PROGRAM: LazyLock<Program> = LazyLock::new(generated::build_program);

/// The settings program's program table.
pub fn program() -> &'static Program {
    &PROGRAM
}

/// The settings program, for [`Game::of`].
pub fn executable() -> Executable {
    Executable {
        program: program(),
        last_instance_id: generated::LAST_INSTANCE_ID,
        font_slots: generated::FONT_SLOTS,
        start: game::Start::First,
    }
}

/// The settings program on its own assets (the bundle of `config.gmk`), before its first room. It starts
/// like the game ([`Game::start`]), in `rInit`.
pub fn new(assets: Arc<gm::Assets>, config: Config) -> Game {
    Game::of(executable(), assets, config)
}

/// What the settings program's "start game on exit" asks for: `execute_program(global.spelunkyEXE, ...)`
/// names this.
pub const SPELUNKY_EXE: &str = "Spelunky.exe";

/// Whether the settings program has asked to start the game (`execute_program("Spelunky.exe", ...)`).
/// The frontend starts it once the settings program has ended.
pub fn launches_game(settings: &Game) -> bool {
    settings
        .executed()
        .iter()
        .any(|(prog, _)| prog.eq_ignore_ascii_case(SPELUNKY_EXE))
}
