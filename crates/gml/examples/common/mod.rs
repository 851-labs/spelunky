//! Shared by the analysis and transpiler-prototype examples: loading every unit of the game (through the
//! `gmk` crate) and classifying names the way GM8's compiler would (local, argument, built-in, resource,
//! script, global, or instance variable).

#![allow(dead_code)]

pub mod spelunky;

use std::collections::{BTreeMap, BTreeSet};

use gml::ast::*;
use gml::visit::{self, Visitor};
use spelunky::{Unit, UnitKind};

/// Built-in variables that live on each instance (GM8 `mappings.rs` instance variables, restricted to
/// the GM8 set). Spelunky uses about half of them.
pub const INSTANCE_BUILTINS: &[&str] = &[
    "x",
    "y",
    "xprevious",
    "yprevious",
    "xstart",
    "ystart",
    "hspeed",
    "vspeed",
    "direction",
    "speed",
    "friction",
    "gravity",
    "gravity_direction",
    "object_index",
    "id",
    "alarm",
    "solid",
    "visible",
    "persistent",
    "depth",
    "sprite_index",
    "sprite_width",
    "sprite_height",
    "sprite_xoffset",
    "sprite_yoffset",
    "image_alpha",
    "image_angle",
    "image_blend",
    "image_index",
    "image_number",
    "image_speed",
    "image_xscale",
    "image_yscale",
    "mask_index",
    "bbox_left",
    "bbox_right",
    "bbox_top",
    "bbox_bottom",
    "path_index",
    "path_position",
    "path_speed",
    "timeline_index",
    "timeline_position",
];

/// Built-in variables that are game-wide (rooms, views, input, ...).
pub const GLOBAL_BUILTINS: &[&str] = &[
    "room",
    "room_width",
    "room_height",
    "room_speed",
    "room_caption",
    "room_persistent",
    "view_enabled",
    "view_current",
    "view_visible",
    "view_xview",
    "view_yview",
    "view_wview",
    "view_hview",
    "view_xport",
    "view_yport",
    "view_wport",
    "view_hport",
    "view_angle",
    "view_hborder",
    "view_vborder",
    "view_hspeed",
    "view_vspeed",
    "view_object",
    "background_index",
    "background_color",
    "keyboard_key",
    "keyboard_lastkey",
    "keyboard_lastchar",
    "keyboard_string",
    "mouse_x",
    "mouse_y",
    "mouse_button",
    "mouse_lastbutton",
    "score",
    "lives",
    "health",
    "instance_count",
    "fps",
    "current_time",
    "working_directory",
];

/// Prefixes of GM8's built-in constants (`c_white`, `vk_enter`, ...).
const CONST_PREFIXES: &[&str] = &[
    "c_", "vk_", "cr_", "bm_", "mb_", "ty_", "dll_", "ev_", "fa_", "pr_", "se_",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum NameKind {
    Local,
    Argument(u8),
    InstanceBuiltin,
    GlobalBuiltin,
    Constant,
    Object,
    Sprite,
    Room,
    Background,
    Script,
    /// Declared with `globalvar`, so a bare name means `global.name`.
    GlobalVar,
    /// A user variable on the current instance.
    InstanceVar,
}

pub struct Source {
    pub gmk: gmk::Gmk,
    pub units: Vec<Unit>,
    pub programs: BTreeMap<String, Program>,
    /// Object name -> parent name.
    pub parents: BTreeMap<String, Option<String>>,
    pub scripts: BTreeSet<String>,
    pub sprites: BTreeSet<String>,
    pub rooms: BTreeSet<String>,
    pub backgrounds: BTreeSet<String>,
    pub globalvars: BTreeSet<String>,
}

fn names<T>(list: &[Option<T>], name: impl Fn(&T) -> &str) -> BTreeSet<String> {
    list.iter().flatten().map(|t| name(t).to_owned()).collect()
}

impl Source {
    /// Reads `spelunky.gmk` from the reference zip and parses every unit.
    pub fn load() -> Source {
        let gmk = spelunky::load_gmk();
        let units = spelunky::units(&gmk);
        let mut programs = BTreeMap::new();
        for unit in &units {
            let program = match unit.kind {
                UnitKind::DndExpression => {
                    let expr = gml::parse_expression(&unit.source).expect("parse");
                    Program {
                        body: vec![Stmt {
                            span: expr.span,
                            kind: StmtKind::Call(expr),
                        }],
                    }
                }
                _ => gml::parse_program(&unit.source)
                    .unwrap_or_else(|e| panic!("{}: {}", unit.name, e.display(&unit.source))),
            };
            programs.insert(unit.name.clone(), program);
        }
        let object_name = |id: i32| {
            gmk.objects
                .get(usize::try_from(id).ok()?)?
                .as_ref()
                .map(|o| o.name.clone())
        };
        let parents = gmk
            .objects
            .iter()
            .flatten()
            .map(|o| (o.name.clone(), object_name(o.parent)))
            .collect();
        let mut globalvars = BTreeSet::new();
        for program in programs.values() {
            struct G<'a>(&'a mut BTreeSet<String>);
            impl Visitor for G<'_> {
                fn visit_stmt(&mut self, stmt: &Stmt) {
                    if let StmtKind::GlobalVar(names) = &stmt.kind {
                        self.0.extend(names.iter().map(|n| n.name.clone()));
                    }
                    visit::walk_stmt(self, stmt);
                }
            }
            visit::walk_program(&mut G(&mut globalvars), program);
        }
        Source {
            scripts: names(&gmk.scripts, |s| &s.name),
            sprites: names(&gmk.sprites, |s| &s.name),
            rooms: names(&gmk.rooms, |r| &r.name),
            backgrounds: names(&gmk.backgrounds, |b| &b.name),
            gmk,
            units,
            programs,
            parents,
            globalvars,
        }
    }

    /// Classifies a bare identifier. `locals` are the unit's `var` names.
    pub fn classify(&self, name: &str, locals: &BTreeSet<String>) -> NameKind {
        if locals.contains(name) {
            NameKind::Local
        } else if let Some(n) = name
            .strip_prefix("argument")
            .and_then(|n| n.parse::<u8>().ok())
        {
            NameKind::Argument(n)
        } else if INSTANCE_BUILTINS.contains(&name) {
            NameKind::InstanceBuiltin
        } else if GLOBAL_BUILTINS.contains(&name) {
            NameKind::GlobalBuiltin
        } else if CONST_PREFIXES.iter().any(|p| name.starts_with(p)) || name == "pi" {
            NameKind::Constant
        } else if self.parents.contains_key(name) {
            NameKind::Object
        } else if self.sprites.contains(name) {
            NameKind::Sprite
        } else if self.rooms.contains(name) {
            NameKind::Room
        } else if self.backgrounds.contains(name) {
            NameKind::Background
        } else if self.scripts.contains(name) {
            NameKind::Script
        } else if self.globalvars.contains(name) {
            NameKind::GlobalVar
        } else {
            NameKind::InstanceVar
        }
    }

    /// The object an event unit belongs to (`oSnake/step#1` -> `oSnake`), if it is an object unit.
    pub fn unit_object(unit: &str) -> Option<&str> {
        if unit.starts_with("scripts/") {
            None
        } else {
            unit.split('/').next()
        }
    }
}

/// Names declared with `var` anywhere in the program (GM8 locals are unit-wide, not block-scoped).
pub fn locals_of(program: &Program) -> BTreeSet<String> {
    struct L(BTreeSet<String>);
    impl Visitor for L {
        fn visit_stmt(&mut self, stmt: &Stmt) {
            if let StmtKind::Var(names) = &stmt.kind {
                self.0.extend(names.iter().map(|n| n.name.clone()));
            }
            visit::walk_stmt(self, stmt);
        }
    }
    let mut l = L(BTreeSet::new());
    visit::walk_program(&mut l, program);
    l.0
}

/// Does any call in `expr` satisfy `pred` (by callee name)?
pub fn calls_any(expr: &Expr, pred: &dyn Fn(&str) -> bool) -> bool {
    struct C<'a>(&'a dyn Fn(&str) -> bool, bool);
    impl Visitor for C<'_> {
        fn visit_expr(&mut self, expr: &Expr) {
            if let ExprKind::Call { name, .. } = &expr.kind
                && (self.0)(&name.name)
            {
                self.1 = true;
            }
            visit::walk_expr(self, expr);
        }
    }
    let mut c = C(pred, false);
    c.visit_expr(expr);
    c.1
}
