//! The port's fast fingerprint path (`gm::FrameEncoder`, which encodes the world straight away) against the
//! one the oracle's side takes (`trace::canon::encode_frame` of the frame `trace_frame` builds), on the real
//! game: the same bytes every frame, so the same digests. `fidelity-check` matching every committed
//! fingerprint shows it for those runs; this keeps it so for whatever a change to either side does.

mod common;

use common::{assets, replay};
use game::replay::Playback;
use harness::fingerprint::Fingerprinter;

#[test]
fn encoding_the_world_is_encoding_its_frame() {
    // The level editor (text in signs and file names), a Mines level and the Temple.
    for name in [
        "editor-sign-no-exit.txt",
        "mines-seed-1.txt",
        "temple-seed-8.txt",
    ] {
        let replay = replay(name);
        let mut p = Playback::boot(assets(), &replay).unwrap();
        let (mut fast, mut slow) = (Fingerprinter::new(), Fingerprinter::new());
        let (mut a, mut b) = (Vec::new(), Vec::new());
        let mut encoder = gm::FrameEncoder::new();
        while let Some(frame) = p.step().unwrap() {
            let (world, built) = (p.game().world(), p.game().trace_with(true));
            a.clear();
            b.clear();
            encoder.encode(world, &mut a);
            trace::canon::encode_frame(&mut b, &built);
            if a != b {
                let at = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
                let around = |v: &[u8]| v[at.saturating_sub(32)..(at + 32).min(v.len())].to_vec();
                panic!(
                    "{name}: frame {frame} encodes differently from byte {at} ({} and {} bytes): \
                     the world's {:?}, the frame's {:?}",
                    a.len(),
                    b.len(),
                    String::from_utf8_lossy(&around(&a)),
                    String::from_utf8_lossy(&around(&b)),
                );
            }
            fast.world(world);
            slow.frame(&built);
        }
        assert_eq!(
            fast.finish(&replay, None),
            slow.finish(&replay, None),
            "{name}"
        );
    }
}
