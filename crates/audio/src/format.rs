//! What the model needs to know about a sound file, read from its headers without decoding it.
//!
//! supersound.dll plays everything as 16- or 8-bit PCM in a DirectSound buffer, so a sound is described by
//! the PCM it becomes: a WAV's own format and `data` chunk, or an Ogg Vorbis file's channels and rate at
//! 16 bits (the DLL decodes with libvorbis, which the game asks for 16-bit samples). Positions and lengths
//! the game can query are byte offsets into that PCM.

use std::fmt;

/// The PCM a sound file plays as.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct SoundInfo {
    /// Sample frames per second at the file's own rate (DirectSound's "original frequency").
    pub sample_rate: u32,
    pub channels: u16,
    /// Bytes per sample frame (DirectSound's block align): the WAV's, or 2 per channel for Ogg Vorbis.
    pub frame_bytes: u16,
    /// Length in sample frames.
    pub frames: u64,
}

impl SoundInfo {
    /// Reads a WAV (PCM) or Ogg Vorbis file's headers.
    pub fn parse(bytes: &[u8]) -> Result<Self, FormatError> {
        if bytes.starts_with(b"RIFF") {
            parse_wav(bytes)
        } else if bytes.starts_with(b"OggS") {
            parse_ogg(bytes)
        } else {
            Err(FormatError::UnknownFormat)
        }
    }

    /// The PCM's length in bytes: a static sound's DirectSound buffer size.
    pub fn bytes(&self) -> u64 {
        self.frames * u64::from(self.frame_bytes)
    }

    /// `nAvgBytesPerSec` of the buffer's format: bytes per second at the original frequency.
    pub fn bytes_per_second(&self) -> u64 {
        u64::from(self.sample_rate) * u64::from(self.frame_bytes)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormatError {
    /// Neither `RIFF` nor `OggS`.
    UnknownFormat,
    /// The file ends inside a header.
    Truncated,
    /// A well-formed file the DLL couldn't play either, e.g. compressed WAV.
    Unsupported(&'static str),
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownFormat => f.write_str("not a WAV or Ogg file"),
            Self::Truncated => f.write_str("file ends inside a header"),
            Self::Unsupported(what) => write!(f, "unsupported: {what}"),
        }
    }
}

impl std::error::Error for FormatError {}

fn u16_at(b: &[u8], at: usize) -> Result<u16, FormatError> {
    b.get(at..at + 2)
        .map(|s| u16::from_le_bytes([s[0], s[1]]))
        .ok_or(FormatError::Truncated)
}

fn u32_at(b: &[u8], at: usize) -> Result<u32, FormatError> {
    b.get(at..at + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        .ok_or(FormatError::Truncated)
}

fn u64_at(b: &[u8], at: usize) -> Result<u64, FormatError> {
    let s = b.get(at..at + 8).ok_or(FormatError::Truncated)?;
    Ok(u64::from_le_bytes(s.try_into().expect("8 bytes")))
}

/// RIFF chunks after `WAVE`: `fmt ` (must be PCM) and `data`, whose size is the buffer size.
fn parse_wav(b: &[u8]) -> Result<SoundInfo, FormatError> {
    if b.get(8..12) != Some(b"WAVE") {
        return Err(FormatError::UnknownFormat);
    }
    let mut fmt = None;
    let mut at: usize = 12;
    // `at` comes from sizes in the file, so every step is checked: a bogus size ends the walk.
    while let Some(body) = at.checked_add(8).filter(|&body| body <= b.len()) {
        let id = &b[at..at + 4];
        let size = u32_at(b, at + 4)? as usize;
        match id {
            b"fmt " => {
                if u16_at(b, body)? != 1 {
                    return Err(FormatError::Unsupported("WAV that isn't PCM"));
                }
                let channels = u16_at(b, body + 2)?;
                let sample_rate = u32_at(b, body + 4)?;
                let frame_bytes = u16_at(b, body + 12)?;
                if channels == 0 || frame_bytes == 0 || sample_rate == 0 {
                    return Err(FormatError::Unsupported("WAV with an empty format"));
                }
                fmt = Some((channels, sample_rate, frame_bytes));
            }
            b"data" => {
                let (channels, sample_rate, frame_bytes) = fmt.ok_or(FormatError::Unsupported(
                    "WAV with its data before its format",
                ))?;
                // A size past the end of the file is cut to what's there.
                let size = size.min(b.len() - body) as u64;
                return Ok(SoundInfo {
                    sample_rate,
                    channels,
                    frame_bytes,
                    frames: size / u64::from(frame_bytes),
                });
            }
            _ => {}
        }
        // Chunks are padded to an even size.
        match body
            .checked_add(size)
            .and_then(|end| end.checked_add(size & 1))
        {
            Some(next) => at = next,
            None => break,
        }
    }
    Err(FormatError::Truncated)
}

/// The Vorbis identification header on the first page gives channels and rate; the last page's granule
/// position is the stream's length in sample frames.
///
/// The walk ends at the first byte that doesn't start a complete page, keeping the last granule seen:
/// libvorbisfile (which the DLL decodes with) skips anything that isn't a page, so trailing tags or
/// padding don't make a file unplayable, and a page cut off at the end of the file is never decoded.
fn parse_ogg(b: &[u8]) -> Result<SoundInfo, FormatError> {
    let mut at = 0;
    let mut ident = None;
    let mut frames = 0;
    while b.get(at..at + 4) == Some(b"OggS") {
        let Some(&segments) = b.get(at + 26) else {
            break;
        };
        let body = at + 27 + usize::from(segments);
        let Some(table) = b.get(at + 27..body) else {
            break;
        };
        let end = body + table.iter().map(|&n| usize::from(n)).sum::<usize>();
        if end > b.len() {
            break;
        }
        let granule = u64_at(b, at + 6)?;
        if ident.is_none() {
            let packet = b.get(body..body + 16).ok_or(FormatError::Truncated)?;
            if &packet[..7] != b"\x01vorbis" {
                return Err(FormatError::Unsupported("Ogg that isn't Vorbis"));
            }
            let channels = u16::from(packet[11]);
            let sample_rate = u32_at(packet, 12)?;
            if channels == 0 || sample_rate == 0 {
                return Err(FormatError::Unsupported("Vorbis with an empty format"));
            }
            ident = Some((channels, sample_rate));
        }
        // -1 marks a page on which no packet ends.
        if granule != u64::MAX {
            frames = granule;
        }
        at = end;
    }
    let (channels, sample_rate) = ident.ok_or(FormatError::Truncated)?;
    Ok(SoundInfo {
        sample_rate,
        channels,
        frame_bytes: channels * 2,
        frames,
    })
}

/// A minimal PCM WAV with `data_len` bytes of silence.
#[cfg(test)]
pub(crate) fn wav(rate: u32, channels: u16, bits: u16, data_len: u32) -> Vec<u8> {
    let align = channels * bits / 8;
    let mut v = Vec::new();
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&(36 + data_len).to_le_bytes());
    v.extend_from_slice(b"WAVEfmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&channels.to_le_bytes());
    v.extend_from_slice(&rate.to_le_bytes());
    v.extend_from_slice(&(rate * u32::from(align)).to_le_bytes());
    v.extend_from_slice(&align.to_le_bytes());
    v.extend_from_slice(&bits.to_le_bytes());
    v.extend_from_slice(b"data");
    v.extend_from_slice(&data_len.to_le_bytes());
    v.resize(v.len() + data_len as usize, 0);
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ogg_page(granule: u64, body: &[u8]) -> Vec<u8> {
        let mut v = b"OggS\0\0".to_vec();
        v.extend_from_slice(&granule.to_le_bytes());
        v.extend_from_slice(&[0; 12]);
        v.push(1);
        v.push(body.len() as u8);
        v.extend_from_slice(body);
        v
    }

    #[test]
    fn reads_wav() {
        let info = SoundInfo::parse(&wav(22050, 1, 16, 1000)).unwrap();
        assert_eq!(
            info,
            SoundInfo {
                sample_rate: 22050,
                channels: 1,
                frame_bytes: 2,
                frames: 500
            }
        );
        assert_eq!(info.bytes(), 1000);
        assert_eq!(info.bytes_per_second(), 44100);
    }

    #[test]
    fn reads_ogg_length_from_last_granule() {
        let mut ident = b"\x01vorbis\0\0\0\0\x02".to_vec();
        ident.extend_from_slice(&44100u32.to_le_bytes());
        let mut file = ogg_page(0, &ident);
        file.extend(ogg_page(u64::MAX, b"x"));
        file.extend(ogg_page(4410, b"y"));
        file.extend(ogg_page(u64::MAX, b"z"));
        let info = SoundInfo::parse(&file).unwrap();
        assert_eq!(
            info,
            SoundInfo {
                sample_rate: 44100,
                channels: 2,
                frame_bytes: 4,
                frames: 4410
            }
        );

        // Trailing bytes that aren't a page, and a page cut off by the end of the file, are ignored.
        let mut tagged = file.clone();
        tagged.extend_from_slice(b"TAG padding");
        assert_eq!(SoundInfo::parse(&tagged), Ok(info));
        let mut cut = file.clone();
        cut.extend_from_slice(&ogg_page(8820, b"w")[..28]);
        assert_eq!(SoundInfo::parse(&cut), Ok(info));
    }

    #[test]
    fn survives_bogus_wav_chunk_sizes() {
        let mut file = wav(44100, 1, 16, 10);
        // An unknown chunk claiming 4 GiB before the format.
        file.splice(12..12, *b"junk\xff\xff\xff\xff");
        assert_eq!(SoundInfo::parse(&file), Err(FormatError::Truncated));
    }

    #[test]
    fn rejects_other_files() {
        assert_eq!(
            SoundInfo::parse(b"ID3 mp3"),
            Err(FormatError::UnknownFormat)
        );
        assert_eq!(
            SoundInfo::parse(&wav(44100, 1, 8, 10)[..30]),
            Err(FormatError::Truncated)
        );
    }
}
