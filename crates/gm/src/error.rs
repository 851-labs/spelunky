//! GML runtime errors.
//!
//! Spelunky is built with "display errors" on and "treat uninitialized variables as 0" off, so GM8 shows a
//! dialog for each of these and lets the player continue. The port can't press "Ignore": it stops the tick
//! and reports the error with the GML unit and line that was running ([`GmlError`]).

use crate::string::GmString;
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
    /// Drawing a sprite, or making a font from one, that doesn't exist. OpenGMK `gml/kernel.rs`
    /// (`NonexistentAsset`).
    NoSuchSprite(i32),
    /// `show_error(text, abort)`. GM8 shows the message and, unless `abort`, offers to continue; the oracle
    /// stops the game whatever `abort` says (OpenGMK `gml/kernel.rs`, `show_error`), and so does the port.
    ShowError(GmString),
    /// A room change to a room that doesn't exist, found when the change happens. OpenGMK `game.rs`
    /// (`load_room`, `NonexistentAsset`).
    NoSuchRoom(i32),
    /// `tile_add` with a background that doesn't exist. OpenGMK `gml/kernel.rs` (`tile_add`).
    NoSuchBackground(i32),
    /// `tile_delete` of a tile that doesn't exist. OpenGMK `gml/kernel.rs` (`tile_delete`).
    NoSuchTile(i32),
    /// `room_goto_next` in the last room. OpenGMK `gml/kernel.rs` (`room_goto_next`, `EndOfRoomOrder`).
    EndOfRoomOrder,
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
            Error::NoSuchSprite(s) => write!(f, "sprite {s} does not exist"),
            Error::ShowError(text) => write!(f, "show_error: {text}"),
            Error::NoSuchRoom(r) => write!(f, "room {r} does not exist"),
            Error::NoSuchBackground(b) => write!(f, "background {b} does not exist"),
            Error::NoSuchTile(t) => write!(f, "tile {t} does not exist"),
            Error::EndOfRoomOrder => f.write_str("moving past the end of the room order"),
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
