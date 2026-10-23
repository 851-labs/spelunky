//! Finds input that takes the player through the next levels on the port and writes it as a replay, so the
//! oracle can be compared on levels a standing player never reaches (levels 2-4 of the Mines, 6-8 of the
//! Jungle, 14-16 of the Temple, the Black Market). The game has no level-select or debug key that would
//! get there (`oGame`'s only key event is `M`, music).
//!
//! It plays the given replay (one that ends in a level, such as `fidelity/gen-area.sh`'s), then for each
//! level searches held-key macros until the player takes the exit, skips the transition room with the
//! attack key, and finally stands still in the last level. The search is best-first over an archive of
//! cells (Go-Explore without the randomness): it keeps the best state found in each cell and expands the
//! one nearest the exit by a tile-grid distance, with a penalty per heart lost. With `--market`, a level
//! with the Black Market's hidden entrance (an `oXMarket` exit inside a block) aims for it instead,
//! counting solid tiles as passable at a cost, and the search may drop bombs; if that fails, it takes the
//! exit. The route is only as good as the port's fidelity: `scripts/fidelity` on the result says whether
//! the oracle follows it.
//!
//! `--steps` gives other goals than exits, in order, for replays that get to a level's characters: reach
//! an object (a shopkeeper, a damsel, treasure, which is picked up on touch), pick up an item, or take the
//! exit carrying it (the damsel, for her kiss). Every instance of the object is a target.
//!
//! With `--olmec`, it then fights Olmec in `rOlmec` (level 16): a second search, over the same kind of
//! archive, lures Olmec into digging through the floor into the lava (`beat_olmec`). The ordinary search
//! then takes the door `oFinalBoss` opens, throwing ropes to climb out of Olmec's shaft, and the ending
//! plays through to the credits, `rHighscores` and the title (`play_ending`). `--title` only walks out of
//! `rHighscores`, for a replay that ends there after a death.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use game::Game;
use game::replay::Playback;
use gm::{GVar, State, Value};
use trace::Replay;
use trace::replay::{Button, Event, InputState};

const USAGE: &str = "\
usage: area-route [options] <replay.txt>

Plays the replay on the port, which must end in a level, then searches for input that takes the player
through the next <levels> levels' exits and writes the extended replay on stdout. After the last exit
it skips the transition and stands still for <wait> frames in the new level.

options:
  --assets <dir>   the asset bundle (default: assets)
  --market         on a level with the Black Market's hidden entrance (oXMarket, in the Jungle), bomb
                   through to it and take it instead of the exit, so that the next level is the market
  --levels <n>     how many exits to take (default 3: from level 1 to level 4)
  --wait <n>       frames to stand still in the last level (default 300)
  --budget <n>     states to expand per level before giving up (default 8000)
  --steps <list>   comma-separated goals instead of <levels> exits, in order: `exit` takes the exit
                   (and skips the transition), `carry` takes it without letting go of the held item,
                   `<object>` reaches an instance of the object or a descendant (or collects one) and
                   `+<object>` picks one up. Each level's shops and damsel are printed on stderr.
  --olmec          then, in rOlmec (after the levels and the wait), drown Olmec, take the door to the
                   ending, play through the ending and the credits, and walk out of rHighscores to the title
  --title          then walk out of rHighscores to the title (for a replay that ends there)

On failure it prints what it explored on stderr: o reached, # wall, H ladder, ^ spikes, E the exit, and .
or , open tiles from which the grid's model of the moves does or doesn't see a way to the exit.
  -h, --help";

const LEFT: u8 = 37;
const UP: u8 = 38;
const RIGHT: u8 = 39;
const DOWN: u8 = 40;
const JUMP: u8 = 90; // Z
const ATTACK: u8 = 88; // X
const BOMB: u8 = 65; // A
const ROPE: u8 = 83; // S

/// The macros the search tries: keys held for some frames. The long ones make progress the archive's cells
/// don't show, such as pushing a block or climbing.
const ACTIONS: &[(&[u8], u64)] = &[
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
const BOMB_ACTION: (&[u8], u64) = (&[DOWN, BOMB], 6);
/// The macro the way out of `rOlmec` adds: throw a rope, to climb out of the shaft Olmec dug.
const ROPE_ACTION: (&[u8], u64) = (&[ROPE], 6);
/// The macro `+<object>` adds: attack while ducking picks up what's at the player's feet.
const PICKUP_ACTION: (&[u8], u64) = (&[DOWN, ATTACK], 6);
const TILE: f64 = 16.0;
/// Distance-field cost of a heart lost.
const HEART: f64 = 30.0;
/// Distance-field cost of a solid tile on the way to the Black Market's entrance, which only bombs clear.
const DIG: f64 = 12.0;
/// How much nearer the exit a state with a lit bomb counts than where the bomb was lit.
const LIT: f64 = 20.0;

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("area-route: {e}");
            ExitCode::from(2)
        }
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let (mut assets, mut levels, mut wait, mut budget) = (PathBuf::from("assets"), 3, 300u64, 8000);
    let mut market = false;
    let mut olmec = false;
    let mut title = false;
    let mut steps: Option<Vec<Goal>> = None;
    let mut replay_path = None;
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        let mut num = |name: &str| -> Result<u64, String> {
            it.next()
                .and_then(|v| v.parse().ok())
                .ok_or(format!("{name} takes a number"))
        };
        match a.as_str() {
            "--assets" => assets = PathBuf::from(it.next().ok_or("--assets takes a dir")?),
            "--levels" => levels = num("--levels")?,
            "--wait" => wait = num("--wait")?,
            "--budget" => budget = num("--budget")? as usize,
            "--market" => market = true,
            "--olmec" => olmec = true,
            "--title" => title = true,
            "--steps" => {
                let list = it.next().ok_or("--steps takes a list")?;
                steps = Some(list.split(',').map(Goal::parse).collect());
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(());
            }
            p if !p.starts_with('-') => replay_path = Some(PathBuf::from(p)),
            _ => return Err(format!("unknown option {a}\n{USAGE}")),
        }
    }
    let replay_path = replay_path.ok_or(USAGE)?;
    if olmec && title {
        return Err(
            "--olmec already walks out to the title: give one of --olmec and --title".into(),
        );
    }
    let base = harness::load_replay(&replay_path)?;
    let assets = harness::load_assets(Path::new(&assets))?;
    let mut playback = Playback::boot(assets, &base).map_err(|e| e.message)?;
    while playback.step().map_err(|e| e.message)?.is_some() {}
    let mut game = playback.into_game();
    let mut out = Replay::new(base.header.clone());
    for &(f, e) in base.events() {
        out.push(f, e).map_err(|e| e.to_string())?;
    }
    let mut input = base.inputs().last().map(|i| i.state).unwrap_or_default();
    let steps = steps.unwrap_or_else(|| (0..levels).map(|_| Goal::Exit).collect());
    report(&game);
    let mut exits = 0;
    for goal in &steps {
        if !in_level(&game) {
            return Err(format!("not in a level but in {}", room_name(&game)));
        }
        let t = std::time::Instant::now();
        let to_market = market
            && *goal == Goal::Exit
            && global(&game, "genBlackMarket")? == 0.0
            && x_market(&game).is_some();
        if to_market {
            eprintln!(
                "area-route: level {} has the Black Market's entrance",
                exits + 1
            );
        }
        let plife = global(&game, "plife")?;
        let path = match search(&game, &input, plife, budget, to_market, goal) {
            Err(e) if to_market => {
                eprintln!("area-route: no way into the Black Market ({e}), taking the exit");
                search(&game, &input, plife, budget, false, goal)?
            }
            r => r?,
        };
        for keys in path {
            apply(&mut game, &mut input, &mut out, keys)?;
        }
        if !goal.is_exit() {
            eprintln!(
                "area-route: {goal:?} done at frame {} ({:.1} s)",
                game.world().frames(),
                t.elapsed().as_secs_f64()
            );
            continue;
        }
        exits += 1;
        eprintln!(
            "area-route: exit {exits} taken at frame {} ({:.1} s)",
            game.world().frames(),
            t.elapsed().as_secs_f64()
        );
        // Skip the transition: tap attack until a level loads.
        let mut n = 0;
        while !in_level(&game) {
            let keys: &[u8] = if n % 8 < 2 { &[ATTACK] } else { &[] };
            apply(&mut game, &mut input, &mut out, keys)?;
            n += 1;
            if n > 3000 {
                return Err(format!("stuck in {}", room_name(&game)));
            }
        }
        report(&game);
    }
    for _ in 0..wait {
        apply(&mut game, &mut input, &mut out, &[])?;
    }
    if olmec {
        if room_name(&game) != "rOlmec" {
            return Err(format!(
                "--olmec: not in rOlmec but in {}",
                room_name(&game)
            ));
        }
        let t = std::time::Instant::now();
        let plife = global(&game, "plife")?;
        for keys in beat_olmec(&game, &input, plife, budget)? {
            apply(&mut game, &mut input, &mut out, keys)?;
        }
        eprintln!(
            "area-route: Olmec dead at frame {} ({:.1} s)",
            game.world().frames(),
            t.elapsed().as_secs_f64()
        );
        let plife = global(&game, "plife")?;
        for keys in search(&game, &input, plife, budget, false, &Goal::END)? {
            apply(&mut game, &mut input, &mut out, keys)?;
        }
        eprintln!(
            "area-route: ending door taken at frame {}",
            game.world().frames()
        );
        play_ending(&mut game, &mut input, &mut out, budget)?;
    } else if title {
        to_title(&mut game, &mut input, &mut out, budget)?;
    }
    out.set_len(game.world().frames())
        .map_err(|e| e.to_string())?;
    print!("{}", out.to_text());
    Ok(())
}

/// Runs one frame with only `keys` down, recording the events that lead there.
fn apply(
    game: &mut Game,
    input: &mut InputState,
    out: &mut Replay,
    keys: &[u8],
) -> Result<(), String> {
    let next = InputState::keys(keys.iter().copied());
    let frame = game.world().frames();
    let events = input.events_to(&next);
    for &e in &events {
        out.push(frame, e).map_err(|e| e.to_string())?;
    }
    *input = next;
    game.step(events.iter()).map_err(|e| game::describe(&e))
}

fn step(game: &mut Game, input: &mut InputState, keys: &[u8]) -> bool {
    let next = InputState::keys(keys.iter().copied());
    let events: Vec<Event> = input.events_to(&next);
    *input = next;
    game.step(events.iter()).is_ok()
}

/// A goal of `--steps`.
#[derive(Debug, PartialEq)]
enum Goal {
    Exit,
    /// The exit, holding on to the held item.
    Carry,
    /// Touch an instance of the object (or a descendant), or see one disappear (collected).
    Reach(String),
    /// Hold an instance of the object (or a descendant).
    Pickup(String),
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
    const END: Goal = Goal::Door {
        object: "oXEnd",
        room: "rEnd",
        ropes: true,
    };
    /// The door back to the title in `rHighscores`.
    const TITLE: Goal = Goal::Door {
        object: "oXTitle",
        room: "rTitle",
        ropes: false,
    };

    fn parse(s: &str) -> Goal {
        match s {
            "exit" => Goal::Exit,
            "carry" => Goal::Carry,
            _ => match s.strip_prefix('+') {
                Some(o) => Goal::Pickup(o.to_string()),
                None => Goal::Reach(s.to_string()),
            },
        }
    }

    fn is_exit(&self) -> bool {
        matches!(self, Goal::Exit | Goal::Carry)
    }

    /// The object whose instances are the targets, for the goals that aren't the exit.
    fn object(&self) -> Option<&str> {
        match self {
            Goal::Reach(o) | Goal::Pickup(o) => Some(o),
            Goal::Door { object, .. } => Some(object),
            Goal::Exit | Goal::Carry => None,
        }
    }

    /// Whether the goal is met in `g` with the player alive, where `ids` are the instances of its object at
    /// the start.
    fn met(&self, g: &Game, ids: &[i32]) -> bool {
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
                ids.iter().any(|id| now.iter().all(|(_, i)| i.id != *id))
                    || now
                        .iter()
                        .any(|(_, i)| (i.x - p.x).abs() <= 10.0 && (i.y - p.y).abs() <= 12.0)
            }
            Goal::Pickup(o) => held(g).is_some_and(|(h, i)| matches(g, h, i, o)),
        }
    }
}

/// Whether the instance is of the object `name` or a descendant. A name ending in `$` only counts
/// instances with a price (`cost` above 0): a shop's wares.
fn matches(game: &Game, h: gm::Inst, i: &gm::Instance, name: &str) -> bool {
    let (name, for_sale) = name.strip_suffix('$').map_or((name, false), |n| (n, true));
    is_a(game.world().program(), i.object, name)
        && (!for_sale || matches!(var(game, h, "cost"), Some(Value::Real(c)) if c > 0.0))
}

/// Every instance that `matches` `name`, active or deactivated.
fn instances<'a>(game: &'a Game, name: &str) -> Vec<(gm::Inst, &'a gm::Instance)> {
    let w = game.world();
    w.active()
        .chain(w.with_state(State::Inactive))
        .map(|h| (h, w.instance(h)))
        .filter(|&(h, i)| matches(game, h, i, name))
        .collect()
}

/// The value of an instance's variable `name`, if it has one.
fn var(game: &Game, h: gm::Inst, name: &str) -> Option<Value> {
    let w = game.world();
    let p = w.program();
    let (v, _) = p
        .layout(w.instance(h).object)
        .vars()
        .find(|&(v, _)| p.var_name(v) == name)?;
    w.get(h, v).ok()
}

/// The item the player holds (`oPlayer1.holdItem`), if any.
fn held(game: &Game) -> Option<(gm::Inst, &gm::Instance)> {
    let Some(Value::Real(id)) = var(game, player(game)?.0, "holdItem") else {
        return None;
    };
    let w = game.world();
    w.active()
        .map(|h| (h, w.instance(h)))
        .find(|(_, i)| id > 0.0 && f64::from(i.id) == id)
}

/// Prints the level's shops (`oShopkeeper.style`), damsel and dice on stderr, to find seeds that have them.
fn report(game: &Game) {
    let mut found: Vec<String> = instances(game, "oShopkeeper")
        .into_iter()
        .map(|(h, _)| match var(game, h, "style") {
            Some(Value::Str(s)) => format!("shop {s}"),
            _ => "shop ?".to_string(),
        })
        .collect();
    for o in ["oDamsel", "oDice"] {
        if !instances(game, o).is_empty() {
            found.push(o.to_string());
        }
    }
    eprintln!(
        "area-route: level {} ({}): {}",
        global(game, "currLevel").unwrap_or(0.0),
        room_name(game),
        found.join(", ")
    );
}

/// Whether the game is in a level's room: `rLevel3` is the Jungle's lake, `rLevel2` the Ice Caves, and
/// `rOlmec` level 16, after the Temple.
fn in_level(game: &Game) -> bool {
    matches!(
        room_name(game).as_str(),
        "rLevel" | "rLevel2" | "rLevel3" | "rOlmec"
    )
}

/// The tile of the Black Market's hidden entrance, if the level has one.
fn x_market(game: &Game) -> Option<(i32, i32)> {
    find(game, "oXMarket").map(|i| ((i.x / TILE).floor() as i32, (i.y / TILE).floor() as i32))
}

/// An instance of the object `name`, active or deactivated (out of `oLevel`'s view region).
fn find<'a>(game: &'a Game, name: &str) -> Option<&'a gm::Instance> {
    let w = game.world();
    let p = w.program();
    w.active()
        .chain(w.with_state(State::Inactive))
        .map(|h| w.instance(h))
        .find(|i| p.object(i.object).is_some_and(|o| o.name == name))
}

fn room_name(game: &Game) -> String {
    let w = game.world();
    w.assets().room_name(w.room().id).unwrap_or("").to_string()
}

fn global(game: &Game, name: &str) -> Result<f64, String> {
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
struct Grid {
    w: i32,
    h: i32,
    solid: Vec<bool>,
    /// Ladders and their tops, the Jungle's vines (`oLadder`'s children), and water, which the player swims up through.
    ladder: Vec<bool>,
    /// The Jungle's tree branches and leaves (`oPlatform`'s children): stood on, not walls.
    platform: Vec<bool>,
    /// Spikes and lava.
    spikes: Vec<bool>,
    exit: (i32, i32),
    /// More tiles that count as the exit, for a goal with several instances to reach.
    more: Vec<(i32, i32)>,
    /// Whether the way to the exit may go through solid tiles, at `DIG` each.
    dig: bool,
}

impl Grid {
    fn of(game: &Game) -> Result<Grid, String> {
        let g = Grid::scan(game)?;
        if g.exit.0 < 0 {
            return Err("no oExit".into());
        }
        Ok(g)
    }

    /// The grid, with its exit at the `oExit` if there is one (`rOlmec` and `rHighscores` have none).
    fn scan(game: &Game) -> Result<Grid, String> {
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
    fn to_market(game: &Game) -> Result<Grid, String> {
        let mut g = Grid::of(game)?;
        g.exit = x_market(game).ok_or("no oXMarket")?;
        g.dig = true;
        Ok(g)
    }

    /// The grid whose exits are the instances of `name` (and its descendants).
    fn to_object(game: &Game, name: &str) -> Result<Grid, String> {
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

    fn idx(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && x < self.w && y < self.h).then(|| (y * self.w + x) as usize)
    }

    fn open(&self, x: i32, y: i32) -> bool {
        self.idx(x, y).is_some_and(|i| !self.solid[i])
    }

    fn ladder(&self, x: i32, y: i32) -> bool {
        self.idx(x, y).is_some_and(|i| self.ladder[i])
    }

    /// The moves out of an open tile, with their costs.
    fn moves(&self, x: i32, y: i32) -> Vec<((i32, i32), f64)> {
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
    fn distances(&self) -> Vec<f64> {
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
struct Node(f64, usize);
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
struct Cand {
    game: Game,
    input: InputState,
    path: Vec<&'static [u8]>,
    /// Distance to the exit (but see `LIT`).
    dist: f64,
    /// Hearts lost, plus half a heart per bomb used on the way to the Black Market.
    hearts_lost: f64,
    /// Whether its macros have been tried. The game is deterministic, so a state is expanded once.
    expanded: bool,
}

impl Cand {
    /// Which to expand next: near the exit, unhurt.
    fn priority(&self) -> f64 {
        self.dist + HEART * self.hearts_lost
    }

    /// Whether this reaches its cell better than `o`: fewer hearts lost, then sooner.
    fn better_than(&self, o: &Cand) -> bool {
        (self.hearts_lost, self.path.len()) < (o.hearts_lost, o.path.len())
    }
}

/// Whether `o` is the object `name` or one of its descendants.
fn is_a(p: &gm::Program, o: gm::ObjectId, name: &str) -> bool {
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
fn player(game: &Game) -> Option<(gm::Inst, &gm::Instance)> {
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
type Cell = (i32, i32, i32, i8, i8, bool, Bombing);
type Bombing = (i8, bool, u16);

/// Searches for macros that take the exit, Go-Explore style: an archive keeps the
/// best way found to reach each 8 px cell, and the search keeps expanding the most promising cell with every
/// macro. Returns the input frame by frame.
fn search(
    game: &Game,
    input: &InputState,
    plife: f64,
    budget: usize,
    market: bool,
    goal: &Goal,
) -> Result<Vec<&'static [u8]>, String> {
    let bombs0 = if market {
        global(game, "bombs").unwrap_or(0.0) as i8
    } else {
        0
    };
    let fixed = match goal.object() {
        _ if market => Grid::to_market(game)?,
        Some(o) => Grid::to_object(game, o)?,
        None => Grid::of(game)?,
    };
    let ids: Vec<i32> = goal.object().map_or(Vec::new(), |o| {
        instances(game, o).iter().map(|(_, i)| i.id).collect()
    });
    if *goal == Goal::Carry && held(game).is_none() {
        return Err("carry: the player holds nothing".into());
    }
    let fixed_dist = fixed.distances();
    let room0 = room_name(game);
    let ropes = matches!(goal, Goal::Door { ropes: true, .. });
    let actions: Vec<(&'static [u8], u64)> = match goal {
        _ if market => ACTIONS.iter().copied().chain([BOMB_ACTION]).collect(),
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
            Some(o) if ropes => Some(Grid::to_object(g, o).ok()?),
            _ => None,
        }
        .map(|grid| {
            let dist = grid.distances();
            (grid, dist)
        });
        let (grid, dist) = live.as_ref().map_or((&fixed, &fixed_dist), |(g, d)| (g, d));
        let bombing = match &live {
            Some((grid, _)) if market => (
                global(g, "bombs").unwrap_or(0.0) as i8,
                find(g, "oBomb").is_some(),
                grid.solid.iter().filter(|&&s| s).count() as u16,
            ),
            // A thrown rope is a way up from the same cell.
            Some((grid, _)) => (
                global(g, "rope").unwrap_or(0.0) as i8,
                false,
                grid.ladder.iter().filter(|&&l| l).count() as u16,
            ),
            None => (0, false, 0),
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
        };
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
                ok = step(&mut g, &mut inp, keys);
                path.push(keys);
                if !ok || room_name(&g) != room0 {
                    break;
                }
                if goal.met(&g, &ids) {
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
        if round % 200 == 0 {
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
    for y in 0..fixed.h {
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

/// `oOlmec`'s `status` while it sinks into the lava (its create event's `DROWNING`).
const OLMEC_DROWNING: f64 = 4.0;
/// How much deeper Olmec has to be (4 per block of the floor gone) to make up for a heart lost.
const OLMEC_HEART: f64 = 24.0;

/// The real instance variable `name` of the first instance of `object`, if it has one.
fn ivar(game: &Game, object: &str, name: &str) -> Option<f64> {
    let (h, _) = *instances(game, object).first()?;
    match var(game, h, name) {
        Some(Value::Real(x)) => Some(x),
        _ => None,
    }
}

/// An archive cell of the fight: Olmec's 16 px column, 8 px row and status, the player's 16 px tile, and
/// the second. Olmec follows the player and slams down where it stands, so the two positions say what can
/// happen next; the time tells apart the waits while Olmec idles or the player has to stay out of its way.
type OlmecCell = (i32, i32, i8, i32, i32, u64);
/// The floor Olmec digs through: the rows of blocks between it and the lava.
const OLMEC_FLOOR: std::ops::Range<f64> = 560.0..720.0;

/// Searches for input that drowns Olmec in the lava, with the same kind of archive as `search`. Classic's
/// Olmec only dies in the lava (`oOlmec`'s step event): each slam breaks the row of blocks under it
/// (`oOlmecSlam`), and it slams when the player is below it, so the player lures it and gets out of the
/// way until it has dug through the floor. The search expands the state where Olmec is deepest, with a
/// penalty per heart lost, and stops once `oFinalBoss` sees it dead (it sinks slowly once in the lava, so
/// this includes the wait). The player may be deep in Olmec's shaft by then: the way to the door uses ropes.
fn beat_olmec(
    game: &Game,
    input: &InputState,
    plife: f64,
    budget: usize,
) -> Result<Vec<&'static [u8]>, String> {
    // The cell, depth and hearts lost of a state, and whether Olmec is dead; `None` if the player is.
    let judge = |g: &Game| -> Option<(OlmecCell, f64, f64, bool)> {
        let life = global(g, "plife").ok()?;
        if life < 1.0 {
            return None;
        }
        let (_, p) = player(g)?;
        // Not under the floor: it's a ledge over the lava, too far down the shaft to climb out with ropes.
        if p.y >= OLMEC_FLOOR.end {
            return None;
        }
        // `oFinalBoss` opens the door the step after Olmec dies (`olmecDead`), and the door is what counts.
        let dead = !instances(g, "oXEnd").is_empty();
        // Olmec only goes away when it dies in the lava: count that as drowned, at the bottom.
        let (ox, oy, status) = match find(g, "oOlmec") {
            Some(o) => (o.x, o.y, ivar(g, "oOlmec", "status").unwrap_or(0.0)),
            None => (0.0, f64::from(g.world().room().height), OLMEC_DROWNING),
        };
        // Progress is the floor's blocks gone (Olmec's own y swings as it bounces), and once it drowns,
        // only the player's safety matters until it's gone.
        let w = g.world();
        let floor = w
            .active()
            .chain(w.with_state(State::Inactive))
            .map(|h| w.instance(h))
            .filter(|i| OLMEC_FLOOR.contains(&i.y) && is_a(w.program(), i.object, "oSolid"))
            .count()
            // Olmec is a solid too (`oMovingSolid`).
            - usize::from(OLMEC_FLOOR.contains(&oy));
        let mut depth = -4.0 * floor as f64;
        if status == OLMEC_DROWNING {
            depth += 10_000.0 + oy;
        }
        let cell = (
            (ox / 16.0).floor() as i32,
            (oy / 8.0).floor() as i32,
            status as i8,
            (p.x / 16.0).floor() as i32,
            (p.y / 16.0).floor() as i32,
            g.world().frames() / 30,
        );
        Some((cell, depth, (plife - life).max(0.0), dead))
    };
    let (cell, depth, lost, _) = judge(game).ok_or("no player")?;
    let mut archive: HashMap<OlmecCell, Cand> = HashMap::new();
    archive.insert(
        cell,
        Cand {
            game: game.clone(),
            input: *input,
            path: Vec::new(),
            dist: -depth,
            hearts_lost: lost,
            expanded: false,
        },
    );
    let priority = |c: &Cand| c.dist + OLMEC_HEART * c.hearts_lost;
    for round in 0..budget {
        let Some((&key, _)) = archive.iter().filter(|(_, c)| !c.expanded).min_by(|a, b| {
            priority(a.1)
                .total_cmp(&priority(b.1))
                // The latest first, so that time goes on while Olmec makes no progress.
                .then(b.1.path.len().cmp(&a.1.path.len()))
                .then(a.0.cmp(b.0))
        }) else {
            break;
        };
        let from = archive.get_mut(&key).unwrap();
        from.expanded = true;
        let (game0, input0, path0) = (from.game.clone(), from.input, from.path.clone());
        // Each macro from the same state, in parallel: the game is a plain value.
        let judge = &judge;
        let results: Vec<Option<(Cand, OlmecCell, bool)>> = std::thread::scope(|s| {
            let jobs: Vec<_> = ACTIONS
                .iter()
                .map(|&(keys, hold)| {
                    // Cloned here: the game is `Send` but not `Sync` (its collision index is a cache).
                    let (mut g, mut inp, mut path) = (game0.clone(), input0, path0.clone());
                    s.spawn(move || {
                        for _ in 0..hold {
                            if !step(&mut g, &mut inp, keys) || room_name(&g) != "rOlmec" {
                                return None;
                            }
                            path.push(keys);
                        }
                        let (cell, depth, lost, dead) = judge(&g)?;
                        let new = Cand {
                            game: g,
                            input: inp,
                            path,
                            dist: -depth,
                            hearts_lost: lost,
                            expanded: false,
                        };
                        Some((new, cell, dead))
                    })
                })
                .collect();
            jobs.into_iter().map(|j| j.join().unwrap()).collect()
        });
        for (new, cell, dead) in results.into_iter().flatten() {
            if dead {
                return Ok(new.path);
            }
            match archive.get_mut(&cell) {
                Some(old) if !new.better_than(old) => {}
                Some(old) => *old = new,
                None => {
                    archive.insert(cell, new);
                }
            }
        }
        if round % 200 == 0 {
            let best = archive
                .values()
                .map(|c| -c.dist)
                .fold(f64::NEG_INFINITY, f64::max);
            eprintln!(
                "area-route: Olmec round {round}, {} cells, deepest {best:.1}",
                archive.len()
            );
        }
    }
    Err(format!(
        "Olmec still alive after {budget} expansions ({} cells)",
        archive.len()
    ))
}

/// Plays the ending after the door in `rOlmec`: the volcano (`rEnd`, `rEnd2`) and the tally (`rEnd3`) play
/// out untouched, since any key skips them; attack leaves the tally once it's done and the credits
/// (`rCredits2`) once the caravan has stopped, then the player walks out of `rHighscores` to the title.
fn play_ending(
    game: &mut Game,
    input: &mut InputState,
    out: &mut Replay,
    budget: usize,
) -> Result<(), String> {
    // Runs until `done`, tapping attack if `tap`.
    let wait_until = |game: &mut Game,
                      input: &mut InputState,
                      out: &mut Replay,
                      what: &str,
                      done: &dyn Fn(&Game) -> bool,
                      tap: bool|
     -> Result<(), String> {
        let mut i = 0u64;
        while !done(game) {
            let keys: &[u8] = if tap && i % 8 < 2 { &[ATTACK] } else { &[] };
            apply(game, input, out, keys)?;
            i += 1;
            if i > 6000 {
                return Err(format!("stuck in {} waiting for {what}", room_name(game)));
            }
        }
        Ok(())
    };
    let in_room = |name: &'static str| move |g: &Game| room_name(g) == name;
    wait_until(game, input, out, "rEnd3", &in_room("rEnd3"), false)?;
    // `oEnd3`'s `drawStatus` 7 is the end of the tally: attack fades it out (8), and then moves on to the
    // credits.
    let tallied = |g: &Game| ivar(g, "oEnd3", "drawStatus").is_some_and(|s| s >= 7.0);
    wait_until(game, input, out, "the tally", &tallied, false)?;
    wait_until(game, input, out, "rCredits2", &in_room("rCredits2"), true)?;
    // `oCredits2`'s alarm 8 stops the caravan (`oCamel.status` 2) after the last credit.
    let stopped = |g: &Game| ivar(g, "oCamel", "status").is_some_and(|s| s == 2.0);
    wait_until(game, input, out, "the caravan", &stopped, false)?;
    // Two seconds of the stopped caravan, so that the replay shows the end of the credits.
    for _ in 0..60 {
        apply(game, input, out, &[])?;
    }
    wait_until(
        game,
        input,
        out,
        "rHighscores",
        &in_room("rHighscores"),
        true,
    )?;
    to_title(game, input, out, budget)
}

/// Walks out of `rHighscores` (where the game goes after a death or the credits) to the title, and stays
/// there for 3 seconds.
fn to_title(
    game: &mut Game,
    input: &mut InputState,
    out: &mut Replay,
    budget: usize,
) -> Result<(), String> {
    if room_name(game) != "rHighscores" {
        return Err(format!("not in rHighscores but in {}", room_name(game)));
    }
    for _ in 0..30 {
        apply(game, input, out, &[])?;
    }
    eprintln!("area-route: rHighscores at frame {}", game.world().frames());
    let plife = global(game, "plife").unwrap_or(0.0);
    for keys in search(game, input, plife, budget, false, &Goal::TITLE)? {
        apply(game, input, out, keys)?;
    }
    eprintln!("area-route: title at frame {}", game.world().frames());
    for _ in 0..90 {
        apply(game, input, out, &[])?;
    }
    Ok(())
}
