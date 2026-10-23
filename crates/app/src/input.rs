//! Real keyboards, mice and gamepads as GM8 input: each tick's raw events in the replay format
//! (`trace::replay`, `docs/formats.md`), which is all the game sees and all a recording needs.
//!
//! Events reach the game at the 30 Hz tick boundary, in the order they happened, so a key pressed and
//! released between two ticks still reports both to that tick (`gm::input`). Keyboard auto-repeat is passed
//! on as repeated presses, which GM8 reports as pressed again (`docs/quirks.md`).
//!
//! Keys are physical keys, named by their place on a US layout: the key where `Z` is on a US keyboard is
//! `Z` everywhere, so the default bindings sit in the same place on every layout. (GM8 got the layout's
//! virtual key instead; positions are the more useful choice for a remake, and replays don't care.)

use trace::keys::KeySet;
use trace::replay::{Axis, Button, Event, JoystickInfo, MouseButton};
use winit::keyboard::KeyCode;

/// The Windows virtual-key code GM8 sees for a physical key, or `None` for keys Windows has no code for.
///
/// Both shift, control and alt keys are `vk_shift`, `vk_control` and `vk_alt` (16, 17, 18), as in the
/// `WM_KEYDOWN` messages GM8 handled; the numpad's enter is `vk_enter`.
pub fn vk(key: KeyCode) -> Option<u8> {
    use KeyCode::*;
    Some(match key {
        Backspace => 8,
        Tab => 9,
        Enter | NumpadEnter => 13,
        ShiftLeft | ShiftRight => 16,
        ControlLeft | ControlRight => 17,
        AltLeft | AltRight => 18,
        Pause => 19,
        CapsLock => 20,
        Escape => 27,
        Space => 32,
        PageUp => 33,
        PageDown => 34,
        End => 35,
        Home => 36,
        ArrowLeft => 37,
        ArrowUp => 38,
        ArrowRight => 39,
        ArrowDown => 40,
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

/// A gamepad's state as GM8's winmm joystick 1 reports an XInput pad: the layout `config.exe`'s
/// "LT (XB)"/"RT (XB)" trigger bindings assume.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PadState {
    /// Bit `b - 1` is GM button `b`: 1 A, 2 B, 3 X, 4 Y, 5 LB, 6 RB, 7 back, 8 start, 9 and 10 the sticks.
    pub buttons: u32,
    /// x and y (left stick, y down), z (left trigger minus right trigger) and the POV hat (the d-pad, in
    /// degrees clockwise from up, -1 centred), indexed like [`Axis`].
    pub axes: [f64; 4],
}

impl Default for PadState {
    fn default() -> Self {
        PadState {
            buttons: 0,
            axes: [0.0, 0.0, 0.0, -1.0],
        }
    }
}

/// The joystick a session with a gamepad declares (the replay's `joystick pov`): the d-pad is a POV hat.
pub const PAD_JOYSTICK: JoystickInfo = JoystickInfo { has_pov: true };

/// Axis values are kept to hundredths, so a resting stick's noise doesn't fill a recording with events
/// and every value survives the replay's text form exactly.
pub fn quantize_axis(v: f32) -> f64 {
    (f64::from(v.clamp(-1.0, 1.0)) * 100.0).round() / 100.0
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

/// Collects window events between ticks and hands each tick its events.
#[derive(Debug, Default)]
pub struct Collector {
    events: Vec<Event>,
    /// Keys and mouse buttons down as far as the game knows, so focus loss can release them and a release
    /// without a press (the key went down before the window had focus) is dropped.
    keys: KeySet,
    mouse_buttons: u8,
    /// The cursor's latest position in GM window pixels, and the last one the game was told.
    mouse: Option<(i32, i32)>,
    sent_mouse: Option<(i32, i32)>,
    /// The pad state the game was last told, when a joystick is connected.
    pad: Option<PadState>,
}

impl Collector {
    /// `joystick`: whether this session declares a joystick (and so may send its events).
    pub fn new(joystick: bool) -> Self {
        Collector {
            pad: joystick.then(PadState::default),
            ..Collector::default()
        }
    }

    /// A key goes down (again, on auto-repeat) or up.
    pub fn key(&mut self, key: KeyCode, down: bool) {
        let Some(vk) = vk(key) else { return };
        if down {
            self.keys.insert(vk);
            self.events.push(Event::Press(Button::Key(vk)));
        } else if self.keys.contains(vk) {
            self.keys.remove(vk);
            self.events.push(Event::Release(Button::Key(vk)));
        }
    }

    pub fn mouse_button(&mut self, b: MouseButton, down: bool) {
        let bit = 1 << (b as u8 - 1);
        if down {
            self.mouse_buttons |= bit;
            self.events.push(Event::Press(Button::Mouse(b)));
        } else if self.mouse_buttons & bit != 0 {
            self.mouse_buttons &= !bit;
            self.events.push(Event::Release(Button::Mouse(b)));
        }
    }

    pub fn wheel(&mut self, up: bool) {
        self.events
            .push(if up { Event::WheelUp } else { Event::WheelDown });
    }

    /// The cursor moves, in GM window pixels. Only where it is at the tick matters to the game.
    pub fn cursor(&mut self, x: i32, y: i32) {
        self.mouse = Some((x, y));
    }

    /// The window loses focus: everything held goes up, as the window won't hear those releases.
    pub fn focus_lost(&mut self) {
        let keys: Vec<u8> = self.keys.iter().collect();
        for vk in keys {
            self.events.push(Event::Release(Button::Key(vk)));
        }
        self.keys = KeySet::new();
        for b in [MouseButton::Left, MouseButton::Right, MouseButton::Middle] {
            self.mouse_button(b, false);
        }
    }

    /// The events for the tick about to run: the window's in order, then the cursor if it moved, then the
    /// gamepad's changes since the previous tick (`pad` is its state now; ignored without a joystick).
    pub fn take_tick(&mut self, pad: Option<&PadState>) -> Vec<Event> {
        let mut events = std::mem::take(&mut self.events);
        if self.mouse != self.sent_mouse
            && let Some((x, y)) = self.mouse
        {
            events.push(Event::MouseMove { x, y });
            self.sent_mouse = self.mouse;
        }
        if let Some(prev) = &mut self.pad {
            let now = pad.copied().unwrap_or_default();
            for n in 1..=32u8 {
                let bit = 1u32 << (n - 1);
                let (was, is) = (prev.buttons & bit != 0, now.buttons & bit != 0);
                if was && !is {
                    events.push(Event::Release(Button::Joy(n)));
                } else if is && !was {
                    events.push(Event::Press(Button::Joy(n)));
                }
            }
            for axis in [Axis::X, Axis::Y, Axis::Z, Axis::Pov] {
                let v = now.axes[axis as usize];
                if v.to_bits() != prev.axes[axis as usize].to_bits() {
                    events.push(Event::Axis(axis, v));
                }
            }
            *prev = now;
        }
        events
    }
}

#[cfg(feature = "gamepad")]
pub use gamepad::Gamepads;

#[cfg(feature = "gamepad")]
mod gamepad {
    use super::{PadState, pov, quantize_axis};
    use gilrs::{Axis, Button, Gamepad, Gilrs};

    /// GM button numbers for the pad's buttons, in order (see [`PadState::buttons`]).
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

    /// Every connected gamepad, read as GM's joystick 1: the first one connected.
    pub struct Gamepads(Box<Gilrs>);

    impl Gamepads {
        /// `None` when the platform has no gamepad support.
        pub fn new() -> Option<Gamepads> {
            Gilrs::new().ok().map(|g| Gamepads(Box::new(g)))
        }

        /// Catches up with the pads' events and reads the first pad's state.
        pub fn poll(&mut self) -> Option<PadState> {
            while self.0.next_event().is_some() {}
            let (_, pad) = self.0.gamepads().next()?;
            Some(read(&pad))
        }
    }

    fn read(pad: &Gamepad) -> PadState {
        let mut buttons = 0;
        for (i, &b) in BUTTONS.iter().enumerate() {
            if pad.is_pressed(b) {
                buttons |= 1 << i;
            }
        }
        let trigger = |b| pad.button_data(b).map_or(0.0, |d| d.value());
        // Some platforms report the d-pad as axes rather than buttons.
        let dpad = |b, axis, sign: f32| pad.is_pressed(b) || pad.value(axis) * sign > 0.5;
        PadState {
            buttons,
            axes: [
                quantize_axis(pad.value(Axis::LeftStickX)),
                // gilrs's y grows upward, winmm's downward.
                quantize_axis(-pad.value(Axis::LeftStickY)),
                quantize_axis(trigger(Button::LeftTrigger2) - trigger(Button::RightTrigger2)),
                pov(
                    dpad(Button::DPadUp, Axis::DPadY, 1.0),
                    dpad(Button::DPadDown, Axis::DPadY, -1.0),
                    dpad(Button::DPadLeft, Axis::DPadX, -1.0),
                    dpad(Button::DPadRight, Axis::DPadX, 1.0),
                ),
            ],
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
        }
        let digits = [
            Digit0, Digit1, Digit2, Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9,
        ];
        for (key, code) in digits.into_iter().zip(b'0'..=b'9') {
            assert_eq!(vk(key), Some(code), "{key:?}");
        }
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
    fn a_tap_between_ticks_reaches_the_next_tick_in_order() {
        let mut c = Collector::new(false);
        c.key(KeyCode::KeyZ, true);
        c.key(KeyCode::KeyZ, false);
        c.key(KeyCode::ArrowLeft, true);
        c.key(KeyCode::ArrowLeft, true); // auto-repeat
        let z = Button::Key(b'Z');
        let left = Button::Key(37);
        assert_eq!(
            c.take_tick(None),
            [
                Event::Press(z),
                Event::Release(z),
                Event::Press(left),
                Event::Press(left)
            ]
        );
        assert_eq!(c.take_tick(None), []);
        // A release the game never saw pressed is dropped; focus loss releases what's held.
        c.key(KeyCode::KeyX, false);
        c.mouse_button(MouseButton::Left, true);
        c.focus_lost();
        assert_eq!(
            c.take_tick(None),
            [
                Event::Press(Button::Mouse(MouseButton::Left)),
                Event::Release(left),
                Event::Release(Button::Mouse(MouseButton::Left)),
            ]
        );
    }

    #[test]
    fn the_cursor_and_pad_report_changes_once_per_tick() {
        let mut c = Collector::new(true);
        c.cursor(1, 2);
        c.cursor(3, 4);
        let pad = PadState {
            buttons: 0b101,
            axes: [0.0, 0.0, 0.0, 90.0],
        };
        assert_eq!(
            c.take_tick(Some(&pad)),
            [
                Event::MouseMove { x: 3, y: 4 },
                Event::Press(Button::Joy(1)),
                Event::Press(Button::Joy(3)),
                Event::Axis(Axis::Pov, 90.0),
            ]
        );
        assert_eq!(c.take_tick(Some(&pad)), []);
        // A pad that goes away reads as released and centred.
        assert_eq!(
            c.take_tick(None),
            [
                Event::Release(Button::Joy(1)),
                Event::Release(Button::Joy(3)),
                Event::Axis(Axis::Pov, -1.0),
            ]
        );
        // Without a joystick the pad is never reported.
        let mut c = Collector::new(false);
        assert_eq!(c.take_tick(Some(&pad)), []);
    }

    #[test]
    fn pad_helpers() {
        assert_eq!(pov(true, false, false, true), 45.0);
        assert_eq!(pov(false, true, true, false), 225.0);
        assert_eq!(pov(true, true, false, false), -1.0);
        assert_eq!(quantize_axis(0.123), 0.12);
        assert_eq!(quantize_axis(-7.0), -1.0);
    }
}
