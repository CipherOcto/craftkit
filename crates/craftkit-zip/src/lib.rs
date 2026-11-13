//! `craftkit-zip` — the **package**: a ZIP archive, read.
//!
//! # What this is
//!
//! A package is a **container**: a set of named parts, each of them
//! bytes, held in one file. The format is a ZIP archive: a directory
//! at the end, and the parts themselves before it.
//!
//! This crate reads one: it finds the directory, it reads a part, and
//! it gives back the bytes — uncompressed, and checked.
//!
//! # Why it is a library and not a plugin
//!
//! Because a package is a package: the reader reads bytes and gives
//! back bytes, and it knows nothing about a document, a canvas or the
//! kit. A workbook reader (§904) is what puts a workbook into a
//! document, and it reads the package with this.
//!
//! # The file is hostile
//!
//! A package came from somewhere else, so every number in it is a
//! number that could be anything. The reader **refuses** what it
//! cannot make sense of, and it refuses **within limits**: a part that
//! says it holds half a gigabyte is not read, because the reading
//! would be the problem, and not the file. The limits are the tree's
//! own: **512 MB** for one part, **1 GB** for the whole package, and
//! **200 000** entries in the directory.
//!
//! # What this does not do
//!
//! - It does not **write** a package. A writer is a different program.
//! - It does not read a package that is **encrypted**: the entries of
//!   an encrypted package are refused with the reason.
//! - It does not read the parts of a package as XML or as anything
//!   else: it gives back bytes, and what they mean is the caller's.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

use std::collections::BTreeMap;

/// The largest part a package may hold.
pub const MAX_PART: u64 = 512 * 1024 * 1024;

/// The largest total a package may inflate to.
pub const MAX_TOTAL: u64 = 1024 * 1024 * 1024;

/// The most entries the directory of a package may hold.
pub const MAX_ENTRIES: usize = 200_000;

/// The signature of the **end of central directory** record.
const EOCD: [u8; 4] = [0x50, 0x4b, 0x05, 0x06];

/// The signature of a **central directory** entry.
const CENTRAL: [u8; 4] = [0x50, 0x4b, 0x01, 0x02];

/// The signature of a **local file header**.
const LOCAL: [u8; 4] = [0x50, 0x4b, 0x03, 0x04];

/// The **stored** method: the part is in the package as it is.
const STORED: u16 = 0;

/// The **deflated** method: the part is a DEFLATE stream.
const DEFLATED: u16 = 8;

/// Why a package could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The bytes are not a package, or the package is broken.
    Malformed(String),
    /// The package uses something this reader does not.
    Unsupported(String),
    /// The package asks for more than the caller allowed.
    TooLarge(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Malformed(message) => write!(f, "malformed package: {message}"),
            Error::Unsupported(message) => write!(f, "{message}"),
            Error::TooLarge(message) => write!(f, "limit exceeded: {message}"),
        }
    }
}

impl std::error::Error for Error {}

/// A **part** of a package, as the directory says it is.
#[derive(Debug, Clone)]
struct Entry {
    /// The name of the part.
    name: String,
    /// How the part is held: stored or deflated.
    method: u16,
    /// The checksum of the part's bytes.
    crc32: u32,
    /// How many bytes the part holds in the package.
    compressed: u64,
    /// How many bytes the part is.
    size: u64,
    /// Where the part's own header is.
    offset: u64,
}

/// A **package**: a set of named parts.
#[derive(Debug)]
pub struct Package<'a> {
    /// The bytes of the package.
    bytes: &'a [u8],
    /// The directory, in the order the package holds it.
    entries: Vec<Entry>,
    /// The names of the parts, normalised, and where each one is.
    names: BTreeMap<String, usize>,
    /// How much of the package's allowance has been spent.
    spent: u64,
}

/// The name of a part, as the package is asked for it.
///
/// The names of a package are not the names of a file system: a part
/// may be written `xl\workbook.xml` or `/xl/workbook.xml`, and it is
/// the same part. So the name is **normalised**: the slashes are
/// slashes, the leading slash is nothing, and the case is not the
/// case.
pub fn normalize(name: &str) -> String {
    name.replace('\\', "/").trim_start_matches('/').to_ascii_lowercase()
}

impl<'a> Package<'a> {
    /// Open a package of those bytes.
    pub fn open(bytes: &'a [u8]) -> Result<Package<'a>, Error> {
        let (entries, directory) = read_directory(bytes)?;
        let mut names = BTreeMap::new();
        for (index, entry) in entries.iter().enumerate() {
            names.entry(normalize(&entry.name)).or_insert(index);
        }
        let _ = directory;
        Ok(Package { bytes, entries, names, spent: 0 })
    }

    /// Whether the package has a part of that name.
    pub fn has(&self, part: &str) -> bool {
        self.names.contains_key(&normalize(part))
    }

    /// The **names** of the parts, in the order the package holds them.
    pub fn names(&self) -> Vec<&str> {
        self.entries.iter().map(|entry| entry.name.as_str()).collect()
    }

    /// Read a part of the package.
    ///
    /// A part that is not there is `None`, and not an error: a
    /// package may hold parts that are optional.
    pub fn read(&mut self, part: &str) -> Result<Option<Vec<u8>>, Error> {
        self.read_limited(part, MAX_PART)
    }

    /// Read a part of the package, and let it be no bigger than
    /// `maximum`.
    pub fn read_limited(&mut self, part: &str, maximum: u64) -> Result<Option<Vec<u8>>, Error> {
        let maximum = maximum.min(MAX_PART);
        let Some(index) = self.names.get(&normalize(part)).copied() else {
            return Ok(None);
        };
        let entry = self.entries.get(index).cloned().ok_or_else(|| {
            Error::Malformed(format!("the directory names a part that is not in it: {part}"))
        })?;
        if entry.size > maximum {
            return Err(Error::TooLarge(format!("{part} expands to {} bytes", entry.size)));
        }
        let left = MAX_TOTAL.saturating_sub(self.spent);
        let limit = maximum.min(left);
        let bytes = self.bytes_of(&entry, part, limit)?;
        self.spent += bytes.len() as u64;
        Ok(Some(bytes))
    }

    /// The **bytes** of a part, as the package holds them.
    fn bytes_of(&self, entry: &Entry, part: &str, limit: u64) -> Result<Vec<u8>, Error> {
        let at = usize::try_from(entry.offset)
            .map_err(|_| Error::Malformed(format!("{part}: the offset is out of range")))?;
        let header = self
            .bytes
            .get(at..at + 30)
            .ok_or_else(|| Error::Malformed(format!("{part}: the header is not in the package")))?;
        if header[..4] != LOCAL {
            return Err(Error::Malformed(format!("{part}: the header is not a header")));
        }
        // The **checksum** is in the header of the part and in the
        // directory, and the two must be the same: a package whose
        // two numbers differ is a package that was written badly.
        let local_crc = u32_at(header, 14);
        if local_crc != entry.crc32 {
            return Err(Error::Malformed(format!("{part}: the checksums do not agree")));
        }
        let name_length = u16_at(header, 26) as usize;
        let extra_length = u16_at(header, 28) as usize;
        let start = at + 30 + name_length + extra_length;
        let length = usize::try_from(entry.compressed)
            .map_err(|_| Error::Malformed(format!("{part}: the size is out of range")))?;
        let data = self
            .bytes
            .get(start..start + length)
            .ok_or_else(|| Error::Malformed(format!("{part}: the data is not in the package")))?;
        if data.len() as u64 > limit {
            return Err(Error::TooLarge(format!("{part} expands to more than {limit} bytes")));
        }
        let out = match entry.method {
            STORED => data.to_vec(),
            DEFLATED => {
                craftkit_deflate::deflate(data, &craftkit_deflate::Limits { max_output: limit as usize })
                    .map_err(|error| Error::Malformed(format!("{part}: {error}")))?
            }
            other => {
                return Err(Error::Unsupported(format!(
                    "{part}: the method {other} is not one this reader has"
                )));
            }
        };
        if crc32(&out) != entry.crc32 {
            return Err(Error::Malformed(format!("{part}: the checksum does not match")));
        }
        Ok(out)
    }
}

/// The **directory** of a package: the entries, and where it is.
fn read_directory(bytes: &[u8]) -> Result<(Vec<Entry>, u64), Error> {
    let at = find_end_of_directory(bytes)?;
    let record = bytes
        .get(at..at + 22)
        .ok_or_else(|| Error::Malformed("the end of the directory is not in the package".to_string()))?;
    let count = u16_at(record, 10) as usize;
    let size = u32_at(record, 12) as u64;
    let offset = u32_at(record, 16) as u64;
    if count > MAX_ENTRIES {
        return Err(Error::TooLarge(format!("{count} entries in the directory")));
    }
    // A package of more than sixty-five thousand entries, or of more
    // than four gigabytes, keeps its real numbers in the **ZIP64**
    // record — and this reader does not read that one.
    if count == u16::MAX as usize || offset == u32::MAX as u64 || size == u32::MAX as u64 {
        return Err(Error::Unsupported("not supported yet: a package that is ZIP64".to_string()));
    }
    let mut entries = Vec::with_capacity(count);
    let mut at =
        usize::try_from(offset).map_err(|_| Error::Malformed("the directory is out of range".to_string()))?;
    for index in 0..count {
        let header = bytes
            .get(at..at + 46)
            .ok_or_else(|| Error::Malformed("the directory is not in the package".to_string()))?;
        if header[..4] != CENTRAL {
            return Err(Error::Malformed("the directory is not a directory".to_string()));
        }
        let flags = u16_at(header, 8);
        let method = u16_at(header, 10);
        let crc = u32_at(header, 16);
        let compressed = u32_at(header, 20) as u64;
        let size = u32_at(header, 24) as u64;
        let name_length = u16_at(header, 28) as usize;
        let extra_length = u16_at(header, 30) as usize;
        let comment_length = u16_at(header, 32) as usize;
        let local = u32_at(header, 42) as u64;
        let name_at = at + 46;
        let name_bytes = bytes
            .get(name_at..name_at + name_length)
            .ok_or_else(|| Error::Malformed("a name is not in the package".to_string()))?;
        let name = String::from_utf8_lossy(name_bytes).into_owned();
        let extra = bytes.get(name_at + name_length..name_at + name_length + extra_length).unwrap_or(&[]);
        let (compressed, size, local) = zip64(extra, compressed, size, local)?;
        if flags & 0x0001 != 0 {
            return Err(Error::Unsupported(format!("{name}: not supported yet: an encrypted part")));
        }
        entries.push(Entry { name, method, crc32: crc, compressed, size, offset: local });
        at = name_at + name_length + extra_length + comment_length;
        let _ = index;
    }
    Ok((entries, size))
}

/// The **ZIP64** numbers of an entry, when it keeps them in its extra
/// field.
///
/// The extra field of a ZIP64 entry holds, in order and only for the
/// numbers that are all ones in the directory, the size of the part,
/// the size it holds, and where its own header is.
fn zip64(extra: &[u8], compressed: u64, size: u64, offset: u64) -> Result<(u64, u64, u64), Error> {
    let mut at = 0usize;
    while at + 4 <= extra.len() {
        let kind = u16_at(extra, at);
        let length = u16_at(extra, at + 2) as usize;
        let start = at + 4;
        let Some(body) = extra.get(start..start + length) else {
            return Err(Error::Malformed("an extra field is not in the package".to_string()));
        };
        if kind == 0x0001 {
            let mut read = 0usize;
            let mut take = |want: u64| -> Result<u64, Error> {
                let value = u64_at(body, read).ok_or_else(|| {
                    Error::Malformed("the ZIP64 numbers are not in the package".to_string())
                })?;
                read += 8;
                let _ = want;
                Ok(value)
            };
            let compressed = if compressed == u32::MAX as u64 { take(0)? } else { compressed };
            let size = if size == u32::MAX as u64 { take(0)? } else { size };
            let offset = if offset == u32::MAX as u64 { take(0)? } else { offset };
            return Ok((compressed, size, offset));
        }
        at = start + length;
    }
    Ok((compressed, size, offset))
}

/// Where the **end of the directory** is.
fn find_end_of_directory(bytes: &[u8]) -> Result<usize, Error> {
    // The record is at the end, and a comment of up to sixty-five
    // thousand bytes may follow it.
    let longest = 22 + u16::MAX as usize;
    let start = bytes.len().saturating_sub(longest);
    let mut at = bytes
        .len()
        .checked_sub(22)
        .ok_or_else(|| Error::Malformed("the package is too short to hold a directory".to_string()))?;
    loop {
        if bytes.get(at..at + 4) == Some(EOCD.as_slice()) {
            return Ok(at);
        }
        if at == start {
            return Err(Error::Malformed("the package has no directory".to_string()));
        }
        at -= 1;
    }
}

/// The **CRC-32** of some bytes: the checksum a package carries.
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = if crc & 1 != 0 { 0xEDB8_8320 } else { 0 };
            crc = (crc >> 1) ^ mask;
        }
    }
    !crc
}

/// The two bytes of the bytes at that place.
fn u16_at(bytes: &[u8], at: usize) -> u16 {
    match bytes.get(at..at + 2) {
        Some(pair) => u16::from_le_bytes([pair[0], pair[1]]),
        None => 0,
    }
}

/// The four bytes of the bytes at that place.
fn u32_at(bytes: &[u8], at: usize) -> u32 {
    match bytes.get(at..at + 4) {
        Some(quad) => u32::from_le_bytes([quad[0], quad[1], quad[2], quad[3]]),
        None => 0,
    }
}

/// The eight bytes of the bytes at that place.
fn u64_at(bytes: &[u8], at: usize) -> Option<u64> {
    let quad = bytes.get(at..at + 8)?;
    Some(u64::from_le_bytes([quad[0], quad[1], quad[2], quad[3], quad[4], quad[5], quad[6], quad[7]]))
}
