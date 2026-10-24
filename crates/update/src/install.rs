//! Putting a new version in place of the running one, and back again.
//!
//! What gets replaced depends on how the app is installed ([`Kind`]):
//!
//! - **macOS**: the whole `.app` bundle, in one rename. The running app keeps running from the bundle
//!   moved aside (a running program is its open files, not its path), and the new bundle is the release's
//!   signed and notarized one, untouched.
//! - **Windows**: the install directory's files, each one the update has. Windows won't let a running
//!   program's `.exe` be replaced or deleted, but it can be renamed, so each old file is first moved aside.
//! - **Linux**: the AppImage, one file, in one rename.
//!
//! Everything happens in a work directory next to what's replaced ([`Install::work_dir`]), on the same
//! file system, so every step is a rename: `new/` holds the unpacked download, `old/` what it replaced.
//! A swap that fails part-way, or a new version that fails to start ([`crate::restart`]), is undone from
//! `old/`. The work directory goes once the new version has started ([`Install::clean_up`]).
//!
//! Nothing here touches the per-user data directory, where the saves are.

use crate::manifest::Format;
use std::fs::{self, File};
use std::io;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A macOS `.app` bundle.
    Bundle,
    /// A directory holding the executable and whatever ships beside it.
    Dir,
    /// One file that is the whole app: an AppImage.
    File,
}

impl Kind {
    /// The download format that installs this kind.
    pub fn format(self) -> Format {
        match self {
            Kind::Bundle => Format::AppZip,
            Kind::Dir => Format::DirZip,
            Kind::File => Format::Appimage,
        }
    }
}

/// An installed app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Install {
    pub kind: Kind,
    /// What an update replaces: the `.app`, the directory, or the AppImage.
    pub target: PathBuf,
    /// What to run to start it: inside the bundle or directory, or the AppImage.
    pub exe: PathBuf,
}

/// A download unpacked in the work directory, ready to [`apply`](Install::apply).
#[derive(Clone, Debug)]
pub struct Staged {
    /// The new `.app`, directory or AppImage.
    root: PathBuf,
}

/// A swap done, with what it takes to undo it.
#[derive(Debug)]
pub struct Applied {
    /// For a directory: each path the update put in place, and whether it replaced a file then in `old/`.
    files: Vec<(PathBuf, bool)>,
    /// For a directory: the directories made for new files, outermost first.
    dirs: Vec<PathBuf>,
}

fn error(path: &Path) -> impl Fn(io::Error) -> String + '_ {
    move |e| format!("{}: {e}", path.display())
}

fn file_name(path: &Path) -> Result<&std::ffi::OsStr, String> {
    path.file_name()
        .ok_or_else(|| format!("{} has no file name", path.display()))
}

impl Install {
    /// How the app running from `exe` (`std::env::current_exe()`) is installed: a bundle if `exe` is one's
    /// executable (macOS), the AppImage `appimage` (Linux: the runtime's `$APPIMAGE`), otherwise the
    /// directory `exe` is in.
    pub fn detect(exe: &Path, appimage: Option<&Path>) -> Result<Install, String> {
        if let Some(image) = appimage {
            return Ok(Install {
                kind: Kind::File,
                target: image.to_owned(),
                exe: image.to_owned(),
            });
        }
        let dir = exe
            .parent()
            .ok_or_else(|| format!("{} has no directory", exe.display()))?;
        if cfg!(target_os = "macos") {
            // Contents/MacOS/<exe> in a .app.
            let bundle = dir
                .parent()
                .filter(|_| dir.file_name().is_some_and(|n| n == "MacOS"))
                .filter(|contents| contents.file_name().is_some_and(|n| n == "Contents"))
                .and_then(Path::parent)
                .filter(|app| app.extension().is_some_and(|e| e == "app"));
            let Some(bundle) = bundle else {
                return Err(format!("{} isn't in a .app bundle", exe.display()));
            };
            // Gatekeeper runs a quarantined app that hasn't been moved from where it was unpacked from a
            // read-only, randomized copy ("App Translocation"), which can't be updated in place.
            if bundle
                .components()
                .any(|c| c.as_os_str() == "AppTranslocation")
            {
                return Err(
                    "it runs from a read-only copy macOS made (App Translocation): move it to the \
                     Applications folder"
                        .into(),
                );
            }
            return Ok(Install {
                kind: Kind::Bundle,
                target: bundle.to_owned(),
                exe: exe.to_owned(),
            });
        }
        Ok(Install {
            kind: Kind::Dir,
            target: dir.to_owned(),
            exe: exe.to_owned(),
        })
    }

    /// Where updates are unpacked and old versions kept until the new one starts: inside the install
    /// directory, or next to the bundle or AppImage, so moving between them is a rename.
    pub fn work_dir(&self) -> PathBuf {
        match self.kind {
            Kind::Dir => self.target.join(".update"),
            Kind::Bundle | Kind::File => {
                let name = self
                    .target
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy();
                self.target.with_file_name(format!(".{name}.update"))
            }
        }
    }

    /// Makes a fresh work directory (dropping any earlier one's leftovers) and returns where to download
    /// to. Fails if the install can't be written to, before anything is downloaded.
    pub fn prepare(&self) -> Result<PathBuf, String> {
        let work = self.work_dir();
        remove(&work)?;
        fs::create_dir_all(&work).map_err(|e| {
            format!(
                "can't update in place: {} isn't writable ({e})",
                work.parent().unwrap_or(&work).display()
            )
        })?;
        Ok(work.join("download"))
    }

    /// Unpacks the download (from [`prepare`](Self::prepare)'s path) into `new/`, checking it's a build
    /// of this app: it must hold an executable named as the running one, where it is now.
    pub fn stage(&self, download: &Path) -> Result<Staged, String> {
        let new = self.work_dir().join("new");
        remove(&new)?;
        fs::create_dir_all(&new).map_err(error(&new))?;
        let exe_name = file_name(&self.exe)?;
        let root = match self.kind {
            Kind::File => {
                let root = new.join(file_name(&self.target)?);
                fs::rename(download, &root).map_err(error(&root))?;
                make_executable(&root)?;
                root
            }
            Kind::Bundle => {
                unzip(download, &new)?;
                let apps = entries(&new)?;
                let [app] = apps.as_slice() else {
                    return Err("the update isn't one .app".into());
                };
                if app.extension().is_none_or(|e| e != "app") {
                    return Err("the update isn't one .app".into());
                }
                if !app.join("Contents/MacOS").join(exe_name).is_file() {
                    return Err(format!(
                        "the update's app has no Contents/MacOS/{}",
                        exe_name.to_string_lossy()
                    ));
                }
                app.clone()
            }
            Kind::Dir => {
                unzip(download, &new)?;
                // A zip of the folder, not its contents, is the same build.
                let top = entries(&new)?;
                let root = match top.as_slice() {
                    [only] if only.is_dir() => only.clone(),
                    _ => new.clone(),
                };
                if !root.join(exe_name).is_file() {
                    return Err(format!("the update has no {}", exe_name.to_string_lossy()));
                }
                root
            }
        };
        let _ = fs::remove_file(download);
        Ok(Staged { root })
    }

    /// Puts the staged version in place, the replaced files going to `old/`. If any step fails, what was
    /// done is undone.
    pub fn apply(&self, staged: &Staged) -> Result<Applied, String> {
        let old = self.work_dir().join("old");
        remove(&old)?;
        fs::create_dir_all(&old).map_err(error(&old))?;
        match self.kind {
            Kind::Bundle | Kind::File => {
                let aside = old.join(file_name(&self.target)?);
                fs::rename(&self.target, &aside).map_err(error(&self.target))?;
                if let Err(e) = fs::rename(&staged.root, &self.target) {
                    let undone = fs::rename(&aside, &self.target);
                    return Err(format!(
                        "{}: {e}{}",
                        self.target.display(),
                        undone
                            .err()
                            .map(|e| format!(" (and putting it back: {e})"))
                            .unwrap_or_default()
                    ));
                }
                Ok(Applied {
                    files: Vec::new(),
                    dirs: Vec::new(),
                })
            }
            Kind::Dir => {
                let mut files = Vec::new();
                walk(&staged.root, Path::new(""), &mut files)?;
                // Check first that every file goes where a file (or nothing) is, so the swap doesn't stop
                // half-way on something it could have seen coming.
                for rel in &files {
                    let to = self.target.join(rel);
                    if to.is_dir() && !to.is_symlink() {
                        return Err(format!("{} is a directory", to.display()));
                    }
                }
                let mut applied = Applied {
                    files: Vec::new(),
                    dirs: Vec::new(),
                };
                for rel in files {
                    if let Err(e) = self.replace(&staged.root, &old, &rel, &mut applied) {
                        let undone = self.roll_back(applied);
                        return Err(format!(
                            "{e}{}",
                            undone
                                .err()
                                .map(|e| format!(" (and undoing it: {e})"))
                                .unwrap_or_default()
                        ));
                    }
                }
                Ok(applied)
            }
        }
    }

    /// Moves `rel`'s current file (if any) to `old/` and the staged one into its place.
    fn replace(
        &self,
        new: &Path,
        old: &Path,
        rel: &Path,
        applied: &mut Applied,
    ) -> Result<(), String> {
        let to = self.target.join(rel);
        let existed = to.symlink_metadata().is_ok();
        if existed {
            let aside = old.join(rel);
            create_parent(&aside)?;
            // Windows lets a running .exe be renamed, not replaced.
            fs::rename(&to, &aside).map_err(error(&to))?;
        } else if let Some(parent) = rel.parent() {
            // Made one level at a time, so a rollback can take away exactly these.
            let mut dir = PathBuf::new();
            for part in parent.components() {
                dir.push(part);
                let at = self.target.join(&dir);
                if !at.exists() {
                    fs::create_dir(&at).map_err(error(&at))?;
                    applied.dirs.push(dir.clone());
                }
            }
        }
        if let Err(e) = fs::rename(new.join(rel), &to) {
            if existed {
                let _ = fs::rename(old.join(rel), &to);
            }
            return Err(format!("{}: {e}", to.display()));
        }
        applied.files.push((rel.to_owned(), existed));
        Ok(())
    }

    /// Undoes [`apply`](Self::apply): the replaced version back in place, what replaced it gone. The new
    /// version must have exited: Windows won't delete a running program's files.
    pub fn roll_back(&self, applied: Applied) -> Result<(), String> {
        let work = self.work_dir();
        let old = work.join("old");
        match self.kind {
            Kind::Bundle | Kind::File => {
                let failed = work.join("failed");
                remove(&failed)?;
                fs::create_dir_all(&failed).map_err(error(&failed))?;
                let name = file_name(&self.target)?;
                retry(|| fs::rename(&self.target, failed.join(name)))
                    .map_err(error(&self.target))?;
                fs::rename(old.join(name), &self.target).map_err(error(&self.target))?;
                let _ = remove(&failed);
            }
            Kind::Dir => {
                // Every file goes back, even if one fails, and the first failure is reported.
                let mut first = Ok(());
                for (rel, existed) in applied.files.into_iter().rev() {
                    let to = self.target.join(&rel);
                    let mut step = retry(|| fs::remove_file(&to)).map_err(error(&to));
                    if step.is_ok() && existed {
                        step = fs::rename(old.join(&rel), &to).map_err(error(&to));
                    }
                    if first.is_ok() {
                        first = step;
                    }
                }
                for dir in applied.dirs.iter().rev() {
                    let _ = fs::remove_dir(self.target.join(dir));
                }
                first?;
            }
        }
        Ok(())
    }

    /// Removes the work directory: the download, and the old version once the new one has started. Best
    /// effort: on Windows the old `.exe` can't go while the old version still runs (it waits for the new
    /// one to start), and the next start tries again.
    pub fn clean_up(&self) {
        let _ = remove(&self.work_dir());
    }
}

/// Windows can hold a program's files for a moment after it exits.
fn retry(mut f: impl FnMut() -> io::Result<()>) -> io::Result<()> {
    let mut tries = 0;
    loop {
        match f() {
            Err(e) if cfg!(windows) && tries < 20 => {
                tries += 1;
                let _ = e;
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            result => return result,
        }
    }
}

fn remove(path: &Path) -> Result<(), String> {
    match fs::remove_dir_all(path) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(format!("{}: {e}", path.display())),
        _ => Ok(()),
    }
}

fn create_parent(path: &Path) -> Result<(), String> {
    match path.parent() {
        Some(dir) => fs::create_dir_all(dir).map_err(error(dir)),
        None => Ok(()),
    }
}

/// A directory's entries, sorted.
fn entries(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out: Vec<PathBuf> = fs::read_dir(dir)
        .map_err(error(dir))?
        .map(|e| e.map(|e| e.path()).map_err(error(dir)))
        .collect::<Result<_, _>>()?;
    out.sort();
    Ok(out)
}

/// Every file and symlink under `dir/rel`, relative to `dir`.
fn walk(dir: &Path, rel: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for path in entries(&dir.join(rel))? {
        let rel = rel.join(file_name(&path)?);
        let kind = path.symlink_metadata().map_err(error(&path))?.file_type();
        if kind.is_dir() {
            walk(dir, &rel, out)?;
        } else {
            out.push(rel);
        }
    }
    Ok(())
}

/// Unpacks a zip into `into`, keeping Unix permissions and symlinks (a bundle's frameworks have them).
/// Entries that would land outside `into` are refused, and so are links that could lead there: a link's
/// target must be relative without `..`, and nothing is unpacked through a link.
pub fn unzip(zip: &Path, into: &Path) -> Result<(), String> {
    let file = File::open(zip).map_err(error(zip))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("{}: {e}", zip.display()))?;
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("{}: {e}", zip.display()))?;
        let rel = entry
            .enclosed_name()
            .filter(|p| p.components().all(|c| matches!(c, Component::Normal(_))))
            .ok_or_else(|| format!("{}: {:?} is outside the zip", zip.display(), entry.name()))?;
        let to = into.join(&rel);
        // Through an earlier entry's link, a later one could land anywhere.
        let mut at = into.to_owned();
        for part in rel.components() {
            at.push(part);
            if at
                .symlink_metadata()
                .is_ok_and(|m| m.file_type().is_symlink())
            {
                return Err(format!(
                    "{}: {} goes through a link",
                    zip.display(),
                    rel.display()
                ));
            }
        }
        if entry.is_dir() {
            fs::create_dir_all(&to).map_err(error(&to))?;
            continue;
        }
        create_parent(&to)?;
        if entry.is_symlink() {
            let mut target = String::new();
            io::Read::read_to_string(&mut entry, &mut target).map_err(error(&to))?;
            // Only links that stay inside what's unpacked, which frameworks' are (`Versions/Current`).
            let target_path = Path::new(&target);
            if target_path.is_absolute()
                || !target_path
                    .components()
                    .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
            {
                return Err(format!(
                    "{}: {} links outside the zip",
                    zip.display(),
                    rel.display()
                ));
            }
            symlink(&target, &to)?;
            continue;
        }
        let mut out = File::create(&to).map_err(error(&to))?;
        io::copy(&mut entry, &mut out).map_err(error(&to))?;
        #[cfg(unix)]
        if let Some(mode) = entry.unix_mode() {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&to, fs::Permissions::from_mode(mode & 0o777))
                .map_err(error(&to))?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn symlink(target: &str, at: &Path) -> Result<(), String> {
    std::os::unix::fs::symlink(target, at).map_err(error(at))
}

#[cfg(not(unix))]
fn symlink(_: &str, at: &Path) -> Result<(), String> {
    Err(format!(
        "{}: a symlink, which only macOS builds have",
        at.display()
    ))
}

fn make_executable(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).map_err(error(path))?;
    }
    let _ = path;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("spelunky-install-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn zip_of(path: &Path, files: &[(&str, &str)]) {
        let mut zip = zip::ZipWriter::new(File::create(path).unwrap());
        let options = zip::write::SimpleFileOptions::default().unix_permissions(0o755);
        for (name, contents) in files {
            zip.start_file(*name, options).unwrap();
            zip.write_all(contents.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }

    fn read(path: &Path) -> String {
        fs::read_to_string(path).unwrap()
    }

    #[test]
    fn a_directory_update_replaces_its_files_and_rolls_back() {
        let dir = temp("dir");
        let install_dir = dir.join("Spelunky");
        fs::create_dir_all(install_dir.join("assets")).unwrap();
        fs::write(install_dir.join("app.exe"), "old exe").unwrap();
        fs::write(install_dir.join("assets/a.png"), "old a").unwrap();
        fs::write(install_dir.join("only-old.txt"), "stays").unwrap();
        let exe = install_dir.join("app.exe");
        // Off macOS, an executable outside a bundle is a directory install; on macOS it's not an install
        // at all, but the directory kind works there when asked for (as the end-to-end test does).
        if !cfg!(target_os = "macos") {
            assert_eq!(Install::detect(&exe, None).unwrap().kind, Kind::Dir);
        }
        let install = Install {
            kind: Kind::Dir,
            target: install_dir.clone(),
            exe,
        };
        let download = install.prepare().unwrap();
        zip_of(
            &download,
            &[
                ("Spelunky/app.exe", "new exe"),
                ("Spelunky/assets/a.png", "new a"),
                ("Spelunky/assets/b.png", "new b"),
                ("Spelunky/new-dir/deeper/c.png", "new c"),
            ],
        );
        let staged = install.stage(&download).unwrap();
        let applied = install.apply(&staged).unwrap();
        assert_eq!(read(&install_dir.join("app.exe")), "new exe");
        assert_eq!(read(&install_dir.join("assets/b.png")), "new b");
        assert_eq!(read(&install_dir.join("only-old.txt")), "stays");
        install.roll_back(applied).unwrap();
        assert_eq!(read(&install_dir.join("app.exe")), "old exe");
        assert_eq!(read(&install_dir.join("assets/a.png")), "old a");
        assert!(!install_dir.join("assets/b.png").exists());
        assert!(!install_dir.join("new-dir").exists());
        install.clean_up();
        assert!(!install.work_dir().exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_download_without_the_app_isnt_staged() {
        let dir = temp("wrong");
        fs::write(dir.join("app.exe"), "old").unwrap();
        let install = Install {
            kind: Kind::Dir,
            target: dir.clone(),
            exe: dir.join("app.exe"),
        };
        let download = install.prepare().unwrap();
        zip_of(&download, &[("other.exe", "x")]);
        assert!(install.stage(&download).unwrap_err().contains("no app.exe"));
        let download = install.prepare().unwrap();
        zip_of(&download, &[("../escape.exe", "x")]);
        assert!(install.stage(&download).unwrap_err().contains("outside"));
        assert!(!dir.parent().unwrap().join("escape.exe").exists());
        assert_eq!(read(&dir.join("app.exe")), "old");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn links_stay_inside_the_zip() {
        let dir = temp("links");
        let zip_with = |entries: &[(&str, Option<&str>)]| {
            let path = dir.join("links.zip");
            let mut zip = zip::ZipWriter::new(File::create(&path).unwrap());
            let options = zip::write::SimpleFileOptions::default();
            for (name, link) in entries {
                match link {
                    Some(target) => zip.add_symlink(*name, *target, options).unwrap(),
                    None => {
                        zip.start_file(*name, options).unwrap();
                        zip.write_all(b"x").unwrap();
                    }
                }
            }
            zip.finish().unwrap();
            path
        };
        // A framework's links unpack.
        let into = dir.join("ok");
        let ok = zip_with(&[
            ("F/Versions/A/F", None),
            ("F/Versions/Current", Some("A")),
            ("F/F", Some("Versions/Current/F")),
        ]);
        unzip(&ok, &into).unwrap();
        assert_eq!(read(&into.join("F/F")), "x");
        // Out, or a link to go through later.
        for entries in [
            &[("up", Some(".."))][..],
            &[("abs", Some("/etc"))],
            &[("d/a", Some("..")), ("x", Some("d/a/.."))],
            &[("in", Some("F")), ("in/evil", None)],
        ] {
            let into = dir.join("bad");
            let _ = fs::remove_dir_all(&into);
            assert!(unzip(&zip_with(entries), &into).is_err(), "{entries:?}");
        }
        assert!(!dir.join("evil").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_bundle_or_file_swaps_whole() {
        let dir = temp("bundle");
        let app = dir.join("Spelunky Classic.app");
        fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
        fs::write(app.join("Contents/MacOS/spelunky"), "old").unwrap();
        let install = Install {
            kind: Kind::Bundle,
            target: app.clone(),
            exe: app.join("Contents/MacOS/spelunky"),
        };
        if cfg!(target_os = "macos") {
            assert_eq!(Install::detect(&install.exe, None).unwrap(), install);
        }
        assert_eq!(install.work_dir(), dir.join(".Spelunky Classic.app.update"));
        let download = install.prepare().unwrap();
        // The release's name for the app needn't be the one it was installed under.
        zip_of(
            &download,
            &[
                ("Spelunky.app/Contents/MacOS/spelunky", "new"),
                ("Spelunky.app/Contents/Info.plist", ""),
            ],
        );
        let staged = install.stage(&download).unwrap();
        let applied = install.apply(&staged).unwrap();
        assert_eq!(read(&install.exe), "new");
        assert!(app.join("Contents/Info.plist").exists());
        install.roll_back(applied).unwrap();
        assert_eq!(read(&install.exe), "old");
        assert!(!app.join("Contents/Info.plist").exists());

        let image = dir.join("Spelunky.AppImage");
        fs::write(&image, "old image").unwrap();
        let install =
            Install::detect(Path::new("/tmp/.mount_x/usr/bin/app"), Some(&image)).unwrap();
        assert_eq!(install.kind, Kind::File);
        let download = install.prepare().unwrap();
        fs::write(&download, "new image").unwrap();
        let staged = install.stage(&download).unwrap();
        install.apply(&staged).unwrap();
        assert_eq!(read(&image), "new image");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&image).unwrap().permissions().mode() & 0o111,
                0o111
            );
        }
        install.clean_up();
        fs::remove_dir_all(&dir).unwrap();
    }
}
