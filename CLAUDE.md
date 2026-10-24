# Working on the Spelunky port

An exact Rust replica of Spelunky Classic 1.1. "Exact" means the same state, tick for tick, as the
original running under the reference oracle, **bugs included**. Never fix original behavior unless it's
behind an off-by-default option, and record each preserved quirk in `docs/quirks.md`.

## Workflow

- Work is tracked in Linear: team **851**, project **spelunky**. Tickets are focused changes; split a ticket when its
  PR would take more than a few minutes to review.
- Never commit straight to `main`. For each ticket:
  1. Move it to In Progress.
  2. Work in a worktree, on a branch off `origin/main` named with the ticket's Linear branch name
     (`alexandru/851-xxxx-...`).
  3. Before opening the PR, rebase onto `origin/main` and run the code-review skill on the change, fixing what it
     finds. The PR's required `pr-ok` check (CI on the Mac minis, `docs/ci.md`) must pass before it merges.
  4. Open one PR per ticket, titled `851-xxxx: <ticket title>`. The body has a `## Why` (a few sentences) and a
     `## What` (what changed and the key decisions), and ends with the Claude Code footer.
  5. Move the ticket to In Review.
  6. After review, squash-merge (`gh pr merge --squash --delete-branch`), move the ticket to Done, and remove the
     worktree.
- At most 3 agents work at once.
- New work found along the way becomes a new ticket, not scope creep in the current PR.

## Code

- Use `scripts/cargo` in place of `cargo` (rustup's toolchain, with the wasm32 target). `scripts/check` is
  the full suite: CI's `pr-ok` runs it on every pull request, and running it locally while iterating saves CI
  round trips. `scripts/check --fast` is the instant pre-commit subset (formatting).
- `scripts/check` also runs `gm`'s unit tests on wasm32 under Node (`scripts/wasm-test`). wasm can't unwind, so a
  panic aborts the whole run: mark `#[should_panic]` tests `#[cfg_attr(target_family = "wasm", ignore)]`.
- Every committed replay (`fidelity/replays/`, `fidelity/settings/`) has a fingerprint of the oracle's run in
  `fidelity/fingerprints/`, which `scripts/check` checks the port against (`fidelity-check`). Adding or changing a
  replay means recording its fingerprint with `scripts/fidelity --refresh-fingerprints <replay>`, which needs the
  oracle and refuses while the port disagrees with it. After a merge that changes the oracle's output, re-record
  them all with `scripts/fidelity --refresh-fingerprints` (docs/formats.md, "Fingerprints").
- Game state never comes from the platform's float functions (`f64::sin`, `powi`, ...): they differ between macOS,
  wasm32 and the oracle. Use `gm::math`, which is deterministic on every platform: correctly rounded where GM8 is,
  and GM8's own x87 results where it isn't (the `x87` model). The workspace `clippy.toml` enforces it.
- The original source is `reference/spelunky_1_1_src.zip` (`spelunky.gmk`). `docs/source-inventory.md`
  describes every object, script, room, and GameMaker built-in the game uses. When unsure what the original
  does, read its GML; don't guess.
- GameMaker *runner* behavior (event order, collision rules, RNG, rounding) isn't in the source. Confirm it
  against OpenGMK's source (https://github.com/OpenGMK/OpenGMK) or the oracle. OpenGMK is GPLv2: read it,
  but never copy its code into this repo. The one exception is `tools/oracle/patches/`, which holds GPLv2
  patch files for the separate oracle tool: they're aggregated alongside the repo, not linked into the port.
  Nowhere else may contain OpenGMK code.
- `gm` and `game` stay headless and deterministic: no globals, no wall-clock time, no hidden randomness, no
  graphics or audio dependencies. The world is a plain value that's cheap to `Clone` (savestates).
- Match the surrounding code's style. Comments explain *why*, especially where the code mirrors a GM8 quirk.
