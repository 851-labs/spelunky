//! Variable access for generated code: instance variables (built-in and user), `target.v`, globals and
//! game-wide built-ins (runtime.md sections 3 and 12). Built-in semantics follow OpenGMK `gml/runtime.rs`
//! (`get_instance_var`, `set_instance_var`), field storage OpenGMK `instance.rs` (`Field`).

use crate::Ctx;
use crate::error::{Error, R};
use crate::instance::Instance;
use crate::kernel::{Target, target_of};
use crate::list::Inst;
use crate::ops::flat;
use crate::room::{BACKGROUNDS, VIEWS, layer};
use crate::value::Value;
use crate::var::{Builtin, GVar, Slot, Var};
use crate::world::{SceneChange, World};

impl World {
    /// `v` on instance `h` (a built-in or a user variable).
    #[inline]
    pub fn get(&self, h: Inst, v: Var) -> R<Value> {
        self.get_ix(h, v, 0)
    }

    /// `v = x` on instance `h`.
    #[inline]
    pub fn set(&mut self, h: Inst, v: Var, x: Value) -> R<()> {
        self.set_ix(h, v, 0, x)
    }

    /// `v[i, j]` on instance `h` (`v[j]` is `v[0, j]`). Indices come from [`ix`](crate::ix).
    #[inline]
    pub fn get_a(&self, h: Inst, v: Var, i: u32, j: u32) -> R<Value> {
        self.get_ix(h, v, flat(i, j))
    }

    /// `v[i, j] = x` on instance `h`.
    #[inline]
    pub fn set_a(&mut self, h: Inst, v: Var, i: u32, j: u32, x: Value) -> R<()> {
        self.set_ix(h, v, flat(i, j), x)
    }

    fn get_ix(&self, h: Inst, v: Var, index: u32) -> R<Value> {
        match Builtin::of(v) {
            Some(b) if b.is_instance() => self.get_builtin(h, b, index),
            // `other.room` and the like read the game-wide value.
            Some(b) => self.get_game(b, index),
            None => {
                let inst = self.insts.get(h);
                self.user_slot(inst, v)
                    .and_then(|s| s.get(index))
                    .ok_or(Error::UnsetVar(v))
            }
        }
    }

    fn set_ix(&mut self, h: Inst, v: Var, index: u32, x: Value) -> R<()> {
        match Builtin::of(v) {
            Some(b) if b.is_instance() => self.set_builtin(h, b, index, x),
            Some(b) => self.set_game(b, index, x),
            None => {
                self.user_slot_mut(h, v).set(index, x);
                Ok(())
            }
        }
    }

    fn user_slot<'a>(&self, inst: &'a Instance, v: Var) -> Option<&'a Slot> {
        match self.program.layout(inst.object).slot(v) {
            Some(i) => Some(&inst.vars[i]),
            None => inst.extra.iter().find(|(x, _)| *x == v).map(|(_, s)| s),
        }
    }

    fn user_slot_mut(&mut self, h: Inst, v: Var) -> &mut Slot {
        let layout = self.program.layout(self.insts.get(h).object);
        let inst = self.insts.get_mut(h);
        if let Some(i) = layout.slot(v) {
            return &mut inst.vars[i];
        }
        let i = match inst.extra.iter().position(|(x, _)| *x == v) {
            Some(i) => i,
            None => {
                inst.extra.push((v, Slot::Unset));
                inst.extra.len() - 1
            }
        };
        &mut inst.extra[i].1
    }

    fn get_builtin(&self, h: Inst, b: Builtin, index: u32) -> R<Value> {
        use Builtin::*;
        let i = self.insts.get(h);
        let sprite = || self.assets.sprite(i.sprite_index);
        let bbox = |f: fn(&crate::Bbox) -> i32| Value::from(f(&self.bbox(h)));
        Ok(match b {
            X => Value::from(i.x),
            Y => Value::from(i.y),
            Xprevious => Value::from(i.xprevious),
            Yprevious => Value::from(i.yprevious),
            Xstart => Value::from(i.xstart),
            Ystart => Value::from(i.ystart),
            Hspeed => Value::from(i.hspeed),
            Vspeed => Value::from(i.vspeed),
            Direction => Value::from(i.direction),
            Speed => Value::from(i.speed),
            Friction => Value::from(i.friction),
            Gravity => Value::from(i.gravity),
            GravityDirection => Value::from(i.gravity_direction),
            ObjectIndex => Value::from(f64::from(i.object.0)),
            Id => Value::from(i.id),
            // GM8 keeps alarms in a map, so alarm[n] past 11 reads -1 until written. Spelunky only uses 0-11.
            Alarm => Value::from(i.alarms.get(index as usize).copied().unwrap_or(-1)),
            Solid => Value::from(i.solid),
            Visible => Value::from(i.visible),
            Persistent => Value::from(i.persistent),
            Depth => Value::from(i.depth),
            SpriteIndex => Value::from(i.sprite_index),
            SpriteWidth => {
                Value::from(sprite().map_or(0.0, |s| i.image_xscale * f64::from(s.width)))
            }
            SpriteHeight => {
                Value::from(sprite().map_or(0.0, |s| i.image_yscale * f64::from(s.height)))
            }
            SpriteXoffset => Value::from(sprite().map_or(0, |s| s.origin_x)),
            SpriteYoffset => Value::from(sprite().map_or(0, |s| s.origin_y)),
            ImageAlpha => Value::from(i.image_alpha),
            ImageAngle => Value::from(i.image_angle),
            ImageBlend => Value::from(i.image_blend),
            ImageIndex => Value::from(i.image_index),
            ImageNumber => Value::from(sprite().map_or(0, |s| s.frames.len()) as f64),
            ImageSpeed => Value::from(i.image_speed),
            ImageXscale => Value::from(i.image_xscale),
            ImageYscale => Value::from(i.image_yscale),
            MaskIndex => Value::from(i.mask_index),
            BboxLeft => bbox(|b| b.left),
            BboxRight => bbox(|b| b.right),
            BboxTop => bbox(|b| b.top),
            BboxBottom => bbox(|b| b.bottom),
            PathIndex | PathPosition | PathSpeed | TimelineIndex | TimelinePosition => {
                return Err(Error::Unsupported("paths and timelines"));
            }
            _ => unreachable!("{b:?} is game-wide"),
        })
    }

    fn set_builtin(&mut self, h: Inst, b: Builtin, index: u32, x: Value) -> R<()> {
        use Builtin::*;
        let frames = match b {
            SpriteIndex => self.frame_count(x.to_int()),
            _ => 0,
        };
        let i = self.insts.get_mut(h);
        match b {
            X => i.x = x.to_real(),
            Y => i.y = x.to_real(),
            Xprevious => i.xprevious = x.to_real(),
            Yprevious => i.yprevious = x.to_real(),
            Xstart => i.xstart = x.to_real(),
            Ystart => i.ystart = x.to_real(),
            Hspeed => i.set_hspeed(x.to_real()),
            Vspeed => i.set_vspeed(x.to_real()),
            Direction => i.set_direction(x.to_real()),
            Speed => i.set_speed(x.to_real()),
            Friction => i.friction = x.to_real(),
            Gravity => i.gravity = x.to_real(),
            GravityDirection => i.gravity_direction = x.to_real(),
            // Rounded half to even like any integer argument (OpenGMK: `value.into()` to i32).
            Alarm => match i.alarms.get_mut(index as usize) {
                Some(a) => *a = x.to_int(),
                None => return Err(Error::Unsupported("alarm index above 11")),
            },
            Solid => i.solid = x.is_truthy(),
            Visible => i.visible = x.is_truthy(),
            Persistent => i.persistent = x.is_truthy(),
            Depth => i.depth = x.to_real(),
            SpriteIndex => i.set_sprite_index(x.to_int(), frames),
            ImageAlpha => i.image_alpha = x.to_real(),
            ImageAngle => i.image_angle = x.to_real(),
            ImageBlend => i.image_blend = x.to_int(),
            ImageIndex => i.image_index = x.to_real(),
            ImageSpeed => i.image_speed = x.to_real(),
            ImageXscale => i.image_xscale = x.to_real(),
            ImageYscale => i.image_yscale = x.to_real(),
            MaskIndex => i.mask_index = x.to_int(),
            ObjectIndex | Id | SpriteWidth | SpriteHeight | SpriteXoffset | SpriteYoffset
            | ImageNumber | BboxLeft | BboxRight | BboxTop | BboxBottom => {
                return Err(Error::ReadOnly(b.var()));
            }
            PathIndex | PathPosition | PathSpeed | TimelineIndex | TimelinePosition => {
                return Err(Error::Unsupported("paths and timelines"));
            }
            _ => unreachable!("{b:?} is game-wide"),
        }
        Ok(())
    }

    /// A game-wide built-in (`room`, `view_xview[i]`, `keyboard_key`, ...). `v` must be one.
    pub fn bget(&self, v: Var, i: u32) -> R<Value> {
        match Builtin::of(v) {
            Some(b) if !b.is_instance() => self.get_game(b, i),
            _ => Err(Error::Unsupported("bget of an instance variable")),
        }
    }

    /// Writes a game-wide built-in.
    pub fn bset(&mut self, v: Var, i: u32, x: Value) -> R<()> {
        match Builtin::of(v) {
            Some(b) if !b.is_instance() => self.set_game(b, i, x),
            _ => Err(Error::Unsupported("bset of an instance variable")),
        }
    }

    /// A game-wide built-in. `view_*[i]` and `background_*[i]` past the last view or layer mean element 0
    /// (`room::layer`).
    fn get_game(&self, b: Builtin, index: u32) -> R<Value> {
        use Builtin::*;
        let input = &self.input;
        let view = &self.room.views[layer(index, VIEWS)];
        let background = &self.room.backgrounds[layer(index, BACKGROUNDS)];
        Ok(match b {
            Room => Value::from(self.room.id),
            RoomWidth => Value::from(self.room.width),
            RoomHeight => Value::from(self.room.height),
            RoomSpeed => Value::from(self.room.speed),
            ViewEnabled => Value::from(self.room.views_enabled),
            ViewCurrent => Value::from(self.view_current as i32),
            ViewVisible => Value::from(view.visible),
            ViewXview => Value::from(view.xview),
            ViewYview => Value::from(view.yview),
            ViewWview => Value::from(view.wview),
            ViewHview => Value::from(view.hview),
            ViewXport => Value::from(view.xport),
            ViewYport => Value::from(view.yport),
            ViewWport => Value::from(view.wport),
            ViewHport => Value::from(view.hport),
            ViewAngle => Value::from(view.angle),
            ViewHborder => Value::from(view.hborder),
            ViewVborder => Value::from(view.vborder),
            ViewHspeed => Value::from(view.hspeed),
            ViewVspeed => Value::from(view.vspeed),
            ViewObject => Value::from(view.object),
            BackgroundIndex => Value::from(background.index),
            BackgroundColor => Value::from(f64::from(self.room.background_color)),
            KeyboardKey => Value::from(i32::from(input.keyboard_key())),
            KeyboardLastkey => Value::from(i32::from(input.keyboard_lastkey())),
            MouseX => Value::from(self.mouse_in_room().0),
            MouseY => Value::from(self.mouse_in_room().1),
            MouseButton => Value::from(input.mouse_button()),
            MouseLastbutton => Value::from(input.mouse_lastbutton()),
            // Counts deleted instances too, until the cleanup. OpenGMK `instancelist.rs` (`count_all`).
            InstanceCount => Value::from(self.insts.count_all() as f64),
            _ => return Err(Error::Unsupported(b.name())),
        })
    }

    /// Writes a game-wide built-in. View and background values are integers, rounded as built-in arguments
    /// are; flags are truthiness. OpenGMK `gml/runtime.rs` (`set_instance_var`).
    fn set_game(&mut self, b: Builtin, index: u32, x: Value) -> R<()> {
        use Builtin::*;
        let n = x.to_int();
        let view = &mut self.room.views[layer(index, VIEWS)];
        match b {
            // Assigning `room` is a room change, like `room_goto`.
            Room => self.request_scene_change(SceneChange::Room(n)),
            RoomSpeed if n <= 0 => return Err(Error::Unsupported("room_speed <= 0")),
            RoomSpeed => self.room.speed = n,
            ViewEnabled => self.room.views_enabled = x.is_truthy(),
            ViewVisible => view.visible = x.is_truthy(),
            ViewXview => view.xview = n,
            ViewYview => view.yview = n,
            ViewWview => view.wview = n,
            ViewHview => view.hview = n,
            ViewXport => view.xport = n,
            ViewYport => view.yport = n,
            ViewWport => view.wport = n,
            ViewHport => view.hport = n,
            ViewAngle => view.angle = x.to_real(),
            ViewHborder => view.hborder = n,
            ViewVborder => view.vborder = n,
            ViewHspeed => view.hspeed = n,
            ViewVspeed => view.vspeed = n,
            ViewObject => view.object = n,
            BackgroundIndex => self.room.backgrounds[layer(index, BACKGROUNDS)].index = n,
            // `round() as u32`: negative colours saturate to 0.
            BackgroundColor => self.room.background_color = crate::math::round(x.to_real()) as u32,
            // GM8 ignores key codes outside 0-255 and buttons other than mb_left/right/middle (OpenGMK
            // `gml/runtime.rs` `set_instance_var`).
            KeyboardKey => {
                if let Ok(vk) = u8::try_from(n) {
                    self.input.set_keyboard_key(vk);
                }
            }
            KeyboardLastkey => {
                if let Ok(vk) = u8::try_from(n) {
                    self.input.set_keyboard_lastkey(vk);
                }
            }
            MouseButton | MouseLastbutton => {
                if (1..=3).contains(&n) {
                    if b == MouseButton {
                        self.input.set_mouse_button(n);
                    } else {
                        self.input.set_mouse_lastbutton(n);
                    }
                }
            }
            RoomWidth | RoomHeight | ViewCurrent | MouseX | MouseY | InstanceCount | Fps
            | CurrentTime | WorkingDirectory => return Err(Error::ReadOnly(b.var())),
            _ => return Err(Error::Unsupported(b.name())),
        }
        Ok(())
    }

    /// `target.v`: reads the first instance `target` names (an object, `all`, an instance id, `self` or
    /// `other`). No instance is an unset-variable error. OpenGMK `gml/runtime.rs` (`Node::Field`).
    pub fn get_on(&self, cx: &Ctx, target: &Value, v: Var) -> R<Value> {
        self.get_on_ix(cx, target, v, 0)
    }

    /// `target.v = x`: writes every instance `target` names, in iteration order.
    pub fn set_on(&mut self, cx: &Ctx, target: &Value, v: Var, x: Value) -> R<()> {
        self.set_on_ix(cx, target, v, 0, x)
    }

    pub fn get_on_a(&self, cx: &Ctx, target: &Value, v: Var, i: u32, j: u32) -> R<Value> {
        self.get_on_ix(cx, target, v, flat(i, j))
    }

    pub fn set_on_a(
        &mut self,
        cx: &Ctx,
        target: &Value,
        v: Var,
        i: u32,
        j: u32,
        x: Value,
    ) -> R<()> {
        self.set_on_ix(cx, target, v, flat(i, j), x)
    }

    fn get_on_ix(&self, cx: &Ctx, target: &Value, v: Var, index: u32) -> R<Value> {
        let h = match target_of(self, cx, target)? {
            Target::One(h) => h,
            Target::List(mut it) => it.next(&self.insts),
        };
        self.get_ix(h.ok_or(Error::UnsetVar(v))?, v, index)
    }

    fn set_on_ix(&mut self, cx: &Ctx, target: &Value, v: Var, index: u32, x: Value) -> R<()> {
        match target_of(self, cx, target)? {
            Target::One(None) => Ok(()),
            Target::One(Some(h)) => self.set_ix(h, v, index, x),
            Target::List(mut it) => {
                while let Some(h) = it.next(&self.insts) {
                    self.set_ix(h, v, index, x.clone())?;
                }
                Ok(())
            }
        }
    }

    /// `global.v`.
    #[inline]
    pub fn gget(&self, v: GVar) -> R<Value> {
        self.gget_ix(v, 0)
    }

    /// `global.v = x`.
    #[inline]
    pub fn gset(&mut self, v: GVar, x: Value) -> R<()> {
        self.gset_ix(v, 0, x)
    }

    pub fn gget_a(&self, v: GVar, i: u32, j: u32) -> R<Value> {
        self.gget_ix(v, flat(i, j))
    }

    pub fn gset_a(&mut self, v: GVar, i: u32, j: u32, x: Value) -> R<()> {
        self.gset_ix(v, flat(i, j), x)
    }

    fn gget_ix(&self, v: GVar, index: u32) -> R<Value> {
        self.globals[usize::from(v.0)]
            .get(index)
            .ok_or(Error::UnsetGlobal(v))
    }

    fn gset_ix(&mut self, v: GVar, index: u32, x: Value) -> R<()> {
        self.globals[usize::from(v.0)].set(index, x);
        Ok(())
    }

    /// Every set user variable of an instance: those in its layout (by variable id), then overflow ones.
    pub(crate) fn user_vars<'a>(
        &'a self,
        inst: &'a Instance,
    ) -> impl Iterator<Item = (Var, &'a Slot)> {
        self.program
            .layout(inst.object)
            .vars()
            .map(|(v, i)| (v, &inst.vars[i]))
            .chain(inst.extra.iter().map(|(v, s)| (*v, s)))
            .filter(|(_, s)| s.is_set())
    }
}
