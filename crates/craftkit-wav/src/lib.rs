//! `craftkit-wav` — **the WAV reader**: the samples of a sound, from
//! the file.
//!
//! # What this is
//!
//! A sound is a **header** and some **numbers**. The header says how
//! many channels there are, how fast they are played and how many bits
//! each sample has; the numbers are the samples themselves.
//!
//! This crate reads that: the **RIFF** container, the **format** chunk
//! and the **data** chunk, and it gives back the samples as `f32` —
//! interleaved, in the range −1 to 1, whatever the file's own width
//! was.
//!
//! # Why it is a library and not a plugin
//!
//! Because a file is a file: the reader reads bytes and gives numbers,
//! and it knows nothing about a document, a canvas or the kit. A
//! battery that plays a sound reads the file with this and puts the
//! samples in a **stream** beside the document (§33).
//!
//! # The file is hostile
//!
//! A file came from somewhere else, so every number in it is a number
//! that could be anything. The reader **refuses** what it cannot make
//! sense of, and it refuses **within limits**: a file that says it holds
//! a gigabyte of samples is not read, because the reading would be the
//! problem, not the file.
//!
//! # What this does not do
//!
//! - **It does not play anything.** There is no audio device here and
//!   no mixer: the reader gives back samples and stops.
//! - **It does not read other formats.** A sound file may be in AIFF,
//!   in FLAC or in something else, and those are refusals with names.
//! - **It does not do the file's own extras.** A WAV may carry a list
//!   of markers, a name or a "cue" chunk; they are left in the file.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

/// The four bytes a RIFF file begins with.
const RIFF: [u8; 4] = *b"RIFF";
/// The four bytes that follow a RIFF file's size.
const WAVE: [u8; 4] = *b"WAVE";
/// The four bytes of the chunk that says the format.
const FMT: [u8; 4] = *b"fmt ";
/// The four bytes of the chunk that holds the samples.
const DATA: [u8; 4] = *b"data";

/// The format of **PCM** samples: whole numbers.
const FORMAT_PCM: u16 = 1;
/// The format of **floating point** samples.
const FORMAT_FLOAT: u16 = 3;
/// The format of **extensible** samples: PCM or float, and the rest is
/// a name.
const FORMAT_EXTENSIBLE: u16 = 0xfffe;

/// The **limits** a file is read within.
///
/// The file is hostile, so every number it says is checked against
/// these before a single sample is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The most bytes the samples may take.
    pub max_bytes: usize,
    /// The most channels a sound may have.
    pub max_channels: u16,
    /// The fastest a sound may be played, in samples a second.
    pub max_rate: u32,
    /// The most frames a sound may have.
    pub max_frames: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Limits { max_bytes: 256 * 1024 * 1024, max_channels: 8, max_rate: 384_000, max_frames: 100_000_000 }
    }
}

/// Why a file was **refused**.
///
/// Every refusal has a name, because a refusal that says nothing is a
/// refusal the caller cannot do anything about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The file is too short to be a WAV at all.
    TooShort,
    /// The file does not begin with `RIFF`.
    NotRiff,
    /// The file is a RIFF, but not a `WAVE`.
    NotWave,
    /// A chunk says it is longer than the file.
    ChunkTooLong {
        /// The four bytes that name the chunk.
        name: String,
    },
    /// The file has no `fmt ` chunk.
    NoFormat,
    /// The file has no `data` chunk.
    NoData,
    /// The format is one the kit does not read.
    UnknownFormat {
        /// The number the file says.
        format: u16,
    },
    /// The number of channels is nothing, or too many.
    BadChannels {
        /// The number the file says.
        channels: u16,
    },
    /// The rate is nothing, or too fast.
    BadRate {
        /// The number the file says.
        rate: u32,
    },
    /// The width of a sample is one the kit does not read.
    UnknownWidth {
        /// The number of bits the file says.
        bits: u16,
    },
    /// The samples are more than the limits allow.
    TooManySamples,
    /// The frames are more than the limits allow.
    TooManyFrames,
    /// The file says it holds more samples than it has bytes.
    ShortData,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::TooShort => write!(f, "the file is too short to be a sound"),
            Error::NotRiff => write!(f, "the file is not a RIFF file"),
            Error::NotWave => write!(f, "the file is a RIFF file and not a sound"),
            Error::ChunkTooLong { name } => write!(f, "the chunk `{name}` is longer than the file"),
            Error::NoFormat => write!(f, "the file has no format"),
            Error::NoData => write!(f, "the file has no samples"),
            Error::UnknownFormat { format } => write!(f, "the format {format} is not one the kit reads"),
            Error::BadChannels { channels } => {
                write!(f, "a sound of {channels} channels is not one the kit reads")
            }
            Error::BadRate { rate } => write!(f, "a rate of {rate} is not one the kit reads"),
            Error::UnknownWidth { bits } => write!(f, "a sample of {bits} bits is not one the kit reads"),
            Error::TooManySamples => write!(f, "the sound has more samples than the limits allow"),
            Error::TooManyFrames => write!(f, "the sound has more frames than the limits allow"),
            Error::ShortData => write!(f, "the file says it has more samples than it has bytes"),
        }
    }
}

impl std::error::Error for Error {}

/// A sound, read from a WAV file.
///
/// The samples are **interleaved** and in the range −1 to 1: a file may
/// hold whole numbers of many widths and of two's complement or of
/// floats, and the reader gives back one thing.
#[derive(Debug, Clone, PartialEq)]
pub struct Sound {
    /// How many channels there are.
    pub channels: u16,
    /// How fast the sound is played, in samples a second.
    pub rate: u32,
    /// How many frames there are: a frame is one sample of every
    /// channel.
    pub frames: u64,
    /// The samples, interleaved: the first channel of the first frame,
    /// the second channel of the first frame, and so on.
    pub samples: Vec<f32>,
}

/// Read a sound from the bytes of a WAV file.
///
/// The bytes came from outside, so they are read **within limits** and
/// what cannot be made sense of is refused.
pub fn read(bytes: &[u8], limits: &Limits) -> Result<Sound, Error> {
    if bytes.len() < 12 {
        return Err(Error::TooShort);
    }
    if bytes[0..4] != RIFF {
        return Err(Error::NotRiff);
    }
    if bytes[8..12] != WAVE {
        return Err(Error::NotWave);
    }
    let mut format: Option<Format> = None;
    let mut data: Option<&[u8]> = None;
    // The chunks run from byte twelve to the end of the file. A chunk
    // that says it is longer than the file is a refusal, and not a
    // panic.
    let mut at = 12_usize;
    while at + 8 <= bytes.len() {
        let name = &bytes[at..at + 4];
        let size = u32::from_le_bytes([bytes[at + 4], bytes[at + 5], bytes[at + 6], bytes[at + 7]]) as usize;
        let start = at + 8;
        let end = start.checked_add(size).ok_or(Error::ChunkTooLong { name: chunk_name(name) })?;
        if end > bytes.len() {
            return Err(Error::ChunkTooLong { name: chunk_name(name) });
        }
        match name {
            _ if name == FMT => format = Some(read_format(&bytes[start..end])?),
            _ if name == DATA => data = Some(&bytes[start..end]),
            _ => {}
        }
        // A chunk is padded to an even number of bytes.
        at = end + (size % 2);
    }
    let format = format.ok_or(Error::NoFormat)?;
    let data = data.ok_or(Error::NoData)?;
    if data.len() > limits.max_bytes {
        return Err(Error::TooManySamples);
    }
    if format.channels == 0 || format.channels > limits.max_channels {
        return Err(Error::BadChannels { channels: format.channels });
    }
    if format.rate == 0 || format.rate > limits.max_rate {
        return Err(Error::BadRate { rate: format.rate });
    }
    let bytes_per_frame = format.bytes_per_frame();
    if bytes_per_frame == 0 {
        return Err(Error::UnknownWidth { bits: format.bits });
    }
    if data.len() % bytes_per_frame != 0 {
        return Err(Error::ShortData);
    }
    let frames = (data.len() / bytes_per_frame) as u64;
    if frames > limits.max_frames {
        return Err(Error::TooManyFrames);
    }
    let samples = decode(data, &format);
    Ok(Sound { channels: format.channels, rate: format.rate, frames, samples })
}

/// What the format chunk says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Format {
    /// PCM or float.
    kind: u16,
    /// How many channels.
    channels: u16,
    /// How fast it is played.
    rate: u32,
    /// How many bits a sample has.
    bits: u16,
}

impl Format {
    /// How many bytes a frame takes: a sample of every channel.
    fn bytes_per_frame(&self) -> usize {
        let width = match (self.kind, self.bits) {
            (FORMAT_PCM, 8) => 1,
            (FORMAT_PCM, 16) => 2,
            (FORMAT_PCM, 24) => 3,
            (FORMAT_PCM, 32) => 4,
            (FORMAT_FLOAT, 32) => 4,
            (FORMAT_FLOAT, 64) => 8,
            _ => return 0,
        };
        width * self.channels as usize
    }
}

/// Read the format chunk.
///
/// The chunk is **extensible** when it says so: then the format is the
/// first two bytes of the chunk's own sub-format, and the rest is a
/// name and a set of numbers the kit does not need.
fn read_format(chunk: &[u8]) -> Result<Format, Error> {
    if chunk.len() < 16 {
        return Err(Error::NoFormat);
    }
    let mut kind = u16::from_le_bytes([chunk[0], chunk[1]]);
    let channels = u16::from_le_bytes([chunk[2], chunk[3]]);
    let rate = u32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]);
    let bits = u16::from_le_bytes([chunk[14], chunk[15]]);
    if kind == FORMAT_EXTENSIBLE {
        // The sub-format's GUID begins with the two bytes that say
        // whether the samples are PCM or float.
        if chunk.len() < 26 {
            return Err(Error::NoFormat);
        }
        kind = u16::from_le_bytes([chunk[24], chunk[25]]);
    }
    if kind != FORMAT_PCM && kind != FORMAT_FLOAT {
        return Err(Error::UnknownFormat { format: kind });
    }
    Ok(Format { kind, channels, rate, bits })
}

/// The samples, as numbers between −1 and 1.
fn decode(data: &[u8], format: &Format) -> Vec<f32> {
    let width = match (format.kind, format.bits) {
        (FORMAT_PCM, 8) => 1,
        (FORMAT_PCM, 16) => 2,
        (FORMAT_PCM, 24) => 3,
        (FORMAT_PCM, 32) => 4,
        (FORMAT_FLOAT, 32) => 4,
        (FORMAT_FLOAT, 64) => 8,
        _ => return Vec::new(),
    };
    let count = data.len() / width;
    let mut samples = Vec::with_capacity(count);
    for index in 0..count {
        let at = index * width;
        let value = match width {
            1 => (data[at] as f32 - 128.0) / 128.0,
            2 => i16::from_le_bytes([data[at], data[at + 1]]) as f32 / 32768.0,
            3 => {
                let raw = i32::from_le_bytes([data[at], data[at + 1], data[at + 2], 0]);
                (raw as f32 / 8_388_608.0).clamp(-1.0, 1.0)
            }
            4 if format.kind == FORMAT_FLOAT => {
                f32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]])
            }
            4 => {
                i32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]) as f32
                    / 2_147_483_648.0
            }
            8 => {
                let mut bytes = [0_u8; 8];
                bytes.copy_from_slice(&data[at..at + 8]);
                f64::from_le_bytes(bytes) as f32
            }
            _ => 0.0,
        };
        samples.push(value);
    }
    samples
}

/// The name of a chunk, for a refusal.
fn chunk_name(name: &[u8]) -> String {
    name.iter().map(|byte| char::from(*byte)).collect()
}

/// Write a sound as a WAV file.
///
/// The file is **sixteen bit** PCM: it is the width every tool reads,
/// and a file the kit writes is a file the kit can read back.
///
/// The samples are **clipped** to the range a file may hold: a mix may
/// be louder than one, and a file cannot say so.
pub fn write(channels: u16, rate: u32, samples: &[f32]) -> Vec<u8> {
    let channel_count = channels.max(1);
    let channels = channel_count as usize;
    let frames = samples.len() / channels;
    let mut bytes: Vec<u8> = Vec::with_capacity(44 + frames * channels * 2);
    bytes.extend_from_slice(&RIFF);
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(&WAVE);
    // The format chunk.
    bytes.extend_from_slice(&FMT);
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&FORMAT_PCM.to_le_bytes());
    bytes.extend_from_slice(&channel_count.to_le_bytes());
    bytes.extend_from_slice(&rate.to_le_bytes());
    bytes.extend_from_slice(&(rate * channels as u32).to_le_bytes());
    bytes.extend_from_slice(&((channels * 2) as u16).to_le_bytes());
    bytes.extend_from_slice(&16_u16.to_le_bytes());
    // The samples.
    bytes.extend_from_slice(&DATA);
    bytes.extend_from_slice(&((frames * channels * 2) as u32).to_le_bytes());
    for sample in samples.iter().take(frames * channels) {
        let value = (sample.clamp(-1.0, 1.0) * 32767.0).round() as i16;
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    let size = (bytes.len() - 8) as u32;
    bytes[4..8].copy_from_slice(&size.to_le_bytes());
    bytes
}
