#!/bin/sh
# Builds the site in site/dist: the TanStack Start app (src/, with public/ and its pages pre-rendered) with
# Vite, as dist/server (the Worker) and dist/client (its static assets), and adds to dist/client the web build
# of the release named in web-release.txt, unpacked at /play/, the native apps' update manifest (that
# release's latest.json, docs/updates.md) at /releases/latest.json, and the wiki's media, committed in
# wiki/media (docs/wiki-media.md), at /wiki/media/. Workers Builds' build command (docs/web-deploy.md), so
# it needs only Node, git, curl, tar and a SHA-256 tool, no Rust.
#
#   site/build.sh                 download the release's spelunky-web-<tag>.tar.gz and latest.json, and
#                                 check their SHA-256s
#   site/build.sh --local PATH [--manifest FILE]
#                                 use a local tarball instead (scripts/web --package PATH), checked against
#                                 PATH.sha256 if it's there, and FILE as latest.json
#   site/build.sh ... --wiki-media DIR
#                                 and other wiki media than wiki/media (site/test.sh's stand-in)
set -eu
die() { echo "site/build.sh: $*" >&2; exit 1; }
usage() { echo "usage: site/build.sh [--local PATH [--manifest FILE]] [--wiki-media DIR]" >&2; exit 2; }
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
  [ -d "$wiki_media" ] || die "no $wiki_media"
  wiki_media=$(cd "$wiki_media" && pwd)
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
  # And whether it seeks: then the clips' player has a slider, and restarts without loading the game again.
  if grep -q '<meta name="spelunky-embed-seek"' "$tmp/play/index.html"; then
    export WIKI_LIVE_SEEK=1
    # And whether it starts its sound at a tap where the browser held it, as Safari does, which every browser
    # on iOS is: then phones get the button too. The player also needs the frames a seeking build reports, to
    # tell when the device can't keep up.
    if grep -q '<meta name="spelunky-embed-held-sound"' "$tmp/play/index.html"; then
      export WIKI_LIVE_PHONES=1
    else
      echo "site/build.sh: the web build can't start its sound at a tap, so phones keep the wiki's clips' videos"
    fi
  else
    echo "site/build.sh: the web build can't seek, so the wiki's live clips have no slider"
  fi
else
  echo "site/build.sh: the web build can't be embedded, so the wiki's clips can't play in it"
fi
# The wiki's media, before the app's build, so that its pages can read the media's indexes as they're
# pre-rendered (src/seo/media-package.ts): WIKI_MEDIA_DIR is where they are (sprites/index.json,
# clips/index.json, and gml/ for the source viewer, src/wiki/build/source.ts), WIKI_MEDIA_URL the path they're
# served at, /wiki/media, WIKI_MEDIA_LOG the file that dates each clip, its uploadDate (git log's list of the
# commits that changed the clips' videos: a clip's is the newest that changed its MP4 or WebM), and
# WIKI_MEDIA_DATE when the media last changed (the last commit to them), the date of a clip the log hasn't got
# (a new one, rendered but not committed). A --wiki-media directory that isn't a git repository of its own has
# no log, and every clip's date is the directory's modification time.
media=${wiki_media:-$(cd .. && pwd)/wiki/media}
for f in sprites/index.json clips/index.json LICENSE.txt NOTICE.txt; do
  [ -f "$media/$f" ] || die "$media has no $f: not what scripts/wiki-media renders"
done
# wrangler.jsonc's run_worker_first keeps the media's sprites and the clips' posters, JSON and replays off the
# Worker, leaving it the clips' videos (src/server.ts): another kind of file would cost a Worker request each,
# and a dot file is what a render that stopped left behind. Not gml/'s files, which the build reads and this
# doesn't serve (the source viewer's pages are).
other=$(cd "$media" && find sprites clips -type f \( -path '*/.*' -o ! \( -path 'sprites/*.png' -o \
  -path 'sprites/*.json' -o -path 'clips/*.png' -o -path 'clips/*.json' -o -path 'clips/*.txt' -o \
  -path 'clips/*.mp4' -o -path 'clips/*.webm' \) \) | head -1)
[ -z "$other" ] || die "$media has $other, which isn't a sprite's .png or .json or a clip's .png, .json, .txt, .mp4 or .webm: add it to wrangler.jsonc's run_worker_first"
git_media() { git -C "$media" "$@"; }
# The checkout's history, or a --wiki-media directory's that's a repository's top (test.sh's stand-in), not one
# that's only somewhere in another repository's work tree, whose history isn't the media's.
if [ -z "$wiki_media" ] || [ "$(git_media rev-parse --show-toplevel 2>/dev/null)" = "$(cd "$media" && pwd -P)" ]; then
  [ "$(git_media rev-parse --is-inside-work-tree 2>/dev/null)" = true ] ||
    die "wiki/media isn't in a git checkout, whose history dates the clips"
  # Workers Builds clones at depth 1, where git shows the checkout's commit adding every file, which would date
  # every clip from it. So a shallow clone fetches its history first, the commits and their trees, all git log
  # reads: not the files (every render of every clip), which a full clone would download. About a second. That
  # leaves it a partial clone, which fetches an old file when a command needs one.
  if [ "$(git_media rev-parse --is-shallow-repository)" = true ]; then
    echo "site/build.sh: fetching the shallow clone's history, which dates the clips"
    git_media fetch --quiet --filter=blob:none --unshallow origin "$(git_media rev-parse HEAD)" ||
      die "couldn't fetch the shallow clone's history, which dates the clips"
  fi
  # (Or the checkout's own commit, in a repository where no commit has the media yet.)
  media_date=$(when() { TZ=UTC git_media log -1 --format=%cd --date=format-local:%Y-%m-%dT%H:%M:%SZ "$@"; } &&
    { d=$(when -- .) && [ -n "$d" ] || d=$(when); } && echo "$d") ||
    die "git can't say when $media last changed"
  # One pass over the history, not one a clip: '@DATE', then the videos the commit changed, relative to clips/
  # as clip.json's files are, unquoted. No renames: a moved clip is a new video at its URL. The first parents
  # only: a merge's own date is when its branch's videos got here.
  media_log=$tmp/clips.log
  (cd "$media/clips" && TZ=UTC git -c core.quotePath=false log --first-parent --relative --no-renames \
    --format=@%cd --date=format-local:%Y-%m-%dT%H:%M:%SZ --name-only -- '*/clip.mp4' '*/clip.webm') >"$media_log" ||
    die "git can't say when the clips last changed"
  echo "site/build.sh: the clips dated by git's history (commits that changed their videos: $(grep -c '^@' "$media_log"))"
else
  media_date=$(node -p 'fs.statSync(process.argv[1]).mtime.toISOString().replace(/\.\d+Z$/, "Z")' "$media")
  media_log=
fi
export WIKI_MEDIA_DIR="$media" WIKI_MEDIA_URL=/wiki/media WIKI_MEDIA_DATE="$media_date" WIKI_MEDIA_LOG="$media_log"

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
# The router mustn't have a page at /wiki/media or /wiki/og: what's under them skips the Worker, cached by
# public/_headers (wrangler.jsonc).
for page in $assets/wiki/media $assets/wiki/media.html $assets/wiki/og.html; do
  [ ! -e "$page" ] || die "the app pre-rendered $page, where the wiki's media or Open Graph images go"
done
# Not the GML, which is the source viewer's to show, as /wiki/source/ pages the app pre-rendered from it
# (src/wiki/build/source.ts), not files to serve: another 1,140 assets, each a Worker request.
mkdir -p $assets/wiki/media
cp -R "$media/sprites" "$media/clips" "$media/LICENSE.txt" "$media/NOTICE.txt" $assets/wiki/media/
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
echo "site/build.sh: and the wiki's media from ${wiki_media:-wiki/media} at /wiki/media/ (last changed $media_date)"
echo "site/build.sh: $files static asset files, of the 20,000 Workers takes"
# wrangler dev holds a file descriptor per asset, and macOS's posix_spawn refuses one past OPEN_MAX (10,240),
# so it can't start on this many there (docs/web-deploy.md, "Testing locally"). Deploying is unaffected.
if [ "$files" -gt 9900 ] && [ "$(uname)" = Darwin ]; then
  echo "site/build.sh: too many for wrangler dev on macOS (spawn EBADF): serve it with npx vite preview"
fi
