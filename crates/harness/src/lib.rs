//! Running replays on the port, headless (`replay-run`), and comparing them with the oracle (`fidelity`).
//! Its own crate so that the other tools, the transpiler above all, don't depend on the generated game.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use game::audio::{Mode, SoundBank};
use game::replay::{Playback, RunError};
use game::{Config, Game};
use serde::Serialize;
use trace::{Filter, Replay, TraceHeader, TraceWriter};

/// The asset bundle in `dir` (what `scripts/extract` writes), as the runtime uses it.
pub fn load_assets(dir: &Path) -> Result<Arc<gm::Assets>, String> {
    Ok(Arc::new(gm::Assets::from_bundle(load_bundle(dir)?)))
}

/// The game's sound files, the asset bundle's `sounds` in `dir`, keyed as the game loads them
/// (`sound/jump.wav`). A file the model can't read is left out with a warning on stderr: the DLL can't
/// load it either, so `SS_LoadSound` answers `"No"` for it.
pub fn load_sounds(dir: &Path, sounds: &[assets::SoundFile]) -> Result<Arc<SoundBank>, String> {
    let mut bank = SoundBank::new();
    for s in sounds {
        let path = dir.join(&s.file);
        let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        if let Err(e) = bank.insert(&format!("sound/{}", s.name), bytes) {
            eprintln!(
                "warning: {}: {e}; SS_LoadSound will answer \"No\" for it",
                path.display()
            );
        }
    }
    Ok(Arc::new(bank))
}

/// The asset bundle in `dir`.
pub fn load_bundle(dir: &Path) -> Result<assets::Bundle, String> {
    let path = dir.join(assets::BUNDLE_FILE);
    let bytes = std::fs::read(&path).map_err(|e| {
        format!(
            "{}: {e} (run scripts/extract to build the assets)",
            path.display()
        )
    })?;
    assets::Bundle::from_bytes(&bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// A replay file.
pub fn load_replay(path: &Path) -> Result<Replay, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Replay::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// How to run a replay on the port.
#[derive(Clone, Debug, Default)]
pub struct RunOptions {
    /// Where to write the trace; none if `None`.
    pub trace: Option<PathBuf>,
    /// Dump every instance's variables and the globals, like the oracle's `--fields`.
    pub fields: bool,
    /// Stop before this frame.
    pub frames: Option<u64>,
    /// Start from this savestate instead of booting.
    pub savestate_in: Option<PathBuf>,
    /// Write a savestate here: before frame `savestate_at` runs, or after the last frame.
    pub savestate_out: Option<PathBuf>,
    pub savestate_at: Option<u64>,
    /// Time the run (`RunSummary::profile`).
    pub profile: bool,
    /// Model supersound.dll as the real game has it ([`Mode::Faithful`]), with these sound files
    /// ([`load_sounds`]). Without them, the sound scripts answer like the oracle's stub
    /// ([`Mode::OracleCompat`]), so the trace can be compared with the oracle's.
    pub sounds: Option<Arc<SoundBank>>,
}

/// How a run ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunOutcome {
    /// Every frame asked for ran.
    Completed,
    /// The game called `game_end`.
    GameEnded,
    /// GML (or a room change) failed; see `error`.
    Error,
}

/// Where and why a run stopped early, for people and agents.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RunFailure {
    /// The frame that failed (it isn't in the trace), or `None` while booting, before frame 0.
    pub frame: Option<u64>,
    pub room: Option<String>,
    /// The GML unit (`oScreen/create#0`, `scrInit`...) and line that were running.
    pub unit: Option<String>,
    pub line: Option<u32>,
    /// The built-in or override the port doesn't have yet, if that's the cause.
    pub unsupported: Option<String>,
    /// The ticket that will implement it.
    pub ticket: Option<String>,
    /// The whole error, located.
    pub message: String,
}

impl From<&RunError> for RunFailure {
    fn from(e: &RunError) -> Self {
        let unsupported = e.unsupported();
        // Shims name their ticket: `draw_text (851-3175)`.
        let (name, ticket) = match unsupported.and_then(|u| u.strip_suffix(')')?.split_once(" (")) {
            Some((name, ticket)) => (Some(name), Some(ticket)),
            None => (unsupported, None),
        };
        RunFailure {
            frame: e.frame,
            room: e.room.clone(),
            unit: e.gml.as_ref().map(|g| g.unit.to_string()),
            line: e.gml.as_ref().map(|g| g.line),
            unsupported: name.map(str::to_string),
            ticket: ticket.map(str::to_string),
            message: e.to_string(),
        }
    }
}

/// What a run did.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RunSummary {
    pub outcome: RunOutcome,
    /// The frames that ran (and are in the trace): `first..=last`, none if `frames_run` is 0.
    pub frames_run: u64,
    pub first_frame: Option<u64>,
    pub last_frame: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RunFailure>,
    /// What became of the savestate asked for with `savestate_out`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub savestate: Option<SavestateReport>,
    /// How fast it ran, if asked for (it isn't deterministic).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<Profile>,
}

impl RunSummary {
    /// Whether the run did everything asked: no GML error, and the savestate (if any) written.
    pub fn succeeded(&self) -> bool {
        self.outcome != RunOutcome::Error && self.savestate.as_ref().is_none_or(|s| s.written)
    }
}

/// The savestate a run was asked to write. A file from an earlier run is deleted first, so the path never
/// holds a stale savestate.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SavestateReport {
    pub path: PathBuf,
    pub written: bool,
    /// The frame it resumes at, if written.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame: Option<u64>,
    /// Why it wasn't written.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// How fast a run was.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Profile {
    /// Headless speed: frames per second of stepping alone (booting and tracing excluded).
    pub ticks_per_second: f64,
    /// Seconds spent stepping, and writing the trace.
    pub step_seconds: f64,
    pub trace_seconds: f64,
}

impl RunSummary {
    /// One line for a terminal.
    pub fn to_text(&self) -> String {
        let frames = match (self.first_frame, self.last_frame) {
            (Some(a), Some(b)) => format!("{} frames ({a}..={b})", self.frames_run),
            _ => "no frames".to_string(),
        };
        let mut s = match (&self.outcome, &self.error) {
            (RunOutcome::Error, Some(e)) => {
                format!("stopped after {frames}. {e_msg}", e_msg = e.message)
            }
            (RunOutcome::GameEnded, _) => format!("ran {frames}; the game ended (game_end)"),
            _ => format!("ran {frames}"),
        };
        match &self.savestate {
            Some(SavestateReport {
                path,
                frame: Some(frame),
                ..
            }) => s += &format!("\nsavestate: {} (resumes at frame {frame})", path.display()),
            Some(SavestateReport {
                path,
                reason: Some(why),
                ..
            }) => s += &format!("\nsavestate: none written to {}: {why}", path.display()),
            _ => {}
        }
        if let Some(p) = &self.profile {
            s += &format!(
                "\nprofile: {:.0} ticks/s ({:.3} s stepping, {:.3} s writing the trace)",
                p.ticks_per_second, p.step_seconds, p.trace_seconds
            );
        }
        s
    }
}

/// Runs a replay on the port: boots it (or loads `savestate_in`), steps it frame by frame with the
/// replay's input, and writes a trace line after each frame. On a GML error the trace holds the frames
/// before the failing one, and the summary says where it failed. A savestate asked for but not written
/// (the run stopped first) is reported in the summary. `Err` is only for I/O and bad input, and bad input
/// is caught before any file is created or deleted.
pub fn run_replay(
    replay: &Replay,
    assets: Arc<gm::Assets>,
    opts: &RunOptions,
) -> Result<RunSummary, String> {
    let resumed = match &opts.savestate_in {
        Some(path) => {
            let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
            let sounds = opts.sounds.clone().unwrap_or_default();
            let game = Game::load_state(assets.clone(), sounds, &bytes)
                .map_err(|e| format!("{}: {e}", path.display()))?;
            // The mode is part of the state; don't quietly run in the other one.
            let (saved, asked) = (game.audio().mode(), run_audio_mode(opts));
            if saved != asked {
                return Err(format!(
                    "{}: the savestate's audio mode is {saved:?}, but this run's is {asked:?} \
                     (--faithful-audio for Faithful)",
                    path.display()
                ));
            }
            Some(game)
        }
        None => None,
    };
    let start = resumed.as_ref().map_or(0, |g| g.world().frames());
    if let Some(at) = opts.savestate_at
        && at < start
    {
        return Err(format!(
            "--at {at} is before the first frame to run ({start})"
        ));
    }
    if let Some(path) = &opts.savestate_out {
        match std::fs::remove_file(path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                return Err(format!("{}: {e}", path.display()));
            }
            _ => {}
        }
    }
    let mut writer = match &opts.trace {
        Some(path) => {
            let file = File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
            let w = TraceWriter::with_header(
                BufWriter::with_capacity(1 << 20, file),
                TraceHeader::new(Some("port")),
                Filter::default(),
            )
            .map_err(|e| format!("{}: {e}", path.display()))?;
            Some((w, path))
        }
        None => None,
    };
    let mut summary = RunSummary {
        outcome: RunOutcome::Completed,
        frames_run: 0,
        first_frame: None,
        last_frame: None,
        error: None,
        savestate: None,
        profile: None,
    };
    let (mut step_time, mut trace_time) = (Duration::ZERO, Duration::ZERO);
    let booted = match resumed {
        Some(game) => Ok(Playback::resume(game, replay)),
        None => {
            let mut config = Config::from_replay(&replay.header);
            config.audio = run_audio_mode(opts);
            if let Some(sounds) = &opts.sounds {
                config.sounds = sounds.clone();
            }
            Playback::boot_with(assets, config, replay)
        }
    };
    let mut playback = match booted {
        Ok(p) => Some(p),
        Err(e) => {
            summary.outcome = RunOutcome::Error;
            summary.error = Some(RunFailure::from(&e));
            None
        }
    };
    if let Some(p) = &mut playback {
        if let Some(end) = opts.frames {
            p.stop_before(end);
        }
        loop {
            if opts.savestate_at == Some(p.next_frame())
                && let Some(path) = &opts.savestate_out
            {
                summary.savestate = Some(write_savestate(p.game(), path)?);
            }
            let t = Instant::now();
            let stepped = p.step();
            step_time += t.elapsed();
            match stepped {
                Ok(Some(frame)) => {
                    summary.frames_run += 1;
                    summary.first_frame.get_or_insert(frame);
                    summary.last_frame = Some(frame);
                    if let Some((w, path)) = &mut writer {
                        let t = Instant::now();
                        w.write_frame(&p.game().trace_with(opts.fields))
                            .map_err(|e| format!("{}: {e}", path.display()))?;
                        trace_time += t.elapsed();
                    }
                }
                Ok(None) => {
                    if p.game_ended() {
                        summary.outcome = RunOutcome::GameEnded;
                    }
                    break;
                }
                Err(e) => {
                    summary.outcome = RunOutcome::Error;
                    summary.error = Some(RunFailure::from(&e));
                    break;
                }
            }
        }
    }
    if let Some((w, path)) = writer {
        w.into_inner()
            .flush()
            .map_err(|e| format!("{}: {e}", path.display()))?;
    }
    if let Some(path) = &opts.savestate_out
        && summary.savestate.is_none()
    {
        summary.savestate = Some(match (&playback, opts.savestate_at) {
            // After the last frame, unless the run failed.
            (Some(p), None) if summary.outcome != RunOutcome::Error => {
                write_savestate(p.game(), path)?
            }
            _ => {
                // The run stopped before the savestate's frame: the same answer whether a GML error,
                // `game_end` or the end of the replay stopped it.
                let wanted = match opts.savestate_at {
                    Some(at) => format!("frame {at}"),
                    None => "the end".into(),
                };
                let stopped = match (&summary.error, summary.outcome) {
                    (Some(e), _) => format!("a GML error stopped the run: {}", e.message),
                    (None, RunOutcome::GameEnded) => format!(
                        "the game ended (game_end) before frame {}",
                        summary.last_frame.map_or(start, |f| f + 1)
                    ),
                    (None, _) => format!(
                        "the run ended before frame {}",
                        summary.last_frame.map_or(start, |f| f + 1)
                    ),
                };
                SavestateReport {
                    path: path.clone(),
                    written: false,
                    frame: None,
                    reason: Some(format!("{stopped}, so it never reached {wanted}")),
                }
            }
        });
    }
    if opts.profile {
        let step_seconds = step_time.as_secs_f64();
        summary.profile = Some(Profile {
            ticks_per_second: summary.frames_run as f64 / step_seconds.max(1e-9),
            step_seconds,
            trace_seconds: trace_time.as_secs_f64(),
        });
    }
    Ok(summary)
}

/// The audio mode `opts` asks for: faithful with sounds, else the oracle's.
fn run_audio_mode(opts: &RunOptions) -> Mode {
    if opts.sounds.is_some() {
        Mode::Faithful
    } else {
        Mode::OracleCompat
    }
}

fn write_savestate(game: &Game, path: &Path) -> Result<SavestateReport, String> {
    std::fs::write(path, game.save_state()).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(SavestateReport {
        path: path.to_path_buf(),
        written: true,
        frame: Some(game.world().frames()),
        reason: None,
    })
}
