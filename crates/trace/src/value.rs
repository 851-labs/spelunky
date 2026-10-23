//! GameMaker values and their lossless JSON encoding, as the oracle (patched OpenGMK) writes them.
//!
//! - A real is a JSON number, written as the shortest decimal that parses back to the same `f64` (Rust's
//!   `{:?}`, `ryu`, Python's `repr`). JSON has no NaN or infinity: in a field that is always a real (`x`,
//!   `depth`...) they are the strings `"NaN"`, `"inf"` and `"-inf"`, which is what the oracle writes. In a
//!   variable, where a string is a GML string, they are `{"real":"nan"}`, `{"real":"inf"}` and
//!   `{"real":"-inf"}`. Readers accept both forms wherever a real is expected.
//! - A string is a JSON string.
//! - An array is a JSON object from flattened index to element, `{"0":1.0,"32001":"a"}`, holding only the
//!   elements that were set. GM 8 arrays are 2-D and `a[i,j]` is stored at `i * 32000 + j` (so `a[j]` is
//!   `a[0,j]` at `j`), as in OpenGMK. GML reads a plain variable `a` as `a[0]`, so comparisons treat a scalar
//!   and an array holding only index 0 as the same value.

use std::collections::BTreeMap;
use std::fmt;

use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{SerializeMap, Serializer};
use serde::{Deserialize, Serialize};

/// The flattening factor for 2-D array indices.
pub const ARRAY_STRIDE: u32 = 32000;

/// A GameMaker real (`f64`) that serializes losslessly.
///
/// Equality is bitwise, except that every NaN equals every other NaN: traces compare exactly by default, so
/// `0.0` and `-0.0` are different values here.
#[derive(Clone, Copy, Debug, Default)]
pub struct Real(pub f64);

impl Real {
    /// Whether two reals are equal, or (with a positive `tolerance`) at most `tolerance` apart.
    pub fn matches(self, other: Real, tolerance: f64) -> bool {
        self == other || (tolerance > 0.0 && (self.0 - other.0).abs() <= tolerance)
    }
}

impl PartialEq for Real {
    fn eq(&self, other: &Self) -> bool {
        (self.0.is_nan() && other.0.is_nan()) || self.0.to_bits() == other.0.to_bits()
    }
}

impl From<f64> for Real {
    fn from(v: f64) -> Self {
        Real(v)
    }
}

impl fmt::Display for Real {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format_real(self.0))
    }
}

/// The shortest decimal that parses back to exactly `v` ("1" rather than "1.0"; NaN is "nan").
pub fn format_real(v: f64) -> String {
    if v.is_nan() {
        return "nan".into();
    }
    // `{:?}` is Rust's shortest round-trip form and switches to exponent notation for very large or small
    // values, where `{}` would print hundreds of digits.
    let s = format!("{v:?}");
    match s.strip_suffix(".0") {
        Some(int) => int.to_string(),
        None => s,
    }
}

/// The oracle's spelling (Rust's `Display` for `f64`) of a non-finite real.
fn non_finite_name(v: f64) -> Option<&'static str> {
    if v.is_nan() {
        Some("NaN")
    } else if v == f64::INFINITY {
        Some("inf")
    } else if v == f64::NEG_INFINITY {
        Some("-inf")
    } else {
        None
    }
}

fn parse_non_finite(s: &str) -> Option<f64> {
    match s {
        "nan" | "NaN" => Some(f64::NAN),
        "inf" => Some(f64::INFINITY),
        "-inf" => Some(f64::NEG_INFINITY),
        _ => None,
    }
}

fn serialize_tagged_real<S: Serializer>(v: f64, s: S) -> Result<S::Ok, S::Error> {
    match non_finite_name(v) {
        None => s.serialize_f64(v),
        Some(name) => {
            let mut map = s.serialize_map(Some(1))?;
            map.serialize_entry("real", &name.to_ascii_lowercase())?;
            map.end()
        }
    }
}

/// In an always-real field, non-finite values are bare strings, as the oracle writes them.
impl Serialize for Real {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match non_finite_name(self.0) {
            None => s.serialize_f64(self.0),
            Some(name) => s.serialize_str(name),
        }
    }
}

impl<'de> Deserialize<'de> for Real {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        match d.deserialize_any(ValueVisitor { allow_array: false })? {
            Value::Real(r) => Ok(r),
            // Unambiguous here: this field can only hold a real.
            Value::Str(s) => parse_non_finite(&s).map(Real).ok_or_else(|| {
                de::Error::custom(format!("expected a real, found the string {s:?}"))
            }),
            Value::Array(_) => Err(de::Error::custom("expected a real, found an array")),
        }
    }
}

/// A scalar GameMaker value: what an array element can hold.
#[derive(Clone, Debug, PartialEq)]
pub enum Scalar {
    Real(Real),
    Str(String),
}

impl Scalar {
    /// Equality with an absolute tolerance for reals.
    pub fn matches(&self, other: &Scalar, tolerance: f64) -> bool {
        match (self, other) {
            (Scalar::Real(a), Scalar::Real(b)) => a.matches(*b, tolerance),
            _ => self == other,
        }
    }
}

/// Any value a GameMaker variable can hold.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Real(Real),
    Str(String),
    /// Elements by flattened index (`i * 32000 + j`).
    Array(BTreeMap<u32, Scalar>),
}

impl Value {
    /// The value as GML indexes it: a scalar is element 0.
    pub fn elements(&self) -> BTreeMap<u32, Scalar> {
        match self {
            Value::Real(r) => BTreeMap::from([(0, Scalar::Real(*r))]),
            Value::Str(s) => BTreeMap::from([(0, Scalar::Str(s.clone()))]),
            Value::Array(a) => a.clone(),
        }
    }
}

/// `[j]` for a 1-D index, `[i,j]` for a 2-D one.
pub fn index_suffix(flat: u32) -> String {
    let (i, j) = (flat / ARRAY_STRIDE, flat % ARRAY_STRIDE);
    if i == 0 {
        format!("[{j}]")
    } else {
        format!("[{i},{j}]")
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Real(Real(v))
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::Str(v.to_string())
    }
}

impl From<Scalar> for Value {
    fn from(s: Scalar) -> Self {
        match s {
            Scalar::Real(r) => Value::Real(r),
            Scalar::Str(s) => Value::Str(s),
        }
    }
}

impl fmt::Display for Scalar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Scalar::Real(r) => r.fmt(f),
            Scalar::Str(s) => write!(f, "{s:?}"),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Real(r) => r.fmt(f),
            Value::Str(s) => write!(f, "{s:?}"),
            Value::Array(a) => {
                f.write_str("{")?;
                for (n, (k, v)) in a.iter().enumerate() {
                    if n > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{}={v}", index_suffix(*k))?;
                }
                f.write_str("}")
            }
        }
    }
}

impl Serialize for Scalar {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Scalar::Real(r) => serialize_tagged_real(r.0, s),
            Scalar::Str(v) => s.serialize_str(v),
        }
    }
}

impl<'de> Deserialize<'de> for Scalar {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        match d.deserialize_any(ValueVisitor { allow_array: false })? {
            Value::Real(r) => Ok(Scalar::Real(r)),
            Value::Str(s) => Ok(Scalar::Str(s)),
            Value::Array(_) => Err(de::Error::custom("arrays cannot be nested")),
        }
    }
}

impl Serialize for Value {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Value::Real(r) => serialize_tagged_real(r.0, s),
            Value::Str(v) => s.serialize_str(v),
            Value::Array(a) => {
                let mut map = s.serialize_map(Some(a.len()))?;
                for (k, v) in a {
                    map.serialize_entry(&k.to_string(), v)?;
                }
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(ValueVisitor { allow_array: true })
    }
}

struct ValueVisitor {
    allow_array: bool,
}

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a number, a string, {\"real\": \"nan\"|\"inf\"|\"-inf\"} or an array {\"index\": value, ...}")
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Value, E> {
        Ok(Value::Real(Real(v)))
    }

    // Integers are exact in f64 up to 2^53; beyond that `as` rounds to nearest, as a decimal parse would.
    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Value, E> {
        Ok(Value::Real(Real(v as f64)))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Value, E> {
        Ok(Value::Real(Real(v as f64)))
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Value, E> {
        Ok(Value::Str(v.to_string()))
    }

    fn visit_string<E: de::Error>(self, v: String) -> Result<Value, E> {
        Ok(Value::Str(v))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        let mut array = BTreeMap::new();
        while let Some(key) = map.next_key::<String>()? {
            if key == "real" && array.is_empty() {
                let name: String = map.next_value()?;
                let v = parse_non_finite(&name).ok_or_else(|| {
                    de::Error::custom(format!(
                        "{{\"real\": {name:?}}}: expected \"nan\", \"inf\" or \"-inf\" (finite reals are plain numbers)"
                    ))
                })?;
                if map.next_key::<String>()?.is_some() {
                    return Err(de::Error::custom("{\"real\": ...} has exactly one key"));
                }
                return Ok(Value::Real(Real(v)));
            }
            if !self.allow_array {
                return Err(de::Error::custom("an array is not allowed here"));
            }
            // Canonical decimal only, so each element has exactly one spelling.
            let index = key
                .parse::<u32>()
                .ok()
                .filter(|i| i.to_string() == key)
                .ok_or_else(|| {
                    de::Error::custom(format!("array index {key:?} is not a non-negative integer"))
                })?;
            let v: Scalar = map.next_value()?;
            if array.insert(index, v).is_some() {
                return Err(de::Error::custom(format!(
                    "array index {index} appears twice"
                )));
            }
        }
        if !self.allow_array {
            return Err(de::Error::custom("an array is not allowed here"));
        }
        Ok(Value::Array(array))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, _seq: A) -> Result<Value, A::Error> {
        Err(de::Error::custom(
            "JSON lists are not values; arrays are objects {\"index\": value, ...}",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(v: &Value) -> Value {
        let json = serde_json::to_string(v).unwrap();
        serde_json::from_str(&json).unwrap()
    }

    #[test]
    fn reals_round_trip_bit_exactly() {
        let samples = [
            0.0,
            -0.0,
            0.1,
            0.6000000000000001,
            1.0 / 3.0,
            -2.5e-310, // subnormal
            f64::MIN_POSITIVE,
            f64::MAX,
            f64::MIN,
            f64::EPSILON,
            123_456_789.123_456_79,
            9007199254740993.0,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ];
        for v in samples {
            let Value::Real(r) = round_trip(&Value::from(v)) else {
                panic!("not a real")
            };
            assert_eq!(r.0.to_bits(), v.to_bits(), "{v:?}");
            let back: Real =
                serde_json::from_str(&serde_json::to_string(&Real(v)).unwrap()).unwrap();
            assert_eq!(back.0.to_bits(), v.to_bits(), "{v:?}");
        }
        let Value::Real(r) = round_trip(&Value::from(f64::NAN)) else {
            panic!()
        };
        assert!(r.0.is_nan());
    }

    #[test]
    fn reals_round_trip_through_pseudo_random_bit_patterns() {
        // An xorshift sweep over raw bit patterns hits many awkward decimal conversions.
        let mut s: u64 = 0x9E37_79B9_7F4A_7C15;
        for _ in 0..20_000 {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            let v = f64::from_bits(s);
            if v.is_nan() {
                continue;
            }
            let json = serde_json::to_string(&Real(v)).unwrap();
            let back: Real = serde_json::from_str(&json).unwrap();
            assert_eq!(back.0.to_bits(), v.to_bits(), "{json}");
            // The oracle's own spelling ({:?}) must parse exactly too.
            let back: Real = serde_json::from_str(&format!("{v:?}")).unwrap();
            assert_eq!(back.0.to_bits(), v.to_bits());
            assert_eq!(
                format_real(v).parse::<f64>().unwrap().to_bits(),
                v.to_bits()
            );
        }
    }

    #[test]
    fn encodings() {
        assert_eq!(serde_json::to_string(&Value::from(1.5)).unwrap(), "1.5");
        assert_eq!(
            serde_json::to_string(&Value::from(f64::NAN)).unwrap(),
            r#"{"real":"nan"}"#
        );
        assert_eq!(serde_json::to_string(&Real(f64::NAN)).unwrap(), r#""NaN""#);
        assert_eq!(
            serde_json::to_string(&Real(f64::NEG_INFINITY)).unwrap(),
            r#""-inf""#
        );
        assert_eq!(
            serde_json::to_string(&Value::from("hi")).unwrap(),
            r#""hi""#
        );
        let a = BTreeMap::from([
            (32001, Scalar::Str("b".into())),
            (0, Scalar::Real(Real(2.0))),
        ]);
        assert_eq!(
            serde_json::to_string(&Value::Array(a.clone())).unwrap(),
            r#"{"0":2.0,"32001":"b"}"#
        );
        assert_eq!(
            round_trip(&Value::Array(a)).to_string(),
            r#"{[0]=2, [1,1]="b"}"#
        );
        assert_eq!(
            round_trip(&Value::Array(BTreeMap::new())),
            Value::Array(BTreeMap::new())
        );
        // Integers parse as reals; a string in a variable stays a string, even "NaN".
        assert_eq!(
            serde_json::from_str::<Value>("-7").unwrap(),
            Value::from(-7.0)
        );
        assert_eq!(
            serde_json::from_str::<Value>(r#""NaN""#).unwrap(),
            Value::from("NaN")
        );
        assert!(serde_json::from_str::<Real>(r#""NaN""#).unwrap().0.is_nan());
        assert!(
            serde_json::from_str::<Real>(r#"{"real":"nan"}"#)
                .unwrap()
                .0
                .is_nan()
        );
        assert_eq!(
            serde_json::from_str::<Real>(r#""inf""#).unwrap().0,
            f64::INFINITY
        );
        // The oracle's arrays.
        let v: Value =
            serde_json::from_str(r#"{"0":0.0,"1":2.0,"32000":{"real":"inf"},"96004":"x"}"#)
                .unwrap();
        assert_eq!(v.to_string(), r#"{[0]=0, [1]=2, [1,0]=inf, [3,4]="x"}"#);
    }

    #[test]
    fn malformed_values_are_errors() {
        for bad in [
            "null",
            "true",
            "[1,2]",
            r#"{"real":"Infinity"}"#,
            r#"{"real":1}"#,
            r#"{"real":"nan","x":1}"#,
            r#"{"0":1,"real":"nan"}"#,
            r#"{"foo":1}"#,
            r#"{"-1":1}"#,
            r#"{"01":1}"#,
            r#"{"4294967296":1}"#,
            r#"{"0":{"0":1}}"#,
            r#"{"0":1,"0":2}"#,
            r#"{"0":null}"#,
        ] {
            assert!(serde_json::from_str::<Value>(bad).is_err(), "{bad}");
        }
        for bad in [r#""1""#, r#""nan ""#, r#"{"0":1}"#, "{}", "null"] {
            assert!(serde_json::from_str::<Real>(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn equality_and_tolerance() {
        assert_ne!(Real(0.0), Real(-0.0));
        assert_eq!(Real(f64::NAN), Real(-f64::NAN));
        assert!(Real(1.0).matches(Real(1.0 + 1e-12), 1e-9));
        assert!(!Real(1.0).matches(Real(1.0 + 1e-12), 0.0));
        assert!(!Real(f64::NAN).matches(Real(1.0), 1e9));
        assert!(Scalar::Real(Real(1.0)).matches(&Scalar::Real(Real(1.05)), 0.1));
        assert!(!Scalar::Real(Real(1.0)).matches(&Scalar::Str("1".into()), 0.1));
        assert_eq!(
            Value::from(5.0).elements(),
            BTreeMap::from([(0, Scalar::Real(Real(5.0)))])
        );
    }

    #[test]
    fn format_real_is_short() {
        assert_eq!(format_real(1.0), "1");
        assert_eq!(format_real(-0.0), "-0");
        assert_eq!(format_real(0.1), "0.1");
        assert_eq!(format_real(1e300), "1e300");
        assert_eq!(format_real(f64::NEG_INFINITY), "-inf");
        assert_eq!(index_suffix(5), "[5]");
        assert_eq!(index_suffix(96004), "[3,4]");
    }
}
