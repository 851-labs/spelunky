# pxfm, vendored for the Spelunky port

[pxfm](https://github.com/awxkee/pxfm) 0.1.30 by Radzivon Bartoshyk (BSD-3-Clause OR Apache-2.0, see
`LICENSE.md` and `LICENSE-APACHE.md`), as published on crates.io, with these changes (851-3167):

- **Trimmed** to the double-precision `f_sin`, `f_cos`, `f_tan`, `f_atan` and `f_atan2` and what they use.
  The other functions' modules are deleted, and `src/lib.rs` and `src/tangent/mod.rs` are rewritten to
  match. `Cargo.toml` is rewritten (no `include`, docs.rs or profile settings).
- **No runtime CPU dispatch.** Upstream `f_atan` and `f_atan2` pick an AVX+FMA variant at run time on x86
  (`is_x86_feature_detected!` cached in a `static OnceLock`). Here they always call the generic
  implementation, and the AVX+FMA functions are deleted. Nothing else in these functions has run-time
  dispatch.

The functions are correctly rounded, so every code path gives the same bits; `tools/math-probe` checks
that across platforms and against upstream. `gm::math` is the only user, and the oracle's OpenGMK
should use this same copy.

To update: copy the new release over this directory, redo the two changes, run its tests (`cargo test`
here) and `tools/math-probe` on every platform in its README, and check the hashes still agree.
