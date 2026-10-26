//! The probes' inputs, chosen deterministically: sweeps, and inputs searched for where the candidate
//! conversions give different answers (an edge on a rounding boundary for one conversion and not another).

use crate::model::*;
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// `v` moved `u` steps along the doubles.
fn nudge(mut v: f64, u: i64) -> f64 {
    for _ in 0..u.abs() {
        v = if u > 0 { v.next_up() } else { v.next_down() };
    }
    v
}

/// An exact GML expression for a finite double, using only literals GM8 reads exactly (whole numbers below
/// 2^53 / 10) and divisions by powers of two: `5`, `(3 / 2)`, `((hi * 67108864 + lo) / 1073741824 / ...)`.
pub fn gml(v: f64) -> String {
    if v == 0.0 {
        return "0".into();
    }
    if v < 0.0 {
        return format!("(-{})", gml(-v));
    }
    if v.fract() == 0.0 && v < 1e15 {
        return format!("{}", v as i64);
    }
    for k in 1..=20 {
        let m = v * f64::from(1u32 << k);
        if m.fract() == 0.0 && m < 1e15 {
            return format!("({} / {})", m as i64, 1u64 << k);
        }
    }
    let bits = v.to_bits();
    let exp = ((bits >> 52) & 0x7ff) as i32;
    assert!(exp > 0 && exp < 1075, "{v:e}");
    let m = (bits & ((1 << 52) - 1)) | (1 << 52);
    let (hi, lo) = (m >> 26, m & ((1 << 26) - 1));
    let mut s = format!("(({hi} * 67108864 + {lo})");
    let mut e = exp - 1075;
    while e < 0 {
        let d = (-e).min(30);
        write!(s, " / {}", 1u64 << d).unwrap();
        e += d;
    }
    s.push(')');
    s
}

// ---- Bounding boxes ----

/// The bbox probe's sprites (from `spelunky.gmk`; the probe prints GM8's own origins and boxes): an origin
/// inside the box, a full block at origin 0, and oArrow's, which the game rotates.
pub const BBOX_SPRITES: [Sprite; 3] = [
    Sprite {
        name: "sTunnelLeft",
        l: 2,
        t: 0,
        r: 13,
        b: 15,
        ox: 8,
        oy: 8,
        w: 16,
        h: 16,
        full: true,
    },
    Sprite {
        name: "sMetalBlock",
        l: 0,
        t: 0,
        r: 15,
        b: 15,
        ox: 0,
        oy: 0,
        w: 16,
        h: 16,
        full: true,
    },
    Sprite {
        name: "sArrowRight",
        l: 0,
        t: 3,
        r: 7,
        b: 7,
        ox: 4,
        oy: 4,
        w: 8,
        h: 8,
        full: false,
    },
];

pub const SCALES: [(f64, f64); 5] = [
    (1.0, 1.0),
    (1.5, 1.0),
    (-1.0, 1.0),
    (2.0, 0.5),
    (0.75, -1.25),
];

#[derive(Clone, Copy, Debug)]
pub struct BboxCase {
    pub s: usize,
    pub x: f64,
    pub y: f64,
    pub xs: f64,
    pub ys: f64,
    pub a: f64,
}

pub fn bbox_cases() -> Vec<(String, BboxCase)> {
    let mut out = Vec::new();
    // Every half degree, the sprites and scales in turn, at fractional positions.
    for k in 0..720u32 {
        let (xs, ys) = SCALES[k as usize % 5];
        let c = BboxCase {
            s: (k % 3) as usize,
            x: 100.0 + f64::from(k % 16) / 16.0,
            y: 50.0 + f64::from(k % 13) / 8.0,
            xs,
            ys,
            a: f64::from(k) / 2.0,
        };
        out.push((format!("bbs_{k}"), c));
    }
    // Per sprite and half degree, the first scale (taken in turn) with a position near the origin where the
    // conversions give different boxes.
    let mut n = 0;
    for (si, s) in BBOX_SPRITES.iter().enumerate() {
        for k in 1..720u32 {
            let a = f64::from(k) / 2.0;
            for j in 0..SCALES.len() {
                let (xs, ys) = SCALES[(k as usize + j) % SCALES.len()];
                if let Some((x, y)) = tune_bbox(s, xs, ys, a) {
                    out.push((
                        format!("bbc_{n}"),
                        BboxCase {
                            s: si,
                            x,
                            y,
                            xs,
                            ys,
                            a,
                        },
                    ));
                    n += 1;
                    break;
                }
            }
        }
    }
    out
}

fn bbox_differs(s: &Sprite, x: f64, y: f64, xs: f64, ys: f64, a: f64) -> bool {
    let r = gm8_bbox(Conv::Runner, s, x, y, xs, ys, a);
    r != gm8_bbox(Conv::ToRadians, s, x, y, xs, ys, a)
        || r != gm8_bbox(Conv::DegToRad, s, x, y, xs, ys, a)
}

/// A position, (x, 0) or (0, y), where the conversions give different GM8 boxes: each side is
/// `round((p + pos) + q)`, so aim pos at `n + 1/2 - (p + q)` for the runner's conversion and try the doubles
/// around it.
fn tune_bbox(s: &Sprite, xs: f64, ys: f64, a: f64) -> Option<(f64, f64)> {
    let sc: Vec<(f64, f64)> = CONVS.iter().map(|&c| sin_cos(c, a)).collect();
    if sc.iter().all(|&v| v == sc[0]) {
        return None;
    }
    let (sn, cs) = sc[0];
    let terms = bbox_terms(s, xs, ys, sn, cs);
    for (i, (p, q)) in terms.iter().enumerate() {
        for n in -1..=1 {
            let p0 = (f64::from(n) + 0.5) - (p + q);
            for u in -6i64..=6 {
                let v = nudge(p0, u);
                let (x, y) = if i < 2 { (v, 0.0) } else { (0.0, v) };
                if bbox_differs(s, x, y, xs, ys, a) {
                    return Some((x, y));
                }
            }
        }
    }
    None
}

pub fn bbox_gml(cases: &[(String, BboxCase)]) -> String {
    let mut g = String::from(
        "// Bounding boxes of a rotated, scaled instance (851-3262): bbox_left/top/right/bottom of oScreen with\n\
         // mask_index set, for a sweep of half degrees and for positions where the candidate degree-to-radian\n\
         // conversions (or GM8's formula and OpenGMK's) round a side differently. Generated by the model\n\
         // described in README.md (\"Angles\"). Too long for run-all.\n",
    );
    // The sprites' boxes and origins, as GM8 has them.
    for s in &BBOX_SPRITES {
        let n = s.name;
        writeln!(
            g,
            "out(\"spr_{n}\", string(sprite_get_xoffset({n})) + \" \" + string(sprite_get_yoffset({n})) + \" \" + string(sprite_get_bbox_left({n})) + \" \" + string(sprite_get_bbox_top({n})) + \" \" + string(sprite_get_bbox_right({n})) + \" \" + string(sprite_get_bbox_bottom({n})));"
        )
        .unwrap();
    }
    let mut last = (usize::MAX, f64::NAN, f64::NAN);
    for (name, c) in cases {
        if c.s != last.0 {
            writeln!(g, "mask_index = {};", BBOX_SPRITES[c.s].name).unwrap();
        }
        if c.xs != last.1 {
            writeln!(g, "image_xscale = {};", gml(c.xs)).unwrap();
        }
        if c.ys != last.2 {
            writeln!(g, "image_yscale = {};", gml(c.ys)).unwrap();
        }
        last = (c.s, c.xs, c.ys);
        writeln!(
            g,
            "x = {}; y = {}; image_angle = {};",
            gml(c.x),
            gml(c.y),
            gml(c.a)
        )
        .unwrap();
        writeln!(
            g,
            "out(\"{name}\", string(bbox_left) + \" \" + string(bbox_top) + \" \" + string(bbox_right) + \" \" + string(bbox_bottom));"
        )
        .unwrap();
    }
    g.push_str("image_angle = 0; image_xscale = 1; image_yscale = 1; mask_index = -1;\n");
    g
}

// ---- Masks: collision_point and place_meeting ----

/// The rotated instance's mask and the other instance's, both full 16x16 blocks at origin 0.
pub const ME: Sprite = Sprite {
    name: "sMetalBlock",
    l: 0,
    t: 0,
    r: 15,
    b: 15,
    ox: 0,
    oy: 0,
    w: 16,
    h: 16,
    full: true,
};
pub const OTHER: Sprite = Sprite {
    name: "sBlock",
    l: 0,
    t: 0,
    r: 15,
    b: 15,
    ox: 0,
    oy: 0,
    w: 16,
    h: 16,
    full: true,
};
/// Where the rotated instance stands.
pub const IX: i32 = 100;
pub const IY: i32 = 60;

#[derive(Clone, Copy, Debug)]
pub enum MaskCase {
    /// `collision_point(IX + dx, IY + dy, oScreen, true, false)` on the rotated instance.
    Point {
        xs: f64,
        ys: f64,
        a: f64,
        dx: i32,
        dy: i32,
    },
    /// `place_meeting(IX, IY, oMenu)` with the other instance (unrotated) at (IX + dx, IY + dy).
    Meet {
        xs: f64,
        ys: f64,
        a: f64,
        dx: i32,
        dy: i32,
    },
}

fn solid(s: &Sprite, mx: i32, my: i32) -> bool {
    assert!(s.full);
    mx >= s.l && my >= s.t && mx <= s.r && my <= s.b
}

fn inside(b: &Bb, x: i32, y: i32) -> bool {
    x >= b.l && x <= b.r && y >= b.t && y <= b.b
}

/// A bounding box and a mask transform for each conversion: both the box and the pixels follow it.
pub type BoxFn = fn(Conv, &Sprite, f64, f64, f64, f64, f64) -> Bb;

/// The answer for a mask case with conversion `c`, with GM8's box ([`gm8_bbox`]) or OpenGMK's
/// ([`opengmk_bbox`]): the box first, then each pixel through both masks.
pub fn mask_hit(c: Conv, bbox: BoxFn, case: &MaskCase) -> bool {
    let (xs, ys, a) = match *case {
        MaskCase::Point { xs, ys, a, .. } | MaskCase::Meet { xs, ys, a, .. } => (xs, ys, a),
    };
    let sc = (a != 0.0).then(|| sin_cos(c, -a));
    let b1 = bbox(c, &ME, f64::from(IX), f64::from(IY), xs, ys, a);
    match *case {
        MaskCase::Point { dx, dy, .. } => {
            let (px, py) = (IX + dx, IY + dy);
            inside(&b1, px, py) && {
                let (mx, my) = mask_xy(&ME, sc, IX, IY, xs, ys, px, py);
                solid(&ME, mx, my)
            }
        }
        MaskCase::Meet { dx, dy, .. } => {
            let (ox, oy) = (IX + dx, IY + dy);
            let b2 = bbox(c, &OTHER, f64::from(ox), f64::from(oy), 1.0, 1.0, 0.0);
            for py in b1.t.max(b2.t)..=b1.b.min(b2.b) {
                for px in b1.l.max(b2.l)..=b1.r.min(b2.r) {
                    let (mx, my) = mask_xy(&ME, sc, IX, IY, xs, ys, px, py);
                    let (nx, ny) = mask_xy(&OTHER, None, ox, oy, 1.0, 1.0, px, py);
                    if solid(&ME, mx, my) && solid(&OTHER, nx, ny) {
                        return true;
                    }
                }
            }
            false
        }
    }
}

fn mask_differs(case: &MaskCase) -> bool {
    let r = mask_hit(Conv::Runner, gm8_bbox, case);
    r != mask_hit(Conv::ToRadians, gm8_bbox, case) || r != mask_hit(Conv::DegToRad, gm8_bbox, case)
}

pub const MASK_SCALES: [(f64, f64); 5] =
    [(1.0, 1.0), (2.0, 2.0), (1.5, 0.5), (-1.0, 1.0), (0.5, 3.0)];

/// The half degrees whose sine and cosine (as doubles) depend on the conversion.
fn sensitive(a: f64) -> bool {
    let sc: Vec<(f64, f64)> = CONVS.iter().map(|&c| sin_cos(c, -a)).collect();
    !sc.iter().all(|&v| v == sc[0])
}

pub fn mask_cases() -> Vec<(String, MaskCase)> {
    let mut out = Vec::new();
    // Points around the instance on every half degree, the scales in turn.
    for k in 0..720 {
        let (xs, ys) = MASK_SCALES[k as usize % 5];
        let (dx, dy) = ((k * 7) % 41 - 20, (k * 11) % 37 - 18);
        out.push((
            format!("pts_{k}"),
            MaskCase::Point {
                xs,
                ys,
                a: f64::from(k) / 2.0,
                dx,
                dy,
            },
        ));
    }
    // Whole points and scales where the conversions disagree (rare: only exact sines land on an edge).
    let mut n = 0;
    let mut per = BTreeMap::new();
    for &(xs, ys) in &MASK_SCALES {
        for k in 1..720 {
            let a = f64::from(k) / 2.0;
            if !sensitive(a) {
                continue;
            }
            for dy in -40..=40 {
                for dx in -40..=40 {
                    let case = MaskCase::Point { xs, ys, a, dx, dy };
                    if mask_differs(&case) {
                        let e = per.entry((k, xs.to_bits(), ys.to_bits())).or_insert(0);
                        if *e < 2 {
                            *e += 1;
                            out.push((format!("ptc_{n}"), case));
                            n += 1;
                        }
                    }
                }
            }
        }
    }
    // Scales tuned so that a point maps onto the mask's far edge (column or row 16 against 15) for one
    // conversion and not another: up to two per half degree.
    let mut n = 0;
    for k in 1..720 {
        let a = f64::from(k) / 2.0;
        if !sensitive(a) {
            continue;
        }
        let (sn, cs) = sin_cos(Conv::Runner, -a);
        let mut found = 0;
        'points: for dy in -40..=40 {
            for dx in -40..=40 {
                let (fx, fy) = (f64::from(dx), f64::from(dy));
                for (axis, v) in [(0, fx * cs + fy * sn), (1, fy * cs - fx * sn)] {
                    if v < 6.0 {
                        continue;
                    }
                    let t = v / f64::from(if axis == 0 { ME.w } else { ME.h });
                    for u in -8i64..=8 {
                        let sc = nudge(t, u);
                        let (xs, ys) = if axis == 0 { (sc, 1.0) } else { (1.0, sc) };
                        let case = MaskCase::Point { xs, ys, a, dx, dy };
                        if mask_differs(&case) {
                            out.push((format!("ptt_{n}"), case));
                            n += 1;
                            found += 1;
                            if found >= 2 {
                                break 'points;
                            }
                            break;
                        }
                    }
                }
            }
        }
    }
    // place_meeting with a corner of the other block on each tuned point.
    let tuned: Vec<MaskCase> = out
        .iter()
        .filter(|(n, _)| n.starts_with("ptt"))
        .map(|(_, c)| *c)
        .collect();
    let mut n = 0;
    for c in tuned {
        let MaskCase::Point { xs, ys, a, dx, dy } = c else {
            continue;
        };
        for (ex, ey) in [(0, 0), (-15, 0), (0, -15), (-15, -15)] {
            let case = MaskCase::Meet {
                xs,
                ys,
                a,
                dx: dx + ex,
                dy: dy + ey,
            };
            if mask_differs(&case) {
                out.push((format!("pmt_{n}"), case));
                n += 1;
                break;
            }
        }
    }
    // Whole placements where the conversions disagree, one per half degree and scale.
    let mut n = 0;
    let mut per = BTreeMap::new();
    for &(xs, ys) in &MASK_SCALES {
        for k in 1..720 {
            let a = f64::from(k) / 2.0;
            if !sensitive(a) {
                continue;
            }
            for dy in -48..=48 {
                for dx in -48..=48 {
                    let case = MaskCase::Meet { xs, ys, a, dx, dy };
                    if mask_differs(&case)
                        && per.insert((k, xs.to_bits(), ys.to_bits()), ()).is_none()
                    {
                        out.push((format!("pmc_{n}"), case));
                        n += 1;
                    }
                }
            }
        }
    }
    // place_meeting on every whole degree.
    for k in 0..360 {
        let (xs, ys) = MASK_SCALES[k as usize % 5];
        let (dx, dy) = ((k * 13) % 49 - 24, (k * 17) % 45 - 22);
        out.push((
            format!("pms_{k}"),
            MaskCase::Meet {
                xs,
                ys,
                a: f64::from(k),
                dx,
                dy,
            },
        ));
    }
    out
}

pub fn mask_gml(cases: &[(String, MaskCase)]) -> String {
    let mut g = String::from(
        "// Rotated masks (851-3262): collision_point and place_meeting against oScreen with a rotated, scaled\n\
         // sMetalBlock mask at (100, 60), for a sweep and for the inputs where the candidate degree-to-radian\n\
         // conversions disagree. The other instance is an oMenu (no events) with sBlock's mask, unrotated.\n\
         // Generated by the model described in README.md (\"Angles\"). Too long for run-all.\n",
    );
    writeln!(
        g,
        "var o;\no = instance_create(-1000, -1000, oMenu);\no.mask_index = {};",
        OTHER.name
    )
    .unwrap();
    writeln!(g, "mask_index = {};\nx = {IX};\ny = {IY};", ME.name).unwrap();
    let mut last = (f64::NAN, f64::NAN, f64::NAN);
    for (name, c) in cases {
        let (xs, ys, a) = match *c {
            MaskCase::Point { xs, ys, a, .. } | MaskCase::Meet { xs, ys, a, .. } => (xs, ys, a),
        };
        if (xs, ys, a) != last {
            writeln!(
                g,
                "image_xscale = {}; image_yscale = {}; image_angle = {};",
                gml(xs),
                gml(ys),
                gml(a)
            )
            .unwrap();
            last = (xs, ys, a);
        }
        match *c {
            MaskCase::Point { dx, dy, .. } => writeln!(
                g,
                "out(\"{name}\", string(collision_point({}, {}, oScreen, true, false) != noone));",
                IX + dx,
                IY + dy
            ),
            MaskCase::Meet { dx, dy, .. } => writeln!(
                g,
                "o.x = {}; o.y = {}; out(\"{name}\", string(place_meeting(x, y, oMenu)));",
                IX + dx,
                IY + dy
            ),
        }
        .unwrap();
    }
    g.push_str("with (o) instance_destroy();\nimage_angle = 0; image_xscale = 1; image_yscale = 1; mask_index = -1;\n");
    g
}

// ---- motion_add (gravity's routine) ----

pub fn motion_gml() -> String {
    String::from(
        "// motion_add (851-3262), which is the runner's own routine behind gravity: hspeed and vspeed after\n\
         // motion_add(dir, amount) on every half degree, after hspeed = (k mod 7) / 4 - 0.75 and vspeed =\n\
         // (k mod 5) / 3 - 0.5, amount 0.5 + (k mod 6) / 4. GM8 reads the literal 0.75 as 0.7500000000000001\n\
         // (851-3280), so the speeds motion_add starts from are printed too (ma0h_, ma0v_). Too long for\n\
         // run-all (README.md, \"Angles\").\n\
         var k;\n\
         for (k = 0; k < 720; k += 1) {\n    \
             hspeed = (k mod 7) / 4 - 0.75;\n    \
             vspeed = (k mod 5) / 3 - 0.5;\n    \
             out(\"ma0h_\" + string(k), hspeed);\n    \
             out(\"ma0v_\" + string(k), vspeed);\n    \
             motion_add(k / 2, 0.5 + (k mod 6) / 4);\n    \
             out(\"mah_\" + string(k), hspeed);\n    \
             out(\"mav_\" + string(k), vspeed);\n\
         }\n\
         speed = 0;\n",
    )
}

/// motion_add's direction and amount for line k.
pub fn motion_args(k: u32) -> (f64, f64) {
    (f64::from(k) / 2.0, 0.5 + f64::from(k % 6) / 4.0)
}

// ---- Views ----

#[derive(Clone, Copy, Debug)]
pub struct ViewCase {
    pub v: View,
    pub mx: i32,
    pub my: i32,
}

/// View 0 as the probe sets it up: a 160x120 view at (10, 20) in a 320x240 port, the window's size.
pub fn base_view() -> View {
    View {
        xv: 10.0,
        yv: 20.0,
        wv: 160.0,
        hv: 120.0,
        xp: 0.0,
        yp: 0.0,
        wp: 320.0,
        hp: 240.0,
        angle: 0.0,
    }
}

pub const MICE: [(i32, i32); 6] = [
    (37, 29),
    (0, 0),
    (319, 239),
    (160, 120),
    (201, 77),
    (13, 230),
];

pub fn view_cases() -> Vec<(String, ViewCase)> {
    let mut out = Vec::new();
    // Every half degree, one mouse position.
    for k in 0..720 {
        let v = View {
            angle: f64::from(k) / 2.0,
            ..base_view()
        };
        out.push((
            format!("vws_{k}"),
            ViewCase {
                v,
                mx: MICE[0].0,
                my: MICE[0].1,
            },
        ));
    }
    // GM8 keeps views' positions and sizes as whole numbers, so search whole views and mouse positions for
    // a coordinate on a rounding boundary under one conversion and not another (only exact sines get there).
    let mut n = 0;
    let views: Vec<View> = [
        (10.0, 20.0, 160.0, 120.0),
        (0.0, 0.0, 320.0, 240.0),
        (7.0, 3.0, 200.0, 150.0),
    ]
    .iter()
    .map(|&(xv, yv, wv, hv)| View {
        xv,
        yv,
        wv,
        hv,
        ..base_view()
    })
    .collect();
    for k in 1..720 {
        let a = f64::from(k) / 2.0;
        let sc: Vec<_> = CONVS.iter().map(|&c| sin_cos80(c, -a)).collect();
        if CONVS
            .iter()
            .all(|&c| radians(c, -a) == radians(Conv::Runner, -a))
        {
            continue;
        }
        let mut found = 0;
        'search: for v in &views {
            let v = View { angle: a, ..*v };
            for my in 0..240 {
                for mx in 0..320 {
                    let (dx, dy) = view_offset(&v, mx, my);
                    let r: Vec<(i32, i32)> = sc
                        .iter()
                        .map(|&(s, c)| {
                            let (x, y) = gm8_view_rotated(&v, s, c, dx, dy);
                            (round(x), round(y))
                        })
                        .collect();
                    if r[1] != r[0] || r[2] != r[0] {
                        out.push((format!("vwc_{n}"), ViewCase { v, mx, my }));
                        n += 1;
                        found += 1;
                        if found >= 2 {
                            break 'search;
                        }
                    }
                }
            }
        }
    }
    // Around GM8's 0.01-degree threshold.
    for (i, &a) in [0.005, 0.0099, 0.01, 0.011, -0.0099, -0.01, 359.995, 0.02]
        .iter()
        .enumerate()
    {
        for (j, &(mx, my)) in MICE.iter().enumerate() {
            let v = View {
                angle: a,
                ..base_view()
            };
            out.push((format!("vwt_{i}_{j}"), ViewCase { v, mx, my }));
        }
    }
    out
}

pub fn view_gml(cases: &[(String, ViewCase)]) -> String {
    let mut g = String::from(
        "// Rotated views (851-3262): window_view_mouse_get_x/y(0) for the mouse at a window position, through\n\
         // view 0 rotated by view_angle[0]. GM8 only takes a view's new settings for the mouse after a redraw,\n\
         // hence screen_redraw() before each read. Generated by the model described in README.md (\"Angles\").\n\
         // Too long for run-all.\n\
         view_enabled = true;\nview_visible[0] = true;\n\
         view_xport[0] = 0; view_yport[0] = 0; view_wport[0] = 320; view_hport[0] = 240;\n\
         view_wview[0] = 160; view_hview[0] = 120;\n",
    );
    let mut last_mouse = (i32::MIN, i32::MIN);
    for (name, c) in cases {
        if (c.mx, c.my) != last_mouse {
            writeln!(g, "window_mouse_set({}, {});", c.mx, c.my).unwrap();
            last_mouse = (c.mx, c.my);
        }
        writeln!(
            g,
            "view_xview[0] = {}; view_yview[0] = {}; view_wview[0] = {}; view_hview[0] = {}; view_angle[0] = {}; screen_redraw();",
            gml(c.v.xv),
            gml(c.v.yv),
            gml(c.v.wv),
            gml(c.v.hv),
            gml(c.v.angle)
        )
        .unwrap();
        writeln!(
            g,
            "out(\"{name}\", string(window_view_mouse_get_x(0)) + \" \" + string(window_view_mouse_get_y(0)));"
        )
        .unwrap();
    }
    g.push_str("view_angle[0] = 0;\nview_enabled = false;\nview_visible[0] = false;\n");
    g
}
