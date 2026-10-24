//! What generated code sees (`use crate::rt::*;` in every file under `generated/`): the `gm` runtime API, the
//! name modules, and a few one-line helpers that keep the generated code short. The same as the game's
//! (`crates/game/src/rt.rs`); the settings program's small amount of GML doesn't use all of it.
#![allow(dead_code, unused_imports)]

pub use crate::builtins as b;
pub use crate::generated::names::{bgr, gv, k, obj, rm, scr_id, spr, v};
pub use crate::overrides::scr;
pub use gm::{Ctx, Local, R, Value, World, case_eq, ix, lenient, repeat_count};

/// A real literal.
#[inline(always)]
pub const fn r(x: f64) -> Value {
    Value::Real(x)
}

/// A comparison's result as a GML value (1 or 0).
#[inline(always)]
pub fn b2v(b: bool) -> Value {
    Value::from(b)
}

/// GML `and` on already evaluated operands: GM8 doesn't short-circuit, so the generated code evaluates
/// both sides before calling this.
#[inline(always)]
pub fn and(a: bool, b: bool) -> bool {
    a & b
}

/// GML `or`, without short-circuit.
#[inline(always)]
pub fn or(a: bool, b: bool) -> bool {
    a | b
}

/// GML `xor` (Spelunky never uses it).
#[inline(always)]
pub fn xor(a: bool, b: bool) -> bool {
    a ^ b
}

/// A string literal. Each use site builds its value once, in an immutable cache, so evaluating it again is
/// a reference-count increment, not an allocation (runtime.md section 3). The empty string needs no
/// allocation at all.
macro_rules! s {
    ("") => {
        gm::Value::Str(gm::GmString::default())
    };
    ($lit:literal) => {{
        static S: std::sync::LazyLock<gm::Value> =
            std::sync::LazyLock::new(|| gm::Value::from($lit));
        S.clone()
    }};
}
pub(crate) use s;
