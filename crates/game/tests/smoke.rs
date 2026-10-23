//! The generated game builds its program and starts. Many built-ins aren't there yet, so this runs the game
//! from its first room until something stops it, and checks that what stops it is a located, ticketed gap
//! (`Unsupported`), not a crash.

use std::sync::{Arc, OnceLock};

use game::audio::Mode;
use gm::{Error, Event, ObjectId, Program};
use trace::Value;
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
    // Backgrounds by size, so runtime tiles work; no sprites (their masks come from the extractor).
    let backgrounds = gmk
        .backgrounds
        .iter()
        .map(|b| {
            b.as_ref().map(|b| assets::Background {
                name: b.name.clone(),
                file: None,
                width: b.image.width,
                height: b.image.height,
                use_as_tileset: b.use_as_tileset,
                tile_width: b.tile_width,
                tile_height: b.tile_height,
                h_offset: b.h_offset,
                v_offset: b.v_offset,
                h_sep: b.h_sep,
                v_sep: b.v_sep,
            })
        })
        .collect();
    let assets = gm::Assets::new(vec![None; gmk.sprites.len()], gmk.rooms.clone())
        .with_room_order(gmk.room_order.clone())
        .with_backgrounds(backgrounds);
    // Both audio modes boot past `oMusic`'s create event, which initializes SuperSound and loads every
    // sound. With no sound files, the faithful DLL answers "No" for each.
    for (mode, mus_title) in [
        (Mode::OracleCompat, Value::Real(0.0.into())),
        (Mode::Faithful, Value::Str("No".into())),
    ] {
        let config = game::Config {
            audio: mode,
            ..game::Config::default()
        };
        let mut game = game::Game::new(Arc::new(assets.clone()), config);
        if let Err(e) = game.start() {
            assert!(!e.unit.starts_with("oMusic"), "{e}");
        }
        let globals = game.trace_with(true).globals.unwrap();
        assert_eq!(globals["dll_SS_PauseSound"], Value::Real(21.0.into()));
        assert_eq!(globals["musTitle"], mus_title, "{mode:?}");
    }
    let mut game = game::Game::new(Arc::new(assets), game::Config::default());
    // Up to a gap the port knows about: an unimplemented built-in or override, naming its ticket, reported
    // at the GML unit that hit it.
    let gap = |game: &game::Game, e: &gm::GmlError| {
        assert!(
            matches!(e.error, Error::Unsupported(what) if what.contains("(851-")),
            "{e}"
        );
        assert!(!e.unit.is_empty(), "{e}");
        eprintln!(
            "frame {}, {}: stops at {}",
            game.world().frames(),
            game.trace().room_name.unwrap_or_default(),
            game::describe(e)
        );
    };
    match game.start() {
        Ok(()) => {
            for _ in 0..100 {
                if let Err(e) = game.step(&[]) {
                    gap(&game, &e);
                    break;
                }
            }
        }
        Err(e) => gap(&game, &e),
    }
    // The first room's instances are placed.
    assert!(!game.trace().instances.is_empty());
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
