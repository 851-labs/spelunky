//! A clip's sound (`Clip::sound`), mixed from the real game's run with its own sound files: `snake/bite`,
//! where the level's music plays and the snake bites the player on scene frame 125.

mod common;

use std::io::Read as _;
use std::path::Path;
use std::sync::Arc;

use audio::offline::FRAMES_PER_TICK;
use game::audio::SoundBank;
use harness::clip::Clip;

/// The game's `sound/` files, from the reference game zip, as `scripts/extract` ships them, with the music
/// (the `.ogg`s) only if `music`.
fn sounds(music: bool) -> Arc<SoundBank> {
    let path = transpile::reference::default_path().with_file_name("spelunky_1_1.zip");
    let mut zip = zip::ZipArchive::new(std::fs::File::open(&path).unwrap()).unwrap();
    let mut bank = SoundBank::new();
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).unwrap();
        let name = file.name().unwrap().into_owned();
        if name.starts_with("sound/") && file.is_file() && (music || !name.ends_with(".ogg")) {
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes).unwrap();
            // As the game would: a file the model can't read is one SS_LoadSound can't load.
            let _ = bank.insert(&name, bytes);
        }
    }
    assert!(bank.len() > 40, "the game's sounds: {}", bank.len());
    Arc::new(bank)
}

fn bite(music: bool) -> Vec<f32> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let clip = Clip::read(&repo, &repo.join("wiki/clips/snake/bite.toml")).unwrap();
    let source = clip.source(&repo).unwrap();
    clip.sound(&source, common::assets(), sounds(music))
        .unwrap()
}

/// The loudest sample from the clip's frame `from` up to `to`.
fn peak(sound: &[f32], from: usize, to: usize) -> f32 {
    let tick = FRAMES_PER_TICK * 2;
    (sound[from * tick..to * tick].iter()).fold(0.0, |m, s| m.max(s.abs()))
}

#[test]
fn a_clip_sounds_the_same_every_time_and_lasts_its_frames() {
    let (a, b) = (bite(true), bite(true));
    // Frames 40 to 149 of the scene.
    assert_eq!(a.len(), 110 * FRAMES_PER_TICK * 2);
    assert!(a.iter().zip(&b).all(|(x, y)| x.to_bits() == y.to_bits()));
    // The level's music, already playing when the clip starts.
    assert!(peak(&a, 0, 1) > 0.05);
}

#[test]
fn a_frames_sound_starts_with_its_picture() {
    // Without the music, which SS_LoadSound then can't load (a game that plays the same), the clip is silent
    // until the bite's sound, sound/hurt.wav, which starts on the bite's frame and lasts 0.13 s.
    let sound = bite(false);
    let tick = FRAMES_PER_TICK * 2;
    let heard: Vec<usize> = (sound.chunks_exact(tick).enumerate())
        .filter(|(_, t)| t.iter().any(|&s| s != 0.0))
        .map(|(i, _)| i + 40)
        .collect();
    assert_eq!(heard, [125, 126, 127, 128]);
}
