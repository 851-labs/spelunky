//! GameMaker 8 virtual-key codes (Windows VK codes) and their names in text replays.
//!
//! The names are a superset of the oracle's replay parser: it knows `left up right down enter return escape
//! esc space shift control ctrl alt tab`, single letters and digits, and decimal codes. [`key_name`] only
//! writes names from that set, so replays this crate writes stay readable by the oracle.

use std::fmt;

/// Named keys, besides the letters `A`-`Z` and digits `0`-`9` (which are their own names). These are the keys
/// GM 8 has `vk_*` constants for, under the same names without the prefix.
const NAMED: &[(u8, &str)] = &[
    (8, "backspace"),
    (9, "tab"),
    (13, "enter"),
    (16, "shift"),
    (17, "control"),
    (18, "alt"),
    (19, "pause"),
    (27, "escape"),
    (32, "space"),
    (33, "pageup"),
    (34, "pagedown"),
    (35, "end"),
    (36, "home"),
    (37, "left"),
    (38, "up"),
    (39, "right"),
    (40, "down"),
    (44, "printscreen"),
    (45, "insert"),
    (46, "delete"),
    (96, "numpad0"),
    (97, "numpad1"),
    (98, "numpad2"),
    (99, "numpad3"),
    (100, "numpad4"),
    (101, "numpad5"),
    (102, "numpad6"),
    (103, "numpad7"),
    (104, "numpad8"),
    (105, "numpad9"),
    (106, "multiply"),
    (107, "add"),
    (109, "subtract"),
    (110, "decimal"),
    (111, "divide"),
    (112, "f1"),
    (113, "f2"),
    (114, "f3"),
    (115, "f4"),
    (116, "f5"),
    (117, "f6"),
    (118, "f7"),
    (119, "f8"),
    (120, "f9"),
    (121, "f10"),
    (122, "f11"),
    (123, "f12"),
];

/// The lowest code a replay may hold. 0 (`vk_nokey`) and 1 (`vk_anykey`) are pseudo-keys the runner derives.
pub const MIN_KEY: u8 = 2;

/// Names the oracle's parser also knows.
const ORACLE_NAMED: [&str; 11] = [
    "left", "up", "right", "down", "enter", "escape", "space", "shift", "control", "alt", "tab",
];

/// Other spellings the oracle accepts.
const ALIASES: [(&str, u8); 3] = [("return", 13), ("esc", 27), ("ctrl", 17)];

/// The canonical name of a key: the letter or digit, a name the oracle knows, or the decimal code (at least
/// two digits, since `5` is the digit key).
pub fn key_name(vk: u8) -> String {
    if vk.is_ascii_uppercase() || vk.is_ascii_digit() {
        return (vk as char).to_string();
    }
    match NAMED
        .iter()
        .find(|(code, name)| *code == vk && ORACLE_NAMED.contains(name))
    {
        Some((_, name)) => (*name).to_string(),
        None => format!("{vk:02}"),
    }
}

/// Parses a key: a name (case-insensitive), a letter or digit, or a code as `NN` (two or more digits, since
/// a single digit is that digit's key) or `vkNN`. Returns `None` for unknown names and for codes below
/// [`MIN_KEY`].
pub fn parse_key(s: &str) -> Option<u8> {
    let lower = s.to_ascii_lowercase();
    let decimal = |d: &str| -> Option<u8> {
        if d.is_empty() || !d.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        d.parse::<u8>().ok()
    };
    let vk = if let [c] = lower.as_bytes() {
        let c = c.to_ascii_uppercase();
        (c.is_ascii_uppercase() || c.is_ascii_digit()).then_some(c)?
    } else if lower.bytes().all(|b| b.is_ascii_digit()) {
        decimal(&lower)?
    } else if let Some(code) = lower.strip_prefix("vk") {
        decimal(code)?
    } else if let Some((_, code)) = ALIASES.iter().find(|(name, _)| *name == lower) {
        *code
    } else {
        NAMED.iter().find(|(_, name)| *name == lower)?.0
    };
    (vk >= MIN_KEY).then_some(vk)
}

/// The set of virtual keys held down during one tick.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct KeySet([u64; 4]);

impl KeySet {
    pub const fn new() -> Self {
        KeySet([0; 4])
    }

    pub fn contains(&self, vk: u8) -> bool {
        self.0[usize::from(vk >> 6)] & (1 << (vk & 63)) != 0
    }

    /// Adds a key; returns false if it was already held.
    pub fn insert(&mut self, vk: u8) -> bool {
        let had = self.contains(vk);
        self.0[usize::from(vk >> 6)] |= 1 << (vk & 63);
        !had
    }

    pub fn remove(&mut self, vk: u8) {
        self.0[usize::from(vk >> 6)] &= !(1 << (vk & 63));
    }

    pub fn is_empty(&self) -> bool {
        self.0 == [0; 4]
    }

    /// Held keys in ascending code order.
    pub fn iter(&self) -> impl Iterator<Item = u8> + '_ {
        (0..=255u8).filter(|&vk| self.contains(vk))
    }

    /// Keys held now but not in `previous`: what `keyboard_check_pressed` reports this tick.
    pub fn pressed_since(&self, previous: &KeySet) -> KeySet {
        KeySet(std::array::from_fn(|i| self.0[i] & !previous.0[i]))
    }

    /// Keys held in `previous` but not now: what `keyboard_check_released` reports this tick.
    pub fn released_since(&self, previous: &KeySet) -> KeySet {
        previous.pressed_since(self)
    }
}

impl FromIterator<u8> for KeySet {
    fn from_iter<I: IntoIterator<Item = u8>>(iter: I) -> Self {
        let mut set = KeySet::new();
        for vk in iter {
            set.insert(vk);
        }
        set
    }
}

impl fmt::Debug for KeySet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter().map(key_name)).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip() {
        for vk in MIN_KEY..=255 {
            assert_eq!(parse_key(&key_name(vk)), Some(vk), "{vk}");
        }
    }

    #[test]
    fn parsing() {
        assert_eq!(parse_key("z"), Some(b'Z'));
        assert_eq!(parse_key("Z"), Some(b'Z'));
        assert_eq!(parse_key("5"), Some(b'5'));
        assert_eq!(parse_key("Left"), Some(37));
        assert_eq!(parse_key("vk38"), Some(38));
        assert_eq!(parse_key("38"), Some(38));
        assert_eq!(parse_key("008"), Some(8));
        assert_eq!(parse_key("shift"), Some(16));
        assert_eq!(parse_key("F12"), Some(123));
        assert_eq!(
            (parse_key("return"), parse_key("ESC"), parse_key("ctrl")),
            (Some(13), Some(27), Some(17))
        );
        assert_eq!(key_name(112), "112");
        assert_eq!(key_name(17), "control");
        for bad in [
            "", "vk", "vk1", "vk0", "01", "256", "vk256", "vk-1", "vk+3", "+38", "leftt", "?", "zz",
        ] {
            assert_eq!(parse_key(bad), None, "{bad}");
        }
    }

    #[test]
    fn key_sets() {
        let prev: KeySet = [37, 90].into_iter().collect();
        let now: KeySet = [37, 88, 255].into_iter().collect();
        assert_eq!(
            now.pressed_since(&prev).iter().collect::<Vec<_>>(),
            [88, 255]
        );
        assert_eq!(now.released_since(&prev).iter().collect::<Vec<_>>(), [90]);
        assert!(!KeySet::new().contains(0));
        assert!(KeySet::new().is_empty());
    }
}
