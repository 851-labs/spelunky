#!/bin/bash
# The job-started hook (job-started.sh) on made-up events: a fork's pull request and workflow_run refused, the
# repository's own and other events let through, and an event it can't read refused. scripts/check runs it on a
# Mac, and so does --apple (macOS: the hook reads JSON with plutil).
set -eu
# A syntax error anywhere fails it here, before the EXIT trap below can pass it: macOS's bash 3.2 exits from one
# with the trap's status, 0 (851-3743; scripts/check-sh).
bash -n "$0" || exit 2
cd "$(dirname "$0")"
tmp=$(mktemp -d)
# Set where it ends, so a run that stops anywhere before fails, whatever the shell's status says: bash 3.2 exits
# from an error (an unset variable under set -u) with this trap's status, 0 for a cleanup (851-3743).
finished=
cleanup() {
  status=$?
  rm -rf "$tmp"
  if [ "$status" -eq 0 ] && [ -z "$finished" ]; then
    echo ".github/runner-hooks/test.sh: stopped before its end, so it fails" >&2
    status=1
  fi
  exit "$status"
}
trap cleanup EXIT
echo '{"pull_request":{"head":{"repo":{"full_name":"851-labs/spelunky"}}}}' >"$tmp/pr-ours.json"
echo '{"pull_request":{"head":{"repo":{"full_name":"someone/spelunky"}}}}' >"$tmp/pr-fork.json"
echo '{"pull_request":{"head":{"repo":null}}}' >"$tmp/pr-gone.json"
echo '{"workflow_run":{"head_repository":{"full_name":"851-labs/spelunky"}}}' >"$tmp/run-ours.json"
echo '{"workflow_run":{"head_repository":{"full_name":"someone/spelunky"}}}' >"$tmp/run-fork.json"
failures=0
expect() {
  local want=$1 event=$2 file=$3 got=0
  GITHUB_EVENT_NAME=$event GITHUB_EVENT_PATH=$tmp/$file GITHUB_REPOSITORY=851-labs/spelunky \
    ./job-started.sh >/dev/null 2>&1 || got=$?
  if [ "$got" = "$want" ]; then echo "ok   $event $file: $got"; else echo "FAIL $event $file: $got, not $want"; failures=$((failures + 1)); fi
}
expect 0 pull_request pr-ours.json
expect 1 pull_request pr-fork.json
expect 1 pull_request_target pr-fork.json
expect 1 pull_request_review_comment pr-fork.json
expect 1 pull_request pr-gone.json
expect 1 pull_request missing.json
expect 0 workflow_run run-ours.json
expect 1 workflow_run run-fork.json
expect 0 push pr-fork.json
expect 0 schedule missing.json
[ "$failures" = 0 ] || { echo ".github/runner-hooks/test.sh: $failures failed" >&2; exit 1; }
echo ".github/runner-hooks/test.sh: all passed"
finished=1
