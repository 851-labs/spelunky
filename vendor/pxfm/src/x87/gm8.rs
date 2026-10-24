// Written for the Spelunky port (851-3257), not part of upstream pxfm. Distributed under the same terms as the
// rest of this crate (BSD-3-Clause OR Apache-2.0); see ../../README.spelunky.md.

//! GameMaker 8's math, as its runner computes it on the x87 (the module docs of [`super`]).
//!
//! Each function is the instruction sequence that reproduces GM8 on a real Intel CPU bit for bit, for every
//! line `tools/gm8-probe` measured (851-3247, 851-3257): every `fmul`, `fdiv`, `fadd` rounds to 53 bits, the
//! constants and the transcendental instructions' results are 80-bit, and a result is a double once it's
//! stored. Arguments and results are doubles, as GML values are. Infinities and NaN never reach the FPU here:
//! where the x87 would compute with them, the result is what it would give (or NaN where GM8 stops with an
//! error, which its runner raises for the x87's unmasked invalid-operation exception).

use super::{F80, Precision::Double, Precision::Extended};

/// Delphi's `Pi / 180`, folded to an 80-bit constant.
const PI_OVER_180: F80 = F80::PI.div(F80::from_u64(180), Extended);
/// Delphi's `180 / Pi`.
const DEG_PER_RAD: F80 = F80::from_u64(180).div(F80::PI, Extended);
const ONE_EIGHTY: F80 = F80::from_u64(180);

fn fld(x: f64) -> Option<F80> {
    F80::from_f64(x)
}

/// x times an 80-bit constant, rounded to 53 bits.
fn scale(x: f64, c: F80) -> f64 {
    match fld(x) {
        Some(x) => x.mul(c, Double).to_f64(),
        // inf * c is inf, and NaN stays NaN.
        None => x * c.to_f64(),
    }
}

/// `degtorad(x)`: Delphi's `DegToRad`, `x * (Pi / 180)` with the constant at 80 bits. 1 ulp above
/// `x * (pi / 180)` in doubles on 11% of half degrees.
pub fn degtorad(x: f64) -> f64 {
    scale(x, PI_OVER_180)
}

/// `radtodeg(x)`: Delphi's `RadToDeg`, `x * (180 / Pi)` with the constant at 80 bits.
pub fn radtodeg(x: f64) -> f64 {
    scale(x, DEG_PER_RAD)
}

/// `sin(x)`: Delphi's `Sin`, the `fsin` instruction ([`F80::sin`]). Beyond 2^63 it gives x back; for an
/// infinity or NaN, NaN.
pub fn sin(x: f64) -> f64 {
    fld(x).map_or(f64::NAN, |x| x.sin().to_f64())
}

/// `cos(x)`: `fcos`, as [`sin`].
pub fn cos(x: f64) -> f64 {
    fld(x).map_or(f64::NAN, |x| x.cos().to_f64())
}

/// `tan(x)`: `fptan`, as [`sin`].
pub fn tan(x: f64) -> f64 {
    fld(x).map_or(f64::NAN, |x| x.tan().to_f64())
}

/// `fpatan` of two doubles, the infinities included (IEEE's atan2 of them, at 64 bits). `None` for a NaN.
fn fpatan(y: f64, x: f64) -> Option<F80> {
    match (fld(y), fld(x)) {
        (Some(y), Some(x)) => Some(F80::atan2(y, x)),
        _ if y.is_nan() || x.is_nan() => None,
        // y finite, x infinite: +-0 or +-pi.
        (Some(_), None) => {
            let a = if x > 0.0 { F80::ZERO } else { F80::PI };
            Some(if y.is_sign_negative() { a.neg() } else { a })
        }
        // y infinite: +-pi/2, or +-pi/4 or +-3pi/4 if x is too.
        (None, x87) => {
            let half_pi = F80::PI.scale(-1);
            let a = match x87 {
                Some(_) => half_pi,
                None if x > 0.0 => F80::PI.scale(-2),
                None => F80::PI.mul(F80::from_u64(3), Extended).scale(-2),
            };
            Some(if y < 0.0 { a.neg() } else { a })
        }
    }
}

/// `arctan(x)`: Delphi's `ArcTan`, `fpatan(x, 1)`.
pub fn arctan(x: f64) -> f64 {
    fpatan(x, 1.0).map_or(f64::NAN, F80::to_f64)
}

/// `arctan2(y, x)`: Delphi's `ArcTan2`, `fpatan(y, x)`.
pub fn arctan2(y: f64, x: f64) -> f64 {
    fpatan(y, x).map_or(f64::NAN, F80::to_f64)
}

/// The angle of (x, y) in degrees, in [0, 360]: `fpatan(y, x) * 180 / Pi`, each step rounded to 53 bits with
/// the arctangent and `fldpi` going in at 80 bits, then `+ 360` if negative. A tiny negative angle wraps to
/// exactly 360, and atan2(-0, x) stays -0.
fn degrees(y: f64, x: f64) -> f64 {
    let Some(a) = fpatan(y, x) else {
        return f64::NAN;
    };
    let d = a.mul(ONE_EIGHTY, Double).div(F80::PI, Double).to_f64();
    if d < 0.0 { d + 360.0 } else { d }
}

/// `point_direction(x1, y1, x2, y2)`: the direction from the first point to the second, in degrees
/// counterclockwise with y pointing down ([`degrees`] of `(x2 - x1, -(y2 - y1))`). Between equal points it's
/// +0, and straight right it's -0 (the negated +0 difference), as GM8 answers.
pub fn point_direction(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let (dx, dy) = (x2 - x1, y2 - y1);
    if dx == 0.0 && dy == 0.0 {
        return 0.0;
    }
    degrees(-dy, dx)
}

/// The runner's snap of a motion component or direction to a whole number within 0.0001 of it (OpenGMK
/// `instance.rs`, `update_hvspeed`). The whole number comes from Delphi's `Round`, an integer, so a value
/// snapped to zero is +0 even when it was -0.
pub fn snap(v: f64) -> f64 {
    let r = v.round_ties_even() + 0.0;
    if (r - v).abs() < 0.0001 { r } else { v }
}

/// The runner's `direction` after `hspeed` or `vspeed` changes: [`degrees`] of `(hspeed, -vspeed)`, snapped.
pub fn direction(hspeed: f64, vspeed: f64) -> f64 {
    snap(degrees(-vspeed, hspeed))
}

/// The runner's `(hspeed, vspeed)` for a `direction` and `speed`: `speed * cos(direction * Pi / 180)` and
/// `-speed * sin(...)`, the angle rounded to 53 bits after each step (the runner's own conversion, not
/// `degtorad`'s), the 80-bit cosine or sine going into the product, then each [`snap`]ped.
pub fn motion(direction: f64, speed: f64) -> (f64, f64) {
    let Some(a) = runner_radians(direction) else {
        return (f64::NAN, f64::NAN);
    };
    let (sin, cos) = a.sin_cos();
    (component(speed, cos), component(speed, sin.neg()))
}

/// The runner's `direction * Pi / 180`, each step rounded to 53 bits. `None` for an infinite or NaN
/// direction, whose `fsin` and `fcos` are invalid operations.
fn runner_radians(direction: f64) -> Option<F80> {
    fld(direction).map(|d| d.mul(F80::PI, Double).div(ONE_EIGHTY, Double))
}

/// `speed * f` rounded to 53 bits, [`snap`]ped. An infinite speed gives an infinity, as on the x87.
fn component(speed: f64, f: F80) -> f64 {
    snap(match fld(speed) {
        Some(s) => s.mul(f, Double).to_f64(),
        None => speed * f.to_f64(),
    })
}

/// `lengthdir_x(len, dir)`: the runner's `hspeed` for speed `len` and direction `dir`, snap included
/// ([`motion`]; measured equal on every half degree).
pub fn lengthdir_x(len: f64, dir: f64) -> f64 {
    runner_radians(dir).map_or(f64::NAN, |a| component(len, a.cos()))
}

/// `lengthdir_y(len, dir)`: the runner's `vspeed`, as [`lengthdir_x`].
pub fn lengthdir_y(len: f64, dir: f64) -> f64 {
    runner_radians(dir).map_or(f64::NAN, |a| component(len, a.sin().neg()))
}

/// `ln(x)`: Delphi's `Ln` ([`F80::ln`]). NaN unless x > 0, where GM8 stops with "Error in function ln().".
pub fn ln(x: f64) -> f64 {
    match fld(x) {
        Some(v) => v.ln().map_or(f64::NAN, F80::to_f64),
        None if x == f64::INFINITY => x,
        None => f64::NAN,
    }
}

/// `exp(x)`: Delphi's `Exp`, `fldl2e; fmul; fld st; frndint; fsub st(1), st; fxch; f2xm1; fld1; fadd; fscale`:
/// t = x * log2(e) rounded to 53 bits, then 2^n * (1 + (2^f - 1)) with n the nearest whole number to t and
/// f = t - n, the sum rounded to 53 bits. Past the double range it's infinite or zero (GM8 stops on the
/// overflow: "Error in function exp().").
pub fn exp(x: f64) -> f64 {
    let Some(v) = fld(x) else {
        return if x == f64::NEG_INFINITY { 0.0 } else { x };
    };
    let t = v.mul(F80::LOG2E, Double);
    let n = t.round_int();
    let f = t.sub(n, Double);
    let Some(m) = f.exp2_m1() else {
        return f64::NAN;
    };
    let s = m.add(F80::ONE, Double);
    match n.to_i64() {
        Some(n) if n.abs() < 1 << 20 => s.scale(n as i32).to_f64(),
        _ if x > 0.0 => f64::INFINITY,
        _ => 0.0,
    }
}
