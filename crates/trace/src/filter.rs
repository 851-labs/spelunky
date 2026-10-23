//! Object and field filters, shared by trace writers (to keep traces small) and `trace-diff`.
//!
//! Patterns are globs where `*` matches any run of characters. Field patterns match *paths*:
//!
//! - frame level: `room`, `room_name`, `seed`, `instance_count`, `globals.NAME`, `builtins.NAME`
//! - instance level: `active`, `object_index`, the real built-ins (`x`, `image_index`...), `sprite_index`,
//!   `sprite`, `visible`, `solid`, `alarms[0]` to `alarms[11]`, and `vars.NAME`
//! - array elements: `vars.NAME[j]` and `vars.NAME[i,j]` (likewise for globals)
//!
//! A pattern also matches every path below it, so `vars` matches `vars.xVel` and `alarms` matches
//! `alarms[3]`. Instance `id` and `object` are identity, not fields, and are never filtered out.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::trace::{ALARMS, Frame, Instance};
use crate::value::{Scalar, Value, index_suffix};

/// Which instances and fields to keep. The default keeps everything.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Filter {
    /// Keep only instances whose object matches one of these (all when empty).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub objects: Vec<String>,
    /// Drop instances whose object matches one of these.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ignore_objects: Vec<String>,
    /// Keep only fields matching one of these (all when empty). The frame's room, seed and instance count
    /// and each instance's `active` are always kept unless ignored, since every other difference follows from
    /// them.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<String>,
    /// Drop fields matching one of these.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ignore_fields: Vec<String>,
}

/// Fields an include list never drops.
const ALWAYS_KEPT: [&str; 5] = ["room", "room_name", "seed", "instance_count", "active"];

impl Filter {
    /// Splits a comma-separated pattern list, as given on a command line.
    pub fn split_list(list: &str) -> Vec<String> {
        list.split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(String::from)
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self == &Filter::default()
    }

    pub fn keeps_object(&self, object: &str) -> bool {
        (self.objects.is_empty() || self.objects.iter().any(|p| glob(p, object)))
            && !self.ignore_objects.iter().any(|p| glob(p, object))
    }

    pub fn keeps_path(&self, path: &str) -> bool {
        (self.fields.is_empty()
            || ALWAYS_KEPT.contains(&path)
            || self.fields.iter().any(|p| path_matches(p, path)))
            && !self.ignore_fields.iter().any(|p| path_matches(p, path))
    }

    /// Removes every instance and field the filter drops. `instance_count` keeps counting every active
    /// instance.
    pub fn apply(&self, frame: &mut Frame) {
        if !self.is_empty() {
            *frame = self.filtered(frame);
        }
    }

    /// A copy of `frame` holding only what the filter keeps, built without cloning what it drops. Array
    /// elements are filtered one by one (by the same paths `trace-diff` compares), so a writer's filter and
    /// the diff agree on what was kept.
    pub fn filtered(&self, frame: &Frame) -> Frame {
        let keep = |path: &str| self.keeps_path(path);
        let instances = |list: &[Instance]| -> Vec<Instance> {
            list.iter()
                .filter(|i| self.keeps_object(&i.object))
                .map(|i| self.filtered_instance(i))
                .collect()
        };
        Frame {
            frame: frame.frame,
            room: frame.room.filter(|_| keep("room")),
            room_name: frame.room_name.clone().filter(|_| keep("room_name")),
            seed: frame.seed.filter(|_| keep("seed")),
            instance_count: frame.instance_count.filter(|_| keep("instance_count")),
            instances: instances(&frame.instances),
            deactivated: frame.deactivated.as_deref().map(instances),
            globals: frame
                .globals
                .as_ref()
                .map(|g| self.filtered_vars("globals", g)),
            builtins: frame
                .builtins
                .as_ref()
                .map(|b| self.filtered_vars("builtins", b)),
        }
    }

    fn filtered_instance(&self, inst: &Instance) -> Instance {
        let keep = |path: &str| self.keeps_path(path);
        let mut out = Instance::new(inst.id, &inst.object);
        out.object_index = inst.object_index.filter(|_| keep("object_index"));
        for name in Instance::REAL_FIELDS {
            if keep(name) {
                *out.real_mut(name).expect("listed field") = inst.real(name);
            }
        }
        out.sprite_index = inst.sprite_index.filter(|_| keep("sprite_index"));
        out.sprite = inst.sprite.clone().filter(|_| keep("sprite"));
        out.visible = inst.visible.filter(|_| keep("visible"));
        out.solid = inst.solid.filter(|_| keep("solid"));
        if (0..ALARMS).any(|i| keep(&format!("alarms[{i}]"))) {
            out.alarms = inst.alarms.as_ref().map(|alarms| {
                alarms
                    .iter()
                    .filter(|(i, _)| keep(&format!("alarms[{i}]")))
                    .map(|(i, v)| (*i, *v))
                    .collect()
            });
        }
        out.vars = inst.vars.as_ref().map(|v| self.filtered_vars("vars", v));
        out
    }

    /// Keeps the variables, and the array elements, whose paths the filter keeps. An array that loses every
    /// element is dropped.
    fn filtered_vars(
        &self,
        prefix: &str,
        vars: &BTreeMap<String, Value>,
    ) -> BTreeMap<String, Value> {
        let mut out = BTreeMap::new();
        for (name, value) in vars {
            let kept = match value {
                Value::Array(elements) if !elements.is_empty() => {
                    let kept: BTreeMap<u32, Scalar> = elements
                        .iter()
                        .filter(|(i, _)| self.keeps_path(&element_path(prefix, name, **i)))
                        .map(|(i, v)| (*i, v.clone()))
                        .collect();
                    (!kept.is_empty()).then_some(Value::Array(kept))
                }
                _ => self
                    .keeps_path(&element_path(prefix, name, 0))
                    .then(|| value.clone()),
            };
            if let Some(v) = kept {
                out.insert(name.clone(), v);
            }
        }
        out
    }
}

/// The path of one element of a variable. Element 0 is the variable itself (GML reads `a` as `a[0]`).
pub(crate) fn element_path(prefix: &str, name: &str, index: u32) -> String {
    if index == 0 {
        format!("{prefix}.{name}")
    } else {
        format!("{prefix}.{name}{}", index_suffix(index))
    }
}

/// Whether `pattern` matches `path` or one of its ancestors (`vars` for `vars.x`, `alarms` for `alarms[3]`,
/// `builtins.view_xview` for `builtins.view_xview[0]`).
fn path_matches(pattern: &str, path: &str) -> bool {
    glob(pattern, path)
        || path
            .char_indices()
            .any(|(i, c)| (c == '.' || c == '[') && glob(pattern, &path[..i]))
}

/// `*` matches any run of characters (including none); everything else matches itself.
pub fn glob(pattern: &str, text: &str) -> bool {
    let (p, t) = (pattern.as_bytes(), text.as_bytes());
    let (mut pi, mut ti) = (0, 0);
    // Backtracking point: the pattern position after the last `*`, and the text position it was tried at.
    let mut star: Option<(usize, usize)> = None;
    while ti < t.len() {
        if pi < p.len() && p[pi] == b'*' {
            pi += 1;
            star = Some((pi, ti));
        } else if pi < p.len() && p[pi] == t[ti] {
            pi += 1;
            ti += 1;
        } else if let Some((sp, st)) = star {
            pi = sp;
            ti = st + 1;
            star = Some((sp, st + 1));
        } else {
            return false;
        }
    }
    p[pi..].iter().all(|&c| c == b'*')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globs() {
        assert!(glob("oPlayer1", "oPlayer1"));
        assert!(!glob("oPlayer", "oPlayer1"));
        assert!(glob("oPlayer*", "oPlayer1"));
        assert!(glob("*", ""));
        assert!(glob("o*Bat*", "oGiantBatX"));
        assert!(glob("*a*b", "xaab"));
        assert!(!glob("*a*b", "xaabc"));
        assert!(glob("a**", "a"));
        assert!(!glob("", "a"));
    }

    #[test]
    fn paths() {
        let f = Filter {
            fields: Filter::split_list("x, vars.xVel,alarms,globals.*money*,, "),
            ..Filter::default()
        };
        assert_eq!(f.fields.len(), 4);
        for keep in [
            "x",
            "vars.xVel",
            "vars.xVel[3]",
            "alarms[3]",
            "globals.money",
            "globals.moneyCount",
            "room",
            "seed",
            "active",
        ] {
            assert!(f.keeps_path(keep), "{keep}");
        }
        for drop in [
            "xprevious",
            "vars.xVelLimit",
            "vars.yVel",
            "globals.lives",
            "builtins.view_xview[0]",
        ] {
            assert!(!f.keeps_path(drop), "{drop}");
        }
        let f = Filter {
            ignore_fields: vec!["alarms[3]".into(), "vars".into(), "seed".into()],
            ..Filter::default()
        };
        assert!(f.keeps_path("alarms[2]") && f.keeps_path("x"));
        assert!(!f.keeps_path("alarms[3]") && !f.keeps_path("vars.a") && !f.keeps_path("seed"));
        let f = Filter {
            ignore_fields: vec!["globals.roomPath[1,*]".into()],
            ..Filter::default()
        };
        assert!(f.keeps_path("globals.roomPath[0,1]") && !f.keeps_path("globals.roomPath[1,3]"));
        let f = Filter {
            fields: vec!["builtins.view_xview".into()],
            ..Filter::default()
        };
        assert!(f.keeps_path("builtins.view_xview[0]"));
    }

    #[test]
    fn objects() {
        let f = Filter {
            objects: vec!["oPlayer1".into(), "oBat*".into()],
            ignore_objects: vec!["oBatTrap".into()],
            ..Filter::default()
        };
        assert!(f.keeps_object("oPlayer1") && f.keeps_object("oBat"));
        assert!(!f.keeps_object("oBatTrap") && !f.keeps_object("oSnake"));
        assert!(Filter::default().keeps_object("anything"));
    }
}
