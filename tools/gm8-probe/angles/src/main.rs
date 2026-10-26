//! Generates the `probes/x87/angle-*.gml` probes and checks their results against models of the runner
//! (851-3262; README.md, "Angles"):
//!
//! ```sh
//! cargo run --release -- gen ../probes/x87              # writes angle-{bbox,mask,motion-add,view}.gml
//! cargo run --release -- check angle-bbox ../results/gm8-intel/x87/angle-bbox.txt
//! ```
//!
//! `check` prints, for each candidate model, on how many lines it differs from the result.

mod cases;
mod model;

use cases::*;
use model::*;
use std::collections::BTreeMap;

/// A model of the mouse through a view.
type ViewFn = fn(Conv, &View, i32, i32) -> (i32, i32);

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match (args.get(1).map(String::as_str), args.get(2), args.get(3)) {
        (Some("gen"), Some(dir), None) => generate(dir),
        (Some("check"), Some(probe), Some(result)) => check(probe, result),
        _ => {
            eprintln!(
                "usage: gm8-angle-probes gen DIR | check angle-bbox|angle-mask|angle-motion-add|angle-view RESULT.txt"
            );
            std::process::exit(2);
        }
    }
}

fn generate(dir: &str) {
    let write =
        |name: &str, text: String| std::fs::write(format!("{dir}/{name}.gml"), text).unwrap();
    write("angle-bbox", bbox_gml(&bbox_cases()));
    write("angle-mask", mask_gml(&mask_cases()));
    write("angle-motion-add", motion_gml());
    write("angle-view", view_gml(&view_cases()));
}

/// A probe's lines by name: the text of a string, or `<bits> <string>` for a real.
fn lines(path: &str) -> BTreeMap<String, String> {
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter_map(|l| l.split_once(" = "))
        .map(|(k, v)| (k.to_string(), v.trim_matches('"').to_string()))
        .collect()
}

fn bits(v: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(v.split(' ').next().unwrap(), 16).unwrap())
}

/// Counts, per model, the lines where it differs from `got`, in total and per line-name prefix.
struct Tally {
    models: Vec<&'static str>,
    total: usize,
    wrong: Vec<usize>,
    groups: BTreeMap<String, (usize, Vec<usize>)>,
}

impl Tally {
    fn new(models: &[&'static str]) -> Tally {
        Tally {
            models: models.to_vec(),
            total: 0,
            wrong: vec![0; models.len()],
            groups: BTreeMap::new(),
        }
    }

    fn line(&mut self, name: &str, right: &[bool]) {
        self.total += 1;
        let group = name.split('_').next().unwrap().to_string();
        let g = self
            .groups
            .entry(group)
            .or_insert_with(|| (0, vec![0; right.len()]));
        g.0 += 1;
        for (i, &ok) in right.iter().enumerate() {
            if !ok {
                self.wrong[i] += 1;
                g.1[i] += 1;
            }
        }
    }

    fn print(&self, probe: &str) {
        println!("{probe}: {} lines", self.total);
        for (m, w) in self.models.iter().zip(&self.wrong) {
            println!("  {m:<52} differs on {w}");
        }
        for (g, (n, w)) in &self.groups {
            println!("  [{g}] {n} lines: {w:?}");
        }
    }
}

fn check(probe: &str, result: &str) {
    let r = lines(result);
    match probe {
        "angle-bbox" => {
            let models: [(&str, Conv, BoxFn); 5] = [
                (
                    "GM8's formula, the runner's conversion",
                    Conv::Runner,
                    gm8_bbox,
                ),
                ("GM8's formula, to_radians", Conv::ToRadians, gm8_bbox),
                ("GM8's formula, degtorad", Conv::DegToRad, gm8_bbox),
                (
                    "OpenGMK's formula, to_radians (the port before 851-3262)",
                    Conv::ToRadians,
                    opengmk_bbox,
                ),
                (
                    "OpenGMK's formula, the runner's conversion (to 851-3278)",
                    Conv::Runner,
                    opengmk_bbox,
                ),
            ];
            let mut t = Tally::new(&models.map(|m| m.0));
            for s in &BBOX_SPRITES {
                let want = format!("{} {} {} {} {} {}", s.ox, s.oy, s.l, s.t, s.r, s.b);
                assert_eq!(
                    r.get(&format!("spr_{}", s.name)),
                    Some(&want),
                    "{}'s box",
                    s.name
                );
            }
            for (name, c) in bbox_cases() {
                let Some(got) = r.get(&name) else { continue };
                let s = &BBOX_SPRITES[c.s];
                let right = models.map(|(_, conv, f)| {
                    let b = f(conv, s, c.x, c.y, c.xs, c.ys, c.a);
                    format!("{} {} {} {}", b.l, b.t, b.r, b.b) == *got
                });
                t.line(&name, &right);
            }
            t.print(probe);
        }
        "angle-mask" => {
            let models: [(&str, Conv, BoxFn); 5] = [
                ("GM8, the runner's conversion", Conv::Runner, gm8_bbox),
                ("GM8, to_radians", Conv::ToRadians, gm8_bbox),
                ("GM8, degtorad", Conv::DegToRad, gm8_bbox),
                (
                    "OpenGMK's box and masks, to_radians (before)",
                    Conv::ToRadians,
                    opengmk_bbox,
                ),
                (
                    "OpenGMK's box and masks, the runner's (to 851-3278)",
                    Conv::Runner,
                    opengmk_bbox,
                ),
            ];
            let mut t = Tally::new(&models.map(|m| m.0));
            for (name, c) in mask_cases() {
                let Some(got) = r.get(&name) else { continue };
                let right = models.map(|(_, conv, f)| mask_hit(conv, f, &c) == (got == "1"));
                t.line(&name, &right);
            }
            t.print(probe);
        }
        "angle-motion-add" => {
            let names = [
                "AddMotion: the runner's conversion, 80-bit sin/cos",
                "the runner's conversion, sin/cos stored as doubles",
                "to_radians, 80-bit",
                "degtorad, 80-bit",
                "the port's gravity before 851-3262",
            ];
            let mut t = Tally::new(&names);
            for k in 0..720 {
                let get = |p: &str| r.get(&format!("{p}_{k}")).map(|v| bits(v));
                let (Some(h0), Some(v0), Some(h), Some(v)) =
                    (get("ma0h"), get("ma0v"), get("mah"), get("mav"))
                else {
                    continue;
                };
                let (dir, amount) = motion_args(k);
                let vals = [
                    add_motion(Conv::Runner, true, h0, v0, dir, amount),
                    add_motion(Conv::Runner, false, h0, v0, dir, amount),
                    add_motion(Conv::ToRadians, true, h0, v0, dir, amount),
                    add_motion(Conv::DegToRad, true, h0, v0, dir, amount),
                    port_add_motion_before(h0, v0, dir, amount),
                ];
                t.line("mah", &vals.map(|m| m.0.to_bits() == h.to_bits()));
                t.line("mav", &vals.map(|m| m.1.to_bits() == v.to_bits()));
            }
            t.print(probe);
        }
        "angle-view" => {
            let models: [(&str, Conv, ViewFn); 5] = [
                (
                    "GM8: the runner's conversion, 80-bit, 0.01 threshold",
                    Conv::Runner,
                    gm8_view,
                ),
                ("GM8's transform, to_radians", Conv::ToRadians, gm8_view),
                ("GM8's transform, degtorad", Conv::DegToRad, gm8_view),
                (
                    "the port's to_room, to_radians (before)",
                    Conv::ToRadians,
                    port_view,
                ),
                (
                    "the port's to_room, the runner's (after)",
                    Conv::Runner,
                    port_view,
                ),
            ];
            let mut t = Tally::new(&models.map(|m| m.0));
            for (name, c) in view_cases() {
                let Some(got) = r.get(&name) else { continue };
                let right = models.map(|(_, conv, f)| {
                    let (x, y) = f(conv, &c.v, c.mx, c.my);
                    format!("{x} {y}") == *got
                });
                t.line(&name, &right);
            }
            t.print(probe);
        }
        _ => panic!("unknown probe {probe}"),
    }
}
