//! Playing a replay headless (`docs/formats.md`), with the oracle's frame conventions
//! (`tools/oracle/README.md`, "Dumps"), so the port's trace lines up with the oracle's line for line:
//!
//! - Booting enters the replay's first room (`rScreenInit` by default), before frame 0. Its creation asks
//!   for `rIntro`, which booting enters too, as GM8 does before its first frame, so frame 0 is a full
//!   `rIntro` frame, as in the oracle.
//! - Frame `f` applies the replay's events for `f` and runs one step, room change included.
//!   [`Game::trace`] after that is the oracle's line for frame `f`.

use std::fmt;
use std::sync::Arc;

use gm::{Error, GmlError, InputEvent};
use trace::Replay;

use crate::{Config, Game, describe};

/// Why a replay stopped before its end.
#[derive(Clone, Debug, PartialEq)]
pub struct RunError {
    /// The frame that failed, or `None` while booting (before frame 0).
    pub frame: Option<u64>,
    /// The room the game was in.
    pub room: Option<String>,
    /// The GML error, if GML failed. It holds the unit and line that were running.
    pub gml: Option<GmlError>,
    /// What went wrong, with GML errors located and their variables named ([`describe`]).
    pub message: String,
}

impl RunError {
    /// The built-in or override the port doesn't have yet, with its ticket (`"draw_text (851-3175)"`), if
    /// that's what stopped the run.
    pub fn unsupported(&self) -> Option<&'static str> {
        match self.gml.as_ref()?.error {
            Error::Unsupported(what) => Some(what),
            _ => None,
        }
    }
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.frame {
            Some(frame) => write!(f, "frame {frame}")?,
            None => write!(f, "booting (before frame 0)")?,
        }
        if let Some(room) = &self.room {
            write!(f, " in {room}")?;
        }
        write!(f, ": {}", self.message)
    }
}

impl std::error::Error for RunError {}

/// A game playing a replay, one frame at a time.
pub struct Playback<'r> {
    game: Game,
    /// The replay's events, sorted by frame.
    events: &'r [(u64, InputEvent)],
    /// The first event of the next frame.
    next_event: usize,
    /// The frame to stop before.
    end: u64,
}

impl<'r> Playback<'r> {
    /// A new game set up as the replay says, booted: its first room entered and the room changes that
    /// requested performed, ready for frame 0.
    pub fn boot(assets: Arc<gm::Assets>, replay: &'r Replay) -> Result<Playback<'r>, RunError> {
        Playback::boot_with(assets, Config::from_replay(&replay.header), replay)
    }

    /// [`boot`](Self::boot) with another configuration than the replay's ([`Config::from_replay`]), e.g.
    /// to play the sounds faithfully.
    pub fn boot_with(
        assets: Arc<gm::Assets>,
        config: Config,
        replay: &'r Replay,
    ) -> Result<Playback<'r>, RunError> {
        let game = Game::new(assets, config);
        let mut p = Playback::resume(game, replay);
        let name = &replay.header.room;
        let room = p
            .game
            .world()
            .assets()
            .room_id(name)
            .ok_or_else(|| RunError {
                frame: None,
                room: None,
                gml: None,
                message: format!("the replay starts in room {name}, which doesn't exist"),
            })?;
        p.game.start_at(room).map_err(|e| p.error(None, e))?;
        Ok(p)
    }

    /// Continues a game (a savestate) from its next frame, with the replay's events for the frames
    /// still to come. The replay should be the one the game was playing, or extend it.
    pub fn resume(game: Game, replay: &'r Replay) -> Playback<'r> {
        let frames = game.world().frames();
        let events = replay.events();
        Playback {
            game,
            events,
            next_event: events.partition_point(|&(f, _)| f < frames),
            end: replay.len(),
        }
    }

    /// Stops before frame `end` instead of at the end of the replay (never after it).
    pub fn stop_before(&mut self, end: u64) {
        self.end = self.end.min(end);
    }

    /// The frame [`step`](Self::step) runs next.
    pub fn next_frame(&self) -> u64 {
        self.game.world().frames()
    }

    /// Runs the next frame and returns its number, or `None` if the replay is over (or the game ended).
    /// After an error the game is mid-frame: don't step it further.
    pub fn step(&mut self) -> Result<Option<u64>, RunError> {
        let frame = self.next_frame();
        if self.game_ended() || frame >= self.end {
            return Ok(None);
        }
        let rest = &self.events[self.next_event..];
        let n = rest.iter().take_while(|&&(f, _)| f == frame).count();
        self.next_event += n;
        self.game
            .step(rest[..n].iter().map(|(_, e)| e))
            .map_err(|e| self.error(Some(frame), e))?;
        Ok(Some(frame))
    }

    /// Whether the game called `game_end`.
    pub fn game_ended(&self) -> bool {
        self.game.ended()
    }

    pub fn game(&self) -> &Game {
        &self.game
    }

    /// The game, to take it when done (a savestate) or to look inside.
    pub fn into_game(self) -> Game {
        self.game
    }

    fn error(&self, frame: Option<u64>, e: GmlError) -> RunError {
        let w = self.game.world();
        RunError {
            frame,
            room: w.assets().room_name(w.room().id).map(str::to_string),
            message: describe(&e),
            gml: Some(e),
        }
    }
}
