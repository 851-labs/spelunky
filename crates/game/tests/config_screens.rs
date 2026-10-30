//! The control config rooms left over from `config.gmk` (851-3165): `rKeyConfig` (`oKeyConfig`) and
//! `rJoyConfig` (`oJoyConfig`). Nothing in Spelunky 1.1 reaches them: their only `room_goto`s are in
//! `oKeyConfigButton` and `oJoyConfigButton`, which no room places and no code creates, and no
//! `room_goto_next` runs next to them in the room order. So neither the oracle nor a replay can enter them
//! (a replay's `room rKeyConfig` skips `scrInit`, and the room's draw event then reads the unset
//! `global.myFontSmall`). These tests boot the game normally and then send it there by hand, to pin down
//! what their GML does: rebinding, `room_end` writing the `.cfg` file, and the dangling `room_goto(rInit)`
//! that ends both (`docs/quirks.md` §1).

mod common;

use std::sync::{Arc, OnceLock};

use game::Game;
use gm::{GVar, InputEvent, Value};
use trace::replay::{Button, Replay};
use transpile::reference;

fn assets() -> Arc<gm::Assets> {
    static GMK: OnceLock<gmk::Gmk> = OnceLock::new();
    let gmk = GMK.get_or_init(|| reference::load_gmk(&reference::default_path()));
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

fn global(game: &Game, name: &str) -> Value {
    game.world().gget(gvar(name)).unwrap()
}

/// A game booted with this replay header (so `scrInit` has read its config files and created
/// `oGamepad`), a few frames into `rIntro`, then sent to `room`.
fn game_in(header: &str, room: &str) -> Game {
    let replay = Replay::parse(header).unwrap();
    let mut game = Game::new(assets(), game::Config::from_replay(&replay.header));
    game.start().unwrap();
    for _ in 0..3 {
        step(&mut game, &[]).unwrap();
    }
    let id = game.world().assets().room_id(room).unwrap();
    game.world_mut().room_goto(id);
    step(&mut game, &[]).unwrap();
    assert_eq!(game.world().room().id, id);
    game
}

fn step(game: &mut Game, events: &[InputEvent]) -> Result<(), String> {
    game.step(events).map_err(|e| game::describe(&e))
}

/// Presses and releases `b` over two frames.
fn tap(game: &mut Game, b: Button) -> Result<(), String> {
    step(game, &[InputEvent::Press(b)])?;
    step(game, &[InputEvent::Release(b)])
}

const KEYS: [&str; 12] = [
    "keyUpVal",
    "keyDownVal",
    "keyLeftVal",
    "keyRightVal",
    "keyJumpVal",
    "keyAttackVal",
    "keyItemVal",
    "keyRunVal",
    "keyBombVal",
    "keyRopeVal",
    "keyFlareVal",
    "keyPayVal",
];

const VK_ESCAPE: u8 = 27;

#[test]
fn each_key_press_binds_one_key() {
    // The "any key" press event runs once per press (docs/quirks.md §5), so each press rebinds the next
    // action: W for up, Escape keeps down, A for left.
    let mut game = game_in("frames 1\n", "rKeyConfig");
    for vk in [b'W', VK_ESCAPE, b'A'] {
        tap(&mut game, Button::Key(vk)).unwrap();
    }
    let got: Vec<Value> = KEYS[..7].iter().map(|n| global(&game, n)).collect();
    let want = [87.0, 40.0, 65.0, 39.0, 90.0, 88.0, 67.0].map(Value::Real);
    assert_eq!(got, want);
}

#[test]
fn the_key_config_ends_in_the_dangling_rinit() {
    let mut game = game_in("frames 1\n", "rKeyConfig");
    let before = game.world().host().files["keys.cfg"].clone();
    for _ in 0..11 {
        tap(&mut game, Button::Key(b'Q')).unwrap();
    }
    // The 12th key binds pay, then `room_goto(rInit)` reads `rInit`, which only `config.gmk` defines.
    let err = step(&mut game, &[InputEvent::Press(Button::Key(b'P'))]).unwrap_err();
    assert!(
        err.starts_with("oKeyConfig/") && err.ends_with(": unknown variable rInit"),
        "{err}"
    );
    assert_eq!(global(&game, "keyRopeVal"), Value::Real(81.0));
    assert_eq!(global(&game, "keyFlareVal"), Value::Real(81.0));
    assert_eq!(global(&game, "keyPayVal"), Value::Real(80.0));
    // The room never ends, so `keys.cfg` is never written.
    assert_eq!(game.world().host().files["keys.cfg"], before);
}

#[test]
fn leaving_the_key_config_writes_keys_cfg_and_the_game_reads_it_back() {
    let mut game = game_in("frames 1\n", "rKeyConfig");
    for vk in [b'W', b'W', VK_ESCAPE, VK_ESCAPE, 32, 32, b'E', b'E'] {
        tap(&mut game, Button::Key(vk)).unwrap();
    }
    // No input leaves the room without the rInit error; any other room change runs its room_end.
    let title = game.world().assets().room_id("rTitle").unwrap();
    game.world_mut().room_goto(title);
    step(&mut game, &[]).unwrap();
    let written = game.world().host().files["keys.cfg"].clone();
    // `string()` of each real, joined by CRLF, with no trailing newline: what config.exe writes too.
    assert_eq!(
        written.as_bytes(),
        b"87\r\n87\r\n37\r\n39\r\n32\r\n32\r\n69\r\n69\r\n65\r\n83\r\n70\r\n80"
    );
    // The same file as a replay's `keys` line (fidelity/replays/config-keys-tutorial.txt), and a new
    // session's scrInit reads the bindings back.
    let header = "keys 87 87 37 39 32 32 69 69 65 83 70 80\nframes 1\n";
    let replay = Replay::parse(header).unwrap();
    assert_eq!(
        gm::Host::from_replay(&replay.header).files["keys.cfg"],
        written
    );
    let mut next = Game::new(assets(), game::Config::from_replay(&replay.header));
    next.start().unwrap();
    for name in KEYS {
        assert_eq!(global(&next, name), global(&game, name), "{name}");
    }
}

#[test]
fn without_a_joystick_the_joy_config_only_skips_with_escape() {
    let mut game = game_in("frames 1\n", "rJoyConfig");
    for _ in 0..8 {
        tap(&mut game, Button::Key(VK_ESCAPE)).unwrap();
    }
    // Nothing was rebound: the shipped gamepad.cfg.
    assert_eq!(global(&game, "joyJumpVal"), Value::Real(2.0));
    assert_eq!(global(&game, "joyPayVal"), Value::Real(6.0));
    let err = step(&mut game, &[InputEvent::Press(Button::Key(VK_ESCAPE))]).unwrap_err();
    assert!(
        err.starts_with("oJoyConfig/") && err.ends_with(": unknown variable rInit"),
        "{err}"
    );
}

#[test]
fn with_a_joystick_the_joy_config_rebinds_buttons() {
    // Port only: the oracle's joystick functions are stubs that report none.
    let mut game = game_in("joystick\nframes 1\n", "rJoyConfig");
    // Button 4 for jump, Escape keeps attack, button 1 for item.
    tap(&mut game, Button::Joy(4)).unwrap();
    tap(&mut game, Button::Key(VK_ESCAPE)).unwrap();
    tap(&mut game, Button::Joy(1)).unwrap();
    assert_eq!(global(&game, "joyJumpVal"), Value::Real(4.0));
    assert_eq!(global(&game, "joyAttackVal"), Value::Real(1.0));
    assert_eq!(global(&game, "joyItemVal"), Value::Real(1.0));
    let title = game.world().assets().room_id("rTitle").unwrap();
    game.world_mut().room_goto(title);
    step(&mut game, &[]).unwrap();
    assert_eq!(
        game.world().host().files["gamepad.cfg"].as_bytes(),
        b"4\r\n1\r\n1\r\n5\r\n7\r\n8\r\n4\r\n6\r\n10"
    );
}
