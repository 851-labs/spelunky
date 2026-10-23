//! Variable layouts (runtime.md section 3): for each object, the user variables its instances can be
//! expected to hold, so they get a slot instead of the overflow list. That's everything assigned on
//! `self` by the object's events and the scripts they call (transitively; a script writes into whichever
//! instance runs it), plus `obj.v = ...` writes and assignments inside `with (obj)`. Ancestors' variables are
//! added by `ProgramBuilder::build`. A variable the analysis misses still works, so this is only an
//! optimisation and can't change behaviour.

use std::collections::{BTreeMap, BTreeSet};

use gml::ast::*;
use gml::visit::{self, Visitor};

use crate::source::{NameKind, Source, locals_of};

/// Whose variables a write lands in, as far as the analysis can tell.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Owner {
    /// The running instance (`self`).
    This,
    /// Every instance of a named object.
    Object(String),
    /// Something only known at run time (`other`, an instance id).
    Unknown,
}

/// What a unit writes and which scripts it calls, each with the owner `self` has at that point.
#[derive(Default)]
struct Effects {
    writes: BTreeSet<(Owner, String)>,
    calls: BTreeSet<(Owner, String)>,
}

struct Walker<'a> {
    src: &'a Source,
    locals: BTreeSet<String>,
    owner: Vec<Owner>,
    fx: Effects,
}

impl Walker<'_> {
    fn owner(&self) -> Owner {
        self.owner.last().cloned().unwrap_or(Owner::This)
    }

    /// The owner a `with` target or a `target.` base names.
    fn owner_of(&self, e: &Expr) -> Owner {
        match &e.kind {
            ExprKind::Const(Const::SelfKw) => self.owner(),
            ExprKind::Ident(n) if self.src.classify(n, &self.locals) == NameKind::Object => {
                Owner::Object(n.clone())
            }
            _ => Owner::Unknown,
        }
    }

    fn write(&mut self, place: &Expr) {
        let place = match &place.kind {
            ExprKind::Index { base, .. } => base,
            _ => place,
        };
        let (owner, name) = match &place.kind {
            ExprKind::Ident(n) if self.src.classify(n, &self.locals) == NameKind::InstanceVar => {
                (self.owner(), n)
            }
            ExprKind::Field { base, name }
                if gm::Builtin::from_name(&name.name).is_none()
                    && !matches!(base.kind, ExprKind::Const(Const::Global)) =>
            {
                (self.owner_of(base), &name.name)
            }
            _ => return,
        };
        self.fx.writes.insert((owner, name.clone()));
    }
}

impl Visitor for Walker<'_> {
    fn visit_stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Assign { place, .. } => self.write(place),
            StmtKind::With { target, body } => {
                self.visit_expr(target);
                let owner = self.owner_of(target);
                self.owner.push(owner);
                self.visit_stmt(body);
                self.owner.pop();
                return;
            }
            _ => {}
        }
        visit::walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &Expr) {
        if let ExprKind::Call { name, .. } = &expr.kind
            && self.src.scripts.contains(&name.name)
        {
            self.fx.calls.insert((self.owner(), name.name.clone()));
        }
        visit::walk_expr(self, expr);
    }
}

fn effects(src: &Source, program: &Program, fx: Effects) -> Effects {
    let mut w = Walker {
        src,
        locals: locals_of(program),
        owner: Vec::new(),
        fx,
    };
    visit::walk_program(&mut w, program);
    w.fx
}

/// Where a callee's write lands, given the owner `self` had at the call.
fn rebase(write: &Owner, at_call: &Owner) -> Owner {
    match write {
        Owner::This => at_call.clone(),
        other => other.clone(),
    }
}

/// Each object's own layout (without its ancestors'), as sorted variable names, by object name.
pub fn layouts(src: &Source) -> BTreeMap<String, BTreeSet<String>> {
    // Every unit's direct effects.
    let mut units: BTreeMap<String, Effects> = BTreeMap::new();
    for (unit, program) in &src.programs {
        units.insert(unit.clone(), effects(src, program, Effects::default()));
    }
    // Drag-and-drop set-variable actions (writes on self) and script calls.
    for object in src.gmk.objects.iter().flatten() {
        for event in &object.events {
            let ev = crate::source::event_name(&src.gmk, event);
            for (i, action) in event.actions.iter().enumerate() {
                let unit = format!("{}/{ev}#{i}", object.name);
                if action.kind == gmk::ActionKind::Variable
                    && let ExprKind::Ident(n) = &src.dnd_exprs[&format!("{unit}:arg0")].kind
                    && src.classify(n, &BTreeSet::new()) == NameKind::InstanceVar
                {
                    let fx = units.entry(unit.clone()).or_default();
                    fx.writes.insert((Owner::This, n.clone()));
                }
                if (action.library_id, action.action_id) == (1, 601) {
                    let id: usize = action.args[0].value.parse().expect("script id");
                    let script = &src.gmk.scripts[id].as_ref().expect("script").name;
                    let fx = units.entry(unit).or_default();
                    fx.calls.insert((Owner::This, script.clone()));
                }
            }
        }
    }

    // What each script writes, through the scripts it calls: iterate to a fixpoint (scripts recurse).
    let mut scripts: BTreeMap<String, BTreeSet<(Owner, String)>> = BTreeMap::new();
    loop {
        let mut changed = false;
        for name in &src.scripts {
            let fx = &units[&format!("scripts/{name}")];
            let mut all = fx.writes.clone();
            for (at, callee) in &fx.calls {
                for (owner, v) in scripts.get(callee).into_iter().flatten() {
                    all.insert((rebase(owner, at), v.clone()));
                }
            }
            if scripts.get(name) != Some(&all) {
                scripts.insert(name.clone(), all);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (unit, fx) in &units {
        let this = match Source::unit_object(unit) {
            Some(o) => Owner::Object(o.to_owned()),
            None => continue,
        };
        let mut writes = fx.writes.clone();
        for (at, callee) in &fx.calls {
            for (owner, v) in &scripts[callee] {
                writes.insert((rebase(owner, at), v.clone()));
            }
        }
        for (owner, v) in writes {
            if let Owner::Object(o) = rebase(&owner, &this)
                && src.parents.contains_key(&o)
            {
                out.entry(o).or_default().insert(v);
            }
        }
    }
    out
}
