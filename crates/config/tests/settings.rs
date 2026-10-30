//! The settings program headless, on `config.gmk`'s rooms, against expectations computed by hand from its
//! GML (`oInit`, `scrSaveSettings`, `scrExit`, `oKeyConfig`). The oracle runs `config.exe` too
//! (`scripts/fidelity --settings` on `fidelity/settings/`), except `execute_program` (`docs/quirks.md`
//! section 7).

use std::sync::{Arc, OnceLock};

use game::{Config, Game};
use gm::{Ctx, Event, InputEvent, Inst, ObjectId, Value};
use trace::replay::{Button, Replay};
use transpile::reference::{self, Program};

fn gmk() -> &'static gmk::Gmk {
    static GMK: OnceLock<gmk::Gmk> = OnceLock::new();
    GMK.get_or_init(|| reference::load_program(&reference::default_path(), Program::Config))
}

/// `config.gmk`'s rooms, without images.
fn assets() -> Arc<gm::Assets> {
    Arc::new(reference::stub_assets(gmk()))
}

/// The settings program booted on `header`'s game files, in `rInit` after its first frame.
fn settings(header: &str) -> Game {
    let replay = Replay::parse(header).unwrap();
    let mut game = config::new(assets(), Config::from_replay(&replay.header));
    game.start().unwrap();
    step(&mut game, &[]).unwrap();
    assert_eq!(room(&game), "rInit");
    game
}

fn step(game: &mut Game, events: &[InputEvent]) -> Result<(), String> {
    game.step(events).map_err(|e| game.describe(&e))
}

fn tap(game: &mut Game, b: Button) {
    step(game, &[InputEvent::Press(b)]).unwrap();
    step(game, &[InputEvent::Release(b)]).unwrap();
}

fn room(game: &Game) -> &str {
    let w = game.world();
    w.assets().room_name(w.room().id).unwrap()
}

fn global(game: &Game, name: &str) -> Value {
    let v =
        game::global_named(config::program(), name).unwrap_or_else(|| panic!("no global {name}"));
    game.world().gget(v).unwrap()
}

fn file(game: &Game, name: &str) -> String {
    String::from_utf8(game.world().host().files[name].as_bytes().to_vec()).unwrap()
}

const VK_RETURN: u8 = 13;
const VK_ESCAPE: u8 = 27;

#[test]
fn it_starts_in_rinit_with_the_settings_it_reads() {
    // fullscreen off, high detail, down-to-run off, gamepad on, scale 2, music 9, sound 30 (clamped to 19).
    let game = settings("settings 0 1 0 1 2 9 30\nframes 1\n");
    let want = [
        ("fullscreen", 0.0),
        ("graphicsHigh", 1.0),
        ("downToRun", 0.0),
        ("gamepadOn", 1.0),
        ("screenScale", 2.0),
        ("musicVol", 9.0),
        ("soundVol", 19.0),
    ];
    for (name, v) in want {
        assert_eq!(global(&game, name), Value::Real(v), "{name}");
    }
    assert!(game.executed().is_empty());
    assert!(!game.ended());
}

#[test]
fn escape_saves_the_settings_and_quits_without_starting_the_game() {
    let mut game = settings("settings 0 1 0 1 2 9 30\nframes 1\n");
    step(&mut game, &[InputEvent::Press(Button::Key(VK_ESCAPE))]).unwrap();
    assert!(game.ended());
    // `string()` of each value, CRLF-separated, no newline at the end; the sound volume as clamped.
    assert_eq!(
        file(&game, "settings.cfg"),
        "0\r\n1\r\n0\r\n1\r\n2\r\n9\r\n19"
    );
    assert!(game.executed().is_empty());
    assert!(!config::launches_game(&game));
}

#[test]
fn the_close_button_saves_the_settings_and_quits() {
    // `oInit`'s close-button event: `scrSaveSettings(); game_end();` (851-3777).
    let mut game = settings("settings 0 1 0 1 2 9 30\nframes 1\n");
    step(&mut game, &[InputEvent::Close]).unwrap();
    assert!(game.ended());
    assert_eq!(
        file(&game, "settings.cfg"),
        "0\r\n1\r\n0\r\n1\r\n2\r\n9\r\n19"
    );
    assert!(!config::launches_game(&game));
}

#[test]
fn enter_quits_without_saving() {
    // `oInit`'s Enter press is a bare `game_end()`, unlike Escape and the close button.
    let mut game = settings("settings 0 1 0 1 2 9 30\nframes 1\n");
    step(&mut game, &[InputEvent::Press(Button::Key(VK_RETURN))]).unwrap();
    assert!(game.ended());
    assert_eq!(
        file(&game, "settings.cfg"),
        "0\r\n1\r\n0\r\n1\r\n2\r\n9\r\n30"
    );
    assert!(game.executed().is_empty());
}

#[test]
fn exiting_with_start_game_checked_asks_for_spelunky() {
    let mut game = settings("frames 1\n");
    // What the OK button runs (`oOKButton` left release): `scrSaveSettings(); scrExit();`. `oCheckBox5`
    // ("START GAME ON EXIT") starts checked.
    let cx = Ctx::event(Inst(0), Inst(0), Event::Create, ObjectId(0));
    config::overrides::scr::scrExit(game.world_mut(), &cx, &[]).unwrap();
    assert_eq!(
        game.executed(),
        [("Spelunky.exe".to_owned(), String::new())]
    );
    assert!(config::launches_game(&game));
    // `game_end()` takes effect at the end of the step, like GM8's.
    step(&mut game, &[]).unwrap();
    assert!(game.ended());
}

#[test]
fn the_key_config_writes_keys_cfg_and_returns_to_rinit() {
    let mut game = settings("frames 1\n");
    // `oKeyConfigButton` (a mouse button) does `scrSaveSettings(); room_goto(rKeyConfig)`.
    let id = game.world().assets().room_id("rKeyConfig").unwrap();
    game.world_mut().room_goto(id);
    step(&mut game, &[]).unwrap();
    assert_eq!(room(&game), "rKeyConfig");
    // The "any key" press event runs once per press (docs/quirks.md section 5), so each press binds the
    // next of config.exe's eleven actions (it has no flare binding: docs/quirks.md section 1), and the
    // eleventh binds pay and leaves for rInit.
    for vk in [
        b'W', b'W', VK_ESCAPE, VK_ESCAPE, 32, 32, b'E', b'E', b'Q', b'Q',
    ] {
        tap(&mut game, Button::Key(vk));
    }
    assert_eq!(room(&game), "rKeyConfig");
    tap(&mut game, Button::Key(b'P'));
    assert_eq!(room(&game), "rInit");
    // Escape keeps left and right; flare keeps the shipped F (70).
    assert_eq!(
        file(&game, "keys.cfg"),
        "87\r\n87\r\n37\r\n39\r\n32\r\n32\r\n69\r\n69\r\n81\r\n81\r\n70\r\n80"
    );
}
