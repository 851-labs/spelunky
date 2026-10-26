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
use gm::{Builtin, Rng, Value, ValueError};

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
            "lengthdir-2p63",
            "mulstr-2p27", "mulstr-2p28", "mulstr-2p28+1", "mulstr-19x2p24", "mulstr-39x2p23", "mulstr-5x2p26",
            "mulstr-3x2p27", "mulstr-2p29", "mulstr-2p30",
            "mulstr-2p22-64", "mulstr-2p23-64",
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
    ];
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
    l.real("lengthdir_x_10_33", math::lengthdir_x(10.0, 33.0));
    l.real("lengthdir_y_10_33", math::lengthdir_y(10.0, 33.0));
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
    }

    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    assert!(strings > 3000, "only {strings} string() lines checked");
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
                    g.collision_point(&cx, x, y, 0, true, false) != -4
                } else {
                    assert!(call.contains("place_meeting(x, y, oMenu)"));
                    let i = g.instance(me);
                    g.place_meeting(&cx, i.x, i.y, 1)
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
