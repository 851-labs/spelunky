//! The instance and event built-ins: `with`, `instance_create`/`destroy`, lookups and `event_inherited`
//! (runtime.md section 6). OpenGMK `gml/kernel.rs` and `gml/runtime.rs`.
//!
//! These take typed arguments; the generated `b::name(g, cx, &[Value])` wrappers convert them the way GM8
//! converts built-in arguments ([`Value::to_real`], [`Value::to_int`], [`Value::is_truthy`]).

use crate::ctx::Ctx;
use crate::error::{Error, R};
use crate::instance::Instance;
use crate::list::{Inst, InstIter};
use crate::program::{Event, Handler, ObjectId};
use crate::value::Value;
use crate::world::World;

/// `self`.
pub const SELF: i32 = -1;
/// `other`.
pub const OTHER: i32 = -2;
/// `all`.
pub const ALL: i32 = -3;
/// `noone`.
pub const NOONE: i32 = -4;
/// `global`.
pub const GLOBAL: i32 = -5;
/// `local`.
pub const LOCAL: i32 = -6;
/// What GM8's compiler uses for a bare name's owner; it means `self`.
pub const UNSPECIFIED: i32 = -7;
/// Values from here up are instance ids; below (and non-negative) are object ids.
pub const FIRST_INSTANCE_ID: i32 = 100_000;

/// What a `target.` or `with (target)` value names.
pub(crate) enum Target {
    /// One instance, or none (`noone`, an id that isn't active).
    One(Option<Inst>),
    /// Every instance an iterator yields (an object and its descendants, or `all`).
    List(InstIter),
}

/// Resolves a target value. OpenGMK `gml/runtime.rs` (`get_target`).
pub(crate) fn target_of(w: &World, cx: &Ctx, target: &Value) -> R<Target> {
    Ok(match target.to_int() {
        SELF | UNSPECIFIED => Target::One(Some(cx.this)),
        OTHER => Target::One(Some(cx.other)),
        ALL => Target::List(w.insts.iter_drawing()),
        NOONE => Target::One(None),
        GLOBAL | LOCAL => {
            return Err(Error::Unsupported(
                "global. or local. through an expression",
            ));
        }
        i if i >= FIRST_INSTANCE_ID => Target::One(w.insts.by_id(i)),
        i => match u32::try_from(i) {
            Ok(o) => Target::List(w.insts.iter_ancestor(ObjectId(o))),
            Err(_) => Target::One(None),
        },
    })
}

/// The instances a `with` statement visits, without borrowing the world, so the body can create and
/// destroy instances (runtime.md section 5).
pub struct WithIter(Target);

impl WithIter {
    #[inline]
    pub fn next(&mut self, g: &World) -> Option<Inst> {
        match &mut self.0 {
            Target::One(h) => h.take(),
            Target::List(it) => it.next(&g.insts),
        }
    }
}

impl World {
    /// Starts `with (target)`. `self` and `other` run the body once on that instance even if it was
    /// destroyed or deactivated; `all` walks the draw order; an object walks its instances and its
    /// descendants'; an instance id runs once if that instance is active; `noone` and other negative
    /// values run nothing. OpenGMK `gml/runtime.rs` (`Instruction::With`).
    ///
    /// The generated loop sets `cx.this` to each instance and `cx.other` to the outer `self`, and restores
    /// both afterwards.
    pub fn with_iter(&self, cx: &Ctx, target: &Value) -> R<WithIter> {
        Ok(WithIter(match target.to_int() {
            // `with` treats every negative value it doesn't know as `noone`, `global` and `local` included.
            GLOBAL | LOCAL => Target::One(None),
            _ => target_of(self, cx, target)?,
        }))
    }

    /// Runs a resolved handler with `self` and `other`, unless a room change is pending: then no event
    /// runs at all. OpenGMK `game/events.rs` (`run_instance_event`). The collision stage runs
    /// [`Program::collision_handler`](crate::Program::collision_handler)s through this.
    #[inline]
    pub fn run_handler(&mut self, this: Inst, other: Inst, h: Handler) -> R<()> {
        if self.pending.is_some() {
            return Ok(());
        }
        let mut cx = Ctx::event(this, other, h.event, h.object);
        (h.f)(self, &mut cx)
    }

    /// Runs instance `h`'s handler for a (non-collision) event, inherited as needed, with `other`.
    pub fn run_event(&mut self, h: Inst, other: Inst, e: Event) -> R<()> {
        let object = self.insts.get(h).object;
        match self.program.handler(object, e) {
            Some(&handler) => self.run_handler(h, other, handler),
            None => Ok(()),
        }
    }

    /// `event_inherited()`: runs the handler the parent of the *defining* object resolves for the same
    /// event, with the same `self` and `other`. OpenGMK `gml/kernel.rs` (`event_inherited`).
    pub fn event_inherited(&mut self, cx: &mut Ctx) -> R<()> {
        let parent = self
            .program
            .object(cx.event_object())
            .and_then(|o| o.parent);
        let Some(parent) = parent else {
            return Ok(());
        };
        // Collision handlers resolve through both object trees; everything else is in the parent's
        // prebuilt table.
        let h = match cx.current_event() {
            e @ Event::Collision(_) => self.program.resolve(parent, e),
            e => self.program.handler(parent, e).copied(),
        };
        if let Some(h) = h {
            self.run_handler(cx.this, cx.other, h)?;
        }
        Ok(())
    }

    /// `instance_create(x, y, obj)`: adds the instance at the end of every list and runs its create event
    /// immediately, with `other` = the new instance. Returns the new id. OpenGMK `gml/kernel.rs`
    /// (`instance_create`).
    pub fn instance_create(&mut self, x: f64, y: f64, object: i32) -> R<i32> {
        let (o, info) = self
            .program
            .object_by_index(object)
            .ok_or(Error::NoSuchObject(object))?;
        self.last_instance_id += 1;
        let id = self.last_instance_id;
        let inst = Instance::new(id, x, y, o, info, self.program.layout(o).len());
        let h = self.insts.insert(inst, self.program);
        self.run_event(h, h, Event::Create)?;
        Ok(id)
    }

    /// `instance_destroy()` on `cx.this`.
    pub fn instance_destroy(&mut self, cx: &Ctx) -> R<()> {
        self.destroy(cx.this)
    }

    /// Runs `h`'s destroy event (with `other` = itself), then marks it deleted. The calling code keeps
    /// running, and the instance stays readable until the end-of-step cleanup. Destroying an instance twice
    /// runs its destroy event twice, as in GM8. OpenGMK `gml/kernel.rs` (`instance_destroy`).
    pub fn destroy(&mut self, h: Inst) -> R<()> {
        self.run_event(h, h, Event::Destroy)?;
        self.insts.mark_deleted(h);
        Ok(())
    }

    /// `instance_exists(obj)`. OpenGMK `gml/kernel.rs` (`instance_exists`); note the object/id boundary is
    /// `<= 100000` here, unlike `with`.
    pub fn instance_exists(&self, cx: &Ctx, target: i32) -> bool {
        match target {
            SELF => self.insts.get(cx.this).is_active(),
            OTHER => self.insts.get(cx.other).is_active(),
            ALL => self.active().next().is_some(),
            o if o <= FIRST_INSTANCE_ID => {
                u32::try_from(o).is_ok_and(|o| self.insts.any_ancestor(ObjectId(o)))
            }
            id => self.insts.by_id(id).is_some(),
        }
    }

    /// `instance_number(obj)`. OpenGMK `gml/kernel.rs` (`instance_number`).
    pub fn instance_number(&self, cx: &Ctx, target: i32) -> i32 {
        let active = |h: Inst| self.insts.get(h).is_active();
        match target {
            SELF => active(cx.this).into(),
            OTHER => active(cx.other).into(),
            ALL => self.active().count() as i32,
            o if o <= FIRST_INSTANCE_ID => {
                u32::try_from(o).map_or(0, |o| self.insts.count_ancestor(ObjectId(o)) as i32)
            }
            id => self.insts.by_id(id).is_some().into(),
        }
    }

    /// `instance_find(obj, n)`: the id of the `n`th instance, or `noone`. OpenGMK `gml/kernel.rs`
    /// (`instance_find`).
    pub fn instance_find(&self, target: i32, n: i32) -> i32 {
        let Ok(n) = usize::try_from(n) else {
            return NOONE;
        };
        let nth = |mut it: InstIter| std::iter::from_fn(|| it.next(&self.insts)).nth(n);
        let h = match target {
            ALL => nth(self.insts.iter_drawing()),
            o if o < 0 => None,
            o if o < FIRST_INSTANCE_ID => nth(self.insts.iter_ancestor(ObjectId(o as u32))),
            id if n == 0 => self.insts.by_id(id),
            _ => None,
        };
        h.map_or(NOONE, |h| self.insts.get(h).id)
    }
}
