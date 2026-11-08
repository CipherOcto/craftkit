//! The **captions** of a **SubRip** file.
//!
//! A caption file is a list of **cues**: a piece of text and the two
//! times it is on the screen — when it comes and when it goes. The
//! **SubRip** format (`.srt`) is one of the tree's caption formats,
//! and this is its reader.
//!
//! ```text
//! 1
//! 00:00:01,000 --> 00:00:03,500
//! First line
//! <i>second line</i>
//!
//! 2
//! …
//! ```
//!
//! # The rules
//!
//! These are the tree's own rules for reading a SubRip file, and they
//! are the rules of a reader that takes what real files contain.
//!
//! - The **timing line** of a cue is `start --> end`, and anything
//!   after the end is the settings of the cue. The clock is
//!   `HH:MM:SS,mmm` or `MM:SS,mmm` — the hours may be left out — and
//!   the mark may be a comma or a point. The fraction may be one to
//!   nine digits long, the minutes and the seconds are below sixty,
//!   and the hours are at most 9999.
//! - The **index** of a cue is a line of digits, and it may be left
//!   out. A cue begins at a timing line, or at an index line that is
//!   followed by a timing line.
//! - The **text** of a cue runs to the end of the line, the blank
//!   line, the next timing line, or the next index line that is
//!   followed by a timing line.
//! - A cue whose **end is not after its start** is skipped, and the
//!   answer says so.
//! - A file that has **no cue** and holds something is not a SubRip
//!   file, and that is refused by name. A file that holds nothing is
//!   a file with no cues.
//! - The **text** of the file is read as UTF-8 (with a mark of the
//!   byte order taken off), as UTF-16 with a mark of the byte order,
//!   or — when it is neither — as Windows-1252. The tree's own rule:
//!   the reader of a caption file takes what real files contain.
//!
//! # The unit
//!
//! A cue is a pair of **milliseconds**, the unit of the format
//! itself. The tree keeps the fraction of a clock exactly, in ticks;
//! here a fraction finer than a millisecond is rounded to the
//! nearest millisecond.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Outcome, Session, Text};

/// The most a caption file may be: sixteen megabytes.
const MAX_BYTES: u64 = 16 * 1024 * 1024;

/// The most a **clock** may say: the hours are at most this.
const MAX_HOURS: i64 = 9999;

/// The most **digits** a part of a clock may have.
const MAX_PART: usize = 9;

/// A **cue**: the text of it, and when it is on the screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cue {
    /// When the cue comes, in milliseconds.
    pub start: i64,
    /// When the cue goes, in milliseconds.
    pub end: i64,
    /// The text of the cue.
    pub text: String,
}

/// The **text** of a file of bytes, as the tree's reader has it.
///
/// A mark of the byte order at the front means UTF-8 or UTF-16, and
/// the mark is taken off. A file that is neither is read as
/// Windows-1252, which is what a file of a Western European office
/// holds.
pub fn decode(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(&bytes[3..]).into_owned();
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        let units: Vec<u16> =
            bytes[2..].as_chunks::<2>().0.iter().map(|pair| u16::from_le_bytes([pair[0], pair[1]])).collect();
        return String::from_utf16_lossy(&units);
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let units: Vec<u16> =
            bytes[2..].as_chunks::<2>().0.iter().map(|pair| u16::from_be_bytes([pair[0], pair[1]])).collect();
        return String::from_utf16_lossy(&units);
    }
    match String::from_utf8(bytes.to_vec()) {
        Ok(text) => text,
        Err(_) => bytes.iter().map(|byte| windows1252(*byte)).collect(),
    }
}

/// The **character** a byte of Windows-1252 stands for.
///
/// The bytes below 0xA0 are the Latin ones, and the bytes from 0x80
/// to 0x9F are the ones Windows-1252 has of its own.
fn windows1252(byte: u8) -> char {
    const HIGH: [char; 32] = [
        '\u{20AC}', '\u{81}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}',
        '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{8D}', '\u{017D}', '\u{8F}', '\u{90}',
        '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}', '\u{02DC}',
        '\u{2122}', '\u{0161}', '\u{203A}', '\u{0153}', '\u{9D}', '\u{017E}', '\u{0178}',
    ];
    match byte {
        0x80..=0x9F => HIGH[(byte - 0x80) as usize],
        0xA0..=0xFF => char::from_u32(u32::from(byte)).unwrap_or('\u{FFFD}'),
        _ => char::from_u32(u32::from(byte)).unwrap_or('\u{FFFD}'),
    }
}

/// The **milliseconds** a clock says, when it says one.
///
/// The clock is `HH:MM:SS,mmm` or `MM:SS,mmm`, the hours may be
/// left out, the mark may be a comma or a point, and the fraction may
/// be one to nine digits long. The minutes and the seconds are below
/// sixty and the hours are at most 9999.
fn clock_of(text: &str) -> Option<i64> {
    let text = text.trim();
    let (hms, fraction) = match text.find([',', '.']) {
        Some(at) => (&text[..at], &text[at + 1..]),
        None => (text, ""),
    };
    let number = |part: &str| -> Option<i64> {
        if part.is_empty() || part.len() > MAX_PART || !part.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        part.parse::<i64>().ok()
    };
    let (hours, minutes, seconds) = match hms.split(':').collect::<Vec<_>>().as_slice() {
        [hours, minutes, seconds] => (number(hours)?, number(minutes)?, number(seconds)?),
        [minutes, seconds] => (0, number(minutes)?, number(seconds)?),
        _ => return None,
    };
    if minutes >= 60 || seconds >= 60 || hours > MAX_HOURS {
        return None;
    }
    let mut milliseconds = ((hours * 60 + minutes) * 60 + seconds) * 1000;
    if !fraction.is_empty() {
        if fraction.len() > MAX_PART || !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        // The **fraction** is a number of digits after the mark, and
        // it is scaled to the millisecond and rounded to the nearest
        // one.
        let scale = 10i64.pow(fraction.len() as u32);
        let digits = fraction.parse::<i64>().ok()?;
        milliseconds += (digits * 1000 + scale / 2) / scale;
    }
    Some(milliseconds)
}

/// The **clock** a number of milliseconds is written as.
///
/// The tree's own shape: `HH:MM:SS,mmm`.
pub fn format_clock(milliseconds: i64) -> String {
    let milliseconds = milliseconds.max(0);
    let seconds = milliseconds / 1000;
    let millis = milliseconds % 1000;
    let hours = seconds / 3600;
    let minutes = (seconds / 60) % 60;
    let secs = seconds % 60;
    format!("{hours:02}:{minutes:02}:{secs:02},{millis:03}")
}

/// Whether a line is the **index** of a cue: digits, and at most ten
/// of them.
fn is_index(line: &str) -> bool {
    let line = line.trim();
    !line.is_empty() && line.len() <= 10 && line.bytes().all(|byte| byte.is_ascii_digit())
}

/// The **timing line** of a cue: the start, the end and the settings.
fn timing_of(line: &str) -> Option<(i64, i64)> {
    let (start, rest) = line.split_once("-->")?;
    let start = clock_of(start)?;
    let rest = rest.trim_start();
    let end_len = rest.find(char::is_whitespace).unwrap_or(rest.len());
    let end = clock_of(&rest[..end_len])?;
    Some((start, end))
}

/// The **cues** of a SubRip file, and the warnings about it.
///
/// A file that holds no cue and holds something is not a SubRip file,
/// and that is an error. A file that holds nothing is a file with no
/// cues.
pub fn cues_of(text: &str) -> Result<(Vec<Cue>, Vec<String>), String> {
    // The **line endings**: a file may end its lines with a line
    // feed, with a carriage return and a line feed, or with a
    // carriage return alone. The tree's own rule.
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<&str> = normalized.lines().collect();
    let mut cues: Vec<Cue> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut at = 0;
    let mut found_any = false;
    while at < lines.len() {
        let Some((start, end)) = timing_of(lines[at]) else {
            at += 1;
            continue;
        };
        found_any = true;
        at += 1;
        // The **text** of the cue: the lines up to a blank line, a
        // timing line, or an index line that is followed by a timing
        // line.
        let mut text_lines: Vec<&str> = Vec::new();
        while at < lines.len() {
            let line = lines[at];
            if line.trim().is_empty() {
                break;
            }
            if timing_of(line).is_some() {
                break;
            }
            if is_index(line) && lines.get(at + 1).is_some_and(|next| timing_of(next).is_some()) {
                break;
            }
            text_lines.push(line.trim_end());
            at += 1;
        }
        if end <= start {
            warnings
                .push(format!("the cue at {} ends before it starts: it was skipped", format_clock(start)));
            continue;
        }
        cues.push(Cue { start, end, text: text_lines.join("\n") });
    }
    if !found_any && !normalized.trim().is_empty() {
        return Err("the file is not a SubRip file".to_string());
    }
    Ok((cues, warnings))
}

/// **Read** the **cues** of a SubRip file.
///
/// The cues are read out of the file, and they are not kept: a
/// caption file is a file, and what is said about it is said here.
#[derive(Command)]
#[command(id = "film.captions.read", area = "film", layer = "film", icon = "list")]
pub struct ReadCaptions {
    /// The file to read.
    pub path: Text,
}

impl ReadCaptions {
    fn run(&self, _session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.captions.read".to_string();
        let path = self.path.0.trim().to_string();
        if path.is_empty() {
            return Err(EngineError::Failed { id: id.clone(), message: "a file has a path".to_string() });
        }
        let bytes =
            craftkit_sdk::files::read_bounded(std::path::Path::new(&path), MAX_BYTES).map_err(|message| {
                EngineError::Failed { id: id.clone(), message: format!("the file cannot be read: {message}") }
            })?;
        let text = decode(&bytes);
        let (cues, warnings) = cues_of(&text).map_err(|message| EngineError::Failed {
            id: id.clone(),
            message: format!("{message}: `{path}`"),
        })?;
        let read: Vec<Json> = cues
            .iter()
            .map(|cue| {
                json!({
                    "start": cue.start,
                    "end": cue.end,
                    "startClock": format_clock(cue.start),
                    "endClock": format_clock(cue.end),
                    "text": cue.text,
                })
            })
            .collect();
        Ok(Outcome::new(format!("`{path}` holds {} cue(s)", cues.len())).with_data(json!({
            "path": path,
            "cues": read,
            "warnings": warnings,
            "count": cues.len(),
        })))
    }
}
