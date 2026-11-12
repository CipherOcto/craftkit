//! The **cues** of an **EBU STL** file.
//!
//! STL (`.stl`) is the subtitling data exchange format of EBU Tech
//! 3264-E: a file is a **General Subtitle Information** block of 1024
//! bytes and then a run of **Text and Timing Information** blocks of
//! 128 bytes each.
//!
//! # The rules
//!
//! These are the tree's own rules for reading the format.
//!
//! - The **rate** is the one the file says in its Disk Format Code:
//!   `STL25.01` is twenty-five frames a second and `STL30.01` is
//!   twenty-nine and ninety-seven. A file with no rate is not an STL
//!   file, and that is refused by name.
//! - The **timecodes** of a block are four bytes — hours, minutes,
//!   seconds and frames — and they are **relative to the start of the
//!   programme**, the timecode the general block carries. A cue that
//!   would start before the programme does starts at its beginning.
//! - A **user-data** block (`EBN 0xFE`) and a **comment** block are
//!   skipped.
//! - A subtitle may run over several blocks: the blocks with the same
//!   subtitle number are one subtitle, and the block with the
//!   extension number `0xFF` is the last of them.
//! - The **text** of a block is a field of 112 bytes: `0x8A` is a line
//!   break, `0x80` and `0x81` are the italics on and off, `0x82` and
//!   `0x83` are the underline on and off, and the control codes are
//!   skipped. The letters are ISO/IEC 6937, and a diacritic is a byte
//!   that is followed by the letter it sits on.
//! - A subtitle whose **end is not after its start**, or whose text is
//!   nothing, is not a cue.

use craftkit_sdk::serde_json::{Value as Json, json};

use crate::captions::Cue;

/// The size of the **general** block.
const GSI: usize = 1024;

/// The size of a **text and timing** block.
const TTI: usize = 128;

/// The size of the **text** field of a block.
const TF: usize = 112;

/// The **specials** of ISO/IEC 6937: the bytes from `0xA0` that hold
/// something other than a letter.
const SPECIALS: &[(u8, char)] = &[
    (0xa0, '\u{a0}'),
    (0xa1, '¡'),
    (0xa2, '¢'),
    (0xa3, '£'),
    (0xa4, '$'),
    (0xa5, '¥'),
    (0xa6, '#'),
    (0xa7, '§'),
    (0xa8, '¤'),
    (0xa9, '‘'),
    (0xaa, '“'),
    (0xab, '«'),
    (0xac, '←'),
    (0xad, '↑'),
    (0xae, '→'),
    (0xaf, '↓'),
    (0xb0, '°'),
    (0xb1, '±'),
    (0xb2, '²'),
    (0xb3, '³'),
    (0xb4, '×'),
    (0xb5, 'µ'),
    (0xb6, '¶'),
    (0xb7, '·'),
    (0xb8, '÷'),
    (0xb9, '’'),
    (0xba, '”'),
    (0xbb, '»'),
    (0xbc, '¼'),
    (0xbd, '½'),
    (0xbe, '¾'),
    (0xbf, '¿'),
    (0xd0, '―'),
    (0xd1, '¹'),
    (0xd2, '®'),
    (0xd3, '©'),
    (0xd4, '™'),
    (0xd5, '♪'),
    (0xd6, '¬'),
    (0xd7, '¦'),
    (0xdc, '⅛'),
    (0xdd, '⅜'),
    (0xde, '⅝'),
    (0xdf, '⅞'),
    (0xe0, 'Ω'),
    (0xe1, 'Æ'),
    (0xe2, 'Đ'),
    (0xe3, 'ª'),
    (0xe4, 'Ħ'),
    (0xe6, 'Ĳ'),
    (0xe7, 'Ŀ'),
    (0xe8, 'Ł'),
    (0xe9, 'Ø'),
    (0xea, 'Œ'),
    (0xeb, 'º'),
    (0xec, 'Þ'),
    (0xed, 'Ŧ'),
    (0xee, 'Ŋ'),
    (0xef, 'ŉ'),
    (0xf0, 'ĸ'),
    (0xf1, 'æ'),
    (0xf2, 'đ'),
    (0xf3, 'ð'),
    (0xf4, 'ħ'),
    (0xf5, 'ı'),
    (0xf6, 'ĳ'),
    (0xf7, 'ŀ'),
    (0xf8, 'ł'),
    (0xf9, 'ø'),
    (0xfa, 'œ'),
    (0xfb, 'ß'),
    (0xfc, 'þ'),
    (0xfd, 'ŧ'),
    (0xfe, 'ŋ'),
];

/// The **diacritics** of ISO/IEC 6937: a byte that sits in front of a
/// letter, the letters it may sit in front of, and the letters that
/// come out of it.
const DIACRITICS: &[(u8, &str, &str)] = &[
    (0xc1, "AEIOUaeiou", "ÀÈÌÒÙàèìòù"),
    (0xc2, "ACEILNORSUYZacegilnorsuyz", "ÁĆÉÍĹŃÓŔŚÚÝŹáćéģíĺńóŕśúýź"),
    (0xc3, "ACEGHIJOSUWYaceghijosuwy", "ÂĈÊĜĤÎĴÔŜÛŴŶâĉêĝĥîĵôŝûŵŷ"),
    (0xc4, "AINOUainou", "ÃĨÑÕŨãĩñõũ"),
    (0xc5, "AEIOUaeiou", "ĀĒĪŌŪāēīōū"),
    (0xc6, "AGUagu", "ĂĞŬăğŭ"),
    (0xc7, "CEGIZcegz", "ĊĖĠİŻċėġż"),
    (0xc8, "AEIOUYaeiouy", "ÄËÏÖÜŸäëïöüÿ"),
    (0xca, "AUau", "ÅŮåů"),
    (0xcb, "CGKLNRSTcklnrst", "ÇĢĶĻŅŖŞŢçķļņŗşţ"),
    (0xcd, "OUou", "ŐŰőű"),
    (0xce, "AEIUaeiu", "ĄĘĮŲąęįų"),
    (0xcf, "CDELNRSTZcdelnrstz", "ČĎĚĽŇŘŠŤŽčďěľňřšťž"),
];

/// The **letter** a diacritic and a base letter make.
fn compose(diacritic: u8, base: u8) -> Option<char> {
    let (_, bases, composed) = DIACRITICS.iter().find(|entry| entry.0 == diacritic)?;
    let at = bases.chars().position(|letter| letter as u32 == u32::from(base))?;
    composed.chars().nth(at)
}

/// The **rate** a disk format code says: the frames a second.
fn rate_of(code: &[u8]) -> Option<(i64, i64)> {
    match code {
        b"STL25.01" => Some((25, 1)),
        b"STL30.01" => Some((30000, 1001)),
        _ => None,
    }
}

/// Whether the bytes are an **STL file**.
pub fn sniff(bytes: &[u8]) -> bool {
    bytes.len() >= GSI && bytes[3..11].starts_with(b"STL") && rate_of(&bytes[3..11]).is_some()
}

/// The **frames** a timecode of hours, minutes, seconds and frames is.
fn frames_of(hours: i64, minutes: i64, seconds: i64, frames: i64, num: i64, den: i64) -> i64 {
    let whole = hours * 3600 + minutes * 60 + seconds;
    ((whole * num + den / 2) / den) + frames
}

/// The **milliseconds** a number of frames is, at a rate.
fn milliseconds_of(frames: i64, num: i64, den: i64) -> i64 {
    (frames * den * 1000 + num / 2) / num
}

/// The **text** of a text field, and the italics and the underline it
/// leaves open.
fn decode_text(field: &[u8], italic: &mut bool, underline: &mut bool) -> String {
    let mut out = String::new();
    let mut at = 0;
    while at < field.len() {
        let byte = field[at];
        at += 1;
        match byte {
            0x8f => {}
            0x8a => {
                // A line break. Two of them in a row are one break:
                // the second is a double-height row.
                if !out.ends_with('\n') || out.is_empty() {
                    out.push('\n');
                }
            }
            0x80 if !*italic => {
                out.push_str("<i>");
                *italic = true;
            }
            0x81 if *italic => {
                out.push_str("</i>");
                *italic = false;
            }
            0x82 if !*underline => {
                out.push_str("<u>");
                *underline = true;
            }
            0x83 if *underline => {
                out.push_str("</u>");
                *underline = false;
            }
            0x00..=0x1f | 0x7f..=0x9f => {}
            0x20..=0x7e => out.push(char::from(byte)),
            0xc1..=0xcf => {
                if let Some(base) = field.get(at)
                    && let Some(letter) = compose(byte, *base)
                {
                    out.push(letter);
                    at += 1;
                }
            }
            _ => {
                if let Some((_, letter)) = SPECIALS.iter().find(|entry| entry.0 == byte) {
                    out.push(*letter);
                }
            }
        }
    }
    out
}

/// The **cues** of an STL file, and the warnings about it.
pub fn cues_of(bytes: &[u8]) -> Result<(Vec<Cue>, Vec<String>), String> {
    if !sniff(bytes) {
        return Err("the file is not an EBU STL file".to_string());
    }
    let Some((num, den)) = rate_of(&bytes[3..11]) else {
        return Err("the file is not an EBU STL file".to_string());
    };
    let general = &bytes[..GSI];
    // The **start of the programme**: the timecodes of the cues are
    // relative to it.
    let start_code = String::from_utf8_lossy(&general[256..264]).trim().to_string();
    let start = if start_code.len() == 8 && start_code.bytes().all(|byte| byte.is_ascii_digit()) {
        let number = |at: usize| start_code[at..at + 2].parse::<i64>().unwrap_or(0);
        frames_of(number(0), number(2), number(4), number(6), num, den)
    } else {
        0
    };
    let mut cues: Vec<Cue> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    // The **subtitle** being read: its number, its times and its text.
    let mut current: Option<(u16, i64, i64, String, bool, bool)> = None;
    let finish = |held: Option<(u16, i64, i64, String, bool, bool)>, cues: &mut Vec<Cue>| {
        let Some((_, tin, tout, mut text, italic, underline)) = held else { return };
        if italic {
            text.push_str("</i>");
        }
        if underline {
            text.push_str("</u>");
        }
        let text = text.lines().map(str::trim).filter(|line| !line.is_empty()).collect::<Vec<_>>().join("\n");
        if tout > tin && !text.is_empty() {
            cues.push(Cue {
                start: milliseconds_of((tin - start).max(0), num, den),
                end: milliseconds_of((tout - start).max(0), num, den),
                text,
            });
        }
    };
    let blocks = &bytes[GSI..];
    let mut at = 0;
    while at + TTI <= blocks.len() {
        let block = &blocks[at..at + TTI];
        at += TTI;
        let number = u16::from_le_bytes([block[1], block[2]]);
        let extension = block[3];
        let comment = block[15];
        if extension == 0xfe || comment != 0 {
            // A **user-data** block and a **comment** block are
            // skipped.
            continue;
        }
        let tin = frames_of(
            i64::from(block[5]),
            i64::from(block[6]),
            i64::from(block[7]),
            i64::from(block[8]),
            num,
            den,
        );
        let tout = frames_of(
            i64::from(block[9]),
            i64::from(block[10]),
            i64::from(block[11]),
            i64::from(block[12]),
            num,
            den,
        );
        // A **new** subtitle finishes the one before it.
        if current.as_ref().is_some_and(|held| held.0 != number) {
            finish(current.take(), &mut cues);
        }
        if current.is_none() {
            current = Some((number, tin, tout, String::new(), false, false));
        }
        if let Some(held) = current.as_mut() {
            let mut italic = held.4;
            let mut underline = held.5;
            let text = decode_text(&block[16..16 + TF], &mut italic, &mut underline);
            held.3.push_str(&text);
            held.4 = italic;
            held.5 = underline;
        }
        if extension == 0xff {
            finish(current.take(), &mut cues);
        }
    }
    finish(current.take(), &mut cues);
    if !blocks.is_empty() && !blocks.len().is_multiple_of(TTI) {
        warnings.push("there are bytes after the last block of the file".to_string());
    }
    Ok((cues, warnings))
}

/// The **cues** of an STL file, as they are said.
pub fn cues_as_json(cues: &[Cue]) -> Vec<Json> {
    cues.iter()
        .map(|cue| {
            json!({
                "start": cue.start,
                "end": cue.end,
                "text": cue.text,
            })
        })
        .collect()
}

/// The **name** the file carries: the tree writes its own name in the
/// header of the file, and this tree writes its own.
const ORIGINATOR: &str = "craftkit";

/// The **bytes** of one character in ISO/IEC 6937, when it has any.
///
/// The letters and the numbers are their own bytes, the specials have
/// theirs, and a letter with a diacritic is a byte for the diacritic
/// and the byte of the letter. A character that the set does not hold
/// has none.
pub fn encode_6937(character: char) -> Option<Vec<u8>> {
    if (' '..='~').contains(&character) {
        return Some(vec![character as u8]);
    }
    if let Some((byte, _)) =
        SPECIALS.iter().find(|entry| entry.1 == character && entry.0 != 0xa4 && entry.0 != 0xa6)
    {
        return Some(vec![*byte]);
    }
    for (diacritic, bases, composed) in DIACRITICS {
        if let Some(at) = composed.chars().position(|letter| letter == character) {
            return Some(vec![*diacritic, bases.as_bytes()[at]]);
        }
    }
    None
}

/// The **text** of a cue, as the bytes of a text field.
///
/// The tags of the tree's text — `<i>`, `</i>`, `<u>` and `</u>` — are
/// the codes of the format, a line break is a line break, and a
/// character the set does not hold is a question mark.
fn encode_text(text: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(character) = rest.chars().next() {
        let is_tag_start = character == '<'
            && rest
                .get(1..)
                .and_then(|tail| tail.strip_prefix('/').or(Some(tail)))
                .and_then(|name| name.as_bytes().first())
                .is_some_and(u8::is_ascii_alphabetic);
        if is_tag_start && let Some(end) = rest.find('>') {
            let tag = &rest[1..end];
            // Only a tag that is syntactically plausible is markup: a
            // text such as "2 < 3 > 1" keeps its middle.
            let name = tag.strip_prefix('/').unwrap_or(tag);
            if name.as_bytes().first().is_some_and(u8::is_ascii_alphabetic) {
                match tag.trim().to_ascii_lowercase().as_str() {
                    "i" => out.push(0x80),
                    "/i" => out.push(0x81),
                    "u" => out.push(0x82),
                    "/u" => out.push(0x83),
                    _ => {}
                }
                rest = &rest[end + 1..];
                continue;
            }
        }
        if character == '\n' {
            out.push(0x8a);
        } else {
            out.extend(encode_6937(character).unwrap_or_else(|| vec![b'?']));
        }
        rest = &rest[character.len_utf8()..];
    }
    out
}

/// The **chunks** a text field is split into: at most 112 bytes each,
/// and a diacritic is never split from the letter it sits on.
fn split_tf(text: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < text.len() {
        let mut end = (at + TF).min(text.len());
        if end < text.len() && (0xc1..=0xcf).contains(&text[end - 1]) {
            end -= 1;
        }
        out.push(&text[at..end]);
        at = end;
    }
    out
}

/// A **cue** of an EBU STL file: when it comes and when it goes, in
/// the **frames** of the sequence, and its text.
pub struct StlCue {
    /// When the cue comes, in the frames of the sequence.
    pub start: i64,
    /// When the cue goes, in the frames of the sequence.
    pub end: i64,
    /// The text of the cue.
    pub text: String,
}

/// The **fields** of a timecode: hours, minutes, seconds and frames.
fn fields_of(frame: i64, base: i64) -> (u8, u8, u8, u8) {
    let frame = frame.max(0);
    let hours = frame / (base * 3600);
    let minutes = (frame / (base * 60)) % 60;
    let seconds = (frame / base) % 60;
    let frames = frame % base;
    (hours.min(255) as u8, minutes as u8, seconds as u8, frames as u8)
}

/// The cues of a caption track as an **EBU STL file**.
///
/// The rate is the rate of the sequence, and the times of the cues are
/// its frames. The file is written at **twenty-nine and ninety-seven**
/// when the sequence is at that rate or at thirty, and at **twenty-
/// five** otherwise.
pub fn write(cues: &[StlCue], num: i64, den: i64) -> Vec<u8> {
    // The **rate of the file**: the tree's own rule.
    let (stl_num, stl_den, dfc) = if (num == 30 && den == 1) || (num == 30000 && den == 1001) {
        (30000, 1001, "STL30.01")
    } else {
        (25, 1, "STL25.01")
    };
    // The **frames of the file**: the frames of the sequence, at the
    // rate of the file.
    let to_frames = |frame: i64| -> i64 {
        let numerator = i128::from(frame) * i128::from(den) * i128::from(stl_num);
        let denominator = i128::from(num) * i128::from(stl_den);
        let half = denominator / 2;
        let at = (numerator + half) / denominator;
        i64::try_from(at).unwrap_or(i64::MAX)
    };
    // The **timecode base** of the file: the frames counted in a
    // second of the timecode.
    let base = if stl_num % stl_den == 0 { stl_num / stl_den } else { stl_num / stl_den + 1 };
    // The **blocks**: the text and timing blocks first, as the general
    // block counts them.
    let mut ttis: Vec<[u8; TTI]> = Vec::new();
    let mut subtitles: u16 = 0;
    let mut first_in: Option<i64> = None;
    for cue in cues.iter().filter(|cue| cue.end > cue.start) {
        let number = subtitles;
        subtitles = subtitles.wrapping_add(1);
        let start = to_frames(cue.start);
        let end = to_frames(cue.end).max(start + 1);
        first_in.get_or_insert(start);
        let text = encode_text(&cue.text);
        let chunks: Vec<&[u8]> = if text.is_empty() { vec![&[][..]] } else { split_tf(&text) };
        let lines = cue.text.lines().count().clamp(1, 11);
        for (at, chunk) in chunks.iter().enumerate() {
            let mut block = [0u8; TTI];
            block[0] = 0;
            block[1..3].copy_from_slice(&number.to_le_bytes());
            block[3] = if at + 1 == chunks.len() { 0xff } else { at.min(0xfd) as u8 };
            block[4] = 0;
            for (offset, frame) in [(5, start), (9, end)] {
                let (hours, minutes, seconds, frames) = fields_of(frame, base);
                block[offset..offset + 4].copy_from_slice(&[hours, minutes, seconds, frames]);
            }
            block[13] = (22 - 2 * (lines - 1)) as u8;
            block[14] = 2;
            block[15] = 0;
            block[16..].fill(0x8f);
            let length = chunk.len().min(TF);
            block[16..16 + length].copy_from_slice(&chunk[..length]);
            ttis.push(block);
        }
    }
    // The **general block**.
    let mut general = [b' '; GSI];
    let put = |buffer: &mut [u8], at: usize, len: usize, text: &str| {
        let bytes = text.as_bytes();
        for (index, byte) in bytes.iter().take(len).enumerate() {
            buffer[at + index] = *byte;
        }
    };
    put(&mut general, 0, 3, "850");
    put(&mut general, 3, 8, dfc);
    put(&mut general, 11, 1, "0");
    put(&mut general, 12, 2, "00");
    put(&mut general, 14, 2, "09");
    put(&mut general, 16, 32, ORIGINATOR);
    put(&mut general, 224, 6, "000101");
    put(&mut general, 230, 6, "000101");
    put(&mut general, 236, 2, "00");
    put(&mut general, 238, 5, &format!("{:05}", ttis.len().min(99_999)));
    put(&mut general, 243, 5, &format!("{:05}", subtitles));
    put(&mut general, 248, 3, "001");
    put(&mut general, 251, 2, "40");
    put(&mut general, 253, 2, "23");
    put(&mut general, 255, 1, "1");
    put(&mut general, 256, 8, "00000000");
    let (hours, minutes, seconds, frames) = fields_of(first_in.unwrap_or(0), base);
    put(&mut general, 264, 8, &format!("{hours:02}{minutes:02}{seconds:02}{frames:02}"));
    put(&mut general, 272, 1, "1");
    put(&mut general, 273, 1, "1");
    // The **file**: the general block and then the blocks.
    let mut out = general.to_vec();
    for block in ttis {
        out.extend_from_slice(&block);
    }
    out
}
