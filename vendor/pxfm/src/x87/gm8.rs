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

/// `tan(x)`: Delphi's `Tan`, `fptan` ([`F80::tan`]) then `fstp st(0)` to drop the 1 it pushes. At 2^63 and
/// beyond `fptan` pushes nothing, so that pops the argument itself and the result is read from an empty
/// register: the x87's invalid operation, an error on GM8, NaN here (unlike [`sin`] and [`cos`], which give the
/// argument back). For an infinity or NaN, NaN too.
pub fn tan(x: f64) -> f64 {
    match fld(x) {
        Some(v) if x.abs() < TWO_63 => v.tan().to_f64(),
        _ => f64::NAN,
    }
}

/// 2^63, where `fsin`, `fcos` and `fptan` stop reducing.
const TWO_63: f64 = 9_223_372_036_854_775_808.0;

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

/// `point_distance(x1, y1, x2, y2)`: Delphi's `Sqrt(Sqr(x2 - x1) + Sqr(y2 - y1))`, each step rounded to 53
/// bits and only the root stored. The squares stay in a register, whose exponent reaches far past a double's,
/// so a gap past 2^512 doesn't overflow and a tiny one doesn't underflow: `point_distance(0, 0, 2^1023, 0)` is
/// 2^1023 on GM8, where in doubles the square is inf. Only a distance that is itself past the largest double
/// is infinite, and GM8 stops there with its overflow error. With an infinite or NaN coordinate it's the same
/// formula in doubles (`inf - inf` is NaN, the x87's invalid operation).
pub fn point_distance(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let (Some(x1), Some(y1), Some(x2), Some(y2)) = (fld(x1), fld(y1), fld(x2), fld(y2)) else {
        let (dx, dy) = (x2 - x1, y2 - y1);
        return (dx * dx + dy * dy).sqrt();
    };
    let (dx, dy) = (x2.sub(x1, Double), y2.sub(y1, Double));
    let sum = dx.mul(dx, Double).add(dy.mul(dy, Double), Double);
    // A sum of squares is never below zero (at worst +0).
    sum.sqrt(Double).map_or(f64::NAN, F80::to_f64)
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

/// The runner's `speed` after `hspeed` or `vspeed` changes: `Sqrt(Sqr(hspeed) + Sqr(vspeed))` as
/// [`point_distance`] computes it, the squares in a register (so huge components don't overflow), then
/// [`snap`]ped like [`direction`] (851-3281, probe `motion-snap`). So `hspeed = 5.00001` gives speed 5, and a
/// speed under 0.0001 is 0: `hspeed = 2^-600` doesn't keep 2^-600, though the register would. The snap's
/// `Round` into an `Int64` is the x87's invalid operation from 2^63 up, where GM8 on Intel stops; this gives
/// what Rosetta, which doesn't trap, stores: the speed unsnapped.
pub fn speed(hspeed: f64, vspeed: f64) -> f64 {
    snap(point_distance(0.0, 0.0, hspeed, vspeed))
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

/// `speed * f` rounded to 53 bits. An infinite speed gives an infinity, as on the x87.
fn product(speed: f64, f: F80) -> f64 {
    match fld(speed) {
        Some(s) => s.mul(f, Double).to_f64(),
        None => speed * f.to_f64(),
    }
}

/// [`product`], [`snap`]ped.
fn component(speed: f64, f: F80) -> f64 {
    snap(product(speed, f))
}

/// The runner's AddMotion, behind gravity (`gravity` towards `gravity_direction`, every step) and
/// `motion_add(dir, amount)`: `hspeed = cos(dir * Pi / 180) * amount + hspeed` and
/// `vspeed = vspeed - sin(...) * amount`, the angle converted as for [`motion`] and the 80-bit cosine and sine
/// going into the products, with no snap (Spelunky.exe's routine, measured on Intel by
/// `tools/gm8-probe/probes/x87/angle-motion-add.gml`, 851-3262). The runner then recomputes `speed` and
/// `direction` from the result. NaN for an infinite or NaN `dir`.
pub fn add_motion(hspeed: f64, vspeed: f64, dir: f64, amount: f64) -> (f64, f64) {
    let Some(a) = runner_radians(dir) else {
        return (f64::NAN, f64::NAN);
    };
    let (sin, cos) = a.sin_cos();
    (
        add_product(hspeed, amount, cos, false),
        add_product(vspeed, amount, sin, true),
    )
}

/// `x + amount * f` (or `x - amount * f`) as the runner computes it: `fmul` then `fadd` (`fsubr`), the product
/// staying in the register, so both steps round to 53 bits with the register's exponent range and the result
/// rounds to a double once, when it's stored. Only a product past a double's range, or below its normal range,
/// tells this from double arithmetic. An infinite or NaN operand gives what doubles give.
fn add_product(x: f64, amount: f64, f: F80, subtract: bool) -> f64 {
    match (fld(x), fld(amount)) {
        (Some(x), Some(a)) => {
            let p = a.mul(f, Double);
            if subtract {
                x.sub(p, Double)
            } else {
                p.add(x, Double)
            }
            .to_f64()
        }
        _ if subtract => x - amount * f.to_f64(),
        _ => amount * f.to_f64() + x,
    }
}

/// The sine and cosine of an angle in degrees as the runner takes them for a rotated bounding box or mask
/// (`image_angle`) and a rotated view (`view_angle`): `fsin` and `fcos` of `angle * Pi / 180`, the angle
/// rounded to 53 bits after each step as for [`motion`] (not [`degtorad`]'s conversion), each result stored
/// as a double. Measured on Intel (`tools/gm8-probe/probes/x87/angle-*.gml`, 851-3262). Where the runner
/// negates the angle first, as its masks do, the results are these negated exactly. NaN for an infinite or
/// NaN angle.
pub fn sin_cos_degrees(angle: f64) -> (f64, f64) {
    runner_radians(angle).map_or((f64::NAN, f64::NAN), |a| {
        let (sin, cos) = a.sin_cos();
        (sin.to_f64(), cos.to_f64())
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
