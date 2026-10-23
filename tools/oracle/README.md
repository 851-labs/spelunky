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

The table isn't saved when the run ends. Savestates carry it, so with `--savestate-in` the lines have no effect.

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
`df1d33db88ca11a0b9faca7cefd0c9fc729f8c9fa3590131a205738cbf755b9d`. (Neither replay reaches code whose
trigonometry patch 0007 changes, so these hashes predate it.)

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
changing it rebuilds the oracle.

## Caveats

- **OpenGMK is not the real runner.** It's a careful reimplementation of GM8, but it has bugs and gaps of its own.
  When the port and the oracle disagree, the oracle is the best evidence we have, not proof. Spot-check suspicious
  mismatches against the GML (`docs/source-inventory.md`) and what's known about GM8 before "fixing" the port, and
  note confirmed OpenGMK deviations in `docs/quirks.md`.
- **Trigonometry is the port's, not GM8's** (patch 0007). GM8 used the x87 FPU, which neither reproduces; the
  oracle and the port agree with each other instead. Other libm calls are still glibc's (aarch64): `exp`, `ln`,
  `power`, `arcsin` and `arccos`, which the game never calls, and `hypot` in `distance_to_object` and
  `distance_to_point`. For `distance_to_object` that's harmless: its arguments are whole bounding-box gaps, where
  glibc's `hypot` equals `sqrt(x*x + y*y)` exactly (checked on every pair up to 2048). `distance_to_point` (one
  call, in `oVampire`) gets fractional arguments, where it differs from that in about 12% of cases, so the port
  must match glibc's `hypot` there or the oracle must change (not decided yet; a follow-up).
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
