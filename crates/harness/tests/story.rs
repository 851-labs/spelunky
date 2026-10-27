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
