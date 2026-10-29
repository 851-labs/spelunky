//! The canonical encoding of a trace frame, which replay fingerprints hash (docs/formats.md,
//! "Fingerprints"): fixed-width little-endian numbers, length-prefixed strings and lists, a tag before each
//! optional value. It makes equal what `trace-diff` counts as equal: a missing alarm is -1, a scalar is
//! element 0 of an array, an empty array is no variable, every NaN is the same NaN, and instances are in id
//! order. The formatting of the JSON a frame was read from never matters.
//!
//! [`encode_frame`] encodes a parsed [`Frame`]. The port also writes these bytes straight from its world
//! (`gm::FrameEncoder`), without building the frame, and has to stay byte for byte the same as this.

use std::collections::BTreeMap;

use crate::trace::ALARMS;
use crate::{Frame, Instance, Real, Scalar, Value};

/// Appends the canonical encoding's parts to a buffer.
pub struct Encoder<'a>(pub &'a mut Vec<u8>);

impl Encoder<'_> {
    #[inline]
    pub fn u64(&mut self, v: u64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }

    #[inline]
    pub fn i32(&mut self, v: i32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }

    #[inline]
    pub fn len(&mut self, n: usize) {
        self.u64(n as u64);
    }

    /// A length to be [`set_len`](Self::set_len) once it's known: where it is.
    #[inline]
    pub fn len_later(&mut self) -> usize {
        let at = self.0.len();
        self.len(0);
        at
    }

    #[inline]
    pub fn set_len(&mut self, at: usize, n: usize) {
        self.0[at..at + 8].copy_from_slice(&(n as u64).to_le_bytes());
    }

    #[inline]
    pub fn bytes(&mut self, b: &[u8]) {
        self.len(b.len());
        self.0.extend_from_slice(b);
    }

    #[inline]
    pub fn str(&mut self, s: &str) {
        self.bytes(s.as_bytes());
    }

    /// Bit for bit, as `Real`'s equality, except that every NaN is the same.
    #[inline]
    pub fn real(&mut self, r: f64) {
        let r = if r.is_nan() { f64::NAN } else { r };
        self.u64(r.to_bits());
    }

    #[inline]
    pub fn bool(&mut self, b: bool) {
        self.0.push(u8::from(b));
    }

    /// The tag before an optional value: whether it's there.
    #[inline]
    pub fn some(&mut self, there: bool) {
        self.0.push(u8::from(there));
    }

    #[inline]
    pub fn opt<T>(&mut self, v: Option<T>, f: impl FnOnce(&mut Self, T)) {
        self.some(v.is_some());
        if let Some(v) = v {
            f(self, v);
        }
    }

    /// The tag before a scalar: 0 for a real, 1 for a string.
    #[inline]
    pub fn real_tag(&mut self) {
        self.0.push(0);
    }

    #[inline]
    pub fn str_tag(&mut self) {
        self.0.push(1);
    }

    #[inline]
    pub fn scalar(&mut self, s: &Scalar) {
        match s {
            Scalar::Real(r) => {
                self.real_tag();
                self.real(r.0);
            }
            Scalar::Str(s) => {
                self.str_tag();
                self.str(s);
            }
        }
    }

    /// Variables as `trace-diff` flattens them: each one's elements, a scalar being element 0, so an
    /// array of just element 0 is the scalar, and an empty array the same as no variable.
    pub fn vars(&mut self, vars: Option<&BTreeMap<String, Value>>) {
        self.opt(vars, |e, vars| {
            let count = |v: &Value| match v {
                Value::Array(a) => a.len(),
                _ => 1,
            };
            e.len(vars.values().filter(|v| count(v) > 0).count());
            for (name, value) in vars {
                if count(value) == 0 {
                    continue;
                }
                e.str(name);
                e.len(count(value));
                match value {
                    Value::Real(r) => {
                        e.u64(0);
                        e.scalar(&Scalar::Real(*r));
                    }
                    Value::Str(s) => {
                        e.u64(0);
                        e.str_tag();
                        e.str(s);
                    }
                    Value::Array(a) => {
                        for (i, s) in a {
                            e.u64(u64::from(*i));
                            e.scalar(s);
                        }
                    }
                }
            }
        });
    }

    fn instances(&mut self, list: &[Instance]) {
        // Writers sort by id; a trace that didn't is still the same set.
        let mut sorted: Vec<&Instance> = list.iter().collect();
        sorted.sort_by_key(|i| i.id);
        self.len(sorted.len());
        for i in sorted {
            self.instance(i);
        }
    }

    fn instance(&mut self, i: &Instance) {
        // Destructured so that a new field can't be left out of the fingerprint.
        let Instance {
            id,
            object_index,
            object,
            x,
            y,
            xprevious,
            yprevious,
            hspeed,
            vspeed,
            speed,
            direction,
            gravity,
            friction,
            image_index,
            image_speed,
            image_xscale,
            image_yscale,
            depth,
            sprite_index,
            sprite,
            visible,
            solid,
            alarms,
            vars,
        } = i;
        self.i32(*id);
        self.opt(*object_index, Self::i32);
        self.str(object);
        for r in [
            x,
            y,
            xprevious,
            yprevious,
            hspeed,
            vspeed,
            speed,
            direction,
            gravity,
            friction,
            image_index,
            image_speed,
            image_xscale,
            image_yscale,
            depth,
        ] {
            self.opt(*r, |e, Real(r)| e.real(r));
        }
        self.opt(*sprite_index, Self::i32);
        self.opt(sprite.as_deref(), Self::str);
        self.opt(*visible, Self::bool);
        self.opt(*solid, Self::bool);
        // A missing alarm is -1, so only the others count; `trace-diff` compares alarms 0 to 11.
        self.opt(alarms.as_ref(), |e, alarms| {
            let set = || alarms.iter().filter(|&(&k, &v)| k < ALARMS && v != -1);
            e.len(set().count());
            for (&k, &v) in set() {
                e.u64(u64::from(k));
                e.i32(v);
            }
        });
        self.vars(vars.as_ref());
    }
}

/// Appends `f`'s canonical encoding to `out`.
pub fn encode_frame(out: &mut Vec<u8>, f: &Frame) {
    let Frame {
        frame,
        room,
        room_name,
        seed,
        instance_count,
        instances,
        deactivated,
        globals,
        builtins,
    } = f;
    let e = &mut Encoder(out);
    e.u64(*frame);
    e.opt(*room, Encoder::i32);
    e.opt(room_name.as_deref(), Encoder::str);
    e.opt(*seed, Encoder::i32);
    e.opt(*instance_count, Encoder::u64);
    e.instances(instances);
    e.opt(deactivated.as_deref(), Encoder::instances);
    e.vars(globals.as_ref());
    e.vars(builtins.as_ref());
}

/// A fixed 64-bit hash: replay fingerprints' (`harness::fingerprint`), and the wiki clips' state at their
/// first frame (`gm::World::frame_hash`) and through their frames ([`hash_frames`]). It's defined here, not the platform's `Hasher`, so what's committed
/// stays the same across Rust releases and targets, native and wasm32 (gm's tests pin it, and run on both).
/// Four lanes, each starting from its own seed and the length, take turns at the 8-byte little-endian words
/// (in 32-byte blocks, the last zero-padded), mixing each in by xor, multiply and rotate; then the lanes are
/// folded together and MurmurHash3's finalizer applied. Each step is a bijection of a lane, so two inputs of
/// the same length that differ in one word always differ after it. Four lanes run four times as fast as one,
/// which matters at hundreds of KB a frame. Not cryptographic: it only has to catch accidental differences.
pub fn hash(bytes: &[u8]) -> u64 {
    const K: u64 = 0x9e37_79b9_7f4a_7c15;
    let mix = |h: u64, w: u64| (h ^ w).wrapping_mul(K).rotate_left(31);
    let fmix = |mut h: u64| {
        h ^= h >> 33;
        h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
        h ^= h >> 33;
        h = h.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
        h ^ (h >> 33)
    };
    let len = bytes.len() as u64;
    let mut lanes = [
        0x243f_6a88_85a3_08d3,
        0x1319_8a2e_0370_7344,
        0xa409_3822_299f_31d0,
        0x082e_fa98_ec4e_6c89,
    ]
    .map(|seed| mix(seed, len));
    let mut block = |b: &[u8; 32]| {
        let (words, _) = b.as_chunks::<8>();
        for (lane, w) in lanes.iter_mut().zip(words) {
            *lane = mix(*lane, u64::from_le_bytes(*w));
        }
    };
    let (blocks, rest) = bytes.as_chunks::<32>();
    for b in blocks {
        block(b);
    }
    if !rest.is_empty() {
        let mut b = [0; 32];
        b[..rest.len()].copy_from_slice(rest);
        block(&b);
    }
    fmix(lanes.iter().fold(len, |h, &lane| mix(h, fmix(lane))))
}

/// A run of frames' states as one [`hash`]: each frame's own (`gm::World::frame_hash`), in order, as 8-byte
/// little-endian words. The wiki clips' state through their frames, first to last, which the web build's embed
/// checks before showing a clip (docs/wiki-media.md, "The states through the clip").
pub fn hash_frames(hashes: &[u64]) -> u64 {
    let bytes: Vec<u8> = hashes.iter().flat_map(|h| h.to_le_bytes()).collect();
    hash(&bytes)
}
