# CI

The GitHub Actions workflows: one for every pull request, two for releases (one that starts them after a merge,
one that makes them), a nightly one, and the pieces they're made of.

| Workflow | When | What | Where |
| --- | --- | --- | --- |
| `pr.yml` | every pull request | `scripts/check`, `scripts/web --check`, `scripts/wiki-media --check` for changes to what the wiki's media are made from, `site/test.sh` for the site's, the updater's tests, then `pr-ok` | the Mac minis, and GitHub's machines for the updater |
| `auto-release.yml` | every merge to `main`, or by hand | a release (`release.yml`) once `main` is quiet, then the pull request that puts it live ([below](#automatic-releases)) | `ubuntu-24.04` |
| `release.yml` | from `auto-release.yml`, a `v*` tag, or by hand | the suite, the oracle fidelity run, the web and native builds, the updater's tests, then the GitHub Release | the Mac minis and `ubuntu-24.04` |
| `build-native.yml` | from `release.yml`, or by hand | the native apps: built, signed for a release, smoke-tested ([docs/releasing.md](releasing.md)) | a Mac mini, then each OS |
| `updater.yml` | from `pr.yml` and `release.yml`, or by hand | the updater's tests, `cargo test -p update` ([docs/updates.md](updates.md)) | GitHub-hosted macOS, Windows and Linux |
| `mini.yml` | from `pr.yml` and `release.yml` | one command on a Mac mini, optionally only for a pull request that changes some paths | the Mac minis |
| `soak.yml` | nightly, or by hand | the chaos soak: new fuzz runs against the oracle, filed as issues ([below](#the-chaos-soak)) | `ubuntu-24.04` |

`pr-ok` is the only check the "Protect main" ruleset requires. It fails unless every other job of the run
succeeded, so jobs can be added or renamed without touching the ruleset; `pr-ok` itself keeps its name.

Every job has a short name for people, which the run's graph shows: "Mac mini / Test suite" (a
`mini.yml` job is named by its caller's `name` input), "Updater / Tests (Windows x64)", "Native / Build
apps", "Replays vs oracle (3)". A job's own name stays under about 20 characters, which is what the graph's
boxes show of it. A matrix job's name leaves the matrix out, and the matrix holds only what tells its jobs
apart, which GitHub adds in parentheses: a name with `${{ matrix.* }}` in it shows unexpanded on a skipped
job.

## Runners

- **The Mac minis**: the org's self-hosted runner group "Mac minis" (Apple Silicon, labels `self-hosted, macOS,
  ARM64, codevisor-macos-arm64`), shared with other repositories. They come with no Rust, Node or Docker, so
  `.github/actions/mini-toolchain` installs rustup (a pinned `rustup-init`, checked against its SHA-256),
  the toolchain `rust-toolchain.toml` pins and Node (`site/.node-version`'s, which Workers Builds builds the
  site with) into the runner's tool cache, where later jobs find them. The checkout's `target/` is a link to a
  cargo target dir kept there too, so builds are incremental across jobs; the action wipes it once it outgrows
  12 GiB (the minis have about 30 GB free). npm's cache is there too, not in `~/.npm`, wiped past 1 GiB, and
  `cleanup.sh` (the last step of each job) drops the extracted assets, the site's `node_modules` and other
  per-job output. A job on the minis is a call to `.github/workflows/mini.yml` with the command to run. The native builds
  (`build-native.yml`) set themselves up with `scripts/release/ci-setup` instead, with a target dir and cross
  toolchains of their own, and from nothing in the job's temp directory for a build that gets signed.
- **`ubuntu-24.04`** (GitHub-hosted, free for public repositories) runs everything that needs Docker (the
  oracle) and builds the web version a release publishes.
- **GitHub-hosted `macos-15-intel`, `windows-2025`, `windows-11-arm`, `ubuntu-24.04` and
  `ubuntu-24.04-arm`** smoke-test the native builds, each on its own OS and architecture.

GitHub's machines are named by OS version (`ubuntu-24.04`, `windows-2025`, `macos-26`), never
`*-latest`, which moves to each new OS on GitHub's schedule (`ubuntu-latest` to 26.04 from October 2026):
the oracle's Docker setup and the Linux smoke test's packages (`libasound2t64`) are 24.04's, and a new OS
should come in a pull request of its own. (`windows-11-arm` has no versioned name.) The Linux release builds
don't depend on it: they're built on a mini with zig, for glibc 2.31 ([docs/releasing.md](releasing.md)).

### The Mac minis' loose objects

On the minis (both of them, macOS 27.2 with Apple's git 2.54), a `git add` that writes and freshens a
thousand-odd loose objects at once sometimes fails partway: `error: unable to create temporary file: Invalid
argument`, then `failed to insert into database` and `fatal: updating files failed`. It looks like the
filesystem rather than git or the job's setup: git's `open()` of a temporary file in `.git/objects/xx/` returns
`EINVAL`, often just after it failed to freshen an object that's already there (most of the files it failed on
had the same content as one added before them). It hit site/test.sh's stand-in media (1,672 files,
194 objects) in about one run in seven, 7 of 50 in a loop on a mini, and never in a loop of 12 on a
laptop with the same macOS and git, nor in 200 on a mini with a made-up tree of the same shape. The clone's
checkout of the same files, and a pack, never failed. So a script that commits many files on CI writes them
with one `git fast-import` (a single pack, as site/test.sh's stand-in does), not `git add` and `git commit`.

## Pull requests

The repository is public, so `pr.yml` runs untrusted code on our own machines. It uses `pull_request` (never
`pull_request_target`), a read-only token, no secrets and no environment, and doesn't leave the token in the
self-hosted checkout (`persist-credentials: false`). A new push to a pull request cancels its previous run.

The updater's tests (`updater.yml`) run on every pull request, whatever it changes (the app reaches the
updater from outside `crates/update` too), on GitHub's machines alongside the minis' jobs, and `pr-ok` needs
them. They take two or three minutes, less than the test suite, so they usually add no time to the run,
unless GitHub's macOS arm64 machines are queueing (the notice under [Warnings](#warnings)). Like every other
check, they don't re-run on `main` after a merge; a release runs them again before it publishes.

The wiki's committed media are checked in the web job, "Mac mini / Web and wiki media", after `scripts/web
--check`: `scripts/wiki-media --check --since HEAD^1` ([docs/wiki-media.md](wiki-media.md#committing)) renders
them from the pull request and fails if `wiki/media` differs at all, rather than in a job of its own, so a pull
request still takes three minis (they're shared with other projects). A pull request that changes nothing the
media are made from skips it at once; otherwise it builds the app, captures every clip's frames on the mini's
GPU and encodes again the clips whose videos it changed, a few minutes more.

The site's smoke test, `site/test.sh` (build it with stand-ins, serve it with `wrangler dev`, check every
path's status, headers and content, [docs/web-deploy.md](web-deploy.md)), runs on a mini, "Mac mini / Site
test", for a pull request that changes `site/`, what the site reads from outside it (`wiki/`, the specs and
the media, `LICENSE`), `scripts/bump-web-release` (which writes the site's pin), or the
job's own workflows and toolchain. It takes under a minute; `npm ci` downloads only what the mini's npm
cache lacks. Every other pull request runs the job too, and `pr-ok` needs it, but it stops after the checkout
and succeeds: `mini.yml`'s `paths` diffs the pull request's merge commit against its base and skips the setup
and the command when none of the paths changed. A job-level `if:` can't see the changed files, and a skipped
job would fail `pr-ok`. Workers Builds builds each pull request's site too, but tests nothing. The test runs with its home directory
in the job's temp directory, so wrangler's config, logs and dev registry stay out of the mini's.

The test fails on any error, not only on a check that fails. The minis' `sh` is macOS's bash 3.2, which exits
from a syntax error, or an unset variable under `set -u`, with the status of the EXIT trap it then runs: 0,
for test.sh's cleanup. So a test.sh that stopped halfway passed, skipping every check after the error
(851-3741). Now it checks that it parses before anything else (`sh -n "$0"`), and its cleanup fails a run
that didn't reach its last line. `scripts/check-sh`, which `scripts/check --fast` runs, checks that every sh and
bash script in the repo parses (`sh -n`, `bash -n`), so a syntax error at a script's top level fails "Test suite"
too; one inside a `$(...)` is only parsed as it runs, and fails the script then. The repo's other scripts with
an EXIT trap are 851-3743.

The minis also sign releases, so code from outside never runs on them. Only collaborators may open pull
requests (the repository's `pull_request_creation_policy` is `collaborators_only`), runs from outside
contributors wait for a maintainer's approval (the Actions setting "Require approval for all external
contributors"), and `pr.yml` itself skips its mini jobs for a pull request from a fork and fails `pr-ok`, so
keeping a fork's code off the minis doesn't rest on those settings alone.

A pull request's code can still change what the minis keep between jobs (the toolchain, the target dir), so
nothing a release publishes is built from those caches: the web build runs on a fresh GitHub-hosted machine.

## Releases

The port's releases are numbered from v0.1.0 (tags `v0.x.y`), so they're never confused with the original
game's 1.1. A merge that changes the game or the apps is released on its own, as the next patch version
([Automatic releases](#automatic-releases)). By hand, for another version (`v0.2.0`, or a prerelease
`v0.2.0-rc1`): first, if the night's [chaos soak](#the-chaos-soak) didn't run on the commit to release, run it
by hand and check it files nothing new. Then push a tag, `git tag -a v0.2.0 -m "Spelunky Classic port v0.2.0" &&
git push origin v0.2.0`, and the automatic releases count on from it. Either way, `release.yml`:

1. creates a draft GitHub Release for the tag (or reuses its draft on a re-run; a published release is never
   touched);
2. runs `scripts/check` on a mini;
3. runs every committed replay on the oracle and the port with `--fields` (`scripts/fidelity-all`), split
   across 10 `ubuntu-24.04` machines (three at a time for an automatic release);
4. builds the web version from scratch on `ubuntu-24.04`, `spelunky-web-TAG.tar.gz` (`scripts/web
   --package`), and checks it the way the site's build will (`site/build.sh --local`);
5. builds, signs, notarizes and smoke-tests the native apps (`build-native.yml`, [docs/releasing.md](releasing.md)):
   all six built on a mini, each file then played on its own OS and architecture, and runs the updater's
   tests (`updater.yml`), as a pull request does;
6. signs the native apps' update manifest, `latest.json`, for those downloads (`scripts/release/manifest`,
   [docs/updates.md](updates.md)), except for a prerelease, which is never offered to the apps;
7. once all of that passed, attaches the files, `latest.json` and a `SHA256SUMS` to the draft and publishes
   it: as a prerelease if the tag has a `-` (`v0.1.0-rc1`), otherwise as the latest release.

Then the web build and the update go live with the pull request of `scripts/bump-web-release TAG`
([docs/web-deploy.md](web-deploy.md)), which `auto-release.yml` opens after its releases, and which a release
by hand needs opened by hand.

Running `release.yml` by hand (Actions → Release → Run workflow) on a tag does the same. With `dry_run`, the
default, it runs on any branch and builds and checks everything but signs, notarizes and publishes nothing:
the files are the run's artifacts. Signing uses the `release` environment's secrets (the Apple ones and
`UPDATE_SIGNING_KEY`); only the jobs that sign reference it, and `release.yml` passes them into
`build-native.yml` with `secrets: inherit` (a called workflow's job gets none of its environment's secrets
without it). Releases are automatic: the environment has no required reviewer, only deployment rules that
allow `main` and `v*` tags (a release runs on its tag, an automatic one on `main`), and only collaborators can
push either.

### Automatic releases

`auto-release.yml` runs on every push to `main`, and releases the ones that change what a release is made from,
which `scripts/release/auto` decides: the crates the apps and the asset extractor build from, `vendor/`, the original's
files in `reference/`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `LICENSE`, and what packages them
(`scripts/cargo`, `scripts/extract`, `scripts/web`, `scripts/release/`, `build-native.yml`). Not `crates/harness`,
the wiki's tools in `crates/tools`, tests, Markdown, the rule's own script, `docs/`, `site/` (deployed on its own, [docs/web-deploy.md](web-deploy.md)),
`wiki/` or the other workflows: a merge of only those ends in seconds. (Not a `paths` filter on the workflow,
which sees only the first 300 files a push changes: a merge with re-rendered wiki media can change more.)
`scripts/release/auto-test`, which `scripts/check` runs, tests the rule. Its jobs:

1. **Plan** compares the commit with the highest release tag, `vMAJOR.MINOR.PATCH` (prereleases don't count),
   and releases it if something it's made from differs and no release tag already has the commit. It waits
   first, until `main` has had no such merge for 10 minutes, but no longer than 30 minutes after the first
   one since the last release, then plans again (a tag pushed by hand meanwhile releases the commit itself).
   The next merge's run cancels the wait, and waits until the same time unless it's such a merge too, so
   merges that land together go out together. The wait holds one of GitHub's machines, and only ever one.
   The run's summary lists the changed files.
2. **Release** is `release.yml`, called with `auto`, one at a time: a later merge's release waits for the one
   running, and replaces an earlier one still waiting, so what merges during a release goes out in one more.
   Its first job tags the commit with the next patch version (or, on a re-run, the tag it already has), an
   annotated tag by `github-actions[bot]`, pushed with the run's own token: a tag pushed with `GITHUB_TOKEN`
   starts no workflow, so it doesn't start a second release. It fails if the commit was tagged by hand while
   it waited (that tag's own run releases it), and refuses to tag or re-release a commit older than a release
   already out, which would offer the apps a downgrade. Nobody waits on an automatic release, so its replays
   take three of GitHub's machines at a time rather than all ten (about an hour instead of 20 minutes): the
   organization's projects share them.
3. **Site bump PR**, once the release is published, runs `scripts/bump-web-release` on `main` as it is then,
   pushes it to `auto-release/TAG` and opens its pull request, "Release: put TAG live on spelunky.ai", which
   closes the pull requests of earlier releases that haven't merged. It never puts back an older version than
   `main` pins, nor opens one while a newer release's is open. `pr-ok` runs on it like any other, and it
   merges itself once that passes if the repository allows auto-merge (Settings → General → "Allow
   auto-merge"); otherwise whoever merges pull requests merges it. It has no Linear ticket, and its body says
   why: the workflow has no Linear credential, and the release and the run are its record.

**The credential.** A pull request opened with `GITHUB_TOKEN` starts no workflow either, so `pr-ok` would never
report on it, and the repository doesn't let Actions open pull requests anyway. So the bump job opens it as a
GitHub App installed on this repository alone, with the permissions **Contents: read and write** and
**Pull requests: read and write** (and Metadata: read, which every App has). Its client ID is the variable
`RELEASE_BOT_CLIENT_ID` and its private key the secret `RELEASE_BOT_PRIVATE_KEY`, both in the `release-bot`
environment, which allows only `main`, so no pull request's run (nor any other branch's) can read the key. The
job mints a token from them for the one run, asking for exactly those two permissions, so an installation
without them fails at once. The commit is the App's bot account's (`NAME[bot]`). A fine-grained personal access
token (this repository only, the same two permissions) in the same environment as `RELEASE_BOT_TOKEN` works
too, but acts as its owner, and lasts until it expires. Without either, the job opens nothing and leaves
a warning and the command to run by hand in the run's summary.

The "Protect main" ruleset has `require_extra_approval_for_unattributed_changes` on, which may hold an App's
pull request for one approval even with no approvals required; if it does, a collaborator approves it
(`gh pr review --approve`), or the ruleset turns the option off.

**By hand.** Actions → Auto release → Run workflow, with `dry_run` (the default), plans the branch's commit
without waiting and releases nothing, then, on `main`, mints the App's token (or reads the token) and runs
`scripts/bump-web-release` for the latest release without pushing it: a check of the setup. Without `dry_run`,
on `main`, it releases `main` now if it has changed, and opens the pull request. A failed job of an automatic run
is re-run like any other: the tag and the draft are reused, and the bump's push replaces its last. Re-run it, as
nothing else will: the tag is pushed when the release starts, so the plan counts the commit as released (its
summary says "Already tagged") and only the next change goes out in a new one. The repository
variable `AUTO_RELEASE=off` (Settings → Secrets and variables → Actions → Variables) turns the automatic runs off;
releases by tag still work.

### The oracle on CI

The `Oracle build` job (`.github/actions/oracle-build`, which the [chaos soak](#the-chaos-soak) shares) caches
the oracle's Docker image and its `gm8emulator` under the build stamp (`tools/oracle/common.sh`), so the
fidelity shards only `docker load` them (`.github/actions/oracle-load`). A new stamp (a patch, the Dockerfile,
the pinned commits or `vendor/pxfm`) rebuilds them, starting from the last build's cargo registry and target
volumes, which are cached too. A cache made on `main`, by the nightly soak, serves a release's tag too. The
`Port build` job builds the `fidelity` binary and extracts the assets once for all shards. Each shard puts its
traces on the runner's largest disk and deletes each replay's once it's compared: with `--fields`, a long
replay's two traces take gigabytes.

## The chaos soak

The committed replays each check one path through the game, and its levels are random, so a divergence can
hide where no replay goes. `soak.yml` keeps looking: every night at 10:17 UTC it plays 80 game seeds that no
run has played, on the oracle and the port with `--fields` (`fuzz-sweep`,
[fidelity/COVERAGE.md](../fidelity/COVERAGE.md#chaos-soak-851-3299)): three in four goal-directed (`--directed`),
the rest random input. A run's seeds follow from its run number, 80 after the last run's (run 5, the first on
`main`, starts at 1451, after the seeds earlier sweeps took), so a re-run plays the same seeds and no two runs share
one. The organization's projects share 20 concurrent jobs on GitHub's machines, so the soak never takes more
than two: its jobs run in turn (the seeds, the two builds, the two shards, the report), and only one soak runs at
a time. Each shard plays 40 seeds, four at a time, in about 40-90 minutes (20 took 20-46), and stops after 150.
It runs only on GitHub's `ubuntu-24.04` machines, with no secrets, and only its last job may write issues.

A run that isn't identical to the oracle fails its shard, and the shard's artifact, `soak-shard-<n>`, keeps
everything about it for 90 days (7 for a clean shard): `results.jsonl` (a line per run: verdict, first
divergent frame, first difference and its signature, the command that reproduces it), the replays
(`replays/<run>.txt`, and `<run>-min.txt`, cut off 30 frames after the divergence and checked to still diverge
there) and fidelity's reports (`traces/<run>/fidelity.json`), under `directed/` or `random/`. The `Report` job
(`.github/scripts/soak-report`) then files it on GitHub by its signature: the first difference's object and
field without instance ids or values (`oPlayer1 x`, `oBat only in b`), or the verdict if nothing diverged
(`port stopped`). A signature with no open issue labelled `chaos-soak` gets one, "Chaos soak: <signature>";
one with an open issue gets a comment on it instead. Either lists the runs, their seeds, first divergent
frames and differences, the run, the commit, and the commands to reproduce them. Seeds whose runs didn't
finish (an error, or a shard's time running out) are filed the same way, as "runs that didn't finish". A clean
night files nothing. A re-run reports only the runs it hasn't yet.

**By hand.** Before a release, run it (Actions → Chaos soak → Run workflow) on the commit being released: it
plays the next 80 seeds, as a night would, and files what differs. `first_seed` and `count` choose other
seeds, such as a night's that didn't finish. To try the issue step, set `fake_mismatch`, which makes shard 1's
first run look as if it diverged, with a throwaway `label` (it refuses `chaos-soak`), then delete the issue it
opens and the label.

**Reproducing a finding.** At the issue's commit, with the oracle built (`tools/oracle/build`,
[tools/oracle/README.md](../tools/oracle/README.md)) and the assets extracted (`scripts/extract`), the
issue's command runs that one run again, with `--keep` keeping its traces under `target/fuzz/traces/`:

```sh
scripts/cargo run --release -p harness --bin fuzz-sweep -- --directed --starts ice --seeds 1234 --fuzz 1 \
  --frames 3000 --fields --keep
```

The fuzzer plays the port to write its replay, so the replay depends on the port: at another commit the same
command can make another replay. The artifact's replay doesn't: `scripts/fidelity --fields <run>-min.txt` runs
it at any commit, and is where to start.

**Turning a finding into a replay and a ticket.** Make the issue a Linear ticket (team 851, project spelunky)
with its signature and the shortest replay that reproduces it (usually `<run>-min.txt`), and link the ticket
from the issue. Leave the issue open until the fix merges: the soak only looks for open issues, so a closed one
whose divergence comes back gets a new issue, which after a fix is what a regression should get. The fix commits that replay under `fidelity/replays/`, named for what it covers, with
a header saying where it came from (`fuzz-goal-ice-seed-63-alien-boss.txt` is one), and records its
fingerprint with `scripts/fidelity --refresh-fingerprints` once the port matches the oracle
([docs/formats.md](formats.md), "Fingerprints"), so the check can't regress.

## Warnings

A run should show no warnings or notices of ours, so a new one stands out: fix it where it comes from,
don't silence it. These come from GitHub or from actions at their latest versions, and nothing here can
fix them:

- **"Due to capacity constraints, jobs targeting macOS arm64 runners may experience longer queue times"**,
  on `updater.yml`'s macOS job (`macos-26`): GitHub's notice about its own Apple Silicon machines.
- **Node's `[DEP0005] DeprecationWarning: Buffer() is deprecated`**, in every `actions/download-artifact`
  step: the action's own dependencies call `Buffer()`
  ([actions/download-artifact#484](https://github.com/actions/download-artifact/issues/484)).
- **git's `Warning: you are leaving 1 commit behind`**, in `actions/checkout` on a Mac mini, whose
  workspace outlives the job: git says so when the last job left a commit checked out that no branch has
  any more, after a force-push.
