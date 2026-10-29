//! The wiki's clip specs (`wiki/clips/<entity>/<move>.toml`, docs/wiki-media.md, "Clips"): which frames of
//! which run a clip shows, and the story facts that say what happens in them. A clip's source is a
//! committed replay, or a staged scene: a custom level (`levels/*.lvl`, docs/source-inventory.md 7.3) the
//! player starts in, plus a few inputs, which [`Spec::source`] turns into a replay that boots the game and
//! loads the level from the title's load-level menu. A replay of the settings program (`fidelity/settings/`)
//! plays it, `config.exe`, rather than the game. `wiki-clips` checks the facts on the port
//! (`scripts/check` runs `--check`, without rendering), so a game change that takes a clip away from what
//! its page says fails like a replay's story would, and renders the clips (`scripts/wiki-clips`).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use audio::offline::{FRAMES_PER_TICK, Recorder};
use game::audio::{Mode, SoundBank};
use game::replay::Playback;
use serde::Deserialize;
use trace::Replay;

use crate::fingerprint::Suite;
use crate::story::{Fact, Story};

/// Where the specs live, relative to the repo.
pub const SPEC_DIR: &str = "wiki/clips";

/// The frames a clip may last: from 3 seconds to 20, so the bundle stays small.
pub const MIN_FRAMES: u64 = 90;
pub const MAX_FRAMES: u64 = 600;

/// The custom level a scene's replay puts in the game directory: the only one there, so it's the load-level
/// menu's first row.
const SCENE_LEVEL: &str = "wiki";

/// How a scene's replay gets into its level, from boot: the intro and the title skipped with the attack key
/// (X), F3 on the title opens the load-level menu (rLoadLevel), a click on its first row (the window is 3x,
/// the shipped `screenScale`) picks the level and Enter loads it, on frame 240 (`scrLoadLevel` creates the
/// level's objects in rLoadLevel itself). As `fidelity/replays/custom-chain-end.txt` does it.
const SCENE_BOOT: &str = "\
20 hold X 2
60 hold X 2
200 hold f3 2
228 mouse 180 348
230 hold mb_left 2
240 hold enter 2
";
/// The frame Enter loads the scene's level on.
const SCENE_LOADED: u64 = 240;
/// Scene frame 0: the first frame in the level, with the player in it.
pub const SCENE_START: u64 = SCENE_LOADED + 1;

/// A clip spec, as written.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    /// The clip's name, for a page and its `VideoObject`.
    pub title: String,
    /// What it shows, a sentence or two (the `VideoObject`'s description).
    pub description: String,
    /// A committed replay (`fidelity/replays/...txt`, relative to the repo) the clip is cut from, or one of
    /// the settings program's (`fidelity/settings/...txt`), which plays it instead of the game. Either this
    /// or `scene`.
    #[serde(default)]
    pub replay: Option<String>,
    /// A staged scene the clip plays. Either this or `replay`.
    #[serde(default)]
    pub scene: Option<Scene>,
    /// The first and last frames shown, inclusive, in the source's frames: the replay's, or the scene's,
    /// which count from the first frame in its level.
    pub from: u64,
    pub to: u64,
    /// The frame the poster shows (the still before the clip plays): after the notice's first frames, up to `to`.
    pub poster: u64,
    /// Output pixels per game pixel: 2 or 4. Even, so that H.264's and VP9's half-resolution colour (4:2:0)
    /// never mixes two game pixels.
    #[serde(default = "default_scale")]
    pub scale: u32,
    /// Whether the page should loop it.
    #[serde(default = "default_loop", rename = "loop")]
    pub looped: bool,
    /// What happens in it, as story facts (docs/formats.md, "Stories") in the source's frames: at least one
    /// that must hold. Each is checked on the clip's frames only, `from` to `to`.
    pub story: String,
}

fn default_scale() -> u32 {
    2
}

fn default_loop() -> bool {
    true
}

/// A staged scene: one monster and the player, say, in a custom level.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    /// The RNG seed at startup (`seed` in a replay).
    #[serde(default)]
    pub seed: i32,
    /// The level, as the level editor saves it: up to 32 rows of up to 40 tile codes (`scrCreateTileObj`:
    /// `1` a brick, `@` the entrance the player starts at, `n` a snake...), padded with `0` (nothing). It
    /// needs exactly one entrance.
    pub level: String,
    /// The player's hearts, bombs and ropes (the level's `lifeStart`, `bombStart`, `ropeStart`; the
    /// editor's default is 4).
    #[serde(default = "default_start")]
    pub lives: u32,
    #[serde(default = "default_start")]
    pub bombs: u32,
    #[serde(default = "default_start")]
    pub ropes: u32,
    /// Replay event lines (`F hold right 20`), in scene frames. `#` comments and blank lines are allowed.
    #[serde(default)]
    pub inputs: String,
}

fn default_start() -> u32 {
    4
}

/// A spec, checked, with its id (its path under [`SPEC_DIR`] without `.toml`: `snake/bite`).
#[derive(Clone, Debug)]
pub struct Clip {
    pub id: String,
    pub path: PathBuf,
    pub spec: Spec,
    /// The story facts, in the source's frames, each window cut to the clip's.
    pub story: Story,
}

/// What a clip plays: the replay, and where the clip's frames are in it.
#[derive(Clone, Debug)]
pub struct Source {
    pub replay: Replay,
    /// The replay as a file: the committed one, or for a scene, the one [`Spec::source`] made.
    pub text: String,
    /// The replay's frame for the source's frame 0: [`SCENE_START`] for a scene, 0 for a replay.
    pub offset: u64,
    /// Whether it plays the settings program (a replay in `fidelity/settings/`), on its own assets, rather
    /// than the game.
    pub settings: bool,
    /// The facts to check on the replay's run, in its frames, with a scene's that it loaded.
    pub story: Story,
}

impl Clip {
    /// Reads and checks the spec at `path`, which must be under `repo`'s [`SPEC_DIR`].
    pub fn read(repo: &Path, path: &Path) -> Result<Clip, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let id = clip_id(&repo.join(SPEC_DIR), path)?;
        Clip::parse(&id, path, &text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Checks a spec's text, `id` its name.
    pub fn parse(id: &str, path: &Path, text: &str) -> Result<Clip, String> {
        let spec: Spec = toml::from_str(text).map_err(|e| e.to_string())?;
        let story = Story::parse(&format!("# story: {}", spec.story))?;
        if !story.tells() {
            return Err("its story needs a fact that must hold".into());
        }
        if spec.title.trim().is_empty() || spec.description.trim().is_empty() {
            return Err("it needs a title and a description".into());
        }
        match (&spec.replay, &spec.scene) {
            (Some(_), None) | (None, Some(_)) => {}
            _ => return Err("it needs either `replay` or a `[scene]`, not both".into()),
        }
        if spec.from > spec.to {
            return Err(format!("from {} is after to {}", spec.from, spec.to));
        }
        let frames = spec.to - spec.from + 1;
        if !(MIN_FRAMES..=MAX_FRAMES).contains(&frames) {
            return Err(format!(
                "{frames} frames: a clip has {MIN_FRAMES} to {MAX_FRAMES} (3 to 20 seconds)"
            ));
        }
        if !(spec.from..=spec.to).contains(&spec.poster) {
            return Err(format!(
                "poster {} isn't in the clip's frames {}..={}",
                spec.poster, spec.from, spec.to
            ));
        }
        // The facts are about what the clip shows: each is checked on its frames only.
        let mut story = story;
        for fact in &mut story.facts {
            let (a, b) = fact.when.unwrap_or((0, u64::MAX));
            let (a, b) = (a.max(spec.from), b.min(spec.to));
            if a > b {
                return Err(format!(
                    "{}: its window is outside the clip's frames, {}..={}",
                    fact.text, spec.from, spec.to
                ));
            }
            fact.when = Some((a, b));
        }
        if !matches!(spec.scale, 2 | 4) {
            return Err(format!("scale {}: 2 or 4", spec.scale));
        }
        if let Some(scene) = &spec.scene {
            level_file(scene)?;
        }
        Ok(Clip {
            id: id.to_owned(),
            path: path.to_owned(),
            spec,
            story,
        })
    }

    /// The frames shown, in the source replay's frames.
    pub fn run_frames(&self, source: &Source) -> (u64, u64) {
        (source.offset + self.spec.from, source.offset + self.spec.to)
    }

    /// The clip's sound: what the app's player plays over the frames shown, mixed offline
    /// ([`audio::offline`]), a tick of interleaved stereo samples per frame. Each frame's sound starts with
    /// its picture, as the app hands the player a tick's commands right after the tick. The mix runs from
    /// boot, so what was already playing at the first frame (the level's music, a sound half done) is
    /// there, from where it had got to.
    pub fn sound(
        &self,
        source: &Source,
        assets: Arc<gm::Assets>,
        sounds: Arc<SoundBank>,
    ) -> Result<Vec<f32>, String> {
        let (first, last) = self.run_frames(source);
        let mut config = game::Config::from_replay(&source.replay.header);
        config.audio = Mode::Faithful;
        config.sounds = sounds.clone();
        let failed = |e: game::replay::RunError| format!("{}: {e}", self.id);
        let exe = if source.settings {
            config::executable()
        } else {
            game::spelunky()
        };
        let mut playback =
            Playback::boot_of(exe, assets, config, &source.replay).map_err(failed)?;
        playback.stop_before(last + 1);
        let mut recorder = Recorder::new(sounds);
        // Unlike the app, which warns and plays on, a sound the player can't play fails the clip, which
        // would be without it.
        let apply = |recorder: &mut Recorder, p: &Playback| {
            // Peeked, not taken: each step drops the last one's.
            recorder
                .apply(p.game().audio().model().commands())
                .map_err(|e| format!("{}: {e}", self.id))
        };
        // What booting loaded or started, as the app's player gets it before frame 0.
        apply(&mut recorder, &playback)?;
        let ticks = (last - first + 1) as usize;
        let mut out = Vec::with_capacity(ticks * FRAMES_PER_TICK * 2);
        let mut before = Vec::with_capacity(FRAMES_PER_TICK * 2);
        while let Some(frame) = playback.step().map_err(failed)? {
            apply(&mut recorder, &playback)?;
            if frame < first {
                before.clear();
                recorder.mix_tick(&mut before);
            } else {
                recorder.mix_tick(&mut out);
            }
        }
        if out.len() != ticks * FRAMES_PER_TICK * 2 {
            return Err(format!(
                "{}: the run stopped before the clip's last frame",
                self.id
            ));
        }
        Ok(out)
    }

    /// The replay the clip plays and the facts to check on it. A replay spec's is read from `repo`.
    pub fn source(&self, repo: &Path) -> Result<Source, String> {
        let spec = &self.spec;
        let (text, offset, settings, mut story) = match (&spec.replay, &spec.scene) {
            (Some(path), _) => {
                let file = repo.join(path);
                let text = std::fs::read_to_string(&file)
                    .map_err(|e| format!("{}: {e}", file.display()))?;
                // As fidelity-check plays it: by its directory.
                let settings = Suite::of(repo, &file).is_ok_and(|s| s.settings);
                (text, 0, settings, self.story.clone())
            }
            (None, Some(scene)) => {
                let text = scene_replay(&self.id, spec, scene)?;
                let mut story = self.story.clone();
                for fact in &mut story.facts {
                    fact.when = fact.when.map(|(a, b)| (a + SCENE_START, b + SCENE_START));
                }
                (text, SCENE_START, false, story)
            }
            (None, None) => unreachable!("Clip::parse checked"),
        };
        let replay = Replay::parse(&text).map_err(|e| format!("{}: {e}", self.id))?;
        if replay.len() <= offset + spec.to {
            return Err(format!(
                "to {} is past the replay's end (frame {})",
                spec.to,
                replay.len().saturating_sub(1).saturating_sub(offset)
            ));
        }
        if spec.scene.is_some() {
            // The level loaded when it should have, or every frame after is somewhere else.
            let name = SCENE_LEVEL.to_uppercase();
            let loaded = format!("customLevelName={name}@{SCENE_LOADED}..{SCENE_LOADED}");
            story.facts.push(Fact::parse(&loaded)?);
        }
        Ok(Source {
            replay,
            text,
            offset,
            settings,
            story,
        })
    }
}

/// A spec's id: its path under `dir`, without `.toml`, in lower-case letters, digits and dashes.
fn clip_id(dir: &Path, path: &Path) -> Result<String, String> {
    let rel = path
        .strip_prefix(dir)
        .map_err(|_| format!("{}: a clip spec goes in {SPEC_DIR}/", path.display()))?;
    let rel = rel
        .to_str()
        .and_then(|r| r.strip_suffix(".toml"))
        .ok_or_else(|| format!("{}: not a .toml file", path.display()))?;
    let id = rel.replace(std::path::MAIN_SEPARATOR, "/");
    let ok = |part: &str| {
        !part.is_empty()
            && part
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    };
    if !id.split('/').all(ok) {
        return Err(format!(
            "{}: name it with lower-case letters, digits and dashes (wiki/clips/<entity>/<move>.toml)",
            path.display()
        ));
    }
    Ok(id)
}

/// Every spec under `repo`'s [`SPEC_DIR`], sorted by path.
pub fn all_specs(repo: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    let mut dirs = vec![repo.join(SPEC_DIR)];
    while let Some(dir) = dirs.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(format!("{}: {e}", dir.display())),
        };
        for entry in entries {
            let path = entry.map_err(|e| format!("{}: {e}", dir.display()))?.path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|e| e == "toml") {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

/// The scene's level as a `.lvl` file: the grid, padded to 40x32 with `0`, then the author, the music, the
/// player's hearts, bombs and ropes, no next level, and no exit or sign names (counts of 0, so the loader
/// reads them as such; docs/quirks.md has what an editor-saved level without them does).
fn level_file(scene: &Scene) -> Result<String, String> {
    let rows: Vec<&str> = scene.level.lines().collect();
    if rows.len() > 32 {
        return Err(format!("the level has {} rows, at most 32", rows.len()));
    }
    let mut lines = Vec::new();
    let mut entrances = 0;
    for (j, row) in rows.iter().enumerate() {
        if row.len() > 40 {
            return Err(format!(
                "level row {} is {} wide, at most 40",
                j + 1,
                row.len()
            ));
        }
        // Printable ASCII, as the editor writes: the game reads it a character at a time.
        if let Some(c) = row.chars().find(|c| !c.is_ascii_graphic()) {
            return Err(format!("level row {} has {c:?}, not a tile code", j + 1));
        }
        entrances += row.matches('@').count();
        lines.push(format!("{row:0<40}"));
    }
    if entrances != 1 {
        return Err(format!(
            "the level has {entrances} entrances (@): it needs one, where the player starts"
        ));
    }
    lines.resize(32, "0".repeat(40));
    let (lives, bombs, ropes) = (scene.lives, scene.bombs, scene.ropes);
    if !(1..=99).contains(&lives) || bombs > 99 || ropes > 99 {
        return Err("lives are 1 to 99, bombs and ropes 0 to 99 (scrLoadLevel)".into());
    }
    for line in ["WIKI", "CAVE"] {
        lines.push(line.to_owned());
    }
    for n in [lives, bombs, ropes] {
        lines.push(n.to_string());
    }
    // No next level, no exit names, no sign messages.
    lines.extend(["", "0", "0"].map(str::to_owned));
    Ok(lines.join("\r\n"))
}

/// The replay that plays a scene: boots, loads the level ([`SCENE_BOOT`]) and plays the inputs, shifted to
/// start at [`SCENE_START`], up to the clip's last frame.
fn scene_replay(id: &str, spec: &Spec, scene: &Scene) -> Result<String, String> {
    let level = serde_json::to_string(&level_file(scene)?).map_err(|e| e.to_string())?;
    let mut text = format!(
        "# The wiki clip {id}'s scene (wiki/clips/{id}.toml), made by wiki-clips: it boots, loads the level from\n\
         # the title's load-level menu (frame {SCENE_LOADED}) and plays the scene's inputs from frame {SCENE_START}, its frame 0.\n\
         seed {}\nframes {}\nfile levels/{SCENE_LEVEL}.lvl {level}\n{SCENE_BOOT}",
        scene.seed,
        SCENE_START + spec.to + 1,
    );
    for (n, line) in scene.inputs.lines().enumerate() {
        let event = line.split_once('#').map_or(line, |(e, _)| e).trim();
        if event.is_empty() {
            continue;
        }
        let (frame, rest) = event.split_once(char::is_whitespace).unwrap_or((event, ""));
        let frame: u64 = frame.parse().map_err(|_| {
            format!(
                "inputs line {}: {event:?} isn't an event (`F hold right 20`)",
                n + 1
            )
        })?;
        if frame > spec.to {
            return Err(format!(
                "inputs line {}: frame {frame} is after the clip's last, {}",
                n + 1,
                spec.to
            ));
        }
        text += &format!("{} {}\n", frame + SCENE_START, rest.trim());
    }
    // Any other mistake in an event, with the scene's line.
    Replay::parse(&text).map_err(|e| format!("inputs: {e}"))?;
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCENE: &str = r#"
title = "Snake bite"
description = "A snake bites the player."
from = 0
to = 119
poster = 60
story = "plife=3@40..100"

[scene]
level = '''
1111
1@0n
1111
'''
inputs = '''
# wait, then walk right
10 hold right 5
'''
"#;

    fn clip(text: &str) -> Result<Clip, String> {
        Clip::parse("snake/bite", Path::new("wiki/clips/snake/bite.toml"), text)
    }

    #[test]
    fn a_scene_becomes_a_replay_that_loads_its_level() {
        let c = clip(SCENE).unwrap();
        assert_eq!((c.spec.scale, c.spec.looped), (2, true));
        let s = c.source(Path::new("/nonexistent")).unwrap();
        assert_eq!(s.offset, SCENE_START);
        assert_eq!(s.replay.len(), SCENE_START + 120);
        assert_eq!(c.run_frames(&s), (SCENE_START, SCENE_START + 119));
        let level = &s.replay.header.files["levels/wiki.lvl"];
        let lines: Vec<&str> = level.split("\r\n").collect();
        assert_eq!(lines.len(), 32 + 8);
        assert_eq!(lines[1], format!("1@0n{}", "0".repeat(36)));
        assert_eq!(lines[31], "0".repeat(40));
        assert_eq!(&lines[32..], ["WIKI", "CAVE", "4", "4", "4", "", "0", "0"]);
        // The input, shifted to the level's first frame; the boot's before it.
        let events: Vec<u64> = s.replay.events().iter().map(|(f, _)| *f).collect();
        assert!(events.contains(&(SCENE_START + 10)));
        assert!(events.contains(&SCENE_LOADED));
        // The facts, shifted too, and that the level loaded.
        assert_eq!(
            s.story.facts[0].when,
            Some((SCENE_START + 40, SCENE_START + 100))
        );
        assert_eq!(s.story.facts[1].text, "customLevelName=WIKI@240..240");
    }

    #[test]
    fn facts_are_checked_on_the_clip_s_frames_only() {
        let c = clip(
            &SCENE
                .replace("plife=3@40..100", "-oSnake !plife<3@..20 +oBat@100..")
                .replace("from = 0", "from = 10")
                .replace("poster = 60", "poster = 70"),
        )
        .unwrap();
        let s = c.source(Path::new("/nonexistent")).unwrap();
        let when: Vec<_> = s.story.facts.iter().map(|f| f.when).collect();
        assert_eq!(
            when[..3],
            [
                Some((SCENE_START + 10, SCENE_START + 119)),
                Some((SCENE_START + 10, SCENE_START + 20)),
                Some((SCENE_START + 100, SCENE_START + 119)),
            ]
        );
    }

    #[test]
    fn bad_specs_say_why() {
        let bad = |from: &str, to: &str| clip(&SCENE.replace(from, to)).unwrap_err();
        assert!(bad("to = 119", "to = 30").contains("90 to 600"));
        assert!(bad("poster = 60", "poster = 200").contains("0..=119"));
        assert!(bad("plife=3@40..100", "plife=3@200").contains("outside the clip"));
        assert!(bad("story = \"plife=3@40..100\"", "story = \"!plife<1\"").contains("must hold"));
        assert!(bad("1@0n", "100n").contains("0 entrances"));
        assert!(bad("1@0n", "1@@n").contains("2 entrances"));
        assert!(bad("[scene]", "replay = \"x.txt\"\n[scene]").contains("not both"));
        assert!(bad("poster = 60", "poster = 60\nscale = 3").contains("2 or 4"));
        assert!(bad("poster = 60", "poster = 60\ncrop = 1").contains("unknown field"));
        let c = clip(&SCENE.replace("10 hold right 5", "130 hold right 5")).unwrap();
        assert!(
            c.source(Path::new("/"))
                .unwrap_err()
                .contains("after the clip's last")
        );
        let c = clip(&SCENE.replace("10 hold right 5", "10 hold nokey 5")).unwrap();
        assert!(c.source(Path::new("/")).unwrap_err().starts_with("inputs:"));
    }

    #[test]
    fn a_settings_replay_plays_the_settings_program() {
        let repo =
            std::fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
        let source = |id: &str| {
            let clip = Clip::read(&repo, &repo.join(SPEC_DIR).join(format!("{id}.toml"))).unwrap();
            clip.source(&repo).unwrap()
        };
        assert!(source("settings/window").settings);
        assert!(!source("controls/pause").settings);
        assert!(!source("snake/bite").settings);
    }

    #[test]
    fn ids_are_paths_under_the_spec_dir() {
        let dir = Path::new("/r/wiki/clips");
        assert_eq!(
            clip_id(dir, &dir.join("snake").join("bite.toml")).unwrap(),
            "snake/bite"
        );
        assert!(clip_id(dir, &dir.join("Snake").join("bite.toml")).is_err());
        assert!(clip_id(dir, Path::new("/elsewhere/bite.toml")).is_err());
    }
}
