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

Each has the extracted assets (`assets/`, without the `gml/` sources), `LICENSE` and `NOTICE.txt`, the
unofficial-mod notice the Spelunky User License asks for, which also says where saves go (`app::save`'s
per-user directory). The app finds the assets beside its executable (`Contents/Resources` in the macOS app),
from any working directory. The icon is the original `spelunky.exe`'s, from the `.gmk` (`tools`' `icon`).

What the updater (851-3255, [docs/updates.md](updates.md)) needs from them, and gets: a release is built with
`SPELUNKY_VERSION` set to its version (`build` sets it for a version like `x.y.z`, not for a dry run's
`dev-<sha>`, which stays a development build); the executable is `spelunky` (`spelunky.exe`) in every
release; the assets are inside what an update replaces (the bundle, the Windows folder, the AppImage); the
macOS zip holds the signed, notarized, stapled app; and Linux updates through the AppImage (a tarball install
is offered the release page).

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
  build script links in. A release build is a GUI program: no console window opens with it.
- **Linux**: `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu` with zig (`cargo zigbuild`), against
  glibc 2.31. The app links two system libraries, ALSA (`libasound.so.2`) and libudev (`libudev.so.1`);
  X11, Wayland and Vulkan are loaded at run time. There are no Linux headers or `pkg-config` on a Mac, so the
  build overrides `alsa-sys`' and `libudev-sys`' build scripts (cargo's `links` overrides) and links Ubuntu
  20.04's libraries, which only lend their sonames: the system's are the ones loaded. The AppImage is
  AppImage's type2 runtime (a pinned release) followed by a gzip squashfs of the same files as the tarball,
  with `AppRun` linking to the game, a `.desktop` file and the icon; `tools` builds `mksquashfs` from
  source for it.

`scripts/release/tools` fetches the toolchains into `target/release-tools/` (pinned versions, checked
against their SHA-256s) and adds the Rust targets; `build` runs it first. The builds use their own target
dir, `target/release-build/`.

## Signing and notarizing the macOS app

`scripts/release/macos-dmg APP OUT.dmg [OUT.zip]` signs and packages the app. With `SPELUNKY_SIGN_IDENTITY`
(a Developer ID Application identity, in `SPELUNKY_KEYCHAIN` or the search list) it signs with the hardened
runtime and a secure timestamp, and signs the `.dmg`. With `SPELUNKY_NOTARY_KEY`, `SPELUNKY_NOTARY_KEY_ID`
and `SPELUNKY_NOTARY_ISSUER` (an App Store Connect API key's `.p8`, ID and issuer) it also notarizes: first
the app (submitted as a zip), stapling its ticket to the bundle, so the updater's zip and the `.dmg` both
hold a stapled app that opens offline; then the `.dmg`, stapled and checked with Gatekeeper. Without them
the app is signed ad hoc, which runs where it was built but which Gatekeeper turns away elsewhere.

## On CI

`.github/workflows/build-native.yml`, which `release.yml` calls with the version and `sign` ([docs/ci.md](ci.md)),
and which runs by hand (Actions › Native builds › Run workflow) to try a branch, unsigned:

1. **build**, on a Mac mini: `scripts/release/ci-setup` installs rustup and the pinned toolchain, then
   `scripts/release/build` makes all six. A signed build sets everything up afresh in the job's temp
   directory, since pull requests' jobs can change what the minis keep; a branch's reuses the tool cache
   (capped at 10 GiB of target dir). It also checks the smoke replay against its committed oracle
   fingerprint (`fidelity-check`, [docs/formats.md](formats.md#fingerprints)) and runs it on `replay-run`
   for the trace every shipped build must reproduce.
2. **macos-sign** (with `sign`), on a Mac mini, in the `release` environment: the only job with the signing
   secrets, and it compiles nothing. It imports the Developer ID identity into a keychain made for the job
   (as lil.tech's release does), runs `macos-dmg` with it and the notary key, and deletes the keychain and
   key however the job ends.
3. **smoke**, on each file as a player downloads it, on its own OS and architecture: the macOS `.dmg` and
   zip as arm64 on a Mac mini and as x86_64 on `macos-15-intel` (the minis have no Rosetta); Windows on
   `windows-latest` and `windows-11-arm` (Direct3D 12 on WARP); Linux's AppImage and tarball on
   `ubuntu-latest` and `ubuntu-24.04-arm` (Vulkan on lavapipe; the AppImage extracts itself, as there's no
   FUSE).

Every file is an artifact named as the file. `release.yml` publishes them only if every job passed.

The first signed run is the first release (or a hand run of this workflow on `main` with `sign`), once
Alexandru approves it in the `release` environment. Its `macos-sign` job's log shows both notarizations'
results, and its macOS smoke test checks `codesign --verify --deep --strict`, `spctl --assess` and
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
4. on macOS, checks the signature, and with `--signed`, Gatekeeper and the stapled tickets.

It needs only Python's standard library. If the game or the renderer legitimately changes what the replay's
last frame shows, shoot the reference again:

```sh
scripts/cargo run --release -p app -- --replay fidelity/replays/terrain-bomb-brick.txt \
  --screenshot scripts/release/smoke/terrain-bomb-brick.png
```
