// Written for the Spelunky port (851-3257), not part of upstream pxfm. Distributed under the same terms as the
// rest of this crate (BSD-3-Clause OR Apache-2.0); see ../../README.spelunky.md.

//! A 128-bit software float for computing the x87's transcendental results before they are rounded to 64 bits.
//!
//! The x87 model rounds the *exact* sine, cosine, arctangent, logarithm or power of two to 64 bits, so these only
//! need to be far more precise than that: every operation here truncates (or rounds) to 128 bits, and a series of
//! a few dozen of them stays within about 2^-120 of the true value. Rounding that to 64 bits can only go the wrong
//! way when the true value lies within 2^-120 of a halfway point between two 64-bit values, which for these
//! functions happens about once in 2^55 arguments. Everything is integer arithmetic, so every target computes the
//! same bits.

use super::{F80, round};

/// `(-1)^neg * m * 2^(exp - 127)`, with `m`'s top bit set, or zero (`m == 0`).
#[derive(Clone, Copy, Debug)]
pub(super) struct Wide {
    pub neg: bool,
    pub exp: i32,
    pub m: u128,
}

const LOW64: u128 = u64::MAX as u128;

impl Wide {
    pub const ZERO: Wide = Wide {
        neg: false,
        exp: 0,
        m: 0,
    };
    pub const ONE: Wide = Wide {
        neg: false,
        exp: 0,
        m: 1 << 127,
    };
    /// pi, to nearest (mpmath).
    pub const PI: Wide = Wide {
        neg: false,
        exp: 1,
        m: 0xc90fdaa22168c234c4c6628b80dc1cd1,
    };
    /// pi / 2.
    pub const PI_2: Wide = Wide {
        neg: false,
        exp: 0,
        m: Wide::PI.m,
    };
    /// ln 2, to nearest (mpmath).
    pub const LN2: Wide = Wide {
        neg: false,
        exp: -1,
        m: 0xb17217f7d1cf79abc9e3b39803f2f6af,
    };

    /// Normalizes `(-1)^neg * m * 2^(exp - 127)`.
    pub const fn new(neg: bool, exp: i32, m: u128) -> Wide {
        if m == 0 {
            return Wide { neg, exp: 0, m: 0 };
        }
        let lz = m.leading_zeros();
        Wide {
            neg,
            exp: exp - lz as i32,
            m: m << lz,
        }
    }

    /// `(-1)^neg * mag * 2^scale`, exactly.
    pub const fn from_int(neg: bool, mag: u128, scale: i32) -> Wide {
        Wide::new(neg, 127 + scale, mag)
    }

    pub const fn from_f80(x: F80) -> Wide {
        Wide::new(x.neg, x.exp, (x.sig as u128) << 64)
    }

    /// Rounded to 64 bits, to nearest: the x87's result.
    pub const fn to_f80(self) -> F80 {
        round(self.neg, self.exp, self.m, false, 64)
    }

    pub const fn is_zero(self) -> bool {
        self.m == 0
    }

    pub const fn neg(self) -> Wide {
        Wide {
            neg: !self.neg,
            ..self
        }
    }

    /// Whether `|self| < |other|`.
    pub const fn abs_lt(self, other: Wide) -> bool {
        if self.m == 0 || other.m == 0 {
            return other.m != 0;
        }
        self.exp < other.exp || (self.exp == other.exp && self.m < other.m)
    }

    /// The product, truncated to 128 bits.
    pub const fn mul(self, o: Wide) -> Wide {
        if self.m == 0 || o.m == 0 {
            return Wide::ZERO;
        }
        let (ah, al) = (self.m >> 64, self.m & LOW64);
        let (bh, bl) = (o.m >> 64, o.m & LOW64);
        let (hl, lh) = (ah * bl, al * bh);
        let (lo, c1) = (al * bl).overflowing_add(hl << 64);
        let (lo, c2) = lo.overflowing_add(lh << 64);
        let hi = ah * bh + (hl >> 64) + (lh >> 64) + c1 as u128 + c2 as u128;
        // The product is hi * 2^128 + lo, in [2^254, 2^256).
        let neg = self.neg != o.neg;
        if hi >> 127 != 0 {
            Wide {
                neg,
                exp: self.exp + o.exp + 1,
                m: hi,
            }
        } else {
            Wide {
                neg,
                exp: self.exp + o.exp,
                m: (hi << 1) | (lo >> 127),
            }
        }
    }

    /// The quotient, truncated to 128 bits. `o` must not be zero.
    pub const fn div(self, o: Wide) -> Wide {
        assert!(o.m != 0, "Wide::div by zero");
        if self.m == 0 {
            return Wide::ZERO;
        }
        // Restoring division, one quotient bit at a time: q collects floor(self.m * 2^127 / o.m), or
        // floor(self.m * 2^128 / o.m) when self.m < o.m, and either way ends with its top bit set.
        let (mut q, mut r, exp, n) = if self.m >= o.m {
            (1u128, self.m - o.m, self.exp - o.exp, 127)
        } else {
            (0u128, self.m, self.exp - o.exp - 1, 128)
        };
        let mut i = 0;
        while i < n {
            let carry = r >> 127;
            r <<= 1;
            q <<= 1;
            if carry != 0 || r >= o.m {
                r = r.wrapping_sub(o.m);
                q |= 1;
            }
            i += 1;
        }
        Wide {
            neg: self.neg != o.neg,
            exp,
            m: q,
        }
    }

    /// The quotient by a small whole number, truncated to 128 bits.
    pub const fn div_small(self, d: u64) -> Wide {
        if self.m == 0 {
            return Wide::ZERO;
        }
        let d = d as u128;
        let (q, r) = (self.m / d, self.m % d);
        let lz = q.leading_zeros();
        // r < d < 2^64 and lz <= 64, so r << lz fits, and its quotient fills the lz bits q's shift leaves.
        Wide {
            neg: self.neg,
            exp: self.exp - lz as i32,
            m: (q << lz) | ((r << lz) / d),
        }
    }

    /// The sum, with 127 bits kept of the operands once aligned.
    pub const fn add(self, o: Wide) -> Wide {
        if o.m == 0 {
            return self;
        }
        if self.m == 0 {
            return o;
        }
        let (x, y) = if o.abs_lt(self) { (self, o) } else { (o, self) };
        let d = (x.exp - y.exp) as u32;
        // One bit of headroom for the carry.
        let mx = x.m >> 1;
        let my = if d >= 127 { 0 } else { (y.m >> 1) >> d };
        let m = if x.neg == y.neg { mx + my } else { mx - my };
        Wide::new(x.neg, x.exp + 1, m)
    }

    pub const fn sub(self, o: Wide) -> Wide {
        self.add(o.neg())
    }

    /// `|self|` rounded to a whole number (halves up). `|self|` must be below 2^126.
    pub const fn round_abs(self) -> u128 {
        if self.m == 0 || self.exp < -1 {
            return 0;
        }
        let sh = (127 - self.exp) as u32;
        ((self.m >> (sh - 1)) + 1) >> 1
    }
}

/// Whether a series term no longer changes a sum of this size at 128 bits.
const fn negligible(term: Wide, sum: Wide) -> bool {
    term.m == 0 || term.exp < sum.exp - 130
}

/// sin(r) for |r| <= 0.8: r - r^3/3! + r^5/5! - ..., about 15 terms at most.
pub(super) const fn sin_series(r: Wide) -> Wide {
    let r2 = r.mul(r);
    let (mut term, mut sum, mut n) = (r, r, 1u64);
    loop {
        term = term.mul(r2).div_small((n + 1) * (n + 2)).neg();
        n += 2;
        if negligible(term, sum) {
            return sum;
        }
        sum = sum.add(term);
    }
}

/// cos(r) for |r| <= 0.8: 1 - r^2/2! + r^4/4! - ...
pub(super) const fn cos_series(r: Wide) -> Wide {
    let r2 = r.mul(r);
    let (mut term, mut sum, mut n) = (Wide::ONE, Wide::ONE, 0u64);
    loop {
        term = term.mul(r2).div_small((n + 1) * (n + 2)).neg();
        n += 2;
        if negligible(term, sum) {
            return sum;
        }
        sum = sum.add(term);
    }
}

/// atan(u) for |u| <= 1/16: u - u^3/3 + u^5/5 - ..., about 16 terms at most.
const fn atan_series(u: Wide) -> Wide {
    let u2 = u.mul(u);
    let (mut power, mut sum, mut k) = (u, u, 1u64);
    loop {
        power = power.mul(u2).neg();
        let term = power.div_small(2 * k + 1);
        k += 1;
        if negligible(term, sum) {
            return sum;
        }
        sum = sum.add(term);
    }
}

/// atan(j / 8) for j = 1 to 8, to nearest (mpmath), for [`atan01`]'s reduction.
pub(super) const ATAN_EIGHTHS: [Wide; 8] = [
    Wide {
        neg: false,
        exp: -4,
        m: 0xfeadd4d5617b6e32c897989f3e888ef8,
    },
    Wide {
        neg: false,
        exp: -3,
        m: 0xfadbafc96406eb156dc79ef5f7a217e6,
    },
    Wide {
        neg: false,
        exp: -2,
        m: 0xb7b0ca0f26f784738aa32122dcfe4483,
    },
    Wide {
        neg: false,
        exp: -2,
        m: 0xed63382b0dda7b456fe445ecbc3a8d03,
    },
    Wide {
        neg: false,
        exp: -1,
        m: 0x8f005d5ef7f59f9b5c835e1665c43748,
    },
    Wide {
        neg: false,
        exp: -1,
        m: 0xa4bc7d1934f7092419a87f2a457dac9f,
    },
    Wide {
        neg: false,
        exp: -1,
        m: 0xb8053e2bc2319e73cb2da55210a4443d,
    },
    Wide {
        neg: false,
        exp: -1,
        m: 0xc90fdaa22168c234c4c6628b80dc1cd1,
    },
];

/// atan(t) for 0 <= t <= 1: with c = j/8 the nearest eighth, atan(t) = atan(c) + atan((t - c) / (1 + t c)),
/// whose argument is at most 1/16.
pub(super) const fn atan01(t: Wide) -> Wide {
    let j = Wide {
        exp: t.exp + 3,
        ..t
    }
    .round_abs();
    if j == 0 {
        return atan_series(t);
    }
    let c = Wide::from_int(false, j, -3);
    let u = t.sub(c).div(Wide::ONE.add(t.mul(c)));
    ATAN_EIGHTHS[j as usize - 1].add(atan_series(u))
}

/// e^g - 1 for |g| <= 0.7: g + g^2/2! + g^3/3! + ..., about 30 terms at most.
pub(super) const fn expm1_series(g: Wide) -> Wide {
    let (mut term, mut sum, mut n) = (g, g, 1u64);
    loop {
        n += 1;
        term = term.mul(g).div_small(n);
        if negligible(term, sum) {
            return sum;
        }
        sum = sum.add(term);
    }
}

/// ln(x) for x > 0, x = m * 2^e with m in [sqrt(1/2), sqrt(2)): e ln 2 + 2 atanh((m - 1) / (m + 1)), whose
/// argument is at most 0.172. When e isn't 0, |ln m| < |e ln 2| / 2, so the sum doesn't cancel.
pub(super) const fn ln(x: Wide) -> Wide {
    // m in [1, 2) first; above sqrt(2) (0xb504f333... / 2^127), halve it.
    let (mut e, mut m) = (x.exp, Wide { exp: 0, ..x });
    if m.m > 0xb504f333f9de6484597d89b3754abe9f {
        m.exp = -1;
        e += 1;
    }
    let u = m.sub(Wide::ONE).div(m.add(Wide::ONE));
    let u2 = u.mul(u);
    let (mut power, mut sum, mut k) = (u, u, 1u64);
    loop {
        power = power.mul(u2);
        let term = power.div_small(2 * k + 1);
        k += 1;
        if negligible(term, sum) {
            break;
        }
        sum = sum.add(term);
    }
    let lnm = Wide {
        exp: sum.exp + 1,
        ..sum
    };
    Wide::from_int(e < 0, e.unsigned_abs() as u128, 0)
        .mul(Wide::LN2)
        .add(lnm)
}
