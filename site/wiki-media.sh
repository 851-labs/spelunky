#!/bin/sh
# Puts the wiki's media, the package pinned in wiki-media.txt, at DIST/wiki/media/HASH/, HASH being the first
# 16 hex digits of the package's SHA-256: downloads it from the GitHub Release, checks the SHA-256 and unpacks
# it (sprites/, clips/, LICENSE.txt and NOTICE.txt, and gml/ in a package made since the source viewer;
# docs/wiki-media.md). A file's name doesn't change when its contents do (a sprite's frames, a clip's caption
# naming the release), so the directory names the package, and the site can cache what's in it for good
# (docs/web-deploy.md). A game release vX.Y.Z carries wiki-media-vX.Y.Z.tar.gz; a wiki-media-* release
# (wiki-media.yml's, between releases) carries <tag>.tar.gz. Does nothing while the pin is "none". Like
# build.sh, it needs only curl, tar and a SHA-256 tool, for Workers Builds.
#
#   site/wiki-media.sh DIST                  the pinned release
#   site/wiki-media.sh DIST --local PATH     a local package instead (scripts/wiki-media-package TAG), checked
#                                            against PATH.sha256 if it's there
set -eu
die() { echo "site/wiki-media.sh: $*" >&2; exit 1; }
usage() { echo "usage: site/wiki-media.sh DIST [--local PATH]" >&2; exit 2; }
given=
case $#:${2-} in
  1:) ;;
  3:--local) given=$3 ;;
  *) usage ;;
esac
dist=$1
[ -d "$dist" ] || die "no $dist"
# Relative to where it was run, not site/.
dist=$(cd "$dist" && pwd)
if [ -n "$given" ]; then
  [ -f "$given" ] || die "no $given"
  given="$(cd "$(dirname "$given")" && pwd)/$(basename "$given")"
fi
cd "$(dirname "$0")"

sha256() {
  if command -v sha256sum >/dev/null; then sha256sum "$1"; else shasum -a 256 "$1"; fi | cut -d' ' -f1
}

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
if [ -n "$given" ]; then
  tarball=$given
  want=
  [ ! -f "$given.sha256" ] || want=$(cut -d' ' -f1 "$given.sha256")
else
  tag=$(awk '$1 == "tag" { print $2 }' wiki-media.txt)
  want=$(awk '$1 == "sha256" { print $2 }' wiki-media.txt)
  if [ "$tag" = none ] || [ "$want" = none ]; then
    echo "site/wiki-media.sh: wiki-media.txt names no release yet, so the site has no /wiki/media/"
    exit 0
  fi
  [ -n "$tag" ] && [ ${#want} = 64 ] || die "wiki-media.txt needs a 'tag TAG' line and a 'sha256 HEX' line"
  # Not this script's, but build.sh's (the clips' uploadDate), which would fail only once deployed.
  awk '$1 == "date" { print $2 }' wiki-media.txt | grep -Eqx '[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z' ||
    die "wiki-media.txt needs a 'date YYYY-MM-DDTHH:MM:SSZ' line: when its release was published"
  case $tag in
    wiki-media-*) file=$tag.tar.gz ;;
    *) file=wiki-media-$tag.tar.gz ;;
  esac
  url=https://github.com/851-labs/spelunky/releases/download/$tag/$file
  echo "site/wiki-media.sh: downloading $url"
  tarball=$tmp/wiki-media.tar.gz
  curl -fsSL --retry 3 -o "$tarball" "$url" || die "couldn't download $url"
fi
got=$(sha256 "$tarball")
if [ -n "$want" ] && [ "$got" != "$want" ]; then
  die "$tarball has SHA-256 $got, not $want"
fi
hash=$(echo "$got" | cut -c1-16)
rm -rf "$dist/wiki/media"
mkdir -p "$dist/wiki/media/$hash"
tar -xzf "$tarball" -C "$dist/wiki/media/$hash"
for f in sprites/index.json clips/index.json LICENSE.txt NOTICE.txt; do
  [ -f "$dist/wiki/media/$hash/$f" ] || die "$tarball has no $f: not a scripts/wiki-media-package build"
done
# wrangler.jsonc's run_worker_first keeps the .png, .json and .txt files off the Worker, leaving it the clips'
# videos (src/server.ts): another kind of file, or a video outside clips/, would cost a Worker request each.
# Not gml/'s .gml files, which the build reads and build.sh doesn't serve (the source viewer's pages are).
other=$(find "$dist/wiki/media/$hash" -type f ! -name '*.png' ! -name '*.json' ! -name '*.txt' \
  ! \( -path "$dist/wiki/media/$hash/clips/*" \( -name '*.mp4' -o -name '*.webm' \) \) \
  ! \( -path "$dist/wiki/media/$hash/gml/*" -name '*.gml' \) | head -1)
[ -z "$other" ] || die "$tarball has ${other#"$dist/wiki/media/$hash/"}, which isn't a .png, .json, .txt or a clip's .mp4 or .webm: add it to wrangler.jsonc's run_worker_first"
echo "site/wiki-media.sh: the wiki's media at /wiki/media/$hash/ (SHA-256 $got)"
