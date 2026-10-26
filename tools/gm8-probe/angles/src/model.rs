//! Models of the runner routines the probes measure, each with a choice of degree-to-radian conversion, and
//! the port's own formulas for comparison.
//!
//! The GM8 routines were read from the runner in `Spelunky.exe` (addresses below), only to describe them; no
//! code was copied. Every one of them converts an angle the same way: `angle * Pi / 180`, with Delphi's
//! 80-bit `Pi` and each step rounded to 53 bits ([`Conv::Runner`]). The bounding box and the masks then
//! store `fsin` and `fcos` as doubles; gravity (`motion_add`'s routine) and the view put the 80-bit results
//! straight into their products.

use pxfm::x87::{F80, Precision};

/// A degree-to-radian conversion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Conv {
    /// The runner's: `angle * Pi / 180`, each step rounded to 53 bits, as for `direction`.
    Runner,
    /// `f64::to_radians`: OpenGMK's, and the port's before 851-3262.
    ToRadians,
    /// Delphi's `DegToRad`, GML's `degtorad`: `angle * (Pi / 180)` with an 80-bit constant.
    DegToRad,
}

pub const CONVS: [Conv; 3] = [Conv::Runner, Conv::ToRadians, Conv::DegToRad];

fn f80(x: f64) -> F80 {
    F80::from_f64(x).expect("finite")
}

pub fn radians(c: Conv, deg: f64) -> f64 {
    match c {
        Conv::Runner => f80(deg)
            .mul(F80::PI, Precision::Double)
            .div(F80::from_u64(180), Precision::Double)
            .to_f64(),
        // The model of OpenGMK's conversion; the port no longer uses it.
        #[allow(clippy::disallowed_methods)]
        Conv::ToRadians => deg.to_radians(),
        Conv::DegToRad => pxfm::x87::gm8::degtorad(deg),
    }
}

/// `fsin` and `fcos` of `deg` converted by `c`, at 80 bits.
pub fn sin_cos80(c: Conv, deg: f64) -> (F80, F80) {
    f80(radians(c, deg)).sin_cos()
}

/// The same, stored as doubles.
pub fn sin_cos(c: Conv, deg: f64) -> (f64, f64) {
    let (s, k) = sin_cos80(c, deg);
    (s.to_f64(), k.to_f64())
}

/// Delphi's `Round`: `fistp`, to nearest, ties to even.
pub fn round(v: f64) -> i32 {
    v.round_ties_even() as i64 as i32
}

/// A sprite: its mask's box, origin and size.
#[derive(Clone, Copy, Debug)]
pub struct Sprite {
    pub name: &'static str,
    pub l: i32,
    pub t: i32,
    pub r: i32,
    pub b: i32,
    pub ox: i32,
    pub oy: i32,
    pub w: i32,
    pub h: i32,
    /// Every pixel of the box collides (true of the two the mask probe uses).
    pub full: bool,
}

/// An instance's bounding box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bb {
    pub l: i32,
    pub t: i32,
    pub r: i32,
    pub b: i32,
}

fn min(a: f64, b: f64) -> f64 {
    if a <= b { a } else { b }
}

fn max(a: f64, b: f64) -> f64 {
    if a >= b { a } else { b }
}

/// The offsets of the mask box's edges from the origin, scaled: (x0, x1, y0, y1).
pub fn edges(s: &Sprite, xs: f64, ys: f64) -> (f64, f64, f64, f64) {
    (
        f64::from(s.l.min(s.r) - s.ox) * xs,
        f64::from(s.l.max(s.r) - s.ox + 1) * xs - 1.0,
        f64::from(s.t.min(s.b) - s.oy) * ys,
        f64::from(s.t.max(s.b) - s.oy + 1) * ys - 1.0,
    )
}

/// The products each side of a rotated box adds to the position, before the rounding (see [`gm8_bbox`]):
/// (left, right) for x and (top, bottom) for y, each as (term with the cosine, term with the sine).
pub fn bbox_terms(s: &Sprite, xs: f64, ys: f64, sn: f64, cs: f64) -> [(f64, f64); 4] {
    let (x0, x1, y0, y1) = edges(s, xs, ys);
    [
        (min(cs * x0, cs * x1), min(sn * y0, sn * y1)),
        (max(cs * x0, cs * x1), max(sn * y0, sn * y1)),
        (min(cs * y0, cs * y1), -max(sn * x0, sn * x1)),
        (max(cs * y0, cs * y1), -min(sn * x0, sn * x1)),
    ]
}

/// GM8's bounding box (the routine at 0x4aa958). Unrotated (`angle == 0` exactly):
/// `round((left - ox) * xscale + x)` and `round(((right - ox + 1) * xscale + x) - 1)`, swapped if
/// reversed, and the same for y. Rotated: each side bounds its products separately,
/// `left = round((min(c*x0, c*x1) + x) + min(s*y0, s*y1))` and so on ([`bbox_terms`]), with `s` and `c` the
/// sine and cosine of `angle` stored as doubles; the port's `gm::Instance::bbox_at` since 851-3278. OpenGMK
/// rotates the four corners instead ([`opengmk_bbox`]): the same box in exact arithmetic, but rounded differently.
pub fn gm8_bbox(c: Conv, s: &Sprite, x: f64, y: f64, xs: f64, ys: f64, angle: f64) -> Bb {
    if angle == 0.0 {
        let mut l = round(f64::from(s.l - s.ox) * xs + x);
        let mut r = round((f64::from(s.r - s.ox + 1) * xs + x) - 1.0);
        if r < l {
            std::mem::swap(&mut l, &mut r);
        }
        let mut t = round(f64::from(s.t - s.oy) * ys + y);
        let mut b = round((f64::from(s.b - s.oy + 1) * ys + y) - 1.0);
        if b < t {
            std::mem::swap(&mut t, &mut b);
        }
        return Bb { l, t, r, b };
    }
    let (sn, cs) = sin_cos(c, angle);
    let [l, r, t, b] = bbox_terms(s, xs, ys, sn, cs);
    // `(p + x) - q` for the top and bottom, which bbox_terms negated: the same bits.
    Bb {
        l: round((l.0 + x) + l.1),
        r: round((r.0 + x) + r.1),
        t: round((t.0 + y) + t.1),
        b: round((b.0 + y) + b.1),
    }
}

/// OpenGMK's bounding box (`update_bbox`, and the port's until 851-3278) with conversion `c`.
pub fn opengmk_bbox(c: Conv, s: &Sprite, x: f64, y: f64, xs: f64, ys: f64, angle: f64) -> Bb {
    let mut tlx = (x - f64::from(s.ox) * xs) + f64::from(s.l) * xs;
    let mut tly = (y - f64::from(s.oy) * ys) + f64::from(s.t) * ys;
    let mut brx = tlx + f64::from(s.r + 1 - s.l) * xs - 1.0;
    let mut bry = tly + f64::from(s.b + 1 - s.t) * ys - 1.0;
    if xs <= 0.0 {
        std::mem::swap(&mut tlx, &mut brx);
    }
    if ys <= 0.0 {
        std::mem::swap(&mut tly, &mut bry);
    }
    let (mut trx, mut try_) = (brx, tly);
    let (mut blx, mut bly) = (tlx, bry);
    let (sin, cos) = if angle == 0.0 {
        (0.0, 1.0)
    } else {
        sin_cos(c, -angle)
    };
    let rot = |px: &mut f64, py: &mut f64| {
        let (dx, dy) = (*px - x, *py - y);
        *px = (dx * cos - dy * sin) + x;
        *py = (dx * sin + dy * cos) + y;
    };
    rot(&mut tlx, &mut tly);
    rot(&mut trx, &mut try_);
    rot(&mut blx, &mut bly);
    rot(&mut brx, &mut bry);
    Bb {
        l: round(tlx.min(trx.min(blx.min(brx)))),
        r: round(tlx.max(trx.max(blx.max(brx)))),
        t: round(tly.min(try_.min(bly.min(bry)))),
        b: round(tly.max(try_.max(bly.max(bry)))),
    }
}

/// The mask pixel that room pixel (px, py) falls on, for an instance at the whole position (ix, iy):
/// GM8's point test (0x48a9f4; its instance test, 0x48a5f0, maps each pixel the same way), with `sn` and
/// `cs` the sine and cosine of `-angle` as doubles: `floor((dx*c + dy*s) / xscale + ox)` and
/// `floor((dy*c - dx*s) / yscale + oy)`. OpenGMK's formula is the same but for the conversion. Unrotated
/// (`angle == 0` exactly), `floor(dx / xscale + ox)`.
#[allow(clippy::too_many_arguments)]
pub fn mask_xy(
    s: &Sprite,
    sc: Option<(f64, f64)>,
    ix: i32,
    iy: i32,
    xs: f64,
    ys: f64,
    px: i32,
    py: i32,
) -> (i32, i32) {
    let (dx, dy) = (f64::from(px - ix), f64::from(py - iy));
    match sc {
        None => (
            (dx / xs + f64::from(s.ox)).floor() as i32,
            (dy / ys + f64::from(s.oy)).floor() as i32,
        ),
        Some((sn, cs)) => (
            ((dx * cs + dy * sn) / xs + f64::from(s.ox)).floor() as i32,
            ((dy * cs - dx * sn) / ys + f64::from(s.oy)).floor() as i32,
        ),
    }
}

/// GM8's AddMotion (0x4abfbc), which the step calls for gravity (`AddMotion(gravity_direction, gravity)`)
/// and `motion_add` calls too: `hspeed = cos(dir) * amount + hspeed`, `vspeed = vspeed - sin(dir) * amount`,
/// with the 80-bit sine and cosine going into the products when `x80` (as the runner does), no snap.
pub fn add_motion(c: Conv, x80: bool, h: f64, v: f64, dir: f64, amount: f64) -> (f64, f64) {
    let (s, k) = sin_cos80(c, dir);
    let (s, k) = if x80 {
        (s, k)
    } else {
        (f80(s.to_f64()), f80(k.to_f64()))
    };
    let a = f80(amount);
    (
        k.mul(a, Precision::Double)
            .add(f80(h), Precision::Double)
            .to_f64(),
        f80(v)
            .sub(s.mul(a, Precision::Double), Precision::Double)
            .to_f64(),
    )
}

/// The port's gravity before 851-3262 (and OpenGMK's): `hspeed + cos * gravity`, `vspeed - sin * gravity`
/// in doubles, converted by `to_radians`.
pub fn port_add_motion_before(h: f64, v: f64, dir: f64, amount: f64) -> (f64, f64) {
    let (s, c) = sin_cos(Conv::ToRadians, dir);
    (h + c * amount, v - s * amount)
}

/// A view: its position and size in the room, its port, its angle.
#[derive(Clone, Copy, Debug)]
pub struct View {
    pub xv: f64,
    pub yv: f64,
    pub wv: f64,
    pub hv: f64,
    pub xp: f64,
    pub yp: f64,
    pub wp: f64,
    pub hp: f64,
    pub angle: f64,
}

/// The mouse's offset into the view (before the rotation): `(mx - xport) * wview / wport`, and for y.
pub fn view_offset(v: &View, mx: i32, my: i32) -> (f64, f64) {
    (
        (f64::from(mx) - v.xp) * v.wv / v.wp,
        (f64::from(my) - v.yp) * v.hv / v.hp,
    )
}

/// Whether GM8's mouse transform rotates: `|angle|` at least an 80-bit 0.01.
pub fn view_rotates(angle: f64) -> bool {
    let hundredth = F80::ONE.div(F80::from_u64(100), Precision::Extended);
    let d = hundredth.sub(f80(angle.abs()), Precision::Extended);
    d.is_zero() || d.is_sign_negative()
}

/// GM8's window-to-room transform for the mouse (the routine around 0x49ba4b, behind `mouse_x` and
/// `window_view_mouse_get_x`), before the rounding: unrotated below 0.01 degrees, `(xview + dx, yview + dy)`;
/// otherwise, with `s` and `c` the 80-bit sine and cosine of `-angle` and `dx' = dx - wview / 2`,
/// `x = (c*dx' + (wview/2 + xview)) + s*dy'` and `y = c*dy' + ((hview/2 + yview) - s*dx')`, each product and
/// sum rounded to 53 bits.
pub fn gm8_view_raw(c: Conv, v: &View, mx: i32, my: i32) -> (f64, f64) {
    let (dx, dy) = view_offset(v, mx, my);
    if !view_rotates(v.angle) {
        return (v.xv + dx, v.yv + dy);
    }
    let (s, k) = sin_cos80(c, -v.angle);
    gm8_view_rotated(v, s, k, dx, dy)
}

/// The rotated part of [`gm8_view_raw`].
pub fn gm8_view_rotated(v: &View, s: F80, k: F80, dx: f64, dy: f64) -> (f64, f64) {
    let (dx, dy) = (f80(dx - v.wv / 2.0), f80(dy - v.hv / 2.0));
    let d = Precision::Double;
    let t = k.mul(dx, d).add(f80(v.wv / 2.0 + v.xv), d);
    let x = s.mul(dy, d).add(t, d).to_f64();
    let t = f80(v.hv / 2.0 + v.yv).sub(s.mul(dx, d), d);
    let y = k.mul(dy, d).add(t, d).to_f64();
    (x, y)
}

pub fn gm8_view(c: Conv, v: &View, mx: i32, my: i32) -> (i32, i32) {
    let (x, y) = gm8_view_raw(c, v, mx, my);
    (round(x), round(y))
}

/// OpenGMK's `transform_point` (and the port's `gm::View::to_room` until 851-3279) with conversion `c`.
pub fn opengmk_view(c: Conv, v: &View, mx: i32, my: i32) -> (i32, i32) {
    let mut x = v.xv + v.wv * (f64::from(mx) - v.xp) / v.wp;
    let mut y = v.yv + v.hv * (f64::from(my) - v.yp) / v.hp;
    let (sin, cos) = sin_cos(c, v.angle);
    let (cx, cy) = (v.xv + v.wv / 2.0, v.yv + v.hv / 2.0);
    (x, y) = (x - cx, y - cy);
    (x, y) = (x * cos - y * sin, x * sin + y * cos);
    (round(x + cx), round(y + cy))
}
