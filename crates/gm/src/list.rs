//! The instance list: a slab of instances plus the order lists GM8 iterates (runtime.md section 5). The
//! structure copies OpenGMK `instancelist.rs` so iteration order, and what happens when the lists change
//! mid-loop, come out the same.

use crate::instance::{Instance, State};
use crate::program::{ObjectId, Program};
use std::collections::BTreeMap;

/// An instance handle: a slot in the slab. GML never sees these; it sees instance ids.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Inst(pub u32);

impl Inst {
    #[inline]
    fn idx(self) -> usize {
        self.0 as usize
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct InstanceList {
    slab: Vec<Option<Instance>>,
    /// Free slab slots, reused last-freed first. Handles are invisible to GML, so reuse can't change
    /// behaviour; it just keeps the slab small.
    free: Vec<u32>,
    /// Every instance of the room. Creation appends; drawing sorts it by depth.
    draw_order: Vec<Inst>,
    /// Active instances of exactly each object (plus instances deleted since the last rebuild).
    by_object: Vec<Vec<Inst>>,
    /// Active instances of each object or any of its descendants.
    by_ancestor: Vec<Vec<Inst>>,
    /// Instance id to handle. Ids are unique, so this finds what GM8's scan of the draw order finds.
    ids: BTreeMap<i32, Inst>,
    /// How many instances are marked deleted, so the cleanup costs nothing on frames without any.
    deleted: usize,
    /// How many instances are deactivated, for `instance_count`.
    inactive: usize,
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

    #[inline]
    pub(crate) fn get_mut(&mut self, h: Inst) -> &mut Instance {
        self.slab[h.idx()].as_mut().expect("live instance handle")
    }

    /// Sizes the per-object lists for a program.
    pub(crate) fn with_objects(n: usize) -> Self {
        InstanceList {
            by_object: vec![Vec::new(); n],
            by_ancestor: vec![Vec::new(); n],
            ..Self::default()
        }
    }

    /// Adds an instance at the end of every list it belongs to. OpenGMK `instancelist.rs` (`insert`).
    pub(crate) fn insert(&mut self, inst: Instance, program: &Program) -> Inst {
        let (id, object) = (inst.id, inst.object);
        let h = match self.free.pop() {
            Some(i) => {
                self.slab[i as usize] = Some(inst);
                Inst(i)
            }
            None => {
                self.slab.push(Some(inst));
                Inst(self.slab.len() as u32 - 1)
            }
        };
        self.draw_order.push(h);
        self.by_object[object.idx()].push(h);
        for &a in program.parents(object) {
            self.by_ancestor[a.idx()].push(h);
        }
        let old = self.ids.insert(id, h);
        debug_assert!(old.is_none(), "instance id {id} reused");
        h
    }

    /// The active instance with this id. Deactivated and deleted instances behave as if they don't exist.
    /// OpenGMK `instancelist.rs` (`get_by_instid`).
    pub(crate) fn by_id(&self, id: i32) -> Option<Inst> {
        let h = *self.ids.get(&id)?;
        self.get(h).is_active().then_some(h)
    }

    /// Any instance with this id, active or not.
    pub(crate) fn by_id_any(&self, id: i32) -> Option<Inst> {
        self.ids.get(&id).copied()
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
        let i = self.get_mut(h);
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
        let i = self.get_mut(h);
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
                self.ids.remove(&inst.id);
                self.free.push(i as u32);
            }
        }
        let slab = &self.slab;
        self.draw_order.retain(|h| slab[h.idx()].is_some());
        self.refresh(program);
    }

    /// Stably sorts the draw order by depth, highest first, NaN depths last. OpenGMK `instancelist.rs`
    /// (`draw_sort`, comparing with `Real::cmp_nan_first` in reverse).
    pub(crate) fn draw_sort(&mut self) {
        let slab = &self.slab;
        let depth = |h: &Inst| slab[h.idx()].as_ref().map_or(0.0, |i| i.depth);
        self.draw_order
            .sort_by(|a, b| cmp_nan_first(depth(b), depth(a)));
    }
}

/// NaN orders below every number. OpenGMK `math.rs` (`Real::cmp_nan_first`).
fn cmp_nan_first(a: f64, b: f64) -> std::cmp::Ordering {
    use std::cmp::Ordering::*;
    match (a.is_nan(), b.is_nan()) {
        (true, true) => Equal,
        (true, false) => Less,
        (false, true) => Greater,
        (false, false) => a.partial_cmp(&b).expect("not NaN"),
    }
}
