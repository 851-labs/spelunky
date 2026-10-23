//! Every GML unit of the game, parsed, plus name classification: what a bare identifier means to GM8's
//! compiler (local, argument, built-in, resource, script, global, or instance variable).

use std::collections::{BTreeMap, BTreeSet};

use gm::Builtin;
use gmk::{Action, ActionKind, ArgKind, Event, EventType, Gmk};
use gml::ast::*;
use gml::visit::{self, Visitor};

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

/// A short, file-name-like name for an event: `create`, `step`, `step_begin`, `alarm_0`,
/// `collision_oBlood`, `keypress_13`, `other_7`, `mouse_4`. Generated event functions are named this way.
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

/// GM8's built-in constants, by family, from the GM 8.0 manual. A name missing here would be taken for an
/// instance variable; the `every_read_variable_is_assigned` test catches that for the names Spelunky uses.
/// Only constants with a [`constant_value`] can be translated; the generator stops on any other.
#[rustfmt::skip]
pub const CONSTANTS: &[&str] = &[
    // Math.
    "pi",
    // Colours.
    "c_aqua", "c_black", "c_blue", "c_dkgray", "c_fuchsia", "c_gray", "c_green", "c_lime", "c_ltgray",
    "c_maroon", "c_navy", "c_olive", "c_orange", "c_purple", "c_red", "c_silver", "c_teal", "c_white",
    "c_yellow",
    // Keys.
    "vk_nokey", "vk_anykey", "vk_left", "vk_right", "vk_up", "vk_down", "vk_enter", "vk_escape",
    "vk_space", "vk_shift", "vk_control", "vk_alt", "vk_backspace", "vk_tab", "vk_home", "vk_end",
    "vk_delete", "vk_insert", "vk_pageup", "vk_pagedown", "vk_pause", "vk_printscreen", "vk_f1", "vk_f2",
    "vk_f3", "vk_f4", "vk_f5", "vk_f6", "vk_f7", "vk_f8", "vk_f9", "vk_f10", "vk_f11", "vk_f12",
    "vk_numpad0", "vk_numpad1", "vk_numpad2", "vk_numpad3", "vk_numpad4", "vk_numpad5", "vk_numpad6",
    "vk_numpad7", "vk_numpad8", "vk_numpad9", "vk_multiply", "vk_divide", "vk_add", "vk_subtract",
    "vk_decimal", "vk_lshift", "vk_lcontrol", "vk_lalt", "vk_rshift", "vk_rcontrol", "vk_ralt",
    // Mouse buttons.
    "mb_any", "mb_none", "mb_left", "mb_right", "mb_middle",
    // Cursors.
    "cr_default", "cr_none", "cr_arrow", "cr_cross", "cr_beam", "cr_size_nesw", "cr_size_ns",
    "cr_size_nwse", "cr_size_we", "cr_uparrow", "cr_hourglass", "cr_drag", "cr_nodrop", "cr_hsplit",
    "cr_vsplit", "cr_multidrag", "cr_sqlwait", "cr_no", "cr_appstart", "cr_help", "cr_handpoint",
    "cr_size_all",
    // Blend modes.
    "bm_normal", "bm_add", "bm_max", "bm_subtract", "bm_zero", "bm_one", "bm_src_color",
    "bm_inv_src_color", "bm_src_alpha", "bm_inv_src_alpha", "bm_dest_alpha", "bm_inv_dest_alpha",
    "bm_dest_color", "bm_inv_dest_color", "bm_src_alpha_sat",
    // External calls.
    "dll_cdecl", "dll_stdcall", "ty_real", "ty_string",
    // Events.
    "ev_create", "ev_destroy", "ev_step", "ev_alarm", "ev_keyboard", "ev_mouse", "ev_collision",
    "ev_other", "ev_draw", "ev_keypress", "ev_keyrelease", "ev_trigger", "ev_step_normal",
    "ev_step_begin", "ev_step_end", "ev_left_button", "ev_right_button", "ev_middle_button",
    "ev_no_button", "ev_left_press", "ev_right_press", "ev_middle_press", "ev_left_release",
    "ev_right_release", "ev_middle_release", "ev_mouse_enter", "ev_mouse_leave", "ev_mouse_wheel_up",
    "ev_mouse_wheel_down", "ev_global_left_button", "ev_global_right_button", "ev_global_middle_button",
    "ev_global_left_press", "ev_global_right_press", "ev_global_middle_press", "ev_global_left_release",
    "ev_global_right_release", "ev_global_middle_release", "ev_outside", "ev_boundary", "ev_game_start",
    "ev_game_end", "ev_room_start", "ev_room_end", "ev_no_more_lives", "ev_no_more_health",
    "ev_animation_end", "ev_end_of_path", "ev_close_button", "ev_user0", "ev_user1", "ev_user2",
    "ev_user3", "ev_user4", "ev_user5", "ev_user6", "ev_user7", "ev_user8", "ev_user9", "ev_user10",
    "ev_user11", "ev_user12", "ev_user13", "ev_user14", "ev_user15",
    // Text alignment and file attributes.
    "fa_left", "fa_center", "fa_right", "fa_top", "fa_middle", "fa_bottom", "fa_readonly", "fa_hidden",
    "fa_sysfile", "fa_volumeid", "fa_directory", "fa_archive",
    // Primitives.
    "pr_pointlist", "pr_linelist", "pr_linestrip", "pr_trianglelist", "pr_trianglestrip",
    "pr_trianglefan",
    // Sound effects.
    "se_none", "se_chorus", "se_echo", "se_flanger", "se_gargle", "se_reverb", "se_compressor",
    "se_equalizer",
    // Effects and particles.
    "ef_explosion", "ef_ring", "ef_ellipse", "ef_firework", "ef_smoke", "ef_smokeup", "ef_star",
    "ef_spark", "ef_flare", "ef_cloud", "ef_rain", "ef_snow", "pt_shape_pixel", "pt_shape_disk",
    "pt_shape_square", "pt_shape_line", "pt_shape_star", "pt_shape_circle", "pt_shape_ring",
    "pt_shape_sphere", "pt_shape_flare", "pt_shape_spark", "pt_shape_explosion", "pt_shape_cloud",
    "pt_shape_smoke", "pt_shape_snow", "ps_shape_rectangle", "ps_shape_ellipse", "ps_shape_diamond",
    "ps_shape_line", "ps_distr_linear", "ps_distr_gaussian", "ps_distr_invgaussian", "ps_change_all",
    "ps_change_shape", "ps_change_motion", "ps_force_constant", "ps_force_linear", "ps_force_quadratic",
    "ps_deflect_vertical", "ps_deflect_horizontal",
];

/// The value of a built-in constant, for the ones the game uses (docs/source-inventory.md, section 5.4)
/// and their close relatives. From the GM 8.0 manual: colours are `0xBBGGRR`, keys are Windows virtual-key
/// codes, blend factors are Direct3D's `D3DBLEND` values.
pub fn constant_value(name: &str) -> Option<f64> {
    Some(match name {
        "pi" => std::f64::consts::PI,
        "c_black" => 0.0,
        "c_red" => 255.0,
        "c_yellow" => 65535.0,
        "c_white" => 16777215.0,
        "vk_nokey" => 0.0,
        "vk_anykey" => 1.0,
        "vk_backspace" => 8.0,
        "vk_tab" => 9.0,
        "vk_enter" => 13.0,
        "vk_shift" => 16.0,
        "vk_control" => 17.0,
        "vk_alt" => 18.0,
        "vk_escape" => 27.0,
        "vk_space" => 32.0,
        "vk_left" => 37.0,
        "vk_up" => 38.0,
        "vk_right" => 39.0,
        "vk_down" => 40.0,
        "vk_delete" => 46.0,
        "mb_any" => -1.0,
        "mb_none" => 0.0,
        "mb_left" => 1.0,
        "mb_right" => 2.0,
        "mb_middle" => 3.0,
        "cr_default" => 0.0,
        "cr_none" => -1.0,
        "bm_normal" => 0.0,
        "bm_add" => 1.0,
        "bm_max" => 2.0,
        "bm_subtract" => 3.0,
        "bm_zero" => 1.0,
        "bm_one" => 2.0,
        "bm_src_color" => 3.0,
        "bm_inv_src_color" => 4.0,
        "bm_src_alpha" => 5.0,
        "bm_inv_src_alpha" => 6.0,
        "bm_dest_alpha" => 7.0,
        "bm_inv_dest_alpha" => 8.0,
        "bm_dest_color" => 9.0,
        "bm_inv_dest_color" => 10.0,
        "bm_src_alpha_sat" => 11.0,
        "dll_cdecl" => 0.0,
        "dll_stdcall" => 1.0,
        "ty_real" => 0.0,
        "ty_string" => 1.0,
        _ => return None,
    })
}

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

/// The game's source: the `.gmk` and every unit, parsed.
pub struct Source {
    pub gmk: Gmk,
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
    /// Reads `spelunky.gmk` from the reference zip in this repository and parses every unit.
    pub fn load() -> Source {
        Source::new(crate::reference::load_gmk(&crate::reference::default_path()))
    }

    /// Parses every unit of a game. Panics on a unit that doesn't parse: every unit of Spelunky does.
    pub fn new(gmk: Gmk) -> Source {
        let units = units(&gmk);
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
        } else if let Some(b) = Builtin::from_name(name) {
            if b.is_instance() {
                NameKind::InstanceBuiltin
            } else {
                NameKind::GlobalBuiltin
            }
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

/// Whether a script can finish without a `return <value>`: some path falls off its end, or it uses `exit`
/// (or a `break`/`continue` outside any loop, which the translation treats as `exit`). GM8 then returns
/// the last assigned or called value (OpenGMK `gml/runtime.rs`: `SetField`, `SetVariable` and
/// `EvalExpression` set `context.return_value`, which `exit` leaves as is).
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
    /// Whether `stmt` leaves the unit early without a value. `in_loop`: `break` and `continue` have a
    /// loop (or `with`) to act on; `in_switch`: `break` has a `switch` to leave.
    fn exits(stmt: &Stmt, in_loop: bool, in_switch: bool) -> bool {
        let any = |b: &[Stmt], l, s| b.iter().any(|x| exits(x, l, s));
        match &stmt.kind {
            StmtKind::Exit => true,
            StmtKind::Break => !in_loop && !in_switch,
            StmtKind::Continue => !in_loop,
            StmtKind::Block(b) => any(b, in_loop, in_switch),
            StmtKind::Switch { body, .. } => any(body, in_loop, true),
            StmtKind::If {
                then, otherwise, ..
            } => {
                exits(then, in_loop, in_switch)
                    || otherwise
                        .as_ref()
                        .is_some_and(|o| exits(o, in_loop, in_switch))
            }
            StmtKind::While { body, .. }
            | StmtKind::DoUntil { body, .. }
            | StmtKind::Repeat { body, .. }
            | StmtKind::With { body, .. } => exits(body, true, false),
            // The init and step statements run outside the loop body.
            StmtKind::For {
                init, step, body, ..
            } => {
                exits(init, in_loop, in_switch)
                    || exits(step, in_loop, in_switch)
                    || exits(body, true, false)
            }
            _ => false,
        }
    }
    body.iter().any(|s| exits(s, false, false)) || !body.last().is_some_and(returns)
}
