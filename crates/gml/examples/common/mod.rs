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

/// GM8's built-in constants, by family, from the GM 8.0 manual. A name missing here would be taken for an
/// instance variable; the `every_read_variable_is_assigned` test in the transpiler catches that for the names
/// Spelunky uses.
pub const CONSTANTS: &[&str] = &[
    // Math.
    "pi",
    // Colours.
    "c_aqua",
    "c_black",
    "c_blue",
    "c_dkgray",
    "c_fuchsia",
    "c_gray",
    "c_green",
    "c_lime",
    "c_ltgray",
    "c_maroon",
    "c_navy",
    "c_olive",
    "c_orange",
    "c_purple",
    "c_red",
    "c_silver",
    "c_teal",
    "c_white",
    "c_yellow",
    // Keys.
    "vk_nokey",
    "vk_anykey",
    "vk_left",
    "vk_right",
    "vk_up",
    "vk_down",
    "vk_enter",
    "vk_escape",
    "vk_space",
    "vk_shift",
    "vk_control",
    "vk_alt",
    "vk_backspace",
    "vk_tab",
    "vk_home",
    "vk_end",
    "vk_delete",
    "vk_insert",
    "vk_pageup",
    "vk_pagedown",
    "vk_pause",
    "vk_printscreen",
    "vk_f1",
    "vk_f2",
    "vk_f3",
    "vk_f4",
    "vk_f5",
    "vk_f6",
    "vk_f7",
    "vk_f8",
    "vk_f9",
    "vk_f10",
    "vk_f11",
    "vk_f12",
    "vk_numpad0",
    "vk_numpad1",
    "vk_numpad2",
    "vk_numpad3",
    "vk_numpad4",
    "vk_numpad5",
    "vk_numpad6",
    "vk_numpad7",
    "vk_numpad8",
    "vk_numpad9",
    "vk_multiply",
    "vk_divide",
    "vk_add",
    "vk_subtract",
    "vk_decimal",
    "vk_lshift",
    "vk_lcontrol",
    "vk_lalt",
    "vk_rshift",
    "vk_rcontrol",
    "vk_ralt",
    // Mouse buttons.
    "mb_any",
    "mb_none",
    "mb_left",
    "mb_right",
    "mb_middle",
    // Cursors.
    "cr_default",
    "cr_none",
    "cr_arrow",
    "cr_cross",
    "cr_beam",
    "cr_size_nesw",
    "cr_size_ns",
    "cr_size_nwse",
    "cr_size_we",
    "cr_uparrow",
    "cr_hourglass",
    "cr_drag",
    "cr_nodrop",
    "cr_hsplit",
    "cr_vsplit",
    "cr_multidrag",
    "cr_sqlwait",
    "cr_no",
    "cr_appstart",
    "cr_help",
    "cr_handpoint",
    "cr_size_all",
    // Blend modes.
    "bm_normal",
    "bm_add",
    "bm_max",
    "bm_subtract",
    "bm_zero",
    "bm_one",
    "bm_src_color",
    "bm_inv_src_color",
    "bm_src_alpha",
    "bm_inv_src_alpha",
    "bm_dest_alpha",
    "bm_inv_dest_alpha",
    "bm_dest_color",
    "bm_inv_dest_color",
    "bm_src_alpha_sat",
    // External calls.
    "dll_cdecl",
    "dll_stdcall",
    "ty_real",
    "ty_string",
    // Events.
    "ev_create",
    "ev_destroy",
    "ev_step",
    "ev_alarm",
    "ev_keyboard",
    "ev_mouse",
    "ev_collision",
    "ev_other",
    "ev_draw",
    "ev_keypress",
    "ev_keyrelease",
    "ev_trigger",
    "ev_step_normal",
    "ev_step_begin",
    "ev_step_end",
    "ev_left_button",
    "ev_right_button",
    "ev_middle_button",
    "ev_no_button",
    "ev_left_press",
    "ev_right_press",
    "ev_middle_press",
    "ev_left_release",
    "ev_right_release",
    "ev_middle_release",
    "ev_mouse_enter",
    "ev_mouse_leave",
    "ev_mouse_wheel_up",
    "ev_mouse_wheel_down",
    "ev_global_left_button",
    "ev_global_right_button",
    "ev_global_middle_button",
    "ev_global_left_press",
    "ev_global_right_press",
    "ev_global_middle_press",
    "ev_global_left_release",
    "ev_global_right_release",
    "ev_global_middle_release",
    "ev_outside",
    "ev_boundary",
    "ev_game_start",
    "ev_game_end",
    "ev_room_start",
    "ev_room_end",
    "ev_no_more_lives",
    "ev_no_more_health",
    "ev_animation_end",
    "ev_end_of_path",
    "ev_close_button",
    "ev_user0",
    "ev_user1",
    "ev_user2",
    "ev_user3",
    "ev_user4",
    "ev_user5",
    "ev_user6",
    "ev_user7",
    "ev_user8",
    "ev_user9",
    "ev_user10",
    "ev_user11",
    "ev_user12",
    "ev_user13",
    "ev_user14",
    "ev_user15",
    // Text alignment and file attributes.
    "fa_left",
    "fa_center",
    "fa_right",
    "fa_top",
    "fa_middle",
    "fa_bottom",
    "fa_readonly",
    "fa_hidden",
    "fa_sysfile",
    "fa_volumeid",
    "fa_directory",
    "fa_archive",
    // Primitives.
    "pr_pointlist",
    "pr_linelist",
    "pr_linestrip",
    "pr_trianglelist",
    "pr_trianglestrip",
    "pr_trianglefan",
    // Sound effects.
    "se_none",
    "se_chorus",
    "se_echo",
    "se_flanger",
    "se_gargle",
    "se_reverb",
    "se_compressor",
    "se_equalizer",
    // Effects and particles.
    "ef_explosion",
    "ef_ring",
    "ef_ellipse",
    "ef_firework",
    "ef_smoke",
    "ef_smokeup",
    "ef_star",
    "ef_spark",
    "ef_flare",
    "ef_cloud",
    "ef_rain",
    "ef_snow",
    "pt_shape_pixel",
    "pt_shape_disk",
    "pt_shape_square",
    "pt_shape_line",
    "pt_shape_star",
    "pt_shape_circle",
    "pt_shape_ring",
    "pt_shape_sphere",
    "pt_shape_flare",
    "pt_shape_spark",
    "pt_shape_explosion",
    "pt_shape_cloud",
    "pt_shape_smoke",
    "pt_shape_snow",
    "ps_shape_rectangle",
    "ps_shape_ellipse",
    "ps_shape_diamond",
    "ps_shape_line",
    "ps_distr_linear",
    "ps_distr_gaussian",
    "ps_distr_invgaussian",
    "ps_change_all",
    "ps_change_shape",
    "ps_change_motion",
    "ps_force_constant",
    "ps_force_linear",
    "ps_force_quadratic",
    "ps_deflect_vertical",
    "ps_deflect_horizontal",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum NameKind {
    Local,
    /// `argument0` to `argument15`.
    Argument(u8),
    /// The `argument[i]` array (bare `argument` is `argument[0]`).
    ArgumentArray,
    /// `argument_count`.
    ArgumentCount,
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
    /// Scripts and code actions, by unit name.
    pub programs: BTreeMap<String, Program>,
    /// Drag-and-drop expression arguments, by unit name.
    pub dnd_exprs: BTreeMap<String, Expr>,
    /// Unit name -> index in `units`.
    index: BTreeMap<String, usize>,
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
        let mut dnd_exprs = BTreeMap::new();
        for unit in &units {
            let fail = |e: gml::Error| format!("{}: {}", unit.name, e.display(&unit.source));
            if unit.kind == UnitKind::DndExpression {
                let expr =
                    gml::parse_expression(&unit.source).unwrap_or_else(|e| panic!("{}", fail(e)));
                dnd_exprs.insert(unit.name.clone(), expr);
            } else {
                let program =
                    gml::parse_program(&unit.source).unwrap_or_else(|e| panic!("{}", fail(e)));
                programs.insert(unit.name.clone(), program);
            }
        }
        let index = units
            .iter()
            .enumerate()
            .map(|(i, u)| (u.name.clone(), i))
            .collect();
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
            dnd_exprs,
            index,
            parents,
            globalvars,
        }
    }

    /// The source text of a unit.
    pub fn source(&self, unit: &str) -> &str {
        &self.units[self.index[unit]].source
    }

    /// Classifies a bare identifier. `locals` are the unit's `var` names.
    pub fn classify(&self, name: &str, locals: &BTreeSet<String>) -> NameKind {
        if locals.contains(name) {
            NameKind::Local
        } else if let Some(n) = name.strip_prefix("argument").and_then(|n| {
            n.parse::<u8>()
                .ok()
                .filter(|v| *v < 16 && v.to_string() == n)
        }) {
            NameKind::Argument(n)
        } else if name == "argument" {
            NameKind::ArgumentArray
        } else if name == "argument_count" {
            NameKind::ArgumentCount
        } else if INSTANCE_BUILTINS.contains(&name) {
            NameKind::InstanceBuiltin
        } else if GLOBAL_BUILTINS.contains(&name) {
            NameKind::GlobalBuiltin
        } else if CONSTANTS.contains(&name) {
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

/// Whether a script can finish without a `return <value>`: some path falls off its end, or it uses `exit`.
/// GM8 then returns the last assigned or called value (OpenGMK `gml/runtime.rs`: `SetField`,
/// `SetVariable` and `EvalExpression` set `context.return_value`, which `exit` leaves as is).
pub fn can_end_without_return(body: &[Stmt]) -> bool {
    fn returns(stmt: &Stmt) -> bool {
        match &stmt.kind {
            StmtKind::Return(_) => true,
            StmtKind::Block(b) => b.last().is_some_and(returns),
            StmtKind::If {
                then,
                otherwise: Some(o),
                ..
            } => returns(then) && returns(o),
            _ => false,
        }
    }
    struct HasExit(bool);
    impl Visitor for HasExit {
        fn visit_stmt(&mut self, stmt: &Stmt) {
            self.0 |= matches!(stmt.kind, StmtKind::Exit);
            visit::walk_stmt(self, stmt);
        }
    }
    let mut exit = HasExit(false);
    for stmt in body {
        exit.visit_stmt(stmt);
    }
    exit.0 || !body.last().is_some_and(returns)
}
