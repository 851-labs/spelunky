#!/usr/bin/env bash
# The Rust half of the mini-toolchain action. Installs rustup (a pinned rustup-init, checked against its
# SHA-256) and the toolchain rust-toolchain.toml pins into the runner's tool cache, not ~/.cargo: the minis
# are shared with other repositories' jobs. Then points the checkout's target/ at a cargo target dir kept in
# the same cache, so builds are incremental across jobs, wiping it first if it has outgrown its cap (the
# minis have about 30 GB free).
#
# Nothing here is locked: each runner installation has its own tool cache and runs one job at a time.
# Pull requests' code runs on the minis too (outside contributors' only once approved), and can change what's
# cached here, so release.yml builds the web version it publishes on a fresh GitHub-hosted machine instead.
set -euo pipefail
cd "${GITHUB_WORKSPACE:?}"
cache="${RUNNER_TOOL_CACHE:?}/spelunky"
rustup_version=1.29.2
rustup_triple=aarch64-apple-darwin
rustup_sha256=f7c7e11eb2e07f006e532a8aaac77b32f4527d305ac386c0c633ca6133d35ca9
# In GiB. A cold scripts/check plus scripts/web is about 6.
target_cap=${SPELUNKY_TARGET_CACHE_GB:-12}

[ "$(uname -sm)" = "Darwin arm64" ] || { echo "mini-toolchain: for Apple Silicon Macs, not $(uname -sm)" >&2; exit 1; }
# scripts/wasm-test skips itself when this is set; CI must run it (a runner's .env could set it).
if [ -n "${SPELUNKY_SKIP_WASM_TEST:-}" ]; then
  echo "mini-toolchain: SPELUNKY_SKIP_WASM_TEST is set, which would skip scripts/check's wasm32 tests" >&2
  exit 1
fi
export RUSTUP_HOME="$cache/rustup" CARGO_HOME="$cache/cargo"
if [ ! -x "$CARGO_HOME/bin/rustup" ]; then
  echo "mini-toolchain: installing rustup $rustup_version into $cache"
  tmp=$(mktemp -d)
  curl --proto '=https' --tlsv1.2 -fsSL --retry 3 -o "$tmp/rustup-init" \
    "https://static.rust-lang.org/rustup/archive/$rustup_version/$rustup_triple/rustup-init"
  echo "$rustup_sha256  $tmp/rustup-init" | shasum -a 256 -c - >/dev/null
  chmod +x "$tmp/rustup-init"
  "$tmp/rustup-init" -y --quiet --no-modify-path --profile minimal --default-toolchain none
  rm -rf "$tmp"
  # Stays the version pinned above.
  "$CARGO_HOME/bin/rustup" set auto-self-update disable
fi
rustup="$CARGO_HOME/bin/rustup"
# With no argument, installs what rust-toolchain.toml asks for: the channel, its components and targets.
"$rustup" toolchain install --no-self-update
pinned=$("$rustup" show active-toolchain | cut -d' ' -f1)
# Toolchains from earlier pins are a gigabyte each.
"$rustup" toolchain list | cut -d' ' -f1 | while read -r toolchain; do
  [ "$toolchain" = "$pinned" ] || "$rustup" toolchain uninstall "$toolchain"
done
{
  echo "RUSTUP_HOME=$RUSTUP_HOME"
  echo "CARGO_HOME=$CARGO_HOME"
  # Incremental artifacts double the target dir and don't help a build from a clean checkout's changes much.
  echo "CARGO_INCREMENTAL=0"
} >>"$GITHUB_ENV"
echo "$CARGO_HOME/bin" >>"$GITHUB_PATH"

mkdir -p "$cache/target"
size=$(du -sk "$cache/target" | cut -f1)
if [ "$size" -gt $((target_cap * 1024 * 1024)) ]; then
  echo "mini-toolchain: the target cache is $((size / 1024 / 1024)) GiB, over its $target_cap GiB cap; starting over"
  rm -rf "$cache/target"
  mkdir "$cache/target"
fi
# actions/checkout's clean removes the link (not what it points to) before the next job.
rm -rf target
ln -s "$cache/target" target
"$rustup" --version
"$CARGO_HOME/bin/cargo" --version
node --version
df -h "$cache" | tail -n 1
