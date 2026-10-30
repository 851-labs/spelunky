//! The game's own file code on the virtual game directory (851-3164): `scrWriteStats` writes `stats.txt`,
//! `scrReadStats` reads it back, and the game boots past both and `scrInit`'s config reading.

mod common;

use std::sync::{Arc, OnceLock};

use game::overrides::scr;
use gm::{Ctx, Event, GVar, GmString, ObjectId, Value, World};
use transpile::reference;

fn gmk() -> &'static gmk::Gmk {
    static GMK: OnceLock<gmk::Gmk> = OnceLock::new();
    GMK.get_or_init(|| reference::load_gmk(&reference::default_path()))
}

/// The rooms and stub sprites.
fn assets() -> Arc<gm::Assets> {
    let gmk = gmk();
    Arc::new(
        gm::Assets::new(common::stub_sprites(gmk), gmk.rooms.clone())
            .with_room_order(gmk.room_order.clone())
            .with_backgrounds(common::stub_backgrounds(gmk)),
    )
}

fn gvar(name: &str) -> GVar {
    let p = game::program();
    (0..p.global_count() as u16)
        .map(GVar)
        .find(|&v| p.global_name(v) == name)
        .unwrap_or_else(|| panic!("no global {name}"))
}

/// The scalar stats and the array ones with their lengths, as `scrReadStats` and `scrWriteStats` use them.
const SCALARS: [&str; 15] = [
    "totalCrates",
    "totalChests",
    "idolsGrabbed",
    "idolsConverted",
    "damselsGrabbed",
    "kissesBought",
    "damselsBought",
    "damselsSavedTotal",
    "damselsKilledTotal",
    "itemsBought",
    "itemsStolen",
    "diceGamesPlayed",
    "diceGamesWon",
    "diceGamesLost",
    "totalMonkeyKills",
];
const ARRAYS: [(&str, u32); 4] = [
    ("levelDeaths", 16),
    ("enemyDeaths", 24),
    ("miscDeaths", 12),
    ("enemyKills", 22),
];

/// A world with one instance to run scripts as (their `file`, `str` and `i` are its variables).
fn world_and_ctx() -> (World, Ctx) {
    let p = game::program();
    let mut g = World::new(p, assets(), 0, 100_000, 0);
    // Any object without a create event will do.
    let obj = (0..p.object_slots() as u32)
        .map(ObjectId)
        .find(|&o| p.object(o).is_some() && p.handler(o, Event::Create).is_none())
        .unwrap();
    let id = g.instance_create(0.0, 0.0, obj.0 as i32).unwrap();
    let me = g.find(id).unwrap();
    (g, Ctx::event(me, me, Event::Create, obj))
}

/// Every stat, in a fixed order.
fn stats(g: &World) -> Vec<Value> {
    let mut out: Vec<Value> = SCALARS.iter().map(|n| g.gget(gvar(n)).unwrap()).collect();
    for (n, len) in ARRAYS {
        out.extend((0..len).map(|i| g.gget_a(gvar(n), 0, i).unwrap()));
    }
    out
}

#[test]
fn stats_survive_a_write_and_read() {
    let (mut g, cx) = world_and_ctx();
    // No stats.txt: every stat starts at 0.
    assert!(!g.host().files.contains_key("stats.txt"));
    scr::scrReadStats(&mut g, &cx, &[]).unwrap();
    assert!(stats(&g).iter().all(|v| *v == Value::Real(0.0)));

    // Distinct values, some of them more than one digit and one fractional, to catch misaligned columns.
    let mut k = 0.0;
    for n in SCALARS {
        k += 7.0;
        g.gset(gvar(n), Value::Real(k)).unwrap();
    }
    for (n, len) in ARRAYS {
        for i in 0..len {
            k += 1.0;
            // Zeros print as `???` rows; keep a few.
            let v = if i % 5 == 0 { 0.0 } else { k };
            g.gset_a(gvar(n), 0, i, Value::Real(v)).unwrap();
        }
    }
    g.gset(gvar("totalCrates"), Value::Real(1234.5)).unwrap();
    let written = stats(&g);
    scr::scrWriteStats(&mut g, &cx, &[]).unwrap();

    let text = g.host().files["stats.txt"].clone();
    let text = text.as_bytes();
    assert!(
        text.starts_with(
            b"+---------------------------+\r\n|  SPELUNKY EXTENDED STATS  |\r\n\
              +---------------------------+\r\n\r\nCRATES OPENED:      1234.50\r\n"
        ),
        "{}",
        String::from_utf8_lossy(text)
    );
    // Windows line ends only, and every handle closed.
    assert!(!text.windows(2).any(|w| w[1] == b'\n' && w[0] != b'\r'));
    for h in 1..=gm::TEXT_FILE_SLOTS as i32 {
        let mut host = g.host().clone();
        assert!(host.file_text_eof(h).is_err());
    }

    // Read back into zeroed stats. `string()` keeps two decimals, so 1234.5 comes back exactly.
    for n in SCALARS {
        g.gset(gvar(n), Value::Real(-1.0)).unwrap();
    }
    scr::scrReadStats(&mut g, &cx, &[]).unwrap();
    assert_eq!(stats(&g), written);

    // The same file with bare LF line ends reads the same.
    let lf: Vec<u8> = text.iter().copied().filter(|&b| b != b'\r').collect();
    g.host_mut()
        .files
        .insert("stats.txt".into(), GmString::from(lf));
    scr::scrReadStats(&mut g, &cx, &[]).unwrap();
    assert_eq!(stats(&g), written);
}

#[test]
fn the_game_boots_past_its_file_reading() {
    // A returning player's stats file and a remapped key, which `oScreen`'s create event reads with
    // `scrReadStats` and `scrInit` while the game boots.
    let (mut g, cx) = world_and_ctx();
    scr::scrReadStats(&mut g, &cx, &[]).unwrap();
    g.gset(gvar("totalChests"), Value::Real(42.0)).unwrap();
    scr::scrWriteStats(&mut g, &cx, &[]).unwrap();
    let mut config = game::Config::default();
    let files = &mut config.host.files;
    files.insert("stats.txt".into(), g.host().files["stats.txt"].clone());
    files.insert("keys.cfg".into(), "90\r\n40\r\n37\r\n39\r\n88".into());

    let mut game = game::Game::new(assets(), config);
    // The boot runs `oScreen`'s create event, file reading included, to the end.
    game.start()
        .unwrap_or_else(|e| panic!("{}", game::describe(&e)));
    let g = game.world();
    assert_eq!(g.gget(gvar("totalChests")).unwrap(), Value::Real(42.0));
    assert_eq!(g.gget(gvar("keyUpVal")).unwrap(), Value::Real(90.0));
    assert_eq!(g.gget(gvar("keyJumpVal")).unwrap(), Value::Real(88.0));
    // keys.cfg ends there: the rest read as "", and `real("")` is 0.
    assert_eq!(g.gget(gvar("keyAttackVal")).unwrap(), Value::Real(0.0));
    // The shipped settings.cfg and gamepad.cfg: `0,1,0,1,3,15,15` and start button 10.
    assert_eq!(g.gget(gvar("screenScale")).unwrap(), Value::Real(3.0));
    assert_eq!(g.gget(gvar("joyStartVal")).unwrap(), Value::Real(10.0));
    // Every file is closed again.
    assert_eq!(g.host().clone().file_text_open_read(b"keys.cfg"), Ok(1));
}

#[test]
fn the_close_button_writes_stats_txt() {
    // `oScreen`'s close-button event is `game_end()`, and its game end event `scrWriteStats()`: the only place
    // the game writes `stats.txt` (851-3777).
    let mut game = game::Game::new(assets(), game::Config::default());
    game.start()
        .unwrap_or_else(|e| panic!("{}", game::describe(&e)));
    for _ in 0..10 {
        game.step(&[]).unwrap();
    }
    assert!(!game.world().host().files.contains_key("stats.txt"));
    game.step(&[gm::InputEvent::Close]).unwrap();
    assert!(game.ended());
    assert!(game.world().host().files.contains_key("stats.txt"));
}
