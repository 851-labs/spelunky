//! AST traversal for analyses. Override the methods you care about and call the `walk_*` functions to
//! keep descending.

use crate::ast::*;
use crate::span::Span;

/// Visits statements and expressions in the order GM8 executes them (for a loop, one iteration's order).
pub trait Visitor {
    fn visit_stmt(&mut self, stmt: &Stmt) {
        walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &Expr) {
        walk_expr(self, expr);
    }
}

pub fn walk_program<V: Visitor + ?Sized>(v: &mut V, program: &Program) {
    for stmt in &program.body {
        v.visit_stmt(stmt);
    }
}

pub fn walk_stmt<V: Visitor + ?Sized>(v: &mut V, stmt: &Stmt) {
    match &stmt.kind {
        StmtKind::Block(body) => {
            for s in body {
                v.visit_stmt(s);
            }
        }
        StmtKind::Switch { value, body } => {
            v.visit_expr(value);
            for s in body {
                v.visit_stmt(s);
            }
        }
        StmtKind::Assign { place, value, .. } => {
            v.visit_expr(place);
            v.visit_expr(value);
        }
        StmtKind::Call(e) | StmtKind::Case(e) | StmtKind::Return(e) => v.visit_expr(e),
        StmtKind::If {
            cond,
            then,
            otherwise,
        } => {
            v.visit_expr(cond);
            v.visit_stmt(then);
            if let Some(o) = otherwise {
                v.visit_stmt(o);
            }
        }
        StmtKind::While { cond, body } => {
            v.visit_expr(cond);
            v.visit_stmt(body);
        }
        // Execution order: the body runs before the condition is first tested.
        StmtKind::DoUntil { body, cond } => {
            v.visit_stmt(body);
            v.visit_expr(cond);
        }
        StmtKind::For {
            init,
            cond,
            step,
            body,
        } => {
            v.visit_stmt(init);
            v.visit_expr(cond);
            v.visit_stmt(body);
            v.visit_stmt(step);
        }
        StmtKind::Repeat { count: e, body } | StmtKind::With { target: e, body } => {
            v.visit_expr(e);
            v.visit_stmt(body);
        }
        StmtKind::Var(_)
        | StmtKind::GlobalVar(_)
        | StmtKind::Default
        | StmtKind::Break
        | StmtKind::Continue
        | StmtKind::Exit
        | StmtKind::Empty => {}
    }
}

pub fn walk_expr<V: Visitor + ?Sized>(v: &mut V, expr: &Expr) {
    match &expr.kind {
        ExprKind::Real(_) | ExprKind::Str(_) | ExprKind::Const(_) | ExprKind::Ident(_) => {}
        ExprKind::Field { base, .. } => v.visit_expr(base),
        ExprKind::Index { base, indices } => {
            v.visit_expr(base);
            for i in indices {
                v.visit_expr(i);
            }
        }
        ExprKind::Call { args, .. } => {
            for a in args {
                v.visit_expr(a);
            }
        }
        ExprKind::Unary { expr, .. } => v.visit_expr(expr),
        ExprKind::Binary { lhs, rhs, .. } => {
            v.visit_expr(lhs);
            v.visit_expr(rhs);
        }
    }
}

/// Resets every span to the default, so trees parsed from differently formatted text can be compared.
pub fn clear_spans(program: &mut Program) {
    for stmt in &mut program.body {
        clear_stmt(stmt);
    }
}

fn clear_stmt(stmt: &mut Stmt) {
    stmt.span = Span::default();
    let clear_names =
        |names: &mut Vec<Ident>| names.iter_mut().for_each(|n| n.span = Span::default());
    match &mut stmt.kind {
        StmtKind::Block(body) => body.iter_mut().for_each(clear_stmt),
        StmtKind::Switch { value, body } => {
            clear_expr(value);
            body.iter_mut().for_each(clear_stmt);
        }
        StmtKind::Var(names) | StmtKind::GlobalVar(names) => clear_names(names),
        StmtKind::Assign { place, value, .. } => {
            clear_expr(place);
            clear_expr(value);
        }
        StmtKind::Call(e) | StmtKind::Case(e) | StmtKind::Return(e) => clear_expr(e),
        StmtKind::If {
            cond,
            then,
            otherwise,
        } => {
            clear_expr(cond);
            clear_stmt(then);
            if let Some(o) = otherwise {
                clear_stmt(o);
            }
        }
        StmtKind::While { cond, body } | StmtKind::DoUntil { body, cond } => {
            clear_expr(cond);
            clear_stmt(body);
        }
        StmtKind::For {
            init,
            cond,
            step,
            body,
        } => {
            clear_stmt(init);
            clear_expr(cond);
            clear_stmt(step);
            clear_stmt(body);
        }
        StmtKind::Repeat { count: e, body } | StmtKind::With { target: e, body } => {
            clear_expr(e);
            clear_stmt(body);
        }
        StmtKind::Default
        | StmtKind::Break
        | StmtKind::Continue
        | StmtKind::Exit
        | StmtKind::Empty => {}
    }
}

/// Resets every span in `expr` (see [`clear_spans`]).
pub fn clear_expr(expr: &mut Expr) {
    expr.span = Span::default();
    match &mut expr.kind {
        ExprKind::Real(_) | ExprKind::Str(_) | ExprKind::Const(_) | ExprKind::Ident(_) => {}
        ExprKind::Field { base, name } => {
            clear_expr(base);
            name.span = Span::default();
        }
        ExprKind::Index { base, indices } => {
            clear_expr(base);
            indices.iter_mut().for_each(clear_expr);
        }
        ExprKind::Call { name, args } => {
            name.span = Span::default();
            args.iter_mut().for_each(clear_expr);
        }
        ExprKind::Unary { expr, .. } => clear_expr(expr),
        ExprKind::Binary { lhs, rhs, .. } => {
            clear_expr(lhs);
            clear_expr(rhs);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Records identifiers in visiting order.
    struct Order(Vec<String>);
    impl Visitor for Order {
        fn visit_expr(&mut self, expr: &Expr) {
            if let ExprKind::Ident(name) = &expr.kind {
                self.0.push(name.clone());
            }
            walk_expr(self, expr);
        }
    }

    #[test]
    fn visits_in_execution_order() {
        let program = crate::parse_program(
            "do { a = b; } until (c) while (d) e = f; for (g = h; i; j = k) l = m; switch (n) { case o: p = q; }",
        )
        .unwrap();
        let mut order = Order(Vec::new());
        walk_program(&mut order, &program);
        assert_eq!(order.0.join(" "), "a b c d e f g h i l m j k n o p q");
    }
}
