#!/bin/sh
# The site's smoke test: builds it with a stand-in web build (build.sh --local), serves it with
# `wrangler dev` (workerd, as Cloudflare runs it), and checks every path's status, headers and content
# (docs/web-deploy.md, "Testing locally"). Needs Node and the network for npm ci, and nothing of the game's but
# the committed media's LICENSE.txt and NOTICE.txt (wiki/media), with its stand-in media.
#
#   site/test.sh          leaves site/dist built with the stand-in, so run build.sh again before serving it
#   SITE_TEST_PORT=N site/test.sh    serves on N and N + 1, and fails if either is taken (by default, the first
#                                    free pair from 8790)
set -eu
cd "$(dirname "$0")"
export WRANGLER_SEND_METRICS=false

# free PORT: nothing answers on PORT, on any address (curl can't connect, its exit 7), and nothing listens there.
free() {
  got=0
  curl -s -o /dev/null --max-time 2 "http://localhost:$1/" || got=$?
  [ "$got" -eq 7 ] && ! lsof -nP -iTCP:"$1" -sTCP:LISTEN >/dev/null 2>&1
}
# taken PORT: SITE_TEST_PORT's pair isn't free, so stop rather than test whatever answers there.
taken() {
  lsof -nP -iTCP:"$1" -iTCP:$(($1 + 1)) -sTCP:LISTEN >&2 || true
  echo "site/test.sh: something else answers on port $1 or $(($1 + 1)) (SITE_TEST_PORT); stop it, or leave SITE_TEST_PORT unset for a free pair" >&2
  exit 1
}
# Before the build, which takes a while (and again before serving, below).
if [ -n "${SITE_TEST_PORT:-}" ]; then
  case $SITE_TEST_PORT in
  *[!0-9]*)
    echo "site/test.sh: SITE_TEST_PORT isn't a port: $SITE_TEST_PORT" >&2
    exit 1
    ;;
  esac
  if ! { free "$SITE_TEST_PORT" && free $((SITE_TEST_PORT + 1)); }; then taken "$SITE_TEST_PORT"; fi
fi

tmp=$(mktemp -d)
pids=
cleanup() {
  for pid in $pids; do kill "$pid" 2>/dev/null || true; done
  rm -rf "$tmp"
}
trap cleanup EXIT

# A stand-in for scripts/web --package's tarball: the page, its manifest, and a content-hashed v/HASH/.
mkdir -p "$tmp/web/v/0123456789abcdef"
# Its page can be embedded, seeks, plays on phones, plays clips cut from committed replays, reports its start-up
# run, reports its state at a clip's first frame, runs a clip through, reporting its states at its frames, and fills
# its frame, as the wiki's clips ask (src/wiki/live-clip.tsx).
echo '<!doctype html><meta name="spelunky-embed" content="1"><meta name="spelunky-embed-seek" content="1"><meta name="spelunky-embed-held-sound" content="1"><meta name="spelunky-embed-replays" content="1"><meta name="spelunky-embed-progress" content="1"><meta name="spelunky-embed-state" content="1"><meta name="spelunky-embed-states" content="1"><meta name="spelunky-embed-fill" content="1"><title>stand-in web build</title>' >"$tmp/web/index.html"
echo 'export default function init() {}' >"$tmp/web/v/0123456789abcdef/app.js"
echo 'stand-in manifest.txt' >"$tmp/web/manifest.txt"
tar -czf "$tmp/web.tar.gz" -C "$tmp/web" .
echo '{"version":"stand-in"}' >"$tmp/latest.json"
# And for the wiki's media (wiki/media, which scripts/wiki-media renders), with its real LICENSE.txt and
# NOTICE.txt: the indexes, the pages' sprites and the Snake's clip, with
# what of docs/wiki-media.md's fields src/seo/media-package.ts reads (test-media.mjs checks what the JSON-LD
# and sitemaps make of them), and the sprites the Open Graph images draw
# (src/wiki/build/og.ts: the fonts, the title and the Spelunker, which test-og.mjs makes). The page's other
# clips aren't there, as a clip whose spec is new isn't until it's rendered.
media=$tmp/wiki-media
mkdir -p "$media/sprites/sBlank" "$media/clips/snake/bite"
# Every written page's infobox sprite and the hubs' card sprites (the pages' and planned pages' `card`s, a
# page without an infobox's own `card`, and
# src/wiki/build/hubs.ts's CARD_SPRITES), and the sprites pages show in their text (`<Sprite name="sName">`:
# a sprite the media haven't got fails the build), each a 16-pixel stand-in of 4 frames whose first
# test-og.mjs draws, and a blank one.
# No package (it runs before build.sh's npm ci): hubs.ts imports none, and the pages' and planned pages'
# sprites are each a `sprite: sName` in the image's or card's flow mapping, on one line (a page's own `card`
# at the top level), or a `<Sprite name="sName"` anywhere in a page.
sprites=$(node --input-type=module -e '
import { readdirSync, readFileSync } from "node:fs";
import { CARD_SPRITES } from "./src/wiki/build/hubs.ts";
const names = new Set(Object.values(CARD_SPRITES).map((card) => card.sprite));
const lines = [
  ...readdirSync("content/wiki").filter((f) => f.endsWith(".mdx")).flatMap((f) => readFileSync(`content/wiki/${f}`, "utf8").split("\n").filter((l) => l.startsWith("  image: ") || l.startsWith("card: ") || l.includes("<Sprite "))),
  ...readFileSync("content/wiki-planned.yaml", "utf8").split("\n").filter((l) => /^[a-z0-9-]+: .*\bcard: /.test(l)),
];
for (const line of lines) for (const [, image, body] of line.matchAll(/\bsprite: (s[A-Za-z0-9_]+)|<Sprite name="(s[A-Za-z0-9_]+)"/g)) names.add(image ?? body);
console.log([...names].filter(Boolean).sort().join("\n"));
')
entries=
for s in $sprites; do
  entries="$entries{\"name\":\"$s\",\"meta\":\"$s/sprite.json\",\"frames\":4,\"width\":16,\"height\":16},"
  mkdir -p "$media/sprites/$s/frames"
  frames=
  for i in 0 1 2 3; do
    frames="$frames${frames:+,}{\"file\":\"$s/frames/$i.png\",\"file_scaled\":\"$s/frames/$i@4x.png\"}"
    echo "stand-in $i@4x.png" >"$media/sprites/$s/frames/$i@4x.png"
  done
  echo "{\"width\":16,\"height\":16,\"origin\":{\"x\":0,\"y\":0},\"frames\":[$frames],\"anim_scaled\":\"$s/anim@4x.png\"}" >"$media/sprites/$s/sprite.json"
  echo 'stand-in anim@4x.png' >"$media/sprites/$s/anim@4x.png"
done
cat >"$media/sprites/index.json" <<EOF2
{"scale":4,"sprites":[$entries
 {"name":"sBlank","meta":"sBlank/sprite.json","frames":1,"width":0,"height":0},
 {"name":"sFont","meta":"sFont/sprite.json","frames":59,"width":16,"height":16},
 {"name":"sFontSmall","meta":"sFontSmall/sprite.json","frames":59,"width":8,"height":8},
 {"name":"sTitle","meta":"sTitle/sprite.json","frames":1,"width":128,"height":16},
 {"name":"sStandLeft","meta":"sStandLeft/sprite.json","frames":1,"width":16,"height":16}],
 "objects":[{"name":"oSnake","sprite":"sSnakeWalkL"},{"name":"oGame","sprite":null}]}
EOF2
# A sprite without pixels has no files.
echo '{"frames":[{"file_scaled":null}],"anim_scaled":null}' >"$media/sprites/sBlank/sprite.json"
cat >"$media/clips/index.json" <<'EOF2'
{"caption":"Recorded from the spelunky.ai port v9.9.9, a stand-in.","clips":[{"id":"snake/bite",
 "title":"Snake bite","description":"A snake bites the player & takes a heart.","duration":3.3333333333333335,
 "width":640,"height":480,"has_audio":true,"source":{"kind":"scene","seed":0},"from":40,"replay_from":281,"state_hash":"5ba7e1e5ba7e1e5b","states_hash":"5ba7e1e5ba7e5ba7",
 "files":{"mp4":"snake/bite/clip.mp4","webm":"snake/bite/clip.webm","poster":"snake/bite/poster.png","replay":"snake/bite/replay.txt"}}]}
EOF2
# A few files of the gml/ tree, the source viewer's (src/wiki/build/source.ts), with each kind of token, a
# string and a block comment that carry on to the next line, and a Super Sound System script, whose page names
# its author. And the file scripts/check's wiki-cites reads first. The pages' citations link to what
# it has (compile.ts's sourceHref): the Snake's of lines 3-4 of its step event, and the double Jar roll's
# Where, all 21 lines of the Jar's, but not the Snake's of lines past their file's end, nor any of the files
# it hasn't got.
mkdir -p "$media/gml/objects/oSnake" "$media/gml/objects/oPlayer1" "$media/gml/objects/oJar" "$media/gml/scripts"
cat >"$media/gml/objects/oSnake/step.gml" <<'EOF2'
// ==== action 1: lib 1 id 603: code ====
if (hp < 1 and x > 0) { scrCreateBlood(x+8, y+8, 3); }
/* a comment
 that ends here */ msg = "two
lines" + 'q';
global.snakes += $1F;
EOF2
echo 'hp = 1;' >"$media/gml/objects/oSnake/create.gml"
echo '// SS_Init()' >"$media/gml/scripts/SS_Init.gml"
echo 'x += 1;' >"$media/gml/objects/oPlayer1/step.gml"
for i in $(seq 21); do echo "n = $i;"; done >"$media/gml/objects/oJar/collision_oWhip.gml"
# The rooms/ the source viewer's room pages are made from, and the room citations link to and are checked against:
# every room the pages cite, as an empty room, and the Tutorial with a few instances of three objects (one with
# events in gml/), a tile and a background, as the extractor writes them.
mkdir -p "$media/rooms"
for room in $(grep -rhoE '<Cite room="[^"]+"' content/wiki | sed 's/.*="//; s/"$//' | sort -u); do
  printf '{"name":"%s","caption":"","width":320,"height":240,"speed":30,"persistent":false,"background_color":"#c0c0c0","draw_background_color":true,"creation_code":"","backgrounds":[],"views_enabled":false,"views":[],"tiles":[],"instances":[]}\n' \
    "$room" >"$media/rooms/$room.json"
done
cat >"$media/rooms/rTutorial.json" <<'EOF2'
{
  "name": "rTutorial",
  "caption": "",
  "width": 672,
  "height": 304,
  "speed": 30,
  "persistent": false,
  "background_color": "#c0c0c0",
  "draw_background_color": false,
  "creation_code": "",
  "backgrounds": [{"slot":0,"visible":true,"foreground":false,"background":"bgCave","x":0,"y":0,"tile_h":true,"tile_v":true,"hspeed":0,"vspeed":0,"stretch":false}],
  "views_enabled": true,
  "views": [{"slot":0,"view_x":0,"view_y":0,"view_w":320,"view_h":240,"port_x":0,"port_y":0,"port_w":640,"port_h":480,"hborder":128,"vborder":96,"hspeed":-1,"vspeed":-1,"follow":"oPlayer1"}],
  "tiles": [
    {"id":10000342,"background":"bgHmm","x":16,"y":96,"bg_x":0,"bg_y":0,"width":32,"height":32,"depth":1000000}
  ],
  "instances": [
    {"id":134001,"object":"oSnake","x":320,"y":112},
    {"id":134002,"object":"oBrick","x":0,"y":0},
    {"id":134003,"object":"oBrick","x":16,"y":0},
    {"id":134005,"object":"oDrawnSprite","x":32,"y":0},
    {"id":134004,"object":"oGoldBar","x":96,"y":112,"creation_code":"value = 2;\n"}
  ]
}
EOF2
# The room's picture (scripts/wiki-media's wiki-rooms): any PNG, 180x180, which build.sh serves and the room's
# page draws, boxing its instances' sprites.
cp public/apple-touch-icon.png "$media/rooms/rTutorial.png"
echo 'stand-in clip.mp4' >"$media/clips/snake/bite/clip.mp4"
echo 'stand-in clip.webm' >"$media/clips/snake/bite/clip.webm"
echo 'stand-in poster.png' >"$media/clips/snake/bite/poster.png"
echo 'stand-in replay.txt' >"$media/clips/snake/bite/replay.txt"
cp ../wiki/media/LICENSE.txt ../wiki/media/NOTICE.txt "$media/"
node test-og.mjs stand-in "$media"
# Committed, as wiki/media is, so that build.sh dates the clip by git's history: from the commit that last
# changed its videos, before a later one that changes only its poster (the media's date). And cloned at depth 1,
# as Workers Builds clones, where git shows the later commit adding the clip, until build.sh fetches the rest.
# (Not a repository a caller's GIT_DIR names.)
# The commits go in with one `git fast-import`, which writes a single pack, not with `git add` and `git commit`,
# which write and freshen the thousand-odd files' objects as loose ones in .git/objects: on the Mac minis, about
# one run in seven had one of those fail ("unable to create temporary file: Invalid argument"), and the clone's
# own pack and checkout never did (docs/ci.md, "The Mac minis' loose objects").
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE
# stand_in_commit DATE: a commit, for fast-import's stream, of every file in the media as they are now.
stand_in_commit() {
  node --input-type=module -e '
import { readdirSync, readFileSync } from "node:fs";
import { join, relative } from "node:path";
const [dir, date] = process.argv.slice(1);
const who = `stand-in <stand-in@example.com> ${Date.parse(date) / 1000} +0000`;
const chunks = [`commit refs/heads/main\nauthor ${who}\ncommitter ${who}\ndata 8\nstand-in\ndeleteall\n`];
const files = readdirSync(dir, { recursive: true, withFileTypes: true }).filter((entry) => entry.isFile());
for (const file of files.map((entry) => relative(dir, join(entry.parentPath, entry.name))).sort()) {
  const data = readFileSync(join(dir, file));
  chunks.push(`M 100644 inline ${file}\ndata ${data.length}\n`, data, "\n");
}
process.stdout.write(Buffer.concat(chunks.map((chunk) => Buffer.from(chunk))));
' "$media" "$1"
}
{
  stand_in_commit 2026-10-20T08:30:00Z
  echo 'stand-in poster.png, again' >"$media/clips/snake/bite/poster.png"
  stand_in_commit 2026-10-27T11:16:56Z
} >"$tmp/wiki-media.fast-import"
git -c init.defaultBranch=main init -q --bare "$tmp/wiki-media-origin"
# A pack however few the objects: under fastimport.unpackLimit's 100 (the stand-in has about 200), fast-import
# would make them loose.
git -C "$tmp/wiki-media-origin" -c fastimport.unpackLimit=0 fast-import --quiet <"$tmp/wiki-media.fast-import"
clip_date=2026-10-20T08:30:00Z
rm -rf "$media"
# Which lets build.sh's fetch leave out the files, as GitHub does.
git -C "$tmp/wiki-media-origin" config uploadpack.allowFilter true
git clone -q --depth 1 "file://$tmp/wiki-media-origin" "$media"
media_url=/wiki/media
./build.sh --local "$tmp/web.tar.gz" --manifest "$tmp/latest.json" --wiki-media "$media"

# running PID: PID hasn't exited (a zombie has, though kill -0 still finds it until the shell reaps it).
running() {
  case $(ps -o stat= -p "$1") in '' | Z*) return 1 ;; esac
}
# descends PID ANCESTOR: PID is ANCESTOR or one of its descendants.
descends() {
  a=$1
  until [ "$a" = "$2" ]; do
    a=$(ps -o ppid= -p "$a" | tr -d ' ')
    case $a in '' | 0 | 1) return 1 ;; esac
  done
}
# serve PORT [ARG...]: wrangler dev on PORT, once it answers. One at a time: two starting at once can pick
# the same internal port. Whatever answers has to be this wrangler dev's (its workerd, a grandchild, listens),
# or every check below would test another server, such as another checkout's site/test.sh's: so every listener
# on PORT must descend from it. Returns 1 if another server has PORT, and fails for anything else.
serve() {
  p=$1
  shift
  node_modules/.bin/wrangler dev --port "$p" "$@" >"$tmp/dev-$p.log" 2>&1 &
  pid=$!
  pids="$pids $pid"
  tries=0
  until curl -s -o /dev/null --max-time 2 "http://localhost:$p/"; do
    tries=$((tries + 1))
    if ! running "$pid" || [ $tries -gt 120 ]; then
      # It stops at once when PORT is taken ("Address already in use").
      if ! running "$pid" && ! free "$p"; then return 1; fi
      cat "$tmp/dev-$p.log" >&2
      echo "site/test.sh: wrangler dev didn't start on port $p" >&2
      exit 1
    fi
    sleep 0.5
  done
  listeners=$(lsof -nP -iTCP:"$p" -sTCP:LISTEN -t) || return 1
  for l in $listeners; do descends "$l" "$pid" || return 1; done
}
# stop: the wrangler devs serve started, gone (and waited for, which keeps the shell from reporting each).
stop() {
  for pid in $pids; do
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
  done
  pids=
}
# Two Workers: one as spelunky.ai sees requests, one as www.spelunky.ai does (--host sets the URL's host), on
# SITE_TEST_PORT and the port after it, which must be free, or else on the first free pair from 8790 up. Another
# server can take one between the check and wrangler dev's start (another checkout's site/test.sh, picking the
# same pair at the same time), and then it's the next pair's turn.
port=${SITE_TEST_PORT:-8790}
until free "$port" && free $((port + 1)) && serve "$port" && serve $((port + 1)) --host www.spelunky.ai; do
  stop
  if [ -n "${SITE_TEST_PORT:-}" ]; then taken "$port"; fi
  port=$((port + 2))
  if [ $port -gt 8888 ]; then
    echo "site/test.sh: no free pair of ports from 8790 to 8889" >&2
    exit 1
  fi
done
www_port=$((port + 1))
echo "site/test.sh: serving on ports $port and $www_port"

failures=0
fail() {
  echo "FAIL $*" >&2
  failures=$((failures + 1))
}
# with [--head] [HEADER: VALUE]...: the next check is a HEAD, and sends these headers.
head=
: >"$tmp/send"
with() {
  if [ "${1-}" = --head ]; then
    head=-I
    shift
  fi
  for header in "$@"; do echo "$header"; done >"$tmp/send"
}
# check PATH STATUS [HEADER: VALUE]...: the response's status, and each header's value exactly.
check() {
  url=$1 status=$2 before=$failures
  shift 2
  # What with set is for this check alone, whether or not it gets an answer.
  sent=$(cat "$tmp/send")
  mv "$tmp/send" "$tmp/sending"
  : >"$tmp/send"
  how=$head
  head=
  curl -s $how -H @"$tmp/sending" -D "$tmp/headers" -o "$tmp/body" "$url" || { fail "$url: no response"; return; }
  tr -d '\r' <"$tmp/headers" >"$tmp/h"
  got=$(head -1 "$tmp/h" | cut -d' ' -f2)
  [ "$got" = "$status" ] || fail "$url: status $got, not $status"
  for header in "$@"; do
    name=${header%%:*}
    want=${header#*: }
    value=$(grep -i "^$name:" "$tmp/h" | head -1 | cut -d' ' -f2-)
    [ "$value" = "$want" ] || fail "$url: $name '$value', not '$want'"
  done
  [ $failures != "$before" ] || echo "ok   $url $got${sent:+ ($(echo "$sent" | tr '\n' ' ' | sed 's/ $//'))}"
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
# starts TEXT: the last response's body starts with TEXT.
starts() {
  printf '%s' "$1" >"$tmp/start"
  head -c $(($(wc -c <"$tmp/start"))) "$tmp/body" | cmp -s - "$tmp/start" || fail "$url: doesn't start with '$1'"
}
# lacks TEXT...: the last response's body contains none of them.
lacks() {
  for text in "$@"; do
    ! grep -qF -- "$text" "$tmp/body" || fail "$url: has '$text'"
  done
}
# The notice that starts the files read without a page's footer (docs/wiki-license.md, "Per-page notice").
notice='Spelunky is © 2008, 2009 Derek Yu and Mossmouth, LLC, and is used under the [Spelunky User License v1.1b](https://spelunky.ai/LICENSE.txt). spelunky.ai is an unofficial fan project, not affiliated with or endorsed by Mossmouth. The game and its assets may not be sold.'
audio_credit='Music by George Buzinkai and Jonathan Perry; sound effects made in SFXR. From Spelunky Classic 1.1, © 2008, 2009 Derek Yu and Mossmouth, LLC.'

site=http://localhost:$port
no_cache="cache-control: no-cache"
immutable="cache-control: public, max-age=31536000, immutable"
# The wiki's media keep their names when they're rendered again (public/_headers).
media_cache="cache-control: public, max-age=86400"

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
# Its head (src/seo/): the canonical URL and the JSON-LD; test-seo.mjs, below, checks every page's.
contains '<link rel="canonical" href="https://spelunky.ai/"/>' \
  '<link rel="search" type="application/opensearchdescription+xml" title="spelunky.ai wiki" href="https://spelunky.ai/opensearch.xml"/>' \
  '<link rel="icon" href="/favicon.ico" sizes="32x32"/>' '<link rel="icon" type="image/svg+xml" href="/icon.svg"/>' \
  '<link rel="apple-touch-icon" href="/apple-touch-icon.png"/>' \
  '<meta name="twitter:card" content="summary_large_image"/>' \
  '<script type="application/ld+json">{"@context":"https://schema.org","@type":"WebSite"' \
  '"urlTemplate":"https://spelunky.ai/wiki?q={search_term_string}"'
check "$site/index.html" 307 "location: /"
# For crawlers: robots.txt names the sitemaps, which src/seo/sitemap.ts makes at build time.
check "$site/robots.txt" 200 "$no_cache" "content-type: text/plain; charset=utf-8"
contains 'User-agent: *' 'Sitemap: https://spelunky.ai/sitemap.xml' 'Sitemap: https://spelunky.ai/sitemap-video.xml' \
  'Sitemap: https://spelunky.ai/sitemap-image.xml'
check "$site/robots.txt/" 307 "location: $site/robots.txt" "$no_cache"
check "$site/sitemap.xml" 200 "$no_cache" "content-type: application/xml"
contains '<loc>https://spelunky.ai/</loc>' '<loc>https://spelunky.ai/play/</loc>'
check "$site/sitemap.xml/" 307 "location: $site/sitemap.xml" "$no_cache"
# The wiki's clips and sprites, by the pages that show them: the Snake page's clip that the media have (not
# the two they haven't) and its infobox's sprite, animated as it shows it.
check "$site/sitemap-video.xml" 200 "$no_cache" "content-type: application/xml"
contains 'xmlns:video="http://www.google.com/schemas/sitemap-video/1.1"' \
  '<loc>https://spelunky.ai/wiki/snake</loc>' '<video:title>Snake bite</video:title>' \
  "<video:content_loc>https://spelunky.ai$media_url/clips/snake/bite/clip.mp4</video:content_loc>" \
  "<video:thumbnail_loc>https://spelunky.ai$media_url/clips/snake/bite/poster.png</video:thumbnail_loc>" \
  '<video:duration>3</video:duration>' "<video:publication_date>$clip_date</video:publication_date>"
[ "$(grep -c '<video:video>' "$tmp/body")" = 1 ] || fail "$url: not one <video:video>"
check "$site/sitemap-image.xml" 200 "$no_cache" "content-type: application/xml"
contains 'xmlns:image="http://www.google.com/schemas/sitemap-image/1.1"' \
  '<loc>https://spelunky.ai/wiki/snake</loc>' \
  "<image:loc>https://spelunky.ai$media_url/sprites/sSnakeWalkL/anim%404x.png</image:loc>"
# For browsers: the wiki's search as a search engine (src/seo/opensearch.ts), which every page names (above,
# and test-seo.mjs). Pre-rendered, and served with its own type rather than the asset store's application/xml,
# a revalidation's 304 too.
opensearch_type="content-type: application/opensearchdescription+xml"
check "$site/opensearch.xml" 200 "$no_cache" "$opensearch_type"
contains '<OpenSearchDescription xmlns="http://a9.com/-/spec/opensearch/1.1/"' '<ShortName>spelunky.ai wiki</ShortName>' \
  '<Url type="text/html" method="get" template="https://spelunky.ai/wiki?q={searchTerms}"/>' \
  '<Image width="16" height="16" type="image/x-icon">https://spelunky.ai/favicon.ico</Image>'
with "If-None-Match: $(grep -i '^etag:' "$tmp/h" | cut -d' ' -f2-)"
check "$site/opensearch.xml" 304 "$no_cache" "$opensearch_type"
check "$site/opensearch.xml/" 307 "location: $site/opensearch.xml" "$no_cache"
check "$site/media/logo.png" 200 "$no_cache" "content-type: image/png"
same public/media/logo.png
# The site's icons (scripts/site-icons), which every page's head names (above, and test-seo.mjs), as 404.html does.
check "$site/favicon.ico" 200 "$no_cache" "content-type: image/vnd.microsoft.icon"
same public/favicon.ico
check "$site/icon.svg" 200 "$no_cache" "content-type: image/svg+xml"
same public/icon.svg
check "$site/apple-touch-icon.png" 200 "$no_cache" "content-type: image/png"
same public/apple-touch-icon.png
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

# The wiki's media: served without the Worker (but the clips' videos, below), and cached for a day.
check "$site$media_url/sprites/sSnakeWalkL/frames/0%404x.png" 200 "$media_cache" "content-type: image/png" \
  "x-content-type-options: nosniff"
same "$media/sprites/sSnakeWalkL/frames/0@4x.png"
# The asset store's canonical form of a path escapes an @, and it redirects to that, so a page asks for
# 0%404x.png (docs/web-deploy.md, "Wiki media").
check "$site$media_url/sprites/sSnakeWalkL/frames/0@4x.png" 307 \
  "location: $media_url/sprites/sSnakeWalkL/frames/0%404x.png"
# The clips' videos come through the Worker, which answers a Range with a 206, as Safari wants (src/server.ts),
# keeping the asset's headers.
clip=$media_url/clips/snake/bite/clip.mp4
size=$(wc -c <"$media/clips/snake/bite/clip.mp4" | tr -d ' ')
check "$site$clip" 200 "$media_cache" "content-type: video/mp4" "accept-ranges: bytes" "content-length: $size" \
  "x-content-type-options: nosniff"
same "$media/clips/snake/bite/clip.mp4"
etag=$(grep -i '^etag:' "$tmp/h" | cut -d' ' -f2-)
with "Range: bytes=0-7"
check "$site$clip" 206 "content-range: bytes 0-7/$size" "content-length: 8" "$media_cache" "content-type: video/mp4" \
  "accept-ranges: bytes" "etag: $etag"
head -c 8 "$media/clips/snake/bite/clip.mp4" >"$tmp/want"
same "$tmp/want"
# Safari's first ask, and Chrome's.
with "Range: bytes=0-1"
check "$site$clip" 206 "content-range: bytes 0-1/$size" "content-length: 2"
with "Range: bytes=0-"
check "$site$clip" 206 "content-range: bytes 0-$((size - 1))/$size" "content-length: $size"
same "$media/clips/snake/bite/clip.mp4"
# The last 4 bytes, and a last byte past the end.
with "Range: bytes=-4"
check "$site$clip" 206 "content-range: bytes $((size - 4))-$((size - 1))/$size" "content-length: 4"
tail -c 4 "$media/clips/snake/bite/clip.mp4" >"$tmp/want"
same "$tmp/want"
with "Range: bytes=3-9"
check "$site$clip" 206 "content-range: bytes 3-9/$size" "content-length: 7"
tail -c +4 "$media/clips/snake/bite/clip.mp4" | head -c 7 >"$tmp/want"
same "$tmp/want"
with "Range: bytes=4-99999"
check "$site$clip" 206 "content-range: bytes 4-$((size - 1))/$size" "content-length: $((size - 4))"
# What the Worker ignores, for a plain 200 with the whole file: none of it (past the end, or an empty suffix),
# several ranges, a malformed one, another unit, and an If-Range that isn't the file's ETag; one that is gets
# its range.
for range in "bytes=$size-" "bytes=-0" "bytes=0-1,4-5" "bytes=5-2" "bytes=x-" "items=0-1"; do
  with "Range: $range"
  check "$site$clip" 200 "content-length: $size" "accept-ranges: bytes" "content-range: " "$media_cache"
  same "$media/clips/snake/bite/clip.mp4"
done
with "Range: bytes=0-7" 'If-Range: "stale"'
check "$site$clip" 200 "content-length: $size"
with "Range: bytes=0-7" "If-Range: $etag"
check "$site$clip" 206 "content-range: bytes 0-7/$size"
# A HEAD has the size the asset store's leaves out.
with --head
check "$site$clip" 200 "content-length: $size" "accept-ranges: bytes" "$media_cache"
with --head "Range: bytes=0-7"
check "$site$clip" 206 "content-length: 8" "content-range: bytes 0-7/$size"
with "If-None-Match: $etag"
check "$site$clip" 304 "etag: $etag"
check "$site$media_url/clips/snake/bite/clip.webm" 200 "$media_cache" "content-type: video/webm" "accept-ranges: bytes"
same "$media/clips/snake/bite/clip.webm"
with "Range: bytes=0-7"
check "$site$media_url/clips/snake/bite/clip.webm" 206 "content-range: bytes 0-7/$(wc -c <"$media/clips/snake/bite/clip.webm" | tr -d ' ')" \
  "content-type: video/webm" "$media_cache"
# A clip that isn't there is the asset store's 404 (with the media's caching, as for a sprite below).
check "$site$media_url/clips/snake/stomp/clip.mp4" 404 "$media_cache"
# The rest of the media skip the Worker, so a Range gets the whole file, with no Accept-Ranges.
for file in sprites/sSnakeWalkL/frames/0%404x.png clips/snake/bite/poster.png clips/snake/bite/replay.txt \
  clips/index.json NOTICE.txt; do
  with "Range: bytes=0-1"
  check "$site$media_url/$file" 200 "$media_cache" "accept-ranges: " "content-range: "
  same "$media/$(echo "$file" | sed 's/%40/@/g')"
done
check "$site$media_url/sprites/index.json" 200 "$media_cache" "content-type: application/json"
# The media's text files, in UTF-8 as _headers says (test-server.mjs checks that rule against an asset store
# like Cloudflare's, which wrangler dev's isn't): the license is LICENSE, windows-1252, transcoded, its ® two
# bytes here.
check "$site$media_url/NOTICE.txt" 200 "$media_cache" "content-type: text/plain; charset=utf-8"
contains 'spelunky.ai wiki media' 'License v1.1b, in LICENSE.txt beside this file'
iconv -f WINDOWS-1252 -t UTF-8 ../LICENSE >"$tmp/LICENSE.txt"
check "$site$media_url/LICENSE.txt" 200 "$media_cache" "content-type: text/plain; charset=utf-8"
same "$tmp/LICENSE.txt"
contains 'Spelunky® User License v1.1b'
# A miss skips the Worker as /play/v/'s do.
check "$site$media_url/sprites/nothing.json" 404 "$media_cache"
# The media's URLs from before they were committed, under a release's package's hash, are what's at the same
# path now (src/server.ts): the clips', and the sprites', posters' and indexes' too, which the Worker sees only
# for that.
for file in clips/snake/bite/clip.mp4 clips/snake/bite/poster.png sprites/sSnakeWalkL/anim%404x.png \
  sprites/index.json LICENSE.txt; do
  check "$site/wiki/media/05d8cd2055735618/$file" 301 "location: $site$media_url/$file"
done
# Nor is the directory a page.
check "$site/wiki/media" 404 "$no_cache"

# The wiki (src/routes/wiki/): the Snake's page, pre-rendered at a flat URL, with its numbers, its notices
# (docs/wiki-license.md), its clip and sprite from the media (the clip a scene's, so with its button to play it in
# the game, src/wiki/live-clip.tsx), and a placeholder for a clip they haven't got;
# and its layout (src/wiki/layout.tsx): the breadcrumbs its JSON-LD has too (through its category's hub), the
# infobox with its citations and relations, the table of contents (the wide screen's rail beside the article,
# none of it marked current until the browser's scroll-spy does, and the body's disclosure, closed, its panel
# in the page), the sections' anchors and the navbox, its title linking the hub.
check "$site/wiki/snake" 200 "$no_cache" "content-type: text/html; charset=utf-8"
contains '<title>Snake – Spelunky Classic wiki</title>' \
  '<link rel="canonical" href="https://spelunky.ai/wiki/snake"/>' \
  '<meta name="description" content="The Snake is a monster in Spelunky Classic, found mostly in the Mines, that slithers back and forth along the ground and bites for 1 heart on contact."/>' \
  '<meta property="og:type" content="article"/>' '<meta property="og:image" content="https://spelunky.ai/wiki/og/snake.' \
  '<meta property="og:image:width" content="1200"/>' '<meta property="og:image:height" content="630"/>' \
  '"@type":"Article"' '"@type":"BreadcrumbList"' \
  "{\"@context\":\"https://schema.org\",\"@type\":\"VideoObject\",\"creditText\":\"Derek Yu and Mossmouth, LLC (Spelunky Classic); music by George Buzinkai and Jonathan Perry\"" \
  "\"name\":\"Snake bite\"" "\"thumbnailUrl\":\"https://spelunky.ai$media_url/clips/snake/bite/poster.png\"" \
  "\"contentUrl\":\"https://spelunky.ai$media_url/clips/snake/bite/clip.mp4\"" "\"uploadDate\":\"$clip_date\"" \
  '"duration":"PT3S","width":640,"height":480' \
  "{\"@context\":\"https://schema.org\",\"@type\":\"ImageObject\"" \
  "\"contentUrl\":\"https://spelunky.ai$media_url/sprites/sSnakeWalkL/anim%404x.png\"" \
  '"caption":"A green snake with a tan belly, coiled and facing left"' \
  '<span class="wiki-badge">Wiki</span>' \
  'spelunky.ai is an unofficial fan project, not affiliated with or endorsed by Mossmouth.' \
  '<h1>Snake</h1>' '<h2 id="behaviour">Behaviour</h2>' \
  '<a class="anchor" href="#behaviour" aria-label="Link to this section" aria-describedby="behaviour">#</a>' \
  '<nav class="breadcrumbs" aria-label="Breadcrumb"><ol><li><a href="/wiki">Wiki</a></li><li><a href="/wiki/monsters">Monsters</a></li><li><span aria-current="page">Snake</span></li></ol></nav>' \
  '"@type":"BreadcrumbList","itemListElement":[{"@type":"ListItem","position":1,"name":"Wiki","item":"https://spelunky.ai/wiki"},{"@type":"ListItem","position":2,"name":"Monsters","item":"https://spelunky.ai/wiki/monsters"},{"@type":"ListItem","position":3,"name":"Snake","item":"https://spelunky.ai/wiki/snake"}]' \
  '<th scope="row">Health</th><td><data value="1" data-cite="objects/oSnake/create.gml:10">1 hit point</data><sup class="cite"><a href="#cite-1">[1]</a></sup></td>' \
  '<li id="cite-1"><code>objects/oSnake/create.gml:10</code></li>' \
  '<li id="cite-4"><code>objects/oSnake/step.gml:39-72</code></li>' \
  '<a href="/wiki/source/objects/oSnake/step#L3-L4"><code>objects/oSnake/step.gml:3-4</code></a></li>' \
  '<ul class="sources"><li><a href="/wiki/source/objects/oSnake/create"><code>objects/oSnake/create.gml</code></a></li><li><a href="/wiki/source/objects/oSnake/step"><code>objects/oSnake/step.gml</code></a></li><li><code>objects/oEnemy/create.gml</code></li>' \
  '<th scope="row">Object</th><td><span><code>oSnake</code></span></td>' \
  '<th scope="row">Found in</th><td><ul class="relation-links"><li><a id="base-ui-' \
  'href="/wiki/mines" data-slug="mines" class="wikilink">The Mines</a></li><li><a id="base-ui-' \
  'href="/wiki/snake-pit" data-slug="snake-pit" class="wikilink">Snake pit</a></li><li><a id="base-ui-' \
  'href="/wiki/tutorial" data-slug="tutorial" class="wikilink">Tutorial</a></li>' \
  '<div class="wiki-page"><nav class="toc-rail" id="toc-rail" aria-labelledby="toc-rail-title"><p class="toc-title" id="toc-rail-title">Contents</p><ol><li><a href="#behaviour">Behaviour</a></li>' \
  '<li><a href="#what-links-here">What links here</a></li></ol></nav><article>' \
  '<nav class="toc" id="toc" aria-labelledby="toc-title" data-closed=""><p class="toc-title" id="toc-title">Contents</p><button type="button" tabindex="0" aria-expanded="false" class="toc-toggle">Contents</button><div data-closed="" hidden=""' \
  'class="toc-panel"><ol><li><a href="#behaviour">Behaviour</a></li>' \
  '<li><a href="#faq">FAQ</a></li><li><a href="#see-also">See also</a></li><li><a href="#references">References</a></li><li><a href="#what-links-here">What links here</a></li></ol></div></nav>' \
  '<ol class="references"><li id="cite-1">' \
  '<div class="heading heading-2"><h2 id="behaviour">' \
  '<nav class="navbox" aria-labelledby="navbox-monsters"><p class="navbox-title" id="navbox-monsters"><a href="/wiki/monsters">Monsters</a></p><dl><div><dt>The Mines</dt><dd><ul><li>' \
  '<li><strong aria-current="page">Snake</strong></li>' \
  '<data value="20-50" data-cite="objects/oSnake/step.gml:69">20–50 steps</data>' \
  '<sup class="cite"><a href="#cite-5">[5]</a></sup>' '<li id="cite-5"><code>objects/oSnake/step.gml:41-54</code></li>' \
  "<source media=\"(prefers-reduced-motion: reduce)\" srcSet=\"$media_url/sprites/sSnakeWalkL/frames/0%404x.png\"/>" \
  "<img class=\"sprite\" src=\"$media_url/sprites/sSnakeWalkL/anim%404x.png\" width=\"128\" height=\"128\"" \
  "poster=\"$media_url/clips/snake/bite/poster.png\"" \
  "<source src=\"$media_url/clips/snake/bite/clip.webm\" type=\"video/webm\"/>" \
  "<source src=\"$media_url/clips/snake/bite/clip.mp4\" type=\"video/mp4\"/>" \
  '</video><div class="clip-live" data-phones=""><button type="button" tabindex="0" aria-disabled="false" class="clip-button">Play live</button>' \
  '<figure class="clip" id="clip-snake-stomp"><div class="clip-missing"' \
  "$audio_credit" \
  'Spelunky Classic 1.1&#x27;s source, © 2008, 2009 Derek Yu and Mossmouth, LLC'
# The bite's state at its first frame reaches its player, which holds the game to it: a prop, so in the page's
# JS, not its HTML.
grep -rqF 5ba7e1e5ba7e1e5b dist/client/assets || fail "the Snake bite's state_hash isn't in the page's JS"
grep -rqF 5ba7e1e5ba7e5ba7 dist/client/assets || fail "the Snake bite's states_hash isn't in the page's JS"
# The bite has sound, so it isn't muted: pressing play plays it. The rail marks no section current before the
# browser has scrolled (the breadcrumbs' and navbox's aria-current="page" aren't one). And the page doesn't say
# which release it was verified against: that's bookkeeping, its JSON-LD's dateModified (docs/wiki-style.md,
# `verified`).
lacks 'muted=""' 'aria-current="location"' 'Verified against the spelunky.ai port' 'class="verified"'
# Its FAQ, shown and as an FAQPage (test-seo.mjs checks they agree), and its Markdown twin, named in its head.
contains '<h2 id="faq">FAQ</h2>' '<dl class="faq"><div><dt>How much damage does a Snake do in Spelunky Classic?</dt>' \
  '{"@context":"https://schema.org","@type":"FAQPage","mainEntity":[{"@type":"Question","name":"How much damage does a Snake do in Spelunky Classic?","acceptedAnswer":{"@type":"Answer","text":"1 heart per bite.' \
  '<link rel="alternate" type="text/markdown" href="https://spelunky.ai/wiki/snake.md"/>'
# Its links and relations (src/wiki/build/relations.ts): links to the pages it links, none of them a red link
# now that every planned page is written, Found in in the infobox, Damaged by under See also with its
# citations, and What links here.
contains 'href="/wiki/tutorial" data-slug="tutorial" class="wikilink">Tutorial</a>' \
  '<div class="heading heading-2"><h2 id="see-also">See also</h2>' '<dt>Damaged by</dt>' \
  'href="/wiki/stomping" data-slug="stomping" class="wikilink">Stomping</a><sup class="cite"><a href="#cite-' \
  '<h2 id="what-links-here">What links here</h2>' '<ul class="links-here"><li><a id="base-ui-' 'data-slug="bat" class="wikilink">Bat</a></li>'
lacks 'class="wikilink-new"'
# Its header's search box (src/wiki/search-box.tsx): a form for /wiki?q= without scripts, Base UI's combobox
# with them, whose list is in a chunk of its own (below), and its status, there from the start.
contains '<form role="search" class="search-box" action="/wiki" method="get"><label id="' 'class="visually-hidden">Search the wiki</label>' \
  'role="combobox" aria-expanded="false" aria-haspopup="listbox" aria-autocomplete="list"' \
  'type="search" placeholder="Search the wiki" name="q" value=""/><button type="submit">Search</button><div role="status" aria-live="polite"'
check "$site/wiki/snake/" 307 "location: /wiki/snake"
# The Markdown twin (src/wiki/build/markdown.ts): a static file, revalidated like the page, as Markdown, naming
# the page as its canonical URL. (https://spelunky.ai/wiki/snake: wrangler dev stands in for spelunky.ai, its
# route, and puts its own URL for it in headers, though not in bodies.) It starts with the notice, and has
# the audio credit, as the bite has sound.
twin_link="link: <$site/wiki/snake>; rel=\"canonical\""
check "$site/wiki/snake.md" 200 "$no_cache" "content-type: text/markdown; charset=utf-8" "$twin_link"
starts "$notice"
contains "$audio_credit" '# Snake' \
  'The Snake is a monster in Spelunky Classic, found mostly in the Mines, that slithers back and forth along the ground and bites for 1 heart on contact.' \
  '## Behaviour' '## FAQ' '### How much damage does a Snake do in Spelunky Classic?' '[^1]: `objects/oSnake/create.gml:10`' \
  ']: [`objects/oSnake/step.gml:3-4`](https://spelunky.ai/wiki/source/objects/oSnake/step#L3-L4)' \
  "**Clip: Snake bite.** A snake slithers up to the player and bites him, taking a heart. ([video](https://spelunky.ai$media_url/clips/snake/bite/clip.mp4))" \
  "![A green snake with a tan belly, coiled and facing left](https://spelunky.ai$media_url/sprites/sSnakeWalkL/anim%404x.png)"
with --head
check "$site/wiki/snake.md" 200 "content-type: text/markdown; charset=utf-8" "$twin_link"
# A revalidation's 304 keeps them.
with "If-None-Match: $(grep -i '^etag:' "$tmp/h" | cut -d' ' -f2-)"
check "$site/wiki/snake.md" 304 "$no_cache" "$twin_link"
# Only a written page has one: not a hub, nor a slug with no page.
check "$site/wiki/monsters.md" 404 "$no_cache" "link: "
check "$site/wiki/no-such-page.md" 404 "$no_cache"
# The index of the twins (https://llmstxt.org), and every page in one file.
check "$site/llms.txt" 200 "$no_cache" "content-type: text/plain; charset=utf-8"
starts "$notice"
contains '# spelunky.ai: the Spelunky Classic wiki' \
  '- [Snake](https://spelunky.ai/wiki/snake.md): The Snake is a monster in Spelunky Classic, found mostly in the Mines'
lacks "$audio_credit"
check "$site/llms-full.txt" 200 "$no_cache" "content-type: text/plain; charset=utf-8"
starts "$notice"
contains "$audio_credit" '# Snake' '| 1 hit point[^snake-1]'
# The hubs (src/wiki/hub.tsx): /wiki, with the search box to come, the featured pages, the categories and how
# the wiki is made; and a category's, a grid of the pages written (links, with their sprites) and of those
# wiki-data knows of that aren't yet (dimmed, not links), with its filters, breadcrumbs and ItemList. One
# with no page written is noindex, and the sitemap leaves it out (test-seo.mjs checks both ways).
check "$site/wiki" 200 "$no_cache" "content-type: text/html; charset=utf-8"
contains '<title>Spelunky Classic wiki</title>' '<h1>Spelunky Classic wiki</h1>' \
  '<meta name="description" content="An unofficial fan wiki for Spelunky Classic 1.1:' \
  '<section class="wiki-search"' 'placeholder="Snake, shotgun, the Mines…"' \
  '<h2 id="featured">Featured pages</h2>' \
  '<a class="category-link" href="/wiki/monsters">' '<a href="/wiki/items/weapons">Weapons</a>' \
  '<h2 id="how">How this wiki is made</h2>' \
  '"@type":"ItemList","name":"Spelunky Classic wiki categories"' \
  '{"@type":"ListItem","position":1,"name":"Monsters","url":"https://spelunky.ai/wiki/monsters"}'
# No count of how much of the wiki is written: it's an encyclopedia, not a progress tracker.
lacks 'class="hub-count"' 'class="category-count"' ' written of the ' ' written</span>'
# A featured card whole: its page's link, its infobox sprite's first frame and its title. Which page it is
# (the most recently verified, then by title) changes with the content.
featured_card="<li class=\"card\"><a class=\"card-body\" href=\"/wiki/[a-z0-9-]+\"><span class=\"card-image\"><img src=\"$media_url/sprites/s[A-Za-z0-9_]+/frames/0%404x\\.png\" width=\"64\" height=\"64\" alt=\"\" loading=\"lazy\"/></span><span class=\"card-title\">[^<]+</span>"
grep -qE -- "$featured_card" "$tmp/body" || fail "$url: no featured card like '$featured_card'"
# No clip, so no audio credit.
lacks "$audio_credit"
cp "$tmp/body" "$tmp/wiki.html"
# The WebSite's SearchAction, /wiki?q=: the same page, whose script shows the results.
check "$site/wiki?q=snake" 200 "$no_cache" "content-type: text/html; charset=utf-8"
same "$tmp/wiki.html"
# The search index: its own content-hashed chunk, which the search box imports when it's first used, so no
# page loads it up front, and the Worker's bundle hasn't got it (test-wiki.mjs checks what it finds).
index=$(cd dist/client && grep -l '"postings":\[' assets/*.js || true)
if [ $(($(echo "$index" | wc -w))) != 1 ]; then
  fail "dist/client/assets: the search index isn't in one chunk: $index"
else
  check "$site/$index" 200 "$immutable"
  contains '{"docs":[{"slug":' '{"slug":"snake","title":"Snake","category":"Monsters"'
  if grep -q "${index#assets/}" dist/client/wiki/snake.html dist/client/wiki.html; then fail "a page loads $index up front"; fi
  if grep -rq '"postings":\[' dist/server; then fail "dist/server: the Worker has the search index"; fi
fi
# The wiki's CSS (src/wiki/stylesheet.tsx): one content-hashed stylesheet, which every wiki page's head links
# and the landing page's doesn't (test-bundle.mjs checks no page's JS has it).
css=$(sed -n 's|.*<link rel="stylesheet" href="/\(assets/wiki-[^"]*\.css\)"/>.*|\1|p' dist/client/wiki.html)
if [ $(($(echo "$css" | wc -w))) != 1 ]; then
  fail "/wiki: doesn't link one stylesheet of the wiki's CSS: $css"
else
  check "$site/$css" 200 "$immutable" "content-type: text/css; charset=utf-8"
  contains '.wiki-header{'
  for page in wiki wiki/snake wiki/monsters wiki/items/weapons; do
    grep -qF "<link rel=\"stylesheet\" href=\"/$css\"/>" "dist/client/$page.html" || fail "/$page doesn't link $css"
  done
  if grep -qF "$css" dist/client/index.html; then fail "/ links $css"; fi
fi
# The hover cards (src/wiki/hover-card.tsx): every page's in a content-hashed chunk of their own, which the
# browser imports the first time a link is hovered or focused, so no page loads it up front and the Worker
# hasn't got it; at most 700 bytes a page, so it stays small as pages are written (test-wiki.mjs checks what
# they say).
cards=$(cd dist/client && grep -l '"kind":"Monster"' assets/*.js || true)
if [ $(($(echo "$cards" | wc -w))) != 1 ]; then
  fail "dist/client/assets: the hover cards aren't in one chunk: $cards"
else
  check "$site/$cards" 200 "$immutable"
  contains '"snake":{"title":"Snake","kind":"Monster","lede":"The Snake is a monster in Spelunky Classic' \
    "\"image\":{\"src\":\"$media_url/sprites/sSnakeWalkL/anim%404x.png\",\"still\":\"$media_url/sprites/sSnakeWalkL/frames/0%404x.png\",\"width\":64,\"height\":64}" \
    '"stats":[["Health","1 hit point"],["Damage","1 heart"],["Speed","1 pixel per step"]]'
  if grep -q "${cards#assets/}" dist/client/wiki/snake.html dist/client/wiki.html; then fail "a page loads $cards up front"; fi
  if grep -rq '"kind":"Monster"' dist/server; then fail "dist/server: the Worker has the hover cards"; fi
  pages=$(($(ls content/wiki/*.mdx | wc -l)))
  size=$(($(wc -c <"dist/client/$cards")))
  [ "$size" -le $((700 * pages)) ] || fail "$cards: $size bytes for $pages page(s), over 700 a page"
fi
# The popups' code (Base UI's positioners and popups): the hover card's (src/wiki/hover-card-popup.tsx) and
# the header search box's list (src/wiki/search-popup.tsx), each in a chunk of its own that the browser loads
# with the cards or the index, so no page loads it up front. (Each is the one with its positioner's class name,
# whatever quotes the minifier gives it.)
for popup in hover-card-positioner search-positioner; do
  chunk=$(cd dist/client && grep -lE "[\"'\`]$popup[\"'\`]" assets/*.js || true)
  if [ $(($(echo "$chunk" | wc -w))) != 1 ]; then
    fail "dist/client/assets: $popup isn't in one chunk: $chunk"
  elif grep -q "${chunk#assets/}" dist/client/wiki/snake.html dist/client/wiki.html dist/client/wiki/monsters.html; then
    fail "a page loads $chunk up front"
  else
    echo "ok   $popup: $chunk, loaded when it's first used"
  fi
done
# The media index (virtual:wiki-media, src/seo/media-package.ts) is the Worker's alone, for the media
# sitemaps: a page's JSON-LD has its own media, made at build time (compile.ts's mediaJsonLd), so no script
# the browser gets has the index. Its entries name their files relative to the media, which nothing else
# does (a page links them by URL), whatever quotes the minifier picks (in the browser's scripts, template
# literals).
media_index='mp4.?: ?.snake/bite/clip\.mp4|image.?: ?.sSnakeWalkL/frames/0@4x\.png'
grep -rqE "$media_index" dist/server || fail "dist/server: no media index, so the check below finds nothing"
in_client=$(grep -rlE "$media_index" dist/client --include='*.js' || true)
[ -z "$in_client" ] || fail "dist/client: the browser's scripts have the media index: $in_client"
check "$site/wiki/monsters" 200 "$no_cache" "content-type: text/html; charset=utf-8"
contains '<title>Monsters – Spelunky Classic wiki</title>' \
  '<link rel="canonical" href="https://spelunky.ai/wiki/monsters"/>' \
  '<nav class="breadcrumbs" aria-label="Breadcrumb"><ol><li><a href="/wiki">Wiki</a></li><li><span aria-current="page">Monsters</span></li></ol></nav>' \
  '"@type":"ItemList","name":"Monsters in the Spelunky Classic wiki","numberOfItems":' \
  '"name":"Snake","url":"https://spelunky.ai/wiki/snake"}' \
  '<a class="card-body" href="/wiki/snake">' \
  "<li class=\"card\"><a class=\"card-body\" href=\"/wiki/bat\"><span class=\"card-image\"><img src=\"$media_url/sprites/sBatLeft/frames/0%404x.png\" width=\"64\" height=\"64\" alt=\"\" loading=\"lazy\"/></span><span class=\"card-title\">Bat</span>" \
  '<div class="hub-query"><label id="' '>Filter by name</label><input id="' 'type="search" autoComplete="off" value=""/>' \
  'role="group" aria-label="Found in" class="hub-choices">' 'aria-pressed="true">All</button>' 'aria-pressed="false">The Mines</button>'
lacks 'class="hub-count"' ' to come.</p>'
check "$site/wiki/items" 200 "$no_cache"
contains '<title>Items – Spelunky Classic wiki</title>' 'role="group" aria-label="Kind" class="hub-choices">' \
  '<li><a href="/wiki/items/equipment">Equipment</a></li>'
# The quirks hub filters by what its quirks involve, and its cards have their ledes; a quirk page's box links
# its docs/quirks.md section on GitHub, its rows there and its replay, at the commit built (build.sh's
# WIKI_COMMIT), and its References cite its row and the replay its clip plays. An item page's citation of a §1
# row no quirk page names links the row and then the section, and its replays link to GitHub.
check "$site/wiki/quirks" 200 "$no_cache"
contains '<title>Quirks and glitches – Spelunky Classic wiki</title>' 'role="group" aria-label="Involves" class="hub-choices">' \
  'aria-pressed="false">Items</button>' '<a class="card-body" href="/wiki/double-jar-roll">' \
  '<span class="card-lede">The double Jar roll is a quirk of Spelunky Classic'
check "$site/wiki/double-jar-roll" 200 "$no_cache"
# Its links to the repo's files on GitHub, each at the last commit to change the file (src/wiki/build/repo.ts), not
# the one built, which would be in every page's code and rename its chunk at every build (851-3740).
pinned() { echo "https://github.com/851-labs/spelunky/blob/$(git log -1 --first-parent --format=%H HEAD -- "../$1")/$1"; }
quirks=$(pinned docs/quirks.md)
jar_replay=$(pinned fidelity/replays/item-jar-whip-rolls-twice.txt)
held_jar_replay=$(pinned fidelity/replays/item-held-jar-shot.txt)
jar_row=$(grep -n '^| \*\*A whipped Jar rolls its contents twice\.' ../docs/quirks.md | cut -d: -f1)
[ -n "$jar_row" ] || fail "docs/quirks.md has no row starting \"A whipped Jar rolls its contents twice.\""
section1="$quirks#1-bugs-and-leftovers-in-the-11-source"
contains '<p class="infobox-title" id="infobox-title">Quirk</p>' \
  "<a href=\"$section1\">docs/quirks.md §1, Bugs and leftovers in the 1.1 source</a>" \
  "<a class=\"infobox-row\" href=\"$quirks?plain=1#L$jar_row\"><q>A whipped Jar rolls its contents twice.</q></a>" \
  "<th scope=\"row\">Pinned by</th><td><a href=\"$jar_replay\"><code>fidelity/replays/item-jar-whip-rolls-twice.txt</code></a></td>" \
  '<th scope="row">Where</th><td><a href="/wiki/source/objects/oJar/collision_oWhip#L1-L21"><code>objects/oJar/collision_oWhip.gml:1-21</code></a></td>' \
  "<a href=\"$quirks?plain=1#L$jar_row\"><q>A whipped Jar rolls its contents twice.</q></a>, <a href=\"$section1\">docs/quirks.md §1</a>" \
  "Replay <a href=\"$jar_replay\"><code>fidelity/replays/item-jar-whip-rolls-twice.txt</code></a>, which the clip <a href=\"#clip-quirks-jar-whip-rolls-twice\">"
lacks 'Quirk 1 (docs/quirks.md)'
leftovers_row=$(grep -n '^| Jar and Skull leftovers' ../docs/quirks.md | cut -d: -f1)
[ -n "$leftovers_row" ] || fail "docs/quirks.md has no row starting \"Jar and Skull leftovers\""
check "$site/wiki/jar" 200 "$no_cache"
contains "<a href=\"$quirks?plain=1#L$leftovers_row\"><q>Jar and Skull leftovers</q></a>, <a href=\"$section1\">docs/quirks.md §1</a>" \
  "Replay <a href=\"$held_jar_replay\"><code>fidelity/replays/item-held-jar-shot.txt</code></a>"
lacks 'Quirk 1 (docs/quirks.md)'
check "$site/wiki/items/weapons" 200 "$no_cache" "content-type: text/html; charset=utf-8"
contains '<title>Weapons – Spelunky Classic wiki</title>' \
  '<link rel="canonical" href="https://spelunky.ai/wiki/items/weapons"/>' \
  '<ol><li><a href="/wiki">Wiki</a></li><li><a href="/wiki/items">Items</a></li><li><span aria-current="page">Weapons</span></li></ol>' \
  '<a class="card-body" href="/wiki/whip">'
check "$site/wiki/items/weapons/" 307 "location: /wiki/items/weapons"
check "$site/wiki/items/carryables" 200 "$no_cache"
contains '<a class="card-body" href="/wiki/fish-bone">' '<a class="card-body" href="/wiki/rock">'
lacks '<span class="card-status">Not written yet</span>' '<meta name="robots" content="noindex"/>'
check "$site/wiki/treasure" 200 "$no_cache"
contains '<a class="card-body" href="/wiki/diamond">' '<a class="card-body" href="/wiki/golden-idol">'
lacks '<span class="card-status">Not written yet</span>' '<meta name="robots" content="noindex"/>'
check "$site/wiki/traps" 200 "$no_cache"
contains '<a class="card-body" href="/wiki/spikes">'
lacks '<meta name="robots" content="noindex"/>'
# The terrain hub: every page written, none to come.
check "$site/wiki/terrain" 200 "$no_cache"
contains '<a class="card-body" href="/wiki/ladder">' '<a class="card-body" href="/wiki/shop-sign">'
lacks '<span class="card-status">Not written yet</span>' '<meta name="robots" content="noindex"/>'
check "$site/wiki/places" 200 "$no_cache"
contains '<a class="card-body" href="/wiki/tutorial">' \
  '<a class="card-body" href="/wiki/mines">' '<a class="card-body" href="/wiki/kalis-altar">' \
  'role="group" aria-label="Found in" class="hub-choices">' 'aria-pressed="false">The Ice Caves</button>'
lacks '<span class="card-status">Not written yet</span>' '<meta name="robots" content="noindex"/>'
# A place page's navbox groups the places by area, as the hub filters them, then the areas' own pages, as
# Areas, and last the one part of no area, the Tutorial, as Other.
check "$site/wiki/snake-pit" 200 "$no_cache"
contains '<nav class="navbox" aria-labelledby="navbox-places"><p class="navbox-title" id="navbox-places"><a href="/wiki/places">Places</a></p><dl><div><dt>The Mines</dt><dd><ul><li>' \
  '<li><strong aria-current="page">Snake pit</strong></li></ul></dd></div><div><dt>The Jungle</dt>' \
  '<div><dt>Areas</dt><dd><ul><li><a id="base-ui-' 'href="/wiki/mines" data-slug="mines">The Mines</a></li><li><a id="base-ui-' \
  '<div><dt>Other</dt><dd><ul><li><a id="base-ui-' 'href="/wiki/tutorial" data-slug="tutorial">Tutorial</a></li></ul></dd></div>'
check "$site/wiki/shops" 200 "$no_cache"
contains '<a class="card-body" href="/wiki/general-store">' '<a class="card-body" href="/wiki/dice-house">'
lacks '<span class="card-status">Not written yet</span>' '<meta name="robots" content="noindex"/>'
check "$site/wiki/characters" 200 "$no_cache"
contains '<a class="card-body" href="/wiki/shopkeeper">' '<a class="card-body" href="/wiki/kali">'
lacks '<span class="card-status">Not written yet</span>' '<meta name="robots" content="noindex"/>'
check "$site/wiki/mechanics" 200 "$no_cache"
contains '<a class="card-body" href="/wiki/stomping">' '<a class="card-body" href="/wiki/level-generation">'
lacks '<span class="card-status">Not written yet</span>' '<meta name="robots" content="noindex"/>'
check "$site/wiki/the-game" 200 "$no_cache"
contains '<a class="card-body" href="/wiki/scoring">' '<a class="card-body" href="/wiki/level-editor">'
lacks '<meta name="robots" content="noindex"/>'
check "$site/wiki/guides" 200 "$no_cache"
contains '<a class="card-body" href="/wiki/beating-olmec">' '<a class="card-body" href="/wiki/speedrunning-basics">'
lacks '<span class="card-status">Not written yet</span>' '<meta name="robots" content="noindex"/>'
check "$site/wiki/items/nothing" 404 "$no_cache"
# A slug with no page: the router renders it, finds none, and the 404 page answers.
check "$site/wiki/no-such-page" 404 "$no_cache" "content-type: text/html; charset=utf-8"
same public/404.html
# An alias (its page's frontmatter's) is a permanent redirect to the page, no-cache as the pages are; with a
# slash, straight to the page; and as a twin, to the page's twin; the query kept (test-links.mjs checks
# every alias). The page has no other URL: its canonical is its own (test-seo.mjs).
check "$site/wiki/pot" 301 "$no_cache" "location: $site/wiki/jar"
check "$site/wiki/jaws/" 301 "$no_cache" "location: $site/wiki/megamouth"
check "$site/wiki/score.md" 301 "$no_cache" "location: $site/wiki/scoring.md"
check "$site/wiki/shopkeeper-spikes-murderer?a=1" 301 "location: $site/wiki/accidental-murder?a=1"
check "$site/sitemap.xml" 200
contains '<loc>https://spelunky.ai/wiki</loc>' '<loc>https://spelunky.ai/wiki/snake</loc>' \
  '<lastmod>2026-10-27</lastmod>' '<loc>https://spelunky.ai/wiki/monsters</loc>'

# The source viewer (src/routes/wiki/source/): a page per file of the media's gml/ tree, at its path without
# .gml, its lines numbered and highlighted at build time, with the source notice above them
# (docs/wiki-license.md) and the script that highlights the lines a link names; noindex, as is the index of
# them all. The .gml files themselves aren't served.
check "$site/wiki/source/objects/oSnake/step" 200 "$no_cache" "content-type: text/html; charset=utf-8"
contains '<title>objects/oSnake/step.gml – GML source – Spelunky Classic wiki</title>' \
  '<meta name="robots" content="noindex"/>' \
  '<link rel="canonical" href="https://spelunky.ai/wiki/source/objects/oSnake/step"/>' \
  '<a href="/wiki/source">Source code</a>' '<h1><code>objects/oSnake/step.gml</code></h1>' \
  '<p class="source-notice">From spelunky.gmk, Spelunky Classic 1.1&#x27;s source, © 2008, 2009 Derek Yu and Mossmouth, LLC, shown whole under the <a href="/LICENSE.txt">Spelunky User License v1.1b</a>' \
  'oSnake&#x27;s step event, 6 lines of GML.' \
  '<a href="/wiki/source/objects/oSnake/create">create.gml</a>' '<strong aria-current="page">step.gml</strong>' \
  '<span class="line" id="L1"><a class="ln" href="#L1" data-n="1" aria-label="Line 1"></a><span><span class="m">// ==== action 1: lib 1 id 603: code ====</span></span></span>' \
  '<span class="k">if</span> (hp &lt; <span class="n">1</span> <span class="k">and</span> x &gt; <span class="n">0</span>) { <span class="f">scrCreateBlood</span>(x+<span class="n">8</span>' \
  '<span class="c"> that ends here */</span> msg = <span class="s">"two</span>' "<span class=\"s\">lines\"</span> + <span class=\"s\">'q'</span>;" \
  '<span class="l">global</span>.snakes += <span class="n">$1F</span>;' \
  'document.getElementById("source-code")' '{background:var(--line-hl)}'
lacks 'id="L7"'
check "$site/wiki/source/scripts/SS_Init" 200 "$no_cache"
contains '<h1><code>scripts/SS_Init.gml</code></h1>' 'This script is part of tsg1zzn&#x27;s Super Sound System.'
check "$site/wiki/source" 200 "$no_cache" "content-type: text/html; charset=utf-8"
contains '<title>GML source – Spelunky Classic wiki</title>' '<meta name="robots" content="noindex"/>' \
  '5 files: 1 script, and the events of 3 objects; and its ' '<a href="/wiki/source/scripts/SS_Init">SS_Init</a>' \
  '<dt id="oSnake">oSnake</dt>' '<a href="/wiki/source/objects/oSnake/step">step</a>' \
  '<h2 id="rooms">Rooms</h2>' '<a href="/wiki/source/rooms/rTutorial">rTutorial</a>'
# A room's page, from the media's rooms/: its settings, its instances by object, by name, each with its
# positions and a link to its events where gml/ has them and to its wiki page where it has one (with its hover
# card; oDrawnSprite, a parent no page claims, has none), the place page that lists the room, its tiles, and
# its instances' creation code, under the room's notice; noindex. The pages' room citations link to it. The .json files themselves aren't served.
check "$site/wiki/source/rooms/rTutorial" 200 "$no_cache" "content-type: text/html; charset=utf-8"
contains '<title>rTutorial – Room source – Spelunky Classic wiki</title>' '<meta name="robots" content="noindex"/>' \
  '<link rel="canonical" href="https://spelunky.ai/wiki/source/rooms/rTutorial"/>' \
  '<a href="/wiki/source">Source code</a>' '<h1><code>rTutorial</code></h1>' \
  'Derek Yu and Mossmouth, LLC, under the <a href="/LICENSE.txt">Spelunky User License v1.1b</a>. Extracted by spelunky.ai&#x27;s tools' \
  'The room rTutorial as the room editor placed it: 5 instances of 4 objects and 1 tile.' \
  '<p class="source-about">The wiki&#x27;s page on this room: <a id="' '" href="/wiki/tutorial" data-slug="tutorial">Tutorial</a>.</p>' \
  '<th scope="row">Size</th><td>672 × 304 pixels</td>' '<td><div>bgCave, tiled across and down</div></td>' \
  'View 0: 320 × 240 at (0, 0), shown at 640 × 480, following oPlayer1 with borders of 128 and 96' \
  '<tr id="oBrick"><th scope="row"><code>oBrick</code><a id="' '" href="/wiki/block" data-slug="block" class="room-page">Block</a></th><td>2</td><td><ul class="room-positions"><li title="Instance 134002">(0, 0)</li><li title="Instance 134003">(16, 0)</li></ul></td></tr>' \
  '<tr id="oDrawnSprite"><th scope="row" tabindex="0"><code>oDrawnSprite</code></th><td>1</td>' \
  '<tr id="oSnake"><th scope="row"><a href="/wiki/source#oSnake"><code>oSnake</code></a><a id="' \
  '" href="/wiki/snake" data-slug="snake" class="room-page">Snake</a></th><td>1</td>' \
  '<tr id="oGoldBar"><th scope="row"><code>oGoldBar</code><a id="' '" href="/wiki/gold-bar" data-slug="gold-bar" class="room-page">Gold Bar</a></th><td>1</td>' \
  '<h2 id="tiles">Tiles</h2>' '<td>(16, 96)</td>' \
  '<h3>Instance 134004 of oGoldBar at (96, 112)</h3>' 'value = 2;'
# Its picture at the top, at the most whole screen pixels a game pixel that fit the 960-pixel column (180 at 5),
# its instances boxed only as their object's row is pointed at, so none in the HTML.
contains '<figure class="room-picture"><div class="room-picture-frame" style="width:900px;height:900px"><img src="/wiki/media/rooms/rTutorial.png" width="900" height="900" alt="The room rTutorial as the room editor placed it"/></div><figcaption>' \
  'at 5 times its size. Point at an object below to see where its instances are.</figcaption></figure><h2 id="settings">Settings</h2>'
lacks 'class="room-box"'
lacks 'shown whole under the' 'GML source'
check "$site/wiki/source/rooms/rNowhere" 404 "$no_cache"
check "$site$media_url/rooms/rTutorial.json" 404
# Its picture, from the asset store with the media's day of caching, as the room page draws it.
check "$site$media_url/rooms/rTutorial.png" 200 "$media_cache" "content-type: image/png"
same "$media/rooms/rTutorial.png"
check "$site/wiki/tutorial" 200 "$no_cache"
contains 'Room <a href="/wiki/source/rooms/rTutorial"><code>rTutorial</code></a>, as the room editor placed it'
check "$site/wiki/tutorial.md" 200
contains 'Room [`rTutorial`](https://spelunky.ai/wiki/source/rooms/rTutorial), as the room editor placed it'
check "$site/wiki/source/objects/oSnake/step/" 307 "location: /wiki/source/objects/oSnake/step"
check "$site/wiki/source/objects/oSnake/nowhere" 404 "$no_cache"
check "$site/wiki/source/objects/oSnake/step.gml" 404 "$no_cache"
check "$site$media_url/gml/objects/oSnake/step.gml" 404

check "$site/releases/latest.json" 200 "$no_cache" "content-type: application/json"
same "$tmp/latest.json"
# The build's list of its hashed files, which the next deploy reads to carry them (carry-assets.mjs;
# docs/web-deploy.md, "Version skew"): through the Worker, no-cache, as each build has its own. Not Workers
# Builds', which carries, so every one of its own scripts and Open Graph images, dated when it was built, and
# nothing else; each of a kind the next build can carry.
check "$site/build-assets.json" 200 "$no_cache" "content-type: application/json"
node --input-type=module -e '
import { readFileSync } from "node:fs";
import { carriable, ownFiles } from "./carry-assets.mjs";
const list = JSON.parse(readFileSync(process.argv[1], "utf8"));
const own = ownFiles("dist/client");
const listed = Object.keys(list.files);
const age = Date.now() - Date.parse(list.date);
const unknown = own.filter((path) => !carriable(path));
if (unknown.length > 0) {
  console.error(`build-assets.json: ${unknown.join(", ")}: not a kind of file in carry-assets.mjs TYPES, which no build would carry`);
  process.exit(1);
}
if (list.format !== 1 || !(age >= 0 && age < 3600_000) || own.length < 100 || listed.join() !== own.join() || Object.values(list.files).some((d) => d !== list.date)) {
  console.error(`build-assets.json: format ${list.format}, date ${list.date}, ${listed.length} files listed of ${own.length}`);
  process.exit(1);
}
' "$tmp/body" || fail "$site/build-assets.json"
# What a page that fails to load a chunk asks the landing page (src/skew.ts): the build's entry script, the
# same on every page, so the landing page's names the build a wiki page or hub is on.
entries=$(for page in index wiki wiki/snake wiki/monsters; do
  grep -o '<script type="module" async="" src="/assets/[^"]*"' "dist/client/$page.html" || echo "$page: none"
done | sort -u)
[ "$(echo "$entries" | wc -l | tr -d ' ')" = 1 ] && case $entries in *index-*) true ;; *) false ;; esac ||
  fail "the pages' entry scripts aren't one /assets/index-HASH.js: $entries"
# The license, in UTF-8 as it's served (build.sh), as the media's is above.
check "$site/LICENSE.txt" 200 "$no_cache" "content-type: text/plain; charset=utf-8"
same "$tmp/LICENSE.txt"
contains 'Spelunky® User License v1.1b'

check "$site/nowhere" 404 "$no_cache" "content-type: text/html; charset=utf-8"
same public/404.html
# Not a page, so the Worker doesn't ask the router.
check "$site/wp-login.php" 404 "$no_cache"
same public/404.html
# Only GET and HEAD: the asset store answers anything else.
got=$(curl -s -o /dev/null -w '%{http_code}' -X POST "$site/")
[ "$got" = 405 ] || fail "POST $site/: status $got, not 405"

# www redirects to the apex, path and query kept (http here only because wrangler dev is).
check "http://localhost:$www_port/play/?a=1" 301 "location: http://spelunky.ai/play/?a=1"

# Every pre-rendered page's title, description, canonical URL and JSON-LD, and the sitemaps against them.
node test-seo.mjs dist/client "$site" || fail "test-seo.mjs"
# The wiki's pages compile, link and relate, and one that breaks a rule fails the build.
node test-wiki.mjs || fail "test-wiki.mjs"
# The carrying itself, against a stand-in live site.
node test-carry.mjs || fail "test-carry.mjs"
# Every pre-rendered page's links on the site, the wiki's pages reachable from /wiki, none to come that no
# ticket is writing, and every alias a redirect to its page.
node test-links.mjs dist/client "$site" || fail "test-links.mjs"
# Every block's left edge on its column's keyline, from the stylesheet.
node test-keyline.mjs || fail "test-keyline.mjs"
# The content types the Worker sets itself, against an asset store like Cloudflare's rather than wrangler dev's,
# which adds the .txt files' charset that spelunky.ai's doesn't.
node test-server.mjs || fail "test-server.mjs"
# The clips' VideoObjects, the sprites' ImageObjects and the media sitemaps, from the stand-in media.
node test-media.mjs "$media" "$site" "$media_url" || fail "test-media.mjs"
# The landing page's scripts: none of the wiki's code, and within their budget.
node test-bundle.mjs dist/client || fail "test-bundle.mjs"
# Every wiki page's and hub's Open Graph image, served, and named in its head.
node test-og.mjs check dist "$site" "$media" "$media_url" || fail "test-og.mjs"

if [ $failures -gt 0 ]; then
  echo "site/test.sh: $failures failed" >&2
  exit 1
fi
echo "site/test.sh: all passed"
