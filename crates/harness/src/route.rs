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
    /// Hold an instance of the object (or a descendant).
    Pickup(String),
    /// Kill an instance of the object (or a descendant) by whipping or stomping it, or with `bombs` by
    /// blowing it up with a bomb dropped at the player's feet.
    Kill {
        object: String,
        bombs: bool,
    },
    /// Take a door that isn't a level's exit, `object`, into `room`, throwing ropes if `ropes`.
    Door {
        object: &'static str,
        room: &'static str,
        ropes: bool,
    },
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
                ("-", o) => Goal::Kill {
                    object: o.to_string(),
                    bombs: false,
                },
                ("*", o) => Goal::Kill {
                    object: o.to_string(),
                    bombs: true,
                },
                _ => Goal::Reach(s.to_string()),
            },
        }
    }

    pub fn is_exit(&self) -> bool {
        matches!(self, Goal::Exit | Goal::Carry)
    }

    /// The object whose instances are the targets, for the goals that aren't the exit.
    pub fn object(&self) -> Option<&str> {
        match self {
            Goal::Reach(o) | Goal::Pickup(o) | Goal::Kill { object: o, .. } => Some(o),
            Goal::Door { object, .. } => Some(object),
            Goal::Exit | Goal::Carry => None,
        }
    }

    /// Whether the goal is met in `g` with the player alive, where `ids` are the instances of its object at
    /// the start, or for a bomb kill the ones alive just before this frame, and where they were.
    pub fn met(&self, g: &Game, ids: &[(i32, f64, f64)]) -> bool {
        let Some((_, p)) = player(g) else {
            return false;
        };
        if global(g, "plife").unwrap_or(0.0) < 1.0 {
            return false;
        }
        match self {
            Goal::Exit | Goal::Carry | Goal::Door { .. } => false,
            Goal::Reach(o) => {
                let now = instances(g, o);
                // Treasure is collected on touch: one of the targets is gone.
                ids.iter()
                    .any(|(id, ..)| now.iter().all(|(_, i)| i.id != *id))
                    || now
                        .iter()
                        .any(|(_, i)| (i.x - p.x).abs() <= 10.0 && (i.y - p.y).abs() <= 12.0)
            }
            Goal::Pickup(o) => held(g).is_some_and(|(h, i)| matches(g, h, i, o)),
            // Dead: destroyed (or turned into another object, as a hanging spider drops), or a corpse.
            Goal::Kill { object, bombs } => {
                let now = Goal::alive(g, object);
                let explosions: Vec<(f64, f64)> = instances(g, "oExplosion")
                    .iter()
                    .map(|(_, e)| (e.x, e.y))
                    .collect();
                ids.iter().any(|&(id, x, y)| {
                    now.iter().all(|&(i, ..)| i != id)
                        // A bomb kill: the target died beside an explosion, not to the whip while one was out.
                        && (!bombs
                            || explosions
                                .iter()
                                .any(|&(ex, ey)| (ex - x).abs() <= BLAST && (ey - y).abs() <= BLAST))
                })
            }
        }
    }

    /// The instances of `object` (or a descendant) that are alive (not corpses, such as a caveman's or a
    /// yeti's, out of hit points), and where they are.
    pub fn alive(g: &Game, object: &str) -> Vec<(i32, f64, f64)> {
        instances(g, object)
            .into_iter()
            .filter(|&(h, _)| !matches!(var(g, h, "hp"), Some(Value::Real(hp)) if hp < 1.0))
            .map(|(_, i)| (i.id, i.x, i.y))
            .collect()
    }
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
    ] {
        if !instances(game, o).is_empty() {
            found.push(o.to_string());
        }
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

    /// The grid whose exits are the instances of `name` (and its descendants).
    pub fn to_object(game: &Game, name: &str) -> Result<Grid, String> {
        let mut g = Grid::scan(game)?;
        let mut tiles = instances(game, name)
            .iter()
            .map(|(_, i)| ((i.x / TILE).floor() as i32, (i.y / TILE).floor() as i32))
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
    let bombing_goal = market || matches!(goal, Goal::Kill { bombs: true, .. });
    let bombs0 = if bombing_goal {
        global(game, "bombs").unwrap_or(0.0) as i8
    } else {
        0
    };
    let fixed = match goal.object() {
        _ if market => Grid::to_market(game)?,
        Some(o) => Grid::to_object(game, o)?,
        None => Grid::of(game)?,
    };
    let ids: Vec<(i32, f64, f64)> = goal.object().map_or(Vec::new(), |o| {
        instances(game, o)
            .iter()
            .map(|(_, i)| (i.id, i.x, i.y))
            .collect()
    });
    if *goal == Goal::Carry && held(game).is_none() {
        return Err("carry: the player holds nothing".into());
    }
    let fixed_dist = fixed.distances();
    let room0 = room_name(game);
    let ropes = matches!(goal, Goal::Door { ropes: true, .. });
    let actions: Vec<(&'static [u8], u64)> = match goal {
        _ if market => ACTIONS.iter().copied().chain([BOMB_ACTION]).collect(),
        // Without the whip, so that the bomb kills.
        Goal::Kill { bombs: true, .. } => ACTIONS
            .iter()
            .copied()
            .filter(|(k, _)| !k.contains(&ATTACK))
            .chain([BOMB_ACTION])
            .collect(),
        _ if ropes => ACTIONS.iter().copied().chain([ROPE_ACTION]).collect(),
        // Attacking would throw the held item.
        Goal::Carry => ACTIONS
            .iter()
            .copied()
            .filter(|(k, _)| !k.contains(&ATTACK))
            .collect(),
        Goal::Pickup(_) => ACTIONS.iter().copied().chain([PICKUP_ACTION]).collect(),
        _ => ACTIONS.to_vec(),
    };
    // The cell, distance and hearts lost of a state; `None` if the player is gone or dead.
    let judge = |g: &Game, inp: &InputState| -> Option<(Cell, f64, f64)> {
        // Bombs and ropes change the level, so the way to the Black Market or out of Olmec's shaft is
        // measured on the level as it is now.
        let live = match goal.object() {
            _ if market => Some(Grid::to_market(g).ok()?),
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
                bombs: false,
            } => target_hp(g, object),
            _ => 0,
        };
        let bombing = match &live {
            Some((grid, _)) if market => (
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
        if *goal == Goal::Carry && held(g).is_none() {
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
                let alive = match goal {
                    Goal::Kill {
                        object,
                        bombs: true,
                    } => Some(Goal::alive(&g, object)),
                    _ => None,
                };
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
    use super::Goal;

    #[test]
    fn parses_goals() {
        assert_eq!(Goal::parse("exit"), Goal::Exit);
        assert_eq!(Goal::parse("oDamsel"), Goal::Reach("oDamsel".into()));
        assert_eq!(Goal::parse("+oItem$"), Goal::Pickup("oItem$".into()));
        assert_eq!(
            Goal::parse("-oSnake"),
            Goal::Kill {
                object: "oSnake".into(),
                bombs: false
            }
        );
        assert_eq!(
            Goal::parse("*oBat"),
            Goal::Kill {
                object: "oBat".into(),
                bombs: true
            }
        );
    }
}
