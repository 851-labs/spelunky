//! Replay and trace formats shared by the port and the reference oracle.
//!
//! - A **replay** ([`replay`]) is the input to a run: the seed and other starting conditions, then the input
//!   events of each frame, as the oracle's hand-writable text script (plus extensions).
//! - A **trace** ([`trace`]) is the output of a run: JSON lines in the oracle's schema, one per frame, with the
//!   room, RNG state, globals and every instance's built-ins and variables.
//! - [`canon`] is the canonical encoding of a frame that replay fingerprints hash.
//! - [`diff`] compares two traces and finds the first frame where they diverge (the `trace-diff` tool).
//!
//! The formats are documented in `docs/formats.md`. The oracle (patched OpenGMK) defined them first; this
//! crate reads its output as is.

pub mod canon;
pub mod diff;
pub mod filter;
pub mod keys;
pub mod replay;
pub mod trace;
pub mod value;

mod error;

pub use error::Error;
pub use filter::Filter;
pub use replay::{Event, InputState, Replay, ReplayHeader};
pub use trace::{Frame, Instance, TraceHeader, TraceReader, TraceWriter};
pub use value::{Real, Scalar, Value};

/// The game version string both formats carry, so traces from a different build are never compared silently.
pub const GAME_VERSION: &str = "spelunky-classic-1.1";
