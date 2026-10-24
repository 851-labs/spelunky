use super::wide::{self, Wide};
use super::{F80, PI66, Precision, TWO_OVER_PI66};

/// A deterministic stream of arbitrary f64 bit patterns (an LCG's high bits).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        let mut out = 0;
        for _ in 0..2 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            out = (out << 32) | (self.0 >> 32);
        }
        out
    }

    /// A finite double with any sign, exponent within `exp` of 1 and any significand.
    fn double(&mut self, exp: i64) -> f64 {
        let b = self.next();
        let e = (1023 + (b >> 52) as i64 % (2 * exp + 1) - exp) as u64;
        f64::from_bits(b & (1 << 63 | ((1 << 52) - 1)) | e << 52)
    }
}

fn f80(x: f64) -> F80 {
    F80::from_f64(x).unwrap()
}

#[test]
fn loads_and_stores_doubles_exactly() {
    let mut r = Lcg(1);
    for _ in 0..100_000 {
        let x = f64::from_bits(r.next());
        if x.is_finite() {
            assert_eq!(f80(x).to_f64().to_bits(), x.to_bits(), "{x:e}");
        } else {
            assert!(F80::from_f64(x).is_none());
        }
    }
    for x in [
        0.0,
        -0.0,
        f64::MIN_POSITIVE,
        5e-324,
        -5e-324,
        f64::MAX,
        1.0,
        -2.5,
    ] {
        assert_eq!(f80(x).to_f64().to_bits(), x.to_bits());
    }
}

/// At 53 bits, the x87's arithmetic on doubles is IEEE double arithmetic, wherever the result is a normal
/// double (below that, the x87 rounds to 53 bits first and again on the store).
#[test]
fn double_precision_is_ieee_double_arithmetic() {
    let mut r = Lcg(2);
    let mut checked = 0;
    for i in 0..400_000 {
        let (a, b) = if i % 4 == 0 {
            // Close magnitudes, for cancellation in sums.
            let a = r.double(3);
            (a, -a * (1.0 + r.double(0) / 64.0))
        } else {
            (r.double(300), r.double(300))
        };
        let p = Precision::Double;
        let cases = [
            (a * b, f80(a).mul(f80(b), p)),
            (a / b, f80(a).div(f80(b), p)),
            (a + b, f80(a).add(f80(b), p)),
            (a - b, f80(a).sub(f80(b), p)),
        ];
        for (want, got) in cases {
            if want.abs() >= f64::MIN_POSITIVE && want.is_finite() {
                assert_eq!(
                    got.to_f64().to_bits(),
                    want.to_bits(),
                    "{a:e} {b:e} -> {want:e}"
                );
                checked += 1;
            }
        }
    }
    assert!(checked > 1_000_000);
}

/// At 24 bits, the same for f32 operands.
#[test]
fn single_precision_is_ieee_single_arithmetic() {
    let mut r = Lcg(3);
    for _ in 0..100_000 {
        let (a, b) = (r.double(60) as f32, r.double(60) as f32);
        let p = Precision::Single;
        let (fa, fb) = (f80(a.into()), f80(b.into()));
        for (want, got) in [
            (a * b, fa.mul(fb, p)),
            (a / b, fa.div(fb, p)),
            (a + b, fa.add(fb, p)),
        ] {
            if want.abs() >= f32::MIN_POSITIVE && want.is_finite() {
                assert_eq!(got.to_f64(), f64::from(want), "{a:e} {b:e}");
            }
        }
    }
}

/// At 64 bits, against exact integer arithmetic: products and quotients of whole numbers.
#[test]
fn extended_precision_rounds_to_64_bits() {
    let p = Precision::Extended;
    // (2^32 + 1)^2 = 2^64 + 2^33 + 1 needs 65 bits: the last is a tie, to even.
    let a = F80::from_u64((1 << 32) + 1);
    let two = F80::from_u64(2);
    assert_eq!(
        a.mul(a, p),
        F80::from_u64(1 << 63)
            .mul(two, p)
            .add(F80::from_u64(1 << 33), p)
    );
    // 2^64 - 1/2 ties between 2^64 - 1 and 2^64, to the even 2^64; 2^64 - 3/2 to 2^64 - 2.
    let half = F80::ONE.scale(-1);
    let m = F80::from_u64(u64::MAX);
    assert_eq!(m.add(half, p), F80::ONE.scale(64));
    assert_eq!(m.sub(F80::ONE, p).add(half, p), F80::from_u64(u64::MAX - 1));
    // 1/3: 0xAAAA...AAAB * 2^-65.
    let third = F80::ONE.div(F80::from_u64(3), p);
    assert_eq!((third.exp, third.sig), (-2, 0xaaaa_aaaa_aaaa_aaab));
    // Quotients of 64-bit whole numbers: q = sig * 2^(exp - 63) is the nearest, |x - q y| <= ulp(q) y / 2.
    let mut r = Lcg(4);
    for _ in 0..100_000 {
        let (x, y) = (r.next() | 1 << 63, r.next() | 1 << 63);
        let q = F80::from_u64(x).div(F80::from_u64(y), p);
        let err = ((x as u128) << (63 - q.exp)).abs_diff(q.sig as u128 * y as u128);
        assert!(err <= (y / 2) as u128, "{x} / {y}");
    }
}

#[test]
fn stores_subnormals_and_overflow() {
    let p = Precision::Double;
    let tiny = f80(f64::MIN_POSITIVE);
    // 2^-1022 * 0.75 is a subnormal, exact.
    assert_eq!(tiny.mul(f80(0.75), p).to_f64(), f64::MIN_POSITIVE * 0.75);
    // Half the smallest subnormal ties to even (0); a little more rounds up.
    let half = f80(5e-324).scale(-1);
    assert_eq!(half.to_f64().to_bits(), 0);
    assert_eq!(half.mul(f80(1.0000001), p).to_f64(), 5e-324);
    assert_eq!(f80(f64::MAX).mul(f80(2.0), p).to_f64(), f64::INFINITY);
    assert_eq!(
        f80(f64::MAX).neg().mul(f80(2.0), p).to_f64(),
        f64::NEG_INFINITY
    );
}

#[test]
fn rounds_to_whole_numbers() {
    for (x, want) in [
        (0.5, 0.0),
        (1.5, 2.0),
        (2.5, 2.0),
        (-0.7, -1.0),
        (0.2, 0.0),
        (1e300, 1e300),
        (-3.0, -3.0),
    ] {
        let r = f80(x).round_int();
        assert_eq!(r.to_f64(), want, "{x}");
        assert_eq!(r.to_i64(), (want.abs() < 1e18).then_some(want as i64));
    }
    assert_eq!(f80(2.5).to_i64(), None);
}

#[test]
fn constants_are_the_x87s() {
    assert_eq!(Wide::PI.to_f80(), F80::PI);
    assert_eq!(Wide::LN2.to_f80(), F80::LN2);
    assert_eq!(Wide::ONE.div(Wide::LN2).to_f80(), F80::LOG2E);
    // Intel's 66-bit pi is the true pi's first 66 bits.
    assert_eq!(PI66, Wide::PI.m >> 60);
    // 2 / pi66, to the last bit or two.
    let two_over = Wide::from_int(false, 1, 67).div(Wide::from_int(false, PI66, 0));
    assert_eq!(two_over.exp, TWO_OVER_PI66.exp);
    assert!(two_over.m.abs_diff(TWO_OVER_PI66.m) <= 2);
}

/// The 128-bit constants, derived again from the series: tan(atan(j/8)) is j/8, e^ln2 is 2, and atan(1) is
/// pi/4.
#[test]
fn wide_constants_check_out() {
    let close = |a: Wide, b: Wide| {
        let d = a.sub(b);
        assert!(d.is_zero() || d.exp < b.exp - 120, "{a:?} vs {b:?}");
    };
    for (j, a) in wide::ATAN_EIGHTHS.iter().enumerate() {
        let t = wide::sin_series(*a).div(wide::cos_series(*a));
        close(t, Wide::from_int(false, j as u128 + 1, -3));
        close(wide::atan01(Wide::from_int(false, j as u128 + 1, -3)), *a);
    }
    close(wide::expm1_series(Wide::LN2), Wide::ONE);
    close(
        Wide {
            exp: 1,
            ..wide::ATAN_EIGHTHS[7]
        },
        Wide::PI,
    );
    close(wide::ln(Wide::from_int(false, 2, 0)), Wide::LN2);
}

/// What each instruction gives for a hundred arguments, against mpmath's exact result rounded to 64 bits
/// (`fsin`, `fcos` and `fptan` reducing by Intel's pi): `VECTORS`, made by
/// `tools/gm8-probe/x87/vectors.py 120 1`, from the model in `ulps.py`.
#[test]
fn instructions_match_the_exact_model() {
    assert!(check_vectors(VECTORS) > 100);
}

/// The same for a file of them: `X87_VECTORS=file cargo test --release -- --ignored`. 851-3257 checked the
/// 199,907 lines of `vectors.py 200000 2`.
#[test]
#[ignore]
fn instructions_match_the_exact_model_at_scale() {
    let path = std::env::var("X87_VECTORS").expect("X87_VECTORS names a vectors.py output");
    let n = check_vectors(&std::fs::read_to_string(path).unwrap());
    println!("{n} vectors");
}

/// Checks lines `fn x y neg exp sig` (x and y doubles' bits in hex), returning how many.
fn check_vectors(text: &str) -> usize {
    let mut n = 0;
    for line in text.lines() {
        let f: Vec<&str> = line.split(' ').collect();
        let x = f64::from_bits(u64::from_str_radix(f[1], 16).unwrap());
        let y = f64::from_bits(u64::from_str_radix(f[2], 16).unwrap());
        let got = match f[0] {
            "sin" => f80(x).sin(),
            "cos" => f80(x).cos(),
            "tan" => f80(x).tan(),
            "atan2" => F80::atan2(f80(x), f80(y)),
            "ln" => f80(x).ln().unwrap(),
            "exp2m1" => f80(x).exp2_m1().unwrap(),
            other => panic!("{other}"),
        };
        let sig = u64::from_str_radix(f[5], 16).unwrap();
        let exp = if sig == 0 { 0 } else { f[4].parse().unwrap() };
        assert_eq!(
            got,
            F80 {
                neg: f[3] == "1",
                exp,
                sig
            },
            "{line}"
        );
        n += 1;
    }
    n
}

#[test]
fn sin_cos_is_sin_and_cos() {
    let mut r = Lcg(5);
    for _ in 0..20_000 {
        let x = f80(r.double(70));
        assert_eq!(x.sin_cos(), (x.sin(), x.cos()), "{x:?}");
    }
    for x in [0.0, -0.0, 1e22, -1e300] {
        assert_eq!(f80(x).sin_cos(), (f80(x).sin(), f80(x).cos()));
    }
}

#[test]
fn motion_with_an_infinite_speed() {
    use super::gm8;
    // inf * sin(0) is the x87's invalid operation: NaN.
    let (h, v) = gm8::motion(0.0, f64::INFINITY);
    assert!(h == f64::INFINITY && v.is_nan());
    assert_eq!(gm8::motion(180.0, f64::INFINITY).0, f64::NEG_INFINITY);
    assert!(gm8::motion(f64::INFINITY, 1.0).0.is_nan());
    assert_eq!(gm8::lengthdir_x(f64::INFINITY, 0.0), f64::INFINITY);
}

#[test]
fn special_operands() {
    let p = F80::PI;
    // fsin beyond 2^63 leaves its operand (GM8 on Intel: sin(1e22) is 1e22); Delphi's Tan then pops it.
    assert_eq!(f80(1e22).sin().to_f64(), 1e22);
    assert_eq!(f80(1e22).tan().to_f64(), 1e22);
    assert!(super::gm8::tan(1e22).is_nan() && super::gm8::tan(-1e22).is_nan());
    assert_eq!(super::gm8::tan(9.2e18), f80(9.2e18).tan().to_f64());
    assert_eq!(f80(-0.0).sin().to_f64().to_bits(), (-0.0f64).to_bits());
    assert_eq!(f80(0.0).cos(), F80::ONE);
    // atan2's zeros and axes.
    assert_eq!(F80::atan2(f80(0.0), f80(-1.0)), p);
    assert_eq!(F80::atan2(f80(-0.0), f80(-0.0)), p.neg());
    assert_eq!(
        F80::atan2(f80(-0.0), f80(0.0)).to_f64().to_bits(),
        (-0.0f64).to_bits()
    );
    assert_eq!(F80::atan2(f80(-3.0), f80(0.0)), p.scale(-1).neg());
    assert_eq!(F80::atan2(f80(5.0), f80(5.0)), Wide::PI.to_f80().scale(-2));
    assert_eq!(f80(1.0).ln(), Some(F80::ZERO));
    assert_eq!(f80(0.0).ln(), None);
    assert_eq!(f80(-1.0).ln(), None);
    assert_eq!(f80(1.5).exp2_m1(), None);
    assert_eq!(f80(-1.0).exp2_m1().map(F80::to_f64), Some(-0.5));
}

const VECTORS: &str = "\
sin bfdc1ad740be7bf7 0000000000000000 1 -2 d9ae85a99a626ff1\n\
cos 3f1cbc71d20a9896 0000000000000000 0 -1 ffffffe631e315e4\n\
tan c194867ef4b95b40 0000000000000000 1 0 81b94f2ccb4cbbfd\n\
atan2 bde6c3ed285fee3a 41e6173446d4ca11 1 -64 83e8ce444d22dfff\n\
ln 3fefae6cd074a691 0000000000000000 1 -7 a3f7b6ee859392ae\n\
exp2m1 3fdc5b6c7cd447e0 0000000000000000 0 -2 b810de0a8e1d5b27\n\
sin 40119ee9a6621b69 0000000000000000 1 -1 f403d9c920f8ee4d\n\
cos be1d2507eb0e06af 0000000000000000 0 -1 ffffffffffffffe5\n\
tan 400b3a259b49db84 0000000000000000 0 -2 8930a2f4f66a6f51\n\
atan2 40de3f2b2737aa58 c110c64d7288b351 0 1 c1e135e7589e3ab3\n\
ln 3ff004548bb0777f 0000000000000000 0 -10 8a7eb926ded92c8a\n\
exp2m1 3fd6a2f913802088 0000000000000000 0 -2 8e3fde6f8e319382\n\
sin 4137eeb9b3fe6dde 0000000000000000 0 0 8000000000000000\n\
cos be1a8cd8e37899ea 0000000000000000 0 -1 ffffffffffffffea\n\
tan 402ad7d149e321d1 0000000000000000 0 0 933a737c3fe30d64\n\
atan2 3e097a717cbe4dc8 41e909a857348f2a 0 -62 824097c56591cb54\n\
ln 3fefcd2f5ad30353 0000000000000000 1 -8 cbe4a30333d81193\n\
exp2m1 3fee4d071f0f5126 0000000000000000 0 -1 ed7fb25464ad5192\n\
sin 43a567681a48e01c 0000000000000000 1 -1 fef075b3d1e4b819\n\
cos c050606e63a0d730 0000000000000000 1 -1 e49f350a0f2f1697\n\
tan 4124525df2e861aa 0000000000000000 1 36 e8115a52206c25ad\n\
atan2 4041000000000000 402e000000000000 0 0 93e102503964d686\n\
ln 3d68ffbb4225a5c7 0000000000000000 1 4 dfc845bbffd9c280\n\
exp2m1 3fddcb4520792398 0000000000000000 0 -2 c2fba2c961a53cb0\n\
sin 415207dbbedb5a20 0000000000000000 1 -2 84df8ee0b9d3a637\n\
cos c0ce7d1d54f95725 0000000000000000 1 -1 f0f014eaff1733b1\n\
tan 3ffc7bd892b3aa92 0000000000000000 1 2 968c54717f9489e1\n\
atan2 c021fe4f6df282e7 3f747bc5d82497aa 1 0 c8fda3cae537e4b3\n\
ln 3fa1193f09a00a36 0000000000000000 1 1 d98e92bba542ae72\n\
exp2m1 3fcbe3cedea0b250 0000000000000000 0 -3 a6f19e9574b71a14\n\
sin 4123494eb06b0d27 0000000000000000 0 -36 a02fa6e000000000\n\
cos 41349efe0daacb85 0000000000000000 1 0 8000000000000000\n\
tan 40124a824a208ce9 0000000000000000 0 2 e3b11bd812d82e89\n\
atan2 c17a8f66cfba65cf bf90ff5f0662c89a 1 0 c90fdaa369121980\n\
ln 405712b9bedab711 0000000000000000 0 2 90cc80ef0c4a2c93\n\
exp2m1 bfd254b4d95e8468 0000000000000000 1 -3 b8632f9df849fd93\n\
sin 3f33cfca05f75f1c 0000000000000000 0 -12 9e7e50073acb83ea\n\
cos c04911314c8c347f 0000000000000000 0 -1 fdcd0dfce0c0a811\n\
tan c235b2e5350aca14 0000000000000000 1 -2 b11fb0575a6ff6d7\n\
atan2 c048000000000000 401c000000000000 1 0 b686a35a22e81e71\n\
ln 41d976e6a1f75615 0000000000000000 0 4 aa12a86e415bb52e\n\
exp2m1 3fe306c30cff983c 0000000000000000 0 -1 8291342bc49ea787\n\
sin bead3c610d129236 0000000000000000 1 -21 e9e308689471265c\n\
cos bf2e810fe4f9ef9f 0000000000000000 0 -1 ffffff8bafe6b2c4\n\
tan 42921d8b35bdb48d 0000000000000000 0 -1 ee5ff5c87b0f1a9b\n\
atan2 bff0000000000000 c01c000000000000 1 1 bffb033fe64c53f1\n\
ln 3d97aa6182a8cea9 0000000000000000 1 4 cf95d5850e7d87bf\n\
exp2m1 3fdd75a0d8296f5c 0000000000000000 0 -2 c06d11efb749f0bd\n\
sin 4015a45f88900a63 0000000000000000 1 -1 c41b7d167a81cec4\n\
cos c3c6029930d188ea 0000000000000000 0 -1 f6686b6bf1b3ef0b\n\
tan be5f8d88a983a1ed 0000000000000000 1 -26 fc6c454c1d0f7c74\n\
atan2 3fa92ad754e51cee c027daaeae2dbe5b 0 1 c8cc54d469868621\n\
ln 416533a199be3b5b 0000000000000000 0 4 81ca800fbe6fe568\n\
exp2m1 3fd30242207dbf90 0000000000000000 0 -3 ea15922a9fa05ae1\n\
sin 3f47e5a75ada2e22 0000000000000000 0 -11 bf2d39ba81d914cb\n\
cos 41c1418ee21a6e7a 0000000000000000 1 -1 fdfe6fdfb052baf3\n\
tan be8289f026202e23 0000000000000000 1 -23 944f81310172217b\n\
atan2 bef6ccab5abbc91b 3efc6ccf0d5f5050 1 -1 ad0f54cde60f07eb\n\
ln 3e137799583f56f1 0000000000000000 1 4 a4c943debec66cbd\n\
exp2m1 3fe66f07604da72e 0000000000000000 0 -1 a02d4de4b54221e0\n\
sin 40fc2b6fc6a055ef 0000000000000000 1 -1 fd6e27a3121029bd\n\
cos bfe6b2f0ffa0b04d 0000000000000000 0 -1 c2400a72b0dd4fc7\n\
tan becbc6147a302dd6 0000000000000000 1 -19 de30a3d184eb5c3f\n\
atan2 4041800000000000 bff0000000000000 0 0 ccb7d3f6442cd112\n\
ln 3fef78cc002b89b6 0000000000000000 1 -6 8854cdfba3325596\n\
exp2m1 bfc9013f6335d5f0 0000000000000000 1 -3 81adbf50a2ddcb0a\n\
sin 3e64351fb3c3a40b 0000000000000000 0 -25 a1a8fd9e1d204d41\n\
cos 412c510da149bd18 0000000000000000 1 0 8000000000000000\n\
tan 41210b79d110148d 0000000000000000 1 -36 f2c1274000000000\n\
atan2 3f943dc8b53f9cb3 40cf0c297e559034 0 -20 a6e6073761935a43\n\
ln 42cd97cf2cdf99d1 0000000000000000 0 4 fe73dc3d7d99dd8f\n\
exp2m1 3fb0a6840f045950 0000000000000000 0 -5 bce29b270007aa55\n\
sin be24bac266d85aaa 0000000000000000 1 -29 a5d61336c2d54ff4\n\
cos bfff6b5f1a4e94b6 0000000000000000 1 -2 c409855b4d7f04fc\n\
tan 3ec3a49713d2f28e 0000000000000000 0 -19 9d24b89e98d03bf4\n\
atan2 3f45d60753388a2e be014dd26a0f2342 0 0 c90fe0f911c97e85\n\
ln 4356e4da49d18f5d 0000000000000000 0 5 97272647c6d1e9a0\n\
exp2m1 3fba3304314bd3e0 0000000000000000 0 -4 968e7c493aecb046\n\
sin be241f2c2c593ff7 0000000000000000 1 -29 a0f96162c9ffb7f5\n\
cos c02a06e6bf91f544 0000000000000000 0 -1 e6d5e0561f9123a6\n\
tan 412122a849012def 0000000000000000 0 34 c16cc8bd7cb69116\n\
atan2 3e0210b739da302a bdd188da3ad20cfc 0 0 d8842f936d34189f\n\
ln 3fefe9dd26ca901c 0000000000000000 1 -9 b154263be34cfc3f\n\
exp2m1 3fb18f01fb344d10 0000000000000000 0 -5 c76f3ec34a0b5ed9\n\
sin bff677e39534dd0f 0000000000000000 1 -1 fc7567d114baa8e4\n\
cos 4134cdd8a775a39e 0000000000000000 1 0 8000000000000000\n\
tan bf5d24498a068a54 0000000000000000 1 -10 e9225c6cf07d764f\n\
atan2 4049000000000000 4039000000000000 0 0 8db70c975df22363\n\
ln 3d0d2e762a39332d 0000000000000000 1 4 ffd0d67a83deb055\n\
exp2m1 bfe5d3d95a0c465e 0000000000000000 1 -2 c0e50001c817056f\n\
sin 4125dee51736e14d 0000000000000000 1 0 8000000000000000\n\
cos 400b0487c762e3ae 0000000000000000 1 -1 f8ed3c2fb8de6df9\n\
tan 412b4a598e5c4ef9 0000000000000000 1 35 f841e8589af419d9\n\
atan2 41faaeb3d988b8cd 40302fc40df6db4e 0 0 c90fda9d470392a2\n\
ln 3e91078abdd1910a 0000000000000000 1 3 f2fd77d63ff3ae9d\n\
exp2m1 3feee2a66f67a2e6 0000000000000000 0 -1 f3c944546fd71860\n\
sin 3f5379154cbd85dc 0000000000000000 0 -10 9bc8a7fe94cfcab1\n\
cos bee77088fb932d33 0000000000000000 0 -1 ffffffffbb52bd55\n\
tan 401577b157fa3bda 0000000000000000 1 0 a6d016c9d5cf35a1\n\
atan2 c247fab65e3b0bda 3eb00e4b596ba6ed 1 0 c90fdaa22168c20a\n\
ln 403fd43bb0f56cbc 0000000000000000 0 1 dd76d9449b5adc6a\n\
exp2m1 bfd1dc544eb2b6ac 0000000000000000 1 -3 b419c2eca1593e93\n\
sin 4101fa6257a99aa0 0000000000000000 1 0 8000000000000000\n\
cos 400ff9745e67cd47 0000000000000000 1 -1 a7f37c09c27ebbe7\n\
tan 3f4712994e53d90e 0000000000000000 0 -11 b894cc7265e156b2\n\
atan2 422324aa35a54b75 c244150c8ffeab3d 0 1 ba16de17275ba325\n\
ln 428936de3b7eb84b 0000000000000000 0 4 e6fda9516c247e61\n\
exp2m1 bfe98a12fba56d26 0000000000000000 1 -2 d98bf1ae47bffe03\n\
sin c327aeab523de49f 0000000000000000 0 -2 b0e4dfcb97c76f68\n\
cos be19dc7731d8bc42 0000000000000000 0 -1 ffffffffffffffeb\n\
tan c12ba52e0669c6ea 0000000000000000 1 -1 f8aa07cd7f59bd62\n\
atan2 c114296156a7c2ca 409c74439245a945 1 0 c85b3592892cf332\n\
ln 3fb3294ec056853b 0000000000000000 1 1 a5e7f1ef50f41ffe\n\
exp2m1 3fe1c696c65c204e 0000000000000000 0 -2 f0788754449dc3ec\n\
sin bf23e81c9487ec35 0000000000000000 1 -13 9f40e499f9ddd221\n\
cos be60f8ff9c3b110f 0000000000000000 0 -1 ffffffffffffdbfe\n\
tan 412012f890bf2172 0000000000000000 1 -35 b4d08ec400000000\n\
atan2 4045000000000000 c047000000000000 0 1 99b41b047efe124f\n\
ln 3ff01a4c7bd60c31 0000000000000000 0 -8 d1b7b33f63047eab\n\
exp2m1 3fd60a983e83b3a8 0000000000000000 0 -2 8a0b9eb1a65509be\n\
";
