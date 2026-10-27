//! The instance list: a slab of instances plus the order lists GM8 iterates (runtime.md section 5). The
//! structure copies OpenGMK `instancelist.rs` so iteration order, and what happens when the lists change
//! mid-loop, come out the same.

use crate::index::SpatialIndex;
use crate::instance::{Bbox, Instance, State};
use crate::program::{ObjectId, Program};
use crate::var::FIRST_USER_VAR;
use std::cell::{Ref, RefCell};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};

/// An instance handle: a slot in the slab. GML never sees these; it sees instance ids.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct Inst(pub u32);

impl Inst {
    #[inline]
    fn idx(self) -> usize {
        self.0 as usize
    }
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct InstanceList {
    slab: Vec<Option<Instance>>,
    /// Free slab slots, lowest first. GML never sees a handle, but the slab order is the order persistent
    /// instances leave a room in ([`take_persistent`](Self::take_persistent)), so slots are reused the way
    /// OpenGMK's chunk list reuses them: the lowest vacant one first (`instancelist.rs`, `ChunkList::insert`).
    free: BinaryHeap<Reverse<u32>>,
    /// Every instance of the room. Creation appends; drawing sorts it by depth.
    draw_order: Vec<Inst>,
    /// Active instances of exactly each object (plus instances deleted since the last rebuild).
    by_object: Vec<Vec<Inst>>,
    /// Active instances of each object or any of its descendants.
    by_ancestor: Vec<Vec<Inst>>,
    /// Instance id to handle. Ids are unique but for `shadowed`, so this finds what GM8's scan of the draw
    /// order finds.
    ids: BTreeMap<i32, Inst>,
    /// Instances whose id `ids` gives another (destroyed) instance: a persistent instance that arrived in a
    /// room placing one with its id, until the cleanup removes that one. Almost always empty.
    shadowed: Vec<Inst>,
    /// How many instances are marked deleted, so the cleanup costs nothing on frames without any.
    deleted: usize,
    /// How many instances are deactivated, for `instance_count`.
    inactive: usize,
    /// By slot: the instance's position in the order every per-object list follows (see
    /// [`list_rank`](Self::list_rank)).
    list_rank: Vec<u32>,
    /// The rank the next created instance gets.
    next_list_rank: u32,
    /// By slot: the instance's position in the draw order.
    draw_pos: Vec<u32>,
    /// Every instance's bounding box in a grid, for collision queries. A cache: cloning drops it. In a
    /// `RefCell` so that queries, which only read the world, can bring it up to date
    /// ([`flush_index`](Self::flush_index)); writes queue moved instances through `get_mut`, which needs no
    /// runtime borrow.
    #[serde(skip)]
    index: RefCell<SpatialIndex>,
}

/// Which list an [`InstIter`] walks.
#[derive(Clone, Copy, Debug)]
enum Source {
    /// The draw order, active instances, never past the count taken at the start.
    Drawing,
    /// The draw order, deactivated instances, never past the count taken at the start.
    Inactive,
    /// Instances of exactly this object.
    Object(ObjectId),
    /// Instances of this object or its descendants.
    Ancestor(ObjectId),
}

/// A non-borrowing iterator over one of the instance lists. It holds a position and the list's length
/// when it started, and on each step scans the *current* list from the position, so the list can change
/// between steps. OpenGMK `instancelist.rs` (`ILIterDrawOrder`, `ObjectIter`, `IdentityIter`,
/// `nb_il_iter`):
///
/// - Draw-order iterators never go past the starting length, so instances created during the loop are
///   never visited.
/// - Object iterators stop once the position reaches the starting length, but each step scans to the end
///   of the current list. An instance appended during the loop is visited if inactive or deleted entries
///   before it are skipped in the same step.
/// - If a (de)activation rebuilds the list mid-loop, iteration continues at the same index of the new
///   list, which can skip or repeat instances. GM8 does the same.
#[derive(Clone, Copy, Debug)]
pub struct InstIter {
    source: Source,
    pos: usize,
    count: usize,
}

impl InstIter {
    /// Moves to a position of the list. Skipping entries this way is only the same as visiting them if
    /// visiting them would do nothing (the collision stage uses it to skip instances that can't collide).
    pub(crate) fn seek(&mut self, pos: usize) {
        self.pos = pos;
    }

    pub(crate) fn next(&mut self, list: &InstanceList) -> Option<Inst> {
        let (vec, want, limit) = match self.source {
            Source::Drawing => (&list.draw_order, State::Active, self.count),
            Source::Inactive => (&list.draw_order, State::Inactive, self.count),
            Source::Object(o) | Source::Ancestor(o) => {
                if self.pos >= self.count {
                    return None;
                }
                let lists = match self.source {
                    Source::Object(_) => &list.by_object,
                    _ => &list.by_ancestor,
                };
                let vec = lists.get(o.0 as usize)?;
                (vec, State::Active, vec.len())
            }
        };
        let end = limit.min(vec.len());
        let slice = vec.get(self.pos..end)?;
        let off = slice.iter().position(|&h| list.get(h).state == want)?;
        self.pos += off + 1;
        Some(slice[off])
    }
}

impl InstanceList {
    #[inline]
    pub(crate) fn get(&self, h: Inst) -> &Instance {
        self.slab[h.idx()].as_ref().expect("live instance handle")
    }

    /// An instance to change. It is queued for the spatial index, so this is the only way to change
    /// anything its bounding box depends on (`x`, `y`, `sprite_index`, `mask_index`, scale, angle).
    #[inline]
    pub(crate) fn get_mut(&mut self, h: Inst) -> &mut Instance {
        self.index.get_mut().moved(h);
        self.get_mut_still(h)
    }

    /// An instance to change without queueing it for the spatial index: only for the frame loop's hot
    /// loops, which write `xprevious`, alarms, speeds and `image_index`, and this list's own state changes.
    /// Everything else uses [`get_mut`](Self::get_mut); queueing twice costs nothing.
    #[inline]
    pub(crate) fn get_mut_still(&mut self, h: Inst) -> &mut Instance {
        self.slab[h.idx()].as_mut().expect("live instance handle")
    }

    /// An instance's rank in the order of the per-object lists: each `by_object` and `by_ancestor` list
    /// is in ascending rank. A rebuild ([`refresh`](Self::refresh)) fills every list from the draw order,
    /// so it sets every instance's rank to its draw position; [`insert`](Self::insert) appends to the
    /// lists, so a new instance ranks after all others. So among the active instances of an object (all of
    /// which are in its lists: activation always rebuilds), the lowest rank is the first a loop visits.
    #[inline]
    pub(crate) fn list_rank(&self, h: Inst) -> u32 {
        self.list_rank[h.idx()]
    }

    /// Where an instance of exactly `o` is in that object's list (`by_object`), found by its rank.
    pub(crate) fn object_position(&self, o: ObjectId, h: Inst) -> usize {
        let rank = self.list_rank(h);
        let list = &self.by_object[o.idx()];
        let pos = list.partition_point(|&x| self.list_rank(x) < rank);
        debug_assert_eq!(list.get(pos), Some(&h), "ranks order the per-object lists");
        pos
    }

    /// How many entries the list of `o` and its descendants (`by_ancestor`) has, active or not.
    pub(crate) fn ancestor_len(&self, o: ObjectId) -> usize {
        self.by_ancestor.get(o.idx()).map_or(0, Vec::len)
    }

    /// How many entries `o`'s list (`by_object`) has, active or not.
    pub(crate) fn object_len(&self, o: ObjectId) -> usize {
        self.by_object.get(o.idx()).map_or(0, Vec::len)
    }

    /// An instance's position in the draw order, which `all` walks.
    #[inline]
    pub(crate) fn draw_rank(&self, h: Inst) -> u32 {
        self.draw_pos[h.idx()]
    }

    /// Brings the spatial index up to date and returns it. `bbox` is GM8's box of an instance.
    pub(crate) fn flush_index(
        &self,
        room: (i32, i32),
        bbox: impl Fn(&Instance) -> Bbox,
    ) -> Ref<'_, SpatialIndex> {
        self.index.borrow_mut().flush(&self.slab, room, bbox);
        self.index.borrow()
    }

    /// Sizes the per-object lists for a program.
    pub(crate) fn with_objects(n: usize) -> Self {
        InstanceList {
            by_object: vec![Vec::new(); n],
            by_ancestor: vec![Vec::new(); n],
            ..Self::default()
        }
    }

    /// Whether the list is one the runtime could have built for `program`: what a decoded savestate must
    /// satisfy before anything indexes with it.
    ///
    /// - Every handle is a live slot, and the free list holds exactly the empty ones.
    /// - The draw order holds every live slot once, and `draw_pos` is each one's position in it.
    /// - `ids` maps each live instance's id to it.
    /// - Each object's lists hold instances of it (`by_object`) or of it and its descendants
    ///   (`by_ancestor`), in ascending `list_rank`, every rank below `next_list_rank`.
    /// - The deleted and inactive counts are right, and each instance has its object's variable layout and
    ///   only knows user variables the program has.
    pub(crate) fn is_consistent(&self, program: &Program) -> bool {
        let slot = |h: Inst| self.slab.get(h.idx()).and_then(Option::as_ref);
        let objects = program.object_slots();
        let slots = self.slab.len();
        let live: Vec<Inst> = (0..slots as u32)
            .map(Inst)
            .filter(|&h| slot(h).is_some())
            .collect();
        let count = |state: State| {
            live.iter()
                .filter(|&&h| slot(h).unwrap().state == state)
                .count()
        };
        let user_vars = FIRST_USER_VAR as usize..FIRST_USER_VAR as usize + program.user_var_count();
        let sizes = self.by_object.len() == objects
            && self.by_ancestor.len() == objects
            && self.list_rank.len() == slots
            && self.draw_pos.len() == slots
            && self.free.len() == slots - live.len()
            && self
                .free
                .iter()
                .all(|&Reverse(f)| matches!(self.slab.get(f as usize), Some(None)));
        // Every instance's object exists before the lists below look up its parents.
        let instances = sizes
            && live.iter().all(|&h| {
                let i = slot(h).unwrap();
                self.list_rank[h.idx()] < self.next_list_rank
                    && program.object(i.object).is_some()
                    && i.vars.len() == program.layout(i.object).len()
                    && i.extra
                        .iter()
                        .all(|(v, _)| user_vars.contains(&usize::from(v.0)))
            });
        if !instances {
            return false;
        }
        let drawing = self.draw_order.len() == live.len()
            && self
                .draw_order
                .iter()
                .enumerate()
                .all(|(pos, &h)| slot(h).is_some() && self.draw_pos[h.idx()] as usize == pos);
        let ids = self.ids.len() == live.len()
            && self
                .ids
                .iter()
                .all(|(&id, &h)| slot(h).is_some_and(|i| i.id == id));
        let in_order = |list: &Vec<Inst>, member: &dyn Fn(ObjectId) -> bool| {
            list.iter()
                .all(|&h| slot(h).is_some_and(|i| member(i.object)))
                && list
                    .windows(2)
                    .all(|w| self.list_rank[w[0].idx()] < self.list_rank[w[1].idx()])
        };
        let lists = (0..objects).all(|o| {
            let o = ObjectId(o as u32);
            in_order(&self.by_object[o.idx()], &|x| x == o)
                && in_order(&self.by_ancestor[o.idx()], &|x| program.is_a(x, o))
        });
        drawing
            && ids
            && lists
            && self.deleted == count(State::Deleted)
            && self.inactive == count(State::Inactive)
    }

    /// Adds an instance at the end of every list it belongs to. OpenGMK `instancelist.rs` (`insert`).
    pub(crate) fn insert(&mut self, inst: Instance, program: &Program) -> Inst {
        let (id, object) = (inst.id, inst.object);
        let h = match self.free.pop() {
            Some(Reverse(i)) => {
                self.slab[i as usize] = Some(inst);
                Inst(i)
            }
            None => {
                self.slab.push(Some(inst));
                self.list_rank.push(0);
                self.draw_pos.push(0);
                Inst(self.slab.len() as u32 - 1)
            }
        };
        self.list_rank[h.idx()] = self.next_list_rank;
        self.next_list_rank += 1;
        self.draw_pos[h.idx()] = self.draw_order.len() as u32;
        self.index.get_mut().moved(h);
        self.draw_order.push(h);
        self.by_object[object.idx()].push(h);
        for &a in program.parents(object) {
            self.by_ancestor[a.idx()].push(h);
        }
        match self.get(h).state {
            State::Active => {}
            State::Inactive => self.inactive += 1,
            State::Deleted => self.deleted += 1,
        }
        // Ids are unique except when a persistent instance arrives in a room that placed an instance with
        // its id; the caller has marked that one deleted.
        match self.ids.get(&id) {
            None => {
                self.ids.insert(id, h);
            }
            Some(&o) => {
                debug_assert_eq!(self.get(o).state, State::Deleted, "instance id {id} reused");
                self.shadowed.push(h);
            }
        }
        h
    }

    /// Takes every persistent instance out, in slab order, whatever its state (deactivated and destroyed
    /// ones too), for the next room. OpenGMK `instancelist.rs` (`remove_as_vec`, which walks the chunks in
    /// slot order), called from `game.rs` `load_room`. The list itself is about to be dropped, so only the
    /// slab is kept consistent.
    pub(crate) fn take_persistent(&mut self) -> Vec<Instance> {
        self.slab
            .iter_mut()
            .filter(|s| s.as_ref().is_some_and(|i| i.persistent))
            .map(|s| s.take().expect("checked"))
            .collect()
    }

    /// The active instance with this id. Deactivated and deleted instances behave as if they don't exist.
    /// OpenGMK `instancelist.rs` (`get_by_instid`).
    /// With two instances of one id (see `shadowed`), the first in draw order decides, active or not, as
    /// in GM8.
    pub(crate) fn by_id(&self, id: i32) -> Option<Inst> {
        let mut h = *self.ids.get(&id)?;
        for &s in &self.shadowed {
            if self.get(s).id == id && self.draw_rank(s) < self.draw_rank(h) {
                h = s;
            }
        }
        self.get(h).is_active().then_some(h)
    }

    /// The active instance with this id that a view follows, past a destroyed one with it. GM8's view following
    /// (Spelunky.exe 0x4beca4) scans the room's array of active instances (deactivated ones are in another) for the
    /// first with the id that isn't marked destroyed, where [`by_id`](Self::by_id) lets the first with the id
    /// decide. They differ while a persistent instance shares its id with one its room placed (see `shadowed`):
    /// the room load's draw follows the persistent one on GM8 (probe `view-follow-id`, 851-3309). Only one
    /// instance with an id is ever not destroyed, so the order doesn't matter.
    pub(crate) fn follow_by_id(&self, id: i32) -> Option<Inst> {
        let h = *self.ids.get(&id)?;
        std::iter::once(h)
            .chain(self.shadowed.iter().copied())
            .find(|&s| self.get(s).id == id && self.get(s).is_active())
    }

    pub(crate) fn iter_drawing(&self) -> InstIter {
        InstIter {
            source: Source::Drawing,
            pos: 0,
            count: self.draw_order.len(),
        }
    }

    pub(crate) fn iter_inactive(&self) -> InstIter {
        InstIter {
            source: Source::Inactive,
            pos: 0,
            count: self.draw_order.len(),
        }
    }

    pub(crate) fn iter_object(&self, o: ObjectId) -> InstIter {
        InstIter {
            source: Source::Object(o),
            pos: 0,
            count: self.by_object.get(o.idx()).map_or(0, Vec::len),
        }
    }

    pub(crate) fn iter_ancestor(&self, o: ObjectId) -> InstIter {
        InstIter {
            source: Source::Ancestor(o),
            pos: 0,
            count: self.by_ancestor.get(o.idx()).map_or(0, Vec::len),
        }
    }

    /// Whether `o` or a descendant has an active instance.
    pub(crate) fn any_ancestor(&self, o: ObjectId) -> bool {
        self.by_ancestor
            .get(o.idx())
            .is_some_and(|v| v.iter().any(|&h| self.get(h).is_active()))
    }

    /// Instances that aren't deactivated, deleted ones included: GM's `instance_count`. OpenGMK
    /// `instancelist.rs` (`count_all`).
    pub(crate) fn count_all(&self) -> usize {
        self.draw_order.len() - self.inactive
    }

    /// Active instances of `o` or its descendants. OpenGMK `instancelist.rs` (`count`).
    pub(crate) fn count_ancestor(&self, o: ObjectId) -> usize {
        self.by_ancestor.get(o.idx()).map_or(0, |v| {
            v.iter().filter(|&&h| self.get(h).is_active()).count()
        })
    }

    /// Every handle in draw order, whatever its state.
    pub(crate) fn draw_order(&self) -> &[Inst] {
        &self.draw_order
    }

    /// Changes an instance's state between active and inactive, if it's in `from`. Destroying goes
    /// through [`mark_deleted`](Self::mark_deleted), which keeps the cleanup's count.
    pub(crate) fn set_state(&mut self, h: Inst, from: State, to: State) -> bool {
        assert!(
            from != State::Deleted && to != State::Deleted,
            "set_state only (de)activates"
        );
        let i = self.get_mut_still(h);
        let changed = i.state == from;
        if changed {
            i.state = to;
            if to == State::Inactive {
                self.inactive += 1;
            } else {
                self.inactive -= 1;
            }
        }
        changed
    }

    /// Marks an instance destroyed; it stays readable until [`remove_deleted`](Self::remove_deleted).
    pub(crate) fn mark_deleted(&mut self, h: Inst) {
        let i = self.get_mut_still(h);
        let was = std::mem::replace(&mut i.state, State::Deleted);
        if was != State::Deleted {
            self.deleted += 1;
        }
        if was == State::Inactive {
            self.inactive -= 1;
        }
    }

    /// Rebuilds the per-object lists from the draw order, active instances only, reusing their
    /// allocations. OpenGMK `instancelist.rs` (`refresh_maps`).
    pub(crate) fn refresh(&mut self, program: &Program) {
        self.by_object.iter_mut().for_each(Vec::clear);
        self.by_ancestor.iter_mut().for_each(Vec::clear);
        for (pos, &h) in self.draw_order.iter().enumerate() {
            self.list_rank[h.idx()] = pos as u32;
        }
        self.next_list_rank = self.draw_order.len() as u32;
        for &h in &self.draw_order {
            let Some(inst) = &self.slab[h.idx()] else {
                continue;
            };
            if inst.state != State::Active {
                continue;
            }
            self.by_object[inst.object.idx()].push(h);
            for &a in program.parents(inst.object) {
                self.by_ancestor[a.idx()].push(h);
            }
        }
    }

    /// Removes deleted instances without reordering the rest, then rebuilds the per-object lists if any
    /// went. OpenGMK `instancelist.rs` (`remove_with`).
    pub(crate) fn remove_deleted(&mut self, program: &Program) {
        if self.deleted == 0 {
            return;
        }
        self.deleted = 0;
        for i in 0..self.slab.len() {
            if self.slab[i]
                .as_ref()
                .is_some_and(|x| x.state == State::Deleted)
            {
                let inst = self.slab[i].take().expect("checked");
                if self.ids.get(&inst.id) == Some(&Inst(i as u32)) {
                    self.ids.remove(&inst.id);
                }
                self.free.push(Reverse(i as u32));
                self.index.get_mut().removed(Inst(i as u32));
            }
        }
        let slab = &self.slab;
        self.draw_order.retain(|h| slab[h.idx()].is_some());
        // A shadowed instance whose namesake is gone owns its id again.
        self.shadowed.retain(|h| slab[h.idx()].is_some());
        let ids = &mut self.ids;
        self.shadowed.retain(|&h| {
            let id = slab[h.idx()].as_ref().expect("live").id;
            let free = !ids.contains_key(&id);
            if free {
                ids.insert(id, h);
            }
            !free
        });
        self.renumber_draw_order();
        self.refresh(program);
    }

    /// Stably sorts the draw order by depth, highest first, NaN depths last. OpenGMK `instancelist.rs`
    /// (`draw_sort`, comparing with `Real::cmp_nan_first` in reverse).
    pub(crate) fn draw_sort(&mut self) {
        let slab = &self.slab;
        let depth = |h: &Inst| slab[h.idx()].as_ref().map_or(0.0, |i| i.depth);
        self.draw_order
            .sort_by(|a, b| cmp_nan_first(depth(b), depth(a)));
        self.renumber_draw_order();
    }

    fn renumber_draw_order(&mut self) {
        for (pos, &h) in self.draw_order.iter().enumerate() {
            self.draw_pos[h.idx()] = pos as u32;
        }
    }
}

/// NaN orders below every number. OpenGMK `math.rs` (`Real::cmp_nan_first`).
pub(crate) fn cmp_nan_first(a: f64, b: f64) -> std::cmp::Ordering {
    use std::cmp::Ordering::*;
    match (a.is_nan(), b.is_nan()) {
        (true, true) => Equal,
        (true, false) => Less,
        (false, true) => Greater,
        (false, false) => a.partial_cmp(&b).expect("not NaN"),
    }
}
