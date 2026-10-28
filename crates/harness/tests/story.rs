//! Story facts (`harness::story`) checked on the real game: `action-throw-jar-snake.txt`, where the player picks
//! up a jar (636), throws it (727), and it breaks on a snake (730), which dies (731).

mod common;

use common::{assets, replay};
use harness::story::{Story, Watcher};
use harness::{RunOptions, run_replay_watched};

/// What fails of `facts` (a `# story:` line's) on the replay's run.
fn failed(facts: &str) -> Result<Vec<String>, String> {
    let story = Story::parse(&format!("# story: {facts}")).unwrap();
    let mut watcher = Watcher::new(&story);
    run_replay_watched(
        &replay("action-throw-jar-snake.txt"),
        assets(),
        &RunOptions::default(),
        &mut |frame, w| watcher.frame(frame, w),
    )
    .unwrap();
    watcher.finish()
}

#[test]
fn facts_hold_where_the_run_shows_them_and_say_why_not_elsewhere() {
    // What happens, each in its window or anywhere in the run.
    let holds = "room=rLevel@565 held=oJar@636 held!=oJar@727 -oJar@730 -oSnake@731 kills=1@731 \
                 +oJar*7@565 +oSnake -oJar*2 kills>0 !plife<4 !-oSnake@..700 !held=oJar@740..780";
    assert_eq!(failed(holds), Ok(vec![]));
    // A condition must become true in its window, not merely be true there: the player still holds the
    // jar at 700, but picked it up at 636.
    assert_eq!(
        failed(
            "held=oJar@700..720 -oSnake@800 -oJar*3 !kills=1 kills=2 +oGhost !held=oJar@700..720"
        ),
        Ok([
            "held=oJar@700..720: not in 700..=720 (first at 636)",
            "-oSnake@800: not in 770..=830 (first at 731)",
            "-oJar*3: only 2 of 3",
            "!kills=1: happened (at 731)",
            "kills=2: never happened",
            "+oGhost: never happened",
            // A negated one mustn't be true on any frame of its window, though it became true before.
            "!held=oJar@700..720: happened in 700..=720 (at 700)",
        ]
        .map(String::from)
        .to_vec())
    );
    // A name the program doesn't have fails the story.
    assert_eq!(
        failed("plife<1 nosuchglobal=1"),
        Err("nosuchglobal=1: no global nosuchglobal".to_string())
    );
    assert!(failed("held=oNothing").is_err());
}

#[test]
fn a_built_in_holds_or_fails_the_story() {
    // The player enters at y 40 (565), ducks to pick up the jar (636), and falls on down the level; the
    // snake starts walking (566), its image_index going 0.4 a step.
    let holds = "oPlayer1.y=40@565 oPlayer1.y>300@724 !oPlayer1.y<40@565.. oPlayer1.visible=1 \
                 oPlayer1.sprite_index=sDuckLeft@636..636 !oPlayer1.sprite_index=sDuckLeft@637.. \
                 oPlayer1.object_index=oPlayer1 oSnake.sprite_index=sSnakeWalkL@566..566 \
                 oSnake.image_index>3.9@575..575 oSnake.mask_index=-1";
    assert_eq!(failed(holds), Ok(vec![]));
    // A wrong value or sprite fails, as does a negated fact that becomes true.
    assert_eq!(
        failed(
            "oPlayer1.y=41@565 oPlayer1.sprite_index=sDuckLeft@700 oPlayer1.sprite_index=sDamselLeft \
             !oPlayer1.sprite_index=sDuckLeft oPlayer1.object_index=oSnake !oPlayer1.visible=1@600.."
        ),
        Ok([
            "oPlayer1.y=41@565: not in 535..=595 (first at 612)",
            "oPlayer1.sprite_index=sDuckLeft@700: not in 670..=730 (first at 636)",
            "oPlayer1.sprite_index=sDamselLeft: never happened",
            "!oPlayer1.sprite_index=sDuckLeft: happened (at 636)",
            "oPlayer1.object_index=oSnake: never happened",
            "!oPlayer1.visible=1@600..: happened in 600.. (at 600)",
        ]
        .map(String::from)
        .to_vec())
    );
    // A sprite, a variable or a built-in the fact can't read fails the story.
    for (fact, why) in [
        ("oPlayer1.sprite_index=sNoSuch", "no sprite sNoSuch"),
        ("oPlayer1.object_index=oNoSuch", "no object oNoSuch"),
        ("oPlayer1.nosuch=1", "oPlayer1 has no variable nosuch"),
        ("oPlayer1.room=1", "room isn't an instance's"),
        ("oPlayer1.path_index=-1", "path_index isn't supported"),
        ("oPlayer1.alarm=1", "alarm isn't supported"),
        // A string never equals a number: `!oPlayer1.x=l6` would hold vacuously.
        ("!oPlayer1.x=l6", "x compares with a number"),
    ] {
        assert_eq!(failed(fact), Err(format!("{fact}: {why}")));
    }
}

#[test]
fn a_global_arrays_element_holds_or_fails_the_story() {
    // The snake's death counts in `global.enemyKills[1]` (oSnake's step), `[1]` being `[0,1]`.
    assert_eq!(
        failed("enemyKills[1]=0 enemyKills[1]=1@731 enemyKills[0,1]=1@731 !enemyKills[1]>1"),
        Ok(vec![])
    );
    // A wrong value fails, as does a negated fact that becomes true. scrReadStats sets 22 elements, so
    // `enemyKills[22]` is unset: neither = nor != anything, and a fact on it fails, negated too.
    assert_eq!(
        failed(
            "enemyKills[1]=2 !enemyKills[1]>0@700.. enemyKills[1]=1@600 enemyKills[22]=0 !enemyKills[22]>0"
        ),
        Ok([
            "enemyKills[1]=2: never happened",
            "!enemyKills[1]>0@700..: happened in 700.. (at 731)",
            "enemyKills[1]=1@600: not in 570..=630 (first at 731)",
            "enemyKills[22]=0: never set",
            "!enemyKills[22]>0: never set",
        ]
        .map(String::from)
        .to_vec())
    );
    assert_eq!(
        failed("enemyKills[1]=1 nosuch[3]=0"),
        Err("nosuch[3]=0: no global nosuch".to_string())
    );
}
