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
# And for scripts/wiki-media-package's: the indexes, the notices, a sprite and a clip, the Snake page's, with
# what of docs/wiki-media.md's fields src/wiki/build/compile.ts and src/seo/media-package.ts read
# (test-media.mjs checks what the latter makes of them). The page's other clips aren't there, as a clip a
# release hasn't rendered yet isn't.
media=$tmp/wiki-media
mkdir -p "$media/sprites/sSnakeWalkL/frames" "$media/sprites/sBlank" "$media/clips/snake/bite"
cat >"$media/sprites/index.json" <<'EOF2'
{"scale":4,"sprites":[{"name":"sSnakeWalkL","meta":"sSnakeWalkL/sprite.json","frames":4,"width":16,"height":16},
 {"name":"sBlank","meta":"sBlank/sprite.json","frames":1,"width":0,"height":0}],
 "objects":[{"name":"oSnake","sprite":"sSnakeWalkL"},{"name":"oGame","sprite":null}]}
EOF2
echo '{"frames":[{"file_scaled":"sSnakeWalkL/frames/0@4x.png"}],"anim_scaled":"sSnakeWalkL/anim@4x.png"}' \
  >"$media/sprites/sSnakeWalkL/sprite.json"
# A sprite without pixels has no files.
echo '{"frames":[{"file_scaled":null}],"anim_scaled":null}' >"$media/sprites/sBlank/sprite.json"
cat >"$media/clips/index.json" <<'EOF2'
{"caption":"Recorded from the spelunky.ai port v9.9.9, a stand-in.","clips":[{"id":"snake/bite",
 "title":"Snake bite","description":"A snake bites the player & takes a heart.","duration":3.3333333333333335,
 "width":640,"height":480,
 "files":{"mp4":"snake/bite/clip.mp4","webm":"snake/bite/clip.webm","poster":"snake/bite/poster.png"}}]}
EOF2
echo 'stand-in LICENSE.txt' >"$media/LICENSE.txt"
echo 'stand-in NOTICE.txt' >"$media/NOTICE.txt"
echo 'stand-in 0@4x.png' >"$media/sprites/sSnakeWalkL/frames/0@4x.png"
echo 'stand-in anim@4x.png' >"$media/sprites/sSnakeWalkL/anim@4x.png"
echo 'stand-in clip.mp4' >"$media/clips/snake/bite/clip.mp4"
echo 'stand-in clip.webm' >"$media/clips/snake/bite/clip.webm"
echo 'stand-in poster.png' >"$media/clips/snake/bite/poster.png"
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
# Its head (src/seo/): the canonical URL and the JSON-LD; test-seo.mjs, below, checks every page's.
contains '<link rel="canonical" href="https://spelunky.ai/"/>' \
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

# The wiki's media: named by the package's hash, so served without the Worker, and cached for good.
check "$site$media_url/sprites/sSnakeWalkL/frames/0%404x.png" 200 "$immutable" "content-type: image/png" \
  "x-content-type-options: nosniff"
same "$media/sprites/sSnakeWalkL/frames/0@4x.png"
# The asset store's canonical form of a path escapes an @, and it redirects to that, so a page asks for
# 0%404x.png (docs/web-deploy.md, "Wiki media").
check "$site$media_url/sprites/sSnakeWalkL/frames/0@4x.png" 307 \
  "location: $media_url/sprites/sSnakeWalkL/frames/0%404x.png"
check "$site$media_url/clips/snake/bite/clip.mp4" 200 "$immutable" "content-type: video/mp4"
same "$media/clips/snake/bite/clip.mp4"
check "$site$media_url/clips/snake/bite/clip.webm" 200 "$immutable" "content-type: video/webm"
same "$media/clips/snake/bite/clip.webm"
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
# and its layout (src/wiki/layout.tsx): the breadcrumbs its JSON-LD has too, the verified line, the infobox
# with its citations and relations, the table of contents, the sections' anchors and the navbox.
check "$site/wiki/snake" 200 "$no_cache" "content-type: text/html; charset=utf-8"
contains '<title>Snake – Spelunky Classic fan wiki</title>' \
  '<link rel="canonical" href="https://spelunky.ai/wiki/snake"/>' \
  '<meta name="description" content="The Snake is a monster in Spelunky Classic, found mostly in the Mines, that slithers back and forth along the ground and bites for 1 heart on contact."/>' \
  '<meta property="og:type" content="article"/>' \
  '"@type":"Article"' '"@type":"BreadcrumbList"' \
  '<p>Unofficial fan wiki. Spelunky is © 2008, 2009 Derek Yu and Mossmouth, LLC.</p>' \
  'spelunky.ai is an unofficial fan project, not affiliated with or endorsed by Mossmouth.' \
  '<h1>Snake</h1>' '<h2 id="behaviour">Behaviour</h2>' \
  '<a class="anchor" href="#behaviour" aria-label="Link to this section" aria-describedby="behaviour">#</a>' \
  '<nav class="breadcrumbs" aria-label="Breadcrumb"><ol><li><a href="/wiki">Wiki</a></li><li><span aria-current="page">Snake</span></li></ol></nav>' \
  '"@type":"BreadcrumbList","itemListElement":[{"@type":"ListItem","position":1,"name":"Wiki","item":"https://spelunky.ai/wiki"},{"@type":"ListItem","position":2,"name":"Snake","item":"https://spelunky.ai/wiki/snake"}]' \
  '<a href="https://github.com/851-labs/spelunky/releases/tag/v0.1.7">v0.1.7</a> on<!-- --> <time dateTime="2026-10-27">27 October 2026</time>' \
  '<th scope="row">Health</th><td><data value="1" data-cite="objects/oSnake/create.gml:10">1 hit point</data><sup class="cite"><a href="#cite-1">[1]</a></sup></td>' \
  '<li id="cite-1"><code>objects/oSnake/create.gml:10</code></li>' \
  '<li id="cite-4"><code>objects/oSnake/step.gml:39-72</code></li>' \
  '<th scope="row">Object</th><td><span><code>oSnake</code></span></td>' \
  '<th scope="row">Found in</th><td><ul class="infobox-links"><li><span class="wikilink-missing" data-slug="mines">The Mines</span></li>' \
  '<nav class="toc" id="toc" aria-labelledby="toc-title"><p class="toc-title" id="toc-title">Contents</p><ol><li><a href="#behaviour">Behaviour</a></li>' \
  '<li><a href="#faq">FAQ</a></li><li><a href="#references">References</a></li></ol></nav>' \
  '<div class="heading heading-2"><h2 id="behaviour">' \
  '<nav class="navbox" aria-labelledby="navbox-monsters"><p class="navbox-title" id="navbox-monsters">Monsters</p><dl><div><dt>The Mines</dt><dd><ul><li><strong aria-current="page">Snake</strong></li></ul></dd></div></dl></nav>' \
  '<data value="20-50" data-cite="objects/oSnake/step.gml:69">20–50 steps</data>' \
  '<sup class="cite"><a href="#cite-5">[5]</a></sup>' '<li id="cite-5"><code>objects/oSnake/step.gml:41-54</code></li>' \
  "<source media=\"(prefers-reduced-motion: reduce)\" srcSet=\"$media_url/sprites/sSnakeWalkL/frames/0%404x.png\"/>" \
  "<img class=\"sprite\" src=\"$media_url/sprites/sSnakeWalkL/anim%404x.png\" width=\"128\" height=\"128\"" \
  "poster=\"$media_url/clips/snake/bite/poster.png\"" \
  "<source src=\"$media_url/clips/snake/bite/clip.webm\" type=\"video/webm\"/>" \
  "<source src=\"$media_url/clips/snake/bite/clip.mp4\" type=\"video/mp4\"/>" \
  'Recorded from the spelunky.ai port v9.9.9, a stand-in.' \
  '<figure class="clip" id="clip-snake-stomp"><div class="clip-missing"' \
  'Spelunky Classic 1.1&#x27;s source, © 2008, 2009 Derek Yu and Mossmouth, LLC'
check "$site/wiki/snake/" 307 "location: /wiki/snake"
check "$site/wiki" 200 "$no_cache" "content-type: text/html; charset=utf-8"
contains '<title>Spelunky Classic fan wiki</title>' '<a href="/wiki/snake">Snake</a>' \
  '<p>Unofficial fan wiki. Spelunky is © 2008, 2009 Derek Yu and Mossmouth, LLC.</p>'
# A slug with no page: the router renders it, finds none, and the 404 page answers.
check "$site/wiki/no-such-page" 404 "$no_cache" "content-type: text/html; charset=utf-8"
same public/404.html
check "$site/sitemap.xml" 200
contains '<loc>https://spelunky.ai/wiki</loc>' '<loc>https://spelunky.ai/wiki/snake</loc>' \
  '<lastmod>2026-10-27</lastmod>'

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

# Every pre-rendered page's title, description, canonical URL and JSON-LD, and the sitemaps against them.
node test-seo.mjs dist/client "$site" || fail "test-seo.mjs"
# The wiki's pages compile, and one that breaks a rule fails the build.
node test-wiki.mjs || fail "test-wiki.mjs"
# The clips' VideoObjects, the sprites' ImageObjects and the media sitemaps, from the stand-in package.
node test-media.mjs "$media" "$site" "$media_url" || fail "test-media.mjs"

if [ $failures -gt 0 ]; then
  echo "site/test.sh: $failures failed" >&2
  exit 1
fi
echo "site/test.sh: all passed"
