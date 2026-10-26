//! GM8 on a real Intel CPU, line for line (851-3247, 851-3257).
//!
//! `tools/gm8-probe/probes/x87/` ran on the real GM8 runner on an Intel Core i7 (`results/gm8-intel/x87/`):
//! about 23,000 values of the math built-ins and the motion variables. The port must compute every one of them
//! bit for bit, signed zeros included. Here rather than in `tests/gm8_probe.rs` so that `scripts/wasm-test`
//! runs it on wasm32 too. `exp`, `ln`, `tan` and `power` aren't GML built-ins in the port (the game never
//! calls them), so their lines check `x87::gm8` directly, `power` composed as Delphi's `Power` does it.

use std::collections::BTreeMap;

use super::x87::{F80, Precision, gm8};
use super::{arctan, atan2, cos, degtorad, point_direction, point_distance, radtodeg, sin, sqrt};
use super::{lengthdir_x, lengthdir_y, speed};

const INTEL: [(&str, &str); 4] = [
    (
        "deg",
        include_str!("../../../../tools/gm8-probe/results/gm8-intel/x87/deg.txt"),
    ),
    (
        "rad",
        include_str!("../../../../tools/gm8-probe/results/gm8-intel/x87/rad.txt"),
    ),
    (
        "dir",
        include_str!("../../../../tools/gm8-probe/results/gm8-intel/x87/dir.txt"),
    ),
    (
        "funcs",
        include_str!("../../../../tools/gm8-probe/results/gm8-intel/x87/funcs.txt"),
    ),
];

/// `probes/x87/extra.gml` under Rosetta, whose arithmetic is the hardware's (its `fsin` and `fcos` aren't,
/// so only the lines without them are compared with the port).
const ROSETTA_EXTRA: &str =
    include_str!("../../../../tools/gm8-probe/results/gm8-wine/x87/extra.txt");

/// A probe's real lines (`name = <bits in hex> <string>`) by name.
fn lines(text: &str) -> BTreeMap<&str, f64> {
    text.lines()
        .filter_map(|l| {
            let (name, value) = l.split_once(" = ")?;
            let bits = u64::from_str_radix(value.split(' ').next()?, 16).ok()?;
            Some((name, f64::from_bits(bits)))
        })
        .collect()
}

/// GM8's `power(x, y)` for a fractional `y`: Delphi's `Power`, `Exp(y * Ln(x))`, the product rounded to 53 bits
/// with the 80-bit logarithm going in.
fn power(x: f64, y: f64) -> f64 {
    let ln = F80::from_f64(x).unwrap().ln().unwrap();
    gm8::exp(
        F80::from_f64(y)
            .unwrap()
            .mul(ln, Precision::Double)
            .to_f64(),
    )
}

/// What the port computes for one `probes/x87/` line, as (function, value).
fn port(probe: &str, name: &str) -> (&'static str, f64) {
    let (f, arg) = name.split_once('_').unwrap();
    let int = |s: &str| f64::from(s.parse::<i32>().unwrap());
    let pair = |s: &str| {
        let (a, b) = s.split_once('_').unwrap();
        (int(a), int(b))
    };
    match (probe, f) {
        ("deg", "rad") => ("degtorad", degtorad(int(arg) / 2.0)),
        ("deg", "sin") => ("sin (degrees)", sin(degtorad(int(arg) / 2.0))),
        ("deg", "cos") => ("cos (degrees)", cos(degtorad(int(arg) / 2.0))),
        ("deg", "hsp" | "vsp") => {
            let k = int(arg);
            let (h, v) = gm8::motion(k / 2.0, 1.0 + (k % 8.0) / 2.0);
            if f == "hsp" {
                ("motion hspeed", h)
            } else {
                ("motion vspeed", v)
            }
        }
        ("deg", "dir" | "spd") => {
            let (a, b) = pair(arg);
            let (h, v) = (a / 2.0 - 6.0, b / 2.0 - 6.0);
            if f == "spd" {
                ("motion speed", speed(h, v).unwrap())
            } else {
                ("motion direction", gm8::direction(h, v))
            }
        }
        ("rad", _) => {
            let k = int(&arg[1..]);
            let (x, set) = match &arg[..1] {
                "c" => (k / 100.0, "k/100"),
                "s" => (k / 64.0, "k/64"),
                _ => (k * k / 3.0, "k*k/3"),
            };
            let (name, y) = match f {
                "sin" => ("sin", sin(x)),
                "cos" => ("cos", cos(x)),
                _ => ("tan", gm8::tan(x)),
            };
            (
                match (name, set) {
                    ("sin", "k/100") => "sin (k/100)",
                    ("sin", "k/64") => "sin (k/64)",
                    ("sin", _) => "sin (k*k/3)",
                    ("cos", "k/100") => "cos (k/100)",
                    ("cos", "k/64") => "cos (k/64)",
                    ("cos", _) => "cos (k*k/3)",
                    (_, "k/100") => "tan (k/100)",
                    (_, "k/64") => "tan (k/64)",
                    _ => "tan (k*k/3)",
                },
                y,
            )
        }
        ("dir", _) => {
            let (i, j) = pair(arg);
            let (dx, dy) = (i * 3.0 / 8.0 - 7.0, j * 5.0 / 8.0 - 5.0);
            match f {
                "pdir" => ("point_direction (whole)", point_direction(0.0, 0.0, i, j)),
                "atan2" => ("arctan2 (whole)", atan2(-j, i)),
                "fpdir" => (
                    "point_direction (fractional)",
                    point_direction(0.0, 0.0, dx, dy),
                ),
                _ => (
                    "point_distance (fractional)",
                    point_distance(0.0, 0.0, dx, dy).unwrap(),
                ),
            }
        }
        ("funcs", _) => {
            let v = int(arg) / 100.0;
            match f {
                "arctan" => ("arctan", arctan(v)),
                "sqrt" => ("sqrt", sqrt(v).unwrap()),
                "ln" => ("ln", gm8::ln(v)),
                "exp" => ("exp", gm8::exp(v)),
                "power" => ("power", power(v, 3.0 / 2.0)),
                _ => (
                    "point_distance",
                    point_distance(0.0, 0.0, v, 13.0 / 10.0).unwrap(),
                ),
            }
        }
        _ => panic!("{probe}: {name}"),
    }
}

/// (function, lines): every one must match.
const LINES: &[(&str, usize)] = &[
    ("arctan", 1000),
    ("arctan2 (whole)", 2401),
    ("cos (degrees)", 720),
    ("cos (k*k/3)", 300),
    ("cos (k/100)", 1000),
    ("cos (k/64)", 640),
    ("degtorad", 720),
    ("exp", 1000),
    ("ln", 1000),
    ("motion direction", 625),
    ("motion hspeed", 720),
    ("motion speed", 625),
    ("motion vspeed", 720),
    ("point_direction (fractional)", 800),
    ("point_direction (whole)", 2401),
    ("point_distance", 1000),
    ("point_distance (fractional)", 800),
    ("power", 1000),
    ("sin (degrees)", 720),
    ("sin (k*k/3)", 300),
    ("sin (k/100)", 1000),
    ("sin (k/64)", 640),
    ("sqrt", 1000),
    ("tan (k*k/3)", 300),
    ("tan (k/100)", 1000),
    ("tan (k/64)", 640),
];

#[test]
fn the_port_computes_what_gm8_computes_on_intel() {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut wrong = Vec::new();
    for (probe, text) in INTEL {
        for (name, gm8) in lines(text) {
            let (f, port) = port(probe, name);
            *counts.entry(f).or_default() += 1;
            if port.to_bits() != gm8.to_bits() {
                wrong.push(format!("{probe}: {name}: port {port:e}, GM8 {gm8:e}"));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "{} lines differ:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
    assert_eq!(counts.into_iter().collect::<Vec<_>>(), LINES);
}

/// `radtodeg` (2,721 values) and `point_direction` between equal points, which `extra.gml` measured, are
/// arithmetic only, so Rosetta's answers are Intel's. `lengthdir_x` and `lengthdir_y` came out equal to the
/// runner's `hspeed` and `vspeed`, bit for bit, on every half degree (with Rosetta's `fcos` and `fsin` in
/// both), which is why the port computes them as the motion variables.
#[test]
fn radtodeg_and_lengthdir_as_gm8_computes_them() {
    let gm8 = lines(ROSETTA_EXTRA);
    let mut n = 0;
    for (name, &want) in &gm8 {
        let (f, arg) = name.split_once('_').unwrap();
        let got = match f {
            "r2d" => radtodeg(f64::from(arg.parse::<i32>().unwrap()) / 100.0).unwrap(),
            "r2dd" => radtodeg(degtorad(f64::from(arg.parse::<i32>().unwrap()) / 2.0)).unwrap(),
            "ldx" => {
                assert_eq!(want.to_bits(), gm8[&*format!("hsp_{arg}")].to_bits());
                continue;
            }
            "ldy" => {
                assert_eq!(want.to_bits(), gm8[&*format!("vsp_{arg}")].to_bits());
                continue;
            }
            "pdir" => match arg {
                "same" => point_direction(3.0, 5.0, 3.0, 5.0),
                "right" => point_direction(3.0, 5.0, 4.0, 5.0),
                _ => point_direction(3.0, 5.0, 2.0, 5.0),
            },
            _ => continue,
        };
        assert_eq!(
            got.to_bits(),
            want.to_bits(),
            "{name}: port {got:e}, GM8 {want:e}"
        );
        n += 1;
    }
    assert_eq!(n, 2001 + 720 + 3);
    // And the port's lengthdir is its motion.
    for k in 0..720 {
        let (d, len) = (f64::from(k) / 2.0, 1.0 + f64::from(k % 8) / 2.0);
        assert_eq!(
            (lengthdir_x(len, d).unwrap(), lengthdir_y(len, d).unwrap()),
            gm8::motion(d, len)
        );
    }
}

/// `probes/x87/angle-motion-add.gml` on Intel (851-3262): `motion_add`, the runner's routine behind gravity,
/// on every half degree from the speeds the probe printed first. Here as well as in `tests/gm8_probe.rs`
/// (which steps the port's gravity) so that the x87 model's runner conversion runs against GM8 on wasm32 too.
#[test]
fn gravity_and_motion_add_as_gm8_computes_them() {
    let gm8 = lines(include_str!(
        "../../../../tools/gm8-probe/results/gm8-intel/x87/angle-motion-add.txt"
    ));
    for k in 0..720 {
        let get = |p: &str| gm8[&*format!("{p}_{k}")];
        // The probe's motion_add(k / 2, 0.5 + (k mod 6) / 4).
        let (dir, amount) = (f64::from(k) / 2.0, 0.5 + f64::from(k % 6) / 4.0);
        let (h, v) = gm8::add_motion(get("ma0h"), get("ma0v"), dir, amount);
        assert_eq!(
            (h.to_bits(), v.to_bits()),
            (get("mah").to_bits(), get("mav").to_bits()),
            "line {k}"
        );
    }
}
