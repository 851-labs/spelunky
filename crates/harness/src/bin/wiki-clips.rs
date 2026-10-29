//! Checks and renders the wiki's clips (`wiki/clips/`, [`harness::clip`]; docs/wiki-media.md, "Clips"):
//! plays each clip's source on the port and checks its story facts, then has the app capture the frames
//! (`app --capture-from`, or `app --settings` for the settings program's), mixes the game's sound over them (`Clip::sound`) and has ffmpeg encode them.
//! `scripts/check` runs `--check`, which needs neither the app's GPU nor ffmpeg.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::{Arc, OnceLock};

use harness::clip::{Clip, Source, all_specs};
use harness::fingerprint::shown;
use harness::story::Watcher;
use harness::{RunOptions, RunOutcome, load_bundle, load_sounds, run_replay_watched};
use serde::Serialize;
use tools::{Arg, Args, WIKI_AUDIO_CREDIT, WIKI_LICENSE_URL, WIKI_NOTICE, write_stdout};

const USAGE: &str = "\
usage: wiki-clips [options] [<spec.toml>...]

Checks each clip spec (by default every one in wiki/clips/): plays its source (a committed replay, or a
staged scene's level and inputs, or a settings program replay from fidelity/settings/) on the port,
headless, as the app plays it, and checks its story facts. Then it renders them into --out: for each clip
an MP4 (H.264, AAC) and a WebM (VP9, Opus) of its frames in the program's own pixels times its scale, with the game's sound and music as the app plays them, a poster
PNG, and clip.json; and index.json, listing them all. A render of every spec also removes the clips whose
spec is gone. docs/wiki-media.md has the spec format and the layout.

options:
  --check               only check the specs and their facts: no app, no ffmpeg
  --replays <dir>       also write each scene's replay there (<entity>-<move>.txt), for replay-story or
                        scripts/fidelity
  --media <dir>         with --check: also check that every spec has a render there, with the spec's
                        title, frames and the rest, and that no clip there is without a spec
  --out <dir>           where to render (default: wiki/media/clips in the repo)
  --reuse <dir>         keep the videos of a clip rendered there whose frames, sound and encoding are
                        the same, rather than encoding them again (it may be --out)
  --jobs <n>            check or render n clips at a time (default 8)
  --app <path>          the app to capture with (default: target/release/app in the repo)
  --ffmpeg <path>       ffmpeg (default: ffmpeg on the PATH; scripts/wiki-media passes the pinned one)
  --assets <dir>        the asset bundle (default: assets/ in the repo; the settings program's is <dir>/config)
  -h, --help

exit status: 0 every spec is good (and rendered), 1 some spec isn't, 2 bad usage, I/O or a tool failed";

/// Bumped when the metadata's shape changes, so the site can tell. 2: no `port`, and each clip's `encoding`.
const FORMAT: u32 = 2;
/// GM's steps a second, which a clip plays at.
const FPS: u64 = 30;
/// The game's picture in its own pixels, which the app's capture is a whole multiple of.
const GAME_SIZE: (u32, u32) = (320, 240);

struct Options {
    specs: Vec<PathBuf>,
    check: bool,
    replays: Option<PathBuf>,
    media: Option<PathBuf>,
    out: PathBuf,
    reuse: Option<PathBuf>,
    jobs: usize,
    app: PathBuf,
    ffmpeg: PathBuf,
    assets: PathBuf,
}

/// Without `..`s, so the paths it prints and writes into the metadata are plain.
fn repo_root() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::canonicalize(&root).unwrap_or(root)
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Option<Options>, String> {
    let repo = repo_root();
    let mut o = Options {
        specs: Vec::new(),
        check: false,
        replays: None,
        media: None,
        out: repo.join("wiki/media/clips"),
        reuse: None,
        jobs: 8,
        app: repo.join("target/release/app"),
        ffmpeg: "ffmpeg".into(),
        assets: repo.join("assets"),
    };
    let mut args = Args::new(args);
    while let Some(arg) = args.next() {
        let flag = match arg {
            Arg::Positional(p) => {
                o.specs.push(PathBuf::from(p));
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
            "--replays" => o.replays = Some(args.value(&flag)?.into()),
            "--media" => o.media = Some(args.value(&flag)?.into()),
            "--out" => o.out = args.value(&flag)?.into(),
            "--reuse" => o.reuse = Some(args.value(&flag)?.into()),
            "--jobs" => {
                o.jobs = (args.value(&flag)?.parse().ok())
                    .filter(|&n| n > 0)
                    .ok_or("--jobs takes a whole number above 0")?;
            }
            "--app" => o.app = args.value(&flag)?.into(),
            "--ffmpeg" => o.ffmpeg = args.value(&flag)?.into(),
            "--assets" => o.assets = args.value(&flag)?.into(),
            f => return Err(format!("unknown option {f}")),
        }
    }
    Ok(Some(o))
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            eprintln!("wiki-clips: {e}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<bool, String> {
    let Some(o) = parse_args(std::env::args().skip(1)).map_err(|e| format!("{e}\n\n{USAGE}"))?
    else {
        write_stdout(&format!("{USAGE}\n"))?;
        return Ok(true);
    };
    let repo = std::fs::canonicalize(repo_root()).map_err(|e| format!("the repository: {e}"))?;
    let paths = if o.specs.is_empty() {
        all_specs(&repo)?
    } else {
        // As absolute paths, so the id comes out the same from anywhere.
        (o.specs.iter())
            .map(|p| std::fs::canonicalize(p).map_err(|e| format!("{}: {e}", p.display())))
            .collect::<Result<_, _>>()?
    };
    if paths.is_empty() {
        return Err("no clip specs".into());
    }
    let mut bundle = load_bundle(&o.assets)?;
    // The app models the sound DLL with the game's sounds, so the check does too: it's the run the clip shows.
    let sounds = load_sounds(&o.assets, &std::mem::take(&mut bundle.sounds))?;
    let programs = Programs {
        dir: o.assets.clone(),
        game: Arc::new(gm::Assets::from_bundle(bundle)),
        settings: OnceLock::new(),
    };

    if let Some(dir) = &o.replays {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let results = in_parallel(&paths, o.jobs, |path| {
        let at = shown(&repo, path);
        let result = Clip::read(&repo, path).and_then(|clip| {
            let source = clip.source(&repo)?;
            // Before checking it, so a scene whose facts fail can be looked into (replay-story).
            if let (Some(dir), Some(_)) = (&o.replays, &clip.spec.scene) {
                let path = dir.join(format!("{}.txt", clip.id.replace('/', "-")));
                std::fs::write(&path, &source.text)
                    .map_err(|e| format!("{}: {e}", path.display()))?;
            }
            let size = check(&clip, &source, programs.of(&source)?, sounds.clone())?;
            Ok(Checked { clip, source, size })
        });
        let line = match &result {
            Ok(_) => format!("ok {at}\n"),
            Err(e) => format!("FAILED {at}: {e}\n"),
        };
        write_stdout(&line).map(|()| result)
    });
    let mut checked = Vec::new();
    let mut ok = true;
    for result in results {
        match result? {
            Ok(c) => checked.push(c),
            Err(_) => ok = false,
        }
    }
    if !ok {
        write_stdout("some clip specs failed: docs/wiki-media.md, \"Clips\", has the format\n")?;
        return Ok(false);
    }
    if o.check {
        let Some(media) = &o.media else {
            return Ok(true);
        };
        let wrong = check_media(media, &repo, &checked, o.specs.is_empty())?;
        for w in &wrong {
            write_stdout(&format!("STALE {w}\n"))?;
        }
        if !wrong.is_empty() {
            write_stdout(&format!(
                "{} isn't what the specs render: scripts/wiki-media renders them (docs/wiki-media.md)\n",
                shown(&repo, media)
            ))?;
        }
        return Ok(wrong.is_empty());
    }
    render_all(&o, &repo, &checked, &programs, &sounds)?;
    Ok(true)
}

/// A clip whose story holds, and what it plays.
struct Checked {
    clip: Clip,
    source: Source,
    /// Its picture in the program's own pixels ([`check`]).
    size: (u32, u32),
}

/// The programs' assets: the game's, and the settings program's (`<assets>/config`), loaded for the first
/// clip of it.
struct Programs {
    dir: PathBuf,
    game: Arc<gm::Assets>,
    settings: OnceLock<Result<Arc<gm::Assets>, String>>,
}

impl Programs {
    /// The assets `source` plays on.
    fn of(&self, source: &Source) -> Result<Arc<gm::Assets>, String> {
        if !source.settings {
            return Ok(self.game.clone());
        }
        let load = || load_bundle(&self.dir.join("config")).map(gm::Assets::from_bundle);
        self.settings.get_or_init(|| load().map(Arc::new)).clone()
    }
}

/// `f` of each item, `jobs` at a time, in the items' order.
fn in_parallel<T: Sync, R: Send>(items: &[T], jobs: usize, f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results: Vec<std::sync::Mutex<Option<R>>> =
        items.iter().map(|_| std::sync::Mutex::new(None)).collect();
    std::thread::scope(|scope| {
        for _ in 0..jobs.min(items.len()) {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(item) = items.get(i) else { break };
                    let result = f(item);
                    *results[i].lock().unwrap_or_else(|e| e.into_inner()) = Some(result);
                }
            });
        }
    });
    (results.into_iter())
        .map(|r| {
            let r = r.into_inner().unwrap_or_else(|e| e.into_inner());
            r.expect("every item has a result once the threads are done")
        })
        .collect()
}

/// What `--check --media` holds the committed render to, without the app's capture: each spec's clip.json
/// has what the spec says (everything but the hashes of its frames and sound and of how they were encoded,
/// which only a render can tell), its files are there and a scene's replay is the scene's; and with `every`
/// spec, no other clip is there and index.json lists exactly these. What it finds wrong.
fn check_media(
    media: &Path,
    repo: &Path,
    clips: &[Checked],
    every: bool,
) -> Result<Vec<String>, String> {
    if !media.is_dir() {
        return Ok(vec![format!(
            "{}: no rendered clips at all",
            media.display()
        )]);
    }
    let mut wrong = Vec::new();
    for Checked { clip, source, size } in clips {
        let dir = media.join(&clip.id);
        let path = dir.join("clip.json");
        let Ok(text) = std::fs::read_to_string(&path) else {
            wrong.push(format!("{}: not rendered", clip.id));
            continue;
        };
        let got: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        let want = serde_json::to_value(clip_meta(repo, clip, *size, Rendered::default()))
            .map_err(|e| e.to_string())?;
        for (key, value) in want.as_object().into_iter().flatten() {
            if !RENDERED_FIELDS.contains(&key.as_str()) && got.get(key) != Some(value) {
                wrong.push(format!("{}: clip.json's {key} isn't the spec's", clip.id));
            }
        }
        for file in ["clip.mp4", "clip.webm", "poster.png"] {
            if !dir.join(file).is_file() {
                wrong.push(format!("{}: no {file}", clip.id));
            }
        }
        if clip.spec.scene.is_some()
            && std::fs::read_to_string(dir.join("replay.txt"))
                .ok()
                .as_deref()
                != Some(source.text.as_str())
        {
            wrong.push(format!("{}: replay.txt isn't the scene's", clip.id));
        }
    }
    if every {
        let ids: std::collections::BTreeSet<&str> =
            clips.iter().map(|c| c.clip.id.as_str()).collect();
        let mut metas = Vec::new();
        collect_metas(media, &mut metas)?;
        for meta in &metas {
            let id = meta["id"].as_str().unwrap_or("?");
            if !ids.contains(id) {
                wrong.push(format!("{id}: rendered, but has no spec"));
            }
        }
        let index = std::fs::read_to_string(media.join("index.json")).unwrap_or_default();
        let listed: std::collections::BTreeSet<String> =
            (serde_json::from_str::<serde_json::Value>(&index).ok())
                .and_then(|i| i["clips"].as_array().cloned())
                .into_iter()
                .flatten()
                .filter_map(|c| c["id"].as_str().map(str::to_owned))
                .collect();
        if listed.iter().map(String::as_str).ne(ids.iter().copied()) {
            wrong.push("index.json: doesn't list exactly the specs' clips".into());
        }
    }
    Ok(wrong)
}

/// Plays the clip's source up to its last frame on its program's `assets`, as the app does (the sound DLL
/// modelled with the game's sounds, which the settings program plays too), and checks its story facts.
/// What size its picture is in the program's own pixels: the game's 320x240, or the settings program's
/// window, whose view ports draw at their own resolution, twice the views' (sprites and backgrounds show
/// detail finer than a room pixel), and whose size differs between its rooms (rInit's 416x600,
/// rKeyConfig's 320x240), so its clips each stay in rooms of one size.
fn check(
    clip: &Clip,
    source: &Source,
    assets: Arc<gm::Assets>,
    sounds: Arc<game::audio::SoundBank>,
) -> Result<(u32, u32), String> {
    let (first, last) = clip.run_frames(source);
    let mut watcher = Watcher::new(&source.story);
    let mut sizes = std::collections::BTreeSet::new();
    let summary = run_replay_watched(
        &source.replay,
        assets,
        &RunOptions {
            frames: Some(last + 1),
            sounds: Some(sounds),
            settings: source.settings,
            ..RunOptions::default()
        },
        &mut |frame, w| {
            watcher.frame(frame, w);
            if source.settings && frame >= first {
                sizes.insert(w.window_size());
            }
        },
    )?;
    if summary.outcome != RunOutcome::Completed || summary.last_frame != Some(last) {
        return Err(format!(
            "the run stopped before the clip's last frame: {}",
            summary.to_text()
        ));
    }
    let failed = watcher.finish()?;
    if !failed.is_empty() {
        return Err(format!("story facts failed: {}", failed.join("; ")));
    }
    if !source.settings {
        return Ok(GAME_SIZE);
    }
    match Vec::from_iter(sizes)[..] {
        [size] => Ok(size),
        ref sizes => Err(format!(
            "the clip's rooms have windows of {} sizes ({sizes:?}): a clip is one size",
            sizes.len()
        )),
    }
}

/// `clip.json`, and the clip's entry in `index.json`. Paths are relative to the output directory.
#[derive(Serialize)]
struct ClipMeta {
    id: String,
    title: String,
    description: String,
    /// The spec, relative to the repo.
    spec: String,
    source: SourceMeta,
    /// The source's first and last frames shown, inclusive (a scene's count from its first frame in the
    /// level), and how many.
    from: u64,
    to: u64,
    frames: u64,
    fps: u64,
    /// In seconds.
    duration: f64,
    width: u32,
    height: u32,
    /// Output pixels per game pixel.
    scale: u32,
    #[serde(rename = "loop")]
    looped: bool,
    /// The source frame the poster shows.
    poster_frame: u64,
    /// `harness::fingerprint::hash` of every frame's pixels in game pixels (RGB, before scaling): the same
    /// for the same port and assets, however ffmpeg encodes them.
    frames_hash: String,
    /// Whether the videos have a sound track: the game made a sound or played music during the clip.
    has_audio: bool,
    /// `harness::fingerprint::hash` of the sound as mixed (stereo `f32` samples, little-endian, at the
    /// mixer's rate), before ffmpeg encodes it; `None` without sound.
    audio_hash: Option<String>,
    /// `harness::fingerprint::hash` of everything else the videos are encoded from: ffmpeg's `-version`, the
    /// codecs' options and the files' metadata. With the two above, what decides the videos' bytes, so a
    /// render with `--reuse` keeps the videos of a clip whose three are the same.
    encoding: String,
    files: Files,
    notice: &'static str,
    license: &'static str,
    caption: &'static str,
}

/// What only a render can tell of a clip, for its clip.json.
#[derive(Default)]
struct Rendered {
    frames_hash: String,
    has_audio: bool,
    audio_hash: Option<String>,
    encoding: String,
}

/// `Rendered`'s fields in clip.json, which `--check --media` can't check.
const RENDERED_FIELDS: [&str; 4] = ["frames_hash", "has_audio", "audio_hash", "encoding"];

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SourceMeta {
    /// Cut from a committed replay, relative to the repo.
    Replay { replay: String },
    /// A staged scene: its custom level and inputs are in the spec; `replay` is the replay that plays it.
    Scene { seed: i32 },
}

#[derive(Serialize)]
struct Files {
    mp4: String,
    webm: String,
    poster: String,
    /// A scene's replay (what it plays, from boot).
    #[serde(skip_serializing_if = "Option::is_none")]
    replay: Option<String>,
}

#[derive(Serialize)]
struct Index {
    format: u32,
    notice: &'static str,
    license: &'static str,
    caption: &'static str,
    /// The credit a page shows when a clip it plays has sound (docs/wiki-license.md).
    audio_credit: &'static str,
    fps: u64,
    /// Each clip's `clip.json`, by id.
    clips: Vec<serde_json::Value>,
}

/// The videos' `comment` (docs/wiki-license.md). It names no version of the port: the committed clips would
/// change with every commit (the commit that rendered a clip is in the repository's history).
const CAPTION: &str = "Recorded from the spelunky.ai port, an unofficial port (a mod) of Spelunky Classic \
                       1.1. Spelunky \u{a9} 2008, 2009 Derek Yu and Mossmouth, LLC.";

fn render_all(
    o: &Options,
    repo: &Path,
    clips: &[Checked],
    programs: &Programs,
    sounds: &Arc<game::audio::SoundBank>,
) -> Result<(), String> {
    // Before replacing anything. Not the app's --help: that's a usage error, which can open a dialog.
    if !o.app.is_file() {
        return Err(format!(
            "no app at {} (scripts/wiki-media builds it)",
            o.app.display()
        ));
    }
    // Which ffmpeg it is, for each clip's `encoding`: another build can encode the same frames differently.
    let ffmpeg = Command::new(&o.ffmpeg)
        .arg("-version")
        .stderr(Stdio::null())
        .output();
    let ffmpeg = match ffmpeg {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).into_owned(),
        _ => {
            return Err(format!(
                "can't run {} (scripts/wiki-media passes the pinned one)",
                o.ffmpeg.display()
            ));
        }
    };
    // Every spec: no clip whose spec is gone stays (below). Only in a render's directory, though: a
    // mistyped --out could name anything.
    if o.specs.is_empty() && o.out.exists() {
        let empty = std::fs::read_dir(&o.out).is_ok_and(|mut d| d.next().is_none());
        if !empty && !o.out.join("index.json").is_file() {
            return Err(format!(
                "{} has no index.json, so it isn't a wiki-clips render: not replacing it",
                o.out.display()
            ));
        }
    }
    let rendered = in_parallel(clips, o.jobs, |checked| {
        let assets = programs.of(&checked.source)?;
        let meta = render(o, repo, checked, &ffmpeg, assets, sounds.clone())?;
        write_stdout(&format!(
            "rendered {} ({} frames, {}x{}{}{})\n",
            checked.clip.id,
            meta.frames,
            meta.width,
            meta.height,
            if meta.has_audio { ", with sound" } else { "" },
            if meta.reused { ", its videos kept" } else { "" },
        ))
    });
    rendered.into_iter().collect::<Result<(), String>>()?;
    let _ = std::fs::remove_dir_all(o.out.join(SCRATCH));
    if o.specs.is_empty() {
        let ids = clips.iter().map(|c| c.clip.id.as_str()).collect();
        prune(&o.out, &o.out, &ids)?;
    }
    // Of every clip in the directory, so a render of some specs leaves the others listed.
    let mut metas = Vec::new();
    collect_metas(&o.out, &mut metas)?;
    metas.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    let index = Index {
        format: FORMAT,
        notice: WIKI_NOTICE,
        license: WIKI_LICENSE_URL,
        caption: CAPTION,
        audio_credit: WIKI_AUDIO_CREDIT,
        fps: FPS,
        clips: metas,
    };
    write_json(&o.out.join("index.json"), &index)?;
    write_stdout(&format!("{}\n", o.out.display()))
}

/// Removes what's under `dir` (in the render `out`) but isn't one of the clips `ids`: a clip whose spec is
/// gone, a directory left empty, and what a render that stopped left behind (its scratch frames and sound).
fn prune(out: &Path, dir: &Path, ids: &std::collections::BTreeSet<&str>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for entry in entries {
        let path = entry.map_err(|e| format!("{}: {e}", dir.display()))?.path();
        let gone = |e: std::io::Error| format!("{}: {e}", path.display());
        let id = path
            .strip_prefix(out)
            .map_or(String::new(), |p| p.to_string_lossy().into_owned());
        if path
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with('.'))
        {
            if path.is_dir() {
                std::fs::remove_dir_all(&path).map_err(gone)?;
            } else {
                std::fs::remove_file(&path).map_err(gone)?;
            }
        } else if path.is_dir() {
            if path.join("clip.json").is_file() && !ids.contains(id.as_str()) {
                std::fs::remove_dir_all(&path).map_err(gone)?;
            } else {
                prune(out, &path, ids)?;
                if std::fs::read_dir(&path).is_ok_and(|mut d| d.next().is_none()) {
                    std::fs::remove_dir(&path).map_err(gone)?;
                }
            }
        }
    }
    Ok(())
}

/// Every `clip.json` under `dir`, but in a dot directory (a render's `.scratch`).
fn collect_metas(dir: &Path, out: &mut Vec<serde_json::Value>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for entry in entries {
        let path = entry.map_err(|e| format!("{}: {e}", dir.display()))?.path();
        if path
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with('.'))
        {
            continue;
        }
        if path.is_dir() {
            collect_metas(&path, out)?;
        } else if path.file_name().is_some_and(|n| n == "clip.json") {
            let text =
                std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            out.push(serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?);
        }
    }
    Ok(())
}

/// The clip.json a render of `clip` writes, given its picture's `size` in the program's pixels and what the
/// render found.
fn clip_meta(repo: &Path, clip: &Clip, (gw, gh): (u32, u32), rendered: Rendered) -> ClipMeta {
    let spec = &clip.spec;
    let rel = |name: &str| format!("{}/{name}", clip.id);
    let count = spec.to - spec.from + 1;
    ClipMeta {
        id: clip.id.clone(),
        title: spec.title.clone(),
        description: spec.description.clone(),
        spec: shown(repo, &clip.path),
        source: match &spec.replay {
            Some(r) => SourceMeta::Replay { replay: r.clone() },
            None => SourceMeta::Scene {
                seed: spec.scene.as_ref().map_or(0, |s| s.seed),
            },
        },
        from: spec.from,
        to: spec.to,
        frames: count,
        fps: FPS,
        duration: count as f64 / FPS as f64,
        width: gw * spec.scale,
        height: gh * spec.scale,
        scale: spec.scale,
        looped: spec.looped,
        poster_frame: spec.poster,
        frames_hash: rendered.frames_hash,
        has_audio: rendered.has_audio,
        audio_hash: rendered.audio_hash,
        encoding: rendered.encoding,
        files: Files {
            mp4: rel("clip.mp4"),
            webm: rel("clip.webm"),
            poster: rel("poster.png"),
            replay: spec.replay.is_none().then(|| rel("replay.txt")),
        },
        notice: WIKI_NOTICE,
        license: WIKI_LICENSE_URL,
        caption: CAPTION,
    }
}

/// Whether the clip rendered in `dir` (`--reuse`'s) has videos encoded from what `meta`'s would be: the same
/// frames, sound and encoding.
fn reusable(dir: &Path, meta: &ClipMeta) -> bool {
    let Some(earlier) = (std::fs::read(dir.join("clip.json")).ok())
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
    else {
        return false;
    };
    let field = |key: &str| earlier.get(key).cloned().unwrap_or_default();
    field("frames_hash") == meta.frames_hash.as_str()
        && field("audio_hash") == serde_json::json!(meta.audio_hash)
        && field("encoding") == meta.encoding.as_str()
        && field("scale") == meta.scale
        && dir.join("clip.mp4").is_file()
        && dir.join("clip.webm").is_file()
}

/// Renders a clip into its directory under `--out`: its videos (or an earlier render's, with `--reuse`, if
/// they'd come out the same), poster, scene replay and clip.json. All of it is made in a scratch directory
/// of its own (`.scratch/<id>`) and replaces the clip's directory only once it's done, so a render that fails
/// leaves the clip as it was, and `--reuse` may be `--out`. What it rendered.
fn render(
    o: &Options,
    repo: &Path,
    Checked { clip, source, size }: &Checked,
    ffmpeg: &str,
    assets: Arc<gm::Assets>,
    sounds: Arc<game::audio::SoundBank>,
) -> Result<Done, String> {
    let spec = &clip.spec;
    let size @ (gw, gh) = *size;
    fn io(path: &Path) -> impl Fn(std::io::Error) -> String + '_ {
        move |e| format!("{}: {e}", path.display())
    }
    let work = o.out.join(SCRATCH).join(&clip.id);
    let _ = std::fs::remove_dir_all(&work);
    let new = work.join("clip");
    std::fs::create_dir_all(&new).map_err(io(&new))?;
    let replay_path = match &spec.replay {
        Some(r) => repo.join(r),
        None => {
            let path = new.join("replay.txt");
            std::fs::write(&path, &source.text).map_err(io(&path))?;
            path
        }
    };

    // The app's pictures, one PNG a frame.
    let (first, last) = clip.run_frames(source);
    let shots = work.join("frames");
    let mut app = Command::new(&o.app);
    if source.settings {
        app.arg("--settings");
    }
    let status = app
        .arg("--mute")
        .arg("--assets")
        .arg(&o.assets)
        .arg("--replay")
        .arg(&replay_path)
        .args(["--frames", &(last + 1).to_string()])
        .args(["--capture-from", &first.to_string()])
        .arg("--screenshot")
        .arg(&shots)
        .status()
        .map_err(|e| format!("{}: {e}", o.app.display()))?;
    if !status.success() {
        return Err(format!("{}: the app failed ({status})", clip.id));
    }
    let mut frames = Vec::new();
    for f in first..=last {
        let path = shots.join(format!("{f:06}.png"));
        frames.push(game_pixels(&path, size)?);
    }
    let _ = std::fs::remove_dir_all(&shots);

    let scale = spec.scale;
    let (w, h) = (gw * scale, gh * scale);
    // Frame by frame, then the frames' hashes: the frames together are up to 140 MB.
    let hashes: Vec<u8> = (frames.iter())
        .flat_map(|f| harness::fingerprint::hash(f).to_le_bytes())
        .collect();
    let frames_hash = format!("{:016x}", harness::fingerprint::hash(&hashes));

    // The sound over the same frames; none if it's silent throughout.
    let sound = clip.sound(source, assets, sounds)?;
    let has_audio = sound.iter().any(|&s| s != 0.0);
    let pcm: Vec<u8> = sound.iter().flat_map(|s| s.to_le_bytes()).collect();
    let audio_hash = has_audio.then(|| format!("{:016x}", harness::fingerprint::hash(&pcm)));

    let copyright = if has_audio {
        format!("{WIKI_NOTICE} {WIKI_AUDIO_CREDIT}")
    } else {
        WIKI_NOTICE.to_owned()
    };
    let metadata = [
        format!("title={}", spec.title),
        format!("copyright={copyright}"),
        format!("comment={CAPTION}"),
    ];
    // Everything ffmpeg is told but where its files are.
    let encoding = {
        let mut what = ffmpeg.to_owned();
        for (_, video, audio) in &ENCODES {
            let sound = has_audio.then_some(("SOUND".as_ref(), *audio));
            for arg in ffmpeg_args(size, scale, sound, video, &metadata, "OUT".as_ref()) {
                what.push('\n');
                what.push_str(&arg.to_string_lossy());
            }
        }
        format!("{:016x}", harness::fingerprint::hash(what.as_bytes()))
    };
    let meta = clip_meta(
        repo,
        clip,
        size,
        Rendered {
            frames_hash,
            has_audio,
            audio_hash,
            encoding,
        },
    );

    let earlier = o.reuse.as_ref().map(|d| d.join(&clip.id));
    let reused = earlier.filter(|d| reusable(d, &meta));
    if let Some(earlier) = &reused {
        // A copy-on-write clone on APFS (std's copy uses fclonefileat).
        for file in ["clip.mp4", "clip.webm"] {
            std::fs::copy(earlier.join(file), new.join(file)).map_err(io(&earlier.join(file)))?;
        }
    } else {
        // ffmpeg reads the sound from a file beside the frames it's piped.
        let sound_file = work.join("sound.f32");
        if has_audio {
            std::fs::write(&sound_file, &pcm).map_err(io(&sound_file))?;
        }
        for (ext, video, audio) in &ENCODES {
            let sound = has_audio.then_some((sound_file.as_path(), *audio));
            let out = new.join(format!("clip.{ext}"));
            encode(o, &frames, size, scale, sound, video, &metadata, &out)?;
        }
    }

    let still = upscale(&frames[(spec.poster - spec.from) as usize], gw, gh, scale);
    write_poster(&new.join("poster.png"), &still, w, h)?;
    write_json(&new.join("clip.json"), &meta)?;
    // Done: in place of the clip's directory.
    let dir = o.out.join(&clip.id);
    match std::fs::remove_dir_all(&dir) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(io(&dir)(e)),
        _ => {}
    }
    if let Some(parent) = dir.parent() {
        std::fs::create_dir_all(parent).map_err(io(parent))?;
    }
    std::fs::rename(&new, &dir).map_err(io(&dir))?;
    let _ = std::fs::remove_dir_all(&work);
    Ok(Done {
        frames: meta.frames,
        width: w,
        height: h,
        has_audio,
        reused: reused.is_some(),
    })
}

/// Where `render` makes each clip before it replaces the clip's directory, under `--out`.
const SCRATCH: &str = ".scratch";

/// What `render` reports of a clip.
struct Done {
    frames: u64,
    width: u32,
    height: u32,
    has_audio: bool,
    reused: bool,
}

/// The two videos: each file's extension, its video codec's options and its audio codec's. H.264 for every
/// browser, at a quality where the pixels stay crisp; its index first (faststart), so a page can start
/// playing before the whole file is in. VP9 in WebM, smaller where it's supported. 4:2:0 colour, which every
/// player takes: the scale is even, so it never mixes two game pixels. The sound in each container's usual
/// codec: AAC in MP4, Opus in WebM.
///
/// x264 has a fixed number of threads: its output depends on how many it has, which is otherwise 1.5 a core,
/// so the same clip would come out differently on a machine with another number of cores (libvpx's doesn't,
/// with row-mt). With them fixed, the pinned ffmpeg makes the same bytes on every Mac (docs/wiki-media.md,
/// "Committing").
const ENCODES: [(&str, &[&str], &[&str]); 2] = [
    (
        "mp4",
        &[
            "-c:v",
            "libx264",
            "-preset",
            "veryslow",
            "-tune",
            "animation",
            "-crf",
            "18",
            "-pix_fmt",
            "yuv420p",
            "-threads",
            "8",
            "-movflags",
            "+faststart",
        ],
        &["-c:a", "aac", "-b:a", "128k"],
    ),
    (
        "webm",
        &[
            "-c:v",
            "libvpx-vp9",
            "-crf",
            "30",
            "-b:v",
            "0",
            "-pix_fmt",
            "yuv420p",
            "-row-mt",
            "1",
        ],
        &["-c:a", "libopus", "-b:a", "96k"],
    ),
];

/// A captured picture (the app's RGB PNG: the game's 320x240 times an odd number, its `screenScale`, or the
/// settings program's window as it is) in the program's own pixels, `size`: each block's centre. The program draws its surface scaled up, so every block must be one colour, or something would be
/// lost.
fn game_pixels(path: &Path, size: (u32, u32)) -> Result<Vec<u8>, String> {
    let at = |e: String| format!("{}: {e}", path.display());
    let file = std::fs::File::open(path).map_err(|e| at(e.to_string()))?;
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .map_err(|e| at(e.to_string()))?;
    let mut buf = vec![0; reader.output_buffer_size().ok_or("PNG too large")?];
    let info = reader.next_frame(&mut buf).map_err(|e| at(e.to_string()))?;
    if (info.color_type, info.bit_depth) != (png::ColorType::Rgb, png::BitDepth::Eight) {
        return Err(at(format!(
            "expected 8-bit RGB, got {:?} {:?}",
            info.color_type, info.bit_depth
        )));
    }
    let (gw, gh) = size;
    let k = info.width / gw;
    if k == 0 || info.width != gw * k || info.height != gh * k {
        return Err(at(format!(
            "{}x{} isn't the program's {gw}x{gh} times a whole number (its window's scale)",
            info.width, info.height
        )));
    }
    let stride = info.line_size;
    let (k, gw, gh) = (k as usize, gw as usize, gh as usize);
    let mut out = Vec::with_capacity(gw * gh * 3);
    for y in 0..gh {
        for x in 0..gw {
            let px = |dx: usize, dy: usize| {
                let i = (y * k + dy) * stride + (x * k + dx) * 3;
                &buf[i..i + 3]
            };
            let c = px(k / 2, k / 2);
            if (0..k).any(|dy| (0..k).any(|dx| px(dx, dy) != c)) {
                return Err(at(format!(
                    "the {k}x{k} block of game pixel {x},{y} isn't one colour"
                )));
            }
            out.extend_from_slice(c);
        }
    }
    Ok(out)
}

/// Nearest-neighbour upscaling of an RGB picture by a whole number.
fn upscale(rgb: &[u8], w: u32, h: u32, scale: u32) -> Vec<u8> {
    let (w, h, s) = (w as usize, h as usize, scale as usize);
    let mut out = Vec::with_capacity(rgb.len() * s * s);
    for y in 0..h {
        let row = &rgb[y * w * 3..(y + 1) * w * 3];
        let mut scaled = Vec::with_capacity(row.len() * s);
        for px in row.as_chunks::<3>().0 {
            for _ in 0..s {
                scaled.extend_from_slice(px);
            }
        }
        for _ in 0..s {
            out.extend_from_slice(&scaled);
        }
    }
    out
}

/// ffmpeg's arguments for encoding the frames (RGB of `size` at `scale`, from its stdin) at 30 fps, given the video
/// codec's options, and the sound if there is any (`Clip::sound`'s samples in a file of raw `f32`s, and the
/// audio codec's options), into `out`, with the notice in the file's metadata.
fn ffmpeg_args(
    size: (u32, u32),
    scale: u32,
    sound: Option<(&Path, &[&str])>,
    video: &[&str],
    metadata: &[String],
    out: &Path,
) -> Vec<std::ffi::OsString> {
    let (gw, gh) = size;
    let (w, h) = (gw * scale, gh * scale);
    let mut args: Vec<std::ffi::OsString> = Vec::new();
    let mut add = |more: &[&str]| args.extend(more.iter().map(Into::into));
    add(&[
        "-loglevel",
        "error",
        "-y",
        "-f",
        "rawvideo",
        "-pix_fmt",
        "rgb24",
    ]);
    add(&["-s", &format!("{w}x{h}"), "-framerate", &FPS.to_string()]);
    add(&["-i", "-"]);
    match sound {
        Some((path, codec)) => {
            let rate = audio::offline::SAMPLE_RATE.to_string();
            add(&["-f", "f32le", "-ar", &rate, "-ac", "2", "-i"]);
            args.push(path.into());
            let mut add = |more: &[&str]| args.extend(more.iter().map(Into::into));
            add(&["-map", "0:v", "-map", "1:a"]);
            add(codec);
            add(&["-flags:a", "+bitexact"]);
        }
        // Silent: no sound track at all, rather than a silent one.
        None => add(&["-an"]),
    }
    // No encoder version or creation time in the container, which would change the bytes for nothing.
    let mut add = |more: &[&str]| args.extend(more.iter().map(Into::into));
    add(&[
        "-map_metadata",
        "-1",
        "-fflags",
        "+bitexact",
        "-flags:v",
        "+bitexact",
    ]);
    add(video);
    for m in metadata {
        add(&["-metadata", m]);
    }
    args.push(out.into());
    args
}

/// Encodes the frames (RGB in the program's pixels, `size`, upscaled by `scale` one at a time) with ffmpeg, as `ffmpeg_args`
/// says. The frames and the sound are the same every time, and so are the bytes the pinned ffmpeg makes of
/// them (docs/wiki-media.md, "Committing").
#[allow(clippy::too_many_arguments)]
fn encode(
    o: &Options,
    frames: &[Vec<u8>],
    size @ (gw, gh): (u32, u32),
    scale: u32,
    sound: Option<(&Path, &[&str])>,
    video: &[&str],
    metadata: &[String],
    out: &Path,
) -> Result<(), String> {
    let mut child = Command::new(&o.ffmpeg)
        .args(ffmpeg_args(size, scale, sound, video, metadata, out))
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| format!("{}: {e}", o.ffmpeg.display()))?;
    let mut stdin = child.stdin.take().ok_or("ffmpeg has no stdin")?;
    for f in frames {
        // A failed write means ffmpeg stopped; its status says why.
        if stdin.write_all(&upscale(f, gw, gh, scale)).is_err() {
            break;
        }
    }
    drop(stdin);
    let status = child.wait().map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("ffmpeg failed on {} ({status})", out.display()))
    }
}

/// The poster: one frame as a PNG, opaque, with the notice in a `Copyright` text chunk like the sprites'.
fn write_poster(path: &Path, rgb: &[u8], w: u32, h: u32) -> Result<(), String> {
    let rgba: Vec<u8> = (rgb.as_chunks::<3>().0.iter())
        .flat_map(|&[r, g, b]| [r, g, b, 255])
        .collect();
    let mut out = Vec::new();
    let mut encoder = tools::png_encoder(&mut out, w, h);
    encoder
        .add_text_chunk("Copyright".to_owned(), WIKI_NOTICE.to_owned())
        .map_err(|e| e.to_string())?;
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(&rgba).map_err(|e| e.to_string())?;
    writer.finish().map_err(|e| e.to_string())?;
    std::fs::write(path, out).map_err(|e| format!("{}: {e}", path.display()))
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let mut out = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    out.push(b'\n');
    std::fs::write(path, out).map_err(|e| format!("{}: {e}", path.display()))
}
