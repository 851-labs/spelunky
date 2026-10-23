//! Every GML unit of Spelunky Classic 1.1, read with the `gmk` crate from
//! `reference/spelunky_1_1_src.zip`. Shared by the full-source test and the examples.

#![allow(dead_code)]

use std::io::Read;

use gmk::{Action, ActionKind, ArgKind, Event, EventType, Gmk};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitKind {
    /// A whole script.
    Script,
    /// One "execute code" action of an object event.
    CodeAction,
    /// One expression argument of a drag-and-drop action (`action_if`, set variable, execute script).
    DndExpression,
}

#[derive(Clone, Debug)]
pub struct Unit {
    /// `scripts/<name>`, `<object>/<event>#<action>`, or `<object>/<event>#<action>:arg<n>`.
    pub name: String,
    pub kind: UnitKind,
    pub source: String,
}

/// Reads and parses `spelunky.gmk` from the reference zip.
pub fn load_gmk() -> Gmk {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../reference/spelunky_1_1_src.zip"
    );
    let file = std::fs::File::open(path).expect("reference/spelunky_1_1_src.zip");
    let mut zip = zip::ZipArchive::new(file).expect("read zip");
    let mut bytes = Vec::new();
    zip.by_name("spelunky.gmk")
        .expect("spelunky.gmk in zip")
        .read_to_end(&mut bytes)
        .unwrap();
    gmk::parse(&bytes).expect("spelunky.gmk parses")
}

/// A short, file-name-like name for an event: `create`, `step`, `step_begin`, `alarm_0`,
/// `collision_oBlood`, `keypress_13`, `other_7`, `mouse_4`.
pub fn event_name(gmk: &Gmk, event: &Event) -> String {
    let sub = event.sub;
    match event.event_type {
        EventType::Create | EventType::Destroy | EventType::Draw => {
            event.event_type.name().to_owned()
        }
        EventType::Step => match sub {
            1 => "step_begin".to_owned(),
            2 => "step_end".to_owned(),
            _ => "step".to_owned(),
        },
        EventType::Collision => {
            let target = gmk.objects.get(sub as usize).and_then(Option::as_ref);
            format!(
                "collision_{}",
                target.map_or(sub.to_string(), |o| o.name.clone())
            )
        }
        other => format!("{}_{sub}", other.name()),
    }
}

/// Every unit: scripts first (by id), then each object's events in storage order.
pub fn units(gmk: &Gmk) -> Vec<Unit> {
    let mut units = Vec::new();
    for script in gmk.scripts.iter().flatten() {
        units.push(Unit {
            name: format!("scripts/{}", script.name),
            kind: UnitKind::Script,
            source: script.code.clone(),
        });
    }
    for object in gmk.objects.iter().flatten() {
        for event in &object.events {
            let prefix = format!("{}/{}", object.name, event_name(gmk, event));
            for (i, action) in event.actions.iter().enumerate() {
                if let Some(code) = action.code() {
                    units.push(Unit {
                        name: format!("{prefix}#{i}"),
                        kind: UnitKind::CodeAction,
                        source: code.to_owned(),
                    });
                    continue;
                }
                for (n, source) in expression_args(action) {
                    units.push(Unit {
                        name: format!("{prefix}#{i}:arg{n}"),
                        kind: UnitKind::DndExpression,
                        source: source.to_owned(),
                    });
                }
            }
        }
    }
    units
}

/// The arguments of a drag-and-drop action that GM8 compiles as GML expressions. A set-variable action's
/// first argument (the variable name) is a string, but GM compiles it as an assignment target, so it is
/// included too.
pub fn expression_args(action: &Action) -> Vec<(usize, &str)> {
    if action.kind == ActionKind::Code {
        return Vec::new();
    }
    action
        .args
        .iter()
        .enumerate()
        .filter(|(n, a)| {
            a.kind == ArgKind::Expression || (action.kind == ActionKind::Variable && *n == 0)
        })
        .map(|(n, a)| (n, a.value.as_str()))
        .collect()
}
