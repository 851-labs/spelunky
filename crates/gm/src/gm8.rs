//! GM8's own number rules where OpenGMK had its own, as `tools/gm8-probe` found them on the real GM8 runner
//! (851-3246): Delphi's integer conversions, `Floor`/`Ceil`, x86 shifts and `FloatToStrF`. `gm::math` and
//! `gm::string` document each rule for GML; this module holds the arithmetic.
//!
//! Every integer conversion here is the x87's `FISTP` into an `Int64`. GM8 runs with the x87's
//! invalid-operation exception unmasked, so on a real Intel CPU a value that doesn't fit (or a NaN) raises
//! instead of storing the "integer indefinite" -2^63 (`floor(1e20)` stops the game: 851-3247). Under Rosetta,
//! which doesn't trap, GM8 stores -2^63; the conversions return `None` where Intel traps. The overflow
//! exception is unmasked too, so an arithmetic result past the largest double traps as well (851-3259).
//!
//! It depends on nothing, because the oracle runs this same file: `tools/oracle/sync-gm8-arith` copies it into
//! patch 0014 as `gm8emulator/src/gml/gm8.rs`, and `scripts/check` fails if the patch's copy is out of date.

/// A whole real stored into an `Int64` by the x87's `FISTP`: `None` (the invalid-operation exception) for
/// anything outside the `i64` range, or NaN.
#[inline]
pub fn to_i64(r: f64) -> Option<i64> {
    // -2^63 is in range; 2^63 is not. A NaN fails both tests.
    (-9_223_372_036_854_775_808.0..9_223_372_036_854_775_808.0)
        .contains(&r)
        .then_some(r as i64)
}

/// Delphi's `Round`: half to even, into an `Int64`.
#[inline]
pub fn round(x: f64) -> Option<f64> {
    bits(x).map(|r| r as f64)
}

/// The result `r` of `a + b`, `a - b`, `a * b` or `a / b` (a nonzero `b`) as the x87 stores it into a double:
/// `None` where it raises an exception GM8 leaves unmasked. That's an overflow (an infinite result from finite
/// operands: `2^1023 * 2`, `1 / 2^-1074`) or an invalid operation (a NaN from operands that aren't, as
/// `inf - inf`, `0 * inf`). Otherwise an infinity or a NaN operand goes through as IEEE says (`inf + 1` is
/// inf), though GM8 on Intel can't make one.
/// Underflow is masked: `2^-1074 / 2` is 0.
#[inline]
pub fn arith(r: f64, a: f64, b: f64) -> Option<f64> {
    let trapped = if r.is_nan() {
        !a.is_nan() && !b.is_nan()
    } else {
        r.is_infinite() && a.is_finite() && b.is_finite()
    };
    (!trapped).then_some(r)
}

/// An integer argument to a built-in, an array index, an integer built-in variable's new value or a `repeat`
/// count: Delphi's `Round` into an `Int64` (so `None` past it, where Intel traps), cut to its low 32 bits.
#[inline]
pub fn int_arg(x: f64) -> Option<i32> {
    bits(x).map(|i| i as i32)
}

/// How many times `repeat (n)` runs, for `n` an [`int_arg`]: `n` itself when positive, else none, but for
/// -2^31 (`repeat (2^31)`), which runs. That fits a loop that decrements the count, then stops once it is
/// negative: -2^31 - 1 wraps to 2^31 - 1, so it would run 2^31 times (the probe stops at 5).
#[inline]
pub fn repeat_count(n: i32) -> u32 {
    if n == i32::MIN {
        1 << 31
    } else {
        n.max(0) as u32
    }
}

/// Delphi's `Trunc(x)` as a 32-bit `Integer`: the `Int64` truncation cut to its low 32 bits.
#[inline]
fn trunc_i32(x: f64) -> Option<i32> {
    to_i64(x.trunc()).map(|t| t as i32)
}

/// Delphi's `Frac`: `x - Int(x)`, a NaN for infinities.
#[inline]
fn frac(x: f64) -> f64 {
    x - x.trunc()
}

/// Delphi's `Floor`, a 32-bit `Integer`: `Trunc(x)`, minus 1 if `Frac(x) < 0`, so it wraps past 2^31.
#[inline]
pub fn floor(x: f64) -> Option<f64> {
    let t = trunc_i32(x)?;
    Some(f64::from(t.wrapping_sub(i32::from(frac(x) < 0.0))))
}

/// Delphi's `Ceil`: `Trunc(x)`, plus 1 if `Frac(x) > 0`.
#[inline]
pub fn ceil(x: f64) -> Option<f64> {
    let t = trunc_i32(x)?;
    Some(f64::from(t.wrapping_add(i32::from(frac(x) > 0.0))))
}

/// The quotient of `div` and `mod`: `Floor(a / b)` when that is at least 0, else `Ceil(a / b)`.
#[inline]
pub fn quotient(a: f64, b: f64) -> Option<f64> {
    let q = a / b;
    if q >= 0.0 { floor(q) } else { ceil(q) }
}

/// The integer a bitwise operator sees: rounded half to even into an `Int64`.
#[inline]
pub fn bits(x: f64) -> Option<i64> {
    to_i64(x.round_ties_even())
}

/// `a << n`, or logical `a >> n`, as GM8's 64-bit shift helpers take the count: its low byte, read as a signed
/// byte `c`. For `c < 32`, negatives included, the shift is by `c & 31`; for `32 <= c < 64`, by `c`; from 64 up
/// the result is 0.
#[inline]
pub fn shift(a: f64, n: f64, left: bool) -> Option<f64> {
    let v = bits(a)?;
    let c = bits(n)? as u8 as i8;
    let by = match c {
        ..32 => (c & 31) as u32,
        32..64 => c as u32,
        _ => return Some(0.0),
    };
    Some(
        (if left {
            v << by
        } else {
            ((v as u64) >> by) as i64
        }) as f64,
    )
}

/// GML `sign`: with the comparison tolerance (1e-13), and 1 for a NaN.
#[inline]
pub fn sign(x: f64) -> f64 {
    if x <= -1e-13 {
        -1.0
    } else if x < 1e-13 {
        0.0
    } else {
        1.0
    }
}

/// GML `string(x)` for a real: the integer if `x == round(x)`, else `FloatToStrF(x, ffFixed, 10, 2)`. Where
/// that `Round` traps (from 2^63 up, infinities, NaN), `string` catches it and gives `"ERROR"`.
pub fn real_to_string(x: f64) -> String {
    match round(x) {
        None => "ERROR".to_owned(),
        Some(r) if x == r => (r as i64).to_string(),
        Some(_) => float_to_str_fixed(x, 10, 2, 2),
    }
}

/// GML `string_format(x, total, decimals)` for a real: `FloatToStrF(x, ffFixed, 18, decimals)` with at most
/// 18 decimals, right-aligned in `total` characters. A negative `decimals` rounds (half up, on the digits) to a
/// multiple of `10^-decimals`, and then prints 18 decimals: `string_format(125, 0, -1)` is
/// `"130.000000000000000000"`.
pub fn string_format(x: f64, total: i32, decimals: i32) -> String {
    let decimals = decimals.min(18);
    let shown = if decimals < 0 { 18 } else { decimals };
    let s = float_to_str_fixed(x, 18, decimals, shown);
    let width = total.max(0) as usize;
    format!("{s:>width$}")
}

/// Exact powers of ten up to 10^22, the largest that a double holds exactly.
const POW10: [f64; 23] = [
    1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15, 1e16,
    1e17, 1e18, 1e19, 1e20, 1e21, 1e22,
];

/// `a * 10^n` the way Delphi's `FPower10` computes it under 53-bit precision: one multiplication or division
/// by an exact power for `|n| <= 22`. Beyond that Delphi steps through its 80-bit power table, which this
/// approximates with doubles; nothing in the game gets there.
fn scale10(a: f64, n: i32) -> f64 {
    let mut s = a;
    let mut k = n.unsigned_abs() as usize;
    while k > 0 {
        let step = k.min(22);
        s = if n > 0 {
            s * POW10[step]
        } else {
            s / POW10[step]
        };
        k -= step;
    }
    s
}

/// Delphi 7's `FloatToStrF(x, ffFixed, precision, decimals)` with the x87 rounding to 53 bits, as GM8 runs
/// it, rounding at `decimals` and printing `shown` decimals. `FloatToDecimal` estimates the decimal exponent `e` from the binary one, scales `|x|` by
/// `10^(18 - e)` (rounded to a double), and, if that reached 10^18, divides by 10 (rounded again) and bumps
/// `e`: the 18-digit integer that leaves holds the digits. They are rounded half up at
/// `min(precision, e + decimals)` digits; a result that rounds to zero loses its sign. `FloatToText` then
/// prints `decimals` decimals, or, with more than `precision` integer digits, `d.dddE<e - 1>` without
/// trailing zeros.
fn float_to_str_fixed(x: f64, precision: i32, decimals: i32, shown: i32) -> String {
    if x.is_nan() {
        return "NAN".to_owned();
    }
    if x.is_infinite() {
        return if x > 0.0 { "INF" } else { "-INF" }.to_owned();
    }
    let a = x.abs();
    // The binary exponent the x87 sees (it normalizes subnormals), then Delphi's estimate of the decimal one:
    // floor(e2 * log10(2)) + 1, with log10(2) as 19728 / 2^16.
    let e2 = if a < f64::MIN_POSITIVE {
        -1011 - a.to_bits().leading_zeros() as i32
    } else {
        ((a.to_bits() >> 52) as i32) - 1023
    };
    let mut e = ((e2 * 19728) >> 16) + 1;
    let mut s = scale10(a, 18 - e);
    if s >= 1e18 {
        s /= 10.0;
        e += 1;
    }
    // s is whole (it is at least 2^53, or 0); FBSTP stores it as 18 BCD digits.
    let mut digits: Vec<u8> = format!("{:018}", s as u64).into_bytes();
    let keep = e.saturating_add(decimals).min(precision);
    if keep < 0 {
        digits.clear();
    } else if (keep as usize) < digits.len() {
        let keep = keep as usize;
        let up = digits[keep] >= b'5';
        digits.truncate(keep);
        if up {
            match digits.iter().rposition(|&d| d != b'9') {
                Some(i) => {
                    digits[i] += 1;
                    digits[i + 1..].fill(b'0');
                }
                None => {
                    digits.fill(b'0');
                    digits.insert(0, b'1');
                    e += 1;
                }
            }
        }
    }
    while digits.last() == Some(&b'0') {
        digits.pop();
    }
    let mut out = String::new();
    if digits.is_empty() {
        e = 0;
    } else if x < 0.0 {
        out.push('-');
    }
    let d = |i: i32| {
        usize::try_from(i)
            .ok()
            .and_then(|i| digits.get(i))
            .map_or('0', |&c| char::from(c))
    };
    if e > precision {
        out.push(d(0));
        if digits.len() > 1 {
            out.push('.');
            out.extend((1..digits.len() as i32).map(d));
        }
        out.push_str(&format!("E{}", e - 1));
    } else {
        if e <= 0 {
            out.push('0');
        } else {
            out.extend((0..e).map(d));
        }
        if shown > 0 {
            out.push('.');
            out.extend((e..e + shown).map(d));
        }
    }
    out
}
