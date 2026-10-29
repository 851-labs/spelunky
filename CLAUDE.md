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
     finds. The PR's required `pr-ok` check (CI on the Mac minis, `docs/ci.md`) runs the full suite and must
     pass before it merges.
  4. Open one PR per ticket, titled `851-xxxx: <ticket title>`. The body has a `## Why` (a few sentences) and a
     `## What` (what changed and the key decisions), and ends with the Claude Code footer.
  5. Move the ticket to In Review.
  6. After review, squash-merge (`gh pr merge --squash --delete-branch`), move the ticket to Done, and remove the
     worktree.
- At most 3 agents work at once.
- New work found along the way becomes a new ticket, not scope creep in the current PR.
- A fidelity ticket (the port, the oracle or GM8 disagreeing) says whether Spelunky reaches the behaviour, with the
  evidence: the object, script or replay that does. Reached ones are priority High and come first. The rest (only
  values the game never produces, built-ins it never calls, the settings program's or level editor's corners) are
  priority Low, wait until nothing else is, and are batched. Matching GM8 everywhere is still the goal; this is
  the order.
- A change that alters play checks, in the same PR, that the replays it changes still show what their headers
  say, and re-makes the ones that don't (or files them as tickets).

## Code

- Use `scripts/cargo` in place of `cargo` (rustup's toolchain, with the wasm32 target). `scripts/check` is
  the full suite, and CI's `pr-ok` runs it on every pull request in a few minutes, faster than a local run (the
  minis' caches are warm; locally, agents share the cores). So leave it to CI: locally, run `scripts/check --fast`
  (the instant pre-commit subset: formatting) and the tests the change touches, then open the PR and push fixes
  if `pr-ok` fails.
- `scripts/check` also runs `gm`'s unit tests on wasm32 under Node (`scripts/wasm-test`). wasm can't unwind, so a
  panic aborts the whole run: mark `#[should_panic]` tests `#[cfg_attr(target_family = "wasm", ignore)]`.
- Every committed replay (`fidelity/replays/`, `fidelity/settings/`) has a fingerprint of the oracle's run in
  `fidelity/fingerprints/`, which `scripts/check` checks the port against (`fidelity-check`). Adding or changing a
  replay means recording its fingerprint with `scripts/fidelity --refresh-fingerprints <replay>`, which needs the
  oracle and refuses while the port disagrees with it. After a merge that changes the oracle's output, re-record
  them all with `scripts/fidelity --refresh-fingerprints` (docs/formats.md, "Fingerprints").
- The wiki's media are committed in `wiki/media`. A change to a clip spec, a replay a clip is cut from, or what
  renders them (the game, drawing, sound, `harness`, the extractor) runs `scripts/wiki-media` and commits what it
  changes, in one commit near the end of the PR (each re-commit of a video stays in history); `scripts/check`
  and CI's media check fail without it (docs/wiki-media.md, "Committing").
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
- The site's (`site/`) UI primitives are built on [Base UI](https://base-ui.com) (`@base-ui/react`): popovers,
  preview and hover cards, tooltips, comboboxes and autocompletes, menus, dialogs, toggles and toggle groups,
  tabs, accordions and the like, styled to the site's look. Don't hand-roll an ARIA widget Base UI provides.
- Match the surrounding code's style. Comments explain *why*, especially where the code mirrors a GM8 quirk.
