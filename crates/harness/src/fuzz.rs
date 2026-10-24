//! Random-input fidelity fuzzing (`fuzz-replay`, `fuzz-sweep`): replays that take a returning player into the
//! first level and then play it with random but game-shaped input, and what such a run covered.
//!
//! The input isn't blind. While generating, the fuzzer plays the replay so far on the port and looks at where
//! the player and the exit are, so it can lean toward the exit and press up at the door. That keeps runs going
//! into later levels without a script. A replay is still a pure function of (game seed, fuzz seed, length) and
//! the port's behavior; once written, it's just a replay, and the oracle never sees how it was made.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use game::Game;
use game::replay::Playback;
use serde::Serialize;
use trace::replay::{Button, Event, Highscore, InputState, ReplayHeader};

use crate::route;
use trace::{Frame, Replay, Value};

/// Virtual-key codes, under the shipped `keys.cfg` (`docs/formats.md`).
pub mod vk {
    pub const LEFT: u8 = 37;
    pub const UP: u8 = 38;
    pub const RIGHT: u8 = 39;
    pub const DOWN: u8 = 40;
    /// Z.
    pub const JUMP: u8 = 90;
    /// X: whip, pick up (ducking), throw, use.
    pub const ATTACK: u8 = 88;
    /// C: switch the selected item.
    pub const ITEM: u8 = 67;
    pub const RUN: u8 = 16;
    /// A.
    pub const BOMB: u8 = 65;
    /// S.
    pub const ROPE: u8 = 83;
    pub const FLARE: u8 = 70;
    pub const PAY: u8 = 80;
    /// Escape: pause. While paused, rope quits the game and bomb kills the player (`oScreen`'s begin step).
    pub const START: u8 = 27;
}

/// Where a fuzz run starts playing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Start {
    /// The first mines level, through the title's START door, as a returning player.
    Mines,
    /// The title's shortcut doors, with the Tunnel Man paid off: the first jungle (level 5), ice caves
    /// (level 9) and temple (level 13) levels.
    Jungle,
    Ice,
    Temple,
}

impl Start {
    pub const ALL: [Start; 4] = [Start::Mines, Start::Jungle, Start::Ice, Start::Temple];

    pub fn name(self) -> &'static str {
        match self {
            Start::Mines => "mines",
            Start::Jungle => "jungle",
            Start::Ice => "ice",
            Start::Temple => "temple",
        }
    }

    pub fn parse(name: &str) -> Result<Start, String> {
        Start::ALL
            .into_iter()
            .find(|s| s.name() == name)
            .ok_or_else(|| format!("unknown start {name} (mines, jungle, ice or temple)"))
    }

    /// The input that gets there: skip the intro (attack, twice) and walk into the door, as
    /// `fidelity/replays/movement-*.txt` and `title-shortcut5.txt` do. For a shortcut, climb the ladder
    /// to the gallery, jump off it to the left and stop at the door.
    fn prelude(self) -> &'static [(u64, u8, u64)] {
        const SKIP: [(u64, u8, u64); 2] = [(20, vk::ATTACK, 2), (60, vk::ATTACK, 2)];
        match self {
            // rLevel on frame 565.
            Start::Mines => &[SKIP[0], SKIP[1], (450, vk::LEFT, 70), (535, vk::UP, 5)],
            // rLevel on frame 786.
            Start::Jungle => &[
                SKIP[0],
                SKIP[1],
                (450, vk::LEFT, 81),
                (538, vk::UP, 100),
                (640, vk::LEFT, 99),
                (640, vk::JUMP, 6),
                (750, vk::UP, 5),
            ],
            // rLevel2 on frame 758.
            Start::Ice => &[
                SKIP[0],
                SKIP[1],
                (450, vk::LEFT, 81),
                (538, vk::UP, 100),
                (640, vk::LEFT, 78),
                (640, vk::JUMP, 6),
                (727, vk::UP, 5),
            ],
            // rLevel on frame 736.
            Start::Temple => &[
                SKIP[0],
                SKIP[1],
                (450, vk::LEFT, 81),
                (538, vk::UP, 100),
                (640, vk::LEFT, 56),
                (640, vk::JUMP, 6),
                (705, vk::UP, 5),
            ],
        }
    }

    /// The first frame with random input: the level's step events have been running for a while.
    pub fn fuzz_start(self) -> u64 {
        match self {
            Start::Mines => 580,
            Start::Jungle => 800,
            Start::Ice => 772,
            Start::Temple => 750,
        }
    }

    /// The highscore table: a returning player with zeroed stats, as `scrResetHighscores` writes on the
    /// first launch (`tools/oracle/examples/returning.txt`). For the shortcuts, TUNNEL1 and TUNNEL2 at their
    /// offsets (nothing owed) and $100 of MONEY, as in `title-shortcut5.txt`.
    pub fn header(self, seed: i32) -> ReplayHeader {
        let (money, tunnel1, tunnel2) = match self {
            Start::Mines => (9_000_000.0, 2_100_001.0, 1_200_001.0),
            _ => (9_000_100.0, 2_000_000.0, 1_000_000.0),
        };
        let mut h = ReplayHeader::new(seed);
        h.highscores = [
            (money, "MONEY"),
            (8_000_000.0, "TIME"),
            (7_000_000.0, "KILLS"),
            (6_000_000.0, "SAVES"),
            (5_000_000.0, "PLAYS"),
            (4_000_000.0, "WINS"),
            (3_000_000.0, "DEATHS"),
            (tunnel1, "TUNNEL1"),
            (tunnel2, "TUNNEL2"),
            (0.0, "MINIGAMES"),
        ]
        .into_iter()
        .map(|(value, name)| Highscore {
            name: name.into(),
            value,
        })
        .collect();
        h
    }
}

/// splitmix64: small, fast, and the same everywhere.
struct Rng(u64);

impl Rng {
    fn new(game_seed: i32, fuzz_seed: u64) -> Rng {
        // Each seed through the mixer in turn, so that no two pairs share a stream.
        let mut r = Rng(u64::from(game_seed as u32));
        r.0 = r.next() ^ fuzz_seed;
        r.next();
        r
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// Uniform in `0..n` (n > 0).
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    /// Uniform in `lo..=hi`.
    fn range(&mut self, lo: u64, hi: u64) -> u64 {
        lo + self.below(hi - lo + 1)
    }

    /// True with probability `percent`/100.
    fn chance(&mut self, percent: u64) -> bool {
        self.below(100) < percent
    }

    /// A hold length: mostly taps and short presses, sometimes long ones.
    fn press_len(&mut self) -> u64 {
        match self.below(100) {
            0..=49 => self.range(1, 3),
            50..=84 => self.range(4, 10),
            _ => self.range(11, 22),
        }
    }

    /// How long to hold a direction.
    fn walk_len(&mut self) -> u64 {
        match self.below(100) {
            0..=39 => self.range(3, 12),
            40..=79 => self.range(12, 45),
            _ => self.range(45, 120),
        }
    }
}

/// What the fuzzer looks at between segments of input.
#[derive(Default)]
struct View {
    player: Option<(f64, f64)>,
    /// Where to go: the level's exit, or a door back into the game.
    exit: Option<(f64, f64)>,
    /// Whether the game can be paused and unpaused safely: the player is alive and in a level (one with an
    /// exit). Start on a dead player's game over leaves for the high score room, where the unpausing
    /// press would pause it instead, and then a rope press would end the game.
    can_pause: bool,
    /// Whether the player is in the title or high score room, on the way back into the game.
    menu: bool,
}

impl View {
    fn of(frame: &Frame) -> View {
        let find = |object: &str| {
            let mut all = frame
                .instances
                .iter()
                .chain(frame.deactivated.iter().flatten());
            all.find(|i| i.object == object)
                .map(|i| (i.x.map_or(0.0, |r| r.0), i.y.map_or(0.0, |r| r.0)))
        };
        let level_exit = find("oExit");
        View {
            player: find("oPlayer1"),
            menu: level_exit.is_none(),
            can_pause: level_exit.is_some()
                && frame.instances.iter().any(|i| {
                    i.object == "oPlayer1" && !truthy(i.vars.as_ref().and_then(|v| v.get("dead")))
                }),
            // A level's exit; after a game over, the high score room's door to the title, and the title's
            // START door, back into the first level.
            exit: level_exit.or_else(|| ["oXTitle", "oXStart"].into_iter().find_map(find)),
        }
    }

    /// The exit relative to the player, (dx, dy), in pixels.
    fn to_exit(&self) -> Option<(f64, f64)> {
        let ((px, py), (ex, ey)) = (self.player?, self.exit?);
        // The exit's origin is its top-left corner, the player's its centre (both are 16x16).
        Some((ex + 8.0 - px, ey + 8.0 - py))
    }
}

struct Generator {
    rng: Rng,
    keys: Keys,
}

/// The input planned so far.
struct Keys {
    events: BTreeMap<u64, Vec<Event>>,
    /// The first frame each key may be pressed again: holds of one key never overlap, and there's at
    /// least one frame between them, so every hold is a fresh press.
    free: [u64; 256],
    end: u64,
}

impl Keys {
    /// Holds `key` from frame `at` for `len` frames, unless it's still held or released too recently then.
    fn hold(&mut self, key: u8, at: u64, len: u64) -> bool {
        let len = len.max(1);
        if at < self.free[usize::from(key)] || at >= self.end {
            return false;
        }
        let k = Button::Key(key);
        self.events.entry(at).or_default().push(Event::Press(k));
        if at + len < self.end {
            self.events
                .entry(at + len)
                .or_default()
                .push(Event::Release(k));
        }
        self.free[usize::from(key)] = at + len + 1;
        true
    }

    /// The first frame from `t` on at which no key is held.
    fn all_free(&self, t: u64) -> u64 {
        self.free.iter().copied().fold(t, u64::max)
    }
}

/// Goal-directed fuzzing: random play in bursts, between routes that `harness::route` searches for toward
/// the level's exit or one of its characters and shops.
#[derive(Clone, Copy, Debug)]
pub struct Directed {
    /// States the route search may expand per goal (`area-route --budget`).
    pub budget: usize,
    /// How many frames of random play between routes, at least and at most.
    pub burst: (u64, u64),
}

impl Default for Directed {
    fn default() -> Self {
        Directed {
            budget: 600,
            burst: (60, 240),
        }
    }
}

impl Generator {
    /// Searches the port for a way from frame `t` (with nothing held) to a goal: mostly the exit, sometimes
    /// a shop's ware (then pay), the damsel (then out the exit with her), the dice or a shopkeeper. Writes
    /// it as input and returns the frame after it, or `t` if no way was found.
    fn route(&mut self, game: &Game, t: u64, d: &Directed) -> u64 {
        let goals: &[&str] = match self.rng.below(100) {
            0..=59 => &["exit"],
            60..=74 => &["+oItem$"],
            75..=84 => &["+oDamsel", "carry"],
            85..=89 => &["oDice"],
            _ => &["oShopkeeper"],
        };
        let mut game = game.clone();
        let mut input = InputState::default();
        let mut f = t;
        'goals: for goal in goals {
            let goal = route::Goal::parse(goal);
            let plife = route::global(&game, "plife").unwrap_or(0.0);
            let Ok(path) = route::search(&game, &input, plife, d.budget, false, &goal, false)
            else {
                break;
            };
            for keys in path {
                let next = InputState::keys(keys.iter().copied());
                let events = input.events_to(&next);
                if !events.is_empty() {
                    self.keys
                        .events
                        .entry(f)
                        .or_default()
                        .extend(events.iter().copied());
                }
                input = next;
                // Cut short (the replay's end, or the port failed): the goal isn't met, so neither pay
                // nor go on to the next goal from a state the replay doesn't reach.
                if f + 1 >= self.keys.end || route::step_events(&mut game, &events).is_err() {
                    break 'goals;
                }
                f += 1;
            }
            if goal == route::Goal::Pickup("oItem$".into()) {
                // Buy it.
                self.keys
                    .events
                    .entry(f)
                    .or_default()
                    .extend(input.events_to(&InputState::keys([vk::PAY])));
                input = InputState::keys([vk::PAY]);
                f += 2;
                break;
            }
        }
        // Let go of everything, and keep the random holds from pressing a key the route still holds.
        let release = input.events_to(&InputState::default());
        if f < self.keys.end && !release.is_empty() {
            self.keys.events.entry(f).or_default().extend(release);
        }
        for free in &mut self.keys.free {
            *free = (*free).max(f + 1);
        }
        f + 1
    }

    /// Plans input from frame `t` on and returns where the next segment starts.
    fn segment(&mut self, t: u64, view: &View) -> u64 {
        let (rng, k) = (&mut self.rng, &mut self.keys);
        let to_exit = view.to_exit();
        // At the door: go in, most of the time.
        if let Some((dx, dy)) = to_exit
            && dx.abs() < 10.0
            && dy.abs() < 12.0
            && rng.chance(80)
        {
            let len = rng.range(1, 3);
            k.hold(vk::UP, t, len);
            return t + len + 2;
        }
        let toward = match to_exit {
            Some((dx, _)) if dx.abs() >= 4.0 => Some(if dx > 0.0 { vk::RIGHT } else { vk::LEFT }),
            Some(_) => None,
            // Not in a level (or no exit): the exit is usually further right and down.
            None => Some(vk::RIGHT),
        };
        // Back to the game: straight to the door. The other doors on the way lead elsewhere, so no up.
        if view.menu
            && let Some(d) = toward
        {
            let len = rng.range(5, 30);
            k.hold(d, t, len);
            if rng.chance(30) {
                k.hold(vk::JUMP, t + rng.below(len), rng.press_len());
            }
            return t + len + 1;
        }
        let away = |d: u8| if d == vk::RIGHT { vk::LEFT } else { vk::RIGHT };
        let dir = match (rng.below(100), toward) {
            (0..=59, Some(d)) => Some(d),
            (60..=84, Some(d)) => Some(away(d)),
            (0..=49, None) => Some(vk::RIGHT),
            (50..=84, None) => Some(vk::LEFT),
            _ => None,
        };
        let len = match rng.below(100) {
            // Walk or run somewhere, jumping, whipping, looking up and down on the way.
            0..=54 => {
                let len = rng.walk_len();
                if let Some(d) = dir {
                    k.hold(d, t, len);
                }
                if rng.chance(35) {
                    k.hold(vk::RUN, t, len);
                }
                for _ in 0..rng.below(4) {
                    let at = t + rng.below(len);
                    let n = rng.press_len();
                    k.hold(vk::JUMP, at, n);
                }
                if rng.chance(25) {
                    let at = t + rng.below(len);
                    let n = rng.range(1, 3);
                    k.hold(vk::ATTACK, at, n);
                }
                if rng.chance(10) {
                    let at = t + rng.below(len);
                    let n = rng.range(1, 20);
                    k.hold(vk::UP, at, n);
                }
                if rng.chance(10) {
                    let at = t + rng.below(len);
                    let n = rng.range(1, 30);
                    k.hold(vk::DOWN, at, n);
                }
                len
            }
            // Duck and pick something up, then maybe throw it (up, forward or down).
            55..=62 => {
                let duck = rng.range(6, 16);
                k.hold(vk::DOWN, t, duck);
                k.hold(vk::ATTACK, t + rng.range(2, 5), 2);
                let throw = t + duck + rng.range(5, 30);
                if rng.chance(70) {
                    match rng.below(3) {
                        0 => {
                            k.hold(vk::UP, throw - 2, 6);
                        }
                        1 => {
                            if let Some(d) = dir {
                                k.hold(d, throw - 2, 8);
                            }
                        }
                        _ => {
                            k.hold(vk::DOWN, throw - 2, 6);
                        }
                    }
                    k.hold(vk::ATTACK, throw, rng.range(1, 3));
                }
                throw + 6 - t
            }
            // A bomb: placed while ducking, or thrown; then get away from it.
            63..=67 => {
                if rng.chance(50) {
                    k.hold(vk::DOWN, t, 8);
                }
                k.hold(vk::BOMB, t + 2, rng.range(1, 3));
                let run = rng.range(20, 50);
                if let Some(d) = dir {
                    k.hold(d, t + 6, run);
                }
                k.hold(vk::RUN, t + 6, run);
                6 + run
            }
            // A rope, up or (ducking at a ledge) down, and maybe climb it.
            68..=72 => {
                if rng.chance(40) {
                    k.hold(vk::DOWN, t, 8);
                }
                k.hold(vk::ROPE, t + 2, rng.range(1, 3));
                let climb = rng.range(10, 60);
                let at = t + rng.range(6, 20);
                let key = if rng.chance(60) { vk::UP } else { vk::DOWN };
                k.hold(key, at, climb);
                if rng.chance(50) {
                    k.hold(vk::JUMP, at + 1, rng.press_len());
                }
                at + climb - t
            }
            // Ladders, doors, looking: up or down for a while, maybe jumping onto something.
            73..=82 => {
                let n = rng.range(5, 60);
                let key = if rng.chance(55) { vk::UP } else { vk::DOWN };
                k.hold(key, t, n);
                if rng.chance(40) {
                    k.hold(vk::JUMP, t + rng.below(n), rng.press_len());
                }
                if let Some(d) = dir
                    && rng.chance(30)
                {
                    k.hold(d, t + rng.below(n), rng.range(2, 15));
                }
                n
            }
            // Switch items, light a flare, pay a shopkeeper; use the selected item.
            83..=86 => {
                let key = match rng.below(10) {
                    0..=6 => vk::ITEM,
                    7 | 8 => vk::PAY,
                    _ => vk::FLARE,
                };
                k.hold(key, t, rng.range(1, 3));
                if rng.chance(40) {
                    k.hold(vk::ATTACK, t + rng.range(4, 12), 2);
                }
                rng.range(8, 20)
            }
            // Stand still.
            87..=93 => rng.range(5, 30),
            // Pause, with nothing else held, and unpause (also once the player is dead: start restarts).
            // Only from a frame with nothing held, so that the player is still alive when it happens.
            94..=95 if view.can_pause && k.all_free(t) == t => {
                let at = t;
                k.hold(vk::START, at, rng.range(1, 2));
                let resume = at + 3 + rng.range(10, 60);
                k.hold(vk::START, resume, rng.range(1, 2));
                let done = resume + 3;
                for f in &mut k.free {
                    *f = (*f).max(done);
                }
                done - t
            }
            // Hop about in place.
            _ => {
                let mut at = t;
                for _ in 0..rng.range(2, 5) {
                    let n = rng.press_len();
                    k.hold(vk::JUMP, at, n);
                    at += n + rng.range(1, 10);
                }
                at - t
            }
        };
        t + len.max(1) + rng.below(4)
    }
}

/// A fuzz replay: the prelude to `start`, then `frames` frames of random input from `fuzz_seed`, in a game
/// with `game_seed`. Stepping the port to guide the input needs `assets`.
pub fn fuzz_replay(
    assets: Arc<gm::Assets>,
    start: Start,
    game_seed: i32,
    fuzz_seed: u64,
    frames: u64,
    directed: Option<Directed>,
) -> Result<Replay, String> {
    let header = start.header(game_seed);
    let end = start.fuzz_start() + frames;
    let mut g = Generator {
        rng: Rng::new(game_seed, fuzz_seed),
        keys: Keys {
            events: BTreeMap::new(),
            free: [0; 256],
            end,
        },
    };
    for &(at, key, len) in start.prelude() {
        g.keys.hold(key, at, len);
    }

    // Booted as the replay will be, by `Playback`, but stepped here: the input is still being written.
    let mut game = Playback::boot(assets, &Replay::new(header.clone()))
        .ok()
        .map(Playback::into_game);
    let mut t = start.fuzz_start();
    // Directed: frames of random play left before the next route.
    let mut burst = 0;
    while t < end {
        // Play up to `t`. If the port fails or the game ends, keep going without looking.
        let mut view = View::default();
        if let Some(gm) = &mut game {
            let mut failed = false;
            while !failed && gm.world().frames() < t && !gm.ended() {
                let f = gm.world().frames();
                let events = g.keys.events.get(&f).map_or(&[][..], Vec::as_slice);
                failed = gm.step(events).is_err();
            }
            if !failed && gm.world().frames() == t {
                view = View::of(&gm.trace_with(true));
            } else {
                game = None;
            }
        }
        if let (Some(d), Some(gm)) = (&directed, &game) {
            if route::room_name(gm).starts_with("rTransition") {
                // The transition between levels: attack skips it.
                g.keys.hold(vk::ATTACK, t, 2);
                t += 8;
                continue;
            }
            if burst == 0 && view.can_pause && g.keys.all_free(t) == t {
                t = g.route(gm, t, d);
                burst = g.rng.range(d.burst.0, d.burst.1);
                continue;
            }
        }
        let next = g.segment(t, &view);
        burst = burst.saturating_sub(next - t);
        t = next;
    }

    let mut replay = Replay::new(header);
    for (f, events) in g.keys.events {
        for e in events {
            replay.push(f, e).map_err(|e| e.to_string())?;
        }
    }
    replay.set_len(end).map_err(|e| e.to_string())?;
    Ok(replay)
}

/// The replay cut off before frame `len`: later events dropped, keys still down stay down.
pub fn truncate(replay: &Replay, len: u64) -> Replay {
    let mut out = Replay::new(replay.header.clone());
    for &(f, e) in replay.events().iter().take_while(|(f, _)| *f < len) {
        out.push(f, e)
            .expect("in order, and valid in the replay already");
    }
    out.set_len(len.min(replay.len()))
        .expect("no event at or after len");
    out
}

/// What a run exercised, from the port's state after every frame.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Coverage {
    pub frames: u64,
    /// Every room the game was in.
    pub rooms: BTreeSet<String>,
    /// The deepest level (`global.currLevel`).
    pub max_level: f64,
    /// How often the player died (`oPlayer1.dead` became true).
    pub deaths: u32,
    /// Objects that were active (in or near the view, so stepping). In a level, only frames after
    /// `oLevel` first deactivates what's out of view count ([`counts_as_active`]).
    pub active: BTreeSet<String>,
    /// Objects that existed at all, active or deactivated.
    pub existed: BTreeSet<String>,
    /// What the player held (`oPlayer1.holdItem`), by object.
    pub held: BTreeSet<String>,
    /// The port's error, if it stopped on one.
    pub error: Option<String>,
}

fn truthy(v: Option<&Value>) -> bool {
    matches!(v, Some(Value::Real(r)) if r.0 >= 0.5)
}

fn real(v: Option<&Value>) -> Option<f64> {
    match v {
        Some(Value::Real(r)) => Some(r.0),
        _ => None,
    }
}

/// Whether `frame`'s active instances count as active in [`Coverage`]: all but a level's first frame,
/// before `oLevel` first deactivates what's out of view. A level loads (room end, then the placed
/// instances' create events, which generate it, room start and the frame-0 draw) with everything it
/// generated active. `oLevel` culls only in its step event, on the next frame, so an instance culled then
/// has run its create event and that frame's begin step and alarms, and its step only if its object comes
/// before `oLevel`'s in id order (`oSmashTrapLit`'s doesn't).
///
/// `oLevel`'s step sets `offset = 96` right before its `instance_deactivate_region`, and its create
/// doesn't, so an `oLevel` without `offset` hasn't culled yet. (It culls whenever `view_enabled`, which
/// holds in every room with an `oLevel`.) "Something is deactivated" isn't a reliable test: the persistent
/// `oMusic` arrives in a level still deactivated from the one before. Frames without an active `oLevel`
/// (other rooms, or the pause screen) all count.
pub fn counts_as_active(frame: &Frame) -> bool {
    frame
        .instances
        .iter()
        .find(|i| i.object == "oLevel")
        .is_none_or(|l| l.vars.as_ref().is_some_and(|v| v.contains_key("offset")))
}

/// Plays `replay` on the port and records what it covered.
pub fn coverage(replay: &Replay, assets: Arc<gm::Assets>) -> Coverage {
    let mut c = Coverage::default();
    let mut p = match Playback::boot(assets, replay) {
        Ok(p) => p,
        Err(e) => {
            c.error = Some(e.message);
            return c;
        }
    };
    let (mut was_dead, mut was_held) = (false, 0.0);
    loop {
        match p.step() {
            Ok(Some(_)) => {}
            Ok(None) => break,
            Err(e) => {
                c.error = Some(e.message);
                break;
            }
        }
        c.frames += 1;
        let frame = p.game().trace_with(true);
        if let Some(room) = &frame.room_name
            && !c.rooms.contains(room)
        {
            c.rooms.insert(room.clone());
        }
        if let Some(level) = real(frame.globals.as_ref().and_then(|g| g.get("currLevel"))) {
            c.max_level = c.max_level.max(level);
        }
        if counts_as_active(&frame) {
            for i in &frame.instances {
                if !c.active.contains(&i.object) {
                    c.active.insert(i.object.clone());
                }
            }
        }
        for i in frame
            .instances
            .iter()
            .chain(frame.deactivated.iter().flatten())
        {
            if !c.existed.contains(&i.object) {
                c.existed.insert(i.object.clone());
            }
        }
        let Some(player) = frame.instances.iter().find(|i| i.object == "oPlayer1") else {
            continue;
        };
        let vars = player.vars.as_ref();
        let dead = truthy(vars.and_then(|v| v.get("dead")));
        if dead && !was_dead {
            c.deaths += 1;
        }
        was_dead = dead;
        let held = real(vars.and_then(|v| v.get("holdItem"))).unwrap_or(0.0);
        if held != was_held
            && held > 0.0
            && let Some(item) = frame.instances.iter().find(|i| f64::from(i.id) == held)
        {
            c.held.insert(item.object.clone());
        }
        was_held = held;
    }
    c
}

/// The gameplay tickets, by the folder an object sits in in the original's resource tree
/// (`docs/source-inventory.md`, section 2).
pub const TICKETS: [(&str, &str, &[&str]); 7] = [
    ("851-3147", "enemies", &["Enemies"]),
    ("851-3152", "items", &["Items", "Collectibles"]),
    ("851-3157", "traps", &["Traps"]),
    ("851-3158", "effects", &["Effects"]),
    ("851-3146", "terrain", &["Blocks"]),
    ("851-3151", "characters", &["Characters"]),
    (
        "851-3159",
        "game flow",
        &["(top level)", "Basis", "Init", "Other"],
    ),
];

/// Object name to resource-tree folder, from the inventory's object tables.
pub fn object_folders(inventory: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut folder = None;
    for line in inventory.lines() {
        if let Some(h) = line.strip_prefix("#### ") {
            // "Enemies (40 objects, 5263 code lines)"
            folder = h.rsplit_once(" (").map(|(name, _)| name.to_string());
        } else if line.starts_with("## ") {
            folder = None;
        } else if let (Some(f), Some(row)) = (&folder, line.strip_prefix("| o")) {
            let name = row.split('|').next().unwrap_or("").trim();
            out.insert(format!("o{name}"), f.clone());
        }
    }
    out
}

/// The ticket an object's behavior belongs to. The player is a character, though it sits at the top level.
pub fn ticket_of(object: &str, folders: &BTreeMap<String, String>) -> Option<&'static str> {
    if matches!(object, "oPlayer1" | "oCharacter") {
        return Some("851-3151");
    }
    let folder = folders.get(object)?;
    TICKETS
        .iter()
        .find(|(_, _, fs)| fs.contains(&folder.as_str()))
        .map(|(t, _, _)| *t)
}

/// The "objects by ticket" Markdown table: for each gameplay ticket, its objects that were active, that only
/// existed deactivated, and that were never created, by the sets [`Coverage`] collects.
pub fn object_table(
    active: &BTreeSet<&str>,
    existed: &BTreeSet<&str>,
    folders: &BTreeMap<String, String>,
) -> String {
    let mut out = String::from(
        "Objects by ticket. *Active* objects were in or near the view on some frame, so their events ran; \
         *inactive* ones only existed deactivated (generated, but never near the player); *never* ones \
         weren't created at all.\n\n| Ticket | Area | Active | Inactive only | Never |\n|---|---|---|---|---|\n",
    );
    for (ticket, area, _) in TICKETS {
        let objects: BTreeSet<&str> = folders
            .keys()
            .map(String::as_str)
            .chain(["oPlayer1", "oCharacter"])
            .filter(|o| ticket_of(o, folders) == Some(ticket))
            .collect();
        let pick =
            |f: &dyn Fn(&str) -> bool| objects.iter().copied().filter(|o| f(o)).collect::<Vec<_>>();
        let a = pick(&|o| active.contains(o));
        let i = pick(&|o| !active.contains(o) && existed.contains(o));
        let n = pick(&|o| !existed.contains(o));
        out.push_str(&format!(
            "| {ticket} | {area} | {} of {}: {} | {} | {} |\n",
            a.len(),
            objects.len(),
            a.join(" "),
            i.join(" "),
            n.join(" ")
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rng_is_stable() {
        // The sweep's replays are named by their seeds; the same seeds must give the same input everywhere.
        let mut r = Rng::new(12345, 1);
        let got: Vec<u64> = (0..3).map(|_| r.below(1000)).collect();
        let mut again = Rng::new(12345, 1);
        assert_eq!(got, (0..3).map(|_| again.below(1000)).collect::<Vec<_>>());
        assert_ne!(Rng::new(12345, 2).next(), Rng::new(12345, 1).next());
        assert_ne!(Rng::new(1, 1).next(), Rng::new(2, 1).next());
    }

    #[test]
    fn holds_never_overlap() {
        let mut g = Keys {
            events: BTreeMap::new(),
            free: [0; 256],
            end: 100,
        };
        assert!(g.hold(vk::JUMP, 10, 5));
        // Still held, then released on 15: the next press needs a frame up.
        assert!(!g.hold(vk::JUMP, 12, 5));
        assert!(!g.hold(vk::JUMP, 15, 5));
        assert!(g.hold(vk::JUMP, 16, 5));
        // A hold past the end has no release.
        assert!(g.hold(vk::LEFT, 98, 5));
        assert!(!g.hold(vk::LEFT, 100, 1) && !g.hold(vk::RIGHT, 100, 1));
        let presses = g
            .events
            .values()
            .flatten()
            .filter(|e| matches!(e, Event::Press(_)))
            .count();
        let releases = g
            .events
            .values()
            .flatten()
            .filter(|e| matches!(e, Event::Release(_)))
            .count();
        assert_eq!((presses, releases), (3, 2));
    }

    #[test]
    fn truncating() {
        let r = Replay::parse("seed 1\nframes 100\n10 hold Z 5\n50 hold left 40\n").unwrap();
        let t = truncate(&r, 52);
        assert_eq!(
            t.to_text(),
            "seed 1\nframes 52\n10 press Z\n15 release Z\n50 press left\n"
        );
    }

    #[test]
    fn a_level_counts_once_olevel_has_culled() {
        // Whether a frame with these active and deactivated instances counts.
        let counts = |active: &str, deactivated: &str| {
            let json =
                format!(r#"{{"frame":0,"instances":[{active}],"deactivated":[{deactivated}]}}"#);
            counts_as_active(&serde_json::from_str(&json).unwrap())
        };
        let trap = r#"{"id":9,"object":"oSmashTrapLit"}"#;
        let music = r#"{"id":2,"object":"oMusic"}"#;
        let loading = r#"{"id":1,"object":"oLevel","vars":{"darkness":1}}"#;
        let culling = r#"{"id":1,"object":"oLevel","vars":{"darkness":1,"offset":96}}"#;
        // A level's first frame: everything generated is active, though the persistent oMusic is still
        // deactivated from the level before.
        assert!(!counts(&format!("{loading},{trap}"), music));
        assert!(counts(culling, &format!("{music},{trap}")));
        // Rooms without an oLevel, and the pause screen (oLevel deactivated).
        assert!(counts(r#"{"id":3,"object":"oTitle"}"#, ""));
        assert!(counts(r#"{"id":4,"object":"oScreen"}"#, loading));
    }

    #[test]
    fn folders() {
        let inv = "## 2. Objects\n#### Enemies (2 objects, 3 code lines)\n\n| Object | Parent |\n|---|---|\n\
                   | oBat | oEnemy |\n#### (top level) (1 objects, 1 code lines)\n| oGame | - |\n## 3. Scripts\n| oNot | x |\n";
        let f = object_folders(inv);
        assert_eq!(f.len(), 2);
        assert_eq!(ticket_of("oBat", &f), Some("851-3147"));
        assert_eq!(ticket_of("oGame", &f), Some("851-3159"));
        assert_eq!(ticket_of("oPlayer1", &f), Some("851-3151"));
        assert_eq!(ticket_of("oNot", &f), None);
    }
}
