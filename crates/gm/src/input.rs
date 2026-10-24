//! Keyboard, mouse and joystick state as the runner sees it, driven by the replay's raw input events
//! (`trace::replay`). OpenGMK `input.rs`.
//!
//! A frame's events are applied in order before the frame runs. Pressed/released flags are set by the
//! events themselves, not by comparing held states, so a key pressed and released within one frame
//! reports both. They are cleared at the end of the frame.
//!
//! GM8 reads joysticks through winmm when a `joystick_*` function is called. Here joystick 1 is the
//! replay's (its `joystick` directive, [`World::connect_joystick`]) and its buttons and axes follow the
//! replay's events; joystick 2 never exists. A joystick that doesn't exist answers what OpenGMK's stubs
//! answer for every joystick, so a replay without the directive runs as it does under the oracle.

use crate::World;
use trace::keys::KeySet;
use trace::replay::{Axis, Button, Event, JoystickInfo, MouseButton};

/// GM's `vk_nokey`: keyboard events with this key fire when no key is down.
pub const VK_NOKEY: u8 = 0;
/// GM's `vk_anykey`: keyboard events with this key fire when any key is down.
pub const VK_ANYKEY: u8 = 1;
/// GM's `mb_any`: `mouse_check_button` with it checks whether any button is in that state.
pub const MB_ANY: i32 = -1;
/// GM's `mb_none`: `mouse_check_button` with it checks whether no button is in that state.
pub const MB_NONE: i32 = 0;

/// Which keyboard state a check reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyState {
    Held,
    Pressed,
    Released,
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct InputState {
    held: KeySet,
    pressed: KeySet,
    released: KeySet,
    keyboard_key: u8,
    keyboard_lastkey: u8,
    /// Bit `b - 1` is mouse button `b`.
    mouse_held: u8,
    mouse_pressed: u8,
    mouse_released: u8,
    mouse_button: i32,
    mouse_lastbutton: i32,
    mouse_x: i32,
    mouse_y: i32,
    /// The mouse where the previous frame left it: the mouse enter and leave events compare against it.
    mouse_previous: (i32, i32),
    /// Whether the wheel turned up, and down, this frame. GM8 keeps both: a frame can fire both events.
    wheel_up: bool,
    wheel_down: bool,
    /// Joystick 1, if connected.
    joystick: Option<Joystick>,
}

/// A connected joystick. GM8 polls joysticks and has no pressed/released flags for them (the game derives
/// its own in `oGamepad`), so only held state is kept: the replay's own model, whose defaults (buttons up,
/// axes 0, POV -1) are also what OpenGMK's stubs answer for a joystick that doesn't exist.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct Joystick {
    has_pov: bool,
    /// Only the joystick buttons and axes are used.
    state: trace::replay::InputState,
}

/// `joystick_direction` for a centred stick, and OpenGMK's stub answer: `vk_numpad5`.
const DIRECTION_CENTRED: i32 = 101;
/// How far an axis must lean for `joystick_direction` to count it. GM8's threshold can't be checked
/// against the oracle, which has no joystick; half the range is an assumption (`docs/quirks.md`).
const DIRECTION_THRESHOLD: f64 = 0.5;

impl InputState {
    /// Starts a frame: the wheel only lasts one frame, and the mouse's previous position becomes where it
    /// is now (OpenGMK `input.rs` `mouse_step`, run before the frame's events).
    pub(crate) fn begin_frame(&mut self) {
        self.mouse_previous = (self.mouse_x, self.mouse_y);
        (self.wheel_up, self.wheel_down) = (false, false);
    }

    /// Applies one input event. OpenGMK `input.rs` (`button_press`, `button_release`, `mouse_press`,
    /// `mouse_release`), which the oracle feeds the replay's events to unchanged.
    ///
    /// A press of a key that is already down reports it pressed again, as OpenGMK's `button_press` does: a
    /// replay that repeats `press` behaves like keyboard auto-repeat. Releasing any mouse button clears
    /// `mouse_button`, even if another is still down (OpenGMK `mouse_release`). Both are recorded in
    /// `docs/quirks.md`.
    pub fn apply(&mut self, e: &Event) {
        match *e {
            Event::Press(Button::Key(vk)) => {
                self.held.insert(vk);
                self.pressed.insert(vk);
                self.keyboard_key = vk;
                self.keyboard_lastkey = vk;
            }
            Event::Release(Button::Key(vk)) => {
                self.held.remove(vk);
                self.released.insert(vk);
                if self.keyboard_key == vk {
                    self.keyboard_key = 0;
                }
            }
            Event::Press(Button::Mouse(b)) => {
                let bit = mouse_bit(b);
                self.mouse_held |= bit;
                self.mouse_pressed |= bit;
                self.mouse_button = b as i32;
                self.mouse_lastbutton = b as i32;
            }
            Event::Release(Button::Mouse(b)) => {
                let bit = mouse_bit(b);
                self.mouse_held &= !bit;
                self.mouse_released |= bit;
                self.mouse_button = 0;
            }
            Event::WheelUp => self.wheel_up = true,
            Event::WheelDown => self.wheel_down = true,
            Event::MouseMove { x, y } => (self.mouse_x, self.mouse_y) = (x, y),
            // Replays reject joystick events without a joystick (`trace::replay`), so ignoring them here
            // only matters to hosts that build events by hand.
            Event::Press(Button::Joy(_)) | Event::Release(Button::Joy(_)) | Event::Axis(..) => {
                if let Some(j) = &mut self.joystick {
                    j.state.apply(e);
                }
            }
        }
    }

    /// Ends a frame: pressed and released flags last one frame. OpenGMK `input.rs` (`step`).
    pub(crate) fn end_frame(&mut self) {
        self.pressed = KeySet::new();
        self.released = KeySet::new();
        self.mouse_pressed = 0;
        self.mouse_released = 0;
    }

    fn keys(&self, state: KeyState) -> &KeySet {
        match state {
            KeyState::Held => &self.held,
            KeyState::Pressed => &self.pressed,
            KeyState::Released => &self.released,
        }
    }

    /// `keyboard_check(vk)` and the pressed/released variants, with `vk_nokey`/`vk_anykey`. OpenGMK
    /// `input.rs` (`keyboard_check_internal_indirect`).
    // TODO: OpenGMK folds left/right shift, control and alt into vk_shift/vk_control/vk_alt and ignores
    // "direct only" codes in vk_anykey. Settle how replays spell those keys against the oracle first.
    pub fn check(&self, state: KeyState, vk: u8) -> bool {
        match vk {
            VK_NOKEY => !self.any(state),
            VK_ANYKEY => self.any(state),
            _ => self.keys(state).contains(vk),
        }
    }

    /// Whether any key is in this state.
    pub fn any(&self, state: KeyState) -> bool {
        !self.keys(state).is_empty()
    }

    pub fn keyboard_key(&self) -> u8 {
        self.keyboard_key
    }

    pub fn keyboard_lastkey(&self) -> u8 {
        self.keyboard_lastkey
    }

    pub(crate) fn set_keyboard_key(&mut self, vk: u8) {
        self.keyboard_key = vk;
    }

    pub(crate) fn set_keyboard_lastkey(&mut self, vk: u8) {
        self.keyboard_lastkey = vk;
    }

    pub(crate) fn set_mouse_button(&mut self, b: i32) {
        self.mouse_button = b;
    }

    pub(crate) fn set_mouse_lastbutton(&mut self, b: i32) {
        self.mouse_lastbutton = b;
    }

    /// The mouse position in window pixels; `World::mouse_in_room` maps it through the views.
    pub fn mouse(&self) -> (i32, i32) {
        (self.mouse_x, self.mouse_y)
    }

    /// `mouse_button`: the button last pressed while it's still down, else 0.
    pub fn mouse_button(&self) -> i32 {
        self.mouse_button
    }

    pub fn mouse_lastbutton(&self) -> i32 {
        self.mouse_lastbutton
    }

    /// Whether a mouse button is in this state.
    pub fn mouse_check(&self, state: KeyState, b: MouseButton) -> bool {
        self.mouse_check_gm(state, b as i32)
    }

    /// The mouse at the start of this frame, in window pixels: where the previous frame left it.
    pub fn mouse_previous(&self) -> (i32, i32) {
        self.mouse_previous
    }

    /// Whether the wheel turned up this frame.
    pub fn wheel_up(&self) -> bool {
        self.wheel_up
    }

    /// Whether the wheel turned down this frame.
    pub fn wheel_down(&self) -> bool {
        self.wheel_down
    }

    /// `mouse_check_button(b)` and friends as GML calls them, with `mb_any` (-1) and `mb_none` (0); other
    /// numbers are never down. OpenGMK `input.rs` (`mouse_check_button_internal_indirect`).
    pub fn mouse_check_gm(&self, state: KeyState, b: i32) -> bool {
        let bits = match state {
            KeyState::Held => self.mouse_held,
            KeyState::Pressed => self.mouse_pressed,
            KeyState::Released => self.mouse_released,
        };
        match b {
            MB_ANY => bits != 0,
            MB_NONE => bits == 0,
            1..=3 => bits & (1 << (b - 1)) != 0,
            _ => false,
        }
    }

    /// Connects joystick 1 with these properties, or disconnects it (`None`). Its buttons start up and its
    /// axes centred.
    pub fn connect_joystick(&mut self, info: Option<&JoystickInfo>) {
        self.joystick = info.map(|info| Joystick {
            has_pov: info.has_pov,
            state: trace::replay::InputState::default(),
        });
    }

    fn joystick(&self, id: i32) -> Option<&Joystick> {
        self.joystick.as_ref().filter(|_| id == 1)
    }

    /// `joystick_exists(id)`.
    pub fn joystick_exists(&self, id: i32) -> bool {
        self.joystick(id).is_some()
    }

    /// `joystick_check_button(id, b)`: buttons outside 1-32 are never down.
    pub fn joystick_check_button(&self, id: i32, b: i32) -> bool {
        self.joystick(id)
            .is_some_and(|j| u8::try_from(b).is_ok_and(|b| j.state.is_down(Button::Joy(b))))
    }

    /// `joystick_xpos`, `_ypos` and `_zpos` (-1 to 1), and `joystick_pov` (degrees clockwise from up; -1
    /// when centred, and always without a POV hat).
    pub fn joystick_axis(&self, id: i32, axis: Axis) -> f64 {
        match self.joystick(id) {
            Some(j) if axis != Axis::Pov || j.has_pov => j.state.axis(axis),
            _ => trace::replay::InputState::default().axis(axis),
        }
    }

    /// `joystick_has_pov(id)`.
    pub fn joystick_has_pov(&self, id: i32) -> bool {
        self.joystick(id).is_some_and(|j| j.has_pov)
    }

    /// `joystick_direction(id)`: the stick's direction as the numpad key that points that way,
    /// `vk_numpad1` (97, down-left) to `vk_numpad9` (105, up-right), and 101 when centred.
    pub fn joystick_direction(&self, id: i32) -> i32 {
        let Some(j) = self.joystick(id) else {
            return DIRECTION_CENTRED;
        };
        let lean = |v: f64| {
            if v < -DIRECTION_THRESHOLD {
                -1
            } else if v > DIRECTION_THRESHOLD {
                1
            } else {
                0
            }
        };
        let (dx, dy) = (lean(j.state.axis(Axis::X)), lean(j.state.axis(Axis::Y)));
        // The numpad's rows count upward while y grows downward: down is the bottom row (97-99).
        DIRECTION_CENTRED + dx - 3 * dy
    }
}

impl World {
    /// Connects joystick 1 (the replay's `joystick` directive), or leaves none, as under the oracle.
    pub fn connect_joystick(&mut self, info: Option<&JoystickInfo>) {
        self.input.connect_joystick(info);
    }
}

fn mouse_bit(b: MouseButton) -> u8 {
    1 << (b as u8 - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tap_within_one_frame_is_pressed_and_released() {
        let mut i = InputState::default();
        i.apply(&Event::Press(Button::Key(90)));
        i.apply(&Event::Release(Button::Key(90)));
        assert!(i.check(KeyState::Pressed, 90));
        assert!(i.check(KeyState::Released, 90));
        assert!(!i.check(KeyState::Held, 90));
        assert!(i.check(KeyState::Held, VK_NOKEY));
        assert_eq!((i.keyboard_key(), i.keyboard_lastkey()), (0, 90));
        i.end_frame();
        assert!(!i.check(KeyState::Pressed, 90));
        assert!(i.check(KeyState::Pressed, VK_NOKEY));
    }

    #[test]
    fn keyboard_key_tracks_the_last_key_still_down() {
        let mut i = InputState::default();
        i.apply(&Event::Press(Button::Key(37)));
        i.apply(&Event::Press(Button::Key(38)));
        i.apply(&Event::Release(Button::Key(37)));
        assert_eq!(i.keyboard_key(), 38);
        assert!(i.check(KeyState::Held, VK_ANYKEY));
    }

    #[test]
    fn mouse_buttons() {
        let mut i = InputState::default();
        i.apply(&Event::Press(Button::Mouse(MouseButton::Right)));
        i.apply(&Event::MouseMove { x: 5, y: 7 });
        assert!(i.mouse_check(KeyState::Held, MouseButton::Right));
        assert!(!i.mouse_check(KeyState::Held, MouseButton::Left));
        assert_eq!((i.mouse_button(), i.mouse()), (2, (5, 7)));
        i.apply(&Event::Release(Button::Mouse(MouseButton::Right)));
        assert_eq!((i.mouse_button(), i.mouse_lastbutton()), (0, 2));
        assert!(i.mouse_check_gm(KeyState::Held, 0));
        assert!(i.mouse_check_gm(KeyState::Released, -1));
        assert!(i.mouse_check_gm(KeyState::Released, 2));
        assert!(!i.mouse_check_gm(KeyState::Released, 4));
    }

    #[test]
    fn without_a_joystick_every_answer_is_opengmks_stub() {
        let mut i = InputState::default();
        // Hosts that build events by hand may still send joystick events; they change nothing.
        i.apply(&Event::Press(Button::Joy(1)));
        i.apply(&Event::Axis(Axis::X, -1.0));
        for id in [0, 1, 2] {
            assert!(!i.joystick_exists(id));
            assert!(!i.joystick_has_pov(id));
            assert!(!i.joystick_check_button(id, 1));
            assert_eq!(i.joystick_direction(id), 101);
            assert_eq!(i.joystick_axis(id, Axis::X), 0.0);
            assert_eq!(i.joystick_axis(id, Axis::Z), 0.0);
            assert_eq!(i.joystick_axis(id, Axis::Pov), -1.0);
        }
    }

    #[test]
    fn joystick_one_follows_its_events() {
        let mut i = InputState::default();
        i.connect_joystick(Some(&JoystickInfo { has_pov: true }));
        assert!(i.joystick_exists(1) && !i.joystick_exists(2));
        assert_eq!(i.joystick_axis(1, Axis::Pov), -1.0);
        i.apply(&Event::Press(Button::Joy(3)));
        i.apply(&Event::Press(Button::Joy(32)));
        i.apply(&Event::Axis(Axis::Z, 0.5));
        i.apply(&Event::Axis(Axis::Pov, 270.0));
        assert!(i.joystick_check_button(1, 3) && i.joystick_check_button(1, 32));
        assert!(!i.joystick_check_button(1, 33) && !i.joystick_check_button(2, 3));
        assert_eq!(i.joystick_axis(1, Axis::Z), 0.5);
        assert_eq!(i.joystick_axis(1, Axis::Pov), 270.0);
        // Held state only: the end of a frame doesn't release joystick buttons.
        i.end_frame();
        assert!(i.joystick_check_button(1, 3));
        i.apply(&Event::Release(Button::Joy(3)));
        assert!(!i.joystick_check_button(1, 3));

        // Without a POV hat, `joystick_pov` stays centred whatever the events say.
        i.connect_joystick(Some(&JoystickInfo { has_pov: false }));
        i.apply(&Event::Axis(Axis::Pov, 90.0));
        assert_eq!(i.joystick_axis(1, Axis::Pov), -1.0);
    }

    #[test]
    fn joystick_direction_is_a_numpad_key() {
        let mut i = InputState::default();
        i.connect_joystick(Some(&JoystickInfo::default()));
        let cases = [
            ((-1.0, 1.0), 97),
            ((0.0, 1.0), 98),
            ((1.0, 1.0), 99),
            ((-1.0, 0.0), 100),
            ((0.0, 0.0), 101),
            ((1.0, 0.0), 102),
            ((-1.0, -1.0), 103),
            ((0.0, -1.0), 104),
            ((1.0, -1.0), 105),
            // Within the threshold counts as centred.
            ((0.5, -0.5), 101),
        ];
        for ((x, y), vk) in cases {
            i.apply(&Event::Axis(Axis::X, x));
            i.apply(&Event::Axis(Axis::Y, y));
            assert_eq!(i.joystick_direction(1), vk, "({x}, {y})");
        }
    }
}
