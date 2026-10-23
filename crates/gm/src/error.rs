//! GML runtime errors.
//!
//! Spelunky is built with "display errors" on and "treat uninitialized variables as 0" off, so GM8 shows a
//! dialog for each of these and lets the player continue. The port can't press "Ignore": it stops the tick
//! and reports the error with the GML unit and line that was running ([`GmlError`]).

use crate::value::ValueError;
use crate::var::{GVar, Var};
use std::fmt;

/// The result of anything generated code calls.
pub type R<T> = Result<T, Error>;

/// A GML runtime error, without its location (the world's cursor has that; see [`GmlError`]).
#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    /// An operator or value built-in failed (`"a" - 1`, `real("x")`).
    Value(ValueError),
    /// Reading an instance variable (built-in arrays included) that was never assigned, or reading
    /// `obj.v` when no instance of `obj` exists. OpenGMK `gml/runtime.rs` (`UninitializedVariable`).
    UnsetVar(Var),
    /// Reading a `global.` variable that was never assigned.
    UnsetGlobal(GVar),
    /// Reading a `var` local that was never assigned.
    UnsetLocal,
    /// Writing a read-only built-in (`id`, `object_index`, `bbox_*`, ...). OpenGMK `gml/runtime.rs`
    /// (`ReadOnlyVariable`).
    ReadOnly(Var),
    /// An array index that rounds to a value outside `0..32000`. OpenGMK `gml/runtime.rs`
    /// (`get_array_index`, `InvalidArrayIndex`).
    BadArrayIndex(i32),
    /// `instance_create` (or similar) with an object id that doesn't exist.
    NoSuchObject(i32),
    /// A built-in variable or target the port doesn't implement yet. Holds a description; reaching it means
    /// the game uses something this runtime was scoped without.
    Unsupported(&'static str),
}

impl From<ValueError> for Error {
    fn from(e: ValueError) -> Self {
        Error::Value(e)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Value(e) => e.fmt(f),
            Error::UnsetVar(v) => write!(f, "unknown variable (id {})", v.0),
            Error::UnsetGlobal(v) => write!(f, "unknown global variable (id {})", v.0),
            Error::UnsetLocal => f.write_str("unknown local variable"),
            Error::ReadOnly(v) => write!(f, "cannot assign to read-only variable (id {})", v.0),
            Error::BadArrayIndex(i) => write!(f, "array index {i} out of range 0..32000"),
            Error::NoSuchObject(o) => write!(f, "object {o} does not exist"),
            Error::Unsupported(what) => write!(f, "not supported by the port yet: {what}"),
        }
    }
}

impl std::error::Error for Error {}

/// An error that stopped a tick, with the GML unit and line that was running (the last
/// [`World::at`](crate::World::at) call).
#[derive(Clone, Debug, PartialEq)]
pub struct GmlError {
    pub error: Error,
    pub unit: &'static str,
    pub line: u32,
}

impl fmt::Display for GmlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.unit, self.line, self.error)
    }
}

impl std::error::Error for GmlError {}
