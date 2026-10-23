//! Input at tick boundaries, and a recorded session played back from its replay text.

mod common;

use common::*;
use gm::input::KeyState;
use gm::*;
use trace::replay::{Axis, Button, JoystickInfo, MouseButton, Replay, ReplayHeader};

const Z: u8 = b'Z';

/// Logs what a step sees of Z, the mouse and joystick 1.
fn watch(g: &mut World, _: &mut Ctx) -> R<()> {
    let i = g.input();
    let z = [KeyState::Held, KeyState::Pressed, KeyState::Released]
        .map(|s| if i.check(s, Z) { "1" } else { "0" })
        .concat();
    let s = format!(
        "z{z} k{} m{:?}{} j{}{}",
        i.keyboard_key(),
        i.mouse(),
        u8::from(i.mouse_check_gm(KeyState::Held, -1)),
        u8::from(i.joystick_check_button(1, 1)),
        i.joystick_direction(1),
    );
    log(g, &s);
    Ok(())
}

fn program() -> &'static Program {
    build(|b| {
        let o = b.push_object(ObjectInfo::new("o"));
        b.event(o, Event::Step, watch)
            .event(o, Event::KeyPress(Z), note);
    })
}

#[test]
fn a_tap_within_one_tick_is_pressed_and_released_in_that_tick_only() {
    let mut g = world(program(), &[ObjectId(0)]);
    let z = Button::Key(Z);
    g.step(&[InputEvent::Press(z), InputEvent::Release(z)])
        .unwrap();
    assert_eq!(take_log(&mut g), "o1.press90 z011 k0 m(0, 0)0 j0101");
    step(&mut g);
    assert_eq!(take_log(&mut g), "z000 k0 m(0, 0)0 j0101");
    // Held over a tick boundary: pressed in the first tick, held in the next, released in the third.
    g.step(&[InputEvent::Press(z)]).unwrap();
    step(&mut g);
    g.step(&[InputEvent::Release(z)]).unwrap();
    assert_eq!(
        take_log(&mut g),
        "o1.press90 z110 k90 m(0, 0)0 j0101 z100 k90 m(0, 0)0 j0101 z001 k0 m(0, 0)0 j0101"
    );
}

/// A hand-played session, tick by tick, as `app::input::Collector` hands it over: a tap inside one tick,
/// auto-repeat, the mouse, and a gamepad.
fn session() -> Vec<Vec<InputEvent>> {
    let z = Button::Key(Z);
    vec![
        vec![InputEvent::Press(z), InputEvent::Release(z)],
        vec![],
        vec![
            InputEvent::Press(z),
            InputEvent::MouseMove { x: 17, y: 230 },
        ],
        vec![
            InputEvent::Press(z),
            InputEvent::Press(Button::Mouse(MouseButton::Right)),
        ],
        vec![
            InputEvent::Release(z),
            InputEvent::Press(Button::Joy(1)),
            InputEvent::Axis(Axis::X, 0.75),
            InputEvent::Axis(Axis::Y, -0.62),
        ],
        vec![
            InputEvent::Release(Button::Mouse(MouseButton::Right)),
            InputEvent::WheelUp,
        ],
        vec![
            InputEvent::Release(Button::Joy(1)),
            InputEvent::Axis(Axis::X, 0.0),
        ],
        vec![],
    ]
}

fn run<'a>(joystick: &JoystickInfo, ticks: impl Iterator<Item = &'a [InputEvent]>) -> String {
    let mut g = world(program(), &[ObjectId(0)]);
    g.connect_joystick(Some(joystick));
    take_log(&mut g);
    for events in ticks {
        g.step(events).unwrap();
    }
    take_log(&mut g)
}

#[test]
fn a_recorded_session_plays_back_identically() {
    let header = ReplayHeader {
        joystick: Some(JoystickInfo { has_pov: true }),
        ..ReplayHeader::new(5)
    };
    let joystick = header.joystick.clone().unwrap();
    let session = session();
    // Recording, as the app does: every tick's events, then the length in ticks.
    let mut recording = Replay::new(header);
    for (frame, events) in session.iter().enumerate() {
        for &e in events {
            recording.push(frame as u64, e).unwrap();
        }
    }
    recording.set_len(session.len() as u64).unwrap();
    let text = recording.to_text();
    let replay = Replay::parse(&text).unwrap();
    assert_eq!(replay, recording, "{text}");

    let live = run(&joystick, session.iter().map(Vec::as_slice));
    let played: Vec<Vec<InputEvent>> = replay
        .inputs()
        .map(|f| f.events.iter().map(|&(_, e)| e).collect())
        .collect();
    assert_eq!(played.len(), session.len());
    let playback = run(
        replay.header.joystick.as_ref().unwrap(),
        played.iter().map(Vec::as_slice),
    );
    assert_eq!(playback, live);
    // The session did reach the game: the tap, the repeat, the mouse and the stick.
    assert!(live.starts_with("o1.press90 z011"), "{live}");
    assert!(live.contains("o1.press90 z110 k90 m(17, 230)1"), "{live}");
    assert!(live.contains("j1105"), "{live}");
}
