//! The native apps' updater (851-3255): checks a signed manifest for a newer release, downloads and
//! verifies it in the background, and once the player agrees, swaps it in and restarts, rolling back if
//! the new version doesn't start. `docs/updates.md` has the whole picture, the release side included.
//!
//! - [`manifest`]: `latest.json`, signed with the release key ([`key`]); a bad signature, a bad
//!   checksum or an older version means no install.
//! - [`check`]: fetch, verify, download ([`fetch`]) and unpack a newer version next to the install.
//! - [`install`]: what an install is on each platform, and swapping a new version in and out of it.
//! - [`restart`]: starting the new version and rolling back if it fails.
//! - [`state`]: the player's setting and the last check, between sessions.
//!
//! All of it is native-only frontend business: the game crates never see it, and the web build (which
//! updates itself) doesn't build it. The app (`app::update`) is its only user, apart from the tests'
//! stand-in app (`src/bin/update-fixture.rs`) and the release tool (`src/bin/release-manifest.rs`).

pub mod check;
pub mod fetch;
pub mod install;
pub mod key;
pub mod manifest;
pub mod restart;
pub mod state;
pub mod version;

pub use version::Version;

/// Where releases publish the manifest.
pub const MANIFEST_URL: &str = "https://spelunky.ai/releases/latest.json";

/// This build's platform, as the manifest names downloads: `macos` (one universal build), or
/// `windows-x86_64`, `windows-aarch64`, `linux-x86_64`, `linux-aarch64`.
pub fn platform() -> String {
    if cfg!(target_os = "macos") {
        return "macos".into();
    }
    let os = if cfg!(windows) {
        "windows"
    } else {
        std::env::consts::OS
    };
    format!("{os}-{}", std::env::consts::ARCH)
}
