//! The level search behind `area-route`, shared with the goal-directed fuzzer (`harness::fuzz`): best-first
//! over an archive of cells (Go-Explore without the randomness) for held-key macros that take the player to
//! a level's exit or another goal. See `area-route`'s docs.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use game::Game;
use gm::{GVar, State, Value};
use trace::replay::{Button, Event, InputState};

pub const LEFT: u8 = 37;
pub const UP: u8 = 38;
pub const RIGHT: u8 = 39;
pub const DOWN: u8 = 40;
pub const JUMP: u8 = 90; // Z
pub const ATTACK: u8 = 88; // X
pub const BOMB: u8 = 65; // A
pub const ROPE: u8 = 83; // S

/// The macros the search tries: keys held for some frames. The long ones make progress the archive's cells
/// don't show, such as pushing a block or climbing.
pub const ACTIONS: &[(&[u8], u64)] = &[
    (&[LEFT], 6),
    (&[RIGHT], 6),
    (&[LEFT, JUMP], 6),
    (&[RIGHT, JUMP], 6),
    (&[JUMP], 6),
    (&[], 6),
    (&[UP], 6),
    (&[DOWN], 6),
    (&[LEFT, ATTACK], 6),
    (&[RIGHT, ATTACK], 6),
    (&[LEFT], 30),
    (&[RIGHT], 30),
    (&[UP], 24),
    (&[DOWN], 24),
    (&[DOWN, JUMP], 6),
];
/// The macro `--market` adds while no bomb is lit: drop one at the player's feet.
pub const BOMB_ACTION: (&[u8], u64) = (&[DOWN, BOMB], 6);
/// The macro the way out of `rOlmec` adds: throw a rope, to climb out of the shaft Olmec dug.
pub const ROPE_ACTION: (&[u8], u64) = (&[ROPE], 6);
/// The macro `+<object>` adds: attack while ducking picks up what's at the player's feet.
pub const PICKUP_ACTION: (&[u8], u64) = (&[DOWN, ATTACK], 6);
pub const TILE: f64 = 16.0;
/// Distance-field cost of a heart lost.
pub const HEART: f64 = 30.0;
/// Distance-field cost of a solid tile on the way to the Black Market's entrance, which only bombs clear.
pub const DIG: f64 = 12.0;
/// How much nearer the exit a state with a lit bomb counts than where the bomb was lit.
pub const LIT: f64 = 20.0;
/// Distance-field cost of a hit point the targets of a kill have left.
const HIT: f64 = 10.0;
/// How near (in x and in y) an explosion an enemy that dies counts as killed by the bomb.
const BLAST: f64 = 40.0;
/// How near (in x and in y) the player a target hurt by a throw counts as stomped instead.
const STOMP: f64 = 20.0;

/// Runs one frame of `game` with these input events.
pub fn step_events(game: &mut Game, events: &[Event]) -> Result<(), String> {
    game.step(events.iter()).map_err(|e| game::describe(&e))
}

pub fn step(game: &mut Game, input: &mut InputState, keys: &[u8]) -> bool {
    let next = InputState::keys(keys.iter().copied());
    let events: Vec<Event> = input.events_to(&next);
    *input = next;
    game.step(events.iter()).is_ok()
}

/// A goal of `--steps`.
#[derive(Debug, PartialEq)]
pub enum Goal {
    Exit,
    /// The exit, holding on to the held item.
    Carry,
    /// Touch an instance of the object (or a descendant), or see one disappear (collected).
    Reach(String),
    /// `Reach`, holding on to the held item, as the sceptre into the gold door.
    Bring(String),
    /// Hold an instance of the object (or a descendant).
    Pickup(String),
    /// `Pickup`, bombing through blocks on the way (a key sealed in rock).
    Dig(String),
    /// Kill an instance of the object (or a descendant) `by` whipping or stomping it, a bomb or the held
    /// item thrown at it.
    Kill {
        object: String,
        by: Weapon,
    },
    /// Take a door that isn't a level's exit, `object`, into `room`, throwing ropes if `ropes`.
    Door {
        object: &'static str,
        room: &'static str,
        ropes: bool,
    },
}

/// How a `Goal::Kill` kills.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Weapon {
    /// The whip, or a stomp (`-`).
    Melee,
    /// A stomp only (`^`), without the whip: what lands on the target is exercised (spike shoes).
    Stomp,
    /// A bomb's explosion (`*`), the target dying within `BLAST` of it, without the whip.
    Bomb,
    /// The held item thrown at the target (`/`): it dies, or loses hit points (a caveman's are 3, so a rock
    /// stuns it), with a thrown item moving within `BLAST` of it, without the whip, the player more than
    /// `STOMP` away.
    Throw,
}

impl Goal {
    /// The door to the ending that `oFinalBoss` opens in `rOlmec` once Olmec is dead. The player may have to
    /// climb out of the shaft Olmec dug.
    pub const END: Goal = Goal::Door {
        object: "oXEnd",
        room: "rEnd",
        ropes: true,
    };
    /// The door back to the title in `rHighscores`.
    pub const TITLE: Goal = Goal::Door {
        object: "oXTitle",
        room: "rTitle",
        ropes: false,
    };

    pub fn parse(s: &str) -> Goal {
        match s {
            "exit" => Goal::Exit,
            "carry" => Goal::Carry,
            _ => match s.split_at(s.find(|c: char| c.is_alphabetic()).unwrap_or(0)) {
                ("+", o) => Goal::Pickup(o.to_string()),
                ("!", o) => Goal::Dig(o.to_string()),
                ("^", o) => Goal::Kill {
                    object: o.to_string(),
                    by: Weapon::Stomp,
                },
                (">", o) => Goal::Bring(o.to_string()),
                ("-", o) => Goal::Kill {
                    object: o.to_string(),
                    by: Weapon::Melee,
                },
                ("*", o) => Goal::Kill {
                    object: o.to_string(),
                    by: Weapon::Bomb,
                },
                ("/", o) => Goal::Kill {
                    object: o.to_string(),
                    by: Weapon::Throw,
                },
                _ => Goal::Reach(s.to_string()),
            },
        }
    }

    /// Whether the player must keep hold of the held item on the way.
    pub fn carries(&self) -> bool {
        matches!(self, Goal::Carry | Goal::Bring(_))
    }

    pub fn is_exit(&self) -> bool {
        matches!(self, Goal::Exit | Goal::Carry)
    }

    /// The object whose instances are the targets, for the goals that aren't the exit.
    pub fn object(&self) -> Option<&str> {
        match self {
            Goal::Reach(o)
            | Goal::Bring(o)
            | Goal::Pickup(o)
            | Goal::Dig(o)
            | Goal::Kill { object: o, .. } => Some(o),
            Goal::Door { object, .. } => Some(object),
            Goal::Exit | Goal::Carry => None,
        }
    }

    /// Whether the goal is met in `g` with the player alive, where `ids` are the instances of its object at
    /// the start, or for a bomb or a throw the ones alive just before this frame, and where they were (and
    /// their hit points).
    pub fn met(&self, g: &Game, ids: &[Target]) -> bool {
        let Some((_, p)) = player(g) else {
            return false;
        };
        if global(g, "plife").unwrap_or(0.0) < 1.0 {
            return false;
        }
        match self {
            Goal::Exit | Goal::Carry | Goal::Door { .. } => false,
            Goal::Bring(_) if held(g).is_none() => false,
            Goal::Reach(o) | Goal::Bring(o) => {
                let now = instances(g, o);
                // Treasure is collected on touch: one of the targets is gone.
                ids.iter()
                    .any(|(id, ..)| now.iter().all(|(_, i)| i.id != *id))
                    || now.iter().any(|&(h, i)| {
                        let (x, y) = centre(g, h, i);
                        (x - p.x).abs() <= 10.0 && (y - p.y).abs() <= 12.0
                    })
            }
            Goal::Pickup(o) | Goal::Dig(o) => held(g).is_some_and(|(h, i)| matches(g, h, i, o)),
            // Dead: destroyed (or turned into another object, as a hanging spider drops), or a corpse.
            Goal::Kill { object, by } => {
                let now = Goal::alive(g, object);
                let near = |e: &gm::Instance, x: f64, y: f64| {
                    (e.x - x).abs() <= BLAST && (e.y - y).abs() <= BLAST
                };
                let hurt = |&(id, _, _, hp): &Target| {
                    now.iter().find(|t| t.0 == id).is_none_or(|t| t.3 < hp)
                };
                let dead = |&(id, ..): &Target| now.iter().all(|t| t.0 != id);
                match by {
                    Weapon::Melee | Weapon::Stomp => ids.iter().any(dead),
                    // The target died beside an explosion, not to the whip while one was out.
                    Weapon::Bomb => {
                        let explosions = instances(g, "oExplosion");
                        ids.iter()
                            .any(|t| dead(t) && explosions.iter().any(|(_, e)| near(e, t.1, t.2)))
                    }
                    Weapon::Throw => {
                        if find(g, "oWhip").is_some() {
                            return false;
                        }
                        // A thrown item still flying (not one lying about or held).
                        let thrown: Vec<&gm::Instance> = instances(g, "oItem")
                            .into_iter()
                            .filter(|&(h, _)| {
                                let speed = |v| match var(g, h, v) {
                                    Some(Value::Real(s)) => s.abs(),
                                    _ => 0.0,
                                };
                                !matches!(var(g, h, "held"), Some(Value::Real(b)) if b >= 0.5)
                                    && (speed("xVel") > 2.0 || speed("yVel") > 2.0)
                            })
                            .map(|(_, i)| i)
                            .collect();
                        // Not a stomp: the player is away from the target's middle.
                        ids.iter().any(|t| {
                            hurt(t)
                                && thrown.iter().any(|e| near(e, t.1, t.2))
                                && ((t.1 + 8.0 - p.x).abs() > STOMP
                                    || (t.2 + 8.0 - p.y).abs() > STOMP)
                        })
                    }
                }
            }
        }
    }

    /// The instances of `object` (or a descendant) that are alive (not corpses, such as a caveman's or a
    /// yeti's, out of hit points), where they are and their hit points (0 if they have none).
    pub fn alive(g: &Game, object: &str) -> Vec<Target> {
        instances(g, object)
            .into_iter()
            .filter_map(|(h, i)| {
                let (x, y) = centre(g, h, i);
                match var(g, h, "hp") {
                    Some(Value::Real(hp)) if hp < 1.0 => None,
                    Some(Value::Real(hp)) => Some((i.id, x, y, hp)),
                    _ => Some((i.id, x, y, 0.0)),
                }
            })
            .collect()
    }
}

/// A goal's target instance: its id, where it is and its hit points.
pub type Target = (i32, f64, f64, f64);

/// Where an instance is, to aim for: the centre of its box, or its position without a mask. A tomb lord's
/// position is the top left corner of its 32 px sprite, above the player's head when they stand side by side.
pub fn centre(game: &Game, h: gm::Inst, i: &gm::Instance) -> (f64, f64) {
    let w = game.world();
    if !w.has_mask(h) {
        return (i.x, i.y);
    }
    let b = w.bbox(h);
    (
        f64::from(b.left + b.right + 1) / 2.0,
        f64::from(b.top + b.bottom + 1) / 2.0,
    )
}

/// Whether the instance is of the object `name` or a descendant. A name ending in `$` only counts
/// instances with a price (`cost` above 0): a shop's wares.
pub fn matches(game: &Game, h: gm::Inst, i: &gm::Instance, name: &str) -> bool {
    let (name, for_sale) = name.strip_suffix('$').map_or((name, false), |n| (n, true));
    is_a(game.world().program(), i.object, name)
        && (!for_sale || matches!(var(game, h, "cost"), Some(Value::Real(c)) if c > 0.0))
}

/// Every instance that `matches` `name`, active or deactivated.
pub fn instances<'a>(game: &'a Game, name: &str) -> Vec<(gm::Inst, &'a gm::Instance)> {
    let w = game.world();
    w.active()
        .chain(w.with_state(State::Inactive))
        .map(|h| (h, w.instance(h)))
        .filter(|&(h, i)| matches(game, h, i, name))
        .collect()
}

/// The value of an instance's variable `name`, if it has one.
pub fn var(game: &Game, h: gm::Inst, name: &str) -> Option<Value> {
    let w = game.world();
    let p = w.program();
    let (v, _) = p
        .layout(w.instance(h).object)
        .vars()
        .find(|&(v, _)| p.var_name(v) == name)?;
    w.get(h, v).ok()
}

/// The item the player holds (`oPlayer1.holdItem`), if any.
pub fn held(game: &Game) -> Option<(gm::Inst, &gm::Instance)> {
    let Some(Value::Real(id)) = var(game, player(game)?.0, "holdItem") else {
        return None;
    };
    let w = game.world();
    w.active()
        .map(|h| (h, w.instance(h)))
        .find(|(_, i)| id > 0.0 && f64::from(i.id) == id)
}

/// Prints the level's shops (`oShopkeeper.style`) and their wares, the damsel, dice, crates, chests and rarer
/// finds (the locked chest and its key, Kali's altar...) on stderr, to find seeds that have them.
pub fn report(game: &Game) {
    let mut found: Vec<String> = instances(game, "oShopkeeper")
        .into_iter()
        .map(|(h, _)| match var(game, h, "style") {
            Some(Value::Str(s)) => format!("shop {s}"),
            _ => "shop ?".to_string(),
        })
        .collect();
    let p = game.world().program();
    let mut wares: Vec<&str> = instances(game, "oItem$")
        .into_iter()
        .map(|(_, i)| p.object(i.object).map_or("?", |o| o.name.as_str()))
        .collect();
    wares.sort_unstable();
    if !wares.is_empty() {
        found.push(format!("wares {}", wares.join(" ")));
    }
    for o in [
        "oDamsel",
        "oDice",
        "oLockedChest",
        "oKey",
        "oFlareCrate",
        "oCrystalSkull",
        "oSacAltarLeft",
        "oAnkh",
        "oTombLord",
        // The City of Gold's way (851-3227): the Moai holds the crown, which the gold door needs.
        "oMoai",
        "oGoldDoor",
    ] {
        if !instances(game, o).is_empty() {
            found.push(o.to_string());
        }
    }
    // A teleporter lying free (not a ware, often inside a block) can take the player into the Moai's sealed
    // chamber, as the ankh's revival does (oPlayer1's step).
    if instances(game, "oTeleporter")
        .iter()
        .any(|&(h, _)| !matches!(var(game, h, "cost"), Some(Value::Real(c)) if c > 0.0))
    {
        found.push("free oTeleporter".to_string());
    }
    for o in ["oCrate", "oChest"] {
        let n = instances(game, o).len();
        if n > 0 {
            found.push(format!("{n} {o}"));
        }
    }
    // The level's kind, which decides its rarer enemies: a dark level has scarabs, the Jungle's cemetery
    // zombies and vampires (scrEntityGen), the lake Jaws, the Ice Caves' alien craft its boss.
    for g in ["darkLevel", "cemetary", "lake", "yetiLair", "alienCraft"] {
        if global(game, g).unwrap_or(0.0) != 0.0 {
            found.push(g.to_string());
        }
    }
    // Every enemy (`oEnemy`'s descendants but the shopkeeper, and the ghost and fake bones, which aren't
    // enemies), counted.
    let mut enemies: Vec<(&str, usize)> = Vec::new();
    for (_, i) in instances(game, "oEnemy")
        .into_iter()
        .chain(instances(game, "oGhost"))
        .chain(instances(game, "oFakeBones"))
    {
        let name = p.object(i.object).map_or("?", |o| o.name.as_str());
        if name == "oShopkeeper" {
            continue;
        }
        match enemies.iter_mut().find(|(n, _)| *n == name) {
            Some((_, c)) => *c += 1,
            None => enemies.push((name, 1)),
        }
    }
    enemies.sort_unstable();
    if !enemies.is_empty() {
        let list: Vec<String> = enemies.iter().map(|(n, c)| format!("{c} {n}")).collect();
        found.push(format!("enemies {}", list.join(" ")));
    }
    eprintln!(
        "area-route: level {} ({}, frame {}): {}",
        global(game, "currLevel").unwrap_or(0.0),
        room_name(game),
        game.world().frames(),
        found.join(", ")
    );
}

/// Whether the game is in a level's room: `rLevel3` is the Jungle's lake, `rLevel2` the Ice Caves, and
/// `rOlmec` level 16, after the Temple.
pub fn in_level(game: &Game) -> bool {
    matches!(
        room_name(game).as_str(),
        "rLevel" | "rLevel2" | "rLevel3" | "rOlmec"
    )
}

/// The tile of the Black Market's hidden entrance, if the level has one.
pub fn x_market(game: &Game) -> Option<(i32, i32)> {
    find(game, "oXMarket").map(|i| ((i.x / TILE).floor() as i32, (i.y / TILE).floor() as i32))
}

/// An instance of the object `name`, active or deactivated (out of `oLevel`'s view region).
pub fn find<'a>(game: &'a Game, name: &str) -> Option<&'a gm::Instance> {
    let w = game.world();
    let p = w.program();
    w.active()
        .chain(w.with_state(State::Inactive))
        .map(|h| w.instance(h))
        .find(|i| p.object(i.object).is_some_and(|o| o.name == name))
}

pub fn room_name(game: &Game) -> String {
    let w = game.world();
    w.assets().room_name(w.room().id).unwrap_or("").to_string()
}

pub fn global(game: &Game, name: &str) -> Result<f64, String> {
    let p = game.world().program();
    let i = (0..p.global_count())
        .find(|&i| p.global_name(GVar(i as u16)) == name)
        .ok_or(format!("no global {name}"))?;
    match game.world().gget(GVar(i as u16)) {
        Ok(Value::Real(x)) => Ok(x),
        _ => Ok(0.0),
    }
}

/// The level as a tile grid: what's solid, what can be climbed or stood on, spikes, and the exit's tile.
pub struct Grid {
    pub w: i32,
    pub h: i32,
    pub solid: Vec<bool>,
    /// Ladders and their tops, the Jungle's vines (`oLadder`'s children), and water, which the player swims up through.
    pub ladder: Vec<bool>,
    /// The Jungle's tree branches and leaves (`oPlatform`'s children): stood on, not walls.
    pub platform: Vec<bool>,
    /// Spikes and lava.
    pub spikes: Vec<bool>,
    pub exit: (i32, i32),
    /// More tiles that count as the exit, for a goal with several instances to reach.
    pub more: Vec<(i32, i32)>,
    /// Whether the way to the exit may go through solid tiles, at `DIG` each.
    pub dig: bool,
}

impl Grid {
    pub fn of(game: &Game) -> Result<Grid, String> {
        let g = Grid::scan(game)?;
        if g.exit.0 < 0 {
            return Err("no oExit".into());
        }
        Ok(g)
    }

    /// The grid, with its exit at the `oExit` if there is one (`rOlmec` and `rHighscores` have none).
    pub fn scan(game: &Game) -> Result<Grid, String> {
        let w = game.world();
        let room = w.assets().room(w.room().id).ok_or("no room")?;
        let (gw, gh) = (room.width / 16 + 1, room.height / 16 + 1);
        let n = (gw * gh) as usize;
        let mut g = Grid {
            w: gw,
            h: gh,
            solid: vec![false; n],
            ladder: vec![false; n],
            platform: vec![false; n],
            spikes: vec![false; n],
            exit: (-1, -1),
            more: Vec::new(),
            dig: false,
        };
        let p = w.program();
        for h in w.active().chain(w.with_state(State::Inactive)) {
            let inst = w.instance(h);
            let name = p.object(inst.object).map_or("", |o| o.name.as_str());
            let (tx, ty) = (
                (inst.x / TILE).floor() as i32,
                (inst.y / TILE).floor() as i32,
            );
            let Some(i) = g.idx(tx, ty) else { continue };
            if is_a(p, inst.object, "oSolid") && name != "oPushBlock" {
                g.solid[i] = true;
            } else if name.starts_with("oLadder")
                || is_a(p, inst.object, "oLadder")
                || (is_a(p, inst.object, "oWater") && !is_a(p, inst.object, "oLava"))
            {
                g.ladder[i] = true;
            } else if is_a(p, inst.object, "oPlatform") {
                g.platform[i] = true;
            } else if name == "oSpikes" || is_a(p, inst.object, "oLava") {
                g.spikes[i] = true;
            } else if name == "oExit" {
                g.exit = (tx, ty);
            }
        }
        Ok(g)
    }

    /// The grid whose exit is the Black Market's hidden entrance, inside a solid tile.
    pub fn to_market(game: &Game) -> Result<Grid, String> {
        let mut g = Grid::of(game)?;
        g.exit = x_market(game).ok_or("no oXMarket")?;
        g.dig = true;
        Ok(g)
    }

    /// `to_object`, with the way there allowed through solid tiles (which bombs clear).
    pub fn to_object_dug(game: &Game, name: &str) -> Result<Grid, String> {
        let mut g = Grid::to_object(game, name)?;
        g.dig = true;
        Ok(g)
    }

    /// The grid whose exits are the instances of `name` (and its descendants).
    pub fn to_object(game: &Game, name: &str) -> Result<Grid, String> {
        let mut g = Grid::scan(game)?;
        let mut tiles = instances(game, name)
            .iter()
            .map(|&(h, i)| {
                let (x, y) = centre(game, h, i);
                ((x / TILE).floor() as i32, (y / TILE).floor() as i32)
            })
            // A target in a solid tile (half inside a block) has no moves into it: aim next to it.
            .filter_map(|(x, y)| {
                [(x, y), (x, y - 1), (x - 1, y), (x + 1, y)]
                    .into_iter()
                    .find(|&(x, y)| g.open(x, y))
            })
            .collect::<Vec<_>>()
            .into_iter();
        g.exit = tiles.next().ok_or(format!("no {name}"))?;
        g.more = tiles.collect();
        Ok(g)
    }

    pub fn idx(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && x < self.w && y < self.h).then(|| (y * self.w + x) as usize)
    }

    pub fn open(&self, x: i32, y: i32) -> bool {
        self.idx(x, y).is_some_and(|i| !self.solid[i])
    }

    pub fn ladder(&self, x: i32, y: i32) -> bool {
        self.idx(x, y).is_some_and(|i| self.ladder[i])
    }

    /// The moves out of an open tile, with their costs.
    pub fn moves(&self, x: i32, y: i32) -> Vec<((i32, i32), f64)> {
        let mut m = Vec::new();
        let platform = self.idx(x, y + 1).is_some_and(|i| self.platform[i]);
        let standing =
            !self.open(x, y + 1) || platform || self.ladder(x, y) || self.ladder(x, y + 1);
        if self.open(x, y + 1) {
            m.push(((x, y + 1), 1.0));
        }
        for dx in [-1, 1] {
            if !self.open(x + dx, y) {
                continue;
            }
            if standing {
                m.push(((x + dx, y), 1.0));
            } else if self.open(x + dx, y + 1) {
                m.push(((x + dx, y + 1), 1.5));
            }
        }
        if standing && self.open(x, y - 1) {
            m.push(((x, y - 1), 2.0));
            if self.open(x, y - 2) {
                m.push(((x, y - 2), 3.0));
            }
            for dx in [-1, 1] {
                if self.open(x + dx, y - 1) {
                    m.push(((x + dx, y - 1), 2.5));
                }
                if self.open(x, y - 2) && self.open(x + dx, y - 2) {
                    m.push(((x + dx, y - 2), 3.5));
                }
                // Jump, hang on the ledge of a block two tiles up and climb onto it.
                if self.open(x, y - 2)
                    && self.open(x, y - 3)
                    && !self.open(x + dx, y - 2)
                    && self.open(x + dx, y - 3)
                {
                    m.push(((x + dx, y - 3), 6.0));
                }
            }
        }
        m.into_iter()
            .filter(|&((x, y), _)| self.open(x, y))
            .map(|(t, c)| {
                let spiky = self.idx(t.0, t.1).is_some_and(|i| self.spikes[i]);
                (t, if spiky { c + 50.0 } else { c })
            })
            .collect()
    }

    /// Each tile's cost to reach the exit (Dijkstra over the reversed moves).
    pub fn distances(&self) -> Vec<f64> {
        let n = (self.w * self.h) as usize;
        let mut rev: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
        for y in 0..self.h {
            for x in 0..self.w {
                if !self.open(x, y) {
                    continue;
                }
                let from = self.idx(x, y).unwrap();
                for ((tx, ty), c) in self.moves(x, y) {
                    rev[self.idx(tx, ty).unwrap()].push((from, c));
                }
            }
        }
        if self.dig {
            // Into any solid neighbour, from any tile, but not into the room's indestructible border.
            for y in 0..self.h {
                for x in 0..self.w {
                    let from = self.idx(x, y).unwrap();
                    for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                        let (tx, ty) = (x + dx, y + dy);
                        if tx <= 0 || ty <= 0 || tx >= self.w - 2 || ty >= self.h - 2 {
                            continue;
                        }
                        if let Some(to) = self.idx(tx, ty).filter(|&i| self.solid[i]) {
                            rev[to].push((from, DIG));
                        }
                    }
                }
            }
        }
        let mut dist = vec![f64::INFINITY; n];
        let mut heap = BinaryHeap::new();
        for &(x, y) in std::iter::once(&self.exit).chain(&self.more) {
            let start = self.idx(x, y).unwrap();
            dist[start] = 0.0;
            heap.push(Node(0.0, start));
        }
        while let Some(Node(d, i)) = heap.pop() {
            if d > dist[i] {
                continue;
            }
            for &(j, c) in &rev[i] {
                if d + c < dist[j] {
                    dist[j] = d + c;
                    heap.push(Node(d + c, j));
                }
            }
        }
        dist
    }
}

#[derive(PartialEq)]
pub struct Node(pub f64, pub usize);
impl Eq for Node {}
impl PartialOrd for Node {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Node {
    fn cmp(&self, o: &Self) -> Ordering {
        o.0.total_cmp(&self.0)
    }
}

/// The best way found to reach one cell of the level.
pub struct Cand {
    pub game: Game,
    pub input: InputState,
    pub path: Vec<&'static [u8]>,
    /// Distance to the exit (but see `LIT`).
    pub dist: f64,
    /// Hearts lost, plus half a heart per bomb used on the way to the Black Market.
    pub hearts_lost: f64,
    /// Whether its macros have been tried. The game is deterministic, so a state is expanded once.
    pub expanded: bool,
}

impl Cand {
    /// Which to expand next: near the exit, unhurt.
    pub fn priority(&self) -> f64 {
        self.dist + HEART * self.hearts_lost
    }

    /// Whether this reaches its cell better than `o`: fewer hearts lost, then sooner.
    pub fn better_than(&self, o: &Cand) -> bool {
        (self.hearts_lost, self.path.len()) < (o.hearts_lost, o.path.len())
    }
}

/// Whether `o` is the object `name` or one of its descendants.
pub fn is_a(p: &gm::Program, o: gm::ObjectId, name: &str) -> bool {
    let mut o = Some(o);
    while let Some(id) = o {
        let Some(info) = p.object(id) else {
            return false;
        };
        if info.name == name {
            return true;
        }
        o = info.parent;
    }
    false
}

/// The player, if it exists.
pub fn player(game: &Game) -> Option<(gm::Inst, &gm::Instance)> {
    let w = game.world();
    let p = w.program();
    w.active()
        .map(|h| (h, w.instance(h)))
        .find(|(_, i)| p.object(i.object).is_some_and(|o| o.name == "oPlayer1"))
}

/// An archive cell: the player's 8 px cell, sprite (standing, climbing, hanging, ...), direction of motion
/// and whether jump is held (which makes the jump higher). The cells tell apart states that can go to
/// different places, without telling apart so many that the search drowns in them. On the way to the
/// Black Market, so does the bombing: bombs left, whether one is lit, and how many solid tiles are left;
/// on the way out of `rOlmec`, the ropes: ropes left and how many tiles can be climbed.
pub type Cell = (i32, i32, i32, i8, i8, bool, Bombing);
pub type Bombing = (i8, bool, u16);

/// Searches for macros that take the exit, Go-Explore style: an archive keeps the
/// best way found to reach each 8 px cell, and the search keeps expanding the most promising cell with every
/// macro. Returns the input frame by frame.
pub fn search(
    game: &Game,
    input: &InputState,
    plife: f64,
    budget: usize,
    market: bool,
    goal: &Goal,
    verbose: bool,
) -> Result<Vec<&'static [u8]>, String> {
    // Goals whose way may go through blocks, so that it's measured on the level as bombs leave it.
    let digging = market || matches!(goal, Goal::Dig(_));
    let bombing_goal = digging
        || matches!(
            goal,
            Goal::Kill {
                by: Weapon::Bomb,
                ..
            }
        );
    // A bomb or a throw kills from a distance: near a target, a short window of launches and getaways is
    // tried (`strike`), which the archive's coarse macros would rarely line up.
    let strikes = match goal {
        Goal::Kill {
            object,
            by: by @ (Weapon::Bomb | Weapon::Throw),
        } => Some((object.as_str(), *by)),
        _ => None,
    };
    let mut strikes_left = STRIKES;
    let bombs0 = if bombing_goal {
        global(game, "bombs").unwrap_or(0.0) as i8
    } else {
        0
    };
    let fixed = match goal.object() {
        _ if market => Grid::to_market(game)?,
        Some(o) if digging => Grid::to_object_dug(game, o)?,
        Some(o) => Grid::to_object(game, o)?,
        None => Grid::of(game)?,
    };
    let ids: Vec<Target> = match goal {
        // Not a corpse already on the level, which would count as killed at once.
        Goal::Kill { object, .. } => Goal::alive(game, object),
        _ => goal.object().map_or(Vec::new(), |o| {
            instances(game, o)
                .iter()
                .map(|&(h, i)| {
                    let (x, y) = centre(game, h, i);
                    (i.id, x, y, 0.0)
                })
                .collect()
        }),
    };
    if strikes.is_some_and(|(_, by)| by == Weapon::Throw) && held(game).is_none() {
        return Err("a throw: the player holds nothing".into());
    }
    if goal.carries() && held(game).is_none() {
        return Err("carry: the player holds nothing".into());
    }
    let fixed_dist = fixed.distances();
    let room0 = room_name(game);
    let ropes = matches!(goal, Goal::Door { ropes: true, .. });
    let actions: Vec<(&'static [u8], u64)> = match goal {
        _ if market => ACTIONS.iter().copied().chain([BOMB_ACTION]).collect(),
        // Without the whip, so that the bomb kills.
        Goal::Kill {
            by: Weapon::Bomb, ..
        } => ACTIONS
            .iter()
            .copied()
            .filter(|(k, _)| !k.contains(&ATTACK))
            .chain([BOMB_ACTION])
            .collect(),
        _ if ropes => ACTIONS.iter().copied().chain([ROPE_ACTION]).collect(),
        // Attacking would throw the held item (for a throw kill, `strike` throws it).
        Goal::Carry
        | Goal::Bring(_)
        | Goal::Kill {
            by: Weapon::Throw | Weapon::Stomp,
            ..
        } => ACTIONS
            .iter()
            .copied()
            .filter(|(k, _)| !k.contains(&ATTACK))
            .collect(),
        Goal::Pickup(_) => ACTIONS.iter().copied().chain([PICKUP_ACTION]).collect(),
        Goal::Dig(_) => ACTIONS
            .iter()
            .copied()
            .chain([BOMB_ACTION, PICKUP_ACTION])
            .collect(),
        _ => ACTIONS.to_vec(),
    };
    // The cell, distance and hearts lost of a state; `None` if the player is gone or dead.
    let judge = |g: &Game, inp: &InputState| -> Option<(Cell, f64, f64)> {
        // Bombs and ropes change the level, so the way to the Black Market or out of Olmec's shaft is
        // measured on the level as it is now.
        let live = match goal.object() {
            _ if market => Some(Grid::to_market(g).ok()?),
            Some(o) if digging => Some(Grid::to_object_dug(g, o).ok()?),
            // Enemies move: aim for where they are now.
            Some(o) if ropes || matches!(goal, Goal::Kill { .. }) => {
                Some(Grid::to_object(g, o).ok()?)
            }
            _ => None,
        }
        .map(|grid| {
            let dist = grid.distances();
            (grid, dist)
        });
        let (grid, dist) = live.as_ref().map_or((&fixed, &fixed_dist), |(g, d)| (g, d));
        // Whipping a tough enemy (a caveman, a man trap) takes several hits: the cell tells apart how many
        // hit points the targets have left, and fewer count as nearer.
        let hp_left = match goal {
            Goal::Kill {
                object,
                by: Weapon::Melee | Weapon::Stomp,
            } => target_hp(g, object),
            _ => 0,
        };
        let bombing = match &live {
            Some((grid, _)) if digging => (
                global(g, "bombs").unwrap_or(0.0) as i8,
                find(g, "oBomb").is_some(),
                grid.solid.iter().filter(|&&s| s).count() as u16,
            ),
            _ if bombing_goal => (
                global(g, "bombs").unwrap_or(0.0) as i8,
                find(g, "oBomb").is_some(),
                0,
            ),
            // A thrown rope is a way up from the same cell.
            Some((grid, _)) if ropes => (
                global(g, "rope").unwrap_or(0.0) as i8,
                false,
                grid.ladder.iter().filter(|&&l| l).count() as u16,
            ),
            _ => (0, false, hp_left),
        };
        let (_, p) = player(g)?;
        // Checked between macros only: at the door the game itself lets the damsel go, a few frames before
        // the room changes (she walks out with the player).
        if goal.carries() && held(g).is_none() {
            return None;
        }
        let (x, y) = (p.x, p.y);
        let life = global(g, "plife").ok()?;
        if life < 1.0 {
            return None;
        }
        let (tx, ty) = ((x / TILE).floor() as i32, (y / TILE).floor() as i32);
        let i = grid.idx(tx, ty)?;
        // Where the grid's model sees no route (the game's moves are richer), head for the exit as the
        // crow flies, behind every tile that has one.
        let crow = std::iter::once(&grid.exit)
            .chain(&grid.more)
            .map(|e| (tx - e.0).abs() + (ty - e.1).abs())
            .min()
            .unwrap_or(0);
        let d = if dist[i].is_finite() {
            dist[i]
        } else {
            300.0 + f64::from(crow)
        } + HIT * f64::from(hp_left);
        let cell = (
            (x / 8.0).floor() as i32,
            (y / 8.0).floor() as i32,
            p.sprite_index,
            (x - p.xprevious).signum() as i8,
            (y - p.yprevious).signum() as i8,
            inp.is_down(Button::Key(JUMP)),
            bombing,
        );
        // A bomb costs half a heart (until it goes off, `LIT` outweighs that), so that bombing only pays
        // when it opens the way.
        let bombs_used = f64::from(bombs0 - bombing.0).max(0.0);
        Some((cell, d, (plife - life).max(0.0) + 0.5 * bombs_used))
    };
    let (cell, d, lost) = judge(game, input).ok_or("no player")?;
    let mut archive: HashMap<Cell, Cand> = HashMap::new();
    archive.insert(
        cell,
        Cand {
            game: game.clone(),
            input: *input,
            path: Vec::new(),
            dist: d,
            hearts_lost: lost,
            expanded: false,
        },
    );
    for round in 0..budget {
        let Some((&key, _)) = archive
            .iter()
            .filter(|(_, c)| !c.expanded)
            .min_by(|a, b| a.1.priority().total_cmp(&b.1.priority()).then(a.0.cmp(b.0)))
        else {
            break;
        };
        let from = archive.get_mut(&key).unwrap();
        from.expanded = true;
        let (game0, input0, path0) = (from.game.clone(), from.input, from.path.clone());
        let (dist0, lit0) = (from.dist, key.6.1);
        for &(keys, hold) in &actions {
            if lit0 && keys == BOMB_ACTION.0 {
                continue;
            }
            let (mut g, mut inp, mut path) = (game0.clone(), input0, path0.clone());
            let mut ok = true;
            for _ in 0..hold {
                // A bomb kill counts only if the target dies while the explosion is out, so its targets are
                // the ones alive just before the frame (not one the player whipped before the bomb went off).
                let alive = strikes.map(|(object, _)| Goal::alive(&g, object));
                ok = step(&mut g, &mut inp, keys);
                path.push(keys);
                if !ok || room_name(&g) != room0 {
                    break;
                }
                if goal.met(&g, alive.as_deref().unwrap_or(&ids)) {
                    return Ok(path);
                }
            }
            if !ok {
                continue;
            }
            if room_name(&g) != room0 {
                // The exit's room change, unless the player died (then it's the death screen) or took the
                // level's exit on the way to the Black Market.
                let room = room_name(&g);
                let taken = match goal {
                    Goal::Door { room: to, .. } => room == *to,
                    _ => {
                        room.starts_with("rTransition")
                            && goal.is_exit()
                            && (!market || global(&g, "genBlackMarket").is_ok_and(|b| b != 0.0))
                    }
                };
                if taken {
                    return Ok(path);
                }
                continue;
            }
            let Some((cell, mut d, lost)) = judge(&g, &inp) else {
                continue;
            };
            // From each new cell near a target, with no bomb already lit.
            if let Some((object, by)) = strikes
                && strikes_left > 0
                && !archive.contains_key(&cell)
                && !cell.6.1
                && armed(&g, by)
                && near_target(&g, object)
            {
                strikes_left -= 1;
                if let Some(tail) = strike(&g, inp, goal, object, by) {
                    path.extend(tail);
                    return Ok(path);
                }
            }
            // While a bomb is lit, the state counts as near as where it was lit, and a little nearer (more so
            // the later it is), so that the search follows it to its explosion, running from it, before it
            // tries anything else.
            if cell.6.1 {
                d = if lit0 { dist0 } else { dist0 - LIT } - 0.01 * hold as f64;
            }
            let new = Cand {
                game: g,
                input: inp,
                path,
                dist: d,
                hearts_lost: lost,
                expanded: false,
            };
            match archive.get_mut(&cell) {
                Some(old) if !new.better_than(old) => {}
                Some(old) => *old = new,
                None => {
                    archive.insert(cell, new);
                }
            }
        }
        if verbose && round % 200 == 0 {
            let best = archive
                .values()
                .map(|c| c.dist)
                .fold(f64::INFINITY, f64::min);
            eprintln!(
                "area-route: round {round}, {} cells, nearest {best:.1}",
                archive.len()
            );
        }
    }
    // What was explored, to see where it got stuck: `o` reached, `.` and `,` open with and without a
    // route to the exit `E` in the grid's model.
    for y in (0..fixed.h).filter(|_| verbose) {
        let row: String = (0..fixed.w)
            .map(|x| {
                let i = fixed.idx(x, y).unwrap();
                match () {
                    _ if (x, y) == fixed.exit || fixed.more.contains(&(x, y)) => 'E',
                    _ if archive.keys().any(|c| (c.0 / 2, c.1 / 2) == (x, y)) => 'o',
                    _ if fixed.solid[i] => '#',
                    _ if fixed.ladder[i] => 'H',
                    _ if fixed.spikes[i] => '^',
                    _ if fixed_dist[i].is_finite() => '.',
                    _ => ',',
                }
            })
            .collect();
        eprintln!("{row}");
    }
    Err(format!(
        "no route within {budget} expansions ({} cells)",
        archive.len()
    ))
}

/// How many new cells near a target `strike` is tried from, per search.
const STRIKES: usize = 150;
/// How near (in x, and in y) a target's middle (16 px enemies have their origin at the top left) `strike`
/// tries from.
const STRIKE_X: f64 = 96.0;
const STRIKE_Y: f64 = 64.0;
/// The launches `strike` tries: a bomb thrown (ahead, both ways, up) or dropped, or the held item thrown
/// (ahead, both ways, up, from a jump).
const BOMB_LAUNCHES: &[&[u8]] = &[
    &[BOMB],
    &[LEFT, BOMB],
    &[RIGHT, BOMB],
    &[UP, BOMB],
    &[DOWN, BOMB],
];
const THROWS: &[&[u8]] = &[
    &[ATTACK],
    &[LEFT, ATTACK],
    &[RIGHT, ATTACK],
    &[UP, ATTACK],
    &[JUMP, ATTACK],
];
/// What `strike` does after the launch: stand, or run (and jump) either way, or climb.
const GETAWAYS: &[&[u8]] = &[
    &[],
    &[LEFT],
    &[RIGHT],
    &[LEFT, JUMP],
    &[RIGHT, JUMP],
    &[UP],
    &[DOWN],
];
/// A bomb goes off 120 frames after it's lit (`oBomb`'s alarms 0 and 1).
const FUSE: u64 = 120;
/// How long `strike` follows a launch: a bomb's fuse, or a throw's flight, and the `AFTERMATH`.
const BOMB_FRAMES: u64 = FUSE + 10 + AFTERMATH;
const THROW_FRAMES: u64 = 60 + AFTERMATH;
/// How long the player must survive the kill.
const AFTERMATH: u64 = 20;

/// Whether the player has what `strike` launches: a bomb, or an item to throw.
fn armed(g: &Game, by: Weapon) -> bool {
    match by {
        Weapon::Bomb => global(g, "bombs").unwrap_or(0.0) >= 1.0,
        Weapon::Throw => held(g).is_some(),
        Weapon::Melee | Weapon::Stomp => false,
    }
}

/// The instance with this id, active or deactivated.
fn find_id(g: &Game, id: i32) -> Option<&gm::Instance> {
    let w = g.world();
    w.active()
        .chain(w.with_state(State::Inactive))
        .map(|h| w.instance(h))
        .find(|i| i.id == id)
}

/// Whether a target of `object` is near enough the player for `strike`.
fn near_target(g: &Game, object: &str) -> bool {
    player(g).is_some_and(|(_, p)| {
        Goal::alive(g, object)
            .iter()
            .any(|t| (t.1 + 8.0 - p.x).abs() <= STRIKE_X && (t.2 + 8.0 - p.y).abs() <= STRIKE_Y)
    })
}

/// From a state near a target of a bomb or throw kill, tries each launch, held for 2 frames, then each
/// getaway, and returns the input of the first that kills it with the player surviving `AFTERMATH`
/// frames. A bomb launch is first followed standing still: unless a target dies then, whatever becomes of
/// the player, the getaways aren't tried (they change little but whether the player survives).
fn strike(
    g0: &Game,
    input: InputState,
    goal: &Goal,
    object: &str,
    by: Weapon,
) -> Option<Vec<&'static [u8]>> {
    let (launches, frames, from) = match by {
        // The goal is only checked from when a bomb can go off.
        Weapon::Bomb => (BOMB_LAUNCHES, BOMB_FRAMES, FUSE - 10),
        Weapon::Throw => (THROWS, THROW_FRAMES, 0),
        Weapon::Melee | Weapon::Stomp => return None,
    };
    // For a throw, the item thrown must be the one that hits (not a trap's arrow): where it last was.
    let item = held(g0).map(|(_, i)| i.id);
    let room0 = room_name(g0);
    // Follows `launch` then `away` from `g1`: the input up to `AFTERMATH` frames after the kill, if the
    // player survives it (or, on a `trial`, up to a target's death, whatever becomes of the player).
    let run = |g1: &Game,
               in1: InputState,
               head: &[&'static [u8]],
               launch: &'static [u8],
               away: &'static [u8],
               trial: bool| {
        let (mut g, mut inp, mut path) = (g1.clone(), in1, head.to_vec());
        let mut killed_at = None;
        let mut item_at = None;
        for t in 0..frames {
            let keys = if t < 2 { launch } else { away };
            let alive = (t >= from).then(|| Goal::alive(&g, object));
            if let Some(id) = item {
                // A jar breaks on what it hits: the place before the frame counts.
                item_at = find_id(&g, id).map(|i| (i.x, i.y)).or(item_at);
            }
            if !step(&mut g, &mut inp, keys) || room_name(&g) != room0 {
                return None;
            }
            path.push(keys);
            if let Some(alive) = alive {
                if trial {
                    let now = Goal::alive(&g, object);
                    if alive.iter().any(|a| now.iter().all(|n| n.0 != a.0)) {
                        return Some(path);
                    }
                } else if killed_at.is_none()
                    && goal.met(&g, &alive)
                    && (by != Weapon::Throw
                        || item_at.is_some_and(|(ix, iy)| {
                            alive
                                .iter()
                                .any(|a| (a.1 - ix).abs() <= BLAST && (a.2 - iy).abs() <= BLAST)
                        }))
                {
                    killed_at = Some(t);
                }
            }
            if !trial && global(&g, "plife").unwrap_or(0.0) < 1.0 {
                return None;
            }
            if killed_at.is_some_and(|k| t >= k + AFTERMATH) {
                return Some(path);
            }
        }
        None
    };
    for &launch in launches {
        // A launch is a key press: let go of it first if it's held.
        let (mut g1, mut in1) = (g0.clone(), input);
        let mut head: Vec<&'static [u8]> = Vec::new();
        if launch.iter().any(|&k| input.is_down(Button::Key(k))) {
            if !step(&mut g1, &mut in1, &[]) {
                continue;
            }
            head.push(&[]);
        }
        if by == Weapon::Bomb && run(&g1, in1, &head, launch, &[], true).is_none() {
            continue;
        }
        for &away in GETAWAYS {
            if let Some(path) = run(&g1, in1, &head, launch, away, false) {
                return Some(path);
            }
        }
    }
    None
}

/// The hit points the living instances of `object` have left, together.
fn target_hp(g: &Game, object: &str) -> u16 {
    let hp: f64 = instances(g, object)
        .into_iter()
        .filter_map(|(h, _)| match var(g, h, "hp") {
            Some(Value::Real(hp)) if hp >= 1.0 => Some(hp),
            _ => None,
        })
        .sum();
    hp.clamp(0.0, f64::from(u16::MAX)) as u16
}

#[cfg(test)]
mod tests {
    use super::{Goal, Weapon};

    #[test]
    fn parses_goals() {
        assert_eq!(Goal::parse("exit"), Goal::Exit);
        assert_eq!(Goal::parse("oDamsel"), Goal::Reach("oDamsel".into()));
        assert_eq!(Goal::parse("+oItem$"), Goal::Pickup("oItem$".into()));
        assert_eq!(Goal::parse("!oKey"), Goal::Dig("oKey".into()));
        assert_eq!(
            Goal::parse("^oCaveman"),
            Goal::Kill {
                object: "oCaveman".into(),
                by: Weapon::Stomp
            }
        );
        assert_eq!(Goal::parse(">oGoldDoor"), Goal::Bring("oGoldDoor".into()));
        assert_eq!(
            Goal::parse("-oSnake"),
            Goal::Kill {
                object: "oSnake".into(),
                by: Weapon::Melee
            }
        );
        assert_eq!(
            Goal::parse("*oBat"),
            Goal::Kill {
                object: "oBat".into(),
                by: Weapon::Bomb
            }
        );
        assert_eq!(
            Goal::parse("/oCaveman"),
            Goal::Kill {
                object: "oCaveman".into(),
                by: Weapon::Throw
            }
        );
    }
}
