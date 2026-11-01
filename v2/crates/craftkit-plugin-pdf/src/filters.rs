//! The **filters**: the ways a PDF stream's bytes may be encoded.
//!
//! A stream in a PDF file may be **encoded**: the `/Filter` entry
//! names the way, and the bytes in the file are the encoded ones. The
//! tree's own filter crate holds five ways and this module holds the
//! same five:
//!
//! - **Flate** — a zlib stream, which the PNG crate's inflate reads
//!   (RFC 1950 and RFC 1951);
//! - **LZW** — the variant LZW with the early change, where the code
//!   grows one entry before the table is full;
//! - **ASCIIHex** — the digits of a byte in hex, and `>` at the end;
//! - **ASCII85** — five digits to four bytes, and `~>` at the end;
//! - **RunLength** — a length byte and then the bytes it counts.
//!
//! # What it does not do
//!
//! - **The predictors are not here.** A predictor is a second step on
//!   top of Flate or LZW, and it is the next arc.
//! - **There is no encoder.** A filter that undoes a stream is worth
//!   having on its own: the tree's encoder writes the streams it reads.
//! - **The bytes are bounded, and the bound is the caller's.** A
//!   stream that decodes to more than the limit allows is refused, and
//!   it is refused before the memory is spent.

use craftkit_sdk::serde_json::Value as Json;

/// A **filter**: the way a stream's bytes are encoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    /// A zlib stream.
    Flate,
    /// LZW, with the early change.
    Lzw,
    /// The digits of a byte in hex.
    AsciiHex,
    /// Five digits to four bytes.
    Ascii85,
    /// A length byte and the bytes it counts.
    RunLength,
}

impl Filter {
    /// The filter by the name the PDF gives it.
    ///
    /// The name in the file is the long one — `FlateDecode` — and the
    /// abbreviation is the short one — `Fl` — and both are read, because
    /// a file may hold either.
    pub fn by_name(name: &str) -> Option<Filter> {
        match name.trim() {
            "FlateDecode" | "Fl" => Some(Filter::Flate),
            "LZWDecode" | "LZW" => Some(Filter::Lzw),
            "ASCIIHexDecode" | "AHx" => Some(Filter::AsciiHex),
            "ASCII85Decode" | "A85" => Some(Filter::Ascii85),
            "RunLengthDecode" | "RL" => Some(Filter::RunLength),
            _ => None,
        }
    }
}

/// The **limits** on a stream that is being decoded.
///
/// The limit is the **caller's**: the kit does not know what a program
/// opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The most bytes the **encoded** stream may hold.
    pub max_input: usize,
    /// The most bytes the **decoded** stream may hold.
    pub max_output: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits { max_input: 256 * 1024 * 1024, max_output: 64 * 1024 * 1024 }
    }
}

/// Why a stream could not be **decoded**.
///
/// Every refusal has a name, because a refusal that says nothing is a
/// refusal the caller cannot do anything about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The bytes are not what the filter says they are.
    Malformed(String),
    /// The kit does not know how to undo this.
    Unsupported(String),
    /// The stream is longer than the limit allows.
    TooLong {
        /// The limit that was reached.
        limit: usize,
    },
}

/// Decode a stream's **bytes**.
pub fn decode(filter: Filter, bytes: &[u8], limits: &Limits) -> Result<Vec<u8>, Error> {
    if bytes.len() > limits.max_input {
        return Err(Error::TooLong { limit: limits.max_input });
    }
    match filter {
        Filter::Flate => flate(bytes, limits),
        Filter::Lzw => lzw(bytes, limits),
        Filter::AsciiHex => ascii_hex(bytes, limits),
        Filter::Ascii85 => ascii85(bytes, limits),
        Filter::RunLength => run_length(bytes, limits),
    }
}

/// Decode a stream from the **parameters** a PDF gives it.
///
/// The `/Filter` entry may name more than one filter, and the filters
/// are applied in the order they are named: the last one named is the
/// first one undone.
pub fn decode_named(names: &[String], bytes: &[u8], limits: &Limits) -> Result<Vec<u8>, Error> {
    let mut filters = Vec::with_capacity(names.len());
    for name in names {
        let Some(filter) = Filter::by_name(name) else {
            return Err(Error::Unsupported(format!("the kit does not know the filter `{name}`")));
        };
        filters.push(filter);
    }
    let mut out = bytes.to_vec();
    for filter in filters.iter().rev() {
        out = decode(*filter, &out, limits)?;
    }
    Ok(out)
}

/// The filter a **payload** names.
///
/// The parameters of a PDF object are a list of pairs, and the filter
/// is the value of `/Filter`: one name, or a list of them.
pub fn filter_of(entries: &[(String, Json)]) -> Option<Vec<String>> {
    let (_, value) = entries.iter().find(|(key, _)| key == "Filter")?;
    match value {
        Json::String(name) => Some(vec![name.clone()]),
        Json::Array(names) => {
            Some(names.iter().filter_map(|name| name.as_str().map(|name| name.to_string())).collect())
        }
        _ => None,
    }
}

/// A zlib stream, undone with the PNG crate's inflate.
fn flate(bytes: &[u8], limits: &Limits) -> Result<Vec<u8>, Error> {
    let png_limits = craftkit_png::Limits { max_output: limits.max_output };
    craftkit_png::inflate_bytes(bytes, &png_limits).map_err(|error| Error::Malformed(error.to_string()))
}

/// The digits of a byte in hex, undone.
fn ascii_hex(bytes: &[u8], limits: &Limits) -> Result<Vec<u8>, Error> {
    let mut out: Vec<u8> = Vec::new();
    let mut high: Option<u8> = None;
    for byte in bytes {
        match byte {
            b'>' => break,
            b' ' | b'\t' | b'\r' | b'\n' | 0x0c | 0x00 => continue,
            _ => {}
        }
        let digit = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            b'A'..=b'F' => byte - b'A' + 10,
            _ => return Err(Error::Malformed(format!("the byte {byte} is not a hex digit"))),
        };
        match high.take() {
            Some(first) => {
                out.push((first << 4) | digit);
                if out.len() > limits.max_output {
                    return Err(Error::TooLong { limit: limits.max_output });
                }
            }
            None => high = Some(digit),
        }
    }
    if high.is_some() {
        return Err(Error::Malformed("the last digit has no pair".to_string()));
    }
    Ok(out)
}

/// Five digits to four bytes, undone.
fn ascii85(bytes: &[u8], limits: &Limits) -> Result<Vec<u8>, Error> {
    let mut out: Vec<u8> = Vec::new();
    let mut group: Vec<u8> = Vec::with_capacity(5);
    for byte in bytes {
        match byte {
            b'~' => break,
            b' ' | b'\t' | b'\r' | b'\n' | 0x0c | 0x00 => continue,
            b'z' => {
                out.extend_from_slice(&[0, 0, 0, 0]);
                if out.len() > limits.max_output {
                    return Err(Error::TooLong { limit: limits.max_output });
                }
                continue;
            }
            _ => {}
        }
        if !(b'!'..=b'u').contains(byte) {
            return Err(Error::Malformed(format!("the byte {byte} is not in the alphabet")));
        }
        group.push(byte - b'!');
        if group.len() == 5 {
            out.extend_from_slice(&group_value(&group)?.to_be_bytes());
            group.clear();
            if out.len() > limits.max_output {
                return Err(Error::TooLong { limit: limits.max_output });
            }
        }
    }
    if !group.is_empty() {
        // The last group is **short**, and it is padded with the
        // largest digit: the bytes it stands for are the ones before
        // the padding.
        let count = group.len();
        let mut padded = group.clone();
        while padded.len() < 5 {
            padded.push(84);
        }
        let value = group_value(&padded)?;
        let bytes = value.to_be_bytes();
        out.extend_from_slice(&bytes[..count - 1]);
    }
    Ok(out)
}

/// The four bytes five digits stand for.
///
/// The five digits are read in **base 85**, and the largest group of
/// them is `85⁵ − 1`, which is larger than a `u32` may hold. The sum
/// is therefore made in a `u64` and the group that does not fit in
/// four bytes is refused.
fn group_value(group: &[u8]) -> Result<u32, Error> {
    let mut value: u64 = 0;
    for digit in group {
        value = value * 85 + u64::from(*digit);
    }
    if value > u64::from(u32::MAX) {
        return Err(Error::Malformed("the group is too large".to_string()));
    }
    Ok(value as u32)
}

/// A length byte and the bytes it counts, undone.
fn run_length(bytes: &[u8], limits: &Limits) -> Result<Vec<u8>, Error> {
    let mut out: Vec<u8> = Vec::new();
    let mut at = 0_usize;
    while at < bytes.len() {
        let length = bytes[at];
        at += 1;
        match length {
            128 => break,
            0..=127 => {
                let count = length as usize + 1;
                let end = at + count;
                if end > bytes.len() {
                    return Err(Error::Malformed("the run is shorter than it says".to_string()));
                }
                out.extend_from_slice(&bytes[at..end]);
                at = end;
            }
            _ => {
                let count = 257 - length as usize;
                let Some(byte) = bytes.get(at).copied() else {
                    return Err(Error::Malformed("the run has no byte".to_string()));
                };
                at += 1;
                out.extend(std::iter::repeat_n(byte, count));
            }
        }
        if out.len() > limits.max_output {
            return Err(Error::TooLong { limit: limits.max_output });
        }
    }
    Ok(out)
}

/// LZW, with the early change, undone.
///
/// The codes are **nine bits** at first and grow to twelve, and they
/// grow one entry before the table is full: that is the difference
/// between the PDF's LZW and the one in a GIF, and a decoder that does
/// it the other way reads the wrong bytes.
fn lzw(bytes: &[u8], limits: &Limits) -> Result<Vec<u8>, Error> {
    let mut out: Vec<u8> = Vec::new();
    let mut table: Vec<Vec<u8>> = (0..=255_u8).map(|byte| vec![byte]).collect();
    // 256 is the **clear** code and 257 is the **end** code.
    table.push(Vec::new());
    table.push(Vec::new());
    let mut width = 9_usize;
    let mut previous: Option<u16> = None;
    let mut bits = Bits::new(bytes);
    loop {
        let code = bits.code(width)?;
        if code == 256 {
            table.truncate(258);
            width = 9;
            previous = None;
            continue;
        }
        if code == 257 {
            break;
        }
        let entry: Vec<u8> = if (code as usize) < table.len() {
            table[code as usize].clone()
        } else if let Some(previous) = previous {
            // The code is the one **after** the last: the entry is the
            // last one with its own first byte at the end.
            let mut entry = table[previous as usize].clone();
            let Some(first) = entry.first().copied() else {
                return Err(Error::Malformed("the table's entry is empty".to_string()));
            };
            entry.push(first);
            entry
        } else {
            return Err(Error::Malformed("the first code is not in the table".to_string()));
        };
        if out.len() + entry.len() > limits.max_output {
            return Err(Error::TooLong { limit: limits.max_output });
        }
        out.extend_from_slice(&entry);
        if let Some(previous) = previous {
            let mut new_entry = table[previous as usize].clone();
            let Some(first) = entry.first().copied() else {
                return Err(Error::Malformed("the table's entry is empty".to_string()));
            };
            new_entry.push(first);
            table.push(new_entry);
            // The **early change**: the code grows one entry before the
            // table is full.
            if table.len() + 1 >= (1 << width) && width < 12 {
                width += 1;
            }
        }
        previous = Some(code);
    }
    Ok(out)
}

/// The bits of a stream, read **most significant first**.
struct Bits<'a> {
    bytes: &'a [u8],
    at: usize,
    held: u32,
    left: usize,
}

impl<'a> Bits<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Bits { bytes, at: 0, held: 0, left: 0 }
    }

    /// The next code, of the given width.
    fn code(&mut self, width: usize) -> Result<u16, Error> {
        while self.left < width {
            let Some(byte) = self.bytes.get(self.at).copied() else {
                return Err(Error::Malformed("the stream ends in the middle of a code".to_string()));
            };
            self.at += 1;
            self.held = (self.held << 8) | u32::from(byte);
            self.left += 8;
        }
        self.left -= width;
        let code = ((self.held >> self.left) & ((1 << width) - 1)) as u16;
        self.held &= (1 << self.left) - 1;
        Ok(code)
    }
}
