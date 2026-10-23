//! Recording what a tick draws, as a `draw::Frame` (runtime.md section 11; the `draw` crate's docs).
//! The draw stage always runs, because draw events change state; what it draws is only kept when
//! recording is on, so headless runs pay nothing for pixels.

use crate::World;

/// The drawing of the current or last tick, if recording.
#[derive(Clone, Debug, Default)]
pub(crate) struct DrawList {
    pub(crate) record: bool,
    pub(crate) frame: draw::Frame,
}

impl World {
    /// Turns recording of draw commands on or off. Off by default.
    pub fn record_draw(&mut self, on: bool) {
        self.draw.record = on;
        self.draw.frame.commands = Vec::new();
    }

    /// Adds a command to this tick's drawing, if recording. The draw built-ins call this.
    #[inline]
    pub fn push_draw(&mut self, cmd: impl Into<draw::Command>) {
        if self.draw.record {
            self.draw.frame.push(cmd);
        }
    }

    /// Starts a tick's recording.
    // TODO(851-3128): the back buffer size comes from the views, and the frame's start state (target,
    // view, blend) from the drawing state the draw built-ins track across ticks.
    pub(crate) fn begin_draw_frame(&mut self) {
        if self.draw.record {
            let (w, h) = (
                self.room.width.max(0) as u32,
                self.room.height.max(0) as u32,
            );
            self.draw.frame = draw::Frame::new(w, h);
        }
    }

    /// Everything the last tick drew, in order (empty unless recording).
    pub fn draw_frame(&self) -> &draw::Frame {
        &self.draw.frame
    }

    /// Replays the last tick's drawing into a sink.
    pub fn replay_draw(&self, sink: &mut dyn draw::DrawSink) {
        self.draw.frame.replay(sink);
    }
}
