//! Relations between the game's things, read from its code: where level generation makes each monster,
//! trap and item (and in which areas and special levels), what each shop sells, what an object leaves
//! behind (a crate's or a jar's contents, a monster's drops), and what hurts what. Like a stat, each
//! comes with the code it's read from and the conditions around it (`docs/wiki-data.md`, "Relations").
//!
//! Level generation is followed from its start (`scrInitLevel`) through the scripts it calls, with their
//! arguments when they're constants (`scrGenerateItem(x, y, 2)` runs only its `argument2 == 2` branch),
//! and into the create events of what it makes (a Mines brick may hold a gem). A walk is made once per
//! area with the area's variable bound (`global.levelType = 0`), so code under another area's `if` is
//! cut off. A special level is a flag (`global.darkLevel`) and, for one built from its own rooms, the
//! room scripts' conditions that pick them (`roomPath == 8`, the snake pit): a room script places
//! things by its room strings' characters (`tile == "s"`), so where a character can come from is read
//! off the strings (the rooms and obstacles) that hold it, and their conditions.

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use gml::ast::{AssignOp, BinOp, Const, ExprKind, StmtKind, UnOp};
use gml::{Expr, LineIndex};
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

use crate::eval::{self, Env, Value};
use crate::find::{self, Pattern, When};
use crate::{At, Game, LICENSE_URL, NOTICE, Result, fact, json_value};

/// Bumped when relations.json's shape changes.
const FORMAT: u32 = 2;

/// Bumped when the site's snapshot's shape changes (`Relations::site_snapshot`), which the site checks.
const SITE_FORMAT: u32 = 1;

/// How deep a walk goes (scripts and create events within each other), against runaway recursion.
const MAX_DEPTH: usize = 12;

/// How many obstacle strings within strings a room character is traced through.
const MAX_TILE_DEPTH: usize = 3;

/// The functions whose object argument a condition tests touching (or being at).
const COLLISION_FUNCTIONS: [&str; 9] = [
    "collision_point",
    "collision_rectangle",
    "collision_line",
    "collision_circle",
    "collision_ellipse",
    "place_meeting",
    "instance_place",
    "position_meeting",
    "instance_position",
];

/// The player's object, which `global.plife` is the life of.
const PLAYER: &str = "oPlayer1";

// ---------------------------------------------------------------------------------------------------------
// Specs

/// The relation specs, from every spec file (`[generation]`, `[[area]]`, `[[level]]`, `[[shop]]`,
/// `[drops]` and `[[damage]]`).
#[derive(Debug, Default)]
pub struct RelationSpecs {
    pub generation: Option<GenerationSpec>,
    pub areas: Vec<AreaSpec>,
    pub levels: Vec<LevelSpec>,
    pub shops: Vec<ShopSpec>,
    pub drops: Option<DropsSpec>,
    pub damage: Vec<DamageSpec>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DropsSpec {
    /// Scripts whose `instance_create`s aren't drops of the code that reaches them (scrHoldItem puts the
    /// item the player had picked up back in his hand), with why, for a reviewer.
    pub not_via: Vec<NotViaSpec>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NotViaSpec {
    pub script: String,
    pub why: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationSpec {
    /// Where level generation starts: scripts, or `object:event`s (the ghost comes from oGame's step).
    pub roots: Vec<String>,
    /// The variable a room script keeps the room string's character in (`tile`).
    pub tile: String,
    /// The variables its room and obstacle strings are assigned to (`strTemp`, `strObs1`...).
    pub templates: Vec<String>,
    /// Rooms a room script has code for that the level's room path never asks for: what's made only
    /// there isn't made at all. The walk doesn't follow `global.roomPath`'s values, so the spec says.
    #[serde(default)]
    pub unreachable: Vec<UnreachableSpec>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UnreachableSpec {
    pub script: String,
    #[serde(rename = "where")]
    pub where_: String,
    /// Why the room path never asks for it, for a reviewer.
    pub why: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AreaSpec {
    pub slug: String,
    pub name: String,
    /// The variables that are the area's (`global.levelType = 0`).
    pub bind: BTreeMap<String, toml::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LevelSpec {
    pub slug: String,
    pub name: String,
    /// The global that's true on this kind of level and false otherwise (`global.darkLevel`).
    pub flag: Option<String>,
    /// The rooms that make it, in the room scripts that make them.
    #[serde(default)]
    pub rooms: Vec<RoomSpec>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RoomSpec {
    pub script: String,
    /// The text of the `if` or `case` condition that picks the rooms (`roomPath == 8`).
    #[serde(rename = "where")]
    pub where_: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShopSpec {
    pub slug: String,
    pub name: String,
    /// The script (or `object:event`) that stocks it.
    #[serde(rename = "in")]
    pub place: String,
    /// The variables that pick the shop there (`shopType = "Weapon"`).
    #[serde(default)]
    pub bind: BTreeMap<String, toml::Value>,
    /// Keep only what's made under an `if` or `case` whose GML contains each of these (`tile == "q"`).
    #[serde(rename = "where", default)]
    pub where_: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DamageSpec {
    /// A pattern (as a stat's) for a statement that hurts something.
    pub find: String,
    /// Who it hurts: `player` (`global.plife`), `self` (the instance running it, or the `with` target),
    /// or `hole` (the instance the pattern's first `?` names: `other.hp -= ?`).
    pub target: String,
}

const TARGETS: [&str; 3] = ["player", "self", "hole"];

impl RelationSpecs {
    pub fn check(&self) -> Result<()> {
        let mut slugs = BTreeSet::new();
        let all = (self.areas.iter().map(|a| &a.slug))
            .chain(self.levels.iter().map(|l| &l.slug))
            .chain(self.shops.iter().map(|s| &s.slug));
        for slug in all {
            if !slugs.insert(slug) {
                return Err(format!("area, level or shop {slug} is defined twice").into());
            }
        }
        for d in &self.damage {
            if !TARGETS.contains(&d.target.as_str()) {
                return Err(format!(
                    "damage {:?}: unknown target {:?} (one of {TARGETS:?})",
                    d.find, d.target
                )
                .into());
            }
            Pattern::parse(&d.find)?;
        }
        if !self.areas.is_empty() && self.generation.is_none() {
            return Err("areas need a [generation] section".into());
        }
        Ok(())
    }
}

fn bind(values: &BTreeMap<String, toml::Value>) -> Result<Env> {
    values
        .iter()
        .map(|(name, v)| {
            let v = match v {
                toml::Value::Integer(i) => Value::Real(*i as f64),
                toml::Value::Float(f) => Value::Real(*f),
                toml::Value::Boolean(b) => Value::Real(f64::from(u8::from(*b))),
                toml::Value::String(s) => Value::Str(s.clone()),
                other => return Err(format!("bind {name}: {other:?} isn't a number or a string")),
            };
            Ok((name.clone(), v))
        })
        .collect::<std::result::Result<_, _>>()
        .map_err(Into::into)
}

fn bind_json(env: &Env) -> BTreeMap<String, Json> {
    env.iter()
        .map(|(k, v)| (k.clone(), json_value(v)))
        .collect()
}

// ---------------------------------------------------------------------------------------------------------
// Output

#[derive(Serialize)]
pub struct Relations {
    format: u32,
    notice: &'static str,
    license: &'static str,
    areas: Vec<AreaOut>,
    levels: Vec<LevelOut>,
    /// The rooms the spec says generation never makes, and why.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    unreachable: Vec<UnreachableSpec>,
    /// The scripts the spec says make no drops, and why.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    not_drops: Vec<NotViaSpec>,
    /// The wiki slug of each object a stats spec is about, to link a relation's objects to pages.
    slugs: BTreeMap<String, String>,
    /// Per object: each place level generation makes it.
    spawns_in: BTreeMap<String, Vec<Spawn>>,
    /// Per object: the rooms the game's room editor placed it in (Olmec in rOlmec, the tutorial's, the
    /// title screen's), by room name, with how many.
    placed_in: BTreeMap<String, BTreeMap<String, usize>>,
    shops: Vec<Shop>,
    /// Per object: the shops that stock it.
    sold_in: BTreeMap<String, Vec<String>>,
    /// Per object: what it leaves behind or holds (see [`Drop`]).
    drops: BTreeMap<String, Vec<Drop>>,
    damage: Vec<Damage>,
}

impl Relations {
    /// The site's snapshot of the relations (`docs/wiki-data.md`, "The site's snapshot"): what the wiki's
    /// build checks a page's relations against, by object and without the code, with inheritance
    /// followed here (`parents` is every object's parent, by name), so the site needn't.
    pub fn site_snapshot(&self, parents: &BTreeMap<String, String>) -> Json {
        let mut places = serde_json::Map::new();
        for (slug, name, kind) in (self.areas.iter().map(|a| (&a.slug, &a.name, "area")))
            .chain(self.levels.iter().map(|l| (&l.slug, &l.name, "level")))
        {
            places.insert(
                slug.clone(),
                serde_json::json!({ "name": name, "kind": kind }),
            );
        }
        let shops: serde_json::Map<String, Json> = (self.shops.iter())
            .map(|s| (s.slug.clone(), s.name.clone().into()))
            .collect();
        // Where generation makes it (its areas, and the special levels it's only on) and the rooms the
        // room editor placed it in.
        let mut found_in: BTreeMap<&str, (BTreeSet<&str>, BTreeSet<&str>)> = BTreeMap::new();
        for (object, spawns) in &self.spawns_in {
            let (places, _) = found_in.entry(object).or_default();
            for s in spawns {
                places.extend(s.areas.iter().chain(&s.only_in).map(String::as_str));
            }
        }
        for (object, rooms) in &self.placed_in {
            found_in
                .entry(object)
                .or_default()
                .1
                .extend(rooms.keys().map(String::as_str));
        }
        // An object and its descendants: what a collision with it, or a test for touching it, also
        // matches.
        let mut children: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for (child, parent) in parents {
            children.entry(parent).or_default().push(child);
        }
        let family = |object: &str| {
            let mut out = BTreeSet::from([object.to_owned()]);
            let mut todo = vec![object.to_owned()];
            while let Some(o) = todo.pop() {
                for &c in children.get(o.as_str()).into_iter().flatten() {
                    if out.insert(c.to_owned()) {
                        todo.push(c.to_owned());
                    }
                }
            }
            out
        };
        // The objects that are `object` where code (`by`, run by `runs_as`) names it: when it's the code's
        // own object, those that run the code (a descendant that overrides the event doesn't), and
        // otherwise it and its descendants.
        let concrete = |object: &str, by: &str, runs_as: &[String]| -> BTreeSet<String> {
            let own = by.split(':').next() == Some(object);
            if own && !runs_as.is_empty() {
                let f = family(object);
                runs_as.iter().filter(|o| f.contains(*o)).cloned().collect()
            } else {
                family(object)
            }
        };
        // Per object, what the data says of the object itself (its own drops, what hurts it by name) and
        // what it inherits or matches as a descendant (oEnemy's code, which the Snake runs; a collision
        // with oEnemy). The wiki checks a relation against both and warns about the first only.
        type Split = BTreeMap<String, [BTreeSet<String>; 2]>;
        let add = |map: &mut Split, object: &str, own: bool, values: &BTreeSet<String>| {
            map.entry(object.to_owned()).or_default()[usize::from(!own)]
                .extend(values.iter().cloned());
        };
        let mut drops = Split::new();
        for (key, list) in &self.drops {
            for d in list {
                let made = BTreeSet::from([d.object.clone()]);
                for dropper in concrete(key, &d.by, &d.runs_as) {
                    add(&mut drops, &dropper, dropper == *key, &made);
                }
            }
        }
        // What hurts a victim by name is the sources the code names (and, for the code's own object, those
        // that run it); their descendants only match (a test for touching oSolid is one for touching
        // Olmec, a solid, which never meets most of the monsters that test it).
        let mut damaged_by = Split::new();
        for d in &self.damage {
            let named: BTreeSet<String> = (d.sources.iter())
                .flat_map(|s| {
                    let own = d.by.split(':').next() == Some(s.as_str());
                    if own {
                        concrete(s, &d.by, &d.runs_as)
                    } else {
                        BTreeSet::from([s.clone()])
                    }
                })
                .collect();
            // Less the descendants a test of the collision's other rules out (a mantrap's `else` of
            // `if (other.type == "Machete")` names the whip's wind-up, oWhipPre, but not the machete's below
            // it).
            let sources: BTreeSet<String> = (d.sources.iter())
                .flat_map(|s| {
                    let mut all = concrete(s, &d.by, &d.runs_as);
                    all.retain(|o| o == s || !d.ruled_out.contains(o));
                    all
                })
                .collect();
            for victim in &d.victims {
                let victims = if d.target == "self" {
                    concrete(victim, &d.by, &d.runs_as)
                } else {
                    family(victim)
                };
                for v in victims {
                    if v == *victim {
                        add(&mut damaged_by, &v, true, &named);
                    }
                    add(&mut damaged_by, &v, false, &sources);
                }
            }
        }
        // Each as `{ own, inherited }`, the second without what the first has.
        let split = |map: Split| -> BTreeMap<String, Json> {
            map.into_iter()
                .map(|(o, [own, all])| {
                    let inherited: Vec<_> = all.difference(&own).collect();
                    (o, serde_json::json!({ "own": own, "inherited": inherited }))
                })
                .collect()
        };
        serde_json::json!({
            "format": SITE_FORMAT,
            "notice": self.notice,
            "license": self.license,
            "places": places,
            "shops": shops,
            "found_in": found_in
                .into_iter()
                .map(|(o, (places, rooms))| (o, serde_json::json!({ "places": places, "rooms": rooms })))
                .collect::<BTreeMap<_, _>>(),
            "sold_in": self.sold_in,
            "drops": split(drops),
            "damaged_by": split(damaged_by),
        })
    }
}

#[derive(Serialize)]
struct AreaOut {
    slug: String,
    name: String,
    bind: BTreeMap<String, Json>,
}

#[derive(Serialize)]
struct LevelOut {
    slug: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    flag: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    rooms: Vec<RoomSpec>,
}

/// One place level generation makes an object.
#[derive(Serialize)]
struct Spawn {
    /// The areas (by slug) where generation reaches it.
    areas: Vec<String>,
    /// The special levels it only happens on, and those it never happens on.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    only_in: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    not_in: Vec<String>,
    /// The room string character that places it, in a room script.
    #[serde(skip_serializing_if = "Option::is_none")]
    tile: Option<String>,
    /// The conditions around the `instance_create` there, outermost first.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    when: Vec<When>,
    at: At,
    /// How generation gets there: each chain of places (scripts and `object:event`s), outermost first.
    via: Vec<String>,
}

#[derive(Serialize)]
struct Shop {
    slug: String,
    name: String,
    #[serde(rename = "in")]
    place: String,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    bind: BTreeMap<String, Json>,
    #[serde(rename = "where", skip_serializing_if = "Vec::is_empty")]
    where_: Vec<String>,
    sells: Vec<Sale>,
}

#[derive(Serialize)]
struct Sale {
    object: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    slug: Option<String>,
    /// Its base price (its create event's `cost`), which stats.json's `objects` cites.
    #[serde(skip_serializing_if = "Option::is_none")]
    price: Option<Json>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    when: Vec<When>,
    at: At,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    via: Vec<String>,
}

/// Something an object's code makes during play (not its create event): a crate's contents, a jar's,
/// a monster's remains. Keyed by the things the code's conditions test touching (the player's step
/// opens the crate it's touching), or else by the object whose code it is.
#[derive(Serialize)]
struct Drop {
    object: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    slug: Option<String>,
    /// The `object:event` whose code makes it, and the objects that run that code (it and the
    /// descendants that inherit the event).
    by: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    runs_as: Vec<String>,
    /// The other object of a collision event.
    #[serde(skip_serializing_if = "Option::is_none")]
    trigger: Option<String>,
    /// The objects the conditions test touching.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    involves: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    when: Vec<When>,
    at: At,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    via: Vec<String>,
}

/// A statement that hurts something.
#[derive(Serialize)]
struct Damage {
    /// The `object:event` (or script) it's in, and the objects that run it.
    by: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    runs_as: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    trigger: Option<String>,
    /// The damage spec's pattern, and who it hurts as the code says it (`player`, `self`, `other`, a
    /// `with` target, a variable), resolved to objects where the code says which.
    find: String,
    target: String,
    victims: Vec<String>,
    /// What does the hurting: the other object of a collision event (or those of it and its descendants
    /// its tests of the other's variables allow: `if (other.puncture)`) or the code's own, and the
    /// objects the conditions test touching (lava, spikes, an arrow).
    sources: Vec<String>,
    /// The other object and descendants those tests rule out, which a kept ancestor's don't include.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    ruled_out: Vec<String>,
    /// The amount: what the pattern's last `?` matched (`hp = 0` has none), its value or range.
    #[serde(skip_serializing_if = "Option::is_none")]
    amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<Json>,
    #[serde(skip_serializing_if = "Option::is_none")]
    range: Option<eval::Range>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    when: Vec<When>,
    at: At,
}

// ---------------------------------------------------------------------------------------------------------
// The code, as calls

/// A call in a place's code, with what following it needs.
struct Call {
    name: String,
    args: Vec<Expr>,
    /// The unit's code, which the arguments' spans are in.
    code: Rc<str>,
    when: Vec<When>,
    at: At,
    use_at: UseAt,
}

/// Where in a place's code something is, for finding which assignment a variable it reads has: the
/// unit (by index), the offset in it, and where the unit's own conditions start in a `when`.
#[derive(Clone, Copy)]
struct UseAt {
    unit: usize,
    start: u32,
    local: usize,
}

/// A room or obstacle string a room script assigns, and the conditions it's assigned under.
struct Template {
    text: String,
    when: Vec<When>,
    use_at: UseAt,
}

/// An assignment to a plain variable, as far as a variable's objects and values go.
struct Assign {
    var: String,
    start: u32,
    /// Its conditions in its unit (not the unit's own).
    when: Vec<When>,
    /// The value a plain `=` sets; `None` for one that changes the old value (`n += 1`).
    value: Option<Expr>,
    /// The objects of an instance found by object (`instance_nearest(x, y, oArrow)`,
    /// `instance_place(x, y, oCrate)`), of a collision test, or of the tests a `true` is set under
    /// (`if (collision_rectangle(..., oSpikes, ...)) colSpikes = true`); empty for anything else.
    objects: BTreeSet<String>,
}

/// A place's code as a walk needs it.
struct PlaceCode {
    units: Vec<crate::Unit>,
    calls: Vec<Call>,
    templates: Vec<Template>,
    /// Each unit's assignments, in source order.
    assigns: Vec<Vec<Assign>>,
    /// Each unit's `if` statements: their conditions in the unit, and what they test.
    ifs: Vec<Vec<(Vec<When>, String)>>,
}

/// One step of a walk: a call (or the `instance_create` that ends it) in a place.
#[derive(Clone)]
struct Step {
    place: String,
    when: Vec<When>,
    at: At,
    tile: Option<char>,
    use_at: UseAt,
}

/// The objects a variable holds, where a condition reads it (see [`resolve`]).
type Vars<'v> = dyn Fn(&str) -> BTreeSet<String> + 'v;

/// The functions that find an instance by object: what a variable assigned from one holds.
const FINDERS: [&str; 3] = ["instance_nearest", "instance_furthest", "instance_find"];

/// The objects a variable read at `at` (with `when`, its conditions in the unit) holds, as far as the
/// assignments before it say: the last one whose conditions enclose the read's (so it runs first), and
/// any conditional ones after it (`colSpikes = false; if (...) colSpikes = true;`).
fn resolve(var: &str, assigns: &[Assign], start: u32, when: &[When]) -> BTreeSet<String> {
    let before: Vec<&Assign> = assigns
        .iter()
        .filter(|a| a.var == var && a.start < start && a.value.is_some())
        .collect();
    // Without one, the variable may hold what an earlier step (or event) left in it: unknown.
    let Some(from) = before.iter().rposition(|a| when.starts_with(&a.when)) else {
        return BTreeSet::new();
    };
    let mut out = BTreeSet::new();
    for a in &before[from..] {
        out.extend(a.objects.iter().cloned());
    }
    out
}

/// Whether a condition reads a collision's other's variable (`other.type`, not `another.type`).
fn reads_other(cond: &str) -> bool {
    cond.match_indices("other.").any(|(i, _)| {
        !cond[..i]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
    })
}

/// The objects an instance variable read at `start` (with `when`) was created as, as `created` (a unit's
/// assignments, with an `instance_create`'s object) says: as [`resolve`], but `None` unless each of the
/// assignments that count creates one. In a loop, those after it in the loop count too: they run before it
/// next time round.
fn created_as(
    var: &str,
    created: &[Assign],
    start: u32,
    when: &[When],
) -> Option<BTreeSet<String>> {
    let before: Vec<&Assign> = (created.iter())
        .filter(|a| a.var == var && a.start < start)
        .collect();
    let from = before.iter().rposition(|a| when.starts_with(&a.when))?;
    let looped = when.iter().position(|w| matches!(w, When::Loop(_)));
    let after = created.iter().filter(|a| {
        a.var == var && a.start > start && looped.is_some_and(|l| a.when.starts_with(&when[..=l]))
    });
    let mut out = BTreeSet::new();
    for a in before[from..].iter().copied().chain(after) {
        if a.objects.is_empty() {
            return None;
        }
        out.extend(a.objects.iter().cloned());
    }
    Some(out)
}

/// Per variable, the objects whose instances code other than their create events sets it on (`None`:
/// any), as [`Analyzer::set_elsewhere`] finds them.
type SetElsewhere = BTreeMap<String, Option<BTreeSet<String>>>;

/// A walk's way to an `instance_create`: its steps, the last the creation itself.
struct Reach {
    object: String,
    steps: Vec<Step>,
}

impl Reach {
    fn via(&self) -> String {
        let places: Vec<&str> = self.steps.iter().map(|s| s.place.as_str()).collect();
        places.join(" > ")
    }

    fn last(&self) -> &Step {
        self.steps.last().expect("a reach has a step")
    }
}

/// A special level's side of a scenario: the level (by index) and whether it's that level.
type Side = Option<(usize, bool)>;

struct Analyzer<'g, 'a> {
    game: &'g Game<'a>,
    tile: String,
    templates: Vec<String>,
    /// Each special level's rooms, as (script, condition text).
    rooms: Vec<Vec<(String, String)>>,
    /// The rooms generation never makes, likewise.
    unreachable: Vec<(String, String)>,
    places: BTreeMap<String, Rc<PlaceCode>>,
    spawners: BTreeMap<String, bool>,
    conditions: BTreeMap<String, Option<Expr>>,
    /// tile_ok's results by scenario (an id the caller gives), place, character and whether a room
    /// condition was already met.
    tiles: BTreeMap<(usize, String, char, bool), bool>,
    /// The variables code other than a create event's own sets (see [`Analyzer::set_elsewhere`]).
    set_elsewhere: Option<Rc<SetElsewhere>>,
    /// [`Analyzer::other_env`]'s, by object id.
    other_envs: BTreeMap<usize, Rc<Env>>,
}

impl<'g, 'a> Analyzer<'g, 'a> {
    fn new(game: &'g Game<'a>, generation: Option<&GenerationSpec>, levels: &[LevelSpec]) -> Self {
        Analyzer {
            game,
            tile: generation.map(|g| g.tile.clone()).unwrap_or_default(),
            templates: generation.map(|g| g.templates.clone()).unwrap_or_default(),
            rooms: levels
                .iter()
                .map(|l| {
                    l.rooms
                        .iter()
                        .map(|r| (r.script.clone(), r.where_.clone()))
                        .collect()
                })
                .collect(),
            unreachable: generation
                .iter()
                .flat_map(|g| &g.unreachable)
                .map(|u| (u.script.clone(), u.where_.clone()))
                .collect(),
            places: BTreeMap::new(),
            spawners: BTreeMap::new(),
            conditions: BTreeMap::new(),
            tiles: BTreeMap::new(),
            set_elsewhere: None,
            other_envs: BTreeMap::new(),
        }
    }

    fn place_code(&mut self, place: &str) -> Result<Rc<PlaceCode>> {
        if let Some(code) = self.places.get(place) {
            return Ok(code.clone());
        }
        let units = self.game.place_units(place)?;
        let mut code = PlaceCode {
            units: Vec::new(),
            calls: Vec::new(),
            templates: Vec::new(),
            assigns: Vec::new(),
            ifs: Vec::new(),
        };
        for (i, unit) in units.iter().enumerate() {
            let lines = LineIndex::new(&unit.code);
            let source: Rc<str> = Rc::from(unit.code.as_str());
            for (expr, when) in find::calls(&unit.program) {
                let ExprKind::Call { name, args } = &expr.kind else {
                    continue;
                };
                code.calls.push(Call {
                    name: name.name.clone(),
                    args: args.clone(),
                    code: source.clone(),
                    when: unit.when.iter().cloned().chain(when).collect(),
                    at: at(unit, &lines, expr.span),
                    use_at: UseAt {
                        unit: i,
                        start: expr.span.start,
                        local: unit.when.len(),
                    },
                });
            }
            for (stmt, when) in find::statements(&unit.program) {
                if let StmtKind::Assign {
                    place,
                    op: AssignOp::Set,
                    value,
                } = &stmt.kind
                    && let ExprKind::Ident(var) = &place.kind
                    && let ExprKind::Str(text) = &value.kind
                    && self.templates.contains(var)
                {
                    code.templates.push(Template {
                        text: text.clone(),
                        when: unit.when.iter().cloned().chain(when).collect(),
                        use_at: UseAt {
                            unit: i,
                            start: stmt.span.start,
                            local: unit.when.len(),
                        },
                    });
                }
            }
            let assigns = self.assigns(unit);
            code.assigns.push(assigns);
            code.ifs.push(ifs(&unit.program));
        }
        code.units = units;
        let code = Rc::new(code);
        self.places.insert(place.to_owned(), code.clone());
        Ok(code)
    }

    /// A unit's assignments to plain variables, for [`resolve`].
    fn assigns(&mut self, unit: &crate::Unit) -> Vec<Assign> {
        let mut out = Vec::new();
        for (stmt, when) in find::statements(&unit.program) {
            let StmtKind::Assign { place, op, value } = &stmt.kind else {
                continue;
            };
            let ExprKind::Ident(var) = &place.kind else {
                continue;
            };
            if *op != AssignOp::Set {
                out.push(Assign {
                    var: var.clone(),
                    start: stmt.span.start,
                    when,
                    value: None,
                    objects: BTreeSet::new(),
                });
                continue;
            }
            let mut objects = BTreeSet::new();
            self.collided(value, true, &|_| BTreeSet::new(), &mut objects);
            if let ExprKind::Call { name, args } = &value.kind
                && FINDERS.contains(&name.name.as_str())
            {
                objects.extend(args.iter().filter_map(|a| self.object_name(a)));
            }
            let is_true = matches!(value.kind, ExprKind::Const(Const::True))
                || matches!(value.kind, ExprKind::Real(v) if v == 1.0);
            if is_true {
                objects = self.involves(&when, &|_| BTreeSet::new());
            }
            out.push(Assign {
                var: var.clone(),
                start: stmt.span.start,
                when,
                value: Some(value.clone()),
                objects,
            });
        }
        out
    }

    fn object_name(&self, expr: &Expr) -> Option<String> {
        match &expr.kind {
            ExprKind::Ident(n) if self.game.objects.contains_key(n.as_str()) => Some(n.clone()),
            _ => None,
        }
    }

    /// The object an `instance_create` call makes, if it names one.
    fn created(&self, call: &Call) -> Option<String> {
        match (call.name.as_str(), call.args.as_slice()) {
            ("instance_create", [_, _, obj]) => match &obj.kind {
                ExprKind::Ident(name) if self.game.objects.contains_key(name.as_str()) => {
                    Some(name.clone())
                }
                _ => None,
            },
            _ => None,
        }
    }

    fn is_script(&self, name: &str) -> bool {
        self.game.scripts.contains_key(name)
    }

    /// Whether a place's code makes anything, itself or through the scripts it calls.
    fn spawns(&mut self, place: &str) -> Result<bool> {
        Ok(self.spawns_from(place, &mut Vec::new())?.0)
    }

    /// [`Self::spawns`], with the places being worked out on the way here (`visiting`), and whether the
    /// answer leaned on one of them (a cycle): a place in a cycle counts as making nothing until it's
    /// known, so a "no" that leaned on one isn't remembered.
    fn spawns_from(&mut self, place: &str, visiting: &mut Vec<String>) -> Result<(bool, bool)> {
        if let Some(&s) = self.spawners.get(place) {
            return Ok((s, false));
        }
        if visiting.iter().any(|v| v == place) {
            return Ok((false, true));
        }
        visiting.push(place.to_owned());
        let code = self.place_code(place)?;
        let (mut spawns, mut leaned) = (false, false);
        for call in &code.calls {
            if self.created(call).is_some() {
                spawns = true;
            } else if self.is_script(&call.name) {
                let (s, l) = self.spawns_from(&call.name, visiting)?;
                spawns = s;
                leaned |= l;
            }
            if spawns {
                break;
            }
        }
        visiting.pop();
        if spawns || !leaned {
            self.spawners.insert(place.to_owned(), spawns);
        }
        Ok((spawns, leaned))
    }

    /// Every `instance_create` reachable from `place` with `env`, following the scripts it calls and,
    /// with `follow_creates`, the create events of what it makes. With `own`, the place's calls are
    /// only those in that object's own code (not a parent's that it inherits).
    fn walk(
        &mut self,
        place: &str,
        env: &Env,
        follow_creates: bool,
        own: Option<&str>,
        path: &mut Vec<Step>,
        out: &mut Vec<Reach>,
    ) -> Result<()> {
        let code = self.place_code(place)?;
        for call in &code.calls {
            if own.is_some() && call.at.object.as_deref() != own {
                continue;
            }
            if self.any_false(&call.when, env) {
                continue;
            }
            let step = Step {
                place: place.to_owned(),
                when: call.when.clone(),
                at: call.at.clone(),
                tile: self.tile_of(&call.when),
                use_at: call.use_at,
            };
            let on_path = |name: &str| name == place || path.iter().any(|s| s.place == name);
            if let Some(object) = self.created(call) {
                let create = format!("{object}:create");
                let follow = follow_creates
                    && path.len() < MAX_DEPTH
                    && !on_path(&create)
                    && self.spawns(&create)?;
                path.push(step);
                out.push(Reach {
                    object,
                    steps: path.clone(),
                });
                if follow {
                    // A new instance: only the globals carry over.
                    let globals: Env = env
                        .iter()
                        .filter(|(k, _)| k.starts_with("global."))
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                    self.walk(&create, &globals, true, None, path, out)?;
                }
                path.pop();
            } else if self.is_script(&call.name)
                && path.len() < MAX_DEPTH
                && !on_path(&call.name)
                && self.spawns(&call.name)?
            {
                // The same instance runs the script: its variables carry over, and the arguments that
                // are constants are bound.
                let mut inner: Env = env
                    .iter()
                    .filter(|(k, _)| !k.starts_with("argument"))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                for (i, arg) in call.args.iter().enumerate() {
                    if let Some(v) = eval::eval(arg, &call.code, env) {
                        inner.insert(format!("argument{i}"), v);
                    }
                }
                path.push(step);
                self.walk(&call.name, &inner, follow_creates, None, path, out)?;
                path.pop();
            }
        }
        Ok(())
    }

    fn condition(&mut self, text: &str) -> Option<Expr> {
        if let Some(e) = self.conditions.get(text) {
            return e.clone();
        }
        let e = gml::parse_expression(text).ok();
        self.conditions.insert(text.to_owned(), e.clone());
        e
    }

    /// Whether a condition holds with `env`, if that's known.
    fn holds(&mut self, when: &When, env: &Env) -> Option<bool> {
        let (text, negated) = match when {
            When::If(c) | When::Case(c) => (c, false),
            When::Not(c) => (c, true),
            _ => return None,
        };
        let expr = self.condition(text)?;
        eval::truth(&expr, text, env).map(|t| t != negated)
    }

    fn any_false(&mut self, when: &[When], env: &Env) -> bool {
        when.iter().any(|w| self.holds(w, env) == Some(false))
    }

    /// What hurts in a collision event's code: the other object and its descendants, less those for which
    /// a test of the other's own variables (`if (other.puncture)`, `other.type == "Machete"`) can't hold,
    /// as their create events set them once and for all; and those it rules out. A yeti's whip code hurts
    /// it only `if (other.puncture)`: a machete's slash or a mattock's hit (descendants of oWhip that set
    /// it true), never the whip itself, which sets it false. Its wind-up code, only `if (other.type ==
    /// "Machete")`: the machete's (oMachetePre), not the whip's (oWhipPre, its parent) or the mattock's.
    /// Only the tests outside any `with` are the collision's other's (inside one, `other` is the instance
    /// that ran it). With nothing ruled out, it's the other object alone, as the code names it.
    fn others_tested(&mut self, other: &str, when: &[When]) -> Result<(Vec<String>, Vec<String>)> {
        let tests: Vec<When> = when
            .iter()
            .take_while(|w| !matches!(w, When::With(_)))
            .filter(|w| matches!(w, When::If(c) | When::Not(c) | When::Case(c) if reads_other(c)))
            .cloned()
            .collect();
        if tests.is_empty() {
            return Ok((vec![other.to_owned()], Vec::new()));
        }
        let game = self.game;
        let (root, _) = game.object(other)?;
        let (mut kept, mut ruled_out) = (Vec::new(), Vec::new());
        for (&name, &id) in &game.objects {
            let mut at = Some(id);
            while at.is_some_and(|i| i != root) {
                at = at.and_then(|i| game.parent(i));
            }
            if at.is_none() {
                continue;
            }
            let env = self.other_env(id)?;
            if self.any_false(&tests, &env) {
                ruled_out.push(name.to_owned());
            } else {
                kept.push(name.to_owned());
            }
        }
        if ruled_out.is_empty() {
            kept = vec![other.to_owned()];
        }
        Ok((kept, ruled_out))
    }

    /// What an instance of object `id` holds for good, as a collision's `other`: the constants its create
    /// event sets (`other.type`), less any variable other code may set on it ([`Analyzer::set_elsewhere`])
    /// and the built-ins, which the runner changes (`x`, `image_index`, `alarm`).
    fn other_env(&mut self, id: usize) -> Result<Rc<Env>> {
        if let Some(env) = self.other_envs.get(&id) {
            return Ok(env.clone());
        }
        let game = self.game;
        let set_elsewhere = self.set_elsewhere()?;
        let mut lineage = BTreeSet::new();
        let mut at = Some(id);
        while let Some(i) = at {
            lineage.extend(game.bundle.objects[i].as_ref().map(|o| o.name.as_str()));
            at = game.parent(i);
        }
        let set_on = |var: &str| match set_elsewhere.get(var) {
            None => false,
            Some(None) => true,
            Some(Some(objects)) => objects.iter().any(|o| lineage.contains(o.as_str())),
        };
        let env: Env = (game.init(id)?.into_iter())
            .filter(|(var, i)| {
                !i.changed_later
                    && !set_on(var)
                    && gm::Builtin::from_name(var).is_none_or(|b| !b.is_instance())
            })
            .filter_map(|(var, i)| {
                let value = match i.value? {
                    Json::String(s) => Value::Str(s),
                    v => Value::Real(v.as_f64()?),
                };
                Some((format!("other.{var}"), value))
            })
            .collect();
        let env = Rc::new(env);
        self.other_envs.insert(id, env.clone());
        Ok(env)
    }

    /// The variables code other than a create event's own sets after it, and the objects whose instances
    /// it may set them on (`None`: any): an event's code, its object's (and the descendants' that run it);
    /// `obj.v = ...`, the object named or the one created (`obj = instance_create(x, y, oExit);
    /// obj.type = "Moai Exit";`); and a script's, a `with`'s or any other instance's, any. A create
    /// event's constant for a variable that isn't set on its object's instances is theirs for good (as far
    /// as the code says: `variable_local_set` and the like aren't followed, and the game never calls
    /// them).
    fn set_elsewhere(&mut self) -> Result<Rc<SetElsewhere>> {
        if let Some(set) = &self.set_elsewhere {
            return Ok(set.clone());
        }
        let game = self.game;
        let mut units = Vec::new();
        for (object, event) in game.own_events() {
            let (id, _) = game.object(&object)?;
            let create = event == "create";
            for unit in game.event_units(id, &event, 0)? {
                // Inherited code is its own object's, but for a create event's: `init` reads only the
                // code that always runs, so a parent's that runs under a question sets it after.
                if create || unit.object.as_deref() == Some(object.as_str()) {
                    units.push((create, Some(object.clone()), unit));
                }
            }
        }
        for script in game.scripts.keys() {
            units.push((false, None, game.script_unit(script)?));
        }
        let mut set = SetElsewhere::new();
        let mut add = |var: &str, objects: Option<BTreeSet<String>>| {
            let have = set
                .entry(var.to_owned())
                .or_insert_with(|| Some(BTreeSet::new()));
            match (have.as_mut(), objects) {
                (Some(have), Some(objects)) => have.extend(objects),
                _ => *have = None,
            }
        };
        for (create, own, unit) in &units {
            let statements = find::statements(&unit.program);
            // What a variable holds as far as an `instance_create` says, for `obj.v = ...`.
            let created: Vec<Assign> = (statements.iter())
                .filter_map(|(stmt, when)| {
                    let StmtKind::Assign { place, op, value } = &stmt.kind else {
                        return None;
                    };
                    let ExprKind::Ident(var) = &place.kind else {
                        return None;
                    };
                    let objects = match &value.kind {
                        ExprKind::Call { name, args }
                            if *op == AssignOp::Set && name.name == "instance_create" =>
                        {
                            args.get(2).and_then(|a| self.object_name(a))
                        }
                        _ => None,
                    };
                    Some(Assign {
                        var: var.clone(),
                        start: stmt.span.start,
                        when: when.clone(),
                        value: None,
                        objects: objects.into_iter().collect(),
                    })
                })
                .collect();
            for (stmt, when) in &statements {
                let StmtKind::Assign { place, .. } = &stmt.kind else {
                    continue;
                };
                let in_with = (unit.when.iter().chain(when)).any(|w| matches!(w, When::With(_)));
                let mut place = place;
                while let ExprKind::Index { base, .. } = &place.kind {
                    place = base;
                }
                let mine = own
                    .clone()
                    .filter(|_| !in_with)
                    .map(|o| BTreeSet::from([o]));
                match &place.kind {
                    // The create event's own, which `init` reads (not a `self.v`, which it doesn't).
                    ExprKind::Ident(_) if *create && unit.when.is_empty() && !in_with => {}
                    ExprKind::Ident(var) => add(var, mine),
                    ExprKind::Field { base, name }
                        if matches!(base.kind, ExprKind::Const(Const::SelfKw)) =>
                    {
                        add(&name.name, mine)
                    }
                    // No instance's.
                    ExprKind::Field { base, .. }
                        if matches!(base.kind, ExprKind::Const(Const::Global)) => {}
                    ExprKind::Field { base, name } => {
                        let objects = match &base.kind {
                            ExprKind::Ident(_) if in_with => None,
                            ExprKind::Ident(o) if game.objects.contains_key(o.as_str()) => {
                                Some(BTreeSet::from([o.clone()]))
                            }
                            ExprKind::Ident(v) => created_as(v, &created, stmt.span.start, when),
                            _ => None,
                        };
                        add(&name.name, objects);
                    }
                    _ => {}
                }
            }
        }
        let set = Rc::new(set);
        self.set_elsewhere = Some(set.clone());
        Ok(set)
    }

    /// The room string character the innermost `if (tile == "c" ...)` tests.
    fn tile_of(&mut self, when: &[When]) -> Option<char> {
        if self.tile.is_empty() {
            return None;
        }
        for w in when.iter().rev() {
            let When::If(text) = w else { continue };
            let Some(expr) = self.condition(text) else {
                continue;
            };
            let mut conjuncts = vec![&expr];
            while let Some(e) = conjuncts.pop() {
                match &e.kind {
                    ExprKind::Binary {
                        op: BinOp::And,
                        lhs,
                        rhs,
                    } => conjuncts.extend([&**lhs, &**rhs]),
                    ExprKind::Binary {
                        op: BinOp::Eq,
                        lhs,
                        rhs,
                    } => {
                        if let (ExprKind::Ident(var), ExprKind::Str(s)) = (&lhs.kind, &rhs.kind)
                            && *var == self.tile
                            && let [c] = s.chars().collect::<Vec<_>>()[..]
                        {
                            return Some(c);
                        }
                    }
                    _ => {}
                }
            }
        }
        None
    }

    /// The room conditions of `side`'s level for code in `place`.
    fn rooms(&self, place: &str, side: Side) -> Vec<String> {
        side.map(|(level, _)| {
            self.rooms[level]
                .iter()
                .filter(|(script, _)| script == place)
                .map(|(_, w)| w.clone())
                .collect()
        })
        .unwrap_or_default()
    }

    /// Whether conditions in `place` pick a room generation never makes.
    fn unreachable(&self, place: &str, when: &[When]) -> bool {
        self.unreachable
            .iter()
            .any(|(script, w)| script == place && mentions(when, w))
    }

    /// Whether a `switch` case among `when` (of code at `use_at` in `code`) is one its value never has
    /// with `env`. A case's value is a constant; the switch's, a `rand` or a variable whose assignments
    /// before it say what it can hold, each under its conditions: the Temple's main rooms roll
    /// `if (global.cityOfGold) n = rand(1,12); else n = rand(1,10);`, so off the City of Gold, cases 11
    /// and 12 never happen. Anything else (a variable they don't pin down, a `default`) can.
    fn case_unreachable(
        &mut self,
        code: &PlaceCode,
        use_at: UseAt,
        when: &[When],
        env: &Env,
    ) -> bool {
        let local = &when[use_at.local.min(when.len())..];
        let assigns = &code.assigns[use_at.unit];
        let ifs = &code.ifs[use_at.unit];
        let source = code.units[use_at.unit].code.as_str();
        for (i, w) in local.iter().enumerate() {
            let When::Case(text) = w else { continue };
            let Some(expr) = self.condition(text) else {
                continue;
            };
            // `n == 11 || n == 12`, when cases fall through.
            let mut tests = vec![&expr];
            let mut never = true;
            while never && let Some(e) = tests.pop() {
                match &e.kind {
                    ExprKind::Binary {
                        op: BinOp::Or,
                        lhs,
                        rhs,
                    } => tests.extend([&**lhs, &**rhs]),
                    ExprKind::Binary {
                        op: BinOp::Eq,
                        lhs,
                        rhs,
                    } => {
                        let case = eval::eval(rhs, text, &Env::new()).and_then(|v| v.real());
                        let values = match &lhs.kind {
                            ExprKind::Ident(var) => {
                                let read = Read {
                                    assigns,
                                    ifs,
                                    source,
                                    start: use_at.start,
                                    ctx: &local[..i],
                                };
                                values(var, &read, &mut |w| !self.any_false(w, env))
                            }
                            _ => eval::range(lhs, text).map(|r| vec![r]),
                        };
                        never = match (case, values) {
                            (Some(k), Some(values)) => {
                                values.iter().all(|r| k < r.min || k > r.max)
                            }
                            _ => false,
                        };
                    }
                    _ => never = false,
                }
            }
            if never {
                return true;
            }
        }
        false
    }

    /// Whether a reach's every step can run in a scenario: `env` (the area's and the level flag's
    /// values) and, for a special level, whether its rooms are where the steps are (`side`).
    fn path_ok(&mut self, reach: &Reach, env: &Env, scenario: usize, side: Side) -> Result<bool> {
        for step in &reach.steps {
            if !self.step_ok(step, env, scenario, side)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn step_ok(&mut self, step: &Step, env: &Env, scenario: usize, side: Side) -> Result<bool> {
        if self.any_false(&step.when, env) || self.unreachable(&step.place, &step.when) {
            return Ok(false);
        }
        let code = self.place_code(&step.place)?;
        if self.case_unreachable(&code, step.use_at, &step.when, env) {
            return Ok(false);
        }
        let rooms = self.rooms(&step.place, side);
        let in_room = rooms.iter().any(|r| mentions(&step.when, r));
        match step.tile {
            Some(c) => self.tile_ok(&step.place, c, env, scenario, side, &rooms, in_room, 0),
            None => Ok(room_side_ok(&rooms, side, in_room)),
        }
    }

    /// Whether a room string character can be where `side` says: in some room or obstacle string of
    /// `place` whose conditions can hold, in the level's rooms or out of them as asked. An obstacle's
    /// string is itself placed by a character of the room's (`tile == "5"`), which is traced back.
    #[allow(clippy::too_many_arguments)]
    fn tile_ok(
        &mut self,
        place: &str,
        c: char,
        env: &Env,
        scenario: usize,
        side: Side,
        rooms: &[String],
        in_room: bool,
        depth: usize,
    ) -> Result<bool> {
        let key = (scenario, place.to_owned(), c, in_room);
        if depth == 0
            && let Some(&ok) = self.tiles.get(&key)
        {
            return Ok(ok);
        }
        let code = self.place_code(place)?;
        let mut ok = false;
        for t in code.templates.iter().filter(|t| t.text.contains(c)) {
            if self.any_false(&t.when, env)
                || self.unreachable(place, &t.when)
                || self.case_unreachable(&code, t.use_at, &t.when, env)
            {
                continue;
            }
            let r = in_room || rooms.iter().any(|room| mentions(&t.when, room));
            let outer = if depth < MAX_TILE_DEPTH {
                self.tile_of(&t.when).filter(|&d| d != c)
            } else {
                None
            };
            ok = match outer {
                Some(d) => self.tile_ok(place, d, env, scenario, side, rooms, r, depth + 1)?,
                None => room_side_ok(rooms, side, r),
            };
            if ok {
                break;
            }
        }
        if depth == 0 {
            self.tiles.insert(key, ok);
        }
        Ok(ok)
    }

    /// The objects a condition tests touching where it's true (`positive`) or false, and those of the
    /// flag variables it tests.
    fn collided(&self, expr: &Expr, positive: bool, vars: &Vars<'_>, out: &mut BTreeSet<String>) {
        match &expr.kind {
            ExprKind::Unary {
                op: UnOp::Not,
                expr,
            } => self.collided(expr, !positive, vars, out),
            ExprKind::Binary {
                op: BinOp::And | BinOp::Or,
                lhs,
                rhs,
            } => {
                self.collided(lhs, positive, vars, out);
                self.collided(rhs, positive, vars, out);
            }
            ExprKind::Call { name, args }
                if positive && COLLISION_FUNCTIONS.contains(&name.name.as_str()) =>
            {
                out.extend(args.iter().filter_map(|a| self.object_name(a)));
            }
            ExprKind::Ident(n) if positive => out.extend(vars(n)),
            _ => {}
        }
    }

    /// The objects conditions test touching: an `if`'s where it holds, an `else`'s where it doesn't.
    fn involves(&mut self, when: &[When], vars: &Vars<'_>) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        for w in when {
            let (text, positive) = match w {
                When::If(c) => (c, true),
                When::Not(c) => (c, false),
                _ => continue,
            };
            if let Some(expr) = self.condition(text) {
                self.collided(&expr, positive, vars, &mut out);
            }
        }
        out
    }
}

/// A program's `if` statements: their conditions, and what they test (as a `When::If` gives it).
fn ifs(program: &gml::Program) -> Vec<(Vec<When>, String)> {
    find::statements(program)
        .into_iter()
        .filter_map(|(stmt, when)| match &stmt.kind {
            StmtKind::If { cond, .. } => Some((when, gml::print::expr(cond))),
            _ => None,
        })
        .collect()
}

/// Whether a step's room standing fits the side asked for. Off the level, it can't be in the level's
/// rooms; on it, it can be anywhere, since a level with a snake pit has its ordinary rooms too.
fn room_side_ok(rooms: &[String], side: Side, in_room: bool) -> bool {
    match side {
        Some((_, false)) if !rooms.is_empty() => !in_room,
        _ => true,
    }
}

/// A read of a variable, for [`values`]: its unit's assignments and `if`s, its source, where the read
/// is and its conditions in the unit (`ctx`).
struct Read<'r> {
    assigns: &'r [Assign],
    ifs: &'r [(Vec<When>, String)],
    source: &'r str,
    start: u32,
    ctx: &'r [When],
}

/// The values variable `var` can hold at `read`, as ranges, if its assignments before there say: the last
/// one that encloses the read and any after it (as [`resolve`] has), or else ones within the read's
/// conditions that cover every way there (`if (...) n = ...; else n = ...;`) and any after them. Each
/// counts if `possible` says its conditions can hold. A read in a loop (where an assignment after it runs
/// before it, next time round) or a `with` (another instance's variable) isn't pinned down, nor one whose
/// conditions name an `if` twice (as two `if (a)`s, `a` may change between). A script called in between
/// could change it too; none of the room scripts' does.
fn values(
    var: &str,
    read: &Read<'_>,
    possible: &mut dyn FnMut(&[When]) -> bool,
) -> Option<Vec<eval::Range>> {
    let ctx = read.ctx;
    if ctx
        .iter()
        .any(|w| matches!(w, When::Loop(_) | When::With(_)))
    {
        return None;
    }
    let one_if = ctx.iter().enumerate().all(|(i, w)| match w {
        When::If(c) | When::Not(c) => single_if(read.ifs, &ctx[..i], c),
        _ => true,
    });
    if !one_if {
        return None;
    }
    let before: Vec<&Assign> = (read.assigns.iter())
        .filter(|a| a.var == var && a.start < read.start)
        .collect();
    let from = match before.iter().rposition(|a| ctx.starts_with(&a.when)) {
        Some(from) => from,
        None => {
            let within: Vec<&Assign> = (before.iter())
                .filter(|a| a.when.starts_with(ctx))
                .copied()
                .collect();
            if !covers(&within, read.ifs, ctx) {
                return None;
            }
            before.iter().position(|a| a.when.starts_with(ctx))?
        }
    };
    let mut out = Vec::new();
    for a in &before[from..] {
        if !possible(&a.when) {
            continue;
        }
        let value = a.value.as_ref()?;
        out.push(match eval::eval(value, read.source, &Env::new()) {
            Some(Value::Real(v)) => eval::Range {
                min: v,
                max: v,
                integer: true,
            },
            Some(Value::Str(_)) => return None,
            None => eval::range(value, read.source)?,
        });
    }
    Some(out)
}

/// Whether one `if` statement right under `prefix` tests `cond`, so an `If` and a `Not` of it there are
/// its two sides.
fn single_if(ifs: &[(Vec<When>, String)], prefix: &[When], cond: &str) -> bool {
    ifs.iter().filter(|(w, c)| w == prefix && c == cond).count() == 1
}

/// Whether some assignment of `assigns` runs on every way through code under `prefix`: one right under
/// it, or ones that cover both sides of an `if` right under it (`ifs`, the unit's).
fn covers(assigns: &[&Assign], ifs: &[(Vec<When>, String)], prefix: &[When]) -> bool {
    assigns.iter().any(|a| {
        if !a.when.starts_with(prefix) {
            return false;
        }
        match a.when.get(prefix.len()) {
            None => true,
            Some(When::If(c)) => {
                single_if(ifs, prefix, c)
                    && [When::If(c.clone()), When::Not(c.clone())]
                        .into_iter()
                        .all(|side| covers(assigns, ifs, &[prefix, &[side]].concat()))
            }
            Some(_) => false,
        }
    })
}

/// Whether an `if` or `case` among `when` has `text` in its condition.
fn mentions(when: &[When], text: &str) -> bool {
    when.iter()
        .any(|w| matches!(w, When::If(_) | When::Case(_)) && w.text().contains(text))
}

/// Where a span of a unit's code is in the `gml/` tree.
fn at(unit: &crate::Unit, lines: &LineIndex, span: gml::Span) -> At {
    let text = span.text(&unit.code);
    At {
        file: unit.file.clone(),
        line: unit.first_line + lines.line_col(span.start).line as usize - 1,
        object: unit.object.clone(),
        event: unit.event.clone(),
        script: unit.script.clone(),
        source: text.replace("\r\n", "\n").trim().to_owned(),
    }
}

// ---------------------------------------------------------------------------------------------------------
// Building

pub fn build(
    game: &Game<'_>,
    specs: &RelationSpecs,
    slugs: BTreeMap<String, String>,
    things: &BTreeSet<String>,
) -> Result<Relations> {
    let mut analyzer = Analyzer::new(game, specs.generation.as_ref(), &specs.levels);
    let runs_as = game.runs_as()?;
    let slug = |object: &str| slugs.get(object).cloned();

    // A drops spec's script must be one (an event would rule out its object's own drops).
    for n in specs.drops.iter().flat_map(|d| &d.not_via) {
        if n.script.contains(':') {
            return Err(format!("[drops] not_via {:?} is an event, not a script", n.script).into());
        }
        analyzer.place_code(&n.script)?;
    }
    // A room condition that's in no `if` or `case` of its script would quietly never match.
    let rooms = specs
        .levels
        .iter()
        .flat_map(|l| &l.rooms)
        .map(|r| (&r.script, &r.where_));
    let unreachable = specs.generation.iter().flat_map(|g| &g.unreachable);
    for (script, where_) in rooms.chain(unreachable.map(|u| (&u.script, &u.where_))) {
        let code = analyzer.place_code(script)?;
        let found = (code.calls.iter().map(|c| &c.when))
            .chain(code.templates.iter().map(|t| &t.when))
            .any(|when| mentions(when, where_));
        if !found {
            return Err(format!("room {where_:?} matches nothing in {script}").into());
        }
    }

    // Spawns: one walk per area.
    let areas: Vec<Env> = specs
        .areas
        .iter()
        .map(|a| bind(&a.bind).map_err(|e| format!("area {}: {e}", a.slug)))
        .collect::<std::result::Result<_, _>>()?;
    struct Entry {
        object: String,
        areas: BTreeSet<usize>,
        /// Per level: whether it can happen on that level, and off it.
        sides: Vec<[bool; 2]>,
        step: Step,
        via: Vec<String>,
    }
    let mut entries: Vec<Entry> = Vec::new();
    let mut index: BTreeMap<(String, String, usize, String), usize> = BTreeMap::new();
    let roots = specs
        .generation
        .as_ref()
        .map(|g| g.roots.clone())
        .unwrap_or_default();
    // Scenario ids for tile_ok's cache: per area, one for the area alone and two per level.
    let scenarios = 1 + 2 * specs.levels.len();
    for (ai, env) in areas.iter().enumerate() {
        let mut reaches = Vec::new();
        for root in &roots {
            analyzer
                .walk(root, env, true, None, &mut Vec::new(), &mut reaches)
                .map_err(|e| format!("generation from {root}: {e}"))?;
        }
        for reach in reaches {
            if !analyzer.path_ok(&reach, env, ai * scenarios, None)? {
                continue;
            }
            let last = reach.last();
            let key = (
                reach.object.clone(),
                last.at.file.clone(),
                last.at.line,
                last.at.source.clone(),
            );
            let i = *index.entry(key).or_insert_with(|| {
                entries.push(Entry {
                    object: reach.object.clone(),
                    areas: BTreeSet::new(),
                    sides: vec![[false; 2]; specs.levels.len()],
                    step: last.clone(),
                    via: Vec::new(),
                });
                entries.len() - 1
            });
            entries[i].areas.insert(ai);
            let via = reach.via();
            if !entries[i].via.contains(&via) {
                entries[i].via.push(via);
            }
            for (li, level) in specs.levels.iter().enumerate() {
                for (si, inside) in [true, false].into_iter().enumerate() {
                    let mut env = env.clone();
                    if let Some(flag) = &level.flag {
                        let value = Value::Real(f64::from(u8::from(inside)));
                        // An area that pins the flag (the Ice Caves are never dark) has only its side.
                        if env.get(flag).is_some_and(|v| *v != value) {
                            continue;
                        }
                        env.insert(flag.clone(), value);
                    }
                    let scenario = ai * scenarios + 1 + 2 * li + si;
                    if analyzer.path_ok(&reach, &env, scenario, Some((li, inside)))? {
                        entries[i].sides[li][si] = true;
                    }
                }
            }
        }
    }
    let mut spawns_in: BTreeMap<String, Vec<Spawn>> = BTreeMap::new();
    for e in entries {
        let level_slugs = |want: [bool; 2]| {
            specs
                .levels
                .iter()
                .zip(&e.sides)
                .filter(|(_, sides)| **sides == want)
                .map(|(l, _)| l.slug.clone())
                .collect::<Vec<_>>()
        };
        spawns_in.entry(e.object).or_default().push(Spawn {
            areas: e
                .areas
                .iter()
                .map(|&a| specs.areas[a].slug.clone())
                .collect(),
            only_in: level_slugs([true, false]),
            not_in: level_slugs([false, true]),
            tile: e.step.tile.map(String::from),
            when: e.step.when,
            at: e.step.at,
            via: e.via,
        });
    }

    // Shops.
    let mut shops = Vec::new();
    let mut sold_in: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let shop_scenario = areas.len() * scenarios;
    for (si, spec) in specs.shops.iter().enumerate() {
        let env = bind(&spec.bind).map_err(|e| format!("shop {}: {e}", spec.slug))?;
        let mut reaches = Vec::new();
        analyzer
            .walk(
                &spec.place,
                &env,
                false,
                None,
                &mut Vec::new(),
                &mut reaches,
            )
            .map_err(|e| format!("shop {}: {e}", spec.slug))?;
        let mut sells: Vec<Sale> = Vec::new();
        for reach in reaches {
            let kept = spec
                .where_
                .iter()
                .all(|w| reach.steps.iter().any(|s| mentions(&s.when, w)));
            if !kept || !analyzer.path_ok(&reach, &env, shop_scenario + si, None)? {
                continue;
            }
            let last = reach.last();
            if sells.iter().any(|s| {
                s.object == reach.object && s.at.file == last.at.file && s.at.line == last.at.line
            }) {
                continue;
            }
            let price = game
                .object(&reach.object)
                .and_then(|(id, _)| game.init(id))?
                .remove("cost")
                .and_then(|c| c.value);
            let via = if reach.steps.len() > 1 {
                vec![reach.via()]
            } else {
                Vec::new()
            };
            let list = sold_in.entry(reach.object.clone()).or_default();
            if !list.contains(&spec.slug) {
                list.push(spec.slug.clone());
            }
            sells.push(Sale {
                object: reach.object.clone(),
                slug: slug(&reach.object),
                price,
                when: last.when.clone(),
                at: last.at.clone(),
                via,
            });
        }
        if sells.is_empty() {
            return Err(format!("shop {}: sells nothing", spec.slug).into());
        }
        shops.push(Shop {
            slug: spec.slug.clone(),
            name: spec.name.clone(),
            place: spec.place.clone(),
            bind: bind_json(&env),
            where_: spec.where_.clone(),
            sells,
        });
    }

    // Drops, and damage, from every object's own events (but create) and, for damage, every script.
    let not_via = specs.drops.as_ref().map_or(&[][..], |d| &d.not_via[..]);
    let mut excluded = vec![false; not_via.len()];
    let mut drops: BTreeMap<String, Vec<Drop>> = BTreeMap::new();
    let mut damage = Vec::new();
    let patterns: Vec<(Pattern, &DamageSpec)> = specs
        .damage
        .iter()
        .map(|d| Ok((Pattern::parse(&d.find)?, d)))
        .collect::<Result<_>>()?;
    for (owner, event) in game.own_events() {
        let place = format!("{owner}:{event}");
        let trigger = event.strip_prefix("collision_").map(str::to_owned);
        let runs = runs_as.get(&place).cloned().unwrap_or_default();
        if event != "create" {
            let mut reaches = Vec::new();
            analyzer.walk(
                &place,
                &Env::new(),
                false,
                Some(&owner),
                &mut Vec::new(),
                &mut reaches,
            )?;
            let code = analyzer.place_code(&place)?;
            for reach in reaches {
                if !things.contains(&reach.object) {
                    continue;
                }
                // Not a drop if a script the spec rules out makes it (the player's held item).
                if let Some(n) = not_via.iter().position(|n| n.script == reach.last().place) {
                    excluded[n] = true;
                    continue;
                }
                // What the object's own code tests touching on the way (the first step is in it).
                let first = &reach.steps[0];
                let use_at = first.use_at;
                let assigns = &code.assigns[use_at.unit];
                let local = &first.when[use_at.local..];
                let vars = |v: &str| resolve(v, assigns, use_at.start, local);
                let involves: Vec<String> = analyzer
                    .involves(&first.when, &vars)
                    .into_iter()
                    .filter(|o| things.contains(o))
                    .collect();
                let keys = if involves.is_empty() {
                    vec![owner.clone()]
                } else {
                    involves.clone()
                };
                let last = reach.last();
                let via = if reach.steps.len() > 1 {
                    vec![reach.via()]
                } else {
                    Vec::new()
                };
                for key in keys {
                    drops.entry(key).or_default().push(Drop {
                        object: reach.object.clone(),
                        slug: slug(&reach.object),
                        by: place.clone(),
                        runs_as: runs.clone(),
                        trigger: trigger.clone(),
                        involves: involves.clone(),
                        when: last.when.clone(),
                        at: last.at.clone(),
                        via: via.clone(),
                    });
                }
            }
        }
        let code = Code {
            place: &place,
            owner: Some(&owner),
            runs_as: &runs,
            trigger: trigger.as_deref(),
        };
        damage.extend(damage_facts(&mut analyzer, &patterns, &code)?);
    }
    for script in game.scripts.keys() {
        let code = Code {
            place: script,
            owner: None,
            runs_as: &[],
            trigger: None,
        };
        damage.extend(damage_facts(&mut analyzer, &patterns, &code)?);
    }
    // One that rules nothing out is stale, or misspelt.
    if let Some(n) = not_via
        .iter()
        .zip(&excluded)
        .find_map(|(n, &used)| (!used).then_some(n))
    {
        return Err(format!("[drops] not_via {:?} rules out no drop", n.script).into());
    }

    Ok(Relations {
        format: FORMAT,
        notice: NOTICE,
        license: LICENSE_URL,
        areas: specs
            .areas
            .iter()
            .zip(&areas)
            .map(|(a, env)| AreaOut {
                slug: a.slug.clone(),
                name: a.name.clone(),
                bind: bind_json(env),
            })
            .collect(),
        levels: specs
            .levels
            .iter()
            .map(|l| LevelOut {
                slug: l.slug.clone(),
                name: l.name.clone(),
                flag: l.flag.clone(),
                rooms: l
                    .rooms
                    .iter()
                    .map(|r| RoomSpec {
                        script: r.script.clone(),
                        where_: r.where_.clone(),
                    })
                    .collect(),
            })
            .collect(),
        unreachable: specs
            .generation
            .iter()
            .flat_map(|g| g.unreachable.iter().cloned())
            .collect(),
        not_drops: not_via.to_vec(),
        slugs,
        spawns_in,
        placed_in: placed_in(game),
        shops,
        sold_in,
        drops,
        damage,
    })
}

/// The rooms each object is placed in by the room editor, with how many.
fn placed_in(game: &Game<'_>) -> BTreeMap<String, BTreeMap<String, usize>> {
    let mut out: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    for room in game.bundle.rooms.iter().flatten() {
        for instance in &room.instances {
            if let Some(o) = gmk::get(&game.bundle.objects, instance.object) {
                *out.entry(o.name.clone())
                    .or_default()
                    .entry(room.name.clone())
                    .or_default() += 1;
            }
        }
    }
    out
}

/// Where some code is, for [`damage_facts`]: its place, the object whose code it is and the objects
/// that run it, and a collision event's other object.
struct Code<'c> {
    place: &'c str,
    owner: Option<&'c str>,
    runs_as: &'c [String],
    trigger: Option<&'c str>,
}

/// The objects an instance expression names in code `depth` `with`s deep (`withs`, the targets, outermost
/// first): `self` is the code's object or the innermost `with`'s target; `other` the collision's other
/// object outside a `with`, and inside one the instance that ran it; a variable, the objects it was found
/// by.
fn instances(
    t: &str,
    depth: usize,
    withs: &[String],
    code: &Code<'_>,
    game: &Game<'_>,
    vars: &Vars<'_>,
) -> Vec<String> {
    match t {
        "self" if depth == 0 => code.owner.map(str::to_owned).into_iter().collect(),
        "self" => instances(&withs[depth - 1], depth - 1, withs, code, game, vars),
        "other" if depth == 0 => code.trigger.map(str::to_owned).into_iter().collect(),
        "other" => instances("self", depth - 1, withs, code, game, vars),
        t if game.objects.contains_key(t) => vec![t.to_owned()],
        t => vars(t).into_iter().collect(),
    }
}

/// The damage statements in a place's own code, with who they hurt and what hurts them.
fn damage_facts(
    analyzer: &mut Analyzer<'_, '_>,
    patterns: &[(Pattern, &DamageSpec)],
    code: &Code<'_>,
) -> Result<Vec<Damage>> {
    let place = analyzer.place_code(code.place)?;
    let game = analyzer.game;
    let mut out = Vec::new();
    for (unit, assigns) in place.units.iter().zip(&place.assigns) {
        if code.owner.is_some() && unit.object.as_deref() != code.owner {
            continue;
        }
        let lines = LineIndex::new(&unit.code);
        for (pattern, spec) in patterns {
            for m in find::find(pattern, &unit.program) {
                let f = fact(unit, &lines, &m);
                let vars = |v: &str| resolve(v, assigns, m.start, &m.when);
                let withs: Vec<String> = f
                    .when
                    .iter()
                    .filter_map(|w| match w {
                        When::With(t) => Some(t.clone()),
                        _ => None,
                    })
                    .collect();
                let depth = withs.len();
                let (target, victims) = match spec.target.as_str() {
                    "player" => ("player".to_owned(), vec![PLAYER.to_owned()]),
                    "self" => (
                        withs.last().cloned().unwrap_or_else(|| "self".to_owned()),
                        instances("self", depth, &withs, code, game, &vars),
                    ),
                    _ => {
                        let hole = m
                            .holes
                            .first()
                            .map(|h| gml::print::expr(h))
                            .unwrap_or_default();
                        let victims = instances(&hole, depth, &withs, code, game, &vars);
                        (hole, victims)
                    }
                };
                // Whether it hurts the instance running the code (not one a `with` or a variable picks):
                // then what hurts it is the collision's other object, else the code's own.
                let hurts_self = match spec.target.as_str() {
                    "player" => code.owner == Some(PLAYER),
                    "self" => depth == 0,
                    _ => false,
                };
                let mut sources: BTreeSet<String> = analyzer.involves(&f.when, &vars);
                // A collision test that finds the victim isn't what hurts it.
                for v in &victims {
                    sources.remove(v);
                }
                let mut ruled_out = Vec::new();
                if hurts_self {
                    if let Some(other) = code.trigger {
                        let (kept, out) = analyzer.others_tested(other, &f.when)?;
                        sources.extend(kept);
                        ruled_out = out;
                    }
                } else {
                    sources.extend(code.owner.map(str::to_owned));
                }
                // The amount is the last hole's (`other.hp -= ?` has the instance first).
                let amount = (m.holes.len() > usize::from(spec.target == "hole"))
                    .then(|| m.holes.last().copied())
                    .flatten();
                let source = &unit.code;
                out.push(Damage {
                    by: code.place.to_owned(),
                    runs_as: code.runs_as.to_vec(),
                    trigger: code.trigger.map(str::to_owned),
                    find: spec.find.clone(),
                    target,
                    victims,
                    sources: sources.into_iter().collect(),
                    ruled_out,
                    amount: amount.map(gml::print::expr),
                    value: amount
                        .and_then(|a| eval::eval(a, source, &Env::new()))
                        .map(|v| json_value(&v)),
                    range: amount.and_then(|a| eval::range(a, source)),
                    when: f.when,
                    at: f.at,
                });
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assign(var: &str, start: u32, when: &[When], objects: &[&str]) -> Assign {
        Assign {
            var: var.to_owned(),
            start,
            when: when.to_vec(),
            value: gml::parse_expression("true").ok(),
            objects: objects.iter().map(|&o| o.to_owned()).collect(),
        }
    }

    #[test]
    fn a_variable_holds_what_its_last_enclosing_assignment_and_later_conditional_ones_set() {
        let hit = When::If("collision_rectangle(x - 4, y - 4, x + 4, y + 8, oSpikes, 0, 0)".into());
        // `colSpikes = false; if (collision...) colSpikes = true; if (colSpikes) ...`
        let assigns = [
            assign("colSpikes", 10, &[], &[]),
            assign("colSpikes", 20, std::slice::from_ref(&hit), &["oSpikes"]),
        ];
        let read = [When::If("colSpikes".into())];
        let spikes = BTreeSet::from(["oSpikes".to_owned()]);
        assert_eq!(resolve("colSpikes", &assigns, 30, &read), spikes);
        // Read before it's set: unknown.
        assert!(resolve("colSpikes", &assigns, 5, &read).is_empty());
        // Set again unconditionally after the conditional one: only the last counts.
        let reset = [
            assign("colSpikes", 10, &[], &[]),
            assign("colSpikes", 20, std::slice::from_ref(&hit), &["oSpikes"]),
            assign("colSpikes", 25, &[], &[]),
        ];
        assert!(resolve("colSpikes", &reset, 30, &read).is_empty());
        // Only conditional assignments: it may hold an earlier step's value, so unknown.
        assert!(resolve("colSpikes", &assigns[1..], 30, &read).is_empty());
    }

    /// The values `n` can hold at `s = 1` in `code`, with `possible` judging conditions.
    fn n_at_s(code: &str, possible: &mut dyn FnMut(&[When]) -> bool) -> Option<Vec<(f64, f64)>> {
        let program = gml::parse_program(code).unwrap();
        let mut assigns = Vec::new();
        let mut read = None;
        for (stmt, when) in find::statements(&program) {
            let StmtKind::Assign { place, op, value } = &stmt.kind else {
                continue;
            };
            let ExprKind::Ident(var) = &place.kind else {
                continue;
            };
            if var == "s" {
                read = Some((stmt.span.start, when));
                continue;
            }
            assigns.push(Assign {
                var: var.clone(),
                start: stmt.span.start,
                when,
                value: (*op == AssignOp::Set).then(|| value.clone()),
                objects: BTreeSet::new(),
            });
        }
        let (start, when) = read.unwrap();
        // The switch's own case is the read's last condition.
        let ctx = &when[..when.len() - 1];
        let ifs = ifs(&program);
        let read = Read {
            assigns: &assigns,
            ifs: &ifs,
            source: code,
            start,
            ctx,
        };
        values("n", &read, possible).map(|v| v.iter().map(|r| (r.min, r.max)).collect())
    }

    #[test]
    fn a_switch_variable_holds_what_its_assignments_before_it_can_set() {
        let city = When::If("global.cityOfGold".into());
        let not_city = When::Not("global.cityOfGold".into());
        let anything = &mut |_: &[When]| true;
        // scrRoomGen4's main rooms: off the City of Gold, 1 to 10; in it, 1 to 12.
        let temple = "if (global.cityOfGold) n = rand(1,12); else n = rand(1,10); switch (n) { case 11: { s = 1; break; } }";
        assert_eq!(
            n_at_s(temple, &mut |w| !w.contains(&city)),
            Some(vec![(1.0, 10.0)])
        );
        assert_eq!(
            n_at_s(temple, &mut |w| !w.contains(&not_city)),
            Some(vec![(1.0, 12.0)])
        );
        // An `else if` chain covers every way, with what comes after it.
        let chain = "if (a) n = 1; else if (b) n = rand(2,3); else n = 4; if (c) n = 9; switch (n) { case 9: s = 1 }";
        assert_eq!(
            n_at_s(chain, anything),
            Some(vec![(1.0, 1.0), (2.0, 3.0), (4.0, 4.0), (9.0, 9.0)])
        );
        // The last enclosing assignment and those after it.
        let reset = "n = 5; if (a) n = 6; n = 7; if (b) n += 1; switch (n) { case 8: s = 1 }";
        assert_eq!(n_at_s(reset, anything), None);
        let set = "n = 5; if (a) n = 6; n = 7; if (b) n = 8; switch (n) { case 8: s = 1 }";
        assert_eq!(n_at_s(set, anything), Some(vec![(7.0, 7.0), (8.0, 8.0)]));
        // Unknown: only one side of an `if` assigns; two `if (a)`s aren't an if and its else; a read
        // in a loop or a `with`.
        let one_side = "if (a) n = 1; switch (n) { case 2: s = 1 }";
        assert_eq!(n_at_s(one_side, anything), None);
        let two_ifs = "if (a) n = 1; if (a) {} else n = 2; switch (n) { case 3: s = 1 }";
        assert_eq!(n_at_s(two_ifs, anything), None);
        let looped = "n = 1; repeat (3) { switch (n) { case 2: s = 1 } n = 2; }";
        assert_eq!(n_at_s(looped, anything), None);
        let with = "n = 1; with (oGame) { switch (n) { case 2: s = 1 } }";
        assert_eq!(n_at_s(with, anything), None);
    }

    #[test]
    fn a_field_assignment_sets_it_on_the_object_its_variable_was_created_as() {
        let created = |var: &str, start: u32, when: &[When], object: Option<&str>| Assign {
            objects: object.map(str::to_owned).into_iter().collect(),
            ..assign(var, start, when, &[])
        };
        let exit = BTreeSet::from(["oExit".to_owned()]);
        // scrRoomGen3: `obj = instance_create(xpos+16, ypos+48, oExit); obj.type = "Moai Exit";`
        let moai = [
            created("obj", 10, &[], Some("oExit")),
            created("obj", 30, &[], Some("oCrown")),
        ];
        assert_eq!(created_as("obj", &moai, 20, &[]), Some(exit));
        // Anything else it may hold, or nothing before it: any instance.
        let a = When::If("a".into());
        let maybe = [
            created("obj", 10, &[], Some("oExit")),
            created("obj", 15, std::slice::from_ref(&a), None),
        ];
        assert_eq!(created_as("obj", &maybe, 20, &[]), None);
        assert_eq!(created_as("obj", &moai, 5, &[]), None);
        // In a loop, the next time round it holds what's set after it.
        let rooms = [When::Loop("repeat (4)".into())];
        let looped = [
            created("obj", 10, &rooms, Some("oExit")),
            created("obj", 30, &rooms, Some("oCrown")),
        ];
        let both = BTreeSet::from(["oCrown".to_owned(), "oExit".to_owned()]);
        assert_eq!(created_as("obj", &looped, 20, &rooms), Some(both));
        let unknown = [
            created("obj", 10, &rooms, Some("oExit")),
            created("obj", 30, &rooms, None),
        ];
        assert_eq!(created_as("obj", &unknown, 20, &rooms), None);
    }

    #[test]
    fn a_test_of_the_other_reads_other_dot_something() {
        assert!(reads_other("other.type == \"Machete\""));
        assert!(reads_other("!whipped && (other.puncture)"));
        assert!(!reads_other("another.type == 1") && !reads_other("obj.other.x"));
        assert!(!reads_other("whipped == 0"));
    }

    #[test]
    fn rooms_only_rule_out_the_off_level_side() {
        let rooms = ["roomPath == 8".to_owned()];
        assert!(room_side_ok(&rooms, Some((0, true)), false));
        assert!(room_side_ok(&rooms, Some((0, true)), true));
        assert!(room_side_ok(&rooms, Some((0, false)), false));
        assert!(!room_side_ok(&rooms, Some((0, false)), true));
        assert!(room_side_ok(&[], Some((0, false)), true));
        // `if` and `case` conditions count, an `else`'s doesn't.
        let when = [
            When::Not("roomPath == 8".into()),
            When::Case("n == 2".into()),
        ];
        assert!(!mentions(&when, "roomPath == 8"));
        assert!(mentions(&when, "n == 2"));
    }
}
