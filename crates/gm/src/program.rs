//! The static program: objects, their parents and children, resolved event handlers and holder lists,
//! and variable layouts (runtime.md sections 4 and 11).
//!
//! `gm` can't depend on the generated `game` crate, so generated code reaches the runtime through this
//! table. The generator emits the raw definitions (objects, the events each object defines, variable
//! names and layouts) into a [`ProgramBuilder`]; [`ProgramBuilder::build`] derives everything GM8 computes
//! at load time (identity lists, inherited handlers, holder lists). Doing that here rather than in the
//! generator keeps the inheritance rules in one tested place.

use crate::ctx::Ctx;
use crate::error::R;
use crate::var::{FIRST_USER_VAR, Var, VarLayout};
use crate::world::World;
use std::collections::{BTreeMap, BTreeSet};

/// An object id (GM's `object_index`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId(pub u32);

impl ObjectId {
    #[inline]
    pub(crate) fn idx(self) -> usize {
        self.0 as usize
    }
}

/// A generated event body: one object's actions for one event.
pub type EventFn = fn(&mut World, &mut Ctx) -> R<()>;

/// A GM8 event. The order of the variants (then their numbers) is the order holder tables iterate in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Event {
    Create,
    Destroy,
    /// Alarm 0 to 11.
    Alarm(u8),
    BeginStep,
    Step,
    EndStep,
    /// Collision with an object (and, through inheritance, its descendants).
    Collision(ObjectId),
    /// Key held: a virtual-key code, or 0 "no key" / 1 "any key".
    Keyboard(u8),
    Mouse(u8),
    /// Other events: 0 outside room, 1 intersect boundary, 2 game start, 3 game end, 4 room start, 5 room
    /// end, 7 animation end, 10-25 user events, 40-57 view events, ...
    Other(u8),
    Draw,
    KeyPress(u8),
    KeyRelease(u8),
}

impl Event {
    pub const OUTSIDE_ROOM: Event = Event::Other(0);
    pub const INTERSECT_BOUNDARY: Event = Event::Other(1);
    pub const ANIMATION_END: Event = Event::Other(7);

    /// The event GM stores as `(type, number)` in a `.gmk` (`gmk::Event`), or `None` for triggers and
    /// unknown types. OpenGMK `gml.rs` (`ev`).
    pub fn from_gm(kind: u32, number: u32) -> Option<Event> {
        let small = || u8::try_from(number).ok();
        Some(match kind {
            0 => Event::Create,
            1 => Event::Destroy,
            2 => Event::Alarm(small().filter(|&n| n < 12)?),
            3 => match number {
                0 => Event::Step,
                1 => Event::BeginStep,
                2 => Event::EndStep,
                _ => return None,
            },
            4 => Event::Collision(ObjectId(number)),
            5 => Event::Keyboard(small()?),
            6 => Event::Mouse(small()?),
            7 => Event::Other(small()?),
            8 => Event::Draw,
            9 => Event::KeyPress(small()?),
            10 => Event::KeyRelease(small()?),
            _ => return None,
        })
    }
}

/// A resolved event handler: the code to run, the object that defines it (which may be an ancestor of the
/// instance's object), and the event as that object defines it. For a collision event that's the
/// collision with the ancestor of the partner object it was found under. `event_inherited` needs both.
#[derive(Clone, Copy, Debug)]
pub struct Handler {
    pub f: EventFn,
    pub object: ObjectId,
    pub event: Event,
}

/// An object's properties: what a new instance starts with.
#[derive(Clone, Debug, PartialEq)]
pub struct ObjectInfo {
    pub name: String,
    pub parent: Option<ObjectId>,
    /// -1 for none.
    pub sprite: i32,
    /// -1 to use the sprite.
    pub mask: i32,
    pub solid: bool,
    pub visible: bool,
    pub persistent: bool,
    pub depth: f64,
}

impl ObjectInfo {
    /// An object with GM's defaults: no sprite, visible, depth 0.
    pub fn new(name: impl Into<String>) -> ObjectInfo {
        ObjectInfo {
            name: name.into(),
            parent: None,
            sprite: -1,
            mask: -1,
            solid: false,
            visible: true,
            persistent: false,
            depth: 0.0,
        }
    }
}

/// Collects the program's definitions; [`build`](Self::build) derives the rest.
#[derive(Default)]
pub struct ProgramBuilder {
    objects: Vec<Option<ObjectInfo>>,
    events: Vec<BTreeMap<Event, EventFn>>,
    layouts: Vec<Vec<Var>>,
    var_names: Vec<String>,
    global_names: Vec<String>,
}

impl ProgramBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Defines object `id`. Ids may have gaps (GM leaves deleted objects' slots empty).
    pub fn object(&mut self, id: ObjectId, info: ObjectInfo) -> &mut Self {
        let i = id.idx();
        if self.objects.len() <= i {
            self.objects.resize(i + 1, None);
            self.events.resize(i + 1, BTreeMap::new());
            self.layouts.resize(i + 1, Vec::new());
        }
        self.objects[i] = Some(info);
        self
    }

    /// Defines the next object id and returns it.
    pub fn push_object(&mut self, info: ObjectInfo) -> ObjectId {
        let id = ObjectId(self.objects.len() as u32);
        self.object(id, info);
        id
    }

    /// Defines `object`'s handler for `event`.
    pub fn event(&mut self, object: ObjectId, event: Event, f: EventFn) -> &mut Self {
        self.events[object.idx()].insert(event, f);
        self
    }

    /// Interns a user variable name and returns its id. Names must be unique.
    pub fn var(&mut self, name: impl Into<String>) -> Var {
        let name = name.into();
        debug_assert!(
            !self.var_names.contains(&name),
            "variable {name} interned twice"
        );
        self.var_names.push(name);
        Var(FIRST_USER_VAR + self.var_names.len() as u16 - 1)
    }

    /// Interns a global variable name and returns its id.
    pub fn global(&mut self, name: impl Into<String>) -> crate::GVar {
        self.global_names.push(name.into());
        crate::GVar(self.global_names.len() as u16 - 1)
    }

    /// Declares user variables `object`'s instances may hold. Children inherit their ancestors' layouts.
    pub fn layout(&mut self, object: ObjectId, vars: &[Var]) -> &mut Self {
        self.layouts[object.idx()].extend_from_slice(vars);
        self
    }

    /// Derives identity lists, resolved handlers and holder lists. Panics on a parent that doesn't exist
    /// or a parent cycle, which GM8 also refuses to load (OpenGMK `game.rs`, "Populate identity lists").
    pub fn build(self) -> Program {
        let n = self.objects.len();
        let exists = |o: ObjectId| self.objects.get(o.idx()).is_some_and(Option::is_some);

        // parents[o]: o, its parent, its grandparent, ... (the order handler lookup walks).
        let mut parents = vec![Vec::new(); n];
        let mut children = vec![Vec::new(); n];
        for (i, info) in self.objects.iter().enumerate() {
            let Some(info) = info else { continue };
            let mut chain = vec![ObjectId(i as u32)];
            let mut p = info.parent;
            while let Some(o) = p {
                assert!(exists(o), "object {i} has a missing ancestor {}", o.0);
                assert!(chain.len() <= n, "object {i} has a parent cycle");
                chain.push(o);
                p = self.objects[o.idx()].as_ref().and_then(|x| x.parent);
            }
            for &a in &chain {
                children[a.idx()].push(ObjectId(i as u32));
            }
            parents[i] = chain;
        }

        let mut program = Program {
            objects: self.objects,
            parents,
            children,
            defined: self.events,
            handlers: Vec::new(),
            holders: BTreeMap::new(),
            collision_pairs: BTreeMap::new(),
            collision: BTreeMap::new(),
            layouts: Vec::new(),
            var_names: self.var_names,
            global_names: self.global_names,
        };

        // Resolved non-collision handlers: walk up from each object to the first definer.
        program.handlers = (0..n)
            .map(|i| {
                let mut table = BTreeMap::new();
                for &a in &program.parents[i] {
                    for &e in program.defined[a.idx()].keys() {
                        if !matches!(e, Event::Collision(_)) && !table.contains_key(&e) {
                            let h = program.resolve(ObjectId(i as u32), e);
                            table.insert(e, h.expect("an ancestor defines it"));
                        }
                    }
                }
                table
            })
            .collect();

        // Holders: for each event, the children of every object defining it, ascending, without
        // duplicates. OpenGMK `game.rs` (`fill_event_holders`).
        let mut holders: BTreeMap<Event, BTreeSet<ObjectId>> = BTreeMap::new();
        let mut pairs: BTreeMap<ObjectId, BTreeSet<ObjectId>> = BTreeMap::new();
        for (i, events) in program.defined.iter().enumerate() {
            for &e in events.keys() {
                let kids = &program.children[i];
                match e {
                    // Collision holders pair the definer's children with the target's children, each
                    // unordered pair once under its lower id. That is what fill_event_holders' swap
                    // and `retain(x >= sub)` steps leave, because children lists are closed under descent.
                    Event::Collision(t) if program.object(t).is_some() => {
                        for &a in kids {
                            for &b in &program.children[t.idx()] {
                                pairs.entry(a.min(b)).or_default().insert(a.max(b));
                            }
                        }
                    }
                    Event::Collision(_) => {}
                    _ => holders.entry(e).or_default().extend(kids),
                }
            }
        }
        program.holders = holders
            .into_iter()
            .map(|(e, objs)| {
                let list = objs
                    .into_iter()
                    .map(|o| (o, program.handlers[o.idx()][&e]))
                    .collect();
                (e, list)
            })
            .collect();
        for (&a, bs) in &pairs {
            for &b in bs {
                for (x, y) in [(a, b), (b, a)] {
                    if let Some(h) = program.resolve(x, Event::Collision(y)) {
                        program.collision.insert((x, y), h);
                    }
                }
            }
        }
        program.collision_pairs = pairs
            .into_iter()
            .map(|(a, bs)| (a, bs.into_iter().collect()))
            .collect();

        // Layouts: each object's own variables, then its ancestors'.
        let user_vars = program.var_names.len();
        program.layouts = (0..n)
            .map(|i| {
                let vars: Vec<Var> = program.parents[i]
                    .iter()
                    .flat_map(|a| self.layouts[a.idx()].iter().copied())
                    .collect();
                VarLayout::new(&vars, user_vars)
            })
            .collect();
        program
    }
}

/// The static program table (runtime.md section 11). Built once; worlds hold a `&'static` to it.
#[derive(Debug)]
pub struct Program {
    objects: Vec<Option<ObjectInfo>>,
    parents: Vec<Vec<ObjectId>>,
    children: Vec<Vec<ObjectId>>,
    defined: Vec<BTreeMap<Event, EventFn>>,
    handlers: Vec<BTreeMap<Event, Handler>>,
    holders: BTreeMap<Event, Vec<(ObjectId, Handler)>>,
    collision_pairs: BTreeMap<ObjectId, Vec<ObjectId>>,
    collision: BTreeMap<(ObjectId, ObjectId), Handler>,
    layouts: Vec<VarLayout>,
    var_names: Vec<String>,
    global_names: Vec<String>,
}

impl Program {
    /// The object with this id, if it exists.
    #[inline]
    pub fn object(&self, o: ObjectId) -> Option<&ObjectInfo> {
        self.objects.get(o.idx()).and_then(Option::as_ref)
    }

    /// The object with this GML value's id (`object_index`), if it exists.
    pub fn object_by_index(&self, o: i32) -> Option<(ObjectId, &ObjectInfo)> {
        let id = ObjectId(u32::try_from(o).ok()?);
        Some((id, self.object(id)?))
    }

    /// One more than the highest object id.
    pub fn object_slots(&self) -> usize {
        self.objects.len()
    }

    /// `o` and its ancestors, nearest first.
    #[inline]
    pub fn parents(&self, o: ObjectId) -> &[ObjectId] {
        &self.parents[o.idx()]
    }

    /// `o` and all its descendants, ascending.
    pub fn children(&self, o: ObjectId) -> &[ObjectId] {
        &self.children[o.idx()]
    }

    /// Whether `o` is `ancestor` or one of its descendants.
    pub fn is_a(&self, o: ObjectId, ancestor: ObjectId) -> bool {
        self.parents(o).contains(&ancestor)
    }

    /// The handler an instance of `o` runs for a non-collision event, inherited from the nearest ancestor
    /// that defines it.
    #[inline]
    pub fn handler(&self, o: ObjectId, e: Event) -> Option<&Handler> {
        self.handlers.get(o.idx())?.get(&e)
    }

    /// The handler an instance of `a` runs when it meets an instance of `b`, for objects paired in
    /// [`collision_pairs`](Self::collision_pairs).
    pub fn collision_handler(&self, a: ObjectId, b: ObjectId) -> Option<&Handler> {
        self.collision.get(&(a, b))
    }

    /// Handler lookup as GM8 does it at run time, starting from `start` (which need not be the instance's
    /// object: `event_inherited` starts at the parent of the defining object). A collision event with `t`
    /// tries `start` and its ancestors for `t`, then all of them again for `t`'s parent, and so on.
    /// OpenGMK `game/events.rs` (`run_instance_event`).
    pub fn resolve(&self, start: ObjectId, e: Event) -> Option<Handler> {
        let found = |t: Event| {
            self.parents.get(start.idx())?.iter().find_map(|&o| {
                self.defined[o.idx()].get(&t).map(|&f| Handler {
                    f,
                    object: o,
                    event: t,
                })
            })
        };
        match e {
            Event::Collision(t) => self
                .parents
                .get(t.idx())?
                .iter()
                .find_map(|&t| found(Event::Collision(t))),
            _ => found(e),
        }
    }

    /// The objects whose instances receive `e`, ascending, each with its resolved handler. Stages run
    /// `for (obj, handler) in holders(e) { for inst in instances of exactly obj { ... } }`, so within a
    /// stage instances run grouped by object id.
    #[inline]
    pub fn holders(&self, e: Event) -> &[(ObjectId, Handler)] {
        self.holders.get(&e).map_or(&[], Vec::as_slice)
    }

    /// Holders of `e` and the following events up to and including `last`, in event order (key events by
    /// key code).
    pub fn holders_range(
        &self,
        first: Event,
        last: Event,
    ) -> impl Iterator<Item = (Event, &[(ObjectId, Handler)])> {
        self.holders
            .range(first..=last)
            .map(|(e, l)| (*e, l.as_slice()))
    }

    /// Collision pairs: for each object `a` with a collision event on either side, the objects `b >= a` its
    /// instances are tested against. For the collision stage (851-3127).
    pub fn collision_pairs(&self) -> impl Iterator<Item = (ObjectId, &[ObjectId])> {
        self.collision_pairs.iter().map(|(a, b)| (*a, b.as_slice()))
    }

    /// The user-variable layout of `o`'s instances.
    #[inline]
    pub fn layout(&self, o: ObjectId) -> &VarLayout {
        &self.layouts[o.idx()]
    }

    /// A variable's GML name, built-in or user.
    pub fn var_name(&self, v: Var) -> &str {
        match crate::var::Builtin::of(v) {
            Some(b) => b.name(),
            None => self
                .var_names
                .get(usize::from(v.0 - FIRST_USER_VAR))
                .map_or("<unknown>", String::as_str),
        }
    }

    /// How many user variables the program interns.
    pub fn user_var_count(&self) -> usize {
        self.var_names.len()
    }

    /// A global's name.
    pub fn global_name(&self, v: crate::GVar) -> &str {
        self.global_names
            .get(usize::from(v.0))
            .map_or("<unknown>", String::as_str)
    }

    /// How many globals the program interns.
    pub fn global_count(&self) -> usize {
        self.global_names.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nop(_: &mut World, _: &mut Ctx) -> R<()> {
        Ok(())
    }
    fn other(_: &mut World, _: &mut Ctx) -> R<()> {
        Ok(())
    }

    fn child(name: &str, parent: ObjectId) -> ObjectInfo {
        ObjectInfo {
            parent: Some(parent),
            ..ObjectInfo::new(name)
        }
    }

    #[test]
    fn handlers_are_inherited_from_the_nearest_definer() {
        let mut b = ProgramBuilder::new();
        let base = b.push_object(ObjectInfo::new("base"));
        let mid = b.push_object(child("mid", base));
        let leaf = b.push_object(child("leaf", mid));
        b.event(base, Event::Step, nop);
        b.event(mid, Event::Step, other);
        b.event(base, Event::Create, nop);
        let p = b.build();
        assert_eq!(p.parents(leaf), &[leaf, mid, base]);
        assert_eq!(p.children(base), &[base, mid, leaf]);
        assert_eq!(p.handler(leaf, Event::Step).unwrap().object, mid);
        assert_eq!(p.handler(leaf, Event::Create).unwrap().object, base);
        assert!(p.handler(leaf, Event::Draw).is_none());
        let holders: Vec<_> = p.holders(Event::Step).iter().map(|(o, _)| *o).collect();
        assert_eq!(holders, vec![base, mid, leaf]);
        assert_eq!(p.resolve(base, Event::Step).unwrap().object, base);
    }

    #[test]
    fn collision_lookup_walks_both_trees() {
        let mut b = ProgramBuilder::new();
        let enemy = b.push_object(ObjectInfo::new("enemy"));
        let snake = b.push_object(child("snake", enemy));
        let item = b.push_object(ObjectInfo::new("item"));
        let rock = b.push_object(child("rock", item));
        b.event(item, Event::Collision(enemy), nop);
        let p = b.build();
        // A rock meeting a snake finds item's handler for enemy, the snake's parent.
        let h = p.collision_handler(rock, snake).unwrap();
        assert_eq!((h.object, h.event), (item, Event::Collision(enemy)));
        assert!(p.collision_handler(snake, rock).is_none());
        let pairs: Vec<_> = p.collision_pairs().map(|(a, b)| (a, b.to_vec())).collect();
        assert_eq!(
            pairs,
            vec![(enemy, vec![item, rock]), (snake, vec![item, rock])]
        );
    }

    #[test]
    fn layouts_include_ancestors() {
        let mut b = ProgramBuilder::new();
        let base = b.push_object(ObjectInfo::new("base"));
        let leaf = b.push_object(child("leaf", base));
        let hp = b.var("hp");
        let kind = b.var("kind");
        b.layout(base, &[hp]).layout(leaf, &[kind]);
        let p = b.build();
        assert_eq!(p.layout(base).len(), 1);
        assert_eq!(p.layout(leaf).slot(kind), Some(0));
        assert_eq!(p.layout(leaf).slot(hp), Some(1));
        assert_eq!(p.var_name(hp), "hp");
        assert_eq!(p.var_name(crate::var::Builtin::Depth.var()), "depth");
    }

    #[test]
    fn events_convert_from_gmk_numbers() {
        assert_eq!(Event::from_gm(3, 1), Some(Event::BeginStep));
        assert_eq!(Event::from_gm(2, 11), Some(Event::Alarm(11)));
        assert_eq!(Event::from_gm(2, 12), None);
        assert_eq!(Event::from_gm(4, 77), Some(Event::Collision(ObjectId(77))));
        assert_eq!(Event::from_gm(7, 7), Some(Event::ANIMATION_END));
        assert_eq!(Event::from_gm(11, 0), None);
    }
}
