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
    format_clock_marked(milliseconds, ',')
}

/// The **clock** a number of milliseconds is written as, with the mark
/// of the format: a comma for SubRip and a point for WebVTT.
pub fn format_clock_marked(milliseconds: i64, mark: char) -> String {
    let milliseconds = milliseconds.max(0);
    let seconds = milliseconds / 1000;
    let millis = milliseconds % 1000;
    let hours = seconds / 3600;
    let minutes = (seconds / 60) % 60;
    let secs = seconds % 60;
    format!("{hours:02}:{minutes:02}:{secs:02}{mark}{millis:03}")
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

/// A **cue** of a WebVTT file: the text of it, the times, and what
/// else the file said about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VttCue {
    /// When the cue comes, in milliseconds.
    pub start: i64,
    /// When the cue goes, in milliseconds.
    pub end: i64,
    /// The **identifier** of the cue, when the file gave it one.
    pub id: Option<String>,
    /// The **settings** of the cue, as the file wrote them.
    pub settings: String,
    /// The **speaker** of the cue, when the text names one.
    pub speaker: Option<String>,
    /// The text of the cue.
    pub text: String,
}

/// The **speaker** of a cue, and the text without it.
///
/// The tree's own rule: a voice span at the front of the text is the
/// speaker and is taken off, and a span that is not at the front, or
/// that holds several voices, is left where it is.
fn split_voice(text: &str) -> (Option<String>, String) {
    let trimmed = text.trim_start();
    let Some(rest) = trimmed.strip_prefix("<v") else { return (None, text.to_string()) };
    if !(rest.starts_with(' ') || rest.starts_with('.') || rest.starts_with('\t')) {
        return (None, text.to_string());
    }
    let Some(close) = rest.find('>') else { return (None, text.to_string()) };
    // The **classes** of the voice are skipped: the name is what is
    // after the first space of the span.
    let inner = &rest[..close];
    let name = match inner.find([' ', '\t']) {
        Some(space) => inner[space..].trim(),
        None => "",
    };
    let body = &rest[close + 1..];
    // A **closing** span at the very end goes too; one in the middle
    // means several voices, and then there is no speaker.
    let body = match body.find("</v>") {
        Some(at) if at + 4 == body.trim_end().len() => body[..at].to_string(),
        Some(_) => return (None, text.to_string()),
        None => body.to_string(),
    };
    if body.contains("<v ") || body.contains("<v.") || body.contains("<v\t") {
        return (None, text.to_string());
    }
    (Some(name.to_string()).filter(|name| !name.is_empty()), body)
}

/// What a **WebVTT file** holds: its cues, the blocks it keeps and the
/// warnings about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VttDocument {
    /// The cues of the file.
    pub cues: Vec<VttCue>,
    /// The blocks the file keeps: the notes, the styles and the
    /// regions.
    pub blocks: Vec<String>,
    /// The warnings about the file.
    pub warnings: Vec<String>,
}

/// The **cues** of a WebVTT file, the **blocks** it keeps, and the
/// warnings about it.
///
/// The blocks are the `NOTE` blocks and the `STYLE` and `REGION`
/// blocks before the first cue: the tree keeps them, and so does
/// this.
pub fn vtt_cues(text: &str) -> Result<VttDocument, String> {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines = normalized.lines().peekable();
    // The **signature** of the file, and the header that may follow
    // it. A file with no signature is read anyway.
    let has_signature = lines.peek().is_some_and(|line| {
        let line = line.trim_start_matches('\u{feff}');
        line == "WEBVTT" || line.starts_with("WEBVTT ") || line.starts_with("WEBVTT\t")
    });
    if has_signature {
        for line in lines.by_ref() {
            if line.trim().is_empty() {
                break;
            }
        }
    }
    // The **blocks** of the file: the parts between the blank lines.
    let mut blocks: Vec<Vec<&str>> = Vec::new();
    let mut block: Vec<&str> = Vec::new();
    for line in lines {
        if line.trim().is_empty() {
            if !block.is_empty() {
                blocks.push(std::mem::take(&mut block));
            }
        } else {
            block.push(line);
        }
    }
    if !block.is_empty() {
        blocks.push(block);
    }
    let mut cues: Vec<VttCue> = Vec::new();
    let mut kept: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut seen_cue = false;
    for block in blocks {
        let first = block[0].trim();
        let has_timing = block.iter().any(|line| line.contains("-->"));
        // A **NOTE** block is kept, and so are the `STYLE` and
        // `REGION` blocks before the first cue.
        if !has_timing && (first == "NOTE" || first.starts_with("NOTE ") || first.starts_with("NOTE\t")) {
            kept.push(block.join("\n"));
            continue;
        }
        if !seen_cue && !has_timing && (first == "STYLE" || first == "REGION") {
            kept.push(block.join("\n"));
            continue;
        }
        // The **identifier** of the cue, when the file gave it one.
        let (id, timing_at) =
            if block[0].contains("-->") { (None, 0) } else { (Some(block[0].trim().to_string()), 1) };
        let Some(timing) = block.get(timing_at) else {
            warnings.push(format!("the block \"{first}\" was ignored"));
            continue;
        };
        let Some((start, end, settings)) = timing_with_settings(timing) else {
            warnings.push(format!("the timing line \"{}\" is not one", timing.trim()));
            continue;
        };
        seen_cue = true;
        if end <= start {
            warnings
                .push(format!("the cue at {} ends before it starts: it was skipped", format_clock(start)));
            continue;
        }
        let raw = block[timing_at + 1..].iter().map(|line| line.trim_end()).collect::<Vec<_>>().join("\n");
        let (speaker, text) = split_voice(&raw);
        cues.push(VttCue { start, end, id, settings, speaker, text });
    }
    if !has_signature && cues.is_empty() && !normalized.trim().is_empty() {
        return Err("the file is not a WebVTT file".to_string());
    }
    Ok(VttDocument { cues, blocks: kept, warnings })
}

/// The **timing line** of a cue, with the **settings** that follow it.
fn timing_with_settings(line: &str) -> Option<(i64, i64, String)> {
    let (start, rest) = line.split_once("-->")?;
    let start = clock_of(start)?;
    let rest = rest.trim_start();
    let end_len = rest.find(char::is_whitespace).unwrap_or(rest.len());
    let end = clock_of(&rest[..end_len])?;
    Some((start, end, rest[end_len..].trim().to_string()))
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

/// **Read** the **cues** of a caption file.
///
/// The cues are read out of the file, and they are not kept: a
/// caption file is a file, and what is said about it is said here.
///
/// The **format** of the file is named: `subrip` or `webvtt`. A format
/// that is not one of them is refused by name.
#[derive(Command)]
#[command(id = "film.captions.read", area = "film", layer = "film", icon = "list")]
pub struct ReadCaptions {
    /// The file to read.
    pub path: Text,
    /// The format of the file: `subrip` when it is not given.
    pub format: Option<Text>,
}

/// The **formats** of a caption file this tree reads.
pub const FORMATS: [&str; 2] = ["subrip", "webvtt"];

impl ReadCaptions {
    fn run(&self, _session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.captions.read".to_string();
        let path = self.path.0.trim().to_string();
        if path.is_empty() {
            return Err(EngineError::Failed { id: id.clone(), message: "a file has a path".to_string() });
        }
        // The **format** of the file: the tree's own names for the
        // formats, and the extension of a file is one of them too.
        let named = self.format.as_ref().map(|format| format.0.trim().to_ascii_lowercase());
        let format = match named.as_deref() {
            None | Some("") | Some("subrip") | Some("srt") => "subrip",
            Some("webvtt") | Some("vtt") => "webvtt",
            Some(other) => {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "`{other}` is not a format of a caption file: the formats are {}",
                        FORMATS.join(", ")
                    ),
                });
            }
        };
        let bytes =
            craftkit_sdk::files::read_bounded(std::path::Path::new(&path), MAX_BYTES).map_err(|message| {
                EngineError::Failed { id: id.clone(), message: format!("the file cannot be read: {message}") }
            })?;
        let text = decode(&bytes);
        let (cues, blocks, warnings) = if format == "webvtt" {
            let document = vtt_cues(&text).map_err(|message| EngineError::Failed {
                id: id.clone(),
                message: format!("{message}: `{path}`"),
            })?;
            let read: Vec<Json> = document
                .cues
                .iter()
                .map(|cue| {
                    json!({
                        "start": cue.start,
                        "end": cue.end,
                        "startClock": format_clock_marked(cue.start, '.'),
                        "endClock": format_clock_marked(cue.end, '.'),
                        "id": cue.id,
                        "speaker": cue.speaker,
                        "settings": cue.settings,
                        "text": cue.text,
                    })
                })
                .collect();
            (read, document.blocks, document.warnings)
        } else {
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
                        "id": Json::Null,
                        "speaker": Json::Null,
                        "settings": Json::Null,
                        "text": cue.text,
                    })
                })
                .collect();
            (read, Vec::new(), warnings)
        };
        Ok(Outcome::new(format!("`{path}` holds {} cue(s)", cues.len())).with_data(json!({
            "path": path,
            "format": format,
            "cues": cues,
            "blocks": blocks,
            "warnings": warnings,
            "count": cues.len(),
        })))
    }
}

/// The most **cues** a caption file may hold.
pub const MAX_CUES: usize = 10_000;

/// The most a **cue's text** may have.
pub const MAX_TEXT: usize = 4096;

/// The most a cue's **time** may be, in milliseconds: a year.
pub const MAX_MILLISECONDS: i64 = 31_536_000_000;

/// The **cues** a caller gives, and the error when they are not ones.
///
/// The cues come from outside, so they are read carefully: the times
/// are whole milliseconds, a cue ends after it starts, and the texts
/// are not longer than they may be.
fn cues_from(value: &Json, id: &str) -> Result<Vec<Cue>, EngineError> {
    let Some(list) = value.as_array() else {
        return Err(EngineError::Failed { id: id.to_string(), message: "the cues are a list".to_string() });
    };
    if list.len() > MAX_CUES {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there are {} cues, and the most there may be is {MAX_CUES}", list.len()),
        });
    }
    let mut cues = Vec::with_capacity(list.len());
    for (at, cue) in list.iter().enumerate() {
        let Some(fields) = cue.as_object() else {
            return Err(EngineError::Failed {
                id: id.to_string(),
                message: format!("the cue {at} is not a cue"),
            });
        };
        let millis = |name: &str| -> Result<i64, EngineError> {
            let Some(number) = fields.get(name).and_then(|number| number.as_i64()) else {
                return Err(EngineError::Failed {
                    id: id.to_string(),
                    message: format!("the cue {at} has no `{name}` in whole milliseconds"),
                });
            };
            if !(0..=MAX_MILLISECONDS).contains(&number) {
                return Err(EngineError::Failed {
                    id: id.to_string(),
                    message: format!("the `{name}` of the cue {at} is not between zero and a year"),
                });
            }
            Ok(number)
        };
        let start = millis("start")?;
        let end = millis("end")?;
        if end <= start {
            return Err(EngineError::Failed {
                id: id.to_string(),
                message: format!("the cue {at} ends before it starts"),
            });
        }
        let text = match fields.get("text") {
            Some(text) => {
                let Some(text) = text.as_str() else {
                    return Err(EngineError::Failed {
                        id: id.to_string(),
                        message: format!("the text of the cue {at} is not text"),
                    });
                };
                if text.chars().count() > MAX_TEXT {
                    return Err(EngineError::Failed {
                        id: id.to_string(),
                        message: format!("the text of the cue {at} is longer than {MAX_TEXT} characters"),
                    });
                }
                text.to_string()
            }
            None => String::new(),
        };
        cues.push(Cue { start, end, text });
    }
    Ok(cues)
}

/// The **cues** of a WebVTT file that a caller gives.
fn vtt_cues_from(value: &Json, id: &str) -> Result<Vec<VttCue>, EngineError> {
    let cues = cues_from(value, id)?;
    let mut out = Vec::with_capacity(cues.len());
    for (at, cue) in cues.iter().enumerate() {
        let Some(fields) = value.as_array().and_then(|list| list.get(at)).and_then(|cue| cue.as_object())
        else {
            return Err(EngineError::Failed {
                id: id.to_string(),
                message: format!("the cue {at} is not a cue"),
            });
        };
        let text = |name: &str| -> Option<String> {
            fields.get(name).and_then(|value| value.as_str()).map(|value| value.to_string())
        };
        out.push(VttCue {
            start: cue.start,
            end: cue.end,
            id: text("id").filter(|id| !id.is_empty() && !id.contains("-->")),
            settings: text("settings").unwrap_or_default(),
            speaker: text("speaker").filter(|speaker| !speaker.is_empty()),
            text: cue.text.clone(),
        });
    }
    Ok(out)
}

/// The **blocks** a caller gives: the notes, the styles and the
/// regions of a WebVTT file.
fn blocks_from(value: &Json, id: &str) -> Result<Vec<String>, EngineError> {
    let Some(list) = value.as_array() else {
        return Err(EngineError::Failed { id: id.to_string(), message: "the blocks are a list".to_string() });
    };
    if list.len() > MAX_CUES {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there are {} blocks, and the most there may be is {MAX_CUES}", list.len()),
        });
    }
    let mut blocks = Vec::with_capacity(list.len());
    for (at, block) in list.iter().enumerate() {
        let Some(block) = block.as_str() else {
            return Err(EngineError::Failed {
                id: id.to_string(),
                message: format!("the block {at} is not text"),
            });
        };
        if block.chars().count() > MAX_TEXT {
            return Err(EngineError::Failed {
                id: id.to_string(),
                message: format!("the block {at} is longer than {MAX_TEXT} characters"),
            });
        }
        blocks.push(block.to_string());
    }
    Ok(blocks)
}

/// Write a caption file of **SubRip** cues.
///
/// The tree's own writer: the cues are numbered from one, the clock is
/// rounded to the nearest millisecond, and a blank line inside the
/// text of a cue is dropped — a blank line would end the cue.
pub fn write_subrip(cues: &[Cue]) -> String {
    let mut out = String::new();
    for (at, cue) in cues.iter().enumerate() {
        out.push_str(&format!("{}\n{} --> {}\n", at + 1, format_clock(cue.start), format_clock(cue.end)));
        for line in cue.text.lines().filter(|line| !line.trim().is_empty()) {
            out.push_str(line);
            out.push('\n');
        }
        out.push('\n');
    }
    out
}

/// Write a caption file of **WebVTT** cues.
///
/// The tree's own writer: the file begins with the signature, the
/// blocks come before the cues, a cue keeps its identifier and its
/// settings, the voice is written in front of the text, and an arrow
/// in the text is written so that it is not an arrow.
pub fn write_vtt(cues: &[VttCue], blocks: &[String]) -> String {
    let mut out = String::from("WEBVTT\n\n");
    for block in blocks {
        let block = block.trim_end();
        if !block.is_empty() {
            out.push_str(block);
            out.push_str("\n\n");
        }
    }
    for cue in cues {
        if let Some(id) = cue.id.as_deref().map(str::trim).filter(|id| !id.is_empty() && !id.contains("-->"))
        {
            out.push_str(id);
            out.push('\n');
        }
        out.push_str(&format!(
            "{} --> {}",
            format_clock_marked(cue.start, '.'),
            format_clock_marked(cue.end, '.')
        ));
        if !cue.settings.trim().is_empty() {
            out.push(' ');
            out.push_str(cue.settings.trim());
        }
        out.push('\n');
        let mut text = String::new();
        if let Some(speaker) = cue.speaker.as_deref().filter(|speaker| !speaker.is_empty()) {
            text.push_str(&format!("<v {speaker}>"));
        }
        text.push_str(&cue.text);
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            out.push_str(&line.replace("-->", "--&gt;"));
            out.push('\n');
        }
        out.push('\n');
    }
    out
}

/// **Write** a caption file.
///
/// The cues are given to the command and the file is written. The
/// rules are the tree's own: the cues are numbered from one, the
/// clock is rounded to the nearest millisecond, and a blank line
/// inside the text of a cue is dropped.
#[derive(Command)]
#[command(id = "film.captions.write", area = "film", layer = "film", icon = "plus", mutates, undoable)]
pub struct WriteCaptions {
    /// The file to write.
    pub path: Text,
    /// The format of the file: `subrip` when it is not given.
    pub format: Option<Text>,
    /// The cues: each one a `start`, an `end` and a `text`.
    pub cues: Json,
    /// The blocks of a WebVTT file: the notes, the styles and the
    /// regions.
    pub blocks: Option<Json>,
}

impl WriteCaptions {
    fn run(&self, _session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.captions.write".to_string();
        let path = self.path.0.trim().to_string();
        if path.is_empty() {
            return Err(EngineError::Failed { id: id.clone(), message: "a file has a path".to_string() });
        }
        let named = self.format.as_ref().map(|format| format.0.trim().to_ascii_lowercase());
        let format = match named.as_deref() {
            None | Some("") | Some("subrip") | Some("srt") => "subrip",
            Some("webvtt") | Some("vtt") => "webvtt",
            Some(other) => {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "`{other}` is not a format of a caption file: the formats are {}",
                        FORMATS.join(", ")
                    ),
                });
            }
        };
        let written = if format == "webvtt" {
            let cues = vtt_cues_from(&self.cues, &id)?;
            let blocks = match &self.blocks {
                Some(blocks) => blocks_from(blocks, &id)?,
                None => Vec::new(),
            };
            write_vtt(&cues, &blocks)
        } else {
            let cues = cues_from(&self.cues, &id)?;
            write_subrip(&cues)
        };
        std::fs::write(&path, written.as_bytes()).map_err(|error| EngineError::Failed {
            id: id.clone(),
            message: format!("the file cannot be written: {error}"),
        })?;
        let count = self.cues.as_array().map(|cues| cues.len()).unwrap_or(0);
        Ok(Outcome::changed(format!("`{path}` was written with {count} cue(s)")).with_data(json!({
            "path": path,
            "format": format,
            "count": count,
            "bytes": written.len(),
        })))
    }
}
