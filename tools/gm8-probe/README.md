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
wined3d). On Apple Silicon, OrbStack runs the container under Rosetta. The image is built on first use. A run
takes 30-60 s, most of it Wine starting under emulation.

The unmodified `Spelunky.exe` boots the same way, with the real `supersound.dll`, and plays its intro
(screenshotted 70 s in); playing it would need input injected (`xdotool`), which nothing here does yet.

A probe whose game hasn't ended after `GM8_PROBE_TIMEOUT` seconds (default 60) is stopped; under Wine, a
screenshot of the screen goes to `OUTDIR/screen.png`, which is how a GM8 error dialog shows up. `out` reopens
`probe.txt` for every line, so the lines before an error are kept. The probes in `probes/errors/` each test one
thing that might be an error.

`results/` holds the outputs of the last runs (`gm8-wine/`, `opengmk/`), with a `# stopped:` line where an error
ended a probe (OpenGMK's message, or the text of GM8's dialog from its screenshot). `python3 -I compare.py`
prints the lines that differ. `crates/gm/tests/gm8_probe.rs` checks that the port computes OpenGMK's answers
for the lines it can; it does, on all of them.

## What GM8 answered

`docs/quirks.md` records each answer where it matters (status "GM8", and §7 for the differences). In short:

| Question | GM8 (Wine) | OpenGMK = port |
|---|---|---|
| FPU precision for GML arithmetic | 53 bits (plain doubles) | f64 |
| `if (0.5)`, `if (0.5 - 2^-54)` | true, false | same |
| Comparison tolerance | 1e-13 (9e-14 equal, 1.1e-13 not) | same |
| Comparisons with NaN, `inf == inf` | `!=`, `>`, `>=` true; `inf == inf` false | all false |
| `round` ties, `round(-0.5)`, past 2^31 | half to even, +0, no wrap | same |
| `floor`/`ceil` past 2^31; `ceil(-0.5)` | wrap to 32 bits; +0 | exact; -0 |
| `string(0.125)`, `string(-0)`, `string(-0.004)` | `"0.13"`, `"0"`, `"0.00"` | `"0.12"`, `"-0"`, `"-0.00"` |
| `string` of big fractions, inf, NaN | `"1.099511628E12"`, `"INF"`, `"-9223372036854775808"` | every digit, `"inf"`, `"NaN"` |
| Real literals (`0.3`) | `0x3fd3333333333334`, not correctly rounded | same |
| `real("1e3")`, `real(" 12 ")`, `real("")`; `real("12abc")` | 1000, 12, 0; error | same |
| `x / 0`, `x div 0`, `x mod 0` | errors | inf/NaN |
| `sqrt(-1)`, `arcsin(2)`, `ln(0)` | NaN, NaN, -inf | errors |
| `-7 div 2`; `div`, `mod` past 2^31 | -3; 32-bit wrapped quotient | -4; exact |
| Bitwise ops on negatives and past 2^63 | signed 64-bit, `>>` logical | negatives become 0 |
| `random_set_seed(12345)` then `random(1)` x 10 | same sequence | same |
| Fresh highscore table, `highscore_add` | ten `<nobody>` at 0; as expected | same (out-of-range places differ) |
| `sin`, `arctan`, `arctan2`, `sqrt`, `point_distance` (120 inputs) | match | correctly rounded |
| `cos`, `point_direction` (120 inputs) | 6 and 1 differ by 1 ulp | correctly rounded |
| `tan`, `exp`, `power`, `logn` | often 1 ulp off | (not in the port) |

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

So Rosetta models the x87 closely, including Intel's argument reduction, and QEMU computes `fsin`/`fcos`
through the host's libm. Rosetta is the better stand-in for real hardware, but neither is proof: results for
transcendentals are "GM8 under Rosetta", and should be re-checked on real x86 hardware before being relied on.
Basic arithmetic (`+ - * /`, `sqrt`) is fully specified by IEEE 754 and the precision mode, and both emulators
agree on it.
