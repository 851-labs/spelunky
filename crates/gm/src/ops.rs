//! The statement-level GML rules generated code needs on top of [`Value`]'s operators: array indices,
//! `switch` matching, `op=` on user variables and `repeat` counts (runtime.md section 13).

use crate::error::{Error, R};
use crate::value::{Value, ValueError};

/// GM8 arrays are 2-D: `a[i, j]` is stored at `i * ARRAY_STRIDE + j`, and `a[j]` is `a[0, j]`.
pub const ARRAY_STRIDE: u32 = 32000;

/// An array index as GM8 computes it: rounded like a built-in's integer argument (strings read as 0; past the
/// `Int64` range an error, [`Value::to_int`]), then range-checked to `0..32000`. OpenGMK `gml/runtime.rs`
/// (`get_array_index`, via `Value::round`).
#[inline]
pub fn ix(v: &Value) -> R<u32> {
    let i = v.to_int()?;
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
///
/// It keeps the old value on a zero divisor too (`v /= 0`, probes `divassign-*`), but not on an exception
/// ([`ValueError::is_exception`]): on a real Intel CPU `v *= 2` past the largest double and `v |= 2^63` trap as
/// `v * 2` and `v | 2^63` do (probes `mulassign-overflow-*`, `orassign-*`), so those errors go through. OpenGMK
/// keeps the old value on every error; the oracle's patch 0014 lets those two through as well. A string too
/// long to allocate (`v *= "a"` with `v` past 304 MiB, 851-3282, or 2^31, 851-3272) goes through too: GM8
/// running out of memory is an exception like the x87's traps (probes `mulstr-39x2p23`, `mulstr-2p31`,
/// `late-mulstr`) rather than a type check (assumed; not probed in an `op=`), and the oracle reports it as an
/// overflow.
#[inline]
pub fn lenient(
    op: fn(&Value, &Value) -> Result<Value, ValueError>,
    old: Value,
    rhs: &Value,
) -> Result<Value, ValueError> {
    match op(&old, rhs) {
        Ok(new) => Ok(new),
        Err(e) if e.is_exception() => Err(e),
        Err(_) => Ok(old),
    }
}

/// `repeat (n)` runs `round(n)` times, an integer argument ([`Value::to_int`]: half to even, cut to 32 bits,
/// strings 0, an error past the `Int64` range); negative counts run none, but -2^31 runs 2^31 times
/// ([`crate::gm8::repeat_count`]). GM8 (probes `int-args`, `repeat-2p63`); OpenGMK `gml/runtime.rs`
/// (`Instruction::Repeat`) doesn't cut to 32 bits.
#[inline]
pub fn repeat_count(v: &Value) -> R<u32> {
    Ok(crate::gm8::repeat_count(v.to_int()?))
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
        // Past the Int64 range GM8 on Intel traps (probe `array-2p63`), where Rosetta reads -2^63, negative.
        assert_eq!(
            ix(&Value::Real(9223372036854775808.0)),
            Err(Error::Value(ValueError::IntOverflow))
        );
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
            Ok(Value::Real(3.0))
        );
        assert_eq!(
            lenient(Value::sub, Value::from("a"), &Value::Real(2.0)),
            Ok(Value::from("a"))
        );
        assert_eq!(
            lenient(Value::div, Value::Real(1.0), &Value::Real(0.0)),
            Ok(Value::Real(1.0))
        );
        // The x87's traps aren't type errors: GM8 on Intel stops (probes `mulassign-overflow-*`, `orassign-*`).
        assert_eq!(
            lenient(Value::mul, Value::Real(f64::MAX), &Value::Real(2.0)),
            Err(ValueError::Overflow)
        );
        assert_eq!(
            lenient(Value::bit_or, Value::Real(1.0), &Value::Real(1e20)),
            Err(ValueError::IntOverflow)
        );
        // Nor is a string too long to allocate (`v *= "a"` with `v` 2^31: 2^31 copies).
        assert_eq!(
            lenient(Value::mul, Value::Real(2_147_483_648.0), &Value::from("a")),
            Err(ValueError::StringTooLong)
        );
    }

    #[test]
    fn repeat_counts_round() {
        assert_eq!(repeat_count(&Value::Real(2.5)), Ok(2));
        assert_eq!(repeat_count(&Value::Real(2.6)), Ok(3));
        assert_eq!(repeat_count(&Value::Real(-3.0)), Ok(0));
        assert_eq!(repeat_count(&Value::from("9")), Ok(0));
        // Cut to 32 bits (probe `int-args`): 2^32 + 2 runs twice, 2^31 (-2^31) runs.
        let p31 = 2147483648.0;
        assert_eq!(repeat_count(&Value::Real(2.0 * p31 + 2.0)), Ok(2));
        assert_eq!(repeat_count(&Value::Real(p31)), Ok(1 << 31));
        assert_eq!(repeat_count(&Value::Real(-p31)), Ok(1 << 31));
        assert_eq!(repeat_count(&Value::Real(2.0 * p31 - 3.0)), Ok(0));
        assert_eq!(
            repeat_count(&Value::Real(f64::INFINITY)),
            Err(Error::Value(ValueError::IntOverflow))
        );
    }
}
