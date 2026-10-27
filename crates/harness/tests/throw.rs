//! `route::thrown_hit`, the throw kill goal's test (`area-route --steps /<object>`), on the real game: a
//! thrown jar's, skull's and rock's kills count, and stomps beside a thrown jar that is still flying don't.

mod common;

use common::{assets, replay};
use game::replay::Playback;
use harness::route::{Goal, held, hurt, step_events, thrown_hit};
use trace::Replay;

/// The frames of a replay, from when the player first holds an item, on which an instance of `object` is
/// hurt (dies or loses hit points), each with
/// whether `thrown_hit` says the item the player last held hurt it, and whether that item is out after the
/// frame: thrown, and not broken.
fn hits(replay: &Replay, object: &str) -> Vec<(u64, bool, bool)> {
    let mut p = Playback::boot(assets(), replay).unwrap();
    let mut out = Vec::new();
    let mut item = None;
    loop {
        let before = p.game().clone();
        let frame = p.next_frame();
        item = held(&before).map(|(_, i)| i.id).or(item);
        if p.step().unwrap().is_none() {
            break;
        }
        let after = p.game();
        let hurt = hurt(&Goal::alive(&before, object), &Goal::alive(after, object));
        let Some(id) = item.filter(|_| !hurt.is_empty()) else {
            continue;
        };
        let input: Vec<_> = replay
            .events()
            .iter()
            .filter(|(f, _)| *f == frame)
            .map(|&(_, e)| e)
            .collect();
        let thrown = thrown_hit(&before, after, object, id, |g| {
            step_events(g, &input).is_ok()
        });
        let w = after.world();
        let out_now = held(after).is_none_or(|(_, i)| i.id != id)
            && w.active().any(|h| w.instance(h).id == id);
        out.push((frame, thrown, out_now));
    }
    out
}

fn fixture(name: &str) -> Replay {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    Replay::parse(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn a_thrown_skull_that_breaks_on_a_snake_kills_it() {
    // The thrown skull breaks on the snake it kills (792); a bomb kills another (990, since GM8's reactivation
    // order, 851-3311).
    let found = hits(&replay("action-skull-bomb-rope-arrow.txt"), "oSnake");
    assert_eq!(found, [(792, true, false), (990, false, false)]);
}

#[test]
fn a_thrown_jar_that_breaks_on_a_snake_kills_it() {
    let found = hits(&replay("action-throw-jar-snake.txt"), "oSnake");
    assert_eq!(found, [(730, true, false)]);
}

#[test]
fn a_thrown_rock_that_flies_on_kills_a_snake() {
    // The rock flies on past the snake it kills.
    let found = hits(&replay("action-throw-rock-snake.txt"), "oSnake");
    assert_eq!(found, [(1102, true, true)]);
}

/// `area-route --steps '+oJar,/oSnake'` (and `/oFrog`) once took these stomps, with the thrown jar flying
/// nearby, for jar kills: its test measured the player's distance from 8 px past the target's middle, so a
/// stomp, 16 px above it, passed for a throw (851-3296). The jar outlives each target.
#[test]
fn a_stomp_beside_a_thrown_jar_is_no_throw_kill() {
    let found = hits(&fixture("stomp-beside-jar-mines-seed-3.txt"), "oSnake");
    assert_eq!(found, [(746, false, true)]);
    let found = hits(&fixture("stomp-beside-jar-jungle-seed-15.txt"), "oFrog");
    assert_eq!(found, [(995, false, true)]);
}
