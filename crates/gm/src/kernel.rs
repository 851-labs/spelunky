//! The instance and event built-ins: `with`, `instance_create`/`destroy`, lookups, (de)activation and
//! `event_inherited` (runtime.md sections 6 and 8). OpenGMK `gml/kernel.rs` and `gml/runtime.rs`.
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

/// Resolves a target value, for variable access through it (`target.x`). An instance id takes the last instance
/// in draw order with it ([`InstanceList::last_by_id`](crate::list::InstanceList::last_by_id)), unlike `with`'s
/// first ([`World::with_iter`]). OpenGMK `gml/runtime.rs` (`get_target`).
pub(crate) fn target_of(w: &World, cx: &Ctx, target: &Value) -> R<Target> {
    Ok(match target.to_int()? {
        SELF | UNSPECIFIED => Target::One(Some(cx.this)),
        OTHER => Target::One(Some(cx.other)),
        ALL => Target::List(w.insts.iter_drawing()),
        NOONE => Target::One(None),
        GLOBAL | LOCAL => {
            return Err(Error::Unsupported(
                "global. or local. through an expression",
            ));
        }
        i if i >= FIRST_INSTANCE_ID => Target::One(w.insts.last_by_id(i)),
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
    /// descendants'; an instance id runs once on the first instance in draw order with it, if that one is
    /// active ([`InstanceList::by_id`](crate::list::InstanceList::by_id)); `noone` and other negative values run
    /// nothing. OpenGMK `gml/runtime.rs` (`Instruction::With`).
    ///
    /// The generated loop sets `cx.this` to each instance and `cx.other` to the outer `self`, and restores
    /// both afterwards.
    pub fn with_iter(&self, cx: &Ctx, target: &Value) -> R<WithIter> {
        Ok(WithIter(match target.to_int()? {
            // `with` treats every negative value it doesn't know as `noone`, `global` and `local` included.
            GLOBAL | LOCAL => Target::One(None),
            i if i >= FIRST_INSTANCE_ID => Target::One(self.insts.by_id(i)),
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
    /// (`instance_create`). Where its bounding box traps on Intel, GM8 is broken past repair
    /// ([`Error::CreationTrap`]).
    pub fn instance_create(&mut self, x: f64, y: f64, object: i32) -> R<i32> {
        let (o, info) = self
            .program
            .object_by_index(object)
            .ok_or(Error::NoSuchObject(object))?;
        self.last_instance_id += 1;
        let id = self.last_instance_id;
        let inst = Instance::new(id, x, y, o, info, self.program.layout(o).len());
        // GM8 has counted the id by then (Spelunky.exe 0x4aedba).
        if let Some(e) = self.assets.bbox_trap(&inst, x, y) {
            return Err(Error::CreationTrap(e));
        }
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

    /// `instance_exists(obj)`. An instance id finds the active instance with it, past a destroyed one with it
    /// ([`InstanceList::live_by_id`](crate::list::InstanceList::live_by_id)). OpenGMK `gml/kernel.rs`
    /// (`instance_exists`); note the object/id boundary is `<= 100000` here, unlike `with`.
    pub fn instance_exists(&self, cx: &Ctx, target: i32) -> bool {
        match target {
            SELF => self.insts.get(cx.this).is_active(),
            OTHER => self.insts.get(cx.other).is_active(),
            ALL => self.active().next().is_some(),
            o if o <= FIRST_INSTANCE_ID => {
                u32::try_from(o).is_ok_and(|o| self.insts.any_ancestor(ObjectId(o)))
            }
            id => self.insts.live_by_id(id).is_some(),
        }
    }

    /// `instance_number(obj)`. An instance id counts 1 if `instance_exists` finds it. OpenGMK `gml/kernel.rs`
    /// (`instance_number`).
    pub fn instance_number(&self, cx: &Ctx, target: i32) -> i32 {
        let active = |h: Inst| self.insts.get(h).is_active();
        match target {
            SELF => active(cx.this).into(),
            OTHER => active(cx.other).into(),
            ALL => self.active().count() as i32,
            o if o <= FIRST_INSTANCE_ID => {
                u32::try_from(o).map_or(0, |o| self.insts.count_ancestor(ObjectId(o)) as i32)
            }
            id => self.insts.live_by_id(id).is_some().into(),
        }
    }

    /// `instance_find(obj, n)`: the id of the `n`th instance, or `noone`. An instance id is its own 0th if
    /// `instance_exists` finds it. OpenGMK `gml/kernel.rs` (`instance_find`).
    pub fn instance_find(&self, target: i32, n: i32) -> i32 {
        let Ok(n) = usize::try_from(n) else {
            return NOONE;
        };
        let nth = |mut it: InstIter| std::iter::from_fn(|| it.next(&self.insts)).nth(n);
        let h = match target {
            ALL => nth(self.insts.iter_drawing()),
            o if o < 0 => None,
            o if o < FIRST_INSTANCE_ID => nth(self.insts.iter_ancestor(ObjectId(o as u32))),
            id if n == 0 => self.insts.live_by_id(id),
            _ => None,
        };
        h.map_or(NOONE, |h| self.insts.get(h).id)
    }

    // (De)activation. GM8's built-ins (Spelunky.exe 0x55c9e4 to 0x55cf78) only set or clear each instance's
    // deactivation flag, the active ones' (destroyed ones too) to deactivate and the deactivated ones' to
    // reactivate, then call one routine (0x4af00c, `InstanceList::settle`) that moves the instances whose flag
    // changed between its two arrays, to the end of the other, and rebuilds the per-object lists if any moved.
    // OpenGMK flipped a state in place and rebuilt the lists after every `self`, `other` and `all` (probe
    // `deactivation-order`, 851-3311).

    /// Flags the instances of one array that `pick` picks: those of the draw order to deactivate them, or with
    /// `activate`, those of the inactive array to reactivate them. Flagging moves nothing, so the array stays put.
    fn flag_where(&mut self, activate: bool, pick: impl Fn(&World, Inst) -> bool) {
        fn array(w: &World, activate: bool) -> &[Inst] {
            if activate {
                w.insts.inactive_order()
            } else {
                w.insts.draw_order()
            }
        }
        for k in 0..array(self, activate).len() {
            let h = array(self, activate)[k];
            if pick(self, h) {
                self.insts.flag(h, !activate);
            }
        }
    }

    /// `instance_deactivate_all(notme)`: every instance of the draw order, then not `self` with `notme`. OpenGMK
    /// `gml/kernel.rs` (`instance_deactivate_all`).
    pub fn instance_deactivate_all(&mut self, cx: &Ctx, notme: bool) {
        self.flag_where(false, |_, _| true);
        if notme {
            self.insts.flag(cx.this, false);
        }
        self.insts.settle(self.program);
    }

    /// `instance_deactivate_object(obj)`: an object (with descendants), `all`, `self`, `other` or an instance
    /// id: every instance of the draw order with it, destroyed ones too. OpenGMK `gml/kernel.rs`
    /// (`instance_deactivate_object`).
    pub fn instance_deactivate_object(&mut self, cx: &Ctx, target: i32) {
        self.set_active_object(cx, target, false);
    }

    /// `instance_activate_object(obj)`: every deactivated instance of the object (or a descendant) or with the
    /// id, `all`, `self` or `other`. OpenGMK `gml/kernel.rs` (`instance_activate_object`).
    pub fn instance_activate_object(&mut self, cx: &Ctx, target: i32) {
        self.set_active_object(cx, target, true);
    }

    fn set_active_object(&mut self, cx: &Ctx, target: i32, activate: bool) {
        match target {
            SELF => self.insts.flag(cx.this, !activate),
            OTHER => self.insts.flag(cx.other, !activate),
            ALL => self.flag_where(activate, |_, _| true),
            o if o < FIRST_INSTANCE_ID => {
                let Ok(o) = u32::try_from(o) else {
                    return;
                };
                let o = ObjectId(o);
                if activate {
                    self.flag_where(true, |w, h| w.program.is_a(w.insts.get(h).object, o));
                } else {
                    // The object's list (0x4aeee0), which holds the destroyed instances of the draw order too.
                    for k in 0..self.insts.ancestor_len(o) {
                        let h = self.insts.ancestor_at(o, k);
                        self.insts.flag(h, true);
                    }
                }
            }
            // Every one with the id (0x55cb08, 0x55cde8), the destroyed one of a shared id too (probe
            // `id-lookups`, 851-3310).
            id => self.flag_where(activate, |w, h| w.insts.get(h).id == id),
        }
        self.insts.settle(self.program);
    }

    /// `instance_deactivate_region(left, top, width, height, inside, notme)`: the instances of the draw order
    /// (destroyed ones too) outside the region, or with `inside` the ones not outside, then not `self` with
    /// `notme`, which so keeps its place. An instance is outside the region if its bounding box doesn't touch
    /// it, or, without a mask, if its position isn't in it. OpenGMK `gml/kernel.rs`
    /// (`instance_deactivate_region`).
    #[allow(clippy::too_many_arguments)]
    pub fn instance_deactivate_region(
        &mut self,
        cx: &Ctx,
        left: f64,
        top: f64,
        width: f64,
        height: f64,
        inside: bool,
        notme: bool,
    ) {
        self.flag_where(false, |w, h| {
            w.outside_region(h, left, top, width, height) != inside
        });
        if notme {
            self.insts.flag(cx.this, false);
        }
        self.insts.settle(self.program);
    }

    /// `instance_activate_region(left, top, width, height, inside)`. OpenGMK `gml/kernel.rs`
    /// (`instance_activate_region`).
    pub fn instance_activate_region(
        &mut self,
        left: f64,
        top: f64,
        width: f64,
        height: f64,
        inside: bool,
    ) {
        self.flag_where(true, |w, h| {
            w.outside_region(h, left, top, width, height) != inside
        });
        self.insts.settle(self.program);
    }

    /// `instance_activate_all()`.
    pub fn instance_activate_all(&mut self) {
        self.flag_where(true, |_, _| true);
        self.insts.settle(self.program);
    }

    fn outside_region(&self, h: Inst, left: f64, top: f64, width: f64, height: f64) -> bool {
        if self.has_mask(h) {
            let b = self.bbox(h);
            left > f64::from(b.right)
                || top > f64::from(b.bottom)
                || left + width < f64::from(b.left)
                || top + height < f64::from(b.top)
        } else {
            let i = self.insts.get(h);
            i.x < left || i.x > left + width || i.y < top || i.y > top + height
        }
    }
}
