# math-probe

Measures how the transcendental functions behind GML `sin`, `cos`, `arctan` and `point_direction` differ
between the platform's math library, the `libm` crate and `pxfm` (correctly rounded), on the inputs
Spelunky 1.1 can produce (851-3167). It informed the choice of `pxfm` in `crates/gm/src/math.rs`, and
checks that the port's vendored copy (`vendor/pxfm`) gives the same bits everywhere.

It compares four implementations: `platform` (`f64::sin` and friends), `libm` (the crate), `pxfm` (the
vendored copy the port uses) and `upstream` (pxfm 0.1.30 from crates.io, which on x86 picks an AVX+FMA
variant at run time). It's a standalone crate (not in the workspace) so it also builds in Linux images.

## Running

From this directory (Docker commands from the repo root, which they mount so the vendored crate is
visible):

```sh
# macOS (Apple libm)
../../scripts/cargo run --release

# wasm32 (std's sin/cos there are the libm crate's), under Node
../../scripts/cargo build --release --target wasm32-unknown-unknown
node run-wasm.mjs target/wasm32-unknown-unknown/release/math_probe.wasm

# glibc, as the oracle sees it (arm64, glibc 2.36). The image tag is in tools/oracle/common.sh.
docker run --rm -v "$PWD":/repo:ro -v /tmp:/out -e CARGO_TARGET_DIR=/tmp/t -e PROBE_DUMP=/out/dump.txt \
  -w /repo/tools/math-probe --entrypoint cargo spelunky-oracle:<tag> run --release --quiet

# x86_64 (emulated), without FMA, with FMA, and with AVX+FMA (which makes upstream take its AVX+FMA variant)
docker run --rm --platform linux/amd64 -v "$PWD":/repo:ro -v /tmp:/out -w /repo/tools/math-probe \
  rust:1.99.1-bookworm sh -c '
    CARGO_TARGET_DIR=/tmp/a cargo run --release --quiet > /out/x86.txt
    RUSTFLAGS="-C target-feature=+fma" CARGO_TARGET_DIR=/tmp/b cargo run --release --quiet > /out/x86-fma.txt
    RUSTFLAGS="-C target-feature=+avx,+fma" CARGO_TARGET_DIR=/tmp/c cargo run --release --quiet > /out/x86-avx.txt'

# Judge the dumped disagreements against mpmath at 200 bits
python -m venv /tmp/mp && /tmp/mp/bin/pip install mpmath && /tmp/mp/bin/python check_reference.py /tmp/dump.txt
```

Each input domain prints a hash of every implementation's results, so runs on different platforms can be
compared, and the number of 1-ulp disagreements between pairs.

## Results (2026-10, pxfm 0.1.30, libm 0.2.16)

Input domains: tenths of a degree in [-720, 1440]; `point_direction` on an integer grid (|d| <= 256) plus
the game's `+ {-1, 0, 1, 180}` offsets; 2M random fractional positions; `sin(xDiff)` over 2M steps of
`xDiff += 0.01` (oTarget); `arctan(yVel / xVel)` on velocity ratios; random radians in [-100, 100];
special values. About 22M evaluations per implementation.

- **The `libm`, `pxfm` and `upstream` hashes are identical in all six runs**: macOS aarch64, wasm32
  (Node), Linux aarch64, and Linux x86_64 without FMA, with `+fma`, and with `+avx,+fma` (where upstream
  takes its AVX+FMA variant). The vendored `pxfm` equals `upstream` everywhere. On wasm32, `f64::sin` and
  friends equal the `libm` crate.
- **glibc vs `libm` crate:** 1 ulp apart on about 3% of `sin`/`cos`, 5% of `atan` and 17% of `atan2`
  inputs. `libm` is the one that's wrong almost every time.
- **glibc (arm64) vs `pxfm`:** 1 ulp apart on about 0.13% of `sin`/`cos`, 0.1% of `atan2` and 0.2% of
  `atan` inputs. In every sampled disagreement (about 35k), `pxfm` was correctly rounded and glibc
  wasn't. x86_64 glibc differs from arm64 glibc too.
- **Apple libm** disagrees with `pxfm` on about 4% of `sin`/`cos` and 1% of `atan2` inputs.
- Special values: all agree, except that `pxfm` returns +0 instead of -0 for an underflowing negative
  `atan2` (`atan2(-2.2e-308, 1e22)`). `gm::math::atan2` corrects the sign.

So no portable library matches glibc bit for bit; a correctly rounded one gets closest, and is the only
choice that every platform (and any future implementation) can agree on.
