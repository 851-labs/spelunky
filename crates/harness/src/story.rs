//! Replay stories (docs/formats.md, "Stories"): a few facts a committed replay's header declares about what
//! happens in it (`# story:` lines), checked on the port's run by `fidelity-check`. A fingerprint only says
//! that the port and the oracle agree. A fidelity fix that changes play in both can take a replay somewhere
//! its header no longer describes, a shop gone or a death come early, and the fingerprints are simply
//! recorded again. The story facts catch that.
//!
//! [`Timeline`] is the other half: what happens in a run (rooms, the globals that rarely change, the player,
//! objects created and destroyed), for writing the facts and for checking a header by eye (`replay-story`).

use std::collections::BTreeMap;
use std::fmt::Write as _;

use gm::{GVar, Inst, ObjectId, State, Value, Var, World};

/// How far either way of `@F` a fact may happen: a second.
pub const SLACK: u64 = 30;

/// A replay's story facts, from its `# story:` lines.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Story {
    pub facts: Vec<Fact>,
}

/// One fact: `[!]WHAT[@WHEN]`.
#[derive(Clone, Debug, PartialEq)]
pub struct Fact {
    /// As written.
    pub text: String,
    /// `!`: the fact must not hold.
    pub negated: bool,
    pub what: What,
    /// The frames it must happen in, inclusive; any frame if `None`.
    pub when: Option<(u64, u64)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum What {
    /// `+oX` (`+oX*N`): an instance of the object `oX` (N of them) appears: created, or there when a room
    /// starts. Children don't count; deactivated instances do.
    Created { object: String, count: u32 },
    /// `-oX` (`-oX*N`): an instance of `oX` (N of them) goes away without the room changing: destroyed, or
    /// changed into another object.
    Destroyed { object: String, count: u32 },
    /// `NAME OP VALUE`: a condition. With `@`, it must become true in the window (it is true on a frame
    /// there and wasn't on the frame before, or that is the run's first); without, hold on some frame.
    /// Negated, it must be false on every frame of the window (or the run).
    Holds {
        subject: Subject,
        op: Cmp,
        value: Lit,
    },
}

/// What a condition looks at.
#[derive(Clone, Debug, PartialEq)]
pub enum Subject {
    /// `room`: the room's name.
    Room,
    /// `held`: the object the player holds (`oPlayer1.holdItem`); none is no object.
    Held,
    /// `NAME`: `global.NAME`.
    Global(String),
    /// `oX.NAME`: the variable of some instance of `oX`, active or deactivated.
    Var { object: String, var: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cmp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// A condition's value: a number if it parses as one, else a string (compared with `=` and `!=` only).
#[derive(Clone, Debug, PartialEq)]
pub enum Lit {
    Real(f64),
    Str(String),
}

impl Story {
    /// The facts on a replay's `# story:` lines, separated by spaces.
    pub fn parse(replay_text: &str) -> Result<Story, String> {
        let mut facts = Vec::new();
        for (n, line) in replay_text.lines().enumerate() {
            let Some(comment) = line.trim_start().strip_prefix('#') else {
                continue;
            };
            let Some(rest) = comment.trim_start().strip_prefix("story:") else {
                continue;
            };
            for token in tokens(rest).map_err(|e| format!("line {}: {e}", n + 1))? {
                facts
                    .push(Fact::parse(token).map_err(|e| format!("line {}: {token}: {e}", n + 1))?);
            }
        }
        Ok(Story { facts })
    }

    pub fn is_empty(&self) -> bool {
        self.facts.is_empty()
    }

    /// Whether it says something happens: a fact that must hold, which every committed replay needs
    /// (`fidelity-check`). Facts that mustn't hold alone would go on holding however far the replay drifted.
    pub fn tells(&self) -> bool {
        self.facts.iter().any(|f| !f.negated)
    }
}

impl Fact {
    pub fn parse(token: &str) -> Result<Fact, String> {
        let (negated, body) = match token.strip_prefix('!') {
            Some(b) => (true, b),
            None => (false, token),
        };
        // The window's `@` comes after a quoted value, which may hold one of its own.
        let after_quote = body.rfind('"').map_or(0, |i| i + 1);
        let (body, when) = match body[after_quote..].find('@') {
            Some(i) => (
                &body[..after_quote + i],
                Some(parse_when(&body[after_quote + i + 1..])?),
            ),
            None => (body, None),
        };
        let what = if let Some(o) = body.strip_prefix('+') {
            let (object, count) = parse_count(o)?;
            What::Created { object, count }
        } else if let Some(o) = body.strip_prefix('-') {
            let (object, count) = parse_count(o)?;
            What::Destroyed { object, count }
        } else {
            parse_condition(body)?
        };
        Ok(Fact {
            text: token.to_string(),
            negated,
            what,
            when,
        })
    }
}

/// A story line's facts: separated by spaces, but for those inside a quoted value (`message="YOU GOT A
/// CAPE!"`).
fn tokens(line: &str) -> Result<Vec<&str>, String> {
    let mut tokens = Vec::new();
    let mut start = None;
    let mut quoted = false;
    for (i, c) in line.char_indices() {
        if c == '"' {
            quoted = !quoted;
        }
        if c.is_whitespace() && !quoted {
            if let Some(s) = start.take() {
                tokens.push(&line[s..i]);
            }
        } else if start.is_none() {
            start = Some(i);
        }
    }
    if quoted {
        return Err("a quote isn't closed".into());
    }
    tokens.extend(start.map(|s| &line[s..]));
    Ok(tokens)
}

/// `F` (within [`SLACK`] of it), `A..B`, `A..` or `..B`.
fn parse_when(w: &str) -> Result<(u64, u64), String> {
    let num = |s: &str| {
        s.parse::<u64>()
            .map_err(|_| format!("bad frame {s:?} after @"))
    };
    match w.split_once("..") {
        Some((a, b)) => {
            let a = if a.is_empty() { 0 } else { num(a)? };
            let b = if b.is_empty() { u64::MAX } else { num(b)? };
            if a > b {
                return Err(format!("empty window {a}..{b}"));
            }
            Ok((a, b))
        }
        None => {
            let f = num(w)?;
            Ok((f.saturating_sub(SLACK), f.saturating_add(SLACK)))
        }
    }
}

/// `oX` or `oX*N`.
fn parse_count(s: &str) -> Result<(String, u32), String> {
    let (object, count) = match s.split_once('*') {
        Some((o, n)) => (
            o,
            n.parse::<u32>()
                .ok()
                .filter(|&n| n > 0)
                .ok_or(format!("bad count {n:?} after *"))?,
        ),
        None => (s, 1),
    };
    if !is_name(object) {
        return Err(format!("bad object name {object:?}"));
    }
    Ok((object.to_string(), count))
}

fn is_name(s: &str) -> bool {
    s.chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn parse_condition(body: &str) -> Result<What, String> {
    let at = body
        .find(['<', '>', '=', '!'])
        .ok_or("expected +oObject, -oObject or NAME=VALUE (or !=, <, <=, >, >=)")?;
    let (lhs, rest) = body.split_at(at);
    let (op, value) = match rest.as_bytes() {
        [b'<', b'=', ..] => (Cmp::Le, &rest[2..]),
        [b'>', b'=', ..] => (Cmp::Ge, &rest[2..]),
        [b'!', b'=', ..] => (Cmp::Ne, &rest[2..]),
        [b'<', ..] => (Cmp::Lt, &rest[1..]),
        [b'>', ..] => (Cmp::Gt, &rest[1..]),
        [b'=', ..] => (Cmp::Eq, &rest[1..]),
        _ => return Err("expected an operator: =, !=, <, <=, >, >=".into()),
    };
    let subject = match lhs.split_once('.') {
        _ if lhs == "room" => Subject::Room,
        _ if lhs == "held" => Subject::Held,
        Some((object, var)) if is_name(object) && is_name(var) => Subject::Var {
            object: object.into(),
            var: var.into(),
        },
        None if is_name(lhs) => Subject::Global(lhs.into()),
        _ => return Err(format!("bad name {lhs:?}")),
    };
    let value = if let Some(q) = value.strip_prefix('"') {
        match q.strip_suffix('"') {
            Some(s) if !s.contains('"') => Lit::Str(s.to_string()),
            _ => return Err(format!("bad quoted value {value}")),
        }
    } else {
        match value.parse::<f64>() {
            Ok(x) if x.is_finite() => Lit::Real(x),
            _ => Lit::Str(value.to_string()),
        }
    };
    if matches!(value, Lit::Str(_)) && !matches!(op, Cmp::Eq | Cmp::Ne) {
        return Err("a string compares with = or != only".into());
    }
    if matches!(subject, Subject::Room | Subject::Held) && matches!(value, Lit::Real(_)) {
        return Err(format!("{lhs} compares with a name"));
    }
    Ok(What::Holds { subject, op, value })
}

/// What a fact reads on each frame, resolved against the program.
enum Probe {
    Room(i32),
    /// The player's slot, `holdItem`, and the object.
    Held(usize, Var, ObjectId),
    Global(GVar),
    /// A tracked object's slot, and its variable.
    Var(usize, Var),
    Created(usize),
    Destroyed(usize),
}

/// An object whose instances are followed.
struct Tracked {
    /// Its instances' ids (sorted) on the frame before, and on this one.
    before: Vec<i32>,
    now: Vec<i32>,
    /// This frame's instances.
    handles: Vec<Inst>,
    created: u32,
    destroyed: u32,
}

#[derive(Clone, Copy, Default)]
struct FactState {
    /// Whether the condition held on the frame before.
    held: bool,
    /// Hits: frames it held (without a window), rises into it (with one), or instances created or destroyed.
    hits: u32,
    /// The first hit anywhere, and in the window.
    first: Option<u64>,
    first_in: Option<u64>,
}

/// Checks a [`Story`] on a run, frame by frame ([`frame`](Self::frame)), then says which facts failed
/// ([`finish`](Self::finish)).
pub struct Watcher<'s> {
    story: &'s Story,
    /// Resolved on the first frame; an unknown name is an error.
    probes: Option<Result<Vec<Probe>, String>>,
    objects: Vec<Tracked>,
    /// Object index -> slot in `objects`.
    slot_of: Vec<Option<usize>>,
    state: Vec<FactState>,
    starts: RoomStarts,
}

impl<'s> Watcher<'s> {
    pub fn new(story: &'s Story) -> Self {
        Watcher {
            story,
            probes: None,
            objects: Vec::new(),
            slot_of: Vec::new(),
            state: vec![FactState::default(); story.facts.len()],
            starts: RoomStarts::default(),
        }
    }

    fn bind(&mut self, w: &World) -> Result<Vec<Probe>, String> {
        let p = w.program();
        self.slot_of = vec![None; p.object_slots()];
        let mut track = |name: &str| -> Result<(usize, ObjectId), String> {
            let o = object_named(p, name).ok_or(format!("no object {name}"))?;
            let slot = *self.slot_of[o.0 as usize].get_or_insert_with(|| {
                self.objects.push(Tracked {
                    before: Vec::new(),
                    now: Vec::new(),
                    handles: Vec::new(),
                    created: 0,
                    destroyed: 0,
                });
                self.objects.len() - 1
            });
            Ok((slot, o))
        };
        let mut probes = Vec::new();
        for f in &self.story.facts {
            let e = |e: String| format!("{}: {e}", f.text);
            probes.push(match &f.what {
                What::Created { object, .. } => Probe::Created(track(object).map_err(e)?.0),
                What::Destroyed { object, .. } => Probe::Destroyed(track(object).map_err(e)?.0),
                What::Holds { subject, value, .. } => match subject {
                    Subject::Room => {
                        let Lit::Str(name) = value else {
                            unreachable!("parse_condition")
                        };
                        Probe::Room(
                            w.assets()
                                .room_id(name)
                                .ok_or_else(|| e(format!("no room {name}")))?,
                        )
                    }
                    Subject::Held => {
                        let Lit::Str(name) = value else {
                            unreachable!("parse_condition")
                        };
                        let (slot, player) = track("oPlayer1").map_err(e)?;
                        let (v, _) = (p.layout(player).vars())
                            .find(|&(v, _)| p.var_name(v) == "holdItem")
                            .ok_or_else(|| e("oPlayer1 has no holdItem".into()))?;
                        let o =
                            object_named(p, name).ok_or_else(|| e(format!("no object {name}")))?;
                        Probe::Held(slot, v, o)
                    }
                    Subject::Global(name) => Probe::Global(
                        game::global_named(p, name)
                            .ok_or_else(|| e(format!("no global {name}")))?,
                    ),
                    Subject::Var { object, var } => {
                        let (slot, o) = track(object).map_err(e)?;
                        let (v, _) = (p.layout(o).vars())
                            .find(|&(v, _)| p.var_name(v) == var)
                            .ok_or_else(|| e(format!("{object} has no variable {var}")))?;
                        Probe::Var(slot, v)
                    }
                },
            });
        }
        Ok(probes)
    }

    /// The state after frame `frame`.
    pub fn frame(&mut self, frame: u64, w: &World) {
        if self.probes.is_none() {
            self.probes = Some(self.bind(w));
        }
        let Some(Ok(probes)) = &self.probes else {
            return;
        };
        let room = w.room().id;
        if !self.objects.is_empty() {
            for t in &mut self.objects {
                std::mem::swap(&mut t.before, &mut t.now);
                t.now.clear();
                t.handles.clear();
            }
            self.starts.begin();
            for h in w.live() {
                let i = w.instance(h);
                self.starts.see(i);
                if let Some(Some(s)) = self.slot_of.get(i.object.0 as usize) {
                    let t = &mut self.objects[*s];
                    t.now.push(i.id);
                    t.handles.push(h);
                }
            }
            // A room's end takes its instances with it; that isn't a destruction.
            let started = self.starts.started(room);
            for t in &mut self.objects {
                t.now.sort_unstable();
                t.created = missing_from(&t.now, &t.before);
                t.destroyed = if started {
                    0
                } else {
                    missing_from(&t.before, &t.now)
                };
            }
        }
        for ((f, probe), s) in self.story.facts.iter().zip(probes).zip(&mut self.state) {
            let n = match probe {
                Probe::Created(t) => self.objects[*t].created,
                Probe::Destroyed(t) => self.objects[*t].destroyed,
                _ => {
                    let What::Holds { op, value, .. } = &f.what else {
                        unreachable!("bind")
                    };
                    let holds = match probe {
                        Probe::Room(r) => (*r == room) == (*op == Cmp::Eq),
                        // Without a player (a transition room, say) he neither holds it nor doesn't.
                        Probe::Held(t, v, o) => self.objects[*t].handles.iter().any(|&h| {
                            let item = match w.get(h, *v) {
                                Ok(Value::Real(id)) if id > 0.0 => held_object(w, id as i32),
                                _ => None,
                            };
                            (item == Some(*o)) == (*op == Cmp::Eq)
                        }),
                        Probe::Global(g) => w.gget(*g).is_ok_and(|x| compare(&x, *op, value)),
                        Probe::Var(t, v) => (self.objects[*t].handles.iter())
                            .any(|&h| w.get(h, *v).is_ok_and(|x| compare(&x, *op, value))),
                        _ => unreachable!(),
                    };
                    let rose = holds && !s.held;
                    s.held = holds;
                    // A fact with a window must become true in it; one that mustn't hold there mustn't be
                    // true on any of its frames, even if it became true before.
                    u32::from(if f.when.is_some() && !f.negated {
                        rose
                    } else {
                        holds
                    })
                }
            };
            if n > 0 {
                s.first.get_or_insert(frame);
                if f.when.is_none_or(|(a, b)| (a..=b).contains(&frame)) {
                    s.hits = s.hits.saturating_add(n);
                    s.first_in.get_or_insert(frame);
                }
            }
        }
    }

    /// Each fact that failed, with why; `Err` if the story names something the program doesn't have.
    pub fn finish(self) -> Result<Vec<String>, String> {
        match self.probes {
            Some(Err(e)) => return Err(e),
            None if !self.story.is_empty() => return Err("the run had no frames".into()),
            _ => {}
        }
        let mut failed = Vec::new();
        for (f, s) in self.story.facts.iter().zip(&self.state) {
            let need = match f.what {
                What::Created { count, .. } | What::Destroyed { count, .. } => count,
                What::Holds { .. } => 1,
            };
            let window = match f.when {
                Some((a, u64::MAX)) => format!(" in {a}.."),
                Some((a, b)) => format!(" in {a}..={b}"),
                None => String::new(),
            };
            let met = s.hits >= need;
            let why = match (f.negated, met) {
                (false, true) | (true, false) => continue,
                (true, true) => format!("happened{window} (at {})", s.first_in.unwrap_or(0)),
                (false, false) => match (s.hits, s.first) {
                    (0, None) => "never happened".into(),
                    (0, Some(first)) => format!("not{window} (first at {first})"),
                    (n, _) => format!("only {n} of {need}{window}"),
                },
            };
            failed.push(format!("{}: {why}", f.text));
        }
        Ok(failed)
    }
}

/// The object of the instance with this id, active or deactivated (the game deactivates a held item along
/// with the rest while it's paused).
fn held_object(w: &World, id: i32) -> Option<ObjectId> {
    let h = w.find(id).or_else(|| {
        w.with_state(State::Inactive)
            .find(|&h| w.instance(h).id == id)
    })?;
    Some(w.instance(h).object)
}

/// Tells when a room starts, from one frame to the next: the room changed, or `room_restart` replaced every
/// instance (its id stays the same). Restarting creates new instances, with new ids, for all but the
/// persistent ones, so none that was there the frame before is left.
#[derive(Default)]
struct RoomStarts {
    room: Option<i32>,
    /// The highest id of the last frame's instances that aren't persistent, and how many of this frame's
    /// have an id up to it.
    last: Option<i32>,
    highest: Option<i32>,
    kept: usize,
}

impl RoomStarts {
    /// Before a frame's instances are [`see`](Self::see)n.
    fn begin(&mut self) {
        self.last = self.highest.take();
        self.kept = 0;
    }

    fn see(&mut self, i: &gm::Instance) {
        if !i.persistent {
            self.highest = self.highest.max(Some(i.id));
            self.kept += usize::from(self.last.is_some_and(|l| i.id <= l));
        }
    }

    /// Whether this frame started a room (not the run's first frame).
    fn started(&mut self, room: i32) -> bool {
        let changed = self.room.is_some_and(|r| r != room);
        self.room = Some(room);
        changed || (self.last.is_some() && self.kept == 0)
    }
}

/// The object called `name`.
fn object_named(p: &gm::Program, name: &str) -> Option<ObjectId> {
    (0..p.object_slots())
        .map(|i| ObjectId(i as u32))
        .find(|&o| p.object(o).is_some_and(|i| i.name == name))
}

/// How many of `a` (sorted) aren't in `b` (sorted).
fn missing_from(a: &[i32], b: &[i32]) -> u32 {
    let mut n = 0;
    let mut j = 0;
    for &x in a {
        while j < b.len() && b[j] < x {
            j += 1;
        }
        if b.get(j) != Some(&x) {
            n += 1;
        }
    }
    n
}

fn compare(x: &Value, op: Cmp, lit: &Lit) -> bool {
    match (x, lit) {
        (Value::Real(x), Lit::Real(y)) => match op {
            Cmp::Eq => x == y,
            Cmp::Ne => x != y,
            Cmp::Lt => x < y,
            Cmp::Le => x <= y,
            Cmp::Gt => x > y,
            Cmp::Ge => x >= y,
        },
        (Value::Str(s), Lit::Str(t)) => (s.as_bytes() == t.as_bytes()) == (op == Cmp::Eq),
        // A string isn't any number, nor a number any string.
        _ => op == Cmp::Ne,
    }
}

/// How many changes a global may make and still be listed change by change.
const RARE_CHANGES: usize = 16;
/// How many distinct frames are listed for an object's creations or destructions.
const FRAMES_LISTED: usize = 8;

/// Each change of a value: the frame, and the new value (`None` while unset).
type Changes<T> = Vec<(u64, Option<T>)>;

/// What happens in a run, for people: rooms entered, the globals that rarely change, the player's held
/// item and death, and when each object's instances appear and go away (as [`What::Created`] and
/// [`What::Destroyed`] count them).
#[derive(Default)]
pub struct Timeline {
    rooms: Vec<(u64, String)>,
    /// Each global's value on the first frame, and its changes.
    globals: Vec<(Option<Value>, Changes<Value>)>,
    held: Option<String>,
    player: Vec<(u64, String)>,
    room: Option<i32>,
    /// Each object's creations and destructions, by name.
    objects: BTreeMap<String, ObjectLog>,
    /// Object index -> its instances' ids on the last frame, sorted.
    ids: Vec<Vec<i32>>,
    starts: RoomStarts,
    started: bool,
    /// Globals or `oX.var`s asked for ([`watch`](Self::watch)), and their changes (`None` also without an
    /// instance).
    watched: Vec<(String, Changes<String>)>,
}

#[derive(Default)]
struct ObjectLog {
    created: Vec<(u64, u32)>,
    destroyed: Vec<(u64, u32)>,
}

impl Timeline {
    pub fn new() -> Self {
        Self::default()
    }

    /// Also lists every change of these: globals, or `oX.var` (on the first active instance of `oX`).
    pub fn watch(&mut self, names: &[String]) {
        self.watched = names.iter().map(|n| (n.clone(), Vec::new())).collect();
    }

    pub fn frame(&mut self, frame: u64, w: &World) {
        let p = w.program();
        for (name, changes) in &mut self.watched {
            let value = match name.split_once('.') {
                None => game::global_named(p, name).and_then(|g| w.gget(g).ok()),
                Some((object, var)) => w
                    .active()
                    .find(|&h| {
                        p.object(w.instance(h).object)
                            .is_some_and(|o| o.name == object)
                    })
                    .and_then(|h| {
                        let (v, _) = (p.layout(w.instance(h).object).vars())
                            .find(|&(v, _)| p.var_name(v) == var)?;
                        w.get(h, v).ok()
                    }),
            }
            .map(|v| show(&v));
            if changes.last().is_none_or(|(_, last)| *last != value) {
                changes.push((frame, value));
            }
        }
        let room = w.room().id;
        if self.room != Some(room) {
            let name = w.assets().room_name(room).unwrap_or("?").to_string();
            let level = game::global_named(p, "currLevel")
                .and_then(|g| w.gget(g).ok())
                .map_or(String::new(), |v| format!(" (currLevel {})", show(&v)));
            self.rooms.push((frame, format!("{name}{level}")));
            self.room = Some(room);
        }
        if !self.started {
            self.globals = (0..p.global_count())
                .map(|i| (w.gget(GVar(i as u16)).ok(), Vec::new()))
                .collect();
            self.ids = vec![Vec::new(); p.object_slots()];
        } else {
            for (i, (last, changes)) in self.globals.iter_mut().enumerate() {
                let now = w.gget(GVar(i as u16)).ok();
                let same = changes.last().map_or(last as &Option<Value>, |(_, v)| v) == &now;
                if !same {
                    changes.push((frame, now));
                }
            }
        }
        // The player: what they hold and whether they're dead (oPlayer1's holdItem and dead).
        let player = w.active().find(|&h| {
            p.object(w.instance(h).object)
                .is_some_and(|o| o.name == "oPlayer1")
        });
        if let Some(h) = player {
            let var = |name: &str| {
                let (v, _) = (p.layout(w.instance(h).object).vars())
                    .find(|&(v, _)| p.var_name(v) == name)?;
                w.get(h, v).ok()
            };
            let held = match var("holdItem") {
                Some(Value::Real(id)) if id > 0.0 => {
                    held_object(w, id as i32).and_then(|o| Some(p.object(o)?.name.clone()))
                }
                _ => None,
            };
            if held != self.held {
                let what = held
                    .as_deref()
                    .map_or("holds nothing".into(), |o| format!("holds {o}"));
                self.player.push((frame, what));
                self.held = held;
            }
            if var("dead").is_some_and(|v| v.is_truthy())
                && self.player.last().is_none_or(|(_, s)| s != "dead")
            {
                self.player.push((frame, "dead".into()));
            }
        }
        let mut now: Vec<Vec<i32>> = vec![Vec::new(); self.ids.len()];
        self.starts.begin();
        for h in w.live() {
            let i = w.instance(h);
            self.starts.see(i);
            if let Some(v) = now.get_mut(i.object.0 as usize) {
                v.push(i.id);
            }
        }
        let new_room = self.starts.started(room);
        for (o, ids) in now.iter_mut().enumerate() {
            ids.sort_unstable();
            let before = &self.ids[o];
            let created = missing_from(ids, before);
            let destroyed = if new_room {
                0
            } else {
                missing_from(before, ids)
            };
            if created + destroyed > 0 {
                let name = p
                    .object(ObjectId(o as u32))
                    .map_or("?".to_string(), |i| i.name.clone());
                let log = self.objects.entry(name).or_default();
                if created > 0 {
                    log.created.push((frame, created));
                }
                if destroyed > 0 {
                    log.destroyed.push((frame, destroyed));
                }
            }
        }
        self.ids = now;
        self.started = true;
    }

    /// The timeline as text, with the program's global names.
    pub fn to_text(&self, p: &gm::Program) -> String {
        let mut s = String::from("rooms:");
        for (f, r) in &self.rooms {
            let _ = write!(s, "\n  {f} {r}");
        }
        s += "\nplayer:";
        for (f, what) in &self.player {
            let _ = write!(s, " {f} {what};");
        }
        s += "\nglobals that changed:";
        let mut busy = Vec::new();
        for (i, (first, changes)) in self.globals.iter().enumerate() {
            if changes.is_empty() {
                continue;
            }
            let name = p.global_name(GVar(i as u16));
            if changes.len() > RARE_CHANGES {
                busy.push(format!("{name} ({})", changes.len()));
                continue;
            }
            let show_opt = |v: &Option<Value>| v.as_ref().map_or("unset".into(), show);
            let _ = write!(s, "\n  {name}: {}", show_opt(first));
            for (f, v) in changes {
                let _ = write!(s, ", {f}: {}", show_opt(v));
            }
        }
        if !busy.is_empty() {
            let _ = write!(s, "\n  changed often: {}", busy.join(", "));
        }
        for (name, changes) in &self.watched {
            let _ = write!(s, "\n{name}:");
            for (f, v) in changes {
                let _ = write!(s, " {f}:{}", v.as_deref().unwrap_or("-"));
            }
        }
        s += "\nobjects (+ appeared, - went away, frame x count):";
        for (name, log) in &self.objects {
            let list = |events: &[(u64, u32)]| {
                let total: u32 = events.iter().map(|e| e.1).sum();
                let mut l: Vec<String> = (events.iter().take(FRAMES_LISTED))
                    .map(|&(f, n)| {
                        if n == 1 {
                            f.to_string()
                        } else {
                            format!("{f}x{n}")
                        }
                    })
                    .collect();
                if events.len() > FRAMES_LISTED {
                    l.push(format!("... ({total} in all)"));
                }
                l.join(" ")
            };
            let _ = write!(s, "\n  {name}:");
            if !log.created.is_empty() {
                let _ = write!(s, " +{}", list(&log.created));
            }
            if !log.destroyed.is_empty() {
                let _ = write!(s, " -{}", list(&log.destroyed));
            }
        }
        s.push('\n');
        s
    }
}

/// A value as a story fact writes it.
fn show(v: &Value) -> String {
    match v {
        Value::Real(x) => format!("{x}"),
        Value::Str(s) => format!("{:?}", s.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facts_parse() {
        let s = Story::parse(
            "seed 2\n# A comment. story: not this\n#story: +oDiamond@8742 -oSpider*6@1561..1707\n\
             # story: !plife<1 room=rOlmec@..4700 oShopkeeper.status=angry money>=4100@100..\n",
        )
        .unwrap();
        let f = &s.facts;
        assert_eq!(f.len(), 6);
        assert_eq!(
            f[0].what,
            What::Created {
                object: "oDiamond".into(),
                count: 1
            }
        );
        assert_eq!(f[0].when, Some((8742 - SLACK, 8742 + SLACK)));
        assert_eq!(
            f[1].what,
            What::Destroyed {
                object: "oSpider".into(),
                count: 6
            }
        );
        assert_eq!(f[1].when, Some((1561, 1707)));
        assert!(f[2].negated);
        assert_eq!(
            f[2].what,
            What::Holds {
                subject: Subject::Global("plife".into()),
                op: Cmp::Lt,
                value: Lit::Real(1.0)
            }
        );
        assert_eq!(f[3].when, Some((0, 4700)));
        assert_eq!(
            f[5].what,
            What::Holds {
                subject: Subject::Global("money".into()),
                op: Cmp::Ge,
                value: Lit::Real(4100.0)
            }
        );
        assert_eq!(f[5].when, Some((100, u64::MAX)));
        assert_eq!(
            f[4].what,
            What::Holds {
                subject: Subject::Var {
                    object: "oShopkeeper".into(),
                    var: "status".into()
                },
                op: Cmp::Eq,
                value: Lit::Str("angry".into())
            }
        );
        assert_eq!(
            Fact::parse("held!=oJar").unwrap().what,
            What::Holds {
                subject: Subject::Held,
                op: Cmp::Ne,
                value: Lit::Str("oJar".into())
            }
        );
        let s =
            Story::parse(r#"# story: message="A CAPE FOR $8@1."@300 !message2="" x="2""#).unwrap();
        let f = &s.facts;
        assert_eq!(f.len(), 3);
        assert_eq!(f[0].text, r#"message="A CAPE FOR $8@1."@300"#);
        assert_eq!(
            f[0].what,
            What::Holds {
                subject: Subject::Global("message".into()),
                op: Cmp::Eq,
                value: Lit::Str("A CAPE FOR $8@1.".into())
            }
        );
        assert_eq!(f[0].when, Some((300 - SLACK, 300 + SLACK)));
        assert_eq!(f[1].when, None);
        assert!(matches!(&f[1].what, What::Holds { value: Lit::Str(s), .. } if s.is_empty()));
        // Quoted, a number is a string.
        assert!(matches!(&f[2].what, What::Holds { value: Lit::Str(s), .. } if s == "2"));
        assert!(Story::parse(r#"# story: message="A CAPE"#).is_err());
        for bad in [
            "held=3", "oDiamond", "+", "+oX*0", "plife<x", "room=3", "x=1@5..2", "x=1@y", "a.b.c=1",
        ] {
            assert!(Fact::parse(bad).is_err(), "{bad}");
        }
        for bad in [r#"x="a"#, r#"x="a"b""#, r#"x<"1""#] {
            assert!(Fact::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_story_tells_what_happens() {
        let tells = |text: &str| Story::parse(text).unwrap().tells();
        assert!(!tells("seed 1\n# No story.\n"));
        assert!(!tells("# story: !plife<1 !+oTunnelMan@5..\n"));
        assert!(tells("# story: !plife<1 currLevel=2@900\n"));
    }

    #[test]
    fn values_compare_as_the_fact_says() {
        let r = |x: f64| Value::Real(x);
        assert!(compare(&r(3.0), Cmp::Lt, &Lit::Real(4.0)));
        assert!(!compare(&r(4.0), Cmp::Lt, &Lit::Real(4.0)));
        assert!(compare(&r(4.0), Cmp::Ge, &Lit::Real(4.0)));
        let s = Value::from("Weapon");
        assert!(compare(&s, Cmp::Eq, &Lit::Str("Weapon".into())));
        assert!(compare(&s, Cmp::Ne, &Lit::Str("General".into())));
        assert!(!compare(&s, Cmp::Eq, &Lit::Real(0.0)));
        assert!(compare(&s, Cmp::Ne, &Lit::Real(0.0)));
    }

    #[test]
    fn missing_counts_ids_gone() {
        assert_eq!(missing_from(&[1, 2, 3, 5], &[2, 5, 6]), 2);
        assert_eq!(missing_from(&[], &[1]), 0);
        assert_eq!(missing_from(&[4], &[]), 1);
    }
}
