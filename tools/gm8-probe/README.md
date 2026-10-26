# GM8 probes

Small GML programs that compute values and write them to a file, run on the **real GameMaker 8 runner**
(under Wine) and on the oracle's OpenGMK, so runner behavior that isn't in the game's source can be checked
against GM8 itself rather than against a reimplementation. No Windows install or GameMaker license is used.

## How a probe becomes a GM8 game

There's no free GM8 compiler, but a GM8 game is the runner with the game data appended, and the runner compiles
every script from its source text when the game starts. So `build-exe.py` takes `Spelunky.exe` (from
`reference/spelunky_1_1.zip`) and swaps the source of a few of its scripts:

- `scrReadStats`, the first script the game runs (oScreen's create event in `rScreenInit`), becomes the probe,
  followed by `out("end", "ok")` and `game_end()`.
- `scrGetFavorMsg` and `scrGetName` become `probes/lib/hex.gml` and `probes/lib/out.gml`; calls to `hex(` and
  `out(` in the probes are renamed to them.
- The SuperSound scripts, `scrInit`, `scrWriteStats` and `stopAllMusic` become empty, so the game touches
  neither the DLL nor its save files.

`gm8patch.py` does the swap: it finds the GM 8.0 game data, undoes its byte-swap encryption, replaces the
scripts' zlib blocks and encrypts the result again. It was written from the format (`gm8patch.py`'s docstring
describes it), copying nothing from GameMaker or OpenGMK. Only the plain GM 8.0 layout is supported, which is
what `Spelunky.exe` uses. Nothing built goes in the repo: the patched exe lives in a temp dir for one run.

Each line a probe writes is `name = <IEEE-754 bits in hex> <string(value)>` for a real, or `name = "text"` for
a string. `hex` reads the bits with operations that are exact at 53-bit precision; it avoids `floor` above 4,
because GM8's `floor` is wrong from 2^31 up (see `results/`).

## Running

```sh
tools/gm8-probe/run wine     tools/gm8-probe/probes/numbers.gml /tmp/p/wine      # real GM8 under Wine
tools/gm8-probe/run opengmk  tools/gm8-probe/probes/numbers.gml /tmp/p/opengmk   # the oracle (tools/oracle/build first)
tools/gm8-probe/run-all wine /tmp/p/wine                                         # every probe
```

`wine` runs Debian's Wine 8 (32-bit) in an amd64 container (`Dockerfile`) with Xvfb, an ALSA null sound device
(without one the runner stops on "No sound device detected") and Mesa's software GL (for Direct3D 8 through
wined3d; `container/run-probe` picks softpipe, because llvmpipe raises an x87 exception that kills the runner
on real hardware). On Apple Silicon, OrbStack runs the container under Rosetta. The image is built on first
use. A run takes 30-60 s, most of it Wine starting under emulation.

On an Intel Mac (real x87), Colima with an x86_64 VM runs it natively. Two things to set there: `TMPDIR` must
be under the home directory (the only one Colima shares with its VM), and the VM's `qemu-i386` binfmt handler
must be off (`echo 0 | sudo tee /proc/sys/fs/binfmt_misc/qemu-i386` in `colima ssh`), or the 32-bit Wine
loader runs under QEMU instead of on the CPU.

The unmodified `Spelunky.exe` boots the same way, with the real `supersound.dll`, and plays its intro
(screenshotted 70 s in); playing it would need input injected (`xdotool`), which nothing here does yet.

`GM8_PROBE_MEMORY` (e.g. `4g`) caps the container's memory, for the probes that build huge strings: GM8 is a
32-bit process, but the oracle isn't. A probe whose game hasn't ended after `GM8_PROBE_TIMEOUT` seconds (default
60) is stopped; under Wine, a screenshot of the screen goes to `OUTDIR/screen.png`, which is how a GM8 error
dialog shows up. `out` reopens `probe.txt` for every line, so the lines before an error are kept. The probes in
`probes/errors/` each test one thing that might be an error. On a real Intel CPU (and so on the oracle, which
follows it) anything that makes a NaN or an infinity is an error, arithmetic past the largest double included, and
so is any integer past 2^63 (`floor`, `round`, a bitwise operator, a built-in's argument, an array index, a
`repeat` count), so the probes that make them (`nonfinite`, `ops`, the second half of `numbers`) stop early there.
`in-range` repeats their lines that don't, under the same names, and `string-format-nonfinite` holds
`string-format`'s.

`results/` holds the outputs of the last runs (`gm8-wine/` under Rosetta, `gm8-intel/` on a real Intel CPU,
`opengmk/`), with a `# stopped:` line where an error ended a probe (OpenGMK's message, or the text of GM8's dialog
from its screenshot). `python3 -I compare.py` prints the lines that differ. `crates/gm/tests/gm8_probe.rs` checks
that the port computes the three runs' answers for the lines it can, and `string()` of every real any probe
printed; it lists the lines where it knowingly differs from GM8, and takes the lines where GM8 on Intel stops
with an error (and Rosetta doesn't) as Intel's.

## What GM8 answered

`docs/quirks.md` records each answer where it matters (status "GM8", and §7 for the differences). Where GM8 and
unpatched OpenGMK disagreed on a clear rule, the port and the oracle follow GM8 (851-3246, oracle patch 0014), and
`results/opengmk/` is the patched oracle's: the port and the oracle give GM8's answers, except for `max`/`min`
(not in the port) and the transcendentals (851-3247). Where GM8 under Rosetta and on
Intel disagree, they follow Intel. In short:

The GM8 column is GM8 under Rosetta (`results/gm8-wine/`); where GM8 on a real Intel CPU (`results/gm8-intel/`)
answers otherwise, the row says so ("Real x87" below).

| Question | GM8 (Wine) | OpenGMK before patch 0014 |
|---|---|---|
| FPU precision for GML arithmetic | 53 bits (plain doubles) | f64 |
| `if (0.5)`, `if (0.5 - 2^-54)` | true, false | same |
| Comparison tolerance | 1e-13 (9e-14 equal, 1.1e-13 not) | same |
| Comparisons with NaN, `inf == inf` | `!=`, `>`, `>=` true, from either side; `inf == inf` false (Rosetta only: on Intel, `exp(1000)` is an error, so the probe makes no infinity) | all false |
| `round` ties, `round(-0.5)`, past 2^31; past 2^63 | half to even, +0, no wrap; Rosetta: -2^63 | same; exact |
| `floor`/`ceil` past 2^31; `ceil(-0.5)`; of inf, NaN | wrap to 32 bits; +0; Rosetta: -1 and 0 | exact; -0; inf, NaN |
| `floor`, `round`, `div`, bitwise operators past 2^63, `string()` there | **Intel: errors** (`floor(1e20)`: "Unexpected error occured when running the game."), `string(1e22)` is `"ERROR"`; Rosetta doesn't trap the x87, so -2^63 and `"1E22"` | exact |
| `string(0.125)`, `string(-0)`, `string(-0.004)` | `"0.13"`, `"0"`, `"0.00"` | `"0.12"`, `"-0"`, `"-0.00"` |
| `string` rounding (`strings` probe: 65 values) | Delphi's `FloatToStrF(x, ffFixed, 10, 2)` at 53 bits | Rust's `{:.2}` |
| `string` of big fractions, inf, NaN | `"1.099511628E12"`; Rosetta: `"INF"`, `"-9223372036854775808"` | every digit, `"inf"`, `"NaN"` |
| Real literals (`literals` probe: 195, decimal, whole and hex) | not correctly rounded: per character `(v * 10 + code) - 48`, then `v += d * f` with `f /= 10`; `0.3` is `0x3fd3333333333334`, `0.15` `0x3fc3333333333334`, `9007199254740991` 2^53. Intel: the same bits, all 195 | OpenGMK (all digits times 0.1^n) is 1 ulp off on some (`0.15`, `2.4`, `1.005`); patched to GM8's (patch 0013) and the port reads them the same |
| `real("1e3")`, `real(" 12 ")`, `real("")`; `real("12abc")` | 1000, 12, 0; error | same |
| `x / 0`, `x div 0`, `x mod 0` (-0 too); `x / 1e-300` | errors; 1e300 | inf/NaN |
| `+ - * /`, a comparison, `sqr`, `radtodeg` past the largest double (`2^1023 * 2`, `1 / 2^-1074`, `2^1023 > -2^1023`); underflow (`2^-1074 / 2`) | **Intel: errors** ("Unexpected error occured when running the game."); Rosetta: ±inf. 0 | ±inf; 0 |
| Integer arguments, array indices, `alarm[0] = x`, `repeat` counts, `real * string`'s count: past 2^32; past 2^63 | cut to 32 bits through an `Int64` (`chr(2^32 + 65)` is `"A"`, `(2^32 + 2) * "a"` `"aa"`; `repeat` and `real * string` run -2^31 2^31 times but no other negative count, so `2^31 * "a"` runs out of memory: "Unexpected error occured when running the game."); **Intel: errors** (Rosetta: -2^63, so 0, and `2^63 * "a"` is `""`) | saturated to `i64`, then 32 bits; `repeat` uncut; `2^31 * "a"` is `""` (patch 0014 follows GM8, 851-3272) |
| `a \|= 2^63`, `a *= 2` past the largest double, in an `op=` (local, field, built-in, argument) | **Intel: errors**, unlike type errors (Rosetta: -2^63, inf) | the old value kept |
| `real("inf")`, `real("nan")`; `real("1e400")` | errors; **Intel: error** (Rosetta: inf) | inf, NaN; inf |
| A literal past the largest double (`1e309`) | **Intel: the game stops while it loads**; Rosetta: inf | inf |
| `point_distance(0, 0, 2^1023, 0)`, `(0, 0, 2^-600, 0)`; past the largest double | 2^1023, 2^-600: the squares stay in a register with the x87's wider exponent (`pdist-range`); **Intel: error** (Rosetta: inf) | inf, 0; inf (the port and patch 0015 follow GM8, 851-3271) |
| The runner's `speed` after `hspeed = 5.00001`, `hspeed = 2^-600`; after `hspeed = vspeed = 3 * 2^61`, `= 2^1023` | 5, 0: `point_distance`'s, snapped to a whole number within 0.0001 like `direction` (`motion-snap`, `motion-range`; `direction` itself as expected); **Intel: errors**, the snap's `Round` of a speed from 2^63 up (`speed-2p63`; Rosetta: 1.06 * 2^63 and 2^1023 * sqrt(2), the squares in a register) | 5.00001, 0; 1.06 * 2^63, inf (the port and patch 0015 follow GM8, 851-3281) |
| `hspeed = 0.3; vspeed = 0.7`, then `speed = speed`; `direction = -360`, `= 360 * 2^31 + 90`, `= 360 * 2^64`; `hspeed = -0` at rest; `hspeed = 1; vspeed = 0.00000001`; `motion_add(0, 0)` | `hspeed` 0.3 (a write of the value already there does nothing; `hspeed` and `vspeed` keep what was written); +0, 773094113370 (Delphi's 32-bit `Floor`), **Intel: error** (its `Trunc`; Rosetta: unwrapped); `direction` 0; 0 (360 wraps); `direction` recomputed, 1 ulp off at times (`motion-setters`, `direction-2p63`, 851-3280) | `0x3fd3333333333330`; -0, 90, 0 (`rem_euclid`); 180; 360; unchanged (the port and patch 0020 follow GM8, 851-3280) |
| `speed = 2^63` (the components' snap), `lengthdir_x(2^63, 0)` | **Intel: errors** (`speedwrite-2p63`, `lengthdir-2p63`); Rosetta: 2^63 | 2^63 (not followed yet: `docs/quirks.md` §7) |
| `make_color_rgb(-1, 1, 0)`, `(2^32 + 1, 0, 0)`, `(0, 0, 2^48)` (`color-components`: components past a byte and past 32 bits) | 255, 4294967297, 0: `r + g * 256 + b * 65536` on `Int64` components, wrapping; the same on Intel | -1, 1, 0: `r \| g << 8 \| b << 16` on 32-bit components (patch 0014 follows GM8, 851-3270) |
| `n * "a"` for `n` of 2^28 + 1, `19 * 2^24` (304 MiB), `39 * 2^23` (312 MiB), 2^29; 2^23 copies of 64 bytes | built, built, "Unexpected error occured when running the game." (out of memory), the same; the same error: the limit is on bytes. The same on Intel (twice) and under Rosetta (`mulstr-2p28+1`, `mulstr-19x2p24`, ...) | built, up to `i32::MAX` bytes (the port and patch 0014 stop past 304 MiB, 851-3282) |
| `v /= 0` (local, instance or built-in variable); `v /= 2` on a string | `v` written back unchanged, no error (`obj.x /= 0` copies the first's `x`) | inf/NaN |
| `string_format` | Delphi's `FloatToStrF(x, ffFixed, 18, decimals)`, padded; the same on Intel | half to even, Rust's formatting |
| `sqrt(-1)`, `arcsin(2)`, `ln(0)`, `power(-8, 1/3)`, `exp(1000)` | Rosetta: NaN, NaN, -inf, NaN, inf. **Intel: errors** ("Cannot apply sqrt to negative number.", "Error in function arcsin().", ...) | errors but `power` (NaN) and `exp` (inf) |
| `-7 div 2`, `7 div 2.5`; `div`, `mod` past 2^31 | -3, 2; 32-bit wrapped quotient | -4, 2; exact |
| Bitwise ops on negatives and past 2^63 | signed 64-bit, `>>` logical | negatives become 0 |
| Shift counts | the low byte, signed: `c & 31` below 32, 0 from 64 | masked to 0..=63 |
| `max(nan, 1)`, `min(nan, 1)` | NaN, 1 | 1, NaN (not in the port) |
| `op=` with a type error (`x += "a"`, `argument0 -= 1` on a string), every operator and kind of name | old value kept, no error | OpenGMK: an error on built-ins and arguments; oracle (patch 0012) and port: kept |
| `random_set_seed(12345)` then `random(1)` x 10 | same sequence | same |
| Fresh highscore table, `highscore_add` | ten `<nobody>` at 0; as expected | same (out-of-range places differ) |
| `sin`, `arctan`, `arctan2`, `sqrt`, `point_distance` (120 inputs) | match | GM8 on Intel (the x87 model, 851-3257) |
| `cos`, `point_direction` (120 inputs) | Rosetta: 6 and 1 differ by 1 ulp. Intel: 1 and 1 | GM8 on Intel: all match |
| `tan`, `exp`, `power`, `logn` | often 1 ulp off (`tan` only under Rosetta) | (not in the port) |
| `string(x)` from 2^63 up (20 `literals` lines, up to 1e30) | Rosetta: 10 significant digits and an exponent (`"9.223372037E18"`). Intel: `"ERROR"` | every digit |
| `sin(pi)`, `cos(pi/2)`, `sin(1e6)`, `string(sin(1e22))` | Rosetta: the true values, `"1E22"`. Intel: fsin's 66-bit-pi values, `"ERROR"` | Intel's |
| `degtorad`, `point_direction`, motion, ~23,000 inputs (`probes/x87/`) | 1 ulp off on 10-35% of them, on Intel and Rosetta alike | Intel's, every line (851-3257; see "Real x87") |
| Degrees to radians for `image_angle` (boxes, masks), `gravity_direction`, `view_angle` (`probes/x87/angle-*.gml`) | the runner's `angle * Pi / 180`, rounded per step, as for `direction`; gravity is `motion_add`'s routine with 80-bit products | `f64::to_radians`; Intel's since 851-3262 (see "Angles"), but for GM8's bounding-box and view formulas |

## Angles

851-3262 measured how the runner turns an angle in degrees into radians where no GML function is involved:
rotated bounding boxes and masks (`image_angle`), gravity (`gravity_direction`) and rotated views
(`view_angle`). Its code (`Spelunky.exe`, read only to describe it) converts each one as it does `direction`:
`angle * Pi / 180` with Delphi's 80-bit `Pi`, each step rounded to 53 bits. The bounding box (0x4aa958) and the
five mask tests (from 0x48a5f0) store `fsin` and `fcos` as doubles; gravity (`AddMotion`, 0x4abfbc, which the
step calls with `gravity_direction` and `gravity` and `motion_add` calls too) and the mouse's view transform
(around 0x49ba4b) put the 80-bit results straight into their products. The four probes test those formulas
against the other candidate conversions, `f64::to_radians` (OpenGMK's) and `degtorad`'s `x * (Pi / 180)`:

- `angle-bbox`: `bbox_left`/`top`/`right`/`bottom` of `oScreen` with `sTunnelLeft`, `sMetalBlock` or `sArrowRight`
  as its mask, on every half degree, and at positions tuned so a side lands on a rounding boundary for one
  conversion and not another.
- `angle-mask`: `collision_point` and `place_meeting` against a rotated, scaled `sMetalBlock` at (100, 60), the
  other instance an `oMenu` (no events) with `sBlock`'s mask; scales tuned so a point lands on the mask's edge.
- `angle-motion-add`: `hspeed` and `vspeed` after `motion_add` on every half degree, and the speeds it starts from
  (which aren't quite the ones in the probe's source: GM8 reads the literal `0.75` as 0.7500000000000001, 851-3280).
- `angle-view`: `window_view_mouse_get_x`/`y(0)` through view 0 rotated by every half degree, around the
  0.01-degree threshold below which GM8 doesn't rotate, and at whole view positions and mouse positions where
  the conversions round differently (GM8 keeps views' positions and sizes whole, and only takes their new
  settings after a redraw, hence the probe's `screen_redraw()`).

They're generated (`angles/`, a standalone crate: `cargo run --release -- gen probes/x87`) and checked against
models of each routine (`cargo run --release -- check angle-bbox results/gm8-intel/x87/angle-bbox.txt`). Each
takes 1-3 minutes; they're too slow for `run-all`. Lines where each model differs:

| Probe (lines) | GM8's formula, the runner's conversion | ... `to_radians` | ... `degtorad` | The port before | The port now |
|---|---|---|---|---|---|
| `angle-bbox` (1,422) | Intel 0, Rosetta 14 | 597 | 579 | 517 | 0 (119 with OpenGMK's formula, until 851-3278) |
| `angle-mask` (1,986) | Intel 0, Rosetta 40 | 774 | 732 | 774 | 0 |
| `angle-motion-add` (1,440; gravity's routine) | Intel 0, Rosetta 204 | 298 | 283 | 438 | 0 |
| `angle-view` (778) | Intel 0, Rosetta 1 | 10 | 0 | 20 | 0 (11 with OpenGMK's transform, until 851-3279) |

The other columns are Intel's. Gravity itself is measured through `motion_add`: no probe can step a frame, and the
step's call to the routine (with `gravity_direction` and `gravity`) is read from the runner's code. The view lines
can't tell the runner's conversion from `degtorad`'s: with whole view positions only exact sines reach a rounding
boundary, and there the two agree; the code is the same as the masks'. Under Rosetta, `motion_add` fits sines and
cosines stored as doubles better (30 lines off) than the 80-bit ones (204): Rosetta's `fsin` and `fcos` aren't the
hardware's (see "Real x87").

The port and the oracle (patch 0017) now convert every one of these angles this way (`x87::gm8::sin_cos_degrees`
and `add_motion`), and compute the bounding box with GM8's formula, which bounds each side separately where
OpenGMK rotated the four corners (the port's `Instance::bbox_at` and patch 0018, 851-3278), and the mouse's view
transform with GM8's, with its 0.01-degree threshold, 80-bit products and order of additions
(`x87::gm8::view_to_room`, the port's `View::to_room` and patch 0019, 851-3279). The patched oracle answers
`angle-bbox`, `angle-mask` and `angle-motion-add` exactly as GM8 on Intel does (`results/opengmk/x87/`; OpenGMK has
no `window_mouse_set`, so no `angle-view`, but its `transform_point` is the port's function).
`crates/gm/tests/gm8_probe.rs` checks the port through its real collision, motion and view code against every Intel
line.

## Emulation

The runner is x86 code, and on an Apple Silicon Mac it runs under Rosetta. Delphi (and so GM8) does its
arithmetic on the x87 FPU, so whether the emulator models it matters. `x87/x87.c` checks: built with
`gcc -O0 -static` and run in an amd64 container (Rosetta) and under `qemu-x86_64` 7.2 (in an arm64 container):

| | Rosetta (OrbStack) | QEMU 7.2 user mode |
|---|---|---|
| Precision control (24, 53, 64 bits) | honored: the double-rounding sum and product give the expected bits in each mode | honored, the same |
| `fsin(double(pi))` | 1.2246063538223773e-16, Intel's (its 66-bit pi) | 1.2246467991473532e-16, the true sine |
| `fsin(1e22)` | operand returned unchanged (out of range), like Intel | the same |
| `fcos(1e6)` | `3fedf9df9906d330` | `3fedf9df9906d32c` |

On real hardware (an Intel Core i7-9750H, natively under macOS) `x87.c` prints the same as Rosetta. But that's not enough: inside GM8, Rosetta gets other transcendentals than the hardware (next section).
Basic arithmetic (`+ - * /`, `sqrt`) is fully specified by IEEE 754 and the precision mode, and both emulators
agree on it with the hardware.

## Real x87

851-3247 ran every probe on an Intel Core i7-9750H Mac: Colima's x86_64 VM (hardware virtualization; the
container sees the i7-9750H in `/proc/cpuinfo`, `uname -m` is `x86_64`, and the VM's `qemu-i386` binfmt handler
was turned off, so nothing is emulated). `probes/x87/` adds about 23,000 inputs: every half degree through
`degtorad`, `sin`, `cos` and the motion variables (`direction`/`speed` to `hspeed`/`vspeed` and back), `sin`,
`cos`, `tan` of k/100, k/64 and k*k/3 radians (up to 30,000), `point_direction` and `arctan2` over a whole and a
fractional grid, and `arctan`, `sqrt`, `ln`, `exp`, `power` and `point_distance` of k/100. They're too slow for
`run-all` (30-50 s each) and are run by name. Results: `results/gm8-intel/` (and `x87/`), with the Rosetta runs of
the same probes in `results/gm8-wine/x87/`. `x87/raw.c` gets the bare `fsin`, `fcos`, `fptan` and `fpatan` for
the same inputs, natively; GM8's `sin`, `cos` and `tan` are those instructions' results, bit for bit.

851-3248 ran `literals` on the same machine the same way: every literal reads to the same bits as under Rosetta,
subnormals included; only `string()` of the 20 values from 2^63 up differs (`"ERROR"`, see the table above).

`uv run --with mpmath python3 -I x87/ulps.py results/gm8-intel/x87 results/gm8-wine/x87` counts, per
function, the lines where GM8 on Intel differs from the port, from a model of the x87 (below), and from
Rosetta:

| Function (inputs) | Lines | Differ from the port | From the model | From Rosetta |
|---|---|---|---|---|
| `degtorad` (half degrees) | 720 | 76 (10.6%), all +1 ulp | 0 | 0 |
| `sin`, `cos` of those | 720 each | 1 and 2: `sin(180)`, `cos(90)`, `cos(270)` (66-bit pi) | 0 | 25, 28 |
| motion `hspeed`, `vspeed` | 720 each | 251, 252 (35%), mostly 1-3 ulps, near zero up to 384 | 0 | 120, 118 |
| motion `direction`; `speed` | 625 each | 112 (18%), 1 ulp; 0 | 0 | 0 |
| `sin`, `cos`, `tan` (k/100, k/64; to 10 rad) | 1640 each | 0, 1, 0 | 0 | 58, 52, 69 (3-4%) |
| `sin`, `cos`, `tan` (k*k/3; to 30,000 rad) | 300 each | 38, 58, 96 (13-32%), up to 44 ulps | 0 | 41, 60, 93 |
| `point_direction` (whole; fractional) | 2401; 800 | 539 (22%); 184 (23%), 1-2 ulps | 0 | 0 |
| `arctan2` (whole) | 2401 | 0 | 0 | 0 |
| `arctan`; `sqrt`, `ln`, `point_distance` | 1000 each | 1; 0 | 0 | 0 |
| `exp`; `power` (to 10; `power(v, 1.5)`) | 1000 each | 707 (71%), 595 (60%), up to 5 ulps | not modelled | 0, 1 |

So:

- **Rosetta isn't real hardware for transcendentals.** Real `fsin`, `fcos` and `fptan` are correctly rounded on
  every input below 10 radians but one and the multiples of pi/2 (where the 66-bit pi shows), where GM8 under Rosetta is 1 ulp off on 3-4%. PR #62's "`cos` is 1 ulp
  off on 6 of 120 inputs" was Rosetta's: on Intel it's 1 (and `sin(pi)` is fsin's, not the true sine).
  Arithmetic, `arctan2`, `ln`, `exp` and `power` agree with Rosetta to the bit (all but one `power` line).
- **Rosetta doesn't trap unmasked x87 exceptions.** GM8 runs with invalid operation, division by zero and
  overflow unmasked: on Intel, `sqrt(-1)`, `arcsin(2)`, `ln(0)`, `power(-8, 1/3)` and `exp(1000)` are GML errors
  ("Error in function ln()."), and `floor(1e20)` (an `fistp` overflow) stops the game with "Unexpected error
  occured when running the game.". Under Rosetta they quietly give NaN, -inf, inf and 0, and the probes after
  them (`nonfinite`, the second half of `numbers`) can't be re-measured on Intel as written. Mesa's llvmpipe
  trips the same trap at startup, hence softpipe.
- **Most of what differs is arithmetic, not transcendentals**, and Rosetta has it right: GM8 keeps 80-bit
  values and constants in registers where the port rounds to doubles. A model explains every line but `exp`
  and `power` (`x87/ulps.py`):
  - `fsin`, `fcos`, `fptan`: reduce the argument by Intel's 66-bit pi (`0xC90FDAA22168C234C / 2^66`) exactly,
    take the exact function of the reduced argument and round it to 64 bits, then to 53 on the store (it also
    matches all 7,260 `x87/raw.c` results); `fpatan` and `fyl2x` (`ln`): the exact value rounded to 64 bits;
  - every `fmul`/`fdiv` rounds to 53 bits (the precision control), but operands that never left the FPU keep
    64: `degtorad(x)` is `x * (Pi/180)` with `Pi/180` an 80-bit constant; `point_direction` and the runner's
    `direction` are `fpatan(dy, dx) * 180 / Pi`, each step rounded to 53 bits, the 80-bit atan2 and `fldpi`'s
    pi going in, then `+ 360` if negative; `hspeed`/`vspeed` are `speed * cos(direction * Pi / 180)` and
    `-speed * sin(...)`, the argument rounded to 53 bits per step and the 80-bit cos or sin going into the
    product (then OpenGMK's snap to a whole number within 0.0001, which GM8 does too).

851-3257 adopted the model: the port (`gm::math::x87`) and the oracle (patch 0015) now compute every line above
bit for bit, `exp` and `power` included (Delphi's `Exp` rounds `x * log2(e)` to 53 bits before `f2xm1`, and
`Power(x, y)` is `Exp(y * Ln(x))`), which `crates/gm/src/math/intel.rs` checks; the "differ from the port" column
is what the port computed before. `probes/x87/extra.gml` settles what these left open, run under Rosetta (its
arithmetic is the hardware's; `results/gm8-wine/x87/extra.txt`): `radtodeg` is `x * (180 / Pi)` with an 80-bit
constant on all 2,721 inputs, `lengthdir_x` and `lengthdir_y` equal the runner's `hspeed` and `vspeed` bit for
bit (snap included), and `point_direction` between equal points is +0. `x87/vectors.py` makes reference values
for each x87 instruction (mpmath) that the vendored pxfm's `x87` tests check.

851-3259 ran every probe on the same machine the same way (all but `x87/`; the ones 851-3247 and 851-3248 ran
came out byte for byte the same), with 35 new ones for what the x87's traps reach beyond the math functions:

- **Overflow traps too.** `+`, `-`, `*`, `/`, a comparison's difference, `sqr` and `radtodeg` past the largest
  double stop the game with "Unexpected error occured when running the game." (`add-overflow`, `cmp-overflow`,
  `div-tiny`, ...), as does a 1e309 literal while the game loads. Underflow doesn't (`underflow`). Under
  Rosetta every one of them is ±inf.
- **Every integer goes through Delphi's `Round` into an `Int64`**: `chr`, `string_char_at`, `make_color_rgb`
  and `string_format` arguments, `a[x]`, `alarm[0] = x` and `repeat (x)` from 2^63 up are the same error
  (`chr-2p63`, `array-2p63`, ...), where Rosetta gets -2^63 (`int-args-nonfinite` has its answers for an
  infinity and a NaN). In range they're cut to 32 bits (`int-args`), but for `make_color_rgb`, which keeps 64.
- **An `op=` ignores type errors, not traps**: `a |= 2^63` stops the game on a local, a field, a built-in and
  an argument, and `a *= 2` past the largest double on a local and a field (`orassign-*`,
  `mulassign-overflow-*`).
- `real()` of `"1e400"` is "Error in function real()." on Intel, as `"inf"` and `"nan"` are everywhere;
  `arccos(2)`, `log2(0)`, `log10(0)`, `logn(2, 0)` and `power(0, -1)` are "Error in function ...()."
- `string_format` (`string-format`, `string-format-neg`, `string-format-big`) and `int-args` answer the same as
  under Rosetta, line for line.

So GML on Intel can't make an infinity or a NaN at all. The port and the oracle (patch 0014) follow Intel on
each of these; `docs/quirks.md` §3 has the rules and §7 what still differs.
