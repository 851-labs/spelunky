//! Little-endian cursor over a `.gmk` byte block. Every read is bounds-checked and returns an [`Error`].

use crate::MAX_INFLATED_BLOCK;
use crate::error::{Error, ErrorKind};

pub(crate) type Result<T> = std::result::Result<T, Error>;

pub(crate) struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    context: String,
}

/// Owned data for a zlib block, so that a [`Reader`] can borrow it.
pub(crate) struct Inflated {
    data: Vec<u8>,
    context: String,
}

impl Inflated {
    pub(crate) fn reader(&self) -> Reader<'_> {
        Reader::new(&self.data, self.context.clone())
    }
}

impl<'a> Reader<'a> {
    pub(crate) fn new(data: &'a [u8], context: impl Into<String>) -> Self {
        Reader {
            data,
            pos: 0,
            context: context.into(),
        }
    }

    pub(crate) fn err(&self, kind: ErrorKind) -> Error {
        Error {
            context: self.context.clone(),
            offset: self.pos,
            kind,
        }
    }

    fn err_at(&self, offset: usize, kind: ErrorKind) -> Error {
        Error {
            context: self.context.clone(),
            offset,
            kind,
        }
    }

    pub(crate) fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    pub(crate) fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        if n > self.remaining() {
            return Err(self.err(ErrorKind::UnexpectedEof {
                wanted: n,
                available: self.remaining(),
            }));
        }
        let out = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(out)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let mut out = [0; N];
        out.copy_from_slice(self.bytes(N)?);
        Ok(out)
    }

    pub(crate) fn i32(&mut self) -> Result<i32> {
        self.array().map(i32::from_le_bytes)
    }

    pub(crate) fn u32(&mut self) -> Result<u32> {
        self.array().map(u32::from_le_bytes)
    }

    pub(crate) fn u16(&mut self) -> Result<u16> {
        self.array().map(u16::from_le_bytes)
    }

    pub(crate) fn u8(&mut self) -> Result<u8> {
        self.array::<1>().map(|b| b[0])
    }

    pub(crate) fn f64(&mut self) -> Result<f64> {
        self.array().map(f64::from_le_bytes)
    }

    /// A 32-bit boolean. GM writes exactly 0 or 1; anything else means the reader is misaligned.
    pub(crate) fn bool(&mut self) -> Result<bool> {
        let start = self.pos;
        match self.i32()? {
            0 => Ok(false),
            1 => Ok(true),
            v => Err(self.err_at(start, ErrorKind::InvalidBool(v))),
        }
    }

    /// A non-negative 32-bit length or count.
    pub(crate) fn len(&mut self) -> Result<usize> {
        let start = self.pos;
        let n = self.i32()?;
        usize::try_from(n).map_err(|_| self.err_at(start, ErrorKind::NegativeLength(n)))
    }

    /// A count of items that each take at least `min_item_size` bytes. Rejecting counts the remaining
    /// data can't hold keeps a corrupt count from driving a huge allocation or a long loop.
    pub(crate) fn count(&mut self, min_item_size: usize) -> Result<usize> {
        let start = self.pos;
        let n = self.len()?;
        let available = self.remaining();
        let wanted = n.saturating_mul(min_item_size.max(1));
        if wanted > available {
            return Err(self.err_at(start, ErrorKind::UnexpectedEof { wanted, available }));
        }
        Ok(n)
    }

    pub(crate) fn blob(&mut self) -> Result<&'a [u8]> {
        let n = self.len()?;
        self.bytes(n)
    }

    /// A length-prefixed string in the Windows-1252 code page (what GM 8.0 writes).
    pub(crate) fn string(&mut self) -> Result<String> {
        self.blob().map(decode_cp1252)
    }

    /// A version field that must hold one specific value.
    pub(crate) fn version(&mut self, what: &'static str, expected: i32) -> Result<()> {
        let start = self.pos;
        let found = self.i32()?;
        if found != expected {
            return Err(self.err_at(start, ErrorKind::UnsupportedVersion { what, found }));
        }
        Ok(())
    }

    /// A length-prefixed zlib block, inflated with a size cap.
    pub(crate) fn inflate(&mut self, context: impl Into<String>) -> Result<Inflated> {
        let start = self.pos;
        let compressed = self.blob()?;
        let context = context.into();
        let data =
            miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(compressed, MAX_INFLATED_BLOCK)
                .map_err(|e| self.err_at(start, ErrorKind::Inflate(format!("{context}: {e}"))))?;
        Ok(Inflated { data, context })
    }

    /// Checks that a block was consumed exactly.
    pub(crate) fn finish(&self) -> Result<()> {
        match self.remaining() {
            0 => Ok(()),
            n => Err(self.err(ErrorKind::TrailingBytes(n))),
        }
    }
}

/// Windows-1252 to Unicode. The five bytes cp1252 leaves undefined map to the matching C1 control
/// characters (as Windows' `MultiByteToWideChar` does), so the conversion is lossless.
pub fn decode_cp1252(bytes: &[u8]) -> String {
    const HIGH: [char; 32] = [
        '\u{20AC}', '\u{0081}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}',
        '\u{2021}', '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{008D}',
        '\u{017D}', '\u{008F}', '\u{0090}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}',
        '\u{2022}', '\u{2013}', '\u{2014}', '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}',
        '\u{0153}', '\u{009D}', '\u{017E}', '\u{0178}',
    ];
    bytes
        .iter()
        .map(|&b| match b {
            0x80..=0x9F => HIGH[usize::from(b - 0x80)],
            _ => char::from(b),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cp1252_round_trips_ascii_and_maps_high_bytes() {
        assert_eq!(decode_cp1252(b"abc"), "abc");
        assert_eq!(decode_cp1252(&[0x80, 0xE9, 0x81]), "\u{20AC}\u{E9}\u{81}");
    }

    #[test]
    fn reads_fail_cleanly_past_the_end() {
        let mut r = Reader::new(&[1, 0, 0], "t");
        assert!(matches!(
            r.i32().unwrap_err().kind,
            ErrorKind::UnexpectedEof {
                wanted: 4,
                available: 3
            }
        ));
        let mut r = Reader::new(&[0xFF, 0xFF, 0xFF, 0x7F], "t");
        assert!(matches!(
            r.count(1).unwrap_err().kind,
            ErrorKind::UnexpectedEof { .. }
        ));
        let mut r = Reader::new(&[0xFF, 0xFF, 0xFF, 0xFF], "t");
        assert_eq!(r.string().unwrap_err().kind, ErrorKind::NegativeLength(-1));
        let mut r = Reader::new(&[2, 0, 0, 0], "t");
        assert_eq!(r.bool().unwrap_err().kind, ErrorKind::InvalidBool(2));
    }
}
