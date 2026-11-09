//! MacCaption MCC (`.mcc`): caption data as text, one line per frame.
//!
//! A line of the file is a **timecode** and the **data** of one frame,
//! and the data is a caption distribution packet (SMPTE ST 334-2) that
//! carries the CEA-608 byte pairs of the field one and, beside them,
//! the CEA-708 service data.
//!
//! ```text
//! File Format=MacCaption_MCC V1.0
//! Time Code Rate=30DF
//!
//! 00:00:01;00 T49S494F43ZZ72F4FC9420RFE...G7400Z4C
//! ```
//!
//! # The rules
//!
//! These are the tree's own rules.
//!
//! - The **header** of the file says `File Format=MacCaption_MCC`, and
//!   a file that does not is not one.
//! - The **rate** of the timecode is named in the header, and a rate
//!   that is not one is refused by name and with the line it is on.
//! - The **data** of a line is hexadecimal with the **abbreviations**
//!   of the format: `G`–`O` are one to nine times `FA 00 00`, `P` is
//!   `FB 80 80`, `Q` is `FC 80 80`, `R` is `FD 80 80`, `S` is `96 69`,
//!   `T` is `61 01`, `U` is `E1 00 00 00` and `Z` is `00`.
//! - A line whose **data** is not one is a **warning**: the file is
//!   read and the line is left out.
//! - The **pairs** of the field one are the **CEA-608** pairs of the
//!   caption, and they are read with the same decoder the SCC reader
//!   uses.
//! - The **CEA-708** service data of the format is not read here.

use craftkit_sdk::serde_json::Value as Json;

use crate::captions::Cue;

/// The **abbreviations** of the format: a letter stands for a run of
/// bytes.
const ABBREV: [(char, &[u8]); 7] = [
    ('P', &[0xfb, 0x80, 0x80]),
    ('Q', &[0xfc, 0x80, 0x80]),
    ('R', &[0xfd, 0x80, 0x80]),
    ('S', &[0x96, 0x69]),
    ('T', &[0x61, 0x01]),
    ('U', &[0xe1, 0x00, 0x00, 0x00]),
    ('Z', &[0x00]),
];

/// The most **lines** a file may have.
const MAX_LINES: usize = 100_000;

/// The most **pairs** a file may carry.
const MAX_PAIRS: usize = 4_000_000;

/// The **rate** of a timecode: the nominal base of the frames, the
/// drop-frame mark and the number of frames a second.
#[derive(Clone, Copy)]
struct Rate {
    /// The **nominal base** of the frames: thirty, sixty, twenty-four,
    /// twenty-five or fifty.
    base: i64,
    /// Whether the **drop-frame** timecode is the one of the header.
    drop_frame: bool,
    /// The **frames** a second, as a fraction.
    num: i64,
    /// The **denominator** of the frames a second.
    den: i64,
}

impl Rate {
    /// The rate of the tree's own files: twenty-nine and ninety-seven,
    /// with the drop-frame mark.
    const fn fps_29_97() -> Self {
        Self { base: 30, drop_frame: true, num: 30_000, den: 1001 }
    }

    /// The **frames** of one second: the number a decoder waits for a
    /// caption that is still showing at the end of the file.
    const fn one_second(&self) -> i64 {
        (self.num + self.den / 2) / self.den
    }

    /// The **milliseconds** of a frame.
    const fn milliseconds_of(&self, frame: i64) -> i64 {
        (frame * self.den * 1000) / self.num
    }
}

/// The **rate** of a timecode, as the header names it.
fn tc_rate(value: &str) -> Option<Rate> {
    Some(match value.trim().to_ascii_uppercase().as_str() {
        "30DF" => Rate { drop_frame: true, ..Rate::fps_29_97() },
        "30" => Rate { drop_frame: false, ..Rate::fps_29_97() },
        "60DF" => Rate { base: 60, drop_frame: true, num: 60_000, den: 1001 },
        "60" => Rate { base: 60, drop_frame: false, num: 60_000, den: 1001 },
        "24" => Rate { base: 24, drop_frame: false, num: 24, den: 1 },
        "25" => Rate { base: 25, drop_frame: false, num: 25, den: 1 },
        "50" => Rate { base: 50, drop_frame: false, num: 50, den: 1 },
        _ => return None,
    })
}

/// The **frames** of hours, minutes, seconds and frames.
///
/// The tree's own rule: the drop-frame timecode skips two frames at the
/// start of every minute — four of them at sixty — and every tenth
/// minute keeps them.
fn fields_to_frames(
    hours: i64,
    minutes: i64,
    seconds: i64,
    frames: i64,
    rate: Rate,
    drop_frame: bool,
) -> i64 {
    let total = ((hours * 60 + minutes) * 60 + seconds) * rate.base + frames;
    if drop_frame {
        let drop = rate.base / 15;
        let total_minutes = hours * 60 + minutes;
        total - drop * (total_minutes - total_minutes / 10)
    } else {
        total
    }
}

/// The **bytes** of the data of a line: the hexadecimal of the format
/// and its abbreviations.
///
/// A half of a byte at the end is not one, and a letter that is not an
/// abbreviation of the format is not one either.
pub fn expand(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut half: Option<u8> = None;
    for character in text.chars().filter(|character| !character.is_whitespace()) {
        if let Some(digit) = character.to_digit(16) {
            match half.take() {
                Some(high) => out.push(high << 4 | digit as u8),
                None => half = Some(digit as u8),
            }
            continue;
        }
        if half.is_some() {
            return None;
        }
        let letter = character.to_ascii_uppercase();
        match letter {
            'G'..='O' => {
                for _ in 0..(letter as u8 - b'G' + 1) {
                    out.extend_from_slice(&[0xfa, 0x00, 0x00]);
                }
            }
            _ => out.extend_from_slice(ABBREV.iter().find(|abbrev| abbrev.0 == letter)?.1),
        }
    }
    if half.is_some() { None } else { Some(out) }
}

/// The **triplets** of the caption data of one line: the marker of each
/// pair and its two bytes.
///
/// The data is an ancillary packet with a caption distribution packet
/// in it, or the distribution packet itself.
fn cc_triplets(bytes: &[u8]) -> Option<Vec<[u8; 3]>> {
    let cdp = if bytes.starts_with(&[0x61, 0x01]) { bytes.get(3..)? } else { bytes };
    if !cdp.starts_with(&[0x96, 0x69]) {
        return None;
    }
    let len = (*cdp.get(2)? as usize).min(cdp.len());
    let cdp = &cdp[..len];
    let mut at = 7;
    let mut out = Vec::new();
    while at < cdp.len() {
        match cdp[at] {
            0x71 => at += 5,
            0x72 => {
                let count = (*cdp.get(at + 1)? & 0x1f) as usize;
                for k in 0..count {
                    let at = at + 2 + 3 * k;
                    out.push([*cdp.get(at)?, *cdp.get(at + 1)?, *cdp.get(at + 2)?]);
                }
                at += 2 + 3 * count;
            }
            0x73 => at += 2 + 7 * (*cdp.get(at + 1)? & 0x0f) as usize,
            0x74 => break,
            0x75..=0xef => at += 2 + *cdp.get(at + 1)? as usize,
            _ => return None,
        }
    }
    Some(out)
}

/// The **cues** of an MCC file.
///
/// The tree's own reader: the pairs of the field one are the CEA-608
/// pairs of the caption, and the CEA-708 service data is left alone.
pub fn cues_of(text: &str) -> Result<(Vec<Cue>, Vec<String>), String> {
    let mut warnings = Vec::new();
    let all: Vec<(usize, &str)> = text.lines().enumerate().collect();
    if all.len() > MAX_LINES {
        return Err(format!("the file has more than {MAX_LINES} lines"));
    }
    let mut lines = all.into_iter();
    // The **header** of the file: a file that does not say what it is
    // is not one.
    match lines.by_ref().find(|(_, line)| !line.trim().is_empty()) {
        Some((_, line)) if line.trim().starts_with("File Format=MacCaption_MCC") => {}
        _ => return Err("not a MacCaption MCC file".to_string()),
    }
    let mut rate = Rate::fps_29_97();
    let mut pairs: Vec<(i64, u8, u8)> = Vec::new();
    let mut any_608 = false;
    for (number, line) in lines {
        let line = line.trim_end();
        if line.trim().is_empty() || line.starts_with("//") {
            continue;
        }
        if let Some(value) = line.strip_prefix("Time Code Rate=") {
            rate = tc_rate(value)
                .ok_or_else(|| format!("line {}: the time code rate `{value}` is not one", number + 1))?;
            continue;
        }
        if line.contains('=') {
            // The other lines of the header are not data.
            continue;
        }
        let Some((timecode, data)) = line.trim_start().split_once(['\t', ' ']) else {
            continue;
        };
        // The timecode may carry the **frame rate** of the line behind a
        // point.
        let timecode = timecode.split('.').next().unwrap_or(timecode);
        let parts: Vec<i64> = timecode.split([':', ';']).filter_map(|part| part.parse().ok()).collect();
        let [hours, minutes, seconds, frames] = parts[..] else {
            return Err(format!("line {}: the timecode `{timecode}` is not one", number + 1));
        };
        let drop_frame = rate.drop_frame || timecode.contains(';');
        let frame = fields_to_frames(hours, minutes, seconds, frames, rate, drop_frame);
        let Some(bytes) = expand(data) else {
            warnings.push(format!("line {}: the caption data is not one", number + 1));
            continue;
        };
        let Some(triplets) = cc_triplets(&bytes) else {
            warnings.push(format!("line {}: the line is not a caption distribution packet", number + 1));
            continue;
        };
        for [marker, first, second] in triplets {
            // The pair is not **valid** when the marker says so.
            if marker & 0x04 == 0 {
                continue;
            }
            match marker & 0x03 {
                // The **field one**: the CEA-608 pairs.
                0 => {
                    any_608 |= first & 0x7f != 0 || second & 0x7f != 0;
                    if pairs.len() < MAX_PAIRS {
                        pairs.push((frame, first, second));
                    }
                }
                // The **field two** is not read.
                1 => {}
                // The **CEA-708** service data is not read here.
                _ => {}
            }
        }
    }
    let cues = if any_608 {
        let frames = crate::scc::decode_pairs(&pairs, rate.one_second());
        frames
            .into_iter()
            .map(|(start, end, text)| Cue {
                start: rate.milliseconds_of(start),
                end: rate.milliseconds_of(end),
                text,
            })
            .collect()
    } else {
        warnings.push("the file carries only CEA-708 data, and it is not read here".to_string());
        Vec::new()
    };
    Ok((cues, warnings))
}

/// The **cues** of an MCC file, as the answer holds them.
pub fn cues_as_json(cues: &[Cue]) -> Vec<Json> {
    crate::scc::cues_as_json(cues)
}

/// The **bytes** of a line, as the text of the format: the
/// hexadecimal of it and its abbreviations.
///
/// The tree's own rule, and the rule of the reader in reverse.
pub fn compress(bytes: &[u8]) -> String {
    let mut out = String::new();
    let mut at = 0;
    'outer: while at < bytes.len() {
        // The **padding** of the format is nine times at most in a row.
        let mut run = 0;
        while run < 9 && bytes[at + 3 * run..].starts_with(&[0xfa, 0x00, 0x00]) {
            run += 1;
        }
        if run > 0 {
            out.push((b'G' + run as u8 - 1) as char);
            at += 3 * run;
            continue;
        }
        for (letter, run) in ABBREV {
            if bytes[at..].starts_with(run) {
                out.push(letter);
                at += run.len();
                continue 'outer;
            }
        }
        out.push_str(&format!("{:02X}", bytes[at]));
        at += 1;
    }
    out
}

/// The **count** of the triplets of a packet.
const CC_COUNT: usize = 20;

/// The **rate code** of the format: twenty-nine and ninety-seven.
const RATE_CODE: u8 = 4;

/// The **bytes** of a caption distribution packet for one frame, with
/// the CEA-608 pair of the field one.
pub fn build_cdp(seq: u16, pair: Option<(u8, u8)>) -> Vec<u8> {
    let mut data: Vec<u8> = Vec::with_capacity(CC_COUNT * 3);
    match pair {
        Some((first, second)) => {
            data.extend_from_slice(&[0xfc, crate::scc::with_parity(first), crate::scc::with_parity(second)])
        }
        None => data.extend_from_slice(&[0xfc, 0x80, 0x80]),
    }
    // The **field two** carries nothing here.
    data.extend_from_slice(&[0xfd, 0x80, 0x80]);
    while data.len() < CC_COUNT * 3 {
        data.extend_from_slice(&[0xfa, 0x00, 0x00]);
    }
    let len = 7 + 2 + data.len() + 4;
    let [high, low] = seq.to_be_bytes();
    let mut cdp =
        vec![0x96, 0x69, len as u8, RATE_CODE << 4 | 0x0f, 0x43, high, low, 0x72, 0xe0 | CC_COUNT as u8];
    cdp.extend_from_slice(&data);
    cdp.extend_from_slice(&[0x74, high, low]);
    let sum: u32 = cdp.iter().map(|byte| *byte as u32).sum();
    cdp.push(((256 - sum % 256) % 256) as u8);
    let mut out = vec![0x61, 0x01, cdp.len() as u8];
    out.extend_from_slice(&cdp);
    // The **checksum** of the ancillary packet: the low eight bits of
    // the sum of the identifier, the count and the data.
    let ancillary: u32 = out.iter().map(|byte| *byte as u32).sum();
    out.push((ancillary & 0xff) as u8);
    out
}

/// The **cues** of a track, as an MCC file.
///
/// The tree's own rule: the file carries the CEA-608 pairs of the
/// field one, and the CEA-708 service data of the tree's writer is
/// not here — the tree writes the two apart, and this writes the first.
pub fn write(cues: &[(i64, i64, String)], drop_frame: bool) -> String {
    let schedule = crate::scc::schedule(cues);
    let mut out = String::from("File Format=MacCaption_MCC V1.0\n\n");
    out.push_str("///////////////////////////////////////////////////////////////////////////////////\n");
    out.push_str("// Closed captions written by craftkit: SMPTE ST 334-2 caption distribution packets\n");
    out.push_str("// carrying CEA-608 field 1 data; empty frames omitted.\n");
    out.push_str("///////////////////////////////////////////////////////////////////////////////////\n\n");
    // The **identifier** of the file: a hash of the cues of it.
    let count = cues.len() as u64;
    let hash = cues.iter().fold(0xcbf2_9ce4_8422_2325u64 ^ count, |hash, cue| {
        (hash ^ cue.0 as u64 ^ (cue.1 as u64).rotate_left(17)).wrapping_mul(0x100_0000_01b3)
    });
    out.push_str(&format!(
        "UUID={:08X}-{:04X}-4{:03X}-8{:03X}-{:012X}\n",
        hash >> 32,
        (hash >> 16) & 0xffff,
        hash & 0xfff,
        (hash >> 20) & 0xfff,
        hash.rotate_left(29) & 0xffff_ffff_ffff
    ));
    out.push_str("Creation Program=craftkit\n");
    out.push_str(&format!("Time Code Rate={}\n\n", if drop_frame { "30DF" } else { "30" }));
    let mut seq = 0u16;
    for (frame, pair) in schedule {
        let bytes = build_cdp(seq, Some(pair));
        out.push_str(&format!("{}\t{}\n", crate::scc::format_timecode(frame, drop_frame), compress(&bytes)));
        seq = seq.wrapping_add(1);
    }
    out
}
