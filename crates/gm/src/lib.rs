//! The subset of the GameMaker 8 runtime that Spelunky Classic 1.1 uses.
//!
//! No graphics, audio, wall-clock time, or global state: the whole world is a plain value. The design is
//! `docs/design/runtime.md`; generated game code calls the API in its section 12:
//!
//! - [`World`]: instances, globals, room, RNG and input. [`World::step`] runs one frame.
//! - [`Program`]: the static table of objects and resolved event handlers, built with
//!   [`ProgramBuilder`].
//! - [`Ctx`]: `self`, `other`, arguments and return value of the code being run.
//! - Variables: [`World::get`]/[`World::set`] for instance variables, [`World::gget`] for globals,
//!   [`World::bget`] for game-wide built-ins, [`World::get_on`] for `target.v`, [`Local`] for `var`s.
//! - `with`: [`World::with_iter`]; instance built-ins in [`kernel`]; collision built-ins
//!   ([`World::collision_point`] and friends) and the collision event stage ([`World::run_collisions`]).
//! - Drawing, fonts, window and dialogs: [`World::draw_text`] and friends (`draw_builtins`).
//! - Output: [`World::trace_frame`] for oracle diffs, [`World::step_drawing`] for the renderer.
//! - Savestates: `World::clone` in memory, [`World::save_state`]/[`World::load_state`] on disk.

mod access;
mod collision;
#[cfg(test)]
mod collision_props;
mod ctx;
mod draw_builtins;
mod drawing;
mod dump;
mod error;
mod frame;
mod host;
mod index;
pub mod input;
mod instance;
pub mod kernel;
mod list;
pub mod math;
mod ops;
mod program;
pub mod rng;
mod room;
mod savestate;
pub mod string;
mod surfaces;
pub mod value;
mod var;
mod world;

pub use ctx::{Ctx, MAX_ARGS};
pub use error::{Error, GmlError, R};
pub use host::{Extension, ExtensionError, HIGHSCORE_ENTRIES, Host};
pub use instance::{ALARMS, BBOX_DEFAULT, Bbox, Instance, NO_MASK_BBOX, State};
pub use kernel::WithIter;
pub use list::Inst;
pub use ops::{ARRAY_STRIDE, case_eq, ix, lenient, repeat_count};
pub use program::{
    Event, EventFn, Handler, ObjectId, ObjectInfo, Program, ProgramBuilder, StageFn,
};
pub use rng::Rng;
pub use room::{BACKGROUNDS, Background, RoomState, Tile, VIEWS, View};
pub use savestate::{SAVESTATE_VERSION, SavestateError};
pub use string::GmString;
pub use trace::replay::Event as InputEvent;
pub use value::{Op, Value, ValueError};
pub use var::{Builtin, FIRST_USER_VAR, GVar, Local, Slot, Var, VarLayout};
pub use world::{Assets, SceneChange, World};
