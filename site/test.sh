#!/bin/sh
# The site's smoke test: builds it with a stand-in web build (build.sh --local), serves it with
# `wrangler dev` (workerd, as Cloudflare runs it), and checks every path's status, headers and content
# (docs/web-deploy.md, "Testing locally"). Needs Node and the network for npm ci, nothing of the game's.
#
#   site/test.sh          leaves site/dist built with the stand-in, so run build.sh again before serving it
set -eu
cd "$(dirname "$0")"
port=${SITE_TEST_PORT:-8790}
www_port=$((port + 1))
export WRANGLER_SEND_METRICS=false

tmp=$(mktemp -d)
pids=
cleanup() {
  for pid in $pids; do kill "$pid" 2>/dev/null || true; done
  rm -rf "$tmp"
}
trap cleanup EXIT

# A stand-in for scripts/web --package's tarball: the page, its manifest, and a content-hashed v/HASH/.
mkdir -p "$tmp/web/v/0123456789abcdef"
echo '<!doctype html><title>stand-in web build</title>' >"$tmp/web/index.html"
echo 'export default function init() {}' >"$tmp/web/v/0123456789abcdef/app.js"
echo 'stand-in manifest.txt' >"$tmp/web/manifest.txt"
tar -czf "$tmp/web.tar.gz" -C "$tmp/web" .
echo '{"version":"stand-in"}' >"$tmp/latest.json"
./build.sh --local "$tmp/web.tar.gz" --manifest "$tmp/latest.json"

# serve PORT [ARG...]: wrangler dev on PORT, once it answers. One at a time: two starting at once can pick
# the same internal port.
serve() {
  p=$1
  shift
  node_modules/.bin/wrangler dev --port "$p" "$@" >"$tmp/dev-$p.log" 2>&1 &
  pids="$pids $!"
  tries=0
  until curl -s -o /dev/null "http://localhost:$p/"; do
    tries=$((tries + 1))
    if [ $tries -gt 120 ]; then
      cat "$tmp/dev-$p.log" >&2
      echo "site/test.sh: wrangler dev didn't start on port $p" >&2
      exit 1
    fi
    sleep 0.5
  done
}
# Two Workers: one as spelunky.ai sees requests, one as www.spelunky.ai does (--host sets the URL's host).
serve "$port"
serve "$www_port" --host www.spelunky.ai

failures=0
fail() {
  echo "FAIL $*" >&2
  failures=$((failures + 1))
}
# check PATH STATUS [HEADER: VALUE]...: the response's status, and each header's value exactly.
check() {
  url=$1 status=$2 before=$failures
  shift 2
  curl -s -D "$tmp/headers" -o "$tmp/body" "$url" || { fail "$url: no response"; return; }
  tr -d '\r' <"$tmp/headers" >"$tmp/h"
  got=$(head -1 "$tmp/h" | cut -d' ' -f2)
  [ "$got" = "$status" ] || fail "$url: status $got, not $status"
  for header in "$@"; do
    name=${header%%:*}
    want=${header#*: }
    value=$(grep -i "^$name:" "$tmp/h" | head -1 | cut -d' ' -f2-)
    [ "$value" = "$want" ] || fail "$url: $name '$value', not '$want'"
  done
  [ $failures != "$before" ] || echo "ok   $url $got"
}
# same FILE: the last response's body is FILE's contents.
same() {
  cmp -s "$tmp/body" "$1" || fail "$url: body isn't $1"
}
# contains TEXT...: the last response's body contains each TEXT.
contains() {
  for text in "$@"; do
    grep -qF -- "$text" "$tmp/body" || fail "$url: no '$text'"
  done
}

site=http://localhost:$port
no_cache="cache-control: no-cache"
immutable="cache-control: public, max-age=31536000, immutable"

# The pre-rendered landing page.
check "$site/" 200 "$no_cache" "content-type: text/html; charset=utf-8"
contains '<html lang="en">' \
  '<title>Spelunky Classic in your browser (unofficial port)</title>' \
  '<meta property="og:image" content="https://spelunky.ai/media/gameplay.png"/>' \
  '<img class="logo" src="media/logo.png" width="160" height="36" alt="Spelunky"/>' \
  'Spelunky Classic 1.1, ported to Rust tick for tick, bugs included.' \
  '<a class="play" href="/play/">Play in your browser</a>' \
  'autoPlay="" loop="" muted="" playsInline=""' \
  '<a href="/LICENSE.txt">Spelunky User License v1.1b</a>' \
  '.play:hover, .play:focus-visible { background: #ffd860; outline: none; }'
check "$site/index.html" 307 "location: /"
check "$site/media/logo.png" 200 "$no_cache" "content-type: image/png"
same public/media/logo.png
# Its scripts: content-hashed, so served without the Worker, and cached for good.
script=$(cd dist/client && ls assets/*.js | head -1)
check "$site/$script" 200 "$immutable" "x-content-type-options: nosniff"

# The web build.
check "$site/play" 307 "location: /play/" "$no_cache"
check "$site/play/" 200 "$no_cache" "content-type: text/html; charset=utf-8"
same "$tmp/web/index.html"
check "$site/play/manifest.txt" 200 "$no_cache"
check "$site/play/v/0123456789abcdef/app.js" 200 "$immutable" "x-content-type-options: nosniff"
same "$tmp/web/v/0123456789abcdef/app.js"
# A miss under v/ skips the Worker too (docs/web-deploy.md, "Rolling back").
check "$site/play/v/fedcba9876543210/app.js" 404 "$immutable"

check "$site/releases/latest.json" 200 "$no_cache" "content-type: application/json"
same "$tmp/latest.json"
check "$site/LICENSE.txt" 200 "$no_cache" "content-type: text/plain; charset=utf-8"
same ../LICENSE

check "$site/nowhere" 404 "$no_cache" "content-type: text/html; charset=utf-8"
same public/404.html
# Not a page, so the Worker doesn't ask the router.
check "$site/favicon.ico" 404 "$no_cache"
same public/404.html
# Only GET and HEAD: the asset store answers anything else.
got=$(curl -s -o /dev/null -w '%{http_code}' -X POST "$site/")
[ "$got" = 405 ] || fail "POST $site/: status $got, not 405"

# www redirects to the apex, path and query kept (http here only because wrangler dev is).
check "http://localhost:$www_port/play/?a=1" 301 "location: http://spelunky.ai/play/?a=1"

if [ $failures -gt 0 ]; then
  echo "site/test.sh: $failures failed" >&2
  exit 1
fi
echo "site/test.sh: all passed"
