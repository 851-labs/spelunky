//! Structural parser tests: precedence, statement forms and error positions.

use gml::ast::*;
use gml::{parse_expression, parse_program};

/// Prints an expression with every binary node parenthesised, to make the tree shape visible.
fn shape(e: &Expr) -> String {
    match &e.kind {
        ExprKind::Binary { op, lhs, rhs } => {
            format!("({} {} {})", shape(lhs), op.as_str(), shape(rhs))
        }
        ExprKind::Unary { op, expr } => format!("{}{}", op.as_str(), shape(expr)),
        _ => gml::print::expr(e),
    }
}

fn expr_shape(src: &str) -> String {
    shape(&parse_expression(src).unwrap())
}

#[test]
fn gm8_precedence() {
    // `and`/`or`/`xor` share one level and associate left (unlike C, where `and` binds tighter).
    assert_eq!(expr_shape("a or b and c"), "((a || b) && c)");
    assert_eq!(expr_shape("a and b or c"), "((a && b) || c)");
    // Comparisons bind looser than bitwise operators.
    assert_eq!(expr_shape("a == b | c"), "(a == (b | c))");
    assert_eq!(expr_shape("a + b * c - d"), "((a + (b * c)) - d)");
    assert_eq!(expr_shape("a - b - c"), "((a - b) - c)");
    assert_eq!(expr_shape("a / b div c mod d"), "(((a / b) div c) mod d)");
    assert_eq!(expr_shape("1 << 2 + 3"), "(1 << (2 + 3))");
    // Unary operators bind tightest.
    assert_eq!(expr_shape("not a == b"), "(!a == b)");
    assert_eq!(expr_shape("-a * b"), "(-a * b)");
    // `=` in an expression is comparison.
    assert_eq!(expr_shape("a = b"), "(a == b)");
}

#[test]
fn postfix_chains() {
    let e = parse_expression("other.target.x").unwrap();
    let ExprKind::Field { base, name } = e.kind else {
        panic!()
    };
    assert_eq!(name.name, "x");
    assert!(matches!(base.kind, ExprKind::Field { .. }));

    let e = parse_expression("global.roomPath[i, j+1]").unwrap();
    let ExprKind::Index { base, indices } = e.kind else {
        panic!()
    };
    assert_eq!(indices.len(), 2);
    assert!(matches!(base.kind, ExprKind::Field { .. }));

    let e = parse_expression("(instance_nearest(x, y, oItem)).held").unwrap();
    assert!(matches!(e.kind, ExprKind::Field { .. }));
}

#[test]
fn statement_forms() {
    let p = parse_program("if a then b = 1 else c = 2 d = 3").unwrap();
    assert_eq!(p.body.len(), 2);
    assert!(matches!(
        p.body[0].kind,
        StmtKind::If {
            otherwise: Some(_),
            ..
        }
    ));

    // The dangling `else` binds to the nearest `if`.
    let p = parse_program("if a if b x = 1; else x = 2;").unwrap();
    let StmtKind::If {
        then,
        otherwise: None,
        ..
    } = &p.body[0].kind
    else {
        panic!()
    };
    assert!(matches!(
        then.kind,
        StmtKind::If {
            otherwise: Some(_),
            ..
        }
    ));

    // `var a b` declares two names, but stops before an identifier that starts a statement.
    let p = parse_program("var a b\nc = 1").unwrap();
    let StmtKind::Var(names) = &p.body[0].kind else {
        panic!()
    };
    assert_eq!(
        names.iter().map(|n| n.name.as_str()).collect::<Vec<_>>(),
        ["a", "b"]
    );
    assert!(matches!(p.body[1].kind, StmtKind::Assign { .. }));
    let p = parse_program("var obj\nfoo(1)").unwrap();
    assert_eq!(p.body.len(), 2);

    let p = parse_program("switch (x) { case 1: case 2: y = 1; break; default: y = 0 }").unwrap();
    let StmtKind::Switch { body, .. } = &p.body[0].kind else {
        panic!()
    };
    assert!(matches!(body[0].kind, StmtKind::Case(_)));
    assert!(matches!(body[4].kind, StmtKind::Default));

    let p = parse_program("with (oEnemy) do instance_destroy()").unwrap();
    let StmtKind::With { body, .. } = &p.body[0].kind else {
        panic!()
    };
    assert!(matches!(body.kind, StmtKind::Call(_)));

    let p = parse_program("a.b[1].c += 2").unwrap();
    assert!(matches!(
        p.body[0].kind,
        StmtKind::Assign {
            op: AssignOp::Add,
            ..
        }
    ));
}

#[test]
fn spans_map_to_lines() {
    let src = "a = 1;\r\nif (b)\r\n{\r\n    c = 2;\r\n}\r\n";
    let p = parse_program(src).unwrap();
    let idx = gml::LineIndex::new(src);
    let StmtKind::If { then, .. } = &p.body[1].kind else {
        panic!()
    };
    let StmtKind::Block(inner) = &then.kind else {
        panic!()
    };
    assert_eq!(idx.line_col(inner[0].span.start).line, 4);
    // Statement spans stop before the optional trailing semicolon.
    assert_eq!(inner[0].span.text(src), "c = 2");
    assert_eq!(p.body[1].span.text(src), "if (b)\r\n{\r\n    c = 2;\r\n}");
}

#[test]
fn errors() {
    for (src, line) in [
        ("a = ;", 1),
        ("if (a) {\n b = 1\n", 3),
        ("x + 1;", 1),
        ("f(1, 2;", 1),
        ("a.1 = 2", 1),
        ("3 = 4", 1),
        ("\n\nelse x = 1", 3),
    ] {
        let err = parse_program(src).expect_err(src);
        let at = gml::LineIndex::new(src).line_col(err.span.start);
        assert_eq!(at.line, line, "{src:?}: {}", err.display(src));
    }
    assert!(parse_expression("a b").is_err());

    // A missing expression at the end of the code (an empty DnD argument, `x = `) is an error at the end,
    // not a panic or an error on the previous token.
    for src in ["", "  ", "// nothing"] {
        let err = parse_expression(src).expect_err(src);
        assert_eq!(err.span.start as usize, src.len(), "{src:?}");
    }
    let err = parse_program("x = ").unwrap_err();
    assert_eq!(err.span.start, 4);
    assert!(
        err.message.ends_with("found end of code"),
        "{}",
        err.message
    );
}

#[test]
fn reals_print_as_their_gm8_source() {
    // GM8 reads `0.6` as `6 * 0.1`, one ulp off the nearest double. Printing gives the source text back,
    // not the shortest round-trip form of that value.
    for src in ["0.6", "0.3", "0.25", "1.5", "100001", "0.05", "2.7"] {
        let e = parse_expression(src).unwrap();
        assert_eq!(gml::print::expr(&e), src);
    }
    let e = parse_expression("0.6").unwrap();
    assert!(matches!(e.kind, ExprKind::Real(v) if v == 0.6000000000000001));
}
