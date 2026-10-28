//! Real keyboards, mice and gamepads as GM8 input: each tick's raw events in the replay format
//! (`trace::replay`, `docs/formats.md`), which is all the game sees and all a recording needs.
//!
//! Events reach the game at the 30 Hz tick boundary, in the order they happened, so a key or pad button
//! pressed and released between two ticks still reports both edges to that tick (`gm::input`). Keyboard
//! auto-repeat is passed on as repeated presses, which GM8 reports as pressed again (`docs/quirks.md`).
//!
//! Keys get the virtual-key code the keyboard layout gives them, as GM8 got from Windows: the key labelled
//! `Z` is `Z` on every layout ([`KeyMap::Layout`]). [`KeyMap::Physical`] names keys by their place on a US
//! keyboard instead.

use std::collections::HashMap;
use trace::keys::KeySet;
use trace::replay::{Axis, Button, Event, InputState, JoystickInfo, MouseButton};
use winit::keyboard::{Key, KeyCode};

pub const VK_SHIFT: u8 = 16;
pub const VK_CONTROL: u8 = 17;
pub const VK_ALT: u8 = 18;
pub const VK_LEFT: u8 = 37;
pub const VK_UP: u8 = 38;
pub const VK_RIGHT: u8 = 39;
pub const VK_DOWN: u8 = 40;

/// The Windows virtual-key code for a physical key on a US layout, or `None` for keys Windows has no code
/// for.
///
/// Both shift, control and alt keys are `vk_shift`, `vk_control` and `vk_alt`, as in the `WM_KEYDOWN`
/// messages GM8 handled; the numpad's enter is `vk_enter`.
pub fn vk(key: KeyCode) -> Option<u8> {
    use KeyCode::*;
    Some(match key {
        Backspace => 8,
        Tab => 9,
        Enter | NumpadEnter => 13,
        ShiftLeft | ShiftRight => VK_SHIFT,
        ControlLeft | ControlRight => VK_CONTROL,
        AltLeft | AltRight => VK_ALT,
        Pause => 19,
        CapsLock => 20,
        Escape => 27,
        Space => 32,
        PageUp => 33,
        PageDown => 34,
        End => 35,
        Home => 36,
        ArrowLeft => VK_LEFT,
        ArrowUp => VK_UP,
        ArrowRight => VK_RIGHT,
        ArrowDown => VK_DOWN,
        PrintScreen => 44,
        Insert => 45,
        Delete => 46,
        Digit0 => b'0',
        Digit1 => b'1',
        Digit2 => b'2',
        Digit3 => b'3',
        Digit4 => b'4',
        Digit5 => b'5',
        Digit6 => b'6',
        Digit7 => b'7',
        Digit8 => b'8',
        Digit9 => b'9',
        KeyA => b'A',
        KeyB => b'B',
        KeyC => b'C',
        KeyD => b'D',
        KeyE => b'E',
        KeyF => b'F',
        KeyG => b'G',
        KeyH => b'H',
        KeyI => b'I',
        KeyJ => b'J',
        KeyK => b'K',
        KeyL => b'L',
        KeyM => b'M',
        KeyN => b'N',
        KeyO => b'O',
        KeyP => b'P',
        KeyQ => b'Q',
        KeyR => b'R',
        KeyS => b'S',
        KeyT => b'T',
        KeyU => b'U',
        KeyV => b'V',
        KeyW => b'W',
        KeyX => b'X',
        KeyY => b'Y',
        KeyZ => b'Z',
        SuperLeft => 91,
        SuperRight => 92,
        ContextMenu => 93,
        Numpad0 => 96,
        Numpad1 => 97,
        Numpad2 => 98,
        Numpad3 => 99,
        Numpad4 => 100,
        Numpad5 => 101,
        Numpad6 => 102,
        Numpad7 => 103,
        Numpad8 => 104,
        Numpad9 => 105,
        NumpadMultiply => 106,
        NumpadAdd => 107,
        NumpadComma => 108,
        NumpadSubtract => 109,
        NumpadDecimal => 110,
        NumpadDivide => 111,
        F1 => 112,
        F2 => 113,
        F3 => 114,
        F4 => 115,
        F5 => 116,
        F6 => 117,
        F7 => 118,
        F8 => 119,
        F9 => 120,
        F10 => 121,
        F11 => 122,
        F12 => 123,
        F13 => 124,
        F14 => 125,
        F15 => 126,
        F16 => 127,
        F17 => 128,
        F18 => 129,
        F19 => 130,
        F20 => 131,
        F21 => 132,
        F22 => 133,
        F23 => 134,
        F24 => 135,
        NumLock => 144,
        ScrollLock => 145,
        Semicolon => 186,
        Equal => 187,
        Comma => 188,
        Minus => 189,
        Period => 190,
        Slash => 191,
        Backquote => 192,
        BracketLeft => 219,
        Backslash => 220,
        BracketRight => 221,
        Quote => 222,
        IntlBackslash => 226,
        _ => return None,
    })
}

/// The virtual-key code Windows gives the key that types `c` (unshifted): letters and digits are
/// themselves, and the US punctuation keys have their `VK_OEM_*` codes. `None` for anything else.
pub fn vk_for_char(c: char) -> Option<u8> {
    let c = c.to_ascii_uppercase();
    Some(match c {
        'A'..='Z' | '0'..='9' => c as u8,
        ';' => 186,
        '=' => 187,
        ',' => 188,
        '-' => 189,
        '.' => 190,
        '/' => 191,
        '`' => 192,
        '[' => 219,
        '\\' => 220,
        ']' => 221,
        '\'' => 222,
        _ => return None,
    })
}

/// How keys get their virtual-key codes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KeyMap {
    /// As GM8 on Windows: from what the layout types on the key (unshifted), falling back to the physical
    /// key for keys that type nothing a US keyboard has (named keys, other scripts, dead keys).
    #[default]
    Layout,
    /// From the key's place on a US keyboard, whatever the layout.
    Physical,
}

impl KeyMap {
    /// The code for a key. `typed` is what the layout types on it without modifiers.
    pub fn vk(self, physical: KeyCode, typed: &Key) -> Option<u8> {
        if self == KeyMap::Layout
            && let Key::Character(s) = typed
        {
            let mut chars = s.chars();
            if let (Some(c), None) = (chars.next(), chars.next())
                && let Some(code) = vk_for_char(c)
            {
                return Some(code);
            }
        }
        vk(physical)
    }
}

/// The joystick a session with a gamepad declares (the replay's `joystick pov`): the d-pad is a POV hat.
pub const PAD_JOYSTICK: JoystickInfo = JoystickInfo { has_pov: true };

/// Axis values are kept to hundredths, so a resting stick's noise doesn't fill a recording with events
/// and every value survives the replay's text form exactly. Never -0: it would differ from 0 for the change
/// check and print as `-0`.
pub fn quantize_axis(v: f32) -> f64 {
    (f64::from(v.clamp(-1.0, 1.0)) * 100.0).round() / 100.0 + 0.0
}

/// The POV angle for a d-pad's directions, -1 when none (or opposing ones) are held.
pub fn pov(up: bool, down: bool, left: bool, right: bool) -> f64 {
    let dy = i32::from(down) - i32::from(up);
    let dx = i32::from(right) - i32::from(left);
    match (dx, dy) {
        (0, -1) => 0.0,
        (1, -1) => 45.0,
        (1, 0) => 90.0,
        (1, 1) => 135.0,
        (0, 1) => 180.0,
        (-1, 1) => 225.0,
        (-1, 0) => 270.0,
        (-1, -1) => 315.0,
        _ => -1.0,
    }
}

/// Collects device events between ticks and hands each tick its events.
#[derive(Debug, Default)]
pub struct Collector {
    map: KeyMap,
    /// Whether the session has a joystick; without one, pad input is dropped.
    joystick: bool,
    events: Vec<Event>,
    /// The input as the game knows it after every event handed over or queued so far.
    sent: InputState,
    /// The code each physical key went down with, so its release matches it.
    down: HashMap<KeyCode, u8>,
    /// Sampled at the tick, as GM8 sampled them every frame: the cursor in GM window pixels, the sticks
    /// and triggers (x, y, z; the POV is an event of its own), and whether the wheel turned up, and down.
    cursor: Option<(i32, i32)>,
    sticks: [f64; 3],
    wheel_up: bool,
    wheel_down: bool,
}

impl Collector {
    pub fn new(map: KeyMap) -> Self {
        Collector {
            map,
            ..Collector::default()
        }
    }

    /// Takes over mid-game from another source of input (a replay) that left the game knowing `state`.
    /// No device is holding what that holds, so the first tick releases it, and centres the pad's axes
    /// if the session has a joystick: the game doesn't run on with a key the reader never pressed.
    pub fn taking_over(map: KeyMap, joystick: bool, state: InputState) -> Self {
        let mut c = Collector {
            map,
            joystick,
            sent: InputState {
                wheel_up: false,
                wheel_down: false,
                ..state
            },
            ..Collector::default()
        };
        c.focus_lost();
        c.pad_lost();
        c
    }

    /// Declares (or not) the joystick that pad input goes to. Set before the first tick.
    pub fn set_joystick(&mut self, joystick: bool) {
        self.joystick = joystick;
    }

    fn push(&mut self, e: Event) {
        self.sent.apply(&e);
        self.events.push(e);
    }

    /// A key goes down (again, on auto-repeat) or up. `typed` is what the layout types on it without
    /// modifiers.
    ///
    /// The two shift (control, alt) keys share a code, as in GM8's `WM_KEYDOWN`/`WM_KEYUP`: releasing
    /// either releases `vk_shift` even while the other is held.
    pub fn key(&mut self, physical: KeyCode, typed: &Key, down: bool) {
        if down {
            let code = match self.down.get(&physical) {
                Some(&code) => code,
                None => match self.map.vk(physical, typed) {
                    Some(code) => code,
                    None => return,
                },
            };
            self.down.insert(physical, code);
            self.push(Event::Press(Button::Key(code)));
        } else if let Some(code) = self.down.remove(&physical)
            && self.sent.is_down(Button::Key(code))
        {
            self.push(Event::Release(Button::Key(code)));
        }
    }

    pub fn mouse_button(&mut self, b: MouseButton, down: bool) {
        let b = Button::Mouse(b);
        if down {
            self.push(Event::Press(b));
        } else if self.sent.is_down(b) {
            self.push(Event::Release(b));
        }
    }

    /// The wheel turns by `dy` (up positive, in any unit). GM8 keeps a flag per direction per frame, so a
    /// tick gets at most one event each way, not one per (trackpad-sized) step, and both if it turned both
    /// ways.
    pub fn wheel(&mut self, dy: f64) {
        self.wheel_up |= dy > 0.0;
        self.wheel_down |= dy < 0.0;
    }

    /// The cursor moves, in GM window pixels.
    pub fn cursor(&mut self, x: i32, y: i32) {
        self.cursor = Some((x, y));
    }

    /// The window loses focus: every key and mouse button goes up, as the window won't hear those
    /// releases.
    pub fn focus_lost(&mut self) {
        let up = InputState {
            keys: KeySet::new(),
            mouse_buttons: 0,
            ..self.sent
        };
        for e in self.sent.events_to(&up) {
            self.push(e);
        }
        self.down.clear();
    }

    /// Pad button `n` (1-32) goes down or up.
    pub fn joy_button(&mut self, n: u8, down: bool) {
        let b = Button::Joy(n);
        if self.joystick && down != self.sent.is_down(b) {
            self.push(if down {
                Event::Press(b)
            } else {
                Event::Release(b)
            });
        }
    }

    /// The POV hat moves (degrees, -1 centred). Like a button, every change is an event, so a d-pad tap
    /// between ticks is recorded.
    pub fn joy_pov(&mut self, v: f64) {
        if self.joystick && v != self.sent.axis(Axis::Pov) {
            self.push(Event::Axis(Axis::Pov, v));
        }
    }

    /// A stick or trigger axis (x, y or z) moves.
    pub fn joy_axis(&mut self, axis: Axis, v: f64) {
        if axis != Axis::Pov {
            self.sticks[axis as usize] = v;
        }
    }

    /// The pad goes away: its buttons go up and its axes centre.
    pub fn pad_lost(&mut self) {
        for n in 1..=32 {
            self.joy_button(n, false);
        }
        self.joy_pov(-1.0);
        self.sticks = [0.0; 3];
    }

    /// The events for the tick about to run: the devices' in order, then what's sampled (cursor, sticks,
    /// wheel) where it changed, in `trace::replay`'s order.
    pub fn take_tick(&mut self) -> Vec<Event> {
        let mut next = self.sent;
        if let Some((x, y)) = self.cursor {
            (next.mouse_x, next.mouse_y) = (x, y);
        }
        if self.joystick {
            next.joy_axes[..3].copy_from_slice(&self.sticks);
        }
        next.wheel_up = std::mem::take(&mut self.wheel_up);
        next.wheel_down = std::mem::take(&mut self.wheel_down);
        for e in self.sent.events_to(&next) {
            self.push(e);
        }
        // The wheel lasts one frame.
        (self.sent.wheel_up, self.sent.wheel_down) = (false, false);
        std::mem::take(&mut self.events)
    }
}

#[cfg(feature = "gamepad")]
pub use gamepad::Gamepads;

/// Without the `gamepad` feature there are never any gamepads.
#[cfg(not(feature = "gamepad"))]
pub struct Gamepads(());

#[cfg(not(feature = "gamepad"))]
impl Gamepads {
    pub fn new() -> Option<Gamepads> {
        None
    }

    pub fn connected(&mut self) -> bool {
        false
    }

    pub fn drain(&mut self, _: &mut Collector) {}
}

#[cfg(feature = "gamepad")]
mod gamepad {
    use super::{Collector, pov, quantize_axis};
    use gilrs::{Axis, Button, EventType, GamepadId, Gilrs};

    /// GM button numbers for the pad's buttons, in order, as winmm numbers an XInput pad's: 1 A, 2 B, 3 X,
    /// 4 Y, 5 LB, 6 RB, 7 back, 8 start, 9 and 10 the sticks.
    const BUTTONS: [Button; 10] = [
        Button::South,
        Button::East,
        Button::West,
        Button::North,
        Button::LeftTrigger,
        Button::RightTrigger,
        Button::Select,
        Button::Start,
        Button::LeftThumb,
        Button::RightThumb,
    ];

    /// The d-pad's directions, in [`pov`]'s argument order.
    const DPAD: [Button; 4] = [
        Button::DPadUp,
        Button::DPadDown,
        Button::DPadLeft,
        Button::DPadRight,
    ];

    /// The gamepads, read as GM's joystick 1: the first one connected, until it goes away. Its x and y are
    /// the left stick (y down), z the left trigger minus the right (`config.exe`'s "LT (XB)" is z > 0.1),
    /// and the d-pad the POV hat.
    pub struct Gamepads {
        gilrs: Box<Gilrs>,
        active: Option<GamepadId>,
        dpad: [bool; 4],
        /// Left and right trigger, 0 to 1.
        triggers: [f32; 2],
    }

    impl Gamepads {
        /// `None` when the platform has no gamepad support.
        pub fn new() -> Option<Gamepads> {
            let gilrs = Gilrs::new().ok()?;
            let active = gilrs.gamepads().next().map(|(id, _)| id);
            Some(Gamepads {
                gilrs: Box::new(gilrs),
                active,
                dpad: [false; 4],
                triggers: [0.0; 2],
            })
        }

        /// Whether a pad is connected, as far as the platform has reported by now. Doesn't wait.
        pub fn connected(&mut self) -> bool {
            self.drain(&mut Collector::default());
            self.active.is_some()
        }

        /// Hands the pad's events since the last call to `c`, in order.
        pub fn drain(&mut self, c: &mut Collector) {
            while let Some(ev) = self.gilrs.next_event() {
                if self.active.is_none() && !matches!(ev.event, EventType::Disconnected) {
                    self.active = Some(ev.id);
                }
                if self.active != Some(ev.id) {
                    continue;
                }
                match ev.event {
                    EventType::ButtonPressed(b, _) => self.button(c, b, true),
                    EventType::ButtonReleased(b, _) => self.button(c, b, false),
                    EventType::ButtonChanged(b, v, _) => {
                        let i = match b {
                            Button::LeftTrigger2 => 0,
                            Button::RightTrigger2 => 1,
                            _ => continue,
                        };
                        self.triggers[i] = v;
                        let z = self.triggers[0] - self.triggers[1];
                        c.joy_axis(trace::replay::Axis::Z, quantize_axis(z));
                    }
                    EventType::AxisChanged(axis, v, _) => match axis {
                        Axis::LeftStickX => c.joy_axis(trace::replay::Axis::X, quantize_axis(v)),
                        // gilrs's y grows upward, winmm's downward.
                        Axis::LeftStickY => c.joy_axis(trace::replay::Axis::Y, quantize_axis(-v)),
                        // Some platforms report the d-pad as axes rather than buttons.
                        Axis::DPadX => {
                            self.dpad[2] = v < -0.5;
                            self.dpad[3] = v > 0.5;
                            self.pov(c);
                        }
                        Axis::DPadY => {
                            self.dpad[0] = v > 0.5;
                            self.dpad[1] = v < -0.5;
                            self.pov(c);
                        }
                        _ => {}
                    },
                    EventType::Disconnected => {
                        self.active = None;
                        self.dpad = [false; 4];
                        self.triggers = [0.0; 2];
                        c.pad_lost();
                    }
                    _ => {}
                }
            }
        }

        fn button(&mut self, c: &mut Collector, b: Button, down: bool) {
            if let Some(i) = BUTTONS.iter().position(|&x| x == b) {
                c.joy_button(i as u8 + 1, down);
            } else if let Some(i) = DPAD.iter().position(|&x| x == b) {
                self.dpad[i] = down;
                self.pov(c);
            }
        }

        fn pov(&self, c: &mut Collector) {
            let [up, down, left, right] = self.dpad;
            c.joy_pov(pov(up, down, left, right));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every key the game reads: its fixed keys (`vk_*` constants, keypress events, `scrGetChar`'s
    /// period and slash) and every key `scrGetKey` can name, which is what `config.exe` binds.
    #[test]
    fn every_key_the_game_reads_has_its_vk_code() {
        use KeyCode::*;
        let fixed = [
            (ArrowLeft, 37),
            (ArrowUp, 38),
            (ArrowRight, 39),
            (ArrowDown, 40),
            (ShiftLeft, 16),
            (ShiftRight, 16),
            (ControlLeft, 17),
            (ControlRight, 17),
            (AltLeft, 18),
            (AltRight, 18),
            (Space, 32),
            (Enter, 13),
            (NumpadEnter, 13),
            (Escape, 27),
            (Backspace, 8),
            (Delete, 46),
            (Period, 190),
            (Slash, 191),
            (F4, 115),
        ];
        for (key, code) in fixed {
            assert_eq!(vk(key), Some(code), "{key:?}");
        }
        let letters = [
            KeyA, KeyB, KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI, KeyJ, KeyK, KeyL, KeyM, KeyN,
            KeyO, KeyP, KeyQ, KeyR, KeyS, KeyT, KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ,
        ];
        for (key, code) in letters.into_iter().zip(b'A'..=b'Z') {
            assert_eq!(vk(key), Some(code), "{key:?}");
            let typed = Key::Character((code as char).to_ascii_lowercase().to_string().into());
            assert_eq!(KeyMap::Layout.vk(key, &typed), Some(code), "{key:?}");
        }
        let digits = [
            Digit0, Digit1, Digit2, Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9,
        ];
        for (key, code) in digits.into_iter().zip(b'0'..=b'9') {
            assert_eq!(vk(key), Some(code), "{key:?}");
        }
        assert_eq!(vk_for_char('.'), Some(190));
        assert_eq!(vk_for_char('/'), Some(191));
        // The shipped keys.cfg, in replay terms.
        let defaults = trace::replay::KeyConfig::default().values();
        let mapped: Vec<i32> = [
            ArrowUp, ArrowDown, ArrowLeft, ArrowRight, KeyZ, KeyX, KeyC, ShiftLeft, KeyA, KeyS,
            KeyF, KeyP,
        ]
        .into_iter()
        .map(|k| i32::from(vk(k).unwrap()))
        .collect();
        assert_eq!(mapped, defaults);
        // Nothing maps to GM's pseudo-keys, which replays reject.
        assert_eq!(vk(Fn), None);
    }

    #[test]
    fn keys_follow_the_layout_unless_mapped_physically() {
        let typed = |s: &str| Key::Character(s.into());
        // AZERTY: the US `Q` key types `a`, the US `W` key `z`.
        assert_eq!(KeyMap::Layout.vk(KeyCode::KeyQ, &typed("a")), Some(b'A'));
        assert_eq!(KeyMap::Layout.vk(KeyCode::KeyW, &typed("z")), Some(b'Z'));
        assert_eq!(KeyMap::Physical.vk(KeyCode::KeyW, &typed("z")), Some(b'W'));
        // Keys that type nothing a US keyboard has fall back to their place: AZERTY's digit row, Cyrillic.
        assert_eq!(KeyMap::Layout.vk(KeyCode::Digit1, &typed("&")), Some(b'1'));
        assert_eq!(KeyMap::Layout.vk(KeyCode::KeyF, &typed("а")), Some(b'F'));
        let named = Key::Named(winit::keyboard::NamedKey::ArrowLeft);
        assert_eq!(KeyMap::Layout.vk(KeyCode::ArrowLeft, &named), Some(VK_LEFT));

        // A release matches its press even if the layout changed in between.
        let mut c = Collector::new(KeyMap::Layout);
        c.key(KeyCode::KeyQ, &typed("a"), true);
        c.key(KeyCode::KeyQ, &typed("q"), false);
        let a = Button::Key(b'A');
        assert_eq!(c.take_tick(), [Event::Press(a), Event::Release(a)]);
    }

    #[test]
    fn a_tap_between_ticks_reaches_the_next_tick_in_order() {
        let typed = |s: &str| Key::Character(s.into());
        let mut c = Collector::new(KeyMap::Layout);
        c.key(KeyCode::KeyZ, &typed("z"), true);
        c.key(KeyCode::KeyZ, &typed("z"), false);
        let left = Key::Named(winit::keyboard::NamedKey::ArrowLeft);
        c.key(KeyCode::ArrowLeft, &left, true);
        c.key(KeyCode::ArrowLeft, &left, true); // auto-repeat
        let z = Button::Key(b'Z');
        let left = Button::Key(VK_LEFT);
        assert_eq!(
            c.take_tick(),
            [
                Event::Press(z),
                Event::Release(z),
                Event::Press(left),
                Event::Press(left)
            ]
        );
        assert_eq!(c.take_tick(), []);
        // A release the game never saw pressed is dropped; focus loss releases what's held.
        c.key(KeyCode::KeyX, &typed("x"), false);
        c.mouse_button(MouseButton::Left, true);
        c.focus_lost();
        assert_eq!(
            c.take_tick(),
            [
                Event::Press(Button::Mouse(MouseButton::Left)),
                Event::Release(left),
                Event::Release(Button::Mouse(MouseButton::Left)),
            ]
        );
    }

    #[test]
    fn sampled_input_reports_changes_once_per_tick() {
        let mut c = Collector::new(KeyMap::Layout);
        c.wheel(0.5);
        c.wheel(0.75);
        c.wheel(-0.25);
        c.cursor(1, 2);
        c.cursor(3, 4);
        // Without a joystick, pad input is dropped.
        c.joy_button(1, true);
        c.joy_axis(Axis::X, 1.0);
        assert_eq!(
            c.take_tick(),
            [
                Event::MouseMove { x: 3, y: 4 },
                Event::WheelUp,
                Event::WheelDown
            ]
        );
        assert_eq!(c.take_tick(), []);

        c.set_joystick(true);
        // A pad tap between ticks reports both edges; the stick only where it ends up.
        c.joy_button(3, true);
        c.joy_button(3, false);
        c.joy_pov(90.0);
        c.joy_axis(Axis::X, 0.25);
        c.joy_axis(Axis::X, 0.5);
        assert_eq!(
            c.take_tick(),
            [
                Event::Press(Button::Joy(3)),
                Event::Release(Button::Joy(3)),
                Event::Axis(Axis::Pov, 90.0),
                Event::Axis(Axis::X, 0.5),
            ]
        );
        c.joy_button(1, true);
        c.pad_lost();
        assert_eq!(
            c.take_tick(),
            [
                Event::Press(Button::Joy(1)),
                Event::Release(Button::Joy(1)),
                Event::Axis(Axis::Pov, -1.0),
                Event::Axis(Axis::X, 0.0),
            ]
        );
    }

    #[test]
    fn taking_over_releases_what_the_replay_held() {
        let mut state = InputState::keys([VK_RIGHT, b'Z']);
        for e in [
            Event::Press(Button::Mouse(MouseButton::Left)),
            Event::MouseMove { x: 5, y: 6 },
            Event::WheelUp,
        ] {
            state.apply(&e);
        }
        let mut c = Collector::taking_over(KeyMap::Layout, false, state);
        // The held keys and button go up; the mouse stays where it was, and last frame's wheel is gone.
        assert_eq!(
            c.take_tick(),
            [
                Event::Release(Button::Key(VK_RIGHT)),
                Event::Release(Button::Key(b'Z')),
                Event::Release(Button::Mouse(MouseButton::Left)),
            ]
        );
        assert_eq!(c.take_tick(), []);

        let mut state = InputState::default();
        state.apply(&Event::Press(Button::Joy(2)));
        state.apply(&Event::Axis(Axis::Y, 1.0));
        let mut c = Collector::taking_over(KeyMap::Layout, true, state);
        assert_eq!(
            c.take_tick(),
            [Event::Release(Button::Joy(2)), Event::Axis(Axis::Y, 0.0),]
        );
    }

    #[test]
    fn pad_helpers() {
        assert_eq!(pov(true, false, false, true), 45.0);
        assert_eq!(pov(false, true, true, false), 225.0);
        assert_eq!(pov(true, true, false, false), -1.0);
        assert_eq!(quantize_axis(0.123), 0.12);
        assert_eq!(quantize_axis(-0.003).to_bits(), 0.0f64.to_bits());
        assert_eq!(quantize_axis(-7.0), -1.0);
    }
}
