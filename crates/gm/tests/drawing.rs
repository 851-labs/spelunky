//! The drawing, font, window and dialog built-ins (851-3175), with the game's real font sprites.

mod common;
#[path = "common/spelunky.rs"]
mod spelunky;

use common::*;
use draw::{Color, Command};
use gm::*;

const ORANGE: i32 = 0x0080FF;

/// One object `a` with sprite 0 whose draw event is `f`, with the game's font sprites loaded.
fn drawing_world(f: EventFn) -> World {
    let (g, started) = try_drawing_world(f);
    started.unwrap();
    g
}

/// [`drawing_world`], returning how the game start (whose frame-0 draw runs `f`) went.
fn try_drawing_world(f: EventFn) -> (World, Result<(), GmlError>) {
    let prog = build(|b| {
        let a = b.push_object(ObjectInfo {
            sprite: 0,
            ..ObjectInfo::new("a")
        });
        b.event(a, Event::Draw, f);
    });
    let mut sprites = spelunky::sprites(&["sFont", "sFontSmall"]);
    sprites[0] = Some(sprite("s", 4, 8));
    let r = room(320, 240, &numbered(&[(ObjectId(0), 10, 10)]));
    let assets = Assets::new(sprites, vec![Some(r)]);
    let mut g = World::new(prog, std::sync::Arc::new(assets), 1, LAST_ID, FONT_SLOTS);
    let started = g.start_game();
    (g, started)
}

/// The commands a frame recorded, without the projection changes every draw stage makes and the present
/// that ends every step.
fn drawn(frame: &draw::Frame) -> Vec<Command> {
    frame
        .commands
        .iter()
        .filter(|c| !matches!(c, Command::SetView { .. } | Command::Present))
        .cloned()
        .collect()
}

/// `font_add_sprite(sprite, ord(' '), false, 0)`, as `scrInit` makes the game's fonts.
fn add_font(g: &mut World, name: &str) -> i32 {
    g.font_add_sprite(spelunky::sprite_id(name), i32::from(b' '), false, 0)
        .unwrap()
}

fn no_draw(_: &mut World, _: &mut Ctx) -> R<()> {
    Ok(())
}

#[test]
fn fonts_get_the_ids_after_the_game_files_and_map_glyphs_from_space() {
    let mut g = drawing_world(no_draw);
    // scrInit makes global.myFont, then global.myFontSmall. The game file has one (deleted) font slot, so
    // GM8 numbers them 1 and 2; the oracle's globals agree.
    assert_eq!(spelunky::font_slots(), FONT_SLOTS);
    assert_eq!(add_font(&mut g, "sFont"), 1);
    assert_eq!(add_font(&mut g, "sFontSmall"), 2);
    assert_eq!(g.font(0), None);
    let font = g.font(1).unwrap();
    assert_eq!(font.sprite, spelunky::sprite_id("sFont") as u32);
    // One frame per character from ' '.
    let s = g.assets().sprite(font.sprite as i32).unwrap();
    let (n, w, h) = (s.frames.len() as u32, s.width, s.height);
    assert!(n > u32::from(b'Z' - b' '), "sFont covers ' ' to 'Z'");
    let glyphs: Vec<_> = font
        .layout(b"A1#$", n, w, h)
        .into_iter()
        .map(|g| (g.frame, g.dx, g.dy))
        .collect();
    let at = |c: u8| u32::from(c - b' ');
    assert_eq!(
        glyphs,
        [
            (at(b'A'), 0, 0),
            (at(b'1'), w as i32, 0),
            (at(b'$'), 0, h as i32)
        ]
    );
    // Missing sprites are an error; `first` is clamped to a byte.
    assert_eq!(
        g.font_add_sprite(9999, 32, false, 0),
        Err(Error::NoSuchSprite(9999))
    );
    let id = g.font_add_sprite(0, 300, false, -1).unwrap();
    assert_eq!((id, g.font(id).unwrap().first), (3, 255));
}

#[test]
fn draw_settings_persist_across_steps_and_clone_with_the_world() {
    let settings = |g: &World| {
        (
            g.draw_color(),
            g.draw_alpha(),
            g.draw_font(),
            g.window_cursor(),
        )
    };
    let mut g = drawing_world(no_draw);
    // GM8's start: black, opaque, the built-in font, the default cursor.
    assert_eq!(settings(&g), (Color::BLACK, 1.0, -1, 0));
    let font = add_font(&mut g, "sFont");
    g.draw_set_color(ORANGE);
    g.draw_set_alpha(0.5);
    g.draw_set_font(font);
    g.window_set_cursor(-1);
    step(&mut g);
    step(&mut g);
    let saved = g.clone();
    assert_eq!(settings(&g), (Color(ORANGE as u32), 0.5, font, -1));
    g.draw_set_color(0);
    let small = add_font(&mut g, "sFontSmall");
    g.draw_set_font(small);
    assert_eq!(settings(&saved), (Color(ORANGE as u32), 0.5, font, -1));
    assert_eq!(
        saved.font(small),
        None,
        "fonts made after the clone aren't in it"
    );
    // Only the colour's low 24 bits count; an id that isn't a font selects the built-in one.
    g.draw_set_color(-1);
    g.draw_set_font(7);
    assert_eq!((g.draw_color(), g.draw_font()), (Color::WHITE, -1));
}

#[test]
fn each_draw_records_one_command_with_the_settings_baked_in() {
    fn draw(g: &mut World, cx: &mut Ctx) -> R<()> {
        g.draw_set_color(ORANGE);
        g.draw_set_alpha(0.5);
        g.draw_set_font(1);
        g.draw_text(1.0, 2.0, &Value::from(2.5));
        g.draw_sprite(cx, 0, 2, 3.0, 4.0)?;
        // A negative subimage is the instance's own frame.
        g.draw_sprite(cx, 0, -1, 3.0, 4.0)?;
        g.draw_sprite_ext(cx, 0, 7, 5.0, 6.0, 2.0, -1.0, 90.0, 0xFF, 0.25)?;
        g.draw_rectangle(0.0, 0.0, 10.0, 10.0, true);
        g.draw_circle(8.0, 9.0, 4.0, false);
        Ok(())
    }
    let mut g = drawing_world(draw);
    let font = add_font(&mut g, "sFont");
    let h = g.find(FIRST_ID).unwrap();
    g.instance_mut(h).image_index = 1.75;
    let mut frame = draw::Frame::default();
    g.step_drawing(&[], &mut frame).unwrap();
    let (color, alpha) = (Color(ORANGE as u32), 0.5);
    let ext = draw::Sprite {
        xscale: 2.0,
        yscale: -1.0,
        angle: 90.0,
        blend: Color(0xFF),
        alpha: 0.25,
        ..draw::Sprite::new(0, 7, 5.0, 6.0)
    };
    let want = vec![
        Command::Text(draw::Text {
            font: g.font(font).unwrap(),
            x: 1.0,
            y: 2.0,
            // As string() shows it.
            text: b"2.50".to_vec(),
            color,
            alpha,
        }),
        // draw_sprite ignores the drawing colour and alpha.
        Command::Sprite(draw::Sprite::new(0, 2, 3.0, 4.0)),
        Command::Sprite(draw::Sprite::new(0, 1, 3.0, 4.0)),
        Command::Sprite(ext),
        Command::Rectangle {
            x1: 0.0,
            y1: 0.0,
            x2: 10.0,
            y2: 10.0,
            color,
            alpha,
            outline: true,
        },
        Command::Circle {
            x: 8.0,
            y: 9.0,
            radius: 4.0,
            color,
            alpha,
            outline: false,
        },
    ];
    assert_eq!(drawn(&frame), want);
}

#[test]
fn text_in_the_built_in_font_records_nothing() {
    fn draw(g: &mut World, _: &mut Ctx) -> R<()> {
        g.draw_text(0.0, 0.0, &Value::from("HI"));
        Ok(())
    }
    let mut g = drawing_world(draw);
    let mut frame = draw::Frame::default();
    g.step_drawing(&[], &mut frame).unwrap();
    assert_eq!(drawn(&frame), []);
}

#[test]
fn headless_draws_record_nothing_but_still_check_sprites() {
    fn draw(g: &mut World, cx: &mut Ctx) -> R<()> {
        g.draw_rectangle(0.0, 0.0, 1.0, 1.0, false);
        g.draw_sprite(cx, 0, 0, 0.0, 0.0)?;
        g.draw_sprite(cx, 42, 0, 0.0, 0.0)
    }
    // Frame 0's draw, which records nothing, already fails.
    let (mut g, started) = try_drawing_world(draw);
    assert_eq!(started.unwrap_err().error, Error::NoSuchSprite(42));
    let err = g.step(&[]).unwrap_err();
    assert_eq!(err.error, Error::NoSuchSprite(42));
    // The same draws record into an attached frame up to the error (after the projection).
    let mut frame = draw::Frame::default();
    assert!(g.step_drawing(&[], &mut frame).is_err());
    assert_eq!(drawn(&frame).len(), 2);
}

#[test]
fn booting_records_the_frame_0_draw_of_the_room_it_ends_in() {
    // Like rScreenInit leading to rIntro: the first room's load asks for the second, whose frame-0 draw
    // happens while booting.
    fn go(g: &mut World, _: &mut Ctx) -> R<()> {
        g.room_goto_next()
    }
    fn draw(g: &mut World, _: &mut Ctx) -> R<()> {
        g.draw_rectangle(0.0, 0.0, 1.0, 1.0, false);
        Ok(())
    }
    let prog = build(|b| {
        let a = b.push_object(ObjectInfo::new("a"));
        let z = b.push_object(ObjectInfo::new("z"));
        b.event(a, Event::Create, go).event(z, Event::Draw, draw);
    });
    let first = room(320, 240, &numbered(&[(ObjectId(0), 0, 0)]));
    let second = room(320, 240, &[(FIRST_ID + 1, ObjectId(1), 0, 0)]);
    let assets = Assets::new(Vec::new(), vec![Some(first), Some(second)]);
    let mut g = World::new(prog, std::sync::Arc::new(assets), 1, LAST_ID, FONT_SLOTS);
    let mut frame = draw::Frame::default();
    g.start_game_drawing(&mut frame).unwrap();
    assert_eq!(g.room().id, 1);
    // The window opening at the first room's size, then the second room's draw.
    assert!(matches!(
        drawn(&frame)[..],
        [Command::ResizeScreen { .. }, Command::Rectangle { .. }]
    ));
}

#[test]
fn dialogs_answer_as_if_dismissed_at_once() {
    let g = drawing_world(no_draw);
    g.show_message(b"Frequency: 0");
    g.show_info();
    assert_eq!(g.show_menu(b"Set Frequency|Play", 3), 3);
    assert_eq!(
        g.get_integer(b"Frequency from 1000 to 100 000", 44100.0),
        44100.0
    );
    // show_error stops the game, as in the oracle.
    assert_eq!(
        g.show_error(b"Initialization of the sound system failed!", false),
        Err(Error::ShowError(
            "Initialization of the sound system failed!".into()
        ))
    );
}
