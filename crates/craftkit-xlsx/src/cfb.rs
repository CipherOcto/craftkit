//! A **compound file**: the container a workbook that is encrypted —
//! or a workbook of the old format — is held in.
//!
//! A compound file is a small file system in one file. It has a
//! **header**, a **table of contents** — the directory — and two
//! **tables of allocation**: the one for the sectors of the file, and
//! the one for the small sectors of the *mini stream*, which is where
//! the small streams live.
//!
//! This module reads one: it opens the file, it reads the directory,
//! and it gives back the bytes of a named stream.
//!
//! The format is the one the **[MS-CFB]** specification describes, and
//! the code here is written from that specification and not from any
//! other program.
//!
//! # The file is hostile
//!
//! Every number in the file is a number that could be anything: a
//! sector that is out of the file, a chain that goes on for ever, a
//! size that is a gigabyte. The reader refuses what it cannot make
//! sense of, and it refuses **within limits**: the file may hold
//! **64 000** sectors of allocation, and a stream of more than
//! **512 MB** is not read.

/// The **signature** of a compound file.
const SIGNATURE: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];

/// The **header** of a compound file: 512 bytes.
const HEADER: usize = 512;

/// The size of a **directory entry**: 128 bytes.
const ENTRY: usize = 128;

/// The most **sectors of allocation** the reader will follow.
const MAX_SECTORS: usize = 64_000;

/// The most bytes of one **stream** the reader will read.
const MAX_STREAM: u64 = 512 * 1024 * 1024;

/// The kinds of a directory **entry**.
mod kind {
    /// An **empty** place in the directory.
    pub const EMPTY: u8 = 0;
    /// A **stream**: bytes of their own.
    pub const STREAM: u8 = 2;
    /// The **root**: the storage that holds the mini stream.
    pub const ROOT: u8 = 5;
}

/// One **entry** of the directory.
#[derive(Debug, Clone)]
struct Entry {
    /// The **name** of the entry.
    name: String,
    /// What the entry **is**.
    kind: u8,
    /// The **first sector** of the entry's stream.
    start: u32,
    /// The **size** of the entry's stream, in bytes.
    size: u64,
}

/// Why a compound file could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    /// What went wrong.
    pub message: String,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for Error {}

/// The **error** of a compound file.
fn error(message: &str) -> Error {
    Error { message: message.to_string() }
}

/// A **compound file**, opened.
#[derive(Debug, Clone)]
pub struct Compound {
    /// The bytes of the file.
    bytes: Vec<u8>,
    /// The size of a **sector**, in bytes.
    sector: usize,
    /// The size of a **mini sector**, in bytes.
    mini_sector: usize,
    /// The size below which a stream lives in the **mini stream**.
    cutoff: u64,
    /// The **table of allocation** of the sectors.
    fat: Vec<u32>,
    /// The **table of allocation** of the mini sectors.
    mini_fat: Vec<u32>,
    /// The **directory**: the entries of the file.
    directory: Vec<Entry>,
    /// The **mini stream**: where the small streams live.
    mini_stream: Vec<u8>,
}

impl Compound {
    /// Open a compound file.
    pub fn open(bytes: &[u8]) -> Result<Compound, Error> {
        if bytes.len() < HEADER {
            return Err(error("the file is too short to be a compound file"));
        }
        if bytes[..8] != SIGNATURE {
            return Err(error("the file is not a compound file"));
        }
        let shift = read_u16(bytes, 30)?;
        let sector = match shift {
            9 => 512,
            12 => 4096,
            _ => return Err(error("the sector size of the file is not one this reader knows")),
        };
        let mini_shift = read_u16(bytes, 32)?;
        let mini_sector = match mini_shift {
            6 => 64,
            _ => return Err(error("the mini sector size of the file is not one this reader knows")),
        };
        let fat_sectors = read_u32(bytes, 44)? as usize;
        let first_directory = read_u32(bytes, 48)?;
        let cutoff = read_u32(bytes, 56)? as u64;
        let first_mini_fat = read_u32(bytes, 60)?;
        let mini_fat_sectors = read_u32(bytes, 64)? as usize;
        let first_difat = read_u32(bytes, 68)?;
        let difat_sectors = read_u32(bytes, 72)? as usize;
        if fat_sectors > MAX_SECTORS || mini_fat_sectors > MAX_SECTORS || difat_sectors > MAX_SECTORS {
            return Err(error("the file says it holds more tables than this reader reads"));
        }
        let mut compound = Compound {
            bytes: bytes.to_vec(),
            sector,
            mini_sector,
            cutoff,
            fat: Vec::new(),
            mini_fat: Vec::new(),
            directory: Vec::new(),
            mini_stream: Vec::new(),
        };
        compound.fat = compound.read_fat(fat_sectors, first_difat, difat_sectors)?;
        let directory = compound.read_chain(first_directory, u64::MAX)?;
        compound.directory = compound.entries_of(&directory)?;
        let mini_fat_bytes = compound.read_chain(first_mini_fat, u64::MAX)?;
        compound.mini_fat = compound.u32s_of(&mini_fat_bytes);
        let root = compound.directory.first().ok_or_else(|| error("the file has no directory"))?.clone();
        if root.kind == kind::ROOT {
            let mini = compound.read_chain(root.start, root.size)?;
            compound.mini_stream = mini;
        }
        Ok(compound)
    }

    /// The **names** of the streams the file holds.
    pub fn names(&self) -> Vec<String> {
        self.directory
            .iter()
            .filter(|entry| entry.kind == kind::STREAM)
            .map(|entry| entry.name.clone())
            .collect()
    }

    /// The bytes of the **stream** of that name.
    ///
    /// `None` says that the file holds no stream of that name.
    pub fn read(&self, name: &str) -> Result<Option<Vec<u8>>, Error> {
        let Some(entry) =
            self.directory.iter().find(|entry| entry.kind == kind::STREAM && entry.name == name)
        else {
            return Ok(None);
        };
        if entry.size > MAX_STREAM {
            return Err(error("the stream says it holds more than this reader reads"));
        }
        let bytes = if entry.size < self.cutoff {
            self.read_mini(entry.start, entry.size)?
        } else {
            // The chain of the sectors may hold more than the stream
            // says: the last sector is padded, and the size of the
            // stream is the size of the stream.
            let mut bytes = self.read_chain(entry.start, entry.size)?;
            bytes.truncate(entry.size as usize);
            bytes
        };
        Ok(Some(bytes))
    }

    /// The **table of allocation** of the sectors: the chain of every
    /// sector of the file.
    fn read_fat(
        &self,
        fat_sectors: usize,
        first_difat: u32,
        difat_sectors: usize,
    ) -> Result<Vec<u32>, Error> {
        // The **DIFAT**: the first 109 sectors of the table are named
        // in the header, and the rest are named in the sectors the
        // header points at.
        let mut sectors: Vec<u32> = Vec::new();
        for index in 0..109 {
            let at = 76 + index * 4;
            let Ok(sector) = read_u32(&self.bytes, at) else { break };
            if sector >= MAX_SECTORS as u32 {
                break;
            }
            sectors.push(sector);
        }
        let mut next = first_difat;
        for _ in 0..difat_sectors {
            if next >= MAX_SECTORS as u32 {
                break;
            }
            let Some(sector) = self.sector_bytes(next) else { break };
            let per = self.sector / 4 - 1;
            for index in 0..per {
                let at = index * 4;
                let Ok(value) = read_u32(sector, at) else { break };
                if value >= MAX_SECTORS as u32 {
                    break;
                }
                sectors.push(value);
            }
            let Ok(after) = read_u32(sector, per * 4) else { break };
            next = after;
        }
        sectors.truncate(fat_sectors);
        let mut fat: Vec<u32> = Vec::new();
        for sector in &sectors {
            let Some(bytes) = self.sector_bytes(*sector) else { continue };
            fat.extend(self.u32s_of(bytes));
        }
        Ok(fat)
    }

    /// The **chain of sectors** that starts at that sector: the bytes
    /// of every sector of it, one after another.
    fn read_chain(&self, start: u32, limit: u64) -> Result<Vec<u8>, Error> {
        let mut bytes: Vec<u8> = Vec::new();
        let mut sector = start;
        let mut seen = 0usize;
        while sector < MAX_SECTORS as u32 {
            let Some(part) = self.sector_bytes(sector) else { break };
            bytes.extend_from_slice(part);
            seen += 1;
            if seen > MAX_SECTORS || bytes.len() as u64 > limit {
                break;
            }
            let Some(next) = self.fat.get(sector as usize).copied() else { break };
            if next >= MAX_SECTORS as u32 {
                break;
            }
            sector = next;
        }
        Ok(bytes)
    }

    /// The bytes of one **sector**.
    fn sector_bytes(&self, sector: u32) -> Option<&[u8]> {
        let start = (sector as usize + 1).checked_mul(self.sector)?;
        self.bytes.get(start..start.checked_add(self.sector)?)
    }

    /// The **entries** of the directory: the 128 bytes of each, one
    /// after another.
    fn entries_of(&self, bytes: &[u8]) -> Result<Vec<Entry>, Error> {
        let mut entries: Vec<Entry> = Vec::new();
        let mut at = 0;
        while at + ENTRY <= bytes.len() {
            let chunk = &bytes[at..at + ENTRY];
            at += ENTRY;
            let kind = chunk[66];
            if kind == kind::EMPTY {
                continue;
            }
            let name_length = read_u16(chunk, 64)? as usize;
            let name_length = name_length.min(64);
            let name: String = chunk[..name_length]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| {
                    let unit = u16::from_le_bytes([pair[0], pair[1]]);
                    char::from_u32(unit as u32).unwrap_or('\u{fffd}')
                })
                .collect();
            let name = name.trim_end_matches('\u{0}').to_string();
            let start = read_u32(chunk, 116)?;
            let size = read_u64(chunk, 120)?;
            entries.push(Entry { name, kind, start, size });
        }
        Ok(entries)
    }

    /// The **mini sectors** of a stream: the small sectors of the mini
    /// stream, one after another.
    fn read_mini(&self, start: u32, size: u64) -> Result<Vec<u8>, Error> {
        let mut bytes: Vec<u8> = Vec::new();
        let mut sector = start;
        let mut seen = 0usize;
        while sector < MAX_SECTORS as u32 && (bytes.len() as u64) < size {
            let at = sector as usize * self.mini_sector;
            let Some(part) = self.mini_stream.get(at..at + self.mini_sector) else { break };
            bytes.extend_from_slice(part);
            seen += 1;
            if seen > MAX_SECTORS {
                break;
            }
            let Some(next) = self.mini_fat.get(sector as usize).copied() else { break };
            if next >= MAX_SECTORS as u32 {
                break;
            }
            sector = next;
        }
        bytes.truncate(size as usize);
        Ok(bytes)
    }

    /// The bytes of a slice, read as the **numbers** they hold.
    fn u32s_of(&self, bytes: &[u8]) -> Vec<u32> {
        bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect()
    }
}

/// The **number** at that place, in two bytes.
fn read_u16(bytes: &[u8], at: usize) -> Result<u16, Error> {
    let chunk = bytes.get(at..at + 2).ok_or_else(|| error("the file is too short"))?;
    Ok(u16::from_le_bytes([chunk[0], chunk[1]]))
}

/// The **number** at that place, in four bytes.
fn read_u32(bytes: &[u8], at: usize) -> Result<u32, Error> {
    let chunk = bytes.get(at..at + 4).ok_or_else(|| error("the file is too short"))?;
    Ok(u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
}

/// The **number** at that place, in eight bytes.
fn read_u64(bytes: &[u8], at: usize) -> Result<u64, Error> {
    let chunk = bytes.get(at..at + 8).ok_or_else(|| error("the file is too short"))?;
    let mut value = [0u8; 8];
    value.copy_from_slice(chunk);
    Ok(u64::from_le_bytes(value))
}
