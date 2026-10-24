//! What generated code sees (`use crate::rt::*;` in every file under `generated/`): the game's
//! program-independent helpers (`game::transpiled`: the `gm` runtime API, literals, `and`/`or`, `s!`), and
//! the settings program's name modules, built-ins and scripts. Its small amount of GML doesn't use all of
//! them.
#![allow(unused_imports)]

pub use crate::builtins as b;
pub use crate::generated::names::{bgr, gv, k, obj, rm, scr_id, spr, v};
pub use crate::overrides::scr;
pub(crate) use game::s;
pub use game::transpiled::*;
