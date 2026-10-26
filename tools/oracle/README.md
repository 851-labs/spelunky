# Reference oracle

The oracle runs the original Spelunky Classic 1.1 (`Spelunky.exe` from `reference/spelunky_1_1.zip`) from a replay,
deterministically and headless, and dumps the game state after every frame. The port is "exact" when its trace
matches this dump tick for tick.

The runner is [OpenGMK](https://github.com/OpenGMK/OpenGMK)'s `gm8emulator`, a GameMaker 8 runner written in
Rust, at a pinned commit plus the patches in `patches/`. It isn't the real GM8 runner: see [Caveats](#caveats).

## Why Linux in Docker

OpenGMK's windowing (the `ramen` crate) only supports Windows and Linux (X11), so it doesn't build on macOS. The
oracle builds and runs it in a Linux container (arm64 under OrbStack on Apple Silicon; amd64 works too) with
Xvfb as the X server and Mesa's software GL. Nothing is drawn to a screen and the frame limiter is off, so a
replay plays as fast as the CPU allows: the 1700-frame example takes 20 to 30 seconds on an Apple Silicon Mac. Most of that is software rendering, which can't be skipped because the game runs logic in its draw
events.

You need Docker: [OrbStack](https://orbstack.dev) on macOS (`brew install orbstack`), or Docker Desktop or
Docker Engine. The scripts check for it and say what's missing.

## Usage

```sh
tools/oracle/build                                    # once, and after any patch change; re-runs are cheap
tools/oracle/run tools/oracle/examples/start.txt /tmp/start.jsonl
tools/oracle/run fidelity/settings/click-through.txt /tmp/settings.jsonl --exe config.exe  # the settings program
```

`tools/oracle/build`:

1. Clones OpenGMK and ramen at the commits pinned in `common.sh` into the cache dir,
   `${SPELUNKY_ORACLE_CACHE:-${XDG_CACHE_HOME:-~/.cache}/spelunky-oracle}`, outside the repo.
2. Vendors ramen into the OpenGMK tree and applies `patches/ramen/*` and then `patches/opengmk/*` in order.
3. Builds the `spelunky-oracle:<Dockerfile hash>` image from `Dockerfile` (Rust pinned by version and digest).
4. Runs `cargo build --release --locked -p gm8emulator` in the container, and installs the binary as
   `/target/oracle/<build stamp>/gm8emulator`. The stamp hashes the pinned commits, the patches and the
   Dockerfile.

Steps 1 and 2 only re-run when a pinned commit, a patch or the Dockerfile changes. Build state lives in two named
Docker volumes, `spelunky-oracle-cargo` (cargo's registry and git checkouts) and `spelunky-oracle-target` (the
build output and the installed binaries). The first build takes a few minutes. With nothing changed, a re-run
takes a few seconds.

All worktrees share the cache dir and the volumes. Builds take turns through a lock (`build.lock` in the cache
dir; one left behind by a killed build is taken over). `run` uses the binary installed under its own checkout's
stamp, so a build of other patches in another worktree never changes what it runs. If that binary doesn't exist,
`run` asks for `tools/oracle/build`.

`tools/oracle/run <replay> <out.jsonl> [--fields] [--savestate-out F] [--savestate-in F]`:

- Extracts the game into the cache dir if needed, then plays the replay on a fresh copy of it, so files the game
  writes (`stats.txt`) never carry over between runs.
- Writes what the run left behind outside the dump to `<out>.host/` (`<out>` without its extension): the game
  directory's files that the replay's header or the game added or changed, under `files/`, and the highscore
  table, as `highscores.jsonl` (patch 0009). The port's `replay-run` writes the same layout; see
  [Host files](../../docs/formats.md#host-files).
- Dumps are big: the 1700-frame example writes about 275 MB. `--fields` adds every instance's variables and the
  globals, which makes lines several times larger.
- `--savestate-out F` writes an OpenGMK savestate after the replay's last frame.
- `--savestate-in F` resumes from such a savestate. The replay must extend the one the savestate was made with.
  Dumping starts at the savestate's frame, and the lines are identical to a full run's.

To start over from nothing:

```sh
rm -rf ~/.cache/spelunky-oracle
docker volume rm spelunky-oracle-cargo spelunky-oracle-target
docker image rm $(docker images -q spelunky-oracle)
```

## Formats

Replays and dumps use the port's formats, specified in [`docs/formats.md`](../../docs/formats.md): the dump is a
trace that `trace-diff` compares directly with the port's. This section covers what the oracle supports and how
it fills them in. Keep the two in step when changing `patches/opengmk/0005-text-replays.patch` or
`0006-state-dump.patch`.

### Replays

Supported:

- `seed`, `time`, `frames`, and `room rScreenInit` (the default; it's the only room supported).
- `settings`, `keys` and `gamepad`: written into the game directory as `settings.cfg`, `keys.cfg` and
  `gamepad.cfg` (CRLF-joined, no trailing newline, as `config.exe` writes them) before the game starts. Without
  them, the shipped files are used.
- `file PATH "contents"`: written into the game directory (as UTF-8) before the game starts.
- `highscore VALUE "NAME"`: see below.
- Events: `press`, `release` and `hold` for keys (every name and code in the spec) and `mb_left`/`mb_right`/
  `mb_middle`, `wheel up`/`wheel down`, and `mouse X Y`. A frame's events apply at its start, in file order. The
  mouse starts at (0, 0) and stays where the last `mouse` event put it.

Not supported, and rejected with an error: any other `room`, `joystick`, `axis` events and `joy1`-`joy32`.
OpenGMK's `joystick_*` functions are stubs that report no joystick, so the oracle can't play joystick input.

The game's bindings come from `keys.cfg`: by default the arrow keys, `Z` jump, `X` attack, `C` item, `Shift`
run, `A` bomb, `S` rope, `F` flare and `P` pay. `Escape` is start (pause).

### Highscore table

Spelunky keeps its save data (money, kills, plays, Tunnel Man progress, ...) in GM8's built-in highscore table,
not in a file (`docs/source-inventory.md`, section 7.2). On startup, `oGlobals`' create event checks the table:
if none of the first nine entries is above its slot's offset (a fresh table), it calls `scrResetHighscores` and
sets `global.firstTime`. The title screen's START door then leads into `rTutorial` instead of `rLevel`, so every run from a fresh table
starts as a first-time player.

`highscore VALUE "NAME"` lines set the table at startup, in order, best first. Up to 10 entries; missing ones
are the fresh table's `<nobody>` entries scoring 0. With no `highscore` lines the table is fresh. Like the
`trace` crate, the oracle only accepts what GM8's table can hold: finite values, never increasing, and none below
0 unless all ten are given. The game reads
the table by place, so give all ten entries, with the game's offsets: MONEY + 9e6, TIME + 8e6, KILLS + 7e6,
SAVES + 6e6, PLAYS + 5e6, WINS + 4e6, DEATHS + 3e6, TUNNEL1 + 2e6, TUNNEL2 + 1e6, then MINIGAMES
(`m1 * 10000 + m2 * 100 + m3`). TUNNEL1 and TUNNEL2 are what's still owed to the Tunnel Man; 0 unlocks that
shortcut.

To start as a returning player with zeroed stats, use the table `scrResetHighscores` writes on the first launch,
as [`examples/returning.txt`](examples/returning.txt) does: MONEY 9000000 down to DEATHS 3000000, then
TUNNEL1 2100001 and TUNNEL2 1200001 (`global.tunnel1Max`/`tunnel2Max` + 1, so nothing is unlocked), then
MINIGAMES 0.

The table isn't saved to disk when the run ends, but it is dumped to `<out>.host/highscores.jsonl`. Savestates
carry it, so with `--savestate-in` the lines have no effect.

### Dumps

One JSON object per frame, one per line, with the frame keys and instance keys of `docs/formats.md`. The oracle
fills them in like this:

- `instances` (active) and `deactivated` (`instance_deactivate_*`) are each sorted by instance id, with the same
  keys. `oLevel` deactivates everything more than 96 px outside the view every step, so most of a level is
  usually in `deactivated`. `instance_count` counts the active ones.
- `builtins` holds `view_xview[0]`, `view_yview[0]`, `view_wview[0]` and `view_hview[0]`, which decide what
  `oLevel` deactivates.
- `vars` and `globals` are only written with `--fields`.
- Reals are Rust's shortest round-trip form. Non-finite reals are `"NaN"`, `"inf"` and `"-inf"` in fixed fields
  and `{"real":"nan"}`, `{"real":"inf"}` and `{"real":"-inf"}` in `vars` and `globals`.
- No header line.

**Room changes.** A frame's line is written after the frame's events have run *and* after any room change they
requested has happened (`room_end`, then the new room's instances and their create events). So the first line
that shows a new room is the frame on which `room_goto` was called, and it already holds the new room's instances.
The new room's step events first run on the next frame: in `examples/start.txt`, frame 1315 is the first in
`rTutorial`, and frame 1316 is the first with deactivated instances. Frame 0 is the first frame of `rIntro`: the
boot room `rScreenInit` is entered and left before frame 0 (`oScreen`'s create event calls `room_goto_next`).

## Checking a build

The dumps are deterministic: the same replay gives byte-identical output on every run, from any starting cache,
and a run resumed from a savestate matches the full run from the savestate's frame on.

```sh
tools/oracle/run tools/oracle/examples/start.txt /tmp/start.jsonl
tools/oracle/examples/trim-dump /tmp/start.jsonl 0 1315 1316 | cmp - tools/oracle/examples/start.expected.jsonl
shasum -a 256 /tmp/start.jsonl
# abb3c8643dba863275e21fbb21d555d9e75cc57471a271687afe225592eceab2
```

`examples/start.expected.jsonl` holds frames 0, 1315 and 1316 of that dump, each cut down to its first three
active and first three deactivated instances by `trim-dump`, which copies what it keeps byte for byte.
`examples/returning.txt` is the same input from a returning player's highscore table. It ends in `rLevel`
instead of `rTutorial`, and its dump's SHA-256 is
`df1d33db88ca11a0b9faca7cefd0c9fc729f8c9fa3590131a205738cbf755b9d`. Both were re-run after patches 0007, 0008 and 0011
and still match: neither replay reaches the trigonometry, the fractional `distance_to_point` or the
`file_find_first` they change.

To check that the oracle's math matches the port's, build `tools/math-probe` in the oracle image (its Rust,
aarch64 Linux, and OpenGMK's release profile) against the same `vendor/pxfm` the oracle copies in, and compare
its `pxfm` hashes with a native run (see `tools/math-probe/README.md`). From the repo root:

```sh
image="$(bash -c 'source tools/oracle/common.sh && image')"
docker run --rm -v "$PWD":/repo:ro -e CARGO_TARGET_DIR=/tmp/t -e CARGO_PROFILE_RELEASE_LTO=thin \
  -e CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 -w /repo/tools/math-probe --entrypoint cargo "$image" run --release --quiet
```

## Patches

Applied in order by `tools/oracle/build`. They are GPLv2 (OpenGMK) and zlib (ramen) licensed; see
[`LICENSE`](LICENSE).

### `ramen/0001-aarch64-c_char.patch`

Two raw-pointer casts in ramen's X11 backend use `*const i8`, but `c_char` is `u8` on aarch64 Linux, so ramen
doesn't compile there. The casts now infer the type.

### `opengmk/0001-vendor-ramen.patch`

Points cargo at the patched ramen in `vendor-ramen/` (a `[patch]` section) and updates `Cargo.lock` to match.

### `opengmk/0002-glx-without-swap-control.patch`

OpenGMK refuses to start without `GLX_EXT_swap_control`, which Xvfb's Mesa doesn't have. Vsync doesn't matter
headless, so the swap interval becomes a no-op without it, and querying it (which crashes inside Mesa) returns 0.

### `opengmk/0003-dummy-externals-off-windows.patch`

The game calls `supersound.dll` for all sound and music. OpenGMK can only load DLLs on Windows, and upstream only
stubs externals while recording a TAS. This stubs them whenever not on Windows, during replays, or when
`OPENGMK_DUMMY_AUDIO` is set. Upstream's SuperSound stubs then apply (see [Caveats](#caveats)).

### `opengmk/0004-highscore-table.patch`

Implements `highscore_clear`, `highscore_add`, `highscore_value` and `highscore_name` (unimplemented upstream, and
the game calls them at startup) with an in-memory 10-entry table kept in savestates. The fresh table is ten
`<nobody>` entries scoring 0. `highscore_add` inserts above the first entry with a strictly lower score and drops
the 11th, and out-of-range places read as -1 and `""`. This is our implementation of GM8's behaviour, not
OpenGMK's or GM8's own code. Adding the table to savestates makes them incompatible with upstream's.

### `opengmk/0005-text-replays.patch`

Lets `-f` take a text replay (`.txt`), as described above: the input, plus the config files, other files and
highscore table to start with. Setting `OPENGMK_WRITE_GMTAS=<file>` also converts it to a `.gmtas` for OpenGMK's
TAS UI (input only).

### `opengmk/0006-state-dump.patch`

Writes the dump. Controlled by environment variables, which `tools/oracle/run` sets:

- `OPENGMK_DUMP=<file>`: write the dump there.
- `OPENGMK_DUMP_FIELDS=1`: include variables and globals.
- `OPENGMK_EXIT_AT_END=1`: exit after the replay's last frame instead of continuing with no input. A savestate
  requested with `-o` is still written first.

### `opengmk/0007-correctly-rounded-math.patch`

OpenGMK's `Real::sin`, `cos`, `tan`, `arctan` and `arctan2` call the port's vendored, correctly rounded math
(`vendor/pxfm`, which `build` copies into `vendor-pxfm/`) instead of the platform's libm. Every trigonometric use
of a `Real` goes through these: the GML built-ins, `point_direction`, `speed`/`direction`, gravity, and
`image_angle` bounding boxes. So the oracle and the port (`gm::math`) compute the same bits; glibc alone is 1 ulp
off on about 0.1–0.2% of the angles the game produces (851-3167, `tools/math-probe`). Trigonometry on plain
`f64` in OpenGMK's renderer only affects pixels and is left alone. The build stamp includes `vendor/pxfm`, so
changing it rebuilds the oracle. Patch 0015 then replaces these with the x87 model.

### `opengmk/0008-correctly-rounded-hypot.patch`

`distance_to_point` and `distance_to_object` call the vendored crate's correctly rounded `f_hypot` (our own code,
see `vendor/pxfm/README.spelunky.md`) instead of glibc's `hypot`, as the port's `gm::math::hypot` does (851-3174).
glibc's is 1 ulp off on 12-16% of the fractional arguments `oVampire`'s `distance_to_point` gets; on
`distance_to_object`'s whole gaps both are correctly rounded, so that one gives the same results as before. The
other `hypot` calls (`dot_product`, motion planning, particles, drawing) are left alone: the game uses none of
them.

### `opengmk/0009-highscore-dump.patch`

`OPENGMK_HIGHSCORES=<file>` writes the highscore table there when the run ends (at the replay's last frame,
before a savestate `-o` asks for, after the game end events when the window closes, or at `game_end` with patch
0010), one `{"name":...,"value":...}`
line per entry, best first, with the dump's number and string encoding. `tools/oracle/run` sets it, so `fidelity`
can compare the table the game leaves behind with the port's. The game directory's files need no patch: the
container script copies the changed ones out after the emulator exits.

### `opengmk/0010-stop-at-game-end.patch`

Upstream's replay loop handles `game_end` by restarting the game (`Game::restart`, which runs the room end and
game end events first) and playing on. With `OPENGMK_EXIT_AT_END` the oracle instead runs the room end and game
end events, dumps that frame and the highscore table, and exits, as GM8 ends the game and as the port does. So a
replay that ends the game matches the port through its last frame, and the files and table compared are the ones
the game end events leave (`stats.txt`).

### `opengmk/0011-file-find-backslash.patch`

`file_find_first` globs its mask as given, but glob only takes `/` as a separator off Windows, so the game's
`levels\*.lvl` (`oLoadLevel`'s level list) matched nothing on Linux. The mask's `\` now becomes `/` there, as GM8 on
Windows reads it. The names are also listed in NTFS's order (upper-cased, as the port's `file_find_first` does)
instead of glob's byte order, which differs for names with `_` and other characters between `Z` and `a`.

### `opengmk/0012-lenient-op-assign.patch`

`op=` ignores a type error and keeps the old value on built-in variables and the arguments too, not only on
fields: `x = 5; x += "a"` leaves 5 and `argument0 = "s"; argument0 -= 1` leaves `"s"`, as on GM8
(`tools/gm8-probe/probes/opassign.gml`: every operator, both operand orders, instance and game-wide built-ins,
`argumentN` and `argument[n]`). Upstream compiled them as strict and stopped the game.

### `opengmk/0013-gm8-number-literals.patch`

The GML lexer reads number literals as GM8's compiler does (851-3248), in doubles with every operation rounding:
`(v * 10 + code) - 48` per digit before the point, `f /= 10` and `v += d * f` after it, and hex digits as
`v * 16` plus the character code less that of `0`, `A` or `a` (plus 10 for a letter). Upstream multiplied all the
digits by 0.1^n at the end, which is 1 ulp off GM8 on some literals (`0.15`, `2.4`, `1.005`; of the game's, only
`oYellowBall`'s `0.15`), and read hex as a `u64`. Every literal of `tools/gm8-probe`'s `literals` probe now reads to
GM8's bits (`results/opengmk/literals.txt`; `crates/gml/tests/gm8_literals.rs`). See `docs/quirks.md` section 3.

### `opengmk/0014-gm8-arithmetic.patch`

Real GM8's number rules where OpenGMK had its own, as `tools/gm8-probe` found them on the GM8 runner (851-3246):
`string()` of a real is Delphi's `FloatToStrF(x, ffFixed, 10, 2)` at 53-bit precision (so `"0.13"` for 0.125,
`"0"` for -0), and `string_format` is `FloatToStrF(x, ffFixed, 18, decimals)`; `/ 0`, `div 0` and `mod 0` are
errors ("Division by 0.", "Modulo by 0."), while `/= 0`, and `/=` on a string, write the variable back unchanged
(patch 0012's lenient `op=`, which keeps the old value on any error); `exp` and `power` are errors when the result isn't finite, as OpenGMK's
`sqrt`, `arcsin` and `ln` already were (GM8 on a real Intel CPU traps the x87's exceptions); `!=`, `>` and `>=`
are true when the difference is NaN; `floor`, `ceil` and `div` (and so `mod`) truncate to 32 bits as Delphi's
`Floor`, `Ceil` and `Trunc` do; `round` and the bitwise operators go through a signed `Int64`, with x86 shift
counts, and all of these are an error past the `Int64` range (a new `IntOverflow`, "Unexpected error occured
when running the game.", where `string()` gives `"ERROR"`); `sign(NaN)` is 1. GM8 on a real Intel CPU traps the
x87's overflow too, and Delphi's `Round` behind every integer (851-3259), so: `+`, `-`, `*`, `/`, a comparison's
difference, `sqr` and `radtodeg` past the largest double are an error (a new `Overflow`, the same dialog); integer
arguments (`_arg_into!(int)`), array indices, `with` and `obj.x` targets, `repeat` counts and the integer
variables the port has go through an `Int64`, an error past it, then cut to 32 bits (`repeat` runs a count of
-2^31); the lenient `op=` still stops on those two errors; and `real()` of a string that reads as an infinity or
a NaN (`"1e400"`, `"inf"`) is an error. The arithmetic is ours: the patch adds the port's `crates/gm/src/gm8.rs`
whole, as `gml/gm8.rs`. Don't edit that part of the patch: change the port's file and run
`tools/oracle/sync-gm8-arith`, which rewrites it (`scripts/check` runs it with `--check`). The runner's own
rounding (`Real::round`, `to_i32`) is unchanged.

### `opengmk/0015-x87-math.patch`

GM8's math as its x87 computes it, from the vendored crate's `x87` model (`pxfm::x87::gm8`), as the port's
`gm::math` does (851-3257, `docs/quirks.md` §7): `Real::sin`, `cos`, `tan`, `arctan` and `arctan2` (in place of
patch 0007's correctly rounded ones), the `degtorad`, `radtodeg`, `point_direction`, `lengthdir_x` and
`lengthdir_y` built-ins, and the instance's `update_hvspeed` and `update_speed_direction`. Those were plain `f64`
formulas (`to_radians`, `to_degrees`, `rem_euclid`); GM8 keeps 80-bit constants and intermediates, which put it
1-2 ulps off them on 10-35% of angles, measured on a real Intel CPU (`tools/gm8-probe`). The motion snap now gives
+0 for a component snapped to zero, as GM8's does, and `lengthdir` snaps like the motion variables. The speed that
`update_speed_direction` recomputes is `point_distance`'s, snapped like the direction (851-3281), and a GML write
of `hspeed` or `vspeed` that changes it is an error from 2^63 up, where the snap's `Round` traps on Intel (the
`runtime.rs` hunk). Patch 0017 does the other angles.

### `opengmk/0016-clipboard-without-xcb.patch`

The `clipboard` crate is built only on Windows and macOS, so the oracle's Linux build leaves it out. On Linux
it reaches X11 through `x11-clipboard` and `xcb` 0.8.2, whose statics of an uninhabited type a future Rust will
reject (`cargo report future-incompatibilities`, rust-lang/rust#74840). Its only job is copy and paste in the TAS
UI's imgui, which the oracle never opens, and without it imgui keeps a clipboard of its own inside the process.
The game's `clipboard_*` functions are unimplemented upstream either way. `Cargo.lock` doesn't change: it lists
every target's dependencies.

### `opengmk/0017-x87-angles.patch`

The angles GM8's runner converts itself, as the port does (851-3262, `docs/quirks.md` §7): `image_angle` in
`update_bbox` and the five precise collision tests, and `view_angle` in `transform_point` (until patch 0019),
through `pxfm::x87::gm8::sin_cos_degrees` (the runner's `angle * Pi / 180`, rounded per step, in place of
`to_radians`); and gravity in `process_speeds` and the `motion_add` built-in, through `pxfm::x87::gm8::add_motion`
(the runner's routine for both, with the 80-bit sine and cosine in its products). Measured on a real Intel CPU
(`tools/gm8-probe`, "Angles"). Patch 0018 does the bounding box's formula, and patch 0019 the view transform's.
`action_move` and `action_set_motion`'s relative forms, `move_contact_*` and particles keep `to_radians`: the game
doesn't use them.

### `opengmk/0018-gm8-bbox.patch`

`update_bbox` computes the box with GM8's formula (the runner's routine at 0x4aa958), as the port's
`Instance::bbox_at` does (851-3278, `docs/quirks.md` §7): the mask's edges are taken from the origin before the
scale (`(left - ox) * xscale + x`, the far edge `((right - ox + 1) * xscale + x) - 1`), and a rotated box bounds
each side's products separately (`left = round((min(c*x0, c*x1) + x) + min(s*y0, s*y1))`, and so on). OpenGMK
rotated the four corners of `(x - ox * xscale) + left * xscale`: the same box in exact arithmetic, but a side near
a rounding boundary rounded the other way, on 119 of the 1,422 `angle-bbox` lines measured on a real Intel CPU and
on unrotated boxes at fractional positions.

### `opengmk/0019-gm8-view-transform.patch`

`transform_point`, the mouse's window-to-room transform through a view, is GM8's (the runner's code around
0x49ba4b), as the port's `View::to_room` (851-3279, `docs/quirks.md` §7): `pxfm::x87::gm8::view_to_room` doesn't
rotate below 0.01 degrees, puts the 80-bit sine and cosine of `-view_angle` straight into its products and adds in
the runner's order, where OpenGMK's `util::rotate_around` always rotated, with doubles. Measured on a real Intel
CPU (`tools/gm8-probe`, "Angles": all 778 `angle-view` lines). OpenGMK has no `window_mouse_set`, so the oracle
can't run that probe, and Spelunky never rotates a view, where the two transforms agree.

### `opengmk/0020-gm8-motion-setters.patch`

The instance's motion setters, as GM8's runner has them (851-3280, `docs/quirks.md` §5, read from its code and
measured by `tools/gm8-probe`'s `motion-setters`): `set_speed` and `set_direction` do nothing when written the value
already there, as `set_hspeed` and `set_vspeed` already didn't; `set_direction` wraps with
`pxfm::x87::gm8::wrap_direction` (Delphi's 32-bit `Floor`) in place of `rem_euclid`, and a GML write whose wrap
traps on Intel (a quotient from 2^63 up) is an error (the `runtime.rs` hunk), as is a `motion_set` whose
direction's does (`set_speed_direction` goes through both setters); and gravity and `motion_add` call a new `add_motion`, which always recomputes speed and
direction, where `set_hvspeed` skipped that when neither component changed. The direction recomputed from the
components (`hspeed` 0, a snapped 360) comes from `pxfm::x87::gm8::direction`, so it needs no hunk. Other callers of
`set_hvspeed` (`move_bounce_*`, `move_towards_point`) keep it: the game uses neither.

### `opengmk/0021-gm8-motion-traps.patch`

The runner snaps `hspeed` and `vspeed` through Delphi's `Round` into an `Int64`, which traps on Intel from 2^63 up and
below -2^63 (851-3283, `docs/quirks.md` §7, measured by `tools/gm8-probe`'s `speedwrite-*`, `direction-turn-2p63`,
`direction-radians-2p63`, `motion-set-2p63`, `motion-add-2p63` and `lengthdir-*`). `update_hvspeed` reports whether
both components' snaps fit (with `gml::gm8::round`, the port's `gm::math::snap_error`), `set_speed`,
`set_direction` and `set_speed_direction` pass it on, and a GML `speed` or `direction` write and `motion_set` are
errors where it doesn't; so are `lengthdir_x` and `lengthdir_y` where their own snap traps, and `motion_add` where
the speed it recomputes does, as an `hspeed` write already was (851-3281). The step's friction and gravity ignore it:
on Intel GM8 doesn't stop there (it skips the rest of the frame, 851-3290). So do `action_move`,
`action_move_point`, `action_create_object_motion`, `move_towards_point` and paths: the game uses none of them.

### `opengmk/0022-gm8-main-loop-exceptions.patch`

After the game has started, GM8's main loop catches an exception (the x87's traps, running out of memory) with an
empty handler that drops the rest of its iteration: the frame's remaining events and stages, its room change and
the end of its input (851-3290, `docs/quirks.md` §5, measured by `tools/gm8-probe`'s `late-*`, `step-trap*`,
`gravity-overflow` and `move-overflow`). Only at game start does one stop the game. `gml::Error::is_exception` (the
port's `gm::Error::is_exception`) names them: `IntOverflow` and `Overflow`, which a string too long to build
already reports. `play_replay` and `run` catch them from `frame` and from a room change (`load_room`, `restart`,
`load_gm_save`, boxed: `is_boxed_exception`), and skip that iteration's room change after one; `game_end`'s events,
which GM8 runs after its loop, still stop the run. The motion stage moves one instance at a time, as GM8 does:
`process_speeds` now does one instance's friction and gravity, and `frame` moves it before the next. Friction's sum,
AddMotion's sums and the move's sums trap past the largest double (`apply_speeds` returns the error, the collision
stage's solid rule included), and the setters store what GM8 stores before a trap, as the port's `gm::Instance`
does: a direction write leaves the components alone where its wrap traps, `update_hvspeed` (with
`pxfm::x87::gm8::motion_unsnapped`) leaves `vspeed` unsnapped where `hspeed`'s snap traps, and
`update_speed_direction` keeps the old speed where the new one overflows. `set_hspeed`, `set_vspeed`,
`set_hvspeed` and `add_motion` return the error, which `motion_add`, `action_set_*`, `action_move_point` and the
`hspeed`/`vspeed` writes pass on, and `motion_set` stops at its direction's error, as GM8's does. The bounces
(`move_bounce_*`) ignore it: the game never bounces.

### `opengmk/0023-gm8-runner-traps.patch`

The rest of the runner's own x87 arithmetic traps where GM8's does on Intel, as the port's does (851-3301,
`docs/quirks.md` §5 and §7, measured by `tools/gm8-probe`'s `anim-trap`, `draw-floor-trap`, `sprite-floor`, `bbox-*`,
`bg-trap` and `view-trap`). `frame`'s background scroll and animation store through `gml::fstp` (the animation
compares Delphi's 32-bit `Floor` of `image_index`, `gm8::floor`), `set_sprite_index` takes the same `Floor`, and
`draw_instance_default` too, its trap no longer ignored with its other errors. The views round their targets with
`gm8::int_arg`. `Instance::gm8_bbox` computes GM8's box with the sides that trap left as the mask's, `update_bbox` stores
it, and `Game::check_bbox` raises the exception where GM8's setters recompute the box: `x`, `y`, the scales and the
angle when changed, `sprite_index` and `mask_index` always, and `Game::set_position`, GM8's position setter, which the
move, the collision stage's moves back, `move_snap` (whose own `Round` of each quotient traps too), `place_meeting` and `instance_place` (there and back) now go
through. `instance_create` of an instance whose box traps stops the game with an error (GM8's instance list is left
with a hole).

## Caveats

- **OpenGMK is not the real runner.** It's a careful reimplementation of GM8, but it has bugs and gaps of its own.
  When the port and the oracle disagree, the oracle is the best evidence we have, not proof. Spot-check suspicious
  mismatches against the GML (`docs/source-inventory.md`) and what's known about GM8 before "fixing" the port, and
  note confirmed OpenGMK deviations in `docs/quirks.md`.
- **Trigonometry, angles, motion and `hypot` are the port's** (patches 0008, 0015, 0017, 0018, 0019, 0020, 0021,
  0022 and 0023): a model of GM8's x87 that matches every value measured on real Intel hardware, and a correctly rounded
  `hypot` (GM8's is the x87's, unmeasured on fractional gaps). Other libm calls are still glibc's (aarch64): `exp`, `ln`,
  `power`, `arcsin` and `arccos`, and `hypot` outside `distance_to_object` and `distance_to_point`, none of which
  the game calls.
- **The highscore table is ours** (patch 0004). The real GM8 keeps it on disk, and its fresh contents (names,
  scores) aren't verified. The game only compares values with its offsets, and never reads names, so the
  defaults are unlikely to matter.
- **Unimplemented functions stop the run.** Upstream leaves some GML functions unimplemented, and calling one
  panics. Besides the highscore functions (patch 0004), the ones the game uses are `show_message`, `show_menu`,
  `show_info` and `get_integer`, all in the leftover SuperSound demo object `core`, which nothing creates.
- **No sound.** SuperSound calls are stubbed (OpenGMK's `should_dummy` in `gm8emulator/src/game/external.rs`): the
  `SS_Init` symbol returns `"Yes"` (so does `SS_Unload`, which is bound to it) and every other export the real 0.
  So `SS_LoadSound` returns 0, and the 62 sound and music handles the game keeps in globals (`global.snd*`,
  `global.mus*`) are 0, not the DLL's handle strings. Every `SS_*` wrapper script starts with
  `if is_real(argument0) then return ...`, so from then on no call reaches the DLL and each query returns its
  wrapper's fixed value: `SS_IsSoundPlaying`, `SS_IsSoundLooping`, `SS_IsSoundPaused`, `SS_GetSoundFreq` and
  `SS_GetSoundPosition` return -1, `SS_GetSoundVol` the string `"-1"`, `SS_GetSoundPan` the string `"0"`, and so
  on. The game only branches on these to decide whether to make more sound calls (`playMusic`, one-at-a-time
  effects, the bow, muting music on pause, `scrMusicFade`), so no instance or game variable depends on them; only
  the handle globals differ from a real run. The port's `audio` crate reproduces the stub in its `OracleCompat`
  mode: run the port in that mode when diffing against the oracle, and `--fields` diffs match on those globals too.
  The default `Faithful` mode models the real DLL. See the `audio` crate docs for the full table.
- **No joystick.** OpenGMK reports no joystick, so `oGamepad` sees nothing even though the shipped
  `settings.cfg` enables the gamepad.
- **The clock doesn't matter.** OpenGMK spoofs it from the replay's `time` and advances it one frame per frame,
  but Spelunky never reads it: no `current_time`, `current_*` or `date_*` (`docs/source-inventory.md`, section 5).
  Nor does it seed the RNG itself, so the replay's `seed` decides every random number.
- **Savestates are OpenGMK's** (bincode, including our highscore table) and only load in the same build. Resuming
  restores the game but not its directory: files the game wrote before the savestate (`stats.txt`) are back to
  their starting contents.
