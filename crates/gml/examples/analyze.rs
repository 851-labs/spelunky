//! Source analyses behind `docs/design/translation.md` and `docs/design/runtime.md`: the GM8 semantics a
//! translation has to reproduce, counted over the whole game.
//!
//! ```sh
//! scripts/cargo run -p gml --example analyze
//! ```

mod common;

use std::collections::{BTreeMap, BTreeSet};

use common::{NameKind, Source, calls_any, locals_of};
use gml::ast::*;
use gml::visit::{self, Visitor};

#[derive(Default)]
struct Stats {
    stmts: usize,
    exprs: usize,
    logical: usize,
    logical_rhs_rng: usize,
    rng_examples: Vec<String>,
    logical_rhs_script: usize,
    logical_rhs_effect: usize,
    logical_rhs_foreign_field: usize,
    precedence_traps: Vec<String>,
    comparisons: usize,
    calls_builtin: usize,
    calls_script: usize,
    builtins: BTreeSet<String>,
    with_targets: BTreeMap<&'static str, usize>,
    field_bases_read: BTreeMap<&'static str, usize>,
    field_bases_write: BTreeMap<&'static str, usize>,
    compound_on_object: Vec<String>,
    destroy_then_more: usize,
    arrays_2d: usize,
    instance_vars: BTreeSet<String>,
    global_vars: BTreeSet<String>,
    /// Object -> instance variables assigned directly (bare name) in its own event code.
    assigned_by_object: BTreeMap<String, BTreeSet<String>>,
    /// Instance variables assigned directly in scripts (they land on whichever instance runs the script).
    assigned_in_scripts: BTreeSet<String>,
    /// Instance variables assigned through `other.`/`obj.`/`id.` on some other instance.
    assigned_remotely: BTreeSet<String>,
}

struct Walker<'a> {
    src: &'a Source,
    unit: &'a str,
    locals: BTreeSet<String>,
    s: &'a mut Stats,
}

fn base_kind(src: &Source, base: &Expr, locals: &BTreeSet<String>) -> &'static str {
    match &base.kind {
        ExprKind::Const(Const::Other) => "other",
        ExprKind::Const(Const::Global) => "global",
        ExprKind::Const(Const::SelfKw) => "self",
        ExprKind::Const(_) => "keyword",
        ExprKind::Ident(name) => match src.classify(name, locals) {
            NameKind::Object => "object name",
            NameKind::Local | NameKind::InstanceVar | NameKind::GlobalVar => {
                "variable (instance id)"
            }
            NameKind::Argument(_) => "argument",
            _ => "other expression",
        },
        ExprKind::Field { .. } => "field (instance id)",
        _ => "other expression",
    }
}

fn is_rng(name: &str) -> bool {
    name == "rand" || name == "random"
}

impl Walker<'_> {
    fn is_script(&self, name: &str) -> bool {
        self.src.scripts.contains(name)
    }

    fn note_assign(&mut self, place: &Expr, op: AssignOp) {
        let (name, base) = match &place.kind {
            ExprKind::Ident(n) => (n, None),
            ExprKind::Field { base, name } => (&name.name, Some(base)),
            ExprKind::Index { base, .. } => match &base.kind {
                ExprKind::Ident(n) => (n, None),
                ExprKind::Field { base, name } => (&name.name, Some(base)),
                _ => return,
            },
            _ => return,
        };
        match base {
            None => {
                if self.src.classify(name, &self.locals) == NameKind::InstanceVar {
                    match Source::unit_object(self.unit) {
                        Some(obj) => {
                            self.s
                                .assigned_by_object
                                .entry(obj.to_owned())
                                .or_default()
                                .insert(name.clone());
                        }
                        None => {
                            self.s.assigned_in_scripts.insert(name.clone());
                        }
                    }
                }
            }
            Some(base) => {
                let kind = base_kind(self.src, base, &self.locals);
                *self.s.field_bases_write.entry(kind).or_default() += 1;
                if kind != "global" && kind != "self" {
                    self.s.assigned_remotely.insert(name.clone());
                }
                if kind == "object name" && op != AssignOp::Set {
                    self.s.compound_on_object.push(format!(
                        "{}: {}",
                        self.unit,
                        gml::print::expr(place)
                    ));
                }
            }
        }
    }
}

impl Visitor for Walker<'_> {
    fn visit_stmt(&mut self, stmt: &Stmt) {
        self.s.stmts += 1;
        match &stmt.kind {
            StmtKind::Assign { place, op, .. } => self.note_assign(place, *op),
            StmtKind::With { target, .. } => {
                let kind = match &target.kind {
                    ExprKind::Const(Const::Other) => "other",
                    ExprKind::Const(Const::SelfKw) => "self",
                    ExprKind::Const(Const::All) => "all",
                    ExprKind::Ident(n)
                        if self.src.classify(n, &self.locals) == NameKind::Object =>
                    {
                        "object name"
                    }
                    _ => "variable or expression (instance id)",
                };
                *self.s.with_targets.entry(kind).or_default() += 1;
            }
            _ => {}
        }
        visit::walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &Expr) {
        self.s.exprs += 1;
        match &expr.kind {
            ExprKind::Binary {
                op: op @ (BinOp::And | BinOp::Or | BinOp::Xor),
                lhs,
                rhs,
            } => {
                self.s.logical += 1;
                let scripts = &self.src.scripts;
                let script = |n: &str| scripts.contains(n) && n != "rand";
                let effect = |n: &str| {
                    is_rng(n)
                        || matches!(n, "instance_create" | "instance_destroy")
                        || scripts.contains(n)
                };
                let (rng, scr, eff) = (
                    calls_any(rhs, &is_rng),
                    calls_any(rhs, &script),
                    calls_any(rhs, &effect),
                );
                self.s.logical_rhs_rng += usize::from(rng);
                if rng && self.s.rng_examples.len() < 6 {
                    self.s
                        .rng_examples
                        .push(format!("{}: {}", self.unit, gml::print::expr(expr)));
                }
                self.s.logical_rhs_script += usize::from(scr);
                self.s.logical_rhs_effect += usize::from(eff);
                struct F<'b>(&'b Source, &'b BTreeSet<String>, bool);
                impl Visitor for F<'_> {
                    fn visit_expr(&mut self, e: &Expr) {
                        if let ExprKind::Field { base, .. } = &e.kind
                            && !matches!(base_kind(self.0, base, self.1), "global" | "self")
                        {
                            self.2 = true;
                        }
                        visit::walk_expr(self, e);
                    }
                }
                let mut f = F(self.src, &self.locals, false);
                f.visit_expr(rhs);
                if f.2 {
                    self.s.logical_rhs_foreign_field += 1;
                }
                // GM8 gives and/or/xor one precedence level; C would bind `and` tighter. A source
                // `a or b and c` therefore means `(a or b) and c`. Parenthesised operands have a span
                // wider than their own operands.
                if let ExprKind::Binary {
                    op: inner @ (BinOp::Or | BinOp::Xor),
                    lhs: l2,
                    rhs: r2,
                } = &lhs.kind
                {
                    let parenthesised = lhs.span != l2.span.to(r2.span);
                    if *op == BinOp::And && !parenthesised {
                        self.s
                            .precedence_traps
                            .push(format!("{}: {:?} under And", self.unit, inner));
                    }
                }
            }
            ExprKind::Binary { op, .. } if op.precedence() == 1 => self.s.comparisons += 1,
            ExprKind::Call { name, .. } => {
                if self.is_script(&name.name) {
                    self.s.calls_script += 1;
                } else {
                    self.s.calls_builtin += 1;
                    self.s.builtins.insert(name.name.clone());
                }
            }
            ExprKind::Field { base, name } => {
                let kind = base_kind(self.src, base, &self.locals);
                *self.s.field_bases_read.entry(kind).or_default() += 1;
                if kind == "global" {
                    self.s.global_vars.insert(name.name.clone());
                } else {
                    self.s.instance_vars.insert(name.name.clone());
                }
            }
            ExprKind::Index { indices, .. } if indices.len() == 2 => self.s.arrays_2d += 1,
            ExprKind::Ident(name) => match self.src.classify(name, &self.locals) {
                NameKind::InstanceVar => {
                    self.s.instance_vars.insert(name.clone());
                }
                NameKind::GlobalVar => {
                    self.s.global_vars.insert(name.clone());
                }
                _ => {}
            },
            _ => {}
        }
        visit::walk_expr(self, expr);
    }
}

/// Whether a script can finish without `return`, in which case GM8 returns the last assigned or called
/// value (OpenGMK `gml/runtime.rs`: `SetField`/`SetVariable`/`EvalExpression` set `context.return_value`).
fn can_fall_off(body: &[Stmt]) -> bool {
    fn ends(stmt: &Stmt) -> bool {
        match &stmt.kind {
            StmtKind::Return(_) | StmtKind::Exit => true,
            StmtKind::Block(b) => b.last().is_some_and(ends),
            StmtKind::If {
                then,
                otherwise: Some(o),
                ..
            } => ends(then) && ends(o),
            _ => false,
        }
    }
    !body.last().is_some_and(ends)
}

/// Counts `instance_destroy()` statements after which more of the unit can still run (GM8 keeps executing
/// the event after destroying `self`). `after` says whether code follows the enclosing statement.
fn destroy_then_more(stmts: &[Stmt], after: bool) -> usize {
    let mut n = 0;
    for (i, stmt) in stmts.iter().enumerate() {
        let rest = &stmts[i + 1..];
        let follows = after
            || rest
                .iter()
                .any(|s| !matches!(s.kind, StmtKind::Exit | StmtKind::Return(_)));
        let follows = follows
            && !rest
                .first()
                .is_some_and(|s| matches!(s.kind, StmtKind::Exit | StmtKind::Return(_)));
        n += match &stmt.kind {
            StmtKind::Call(Expr {
                kind: ExprKind::Call { name, .. },
                ..
            }) => usize::from(name.name == "instance_destroy" && follows),
            StmtKind::Block(b) => destroy_then_more(b, follows),
            StmtKind::If {
                then, otherwise, ..
            } => {
                destroy_then_more(std::slice::from_ref(then), follows)
                    + otherwise
                        .as_ref()
                        .map_or(0, |o| destroy_then_more(std::slice::from_ref(o), follows))
            }
            // Loop bodies run again, so code always follows inside them.
            StmtKind::While { body, .. }
            | StmtKind::DoUntil { body, .. }
            | StmtKind::For { body, .. }
            | StmtKind::Repeat { count: _, body }
            | StmtKind::With { target: _, body } => {
                destroy_then_more(std::slice::from_ref(body), true)
            }
            StmtKind::Switch { body, .. } => destroy_then_more(body, follows),
            _ => 0,
        };
    }
    n
}

fn main() {
    let src = Source::load();
    let mut s = Stats::default();
    for (unit, program) in &src.programs {
        let locals = locals_of(program);
        let mut w = Walker {
            src: &src,
            unit,
            locals,
            s: &mut s,
        };
        visit::walk_program(&mut w, program);
        s.destroy_then_more += destroy_then_more(&program.body, false);
    }

    // Scripts whose value is used but which may end without `return`.
    let mut value_used: BTreeMap<String, usize> = BTreeMap::new();
    for program in src.programs.values() {
        struct U<'a>(&'a Source, &'a mut BTreeMap<String, usize>);
        impl Visitor for U<'_> {
            fn visit_stmt(&mut self, stmt: &Stmt) {
                // A call statement discards its value; only its arguments are value uses.
                if let StmtKind::Call(Expr {
                    kind: ExprKind::Call { args, .. },
                    ..
                }) = &stmt.kind
                {
                    for a in args {
                        self.visit_expr(a);
                    }
                    return;
                }
                visit::walk_stmt(self, stmt);
            }
            fn visit_expr(&mut self, e: &Expr) {
                if let ExprKind::Call { name, .. } = &e.kind
                    && self.0.scripts.contains(&name.name)
                {
                    *self.1.entry(name.name.clone()).or_default() += 1;
                }
                visit::walk_expr(self, e);
            }
        }
        visit::walk_program(&mut U(&src, &mut value_used), program);
    }
    let fall_off: Vec<_> = src
        .programs
        .iter()
        .filter_map(|(unit, p)| unit.strip_prefix("scripts/").map(|name| (name, p)))
        .filter(|(name, p)| can_fall_off(&p.body) && value_used.contains_key(*name))
        .map(|(name, _)| format!("{name} ({} value uses)", value_used[name]))
        .collect();

    // Instance variables each object can hold through its own and its ancestors' event code.
    let mut per_object = Vec::new();
    for obj in src.parents.keys() {
        let mut vars = BTreeSet::new();
        let mut cur = Some(obj.clone());
        while let Some(o) = cur {
            if let Some(v) = s.assigned_by_object.get(&o) {
                vars.extend(v.iter().cloned());
            }
            cur = src.parents.get(&o).cloned().flatten();
        }
        per_object.push(vars.len());
    }
    per_object.sort_unstable();
    let total: usize = per_object.iter().sum();

    let lines: usize = src.units.iter().map(|u| u.source.lines().count()).sum();
    println!("units: {} ({} source lines)", src.units.len(), lines);
    println!("statements: {}, expressions: {}", s.stmts, s.exprs);
    println!("comparisons (epsilon-compared in GM8): {}", s.comparisons);
    println!(
        "and/or/xor: {} total; right operand calls rand/random: {}; calls a script other than rand: {}; \
         has any side-effecting call (rng, script, instance_create/destroy): {}; reads a field of another \
         instance: {}",
        s.logical,
        s.logical_rhs_rng,
        s.logical_rhs_script,
        s.logical_rhs_effect,
        s.logical_rhs_foreign_field
    );
    for e in &s.rng_examples {
        println!("    {e}");
    }
    println!(
        "`a or b and c` (GM8 groups as `(a or b) and c`): {}",
        s.precedence_traps.len()
    );
    for t in &s.precedence_traps {
        println!("    {t}");
    }
    println!(
        "calls: {} built-in ({} distinct), {} script",
        s.calls_builtin,
        s.builtins.len(),
        s.calls_script
    );
    println!("with targets: {:?}", s.with_targets);
    println!("field reads by base: {:?}", s.field_bases_read);
    println!("field writes by base: {:?}", s.field_bases_write);
    println!(
        "compound assignment through an object name (reads first, writes all): {}",
        s.compound_on_object.len()
    );
    for c in &s.compound_on_object {
        println!("    {c}");
    }
    println!(
        "instance_destroy() statements after which more of the unit can run: {}",
        s.destroy_then_more
    );
    println!("2-D array accesses: {}", s.arrays_2d);
    println!(
        "user instance variables: {} distinct; assigned in scripts: {}; assigned on other instances: {}",
        s.instance_vars.len(),
        s.assigned_in_scripts.len(),
        s.assigned_remotely.len()
    );
    println!(
        "per-object instance variables from own+ancestor events: max {}, median {}, mean {:.1}",
        per_object.last().unwrap(),
        per_object[per_object.len() / 2],
        total as f64 / per_object.len() as f64
    );
    println!("global variables: {}", s.global_vars.len());
    println!(
        "scripts whose value is used but which can end without `return`: {}",
        fall_off.len()
    );
    for f in &fall_off {
        println!("    {f}");
    }
}
