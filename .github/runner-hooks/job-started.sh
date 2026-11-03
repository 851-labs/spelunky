#!/bin/bash
# The Mac minis' job-started hook (docs/ci.md, "Pull requests", 851-3900): fails a job that would run a fork's
# pull request code, before its first step. A pull request's run uses the workflow files from the pull request
# itself, so a check in pr.yml (its `if:`) is one the fork's pull request can drop; this one is on the machine,
# where no pull request reaches. It isn't read from the repository at run time: a copy is installed on each mini,
# outside the runner's work directory, and named in the runner's .env as ACTIONS_RUNNER_HOOK_JOB_STARTED (the
# runner takes only a .sh or .ps1 there), by an admin of the machines ("For the user" in 851-3900). The minis are
# shared with the organization's other repositories, and it applies to their jobs too.
#
# Refused:
# - a pull request event (pull_request, pull_request_target, the review events) whose head repository isn't the
#   job's repository;
# - a workflow_run whose triggering run's head repository isn't the job's (a fork's pull request's run starts it).
# A job it can't decide on (no event file, a field missing: a fork deleted since) is refused too, saying why.
# Every other event goes on. It reads the event's JSON with plutil, which every macOS has.
set -u
refuse() {
  echo "::error::This Mac mini doesn't run code from forks: $*"
  exit 1
}
case "${GITHUB_EVENT_NAME:-}" in
  pull_request | pull_request_target | pull_request_review | pull_request_review_comment)
    field=pull_request.head.repo.full_name ;;
  workflow_run) field=workflow_run.head_repository.full_name ;;
  *) exit 0 ;;
esac
[ -n "${GITHUB_REPOSITORY:-}" ] || refuse "GITHUB_REPOSITORY isn't set, so a $GITHUB_EVENT_NAME job can't be checked"
[ -r "${GITHUB_EVENT_PATH:-}" ] || refuse "the event file (GITHUB_EVENT_PATH, '${GITHUB_EVENT_PATH:-}') can't be read"
head=$(/usr/bin/plutil -extract "$field" raw -o - "$GITHUB_EVENT_PATH" 2>/dev/null) ||
  refuse "the $GITHUB_EVENT_NAME event has no $field"
[ "$head" = "$GITHUB_REPOSITORY" ] || refuse "a $GITHUB_EVENT_NAME from $head into $GITHUB_REPOSITORY"
exit 0
