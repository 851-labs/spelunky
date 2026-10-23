//! Recording what a tick draws, as a `draw::Frame` (runtime.md section 11; the `draw` crate's docs).
//! The draw stage always runs, because draw events change state. What it draws is only kept when the
//! caller passes a frame to [`World::step_drawing`], so headless runs pay nothing for pixels. The frame
//! belongs to the caller and is lent to the world for the tick only, so it is reused from tick to tick and
//! cloning a world never copies drawing.

use crate::World;

impl World {
    /// Adds a command to this tick's drawing, if recording. The draw built-ins call this.
    #[inline]
    pub fn push_draw(&mut self, cmd: impl Into<draw::Command>) {
        if let Some(frame) = &mut self.draw {
            frame.push(cmd);
        }
    }

    /// Whether this tick's drawing is being recorded (so built-ins can skip building commands).
    #[inline]
    pub fn recording_draw(&self) -> bool {
        self.draw.is_some()
    }

    /// Whether GM8 shows the tick just run. It doesn't flip the back buffer when the frame ends with a
    /// room change pending (OpenGMK `game.rs` `Game::frame`, the final `present`), so a frontend should
    /// keep showing the previous picture. The tick's commands still ran: surfaces they drew into keep
    /// their contents.
    pub fn presents(&self) -> bool {
        self.pending.is_none()
    }

    /// Resets a reused frame for a new tick.
    // TODO(851-3128): the back buffer size comes from the views, and the frame's start state (target,
    // view, blend) from the drawing state the draw built-ins track across ticks.
    pub(crate) fn begin_draw_frame(&self, frame: &mut draw::Frame) {
        let (w, h) = (
            self.room.width.max(0) as u32,
            self.room.height.max(0) as u32,
        );
        frame.width = w;
        frame.height = h;
        frame.start = draw::DrawState::initial(w, h);
        frame.commands.clear();
    }
}
