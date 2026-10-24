//! Replay fingerprints (docs/formats.md, "Fingerprints"): a compact record of what the oracle did with a
//! committed replay, so `fidelity-check` can check the port against it without Docker.
//!
//! A fingerprint hashes the trace frame by frame, with checkpoints (the running hash) every
//! [`CHECKPOINT_EVERY`] frames and at each room change, so a mismatch says roughly where the runs part; then
//! the host files and highscore table. Both sides hash the same canonical encoding of a parsed
//! [`trace::Frame`], never the JSON bytes: the oracle's trace is read back into frames, the port's frames
//! come straight from the world, and the encoding makes equal what `trace-diff` counts as equal (a missing
//! alarm is -1, a scalar is element 0 of an array, every NaN is the same NaN), so the two writers' formatting
//! never matters. The hash is a fixed 64-bit one defined here, not the platform's `Hasher`, so fingerprints
//! stay the same across Rust releases and targets.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use trace::trace::ALARMS;
use trace::{Frame, Instance, Real, Replay, Scalar, Value};

use crate::host::Entry;

/// A checkpoint every this many frames (after frames 299, 599...), besides those at room changes.
pub const CHECKPOINT_EVERY: u64 = 300;

/// What a replay did: its trace and host files, hashed. Hashes are 16 hex digits.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fingerprint {
    /// The replay it was recorded for, in [`Replay::to_text`]'s canonical form (so comments don't count).
    pub replay: String,
    /// How many frames the trace holds.
    pub frames: u64,
    /// The whole trace: the last checkpoint's hash.
    pub trace: String,
    /// The files and highscore table the run left behind ([`crate::host::HostContents`]).
    pub host: String,
    /// The running hash after the trace's first frame, every [`CHECKPOINT_EVERY`] frames, at the first
    /// frame in each new room, and after the last frame.
    pub checkpoints: Vec<Checkpoint>,
}

/// The running trace hash after a frame: `[frame, room name, hash]`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoint(pub u64, pub String, pub String);

impl Fingerprint {
    /// The file's text: JSON with one checkpoint per line, so diffs stay readable.
    pub fn to_text(&self) -> String {
        let mut s = format!(
            "{{\n  \"replay\": {},\n  \"frames\": {},\n  \"trace\": {},\n  \"host\": {},\n  \"checkpoints\": [",
            json(&self.replay),
            self.frames,
            json(&self.trace),
            json(&self.host),
        );
        for (i, c) in self.checkpoints.iter().enumerate() {
            let sep = if i == 0 { "" } else { "," };
            let _ = write!(s, "{sep}\n    {}", json(c));
        }
        s.push_str("\n  ]\n}\n");
        s
    }

    pub fn parse(text: &str) -> Result<Fingerprint, String> {
        serde_json::from_str(text).map_err(|e| e.to_string())
    }

    pub fn read(path: &Path) -> Result<Fingerprint, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Fingerprint::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn write(&self, path: &Path) -> Result<(), String> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::write(path, self.to_text()).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// How `actual` (the port's) differs from `self` (the oracle's), or `None` if it doesn't.
    pub fn mismatch(&self, actual: &Fingerprint) -> Option<String> {
        if self.replay != actual.replay {
            return Some(
                "the replay changed since its fingerprint was recorded (record it again)".into(),
            );
        }
        let (a, b) = (&self.checkpoints, &actual.checkpoints);
        if let Some(i) = (0..a.len().max(b.len())).find(|&i| a.get(i) != b.get(i)) {
            let after = match i.checked_sub(1).map(|i| &a[i]) {
                Some(Checkpoint(frame, room, _)) => {
                    format!("matches up to frame {frame} (in {room}), then ")
                }
                None => String::new(),
            };
            let at = |c: Option<&Checkpoint>| match c {
                Some(Checkpoint(frame, room, _)) => format!("frame {frame} in {room}"),
                None => "none".into(),
            };
            return Some(format!(
                "the trace {after}differs by the next checkpoint (oracle: {}, port: {}; {} frames, port {})",
                at(a.get(i)),
                at(b.get(i)),
                self.frames,
                actual.frames
            ));
        }
        if self.frames != actual.frames || self.trace != actual.trace {
            return Some(format!(
                "the trace differs ({} frames, port {})",
                self.frames, actual.frames
            ));
        }
        (self.host != actual.host)
            .then(|| "the trace matches, but the host files or highscore table differ".into())
    }
}

/// A string or checkpoint as JSON, which can't fail.
fn json<T: Serialize + ?Sized>(v: &T) -> String {
    serde_json::to_string(v).expect("strings and checkpoints serialize")
}

/// Where a suite's replay's fingerprint is kept: `fidelity/fingerprints/<suite dir>/<stem>.json`.
pub fn fingerprint_path(repo: &Path, suite: &Suite, replay: &Path) -> PathBuf {
    let stem = replay.file_stem().unwrap_or_default();
    let mut name = stem.to_os_string();
    name.push(".json");
    repo.join("fidelity/fingerprints")
        .join(suite.name)
        .join(name)
}

/// A directory of committed replays, all of one program.
pub struct Suite {
    /// The directory under `fidelity/`, and under `fidelity/fingerprints/`.
    pub name: &'static str,
    /// The settings program's replays (`--settings`), rather than the game's.
    pub settings: bool,
}

/// Every committed replay suite.
pub const SUITES: [Suite; 2] = [
    Suite {
        name: "replays",
        settings: false,
    },
    Suite {
        name: "settings",
        settings: true,
    },
];

impl Suite {
    pub fn dir(&self, repo: &Path) -> PathBuf {
        repo.join("fidelity").join(self.name)
    }

    /// The suite's replays, `*.txt`, sorted.
    pub fn replays(&self, repo: &Path) -> Result<Vec<PathBuf>, String> {
        sorted_with_extension(&self.dir(repo), "txt")
    }

    /// The suite a replay belongs to, by its directory.
    pub fn of(repo: &Path, replay: &Path) -> Result<&'static Suite, String> {
        let canon =
            |p: &Path| std::fs::canonicalize(p).map_err(|e| format!("{}: {e}", p.display()));
        // A bare file name's parent is "".
        let parent = replay.parent().filter(|p| !p.as_os_str().is_empty());
        let dir = canon(parent.unwrap_or(Path::new(".")))?;
        for s in &SUITES {
            if canon(&s.dir(repo)).is_ok_and(|d| d == dir) {
                return Ok(s);
            }
        }
        Err(format!(
            "{}: only the committed replays (fidelity/replays/, fidelity/settings/) have fingerprints",
            replay.display()
        ))
    }
}

/// The files in `dir` with this extension, sorted; none if `dir` doesn't exist.
pub fn sorted_with_extension(dir: &Path, ext: &str) -> Result<Vec<PathBuf>, String> {
    let entries = match std::fs::read_dir(dir) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        r => r.map_err(|e| format!("{}: {e}", dir.display()))?,
    };
    let mut out: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == ext))
        .collect();
    out.sort();
    Ok(out)
}

/// Builds a [`Fingerprint`] from a trace's frames, in order.
#[derive(Default)]
pub struct Fingerprinter {
    /// The frame being encoded, reused.
    buf: Vec<u8>,
    running: u64,
    frames: u64,
    /// The last frame's number and room, and whether it got a checkpoint.
    last: Option<(u64, Option<i32>, bool)>,
    last_room_name: String,
    checkpoints: Vec<Checkpoint>,
}

impl Fingerprinter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn frame(&mut self, f: &Frame) {
        self.buf.clear();
        encode_frame(&mut Encoder(&mut self.buf), f);
        let h = hash(&self.buf);
        let mut pair = [0; 16];
        pair[..8].copy_from_slice(&self.running.to_le_bytes());
        pair[8..].copy_from_slice(&h.to_le_bytes());
        self.running = hash(&pair);
        self.frames += 1;
        let new_room = self.last.is_none_or(|(_, room, _)| room != f.room);
        let checkpoint = new_room || f.frame % CHECKPOINT_EVERY == CHECKPOINT_EVERY - 1;
        self.last_room_name = f.room_name.clone().unwrap_or_default();
        if checkpoint {
            self.checkpoint(f.frame);
        }
        self.last = Some((f.frame, f.room, checkpoint));
    }

    fn checkpoint(&mut self, frame: u64) {
        self.checkpoints.push(Checkpoint(
            frame,
            self.last_room_name.clone(),
            hex(self.running),
        ));
    }

    /// The fingerprint of the frames seen, with `host` (none for a run that failed, which leaves none).
    pub fn finish(
        mut self,
        replay: &Replay,
        host: Option<&crate::host::HostContents>,
    ) -> Fingerprint {
        if let Some((frame, _, false)) = self.last {
            self.checkpoint(frame);
        }
        Fingerprint {
            replay: hex(hash(replay.to_text().as_bytes())),
            frames: self.frames,
            trace: hex(self.running),
            host: host.map_or_else(|| "none".into(), |h| hex(hash_host(h))),
            checkpoints: self.checkpoints,
        }
    }
}

/// The port's side: plays `replay` headless as `fidelity --fields` does (OracleCompat audio, `settings` for
/// the settings program) and fingerprints the run.
pub fn run_fingerprint(
    replay: &Replay,
    assets: Arc<gm::Assets>,
    settings: bool,
) -> Result<(Fingerprint, crate::RunSummary), String> {
    let mut summary = crate::run_replay(
        replay,
        assets,
        &crate::RunOptions {
            settings,
            fingerprint: true,
            ..crate::RunOptions::default()
        },
    )?;
    let fp = summary.fingerprint.take().expect("asked for");
    Ok((fp, summary))
}

/// How the port's run (`port`, with its `summary`) differs from the oracle's fingerprint, or `None`.
pub fn port_mismatch(
    oracle: &Fingerprint,
    port: &Fingerprint,
    summary: &crate::RunSummary,
) -> Option<String> {
    let m = oracle.mismatch(port)?;
    Some(match &summary.error {
        Some(e) => format!("{m}; the port stopped: {}", e.message),
        None => m,
    })
}

/// The oracle's side: the fingerprint of a trace file and the host files next to it
/// ([`crate::host::host_dir`]).
pub fn fingerprint_trace(replay: &Replay, trace_path: &Path) -> Result<Fingerprint, String> {
    let path = trace_path.to_string_lossy();
    let mut fp = Fingerprinter::new();
    for frame in tools::open_trace(&path)? {
        fp.frame(&frame.map_err(|e| format!("{path}: {e}"))?);
    }
    // A run that failed leaves none, as on the port.
    let dir = crate::host::host_dir(trace_path);
    let host = if dir.exists() {
        Some(crate::host::read_host(&dir)?)
    } else {
        None
    };
    Ok(fp.finish(replay, host.as_ref()))
}

fn hex(h: u64) -> String {
    format!("{h:016x}")
}

/// A fixed 64-bit hash. Four lanes, each starting from its own seed and the length, take turns at the
/// 8-byte little-endian words (in 32-byte blocks, the last zero-padded), mixing each in by xor, multiply and
/// rotate; then the lanes are folded together and MurmurHash3's finalizer applied. Each step is a bijection
/// of a lane, so two inputs of the same length that differ in one word always differ after it. Four lanes
/// run four times as fast as one, which matters at hundreds of KB a frame. Not cryptographic: it only has
/// to catch accidental differences.
fn hash(bytes: &[u8]) -> u64 {
    const K: u64 = 0x9e37_79b9_7f4a_7c15;
    let mix = |h: u64, w: u64| (h ^ w).wrapping_mul(K).rotate_left(31);
    let fmix = |mut h: u64| {
        h ^= h >> 33;
        h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
        h ^= h >> 33;
        h = h.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
        h ^ (h >> 33)
    };
    let len = bytes.len() as u64;
    let mut lanes = [
        0x243f_6a88_85a3_08d3,
        0x1319_8a2e_0370_7344,
        0xa409_3822_299f_31d0,
        0x082e_fa98_ec4e_6c89,
    ]
    .map(|seed| mix(seed, len));
    let mut block = |b: &[u8; 32]| {
        let (words, _) = b.as_chunks::<8>();
        for (lane, w) in lanes.iter_mut().zip(words) {
            *lane = mix(*lane, u64::from_le_bytes(*w));
        }
    };
    let (blocks, rest) = bytes.as_chunks::<32>();
    for b in blocks {
        block(b);
    }
    if !rest.is_empty() {
        let mut b = [0; 32];
        b[..rest.len()].copy_from_slice(rest);
        block(&b);
    }
    fmix(lanes.iter().fold(len, |h, &lane| mix(h, fmix(lane))))
}

fn hash_host(h: &crate::host::HostContents) -> u64 {
    let mut buf = Vec::new();
    let mut e = Encoder(&mut buf);
    e.len(h.files.len());
    for (path, bytes) in &h.files {
        e.str(path);
        e.bytes(bytes);
    }
    e.len(h.highscores.len());
    for Entry { name, value } in &h.highscores {
        e.str(name);
        e.real(*value);
    }
    hash(&buf)
}

/// The canonical encoding: fixed-width little-endian numbers, length-prefixed strings and lists, a tag
/// before each optional value.
struct Encoder<'a>(&'a mut Vec<u8>);

impl Encoder<'_> {
    fn u64(&mut self, v: u64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }

    fn i32(&mut self, v: i32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }

    fn len(&mut self, n: usize) {
        self.u64(n as u64);
    }

    fn bytes(&mut self, b: &[u8]) {
        self.len(b.len());
        self.0.extend_from_slice(b);
    }

    fn str(&mut self, s: &str) {
        self.bytes(s.as_bytes());
    }

    /// Bit for bit, as `Real`'s equality, except that every NaN is the same.
    fn real(&mut self, r: Real) {
        let r = if r.0.is_nan() { f64::NAN } else { r.0 };
        self.u64(r.to_bits());
    }

    fn bool(&mut self, b: bool) {
        self.0.push(u8::from(b));
    }

    fn opt<T: Copy>(&mut self, v: Option<T>, f: impl FnOnce(&mut Self, T)) {
        match v {
            None => self.0.push(0),
            Some(v) => {
                self.0.push(1);
                f(self, v);
            }
        }
    }

    fn scalar(&mut self, s: &Scalar) {
        match s {
            Scalar::Real(r) => {
                self.0.push(0);
                self.real(*r);
            }
            Scalar::Str(s) => {
                self.0.push(1);
                self.str(s);
            }
        }
    }

    /// Variables as `trace-diff` flattens them: each one's elements, a scalar being element 0, so an
    /// array of just element 0 is the scalar, and an empty array the same as no variable.
    fn vars(&mut self, vars: Option<&BTreeMap<String, Value>>) {
        self.opt(vars, |e, vars| {
            let count = |v: &Value| match v {
                Value::Array(a) => a.len(),
                _ => 1,
            };
            e.len(vars.values().filter(|v| count(v) > 0).count());
            for (name, value) in vars {
                if count(value) == 0 {
                    continue;
                }
                e.str(name);
                e.len(count(value));
                match value {
                    Value::Real(r) => {
                        e.u64(0);
                        e.scalar(&Scalar::Real(*r));
                    }
                    Value::Str(s) => {
                        e.u64(0);
                        e.0.push(1);
                        e.str(s);
                    }
                    Value::Array(a) => {
                        for (i, s) in a {
                            e.u64(u64::from(*i));
                            e.scalar(s);
                        }
                    }
                }
            }
        });
    }

    fn instances(&mut self, list: &[Instance]) {
        // Writers sort by id; a trace that didn't is still the same set.
        let mut sorted: Vec<&Instance> = list.iter().collect();
        sorted.sort_by_key(|i| i.id);
        self.len(sorted.len());
        for i in sorted {
            self.instance(i);
        }
    }

    fn instance(&mut self, i: &Instance) {
        // Destructured so that a new field can't be left out of the fingerprint.
        let Instance {
            id,
            object_index,
            object,
            x,
            y,
            xprevious,
            yprevious,
            hspeed,
            vspeed,
            speed,
            direction,
            gravity,
            friction,
            image_index,
            image_speed,
            image_xscale,
            image_yscale,
            depth,
            sprite_index,
            sprite,
            visible,
            solid,
            alarms,
            vars,
        } = i;
        self.i32(*id);
        self.opt(*object_index, Self::i32);
        self.str(object);
        for r in [
            x,
            y,
            xprevious,
            yprevious,
            hspeed,
            vspeed,
            speed,
            direction,
            gravity,
            friction,
            image_index,
            image_speed,
            image_xscale,
            image_yscale,
            depth,
        ] {
            self.opt(*r, Self::real);
        }
        self.opt(*sprite_index, Self::i32);
        self.opt(sprite.as_deref(), Self::str);
        self.opt(*visible, Self::bool);
        self.opt(*solid, Self::bool);
        // A missing alarm is -1, so only the others count; `trace-diff` compares alarms 0 to 11.
        self.opt(alarms.as_ref(), |e, alarms| {
            let set = || alarms.iter().filter(|&(&k, &v)| k < ALARMS && v != -1);
            e.len(set().count());
            for (&k, &v) in set() {
                e.u64(u64::from(k));
                e.i32(v);
            }
        });
        self.vars(vars.as_ref());
    }
}

fn encode_frame(e: &mut Encoder, f: &Frame) {
    let Frame {
        frame,
        room,
        room_name,
        seed,
        instance_count,
        instances,
        deactivated,
        globals,
        builtins,
    } = f;
    e.u64(*frame);
    e.opt(*room, Encoder::i32);
    e.opt(room_name.as_deref(), Encoder::str);
    e.opt(*seed, Encoder::i32);
    e.opt(*instance_count, Encoder::u64);
    e.instances(instances);
    e.opt(deactivated.as_deref(), Encoder::instances);
    e.vars(globals.as_ref());
    e.vars(builtins.as_ref());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instance(id: i32, object: &str, x: f64) -> Instance {
        let mut i = Instance::new(id, object);
        i.x = Some(Real(x));
        i.y = Some(Real(1.0));
        i
    }

    fn frame(n: u64, instances: Vec<Instance>) -> Frame {
        Frame {
            frame: n,
            room: Some(3),
            room_name: Some("rLevel".into()),
            instances,
            ..Frame::default()
        }
    }

    fn print(frames: &[Frame]) -> Fingerprint {
        let mut fp = Fingerprinter::new();
        for f in frames {
            fp.frame(f);
        }
        fp.finish(&Replay::parse("frames 3\n").unwrap(), None)
    }

    #[test]
    fn what_trace_diff_counts_as_equal_hashes_the_same() {
        let mut a = frame(0, vec![instance(5, "oA", 1.0), instance(3, "oB", f64::NAN)]);
        let mut b = a.clone();
        a.instances[0].alarms = Some(BTreeMap::from([(0, 17), (8, -1)]));
        b.instances[0].alarms = Some(BTreeMap::from([(0, 17)]));
        a.instances[0].vars = Some(BTreeMap::from([
            ("s".into(), Value::from(2.0)),
            ("e".into(), Value::Array(BTreeMap::new())),
        ]));
        b.instances[0].vars = Some(BTreeMap::from([(
            "s".into(),
            Value::Array(BTreeMap::from([(0, Scalar::Real(Real(2.0)))])),
        )]));
        // Another NaN, and the instances out of order.
        b.instances[1].x = Some(Real(-f64::NAN));
        b.instances.reverse();
        assert_eq!(print(&[a.clone()]), print(&[b.clone()]));
        // But -0 isn't 0, and order of frames matters.
        b.instances[0].y = Some(Real(-0.0));
        a.instances[1].y = Some(Real(0.0));
        assert_ne!(print(&[a.clone()]).trace, print(&[b]).trace);
        let c = frame(1, vec![]);
        assert_ne!(print(&[a.clone(), c.clone()]).trace, print(&[c, a]).trace);
    }

    #[test]
    fn checkpoints_at_room_changes_every_300_frames_and_the_end() {
        let frames: Vec<Frame> = (0..700)
            .map(|n| {
                let mut f = frame(n, vec![]);
                f.room = Some(if n < 10 { 1 } else { 2 });
                f.room_name = Some(if n < 10 { "rA" } else { "rB" }.into());
                f
            })
            .collect();
        let fp = print(&frames);
        let at: Vec<(u64, &str)> = fp.checkpoints.iter().map(|c| (c.0, c.1.as_str())).collect();
        assert_eq!(
            at,
            [(0, "rA"), (10, "rB"), (299, "rB"), (599, "rB"), (699, "rB")]
        );
        assert_eq!(fp.trace, fp.checkpoints[4].2);
        assert_eq!(fp.frames, 700);
        assert_eq!(Fingerprint::parse(&fp.to_text()).unwrap(), fp);

        // A change on frame 400 shows up at the checkpoint after it, and not before.
        let mut changed = frames.clone();
        changed[400].seed = Some(1);
        let msg = fp.mismatch(&print(&changed)).unwrap();
        assert!(
            msg.contains("matches up to frame 299 (in rB), then differs by the next checkpoint (oracle: frame 599"),
            "{msg}"
        );
        assert_eq!(fp.mismatch(&fp.clone()), None);
        let short = print(&frames[..650]);
        assert!(fp.mismatch(&short).unwrap().contains("port: frame 649"));
    }

    #[test]
    fn the_hash_is_fixed() {
        // Fingerprints are committed: changing the hash means recording them all again.
        assert_eq!(hex(hash(b"")), "f4a9e55670e191bb");
        assert_eq!(hex(hash(b"spelunky")), "b42d3d893cbed43f");
    }
}
