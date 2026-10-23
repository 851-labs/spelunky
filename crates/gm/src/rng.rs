//! GM8's random number generator, bit-exact (851-3124).
//!
//! The GM8 runner is written in Delphi and uses Delphi's `System.Random`: a 32-bit linear congruential
//! generator `seed = seed * 0x08088405 + 1 (mod 2^32)` whose float output is `seed / 2^32`, read as an
//! unsigned 32-bit integer. Confirmed against OpenGMK `gm8emulator/src/gml/rand.rs` (constants `MULTIPLIER`,
//! `INCREMENT`, `INT_STEP`) and its `random` kernel function in `gm8emulator/src/gml/kernel.rs`, and checked
//! against per-frame seeds from the oracle (see the tests).
//!
//! The whole state is one `i32`, so the generator is `Copy` and savestates are free.

/// Delphi's LCG multiplier.
const MULTIPLIER: u32 = 0x0808_8405;
/// Delphi's LCG increment.
const INCREMENT: u32 = 1;
/// 2^-32: maps the 32-bit state onto [0, 1). Exactly representable, so the scaling never rounds.
const INV_2_POW_32: f64 = 1.0 / 4_294_967_296.0;

/// The runner's single, global random stream. GM8 has exactly one; level generation and cosmetic effects
/// share it, so call order matters for replays.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rng {
    /// `random_get_seed()` exposes the state as a signed 32-bit integer, so store it that way.
    seed: i32,
}

impl Rng {
    /// A generator in a known state. This is the replay API: seed it with the start seed the oracle recorded.
    #[inline]
    pub const fn with_seed(seed: i32) -> Self {
        Self { seed }
    }

    /// The state the GM8 runner is in when the first room starts, given the entropy `randomize` would
    /// have drawn.
    ///
    /// The runner randomizes at startup, then names its temp directory `gm_ttt_<n>` with one
    /// `Random(100000)` draw (another per name that already exists), before any game code runs.
    /// OpenGMK `gm8emulator/src/game.rs` (`Game::launch`, `make_temp_dir`) reproduces this and records the
    /// replay start seed *after* it, so a replay should use [`Rng::with_seed`] with the recorded seed instead.
    pub fn gm8_startup(entropy: i32) -> Self {
        let mut rng = Self::with_seed(entropy);
        rng.random_int(100_000);
        rng
    }

    /// GML `random_get_seed()`.
    #[inline]
    pub const fn seed(&self) -> i32 {
        self.seed
    }

    /// GML `random_set_seed(seed)`. The GML argument is rounded to an `i32` first (see
    /// [`crate::Value::to_int`]); GM8 doesn't step the generator here.
    #[inline]
    pub fn set_seed(&mut self, seed: i32) {
        self.seed = seed;
    }

    /// GML `randomize()`. GM8 reseeds from the clock; the caller supplies that entropy so the runtime stays
    /// deterministic.
    #[inline]
    pub fn randomize(&mut self, entropy: i32) {
        self.seed = entropy;
    }

    /// Advances the state one step and returns it as an unsigned 32-bit integer.
    #[inline]
    fn step(&mut self) -> u32 {
        let next = (self.seed as u32)
            .wrapping_mul(MULTIPLIER)
            .wrapping_add(INCREMENT);
        self.seed = next as i32;
        next
    }

    /// Advances the state `n` steps without producing values, e.g. to resynchronize with a trace.
    pub fn skip(&mut self, n: u64) {
        for _ in 0..n {
            self.step();
        }
    }

    /// GML `random(x)`: a real in [0, x) (or (x, 0] for negative `x`), one step per call.
    ///
    /// Computed as `(state / 2^32) * x`. The first product is exact, so the result is the correctly
    /// rounded `state * x / 2^32`; for integer `x` below 2^21 it is exact, which makes
    /// `floor(random(n))` (the game's `rand` script) free of rounding.
    #[inline]
    pub fn random(&mut self, x: f64) -> f64 {
        f64::from(self.step()) * INV_2_POW_32 * x
    }

    /// Delphi's `Random(range)`: an integer in [0, range), one step per call, computed as
    /// `(state * range) >> 32`. GML's `irandom(n)` is `Random(n + 1)`. The game never calls `irandom`, but
    /// the runner does at startup (see [`Rng::gm8_startup`]).
    #[inline]
    pub fn random_int(&mut self, range: u32) -> u32 {
        ((u64::from(self.step()) * u64::from(range)) >> 32) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The recurrence written independently with 64-bit arithmetic, as a cross-check on `step`.
    fn reference_next(seed: i64) -> i64 {
        let next = (seed.rem_euclid(1 << 32) * 0x0808_8405 + 1).rem_euclid(1 << 32);
        if next >= 1 << 31 {
            next - (1 << 32)
        } else {
            next
        }
    }

    #[test]
    fn first_steps_from_zero() {
        // Derived by hand from the documented algorithm, not from a running game: 0 -> 1 -> 0x08088406.
        let mut rng = Rng::with_seed(0);
        assert_eq!(rng.random(1.0), 1.0 / 4_294_967_296.0);
        assert_eq!(rng.seed(), 1);
        assert_eq!(rng.random(1.0), 134_775_814.0 / 4_294_967_296.0);
        assert_eq!(rng.seed(), 0x0808_8406);
    }

    #[test]
    fn matches_reference_recurrence_and_wraps_signed() {
        let mut rng = Rng::with_seed(12345);
        let mut seed = 12345i64;
        let mut saw_negative = false;
        for _ in 0..10_000 {
            seed = reference_next(seed);
            rng.random(1.0);
            assert_eq!(i64::from(rng.seed()), seed);
            saw_negative |= seed < 0;
        }
        assert!(
            saw_negative,
            "random_get_seed must expose the state as a signed i32"
        );
    }

    #[test]
    fn random_scales_unsigned_state() {
        // A state with the top bit set is a negative seed but a value above 0.5.
        let mut rng = Rng::with_seed(-1); // 0xFFFFFFFF * 0x08088405 + 1 = 0xF7F77BFC (mod 2^32)
        let r = rng.random(10.0);
        assert_eq!(rng.seed() as u32, 0xF7F7_7BFC);
        assert_eq!(r, f64::from(0xF7F7_7BFCu32) / 4_294_967_296.0 * 10.0);
        assert!(r > 9.0 && r < 10.0);
        // Negative bounds give values in (x, 0].
        let mut rng = Rng::with_seed(-1);
        assert_eq!(rng.random(-10.0), -r);
    }

    #[test]
    fn random_is_in_range_and_rand_script_is_inclusive() {
        // `rand(a, b)` in scripts/rand.gml is `floor(random(b - a + 1)) + a`.
        let mut rng = Rng::with_seed(42);
        let mut seen = [false; 6];
        for _ in 0..10_000 {
            let r = rng.random(6.0);
            assert!((0.0..6.0).contains(&r));
            seen[r.floor() as usize] = true;
        }
        assert!(seen.iter().all(|&s| s));
    }

    #[test]
    fn random_int_is_delphi_random_range() {
        let mut a = Rng::with_seed(777);
        let mut b = Rng::with_seed(777);
        for _ in 0..1000 {
            let n = a.random_int(100_000);
            let state = u64::from(b.step());
            assert_eq!(u64::from(n), state * 100_000 / (1 << 32));
            assert!(n < 100_000);
        }
    }

    #[test]
    fn set_seed_and_randomize_do_not_step() {
        let mut rng = Rng::with_seed(5);
        rng.set_seed(-7);
        assert_eq!(rng.seed(), -7);
        rng.randomize(99);
        assert_eq!(rng.seed(), 99);
        let mut copy = rng;
        assert_eq!(
            copy.random(1.0),
            rng.random(1.0),
            "clones replay the same stream"
        );
    }

    /// The steps between two seeds, if `to` is reachable from `from` within `max` steps.
    fn steps_between(from: i32, to: i32, max: u64) -> Option<u64> {
        let mut rng = Rng::with_seed(from);
        (0..=max).find(|_| {
            let hit = rng.seed() == to;
            rng.step();
            hit
        })
    }

    /// `n` for each of the 203 `oDark` instances in `rIntro` at frame 0, in instance-id order, from the
    /// oracle (OpenGMK running Spelunky 1.1 from replay seed 12345). Each `oDark` create event runs
    /// `n = rand(1,100)`, i.e. `floor(random(100)) + 1`, and these are the first RNG calls of the run.
    const ORACLE_ODARK_N: [u8; 203] = [
        39, 29, 8, 56, 64, 35, 44, 8, 4, 93, 98, 33, 25, 79, 52, 43, 85, 81, 78, 89, 20, 6, 71, 44,
        19, 72, 39, 29, 47, 7, 77, 13, 86, 28, 54, 84, 81, 5, 67, 80, 37, 6, 62, 21, 94, 83, 22,
        97, 67, 57, 57, 77, 43, 96, 72, 67, 58, 5, 77, 26, 9, 67, 68, 3, 65, 8, 6, 81, 87, 65, 59,
        15, 67, 98, 83, 20, 44, 57, 76, 32, 71, 2, 9, 53, 22, 47, 24, 57, 73, 51, 68, 10, 3, 1, 78,
        44, 76, 57, 54, 67, 29, 40, 84, 20, 60, 87, 98, 2, 82, 25, 71, 26, 15, 5, 8, 79, 9, 22, 67,
        96, 25, 4, 13, 70, 18, 43, 14, 65, 9, 12, 9, 98, 20, 74, 7, 8, 23, 90, 20, 24, 30, 82, 18,
        12, 10, 98, 57, 53, 25, 99, 41, 78, 13, 51, 88, 79, 60, 97, 39, 54, 42, 53, 12, 62, 34, 35,
        96, 100, 99, 18, 41, 26, 22, 92, 76, 82, 52, 59, 85, 2, 51, 80, 89, 3, 76, 75, 49, 95, 44,
        37, 63, 59, 44, 6, 30, 78, 12, 7, 19, 26, 73, 75, 26,
    ];

    #[test]
    fn oracle_rand_values_from_replay_seed() {
        let mut rng = Rng::with_seed(12345);
        for (i, &n) in ORACLE_ODARK_N.iter().enumerate() {
            assert_eq!(rng.random(100.0).floor() + 1.0, f64::from(n), "oDark #{i}");
        }
    }

    #[test]
    fn oracle_per_frame_seeds_are_reachable() {
        // (seed after frame t, seed after a later frame, steps), from the oracle's per-frame seed dump of
        // the intro, title and tutorial. Across all 1,725 consecutive frame pairs in that dump, every
        // seed is reachable from the previous one: 1,578 frames take 0 steps, 141 take 4, 4 take 3, and
        // the two room transitions take 465 and 1,766.
        let cases = [
            (12345, 1_754_002_487, 226), // replay start -> end of frame 0 (rIntro creation)
            (1_754_002_487, -1_854_218_502, 3), // frame 485 -> 486: oBatIntro's three random() calls
            (-778_021_381, 1_653_549_592, 465), // frame 813 -> 814: rIntro -> rTitle
            (-1_407_888_380, 353_348_486, 1766), // frame 1314 -> 1315: rTitle -> rTutorial
        ];
        for (from, to, steps) in cases {
            assert_eq!(
                steps_between(from, to, 10_000),
                Some(steps),
                "{from} -> {to}"
            );
        }
    }

    #[test]
    fn startup_consumes_one_draw() {
        let mut expected = Rng::with_seed(1234);
        expected.skip(1);
        assert_eq!(Rng::gm8_startup(1234), expected);
    }
}
