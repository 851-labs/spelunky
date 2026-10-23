//! Plays a sound effect and cross-fades from the title music to the cave music, calling the `SS_*` scripts
//! as the game does and feeding the model's commands to the device backend, one tick every 1/30 s.
//!
//! ```sh
//! scripts/extract                                             # once: writes assets/sounds
//! scripts/cargo run -p audio --example play --features backend [-- <sounds dir>]
//! ```

use audio::backend::Player;
use audio::{Mode, Script, SoundBank, SuperSound, TICKS_PER_SECOND};
use gm::Value;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::slice::from_ref;
use std::sync::Arc;
use std::time::{Duration, Instant};

const FILES: [&str; 4] = ["jump.wav", "coin.wav", "title.ogg", "cave.ogg"];

/// When the cave music starts, and how many ticks the cross-fade takes (two seconds).
const FADE_START: u32 = 90;
const FADE: u32 = 60;

/// What a game override does: run the script, set the globals it assigns (here, just report them),
/// and hand back its value.
fn run(ss: &mut SuperSound, script: Script, args: &[Value]) -> Result<Value, gm::ValueError> {
    let outcome = ss.call(script, args)?;
    if !outcome.globals.is_empty() {
        println!(
            "{} sets {} globals, {} = {} to {} = {}",
            script.name(),
            outcome.globals.len(),
            outcome.globals[0].0,
            outcome.globals[0].1,
            outcome.globals[outcome.globals.len() - 1].0,
            outcome.globals[outcome.globals.len() - 1].1,
        );
    }
    Ok(outcome.value)
}

fn main() -> Result<(), Box<dyn Error>> {
    let dir = std::env::args().nth(1).map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/sounds"),
        PathBuf::from,
    );
    let mut bank = SoundBank::new();
    for name in FILES {
        let file = dir.join(name);
        let bytes = std::fs::read(&file)
            .map_err(|e| format!("{}: {e} (run scripts/extract first)", file.display()))?;
        // The game loads its files as `sound/<name>`, relative to its directory.
        bank.insert(&format!("sound/{name}"), bytes)?;
    }
    // The model and the player share one bank, so they agree on every file.
    let bank = Arc::new(bank);
    let mut player = Player::new(bank.clone())?;
    let mut ss = SuperSound::new(bank, Mode::Faithful);
    run(&mut ss, Script::Init, &[])?;
    let mut load = |name: &str, stream: f64| {
        let path = format!("sound/{name}");
        run(
            &mut ss,
            Script::LoadSound,
            &[path.as_str().into(), stream.into()],
        )
    };
    let jump = load("jump.wav", 0.0)?;
    let coin = load("coin.wav", 0.0)?;
    let title = load("title.ogg", 1.0)?;
    let cave = load("cave.ogg", 1.0)?;

    // startMusic's volume at the default music volume, 15 of 17.
    let music_volume = 2000.0 + 8000.0 * (15.0 / 18.0);
    run(&mut ss, Script::LoopSound, from_ref(&title))?;
    run(
        &mut ss,
        Script::SetSoundVol,
        &[title.clone(), music_volume.into()],
    )?;

    let start = Instant::now();
    let tick_length = Duration::from_secs(1) / TICKS_PER_SECOND as u32;
    for tick in 0..=FADE_START + FADE + 30 {
        if tick == 30 || tick == 45 {
            run(&mut ss, Script::PlaySound, from_ref(&jump))?;
        }
        if tick == FADE_START {
            run(&mut ss, Script::LoopSound, from_ref(&cave))?;
            run(&mut ss, Script::SetSoundVol, &[cave.clone(), 0.0.into()])?;
        }
        if tick > FADE_START && tick <= FADE_START + FADE {
            // Linear in the game's 0..10000 scale, which is linear in decibels.
            let x = f64::from(tick - FADE_START) / f64::from(FADE);
            let title_volume = music_volume * (1.0 - x);
            run(
                &mut ss,
                Script::SetSoundVol,
                &[title.clone(), title_volume.into()],
            )?;
            run(
                &mut ss,
                Script::SetSoundVol,
                &[cave.clone(), (music_volume * x).into()],
            )?;
        }
        if tick == FADE_START + FADE {
            run(&mut ss, Script::StopSound, from_ref(&title))?;
            run(&mut ss, Script::PlaySound, from_ref(&coin))?;
        }
        if tick % 30 == 0 {
            let mut query =
                |script: Script, h: &Value| run(&mut ss, script, from_ref(h)).map(|v| v.to_real());
            println!(
                "{:>3.1} s: title playing {} at volume {:>6}; cave playing {} at volume {:>6}, \
                 position {:>5}; jump playing {}",
                f64::from(tick) / TICKS_PER_SECOND as f64,
                query(Script::IsSoundPlaying, &title)?,
                query(Script::GetSoundVol, &title)?,
                query(Script::IsSoundPlaying, &cave)?,
                query(Script::GetSoundVol, &cave)?,
                query(Script::GetSoundPosition, &cave)?,
                query(Script::IsSoundPlaying, &jump)?,
            );
        }
        // The end of a frame: advance the model, then bring the device up to date with it.
        ss.tick();
        player.apply(&ss.take_commands())?;
        let next = start + tick_length * (tick + 1);
        std::thread::sleep(next.saturating_duration_since(Instant::now()));
    }
    Ok(())
}
