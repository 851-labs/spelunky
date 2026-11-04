# CI

The GitHub Actions workflows: one for every pull request, one for releases, a nightly one, and the pieces
they're made of.

| Workflow | When | What | Where |
| --- | --- | --- | --- |
| `pr.yml` | every pull request | `scripts/check` (`suite.yml`), `site/test.sh` for changes to the site, then on one Mac mini `scripts/check --apple`, `scripts/web --check` and `scripts/wiki-media --check` for changes to what the wiki's media are made from; the updater's tests; then `pr-ok` | `ubuntu-24.04` for the suite and the site, one Mac mini, GitHub's machines for the updater |
| `suite.yml` | from `pr.yml`, and every push to `main` | `scripts/check`, in two halves side by side, with the build caches main's run saves for every pull request's ([below](#pull-requests)) | `ubuntu-24.04` |
| `release.yml` | a `v*` tag, or by hand | the suite, the oracle fidelity run, the web and native builds, the updater's tests, then the GitHub Release and the pull request that puts it live ([below](#releases)) | the Mac minis and `ubuntu-24.04` |
| `build-native.yml` | from `release.yml`, or by hand | the native apps: built, signed for a release, smoke-tested ([docs/releasing.md](releasing.md)) | GitHub's `macos-15` for the build and the signing, then each OS, all on GitHub's machines |
| `updater.yml` | from `pr.yml` and `release.yml`, or by hand | the updater's tests, `cargo test -p update` ([docs/updates.md](updates.md)) | GitHub-hosted macOS, Windows and Linux |
| `mini.yml` | from `pr.yml` and `release.yml` | one command on a Mac mini, optionally only for a pull request that changes some paths | the Mac minis |
| `soak.yml` | nightly, or by hand | the chaos soak: new fuzz runs against the oracle, filed as issues ([below](#the-chaos-soak)) | `ubuntu-24.04` |

`pr-ok` is the only check the "Protect main" ruleset requires. It fails unless every other job of the run
succeeded, so jobs can be added or renamed without touching the ruleset; `pr-ok` itself keeps its name.

Every job has a short name for people, which the run's graph shows: "Linux / Test suite (1)" (a
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
  per-job output. A job on the minis is a call to `.github/workflows/mini.yml` with the command to run. What a
  release builds or signs never runs on them (below).
- **`ubuntu-24.04`** (GitHub-hosted, free for public repositories) runs a pull request's test suite and site
  test ([Pull requests](#pull-requests)), everything that needs Docker (the oracle), and builds the web version a
  release publishes.
- **GitHub-hosted `macos-15`** (Apple silicon, a fresh VM a job) builds a release's native apps and signs the
  macOS one (`build-native.yml`), set up from nothing by `scripts/release/ci-setup`.
- **GitHub-hosted `macos-15-intel`, `windows-2025`, `windows-11-arm`, `ubuntu-24.04` and
  `ubuntu-24.04-arm`** smoke-test the native builds, each on its own OS and architecture.

GitHub's machines are named by OS version (`ubuntu-24.04`, `windows-2025`, `macos-26`), never
`*-latest`, which moves to each new OS on GitHub's schedule (`ubuntu-latest` to 26.04 from October 2026):
the oracle's Docker setup and the Linux smoke test's packages (`libasound2t64`) are 24.04's, and a new OS
should come in a pull request of its own. (`windows-11-arm` has no versioned name.) The Linux release builds
don't depend on it: they're built on `macos-15` with zig, for glibc 2.31 ([docs/releasing.md](releasing.md)).

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

### Fallback: GitHub-hosted macOS

If the minis are ever unavailable, `pr.yml`'s mini job runs unchanged on GitHub's `macos-15` (Apple silicon,
free for a public repository), only slower. 851-3990 ran its commands there (image 20261028: "Apple M1
(Virtual)", 3 cores, 7 GiB, 43 GiB free; [PR #507](https://github.com/851-labs/spelunky/pull/507)'s runs):

- **The GPU.** Metal has one device, "Apple Paravirtual device", which wgpu takes (the Metal backend, an
  integrated GPU, WebGPU-compliant).
- **Every check passed as it is**, nothing loosened: `fidelity-shots` matched all 18 pictures exactly;
  `scripts/wiki-media --check`, rendering every clip, found `wiki/media` byte for byte what it renders (329 clips,
  each video kept), and with `--full`, encoding every clip again with the pinned ffmpeg, so were the videos;
  fidelity-check matched all 336 replays; the rest of `scripts/check --apple` and `scripts/web --check` passed.
- **The time.** The VM's three cores are much slower than a mini: about 7 times on what's the same work on both
  (fidelity-check's run of the replays, the clips' render). One run each, and GitHub's VMs vary from run to run
  (the warm run was slower than the cold one on some steps). The mini's column is one run that rendered the
  media ([41639042037](https://github.com/851-labs/spelunky/actions/runs/41639042037)); one that renders nothing
  takes about 2:05 in all:

  | Step | `macos-15`, cold | `macos-15`, warm | Mac mini |
  | --- | --- | --- | --- |
  | Setup: checkout, toolchain, Node, cache | 0:39 | 0:58 | 0:05 |
  | `scripts/check --apple` | 19:55 | 14:03 | 1:57 |
  | of which fidelity-check's run | 1:47 | | 0:15 |
  | `scripts/web --check` | 4:48 | 6:24 | 2:02, both steps |
  | `scripts/wiki-media --check`, every clip rendered | 10:04 | 14:19 | (above) |
  | of which the clips' render | 8:49 | | 1:20 |
  | `scripts/wiki-media --check --full`, every clip encoded again | | 36:28 | |
  | The job | about 36 min | about 36 min | 4:05 |

  "Warm" is a second run restoring the first's Swatinem/rust-cache (643 MiB): that keeps the dependencies, not the
  workspace's crates, which each run builds again, so it saves little. So the minis stay: on `macos-15` the job
  would take about 20 to 27 minutes for every pull request, and about 36 for one that changes what the media are
  made from, where a mini takes two to four.

To switch, in `pr.yml`'s `web` job: replace `uses: ./.github/workflows/mini.yml` (and its `with:`) with
`runs-on: macos-15`, `shell: bash` (as `mini.yml` has: bash with `pipefail`) and steps of its own:
`actions/checkout` (`persist-credentials: false`, `fetch-depth: 2`), `rustup toolchain install --no-self-update`
(the image has rustup), `actions/setup-node` with `site/.node-version`, `Swatinem/rust-cache` with a `shared-key`
of its own, then the same three commands, with a timeout of about 60 minutes. A pull request's cache serves only
its own later runs, so to share one, the job goes in a reusable workflow that a push to `main` runs too and that
alone saves the cache, as `suite.yml` does (not a push trigger on `pr.yml`, which reads the pull request's
fields). `release.yml`'s mini job, the full `scripts/check`, moves the same way, but it wasn't timed on
`macos-15`: give it a longer timeout than the default 60 minutes. Once no job of `pr.yml` or `release.yml` runs on
the minis, the fork `if:` and `pr-ok`'s refusal of forks, which are there for the minis, can go: a fresh VM keeps
nothing between jobs. It's one more of GitHub's macOS machines a pull request (the updater's is the other), of
the organization's five or so at once.

## Pull requests

The repository is public, so `pr.yml` runs untrusted code, on our own machines too. It uses `pull_request` (never
`pull_request_target`), a read-only token, no secrets and no environment, and doesn't leave the token in a
checkout (`persist-credentials: false`). A new push to a pull request cancels its previous run.

**Where each job runs** (851-3953). The two Mac minis are shared with other projects, and a pull request's three
mini jobs used to queue behind theirs: a run that took two or three minutes of work waited 6 to 60 minutes for
the minis. So a pull request now takes one mini, for what needs Apple silicon, and runs the rest on GitHub's
machines (free for a public repository): three more of them a pull request, beside the updater's (the
organization's projects share GitHub's machines too):

- **"Linux / Test suite (1)" and "(2)"** (`suite.yml`, two `ubuntu-24.04` machines): the whole of `scripts/check`,
  half on each (`--part 1` and `--part 2`, 851-3998). Linux builds and tests the workspace as the minis did,
  fidelity-check's replays included (the port is the same on every platform: `gm::math`); each installs the
  libraries the app links on Linux (ALSA, udev, Wayland, xkbcommon), the pinned toolchain and Node. On one machine
  the suite took about 15 minutes, nearly all of it building: the tests' build about 5.5 and the release build of
  the replays' tools about 3, each before it ran. So the halves are split by what they build, neither building
  what the other does: part 1 is the instant checks, clippy, the transpiler and every test; part 2 the release
  builds (the assets, the replays, the wiki's and the site's checks) and wasm32. Each half has its own build cache
  (Swatinem/rust-cache: the registry and `target/`), saved by the run on every push to `main`, which also checks
  `main` after each merge; a pull request's run restores it and saves nothing, as a pull request's cache could
  only be read by its own later runs, and would push main's out of GitHub's 10 GB.
- **"Site test"** (`ubuntu-24.04`): `site/test.sh` (build the site with stand-ins, serve it with `wrangler dev`,
  check every path's status, headers and content, [docs/web-deploy.md](web-deploy.md)), for a pull request that
  changes `site/`, what the site reads from outside it (`wiki/`, the specs and the media, `LICENSE`,
  `docs/quirks.md`), `scripts/bump-web-release` (which writes the site's pin), or `pr.yml`, or that removes or renames
  a committed replay (`fidelity/replays`, `fidelity/settings`), which the pages cite (only the first are Workers
  Builds' watch paths: [docs/web-deploy.md](web-deploy.md)). Every other pull
  request runs the job too, and `pr-ok` needs it, but it stops after the checkout and succeeds: a step diffs the
  pull request's merge commit against its base and skips the rest when none of the paths changed (a job-level
  `if:` can't see the changed files, and a skipped job would fail `pr-ok`). Workers Builds builds each pull
  request's site too, but tests nothing.
- **"Mac mini / Web, wiki media and macOS"**, one mini: `scripts/check --apple`, what a Mac checks beside Linux
  (the sh scripts under macOS's bash 3.2; the macOS-only code, which Linux doesn't compile: clippy over the
  workspace as macOS builds it, and the app's tests; `crates/workspace-hack`'s checks, which are Apple silicon
  Macs'; fidelity-check's replays on the Mac app's own architecture, so a pull request still checks the
  port is the same there as on Linux; and `scripts/fidelity-shots`, the renderer's pictures of chosen frames
  against the oracle's, on the mini's GPU: [formats.md](formats.md#screenshots)), then `scripts/web --check` and
  the wiki's committed media: `scripts/wiki-media --check --since HEAD^1`
  ([docs/wiki-media.md](wiki-media.md#committing)) renders them from the pull request and fails if `wiki/media`
  differs at all. A pull request that changes nothing the media are made from skips that at once; otherwise it
  builds the app, captures every clip's frames on the mini's GPU and encodes again the clips whose videos it
  changed, a few minutes more.

The updater's tests (`updater.yml`) run on every pull request, whatever it changes (the app reaches the
updater from outside `crates/update` too), on GitHub's machines alongside the others, and `pr-ok` needs
them. They take two or three minutes, less than the test suite, so they usually add no time to the run,
unless GitHub's macOS arm64 machines are queueing (the notice under [Warnings](#warnings)). They don't re-run on
`main` after a merge (only `suite.yml` does); a release runs them again before it publishes.

The site test fails on any error, not only on a check that fails. On a Mac (where it ran on CI until 851-3953, and
where it runs for anyone working on the site), `sh` is macOS's bash 3.2, which exits
from a syntax error, or an unset variable under `set -u`, with the status of the EXIT trap it then runs: 0,
for test.sh's cleanup. So a test.sh that stopped halfway passed, skipping every check after the error
(851-3741). Now it checks that it parses before anything else (`sh -n "$0"`), and its cleanup fails a run
that didn't reach its last line. `scripts/check-sh`, which `scripts/check --fast` runs, checks that every sh and
bash script in the repo parses (`sh -n`, `bash -n`), so a syntax error at a script's top level fails "Test suite (1)"
too; one inside a `$(...)` is only parsed as it runs, and fails the script then.

Every other script with an EXIT trap does the same (851-3743): `site/build.sh` (Workers Builds' build, whose
`sh` on Linux is dash, which exits with the error's status, but which runs on the Macs too), `scripts/wiki-media`
(the media check), `scripts/web`, `scripts/release/macos-dmg`, `scripts/bump-web-release`, `scripts/clip`,
`scripts/site-icons`, `scripts/seed-target`, and the bash ones, `tools/oracle/build`, `tools/oracle/run` and
`tools/gm8-probe/run` (macOS's bash is 3.2 too). Each checks that it parses (`sh -n "$0"` or `bash -n "$0"`)
before its trap, and its cleanup keeps a failing status and fails a run that didn't set `finished=1` where it
ends, before an intended `exit 0` too. `scripts/check-sh` fails a script that sets an EXIT trap and doesn't do
all of that, line by line: the parse check before `trap cleanup EXIT` (no other form), a cleanup that reads
`finished`, `finished=1` as the last line (unless it ends by `exec`), and `finished=1` before every `exit 0`
after the trap.

**Fork pull requests and the minis.** Code from outside must never run on the minis: they're persistent and
shared (with the organization's other repositories too), so a job can leave something behind for the next
(the reason releases are built and signed elsewhere, on GitHub's fresh `macos-15`). What
keeps a fork's code off them isn't `pr.yml` (851-3900). A pull request's run uses the workflow files from the
pull request, so a fork's pull request runs its own `pr.yml` and `mini.yml`, which can drop the mini job's
`if:` or point a job at the minis' labels; and the "Mac minis" runner group is open to this public repository.
The locks are outside the repository's files:

- only collaborators may open pull requests (the repository's `pull_request_creation_policy` is
  `collaborators_only`);
- runs from outside contributors wait for a maintainer's approval (the Actions setting "Require approval for
  all external contributors"), which is a person clicking Approve on a run whose workflow changes may not
  stand out;
- each mini's job-started hook, `.github/runner-hooks/job-started.sh`, fails a job before its first step for a
  pull request event whose head repository isn't the job's, or a `workflow_run` started by a fork's run, and
  any such job it can't read (it says why). It's a copy on the machine, named in the runner's `.env` as
  `ACTIONS_RUNNER_HOOK_JOB_STARTED` (so a pull request can't change it), installed by an admin of the minis;
  until it is, the first two are all there is. Each mini job says in its log if the hook is missing (a notice)
  or isn't the repository's copy (a warning: install it again), and `scripts/check --apple` tests the
  repository's on made-up events (`.github/runner-hooks/test.sh`).

`pr.yml`'s own check, skipping the mini job for a fork and failing `pr-ok`, is the fast, readable answer for a
fork's pull request that leaves `pr.yml` alone, not a lock.

A pull request's code can still change what the minis keep between jobs (the toolchain, the target dir, and
anything else its user can write: a `.cargo/config.toml` above a checkout, a login item, a tool on `PATH`), so
nothing a release publishes is built or signed on them: the web build and the native builds and their signing
run on fresh GitHub-hosted machines ([docs/releasing.md](releasing.md#on-ci), 851-3899).

## Releases

The port's releases are numbered from v0.1.0 (tags `v0.x.y`), so they're never confused with the original
game's 1.1. A release is cut by hand, by the user or the supervisor: CI never tags one itself, as each release
prompts every installed native app to update. First, if the night's [chaos soak](#the-chaos-soak) didn't run on
the commit to release, run it by hand and check it files nothing new. Then push the next version's tag
(`v0.1.N` for the next patch, `v0.2.0`, or a prerelease `v0.2.0-rc1`), annotated as every release's is:

```sh
git tag -a v0.1.N -m "Spelunky Classic port v0.1.N" && git push origin v0.1.N
```

and `release.yml`:

0. refuses the tag unless its commit is on `main`, before anything that uses the release secrets
   ([Release tags](#release-tags), below), and a release's (not a prerelease's) unless its version is above the
   latest release's and its commit is that release's or after it: it becomes GitHub's latest release, which the
   site's downloads and the apps' updater follow, so a mistyped tag (`v0.1.6` for `v0.1.17`) would otherwise put
   an older build there. (The latest release's own tag passes, so a failed run can be re-run; once it's published,
   Draft release refuses it.) Publish checks the version again, as another release may have come out meanwhile;
1. creates a draft GitHub Release for the tag (or reuses its draft on a re-run; a published release is never
   touched), titled "Spelunky Classic port vX.Y.Z (unofficial)", its notes the notice the Spelunky User License
   asks of a mod's documentation (`scripts/release/NOTICE.txt`'s, at the tag, up to its "Playing": the copyright,
   that it's an unofficial mod, the license, that it may not be sold) and the downloads, above GitHub's generated
   list of changes, as its page is where players get the files;
2. runs `scripts/check` on a mini;
3. runs every committed replay on the oracle and the port with `--fields` (`scripts/fidelity-all`), split
   across 10 `ubuntu-24.04` machines, three at a time (about an hour instead of 20 minutes), as the
   organization's projects share them;
4. builds the web version from scratch on `ubuntu-24.04`, `spelunky-web-TAG.tar.gz` (`scripts/web
   --package`), and checks it the way the site's build will (`site/build.sh --local`);
5. builds, signs, notarizes and smoke-tests the native apps (`build-native.yml`, [docs/releasing.md](releasing.md)):
   all six built on GitHub's `macos-15`, each file then played on its own OS and architecture, and runs the updater's
   tests (`updater.yml`), as a pull request does;
6. signs the native apps' update manifest, `latest.json`, for those downloads (`scripts/release/manifest`,
   [docs/updates.md](updates.md)), except for a prerelease, which is never offered to the apps;
7. once all of that passed, attests each file's build provenance (`actions/attest`: a statement, signed with
   Sigstore, that this run built it, which `gh attestation verify FILE --repo 851-labs/spelunky` checks), attaches
   the files, `latest.json` and a `SHA256SUMS` to the draft and publishes it: as a prerelease if the tag has a `-`
   (`v0.1.0-rc1`), otherwise as the latest release;
8. opens the pull request that puts it live ([below](#the-sites-bump-pull-request)), except for a prerelease.

Running `release.yml` by hand (Actions → Release → Run workflow) on a tag does the same. With `dry_run`, the
default, it runs on any branch and builds and checks everything but signs, notarizes and publishes nothing,
and opens no pull request: the files are the run's artifacts. Signing uses the `release` environment's secrets
(the Apple ones and `UPDATE_SIGNING_KEY`); only the jobs that sign reference it, and `release.yml` passes them
into `build-native.yml` with `secrets: inherit` (a called workflow's job gets none of its environment's
secrets without it). The environment has no required reviewer, only deployment rules that allow `main` and
`v*` tags, and only collaborators can push either. A failed job is re-run like any other: the draft is
reused, and the bump's push replaces its last.

### Release tags

A `v*` tag runs the workflow file at its commit, with the release environments' secrets, so what may push one is
what may publish a release, and a tag on a branch's commit, never reviewed or passed by `pr-ok`, would run that
commit's own workflows with every release secret and publish what they build. `release.yml`'s first job (Version)
refuses a tag whose commit `main` hasn't got (GitHub's compare of `refs/heads/main` with it: `identical` or
`behind`), and every job that uses an environment or passes secrets on needs it. But a tag's own copy of
`release.yml` could leave that out, so the lock is the repository's settings, which an admin makes (851-3901: until
they're made, any collaborator's `v*` tag still runs its own workflows):

- a tag ruleset on `refs/tags/v*`: creation only for its bypass list (the admins, who push release tags or have
  them pushed for them), and no updates or deletions, so a release's tag can never be moved to other code;
- optionally, a required reviewer on the `release` environment, so a person approves each release's signing;
- GitHub's immutable releases, so a published release's files can't be replaced.

### The site's bump pull request

A published release isn't live until `scripts/bump-web-release TAG` pins it in `site/web-release.txt` and that
merges ([docs/web-deploy.md](web-deploy.md)). `release.yml`'s last two jobs do it. **Site bump** runs the script on
`main` as it is then, with no secret: the script builds and runs Rust (`scripts/release/manifest` checks the
release's `latest.json`), so every dependency's build script runs there, and it passes on the one file it
changes. **Site bump PR**, on another machine and in the `release-bot` environment, runs none of the repository's
code: it checks that file (`main`'s comment and three lines, pinning this release and no older one than `main`
does, whose SHA-256s are those of the web build and `latest.json` the release has published, which it downloads
again), commits it on the `main` the first job read, pushes it to `bump-web-release/TAG` (the token reaching git through gh's
credential helper, never `.git/config` or a command line) and opens its pull request, "Release: put TAG live on
spelunky.ai", which closes the pull requests of earlier releases that haven't merged. So no code a build runs is
ever on a machine with the token (851-3906), and what that code could change of the one file is checked
against the release. What it can't recheck without running Rust is `latest.json`'s signature, which only the
first job verifies: but the file is the release's own, which this workflow signed. It never puts back an older
version than `main` pins (a re-run of an older release, or tags pushed out of order), nor opens one while
a newer release's is open. `pr-ok` runs on it like any other, and it merges itself once that passes if the
repository allows auto-merge (Settings → General → "Allow auto-merge"); otherwise whoever merges pull requests
merges it. It has no Linear ticket, and its body says why: the workflow has no Linear credential, and the
release and the run are its record.

**The credential**, which is optional. A pull request opened with `GITHUB_TOKEN` starts no workflow, so `pr-ok`
would never report on it, and the repository doesn't let Actions open pull requests anyway. So the job opens
it as a GitHub App installed on this repository alone, with the permissions **Contents: read and write** and
**Pull requests: read and write** (and Metadata: read, which every App has). Its client ID is the variable
`RELEASE_BOT_CLIENT_ID` and its private key the secret `RELEASE_BOT_PRIVATE_KEY`, both in the `release-bot`
environment, whose deployment rules allow only `main` and `v*` tags (the job runs on the release's tag), so no
pull request's run (nor another branch's) can read the key. The job mints a token from them for the one run,
asking for exactly those two permissions, so an installation without them fails at once. The commit is the
App's bot account's (`NAME[bot]`). A fine-grained personal access token (this repository only, the same two
permissions) in the same environment as `RELEASE_BOT_TOKEN` works too, but acts as its owner, and lasts until
it expires. Without either, the job opens nothing and leaves a warning and the command to run by hand in the
run's summary: run it and open the pull request yourself.

The "Protect main" ruleset has `require_extra_approval_for_unattributed_changes` on, which may hold an App's
pull request for one approval even with no approvals required; if it does, a collaborator approves it
(`gh pr review --approve`), or the ruleset turns the option off.

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
[fidelity/COVERAGE.md](../fidelity/COVERAGE.md#chaos-soak-851-3299)): half goal-directed (`--directed`) from a
level start, a quarter random input from a level start, and a quarter random input from partway through a
committed replay (`--starts olmec,ending,credits,title,editor,custom,stars`, `harness::fuzz::FROM_REPLAYS`), where
a run from a level start never gets: Olmec's Lair, the ending and the credits, the title's menus, the level editor
(with the mouse and its keys) and a custom level, a bonus room (851-3793). A replay start plays its replay's own game
seed, and the run's seed picks where in it to start (each start's frames: `FromReplay::frames`); it fuzzes freely in
any room, and its frame count and coverage are its fuzzed frames only. Besides the ten actions, every run presses M
and Enter now and then, and F10 from a level's pause screen, which ends the game; and a third of the level starts
begin with a `stats.txt` a game left behind. A run's seeds follow from its run number, 80 after the last run's (run 5, the first on
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

## Dependency audit

`audit.yml` checks what ships for known vulnerabilities: `cargo audit` (RustSec's own binary, pinned by version and
SHA-256, `CARGO_AUDIT`; it reads a lockfile alone, so no toolchain) on every tracked
`Cargo.lock` (the game's, which the apps and the web build come from, and the probes' under `tools/`) against the
RustSec advisory database, and `npm audit --omit=dev` on `site/package-lock.json`'s production packages (the pages'
scripts and the Worker; the build's tools, `devDependencies`, run on Workers Builds' and our machines, though Vite
and the Cloudflare plugin put a few helpers of their own in the bundles, which this doesn't cover). A
vulnerability fails it; an unmaintained or yanked crate is a warning in its log (today, RUSTSEC-2026-0192:
`ttf-parser` is unmaintained, under winit's Wayland decorations on Linux, with nothing to upgrade to).

It runs every night (an advisory can come out against a crate nothing here changed), and for a pull request or a
push to `main` that changes a lockfile, as one job on one of GitHub's machines, never the minis, with no secrets.
`pr-ok` doesn't wait for it, so an advisory against what's on `main` doesn't stop every pull request: a red night
is a ticket: upgrade the crate (or the package), or, for a crate with no fix whose flaw doesn't reach what ships,
add its advisory's ID to `audit.yml`'s `CARGO_AUDIT_IGNORE`, with why in the comment above it. npm has no such list:
an npm advisory without a fix takes an `overrides` entry in `site/package.json` pinning a fixed version. GitHub mails a failed scheduled run to whoever last changed its schedule. Dependabot's alerts
cover the same ground once the repository's dependency graph is on (Settings → Advanced Security), which needs
an admin.

## Pinned actions

Every action a workflow uses is pinned to a full commit SHA, its release in a comment
(`actions/checkout@3d3c42e5… # v7.0.1`), not a tag, which its owner (or someone who takes over their account or
release process) can move to other code: the jobs that hold the signing key, the update key and the release
bot's token run actions too (851-3902). `scripts/check --fast` fails on a `uses:` that isn't pinned that way.
Dependabot (`.github/dependabot.yml`) opens a weekly pull request moving the pins to the newest releases, the minor
and patch ones in one and a major one alone, never a release in its first week (a hijacked one is usually pulled
by then). Its pull requests run `pr.yml` like any other, so they're read before they merge: what a pin moves to
is code our jobs run. The check reads only the pin's shape; that a SHA is the commit of the version its comment
names is for the reviewer (`git ls-remote https://github.com/OWNER/REPO refs/tags/VERSION`). The repository's settings can enforce the same (Settings → Actions → General: "Require actions to be
pinned to a full-length commit SHA", and allowing only GitHub's actions and the ones listed), which an admin
sets.

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
