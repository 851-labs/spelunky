//! A spatial index for collision queries: every instance's bounding box, cached, in a uniform grid of
//! 16-pixel cells (Spelunky's tile size). It is a cache, so it can't change behaviour:
//!
//! - **Which instances.** Every instance in the slab is indexed whatever its state; queries filter by
//!   state and object afterwards. A query collects the instances in the cells its shape's pixel range
//!   touches, plus the [big](SpatialIndex::big) ones, and tests each.
//! - **No misses.** Every collision test starts by comparing bounding boxes, and fails unless the
//!   instance's box overlaps the query's pixel range (`Query::region` shows why for each shape). Overlapping
//!   ranges touch a common cell. Ranges outside the grid are clamped to its edge cells, and clamping is
//!   monotonic, so it keeps overlapping ranges overlapping.
//! - **Same first match.** The candidates come out in no particular order, so the query takes the hit
//!   that comes first in the list the linear scan would walk, by rank ([`InstanceList::list_rank`],
//!   [`InstanceList::draw_rank`](crate::list::InstanceList::draw_rank)). A query runs no GML, so the lists
//!   can't change while it runs.
//! - **Up to date.** Every mutable access to an instance goes through
//!   [`InstanceList::get_mut`](crate::list::InstanceList::get_mut), which queues it, except the frame
//!   loop's hot loops that only write `xprevious`, alarms, speeds and `image_index` (`get_mut_still`).
//!   Queries [`flush`](SpatialIndex::flush) the queue first, re-placing each queued instance whose box
//!   changed. Debug builds compare every box a query reads with a fresh one.
//! - **Read-only queries.** The index lives in a `RefCell`, so `&World` queries can flush it. A write
//!   can't re-index eagerly (it hands out `&mut Instance`, with no hook after the write), and the flush
//!   is the only mutation behind the shared reference.
//!
//! The property tests in `collision_props.rs` check all of this against linear scans on random worlds.
//!
//! [`InstanceList::list_rank`]: crate::list::InstanceList::list_rank

use crate::instance::{Bbox, Instance};
use crate::list::Inst;

/// Cells are 16 pixels square.
const CELL_SHIFT: u32 = 4;
/// Cells of margin around the room, so instances just outside it don't all land in the edge cells.
const MARGIN: i64 = 8;
/// The grid never has more than this many cells per side; a huge room clamps more into the edges.
const MAX_SIDE: i64 = 256;
/// An instance whose box covers more cells than this goes to the big list instead.
const MAX_ENTRY_CELLS: i64 = 64;
/// A query covering more cells than this scans linearly instead.
const MAX_QUERY_CELLS: i64 = 256;

/// An inclusive range of cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Cells {
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
}

impl Cells {
    fn count(&self) -> i64 {
        (i64::from(self.x1) - i64::from(self.x0) + 1)
            * (i64::from(self.y1) - i64::from(self.y0) + 1)
    }
}

/// Where an instance is in the index.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Place {
    /// Not indexed (no instance in this slot, or not flushed yet).
    #[default]
    Nowhere,
    Cells(Cells),
    Big,
}

#[derive(Clone, Copy, Debug, Default)]
struct Entry {
    bbox: Bbox,
    place: Place,
    /// Whether the slot is in the `moved` queue.
    queued: bool,
}

/// See the module docs. Built lazily on the first flush; cloning gives an unbuilt index, so savestates
/// don't copy the grid (the clone rebuilds it on its first query).
#[derive(Debug, Default)]
pub(crate) struct SpatialIndex {
    built: bool,
    /// The pixel at the top left of cell (0, 0).
    origin: (i64, i64),
    cols: i64,
    rows: i64,
    /// Instances by cell, row-major, in no particular order.
    cells: Vec<Vec<Inst>>,
    /// Instances whose boxes cover too many cells, in no particular order.
    big: Vec<Inst>,
    /// By slab slot.
    entries: Vec<Entry>,
    /// Slots that may have moved since the last flush.
    moved: Vec<Inst>,
}

impl Clone for SpatialIndex {
    fn clone(&self) -> Self {
        SpatialIndex::default()
    }
}

impl SpatialIndex {
    /// Queues an instance whose bounding box may have changed (or that was just created).
    #[inline]
    pub(crate) fn moved(&mut self, h: Inst) {
        if !self.built {
            return;
        }
        let i = h.0 as usize;
        if self.entries.len() <= i {
            self.entries.resize(i + 1, Entry::default());
        }
        let e = &mut self.entries[i];
        if !e.queued {
            e.queued = true;
            self.moved.push(h);
        }
    }

    /// Takes a removed instance out. Its slot may be reused, and is then queued as a new instance.
    pub(crate) fn removed(&mut self, h: Inst) {
        if !self.built {
            return;
        }
        if let Some(e) = self.entries.get_mut(h.0 as usize) {
            let place = std::mem::take(&mut e.place);
            self.unplace(h, place);
        }
    }

    /// Brings the index up to date: builds it for a room of this size if it isn't built, otherwise
    /// re-places the queued instances. `slab` holds the instances by slot; `bbox` is GM8's box of one.
    pub(crate) fn flush(
        &mut self,
        slab: &[Option<Instance>],
        room: (i32, i32),
        bbox: impl Fn(&Instance) -> Bbox,
    ) {
        if !self.built {
            self.build(slab, room, bbox);
            return;
        }
        while let Some(h) = self.moved.pop() {
            let i = h.0 as usize;
            self.entries[i].queued = false;
            let Some(inst) = &slab[i] else { continue };
            let b = bbox(inst);
            let place = self.place_of(&b);
            let e = &mut self.entries[i];
            e.bbox = b;
            if e.place != place {
                let old = std::mem::replace(&mut e.place, place);
                self.unplace(h, old);
                self.put(h, place);
            }
        }
    }

    fn build(
        &mut self,
        slab: &[Option<Instance>],
        room: (i32, i32),
        bbox: impl Fn(&Instance) -> Bbox,
    ) {
        let side =
            |pixels: i32| ((i64::from(pixels.max(0)) >> CELL_SHIFT) + 1 + 2 * MARGIN).min(MAX_SIDE);
        let margin = MARGIN << CELL_SHIFT;
        *self = SpatialIndex {
            built: true,
            origin: (-margin, -margin),
            cols: side(room.0),
            rows: side(room.1),
            entries: vec![Entry::default(); slab.len()],
            ..SpatialIndex::default()
        };
        self.cells = vec![Vec::new(); (self.cols * self.rows) as usize];
        for (i, inst) in slab.iter().enumerate() {
            let Some(inst) = inst else { continue };
            let b = bbox(inst);
            let place = self.place_of(&b);
            self.entries[i] = Entry {
                bbox: b,
                place,
                queued: false,
            };
            self.put(Inst(i as u32), place);
        }
    }

    /// The cached bounding box of an instance. Only valid right after a [`flush`](Self::flush).
    #[inline]
    pub(crate) fn bbox(&self, h: Inst) -> Bbox {
        debug_assert!(self.built && self.moved.is_empty(), "flushed");
        self.entries[h.0 as usize].bbox
    }

    /// The cells a pixel range (inclusive) touches, or `None` if that's too many to be worth it.
    pub(crate) fn cells(&self, left: i64, top: i64, right: i64, bottom: i64) -> Option<Cells> {
        let c = self.clamp(left, top, right, bottom);
        (c.count() <= MAX_QUERY_CELLS).then_some(c)
    }

    /// Calls `f` once with every instance indexed in these cells, and with every big instance.
    #[inline]
    pub(crate) fn candidates(&self, c: Cells, mut f: impl FnMut(Inst)) {
        for y in c.y0..=c.y1 {
            let row = y as usize * self.cols as usize;
            for x in c.x0..=c.x1 {
                for &h in &self.cells[row + x as usize] {
                    // An instance in several cells comes up in the first one it shares with the range.
                    if let Place::Cells(e) = self.entries[h.0 as usize].place
                        && (x != e.x0.max(c.x0) || y != e.y0.max(c.y0))
                    {
                        continue;
                    }
                    f(h);
                }
            }
        }
        self.big.iter().for_each(|&h| f(h));
    }

    /// Whether an instance's box covers too many cells for the grid (it's then a candidate everywhere).
    pub(crate) fn is_big(&self, h: Inst) -> bool {
        self.entries[h.0 as usize].place == Place::Big
    }

    fn clamp(&self, left: i64, top: i64, right: i64, bottom: i64) -> Cells {
        let col = |x: i64| ((x - self.origin.0) >> CELL_SHIFT).clamp(0, self.cols - 1) as u32;
        let row = |y: i64| ((y - self.origin.1) >> CELL_SHIFT).clamp(0, self.rows - 1) as u32;
        Cells {
            x0: col(left),
            y0: row(top),
            x1: col(right),
            y1: row(bottom),
        }
    }

    fn place_of(&self, b: &Bbox) -> Place {
        // A box always has left <= right and top <= bottom (they're the min and max of its corners).
        let c = self.clamp(
            i64::from(b.left),
            i64::from(b.top),
            i64::from(b.right),
            i64::from(b.bottom),
        );
        if c.count() > MAX_ENTRY_CELLS {
            Place::Big
        } else {
            Place::Cells(c)
        }
    }

    fn put(&mut self, h: Inst, place: Place) {
        match place {
            Place::Nowhere => {}
            Place::Big => self.big.push(h),
            Place::Cells(c) => {
                for y in c.y0..=c.y1 {
                    for x in c.x0..=c.x1 {
                        self.cells[(i64::from(y) * self.cols + i64::from(x)) as usize].push(h);
                    }
                }
            }
        }
    }

    fn unplace(&mut self, h: Inst, place: Place) {
        let remove = |v: &mut Vec<Inst>| {
            let i = v
                .iter()
                .position(|&x| x == h)
                .expect("indexed where recorded");
            v.swap_remove(i);
        };
        match place {
            Place::Nowhere => {}
            Place::Big => remove(&mut self.big),
            Place::Cells(c) => {
                for y in c.y0..=c.y1 {
                    for x in c.x0..=c.x1 {
                        remove(&mut self.cells[(i64::from(y) * self.cols + i64::from(x)) as usize]);
                    }
                }
            }
        }
    }
}
