//! Keyboard and mouse state as the runner sees it, driven by the replay's raw input events
//! (`trace::replay`). OpenGMK `input.rs`.
//!
//! A frame's events are applied in order before the frame runs. Pressed/released flags are set by the
//! events themselves, not by comparing held states, so a key pressed and released within one frame
//! reports both. They are cleared at the end of the frame.

use trace::keys::KeySet;
use trace::replay::{Button, Event, MouseButton};

/// GM's `vk_nokey`: keyboard events with this key fire when no key is down.
pub const VK_NOKEY: u8 = 0;
/// GM's `vk_anykey`: keyboard events with this key fire when any key is down.
pub const VK_ANYKEY: u8 = 1;

/// Which keyboard state a check reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyState {
    Held,
    Pressed,
    Released,
}

#[derive(Clone, Debug, Default, PartialEq)]
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
    /// This frame's wheel: 1 up, -1 down, 0 none.
    wheel: i8,
}

impl InputState {
    /// Starts a frame: the wheel only lasts one frame (OpenGMK `input.rs` `mouse_step`, run before the
    /// frame's events).
    pub(crate) fn begin_frame(&mut self) {
        self.wheel = 0;
    }

    /// Applies one input event. OpenGMK `input.rs` (`button_press`, `button_release`, `mouse_press`,
    /// `mouse_release`).
    // TODO(851-3128): map mouse positions from window pixels into the room through the view.
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
            Event::WheelUp => self.wheel = 1,
            Event::WheelDown => self.wheel = -1,
            Event::MouseMove { x, y } => (self.mouse_x, self.mouse_y) = (x, y),
            // Joysticks are read by built-in functions, outside this runtime's scope so far.
            Event::Press(Button::Joy(_)) | Event::Release(Button::Joy(_)) | Event::Axis(..) => {}
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

    /// The mouse position, in window pixels until views map it.
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
        let bits = match state {
            KeyState::Held => self.mouse_held,
            KeyState::Pressed => self.mouse_pressed,
            KeyState::Released => self.mouse_released,
        };
        bits & mouse_bit(b) != 0
    }

    /// This frame's wheel movement: 1 up, -1 down, 0 none.
    pub fn wheel(&self) -> i8 {
        self.wheel
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
    }
}
