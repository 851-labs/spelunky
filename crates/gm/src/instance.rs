//! One instance: GM8's typed built-in fields plus user-variable slots (runtime.md section 3). Field
//! defaults and the motion setters follow OpenGMK `instance.rs`.

use crate::math;
use crate::program::{ObjectId, ObjectInfo};
use crate::value::ValueError;
use crate::var::{Slot, Var};

/// Whether an instance takes part in the game. OpenGMK `instance.rs` (`InstanceState`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

/// The bounding box of an instance without a mask sprite: the single pixel (-100000, -100000). OpenGMK
/// `instance.rs` (`update_bbox` sets `BBOX_DEFAULT` when there is no sprite). `bbox_*` read it, and
/// collision tests use it like any other box, so such an instance is "at" that pixel for non-precise point
/// and rectangle tests and for `distance_to_object`.
pub const NO_MASK_BBOX: Bbox = Bbox {
    left: BBOX_DEFAULT,
    top: BBOX_DEFAULT,
    right: BBOX_DEFAULT,
    bottom: BBOX_DEFAULT,
};

/// The number of alarms per instance.
pub const ALARMS: usize = 12;

/// One instance. Read the fields freely; write through [`World::set`](crate::World::set) (or the setters
/// here) so side effects like the speed/direction coupling happen.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
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

    /// `direction = d`: stores `d` wrapped as GM8's runner wraps it ([`math::wrap_direction`]) and updates
    /// hspeed/vspeed, unless `d` is the direction already there. The error is where GM8 on Intel stops on the
    /// wrap (a quotient past the `Int64` range), with `d` stored unwrapped as under Rosetta.
    ///
    /// Like every motion setter in the runner (Spelunky.exe 0x4ab23c, 0x4ab2a8, 0x4ab2dc, 0x4ab310), it compares
    /// the value written with the one stored and does nothing when they're equal, so writing `speed` or
    /// `direction` back unchanged keeps components that came from `hspeed` and `vspeed` (`divassign-skip`,
    /// `motion-setters`, 851-3280; OpenGMK skips only for `hspeed` and `vspeed`).
    pub fn set_direction(&mut self, d: f64) -> Result<(), ValueError> {
        if self.direction == d {
            return Ok(());
        }
        let wrapped = math::wrap_direction(d);
        self.direction = wrapped.unwrap_or(d);
        self.update_hvspeed();
        wrapped.map(|_| ())
    }

    /// `speed = s`: updates hspeed/vspeed, unless `s` is the speed already there ([`Instance::set_direction`]).
    pub fn set_speed(&mut self, s: f64) {
        if self.speed != s {
            self.speed = s;
            self.update_hvspeed();
        }
    }

    /// `hspeed = h`: updates speed/direction, but only if it changed, and never `vspeed`: GM8's runner doesn't
    /// recompute the other component (`motion-setters`, 851-3280). The error is where GM8 on Intel stops on the
    /// new speed ([`math::speed`]).
    pub fn set_hspeed(&mut self, h: f64) -> Result<(), ValueError> {
        if self.hspeed != h {
            self.hspeed = h;
            self.update_speed_direction()?;
        }
        Ok(())
    }

    /// `vspeed = v`: as [`Instance::set_hspeed`].
    pub fn set_vspeed(&mut self, v: f64) -> Result<(), ValueError> {
        if self.vspeed != v {
            self.vspeed = v;
            self.update_speed_direction()?;
        }
        Ok(())
    }

    /// The runner's AddMotion (Spelunky.exe 0x4abfbc), behind gravity: `amount` towards `dir` added to the
    /// components ([`math::x87::gm8::add_motion`]), then speed and direction recomputed from them, always, even
    /// when neither component changed (an amount too small to move them), where OpenGMK and the port before
    /// 851-3280 skipped it. Gravity is the runner's own arithmetic, which nothing here checks for GM8's traps
    /// (`docs/quirks.md` §7), so a speed whose snap would trap is kept unsnapped, as under Rosetta.
    pub fn add_motion(&mut self, dir: f64, amount: f64) {
        (self.hspeed, self.vspeed) =
            math::x87::gm8::add_motion(self.hspeed, self.vspeed, dir, amount);
        // Unchecked, as above: the speed is stored either way.
        let _ = self.update_speed_direction();
    }

    /// `speed * cos(direction * Pi / 180)` and `-speed * sin(...)` as GM8's x87 computes them
    /// (`x87::gm8::motion`), each snapped to the nearest integer when within 0.0001 of it, so
    /// `direction = 90` gives `hspeed == 0` exactly (and +0, never -0). OpenGMK `instance.rs`
    /// (`update_hvspeed`; the oracle's patch 0015 makes it this).
    fn update_hvspeed(&mut self) {
        (self.hspeed, self.vspeed) = math::x87::gm8::motion(self.direction, self.speed);
    }

    /// The direction as `point_direction` computes it and the speed as `point_distance` does (its squares in
    /// an x87 register), each snapped like `update_hvspeed`'s components (`x87::gm8::direction` and `speed`):
    /// `hspeed = 2^-600` gives speed 0. Where GM8 on Intel stops on the speed's snap (from 2^63 up), both are
    /// stored as under Rosetta (components of 2^1023 give 2^1023 * sqrt(2)) and the error returned. OpenGMK
    /// `instance.rs` (`update_speed_direction`; patch 0015).
    fn update_speed_direction(&mut self) -> Result<(), ValueError> {
        let (h, v) = (self.hspeed, self.vspeed);
        self.direction = math::x87::gm8::direction(h, v);
        self.speed = math::x87::gm8::speed(h, v);
        math::speed_error(self.speed, h, v)
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

    /// The bounding box at (x, y) (the instance's position, or where `instance_place` tests it) for a mask
    /// sprite with origin `(ox, oy)` and mask box `mask` (the union of its frames' boxes), as GM8's runner
    /// computes it (`Spelunky.exe` 0x4aa958, read for 851-3262 and measured on Intel: `angle-bbox`). The
    /// mask's edges are taken from the origin before the scale, `(left - ox) * xscale`, and a rotated box
    /// bounds each side's products separately. OpenGMK (`instance.rs`, `update_bbox`) rotates the four corners
    /// of `(x - ox * xscale) + left * xscale` instead: the same box in exact arithmetic, but a side near a
    /// rounding boundary rounds the other way, unrotated boxes at fractional positions included (851-3278;
    /// the oracle's patch 0018 makes OpenGMK's GM8's). Sides round half to even. GM8 caches the box and recomputes
    /// it when x, y, scale, angle or the mask change; every input is one of those, so computing it on demand
    /// gives the same values. [`Assets::bbox_at`](crate::Assets::bbox_at) picks the mask.
    pub fn bbox_at(&self, x: f64, y: f64, ox: i32, oy: i32, mask: Bbox) -> Bbox {
        let (xs, ys) = (self.image_xscale, self.image_yscale);
        if self.image_angle == 0.0 {
            // One axis: the near edge, and the far one past its last pixel less 1, after the position is
            // added. Swapped when the scale reverses them (or is 0, where the far edge lands 1 before).
            let axis = |near: i32, far: i32, o: i32, s: f64, p: f64| {
                let a = math::to_int(f64::from(near - o) * s + p);
                let b = math::to_int((f64::from(far - o + 1) * s + p) - 1.0);
                if b < a { (b, a) } else { (a, b) }
            };
            let (left, right) = axis(mask.left, mask.right, ox, xs, x);
            let (top, bottom) = axis(mask.top, mask.bottom, oy, ys, y);
            return Bbox {
                left,
                top,
                right,
                bottom,
            };
        }
        // The edges' offsets from the origin, scaled; the far one less 1 before the position is added.
        let edges = |a: i32, b: i32, o: i32, s: f64| {
            (
                f64::from(a.min(b) - o) * s,
                f64::from(a.max(b) - o + 1) * s - 1.0,
            )
        };
        let (x0, x1) = edges(mask.left, mask.right, ox, xs);
        let (y0, y1) = edges(mask.top, mask.bottom, oy, ys);
        // Of `image_angle` itself (the masks' are of `-image_angle`), stored as doubles.
        let (s, c) = math::x87::gm8::sin_cos_degrees(self.image_angle);
        // The model's `<=` and `>=` (`tools/gm8-probe/angles`): `f64::min` differs only for a NaN, which GML on
        // Intel never makes, and in a zero's sign, which the rounding discards.
        let min = |a: f64, b: f64| if a <= b { a } else { b };
        let max = |a: f64, b: f64| if a >= b { a } else { b };
        Bbox {
            left: math::to_int((min(c * x0, c * x1) + x) + min(s * y0, s * y1)),
            top: math::to_int((min(c * y0, c * y1) + y) - max(s * x0, s * x1)),
            right: math::to_int((max(c * x0, c * x1) + x) + max(s * y0, s * y1)),
            bottom: math::to_int((max(c * y0, c * y1) + y) - min(s * x0, s * x1)),
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
        i.set_direction(90.0).unwrap();
        // cos(90 deg) is 6e-17, snapped to 0; y points down, so up is negative vspeed.
        assert_eq!((i.hspeed, i.vspeed), (0.0, -2.0));
        i.set_direction(-90.0).unwrap();
        assert_eq!(i.direction, 270.0);
        assert_eq!(i.vspeed, 2.0);
    }

    #[test]
    fn components_drive_speed_and_direction() {
        let mut i = inst();
        i.set_hspeed(3.0).unwrap();
        i.set_vspeed(-4.0).unwrap();
        assert_eq!(i.speed, 5.0);
        assert!((i.direction - 53.130_102_354_155_98).abs() < 1e-12);
        i.set_hspeed(-1.0).unwrap();
        i.set_vspeed(0.0).unwrap();
        assert_eq!((i.speed, i.direction), (1.0, 180.0));
    }

    #[test]
    fn components_give_a_snapped_speed_squared_in_a_register() {
        // GM8's answers (probes `motion-range`, `motion-snap`, `speed-2p63`; 851-3281).
        let (p, p63) = (
            f64::from_bits(0x7fe0_0000_0000_0000),
            9_223_372_036_854_775_808.0,
        );
        let mut i = inst();
        i.set_hspeed(5.00001).unwrap();
        assert_eq!((i.hspeed, i.speed), (5.00001, 5.0));
        i.set_hspeed(f64::from_bits(0x1a70_0000_0000_0000)).unwrap(); // 2^-600
        assert_eq!(i.speed.to_bits(), 0);
        // From 2^63 up the speed's snap traps, even with each component below it; what's stored is Rosetta's
        // answer, the squares kept in a register.
        i.set_hspeed(0.75 * p63).unwrap();
        assert_eq!(i.set_vspeed(0.75 * p63), Err(ValueError::IntOverflow));
        assert_eq!(i.speed.to_bits(), 0x43e0_f876_ccdf_6cd9); // GM8 under Rosetta
        assert_eq!(i.set_hspeed(p), Err(ValueError::IntOverflow));
        assert_eq!(i.set_vspeed(p), Err(ValueError::IntOverflow));
        assert_eq!(i.speed, p * std::f64::consts::SQRT_2);
        assert_eq!(i.direction, 315.0);
        // Past the largest double, storing the speed overflows first, as `point_distance`'s does.
        assert_eq!(i.set_hspeed(1.5 * p), Err(ValueError::IntOverflow));
        assert_eq!(i.set_vspeed(1.5 * p), Err(ValueError::Overflow));
        assert_eq!(i.speed, f64::INFINITY);
        // Gravity isn't checked.
        (i.hspeed, i.vspeed) = (0.0, 0.0);
        i.add_motion(0.0, p63);
        assert_eq!(i.speed, p63);
    }

    #[test]
    fn motion_setters_skip_the_value_already_there() {
        // GM8's answers (probes `divassign-skip`, `motion-setters`; 851-3280).
        let mut i = inst();
        i.set_hspeed(0.3).unwrap();
        i.set_vspeed(0.7).unwrap();
        // Each component keeps the value written: neither write recomputes the other.
        assert_eq!((i.hspeed, i.vspeed), (0.3, 0.7));
        // Writing speed or direction back changes nothing, though recomputing would.
        i.set_speed(i.speed);
        i.set_direction(i.direction).unwrap();
        assert_eq!((i.hspeed, i.vspeed), (0.3, 0.7));
        i.set_direction(i.direction + 360.0).unwrap();
        assert_eq!(i.hspeed.to_bits(), 0x3fd3_3333_3333_333b);
    }

    #[test]
    fn written_directions_wrap_with_a_32_bit_floor() {
        let mut i = inst();
        i.set_speed(2.0);
        i.set_direction(-360.0).unwrap();
        assert_eq!(i.direction.to_bits(), 0); // +0, where rem_euclid gives -0
        i.set_direction(1000.5).unwrap();
        assert_eq!(i.direction, 280.5);
        // 2^31 * 360 is 0 in 32 bits.
        let d = 360.0 * 2_147_483_648.0 + 90.0;
        i.set_direction(d).unwrap();
        assert_eq!(i.direction, d);
        // From a quotient of 2^63 up Floor traps; the direction is Rosetta's, unwrapped.
        let d = 360.0 * 18_446_744_073_709_551_616.0;
        assert_eq!(i.set_direction(d), Err(ValueError::IntOverflow));
        assert_eq!(i.direction, d);
    }

    #[test]
    fn components_give_0_for_hspeed_0_and_for_a_snapped_360() {
        let mut i = inst();
        i.set_hspeed(1.0).unwrap();
        i.set_hspeed(-0.0).unwrap();
        // atan2 of (-0, -0) would be 180.
        assert_eq!((i.direction.to_bits(), i.speed), (0, 0.0));
        i.set_hspeed(1.0).unwrap();
        i.set_vspeed(1e-8).unwrap();
        assert_eq!(i.direction.to_bits(), 0);
    }

    #[test]
    fn add_motion_always_updates_speed_and_direction() {
        let mut i = inst();
        i.set_speed(1.0 / 3.0);
        i.set_direction(33.0 + 1.0 / 3.0).unwrap();
        let (h, v, d) = (i.hspeed, i.vspeed, i.direction);
        i.add_motion(0.0, 0.0);
        assert_eq!((i.hspeed, i.vspeed), (h, v));
        // GM8's `motion_add(0, 0)` (`motion-setters`, `am_3`).
        assert_eq!(i.direction.to_bits(), 0x4040_aaaa_aaaa_aaac);
        assert_ne!(i.direction, d);
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
        let b = i.bbox_at(i.x, i.y, 8, 8, mask);
        assert_eq!((b.left, b.top, b.right, b.bottom), (2, 12, 17, 27));
        // Flipped, GM8's box is one pixel wider: the `- 1` for the inclusive edge goes the wrong way.
        i.image_xscale = -1.0;
        let b = i.bbox_at(i.x, i.y, 8, 8, mask);
        assert_eq!((b.left, b.right), (1, 18));
    }
}
