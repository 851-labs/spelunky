// Written for the Spelunky port (851-3174), not part of upstream pxfm: a correctly rounded `hypot`, in place of
// upstream's `f_hypot`, which is sometimes 1 ulp off. Distributed under the same terms as the rest of this
// crate (BSD-3-Clause OR Apache-2.0); see ../README.spelunky.md.

use core::cmp::Ordering;

/// `sqrt(x^2 + y^2)`, correctly rounded (to nearest, ties to even), without undue overflow or underflow.
///
/// As IEEE 754 specifies: an infinite argument gives +inf even if the other is NaN, otherwise a NaN
/// argument gives NaN, and signs never matter (`f_hypot(-0.0, -0.0)` is +0).
///
/// The result is computed exactly, in integers, so every target gets the same bits whether or not it has
/// FMA. The larger magnitude `a` is scaled into [1, 2) and the smaller `b` by the same power of two. If
/// `b < 2^-28 a`, the answer is `a`. Otherwise an ordinary `sqrt(a*a + b*b)` gives a guess within a few
/// ulps, which is stepped up or down until the exact sum of squares lies between the squares of the
/// guess's two half-way points (compared as 128-bit integers). Subnormal results are rounded on their own
/// coarser grid by the same exact comparison, and large ones overflow to +inf only when the rounded
/// result does.
pub fn f_hypot(x: f64, y: f64) -> f64 {
    let (x, y) = (x.abs(), y.abs());
    if x == f64::INFINITY || y == f64::INFINITY {
        return f64::INFINITY;
    }
    if x.is_nan() || y.is_nan() {
        return x + y;
    }
    let (a, b) = if x >= y { (x, y) } else { (y, x) };
    if b == 0.0 {
        return a;
    }
    if a < f64::MIN_POSITIVE {
        return subnormal_hypot(a, b);
    }

    let (ma, ea) = split(a);
    let (mb, eb) = split(b);
    // b < 2^(eb + 1) <= 2^(ea - 28) <= 2^-28 a, so with r = b / a the result is a * sqrt(1 + r^2), which is
    // above a by less than a * r^2 / 2 < 2^-57 a. Half an ulp of a is more than 2^-54 a, so it rounds to a.
    let d = ea - eb;
    if d > 28 {
        return a;
    }

    // a and b scaled by 2^-ea: a_s = ma 2^-52 in [1, 2), b_s = mb 2^-(52 + d) in [2^-28, 2). Both exact.
    let a_s = ma as f64 * pow2(-52);
    let b_s = mb as f64 * pow2(-52 - d);
    // The exact sum of squares a_s^2 + b_s^2, times 2^108: an integer part and whether a fraction is left
    // over. 2^108 is the scale at which every half-way point's square (below) is an integer.
    let (ma2, mb2) = (
        u128::from(ma) * u128::from(ma),
        u128::from(mb) * u128::from(mb),
    );
    let shift = 2 * d - 4;
    let sum = if shift <= 0 {
        Exact {
            int: (ma2 << 4) + (mb2 << -shift),
            frac: false,
        }
    } else {
        Exact {
            int: (ma2 << 4) + (mb2 >> shift),
            frac: mb2 & ((1 << shift) - 1) != 0,
        }
    };

    // The guess is in [1, 4): sqrt(1) <= sqrt(a_s^2 + b_s^2) <= sqrt(8), give or take a few ulps.
    let mut r = (a_s * a_s + b_s * b_s).sqrt();
    loop {
        let up = next_up(r);
        let above = sum.cmp_square_of_midpoint(r, up);
        if above == Ordering::Greater || (above == Ordering::Equal && is_odd(r)) {
            r = up;
            continue;
        }
        let down = next_down(r);
        let below = sum.cmp_square_of_midpoint(r, down);
        if below == Ordering::Less || (below == Ordering::Equal && is_odd(r)) {
            r = down;
            continue;
        }
        break;
    }
    // Exact, or +inf when the rounded result is 2^1024 or more. ea is in [-1022, 1023], and the result is
    // at least a, which is normal, so it can't be subnormal.
    r * pow2(ea)
}

/// Both arguments subnormal (`b <= a < 2^-1022`): they are `X 2^-1074` and `Y 2^-1074` for integers below
/// 2^52, and the result is `sqrt(X^2 + Y^2) 2^-1074`, which is below 2^-1021, where the doubles are the
/// multiples of 2^-1074. So round `sqrt(X^2 + Y^2)` to the nearest integer. It's never half-way:
/// `(n + 1/2)^2` isn't an integer.
fn subnormal_hypot(a: f64, b: f64) -> f64 {
    let (x, y) = (u128::from(a.to_bits()), u128::from(b.to_bits()));
    let t = x * x + y * y;
    let mut n = t.isqrt();
    // t > (n + 1/2)^2 = n^2 + n + 1/4 exactly when t > n^2 + n, since t is an integer.
    if t - n * n > n {
        n += 1;
    }
    // n < 2^53, so both conversions are exact.
    n as u64 as f64 * f64::from_bits(1)
}

/// A non-negative number known exactly as `int + (something in [0, 1) if frac)`.
struct Exact {
    int: u128,
    frac: bool,
}

impl Exact {
    /// Compares `self` (the sum of squares, times 2^108) with the square of the half-way point between the
    /// neighbouring doubles `r` and `q`, both in [0.5, 8). Those are multiples of 2^-53, so
    /// `n = (r + q) 2^53` is an integer below 2^57, the half-way point is `n 2^-54`, and its square times
    /// 2^108 is the integer `n^2`.
    fn cmp_square_of_midpoint(&self, r: f64, q: f64) -> Ordering {
        let n = u128::from(to_units(r) + to_units(q));
        match self.int.cmp(&(n * n)) {
            Ordering::Equal if self.frac => Ordering::Greater,
            ordering => ordering,
        }
    }
}

/// `v 2^53` as an integer, for `v` a multiple of 2^-53 in [0.5, 8). Exact.
fn to_units(v: f64) -> u64 {
    (v * pow2(53)) as u64
}

/// A positive finite non-zero `v` as `(m, e)` with `v = m 2^(e - 52)` and `m` in [2^52, 2^53).
fn split(v: f64) -> (u64, i32) {
    let bits = v.to_bits();
    let biased = (bits >> 52) as i32;
    let fraction = bits & ((1 << 52) - 1);
    if biased == 0 {
        // Subnormal: shift the leading bit up to bit 52.
        let shift = fraction.leading_zeros() as i32 - 11;
        (fraction << shift, -1022 - shift)
    } else {
        (fraction | (1 << 52), biased - 1023)
    }
}

/// `2^e`, for `e` in [-1022, 1023].
fn pow2(e: i32) -> f64 {
    f64::from_bits(((e + 1023) as u64) << 52)
}

/// The next double up from a positive finite `v`.
fn next_up(v: f64) -> f64 {
    f64::from_bits(v.to_bits() + 1)
}

/// The next double down from a positive `v` above the smallest subnormal.
fn next_down(v: f64) -> f64 {
    f64::from_bits(v.to_bits() - 1)
}

fn is_odd(v: f64) -> bool {
    v.to_bits() & 1 == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn special_values() {
        let inf = f64::INFINITY;
        assert_eq!(f_hypot(inf, f64::NAN), inf);
        assert_eq!(f_hypot(f64::NAN, -inf), inf);
        assert!(f_hypot(f64::NAN, 1.0).is_nan());
        assert!(f_hypot(0.0, f64::NAN).is_nan());
        assert_eq!(f_hypot(-0.0, -0.0).to_bits(), 0);
        assert_eq!(f_hypot(-3.0, 0.0), 3.0);
        assert_eq!(f_hypot(f64::MAX, f64::MAX), inf);
        assert_eq!(f_hypot(f64::MAX, 1.0), f64::MAX);
        assert_eq!(
            f_hypot(5e-324, 5e-324),
            5e-324,
            "sqrt(2) rounds to 1 on the subnormal grid"
        );
        assert_eq!(f_hypot(3.0 * 5e-324, 4.0 * 5e-324), 5.0 * 5e-324);
    }

    #[test]
    fn exact_triples() {
        assert_eq!(f_hypot(3.0, 4.0), 5.0);
        assert_eq!(f_hypot(5.0, 12.0), 13.0);
        assert_eq!(f_hypot(3e300, 4e300), 5e300);
        assert_eq!(f_hypot(3e-300, 4e-300), 5e-300);
        assert_eq!(f_hypot(1.0, 0.0), 1.0);
    }

    #[test]
    fn ties_go_to_even() {
        // A Pythagorean triple A^2 + B^2 = M^2 (A = m^2 - n^2, B = 2 m n, M = m^2 + n^2) with A and B below 2^53
        // and M odd in [2^53, 2^54), where the doubles are the even integers: hypot(A, B) = M is exactly
        // half-way between M - 1 and M + 1, and the one with the even significand wins.
        for (m, n) in [
            (92_503_185u64, 47_453_132u64),
            (92_503_187, 47_453_132),
            (92_503_185, 47_453_130),
        ] {
            let (a, b, c) = (m * m - n * n, 2 * m * n, m * m + n * n);
            assert!(
                a < 1 << 53 && b < 1 << 53 && c >> 53 == 1 && c % 2 == 1,
                "not a tie"
            );
            let even = if (c - 1) / 2 % 2 == 0 { c - 1 } else { c + 1 };
            assert_eq!(f_hypot(a as f64, b as f64), even as f64);
            assert_eq!(f_hypot(b as f64, -(a as f64)), even as f64);
        }
    }

    #[test]
    fn far_apart_returns_the_larger() {
        assert_eq!(f_hypot(1.0, pow2(-29)), 1.0);
        assert_eq!(f_hypot(1.0, pow2(-26)), 1.0);
        assert_eq!(f_hypot(1.0, pow2(-26) * 1.5), next_up(1.0));
        assert_eq!(f_hypot(f64::MIN_POSITIVE, 5e-324), f64::MIN_POSITIVE);
    }
}
