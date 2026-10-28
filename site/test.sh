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
# And for scripts/wiki-media-package's: the indexes, the notices, the pages' sprites and the Snake's clip, with
# what of docs/wiki-media.md's fields src/wiki/build/compile.ts and src/seo/media-package.ts read
# (test-media.mjs checks what the latter makes of them), and the sprites the Open Graph images draw
# (src/wiki/build/og.ts: the fonts, the title and the Spelunker, which test-og.mjs makes). The page's other
# clips aren't there, as a clip a release hasn't rendered yet isn't.
media=$tmp/wiki-media
mkdir -p "$media/sprites/sBlank" "$media/clips/snake/bite"
# Every written page's infobox sprite (a page whose sprite the media haven't got fails the build), each a
# 16-pixel stand-in whose first frame test-og.mjs draws, and a blank one.
sprites=$(sed -n 's/^  image: { sprite: \([A-Za-z0-9_]*\).*/\1/p' content/wiki/*.mdx | sort -u)
entries=
for s in $sprites; do
  entries="$entries{\"name\":\"$s\",\"meta\":\"$s/sprite.json\",\"frames\":4,\"width\":16,\"height\":16},"
  mkdir -p "$media/sprites/$s/frames"
  echo "{\"frames\":[{\"file\":\"$s/frames/0.png\",\"file_scaled\":\"$s/frames/0@4x.png\"}],\"anim_scaled\":\"$s/anim@4x.png\"}" \
    >"$media/sprites/$s/sprite.json"
  echo 'stand-in 0@4x.png' >"$media/sprites/$s/frames/0@4x.png"
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
 "width":640,"height":480,"has_audio":true,
 "files":{"mp4":"snake/bite/clip.mp4","webm":"snake/bite/clip.webm","poster":"snake/bite/poster.png"}}]}
EOF2
echo 'stand-in LICENSE.txt' >"$media/LICENSE.txt"
echo 'stand-in NOTICE.txt' >"$media/NOTICE.txt"
echo 'stand-in clip.mp4' >"$media/clips/snake/bite/clip.mp4"
echo 'stand-in clip.webm' >"$media/clips/snake/bite/clip.webm"
echo 'stand-in poster.png' >"$media/clips/snake/bite/poster.png"
node test-og.mjs stand-in "$media"
tar -czf "$tmp/wiki-media.tar.gz" -C "$media" .
# Served under the first 16 hex digits of its SHA-256 (site/wiki-media.sh).
if command -v sha256sum >/dev/null; then sha=sha256sum; else sha="shasum -a 256"; fi
media_url=/wiki/media/$($sha "$tmp/wiki-media.tar.gz" | cut -c1-16)
./build.sh --local "$tmp/web.tar.gz" --manifest "$tmp/latest.json" --wiki-media "$tmp/wiki-media.tar.gz"

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
  '<video:duration>3</video:duration>' '<video:publication_date>'
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

# The wiki's media: named by the package's hash, so served without the Worker (but the clips' videos, below),
# and cached for good.
check "$site$media_url/sprites/sSnakeWalkL/frames/0%404x.png" 200 "$immutable" "content-type: image/png" \
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
check "$site$clip" 200 "$immutable" "content-type: video/mp4" "accept-ranges: bytes" "content-length: $size" \
  "x-content-type-options: nosniff"
same "$media/clips/snake/bite/clip.mp4"
etag=$(grep -i '^etag:' "$tmp/h" | cut -d' ' -f2-)
with "Range: bytes=0-7"
check "$site$clip" 206 "content-range: bytes 0-7/$size" "content-length: 8" "$immutable" "content-type: video/mp4" \
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
  check "$site$clip" 200 "content-length: $size" "accept-ranges: bytes" "content-range: " "$immutable"
  same "$media/clips/snake/bite/clip.mp4"
done
with "Range: bytes=0-7" 'If-Range: "stale"'
check "$site$clip" 200 "content-length: $size"
with "Range: bytes=0-7" "If-Range: $etag"
check "$site$clip" 206 "content-range: bytes 0-7/$size"
# A HEAD has the size the asset store's leaves out.
with --head
check "$site$clip" 200 "content-length: $size" "accept-ranges: bytes" "$immutable"
with --head "Range: bytes=0-7"
check "$site$clip" 206 "content-length: 8" "content-range: bytes 0-7/$size"
with "If-None-Match: $etag"
check "$site$clip" 304 "etag: $etag"
check "$site$media_url/clips/snake/bite/clip.webm" 200 "$immutable" "content-type: video/webm" "accept-ranges: bytes"
same "$media/clips/snake/bite/clip.webm"
with "Range: bytes=0-7"
check "$site$media_url/clips/snake/bite/clip.webm" 206 "content-range: bytes 0-7/$(wc -c <"$media/clips/snake/bite/clip.webm" | tr -d ' ')" \
  "content-type: video/webm" "$immutable"
# Another package's clip is the asset store's 404 (its immutable caching too, as for its sprites below).
check "$site/wiki/media/fedcba9876543210/clips/snake/bite/clip.mp4" 404 "$immutable"
# The rest of the media skip the Worker, so a Range gets the whole file, with no Accept-Ranges.
for file in sprites/sSnakeWalkL/frames/0%404x.png clips/snake/bite/poster.png clips/index.json NOTICE.txt; do
  with "Range: bytes=0-1"
  check "$site$media_url/$file" 200 "$immutable" "accept-ranges: " "content-range: "
  same "$media/$(echo "$file" | sed 's/%40/@/g')"
done
check "$site$media_url/sprites/index.json" 200 "$immutable" "content-type: application/json"
check "$site$media_url/NOTICE.txt" 200 "$immutable" "content-type: text/plain; charset=utf-8"
# The pin's date (the clips' uploadDate), which build.sh only reads building the pinned media, not the
# stand-in: a bad one would fail only the deploy. As site/wiki-media.sh checks it.
pin_date=$(awk '$1 == "date" { print $2 }' wiki-media.txt)
grep -q '^tag none$' wiki-media.txt || echo "$pin_date" |
  grep -Eqx '[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z' ||
  fail "wiki-media.txt: date '$pin_date', not YYYY-MM-DDTHH:MM:SSZ"
# A miss, another package's say, skips the Worker as /play/v/'s do.
check "$site/wiki/media/fedcba9876543210/sprites/index.json" 404 "$immutable"
# Nor is the directory a page.
check "$site/wiki/media" 404 "$no_cache"

# The wiki (src/routes/wiki/): the Snake's page, pre-rendered at a flat URL, with its numbers, its notices
# (docs/wiki-license.md), its clip and sprite from the media, and a placeholder for a clip they haven't got;
# and its layout (src/wiki/layout.tsx): the breadcrumbs its JSON-LD has too (through its category's hub), the
# verified line, the infobox with its citations and relations, the table of contents (the wide screen's rail
# beside the article, none of it marked current until the browser's scroll-spy does, and the body's
# disclosure, closed, its panel in the page), the sections' anchors and the navbox, its title linking the hub.
check "$site/wiki/snake" 200 "$no_cache" "content-type: text/html; charset=utf-8"
contains '<title>Snake – Spelunky Classic fan wiki</title>' \
  '<link rel="canonical" href="https://spelunky.ai/wiki/snake"/>' \
  '<meta name="description" content="The Snake is a monster in Spelunky Classic, found mostly in the Mines, that slithers back and forth along the ground and bites for 1 heart on contact."/>' \
  '<meta property="og:type" content="article"/>' '<meta property="og:image" content="https://spelunky.ai/wiki/og/snake.' \
  '<meta property="og:image:width" content="1200"/>' '<meta property="og:image:height" content="630"/>' \
  '"@type":"Article"' '"@type":"BreadcrumbList"' \
  "{\"@context\":\"https://schema.org\",\"@type\":\"VideoObject\",\"creditText\":\"Derek Yu and Mossmouth, LLC (Spelunky Classic); music by George Buzinkai and Jonathan Perry\"" \
  "\"name\":\"Snake bite\"" "\"thumbnailUrl\":\"https://spelunky.ai$media_url/clips/snake/bite/poster.png\"" \
  "\"contentUrl\":\"https://spelunky.ai$media_url/clips/snake/bite/clip.mp4\"" '"duration":"PT3S","width":640,"height":480' \
  "{\"@context\":\"https://schema.org\",\"@type\":\"ImageObject\"" \
  "\"contentUrl\":\"https://spelunky.ai$media_url/sprites/sSnakeWalkL/anim%404x.png\"" \
  '"caption":"A green snake with a tan belly, coiled and facing left"' \
  'spelunky.ai is an unofficial fan project, not affiliated with or endorsed by Mossmouth.' \
  '<h1>Snake</h1>' '<h2 id="behaviour">Behaviour</h2>' \
  '<a class="anchor" href="#behaviour" aria-label="Link to this section" aria-describedby="behaviour">#</a>' \
  '<nav class="breadcrumbs" aria-label="Breadcrumb"><ol><li><a href="/wiki">Wiki</a></li><li><a href="/wiki/monsters">Monsters</a></li><li><span aria-current="page">Snake</span></li></ol></nav>' \
  '"@type":"BreadcrumbList","itemListElement":[{"@type":"ListItem","position":1,"name":"Wiki","item":"https://spelunky.ai/wiki"},{"@type":"ListItem","position":2,"name":"Monsters","item":"https://spelunky.ai/wiki/monsters"},{"@type":"ListItem","position":3,"name":"Snake","item":"https://spelunky.ai/wiki/snake"}]' \
  '<a href="https://github.com/851-labs/spelunky/releases/tag/v0.1.7">v0.1.7</a> on<!-- --> <time dateTime="2026-10-27">27 October 2026</time>' \
  '<th scope="row">Health</th><td><data value="1" data-cite="objects/oSnake/create.gml:10">1 hit point</data><sup class="cite"><a href="#cite-1">[1]</a></sup></td>' \
  '<li id="cite-1"><code>objects/oSnake/create.gml:10</code></li>' \
  '<li id="cite-4"><code>objects/oSnake/step.gml:39-72</code></li>' \
  '<th scope="row">Object</th><td><span><code>oSnake</code></span></td>' \
  '<th scope="row">Found in</th><td><ul class="relation-links"><li><a id="base-ui-' \
  'href="/wiki/mines" data-slug="mines" class="wikilink">The Mines</a></li><li><span class="wikilink-new" data-slug="snake-pit" title="This page hasn&#x27;t been written yet">Snake pit</span></li>' \
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
  '<figure class="clip" id="clip-snake-stomp"><div class="clip-missing"' \
  "$audio_credit" \
  'Spelunky Classic 1.1&#x27;s source, © 2008, 2009 Derek Yu and Mossmouth, LLC'
# The bite has sound, so it isn't muted: pressing play plays it. And the rail marks no section current
# before the browser has scrolled (the breadcrumbs' and navbox's aria-current="page" aren't one).
lacks 'muted=""' 'aria-current="location"'
# Its FAQ, shown and as an FAQPage (test-seo.mjs checks they agree), and its Markdown twin, named in its head.
contains '<h2 id="faq">FAQ</h2>' '<dl class="faq"><div><dt>How much damage does a Snake do in Spelunky Classic?</dt>' \
  '{"@context":"https://schema.org","@type":"FAQPage","mainEntity":[{"@type":"Question","name":"How much damage does a Snake do in Spelunky Classic?","acceptedAnswer":{"@type":"Answer","text":"1 heart per bite.' \
  '<link rel="alternate" type="text/markdown" href="https://spelunky.ai/wiki/snake.md"/>'
# Its links and relations (src/wiki/build/relations.ts): red links to the planned pages it links, Found in
# in the infobox, Damaged by under See also with its citations, and What links here.
contains '<span class="wikilink-new" data-slug="snake-pit" title="This page hasn&#x27;t been written yet">snake pits</span>' \
  '<div class="heading heading-2"><h2 id="see-also">See also</h2>' '<dt>Damaged by</dt>' \
  'data-slug="stomping" title="This page hasn&#x27;t been written yet">Stomping</span><sup class="cite"><a href="#cite-' \
  '<h2 id="what-links-here">What links here</h2>' '<ul class="links-here"><li><a id="base-ui-' 'data-slug="bat" class="wikilink">Bat</a></li>'
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
contains '# spelunky.ai: the Spelunky Classic fan wiki' \
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
contains '<title>Spelunky Classic fan wiki</title>' \
  '<section class="wiki-search"' 'placeholder="Snake, shotgun, the Mines…"' \
  '<h2 id="featured">Featured pages</h2>' \
  '<a class="category-link" href="/wiki/monsters">' '<a href="/wiki/items/weapons">Weapons</a>' \
  '<h2 id="how">How this wiki is made</h2>' \
  '"@type":"ItemList","name":"Spelunky Classic fan wiki categories"' \
  '{"@type":"ListItem","position":1,"name":"Monsters","url":"https://spelunky.ai/wiki/monsters"}'
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
# the browser gets has the index. Its entries name their files relative to the package, which nothing else
# does (a page links them by URL), whatever quotes the minifier picks (in the browser's scripts, template
# literals).
media_index='mp4.?: ?.snake/bite/clip\.mp4|image.?: ?.sSnakeWalkL/frames/0@4x\.png'
grep -rqE "$media_index" dist/server || fail "dist/server: no media index, so the check below finds nothing"
in_client=$(grep -rlE "$media_index" dist/client --include='*.js' || true)
[ -z "$in_client" ] || fail "dist/client: the browser's scripts have the media index: $in_client"
check "$site/wiki/monsters" 200 "$no_cache" "content-type: text/html; charset=utf-8"
contains '<title>Monsters – Spelunky Classic fan wiki</title>' \
  '<link rel="canonical" href="https://spelunky.ai/wiki/monsters"/>' \
  '<nav class="breadcrumbs" aria-label="Breadcrumb"><ol><li><a href="/wiki">Wiki</a></li><li><span aria-current="page">Monsters</span></li></ol></nav>' \
  '"@type":"ItemList","name":"Monsters in the Spelunky Classic fan wiki","numberOfItems":' \
  '"name":"Snake","url":"https://spelunky.ai/wiki/snake"}' \
  '<a class="card-body" href="/wiki/snake">' \
  "<li class=\"card\"><a class=\"card-body\" href=\"/wiki/bat\"><span class=\"card-image\"><img src=\"$media_url/sprites/sBatLeft/frames/0%404x.png\" width=\"64\" height=\"64\" alt=\"\" loading=\"lazy\"/></span><span class=\"card-title\">Bat</span>" \
  '<div class="hub-query"><label id="' '>Filter by name</label><input id="' 'type="search" autoComplete="off" value=""/>' \
  'role="group" aria-label="Found in" class="hub-choices">' 'aria-pressed="true">All</button>' 'aria-pressed="false">The Mines</button>'
check "$site/wiki/items" 200 "$no_cache"
contains '<title>Items – Spelunky Classic fan wiki</title>' 'role="group" aria-label="Kind" class="hub-choices">' \
  '<li><a href="/wiki/items/equipment">Equipment</a></li>'
check "$site/wiki/items/weapons" 200 "$no_cache" "content-type: text/html; charset=utf-8"
contains '<title>Weapons – Spelunky Classic fan wiki</title>' \
  '<link rel="canonical" href="https://spelunky.ai/wiki/items/weapons"/>' \
  '<ol><li><a href="/wiki">Wiki</a></li><li><a href="/wiki/items">Items</a></li><li><span aria-current="page">Weapons</span></li></ol>' \
  '<a class="card-body" href="/wiki/whip">'
check "$site/wiki/items/weapons/" 307 "location: /wiki/items/weapons"
check "$site/wiki/items/carryables" 200 "$no_cache"
contains '<span class="card-title">Bomb Bag</span><span class="card-status">Not written yet</span>' \
  '<a class="card-body" href="/wiki/rock">'
lacks '<meta name="robots" content="noindex"/>'
check "$site/wiki/treasure" 200 "$no_cache"
contains '<span class="card-title">Diamond</span><span class="card-status">Not written yet</span>' \
  '<a class="card-body" href="/wiki/golden-idol">'
lacks '<meta name="robots" content="noindex"/>'
check "$site/wiki/traps" 200 "$no_cache"
contains '<a class="card-body" href="/wiki/spikes">'
lacks '<meta name="robots" content="noindex"/>'
check "$site/wiki/places" 200 "$no_cache"
contains '<span class="card-title">Black Market</span><span class="card-status">Not written yet</span>' \
  '<a class="card-body" href="/wiki/mines">'
lacks '<meta name="robots" content="noindex"/>'
check "$site/wiki/shops" 200 "$no_cache"
contains '<span class="card-title">General Store</span><span class="card-status">Not written yet</span>' \
  '<meta name="robots" content="noindex"/>'
check "$site/wiki/items/nothing" 404 "$no_cache"
# A slug with no page: the router renders it, finds none, and the 404 page answers.
check "$site/wiki/no-such-page" 404 "$no_cache" "content-type: text/html; charset=utf-8"
same public/404.html
check "$site/sitemap.xml" 200
contains '<loc>https://spelunky.ai/wiki</loc>' '<loc>https://spelunky.ai/wiki/snake</loc>' \
  '<lastmod>2026-10-27</lastmod>' '<loc>https://spelunky.ai/wiki/monsters</loc>'

check "$site/releases/latest.json" 200 "$no_cache" "content-type: application/json"
same "$tmp/latest.json"
check "$site/LICENSE.txt" 200 "$no_cache" "content-type: text/plain; charset=utf-8"
same ../LICENSE

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
# The clips' VideoObjects, the sprites' ImageObjects and the media sitemaps, from the stand-in package.
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
