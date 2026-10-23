//! Native and web frontend for the Spelunky Classic 1.1 port: a winit window and a wgpu renderer for the
//! `draw` crate's draw lists.
//!
//! Everything here builds for wasm32 too. The pieces that can't run in a browser (blocking GPU readback,
//! the file system, the command line) are native-only, and asset loading takes bytes from a callback.

pub mod gpu;
pub mod present;
pub mod preview;
pub mod render;
pub mod textures;
pub mod window;
