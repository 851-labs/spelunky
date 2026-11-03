//! The port's renderer against the oracle's pictures of chosen frames (851-3872, `harness::shots`).

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use harness::fingerprint::{oracle_build_stamp, replay_hash};
use harness::shots::{
    SOURCE, Shots, compare, read_list, read_png, screenshots_dir, select, source, write_png,
};
use tools::{Arg, Args, write_stdout};

const USAGE: &str = "\
usage: fidelity-shots [--assets <dir>] [--app <path>] [<replay.txt>...]

Renders the frames fidelity/screenshots/shots.txt lists of each replay (all of them by default) with the app,
headless (`app --replay R --shots F1,F2,... --screenshot DIR`), into target/fidelity-shots/<replay name>/, and
compares each with the oracle's picture of it, fidelity/screenshots/<replay name>/<frame>.png, within the
line's tolerance (exact by default). The pictures must be of the replay as it is, from the checkout's oracle
build (their source.txt), and none may be left that no line lists. Where they differ it writes <frame>.diff.png beside the port's picture:
the port's, dimmed where they agree, magenta where they don't. The pictures are recorded with
scripts/fidelity --refresh-screenshots.

options:
  --assets <dir>   the asset bundle (default: assets/ in the repo, from scripts/extract)
  --app <path>     the app to render with (default: the app beside this binary, as scripts/fidelity-shots
                   builds it)
  -h, --help

exit status: 0 every picture within its tolerance, 1 some aren't, 2 error";

struct Options {
    assets: Option<PathBuf>,
    app: Option<PathBuf>,
    replays: Vec<PathBuf>,
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Option<Options>, String> {
    let mut o = Options {
        assets: None,
        app: None,
        replays: Vec::new(),
    };
    let mut args = Args::new(args);
    while let Some(arg) = args.next() {
        let flag = match arg {
            Arg::Positional(p) => {
                o.replays.push(p.into());
                continue;
            }
            Arg::Flag(flag) => flag,
        };
        match flag.name.as_str() {
            "-h" | "--help" => {
                args.switch(&flag)?;
                return Ok(None);
            }
            "--assets" => o.assets = Some(args.value(&flag)?.into()),
            "--app" => o.app = Some(args.value(&flag)?.into()),
            f => return Err(format!("unknown option {f}")),
        }
    }
    Ok(Some(o))
}

/// The repository root: run from anywhere, as `scripts/fidelity-shots` does.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Renders and compares one line's frames; what's wrong, a line each.
fn check(
    repo: &Path,
    app: &Path,
    assets: &Path,
    stamp: &str,
    shots: &Shots,
) -> Result<Vec<String>, String> {
    let name = shots.name();
    let references = shots.dir(&screenshots_dir(repo));
    // The pictures must be of this replay, from this oracle build, as a fingerprint must.
    let replay = harness::load_replay(&repo.join(&shots.replay))?;
    let want = source(stamp, &replay_hash(&replay));
    let record = format!(
        "record them with scripts/fidelity --refresh-screenshots {}",
        shots.replay.display()
    );
    match std::fs::read_to_string(references.join(SOURCE)) {
        Ok(had) if had == want => {}
        Ok(_) => {
            return Ok(vec![format!(
                "{name}: the oracle's pictures are of another oracle build or another version of the replay; \
                 {record}"
            )]);
        }
        Err(_) => return Ok(vec![format!("{name}: no oracle's pictures; {record}")]),
    }
    let missing: Vec<String> = (shots.frames.iter())
        .filter(|f| !references.join(format!("{f}.png")).exists())
        .map(u64::to_string)
        .collect();
    if !missing.is_empty() {
        return Ok(vec![format!(
            "{name}: no oracle's picture of frame {}; {record}",
            missing.join(", ")
        )]);
    }
    let out = repo.join("target/fidelity-shots").join(&name);
    match std::fs::remove_dir_all(&out) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            return Err(format!("{}: {e}", out.display()));
        }
        _ => {}
    }
    let list: Vec<String> = shots.frames.iter().map(u64::to_string).collect();
    let mut cmd = Command::new(app);
    cmd.arg("--assets")
        .arg(assets)
        .arg("--replay")
        .arg(repo.join(&shots.replay))
        .arg("--shots")
        .arg(list.join(","))
        .arg("--screenshot")
        .arg(&out)
        .arg("--mute");
    if shots.settings() {
        cmd.arg("--settings");
    }
    let run = cmd
        .output()
        .map_err(|e| format!("{}: {e}", app.display()))?;
    if !run.status.success() {
        return Err(format!(
            "{name}: the app failed ({}): {}",
            run.status,
            String::from_utf8_lossy(&run.stderr).trim()
        ));
    }
    let mut problems = Vec::new();
    for f in &shots.frames {
        let oracle = read_png(&references.join(format!("{f}.png")))?;
        let port = read_png(&out.join(format!("{f}.png")))?;
        let (d, map) = compare(&oracle, &port).map_err(|e| format!("{name} frame {f}: {e}"))?;
        if d.pixels > 0 {
            let diff = out.join(format!("{f}.diff.png"));
            write_png(&diff, &map)?;
            if !d.within(shots.tolerance) {
                let (l, t, r, b) = d.bounds.expect("pixels differ");
                problems.push(format!(
                    "{name} frame {f}: {} pixels differ, by up to {} in a channel, in ({l},{t})-({r},{b}); \
                     where: {}",
                    d.pixels,
                    d.step,
                    diff.display()
                ));
            }
        }
    }
    Ok(problems)
}

fn run() -> Result<bool, String> {
    let Some(args) = parse_args(std::env::args().skip(1)).map_err(|e| format!("{e}\n\n{USAGE}"))?
    else {
        write_stdout(&format!("{USAGE}\n"))?;
        return Ok(true);
    };
    let repo = std::fs::canonicalize(repo_root()).map_err(|e| format!("the repository: {e}"))?;
    let app = match args.app {
        Some(app) => app,
        None => std::env::current_exe()
            .map_err(|e| format!("this binary: {e}"))?
            .with_file_name(format!("app{}", std::env::consts::EXE_SUFFIX)),
    };
    let assets = args.assets.unwrap_or_else(|| repo.join("assets"));
    let all = read_list(&repo)?;
    let mut problems = Vec::new();
    if args.replays.is_empty() {
        // Pictures no line asks for any more.
        let dir = screenshots_dir(&repo);
        let entries = std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        for e in entries {
            let e = e.map_err(|e| format!("{}: {e}", dir.display()))?;
            let name = e.file_name().to_string_lossy().into_owned();
            if e.path().is_dir() && !all.iter().any(|s| s.name() == name) {
                problems.push(format!(
                    "{name}: pictures that fidelity/screenshots/shots.txt doesn't list; delete them"
                ));
            }
        }
    }
    let list = select(all, &args.replays)?;
    let stamp = oracle_build_stamp(&repo)?;
    let mut frames = 0;
    for shots in &list {
        problems.extend(check(&repo, &app, &assets, &stamp, shots)?);
        frames += shots.frames.len();
    }
    for p in &problems {
        eprintln!("fidelity-shots: {p}");
    }
    eprintln!(
        "fidelity-shots: {} problems in {frames} pictures of {} replays",
        problems.len(),
        list.len()
    );
    Ok(problems.is_empty())
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            eprintln!("fidelity-shots: {e}");
            ExitCode::from(2)
        }
    }
}
