//! The subset of the GameMaker 8 runtime that Spelunky Classic 1.1 uses.
//!
//! No graphics, audio, wall-clock time, or global state: the whole world is a plain value.

pub mod rng;

pub use rng::Rng;
