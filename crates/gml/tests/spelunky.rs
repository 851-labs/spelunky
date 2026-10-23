//! Parses every GML unit of Spelunky Classic 1.1, read with the `gmk` crate straight from
//! `reference/spelunky_1_1_src.zip`: all scripts, all code actions, and every drag-and-drop argument that GM8
//! compiles as an expression. Each must parse and round-trip through the printer.

#[path = "../examples/common/spelunky.rs"]
mod spelunky;

use gml::visit::{clear_expr, clear_spans};
use spelunky::UnitKind;

#[test]
fn every_unit_parses_and_round_trips() {
    let units = spelunky::units(&spelunky::load_gmk());
    let count = |kind| units.iter().filter(|u| u.kind == kind).count();
    let mut failures = Vec::new();

    for unit in &units {
        let result = match unit.kind {
            UnitKind::Script | UnitKind::CodeAction => {
                gml::parse_program(&unit.source).map(|mut ast| {
                    // Printing and re-parsing must give the same tree: this checks the printer's
                    // parenthesisation against the parser's precedence on real code.
                    let printed = gml::print::program(&ast);
                    let mut again = gml::parse_program(&printed).unwrap_or_else(|e| {
                        panic!("{}: printed code doesn't parse: {e}\n{printed}", unit.name)
                    });
                    clear_spans(&mut ast);
                    clear_spans(&mut again);
                    assert_eq!(ast, again, "{} does not round-trip:\n{printed}", unit.name);
                })
            }
            UnitKind::DndExpression => gml::parse_expression(&unit.source).map(|mut ast| {
                let mut again = gml::parse_expression(&gml::print::expr(&ast)).unwrap();
                clear_expr(&mut ast);
                clear_expr(&mut again);
                assert_eq!(ast, again, "{} does not round-trip", unit.name);
            }),
        };
        if let Err(e) = result {
            failures.push(format!("{}: {}", unit.name, e.display(&unit.source)));
        }
    }

    assert!(
        failures.is_empty(),
        "units that failed to parse:\n{}",
        failures.join("\n")
    );
    // docs/source-inventory.md: 139 scripts and 969 code actions. The 68 DnD expressions are the
    // conditions of 3 `action_if`s, both sides of 15 set-variable actions, and 5 arguments each of 7
    // `action_execute_script`s.
    assert_eq!(
        (
            count(UnitKind::Script),
            count(UnitKind::CodeAction),
            count(UnitKind::DndExpression)
        ),
        (139, 969, 68)
    );
}
