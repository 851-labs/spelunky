//! Objects, their events, and drag-and-drop action lists.

use crate::error::ErrorKind;
use crate::reader::{Reader, Result};
use crate::resources::read_section;

/// GM 8.0's event types, in the order GM stores them (the discriminant is the stored number).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EventType {
    Create = 0,
    Destroy = 1,
    Alarm = 2,
    Step = 3,
    Collision = 4,
    Keyboard = 5,
    Mouse = 6,
    Other = 7,
    Draw = 8,
    KeyPress = 9,
    KeyRelease = 10,
    Trigger = 11,
}

impl EventType {
    pub const ALL: [EventType; 12] = [
        EventType::Create,
        EventType::Destroy,
        EventType::Alarm,
        EventType::Step,
        EventType::Collision,
        EventType::Keyboard,
        EventType::Mouse,
        EventType::Other,
        EventType::Draw,
        EventType::KeyPress,
        EventType::KeyRelease,
        EventType::Trigger,
    ];

    pub fn from_index(i: usize) -> Option<EventType> {
        Self::ALL.get(i).copied()
    }

    pub fn name(self) -> &'static str {
        match self {
            EventType::Create => "create",
            EventType::Destroy => "destroy",
            EventType::Alarm => "alarm",
            EventType::Step => "step",
            EventType::Collision => "collision",
            EventType::Keyboard => "keyboard",
            EventType::Mouse => "mouse",
            EventType::Other => "other",
            EventType::Draw => "draw",
            EventType::KeyPress => "keypress",
            EventType::KeyRelease => "keyrelease",
            EventType::Trigger => "trigger",
        }
    }
}

/// One event of an object: its type, sub-number (alarm index, step kind, collision object id, key code,
/// mouse or "other" event number) and action list.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Event {
    pub event_type: EventType,
    pub sub: i32,
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ActionKind {
    Normal,
    BeginGroup,
    EndGroup,
    Else,
    Exit,
    Repeat,
    Variable,
    Code,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExecType {
    None,
    Function,
    Code,
}

/// Kind of a drag-and-drop action argument, which says how to interpret its string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ArgKind {
    Expression,
    String,
    Both,
    Boolean,
    Menu,
    Sprite,
    Sound,
    Background,
    Path,
    Script,
    Object,
    Room,
    Font,
    Color,
    Timeline,
    FontString,
}

impl ArgKind {
    fn from_i32(v: i32) -> Option<ArgKind> {
        use ArgKind::*;
        const KINDS: [ArgKind; 16] = [
            Expression, String, Both, Boolean, Menu, Sprite, Sound, Background, Path, Script,
            Object, Room, Font, Color, Timeline, FontString,
        ];
        usize::try_from(v).ok().and_then(|i| KINDS.get(i).copied())
    }
}

/// A drag-and-drop action. A code block is the action with `kind == ActionKind::Code` (library 1, id 603),
/// whose single argument is the GML.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Action {
    pub library_id: i32,
    pub action_id: i32,
    pub kind: ActionKind,
    pub may_be_relative: bool,
    pub is_question: bool,
    pub has_applies_to: bool,
    pub exec_type: ExecType,
    /// The runner function called when `exec_type` is `Function`, e.g. `action_inherited`.
    pub function: String,
    /// GML run when `exec_type` is `Code`.
    pub exec_code: String,
    /// -1 self, -2 other, otherwise an object id.
    pub applies_to: i32,
    pub relative: bool,
    /// The used arguments (GM always stores 8 slots; only the first `arg_count` mean anything).
    pub args: Vec<Arg>,
    /// The "NOT" checkbox of a question action.
    pub invert: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Arg {
    pub kind: ArgKind,
    pub value: String,
}

impl Action {
    /// The GML of a code action.
    pub fn code(&self) -> Option<&str> {
        match self.kind {
            ActionKind::Code => self.args.first().map(|a| a.value.as_str()),
            _ => None,
        }
    }
}

pub(crate) fn read_actions(r: &mut Reader<'_>) -> Result<Vec<Action>> {
    r.version("action list", 400)?;
    // An action is at least 64 bytes even with empty strings and no arguments.
    let n = r.count(64)?;
    let mut out = Vec::new();
    for _ in 0..n {
        out.push(read_action(r)?);
    }
    Ok(out)
}

fn read_action(r: &mut Reader<'_>) -> Result<Action> {
    r.version("action", 440)?;
    let library_id = r.i32()?;
    let action_id = r.i32()?;
    let kind = r.i32()?;
    let kind = match kind {
        0 => ActionKind::Normal,
        1 => ActionKind::BeginGroup,
        2 => ActionKind::EndGroup,
        3 => ActionKind::Else,
        4 => ActionKind::Exit,
        5 => ActionKind::Repeat,
        6 => ActionKind::Variable,
        7 => ActionKind::Code,
        value => {
            return Err(r.err(ErrorKind::InvalidEnum {
                what: "action kind",
                value,
            }));
        }
    };
    let may_be_relative = r.bool()?;
    let is_question = r.bool()?;
    let has_applies_to = r.bool()?;
    let exec_type = match r.i32()? {
        0 => ExecType::None,
        1 => ExecType::Function,
        2 => ExecType::Code,
        value => {
            return Err(r.err(ErrorKind::InvalidEnum {
                what: "action exec type",
                value,
            }));
        }
    };
    let function = r.string()?;
    let exec_code = r.string()?;
    let arg_count = r.len()?;
    let n_kinds = r.count(4)?;
    let mut kinds = Vec::new();
    for _ in 0..n_kinds {
        let v = r.i32()?;
        kinds.push(ArgKind::from_i32(v).ok_or_else(|| {
            r.err(ErrorKind::InvalidEnum {
                what: "argument kind",
                value: v,
            })
        })?);
    }
    let applies_to = r.i32()?;
    let relative = r.bool()?;
    let n_args = r.count(4)?;
    let mut values = Vec::new();
    for _ in 0..n_args {
        values.push(r.string()?);
    }
    let invert = r.bool()?;
    if arg_count > kinds.len() || arg_count > values.len() {
        return Err(r.err(ErrorKind::InvalidEnum {
            what: "action argument count",
            value: arg_count as i32,
        }));
    }
    let args = kinds
        .into_iter()
        .zip(values)
        .take(arg_count)
        .map(|(kind, value)| Arg { kind, value })
        .collect();
    Ok(Action {
        library_id,
        action_id,
        kind,
        may_be_relative,
        is_question,
        has_applies_to,
        exec_type,
        function,
        exec_code,
        applies_to,
        relative,
        args,
        invert,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Object {
    pub name: String,
    /// Sprite id, or -1.
    pub sprite: i32,
    pub solid: bool,
    pub visible: bool,
    pub depth: i32,
    pub persistent: bool,
    /// Parent object id, or -1.
    pub parent: i32,
    /// Mask sprite id, or -1 (use `sprite`).
    pub mask: i32,
    /// Events in GM's storage order: by type, then in the order the editor saved the sub-events.
    pub events: Vec<Event>,
}

pub(crate) fn read_objects(r: &mut Reader<'_>) -> Result<Vec<Option<Object>>> {
    read_section(r, "object", 800, |s, name| {
        s.version("object", 430)?;
        let sprite = s.i32()?;
        let solid = s.bool()?;
        let visible = s.bool()?;
        let depth = s.i32()?;
        let persistent = s.bool()?;
        let parent = s.i32()?;
        let mask = s.i32()?;
        // The stored number is the highest event type, so there are one more lists than it says.
        let last_type = s.len()?;
        let mut events = Vec::new();
        for i in 0..=last_type {
            let event_type = EventType::from_index(i).ok_or_else(|| {
                s.err(ErrorKind::InvalidEnum {
                    what: "event type",
                    value: i as i32,
                })
            })?;
            loop {
                let sub = s.i32()?;
                if sub == -1 {
                    break;
                }
                events.push(Event {
                    event_type,
                    sub,
                    actions: read_actions(s)?,
                });
            }
        }
        Ok(Object {
            name,
            sprite,
            solid,
            visible,
            depth,
            persistent,
            parent,
            mask,
            events,
        })
    })
}
