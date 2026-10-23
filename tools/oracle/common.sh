# shellcheck shell=bash disable=SC2034
# Shared settings for tools/oracle/build and tools/oracle/run. Sourced, not executed.

# Pinned upstream sources. Changing either (or any patch, vendor/pxfm, or the Dockerfile) makes `build` re-prepare
# the tree.
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

# Named volumes keep build state between runs: cargo's registry and git checkouts, and the target dir. Each build
# installs its gm8emulator under /target/oracle/<build stamp>/, so a run only ever uses a binary built from the
# patches in its own checkout, even while another worktree builds different ones.
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

# The vendored math crate the port uses (see its README.spelunky.md). The oracle builds it in too.
PXFM_DIR="$REPO_ROOT/vendor/pxfm"

# The crate's build inputs under directory $1 (default PXFM_DIR), as the port's build sees them in the working tree:
# Cargo.toml and every .rs file under src/. Relative paths, NUL-terminated and sorted.
pxfm_files() {
  (cd "${1:-$PXFM_DIR}" && find Cargo.toml src -type f \( -name Cargo.toml -o -name '*.rs' \) -print0 | LC_ALL=C sort -z)
}

# Copies those files from the working tree into the empty directory $1.
pxfm_snapshot() {
  (cd "$PXFM_DIR" && pxfm_files | tar --null -T - -cf -) | tar -xf - -C "$1"
}

# pxfm's version, from the Cargo.toml in directory $1.
pxfm_version() {
  sed -n 's/^version = "\(.*\)"$/\1/p' "$1/Cargo.toml" | head -n 1
}

# Identifies a build: the pinned commits, the patches, the Dockerfile, and the vendored math crate's files in
# directory $1 (default PXFM_DIR; `build` passes its snapshot).
build_stamp() {
  local dir="${1:-$PXFM_DIR}" file
  [[ -f "$dir/Cargo.toml" && -d "$dir/src" ]] || die "the vendored math crate isn't at $dir (is this a full checkout?)"
  {
    echo "$OPENGMK_COMMIT $RAMEN_COMMIT"
    cat "$ORACLE_DIR/Dockerfile" "$ORACLE_DIR"/patches/opengmk/*.patch "$ORACLE_DIR"/patches/ramen/*.patch
    while IFS= read -r -d '' file; do
      printf '%s\0' "$file"
      cat "$dir/$file"
    done < <(pxfm_files "$dir")
  } | sha256
}

# The image, tagged by its Dockerfile so worktrees with different Dockerfiles don't overwrite each other's.
image() {
  echo "spelunky-oracle:$(sha256 <"$ORACLE_DIR/Dockerfile" | cut -c1-16)"
}

# Where this checkout's build of gm8emulator lives in the target volume; $1 is the build stamp if already known.
emulator_path() {
  local stamp="${1:-}"
  if [[ -z "$stamp" ]]; then
    stamp="$(build_stamp)" || return 1
  fi
  echo "/target/oracle/$stamp/gm8emulator"
}

# Takes lock directory $1, waiting while someone else holds it; unlock_all releases it (call it on exit). mkdir is
# atomic on every platform, while flock isn't available on macOS. A lock whose holder has died is taken over.
LOCKS=()
lock() {
  local dir="$1" holder waiting=""
  until mkdir "$dir" 2>/dev/null; do
    holder="$(cat "$dir/pid" 2>/dev/null || true)"
    if [[ -n "$holder" ]] && ! kill -0 "$holder" 2>/dev/null; then
      echo "oracle: taking over $(basename "$dir") from pid $holder, which has exited" >&2
      rm -rf "$dir"
      continue
    fi
    if [[ -z "$waiting" ]]; then
      echo "oracle: waiting for $(basename "$dir") (held by pid ${holder:-?})" >&2
      waiting=1
    fi
    sleep 1
  done
  echo "$$" >"$dir/pid"
  LOCKS+=("$dir")
}

unlock_all() {
  local dir
  for dir in "${LOCKS[@]+"${LOCKS[@]}"}"; do
    rm -rf "$dir"
  done
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
