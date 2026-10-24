//! The port's answers to the GM8 probes (`tools/gm8-probe/`), checked against OpenGMK's: the oracle is
//! what the port matches, so these must agree line for line. Where real GM8 (under Wine) answers
//! differently, `docs/quirks.md` §7 says so; the committed `results/gm8-wine/` has its lines.
//!
//! Each probe line is `name = <bits in hex> <string(value)>` for a real, or `name = "text"` for a string.
//! Only the lines the port has the functions for are recomputed here (the game never calls `tan`, `exp`,
//! `power`, `ln` and the like, so `gm` doesn't have them).

use std::collections::BTreeMap;

use gm::Rng;
use gm::math::{self, is_truthy};
use gm::string::{real_to_string, string_to_real};

const OPENGMK: &[(&str, &str)] = &[
    (
        "numbers",
        include_str!("../../../tools/gm8-probe/results/opengmk/numbers.txt"),
    ),
    (
        "precision",
        include_str!("../../../tools/gm8-probe/results/opengmk/precision.txt"),
    ),
    (
        "rng",
        include_str!("../../../tools/gm8-probe/results/opengmk/rng.txt"),
    ),
    (
        "math",
        include_str!("../../../tools/gm8-probe/results/opengmk/math.txt"),
    ),
    (
        "div0",
        include_str!("../../../tools/gm8-probe/results/opengmk/div0.txt"),
    ),
    (
        "intdiv0",
        include_str!("../../../tools/gm8-probe/results/opengmk/intdiv0.txt"),
    ),
    (
        "mod0",
        include_str!("../../../tools/gm8-probe/results/opengmk/mod0.txt"),
    ),
    (
        "nonfinite",
        include_str!("../../../tools/gm8-probe/results/opengmk/nonfinite.txt"),
    ),
];

/// A GM8 decimal literal: all its digits as one whole number times 0.1 per decimal (`gml` lexer).
fn lit(text: &str) -> f64 {
    let (int, frac) = text.split_once('.').unwrap_or((text, ""));
    let whole = int
        .bytes()
        .chain(frac.bytes())
        .fold(0.0f64, |n, d| n * 10.0 + f64::from(d - b'0'));
    whole * frac.bytes().fold(1.0f64, |s, _| s / 10.0)
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
        l.real(&format!("round_{name}"), math::round(x));
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
        l.real(&format!("floor_{name}"), math::floor(x));
    }
    l.real("ceil_2p40+0.5", math::ceil(big + 0.5));
    l.real("ceil_-0.5", math::ceil(-0.5));
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

    l.real("and_5_3", math::bit_and(5.0, 3.0));
    l.real("and_-1_255", math::bit_and(-1.0, 255.0));
    l.real("or_-5_2", math::bit_or(-5.0, 2.0));
    l.real("xor_-5_1", math::bit_xor(-5.0, 1.0));
    l.real("and_3.7_1", math::bit_and(lit("3.7"), 1.0));
    l.real("and_2.5_3", math::bit_and(2.5, 3.0));
    l.real("or_-2.5_0", math::bit_or(-2.5, 0.0));
    l.real("or_2p31_0", math::bit_or(pow2(31), 0.0));
    l.real("or_2p32+5_0", math::bit_or(pow2(32) + 5.0, 0.0));
    l.real("or_2p40_1", math::bit_or(big, 1.0));
    l.real("or_1e20_0", math::bit_or(big * big * 100.0, 0.0));
    l.real("shl_1_31", math::shl(1.0, 31.0));
    l.real("shl_1_32", math::shl(1.0, 32.0));
    l.real("shl_1_40", math::shl(1.0, 40.0));
    l.real("shr_-1_1", math::shr(-1.0, 1.0));
    l.real("shr_2p31_1", math::shr(pow2(31), 1.0));
    l.real("shr_-8_1", math::shr(-8.0, 1.0));

    l.real("div_7_2", math::int_div(7.0, 2.0));
    l.real("div_-7_2", math::int_div(-7.0, 2.0));
    l.real("div_7_-2", math::int_div(7.0, -2.0));
    l.real("div_7.5_2", math::int_div(7.5, 2.0));
    l.real("div_2p40+1_3", math::int_div(big + 1.0, 3.0));
    l.real("mod_-7_3", math::modulo(-7.0, 3.0));
    l.real("mod_7_-3", math::modulo(7.0, -3.0));
    l.real("mod_5.5_2", math::modulo(5.5, 2.0));
    l.real("mod_-5.5_2", math::modulo(-5.5, 2.0));
    l.real("mod_1_0.1", math::modulo(1.0, lit("0.1")));
    l.real("mod_2p40+3_7", math::modulo(big + 3.0, 7.0));
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
    l.real("sqrt2", math::sqrt(2.0).unwrap());
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
        l.real(&format!("sqrt_{n}"), math::sqrt(v).unwrap());
        l.real(
            &format!("pdir_{n}"),
            math::point_direction(0.0, 0.0, f64::from(n), 7.0),
        );
        l.real(
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
    l.real("sqr_1.1", math::sqr(lit("1.1")));
    l.real("degtorad_33", math::degtorad(33.0));
    l.real("radtodeg_1", math::radtodeg(1.0));
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

fn zero_divisors(l: &mut Lines) {
    let z = 0.0;
    let (pinf, ninf, nan) = (f64::INFINITY, f64::NEG_INFINITY, f64::NAN);
    l.real("div_1_0", pinf);
    l.real("div_-1_0", ninf);
    l.real("div_0_0", nan);
    l.string_of("str_inf", pinf);
    l.string_of("str_-inf", ninf);
    l.string_of("str_nan", nan);
    l.real("inf_minus_inf", f64::NAN);
    l.real("nan_eq_nan", b(math::eq(nan, nan)));
    l.real("nan_ne_nan", b(math::ne(nan, nan)));
    l.real("nan_gt_0.5", b(math::gt(nan, 0.5)));
    l.real("inf_or_0", math::bit_or(pinf, 0.0));
    l.real("round_inf", math::round(pinf));
    l.real("nan_lt_0", b(math::lt(nan, 0.0)));
    l.real("nan_le_0", b(math::le(nan, 0.0)));
    l.real("nan_gt_0", b(math::gt(nan, 0.0)));
    l.real("nan_ge_0", b(math::ge(nan, 0.0)));
    l.real("nan_value", nan);
    l.real("inf_eq_inf", b(math::eq(pinf, pinf)));
    l.real("inf_gt_1", b(math::gt(pinf, 1.0)));
    l.real("ninf_lt_0", b(math::lt(ninf, 0.0)));
    l.real("floor_inf", math::floor(pinf));
    l.real("intdiv_7_0", math::int_div(7.0, z));
    l.real("mod_7_0", math::modulo(7.0, z));

    // nonfinite.gml: the same without a division by zero
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
    l.string_of("nf_round_inf", math::round(pinf));
    l.string_of("nf_floor_inf", math::floor(pinf));
    l.real("nf_or_inf_0", math::bit_or(pinf, 0.0));
}

#[test]
fn the_port_answers_the_probes_like_opengmk() {
    let mut port = Lines::default();
    numbers(&mut port);
    precision(&mut port);
    rng(&mut port);
    transcendentals(&mut port);
    zero_divisors(&mut port);

    let mut expected = BTreeMap::new();
    for (probe, text) in OPENGMK {
        // `#` lines record how a probe stopped.
        for line in text.lines().filter(|l| !l.starts_with('#')) {
            let (name, value) = line.split_once(" = ").expect("a probe line");
            // `sin_pi` is in two probes, with the same answer.
            expected.insert(name.to_string(), (probe, value.to_string()));
        }
    }
    let mut checked = 0;
    let mut wrong = Vec::new();
    for (name, value) in &port.0 {
        let Some((probe, want)) = expected.get(name) else {
            panic!("no OpenGMK line for {name}");
        };
        checked += 1;
        if value != want {
            wrong.push(format!("{probe}: {name}: port {value}, OpenGMK {want}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    assert!(checked > 900, "only {checked} lines checked");
}
