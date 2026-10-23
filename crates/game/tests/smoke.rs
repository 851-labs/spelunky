//! The generated game builds its program and starts. Room loading (851-3128) and most built-ins aren't
//! there yet, so this only goes as far as the first room's create events, and checks that whatever stops
//! them is a located, ticketed gap (`Unsupported`), not a crash.

use std::sync::{Arc, OnceLock};

use gm::{Error, Event, ObjectId, Program};
use transpile::reference;

fn gmk() -> &'static gmk::Gmk {
    static GMK: OnceLock<gmk::Gmk> = OnceLock::new();
    GMK.get_or_init(|| reference::load_gmk(&reference::default_path()))
}

fn object(p: &Program, name: &str) -> ObjectId {
    (0..p.object_slots() as u32)
        .map(ObjectId)
        .find(|&o| p.object(o).is_some_and(|i| i.name == name))
        .unwrap_or_else(|| panic!("no object {name}"))
}

#[test]
fn the_program_has_every_object_and_event() {
    let gmk = gmk();
    let p = game::program();
    assert_eq!(p.object_slots(), gmk.objects.len());
    for (i, o) in gmk.objects.iter().enumerate() {
        let info = p.object(ObjectId(i as u32));
        assert_eq!(
            info.map(|i| i.name.as_str()),
            o.as_ref().map(|o| o.name.as_str())
        );
    }
    // Handlers resolve through parents; layouts hold what an object's code and its scripts assign.
    let player = object(p, "oPlayer1");
    let character = object(p, "oCharacter");
    assert_eq!(p.handler(player, Event::Step).unwrap().object, player);
    assert!(p.parents(player).starts_with(&[player, character]));
    let snake = object(p, "oSnake");
    assert!(p.handler(snake, Event::Create).is_some());
    assert!(p.layout(player).len() > 100, "{}", p.layout(player).len());
    assert_eq!(p.var_name(gm::Var(gm::FIRST_USER_VAR)), "ACTIVE");
}

#[test]
fn the_game_starts_until_a_ticketed_gap() {
    let gmk = gmk();
    let first = gmk.room_order[0];
    assert_eq!(
        gmk.rooms[first as usize].as_ref().unwrap().name,
        "rScreenInit"
    );
    let assets = gm::Assets::new(vec![None; gmk.sprites.len()], gmk.rooms.clone());
    let mut game = game::Game::new(Arc::new(assets), game::Config::default());
    match game.start(first) {
        Ok(()) => {}
        // A gap the port knows about: an unimplemented built-in or override, naming its ticket, reported
        // at the GML unit that hit it. Today that's `SS_Init` in `oMusic`'s create event (851-3177).
        Err(e) => {
            assert!(
                matches!(e.error, Error::Unsupported(what) if what.contains("(851-")),
                "{e}"
            );
            assert!(!e.unit.is_empty(), "{e}");
            eprintln!("the first room stops at {}", game::describe(&e));
        }
    }
    // The room's instances are placed.
    let placed = game.trace().instances.len();
    assert!(placed > 0);
    // A clone is a savestate: stepping the game doesn't touch it.
    let snapshot = game.clone();
    let before = snapshot.trace();
    let _ = game.step(&[]);
    assert_eq!(snapshot.trace(), before);
    assert_eq!(game.world().frames(), snapshot.world().frames() + 1);
}

#[test]
fn errors_name_their_variables() {
    let e = gm::GmlError {
        error: Error::UnsetVar(gm::Var(gm::FIRST_USER_VAR)),
        unit: "oX/step#0",
        line: 3,
    };
    assert_eq!(game::describe(&e), "oX/step#0:3: unknown variable ACTIVE");
}
