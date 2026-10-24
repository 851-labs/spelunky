//! Number literals read as GM8 reads them: every literal of the `tools/gm8-probe` `literals` probe, lexed
//! here, against the real GM8 runner's answers (`results/gm8-wine/literals.txt`) and the oracle's
//! (`results/opengmk/literals.txt`), bit for bit.

use std::collections::BTreeMap;

use gml::lexer::{Tok, tokenize};

const PROBE: &str = include_str!("../../../tools/gm8-probe/probes/literals.gml");
const GM8: &str = include_str!("../../../tools/gm8-probe/results/gm8-wine/literals.txt");
const OPENGMK: &str = include_str!("../../../tools/gm8-probe/results/opengmk/literals.txt");

/// The probe's `out("lit_...", <literal>);` and `out("hex_...", $<digits>);` lines: name and literal.
fn literals() -> Vec<(&'static str, &'static str)> {
    PROBE
        .lines()
        .filter_map(|l| {
            let rest = l.strip_prefix("out(\"")?;
            let (name, rest) = rest.split_once("\", ")?;
            let literal = rest.strip_suffix(");")?;
            (name.starts_with("lit_") || name.starts_with("hex_")).then_some((name, literal))
        })
        .collect()
}

/// A result file's lines as name -> IEEE-754 bits.
fn bits(results: &str) -> BTreeMap<&str, u64> {
    results
        .lines()
        .filter_map(|l| {
            let (name, value) = l.split_once(" = ")?;
            let hex = value.split(' ').next()?;
            Some((name, u64::from_str_radix(hex, 16).ok()?))
        })
        .collect()
}

#[test]
fn literals_read_as_on_gm8_and_the_oracle() {
    let literals = literals();
    assert!(literals.len() > 180, "only {} literals", literals.len());
    for (runner, results) in [("GM8", bits(GM8)), ("the oracle", bits(OPENGMK))] {
        let mut wrong = Vec::new();
        for &(name, literal) in &literals {
            let toks: Vec<Tok> = tokenize(literal)
                .unwrap()
                .into_iter()
                .map(|t| t.tok)
                .collect();
            let [Tok::Real(v), Tok::Eof] = toks[..] else {
                panic!("{name}: {literal} isn't one number");
            };
            let want = *results
                .get(name)
                .unwrap_or_else(|| panic!("{runner} has no {name}"));
            if v.to_bits() != want {
                wrong.push(format!(
                    "{name}: port {:016x}, {runner} {want:016x}",
                    v.to_bits()
                ));
            }
        }
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }
}
