#!/bin/sh
# Builds the site in site/dist: the TanStack Start app (src/, with public/ and its pages pre-rendered) with
# Vite, as dist/server (the Worker) and dist/client (its static assets), and adds to dist/client the web build
# of the release named in web-release.txt, unpacked at /play/, the native apps' update manifest (that
# release's latest.json, docs/updates.md) at /releases/latest.json, and the wiki's media pinned in
# wiki-media.txt at /wiki/media/HASH/ (wiki-media.sh). Workers Builds' build command (docs/web-deploy.md), so
# it needs only Node, curl, tar and a SHA-256 tool, no Rust.
#
#   site/build.sh                 download the release's spelunky-web-<tag>.tar.gz and latest.json and the
#                                 pinned wiki media, and check their SHA-256s
#   site/build.sh --local PATH [--manifest FILE]
#                                 use a local tarball instead (scripts/web --package PATH), checked against
#                                 PATH.sha256 if it's there, and FILE as latest.json
#   site/build.sh ... --wiki-media PATH
#                                 and a local wiki media package (scripts/wiki-media-package), checked
#                                 against PATH.sha256 if it's there
set -eu
die() { echo "site/build.sh: $*" >&2; exit 1; }
usage() { echo "usage: site/build.sh [--local PATH [--manifest FILE]] [--wiki-media PATH]" >&2; exit 2; }
given=
manifest=
wiki_media=
while [ $# -gt 0 ]; do
  [ $# -ge 2 ] || usage
  case $1 in
    --local) given=$2 ;;
    --manifest) manifest=$2 ;;
    --wiki-media) wiki_media=$2 ;;
    *) usage ;;
  esac
  shift 2
done
# Without --local, the manifest is the release's.
[ -z "$manifest" ] || [ -n "$given" ] || usage
# Relative to where it was run, not site/.
absolute() {
  if [ -f "$1" ]; then echo "$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"; else echo "$1"; fi
}
[ -z "$given" ] || given=$(absolute "$given")
if [ -n "$manifest" ]; then
  [ -f "$manifest" ] || die "no $manifest"
  manifest=$(absolute "$manifest")
fi
if [ -n "$wiki_media" ]; then
  [ -f "$wiki_media" ] || die "no $wiki_media"
  wiki_media=$(absolute "$wiki_media")
fi
cd "$(dirname "$0")"

sha256() {
  if command -v sha256sum >/dev/null; then sha256sum "$1"; else shasum -a 256 "$1"; fi | cut -d' ' -f1
}

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
if [ -n "$given" ]; then
  [ -f "$given" ] || die "no $given"
  tarball=$given
  if [ -f "$given.sha256" ]; then
    want=$(cut -d' ' -f1 "$given.sha256")
  else
    want=
  fi
else
  tag=$(awk '$1 == "tag" { print $2 }' web-release.txt)
  want=$(awk '$1 == "sha256" { print $2 }' web-release.txt)
  if [ "$tag" = none ] || [ "$want" = none ]; then
    die "web-release.txt names no release yet. Once a GitHub Release has spelunky-web-<tag>.tar.gz, run scripts/bump-web-release <tag> and merge that."
  fi
  [ -n "$tag" ] && [ ${#want} = 64 ] || die "web-release.txt needs a 'tag TAG' line and a 'sha256 HEX' line"
  url=https://github.com/851-labs/spelunky/releases/download/$tag/spelunky-web-$tag.tar.gz
  echo "site/build.sh: downloading $url"
  tarball=$tmp/web.tar.gz
  curl -fsSL --retry 3 -o "$tarball" "$url" || die "couldn't download $url"
  # The update manifest is signed, which the apps check; the checksum pins it to what the bump verified.
  want_manifest=$(awk '$1 == "manifest-sha256" { print $2 }' web-release.txt)
  if [ -n "$want_manifest" ] && [ "$want_manifest" != none ]; then
    url=https://github.com/851-labs/spelunky/releases/download/$tag/latest.json
    echo "site/build.sh: downloading $url"
    manifest=$tmp/latest.json
    curl -fsSL --retry 3 -o "$manifest" "$url" || die "couldn't download $url"
    got_manifest=$(sha256 "$manifest")
    [ "$got_manifest" = "$want_manifest" ] || die "$url has SHA-256 $got_manifest, not $want_manifest"
  fi
fi
got=$(sha256 "$tarball")
if [ -n "$want" ] && [ "$got" != "$want" ]; then
  die "$tarball has SHA-256 $got, not $want"
fi
# Unpacked before the app's build, which needs to know whether the build can be embedded in a page, as its page
# says (crates/app/web/index.html): then a wiki clip whose replay the media have can play in it
# (src/wiki/live-clip.tsx). A release from before that plays a replay from the boot, without the page's controls.
mkdir "$tmp/play"
tar -xzf "$tarball" -C "$tmp/play"
[ -f "$tmp/play/index.html" ] || die "$tarball has no index.html: not a scripts/web --package build"
if grep -q '<meta name="spelunky-embed"' "$tmp/play/index.html"; then
  export WIKI_LIVE_CLIPS=1
else
  echo "site/build.sh: the web build can't be embedded, so the wiki's clips can't play in it"
fi
# The wiki's media, before the app's build, so that its pages can read the media's indexes as they're
# pre-rendered (src/seo/media-package.ts): WIKI_MEDIA_DIR is the unpacked package (sprites/index.json,
# clips/index.json, and gml/ for the source viewer, src/wiki/build/source.ts), WIKI_MEDIA_URL the path it's served at, /wiki/media/HASH, and WIKI_MEDIA_DATE when it was
# published, the clips' uploadDate: wiki-media.txt's date, or a local package's modification time. None is
# set while wiki-media.txt pins nothing.
mkdir "$tmp/wiki-media"
./wiki-media.sh "$tmp/wiki-media" ${wiki_media:+--local "$wiki_media"}
media_hash=
[ ! -d "$tmp/wiki-media/wiki/media" ] || media_hash=$(ls "$tmp/wiki-media/wiki/media")
if [ -n "$media_hash" ]; then
  if [ -n "$wiki_media" ]; then
    media_date=$(node -p 'fs.statSync(process.argv[1]).mtime.toISOString().replace(/\.\d+Z$/, "Z")' "$wiki_media")
  else
    media_date=$(awk '$1 == "date" { print $2 }' wiki-media.txt)
    [ -n "$media_date" ] || die "wiki-media.txt needs a 'date ISO-8601' line: when its release was published"
  fi
  export WIKI_MEDIA_DIR="$tmp/wiki-media/wiki/media/$media_hash" WIKI_MEDIA_URL="/wiki/media/$media_hash" \
    WIKI_MEDIA_DATE="$media_date"
  # _headers serves the media's .txt files as UTF-8, but a package made before scripts/wiki-media-package
  # transcoded LICENSE (851-3542), which the pin may be until the next one, or again after a rollback, has it in
  # windows-1252: its ® would be a replacement character. So that one is transcoded here, as /LICENSE.txt is.
  node -e 'const b = fs.readFileSync(process.argv[1]);
    try { new TextDecoder("utf-8", { fatal: true }).decode(b); }
    catch { fs.writeFileSync(process.argv[1], new TextDecoder("windows-1252").decode(b)); }' \
    "$WIKI_MEDIA_DIR/LICENSE.txt"
fi

# package-lock.json pins every package: npm ci installs exactly those, or fails.
npm ci --no-audit --no-fund
rm -rf dist
node_modules/.bin/vite build
# Vite only strips the types. After the build, which brings src/routeTree.gen.ts up to date with src/routes/.
node_modules/.bin/tsc -p .
assets=dist/client
[ -f $assets/index.html ] || die "vite build pre-rendered no $assets/index.html"
# The landing page links the license, which a Covered Work has to come with. LICENSE is the original's
# COPYING.txt, so windows-1252 (its ® is the byte AE), and the Worker serves a .txt as UTF-8 (src/server.ts).
node -e 'fs.writeFileSync(process.argv[2], new TextDecoder("windows-1252").decode(fs.readFileSync(process.argv[1])))' \
  ../LICENSE $assets/LICENSE.txt
mv "$tmp/play" $assets/play
if [ -n "$manifest" ]; then
  mkdir $assets/releases
  cp "$manifest" $assets/releases/latest.json
fi
# The router mustn't have a page at /wiki/media or /wiki/og, pinned media or not: everything under them
# skips the Worker and is cached for good (wrangler.jsonc, public/_headers).
for page in $assets/wiki/media $assets/wiki/media.html $assets/wiki/og.html; do
  [ ! -e "$page" ] || die "the app pre-rendered $page, where the wiki's media or Open Graph images go"
done
if [ -n "$media_hash" ]; then
  mkdir -p $assets/wiki
  # The GML is the source viewer's to show, as /wiki/source/ pages the app pre-rendered from it
  # (src/wiki/build/source.ts), not files to serve: another 1,140 assets, each a Worker request (wiki-media.sh).
  rm -rf "$tmp/wiki-media/wiki/media/$media_hash/gml"
  mv "$tmp/wiki-media/wiki/media" $assets/wiki/media
fi
# Workers static assets take files of up to 25 MiB each, and 20,000 of them a version on the free plan
# (docs/web-deploy.md, "Limits"), most of them the wiki's media. Wrangler reads _headers and .assetsignore
# (the Cloudflare plugin's) and uploads neither.
big=$(find $assets -type f -size +25600k)
[ -z "$big" ] || die "over Workers' 25 MiB per-file limit: $big"
files=$(find $assets -type f ! -path $assets/_headers ! -path $assets/.assetsignore | wc -l | tr -d ' ')
[ "$files" -le 20000 ] || die "$files files, over Workers' 20,000 a version (docs/web-deploy.md, \"Limits\")"
echo "site/build.sh: dist/ ready, with $(cd $assets/play && ls v) at /play/ (SHA-256 $got)"
if [ -n "$manifest" ]; then
  echo "site/build.sh: and the update manifest at /releases/latest.json"
else
  echo "site/build.sh: no update manifest: the native apps find no update"
fi
if [ -n "$media_hash" ]; then
  echo "site/build.sh: and the wiki's media at /wiki/media/$media_hash/"
else
  echo "site/build.sh: no wiki media: wiki-media.txt pins none"
fi
echo "site/build.sh: $files static asset files, of the 20,000 Workers takes"
# wrangler dev holds a file descriptor per asset, and macOS's posix_spawn refuses one past OPEN_MAX (10,240),
# so it can't start on this many there (docs/web-deploy.md, "Testing locally"). Deploying is unaffected.
if [ "$files" -gt 9900 ] && [ "$(uname)" = Darwin ]; then
  echo "site/build.sh: too many for wrangler dev on macOS (spawn EBADF): serve it with npx vite preview"
fi
