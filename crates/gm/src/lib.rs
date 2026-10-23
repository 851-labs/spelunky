//! The subset of the GameMaker 8 runtime that Spelunky Classic 1.1 uses.
//!
//! No graphics, audio, wall-clock time, or global state: the whole world is a plain value. The design is
//! `docs/design/runtime.md`; generated game code calls the API in its section 12:
//!
//! - [`World`]: instances, globals, room and RNG.
//! - [`Program`]: the static table of objects and resolved event handlers, built with
//!   [`ProgramBuilder`].
//! - [`Ctx`]: `self`, `other`, arguments and return value of the code being run.
//! - Variables: [`World::get`]/[`World::set`] for instance variables, [`World::gget`] for globals,
//!   [`World::bget`] for game-wide built-ins, [`World::get_on`] for `target.v`, [`Local`] for `var`s.
//! - `with`: [`World::with_iter`]; instance built-ins in [`kernel`].

mod access;
mod ctx;
mod error;
mod instance;
pub mod kernel;
mod list;
pub mod math;
mod ops;
mod program;
pub mod rng;
pub mod string;
pub mod value;
mod var;
mod world;

pub use ctx::{Ctx, MAX_ARGS};
pub use error::{Error, GmlError, R};
pub use instance::{ALARMS, BBOX_DEFAULT, Bbox, Instance, State};
pub use kernel::WithIter;
pub use list::Inst;
pub use ops::{ARRAY_STRIDE, case_eq, ix, lenient, repeat_count};
pub use program::{Event, EventFn, Handler, ObjectId, ObjectInfo, Program, ProgramBuilder};
pub use rng::Rng;
pub use string::GmString;
pub use value::{Op, Value, ValueError};
pub use var::{Builtin, FIRST_USER_VAR, GVar, Local, Slot, Var, VarLayout};
pub use world::{Assets, PlacedInstance, RoomStart, RoomState, SceneChange, World};
