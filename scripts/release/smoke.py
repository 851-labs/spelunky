#!/usr/bin/env python3
"""Smoke-tests a native release package as a player gets it (docs/releasing.md).

    smoke.py run PACKAGE --fingerprint HEX --out DIR [--arch x86_64] [--signed]
    smoke.py fingerprint TRACE.jsonl
    smoke.py compare A.png B.png

`run` unpacks PACKAGE (a .dmg, .zip, .tar.gz or .AppImage scripts/release/build made) as a player would,
then:

1. plays the smoke replay headless with no --assets, from an empty working directory, so the game has to
   find the assets it ships with: `spelunky --replay R --screenshot shot.png --trace trace.jsonl`;
2. checks that it exits cleanly, that the screenshot isn't blank and matches the committed reference
   within tolerance (the renderers differ: Metal, Direct3D 12 on WARP, Vulkan on lavapipe), and that the
   trace's fingerprint is the expected one: the same game, tick for tick, on every platform;
3. opens the settings program (`--settings`) headless for a few frames;
4. starts it broken (assets that aren't there), headless and as a session of play, with
   SPELUNKY_DIALOG_LOG set (app::fatal): each must fail promptly, the headless run with no dialog (one
   would hang it), and on Windows and macOS the session with the dialog a player would see, whose text
   this prints;
5. on macOS, checks the app's signature, and with --signed, that Gatekeeper accepts it and that the
   notarization tickets are stapled to the app (and the .dmg).

--fingerprint is the replay's: `fingerprint` of the trace replay-run writes for it on the build machine
(`replay-run fidelity/replays/terrain-bomb-brick.txt trace.jsonl`). --arch x86_64 runs a universal macOS
app's Intel half (natively on an Intel Mac, else under Rosetta). Screenshots and traces are left in DIR.
The reference screenshot is the native app's: `app --replay fidelity/replays/terrain-bomb-brick.txt
--screenshot` it again when the game or its renderer changes what that frame shows.

Only the standard library, so it runs on every runner as it comes.
"""

import argparse
import hashlib
import os
import platform
import shutil
import struct
import subprocess
import sys
import tempfile
import zipfile
import zlib
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent
# The replay: Mines level 1 (seed 1), a bomb destroying bricks. Level generation, the RNG, physics,
# explosions and the HUD, in 760 frames.
REPLAY = REPO / "fidelity/replays/terrain-bomb-brick.txt"
REFERENCE_SHOT = HERE / "smoke/terrain-bomb-brick.png"
SETTINGS_FRAMES = 60
TIMEOUT = 900
# A start that can't find its assets fails before anything else, so it has this long.
BROKEN_TIMEOUT = 60
# The platforms where the app shows its fatal errors in a dialog, and the dialog's title (app::fatal).
DIALOG_PLATFORMS = ("Windows", "Darwin")
DIALOG_TITLE = "Spelunky Classic"

# How far a screenshot may stray from the reference: a pixel differs if a channel is more than
# CHANNEL_TOLERANCE off, and at most PIXEL_TOLERANCE of them may.
CHANNEL_TOLERANCE = 8
PIXEL_TOLERANCE = 0.005
# A blank (or nearly) picture has fewer colours than this.
MIN_COLOURS = 16


def fail(message):
    print(f"smoke: FAIL: {message}", flush=True)
    sys.exit(1)


def ok(message):
    print(f"smoke: ok: {message}", flush=True)


def read_png(path):
    """An 8-bit RGB or RGBA PNG (what the app writes) as (width, height, channels, rows of bytes)."""
    data = Path(path).read_bytes()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError(f"{path} isn't a PNG")
    pos, idat, header = 8, bytearray(), None
    while pos < len(data):
        (length,) = struct.unpack(">I", data[pos : pos + 4])
        kind = data[pos + 4 : pos + 8]
        body = data[pos + 8 : pos + 8 + length]
        pos += 12 + length
        if kind == b"IHDR":
            header = struct.unpack(">IIBBBBB", body)
        elif kind == b"IDAT":
            idat += body
        elif kind == b"IEND":
            break
    width, height, depth, colour, _, _, interlace = header
    channels = {2: 3, 6: 4}.get(colour)
    if depth != 8 or channels is None or interlace:
        raise ValueError(f"{path}: only 8-bit, non-interlaced RGB or RGBA")
    raw = zlib.decompress(bytes(idat))
    stride = width * channels
    rows, previous = [], bytearray(stride)
    for y in range(height):
        start = y * (stride + 1)
        kind, line = raw[start], bytearray(raw[start + 1 : start + 1 + stride])
        for i in range(stride):
            a = line[i - channels] if i >= channels else 0
            b = previous[i]
            c = previous[i - channels] if i >= channels else 0
            if kind == 1:
                line[i] = (line[i] + a) & 255
            elif kind == 2:
                line[i] = (line[i] + b) & 255
            elif kind == 3:
                line[i] = (line[i] + (a + b) // 2) & 255
            elif kind == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[i] = (line[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        rows.append(line)
        previous = line
    return width, height, channels, rows


def pixels(png):
    width, height, channels, rows = png
    for row in rows:
        for x in range(width):
            yield bytes(row[x * channels : x * channels + 3])


def colours(path):
    return len(set(pixels(read_png(path))))


def compare(a, b):
    """The fraction of pixels that differ by more than CHANNEL_TOLERANCE in a channel, and the largest
    difference."""
    pa, pb = read_png(a), read_png(b)
    if pa[:2] != pb[:2]:
        return 1.0, 255, f"{pa[0]}x{pa[1]} against {pb[0]}x{pb[1]}"
    differ = worst = total = 0
    for x, y in zip(pixels(pa), pixels(pb)):
        total += 1
        if x != y:
            d = max(abs(i - j) for i, j in zip(x, y))
            worst = max(worst, d)
            differ += d > CHANNEL_TOLERANCE
    return differ / total, worst, f"{pa[0]}x{pa[1]}"


def fingerprint(trace):
    """The SHA-256 of a trace's frames: every line after the header, which names the program that wrote
    it."""
    digest = hashlib.sha256()
    with open(trace, "rb") as f:
        f.readline()
        for line in f:
            digest.update(line.rstrip(b"\r\n") + b"\n")
    return digest.hexdigest()


def unpack_app(app):
    """The macOS app's executable, once it's checked the app has what a release ships."""
    for extra in ("LICENSE", "NOTICE.txt", "AppIcon.icns", "assets/bundle.bin", "assets/config/bundle.bin"):
        if not (app / "Contents/Resources" / extra).is_file():
            fail(f"{app.name} has no {extra}")
    return app / "Contents/MacOS/spelunky", app


def unpack(package, into):
    """Unpacks as a player would: copies the app out of the .dmg, extracts the archive, or puts the
    AppImage where it can run. Returns the executable and, on macOS, the app."""
    name = package.name
    if name.endswith(".AppImage"):
        exe = into / name
        shutil.copyfile(package, exe)
        exe.chmod(0o755)
        return exe, None
    if name.endswith("-macos.zip"):
        # ditto, as Finder does: it keeps the bundle's signature intact.
        subprocess.run(["ditto", "-x", "-k", package, into], check=True)
        apps = list(into.glob("*.app"))
        if len(apps) != 1:
            fail(f"{name} holds {len(apps)} apps")
        return unpack_app(apps[0])
    if name.endswith(".dmg"):
        mount = Path(tempfile.mkdtemp(prefix="spelunky-dmg-"))
        subprocess.run(
            ["hdiutil", "attach", "-nobrowse", "-readonly", "-noautoopen", "-mountpoint", mount, package],
            check=True,
            stdout=subprocess.DEVNULL,
        )
        try:
            apps = list(mount.glob("*.app"))
            if len(apps) != 1:
                fail(f"{name} holds {len(apps)} apps")
            for extra in ("LICENSE", "NOTICE.txt", "Applications"):
                if not (mount / extra).exists() and not (mount / extra).is_symlink():
                    fail(f"{name} has no {extra}")
            app = into / apps[0].name
            subprocess.run(["ditto", apps[0], app], check=True)
        finally:
            subprocess.run(["hdiutil", "detach", "-quiet", mount], check=False)
        return unpack_app(app)
    if name.endswith(".zip"):
        with zipfile.ZipFile(package) as z:
            z.extractall(into)
        exe = "spelunky.exe"
    elif name.endswith(".tar.gz"):
        # tar keeps the executable bit, which zipfile and older tarfiles don't.
        subprocess.run(["tar", "-xzf", package, "-C", into], check=True)
        exe = "spelunky"
    else:
        fail(f"unknown package {name}")
    tops = [p for p in into.iterdir()]
    if len(tops) != 1 or not tops[0].is_dir():
        fail(f"{name} doesn't unpack into one folder")
    for extra in ("LICENSE", "NOTICE.txt", "assets/bundle.bin", "assets/config/bundle.bin"):
        if not (tops[0] / extra).is_file():
            fail(f"{name} has no {extra}")
    return tops[0] / exe, None


def launch(command, cwd, what, timeout=TIMEOUT, env=None, fails=False):
    """Runs the game to its end, which must be a clean exit, or with `fails` an error. Returns the
    result."""
    print(f"smoke: {what}: {' '.join(str(c) for c in command)}", flush=True)
    # CI machines have no FUSE to mount an AppImage with, so it unpacks itself to a temporary directory
    # and runs from there: the same files, by the same runtime.
    env = dict(os.environ, APPIMAGE_EXTRACT_AND_RUN="1", **(env or {}))
    try:
        result = subprocess.run(command, cwd=cwd, env=env, capture_output=True, text=True, timeout=timeout)
    except subprocess.TimeoutExpired:
        fail(f"{what} didn't finish in {timeout} s")
    sys.stdout.write(result.stdout)
    sys.stdout.write(result.stderr)
    if fails:
        if result.returncode == 0:
            fail(f"{what} exited cleanly")
        ok(f"{what} failed promptly, with {result.returncode}")
    elif result.returncode != 0:
        fail(f"{what} exited with {result.returncode}")
    else:
        ok(f"{what} exited cleanly")
    return result


def broken_start(prefix, exe, cwd, out, tag):
    """Starts the game where it can't find its assets: headless, and as a session of play. Each must fail
    with its error on stderr, not hang. The dialog (app::fatal) goes to SPELUNKY_DIALOG_LOG instead of on
    screen: never for the headless run, and on the platforms that have one, for the session, with the
    error stderr has."""
    missing = cwd / "no-assets"
    log = out / f"{tag}-dialog.txt"
    with_dialog = platform.system() in DIALOG_PLATFORMS
    for what, extra, dialog in (
        ("a headless start without assets", ["--replay", REPLAY, "--screenshot", out / f"{tag}-broken.png"], False),
        ("a session without assets", [], with_dialog),
    ):
        log.unlink(missing_ok=True)
        result = launch(
            prefix + [exe, "--assets", missing] + extra,
            cwd,
            what,
            timeout=BROKEN_TIMEOUT,
            env={"SPELUNKY_DIALOG_LOG": str(log)},
            fails=True,
        )
        error = result.stderr.strip()
        if not error.startswith("app: "):
            fail(f"{what} gave no error on stderr")
        if not dialog:
            if log.exists():
                fail(f"{what} showed a dialog: {log.read_text(encoding='utf-8')!r}")
            ok(f"{what} showed no dialog")
            continue
        if not log.is_file():
            fail(f"{what} showed no dialog")
        title, _, message = log.read_text(encoding="utf-8").partition("\n")
        print(f"smoke: the dialog: [{title}] {message.strip()}")
        if title != DIALOG_TITLE or message.strip() != error:
            fail(f"{what}'s dialog isn't {DIALOG_TITLE!r} with the error stderr has")
        ok(f"{what} showed its error in a dialog")
    log.unlink(missing_ok=True)


def run(args):
    package = Path(args.package).resolve()
    out = Path(args.out).resolve()
    out.mkdir(parents=True, exist_ok=True)
    unpacked = Path(tempfile.mkdtemp(prefix="spelunky-smoke-"))
    exe, app = unpack(package, unpacked)
    ok(f"unpacked {package.name} ({package.stat().st_size / 1e6:.1f} MB)")
    prefix = []
    if args.arch:
        if platform.system() != "Darwin":
            fail("--arch is for macOS")
        prefix = ["arch", f"-{args.arch}"]
    tag = args.tag or package.name
    # Nothing here, so the game can only find the assets it ships with.
    empty = Path(tempfile.mkdtemp(prefix="spelunky-cwd-"))

    if app is not None:
        subprocess.run(["codesign", "--verify", "--deep", "--strict", "--verbose=2", app], check=True)
        ok("codesign --verify --deep --strict")
        if args.arch:
            lipo = subprocess.run(["lipo", "-archs", exe], capture_output=True, text=True, check=True)
            if args.arch not in lipo.stdout.split():
                fail(f"the app has no {args.arch} slice ({lipo.stdout.strip()})")
        if args.signed:
            subprocess.run(["spctl", "--assess", "--type", "execute", "--verbose=2", app], check=True)
            ok("spctl --assess accepts the app")
            subprocess.run(["xcrun", "stapler", "validate", app], check=True)
            ok("the app's notarization ticket is stapled")
            if package.name.endswith(".dmg"):
                subprocess.run(["xcrun", "stapler", "validate", package], check=True)
                ok("the .dmg's notarization ticket is stapled")

    shot = out / f"{tag}.png"
    trace = out / f"{tag}.jsonl"
    launch(
        prefix + [exe, "--replay", REPLAY, "--screenshot", shot, "--trace", trace],
        empty,
        "the game",
    )
    if not shot.is_file():
        fail("no screenshot")
    n = colours(shot)
    if n < MIN_COLOURS:
        fail(f"the screenshot is blank ({n} colours)")
    ok(f"the screenshot has {n} colours")
    differ, worst, size = compare(shot, REFERENCE_SHOT)
    print(f"smoke: {size}: {differ:.4%} of pixels differ from the reference by more than {CHANNEL_TOLERANCE} (worst {worst})")
    if differ > PIXEL_TOLERANCE:
        fail(f"the screenshot doesn't match {REFERENCE_SHOT.name}")
    ok("the screenshot matches the reference")

    expected = args.fingerprint
    got = fingerprint(trace)
    print(f"smoke: trace fingerprint {got}, expected {expected}")
    if got != expected:
        fail("the trace's fingerprint isn't the replay's: this build plays the game differently")
    ok("the trace's fingerprint matches")

    settings = out / f"{tag}-settings.png"
    launch(
        prefix + [exe, "--settings", "--frames", str(SETTINGS_FRAMES), "--screenshot", settings],
        empty,
        "the settings program",
    )
    n = colours(settings) if settings.is_file() else 0
    if n < 2:
        fail("the settings program's screenshot is blank")
    ok(f"the settings program drew {n} colours")
    broken_start(prefix, exe, empty, out, tag)
    shutil.rmtree(unpacked, ignore_errors=True)
    print(f"smoke: PASS {package.name}{' (' + args.arch + ')' if args.arch else ''}")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    p = sub.add_parser("run")
    p.add_argument("package")
    p.add_argument("--fingerprint", required=True, help="the replay's trace fingerprint")
    p.add_argument("--out", required=True)
    p.add_argument("--arch", choices=["arm64", "x86_64"])
    p.add_argument("--signed", action="store_true")
    p.add_argument("--tag", help="the screenshots' and trace's name (default: the package's)")
    p = sub.add_parser("fingerprint")
    p.add_argument("trace")
    p = sub.add_parser("compare")
    p.add_argument("a")
    p.add_argument("b")
    args = parser.parse_args()
    if args.command == "run":
        run(args)
    elif args.command == "fingerprint":
        print(fingerprint(args.trace))
    else:
        differ, worst, size = compare(args.a, args.b)
        print(f"{size}: {differ:.4%} of pixels differ by more than {CHANNEL_TOLERANCE} (worst {worst})")


if __name__ == "__main__":
    main()
