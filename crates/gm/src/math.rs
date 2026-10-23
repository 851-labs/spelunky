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

/// GML `a == b` on reals: `|a - b| < 1e-13`. NaN equals nothing, itself included.
#[inline]
pub fn eq(a: f64, b: f64) -> bool {
    (a - b).abs() < CMP_EPSILON
}

/// GML `a != b` on reals: `|a - b| >= 1e-13`. Not quite `!eq`: with a NaN operand both are false.
#[inline]
pub fn ne(a: f64, b: f64) -> bool {
    (a - b).abs() >= CMP_EPSILON
}

/// GML `a < b` on reals: `a - b <= -1e-13`.
#[inline]
pub fn lt(a: f64, b: f64) -> bool {
    a - b <= -CMP_EPSILON
}

/// GML `a <= b` on reals: `a - b < 1e-13`.
#[inline]
pub fn le(a: f64, b: f64) -> bool {
    a - b < CMP_EPSILON
}

/// GML `a > b` on reals: `a - b >= 1e-13`.
#[inline]
pub fn gt(a: f64, b: f64) -> bool {
    a - b >= CMP_EPSILON
}

/// GML `a >= b` on reals: `a - b > -1e-13`.
#[inline]
pub fn ge(a: f64, b: f64) -> bool {
    a - b > -CMP_EPSILON
}

/// GML `round(x)`: round half to even ("banker's rounding"), like Delphi's `Round`. OpenGMK `math.rs`
/// (`ieee_round`, `Real::round`). That computes `floor(x)` plus a correction, so a negative `x` that
/// rounds to zero gives +0, not -0 (which `string()` would print as `"-0"`); only -0 itself stays -0.
#[inline]
pub fn round(x: f64) -> f64 {
    let r = x.round_ties_even();
    if r == 0.0 && x != 0.0 { 0.0 } else { r }
}

/// How the runner turns a real into an integer argument (`round`, then truncate to 32 bits). OpenGMK
/// `value.rs` (`From<Value> for i32`) and `math.rs` (`Real::to_i32`: through `i64`, so it wraps rather
/// than saturates past the `i32` range). Out-of-range `i64` values and NaN saturate as Rust casts do.
#[inline]
pub fn to_int(x: f64) -> i32 {
    round(x) as i64 as i32
}

/// GML `a div b`: `floor(a / b)`, so `-7 div 2 == -4`. Division by zero is not an error and gives
/// ±inf or NaN. OpenGMK `value.rs` (`Value::intdiv`). The game never uses `div`.
#[inline]
pub fn int_div(a: f64, b: f64) -> f64 {
    (a / b).floor()
}

/// GML `a mod b`: `a - trunc(a / b) * b`. The sign follows `a`, and the result is *computed* through a
/// division, so it can differ from IEEE `fmod` (Rust's `%`) in the last bits. OpenGMK `value.rs`
/// (`Value::modulo`, `Real::floor_towards_zero`).
#[inline]
pub fn modulo(a: f64, b: f64) -> f64 {
    a - (a / b).trunc() * b
}

/// The integer a bitwise operator sees: rounded half to even, then converted to `u64` with Rust's
/// saturating cast, so negative values and NaN become 0. OpenGMK `value.rs` (`bitand` and friends use
/// `round().to_u64()`, and `Real::to_u64` is `as u64`). Unverified against GM8 itself, where negatives
/// probably keep their two's-complement bits; the game uses no bitwise operators.
#[inline]
fn to_bits(x: f64) -> u64 {
    round(x) as u64
}

/// The shift count a shift operator sees: OpenGMK rounds it, wraps it to `i8`, and shifts a `u64`, which
/// in a release build masks the count to 0..=63. OpenGMK `value.rs` (`Value::shl`, `Value::shr`).
#[inline]
fn shift_count(x: f64) -> u32 {
    (round(x) as i64 as u32) & 63
}

/// GML `a & b`.
#[inline]
pub fn bit_and(a: f64, b: f64) -> f64 {
    (to_bits(a) & to_bits(b)) as f64
}

/// GML `a | b`.
#[inline]
pub fn bit_or(a: f64, b: f64) -> f64 {
    (to_bits(a) | to_bits(b)) as f64
}

/// GML `a ^ b`.
#[inline]
pub fn bit_xor(a: f64, b: f64) -> f64 {
    (to_bits(a) ^ to_bits(b)) as f64
}

/// GML `~a`: the 64-bit complement, so `~0` is 2^64 - 1 (rounded to 2^64 as a real). OpenGMK `value.rs`
/// (`Value::complement`).
#[inline]
pub fn bit_not(a: f64) -> f64 {
    (!to_bits(a)) as f64
}

/// GML `a << b`.
#[inline]
pub fn shl(a: f64, b: f64) -> f64 {
    (to_bits(a) << shift_count(b)) as f64
}

/// GML `a >> b` (logical, on the unsigned value).
#[inline]
pub fn shr(a: f64, b: f64) -> f64 {
    (to_bits(a) >> shift_count(b)) as f64
}

/// GML `abs(x)`.
#[inline]
pub fn abs(x: f64) -> f64 {
    x.abs()
}

/// GML `floor(x)`.
#[inline]
pub fn floor(x: f64) -> f64 {
    x.floor()
}

/// GML `ceil(x)`.
#[inline]
pub fn ceil(x: f64) -> f64 {
    x.ceil()
}

/// GML `frac(x)`: `x - trunc(x)`, so the sign follows `x` (`frac(-1.25) == -0.25`), like Delphi's `Frac`.
/// OpenGMK `kernel.rs` (`frac`, via `f64::fract`).
#[inline]
pub fn frac(x: f64) -> f64 {
    x.fract()
}

/// GML `sign(x)`: uses the comparison tolerance, so `sign(1e-14) == 0`. OpenGMK `kernel.rs` (`sign`).
#[inline]
pub fn sign(x: f64) -> f64 {
    if x >= CMP_EPSILON {
        1.0
    } else if x <= -CMP_EPSILON {
        -1.0
    } else {
        0.0
    }
}

/// GML `sqr(x)`: `x * x`.
#[inline]
pub fn sqr(x: f64) -> f64 {
    x * x
}

/// GML `sqrt(x)`. A result that isn't finite (a negative or infinite argument) is a runtime error.
/// OpenGMK `kernel.rs` (`sqrt`).
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

/// `x` to the integer power `n` by binary exponentiation (square-and-multiply, then `1 / r` for a
/// negative `n`): the algorithm behind Rust's `f64::powi` (compiler-builtins' `__powidf2`), which OpenGMK
/// uses for `string_format`. `f64::powi` itself is banned in this crate because Rust leaves its precision
/// unspecified (LLVM may constant-fold it differently, for one), so it can't be relied on for exact bits.
pub fn powi(x: f64, n: i32) -> f64 {
    let mut base = x;
    let mut exp = n.unsigned_abs();
    let mut r = 1.0;
    loop {
        if exp & 1 != 0 {
            r *= base;
        }
        exp >>= 1;
        if exp == 0 {
            break;
        }
        base *= base;
    }
    if n < 0 { 1.0 / r } else { r }
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
    fn nan_compares_false_everywhere() {
        let n = f64::NAN;
        assert!(!eq(n, n) && !ne(n, n) && !lt(n, 0.0) && !le(n, 0.0) && !gt(n, 0.0) && !ge(n, 0.0));
    }

    #[test]
    fn round_is_bankers() {
        assert_eq!(round(0.5), 0.0);
        assert_eq!(round(1.5), 2.0);
        assert_eq!(round(2.5), 2.0);
        assert_eq!(round(-0.5), 0.0);
        assert_eq!(round(-1.5), -2.0);
        assert_eq!(round(-2.5), -2.0);
        assert_eq!(round(2.5000001), 3.0);
        assert_eq!(round(-13.37), -13.0);
        // Negative values that round to zero give +0, as OpenGMK's floor-based rounding does.
        assert!(round(-0.4).is_sign_positive());
        assert!(round(-0.5).is_sign_positive());
        assert!(round(-1e-300).is_sign_positive());
        assert!(round(-0.0).is_sign_negative(), "-0 itself is left alone");
        assert!(f64::NAN.round_ties_even().is_nan() && round(f64::NAN).is_nan());
        assert_eq!(to_int(2.5), 2);
        assert_eq!(to_int(3.5), 4);
        // Through i64, so past the i32 range it wraps instead of saturating.
        assert_eq!(to_int(4_294_967_297.0), 1);
        assert_eq!(to_int(f64::NAN), 0);
    }

    #[test]
    fn div_floors_and_mod_truncates() {
        assert_eq!(int_div(7.0, 2.0), 3.0);
        assert_eq!(int_div(-7.0, 2.0), -4.0);
        assert_eq!(int_div(7.5, 2.5), 3.0);
        assert_eq!(modulo(7.0, 3.0), 1.0);
        assert_eq!(modulo(-7.0, 3.0), -1.0);
        assert_eq!(modulo(7.0, -3.0), 1.0);
        assert_eq!(modulo(160.5, 2.25), 0.75);
        assert_eq!(modulo(5.5, 2.0), 1.5);
        // Computed through a division, not fmod: these differ in the last bits.
        // 1 / 0.1 rounds to exactly 10, so GML gets 0 where fmod sees 0.1's representation error.
        assert_eq!(modulo(1.0, 0.1), 0.0);
        assert_eq!(1.0_f64 % 0.1, 0.09999999999999995);
        // Division by zero is not an error.
        assert!(modulo(1.0, 0.0).is_nan());
        assert_eq!(int_div(1.0, 0.0), f64::INFINITY);
    }

    #[test]
    fn bitwise_rounds_then_uses_unsigned_64_bits() {
        assert_eq!(bit_and(12.0, 10.0), 8.0);
        assert_eq!(bit_or(12.0, 10.0), 14.0);
        assert_eq!(bit_xor(12.0, 10.0), 6.0);
        assert_eq!(bit_and(2.5, 3.0), 2.0, "2.5 rounds to 2");
        assert_eq!(bit_and(3.5, 7.0), 4.0, "3.5 rounds to 4");
        assert_eq!(bit_or(-1.0, 1.0), 1.0, "negatives saturate to 0");
        assert_eq!(bit_not(0.0), 18_446_744_073_709_551_615.0);
        assert_eq!(shl(1.0, 4.0), 16.0);
        assert_eq!(shl(1.0, 64.0), 1.0, "the count is masked to 6 bits");
        assert_eq!(shr(256.0, 4.0), 16.0);
    }

    #[test]
    fn frac_sign_sqr_sqrt() {
        assert_eq!(frac(1.25), 0.25);
        assert_eq!(frac(-1.25), -0.25);
        assert_eq!(sign(3.0), 1.0);
        assert_eq!(sign(-0.001), -1.0);
        assert_eq!(sign(1e-14), 0.0, "sign uses the comparison tolerance");
        assert_eq!(sign(-1e-14), 0.0);
        assert_eq!(sqr(-3.0), 9.0);
        assert_eq!(sqrt(9.0), Ok(3.0));
        assert_eq!(sqrt(-1.0), Err(ValueError::Domain("sqrt")));
        assert_eq!(sqrt(f64::INFINITY), Err(ValueError::Domain("sqrt")));
        assert_eq!(abs(-2.5), 2.5);
        assert_eq!(floor(-2.5), -3.0);
        assert_eq!(ceil(-2.5), -2.0);
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
    fn powi_is_binary_exponentiation() {
        assert_eq!(powi(10.0, 0), 1.0);
        assert_eq!(powi(10.0, 22), 1e22, "exact up to 10^22");
        assert_eq!(powi(10.0, -3), 1.0 / 1000.0);
        assert_eq!(
            powi(2.0, -1074),
            0.0,
            "1 / 2^1074, and 2^1074 overflows first"
        );
        assert_eq!(
            powi(10.0, i32::MIN),
            0.0,
            "10^(2^31) overflows, and 1/inf is 0"
        );
        assert_eq!(powi(-2.0, 3), -8.0);
    }

    #[test]
    fn make_color_rgb_packs_without_clamping() {
        assert_eq!(make_color_rgb(255, 0, 0), 255.0);
        assert_eq!(make_color_rgb(0, 255, 0), 65280.0);
        assert_eq!(make_color_rgb(1, 2, 3), f64::from(0x03_02_01));
        assert_eq!(make_color_rgb(256, 0, 0), make_color_rgb(0, 1, 0));
    }
}
