//! Collision tests, the collision built-ins and the collision event stage (runtime.md section 7).
//!
//! Runner behaviour follows OpenGMK: `game.rs` (`check_collision`, `check_collision_point`,
//! `check_collision_rectangle`, `check_collision_line`, `find_instance_with`), `asset/sprite.rs`
//! (`Collider::check_collision_point_precise`), `gml/kernel.rs` (the built-ins) and `game/events.rs`
//! (`run_collisions`). Masks are the GM8 ones in the asset bundle (`assets::Mask`), one per frame or one
//! shared, in sprite pixel coordinates.
//!
//! Every test first compares bounding boxes (inclusive integer edges, [`Assets::bbox_at`]). A precise
//! test then maps room pixels into the mask: GM8 rounds the instance's position half to even, rotates the
//! pixel's offset from it by `image_angle`, divides by the scale, adds the origin and floors.

use crate::ctx::Ctx;
use crate::error::R;
use crate::index::SpatialIndex;
use crate::instance::{Bbox, Instance};
use crate::kernel::{ALL, FIRST_INSTANCE_ID, NOONE, OTHER, SELF};
use crate::list::{Inst, InstIter};
use crate::math;
use crate::program::ObjectId;
use crate::world::{Assets, World};
use std::cell::Ref;

/// An object's instance list counts as short with at most this many entries (active or not). Queries scan a
/// short list rather than look in the spatial index, and the collision stage tests a short target list's
/// instances one by one rather than jumping to the overlapping ones.
const SHORT_LIST: usize = 8;

fn is_short(len: usize) -> bool {
    len <= SHORT_LIST
}

/// What `distance_to_object` returns when no instance qualifies. OpenGMK `gml/kernel.rs`
/// (`distance_to_object`, "GML default").
const NO_DISTANCE: f64 = 1_000_000.0;

/// What a collision test reads of one instance, possibly placed where it isn't (`instance_place`).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Body {
    pub(crate) bbox: Bbox,
    /// The mask sprite: `mask_index`, or `sprite_index` if that's negative.
    sprite: i32,
    x: f64,
    y: f64,
    xscale: f64,
    yscale: f64,
    angle: f64,
    image_index: f64,
}

impl Body {
    pub(crate) fn new(inst: &Instance, x: f64, y: f64, bbox: Bbox) -> Body {
        Body {
            bbox,
            sprite: inst.mask_sprite(),
            x,
            y,
            xscale: inst.image_xscale,
            yscale: inst.image_yscale,
            angle: inst.image_angle,
            image_index: inst.image_index,
        }
    }

    /// The instance's sine and cosine of `image_angle` (not negated, as for the bounding box), converted as
    /// GM8's runner converts it ([`math::sin_cos_degrees`]). GM8 takes them of `-image_angle` and adds where
    /// `map` subtracts, which gives the same bits.
    fn sin_cos(&self) -> (f64, f64) {
        math::sin_cos_degrees(self.angle)
    }
}

/// The mask an instance tests with for its current frame. GM8 has two rules (OpenGMK `game.rs`):
/// instance-to-instance tests take `floor(image_index)` as a 32-bit integer modulo the frame count, always
/// non-negative (`euclid`); point, rectangle and line tests convert `floor(image_index)` to an unsigned
/// 64-bit integer first, which turns negative values into 0. `None` (no collision) for a sprite without
/// masks.
fn collider(sprite: &assets::Sprite, image_index: f64, euclid: bool) -> Option<&assets::Mask> {
    if !sprite.separate_masks {
        return sprite.masks.first();
    }
    let n = sprite.masks.len();
    if n == 0 {
        return None;
    }
    let f = image_index.floor();
    let i = if euclid {
        (f as i64 as i32).rem_euclid(n as i32) as usize
    } else {
        // `as usize` in OpenGMK, which is 64 bits on the oracle; u64 keeps that on wasm32.
        (f as u64 % n as u64) as usize
    };
    sprite.masks.get(i)
}

/// Whether mask pixel (x, y) is solid: inside the mask's box and set. OpenGMK `asset/sprite.rs`
/// (`Collider::check_collision_point_precise`, last lines).
#[inline]
fn mask_pixel(mask: &assets::Mask, x: i32, y: i32) -> bool {
    let b = mask.bbox;
    // An empty mask's box has left > right, so nothing is inside it.
    x >= b.left as i32
        && y >= b.top as i32
        && x <= b.right as i32
        && y <= b.bottom as i32
        && mask.get(x as u32, y as u32)
}

/// How one instance maps room pixels into its mask. OpenGMK `asset/sprite.rs`
/// (`Collider::check_collision_point_precise`).
struct Xform<'a> {
    mask: &'a assets::Mask,
    /// The instance's position, rounded half to even.
    ix: i32,
    iy: i32,
    ox: i32,
    oy: i32,
    xscale: f64,
    yscale: f64,
    sin: f64,
    cos: f64,
    /// Unrotated with scales of exactly ±1, the float transform is an integer one: (x signs, y sign).
    unit: Option<(i64, i64)>,
}

impl<'a> Xform<'a> {
    fn new(body: &Body, sprite: &assets::Sprite, mask: &'a assets::Mask) -> Xform<'a> {
        let (sin, cos) = body.sin_cos();
        let sign = |s: f64| match s {
            1.0 => Some(1),
            -1.0 => Some(-1),
            _ => None,
        };
        let unit = match (
            sin == 0.0 && cos == 1.0,
            sign(body.xscale),
            sign(body.yscale),
        ) {
            (true, Some(sx), Some(sy)) => Some((sx, sy)),
            _ => None,
        };
        Xform {
            mask,
            ix: math::to_int(body.x),
            iy: math::to_int(body.y),
            ox: sprite.origin_x,
            oy: sprite.origin_y,
            xscale: body.xscale,
            yscale: body.yscale,
            sin,
            cos,
            unit,
        }
    }

    /// Whether room pixel (x, y) is solid in the mask.
    #[inline]
    fn hit(&self, x: i32, y: i32) -> bool {
        let (mx, my) = self.map(x, y);
        mask_pixel(self.mask, mx, my)
    }

    /// The mask pixel room pixel (x, y) falls on.
    #[inline]
    fn map(&self, x: i32, y: i32) -> (i32, i32) {
        // OpenGMK subtracts in i32, which wraps in its release build.
        let dx = x.wrapping_sub(self.ix);
        let dy = y.wrapping_sub(self.iy);
        match self.unit {
            // With sin 0, cos 1 and scale ±1 every float step below is exact, so this is the same result
            // without the float work: `floor(o + d / ±1)` is `o ± d`, and `as i64 as i32` wraps.
            Some((sx, sy)) => (
                (i64::from(self.ox) + sx * i64::from(dx)) as i32,
                (i64::from(self.oy) + sy * i64::from(dy)) as i32,
            ),
            None => {
                let (dx, dy) = (f64::from(dx), f64::from(dy));
                let rx = dx * self.cos - dy * self.sin;
                let ry = dx * self.sin + dy * self.cos;
                (
                    (f64::from(self.ox) + rx / self.xscale).floor() as i64 as i32,
                    (f64::from(self.oy) + ry / self.yscale).floor() as i64 as i32,
                )
            }
        }
    }
}

/// Whether two bounding boxes share a pixel.
#[inline]
fn overlap(a: &Bbox, b: &Bbox) -> bool {
    !(a.right < b.left || b.right < a.left || a.bottom < b.top || b.bottom < a.top)
}

/// Instance against instance: bounding boxes, then every pixel of their intersection, which needs both
/// masks. Without a mask sprite on either side there is no collision. OpenGMK `game.rs`
/// (`check_collision`).
pub(crate) fn bodies_collide(assets: &Assets, a: &Body, b: &Body) -> bool {
    overlap(&a.bbox, &b.bbox) && masks_collide(assets, a, b)
}

/// The part of [`bodies_collide`] after the bounding boxes passed.
fn masks_collide(assets: &Assets, a: &Body, b: &Body) -> bool {
    let (ba, bb) = (a.bbox, b.bbox);
    let (Some(sa), Some(sb)) = (assets.sprite(a.sprite), assets.sprite(b.sprite)) else {
        return false;
    };
    let (Some(ca), Some(cb)) = (
        collider(sa, a.image_index, true),
        collider(sb, b.image_index, true),
    ) else {
        return false;
    };
    let (xa, xb) = (Xform::new(a, sa, ca), Xform::new(b, sb, cb));
    let (left, right) = (ba.left.max(bb.left), ba.right.min(bb.right));
    for y in ba.top.max(bb.top)..=ba.bottom.min(bb.bottom) {
        for x in left..=right {
            if xa.hit(x, y) && xb.hit(x, y) {
                return true;
            }
        }
    }
    false
}

/// The mask an instance tests points, rectangles and lines with, with its transform.
fn point_xform<'a>(assets: &'a Assets, body: &Body) -> Option<Xform<'a>> {
    let sprite = assets.sprite(body.sprite)?;
    let mask = collider(sprite, body.image_index, false)?;
    Some(Xform::new(body, sprite, mask))
}

/// A collision test against a shape, for the `collision_*` and `instance_*` built-ins.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Query {
    /// `collision_point`, `instance_position`. OpenGMK `game.rs` (`check_collision_point`).
    Point { x: f64, y: f64, precise: bool },
    /// `collision_rectangle`; the corners are normalised. OpenGMK `check_collision_rectangle`.
    Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
        precise: bool,
    },
    /// `collision_line`. OpenGMK `check_collision_line`.
    Line {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        precise: bool,
    },
    /// Another instance's mask (`instance_place`, `place_meeting`). OpenGMK `check_collision`.
    Body(Body),
}

impl Query {
    pub(crate) fn rect(x1: i32, y1: i32, x2: i32, y2: i32, precise: bool) -> Query {
        Query::Rect {
            left: x1.min(x2),
            top: y1.min(y2),
            right: x1.max(x2),
            bottom: y1.max(y2),
            precise,
        }
    }

    /// The pixels (inclusive left, top, right, bottom) a bounding box must overlap for [`hits`] to
    /// pass its first test; `None` if a coordinate is NaN (every comparison with NaN is false, so the box
    /// test passes for any box). Points and line ends are floored: `left <= x <= right` with integer edges
    /// implies `left <= floor(x) <= right`, and the line's `x < right + 1` implies `floor(x) <= right`.
    ///
    /// [`hits`]: Self::hits
    pub(crate) fn region(&self) -> Option<(i64, i64, i64, i64)> {
        // Far coordinates are clamped: the index clamps to its grid anyway.
        let floor = |v: f64| (!v.is_nan()).then(|| v.floor().clamp(-1e12, 1e12) as i64);
        match *self {
            Query::Point { x, y, .. } => {
                let (x, y) = (floor(x)?, floor(y)?);
                Some((x, y, x, y))
            }
            Query::Rect {
                left,
                top,
                right,
                bottom,
                ..
            } => Some((left.into(), top.into(), right.into(), bottom.into())),
            Query::Line { x1, y1, x2, y2, .. } => {
                let (x1, y1, x2, y2) = (floor(x1)?, floor(y1)?, floor(x2)?, floor(y2)?);
                Some((x1.min(x2), y1.min(y2), x1.max(x2), y1.max(y2)))
            }
            Query::Body(ref b) => {
                let b = b.bbox;
                Some((b.left.into(), b.top.into(), b.right.into(), b.bottom.into()))
            }
        }
    }

    /// Whether `body` meets this shape.
    pub(crate) fn hits(&self, assets: &Assets, body: &Body) -> bool {
        let b = body.bbox;
        match *self {
            Query::Point { x, y, precise } => {
                if f64::from(b.right) < x
                    || x < f64::from(b.left)
                    || f64::from(b.bottom) < y
                    || y < f64::from(b.top)
                {
                    return false;
                }
                if !precise {
                    return true;
                }
                point_xform(assets, body).is_some_and(|xf| xf.hit(math::to_int(x), math::to_int(y)))
            }
            Query::Rect {
                left,
                top,
                right,
                bottom,
                precise,
            } => {
                let rect = Bbox {
                    left,
                    top,
                    right,
                    bottom,
                };
                if !overlap(&b, &rect) {
                    return false;
                }
                if !precise {
                    return true;
                }
                let Some(xf) = point_xform(assets, body) else {
                    return false;
                };
                let (l, r) = (b.left.max(left), b.right.min(right));
                (b.top.max(top)..=b.bottom.min(bottom)).any(|y| (l..=r).any(|x| xf.hit(x, y)))
            }
            Query::Line {
                x1,
                y1,
                x2,
                y2,
                precise,
            } => line_hits(assets, body, x1, y1, x2, y2, precise),
            Query::Body(ref other) => bodies_collide(assets, other, body),
        }
    }
}

/// OpenGMK `game.rs` (`check_collision_line`), step by step. Its quirks, kept: the bounding box counts
/// as one pixel wider and taller for the line (`right + 1`, `bottom + 1`); the line is clipped to the box
/// horizontally only; and the precise test walks the rounded line one pixel per step along its longer
/// axis, mapping each unrounded point into the mask with `origin + floor(offset / scale)`, not the point
/// test's `floor(origin + offset / scale)` on rounded pixels.
fn line_hits(
    assets: &Assets,
    body: &Body,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    precise: bool,
) -> bool {
    let b = body.bbox;
    let (left, right) = (f64::from(b.left), f64::from(b.right));
    let (top, bottom) = (f64::from(b.top), f64::from(b.bottom));
    if right + 1.0 <= x1.min(x2)
        || x1.max(x2) < left
        || bottom + 1.0 <= y1.min(y2)
        || y1.max(y2) < top
    {
        return false;
    }
    let (mut x1, mut y1, mut x2, mut y2) = if x2 < x1 {
        (x2, y2, x1, y1)
    } else {
        (x1, y1, x2, y2)
    };
    if x1 < left {
        y1 = (y2 - y1) * (left - x1) / (x2 - x1) + y1;
        x1 = left;
    }
    if x2 > right + 1.0 {
        let nx2 = right + 1.0;
        y2 = (y2 - y1) * (nx2 - x2) / (x2 - x1) + y2;
        x2 = nx2;
    }
    if (top > y1 && top > y2) || (y1 >= bottom + 1.0 && y2 >= bottom + 1.0) {
        return false;
    }
    if !precise {
        return true;
    }
    let Some(sprite) = assets.sprite(body.sprite) else {
        return false;
    };
    let Some(mask) = collider(sprite, body.image_index, false) else {
        return false;
    };
    let (ix, iy) = (math::round_half_even(body.x), math::round_half_even(body.y));
    let (sin, cos) = body.sin_cos();
    let (x1, y1, x2, y2) = (
        math::to_int(x1),
        math::to_int(y1),
        math::to_int(x2),
        math::to_int(y2),
    );
    // In i32, wrapping like OpenGMK's release build.
    let vertical = x2.wrapping_sub(x1).wrapping_abs() < y2.wrapping_sub(y1).wrapping_abs();
    let (x1, y1, x2, y2) = if vertical && y2 < y1 {
        (x2, y2, x1, y1)
    } else {
        (x1, y1, x2, y2)
    };
    let count = if vertical {
        y2.wrapping_sub(y1)
    } else {
        x2.wrapping_sub(x1)
    }
    .wrapping_add(1);
    // OpenGMK computes the slope at every step (unused for a single point, where it would divide by 0).
    let slope = if vertical {
        f64::from(x2.wrapping_sub(x1)) / f64::from(y2.wrapping_sub(y1))
    } else {
        f64::from(y2.wrapping_sub(y1)) / f64::from(x2.wrapping_sub(x1))
    };
    let point = |i: i32| match (count, vertical) {
        (1, _) => (f64::from(x1), f64::from(y1)),
        (_, true) => (
            f64::from(x1) + f64::from(i) * slope,
            f64::from(y1.wrapping_add(i)),
        ),
        (_, false) => (
            f64::from(x1.wrapping_add(i)),
            f64::from(y1) + f64::from(i) * slope,
        ),
    };
    (0..count).any(|i| {
        let (px, py) = point(i);
        let (dx, dy) = (px - ix, py - iy);
        let (rx, ry) = (dx * cos - dy * sin, dx * sin + dy * cos);
        let mx = (f64::from(sprite.origin_x) + (rx / body.xscale).floor()) as i64 as i32;
        let my = (f64::from(sprite.origin_y) + (ry / body.yscale).floor()) as i64 as i32;
        mask_pixel(mask, mx, my)
    })
}

impl World {
    /// The spatial index (`index.rs`), brought up to date. Every query and collision test reads boxes
    /// through it; flushing costs nothing when no instance moved.
    pub(crate) fn index(&self) -> Ref<'_, SpatialIndex> {
        let room = (self.room.width, self.room.height);
        self.insts
            .flush_index(room, |i| self.assets.bbox_at(i, i.x, i.y))
    }

    /// Instance `h`'s bounding box from the index. Debug builds check it against a fresh one, so a write
    /// that moved an instance without queueing it (see `InstanceList::get_mut`) fails the tests.
    #[inline]
    fn cached_bbox(&self, ix: &SpatialIndex, h: Inst) -> Bbox {
        let b = ix.bbox(h);
        debug_assert_eq!(b, self.bbox(h), "the spatial index missed a move of {h:?}");
        b
    }

    /// Instance `h` with its cached bounding box.
    fn body(&self, ix: &SpatialIndex, h: Inst) -> Body {
        let inst = self.insts.get(h);
        Body::new(inst, inst.x, inst.y, self.cached_bbox(ix, h))
    }

    /// Instance `h` as if it were at (x, y).
    fn body_at(&self, h: Inst, x: f64, y: f64) -> Body {
        let inst = self.insts.get(h);
        Body::new(inst, x, y, self.assets.bbox_at(inst, x, y))
    }

    /// Whether instance `h`'s mask contains the point, as a precise `collision_point` tests it: the mouse
    /// events' test. OpenGMK `game.rs` (`check_collision_point`, with `precise`).
    pub(crate) fn instance_at_point(&self, h: Inst, x: i32, y: i32) -> bool {
        let q = Query::Point {
            x: f64::from(x),
            y: f64::from(y),
            precise: true,
        };
        q.hits(&self.assets, &self.body(&self.index(), h))
    }

    /// Whether two instances collide; never with themselves. OpenGMK `game.rs` (`check_collision`).
    pub fn instances_collide(&self, a: Inst, b: Inst) -> bool {
        let ix = self.index();
        a != b && self.meets(&ix, &self.body(&ix, a), b)
    }

    /// Whether instance `b` collides with `a`'s body, which must be another instance's.
    fn meets(&self, ix: &SpatialIndex, a: &Body, b: Inst) -> bool {
        let bb = self.cached_bbox(ix, b);
        overlap(&a.bbox, &bb) && {
            let inst = self.insts.get(b);
            masks_collide(&self.assets, a, &Body::new(inst, inst.x, inst.y, bb))
        }
    }

    /// Whether an instance can collide with other instances at all: it needs a mask sprite with masks.
    fn can_collide(&self, h: Inst) -> bool {
        self.assets
            .sprite(self.insts.get(h).mask_sprite())
            .is_some_and(|s| !s.masks.is_empty())
    }

    /// The first instance `target` names that meets `q`, skipping `skip`: `all` in draw order, an object
    /// with its descendants in list order, or an active instance id. Every other negative value (`self`,
    /// `other`, `noone`) finds nothing. OpenGMK `game.rs` (`find_instance_with`).
    ///
    /// Through the spatial index, this tests only the instances near the query and keeps the one that
    /// comes first in the list a linear scan would walk; `index.rs` explains why that's the same instance.
    pub(crate) fn find_hit(&self, target: i32, q: &Query, skip: Option<Inst>) -> Option<Inst> {
        let ix = self.index();
        let ok = |h: Inst| Some(h) != skip && q.hits(&self.assets, &self.body(&ix, h));
        let object = match target {
            ALL => None,
            t if t < 0 => return None,
            t if t < FIRST_INSTANCE_ID => Some(ObjectId(t as u32)),
            id => return self.insts.by_id(id).filter(|&h| ok(h)),
        };
        // A short list is quicker to scan than the grid; so is a huge region. A NaN coordinate has none.
        let short = object.is_some_and(|o| is_short(self.insts.ancestor_len(o)));
        let region = q.region().filter(|_| !short);
        let Some(cells) = region.and_then(|(l, t, r, b)| ix.cells(l, t, r, b)) else {
            let mut it = match object {
                None => self.insts.iter_drawing(),
                Some(o) => self.insts.iter_ancestor(o),
            };
            return std::iter::from_fn(|| it.next(&self.insts)).find(|&h| ok(h));
        };
        let mut best: Option<(u32, Inst)> = None;
        ix.candidates(cells, |h| {
            let inst = self.insts.get(h);
            if !inst.is_active() {
                return;
            }
            // The active instances in the list the scan walks: all of them, or the object's (every
            // active instance is in its objects' lists).
            let rank = match object {
                None => self.insts.draw_rank(h),
                Some(o) if self.program.is_a(inst.object, o) => self.insts.list_rank(h),
                Some(_) => return,
            };
            if best.is_none_or(|(r, _)| rank < r) && ok(h) {
                best = Some((rank, h));
            }
        });
        best.map(|(_, h)| h)
    }

    fn id_or_noone(&self, h: Option<Inst>) -> i32 {
        h.map_or(NOONE, |h| self.insts.get(h).id)
    }

    /// `collision_point(x, y, obj, prec, notme)`: the id of the first instance of `obj` (an object, `all`
    /// or an instance id) whose bounding box contains the point, and with `prec` whose mask does too; or
    /// `noone`. With `notme` it skips `self`. `self` and `other` as `obj` find nothing. OpenGMK
    /// `gml/kernel.rs` (`collision_point`).
    pub fn collision_point(
        &self,
        cx: &Ctx,
        x: f64,
        y: f64,
        obj: i32,
        prec: bool,
        notme: bool,
    ) -> i32 {
        let q = Query::Point {
            x,
            y,
            precise: prec,
        };
        let h = self.find_hit(obj, &q, notme.then_some(cx.this));
        self.id_or_noone(h)
    }

    /// `collision_rectangle(x1, y1, x2, y2, obj, prec, notme)`. The corners are integers (the wrapper
    /// rounds them) in either order, and the rectangle includes both edges. OpenGMK `gml/kernel.rs`
    /// (`collision_rectangle`).
    #[allow(clippy::too_many_arguments)]
    pub fn collision_rectangle(
        &self,
        cx: &Ctx,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        obj: i32,
        prec: bool,
        notme: bool,
    ) -> i32 {
        let q = Query::rect(x1, y1, x2, y2, prec);
        let h = self.find_hit(obj, &q, notme.then_some(cx.this));
        self.id_or_noone(h)
    }

    /// `collision_line(x1, y1, x2, y2, obj, prec, notme)`. OpenGMK `gml/kernel.rs` (`collision_line`); see
    /// [`line_hits`] for GM8's line rules.
    #[allow(clippy::too_many_arguments)]
    pub fn collision_line(
        &self,
        cx: &Ctx,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        obj: i32,
        prec: bool,
        notme: bool,
    ) -> i32 {
        let q = Query::Line {
            x1,
            y1,
            x2,
            y2,
            precise: prec,
        };
        let h = self.find_hit(obj, &q, notme.then_some(cx.this));
        self.id_or_noone(h)
    }

    /// `instance_position(x, y, obj)`: [`collision_point`](Self::collision_point), always precise, never
    /// skipping `self`. OpenGMK `gml/kernel.rs` (`instance_position`).
    pub fn instance_position(&self, x: f64, y: f64, obj: i32) -> i32 {
        let q = Query::Point {
            x,
            y,
            precise: true,
        };
        let h = self.find_hit(obj, &q, None);
        self.id_or_noone(h)
    }

    /// `instance_place(x, y, obj)`: the id of the first instance of `obj` that `self` would collide with
    /// at (x, y), or `noone`. `self` and `other` as `obj` find nothing. OpenGMK `gml/kernel.rs`
    /// (`instance_place`; it moves the instance there and back, which only matters where that traps:
    /// [`test_at`](Self::test_at)).
    pub fn instance_place(&mut self, cx: &Ctx, x: f64, y: f64, obj: i32) -> R<i32> {
        self.test_at(cx.this, x, y, |w| {
            let q = Query::Body(w.body_at(cx.this, x, y));
            let h = w.find_hit(obj, &q, Some(cx.this));
            w.id_or_noone(h)
        })
    }

    /// `place_meeting(x, y, obj)`: whether `self` would collide with an instance of `obj` at (x, y).
    /// Unlike the other queries, `self` is never met and `other` is tested (even if it's deactivated).
    /// OpenGMK `gml/kernel.rs` (`place_meeting`), and GM8's move there and back ([`test_at`](Self::test_at)).
    pub fn place_meeting(&mut self, cx: &Ctx, x: f64, y: f64, obj: i32) -> R<bool> {
        self.test_at(cx.this, x, y, |w| {
            let me = w.body_at(cx.this, x, y);
            match obj {
                SELF => false,
                OTHER => {
                    let ix = w.index();
                    cx.other != cx.this && bodies_collide(&w.assets, &me, &w.body(&ix, cx.other))
                }
                obj => w.find_hit(obj, &Query::Body(me), Some(cx.this)).is_some(),
            }
        })
    }

    /// `test` of instance `h` at (x, y), as GM8 makes it: it moves the instance there through its position
    /// setter, tests, and moves it back (Spelunky.exe 0x4e46fc and its siblings), and each move recomputes the
    /// bounding box, which can trap on Intel ([`check_bbox`](Self::check_bbox)). Where the move there traps,
    /// the instance is left at (x, y); where the move back does, it's back and the test's answer is lost with
    /// the rest of the frame (probe `bbox-trap`, 851-3301). The test itself needs no move: the box at (x, y)
    /// is computed on demand.
    fn test_at<T>(&mut self, h: Inst, x: f64, y: f64, test: impl FnOnce(&World) -> T) -> R<T> {
        let i = self.insts.get(h);
        let moved = i.x != x || i.y != y;
        if moved && let Some(e) = self.assets.bbox_trap(i, x, y) {
            let i = self.insts.get_mut(h);
            (i.x, i.y) = (x, y);
            return Err(e.into());
        }
        let r = test(self);
        if moved {
            self.check_bbox(h)?;
        }
        Ok(r)
    }

    /// `distance_to_point(x, y)`: from `self`'s bounding box to the point, 0 inside it. OpenGMK
    /// `gml/kernel.rs` (`distance_to_point`).
    pub fn distance_to_point(&self, cx: &Ctx, x: f64, y: f64) -> f64 {
        let b = self.cached_bbox(&self.index(), cx.this);
        let axis = |v: f64, lo: i32, hi: i32| {
            if v < f64::from(lo) {
                v - f64::from(lo)
            } else if v > f64::from(hi) {
                v - f64::from(hi)
            } else {
                0.0
            }
        };
        math::hypot(axis(x, b.left, b.right), axis(y, b.top, b.bottom))
    }

    /// `distance_to_object(obj)`: the smallest distance between `self`'s bounding box and those of the
    /// instances of `obj` (an object, `all`, an instance id or `other`), or 1000000 if there are none.
    /// `self` doesn't count. `other` is measured even if it's deactivated; `self`, `noone`, and `other`
    /// when it is `self`, give 1000000; 100000 is an object. OpenGMK `gml/kernel.rs`
    /// (`distance_to_object`).
    pub fn distance_to_object(&self, cx: &Ctx, obj: i32) -> f64 {
        let ix = self.index();
        let me = self.cached_bbox(&ix, cx.this);
        let to = |h: Inst| bbox_distance(&me, &self.cached_bbox(&ix, h));
        let nearest = |mut it: InstIter| {
            let mut best = NO_DISTANCE;
            while let Some(h) = it.next(&self.insts) {
                if h != cx.this {
                    let d = to(h);
                    if d < best {
                        best = d;
                    }
                }
            }
            best
        };
        match obj {
            OTHER if cx.other != cx.this => to(cx.other),
            ALL => nearest(self.insts.iter_drawing()),
            o if o <= FIRST_INSTANCE_ID => match u32::try_from(o) {
                Ok(o) => nearest(self.insts.iter_ancestor(ObjectId(o))),
                Err(_) => NO_DISTANCE,
            },
            id => match self.insts.by_id(id) {
                Some(h) if h != cx.this => to(h),
                _ => NO_DISTANCE,
            },
        }
    }

    /// `instance_nearest(x, y, obj)`: the id of the active instance of `obj` (`all` in draw order, or an
    /// object with its descendants in list order) whose position, not its bounding box, is closest to
    /// (x, y); or `noone`. The comparison is on squared distances and strict, so of equally near
    /// instances the first wins, and it starts from 10^10, so an instance 100000 pixels or more away
    /// (or at a NaN distance) is never found. Every other `obj`, `self`, `other` and instance ids (100000
    /// included) among them, finds nothing. OpenGMK `gml/kernel.rs` (`instance_nearest`, "GML default").
    pub fn instance_nearest(&self, x: f64, y: f64, obj: i32) -> i32 {
        let mut it = match obj {
            ALL => self.insts.iter_drawing(),
            o if (0..FIRST_INSTANCE_ID).contains(&o) => {
                self.insts.iter_ancestor(ObjectId(o as u32))
            }
            _ => return NOONE,
        };
        let mut best = 10_000_000_000.0;
        let mut nearest = None;
        while let Some(h) = it.next(&self.insts) {
            let inst = self.insts.get(h);
            let (dx, dy) = (inst.x - x, inst.y - y);
            let d = dx * dx + dy * dy;
            if d < best {
                best = d;
                nearest = Some(h);
            }
        }
        self.id_or_noone(nearest)
    }

    /// `move_snap(hsnap, vsnap)`: rounds `self`'s position to a multiple of each positive snap (half to
    /// even). OpenGMK `gml/kernel.rs` (`move_snap`). The new position goes through the position setter, whose
    /// bounding box can trap.
    pub fn move_snap(&mut self, cx: &Ctx, hsnap: f64, vsnap: f64) -> R<()> {
        let i = self.insts.get(cx.this);
        let (mut x, mut y) = (i.x, i.y);
        if hsnap > 0.0 {
            x = math::round_half_even(x / hsnap) * hsnap;
        }
        if vsnap > 0.0 {
            y = math::round_half_even(y / vsnap) * vsnap;
        }
        self.set_position(cx.this, x, y)
    }

    /// The collision event stage (stage 10), the default [`StageFn`](crate::StageFn) for collisions.
    /// OpenGMK `game/events.rs` (`run_collisions`). The loops nest as: each object `a` with collision
    /// pairs, ascending; each instance `i` of exactly `a`, in list order; each object `b` paired with `a`
    /// (`b >= a`), ascending; each instance `j` of exactly `b`, in list order. If `i` and `j` collide, `i`'s
    /// event runs with `other = j`, then `j`'s with `other = i`. So all of `i`'s meetings, across every
    /// `b`, come before the next instance of `a`'s.
    ///
    /// The solid rule: if either instance is solid, both go back to `xprevious`/`yprevious` before the
    /// events. Afterwards, if either is (still) solid, both move by `hspeed`/`vspeed` again, and if both
    /// are still active and still colliding, both go back again. The move-backs happen even while a room
    /// change is pending (when no events run). `j`'s event runs even if `i`'s destroyed `j`. Once `i` is
    /// no longer active after a meeting, it meets nothing more this frame.
    pub fn run_collisions(&mut self) -> R<()> {
        let program = self.program;
        // Positions in a target list that may collide with the current instance; see `jumping`.
        let mut skip_to = Vec::new();
        for (a, bs) in program.collision_pairs() {
            let mut it = self.insts.iter_object(a);
            'instance: while let Some(i) = it.next(&self.insts) {
                // Without a mask `i` meets nothing, and nothing changes until a meeting: skip it.
                if !self.can_collide(i) {
                    continue;
                }
                // `i` only changes in a meeting, so its body is built once and again after each meeting.
                let mut me = self.body(&self.index(), i);
                for &b in bs {
                    let mut targets = self.insts.iter_object(b);
                    // A shortcut through the spatial index for long target lists: until the first meeting
                    // (before which nothing changes), only the instances whose boxes overlap `i`'s can
                    // collide with it, and visiting the others does nothing. So jump from one to the next,
                    // in list order; after a meeting, walk the list as GM8 does.
                    let mut jumping = !is_short(self.insts.object_len(b))
                        && self.overlapping_in(&me, i, b, &mut skip_to);
                    let mut next_jump = 0;
                    loop {
                        if jumping {
                            let Some(&pos) = skip_to.get(next_jump) else {
                                break;
                            };
                            next_jump += 1;
                            targets.seek(pos);
                        }
                        let Some(j) = targets.next(&self.insts) else {
                            break;
                        };
                        if i == j || !self.meets(&self.index(), &me, j) {
                            continue;
                        }
                        jumping = false;
                        if self.either_solid(i, j) {
                            self.move_back(i)?;
                            self.move_back(j)?;
                        }
                        if let Some(&h) = program.collision_handler(a, b) {
                            self.run_handler(i, j, h)?;
                        }
                        if let Some(&h) = program.collision_handler(b, a) {
                            self.run_handler(j, i, h)?;
                        }
                        if self.either_solid(i, j) {
                            self.apply_speeds(i)?;
                            self.apply_speeds(j)?;
                            let active = |h| self.insts.get(h).is_active();
                            if active(i) && active(j) && self.instances_collide(i, j) {
                                self.move_back(i)?;
                                self.move_back(j)?;
                            }
                        }
                        if !self.insts.get(i).is_active() {
                            continue 'instance;
                        }
                        me = self.body(&self.index(), i);
                    }
                }
            }
        }
        Ok(())
    }

    /// Fills `out` with the positions in object `b`'s list of its active instances whose boxes overlap
    /// `me` (instance `i`'s), ascending. False, leaving the list to be walked, if `i`'s box covers too many
    /// cells to be worth it.
    fn overlapping_in(&self, me: &Body, i: Inst, b: ObjectId, out: &mut Vec<usize>) -> bool {
        out.clear();
        let ix = self.index();
        let wide = |v: i32| i64::from(v);
        let m = me.bbox;
        let cells = ix.cells(wide(m.left), wide(m.top), wide(m.right), wide(m.bottom));
        let Some(cells) = cells.filter(|_| !ix.is_big(i)) else {
            return false;
        };
        ix.candidates(cells, |j| {
            let inst = self.insts.get(j);
            if inst.object == b && inst.is_active() && overlap(&m, &self.cached_bbox(&ix, j)) {
                out.push(self.insts.object_position(b, j));
            }
        });
        out.sort_unstable();
        true
    }

    fn either_solid(&self, i: Inst, j: Inst) -> bool {
        self.insts.get(i).solid || self.insts.get(j).solid
    }

    /// Back to `xprevious`/`yprevious`, through the position setter, whose bounding box can trap.
    fn move_back(&mut self, h: Inst) -> R<()> {
        let i = self.insts.get(h);
        self.set_position(h, i.xprevious, i.yprevious)
    }
}

/// The distance between two bounding boxes: per axis, the gap between them (0 if they overlap), then the
/// hypotenuse. OpenGMK `gml/kernel.rs` (`distance_to_object`, `instance_distance`). The gaps are i32
/// differences, which wrap in OpenGMK's release build: boxes more than 2^31 pixels apart on one axis are
/// a negative distance on that axis, returned as is when the other gap is 0 (so it wins the nearest-of
/// comparison), and squared away otherwise.
pub(crate) fn bbox_distance(a: &Bbox, b: &Bbox) -> f64 {
    let gap = |lo1: i32, hi1: i32, lo2: i32, hi2: i32| {
        // i32 arithmetic, wrapping as in OpenGMK's release build.
        if lo1 > hi2 {
            lo1.wrapping_sub(hi2)
        } else if lo2 > hi1 {
            lo2.wrapping_sub(hi1)
        } else {
            0
        }
    };
    let dx = gap(a.left, a.right, b.left, b.right);
    let dy = gap(a.top, a.bottom, b.top, b.bottom);
    match (dx, dy) {
        (0, 0) => 0.0,
        (d, 0) | (0, d) => f64::from(d),
        (dx, dy) => math::hypot(f64::from(dx), f64::from(dy)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mask() -> assets::Mask {
        assets::Mask {
            bbox: assets::Bbox {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
            data: assets::MaskData::Filled,
        }
    }

    fn sprite(masks: usize) -> assets::Sprite {
        assets::Sprite {
            name: "s".into(),
            width: 1,
            height: 1,
            origin_x: 0,
            origin_y: 0,
            frames: Vec::new(),
            shape: assets::MaskShape::Precise,
            bbox_mode: assets::BboxMode::Automatic,
            alpha_tolerance: 0,
            separate_masks: true,
            masks: (0..masks)
                .map(|i| assets::Mask {
                    bbox: assets::Bbox {
                        left: i as u32,
                        ..mask().bbox
                    },
                    ..mask()
                })
                .collect(),
        }
    }

    #[test]
    fn negative_frames_pick_different_masks_per_test() {
        let s = sprite(3);
        let frame = |index: f64, euclid: bool| collider(&s, index, euclid).unwrap().bbox.left;
        assert_eq!((frame(4.9, true), frame(4.9, false)), (1, 1));
        // Instance tests count back from the last frame; the others clamp to frame 0.
        assert_eq!((frame(-0.5, true), frame(-0.5, false)), (2, 0));
        assert_eq!((frame(f64::NAN, true), frame(f64::NAN, false)), (0, 0));
        assert!(collider(&sprite(0), 0.0, true).is_none());
    }

    #[test]
    fn the_integer_transform_matches_the_float_one() {
        // A small LCG: deterministic inputs on every platform.
        let mut state = 0x2545_f491_4f6c_dd1du64;
        let mut next = |range: i32| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            ((state >> 33) as i64 % i64::from(range)) as i32 - range / 2
        };
        let (s, m) = (sprite(1), mask());
        for _ in 0..20_000 {
            let body = Body {
                bbox: Bbox::default(),
                sprite: 0,
                x: f64::from(next(4000)) / 4.0,
                y: f64::from(next(4000)) / 4.0,
                xscale: if next(2) < 0 { -1.0 } else { 1.0 },
                yscale: if next(2) < 0 { -1.0 } else { 1.0 },
                angle: 0.0,
                image_index: 0.0,
            };
            let fast = Xform::new(&body, &s, &m);
            assert!(fast.unit.is_some());
            let slow = Xform { unit: None, ..fast };
            let fast = Xform {
                ox: next(100),
                oy: next(100),
                ..fast
            };
            let slow = Xform {
                ox: fast.ox,
                oy: fast.oy,
                ..slow
            };
            let (x, y) = (next(6000), next(6000));
            assert_eq!(fast.map(x, y), slow.map(x, y), "{body:?} at ({x}, {y})");
        }
    }
}
