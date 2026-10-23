# spelunky

An unofficial Rust port of **Spelunky Classic 1.1** by Derek Yu (2009). It's a fan-made mod, not an
official or unmodified version of Spelunky.

> Spelunky is Copyright (c) 2008, 2009 Derek Yu and Mossmouth, LLC. <https://spelunkyworld.com>
>
> This port is a modified version of the Spelunky source and is distributed under the
> [Spelunky User License v1.1b](LICENSE). It may not be sold.

The goal is an exact replica: the same behavior tick for tick, bugs included, verified against the original
game. It runs natively (Metal on macOS) and in the browser (WebGPU).

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

Use `scripts/cargo` in place of `cargo` if a Homebrew `rust` install shadows rustup on your PATH.

## In the browser

`scripts/web` builds `app` for `wasm32-unknown-unknown`, runs the `wasm-bindgen` CLI (installed into `target/tools`
at Cargo.lock's version on first use) and serves the page with the assets next to it. The page's URL query takes
the native options: `?room=rTutorial&view=0,64&scale=3&replay=pan.txt&no-gamepad&physical-keys`. Recording and
audio aren't wired up on the web yet (nor is audio natively).

It draws with WebGPU where the browser has a working adapter and falls back to WebGL2 elsewhere; both render the
same pixels as native.

| Browser | Graphics |
| --- | --- |
| Chrome, Edge 113+ (Windows, macOS, ChromeOS); Chrome 121+ (Android 12+) | WebGPU |
| Safari 26+ (macOS, iOS, iPadOS) | WebGPU |
| Firefox 141+ (Windows), 145+ (macOS on Apple silicon) | WebGPU |
| Chrome and Firefox on Linux, Firefox on Android, older Safari (15+) | WebGL2 |

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
- `tools/`: import and analysis tooling.

## License

The game port is a Covered Work under the Spelunky User License v1.1b (`LICENSE`). Future AI, harness and
training work will live in separate directories under its own license (MIT); the game's license doesn't
cover that work.
