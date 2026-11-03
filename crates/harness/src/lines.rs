//! GML line coverage (851-3775): which lines of the game's GML the committed replays run, so "what has the
//! oracle never compared?" has an answer below the object level. An object can be active in many replays
//! while some of its events, and most branches of its scripts, never run.
//!
//! The lines a run reaches come from [`gm::World::record_lines`] (every `(unit, line)` the generated code's
//! `at` calls pass, booting included). Every line there is comes from the generated code itself: each
//! statement is a `// N: <GML>` comment then `g.at("<unit>", N);` (crates/transpile), so the universe and
//! the GML text of each line are read from `crates/game/src/generated` and `crates/config/src/generated`,
//! as committed. Hand-written overrides (the `SS_*` wrappers) call no `at`, so they're in neither.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// A unit's lines: line number to its GML, as the generated code's comment has it.
pub type UnitLines = BTreeMap<u32, String>;

/// Every GML line of a program's generated code, by unit (`oSnake/step#1`, `scrInit`).
pub fn universe(generated: &Path) -> Result<BTreeMap<String, UnitLines>, String> {
    let mut units: BTreeMap<String, UnitLines> = BTreeMap::new();
    let mut files = Vec::new();
    rust_files(generated, &mut files)?;
    files.sort();
    for file in files {
        let text =
            std::fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
        add_lines(&text, &mut units);
    }
    Ok(units)
}

fn rust_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) -> Result<(), String> {
    for entry in std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let path = entry.map_err(|e| format!("{}: {e}", dir.display()))?.path();
        if path.is_dir() {
            rust_files(&path, out)?;
        } else if path.extension().is_some_and(|x| x == "rs") {
            out.push(path);
        }
    }
    Ok(())
}

/// The `(unit, line)`s of one generated file, with the GML each comment gives (empty if none does).
fn add_lines(text: &str, units: &mut BTreeMap<String, UnitLines>) {
    let mut comment: Option<(u32, &str)> = None;
    for raw in text.lines() {
        let line = raw.trim();
        if let Some(rest) = line.strip_prefix("// ")
            && let Some((n, gml)) = rest.split_once(": ")
            && let Ok(n) = n.parse::<u32>()
        {
            comment = Some((n, gml));
            continue;
        }
        // `g.at("unit", N);`, the only call of `at` the generator writes.
        if let Some(rest) = line.strip_prefix("g.at(\"")
            && let Some((unit, rest)) = rest.split_once("\", ")
            && let Some(n) = rest.strip_suffix(");")
            && let Ok(n) = n.parse::<u32>()
        {
            let gml = comment.filter(|&(c, _)| c == n).map_or("", |(_, gml)| gml);
            let lines = units.entry(unit.to_owned()).or_default();
            // A line split over several statements (a `for`'s parts) is one line, its first comment's.
            lines.entry(n).or_insert_with(|| gml.to_owned());
        }
    }
}

/// A partly run unit: its name, how many lines it has, and the ones that didn't run with their GML.
pub type PartlyRun = (String, usize, Vec<(u32, String)>);

/// What a set of runs covered of a program's lines.
#[derive(Debug, Default)]
pub struct LineReport {
    pub lines: usize,
    pub lines_run: usize,
    pub units: usize,
    /// Units none of whose lines ran.
    pub never: Vec<(String, usize)>,
    /// Units some of whose lines ran: the ones that didn't, with their GML.
    pub partly: Vec<PartlyRun>,
}

/// Holds `run`, every `(unit, line)` the runs reached, to `universe`.
pub fn report(universe: &BTreeMap<String, UnitLines>, run: &BTreeSet<(String, u32)>) -> LineReport {
    let mut r = LineReport::default();
    for (unit, lines) in universe {
        r.units += 1;
        r.lines += lines.len();
        let missed: Vec<(u32, String)> = lines
            .iter()
            .filter(|(n, _)| !run.contains(&(unit.clone(), **n)))
            .map(|(n, gml)| (*n, gml.clone()))
            .collect();
        r.lines_run += lines.len() - missed.len();
        if missed.len() == lines.len() {
            r.never.push((unit.clone(), lines.len()));
        } else if !missed.is_empty() {
            r.partly.push((unit.clone(), lines.len(), missed));
        }
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    const GENERATED: &str = r#"
/// GML: `oSnake/step#1`.
pub fn step_a1(g: &mut World, cx: &mut Ctx) -> R<()> {
    // 2: if (hp < 1)
    g.at("oSnake/step#1", 2);
    if truthy(g.get(cx.this, v::hp)?) {
        // 3: instance_destroy();
        g.at("oSnake/step#1", 3);
        instance_destroy(g, cx)?;
    }
    // 5: for (i = 0; i < 3; i += 1) x += 1;
    g.at("oSnake/step#1", 5);
    loop {
        g.at("oSnake/step#1", 5);
    }
}
/// GML: `scrDead`.
pub fn scrDead(g: &mut World) -> R<()> {
    // 1: return 0;
    g.at("scrDead", 1);
}
"#;

    #[test]
    fn reads_every_line_and_its_gml_and_reports_what_ran() {
        let mut units = BTreeMap::new();
        add_lines(GENERATED, &mut units);
        assert_eq!(
            units["oSnake/step#1"],
            BTreeMap::from([
                (2, "if (hp < 1)".to_owned()),
                (3, "instance_destroy();".to_owned()),
                (5, "for (i = 0; i < 3; i += 1) x += 1;".to_owned())
            ])
        );
        assert_eq!(units["scrDead"].len(), 1);
        let run = BTreeSet::from([
            ("oSnake/step#1".to_owned(), 2),
            ("oSnake/step#1".to_owned(), 5),
        ]);
        let r = report(&units, &run);
        assert_eq!((r.units, r.lines, r.lines_run), (2, 4, 2));
        assert_eq!(r.never, [("scrDead".to_owned(), 1)]);
        assert_eq!(
            r.partly,
            [(
                "oSnake/step#1".to_owned(),
                3,
                vec![(3, "instance_destroy();".to_owned())]
            )]
        );
    }
}
