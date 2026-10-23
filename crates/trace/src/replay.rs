//! Replays: the starting conditions of a run plus the raw input events of every frame.
//!
//! The text form is the oracle's replay script (`seed`, `time`, `frames`, and `N press|release|hold KEY [n]`
//! lines), extended with more directives and events. A replay that uses no extension is written exactly in the
//! oracle's subset, so the oracle can run it. See `docs/formats.md` for the grammar.
//!
//! Input is recorded raw (key, mouse and joystick-button transitions, mouse position, joystick axes), not as
//! the game's `checkJump`-style actions, so the same replay drives any key binding and every code path that
//! reads input. Events are applied in order at the start of their frame, before any event of that frame runs;
//! [`Replay::inputs`] derives the held state of every frame from them.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::Error;
use crate::keys::{self, KeySet};
use crate::value::format_real;

/// The oracle's default spoofed start clock (2010-01-01, in ns since the Unix epoch).
pub const DEFAULT_TIME: u128 = 1_262_304_000_000_000_000;

/// The room the original starts in. Starting anywhere else skips `scrInit` and the rest of the boot rooms.
pub const DEFAULT_ROOM: &str = "rScreenInit";

/// Everything besides input that decides how a run plays out.
#[derive(Clone, Debug, PartialEq)]
pub struct ReplayHeader {
    /// The runner's RNG seed at the start of frame 0 (`seed`). GM 8 randomizes it at startup; a replay pins it.
    pub seed: i32,
    /// The start of the spoofed system clock in ns since the Unix epoch (`time`), for `current_time` and dates.
    pub time: u128,
    /// The first room (`room`, an extension).
    pub room: String,
    /// `settings.cfg` (`settings`, an extension).
    pub settings: Settings,
    /// `keys.cfg` (`keys`, an extension).
    pub keys: KeyConfig,
    /// `gamepad.cfg` (`gamepad`, an extension).
    pub gamepad: GamepadConfig,
    /// GM joystick 1, or `None` when none is connected and `joystick_exists` is false (`joystick`, an
    /// extension).
    pub joystick: Option<JoystickInfo>,
    /// The runner's highscore table at startup, which the game uses as save data (source inventory 7.2),
    /// best first. Empty leaves the runner's initial table; GM's table is never empty (`highscore`, an
    /// extension). [`Replay::parse`] accepts what GM's table can hold: at most [`HIGHSCORE_ENTRIES`] finite
    /// scores in descending order, and if fewer, none below 0 (the rest are the fresh table's 0 scores).
    pub highscores: Vec<Highscore>,
    /// Other files in the game directory at startup (`stats.txt`, `levels/*.lvl`): path to exact contents
    /// (`file`, an extension). The three config files come from the fields above.
    pub files: BTreeMap<String, String>,
}

impl Default for ReplayHeader {
    fn default() -> Self {
        ReplayHeader::new(0)
    }
}

impl ReplayHeader {
    /// A header with the shipped config files and no joystick.
    pub fn new(seed: i32) -> Self {
        ReplayHeader {
            seed,
            time: DEFAULT_TIME,
            room: DEFAULT_ROOM.to_string(),
            settings: Settings::default(),
            keys: KeyConfig::default(),
            gamepad: GamepadConfig::default(),
            joystick: None,
            highscores: Vec::new(),
            files: BTreeMap::new(),
        }
    }

    /// The files to place in the game directory before the run: the three config files, rendered exactly as
    /// `config.exe` writes them (CRLF between values, no trailing newline), plus [`files`](Self::files).
    pub fn game_files(&self) -> BTreeMap<String, String> {
        let mut files = self.files.clone();
        files.insert("settings.cfg".into(), cfg_lines(&self.settings.values()));
        files.insert("keys.cfg".into(), cfg_lines(&self.keys.values()));
        files.insert("gamepad.cfg".into(), cfg_lines(&self.gamepad.values()));
        files
    }
}

const CONFIG_FILES: [&str; 3] = ["settings.cfg", "keys.cfg", "gamepad.cfg"];

fn cfg_lines(values: &[i32]) -> String {
    values
        .iter()
        .map(i32::to_string)
        .collect::<Vec<_>>()
        .join("\r\n")
}

/// `settings.cfg`, in file order. The defaults are the shipped file (`0,1,0,1,3,15,15`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    pub fullscreen: i32,
    pub graphics_high: i32,
    pub down_to_run: i32,
    pub gamepad_on: i32,
    pub screen_scale: i32,
    pub music_vol: i32,
    pub sound_vol: i32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings::from_values(&[0, 1, 0, 1, 3, 15, 15])
    }
}

impl Settings {
    pub fn values(&self) -> Vec<i32> {
        vec![
            self.fullscreen,
            self.graphics_high,
            self.down_to_run,
            self.gamepad_on,
            self.screen_scale,
            self.music_vol,
            self.sound_vol,
        ]
    }

    fn from_values(v: &[i32; 7]) -> Self {
        Settings {
            fullscreen: v[0],
            graphics_high: v[1],
            down_to_run: v[2],
            gamepad_on: v[3],
            screen_scale: v[4],
            music_vol: v[5],
            sound_vol: v[6],
        }
    }
}

/// `keys.cfg`: virtual-key codes, in file order. The defaults are the shipped file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyConfig {
    pub up: i32,
    pub down: i32,
    pub left: i32,
    pub right: i32,
    pub jump: i32,
    pub attack: i32,
    pub item: i32,
    pub run: i32,
    pub bomb: i32,
    pub rope: i32,
    pub flare: i32,
    pub pay: i32,
}

impl Default for KeyConfig {
    fn default() -> Self {
        KeyConfig::from_values(&[38, 40, 37, 39, 90, 88, 67, 16, 65, 83, 70, 80])
    }
}

impl KeyConfig {
    pub fn values(&self) -> Vec<i32> {
        vec![
            self.up,
            self.down,
            self.left,
            self.right,
            self.jump,
            self.attack,
            self.item,
            self.run,
            self.bomb,
            self.rope,
            self.flare,
            self.pay,
        ]
    }

    fn from_values(v: &[i32; 12]) -> Self {
        KeyConfig {
            up: v[0],
            down: v[1],
            left: v[2],
            right: v[3],
            jump: v[4],
            attack: v[5],
            item: v[6],
            run: v[7],
            bomb: v[8],
            rope: v[9],
            flare: v[10],
            pay: v[11],
        }
    }
}

/// `gamepad.cfg`: joystick button numbers, in file order. The defaults are the shipped file, whose ninth
/// line is `start` (10). `None` writes an eight-line file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GamepadConfig {
    pub jump: i32,
    pub attack: i32,
    pub item: i32,
    pub run: i32,
    pub bomb: i32,
    pub rope: i32,
    pub flare: i32,
    pub pay: i32,
    pub start: Option<i32>,
}

impl Default for GamepadConfig {
    fn default() -> Self {
        GamepadConfig::from_values(&[2, 1, 3, 5, 7, 8, 4, 6, 10]).expect("nine values")
    }
}

impl GamepadConfig {
    pub fn values(&self) -> Vec<i32> {
        let mut v = vec![
            self.jump,
            self.attack,
            self.item,
            self.run,
            self.bomb,
            self.rope,
            self.flare,
            self.pay,
        ];
        v.extend(self.start);
        v
    }

    fn from_values(v: &[i32]) -> Option<Self> {
        let (&[jump, attack, item, run, bomb, rope, flare, pay], start) =
            v.split_first_chunk::<8>()?;
        let start = match start {
            [] => None,
            [s] => Some(*s),
            _ => return None,
        };
        Some(GamepadConfig {
            jump,
            attack,
            item,
            run,
            bomb,
            rope,
            flare,
            pay,
            start,
        })
    }
}

/// The connected joystick's fixed properties.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct JoystickInfo {
    /// `joystick_has_pov`.
    pub has_pov: bool,
}

/// How many entries GM 8's highscore table holds.
pub const HIGHSCORE_ENTRIES: usize = 10;

/// One highscore table entry.
#[derive(Clone, Debug, PartialEq)]
pub struct Highscore {
    pub name: String,
    pub value: f64,
}

/// A mouse button, numbered as GM's `mb_*` constants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left = 1,
    Right = 2,
    Middle = 3,
}

/// Something that is held down.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Button {
    /// A virtual-key code, 2-255.
    Key(u8),
    Mouse(MouseButton),
    /// A joystick button, 1-32.
    Joy(u8),
}

/// A joystick axis (`joystick_xpos`, `_ypos`, `_zpos`) or the POV hat (`joystick_pov`, degrees, -1 centred).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Axis {
    X,
    Y,
    Z,
    Pov,
}

impl Axis {
    const ALL: [Axis; 4] = [Axis::X, Axis::Y, Axis::Z, Axis::Pov];

    fn name(self) -> &'static str {
        match self {
            Axis::X => "x",
            Axis::Y => "y",
            Axis::Z => "z",
            Axis::Pov => "pov",
        }
    }
}

/// One input event. Events of a frame are applied in order, before the frame runs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    Press(Button),
    Release(Button),
    WheelUp,
    WheelDown,
    /// The mouse moves to this position in window client pixels (the runtime maps it into the room).
    MouseMove {
        x: i32,
        y: i32,
    },
    /// An axis changes to this value.
    Axis(Axis, f64),
}

/// The held input state during one frame, after that frame's events.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct InputState {
    pub keys: KeySet,
    /// Bit `b - 1` is mouse button `b`.
    pub mouse_buttons: u8,
    pub mouse_x: i32,
    pub mouse_y: i32,
    /// This frame's wheel event: 1 up, -1 down, 0 none. Not held: it resets every frame.
    pub wheel: i8,
    /// Bit `b - 1` is joystick button `b`.
    pub joy_buttons: u32,
    /// x, y, z and POV, indexed like [`Axis`].
    pub joy_axes: [f64; 4],
}

impl Default for InputState {
    fn default() -> Self {
        InputState {
            keys: KeySet::new(),
            mouse_buttons: 0,
            mouse_x: 0,
            mouse_y: 0,
            wheel: 0,
            joy_buttons: 0,
            joy_axes: [0.0, 0.0, 0.0, -1.0],
        }
    }
}

impl InputState {
    /// Only these keys down.
    pub fn keys(keys: impl IntoIterator<Item = u8>) -> Self {
        InputState {
            keys: keys.into_iter().collect(),
            ..InputState::default()
        }
    }

    pub fn is_down(&self, b: Button) -> bool {
        match b {
            Button::Key(vk) => self.keys.contains(vk),
            Button::Mouse(m) => self.mouse_buttons & (1 << (m as u8 - 1)) != 0,
            // Out-of-range buttons are never down.
            Button::Joy(n) => joy_bit(n).is_some_and(|bit| self.joy_buttons & bit != 0),
        }
    }

    pub fn axis(&self, axis: Axis) -> f64 {
        self.joy_axes[axis as usize]
    }

    fn set(&mut self, b: Button, down: bool) {
        match b {
            Button::Key(vk) if down => {
                self.keys.insert(vk);
            }
            Button::Key(vk) => self.keys.remove(vk),
            Button::Mouse(m) if down => self.mouse_buttons |= 1 << (m as u8 - 1),
            Button::Mouse(m) => self.mouse_buttons &= !(1 << (m as u8 - 1)),
            // Out-of-range buttons are ignored; `Replay::push` and the parser reject them.
            Button::Joy(n) => {
                let bit = joy_bit(n).unwrap_or(0);
                if down {
                    self.joy_buttons |= bit;
                } else {
                    self.joy_buttons &= !bit;
                }
            }
        }
    }

    /// Applies one event.
    pub fn apply(&mut self, event: &Event) {
        match *event {
            Event::Press(b) => self.set(b, true),
            Event::Release(b) => self.set(b, false),
            Event::WheelUp => self.wheel = 1,
            Event::WheelDown => self.wheel = -1,
            Event::MouseMove { x, y } => (self.mouse_x, self.mouse_y) = (x, y),
            Event::Axis(axis, v) => self.joy_axes[axis as usize] = v,
        }
    }

    /// The events that lead from this state to `next`: releases, then presses, then mouse movement, axes
    /// (compared bitwise) and `next`'s wheel.
    pub fn events_to(&self, next: &InputState) -> Vec<Event> {
        let (prev, now) = (self.buttons(), next.buttons());
        let mut events: Vec<Event> = prev
            .iter()
            .filter(|b| !now.contains(b))
            .map(|&b| Event::Release(b))
            .collect();
        events.extend(
            now.iter()
                .filter(|b| !prev.contains(b))
                .map(|&b| Event::Press(b)),
        );
        if (next.mouse_x, next.mouse_y) != (self.mouse_x, self.mouse_y) {
            events.push(Event::MouseMove {
                x: next.mouse_x,
                y: next.mouse_y,
            });
        }
        for axis in Axis::ALL {
            if next.axis(axis).to_bits() != self.axis(axis).to_bits() {
                events.push(Event::Axis(axis, next.axis(axis)));
            }
        }
        match next.wheel {
            1 => events.push(Event::WheelUp),
            -1 => events.push(Event::WheelDown),
            _ => {}
        }
        events
    }

    /// Every button held in this state, in a fixed order (keys ascending, then mouse, then joystick).
    fn buttons(&self) -> Vec<Button> {
        let mut out: Vec<Button> = self.keys.iter().map(Button::Key).collect();
        out.extend(
            [MouseButton::Left, MouseButton::Right, MouseButton::Middle]
                .into_iter()
                .map(Button::Mouse)
                .filter(|&b| self.is_down(b)),
        );
        out.extend((1..=32).map(Button::Joy).filter(|&b| self.is_down(b)));
        out
    }
}

/// A frame's input: its events and the state they leave.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameInput<'a> {
    pub frame: u64,
    pub events: &'a [(u64, Event)],
    pub state: InputState,
}

/// A header, a length in frames, and input events sorted by frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Replay {
    pub header: ReplayHeader,
    len: u64,
    events: Vec<(u64, Event)>,
}

impl Replay {
    pub fn new(header: ReplayHeader) -> Self {
        Replay {
            header,
            len: 0,
            events: Vec::new(),
        }
    }

    /// The number of frames to run.
    pub fn len(&self) -> u64 {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Sets the number of frames; fails if an event would fall outside.
    pub fn set_len(&mut self, len: u64) -> Result<(), Error> {
        match self.events.last() {
            Some((f, _)) if *f >= len => Err(Error::new(format!(
                "an event is on frame {f}, beyond {len} frames"
            ))),
            _ => {
                self.len = len;
                Ok(())
            }
        }
    }

    /// All events, sorted by frame; events of one frame are in the order they happen.
    pub fn events(&self) -> &[(u64, Event)] {
        &self.events
    }

    /// Appends an event. Frames must not decrease; the replay grows to include the frame.
    pub fn push(&mut self, frame: u64, event: Event) -> Result<(), Error> {
        if let Some((last, _)) = self.events.last()
            && frame < *last
        {
            return Err(Error::new(format!(
                "event on frame {frame} after one on frame {last}"
            )));
        }
        validate_event(&event, self.header.joystick.is_some())?;
        let end = frame_end(frame)?;
        self.events.push((frame, event));
        self.len = self.len.max(end);
        Ok(())
    }

    /// Every frame's input, in order.
    pub fn inputs(&self) -> impl Iterator<Item = FrameInput<'_>> {
        let mut state = InputState::default();
        let mut rest = self.events.as_slice();
        (0..self.len).map(move |frame| {
            let n = rest.iter().take_while(|(f, _)| *f == frame).count();
            let (events, tail) = rest.split_at(n);
            rest = tail;
            state.wheel = 0;
            for (_, e) in events {
                state.apply(e);
            }
            FrameInput {
                frame,
                events,
                state,
            }
        })
    }

    /// Parses the text form.
    pub fn parse(text: &str) -> Result<Replay, Error> {
        let mut header = ReplayHeader::default();
        let mut frames: Option<u64> = None;
        let mut events: Vec<(u64, Event, u64)> = Vec::new(); // (frame, event, line)
        let mut last_highscore_line = 0;
        for (n, raw) in text.lines().enumerate() {
            let line_no = n as u64 + 1;
            let at = |m: String| Error::at(line_no, m);
            let trimmed = raw.trim();
            // These end in a JSON string, which may contain `#`.
            let line = if trimmed.starts_with("file ") || trimmed.starts_with("highscore ") {
                trimmed
            } else {
                trimmed.split('#').next().unwrap_or_default().trim()
            };
            let tokens: Vec<&str> = line.split_whitespace().collect();
            match tokens.as_slice() {
                [] => {}
                ["seed", v] => {
                    header.seed = v
                        .parse()
                        .map_err(|_| at(format!("bad seed {v:?} (an i32)")))?
                }
                ["time", v] => {
                    header.time = v.parse().map_err(|_| at(format!("bad time {v:?}")))?
                }
                ["frames", v] => {
                    frames = Some(
                        v.parse()
                            .map_err(|_| at(format!("bad frame count {v:?}")))?,
                    )
                }
                ["room", name] => header.room = name.to_string(),
                ["settings", vals @ ..] => {
                    let v: [i32; 7] = ints(vals)
                        .and_then(|v| v.try_into().ok())
                        .ok_or_else(|| at("settings takes 7 integers".into()))?;
                    header.settings = Settings::from_values(&v);
                }
                ["keys", vals @ ..] => {
                    let v: [i32; 12] = ints(vals)
                        .and_then(|v| v.try_into().ok())
                        .ok_or_else(|| at("keys takes 12 integers".into()))?;
                    header.keys = KeyConfig::from_values(&v);
                }
                ["gamepad", vals @ ..] => {
                    header.gamepad = ints(vals)
                        .and_then(|v| GamepadConfig::from_values(&v))
                        .ok_or_else(|| at("gamepad takes 8 or 9 integers".into()))?;
                }
                ["joystick"] => header.joystick = Some(JoystickInfo { has_pov: false }),
                ["joystick", "pov"] => header.joystick = Some(JoystickInfo { has_pov: true }),
                ["highscore", v, ..] => {
                    // Only what GM 8's table can hold: finite scores, at most 10, best first. The oracle
                    // enforces the same limits.
                    let value = v
                        .parse::<f64>()
                        .ok()
                        .filter(|v| v.is_finite())
                        .ok_or_else(|| at(format!("bad highscore value {v:?}")))?;
                    let name: String = serde_json::from_str(after_token(line, 1)).map_err(|e| {
                        at(format!(
                            "highscore {v}: the name must be one JSON string: {e}"
                        ))
                    })?;
                    if header.highscores.len() == HIGHSCORE_ENTRIES {
                        return Err(at(format!(
                            "the highscore table holds only {HIGHSCORE_ENTRIES} entries"
                        )));
                    }
                    if header
                        .highscores
                        .last()
                        .is_some_and(|prev| value > prev.value)
                    {
                        return Err(at("highscore entries must be best first".into()));
                    }
                    header.highscores.push(Highscore { name, value });
                    last_highscore_line = line_no;
                }
                ["file", path, ..] => {
                    let contents: String =
                        serde_json::from_str(after_token(line, 1)).map_err(|e| {
                            at(format!(
                                "file {path}: contents must be one JSON string: {e}"
                            ))
                        })?;
                    if !is_game_path(path) {
                        return Err(at(format!(
                            "file {path}: not a relative path inside the game directory"
                        )));
                    }
                    if CONFIG_FILES.iter().any(|c| c.eq_ignore_ascii_case(path)) {
                        return Err(at(format!("file can't set {path}; use its directive")));
                    }
                    header.files.insert(path.to_string(), contents);
                }
                [f, verb, args @ ..] if f.bytes().all(|b| b.is_ascii_digit()) => {
                    let frame: u64 = f.parse().map_err(|_| at(format!("bad frame {f:?}")))?;
                    for (offset, event) in parse_event(verb, args).map_err(at)? {
                        let frame = frame
                            .checked_add(offset)
                            .ok_or_else(|| at("frame overflows".into()))?;
                        events.push((frame, event, line_no));
                    }
                }
                _ => return Err(at(format!("unrecognised line {line:?}"))),
            }
        }
        // A shorter table is filled up with the fresh table's entries, which score 0, so it stays sorted only if
        // its scores are all at least 0.
        if header.highscores.len() < HIGHSCORE_ENTRIES
            && header.highscores.last().is_some_and(|h| h.value < 0.0)
        {
            return Err(Error::at(
                last_highscore_line,
                "a highscore table with fewer than 10 entries can't have scores below 0",
            ));
        }
        // Like the oracle, keep each frame's events in file order.
        events.sort_by_key(|(f, _, _)| *f);
        let has_joystick = header.joystick.is_some();
        let mut replay = Replay::new(header);
        for (frame, event, line) in events {
            if let Some(len) = frames
                && frame >= len
            {
                return Err(Error::at(
                    line,
                    format!("event on frame {frame} beyond `frames {len}`"),
                ));
            }
            validate_event(&event, has_joystick).map_err(|e| Error::at(line, e.message))?;
            replay.events.push((frame, event));
        }
        replay.len = match (frames, replay.events.last()) {
            (Some(len), _) => len,
            (None, Some((f, _))) => frame_end(*f)?,
            (None, None) => 0,
        };
        Ok(replay)
    }

    /// The canonical text form. Extensions are written only when they differ from the defaults, so a replay
    /// that needs none of them stays in the oracle's subset.
    pub fn to_text(&self) -> String {
        let mut out = self.header.text(Some(self.len));
        for (frame, event) in &self.events {
            out.push_str(&event_line(*frame, event));
        }
        out
    }

    /// Frame `frame`'s events, in order (none past the end).
    pub fn frame_events(&self, frame: u64) -> impl Iterator<Item = Event> + '_ {
        let start = self.events.partition_point(|(f, _)| *f < frame);
        self.events[start..]
            .iter()
            .take_while(move |(f, _)| *f == frame)
            .map(|&(_, e)| e)
    }
}

impl ReplayHeader {
    /// The header's lines in [`Replay::to_text`]'s form, without `frames`: a recorder that streams its
    /// events writes this first and `frames N` last (directives may come in any order).
    pub fn to_text(&self) -> String {
        self.text(None)
    }

    fn text(&self, frames: Option<u64>) -> String {
        let h = self;
        let defaults = ReplayHeader::default();
        let mut out = format!("seed {}\n", h.seed);
        if h.time != DEFAULT_TIME {
            let _ = writeln!(out, "time {}", h.time);
        }
        if let Some(frames) = frames {
            let _ = writeln!(out, "frames {frames}");
        }
        if h.room != defaults.room {
            let _ = writeln!(out, "room {}", h.room);
        }
        let join = |v: Vec<i32>| v.iter().map(i32::to_string).collect::<Vec<_>>().join(" ");
        if h.settings != defaults.settings {
            let _ = writeln!(out, "settings {}", join(h.settings.values()));
        }
        if h.keys != defaults.keys {
            let _ = writeln!(out, "keys {}", join(h.keys.values()));
        }
        if h.gamepad != defaults.gamepad {
            let _ = writeln!(out, "gamepad {}", join(h.gamepad.values()));
        }
        match &h.joystick {
            Some(JoystickInfo { has_pov: true }) => out.push_str("joystick pov\n"),
            Some(JoystickInfo { has_pov: false }) => out.push_str("joystick\n"),
            None => {}
        }
        for hs in &h.highscores {
            let name = serde_json::to_string(&hs.name).expect("strings serialize");
            let _ = writeln!(out, "highscore {} {name}", format_real(hs.value));
        }
        for (path, contents) in &h.files {
            let _ = writeln!(
                out,
                "file {path} {}",
                serde_json::to_string(contents).expect("strings serialize")
            );
        }
        out
    }
}

/// One event's line in a replay's text form, newline included.
pub fn event_line(frame: u64, event: &Event) -> String {
    format!("{frame} {}\n", format_event(event))
}

/// The length of a replay whose last frame is `frame`.
fn frame_end(frame: u64) -> Result<u64, Error> {
    frame
        .checked_add(1)
        .ok_or_else(|| Error::new(format!("frame {frame} is past the last possible frame")))
}

/// The bit for joystick button `n`, or `None` outside 1-32.
fn joy_bit(n: u8) -> Option<u32> {
    (1..=32).contains(&n).then(|| 1 << (n - 1))
}

fn ints(vals: &[&str]) -> Option<Vec<i32>> {
    vals.iter().map(|v| v.parse().ok()).collect()
}

/// The rest of `line` after its first `n` whitespace-separated tokens and the directive, trimmed.
fn after_token(line: &str, n: usize) -> &str {
    let mut rest = line.trim_start();
    for _ in 0..=n {
        let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        rest = rest[end..].trim_start();
    }
    rest.trim_end()
}

/// A relative path that stays inside the game directory, so a runner can write it there safely:
/// `/`-separated, with no empty, `.` or `..` components, backslashes or drive letters.
fn is_game_path(path: &str) -> bool {
    !path.contains(['\\', ':'])
        && path
            .split('/')
            .all(|c| !c.is_empty() && c != "." && c != "..")
}

fn validate_event(event: &Event, has_joystick: bool) -> Result<(), Error> {
    let joystick = matches!(
        event,
        Event::Axis(..) | Event::Press(Button::Joy(_)) | Event::Release(Button::Joy(_))
    );
    if joystick && !has_joystick {
        return Err(Error::new("joystick input, but no `joystick` directive"));
    }
    match *event {
        Event::Press(Button::Key(vk)) | Event::Release(Button::Key(vk)) if vk < keys::MIN_KEY => {
            Err(Error::new(format!("key code {vk} is a pseudo-key")))
        }
        Event::Press(Button::Joy(n)) | Event::Release(Button::Joy(n)) if !(1..=32).contains(&n) => {
            Err(Error::new(format!("joystick button {n} is not 1-32")))
        }
        Event::Axis(_, v) if !v.is_finite() => Err(Error::new("axis values must be finite")),
        _ => Ok(()),
    }
}

fn button_name(b: Button) -> String {
    match b {
        Button::Key(vk) => keys::key_name(vk),
        Button::Mouse(MouseButton::Left) => "mb_left".into(),
        Button::Mouse(MouseButton::Right) => "mb_right".into(),
        Button::Mouse(MouseButton::Middle) => "mb_middle".into(),
        Button::Joy(n) => format!("joy{n}"),
    }
}

fn parse_button(s: &str) -> Result<Button, String> {
    let lower = s.to_ascii_lowercase();
    let b = match lower.as_str() {
        "mb_left" => Button::Mouse(MouseButton::Left),
        "mb_right" => Button::Mouse(MouseButton::Right),
        "mb_middle" => Button::Mouse(MouseButton::Middle),
        _ => match lower.strip_prefix("joy") {
            Some(n) => Button::Joy(
                n.parse::<u8>()
                    .ok()
                    .filter(|n| (1..=32).contains(n))
                    .ok_or_else(|| format!("bad joystick button {s:?} (joy1 to joy32)"))?,
            ),
            None => Button::Key(keys::parse_key(s).ok_or_else(|| format!("bad key {s:?}"))?),
        },
    };
    Ok(b)
}

fn format_event(e: &Event) -> String {
    match *e {
        Event::Press(b) => format!("press {}", button_name(b)),
        Event::Release(b) => format!("release {}", button_name(b)),
        Event::WheelUp => "wheel up".into(),
        Event::WheelDown => "wheel down".into(),
        Event::MouseMove { x, y } => format!("mouse {x} {y}"),
        Event::Axis(a, v) => format!("axis {} {}", a.name(), format_real(v)),
    }
}

/// Parses the part of an event line after the frame, into events at frame offsets (`hold` makes two).
fn parse_event(verb: &str, args: &[&str]) -> Result<Vec<(u64, Event)>, String> {
    Ok(match (verb, args) {
        ("press", [b]) => vec![(0, Event::Press(parse_button(b)?))],
        ("release", [b]) => vec![(0, Event::Release(parse_button(b)?))],
        ("hold", [b, n]) => {
            let b = parse_button(b)?;
            let n: u64 = n.parse().map_err(|_| format!("bad hold length {n:?}"))?;
            vec![(0, Event::Press(b)), (n, Event::Release(b))]
        }
        ("wheel", ["up"]) => vec![(0, Event::WheelUp)],
        ("wheel", ["down"]) => vec![(0, Event::WheelDown)],
        ("mouse", [x, y]) => {
            let x = x.parse().map_err(|_| format!("bad mouse x {x:?}"))?;
            let y = y.parse().map_err(|_| format!("bad mouse y {y:?}"))?;
            vec![(0, Event::MouseMove { x, y })]
        }
        ("axis", [a, v]) => {
            let axis = Axis::ALL
                .into_iter()
                .find(|ax| ax.name() == *a)
                .ok_or_else(|| format!("bad axis {a:?} (x, y, z, pov)"))?;
            let v = v
                .parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
                .ok_or_else(|| format!("bad axis value {v:?}"))?;
            vec![(0, Event::Axis(axis, v))]
        }
        _ => return Err(format!("unrecognised event `{verb} {}`", args.join(" "))),
    })
}

/// Builds a replay from per-frame input state, as an in-game recorder sees it.
pub struct Recorder {
    replay: Replay,
    previous: InputState,
}

impl Recorder {
    pub fn new(header: ReplayHeader) -> Self {
        Recorder {
            replay: Replay::new(header),
            previous: InputState::default(),
        }
    }

    /// Records the next frame's state as the events that lead to it from the previous frame: releases, then
    /// presses, then mouse movement, axes and the wheel.
    pub fn frame(&mut self, state: &InputState) -> Result<(), Error> {
        let frame = self.replay.len;
        for e in self.previous.events_to(state) {
            self.replay.push(frame, e)?;
        }
        self.replay.len = frame_end(frame)?;
        self.previous = *state;
        Ok(())
    }

    pub fn finish(self) -> Replay {
        self.replay
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One of the oracle's own replays.
    const ORACLE_START: &str = "seed 12345\nframes 1700\n1200 hold left 70\n1285 hold up 5\n";

    #[test]
    fn reads_the_oracle_replay() {
        let r = Replay::parse(ORACLE_START).unwrap();
        assert_eq!(r.header, ReplayHeader::new(12345));
        assert_eq!(r.len(), 1700);
        assert_eq!(
            r.events(),
            [
                (1200, Event::Press(Button::Key(37))),
                (1270, Event::Release(Button::Key(37))),
                (1285, Event::Press(Button::Key(38))),
                (1290, Event::Release(Button::Key(38))),
            ]
        );
        let inputs: Vec<_> = r.inputs().collect();
        assert_eq!(inputs.len(), 1700);
        assert!(!inputs[1199].state.keys.contains(37));
        assert!(inputs[1200].state.keys.contains(37) && inputs[1269].state.keys.contains(37));
        assert!(!inputs[1270].state.keys.contains(37));
        assert_eq!(
            inputs[1270].events,
            [(1270, Event::Release(Button::Key(37)))]
        );
        // Written back in the oracle's subset.
        assert_eq!(
            r.to_text(),
            "seed 12345\nframes 1700\n1200 press left\n1270 release left\n1285 press up\n1290 release up\n"
        );
    }

    #[test]
    fn oracle_grammar_details() {
        // Comments, `time`, numeric codes, aliases, `frames` defaulting to the last event + 1, events out of
        // order, and file order within a frame.
        let text =
            "# c\n\ntime 5\n30 press 13 # enter\n10 release ESC\n10 press return\n10 hold 5 0\n";
        let r = Replay::parse(text).unwrap();
        assert_eq!(r.header.time, 5);
        assert_eq!(r.header.seed, 0);
        assert_eq!(r.len(), 31);
        let k = |vk| Button::Key(vk);
        assert_eq!(
            r.events(),
            [
                (10, Event::Release(k(27))),
                (10, Event::Press(k(13))),
                (10, Event::Press(k(b'5'))),
                (10, Event::Release(k(b'5'))),
                (30, Event::Press(k(13))),
            ]
        );
        assert_eq!(r.to_text().lines().nth(1), Some("time 5"));
    }

    fn full() -> Replay {
        let mut header = ReplayHeader::new(-7);
        header.time = 99;
        header.room = "rLevel".into();
        header.settings.gamepad_on = 0;
        header.keys.jump = 32;
        // The shipped file has a start button; an eight-line one differs from the default.
        header.gamepad.start = None;
        header.joystick = Some(JoystickInfo { has_pov: true });
        header.highscores = vec![
            Highscore {
                name: "MONEY".into(),
                value: 9_000_123.5,
            },
            Highscore {
                name: "<no one> # \"x\"".into(),
                value: -0.0,
            },
        ];
        header
            .files
            .insert("levels/a.lvl".into(), "line # one\r\n\"q\"\n".into());
        let mut r = Replay::new(header);
        r.push(0, Event::Press(Button::Key(255))).unwrap();
        r.push(3, Event::Press(Button::Mouse(MouseButton::Middle)))
            .unwrap();
        r.push(3, Event::MouseMove { x: -4, y: 700 }).unwrap();
        r.push(4, Event::WheelDown).unwrap();
        r.push(4, Event::Press(Button::Joy(32))).unwrap();
        r.push(5, Event::Axis(Axis::X, -0.0)).unwrap();
        r.push(5, Event::Axis(Axis::Pov, 270.0)).unwrap();
        r.push(6, Event::Release(Button::Key(255))).unwrap();
        r.set_len(10).unwrap();
        r
    }

    #[test]
    fn text_round_trip_with_every_extension() {
        let r = full();
        let text = r.to_text();
        let back = Replay::parse(&text).unwrap();
        assert_eq!(back, r, "{text}");
        let Event::Axis(_, v) = back.events()[5].1 else {
            panic!()
        };
        assert_eq!(v.to_bits(), (-0.0f64).to_bits());
        assert!(text.contains("\n0 press 255\n"));
        assert!(text.contains(
            "\n3 press mb_middle\n3 mouse -4 700\n4 wheel down\n4 press joy32\n5 axis x -0\n"
        ));
    }

    #[test]
    fn derived_state() {
        let r = full();
        let inputs: Vec<_> = r.inputs().collect();
        assert_eq!(inputs.len(), 10);
        let s = inputs[4].state;
        assert!(
            s.is_down(Button::Key(255))
                && s.is_down(Button::Mouse(MouseButton::Middle))
                && s.is_down(Button::Joy(32))
        );
        assert_eq!((s.mouse_x, s.mouse_y, s.wheel), (-4, 700, -1));
        assert_eq!(inputs[5].state.wheel, 0);
        assert_eq!(inputs[5].state.axis(Axis::Pov), 270.0);
        assert!(!inputs[6].state.is_down(Button::Key(255)));
        assert!(inputs[9].events.is_empty());
    }

    #[test]
    fn recorder_produces_the_same_states() {
        let r = full();
        let mut rec = Recorder::new(r.header.clone());
        for input in r.inputs() {
            rec.frame(&input.state).unwrap();
        }
        let recorded = rec.finish();
        assert_eq!(recorded.len(), r.len());
        let a: Vec<_> = r.inputs().map(|i| i.state).collect();
        let b: Vec<_> = recorded.inputs().map(|i| i.state).collect();
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(&b) {
            assert_eq!(x.keys, y.keys);
            assert_eq!(
                (
                    x.mouse_buttons,
                    x.mouse_x,
                    x.mouse_y,
                    x.wheel,
                    x.joy_buttons
                ),
                (
                    y.mouse_buttons,
                    y.mouse_x,
                    y.mouse_y,
                    y.wheel,
                    y.joy_buttons
                )
            );
            assert_eq!(x.joy_axes.map(f64::to_bits), y.joy_axes.map(f64::to_bits));
        }
        // Idle frames cost nothing.
        let mut rec = Recorder::new(ReplayHeader::default());
        for _ in 0..1000 {
            rec.frame(&InputState::default()).unwrap();
        }
        assert_eq!(rec.finish().to_text(), "seed 0\nframes 1000\n");
    }

    #[test]
    fn game_files_match_the_shipped_ones() {
        let files = ReplayHeader::default().game_files();
        assert_eq!(files["settings.cfg"], "0\r\n1\r\n0\r\n1\r\n3\r\n15\r\n15");
        assert_eq!(
            files["keys.cfg"],
            "38\r\n40\r\n37\r\n39\r\n90\r\n88\r\n67\r\n16\r\n65\r\n83\r\n70\r\n80"
        );
        assert_eq!(
            files["gamepad.cfg"],
            "2\r\n1\r\n3\r\n5\r\n7\r\n8\r\n4\r\n6\r\n10"
        );
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn api_errors() {
        let mut r = Replay::new(ReplayHeader::default());
        r.push(5, Event::WheelUp).unwrap();
        assert!(r.push(4, Event::WheelUp).is_err());
        assert!(r.set_len(5).is_err());
        assert!(r.push(6, Event::Press(Button::Joy(1))).is_err());
        assert!(r.push(6, Event::Press(Button::Key(1))).is_err());
        assert!(r.push(u64::MAX, Event::WheelUp).is_err());
        assert_eq!(r.len(), 6);
        // The last frame that fits still works.
        let r = Replay::parse("18446744073709551614 press left").unwrap();
        assert_eq!(r.len(), u64::MAX);
    }

    #[test]
    fn out_of_range_joystick_buttons_never_panic() {
        let mut s = InputState::default();
        for n in [0, 33, 255] {
            s.apply(&Event::Press(Button::Joy(n)));
            s.apply(&Event::Release(Button::Joy(n)));
            assert!(!s.is_down(Button::Joy(n)));
        }
        assert_eq!(s.joy_buttons, 0);
        s.apply(&Event::Press(Button::Joy(32)));
        assert_eq!(s.joy_buttons, 1 << 31);
    }

    #[test]
    fn highscores_are_limited_to_what_gm_can_store() {
        let table = |values: &[f64]| ReplayHeader {
            highscores: values
                .iter()
                .map(|&value| Highscore {
                    name: "X".into(),
                    value,
                })
                .collect(),
            ..ReplayHeader::default()
        };
        let parse = |values: &[f64]| Replay::parse(&Replay::new(table(values)).to_text());

        // A full table may go below 0; ties are allowed.
        let full: Vec<f64> = (0..10).map(|i| 5.0 - f64::from(i)).collect();
        let back = parse(&full).unwrap();
        assert_eq!(back.header.highscores.len(), 10);
        assert!(parse(&[3.0, 3.0, 0.0]).is_ok());

        let cases: &[(&[f64], &str)] = &[
            (&[f64::NAN], "bad highscore value \"nan\""),
            (&[f64::INFINITY], "bad highscore value \"inf\""),
            (&[1.0, f64::NEG_INFINITY], "bad highscore value \"-inf\""),
            (&[1.0, 2.0], "must be best first"),
            (&[0.0; 11], "holds only 10 entries"),
            (
                &[1.0, -1.0],
                "fewer than 10 entries can't have scores below 0",
            ),
        ];
        for (values, expected) in cases {
            let err = parse(values).unwrap_err().to_string();
            assert!(err.contains(expected), "{values:?}: {err}");
        }
    }

    #[test]
    fn malformed_input_is_an_error() {
        let cases: &[(&str, &str)] = &[
            ("seed 2147483648", "line 1: bad seed"),
            ("seed", "unrecognised line"),
            ("seed 1 2", "unrecognised line"),
            ("time -1", "bad time"),
            ("frames x", "bad frame count"),
            (
                "frames 10\n10 press left",
                "line 2: event on frame 10 beyond `frames 10`",
            ),
            (
                "frames 10\n5 hold left 5",
                "line 2: event on frame 10 beyond",
            ),
            ("1 hold left", "unrecognised event"),
            ("1 hold left x", "bad hold length"),
            ("1 press", "unrecognised event"),
            ("1 press leftt", "bad key \"leftt\""),
            ("1 press 256", "bad key"),
            ("1 press 01", "bad key"),
            ("1 tap left", "unrecognised event"),
            ("-1 press left", "unrecognised line"),
            ("1.5 press left", "unrecognised line"),
            ("99999999999999999999 press left", "bad frame"),
            ("18446744073709551615 hold left 1", "frame overflows"),
            ("1 press joy1", "no `joystick` directive"),
            ("joystick\n1 press joy33", "bad joystick button"),
            ("joystick\n1 press joy0", "bad joystick button"),
            ("1 axis x 1", "no `joystick` directive"),
            ("joystick\n1 axis w 1", "bad axis"),
            ("joystick\n1 axis x nan", "bad axis value"),
            ("joystick sideways", "unrecognised line"),
            ("1 wheel left", "unrecognised event"),
            ("1 mouse 1", "unrecognised event"),
            ("1 mouse 1 y", "bad mouse y"),
            ("settings 1 2 3", "settings takes 7 integers"),
            ("keys 1 2 3 4 5 6 7 8 9 10 11 x", "keys takes 12 integers"),
            ("gamepad 1 2 3 4 5 6 7", "gamepad takes 8 or 9 integers"),
            (
                "gamepad 1 2 3 4 5 6 7 8 9 10",
                "gamepad takes 8 or 9 integers",
            ),
            ("highscore lots \"MONEY\"", "bad highscore value"),
            (
                "18446744073709551615 press left",
                "past the last possible frame",
            ),
            ("highscore 5 MONEY", "the name must be one JSON string"),
            ("highscore 5", "the name must be one JSON string"),
            ("file stats.txt unquoted", "must be one JSON string"),
            ("file stats.txt \"a\" \"b\"", "must be one JSON string"),
            ("file Settings.cfg \"1\"", "file can't set Settings.cfg"),
            ("file ./settings.cfg \"1\"", "not a relative path inside"),
            (
                "file levels/../keys.cfg \"1\"",
                "not a relative path inside",
            ),
            ("file ../../.bashrc \"x\"", "not a relative path inside"),
            ("file /tmp/x \"x\"", "not a relative path inside"),
            ("file levels//a.lvl \"x\"", "not a relative path inside"),
            ("file C:x \"x\"", "not a relative path inside"),
            ("file levels\\a.lvl \"x\"", "not a relative path inside"),
            ("press left", "unrecognised line"),
            ("hello", "unrecognised line"),
        ];
        for (text, want) in cases {
            let err = Replay::parse(text).expect_err(text).to_string();
            assert!(err.contains(want), "{text:?}: got {err:?}, want {want:?}");
        }
    }

    #[test]
    fn a_streamed_replay_parses_like_the_canonical_one() {
        let r = Replay::parse(ORACLE_START).unwrap();
        assert_eq!(
            r.frame_events(1200).collect::<Vec<_>>(),
            [Event::Press(Button::Key(37))]
        );
        assert_eq!(r.frame_events(1201).count(), 0);
        assert_eq!(r.frame_events(9999).count(), 0);
        // Header first, events as they happen, `frames` last.
        let mut streamed = r.header.to_text();
        assert!(!streamed.contains("frames"));
        for (f, e) in r.events() {
            streamed.push_str(&event_line(*f, e));
        }
        streamed.push_str(&format!("frames {}\n", r.len()));
        assert_eq!(Replay::parse(&streamed).unwrap(), r);
    }
}
