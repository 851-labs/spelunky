//! Extracts the wiki's numbers from the game's code: each monster's health, damage and speeds, each
//! item's price and damage, treasure values, shop markups. Every number carries where it came from, so a
//! page can cite the code and a reviewer can check it.
//!
//! ```sh
//! wiki-data --assets assets/ --specs wiki/data/ --out target/wiki-data/stats.json   # and relations.json
//!     [--site site/content/wiki-data/stats.json [--check]]
//! wiki-data --assets assets/ --find 'hp = ?' --in oSnake:create    # try a pattern while writing a spec
//! ```
//!
//! `scripts/wiki-data` runs the first with those defaults. `--site` also writes the site's committed
//! snapshots of the stats and, beside them, the relations (below), and `--check` only checks those are
//! current, writing nothing. The input is the bundle `scripts/extract`
//! writes: its objects' and scripts' GML, the original source, which the citations point into (the
//! `gml/` tree beside it). Not the transpiled `crates/game`: that is generated from the same GML, and
//! citing the original is what lets a reader check a number against the game's own code.
//!
//! # Specs
//!
//! What to extract is written down in `wiki/data/*.toml` (`docs/wiki-data.md`): per entity, each stat is
//! a GML pattern (`hp = ?`, `global.plife -= ?`) in an object's event or a script. The tool finds every
//! match, follows GM's event inheritance (an object without the event runs its parent's, and
//! `action_inherited` runs the parent's in place), and reports each match with its conditions (the `if`s,
//! loops and `with`s around it), its value when it's a constant, its range when it's random, and its
//! file and line. A pattern that matches nothing is an error, so a spec can't silently go stale.
//! Besides the specs, every monster's, item's and treasure's create event is read whole: the variables
//! it sets at the top level are each object's `init`.
//!
//! The relations (where things spawn, what shops sell, what things drop, what hurts what) go to
//! `relations.json` beside the stats: `relations.rs`.
//!
//! # The site's snapshot
//!
//! The site's build (Workers Builds) has no Rust and no extracted assets, so it can't run this tool: it
//! reads trimmed copies committed under `site/content/wiki-data/`, which `scripts/check` keeps current
//! with `--check`. The stats' keeps what a page renders and cites (each stat's label, unit, value and
//! lines; each object's create-event values), one stat or variable a line, so a diff reads well. The
//! relations' (`relations.json` beside it) keeps what the build checks a page's relations against, by
//! object, one object a line (`Relations::site_snapshot`).
//!
//! # Determinism
//!
//! The same bundle and specs give byte-identical output: specs are read in file-name order, objects
//! are listed by name, and nothing depends on time or the file system's order.

mod eval;
mod find;
mod relations;

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use assets::{ActionKind, Bundle, Object};
use gml::ast::{AssignOp, StmtKind};
use gml::{LineIndex, Program};
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;
use tools::{Arg, Args, WIKI_LICENSE_URL as LICENSE_URL, WIKI_NOTICE as NOTICE, gml_tree};

use eval::{Env, Range, Value};
use find::{Match, Pattern, When};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const USAGE: &str =
    "usage: wiki-data --assets <extracted bundle dir> --specs <spec dir> --out <file.json>
                 [--site <snapshot.json> [--check]]
       wiki-data --assets <extracted bundle dir> --find <pattern> --in <object>:<event> | <script>";

/// Bumped when the output's shape changes, so the site can tell.
const FORMAT: u32 = 1;

/// The objects whose descendants get an `objects` entry: every monster, item and treasure.
const ROOTS: [&str; 3] = ["oEnemy", "oItem", "oTreasure"];

/// The entity kinds a spec may give.
const KINDS: [&str; 8] = [
    "monster",
    "character",
    "weapon",
    "item",
    "equipment",
    "treasure",
    "trap",
    "mechanic",
];

fn main() -> ExitCode {
    match run(std::env::args().skip(1)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("wiki-data: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: impl IntoIterator<Item = String>) -> Result<()> {
    let mut args = Args::new(args);
    let (mut assets_dir, mut specs, mut out, mut pattern, mut place) =
        (None, None, None, None, None);
    let (mut site, mut check) = (None, false);
    while let Some(arg) = args.next() {
        let flag = match arg {
            Arg::Flag(flag) => flag,
            Arg::Positional(p) => return Err(format!("unexpected argument {p:?}\n{USAGE}").into()),
        };
        match flag.name.as_str() {
            "-h" | "--help" => {
                args.switch(&flag)?;
                println!("{USAGE}");
                return Ok(());
            }
            "--assets" => assets_dir = Some(PathBuf::from(args.value(&flag)?)),
            "--specs" => specs = Some(PathBuf::from(args.value(&flag)?)),
            "--out" => out = Some(PathBuf::from(args.value(&flag)?)),
            "--find" => pattern = Some(args.value(&flag)?),
            "--in" => place = Some(args.value(&flag)?),
            "--site" => site = Some(PathBuf::from(args.value(&flag)?)),
            "--check" => {
                args.switch(&flag)?;
                check = true;
            }
            name => return Err(format!("unexpected argument {name:?}\n{USAGE}").into()),
        }
    }
    let Some(assets_dir) = assets_dir else {
        return Err(USAGE.into());
    };
    if check && site.is_none() {
        return Err(format!("--check checks the --site snapshot, so it needs one\n{USAGE}").into());
    }
    if site.is_some() && pattern.is_some() {
        return Err(USAGE.into());
    }
    match (specs, out, pattern, place) {
        (Some(specs), Some(out), None, None) => {
            let specs = read_specs(&specs)?;
            let bundle = read_bundle(&assets_dir)?;
            let game = Game::new(&bundle);
            let stats = build(&game, &specs)?;
            let relations = relations::build(
                &game,
                &specs.relations,
                specs
                    .entities
                    .iter()
                    .filter_map(|e| Some((e.object.clone()?, e.slug.clone())))
                    .collect(),
                &stats.objects.keys().cloned().collect(),
            )
            .map_err(|e| format!("relations: {e}"))?;
            // relations.json goes beside stats.json, and the site's snapshot of it beside the site's.
            let relations_out = out.with_file_name("relations.json");
            if same_file(&relations_out, &out)? {
                return Err("--out can't be relations.json, which the relations go to".into());
            }
            if let Some(site) = &site {
                let site_relations = site.with_file_name("relations.json");
                if same_file(&site_relations, site)? || same_file(&site_relations, &relations_out)?
                {
                    return Err(format!(
                        "--site can't be beside --out or be relations.json: the site's relations go to {}",
                        site_relations.display()
                    )
                    .into());
                }
                let snapshots = [
                    (site.clone(), site_snapshot(&stats)?),
                    (
                        site_relations,
                        layered(&relations.site_snapshot(&parents(&game)), 2)?,
                    ),
                ];
                if check {
                    for (path, snapshot) in &snapshots {
                        if fs::read(path).ok().as_deref() != Some(&snapshot[..]) {
                            return Err(format!(
                                "{} isn't what the GML and the specs give: run scripts/wiki-data and commit it",
                                path.display()
                            )
                            .into());
                        }
                        println!("wiki-data: {} is current", path.display());
                    }
                    return Ok(());
                }
                for (path, snapshot) in &snapshots {
                    write_file(path, snapshot)?;
                }
            }
            let (json, relations_json) = (to_json(&stats)?, to_json(&relations)?);
            write_file(&out, &json)?;
            write_file(&relations_out, &relations_json)?;
            println!(
                "wiki-data: wrote {} entities ({} KiB) to {}, and relations ({} KiB) to {}",
                specs.entities.len(),
                json.len() / 1024,
                out.display(),
                relations_json.len() / 1024,
                relations_out.display()
            );
            Ok(())
        }
        (None, None, Some(pattern), Some(place)) => {
            let bundle = read_bundle(&assets_dir)?;
            let game = Game::new(&bundle);
            let (object, event, script) = match place.split_once(':') {
                Some((o, e)) => (Some(o.to_owned()), Some(e.to_owned()), None),
                None => (None, None, Some(place)),
            };
            let spec = StatSpec {
                find: pattern,
                object,
                event,
                script,
                ..StatSpec::default()
            };
            let (found, _) = game.find(&spec, None)?;
            for fact in &found.facts {
                println!("{}", serde_json::to_string(fact)?);
            }
            Ok(())
        }
        _ => Err(USAGE.into()),
    }
}

fn read_bundle(dir: &Path) -> Result<Bundle> {
    let path = dir.join(assets::BUNDLE_FILE);
    let bytes = fs::read(&path)
        .map_err(|e| format!("{}: {e} (run scripts/extract first)", path.display()))?;
    Ok(Bundle::from_bytes(&bytes).map_err(|e| format!("{}: {e}", path.display()))?)
}

/// Whether two paths, which needn't exist yet, name the same file: compared absolute, with `.` and `..`
/// resolved, and through the symbolic links of the part of each that exists.
fn same_file(a: &Path, b: &Path) -> Result<bool> {
    fn normal(path: &Path) -> Result<PathBuf> {
        let mut out = PathBuf::new();
        for c in std::path::absolute(path)?.components() {
            match c {
                std::path::Component::ParentDir => {
                    out.pop();
                }
                std::path::Component::CurDir => {}
                c => out.push(c),
            }
        }
        // The longest part that exists, through its links, then the rest.
        let mut existing = out.as_path();
        let mut rest = Vec::new();
        while !existing.exists() {
            let (Some(parent), Some(name)) = (existing.parent(), existing.file_name()) else {
                break;
            };
            rest.push(name.to_owned());
            existing = parent;
        }
        let mut real = existing
            .canonicalize()
            .unwrap_or_else(|_| existing.to_owned());
        real.extend(rest.iter().rev());
        Ok(real)
    }
    Ok(normal(a)? == normal(b)?)
}

/// Writes `data` to `path` through a sibling temporary file, so a failed run leaves the old file whole.
fn write_file(path: &Path, data: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".partial");
    let tmp = PathBuf::from(tmp);
    fs::write(&tmp, data).map_err(|e| format!("{}: {e}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(())
}

fn to_json(value: &impl Serialize) -> Result<Vec<u8>> {
    let mut out = serde_json::to_vec_pretty(value)?;
    out.push(b'\n');
    Ok(out)
}

/// The site's snapshot of the stats (the module docs): entities and objects by name, each stat and each
/// create-event variable a line. Citations are `file:line` in the `gml/` tree, as pages write them.
fn site_snapshot(stats: &Output) -> Result<Vec<u8>> {
    let cite =
        |file: &str, line: usize| format!("{}:{line}", file.strip_prefix("gml/").unwrap_or(file));
    let mut entities = serde_json::Map::new();
    for e in &stats.entities {
        let mut entity_stats = serde_json::Map::new();
        for s in &e.stats {
            let mut stat = serde_json::Map::new();
            stat.insert("label".into(), s.label.clone().into());
            if let Some(unit) = &s.unit {
                stat.insert("unit".into(), unit.clone().into());
            }
            for (key, v) in [("value", &s.value), ("min", &s.min), ("max", &s.max)] {
                if let Some(v) = v {
                    stat.insert(key.into(), v.clone());
                }
            }
            if let Some(table) = &s.table {
                stat.insert("table".into(), serde_json::to_value(&table.rows)?);
            }
            let mut at: Vec<String> = Vec::new();
            for f in &s.facts {
                let line = cite(&f.at.file, f.at.line);
                if !at.contains(&line) {
                    at.push(line);
                }
            }
            stat.insert("at".into(), at.into());
            entity_stats.insert(s.id.clone(), stat.into());
        }
        let mut entity = serde_json::Map::new();
        entity.insert("name".into(), e.name.clone().into());
        entity.insert("kind".into(), e.kind.clone().into());
        if let Some(object) = &e.object {
            entity.insert("object".into(), object.clone().into());
        }
        entity.insert("stats".into(), entity_stats.into());
        entities.insert(e.slug.clone(), entity.into());
    }
    let mut objects = serde_json::Map::new();
    for (name, o) in &stats.objects {
        let mut init = serde_json::Map::new();
        // Only what a page can show: a constant or a random range.
        for (var, i) in &o.init {
            let mut v = serde_json::Map::new();
            if let Some(value) = &i.value {
                v.insert("value".into(), value.clone());
            } else if let Some(range) = &i.range {
                v.insert("range".into(), serde_json::to_value(range)?);
            } else {
                continue;
            }
            if i.changed_later {
                v.insert("changed_later".into(), true.into());
            }
            v.insert("at".into(), cite(&i.file, i.line).into());
            init.insert(var.clone(), v.into());
        }
        let mut object = serde_json::Map::new();
        if let Some(parent) = &o.parent {
            object.insert("parent".into(), parent.clone().into());
        }
        if let Some(type_name) = &o.type_name {
            object.insert("type".into(), type_name.clone().into());
        }
        object.insert("init".into(), init.into());
        objects.insert(name.clone(), object.into());
    }
    let mut root = serde_json::Map::new();
    root.insert("format".into(), stats.format.into());
    root.insert("notice".into(), stats.notice.into());
    root.insert("license".into(), stats.license.into());
    root.insert("room_speed".into(), stats.room_speed.into());
    root.insert("entities".into(), entities.into());
    root.insert("objects".into(), objects.into());
    // Indented down to a stat or a variable, so each is one line.
    layered(&Json::Object(root), 4)
}

/// Every object's parent, by name: the relations' snapshot has them for following inheritance.
fn parents(game: &Game) -> BTreeMap<String, String> {
    let name = |id: usize| {
        let o = game.bundle.objects[id].as_ref().expect("object ids exist");
        o.name.clone()
    };
    (game.objects.values())
        .filter_map(|&id| Some((name(id), name(game.parent(id)?))))
        .collect()
}

/// `value` as JSON, with a newline, indented down to `levels` deep and compact from there, so that each
/// entry at that depth is one line and a diff reads well.
fn layered(value: &Json, levels: usize) -> Result<Vec<u8>> {
    let mut out = String::new();
    write_layered(&mut out, value, 0, levels)?;
    out.push('\n');
    Ok(out.into_bytes())
}

fn write_layered(out: &mut String, value: &Json, depth: usize, levels: usize) -> Result<()> {
    match value {
        Json::Object(map) if depth < levels && !map.is_empty() => {
            let indent = "  ".repeat(depth + 1);
            out.push('{');
            for (i, (key, v)) in map.iter().enumerate() {
                out.push_str(if i == 0 { "\n" } else { ",\n" });
                out.push_str(&indent);
                out.push_str(&serde_json::to_string(key)?);
                out.push_str(": ");
                write_layered(out, v, depth + 1, levels)?;
            }
            out.push('\n');
            out.push_str(&"  ".repeat(depth));
            out.push('}');
        }
        _ => out.push_str(&serde_json::to_string(value)?),
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------
// Specs

/// Every spec file, merged.
#[derive(Debug, Default)]
struct Specs {
    templates: BTreeMap<String, Vec<StatSpec>>,
    entities: Vec<EntitySpec>,
    relations: relations::RelationSpecs,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SpecFile {
    #[serde(default)]
    template: BTreeMap<String, Vec<StatSpec>>,
    #[serde(default)]
    entity: Vec<EntitySpec>,
    // Relations (relations.rs).
    generation: Option<relations::GenerationSpec>,
    #[serde(default)]
    area: Vec<relations::AreaSpec>,
    #[serde(default)]
    level: Vec<relations::LevelSpec>,
    #[serde(default)]
    shop: Vec<relations::ShopSpec>,
    #[serde(default)]
    damage: Vec<relations::DamageSpec>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EntitySpec {
    /// The wiki page's slug (`snake`, `giant-spider`).
    slug: String,
    name: String,
    kind: String,
    /// The object the stats are read from unless a stat says otherwise.
    object: Option<String>,
    /// Templates whose stats come first, in order.
    #[serde(default)]
    templates: Vec<String>,
    #[serde(default)]
    stat: Vec<StatSpec>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct StatSpec {
    id: String,
    label: String,
    unit: Option<String>,
    note: Option<String>,
    /// The object whose event to search (the entity's by default), or a script instead.
    object: Option<String>,
    script: Option<String>,
    /// The event, as the `gml/` tree names its file (`create`, `step`, `alarm_0`,
    /// `collision_oCharacter`). `create` by default.
    event: Option<String>,
    /// The pattern: GML with `?` for any expression.
    find: String,
    /// Keep only the matches that, for each of these texts, are under an `if` or `case` whose GML
    /// contains it (not an `else`'s).
    #[serde(rename = "where", default)]
    where_: Vec<String>,
    /// Keep only this match (0-based, in execution order, after `where`).
    nth: Option<usize>,
    /// How many matches there must be (after `where`, before `nth`), to catch a pattern that matches
    /// more than meant.
    matches: Option<usize>,
    table: Option<TableSpec>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TableSpec {
    /// The variable the table varies, as GML (`global.currLevel`).
    var: String,
    from: i64,
    to: i64,
    /// A name for each row, from `from` up.
    labels: Option<Vec<String>>,
    /// Other variables: a number, or the id of one of the entity's earlier stats (its `value`).
    #[serde(default)]
    bind: BTreeMap<String, toml::Value>,
}

fn read_specs(dir: &Path) -> Result<Specs> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let path = entry?.path();
        if path.extension().is_some_and(|e| e == "toml") {
            paths.push(path);
        }
    }
    paths.sort();
    let mut specs = Specs::default();
    for path in paths {
        let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let file: SpecFile =
            toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        for (name, stats) in file.template {
            if specs.templates.insert(name.clone(), stats).is_some() {
                return Err(format!("{}: template {name} is defined twice", path.display()).into());
            }
        }
        specs.entities.extend(file.entity);
        let r = &mut specs.relations;
        if let Some(generation) = file.generation
            && r.generation.replace(generation).is_some()
        {
            return Err(format!("{}: [generation] is given twice", path.display()).into());
        }
        r.areas.extend(file.area);
        r.levels.extend(file.level);
        r.shops.extend(file.shop);
        r.damage.extend(file.damage);
    }
    specs.relations.check()?;
    let mut slugs = BTreeSet::new();
    for e in &specs.entities {
        if !slugs.insert(e.slug.as_str()) {
            return Err(format!("entity {} is defined twice", e.slug).into());
        }
        if !KINDS.contains(&e.kind.as_str()) {
            return Err(format!(
                "entity {}: unknown kind {:?} (one of {KINDS:?})",
                e.slug, e.kind
            )
            .into());
        }
        for t in &e.templates {
            if !specs.templates.contains_key(t) {
                return Err(format!("entity {}: no template {t}", e.slug).into());
            }
        }
    }
    Ok(specs)
}

// ---------------------------------------------------------------------------------------------------------
// Output

#[derive(Serialize)]
struct Output {
    format: u32,
    notice: &'static str,
    license: &'static str,
    /// Steps a second: a speed in pixels a step times this is pixels a second, a timer in steps over
    /// this is seconds.
    room_speed: u32,
    entities: Vec<Entity>,
    /// Every monster, item and treasure object (and any other an entity reads), by name.
    objects: BTreeMap<String, ObjectInfo>,
}

#[derive(Serialize)]
struct Entity {
    slug: String,
    name: String,
    kind: String,
    object: Option<String>,
    stats: Vec<Stat>,
}

#[derive(Serialize)]
struct Stat {
    id: String,
    label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
    /// The pattern, where it was looked for (`object:event` or a script), and which matches were kept
    /// (the spec's `where` and `nth`).
    find: String,
    #[serde(rename = "in")]
    place: String,
    #[serde(rename = "where", skip_serializing_if = "Vec::is_empty")]
    where_: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nth: Option<usize>,
    /// The value every fact agrees on, if they do.
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<Json>,
    /// Otherwise the least and greatest any fact can give, if they're all numbers or ranges.
    #[serde(skip_serializing_if = "Option::is_none")]
    min: Option<Json>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max: Option<Json>,
    facts: Vec<Fact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    table: Option<Table>,
}

/// One place in the code that gives a stat.
#[derive(Clone, Serialize)]
struct Fact {
    /// The value's expression, as GML: what the pattern's first `?` matched (or the whole match).
    expr: String,
    /// Every `?`'s match, when the pattern has more than one.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    holes: Vec<String>,
    /// The expression's value, when it's a constant.
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<Json>,
    /// Its range, when it's random.
    #[serde(skip_serializing_if = "Option::is_none")]
    range: Option<Range>,
    /// What has to hold for the code to run, outermost first.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    when: Vec<When>,
    at: At,
}

/// Where a fact is in the original code.
#[derive(Clone, Serialize)]
struct At {
    /// The `gml/` tree's file, relative to the extracted assets, and the 1-based line there.
    file: String,
    line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    object: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    event: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    script: Option<String>,
    /// The matched code, verbatim (a compound statement's first line only).
    source: String,
}

#[derive(Serialize)]
struct Table {
    var: String,
    rows: Vec<Row>,
    /// Conditions the table took to hold because it couldn't evaluate them (a call, an unbound name).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    assumed: Vec<When>,
}

#[derive(Serialize)]
struct Row {
    at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<String>,
    value: Json,
}

#[derive(Serialize)]
struct ObjectInfo {
    parent: Option<String>,
    /// Its `type` string, which the game's code tests and shows (`"Snake"`, `"Bomb Bag"`).
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    type_name: Option<String>,
    /// What its create event (with its parents') sets at the top level, by variable.
    init: BTreeMap<String, Init>,
}

/// A variable a create event sets: `<name> = <expr>` at the given line.
#[derive(Serialize)]
struct Init {
    expr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<Json>,
    #[serde(skip_serializing_if = "Option::is_none")]
    range: Option<Range>,
    file: String,
    line: usize,
    /// Whether the create event may set it again afterwards, under a condition.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    changed_later: bool,
}

/// A number as JSON: whole numbers without a fraction, so a page prints `1`, not `1.0`.
fn number(v: f64) -> Json {
    if v.fract() == 0.0 && v.abs() < 9e15 {
        Json::from(v as i64)
    } else {
        Json::from(v)
    }
}

/// Serializes a number as [`number`] does.
fn serialize_number<S: serde::Serializer>(v: &f64, s: S) -> std::result::Result<S::Ok, S::Error> {
    number(*v).serialize(s)
}

fn json_value(v: &Value) -> Json {
    match v {
        Value::Real(r) => number(*r),
        Value::Str(s) => Json::from(s.clone()),
    }
}

// ---------------------------------------------------------------------------------------------------------
// The game's code

/// One piece of code that runs as part of an event: a code action, or a script.
struct Unit {
    code: String,
    program: Program,
    /// Its file in the `gml/` tree and the line its code starts on there.
    file: String,
    first_line: usize,
    object: Option<String>,
    event: Option<String>,
    script: Option<String>,
    /// Conditions on the whole unit: it applies to other instances, or follows a question action.
    when: Vec<When>,
}

struct Game<'a> {
    bundle: &'a Bundle,
    objects: BTreeMap<&'a str, usize>,
    scripts: BTreeMap<&'a str, usize>,
    script_files: Vec<Option<String>>,
    event_files: Vec<Option<Vec<String>>>,
}

/// The facts a stat's pattern found, and the matches' details a table needs.
struct Found {
    facts: Vec<Fact>,
    /// Per fact, what a table evaluates.
    evals: Vec<Evaluable>,
}

struct Evaluable {
    /// The matched statement's assignment, if it's one: its place and operator.
    assign: Option<(String, AssignOp)>,
    /// The source of the assignment's value, or of the expression the fact's `expr` is (its first hole,
    /// or the whole match).
    expr: String,
}

impl<'a> Game<'a> {
    fn new(bundle: &'a Bundle) -> Game<'a> {
        let names = |list: Vec<Option<&'a str>>| {
            list.into_iter()
                .enumerate()
                .filter_map(|(id, n)| Some((n?, id)))
                .collect()
        };
        Game {
            bundle,
            objects: names(
                bundle
                    .objects
                    .iter()
                    .map(|o| Some(o.as_ref()?.name.as_str()))
                    .collect(),
            ),
            scripts: names(
                bundle
                    .scripts
                    .iter()
                    .map(|s| Some(s.as_ref()?.name.as_str()))
                    .collect(),
            ),
            script_files: gml_tree::script_files(&bundle.scripts),
            event_files: gml_tree::event_files(&bundle.objects),
        }
    }

    fn object(&self, name: &str) -> Result<(usize, &'a Object)> {
        let id = *self
            .objects
            .get(name)
            .ok_or_else(|| format!("no object {name}"))?;
        Ok((
            id,
            self.bundle.objects[id]
                .as_ref()
                .expect("named objects exist"),
        ))
    }

    fn parent(&self, id: usize) -> Option<usize> {
        let o = self.bundle.objects[id].as_ref()?;
        gmk::get(&self.bundle.objects, o.parent)?;
        Some(o.parent as usize)
    }

    /// The code `object`'s `event` runs, in order: its own, or its parent's if it has none, with the
    /// parent's spliced in where an `action_inherited` action is.
    fn event_units(&self, id: usize, event: &str, depth: usize) -> Result<Vec<Unit>> {
        let o = self.bundle.objects[id].as_ref().expect("object ids exist");
        if depth > self.bundle.objects.len() {
            return Err(format!("{}: parent cycle", o.name).into());
        }
        let files = self.event_files[id].as_ref().expect("object ids exist");
        let found = o
            .events
            .iter()
            .zip(files)
            .find(|(e, _)| gml_tree::event_name(&self.bundle.objects, e) == event);
        let Some((ev, file)) = found else {
            return match self.parent(id) {
                Some(p) => self.event_units(p, event, depth + 1),
                None => Ok(Vec::new()),
            };
        };
        let lines = gml_tree::code_lines(&ev.actions);
        let mut units = Vec::new();
        // GM8 runs the action after a question only if it said yes, and the one after an `else` only if
        // it said no; a begin/end group makes the actions between them one action for this.
        let mut groups: Vec<Vec<When>> = Vec::new();
        let mut pending: Option<When> = None;
        let mut last_question: Option<String> = None;
        // After an exit action, the rest runs only if it didn't.
        let mut exited: Vec<When> = Vec::new();
        for (action, line) in ev.actions.iter().zip(lines) {
            match action.kind {
                ActionKind::BeginGroup => {
                    groups.push(pending.take().into_iter().collect());
                    continue;
                }
                ActionKind::EndGroup => {
                    groups.pop();
                    continue;
                }
                ActionKind::Else => {
                    pending = last_question
                        .take()
                        .map(|q| When::Question(format!("not {q}")));
                    continue;
                }
                _ => {}
            }
            let mut when: Vec<When> = groups.iter().flatten().cloned().collect();
            when.extend(pending.take());
            when.extend(exited.iter().cloned());
            if action.is_question {
                let question = if action.invert {
                    format!("not {}", action.function)
                } else {
                    action.function.clone()
                };
                pending = Some(When::Question(question.clone()));
                last_question = Some(question);
            } else if action.kind == ActionKind::Repeat {
                pending = Some(When::Loop("repeat action".to_owned()));
            } else if action.kind == ActionKind::Exit {
                exited.push(When::NotExited("exit action".to_owned()));
            }
            if action.function == "action_inherited" {
                if let Some(p) = self.parent(id) {
                    for mut unit in self.event_units(p, event, depth + 1)? {
                        unit.when.splice(0..0, when.iter().cloned());
                        units.push(unit);
                    }
                }
            } else if let (Some(code), Some(line)) = (action.code(), line) {
                let program =
                    gml::parse_program(code).map_err(|e| format!("{file}: {}", e.display(code)))?;
                if action.has_applies_to && action.applies_to != -1 {
                    let target = match action.applies_to {
                        -2 => "other".to_owned(),
                        t => gmk::get(&self.bundle.objects, t)
                            .map_or_else(|| t.to_string(), |o| o.name.clone()),
                    };
                    when.push(When::With(target));
                }
                units.push(Unit {
                    code: code.to_owned(),
                    program,
                    file: file.clone(),
                    first_line: line,
                    object: Some(o.name.clone()),
                    event: Some(event.to_owned()),
                    script: None,
                    when,
                });
            }
        }
        Ok(units)
    }

    fn script_unit(&self, name: &str) -> Result<Unit> {
        let id = *self
            .scripts
            .get(name)
            .ok_or_else(|| format!("no script {name}"))?;
        let script = self.bundle.scripts[id]
            .as_ref()
            .expect("named scripts exist");
        let file = self.script_files[id].clone().expect("named scripts exist");
        let program = gml::parse_program(&script.code)
            .map_err(|e| format!("{file}: {}", e.display(&script.code)))?;
        Ok(Unit {
            code: script.code.clone(),
            program,
            file,
            first_line: 1,
            object: None,
            event: None,
            script: Some(name.to_owned()),
            when: Vec::new(),
        })
    }

    /// The code a place runs: a script, or an `object:event` (with inheritance).
    fn place_units(&self, place: &str) -> Result<Vec<Unit>> {
        match place.split_once(':') {
            Some((object, event)) => self.event_units(self.object(object)?.0, event, 0),
            None => Ok(vec![self.script_unit(place)?]),
        }
    }

    /// Every object's own events, as (object, event) names, by object name.
    fn own_events(&self) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for &id in self.objects.values() {
            let o = self.bundle.objects[id]
                .as_ref()
                .expect("named objects exist");
            for e in &o.events {
                out.push((
                    o.name.clone(),
                    gml_tree::event_name(&self.bundle.objects, e),
                ));
            }
        }
        out
    }

    /// For each object's own event (`object:event`), the objects whose event runs its code: it, and the
    /// descendants that inherit the event or call it with `action_inherited`.
    fn runs_as(&self) -> Result<BTreeMap<String, Vec<String>>> {
        let mut runs: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (&name, &id) in &self.objects {
            // The events it has, its own and its ancestors'.
            let mut events = BTreeSet::new();
            let mut ancestor = Some(id);
            let mut depth = 0;
            while let Some(a) = ancestor
                && depth <= self.bundle.objects.len()
            {
                let o = self.bundle.objects[a].as_ref().expect("object ids exist");
                events.extend(
                    o.events
                        .iter()
                        .map(|e| gml_tree::event_name(&self.bundle.objects, e)),
                );
                ancestor = self.parent(a);
                depth += 1;
            }
            for event in events {
                for unit in self.event_units(id, &event, 0)? {
                    if let Some(owner) = unit.object {
                        runs.entry(format!("{owner}:{event}"))
                            .or_default()
                            .insert(name.to_owned());
                    }
                }
            }
        }
        Ok(runs
            .into_iter()
            .map(|(k, v)| (k, v.into_iter().collect()))
            .collect())
    }

    /// The units a stat searches, and a description of where.
    fn units(&self, spec: &StatSpec, entity_object: Option<&str>) -> Result<(Vec<Unit>, String)> {
        if let Some(script) = &spec.script {
            if spec.object.is_some() || spec.event.is_some() {
                return Err("a stat looks in a script or an object's event, not both".into());
            }
            return Ok((vec![self.script_unit(script)?], script.clone()));
        }
        let object = spec
            .object
            .as_deref()
            .or(entity_object)
            .ok_or("no object to look in")?;
        let event = spec.event.as_deref().unwrap_or("create");
        let (id, _) = self.object(object)?;
        let units = self.event_units(id, event, 0)?;
        if units.is_empty() {
            return Err(format!("{object} has no {event} event (nor does a parent)").into());
        }
        Ok((units, format!("{object}:{event}")))
    }

    /// Every match of a stat's pattern, as facts, and where it looked.
    fn find(&self, spec: &StatSpec, entity_object: Option<&str>) -> Result<(Found, String)> {
        let pattern = Pattern::parse(&spec.find)?;
        let (units, place) = self.units(spec, entity_object)?;
        let mut found = Found {
            facts: Vec::new(),
            evals: Vec::new(),
        };
        for unit in &units {
            let lines = LineIndex::new(&unit.code);
            for m in find::find(&pattern, &unit.program) {
                found.facts.push(fact(unit, &lines, &m));
                found.evals.push(Evaluable {
                    assign: m.stmt.and_then(|s| match &s.kind {
                        StmtKind::Assign { place, op, .. } => Some((gml::print::expr(place), *op)),
                        _ => None,
                    }),
                    expr: match m.stmt {
                        Some(_) => m.expr,
                        None => m.holes.first().copied().unwrap_or(m.expr),
                    }
                    .span
                    .text(&unit.code)
                    .to_owned(),
                });
            }
        }
        // In a create event, a later plain assignment replaces an earlier one: the game never sees it.
        if spec.event.as_deref().unwrap_or("create") == "create"
            && spec.script.is_none()
            && pattern.sets().is_some()
            && let Some(last) = found.facts.iter().rposition(|f| f.when.is_empty())
        {
            found.facts.drain(..last);
            found.evals.drain(..last);
        }
        Ok((found, place))
    }

    /// What an object's create event sets at the top level (see [`ObjectInfo::init`]).
    fn init(&self, id: usize) -> Result<BTreeMap<String, Init>> {
        let mut init: BTreeMap<String, Init> = BTreeMap::new();
        for unit in self.event_units(id, "create", 0)? {
            if !unit.when.is_empty() {
                continue;
            }
            let lines = LineIndex::new(&unit.code);
            let statements = find::statements(&unit.program);
            // Locals (`var`) and loop counters aren't the instance's variables.
            let mut locals = BTreeSet::new();
            let mut loop_inits = BTreeSet::new();
            for (stmt, _) in &statements {
                match &stmt.kind {
                    StmtKind::Var(names) => locals.extend(names.iter().map(|n| n.name.clone())),
                    StmtKind::For { init, .. } => {
                        loop_inits.insert((init.span.start, init.span.end));
                    }
                    _ => {}
                }
            }
            for (stmt, when) in statements {
                let StmtKind::Assign { place, op, value } = &stmt.kind else {
                    continue;
                };
                let name = gml::print::expr(place);
                // In a `with`, the code sets another instance's variables.
                if locals.contains(&name)
                    || loop_inits.contains(&(stmt.span.start, stmt.span.end))
                    || when.iter().any(|w| matches!(w, When::With(_)))
                {
                    continue;
                }
                if !when.is_empty() || *op != AssignOp::Set {
                    if let Some(entry) = init.get_mut(&name) {
                        entry.changed_later = true;
                    }
                    continue;
                }
                let m = Match {
                    stmt: Some(stmt),
                    expr: value,
                    start: stmt.span.start,
                    end: stmt.span.end,
                    holes: vec![value],
                    when,
                };
                let f = fact(&unit, &lines, &m);
                init.insert(
                    name,
                    Init {
                        expr: f.expr,
                        value: f.value,
                        range: f.range,
                        file: f.at.file,
                        line: f.at.line,
                        changed_later: false,
                    },
                );
            }
        }
        Ok(init)
    }
}

/// A match as a fact.
fn fact(unit: &Unit, lines: &LineIndex, m: &Match<'_>) -> Fact {
    let value_expr = m.holes.first().copied().unwrap_or(m.expr);
    let text = &unit.code[m.start as usize..m.end as usize];
    let compound = m
        .stmt
        .is_some_and(|s| !matches!(s.kind, StmtKind::Assign { .. }));
    let source = if compound {
        text.lines().next().unwrap_or_default().trim().to_owned()
    } else {
        text.replace("\r\n", "\n").trim().to_owned()
    };
    let mut when = unit.when.clone();
    when.extend(m.when.iter().cloned());
    Fact {
        expr: gml::print::expr(value_expr),
        holes: if m.holes.len() > 1 {
            m.holes.iter().map(|h| gml::print::expr(h)).collect()
        } else {
            Vec::new()
        },
        value: eval::eval(value_expr, &unit.code, &Env::new()).map(|v| json_value(&v)),
        range: eval::range(value_expr, &unit.code),
        when,
        at: At {
            file: unit.file.clone(),
            line: unit.first_line + lines.line_col(m.start).line as usize - 1,
            object: unit.object.clone(),
            event: unit.event.clone(),
            script: unit.script.clone(),
            source,
        },
    }
}

// ---------------------------------------------------------------------------------------------------------
// Building

fn build(game: &Game<'_>, specs: &Specs) -> Result<Output> {
    let mut entities = Vec::new();
    let mut wanted: BTreeSet<usize> = BTreeSet::new();
    for spec in &specs.entities {
        let entity =
            build_entity(game, specs, spec).map_err(|e| format!("entity {}: {e}", spec.slug))?;
        let stat_objects = spec
            .templates
            .iter()
            .flat_map(|t| &specs.templates[t])
            .chain(&spec.stat)
            .filter_map(|s| s.object.as_ref());
        for object in spec.object.iter().chain(stat_objects) {
            wanted.insert(game.object(object)?.0);
        }
        entities.push(entity);
    }

    let roots: Vec<usize> = ROOTS
        .iter()
        .map(|r| game.object(r).map(|(id, _)| id))
        .collect::<Result<_>>()?;
    for id in 0..game.bundle.objects.len() {
        let mut ancestor = game.parent(id);
        let mut depth = 0;
        while let Some(a) = ancestor
            && depth <= game.bundle.objects.len()
        {
            if roots.contains(&a) {
                wanted.insert(id);
                break;
            }
            ancestor = game.parent(a);
            depth += 1;
        }
    }
    let mut objects = BTreeMap::new();
    for id in wanted {
        let o = game.bundle.objects[id].as_ref().expect("object ids exist");
        let init = game.init(id)?;
        let type_name = init
            .get("type")
            .and_then(|i| i.value.as_ref()?.as_str().map(str::to_owned))
            .filter(|t| !t.is_empty() && t != "None");
        objects.insert(
            o.name.clone(),
            ObjectInfo {
                parent: game.parent(id).map(|p| {
                    game.bundle.objects[p]
                        .as_ref()
                        .expect("parent exists")
                        .name
                        .clone()
                }),
                type_name,
                init,
            },
        );
    }

    Ok(Output {
        format: FORMAT,
        notice: NOTICE,
        license: LICENSE_URL,
        room_speed: tools::room_speed(game.bundle),
        entities,
        objects,
    })
}

fn build_entity(game: &Game<'_>, specs: &Specs, spec: &EntitySpec) -> Result<Entity> {
    let stat_specs = spec
        .templates
        .iter()
        .flat_map(|t| &specs.templates[t])
        .chain(&spec.stat);
    let mut stats: Vec<Stat> = Vec::new();
    for s in stat_specs {
        if stats.iter().any(|st| st.id == s.id) {
            return Err(format!("stat {} is given twice", s.id).into());
        }
        let stat = build_stat(game, spec, s, &stats).map_err(|e| format!("stat {}: {e}", s.id))?;
        stats.push(stat);
    }
    Ok(Entity {
        slug: spec.slug.clone(),
        name: spec.name.clone(),
        kind: spec.kind.clone(),
        object: spec.object.clone(),
        stats,
    })
}

fn build_stat(
    game: &Game<'_>,
    entity: &EntitySpec,
    spec: &StatSpec,
    earlier: &[Stat],
) -> Result<Stat> {
    let (mut found, place) = game.find(spec, entity.object.as_deref())?;
    let scope = if spec.where_.is_empty() {
        place.clone()
    } else {
        format!("{place} where {}", spec.where_.join(", "))
    };
    if !spec.where_.is_empty() {
        let holds = |f: &Fact| {
            spec.where_.iter().all(|cond| {
                f.when.iter().any(|w| {
                    matches!(w, When::If(_) | When::Case(_)) && w.text().contains(cond.as_str())
                })
            })
        };
        (found.facts, found.evals) = std::mem::take(&mut found.facts)
            .into_iter()
            .zip(std::mem::take(&mut found.evals))
            .filter(|(f, _)| holds(f))
            .unzip();
    }
    if found.facts.is_empty() {
        return Err(format!("{:?} matches nothing in {scope}", spec.find).into());
    }
    if let Some(n) = spec.matches
        && n != found.facts.len()
    {
        return Err(format!(
            "{:?} matches {} times in {scope}, not {n}",
            spec.find,
            found.facts.len()
        )
        .into());
    }
    if let Some(n) = spec.nth {
        if n >= found.facts.len() {
            return Err(format!(
                "no match {n}: {:?} matches {} times",
                spec.find,
                found.facts.len()
            )
            .into());
        }
        found.facts = vec![found.facts.swap_remove(n)];
        found.evals = vec![found.evals.swap_remove(n)];
    }

    let (mut value, mut min, mut max) = (None, None, None);
    let first = found.facts[0].value.clone();
    if first.is_some() && found.facts.iter().all(|f| f.value == first) {
        value = first;
    } else {
        let bounds: Option<Vec<(f64, f64)>> = found
            .facts
            .iter()
            .map(|f| match (&f.value, &f.range) {
                (Some(v), _) => v.as_f64().map(|v| (v, v)),
                (None, Some(r)) => Some((r.min, r.max)),
                (None, None) => None,
            })
            .collect();
        if let Some(b) = bounds {
            min = b.iter().map(|b| b.0).reduce(f64::min).map(number);
            max = b.iter().map(|b| b.1).reduce(f64::max).map(number);
        }
    }

    let table = spec
        .table
        .as_ref()
        .map(|t| table(t, &found, earlier))
        .transpose()?;
    Ok(Stat {
        id: spec.id.clone(),
        label: spec.label.clone(),
        unit: spec.unit.clone(),
        note: spec.note.clone(),
        find: spec.find.clone(),
        place,
        where_: spec.where_.clone(),
        nth: spec.nth,
        value,
        min,
        max,
        facts: found.facts,
        table,
    })
}

/// Evaluates a stat's one fact for each value of the table's variable.
fn table(spec: &TableSpec, found: &Found, earlier: &[Stat]) -> Result<Table> {
    let ([fact], [ev]) = (found.facts.as_slice(), found.evals.as_slice()) else {
        return Err(format!(
            "a table needs exactly one match, not {} (use nth)",
            found.facts.len()
        )
        .into());
    };
    if spec.from > spec.to || spec.to - spec.from > 1000 {
        return Err(format!("bad table range {}..{}", spec.from, spec.to).into());
    }
    let mut env = Env::new();
    for (name, v) in &spec.bind {
        let v = match v {
            toml::Value::Integer(i) => *i as f64,
            toml::Value::Float(f) => *f,
            toml::Value::String(id) => earlier
                .iter()
                .find(|s| &s.id == id)
                .ok_or_else(|| format!("bind {name}: no earlier stat {id}"))?
                .value
                .as_ref()
                .and_then(Json::as_f64)
                .ok_or_else(|| format!("bind {name}: stat {id} has no single number"))?,
            other => {
                return Err(
                    format!("bind {name}: {other:?} is neither a number nor a stat id").into(),
                );
            }
        };
        env.insert(name.clone(), Value::Real(v));
    }
    // Each condition, parsed: `Some((negated, expr, source))` for an `if` or an `else`, which a table can
    // evaluate.
    let conditions: Vec<Option<(bool, gml::Expr, &str)>> = fact
        .when
        .iter()
        .map(|w| {
            let (negated, text) = match w {
                When::If(c) => (false, c.as_str()),
                When::Not(c) => (true, c.as_str()),
                _ => return Ok(None),
            };
            let expr =
                gml::parse_expression(text).map_err(|e| format!("condition {text:?}: {e}"))?;
            Ok(Some((negated, expr, text)))
        })
        .collect::<Result<_>>()?;
    let expr = gml::parse_expression(&ev.expr).map_err(|e| format!("re-parsing the match: {e}"))?;
    let mut rows = Vec::new();
    let mut assumed: Vec<When> = Vec::new();
    for (i, at) in (spec.from..=spec.to).enumerate() {
        env.insert(spec.var.clone(), Value::Real(at as f64));
        let mut runs = true;
        for (w, condition) in fact.when.iter().zip(&conditions) {
            let holds = condition.as_ref().and_then(|(negated, expr, text)| {
                let v = eval::eval(expr, text, &env)?.real()?;
                Some((v >= 0.5) != *negated)
            });
            match holds {
                Some(h) => runs &= h,
                None if !assumed.contains(w) => assumed.push(w.clone()),
                None => {}
            }
        }
        let new = || -> Result<f64> {
            eval::eval(&expr, &ev.expr, &env)
                .and_then(|v| v.real())
                .ok_or_else(|| {
                    format!("can't evaluate {:?} at {} = {at}", fact.expr, spec.var).into()
                })
        };
        let value = match &ev.assign {
            Some((place, op)) => {
                let old = || {
                    env.get(place)
                        .and_then(Value::real)
                        .ok_or_else(|| format!("bind {place}: the table needs its value before"))
                };
                if !runs {
                    old()?
                } else {
                    match op {
                        AssignOp::Set => new()?,
                        AssignOp::Add => old()? + new()?,
                        AssignOp::Sub => old()? - new()?,
                        AssignOp::Mul => old()? * new()?,
                        AssignOp::Div => old()? / new()?,
                        _ => {
                            return Err(
                                format!("can't tabulate a {} assignment", op.as_str()).into()
                            );
                        }
                    }
                }
            }
            None if runs => new()?,
            None => return Err(format!("the match doesn't run at {} = {at}", spec.var).into()),
        };
        rows.push(Row {
            at,
            label: spec.labels.as_ref().and_then(|l| l.get(i).cloned()),
            value: number(value),
        });
    }
    Ok(Table {
        var: spec.var.clone(),
        rows,
        assumed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments_are_checked() {
        assert!(run(["--assets".to_owned(), "x".to_owned()]).is_err());
        assert!(run(["--bogus".to_owned()]).is_err());
        let both = ["--assets", "a", "--specs", "s", "--out", "o", "--find", "x"];
        assert!(run(both.map(str::to_owned)).is_err());
    }

    #[test]
    fn the_same_file_spelled_two_ways_is_the_same() {
        let p = |s: &str| Path::new(s).to_owned();
        assert!(same_file(&p("target/x/../w/a.json"), &p("./target/w/a.json")).unwrap());
        assert!(!same_file(&p("target/w/a.json"), &p("target/w/b.json")).unwrap());
    }

    #[test]
    fn whole_numbers_print_without_a_fraction() {
        assert_eq!(number(15000.0).to_string(), "15000");
        assert_eq!(number(-2.5).to_string(), "-2.5");
        assert_eq!(number(0.6).to_string(), "0.6");
    }
}
