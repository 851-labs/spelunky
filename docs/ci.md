# CI

The GitHub Actions workflows: one for every pull request, one for releases, a nightly one, and the pieces they're
made of.

| Workflow | When | What | Where |
| --- | --- | --- | --- |
| `pr.yml` | every pull request | `scripts/check` and `scripts/web --check`, the updater's tests, then `pr-ok` | the Mac minis, and GitHub's machines for the updater |
| `release.yml` | a `v*` tag, or by hand | the suite, the oracle fidelity run, the web and native builds, the updater's tests, then the GitHub Release | the Mac minis and `ubuntu-24.04` |
| `build-native.yml` | from `release.yml`, or by hand | the native apps: built, signed for a release, smoke-tested ([docs/releasing.md](releasing.md)) | a Mac mini, then each OS |
| `updater.yml` | from `pr.yml` and `release.yml`, or by hand | the updater's tests, `cargo test -p update` ([docs/updates.md](updates.md)) | GitHub-hosted macOS, Windows and Linux |
| `mini.yml` | from `pr.yml` and `release.yml` | one command on a Mac mini | the Mac minis |
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
  the toolchain `rust-toolchain.toml` pins and Node into the runner's tool cache, where later jobs find them.
  The checkout's `target/` is a link to a cargo target dir kept there too, so builds are incremental across
  jobs; the action wipes it once it outgrows 12 GiB (the minis have about 30 GB free), and
  `cleanup.sh` (the last step of each job) drops the extracted assets and other per-job output. A job on
  the minis is a call to `.github/workflows/mini.yml` with the command to run. The native builds
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

## Pull requests

The repository is public, so `pr.yml` runs untrusted code on our own machines. It uses `pull_request` (never
`pull_request_target`), a read-only token, no secrets and no environment, and doesn't leave the token in the
self-hosted checkout (`persist-credentials: false`). A new push to a pull request cancels its previous run.

The updater's tests (`updater.yml`) run on every pull request, whatever it changes (the app reaches the
updater from outside `crates/update` too), on GitHub's machines alongside the minis' jobs, and `pr-ok` needs
them. They take two or three minutes, less than the test suite, so they usually add no time to the run,
unless GitHub's macOS arm64 machines are queueing (the notice under [Warnings](#warnings)). Like every other
check, they don't re-run on `main` after a merge; a release runs them again before it publishes.

The minis also sign releases, so code from outside never runs on them. Only collaborators may open pull
requests (the repository's `pull_request_creation_policy` is `collaborators_only`), runs from outside
contributors wait for a maintainer's approval (the Actions setting "Require approval for all external
contributors"), and `pr.yml` itself skips its mini jobs for a pull request from a fork and fails `pr-ok`, so
keeping a fork's code off the minis doesn't rest on those settings alone.

A pull request's code can still change what the minis keep between jobs (the toolchain, the target dir), so
nothing a release publishes is built from those caches: the web build runs on a fresh GitHub-hosted machine.

## Releases

The port's releases are numbered from v0.1.0 (tags `v0.x.y`), so they're never confused with the original
game's 1.1. First, if the night's [chaos soak](#the-chaos-soak) didn't run on the commit to release, run it by
hand and check it files nothing new. Then push a tag, `git tag v0.1.0 && git push origin v0.1.0`, and `release.yml`:

1. creates a draft GitHub Release for the tag (or reuses its draft on a re-run; a published release is never
   touched);
2. runs `scripts/check` on a mini;
3. runs every committed replay on the oracle and the port with `--fields` (`scripts/fidelity-all`), split
   across 10 `ubuntu-24.04` machines;
4. builds the web version from scratch on `ubuntu-24.04`, `spelunky-web-TAG.tar.gz` (`scripts/web
   --package`), and checks it the way the site's build will (`site/build.sh --local`);
5. builds, signs, notarizes and smoke-tests the native apps (`build-native.yml`, [docs/releasing.md](releasing.md)):
   all six built on a mini, each file then played on its own OS and architecture, and runs the updater's
   tests (`updater.yml`), as a pull request does;
6. signs the native apps' update manifest, `latest.json`, for those downloads (`scripts/release/manifest`,
   [docs/updates.md](updates.md)), except for a prerelease, which is never offered to the apps;
7. once all of that passed, attaches the files, `latest.json` and a `SHA256SUMS` to the draft and publishes
   it: as a prerelease if the tag has a `-` (`v0.1.0-rc1`), otherwise as the latest release.

Then put the web build and the update live with `scripts/bump-web-release TAG`
([docs/web-deploy.md](web-deploy.md)).

Running `release.yml` by hand (Actions → Release → Run workflow) on a tag does the same. With `dry_run`, the
default, it runs on any branch and builds and checks everything but signs, notarizes and publishes nothing:
the files are the run's artifacts. Signing uses the `release` environment's secrets (the Apple ones and
`UPDATE_SIGNING_KEY`); only the jobs that sign reference it, and `release.yml` passes them into
`build-native.yml` with `secrets: inherit` (a called workflow's job gets none of its environment's secrets
without it). Releases are automatic: the environment has no required reviewer, only deployment rules that
allow `main` and `v*` tags (a release runs on its tag), and only collaborators can push either.

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
`main`, starts at 1451: seeds 1-1450 were swept before), so a re-run plays the same seeds and no two runs share
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
a header saying where it came from (`fuzz-goal-ice-seed-55-ufo-aliens.txt` is one), and records its
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
