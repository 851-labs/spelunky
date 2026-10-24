//! The ed25519 keys that sign update manifests.
//!
//! The release key's private half exists only as the `release` environment's `UPDATE_SIGNING_KEY` secret
//! (and its backup); its public half is `keys/release.pub`, compiled into the app. Until that file holds a
//! real key it's a placeholder, and the app doesn't update at all ([`release_key`] is `None`).
//!
//! `keys/test.key` and `keys/test.pub` are a published keypair for the tests only: nothing that ships
//! trusts it.
//!
//! A key file is one line of hex (the 32-byte public key, or the 32-byte private seed), after any `#`
//! comment lines.

use ed25519_dalek::{SigningKey, VerifyingKey};

/// `keys/release.pub`, as committed.
pub const RELEASE_PUBLIC_KEY_FILE: &str = include_str!("../keys/release.pub");

/// The release key the app trusts; `None` while `keys/release.pub` is the placeholder.
pub fn release_key() -> Option<VerifyingKey> {
    public_key(RELEASE_PUBLIC_KEY_FILE).ok()
}

/// A public key file's key.
pub fn public_key(file: &str) -> Result<VerifyingKey, String> {
    let bytes = key_bytes(file)?;
    VerifyingKey::from_bytes(&bytes).map_err(|e| format!("not an ed25519 public key: {e}"))
}

/// A private key (a file's, or the `UPDATE_SIGNING_KEY` secret's): the 32-byte seed in hex.
pub fn signing_key(file: &str) -> Result<SigningKey, String> {
    Ok(SigningKey::from_bytes(&key_bytes(file)?))
}

/// A fresh signing key, from the OS's randomness.
pub fn generate() -> Result<SigningKey, String> {
    let mut seed = [0; 32];
    getrandom::fill(&mut seed).map_err(|e| format!("no randomness: {e}"))?;
    Ok(SigningKey::from_bytes(&seed))
}

/// The hex of the key's bytes, as key files hold them.
pub fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn key_bytes(file: &str) -> Result<[u8; 32], String> {
    let line = file
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with('#'))
        .ok_or("no key in the file")?;
    let bytes = from_hex(line).ok_or_else(|| format!("{line:?} isn't hex"))?;
    bytes
        .try_into()
        .map_err(|b: Vec<u8>| format!("a key is 32 bytes, not {}", b.len()))
}

/// Lower- or upper-case hex, an even number of digits.
pub fn from_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok())
        .collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub const TEST_KEY: &str = include_str!("../keys/test.key");
    pub const TEST_PUB: &str = include_str!("../keys/test.pub");

    #[test]
    fn the_test_keypair_is_a_pair() {
        let key = signing_key(TEST_KEY).unwrap();
        assert_eq!(key.verifying_key(), public_key(TEST_PUB).unwrap());
    }

    #[test]
    fn the_app_never_trusts_the_test_key() {
        let test = public_key(TEST_PUB).unwrap();
        assert_ne!(release_key(), Some(test));
    }

    #[test]
    fn the_placeholder_is_no_key() {
        assert!(public_key("# PLACEHOLDER\nPLACEHOLDER\n").is_err());
        assert!(public_key(&format!("# comment\n{}\n", "00".repeat(31))).is_err());
    }
}
