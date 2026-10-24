//! What the updater remembers between sessions, in `update.json` in the per-user data directory (beside
//! the saves, which it never touches): whether to check automatically, when it last did, a version the
//! player chose to skip, and an update that had to be rolled back.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Automatic checks happen at most this often, in seconds.
pub const CHECK_EVERY: u64 = 24 * 60 * 60;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    /// Check on launch (at most once a day). On unless the player turns it off.
    pub automatic: bool,
    /// When the last check started, in seconds since the Unix epoch.
    pub last_check: Option<u64>,
    /// A version not to offer again automatically ("skip this version"). Checking by hand still does.
    pub skip: Option<String>,
    /// An update that didn't start and was rolled back, to tell the player about once.
    pub failed: Option<Failed>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Failed {
    pub version: String,
    pub reason: String,
}

impl Default for State {
    fn default() -> State {
        State {
            automatic: true,
            last_check: None,
            skip: None,
            failed: None,
        }
    }
}

impl State {
    /// The state saved at `path`; the default if there's none or it can't be read.
    pub fn load(path: &Path) -> State {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Saves to `path`, whole: written beside it, then renamed over it.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let error = |e: std::io::Error| format!("{}: {e}", path.display());
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(error)?;
        }
        let mut text = serde_json::to_string_pretty(self).expect("the state serializes");
        text.push('\n');
        let temp = path.with_extension("json.new");
        std::fs::write(&temp, text).map_err(error)?;
        std::fs::rename(&temp, path).map_err(error)
    }

    /// Whether an automatic check is due at `now` (seconds since the epoch). A clock that went back counts
    /// as a day gone by, so a wrong clock can't stop checks for good.
    pub fn check_due(&self, now: u64) -> bool {
        self.automatic
            && self
                .last_check
                .is_none_or(|last| now < last || now - last >= CHECK_EVERY)
    }
}

/// Seconds since the Unix epoch, now.
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_at_most_daily_and_only_when_on() {
        let mut state = State::default();
        assert!(state.check_due(1000));
        state.last_check = Some(1000);
        assert!(!state.check_due(1000 + CHECK_EVERY - 1));
        assert!(state.check_due(1000 + CHECK_EVERY));
        assert!(state.check_due(999));
        state.automatic = false;
        assert!(!state.check_due(1000 + CHECK_EVERY));
    }

    #[test]
    fn round_trips_and_survives_junk() {
        let dir = std::env::temp_dir().join(format!("spelunky-state-{}", std::process::id()));
        let path = dir.join("update.json");
        assert_eq!(State::load(&path), State::default());
        let state = State {
            automatic: false,
            last_check: Some(5),
            skip: Some("1.2.0".into()),
            failed: Some(Failed {
                version: "1.3.0".into(),
                reason: "it quit".into(),
            }),
        };
        state.save(&path).unwrap();
        assert_eq!(State::load(&path), state);
        std::fs::write(&path, "{ not json").unwrap();
        assert_eq!(State::load(&path), State::default());
        // An older file without newer fields keeps their defaults.
        std::fs::write(&path, r#"{"last_check": 7}"#).unwrap();
        assert_eq!(State::load(&path).last_check, Some(7));
        assert!(State::load(&path).automatic);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
