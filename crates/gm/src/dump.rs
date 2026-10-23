//! The world as a `trace::Frame`, in the oracle's schema (docs/formats.md, "Traces").

use crate::instance::{Instance, State};
use crate::string::GmString;
use crate::value::Value;
use crate::var::Slot;
use crate::world::World;
use std::collections::BTreeMap;

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

    /// The built-ins the oracle dumps: view 0's rectangle (it decides `oLevel`'s deactivation region).
    fn trace_builtins(&self) -> BTreeMap<String, trace::Value> {
        let v = &self.room.views[0];
        [
            ("view_hview[0]", v.hview),
            ("view_wview[0]", v.wview),
            ("view_xview[0]", v.xview),
            ("view_yview[0]", v.yview),
        ]
        .into_iter()
        .map(|(k, n)| (k.to_string(), scalar(&Value::from(n)).into()))
        .collect()
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
