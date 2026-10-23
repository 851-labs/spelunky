//! Native and web frontend for the Spelunky Classic 1.1 port: a winit window that plays the game
//! ([`play`]), a wgpu renderer for the `draw` crate's draw lists, and the game's sound and saves.
//!
//! Everything here builds for wasm32 too. The pieces that can't run in a browser (blocking GPU readback,
//! the file system, the command line) are native-only, and asset loading takes bytes from a callback.

pub mod gpu;
pub mod input;
pub mod notice;
pub mod play;
pub mod present;
pub mod preview;
pub mod render;
pub mod save;
pub mod textures;
#[cfg(target_arch = "wasm32")]
pub mod web;
pub mod window;
