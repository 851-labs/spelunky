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

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use game::Game;
use game::replay::Playback;
use gm::{State, Value};
use harness::route::*;
use trace::Replay;
use trace::replay::InputState;

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
                   `<object>` reaches an instance of the object or a descendant (or collects one),
                   `+<object>` picks one up (`!<object>` bombing through blocks to it, for a sealed
                   key) and `-<object>` kills one (whipping or stomping it; `^<object>` only stomping,
                   `*<object>` with a bomb, `/<object>` with the held item thrown at it); `><object>`
                   reaches one holding on to the held item (the sceptre into the gold door); `@X:Y`
                   reaches a point of the room in pixels (`>@X:Y` holding on to the held item). A
                   route that starts with the ankh keeps it. Each level's shops, wares, rarer finds,
                   kind (dark, cemetery, lake...), enemies and what the player has are printed on
                   stderr.
  --olmec          then, in rOlmec (after the levels and the wait), drown Olmec, take the door to the
                   ending, play through the ending and the credits, and walk out of rHighscores to the title
  --title          then walk out of rHighscores to the title (for a replay that ends there)

On failure it prints what it explored on stderr: o reached, # wall, H ladder, ^ spikes, E the exit, and .
or , open tiles from which the grid's model of the moves does or doesn't see a way to the exit.
  -h, --help";

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
        let path = match search(&game, &input, plife, budget, to_market, goal, true) {
            Err(e) if to_market => {
                eprintln!("area-route: no way into the Black Market ({e}), taking the exit");
                search(&game, &input, plife, budget, false, goal, true)?
            }
            r => r?,
        };
        for keys in path {
            apply(&mut game, &mut input, &mut out, keys)?;
        }
        if !goal.is_exit() {
            // A kill counts once the target is out of hit points; its own step destroys it a frame or two
            // later, leaving what it drops (the tomb lord's sceptre) for the next goal.
            if matches!(goal, Goal::Kill { .. }) {
                for _ in 0..2 {
                    apply(&mut game, &mut input, &mut out, &[])?;
                }
            }
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
        for keys in search(&game, &input, plife, budget, false, &Goal::END, true)? {
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
    for keys in search(game, input, plife, budget, false, &Goal::TITLE, true)? {
        apply(game, input, out, keys)?;
    }
    eprintln!("area-route: title at frame {}", game.world().frames());
    for _ in 0..90 {
        apply(game, input, out, &[])?;
    }
    Ok(())
}
