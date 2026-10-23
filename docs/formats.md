# Replay and trace formats

We verify the port by running the same **replay** (inputs) on the original game under the oracle (OpenGMK,
patched; 851-3118) and on the port. Each run writes a **trace** (the state after every frame), and
`trace-diff` finds the first frame where the two traces differ. The `trace` crate reads and writes both
formats (`crates/trace`), and `trace-diff` is `crates/tools/src/bin/trace-diff.rs`.

The oracle defined both formats first. The formats below are the oracle's, plus some **extensions**, each
marked as one. The oracle doesn't emit or accept the extensions yet. A file that uses no extensions is
exactly what the oracle reads and writes.

A *frame* is one GM step (the ticket says "tick"), at 30 per second. Frames are numbered from 0.

## Replays

A replay is a line-based text file. `#` starts a comment, except on `file` and `highscore` lines, which end
in a JSON string. Blank lines are ignored.
Here is one of the oracle's own replays:

```
seed 12345
frames 1700
1200 hold left 70
1285 hold up 5
```

### Directives

| Line | Meaning | Default |
|---|---|---|
| `seed N` | The RNG seed at the start of frame 0, as an `i32`. GM 8 randomizes the seed at startup; a replay pins it. | 0 |
| `time N` | The spoofed start of the system clock, in ns since the Unix epoch (`current_time`, dates). | 1262304000000000000 (2010-01-01) |
| `frames N` | How many frames to run. Every event must fall inside this range. | last event frame + 1 |
| `room NAME` *(ext)* | The first room. Any room other than the default skips `scrInit` and the boot rooms. | `rScreenInit` |
| `settings a b c d e f g` *(ext)* | `settings.cfg`: fullscreen, graphicsHigh, downToRun, gamepadOn, screenScale, musicVol, soundVol. | `0 1 0 1 3 15 15` (shipped) |
| `keys` + 12 ints *(ext)* | `keys.cfg`: the VK codes for up, down, left, right, jump, attack, item, run, bomb, rope, flare, pay. | `38 40 37 39 90 88 67 16 65 83 70 80` |
| `gamepad` + 8 or 9 ints *(ext)* | `gamepad.cfg`: the joystick buttons for jump, attack, item, run, bomb, rope, flare, pay, [start]. The shipped file has 8 lines. | `2 1 3 5 7 8 4 6` |
| `joystick` / `joystick pov` *(ext)* | A joystick is connected (`joystick_exists`), and whether it has a POV hat. | none connected |
| `highscore VALUE "json name"` *(ext)* | One highscore-table entry at startup, repeated in table order, best first. The game stores its stats here (inventory 7.2). | the runner's initial table |
| `file PATH "json string"` *(ext)* | Another file in the game directory at startup, such as `stats.txt` or `levels/x.lvl`. The contents are exact. PATH must stay inside the game directory: it uses `/`, has no spaces, and has no `.`, `..` or empty components, backslashes or drive letters. | none |

The three `.cfg` files are rendered exactly as `config.exe` writes them: values joined by CRLF, with no
trailing newline (`ReplayHeader::game_files`). Both runners must put these bytes in the game directory.

### Events

`F VERB ...` happens at frame `F`. All of a frame's events are applied in order **at the start of that
frame**, before its begin step. Events on the same frame keep their file order. Lines don't have to be
sorted by frame.

| Event | Meaning |
|---|---|
| `F press B` | B goes down. |
| `F release B` | B goes up. |
| `F hold B N` | Shorthand for `F press B` plus `F+N release B`. B is down during frames F to F+N-1. |
| `F wheel up` / `F wheel down` *(ext)* | One mouse wheel event. |
| `F mouse X Y` *(ext)* | The mouse moves to (X, Y) in window client pixels. The runtime maps this into the room. |
| `F axis x\|y\|z\|pov V` *(ext)* | A joystick axis (-1 to 1), or the POV hat in degrees (-1 is centred), changes to V. |

A button `B` can be any of these:

- A key, written as a letter or digit (`Z`, `5`), a name (`left up right down enter return escape esc space
  shift control ctrl alt tab`; *(ext)* also `backspace pause pageup pagedown end home insert delete
  printscreen numpad0`-`9 multiply add subtract decimal divide f1`-`f12`), or a VK code (`13`, or *(ext)*
  `vk13`). A code needs two or more digits, since `5` is the digit key. Codes 0 and 1 are rejected; they are
  GM's pseudo-keys.
- *(ext)* `mb_left`, `mb_right` or `mb_middle`.
- *(ext)* `joy1` to `joy32`. These require the `joystick` directive.

Input is raw: transitions of physical keys and buttons, not the game's `checkJump`-style actions. The same
replay therefore works under any key binding and drives every path that reads input (`keyboard_check*`,
`keyboard_key`, keypress events, `oGamepad`). The runtime derives the held and pressed/released state of each
frame from the events. `Replay::inputs()` does this; it yields each frame's events and the resulting
`InputState`. `Recorder` goes the other way and turns per-frame state into events, for recording in-game.
The event list is already compact: an idle frame costs nothing.

When the crate writes a replay, it emits extensions only when they differ from the defaults. It turns `hold`
into a `press`/`release` pair and writes keys under names the oracle knows.

## Traces

A trace is JSON lines, one object per frame, as the oracle writes it. Each line is written **after the
frame's draw events and after any room change requested during the frame** has happened. It is the state the
next frame starts from, so after a `room_goto` the line already shows the new room and its creation events.

```json
{"frame":0,"room":100,"room_name":"rIntro","seed":1754002487,"instance_count":431,"instances":[{"id":130330,"object_index":610,"object":"oDesert","x":0.0,"y":224.0,"xprevious":0.0,"yprevious":224.0,"hspeed":0.0,"vspeed":0.0,"speed":0.0,"direction":0.0,"gravity":0.0,"friction":0.0,"image_index":0.0,"image_speed":1.0,"image_xscale":1.0,"image_yscale":1.0,"depth":100.0,"sprite_index":779,"sprite":"sDesertNight","visible":true,"solid":true,"alarms":{}}]}
```

### Frame keys

| Key | Meaning |
|---|---|
| `frame` | The frame number. It is the only required key, and it strictly increases. A trace may start late or skip frames. |
| `room`, `room_name` | The room index and name. |
| `seed` | The runner's RNG state as an `i32` (OpenGMK's `rand.seed()`). |
| `instance_count` | The number of active instances. |
| `instances` | The active instances, **sorted by id**. |
| `globals` | `global.*` variables, without the prefix. The oracle writes this only with `OPENGMK_DUMP_FIELDS=1`. |
| `deactivated` *(ext)* | Deactivated instances (`instance_deactivate_*`, `oLevel` culling), sorted by id, in the same shape as active ones. If absent, they weren't dumped. |
| `builtins` *(ext)* | Runner built-ins that are neither per-instance nor global, keyed by GML name with the index written out: `view_xview[0]`, `keyboard_key`, and so on. |

### Instance keys

`id`, `object_index`, `object` (name), `x`, `y`, `xprevious`, `yprevious`, `hspeed`, `vspeed`, `speed`,
`direction`, `gravity`, `friction`, `image_index`, `image_speed`, `image_xscale`, `image_yscale`, `depth`,
`sprite_index` (-1 for none), `sprite` (name, `""` for none), `visible`, `solid`, `alarms`, and `vars`.

- `alarms` maps the alarm index (`"0"` to `"11"`) to an integer. Like OpenGMK, it holds only the alarms that
  were ever set. **A missing alarm means -1.**
- `vars` holds the instance's own variables. The oracle writes it only with `OPENGMK_DUMP_FIELDS=1`.
- Only `id` and `object` are required. Any other key may be absent because it was filtered out or not
  dumped. `null` means the same as absent.

### Values

- **Reals** are JSON numbers, written as the shortest decimal that parses back to the same `f64` (Rust
  `{:?}`, `ryu`, Python `repr`). Readers must parse them exactly; the crate turns on serde_json's
  `float_roundtrip` for this. JSON can't hold NaN or infinity:
  - In a field that is always a real (`x`, `depth`...), the oracle writes them as the strings `"NaN"`,
    `"inf"` and `"-inf"`.
  - In a variable, a string is a GML string, so *(ext)* they are written `{"real":"nan"}`, `{"real":"inf"}`
    and `{"real":"-inf"}`. Readers accept both forms anywhere a real is expected.
- **Strings** are JSON strings.
- **Arrays** are objects from flattened index to element, `{"0":1.0,"32001":"a"}`, as OpenGMK writes them.
  GM 8 arrays are 2-D: `a[i,j]` is stored at `i*32000 + j`, and `a[j]` is `a[0,j]`. Only elements that were
  set appear. GML reads a plain `a` as `a[0]`.

### Header line *(ext)*

A trace may start with a header such as
`{"format":"spelunky-trace","version":1,"game":"spelunky-classic-1.1","source":"port","filter":{...}}`.
Readers tell it apart from a frame by its `format` key. It records who wrote the trace and which filter was
applied. The oracle doesn't write one; `TraceWriter::new` doesn't either, but `TraceWriter::with_header`
does.

### Filters

A frame with variables is about 170 KB. A `Filter` keeps traces manageable. The writer applies it before writing (`TraceWriter`), and `trace-diff` applies it before
comparing. It has four lists of globs, where `*` matches any run of characters: `objects`, `ignore_objects`,
`fields`, and `ignore_fields`. Field patterns match these **paths**:

- Frame paths: `room`, `room_name`, `seed`, `instance_count`, `globals.NAME`, `builtins.NAME`.
- Instance paths: `active`, `object_index`, `x` and the other real built-ins, `sprite_index`, `sprite`,
  `visible`, `solid`, `alarms[0]` to `alarms[11]`, `vars.NAME`.
- Array elements: `vars.NAME[j]` and `vars.NAME[i,j]`. Element 0 is `vars.NAME` itself.

A pattern also matches every path below it, so `vars` matches all variables and `alarms` matches every
alarm. A `fields` list always keeps `room`, `room_name`, `seed`, `instance_count` and `active`, unless they
are listed in `ignore_fields`. An instance's `id` and `object` are never filtered out.

The writer filters array elements one by one, by the same paths the diff compares: `vars.grid[3]` keeps only
that element, and `vars.grid` keeps the whole array. It drops filtered keys but keeps the map itself, so
`"vars":{}` can mean "every variable filtered out".
That is why the header records the filter. `trace-diff` applies a trace's recorded filter to **both** traces,
so what one writer dropped is never compared against the other trace.

## trace-diff

```
trace-diff [options] <a.jsonl[.gz]> <b.jsonl[.gz]>     (`-` reads stdin)
  --json                  machine-readable report
  --from F / --to F       only frames in this range
  --tolerance X           reals at most X apart are equal (default: exact)
  --match auto|id|object  instance pairing (default auto)
  --objects L / --ignore-objects L / --fields L / --ignore-fields L   filters (comma-separated globs)
  --context N             frames of history before the divergence (default 3)
  --max-diffs N           differences listed (default 50, 0 = all)
exit status: 0 identical, 1 diverged, frames only in one trace, or nothing compared; 2 error
```

For example: `scripts/cargo run --release -p tools --bin trace-diff -- oracle.jsonl.gz port.jsonl
--ignore-fields vars.randNum`.

How frames are compared:

- **Frames are paired by number.** A frame that only one trace has isn't compared. It is counted
  (`frames_only_in_a`/`_b`), and the outcome is `unmatched_frames` if everything else matched. Use this for a
  savestate resume that starts at frame 1300, and `--from`/`--to` to restrict the range. If no frame is in
  both traces, the outcome is `nothing_compared`. That exits 1, so an empty range never passes.
- **Instances are matched** by id when the objects agree. In `auto` mode, the rest are matched by object and
  creation order: within each object, unmatched instances are sorted by id and paired in that order. `id`
  pairs by id alone, and a different object is then reported as an `object` difference. `object` ignores ids
  entirely, for traces whose ids are allocated differently. Deactivated instances take part (with
  `active: false`) when both traces dumped them.
- **Comparison is exact.** Reals are compared bit for bit, so `0` and `-0` differ, while NaN equals NaN.
  `--tolerance` allows an absolute difference.
- **Missing values.** If one trace didn't dump a field or map at all (the oracle without fields has no
  `vars` or `globals`; only the port dumps `deactivated`), it isn't compared. The report lists it under
  `not_compared_only_in_a`/`_b`. Inside a map that both traces dumped, a key on only one side is a difference
  (shown as `<absent>`, or `null` in JSON). A missing alarm counts as -1, and a scalar `a` equals `a[0]`.

The report gives the first diverging frame, every difference in it (field differences and instances only in
one trace), and the **history** of each differing field over the `--context` compared frames before it:

```
diverged at frame 1260 in rTitle, after 1260 equal frames: 1 difference
  oPlayer1 #135611 x: a=424 b=1424
history (frame: value, or a/b where they differ):
  oPlayer1 #135611 x: 1256: 436 | 1257: 433 | 1258: 430 | 1259: 427 | 1260: 424 / 1424
```

`--json` prints the same as one object:

- `outcome`: `identical`, `diverged`, `unmatched_frames`, or `nothing_compared`.
- `equal_frames` and `last_equal_frame`.
- `divergence`: has `frame`, `room_name_a`/`_b`, `total_differences`, `differences`, and `history`.
  - Each entry in `differences` has a `kind`. A `field` entry has `path`, `a`, `b`, and `a_instance`/
    `b_instance` (`{id, object}`, absent for frame-level fields). An `only_in` entry has `side`, `instance`,
    and `active`.
  - Each entry in `history` has `path`, the instances, and `points` of `{frame, a, b}`.
- `frames_only_in_a`/`_b`: `{count, first, last}`.
- `not_compared_only_in_a`/`_b`.
- `warnings`.

## Proposed oracle changes

These would let the oracle use the extensions above:

1. Dump deactivated instances as `deactivated`.
2. Write non-finite reals in `vars`/`globals` as `{"real":"nan"}`. A bare `"NaN"` there reads as a GML
   string.
3. Optionally, dump `builtins`. The views drive `oLevel`'s deactivation region, so `view_xview[0]` and
   `view_yview[0]` are the most useful.
4. Accept the replay extensions it can support (`settings`, `keys`, `gamepad`, `file`, `highscore`, mouse
   events). Joystick events need joystick support, since OpenGMK's `joystick_*` functions are stubs.
