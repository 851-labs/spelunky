//! The built-ins the settings program calls: the game's ([`game::builtins`]; all 43 it shares are
//! implemented there), plus `execute_program`, which only `config.gmk` calls.

pub use game::builtins::*;

use game::HostState;
use gm::{Ctx, Error, R, Value, World};

/// `execute_program(prog, arg, wait)`: GM8 starts `prog` with the command line `arg`, waiting for it to
/// end if `wait`. The port can't start an executable, and a program it starts isn't part of this one's
/// simulation, so it records the call in the world's [`HostState::executed`] and returns at once; the
/// frontend starts the game when the settings program ends (`docs/quirks.md` section 7). `scrExit`, the
/// only caller, passes `wait = false` and calls `game_end()` next. Waiting has nothing to wait for here,
/// so it's an error rather than a silent no-op.
pub fn execute_program(g: &mut World, _: &Ctx, [prog, arg, wait]: [Value; 3]) -> R<Value> {
    if wait.is_truthy() {
        return Err(Error::Unsupported(
            "execute_program waiting for the program to end",
        ));
    }
    let host = HostState::get_mut(g).ok_or(Error::Unsupported(
        "execute_program in a world without the game's host state",
    ))?;
    // `scrExit` passes the real 0 for `arg`, which reads as the empty string, like any real given to a
    // string argument. Only the program name matters to the frontend.
    let text = |v: &Value| String::from_utf8_lossy(v.to_str()).into_owned();
    let (prog, arg) = (text(&prog), text(&arg));
    host.executed.push((prog, arg));
    Ok(Value::Real(0.0))
}
