//! Finds the first frame where two traces diverge (the engine behind `trace-diff`).
//!
//! Frames are compared pairwise in order. Within a frame, every field is flattened to a *path* (see
//! [`crate::filter`]) and compared exactly, or within an absolute tolerance for reals. A field that one trace
//! didn't dump at all (the oracle without `OPENGMK_DUMP_FIELDS` has no `vars`; only the port dumps
//! `deactivated`) isn't compared; the report lists such fields instead of flagging every instance. Inside a
//! map both traces dumped (`vars`, `globals`), a key on one side only is a difference. A missing alarm is -1.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::fmt::{self, Write as _};
use std::io::BufRead;

use serde::Serialize;
use serde::ser::Serializer;

use crate::Error;
use crate::filter::Filter;
use crate::trace::{ALARMS, Frame, Instance, TraceReader};
use crate::value::{Real, Scalar, Value, index_suffix};

/// How instances in one trace are paired with instances in the other.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Matching {
    /// By id when the objects agree; the rest by object and creation order.
    #[default]
    Auto,
    /// By id only, even across objects (an object mismatch is then a difference).
    Id,
    /// By object and creation order only, for traces whose ids are allocated differently.
    Object,
}

impl Matching {
    pub fn parse(s: &str) -> Option<Matching> {
        match s {
            "auto" => Some(Matching::Auto),
            "id" => Some(Matching::Id),
            "object" => Some(Matching::Object),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DiffOptions {
    pub filter: Filter,
    /// Reals at most this far apart are equal. 0 compares exactly.
    pub tolerance: f64,
    pub matching: Matching,
    /// How many equal frames before the divergence to show the differing fields' history for.
    pub context: usize,
    /// How many differences to report in detail; 0 for all.
    pub max_differences: usize,
    /// Only compare frames from this one on.
    pub from: Option<u64>,
    /// Only compare frames up to this one (inclusive).
    pub to: Option<u64>,
}

impl Default for DiffOptions {
    fn default() -> Self {
        DiffOptions {
            filter: Filter::default(),
            tolerance: 0.0,
            matching: Matching::Auto,
            context: 3,
            max_differences: 50,
            from: None,
            to: None,
        }
    }
}

/// Which trace something came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    A,
    B,
}

impl fmt::Display for Side {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Side::A => "a",
            Side::B => "b",
        })
    }
}

/// A malformed trace, and which one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffError {
    pub side: Side,
    pub error: Error,
}

impl fmt::Display for DiffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "trace {}: {}", self.side, self.error)
    }
}

impl std::error::Error for DiffError {}

/// One compared value. Serializes as plain JSON (non-finite reals as `{"real":"nan"}`).
#[derive(Clone, Debug, PartialEq)]
pub enum Datum {
    /// Not present: the instance or key doesn't exist on this side.
    Absent,
    Real(Real),
    Int(i64),
    Bool(bool),
    Text(String),
    Scalar(Scalar),
}

impl Datum {
    fn matches(&self, other: &Datum, tolerance: f64) -> bool {
        match (self, other) {
            (Datum::Real(a), Datum::Real(b)) => a.matches(*b, tolerance),
            (Datum::Scalar(a), Datum::Scalar(b)) => a.matches(b, tolerance),
            _ => self == other,
        }
    }

    fn is_absent(&self) -> bool {
        *self == Datum::Absent
    }
}

impl fmt::Display for Datum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Datum::Absent => f.write_str("<absent>"),
            Datum::Real(r) => r.fmt(f),
            Datum::Int(i) => i.fmt(f),
            Datum::Bool(b) => b.fmt(f),
            Datum::Text(s) => write!(f, "{s:?}"),
            Datum::Scalar(s) => s.fmt(f),
        }
    }
}

impl Serialize for Datum {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Datum::Absent => s.serialize_none(),
            Datum::Real(r) => Scalar::Real(*r).serialize(s),
            Datum::Int(i) => s.serialize_i64(*i),
            Datum::Bool(b) => s.serialize_bool(*b),
            Datum::Text(t) => s.serialize_str(t),
            Datum::Scalar(v) => v.serialize(s),
        }
    }
}

/// An instance, as named in a report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InstanceRef {
    pub id: i32,
    pub object: String,
}

impl fmt::Display for InstanceRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} #{}", self.object, self.id)
    }
}

/// One difference in the diverging frame.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Difference {
    /// A field differs. Without instances it's a frame-level field (`seed`, `globals.money`...).
    Field {
        #[serde(skip_serializing_if = "Option::is_none")]
        a_instance: Option<InstanceRef>,
        #[serde(skip_serializing_if = "Option::is_none")]
        b_instance: Option<InstanceRef>,
        path: String,
        /// `null` when absent on this side.
        a: Datum,
        b: Datum,
    },
    /// An instance with no counterpart in the other trace.
    OnlyIn {
        side: Side,
        instance: InstanceRef,
        active: bool,
    },
}

/// A differing field's values over the frames before the divergence, and at it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct History {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub a_instance: Option<InstanceRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub b_instance: Option<InstanceRef>,
    pub path: String,
    pub points: Vec<HistoryPoint>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HistoryPoint {
    /// Trace a's frame number (the traces agree on it up to the divergence).
    pub frame: u64,
    pub a: Datum,
    pub b: Datum,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Divergence {
    pub frame: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_name_a: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_name_b: Option<String>,
    /// The number of differences in this frame; `differences` may be truncated.
    pub total_differences: usize,
    pub differences: Vec<Difference>,
    /// For each reported field difference, its values over the preceding compared frames.
    pub history: Vec<History>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// Every frame is in both traces, and equal.
    Identical,
    /// Some frame in both traces differs.
    Diverged,
    /// Every frame in both traces is equal, but some frames are in only one (one trace is longer, starts
    /// later, or skips frames).
    UnmatchedFrames,
    /// No frame (in range) is in both traces.
    NothingCompared,
}

/// Frames (in range) that only one trace has.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Unmatched {
    pub count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last: Option<u64>,
}

impl Unmatched {
    fn add(&mut self, frame: u64) {
        self.count += 1;
        self.first.get_or_insert(frame);
        self.last = Some(frame);
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Report {
    pub outcome: Outcome,
    /// Frames in both traces, compared and found equal.
    pub equal_frames: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_equal_frame: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub divergence: Option<Divergence>,
    /// Frames only one trace has, up to the divergence (or the end).
    pub frames_only_in_a: Unmatched,
    pub frames_only_in_b: Unmatched,
    /// Fields only one trace dumped, so they weren't compared (`vars`, `deactivated`, `x`...).
    pub not_compared_only_in_a: BTreeSet<String>,
    pub not_compared_only_in_b: BTreeSet<String>,
    pub warnings: Vec<String>,
}

/// Compares two traces, pairing frames by number, and stops at the first divergence.
pub fn diff_traces<A: BufRead, B: BufRead>(
    a: TraceReader<A>,
    b: TraceReader<B>,
    opts: &DiffOptions,
) -> Result<Report, DiffError> {
    let mut report = Report {
        outcome: Outcome::Identical,
        equal_frames: 0,
        last_equal_frame: None,
        divergence: None,
        frames_only_in_a: Unmatched::default(),
        frames_only_in_b: Unmatched::default(),
        not_compared_only_in_a: BTreeSet::new(),
        not_compared_only_in_b: BTreeSet::new(),
        warnings: Vec::new(),
    };
    let mut filters = vec![opts.filter.clone()];
    for (side, header) in [(Side::A, a.header()), (Side::B, b.header())] {
        if let Some(filter) = header.and_then(|h| h.filter.as_ref()) {
            report.warnings.push(format!(
                "trace {side} was written with a filter ({}); what it dropped isn't compared in either trace",
                serde_json::to_string(filter).unwrap_or_default()
            ));
            filters.push(filter.clone());
        }
    }
    let filters = Filters(filters);
    let mut a = InRange {
        reader: a,
        side: Side::A,
        opts,
    };
    let mut b = InRange {
        reader: b,
        side: Side::B,
        opts,
    };
    let (mut fa, mut fb) = (a.next()?, b.next()?);
    // Not preallocated: `context` comes from the command line.
    let mut history: VecDeque<(Frame, Frame)> = VecDeque::new();
    loop {
        // Advance whichever trace is behind; its frame has no counterpart.
        let (frame_a, frame_b) = match (&fa, &fb) {
            (None, None) => break,
            (Some(x), Some(y)) => (x, y),
            (Some(x), None) => {
                report.frames_only_in_a.add(x.frame);
                fa = a.next()?;
                continue;
            }
            (None, Some(y)) => {
                report.frames_only_in_b.add(y.frame);
                fb = b.next()?;
                continue;
            }
        };
        if frame_a.frame < frame_b.frame {
            report.frames_only_in_a.add(frame_a.frame);
            fa = a.next()?;
            continue;
        }
        if frame_b.frame < frame_a.frame {
            report.frames_only_in_b.add(frame_b.frame);
            fb = b.next()?;
            continue;
        }
        let mut cmp = Comparison::new(opts, &filters);
        cmp.frames(frame_a, frame_b);
        report.not_compared_only_in_a.extend(cmp.only_a);
        report.not_compared_only_in_b.extend(cmp.only_b);
        if !cmp.differences.is_empty() {
            report.outcome = Outcome::Diverged;
            report.divergence = Some(divergence(
                cmp.differences,
                frame_a,
                frame_b,
                &history,
                opts,
                &filters,
            ));
            return Ok(report);
        }
        report.equal_frames += 1;
        report.last_equal_frame = Some(frame_a.frame);
        let pair = (
            fa.take().expect("matched above"),
            fb.take().expect("matched above"),
        );
        if opts.context > 0 {
            if history.len() == opts.context {
                history.pop_front();
            }
            history.push_back(pair);
        }
        (fa, fb) = (a.next()?, b.next()?);
    }
    if report.equal_frames == 0 {
        // Don't let an empty range or two empty traces pass as identical.
        report.outcome = Outcome::NothingCompared;
    } else if report.frames_only_in_a.count > 0 || report.frames_only_in_b.count > 0 {
        report.outcome = Outcome::UnmatchedFrames;
    }
    Ok(report)
}

/// Every filter that applies: the caller's, plus any the traces were written with. A field or instance is
/// compared only if all of them keep it, so whatever a writer dropped is left out of both sides.
struct Filters(Vec<Filter>);

impl Filters {
    fn keeps_object(&self, object: &str) -> bool {
        self.0.iter().all(|f| f.keeps_object(object))
    }

    fn keeps_path(&self, path: &str) -> bool {
        self.0.iter().all(|f| f.keeps_path(path))
    }
}

/// A trace restricted to the frames in `opts.from..=opts.to`.
struct InRange<'o, R: BufRead> {
    reader: TraceReader<R>,
    side: Side,
    opts: &'o DiffOptions,
}

impl<R: BufRead> InRange<'_, R> {
    fn next(&mut self) -> Result<Option<Frame>, DiffError> {
        loop {
            let frame = self.reader.next().transpose().map_err(|error| DiffError {
                side: self.side,
                error,
            })?;
            match frame {
                Some(f) if self.opts.from.is_some_and(|from| f.frame < from) => continue,
                // Frames increase, so nothing later is in range either.
                Some(f) if self.opts.to.is_some_and(|to| f.frame > to) => return Ok(None),
                other => return Ok(other),
            }
        }
    }
}

fn divergence(
    all: Vec<Difference>,
    fa: &Frame,
    fb: &Frame,
    history: &VecDeque<(Frame, Frame)>,
    opts: &DiffOptions,
    filters: &Filters,
) -> Divergence {
    let total = all.len();
    let shown: Vec<Difference> = if opts.max_differences == 0 {
        all
    } else {
        all.into_iter().take(opts.max_differences).collect()
    };
    let mut histories = Vec::new();
    // Each history frame's fields, flattened once per (frame, side, instance).
    let mut cache: HashMap<(usize, Side, Option<i32>), Option<FieldMap>> = HashMap::new();
    let mut lookup =
        |n: usize, side: Side, frame: &Frame, target: Option<&InstanceRef>, path: &str| {
            cache
                .entry((n, side, target.map(|r| r.id)))
                .or_insert_with(|| flatten(frame, target, filters))
                .as_ref()
                .and_then(|fields| fields.get(path).cloned())
                .unwrap_or(Datum::Absent)
        };
    for d in &shown {
        let Difference::Field {
            a_instance,
            b_instance,
            path,
            a,
            b,
        } = d
        else {
            continue;
        };
        let mut points: Vec<HistoryPoint> = history
            .iter()
            .enumerate()
            .map(|(n, (ha, hb))| HistoryPoint {
                frame: ha.frame,
                a: lookup(n, Side::A, ha, a_instance.as_ref(), path),
                b: lookup(n, Side::B, hb, b_instance.as_ref(), path),
            })
            .collect();
        points.push(HistoryPoint {
            frame: fa.frame,
            a: a.clone(),
            b: b.clone(),
        });
        histories.push(History {
            a_instance: a_instance.clone(),
            b_instance: b_instance.clone(),
            path: path.clone(),
            points,
        });
    }
    Divergence {
        frame: fa.frame,
        room_name_a: fa.room_name.clone(),
        room_name_b: fb.room_name.clone(),
        total_differences: total,
        differences: shown,
        history: histories,
    }
}

/// Fields by path.
type FieldMap = HashMap<String, Datum>;

/// A frame's fields, or one instance's, by path; `None` when the instance isn't in the frame.
fn flatten(frame: &Frame, instance: Option<&InstanceRef>, filters: &Filters) -> Option<FieldMap> {
    let fields = match instance {
        None => frame_fields(frame, filters).0,
        Some(r) => {
            let mut all = frame
                .instances
                .iter()
                .map(|i| (i, true))
                .chain(frame.deactivated.iter().flatten().map(|i| (i, false)));
            let (inst, active) = all.find(|(i, _)| i.id == r.id)?;
            instance_fields(inst, active, filters).0
        }
    };
    Some(
        fields
            .into_iter()
            .map(|(p, d)| (p.into_owned(), d))
            .collect(),
    )
}

/// Flattened fields, plus the maps that were dumped (so a key missing from one is a real difference).
type Fields = (Vec<(Cow<'static, str>, Datum)>, Vec<&'static str>);

fn frame_fields(f: &Frame, filter: &Filters) -> Fields {
    let mut out: Vec<(Cow<'static, str>, Datum)> = Vec::new();
    let mut push = |path: &'static str, d: Option<Datum>| {
        if let Some(d) = d
            && filter.keeps_path(path)
        {
            out.push((Cow::Borrowed(path), d));
        }
    };
    push("room", f.room.map(|r| Datum::Int(r.into())));
    push("room_name", f.room_name.clone().map(Datum::Text));
    push("seed", f.seed.map(|s| Datum::Int(s.into())));
    push(
        "instance_count",
        f.instance_count.map(|c| Datum::Int(c as i64)),
    );
    let mut maps = Vec::new();
    for (prefix, map) in [("globals", &f.globals), ("builtins", &f.builtins)] {
        if let Some(map) = map {
            maps.push(prefix);
            push_vars(&mut out, prefix, map, filter);
        }
    }
    (out, maps)
}

fn instance_fields(inst: &Instance, active: bool, filter: &Filters) -> Fields {
    // `object` is identity, so no filter drops it.
    let mut out: Vec<(Cow<'static, str>, Datum)> =
        vec![(Cow::Borrowed("object"), Datum::Text(inst.object.clone()))];
    let mut push = |path: Cow<'static, str>, d: Option<Datum>| {
        if let Some(d) = d
            && filter.keeps_path(&path)
        {
            out.push((path, d));
        }
    };
    push("active".into(), Some(Datum::Bool(active)));
    push(
        "object_index".into(),
        inst.object_index.map(|v| Datum::Int(v.into())),
    );
    for name in Instance::REAL_FIELDS {
        push(name.into(), inst.real(name).map(Datum::Real));
    }
    push(
        "sprite_index".into(),
        inst.sprite_index.map(|v| Datum::Int(v.into())),
    );
    push("sprite".into(), inst.sprite.clone().map(Datum::Text));
    push("visible".into(), inst.visible.map(Datum::Bool));
    push("solid".into(), inst.solid.map(Datum::Bool));
    if inst.alarms.is_some() {
        for i in 0..ALARMS {
            push(
                format!("alarms[{i}]").into(),
                inst.alarm(i).map(|v| Datum::Int(v.into())),
            );
        }
    }
    let mut maps = Vec::new();
    if let Some(vars) = &inst.vars {
        maps.push("vars");
        push_vars(&mut out, "vars", vars, filter);
    }
    (out, maps)
}

/// Flattens variables. Element 0 is the variable itself (GML reads `a` as `a[0]`), so a scalar and an array
/// starting with the same value only differ in their other elements.
fn push_vars(
    out: &mut Vec<(Cow<'static, str>, Datum)>,
    prefix: &str,
    vars: &BTreeMap<String, Value>,
    filter: &Filters,
) {
    let mut push = |name: &str, index: u32, element: Scalar| {
        let path = if index == 0 {
            format!("{prefix}.{name}")
        } else {
            format!("{prefix}.{name}{}", index_suffix(index))
        };
        if filter.keeps_path(&path) {
            out.push((path.into(), Datum::Scalar(element)));
        }
    };
    for (name, value) in vars {
        match value {
            Value::Real(r) => push(name, 0, Scalar::Real(*r)),
            Value::Str(v) => push(name, 0, Scalar::Str(v.clone())),
            Value::Array(elements) => {
                for (index, element) in elements {
                    push(name, *index, element.clone());
                }
            }
        }
    }
}

/// The map a path belongs to, or the path's own field.
fn group(path: &str) -> &str {
    let end = path.find(['.', '[']).unwrap_or(path.len());
    &path[..end]
}

struct Comparison<'o> {
    opts: &'o DiffOptions,
    filters: &'o Filters,
    differences: Vec<Difference>,
    only_a: BTreeSet<String>,
    only_b: BTreeSet<String>,
}

impl<'o> Comparison<'o> {
    fn new(opts: &'o DiffOptions, filters: &'o Filters) -> Self {
        Comparison {
            opts,
            filters,
            differences: Vec::new(),
            only_a: BTreeSet::new(),
            only_b: BTreeSet::new(),
        }
    }

    fn frames(&mut self, fa: &Frame, fb: &Frame) {
        // Fast path: equal values always match, so only flatten what differs.
        let same = fa.room == fb.room
            && fa.room_name == fb.room_name
            && fa.seed == fb.seed
            && fa.instance_count == fb.instance_count
            && fa.globals == fb.globals
            && fa.builtins == fb.builtins;
        if !same {
            let filter = self.filters;
            self.fields(None, frame_fields(fa, filter), frame_fields(fb, filter));
        }
        self.instances(fa, fb);
    }

    /// Compares flattened fields; `refs` names the instance pair, if any.
    fn fields(
        &mut self,
        refs: Option<(&InstanceRef, &InstanceRef)>,
        (a, maps_a): Fields,
        (b, maps_b): Fields,
    ) {
        let mut b_index: HashMap<&str, usize> = b
            .iter()
            .enumerate()
            .map(|(n, (p, _))| (p.as_ref(), n))
            .collect();
        let mut pairs: Vec<(&str, &Datum, &Datum)> = Vec::new();
        for (path, da) in &a {
            match b_index.remove(path.as_ref()) {
                Some(n) => pairs.push((path, da, &b[n].1)),
                None => pairs.push((path, da, &Datum::Absent)),
            }
        }
        let mut b_only: Vec<usize> = b_index.into_values().collect();
        b_only.sort_unstable();
        pairs.extend(
            b_only
                .into_iter()
                .map(|n| (b[n].0.as_ref(), &Datum::Absent, &b[n].1)),
        );
        for (path, da, db) in pairs {
            if da.is_absent() != db.is_absent() {
                let g = group(path);
                // A field (or whole map) only one side dumped isn't comparable.
                let dumped_by_both = maps_a.contains(&g) && maps_b.contains(&g);
                if !dumped_by_both {
                    let set = if da.is_absent() {
                        &mut self.only_b
                    } else {
                        &mut self.only_a
                    };
                    set.insert(g.to_string());
                    continue;
                }
            }
            if !da.matches(db, self.opts.tolerance) {
                self.differences.push(Difference::Field {
                    a_instance: refs.map(|(r, _)| r.clone()),
                    b_instance: refs.map(|(_, r)| r.clone()),
                    path: path.to_string(),
                    a: da.clone(),
                    b: db.clone(),
                });
            }
        }
    }

    fn instances(&mut self, fa: &Frame, fb: &Frame) {
        let filter = self.filters;
        // Deactivated instances are only comparable when both traces dumped them.
        let both_dump_deactivated = fa.deactivated.is_some() && fb.deactivated.is_some();
        for (frame, set) in [(fa, &mut self.only_a), (fb, &mut self.only_b)] {
            if frame.deactivated.is_some() && !both_dump_deactivated {
                set.insert("deactivated".into());
            }
        }
        let (a, b) = (
            gather(fa, both_dump_deactivated, filter),
            gather(fb, both_dump_deactivated, filter),
        );
        let (pairs, only_a, only_b) = match_instances(&a, &b, self.opts.matching);
        for (ia, ib) in pairs {
            let ((inst_a, act_a), (inst_b, act_b)) = (a[ia], b[ib]);
            if act_a == act_b && inst_a == inst_b {
                continue; // equal values always match
            }
            let refs = (iref(inst_a), iref(inst_b));
            let mut fields_a = instance_fields(inst_a, act_a, filter);
            let mut fields_b = instance_fields(inst_b, act_b, filter);
            if self.opts.matching != Matching::Id {
                // Paired by object, so the objects agree; don't report it as a field.
                fields_a.0.retain(|(p, _)| p != "object");
                fields_b.0.retain(|(p, _)| p != "object");
            }
            self.fields(Some((&refs.0, &refs.1)), fields_a, fields_b);
        }
        for (side, list, idx) in [(Side::A, &a, only_a), (Side::B, &b, only_b)] {
            for n in idx {
                let (inst, active) = list[n];
                self.differences.push(Difference::OnlyIn {
                    side,
                    instance: iref(inst),
                    active,
                });
            }
        }
    }
}

/// The instances to compare, each with whether it's active.
fn gather<'f>(f: &'f Frame, deactivated: bool, filter: &Filters) -> Vec<(&'f Instance, bool)> {
    let inactive = f
        .deactivated
        .iter()
        .flatten()
        .filter(|_| deactivated)
        .map(|i| (i, false));
    f.instances
        .iter()
        .map(|i| (i, true))
        .chain(inactive)
        .filter(|(i, _)| filter.keeps_object(&i.object))
        .collect()
}

fn iref(i: &Instance) -> InstanceRef {
    InstanceRef {
        id: i.id,
        object: i.object.clone(),
    }
}

type Matched = (Vec<(usize, usize)>, Vec<usize>, Vec<usize>);

/// Pairs instances; returns the pairs (sorted by a's order) and the unpaired indices of each side.
fn match_instances(a: &[(&Instance, bool)], b: &[(&Instance, bool)], mode: Matching) -> Matched {
    let mut pairs = Vec::new();
    let mut used_b = vec![false; b.len()];
    let mut left_a = Vec::new();
    if mode == Matching::Object {
        left_a.extend(0..a.len());
    } else {
        let by_id: HashMap<i32, usize> =
            b.iter().enumerate().map(|(n, (i, _))| (i.id, n)).collect();
        for (n, (inst, _)) in a.iter().enumerate() {
            match by_id.get(&inst.id) {
                Some(&m) if mode == Matching::Id || b[m].0.object == inst.object => {
                    pairs.push((n, m));
                    used_b[m] = true;
                }
                _ => left_a.push(n),
            }
        }
    }
    let left_b: Vec<usize> = (0..b.len()).filter(|&m| !used_b[m]).collect();
    let (mut only_a, mut only_b) = (Vec::new(), Vec::new());
    if mode == Matching::Id {
        only_a = left_a;
        only_b = left_b;
    } else {
        // Fallback: within each object, pair in creation (id) order.
        let mut groups: BTreeMap<&str, (Vec<usize>, Vec<usize>)> = BTreeMap::new();
        for &n in &left_a {
            groups.entry(a[n].0.object.as_str()).or_default().0.push(n);
        }
        for &m in &left_b {
            groups.entry(b[m].0.object.as_str()).or_default().1.push(m);
        }
        for (mut ga, mut gb) in groups.into_values() {
            ga.sort_by_key(|&n| a[n].0.id);
            gb.sort_by_key(|&m| b[m].0.id);
            let k = ga.len().min(gb.len());
            pairs.extend(ga[..k].iter().copied().zip(gb[..k].iter().copied()));
            only_a.extend_from_slice(&ga[k..]);
            only_b.extend_from_slice(&gb[k..]);
        }
        only_a.sort_unstable();
        only_b.sort_unstable();
    }
    pairs.sort_unstable();
    (pairs, only_a, only_b)
}

impl Report {
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("reports serialize")
    }

    /// A readable summary.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        match self.outcome {
            Outcome::Identical => {
                let _ = writeln!(out, "identical: {} frames compared", self.equal_frames);
            }
            Outcome::NothingCompared => {
                let _ = writeln!(
                    out,
                    "nothing compared: no frame is in both traces (in range)"
                );
            }
            Outcome::UnmatchedFrames => {
                let _ = writeln!(
                    out,
                    "unmatched frames: the {} frames in both traces are equal, but some frames are in only one",
                    self.equal_frames
                );
            }
            Outcome::Diverged => {
                let d = self
                    .divergence
                    .as_ref()
                    .expect("diverged reports have a divergence");
                let frame = format!("frame {}", d.frame);
                let room = match (&d.room_name_a, &d.room_name_b) {
                    (Some(a), Some(b)) if a == b => format!(" in {a}"),
                    (Some(a), Some(b)) => format!(" in {a} (a) / {b} (b)"),
                    _ => String::new(),
                };
                let _ = writeln!(
                    out,
                    "diverged at {frame}{room}, after {} equal frames: {} difference{}",
                    self.equal_frames,
                    d.total_differences,
                    if d.total_differences == 1 { "" } else { "s" }
                );
                for diff in &d.differences {
                    let _ = writeln!(out, "  {}", describe(diff));
                }
                if d.differences.len() < d.total_differences {
                    let _ = writeln!(
                        out,
                        "  ... {} more",
                        d.total_differences - d.differences.len()
                    );
                }
                if d.history.iter().any(|h| h.points.len() > 1) {
                    let _ = writeln!(out, "history (frame: value, or a/b where they differ):");
                    for h in &d.history {
                        let points: Vec<String> = h
                            .points
                            .iter()
                            .map(|p| {
                                if p.a.matches(&p.b, 0.0) {
                                    format!("{}: {}", p.frame, p.a)
                                } else {
                                    format!("{}: {} / {}", p.frame, p.a, p.b)
                                }
                            })
                            .collect();
                        let _ = writeln!(
                            out,
                            "  {}: {}",
                            target(h.a_instance.as_ref(), h.b_instance.as_ref(), &h.path),
                            points.join(" | ")
                        );
                    }
                }
            }
        }
        for (side, u) in [("a", &self.frames_only_in_a), ("b", &self.frames_only_in_b)] {
            if let (Some(first), Some(last)) = (u.first, u.last) {
                let range = if first == last {
                    format!("frame {first}")
                } else {
                    format!("first {first}, last {last}")
                };
                let _ = writeln!(
                    out,
                    "{} frame{} only in trace {side} ({range})",
                    u.count,
                    if u.count == 1 { "" } else { "s" }
                );
            }
        }
        for (side, set) in [
            ("a", &self.not_compared_only_in_a),
            ("b", &self.not_compared_only_in_b),
        ] {
            if !set.is_empty() {
                let list: Vec<&str> = set.iter().map(String::as_str).collect();
                let _ = writeln!(
                    out,
                    "not compared, only in trace {side}: {}",
                    list.join(", ")
                );
            }
        }
        for w in &self.warnings {
            let _ = writeln!(out, "warning: {w}");
        }
        out
    }
}

fn target(a: Option<&InstanceRef>, b: Option<&InstanceRef>, path: &str) -> String {
    match (a, b) {
        (Some(a), Some(b)) if a == b => format!("{a} {path}"),
        (Some(a), Some(b)) if a.object == b.object => {
            format!("{} #{}/#{} {path}", a.object, a.id, b.id)
        }
        (Some(a), Some(b)) => format!("{a} / {b} {path}"),
        _ => path.to_string(),
    }
}

fn describe(d: &Difference) -> String {
    match d {
        Difference::Field {
            a_instance,
            b_instance,
            path,
            a,
            b,
        } => {
            format!(
                "{}: a={a} b={b}",
                target(a_instance.as_ref(), b_instance.as_ref(), path)
            )
        }
        Difference::OnlyIn {
            side,
            instance,
            active,
        } => {
            format!(
                "{instance}: only in trace {side}{}",
                if *active { "" } else { " (deactivated)" }
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace::TraceWriter;
    use crate::trace::tests::{frame, instance};

    fn text(frames: &[Frame]) -> String {
        let mut w = TraceWriter::new(Vec::new(), Filter::default());
        for f in frames {
            w.write_frame(f).unwrap();
        }
        String::from_utf8(w.into_inner()).unwrap()
    }

    fn diff_with(a: &[Frame], b: &[Frame], opts: &DiffOptions) -> Report {
        let (ta, tb) = (text(a), text(b));
        let ra = TraceReader::new(ta.as_bytes()).unwrap();
        let rb = TraceReader::new(tb.as_bytes()).unwrap();
        diff_traces(ra, rb, opts).unwrap()
    }

    fn diff(a: &[Frame], b: &[Frame]) -> Report {
        diff_with(a, b, &DiffOptions::default())
    }

    /// Ten frames of a player walking right next to a bat.
    fn walk() -> Vec<Frame> {
        (0..10)
            .map(|n| {
                frame(
                    n,
                    vec![
                        instance(100001, "oPlayer1", 16.0 + n as f64),
                        instance(100002, "oBat", 40.0),
                    ],
                )
            })
            .collect()
    }

    fn fields(r: &Report) -> Vec<(String, Datum, Datum)> {
        r.divergence
            .as_ref()
            .unwrap()
            .differences
            .iter()
            .filter_map(|d| match d {
                Difference::Field { path, a, b, .. } => Some((path.clone(), a.clone(), b.clone())),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn identical() {
        let r = diff(&walk(), &walk());
        assert_eq!(r.outcome, Outcome::Identical);
        assert_eq!((r.equal_frames, r.last_equal_frame), (10, Some(9)));
        assert!(r.to_text().starts_with("identical: 10 frames compared"));
    }

    #[test]
    fn nothing_compared_is_not_a_pass() {
        let r = diff(&[], &[]);
        assert_eq!((r.outcome, r.equal_frames), (Outcome::NothingCompared, 0));
        let past_the_end = DiffOptions {
            from: Some(50),
            ..DiffOptions::default()
        };
        let r = diff_with(&walk(), &walk(), &past_the_end);
        assert_eq!(r.outcome, Outcome::NothingCompared);
        assert!(r.to_text().starts_with("nothing compared"));
        let r = diff(&walk()[..3], &walk()[3..]);
        assert_eq!(r.outcome, Outcome::NothingCompared);
        assert_eq!((r.frames_only_in_a.count, r.frames_only_in_b.count), (3, 7));
    }

    #[test]
    fn what_a_writer_filtered_out_is_not_compared() {
        let filter = Filter {
            objects: vec!["oPlayer*".into()],
            fields: vec!["x".into(), "alarms[8]".into(), "vars.xVel".into()],
            ..Filter::default()
        };
        let mut w = TraceWriter::with_header(
            Vec::new(),
            crate::trace::TraceHeader::new(Some("port")),
            filter,
        )
        .unwrap();
        let mut full = walk();
        for f in &mut full {
            f.instances[0].alarms = Some(BTreeMap::from([(0, 17), (8, -1)]));
        }
        for f in &full {
            w.write_frame(f).unwrap();
        }
        let filtered = String::from_utf8(w.into_inner()).unwrap();
        assert!(filtered.contains("\"globals\":{}"));
        let full = text(&full);
        let r = diff_traces(
            TraceReader::new(filtered.as_bytes()).unwrap(),
            TraceReader::new(full.as_bytes()).unwrap(),
            &DiffOptions::default(),
        )
        .unwrap();
        assert_eq!(r.outcome, Outcome::Identical, "{}", r.to_text());
        assert!(r.warnings[0].starts_with("trace a was written with a filter"));
    }

    #[test]
    fn object_is_never_filtered_out() {
        let a = vec![frame(0, vec![instance(5, "oBat", 1.0)])];
        let b = vec![frame(0, vec![instance(5, "oSnake", 1.0)])];
        let opts = DiffOptions {
            matching: Matching::Id,
            filter: Filter {
                fields: vec!["x".into()],
                ..Filter::default()
            },
            ..DiffOptions::default()
        };
        assert_eq!(
            fields(&diff_with(&a, &b, &opts)),
            [(
                "object".into(),
                Datum::Text("oBat".into()),
                Datum::Text("oSnake".into())
            )]
        );
    }

    #[test]
    fn first_divergence_with_history() {
        let mut b = walk();
        for f in &mut b[5..] {
            f.instances[0].x = Some(Real(f.instances[0].x.unwrap().0 + 0.5));
        }
        b[7].seed = Some(1);
        let r = diff(&walk(), &b);
        assert_eq!(r.outcome, Outcome::Diverged);
        assert_eq!(r.equal_frames, 5);
        let d = r.divergence.as_ref().unwrap();
        assert_eq!((d.frame, d.total_differences), (5, 1));
        assert_eq!(
            fields(&r),
            [("x".into(), Datum::Real(Real(21.0)), Datum::Real(Real(21.5)))]
        );
        let Difference::Field { a_instance, .. } = &d.differences[0] else {
            panic!()
        };
        assert_eq!(a_instance.as_ref().unwrap().to_string(), "oPlayer1 #100001");
        // Three frames of context, then the diverging one.
        let h = &d.history[0];
        assert_eq!(
            h.points.iter().map(|p| p.frame).collect::<Vec<_>>(),
            [2, 3, 4, 5]
        );
        assert_eq!(h.points[0].a, Datum::Real(Real(18.0)));
        let text = r.to_text();
        assert!(
            text.contains("diverged at frame 5 in rLevel, after 5 equal frames: 1 difference\n"),
            "{text}"
        );
        assert!(
            text.contains("  oPlayer1 #100001 x: a=21 b=21.5\n"),
            "{text}"
        );
        assert!(
            text.contains("  oPlayer1 #100001 x: 2: 18 | 3: 19 | 4: 20 | 5: 21 / 21.5\n"),
            "{text}"
        );
        // The JSON form carries the same.
        let json: serde_json::Value = serde_json::from_str(&r.to_json()).unwrap();
        assert_eq!(json["outcome"], "diverged");
        assert_eq!(json["divergence"]["differences"][0]["kind"], "field");
        assert_eq!(json["divergence"]["differences"][0]["path"], "x");
        assert_eq!(json["divergence"]["differences"][0]["b"], 21.5);
        assert_eq!(
            json["divergence"]["differences"][0]["a_instance"]["id"],
            100001
        );
        assert_eq!(
            json["divergence"]["history"][0]["points"]
                .as_array()
                .unwrap()
                .len(),
            4
        );
    }

    #[test]
    fn exact_by_default_tolerance_on_request() {
        let mut b = walk();
        b[3].instances[1].y = Some(Real(b[3].instances[1].y.unwrap().0 + 1e-12));
        b[4].instances[1]
            .vars
            .as_mut()
            .unwrap()
            .insert("xVel".into(), Value::from(0.30000000000001));
        assert_eq!(diff(&walk(), &b).outcome, Outcome::Diverged);
        let opts = DiffOptions {
            tolerance: 1e-9,
            ..DiffOptions::default()
        };
        assert_eq!(diff_with(&walk(), &b, &opts).outcome, Outcome::Identical);
        // -0 and 0 differ exactly; NaN equals NaN.
        let mut a = walk();
        let mut b = walk();
        a[0].instances[0].y = Some(Real(0.0));
        b[0].instances[0].y = Some(Real(-0.0));
        a[1].instances[0].depth = Some(Real(f64::NAN));
        b[1].instances[0].depth = Some(Real(f64::NAN));
        assert_eq!(
            fields(&diff(&a, &b)),
            [("y".into(), Datum::Real(Real(0.0)), Datum::Real(Real(-0.0)))]
        );
        b[0].instances[0].y = Some(Real(0.0));
        assert_eq!(diff(&a, &b).outcome, Outcome::Identical);
    }

    #[test]
    fn created_and_destroyed_instances() {
        let mut b = walk();
        b[2].instances.remove(1);
        b[2].instances.push(instance(100003, "oBlood", 1.0));
        let r = diff(&walk(), &b);
        let d = r.divergence.unwrap();
        assert_eq!(d.frame, 2);
        assert_eq!(
            d.differences,
            [
                Difference::OnlyIn {
                    side: Side::A,
                    instance: InstanceRef {
                        id: 100002,
                        object: "oBat".into()
                    },
                    active: true
                },
                Difference::OnlyIn {
                    side: Side::B,
                    instance: InstanceRef {
                        id: 100003,
                        object: "oBlood".into()
                    },
                    active: true
                },
            ]
        );
    }

    /// The port numbering instances differently: b's ids are a's plus 1000.
    fn renumbered() -> Vec<Frame> {
        let mut b = walk();
        for f in &mut b {
            for i in &mut f.instances {
                i.id += 1000;
            }
        }
        b
    }

    #[test]
    fn matching_strategies() {
        let opts = |matching| DiffOptions {
            matching,
            ..DiffOptions::default()
        };
        // Auto falls back to object and creation order; so does object matching.
        assert_eq!(
            diff_with(&walk(), &renumbered(), &opts(Matching::Auto)).outcome,
            Outcome::Identical
        );
        assert_eq!(
            diff_with(&walk(), &renumbered(), &opts(Matching::Object)).outcome,
            Outcome::Identical
        );
        // Id matching sees four unrelated instances.
        let r = diff_with(&walk(), &renumbered(), &opts(Matching::Id));
        assert_eq!(r.divergence.unwrap().total_differences, 4);

        // Two bats pair up by creation order, whatever their ids.
        let bat = |id, x| Instance {
            object_index: Some(9),
            ..instance(id, "oBat", x)
        };
        let two_bats = |ids: [i32; 2], xs: [f64; 2]| {
            vec![frame(0, vec![bat(ids[0], xs[0]), bat(ids[1], xs[1])])]
        };
        let r = diff_with(
            &two_bats([1, 2], [5.0, 6.0]),
            &two_bats([11, 12], [5.0, 7.0]),
            &opts(Matching::Object),
        );
        let d = r.divergence.unwrap();
        let Difference::Field {
            a_instance,
            b_instance,
            path,
            ..
        } = &d.differences[0]
        else {
            panic!()
        };
        assert_eq!(
            (
                a_instance.as_ref().unwrap().id,
                b_instance.as_ref().unwrap().id,
                path.as_str()
            ),
            (2, 12, "x")
        );

        // Same id, different object: auto reports two lone instances, id matching an object difference.
        let a = vec![frame(0, vec![instance(5, "oBat", 1.0)])];
        let b = vec![frame(0, vec![instance(5, "oSnake", 1.0)])];
        assert_eq!(
            diff_with(&a, &b, &opts(Matching::Auto))
                .divergence
                .unwrap()
                .total_differences,
            2
        );
        let r = diff_with(&a, &b, &opts(Matching::Id));
        assert_eq!(
            fields(&r)[0],
            (
                "object".into(),
                Datum::Text("oBat".into()),
                Datum::Text("oSnake".into())
            )
        );
        assert!(
            r.to_text()
                .contains("oBat #5 / oSnake #5 object: a=\"oBat\" b=\"oSnake\""),
            "{}",
            r.to_text()
        );
    }

    #[test]
    fn fields_only_one_trace_dumped_are_not_compared() {
        // Like the oracle without OPENGMK_DUMP_FIELDS: no vars, no globals.
        let mut b = walk();
        for f in &mut b {
            f.globals = None;
            for i in &mut f.instances {
                i.vars = None;
                i.alarms = None;
                i.xprevious = None;
            }
        }
        let r = diff(&walk(), &b);
        assert_eq!(r.outcome, Outcome::Identical);
        let only_a: Vec<&str> = r
            .not_compared_only_in_a
            .iter()
            .map(String::as_str)
            .collect();
        assert_eq!(only_a, ["alarms", "globals", "vars", "xprevious"]);
        assert!(
            r.to_text()
                .contains("not compared, only in trace a: alarms, globals, vars, xprevious")
        );
    }

    #[test]
    fn keys_missing_from_a_dumped_map_are_differences() {
        let mut b = walk();
        b[1].instances[0].vars.as_mut().unwrap().remove("name");
        b[1].globals
            .as_mut()
            .unwrap()
            .insert("lives".into(), Value::from(4.0));
        let r = diff(&walk(), &b);
        assert_eq!(
            fields(&r),
            [
                (
                    "globals.lives".into(),
                    Datum::Absent,
                    Datum::Scalar(Scalar::Real(Real(4.0)))
                ),
                (
                    "vars.name".into(),
                    Datum::Scalar(Scalar::Str("bob".into())),
                    Datum::Absent
                ),
            ]
        );
        let json: serde_json::Value = serde_json::from_str(&r.to_json()).unwrap();
        assert!(json["divergence"]["differences"][0]["a"].is_null());
    }

    #[test]
    fn alarms_and_arrays_compare_by_meaning() {
        // A missing alarm is -1, so the oracle's sparse map equals a full one.
        let mut a = walk();
        let mut b = walk();
        a[0].instances[0].alarms = Some(BTreeMap::from([(0, 5)]));
        b[0].instances[0].alarms =
            Some((0..12).map(|i| (i, if i == 0 { 5 } else { -1 })).collect());
        // GML reads `a` as `a[0]`, so a scalar equals a one-element array.
        a[0].globals
            .as_mut()
            .unwrap()
            .insert("arr".into(), Value::from(3.0));
        b[0].globals.as_mut().unwrap().insert(
            "arr".into(),
            Value::Array(BTreeMap::from([(0, Scalar::Real(Real(3.0)))])),
        );
        assert_eq!(diff(&a, &b).outcome, Outcome::Identical);

        b[0].instances[0].alarms.as_mut().unwrap().insert(11, 2);
        b[0].globals.as_mut().unwrap().insert(
            "arr".into(),
            Value::Array(BTreeMap::from([
                (0, Scalar::Real(Real(3.0))),
                (32001, Scalar::Str("x".into())),
            ])),
        );
        let r = diff(&a, &b);
        assert_eq!(
            fields(&r),
            [
                (
                    "globals.arr[1,1]".into(),
                    Datum::Absent,
                    Datum::Scalar(Scalar::Str("x".into()))
                ),
                ("alarms[11]".into(), Datum::Int(-1), Datum::Int(2)),
            ]
        );
    }

    #[test]
    fn filters() {
        let mut b = walk();
        b[3].instances[1].x = Some(Real(0.0)); // the bat
        b[4].instances[0]
            .vars
            .as_mut()
            .unwrap()
            .insert("randNum".into(), Value::from(7.0));
        b[6].instances[0].image_index = Some(Real(9.0));
        let run = |filter: Filter| {
            diff_with(
                &walk(),
                &b,
                &DiffOptions {
                    filter,
                    ..DiffOptions::default()
                },
            )
        };
        assert_eq!(run(Filter::default()).divergence.unwrap().frame, 3);
        let no_bats = Filter {
            ignore_objects: vec!["oBat".into()],
            ..Filter::default()
        };
        assert_eq!(run(no_bats.clone()).divergence.unwrap().frame, 4);
        let quiet = Filter {
            ignore_fields: vec!["vars.randNum".into()],
            ..no_bats
        };
        assert_eq!(run(quiet.clone()).divergence.unwrap().frame, 6);
        let positions = Filter {
            fields: vec!["x".into(), "y".into()],
            ..quiet
        };
        assert_eq!(run(positions).outcome, Outcome::Identical);
        let players = Filter {
            objects: vec!["oPlay*".into()],
            fields: vec!["vars".into()],
            ..Filter::default()
        };
        assert_eq!(run(players).divergence.unwrap().frame, 4);
    }

    #[test]
    fn deactivated_instances() {
        let mut a = walk();
        let mut b = walk();
        for f in a.iter_mut().chain(b.iter_mut()) {
            f.deactivated = Some(vec![instance(5, "oBrick", 0.0)]);
        }
        // The bat is culled in b at frame 4.
        let bat = b[4].instances.remove(1);
        b[4].deactivated.as_mut().unwrap().push(bat);
        let r = diff(&a, &b);
        assert_eq!(
            fields(&r),
            [("active".into(), Datum::Bool(true), Datum::Bool(false))]
        );
        assert_eq!(r.divergence.unwrap().frame, 4);

        // Only one trace dumps deactivated instances: they're left out.
        let r = diff(&walk(), &a);
        assert_eq!(r.outcome, Outcome::Identical);
        assert!(r.not_compared_only_in_b.contains("deactivated"));
        // ... so a culled instance looks destroyed.
        let r = diff(&walk(), &b);
        let d = r.divergence.unwrap();
        assert_eq!(d.frame, 4);
        assert!(matches!(
            &d.differences[0],
            Difference::OnlyIn {
                side: Side::A,
                active: true,
                ..
            }
        ));
    }

    #[test]
    fn frames_pair_by_number() {
        let only = |count, first, last| Unmatched {
            count,
            first: Some(first),
            last: Some(last),
        };
        // A longer trace.
        let r = diff(&walk(), &walk()[..7]);
        assert_eq!((r.outcome, r.equal_frames), (Outcome::UnmatchedFrames, 7));
        assert_eq!(
            (r.frames_only_in_a.clone(), r.frames_only_in_b.count),
            (only(3, 7, 9), 0)
        );
        let text = r.to_text();
        assert!(
            text.starts_with("unmatched frames: the 7 frames in both traces are equal, but some"),
            "{text}"
        );
        assert!(
            text.contains("3 frames only in trace a (first 7, last 9)\n"),
            "{text}"
        );
        // A trace that starts later (a savestate resume) and one that skips a frame.
        let mut b = walk()[4..].to_vec();
        b.remove(2);
        let r = diff(&walk(), &b);
        assert_eq!(
            (r.outcome, r.equal_frames, r.frames_only_in_a.count),
            (Outcome::UnmatchedFrames, 5, 5)
        );
        assert_eq!(
            diff(&walk()[..1], &walk()[1..2]).frames_only_in_b,
            only(1, 1, 1)
        );
        // ... which --from and --to can exclude.
        let opts = DiffOptions {
            from: Some(4),
            to: Some(5),
            ..DiffOptions::default()
        };
        let r = diff_with(&walk(), &b, &opts);
        assert_eq!(
            (r.outcome, r.equal_frames, r.last_equal_frame),
            (Outcome::Identical, 2, Some(5))
        );
        // Unmatched frames before a divergence are still counted.
        b[3].seed = Some(0);
        let r = diff(&walk(), &b);
        assert_eq!(
            (
                r.outcome,
                r.divergence.unwrap().frame,
                r.frames_only_in_a.count
            ),
            (Outcome::Diverged, 8, 5)
        );
    }

    #[test]
    fn many_differences_are_truncated() {
        let mut b = walk();
        for (n, i) in b[0].instances.iter_mut().enumerate() {
            for name in Instance::REAL_FIELDS {
                *i.real_mut(name).unwrap() = Some(Real(1000.0 + n as f64));
            }
        }
        let opts = DiffOptions {
            max_differences: 4,
            context: 0,
            ..DiffOptions::default()
        };
        let r = diff_with(&walk(), &b, &opts);
        let d = r.divergence.as_ref().unwrap();
        assert_eq!(
            (d.total_differences, d.differences.len(), d.history.len()),
            (30, 4, 4)
        );
        assert!(d.history.iter().all(|h| h.points.len() == 1));
        assert!(r.to_text().contains("  ... 26 more\n"));
        let all = DiffOptions {
            max_differences: 0,
            ..DiffOptions::default()
        };
        assert_eq!(
            diff_with(&walk(), &b, &all)
                .divergence
                .unwrap()
                .differences
                .len(),
            30
        );
    }

    #[test]
    fn history_tracks_instances_that_appear() {
        let mut a = walk();
        let mut b = walk();
        for f in a[3..].iter_mut().chain(b[3..].iter_mut()) {
            f.instances.push(instance(100009, "oRock", 1.0));
        }
        b[4].instances[2].x = Some(Real(2.0));
        let r = diff(&a, &b);
        let h = &r.divergence.unwrap().history[0];
        let points: Vec<(u64, Datum)> = h.points.iter().map(|p| (p.frame, p.a.clone())).collect();
        assert_eq!(
            points[..3],
            [
                (1, Datum::Absent),
                (2, Datum::Absent),
                (3, Datum::Real(Real(1.0)))
            ]
        );
    }

    #[test]
    fn malformed_traces_report_their_side() {
        let good = text(&walk());
        let bad = format!("{good}{{\"frame\":10,\"seed\":\"x\"}}\n");
        let longer = format!("{good}{{\"frame\":10}}\n");
        let opts = DiffOptions::default();
        let err = diff_traces(
            TraceReader::new(bad.as_bytes()).unwrap(),
            TraceReader::new(longer.as_bytes()).unwrap(),
            &opts,
        )
        .unwrap_err();
        assert_eq!(err.side, Side::A);
        assert!(
            err.to_string()
                .starts_with("trace a: line 11: invalid frame"),
            "{err}"
        );
        let dup = format!("{}{{\"frame\":0}}\n", text(&walk()[..1]));
        let err = diff_traces(
            TraceReader::new(good.as_bytes()).unwrap(),
            TraceReader::new(dup.as_bytes()).unwrap(),
            &opts,
        )
        .unwrap_err();
        assert_eq!(err.side, Side::B);
    }

    #[test]
    fn oracle_fixture_against_a_perturbed_copy() {
        let text = include_str!("../tests/fixtures/oracle_frames.jsonl");
        let read = || -> Vec<Frame> {
            TraceReader::new(text.as_bytes())
                .unwrap()
                .map(Result::unwrap)
                .collect()
        };
        assert_eq!(diff(&read(), &read()).outcome, Outcome::Identical);
        let mut frames = read();
        let player = frames[2]
            .instances
            .iter_mut()
            .find(|i| i.object == "oPlayer1")
            .unwrap();
        player
            .vars
            .as_mut()
            .unwrap()
            .insert("climbAcc".into(), Value::from(0.6));
        let r = diff(&read(), &frames);
        let d = r.divergence.as_ref().unwrap();
        assert_eq!(
            (d.frame, d.room_name_a.as_deref()),
            (1500, Some("rTutorial"))
        );
        assert_eq!(
            fields(&r),
            [(
                "vars.climbAcc".into(),
                Datum::Scalar(Scalar::Real(Real(0.6000000000000001))),
                Datum::Scalar(Scalar::Real(Real(0.6)))
            )]
        );
        assert!(r.not_compared_only_in_a.is_empty() && r.not_compared_only_in_b.is_empty());
    }
}
