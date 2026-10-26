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
    stored(r, [a, b])
}

/// The result `r` of a built-in computed on the x87 from `operands` (`point_distance`), as [`arith`]: `None`
/// for a NaN from operands that aren't, or an infinity from finite ones.
#[inline]
pub fn stored<const N: usize>(r: f64, operands: [f64; N]) -> Option<f64> {
    let trapped = if r.is_nan() {
        !operands.iter().any(|x| x.is_nan())
    } else {
        r.is_infinite() && operands.iter().all(|x| x.is_finite())
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

/// The longest string `real * string` builds, in bytes: 304 MiB (19 * 2^24). GM8's runner is a 32-bit process,
/// and a longer string runs it out of memory, which stops the game with the same "Unexpected error" dialog as
/// the x87's traps. GM8 builds 304 MiB of `"a"` but not 312 MiB, on a real Intel CPU (twice) and under Rosetta
/// alike, and the limit is on bytes rather than copies: 2^23 copies of a 64-byte string stop it, where 2^23 of
/// `"a"` wouldn't (probes `mulstr-2p27` to `mulstr-2p30`, `mulstr-2p28+1`, `mulstr-19x2p24`, `mulstr-39x2p23`,
/// `mulstr-2p22-64`, ...; 851-3282). The edge itself is the process's free address space, not a rule, so this
/// is the longest length measured to work: the port and the oracle stop at most 8 MiB before GM8 under Wine
/// (`docs/quirks.md` §7), and never build gigabytes, which a wasm32 build can't unwind from failing to allocate.
pub const REPEAT_MAX_LEN: usize = 19 << 24;

/// The length of `n` copies of a `len`-byte string (`n` a [`repeat_count`]), or `None` past
/// [`REPEAT_MAX_LEN`], where GM8 runs out of memory.
#[inline]
pub fn repeat_len(len: usize, n: u32) -> Option<usize> {
    len.checked_mul(n as usize)
        .filter(|&total| total <= REPEAT_MAX_LEN)
}

/// `make_color_rgb(r, g, b)`: each component Delphi's `Round` into an `Int64` (so `None` past it, where Intel
/// traps), uncut, then `r + g * 256 + b * 65536` in 64 bits, wrapping: overlapping bits add rather than merge
/// (`(-1, 1, 0)` is 255), `(0, 0, 2^47)` is -2^63 and `(0, 0, 2^48)` is 0. Stored as a double, so rounded to
/// 53 bits past 2^53.
#[inline]
pub fn make_color_rgb(r: f64, g: f64, b: f64) -> Option<f64> {
    let (r, g, b) = (bits(r)?, bits(g)?, bits(b)?);
    Some(
        r.wrapping_add(g.wrapping_mul(256))
            .wrapping_add(b.wrapping_mul(65536)) as f64,
    )
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

// ---- Drawing's own conversions (851-3304) ----
//
// GM8's drawing routines (Spelunky.exe, read only to describe them) hand Direct3D each corner as a single, and some
// round or floor a position into an `Int64` first; these say where that traps (probes `draw-builtin-trap`,
// `draw-default-trap`, `draw-stage-trap`). They compute in doubles, as the x87 does at GM8's 53-bit precision, but
// for the register's wider exponent: where a sum or product in a register goes past the largest double, it's
// infinite here, and either GM8 stores it as a double, which traps, or a corner it leads to is past the largest
// single, which traps too.

/// From here up, the x87 rounds a double it stores as a single (`FSTP` to 32 bits) past the largest single: 2^128 -
/// 2^103 is halfway to 2^128, where a tie rounds. That's the overflow exception, which GM8 leaves unmasked.
pub const SINGLE_LIMIT: f64 = 340_282_356_779_733_661_637_539_395_458_142_568_448.0;

/// Which of the x87's exceptions a draw raises, where one routine can raise either.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawTrap {
    /// An integer store's invalid operation: `FISTP` past the `Int64` range.
    Invalid,
    /// A store's overflow: past the largest double or single.
    Overflow,
}

/// `v` stored as a single: `None` where that traps. An infinity or NaN went past the largest double on the way.
#[inline]
pub fn single(v: f64) -> Option<()> {
    (v.abs() < SINGLE_LIMIT).then_some(())
}

/// `v` stored as a double: `None` where it went past the largest double (an infinity, or a NaN from one).
#[inline]
fn double(v: f64) -> Option<f64> {
    v.is_finite().then_some(v)
}

/// The plain texture draw (0x4a3a38; `draw_sprite`, an untransformed default draw, `draw_surface`): a `w` x `h`
/// texture with its corner at `(x, y)` is a quad from `(x - 0.5, y - 0.5)` to `(w + x - 0.5, h + y - 0.5)`.
#[inline]
pub fn draw_quad(x: f64, y: f64, w: f64, h: f64) -> Option<()> {
    single(x - 0.5)?;
    single(y - 0.5)?;
    single((w + x) - 0.5)?;
    single((h + y) - 0.5)
}

/// The scaled and rotated texture draw (0x4a3ad8; `draw_sprite_ext`, a transformed default draw, `draw_text`'s
/// characters, `draw_surface_stretched`), with `angle` in radians. It stores the corners' offsets as doubles:
/// `-xscale * xorig - 0.5`, and `w * xscale` plus that (as for y). Below 0.001 radians (an 80-bit constant, which
/// no double falls between) the corners are `x` plus those; from there each corner's offsets `(u, v)` are rotated,
/// `(u * cos + x) + v * sin` and `(y - u * sin) + v * cos`, by the sine and cosine `sin_cos` gives (`fsin` and
/// `fcos` stored as doubles, which give the angle back from 2^63 radians). Each corner goes to a double, then a
/// single.
#[inline]
pub fn draw_quad_ext(
    origin: (f64, f64),
    at: (f64, f64),
    scale: (f64, f64),
    size: (f64, f64),
    angle: f64,
    sin_cos: impl FnOnce(f64) -> (f64, f64),
) -> Option<()> {
    let ((xo, yo), (x, y), (xs, ys), (w, h)) = (origin, at, scale, size);
    let u1 = double(-xs * xo - 0.5)?;
    let v1 = double(-ys * yo - 0.5)?;
    let u2 = double(w * xs + u1)?;
    let v2 = double(h * ys + v1)?;
    if angle.abs() < 0.001 {
        for c in [x + u1, y + v1, x + u2, y + v2] {
            single(c)?;
        }
    } else {
        // Below 2^63 radians the sine and cosine are at most 1, so a corner is within `|x| + |u| + |v|` (or y's)
        // but for rounding: well below the limit, nothing needs them.
        let reach = |p: f64| p.abs() + u1.abs().max(u2.abs()) + v1.abs().max(v2.abs());
        if angle.abs() < 9_223_372_036_854_775_808.0 && reach(x).max(reach(y)) < SINGLE_LIMIT * 0.5
        {
            return Some(());
        }
        let (s, c) = sin_cos(angle);
        for (u, v) in [(u1, v1), (u2, v1), (u2, v2), (u1, v2)] {
            single((u * c + x) + v * s)?;
            single((y - u * s) + v * c)?;
        }
    }
    Some(())
}

/// Whether the default draw of an instance with this transform is the plain texture draw (Spelunky.exe 0x4bf505):
/// scale 1, angle 0, white and opaque (each compared exactly); otherwise it's the scaled and rotated one.
#[inline]
pub fn draw_is_plain(xscale: f64, yscale: f64, angle: f64, blend: i32, alpha: f64) -> bool {
    xscale == 1.0 && yscale == 1.0 && angle == 0.0 && blend == 0xFF_FF_FF && alpha == 1.0
}

/// Whether `draw_text` draws any character of `text` in a font with the characters `chars`: GM8 skips the others
/// (Spelunky.exe 0x5193a8), and a line break draws nothing (`#`, `\n`, `\r`, but `\#` is a `#`). Its corners are the
/// text's position plus some pixels, far below the doubles' spacing where a corner reaches [`SINGLE_LIMIT`], so they
/// trap exactly where `x` or `y` does.
pub fn text_draws(text: &[u8], chars: core::ops::RangeInclusive<u32>) -> bool {
    let mut bytes = text.iter().copied().peekable();
    while let Some(c) = bytes.next() {
        let c = match c {
            b'#' | b'\n' | b'\r' => continue,
            b'\\' if bytes.peek() == Some(&b'#') => {
                bytes.next();
                b'#'
            }
            c => c,
        };
        if chars.contains(&u32::from(c)) {
            return true;
        }
    }
    false
}

/// `draw_rectangle` (0x49d270): it swaps each pair if the second is less, takes Delphi's [`floor`] of the larger x
/// and y (to move an edge on a whole pixel by 0.01, which keeps it below 2^63), and stores the corners as singles,
/// then draws them outlined or not.
#[inline]
pub fn draw_rectangle(x1: f64, y1: f64, x2: f64, y2: f64) -> Result<(), DrawTrap> {
    let (x1, x2) = if x2 < x1 { (x2, x1) } else { (x1, x2) };
    let (y1, y2) = if y2 < y1 { (y2, y1) } else { (y1, y2) };
    floor(x2).ok_or(DrawTrap::Invalid)?;
    floor(y2).ok_or(DrawTrap::Invalid)?;
    single(x1).ok_or(DrawTrap::Overflow)?;
    single(y1).ok_or(DrawTrap::Overflow)
}

/// `draw_circle` (0x49d844, then the ellipse 0x49d6ec): it stores `x - r`, `y - r`, `x + r` and `y + r` as doubles,
/// then the centre (their means) and the rim (half their differences, unsigned, times the cosines and sines of
/// `circle_precision` steps, plus the centre) as singles, then draws them outlined or not. The precision is a
/// multiple of 4, so the steps' cosines and sines (stored as doubles) include exactly 1 and -1: the rim's extremes
/// are the centre plus and minus the radius.
#[inline]
pub fn draw_circle(x: f64, y: f64, r: f64) -> Option<()> {
    let (x1, y1) = (double(x - r)?, double(y - r)?);
    let (x2, y2) = (double(x + r)?, double(y + r)?);
    let (cx, cy) = ((x1 + x2) / 2.0, (y1 + y2) / 2.0);
    let (rx, ry) = (((x1 - x2) / 2.0).abs(), ((y1 - y2) / 2.0).abs());
    for c in [cx, cy, rx + cx, -rx + cx, ry + cy, -ry + cy] {
        single(c)?;
    }
    Some(())
}

/// The left edge and width of the room a view's background layers are drawn over, from its `view_xview`,
/// `view_wview`, `view_hview` and `view_angle` (Spelunky.exe 0x4bf8f5): the view's own, or, from 0.01 degrees (an
/// 80-bit constant, which no double falls between), half its height wider each way, in wrapping 32-bit arithmetic.
/// GM8 takes it once, before the view's draw events, for its foreground layers too.
#[inline]
pub fn view_span(x: i32, w: i32, h: i32, angle: f64) -> (i32, i32) {
    if angle.abs() < 0.01 {
        (x, w)
    } else {
        let left = x.wrapping_sub(h / 2);
        let right = x.wrapping_add(w).wrapping_add(h / 2);
        (left, right.wrapping_sub(left))
    }
}

/// A background layer's tiled draw (0x4a459c) of a `w` x `h` background over a view starting at `left`, `width`
/// wide ([`view_span`]). It rounds `w * xscale` and `h * yscale` into `Int64`s and keeps 32 bits, drawing nothing if
/// either is 0 or less, then rounds `x` (32 bits kept). Tiled horizontally, it moves that column by whole tiles to
/// just before the view's left edge, past it if the column is right of 0. Only if the column is then left of the
/// view's right edge does it round `y`. The corners it stores are 32-bit columns and rows plus a tile, which never
/// trap.
#[inline]
pub fn draw_background(
    size: (f64, f64),
    at: (f64, f64),
    scale: (f64, f64),
    tile_h: bool,
    view: (i32, i32),
) -> Option<()> {
    let ((w, h), (x, y), (xs, ys), (left, width)) = (size, at, scale, view);
    let tw = bits(w * xs)? as i32;
    let th = bits(h * ys)? as i32;
    if tw <= 0 || th <= 0 {
        return Some(());
    }
    let mut col = i64::from(bits(x)? as i32);
    if tile_h {
        let tw = i64::from(tw);
        col %= tw;
        if col > 0 {
            col -= tw;
        }
        // GM8 steps one tile at a time while `col + tw < left`.
        let short = i64::from(left) - (col + tw);
        if short > 0 {
            col += (short + tw - 1) / tw * tw;
        }
    }
    if (col as f64) < f64::from(left) + f64::from(width) {
        bits(y)?;
    }
    Some(())
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
