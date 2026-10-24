//! GML real-number semantics and the math built-ins Spelunky uses (851-3125).
//!
//! Everything here works on plain `f64` so hand-ported game code can call it without boxing values in
//! [`crate::Value`]. The GM8 runner is Delphi, but the reference oracle is OpenGMK, which models Delphi's
//! `Double` as an `f64` with ordinary IEEE-754 double arithmetic (OpenGMK `gm8emulator/src/math.rs`, type
//! `Real`). We match that, so each function below is one IEEE operation or a short fixed sequence of them.
//!
//! Sources are cited per item: "OpenGMK `kernel.rs`" means `gm8emulator/src/gml/kernel.rs`, "OpenGMK
//! `value.rs`" means `gm8emulator/src/gml/value.rs`, and "OpenGMK `math.rs`" means `gm8emulator/src/math.rs`.
//!
//! # Transcendental functions (851-3167)
//!
//! `+ - * /`, `sqrt`, `floor`, `round_ties_even`, `%` and friends are exact IEEE operations, so they give
//! the same bits everywhere. `sin`, `cos`, `atan` and `atan2` are not: Rust's `f64::sin` calls the
//! platform's libm (Apple's on macOS, a musl port on wasm32, glibc on Linux), and those disagree in
//! the last bit on a few percent of the angles the game produces. So every transcendental here goes
//! through `pxfm`, a pure-Rust port of CORE-MATH whose results are *correctly rounded*: there is exactly
//! one right answer per input, so native, wasm32 and any other target agree bit for bit. The workspace
//! `clippy.toml` bans the platform's float methods, so game code can't bypass this module.
//!
//! `pxfm` is vendored (`vendor/pxfm`, see its `README.spelunky.md`), trimmed to these functions, with two
//! patches. Upstream `atan`/`atan2` choose an AVX+FMA variant at run time on x86 through a `static
//! OnceLock`, which is hidden global state, so ours always take the generic path. And upstream `atan2`
//! returns +0 where IEEE gives -0 (a negative result that underflows), which ours corrects. Code paths
//! still differ by target at compile time (hardware FMA on aarch64 or with `+fma`, separate multiply and
//! add elsewhere), but being correctly rounded, they must agree. `tools/math-probe` checked that on about
//! 22M game-like inputs: identical bits on macOS aarch64, Linux aarch64, wasm32, x86_64 with and without
//! `+fma`, and upstream's own AVX+FMA variant.
//!
//! The oracle uses this same vendored crate for OpenGMK's trigonometry (851-3173, its patch 0007), so the
//! two agree bit for bit. Unpatched, OpenGMK would use glibc, which is correctly rounded on all but about
//! 0.1% (`sin`, `cos`, `atan2`) to 0.2% (`atan`) of the game's inputs and 1 ulp off there; the `libm`
//! crate is worse, disagreeing with glibc on about 3% of `sin`/`cos` and 17% of `atan2` inputs.
//! `tools/math-probe` measures this; see its README. Real GM8 used the x87 FPU, which neither matches.
//!
//! `hypot` (for `distance_to_object` and `distance_to_point`) is the same story (851-3174): glibc, Apple's
//! libm, the `libm` crate and upstream pxfm are each 1 ulp off on 12-21% of `distance_to_point`'s
//! fractional arguments. The vendored pxfm's `f_hypot` is our own, correctly rounded and checked against
//! mpmath on about 7.5M inputs, and the oracle uses it too (patch 0008).

use crate::gm8;
use crate::value::ValueError;

/// GML `true`.
pub const TRUE: f64 = 1.0;
/// GML `false`.
pub const FALSE: f64 = 0.0;

/// GM8 compares reals with this tolerance, so `0.1 + 0.2 == 0.3` holds. OpenGMK `math.rs`
/// (`Real::CMP_EPSILON`), used by the comparison operators in OpenGMK `value.rs` and by `sign`.
pub const CMP_EPSILON: f64 = 1e-13;

/// `true`/`false` as a GML real.
#[inline]
pub const fn from_bool(b: bool) -> f64 {
    if b { TRUE } else { FALSE }
}

/// GML truthiness: a real is true when it is at least 0.5, so `0.5` is true and `0.49` is false. NaN is
/// false. OpenGMK `value.rs` (`Value::is_truthy`), used for `if`, `while`, `not`, `and`/`or`/`xor` and
/// boolean built-in variables.
#[inline]
pub fn is_truthy(x: f64) -> bool {
    x >= 0.5
}

/// GML `a == b` on reals: `|a - b| < 1e-13`. False when `a - b` is NaN (a NaN operand, or `inf == inf`).
#[inline]
pub fn eq(a: f64, b: f64) -> bool {
    (a - b).abs() < CMP_EPSILON
}

/// GML `a != b` on reals: `!(a == b)`, so it is *true* when `a - b` is NaN. GM8 (probe `nonfinite`,
/// `ops`), not OpenGMK, which makes every comparison with a NaN false (oracle patch 0014 follows GM8).
#[inline]
pub fn ne(a: f64, b: f64) -> bool {
    !eq(a, b)
}

/// GML `a < b` on reals: `a - b <= -1e-13`. False when `a - b` is NaN.
#[inline]
pub fn lt(a: f64, b: f64) -> bool {
    a - b <= -CMP_EPSILON
}

/// GML `a <= b` on reals: `a - b < 1e-13`. False when `a - b` is NaN.
#[inline]
pub fn le(a: f64, b: f64) -> bool {
    a - b < CMP_EPSILON
}

/// GML `a > b` on reals: `!(a <= b)`, so like `!=` it is true when `a - b` is NaN (GM8, probe `ops`:
/// `nan > 0`, `0 > nan` and `inf > inf` are all true).
#[inline]
pub fn gt(a: f64, b: f64) -> bool {
    !le(a, b)
}

/// GML `a >= b` on reals: `!(a < b)`, true when `a - b` is NaN (GM8).
#[inline]
pub fn ge(a: f64, b: f64) -> bool {
    !lt(a, b)
}

/// GML `round(x)`: round half to even, like Delphi's `Round`, into an `Int64`: so +0 for `-0.5` (and for
/// -0). From 2^63 up, below -2^63, and for infinities and NaN, the x87 traps (GM8 runs with its
/// invalid-operation exception unmasked): [`ValueError::IntOverflow`]. GM8 (probes `numbers`, `ops`; the trap
/// on a real Intel CPU, `floor(1e20)` in `numbers`, where Rosetta gives -2^63 instead); OpenGMK returns the
/// rounded real itself (oracle patch 0014 follows GM8). The runner's own rounding (snapping, collision
/// positions) is [`round_half_even`].
#[inline]
pub fn round(x: f64) -> Result<f64, ValueError> {
    gm8::round(x).ok_or(ValueError::IntOverflow)
}

/// The runner's internal rounding: half to even on the real, as OpenGMK's `Real::round` (`math.rs`,
/// `ieee_round`). That computes `floor(x)` plus a correction, so a negative `x` that rounds to zero gives
/// +0, not -0; only -0 itself stays -0. Not GML's `round` ([`round`]), which differs past 2^63.
#[inline]
pub fn round_half_even(x: f64) -> f64 {
    let r = x.round_ties_even();
    if r == 0.0 && x != 0.0 { 0.0 } else { r }
}

/// How the runner turns a real into an integer argument (`round`, then truncate to 32 bits). OpenGMK
/// `value.rs` (`From<Value> for i32`) and `math.rs` (`Real::to_i32`: through `i64`, so it wraps rather
/// than saturates past the `i32` range). Out-of-range `i64` values and NaN saturate as Rust casts do.
#[inline]
pub fn to_int(x: f64) -> i32 {
    round_half_even(x) as i64 as i32
}

/// GML `a / b`. Dividing by zero (+0 or -0) is a runtime error, "Division by 0."; any other divisor
/// divides, so `1 / 1e-300` is 1e300 and an overflow is ±inf. GM8 (probes `div0`, `div-negzero`,
/// `div-tiny`); OpenGMK gives ±inf or NaN (oracle patch 0014 follows GM8).
#[inline]
pub fn div(a: f64, b: f64) -> Result<f64, ValueError> {
    if b == 0.0 {
        Err(ValueError::DivisionByZero)
    } else {
        Ok(a / b)
    }
}

/// GML `a div b`: `a / b` truncated toward zero to a 32-bit integer, so `-7 div 2 == -3`, `7 div 2.5 == 2`
/// and `2^31 div 1 == -2^31`. A zero divisor is an error, "Division by 0.", and a quotient from 2^63 up is
/// [`round`]'s trap. GM8 (probes `numbers`, `ops`, `intdiv0`, `intdiv-frac`); OpenGMK floors and doesn't
/// wrap (oracle patch 0014 follows GM8). The game never uses `div`.
#[inline]
pub fn int_div(a: f64, b: f64) -> Result<f64, ValueError> {
    if b == 0.0 {
        return Err(ValueError::DivisionByZero);
    }
    gm8::quotient(a, b).ok_or(ValueError::IntOverflow)
}

/// GML `a mod b`: `a - (a div b) * b`, with `div`'s 32-bit quotient, so the sign follows `a`, and past 2^31
/// the wrapped quotient gives results bigger than `b` (`(2^40 + 3) mod 7` is 1112396529669). A zero divisor
/// is an error, "Modulo by 0.". Being computed through a division, it can differ from IEEE `fmod`
/// (Rust's `%`) in the last bits. GM8 (probes `numbers`, `ops`, `mod0`, `mod-tiny`); OpenGMK truncates
/// without wrapping (oracle patch 0014 follows GM8).
#[inline]
pub fn modulo(a: f64, b: f64) -> Result<f64, ValueError> {
    if b == 0.0 {
        return Err(ValueError::ModuloByZero);
    }
    let q = gm8::quotient(a, b).ok_or(ValueError::IntOverflow)?;
    Ok(a - q * b)
}

/// A bitwise operand: rounded half to even into a signed 64-bit integer, or [`round`]'s trap.
#[inline]
fn bits(x: f64) -> Result<i64, ValueError> {
    gm8::bits(x).ok_or(ValueError::IntOverflow)
}

/// GML `a & b`. Bitwise operands are rounded half to even into a signed 64-bit integer; from 2^63 up (and
/// NaN) it's [`round`]'s trap. GM8 (probes `numbers`, `ops`: `-1 & 255 == 255`, `-2.5 | 0 == -2`); OpenGMK
/// saturates to an unsigned one, making negatives 0 (oracle patch 0014 follows GM8). The game uses no
/// bitwise operators.
#[inline]
pub fn bit_and(a: f64, b: f64) -> Result<f64, ValueError> {
    Ok((bits(a)? & bits(b)?) as f64)
}

/// GML `a | b`.
#[inline]
pub fn bit_or(a: f64, b: f64) -> Result<f64, ValueError> {
    Ok((bits(a)? | bits(b)?) as f64)
}

/// GML `a ^ b`.
#[inline]
pub fn bit_xor(a: f64, b: f64) -> Result<f64, ValueError> {
    Ok((bits(a)? ^ bits(b)?) as f64)
}

/// GML `~a`: the 64-bit complement, so `~0 == -1` and `~5 == -6`.
#[inline]
pub fn bit_not(a: f64) -> Result<f64, ValueError> {
    Ok((!bits(a)?) as f64)
}

/// GML `a << b`. The shift count is its low byte, read as a signed byte `c`: for `c < 32`, negatives
/// included, the shift is by `c & 31`; for `32 <= c < 64`, by `c`; from 64 up the result is 0. So `1 << 255`
/// is 2^31 (255 is -1), `1 << 256` is 1 and `-1 >> 300` is `-1 >> 44` (GM8, probe `ops`).
#[inline]
pub fn shl(a: f64, b: f64) -> Result<f64, ValueError> {
    gm8::shift(a, b, true).ok_or(ValueError::IntOverflow)
}

/// GML `a >> b`: logical, on the 64 bits, so `-2 >> 1 == 2^63 - 1`, with [`shl`]'s shift count.
#[inline]
pub fn shr(a: f64, b: f64) -> Result<f64, ValueError> {
    gm8::shift(a, b, false).ok_or(ValueError::IntOverflow)
}

/// GML `abs(x)`.
#[inline]
pub fn abs(x: f64) -> f64 {
    x.abs()
}

/// GML `floor(x)`: Delphi's `Floor`, which returns a 32-bit `Integer`: `Trunc(x)`, minus 1 if `Frac(x) < 0`.
/// So it wraps from 2^31 on (`floor(2^31 + 0.5) == -2^31`, `floor(2^32 + 0.5) == 0`), and from 2^63 up
/// (and for infinities and NaN) `Trunc`'s `FISTP` traps, as in [`round`]: "Unexpected error occured when
/// running the game." on a real Intel CPU (probe `numbers`, `floor(1e20)`). GM8 (probes `numbers`, `ops`);
/// OpenGMK is exact (oracle patch 0014 follows GM8).
#[inline]
pub fn floor(x: f64) -> Result<f64, ValueError> {
    gm8::floor(x).ok_or(ValueError::IntOverflow)
}

/// GML `ceil(x)`: Delphi's `Ceil`, `Trunc(x)` plus 1 if `Frac(x) > 0`, as a 32-bit `Integer` like
/// [`floor`], with the same trap. `ceil(-0.5)` is +0. GM8 (probes `numbers`, `ops`).
#[inline]
pub fn ceil(x: f64) -> Result<f64, ValueError> {
    gm8::ceil(x).ok_or(ValueError::IntOverflow)
}

/// GML `frac(x)`: `x - trunc(x)`, so the sign follows `x` (`frac(-1.25) == -0.25`), like Delphi's `Frac`.
/// OpenGMK `kernel.rs` (`frac`, via `f64::fract`).
#[inline]
pub fn frac(x: f64) -> f64 {
    x.fract()
}

/// GML `sign(x)`: uses the comparison tolerance, so `sign(1e-14) == 0`, and a NaN is 1 (GM8, probe `ops`).
/// OpenGMK `kernel.rs` (`sign`), which gives 0 for a NaN.
#[inline]
pub fn sign(x: f64) -> f64 {
    gm8::sign(x)
}

/// GML `sqr(x)`: `x * x`.
#[inline]
pub fn sqr(x: f64) -> f64 {
    x * x
}

/// GML `sqrt(x)`. A result that isn't finite (a negative or infinite argument) is a runtime error: GM8 on a
/// real Intel CPU ("Cannot apply sqrt to negative number.", probe `sqrt-neg`; under Rosetta, which doesn't
/// trap, it's a NaN) and OpenGMK `kernel.rs` (`sqrt`).
#[inline]
pub fn sqrt(x: f64) -> Result<f64, ValueError> {
    let r = x.sqrt();
    if r.is_finite() {
        Ok(r)
    } else {
        Err(ValueError::Domain("sqrt"))
    }
}

/// GML `sin(x)` (radians), correctly rounded. OpenGMK `kernel.rs` (`sin`, via `f64::sin`).
#[inline]
pub fn sin(x: f64) -> f64 {
    pxfm::f_sin(x)
}

/// GML `cos(x)` (radians), correctly rounded. OpenGMK `kernel.rs` (`cos`, via `f64::cos`).
#[inline]
pub fn cos(x: f64) -> f64 {
    pxfm::f_cos(x)
}

/// GML `arctan(x)` (radians), correctly rounded. OpenGMK `kernel.rs` (`arctan`, via `f64::atan`).
#[inline]
pub fn arctan(x: f64) -> f64 {
    pxfm::f_atan(x)
}

/// `atan2(y, x)` in radians, correctly rounded, for `point_direction` and the runner's `direction`. The
/// game never calls GML `arctan2`. (The vendored pxfm fixes upstream's sign of an underflowing negative
/// result: -0, as in IEEE and glibc.)
#[inline]
pub fn atan2(y: f64, x: f64) -> f64 {
    pxfm::f_atan2(y, x)
}

/// GML `degtorad(x)`: `x * (pi / 180)`, exactly as `f64::to_radians` computes it. OpenGMK `kernel.rs`
/// (`degtorad`, via `Real::to_radians`).
#[inline]
pub fn degtorad(x: f64) -> f64 {
    x.to_radians()
}

/// GML `radtodeg(x)`: `x * (180 / pi)`, exactly as `f64::to_degrees` computes it. OpenGMK `kernel.rs`
/// (`radtodeg`).
#[inline]
pub fn radtodeg(x: f64) -> f64 {
    x.to_degrees()
}

/// GML `point_distance(x1, y1, x2, y2)`: `sqrt(dx*dx + dy*dy)`, not `hypot`, so the rounding matches.
/// OpenGMK `kernel.rs` (`point_distance`).
#[inline]
pub fn point_distance(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    (dx * dx + dy * dy).sqrt()
}

/// `hypot(x, y)`, the length `sqrt(x*x + y*y)`, correctly rounded (851-3174), for the runner's
/// `distance_to_object` and `distance_to_point`, which call `f64::hypot` (OpenGMK `kernel.rs`), unlike
/// `point_distance`. For `distance_to_object`'s whole bounding-box gaps that's the same as
/// `sqrt(x*x + y*y)`, but for `distance_to_point`'s fractional ones (`oVampire`) neither that nor any
/// platform's `hypot` (glibc, Apple's, the `libm` crate) is always correctly rounded. So, as for the
/// trigonometry, the port and the oracle (patch 0008) both call the vendored pxfm's `f_hypot`, which we
/// wrote to be correctly rounded.
#[inline]
pub fn hypot(x: f64, y: f64) -> f64 {
    pxfm::f_hypot(x, y)
}

/// GML `point_direction(x1, y1, x2, y2)`: degrees counterclockwise with y pointing down, in [0, 360]. It
/// is `atan2(y1 - y2, x2 - x1)` in degrees, wrapped with `rem_euclid(360)`; a tiny negative angle wraps to
/// exactly 360. OpenGMK `kernel.rs` (`point_direction`).
#[inline]
pub fn point_direction(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    atan2(y1 - y2, x2 - x1).to_degrees().rem_euclid(360.0)
}

/// GML `make_color_rgb(r, g, b)`: `r | g << 8 | b << 16` on the rounded integer arguments, with no
/// clamping, so out-of-range components bleed into each other. OpenGMK `kernel.rs` (`make_color_rgb`).
#[inline]
pub fn make_color_rgb(r: i32, g: i32, b: i32) -> f64 {
    f64::from(r | (g << 8) | (b << 16))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truthiness_threshold_is_half_inclusive() {
        assert!(is_truthy(0.5));
        assert!(is_truthy(1.0));
        assert!(!is_truthy(0.49999999));
        assert!(!is_truthy(0.0));
        assert!(!is_truthy(-1.0));
        assert!(!is_truthy(f64::NAN));
    }

    #[test]
    fn comparisons_use_epsilon() {
        assert_eq!(
            0.1 + 0.2,
            0.300_000_000_000_000_04,
            "the IEEE sum is not exactly 0.3"
        );
        assert!(eq(0.1 + 0.2, 0.3));
        assert!(!ne(0.1 + 0.2, 0.3));
        assert!(!lt(0.3, 0.1 + 0.2));
        assert!(le(0.1 + 0.2, 0.3));
        assert!(!gt(0.1 + 0.2, 0.3));
        assert!(ge(0.3, 0.1 + 0.2));
        // Just inside and just outside the tolerance.
        assert!(eq(1.0, 1.0 + 0.9e-13));
        assert!(!eq(1.0, 1.0 + 1.1e-13));
        assert!(lt(1.0, 1.0 + 1.1e-13));
        assert!(!lt(1.0, 1.0 + 0.9e-13));
        assert!(gt(1.0 + 1.1e-13, 1.0));
        // Repeated += 0.2 reaches 19 under GML equality (OpenGMK math.rs test `prec_19`).
        let mut x = 0.2;
        let mut steps = 1;
        while !eq(x, 19.0) {
            x += 0.2;
            steps += 1;
            assert!(steps < 200);
        }
        assert_eq!(steps, 95);
    }

    #[test]
    fn a_nan_difference_makes_ne_gt_and_ge_true() {
        let (n, inf) = (f64::NAN, f64::INFINITY);
        for (a, b) in [(n, n), (n, 0.0), (0.0, n), (inf, inf)] {
            assert!(!eq(a, b) && !lt(a, b) && !le(a, b), "{a} {b}");
            assert!(ne(a, b) && gt(a, b) && ge(a, b), "{a} {b}");
        }
        assert!(gt(inf, -inf) && !gt(-inf, inf));
    }

    #[test]
    fn round_is_bankers_into_an_int64() {
        assert_eq!(round(0.5), Ok(0.0));
        assert_eq!(round(1.5), Ok(2.0));
        assert_eq!(round(2.5), Ok(2.0));
        assert_eq!(round(-0.5), Ok(0.0));
        assert_eq!(round(-1.5), Ok(-2.0));
        assert_eq!(round(-2.5), Ok(-2.0));
        assert_eq!(round(2.5000001), Ok(3.0));
        assert_eq!(round(-13.37), Ok(-13.0));
        // An integer has no -0.
        assert!(round(-0.4).unwrap().is_sign_positive() && round(-0.0).unwrap().is_sign_positive());
        // Exact in the Int64 range; outside it, FISTP's invalid-operation exception, which GM8 doesn't mask.
        let p63 = 9_223_372_036_854_775_808.0;
        assert_eq!(
            round(4_611_686_018_427_388_928.0),
            Ok(4_611_686_018_427_388_928.0)
        );
        for x in [
            p63,
            2.0 * p63,
            1e20,
            -1e20,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
        ] {
            assert_eq!(round(x), Err(ValueError::IntOverflow), "{x}");
        }
        assert_eq!(round(-p63), Ok(-p63));
        // The runner's own rounding keeps the real.
        assert_eq!(round_half_even(1e20), 1e20);
        assert!(round_half_even(-0.0).is_sign_negative());
        assert!(round_half_even(-0.4).is_sign_positive());
        assert_eq!(to_int(2.5), 2);
        assert_eq!(to_int(3.5), 4);
        // Through i64, so past the i32 range it wraps instead of saturating.
        assert_eq!(to_int(4_294_967_297.0), 1);
        assert_eq!(to_int(f64::NAN), 0);
    }

    #[test]
    fn floor_and_ceil_are_32_bit() {
        let (p31, p32) = (2_147_483_648.0, 4_294_967_296.0);
        assert_eq!(floor(-2.5), Ok(-3.0));
        assert_eq!(ceil(-2.5), Ok(-2.0));
        assert_eq!(floor(p31 - 0.5), Ok(p31 - 1.0));
        assert_eq!(floor(p31 + 0.5), Ok(-p31));
        assert_eq!(floor(p32 + 0.5), Ok(0.0));
        assert_eq!(floor(p32 - 0.5), Ok(-1.0));
        assert_eq!(floor(-p31 - 0.5), Ok(p31 - 1.0));
        assert_eq!(ceil(p31 - 0.5), Ok(-p31));
        assert_eq!(ceil(p32 + 0.5), Ok(1.0));
        assert!(ceil(-0.5).unwrap().is_sign_positive() && floor(-0.0).unwrap().is_sign_positive());
        // Trunc's FISTP traps outside the Int64 range (GM8 on Intel: `floor(1e20)` stops the game).
        assert_eq!(floor(9_223_372_036_854_774_784.0), Ok(-1024.0));
        for x in [1e20, -1e20, f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
            assert_eq!(floor(x), Err(ValueError::IntOverflow), "{x}");
            assert_eq!(ceil(x), Err(ValueError::IntOverflow), "{x}");
        }
    }

    #[test]
    fn div_truncates_to_32_bits_and_mod_follows() {
        assert_eq!(int_div(7.0, 2.0), Ok(3.0));
        assert_eq!(int_div(-7.0, 2.0), Ok(-3.0));
        assert_eq!(int_div(7.0, -2.0), Ok(-3.0));
        assert_eq!(int_div(7.0, 2.5), Ok(2.0));
        assert_eq!(int_div(7.0, 0.5), Ok(14.0));
        assert_eq!(int_div(2_147_483_648.0, 1.0), Ok(-2_147_483_648.0));
        assert_eq!(int_div(1_099_511_627_777.0, 3.0), Ok(1_431_655_765.0));
        assert_eq!(int_div(f64::INFINITY, 2.0), Err(ValueError::IntOverflow));
        assert_eq!(int_div(f64::NAN, 2.0), Err(ValueError::IntOverflow));
        assert_eq!(modulo(1e20, 3.0), Err(ValueError::IntOverflow));
        assert_eq!(modulo(7.0, 3.0), Ok(1.0));
        assert_eq!(modulo(-7.0, 3.0), Ok(-1.0));
        assert_eq!(modulo(7.0, -3.0), Ok(1.0));
        assert_eq!(modulo(160.5, 2.25), Ok(0.75));
        assert_eq!(modulo(5.5, 2.0), Ok(1.5));
        assert_eq!(modulo(1_099_511_627_779.0, 7.0), Ok(1_112_396_529_669.0));
        // Computed through a division, not fmod: these differ in the last bits.
        // 1 / 0.1 rounds to exactly 10, so GML gets 0 where fmod sees 0.1's representation error.
        assert_eq!(modulo(1.0, 0.1), Ok(0.0));
        assert_eq!(1.0_f64 % 0.1, 0.09999999999999995);
        // A zero divisor, either sign, is an error; a tiny one isn't.
        assert_eq!(div(1.0, 0.0), Err(ValueError::DivisionByZero));
        assert_eq!(div(1.0, -0.0), Err(ValueError::DivisionByZero));
        assert_eq!(int_div(1.0, 0.0), Err(ValueError::DivisionByZero));
        assert_eq!(modulo(1.0, -0.0), Err(ValueError::ModuloByZero));
        assert_eq!(div(1.0, 1e-300), Ok(1.0 / 1e-300));
        assert_eq!(div(1.0, 5e-324), Ok(f64::INFINITY));
    }

    #[test]
    fn bitwise_rounds_into_signed_64_bits() {
        let p63 = 9_223_372_036_854_775_808.0;
        assert_eq!(bit_and(12.0, 10.0), Ok(8.0));
        assert_eq!(bit_or(12.0, 10.0), Ok(14.0));
        assert_eq!(bit_xor(12.0, 10.0), Ok(6.0));
        assert_eq!(bit_and(2.5, 3.0), Ok(2.0), "2.5 rounds to 2");
        assert_eq!(bit_and(3.5, 7.0), Ok(4.0), "3.5 rounds to 4");
        assert_eq!(bit_and(-1.0, 255.0), Ok(255.0));
        assert_eq!(bit_or(-5.0, 2.0), Ok(-5.0));
        assert_eq!(bit_xor(-5.0, 1.0), Ok(-6.0));
        assert_eq!(bit_or(-p63, 0.0), Ok(-p63));
        assert_eq!(bit_or(p63, 0.0), Err(ValueError::IntOverflow));
        assert_eq!(bit_or(0.0, f64::NAN), Err(ValueError::IntOverflow));
        assert_eq!(shl(p63, 1.0), Err(ValueError::IntOverflow));
        assert_eq!(bit_not(0.0), Ok(-1.0));
        assert_eq!(bit_not(-p63), Ok(p63), "2^63 - 1, rounded");
        assert_eq!(shl(1.0, 4.0), Ok(16.0));
        assert_eq!(shl(1.0, 63.0), Ok(-p63));
        assert_eq!(shl(1.0, 64.0), Ok(0.0));
        assert_eq!(
            shl(1.0, 255.0),
            Ok(2_147_483_648.0),
            "255 is -1: a shift by 31"
        );
        assert_eq!(shl(1.0, 256.0), Ok(1.0), "only the low byte counts");
        assert_eq!(shr(256.0, 4.0), Ok(16.0));
        assert_eq!(shr(-1.0, 63.0), Ok(1.0), "logical");
        assert_eq!(shr(-1.0, 300.0), Ok(1_048_575.0), "300 is 44");
        assert_eq!(shr(-1.0, -1.0), Ok(8_589_934_591.0));
    }

    #[test]
    fn frac_sign_sqr_sqrt() {
        assert_eq!(frac(1.25), 0.25);
        assert_eq!(frac(-1.25), -0.25);
        assert_eq!(sign(3.0), 1.0);
        assert_eq!(sign(-0.001), -1.0);
        assert_eq!(sign(1e-14), 0.0, "sign uses the comparison tolerance");
        assert_eq!(sign(-1e-14), 0.0);
        assert_eq!(sign(f64::NAN), 1.0);
        assert_eq!(sqr(-3.0), 9.0);
        assert_eq!(sqrt(9.0), Ok(3.0));
        assert_eq!(sqrt(-0.0), Ok(-0.0));
        assert_eq!(sqrt(-1.0), Err(ValueError::Domain("sqrt")));
        assert_eq!(sqrt(f64::INFINITY), Err(ValueError::Domain("sqrt")));
        assert_eq!(abs(-2.5), 2.5);
    }

    #[test]
    fn angles() {
        assert_eq!(degtorad(180.0), std::f64::consts::PI);
        assert_eq!(radtodeg(std::f64::consts::PI), 180.0);
        assert_eq!(point_direction(0.0, 0.0, 1.0, 0.0), 0.0);
        assert_eq!(point_direction(0.0, 0.0, 0.0, -1.0), 90.0, "y points down");
        assert_eq!(point_direction(0.0, 0.0, -1.0, 0.0), 180.0);
        assert_eq!(point_direction(0.0, 0.0, 0.0, 1.0), 270.0);
        // A tiny negative angle wraps to exactly 360, not 0.
        assert_eq!(point_direction(0.0, 0.0, 1.0, 1e-300), 360.0);
        assert_eq!(point_distance(0.0, 0.0, 3.0, 4.0), 5.0);
        // The game draws `string(radtodeg(arctan(1)))` in its debug text.
        assert_eq!(radtodeg(arctan(1.0)), 45.0);
    }

    /// Pinned bit patterns, so a platform (or a dependency bump) that rounds differently fails here. Every
    /// expected value is the correctly rounded result, checked with mpmath at 200 bits. The inputs come
    /// from `tools/math-probe` and are game-like angles where glibc (the oracle), the `libm` crate (what
    /// wasm32's `f64::sin` uses), or both return the other neighbouring double, so falling back to either
    /// would fail. This test has no platform-specific code and runs the same on wasm32.
    #[test]
    fn transcendentals_are_correctly_rounded() {
        type Unary = (&'static str, fn(f64) -> f64, u64, u64);
        #[rustfmt::skip]
        let unary: &[Unary] = &[
            // Wrong in glibc.
            ("sin", sin, 0xc027_9f0c_9b09_e3b3, 0x3fe5_f239_549a_3ab3),
            ("sin", sin, 0xc022_a215_243b_ee84, 0xbfbb_a5d8_84bf_16d8),
            ("cos", cos, 0xc027_c579_72e6_e21e, 0x3fe8_de61_3515_a323),
            ("cos", cos, 0xc013_cc8c_3fd2_b893, 0x3fce_1923_02d8_0f01),
            ("arctan", arctan, 0x3fea_a349_572d_aa35, 0x3fe6_36f0_4255_fef8),
            ("arctan", arctan, 0x3fee_6076_b981_dae5, 0x3fe8_4cda_e9e8_4efc),
            // Wrong in glibc and the libm crate.
            ("sin", sin, 0xc023_1b9d_048b_b9e4, 0x3fc0_7c5f_c595_b015),
            ("sin", sin, 0xc018_8121_d88f_4591, 0x3fc4_060b_67a8_537d),
            ("cos", cos, 0xc024_84a1_975a_a519, 0xbfe5_7ebc_2d66_259f),
            ("cos", cos, 0xbffd_f6bc_6c24_e865, 0xbfd3_0830_9fb4_3cc5),
            ("arctan", arctan, 0x3ff6_a0ea_0ea0_ea0f, 0x3fee_9226_97ab_788d),
            ("arctan", arctan, 0xbff6_a0ea_0ea0_ea0f, 0xbfee_9226_97ab_788d),
            // Wrong in the libm crate.
            ("sin", sin, 0xc027_f3f1_2a8c_ec1e, 0x3fe1_cd2c_766f_6c86),
            ("cos", cos, 0xc028_c6d5_6c08_548f, 0x3fef_7e87_c9a3_0807),
            ("arctan", arctan, 0x3ff1_fa3f_47e8_fd1f, 0x3fea_fe3a_41f6_da81),
            // sin(xDiff) after 63 steps of `xDiff += 0.01` (oTarget), wrong in the libm crate.
            ("sin", sin, 0x3fe4_28f5_c28f_5c2c, 0x3fe2_da46_1b7f_19af),
        ];
        for &(name, f, x, expected) in unary {
            let got = f(f64::from_bits(x)).to_bits();
            assert_eq!(
                got, expected,
                "{name}({x:#018x}) = {got:#018x}, want {expected:#018x}"
            );
        }

        // atan2 on integer deltas, wrong in glibc, the libm crate, or both; and the point_direction those
        // deltas give (`point_direction(0, 0, dx, dy)` is `atan2(-dy, dx)` in degrees).
        #[rustfmt::skip]
        let atan2s: &[(f64, f64, u64, u64)] = &[
            (108.0, 237.0, 0xbff2_4a9c_6228_7053, 0x4072_67fa_28ee_6d67),
            (-95.0, 256.0, 0xbffe_d176_06b7_22f4, 0x406f_347e_5c0b_b81d),
            (-47.0, 249.0, 0xbffc_1e21_bde4_9d39, 0x4070_34f9_6d61_2dc3),
            (124.0, 236.0, 0xbff1_646c_285b_1d8d, 0x4072_9b7e_fb90_32ba),
            (-250.0, 256.0, 0xc002_c133_ef39_53b3, 0x406c_35bd_5b27_d8e8),
        ];
        for &(dx, dy, radians, degrees) in atan2s {
            let got = atan2(-dy, dx).to_bits();
            assert_eq!(
                got, radians,
                "atan2({}, {dx}) = {got:#018x}, want {radians:#018x}",
                -dy
            );
            let got = point_direction(0.0, 0.0, dx, dy).to_bits();
            assert_eq!(
                got, degrees,
                "point_direction(0, 0, {dx}, {dy}) = {got:#018x}, want {degrees:#018x}"
            );
        }
    }

    #[test]
    fn transcendental_special_values() {
        assert_eq!(sin(0.0).to_bits(), 0.0_f64.to_bits());
        assert_eq!(sin(-0.0).to_bits(), (-0.0_f64).to_bits());
        assert_eq!(cos(0.0), 1.0);
        assert!(sin(f64::INFINITY).is_nan() && cos(f64::NAN).is_nan() && arctan(f64::NAN).is_nan());
        assert_eq!(arctan(f64::INFINITY), std::f64::consts::FRAC_PI_2);
        assert_eq!(atan2(0.0, -1.0), std::f64::consts::PI);
        assert_eq!(atan2(-0.0, -1.0), -std::f64::consts::PI);
        assert_eq!(atan2(0.0, 0.0).to_bits(), 0.0_f64.to_bits());
        // An underflowing negative quotient is -0, as in IEEE and glibc (upstream pxfm gives +0; the vendored copy is fixed).
        assert_eq!(
            atan2(-f64::MIN_POSITIVE, 1e300).to_bits(),
            (-0.0_f64).to_bits()
        );
        assert_eq!(atan2(f64::MIN_POSITIVE, 1e300).to_bits(), 0.0_f64.to_bits());
        // Same point: atan2(0, 0) = 0 degrees.
        assert_eq!(point_direction(5.0, 5.0, 5.0, 5.0), 0.0);
    }

    /// Pinned like `transcendentals_are_correctly_rounded`. The inputs are `oVampire`'s
    /// `distance_to_point` gaps (fractional player position minus an integer bounding-box edge) from
    /// `tools/math-probe`, where glibc's `hypot` (the unpatched oracle), the `libm` crate's (what wasm32's
    /// `f64::hypot` uses), `sqrt(x*x + y*y)`, or all three return the other neighbouring double. Every
    /// expected value was checked with mpmath at 300 bits.
    #[test]
    fn hypot_is_correctly_rounded() {
        #[rustfmt::skip]
        let cases: &[(f64, f64, u64)] = &[
            // Wrong in glibc.
            (587.9019586838158, 519.7631099972803, 0x4088_85be_8c2b_309e),
            (789.3757839447743, 498.2474691270305, 0x408d_2bc0_d5dc_ce03),
            (208.71432459234268, 260.6924890818484, 0x4074_df30_ff0a_593f),
            // Wrong in the libm crate.
            (62.573402660959346, 22.819472565300202, 0x4050_a6b0_1511_eef8),
            (425.51011207700935, 165.07928794714792, 0x407c_868f_15bb_0b77),
            // Wrong as sqrt(x*x + y*y).
            (-424.9977325483944, 354.34233424074455, 0x4081_4ab1_abb1_3a5c),
            (449.1678086647154, 141.17409424906975, 0x407d_6d4b_d6fc_e8f1),
            (8.461521353887179, -254.4216443902386, 0x406f_d1fe_74db_bbf8),
            // Wrong in all three.
            (345.29828785435166, -159.02209141638832, 0x4077_c280_daec_766b),
            (-40.431498102427554, -24.118981078674437, 0x4047_8a1c_570b_e9c9),
            (64.20412851758908, -10.850142961218125, 0x4050_4753_aadb_4316),
        ];
        for &(x, y, expected) in cases {
            for (x, y) in [(x, y), (y, x), (-x, -y)] {
                let got = hypot(x, y).to_bits();
                assert_eq!(
                    got, expected,
                    "hypot({x}, {y}) = {got:#018x}, want {expected:#018x}"
                );
            }
        }
        // distance_to_object's whole gaps: the same as sqrt(x*x + y*y), exact or not.
        for (x, y) in [(3.0, 4.0), (1.0, 1.0), (16.0, 100.0), (1023.0, 1024.0)] {
            assert_eq!(hypot(x, y), (x * x + y * y).sqrt());
        }
    }

    /// The IEEE special cases and each range, on every target `gm` is tested on (wasm32 too). Ties and the
    /// far-apart cutoff are tested in `vendor/pxfm/src/hypot.rs`, which `scripts/check` also runs.
    #[test]
    fn hypot_edge_cases() {
        let inf = f64::INFINITY;
        assert_eq!(hypot(inf, f64::NAN), inf, "an infinity beats NaN");
        assert_eq!(hypot(f64::NAN, -inf), inf);
        assert_eq!(hypot(-f64::NAN, 0.0).to_bits(), f64::NAN.to_bits());
        assert_eq!(hypot(-0.0, -0.0).to_bits(), 0.0_f64.to_bits());
        assert_eq!(hypot(-2.5, 0.0), 2.5);
        // No overflow in the intermediate squares, and overflow only when the result does.
        assert_eq!(hypot(3e300, 4e300), 5e300);
        assert_eq!(hypot(f64::MAX, 1e300), f64::MAX);
        assert_eq!(hypot(f64::MAX, f64::MAX), inf);
        // Subnormals: no underflow in the squares, and rounding on the coarser grid.
        assert_eq!(hypot(3e-320, 4e-320), 5e-320);
        assert_eq!(hypot(5e-324, 5e-324), 5e-324, "sqrt(2) ulps rounds to 1");
        assert_eq!(
            hypot(2.0 * 5e-324, 2.0 * 5e-324),
            3.0 * 5e-324,
            "2 sqrt(2) rounds to 3"
        );
    }

    #[test]
    fn make_color_rgb_packs_without_clamping() {
        assert_eq!(make_color_rgb(255, 0, 0), 255.0);
        assert_eq!(make_color_rgb(0, 255, 0), 65280.0);
        assert_eq!(make_color_rgb(1, 2, 3), f64::from(0x03_02_01));
        assert_eq!(make_color_rgb(256, 0, 0), make_color_rgb(0, 1, 0));
    }
}
