//! The instance list: a slab of instances plus the order lists GM8 iterates (runtime.md section 5). The
//! structure follows OpenGMK `instancelist.rs` so iteration order, and what happens when the lists change
//! mid-loop, come out the same, but keeps GM8's two arrays, of active and of deactivated instances, where
//! OpenGMK had one (851-3311).

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
    /// Free slab slots, lowest first, as OpenGMK's chunk list reuses them (`instancelist.rs`,
    /// `ChunkList::insert`). Nothing GML sees depends on the slab order: persistent instances leave a room in
    /// the draw order ([`take_persistent`](Self::take_persistent)), not in OpenGMK's slot order.
    free: BinaryHeap<Reverse<u32>>,
    /// GM8's array of the room's active instances, destroyed ones too until the cleanup: the order `all`
    /// and drawing walk. Creation and reactivation append; the depth sort before a draw reorders it.
    draw_order: Vec<Inst>,
    /// GM8's array of deactivated instances, destroyed ones too, in the order they were deactivated. It's
    /// never sorted or cleaned up: see [`settle`](Self::settle).
    inactive_order: Vec<Inst>,
    /// By slot: GM8's deactivation flag (Spelunky.exe: the instance's byte at +0x109). Set exactly while the
    /// instance is in `inactive_order`, except within a (de)activation built-in, which sets and clears flags
    /// and then [`settle`](Self::settle)s.
    flagged: Vec<bool>,
    /// Whether a flag changed since the last [`settle`](Self::settle). Always false between built-ins.
    #[serde(skip)]
    unsettled: bool,
    /// The instances of `draw_order` of exactly each object, destroyed ones included.
    by_object: Vec<Vec<Inst>>,
    /// The instances of `draw_order` of each object or any of its descendants, destroyed ones included.
    by_ancestor: Vec<Vec<Inst>>,
    /// Instance id to handle. Ids are unique but for `shadowed`, so a lookup must take both: through
    /// [`by_id`](Self::by_id), [`last_by_id`](Self::last_by_id) or [`live_by_id`](Self::live_by_id).
    ids: BTreeMap<i32, Inst>,
    /// Instances whose id `ids` gives another (destroyed) instance: a persistent instance that arrived in a
    /// room placing one with its id, until the cleanup removes that one. Almost always empty.
    shadowed: Vec<Inst>,
    /// How many instances of `draw_order` are marked deleted, so the cleanup costs nothing on frames without
    /// any.
    deleted: usize,
    /// By slot: the instance's position in the order every per-object list follows (see
    /// [`list_rank`](Self::list_rank)).
    list_rank: Vec<u32>,
    /// The rank the next created instance gets.
    next_list_rank: u32,
    /// By slot: the instance's position in `draw_order` (stale for the instances of `inactive_order`).
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
///   of the current list. An instance appended during the loop is visited if deleted entries before it
///   are skipped in the same step.
/// - If a (de)activation moves instances between the arrays mid-loop, iteration continues at the same
///   index of the new array or list, which can skip or repeat instances. GM8 does the same (Spelunky.exe
///   0x528510, `with`: the list's entry at each index, up to the starting count).
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
        let (vec, limit) = match self.source {
            Source::Drawing => (&list.draw_order, self.count),
            Source::Object(o) | Source::Ancestor(o) => {
                if self.pos >= self.count {
                    return None;
                }
                let lists = match self.source {
                    Source::Object(_) => &list.by_object,
                    _ => &list.by_ancestor,
                };
                let vec = lists.get(o.0 as usize)?;
                (vec, vec.len())
            }
        };
        let end = limit.min(vec.len());
        let slice = vec.get(self.pos..end)?;
        let off = slice.iter().position(|&h| list.get(h).is_active())?;
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
    /// which are in its lists: a reactivation moves the instance, which rebuilds them), the lowest rank is the
    /// first a loop visits. The depth sort doesn't rebuild them, so ranks needn't follow the draw order.
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

    /// The `k`th entry of the list of `o` and its descendants (`by_ancestor`), active or not.
    pub(crate) fn ancestor_at(&self, o: ObjectId, k: usize) -> Inst {
        self.by_ancestor[o.idx()][k]
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
    /// - The draw order and the inactive array hold every live slot once between them, the inactive array
    ///   exactly the flagged ones, none of them active (the draw order none inactive), and `draw_pos` is
    ///   each instance of the draw order's position in it.
    /// - `ids` maps each live instance's id to it.
    /// - Each object's lists hold instances of the draw order of it (`by_object`) or of it and its
    ///   descendants (`by_ancestor`), in ascending `list_rank`, every draw order rank below `next_list_rank`.
    /// - The deleted count is right, and each instance has its object's variable layout and only knows user
    ///   variables the program has.
    pub(crate) fn is_consistent(&self, program: &Program) -> bool {
        let slot = |h: Inst| self.slab.get(h.idx()).and_then(Option::as_ref);
        let objects = program.object_slots();
        let slots = self.slab.len();
        let live: Vec<Inst> = (0..slots as u32)
            .map(Inst)
            .filter(|&h| slot(h).is_some())
            .collect();
        let user_vars = FIRST_USER_VAR as usize..FIRST_USER_VAR as usize + program.user_var_count();
        let sizes = self.by_object.len() == objects
            && self.by_ancestor.len() == objects
            && self.list_rank.len() == slots
            && self.draw_pos.len() == slots
            && self.flagged.len() == slots
            && self.free.len() == slots - live.len()
            && self
                .free
                .iter()
                .all(|&Reverse(f)| matches!(self.slab.get(f as usize), Some(None)));
        // Every instance's object exists before the lists below look up its parents.
        let instances = sizes
            && live.iter().all(|&h| {
                let i = slot(h).unwrap();
                (self.flagged[h.idx()] || self.list_rank[h.idx()] < self.next_list_rank)
                    && program.object(i.object).is_some()
                    && i.vars.len() == program.layout(i.object).len()
                    && i.extra
                        .iter()
                        .all(|(v, _)| user_vars.contains(&usize::from(v.0)))
            });
        if !instances {
            return false;
        }
        // Each live slot once in the two arrays: as many entries as live slots, each live and in its own
        // array (by its flag), and those of the draw order at their `draw_pos`, so none twice; then the
        // inactive array's are distinct if, counted by slot, none is there twice.
        let mut seen = vec![false; slots];
        let drawing = self.draw_order.len() + self.inactive_order.len() == live.len()
            && self.draw_order.iter().enumerate().all(|(pos, &h)| {
                slot(h).is_some_and(|i| i.state != State::Inactive)
                    && !self.flagged[h.idx()]
                    && self.draw_pos[h.idx()] as usize == pos
            })
            && self.inactive_order.iter().all(|&h| {
                slot(h).is_some_and(|i| i.state != State::Active)
                    && self.flagged[h.idx()]
                    && !std::mem::replace(&mut seen[h.idx()], true)
            });
        let ids = self.ids.len() == live.len()
            && self
                .ids
                .iter()
                .all(|(&id, &h)| slot(h).is_some_and(|i| i.id == id));
        let in_order = |list: &Vec<Inst>, member: &dyn Fn(ObjectId) -> bool| {
            list.iter()
                .all(|&h| slot(h).is_some_and(|i| member(i.object)) && !self.flagged[h.idx()])
                && list
                    .windows(2)
                    .all(|w| self.list_rank[w[0].idx()] < self.list_rank[w[1].idx()])
        };
        let lists = (0..objects).all(|o| {
            let o = ObjectId(o as u32);
            in_order(&self.by_object[o.idx()], &|x| x == o)
                && in_order(&self.by_ancestor[o.idx()], &|x| program.is_a(x, o))
        });
        let deleted = self
            .draw_order
            .iter()
            .filter(|&&h| slot(h).is_some_and(|i| i.state == State::Deleted))
            .count();
        drawing && ids && lists && self.deleted == deleted
    }

    /// Adds an instance at the end of every list it belongs to, as GM8 does (Spelunky.exe 0x4aed84): the
    /// draw order and its object's lists. OpenGMK `instancelist.rs` (`insert`).
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
                self.flagged.push(false);
                Inst(self.slab.len() as u32 - 1)
            }
        };
        self.list_rank[h.idx()] = self.next_list_rank;
        self.next_list_rank += 1;
        self.index.get_mut().moved(h);
        // Every instance arrives active: a created or placed one, or a persistent one, which leaves its room
        // only if it's active (`take_persistent`).
        assert_eq!(self.get(h).state, State::Active, "an inserted instance");
        self.flagged[h.idx()] = false;
        self.draw_pos[h.idx()] = self.draw_order.len() as u32;
        self.draw_order.push(h);
        self.by_object[object.idx()].push(h);
        for &a in program.parents(object) {
            self.by_ancestor[a.idx()].push(h);
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

    /// Takes the persistent instances out for the next room: the active ones, in draw order. GM8 (Spelunky.exe
    /// 0x53e270, after the room end events) scans the room's array of active instances for the persistent ones
    /// that aren't destroyed and copies each into the next room in that order; a deactivated one (in the other
    /// array) or a destroyed one is freed with the room (probe `persistent-carry`, 851-3315). OpenGMK
    /// (`game.rs` `load_room`, `instancelist.rs` `remove_as_vec`) took every persistent instance, in slot
    /// order. The list itself is about to be dropped, so only the slab is kept consistent.
    pub(crate) fn take_persistent(&mut self) -> Vec<Instance> {
        let Self {
            slab, draw_order, ..
        } = self;
        draw_order
            .iter()
            .filter_map(|&h| {
                let slot = &mut slab[h.idx()];
                let carried = slot
                    .as_ref()
                    .is_some_and(|i| i.persistent && i.state == State::Active);
                if carried { slot.take() } else { None }
            })
            .collect()
    }

    /// The instance with this id for `with (id)` and `distance_to_object(id)`: the first in draw order with the
    /// id, if it's active. Deactivated and deleted instances behave as if they don't exist.
    ///
    /// Ids are unique but while a persistent instance shares its id with one its room placed, which the load
    /// destroyed (see `shadowed`). GM8's lookups by id treat that pair in three ways, and this is one: `with`
    /// (Spelunky.exe 0x528596) and `distance_to_object` (0x55b6c6) scan the room's array of active instances
    /// (deactivated ones are in another) for the first with the id, and do nothing if it's destroyed, so they
    /// find nothing (probe `id-lookups`, 851-3310). The others are [`live_by_id`](Self::live_by_id) and
    /// [`last_by_id`](Self::last_by_id). OpenGMK (`instancelist.rs`, `get_by_instid`) uses this rule for all
    /// of them.
    pub(crate) fn by_id(&self, id: i32) -> Option<Inst> {
        self.with_id(id)
            .min_by_key(|&h| self.draw_rank(h))
            .filter(|&h| self.get(h).is_active())
    }

    /// The instance with this id for variable reads and writes through it (`id.x`): the last in draw order
    /// with the id, if it's active. GM8 (Spelunky.exe 0x525e74 reads, 0x526084 writes) scans the room's array of
    /// active instances from its end, so while a persistent instance shares its id with the placed one the
    /// load destroyed (see [`by_id`](Self::by_id)), that finds the persistent one, which comes after it.
    ///
    /// Before scanning, GM8 tries two array positions it cached from earlier scans, reads and writes alike:
    /// one holding an instance with the id is taken (by a read only if it isn't destroyed). A write can so take
    /// the destroyed one of the pair and write nothing (probe `id-lookups`: `c_cached_write`). That cache isn't
    /// modeled: only the pair can tell, and neither program forms it (docs/quirks.md §7).
    pub(crate) fn last_by_id(&self, id: i32) -> Option<Inst> {
        self.with_id(id)
            .max_by_key(|&h| self.draw_rank(h))
            .filter(|&h| self.get(h).is_active())
    }

    /// The active instance with this id, past a destroyed one with it: `instance_exists`, `instance_number` and
    /// `instance_find` with an id, the collision tests with one, `instance_deactivate_object` with one, and view
    /// following. GM8 (Spelunky.exe 0x55c460, 0x55c5fc, 0x55c2f0, the scans from 0x4e3898, 0x55ca4c and
    /// 0x4beca4) scans the room's array of active instances for the first with the id that isn't marked
    /// destroyed, so while a persistent instance shares its id with the placed one the load destroyed (see
    /// [`by_id`](Self::by_id)), these find the persistent one (probes `id-lookups`, 851-3310, and
    /// `view-follow-id`, 851-3309). Only one instance with an id is ever not destroyed, so the order doesn't
    /// matter. (`instance_deactivate_object` takes every instance with the id, the destroyed one too:
    /// [`with_id`](Self::with_id).)
    pub(crate) fn live_by_id(&self, id: i32) -> Option<Inst> {
        self.with_id(id).find(|&h| self.get(h).is_active())
    }

    /// The instances with this id that GM8's scans see: those in its array of active instances, the destroyed
    /// ones included (they stay there until the cleanup), but not the deactivated ones, which are in another.
    pub(crate) fn with_id(&self, id: i32) -> impl Iterator<Item = Inst> + '_ {
        let shadowed = self
            .shadowed
            .iter()
            .copied()
            .filter(move |&s| self.get(s).id == id);
        self.ids
            .get(&id)
            .copied()
            .into_iter()
            .chain(shadowed)
            .filter(|&h| !self.flagged[h.idx()])
    }

    pub(crate) fn iter_drawing(&self) -> InstIter {
        InstIter {
            source: Source::Drawing,
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

    /// The instances of the draw order, destroyed ones included until the cleanup: GM's `instance_count`. A
    /// destroyed instance that was deactivated isn't counted (probe `deactivation-order`).
    pub(crate) fn count_all(&self) -> usize {
        self.draw_order.len()
    }

    /// Active instances of `o` or its descendants. OpenGMK `instancelist.rs` (`count`).
    pub(crate) fn count_ancestor(&self, o: ObjectId) -> usize {
        self.by_ancestor.get(o.idx()).map_or(0, |v| {
            v.iter().filter(|&&h| self.get(h).is_active()).count()
        })
    }

    /// The draw order: GM8's array of active instances, destroyed ones included until the cleanup.
    pub(crate) fn draw_order(&self) -> &[Inst] {
        &self.draw_order
    }

    /// GM8's array of deactivated instances, destroyed ones included, in the order they were deactivated.
    pub(crate) fn inactive_order(&self) -> &[Inst] {
        &self.inactive_order
    }

    /// Sets or clears an instance's deactivation flag, which [`settle`](Self::settle) then acts on.
    pub(crate) fn flag(&mut self, h: Inst, deactivate: bool) {
        let was = std::mem::replace(&mut self.flagged[h.idx()], deactivate);
        self.unsettled |= was != deactivate;
    }

    /// Moves the instances whose flag no longer matches their array, as GM8 does at the end of every
    /// (de)activation built-in (Spelunky.exe 0x4af00c): the unflagged ones of the inactive array to the end of
    /// the draw order, then the flagged ones of the draw order to the end of the inactive array, each in the
    /// order they were in. If anything moved, it rebuilds the per-object lists from the new draw order
    /// (0x4ad67c, 0x4ad814), and not otherwise. So a reactivated instance comes after the others, at its depth
    /// after the next draw's sort, and loops over its object reach it last, where OpenGMK flipped its state in
    /// place (probe `deactivation-order`, 851-3311). Destroyed instances move too: one deactivated stays
    /// destroyed in the inactive array, which the cleanup doesn't reach, until it's reactivated.
    pub(crate) fn settle(&mut self, program: &Program) {
        // Most calls in a level change no flag: `oLevel` reactivates objects that are already active.
        if !std::mem::take(&mut self.unsettled) {
            return;
        }
        let back = self.inactive_order.iter().any(|&h| !self.flagged[h.idx()]);
        let away = self.draw_order.iter().any(|&h| self.flagged[h.idx()]);
        if !back && !away {
            return;
        }
        // In place, as this runs several times a frame (oLevel's step).
        let Self {
            slab,
            draw_order,
            inactive_order,
            flagged,
            deleted,
            ..
        } = self;
        fn state(slab: &mut [Option<Instance>], h: Inst) -> &mut State {
            &mut slab[h.idx()].as_mut().expect("live instance handle").state
        }
        inactive_order.retain(|&h| {
            if flagged[h.idx()] {
                return true;
            }
            match state(slab, h) {
                s @ State::Inactive => *s = State::Active,
                State::Deleted => *deleted += 1,
                State::Active => unreachable!("an active instance in the inactive array"),
            }
            draw_order.push(h);
            false
        });
        draw_order.retain(|&h| {
            if !flagged[h.idx()] {
                return true;
            }
            match state(slab, h) {
                s @ State::Active => *s = State::Inactive,
                State::Deleted => *deleted -= 1,
                State::Inactive => unreachable!("an inactive instance in the draw order"),
            }
            inactive_order.push(h);
            false
        });
        self.renumber_draw_order();
        self.refresh(program);
    }

    /// Marks an instance destroyed; it stays readable until [`remove_deleted`](Self::remove_deleted), or,
    /// if it's deactivated, until it's reactivated and then cleaned up.
    pub(crate) fn mark_deleted(&mut self, h: Inst) {
        let in_draw_order = !self.flagged[h.idx()];
        let i = self.get_mut_still(h);
        let was = std::mem::replace(&mut i.state, State::Deleted);
        if was != State::Deleted && in_draw_order {
            self.deleted += 1;
        }
    }

    /// Rebuilds the per-object lists from the draw order, destroyed instances included, as GM8 does
    /// (Spelunky.exe 0x4ad67c, 0x4ad814), reusing their allocations. OpenGMK `instancelist.rs`
    /// (`refresh_maps`) left the destroyed ones out.
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
            self.by_object[inst.object.idx()].push(h);
            for &a in program.parents(inst.object) {
                self.by_ancestor[a.idx()].push(h);
            }
        }
    }

    /// Removes the deleted instances of the draw order without reordering the rest, then rebuilds the
    /// per-object lists if any went. GM8 (Spelunky.exe 0x4aefb0) leaves the inactive array alone, so a
    /// destroyed instance there stays until it's reactivated. OpenGMK `instancelist.rs` (`remove_with`).
    pub(crate) fn remove_deleted(&mut self, program: &Program) {
        if self.deleted == 0 {
            return;
        }
        self.deleted = 0;
        for k in 0..self.draw_order.len() {
            let h = self.draw_order[k];
            if self.get(h).state == State::Deleted {
                let inst = self.slab[h.idx()].take().expect("checked");
                if self.ids.get(&inst.id) == Some(&h) {
                    self.ids.remove(&inst.id);
                }
                self.free.push(Reverse(h.0));
                self.index.get_mut().removed(h);
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

    /// Stably sorts the draw order by depth, highest first, NaN depths last; not the inactive array, nor the
    /// per-object lists. GM8 (Spelunky.exe 0x4aef24) insertion-sorts its array of active instances.
    /// OpenGMK `instancelist.rs` (`draw_sort`, comparing with `Real::cmp_nan_first` in reverse) sorted its
    /// deactivated instances with them.
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
