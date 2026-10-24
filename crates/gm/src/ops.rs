//! The statement-level GML rules generated code needs on top of [`Value`]'s operators: array indices,
//! `switch` matching, `op=` on user variables and `repeat` counts (runtime.md section 13).

use crate::error::{Error, R};
use crate::value::{Value, ValueError};

/// GM8 arrays are 2-D: `a[i, j]` is stored at `i * ARRAY_STRIDE + j`, and `a[j]` is `a[0, j]`.
pub const ARRAY_STRIDE: u32 = 32000;

/// An array index as GM8 computes it: rounded like a built-in's integer argument (strings read as 0), then
/// range-checked to `0..32000`. OpenGMK `gml/runtime.rs` (`get_array_index`, via `Value::round`).
#[inline]
pub fn ix(v: &Value) -> R<u32> {
    let i = v.to_int();
    if (0..ARRAY_STRIDE as i32).contains(&i) {
        Ok(i as u32)
    } else {
        Err(Error::BadArrayIndex(i))
    }
}

/// The flat storage index of `a[i, j]` (both already checked by [`ix`]).
#[inline]
pub(crate) fn flat(i: u32, j: u32) -> u32 {
    i * ARRAY_STRIDE + j
}

/// `switch` matches a case by exact equality: reals bit-for-bit as `==` on `f64` (no epsilon), strings by
/// bytes, and never across types. OpenGMK `gml/value.rs` (`Value::almost_equals`, despite the name).
#[inline]
pub fn case_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Real(a), Value::Real(b)) => a == b,
        (Value::Str(a), Value::Str(b)) => a == b,
        _ => false,
    }
}

/// `v op= x`: GM8 ignores a type error and keeps the old value, on every name (user variables, globals,
/// locals, built-ins and the arguments; `tools/gm8-probe/probes/opassign.gml`). OpenGMK's
/// `gml/compiler.rs` (`make_modify_instruction`) sets `type_unsafe` only for fields, and
/// `gml/runtime.rs` (`Node::Binary`) keeps the old value when it's set; the oracle's patch 0012 sets it
/// for built-ins and the arguments too.
#[inline]
pub fn lenient(
    op: fn(&Value, &Value) -> Result<Value, ValueError>,
    old: Value,
    rhs: &Value,
) -> Value {
    op(&old, rhs).unwrap_or(old)
}

/// `repeat (n)` runs `round(n)` times (half to even, strings 0, negative counts none). OpenGMK
/// `gml/runtime.rs` (`Instruction::Repeat`).
#[inline]
pub fn repeat_count(v: &Value) -> u32 {
    v.to_int().max(0) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indices_round_half_to_even_and_check_range() {
        assert_eq!(ix(&Value::Real(2.5)), Ok(2));
        assert_eq!(ix(&Value::Real(3.5)), Ok(4));
        assert_eq!(ix(&Value::Real(-0.4)), Ok(0));
        assert_eq!(ix(&Value::from("a")), Ok(0));
        assert_eq!(ix(&Value::Real(31999.0)), Ok(31999));
        assert_eq!(ix(&Value::Real(32000.0)), Err(Error::BadArrayIndex(32000)));
        assert_eq!(ix(&Value::Real(-1.0)), Err(Error::BadArrayIndex(-1)));
        assert_eq!(flat(2, 5), 64005);
    }

    #[test]
    fn switch_matching_is_exact() {
        assert!(case_eq(&Value::Real(1.0), &Value::Real(1.0)));
        assert!(!case_eq(&Value::Real(0.3), &Value::Real(0.1 + 0.2)));
        assert!(case_eq(&Value::from("a"), &Value::from("a")));
        assert!(!case_eq(&Value::Real(0.0), &Value::from("")));
    }

    #[test]
    fn lenient_keeps_the_old_value_on_type_errors() {
        assert_eq!(
            lenient(Value::add, Value::Real(1.0), &Value::Real(2.0)),
            Value::Real(3.0)
        );
        assert_eq!(
            lenient(Value::sub, Value::from("a"), &Value::Real(2.0)),
            Value::from("a")
        );
    }

    #[test]
    fn repeat_counts_round() {
        assert_eq!(repeat_count(&Value::Real(2.5)), 2);
        assert_eq!(repeat_count(&Value::Real(2.6)), 3);
        assert_eq!(repeat_count(&Value::Real(-3.0)), 0);
        assert_eq!(repeat_count(&Value::from("9")), 0);
    }
}
