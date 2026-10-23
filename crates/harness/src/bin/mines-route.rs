//! Finds input that takes the player through the next levels on the port and writes it as a replay, so the
//! oracle can be compared on levels a standing player never reaches (levels 2-4 of the Mines). The game has
//! no level-select or debug key that would get there (`oGame`'s only key event is `M`, music).
//!
//! It plays the given replay (one that ends in a level, such as `fidelity/gen-mines.sh`'s), then for each
//! level searches held-key macros until the player takes the exit, skips the transition room with the
//! attack key, and finally stands still in the last level. The search is best-first over an archive of
//! cells (Go-Explore without the randomness): it keeps the best state found in each cell and expands the
//! one nearest the exit by a tile-grid distance, with a penalty per heart lost. The route is only as good
//! as the port's fidelity: `scripts/fidelity` on the result says whether the oracle follows it.

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
usage: mines-route [options] <replay.txt>

Plays the replay on the port, which must end in a level, then searches for input that takes the player
through the next <levels> levels' exits and writes the extended replay on stdout. After the last exit
it skips the transition and stands still for <wait> frames in the new level.

options:
  --assets <dir>   the asset bundle (default: assets)
  --levels <n>     how many exits to take (default 3: from level 1 to level 4)
  --wait <n>       frames to stand still in the last level (default 300)
  --budget <n>     states to expand per level before giving up (default 8000)

On failure it prints what it explored on stderr: o reached, # wall, H ladder, ^ spikes, E the exit, and .
or , open tiles from which the grid's model of the moves does or doesn't see a way to the exit.
  -h, --help";

const LEFT: u8 = 37;
const UP: u8 = 38;
const RIGHT: u8 = 39;
const DOWN: u8 = 40;
const JUMP: u8 = 90; // Z
const ATTACK: u8 = 88; // X

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
const TILE: f64 = 16.0;
/// Distance-field cost of a heart lost.
const HEART: f64 = 30.0;

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("mines-route: {e}");
            ExitCode::from(2)
        }
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let (mut assets, mut levels, mut wait, mut budget) = (PathBuf::from("assets"), 3, 300u64, 8000);
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
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(());
            }
            p if !p.starts_with('-') => replay_path = Some(PathBuf::from(p)),
            _ => return Err(format!("unknown option {a}\n{USAGE}")),
        }
    }
    let replay_path = replay_path.ok_or(USAGE)?;
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
    for level in 0..levels {
        if room_name(&game) != "rLevel" {
            return Err(format!("not in rLevel but {}", room_name(&game)));
        }
        let t = std::time::Instant::now();
        let path = search(&game, &input, global(&game, "plife")?, budget)?;
        for keys in path {
            apply(&mut game, &mut input, &mut out, keys)?;
        }
        eprintln!(
            "mines-route: exit {} taken at frame {} ({:.1} s)",
            level + 1,
            game.world().frames(),
            t.elapsed().as_secs_f64()
        );
        // Skip the transition: tap attack until a level loads.
        let mut n = 0;
        while room_name(&game) != "rLevel" {
            let keys: &[u8] = if n % 8 < 2 { &[ATTACK] } else { &[] };
            apply(&mut game, &mut input, &mut out, keys)?;
            n += 1;
            if n > 3000 {
                return Err(format!("stuck in {}", room_name(&game)));
            }
        }
    }
    for _ in 0..wait {
        apply(&mut game, &mut input, &mut out, &[])?;
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

/// The level as a tile grid: what's solid, ladders, spikes, and the exit's tile.
struct Grid {
    w: i32,
    h: i32,
    solid: Vec<bool>,
    ladder: Vec<bool>,
    spikes: Vec<bool>,
    exit: (i32, i32),
}

impl Grid {
    fn of(game: &Game) -> Result<Grid, String> {
        let w = game.world();
        let room = w.assets().room(w.room().id).ok_or("no room")?;
        let (gw, gh) = (room.width / 16 + 1, room.height / 16 + 1);
        let n = (gw * gh) as usize;
        let mut g = Grid {
            w: gw,
            h: gh,
            solid: vec![false; n],
            ladder: vec![false; n],
            spikes: vec![false; n],
            exit: (-1, -1),
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
            if is_solid(p, inst.object) && name != "oPushBlock" {
                g.solid[i] = true;
            } else if name.starts_with("oLadder") {
                g.ladder[i] = true;
            } else if name == "oSpikes" {
                g.spikes[i] = true;
            } else if name == "oExit" {
                g.exit = (tx, ty);
            }
        }
        if g.exit.0 < 0 {
            return Err("no oExit".into());
        }
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
        let standing = !self.open(x, y + 1) || self.ladder(x, y) || self.ladder(x, y + 1);
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
        let mut dist = vec![f64::INFINITY; n];
        let start = self.idx(self.exit.0, self.exit.1).unwrap();
        dist[start] = 0.0;
        let mut heap = BinaryHeap::new();
        heap.push(Node(0.0, start));
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
    /// Distance to the exit.
    dist: f64,
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

/// Whether instances of `o` are walls: `oSolid` or one of its children.
fn is_solid(p: &gm::Program, o: gm::ObjectId) -> bool {
    let mut o = Some(o);
    while let Some(id) = o {
        let Some(info) = p.object(id) else {
            return false;
        };
        if info.name == "oSolid" {
            return true;
        }
        o = info.parent;
    }
    false
}

/// The player, if it exists.
fn player(game: &Game) -> Option<&gm::Instance> {
    let w = game.world();
    let p = w.program();
    w.active()
        .map(|h| w.instance(h))
        .find(|i| p.object(i.object).is_some_and(|o| o.name == "oPlayer1"))
}

/// An archive cell: the player's 8 px cell, sprite (standing, climbing, hanging, ...), direction of motion
/// and whether jump is held (which makes the jump higher). The cells tell apart states that can go to
/// different places, without telling apart so many that the search drowns in them.
type Cell = (i32, i32, i32, i8, i8, bool);

/// Searches for macros that take the exit, Go-Explore style: an archive keeps the
/// best way found to reach each 8 px cell, and the search keeps expanding the most promising cell with every
/// macro. Returns the input frame by frame.
fn search(
    game: &Game,
    input: &InputState,
    plife: f64,
    budget: usize,
) -> Result<Vec<&'static [u8]>, String> {
    let grid = Grid::of(game)?;
    let dist = grid.distances();
    // The cell, distance and hearts lost of a state; `None` if the player is gone or dead.
    let judge = |g: &Game, inp: &InputState| -> Option<(Cell, f64, f64)> {
        let p = player(g)?;
        let (x, y) = (p.x, p.y);
        let life = global(g, "plife").ok()?;
        if life < 1.0 {
            return None;
        }
        let (tx, ty) = ((x / TILE).floor() as i32, (y / TILE).floor() as i32);
        let i = grid.idx(tx, ty)?;
        // Where the grid's model sees no route (the game's moves are richer), head for the exit as the
        // crow flies, behind every tile that has one.
        let crow = (tx - grid.exit.0).abs() + (ty - grid.exit.1).abs();
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
        );
        Some((cell, d, (plife - life).max(0.0)))
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
        for &(keys, hold) in ACTIONS {
            let (mut g, mut inp, mut path) = (game0.clone(), input0, path0.clone());
            let mut ok = true;
            for _ in 0..hold {
                ok = step(&mut g, &mut inp, keys);
                path.push(keys);
                if !ok || room_name(&g) != "rLevel" {
                    break;
                }
            }
            if !ok {
                continue;
            }
            if room_name(&g) != "rLevel" {
                // The exit's room change, unless the player died (then it's the death screen).
                if room_name(&g).starts_with("rTransition") {
                    return Ok(path);
                }
                continue;
            }
            let Some((cell, d, lost)) = judge(&g, &inp) else {
                continue;
            };
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
                "mines-route: round {round}, {} cells, nearest {best:.1}",
                archive.len()
            );
        }
    }
    // What was explored, to see where it got stuck: `o` reached, `.` and `,` open with and without a
    // route to the exit `E` in the grid's model.
    for y in 0..grid.h {
        let row: String = (0..grid.w)
            .map(|x| {
                let i = grid.idx(x, y).unwrap();
                match () {
                    _ if (x, y) == grid.exit => 'E',
                    _ if archive.keys().any(|c| (c.0 / 2, c.1 / 2) == (x, y)) => 'o',
                    _ if grid.solid[i] => '#',
                    _ if grid.ladder[i] => 'H',
                    _ if grid.spikes[i] => '^',
                    _ if dist[i].is_finite() => '.',
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
