//! Compares the platform math library (what Rust's `f64::sin` and friends call: glibc on Linux, Apple's
//! libm on macOS) with the `libm` crate and two builds of `pxfm`, bit for bit, on the inputs Spelunky's
//! GML can feed the transcendental built-ins (851-3167). Run it natively, on wasm32, and in Docker on Linux
//! arm64 and amd64; see README.md.

// Calling the platform's libm is the point here.
#![allow(clippy::disallowed_methods)]

use std::collections::BTreeMap;
use std::fmt::Write as _;

/// A tiny deterministic generator so every platform sees the same random inputs.
struct XorShift(u64);

impl XorShift {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    /// Uniform in [lo, hi).
    fn uniform(&mut self, lo: f64, hi: f64) -> f64 {
        let unit = (self.next() >> 11) as f64 / (1u64 << 53) as f64;
        lo + unit * (hi - lo)
    }
}

/// `pxfm` is the vendored copy the port uses (`vendor/pxfm`); `upstream` is pxfm 0.1.30 from crates.io, which on
/// x86 picks an AVX+FMA code path at run time.
const NAMES: [&str; 4] = ["platform", "libm", "pxfm", "upstream"];
const PAIRS: [(usize, usize); 4] = [(0, 1), (0, 2), (1, 2), (2, 3)];
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

/// Mismatch counts for one pair of implementations.
#[derive(Default)]
struct Pair {
    mismatches: u64,
    max_ulps: u64,
    examples: Vec<String>,
}

#[derive(Default)]
struct Tally {
    count: u64,
    /// In `PAIRS` order.
    pairs: [Pair; 4],
    /// FNV-1a over each implementation's results, to compare runs on different platforms.
    hashes: [u64; 4],
    /// Disagreements written to the dump so far.
    dumped: u64,
}

/// Bitwise equality, except that any two NaNs count as the same (payloads and signs vary by platform).
fn same(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
}

fn ulps(a: f64, b: f64) -> u64 {
    let key = |x: f64| {
        let bits = x.to_bits() as i64;
        if bits < 0 { i64::MIN - bits } else { bits }
    };
    key(a).abs_diff(key(b))
}

struct Report<'a> {
    tallies: BTreeMap<String, Tally>,
    /// The first disagreements per label go here as hex lines (`function args... platform libm pxfm`),
    /// for `check_reference.py` to judge.
    dump: Option<&'a mut dyn std::io::Write>,
}

impl Report<'_> {
    fn check(&mut self, label: &str, args: &[f64], results: [f64; 4]) {
        let tally = self
            .tallies
            .entry(label.to_owned())
            .or_insert_with(|| Tally {
                hashes: [FNV_OFFSET; 4],
                ..Tally::default()
            });
        tally.count += 1;
        let disagree = results.iter().any(|&r| !same(r, results[0]));
        if let (true, true, Some(dump)) = (disagree, tally.dumped < 3000, self.dump.as_mut()) {
            use std::io::Write as _;
            tally.dumped += 1;
            let function = label.rsplit(": ").next().unwrap_or(label);
            let hex: Vec<String> = args
                .iter()
                .chain(&results)
                .map(|v| format!("{:016x}", v.to_bits()))
                .collect();
            let _ = writeln!(dump, "{function} {}", hex.join(" "));
        }
        for (hash, r) in tally.hashes.iter_mut().zip(results) {
            for byte in r.to_bits().to_le_bytes() {
                *hash = (*hash ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3);
            }
        }
        for (pair, (i, j)) in tally.pairs.iter_mut().zip(PAIRS) {
            let (a, b) = (results[i], results[j]);
            if !same(a, b) {
                pair.mismatches += 1;
                pair.max_ulps = pair.max_ulps.max(ulps(a, b));
                if pair.examples.len() < 8 {
                    let args: Vec<String> = args
                        .iter()
                        .map(|a| format!("{a:?} ({:#018x})", a.to_bits()))
                        .collect();
                    pair.examples.push(format!(
                        "{} -> {} {a:?} ({:#018x}), {} {b:?} ({:#018x})",
                        args.join(", "),
                        NAMES[i],
                        a.to_bits(),
                        NAMES[j],
                        b.to_bits()
                    ));
                }
            }
        }
    }

    fn sin(&mut self, domain: &str, x: f64) {
        self.check(
            &format!("{domain}: sin"),
            &[x],
            [x.sin(), libm::sin(x), pxfm::f_sin(x), upstream::f_sin(x)],
        );
    }

    fn sin_cos(&mut self, domain: &str, x: f64) {
        self.sin(domain, x);
        self.check(
            &format!("{domain}: cos"),
            &[x],
            [x.cos(), libm::cos(x), pxfm::f_cos(x), upstream::f_cos(x)],
        );
    }

    fn atan2(&mut self, domain: &str, y: f64, x: f64) {
        self.check(
            &format!("{domain}: atan2"),
            &[y, x],
            [
                y.atan2(x),
                libm::atan2(y, x),
                pxfm::f_atan2(y, x),
                upstream::f_atan2(y, x),
            ],
        );
    }

    fn tan(&mut self, domain: &str, x: f64) {
        self.check(
            &format!("{domain}: tan"),
            &[x],
            [x.tan(), libm::tan(x), pxfm::f_tan(x), upstream::f_tan(x)],
        );
    }

    fn atan(&mut self, domain: &str, x: f64) {
        self.check(
            &format!("{domain}: atan"),
            &[x],
            [
                x.atan(),
                libm::atan(x),
                pxfm::f_atan(x),
                upstream::f_atan(x),
            ],
        );
    }
}

/// GML `point_direction`, used to build realistic `sin`/`cos` inputs. Any correct `atan2` will do here;
/// `atan2` itself is checked separately.
fn point_direction(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    pxfm::f_atan2(y1 - y2, x2 - x1)
        .to_degrees()
        .rem_euclid(360.0)
}

/// Runs every check and returns the report as text.
pub fn run(dump: Option<&mut dyn std::io::Write>) -> String {
    let mut report = Report {
        tallies: BTreeMap::new(),
        dump,
    };
    let mut rng = XorShift(0x5eed_1234_abcd_ef01);

    // Tenths of a degree, as `degtorad(n)`: runner motion (`direction`), `image_angle`, and the `+ 180`
    // and `+ rand(0,1) - rand(0,1)` offsets the game adds to directions.
    for tenth in -7200..=14400 {
        let deg = f64::from(tenth) / 10.0;
        report.sin_cos("degtorad(k/10), k/10 in -720..1440 deg", deg.to_radians());
        report.tan("degtorad(k/10), k/10 in -720..1440 deg", deg.to_radians());
    }

    // point_direction between points on an integer grid, then the game's offsets and sin/cos of the
    // result (oBat, oVampire, oGhost, oPiranha, oDeadFish, oJaws, oScarab, oPsychic*).
    const R: i32 = 256;
    for dy in -R..=R {
        for dx in -R..=R {
            let (dx, dy) = (f64::from(dx), f64::from(dy));
            report.atan2("atan2 on integer grid |d|<=256", dy, dx);
            let dir = point_direction(0.0, 0.0, dx, dy);
            for offset in [-1.0, 0.0, 1.0, 180.0] {
                report.sin_cos(
                    "degtorad(point_direction(int grid) + {-1,0,1,180})",
                    (dir + offset).to_radians(),
                );
            }
        }
    }

    // Monsters move by fractional velocities, so also random fractional positions.
    for _ in 0..2_000_000 {
        let (x1, y1) = (rng.uniform(0.0, 1024.0), rng.uniform(0.0, 1024.0));
        let (x2, y2) = (rng.uniform(0.0, 1024.0), rng.uniform(0.0, 1024.0));
        report.atan2("atan2 on random fractional positions", y1 - y2, x2 - x1);
        let dir = point_direction(x1, y1, x2, y2);
        report.sin_cos(
            "degtorad(point_direction(random positions))",
            dir.to_radians(),
        );
    }

    // oTarget / oBatTarget: `sin(xDiff)` with `xDiff += 0.01` every step from 0 (2M steps is 18.5 hours).
    let mut x_diff = 0.0_f64;
    for _ in 0..2_000_000 {
        report.sin("sin(xDiff), xDiff += 0.01", x_diff);
        x_diff += 0.01;
    }

    // oArrow / oFishBone: `arctan(yVel / xVel)` on velocity ratios.
    for yv in -200..=200 {
        for xv in -200..=200 {
            if xv != 0 {
                report.atan(
                    "atan(p/q), p,q in -20..20 step 0.1",
                    (f64::from(yv) / 10.0) / (f64::from(xv) / 10.0),
                );
            }
        }
    }
    for _ in 0..2_000_000 {
        let ratio =
            libm::exp(rng.uniform(-3.0, 3.0)) * if rng.next() & 1 == 0 { 1.0 } else { -1.0 };
        report.atan("atan(random ratio, |r| in e^-3..e^3)", ratio);
    }

    // Special values: signed zeros, infinities, NaN, tiny and huge magnitudes, multiples of pi.
    let specials = [
        0.0,
        -0.0,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        5e-324,
        1e-300,
        1e-8,
        1.0,
        -1.0,
        1e8,
        1e22,
        1e300,
        f64::MAX,
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::PI,
        -std::f64::consts::PI,
        2.0 * std::f64::consts::PI,
    ];
    for &x in &specials {
        report.sin_cos("special values", x);
        report.tan("special values", x);
        report.atan("special values", x);
        for &y in &specials {
            report.atan2("special values", y, x);
        }
    }

    // Wide random radians, in case some path produces large angles.
    for _ in 0..2_000_000 {
        report.sin_cos("random radians in [-100, 100]", rng.uniform(-100.0, 100.0));
    }
    // `tan` isn't in the game, but the oracle routes GML `tan` through pxfm too, so check it the same way.
    for _ in 0..1_000_000 {
        report.tan("random radians in [-100, 100]", rng.uniform(-100.0, 100.0));
    }

    let mut out = String::new();
    for (label, tally) in &report.tallies {
        let _ = writeln!(
            out,
            "{label} (n={})\n  hashes: platform {:016x}, libm {:016x}, pxfm {:016x}, upstream {:016x}",
            tally.count, tally.hashes[0], tally.hashes[1], tally.hashes[2], tally.hashes[3]
        );
        for (pair, (i, j)) in tally.pairs.iter().zip(PAIRS) {
            let _ = writeln!(
                out,
                "  {} vs {}: mismatches={} max_ulps={}",
                NAMES[i], NAMES[j], pair.mismatches, pair.max_ulps
            );
            if (i, j) == (0, 2) || (i, j) == (2, 3) {
                for example in &pair.examples {
                    let _ = writeln!(out, "    {example}");
                }
            }
        }
    }
    out
}

/// The wasm32 entry point: runs the probe and returns a pointer to the report, whose length `probe_len`
/// gives. `run-wasm.mjs` drives it under Node.
#[cfg(target_arch = "wasm32")]
mod wasm {
    use std::sync::Mutex;

    static REPORT: Mutex<String> = Mutex::new(String::new());

    #[unsafe(no_mangle)]
    pub extern "C" fn probe_run() -> *const u8 {
        let mut report = REPORT.lock().unwrap();
        *report = super::run(None);
        report.as_ptr()
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn probe_len() -> usize {
        REPORT.lock().unwrap().len()
    }
}
