//! The GM8 built-in functions the game calls, as generated code calls them: `b::name(g, cx, [args])`.
//!
//! Each takes its arguments as a fixed-size array, so a call with the wrong argument count doesn't
//! compile (GM8 rejects it at load time too), and converts them the way GM8 converts built-in arguments
//! (`Value::to_real`, `to_int`, `to_str`, `is_truthy`) before calling the typed `gm` function. Functions
//! GM8 treats as returning nothing return 0.
//!
//! Built-ins `gm` doesn't provide yet are **shims** that fail with [`Error::Unsupported`] naming the
//! function and the ticket that owns it, so the whole game compiles and a missing piece stops the tick
//! with a located error (`unit:line`) instead of silently doing nothing. Built-ins only unreachable code
//! calls are **unreachable in 1.1**: they fail the same way, naming no ticket, and a test checks that only
//! that code calls them.
//!
//! # Coverage
//!
//! 114 functions: the 112 of `docs/source-inventory.md` section 5.1, plus the drag-and-drop actions
//! `action_color` and `action_fullscreen`. 111 are implemented and 3 unreachable in 1.1; none is shimmed.
//!
//! | Group | Functions | Status |
//! |---|---|---|
//! | Instances | `instance_create`, `instance_destroy`, `instance_exists`, `instance_number`, `instance_activate_object`, `instance_deactivate_object`, `instance_activate_region`, `instance_deactivate_region`, `instance_activate_all`, `instance_deactivate_all`, `object_get_parent` | implemented (`gm::kernel`) |
//! | Math and random | `abs`, `round`, `floor`, `ceil`, `frac`, `sqr`, `sqrt`, `sin`, `cos`, `arctan`, `degtorad`, `radtodeg`, `point_distance`, `point_direction`, `make_color_rgb`, `random` | implemented (`gm::math`, `gm::Rng`) |
//! | Strings | `string`, `real`, `ord`, `string_length`, `string_char_at`, `string_delete`, `string_insert`, `string_upper`, `string_lower`, `is_real`, `is_string` | implemented (`gm::string`, `gm::Value`) |
//! | Keyboard | `keyboard_check`, `keyboard_check_pressed`, `keyboard_check_released` | implemented (`gm::input`) |
//! | Rooms, views, tiles | `room_goto`, `room_goto_next`, `room_restart`, `game_end`, `room_get_name`, `room_set_view`, `tile_add`, `tile_delete`, `tile_layer_find` | implemented (`gm` `room.rs`) |
//! | Collision and movement | `collision_point`, `collision_rectangle`, `collision_line`, `place_meeting`, `instance_place`, `instance_position`, `distance_to_object`, `distance_to_point`, `instance_nearest`, `move_snap` | implemented (`gm::collision`) |
//! | Surfaces, blending, screen | `surface_create`, `surface_exists`, `surface_free`, `surface_set_target`, `surface_reset_target`, `draw_surface`, `draw_surface_stretched`, `draw_set_blend_mode`, `draw_set_blend_mode_ext`, `draw_clear`, `screen_redraw`, `screen_refresh`, `action_fullscreen` | implemented (`gm` `surfaces.rs`) |
//! | Drawing, fonts, window, dialogs | `draw_text`, `draw_set_color`, `draw_sprite`, `draw_set_font`, `draw_sprite_ext`, `draw_set_alpha`, `draw_circle`, `draw_rectangle`, `font_add_sprite`, `action_color`, `window_set_cursor`, `show_message`, `show_error`, `show_info`, `show_menu`, `get_integer` | implemented (`World::draw_text` and friends; dialogs answer as if dismissed) |
//! | Files and highscores | `file_exists`, `file_delete`, `file_find_first`, `file_find_next`, `file_text_open_read`, `file_text_open_write`, `file_text_close`, `file_text_eof`, `file_text_read_string`, `file_text_readln`, `file_text_write_string`, `file_text_writeln`, `highscore_add`, `highscore_value`, `highscore_clear` | implemented (`gm::Host`, over its virtual game directory) |
//! | Sound and DLLs | `sound_play` (only in `oTemp`, which nothing creates), `external_define`, `external_call` (only in the `SS_*` scripts, which are overridden) | unreachable in 1.1 |
//! | Joystick and mouse | `joystick_exists`, `joystick_check_button`, `joystick_direction`, `joystick_has_pov`, `joystick_pov`, `joystick_zpos`, `mouse_check_button` | implemented (`gm::input`; without a joystick, OpenGMK's stub answers) |

use crate::generated::names::PARENT_INDEX;
use gm::input::KeyState;
use gm::{Ctx, Error, FileError, GmString, ObjectId, R, Value, World, math, string};
use trace::replay::Axis;

/// What a built-in that returns nothing gives back.
#[inline]
fn none() -> R<Value> {
    Ok(Value::Real(0.0))
}

/// A string argument as a `GmString` (reals read as the empty string, like [`Value::to_str`]).
fn gm_str(v: &Value) -> GmString {
    match v {
        Value::Str(s) => s.clone(),
        Value::Real(_) => GmString::default(),
    }
}

// ---- Instances (gm::kernel) ----

pub fn instance_create(g: &mut World, _: &Ctx, [x, y, obj]: [Value; 3]) -> R<Value> {
    Ok(g.instance_create(x.to_real(), y.to_real(), obj.to_int()?)?
        .into())
}

/// Also the `action_kill_object` drag-and-drop action.
pub fn instance_destroy(g: &mut World, cx: &Ctx, []: [Value; 0]) -> R<Value> {
    g.instance_destroy(cx)?;
    none()
}

pub fn instance_exists(g: &mut World, cx: &Ctx, [obj]: [Value; 1]) -> R<Value> {
    Ok(g.instance_exists(cx, obj.to_int()?).into())
}

pub fn instance_number(g: &mut World, cx: &Ctx, [obj]: [Value; 1]) -> R<Value> {
    Ok(g.instance_number(cx, obj.to_int()?).into())
}

pub fn instance_activate_object(g: &mut World, cx: &Ctx, [obj]: [Value; 1]) -> R<Value> {
    g.instance_activate_object(cx, obj.to_int()?);
    none()
}

pub fn instance_deactivate_object(g: &mut World, cx: &Ctx, [obj]: [Value; 1]) -> R<Value> {
    g.instance_deactivate_object(cx, obj.to_int()?);
    none()
}

pub fn instance_activate_region(
    g: &mut World,
    _: &Ctx,
    [left, top, width, height, inside]: [Value; 5],
) -> R<Value> {
    g.instance_activate_region(
        left.to_real(),
        top.to_real(),
        width.to_real(),
        height.to_real(),
        inside.is_truthy(),
    );
    none()
}

pub fn instance_deactivate_region(
    g: &mut World,
    cx: &Ctx,
    [left, top, width, height, inside, notme]: [Value; 6],
) -> R<Value> {
    g.instance_deactivate_region(
        cx,
        left.to_real(),
        top.to_real(),
        width.to_real(),
        height.to_real(),
        inside.is_truthy(),
        notme.is_truthy(),
    );
    none()
}

pub fn instance_activate_all(g: &mut World, _: &Ctx, []: [Value; 0]) -> R<Value> {
    g.instance_activate_all();
    none()
}

pub fn instance_deactivate_all(g: &mut World, cx: &Ctx, [notme]: [Value; 1]) -> R<Value> {
    g.instance_deactivate_all(cx, notme.is_truthy());
    none()
}

/// `object_get_parent(obj)`: the parent index as the game data stores it, which for "no parent" is -1 or
/// -100 depending on the object (the GM 8.0 manual only mentions -1; GM8 returns the stored value, as
/// OpenGMK does: verify against the oracle). The game only compares the result with object ids. A
/// missing object gives -1 here (unverified; the game never asks).
pub fn object_get_parent(g: &mut World, _: &Ctx, [obj]: [Value; 1]) -> R<Value> {
    let parent = match g.program().object_by_index(obj.to_int()?) {
        Some((ObjectId(o), _)) => PARENT_INDEX[o as usize],
        None => -1,
    };
    Ok(parent.into())
}

// ---- Math and random (gm::math, gm::Rng) ----

macro_rules! real_fns {
    ($($name:ident),*) => {$(
        pub fn $name(_: &mut World, _: &Ctx, [x]: [Value; 1]) -> R<Value> {
            Ok(math::$name(x.to_real()).into())
        }
    )*};
}

real_fns!(abs, frac, sin, cos, arctan, degtorad);

/// The math built-ins that can fail: `sqrt` of a negative, `round`, `floor` and `ceil` past the `Int64`
/// range, and `sqr` and `radtodeg` past the largest double (and `point_distance`, below).
macro_rules! checked_real_fns {
    ($($name:ident),*) => {$(
        pub fn $name(_: &mut World, _: &Ctx, [x]: [Value; 1]) -> R<Value> {
            Ok(math::$name(x.to_real())?.into())
        }
    )*};
}

checked_real_fns!(round, floor, ceil, sqrt, sqr, radtodeg);

pub fn point_distance(_: &mut World, _: &Ctx, [x1, y1, x2, y2]: [Value; 4]) -> R<Value> {
    Ok(math::point_distance(x1.to_real(), y1.to_real(), x2.to_real(), y2.to_real())?.into())
}

pub fn point_direction(_: &mut World, _: &Ctx, [x1, y1, x2, y2]: [Value; 4]) -> R<Value> {
    Ok(math::point_direction(x1.to_real(), y1.to_real(), x2.to_real(), y2.to_real()).into())
}

/// `make_color_rgb(r, g, b)`: its components are integer arguments that aren't cut to 32 bits, so they're
/// read as reals (a string as 0, as [`Value::to_int`] reads it).
pub fn make_color_rgb(_: &mut World, _: &Ctx, [r, gr, b]: [Value; 3]) -> R<Value> {
    Ok(math::make_color_rgb(r.to_real(), gr.to_real(), b.to_real())?.into())
}

/// `random(x)`: one step of the world's generator.
pub fn random(g: &mut World, _: &Ctx, [x]: [Value; 1]) -> R<Value> {
    Ok(g.rng().random(x.to_real()).into())
}

// ---- Strings (gm::string, gm::Value) ----

pub fn string(_: &mut World, _: &Ctx, [x]: [Value; 1]) -> R<Value> {
    Ok(x.to_gml_string().into())
}

pub fn real(_: &mut World, _: &Ctx, [x]: [Value; 1]) -> R<Value> {
    Ok(x.to_gml_real()?.into())
}

pub fn ord(_: &mut World, _: &Ctx, [s]: [Value; 1]) -> R<Value> {
    Ok(string::ord(s.to_str()).into())
}

pub fn string_length(_: &mut World, _: &Ctx, [s]: [Value; 1]) -> R<Value> {
    Ok(string::string_length(s.to_str()).into())
}

pub fn string_char_at(_: &mut World, _: &Ctx, [s, pos]: [Value; 2]) -> R<Value> {
    Ok(string::string_char_at(s.to_str(), pos.to_int()?).into())
}

pub fn string_delete(_: &mut World, _: &Ctx, [s, pos, count]: [Value; 3]) -> R<Value> {
    Ok(string::string_delete(&gm_str(&s), pos.to_int()?, count.to_int()?).into())
}

pub fn string_insert(_: &mut World, _: &Ctx, [substr, s, pos]: [Value; 3]) -> R<Value> {
    Ok(string::string_insert(substr.to_str(), s.to_str(), pos.to_int()?).into())
}

pub fn string_upper(_: &mut World, _: &Ctx, [s]: [Value; 1]) -> R<Value> {
    Ok(string::string_upper(s.to_str()).into())
}

pub fn string_lower(_: &mut World, _: &Ctx, [s]: [Value; 1]) -> R<Value> {
    Ok(string::string_lower(s.to_str()).into())
}

pub fn is_real(_: &mut World, _: &Ctx, [x]: [Value; 1]) -> R<Value> {
    Ok(x.is_real().into())
}

pub fn is_string(_: &mut World, _: &Ctx, [x]: [Value; 1]) -> R<Value> {
    Ok(x.is_string().into())
}

// ---- Keyboard (gm::input) ----

/// A keyboard check: key codes outside 0-255 are never down.
fn key(g: &World, state: KeyState, vk: &Value) -> R<Value> {
    let down = u8::try_from(vk.to_int()?).is_ok_and(|vk| g.input().check(state, vk));
    Ok(down.into())
}

pub fn keyboard_check(g: &mut World, _: &Ctx, [vk]: [Value; 1]) -> R<Value> {
    key(g, KeyState::Held, &vk)
}

pub fn keyboard_check_pressed(g: &mut World, _: &Ctx, [vk]: [Value; 1]) -> R<Value> {
    key(g, KeyState::Pressed, &vk)
}

pub fn keyboard_check_released(g: &mut World, _: &Ctx, [vk]: [Value; 1]) -> R<Value> {
    key(g, KeyState::Released, &vk)
}

// ---- Joystick and mouse (gm::input) ----
//
// Without the replay's `joystick` directive no joystick exists, and these answer what the oracle's
// (OpenGMK's) stubs answer, so keyboard replays run the same on both.

pub fn joystick_exists(g: &mut World, _: &Ctx, [id]: [Value; 1]) -> R<Value> {
    Ok(g.input().joystick_exists(id.to_int()?).into())
}

pub fn joystick_check_button(g: &mut World, _: &Ctx, [id, b]: [Value; 2]) -> R<Value> {
    Ok(g.input()
        .joystick_check_button(id.to_int()?, b.to_int()?)
        .into())
}

pub fn joystick_direction(g: &mut World, _: &Ctx, [id]: [Value; 1]) -> R<Value> {
    Ok(g.input().joystick_direction(id.to_int()?).into())
}

pub fn joystick_has_pov(g: &mut World, _: &Ctx, [id]: [Value; 1]) -> R<Value> {
    Ok(g.input().joystick_has_pov(id.to_int()?).into())
}

pub fn joystick_pov(g: &mut World, _: &Ctx, [id]: [Value; 1]) -> R<Value> {
    Ok(g.input().joystick_axis(id.to_int()?, Axis::Pov).into())
}

pub fn joystick_zpos(g: &mut World, _: &Ctx, [id]: [Value; 1]) -> R<Value> {
    Ok(g.input().joystick_axis(id.to_int()?, Axis::Z).into())
}

pub fn mouse_check_button(g: &mut World, _: &Ctx, [b]: [Value; 1]) -> R<Value> {
    Ok(g.input().mouse_check_gm(KeyState::Held, b.to_int()?).into())
}

// ---- Rooms, views and tiles (gm `room.rs`) ----

pub fn room_goto(g: &mut World, _: &Ctx, [room]: [Value; 1]) -> R<Value> {
    g.room_goto(room.to_int()?);
    none()
}

pub fn room_goto_next(g: &mut World, _: &Ctx, []: [Value; 0]) -> R<Value> {
    g.room_goto_next()?;
    none()
}

pub fn room_restart(g: &mut World, _: &Ctx, []: [Value; 0]) -> R<Value> {
    g.room_restart();
    none()
}

pub fn game_end(g: &mut World, _: &Ctx, []: [Value; 0]) -> R<Value> {
    g.game_end();
    none()
}

pub fn room_set_view(
    g: &mut World,
    _: &Ctx,
    [
        room,
        index,
        visible,
        xv,
        yv,
        wv,
        hv,
        xp,
        yp,
        wp,
        hp,
        hb,
        vb,
        hs,
        vs,
        obj,
    ]: [Value; 16],
) -> R<Value> {
    let view = gm::View {
        visible: visible.is_truthy(),
        xview: xv.to_int()?,
        yview: yv.to_int()?,
        wview: wv.to_int()?,
        hview: hv.to_int()?,
        xport: xp.to_int()?,
        yport: yp.to_int()?,
        wport: wp.to_int()?,
        hport: hp.to_int()?,
        angle: 0.0,
        hborder: hb.to_int()?,
        vborder: vb.to_int()?,
        hspeed: hs.to_int()?,
        vspeed: vs.to_int()?,
        object: obj.to_int()?,
    };
    g.room_set_view(room.to_int()?, index.to_int()?, view);
    none()
}

pub fn tile_add(
    g: &mut World,
    _: &Ctx,
    [bg, left, top, w, h, x, y, depth]: [Value; 8],
) -> R<Value> {
    let id = g.tile_add(
        bg.to_int()?,
        left.to_int()?,
        top.to_int()?,
        w.to_int()?,
        h.to_int()?,
        x.to_real(),
        y.to_real(),
        depth.to_real(),
    )?;
    Ok(id.into())
}

pub fn tile_delete(g: &mut World, _: &Ctx, [id]: [Value; 1]) -> R<Value> {
    g.tile_delete(id.to_int()?)?;
    none()
}

pub fn tile_layer_find(g: &mut World, _: &Ctx, [depth, x, y]: [Value; 3]) -> R<Value> {
    Ok(g.tile_layer_find(depth.to_real(), x.to_real(), y.to_real())
        .into())
}

/// `room_get_name(room)`: `"<undefined>"` for a room that doesn't exist, as OpenGMK's `*_get_name`
/// functions answer (verify against the oracle; the game passes `room`).
pub fn room_get_name(g: &mut World, _: &Ctx, [room]: [Value; 1]) -> R<Value> {
    Ok(g.assets()
        .room_name(room.to_int()?)
        .unwrap_or("<undefined>")
        .into())
}

// ---- Collision and movement (gm::collision) ----

pub fn collision_point(g: &mut World, cx: &Ctx, [x, y, obj, prec, notme]: [Value; 5]) -> R<Value> {
    let id = g.collision_point(
        cx,
        x.to_real(),
        y.to_real(),
        obj.to_int()?,
        prec.is_truthy(),
        notme.is_truthy(),
    )?;
    Ok(id.into())
}

/// The corners are rounded to integers, as GM8's argument conversion does.
pub fn collision_rectangle(
    g: &mut World,
    cx: &Ctx,
    [x1, y1, x2, y2, obj, prec, notme]: [Value; 7],
) -> R<Value> {
    let id = g.collision_rectangle(
        cx,
        x1.to_int()?,
        y1.to_int()?,
        x2.to_int()?,
        y2.to_int()?,
        obj.to_int()?,
        prec.is_truthy(),
        notme.is_truthy(),
    )?;
    Ok(id.into())
}

pub fn collision_line(
    g: &mut World,
    cx: &Ctx,
    [x1, y1, x2, y2, obj, prec, notme]: [Value; 7],
) -> R<Value> {
    let id = g.collision_line(
        cx,
        x1.to_real(),
        y1.to_real(),
        x2.to_real(),
        y2.to_real(),
        obj.to_int()?,
        prec.is_truthy(),
        notme.is_truthy(),
    )?;
    Ok(id.into())
}

pub fn place_meeting(g: &mut World, cx: &Ctx, [x, y, obj]: [Value; 3]) -> R<Value> {
    Ok(
        g.place_meeting(cx, x.to_real(), y.to_real(), obj.to_int()?)?
            .into(),
    )
}

pub fn instance_place(g: &mut World, cx: &Ctx, [x, y, obj]: [Value; 3]) -> R<Value> {
    Ok(
        g.instance_place(cx, x.to_real(), y.to_real(), obj.to_int()?)?
            .into(),
    )
}

pub fn instance_position(g: &mut World, _: &Ctx, [x, y, obj]: [Value; 3]) -> R<Value> {
    Ok(
        g.instance_position(x.to_real(), y.to_real(), obj.to_int()?)?
            .into(),
    )
}

pub fn distance_to_object(g: &mut World, cx: &Ctx, [obj]: [Value; 1]) -> R<Value> {
    Ok(g.distance_to_object(cx, obj.to_int()?).into())
}

pub fn distance_to_point(g: &mut World, cx: &Ctx, [x, y]: [Value; 2]) -> R<Value> {
    Ok(g.distance_to_point(cx, x.to_real(), y.to_real()).into())
}

pub fn instance_nearest(g: &mut World, _: &Ctx, [x, y, obj]: [Value; 3]) -> R<Value> {
    Ok(g.instance_nearest(x.to_real(), y.to_real(), obj.to_int()?)
        .into())
}

pub fn move_snap(g: &mut World, cx: &Ctx, [hsnap, vsnap]: [Value; 2]) -> R<Value> {
    g.move_snap(cx, hsnap.to_real(), vsnap.to_real())?;
    none()
}

// ---- Surfaces, blending, screen (gm surfaces.rs) ----

pub fn surface_create(g: &mut World, _: &Ctx, [w, h]: [Value; 2]) -> R<Value> {
    Ok(g.surface_create(w.to_int()?, h.to_int()?)?.into())
}

pub fn surface_exists(g: &mut World, _: &Ctx, [id]: [Value; 1]) -> R<Value> {
    Ok(g.surface_exists(id.to_int()?).into())
}

pub fn surface_free(g: &mut World, _: &Ctx, [id]: [Value; 1]) -> R<Value> {
    g.surface_free(id.to_int()?);
    none()
}

pub fn surface_set_target(g: &mut World, _: &Ctx, [id]: [Value; 1]) -> R<Value> {
    g.surface_set_target(id.to_int()?);
    none()
}

pub fn surface_reset_target(g: &mut World, _: &Ctx, []: [Value; 0]) -> R<Value> {
    g.surface_reset_target();
    none()
}

pub fn draw_surface(g: &mut World, _: &Ctx, [id, x, y]: [Value; 3]) -> R<Value> {
    g.draw_surface(id.to_int()?, x.to_real(), y.to_real())?;
    none()
}

pub fn draw_surface_stretched(g: &mut World, _: &Ctx, [id, x, y, w, h]: [Value; 5]) -> R<Value> {
    let (x, y, w, h) = (x.to_real(), y.to_real(), w.to_real(), h.to_real());
    g.draw_surface_stretched(id.to_int()?, x, y, w, h)?;
    none()
}

pub fn draw_set_blend_mode(g: &mut World, _: &Ctx, [mode]: [Value; 1]) -> R<Value> {
    g.draw_set_blend_mode(mode.to_int()?);
    none()
}

pub fn draw_set_blend_mode_ext(g: &mut World, _: &Ctx, [src, dest]: [Value; 2]) -> R<Value> {
    g.draw_set_blend_mode_ext(src.to_int()?, dest.to_int()?);
    none()
}

pub fn draw_clear(g: &mut World, _: &Ctx, [col]: [Value; 1]) -> R<Value> {
    g.draw_clear(col.to_int()?);
    none()
}

/// `screen_redraw()`: the whole draw stage, draw events included (runtime.md section 1).
pub fn screen_redraw(g: &mut World, _: &Ctx, []: [Value; 0]) -> R<Value> {
    g.screen_redraw()?;
    none()
}

pub fn screen_refresh(g: &mut World, _: &Ctx, []: [Value; 0]) -> R<Value> {
    g.screen_refresh();
    none()
}

/// The `action_fullscreen` drag-and-drop action.
pub fn action_fullscreen(g: &mut World, _: &Ctx, [action]: [Value; 1]) -> R<Value> {
    g.action_fullscreen(action.to_int()?);
    none()
}

// ---- Drawing, fonts, window and dialogs (World::draw_text and friends) ----

pub fn draw_set_color(g: &mut World, _: &Ctx, [col]: [Value; 1]) -> R<Value> {
    g.draw_set_color(col.to_int()?);
    none()
}

/// The `action_color` drag-and-drop action is `draw_set_color`.
pub fn action_color(g: &mut World, cx: &Ctx, args: [Value; 1]) -> R<Value> {
    draw_set_color(g, cx, args)
}

pub fn draw_set_alpha(g: &mut World, _: &Ctx, [alpha]: [Value; 1]) -> R<Value> {
    g.draw_set_alpha(alpha.to_real());
    none()
}

pub fn draw_set_font(g: &mut World, _: &Ctx, [font]: [Value; 1]) -> R<Value> {
    g.draw_set_font(font.to_int()?);
    none()
}

pub fn font_add_sprite(g: &mut World, _: &Ctx, [spr, first, prop, sep]: [Value; 4]) -> R<Value> {
    let id = g.font_add_sprite(
        spr.to_int()?,
        first.to_int()?,
        prop.is_truthy(),
        sep.to_int()?,
    )?;
    Ok(id.into())
}

/// `text` is converted as `string()` would.
pub fn draw_text(g: &mut World, _: &Ctx, [x, y, text]: [Value; 3]) -> R<Value> {
    g.draw_text(x.to_real(), y.to_real(), &text)?;
    none()
}

pub fn draw_sprite(g: &mut World, cx: &Ctx, [spr, sub, x, y]: [Value; 4]) -> R<Value> {
    g.draw_sprite(cx, spr.to_int()?, sub.to_int()?, x.to_real(), y.to_real())?;
    none()
}

pub fn draw_sprite_ext(
    g: &mut World,
    cx: &Ctx,
    [spr, sub, x, y, xscale, yscale, angle, col, alpha]: [Value; 9],
) -> R<Value> {
    g.draw_sprite_ext(
        cx,
        spr.to_int()?,
        sub.to_int()?,
        x.to_real(),
        y.to_real(),
        xscale.to_real(),
        yscale.to_real(),
        angle.to_real(),
        col.to_int()?,
        alpha.to_real(),
    )?;
    none()
}

pub fn draw_rectangle(g: &mut World, _: &Ctx, [x1, y1, x2, y2, outline]: [Value; 5]) -> R<Value> {
    g.draw_rectangle(
        x1.to_real(),
        y1.to_real(),
        x2.to_real(),
        y2.to_real(),
        outline.is_truthy(),
    )?;
    none()
}

pub fn draw_circle(g: &mut World, _: &Ctx, [x, y, r, outline]: [Value; 4]) -> R<Value> {
    g.draw_circle(x.to_real(), y.to_real(), r.to_real(), outline.is_truthy())?;
    none()
}

pub fn window_set_cursor(g: &mut World, _: &Ctx, [cursor]: [Value; 1]) -> R<Value> {
    g.window_set_cursor(cursor.to_int()?);
    none()
}

pub fn show_message(g: &mut World, _: &Ctx, [text]: [Value; 1]) -> R<Value> {
    g.show_message(text.to_str());
    none()
}

pub fn show_error(g: &mut World, _: &Ctx, [text, abort]: [Value; 2]) -> R<Value> {
    g.show_error(text.to_str(), abort.is_truthy())?;
    none()
}

pub fn show_info(g: &mut World, _: &Ctx, []: [Value; 0]) -> R<Value> {
    g.show_info();
    none()
}

pub fn show_menu(g: &mut World, _: &Ctx, [items, def]: [Value; 2]) -> R<Value> {
    Ok(g.show_menu(items.to_str(), def.to_int()?).into())
}

pub fn get_integer(g: &mut World, _: &Ctx, [prompt, def]: [Value; 2]) -> R<Value> {
    Ok(g.get_integer(prompt.to_str(), def.to_real()).into())
}

// ---- Files and highscores (gm::Host, over its virtual game directory) ----

/// A file built-in's result, its error naming the function.
fn file<T>(function: &'static str, r: Result<T, FileError>) -> R<T> {
    r.map_err(|e| Error::File(function, e))
}

/// `file_exists(name)`: a real argument is never a file (OpenGMK checks the type, not the string).
pub fn file_exists(g: &mut World, _: &Ctx, [name]: [Value; 1]) -> R<Value> {
    Ok((name.is_string() && g.host().file_exists(name.to_str())).into())
}

pub fn file_delete(g: &mut World, _: &Ctx, [name]: [Value; 1]) -> R<Value> {
    g.host_mut().file_delete(name.to_str());
    none()
}

pub fn file_find_first(g: &mut World, _: &Ctx, [mask, attr]: [Value; 2]) -> R<Value> {
    Ok(g.host_mut()
        .file_find_first(mask.to_str(), attr.to_int()?)
        .into())
}

pub fn file_find_next(g: &mut World, _: &Ctx, []: [Value; 0]) -> R<Value> {
    Ok(g.host_mut().file_find_next().into())
}

pub fn file_text_open_read(g: &mut World, _: &Ctx, [name]: [Value; 1]) -> R<Value> {
    let r = g.host_mut().file_text_open_read(name.to_str());
    Ok(file("file_text_open_read", r)?.into())
}

pub fn file_text_open_write(g: &mut World, _: &Ctx, [name]: [Value; 1]) -> R<Value> {
    let r = g.host_mut().file_text_open_write(name.to_str());
    Ok(file("file_text_open_write", r)?.into())
}

pub fn file_text_close(g: &mut World, _: &Ctx, [handle]: [Value; 1]) -> R<Value> {
    let r = g.host_mut().file_text_close(handle.to_int()?);
    file("file_text_close", r)?;
    none()
}

pub fn file_text_eof(g: &mut World, _: &Ctx, [handle]: [Value; 1]) -> R<Value> {
    let r = g.host_mut().file_text_eof(handle.to_int()?);
    Ok(file("file_text_eof", r)?.into())
}

pub fn file_text_read_string(g: &mut World, _: &Ctx, [handle]: [Value; 1]) -> R<Value> {
    let r = g.host_mut().file_text_read_string(handle.to_int()?);
    Ok(file("file_text_read_string", r)?.into())
}

pub fn file_text_readln(g: &mut World, _: &Ctx, [handle]: [Value; 1]) -> R<Value> {
    let r = g.host_mut().file_text_readln(handle.to_int()?);
    file("file_text_readln", r)?;
    none()
}

pub fn file_text_write_string(g: &mut World, _: &Ctx, [handle, s]: [Value; 2]) -> R<Value> {
    let r = g
        .host_mut()
        .file_text_write_string(handle.to_int()?, s.to_str());
    file("file_text_write_string", r)?;
    none()
}

pub fn file_text_writeln(g: &mut World, _: &Ctx, [handle]: [Value; 1]) -> R<Value> {
    let r = g.host_mut().file_text_writeln(handle.to_int()?);
    file("file_text_writeln", r)?;
    none()
}

pub fn highscore_add(g: &mut World, _: &Ctx, [name, value]: [Value; 2]) -> R<Value> {
    g.host_mut().highscore_add(gm_str(&name), value.to_real());
    none()
}

pub fn highscore_value(g: &mut World, _: &Ctx, [place]: [Value; 1]) -> R<Value> {
    Ok(g.host().highscore_value(place.to_int()?).into())
}

pub fn highscore_clear(g: &mut World, _: &Ctx, []: [Value; 0]) -> R<Value> {
    g.host_mut().highscore_clear();
    none()
}

// ---- Shims ----

macro_rules! shims {
    ($($ticket:literal: [$($name:ident($n:literal),)*])*) => {
        $($(
            #[doc = concat!("Not implemented yet (", $ticket, ").")]
            pub fn $name(_: &mut World, _: &Ctx, _: [Value; $n]) -> R<Value> {
                Err(Error::Unsupported(concat!(stringify!($name), " (", $ticket, ")")))
            }
        )*)*

        /// Every shimmed built-in with the ticket that implements it.
        pub const SHIMS: &[(&str, &str)] = &[$($((stringify!($name), $ticket),)*)*];
    };
}

shims! {}

/// Built-ins only unreachable code calls, and the generated units that do
/// (`tests::unreachable_built_ins_stay_unreachable`).
pub const UNREACHABLE: &[(&str, &[&str])] = &[
    ("sound_play", &["objects/oTemp.rs"]),
    ("external_define", &["scripts/SS_"]),
    ("external_call", &["scripts/SS_"]),
];

/// `sound_play(sound)`: GM's own sound system. Only `oTemp` calls it, and nothing creates or places
/// `oTemp` (the game's sounds all go through SuperSound, the `SS_*` overrides).
pub fn sound_play(_: &mut World, _: &Ctx, _: [Value; 1]) -> R<Value> {
    Err(Error::Unsupported("sound_play (unreachable in 1.1)"))
}

/// `external_define(dll, name, calltype, restype, argnumber, argtypes...)`. Only the `SS_*` scripts call
/// it, and they are overridden, so this is reached only through their generated twins.
pub fn external_define<const N: usize>(_: &mut World, _: &Ctx, _: [Value; N]) -> R<Value> {
    Err(Error::Unsupported("external_define (unreachable in 1.1)"))
}

/// `external_call(id, args...)`; see [`external_define`].
pub fn external_call<const N: usize>(_: &mut World, _: &Ctx, _: [Value; N]) -> R<Value> {
    Err(Error::Unsupported("external_call (unreachable in 1.1)"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gm::{Event, InputEvent, Inst};
    use std::sync::Arc;
    use trace::replay::{Button, JoystickInfo, MouseButton};

    fn game(joystick: Option<JoystickInfo>) -> crate::Game {
        let assets = Arc::new(gm::Assets::new(Vec::new(), Vec::new()));
        crate::Game::new(
            assets,
            crate::Config {
                joystick,
                ..crate::Config::default()
            },
        )
    }

    /// Calls `f` as GML would, returning a real.
    fn call<const N: usize>(
        g: &mut World,
        f: fn(&mut World, &Ctx, [Value; N]) -> R<Value>,
        args: [f64; N],
    ) -> f64 {
        let cx = Ctx::event(Inst(0), Inst(0), Event::Step, ObjectId(0));
        f(g, &cx, args.map(Value::Real)).unwrap().to_real()
    }

    #[test]
    fn without_a_joystick_the_builtins_answer_like_the_oracle() {
        let mut game = game(None);
        let g = &mut game.world;
        // OpenGMK's stubs (`gml/kernel.rs`), for both joystick ids the game asks about.
        for id in [1.0, 2.0] {
            assert_eq!(call(g, joystick_exists, [id]), 0.0);
            assert_eq!(call(g, joystick_check_button, [id, 1.0]), 0.0);
            assert_eq!(call(g, joystick_direction, [id]), 101.0);
            assert_eq!(call(g, joystick_has_pov, [id]), 0.0);
            assert_eq!(call(g, joystick_pov, [id]), -1.0);
            assert_eq!(call(g, joystick_zpos, [id]), 0.0);
        }
    }

    #[test]
    fn with_a_joystick_the_builtins_read_its_events() {
        let mut game = game(Some(JoystickInfo { has_pov: true }));
        game.step(&[
            InputEvent::Press(Button::Joy(2)),
            InputEvent::Axis(Axis::X, -1.0),
            InputEvent::Axis(Axis::Z, -0.5),
            InputEvent::Axis(Axis::Pov, 90.0),
            InputEvent::Press(Button::Mouse(MouseButton::Left)),
        ])
        .unwrap();
        let g = &mut game.world;
        assert_eq!(call(g, joystick_exists, [1.0]), 1.0);
        assert_eq!(call(g, joystick_exists, [2.0]), 0.0);
        assert_eq!(call(g, joystick_check_button, [1.0, 2.0]), 1.0);
        assert_eq!(call(g, joystick_check_button, [1.0, 1.0]), 0.0);
        assert_eq!(call(g, joystick_direction, [1.0]), 100.0);
        assert_eq!(call(g, joystick_has_pov, [1.0]), 1.0);
        assert_eq!(call(g, joystick_pov, [1.0]), 90.0);
        assert_eq!(call(g, joystick_zpos, [1.0]), -0.5);
        assert_eq!(call(g, mouse_check_button, [1.0]), 1.0);
        assert_eq!(call(g, mouse_check_button, [-1.0]), 1.0);
        assert_eq!(call(g, mouse_check_button, [0.0]), 0.0);
        assert_eq!(call(g, mouse_check_button, [2.0]), 0.0);
    }

    #[test]
    fn the_coverage_table_counts_match() {
        assert_eq!(SHIMS.len(), 0);
        assert_eq!(UNREACHABLE.len(), 3);
        let doc = include_str!("builtins.rs");
        let row = |name: &str| {
            doc.lines()
                .find(|l| l.starts_with("//! |") && l.contains(&format!("`{name}`")))
                .unwrap_or_else(|| panic!("{name} is missing from the coverage table"))
        };
        for (name, ticket) in SHIMS {
            assert!(row(name).ends_with(&format!("shim: {ticket} |")), "{name}");
        }
        for (name, _) in UNREACHABLE {
            assert!(row(name).ends_with("unreachable in 1.1 |"), "{name}");
        }
    }

    /// The unreachable built-ins are only called from the units [`UNREACHABLE`] lists: the overridden
    /// `SS_*` scripts, and `oTemp`, which no generated code creates (its placement in rooms is checked
    /// by the smoke test).
    #[test]
    fn unreachable_built_ins_stay_unreachable() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generated");
        let mut units = Vec::new();
        for sub in ["objects", "scripts"] {
            for e in std::fs::read_dir(dir.join(sub)).unwrap() {
                let path = e.unwrap().path();
                let text = std::fs::read_to_string(&path).unwrap();
                let name = path.file_name().unwrap().to_str().unwrap().to_string();
                units.push((format!("{sub}/{name}"), text));
            }
        }
        assert!(units.len() > 100);
        for (builtin, allowed) in UNREACHABLE {
            for (unit, text) in &units {
                if text.contains(&format!("b::{builtin}(")) {
                    assert!(
                        allowed.iter().any(|a| unit.starts_with(a)),
                        "{unit} calls {builtin}"
                    );
                }
            }
        }
        for (unit, text) in &units {
            // `obj::oTemp` exactly, not `obj::oTemple...`.
            let refers = text.match_indices("obj::oTemp").any(|(i, m)| {
                !text[i + m.len()..].starts_with(|c: char| c.is_alphanumeric() || c == '_')
            });
            assert!(!refers, "{unit} refers to oTemp");
        }
        // ...and the `SS_*` scripts never run: every one is overridden.
        for (unit, _) in &units {
            if let Some(script) = unit
                .strip_prefix("scripts/")
                .and_then(|u| u.strip_suffix(".rs"))
                && script.starts_with("SS_")
            {
                assert!(crate::overrides::OVERRIDDEN.contains(&script), "{unit}");
            }
        }
    }
}
