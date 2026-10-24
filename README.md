# spelunky

An unofficial Rust port of **Spelunky Classic 1.1** by Derek Yu (2009). It's a fan-made mod, not an
official or unmodified version of Spelunky.

> Spelunky is Copyright (c) 2008, 2009 Derek Yu and Mossmouth, LLC. <https://spelunkyworld.com>
>
> This port is a modified version of the Spelunky source and is distributed under the
> [Spelunky User License v1.1b](LICENSE). It may not be sold.

The goal is an exact replica: the same behavior tick for tick, bugs included, verified against the original
game. It runs natively (Metal on macOS) and in the browser (WebGPU).

Play: https://spelunky.ai (live once Cloudflare's Workers Builds is connected to the repo and a release's web
build is bumped in: see [docs/web-deploy.md](docs/web-deploy.md)).

## Setup

Requires rustup (the toolchain is pinned in `rust-toolchain.toml`, including the wasm32 targets),
[lefthook](https://github.com/evilmartians/lefthook), and Node (`scripts/check` runs `gm`'s tests on wasm32 with it).

```sh
lefthook install      # git hooks: fast checks on commit, the full suite on push
scripts/check         # the full suite by hand: fmt, clippy, tests (native and wasm32), wasm32 build
scripts/extract       # regenerate assets/ (gitignored) from the reference zips
scripts/transpile     # regenerate crates/game/src/generated/ (committed) from the reference .gmk
scripts/web           # build the browser version and serve it at http://localhost:8000 (--check: build only)
```

## Playing

```sh
scripts/extract                                    # once: the game's assets
scripts/cargo run --release -p app                 # play: arrows to move, Z jump, X whip, Enter skips the intro
scripts/cargo run --release -p app -- --record out.txt --trace out.jsonl   # record a session (and its trace)
scripts/cargo run --release -p app -- --replay out.txt                     # watch it again
scripts/cargo run --release -p app -- --preview --room rTutorial           # a room's placed state, for debugging
```

Saves (`stats.txt`, the `.cfg` files, highscores) go in the per-user data directory (`~/Library/Application
Support/Spelunky Classic` on macOS) and in `localStorage` in the browser. A recording starts from the shipped
files instead, so `replay-run --faithful-audio out.txt` replays it headless to the same trace. See
`crates/app/src/main.rs` for every option.

Use `scripts/cargo` in place of `cargo` if a Homebrew `rust` install shadows rustup on your PATH.

## In the browser

`scripts/web` builds `app` for `wasm32-unknown-unknown`, runs the `wasm-bindgen` CLI (installed into `target/tools`
at Cargo.lock's version on first use) and serves the page with the assets next to it. The page's URL query takes
the native options: `?replay=out.txt&mute&no-gamepad&physical-keys`, or `?preview&room=rTutorial&view=0,64&scale=3`.
Sound starts at the first key press (browsers need a user gesture); each music track is decoded whole when it
starts, which stalls that frame. Recording isn't wired up on the web yet.

It draws with WebGPU where the browser has a working adapter and falls back to WebGL2 elsewhere; both render the
same pixels as native.

| Browser | Graphics |
| --- | --- |
| Chrome, Edge 113+ (Windows, macOS, ChromeOS); Chrome 121+ (Android 12+) | WebGPU |
| Safari 26+ (macOS, iOS, iPadOS) | WebGPU |
| Firefox 141+ (Windows), 145+ (macOS on Apple silicon) | WebGPU |
| Chrome and Firefox on Linux, Firefox on Android, older Safari (15+) | WebGL2 |

`scripts/web --package OUT.tar.gz` makes the release build for the site, which `site/` deploys to spelunky.ai
([docs/web-deploy.md](docs/web-deploy.md)).

## Layout

- `crates/gmk`: reader for GameMaker 8 `.gmk` files.
- `crates/assets`: the format of the extracted asset bundle in `assets/` (written by `tools`, loaded by the game).
- `crates/gml`: lexer and parser for GML.
- `crates/gm`: the GameMaker 8 runtime subset the game uses (values, RNG, instances, events, collision, rooms).
- `crates/trace`: replay and trace formats shared with the reference oracle.
- `crates/transpile`: the GML-to-Rust transpiler that generates the game's code.
- `crates/game`: Spelunky's logic, transpiled from the original GML: headless, deterministic, cloneable.
- `crates/app`: native and web frontend.
- `crates/tools`: extractor, transpiler driver, replay runner, trace diff.
- `reference/`: the original 1.1 game and source, unmodified (see `reference/README.md`).
- `docs/`: design notes and the [source inventory](docs/source-inventory.md).
- `site/`: spelunky.ai, the landing page and the web build's deploy ([docs/web-deploy.md](docs/web-deploy.md)).
- `tools/`: import and analysis tooling.

## License

The game port is a Covered Work under the Spelunky User License v1.1b (`LICENSE`). Future AI, harness and
training work will live in separate directories under its own license (MIT); the game's license doesn't
cover that work.
