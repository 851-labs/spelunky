# Updating the native apps

The macOS, Windows and Linux apps update themselves (851-3255); the web build at spelunky.ai/play always
serves the newest release anyway. The code is the `update` crate (checking, verifying, swapping, rolling
back) and `app::update` (when to check, and asking the player). Neither is in `gm` or `game`, and the web
build doesn't build them.

## What the player sees

- **Checks:** on launch, at most once a day, unless turned off; or when the player asks. On macOS the menu
  bar has **Check for Updates…** and **Check for Updates Automatically** (a checkmark). Windows and Linux
  get an updates menu on **F2** with the same two choices: their window has no menu bar, because a menu bar
  would take the Alt key from the game (Alt focuses it), and muda's Linux menus need GTK, which winit's
  windows aren't.
- **Never mid-run:** checks start, and prompts show, only between runs: the intro, the title screen and the
  rooms it opens (highscores, credits, key and joystick setup). A prompt that arrives during a run waits for
  the next time the player is back on the title screen. A replay, a recording (`--record`, `--trace`) or a
  headless run never has an updater at all.
- **Download, verify, then ask:** a newer version is downloaded and checked in the background, then
  offered: *Restart and update*, *Later*, *Skip this version*, *Release notes*. The prompt is drawn over the
  game's picture in its own font, like the license notice, and takes the keys while it's up (arrows, Enter,
  Esc); the game never sees them.
- **Restart:** the window closes, the new version is swapped in and started. If it doesn't start, the old
  version is put back and started again, and it says so; that version isn't offered again automatically.
- **Saves untouched:** stats, highscores, `.cfg` files and custom levels live in the per-user data
  directory (`app::save`), which an update never touches. The updater keeps its own few settings in
  `update.json` there.
- A copy that can't update itself in place (a development build, an app macOS is running from a read-only
  translocated copy, a release without a build for this platform) says a new version is out and offers the
  release page instead.

## The manifest

Each release publishes `https://spelunky.ai/releases/latest.json`:

```json
{
  "manifest": "{\"version\":\"1.1.0\",\"notes\":\"https://github.com/851-labs/spelunky/releases/tag/v1.1.0\",\"assets\":{\"macos\":{\"url\":\"https://github.com/851-labs/spelunky/releases/download/v1.1.0/spelunky-classic-v1.1.0-macos.zip\",\"sha256\":\"...\",\"size\":41234567,\"format\":\"app-zip\"},...}}",
  "signature": "<ed25519 signature, hex>"
}
```

The signature is ed25519, over `spelunky-classic update manifest v1\n` followed by the `manifest` string's
exact bytes, so there's no JSON canonical form to agree on. Platforms are `macos`, `windows-x86_64`,
`windows-aarch64`, `linux-x86_64` and `linux-aarch64`; formats `app-zip`, `dir-zip` and `appimage`.

The app refuses to install unless:

- the signature is the release key's (`verify_strict`: no malleable signatures or weak keys);
- the version is newer than its own (an older or equal one is "up to date": no downgrades, even with a
  validly signed old manifest);
- every URL is HTTPS (redirects too: GitHub's downloads redirect to its CDN);
- the download is exactly the manifest's size (it stops reading one byte past it) and SHA-256;
- the download unpacks inside its own directory (no `..` or absolute paths, no symlinks pointing out) and
  holds an executable named like the running one, in the same place.

## Installs, per platform

| Platform | Replaced | Download | How |
| --- | --- | --- | --- |
| macOS | the whole `.app` | `app-zip`: a zip with one `.app` | one rename aside, one rename in |
| Windows | the install directory's files | `dir-zip`: the directory, maybe in one top folder | each file: the old one renamed aside (Windows allows that for a running `.exe`, not replacing it), the new one renamed in |
| Linux | the AppImage (`$APPIMAGE`) | `appimage`: the AppImage itself | one rename aside, one rename in |

Everything happens in a work directory beside what's replaced, on the same file system, so each step is a
rename: `.Spelunky Classic.app.update/` next to the bundle, `.update/` inside a Windows install,
`.<name>.AppImage.update/` next to an AppImage. `new/` holds the unpacked download, `old/` what it
replaced. If the install isn't writable (an app in `/Applications` for a user who isn't an admin), the
check says so before downloading anything.

**Rollback:** the old version swaps, then starts the new one with `SPELUNKY_UPDATE_CONFIRM` naming a file,
and waits. The new version writes it once its window has shown a frame. If the new version exits first, or
doesn't get there in 60 seconds, the old one stops it, moves everything back from `old/`, records why in
`update.json`, and starts itself again. Once confirmed, the work directory goes; on Windows the old `.exe`
can't be deleted while the old process still runs, so the next launch finishes the clean-up. (A new version
on probation leaves the work directory alone.)

## What the release builds must do (851-3251, 851-3250)

- **Version:** build the app with `SPELUNKY_VERSION` set to the release's version, the tag without its `v`
  (`SPELUNKY_VERSION=${TAG#v} scripts/cargo build --release -p app ...`). Without it a build is a
  development build and never updates.
- **macOS:** besides the `.dmg`, attach a zip of the same signed, notarized and stapled `.app`:
  `ditto -c -k --keepParent "Spelunky Classic.app" spelunky-classic-TAG-macos.zip`. The update installs
  exactly that bundle, so its signature and notarization are intact.
- **Windows:** the release zip is the download as is: the install directory, in one top-level folder or
  not.
- **Linux:** the AppImage is the download. A Linux tarball install can't update in place (it's offered the
  release page).
- **All:** keep the executable's name the same from release to release (the update checks it's there), keep
  everything the app reads (its `assets/`) inside what's replaced (the bundle, the install directory, the
  AppImage), and name release files with letters, digits and `._-+` only.

## Publishing a release's manifest

`release.yml` (851-3250) signs the manifest once every download is built and smoke-tested, and attaches it
to the GitHub Release with the others:

```yaml
    environment: release            # for the secret; it's main-only and needs Alexandru's approval
    steps:
      # ... the downloads are in dist/ ...
      - name: Sign the update manifest
        env:
          UPDATE_SIGNING_KEY: ${{ secrets.UPDATE_SIGNING_KEY }}
        run: |
          scripts/release/manifest sign "$TAG" \
            macos=dist/spelunky-classic-$TAG-macos.zip \
            windows-x86_64=dist/spelunky-classic-$TAG-windows-x86_64.zip \
            windows-aarch64=dist/spelunky-classic-$TAG-windows-aarch64.zip \
            linux-x86_64=dist/spelunky-classic-$TAG-linux-x86_64.AppImage \
            linux-aarch64=dist/spelunky-classic-$TAG-linux-aarch64.AppImage \
            --out dist/latest.json
      # ... then upload dist/latest.json to the release with the downloads.
```

(The file names are whatever 851-3251 names them; each must be uploaded under that name, since the manifest
points at `https://github.com/851-labs/spelunky/releases/download/TAG/NAME`.) It fails if
`UPDATE_SIGNING_KEY` isn't the key in `crates/update/keys/release.pub`, so the apps can't be sent a
manifest they'd refuse. Nothing else needs the secret, and it must never reach a pull request's jobs.

The manifest goes live with the web build: `scripts/bump-web-release TAG` downloads the release's
`latest.json` as well, verifies it, and pins its SHA-256 in `site/web-release.txt`; `site/build.sh` then
puts it at `/releases/latest.json` (`docs/web-deploy.md`). So merging the bump is what offers the update to
every app, and reverting it (or bumping an older tag) stops offering it. Apps that already updated stay
updated: nothing downgrades.

## The release key (once)

The private key exists only as the `release` environment's `UPDATE_SIGNING_KEY` secret and a backup. Until
it's made, `crates/update/keys/release.pub` is a marked placeholder: builds don't update, and
`scripts/release/manifest sign` refuses. `crates/update/keys/test.key` is a published test keypair that
nothing shipped trusts (a test checks that).

1. On a trusted machine, in the repo: `scripts/release/manifest keygen ~/update-signing.key`. It writes the
   private seed (hex) to that new file, readable only by you, and prints the new contents of
   `crates/update/keys/release.pub`.
2. Store the private key as the secret: `gh secret set UPDATE_SIGNING_KEY --env release --repo
   851-labs/spelunky < ~/update-signing.key`.
3. Back it up: `op document create ~/update-signing.key --title "spelunky – update signing key" --vault
   Private` (the same vault as the other release secrets).
4. Delete the file: `rm -P ~/update-signing.key`.
5. Commit the printed public key as `crates/update/keys/release.pub`, in a PR. Releases built after it
   merges can update; ones before it can't (they trust no key).

If the private key is ever lost, the apps can't be updated any more except by downloading a new build by
hand: keep the backup. If it leaks, make a new key the same way, release a build that trusts only it, and
tell players to download that build by hand.

## Testing

- `scripts/cargo test -p update`: the manifest (a good one, tampered ones, another key's, a signature made
  for something else, a refused downgrade), versions, the swap and rollback of each kind of install, and
  `tests/e2e.rs`: updates from a fake older build to a fake newer one, end to end, through a server on
  127.0.0.1. The fake builds are copies of `update-fixture`, a stand-in app that goes through the same
  calls as the app, with its version read from beside it. It covers a directory install everywhere, a
  `.app` on macOS and an AppImage on Linux; rollback of a new version that quits or hangs; a download whose
  checksum or size is wrong, a manifest signed with another key and an older version, none of which
  install; and the saves in the data directory, byte for byte.
- CI (`.github/workflows/update.yml`) runs `cargo test -p update` on macOS, Windows and Linux.
- Trying the real app: a debug build takes `SPELUNKY_VERSION`, `SPELUNKY_UPDATE_URL` and
  `SPELUNKY_UPDATE_KEY` (a public key file) from the environment, which a release build ignores. Put the
  debug binary in a `.app` (`Contents/MacOS/app`), make a newer zip of it, sign a manifest with the test key
  (`UPDATE_SIGNING_KEY=<test.key's hex> scripts/release/manifest sign v1.1.0 macos=new.zip --base-url
  http://127.0.0.1:8000 --public-key crates/update/keys/test.pub --out latest.json`), serve it with
  `python3 -m http.server`, and start the app with `SPELUNKY_VERSION=1.0.0` and the other two pointing there.
