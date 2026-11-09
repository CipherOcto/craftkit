//! Scenarist SCC (`.scc`): CEA-608 byte pairs with SMPTE timecodes at
//! twenty-nine and ninety-seven frames a second.
//!
//! ```text
//! Scenarist_SCC V1.0
//!
//! 00:00:00;00 9420 9420 94ae 94ae 1370 1370 c845 4c4c cf80 942f 942f
//!
//! 00:00:02;00 942c 942c
//! ```
//!
//! A tab separates the timecode from the code words. Each line's
//! pairs are sent one per frame, starting at its timecode.
//!
//! # The rules
//!
//! These are the tree's own rules.
//!
//! - The **header** of the file says `Scenarist_SCC`, and a file that
//!   does not is not one.
//! - A **timecode** is hours, minutes, seconds and frames, and it may
//!   carry the drop-frame mark. A timecode that is not one is refused
//!   by name and with the line it is on.
//! - A **code word** is four hexadecimal digits, and a code word that
//!   is not one is a **warning**: the file is read and the word is
//!   left out.
//! - The **pairs** are read on **channel one**, and the redundant
//!   control code that follows a control code is ignored.
//! - The captions are **pop-on**, **roll-up** or **paint-on**, as the
//!   file says, and a caption that is still showing when the file ends
//!   stays up for **one second** more.

use craftkit_sdk::serde_json::{Value as Json, json};

use crate::captions::Cue;

/// The **columns** of a caption: thirty-two, as the format says.
const COLS: usize = 32;

/// The **rows** of a caption: fifteen, as the format says.
const ROWS: usize = 15;

/// The rate of the **timecode**: twenty-nine and ninety-seven.
const NUM: i64 = 30000;
const DEN: i64 = 1001;

/// The most **lines** a file may have.
const MAX_LINES: usize = 100_000;

/// The most **code words** a line may have.
const MAX_WORDS: usize = 4096;

/// The **basic character set**, which differs from ASCII in ten places.
pub fn basic_char(byte: u8) -> Option<char> {
    Some(match byte {
        0x2a => 'á',
        0x5c => 'é',
        0x5e => 'í',
        0x5f => 'ó',
        0x60 => 'ú',
        0x7b => 'ç',
        0x7c => '÷',
        0x7d => 'Ñ',
        0x7e => 'ñ',
        0x7f => '█',
        0x20..=0x7e => byte as char,
        _ => return None,
    })
}

/// The **special characters**, which come as a control pair.
const SPECIAL: [char; 16] =
    ['®', '°', '½', '¿', '™', '¢', '£', '♪', 'à', '\u{a0}', 'è', 'â', 'ê', 'î', 'ô', 'û'];

/// The **extended Spanish, French and miscellaneous** characters.
const EXT_12: [char; 32] = [
    'Á', 'É', 'Ó', 'Ú', 'Ü', 'ü', '‘', '¡', '*', '\'', '—', '©', '℠', '•', '“', '”', 'À', 'Â', 'Ç', 'È', 'Ê',
    'Ë', 'ë', 'Î', 'Ï', 'ï', 'Ô', 'Ù', 'ù', 'Û', '«', '»',
];

/// The **extended Portuguese, German and Danish** characters.
const EXT_13: [char; 32] = [
    'Ã', 'ã', 'Í', 'Ì', 'ì', 'Ò', 'ò', 'Õ', 'õ', '{', '}', '\\', '^', '_', '|', '~', 'Ä', 'ä', 'Ö', 'ö', 'ß',
    '¥', '¤', '¦', 'Å', 'å', 'Ø', 'ø', '┌', '┐', '└', '┘',
];

/// The **miscellaneous codes** of the format.
mod misc {
    /// Resume caption loading: the pop-on mode.
    pub const RCL: u8 = 0x20;
    /// Backspace.
    pub const BS: u8 = 0x21;
    /// Delete to the end of the row.
    pub const DER: u8 = 0x24;
    /// Roll-up of two rows.
    pub const RU2: u8 = 0x25;
    /// Roll-up of three rows.
    pub const RU3: u8 = 0x26;
    /// Roll-up of four rows.
    pub const RU4: u8 = 0x27;
    /// Resume direct captioning: the paint-on mode.
    pub const RDC: u8 = 0x29;
    /// Erase the displayed memory.
    pub const EDM: u8 = 0x2c;
    /// Carriage return: the roll-up.
    pub const CR: u8 = 0x2d;
    /// Erase the non-displayed memory.
    pub const ENM: u8 = 0x2e;
    /// End of caption: the memories swap.
    pub const EOC: u8 = 0x2f;
}

/// The **row** and the **indent** a preamble address code names.
pub fn decode_pac(b1: u8, b2: u8) -> Option<(usize, usize)> {
    if !(0x40..=0x7f).contains(&b2) {
        return None;
    }
    let hi = b2 >= 0x60;
    let row = match (b1, hi) {
        (0x11, false) => 1,
        (0x11, true) => 2,
        (0x12, false) => 3,
        (0x12, true) => 4,
        (0x15, false) => 5,
        (0x15, true) => 6,
        (0x16, false) => 7,
        (0x16, true) => 8,
        (0x17, false) => 9,
        (0x17, true) => 10,
        (0x10, false) => 11,
        (0x13, false) => 12,
        (0x13, true) => 13,
        (0x14, false) => 14,
        (0x14, true) => 15,
        _ => return None,
    };
    let bits = b2 & 0x1f;
    let indent = if bits & 0x10 != 0 { ((bits & 0x0e) >> 1) as usize * 4 } else { 0 };
    Some((row, indent))
}

/// The **frames** of a timecode: hours, minutes, seconds and frames,
/// and the drop-frame mark.
pub fn parse_timecode(text: &str) -> Option<i64> {
    let drop_frame = text.contains([';', '.', ',']);
    let parts: Vec<&str> = text.split([':', ';', '.', ',']).collect();
    let [hours, minutes, seconds, frames] = parts.as_slice() else { return None };
    let number = |part: &str| {
        if (1..=3).contains(&part.len()) && part.bytes().all(|byte| byte.is_ascii_digit()) {
            part.parse::<i64>().ok()
        } else {
            None
        }
    };
    let (hours, minutes, seconds, frames) =
        (number(hours)?, number(minutes)?, number(seconds)?, number(frames)?);
    Some(fields_to_frames(hours, minutes, seconds, frames, drop_frame))
}

/// The **frames** of hours, minutes, seconds and frames at the rate of
/// the format.
fn fields_to_frames(hours: i64, minutes: i64, seconds: i64, frames: i64, drop_frame: bool) -> i64 {
    // The drop-frame timecode skips two frames at the start of every
    // minute, and every tenth minute keeps them.
    let base = 30;
    let total = ((hours * 60 + minutes) * 60 + seconds) * base + frames;
    if drop_frame {
        let minutes = hours * 60 + minutes;
        let dropped = minutes - minutes / 10;
        total - dropped * 2
    } else {
        total
    }
}

/// The **milliseconds** of a frame of the format.
fn milliseconds_of(frame: i64) -> i64 {
    (frame * DEN * 1000) / NUM
}

/// The **memory** of a caption: fifteen rows of thirty-two columns.
type Memory = [[Option<char>; COLS]; ROWS];

/// The **mode** a caption is in.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// The pop-on mode: the text is loaded and then shown.
    PopOn,
    /// The roll-up mode, with the number of rows it rolls.
    RollUp(usize),
    /// The paint-on mode: the text is written as it comes.
    PaintOn,
}

/// The **decoder** of the format: the memories, the mode and the cues.
struct Decoder {
    /// The memory that is shown.
    displayed: Memory,
    /// The memory that is loaded.
    hidden: Memory,
    /// The mode.
    mode: Mode,
    /// The row that is written.
    row: usize,
    /// The column that is written.
    col: usize,
    /// The last control pair, for the redundancy that is ignored.
    last_control: Option<(u8, u8)>,
    /// Whether the data is of channel one.
    channel_one: bool,
    /// The cue that is shown: the frame it began at and its text.
    open: Option<(i64, String)>,
    /// The cues that are done.
    cues: Vec<(i64, i64, String)>,
}

/// The **text** of a memory: the rows that hold something.
fn memory_text(memory: &Memory) -> String {
    memory
        .iter()
        .map(|row| row.iter().map(|cell| cell.unwrap_or(' ')).collect::<String>().trim().to_string())
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

impl Decoder {
    /// A decoder with nothing in it.
    fn new() -> Self {
        Self {
            displayed: [[None; COLS]; ROWS],
            hidden: [[None; COLS]; ROWS],
            mode: Mode::PopOn,
            row: ROWS - 1,
            col: 0,
            last_control: None,
            channel_one: true,
            open: None,
            cues: Vec::new(),
        }
    }

    /// The memory that is written.
    fn target(&mut self) -> &mut Memory {
        if self.mode == Mode::PopOn { &mut self.hidden } else { &mut self.displayed }
    }

    /// Put a **character** into the memory.
    fn put(&mut self, character: char) {
        let row = self.row.min(ROWS - 1);
        let col = self.col.min(COLS - 1);
        self.target()[row][col] = Some(character);
        self.col = (self.col + 1).min(COLS - 1);
    }

    /// The displayed memory may have changed at the frame: the cue
    /// that was shown ends and the one that is shown begins.
    fn sync(&mut self, frame: i64) {
        let now = memory_text(&self.displayed);
        let shown = self.open.as_ref().map(|open| open.1.clone()).unwrap_or_default();
        if now == shown {
            return;
        }
        if let Some((start, text)) = self.open.take()
            && frame > start
        {
            self.cues.push((start, frame, text));
        }
        if !now.is_empty() {
            self.open = Some((frame, now));
        }
    }

    /// A **pair** of bytes at the frame.
    fn pair(&mut self, frame: i64, first: u8, second: u8) {
        let (first, second) = (first & 0x7f, second & 0x7f);
        if (0x10..=0x1f).contains(&first) {
            // A control code that follows the same control code is
            // redundant, and it is ignored.
            if self.last_control == Some((first, second)) {
                self.last_control = None;
                return;
            }
            self.last_control = Some((first, second));
            self.channel_one = first & 0x08 == 0;
            if !self.channel_one {
                return;
            }
            self.control(frame, first & 0x17, second);
            return;
        }
        self.last_control = None;
        if !self.channel_one {
            return;
        }
        for byte in [first, second] {
            if let Some(character) = basic_char(byte) {
                self.put(character);
            }
        }
        if self.mode != Mode::PopOn {
            self.sync(frame);
        }
    }

    /// A **control code** at the frame.
    fn control(&mut self, frame: i64, first: u8, second: u8) {
        // The miscellaneous codes: they come as 0x14 or 0x15 and a
        // code in the second byte.
        if (first == 0x14 || first == 0x15) && (0x20..=0x2f).contains(&second) {
            match second {
                misc::RCL => self.mode = Mode::PopOn,
                misc::RDC => self.mode = Mode::PaintOn,
                misc::RU2 | misc::RU3 | misc::RU4 => {
                    let rows = (second - misc::RU2) as usize + 2;
                    if !matches!(self.mode, Mode::RollUp(_)) {
                        self.displayed = [[None; COLS]; ROWS];
                        self.row = ROWS - 1;
                    }
                    self.mode = Mode::RollUp(rows);
                    self.col = 0;
                }
                misc::BS => {
                    self.col = self.col.saturating_sub(1);
                    let (row, col) = (self.row, self.col);
                    self.target()[row][col] = None;
                }
                misc::DER => {
                    let (row, col) = (self.row, self.col);
                    for at in col..COLS {
                        self.target()[row][at] = None;
                    }
                }
                misc::EDM => self.displayed = [[None; COLS]; ROWS],
                misc::ENM => self.hidden = [[None; COLS]; ROWS],
                misc::EOC => {
                    std::mem::swap(&mut self.displayed, &mut self.hidden);
                    self.mode = Mode::PopOn;
                }
                misc::CR => {
                    if let Mode::RollUp(rows) = self.mode {
                        let top = self.row + 1 - rows.min(self.row + 1);
                        for row in top..self.row {
                            self.displayed[row] = self.displayed[row + 1];
                        }
                        if top > 0 {
                            self.displayed[top - 1] = [None; COLS];
                        }
                        self.displayed[self.row] = [None; COLS];
                        self.col = 0;
                    }
                }
                _ => {}
            }
            self.sync(frame);
            return;
        }
        // The **tab offsets**: the column moves on.
        if first == 0x17 && (0x21..=0x23).contains(&second) {
            self.col = (self.col + (second - 0x20) as usize).min(COLS - 1);
            return;
        }
        // The **special characters**.
        if first == 0x11 && (0x30..=0x3f).contains(&second) {
            self.put(SPECIAL[(second - 0x30) as usize]);
            if self.mode != Mode::PopOn {
                self.sync(frame);
            }
            return;
        }
        // The **extended characters**: they replace the character before.
        if (first == 0x12 || first == 0x13) && (0x20..=0x3f).contains(&second) {
            self.col = self.col.saturating_sub(1);
            let set = if first == 0x12 { &EXT_12 } else { &EXT_13 };
            self.put(set[(second - 0x20) as usize]);
            if self.mode != Mode::PopOn {
                self.sync(frame);
            }
            return;
        }
        // The **mid-row style change**: it is shown as a space.
        if first == 0x11 && (0x20..=0x2f).contains(&second) {
            self.put(' ');
            return;
        }
        // The **preamble address code**: the row and the indent.
        if let Some((row, indent)) = decode_pac(first, second) {
            let row = row.saturating_sub(1);
            if let Mode::RollUp(rows) = self.mode
                && row != self.row
            {
                // The roll-up window moves with the base row.
                let old = self.displayed;
                self.displayed = [[None; COLS]; ROWS];
                for at in 0..rows {
                    if self.row >= at && row >= at {
                        self.displayed[row - at] = old[self.row - at];
                    }
                }
            }
            self.row = row;
            self.col = indent;
        }
    }
}

/// The **cues** of an SCC file: the tree's own reader.
pub fn cues_of(text: &str) -> Result<(Vec<Cue>, Vec<String>), String> {
    let mut warnings = Vec::new();
    // The **lines** of the file: they are bounded.
    let all: Vec<(usize, &str)> =
        text.lines().enumerate().filter(|(_, line)| !line.trim().is_empty()).collect();
    if all.len() > MAX_LINES {
        return Err(format!("the file has more than {MAX_LINES} lines"));
    }
    let mut lines = all.into_iter();
    let header = lines.next();
    match header {
        Some((_, line)) if line.trim().starts_with("Scenarist_SCC") => {}
        _ => return Err("not a Scenarist SCC file".to_string()),
    }
    let mut decoder = Decoder::new();
    let mut last_frame = 0i64;
    for (number, line) in lines {
        let mut words = line.split_whitespace();
        let Some(timecode) = words.next() else { continue };
        let Some(frame) = parse_timecode(timecode) else {
            return Err(format!("line {}: the timecode `{timecode}` is not one", number + 1));
        };
        let mut at = 0usize;
        for word in words {
            if at >= MAX_WORDS {
                warnings.push(format!("line {}: the line has more than {MAX_WORDS} code words", number + 1));
                break;
            }
            let Ok(value) = u16::from_str_radix(word, 16) else {
                warnings.push(format!("line {}: the code word `{word}` is not one", number + 1));
                at += 1;
                continue;
            };
            let at_frame = frame + at as i64;
            decoder.pair(at_frame, (value >> 8) as u8, value as u8);
            last_frame = last_frame.max(at_frame + 1);
            at += 1;
        }
    }
    // A caption that is still showing when the file ends stays up for
    // one second more.
    if let Some((start, text)) = decoder.open.take() {
        let end = last_frame.max(start + 1) + 30;
        decoder.cues.push((start, end, text));
    }
    let cues = decoder
        .cues
        .into_iter()
        .map(|(start, end, text)| Cue { start: milliseconds_of(start), end: milliseconds_of(end), text })
        .collect();
    Ok((cues, warnings))
}

/// The **cues** of an SCC file, as the answer holds them.
pub fn cues_as_json(cues: &[Cue]) -> Vec<Json> {
    cues.iter()
        .map(|cue| {
            json!({
                "start": cue.start,
                "end": cue.end,
                "startClock": crate::captions::format_clock(cue.start),
                "endClock": crate::captions::format_clock(cue.end),
                "text": cue.text,
            })
        })
        .collect()
}
