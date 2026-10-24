# CI

Two GitHub Actions workflows: one for every pull request, one for releases.

| Workflow | When | What | Where |
| --- | --- | --- | --- |
| `pr.yml` | every pull request | `scripts/check` and `scripts/web --check`, then `pr-ok` | the Mac minis |
| `release.yml` | a `v*` tag, or by hand | the suite, the oracle fidelity run, the web and native builds, then the GitHub Release | the Mac minis and `ubuntu-latest` |

`pr-ok` is the only check the "Protect main" ruleset requires. It fails unless every other job of the run
succeeded, so jobs can be added or renamed without touching the ruleset.

## Runners

- **The Mac minis**: the org's self-hosted runner group "Mac minis" (Apple Silicon, labels `self-hosted, macOS,
  ARM64, codevisor-macos-arm64`), shared with other repositories. They come with no Rust, Node or Docker, so
  `.github/actions/mini-toolchain` installs rustup (a pinned `rustup-init`, checked against its SHA-256),
  the toolchain `rust-toolchain.toml` pins and Node into the runner's tool cache, where later jobs find them.
  The checkout's `target/` is a link to a cargo target dir kept there too, so builds are incremental across
  jobs; the action wipes it once it outgrows 12 GiB (the minis have about 30 GB free), and
  `cleanup.sh` (the last step of each job) drops the extracted assets and other per-job output. A job on
  the minis is a call to `.github/workflows/mini.yml` with the command to run.
- **`ubuntu-latest`** (GitHub-hosted, free for public repositories) runs everything that needs Docker (the
  oracle) and builds the web version a release publishes.

## Pull requests

The repository is public, so `pr.yml` runs untrusted code on our own machines. It uses `pull_request` (never
`pull_request_target`), a read-only token, no secrets and no environment, and doesn't leave the token in the
self-hosted checkout (`persist-credentials: false`). Runs from outside contributors wait for a maintainer's
approval (the repository's Actions setting "Require approval for all external contributors"). A new push to
a pull request cancels its previous run.

A pull request's code can still change what the minis keep between jobs (the toolchain, the target dir), so
nothing a release publishes is built from those caches: the web build runs on a fresh GitHub-hosted machine.

## Releases

The port's releases are numbered from v0.1.0 (tags `v0.x.y`), so they're never confused with the original
game's 1.1. Push a tag, `git tag v0.1.0 && git push origin v0.1.0`, and `release.yml`:

1. creates a draft GitHub Release for the tag (or reuses its draft on a re-run; a published release is never
   touched);
2. runs `scripts/check` on a mini;
3. runs every committed replay on the oracle and the port with `--fields` (`scripts/fidelity-all`), split
   across 10 `ubuntu-latest` machines;
4. builds the web version from scratch on `ubuntu-latest`, `spelunky-web-TAG.tar.gz` (`scripts/web
   --package`), and checks it the way the site's build will (`site/build.sh --local`);
5. builds, signs, notarizes and smoke-tests the native apps (`build-native.yml`; a placeholder until 851-3251,
   which fails a real release so none goes out without them);
6. signs the native apps' update manifest, `latest.json`, for those downloads (`scripts/release/manifest`,
   [docs/updates.md](updates.md)), except for a prerelease, which is never offered to the apps;
7. once all of that passed, attaches the files, `latest.json` and a `SHA256SUMS` to the draft and publishes
   it: as a prerelease if the tag has a `-` (`v0.1.0-rc1`), otherwise as the latest release.

Then put the web build and the update live with `scripts/bump-web-release TAG`
([docs/web-deploy.md](web-deploy.md)).

Running `release.yml` by hand (Actions → Release → Run workflow) on a tag does the same. With `dry_run`, the
default, it runs on any branch and builds and checks everything but signs, notarizes and publishes nothing:
the files are the run's artifacts. Signing uses the `release` environment's secrets (the Apple ones and
`UPDATE_SIGNING_KEY`); only the jobs that sign reference it, and each waits for a maintainer's approval there. A release runs on its tag, so the
environment's deployment rules must allow `v*` tags as well as `main`.

### The oracle on CI

The `oracle build` job caches the oracle's Docker image and its `gm8emulator` under the build stamp
(`tools/oracle/common.sh`), so the fidelity shards only `docker load` them. A new stamp (a patch, the
Dockerfile, the pinned commits or `vendor/pxfm`) rebuilds them, starting from the last build's cargo registry
and target volumes, which are cached too. The `port build` job builds the `fidelity` binary and extracts the
assets once for all shards. Each shard puts its traces on the runner's largest disk and deletes each replay's
once it's compared: with `--fields`, a long replay's two traces take gigabytes.
