//! What generated code sees (`use crate::rt::*;` in every file under `generated/`): the program-independent
//! helpers ([`crate::transpiled`]: the `gm` runtime API, literals, `and`/`or`, `s!`), and the game's name
//! modules, built-ins and scripts.

pub use crate::builtins as b;
// Not every kind of name is used (no script is referenced as a value), so some imports are unused.
#[allow(unused_imports)]
pub use crate::generated::names::{bgr, gv, k, obj, rm, scr_id, spr, v};
pub use crate::overrides::scr;
pub(crate) use crate::s;
// Spelunky never uses `xor`.
#[allow(unused_imports)]
pub use crate::transpiled::*;
