# Working on the Spelunky port

An exact Rust replica of Spelunky Classic 1.1. "Exact" means the same state, tick for tick, as the
original running under the reference oracle, **bugs included**. Never fix original behavior unless it's
behind an off-by-default option, and record each preserved quirk in `docs/quirks.md`.

- Work is tracked in Linear (project `spelunky`, tickets `851-xxxx`). Mention the ticket in commit messages.
- Use `scripts/cargo` in place of `cargo` (rustup's toolchain, with the wasm32 target). `scripts/check` is
  the full suite and must pass before work lands. `scripts/check --fast` is the pre-commit subset.
- The original source is `reference/spelunky_1_1_src.zip` (`spelunky.gmk`). `docs/source-inventory.md`
  describes every object, script, room, and GameMaker built-in the game uses. When unsure what the original
  does, read its GML; don't guess.
- GameMaker *runner* behavior (event order, collision rules, RNG, rounding) isn't in the source. Confirm it
  against OpenGMK's source (https://github.com/OpenGMK/OpenGMK) or the oracle. OpenGMK is GPLv2: read it,
  but never copy its code into this repo.
- `gm` and `game` stay headless and deterministic: no globals, no wall-clock time, no hidden randomness, no
  graphics or audio dependencies. The world is a plain value that's cheap to `Clone` (savestates).
- Match the surrounding code's style. Comments explain *why*, especially where the code mirrors a GM8 quirk.
