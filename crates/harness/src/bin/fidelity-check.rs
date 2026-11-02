//! Checks every committed replay on the port against its fingerprint of the oracle's run
//! (`fidelity/fingerprints/`), without the oracle, and that the run still shows the story its header
//! declares (`# story:` lines, [`harness::story`]): what `scripts/check` runs so that a change can't break
//! fidelity, or take a replay away from what it's there to show, silently. Every committed replay needs a
//! story, so a new one can't be added without saying what it shows. `scripts/fidelity
//! --refresh-fingerprints` records the fingerprints.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

use harness::fingerprint::{
    Fingerprint, Fingerprinter, SUITES, Suite, all_replays, faithful_audio_mismatch,
    fingerprint_path, oracle_build_stamp, oracle_stamp_path, oracle_stamp_problem, port_mismatch,
    recorded_oracle_stamp, replay_hash, run_fingerprint_watched, shown, sorted_with_extension, sound_handle,
};
use harness::story::{Story, Watcher};
use harness::{load_bundle, load_replay_story, load_sounds};
use tools::{Arg, Args, write_stdout};
use trace::Replay;

const USAGE: &str = "\
usage: fidelity-check [options] [<replay.txt>...]

Plays each committed replay (by default all of fidelity/replays/ and fidelity/settings/) on the port,
headless, as `fidelity --fields` does, and checks the run against the replay's fingerprint of the
oracle's (fidelity/fingerprints/): its trace, checkpoint by checkpoint, and its host files. It also
checks the replay's story facts (its `# story:` lines, docs/formats.md), so that a change that takes
port and oracle somewhere new together doesn't leave a header describing what no longer happens. A
replay without a story fact that must hold is an error. It doesn't need the oracle or Docker. Without replays, a
fingerprint whose replay is gone is an error too. So is an oracle build (tools/oracle/common.sh's build_stamp)
other than the one the fingerprints were recorded with (fidelity/fingerprints/ORACLE), as an oracle change can
make them stale while the port still matches them.

The replays of AUDIO_REPLAYS (music in every area, the pause, Olmec, the ending, the credits, a custom level,
the bonus rooms) are also played in the audio mode players get, Faithful, with the game's sounds, which must
play them as the oracle's stub mode does but for the sound handles (global.snd*, global.mus*), the only state
the two may differ in (docs/quirks.md §7).

options:
  --jobs <n>      replays played at once (default: one per core)
  --assets <dir>  the asset bundle (default: assets/ in the repo; the settings program's is <dir>/config)
  -h, --help

exit status: 0 every replay matches and its story holds, 1 some replay doesn't, or has no fingerprint or
no story, 2 bad usage or I/O";

struct Options {
    replays: Vec<PathBuf>,
    jobs: usize,
    assets: PathBuf,
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Option<Options>, String> {
    let mut o = Options {
        replays: Vec::new(),
        jobs: std::thread::available_parallelism().map_or(4, |n| n.get()),
        assets: repo_root().join("assets"),
    };
    let mut args = Args::new(args);
    while let Some(arg) = args.next() {
        let flag = match arg {
            Arg::Positional(p) => {
                o.replays.push(PathBuf::from(p));
                continue;
            }
            Arg::Flag(flag) => flag,
        };
        match flag.name.as_str() {
            "-h" | "--help" => {
                args.switch(&flag)?;
                return Ok(None);
            }
            "--jobs" => o.jobs = args.number::<usize>(&flag)?.max(1),
            "--assets" => o.assets = args.value(&flag)?.into(),
            f => return Err(format!("unknown option {f}")),
        }
    }
    Ok(Some(o))
}

/// The replays also played in Faithful audio, which must play them as the oracle's stub mode does
/// (851-3792): between them, every area's music and its changes, the pause, Olmec and the ending, the
/// credits, a custom level's, the bonus rooms and the high scores room.
const AUDIO_REPLAYS: [&str; 7] = [
    "ending-seed-1",
    "mines-seed-1",
    "flow-pause-dying-quit-seed-1",
    "custom-chain-end",
    "bonus-stars-shotgun",
    "flow-highscores-moon-room-slow-win",
    "title-credits-to-the-end",
];

/// Whether the replay is one of [`AUDIO_REPLAYS`].
fn audio_checked(job: &Job) -> bool {
    !job.suite.settings
        && (job.path.file_stem().and_then(|s| s.to_str()))
            .is_some_and(|s| AUDIO_REPLAYS.contains(&s))
}

/// A replay to check.
struct Job {
    path: PathBuf,
    suite: &'static Suite,
    replay: Replay,
    story: Story,
}

/// What came of one replay: `None` if it matched, else what to say.
type Outcome = (PathBuf, Option<String>);

/// What to say about a replay whose story doesn't say what happens in it ([`Story::tells`]), which every
/// committed replay's must: without that, a change that takes port and oracle somewhere new together could
/// leave its header describing what no longer happens.
fn untold(job: &Job, repo: &Path) -> Option<String> {
    (!job.story.tells()).then(|| {
        let what = if job.story.is_empty() {
            "no story facts"
        } else {
            "no story fact that must hold, only ones that mustn't"
        };
        format!(
            "{what}: add `# story:` facts to its header that say what it shows (docs/formats.md, \
             \"Stories\"; replay-story {} prints what happens in it)",
            shown(repo, &job.path)
        )
    })
}

/// `sounds`: the game's, for Faithful audio's run of one of [`AUDIO_REPLAYS`].
fn check(
    job: &Job,
    assets: Arc<gm::Assets>,
    repo: &Path,
    sounds: Option<&Arc<game::audio::SoundBank>>,
) -> Option<String> {
    let fp_path = fingerprint_path(repo, job.suite, &job.path);
    let replay = shown(repo, &job.path);
    if !fp_path.exists() {
        return Some(format!(
            "no fingerprint at {}; record one with scripts/fidelity --refresh-fingerprints {replay}",
            shown(repo, &fp_path)
        ));
    }
    let recorded = match Fingerprint::read(&fp_path) {
        Ok(fp) => fp,
        Err(e) => return Some(e),
    };
    // Before playing it: a long replay takes a while to find out.
    if recorded.replay != replay_hash(&job.replay) {
        return Some(format!(
            "the replay changed since its fingerprint was recorded; record it again with \
             scripts/fidelity --refresh-fingerprints {replay}"
        ));
    }
    let mut watcher = Watcher::new(&job.story);
    // For one of AUDIO_REPLAYS, the same run fingerprinted again without the sound handles, to hold
    // Faithful audio's run to.
    let sounds = sounds.filter(|_| audio_checked(job));
    let mut quiet = sounds.map(|_| Fingerprinter::skipping_globals(sound_handle));
    let (port, summary) = match run_fingerprint_watched(
        &job.replay,
        assets.clone(),
        job.suite.settings,
        &mut |frame, w| {
            watcher.frame(frame, w);
            if let Some(q) = &mut quiet {
                q.world(w);
            }
        },
    ) {
        Ok(r) => r,
        Err(e) => return Some(e),
    };
    let mut problems = Vec::new();
    if let (Some(quiet), Some(sounds)) = (quiet, sounds) {
        // The host files are the run's own, which the skipped globals don't touch.
        let stub = Fingerprint {
            host: port.host.clone(),
            ..quiet.finish(&job.replay, None)
        };
        match faithful_audio_mismatch(&stub, &summary, &job.replay, assets, sounds.clone()) {
            Ok(None) => {}
            Ok(Some(m)) | Err(m) => problems.push(m),
        }
    }
    if let Some(m) = port_mismatch(&recorded, &port, &summary) {
        let settings = if job.suite.settings {
            " --settings"
        } else {
            ""
        };
        problems.push(format!(
            "{m}. For the exact frame: scripts/fidelity --fields{settings} {replay}"
        ));
    }
    let story = match watcher.finish() {
        Ok(failed) => failed,
        Err(e) => vec![e],
    };
    if !story.is_empty() {
        problems.push(format!(
            "the run no longer shows its story: {}. Fix the header and its story, or re-make the replay \
             (replay-story {replay} shows what happens)",
            story.join("; ")
        ));
    }
    (!problems.is_empty()).then(|| problems.join("\n  "))
}

fn run() -> Result<bool, String> {
    let Some(o) = parse_args(std::env::args().skip(1)).map_err(|e| format!("{e}\n\n{USAGE}"))?
    else {
        write_stdout(&format!("{USAGE}\n"))?;
        return Ok(true);
    };
    let started = Instant::now();
    let repo = std::fs::canonicalize(repo_root()).map_err(|e| format!("the repository: {e}"))?;
    let mut problems: Vec<Outcome> = Vec::new();
    // The oracle's patches, Dockerfile, pinned commits and math crate are the fingerprints' as recorded
    // (851-3791): a pull request that changes them can't pass on fingerprints the old oracle made.
    let stamp = oracle_build_stamp(&repo)?;
    if let Some(p) = oracle_stamp_problem(recorded_oracle_stamp(&repo)?.as_deref(), &stamp) {
        problems.push((oracle_stamp_path(&repo), Some(p)));
    }
    let paths: Vec<(PathBuf, &'static Suite)> = if o.replays.is_empty() {
        for suite in &SUITES {
            // A fingerprint left behind by a deleted or renamed replay.
            let fp_dir = repo.join("fidelity/fingerprints").join(suite.name);
            for fp in sorted_with_extension(&fp_dir, "json")? {
                let replay = suite
                    .dir(&repo)
                    .join(fp.with_extension("txt").file_name().unwrap_or_default());
                if !replay.exists() {
                    problems.push((
                        fp,
                        Some(format!(
                            "no replay {}: delete the fingerprint",
                            shown(&repo, &replay)
                        )),
                    ));
                }
            }
        }
        all_replays(&repo)?
    } else {
        (o.replays.iter())
            .map(|p| Ok((p.clone(), Suite::of(&repo, p)?)))
            .collect::<Result<_, String>>()?
    };
    let mut jobs = paths
        .into_iter()
        .map(|(path, suite)| {
            let (replay, story) = load_replay_story(&path)?;
            Ok(Job {
                path,
                suite,
                replay,
                story,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    // A replay Faithful audio is checked on can't go without its check going too.
    if o.replays.is_empty() {
        for name in AUDIO_REPLAYS {
            if !jobs
                .iter()
                .any(|j| !j.suite.settings && j.path.file_stem() == Some(name.as_ref()))
            {
                problems.push((
                    SUITES[0].dir(&repo).join(format!("{name}.txt")),
                    Some(
                        "gone, but fidelity-check's AUDIO_REPLAYS still names it: name another"
                            .into(),
                    ),
                ));
            }
        }
    }
    // The longest first, so that one doesn't start last and hold everything up.
    // One of AUDIO_REPLAYS is played twice.
    jobs.sort_by_key(|j| std::cmp::Reverse(j.replay.len() * if audio_checked(j) { 2 } else { 1 }));
    let bundle = load_bundle(&o.assets)?;
    // The game's sounds, for Faithful audio's runs, if any replay checked has one.
    let sounds = if jobs.iter().any(audio_checked) {
        Some(load_sounds(&o.assets, &bundle.sounds)?)
    } else {
        None
    };
    let game = Arc::new(gm::Assets::from_bundle(bundle));
    let settings = if jobs.iter().any(|j| j.suite.settings) {
        Some(Arc::new(gm::Assets::from_bundle(load_bundle(
            &o.assets.join("config"),
        )?)))
    } else {
        None
    };
    let next = AtomicUsize::new(0);
    let done = Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..o.jobs.min(jobs.len()) {
            s.spawn(|| {
                while let Some(job) = jobs.get(next.fetch_add(1, Ordering::Relaxed)) {
                    let assets = if job.suite.settings {
                        settings.clone().expect("loaded for settings replays")
                    } else {
                        game.clone()
                    };
                    // One without a story is still played, for whatever else is wrong with it.
                    let found: Vec<_> = [
                        untold(job, &repo),
                        check(job, assets, &repo, sounds.as_ref()),
                    ]
                    .into_iter()
                    .flatten()
                    .collect();
                    let result = (!found.is_empty()).then(|| found.join("\n  "));
                    done.lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .push((job.path.clone(), result));
                }
            });
        }
    });
    let done = done.into_inner().unwrap_or_else(PoisonError::into_inner);
    let checked = done.len();
    let matched = done.iter().filter(|(_, m)| m.is_none()).count();
    problems.extend(done.into_iter().filter(|(_, m)| m.is_some()));
    problems.sort_by(|a, b| a.0.cmp(&b.0));
    let frames: u64 = jobs.iter().map(|j| j.replay.len()).sum();
    let facts: usize = jobs.iter().map(|j| j.story.facts.len()).sum();
    let mut out = String::new();
    for (path, m) in &problems {
        out += &format!(
            "{}: {}\n",
            shown(&repo, path),
            m.as_deref().unwrap_or_default()
        );
    }
    out += &format!(
        "fidelity-check: {matched} of {checked} replays ({frames} frames) match their fingerprints and \
         stories ({facts} story facts; {:.1} s)\n",
        started.elapsed().as_secs_f64()
    );
    write_stdout(&out)?;
    Ok(problems.is_empty())
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            eprintln!("fidelity-check: {e}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments() {
        let parse = |a: &[&str]| parse_args(a.iter().map(|s| s.to_string()));
        let o = parse(&["a.txt", "--jobs", "0", "b.txt"]).unwrap().unwrap();
        assert_eq!(o.replays, [PathBuf::from("a.txt"), PathBuf::from("b.txt")]);
        assert_eq!(o.jobs, 1);
        assert!(parse(&["--help"]).unwrap().is_none());
        assert!(parse(&["--frames", "3"]).is_err());
    }
}
