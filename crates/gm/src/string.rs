//! GML strings and the string built-ins Spelunky uses (851-3125).
//!
//! Spelunky is a GameMaker 8.0 game, and GM 8.0 strings are byte strings in the system ANSI code page
//! (Windows-1252 for the original), not UTF-8. Lengths, positions and `ord` count bytes. OpenGMK
//! `gm8emulator/src/gml/kernel.rs` takes the `Version::GameMaker8_0` branch of each string function for
//! this. String positions are 1-based.

use crate::value::ValueError;
use std::fmt;
use std::sync::Arc;

/// An immutable GML string: shared bytes, so cloning a `Value` never copies text.
///
/// `Arc` rather than `Rc` so a world holding strings can move between threads (parallel RL environments).
/// The empty string is `None`, so producing one never allocates; constructors keep that invariant.
#[derive(Clone, Default)]
pub struct GmString(Option<Arc<[u8]>>);

impl GmString {
    /// The raw bytes.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_deref().unwrap_or_default()
    }

    /// Length in bytes (GML `string_length`).
    #[inline]
    pub fn len(&self) -> usize {
        self.as_bytes().len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_none()
    }

    /// Wraps already-shared bytes, keeping the empty string allocation-free.
    #[inline]
    fn from_arc(bytes: Arc<[u8]>) -> Self {
        Self((!bytes.is_empty()).then_some(bytes))
    }

    /// `copies` copies of `bytes`, in one allocation (a mapped range has an exact length): `real * string` can
    /// make 304 MiB, and a `Vec` copied into the `Arc` would need twice that at once.
    pub(crate) fn repeat(bytes: &[u8], copies: usize) -> Self {
        let n = bytes.len();
        Self::from_arc((0..n * copies).map(|i| bytes[i % n]).collect())
    }

    /// `a` followed by `b`, in one allocation (the chained slice iterators have an exact length).
    pub(crate) fn concat2(a: &[u8], b: &[u8]) -> Self {
        Self::from_arc(a.iter().chain(b).copied().collect())
    }

    /// `a`, `b` and `c` joined, in one allocation.
    fn concat3(a: &[u8], b: &[u8], c: &[u8]) -> Self {
        Self::from_arc(a.iter().chain(b).chain(c).copied().collect())
    }
}

impl PartialEq for GmString {
    fn eq(&self, other: &Self) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl Eq for GmString {}

impl PartialOrd for GmString {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for GmString {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_bytes().cmp(other.as_bytes())
    }
}

impl std::hash::Hash for GmString {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_bytes().hash(state);
    }
}

impl From<&[u8]> for GmString {
    fn from(bytes: &[u8]) -> Self {
        Self::from_arc(bytes.into())
    }
}

impl From<Vec<u8>> for GmString {
    fn from(bytes: Vec<u8>) -> Self {
        Self::from_arc(bytes.into())
    }
}

/// Takes the UTF-8 bytes as they are. Fine for ASCII; text from the `.gmk` should come in as raw bytes.
impl From<&str> for GmString {
    fn from(s: &str) -> Self {
        Self::from(s.as_bytes())
    }
}

impl From<String> for GmString {
    fn from(s: String) -> Self {
        Self::from(s.into_bytes())
    }
}

/// The raw bytes, for savestates. Strings that shared bytes before saving get their own copies after
/// loading, which GML can't tell apart: strings are immutable.
impl serde::Serialize for GmString {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(self.as_bytes())
    }
}

impl<'de> serde::Deserialize<'de> for GmString {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Bytes;
        impl<'de> serde::de::Visitor<'de> for Bytes {
            type Value = GmString;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a byte string")
            }
            fn visit_bytes<E>(self, v: &[u8]) -> Result<GmString, E> {
                Ok(v.into())
            }
            fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<GmString, E> {
                Ok(v.into())
            }
        }
        d.deserialize_bytes(Bytes)
    }
}

/// Bytes as the chars U+0000-U+00FF (Latin-1, close to Windows-1252): lossless, and the inverse of
/// [`from_latin1`]. How file names and saved files become text.
pub fn latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| char::from(b)).collect()
}

/// The inverse of [`latin1`]; `None` for a char past U+00FF.
pub fn from_latin1(s: &str) -> Option<Vec<u8>> {
    s.chars().map(|c| u8::try_from(c).ok()).collect()
}

/// Shows the bytes as Latin-1 (close to Windows-1252), for logs and debugging only.
impl fmt::Display for GmString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_bytes()
            .iter()
            .try_for_each(|&b| fmt::Write::write_char(f, char::from(b)))
    }
}

impl fmt::Debug for GmString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.to_string())
    }
}

/// GML `string(x)` for a real, as GM8 does it (probes `numbers`, `strings`, `math`). A real equal to its own
/// `round` (an `Int64`, see [`math::round`]) prints as that integer, so `-0` is `"0"`. Anything else goes
/// through Delphi's `FloatToStrF(x, ffFixed, 10, 2)` ([`fixed_2`]): two decimals, rounded half away from zero
/// on an 18-digit decimal expansion, so `string(0.125)` is `"0.13"`; a result that rounds to zero loses its
/// sign (`"0.00"` for -0.004); and past 10 integer digits it switches to 10 significant digits in scientific
/// notation (`"1.099511628E12"` for 2^40 + 0.5). From 2^63 up (and for infinities and NaN) that `round`
/// traps, and `string` gives `"ERROR"` (GM8 on a real Intel CPU: `string(sin(1e22))`; under Rosetta, which
/// doesn't trap, it's `"1E22"`). OpenGMK (`value.rs`, `Value::repr`) formats with Rust's `{:.0}`/`{:.2}`
/// instead, which rounds the exact binary value half to even; oracle patch 0014 follows GM8.
pub fn real_to_string(x: f64) -> GmString {
    crate::gm8::real_to_string(x).into()
}

/// Bytes `real()` trims: ASCII whitespace and Windows-1252's no-break space (0xA0). OpenGMK decodes the
/// string with the game's code page and calls `str::trim`; these are the bytes that decode to Unicode
/// whitespace.
fn is_trimmed(b: u8) -> bool {
    matches!(b, b'\t' | b'\n' | 0x0B | 0x0C | b'\r' | b' ' | 0xA0)
}

/// GML `real(s)` for a string: trims whitespace, treats an empty string as 0 and otherwise parses with
/// Rust's `f64` grammar (so `"1e3"`, `".5"` and `"+2"` parse), correctly rounded. Anything else, such as
/// `"12abc"`, is a runtime error, and so is a string that reads as an infinity or a NaN: GM8 errors on `"inf"`
/// and `"nan"`, and on Intel on `"1e400"` too (Rosetta reads that as inf; probes `real-inf`, `real-nan`,
/// `real-overflow`). OpenGMK `gm8emulator/src/gml/kernel.rs` (`real`) takes all three (oracle patch 0014
/// follows GM8 on Intel).
pub fn string_to_real(s: &[u8]) -> Result<f64, ValueError> {
    let start = s.iter().position(|&b| !is_trimmed(b)).unwrap_or(s.len());
    let end = s
        .iter()
        .rposition(|&b| !is_trimmed(b))
        .map_or(start, |i| i + 1);
    let trimmed = &s[start..end];
    if trimmed.is_empty() {
        return Ok(0.0);
    }
    std::str::from_utf8(trimmed)
        .ok()
        .and_then(|t| t.parse().ok())
        .filter(|x: &f64| x.is_finite())
        .ok_or(ValueError::BadRealString)
}

/// GML `string_format(x, total, decimals)` for a real: Delphi's `FloatToStrF(x, ffFixed, 18, decimals)`,
/// rounded as [`real_to_string`] rounds but to `decimals` decimals (at most 18) and 18 significant digits,
/// then right-aligned in at least `total` characters: `string_format(0.125, 10, 2)` is `"      0.13"`,
/// `string_format(1 / 3, 0, 18)` is `"0.333333333333333312"`, `string_format(1e20, 0, 2)` is `"1E20"`, and
/// infinities and NaN are `"INF"`, `"-INF"` and `"NAN"`. A negative `decimals` rounds (half up) to a multiple
/// of `10^-decimals` and prints 18 decimals (`string_format(125, 0, -1)` is `"130.000000000000000000"`). GM8
/// (probes `string-format`, `string-format-neg`, `string-format-big`: the same under Rosetta and on a real
/// Intel CPU, but infinities and NaN, which Intel can't make); OpenGMK (`kernel.rs`, `string_format`) rounds
/// half to even and formats with Rust (oracle patch 0014 follows GM8). A string `x` is returned unchanged by
/// [`crate::Value::string_format`]. Not used by the game.
pub fn string_format(x: f64, total: i32, decimals: i32) -> GmString {
    crate::gm8::string_format(x, total, decimals).into()
}

/// The 0-based byte index for a 1-based GML position. Positions below 1 clamp to the first byte.
#[inline]
fn index_from(pos: i32) -> usize {
    (i64::from(pos) - 1).max(0) as usize
}

/// GML `string_length(s)`: the length in bytes.
#[inline]
pub fn string_length(s: &[u8]) -> f64 {
    s.len() as f64
}

/// GML `ord(s)`: the first byte, or 0 for an empty string. OpenGMK `gm8emulator/src/gml/kernel.rs`
/// (`ord`).
#[inline]
pub fn ord(s: &[u8]) -> f64 {
    s.first().map_or(0.0, |&b| f64::from(b))
}

/// GML `string_char_at(s, pos)`: the byte at 1-based `pos` as a one-byte string, or `""` past the end.
/// Positions below 1 read the first byte, so `string_char_at("abc", 0)` is `"a"`. OpenGMK
/// `gm8emulator/src/gml/kernel.rs` (`string_char_at`).
pub fn string_char_at(s: &[u8], pos: i32) -> GmString {
    s.get(index_from(pos)).map_or_else(GmString::default, |b| {
        GmString::from(std::slice::from_ref(b))
    })
}

/// GML `string_delete(s, pos, count)`: removes up to `count` bytes starting at 1-based `pos`. A `pos`
/// below 1 or a `count` below 1 leaves the string unchanged (no clamping, unlike `string_char_at`).
/// OpenGMK `gm8emulator/src/gml/kernel.rs` (`string_delete`).
pub fn string_delete(s: &GmString, pos: i32, count: i32) -> GmString {
    let bytes = s.as_bytes();
    let start = i64::from(pos) - 1;
    if start < 0 || count <= 0 || start as usize >= bytes.len() {
        return s.clone();
    }
    let start = start as usize;
    let end = start.saturating_add(count as usize).min(bytes.len());
    GmString::concat2(&bytes[..start], &bytes[end..])
}

/// GML `string_insert(substr, s, pos)`: inserts `substr` into `s` before 1-based `pos`, which is clamped
/// to the string, so positions below 1 prepend and positions past the end append. Note the argument order.
/// OpenGMK `gm8emulator/src/gml/kernel.rs` (`string_insert`).
pub fn string_insert(substr: &[u8], s: &[u8], pos: i32) -> GmString {
    let at = index_from(pos).min(s.len());
    GmString::concat3(&s[..at], substr, &s[at..])
}

/// GML `string_upper(s)`: ASCII letters only, as GM 8.0 does. OpenGMK `gm8emulator/src/gml/kernel.rs`
/// (`string_upper`, 8.0 branch).
pub fn string_upper(s: &[u8]) -> GmString {
    s.to_ascii_uppercase().into()
}

/// GML `string_lower(s)`: ASCII letters only.
pub fn string_lower(s: &[u8]) -> GmString {
    s.to_ascii_lowercase().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(x: &str) -> GmString {
        GmString::from(x)
    }

    #[test]
    fn real_to_string_uses_zero_or_two_decimals() {
        assert_eq!(real_to_string(42.0), s("42"));
        assert_eq!(real_to_string(-7.0), s("-7"));
        assert_eq!(real_to_string(0.5), s("0.50"));
        assert_eq!(real_to_string(1.0 / 3.0), s("0.33"));
        assert_eq!(
            real_to_string(2.999),
            s("3.00"),
            "not whole, so two decimals even when they round to .00"
        );
        assert_eq!(real_to_string(1e-10), s("0.00"));
        // A result that rounds to zero loses its sign; -0 is an integer.
        assert_eq!(real_to_string(-0.001), s("0.00"));
        assert_eq!(real_to_string(-0.005), s("-0.01"));
        assert_eq!(real_to_string(-0.0), s("0"));
        assert_eq!(real_to_string(1e15), s("1000000000000000"));
        assert_eq!(
            real_to_string(1.0 + 1e-14),
            s("1.00"),
            "whole means exactly whole"
        );
        // Half away from zero on 18 digits, each scaling rounded to a double: 0.125 is a tie, and
        // 0x1.28f5c28f5c28fp-3, just below 0.145, scales to 145000000000000000 exactly.
        assert_eq!(real_to_string(0.125), s("0.13"));
        assert_eq!(real_to_string(-0.125), s("-0.13"));
        assert_eq!(real_to_string(0.625), s("0.63"));
        assert_eq!(real_to_string(1.005), s("1.00"));
        assert_eq!(
            real_to_string(f64::from_bits(0x3fc2_8f5c_28f5_c28f)),
            s("0.15")
        );
        assert_eq!(
            real_to_string(f64::from_bits(0x3f99_9999_9999_9999)),
            s("0.02")
        );
        assert_eq!(real_to_string(99999999.995), s("100000000.00"));
        // At most 10 significant digits, then scientific notation.
        assert_eq!(real_to_string(123456789.125), s("123456789.10"));
        assert_eq!(real_to_string(1234567890.125), s("1234567890.00"));
        assert_eq!(real_to_string(9999999999.5), s("1E10"));
        assert_eq!(real_to_string(1099511627776.5), s("1.099511628E12"));
        // Whole up to the Int64 range; past it, Round traps and `string` catches it.
        assert_eq!(
            real_to_string(4611686018427387904.0),
            s("4611686018427387904")
        );
        assert_eq!(
            real_to_string(-9223372036854775808.0),
            s("-9223372036854775808")
        );
        for x in [
            9223372036854775808.0,
            -1e25,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
        ] {
            assert_eq!(real_to_string(x), s("ERROR"), "{x}");
        }
        // Subnormals don't trip the exponent estimate.
        assert_eq!(real_to_string(5e-324), s("0.00"));
    }

    #[test]
    fn string_to_real_trims_and_parses() {
        assert_eq!(string_to_real(b"42"), Ok(42.0));
        assert_eq!(string_to_real(b"  -3.5\r\n"), Ok(-3.5));
        assert_eq!(string_to_real(b""), Ok(0.0));
        assert_eq!(string_to_real(b"   "), Ok(0.0));
        assert_eq!(string_to_real(b"1e3"), Ok(1000.0));
        assert_eq!(string_to_real(b".5"), Ok(0.5));
        assert_eq!(string_to_real(b"+2"), Ok(2.0));
        assert_eq!(string_to_real(b"\xA01\xA0"), Ok(1.0));
        assert_eq!(string_to_real(b"12abc"), Err(ValueError::BadRealString));
        assert_eq!(string_to_real(b"inf"), Err(ValueError::BadRealString));
        assert_eq!(string_to_real(b"nan"), Err(ValueError::BadRealString));
        assert_eq!(string_to_real(b"1e400"), Err(ValueError::BadRealString));
        assert_eq!(string_to_real(b"1e-400"), Ok(0.0));
        assert_eq!(string_to_real(b"abc"), Err(ValueError::BadRealString));
        assert_eq!(string_to_real(b"1 2"), Err(ValueError::BadRealString));
        assert_eq!(string_to_real(b"\xE91"), Err(ValueError::BadRealString));
    }

    #[test]
    fn string_format_pads_and_rounds() {
        assert_eq!(string_format(1.23456, 8, 2), s("    1.23"));
        assert_eq!(string_format(1.23456, 0, 0), s("1"));
        assert_eq!(string_format(-2.5, 6, 1), s("  -2.5"));
        assert_eq!(
            string_format(1.0, -5, 30),
            s("1.000000000000000000"),
            "decimals cap at 18"
        );
        assert_eq!(string_format(0.125, 10, 2), s("      0.13"), "half up");
        assert_eq!(string_format(-0.001, 0, 2), s("0.00"));
        assert_eq!(string_format(1.0 / 3.0, 0, 18), s("0.333333333333333312"));
        assert_eq!(string_format(1e20, 0, 2), s("1E20"), "past 18 digits");
        assert_eq!(string_format(f64::NAN, 0, 2), s("NAN"));
        assert_eq!(string_format(f64::NEG_INFINITY, 6, 2), s("  -INF"));
        assert_eq!(
            string_format(1250.0, 0, -2),
            s("1300.000000000000000000"),
            "half up, then 18 decimals"
        );
        assert_eq!(
            string_format(-0.004, 0, i32::MIN),
            s("0.000000000000000000"),
            "no overflow panic"
        );
    }

    #[test]
    fn positions_are_one_based_bytes() {
        assert_eq!(string_length(b"hello"), 5.0);
        assert_eq!(string_length("é".as_bytes()), 2.0, "bytes, not characters");
        assert_eq!(ord(b"A"), 65.0);
        assert_eq!(ord(b""), 0.0);
        assert_eq!(ord(b"\xFF"), 255.0);
        assert_eq!(string_char_at(b"abc", 1), s("a"));
        assert_eq!(string_char_at(b"abc", 3), s("c"));
        assert_eq!(string_char_at(b"abc", 4), s(""));
        assert_eq!(
            string_char_at(b"abc", 0),
            s("a"),
            "positions below 1 clamp to the first byte"
        );
        assert_eq!(string_char_at(b"abc", -5), s("a"));
        assert_eq!(string_char_at(b"", 1), s(""));
    }

    #[test]
    fn string_delete_edges() {
        let abcde = s("abcde");
        assert_eq!(string_delete(&abcde, 2, 2), s("ade"));
        assert_eq!(string_delete(&abcde, 1, 1), s("bcde"));
        assert_eq!(string_delete(&abcde, 4, 100), s("abc"));
        assert_eq!(string_delete(&abcde, 6, 1), abcde);
        assert_eq!(
            string_delete(&abcde, 0, 2),
            abcde,
            "pos below 1 deletes nothing"
        );
        assert_eq!(string_delete(&abcde, 2, 0), abcde);
        assert_eq!(string_delete(&abcde, 2, -1), abcde);
        assert_eq!(string_delete(&abcde, i32::MIN, 1), abcde);
        assert_eq!(string_delete(&abcde, 2, i32::MAX), s("a"));
    }

    #[test]
    fn string_insert_clamps() {
        assert_eq!(string_insert(b"XY", b"abc", 1), s("XYabc"));
        assert_eq!(string_insert(b"XY", b"abc", 2), s("aXYbc"));
        assert_eq!(string_insert(b"XY", b"abc", 4), s("abcXY"));
        assert_eq!(string_insert(b"XY", b"abc", 99), s("abcXY"));
        assert_eq!(string_insert(b"XY", b"abc", 0), s("XYabc"));
        assert_eq!(string_insert(b"XY", b"abc", i32::MIN), s("XYabc"));
    }

    #[test]
    fn case_mapping_is_ascii_only() {
        assert_eq!(string_upper(b"abc xyz!"), s("ABC XYZ!"));
        assert_eq!(string_lower(b"ABC \xC9"), GmString::from(&b"abc \xC9"[..]));
    }

    #[test]
    fn empty_strings_are_normalized() {
        let empty = GmString::default();
        assert_eq!(empty, GmString::from(""));
        assert_eq!(empty, GmString::from(Vec::new()));
        assert_eq!(empty, string_char_at(b"abc", 9));
        assert!(string_delete(&s("ab"), 1, 2).is_empty());
        assert!(empty < s("a"));
        assert_eq!(empty.as_bytes(), b"");
    }

    #[test]
    fn display_is_latin1() {
        assert_eq!(GmString::from(&b"caf\xE9"[..]).to_string(), "café");
    }
}
