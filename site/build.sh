#!/bin/sh
# Assembles the site in site/dist: public/ (the landing page), with the web build of the release named
# in web-release.txt unpacked at /play/. Workers Builds' build command (docs/web-deploy.md), so it needs
# only curl, tar and a SHA-256 tool, no Rust.
#
#   site/build.sh                 download the release's spelunky-web-<tag>.tar.gz and check its SHA-256
#   site/build.sh --local PATH    use a local tarball instead (scripts/web --package PATH), checked against
#                                 PATH.sha256 if it's there
set -eu
die() { echo "site/build.sh: $*" >&2; exit 1; }
given=
case $#:${1-} in
  0:) ;;
  2:--local) given=$2 ;;
  *) echo "usage: site/build.sh [--local PATH]" >&2; exit 2 ;;
esac
# Relative to where it was run, not site/.
[ -z "$given" ] || [ ! -f "$given" ] || given=$(cd "$(dirname "$given")" && pwd)/$(basename "$given")
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
fi
got=$(sha256 "$tarball")
if [ -n "$want" ] && [ "$got" != "$want" ]; then
  die "$tarball has SHA-256 $got, not $want"
fi

rm -rf dist
cp -R public dist
# The landing page links the license, which a Covered Work has to come with.
cp ../LICENSE dist/LICENSE.txt
mkdir dist/play
tar -xzf "$tarball" -C dist/play
[ -f dist/play/index.html ] || die "$tarball has no index.html: not a scripts/web --package build"
# Workers static assets take files of up to 25 MiB each.
big=$(find dist -type f -size +25600k)
[ -z "$big" ] || die "over Workers' 25 MiB per-file limit: $big"
echo "site/build.sh: dist/ ready, with $(cd dist/play && ls v) at /play/ (SHA-256 $got)"
