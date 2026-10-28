//! Prints what happens in a replay on the port, and checks its story facts (docs/formats.md, "Stories"):
//! for writing a replay's `# story:` lines and checking its header by eye.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use harness::fingerprint::{Suite, shown};
use harness::story::{Timeline, Watcher};
use harness::{RunOptions, load_assets, load_replay_story, run_replay_watched};
use tools::{Arg, Args, write_stdout};

const USAGE: &str = "\
usage: replay-story [options] <replay.txt>...

Plays each replay on the port (as fidelity-check does, no oracle) and prints what happens in it: the
rooms entered (with global.currLevel), the player's held item and death, each global that changed (those
that changed often only counted), and the frames instances of each object appeared on (+) and went away
on (-) without the room changing. Then it checks the replay's story facts (its `# story:` lines).

options:
  --check         only check the story facts
  --watch <names> also list every change of these (comma-separated): globals, or oX.var for the first
                  active instance of oX (oPlayer1.state, or a built-in: oPlayer1.y, oPlayer1.sprite_index)
  --assets <dir>  the asset bundle (default: assets/ in the repo; the settings program's is <dir>/config,
                  for replays in fidelity/settings/)
  -h, --help

exit status: 0 every story holds, 1 some fact failed, 2 bad usage or I/O";

struct Options {
    replays: Vec<PathBuf>,
    check: bool,
    watch: Vec<String>,
    assets: PathBuf,
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Option<Options>, String> {
    let mut o = Options {
        replays: Vec::new(),
        check: false,
        watch: Vec::new(),
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
            "--check" => {
                args.switch(&flag)?;
                o.check = true;
            }
            "--watch" => {
                o.watch = (args.value(&flag)?.split(','))
                    .filter(|n| !n.is_empty())
                    .map(str::to_string)
                    .collect();
            }
            "--assets" => o.assets = args.value(&flag)?.into(),
            f => return Err(format!("unknown option {f}")),
        }
    }
    if o.replays.is_empty() {
        return Err("no replay given".into());
    }
    Ok(Some(o))
}

fn run() -> Result<bool, String> {
    let Some(o) = parse_args(std::env::args().skip(1)).map_err(|e| format!("{e}\n\n{USAGE}"))?
    else {
        write_stdout(&format!("{USAGE}\n"))?;
        return Ok(true);
    };
    let repo = std::fs::canonicalize(repo_root()).map_err(|e| format!("the repository: {e}"))?;
    let mut ok = true;
    let mut bundles: [Option<std::sync::Arc<gm::Assets>>; 2] = [None, None];
    for path in &o.replays {
        let (replay, story) = load_replay_story(path)?;
        // The settings program's replays live in fidelity/settings/, and run on its own assets.
        let settings = Suite::of(&repo, path).is_ok_and(|s| s.settings);
        let assets = match &mut bundles[usize::from(settings)] {
            Some(a) => a.clone(),
            slot => slot
                .insert(load_assets(&if settings {
                    o.assets.join("config")
                } else {
                    o.assets.clone()
                })?)
                .clone(),
        };
        let mut watcher = Watcher::new(&story);
        let mut timeline = (!o.check).then(|| {
            let mut t = Timeline::new();
            t.watch(&o.watch);
            t
        });
        let mut program = None;
        let summary = run_replay_watched(
            &replay,
            assets,
            &RunOptions {
                settings,
                ..RunOptions::default()
            },
            &mut |frame, w| {
                watcher.frame(frame, w);
                if let Some(t) = &mut timeline {
                    t.frame(frame, w);
                }
                program.get_or_insert(w.program());
            },
        )?;
        let mut out = format!("== {}: {}\n", shown(&repo, path), summary.to_text());
        if let (Some(t), Some(p)) = (&timeline, program) {
            out += &t.to_text(p);
        }
        match watcher.finish() {
            Err(e) => {
                ok = false;
                out += &format!("story: {e}\n");
            }
            // Not a failure here, where facts are written, but fidelity-check's for a committed replay.
            Ok(_) if story.is_empty() => {
                out += "story: no facts yet (fidelity-check requires them of a committed replay)\n";
            }
            Ok(failed) if failed.is_empty() => {
                out += &format!("story: {} facts hold\n", story.facts.len());
            }
            Ok(failed) => {
                ok = false;
                for f in failed {
                    out += &format!("story failed: {f}\n");
                }
            }
        }
        write_stdout(&out)?;
    }
    Ok(ok)
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            eprintln!("replay-story: {e}");
            ExitCode::from(2)
        }
    }
}
