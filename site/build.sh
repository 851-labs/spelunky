#!/bin/sh
# Builds the site in site/dist: the TanStack Start app (src/, with public/ and its pages pre-rendered) with
# Vite, as dist/server (the Worker) and dist/client (its static assets), and adds to dist/client the web build
# of the release named in web-release.txt, unpacked at /play/, and the native apps' update manifest (that
# release's latest.json, docs/updates.md) at /releases/latest.json. Workers Builds' build command
# (docs/web-deploy.md), so it needs only Node, curl, tar and a SHA-256 tool, no Rust.
#
#   site/build.sh                 download the release's spelunky-web-<tag>.tar.gz and latest.json, and
#                                 check their SHA-256s
#   site/build.sh --local PATH [--manifest FILE]
#                                 use a local tarball instead (scripts/web --package PATH), checked against
#                                 PATH.sha256 if it's there, and FILE as latest.json
set -eu
die() { echo "site/build.sh: $*" >&2; exit 1; }
usage() { echo "usage: site/build.sh [--local PATH [--manifest FILE]]" >&2; exit 2; }
given=
manifest=
case $#:${1-}:${3-} in
  0::) ;;
  2:--local:) given=$2 ;;
  4:--local:--manifest) given=$2 manifest=$4 ;;
  *) usage ;;
esac
# Relative to where it was run, not site/.
absolute() {
  if [ -f "$1" ]; then echo "$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"; else echo "$1"; fi
}
[ -z "$given" ] || given=$(absolute "$given")
if [ -n "$manifest" ]; then
  [ -f "$manifest" ] || die "no $manifest"
  manifest=$(absolute "$manifest")
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

# package-lock.json pins every package: npm ci installs exactly those, or fails.
npm ci --no-audit --no-fund
rm -rf dist
node_modules/.bin/vite build
# Vite only strips the types. After the build, which brings src/routeTree.gen.ts up to date with src/routes/.
node_modules/.bin/tsc -p .
assets=dist/client
[ -f $assets/index.html ] || die "vite build pre-rendered no $assets/index.html"
# The landing page links the license, which a Covered Work has to come with.
cp ../LICENSE $assets/LICENSE.txt
mkdir $assets/play
tar -xzf "$tarball" -C $assets/play
[ -f $assets/play/index.html ] || die "$tarball has no index.html: not a scripts/web --package build"
if [ -n "$manifest" ]; then
  mkdir $assets/releases
  cp "$manifest" $assets/releases/latest.json
fi
# Workers static assets take files of up to 25 MiB each.
big=$(find $assets -type f -size +25600k)
[ -z "$big" ] || die "over Workers' 25 MiB per-file limit: $big"
echo "site/build.sh: dist/ ready, with $(cd $assets/play && ls v) at /play/ (SHA-256 $got)"
if [ -n "$manifest" ]; then
  echo "site/build.sh: and the update manifest at /releases/latest.json"
else
  echo "site/build.sh: no update manifest: the native apps find no update"
fi
