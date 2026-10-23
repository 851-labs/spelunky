//! GML values: a real (`f64`) or a string (851-3125).
//!
//! Operators follow GM8 as the reference oracle (OpenGMK) runs it; each method cites where its behavior
//! is defined. "OpenGMK `value.rs`" means `gm8emulator/src/gml/value.rs`. Real arithmetic itself lives in
//! [`crate::math`] so game code can use it on bare `f64`s.
//!
//! Operators on mismatched types are runtime errors in GM8 ("wrong type of arguments"), returned here as
//! [`ValueError::WrongTypes`]. Built-in *functions* never fail on types: they coerce their arguments with
//! [`Value::to_real`], [`Value::to_int`] and [`Value::to_str`].

use crate::math;
use crate::string::{self, GmString};
use std::fmt;

/// A GML runtime error raised by an operator or a value built-in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueError {
    /// An operator was applied to operand types it doesn't accept, e.g. `"a" - 1` or `1 < "a"`.
    WrongTypes(Op),
    /// `real(s)` on a string that isn't a number.
    BadRealString,
    /// A math function's result isn't finite, e.g. `sqrt(-1)`. Holds the function name.
    Domain(&'static str),
}

impl fmt::Display for ValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongTypes(op) => write!(f, "wrong type of arguments to {op}"),
            Self::BadRealString => f.write_str("real(): the string is not a number"),
            Self::Domain(name) => write!(f, "{name}(): argument out of range"),
        }
    }
}

impl std::error::Error for ValueError {}

/// GML operators, for error reporting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    IntDiv,
    Mod,
    Neg,
    Not,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    BitAnd,
    BitOr,
    BitXor,
    BitNot,
    Shl,
    Shr,
}

impl fmt::Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::IntDiv => "div",
            Self::Mod => "mod",
            Self::Neg => "unary -",
            Self::Not => "!",
            Self::Eq => "==",
            Self::Ne => "!=",
            Self::Lt => "<",
            Self::Le => "<=",
            Self::Gt => ">",
            Self::Ge => ">=",
            Self::BitAnd => "&",
            Self::BitOr => "|",
            Self::BitXor => "^",
            Self::BitNot => "~",
            Self::Shl => "<<",
            Self::Shr => ">>",
        })
    }
}

/// A GML value. Reals are stored inline; strings share their bytes, so cloning never allocates.
///
/// `PartialEq` is exact Rust equality (for tests and caches). GML's `==` is [`Value::gml_eq`].
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Real(f64),
    Str(GmString),
}

impl Default for Value {
    /// Uninitialized GML values read as the real 0.
    fn default() -> Self {
        Self::Real(0.0)
    }
}

impl From<f64> for Value {
    #[inline]
    fn from(x: f64) -> Self {
        Self::Real(x)
    }
}

impl From<i32> for Value {
    #[inline]
    fn from(x: i32) -> Self {
        Self::Real(f64::from(x))
    }
}

impl From<bool> for Value {
    #[inline]
    fn from(b: bool) -> Self {
        Self::Real(math::from_bool(b))
    }
}

impl From<GmString> for Value {
    #[inline]
    fn from(s: GmString) -> Self {
        Self::Str(s)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Self::Str(s.into())
    }
}

/// Applies a real-only binary operator, or fails with `WrongTypes(op)`.
#[inline]
fn real_op(a: &Value, b: &Value, op: Op, f: fn(f64, f64) -> f64) -> Result<Value, ValueError> {
    match (a, b) {
        (Value::Real(a), Value::Real(b)) => Ok(Value::Real(f(*a, *b))),
        _ => Err(ValueError::WrongTypes(op)),
    }
}

/// Applies a comparison: reals with the GM8 tolerance, strings bytewise; mixed types are an error
/// (OpenGMK `value.rs`, `gml_cmp_impl!`).
#[inline]
fn compare(
    a: &Value,
    b: &Value,
    op: Op,
    real: fn(f64, f64) -> bool,
    bytes: fn(&[u8], &[u8]) -> bool,
) -> Result<bool, ValueError> {
    match (a, b) {
        (Value::Real(a), Value::Real(b)) => Ok(real(*a, *b)),
        (Value::Str(a), Value::Str(b)) => Ok(bytes(a.as_bytes(), b.as_bytes())),
        _ => Err(ValueError::WrongTypes(op)),
    }
}

impl Value {
    /// GML `is_real(x)`.
    #[inline]
    pub fn is_real(&self) -> bool {
        matches!(self, Self::Real(_))
    }

    /// GML `is_string(x)`.
    #[inline]
    pub fn is_string(&self) -> bool {
        matches!(self, Self::Str(_))
    }

    /// The real, if this is one.
    #[inline]
    pub fn as_real(&self) -> Option<f64> {
        match self {
            Self::Real(x) => Some(*x),
            Self::Str(_) => None,
        }
    }

    /// GML truthiness for `if`, `while`, `!` and the logical operators: a real `>= 0.5`
    /// ([`math::is_truthy`]). Strings are always false, without an error (OpenGMK `value.rs`,
    /// `Value::is_truthy`).
    #[inline]
    pub fn is_truthy(&self) -> bool {
        match self {
            Self::Real(x) => math::is_truthy(*x),
            Self::Str(_) => false,
        }
    }

    /// A built-in function's real argument: strings read as 0 rather than raising an error. OpenGMK
    /// `value.rs` (`From<Value> for Real`), used by every kernel function that takes a real.
    #[inline]
    pub fn to_real(&self) -> f64 {
        match self {
            Self::Real(x) => *x,
            Self::Str(_) => 0.0,
        }
    }

    /// A built-in function's integer argument: rounded half to even, truncated to 32 bits
    /// ([`math::to_int`]); strings read as 0. OpenGMK `value.rs` (`From<Value> for i32`).
    #[inline]
    pub fn to_int(&self) -> i32 {
        match self {
            Self::Real(x) => math::to_int(*x),
            Self::Str(_) => 0,
        }
    }

    /// A built-in function's string argument: reals read as the empty string. OpenGMK `value.rs`
    /// (`From<&Value> for &[u8]`).
    #[inline]
    pub fn to_str(&self) -> &[u8] {
        match self {
            Self::Real(_) => b"",
            Self::Str(s) => s.as_bytes(),
        }
    }

    /// GML `string(x)`: strings pass through; reals format as in [`string::real_to_string`].
    pub fn to_gml_string(&self) -> GmString {
        match self {
            Self::Real(x) => string::real_to_string(*x),
            Self::Str(s) => s.clone(),
        }
    }

    /// GML `real(x)`: reals pass through; strings parse as in [`string::string_to_real`].
    pub fn to_gml_real(&self) -> Result<f64, ValueError> {
        match self {
            Self::Real(x) => Ok(*x),
            Self::Str(s) => string::string_to_real(s.as_bytes()),
        }
    }

    /// GML `string_format(x, total, decimals)`: strings pass through unchanged; reals format as in
    /// [`string::string_format`]. OpenGMK `gm8emulator/src/gml/kernel.rs` (`string_format`).
    pub fn string_format(&self, total: i32, decimals: i32) -> GmString {
        match self {
            Self::Real(x) => string::string_format(*x, total, decimals),
            Self::Str(s) => s.clone(),
        }
    }

    /// GML `a + b`: real addition or string concatenation.
    ///
    /// The interpreter must reproduce one quirk on top of this: `field += x` on a user variable ignores a
    /// type error and leaves the field unchanged (built-in variables still raise it). OpenGMK
    /// `gm8emulator/src/gml/compiler.rs` (`type_unsafe`) and `runtime.rs` (`Node::Binary`).
    pub fn add(&self, rhs: &Value) -> Result<Value, ValueError> {
        match (self, rhs) {
            (Self::Real(a), Self::Real(b)) => Ok(Self::Real(a + b)),
            (Self::Str(a), Self::Str(b)) => {
                Ok(Self::Str([a.as_bytes(), b.as_bytes()].concat().into()))
            }
            _ => Err(ValueError::WrongTypes(Op::Add)),
        }
    }

    /// GML `a - b`.
    #[inline]
    pub fn sub(&self, rhs: &Value) -> Result<Value, ValueError> {
        real_op(self, rhs, Op::Sub, |a, b| a - b)
    }

    /// GML `a * b`. Besides reals, `real * string` repeats the string `round(real)` times (none if that is
    /// below 1), while `string * real` is an error. OpenGMK `value.rs` (`Value::mul`).
    pub fn mul(&self, rhs: &Value) -> Result<Value, ValueError> {
        match (self, rhs) {
            (Self::Real(a), Self::Real(b)) => Ok(Self::Real(a * b)),
            (Self::Real(n), Self::Str(s)) => {
                let n = math::to_int(*n);
                Ok(Self::Str(if n > 0 {
                    s.as_bytes().repeat(n as usize).into()
                } else {
                    GmString::default()
                }))
            }
            _ => Err(ValueError::WrongTypes(Op::Mul)),
        }
    }

    /// GML `a / b`. Division by zero is not an error: it gives ±inf or NaN, as in OpenGMK `value.rs`
    /// (`Value::div`).
    #[inline]
    pub fn div(&self, rhs: &Value) -> Result<Value, ValueError> {
        real_op(self, rhs, Op::Div, |a, b| a / b)
    }

    /// GML `a div b` ([`math::int_div`]).
    #[inline]
    pub fn int_div(&self, rhs: &Value) -> Result<Value, ValueError> {
        real_op(self, rhs, Op::IntDiv, math::int_div)
    }

    /// GML `a mod b` ([`math::modulo`]).
    #[inline]
    pub fn modulo(&self, rhs: &Value) -> Result<Value, ValueError> {
        real_op(self, rhs, Op::Mod, math::modulo)
    }

    /// GML unary `-a`. An error on strings (OpenGMK `value.rs`, `Value::neg`).
    #[inline]
    pub fn neg(&self) -> Result<Value, ValueError> {
        match self {
            Self::Real(x) => Ok(Self::Real(-x)),
            Self::Str(_) => Err(ValueError::WrongTypes(Op::Neg)),
        }
    }

    /// GML `!a` / `not a`. An error on strings, even though strings are simply false in `&&`
    /// (OpenGMK `value.rs`, `Value::not`).
    #[inline]
    pub fn not(&self) -> Result<Value, ValueError> {
        match self {
            Self::Real(x) => Ok(Self::from(!math::is_truthy(*x))),
            Self::Str(_) => Err(ValueError::WrongTypes(Op::Not)),
        }
    }

    /// GML `a && b`. Never an error (strings are false). GM8 doesn't short-circuit: the interpreter must
    /// evaluate both operands, side effects included, before calling this (OpenGMK `runtime.rs`,
    /// `Node::Binary` evaluates both sides for every operator).
    #[inline]
    pub fn and(&self, rhs: &Value) -> Value {
        Self::from(self.is_truthy() && rhs.is_truthy())
    }

    /// GML `a || b`. Never an error; no short-circuit (see [`Value::and`]).
    #[inline]
    pub fn or(&self, rhs: &Value) -> Value {
        Self::from(self.is_truthy() || rhs.is_truthy())
    }

    /// GML `a ^^ b`. Never an error.
    #[inline]
    pub fn xor(&self, rhs: &Value) -> Value {
        Self::from(self.is_truthy() != rhs.is_truthy())
    }

    /// GML `a & b` ([`math::bit_and`]).
    #[inline]
    pub fn bit_and(&self, rhs: &Value) -> Result<Value, ValueError> {
        real_op(self, rhs, Op::BitAnd, math::bit_and)
    }

    /// GML `a | b` ([`math::bit_or`]).
    #[inline]
    pub fn bit_or(&self, rhs: &Value) -> Result<Value, ValueError> {
        real_op(self, rhs, Op::BitOr, math::bit_or)
    }

    /// GML `a ^ b` ([`math::bit_xor`]).
    #[inline]
    pub fn bit_xor(&self, rhs: &Value) -> Result<Value, ValueError> {
        real_op(self, rhs, Op::BitXor, math::bit_xor)
    }

    /// GML `~a` ([`math::bit_not`]).
    #[inline]
    pub fn bit_not(&self) -> Result<Value, ValueError> {
        match self {
            Self::Real(x) => Ok(Self::Real(math::bit_not(*x))),
            Self::Str(_) => Err(ValueError::WrongTypes(Op::BitNot)),
        }
    }

    /// GML `a << b` ([`math::shl`]).
    #[inline]
    pub fn shl(&self, rhs: &Value) -> Result<Value, ValueError> {
        real_op(self, rhs, Op::Shl, math::shl)
    }

    /// GML `a >> b` ([`math::shr`]).
    #[inline]
    pub fn shr(&self, rhs: &Value) -> Result<Value, ValueError> {
        real_op(self, rhs, Op::Shr, math::shr)
    }

    /// GML `a == b`: reals within 1e-13 ([`math::eq`]), strings bytewise, mixed types an error.
    #[inline]
    pub fn gml_eq(&self, rhs: &Value) -> Result<bool, ValueError> {
        compare(self, rhs, Op::Eq, math::eq, |a, b| a == b)
    }

    /// GML `a != b`.
    #[inline]
    pub fn gml_ne(&self, rhs: &Value) -> Result<bool, ValueError> {
        compare(self, rhs, Op::Ne, math::ne, |a, b| a != b)
    }

    /// GML `a < b`. Strings compare bytewise (ordinal), so `"B" < "a"`.
    #[inline]
    pub fn gml_lt(&self, rhs: &Value) -> Result<bool, ValueError> {
        compare(self, rhs, Op::Lt, math::lt, |a, b| a < b)
    }

    /// GML `a <= b`.
    #[inline]
    pub fn gml_le(&self, rhs: &Value) -> Result<bool, ValueError> {
        compare(self, rhs, Op::Le, math::le, |a, b| a <= b)
    }

    /// GML `a > b`.
    #[inline]
    pub fn gml_gt(&self, rhs: &Value) -> Result<bool, ValueError> {
        compare(self, rhs, Op::Gt, math::gt, |a, b| a > b)
    }

    /// GML `a >= b`.
    #[inline]
    pub fn gml_ge(&self, rhs: &Value) -> Result<bool, ValueError> {
        compare(self, rhs, Op::Ge, math::ge, |a, b| a >= b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(x: f64) -> Value {
        Value::Real(x)
    }

    fn s(x: &str) -> Value {
        Value::from(x)
    }

    #[test]
    fn reals_stay_small() {
        // A real must never allocate; the enum is a tag plus a fat `Arc` pointer.
        assert!(std::mem::size_of::<Value>() <= 24);
    }

    #[test]
    fn arithmetic_and_concatenation() {
        assert_eq!(r(1.5).add(&r(2.0)), Ok(r(3.5)));
        assert_eq!(s("ab").add(&s("cd")), Ok(s("abcd")));
        assert_eq!(r(5.0).sub(&r(7.0)), Ok(r(-2.0)));
        assert_eq!(r(6.0).div(&r(4.0)), Ok(r(1.5)));
        assert_eq!(
            r(1.0).div(&r(0.0)),
            Ok(r(f64::INFINITY)),
            "no division-by-zero error"
        );
        assert_eq!(r(-7.0).int_div(&r(2.0)), Ok(r(-4.0)));
        assert_eq!(r(-7.0).modulo(&r(3.0)), Ok(r(-1.0)));
    }

    #[test]
    fn mismatched_types_are_errors() {
        assert_eq!(r(1.0).add(&s("a")), Err(ValueError::WrongTypes(Op::Add)));
        assert_eq!(s("a").add(&r(1.0)), Err(ValueError::WrongTypes(Op::Add)));
        assert_eq!(s("a").sub(&s("b")), Err(ValueError::WrongTypes(Op::Sub)));
        assert_eq!(s("a").div(&s("b")), Err(ValueError::WrongTypes(Op::Div)));
        assert_eq!(s("a").modulo(&r(2.0)), Err(ValueError::WrongTypes(Op::Mod)));
        assert_eq!(s("a").neg(), Err(ValueError::WrongTypes(Op::Neg)));
        assert_eq!(s("a").not(), Err(ValueError::WrongTypes(Op::Not)));
        assert_eq!(
            s("a").bit_or(&r(1.0)),
            Err(ValueError::WrongTypes(Op::BitOr))
        );
        assert_eq!(r(1.0).gml_eq(&s("1")), Err(ValueError::WrongTypes(Op::Eq)));
        assert_eq!(s("1").gml_lt(&r(1.0)), Err(ValueError::WrongTypes(Op::Lt)));
        assert_eq!(
            ValueError::WrongTypes(Op::Add).to_string(),
            "wrong type of arguments to +"
        );
    }

    #[test]
    fn real_times_string_repeats() {
        assert_eq!(r(3.0).mul(&s("ab")), Ok(s("ababab")));
        assert_eq!(
            r(2.5).mul(&s("x")),
            Ok(s("xx")),
            "count rounds half to even"
        );
        assert_eq!(r(0.0).mul(&s("x")), Ok(s("")));
        assert_eq!(r(-2.0).mul(&s("x")), Ok(s("")));
        assert_eq!(
            s("x").mul(&r(3.0)),
            Err(ValueError::WrongTypes(Op::Mul)),
            "only real * string"
        );
        assert_eq!(s("x").mul(&s("y")), Err(ValueError::WrongTypes(Op::Mul)));
    }

    #[test]
    fn truthiness_and_logic() {
        assert!(r(0.5).is_truthy());
        assert!(!r(0.4).is_truthy());
        assert!(!s("1").is_truthy(), "strings are false");
        assert_eq!(r(0.5).not(), Ok(r(0.0)));
        assert_eq!(r(0.2).not(), Ok(r(1.0)));
        assert_eq!(r(1.0).and(&r(0.6)), r(1.0));
        assert_eq!(r(1.0).and(&s("x")), r(0.0), "no error, strings are false");
        assert_eq!(r(0.0).or(&r(0.5)), r(1.0));
        assert_eq!(r(1.0).xor(&r(1.0)), r(0.0));
        assert_eq!(r(1.0).xor(&r(0.0)), r(1.0));
    }

    #[test]
    fn comparisons() {
        assert_eq!(r(0.1 + 0.2).gml_eq(&r(0.3)), Ok(true));
        assert_eq!(r(0.1 + 0.2).gml_ne(&r(0.3)), Ok(false));
        assert_eq!(r(1.0).gml_lt(&r(2.0)), Ok(true));
        assert_eq!(r(2.0).gml_le(&r(2.0)), Ok(true));
        assert_eq!(r(2.0).gml_gt(&r(2.0)), Ok(false));
        assert_eq!(r(2.0).gml_ge(&r(2.0)), Ok(true));
        assert_eq!(s("abc").gml_eq(&s("abc")), Ok(true));
        assert_eq!(s("abc").gml_eq(&s("ABC")), Ok(false), "case-sensitive");
        assert_eq!(s("B").gml_lt(&s("a")), Ok(true), "bytewise order");
        assert_eq!(s("ab").gml_lt(&s("abc")), Ok(true));
        assert_eq!(s("b").gml_ge(&s("abc")), Ok(true));
    }

    #[test]
    fn bitwise_ops() {
        assert_eq!(r(6.0).bit_and(&r(3.0)), Ok(r(2.0)));
        assert_eq!(r(6.0).bit_or(&r(3.0)), Ok(r(7.0)));
        assert_eq!(r(6.0).bit_xor(&r(3.0)), Ok(r(5.0)));
        assert_eq!(r(1.0).shl(&r(3.0)), Ok(r(8.0)));
        assert_eq!(r(8.0).shr(&r(3.0)), Ok(r(1.0)));
        assert_eq!(r(0.0).bit_not(), Ok(r(18_446_744_073_709_551_615.0)));
    }

    #[test]
    fn function_argument_coercion() {
        assert_eq!(s("12").to_real(), 0.0, "strings read as 0, not parsed");
        assert_eq!(r(2.5).to_int(), 2);
        assert_eq!(r(-2.5).to_int(), -2);
        assert_eq!(r(3.5).to_int(), 4);
        assert_eq!(s("7").to_int(), 0);
        assert_eq!(r(5.0).to_str(), b"");
        assert_eq!(s("hi").to_str(), b"hi");
        assert_eq!(Value::default(), r(0.0));
        assert!(r(1.0).is_real() && !r(1.0).is_string());
        assert!(s("").is_string() && !s("").is_real());
    }

    #[test]
    fn string_and_real_conversions() {
        assert_eq!(r(15.0).to_gml_string(), GmString::from("15"));
        assert_eq!(r(1.5).to_gml_string(), GmString::from("1.50"));
        assert_eq!(s("x").to_gml_string(), GmString::from("x"));
        assert_eq!(s(" 12 ").to_gml_real(), Ok(12.0));
        assert_eq!(r(3.0).to_gml_real(), Ok(3.0));
        assert_eq!(s("x").to_gml_real(), Err(ValueError::BadRealString));
        assert_eq!(s("x").string_format(5, 2), GmString::from("x"));
        assert_eq!(r(2.0).string_format(5, 2), GmString::from(" 2.00"));
        // The game's own pattern: `string(real(argument1) - 10000)`.
        let v = r(s("10042").to_gml_real().unwrap())
            .sub(&r(10000.0))
            .unwrap();
        assert_eq!(v.to_gml_string(), GmString::from("42"));
    }
}
