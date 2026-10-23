//! One instance: GM8's typed built-in fields plus user-variable slots (runtime.md section 3). Field
//! defaults and the motion setters follow OpenGMK `instance.rs`.

use crate::math;
use crate::program::{ObjectId, ObjectInfo};
use crate::var::{Slot, Var};

/// Whether an instance takes part in the game. OpenGMK `instance.rs` (`InstanceState`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Active,
    /// Deactivated (`instance_deactivate_*`): keeps its state but runs no events and can't be found.
    Inactive,
    /// Destroyed this frame; removed from the lists at the end-of-step cleanup.
    Deleted,
}

/// GM8's bounding box: inclusive integer edges.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Bbox {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// What `bbox_*` read when an instance has no mask sprite. OpenGMK `instance.rs` (`BBOX_DEFAULT`).
pub const BBOX_DEFAULT: i32 = -100000;

/// The number of alarms per instance.
pub const ALARMS: usize = 12;

/// One instance. Read the fields freely; write through [`World::set`](crate::World::set) (or the setters
/// here) so side effects like the speed/direction coupling happen.
#[derive(Clone, Debug)]
pub struct Instance {
    pub(crate) state: State,
    pub id: i32,
    pub object: ObjectId,
    pub x: f64,
    pub y: f64,
    pub xprevious: f64,
    pub yprevious: f64,
    pub xstart: f64,
    pub ystart: f64,
    pub hspeed: f64,
    pub vspeed: f64,
    pub speed: f64,
    pub direction: f64,
    pub friction: f64,
    pub gravity: f64,
    pub gravity_direction: f64,
    pub solid: bool,
    pub visible: bool,
    pub persistent: bool,
    pub depth: f64,
    /// -1 for none.
    pub sprite_index: i32,
    /// -1 to use the sprite.
    pub mask_index: i32,
    pub image_index: f64,
    pub image_speed: f64,
    pub image_xscale: f64,
    pub image_yscale: f64,
    pub image_angle: f64,
    pub image_alpha: f64,
    pub image_blend: i32,
    /// -1 is off.
    pub alarms: [i32; ALARMS],
    /// User variables in the object's layout.
    pub(crate) vars: Box<[Slot]>,
    /// User variables outside the layout (written through dynamic instance ids the generator couldn't
    /// see). In first-write order.
    pub(crate) extra: Vec<(Var, Slot)>,
}

impl Instance {
    /// A new active instance of `object` at (x, y), with `slots` user-variable slots. OpenGMK
    /// `instance.rs` (`Instance::new_ext`).
    pub(crate) fn new(
        id: i32,
        x: f64,
        y: f64,
        object: ObjectId,
        info: &ObjectInfo,
        slots: usize,
    ) -> Instance {
        Instance {
            state: State::Active,
            id,
            object,
            x,
            y,
            xprevious: x,
            yprevious: y,
            xstart: x,
            ystart: y,
            hspeed: 0.0,
            vspeed: 0.0,
            speed: 0.0,
            direction: 0.0,
            friction: 0.0,
            gravity: 0.0,
            gravity_direction: 270.0,
            solid: info.solid,
            visible: info.visible,
            persistent: info.persistent,
            depth: info.depth,
            sprite_index: info.sprite,
            mask_index: info.mask,
            image_index: 0.0,
            image_speed: 1.0,
            image_xscale: 1.0,
            image_yscale: 1.0,
            image_angle: 0.0,
            image_alpha: 1.0,
            image_blend: 0xFF_FFFF,
            alarms: [-1; ALARMS],
            vars: vec![Slot::Unset; slots].into_boxed_slice(),
            extra: Vec::new(),
        }
    }

    #[inline]
    pub fn state(&self) -> State {
        self.state
    }

    #[inline]
    pub fn is_active(&self) -> bool {
        self.state == State::Active
    }

    /// `direction = d`: wraps to [0, 360) and updates hspeed/vspeed.
    pub fn set_direction(&mut self, d: f64) {
        self.direction = d.rem_euclid(360.0);
        self.update_hvspeed();
    }

    /// `speed = s`: updates hspeed/vspeed.
    pub fn set_speed(&mut self, s: f64) {
        self.speed = s;
        self.update_hvspeed();
    }

    /// `hspeed = h`: updates speed/direction, but only if it changed.
    pub fn set_hspeed(&mut self, h: f64) {
        if self.hspeed != h {
            self.hspeed = h;
            self.update_speed_direction();
        }
    }

    /// `vspeed = v`: updates speed/direction, but only if it changed.
    pub fn set_vspeed(&mut self, v: f64) {
        if self.vspeed != v {
            self.vspeed = v;
            self.update_speed_direction();
        }
    }

    /// Both components at once (gravity), updating speed/direction once if either changed.
    pub fn set_hvspeed(&mut self, h: f64, v: f64) {
        if self.hspeed != h || self.vspeed != v {
            self.hspeed = h;
            self.vspeed = v;
            self.update_speed_direction();
        }
    }

    /// GM8 snaps a component to the nearest integer when it is within 0.0001 of it, so `direction = 90`
    /// gives `hspeed == 0` exactly. OpenGMK `instance.rs` (`update_hvspeed`).
    fn update_hvspeed(&mut self) {
        let snap = |v: f64| {
            let r = math::round(v);
            if (r - v).abs() < 0.0001 { r } else { v }
        };
        let rad = self.direction.to_radians();
        self.hspeed = snap(math::cos(rad) * self.speed);
        self.vspeed = snap(-math::sin(rad) * self.speed);
    }

    /// OpenGMK `instance.rs` (`update_speed_direction`).
    fn update_speed_direction(&mut self) {
        let d = math::atan2(-self.vspeed, self.hspeed)
            .to_degrees()
            .rem_euclid(360.0);
        let r = math::round(d);
        self.direction = if (d - r).abs() < 0.0001 { r } else { d };
        self.speed = (self.hspeed * self.hspeed + self.vspeed * self.vspeed).sqrt();
    }

    /// `sprite_index = s` for a sprite with `frames` frames (0 if it doesn't exist): `image_index` resets
    /// only if it's past the new sprite's last frame. OpenGMK `instance.rs` (`set_sprite_index`).
    pub fn set_sprite_index(&mut self, s: i32, frames: usize) {
        self.sprite_index = s;
        if frames as f64 <= self.image_index.floor() {
            self.image_index = 0.0;
        }
    }

    /// The sprite whose mask collides: `mask_index`, or `sprite_index` if that's negative.
    #[inline]
    pub fn mask_sprite(&self) -> i32 {
        if self.mask_index >= 0 {
            self.mask_index
        } else {
            self.sprite_index
        }
    }

    /// The bounding box for a mask sprite with origin `(ox, oy)` and mask box `mask` (the union of its
    /// frames' boxes): scaled, rotated about (x, y), then rounded half to even. OpenGMK `instance.rs`
    /// (`update_bbox`). GM8 caches this and recomputes when x, y, scale, angle or the mask change; every
    /// input is one of those, so computing it on demand gives the same values.
    pub fn bbox_for(&self, ox: i32, oy: i32, mask: Bbox) -> Bbox {
        let (x, y) = (self.x, self.y);
        let (xs, ys) = (self.image_xscale, self.image_yscale);
        let mut tlx = (x - f64::from(ox) * xs) + f64::from(mask.left) * xs;
        let mut tly = (y - f64::from(oy) * ys) + f64::from(mask.top) * ys;
        let mut brx = tlx + f64::from(mask.right + 1 - mask.left) * xs - 1.0;
        let mut bry = tly + f64::from(mask.bottom + 1 - mask.top) * ys - 1.0;
        if xs <= 0.0 {
            std::mem::swap(&mut tlx, &mut brx);
        }
        if ys <= 0.0 {
            std::mem::swap(&mut tly, &mut bry);
        }
        let (mut trx, mut try_) = (brx, tly);
        let (mut blx, mut bly) = (tlx, bry);
        // Always rotated, even by 0: `(p - x) + x` can differ from `p` in the last bit. Unrotated, the
        // rotation is the identity up to the sign of a zero, which the rounding below discards, so it skips
        // the trigonometry.
        let angle = -self.image_angle.to_radians();
        let (sin, cos) = if angle == 0.0 {
            (0.0, 1.0)
        } else {
            (math::sin(angle), math::cos(angle))
        };
        let rotate = |px: &mut f64, py: &mut f64| {
            let (dx, dy) = (*px - x, *py - y);
            *px = (dx * cos - dy * sin) + x;
            *py = (dx * sin + dy * cos) + y;
        };
        rotate(&mut tlx, &mut tly);
        rotate(&mut trx, &mut try_);
        rotate(&mut blx, &mut bly);
        rotate(&mut brx, &mut bry);
        Bbox {
            left: math::to_int(tlx.min(trx.min(blx.min(brx)))),
            right: math::to_int(tlx.max(trx.max(blx.max(brx)))),
            top: math::to_int(tly.min(try_.min(bly.min(bry)))),
            bottom: math::to_int(tly.max(try_.max(bly.max(bry)))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inst() -> Instance {
        Instance::new(100001, 0.0, 0.0, ObjectId(0), &ObjectInfo::new("o"), 0)
    }

    #[test]
    fn speed_and_direction_drive_components() {
        let mut i = inst();
        i.set_speed(2.0);
        assert_eq!((i.hspeed, i.vspeed), (2.0, 0.0));
        i.set_direction(90.0);
        // cos(90 deg) is 6e-17, snapped to 0; y points down, so up is negative vspeed.
        assert_eq!((i.hspeed, i.vspeed), (0.0, -2.0));
        i.set_direction(-90.0);
        assert_eq!(i.direction, 270.0);
        assert_eq!(i.vspeed, 2.0);
    }

    #[test]
    fn components_drive_speed_and_direction() {
        let mut i = inst();
        i.set_hspeed(3.0);
        i.set_vspeed(-4.0);
        assert_eq!(i.speed, 5.0);
        assert!((i.direction - 53.130_102_354_155_98).abs() < 1e-12);
        i.set_hvspeed(-1.0, 0.0);
        assert_eq!((i.speed, i.direction), (1.0, 180.0));
    }

    #[test]
    fn sprite_change_keeps_frame_when_it_fits() {
        let mut i = inst();
        i.image_index = 2.7;
        i.set_sprite_index(5, 3);
        assert_eq!(i.image_index, 2.7);
        i.set_sprite_index(6, 2);
        assert_eq!(i.image_index, 0.0);
    }

    #[test]
    fn bbox_scales_and_flips() {
        let mut i = inst();
        i.x = 10.0;
        i.y = 20.0;
        let mask = Bbox {
            left: 0,
            top: 0,
            right: 15,
            bottom: 15,
        };
        let b = i.bbox_for(8, 8, mask);
        assert_eq!((b.left, b.top, b.right, b.bottom), (2, 12, 17, 27));
        // Flipped, GM8's box is one pixel wider: the `- 1` for the inclusive edge goes the wrong way.
        i.image_xscale = -1.0;
        let b = i.bbox_for(8, 8, mask);
        assert_eq!((b.left, b.right), (1, 18));
    }
}
