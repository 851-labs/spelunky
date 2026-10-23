//! Variable ids and storage (runtime.md section 3).
//!
//! The transpiler interns every variable name into a [`Var`]. Built-ins take the low ids, in the order of
//! [`Builtin`], so a generator that depends on `gm` interns them with [`Builtin::from_name`] and numbers
//! user variables from [`FIRST_USER_VAR`]. Globals get their own dense id space, [`GVar`].

use crate::error::{Error, R};
use crate::ops::ARRAY_STRIDE;
use crate::value::Value;
use std::collections::BTreeMap;

/// A variable id: a built-in (instance or game-wide) below [`FIRST_USER_VAR`], a user instance variable
/// from there on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Var(pub u16);

/// A global variable id (`global.name` or a `globalvar` name).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GVar(pub u16);

macro_rules! builtins {
    (instance: [$($i:ident $iname:literal,)*] game: [$($g:ident $gname:literal,)*]) => {
        /// The built-in variables, instance ones first. Their discriminants are their [`Var`] ids. The names
        /// are the GM8 set the transpiler recognises (`crates/gml/examples/common/mod.rs`,
        /// `INSTANCE_BUILTINS` and `GLOBAL_BUILTINS`).
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(u16)]
        pub enum Builtin { $($i,)* $($g,)* }

        impl Builtin {
            /// Every built-in, indexed by id.
            pub const ALL: &'static [Builtin] = &[$(Builtin::$i,)* $(Builtin::$g,)*];
            /// How many of [`ALL`](Self::ALL) live on each instance; the rest are game-wide.
            pub const INSTANCE_COUNT: u16 = [$(Builtin::$i,)*].len() as u16;

            /// The GML name.
            pub const fn name(self) -> &'static str {
                match self { $(Builtin::$i => $iname,)* $(Builtin::$g => $gname,)* }
            }
        }
    };
}

builtins! {
    instance: [
        X "x", Y "y", Xprevious "xprevious", Yprevious "yprevious", Xstart "xstart", Ystart "ystart",
        Hspeed "hspeed", Vspeed "vspeed", Direction "direction", Speed "speed", Friction "friction",
        Gravity "gravity", GravityDirection "gravity_direction", ObjectIndex "object_index", Id "id",
        Alarm "alarm", Solid "solid", Visible "visible", Persistent "persistent", Depth "depth",
        SpriteIndex "sprite_index", SpriteWidth "sprite_width", SpriteHeight "sprite_height",
        SpriteXoffset "sprite_xoffset", SpriteYoffset "sprite_yoffset", ImageAlpha "image_alpha",
        ImageAngle "image_angle", ImageBlend "image_blend", ImageIndex "image_index",
        ImageNumber "image_number", ImageSpeed "image_speed", ImageXscale "image_xscale",
        ImageYscale "image_yscale", MaskIndex "mask_index", BboxLeft "bbox_left", BboxRight "bbox_right",
        BboxTop "bbox_top", BboxBottom "bbox_bottom", PathIndex "path_index",
        PathPosition "path_position", PathSpeed "path_speed", TimelineIndex "timeline_index",
        TimelinePosition "timeline_position",
    ]
    game: [
        Room "room", RoomWidth "room_width", RoomHeight "room_height", RoomSpeed "room_speed",
        RoomCaption "room_caption", RoomPersistent "room_persistent", ViewEnabled "view_enabled",
        ViewCurrent "view_current", ViewVisible "view_visible", ViewXview "view_xview",
        ViewYview "view_yview", ViewWview "view_wview", ViewHview "view_hview", ViewXport "view_xport",
        ViewYport "view_yport", ViewWport "view_wport", ViewHport "view_hport", ViewAngle "view_angle",
        ViewHborder "view_hborder", ViewVborder "view_vborder", ViewHspeed "view_hspeed",
        ViewVspeed "view_vspeed", ViewObject "view_object", BackgroundIndex "background_index",
        BackgroundColor "background_color", KeyboardKey "keyboard_key",
        KeyboardLastkey "keyboard_lastkey", KeyboardLastchar "keyboard_lastchar",
        KeyboardString "keyboard_string", MouseX "mouse_x", MouseY "mouse_y",
        MouseButton "mouse_button", MouseLastbutton "mouse_lastbutton", Score "score", Lives "lives",
        Health "health", InstanceCount "instance_count", Fps "fps", CurrentTime "current_time",
        WorkingDirectory "working_directory",
    ]
}

/// The first user variable id: every id below it is a [`Builtin`].
pub const FIRST_USER_VAR: u16 = Builtin::ALL.len() as u16;

impl Builtin {
    /// The built-in with this GML name.
    pub fn from_name(name: &str) -> Option<Builtin> {
        Builtin::ALL.iter().copied().find(|b| b.name() == name)
    }

    /// The built-in a variable id names, if it is one.
    #[inline]
    pub fn of(v: Var) -> Option<Builtin> {
        Builtin::ALL.get(usize::from(v.0)).copied()
    }

    /// Whether it lives on each instance (`x`, `alarm`) rather than game-wide (`room`, `view_xview`).
    #[inline]
    pub fn is_instance(self) -> bool {
        (self as u16) < Builtin::INSTANCE_COUNT
    }

    #[inline]
    pub const fn var(self) -> Var {
        Var(self as u16)
    }
}

/// One variable's storage: unset, a scalar, or a GM8 array. GM8 treats `a` and `a[0]` as the same storage
/// (OpenGMK `instance.rs`, `Field`).
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Slot {
    #[default]
    Unset,
    Scalar(Value),
    /// Elements by flat index (`i * 32000 + j`). Boxed so a slot stays the size of a value.
    Array(Box<BTreeMap<u32, Value>>),
}

impl Slot {
    /// Element `index` (0 for a plain read). An array element that was never set reads as 0 if a higher
    /// element in the same row was set, because GM8 arrays are dense up to their highest index; otherwise
    /// it's unset. OpenGMK `instance.rs` (`Field::get`).
    pub fn get(&self, index: u32) -> Option<Value> {
        match self {
            Slot::Unset => None,
            Slot::Scalar(v) => (index == 0).then(|| v.clone()),
            Slot::Array(m) => m.get(&index).cloned().or_else(|| {
                let row_end = (index / ARRAY_STRIDE + 1) * ARRAY_STRIDE;
                m.range(index + 1..row_end).next().map(|_| Value::Real(0.0))
            }),
        }
    }

    /// Sets element `index`. Writing a non-zero index to a scalar turns it into an array that keeps the
    /// scalar as element 0. OpenGMK `instance.rs` (`Field::new`, `Field::set`).
    pub fn set(&mut self, index: u32, value: Value) {
        match self {
            Slot::Unset if index == 0 => *self = Slot::Scalar(value),
            Slot::Unset => *self = Slot::Array(Box::new(BTreeMap::from([(index, value)]))),
            Slot::Scalar(v) if index == 0 => *v = value,
            Slot::Scalar(v) => {
                let old = std::mem::take(v);
                *self = Slot::Array(Box::new(BTreeMap::from([(0, old), (index, value)])));
            }
            Slot::Array(m) => {
                m.insert(index, value);
            }
        }
    }

    pub fn is_set(&self) -> bool {
        !matches!(self, Slot::Unset)
    }
}

/// A `var` local of generated code: unset until assigned, then a scalar or an array like any variable.
#[derive(Clone, Debug, Default)]
pub struct Local(Slot);

impl Local {
    pub fn get(&self) -> R<Value> {
        self.0.get(0).ok_or(Error::UnsetLocal)
    }

    pub fn set(&mut self, v: Value) {
        self.0.set(0, v)
    }

    pub fn get_a(&self, i: u32, j: u32) -> R<Value> {
        self.0.get(crate::ops::flat(i, j)).ok_or(Error::UnsetLocal)
    }

    pub fn set_a(&mut self, i: u32, j: u32, v: Value) {
        self.0.set(crate::ops::flat(i, j), v)
    }
}

/// Which user variables an object's instances have a slot for (runtime.md section 3): a dense table from
/// user variable to slot. A variable outside the layout still works; it goes to the instance's overflow
/// list, so the layout is only an optimisation.
#[derive(Clone, Debug, Default)]
pub struct VarLayout {
    /// Indexed by `var - FIRST_USER_VAR`; [`VarLayout::NONE`] for no slot.
    slots: Box<[u16]>,
    len: u16,
}

impl VarLayout {
    const NONE: u16 = u16::MAX;

    /// A layout holding `vars` in this order, for a program with `user_vars` user variables.
    pub fn new(vars: &[Var], user_vars: usize) -> VarLayout {
        let mut slots = vec![Self::NONE; user_vars].into_boxed_slice();
        let mut len = 0u16;
        for v in vars {
            let i = usize::from(v.0 - FIRST_USER_VAR);
            if slots[i] == Self::NONE {
                slots[i] = len;
                len += 1;
            }
        }
        VarLayout { slots, len }
    }

    /// The slot for a user variable, if the layout has one.
    #[inline]
    pub fn slot(&self, v: Var) -> Option<usize> {
        let s = *self
            .slots
            .get(usize::from(v.0.checked_sub(FIRST_USER_VAR)?))?;
        (s != Self::NONE).then_some(usize::from(s))
    }

    /// How many slots an instance needs.
    #[inline]
    pub fn len(&self) -> usize {
        usize::from(self.len)
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The variables in the layout with their slots, by variable id.
    pub fn vars(&self) -> impl Iterator<Item = (Var, usize)> + '_ {
        self.slots
            .iter()
            .enumerate()
            .filter(|(_, s)| **s != Self::NONE)
            .map(|(i, s)| (Var(i as u16 + FIRST_USER_VAR), usize::from(*s)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_ids_are_stable() {
        assert_eq!(Builtin::X.var(), Var(0));
        assert_eq!(Builtin::from_name("alarm"), Some(Builtin::Alarm));
        assert_eq!(Builtin::INSTANCE_COUNT, 43);
        assert!(Builtin::TimelinePosition.is_instance());
        assert!(!Builtin::Room.is_instance());
        assert_eq!(FIRST_USER_VAR, 83);
        assert_eq!(Builtin::of(Var(FIRST_USER_VAR)), None);
        for (i, b) in Builtin::ALL.iter().enumerate() {
            assert_eq!(*b as usize, i);
            assert_eq!(Builtin::from_name(b.name()), Some(*b));
        }
    }

    #[test]
    fn scalars_and_arrays_share_storage() {
        let mut s = Slot::Unset;
        assert_eq!(s.get(0), None);
        s.set(0, Value::Real(7.0));
        assert_eq!(s.get(0), Some(Value::Real(7.0)));
        assert_eq!(s.get(1), None);
        s.set(3, Value::Real(9.0));
        assert_eq!(s.get(0), Some(Value::Real(7.0)));
        // Below the highest set element of the row: 0. Above it, or in another row: unset.
        assert_eq!(s.get(2), Some(Value::Real(0.0)));
        assert_eq!(s.get(4), None);
        assert_eq!(s.get(ARRAY_STRIDE + 1), None);
    }

    #[test]
    fn two_d_rows_are_independent() {
        let mut s = Slot::Unset;
        s.set(2 * ARRAY_STRIDE + 5, Value::Real(1.0));
        assert!(matches!(s, Slot::Array(_)));
        assert_eq!(s.get(2 * ARRAY_STRIDE + 4), Some(Value::Real(0.0)));
        assert_eq!(s.get(ARRAY_STRIDE + 4), None);
        assert_eq!(s.get(0), None);
    }

    #[test]
    fn layouts_map_vars_to_dense_slots() {
        let a = Var(FIRST_USER_VAR + 2);
        let b = Var(FIRST_USER_VAR);
        let l = VarLayout::new(&[a, b, a], 4);
        assert_eq!(l.len(), 2);
        assert_eq!(l.slot(a), Some(0));
        assert_eq!(l.slot(b), Some(1));
        assert_eq!(l.slot(Var(FIRST_USER_VAR + 1)), None);
        assert_eq!(l.slot(Builtin::X.var()), None);
        assert_eq!(l.vars().collect::<Vec<_>>(), vec![(b, 1), (a, 0)]);
    }
}
