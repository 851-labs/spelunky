//! Updates from a fake older build to a fake newer one, end to end: a local server serves a signed
//! manifest and the newer build, and copies of the stand-in app (`src/bin/update-fixture.rs`) check,
//! download, verify, swap, restart and roll back as the app does, on this machine's kind of install
//! (a `.app` on macOS, an AppImage on Linux, a directory everywhere, which is Windows'). The saves in the
//! data directory are checked byte for byte afterwards.
//!
//! The only network is the server on 127.0.0.1.

use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use update::key;
use update::manifest::{self, Asset, Format, Manifest};

const FIXTURE: &str = env!("CARGO_BIN_EXE_update-fixture");
const TEST_KEY: &str = include_str!("../keys/test.key");

/// Serves `files` (by path) over HTTP on 127.0.0.1 until the test ends.
#[derive(Clone)]
struct Server {
    base: String,
    files: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
}

impl Server {
    fn start() -> Server {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let files: Arc<Mutex<BTreeMap<String, Vec<u8>>>> = Arc::default();
        let served = files.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut request = String::new();
                if reader.read_line(&mut request).is_err() {
                    continue;
                }
                // The headers, up to the blank line.
                let mut line = String::new();
                while reader.read_line(&mut line).is_ok_and(|n| n > 2) {
                    line.clear();
                }
                let path = request.split_whitespace().nth(1).unwrap_or("").to_owned();
                let body = served.lock().unwrap().get(&path).cloned();
                let _ = match body {
                    Some(body) => stream
                        .write_all(
                            format!(
                                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                body.len()
                            )
                            .as_bytes(),
                        )
                        .and_then(|()| stream.write_all(&body)),
                    None => stream.write_all(
                        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    ),
                };
            }
        });
        Server { base, files }
    }

    fn put(&self, path: &str, body: Vec<u8>) -> String {
        self.files.lock().unwrap().insert(path.to_owned(), body);
        format!("{}{path}", self.base)
    }
}

/// A fresh directory for one test.
fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("spelunky-e2e-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    // Canonical, as current_exe() reports it: macOS's /var is /private/var. (Windows' canonical paths are
    // verbatim \\?\ ones, which current_exe() doesn't give.)
    if cfg!(windows) {
        dir
    } else {
        dir.canonicalize().unwrap()
    }
}

fn exe_name() -> String {
    format!("spelunky{}", std::env::consts::EXE_SUFFIX)
}

/// A zip of `files` (path, contents), executables marked so.
fn zip(files: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut out = std::io::Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(&mut out);
    for (name, contents) in files {
        let options = zip::write::SimpleFileOptions::default().unix_permissions(0o755);
        zip.start_file(name.as_str(), options).unwrap();
        zip.write_all(contents).unwrap();
    }
    zip.finish().unwrap();
    out.into_inner()
}

/// Publishes `version` for `platform` as `download`, signed with `key`, and returns the manifest's URL.
fn publish(
    server: &Server,
    version: &str,
    platform: &str,
    format: Format,
    download: Vec<u8>,
    key: &ed25519_dalek::SigningKey,
) -> String {
    let sha = sha256(&download);
    let asset = Asset {
        url: String::new(),
        sha256: sha,
        size: download.len() as u64,
        format,
    };
    publish_asset(server, version, platform, asset, download, key)
}

fn publish_asset(
    server: &Server,
    version: &str,
    platform: &str,
    asset: Asset,
    download: Vec<u8>,
    key: &ed25519_dalek::SigningKey,
) -> String {
    let url = server.put(&format!("/download/v{version}/build"), download);
    let manifest = Manifest {
        version: version.into(),
        notes: format!("{}/notes/v{version}", server.base),
        assets: BTreeMap::from([(platform.to_owned(), Asset { url, ..asset })]),
    };
    server.put(
        "/releases/latest.json",
        manifest::sign(&manifest, key).into_bytes(),
    )
}

fn sha256(bytes: &[u8]) -> String {
    use sha2::Digest;
    key::to_hex(&sha2::Sha256::digest(bytes))
}

fn test_key() -> ed25519_dalek::SigningKey {
    key::signing_key(TEST_KEY).unwrap()
}

/// A data directory with saves in it, and a copy of what they were.
fn saves(dir: &Path) -> (PathBuf, BTreeMap<PathBuf, Vec<u8>>) {
    let data = dir.join("data");
    let files = [
        ("files/stats.txt", b"plays 12\ndeaths 11\n".to_vec()),
        ("files/settings.cfg", vec![1, 0, 255, 7]),
        ("files/levels/my level.lvl", b"custom level".to_vec()),
        ("highscores.txt", b"40000\t414c4558\n".to_vec()),
    ];
    let mut copy = BTreeMap::new();
    for (path, contents) in files {
        let path = data.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, &contents).unwrap();
        copy.insert(path, contents);
    }
    (data, copy)
}

fn assert_saves_untouched(saves: &BTreeMap<PathBuf, Vec<u8>>) {
    for (path, contents) in saves {
        assert_eq!(&fs::read(path).unwrap(), contents, "{}", path.display());
    }
}

/// Runs the installed fixture `exe` with `args` (an AppImage's path as `$APPIMAGE`), returning its output:
/// what it did, or the `error: ...` that stopped it.
fn run(exe: &Path, appimage: bool, args: &[&str]) -> String {
    let mut command = Command::new(exe);
    command.args(args);
    if appimage {
        command.env("APPIMAGE", exe);
    }
    let out = command.output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success() || (out.status.code() == Some(2) && text.starts_with("error: ")),
        "{} {args:?}: {}\n{text}{}",
        exe.display(),
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    text.trim().to_owned()
}

/// `update-fixture update ...` from `exe`: the player choosing "restart and update".
fn update(
    exe: &Path,
    appimage: bool,
    url: &str,
    platform: &str,
    data: &Path,
    report: &Path,
    dir: bool,
) -> String {
    let test_pub = Path::new(
        &std::env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR: run it with cargo"),
    )
    .join("keys/test.pub");
    let mut args = vec![
        "update",
        url,
        test_pub.to_str().unwrap(),
        platform,
        data.to_str().unwrap(),
        report.to_str().unwrap(),
        "5",
    ];
    if dir {
        args.push("--dir");
    }
    run(exe, appimage, &args)
}

/// What the version started after the update wrote: its version, once it has.
fn reported(report: &Path) -> String {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if let Ok(text) = fs::read_to_string(report)
            && !text.is_empty()
        {
            return text.trim().to_owned();
        }
        assert!(Instant::now() < deadline, "nothing started");
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// A directory install of `version`: the executable, `version.txt` and an asset.
fn dir_build(version: &str, asset: &str) -> Vec<(String, Vec<u8>)> {
    vec![
        (exe_name(), fs::read(FIXTURE).unwrap()),
        ("version.txt".into(), version.as_bytes().to_vec()),
        ("assets/data.txt".into(), asset.as_bytes().to_vec()),
    ]
}

fn install_dir(dir: &Path, build: &[(String, Vec<u8>)]) -> PathBuf {
    let root = dir.join("Spelunky Classic");
    for (name, contents) in build {
        let path = root.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, contents).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
    root.join(exe_name())
}

/// The newer build zipped as releases zip it: in a top-level folder.
fn dir_zip(build: &[(String, Vec<u8>)]) -> Vec<u8> {
    let in_folder: Vec<_> = build
        .iter()
        .map(|(name, c)| (format!("spelunky-classic/{name}"), c.clone()))
        .collect();
    zip(&in_folder)
}

#[test]
fn updates_a_directory_install() {
    let dir = temp("dir");
    let exe = install_dir(&dir, &dir_build("0.1.0", "old"));
    let (data, saved) = saves(&dir);
    let server = Server::start();
    let url = publish(
        &server,
        "0.2.0",
        "test",
        Format::DirZip,
        dir_zip(&dir_build("0.2.0", "new")),
        &test_key(),
    );
    let report = dir.join("report.txt");

    assert_eq!(
        update(&exe, false, &url, "test", &data, &report, true),
        "updated to 0.2.0"
    );
    assert_eq!(reported(&report), "0.2.0");
    assert_eq!(run(&exe, false, &["version"]), "0.2.0");
    let root = exe.parent().unwrap();
    assert_eq!(
        fs::read_to_string(root.join("assets/data.txt")).unwrap(),
        "new"
    );
    assert_saves_untouched(&saved);
    // The old version is gone with the work directory. (On Windows its .exe may still have been running,
    // waiting: the next start removes it.)
    if !cfg!(windows) {
        assert!(!root.join(".update").exists());
    }

    // Updated, it's up to date.
    assert_eq!(
        update(&exe, false, &url, "test", &data, &report, true),
        "up to date"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn rolls_back_a_new_version_that_fails_to_start() {
    for (how, why) in [
        ("broken", "quit before it started"),
        ("hangs", "didn't start within 5 seconds"),
    ] {
        let dir = temp(&format!("rollback-{how}"));
        let exe = install_dir(&dir, &dir_build("0.1.0", "old"));
        let (data, saved) = saves(&dir);
        let server = Server::start();
        let newer = dir_build(&format!("0.2.0 {how}"), "new");
        let url = publish(
            &server,
            "0.2.0",
            "test",
            Format::DirZip,
            dir_zip(&newer),
            &test_key(),
        );
        let report = dir.join("report.txt");

        let out = update(&exe, false, &url, "test", &data, &report, true);
        assert!(
            out.starts_with("rolled back from Some(\"0.2.0\")") && out.contains(why),
            "{out}"
        );
        // The old version is back, and started again.
        assert_eq!(reported(&report), "0.1.0");
        assert_eq!(run(&exe, false, &["version"]), "0.1.0");
        let root = exe.parent().unwrap();
        assert_eq!(
            fs::read_to_string(root.join("assets/data.txt")).unwrap(),
            "old"
        );
        assert_saves_untouched(&saved);
        let _ = fs::remove_dir_all(&dir);
    }
}

#[test]
fn installs_nothing_unverified_or_older() {
    let dir = temp("refused");
    let exe = install_dir(&dir, &dir_build("0.1.0", "old"));
    let (data, saved) = saves(&dir);
    let server = Server::start();
    let report = dir.join("report.txt");
    let newer = dir_zip(&dir_build("0.2.0", "new"));

    // A download that isn't what the manifest says.
    let mut asset = Asset {
        url: String::new(),
        sha256: sha256(b"something else"),
        size: newer.len() as u64,
        format: Format::DirZip,
    };
    let url = publish_asset(
        &server,
        "0.2.0",
        "test",
        asset.clone(),
        newer.clone(),
        &test_key(),
    );
    let out = update(&exe, false, &url, "test", &data, &report, true);
    assert!(out.contains("not the manifest's"), "{out}");
    asset.sha256 = sha256(&newer);
    asset.size -= 1;
    let url = publish_asset(&server, "0.2.0", "test", asset, newer.clone(), &test_key());
    let out = update(&exe, false, &url, "test", &data, &report, true);
    assert!(out.contains("bigger than the manifest's"), "{out}");

    // A manifest signed with another key.
    let other = key::generate().unwrap();
    let url = publish(
        &server,
        "0.2.0",
        "test",
        Format::DirZip,
        newer.clone(),
        &other,
    );
    let out = update(&exe, false, &url, "test", &data, &report, true);
    assert!(out.contains("isn't signed with the release key"), "{out}");

    // An older version, properly signed: never a downgrade.
    let older = dir_zip(&dir_build("0.0.9", "older"));
    let url = publish(&server, "0.0.9", "test", Format::DirZip, older, &test_key());
    assert_eq!(
        update(&exe, false, &url, "test", &data, &report, true),
        "up to date"
    );

    assert_eq!(run(&exe, false, &["version"]), "0.1.0");
    assert_eq!(
        fs::read_to_string(exe.parent().unwrap().join("assets/data.txt")).unwrap(),
        "old"
    );
    assert!(!report.exists());
    assert_saves_untouched(&saved);
    let _ = fs::remove_dir_all(&dir);
}

/// macOS: the whole `.app` is swapped.
#[cfg(target_os = "macos")]
#[test]
fn updates_a_mac_app() {
    let dir = temp("app");
    let app = dir.join("Applications/Spelunky Classic.app");
    let exe = app.join("Contents/MacOS/spelunky");
    let build = |version: &str| {
        vec![
            (
                "Contents/MacOS/spelunky".to_owned(),
                fs::read(FIXTURE).unwrap(),
            ),
            (
                "Contents/Resources/version.txt".to_owned(),
                version.as_bytes().to_vec(),
            ),
        ]
    };
    for (name, contents) in build("0.1.0") {
        let path = app.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, contents).unwrap();
    }
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&exe, fs::Permissions::from_mode(0o755)).unwrap();
    let (data, saved) = saves(&dir);
    let server = Server::start();
    let newer: Vec<_> = build("0.2.0")
        .into_iter()
        .map(|(name, c)| (format!("Spelunky Classic.app/{name}"), c))
        .collect();
    let url = publish(
        &server,
        "0.2.0",
        "macos",
        Format::AppZip,
        zip(&newer),
        &test_key(),
    );
    let report = dir.join("report.txt");

    assert_eq!(
        update(&exe, false, &url, "macos", &data, &report, false),
        "updated to 0.2.0"
    );
    assert_eq!(reported(&report), "0.2.0");
    assert_eq!(run(&exe, false, &["version"]), "0.2.0");
    assert!(
        !dir.join("Applications/.Spelunky Classic.app.update")
            .exists()
    );
    assert_saves_untouched(&saved);
    let _ = fs::remove_dir_all(&dir);
}

/// Linux: the AppImage is replaced.
#[cfg(target_os = "linux")]
#[test]
fn updates_an_appimage() {
    let dir = temp("appimage");
    let image = |version: &str| {
        let mut bytes = fs::read(FIXTURE).unwrap();
        bytes.extend_from_slice(format!("\nFIXTURE-VERSION={version}\n").as_bytes());
        bytes
    };
    let exe = dir.join("Spelunky_Classic-x86_64.AppImage");
    fs::write(&exe, image("0.1.0")).unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&exe, fs::Permissions::from_mode(0o755)).unwrap();
    let (data, saved) = saves(&dir);
    let server = Server::start();
    let report = dir.join("report.txt");

    // A broken one first: rolled back.
    let url = publish(
        &server,
        "0.2.0",
        "linux",
        Format::Appimage,
        image("0.2.0 broken"),
        &test_key(),
    );
    let out = update(&exe, true, &url, "linux", &data, &report, false);
    assert!(out.starts_with("rolled back"), "{out}");
    assert_eq!(reported(&report), "0.1.0");
    fs::remove_file(&report).unwrap();

    let url = publish(
        &server,
        "0.2.0",
        "linux",
        Format::Appimage,
        image("0.2.0"),
        &test_key(),
    );
    assert_eq!(
        update(&exe, true, &url, "linux", &data, &report, false),
        "updated to 0.2.0"
    );
    assert_eq!(reported(&report), "0.2.0");
    assert_eq!(run(&exe, true, &["version"]), "0.2.0");
    assert_saves_untouched(&saved);
    let _ = fs::remove_dir_all(&dir);
}
