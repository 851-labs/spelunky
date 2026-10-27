//! The layout of the readable GML tree `extract` writes (`gml/` in the asset directory): its file names,
//! and where each code action starts in its event's file. Shared with `wiki-data`, whose citations point
//! into the tree, so the two can't disagree.

use gmk::{Action, Event, EventType, Object, Script};

use crate::UniqueNames;

/// Code as the tree writes it: LF line endings and a final newline.
pub fn lf(code: &str) -> String {
    let mut s = code.replace("\r\n", "\n");
    if !s.is_empty() && !s.ends_with('\n') {
        s.push('\n');
    }
    s
}

/// Each script's file, `gml/scripts/<name>.gml`, by script id.
pub fn script_files(scripts: &[Option<Script>]) -> Vec<Option<String>> {
    let mut names = UniqueNames::default();
    scripts
        .iter()
        .enumerate()
        .map(|(id, s)| {
            let s = s.as_ref()?;
            Some(format!("gml/scripts/{}.gml", names.get(&s.name, id)))
        })
        .collect()
}

/// Each object's event files, `gml/objects/<object>/<event>.gml`, by object id and then in the
/// object's event order.
pub fn event_files(objects: &[Option<Object>]) -> Vec<Option<Vec<String>>> {
    let mut names = UniqueNames::default();
    objects
        .iter()
        .enumerate()
        .map(|(id, o)| {
            let o = o.as_ref()?;
            let dir = format!("gml/objects/{}", names.get(&o.name, id));
            let mut event_names = UniqueNames::default();
            Some(
                o.events
                    .iter()
                    .enumerate()
                    .map(|(i, e)| {
                        format!("{dir}/{}.gml", event_names.get(&event_name(objects, e), i))
                    })
                    .collect(),
            )
        })
        .collect()
}

/// Whether an event's file gives every action a `// ==== action N: ... ====` header line. Only a lone
/// plain code action that runs on self goes without one.
pub fn has_headers(actions: &[Action]) -> bool {
    !matches!(actions, [only] if only.code().is_some()
        && (!only.has_applies_to || only.applies_to == -1)
        && !only.relative
        && !only.invert)
}

/// The 1-based line of the event's file on which each action's code starts (each header is one line:
/// `extract` keeps it so), or `None` for an action without code.
pub fn code_lines(actions: &[Action]) -> Vec<Option<usize>> {
    let headers = has_headers(actions);
    let mut line = 1;
    actions
        .iter()
        .map(|action| {
            line += usize::from(headers);
            let code = action.code()?;
            let start = line;
            line += lf(code).matches('\n').count();
            Some(start)
        })
        .collect()
}

/// A file name for an event, e.g. `alarm_0`, `step_end`, `collision_oSolid`, `keypress_F1`.
pub fn event_name(objects: &[Option<Object>], event: &Event) -> String {
    let sub = event.sub;
    let base = event.event_type.name();
    match event.event_type {
        EventType::Create | EventType::Destroy | EventType::Draw => base.to_owned(),
        EventType::Alarm | EventType::Trigger => format!("{base}_{sub}"),
        EventType::Step => match sub {
            0 => "step".to_owned(),
            1 => "step_begin".to_owned(),
            2 => "step_end".to_owned(),
            _ => format!("step_{sub}"),
        },
        EventType::Collision => match gmk::get(objects, sub) {
            Some(o) => format!("{base}_{}", o.name),
            None => format!("{base}_<{sub}>"),
        },
        EventType::Keyboard | EventType::KeyPress | EventType::KeyRelease => {
            format!("{base}_{}", key_name(sub))
        }
        EventType::Mouse => format!("{base}_{}", mouse_event_name(sub)),
        EventType::Other => format!("{base}_{}", other_event_name(sub)),
    }
}

fn key_name(code: i32) -> String {
    let named = match code {
        0 => "nokey",
        1 => "anykey",
        8 => "backspace",
        9 => "tab",
        13 => "enter",
        16 => "shift",
        17 => "control",
        18 => "alt",
        19 => "pause",
        27 => "escape",
        32 => "space",
        33 => "pageup",
        34 => "pagedown",
        35 => "end",
        36 => "home",
        37 => "left",
        38 => "up",
        39 => "right",
        40 => "down",
        45 => "insert",
        46 => "delete",
        _ => "",
    };
    match code {
        _ if !named.is_empty() => named.to_owned(),
        48..=57 | 65..=90 => char::from(code as u8).to_string(),
        96..=105 => format!("numpad{}", code - 96),
        112..=123 => format!("F{}", code - 111),
        _ => code.to_string(),
    }
}

fn mouse_event_name(sub: i32) -> String {
    const NAMES: [&str; 12] = [
        "left_button",
        "right_button",
        "middle_button",
        "no_button",
        "left_press",
        "right_press",
        "middle_press",
        "left_release",
        "right_release",
        "middle_release",
        "enter",
        "leave",
    ];
    const GLOBAL: [&str; 9] = [
        "global_left_button",
        "global_right_button",
        "global_middle_button",
        "global_left_press",
        "global_right_press",
        "global_middle_press",
        "global_left_release",
        "global_right_release",
        "global_middle_release",
    ];
    let name = match sub {
        0..=11 => NAMES.get(sub as usize).copied(),
        50..=58 => GLOBAL.get((sub - 50) as usize).copied(),
        60 => Some("wheel_up"),
        61 => Some("wheel_down"),
        _ => None,
    };
    name.map_or_else(|| sub.to_string(), str::to_owned)
}

fn other_event_name(sub: i32) -> String {
    let name = match sub {
        0 => "outside",
        1 => "boundary",
        2 => "game_start",
        3 => "game_end",
        4 => "room_start",
        5 => "room_end",
        6 => "no_more_lives",
        7 => "animation_end",
        8 => "end_of_path",
        9 => "no_more_health",
        10..=25 => return format!("user{}", sub - 10),
        30 => "close_button",
        _ => return sub.to_string(),
    };
    name.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_and_key_names() {
        assert_eq!(key_name(13), "enter");
        assert_eq!(key_name(77), "M");
        assert_eq!(key_name(113), "F2");
        assert_eq!(key_name(300), "300");
        assert_eq!(mouse_event_name(12), "12");
        assert_eq!(mouse_event_name(60), "wheel_up");
        assert_eq!(other_event_name(7), "animation_end");
        assert_eq!(other_event_name(12), "user2");
    }
}
