//! The **inflate**: DEFLATE, from RFC 1951.
//!
//! # Why it is here
//!
//! A PNG file's pixels are in a **zlib** stream, and a zlib stream is a
//! DEFLATE stream. So a kit that reads PNG needs an inflate, and the
//! kit had none: it wrote PNG (§67) and read a PNG's *header* (§68),
//! and the pixels were out of reach.
//!
//! # What it is
//!
//! The three kinds of block DEFLATE has — stored, fixed Huffman and
//! dynamic Huffman — and the zlib wrapper around them: the two header
//! bytes and the Adler-32 checksum at the end.
//!
//! The Huffman decoding is the **canonical** one: the code lengths are
//! counted, the codes are numbered in order, and the bits are read one
//! at a time until a code is found. It is the slowest of the ways to do
//! it and the one that is easiest to be sure of, and a decoder that is
//! easy to be sure of is a decoder that can be checked.
//!
//! # What it does not do
//!
//! It does not check the checksum of a zlib stream that does not have
//! one, and it does not decode a stream that is not DEFLATE: those are
//! refusals with names, and they are made here.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

/// Why a stream could not be inflated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The stream is not DEFLATE, or is not the zlib wrapper around it.
    Malformed(String),
    /// The stream is DEFLATE and uses something this decoder does not.
    Unsupported(String),
    /// The stream asked for more memory than the caller allowed.
    LimitExceeded(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Malformed(message) => write!(f, "malformed stream: {message}"),
            Error::Unsupported(message) => write!(f, "{message}"),
            Error::LimitExceeded(message) => write!(f, "limit exceeded: {message}"),
        }
    }
}

impl std::error::Error for Error {}

/// How much a stream may inflate to.
///
/// The limit is **the caller's**: a picture that says it is a hundred
/// thousand by a hundred thousand pixels is a bomb, and the decoder does
/// not decide how much memory a program may use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The most bytes the inflated stream may hold.
    pub max_output: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits { max_output: 512 * 1024 * 1024 }
    }
}

/// The bits of the stream, read one at a time.
///
/// DEFLATE packs its bits **least significant first**, and a byte is
/// returned when it is spent.
struct Bits<'a> {
    bytes: &'a [u8],
    at: usize,
    /// The bits of the byte in hand.
    held: u32,
    /// How many of them are left.
    count: u32,
}

impl<'a> Bits<'a> {
    /// The bits of those bytes.
    fn new(bytes: &'a [u8]) -> Self {
        Bits { bytes, at: 0, held: 0, count: 0 }
    }

    /// The next bit, least significant first.
    fn bit(&mut self) -> Result<u32, Error> {
        if self.count == 0 {
            let byte = *self
                .bytes
                .get(self.at)
                .ok_or_else(|| Error::Malformed("the stream ended in the middle".to_string()))?;
            self.at += 1;
            self.held = u32::from(byte);
            self.count = 8;
        }
        let value = self.held & 1;
        self.held >>= 1;
        self.count -= 1;
        Ok(value)
    }

    /// The next `count` bits, least significant first.
    fn bits(&mut self, count: u32) -> Result<u32, Error> {
        let mut value = 0;
        for index in 0..count {
            value |= self.bit()? << index;
        }
        Ok(value)
    }

    /// The stream is aligned on a byte, and the rest of the byte is
    /// dropped.
    fn align(&mut self) {
        self.held = 0;
        self.count = 0;
    }

    /// The bytes that are left, from the current position.
    fn rest(&self) -> &'a [u8] {
        self.bytes.get(self.at..).unwrap_or(&[])
    }
}

/// A Huffman table in the **canonical** form: the count of the codes of
/// each length, and the symbols in order.
struct Huffman {
    counts: [u16; 16],
    symbols: Vec<u16>,
}

impl Huffman {
    /// The table of those code lengths.
    fn of(lengths: &[u8]) -> Result<Huffman, Error> {
        let mut counts = [0u16; 16];
        for length in lengths {
            let length = *length;
            if length as usize > 15 {
                return Err(Error::Malformed("a code is longer than fifteen bits".to_string()));
            }
            if length > 0 {
                counts[length as usize - 1] += 1;
            }
        }
        // The codes must not be over-subscribed: the sum of the codes of
        // each length may not exceed what a length can hold.
        let mut left = 1i32;
        for count in counts.iter() {
            left <<= 1;
            left -= i32::from(*count);
            if left < 0 {
                return Err(Error::Malformed("the codes do not fit".to_string()));
            }
        }
        // The symbols of each length start after the symbols of every
        // shorter length: the canonical order is by length first, and by
        // symbol second.
        let mut offsets = [0u16; 16];
        for length in 2..16 {
            offsets[length] = offsets[length - 1] + counts[length - 2];
        }
        let mut symbols = vec![0u16; lengths.len()];
        for (symbol, length) in lengths.iter().enumerate() {
            if *length > 0 {
                symbols[offsets[*length as usize] as usize] = symbol as u16;
                offsets[*length as usize] += 1;
            }
        }
        Ok(Huffman { counts, symbols })
    }

    /// The symbol the next bits spell.
    fn symbol(&self, bits: &mut Bits<'_>) -> Result<u16, Error> {
        let mut code = 0i32;
        let mut first = 0i32;
        let mut index = 0i32;
        for length in 1..=15 {
            code |= bits.bit()? as i32;
            let count = i32::from(self.counts[length - 1]);
            if code - count < first {
                let at = (index + (code - first)) as usize;
                return self
                    .symbols
                    .get(at)
                    .copied()
                    .ok_or_else(|| Error::Malformed("the code is not in the table".to_string()));
            }
            index += count;
            first = (first + count) << 1;
            code <<= 1;
        }
        Err(Error::Malformed("the code is longer than the table".to_string()))
    }
}

/// The lengths of the literal and length codes of a fixed table.
fn fixed_lengths() -> [u8; 288] {
    let mut lengths = [0u8; 288];
    for (symbol, length) in lengths.iter_mut().enumerate() {
        *length = match symbol {
            0..=143 => 8,
            144..=255 => 9,
            256..=279 => 7,
            _ => 8,
        };
    }
    lengths
}

/// The lengths of the distance codes of a fixed table.
fn fixed_distance_lengths() -> [u8; 30] {
    [5u8; 30]
}

/// The base of each length code, and how many extra bits it has.
const LENGTH_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195,
    227, 258,
];
const LENGTH_EXTRA: [u32; 29] =
    [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0];
const DISTANCE_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073,
    4097, 6145, 8193, 12289, 16385, 24577,
];
const DISTANCE_EXTRA: [u32; 30] =
    [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13];
/// The order the code lengths of a dynamic table are written in.
const CODE_LENGTH_ORDER: [usize; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

/// Inflate a **zlib** stream.
///
/// The two header bytes are checked, the DEFLATE stream is inflated, and
/// the Adler-32 at the end is checked against what was inflated.
pub fn zlib(bytes: &[u8], limits: &Limits) -> Result<Vec<u8>, Error> {
    let header = bytes.get(..2).ok_or_else(|| Error::Malformed("the stream is too short".to_string()))?;
    let method = header[0] & 0x0F;
    if method != 8 {
        return Err(Error::Unsupported("the stream is not deflate".to_string()));
    }
    if (u16::from(header[0]) * 256 + u16::from(header[1])) % 31 != 0 {
        return Err(Error::Malformed("the stream's header is wrong".to_string()));
    }
    if header[1] & 0x20 != 0 {
        return Err(Error::Unsupported("the stream has a preset dictionary".to_string()));
    }
    let mut bits = Bits::new(&bytes[2..]);
    let out = deflate(&mut bits, limits)?;
    bits.align();
    let checksum = bytes
        .get(bits.at + 2..bits.at + 6)
        .ok_or_else(|| Error::Malformed("the stream has no checksum".to_string()))?;
    let expected = u32::from_be_bytes([checksum[0], checksum[1], checksum[2], checksum[3]]);
    if adler32(&out) != expected {
        return Err(Error::Malformed("the stream's checksum does not match".to_string()));
    }
    Ok(out)
}

/// Inflate a **DEFLATE** stream.
fn deflate(bits: &mut Bits<'_>, limits: &Limits) -> Result<Vec<u8>, Error> {
    let mut out: Vec<u8> = Vec::new();
    loop {
        let last = bits.bit()? == 1;
        let kind = bits.bits(2)?;
        match kind {
            0 => {
                bits.align();
                let header = bits.rest();
                if header.len() < 4 {
                    return Err(Error::Malformed("the stored block is too short".to_string()));
                }
                let length = usize::from(u16::from_le_bytes([header[0], header[1]]));
                let complement = usize::from(u16::from_le_bytes([header[2], header[3]]));
                if length != !complement & 0xFFFF {
                    return Err(Error::Malformed("the stored block's length is wrong".to_string()));
                }
                let data = header.get(4..4 + length).ok_or_else(|| {
                    Error::Malformed("the stored block is shorter than it says".to_string())
                })?;
                if out.len() + length > limits.max_output {
                    return Err(Error::LimitExceeded("the stream is too big".to_string()));
                }
                out.extend_from_slice(data);
                // The bytes were taken whole, so the reader is moved on
                // past them.
                bits.at += 4 + length;
                bits.held = 0;
                bits.count = 0;
            }
            1 => {
                let literals = Huffman::of(&fixed_lengths())?;
                let distances = Huffman::of(&fixed_distance_lengths())?;
                block(bits, &literals, &distances, &mut out, limits)?;
            }
            2 => {
                let (literals, distances) = dynamic_tables(bits)?;
                block(bits, &literals, &distances, &mut out, limits)?;
            }
            _ => {
                return Err(Error::Malformed("the block is not one of the three".to_string()));
            }
        }
        if last {
            return Ok(out);
        }
    }
}

/// The two tables of a dynamic block.
fn dynamic_tables(bits: &mut Bits<'_>) -> Result<(Huffman, Huffman), Error> {
    let literal_count = bits.bits(5)? as usize + 257;
    let distance_count = bits.bits(5)? as usize + 1;
    let code_count = bits.bits(4)? as usize + 4;
    let mut code_lengths = [0u8; 19];
    for index in 0..code_count {
        code_lengths[CODE_LENGTH_ORDER[index]] = bits.bits(3)? as u8;
    }
    let code_table = Huffman::of(&code_lengths)?;
    let mut lengths = vec![0u8; literal_count + distance_count];
    let mut at = 0;
    while at < lengths.len() {
        let symbol = code_table.symbol(bits)?;
        match symbol {
            0..=15 => {
                lengths[at] = symbol as u8;
                at += 1;
            }
            16 => {
                if at == 0 {
                    return Err(Error::Malformed("the code repeats nothing".to_string()));
                }
                let previous = lengths[at - 1];
                let repeat = 3 + bits.bits(2)? as usize;
                if at + repeat > lengths.len() {
                    return Err(Error::Malformed("the code repeats too much".to_string()));
                }
                for _ in 0..repeat {
                    lengths[at] = previous;
                    at += 1;
                }
            }
            17 => {
                let repeat = 3 + bits.bits(3)? as usize;
                if at + repeat > lengths.len() {
                    return Err(Error::Malformed("the code repeats too much".to_string()));
                }
                at += repeat;
            }
            18 => {
                let repeat = 11 + bits.bits(7)? as usize;
                if at + repeat > lengths.len() {
                    return Err(Error::Malformed("the code repeats too much".to_string()));
                }
                at += repeat;
            }
            _ => {
                return Err(Error::Malformed("the code is not one of the nineteen".to_string()));
            }
        }
    }
    let literals = Huffman::of(&lengths[..literal_count])?;
    let distances = Huffman::of(&lengths[literal_count..])?;
    Ok((literals, distances))
}

/// One block of a Huffman-coded stream.
fn block(
    bits: &mut Bits<'_>,
    literals: &Huffman,
    distances: &Huffman,
    out: &mut Vec<u8>,
    limits: &Limits,
) -> Result<(), Error> {
    loop {
        let symbol = literals.symbol(bits)?;
        match symbol {
            0..=255 => {
                if out.len() + 1 > limits.max_output {
                    return Err(Error::LimitExceeded("the stream is too big".to_string()));
                }
                out.push(symbol as u8);
            }
            256 => return Ok(()),
            257..=285 => {
                let at = (symbol - 257) as usize;
                let length = LENGTH_BASE[at] as usize + bits.bits(LENGTH_EXTRA[at])? as usize;
                let distance_symbol = distances.symbol(bits)? as usize;
                if distance_symbol >= DISTANCE_BASE.len() {
                    return Err(Error::Malformed("the distance is not a distance".to_string()));
                }
                let distance = DISTANCE_BASE[distance_symbol] as usize
                    + bits.bits(DISTANCE_EXTRA[distance_symbol])? as usize;
                if distance > out.len() {
                    return Err(Error::Malformed("the copy reaches before the stream".to_string()));
                }
                if out.len() + length > limits.max_output {
                    return Err(Error::LimitExceeded("the stream is too big".to_string()));
                }
                let from = out.len() - distance;
                for offset in 0..length {
                    let byte = out[from + offset];
                    out.push(byte);
                }
            }
            _ => {
                return Err(Error::Malformed("the symbol is not a literal or a length".to_string()));
            }
        }
    }
}

/// The checksum a zlib stream ends with.
pub fn adler32(bytes: &[u8]) -> u32 {
    let mut a = 1u32;
    let mut b = 0u32;
    for byte in bytes {
        a = (a + u32::from(*byte)) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}
