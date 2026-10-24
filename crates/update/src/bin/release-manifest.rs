//! The release side of the updater (`scripts/release/manifest` runs it; `docs/updates.md`):
//!
//! ```sh
//! release-manifest sign TAG PLATFORM=FILE... [--out latest.json] [--public-key FILE] [--base-url URL]
//! release-manifest verify latest.json [--public-key FILE]
//! release-manifest keygen PRIVATE-KEY-FILE
//! ```
//!
//! `sign` makes a release's `latest.json`: the version is TAG without its `v`, each PLATFORM's download
//! is FILE as the GitHub Release `TAG` serves it (its name, size and SHA-256), and the signing key is the
//! `UPDATE_SIGNING_KEY` environment variable (the private seed, in hex). The result is checked against the
//! public key the app trusts (`crates/update/keys/release.pub`, or `--public-key`), so a release can't
//! publish a manifest the apps would refuse. `--base-url` serves the downloads from somewhere else than the
//! GitHub Release (a local server, to rehearse an update with a debug build).
//!
//! `verify` checks a `latest.json` and prints its manifest. `keygen` makes the release key, once: the
//! private seed goes to a new file (readable only by its owner), and the public key file's contents are
//! printed, for `crates/update/keys/release.pub`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use update::key;
use update::manifest::{self, Asset, Format, Manifest};

const REPO: &str = "https://github.com/851-labs/spelunky";

const USAGE: &str =
    "usage: release-manifest sign TAG PLATFORM=FILE... [--out PATH] [--public-key FILE] [--base-url URL]
       release-manifest verify PATH [--public-key FILE]
       release-manifest keygen PRIVATE-KEY-FILE";

fn main() {
    if let Err(e) = run(std::env::args().skip(1).collect()) {
        eprintln!("release-manifest: {e}");
        std::process::exit(1);
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let mut positional = Vec::new();
    let mut out = None;
    let mut public_key = None;
    let mut base_url = None;
    let mut it = args.into_iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--out" => out = Some(PathBuf::from(it.next().ok_or("--out needs a path")?)),
            "--public-key" => {
                public_key = Some(PathBuf::from(it.next().ok_or("--public-key needs a file")?))
            }
            "--base-url" => base_url = Some(it.next().ok_or("--base-url needs a URL")?),
            "-h" | "--help" => return Err(USAGE.into()),
            _ => positional.push(arg),
        }
    }
    let trusted = || match &public_key {
        Some(path) => key::public_key(&read(path)?).map_err(|e| format!("{}: {e}", path.display())),
        None => key::release_key().ok_or_else(|| {
            "crates/update/keys/release.pub is still the placeholder, so no app would accept a \
             manifest: make the release key first (docs/updates.md)"
                .to_owned()
        }),
    };
    match positional.split_first() {
        Some((cmd, [tag, assets @ ..])) if cmd == "sign" && !assets.is_empty() => {
            let base = base_url.unwrap_or_else(|| format!("{REPO}/releases/download/{tag}"));
            let manifest = manifest_for(tag, assets, &base)?;
            let secret = std::env::var("UPDATE_SIGNING_KEY").map_err(
                |_| "UPDATE_SIGNING_KEY (the release key's private seed, in hex) isn't set",
            )?;
            let signing =
                key::signing_key(&secret).map_err(|e| format!("UPDATE_SIGNING_KEY: {e}"))?;
            let text = manifest::sign(&manifest, &signing);
            manifest::open(&text, &trusted()?)
                .map_err(|e| format!("UPDATE_SIGNING_KEY isn't the key the app trusts ({e})"))?;
            match out {
                Some(path) => {
                    std::fs::write(&path, &text).map_err(|e| format!("{}: {e}", path.display()))?;
                    eprintln!(
                        "release-manifest: {} signed for {}",
                        path.display(),
                        manifest.version
                    );
                }
                None => print!("{text}"),
            }
            Ok(())
        }
        Some((cmd, [path])) if cmd == "verify" => {
            let manifest = manifest::open(&read(Path::new(path))?, &trusted()?)
                .map_err(|e| format!("{path}: {e}"))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&manifest).expect("a manifest serializes")
            );
            Ok(())
        }
        Some((cmd, [path])) if cmd == "keygen" => keygen(Path::new(path)),
        _ => Err(USAGE.into()),
    }
}

fn read(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

/// The manifest for release `tag`, with `PLATFORM=FILE` downloads.
fn manifest_for(tag: &str, assets: &[String], base: &str) -> Result<Manifest, String> {
    let version = update::Version::parse(tag)?;
    if !tag.starts_with('v') {
        return Err(format!("{tag} isn't a release tag (vMAJOR.MINOR.PATCH)"));
    }
    let mut out = BTreeMap::new();
    for spec in assets {
        let (platform, file) = spec
            .split_once('=')
            .ok_or_else(|| format!("{spec} isn't PLATFORM=FILE"))?;
        let format = match platform {
            "macos" => Format::AppZip,
            "windows-x86_64" | "windows-aarch64" => Format::DirZip,
            "linux-x86_64" | "linux-aarch64" => Format::Appimage,
            _ => {
                return Err(format!(
                    "{platform} isn't a platform: macos, windows-x86_64, windows-aarch64, linux-x86_64 \
                     or linux-aarch64"
                ));
            }
        };
        let path = Path::new(file);
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| format!("{file} has no file name"))?;
        let extension_ok = match format {
            Format::AppZip | Format::DirZip => name.ends_with(".zip"),
            Format::Appimage => name.ends_with(".AppImage"),
        };
        if !extension_ok {
            return Err(format!(
                "{platform}'s download is a {format:?}, which {name} isn't"
            ));
        }
        // GitHub serves a release asset under its name with spaces as dots; names without them are safe.
        if !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-+".contains(&b))
        {
            return Err(format!(
                "{name}: name release assets with letters, digits and ._-+ only"
            ));
        }
        let size = std::fs::metadata(path)
            .map_err(|e| format!("{file}: {e}"))?
            .len();
        let previous = out.insert(
            platform.to_owned(),
            Asset {
                url: format!("{}/{name}", base.trim_end_matches('/')),
                sha256: update::fetch::sha256_file(path)?,
                size,
                format,
            },
        );
        if previous.is_some() {
            return Err(format!("{platform} is given twice"));
        }
    }
    Ok(Manifest {
        version: version.to_string(),
        notes: format!("{REPO}/releases/tag/{tag}"),
        assets: out,
    })
}

fn keygen(private: &Path) -> Result<(), String> {
    let signing = key::generate()?;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(private)
        .map_err(|e| format!("{}: {e}", private.display()))?;
    std::io::Write::write_all(
        &mut file,
        format!("{}\n", key::to_hex(&signing.to_bytes())).as_bytes(),
    )
    .map_err(|e| format!("{}: {e}", private.display()))?;
    eprintln!(
        "release-manifest: the private key is in {}: store it as UPDATE_SIGNING_KEY, back it up, and \
         delete the file. crates/update/keys/release.pub becomes:\n",
        private.display()
    );
    println!(
        "# The release key's public half: every update manifest must be signed with it (docs/updates.md).\n{}",
        key::to_hex(signing.verifying_key().as_bytes())
    );
    Ok(())
}
