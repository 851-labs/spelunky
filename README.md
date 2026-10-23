# spelunky

An unofficial Rust port of **Spelunky Classic 1.1** by Derek Yu (2009). It's a fan-made mod, not an
official or unmodified version of Spelunky.

> Spelunky is Copyright (c) 2008, 2009 Derek Yu and Mossmouth, LLC. <https://spelunkyworld.com>
>
> This port is a modified version of the Spelunky source and is distributed under the
> [Spelunky User License v1.1b](LICENSE). It may not be sold.

The goal is an exact replica: the same behavior tick for tick, bugs included, verified against the original
game. It runs natively (Metal on macOS) and in the browser (WebGPU).

## Layout

- `reference/`: the original 1.1 game and source, unmodified (see `reference/README.md`).
- `docs/`: design notes and the [source inventory](docs/source-inventory.md).
- `tools/`: import and analysis tooling.

## License

The game port is a Covered Work under the Spelunky User License v1.1b (`LICENSE`). Future AI, harness and
training work will live in separate directories under its own license (MIT); the game's license doesn't
cover that work.
