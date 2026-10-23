//! How individual GML constructs translate: the GM8 rules of `docs/design/translation.md`, checked on
//! small snippets in the context of the real game (so names classify as they do in Spelunky).

use std::collections::BTreeSet;
use std::sync::OnceLock;

use gmk::{Action, ActionKind, Arg, ArgKind, EventType, ExecType};
use gml::ast::*;
use transpile::Source;
use transpile::emit::{Symbols, emit_event, emit_program};
use transpile::source::{NameKind, locals_of};

fn spelunky() -> &'static Source {
    static SRC: OnceLock<Source> = OnceLock::new();
    SRC.get_or_init(Source::load)
}

/// Translates `code` as a script (`unit` starting with `scripts/`) or an event action.
fn emit(unit: &str, code: &str) -> String {
    let program = gml::parse_program(code).unwrap();
    emit_program(
        spelunky(),
        unit,
        &program,
        code,
        "t",
        &mut Symbols::default(),
    )
}

#[test]
fn and_or_evaluate_both_operands() {
    let out = emit("oX/step#0", "if (a and rand(1, 8) == 1) b = 1;");
    assert!(
        out.contains(
            "if and(g.get(cx.this, v::a)?.is_truthy(), scr::rand(g, cx, &[r(1.0), r(8.0)])?.gml_eq(&r(1.0))?) {"
        ),
        "{out}"
    );
}

#[test]
fn not_rejects_strings_like_gm8() {
    let out = emit("oX/step#0", "if (not a) b = !c; d = !(e == 1);");
    assert!(
        out.contains("if g.get(cx.this, v::a)?.not()?.is_truthy() {"),
        "{out}"
    );
    assert!(
        out.contains("g.set(cx.this, v::b, g.get(cx.this, v::c)?.not()?)?;"),
        "{out}"
    );
    // On a comparison (always a real), `!` is plain negation.
    assert!(
        out.contains("b2v(!(g.get(cx.this, v::e)?.gml_eq(&r(1.0))?))"),
        "{out}"
    );
}

#[test]
fn strict_and_lenient_compound_assignment() {
    // Built-ins (including arguments) are strict...
    for (code, read) in [
        ("argument0 += 1", "cx.arg(0)?"),
        ("argument[1] += 1", "cx.arg_ix(1)?"),
        ("x += 1", "g.get(cx.this, v::x)?"),
        ("other.x += 1", "g.get(cx.other, v::x)?"),
    ] {
        let out = emit("scripts/t", code);
        assert!(
            out.contains(&format!("let x = {read}.add(&r(1.0))?;"))
                || out.contains(&format!("let old = {read};")),
            "{code}: {out}"
        );
        assert!(!out.contains("lenient"), "{code}: {out}");
    }
    // GM8 decides by the name alone, whatever the base: built-ins through `obj.` or `other.`, game-wide
    // ones included, are strict (OpenGMK `compiler.rs` make_modify_instruction).
    for code in [
        "obj.sprite_index += 1",
        "other.score += s",
        "obj.room -= 1",
        "view_xview[0] += 1",
        "global.x += 1",
    ] {
        let out = emit("oX/step#0", code);
        assert!(!out.contains("lenient"), "{code}: {out}");
    }
    // ...user variables, globals and locals are lenient.
    for code in [
        "a += 1",
        "other.a += 1",
        "obj.a += 1",
        "global.a += 1",
        "var l; l = 0; l += 1",
    ] {
        let out = emit("oX/step#0", code);
        assert!(out.contains("lenient(Value::add, "), "{code}: {out}");
        assert!(out.contains(", &r(1.0))"), "{code}: {out}");
    }
    assert!(transpile::emit::builtin_assign("argument15"));
    assert!(!transpile::emit::builtin_assign("argument16"));
}

#[test]
fn object_field_writes_write_all_and_read_the_first() {
    let out = emit("oX/step#0", "oMoonRoom.baskets += 1;");
    // The base is evaluated once for the write and again for the read.
    assert!(out.contains("let t = obj::oMoonRoom;"), "{out}");
    assert!(
        out.contains(
            "g.set_on(cx, &t, v::baskets, lenient(Value::add, { let t2 = obj::oMoonRoom; g.get_on(cx, &t2, v::baskets)? }, &r(1.0)))?;"
        ),
        "{out}"
    );
}

#[test]
fn values_that_call_are_bound_before_the_write() {
    let out = emit("oX/step#0", "a[i] = random(2);");
    // Index, then value, then the write.
    assert!(
        out.contains("let j = ix(&g.get(cx.this, v::i)?)?;"),
        "{out}"
    );
    assert!(
        out.contains("let x = b::random(g, cx, [r(2.0)])?;"),
        "{out}"
    );
    assert!(out.contains("g.set_a(cx.this, v::a, 0, j, x)?;"), "{out}");
    // One-dimensional built-in arrays use the column; literal indices aren't bound.
    let out = emit("oX/step#0", "alarm[3] = 10; view_xview[0] = 5;");
    assert!(
        out.contains("g.set_a(cx.this, v::alarm, 0, 3, r(10.0))?;"),
        "{out}"
    );
    assert!(out.contains("g.bset(v::view_xview, 0, r(5.0))?;"), "{out}");
}

#[test]
fn scripts_return_their_last_assignment() {
    let out = emit("scripts/t", "if (a) return 1; b = 2;");
    assert!(out.contains("cx.ret = x.clone();"), "{out}");
    assert!(out.ends_with("    Ok(cx.ret.clone())\n}\n"), "{out}");
    let out = emit("scripts/t", "return 1;");
    assert!(!out.contains("cx.ret ="), "{out}");
    // A `break` outside any loop ends the script like `exit`, with the last value.
    let out = emit("scripts/t", "a = 5; if (argument0) break; return 1;");
    assert!(out.contains("cx.ret = x.clone();"), "{out}");
    let out = emit("scripts/t", "while (a) { a -= 1; break; } return 1;");
    assert!(!out.contains("cx.ret ="), "{out}");
}

#[test]
fn switch_matches_exactly_and_falls_through() {
    let out = emit(
        "oX/step#0",
        "switch (a) { case 1: b = 1; case 2: b = 2; break; default: b = 3; }",
    );
    assert!(
        out.contains(
            "let entry: usize = if case_eq(&sw, &r(1.0)) { 0 } else if case_eq(&sw, &r(2.0)) { 1 } else { 2 };"
        ),
        "{out}"
    );
    assert!(out.contains("if entry <= 1 {"), "{out}");
    assert!(out.contains("break 'switch1;"), "{out}");
}

#[test]
fn with_swaps_self_and_other_and_restores_them() {
    let out = emit("oX/step#0", "with (oSnake) { hp = 0; break; }");
    assert!(out.contains("let cx = &mut cx.clone();"), "{out}");
    assert!(out.contains("let mut it1 = g.with_iter(cx, &t)?;"), "{out}");
    assert!(
        out.contains("(cx.this, cx.other) = (h2, outer3.0);"),
        "{out}"
    );
    assert!(out.contains("break 'with1;"), "{out}");
    assert!(out.contains("(cx.this, cx.other) = outer3;"), "{out}");
}

#[test]
fn loops_keep_continue_semantics() {
    let out = emit(
        "oX/step#0",
        "for (i = 0; i < 3; i += 1) { if (a) continue; b = 1; }",
    );
    // `continue` leaves the body block, so the step still runs.
    assert!(out.contains("'for1: loop {"), "{out}");
    assert!(out.contains("break 'body2;"), "{out}");
    let out = emit("oX/step#0", "repeat (n) a += 1;");
    assert!(
        out.contains("'repeat1: for _ in 0..repeat_count(&g.get(cx.this, v::n)?) {"),
        "{out}"
    );
}

#[test]
fn one_dimensional_arrays_take_computed_indices() {
    let out = emit(
        "scripts/t",
        "a = argument[scrX()]; b = view_xview[rand(0, 1)];",
    );
    assert!(out.contains("cx.arg_ix(j)?"), "{out}");
    assert!(out.contains("g.bget(v::view_xview, j)?"), "{out}");
}

#[test]
fn script_argument_forms() {
    let out = emit(
        "scripts/t",
        "a = argument[2] + argument_count; argument[0] = argument;",
    );
    assert!(out.contains("cx.arg_ix(2)?"), "{out}");
    assert!(out.contains("cx.argument_count()"), "{out}");
    assert!(out.contains("cx.set_arg_ix(0, x)?"), "{out}");
    assert!(out.contains("let x = cx.arg_ix(0)?;"), "{out}");
    let src = spelunky();
    let none = BTreeSet::new();
    assert_eq!(src.classify("argument15", &none), NameKind::Argument(15));
    assert_eq!(src.classify("argument16", &none), NameKind::InstanceVar);
}

#[test]
fn constants_come_from_the_table() {
    let src = spelunky();
    let none = BTreeSet::new();
    // The 27 constants Spelunky uses (docs/source-inventory.md, 5.4), each with a value.
    for c in [
        "c_white",
        "c_yellow",
        "ty_string",
        "dll_stdcall",
        "vk_enter",
        "vk_shift",
        "cr_default",
        "vk_escape",
        "c_black",
        "vk_backspace",
        "vk_delete",
        "cr_none",
        "bm_normal",
        "vk_up",
        "vk_down",
        "vk_left",
        "vk_right",
        "vk_space",
        "bm_one",
        "mb_left",
        "vk_control",
        "vk_alt",
        "bm_src_color",
        "bm_zero",
        "mb_right",
        "c_red",
        "bm_dest_color",
    ] {
        assert_eq!(src.classify(c, &none), NameKind::Constant, "{c}");
        assert!(transpile::source::constant_value(c).is_some(), "{c}");
    }
    // A user variable that merely looks like a constant is a variable.
    assert_eq!(src.classify("c_myColour", &none), NameKind::InstanceVar);
}

/// The classification cross-check from the inventory: every bare name read as an instance variable is
/// assigned somewhere, except the inventory's known dangling references.
#[test]
fn every_read_variable_is_assigned() {
    use gml::visit::{self, Visitor};
    struct Names<'a> {
        src: &'a Source,
        locals: BTreeSet<String>,
        read: BTreeSet<String>,
        assigned: BTreeSet<String>,
    }
    impl Visitor for Names<'_> {
        fn visit_stmt(&mut self, stmt: &Stmt) {
            if let StmtKind::Assign { place, .. } = &stmt.kind {
                let place = match &place.kind {
                    ExprKind::Index { base, .. } => base,
                    _ => place,
                };
                match &place.kind {
                    ExprKind::Ident(n) => self.assigned.insert(n.clone()),
                    ExprKind::Field { name, .. } => self.assigned.insert(name.name.clone()),
                    _ => false,
                };
            }
            visit::walk_stmt(self, stmt);
        }
        fn visit_expr(&mut self, expr: &Expr) {
            if let ExprKind::Ident(n) = &expr.kind
                && self.src.classify(n, &self.locals) == NameKind::InstanceVar
            {
                self.read.insert(n.clone());
            }
            visit::walk_expr(self, expr);
        }
    }
    let src = spelunky();
    let mut names = Names {
        src,
        locals: BTreeSet::new(),
        read: BTreeSet::new(),
        assigned: BTreeSet::new(),
    };
    for program in src.programs.values() {
        names.locals = locals_of(program);
        visit::walk_program(&mut names, program);
    }
    names.locals.clear();
    for (unit, expr) in &src.dnd_exprs {
        if unit.ends_with(":arg0") {
            // A set-variable action's target.
            if let ExprKind::Ident(n) = &expr.kind {
                names.assigned.insert(n.clone());
            }
        }
        names.visit_expr(expr);
    }
    let dangling: Vec<_> = names.read.difference(&names.assigned).cloned().collect();
    assert_eq!(dangling, ["oInit", "rInit"]);
}

fn action(id: i32, kind: ActionKind, args: &[&str], invert: bool) -> Action {
    Action {
        library_id: 1,
        action_id: id,
        kind,
        may_be_relative: false,
        is_question: id == 408,
        has_applies_to: true,
        exec_type: ExecType::Function,
        function: String::new(),
        exec_code: String::new(),
        applies_to: -1,
        relative: false,
        args: args
            .iter()
            .map(|v| Arg {
                kind: ArgKind::Expression,
                value: (*v).to_owned(),
            })
            .collect(),
        invert,
    }
}

#[test]
fn dnd_questions_nest() {
    let mut src = Source::load();
    let object = gmk::Object {
        name: "oTest".to_owned(),
        sprite: -1,
        solid: false,
        visible: true,
        depth: 0,
        persistent: false,
        parent: -1,
        mask: -1,
        events: Vec::new(),
    };
    let event = gmk::Event {
        event_type: EventType::Step,
        sub: 0,
        actions: vec![
            action(408, ActionKind::Normal, &["a"], false),
            action(408, ActionKind::Normal, &["b"], true),
            action(203, ActionKind::Normal, &[], false),
            action(408, ActionKind::Normal, &["c"], false),
        ],
    };
    for (i, name) in [(0, "a"), (1, "b"), (3, "c")] {
        src.dnd_exprs.insert(
            format!("oTest/step#{i}:arg0"),
            gml::parse_expression(name).unwrap(),
        );
    }
    let out = emit_event(&src, &object, &event, &mut Symbols::default());
    // [if a, if not b, destroy] nests, so `b` is only evaluated when `a` holds. Each condition records
    // its own action's location.
    assert!(
        out.contains(
            "if { g.at(\"oTest/step#0\", 0); g.get(cx.this, v::a)?.is_truthy() } { \
             if { g.at(\"oTest/step#1\", 0); !g.get(cx.this, v::b)?.is_truthy() } { \
             g.at(\"oTest/step#2\", 0); b::instance_destroy(g, cx, [])?; } }"
        ),
        "{out}"
    );
    // A trailing question still evaluates its condition.
    assert!(
        out.contains("if { g.at(\"oTest/step#3\", 0); g.get(cx.this, v::c)?.is_truthy() } {  }"),
        "{out}"
    );
}

#[test]
fn print_mode_selects_units() {
    let src = spelunky();
    // oGamepad's step has an `action_if`, whose argument is a DnD unit with the same prefix: only the code
    // action is printed.
    let out = transpile::print(src, &["oGamepad/step".to_owned()]).unwrap();
    assert!(out.contains("pub fn step_a1("), "{out}");
    assert!(!out.contains(":arg0"), "{out}");
    let out = transpile::print(src, &["scripts/rand".to_owned(), "oSnake/".to_owned()]).unwrap();
    assert!(out.contains("pub fn rand("), "{out}");
    assert!(out.contains("pub fn step(g: &mut World"), "{out}");
    assert!(transpile::print(src, &["oNoSuchObject/".to_owned()]).is_err());
    assert!(transpile::print(src, &["nothing".to_owned()]).is_err());
}
