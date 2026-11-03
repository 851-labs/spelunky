//! The world as a `trace::Frame`, in the oracle's schema (docs/formats.md, "Traces"), and in that frame's
//! canonical encoding ([`FrameEncoder`]), which replay fingerprints hash.

use crate::instance::{ALARMS, Instance, State};
use crate::list::Inst;
use crate::program::{ObjectId, Program};
use crate::room::{BACKGROUNDS, VIEWS};
use crate::string::GmString;
use crate::value::Value;
use crate::var::{GVar, Slot, Var};
use crate::world::World;
use std::collections::BTreeMap;
use trace::canon::Encoder;

/// What the trace's `builtins` holds, by GML name with the index written out, as the oracle dumps it (its
/// patch 0006, and 0040 for all but view 0's rectangle, 851-3870): every view (`oLevel` deactivates by view
/// 0's rectangle; the rest only drawing reads), every background layer and the room's colour, and the drawing
/// state the frame leaves (GML has no variables for those, so they're named for what sets them: `draw_color`,
/// `draw_alpha`, `draw_font` (-1 for GM's built-in font), the blend mode's `bm_*` factors
/// `draw_blend_src`/`draw_blend_dest`, and `draw_target`, the surface drawn to or -1 for the screen). Every
/// value is a real, as GML reads it (`true` is 1). In the order [`World::builtin_values`] gives them, not
/// name order: [`BuiltinOrder`] sorts them.
fn builtin_names() -> Vec<String> {
    let mut names: Vec<String> = BUILTIN_FRAME.iter().map(|n| n.to_string()).collect();
    for i in 0..VIEWS {
        names.extend(BUILTIN_VIEW.iter().map(|n| format!("{n}[{i}]")));
    }
    for i in 0..BACKGROUNDS {
        names.extend(BUILTIN_BACKGROUND.iter().map(|n| format!("{n}[{i}]")));
    }
    names
}

/// The built-ins of the frame, in [`World::builtin_values`]' order.
const BUILTIN_FRAME: [&str; 9] = [
    "background_color",
    "background_showcolor",
    "view_enabled",
    "draw_color",
    "draw_alpha",
    "draw_font",
    "draw_blend_src",
    "draw_blend_dest",
    "draw_target",
];

/// Each view's, `view_*[i]`.
const BUILTIN_VIEW: [&str; 15] = [
    "view_visible",
    "view_xview",
    "view_yview",
    "view_wview",
    "view_hview",
    "view_xport",
    "view_yport",
    "view_wport",
    "view_hport",
    "view_angle",
    "view_hborder",
    "view_vborder",
    "view_hspeed",
    "view_vspeed",
    "view_object",
];

/// Each background layer's, `background_*[i]`.
const BUILTIN_BACKGROUND: [&str; 13] = [
    "background_visible",
    "background_foreground",
    "background_index",
    "background_x",
    "background_y",
    "background_htiled",
    "background_vtiled",
    "background_hspeed",
    "background_vspeed",
    "background_xscale",
    "background_yscale",
    "background_blend",
    "background_alpha",
];

/// How many built-ins the trace holds.
const BUILTIN_COUNT: usize =
    BUILTIN_FRAME.len() + VIEWS * BUILTIN_VIEW.len() + BACKGROUNDS * BUILTIN_BACKGROUND.len();

/// The built-ins' names in name order (the frame's `BTreeMap` order, which the canonical encoding follows),
/// each with its place in [`World::builtin_values`].
#[derive(Clone, Debug)]
struct BuiltinOrder(Box<[(String, usize)]>);

impl Default for BuiltinOrder {
    fn default() -> BuiltinOrder {
        let mut names: Vec<(String, usize)> = builtin_names().into_iter().zip(0..).collect();
        names.sort();
        BuiltinOrder(names.into())
    }
}

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
    /// their first frame, and at every frame of theirs ([`trace::canon::hash_frames`]), and the web build's
    /// embed reports its own, so a page can tell a build that plays the clip unlike its video
    /// (docs/wiki-media.md, "The state at the first frame" and "The states through the clip"). The same on
    /// every platform, as the world is.
    pub fn frame_hash(&self) -> u64 {
        let mut buf = Vec::new();
        FrameEncoder::new().encode(self, &mut buf);
        trace::canon::hash(&buf)
    }

    /// The state after the last frame, numbered as the oracle numbers it (the first frame is 0). Instances
    /// and deactivated instances are sorted by id; alarms list those that are set; `vars` and `globals`
    /// hold every assigned variable.
    /// `builtins` holds the views, the background layers and the drawing state ([`builtin_names`]), as the
    /// oracle dumps them.
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
            tiles: Some({
                let mut order = Vec::new();
                self.tiles_in_draw_order(&mut order);
                order
                    .iter()
                    .map(|&i| trace_tile(&self.room.tiles[i]))
                    .collect()
            }),
        }
    }

    /// The room's tiles in the order a draw puts them: by depth, highest first, at one depth in the order they
    /// were added. The draw's own sort is stable, so this is that order whether or not one has run (and
    /// headless none does), as the oracle's dump, which sorts its draw order the same way, has it. Into `order`,
    /// as indices into the room's tiles; after a recorded draw they're in that order already.
    fn tiles_in_draw_order(&self, order: &mut Vec<usize>) {
        let tiles = &self.room.tiles;
        let cmp = |a: &crate::room::Tile, b: &crate::room::Tile| {
            crate::list::cmp_nan_first(b.depth, a.depth)
        };
        order.clear();
        order.extend(0..tiles.len());
        if !tiles.is_sorted_by(|a, b| cmp(a, b).is_le()) {
            order.sort_by(|&a, &b| cmp(&tiles[a], &tiles[b]));
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
        (builtin_names().into_iter().zip(self.builtin_values()))
            .map(|(k, n)| (k, scalar(&Value::from(n)).into()))
            .collect()
    }

    /// The values of [`builtin_names`]' built-ins, in its order.
    fn builtin_values(&self) -> [f64; BUILTIN_COUNT] {
        let b = |b: bool| f64::from(u8::from(b));
        let r = &self.room;
        let s = self.draw_state;
        let target = match s.target {
            draw::Target::Screen => -1.0,
            draw::Target::Surface(id) => f64::from(id.0),
        };
        let mut out = [0.0; BUILTIN_COUNT];
        let frame = [
            f64::from(r.background_color as i32),
            b(r.show_color),
            b(r.views_enabled),
            f64::from(self.draw_color().0 as i32),
            self.draw_alpha(),
            f64::from(self.draw_font()),
            f64::from(s.blend.src as i32),
            f64::from(s.blend.dest as i32),
            target,
        ];
        let views = r.views.iter().flat_map(|v| {
            [
                b(v.visible),
                f64::from(v.xview),
                f64::from(v.yview),
                f64::from(v.wview),
                f64::from(v.hview),
                f64::from(v.xport),
                f64::from(v.yport),
                f64::from(v.wport),
                f64::from(v.hport),
                v.angle,
                f64::from(v.hborder),
                f64::from(v.vborder),
                f64::from(v.hspeed),
                f64::from(v.vspeed),
                f64::from(v.object),
            ]
        });
        let backgrounds = r.backgrounds.iter().flat_map(|l| {
            [
                b(l.visible),
                b(l.foreground),
                f64::from(l.index),
                l.x,
                l.y,
                b(l.tile_h),
                b(l.tile_v),
                l.hspeed,
                l.vspeed,
                l.xscale,
                l.yscale,
                f64::from(l.blend as i32),
                l.alpha,
            ]
        });
        for (o, v) in out
            .iter_mut()
            .zip(frame.into_iter().chain(views).chain(backgrounds))
        {
            *o = v;
        }
        out
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
            image_angle: r(i.image_angle),
            image_alpha: r(i.image_alpha),
            depth: r(i.depth),
            sprite_index: Some(i.sprite_index),
            sprite: Some(sprite.map_or(String::new(), |s| s.name.clone())),
            visible: Some(i.visible),
            solid: Some(i.solid),
            image_blend: Some(i.image_blend),
            alarms: Some(alarms),
            vars,
        }
    }
}

fn trace_tile(t: &crate::room::Tile) -> trace::Tile {
    trace::Tile {
        id: t.id,
        background: t.background,
        left: t.left,
        top: t.top,
        width: t.width,
        height: t.height,
        x: trace::Real(t.x),
        y: trace::Real(t.y),
        depth: trace::Real(t.depth),
        xscale: trace::Real(t.xscale),
        yscale: trace::Real(t.yscale),
        blend: t.blend as i32,
        alpha: trace::Real(t.alpha),
        visible: t.visible,
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
    /// The tiles in draw order, by index, reused.
    tiles: Vec<usize>,
    /// The built-ins' names in name order.
    builtins: BuiltinOrder,
    /// Globals left out of the frame, by name ([`skipping_globals`](Self::skipping_globals)).
    skip_global: Option<fn(&str) -> bool>,
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
        tiles: _,
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
        image_angle: _,
        image_alpha: _,
        depth: _,
        sprite_index: _,
        sprite: _,
        visible: _,
        solid: _,
        image_blend: _,
        alarms: _,
        vars: _,
    } = i;
};

impl FrameEncoder {
    pub fn new() -> FrameEncoder {
        FrameEncoder::default()
    }

    /// One that leaves out the globals `skip` names: a frame that isn't the trace's, for comparing two
    /// runs that may differ only there (the sound handles of two audio modes, `harness`'s
    /// `fidelity-check`).
    pub fn skipping_globals(skip: fn(&str) -> bool) -> FrameEncoder {
        FrameEncoder {
            skip_global: Some(skip),
            ..FrameEncoder::default()
        }
    }

    /// Appends the world's frame to `out`.
    pub fn encode<'w>(&mut self, w: &'w World, out: &mut Vec<u8>) -> FrameKey<'w> {
        let p = w.program;
        if !self.program.is_some_and(|q| std::ptr::eq(q, p)) {
            let skip = self.skip_global;
            let mut globals: Vec<u16> = (0..p.global_count() as u16)
                .filter(|&g| !skip.is_some_and(|skip| skip(p.global_name(GVar(g)))))
                .collect();
            globals.sort_by(|&a, &b| p.global_name(GVar(a)).cmp(p.global_name(GVar(b))));
            *self = FrameEncoder {
                program: Some(p),
                globals: globals.into(),
                skip_global: skip,
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
        e.len(BUILTIN_COUNT);
        let values = w.builtin_values();
        for (name, at) in &self.builtins.0 {
            var(e, name, &Slot::Scalar(Value::from(values[*at])));
        }
        e.some(true);
        e.len(w.room.tiles.len());
        let mut order = std::mem::take(&mut self.tiles);
        w.tiles_in_draw_order(&mut order);
        for &i in &order {
            e.tile(&trace_tile(&w.room.tiles[i]));
        }
        self.tiles = order;
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
            i.image_angle,
            i.image_alpha,
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
        e.i32(i.image_blend);
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
        // aren't ASCII, NaN and -0, set alarms, a deactivated instance, tiles added out of depth order, the
        // draw-only state.
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
        let background = assets::Background {
            name: "bg".into(),
            file: None,
            width: 64,
            height: 32,
            use_as_tileset: true,
            tile_width: 16,
            tile_height: 16,
            h_offset: 0,
            v_offset: 0,
            h_sep: 0,
            v_sep: 0,
        };
        let assets =
            Assets::new(Vec::new(), vec![Some(room)]).with_backgrounds(vec![Some(background)]);
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
        // What only drawing reads: an instance's, the layers' and views', the drawing state.
        let i = w.instance_mut(h0);
        (i.image_alpha, i.image_blend, i.image_angle) = (0.25, 0x12_34_56, -0.0);
        w.room.views[3].visible = true;
        w.room.views[7].angle = f64::NAN;
        w.room.backgrounds[2].x = -1.5;
        w.draw_set_color(0x00_ff_00);
        w.draw_set_alpha(0.5);
        w.draw_set_blend_mode(1);
        for (depth, x) in [(5.0, 1.0), (9.0, 2.0), (5.0, -0.0), (-1.0, 3.5)] {
            w.tile_add(0, 16, 0, 16, 16, x, 4.0, depth).unwrap();
        }
        let order: Vec<i32> = w
            .trace_frame()
            .tiles
            .unwrap()
            .iter()
            .map(|t| t.id)
            .collect();
        assert_eq!(order, [10_000_002, 10_000_001, 10_000_003, 10_000_004]);
        assert_eq!(
            w.instance(h1).extra.len(),
            3,
            "the extras are outside the layout"
        );
        let (fast, slow) = both(&w);
        assert_eq!(fast, slow);
        // Committed in the wiki's clips (`World::frame_hash`), and compared with the web build's: the same
        // natively and on wasm32 (scripts/wasm-test), NaN, -0 and all.
        assert_eq!(format!("{:016x}", w.frame_hash()), "98e8b21eb68b8cff");
    }

    #[test]
    fn the_hash_is_fixed() {
        // Fingerprints and the wiki's clips are committed: changing the hash means recording them all again.
        assert_eq!(trace::canon::hash(b""), 0xf4a9_e556_70e1_91bb);
        assert_eq!(trace::canon::hash(b"spelunky"), 0xb42d_3d89_3cbe_d43f);
        assert_eq!(trace::canon::hash_frames(&[]), trace::canon::hash(b""));
        assert_eq!(
            trace::canon::hash_frames(&[1, u64::MAX]),
            0x7627_b6a6_aa7f_7b70
        );
    }

    #[test]
    fn builtins_are_named_once_each() {
        let names = builtin_names();
        assert_eq!(names.len(), BUILTIN_COUNT);
        let order = BuiltinOrder::default();
        assert!(order.0.is_sorted());
        assert!(order.0.windows(2).all(|p| p[0].0 != p[1].0), "{names:?}");
        assert!(
            names.contains(&"view_xview[0]".into())
                && names.contains(&"background_alpha[7]".into())
        );
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
