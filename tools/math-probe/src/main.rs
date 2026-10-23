//! Native entry point. With `PROBE_DUMP=path`, disagreements are written there for `check_reference.py`.

use std::io::Write as _;

fn main() {
    let mut dump = std::env::var_os("PROBE_DUMP").map(|path| {
        std::io::BufWriter::new(std::fs::File::create(path).expect("can't create PROBE_DUMP"))
    });
    print!(
        "{}",
        math_probe::run(dump.as_mut().map(|d| d as &mut dyn std::io::Write))
    );
    if let Some(mut dump) = dump {
        dump.flush().expect("can't write PROBE_DUMP");
    }
}
