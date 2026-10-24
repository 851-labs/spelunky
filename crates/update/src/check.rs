//! A check for updates, start to finish: fetch and verify the manifest, and if it offers a newer version
//! this install can take, download it, check it and unpack it next to the install, ready to swap in.
//! [`spawn`] runs one on a thread, for the app, which only asks the player once it's [`Found::Ready`].

use crate::install::{Install, Staged};
use crate::manifest::{self, Offer};
use crate::version::Version;
use ed25519_dalek::VerifyingKey;
use std::sync::mpsc::{Receiver, channel};

/// Where and what to check.
#[derive(Clone, Debug)]
pub struct Source {
    pub manifest_url: String,
    pub key: VerifyingKey,
    /// The version running.
    pub current: Version,
    /// Its [`crate::platform`].
    pub platform: String,
    /// How it's installed, or why it can't be updated in place (a development build, say).
    pub install: Result<Install, String>,
}

/// A newer version, downloaded, verified and unpacked.
#[derive(Clone, Debug)]
pub struct Ready {
    pub version: Version,
    pub notes: String,
    pub install: Install,
    pub staged: Staged,
}

#[derive(Debug)]
pub enum Found {
    UpToDate,
    /// A newer version this install can't take in place, and why: the player can download it.
    Newer {
        version: Version,
        notes: String,
        why_not: String,
    },
    Ready(Ready),
}

/// How a check is going.
#[derive(Debug)]
pub enum Progress {
    /// Downloading `version`: `got` of `size` bytes.
    Downloading {
        version: Version,
        got: u64,
        size: u64,
    },
}

/// Checks, and downloads what it finds. `skip` (a version the player skipped) counts as up to date.
pub fn check(
    source: &Source,
    skip: Option<&Version>,
    mut progress: impl FnMut(Progress),
) -> Result<Found, String> {
    let text = crate::fetch::manifest(&source.manifest_url)?;
    let manifest = manifest::open(&text, &source.key).map_err(|e| e.to_string())?;
    let (version, notes, asset) =
        match manifest::offer(&manifest, &source.current, &source.platform)? {
            Offer::UpToDate => return Ok(Found::UpToDate),
            Offer::Newer { version, .. } if Some(&version) == skip => return Ok(Found::UpToDate),
            Offer::Newer {
                version,
                notes,
                asset,
            } => (version, notes, asset),
        };
    let newer = |why_not: String| {
        Ok(Found::Newer {
            version: version.clone(),
            notes: notes.clone(),
            why_not,
        })
    };
    let install = match &source.install {
        Ok(install) => install,
        Err(e) => return newer(format!("this copy can't update itself: {e}")),
    };
    let Some(asset) = asset else {
        return newer(format!("the release has no {} build", source.platform));
    };
    if asset.format != install.kind.format() {
        return newer(format!(
            "the release's {} build is a {:?}, not what this copy was installed from",
            source.platform, asset.format
        ));
    }
    // An install the player can't write to (Program Files, or /Applications without being an admin) can
    // still be told where to get the new version.
    let download = match install.prepare() {
        Ok(download) => download,
        Err(e) => return newer(e),
    };
    let size = asset.size;
    crate::fetch::asset(&asset, &download, |got| {
        progress(Progress::Downloading {
            version: version.clone(),
            got,
            size,
        })
    })?;
    let staged = install.stage(&download)?;
    Ok(Found::Ready(Ready {
        version,
        notes,
        install: install.clone(),
        staged,
    }))
}

/// What a check on a thread sends.
#[derive(Debug)]
pub enum Message {
    Progress(Progress),
    Done(Result<Found, String>),
}

/// Runs [`check`] on a thread, which sends its progress and then its result.
pub fn spawn(source: Source, skip: Option<Version>) -> Receiver<Message> {
    let (send, receive) = channel();
    std::thread::spawn(move || {
        let result = check(&source, skip.as_ref(), |p| {
            let _ = send.send(Message::Progress(p));
        });
        let _ = send.send(Message::Done(result));
    });
    receive
}
