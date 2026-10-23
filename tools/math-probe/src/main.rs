//! Native entry point. With `PROBE_DUMP=path`, disagreements are written there for `check_reference.py`;
//! adding `PROBE_DUMP_ALL_HYPOT=1` writes every `hypot` evaluation too.

use std::io::Write as _;

fn main() {
    let mut dump = std::env::var_os("PROBE_DUMP").map(|path| {
        std::io::BufWriter::new(std::fs::File::create(path).expect("can't create PROBE_DUMP"))
    });
    let dump_all_hypot = std::env::var_os("PROBE_DUMP_ALL_HYPOT").is_some_and(|v| v == "1");
    print!(
        "{}",
        math_probe::run(
            dump.as_mut().map(|d| d as &mut dyn std::io::Write),
            dump_all_hypot
        )
    );
    if let Some(mut dump) = dump {
        dump.flush().expect("can't write PROBE_DUMP");
    }
}
