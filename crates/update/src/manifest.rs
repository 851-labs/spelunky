//! The update manifest, `https://spelunky.ai/releases/latest.json`: the newest release's version, its
//! release notes, and each platform's download, signed with the release key ([`crate::key`]).
//!
//! The file is an envelope around the manifest's exact text, so the signature covers bytes, with no JSON
//! canonical form to agree on:
//!
//! ```json
//! {
//!   "manifest": "{\"version\":\"0.2.0\",\"notes\":\"https://...\",\"assets\":{...}}",
//!   "signature": "<ed25519 signature, hex>"
//! }
//! ```
//!
//! The signature is over [`CONTEXT`] followed by the manifest text, so a signature made for anything else
//! with the same key can't pass for a manifest's. The manifest itself:
//!
//! ```json
//! {
//!   "version": "0.2.0",
//!   "notes": "https://github.com/851-labs/spelunky/releases/tag/v1.1.0",
//!   "assets": {
//!     "macos": { "url": "https://github.com/...", "sha256": "<hex>", "size": 12345, "format": "app-zip" },
//!     "windows-x86_64": { ..., "format": "dir-zip" },
//!     "linux-x86_64": { ..., "format": "appimage" }
//!   }
//! }
//! ```
//!
//! [`open`] checks the signature before reading anything else, then the manifest's own sense: a version, and
//! assets whose URLs are HTTPS and whose checksums are SHA-256s.

use crate::key::{from_hex, to_hex};
use crate::version::Version;
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// What the signature covers before the manifest's text.
pub const CONTEXT: &[u8] = b"spelunky-classic update manifest v1\n";

/// The largest download a manifest may name: far over any build (the macOS app is about 40 MB).
pub const MAX_ASSET_SIZE: u64 = 1 << 30;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// The release's version, without the tag's `v`.
    pub version: String,
    /// Its release notes (the GitHub Release page).
    pub notes: String,
    /// Each platform's download, by [`crate::platform`] name.
    pub assets: BTreeMap<String, Asset>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    pub url: String,
    /// Lower-case hex.
    pub sha256: String,
    pub size: u64,
    pub format: Format,
}

/// What a download holds, which is how it's installed ([`crate::install`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Format {
    /// A zip holding one `.app` bundle, swapped in whole (macOS).
    AppZip,
    /// A zip of the install directory, maybe inside one top-level folder: its files replace the
    /// install's (Windows).
    DirZip,
    /// The AppImage itself, replacing the one that runs (Linux).
    Appimage,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    manifest: String,
    signature: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// Not an envelope at all (or not JSON).
    Malformed(String),
    /// The signature isn't the release key's on this manifest: tampered with, or signed with another key.
    BadSignature,
    /// Signed, but not a sensible manifest (only a broken release could make one).
    Invalid(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Malformed(e) => write!(f, "the update manifest is malformed: {e}"),
            Error::BadSignature => {
                write!(f, "the update manifest isn't signed with the release key")
            }
            Error::Invalid(e) => write!(f, "the update manifest is invalid: {e}"),
        }
    }
}

/// The envelope for `manifest`, signed with `key`: `latest.json`'s contents.
pub fn sign(manifest: &Manifest, key: &SigningKey) -> String {
    let text = serde_json::to_string(manifest).expect("a manifest serializes");
    let signature = key.sign(&message(&text));
    let envelope = Envelope {
        manifest: text,
        signature: to_hex(&signature.to_bytes()),
    };
    let mut out = serde_json::to_string_pretty(&envelope).expect("an envelope serializes");
    out.push('\n');
    out
}

/// The manifest in `latest.json`'s `text`, if `key` signed it and it makes sense.
pub fn open(text: &str, key: &VerifyingKey) -> Result<Manifest, Error> {
    let envelope: Envelope =
        serde_json::from_str(text).map_err(|e| Error::Malformed(e.to_string()))?;
    let signature = from_hex(&envelope.signature)
        .and_then(|b| <[u8; 64]>::try_from(b).ok())
        .ok_or(Error::BadSignature)?;
    // Strict: no malleable signatures or weak keys.
    key.verify_strict(
        &message(&envelope.manifest),
        &Signature::from_bytes(&signature),
    )
    .map_err(|_| Error::BadSignature)?;
    let manifest: Manifest =
        serde_json::from_str(&envelope.manifest).map_err(|e| Error::Invalid(e.to_string()))?;
    manifest.validate().map_err(Error::Invalid)?;
    Ok(manifest)
}

fn message(text: &str) -> Vec<u8> {
    [CONTEXT, text.as_bytes()].concat()
}

impl Manifest {
    pub fn version(&self) -> Result<Version, String> {
        Version::parse(&self.version)
    }

    fn validate(&self) -> Result<(), String> {
        self.version()?;
        check_url(&self.notes).map_err(|e| format!("notes: {e}"))?;
        for (platform, asset) in &self.assets {
            let error = |e: String| format!("{platform}: {e}");
            check_url(&asset.url).map_err(error)?;
            if asset.sha256.len() != 64
                || !asset
                    .sha256
                    .bytes()
                    .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
            {
                return Err(error("sha256 isn't 64 lower-case hex digits".into()));
            }
            if asset.size == 0 || asset.size > MAX_ASSET_SIZE {
                return Err(error(format!("a size of {} bytes", asset.size)));
            }
        }
        Ok(())
    }
}

/// The URLs the updater fetches: HTTPS, or plain HTTP to this machine (the tests' server).
pub fn check_url(url: &str) -> Result<(), String> {
    if url.starts_with("https://") || is_loopback(url) {
        Ok(())
    } else {
        Err(format!("{url} isn't an https:// URL"))
    }
}

/// `http://127.0.0.1`, `http://localhost` or `http://[::1]`, with any port and path.
pub fn is_loopback(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("http://") else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    // `http://localhost:x@example.com/` is example.com.
    if authority.contains('@') {
        return false;
    }
    let host = match authority.strip_prefix('[') {
        Some(v6) => v6.split(']').next().unwrap_or(""),
        None => authority.split(':').next().unwrap_or(""),
    };
    matches!(host, "127.0.0.1" | "localhost" | "::1")
}

/// What a check found, given the version running.
#[derive(Debug, PartialEq, Eq)]
pub enum Offer {
    /// The manifest's version isn't newer: nothing to do. An older manifest (a rollback of the site, or
    /// an old one replayed by someone in the way) is never a downgrade.
    UpToDate,
    /// A newer version, with this platform's download if it has one.
    Newer {
        version: Version,
        notes: String,
        asset: Option<Asset>,
    },
}

/// What `manifest` offers to a build of `current` on `platform`.
pub fn offer(manifest: &Manifest, current: &Version, platform: &str) -> Result<Offer, String> {
    let version = manifest.version()?;
    if version <= *current {
        return Ok(Offer::UpToDate);
    }
    Ok(Offer::Newer {
        version,
        notes: manifest.notes.clone(),
        asset: manifest.assets.get(platform).cloned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::tests::{TEST_KEY, TEST_PUB};
    use crate::key::{generate, public_key, signing_key};

    fn manifest(version: &str) -> Manifest {
        let asset = |format| Asset {
            url: "https://github.com/851-labs/spelunky/releases/download/v1.1.0/a.zip".into(),
            sha256: "ab".repeat(32),
            size: 1234,
            format,
        };
        Manifest {
            version: version.into(),
            notes: "https://github.com/851-labs/spelunky/releases/tag/v1.1.0".into(),
            assets: BTreeMap::from([
                ("macos".into(), asset(Format::AppZip)),
                ("windows-x86_64".into(), asset(Format::DirZip)),
                ("linux-x86_64".into(), asset(Format::Appimage)),
            ]),
        }
    }

    fn test_key() -> (SigningKey, VerifyingKey) {
        (
            signing_key(TEST_KEY).unwrap(),
            public_key(TEST_PUB).unwrap(),
        )
    }

    #[test]
    fn a_good_manifest_opens() {
        let (private, public) = test_key();
        let m = manifest("0.2.0");
        let file = sign(&m, &private);
        assert_eq!(open(&file, &public), Ok(m.clone()));
        // The formats' spellings are part of the file format.
        assert!(file.contains(r#"\"format\":\"app-zip\""#), "{file}");
        assert!(file.contains(r#"\"format\":\"dir-zip\""#));
        assert!(file.contains(r#"\"format\":\"appimage\""#));
    }

    #[test]
    fn a_tampered_manifest_doesnt() {
        let (private, public) = test_key();
        let file = sign(&manifest("0.2.0"), &private);
        // Anything changed in the signed text: another download, or another version.
        for (from, to) in [("abab", "abac"), ("0.2.0", "0.2.1"), ("1234", "1235")] {
            let tampered = file.replacen(from, to, 1);
            assert_ne!(tampered, file);
            assert_eq!(open(&tampered, &public), Err(Error::BadSignature), "{from}");
        }
        // The signature itself.
        let sig_at = file.find("\"signature\": \"").unwrap() + 14;
        let mut bytes = file.into_bytes();
        bytes[sig_at] = if bytes[sig_at] == b'0' { b'1' } else { b'0' };
        let tampered = String::from_utf8(bytes).unwrap();
        assert_eq!(open(&tampered, &public), Err(Error::BadSignature));
    }

    #[test]
    fn another_keys_manifest_doesnt() {
        let (_, public) = test_key();
        let other = generate().unwrap();
        let file = sign(&manifest("0.2.0"), &other);
        assert_eq!(open(&file, &public), Err(Error::BadSignature));
        assert!(open(&file, &other.verifying_key()).is_ok());
    }

    #[test]
    fn a_signature_made_for_something_else_doesnt() {
        // Signed without the context: the bare manifest text, as if the key signed some other file.
        let (private, public) = test_key();
        let text = serde_json::to_string(&manifest("0.2.0")).unwrap();
        let envelope = Envelope {
            signature: to_hex(&private.sign(text.as_bytes()).to_bytes()),
            manifest: text,
        };
        let file = serde_json::to_string(&envelope).unwrap();
        assert_eq!(open(&file, &public), Err(Error::BadSignature));
    }

    #[test]
    fn junk_is_malformed() {
        let (_, public) = test_key();
        for junk in [
            "",
            "{}",
            "<html>",
            r#"{"manifest":"{}","signature":"00","extra":1}"#,
        ] {
            assert!(
                matches!(open(junk, &public), Err(Error::Malformed(_))),
                "{junk}"
            );
        }
        assert_eq!(
            open(r#"{"manifest":"{}","signature":"zz"}"#, &public),
            Err(Error::BadSignature)
        );
    }

    #[test]
    fn a_signed_manifest_must_make_sense() {
        let (private, public) = test_key();
        let mut bad = Vec::new();
        bad.push(manifest("one"));
        let mut m = manifest("0.2.0");
        m.assets.get_mut("macos").unwrap().url = "http://github.com/a.zip".into();
        bad.push(m);
        let mut m = manifest("0.2.0");
        m.assets.get_mut("macos").unwrap().sha256 = "AB".repeat(32);
        bad.push(m);
        let mut m = manifest("0.2.0");
        m.assets.get_mut("macos").unwrap().size = MAX_ASSET_SIZE + 1;
        bad.push(m);
        for m in bad {
            assert!(matches!(
                open(&sign(&m, &private), &public),
                Err(Error::Invalid(_))
            ));
        }
    }

    #[test]
    fn only_a_newer_version_is_offered() {
        let current = Version::parse("0.2.0").unwrap();
        for older_or_same in ["0.1.0", "0.2.0", "0.2.0-rc.1", "0.0.9"] {
            assert_eq!(
                offer(&manifest(older_or_same), &current, "macos"),
                Ok(Offer::UpToDate),
                "{older_or_same}"
            );
        }
        let Ok(Offer::Newer { version, asset, .. }) = offer(&manifest("0.3.0"), &current, "macos")
        else {
            panic!("0.3.0 is newer")
        };
        assert_eq!(version.to_string(), "0.3.0");
        assert_eq!(asset.unwrap().format, Format::AppZip);
        let Ok(Offer::Newer { asset, .. }) = offer(&manifest("0.3.0"), &current, "linux-aarch64")
        else {
            panic!("0.3.0 is newer")
        };
        assert_eq!(asset, None);
    }

    #[test]
    fn only_https_or_this_machine() {
        for ok in [
            "https://spelunky.ai/releases/latest.json",
            "http://127.0.0.1:8000/latest.json",
            "http://localhost/x",
            "http://[::1]:80/x",
        ] {
            assert_eq!(check_url(ok), Ok(()), "{ok}");
        }
        for bad in [
            "http://spelunky.ai/x",
            "http://127.0.0.1.evil.com/x",
            "http://localhost.evil.com/x",
            "http://localhost:x@evil.com/x",
            "http://127.0.0.1@evil.com/x",
            "ftp://127.0.0.1/x",
            "file:///etc/passwd",
        ] {
            assert!(check_url(bad).is_err(), "{bad}");
        }
    }
}
