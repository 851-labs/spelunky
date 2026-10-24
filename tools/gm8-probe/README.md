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

A probe whose game hasn't ended after `GM8_PROBE_TIMEOUT` seconds (default 60) is stopped; under Wine, a
screenshot of the screen goes to `OUTDIR/screen.png`, which is how a GM8 error dialog shows up. `out` reopens
`probe.txt` for every line, so the lines before an error are kept. The probes in `probes/errors/` each test one
thing that might be an error.

`results/` holds the outputs of the last runs (`gm8-wine/` under Rosetta, `gm8-intel/` on a real Intel CPU,
`opengmk/`), with a `# stopped:` line where an error
ended a probe (OpenGMK's message, or the text of GM8's dialog from its screenshot). `python3 -I compare.py`
prints the lines that differ. `crates/gm/tests/gm8_probe.rs` checks that the port computes OpenGMK's answers
for the lines it can; it does, on all of them.

## What GM8 answered

`docs/quirks.md` records each answer where it matters (status "GM8", and §7 for the differences). In short:

The GM8 column is GM8 under Rosetta (`results/gm8-wine/`); where GM8 on a real Intel CPU (`results/gm8-intel/`)
answers otherwise, the row says so ("Real x87" below).

| Question | GM8 (Wine) | OpenGMK = port |
|---|---|---|
| FPU precision for GML arithmetic | 53 bits (plain doubles) | f64 |
| `if (0.5)`, `if (0.5 - 2^-54)` | true, false | same |
| Comparison tolerance | 1e-13 (9e-14 equal, 1.1e-13 not) | same |
| Comparisons with NaN, `inf == inf` | `!=`, `>`, `>=` true; `inf == inf` false (Rosetta only: on Intel, `exp(1000)` is an error, so the probe makes no infinity) | all false |
| `round` ties, `round(-0.5)`, past 2^31 | half to even, +0, no wrap | same |
| `floor`/`ceil` past 2^31; `ceil(-0.5)` | wrap to 32 bits; +0. On Intel, `floor(1e20)` stops the game ("Unexpected error occured when running the game."), so the `numbers` lines after it weren't measured there | exact; -0 |
| `string(0.125)`, `string(-0)`, `string(-0.004)` | `"0.13"`, `"0"`, `"0.00"` | `"0.12"`, `"-0"`, `"-0.00"` |
| `string` of big fractions, inf, NaN | `"1.099511628E12"`, `"INF"`, `"-9223372036854775808"` | every digit, `"inf"`, `"NaN"` |
| Real literals (`0.3`) | `0x3fd3333333333334`, not correctly rounded | same |
| `real("1e3")`, `real(" 12 ")`, `real("")`; `real("12abc")` | 1000, 12, 0; error | same |
| `x / 0`, `x div 0`, `x mod 0` | errors | inf/NaN |
| `sqrt(-1)`, `arcsin(2)`, `ln(0)`, `power(-8, 1/3)`, `exp(1000)` | Rosetta: NaN, NaN, -inf, NaN, inf. **Intel: errors** ("Cannot apply sqrt to negative number.", "Error in function arcsin().", ...) | errors but `power` (NaN) and `exp` (inf) |
| `-7 div 2`; `div`, `mod` past 2^31 | -3; 32-bit wrapped quotient | -4; exact |
| Bitwise ops on negatives and past 2^63 | signed 64-bit, `>>` logical | negatives become 0 |
| `random_set_seed(12345)` then `random(1)` x 10 | same sequence | same |
| Fresh highscore table, `highscore_add` | ten `<nobody>` at 0; as expected | same (out-of-range places differ) |
| `sin`, `arctan`, `arctan2`, `sqrt`, `point_distance` (120 inputs) | match | correctly rounded |
| `cos`, `point_direction` (120 inputs) | Rosetta: 6 and 1 differ by 1 ulp. Intel: 1 and 1 | correctly rounded |
| `tan`, `exp`, `power`, `logn` | often 1 ulp off (`tan` only under Rosetta) | (not in the port) |
| `sin(pi)`, `cos(pi/2)`, `sin(1e6)`, `string(sin(1e22))` | Rosetta: the true values, `"1E22"`. Intel: fsin's 66-bit-pi values, `"ERROR"` | correctly rounded |
| `degtorad`, `point_direction`, motion, ~23,000 inputs (`probes/x87/`) | 1 ulp off on 10-35% of them, on Intel and Rosetta alike | see "Real x87" |

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

`crates/gm/tests/gm8_probe.rs` (`gm8_on_intel_differs_from_the_port_this_often`) pins the "differ from the port"
column. The port's math is unchanged; whether to adopt the model is up to a follow-up ticket.
