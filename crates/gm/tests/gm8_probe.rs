//! The port's answers to the GM8 probes (`tools/gm8-probe/`), checked against the real GM8 runner's (under
//! Wine and Rosetta, `results/gm8-wine/`, and on a real Intel CPU, `results/gm8-intel/`) and the patched
//! oracle's (`results/opengmk/`). The port and the oracle follow GM8 on Intel for every rule the probes
//! pinned (oracle patches 0014 and 0015), so the port must agree with OpenGMK and with GM8 on Intel on every
//! line, and with GM8 under Rosetta on every line but those [`gm8_differs`] lists. Rosetta doesn't trap the
//! x87's exceptions, so where GM8 on Intel (and the port) stops with an error, or `string()` gives `"ERROR"`,
//! the Rosetta line differs too.
//!
//! Each probe line is `name = <bits in hex> <string(value)>` for a real, or `name = "text"` for a string.
//! Only the lines the port has the functions for are recomputed here (the game never calls `tan`, `exp`,
//! `power`, `ln`, `max` and the like, so `gm` doesn't have them). On top of that, `string()` is checked on
//! every real any probe printed, from its bits. GM8 on real Intel hardware is checked line for line by
//! `gm::math`'s own tests (`math/intel.rs`), so that they run on wasm32 too. The angle probes
//! (`probes/x87/angle-*.gml`, 851-3262) are checked at the end, through the port's collision, motion and view
//! code against GM8 on Intel and the patched oracle (patch 0017).

use std::collections::BTreeMap;

use gm::math::{self, is_truthy};
use gm::string::{real_to_string, string_format, string_to_real};
use gm::{Builtin, GmString, Rng, Value, ValueError};

mod common;
#[path = "common/spelunky.rs"]
mod spelunky;

/// Each probe's results from `tools/gm8-probe/results/<runner>/`, by name.
macro_rules! results {
    ($runner:literal) => {
        results!($runner, [
            "numbers", "precision", "rng", "math", "div0", "intdiv0", "mod0", "nonfinite", "strings", "ops",
            "div-negzero", "div-tiny", "divassign-local", "divassign-field", "divassign-builtin",
            "intdiv-frac", "mod-tiny", "sqrt-neg", "exp-overflow", "string-format", "string-format-neg",
            "divassign-str-arg", "divassign-str-arg2", "divassign-str-field", "divassign-skip", "in-range",
            "round-2p63", "or-2p63", "div-2p63", "string-format-nonfinite", "add-overflow", "sub-overflow",
            "mul-overflow", "div-overflow", "cmp-overflow", "underflow", "sqr-overflow", "radtodeg-overflow",
            "pdist-overflow", "real-overflow", "real-inf", "real-nan", "literal-overflow", "int-args",
            "int-args-nonfinite", "chr-2p63", "chr-neg-2p64", "charat-2p63", "color-2p63", "sf-total-2p63",
            "repeat-2p63", "array-2p63", "alarm-2p63", "orassign-local", "orassign-field", "orassign-builtin",
            "orassign-arg", "mulassign-overflow-local", "mulassign-overflow-field", "string-format-big",
            "pdist-range", "pdist-store-overflow", "color-components",
            "mulstr", "mulstr-2p63", "mulstr-2p32", "mulstr-2p31", "mulstr-2p31+1", "mulstr-2p31-empty",
            "motion-range", "motion-snap", "speed-2p63", "speed-store-overflow", "speedwrite-2p63",
            "lengthdir-2p63", "motion-setters", "direction-2p63",
            "speedwrite-neg2p63", "direction-turn-2p63", "direction-radians-2p63", "hspeed-neg2p63",
            "motion-set-2p63", "motion-add-2p63", "lengthdir-y-2p63", "lengthdir-neg2p63", "friction-2p63",
            "gravity-2p63", "friction-snap", "gravity-overflow", "step-trap", "late-speedwrite-2p63", "late-trap",
            "late-scope", "late-room-trap", "late-create-trap", "late-instcreate-trap", "late-draw-trap",
            "late-mulstr", "step-trap-order", "move-overflow", "late-div0", "late-ln0",
            "mulstr-2p27", "mulstr-2p28", "mulstr-2p28+1", "mulstr-19x2p24", "mulstr-39x2p23", "mulstr-5x2p26",
            "mulstr-3x2p27", "mulstr-2p29", "mulstr-2p30",
            "mulstr-2p22-64", "mulstr-2p23-64",
            "anim-trap", "draw-floor-trap", "sprite-floor", "bbox-trap", "bbox-scale", "bbox-place", "bbox-move",
            "bbox-create", "bg-trap", "view-trap",
            "collision-frame", "collision-frame-trap", "view-follow", "view-follow-id", "id-lookups",
            "draw-builtin-trap", "draw-default-trap", "draw-stage-trap",
            "collision-edges", "collision-edges-trap",
            "draw-skips", "surface-missing", "surface-missing-stretched", "deactivation-order",
        ])
    };
    ($runner:literal, [$($probe:literal),* $(,)?]) => {
        &[$((
            $probe,
            include_str!(concat!("../../../tools/gm8-probe/results/", $runner, "/", $probe, ".txt")),
        )),*]
    };
}

/// The patched oracle's answers.
const OPENGMK: &[(&str, &str)] = results!("opengmk");

/// GM8 under Wine and Rosetta.
const GM8: &[(&str, &str)] = results!("gm8-wine");

/// GM8 on a real Intel CPU (851-3247, 851-3259).
const GM8_INTEL: &[(&str, &str)] = results!("gm8-intel");

/// Lines where the port (and the oracle) knowingly differ from GM8 under Rosetta, with why. The port computes
/// the x87 as a real Intel CPU does (`gm::math::x87`, 851-3257), and Rosetta's `fsin` and `fcos` aren't the
/// hardware's (`tools/gm8-probe/README.md`, "Real x87"): it gives the true sine at multiples of pi, where Intel's
/// 66-bit pi shows, and is 1 ulp off on a few percent of small arguments. On Intel, every line agrees.
fn gm8_differs(name: &str) -> Option<&'static str> {
    const X87: &[&str] = &[
        "cos_7",
        "cos_9",
        "cos_53",
        "cos_70",
        "cos_107",
        "sin_pi",
        "sin_2pi",
        "cos_pi/2",
        "sin_1e6",
        "cos_1e6",
        "sin_1e22",
        "lengthdir_x_10_33",
        // Components from a direction (`motion-setters`).
        "v_same_dir+360",
        "h_dir_360x2p40+90",
        // The step's components from a direction (`step-trap`).
        "step_trap_before_h",
    ];
    // Where GM8 on Intel drops the rest of a frame on a trap after the game's start (851-3290), and Rosetta, which
    // doesn't trap, runs it to the end.
    const DROPPED: &[&str] = &[
        "late_trap_x_4",
        "step_trap_v",
        "step_trap_x",
        "step_trap_y",
        "gravity_overflow_hspeed_past_2p1023",
        "gravity_overflow_x_past_2p1023",
        "scope_x_4",
        "scope_core_x_4",
        "scope_core_image_4",
        "scope_log_4",
        "create_trap_bricks",
        "create_trap_game_created",
        "create_trap_bat_created",
        "instcreate_x_3",
        "instcreate_log",
        "instcreate_core_bps",
        "draw_trap_image_3",
        "draw_trap_core_image_3",
        "draw_trap_image_4",
        "draw_trap_core_image_4",
        "order_c2_v",
        "order_c2_x",
        "order_c3_x",
        "order_c3_y",
        "order_c3_v",
        "move_overflow_log_2",
        "move_overflow_log_3",
        // 851-3301: the runner's own arithmetic.
        "anim_c2_2",
        "anim_c3_2",
        "anim_c3_3",
        "draw_floor_log_3",
        "draw_floor_c1_3",
        "draw_floor_c3_3",
        "draw_floor_log_4",
        "draw_floor_c1_4",
        "draw_floor_c3_4",
        "bbox_move_c2_2",
        "bg_x3_2",
        "bg_y3_2",
        "bg_x4_2",
        "bg_y4_2",
        "bg_image_2",
        "bg_y3_3",
        "bg_x4_3",
        "bg_y4_3",
        "bg_image_3",
        "view_y2_2",
        "view_log_2",
        "view_log_3",
        // 851-3305: place_meeting's test traps after the move there, so there's no move back.
        "cft_self_x_6",
        "cft_self_y_6",
        "cft_other_x_7",
        "cft_other_y_7",
    ];
    // Bounding boxes Rosetta computes where Intel traps: a side past the Int64 range is the x87's "integer
    // indefinite", -2^63, 0 in 32 bits, where Intel leaves the mask's own side (851-3301).
    const BOX: &[&str] = &[
        "bbox_2",
        "bbox_3",
        "bbox_4",
        "bbox_5",
        "bbox_scale_2",
        "bbox_place_2",
        "bbox_place_3",
        "bbox_place_4",
        "bbox_place_x_4",
        "bbox_move_2",
        // 851-3305: and so a point in the mask's own box doesn't meet Rosetta's.
        "cft_d_box",
        "cft_pos_imprecise",
    ];
    if BOX.contains(&name) {
        return Some("Rosetta doesn't trap, so it stores the box's integer indefinite");
    }
    if DROPPED.contains(&name) {
        return Some("Rosetta doesn't trap, so it runs the rest of the frame");
    }
    // 851-3304: Rosetta traps none of drawing's own conversions. `draw-builtin-trap`'s "b" follows each draw; in the
    // others, the `core`s' animation differs from the first dropped frame on, and the log after each dropped frame
    // (`draw_default_log_k` and `draw_stage_log_k` show frame k - 1's draws).
    let case = name
        .strip_prefix("draw_")
        .and_then(|k| k.parse::<u32>().ok());
    let drawn = |prefix: &str, logs: &[u32]| {
        let (what, k) = name.strip_prefix(prefix)?.rsplit_once('_')?;
        let k = k.parse::<u32>().ok()?;
        Some(if what == "log" {
            logs.contains(&k)
        } else {
            k >= 3
        })
    };
    if case.is_some_and(|k| DRAW_BUILTIN_TRAPS.iter().any(|&(t, _)| t == k))
        || drawn("draw_default_", &[3, 4, 5, 7, 8, 9, 11, 13, 14, 15, 16]) == Some(true)
        || drawn("draw_stage_", &[3, 4, 6, 7, 10]) == Some(true)
    {
        return Some("Rosetta doesn't trap, so it draws on");
    }
    X87.contains(&name).then_some("Rosetta's x87 trigonometry")
}

/// A GM8 number literal, read by the `gml` lexer as the transpiler reads the game's (GM8's own reading).
fn lit(text: &str) -> f64 {
    match gml::lexer::tokenize(text).unwrap()[0].tok {
        gml::lexer::Tok::Real(v) => v,
        ref t => panic!("{text} is a {t:?}, not a number"),
    }
}

fn b(x: bool) -> f64 {
    if x { 1.0 } else { 0.0 }
}

fn pow2(n: i32) -> f64 {
    let mut v = 1.0;
    for _ in 0..n.unsigned_abs() {
        if n < 0 { v /= 2.0 } else { v *= 2.0 }
    }
    v
}

#[derive(Default)]
struct Lines(BTreeMap<String, String>);

impl Lines {
    fn real(&mut self, name: &str, x: f64) {
        // The probe's `hex` can't tell a NaN from an infinity on GM8.
        let bits = if !x.is_finite() {
            "nonfinite".to_string()
        } else {
            format!("{:016x}", x.to_bits())
        };
        let s = real_to_string(x).to_string();
        self.0.insert(name.to_string(), format!("{bits} {s}"));
    }

    fn text(&mut self, name: &str, s: &str) {
        self.0.insert(name.to_string(), format!("\"{s}\""));
    }

    fn string_of(&mut self, name: &str, x: f64) {
        self.text(name, &real_to_string(x).to_string());
    }

    /// A line the port can stop on with an error, as GM8 on Intel does.
    fn result(&mut self, name: &str, r: Result<f64, ValueError>) {
        match r {
            Ok(x) => self.real(name, x),
            Err(e) => self.error(name, e),
        }
    }

    fn string_of_result(&mut self, name: &str, r: Result<f64, ValueError>) {
        match r {
            Ok(x) => self.string_of(name, x),
            Err(e) => self.error(name, e),
        }
    }

    fn text_result(&mut self, name: &str, r: Result<String, ValueError>) {
        match r {
            Ok(s) => self.text(name, &s),
            Err(e) => self.error(name, e),
        }
    }

    fn error(&mut self, name: &str, e: ValueError) {
        // The variant too: several print GM8's one "Unexpected error" dialog.
        self.0
            .insert(name.to_string(), format!("{ERROR}{e:?}: {e}"));
    }
}

/// How a port line that stops with an error starts.
const ERROR: &str = "error: ";

/// Whether a port line is one where Rosetta, which doesn't trap the x87's exceptions, can't agree: an error
/// (GM8 on Intel stops there), or `string()`'s `"ERROR"` (its `Round` trapped).
fn traps(port: &str) -> bool {
    port.starts_with(ERROR) || port.ends_with(" ERROR") || port == "\"ERROR\""
}

/// Every real line's `string(value)` as the port prints the value its bits give. Lines whose bits are
/// `nonfinite` (the probes' `hex` can't tell a NaN from an infinity) are skipped; the probe functions check
/// those. Under Rosetta (`rosetta`), a value whose `Round` traps on Intel prints as a number, not `"ERROR"`.
fn strings_of_every_real(text: &str, rosetta: bool, wrong: &mut Vec<String>) -> usize {
    let mut checked = 0;
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let (name, value) = line.split_once(" = ").expect("a probe line");
        let Some((bits, printed)) = value.split_once(' ') else {
            continue;
        };
        let Ok(bits) = u64::from_str_radix(bits, 16) else {
            continue;
        };
        let port = real_to_string(f64::from_bits(bits)).to_string();
        if port != printed && !(rosetta && port == "ERROR") {
            wrong.push(format!(
                "{name}: string({bits:016x}): port {port}, probe {printed}"
            ));
        }
        checked += 1;
    }
    checked
}

fn numbers(l: &mut Lines) {
    let h = 0.5;
    let k = pow2(-54);
    l.real("if_0.5", b(is_truthy(h)));
    l.real("if_below_0.5", b(is_truthy(h - k)));
    l.real("if_-0.5", b(is_truthy(-h)));
    l.real("not_0.5", b(!is_truthy(h)));
    l.real("not_below_0.5", b(!is_truthy(h - k)));
    l.real("and_0.5", b(is_truthy(h) && is_truthy(h)));

    let mut eps = 1.0;
    for _ in 0..14 {
        eps /= 10.0;
    }
    l.real("eq_1_1+1e-14", b(math::eq(1.0, 1.0 + eps)));
    l.real("eq_1_1+9e-14", b(math::eq(1.0, 1.0 + 9.0 * eps)));
    l.real("eq_1_1+1.1e-13", b(math::eq(1.0, 1.0 + 11.0 * eps)));
    l.real("eq_1_1+2e-13", b(math::eq(1.0, 1.0 + 20.0 * eps)));
    l.real("lt_1_1+5e-14", b(math::lt(1.0, 1.0 + 5.0 * eps)));
    l.real("le_1+5e-14_1", b(math::le(1.0 + 5.0 * eps, 1.0)));
    l.real("gt_1+5e-14_1", b(math::gt(1.0 + 5.0 * eps, 1.0)));
    l.real("ne_1_1+5e-14", b(math::ne(1.0, 1.0 + 5.0 * eps)));
    l.real(
        "eq_0.1+0.2_0.3",
        b(math::eq(lit("0.1") + lit("0.2"), lit("0.3"))),
    );
    let big = pow2(40);
    l.real("eq_2p40_2p40+1e-4", b(math::eq(big, big + lit("0.0001"))));
    l.real("eq_tiny_0", b(math::eq(eps * eps, 0.0)));

    for (name, x) in [
        ("0.5", 0.5),
        ("1.5", 1.5),
        ("2.5", 2.5),
        ("-0.5", -0.5),
        ("-1.5", -1.5),
        ("-2.5", -2.5),
        ("0.5-2^-54", h - k),
        ("2p31+0.5", lit("2147483648.5")),
        ("2p40+0.5", big + 0.5),
    ] {
        l.result(&format!("round_{name}"), math::round(x));
    }
    for (name, x) in [
        ("2p31-0.5", lit("2147483647.5")),
        ("2p31+0.5", lit("2147483648.5")),
        ("2p32+0.5", lit("4294967296.5")),
        ("2p40+0.5", big + 0.5),
        ("-2p31-0.5", -lit("2147483648.5")),
        ("-2p40-0.5", -big - 0.5),
        ("1e20", big * big * 100.0),
    ] {
        l.result(&format!("floor_{name}"), math::floor(x));
    }
    l.result("ceil_2p40+0.5", math::ceil(big + 0.5));
    l.result("ceil_-0.5", math::ceil(-0.5));
    l.real("frac_2p40+0.25", math::frac(big + 0.25));
    l.real("frac_-1.25", math::frac(-1.25));
    l.real("sign_1e-14", math::sign(eps));
    l.real("abs_-0", math::abs(-0.0));

    let nz = -0.0;
    l.real("neg_zero", nz);
    for (name, x) in [
        ("0.125", lit("0.125")),
        ("0.375", lit("0.375")),
        ("0.625", lit("0.625")),
        ("-0.125", -lit("0.125")),
        ("2.5", 2.5),
        ("-0.5", -0.5),
        ("-0", nz),
        ("-0.004", -lit("0.004")),
        ("-0.005", -lit("0.005")),
        ("0.005", lit("0.005")),
        ("0.015", lit("0.015")),
        ("1.005", lit("1.005")),
        ("2.675", lit("2.675")),
        ("0.995", lit("0.995")),
        ("99.995", lit("99.995")),
        ("1/3", 1.0 / 3.0),
        ("2p40", big),
        ("2p40+0.5", big + 0.5),
        ("1e15", lit("1000000000000000")),
        ("1e15+0.3", lit("1000000000000000") + lit("0.3")),
        ("2p53", big * 8192.0),
        ("1e20", big * big * 100.0),
        ("1e-5", eps * 1e9),
        ("-1e-14", -eps),
    ] {
        l.string_of(&format!("str_{name}"), x);
    }
    l.text("str_int_string", "12|-3");

    for (name, s) in [
        ("12", "12"),
        (" 12 ", " 12 "),
        ("1.5", "1.5"),
        ("-0", "-0"),
        ("empty", ""),
        (".5", ".5"),
        ("+3", "+3"),
        ("0.1", "0.1"),
        ("0.3", "0.3"),
        ("1e3", "1e3"),
    ] {
        l.real(
            &format!("real_{name}"),
            string_to_real(s.as_bytes()).unwrap(),
        );
    }
    l.real("lit_0.3", lit("0.3"));
    l.real("lit_0.1", lit("0.1"));

    l.result("and_5_3", math::bit_and(5.0, 3.0));
    l.result("and_-1_255", math::bit_and(-1.0, 255.0));
    l.result("or_-5_2", math::bit_or(-5.0, 2.0));
    l.result("xor_-5_1", math::bit_xor(-5.0, 1.0));
    l.result("and_3.7_1", math::bit_and(lit("3.7"), 1.0));
    l.result("and_2.5_3", math::bit_and(2.5, 3.0));
    l.result("or_-2.5_0", math::bit_or(-2.5, 0.0));
    l.result("or_2p31_0", math::bit_or(pow2(31), 0.0));
    l.result("or_2p32+5_0", math::bit_or(pow2(32) + 5.0, 0.0));
    l.result("or_2p40_1", math::bit_or(big, 1.0));
    l.result("or_1e20_0", math::bit_or(big * big * 100.0, 0.0));
    l.result("shl_1_31", math::shl(1.0, 31.0));
    l.result("shl_1_32", math::shl(1.0, 32.0));
    l.result("shl_1_40", math::shl(1.0, 40.0));
    l.result("shr_-1_1", math::shr(-1.0, 1.0));
    l.result("shr_2p31_1", math::shr(pow2(31), 1.0));
    l.result("shr_-8_1", math::shr(-8.0, 1.0));

    l.real("div_7_2", math::int_div(7.0, 2.0).unwrap());
    l.real("div_-7_2", math::int_div(-7.0, 2.0).unwrap());
    l.real("div_7_-2", math::int_div(7.0, -2.0).unwrap());
    l.real("div_7.5_2", math::int_div(7.5, 2.0).unwrap());
    l.real("div_2p40+1_3", math::int_div(big + 1.0, 3.0).unwrap());
    l.real("mod_-7_3", math::modulo(-7.0, 3.0).unwrap());
    l.real("mod_7_-3", math::modulo(7.0, -3.0).unwrap());
    l.real("mod_5.5_2", math::modulo(5.5, 2.0).unwrap());
    l.real("mod_-5.5_2", math::modulo(-5.5, 2.0).unwrap());
    l.real("mod_1_0.1", math::modulo(1.0, lit("0.1")).unwrap());
    l.real("mod_2p40+3_7", math::modulo(big + 3.0, 7.0).unwrap());
}

fn precision(l: &mut Lines) {
    // Plain f64 arithmetic: what an FPU rounding to 53 bits gives.
    l.real("pc_2p24_plus_1", 16777216.0 + 1.0);
    l.real("pc_double_rounding_add", 1.0 + (pow2(-53) + pow2(-64)));
    l.real(
        "pc_double_rounding_mul",
        (1.0 + pow2(-26)) * (1.0 + pow2(-27) + pow2(-51)),
    );
    l.real("third", 1.0 / 3.0);
    l.result("sqrt2", math::sqrt(2.0));
    l.real("tenth_plus_fifth", lit("0.1") + lit("0.2"));
    l.real("sin_pi", math::sin(std::f64::consts::PI));
    l.real("pi", std::f64::consts::PI);
}

fn rng(l: &mut Lines) {
    let mut r = Rng::with_seed(0);
    r.set_seed(12345);
    l.real("seed_after_set", f64::from(r.seed()));
    for n in 0..8 {
        l.real(&format!("random_1_{n}"), r.random(1.0));
    }
    l.real("random_100", r.random(100.0));
    l.real("random_-5", r.random(-5.0));
    l.real("seed_after_draws", f64::from(r.seed()));
    r.set_seed(math::to_int(-1.0));
    l.real("seed_-1", f64::from(r.seed()));
    l.real("random_after_-1", r.random(1.0));
    r.set_seed(math::to_int(pow2(32) + 7.0));
    l.real("seed_2p32+7", f64::from(r.seed()));
    r.set_seed(math::to_int(2.5));
    l.real("seed_2.5", f64::from(r.seed()));
}

fn transcendentals(l: &mut Lines) {
    for n in 1..=120 {
        let v = f64::from(n) * 37.0 / 100.0;
        l.real(&format!("sin_{n}"), math::sin(v));
        l.real(&format!("cos_{n}"), math::cos(v));
        l.real(&format!("arctan_{n}"), math::arctan(v));
        l.real(&format!("arctan2_{n}"), math::atan2(v, lit("1.7")));
        l.result(&format!("sqrt_{n}"), math::sqrt(v));
        l.real(
            &format!("pdir_{n}"),
            math::point_direction(0.0, 0.0, f64::from(n), 7.0),
        );
        l.result(
            &format!("pdist_{n}"),
            math::point_distance(0.0, 0.0, v, lit("1.3")),
        );
    }
    let pi = std::f64::consts::PI;
    l.real("sin_pi", math::sin(pi));
    l.real("cos_pi/2", math::cos(pi / 2.0));
    l.real("sin_2pi", math::sin(2.0 * pi));
    l.real("sin_1e6", math::sin(1e6));
    l.real("cos_1e6", math::cos(1e6));
    l.real("sin_1e22", math::sin(1e22));
    l.result("sqr_1.1", math::sqr(lit("1.1")));
    l.real("degtorad_33", math::degtorad(33.0));
    l.result("radtodeg_1", math::radtodeg(1.0));
    l.result("lengthdir_x_10_33", math::lengthdir_x(10.0, 33.0));
    l.result("lengthdir_y_10_33", math::lengthdir_y(10.0, 33.0));
    let mut s = 0.0;
    for _ in 0..1000 {
        s += lit("0.1");
    }
    l.real("sum_0.1_x1000", s);
    let mut p = 1.0;
    for _ in 0..100 {
        p *= lit("1.1");
    }
    l.real("prod_1.1_x100", p);
    let tiny = pow2(-1022);
    l.real("subnormal_0.75", tiny * 0.75);
    l.real("subnormal_div", tiny / 3.0);
}

fn nonfinite(l: &mut Lines) {
    // nonfinite.gml: an infinity from exp(1000) and a NaN from power(-8, 1/3)
    let (pinf, ninf, nan) = (f64::INFINITY, f64::NEG_INFINITY, f64::NAN);
    l.string_of("nf_str_inf", pinf);
    l.string_of("nf_str_-inf", ninf);
    l.string_of("nf_str_nan", nan);
    l.real("nf_nan_eq_nan", b(math::eq(nan, nan)));
    l.real("nf_nan_ne_nan", b(math::ne(nan, nan)));
    l.real("nf_nan_lt_0", b(math::lt(nan, 0.0)));
    l.real("nf_nan_le_0", b(math::le(nan, 0.0)));
    l.real("nf_nan_gt_0", b(math::gt(nan, 0.0)));
    l.real("nf_nan_ge_0", b(math::ge(nan, 0.0)));
    l.real("nf_if_nan", b(is_truthy(nan)));
    l.real("nf_inf_eq_inf", b(math::eq(pinf, pinf)));
    l.real("nf_inf_ne_inf", b(math::ne(pinf, pinf)));
    l.real("nf_inf_gt_1", b(math::gt(pinf, 1.0)));
    l.real("nf_ninf_lt_0", b(math::lt(ninf, 0.0)));
    l.string_of("nf_str_inf_minus_inf", f64::NAN);
    l.string_of_result("nf_round_inf", math::round(pinf));
    l.string_of_result("nf_floor_inf", math::floor(pinf));
    l.result("nf_or_inf_0", math::bit_or(pinf, 0.0));
    l.result("sqrt_-1", math::sqrt(-1.0));
    l.real("exp_1000", pinf);
    l.string_of("str_exp_1000", pinf);
}

fn ops(l: &mut Lines) {
    let (pinf, ninf, nan) = (f64::INFINITY, f64::NEG_INFINITY, f64::NAN);
    for (name, a, x) in [
        ("0_lt_nan", 0.0, nan),
        ("0_le_nan", 0.0, nan),
        ("0_gt_nan", 0.0, nan),
        ("0_ge_nan", 0.0, nan),
        ("0_eq_nan", 0.0, nan),
        ("0_ne_nan", 0.0, nan),
        ("nan_le_nan", nan, nan),
        ("nan_ge_nan", nan, nan),
        ("inf_lt_inf", pinf, pinf),
        ("inf_ge_inf", pinf, pinf),
        ("ninf_le_ninf", ninf, ninf),
        ("inf_gt_ninf", pinf, ninf),
    ] {
        let f = match name.split('_').nth(1).unwrap() {
            "lt" => math::lt,
            "le" => math::le,
            "gt" => math::gt,
            "ge" => math::ge,
            "eq" => math::eq,
            _ => math::ne,
        };
        l.real(&format!("o_{name}"), b(f(a, x)));
    }
    l.real("o_not_nan", b(!is_truthy(nan)));
    l.real("o_nan_and_1", b(is_truthy(nan) && is_truthy(1.0)));
    l.real("o_nan_or_0", b(is_truthy(nan) || is_truthy(0.0)));
    l.real("o_while_nan", b(is_truthy(nan)));
    l.real("o_sign_nan", math::sign(nan));
    l.real("o_abs_nan_ge0", b(math::ge(math::abs(nan), 0.0)));
    l.text(
        "o_nan_eq_str",
        &real_to_string(b(math::eq(nan, 1.0))).to_string(),
    );
    l.result(
        "o_sqrt_-4_eq",
        math::sqrt(-4.0).map(|r| b(math::eq(r, 0.0))),
    );

    let (p31, p32) = (pow2(31), pow2(32));
    let (p62, p63) = (pow2(62), pow2(63));
    let big = p62 * p62;
    for (name, x) in [
        ("2p31-1", p31 - 1.0),
        ("-2p31", -p31),
        ("-2p31+0.5", -p31 + 0.5),
        ("2p32-0.5", p32 - 0.5),
        ("2p32+2p31+0.5", p32 + p31 + 0.5),
        ("2p63", p63),
        ("-2p63", -p63),
        ("2p62+3", p62 + 4096.0),
        ("nan", nan),
        ("ninf", ninf),
        ("-0.5", -0.5),
        ("-0", -0.0),
        ("3", 3.0),
        ("-3", -3.0),
    ] {
        l.result(&format!("o_floor_{name}"), math::floor(x));
    }
    for (name, x) in [
        ("2p31-0.5", p31 - 0.5),
        ("-2p31-0.5", -p31 - 0.5),
        ("2p32+0.5", p32 + 0.5),
        ("nan", nan),
        ("inf", pinf),
        ("3", 3.0),
    ] {
        l.result(&format!("o_ceil_{name}"), math::ceil(x));
    }
    for (name, x) in [
        ("2p32+0.5", p32 + 0.5),
        ("2p62+1", p62 + 1024.0),
        ("2p63", p63),
        ("-2p63", -p63),
        ("2p64", p63 * 2.0),
        ("1e20", big),
        ("nan", nan),
        ("ninf", ninf),
        ("-0.5", -0.5),
        ("-1.5", -1.5),
    ] {
        l.result(&format!("o_round_{name}"), math::round(x));
    }
    for (name, x) in [
        ("2p63+", p63 + 2048.0),
        ("nan", nan),
        ("inf", pinf),
        ("-0.5", -0.5),
    ] {
        l.real(&format!("o_frac_{name}"), math::frac(x));
    }

    for (name, a, x) in [
        ("-7.5_2", -7.5, 2.0),
        ("7_2.5", 7.0, 2.5),
        ("-1_3", -1.0, 3.0),
        ("2p31_1", p31, 1.0),
        ("2p32+5_1", p32 + 5.0, 1.0),
        ("-2p31-1_1", -p31 - 1.0, 1.0),
        ("2p63_1", p63, 1.0),
        ("2p40_0.5", p32 * 256.0, 0.5),
        ("inf_2", pinf, 2.0),
        ("nan_2", nan, 2.0),
        ("2_inf", 2.0, pinf),
    ] {
        l.result(&format!("o_div_{name}"), math::int_div(a, x));
    }
    for (name, a, x) in [
        ("7_2.5", 7.0, 2.5),
        ("-7_2.5", -7.0, 2.5),
        ("7_-2.5", 7.0, -2.5),
        ("7_0.5", 7.0, 0.5),
        ("5.25_0.5", lit("5.25"), 0.5),
        ("2p31+1_2p31", p31 + 1.0, p31),
        ("2p32+5_3", p32 + 5.0, 3.0),
        ("2p63_3", p63, 3.0),
        ("inf_2", pinf, 2.0),
        ("nan_2", nan, 2.0),
        ("2_inf", 2.0, pinf),
        ("0.3_0.1", lit("0.3"), lit("0.1")),
    ] {
        l.result(&format!("o_mod_{name}"), math::modulo(a, x));
    }

    for (name, a, x) in [
        ("and_-0.5_7", -0.5, 7.0),
        ("and_-1.5_7", -1.5, 7.0),
        ("and_-2.5_7", -2.5, 7.0),
        ("and_0.5_1", 0.5, 1.0),
        ("and_1.5_3", 1.5, 3.0),
        ("or_-2p63_0", -p63, 0.0),
        ("or_2p63_0", p63, 0.0),
        ("or_2p63-1024_0", p63 - 1024.0, 0.0),
        ("or_2p64_0", p63 * 2.0, 0.0),
        ("or_nan_0", nan, 0.0),
        ("or_ninf_0", ninf, 0.0),
        ("xor_-1_-1", -1.0, -1.0),
        ("xor_-1_0", -1.0, 0.0),
        ("and_-1_-2p62", -1.0, -p62),
    ] {
        let f = match &name[..name.find('_').unwrap()] {
            "and" => math::bit_and,
            "or" => math::bit_or,
            _ => math::bit_xor,
        };
        l.result(&format!("b_{name}"), f(a, x));
    }
    for (name, x) in [
        ("0", 0.0),
        ("5", 5.0),
        ("-1", -1.0),
        ("2.5", 2.5),
        ("2p63", p63),
    ] {
        l.result(&format!("b_not_{name}"), math::bit_not(x));
    }
    for (name, a, n) in [
        ("1_62", 1.0, 62.0),
        ("1_63", 1.0, 63.0),
        ("1_64", 1.0, 64.0),
        ("1_65", 1.0, 65.0),
        ("3_62", 3.0, 62.0),
        ("1_-1", 1.0, -1.0),
        ("1_100", 1.0, 100.0),
        ("1_255", 1.0, 255.0),
        ("1_256", 1.0, 256.0),
        ("1_257", 1.0, 257.0),
        ("1_0.5", 1.0, 0.5),
        ("1_1.5", 1.0, 1.5),
        ("-1_1", -1.0, 1.0),
        ("5_2.5", 5.0, 2.5),
    ] {
        l.result(&format!("b_shl_{name}"), math::shl(a, n));
    }
    for (name, a, n) in [
        ("-1_0", -1.0, 0.0),
        ("-1_63", -1.0, 63.0),
        ("-1_64", -1.0, 64.0),
        ("-1_65", -1.0, 65.0),
        ("-2_1", -2.0, 1.0),
        ("-1_32", -1.0, 32.0),
        ("-1_-1", -1.0, -1.0),
        ("256_1.5", 256.0, 1.5),
        ("2p62_62", p62, 62.0),
        ("-2p63_63", -p63, 63.0),
        ("-1_300", -1.0, 300.0),
    ] {
        l.result(&format!("b_shr_{name}"), math::shr(a, n));
    }
}

/// The probes that stop on an error, up to the error, and the zero divisors that are no error.
fn zero_divisors(l: &mut Lines) {
    let mut e = 1.0;
    for _ in 0..14 {
        e /= 10.0;
    }
    l.real("div_1_1e-14", math::div(1.0, e).unwrap());
    let mut e = 1.0;
    for _ in 0..300 {
        e /= 10.0;
    }
    l.real("div_1_1e-300", math::div(1.0, e).unwrap());
    l.result("div_1_2^-1074", math::div(1.0, pow2(-1074)));
    for (name, x) in [("7_2.5", 2.5), ("7_0.5", 0.5), ("1_0.3", lit("0.3"))] {
        let a = if name.starts_with('1') { 1.0 } else { 7.0 };
        l.real(&format!("intdiv_{name}"), math::int_div(a, x).unwrap());
    }
    let mut e = 1.0;
    for _ in 0..14 {
        e /= 10.0;
    }
    l.real("intdiv_7_1e-14", math::int_div(7.0, e).unwrap());
    l.real("mod_7_0.5", math::modulo(7.0, 0.5).unwrap());
    l.real("mod_7_1e-14", math::modulo(7.0, e).unwrap());
    // `/=` by zero keeps the old value, on a local, a field and a built-in alike: every `op=` is
    // `lenient(Value::div, ..)`, which keeps it on any error (`transpile/tests/emit.rs`).
    let keep =
        |old: f64| match gm::lenient(Value::div, Value::Real(old), &Value::Real(0.0)).unwrap() {
            Value::Real(x) => x,
            v => panic!("{v:?}"),
        };
    l.real("divassign_local", keep(1.0));
    l.real("divassign_field", keep(1.0));
    l.real("divassign_builtin", keep(5.0));

    // The errors that stopped the other lines.
    assert_eq!(math::div(1.0, 0.0), Err(ValueError::DivisionByZero)); // div0
    assert_eq!(math::div(1.0, -0.0), Err(ValueError::DivisionByZero)); // div-negzero
    assert_eq!(math::int_div(7.0, 0.0), Err(ValueError::DivisionByZero)); // intdiv0
    assert_eq!(math::modulo(7.0, 0.0), Err(ValueError::ModuloByZero)); // mod0
    assert_eq!(math::modulo(7.0, -0.0), Err(ValueError::ModuloByZero)); // mod-tiny
    assert_eq!(ValueError::DivisionByZero.to_string(), "Division by 0.");
    assert_eq!(ValueError::ModuloByZero.to_string(), "Modulo by 0.");
}

fn string_formats(l: &mut Lines) {
    let sf = |x: f64, total: i32, decimals: i32| format!("[{}]", string_format(x, total, decimals));
    let (pinf, nan) = (f64::INFINITY, f64::NAN);
    let mut k = 1.0;
    for _ in 0..14 {
        k /= 10.0;
    }
    let v = 0.125;
    for (name, x, total, decimals) in [
        ("0.125_0_2", v, 0, 2),
        ("0.125_10_2", v, 10, 2),
        ("0.125_0_1", v, 0, 1),
        ("0.625_0_2", 5.0 * v, 0, 2),
        ("-0.125_0_2", -v, 0, 2),
        ("-0.001_0_2", -lit("0.001"), 0, 2),
        ("-0_0_2", -0.0, 0, 2),
        ("-0_0_0", -0.0, 0, 0),
        ("0.5_0_0", 0.5, 0, 0),
        ("1.5_0_0", 1.5, 0, 0),
        ("2.5_0_0", 2.5, 0, 0),
        ("-0.5_0_0", -0.5, 0, 0),
        ("12_5_0", 12.0, 5, 0),
        ("12_0_2", 12.0, 0, 2),
        ("-12_8_3", -12.0, 8, 3),
        ("1234.5678_3_1", lit("1234.5678"), 3, 1),
        ("1/3_0_5", 1.0 / 3.0, 0, 5),
        ("1/3_0_15", 1.0 / 3.0, 0, 15),
        ("1/3_0_18", 1.0 / 3.0, 0, 18),
        ("1/3_0_25", 1.0 / 3.0, 0, 25),
        ("2/3_0_0", 2.0 / 3.0, 0, 0),
        ("123.456_0_-1", lit("123.456"), 0, -1),
        ("123.456_0_-2", lit("123.456"), 0, -2),
        ("12345678901.5_0_2", lit("12345678901.5"), 0, 2),
        ("123456789012345.5_0_1", lit("123456789012345.5"), 0, 1),
        ("1e15_0_2", lit("1000000000000000"), 0, 2),
        ("1e20_0_2", lit("100000000000000000000"), 0, 2),
        ("1e-5_0_6", k * 1e9, 0, 6),
        ("1e-14_0_20", k, 0, 20),
        ("99.995_0_2", lit("99.995"), 0, 2),
        ("0.015_0_2", lit("0.015"), 0, 2),
        ("123456.785_0_2", lit("123456.785"), 0, 2),
        ("12345678.125_0_2", lit("12345678.125"), 0, 2),
        ("7_-3_2", 7.0, -3, 2),
        ("2.5_1.5_0.5", 2.5, math::to_int(1.5), math::to_int(0.5)),
        ("inf_0_2", pinf, 0, 2),
        ("-inf_6_2", -pinf, 6, 2),
        ("nan_0_2", nan, 0, 2),
    ] {
        l.text(&format!("sf_{name}"), &sf(x, total, decimals));
    }
    l.text(
        "sf_str_5_2",
        &format!("[{}]", Value::from("ab").string_format(5, 2)),
    );
    for (name, x, total, decimals) in [
        ("125_-1", 125.0, 0, -1),
        ("135_-1", 135.0, 0, -1),
        ("-125_-1", -125.0, 0, -1),
        ("5_-1", 5.0, 0, -1),
        ("15_-1", 15.0, 0, -1),
        ("250_-2", 250.0, 0, -2),
        ("50000_-5", 50000.0, 0, -5),
        ("124.9_-1", lit("124.9"), 0, -1),
        ("1234.5_8_-2", 1234.5, 8, -2),
        ("7_0_-20", 7.0, 0, -20),
    ] {
        l.text(&format!("sfn_{name}"), &sf(x, total, decimals));
    }
}

/// The x87's traps that stop GM8 on a real Intel CPU (851-3259): arithmetic past the largest double,
/// integers past the `Int64` range (arguments, array indices, built-in variables, `repeat` counts), those in
/// an `op=`, and `real()` of an infinity; and what goes through: underflow and integers in range.
fn x87_traps(l: &mut Lines) {
    let (p, p63, p62, p32, p31) = (pow2(1023), pow2(63), pow2(62), pow2(32), pow2(31));
    l.result("add_2p1023_2p1022", math::add(p, p / 2.0));
    l.result("add_2p1023_2p1023", math::add(p, p));
    l.result("sub_-2p1023_2p1023", math::sub(-p, p));
    l.result("mul_2p1023_2", math::mul(p, 2.0));
    l.result("div_2p1023_0.5", math::div(p, 0.5));
    l.result(
        "cmp_2p1023_gt_-2p1023",
        Value::Real(p).gml_gt(&Value::Real(-p)).map(b),
    );
    l.result("sqr_2p1023", math::sqr(p));
    l.result("radtodeg_2p1023", math::radtodeg(p));
    l.result("pdist_2p1023", math::point_distance(0.0, 0.0, p, 0.0));
    // `point_distance` squares in an x87 register, so only a distance past the largest double overflows
    // (851-3271: `pdist-range`, `pdist-store-overflow`).
    let (h, q, t) = (pow2(-600), pow2(-540), pow2(-1074));
    for (name, [x1, y1, x2, y2]) in [
        ("pdist_2p1023_2p1023", [0.0, 0.0, p, p]),
        ("pdist_-2p1023", [p, 0.0, 0.0, 0.0]),
        ("pdist_0.75x2p1023_2p1023", [0.0, 0.0, p * 0.75, p]),
        ("pdist_2p-600", [0.0, 0.0, h, 0.0]),
        ("pdist_3x2p-600_2p-600", [0.0, 0.0, 3.0 * h, h]),
        ("pdist_5x2p-540_7x2p-540", [0.0, 0.0, 5.0 * q, 7.0 * q]),
        ("pdist_3x2p-1074_4x2p-1074", [0.0, 0.0, 3.0 * t, 4.0 * t]),
        ("pdist_2p-1074_2p-1074", [0.0, 0.0, t, t]),
        ("pdist_1.5x2p1023_1.5x2p1023", [0.0, 0.0, p * 1.5, p * 1.5]),
    ] {
        l.result(name, math::point_distance(x1, y1, x2, y2));
    }
    l.real("uf_2p-1074", t);
    l.result("uf_2p-1074_div_2", math::div(t, 2.0));
    l.result("uf_2p-1074_sq", math::mul(t, t));
    l.result("uf_3x2p-1074_div_2", math::div(t * 3.0, 2.0));
    l.result("uf_-2p-1074_div_4", math::div(-t, 4.0));
    for (name, s) in [
        ("real_1e400", "1e400"),
        ("real_inf", "inf"),
        ("real_nan", "nan"),
    ] {
        l.result(name, string_to_real(s.as_bytes()));
    }
    l.real("lit_1e309", lit(&format!("1{}", "0".repeat(309))));

    // `ord(chr(n))`: the port has no `chr`, which keeps the low byte of its integer argument.
    let (inf, nan) = (f64::INFINITY, f64::NAN);
    for (name, x) in [
        ("chr_2p63", p63),
        ("chr_-2p64", -2.0 * p63),
        ("chr_inf", inf),
        ("chr_nan", nan),
        ("ia_chr_2p32+65", p32 + 65.0),
        ("ia_chr_65.5", 65.5),
        ("ia_chr_66.5", 66.5),
        ("ia_chr_-2p63", -p63),
        ("ia_chr_2p63-1024", p63 - 1024.0),
    ] {
        l.result(name, math::int_arg(x).map(|n| f64::from(n as u8)));
    }
    let char_at = |s: &str, x: f64| {
        math::int_arg(x).map(|n| gm::string::string_char_at(s.as_bytes(), n).to_string())
    };
    for (name, s, x) in [
        ("charat_2p63", "abc", p63),
        ("charat_inf", "abc", inf),
        ("ia_charat_2p32+2", "abcdef", p32 + 2.0),
        ("ia_charat_2p31+2", "abcdef", p31 + 2.0),
        ("ia_charat_-2p32+3", "abcdef", -p32 + 3.0),
    ] {
        l.text_result(name, char_at(s, x));
    }
    for (name, x) in [
        ("color_2p63", p63),
        ("ia_color_2p32+1", p32 + 1.0),
        ("ia_color_2p62+1024", p62 + 1024.0),
    ] {
        l.result(name, math::make_color_rgb(x, 0.0, 0.0));
    }
    // Components past a byte add, in an Int64 that wraps (851-3270: `color-components`).
    let (p40, p56) = (pow2(40), pow2(56));
    for (name, [r, g, b]) in [
        ("color_255_255_255", [255.0, 255.0, 255.0]),
        ("color_-1_0_0", [-1.0, 0.0, 0.0]),
        ("color_0_-1_0", [0.0, -1.0, 0.0]),
        ("color_0_0_-1", [0.0, 0.0, -1.0]),
        ("color_-1_1_0", [-1.0, 1.0, 0.0]),
        ("color_-1_-1_0", [-1.0, -1.0, 0.0]),
        ("color_-256_1_0", [-256.0, 1.0, 0.0]),
        ("color_256_1_0", [256.0, 1.0, 0.0]),
        ("color_0_256_0", [0.0, 256.0, 0.0]),
        ("color_0_256_1", [0.0, 256.0, 1.0]),
        ("color_1.5_2.5_-0.5", [1.5, 2.5, -0.5]),
        ("color_0_2p32_0", [0.0, p32, 0.0]),
        ("color_0_0_2p40", [0.0, 0.0, p40]),
        ("color_0_0_2p47", [0.0, 0.0, pow2(47)]),
        ("color_0_0_2p48", [0.0, 0.0, pow2(48)]),
        ("color_0_2p56_0", [0.0, p56, 0.0]),
        ("color_1_0_2p48", [1.0, 0.0, pow2(48)]),
        ("color_-1_0_1", [-1.0, 0.0, 1.0]),
        ("color_65536_0_1", [65536.0, 0.0, 1.0]),
    ] {
        l.result(name, math::make_color_rgb(r, g, b));
    }
    l.text_result(
        "sf_1_2p63_2",
        math::int_arg(p63).map(|total| format!("[{}]", string_format(1.0, total, 2))),
    );
    // Each probe stops its loop at 3 or 5 runs.
    let runs = |x: f64, most: u32| match gm::repeat_count(&Value::Real(x)) {
        Ok(n) => Ok(f64::from(n.min(most))),
        Err(gm::Error::Value(e)) => Err(e),
        Err(e) => panic!("{e}"),
    };
    l.result("repeat_2p63", runs(p63, 3));
    l.result("repeat_inf", runs(inf, 3));
    l.result("repeat_nan", runs(nan, 3));
    for (name, x) in [
        ("ia_repeat_2p32+2", p32 + 2.0),
        ("ia_repeat_2p31", p31),
        ("ia_repeat_2p62", p62),
        ("ia_repeat_-2p32+2", -p32 + 2.0),
        ("ia_repeat_-3", -3.0),
        ("ia_repeat_2p32-3", p32 - 3.0),
        ("ia_repeat_-2p31", -p31),
    ] {
        l.result(name, runs(x, 5));
    }
    match gm::ix(&Value::Real(p63)) {
        Err(gm::Error::Value(e)) => l.error("array_2p63", e),
        r => panic!("{r:?}"),
    }
    // `alarm[0] = x` stores the integer argument.
    for (name, x) in [
        ("alarm_2p63", p63),
        ("ia_alarm_2p32+9", p32 + 9.0),
        ("ia_alarm_-2p63", -p63),
    ] {
        l.result(name, math::int_arg(x).map(f64::from));
    }
    // In an `op=`, on every kind of name: the x87's traps aren't type errors, which `op=` ignores.
    let real = |v: Result<Value, ValueError>| v.map(|v| v.as_real().unwrap());
    for name in ["local", "field", "builtin", "arg"] {
        let r = gm::lenient(Value::bit_or, Value::Real(1.0), &Value::Real(p63));
        l.result(&format!("orassign_{name}"), real(r));
    }
    for name in ["local", "field"] {
        let r = gm::lenient(Value::mul, Value::Real(p), &Value::Real(2.0));
        l.result(&format!("mulassign_overflow_{name}"), real(r));
    }

    let sf = |x: f64, decimals: i32| format!("[{}]", string_format(x, 0, decimals));
    let mut v = 1.0;
    for _ in 0..25 {
        v *= 10.0;
    }
    for (name, x, decimals) in [
        ("2p63_0_2", p63, 2),
        ("-2p63_0_0", -p63, 0),
        ("1e25_0_2", v, 2),
        ("1.5e25_0_2", v * 1.5, 2),
        ("2p1023_0_2", p, 2),
        ("2p-1074_0_18", t, 18),
        ("1e-25_0_18", 1.0 / v, 18),
    ] {
        l.text(&format!("sfb_{name}"), &sf(x, decimals));
    }
}

/// `/=` on a string, on an argument and on an instance variable: both keep it, as every `op=` does on an error.
fn divassign_strings(l: &mut Lines) {
    let abc = Value::from("abc");
    let text = |v: Result<Value, ValueError>| match v.unwrap() {
        Value::Str(s) => s.to_string(),
        v => panic!("{v:?}"),
    };
    l.text(
        "divassign_str_arg",
        &text(gm::lenient(Value::div, abc.clone(), &Value::Real(0.0))),
    );
    l.text(
        "divassign_str_arg2",
        &text(gm::lenient(Value::div, abc.clone(), &Value::Real(2.0))),
    );
    l.text(
        "divassign_str_field_0",
        &text(gm::lenient(Value::div, abc.clone(), &Value::Real(0.0))),
    );
    l.text(
        "divassign_str_field_2",
        &text(gm::lenient(Value::div, abc.clone(), &Value::Real(2.0))),
    );
}

/// A runner's lines by name, each with its probe, and the probes an error stopped.
struct Results {
    lines: BTreeMap<String, (&'static str, String)>,
    stopped: Vec<&'static str>,
}

fn results(table: &[(&'static str, &'static str)]) -> Results {
    let mut r = Results {
        lines: BTreeMap::new(),
        stopped: Vec::new(),
    };
    for &(probe, text) in table {
        for line in text.lines() {
            // `#` lines record how a probe stopped.
            if line.starts_with('#') {
                r.stopped.push(probe);
                continue;
            }
            let (name, value) = line.split_once(" = ").expect("a probe line");
            // A line in two probes (`sin_pi`; `in-range` repeats lines of `numbers` and `ops`) has one answer.
            if let Some((other, old)) = r.lines.insert(name.to_string(), (probe, value.to_string()))
            {
                assert_eq!(old, value, "{name} in {other} and {probe}");
            }
        }
    }
    r
}

/// Records a port line that differs from a runner's, or that agrees though `why` says it differs.
fn compare(
    wrong: &mut Vec<String>,
    runner: &str,
    line: (&str, &str),
    port: &str,
    want: &str,
    why: Option<&str>,
) {
    let (probe, name) = line;
    match why {
        None if port != want => {
            wrong.push(format!("{probe}: {name}: port {port}, {runner} {want}"))
        }
        Some(why) if port == want => {
            wrong.push(format!(
                "{probe}: {name}: listed as differing from {runner} ({why}) but agrees"
            ));
        }
        _ => {}
    }
}

/// `real * string`: its count is an integer argument run as `repeat` runs one (851-3272: `mulstr`, `mulstr-*`).
fn mulstr(l: &mut Lines) {
    let (p63, p32, p31) = (pow2(63), pow2(32), pow2(31));
    let mulstr = |n: f64, s: &str| match Value::Real(n).mul(&Value::Str(s.into()))? {
        Value::Str(s) => Ok(String::from_utf8(s.as_bytes().to_vec()).unwrap()),
        v => panic!("{v:?}"),
    };
    for (name, n, s) in [
        ("mulstr_3_ab", 3.0, "ab"),
        ("mulstr_2.5_a", 2.5, "a"),
        ("mulstr_3.5_a", 3.5, "a"),
        ("mulstr_0.5_a", 0.5, "a"),
        ("mulstr_0.6_a", lit("0.6"), "a"),
        ("mulstr_-3_a", -3.0, "a"),
        ("mulstr_1_empty", 1.0, ""),
        ("mulstr_-2p32+3_a", -p32 + 3.0, "a"),
        ("mulstr_2p63_a", p63, "a"),
        ("mulstr_2p32+2_a", p32 + 2.0, "a"),
        ("mulstr_2p31_a", p31, "a"),
        ("mulstr_2p31+1_a", p31 + 1.0, "a"),
        ("mulstr_2p31_empty", p31, ""),
    ] {
        l.text_result(name, mulstr(n, s));
    }
    // `string_length(n * s)` where GM8 builds huge strings or runs out of memory (851-3282), from the length
    // alone: building them here would take gigabytes.
    let wide = "0123456789abcdef".repeat(4);
    for (name, n, s) in [
        ("mulstr_2p27_a_len", pow2(27), "a"),
        ("mulstr_2p28_a_len", pow2(28), "a"),
        ("mulstr_2p28+1_a_len", pow2(28) + 1.0, "a"),
        ("mulstr_19x2p24_a_len", 19.0 * pow2(24), "a"),
        ("mulstr_39x2p23_a_len", 39.0 * pow2(23), "a"),
        ("mulstr_5x2p26_a_len", 5.0 * pow2(26), "a"),
        ("mulstr_3x2p27_a_len", 3.0 * pow2(27), "a"),
        ("mulstr_2p29_a_len", pow2(29), "a"),
        ("mulstr_2p30_a_len", pow2(30), "a"),
        ("mulstr_2p22_64_len", pow2(22), &wide),
        ("mulstr_2p23_64_len", pow2(23), &wide),
    ] {
        let copies = gm::value::repeat_copies(n, s.as_bytes());
        l.result(name, copies.map(|c| (c * s.len()) as f64));
    }
}

/// The runner's motion setters (851-3280, `motion-setters`, `direction-2p63`), statement by statement on one
/// instance through `World::set`, and `motion_add` as the runner's AddMotion (`Instance::add_motion`, which
/// gravity calls): `hspeed` and `vspeed` recompute only `speed` and `direction`, every setter skips a write of
/// the value already there, a written direction wraps with Delphi's 32-bit `Floor`, the update from the
/// components gives 0 for `hspeed` 0 and for a direction snapped to 360, and AddMotion always updates.
fn motion_setters(l: &mut Lines) {
    use Builtin::{Direction as D, Hspeed as H, Speed as S, Vspeed as V};
    let program = common::build(|b| {
        b.push_object(gm::ObjectInfo::new("o"));
    });
    let mut g = common::world(program, &[gm::ObjectId(0)]);
    let me = g.find(common::FIRST_ID).unwrap();
    let set = |g: &mut gm::World, writes: &[(Builtin, f64)]| {
        for &(b, x) in writes {
            g.set(me, b.var(), Value::Real(x)).unwrap();
        }
    };
    let get = |g: &gm::World, b: Builtin| g.get(me, b.var()).unwrap().to_real();
    let read = |g: &gm::World, l: &mut Lines, case: &str| {
        for (v, b) in [("h", H), ("v", V), ("s", S), ("d", D)] {
            l.real(&format!("{v}_{case}"), get(g, b));
        }
    };
    let rest = [(H, 0.0), (V, 0.0)];
    let still = [(S, 0.0), (D, 0.0)];

    // The ticket's case: GM8 reads the literal 0.75 as 0.7500000000000001.
    let n = lit("0.75");
    l.real("lit_0.75", n);
    set(&mut g, &[&rest[..], &[(H, -n), (V, -0.5)]].concat());
    read(&g, l, "lit_-0.75_-0.5");
    set(&mut g, &[&rest[..], &[(H, 0.75 - n), (V, 0.5)]].concat());
    read(&g, l, "lit_0_0.5");

    // Both write orders.
    for k in 0..35 {
        let a = (f64::from(k % 7) - 3.0) / 4.0;
        let b = f64::from(k % 5) / 3.0 - 0.5;
        set(&mut g, &[&rest[..], &[(H, a), (V, b)]].concat());
        read(&g, l, &format!("hv_{k}"));
        set(&mut g, &[&rest[..], &[(V, b), (H, a)]].concat());
        read(&g, l, &format!("vh_{k}"));
    }

    // hspeed 0, with a signed zero.
    for (case, writes) in [
        ("-0_0", [(H, 1.0), (V, 0.0), (H, -0.0)].as_slice()),
        ("-0_-0", &[(H, 1.0), (V, -0.0), (H, -0.0)]),
        ("0_-0", &[(H, 1.0), (V, -0.0), (H, 0.0)]),
        ("-0_1", &[(H, 1.0), (V, 1.0), (H, -0.0)]),
        ("-0_-1", &[(H, 1.0), (V, -1.0), (H, -0.0)]),
        ("1_-0", &[(H, 0.0), (V, 1.0), (H, 1.0), (V, -0.0)]),
        ("-1_-0", &[(H, 0.0), (V, 1.0), (H, -1.0), (V, -0.0)]),
        ("-1_0", &[(H, 0.0), (V, 1.0), (H, -1.0), (V, 0.0)]),
    ] {
        set(&mut g, &[&still[..], writes].concat());
        read(&g, l, &format!("zero_{case}"));
    }

    // A direction snapped to 360.
    for (case, v) in [
        ("1e-8", 1.0 / 100_000_000.0),
        ("1e-6", 1.0 / 1_000_000.0),
        ("1e-3", 1.0 / 1000.0),
        ("-1e-8", -1.0 / 100_000_000.0),
    ] {
        set(&mut g, &[&rest[..], &[(H, 1.0), (V, v)]].concat());
        read(&g, l, &format!("wrap_{case}"));
    }

    // Writing speed or direction back.
    let (h, v) = (3.0 / 10.0, 7.0 / 10.0);
    set(&mut g, &[&rest[..], &[(H, h), (V, v)]].concat());
    read(&g, l, "same_0");
    let s = get(&g, S);
    set(&mut g, &[(S, s)]);
    read(&g, l, "same_speed");
    let d = get(&g, D);
    set(&mut g, &[(D, d)]);
    read(&g, l, "same_dir");
    let d = get(&g, D);
    set(&mut g, &[(D, d + 360.0)]);
    read(&g, l, "same_dir+360");
    set(&mut g, &[&rest[..], &[(H, h), (V, v)]].concat());
    let a = get(&g, S);
    set(&mut g, &[(S, a * 2.0), (S, a)]);
    read(&g, l, "same_speed2");
    set(&mut g, &[&rest[..], &[(H, h), (V, v), (H, h)]].concat());
    read(&g, l, "same_h");
    set(&mut g, &[&still[..], &[(S, 0.0)]].concat());
    read(&g, l, "same_rest");

    // direction = d.
    let p = pow2(31);
    for (case, d) in [
        ("-90", -90.0),
        ("360", 360.0),
        ("720", 720.0),
        ("-360", -360.0),
        ("-0", -0.0),
        ("-2p-60", -1.0 / lit("1152921504606846976")),
        ("1000.5", 1000.5),
        ("-1000.5", -1000.5),
        ("360x2p31+90", 360.0 * p + 90.0),
        ("360x2p31-90", 360.0 * p - 90.0),
        ("-360x2p31-90", -360.0 * p - 90.0),
        ("360x2p32+90", 360.0 * p * 2.0 + 90.0),
        ("360x2p40+90", 360.0 * p * 512.0 + 90.0),
    ] {
        set(&mut g, &[&still[..], &[(S, 2.0), (D, d)]].concat());
        read(&g, l, &format!("dir_{case}"));
    }

    // motion_add, with amounts that don't change the components.
    let tiny = pow2(-60);
    for (i, (s, d)) in [
        (3.0, 45.0 + 3.0 / 10.0),
        (7.0, 10.0 + 1.0 / 10.0),
        (5.0, 200.0 + 7.0 / 10.0),
        (1.0 / 3.0, 33.0 + 1.0 / 3.0),
        (2.0, 123.0 + 4.0 / 10.0),
        (9.0, 300.0 + 1.0 / 7.0),
    ]
    .into_iter()
    .enumerate()
    {
        for (case, amount) in [("am", 0.0), ("amp", tiny)] {
            set(&mut g, &[&still[..], &[(S, s), (D, d)]].concat());
            if case == "am" {
                read(&g, l, &format!("am0_{i}"));
            }
            g.instance_mut(me).add_motion(0.0, amount).unwrap();
            read(&g, l, &format!("{case}_{i}"));
        }
    }

    // direction-2p63: at speed 0, a quotient of 2^63 - 1024 wraps (to 360 * 2^63) and one of 2^64 traps.
    let p63 = pow2(63);
    set(&mut g, &[(S, 0.0), (D, 360.0 * (p63 - 1024.0))]);
    l.real("direction_360x(2p63-1024)", get(&g, D));
    l.real("hspeed_360x(2p63-1024)", get(&g, H));
    match g.set(me, D.var(), Value::Real(360.0 * p63 * 2.0)) {
        Err(gm::Error::Value(e)) => {
            l.error("direction_360x2p64", e);
            l.error("hspeed_360x2p64", e);
        }
        r => panic!("direction = 360 * 2^64: {r:?}"),
    }
}

/// Some writes to an instance's motion variables, then the lines read after them, by name.
type MotionCase = (Vec<(Builtin, f64)>, Vec<(String, Builtin)>);

/// Runs a motion probe's cases on a fresh instance through `World::set`, as the probe writes `hspeed`,
/// `vspeed`, `speed` and `direction`. A write that stops GM8 on Intel is the error on each line of its case
/// (and the rest of the probe's lines, missing on Intel, are Rosetta's).
fn run_motion(l: &mut Lines, cases: &[MotionCase]) {
    let program = common::build(|b| {
        b.push_object(gm::ObjectInfo::new("o"));
    });
    let mut g = common::world(program, &[gm::ObjectId(0)]);
    let h = g.find(common::FIRST_ID).unwrap();
    for (writes, reads) in cases {
        // Every write happens, as under Rosetta, which doesn't trap; the case keeps the first error.
        let mut wrote = Ok(());
        for &(b, x) in writes {
            let r = g.set(h, b.var(), Value::Real(x));
            wrote = wrote.and(r);
        }
        for (name, b) in reads {
            match &wrote {
                Ok(()) => l.real(name, g.get(h, b.var()).unwrap().to_real()),
                Err(gm::Error::Value(e)) => l.error(name, *e),
                Err(e) => panic!("{name}: {e:?}"),
            }
        }
    }
}

/// The runner's `speed` and `direction` after `hspeed` or `vspeed` changes (851-3281: `motion-range`,
/// `motion-snap`, `speed-2p63`, `speed-store-overflow`): the speed is `point_distance`'s, snapped to a whole
/// number within 0.0001, and from 2^63 up the snap's `Round` stops GM8 on Intel.
fn motion(l: &mut Lines) {
    use Builtin::{Direction as D, Hspeed as H, Speed as S, Vspeed as V};
    let (p, g, h, q, t, p63) = (
        pow2(1023),
        pow2(600),
        pow2(-600),
        pow2(-540),
        pow2(-1074),
        pow2(63),
    );
    let from_rest = |w: &[(Builtin, f64)]| [&[(H, 0.0), (V, 0.0)][..], w].concat();
    let reads = |case: &str, vars: &[(&str, Builtin)]| -> Vec<(String, Builtin)> {
        vars.iter()
            .map(|&(v, b)| (format!("{v}_{case}"), b))
            .collect()
    };

    let sd = [("speed", S), ("direction", D)];
    let mut range: Vec<MotionCase> = [
        ("2p-600_0", vec![(H, h)]),
        ("0_2p-600", vec![(V, h)]),
        ("3x2p-600_-4x2p-600", vec![(H, 3.0 * h), (V, -4.0 * h)]),
        ("5x2p-540_7x2p-540", vec![(H, 5.0 * q), (V, 7.0 * q)]),
        ("3x2p-1074_4x2p-1074", vec![(H, 3.0 * t), (V, 4.0 * t)]),
        ("2p-1074_2p-1074", vec![(H, t), (V, t)]),
        ("2p600_0", vec![(H, g)]),
        ("-2p600_2p600", vec![(H, -g), (V, g)]),
        ("2p1023_2p1023", vec![(H, p), (V, p)]),
        ("0.75x2p1023_-2p1023", vec![(H, p * lit("0.75")), (V, -p)]),
    ]
    .iter()
    .map(|(case, w)| (from_rest(w), reads(case, &sd)))
    .collect();
    range.push((
        vec![],
        reads("0.75x2p1023_-2p1023", &[("hspeed", H), ("vspeed", V)]),
    ));
    run_motion(l, &range);

    let all = [("h", H), ("v", V), ("s", S), ("d", D)];
    let n = |text: &str| lit(text);
    let mut snap: Vec<MotionCase> = [
        ("2p-600_0", vec![(H, h)]),
        ("0.00005_0", vec![(H, n("0.00005"))]),
        ("0.0002_0", vec![(H, n("0.0002"))]),
        ("5.00001_0", vec![(H, n("5.00001"))]),
        (
            "3.00003_4.00004",
            vec![(H, n("3.00003")), (V, n("4.00004"))],
        ),
        ("0.3_0.7", vec![(H, n("0.3")), (V, n("0.7"))]),
        ("0.75_-1", vec![(H, n("0.75")), (V, -1.0)]),
        ("1000.3_2000.7", vec![(H, n("1000.3")), (V, n("2000.7"))]),
        (
            "0.3e12_0.7e12",
            vec![(H, n("0.3") * 1e12), (V, n("0.7") * 1e12)],
        ),
        ("0.75x2p600_-2p600", vec![(H, n("0.75") * g), (V, -g)]),
        ("0.75x2p1023_0", vec![(H, n("0.75") * p)]),
        ("0_-2p1023", vec![(V, -p)]),
    ]
    .iter()
    .map(|(case, w)| (from_rest(w), reads(case, &all)))
    .collect();
    // Then `hspeed` written on top of the last case's `vspeed`.
    snap.push((vec![(H, n("0.75") * p)], reads("0.75x2p1023_-2p1023", &all)));
    for (case, x) in [("speed_2p-600", h), ("speed_5.00001", n("5.00001"))] {
        let w = vec![(S, 0.0), (D, 0.0), (S, x)];
        snap.push((w, reads(case, &[("h", H), ("s", S)])));
    }
    run_motion(l, &snap);

    let one = |w: (Builtin, f64), case: &str| (vec![w], reads(case, &[("speed", S)]));
    run_motion(
        l,
        &[
            one((H, p63 - 1024.0), "2p63-1024_0"),
            one((H, 3.0 * p63 / 4.0), "3x2p61_0"),
            one((V, 3.0 * p63 / 4.0), "3x2p61_3x2p61"),
        ],
    );
    run_motion(
        l,
        &[
            one((H, p * 1.5), "1.5x2p1023_0"),
            (vec![(V, p * 1.5)], reads("1.5x2p1023_1.5x2p1023", &sd)),
        ],
    );
}

/// The components' snap past 2^63 (851-3283: `speedwrite-2p63`, `speedwrite-neg2p63`, `direction-turn-2p63`,
/// `direction-radians-2p63`, `hspeed-neg2p63`, `motion-set-2p63`, the `lengthdir` probes): Delphi's `Round` into
/// an `Int64`, which stops GM8 on Intel from 2^63 up and below -2^63, after a `speed` or `direction` write and
/// in `lengthdir_x`/`lengthdir_y`. `motion_set` is the direction's setter and then the speed's (851-3280).
fn motion_traps(l: &mut Lines) {
    use Builtin::{Direction as D, Hspeed as H, Speed as S, Vspeed as V};
    let (p63, p70) = (pow2(63), pow2(70));
    let read = |name: &str, b: Builtin| (name.to_string(), b);
    let cases: [Vec<MotionCase>; 6] = [
        vec![
            (
                vec![(S, p63 - 1024.0)],
                vec![read("hspeed_speed_2p63-1024", H)],
            ),
            (vec![(S, p63)], vec![read("hspeed_speed_2p63", H)]),
        ],
        vec![
            (vec![(S, -p63)], vec![read("hspeed_speed_-2p63", H)]),
            (
                vec![(S, -p63 - 2048.0)],
                vec![read("hspeed_speed_-2p63-2048", H)],
            ),
        ],
        vec![
            (
                vec![(D, 45.0), (S, p63)],
                vec![read("hspeed_45_2p63", H), read("vspeed_45_2p63", V)],
            ),
            (vec![(D, 0.0)], vec![read("hspeed_0_2p63", H)]),
        ],
        vec![
            (
                vec![(S, 0.0), (D, p70)],
                vec![read("direction_2p70", D), read("hspeed_2p70_0", H)],
            ),
            (
                vec![(S, 1.0)],
                vec![read("hspeed_2p70_1", H), read("vspeed_2p70_1", V)],
            ),
        ],
        vec![
            (
                vec![(H, -(p63 - 1024.0))],
                vec![read("speed_-2p63+1024_0", S)],
            ),
            (vec![(H, -p63)], vec![read("speed_-2p63_0", S)]),
        ],
        vec![
            (
                vec![(D, 45.0), (S, p63)],
                vec![read("motion_set_hspeed_45_2p63", H)],
            ),
            (
                vec![(D, 0.0), (S, p63)],
                vec![read("motion_set_hspeed_0_2p63", H)],
            ),
        ],
    ];
    for c in &cases {
        run_motion(l, c);
    }
    for (name, len, dir) in [
        ("lengthdir_x_2p63-1024_0", p63 - 1024.0, 0.0),
        ("lengthdir_x_2p63_0", p63, 0.0),
        ("lengthdir_x_-2p63_0", -p63, 0.0),
        ("lengthdir_x_-2p63-2048_0", -p63 - 2048.0, 0.0),
    ] {
        l.result(name, gm::math::lengthdir_x(len, dir));
    }
    for (name, len) in [
        ("lengthdir_y_2p63-1024_270", p63 - 1024.0),
        ("lengthdir_y_2p63_270", p63),
    ] {
        l.result(name, gm::math::lengthdir_y(len, 270.0));
    }
}

/// The step probes' friction and gravity, through the port's step (851-3283: `friction-2p63`, `gravity-2p63`,
/// `friction-snap`). The step's friction snaps the components as a `speed` write does, but a snap from 2^63 up
/// doesn't stop GM8 there, nor does gravity's speed. A probe's create code writes first; then its step code runs
/// at the start of each step, before that step's motion: it reads its lines, then writes.
fn motion_steps(l: &mut Lines) {
    use Builtin::{
        Direction as D, Friction as F, Gravity as G, GravityDirection as GD, Hspeed as H,
        Speed as S, Vspeed as V,
    };
    type Step = (Vec<(Builtin, f64)>, Vec<(&'static str, Builtin)>);
    let run = |l: &mut Lines, create: &[(Builtin, f64)], steps: &[Step]| {
        let (mut g, me, _) = angle_world();
        let write = |g: &mut gm::World, writes: &[(Builtin, f64)]| {
            for &(b, x) in writes {
                g.set(me, b.var(), Value::Real(x)).unwrap();
            }
        };
        write(&mut g, create);
        for (writes, reads) in steps {
            for (name, b) in reads {
                l.real(name, g.get(me, b.var()).unwrap().to_real());
            }
            write(&mut g, writes);
            common::step(&mut g);
        }
    };
    let (p62, p63) = (pow2(62), pow2(63));
    run(
        l,
        &[(S, p62), (F, -(p62 - 1024.0))],
        &[
            (vec![], vec![("friction_before_hspeed_2p62", H)]),
            (
                vec![(S, p62), (F, -p62)],
                vec![
                    ("friction_hspeed_2p62_-2p62+1024", H),
                    ("friction_speed_2p62_-2p62+1024", S),
                ],
            ),
            (vec![], vec![("friction_hspeed_2p62_-2p62", H)]),
        ],
    );
    run(
        l,
        &[(GD, 0.0), (G, p63 - 1024.0)],
        &[
            (vec![], vec![("gravity_before_hspeed", H)]),
            (
                vec![(H, 0.0), (G, p63)],
                vec![
                    ("gravity_hspeed_2p63-1024", H),
                    ("gravity_speed_2p63-1024", S),
                ],
            ),
            (vec![], vec![("gravity_speed_2p63", S)]),
        ],
    );
    run(
        l,
        &[(D, 90.0), (S, 2.0), (F, 1.0)],
        &[
            (vec![], vec![]),
            (
                vec![(D, 30.0), (S, 3.0), (F, 1.0)],
                vec![
                    ("friction_h_90_2_1", H),
                    ("friction_v_90_2_1", V),
                    ("friction_s_90_2_1", S),
                    ("friction_d_90_2_1", D),
                ],
            ),
            (
                vec![(D, 200.0), (S, 10.0), (F, lit("0.3"))],
                vec![
                    ("friction_h_30_3_1", H),
                    ("friction_v_30_3_1", V),
                    ("friction_s_30_3_1", S),
                ],
            ),
            (
                vec![
                    (F, 0.0),
                    (S, 0.0),
                    (D, 0.0),
                    (S, lit("1000.00005")),
                    (F, lit("0.00001")),
                ],
                vec![
                    ("friction_h_200_10_0.3", H),
                    ("friction_v_200_10_0.3", V),
                    ("friction_s_200_10_0.3", S),
                    ("friction_d_200_10_0.3", D),
                ],
            ),
            (
                vec![],
                vec![
                    ("friction_h_0_1000.00005_0.00001", H),
                    ("friction_s_0_1000.00005_0.00001", S),
                ],
            ),
        ],
    );
}

#[test]
fn the_port_answers_the_probes_like_gm8_and_the_oracle() {
    let mut port = Lines::default();
    numbers(&mut port);
    precision(&mut port);
    rng(&mut port);
    transcendentals(&mut port);
    nonfinite(&mut port);
    ops(&mut port);
    zero_divisors(&mut port);
    string_formats(&mut port);
    divassign_strings(&mut port);
    x87_traps(&mut port);
    mulstr(&mut port);
    motion(&mut port);
    motion_traps(&mut port);
    motion_steps(&mut port);
    motion_setters(&mut port);
    main_loop(&mut port);
    collision_frames(&mut port);
    view_follow(&mut port);
    view_follow_id(&mut port);
    id_lookups(&mut port);
    collision_edges(&mut port);
    deactivation_order(&mut port);

    let mut wrong = Vec::new();
    let mut strings = 0;
    for (table, rosetta) in [(GM8, true), (OPENGMK, false), (GM8_INTEL, false)] {
        for (_, text) in table {
            strings += strings_of_every_real(text, rosetta, &mut wrong);
        }
    }

    // GM8 under Rosetta ran every probe to the end but some `errors/` ones, so it has a line for everything
    // the port computes but where both stop with an error (`a[2^63]`, `real("inf")`).
    let rosetta = results(GM8);
    for (name, value) in &port.0 {
        let Some((probe, want)) = rosetta.lines.get(name) else {
            assert!(value.starts_with(ERROR), "no GM8 line for {name}");
            continue;
        };
        let why = gm8_differs(name).or(traps(value).then_some("Rosetta doesn't trap the x87"));
        compare(&mut wrong, "GM8", (probe, name), value, want, why);
    }

    // The oracle stops at the first error, as GM8 on Intel does, so a line may be missing where the port
    // errors too, or after an error in the same probe (`in-range` has the lines that follow one).
    let oracle = results(OPENGMK);
    let mut checked = 0;
    for (name, value) in &port.0 {
        let probe = rosetta.lines.get(name).map_or("?", |l| l.0);
        match oracle.lines.get(name) {
            Some((probe, want)) => {
                checked += 1;
                compare(&mut wrong, "OpenGMK", (probe, name), value, want, None);
            }
            None if value.starts_with(ERROR) || oracle.stopped.contains(&probe) => {}
            None => wrong.push(format!("{probe}: {name}: port {value}, no OpenGMK line")),
        }
    }
    assert!(checked > 1100, "only {checked} OpenGMK lines checked");

    // GM8 on Intel stops at every x87 trap: `numbers` at `floor(1e20)`, `nonfinite` and `ops` at their first
    // infinity, and every probe below at the line named, where the port stops too.
    let intel = results(GM8_INTEL);
    let mut checked = 0;
    for (name, (probe, want)) in &intel.lines {
        // The functions the port doesn't have.
        let Some(value) = port.0.get(name) else {
            continue;
        };
        checked += 1;
        compare(&mut wrong, "GM8 on Intel", (probe, name), value, want, None);
    }
    assert!(checked > 900, "only {checked} GM8 on Intel lines checked");
    use ValueError::{BadRealString, IntOverflow, Overflow, StringTooLong};
    for (probe, name, error) in [
        ("numbers", "floor_1e20", IntOverflow),
        ("sqrt-neg", "sqrt_-1", ValueError::Domain("sqrt")),
        // 851-3259: arithmetic past the largest double.
        ("div-tiny", "div_1_2^-1074", Overflow),
        ("add-overflow", "add_2p1023_2p1023", Overflow),
        ("sub-overflow", "sub_-2p1023_2p1023", Overflow),
        ("mul-overflow", "mul_2p1023_2", Overflow),
        ("div-overflow", "div_2p1023_0.5", Overflow),
        ("cmp-overflow", "cmp_2p1023_gt_-2p1023", Overflow),
        ("sqr-overflow", "sqr_2p1023", Overflow),
        ("radtodeg-overflow", "radtodeg_2p1023", Overflow),
        (
            "pdist-store-overflow",
            "pdist_1.5x2p1023_1.5x2p1023",
            Overflow,
        ),
        // Integers past the Int64 range.
        ("chr-2p63", "chr_2p63", IntOverflow),
        ("chr-neg-2p64", "chr_-2p64", IntOverflow),
        ("charat-2p63", "charat_2p63", IntOverflow),
        ("color-2p63", "color_2p63", IntOverflow),
        ("sf-total-2p63", "sf_1_2p63_2", IntOverflow),
        ("mulstr-2p63", "mulstr_2p63_a", IntOverflow),
        // 2^31 is -2^31 in 32 bits, which runs 2^31 times: GM8 fails to allocate the string.
        ("mulstr-2p31", "mulstr_2p31_a", StringTooLong),
        // Past 304 MiB GM8 runs out of memory: in bytes, not copies (851-3282).
        ("mulstr-39x2p23", "mulstr_39x2p23_a_len", StringTooLong),
        ("mulstr-5x2p26", "mulstr_5x2p26_a_len", StringTooLong),
        ("mulstr-3x2p27", "mulstr_3x2p27_a_len", StringTooLong),
        ("mulstr-2p29", "mulstr_2p29_a_len", StringTooLong),
        ("mulstr-2p30", "mulstr_2p30_a_len", StringTooLong),
        ("mulstr-2p23-64", "mulstr_2p23_64_len", StringTooLong),
        ("repeat-2p63", "repeat_2p63", IntOverflow),
        // The runner's speed, snapped through an Int64 after `hspeed` or `vspeed` changes (851-3281).
        ("motion-range", "speed_2p600_0", IntOverflow),
        ("motion-snap", "h_0.75x2p600_-2p600", IntOverflow),
        ("speed-2p63", "speed_3x2p61_3x2p61", IntOverflow),
        ("speed-store-overflow", "speed_1.5x2p1023_0", IntOverflow),
        // The components' snap, after a speed or direction write and in lengthdir (851-3283).
        ("speedwrite-2p63", "hspeed_speed_2p63", IntOverflow),
        ("speedwrite-neg2p63", "hspeed_speed_-2p63-2048", IntOverflow),
        ("direction-turn-2p63", "hspeed_0_2p63", IntOverflow),
        ("direction-radians-2p63", "hspeed_2p70_1", IntOverflow),
        ("hspeed-neg2p63", "speed_-2p63_0", IntOverflow),
        ("motion-set-2p63", "motion_set_hspeed_0_2p63", IntOverflow),
        ("lengthdir-2p63", "lengthdir_x_2p63_0", IntOverflow),
        ("lengthdir-neg2p63", "lengthdir_x_-2p63-2048_0", IntOverflow),
        ("lengthdir-y-2p63", "lengthdir_y_2p63_270", IntOverflow),
        // The wrap of a written direction, Delphi's Floor through an Int64 (851-3280).
        ("direction-2p63", "direction_360x2p64", IntOverflow),
        ("array-2p63", "array_2p63", IntOverflow),
        ("alarm-2p63", "alarm_2p63", IntOverflow),
        // In an `op=`, which ignores type errors.
        ("orassign-local", "orassign_local", IntOverflow),
        ("orassign-field", "orassign_field", IntOverflow),
        ("orassign-builtin", "orassign_builtin", IntOverflow),
        ("orassign-arg", "orassign_arg", IntOverflow),
        (
            "mulassign-overflow-local",
            "mulassign_overflow_local",
            Overflow,
        ),
        (
            "mulassign-overflow-field",
            "mulassign_overflow_field",
            Overflow,
        ),
        // `real()` of a number past the largest double, and of `inf` and `nan`.
        ("real-overflow", "real_1e400", BadRealString),
        ("real-inf", "real_inf", BadRealString),
        ("real-nan", "real_nan", BadRealString),
    ] {
        assert!(intel.stopped.contains(&probe), "{probe}");
        assert!(!intel.lines.contains_key(name), "{name}");
        assert_eq!(port.0[name], format!("{ERROR}{error:?}: {error}"), "{name}");
    }
    for e in [IntOverflow, Overflow, StringTooLong] {
        assert_eq!(
            e.to_string(),
            "Unexpected error occured when running the game.",
            "GM8's dialog"
        );
        assert!(e.is_exception());
    }
    // After the game's start an exception only drops the rest of the frame (`main_loop`, 851-3290), but GML's own
    // errors still show GM8's dialog: "Division by 0.", and "Error in function ln().", the trap ln() reports itself.
    for (probe, name) in [("late-div0", "late_div0"), ("late-ln0", "late_ln0")] {
        assert!(intel.stopped.contains(&probe), "{probe}");
        assert!(!intel.lines.contains_key(name), "{name}");
    }
    assert!(!ValueError::DivisionByZero.is_exception());
    // A `draw_surface*` of a surface that doesn't exist is GML's error everywhere, even a stretch that wouldn't draw
    // (851-3308; the port's `Error::NoSuchSurface`, tested in `surfaces.rs`).
    for (probe, name) in [
        ("surface-missing", "surface_missing_drawn"),
        (
            "surface-missing-stretched",
            "surface_missing_stretched_drawn",
        ),
    ] {
        for r in [&rosetta, &oracle, &intel] {
            assert!(r.stopped.contains(&probe), "{probe}");
            assert!(!r.lines.contains_key(name), "{name}");
        }
    }
    for probe in [
        "late-trap",
        "late-scope",
        "late-room-trap",
        "late-create-trap",
        "late-instcreate-trap",
        "late-draw-trap",
        "late-mulstr",
        "step-trap",
        "step-trap-order",
        "gravity-overflow",
        "move-overflow",
    ]
    .into_iter()
    .chain(RUNNER_PROBES.into_iter().filter(|&p| p != "bbox-create"))
    .chain(DRAW_PROBES)
    {
        assert!(!intel.stopped.contains(&probe), "{probe}");
    }
    // An instance created where its box traps leaves GM8 on Intel with no frame to run (851-3301).
    assert!(intel.stopped.contains(&"bbox-create"));
    assert!(!intel.lines.contains_key("bbox_create_blacks_2"));
    // The probes of the runner's arithmetic (851-3301) run whole in the port: it prints every line GM8 on Intel did.
    for (name, (probe, _)) in &intel.lines {
        if (RUNNER_PROBES.contains(probe) || DRAW_PROBES.contains(probe)) && name != "end" {
            assert!(port.0.contains_key(name), "{probe}: no port line {name}");
        }
    }

    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    assert!(strings > 3000, "only {strings} string() lines checked");
}

// ---- The main loop (851-3290): an exception after the game's start drops the rest of the frame ----
//
// The step probes run their step code at the start of oScreen's begin step (`build-exe.py`), after the game has
// started, where GM8 on Intel doesn't stop on an exception but drops the rest of the frame. Each runs here as a
// small program in the port, built as the probe is: object 0 plays oScreen (persistent, at x 16 like the game's),
// whose begin step is the probe's step code; object 1 the SuperSound demo object `core` (no sprite; its create event
// sets `handle`, then `bps`, and its draw event draws), with the probe's logging; and objects 2-6 the end of
// `rIntro` (`oIntro`, `oBricks`, `oGlobals`, `oGame` and four `oBatIntro`), whose create events `late-create-trap`
// looks at. A probe's lines at the hook's call k are read just before the step that makes that call, and kept only
// if the hook ran in it: a frame with a room change pending runs no events.

use gm::{Ctx, Error, Event, GVar, GmlError, Inst, ObjectId, ObjectInfo, Program, R, Var, World};

const STEP: GVar = GVar(1);
const PROBE: GVar = GVar(2);
const PLOG: GVar = GVar(3);
const FLAG: GVar = GVar(4);
const CORE: GVar = GVar(5);
const SCREEN: GVar = GVar(6);
const IN_CREATE: GVar = GVar(7);
const SELF_OK: GVar = GVar(8);
const OTHER_OK: GVar = GVar(9);
const WITH_OTHER_OK: GVar = GVar(10);

const SCREEN_OBJ: ObjectId = ObjectId(0);
const CORE_OBJ: ObjectId = ObjectId(1);
const INTRO_OBJ: ObjectId = ObjectId(2);
const BRICKS_OBJ: ObjectId = ObjectId(3);
const GLOBALS_OBJ: ObjectId = ObjectId(4);
const GAME_OBJ: ObjectId = ObjectId(5);
const BAT_OBJ: ObjectId = ObjectId(6);
const BLACK_OBJ: ObjectId = ObjectId(7);
const MENU_TOP_OBJ: ObjectId = ObjectId(8);
const MENU_LEFT_OBJ: ObjectId = ObjectId(9);

/// The step probes of the runner's own arithmetic (851-3301), which `results!` lists too.
const RUNNER_PROBES: [&str; 10] = [
    "anim-trap",
    "draw-floor-trap",
    "sprite-floor",
    "bbox-trap",
    "bbox-scale",
    "bbox-place",
    "bbox-move",
    "bbox-create",
    "bg-trap",
    "view-trap",
];

/// The step probes of drawing's own conversions (851-3304), which `results!` lists too.
const DRAW_PROBES: [&str; 3] = ["draw-builtin-trap", "draw-default-trap", "draw-stage-trap"];

/// `draw-builtin-trap`'s cases that trap on Intel, by step, and the exception each is in the port.
const DRAW_BUILTIN_TRAPS: [(u32, ValueError); 20] = [
    (1, ValueError::IntOverflow),
    (3, ValueError::Overflow),
    (5, ValueError::Overflow),
    (6, ValueError::Overflow),
    (8, ValueError::Overflow),
    (9, ValueError::IntOverflow),
    (11, ValueError::Overflow),
    (15, ValueError::Overflow),
    (17, ValueError::IntOverflow),
    (18, ValueError::IntOverflow),
    (20, ValueError::Overflow),
    (21, ValueError::IntOverflow),
    (22, ValueError::Overflow),
    (23, ValueError::Overflow),
    (25, ValueError::Overflow),
    (26, ValueError::Overflow),
    (28, ValueError::Overflow),
    (31, ValueError::Overflow),
    (35, ValueError::Overflow),
    (36, ValueError::Overflow),
];

/// The sprites the step probes of the runner's arithmetic use (851-3301), by name.
const STEP_SPRITES: [&str; 7] = [
    "sMetalBlock",
    "sTunnelLeft",
    "sExplosion",
    "sBlack",
    "sMattockHitR",
    "sBlock",
    "sFont",
];

/// `draw-stage-trap`'s `backgroundNight`, 32x32, at its id in the game (851-3304).
const NIGHT: usize = 28;

/// The step probes' backgrounds: only [`NIGHT`].
fn step_backgrounds() -> Vec<Option<assets::Background>> {
    let mut bgs = vec![None; NIGHT + 1];
    bgs[NIGHT] = Some(assets::Background {
        name: "backgroundNight".into(),
        file: None,
        width: 32,
        height: 32,
        use_as_tileset: false,
        tile_width: 16,
        tile_height: 16,
        h_offset: 0,
        v_offset: 0,
        h_sep: 0,
        v_sep: 0,
    });
    bgs
}

thread_local! {
    /// The lines a probe's step code prints itself, as the probes do (`out`): [`run_step_probe`] collects them after
    /// each step.
    static OUT: std::cell::RefCell<Lines> = std::cell::RefCell::new(Lines::default());
}

/// `out(name, x)` for a real, from a probe's step code.
fn out(name: &str, x: f64) {
    OUT.with(|o| o.borrow_mut().real(name, x));
}

/// `out(name, s)` for a string.
fn out_text(name: &str, s: &str) {
    OUT.with(|o| o.borrow_mut().text(name, s));
}

/// The step probes, by what their step code does (`StepProbe::hook`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum StepProbe {
    LateTrap = 1,
    LateSpeedwrite,
    StepTrap,
    GravityOverflow,
    Scope,
    RoomTrap,
    CreateTrap,
    InstcreateTrap,
    DrawTrap,
    Mulstr,
    Order,
    MoveOverflow,
    Div0,
    AnimTrap,
    DrawFloorTrap,
    SpriteFloor,
    BboxTrap,
    BboxScale,
    BboxPlace,
    BboxMove,
    BboxCreate,
    BgTrap,
    ViewTrap,
    CollisionFrameTrap,
    DrawBuiltinTrap,
    DrawDefaultTrap,
    DrawStageTrap,
    CollisionEdgesTrap,
}

impl StepProbe {
    fn of(g: &World) -> Option<StepProbe> {
        use StepProbe::*;
        let all = [
            LateTrap,
            LateSpeedwrite,
            StepTrap,
            GravityOverflow,
            Scope,
            RoomTrap,
            CreateTrap,
            InstcreateTrap,
            DrawTrap,
            Mulstr,
            Order,
            MoveOverflow,
            Div0,
            AnimTrap,
            DrawFloorTrap,
            SpriteFloor,
            BboxTrap,
            BboxScale,
            BboxPlace,
            BboxMove,
            BboxCreate,
            BgTrap,
            ViewTrap,
            CollisionFrameTrap,
            DrawBuiltinTrap,
            DrawDefaultTrap,
            DrawStageTrap,
            CollisionEdgesTrap,
        ];
        all.into_iter().find(|&p| p as u32 as f64 == num(g, PROBE))
    }

    /// Whether the probe's `SS_GetSoundBytesPerSecond` logs `core`'s create events.
    fn logs_creates(self) -> bool {
        matches!(
            self,
            StepProbe::Scope | StepProbe::RoomTrap | StepProbe::InstcreateTrap
        )
    }

    /// Whether its `SS_GetSoundPosition` logs `core`'s draws.
    fn logs_draws(self) -> bool {
        matches!(
            self,
            StepProbe::Scope
                | StepProbe::RoomTrap
                | StepProbe::InstcreateTrap
                | StepProbe::DrawTrap
                | StepProbe::MoveOverflow
                | StepProbe::DrawFloorTrap
                | StepProbe::ViewTrap
                | StepProbe::DrawDefaultTrap
                | StepProbe::DrawStageTrap
        )
    }
}

/// The program, and its `core`'s and `rIntro`'s variables.
struct StepProgram {
    program: &'static Program,
    handle: Var,
    bps: Var,
    fade_level: Var,
    draw_status: Var,
    y_acc: Var,
}

fn step_program() -> &'static StepProgram {
    static P: std::sync::OnceLock<StepProgram> = std::sync::OnceLock::new();
    P.get_or_init(|| {
        let mut vars = Vec::new();
        let program = common::build(|b| {
            let globals = [
                "probe_step",
                "probe",
                "probe_log",
                "probe_flag",
                "probe_core",
                "probe_screen",
                "probe_in_create",
                "probe_self",
                "probe_other",
                "probe_with_other",
            ];
            for (i, name) in globals.into_iter().enumerate() {
                assert_eq!(b.global(name), GVar(i as u16 + 1));
            }
            vars = ["handle", "bps", "fadeLevel", "drawStatus", "yAcc"]
                .map(|v| b.var(v))
                .to_vec();
            let persistent = ObjectInfo {
                persistent: true,
                ..ObjectInfo::new("oScreen")
            };
            for (id, info) in [
                (SCREEN_OBJ, persistent),
                (CORE_OBJ, ObjectInfo::new("core")),
                (INTRO_OBJ, ObjectInfo::new("oIntro")),
                (BRICKS_OBJ, ObjectInfo::new("oBricks")),
                (GLOBALS_OBJ, ObjectInfo::new("oGlobals")),
                (GAME_OBJ, ObjectInfo::new("oGame")),
                (BAT_OBJ, ObjectInfo::new("oBatIntro")),
                (
                    BLACK_OBJ,
                    ObjectInfo {
                        sprite: spelunky::sprite_id("sBlack"),
                        depth: 1.0,
                        ..ObjectInfo::new("oBlack")
                    },
                ),
                (MENU_TOP_OBJ, ObjectInfo::new("oMenuTop")),
                (MENU_LEFT_OBJ, ObjectInfo::new("oMenuLeft")),
            ] {
                assert_eq!(b.push_object(info), id);
            }
            b.layout(CORE_OBJ, &vars[0..2])
                .layout(INTRO_OBJ, &vars[2..3])
                .layout(GAME_OBJ, &vars[3..4])
                .layout(BAT_OBJ, &vars[4..5])
                .event(SCREEN_OBJ, Event::BeginStep, hook)
                .event(CORE_OBJ, Event::Create, core_create)
                .event(CORE_OBJ, Event::Draw, core_draw)
                .event(INTRO_OBJ, Event::Create, |g, cx| {
                    set_var(g, cx, step_program().fade_level)
                })
                .event(BRICKS_OBJ, Event::Create, bricks_create)
                .event(GAME_OBJ, Event::Create, |g, cx| {
                    set_var(g, cx, step_program().draw_status)
                })
                .event(BAT_OBJ, Event::Create, |g, cx| {
                    set_var(g, cx, step_program().y_acc)
                });
        });
        StepProgram {
            program,
            handle: vars[0],
            bps: vars[1],
            fade_level: vars[2],
            draw_status: vars[3],
            y_acc: vars[4],
        }
    })
}

/// A global's number, 0 while it's unset (before a probe sets its globals).
fn num(g: &World, v: GVar) -> f64 {
    g.gget(v).map_or(0.0, |v| v.to_real())
}

fn log_text(g: &World) -> String {
    g.gget(PLOG)
        .map_or(String::new(), |v| v.to_gml_string().to_string())
}

/// `global.probe_log += s`.
fn plog(g: &mut World, s: &str) -> R<()> {
    let text = log_text(g) + s;
    g.gset(PLOG, Value::from(text.as_str()))
}

/// Takes the flag a probe sets for its trap, if set.
fn take_flag(g: &mut World) -> R<bool> {
    let set = num(g, FLAG) != 0.0;
    g.gset(FLAG, Value::Real(0.0))?;
    Ok(set)
}

/// The probes' trap: `floor(2^64)`.
fn trap() -> R<()> {
    math::floor(pow2(64))?;
    Ok(())
}

fn set_var(g: &mut World, cx: &mut Ctx, v: Var) -> R<()> {
    g.set(cx.this, v, Value::Real(0.0))
}

fn set_b(g: &mut World, h: Inst, b: Builtin, x: f64) -> R<()> {
    g.set(h, b.var(), Value::Real(x))
}

fn get_b(g: &World, h: Inst, b: Builtin) -> f64 {
    g.get(h, b.var()).unwrap().to_real()
}

fn create_core(g: &mut World) -> R<Inst> {
    let id = g.instance_create(0.0, 0.0, CORE_OBJ.0 as i32)?;
    Ok(g.find(id).expect("created"))
}

/// `core`'s create event: `handle = SS_LoadSound(...)`, then `bps = SS_GetSoundBytesPerSecond(handle)`, which logs
/// and, in `late-instcreate-trap`, traps once flagged.
fn core_create(g: &mut World, cx: &mut Ctx) -> R<()> {
    let p = step_program();
    g.set(cx.this, p.handle, Value::Real(0.0))?;
    let probe = StepProbe::of(g);
    if probe.is_some_and(StepProbe::logs_creates) {
        plog(g, &format!("c{}", num(g, STEP)))?;
    }
    if probe == Some(StepProbe::InstcreateTrap) && take_flag(g)? {
        trap()?;
    }
    g.set(cx.this, p.bps, Value::Real(1.0))
}

/// `core`'s draw event: `SS_GetSoundPosition` logs (or, in `draw-builtin-trap`, draws), and in `late-draw-trap`
/// `SS_GetSoundLength` traps once flagged.
fn core_draw(g: &mut World, cx: &mut Ctx) -> R<()> {
    let probe = StepProbe::of(g);
    if probe == Some(StepProbe::DrawBuiltinTrap) {
        return draw_case(g, cx);
    }
    if probe.is_some_and(StepProbe::logs_draws) {
        plog(g, &format!("d{}", num(g, STEP)))?;
    }
    if probe == Some(StepProbe::DrawTrap) && take_flag(g)? {
        plog(g, "t")?;
        trap()?;
    }
    Ok(())
}

/// `oBricks`'s create event: `scrSetupWalls(224)`, a trap in `late-create-trap` once flagged, then
/// `instance_destroy()`.
fn bricks_create(g: &mut World, cx: &mut Ctx) -> R<()> {
    if StepProbe::of(g) == Some(StepProbe::CreateTrap) && take_flag(g)? {
        g.gset(IN_CREATE, Value::Real(1.0))?;
        trap()?;
    }
    g.instance_destroy(cx)
}

/// oScreen's begin step: the probe's step code, after the hook counts its call.
fn hook(g: &mut World, cx: &mut Ctx) -> R<()> {
    use Builtin::{
        Direction as D, Friction as F, Gravity as G, GravityDirection as GD, Hspeed as H,
        ImageSpeed, Speed as S, X,
    };
    use StepProbe::*;
    let step = num(g, STEP) + 1.0;
    g.gset(STEP, Value::Real(step))?;
    let Some(probe) = StepProbe::of(g) else {
        return Ok(());
    };
    match (probe, step as u32) {
        (LateTrap, 3) => trap()?,
        (LateSpeedwrite, 3) => set_b(g, cx.this, S, pow2(63))?,
        (Scope | RoomTrap | DrawTrap, 1) => {
            let c = create_core(g)?;
            let id = get_b(g, c, Builtin::Id);
            g.gset(CORE, Value::Real(id))?;
            if probe == Scope {
                set_b(g, c, H, 1.0)?;
                set_b(g, c, ImageSpeed, 1.0)?;
            }
        }
        (Scope, 3) => {
            // with (core) { SS_IsHandleValid(1) }, which logs and traps.
            let outer = (cx.this, cx.other);
            let mut it = g.with_iter(cx, &Value::Real(num(g, CORE)))?;
            while let Some(h) = it.next(g) {
                (cx.this, cx.other) = (h, outer.0);
                plog(g, "t")?;
                trap()?;
            }
            (cx.this, cx.other) = outer;
        }
        (Scope, 4) => {
            let screen = num(g, SCREEN);
            let id = |g: &World, h| get_b(g, h, Builtin::Id);
            g.gset(SELF_OK, Value::Real(b(id(g, cx.this) == screen)))?;
            g.gset(OTHER_OK, Value::Real(b(id(g, cx.other) == screen)))?;
            let outer = (cx.this, cx.other);
            let mut it = g.with_iter(cx, &Value::Real(num(g, CORE)))?;
            while let Some(h) = it.next(g) {
                (cx.this, cx.other) = (h, outer.0);
                g.gset(WITH_OTHER_OK, Value::Real(b(id(g, cx.other) == screen)))?;
            }
            (cx.this, cx.other) = outer;
        }
        (RoomTrap, 3) => {
            g.room_restart();
            trap()?;
        }
        (CreateTrap, 2) => {
            g.gset(FLAG, Value::Real(1.0))?;
            g.room_restart();
        }
        (InstcreateTrap, 2) => {
            g.gset(FLAG, Value::Real(1.0))?;
            create_core(g)?;
        }
        (DrawTrap, 2) => g.gset(FLAG, Value::Real(1.0))?,
        (Mulstr, 3) => {
            Value::Real(39.0 * pow2(23)).mul(&Value::from("a"))?;
        }
        (Order, 1) => {
            let p = pow2(62);
            let c: Vec<Inst> = (0..3).map(|_| create_core(g)).collect::<R<_>>()?;
            for &i in [c[0], c[2]].iter() {
                set_b(g, i, H, 1.0)?;
                set_b(g, i, G, 1.0)?;
            }
            for (b, x) in [
                (D, 1.0),
                (S, 1.5 * p),
                (F, -p),
                (GD, 270.0),
                (G, p / 4194304.0),
            ] {
                set_b(g, c[1], b, x)?;
            }
            for (k, &i) in c.iter().enumerate() {
                let id = get_b(g, i, Builtin::Id);
                g.gset_a(CORE, 0, k as u32, Value::Real(id))?;
            }
        }
        (MoveOverflow, 1) => {
            let c = create_core(g)?;
            let id = get_b(g, c, Builtin::Id);
            g.gset(CORE, Value::Real(id))?;
            set_b(g, c, X, pow2(1023))?;
            set_b(g, c, H, pow2(1023))?;
        }
        (Div0, 3) => {
            math::div(1.0, 0.0)?;
        }
        (
            AnimTrap | DrawFloorTrap | SpriteFloor | BboxTrap | BboxScale | BboxPlace | BboxMove
            | BboxCreate | BgTrap | ViewTrap,
            k,
        ) => runner_probe(g, probe, k)?,
        (CollisionFrameTrap, k) => collision_probe(g, k)?,
        (DrawBuiltinTrap | DrawDefaultTrap | DrawStageTrap, k) => draw_probe(g, probe, k)?,
        (CollisionEdgesTrap, k) => collision_edges_probe(g, k)?,
        _ => {}
    }
    Ok(())
}

/// The step code of the probes of the runner's own arithmetic (851-3301), line for line as the GML: each prints its
/// lines itself ([`out`]).
fn runner_probe(g: &mut World, probe: StepProbe, k: u32) -> R<()> {
    use Builtin::{
        BboxBottom, BboxLeft, BboxRight, BboxTop, Depth, Hspeed, ImageAngle, ImageIndex,
        ImageNumber, ImageSpeed, ImageXscale, MaskIndex, SpriteIndex, ViewHborder, ViewHport,
        ViewHspeed, ViewHview, ViewObject, ViewVborder, ViewVisible, ViewVspeed, ViewWport,
        ViewWview, ViewXport, ViewXview, ViewYport, ViewYview, X, Y,
    };
    use StepProbe::*;
    let p = pow2(63);
    let big = pow2(1023) * 1.5;
    let sprite = |name: &str| f64::from(spelunky::sprite_id(name));
    let create = |g: &mut World, x: f64, y: f64, o: ObjectId| -> R<Inst> {
        let id = g.instance_create(x, y, o.0 as i32)?;
        Ok(g.find(id).expect("created"))
    };
    // The probe's instances, by their ids in `global.probe_core[n]`.
    let keep = |g: &mut World, hs: &[Inst]| -> R<()> {
        for (n, &h) in hs.iter().enumerate() {
            let id = get_b(g, h, Builtin::Id);
            g.gset_a(CORE, 0, n as u32, Value::Real(id))?;
        }
        Ok(())
    };
    let kept = |g: &World, n: u32| {
        g.find(g.gget_a(CORE, 0, n).unwrap().to_real() as i32)
            .unwrap()
    };
    let bbox = |g: &World, h: Inst| {
        let side = |b| real_to_string(get_b(g, h, b)).to_string();
        let [l, t, r, b] = [BboxLeft, BboxTop, BboxRight, BboxBottom].map(side);
        format!("{l} {t} {r} {b}")
    };
    let screen = |g: &World| common::ctx(g, screen_of(g));
    match probe {
        AnimTrap => {
            if k == 1 {
                let c: Vec<Inst> = (0..3).map(|_| create_core(g)).collect::<R<_>>()?;
                for (&h, depth) in c.iter().zip([30.0, 20.0, 10.0]) {
                    set_b(g, h, Depth, depth)?;
                }
                for (h, index, speed) in [
                    (c[0], None, 0.25),
                    (c[1], Some(big), big),
                    (c[2], None, 0.25),
                ] {
                    set_b(g, h, SpriteIndex, sprite("sMetalBlock"))?;
                    if let Some(i) = index {
                        set_b(g, h, ImageIndex, i)?;
                    }
                    set_b(g, h, ImageSpeed, speed)?;
                }
                keep(g, &c)?;
                out("anim_frames", get_b(g, c[0], ImageNumber));
                out("anim_c2_1", get_b(g, c[1], ImageIndex));
            }
            let (c1, c2, c3) = (kept(g, 0), kept(g, 1), kept(g, 2));
            let n = get_b(g, c1, ImageNumber);
            if k >= 2 {
                for (name, h) in [("c1", c1), ("c2", c2), ("c3", c3)] {
                    out(&format!("anim_{name}_{k}"), get_b(g, h, ImageIndex));
                }
                out(&format!("anim_tenth_{k}"), math::div(1.0, 10.0)?);
            }
            if k == 2 {
                set_b(g, c2, ImageSpeed, 0.0)?;
                set_b(g, c2, ImageIndex, p)?;
            }
            if k == 3 {
                set_b(g, c1, ImageSpeed, 0.0)?;
                set_b(g, c1, ImageIndex, p / 2147483648.0 + n + 0.5)?;
                set_b(g, c2, ImageIndex, p / 2147483648.0 + 0.5)?;
                set_b(g, c3, ImageSpeed, 0.0)?;
                set_b(g, c3, ImageIndex, p / 4294967296.0 + 0.5)?;
            }
        }
        DrawFloorTrap => {
            if k == 1 {
                let c = [
                    create_core(g)?,
                    create(g, 0.0, 0.0, BLACK_OBJ)?,
                    create_core(g)?,
                ];
                for (&h, depth) in c.iter().zip([30.0, 20.0, 10.0]) {
                    set_b(g, h, Depth, depth)?;
                }
                set_b(g, c[1], ImageSpeed, 0.0)?;
                keep(g, &c)?;
            }
            let (c1, b, c3) = (kept(g, 0), kept(g, 1), kept(g, 2));
            if k >= 2 {
                out_text(&format!("draw_floor_log_{k}"), &log_text(g));
                out(&format!("draw_floor_c1_{k}"), get_b(g, c1, ImageIndex));
                out(&format!("draw_floor_c3_{k}"), get_b(g, c3, ImageIndex));
                out(&format!("draw_floor_b_{k}"), get_b(g, b, ImageIndex));
            }
            match k {
                2 => set_b(g, b, ImageIndex, p)?,
                3 => set_b(g, b, ImageIndex, 0.0)?,
                _ => {}
            }
        }
        SpriteFloor => {
            if k == 1 {
                let c = create_core(g)?;
                keep(g, &[c])?;
                set_b(g, c, ImageSpeed, 0.0)?;
                set_b(g, c, ImageIndex, p / 2147483648.0 + 1.5)?;
                set_b(g, c, SpriteIndex, sprite("sExplosion"))?;
                out("sprite_floor_frames", get_b(g, c, ImageNumber));
                out("sprite_floor_wrapped", get_b(g, c, ImageIndex));
                set_b(g, c, ImageIndex, p)?;
                set_b(g, c, SpriteIndex, sprite("sMetalBlock"))?;
                out("sprite_floor_after", get_b(g, c, ImageIndex));
            }
            let c = kept(g, 0);
            if k == 2 {
                out(
                    "sprite_floor_sprite",
                    b(get_b(g, c, SpriteIndex) == sprite("sMetalBlock")),
                );
                out("sprite_floor_index", get_b(g, c, ImageIndex));
                set_b(g, c, ImageIndex, 0.0)?;
            }
        }
        BboxTrap => {
            if k == 1 {
                let c = create(g, 100.0, 50.0, CORE_OBJ)?;
                keep(g, &[c])?;
                set_b(g, c, MaskIndex, sprite("sTunnelLeft"))?;
            }
            let c = kept(g, 0);
            out_text(&format!("bbox_{k}"), &bbox(g, c));
            out(&format!("bbox_x_{k}"), get_b(g, c, X));
            out(&format!("bbox_y_{k}"), get_b(g, c, Y));
            match k {
                1 => set_b(g, c, X, p)?,
                2 => {
                    set_b(g, c, X, get_b(g, c, X))?;
                    set_b(g, c, Y, get_b(g, c, Y))?;
                    out("bbox_same_2", 1.0);
                    set_b(g, c, X, 100.0)?;
                    out_text("bbox_back_2", &bbox(g, c));
                    set_b(g, c, Y, p)?;
                }
                3 => {
                    set_b(g, c, Y, 50.0)?;
                    set_b(g, c, ImageAngle, 30.0)?;
                    out_text("bbox_rotated_3", &bbox(g, c));
                    set_b(g, c, ImageXscale, big)?;
                }
                4 => {
                    out("bbox_xscale_4", get_b(g, c, ImageXscale));
                    set_b(g, c, ImageXscale, 1.0)?;
                    set_b(g, c, ImageAngle, 0.0)?;
                    set_b(g, c, MaskIndex, -1.0)?;
                    set_b(g, c, X, p)?;
                    out_text("bbox_nomask_4", &bbox(g, c));
                    set_b(g, c, MaskIndex, sprite("sTunnelLeft"))?;
                }
                5 => out(
                    "bbox_mask_5",
                    b(get_b(g, c, MaskIndex) == sprite("sTunnelLeft")),
                ),
                _ => {}
            }
        }
        BboxScale => {
            if k == 1 {
                let c = create(g, p / 2.0, 50.0, CORE_OBJ)?;
                keep(g, &[c])?;
                set_b(g, c, MaskIndex, sprite("sTunnelLeft"))?;
                out_text("bbox_scale_1", &bbox(g, c));
                set_b(g, c, ImageXscale, p / 8.0)?;
            }
            let c = kept(g, 0);
            if k == 2 {
                out_text("bbox_scale_2", &bbox(g, c));
                out("bbox_scale_xscale_2", get_b(g, c, ImageXscale));
                set_b(g, c, ImageXscale, 1.0)?;
            }
        }
        BboxPlace => {
            if k == 1 {
                let c = create(g, 100.0, 50.0, CORE_OBJ)?;
                keep(g, &[c])?;
                set_b(g, c, MaskIndex, sprite("sTunnelLeft"))?;
                set_b(g, c, X, p)?;
            }
            let c = kept(g, 0);
            if k >= 2 {
                out_text(&format!("bbox_place_{k}"), &bbox(g, c));
                out(&format!("bbox_place_x_{k}"), get_b(g, c, X));
            }
            let black = BLACK_OBJ.0 as i32;
            match k {
                2 => {
                    let met = g.place_meeting(&common::ctx(g, c), 100.0, 50.0, black)?;
                    out("bbox_place_back_2", b(met));
                }
                3 => {
                    set_b(g, c, X, 100.0)?;
                    let met = g.place_meeting(&common::ctx(g, c), p, 50.0, black)?;
                    out("bbox_place_there_3", b(met));
                }
                4 => set_b(g, c, X, 100.0)?,
                _ => {}
            }
        }
        BboxMove => {
            if k == 1 {
                let c = create(g, p / 2.0, 50.0, CORE_OBJ)?;
                set_b(g, c, MaskIndex, sprite("sTunnelLeft"))?;
                set_b(g, c, Hspeed, p / 2.0)?;
                let c2 = create_core(g)?;
                set_b(g, c2, Hspeed, 1.0)?;
                keep(g, &[c, c2])?;
            }
            let (c, c2) = (kept(g, 0), kept(g, 1));
            if k == 2 {
                out_text("bbox_move_2", &bbox(g, c));
                out("bbox_move_x_2", get_b(g, c, X));
                out("bbox_move_y_2", get_b(g, c, Y));
                out("bbox_move_c2_2", get_b(g, c2, X));
                set_b(g, c, Hspeed, 0.0)?;
                set_b(g, c, X, 100.0)?;
            }
        }
        BboxCreate => {
            if k == 1 {
                let c = create_core(g)?;
                keep(g, &[c])?;
                out("bbox_create_1", 1.0);
                create(g, p, 0.0, BLACK_OBJ)?;
            }
            if k >= 2 {
                let blacks = g.instance_number(&screen(g), BLACK_OBJ.0 as i32);
                out(&format!("bbox_create_blacks_{k}"), f64::from(blacks));
                let first = get_b(g, kept(g, 0), Builtin::Id);
                let next = g.instance_create(0.0, 0.0, CORE_OBJ.0 as i32)?;
                out(&format!("bbox_create_ids_{k}"), f64::from(next) - first);
            }
        }
        BgTrap => {
            if k == 1 {
                let c = create_core(g)?;
                keep(g, &[c])?;
                for i in 2..=4 {
                    let bg = g.background_mut(i);
                    bg.visible = false;
                    (bg.x, bg.y, bg.hspeed, bg.vspeed) = (0.0, 0.0, 1.0, 1.0);
                }
                let bg = g.background_mut(3);
                (bg.x, bg.hspeed) = (big, big);
            }
            if k >= 2 {
                for i in 2..=4 {
                    let bg = &g.room().backgrounds[i];
                    out(&format!("bg_x{i}_{k}"), bg.x);
                    out(&format!("bg_y{i}_{k}"), bg.y);
                }
                out(&format!("bg_image_{k}"), get_b(g, kept(g, 0), ImageIndex));
            }
            if k == 2 {
                let bg = g.background_mut(3);
                (bg.x, bg.hspeed, bg.y, bg.vspeed) = (0.0, 1.0, big, big);
            }
        }
        ViewTrap => {
            if k == 1 {
                let c1 = create(g, 300.0, 100.0, CORE_OBJ)?;
                let c2 = create(g, 300.0, 100.0, CORE_OBJ)?;
                keep(g, &[c2])?;
                for i in 1..=2 {
                    for (b, x) in [
                        (ViewVisible, 1.0),
                        (ViewXview, 0.0),
                        (ViewYview, 0.0),
                        (ViewWview, 100.0),
                        (ViewHview, 100.0),
                        (ViewXport, 0.0),
                        (ViewYport, 0.0),
                        (ViewWport, 100.0),
                        (ViewHport, 100.0),
                        (ViewHborder, 10.0),
                        (ViewVborder, 10.0),
                        (ViewHspeed, -1.0),
                        (ViewVspeed, -1.0),
                    ] {
                        g.bset(b.var(), i, Value::Real(x))?;
                    }
                }
                for (i, c) in [(1, c1), (2, c2)] {
                    let id = get_b(g, c, Builtin::Id);
                    g.bset(ViewObject.var(), i, Value::Real(id))?;
                }
                set_b(g, c2, X, p)?;
            }
            if k >= 2 {
                for i in 1..=2 {
                    for (axis, b) in [("x", ViewXview), ("y", ViewYview)] {
                        let v = g.bget(b.var(), i)?.to_real();
                        out(&format!("view_{axis}{i}_{k}"), v);
                    }
                }
                out_text(&format!("view_log_{k}"), &log_text(g));
            }
            if k == 2 {
                set_b(g, kept(g, 0), X, p / 2147483648.0 + 300.0)?;
            }
        }
        _ => unreachable!("{probe:?}"),
    }
    Ok(())
}

/// When a probe reads its lines: before the step that makes the hook's call, or after it (the lines it prints after
/// writing).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Before,
    After,
}

/// What running a step probe gave: the exception each step dropped the rest of its frame on, by the hook's call
/// count after it, and the error that stopped it, if one did.
struct StepRun {
    exceptions: Vec<(u32, ValueError)>,
    stopped: Option<GmlError>,
}

/// `collision-frame-trap`'s step code (851-3305), line for line as the GML: the precise tests' `Floor` of
/// `image_index` and `Round`s of positions trap once the boxes meet, one a step.
fn collision_probe(g: &mut World, k: u32) -> R<()> {
    use Builtin::{
        BboxBottom, BboxLeft, BboxRight, BboxTop, Id, ImageIndex, ImageSpeed, ImageXscale,
        ImageYscale, MaskIndex, Visible, X, Y,
    };
    let (p63, p64) = (pow2(63), pow2(64));
    let sprite = |name: &str| f64::from(spelunky::sprite_id(name));
    let (black, top, left) = (
        BLACK_OBJ.0 as i32,
        MENU_TOP_OBJ.0 as i32,
        MENU_LEFT_OBJ.0 as i32,
    );
    let create = |g: &mut World, x: f64, y: f64, o: ObjectId, mask: &str| -> R<Inst> {
        let id = g.instance_create(x, y, o.0 as i32)?;
        let h = g.find(id).expect("created");
        set_b(g, h, Visible, 0.0)?;
        set_b(g, h, ImageSpeed, 0.0)?;
        set_b(g, h, MaskIndex, sprite(mask))?;
        Ok(h)
    };
    // `global.probe_a`, `_b` and `_d`, as slots of `global.probe_core`.
    let keep = |g: &mut World, n: u32, h: Inst| {
        let id = get_b(g, h, Id);
        g.gset_a(CORE, 0, n, Value::Real(id))
    };
    let kept = |g: &World, n: u32| g.find(g.gget_a(CORE, 0, n).unwrap().to_real() as i32);
    let screen = |g: &World| common::ctx(g, screen_of(g));
    let point = |g: &World, x: f64, y: f64, obj: i32, prec: bool| -> R<f64> {
        Ok(b(
            g.collision_point(&screen(g), x, y, obj, prec, false)? != -4
        ))
    };
    let place = |g: &mut World, h: Inst, x: f64, y: f64, obj: i32| -> R<f64> {
        Ok(b(g.place_meeting(&common::ctx(g, h), x, y, obj)?))
    };
    if k == 1 {
        let a = create(g, 100.0, 100.0, BLACK_OBJ, "sMattockHitR")?;
        let bb = create(g, 200.0, 60.0, MENU_TOP_OBJ, "sBlock")?;
        set_b(g, bb, ImageXscale, 1.0 / 16.0)?;
        set_b(g, bb, ImageYscale, 1.0 / 16.0)?;
        keep(g, 0, a)?;
        keep(g, 1, bb)?;
        out("cft_frame0", point(g, 97.0, 94.0, black, true)?);
        out("cft_self0", place(g, a, 203.0, 66.0, top)?);
        return Ok(());
    }
    let (a, bb) = (kept(g, 0).unwrap(), kept(g, 1).unwrap());
    match k {
        2 => {
            set_b(g, a, ImageIndex, p63)?;
            out("cft_point_before", 1.0);
            out("cft_point", point(g, 97.0, 94.0, black, true)?);
            out("cft_point_after", 1.0);
        }
        3 => {
            out("cft_index_3", get_b(g, a, ImageIndex));
            out("cft_point_imprecise", point(g, 97.0, 94.0, black, false)?);
            out("cft_point_outside", point(g, 50.0, 50.0, black, true)?);
            let rect = |g: &World, prec| -> R<f64> {
                Ok(b(g.collision_rectangle(
                    &screen(g),
                    97.0,
                    94.0,
                    97.0,
                    94.0,
                    black,
                    prec,
                    false,
                )? != -4))
            };
            out("cft_rect_imprecise", rect(g, false)?);
            out("cft_rect_before", 1.0);
            out("cft_rect", rect(g, true)?);
        }
        4 => {
            out("cft_line_before", 1.0);
            let hit = g.collision_line(&screen(g), 97.0, 94.0, 97.0, 94.0, black, true, false)?;
            out("cft_line", b(hit != -4));
        }
        5 => {
            out("cft_self_before", 1.0);
            out("cft_self", place(g, a, 203.0, 66.0, top)?);
        }
        6 => {
            out("cft_self_x_6", get_b(g, a, X));
            out("cft_self_y_6", get_b(g, a, Y));
            set_b(g, a, X, 100.0)?;
            set_b(g, a, Y, 100.0)?;
            set_b(g, a, ImageIndex, 0.0)?;
            set_b(g, bb, ImageIndex, p63)?;
            out("cft_other_before", 1.0);
            out("cft_other", place(g, a, 203.0, 66.0, top)?);
        }
        7 => {
            out("cft_other_x_7", get_b(g, a, X));
            out("cft_other_y_7", get_b(g, a, Y));
            set_b(g, a, X, 100.0)?;
            set_b(g, a, Y, 100.0)?;
            set_b(g, bb, ImageIndex, 0.0)?;
            set_b(g, a, ImageIndex, -p63 - 2048.0)?;
            out("cft_neg_before", 1.0);
            out("cft_neg", point(g, 97.0, 94.0, black, true)?);
        }
        8 => {
            set_b(g, a, ImageIndex, -p63)?;
            out("cft_neg2p63", point(g, 97.0, 94.0, black, true)?);
            set_b(g, a, ImageIndex, 0.0)?;
            out("cft_line_far_before", 1.0);
            let hit = g.collision_line(&screen(g), 97.0, 94.0, 97.0, p64, black, true, false)?;
            out("cft_line_far", b(hit != -4));
        }
        9 => {
            let d = create(g, 0.0, 0.0, MENU_LEFT_OBJ, "sBlock")?;
            keep(g, 2, d)?;
            out("cft_d_before", 1.0);
            set_b(g, d, X, p63)?;
        }
        10 => {
            let d = kept(g, 2).unwrap();
            let side = |s| real_to_string(get_b(g, d, s)).to_string();
            let sides = [BboxLeft, BboxTop, BboxRight, BboxBottom].map(side);
            out_text("cft_d_box", &sides.join(" "));
            out("cft_pos_imprecise", point(g, 5.0, 5.0, left, false)?);
            out("cft_pos_before", 1.0);
            out("cft_pos", point(g, 5.0, 5.0, left, true)?);
        }
        11 => {
            set_b(g, bb, X, 8.0)?;
            set_b(g, bb, Y, 8.0)?;
            out("cft_pos_inst_before", 1.0);
            out("cft_pos_inst", place(g, bb, 8.0, 8.0, left)?);
        }
        _ => {}
    }
    Ok(())
}

/// `collision-edges-trap`'s step code (851-3306), line for line as the GML: a rectangle's corners are rounded (and
/// trap from 2^63) only by a precise test whose box meets the rectangle, on an instance with a mask, one test a step.
fn collision_edges_probe(g: &mut World, k: u32) -> R<()> {
    use Builtin::{Id, ImageSpeed, MaskIndex, SpriteIndex, Visible};
    let p64 = pow2(64);
    let black = BLACK_OBJ.0 as i32;
    let screen = |g: &World| common::ctx(g, screen_of(g));
    let rect = |g: &World, x1, y1, x2, y2, prec| -> R<f64> {
        Ok(b(g.collision_rectangle(
            &screen(g),
            x1,
            y1,
            x2,
            y2,
            black,
            prec,
            false,
        )? != -4))
    };
    match k {
        1 => {
            let id = g.instance_create(100.0, 100.0, black)?;
            let a = g.find(id).expect("created");
            set_b(g, a, Visible, 0.0)?;
            set_b(g, a, ImageSpeed, 0.0)?;
            set_b(g, a, MaskIndex, f64::from(spelunky::sprite_id("sBlock")))?;
            let id = get_b(g, a, Id);
            g.gset(CORE, Value::Real(id))?;
            out("cet_near", rect(g, 100.0, 104.0, 110.0, 112.0, true)?);
        }
        2 => {
            out("cet_far_imprecise_before", 1.0);
            out(
                "cet_far_imprecise",
                rect(g, 100.0, 104.0, p64, 112.0, false)?,
            );
        }
        3 => {
            out("cet_far_miss_before", 1.0);
            out("cet_far_miss", rect(g, p64, 104.0, p64 * 2.0, 112.0, true)?);
        }
        4 => {
            out("cet_far_miss_neg_before", 1.0);
            out("cet_far_miss_neg", rect(g, -p64, 104.0, 50.0, 112.0, true)?);
        }
        5 => {
            out("cet_far_max_before", 1.0);
            out("cet_far_max", rect(g, 100.0, 104.0, p64, 112.0, true)?);
        }
        6 => {
            out("cet_far_min_before", 1.0);
            out("cet_far_min", rect(g, -p64, 104.0, 108.0, 112.0, true)?);
        }
        7 => {
            out("cet_far_point_before", 1.0);
            let hit = g.collision_point(&screen(g), p64, 108.0, black, true, false)?;
            out("cet_far_point", b(hit != -4));
        }
        8 => {
            let left = MENU_LEFT_OBJ.0 as i32;
            let id = g.instance_create(300.0, 100.0, left)?;
            let e = g.find(id).expect("created");
            set_b(g, e, Visible, 0.0)?;
            set_b(g, e, SpriteIndex, -1.0)?;
            set_b(g, e, MaskIndex, -1.0)?;
            out("cet_nomask_before", 1.0);
            let y = -100000.0;
            let hit = g.collision_rectangle(&screen(g), -p64, y, -99990.0, y, left, true, false)?;
            out("cet_nomask", b(hit != -4));
        }
        _ => {}
    }
    Ok(())
}

/// oScreen, which is persistent: the same id in every room, but after a room change the placed instance it replaced
/// (deleted until the frame's cleanup) shadows that id for `find`, as in GM8.
fn screen_of(g: &World) -> Inst {
    g.active()
        .find(|&h| g.instance(h).object == SCREEN_OBJ)
        .expect("oScreen")
}

/// Runs a step probe until the hook's call `calls`: `setup` is its create code (on oScreen), and `read` records the
/// lines at each call.
fn run_step_probe(
    l: &mut Lines,
    probe: StepProbe,
    setup: impl FnOnce(&mut World, Inst),
    calls: u32,
    mut read: impl FnMut(&mut World, Inst, u32, Phase, &mut Lines),
) -> StepRun {
    let room = common::room(
        320,
        240,
        &common::numbered(&[
            (SCREEN_OBJ, 16, 0),
            (INTRO_OBJ, 0, 0),
            (BRICKS_OBJ, 16, 0),
            (GLOBALS_OBJ, 32, 0),
            (GAME_OBJ, 48, 0),
            (BAT_OBJ, 608, 144),
            (BAT_OBJ, 592, 128),
            (BAT_OBJ, 576, 160),
            (BAT_OBJ, 544, 144),
        ]),
    );
    let assets = gm::Assets::new(spelunky::sprites(&STEP_SPRITES), Vec::new())
        .with_backgrounds(step_backgrounds());
    let mut g = common::start(step_program().program, assets, &[(0, room)]);
    let screen = screen_of(&g);
    g.gset(PROBE, Value::Real(probe as u32 as f64)).unwrap();
    g.gset(SCREEN, Value::Real(f64::from(common::FIRST_ID)))
        .unwrap();
    g.gset(PLOG, Value::from("")).unwrap();
    setup(&mut g, screen);
    OUT.with(|o| o.take());
    let mut run = StepRun {
        exceptions: Vec::new(),
        stopped: None,
    };
    for _ in 0..3 * calls {
        let k = num(&g, STEP) as u32 + 1;
        if k > calls {
            break;
        }
        let screen = screen_of(&g);
        let mut lines = Lines::default();
        read(&mut g, screen, k, Phase::Before, &mut lines);
        let stepped = g.step(&[]);
        OUT.with(|o| lines.0.append(&mut o.borrow_mut().0));
        if let Err(e) = stepped {
            if num(&g, STEP) as u32 == k {
                l.0.append(&mut lines.0);
            }
            run.stopped = Some(e);
            return run;
        }
        if let Some(e) = g.exception() {
            let Error::Value(e) = e.error else {
                panic!("{e}")
            };
            run.exceptions.push((num(&g, STEP) as u32, e));
        }
        if num(&g, STEP) as u32 == k {
            let screen = screen_of(&g);
            read(&mut g, screen, k, Phase::After, &mut lines);
            l.0.append(&mut lines.0);
        }
    }
    assert_eq!(
        num(&g, STEP) as u32,
        calls,
        "{probe:?} didn't reach its last call"
    );
    run
}

/// The step probes through the port's frame loop (851-3290; `friction-2p63`, `gravity-2p63` and `friction-snap` are
/// [`motion_steps`]). Each run is checked against GM8 on Intel line for line with the other probes; this checks
/// where the exceptions were, and that the probes GM8 shows an error dialog for stop the port too.
fn main_loop(l: &mut Lines) {
    use Builtin::{
        Direction as D, Friction as F, Gravity as G, GravityDirection as GD, Hspeed as H,
        ImageIndex, Speed as S, Vspeed as V, X, Y,
    };
    use Phase::{After, Before};
    use StepProbe::*;
    use ValueError::{IntOverflow, Overflow, StringTooLong};
    let real = |g: &World, h: Inst, b: Builtin| get_b(g, h, b);
    let (p62, p63, p1023) = (pow2(62), pow2(63), pow2(1023));
    let core = |g: &World| g.find(num(g, CORE) as i32);
    let hspeed_1 = |g: &mut World, h: Inst| set_b(g, h, H, 1.0).unwrap();

    // late-trap: the trap in step 3 leaves oScreen where it was.
    let run = run_step_probe(l, LateTrap, hspeed_1, 4, |g, s, k, phase, l| {
        if phase == Before && k >= 2 {
            l.real(&format!("late_trap_x_{k}"), real(g, s, X));
        }
    });
    assert_eq!(run.exceptions, [(3, IntOverflow)]);

    // late-speedwrite-2p63: a speed write that traps doesn't stop the game.
    let run = run_step_probe(
        l,
        LateSpeedwrite,
        |_, _| {},
        4,
        |g, s, k, phase, l| {
            if (k, phase) == (3, Before) {
                set_b(g, s, S, p63 - 1024.0).unwrap();
                l.real("late_hspeed_speed_2p63-1024", real(g, s, H));
            }
        },
    );
    assert_eq!(run.exceptions, [(3, IntOverflow)]);

    // step-trap: friction's snap traps; what it stored stays, and gravity and the move are skipped.
    let setup = |g: &mut World, s: Inst| {
        for (b, x) in [
            (X, 0.0),
            (Y, 0.0),
            (D, 1.0),
            (S, 1.5 * p62),
            (F, -p62),
            (GD, 270.0),
            (G, p62 / 4194304.0),
        ] {
            set_b(g, s, b, x).unwrap();
        }
    };
    let run = run_step_probe(l, StepTrap, setup, 2, |g, s, k, phase, l| {
        match (k, phase) {
            (1, Before) => {
                l.real("step_trap_before_h", real(g, s, H));
                l.real("step_trap_before_v", real(g, s, V));
            }
            (2, Before) => {
                for (name, b) in [("h", H), ("v", V), ("s", S), ("d", D), ("x", X), ("y", Y)] {
                    l.real(&format!("step_trap_{name}"), real(g, s, b));
                }
            }
            _ => {}
        }
    });
    // Friction traps again in step 2, as it would on GM8.
    assert_eq!(run.exceptions, [(1, IntOverflow), (2, IntOverflow)]);

    // gravity-overflow: gravity's speed traps in step 1 (no move), its sum in step 2 (not stored).
    let setup = |g: &mut World, s: Inst| {
        set_b(g, s, GD, 0.0).unwrap();
        set_b(g, s, G, p1023).unwrap();
    };
    let run = run_step_probe(l, GravityOverflow, setup, 3, |g, s, k, phase, l| {
        match (k, phase) {
            (2, Before) => {
                l.real("gravity_overflow_hspeed_2p1023", real(g, s, H));
                l.real("gravity_overflow_speed_2p1023", real(g, s, S));
            }
            (3, Before) => {
                l.real(
                    "gravity_overflow_hspeed_past_2p1023",
                    b(real(g, s, H) > p1023),
                );
                l.real("gravity_overflow_x_past_2p1023", b(real(g, s, X) > p1023));
            }
            _ => {}
        }
    });
    assert_eq!(
        run.exceptions,
        [(1, IntOverflow), (2, Overflow), (3, Overflow)]
    );

    // late-scope: nothing after the trap runs that frame, for any instance; self and other are oScreen after.
    let run = run_step_probe(l, Scope, hspeed_1, 4, |g, s, k, phase, l| {
        if phase == Before && k >= 3 {
            let c = core(g).unwrap();
            l.real(&format!("scope_x_{k}"), real(g, s, X));
            l.real(&format!("scope_core_x_{k}"), real(g, c, X));
            l.real(&format!("scope_core_image_{k}"), real(g, c, ImageIndex));
            l.text(&format!("scope_log_{k}"), &log_text(g));
        }
        if (k, phase) == (4, After) {
            l.real("scope_self_4", num(g, SELF_OK));
            l.real("scope_other_4", num(g, OTHER_OK));
            l.real("scope_with_other_4", num(g, WITH_OTHER_OK));
        }
    });
    assert_eq!(run.exceptions, [(3, IntOverflow)]);

    // late-room-trap: the room_restart() before the trap is made after the next frame, which runs no events.
    let run = run_step_probe(l, RoomTrap, hspeed_1, 4, |g, s, k, phase, l| {
        if phase == Before && k >= 3 {
            l.real(&format!("room_trap_x_{k}"), real(g, s, X));
            let exists = g.instance_exists(&common::ctx(g, s), num(g, CORE) as i32);
            l.real(&format!("room_trap_core_{k}"), b(exists));
            l.text(&format!("room_trap_log_{k}"), &log_text(g));
        }
    });
    assert_eq!(run.exceptions, [(3, IntOverflow)]);

    // late-create-trap: a trap in a create event while a room loads stops the load there.
    let p = step_program();
    let run = run_step_probe(l, CreateTrap, hspeed_1, 3, |g, s, k, phase, l| {
        match (k, phase) {
            (2, Before) => l.real("create_trap_x_2", real(g, s, X)),
            (3, Before) => {
                let cx = common::ctx(g, s);
                if num(g, IN_CREATE) == 1.0 {
                    l.real("create_trap_in_create", 1.0);
                }
                l.real("create_trap_x_3", real(g, s, X));
                l.real("create_trap_room", b(g.room().id == 0));
                for (name, o) in [
                    ("bricks", BRICKS_OBJ),
                    ("globals", GLOBALS_OBJ),
                    ("game", GAME_OBJ),
                    ("bats", BAT_OBJ),
                ] {
                    l.real(
                        &format!("create_trap_{name}"),
                        f64::from(g.instance_number(&cx, o.0 as i32)),
                    );
                }
                for (name, o, v) in [
                    ("intro", INTRO_OBJ, p.fade_level),
                    ("game", GAME_OBJ, p.draw_status),
                    ("bat", BAT_OBJ, p.y_acc),
                ] {
                    let h = g.find(g.instance_find(o.0 as i32, 0)).unwrap();
                    l.real(
                        &format!("create_trap_{name}_created"),
                        b(g.get(h, v).is_ok()),
                    );
                }
            }
            _ => {}
        }
    });
    assert_eq!(run.exceptions, [(2, IntOverflow)]);

    // late-instcreate-trap: instance_create's create event traps: the instance stays, half created.
    let run = run_step_probe(l, InstcreateTrap, hspeed_1, 3, |g, s, k, phase, l| {
        match (k, phase) {
            (2, Before) => l.real("instcreate_x_2", real(g, s, X)),
            (3, Before) => {
                let cx = common::ctx(g, s);
                l.real("instcreate_x_3", real(g, s, X));
                l.text("instcreate_log", &log_text(g));
                l.real(
                    "instcreate_cores",
                    f64::from(g.instance_number(&cx, CORE_OBJ.0 as i32)),
                );
                let c = g.find(g.instance_find(CORE_OBJ.0 as i32, 0)).unwrap();
                l.real("instcreate_core_bps", b(g.get(c, p.bps).is_ok()));
                l.real("instcreate_core_handle", b(g.get(c, p.handle).is_ok()));
            }
            _ => {}
        }
    });
    assert_eq!(run.exceptions, [(2, IntOverflow)]);

    // late-draw-trap: a trap in the draw stage skips the animation after it.
    let run = run_step_probe(
        l,
        DrawTrap,
        |_, _| {},
        4,
        |g, s, k, phase, l| {
            if phase == Before && k >= 2 {
                l.real(&format!("draw_trap_image_{k}"), real(g, s, ImageIndex));
                l.real(
                    &format!("draw_trap_core_image_{k}"),
                    real(g, core(g).unwrap(), ImageIndex),
                );
                l.text(&format!("draw_trap_log_{k}"), &log_text(g));
            }
        },
    );
    assert_eq!(run.exceptions, [(2, IntOverflow)]);

    // late-mulstr: running out of memory is the same exception.
    let run = run_step_probe(l, Mulstr, hspeed_1, 4, |g, s, k, phase, l| {
        if phase == Before && k >= 3 {
            l.real(&format!("late_mulstr_x_{k}"), real(g, s, X));
        }
    });
    assert_eq!(run.exceptions, [(3, StringTooLong)]);

    // step-trap-order: GM8 moves the instances one at a time.
    let run = run_step_probe(l, Order, hspeed_1, 2, |g, s, k, phase, l| {
        match (k, phase) {
            (1, Before) => l.real("order_x_1", real(g, s, X)),
            (2, Before) => {
                let id = |k| g.gget_a(CORE, 0, k).unwrap().to_real();
                let c: Vec<Inst> = (0..3).map(|k| g.find(id(k) as i32).unwrap()).collect();
                l.real("order_ids", b(id(0) < id(1) && id(1) < id(2)));
                l.real("order_x_2", real(g, s, X));
                for (name, i, b) in [
                    ("c1_x", 0, X),
                    ("c1_y", 0, Y),
                    ("c1_v", 0, V),
                    ("c2_h", 1, H),
                    ("c2_v", 1, V),
                    ("c2_s", 1, S),
                    ("c2_x", 1, X),
                    ("c3_x", 2, X),
                    ("c3_y", 2, Y),
                    ("c3_v", 2, V),
                ] {
                    l.real(&format!("order_{name}"), real(g, c[i], b));
                }
            }
            _ => {}
        }
    });
    assert_eq!(run.exceptions, [(1, IntOverflow), (2, IntOverflow)]);

    // move-overflow: the hspeed write traps in step 1, the move in step 2.
    let run = run_step_probe(
        l,
        MoveOverflow,
        |_, _| {},
        3,
        |g, _, k, phase, l| match (k, phase) {
            (1, After) => l.real("move_overflow_x_1", real(g, core(g).unwrap(), X)),
            (2 | 3, Before) => {
                let c = core(g).unwrap();
                l.real(&format!("move_overflow_x_{k}"), real(g, c, X));
                l.real(&format!("move_overflow_hspeed_{k}"), real(g, c, H));
                l.text(&format!("move_overflow_log_{k}"), &log_text(g));
            }
            _ => {}
        },
    );
    assert_eq!(
        run.exceptions,
        [(1, IntOverflow), (2, Overflow), (3, Overflow)]
    );

    // late-div0: GML's own errors still stop it, as GM8's dialog does.
    let run = run_step_probe(
        l,
        Div0,
        |_, _| {},
        3,
        |_, _, k, phase, l| {
            if (k, phase) == (3, Before) {
                l.real("late_div0_before", 1.0);
            }
        },
    );
    assert!(run.exceptions.is_empty());
    assert_eq!(
        run.stopped.map(|e| e.error),
        Some(Error::Value(ValueError::DivisionByZero))
    );

    // 851-3301: the runner's own arithmetic, each probe's step code printing its own lines (`runner_probe`).
    let mut runner = |probe, calls, setup: fn(&mut World, Inst)| {
        run_step_probe(l, probe, setup, calls, |_, _, _, _, _| {})
    };
    let none: fn(&mut World, Inst) = |_, _| {};
    // anim-trap: the animation's sum traps unstored in step 1, its Floor after the store in step 2.
    let run = runner(AnimTrap, 4, none);
    assert_eq!(run.exceptions, [(1, Overflow), (2, IntOverflow)]);
    // draw-floor-trap: the default draw's Floor traps in step 2.
    let run = runner(DrawFloorTrap, 4, none);
    assert_eq!(run.exceptions, [(2, IntOverflow)]);
    // sprite-floor: the sprite setter's Floor, after the store.
    let run = runner(SpriteFloor, 3, none);
    assert_eq!(run.exceptions, [(1, IntOverflow)]);
    // bbox-trap: x, y, a rotated scale (its stored offset overflows) and the mask.
    let run = runner(BboxTrap, 5, none);
    assert_eq!(
        run.exceptions,
        [
            (1, IntOverflow),
            (2, IntOverflow),
            (3, Overflow),
            (4, IntOverflow)
        ]
    );
    // bbox-scale: the right side only.
    let run = runner(BboxScale, 2, none);
    assert_eq!(run.exceptions, [(1, IntOverflow)]);
    // bbox-place: x, place_meeting's move back, its move there.
    let run = runner(BboxPlace, 4, none);
    assert_eq!(
        run.exceptions,
        [(1, IntOverflow), (2, IntOverflow), (3, IntOverflow)]
    );
    // bbox-move: the step's move.
    let run = runner(BboxMove, 2, none);
    assert_eq!(run.exceptions, [(1, IntOverflow)]);
    // bbox-create: GM8 on Intel runs no frame again; the port stops.
    let run = runner(BboxCreate, 3, none);
    assert!(run.exceptions.is_empty());
    assert_eq!(
        run.stopped.map(|e| e.error),
        Some(Error::CreationTrap(IntOverflow))
    );
    // bg-trap: layer 3's x sum in step 1, its y sum in step 2 and again in step 3 (where the probe ends the game).
    let run = runner(BgTrap, 3, none);
    assert_eq!(
        run.exceptions,
        [(1, Overflow), (2, Overflow), (3, Overflow)]
    );
    // view-trap: view 2's target at 2^63, in step 1. `rIntro` has views on, with view 0 shown.
    let views: fn(&mut World, Inst) = |g, _| {
        use Builtin::{ViewEnabled, ViewHport, ViewHview, ViewVisible, ViewWport, ViewWview};
        g.bset(ViewEnabled.var(), 0, Value::Real(1.0)).unwrap();
        for (b, x) in [
            (ViewVisible, 1.0),
            (ViewWview, 320.0),
            (ViewHview, 240.0),
            (ViewWport, 640.0),
            (ViewHport, 480.0),
        ] {
            g.bset(b.var(), 0, Value::Real(x)).unwrap();
        }
    };
    let run = runner(ViewTrap, 3, views);
    assert_eq!(run.exceptions, [(1, IntOverflow)]);
    // collision-frame-trap (851-3305): one trap a step from step 2 (in step 8, where -2^63 fits, the line's far
    // end).
    let run = runner(CollisionFrameTrap, 12, none);
    let steps: Vec<_> = (2..=11).map(|k| (k, IntOverflow)).collect();
    assert_eq!(run.exceptions, steps);
    // collision-edges-trap (851-3306): only the precise rectangles that meet the box of an instance with a mask round
    // their far corner, in steps 5 and 6 (not the one without a mask, in step 8).
    let run = runner(CollisionEdgesTrap, 9, none);
    assert_eq!(run.exceptions, [(5, IntOverflow), (6, IntOverflow)]);

    // 851-3304: drawing's own conversions. draw-builtin-trap: one draw a step, from `core`'s draw event.
    let run = runner(DrawBuiltinTrap, 38, none);
    assert_eq!(run.exceptions, DRAW_BUILTIN_TRAPS);
    // draw-default-trap: the setters' boxes in the even steps to 12 and in 14 and 15, the default draw in the frames
    // after x = T, xscale = 2^1023 and the angle.
    let run = runner(DrawDefaultTrap, 18, none);
    let traps: Vec<_> = run
        .exceptions
        .iter()
        .filter(|&&(k, _)| [3, 5, 7, 9, 11, 13, 16, 17].contains(&k))
        .copied()
        .collect();
    assert_eq!(traps, [(3, Overflow), (7, Overflow), (13, Overflow)]);
    let setters: Vec<_> = run
        .exceptions
        .iter()
        .map(|&(k, _)| k)
        .filter(|k| k % 2 == 0 || *k == 15)
        .collect();
    assert_eq!(setters, [2, 4, 6, 8, 10, 12, 14, 15]);
    // draw-stage-trap: the layer's Rounds, the foreground's after the instances, the view's projection.
    let run = runner(DrawStageTrap, 13, views);
    assert_eq!(
        run.exceptions,
        [
            (2, IntOverflow),
            (3, IntOverflow),
            (5, IntOverflow),
            (6, IntOverflow),
            (8, IntOverflow),
            (9, Overflow)
        ]
    );
}

/// The step code of the probes of drawing's own conversions (851-3304), line for line as the GML: each prints its
/// lines itself ([`out`]). `draw-builtin-trap`'s draws are in `core`'s draw event ([`draw_case`]).
fn draw_probe(g: &mut World, probe: StepProbe, k: u32) -> R<()> {
    use Builtin::{
        BackgroundIndex, Depth, Id, ImageAngle, ImageIndex, ImageSpeed, ImageXscale, ImageYscale,
        ViewAngle, X,
    };
    use StepProbe::*;
    let p = pow2(63);
    let (t, tb) = (draw_limit(), draw_limit() - pow2(75));
    let keep = |g: &mut World, values: &[f64]| -> R<()> {
        for (n, &v) in values.iter().enumerate() {
            g.gset_a(CORE, 0, n as u32, Value::Real(v))?;
        }
        Ok(())
    };
    let kept = |g: &World, n: u32| g.gget_a(CORE, 0, n).unwrap().to_real();
    let inst = |g: &World, n: u32| g.find(kept(g, n) as i32).unwrap();
    match probe {
        DrawBuiltinTrap => {
            if k == 1 {
                let c = create_core(g)?;
                set_b(g, c, ImageSpeed, 0.0)?;
                let font = g.font_add_sprite(spelunky::sprite_id("sFont"), 32, false, 0)?;
                let surf = g.surface_create(16, 16)?;
                keep(g, &[get_b(g, c, Id), f64::from(font), f64::from(surf)])?;
            }
            if k >= 2 {
                out_text(&format!("draw_{}", k - 1), &log_text(g));
                g.gset(PLOG, Value::from(""))?;
            }
            let index = match k {
                1 => p,
                2 => p / 2.0,
                _ => 0.0,
            };
            set_b(g, inst(g, 0), ImageIndex, index)?;
        }
        DrawDefaultTrap => {
            if k == 1 {
                let c1 = create_core(g)?;
                let id = g.instance_create(0.0, 0.0, BLACK_OBJ.0 as i32)?;
                let b = g.find(id).expect("created");
                let c3 = create_core(g)?;
                for (h, depth) in [(c1, 30.0), (b, 20.0), (c3, 10.0)] {
                    set_b(g, h, Depth, depth)?;
                }
                set_b(g, b, ImageSpeed, 0.0)?;
                let ids = [c1, b, c3].map(|h| get_b(g, h, Id));
                keep(g, &ids)?;
            }
            let (c1, b, c3) = (inst(g, 0), inst(g, 1), inst(g, 2));
            if k >= 2 {
                out_text(&format!("draw_default_log_{k}"), &log_text(g));
                out(&format!("draw_default_c1_{k}"), get_b(g, c1, ImageIndex));
                out(&format!("draw_default_c3_{k}"), get_b(g, c3, ImageIndex));
                g.gset(PLOG, Value::from(""))?;
            }
            let s = pow2(123) * 1.5;
            match k {
                2 => set_b(g, b, X, t)?,
                4 => set_b(g, b, X, tb)?,
                6 => {
                    set_b(g, b, X, 0.0)?;
                    set_b(g, b, ImageXscale, pow2(1023))?;
                }
                8 => set_b(g, b, ImageXscale, s)?,
                10 => set_b(g, b, ImageYscale, s)?,
                12 => set_b(g, b, ImageAngle, 45.0)?,
                14 => set_b(g, b, ImageAngle, 0.0)?,
                15 => set_b(g, b, ImageXscale, 1.0)?,
                16 => set_b(g, b, ImageYscale, 1.0)?,
                _ => {}
            }
        }
        DrawStageTrap => {
            if k == 1 {
                let c1 = create_core(g)?;
                let c3 = create_core(g)?;
                set_b(g, c1, Depth, 30.0)?;
                set_b(g, c3, Depth, 10.0)?;
                let ids = [c1, c3].map(|h| get_b(g, h, Id));
                keep(g, &ids)?;
                g.bset(BackgroundIndex.var(), 1, Value::Real(NIGHT as f64))?;
                let bg = g.background_mut(1);
                (bg.visible, bg.foreground, bg.tile_h, bg.tile_v) = (true, false, true, true);
                (bg.x, bg.y, bg.xscale, bg.yscale) = (0.0, 0.0, 1.0, 1.0);
                (bg.hspeed, bg.vspeed) = (0.0, 0.0);
            }
            let (c1, c3) = (inst(g, 0), inst(g, 1));
            if k >= 2 {
                out_text(&format!("draw_stage_log_{k}"), &log_text(g));
                out(&format!("draw_stage_c1_{k}"), get_b(g, c1, ImageIndex));
                out(&format!("draw_stage_c3_{k}"), get_b(g, c3, ImageIndex));
                g.gset(PLOG, Value::from(""))?;
            }
            let bg = g.background_mut(1);
            match k {
                2 => bg.x = p,
                3 => (bg.x, bg.y) = (0.0, p),
                4 => (bg.tile_h, bg.x) = (false, 2.0 * p / 2147483648.0 + 5000.0),
                5 => bg.x = 2.0 * p / 2147483648.0,
                6 => (bg.tile_h, bg.x, bg.y, bg.xscale) = (true, 0.0, 0.0, p / 16.0),
                7 => (bg.xscale, bg.y) = (134217728.0, p),
                8 => (bg.xscale, bg.y, bg.foreground, bg.x) = (1.0, 0.0, true, p),
                9 => {
                    (bg.visible, bg.foreground, bg.x) = (false, false, 0.0);
                    g.bset(ViewAngle.var(), 0, Value::Real(pow2(126) * 2048.0))?;
                }
                10 => g.bset(ViewAngle.var(), 0, Value::Real(pow2(126)))?,
                11 => g.bset(ViewAngle.var(), 0, Value::Real(0.0))?,
                _ => {}
            }
        }
        _ => unreachable!("{probe:?}"),
    }
    Ok(())
}

/// 2^128 - 2^103, where a corner stored as a single traps (`T` in the probes).
fn draw_limit() -> f64 {
    pow2(103) * 33554431.0
}

/// `draw-builtin-trap`'s `SS_GetSoundPosition`, in `core`'s draw event (851-3304): the step's case between an "a" and
/// a "b" in the log, as the GML's `switch`.
fn draw_case(g: &mut World, cx: &Ctx) -> R<()> {
    let k = num(g, STEP) as u32;
    let (p, big, s) = (pow2(63), pow2(1023), pow2(123) * 1.5);
    let (t, tb) = (draw_limit(), draw_limit() - pow2(75));
    let block = spelunky::sprite_id("sMetalBlock");
    let white = 0xFF_FF_FF;
    let font = g.gget_a(CORE, 0, 1)?.to_real() as i32;
    let surf = g.gget_a(CORE, 0, 2)?.to_real() as i32;
    let text = |b: &[u8]| Value::Str(GmString::from(b));
    plog(g, "a")?;
    match k {
        1 | 2 => g.draw_sprite(cx, block, -1, 0.0, 0.0)?,
        3 => g.draw_sprite(cx, block, 0, t, 0.0)?,
        4 => g.draw_sprite(cx, block, 0, tb, 0.0)?,
        5 => g.draw_sprite(cx, block, 0, 0.0, -t)?,
        6 => g.draw_sprite_ext(cx, block, 0, 0.0, 0.0, big, 1.0, 0.0, white, 1.0)?,
        7 => g.draw_sprite_ext(cx, block, 0, 0.0, 0.0, s, s, 0.0, white, 1.0)?,
        8 => g.draw_sprite_ext(cx, block, 0, 0.0, 0.0, s, s, 45.0, white, 1.0)?,
        9 => g.draw_text(0.0, 0.0, &Value::Real(p))?,
        10 => g.draw_text(0.0, 0.0, &Value::Real(p / 2.0))?,
        11..=14 | 30..=37 => {
            g.draw_set_font(if (34..=37).contains(&k) { -1 } else { font });
            let (x, chars): (f64, &[u8]) = match k {
                11 => (t, b"A"),
                12 => (t, b""),
                13 => (tb, b"A"),
                14 => (t, &[200]),
                30 | 34 => (t, &[31]),
                31 => (t, b"Z"),
                32 => (t, b"["),
                33 => (t, b"#"),
                35 => (t, &[32]),
                36 => (t, &[127]),
                _ => (t, &[128]),
            };
            g.draw_text(x, 0.0, &text(chars))?;
        }
        15 | 16 => {
            g.draw_set_font(-1);
            g.draw_text(t, 0.0, &text(if k == 15 { b"A" } else { &[200] }))?;
        }
        17 => g.draw_rectangle(0.0, 0.0, p, 10.0, false)?,
        18 => g.draw_rectangle(p, 0.0, 0.0, 10.0, false)?,
        19 => g.draw_rectangle(0.0, 0.0, p / 2.0, 10.0, false)?,
        20 => g.draw_rectangle(-t, 0.0, 0.0, 10.0, false)?,
        21 => g.draw_rectangle(0.0, -2.0 * p, 10.0, -2.0 * p, false)?,
        22 => g.draw_circle(t, 0.0, 1.0, false)?,
        23 => g.draw_circle(0.0, 0.0, t, false)?,
        24 => g.draw_circle(0.0, 0.0, tb, false)?,
        25 => g.draw_circle(big * 1.5, 0.0, big * 1.5, false)?,
        26 => g.draw_surface(surf, t, 0.0)?,
        27 => g.draw_surface(surf, tb, 0.0)?,
        28 => g.draw_surface_stretched(surf, 0.0, 0.0, 2.0 * t, 10.0)?,
        29 => g.draw_surface_stretched(surf, 0.0, 0.0, -2.0 * t, 10.0)?,
        _ => {}
    }
    plog(g, "b")
}

// ---- Angles (851-3262): rotated boxes and masks, gravity, rotated views ----
//
// `probes/x87/angle-*.gml` (generated by `tools/gm8-probe/angles`) ran on GM8 on a real Intel CPU and on the
// patched oracle. GM8's runner converts every angle as it does `direction` (`x87::gm8::sin_cos_degrees`,
// `add_motion`), and the port must give its answers through its real collision, motion and view code. Its
// bounding box and view transform are GM8's formulas too (851-3278, 851-3279).

/// An angle probe: its GML, and its results on Intel and on the patched oracle (OpenGMK has no
/// `window_mouse_set`, so it can't run the view probe).
struct AngleProbe {
    gml: &'static str,
    intel: &'static str,
    oracle: Option<&'static str>,
}

macro_rules! angle_probe {
    ($name:literal) => {
        angle_probe!(
            $name,
            Some(include_str!(concat!(
                "../../../tools/gm8-probe/results/opengmk/x87/",
                $name,
                ".txt"
            )))
        )
    };
    ($name:literal, $oracle:expr) => {
        AngleProbe {
            gml: include_str!(concat!(
                "../../../tools/gm8-probe/probes/x87/",
                $name,
                ".gml"
            )),
            intel: include_str!(concat!(
                "../../../tools/gm8-probe/results/gm8-intel/x87/",
                $name,
                ".txt"
            )),
            oracle: $oracle,
        }
    };
}

/// A probe's lines by name: a string's text, or a real's `<bits> <string>`.
fn named(text: &str) -> BTreeMap<&str, &str> {
    text.lines()
        .filter_map(|l| l.split_once(" = "))
        .map(|(k, v)| (k, v.trim_matches('"')))
        .collect()
}

/// One statement of a generated angle probe.
enum Stmt<'a> {
    /// `name = expression`.
    Set(&'a str, &'a str),
    /// `out("name", ...)`, with the whole call.
    Out(&'a str, &'a str),
    /// Anything else (`screen_redraw()`, `window_mouse_set(x, y)`, ...).
    Other(&'a str),
}

/// The statements of a generated probe, in order. The generator puts them one or a few per line, separated
/// by `; `, with no `;` or ` = ` inside an `out` call.
fn statements(gml: &str) -> impl Iterator<Item = Stmt<'_>> {
    gml.lines()
        .filter(|l| !l.starts_with("//"))
        .flat_map(|l| l.split("; "))
        .map(|s| s.trim().trim_end_matches(';'))
        .filter(|s| !s.is_empty())
        .map(|s| {
            if let Some(rest) = s.strip_prefix("out(\"") {
                Stmt::Out(&rest[..rest.find('"').unwrap()], s)
            } else if let Some((lhs, rhs)) = s.split_once(" = ") {
                Stmt::Set(lhs, rhs)
            } else {
                Stmt::Other(s)
            }
        })
}

/// The value of a generated expression: whole numbers, parentheses, unary minus, `+`, `*` and `/`, which
/// GM8 evaluates in plain doubles (the probes only divide by powers of two, so every step is exact).
fn eval(expr: &str) -> f64 {
    fn sum(s: &mut &[u8]) -> f64 {
        let mut v = product(s);
        while let Some(&c @ (b'+' | b'-')) = next(s) {
            *s = &s[1..];
            let r = product(s);
            v = if c == b'+' { v + r } else { v - r };
        }
        v
    }
    fn product(s: &mut &[u8]) -> f64 {
        let mut v = factor(s);
        while let Some(&c @ (b'*' | b'/')) = next(s) {
            *s = &s[1..];
            let r = factor(s);
            v = if c == b'*' { v * r } else { v / r };
        }
        v
    }
    fn factor(s: &mut &[u8]) -> f64 {
        match next(s) {
            Some(b'-') => {
                *s = &s[1..];
                -factor(s)
            }
            Some(b'(') => {
                *s = &s[1..];
                let v = sum(s);
                assert_eq!(next(s), Some(&b')'));
                *s = &s[1..];
                v
            }
            _ => {
                let n = s.iter().take_while(|c| c.is_ascii_digit()).count();
                let v = std::str::from_utf8(&s[..n])
                    .unwrap()
                    .parse::<f64>()
                    .unwrap();
                *s = &s[n..];
                v
            }
        }
    }
    fn next<'a>(s: &mut &'a [u8]) -> Option<&'a u8> {
        while s.first() == Some(&b' ') {
            *s = &s[1..];
        }
        s.first()
    }
    let mut s = expr.as_bytes();
    let v = sum(&mut s);
    assert!(s.is_empty(), "{expr}");
    v
}

/// The frame each precise test takes from `image_index` (`collision-frame`, 851-3305), through the port's real
/// queries: an oBlack with sMattockHitR's mask, whose frame f alone has the pixel `PICKS[f]` from its origin, and an
/// oMenuTop with sBlock's mask scaled to one pixel. Each line is the frames whose pixel a test hits, as bits.
fn collision_frames(l: &mut Lines) {
    const PICKS: [(f64, f64); 3] = [(-3.0, -6.0), (-4.0, 5.0), (-3.0, -4.0)];
    use gm::Builtin::{ImageIndex, ImageXscale, ImageYscale, MaskIndex, X, Y};
    let p = common::build(|b| {
        b.push_object(gm::ObjectInfo::new("oScreen"));
        b.push_object(gm::ObjectInfo::new("oBlack"));
        b.push_object(gm::ObjectInfo::new("oMenuTop"));
    });
    let assets = gm::Assets::new(spelunky::sprites(&["sMattockHitR", "sBlock"]), Vec::new());
    let room = common::room(320, 240, &[(common::FIRST_ID, gm::ObjectId(0), 0, 0)]);
    let mut g = common::start(p, assets, &[(0, room)]);
    let screen = common::ctx(&g, g.find(common::FIRST_ID).unwrap());
    let set = |g: &mut gm::World, h: gm::Inst, b: gm::Builtin, x: f64| {
        g.set(h, b.var(), Value::Real(x)).unwrap()
    };
    let create = |g: &mut gm::World, x: f64, y: f64, o: i32, mask: &str| {
        let id = g.instance_create(x, y, o).unwrap();
        let h = g.find(id).unwrap();
        set(g, h, MaskIndex, f64::from(spelunky::sprite_id(mask)));
        h
    };
    let a = create(&mut g, 100.0, 100.0, 1, "sMattockHitR");
    let other = create(&mut g, 200.0, 60.0, 2, "sBlock");
    set(&mut g, other, ImageXscale, 1.0 / 16.0);
    set(&mut g, other, ImageYscale, 1.0 / 16.0);
    let frames = g
        .assets()
        .sprite(spelunky::sprite_id("sMattockHitR"))
        .unwrap()
        .frames
        .len();
    l.real("frame_count", frames as f64);
    let bb = g.bbox(other);
    l.text(
        "frame_box_b",
        &format!("{} {} {} {}", bb.left, bb.top, bb.right, bb.bottom),
    );
    let (p31, p32, p52, p62, p63) = (pow2(31), pow2(32), pow2(52), pow2(62), pow2(63));
    let values = [
        0.0,
        1.0,
        2.0,
        2.5,
        3.0,
        4.75,
        5.0,
        -0.25,
        -0.5,
        -1.0,
        -1.5,
        -2.0,
        -2.5,
        -3.0,
        -3.5,
        -4.0,
        -7.25,
        p31 - 0.5,
        p31,
        p31 + 0.5,
        p31 + 1.5,
        p32 - 0.5,
        p32,
        p32 + 1.5,
        p32 + 2.5,
        p32 * 2.0 + 1.0,
        -p31 - 0.5,
        -p32 + 0.5,
        -p32 - 1.0,
        p52 + 1.0,
        p52 * 2.0 + 2.0,
        p62,
        p62 + 1024.0,
        p63 - 1024.0,
        -p63 + 1024.0,
        -p63,
    ];
    for (i, &v) in values.iter().enumerate() {
        set(&mut g, a, ImageIndex, v);
        let mut sums = [0.0; 5];
        for (f, &(px, py)) in PICKS.iter().enumerate() {
            let (x, y) = (100.0 + px, 100.0 + py);
            let me = common::ctx(&g, a);
            let hits = [
                g.collision_point(&screen, x, y, 1, true, false).unwrap() != -4,
                g.collision_rectangle(&screen, x, y, x, y, 1, true, false)
                    .unwrap()
                    != -4,
                g.collision_line(&screen, x, y, x, y, 1, true, false)
                    .unwrap()
                    != -4,
                g.place_meeting(&me, 200.0 - px, 60.0 - py, 2).unwrap(),
                {
                    set(&mut g, a, X, 200.0 - px);
                    set(&mut g, a, Y, 60.0 - py);
                    let them = common::ctx(&g, other);
                    let (ox, oy) = (g.instance(other).x, g.instance(other).y);
                    let hit = g.place_meeting(&them, ox, oy, 1).unwrap();
                    set(&mut g, a, X, 100.0);
                    set(&mut g, a, Y, 100.0);
                    hit
                },
            ];
            for (sum, hit) in sums.iter_mut().zip(hits) {
                *sum += f64::from(1 << f) * b(hit);
            }
        }
        l.real(&format!("frame_index_{i}"), g.instance(a).image_index);
        for (test, sum) in ["point", "rect", "line", "self", "other"].iter().zip(sums) {
            l.real(&format!("frame_{test}_{i}"), sum);
        }
    }
}

/// View following (`view-follow`, 851-3303): the probe's seven views, each following its target for two frames
/// in `rIntro` (960x240), through `View::follow` directly (`main_loop`'s `view-trap` takes the whole draw stage).
fn view_follow(l: &mut Lines) {
    // xview, yview, wview, hview, hborder, vborder, hspeed, vspeed, target x, target y
    let b = (1 << 30) + 10;
    let cases: [[i32; 10]; 7] = [
        [900, 0, 100, 100, 10, 10, 5, 5, 905, 50],
        [0, 200, 100, 100, 10, 10, 5, 5, 50, 205],
        [2000, 0, 100, 100, 10, 10, 0, 0, 905, 50],
        [0, 0, 101, 101, 50, 50, -5, -1_000_000, 300, 150],
        [0, 0, 100, 100, b, b, -1, -1, 300, 150],
        [-500, 0, 100, 100, 10, 10, 5, 5, 50, 50],
        [0, 0, 1000, 100, 10, 10, 5, 5, 500, 50],
    ];
    for (
        i,
        [
            xview,
            yview,
            wview,
            hview,
            hborder,
            vborder,
            hspeed,
            vspeed,
            x,
            y,
        ],
    ) in (1..).zip(cases)
    {
        let mut v = gm::View {
            visible: true,
            xview,
            yview,
            wview,
            hview,
            hborder,
            vborder,
            hspeed,
            vspeed,
            ..gm::View::default()
        };
        for k in 2..=3 {
            v.follow(f64::from(x), f64::from(y), 960, 240).unwrap();
            l.real(&format!("follow_x{i}_{k}"), f64::from(v.xview));
            l.real(&format!("follow_y{i}_{k}"), f64::from(v.yview));
        }
    }
}

/// View following by id (`view-follow-id`, 851-3309), in a world of the probe's two objects: `oBlack` and the
/// persistent `oGlobals`, placed in `rIntro` (960x240, views on). Views 1-3 follow a live, a destroyed and a
/// deactivated `oBlack` through `screen_redraw()`; then `room_restart()` leaves the placed `oGlobals` destroyed
/// beside the persistent one with its id, which views 4 (by id) and 5 (by object) follow from the load's draw on.
fn view_follow_id(l: &mut Lines) {
    use Builtin::{
        ViewHborder, ViewHport, ViewHspeed, ViewHview, ViewObject, ViewVborder, ViewVisible,
        ViewVspeed, ViewWport, ViewWview, ViewXview,
    };
    let (black, globals) = (ObjectId(0), ObjectId(1));
    let p = common::build(|b| {
        b.push_object(ObjectInfo::new("oBlack"));
        b.push_object(ObjectInfo {
            persistent: true,
            ..ObjectInfo::new("oGlobals")
        });
    });
    let mut intro = common::room(960, 240, &common::numbered(&[(globals, 0, 0)]));
    intro.views_enabled = true;
    let mut g = common::start(p, gm::Assets::default(), &[(0, intro)]);
    let xview = |g: &World, i: usize| f64::from(g.room().views[i].xview);
    let count = |g: &World| {
        let h = g.active().next().unwrap();
        f64::from(g.instance_number(&common::ctx(g, h), globals.0 as i32))
    };
    // Step 1.
    for i in 1..=3 {
        let id = g.instance_create(300.0, 50.0, black.0 as i32).unwrap();
        for (b, x) in [
            (ViewVisible, 1.0),
            (ViewXview, 500.0),
            (ViewWview, 100.0),
            (ViewHview, 100.0),
            (ViewWport, 100.0),
            (ViewHport, 100.0),
            (ViewHborder, 10.0),
            (ViewVborder, 10.0),
            (ViewHspeed, -1.0),
            (ViewVspeed, -1.0),
            (ViewObject, f64::from(id)),
        ] {
            g.bset(b.var(), i, Value::Real(x)).unwrap();
        }
        let h = g.find(id).unwrap();
        match i {
            2 => g.destroy(h).unwrap(),
            3 => g.instance_deactivate_object(&common::ctx(&g, h), id),
            _ => {}
        }
    }
    g.screen_redraw().unwrap();
    for i in 1..=3 {
        l.real(&format!("id_redraw_x{i}"), xview(&g, i));
    }
    l.real("id_globals", count(&g));
    let h = g
        .active()
        .find(|&h| g.instance(h).object == globals)
        .unwrap();
    let id = g.instance(h).id;
    (g.instance_mut(h).x, g.instance_mut(h).y) = (300.0, 50.0);
    let view = gm::View {
        visible: true,
        xview: 500,
        wview: 100,
        hview: 100,
        wport: 100,
        hport: 100,
        hborder: 10,
        vborder: 10,
        hspeed: -1,
        vspeed: -1,
        ..gm::View::default()
    };
    g.room_set_view(0, 4, gm::View { object: id, ..view });
    let object = globals.0 as i32;
    g.room_set_view(0, 5, gm::View { object, ..view });
    g.room_restart();
    g.step(&[]).unwrap();
    // Step 2: the placed `oGlobals`, destroyed, is still there.
    l.real("id_globals_2", count(&g));
    for i in 4..=5 {
        l.real(&format!("id_load_x{i}"), xview(&g, i));
        g.bset(ViewXview.var(), i as u32, Value::Real(500.0))
            .unwrap();
    }
    g.screen_redraw().unwrap();
    for i in 4..=5 {
        l.real(&format!("id_redraw_x{i}"), xview(&g, i));
    }
    g.step(&[]).unwrap();
    // Step 3.
    for i in 4..=5 {
        l.real(&format!("id_next_x{i}"), xview(&g, i));
    }
}

/// `oGlobals`' id in `rIntro`, which `id-lookups` prints.
const ID_LOOKUPS_GLOBALS: i32 = 131_010;

thread_local! {
    /// Whether `id-lookups`' `oBricks` runs the lookups in its next create event.
    static ID_LOOKUPS_ARMED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Lookups by instance id while a persistent instance shares its id with the placed one the room load destroyed
/// (`id-lookups`, 851-3310), in a world of the probe's two objects: `oBricks`, whose create event runs the lookups
/// during `rIntro`'s reload (prefix `c`), and the persistent `oGlobals`, placed with its id. They run again after
/// the load (`s`), before the cleanup, as in the next begin step. Lines the port doesn't compute: GM8's cache of
/// array positions for variable access (`*_cached_write`, `*_cached_read`), which the port doesn't model;
/// `*_deact_count` and `*_act_count`, which count all of `rIntro` (the port deactivates the destroyed instance too,
/// as GM8 does: 851-3311); and `position_meeting`, which the port doesn't have.
fn id_lookups(l: &mut Lines) {
    let (bricks, globals) = (ObjectId(0), ObjectId(1));
    let p = common::build(|b| {
        let o = b.push_object(ObjectInfo::new("oBricks"));
        b.event(o, Event::Create, id_lookups_create);
        b.push_object(ObjectInfo {
            persistent: true,
            ..ObjectInfo::new("oGlobals")
        });
    });
    let assets = gm::Assets::new(spelunky::sprites(&["sBlack"]), Vec::new());
    let intro = common::room(
        960,
        240,
        &[
            (131_007, bricks, 16, 0),
            (ID_LOOKUPS_GLOBALS, globals, 32, 0),
        ],
    );
    let mut g = common::start(p, assets, &[(0, intro)]);
    // Step 1: the persistent `oGlobals` at (300, 50) with `sBlack`.
    let q = g.find(ID_LOOKUPS_GLOBALS).unwrap();
    set_b(&mut g, q, Builtin::X, 300.0).unwrap();
    set_b(&mut g, q, Builtin::Y, 50.0).unwrap();
    let black = spelunky::sprite_id("sBlack").into();
    set_b(&mut g, q, Builtin::SpriteIndex, black).unwrap();
    ID_LOOKUPS_ARMED.with(|a| a.set(true));
    OUT.with(|o| o.take());
    g.room_restart();
    g.step(&[]).unwrap();
    assert!(!ID_LOOKUPS_ARMED.with(|a| a.get()));
    let cx = common::ctx(&g, g.find(131_007).unwrap());
    id_lookups_run(&mut g, &cx, "s").unwrap();
    OUT.with(|o| l.0.append(&mut o.borrow_mut().0));
}

/// `id-lookups`' `oBricks` create event: the lookups, once armed.
fn id_lookups_create(g: &mut World, cx: &mut Ctx) -> R<()> {
    if ID_LOOKUPS_ARMED.with(|a| a.replace(false)) {
        id_lookups_run(g, cx, "c")?;
    }
    Ok(())
}

/// `id-lookups`' lookups (its `SS_IsHandleValid`), as `self`, printing with the prefix `p`.
fn id_lookups_run(g: &mut World, cx: &Ctx, p: &str) -> R<()> {
    let id = ID_LOOKUPS_GLOBALS;
    let target = Value::Real(id.into());
    let object = Value::Real(1.0);
    let x = Builtin::X.var();
    let name = |s: &str| format!("{p}_{s}");
    let flag = |b: bool| f64::from(u8::from(b));
    g.set_on(cx, &object, x, Value::Real(300.0))?;
    g.set_on(cx, &object, Builtin::Y.var(), Value::Real(50.0))?;
    out(&name("globals"), g.instance_number(cx, 1).into());
    out(&name("exists"), flag(g.instance_exists(cx, id)));
    out(&name("number"), g.instance_number(cx, id).into());
    out(&name("find"), g.instance_find(id, 0).into());
    let mut with = g.with_iter(cx, &target)?;
    let mut n = 0;
    while with.next(g).is_some() {
        n += 1;
    }
    out(&name("with"), n.into());
    out(&name("distance"), g.distance_to_object(cx, id));
    out(
        &name("point"),
        g.collision_point(cx, 304.0, 54.0, id, false, false)?.into(),
    );
    out(
        &name("rectangle"),
        g.collision_rectangle(cx, 296.0, 46.0, 304.0, 54.0, id, false, false)?
            .into(),
    );
    out(
        &name("line"),
        g.collision_line(cx, 290.0, 54.0, 310.0, 54.0, id, false, false)?
            .into(),
    );
    out(
        &name("position"),
        g.instance_position(304.0, 54.0, id)?.into(),
    );
    set_b(
        g,
        cx.this,
        Builtin::SpriteIndex,
        spelunky::sprite_id("sBlack").into(),
    )?;
    out(
        &name("place"),
        g.instance_place(cx, 296.0, 46.0, id)?.into(),
    );
    out(
        &name("place_meeting"),
        flag(g.place_meeting(cx, 296.0, 46.0, id)?),
    );
    set_b(g, cx.this, Builtin::SpriteIndex, -1.0)?;
    out(&name("read"), g.get_on(cx, &target, x)?.to_real());
    g.set_on(cx, &target, x, Value::Real(310.0))?;
    out(&name("write"), g.get_on(cx, &object, x)?.to_real());
    // `with (oGlobals) instance_deactivate_object(self)`: the persistent one alone.
    let q = g.active().find(|&h| g.instance(h).object == ObjectId(1));
    g.instance_deactivate_object(&common::ctx(g, q.unwrap()), -1);
    out(&name("alone_exists"), flag(g.instance_exists(cx, id)));
    g.set_on(cx, &target, x, Value::Real(5.0))?;
    g.instance_activate_object(cx, id);
    out(&name("back_exists"), flag(g.instance_exists(cx, id)));
    // GM8 writes nothing here, taking the destroyed one from its cache (`*_cached_write`).
    g.set_on(cx, &target, x, Value::Real(7.0))?;
    g.instance_deactivate_object(cx, id);
    out(&name("deact_exists"), flag(g.instance_exists(cx, id)));
    out(&name("deact_globals"), g.instance_number(cx, 1).into());
    g.instance_activate_object(cx, id);
    out(&name("act_exists"), flag(g.instance_exists(cx, id)));
    out(&name("act_globals"), g.instance_number(cx, 1).into());
    Ok(())
}

/// How the point and rectangle tests compare unrounded coordinates with a box (`collision-edges`, 851-3306), through
/// the port's real queries: an oBlack with sBlock's mask at (100, 100), an oMenuTop with sMattockHitR's at (200, 100)
/// and an oMenuLeft without a sprite or mask, each line the tests that meet a coordinate, as bits (the probe's
/// comments list them).
fn collision_edges(l: &mut Lines) {
    use gm::Builtin::{BboxBottom, BboxLeft, BboxRight, BboxTop, MaskIndex, SpriteIndex};
    let p = common::build(|b| {
        b.push_object(gm::ObjectInfo::new("oScreen"));
        b.push_object(gm::ObjectInfo::new("oBlack"));
        b.push_object(gm::ObjectInfo::new("oMenuTop"));
        b.push_object(gm::ObjectInfo::new("oMenuLeft"));
    });
    let assets = gm::Assets::new(spelunky::sprites(&["sMattockHitR", "sBlock"]), Vec::new());
    let room = common::room(320, 240, &[(common::FIRST_ID, gm::ObjectId(0), 0, 0)]);
    let mut g = common::start(p, assets, &[(0, room)]);
    let cx = common::ctx(&g, g.find(common::FIRST_ID).unwrap());
    let create = |g: &mut gm::World, x: f64, o: i32, mask: f64| {
        let id = g.instance_create(x, 100.0, o).unwrap();
        let h = g.find(id).unwrap();
        g.set(h, MaskIndex.var(), Value::Real(mask)).unwrap();
        h
    };
    let (black, top, left) = (1, 2, 3);
    let a = create(
        &mut g,
        100.0,
        black,
        f64::from(spelunky::sprite_id("sBlock")),
    );
    let c = create(
        &mut g,
        200.0,
        top,
        f64::from(spelunky::sprite_id("sMattockHitR")),
    );
    let e = create(&mut g, 300.0, left, -1.0);
    g.set(e, SpriteIndex.var(), Value::Real(-1.0)).unwrap();
    for (name, h) in [("a", a), ("c", c), ("e", e)] {
        let side =
            |s: gm::Builtin| real_to_string(g.get(h, s.var()).unwrap().to_real()).to_string();
        let sides = [BboxLeft, BboxTop, BboxRight, BboxBottom].map(side);
        l.text(&format!("edges_box_{name}"), &sides.join(" "));
    }
    let point = |x, y, obj, prec| g.collision_point(&cx, x, y, obj, prec, false).unwrap() != -4;
    let rect = |x1, y1, x2, y2, obj, prec| {
        g.collision_rectangle(&cx, x1, y1, x2, y2, obj, prec, false)
            .unwrap()
            != -4
    };
    let line = |x1, y1, x2, y2, obj, prec| {
        g.collision_line(&cx, x1, y1, x2, y2, obj, prec, false)
            .unwrap()
            != -4
    };
    let bits = |hits: &[bool]| {
        hits.iter()
            .enumerate()
            .map(|(i, &h)| f64::from(1 << i) * b(h))
            .sum::<f64>()
    };
    // The probe's literals, as GM8 reads them (`99.99` is 1 ulp above the nearest double).
    let values = [
        "99", "99.4", "99.5", "99.6", "99.99", "100", "100.5", "114.5", "115", "115.4", "115.5",
        "115.6", "115.99", "116", "116.5",
    ]
    .map(lit);
    for (i, &v) in values.iter().enumerate() {
        l.real(&format!("edges_v_{i}"), v);
        for prec in [false, true] {
            let mut hits = vec![
                point(v, 108.0, black, prec),
                point(108.0, v, black, prec),
                rect(v, 108.0, v, 108.0, black, prec),
                rect(90.0, 104.0, v, 112.0, black, prec),
                rect(v, 112.0, 90.0, 104.0, black, prec),
                rect(v, 104.0, 130.0, 112.0, black, prec),
                rect(130.0, 112.0, v, 104.0, black, prec),
                rect(104.0, 90.0, 112.0, v, black, prec),
                rect(104.0, v, 112.0, 130.0, black, prec),
                line(v, 108.0, v, 108.0, black, prec),
                line(108.0, v, 108.0, v, black, prec),
            ];
            if prec {
                hits.push(g.instance_position(v, 108.0, black).unwrap() != -4);
            }
            let name = if prec { "edges_p" } else { "edges_np" };
            l.real(&format!("{name}_{i}"), bits(&hits));
        }
    }
    let hole = [
        "196", "196.4", "196.5", "196.6", "197", "197.4", "197.5", "197.6", "198",
    ]
    .map(lit);
    for (i, &w) in hole.iter().enumerate() {
        let hits = [
            point(w, 94.0, top, true),
            point(197.0, w - 103.0, top, true),
            rect(w, 94.0, w, 94.0, top, true),
            rect(190.0, 94.0, w, 94.0, top, true),
            rect(w, 94.0, 205.0, 94.0, top, true),
            line(w, 94.0, w, 94.0, top, true),
        ];
        l.real(&format!("edges_hole_w_{i}"), w);
        l.real(&format!("edges_hole_{i}"), bits(&hits));
    }
    let none = [
        "100000.6", "100000.5", "100000", "99999.5", "99999.4", "99999.01", "99999",
    ]
    .map(|t| -lit(t));
    for (i, &u) in none.iter().enumerate() {
        l.real(&format!("edges_none_u_{i}"), u);
        for (k, prec) in [false, true].into_iter().enumerate() {
            let y = -100000.0;
            let hits = [
                point(u, y, left, prec),
                rect(u, y, u, y, left, prec),
                rect(u, y, -99990.0, y, left, prec),
                rect(-100010.0, y, u, y, left, prec),
                line(u, y, u, y, left, prec),
            ];
            l.real(&format!("edges_none_{k}_{i}"), bits(&hits));
        }
    }
    let p32 = pow2(32);
    let cuts = [
        [p32 + 50.0, 104.0, 105.0, 112.0],
        [p32 - 10.0, 104.0, 108.0, 112.0],
        [100.0, 104.0, p32 + 105.0, 112.0],
        [p32 + 100.0, 104.0, p32 + 110.0, 112.0],
    ];
    for (i, [x1, y1, x2, y2]) in cuts.into_iter().enumerate() {
        let hits = [false, true].map(|prec| rect(x1, y1, x2, y2, black, prec));
        l.real(&format!("edges_cut_{i}"), bits(&hits));
    }
}

/// `deactivation-order`'s `tag` variable.
const DEACTIVATION_TAG: Var = Var(gm::FIRST_USER_VAR);

thread_local! {
    /// The tags of the `core`s whose draw events ran (`deactivation-order`'s `global.probe_log`).
    static DEACTIVATION_DRAWS: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

/// The order of instances after (de)activation (`deactivation-order`, 851-3311), in a world of the probe's
/// objects: `core` (with a draw event logging its tag), `oMenuTop` and the probe's own (as `oScreen`), placed in
/// `rIntro`. Lines the port doesn't compute: `order_h_count`, `order_h7_activate` and `order_i_count`, which count
/// all of `rIntro`, whose own instances change between the steps.
fn deactivation_order(l: &mut Lines) {
    let (core, top, screen) = (ObjectId(0), ObjectId(1), ObjectId(2));
    let p = common::build(|b| {
        let o = b.push_object(ObjectInfo::new("core"));
        let tag = b.var("tag");
        assert_eq!(tag, DEACTIVATION_TAG);
        b.layout(o, &[tag]);
        b.event(o, Event::Draw, |g, cx| {
            let tag = g.get(cx.this, DEACTIVATION_TAG)?.to_real();
            DEACTIVATION_DRAWS.with(|d| d.borrow_mut().push_str(&format!("{tag}")));
            Ok(())
        });
        b.push_object(ObjectInfo::new("oMenuTop"));
        b.push_object(ObjectInfo::new("oScreen"));
    });
    let intro = common::room(960, 240, &common::numbered(&[(screen, 0, 0)]));
    let mut g = common::start(p, gm::Assets::default(), &[(0, intro)]);
    let me = common::ctx(&g, g.active().next().unwrap());
    let ctx_of = |g: &World, id: i32| common::ctx(g, g.find(id).unwrap());
    let count = |g: &World| g.bget(Builtin::InstanceCount.var(), 0).unwrap().to_real();
    let draws = || DEACTIVATION_DRAWS.with(|d| std::mem::take(&mut *d.borrow_mut()));
    let all = Value::Real(-3.0);
    let cores = Value::Real(f64::from(core.0));
    // The probe's SS_IsHandleValid: the cores' tags through `with (core)`, `instance_find` and `with (all)`.
    let orders = |g: &World, p: &str, l: &mut Lines| {
        let tag = |h: Inst| format!("{}", g.get(h, DEACTIVATION_TAG).unwrap().to_real());
        let mut s = String::new();
        let mut it = g.with_iter(&me, &cores).unwrap();
        while let Some(h) = it.next(g) {
            s += &tag(h);
        }
        l.text(&format!("order_{p}_with"), &s);
        let n = g.instance_number(&me, core.0 as i32);
        let s: String = (0..n)
            .map(|i| tag(g.find(g.instance_find(core.0 as i32, i)).unwrap()))
            .collect();
        l.text(&format!("order_{p}_find"), &s);
        let mut s = String::new();
        let mut it = g.with_iter(&me, &all).unwrap();
        while let Some(h) = it.next(g) {
            if g.instance(h).object == core {
                s += &tag(h);
            }
        }
        l.text(&format!("order_{p}_all"), &s);
    };
    let create = |g: &mut World, x: f64, object: ObjectId, tag: f64| {
        let id = g.instance_create(x, -5000.0, object.0 as i32).unwrap();
        let h = g.find(id).unwrap();
        if object == core {
            g.set(h, DEACTIVATION_TAG, Value::Real(tag)).unwrap();
            set_b(g, h, Builtin::Depth, 5.0).unwrap();
        }
        id
    };
    // Step 1.
    l.real(
        "order_a_cores",
        g.instance_number(&me, core.0 as i32).into(),
    );
    let c: Vec<i32> = (1..=4)
        .map(|i| create(&mut g, -5000.0 - 10.0 * f64::from(i), core, f64::from(i)))
        .collect();
    let x2 = g.instance(g.find(c[1]).unwrap()).x;
    let h4 = g.find(c[3]).unwrap();
    set_b(&mut g, h4, Builtin::X, x2).unwrap();
    orders(&g, "a", l);
    g.instance_deactivate_object(&me, c[1]);
    g.instance_activate_object(&me, c[1]);
    orders(&g, "b", l);
    for i in [3, 0, 2] {
        g.instance_deactivate_object(&me, c[i]);
    }
    g.instance_activate_object(&me, core.0 as i32);
    orders(&g, "c", l);
    let h3 = g.find(c[2]).unwrap();
    set_b(&mut g, h3, Builtin::Depth, 9.0).unwrap();
    draws();
    g.screen_redraw().unwrap();
    l.text("order_d_draw", &draws());
    orders(&g, "d", l);
    g.instance_activate_all();
    orders(&g, "e", l);
    let cx2 = ctx_of(&g, c[1]);
    g.instance_deactivate_region(&cx2, x2 - 1.0, -5001.0, 2.0, 2.0, true, true);
    l.real(
        "order_f_c4_exists",
        u8::from(g.instance_exists(&me, c[3])).into(),
    );
    g.instance_activate_all();
    orders(&g, "f", l);
    let m = count(&g);
    let c7 = create(&mut g, -6000.0, top, 7.0);
    l.real("order_g7_create", count(&g) - m);
    g.destroy(g.find(c7).unwrap()).unwrap();
    l.real("order_g7_destroy", count(&g) - m);
    g.instance_deactivate_object(&me, c7);
    l.real("order_g7_deactivate", count(&g) - m);
    let c8 = create(&mut g, -6000.0, core, 8.0);
    let m = count(&g);
    g.destroy(g.find(c8).unwrap()).unwrap();
    g.instance_deactivate_object(&me, core.0 as i32);
    l.real("order_g_deactivate", count(&g) - m);
    g.instance_activate_object(&me, core.0 as i32);
    l.real("order_g_activate", count(&g) - m);
    orders(&g, "g", l);
    g.step(&[]).unwrap();
    // Step 2.
    l.real(
        "order_h7_exists",
        u8::from(g.instance_exists(&me, c7)).into(),
    );
    g.instance_activate_object(&me, top.0 as i32);
    l.real(
        "order_h7_exists_after",
        u8::from(g.instance_exists(&me, c7)).into(),
    );
    l.real(
        "order_h7_number",
        g.instance_number(&me, top.0 as i32).into(),
    );
    orders(&g, "h", l);
    draws();
    g.step(&[]).unwrap();
    // Step 3.
    l.text("order_i_draw", &draws());
    orders(&g, "i", l);
}

/// The sprites the angle probes use, and a world with the probed instance (object 0, no sprite, as
/// `oScreen`) and the other one (object 1, as `oMenu`), placed by the room like GM8's.
fn angle_world() -> (gm::World, gm::Inst, gm::Inst) {
    const SPRITES: [&str; 4] = ["sTunnelLeft", "sMetalBlock", "sArrowRight", "sBlock"];
    let p = common::build(|b| {
        b.push_object(gm::ObjectInfo::new("oScreen"));
        b.push_object(gm::ObjectInfo::new("oMenu"));
    });
    let assets = gm::Assets::new(spelunky::sprites(&SPRITES), Vec::new());
    let mut g = common::start(
        p,
        assets,
        &[(
            0,
            common::room(
                320,
                240,
                &[
                    (common::FIRST_ID, gm::ObjectId(0), 0, 0),
                    (common::FIRST_ID + 1, gm::ObjectId(1), -1000, -1000),
                ],
            ),
        )],
    );
    let (me, other) = (
        g.find(common::FIRST_ID).unwrap(),
        g.find(common::FIRST_ID + 1).unwrap(),
    );
    let menu = spelunky::sprite_id("sBlock");
    g.set(
        other,
        gm::Builtin::MaskIndex.var(),
        Value::Real(f64::from(menu)),
    )
    .unwrap();
    (g, me, other)
}

/// A built-in of the probed instance, set from a generated expression or a sprite name.
fn set_builtin(g: &mut gm::World, h: gm::Inst, name: &str, rhs: &str) {
    use gm::Builtin as B;
    let b = match name {
        "x" => B::X,
        "y" => B::Y,
        "image_xscale" => B::ImageXscale,
        "image_yscale" => B::ImageYscale,
        "image_angle" => B::ImageAngle,
        "mask_index" => B::MaskIndex,
        _ => panic!("{name}"),
    };
    let v = if rhs.starts_with('s') {
        f64::from(spelunky::sprite_id(rhs))
    } else {
        eval(rhs)
    };
    g.set(h, b.var(), Value::Real(v)).unwrap();
}

/// The bounding boxes of a rotated, scaled instance (`angle-bbox`), through the port's real code
/// (`World::bbox`, GM8's routine since 851-3278): GM8's on Intel and the patched oracle's on every line.
/// (GM8 under Rosetta differs from Intel on 14 lines: Rosetta's `fsin` and `fcos` aren't the hardware's.)
#[test]
fn rotated_bounding_boxes_convert_angles_as_gm8() {
    let probe = angle_probe!("angle-bbox");
    let (intel, oracle) = (named(probe.intel), named(probe.oracle.unwrap()));
    let (mut g, me, _) = angle_world();
    let (mut lines, mut wrong) = (0, Vec::new());
    for s in statements(probe.gml) {
        match s {
            Stmt::Set(name, rhs) => set_builtin(&mut g, me, name, rhs),
            Stmt::Out(name, _) if name.starts_with("spr_") => {
                // GM8's origin and box for each sprite are the port's.
                let s = spelunky::sprites(&[&name[4..]])
                    .into_iter()
                    .flatten()
                    .next()
                    .unwrap();
                let b = s.masks.first().unwrap().bbox;
                let want = format!(
                    "{} {} {} {} {} {}",
                    s.origin_x, s.origin_y, b.left, b.top, b.right, b.bottom
                );
                assert_eq!(intel[name], want, "{name}");
            }
            Stmt::Out(name, _) => {
                let b = g.bbox(me);
                let port = format!("{} {} {} {}", b.left, b.top, b.right, b.bottom);
                lines += 1;
                for (who, want) in [("GM8 on Intel", intel[name]), ("the oracle", oracle[name])] {
                    if port != want {
                        wrong.push(format!("{name}: port {port}, {who} {want}"));
                    }
                }
            }
            Stmt::Other(s) => panic!("{s}"),
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    assert_eq!(lines, 1422);
}

/// `collision_point` and `place_meeting` with a rotated, scaled mask (`angle-mask`): the port answers GM8
/// on Intel, and the oracle, on every line, its box included.
#[test]
fn rotated_masks_convert_angles_as_gm8() {
    let probe = angle_probe!("angle-mask");
    let (intel, oracle) = (named(probe.intel), named(probe.oracle.unwrap()));
    let (mut g, me, other) = angle_world();
    let cx = common::ctx(&g, me);
    let mut lines = 0;
    let mut wrong = Vec::new();
    for s in statements(probe.gml) {
        match s {
            Stmt::Set("o" | "o.mask_index", _) | Stmt::Other(_) => {}
            Stmt::Set(name @ ("o.x" | "o.y"), rhs) => {
                let b = if name == "o.x" {
                    gm::Builtin::X
                } else {
                    gm::Builtin::Y
                };
                g.set(other, b.var(), Value::Real(eval(rhs))).unwrap();
            }
            Stmt::Set(name, rhs) => set_builtin(&mut g, me, name, rhs),
            Stmt::Out(name, call) => {
                let hit = if let Some(args) = call.split_once("collision_point(").map(|(_, a)| a) {
                    let mut a = args.split(", ");
                    let (x, y) = (eval(a.next().unwrap()), eval(a.next().unwrap()));
                    g.collision_point(&cx, x, y, 0, true, false).unwrap() != -4
                } else {
                    assert!(call.contains("place_meeting(x, y, oMenu)"));
                    let (x, y) = (g.instance(me).x, g.instance(me).y);
                    g.place_meeting(&cx, x, y, 1).unwrap()
                };
                let port = if hit { "1" } else { "0" };
                lines += 1;
                for (runner, want) in [("GM8", intel[name]), ("oracle", oracle[name])] {
                    if port != want {
                        wrong.push(format!("{name}: port {port}, {runner} {want}"));
                    }
                }
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    assert_eq!(lines, 1986);
}

/// Gravity (`angle-motion-add`): GM8's runner adds it through the routine behind `motion_add`, which the
/// probe measured from the speeds it printed first. A frame of the port's gravity gives the same bits.
#[test]
fn gravity_adds_motion_as_gm8() {
    let probe = angle_probe!("angle-motion-add");
    let intel = named(probe.intel);
    assert_eq!(probe.intel, probe.oracle.unwrap());
    let bits = |name: String| {
        f64::from_bits(u64::from_str_radix(intel[&*name].split(' ').next().unwrap(), 16).unwrap())
    };
    let (mut g, me, _) = angle_world();
    for k in 0..720 {
        // The probe's motion_add(k / 2, 0.5 + (k mod 6) / 4).
        let (dir, amount) = (f64::from(k) / 2.0, 0.5 + f64::from(k % 6) / 4.0);
        let (h0, v0) = (bits(format!("ma0h_{k}")), bits(format!("ma0v_{k}")));
        let i = g.instance_mut(me);
        (i.x, i.y, i.hspeed, i.vspeed, i.gravity, i.gravity_direction) =
            (0.0, 0.0, h0, v0, amount, dir);
        common::step(&mut g);
        let i = g.instance(me);
        let (h, v) = (bits(format!("mah_{k}")), bits(format!("mav_{k}")));
        assert_eq!(
            (i.hspeed.to_bits(), i.vspeed.to_bits()),
            (h.to_bits(), v.to_bits()),
            "line {k}: ({:e}, {:e}) for GM8's ({h:e}, {v:e})",
            i.hspeed,
            i.vspeed
        );
        assert_eq!((i.x, i.y), (h, v), "the instance moves by its new speeds");
    }
}

/// The mouse through a rotated view (`angle-view`, only on GM8): the port's `View::to_room` gives GM8's answer
/// on every line, below the 0.01 degrees where GM8 doesn't rotate too (851-3279).
#[test]
fn rotated_views_convert_angles_as_gm8() {
    let probe = angle_probe!("angle-view", None);
    let intel = named(probe.intel);
    let mut v = gm::View::default();
    let (mut mouse, mut lines, mut wrong) = ((0, 0), 0, Vec::new());
    for s in statements(probe.gml) {
        match s {
            Stmt::Set(name, rhs) => match name {
                "view_xview[0]" => v.xview = eval(rhs) as i32,
                "view_yview[0]" => v.yview = eval(rhs) as i32,
                "view_wview[0]" => v.wview = eval(rhs) as i32,
                "view_hview[0]" => v.hview = eval(rhs) as i32,
                "view_xport[0]" => v.xport = eval(rhs) as i32,
                "view_yport[0]" => v.yport = eval(rhs) as i32,
                "view_wport[0]" => v.wport = eval(rhs) as i32,
                "view_hport[0]" => v.hport = eval(rhs) as i32,
                "view_angle[0]" => v.angle = eval(rhs),
                // The window is 320x240, as the port, so window and port coordinates are the same.
                "view_enabled" | "view_visible[0]" => {}
                _ => panic!("{name} = {rhs}"),
            },
            Stmt::Other(s) => {
                if let Some(args) = s.strip_prefix("window_mouse_set(") {
                    let (x, y) = args.trim_end_matches(')').split_once(", ").unwrap();
                    mouse = (x.parse().unwrap(), y.parse().unwrap());
                }
            }
            Stmt::Out(name, _) => {
                lines += 1;
                let (x, y) = v.to_room(mouse.0, mouse.1);
                if format!("{x} {y}") != intel[name] {
                    wrong.push(format!("{name}: port {x} {y}, GM8 {}", intel[name]));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    assert_eq!(lines, 778);
}
