# shellcheck shell=bash disable=SC2034
# Shared settings for tools/oracle/build and tools/oracle/run. Sourced, not executed.

# Pinned upstream sources. Changing either (or any patch, or the Dockerfile) makes `build` re-prepare the tree.
OPENGMK_URL="https://github.com/OpenGMK/OpenGMK"
OPENGMK_COMMIT="9e4282e6d19f4dd00961811840ddc3e11da6dd5a"
RAMEN_URL="https://github.com/viriuwu/ramen"
RAMEN_COMMIT="ef715f65981f6c96f7f889258ff43b63cb472626" # the rev OpenGMK's Cargo.lock pins

ORACLE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$ORACLE_DIR/../.." && pwd)"
GAME_ZIP="$REPO_ROOT/reference/spelunky_1_1.zip"

CACHE_DIR="${SPELUNKY_ORACLE_CACHE:-${XDG_CACHE_HOME:-$HOME/.cache}/spelunky-oracle}"
SRC_DIR="$CACHE_DIR/opengmk"
RAMEN_DIR="$CACHE_DIR/ramen"
BUILT_STAMP="$CACHE_DIR/built.stamp"

IMAGE="spelunky-oracle"
# Named volumes keep build state between runs: cargo's registry and git checkouts, and the target dir that holds
# the built gm8emulator.
CARGO_VOLUME="spelunky-oracle-cargo"
TARGET_VOLUME="spelunky-oracle-target"

die() {
  echo "oracle: $*" >&2
  exit 1
}

sha256() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum | cut -d' ' -f1
  else
    shasum -a 256 | cut -d' ' -f1
  fi
}

# Identifies a build: the pinned commits, the patches and the Dockerfile.
build_stamp() {
  {
    echo "$OPENGMK_COMMIT $RAMEN_COMMIT"
    cat "$ORACLE_DIR/Dockerfile" "$ORACLE_DIR"/patches/opengmk/*.patch "$ORACLE_DIR"/patches/ramen/*.patch
  } | sha256
}

require_docker() {
  if ! command -v docker >/dev/null 2>&1; then
    die "docker not found. The oracle runs in a Linux container. Install OrbStack (macOS: brew install orbstack,
        https://orbstack.dev) or Docker Desktop / Docker Engine, then re-run."
  fi
  if ! docker info >/dev/null 2>&1; then
    die "docker is installed but its daemon isn't reachable. Start OrbStack (or Docker Desktop) and re-run."
  fi
}
