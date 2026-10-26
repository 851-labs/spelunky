// Written for the Spelunky port (851-3257), not part of upstream pxfm. Distributed under the same terms as the
// rest of this crate (BSD-3-Clause OR Apache-2.0); see ../../README.spelunky.md.

//! A software model of the x87 FPU's extended precision, as GameMaker 8's runner uses it.
//!
//! GM8 is a Delphi program, and Delphi computes on the x87: values live in 80-bit registers (a 64-bit
//! significand), and GM8 sets the precision control to 53 bits, so every `fadd`, `fsub`, `fmul`, `fdiv` and
//! `fsqrt` rounds its result to 53 bits, but the operands that never left the FPU keep their 64. Delphi's
//! constants (`Pi`, `Pi / 180`) are 80-bit, and the transcendental instructions (`fsin`, `fpatan`, ...) ignore
//! the precision control and give 64-bit results. So GM8's `degtorad`, `point_direction` and motion aren't the
//! same as computing in doubles, and `tools/gm8-probe` measured them on a real Intel CPU (851-3247).
//!
//! [`F80`] is a register value, and its arithmetic rounds exactly as the x87 does under a given
//! [`Precision`]. The transcendental instructions follow the model that reproduces every measured line
//! (`tools/gm8-probe/x87/ulps.py`, README "Real x87"): `fsin`, `fcos` and `fptan` reduce their argument by
//! Intel's 66-bit pi, exactly, and round the exact function of the reduced argument to 64 bits; `fpatan`,
//! `fyl2x` (as `ln`) and `f2xm1` round the exact result to 64 bits. Those exact results are computed with a
//! 128-bit software float (`wide.rs`). [`gm8`] has GM8's built-ins on top.
//!
//! Everything is integer arithmetic, with no platform float functions, so every target gives the same bits.
//! The x87's 15-bit exponent range isn't modelled (exponents are `i32`): the values GM8's formulas produce
//! from doubles stay far inside it. They do leave a double's, which is why the register matters:
//! `point_distance` squares a gap of 2^1023 to 2^2046 and takes the root before anything is stored.
//! Infinities and NaN aren't register values here; [`F80::from_f64`] returns `None` for them, and [`gm8`]
//! handles them before they reach the FPU.

mod wide;

pub mod gm8;

use wide::Wide;

/// The x87's precision control: how many significant bits arithmetic results are rounded to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Precision {
    /// 24 bits.
    Single,
    /// 53 bits, what GM8 runs with.
    Double,
    /// 64 bits, the register's full significand (and what constant folding in Delphi uses).
    Extended,
}

impl Precision {
    const fn bits(self) -> u32 {
        match self {
            Precision::Single => 24,
            Precision::Double => 53,
            Precision::Extended => 64,
        }
    }
}

/// A finite x87 register value: `(-1)^neg * sig * 2^(exp - 63)`, with `sig`'s top bit (the x87's explicit
/// integer bit) set, or a signed zero (`sig == 0`, `exp == 0`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct F80 {
    neg: bool,
    exp: i32,
    sig: u64,
}

/// Rounds `(-1)^neg * m * 2^(exp - 127)` to `bits` significant bits, to nearest with ties to even. `sticky` says
/// the true value has nonzero bits below `m`'s last (it is a little above `m`).
const fn round(neg: bool, exp: i32, m: u128, sticky: bool, bits: u32) -> F80 {
    if m == 0 {
        return F80 {
            neg,
            exp: 0,
            sig: 0,
        };
    }
    let lz = m.leading_zeros();
    let (m, mut exp) = (m << lz, exp - lz as i32);
    let drop = 128 - bits;
    let mut q = m >> drop;
    let rest = m & ((1 << drop) - 1);
    let half = 1 << (drop - 1);
    if rest > half || (rest == half && (sticky || q & 1 == 1)) {
        q += 1;
        if q >> bits != 0 {
            q >>= 1;
            exp += 1;
        }
    }
    F80 {
        neg,
        exp,
        sig: (q << (64 - bits)) as u64,
    }
}

impl F80 {
    pub const ZERO: F80 = F80 {
        neg: false,
        exp: 0,
        sig: 0,
    };
    pub const ONE: F80 = F80 {
        neg: false,
        exp: 0,
        sig: 1 << 63,
    };
    /// `fldpi`: pi rounded to 64 bits (Delphi's `Pi`).
    pub const PI: F80 = F80 {
        neg: false,
        exp: 1,
        sig: 0xc90fdaa22168c235,
    };
    /// `fldln2`: ln 2 rounded to 64 bits.
    pub const LN2: F80 = F80 {
        neg: false,
        exp: -1,
        sig: 0xb17217f7d1cf79ac,
    };
    /// `fldl2e`: log2(e) rounded to 64 bits.
    pub const LOG2E: F80 = F80 {
        neg: false,
        exp: 0,
        sig: 0xb8aa3b295c17f0bc,
    };

    /// `fld` of a double: exact. `None` for an infinity or NaN.
    pub const fn from_f64(x: f64) -> Option<F80> {
        let b = x.to_bits();
        let neg = b >> 63 != 0;
        let e = ((b >> 52) & 0x7ff) as i32;
        let f = b & ((1 << 52) - 1);
        if e == 0x7ff {
            None
        } else if e == 0 {
            if f == 0 {
                Some(F80 {
                    neg,
                    exp: 0,
                    sig: 0,
                })
            } else {
                let lz = f.leading_zeros();
                Some(F80 {
                    neg,
                    exp: 63 - lz as i32 - 1074,
                    sig: f << lz,
                })
            }
        } else {
            Some(F80 {
                neg,
                exp: e - 1023,
                sig: (f | 1 << 52) << 11,
            })
        }
    }

    /// `fild` of a whole number: exact.
    pub const fn from_u64(n: u64) -> F80 {
        round(false, 127, n as u128, false, 64)
    }

    /// `fstp` to a double: rounded to nearest (ties to even) on the double's grid, subnormals included, and
    /// to infinity past the largest double.
    pub const fn to_f64(self) -> f64 {
        let sign = (self.neg as u64) << 63;
        if self.sig == 0 {
            return f64::from_bits(sign);
        }
        if self.exp >= -1022 {
            let r = round(self.neg, self.exp, (self.sig as u128) << 64, false, 53);
            if r.exp > 1023 {
                return f64::from_bits(sign | 0x7ff << 52);
            }
            let frac = (r.sig >> 11) & ((1 << 52) - 1);
            return f64::from_bits(sign | ((r.exp + 1023) as u64) << 52 | frac);
        }
        // Subnormal: the nearest whole multiple of 2^-1074, which is sig * 2^(exp - 63 + 1074).
        let shift = (63 - 1074 - self.exp) as u32;
        let m = self.sig as u128;
        let n = if shift >= 128 {
            0
        } else {
            let (q, rest, half) = (m >> shift, m & ((1 << shift) - 1), 1 << (shift - 1));
            if rest > half || (rest == half && q & 1 == 1) {
                q + 1
            } else {
                q
            }
        };
        // n = 2^52 is the smallest normal double, which these bits spell too.
        f64::from_bits(sign | n as u64)
    }

    pub const fn is_zero(self) -> bool {
        self.sig == 0
    }

    pub const fn is_sign_negative(self) -> bool {
        self.neg
    }

    /// `fchs`.
    pub const fn neg(self) -> F80 {
        F80 {
            neg: !self.neg,
            ..self
        }
    }

    /// `fabs`.
    pub const fn abs(self) -> F80 {
        F80 { neg: false, ..self }
    }

    /// The value rounded to `p`, as an arithmetic result would be.
    pub const fn round_to(self, p: Precision) -> F80 {
        round(
            self.neg,
            self.exp,
            (self.sig as u128) << 64,
            false,
            p.bits(),
        )
    }

    /// `fmul`: the exact product, rounded to `p`.
    pub const fn mul(self, rhs: F80, p: Precision) -> F80 {
        let neg = self.neg != rhs.neg;
        if self.sig == 0 || rhs.sig == 0 {
            return F80 {
                neg,
                exp: 0,
                sig: 0,
            };
        }
        // sig * sig is in [2^126, 2^128).
        round(
            neg,
            self.exp + rhs.exp + 1,
            self.sig as u128 * rhs.sig as u128,
            false,
            p.bits(),
        )
    }

    /// `fdiv`: the exact quotient, rounded to `p`. `rhs` must not be zero (the x87 would raise its
    /// division-by-zero exception, which GM8 doesn't mask).
    pub const fn div(self, rhs: F80, p: Precision) -> F80 {
        assert!(rhs.sig != 0, "F80::div by zero");
        let neg = self.neg != rhs.neg;
        if self.sig == 0 {
            return F80 {
                neg,
                exp: 0,
                sig: 0,
            };
        }
        // 96 quotient bits or more, and the remainder for the sticky bit.
        let (n, d) = ((self.sig as u128) << 64, rhs.sig as u128);
        let (q, r) = (n / d, (n % d) << 32);
        round(
            neg,
            self.exp - rhs.exp + 31,
            (q << 32) | (r / d),
            r % d != 0,
            p.bits(),
        )
    }

    /// `fadd`: the exact sum, rounded to `p`. An exact zero sum is +0 unless both operands are -0.
    pub const fn add(self, rhs: F80, p: Precision) -> F80 {
        if rhs.sig == 0 && self.sig == 0 {
            return F80 {
                neg: self.neg && rhs.neg,
                exp: 0,
                sig: 0,
            };
        }
        if rhs.sig == 0 {
            return self.round_to(p);
        }
        if self.sig == 0 {
            return rhs.round_to(p);
        }
        let swap = self.exp < rhs.exp || (self.exp == rhs.exp && self.sig < rhs.sig);
        let (x, y) = if swap { (rhs, self) } else { (self, rhs) };
        let d = (x.exp - y.exp) as u32;
        // x's significand from bit 126 down, with 63 bits below it for y's, and the rest of y as a sticky bit.
        let mx = (x.sig as u128) << 63;
        let (my, sticky) = if d >= 127 {
            (0, true)
        } else {
            let s = (y.sig as u128) << 63;
            (s >> d, s & ((1 << d) - 1) != 0)
        };
        if x.neg == y.neg {
            return round(x.neg, x.exp + 1, mx + my, sticky, p.bits());
        }
        let m = mx - my;
        if m == 0 && !sticky {
            return F80::ZERO;
        }
        // The true difference is a little below m - my: (m - 1) plus a nonzero fraction.
        round(
            x.neg,
            x.exp + 1,
            if sticky { m - 1 } else { m },
            sticky,
            p.bits(),
        )
    }

    /// `fsub`: the exact difference, rounded to `p`.
    pub const fn sub(self, rhs: F80, p: Precision) -> F80 {
        self.add(rhs.neg(), p)
    }

    /// `fsqrt`: the exact square root, rounded to `p`; `sqrt(-0)` is -0. `None` below zero (the x87's
    /// invalid-operation exception, which GM8 doesn't mask).
    pub const fn sqrt(self, p: Precision) -> Option<F80> {
        if self.sig == 0 {
            return Some(self);
        }
        if self.neg {
            return None;
        }
        // The value is n * 2^e with n = sig << s, s = 63 or 64, picked so that e = exp - 63 - s is even: then
        // its root is sqrt(n) * 2^(e / 2), and n in [2^126, 2^128) has a root in [2^63, 2^64).
        let s = if self.exp % 2 == 0 { 63 } else { 64 };
        let n = (self.sig as u128) << s;
        let r = n.isqrt();
        let rest = n - r * r;
        // sqrt(n) is r plus a fraction below 1, which is at least 1/2 when n >= r^2 + r + 1/4, that is when
        // rest > r. It is never exactly 1/2 (n is whole), and whenever rest != 0 the root is irrational, so
        // the half bit and a sticky bit for anything below it describe it exactly.
        let half = rest > r;
        let m = (r << 1) | half as u128;
        // m * 2^(e / 2 - 1), in `round`'s m * 2^(exp - 127).
        let e = self.exp - 63 - s;
        Some(round(false, e / 2 + 126, m, rest != 0, p.bits()))
    }

    /// `frndint` (rounding to nearest, ties to even, as GM8 runs it).
    pub const fn round_int(self) -> F80 {
        if self.sig == 0 || self.exp >= 63 {
            self
        } else if self.exp >= 0 {
            round(
                self.neg,
                self.exp,
                (self.sig as u128) << 64,
                false,
                self.exp as u32 + 1,
            )
        } else if self.exp == -1 && self.sig != 1 << 63 {
            F80 {
                neg: self.neg,
                ..F80::ONE
            }
        } else {
            F80 {
                neg: self.neg,
                exp: 0,
                sig: 0,
            }
        }
    }

    /// The value as a whole number, if it is one that fits in an `i64`.
    pub const fn to_i64(self) -> Option<i64> {
        if self.sig == 0 {
            return Some(0);
        }
        if self.exp < 0 || self.exp > 62 || self.sig << self.exp << 1 != 0 {
            return None;
        }
        let n = (self.sig >> (63 - self.exp)) as i64;
        Some(if self.neg { -n } else { n })
    }

    /// `fistp` into an `Int64` with the rounding control set to truncate, as Delphi's `Trunc` does it. `None`
    /// from 2^63 up (and below -2^63), where it raises the x87's invalid-operation exception.
    pub const fn trunc(self) -> Option<i64> {
        if self.sig == 0 || self.exp < 0 {
            return Some(0);
        }
        if self.exp > 62 {
            // Only -2^63 itself fits.
            return if self.neg && self.exp == 63 && self.sig == 1 << 63 {
                Some(i64::MIN)
            } else {
                None
            };
        }
        let n = (self.sig >> (63 - self.exp)) as i64;
        Some(if self.neg { -n } else { n })
    }

    /// Whether Delphi's `Frac(x)` (`x - Int(x)`) is below zero: x is negative and not a whole number.
    pub const fn has_negative_fraction(self) -> bool {
        self.neg
            && self.sig != 0
            && (self.exp < 0 || (self.exp < 63 && self.sig << self.exp << 1 != 0))
    }

    /// `fscale` by a whole number: exact.
    pub const fn scale(self, n: i32) -> F80 {
        if self.sig == 0 {
            self
        } else {
            F80 {
                exp: self.exp + n,
                ..self
            }
        }
    }

    /// `fsin`: the sine of the argument reduced by Intel's 66-bit pi, rounded to 64 bits. At 2^63 and beyond,
    /// the instruction leaves its operand unchanged (and sets C2, which Delphi's `Sin` doesn't check).
    pub const fn sin(self) -> F80 {
        if self.sig == 0 || self.exp >= 63 {
            return self;
        }
        let (q, r) = reduce(self);
        let s = match q {
            0 => wide::sin_series(r),
            1 => wide::cos_series(r),
            2 => wide::sin_series(r).neg(),
            _ => wide::cos_series(r).neg(),
        };
        (if self.neg { s.neg() } else { s }).to_f80()
    }

    /// `fcos`: as [`F80::sin`].
    pub const fn cos(self) -> F80 {
        if self.sig == 0 {
            return F80::ONE;
        }
        if self.exp >= 63 {
            return self;
        }
        let (q, r) = reduce(self);
        match q {
            0 => wide::cos_series(r),
            1 => wide::sin_series(r).neg(),
            2 => wide::cos_series(r).neg(),
            _ => wide::sin_series(r),
        }
        .to_f80()
    }

    /// `fsin` and `fcos` of the same operand, as [`F80::sin`] and [`F80::cos`] give them, reducing it once.
    pub const fn sin_cos(self) -> (F80, F80) {
        if self.sig == 0 {
            return (self, F80::ONE);
        }
        if self.exp >= 63 {
            return (self, self);
        }
        let (q, r) = reduce(self);
        let (s, c) = (wide::sin_series(r), wide::cos_series(r));
        let (sin, cos) = match q {
            0 => (s, c),
            1 => (c, s.neg()),
            2 => (s.neg(), c.neg()),
            _ => (c.neg(), s),
        };
        (
            (if self.neg { sin.neg() } else { sin }).to_f80(),
            cos.to_f80(),
        )
    }

    /// `fptan`'s tangent (it then pushes a 1, which Delphi's `Tan` pops), as [`F80::sin`]: at 2^63 and beyond,
    /// the operand unchanged (`fptan` sets C2 and pushes nothing; [`gm8::tan`] says what Delphi makes of that).
    pub const fn tan(self) -> F80 {
        if self.sig == 0 || self.exp >= 63 {
            return self;
        }
        let (q, r) = reduce(self);
        let (s, c) = (wide::sin_series(r), wide::cos_series(r));
        let t = if q % 2 == 0 { s.div(c) } else { c.div(s).neg() };
        (if self.neg { t.neg() } else { t }).to_f80()
    }

    /// `fpatan`: atan2(y, x) in (-pi, pi], rounded to 64 bits, with IEEE's signed zeros: atan2(+-0, x) is +-0
    /// for x = +0 or above and +-pi (`fldpi`) for x = -0 or below.
    pub const fn atan2(y: F80, x: F80) -> F80 {
        if y.sig == 0 {
            return if x.neg {
                F80 {
                    neg: y.neg,
                    ..F80::PI
                }
            } else {
                y
            };
        }
        if x.sig == 0 {
            return F80 {
                neg: y.neg,
                exp: 0,
                ..F80::PI
            };
        }
        let (a, b) = (Wide::from_f80(y.abs()), Wide::from_f80(x.abs()));
        let mut t = if b.abs_lt(a) {
            Wide::PI_2.sub(wide::atan01(b.div(a)))
        } else {
            wide::atan01(a.div(b))
        };
        if x.neg {
            t = Wide::PI.sub(t);
        }
        Wide { neg: y.neg, ..t }.to_f80()
    }

    /// `fldln2; fxch; fyl2x`, Delphi's `Ln`: ln(x) rounded to 64 bits, or `None` unless x > 0 (the x87's
    /// invalid-operation or division-by-zero exception, which GM8 doesn't mask).
    pub const fn ln(self) -> Option<F80> {
        if self.sig == 0 || self.neg {
            return None;
        }
        Some(wide::ln(Wide::from_f80(self)).to_f80())
    }

    /// `f2xm1`: 2^x - 1 rounded to 64 bits, for |x| <= 1 (the instruction's domain; `None` outside it).
    pub const fn exp2_m1(self) -> Option<F80> {
        if self.sig == 0 {
            return Some(self);
        }
        if self.exp > 0 || (self.exp == 0 && self.sig != 1 << 63) {
            return None;
        }
        Some(wide::expm1_series(Wide::from_f80(self).mul(Wide::LN2)).to_f80())
    }
}

/// Intel's 66-bit pi, `0xC90FDAA22168C234C / 2^66`, times 2^66: what `fsin`, `fcos` and `fptan` reduce by.
const PI66: u128 = 0xc90fdaa22168c234c;
/// 2 / Intel's pi, to nearest (mpmath): for the reduction's quotient.
const TWO_OVER_PI66: Wide = Wide {
    neg: false,
    exp: -1,
    m: 0xa2f9836e4e44152a00062bc40da276bf,
};

/// `|x| = k * pi66/2 + r` with k a whole number and |r| <= pi66/4, exactly, for 0 < |x| < 2^63: returns
/// (k mod 4, r).
const fn reduce(x: F80) -> (u32, Wide) {
    if x.exp < -1 {
        // |x| < 1/2 < pi66/4.
        return (0, Wide::from_f80(x.abs()));
    }
    // In units of 2^-67 (pi66/2 = PI66 * 2^-67), |x| is the whole number sig * 2^(exp + 4), below 2^131. Its
    // remainder by PI66 is exact in i128 as long as the true remainder is small, which it is once k is within a
    // few of the nearest quotient: so compute mod 2^128, from an estimate of k, then correct k.
    let xs = (x.sig as u128) << (x.exp + 4);
    let mut k = Wide::from_f80(x.abs()).mul(TWO_OVER_PI66).round_abs();
    let mut r = xs.wrapping_sub(k.wrapping_mul(PI66)) as i128;
    let (p, half) = (PI66 as i128, (PI66 / 2) as i128);
    while r > half {
        r -= p;
        k += 1;
    }
    while r < -half {
        r += p;
        k -= 1;
    }
    ((k % 4) as u32, Wide::from_int(r < 0, r.unsigned_abs(), -67))
}

#[cfg(test)]
mod tests;
