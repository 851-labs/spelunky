//! Checks and renders the wiki's clips (`wiki/clips/`, [`harness::clip`]; docs/wiki-media.md, "Clips"):
//! plays each clip's source on the port and checks its story facts, then has the app capture the frames
//! (`app --capture-from`, with the copyright notice burned into the first ones) and ffmpeg encode them.
//! `scripts/check` runs `--check`, which needs neither the app's GPU nor ffmpeg.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::Arc;

use harness::clip::{Clip, NOTICE_FRAMES, Source, all_specs};
use harness::fingerprint::shown;
use harness::story::Watcher;
use harness::{RunOptions, RunOutcome, load_bundle, load_sounds, run_replay_watched};
use serde::Serialize;
use tools::{Arg, Args, WIKI_LICENSE_URL, WIKI_NOTICE, write_stdout};

const USAGE: &str = "\
usage: wiki-clips [options] [<spec.toml>...]

Checks each clip spec (by default every one in wiki/clips/): plays its source (a committed replay, or a
staged scene's level and inputs) on the port, headless, as the app plays it, and checks its story facts.
Then it renders them into --out, replacing it: for each clip an MP4 (H.264) and a WebM (VP9) of its
frames in the game's own pixels times its scale, with the copyright notice over the first two seconds, a
poster PNG, and clip.json; and index.json, listing them all. docs/wiki-media.md has the spec format and
the layout.

options:
  --check               only check the specs and their facts: no app, no ffmpeg
  --replays <dir>       also write each scene's replay there (<entity>-<move>.txt), for replay-story or
                        scripts/fidelity
  --out <dir>           where to render (default: target/wiki-media/clips in the repo)
  --app <path>          the app to capture with (default: target/release/app in the repo)
  --ffmpeg <path>       ffmpeg (default: ffmpeg on the PATH)
  --assets <dir>        the asset bundle (default: assets/ in the repo)
  --port-version <v>    the port's version for the caption (scripts/wiki-clips passes git describe's)
  -h, --help

exit status: 0 every spec is good (and rendered), 1 some spec isn't, 2 bad usage, I/O or a tool failed";

/// Bumped when the metadata's shape changes, so the site can tell.
const FORMAT: u32 = 1;
/// GM's steps a second, which a clip plays at.
const FPS: u64 = 30;
/// The game's picture in its own pixels, which the app's capture is a whole multiple of.
const GAME_SIZE: (u32, u32) = (320, 240);

struct Options {
    specs: Vec<PathBuf>,
    check: bool,
    replays: Option<PathBuf>,
    out: PathBuf,
    app: PathBuf,
    ffmpeg: PathBuf,
    assets: PathBuf,
    port_version: String,
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
        out: repo.join("target/wiki-media/clips"),
        app: repo.join("target/release/app"),
        ffmpeg: "ffmpeg".into(),
        assets: repo.join("assets"),
        port_version: "unreleased".into(),
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
            "--out" => o.out = args.value(&flag)?.into(),
            "--app" => o.app = args.value(&flag)?.into(),
            "--ffmpeg" => o.ffmpeg = args.value(&flag)?.into(),
            "--assets" => o.assets = args.value(&flag)?.into(),
            "--port-version" => o.port_version = args.value(&flag)?,
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
    let assets = Arc::new(gm::Assets::from_bundle(bundle));

    if let Some(dir) = &o.replays {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let mut ok = true;
    let mut checked = Vec::new();
    for path in &paths {
        let at = shown(&repo, path);
        let result = Clip::read(&repo, path).and_then(|clip| {
            let source = clip.source(&repo)?;
            // Before checking it, so a scene whose facts fail can be looked into (replay-story).
            if let (Some(dir), Some(_)) = (&o.replays, &clip.spec.scene) {
                let path = dir.join(format!("{}.txt", clip.id.replace('/', "-")));
                std::fs::write(&path, &source.text)
                    .map_err(|e| format!("{}: {e}", path.display()))?;
            }
            check(&clip, &source, assets.clone(), sounds.clone())?;
            Ok((clip, source))
        });
        match result {
            Ok(c) => {
                write_stdout(&format!("ok {at}\n"))?;
                checked.push(c);
            }
            Err(e) => {
                ok = false;
                write_stdout(&format!("FAILED {at}: {e}\n"))?;
            }
        }
    }
    if !ok {
        write_stdout("some clip specs failed: docs/wiki-media.md, \"Clips\", has the format\n")?;
        return Ok(false);
    }
    if o.check {
        return Ok(true);
    }
    render_all(&o, &repo, &checked)?;
    Ok(true)
}

/// Plays the clip's source up to its last frame, as the app does (the sound DLL modelled with the game's
/// sounds), and checks its story facts.
fn check(
    clip: &Clip,
    source: &Source,
    assets: Arc<gm::Assets>,
    sounds: Arc<game::audio::SoundBank>,
) -> Result<(), String> {
    let (_, last) = clip.run_frames(source);
    let mut watcher = Watcher::new(&source.story);
    let summary = run_replay_watched(
        &source.replay,
        assets,
        &RunOptions {
            frames: Some(last + 1),
            sounds: Some(sounds),
            ..RunOptions::default()
        },
        &mut |frame, w| watcher.frame(frame, w),
    )?;
    if summary.outcome != RunOutcome::Completed || summary.last_frame != Some(last) {
        return Err(format!(
            "the run stopped before the clip's last frame: {}",
            summary.to_text()
        ));
    }
    let failed = watcher.finish()?;
    if failed.is_empty() {
        Ok(())
    } else {
        Err(format!("story facts failed: {}", failed.join("; ")))
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
    /// How many of the first frames carry the copyright notice.
    notice_frames: u64,
    /// `harness::fingerprint::hash` of every frame's pixels in game pixels (RGB, before scaling): the same
    /// for the same port and assets, however ffmpeg encodes them.
    frames_hash: String,
    files: Files,
    notice: &'static str,
    license: &'static str,
    caption: String,
}

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
struct Index<'a> {
    format: u32,
    notice: &'static str,
    license: &'static str,
    /// The port's version the clips were recorded with, and the caption a page shows under each.
    port: &'a str,
    caption: String,
    fps: u64,
    clips: Vec<ClipMeta>,
}

/// The caption under every clip (docs/wiki-license.md).
fn caption(port: &str) -> String {
    format!(
        "Recorded from the spelunky.ai port {port}, an unofficial port (a mod) of Spelunky Classic 1.1. \
         Spelunky \u{a9} 2008, 2009 Derek Yu and Mossmouth, LLC."
    )
}

fn render_all(o: &Options, repo: &Path, clips: &[(Clip, Source)]) -> Result<(), String> {
    // Before replacing anything. Not the app's --help: that's a usage error, which can open a dialog.
    if !o.app.is_file() {
        return Err(format!(
            "no app at {} (scripts/wiki-clips builds it)",
            o.app.display()
        ));
    }
    let ffmpeg = Command::new(&o.ffmpeg)
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    if !ffmpeg.is_ok_and(|s| s.success()) {
        return Err(format!(
            "can't run {} (brew install ffmpeg)",
            o.ffmpeg.display()
        ));
    }
    match std::fs::remove_dir_all(&o.out) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            return Err(format!("{}: {e}", o.out.display()));
        }
        _ => {}
    }
    let mut metas = Vec::new();
    for (clip, source) in clips {
        let meta = render(o, repo, clip, source)?;
        write_stdout(&format!(
            "rendered {} ({} frames, {}x{})\n",
            clip.id, meta.frames, meta.width, meta.height
        ))?;
        metas.push(meta);
    }
    let index = Index {
        format: FORMAT,
        notice: WIKI_NOTICE,
        license: WIKI_LICENSE_URL,
        port: &o.port_version,
        caption: caption(&o.port_version),
        fps: FPS,
        clips: metas,
    };
    write_json(&o.out.join("index.json"), &index)?;
    write_stdout(&format!("{}\n", o.out.display()))
}

fn render(o: &Options, repo: &Path, clip: &Clip, source: &Source) -> Result<ClipMeta, String> {
    let spec = &clip.spec;
    let dir = o.out.join(&clip.id);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let rel = |name: &str| format!("{}/{name}", clip.id);
    let (replay_path, source_meta, replay_file) = match &spec.replay {
        Some(r) => (repo.join(r), SourceMeta::Replay { replay: r.clone() }, None),
        None => {
            let path = dir.join("replay.txt");
            std::fs::write(&path, &source.text).map_err(|e| format!("{}: {e}", path.display()))?;
            let seed = spec.scene.as_ref().map_or(0, |s| s.seed);
            (path, SourceMeta::Scene { seed }, Some(rel("replay.txt")))
        }
    };

    // The app's pictures, one PNG a frame, in a scratch directory beside the output.
    let (first, last) = clip.run_frames(source);
    let shots = o.out.join(format!(".frames-{}", clip.id.replace('/', "-")));
    let _ = std::fs::remove_dir_all(&shots);
    let status = Command::new(&o.app)
        .arg("--mute")
        .arg("--assets")
        .arg(&o.assets)
        .arg("--replay")
        .arg(&replay_path)
        .args(["--frames", &(last + 1).to_string()])
        .args(["--capture-from", &first.to_string()])
        .args(["--notice-frames", &NOTICE_FRAMES.to_string()])
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
        frames.push(game_pixels(&path, f - first >= NOTICE_FRAMES)?);
    }
    let _ = std::fs::remove_dir_all(&shots);

    let (gw, gh) = GAME_SIZE;
    let scale = spec.scale;
    let (w, h) = (gw * scale, gh * scale);
    let all: Vec<u8> = frames.concat();
    let frames_hash = format!("{:016x}", harness::fingerprint::hash(&all));
    let scaled: Vec<Vec<u8>> = frames.iter().map(|f| upscale(f, gw, gh, scale)).collect();

    let caption = caption(&o.port_version);
    let metadata = [
        format!("title={}", spec.title),
        format!("copyright={WIKI_NOTICE}"),
        format!("comment={caption}"),
    ];
    let mp4 = rel("clip.mp4");
    let webm = rel("clip.webm");
    // H.264 for every browser, at a quality where the pixels stay crisp; its index first (faststart), so
    // a page can start playing before the whole file is in. VP9 in WebM, smaller where it's supported.
    // 4:2:0 colour, which every player takes: the scale is even, so it never mixes two game pixels.
    let encodes: [(&str, &[&str]); 2] = [
        (
            &mp4,
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
                "-movflags",
                "+faststart",
            ],
        ),
        (
            &webm,
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
        ),
    ];
    for (file, codec) in encodes {
        encode(o, &scaled, (w, h), codec, &metadata, &o.out.join(file))?;
    }

    let poster = rel("poster.png");
    let index = (spec.poster - spec.from) as usize;
    write_poster(&o.out.join(&poster), &scaled[index], w, h)?;

    let count = spec.to - spec.from + 1;
    let meta = ClipMeta {
        id: clip.id.clone(),
        title: spec.title.clone(),
        description: spec.description.clone(),
        spec: shown(repo, &clip.path),
        source: source_meta,
        from: spec.from,
        to: spec.to,
        frames: count,
        fps: FPS,
        duration: count as f64 / FPS as f64,
        width: w,
        height: h,
        scale,
        looped: spec.looped,
        poster_frame: spec.poster,
        notice_frames: NOTICE_FRAMES.min(count),
        frames_hash,
        files: Files {
            mp4,
            webm,
            poster,
            replay: replay_file,
        },
        notice: WIKI_NOTICE,
        license: WIKI_LICENSE_URL,
        caption,
    };
    write_json(&dir.join("clip.json"), &meta)?;
    Ok(meta)
}

/// A captured picture (the app's RGB PNG, the game's 320x240 times an odd number) in the game's own
/// pixels: each block's centre. The game draws its 320x240 surface scaled up, so with `exact`, every block
/// must be one colour, or something would be lost. The notice is drawn over the scaled picture in game
/// pixels, but GM's half-pixel shift of every image (`app::render`) can put its text a screen pixel off
/// the blocks; the centre of an odd block is still in the right one.
fn game_pixels(path: &Path, exact: bool) -> Result<Vec<u8>, String> {
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
    let (gw, gh) = GAME_SIZE;
    let k = info.width / gw;
    if k % 2 == 0 || info.width != gw * k || info.height != gh * k {
        return Err(at(format!(
            "{}x{} isn't the game's {gw}x{gh} times an odd number (the replay's screenScale)",
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
            if exact && (0..k).any(|dy| (0..k).any(|dx| px(dx, dy) != c)) {
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

/// Encodes the frames (RGB, `size`) at 30 fps with ffmpeg, given the codec's options, into `out`, silent
/// and with the notice in the file's metadata. The frames are the same every time; the bytes ffmpeg makes
/// of them can differ between its versions.
fn encode(
    o: &Options,
    frames: &[Vec<u8>],
    (w, h): (u32, u32),
    codec: &[&str],
    metadata: &[String],
    out: &Path,
) -> Result<(), String> {
    let mut cmd = Command::new(&o.ffmpeg);
    cmd.args([
        "-loglevel",
        "error",
        "-y",
        "-f",
        "rawvideo",
        "-pix_fmt",
        "rgb24",
    ])
    .args(["-s", &format!("{w}x{h}"), "-framerate", &FPS.to_string()])
    .args(["-i", "-", "-an", "-map_metadata", "-1"])
    // No encoder version or creation time in the container, which would change the bytes for nothing.
    .args(["-fflags", "+bitexact", "-flags:v", "+bitexact"])
    .args(codec);
    for m in metadata {
        cmd.args(["-metadata", m]);
    }
    let mut child = cmd
        .arg(out)
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| format!("{}: {e}", o.ffmpeg.display()))?;
    let mut stdin = child.stdin.take().ok_or("ffmpeg has no stdin")?;
    for f in frames {
        // A failed write means ffmpeg stopped; its status says why.
        if stdin.write_all(f).is_err() {
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
