//! The world as a `trace::Frame`, in the oracle's schema (docs/formats.md, "Traces"), and in that frame's
//! canonical encoding ([`FrameEncoder`]), which replay fingerprints hash.

use crate::instance::{ALARMS, Instance, State};
use crate::list::Inst;
use crate::program::{ObjectId, Program};
use crate::string::GmString;
use crate::value::Value;
use crate::var::{GVar, Slot, Var};
use crate::world::World;
use std::collections::BTreeMap;
use trace::canon::Encoder;

/// The built-ins the oracle dumps, in name order (the frame's `BTreeMap` order, which the canonical
/// encoding follows): view 0's rectangle (it decides `oLevel`'s deactivation region).
const BUILTINS: [&str; 4] = [
    "view_hview[0]",
    "view_wview[0]",
    "view_xview[0]",
    "view_yview[0]",
];

/// GM strings are bytes; traces hold text. Each byte maps to the code point of the same value (Latin-1,
/// which agrees with Windows-1252 on everything but 0x80-0x9F; Spelunky's strings are ASCII).
fn text(s: &GmString) -> String {
    s.to_string()
}

fn scalar(v: &Value) -> trace::Scalar {
    match v {
        Value::Real(x) => trace::Scalar::Real(trace::Real(*x)),
        Value::Str(s) => trace::Scalar::Str(text(s)),
    }
}

fn slot_value(s: &Slot) -> Option<trace::Value> {
    Some(match s {
        Slot::Unset => return None,
        Slot::Scalar(v) => scalar(v).into(),
        Slot::Array(m) => trace::Value::Array(m.iter().map(|(k, v)| (*k, scalar(v))).collect()),
    })
}

impl World {
    /// The state after the last frame, every field, hashed: what a replay fingerprint hashes for each frame
    /// (`harness::fingerprint`, [`FrameEncoder`] and [`trace::canon::hash`]). The wiki's clips record it at
    /// their first frame, and the web build's embed reports its own there, so a page can tell a build that
    /// plays the clip unlike its video (docs/wiki-media.md, "The state at the first frame"). The same on every
    /// platform, as the world is.
    pub fn frame_hash(&self) -> u64 {
        let mut buf = Vec::new();
        FrameEncoder::new().encode(self, &mut buf);
        trace::canon::hash(&buf)
    }

    /// The state after the last frame, numbered as the oracle numbers it (the first frame is 0). Instances
    /// and deactivated instances are sorted by id; alarms list those that are set; `vars` and `globals`
    /// hold every assigned variable.
    /// `builtins` holds view 0's rectangle, as the oracle dumps it.
    pub fn trace_frame(&self) -> trace::Frame {
        self.trace_frame_with(true)
    }

    /// [`trace_frame`](Self::trace_frame), leaving out `vars` and `globals` unless `fields` (what the
    /// oracle does without `--fields`).
    pub fn trace_frame_with(&self, fields: bool) -> trace::Frame {
        let dump = |state: State| {
            let mut v: Vec<trace::Instance> = self
                .with_state(state)
                .map(|h| self.trace_instance(self.instance(h), fields))
                .collect();
            v.sort_by_key(|i| i.id);
            v
        };
        let instances = dump(State::Active);
        let globals = fields.then(|| self.trace_globals());
        trace::Frame {
            frame: self.frames.saturating_sub(1),
            room: Some(self.room.id),
            room_name: self.assets.room_name(self.room.id).map(str::to_string),
            seed: Some(self.rng.seed()),
            instance_count: Some(instances.len() as u64),
            instances,
            deactivated: Some(dump(State::Inactive)),
            globals,
            builtins: Some(self.trace_builtins()),
        }
    }

    fn trace_globals(&self) -> BTreeMap<String, trace::Value> {
        (0..self.program.global_count())
            .filter_map(|i| {
                let v = slot_value(&self.globals[i])?;
                Some((
                    self.program.global_name(crate::GVar(i as u16)).to_string(),
                    v,
                ))
            })
            .collect()
    }

    fn trace_builtins(&self) -> BTreeMap<String, trace::Value> {
        (BUILTINS.into_iter().zip(self.builtins()))
            .map(|(k, n)| (k.to_string(), scalar(&Value::from(n)).into()))
            .collect()
    }

    /// [`BUILTINS`]' values.
    fn builtins(&self) -> [i32; BUILTINS.len()] {
        let v = &self.room.views[0];
        [v.hview, v.wview, v.xview, v.yview]
    }

    fn trace_instance(&self, i: &Instance, fields: bool) -> trace::Instance {
        let p = self.program;
        let r = |x: f64| Some(trace::Real(x));
        let sprite = self.assets.sprite(i.sprite_index);
        let alarms = (0..i.alarms.len() as u32)
            .zip(i.alarms)
            .filter(|&(_, a)| a != -1)
            .collect();
        let vars = fields.then(|| {
            self.user_vars(i)
                .filter_map(|(v, s)| Some((p.var_name(v).to_string(), slot_value(s)?)))
                .collect()
        });
        trace::Instance {
            id: i.id,
            object_index: Some(i.object.0 as i32),
            object: p.object(i.object).map_or(String::new(), |o| o.name.clone()),
            x: r(i.x),
            y: r(i.y),
            xprevious: r(i.xprevious),
            yprevious: r(i.yprevious),
            hspeed: r(i.hspeed),
            vspeed: r(i.vspeed),
            speed: r(i.speed),
            direction: r(i.direction),
            gravity: r(i.gravity),
            friction: r(i.friction),
            image_index: r(i.image_index),
            image_speed: r(i.image_speed),
            image_xscale: r(i.image_xscale),
            image_yscale: r(i.image_yscale),
            depth: r(i.depth),
            sprite_index: Some(i.sprite_index),
            sprite: Some(sprite.map_or(String::new(), |s| s.name.clone())),
            visible: Some(i.visible),
            solid: Some(i.solid),
            alarms: Some(alarms),
            vars,
        }
    }
}

/// Which frame of the world a [`FrameEncoder`] encoded, numbered as [`World::trace_frame`] numbers it: what
/// fingerprint checkpoints are named by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameKey<'w> {
    pub frame: u64,
    pub room: i32,
    pub room_name: Option<&'w str>,
}

/// Writes the world's frame with every field, as [`World::trace_frame`] has it, in the canonical encoding
/// ([`trace::canon`]), straight from the world: the same bytes as `trace::canon::encode_frame` of
/// `trace_frame()`, without building the frame. Building it (every variable name a `String`, every
/// instance's variables a `BTreeMap`) and dropping it took three times as long as stepping the game, and
/// fingerprinting the committed replays does it every frame. It has to change with `trace_frame`: the
/// harness's tests compare the two on real runs, frame by frame.
///
/// Kept from frame to frame for what it works out once per program: each object's variables and the
/// globals in name order, the order the frame's maps hold them in.
#[derive(Clone, Debug, Default)]
pub struct FrameEncoder {
    /// The program the orders below are for.
    program: Option<&'static Program>,
    /// Each object's layout by object index, filled in as objects come up.
    layouts: Vec<Option<SortedLayout>>,
    /// The globals' indices in name order.
    globals: Box<[u16]>,
    /// The instances being encoded, reused: `(id, place in draw order, handle)`.
    insts: Vec<(i32, u32, Inst)>,
    /// An instance's variables outside its layout, by index, reused.
    extra: Vec<usize>,
}

/// An object's layout as `(variable, slot)`, in name order.
type SortedLayout = Box<[(Var, u16)]>;

// The canonical encoding keeps alarms 0 to `trace::trace::ALARMS - 1`; the port has no others.
const _: () = assert!(ALARMS as u32 <= trace::trace::ALARMS);

// `FrameEncoder` writes each of these fields, in `trace::canon::encode_frame`'s order. This stops compiling
// when `trace::Frame` or `trace::Instance` gains a field, so the new one can't be left out here.
const _: fn(&trace::Frame, &trace::Instance) = |f, i| {
    let trace::Frame {
        frame: _,
        room: _,
        room_name: _,
        seed: _,
        instance_count: _,
        instances: _,
        deactivated: _,
        globals: _,
        builtins: _,
    } = f;
    let trace::Instance {
        id: _,
        object_index: _,
        object: _,
        x: _,
        y: _,
        xprevious: _,
        yprevious: _,
        hspeed: _,
        vspeed: _,
        speed: _,
        direction: _,
        gravity: _,
        friction: _,
        image_index: _,
        image_speed: _,
        image_xscale: _,
        image_yscale: _,
        depth: _,
        sprite_index: _,
        sprite: _,
        visible: _,
        solid: _,
        alarms: _,
        vars: _,
    } = i;
};

impl FrameEncoder {
    pub fn new() -> FrameEncoder {
        FrameEncoder::default()
    }

    /// Appends the world's frame to `out`.
    pub fn encode<'w>(&mut self, w: &'w World, out: &mut Vec<u8>) -> FrameKey<'w> {
        let p = w.program;
        if !self.program.is_some_and(|q| std::ptr::eq(q, p)) {
            let mut globals: Vec<u16> = (0..p.global_count() as u16).collect();
            globals.sort_by(|&a, &b| p.global_name(GVar(a)).cmp(p.global_name(GVar(b))));
            *self = FrameEncoder {
                program: Some(p),
                globals: globals.into(),
                ..FrameEncoder::default()
            };
        }
        let key = FrameKey {
            frame: w.frames.saturating_sub(1),
            room: w.room.id,
            room_name: w.assets.room_name(w.room.id),
        };
        // In `encode_frame`'s order. Every optional field but the room name is always there in the port's
        // frames.
        let e = &mut Encoder(out);
        e.u64(key.frame);
        e.some(true);
        e.i32(key.room);
        e.opt(key.room_name, Encoder::str);
        e.some(true);
        e.i32(w.rng.seed());
        let mut insts = std::mem::take(&mut self.insts);
        by_id(w, State::Active, &mut insts);
        // `instance_count`, then the instances.
        e.some(true);
        e.u64(insts.len() as u64);
        e.len(insts.len());
        for &(_, _, h) in &insts {
            self.instance(e, w, w.instance(h));
        }
        by_id(w, State::Inactive, &mut insts);
        e.some(true);
        e.len(insts.len());
        for &(_, _, h) in &insts {
            self.instance(e, w, w.instance(h));
        }
        self.insts = insts;
        e.some(true);
        let at = e.len_later();
        let mut n = 0;
        for &g in &self.globals {
            let name = p.global_name(GVar(g));
            n += usize::from(var(e, name, &w.globals[usize::from(g)]));
        }
        e.set_len(at, n);
        e.some(true);
        e.len(BUILTINS.len());
        for (name, n) in BUILTINS.into_iter().zip(w.builtins()) {
            var(e, name, &Slot::Scalar(Value::from(n)));
        }
        key
    }

    fn instance(&mut self, e: &mut Encoder, w: &World, i: &Instance) {
        let p = w.program;
        e.i32(i.id);
        e.some(true);
        e.i32(i.object.0 as i32);
        e.str(p.object(i.object).map_or("", |o| &o.name));
        for r in [
            i.x,
            i.y,
            i.xprevious,
            i.yprevious,
            i.hspeed,
            i.vspeed,
            i.speed,
            i.direction,
            i.gravity,
            i.friction,
            i.image_index,
            i.image_speed,
            i.image_xscale,
            i.image_yscale,
            i.depth,
        ] {
            e.some(true);
            e.real(r);
        }
        e.some(true);
        e.i32(i.sprite_index);
        e.some(true);
        e.str(w.assets.sprite(i.sprite_index).map_or("", |s| &s.name));
        e.some(true);
        e.bool(i.visible);
        e.some(true);
        e.bool(i.solid);
        e.some(true);
        e.len(i.alarms.iter().filter(|&&a| a != -1).count());
        for (k, &a) in (0u64..).zip(&i.alarms) {
            if a != -1 {
                e.u64(k);
                e.i32(a);
            }
        }
        // The layout's variables and those outside it (written through instance ids the generator couldn't
        // see: rare, and few), merged in name order. A name is in one or the other.
        let mut extra = std::mem::take(&mut self.extra);
        extra.clear();
        extra.extend(0..i.extra.len());
        extra.sort_unstable_by_key(|&k| p.var_name(i.extra[k].0));
        let mut extra_vars = (extra.iter())
            .map(|&k| (p.var_name(i.extra[k].0), &i.extra[k].1))
            .peekable();
        e.some(true);
        let at = e.len_later();
        let mut n = 0;
        for &(v, slot) in self.layout(p, i.object) {
            let name = p.var_name(v);
            while let Some((x, s)) = extra_vars.next_if(|&(x, _)| x < name) {
                n += usize::from(var(e, x, s));
            }
            n += usize::from(var(e, name, &i.vars[usize::from(slot)]));
        }
        for (x, s) in extra_vars {
            n += usize::from(var(e, x, s));
        }
        e.set_len(at, n);
        self.extra = extra;
    }

    /// `o`'s layout in name order.
    fn layout(&mut self, p: &Program, o: ObjectId) -> &[(Var, u16)] {
        let i = o.idx();
        if self.layouts.len() <= i {
            self.layouts.resize(i + 1, None);
        }
        self.layouts[i].get_or_insert_with(|| {
            let mut vars: Vec<(Var, u16)> =
                (p.layout(o).vars()).map(|(v, s)| (v, s as u16)).collect();
            vars.sort_by(|a, b| p.var_name(a.0).cmp(p.var_name(b.0)));
            vars.into()
        })
    }
}

/// The instances with this state into `out`, by id, as `trace_frame` sorts them (it sorts stably, so
/// equal ids, which the port never has, stay in draw order).
fn by_id(w: &World, state: State, out: &mut Vec<(i32, u32, Inst)>) {
    out.clear();
    out.extend(
        (0u32..)
            .zip(w.with_state(state))
            .map(|(n, h)| (w.instance(h).id, n, h)),
    );
    out.sort_unstable();
}

/// A variable as [`Encoder::vars`] writes it, if it counts (an unset one or an empty array doesn't), and
/// whether it did.
fn var(e: &mut Encoder, name: &str, s: &Slot) -> bool {
    match s {
        Slot::Unset => false,
        Slot::Array(m) if m.is_empty() => false,
        Slot::Scalar(v) => {
            e.str(name);
            e.len(1);
            e.u64(0);
            scalar_to(e, v);
            true
        }
        Slot::Array(m) => {
            e.str(name);
            e.len(m.len());
            for (&k, v) in m.iter() {
                e.u64(u64::from(k));
                scalar_to(e, v);
            }
            true
        }
    }
}

/// [`scalar`], encoded.
fn scalar_to(e: &mut Encoder, v: &Value) {
    match v {
        Value::Real(x) => {
            e.real_tag();
            e.real(*x);
        }
        Value::Str(s) => {
            e.str_tag();
            text_to(e, s);
        }
    }
}

/// [`text`], encoded, without making the `String`: each byte is a `char` as `GmString`'s `Display` has it.
fn text_to(e: &mut Encoder, s: &GmString) {
    let b = s.as_bytes();
    if b.is_ascii() {
        e.bytes(b);
        return;
    }
    e.len(b.iter().map(|&c| char::from(c).len_utf8()).sum());
    for &c in b {
        let mut utf8 = [0; 4];
        e.0.extend_from_slice(char::from(c).encode_utf8(&mut utf8).as_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Assets, ObjectInfo, ProgramBuilder};
    use std::sync::Arc;

    /// What `FrameEncoder` writes for `w`, and what encoding its `trace_frame()` does.
    fn both(w: &World) -> (Vec<u8>, Vec<u8>) {
        let (mut fast, mut slow) = (Vec::new(), Vec::new());
        let key = FrameEncoder::new().encode(w, &mut fast);
        let f = w.trace_frame();
        trace::canon::encode_frame(&mut slow, &f);
        assert_eq!(
            (key.frame, Some(key.room), key.room_name),
            (f.frame, f.room, f.room_name.as_deref())
        );
        (fast, slow)
    }

    #[test]
    fn the_encoder_writes_what_encoding_the_frame_does() {
        // The cases the real game's runs (harness's tests) may never reach: names out of order in the
        // layout and the globals, a variable outside the layout, arrays (an empty one too), strings that
        // aren't ASCII, NaN and -0, set alarms, a deactivated instance.
        let mut b = ProgramBuilder::new();
        let (zeta, alpha) = (b.global("zeta"), b.global("alpha"));
        b.global("unset");
        let o = b.push_object(ObjectInfo::new("oThing"));
        let (vb, va, vc) = (b.var("b"), b.var("a"), b.var("c"));
        // Outside the layout: names before, among and after its own.
        let extras = [b.var("zz_extra"), b.var("aa_extra"), b.var("A_extra")];
        b.layout(o, &[vb, va, vc]);
        let program = Box::leak(Box::new(b.build()));
        let room = assets::Room {
            name: "rTest".into(),
            width: 320,
            height: 240,
            speed: 30,
            ..Default::default()
        };
        let assets = Assets::new(Vec::new(), vec![Some(room)]);
        let mut w = World::new(program, Arc::new(assets), 1, 100_000, 0);
        w.start_game().unwrap();
        let (fast, slow) = both(&w);
        assert_eq!(fast, slow);

        let ids = [
            w.instance_create(1.0, 2.0, 0).unwrap(),
            w.instance_create(f64::NAN, -0.0, 0).unwrap(),
            w.instance_create(5.0, 5.0, 0).unwrap(),
        ];
        let [h0, h1, h2] = ids.map(|id| w.find(id).unwrap());
        w.set(h0, vb, Value::from(3.0)).unwrap();
        w.set(h0, vc, Value::from(GmString::from(&b"caf\xe9"[..])))
            .unwrap();
        w.set_a(h0, va, 1, 2, Value::from(-0.0)).unwrap();
        w.set_a(h0, va, 0, 0, Value::from("x")).unwrap();
        for (v, x) in extras.into_iter().zip([f64::NAN, 2.0, -1.5]) {
            w.set(h1, v, Value::from(x)).unwrap();
        }
        w.set(h1, va, Value::from(1.0)).unwrap();
        w.instance_mut(h1).vars[0] = Slot::Array(Box::default());
        w.instance_mut(h1).alarms[3] = 7;
        w.instance_mut(h1).alarms[11] = 0;
        w.instance_mut(h2).state = State::Inactive;
        w.gset(zeta, Value::from(1.0)).unwrap();
        w.gset_a(alpha, 0, 3, Value::from("\u{7f}")).unwrap();
        assert_eq!(
            w.instance(h1).extra.len(),
            3,
            "the extras are outside the layout"
        );
        let (fast, slow) = both(&w);
        assert_eq!(fast, slow);
        // Committed in the wiki's clips (`World::frame_hash`), and compared with the web build's: the same
        // natively and on wasm32 (scripts/wasm-test), NaN, -0 and all.
        assert_eq!(format!("{:016x}", w.frame_hash()), "f12bd6164a48c894");
    }

    #[test]
    fn the_hash_is_fixed() {
        // Fingerprints and the wiki's clips are committed: changing the hash means recording them all again.
        assert_eq!(trace::canon::hash(b""), 0xf4a9_e556_70e1_91bb);
        assert_eq!(trace::canon::hash(b"spelunky"), 0xb42d_3d89_3cbe_d43f);
    }

    #[test]
    fn builtins_are_in_name_order() {
        assert!(BUILTINS.is_sorted());
    }

    #[test]
    fn strings_encode_as_text_does() {
        for bytes in [&b""[..], b"spelunky", b"caf\xe9 \x80\xff\x7f"] {
            let s = GmString::from(bytes);
            let (mut fast, mut slow) = (Vec::new(), Vec::new());
            text_to(&mut Encoder(&mut fast), &s);
            Encoder(&mut slow).str(&text(&s));
            assert_eq!(fast, slow, "{bytes:?}");
        }
    }
}
