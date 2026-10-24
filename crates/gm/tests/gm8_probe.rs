//! The port's answers to the GM8 probes (`tools/gm8-probe/`), checked against the real GM8 runner's (under
//! Wine and Rosetta, `results/gm8-wine/`, and on a real Intel CPU, `results/gm8-intel/`) and the patched
//! oracle's (`results/opengmk/`). The port and the oracle follow GM8 on Intel for every rule the probes
//! pinned (oracle patch 0014), so the port must agree with OpenGMK on every line, and with GM8 on every line
//! but those [`gm8_differs`] lists. Rosetta doesn't trap the x87's exceptions, so where GM8 on Intel (and
//! the port) stops with an error, or `string()` gives `"ERROR"`, the Rosetta line differs too.
//!
//! Each probe line is `name = <bits in hex> <string(value)>` for a real, or `name = "text"` for a string.
//! Only the lines the port has the functions for are recomputed here (the game never calls `tan`, `exp`,
//! `power`, `ln`, `max` and the like, so `gm` doesn't have them). On top of that, `string()` is checked on
//! every real any probe printed, from its bits.

use std::collections::BTreeMap;

use gm::math::{self, is_truthy};
use gm::string::{real_to_string, string_format, string_to_real};
use gm::{Rng, Value, ValueError};

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
    (
        "strings",
        include_str!("../../../tools/gm8-probe/results/opengmk/strings.txt"),
    ),
    (
        "ops",
        include_str!("../../../tools/gm8-probe/results/opengmk/ops.txt"),
    ),
    (
        "div-negzero",
        include_str!("../../../tools/gm8-probe/results/opengmk/div-negzero.txt"),
    ),
    (
        "div-tiny",
        include_str!("../../../tools/gm8-probe/results/opengmk/div-tiny.txt"),
    ),
    (
        "divassign-local",
        include_str!("../../../tools/gm8-probe/results/opengmk/divassign-local.txt"),
    ),
    (
        "divassign-field",
        include_str!("../../../tools/gm8-probe/results/opengmk/divassign-field.txt"),
    ),
    (
        "divassign-builtin",
        include_str!("../../../tools/gm8-probe/results/opengmk/divassign-builtin.txt"),
    ),
    (
        "intdiv-frac",
        include_str!("../../../tools/gm8-probe/results/opengmk/intdiv-frac.txt"),
    ),
    (
        "mod-tiny",
        include_str!("../../../tools/gm8-probe/results/opengmk/mod-tiny.txt"),
    ),
    (
        "sqrt-neg",
        include_str!("../../../tools/gm8-probe/results/opengmk/sqrt-neg.txt"),
    ),
    (
        "exp-overflow",
        include_str!("../../../tools/gm8-probe/results/opengmk/exp-overflow.txt"),
    ),
    (
        "string-format",
        include_str!("../../../tools/gm8-probe/results/opengmk/string-format.txt"),
    ),
    (
        "string-format-neg",
        include_str!("../../../tools/gm8-probe/results/opengmk/string-format-neg.txt"),
    ),
    (
        "divassign-str-arg",
        include_str!("../../../tools/gm8-probe/results/opengmk/divassign-str-arg.txt"),
    ),
    (
        "divassign-str-arg2",
        include_str!("../../../tools/gm8-probe/results/opengmk/divassign-str-arg2.txt"),
    ),
    (
        "divassign-str-field",
        include_str!("../../../tools/gm8-probe/results/opengmk/divassign-str-field.txt"),
    ),
    (
        "divassign-skip",
        include_str!("../../../tools/gm8-probe/results/opengmk/divassign-skip.txt"),
    ),
    (
        "in-range",
        include_str!("../../../tools/gm8-probe/results/opengmk/in-range.txt"),
    ),
    (
        "round-2p63",
        include_str!("../../../tools/gm8-probe/results/opengmk/round-2p63.txt"),
    ),
    (
        "or-2p63",
        include_str!("../../../tools/gm8-probe/results/opengmk/or-2p63.txt"),
    ),
    (
        "div-2p63",
        include_str!("../../../tools/gm8-probe/results/opengmk/div-2p63.txt"),
    ),
    (
        "string-format-nonfinite",
        include_str!("../../../tools/gm8-probe/results/opengmk/string-format-nonfinite.txt"),
    ),
];

const GM8: &[(&str, &str)] = &[
    (
        "numbers",
        include_str!("../../../tools/gm8-probe/results/gm8-wine/numbers.txt"),
    ),
    (
        "precision",
        include_str!("../../../tools/gm8-probe/results/gm8-wine/precision.txt"),
    ),
    (
        "rng",
        include_str!("../../../tools/gm8-probe/results/gm8-wine/rng.txt"),
    ),
    (
        "math",
        include_str!("../../../tools/gm8-probe/results/gm8-wine/math.txt"),
    ),
    (
        "div0",
        include_str!("../../../tools/gm8-probe/results/gm8-wine/div0.txt"),
    ),
    (
        "intdiv0",
        include_str!("../../../tools/gm8-probe/results/gm8-wine/intdiv0.txt"),
    ),
    (
        "mod0",
        include_str!("../../../tools/gm8-probe/results/gm8-wine/mod0.txt"),
    ),
    (
        "nonfinite",
        include_str!("../../../tools/gm8-probe/results/gm8-wine/nonfinite.txt"),
    ),
    (
        "strings",
        include_str!("../../../tools/gm8-probe/results/gm8-wine/strings.txt"),
    ),
    (
        "ops",
        include_str!("../../../tools/gm8-probe/results/gm8-wine/ops.txt"),
    ),
    (
        "div-negzero",
        include_str!("../../../tools/gm8-probe/results/gm8-wine/div-negzero.txt"),
    ),
    (
        "div-tiny",
        include_str!("../../../tools/gm8-probe/results/gm8-wine/div-tiny.txt"),
    ),
    (
        "divassign-local",
        include_str!("../../../tools/gm8-probe/results/gm8-wine/divassign-local.txt"),
    ),
    (
        "divassign-field",
        include_str!("../../../tools/gm8-probe/results/gm8-wine/divassign-field.txt"),
    ),
    (
        "divassign-builtin",
        include_str!("../../../tools/gm8-probe/results/gm8-wine/divassign-builtin.txt"),
    ),
    (
        "intdiv-frac",
        include_str!("../../../tools/gm8-probe/results/gm8-wine/intdiv-frac.txt"),
    ),
    (
        "mod-tiny",
        include_str!("../../../tools/gm8-probe/results/gm8-wine/mod-tiny.txt"),
    ),
    (
        "sqrt-neg",
        include_str!("../../../tools/gm8-probe/results/gm8-wine/sqrt-neg.txt"),
    ),
    (
        "exp-overflow",
        include_str!("../../../tools/gm8-probe/results/gm8-wine/exp-overflow.txt"),
    ),
    (
        "string-format",
        include_str!("../../../tools/gm8-probe/results/gm8-wine/string-format.txt"),
    ),
    (
        "string-format-neg",
        include_str!("../../../tools/gm8-probe/results/gm8-wine/string-format-neg.txt"),
    ),
    (
        "divassign-str-arg",
        include_str!("../../../tools/gm8-probe/results/gm8-wine/divassign-str-arg.txt"),
    ),
    (
        "divassign-str-arg2",
        include_str!("../../../tools/gm8-probe/results/gm8-wine/divassign-str-arg2.txt"),
    ),
    (
        "divassign-str-field",
        include_str!("../../../tools/gm8-probe/results/gm8-wine/divassign-str-field.txt"),
    ),
    (
        "divassign-skip",
        include_str!("../../../tools/gm8-probe/results/gm8-wine/divassign-skip.txt"),
    ),
    (
        "in-range",
        include_str!("../../../tools/gm8-probe/results/gm8-wine/in-range.txt"),
    ),
    (
        "round-2p63",
        include_str!("../../../tools/gm8-probe/results/gm8-wine/round-2p63.txt"),
    ),
    (
        "or-2p63",
        include_str!("../../../tools/gm8-probe/results/gm8-wine/or-2p63.txt"),
    ),
    (
        "div-2p63",
        include_str!("../../../tools/gm8-probe/results/gm8-wine/div-2p63.txt"),
    ),
    (
        "string-format-nonfinite",
        include_str!("../../../tools/gm8-probe/results/gm8-wine/string-format-nonfinite.txt"),
    ),
];

/// The probes GM8 ran on a real Intel CPU (851-3247) that the port has lines for.
const GM8_INTEL: &[(&str, &str)] = &[
    (
        "numbers",
        include_str!("../../../tools/gm8-probe/results/gm8-intel/numbers.txt"),
    ),
    (
        "precision",
        include_str!("../../../tools/gm8-probe/results/gm8-intel/precision.txt"),
    ),
    (
        "rng",
        include_str!("../../../tools/gm8-probe/results/gm8-intel/rng.txt"),
    ),
    (
        "math",
        include_str!("../../../tools/gm8-probe/results/gm8-intel/math.txt"),
    ),
    (
        "div0",
        include_str!("../../../tools/gm8-probe/results/gm8-intel/div0.txt"),
    ),
    (
        "intdiv0",
        include_str!("../../../tools/gm8-probe/results/gm8-intel/intdiv0.txt"),
    ),
    (
        "mod0",
        include_str!("../../../tools/gm8-probe/results/gm8-intel/mod0.txt"),
    ),
    (
        "sqrt-neg",
        include_str!("../../../tools/gm8-probe/results/gm8-intel/sqrt-neg.txt"),
    ),
];

/// Lines where the port (and the oracle) knowingly differ from GM8, with why.
fn gm8_differs(name: &str) -> Option<&'static str> {
    // x87 transcendentals under Rosetta (851-3247).
    const X87: &[&str] = &[
        "cos_7", "cos_9", "cos_30", "cos_53", "cos_70", "cos_107", "pdir_4", "sin_1e22",
    ];
    X87.contains(&name).then_some("x87 trigonometry")
}

/// Lines where the port (and the oracle) knowingly differ from GM8 on a real Intel CPU, with why.
fn gm8_on_intel_differs(name: &str) -> Option<&'static str> {
    // The x87's transcendentals (851-3247, 851-3257): fsin and fcos reduce by Intel's 66-bit pi (at multiples
    // of pi and far out), `cos` and `point_direction` are 1 ulp off once each, and `sin(1e22)` is out of
    // fsin's range, so it returns 1e22.
    const X87: &[&str] = &[
        "cos_30", "pdir_4", "sin_pi", "sin_2pi", "cos_pi/2", "sin_1e6", "cos_1e6", "sin_1e22",
    ];
    X87.contains(&name).then_some("x87 trigonometry")
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

    fn error(&mut self, name: &str, e: ValueError) {
        self.0.insert(name.to_string(), format!("{ERROR}{e}"));
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
    l.real("div_1_2^-1074", math::div(1.0, pow2(-1074)).unwrap());
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
    let keep = |old: f64| match gm::lenient(Value::div, Value::Real(old), &Value::Real(0.0)) {
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

/// `/=` on a string, on an argument and on an instance variable: both keep it, as every `op=` does on an error.
fn divassign_strings(l: &mut Lines) {
    let abc = Value::from("abc");
    let text = |v: Value| match v {
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

    let mut wrong = Vec::new();
    let mut strings = 0;
    for (table, rosetta) in [(GM8, true), (OPENGMK, false), (GM8_INTEL, false)] {
        for (_, text) in table {
            strings += strings_of_every_real(text, rosetta, &mut wrong);
        }
    }

    // GM8 under Rosetta ran every probe to the end but the `errors/` ones, so it has a line for everything
    // the port computes.
    let rosetta = results(GM8);
    for (name, value) in &port.0 {
        let Some((probe, want)) = rosetta.lines.get(name) else {
            panic!("no GM8 line for {name}");
        };
        let why = gm8_differs(name).or(traps(value).then_some("Rosetta doesn't trap the x87"));
        compare(&mut wrong, "GM8", (probe, name), value, want, why);
    }

    // The oracle stops at the first error, as GM8 on Intel does, so a line may be missing where the port
    // errors too, or after an error in the same probe (`in-range` has the lines that follow one).
    let oracle = results(OPENGMK);
    let mut checked = 0;
    for (name, value) in &port.0 {
        let probe = rosetta.lines[name].0;
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

    // GM8 on Intel ran fewer probes, and stopped `numbers` at `floor(1e20)`.
    let intel = results(GM8_INTEL);
    let mut checked = 0;
    for (name, (probe, want)) in &intel.lines {
        // The functions the port doesn't have.
        let Some(value) = port.0.get(name) else {
            continue;
        };
        checked += 1;
        compare(
            &mut wrong,
            "GM8 on Intel",
            (probe, name),
            value,
            want,
            gm8_on_intel_differs(name),
        );
    }
    assert!(checked > 900, "only {checked} GM8 on Intel lines checked");
    for (probe, name, error) in [
        ("numbers", "floor_1e20", ValueError::IntOverflow),
        ("sqrt-neg", "sqrt_-1", ValueError::Domain("sqrt")),
    ] {
        assert!(intel.stopped.contains(&probe), "{probe}");
        assert_eq!(port.0[name], format!("{ERROR}{error}"));
    }
    assert_eq!(
        ValueError::IntOverflow.to_string(),
        "Unexpected error occured when running the game.",
        "GM8's dialog"
    );

    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    assert!(strings > 3000, "only {strings} string() lines checked");
}

// ---- GM8 on real x87 hardware (851-3247) ----
//
// The `probes/x87/` probes, run by the real GM8 runner on an Intel CPU (`results/gm8-intel/x87/`), against
// what the port computes for the same lines. This pins how often GM8's answers differ from the port's, per
// function; `tools/gm8-probe/README.md` ("Real x87") says why, and `x87/ulps.py` checks the model of the x87
// that explains every differing line but those of `exp` and `power` (which the port doesn't have).

const INTEL_X87: &[(&str, &str)] = &[
    (
        "deg",
        include_str!("../../../tools/gm8-probe/results/gm8-intel/x87/deg.txt"),
    ),
    (
        "rad",
        include_str!("../../../tools/gm8-probe/results/gm8-intel/x87/rad.txt"),
    ),
    (
        "dir",
        include_str!("../../../tools/gm8-probe/results/gm8-intel/x87/dir.txt"),
    ),
    (
        "funcs",
        include_str!("../../../tools/gm8-probe/results/gm8-intel/x87/funcs.txt"),
    ),
];

/// (function, lines, lines where GM8 on Intel differs from the port). Values compare with `==`, so a -0
/// where the port has +0 doesn't count.
const INTEL_VS_PORT: &[(&str, usize, usize)] = &[
    // GM8 multiplies by Pi / 180 as an 80-bit constant.
    ("degtorad", 720, 76),
    // fsin and fcos are correctly rounded here, but for sin(180) and cos(90), cos(270) degrees, where
    // Intel's 66-bit pi shows.
    ("sin (degrees)", 720, 1),
    ("cos (degrees)", 720, 2),
    // The runner's own direction * pi / 180, the 80-bit cos or sin of that, then * speed.
    ("motion hspeed", 720, 251),
    ("motion vspeed", 720, 252),
    // atan2 kept at 80 bits, then * 180 and / pi, each rounded to 53 bits.
    ("motion direction", 625, 112),
    ("motion speed", 625, 0),
    ("sin (k/100)", 1000, 0),
    ("cos (k/100)", 1000, 0),
    ("sin (k/64)", 640, 0),
    ("cos (k/64)", 640, 1),
    // Up to 30000 radians, where the 66-bit pi's error grows with the argument.
    ("sin (k*k/3)", 300, 38),
    ("cos (k*k/3)", 300, 58),
    ("point_direction (whole)", 2401, 539),
    ("arctan2 (whole)", 2401, 0),
    ("point_direction (fractional)", 800, 184),
    ("point_distance (fractional)", 800, 0),
    ("arctan", 1000, 1),
    ("sqrt", 1000, 0),
    ("point_distance", 1000, 0),
];

/// What the port computes for one `probes/x87/` line, as (function, value), or None for the functions the
/// port doesn't have (`tan`, `ln`, `exp`, `power`). `gm8` holds the probe's lines: the degree probe's `sin`
/// and `cos` take GM8's own radians.
fn port_x87(probe: &str, name: &str, gm8: &BTreeMap<String, f64>) -> Option<(&'static str, f64)> {
    let (f, arg) = name.split_once('_')?;
    let int = |s: &str| f64::from(s.parse::<i32>().unwrap());
    // gm::Instance's motion snaps to a whole number within 0.0001 (`update_hvspeed`).
    let snap = |v: f64| {
        let r = math::round_half_even(v);
        if (r - v).abs() < 0.0001 { r } else { v }
    };
    Some(match (probe, f) {
        ("deg", "rad") => ("degtorad", math::degtorad(int(arg) / 2.0)),
        ("deg", "sin") => ("sin (degrees)", math::sin(gm8[&format!("rad_{arg}")])),
        ("deg", "cos") => ("cos (degrees)", math::cos(gm8[&format!("rad_{arg}")])),
        ("deg", "hsp" | "vsp") => {
            let k = int(arg);
            let (speed, rad) = (1.0 + (k % 8.0) / 2.0, (k / 2.0).to_radians());
            if f == "hsp" {
                ("motion hspeed", snap(math::cos(rad) * speed))
            } else {
                ("motion vspeed", snap(-math::sin(rad) * speed))
            }
        }
        ("deg", "dir" | "spd") => {
            let (a, b) = arg.split_once('_')?;
            let (h, v) = (int(a) / 2.0 - 6.0, int(b) / 2.0 - 6.0);
            if f == "spd" {
                ("motion speed", (h * h + v * v).sqrt())
            } else {
                (
                    "motion direction",
                    snap(math::point_direction(0.0, 0.0, h, v)),
                )
            }
        }
        ("rad", "sin" | "cos") => {
            let k = int(&arg[1..]);
            let sin = f == "sin";
            let trig = |x| if sin { math::sin(x) } else { math::cos(x) };
            match (&arg[..1], sin) {
                ("c", true) => ("sin (k/100)", trig(k / 100.0)),
                ("c", false) => ("cos (k/100)", trig(k / 100.0)),
                ("s", true) => ("sin (k/64)", trig(k / 64.0)),
                ("s", false) => ("cos (k/64)", trig(k / 64.0)),
                (_, true) => ("sin (k*k/3)", trig(k * k / 3.0)),
                (_, false) => ("cos (k*k/3)", trig(k * k / 3.0)),
            }
        }
        ("dir", _) => {
            let (i, j) = arg.split_once('_')?;
            let (i, j) = (int(i), int(j));
            let (dx, dy) = (i * 3.0 / 8.0 - 7.0, j * 5.0 / 8.0 - 5.0);
            match f {
                "pdir" => (
                    "point_direction (whole)",
                    math::point_direction(0.0, 0.0, i, j),
                ),
                "atan2" => ("arctan2 (whole)", math::atan2(-j, i)),
                "fpdir" => (
                    "point_direction (fractional)",
                    math::point_direction(0.0, 0.0, dx, dy),
                ),
                _ => (
                    "point_distance (fractional)",
                    math::point_distance(0.0, 0.0, dx, dy),
                ),
            }
        }
        ("funcs", "arctan" | "sqrt" | "pdist") => {
            let v = int(arg) / 100.0;
            match f {
                "arctan" => ("arctan", math::arctan(v)),
                "sqrt" => ("sqrt", math::sqrt(v).unwrap()),
                _ => (
                    "point_distance",
                    math::point_distance(0.0, 0.0, v, 13.0 / 10.0),
                ),
            }
        }
        _ => return None,
    })
}

#[test]
fn gm8_on_intel_differs_from_the_port_this_often() {
    let mut counts: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for (probe, text) in INTEL_X87 {
        let gm8: BTreeMap<String, f64> = text
            .lines()
            .filter_map(|l| {
                let (name, value) = l.split_once(" = ")?;
                let bits = u64::from_str_radix(value.split(' ').next()?, 16).ok()?;
                Some((name.to_string(), f64::from_bits(bits)))
            })
            .collect();
        for (name, &got) in &gm8 {
            if let Some((f, port)) = port_x87(probe, name, &gm8) {
                let c = counts.entry(f).or_default();
                c.0 += 1;
                c.1 += usize::from(got != port);
            }
        }
    }
    let mut want: Vec<_> = INTEL_VS_PORT.iter().map(|&(f, n, d)| (f, (n, d))).collect();
    want.sort();
    assert_eq!(counts.into_iter().collect::<Vec<_>>(), want);
}
