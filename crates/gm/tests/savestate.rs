//! Savestates: a world saved to bytes and loaded back continues exactly like the original. The program
//! below touches every kind of state a frame depends on: the RNG, instances created and destroyed mid-run
//! (ids, list order), alarms, motion, animation, depth (draw order), user variables (scalars, arrays,
//! strings), globals, held and pressed keys, and deactivation.

mod common;

use std::io::Cursor;
use std::sync::Arc;

use common::*;
use gm::input::KeyState;
use gm::*;
use trace::diff::{DiffOptions, Outcome, diff_traces};
use trace::replay::Button;
use trace::{Filter, TraceReader, TraceWriter};

const ALARM: Var = Builtin::Alarm.var();
const SPAWNER: ObjectId = ObjectId(0);
const MOVER: ObjectId = ObjectId(1);
const COUNT: Var = Var(FIRST_USER_VAR);
const HIST: Var = Var(FIRST_USER_VAR + 1);
const NAME: Var = Var(FIRST_USER_VAR + 2);

fn real(x: f64) -> Value {
    Value::Real(x)
}

fn spawner_create(g: &mut World, cx: &mut Ctx) -> R<()> {
    g.set_a(cx.this, ALARM, 0, 0, real(3.0))?;
    g.set(cx.this, COUNT, real(0.0))
}

/// Spawns a mover somewhere random, every 1 to 4 frames.
fn spawner_alarm(g: &mut World, cx: &mut Ctx) -> R<()> {
    let (x, y) = (g.rng().random(300.0), g.rng().random(200.0));
    g.instance_create(x, y, MOVER.0 as i32)?;
    let next = 1.0 + g.rng().random_int(4) as f64;
    g.set_a(cx.this, ALARM, 0, 0, real(next))?;
    let count = g.get(cx.this, COUNT)?.to_real() + 1.0;
    g.set(cx.this, COUNT, real(count))?;
    let r = g.rng().random(10.0);
    g.set_a(cx.this, HIST, 0, count as u32, real(r))?;
    log(g, &format!("s{count}"));
    Ok(())
}

/// Z held: new random depths (so the draw order changes). X pressed: deactivate the movers; C pressed:
/// activate everything.
fn spawner_step(g: &mut World, cx: &mut Ctx) -> R<()> {
    if g.input().check(KeyState::Held, b'Z') {
        let d = g.rng().random(10.0).floor();
        g.instance_mut(cx.this).depth = d;
    }
    if g.input().check(KeyState::Pressed, b'X') {
        g.instance_deactivate_object(cx, MOVER.0 as i32);
    }
    if g.input().check(KeyState::Pressed, b'C') {
        g.instance_activate_all();
    }
    Ok(())
}

fn mover_create(g: &mut World, cx: &mut Ctx) -> R<()> {
    let hspeed = g.rng().random(4.0) - 2.0;
    let vspeed = -g.rng().random(3.0);
    let life = 10.0 + g.rng().random_int(20) as f64;
    let id = g.instance(cx.this).id;
    let i = g.instance_mut(cx.this);
    i.hspeed = hspeed;
    i.vspeed = vspeed;
    i.gravity = 0.2;
    i.image_speed = 0.3;
    i.depth = -(id % 5) as f64;
    g.set_a(cx.this, ALARM, 0, 1, real(life))?;
    g.set(cx.this, NAME, Value::from(format!("m{id}").as_str()))
}

fn mover_alarm(g: &mut World, cx: &mut Ctx) -> R<()> {
    g.instance_destroy(cx)
}

fn program() -> &'static Program {
    build(|b| {
        let (count, hist, name) = (b.var("count"), b.var("hist"), b.var("name"));
        assert_eq!((count, hist, name), (COUNT, HIST, NAME));
        let s = b.push_object(ObjectInfo {
            sprite: 0,
            ..ObjectInfo::new("spawner")
        });
        let m = b.push_object(ObjectInfo {
            sprite: 0,
            ..ObjectInfo::new("mover")
        });
        assert_eq!((s, m), (SPAWNER, MOVER));
        b.event(s, Event::Create, spawner_create)
            .event(s, Event::Alarm(0), spawner_alarm)
            .event(s, Event::Step, spawner_step)
            .event(m, Event::Create, mover_create)
            .event(m, Event::Alarm(1), mover_alarm)
            .layout(s, &[count, hist])
            .layout(m, &[name]);
    })
}

/// Loads a savestate with `like`'s program and assets.
fn reload(like: &World, bytes: &[u8]) -> Result<World, SavestateError> {
    World::load_state(
        like.program(),
        Arc::new(like.assets().clone()),
        FONT_SLOTS,
        bytes,
    )
}

fn assets() -> Assets {
    Assets::new(vec![Some(sprite("s", 4, 8))], vec![])
}

/// The input of frame `f`.
fn input(f: u64) -> Vec<InputEvent> {
    let key = |c: u8| Button::Key(c);
    match f {
        5 => vec![InputEvent::Press(key(b'Z'))],
        9 => vec![InputEvent::Release(key(b'Z'))],
        // Saved below between frames 24 and 25, with Z held: held keys must survive.
        22 => vec![InputEvent::Press(key(b'Z'))],
        28 => vec![InputEvent::Release(key(b'Z')), InputEvent::Press(key(b'X'))],
        29 => vec![InputEvent::Release(key(b'X'))],
        36 => vec![InputEvent::Press(key(b'C'))],
        _ => vec![],
    }
}

/// Runs frames `from..to`, returning the trace.
fn run(g: &mut World, from: u64, to: u64) -> Vec<u8> {
    let mut w = TraceWriter::new(Vec::new(), Filter::default());
    for f in from..to {
        assert_eq!(g.frames(), f);
        g.step(&input(f)).unwrap();
        w.write_frame(&g.trace_frame()).unwrap();
    }
    w.into_inner()
}

fn assert_same(a: &[u8], b: &[u8], frames: u64) {
    let read = |t: &[u8]| TraceReader::new(Cursor::new(t.to_vec())).unwrap();
    let report = diff_traces(read(a), read(b), &DiffOptions::default()).unwrap();
    assert_eq!(report.outcome, Outcome::Identical, "{}", report.to_text());
    assert_eq!(report.equal_frames, frames);
}

#[test]
fn a_loaded_savestate_continues_like_the_original() {
    let (n, m) = (25, 35);
    let p = program();
    let mut full = world_with(p, assets(), &[SPAWNER]);
    run(&mut full, 0, n);
    let bytes = full.save_state();
    let cloned = full.clone();
    let rest = run(&mut full, n, n + m);
    // Something happened in both halves: instances were created, destroyed and deactivated.
    assert!(full.trace_frame().instances.len() > 1);

    let mut loaded = reload(&full, &bytes).unwrap();
    assert_eq!(loaded.frames(), n);
    assert_eq!(loaded.save_state(), bytes, "saving is deterministic");
    assert_same(&run(&mut loaded, n, n + m), &rest, m);
    // The in-memory savestate too.
    let mut cloned = cloned;
    assert_same(&run(&mut cloned, n, n + m), &rest, m);
    assert_eq!(loaded.save_state(), full.save_state());
}

#[test]
fn every_frame_can_be_a_savestate() {
    let p = program();
    let end = 45;
    let mut full = world_with(p, assets(), &[SPAWNER]);
    let reference = run(&mut full, 0, end);
    let mut g = world_with(p, assets(), &[SPAWNER]);
    let mut trace = TraceWriter::new(Vec::new(), Filter::default());
    // Save and reload before every frame: the trace must not notice.
    for f in 0..end {
        g = reload(&g, &g.save_state()).unwrap();
        g.step(&input(f)).unwrap();
        trace.write_frame(&g.trace_frame()).unwrap();
    }
    assert_same(&trace.into_inner(), &reference, end);
}

#[test]
fn savestates_keep_the_host_and_the_pending_room_change() {
    let p = program();
    let mut g = world_with(p, assets(), &[SPAWNER]);
    g.host_mut()
        .files
        .insert("stats.txt".into(), GmString::from("1\r\n2"));
    g.host_mut().highscores[0] = (GmString::from("me"), 9e6);
    g.step(&[InputEvent::Press(Button::Key(b'Z'))]).unwrap();
    g.request_scene_change(SceneChange::Room(3));
    // Drawing settings persist across steps.
    let font = g.font_add_sprite(0, 32, false, 1).unwrap();
    g.draw_set_font(font);
    g.draw_set_color(0x12_34_56);
    g.draw_set_alpha(0.25);
    g.window_set_cursor(-1);
    let loaded = reload(&g, &g.save_state()).unwrap();
    assert_eq!(
        (loaded.draw_color(), loaded.draw_alpha(), loaded.draw_font()),
        (g.draw_color(), g.draw_alpha(), g.draw_font())
    );
    assert_eq!(loaded.font(font), g.font(font));
    assert_eq!(loaded.window_cursor(), -1);
    assert_eq!(loaded.host(), g.host());
    assert_eq!(loaded.pending(), Some(SceneChange::Room(3)));
    assert_eq!(loaded.input(), g.input());
    assert_eq!(loaded.room(), g.room());
    assert_eq!(loaded.trace_frame(), g.trace_frame());
}

/// Corrupting any byte either fails to load or loads a world that runs without panicking: whatever
/// decodes is validated before anything indexes with it.
#[test]
fn corrupt_savestates_never_panic() {
    let p = program();
    let mut g = world_with(p, assets(), &[SPAWNER]);
    run(&mut g, 0, 30);
    g.request_scene_change(SceneChange::Room(0));
    let bytes = g.save_state();
    let mut rejected = 0;
    for i in 20..bytes.len() {
        for flip in [0x01, 0x80, 0xFF] {
            let mut bad = bytes.clone();
            bad[i] ^= flip;
            match reload(&g, &bad) {
                Ok(mut w) => {
                    let _ = w.step(&input(30));
                    let _ = w.trace_frame();
                    let _ = w.save_state();
                }
                Err(_) => rejected += 1,
            }
        }
    }
    assert!(rejected > 0);
}

#[test]
fn bad_savestates_are_rejected() {
    let p = program();
    let g = world_with(p, assets(), &[SPAWNER]);
    let bytes = g.save_state();
    let load = |b: &[u8]| reload(&g, b).err().unwrap();
    assert!(matches!(load(b"nope"), SavestateError::BadMagic));
    let mut wrong = bytes.clone();
    wrong[8] ^= 1;
    assert!(matches!(load(&wrong), SavestateError::WrongVersion(_)));
    assert!(matches!(
        load(&bytes[..bytes.len() - 1]),
        SavestateError::Decode(_)
    ));
    let mut longer = bytes.clone();
    longer.push(0);
    assert!(matches!(load(&longer), SavestateError::Decode(_)));
    // Another program (one more global) or other assets.
    let other = build(|b| {
        b.global("extra");
    });
    let e = World::load_state(other, Arc::new(g.assets().clone()), FONT_SLOTS, &bytes).err();
    assert!(matches!(e, Some(SavestateError::WrongProgram)));
    let e = World::load_state(p, Arc::new(Assets::default()), FONT_SLOTS, &bytes).err();
    assert!(matches!(e, Some(SavestateError::WrongProgram)));
}
