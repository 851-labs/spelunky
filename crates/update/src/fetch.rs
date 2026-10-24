//! Downloading: the manifest, then the asset it names, checked against its size and SHA-256 as it
//! arrives.

use crate::manifest::{Asset, check_url};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;

/// The most a manifest may be: it names a handful of downloads.
const MAX_MANIFEST: u64 = 64 * 1024;

fn agent(url: &str, timeout: Duration) -> Result<ureq::Agent, String> {
    check_url(url)?;
    Ok(ureq::Agent::config_builder()
        // An HTTPS fetch may only be redirected to HTTPS (GitHub's downloads redirect to its CDN).
        .https_only(url.starts_with("https://"))
        .timeout_global(Some(timeout))
        .user_agent(concat!(
            "spelunky-classic-updater/",
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .new_agent())
}

/// `latest.json`'s text.
pub fn manifest(url: &str) -> Result<String, String> {
    let error = |e: ureq::Error| format!("{url}: {e}");
    let mut response = agent(url, Duration::from_secs(20))?
        .get(url)
        .call()
        .map_err(error)?;
    response
        .body_mut()
        .with_config()
        .limit(MAX_MANIFEST)
        .read_to_string()
        .map_err(error)
}

/// Downloads `asset` to `to`, failing (and removing `to`) unless it's exactly the manifest's size and
/// SHA-256. `progress` gets the bytes so far.
pub fn asset(asset: &Asset, to: &Path, mut progress: impl FnMut(u64)) -> Result<(), String> {
    let result = (|| {
        let url = &asset.url;
        let error = |e: ureq::Error| format!("{url}: {e}");
        // Generous: a slow connection takes minutes for a build.
        let mut response = agent(url, Duration::from_secs(30 * 60))?
            .get(url)
            .call()
            .map_err(error)?;
        // One byte over the size is enough to know it's wrong.
        let mut body = response
            .body_mut()
            .with_config()
            .limit(asset.size + 1)
            .reader();
        let mut file = File::create(to).map_err(|e| format!("{}: {e}", to.display()))?;
        let mut hash = Sha256::new();
        let mut got = 0u64;
        let mut buf = vec![0; 64 * 1024];
        loop {
            let n = match body.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(format!("{url}: {e}")),
            };
            got += n as u64;
            if got > asset.size {
                return Err(format!(
                    "{url} is bigger than the manifest's {} bytes",
                    asset.size
                ));
            }
            hash.update(&buf[..n]);
            file.write_all(&buf[..n])
                .map_err(|e| format!("{}: {e}", to.display()))?;
            progress(got);
        }
        file.sync_all()
            .map_err(|e| format!("{}: {e}", to.display()))?;
        if got != asset.size {
            return Err(format!(
                "{url} is {got} bytes, not the manifest's {}",
                asset.size
            ));
        }
        let sha256 = crate::key::to_hex(&hash.finalize());
        if sha256 != asset.sha256 {
            return Err(format!(
                "{url} has SHA-256 {sha256}, not the manifest's {}",
                asset.sha256
            ));
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(to);
    }
    result
}

/// A file's SHA-256, in hex.
pub fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut hash = Sha256::new();
    let mut buf = vec![0; 64 * 1024];
    loop {
        match file.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => hash.update(&buf[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(format!("{}: {e}", path.display())),
        }
    }
    Ok(crate::key::to_hex(&hash.finalize()))
}
