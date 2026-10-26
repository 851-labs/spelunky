# pxfm, vendored for the Spelunky port

[pxfm](https://github.com/awxkee/pxfm) 0.1.30 by Radzivon Bartoshyk (BSD-3-Clause OR Apache-2.0, see
`LICENSE.md` and `LICENSE-APACHE.md`), as published on crates.io, with these changes (851-3167, 851-3173,
851-3174):

- **Trimmed** to the double-precision `f_sin`, `f_cos`, `f_tan`, `f_atan` and `f_atan2` and what they use.
  The other functions' modules are deleted, and `src/lib.rs` and `src/tangent/mod.rs` are rewritten to
  match. `Cargo.toml` is rewritten (no `include`, docs.rs or profile settings).
- **No runtime CPU dispatch.** Upstream `f_atan` and `f_atan2` pick an AVX+FMA variant at run time on x86
  (`is_x86_feature_detected!` cached in a `static OnceLock`). Here they always call the generic
  implementation, and the AVX+FMA functions are deleted. Nothing else in these functions has run-time
  dispatch.
- **Signed zero in `f_atan2`.** Upstream returns +0 when a negative result underflows
  (`f_atan2(-2.2e-308, 1e300)`); IEEE and glibc give -0, and so does this copy.
- **Our own `f_hypot`** (`src/hypot.rs`, 851-3174). Upstream's `f_hypot` isn't correctly rounded: it's 1 ulp
  off on 16-21% of the fractional arguments the game's `distance_to_point` produces. So it's replaced by
  one we wrote (not derived from upstream's), which computes the rounding exactly with integer arithmetic
  and so is correctly rounded; see its doc comment. It's distributed under the same licence as the rest of
  the crate, so the oracle links it like the trigonometry.
- **Our own `x87` module** (`src/x87/`, 851-3257): a software model of the x87 FPU's 80-bit registers and
  instructions, and GameMaker 8's math built-ins on top (`x87::gm8`), which real GM8 on Intel matches bit for bit.
  Written for the port, not derived from anything, and under the same licence as the rest of the crate so the
  oracle links it too (patch `0015-x87-math`; `sin_cos_degrees` and `add_motion`, the runner's own angle conversion
  and gravity, 851-3262, patch `0017-x87-angles`; `view_to_room`, the runner's view transform, 851-3279,
  patch `0019-gm8-view-transform`). Integer arithmetic only. Its tests check the instructions against
  mpmath (`tools/gm8-probe/x87/vectors.py`).

The upstream functions are correctly rounded, so every code path gives the same bits; `tools/math-probe`
checks that across platforms and against upstream, and checks every `hypot` result against mpmath. Its users are
`gm::math` and the oracle, whose `tools/oracle/build` copies this directory into OpenGMK (patches
`0007-correctly-rounded-math`, `0008-correctly-rounded-hypot`, `0015-x87-math`, `0017-x87-angles` and
`0019-gm8-view-transform`), so the port and the oracle compute the same bits. Since 851-3257 they use `f_hypot` and the `x87` module only: GM8's `sin`, `cos`, `tan`,
`arctan` and `arctan2` aren't correctly rounded, and `f_sin`, `f_cos`, `f_tan`, `f_atan` and `f_atan2` are left for
`tools/math-probe` (and could be trimmed). Any change here rebuilds the oracle.

To update: copy the new release over this directory, redo the changes, bump the `+spelunky.N` version (the
oracle's build copies it into OpenGMK's `Cargo.lock`), run its tests (`cargo test` here) and `tools/math-probe`
on every platform in its README, and check the hashes still agree.
