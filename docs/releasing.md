# Native releases

The game ships for six platforms, each a file a player downloads, unpacks and runs. VERSION is the port's
own, from `v0.1.0` on (the tag without its `v`), never the original game's 1.1:

| File | What |
| --- | --- |
| `spelunky-VERSION-macos.dmg` | `Spelunky Classic.app`, universal (Apple silicon and Intel), macOS 11 on; signed with Developer ID, notarized and stapled |
| `spelunky-VERSION-macos.zip` | the same app, zipped with `ditto`: what the updater installs |
| `spelunky-VERSION-windows-x64.zip`, `-windows-arm64.zip` | `spelunky.exe` in a folder, unsigned, Windows 10 on |
| `spelunky-VERSION-linux-x64.AppImage`, `-linux-arm64.AppImage` | the game as one file, for glibc 2.31 on (Ubuntu 20.04, Debian 11) with ALSA and Vulkan |
| `spelunky-VERSION-linux-x64.tar.gz`, `-linux-arm64.tar.gz` | the same files as the AppImage, in a folder |

Each has the extracted assets (`assets/`, without the `gml/` sources), `LICENSE`, `NOTICE.txt`, the
unofficial-mod notice the Spelunky User License asks for, which also says where saves go (`app::save`'s
per-user directory), and `THIRD-PARTY-LICENSES.txt` ([Third-party licenses](#third-party-licenses)). The app finds the assets beside its executable (`Contents/Resources` in the macOS app),
from any working directory. The icon is the original `spelunky.exe`'s, from the `.gmk` (`tools`' `icon`).

What the updater (851-3255, [docs/updates.md](updates.md)) needs from them, and gets: a release is built with
`SPELUNKY_VERSION` set to its version (`build` sets it for a version like `x.y.z`, not for a dry run's
`dev-<sha>`, which stays a development build); the executable is `spelunky` (`spelunky.exe`) in every
release; the assets are inside what an update replaces (the bundle, the Windows folder, the AppImage); the
macOS zip holds the signed, notarized, stapled app; and Linux updates through the AppImage (a tarball install
is offered the release page).

## Third-party licenses

The builds link about 250 third-party Rust crates (MIT, Apache-2.0, ISC, BSD, Zlib, MPL-2.0, Unicode-3.0 and
CDLA-Permissive-2.0), the Rust standard library, the Windows builds' mingw-w64 C runtime and LLVM's libunwind, the
Linux builds' Zig runtime and Cantarell font, and the AppImages start with AppImage's runtime (libfuse, under the
LGPL 2.1, squashfuse, zstd, zlib and musl). Their licenses ask for their notices to go with every copy, so every
package and the web build (`scripts/web --package`, beside its `LICENSE` in `v/HASH/`, linked from `/play/`'s page)
carry `scripts/release/THIRD-PARTY-LICENSES.txt`, and the macOS app's About panel says where it is.

`scripts/third-party-licenses` writes it: the runtimes' and the font's notices from
`scripts/release/licenses/runtimes.txt`, kept by hand (a new runtime, a new AppImage runtime or Zig, goes there);
the crates' from `Cargo.lock` with [cargo-about](https://github.com/EmbarkStudios/cargo-about) 0.9.3, for every
target the app is built for, without build or dev dependencies (it lists the proc macros the crates use too,
which run in the compiler and ship nothing; `--check` doesn't require them) (`scripts/release/licenses/about.toml`, whose `accepted` licenses are the only ones it takes: a crate under another
fails it, and is a decision to make, not to wave through); and the standard library's from the toolchain's
`COPYRIGHT-library`. `scripts/check` runs it with `--check`, which needs only cargo (and crates.io for any crate it
hasn't downloaded): it fails when a crate the app links on any platform isn't in it, when it lists one the app no
longer depends on, when its runtimes or Rust's version aren't current, or when `about.toml` or `about.hbs` changed
since it was written (its introduction names their hashes). So a dependency change comes with running it (`cargo install cargo-about --locked --features cli
--version 0.9.3` first), and the file's diff shows what changed.

## Building them

All six build on a Mac, with open-source cross toolchains only: no Windows SDK, MSVC or Windows license.

```sh
scripts/release/build --version 0.1.0                 # all six, into target/release-packages/
scripts/release/build --version 0.1.0 macos linux-x64 # some of them
```

- **macOS**: `aarch64-apple-darwin` and `x86_64-apple-darwin`, combined with `lipo` into one executable
  in the `.app`, then `scripts/release/macos-dmg`, which signs it ad hoc unless told otherwise (below) and
  makes the `.dmg` and the zip.
- **Windows**: `x86_64-pc-windows-gnullvm` and `aarch64-pc-windows-gnullvm` with
  [llvm-mingw](https://github.com/mstorsjo/llvm-mingw) (clang, lld, mingw-w64 on the UCRT). The C runtime and
  the unwinder are linked in (`crt-static`), so the game needs only Windows' own DLLs; the build fails if it
  imports any other. The icon and version information are a resource object `windres` makes, which the app's
  build script links in. A release build is a GUI program: no console window opens with it. So that a
  game that can't start doesn't just do nothing, its error (or panic) shows in a message box when it has no
  stderr (`app::fatal`; macOS shows an alert when opened from Finder), never in a headless run or a replay.
- **Linux**: `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu` with zig (`cargo zigbuild`), against
  glibc 2.31. The app links two system libraries, ALSA (`libasound.so.2`) and libudev (`libudev.so.1`);
  X11, Wayland and Vulkan are loaded at run time. There are no Linux headers or `pkg-config` on a Mac, so the
  build overrides `alsa-sys`' and `libudev-sys`' build scripts (cargo's `links` overrides) and links Ubuntu
  20.04's libraries, which only lend their sonames: the system's are the ones loaded. The AppImage is
  AppImage's type2 runtime (a pinned release) followed by a gzip squashfs of the same files as the tarball,
  with `AppRun` linking to the game, a `.desktop` file and the icon; `tools` builds `mksquashfs` from
  source for it.

`scripts/release/tools` fetches the toolchains into `target/release-tools/` (pinned versions, checked
against their SHA-256s) and adds the Rust targets; `build` runs it first. `scripts/release/tools ffmpeg`
fetches the one tool that isn't for the apps: a static ffmpeg (Martin Riedl's macOS build, which links only
the system's libraries) for the wiki's clips. The builds use their own target
dir, `target/release-build/`.

## Signing and notarizing the macOS app

`scripts/release/macos-dmg APP OUT.dmg [OUT.zip]` signs and packages the app. With `SPELUNKY_SIGN_IDENTITY`
(a Developer ID Application identity, in `SPELUNKY_KEYCHAIN` or the search list) it signs with the hardened
runtime and a secure timestamp, and signs the `.dmg`. With `SPELUNKY_NOTARY_KEY`, `SPELUNKY_NOTARY_KEY_ID`
and `SPELUNKY_NOTARY_ISSUER` (an App Store Connect API key's `.p8`, ID and issuer) it also notarizes: first
the app (submitted as a zip), stapling its ticket to the bundle, so the updater's zip and the `.dmg` both
hold a stapled app that opens offline; then the `.dmg`, stapled and checked with Gatekeeper. Stapling and
`stapler validate` fetch the ticket from Apple's ticket service, so each is tried again (for up to 2.5
minutes) when its connection drops, but not when the ticket itself is missing or invalid. Without these
variables the app is signed ad hoc, which runs where it was built but which Gatekeeper turns away elsewhere.

## On CI

`.github/workflows/build-native.yml`, which `release.yml` calls with the version and `sign` ([docs/ci.md](ci.md)),
and which runs by hand (Actions › Native builds › Run workflow) to try a branch, unsigned:

1. **Build apps** (`build`), on GitHub's Apple silicon Mac (`macos-15`, a fresh VM), never a Mac mini: the
   minis are persistent and shared with pull requests' jobs and other repositories', so a job before could
   leave something a build would run (a `.cargo/config.toml` above the checkout, which Cargo reads whatever
   `CARGO_HOME` is, a login item, a tool on `PATH`; 851-3899). `scripts/release/ci-setup` installs
   rustup, the pinned toolchain and the cross toolchains in the job's temp directory, failing if a Cargo
   config sits above the checkout, then `scripts/release/build` makes all six, from nothing every time. It also
   checks the smoke replay against its committed oracle fingerprint (`fidelity-check`,
   [docs/formats.md](formats.md#fingerprints)) and runs it on `replay-run` for the trace every shipped build
   must reproduce. (The wiki's media aren't a release's: they're committed,
   [docs/wiki-media.md](wiki-media.md#committing).)
2. **Sign Mac app** (`macos-sign`, with `sign`), on another fresh `macos-15`, in the `release` environment:
   the only job with the signing secrets, and it compiles nothing, so the Developer ID key is only ever on a
   machine no other job has run on. It imports the Developer ID identity into
   a keychain made for the job (as lil.tech's release does), runs `macos-dmg` with it and the notary key,
   and deletes the keychain and key however the job ends.
3. **Test Mac app** and **Test Win/Linux app** (`smoke-macos`, `smoke`), on each file as a player
   downloads it, on its own OS and architecture: the macOS `.dmg` and
   zip as arm64 on `macos-15` and as x86_64 on `macos-15-intel`, GitHub's fresh VMs, never a Mac mini (a
   job before could leave something there, a tool on `PATH` or the `python3` the test runs under, that sways
   this release gate); Windows on
   `windows-2025` and `windows-11-arm` (Direct3D 12 on WARP); Linux's AppImage and tarball on
   `ubuntu-24.04` and `ubuntu-24.04-arm` (Vulkan on lavapipe; the AppImage extracts itself, as there's no
   FUSE).

Every file is an artifact named as the file. `release.yml` publishes them only if every job passed. (Releases
carried the wiki's media until 851-3577; they're committed in `wiki/media` now.)

The first signed run is the first release (or a hand run of this workflow on `main` with `sign`); nobody
has to approve it. Its `macos-sign` job's log shows both notarizations'
results, and its macOS smoke test checks `codesign --verify --deep --strict`, `spctl --assess` and
(in the arm64 run, retrying network errors; GitHub's Intel Macs keep losing Apple's ticket service)
`xcrun stapler validate` on the downloaded app and `.dmg`.

## The smoke test

`scripts/release/smoke.py run PACKAGE --fingerprint HEX --out DIR` unpacks the package as a player would
(copies the app out of the `.dmg`, unzips, untars, or runs the AppImage) and, from an empty working
directory, so the game must find its own assets:

1. plays `fidelity/replays/terrain-bomb-brick.txt` (Mines level 1, a bomb destroying bricks: 760 frames)
   headless, writing a screenshot of the last frame and the trace;
2. checks a clean exit; that the screenshot isn't blank and matches
   `scripts/release/smoke/terrain-bomb-brick.png` (at most 0.5% of pixels more than 8 off in a channel: the
   renderers may differ slightly); and that the trace's SHA-256 (of its frames) is the one `replay-run`'s
   gave on the build machine, whose run `fidelity-check` held to the oracle's fingerprint. So the shipped
   game plays as the original does, tick for tick, on every platform. (The committed fingerprint itself
   needs the harness to compute, which the smoke machines don't have; this chain needs only Python.)
3. runs the settings program (`--settings`) for 60 frames;
4. starts the game without its assets, headless and as a session of play, with `SPELUNKY_DIALOG_LOG` set
   (the dialog's text goes to that file instead of on screen): both must fail within a minute, the headless
   run without a dialog (one nobody clicks away would hang it), and on Windows and macOS the session with
   the dialog a player would see, its text the error stderr has;
5. on macOS, checks the signature, and with `--signed`, Gatekeeper and the stapled tickets.

It needs only Python's standard library. If the game or the renderer legitimately changes what the replay's
last frame shows, shoot the reference again:

```sh
scripts/cargo run --release -p app -- --replay fidelity/replays/terrain-bomb-brick.txt \
  --screenshot scripts/release/smoke/terrain-bomb-brick.png
```
