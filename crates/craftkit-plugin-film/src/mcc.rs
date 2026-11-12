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
//! - The **CEA-708** service data is carried beside them: the file
//!   writes the two, and a file that carries only the service data is
//!   read with the **service decoder** — the tree's own rule is that
//!   the two streams are alternatives and the field one comes first.

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
    let mut service: Vec<(i64, bool, u8, u8)> = Vec::new();
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
                // The **CEA-708 service data**: the pairs of the DTVCC
                // packets, and the first of a packet says that it
                // starts here.
                other => {
                    if service.len() < MAX_PAIRS {
                        service.push((frame, other == 3, first, second));
                    }
                }
            }
        }
    }
    let cues = if any_608 {
        // The **field one** comes first: the tree's own rule is that
        // the two streams are alternatives.
        let frames = crate::scc::decode_pairs(&pairs, rate.one_second());
        frames
            .into_iter()
            .map(|(start, end, text)| Cue {
                start: rate.milliseconds_of(start),
                end: rate.milliseconds_of(end),
                text,
            })
            .collect()
    } else if service.is_empty() {
        warnings.push("the file carries no caption data".to_string());
        Vec::new()
    } else {
        cues_708_of(&service, &rate)
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
pub fn build_cdp(seq: u16, pair: Option<(u8, u8)>, service: &[(bool, u8, u8)]) -> Vec<u8> {
    let mut data: Vec<u8> = Vec::with_capacity(CC_COUNT * 3);
    match pair {
        Some((first, second)) => {
            data.extend_from_slice(&[0xfc, crate::scc::with_parity(first), crate::scc::with_parity(second)])
        }
        None => data.extend_from_slice(&[0xfc, 0x80, 0x80]),
    }
    // The **field two** carries nothing here.
    data.extend_from_slice(&[0xfd, 0x80, 0x80]);
    // The **CEA-708 service data** follows: the pairs of the DTVCC
    // packets, the first of a packet saying that it starts here.
    for (start, first, second) in service.iter().take(DTVCC_PAIRS) {
        data.extend_from_slice(&[if *start { 0xff } else { 0xfe }, *first, *second]);
    }
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
/// field one **and** the CEA-708 service data beside them.
pub fn write(cues: &[(i64, i64, String)], drop_frame: bool) -> String {
    write_with(cues, drop_frame, true, true)
}

/// The **cues** of a track, as an MCC file, with the **streams** that
/// are written.
///
/// The tree's own rule: the CEA-608 field one and the CEA-708 service
/// one are two streams of their own, and either may be left out.
pub fn write_with(cues: &[(i64, i64, String)], drop_frame: bool, cea_608: bool, cea_708: bool) -> String {
    let field_one = if cea_608 { crate::scc::schedule(cues) } else { Default::default() };
    let service = if cea_708 { schedule_708(cues) } else { Default::default() };
    let mut out = String::from("File Format=MacCaption_MCC V1.0\n\n");
    out.push_str("///////////////////////////////////////////////////////////////////////////////////\n");
    out.push_str("// Closed captions written by craftkit: SMPTE ST 334-2 caption distribution packets\n");
    out.push_str("// carrying CEA-608 field 1 and CEA-708 service 1 data; empty frames omitted.\n");
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
    // The two **schedules** are put in a row: a frame that carries
    // either of them is written, and the others are not.
    let mut field_one = field_one.into_iter().peekable();
    let mut service = service.into_iter().peekable();
    let mut seq = 0u16;
    while let Some(frame) = field_one
        .peek()
        .map(|(frame, _)| *frame)
        .into_iter()
        .chain(service.peek().map(|(frame, _)| *frame))
        .min()
    {
        let pair = field_one.next_if(|(held, _)| *held == frame).map(|(_, pair)| pair);
        let data = service.next_if(|(held, _)| *held == frame).map(|(_, pairs)| pairs).unwrap_or_default();
        let bytes = build_cdp(seq, pair, &data);
        out.push_str(&format!("{}\t{}\n", crate::scc::format_timecode(frame, drop_frame), compress(&bytes)));
        seq = seq.wrapping_add(1);
    }
    out
}

// ---------------------------------------------------------------------------------------------
// CEA-708 service data
// ---------------------------------------------------------------------------------------------

/// The **G2** characters the service writes and reads, after the
/// escape that introduces them.
const G2: &[(u8, char)] = &[
    (0x25, '…'),
    (0x2a, 'Š'),
    (0x2c, 'Œ'),
    (0x31, '‘'),
    (0x32, '’'),
    (0x33, '“'),
    (0x34, '”'),
    (0x35, '•'),
    (0x39, '™'),
    (0x3a, 'š'),
    (0x3c, 'œ'),
    (0x3d, '℠'),
    (0x3f, 'Ÿ'),
    (0x76, '⅛'),
    (0x77, '⅜'),
    (0x78, '⅝'),
    (0x79, '⅞'),
];

/// The **atoms** of a character: a command with its parameters, or the
/// character itself.
fn char_atom(character: char) -> Vec<u8> {
    match character {
        '♪' => vec![0x7f],
        ' '..='~' => vec![character as u8],
        '\u{a0}'..='\u{ff}' => vec![character as u32 as u8],
        _ => match G2.iter().find(|(_, held)| *held == character) {
            Some((byte, _)) => vec![0x10, *byte],
            None => vec![b'?'],
        },
    }
}

/// The atoms that **define** a window hidden and load the rows into
/// it.
fn load_atoms(window: u8, rows: &[String]) -> Vec<Vec<u8>> {
    let count = rows.len().clamp(1, 12);
    let columns = rows.iter().map(|row| row.chars().count()).max().unwrap_or(1).clamp(1, 42);
    let mut out = vec![
        // DefineWindow: hidden, the row and column locked, the relative
        // position ninety per cent down and fifty per cent across,
        // anchored at its bottom centre, the rows and the columns, the
        // window style one and the pen style one.
        vec![0x98 + window, 0x18, 0x80 | 90, 50, 7 << 4 | (count as u8 - 1), (columns as u8 - 1), 1 << 3 | 1],
        vec![0x88, 1 << window], // ClearWindows
    ];
    for (index, row) in rows.iter().enumerate() {
        if index > 0 {
            out.push(vec![0x0d]);
        }
        out.extend(row.chars().map(char_atom));
    }
    out
}

/// Pack the atoms into **DTVCC packets** of the service one: a packet
/// holds thirty-one bytes at the most, so one fits in a frame.
fn packets(atoms: &[Vec<u8>], sequence: &mut u8) -> Vec<Vec<u8>> {
    let mut blocks: Vec<Vec<u8>> = Vec::new();
    let mut current: Vec<u8> = Vec::new();
    for atom in atoms {
        if current.len() + atom.len() > 31 {
            blocks.push(std::mem::take(&mut current));
        }
        current.extend_from_slice(atom);
    }
    if !current.is_empty() {
        blocks.push(current);
    }
    blocks
        .into_iter()
        .map(|block| {
            let mut packet = vec![0, 1 << 5 | block.len() as u8];
            packet.extend(block);
            if packet.len() % 2 == 1 {
                packet.push(0);
            }
            packet[0] = (*sequence & 3) << 6 | (packet.len() / 2) as u8;
            *sequence = sequence.wrapping_add(1);
            packet
        })
        .collect()
}

/// The **pairs** of a packet: the first pair says that the packet
/// starts here.
fn pairs_of(packet: &[u8]) -> Vec<(bool, u8, u8)> {
    packet
        .chunks(2)
        .enumerate()
        .map(|(index, chunk)| {
            let first = chunk.first().copied().unwrap_or(0);
            let second = chunk.get(1).copied().unwrap_or(0);
            (index == 0, first, second)
        })
        .collect()
}

/// The **DTVCC** data of a file: the pairs of each frame.
struct Dtv {
    frames: std::collections::BTreeMap<i64, Vec<(bool, u8, u8)>>,
}

impl Dtv {
    /// How many pairs a frame has room for.
    fn room(&self, frame: i64) -> usize {
        DTVCC_PAIRS - self.frames.get(&frame).map_or(0, Vec::len)
    }

    /// Put a packet in the first frame at or after `frame` with room
    /// for it, and give that frame back.
    fn place(&mut self, mut frame: i64, packet: &[u8]) -> i64 {
        let pairs = pairs_of(packet);
        while self.room(frame) < pairs.len() {
            frame += 1;
        }
        self.frames.entry(frame).or_default().extend(pairs);
        frame
    }
}

/// The most **pairs** a frame may carry.
const DTVCC_PAIRS: usize = 15;

/// Schedule the **CEA-708 service one** captions: the tree's own rule.
///
/// The captions are **pop-on**: a window is defined and loaded, and it
/// is shown when its cue begins and hidden when it ends.
fn schedule_708(cues: &[(i64, i64, String)]) -> std::collections::BTreeMap<i64, Vec<(bool, u8, u8)>> {
    let mut dtv = Dtv { frames: std::collections::BTreeMap::new() };
    let mut sequence = 0u8;
    let mut cursor = 0i64;
    let mut cues: Vec<&(i64, i64, String)> = cues.iter().filter(|cue| cue.1 > cue.0).collect();
    cues.sort_by_key(|cue| cue.0);
    for (index, cue) in cues.iter().enumerate() {
        let window = (index % 2) as u8;
        // The cues are in **frames**: the same grid the field one is
        // written on.
        let start = cue.0.max(cursor);
        let end = cue.1;
        let rows = crate::scc::wrap_rows(&crate::scc::plain_text(&cue.2));
        let load = packets(&load_atoms(window, &rows), &mut sequence);
        let mut frame = (start - load.len() as i64).max(cursor);
        let mut last = frame;
        for packet in &load {
            last = dtv.place(frame, packet);
            frame = last;
        }
        let show = packets(&[vec![0x8a, 1 << (1 - window)], vec![0x89, 1 << window]], &mut sequence);
        let shown = dtv.place(start.max(last), show.first().map(Vec::as_slice).unwrap_or(&[]));
        cursor = shown + 1;
        let next = cues.get(index + 1).map(|next| next.0);
        if next != Some(end) {
            let hide = packets(&[vec![0x8a, 1 << window]], &mut sequence);
            dtv.place(end.max(cursor), hide.first().map(Vec::as_slice).unwrap_or(&[]));
        }
    }
    dtv.frames
}

/// A **window** of the service: the rows it holds and whether it is
/// shown.
#[derive(Clone, Default)]
struct Window {
    /// Whether the window is **defined** at all.
    defined: bool,
    /// Whether the window is **shown**.
    visible: bool,
    /// The rows of it.
    rows: Vec<String>,
    /// How many rows it may hold.
    max_rows: usize,
    /// How many characters a row may hold.
    max_cols: usize,
}

/// The **decoder** of the service one.
struct Dec708 {
    windows: [Window; 8],
    current: usize,
    packet: Vec<u8>,
    want: usize,
    open: Option<(i64, String)>,
    cues: Vec<(i64, i64, String)>,
}

impl Dec708 {
    /// A decoder with nothing in it.
    fn new() -> Self {
        Self {
            windows: Default::default(),
            current: 0,
            packet: Vec::new(),
            want: 0,
            open: None,
            cues: Vec::new(),
        }
    }

    /// The **text** that is shown now.
    fn visible_text(&self) -> String {
        let mut lines: Vec<String> = Vec::new();
        for window in self.windows.iter().filter(|window| window.defined && window.visible) {
            lines.extend(window.rows.iter().map(|row| row.trim().to_string()).filter(|row| !row.is_empty()));
        }
        lines.join("\n")
    }

    /// The text that is shown has **changed**: the cue that was open
    /// ends here, and the new one begins.
    fn sync(&mut self, frame: i64) {
        let now = self.visible_text();
        if self.open.as_ref().map(|open| open.1.as_str()) == Some(now.as_str())
            || (self.open.is_none() && now.is_empty())
        {
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

    /// A **pair** of the service data.
    fn pair(&mut self, frame: i64, start: bool, first: u8, second: u8) {
        if start {
            self.packet.clear();
            let size = (first & 0x3f) as usize;
            self.want = if size == 0 { 128 } else { size * 2 };
        } else if self.want == 0 {
            return;
        }
        self.packet.extend_from_slice(&[first, second]);
        if self.packet.len() >= self.want {
            let packet = std::mem::take(&mut self.packet);
            self.want = 0;
            if let Some(rest) = packet.get(1..) {
                self.service_blocks(rest);
            }
            self.sync(frame);
        }
    }

    /// The **service blocks** of a packet.
    fn service_blocks(&mut self, mut bytes: &[u8]) {
        while let Some(head) = bytes.first().copied() {
            if head == 0 {
                break;
            }
            let service = head >> 5;
            let length = (head & 0x1f) as usize;
            let (service, skip) =
                if service == 7 { (bytes.get(1).map_or(0, |value| value & 0x3f), 2) } else { (service, 1) };
            let data = bytes.get(skip..skip + length).unwrap_or(&[]);
            if service == 1 {
                self.service_data(data);
            }
            bytes = bytes.get(skip + length..).unwrap_or(&[]);
        }
    }

    /// The **window** that is written into now.
    fn window(&mut self) -> &mut Window {
        let index = self.current.min(self.windows.len() - 1);
        &mut self.windows[index]
    }

    /// Put a **character** into the window that is written into now.
    fn put(&mut self, character: char) {
        let window = self.window();
        match window.rows.last_mut() {
            Some(row) if row.chars().count() < window.max_cols => row.push(character),
            None if window.max_rows > 0 && window.max_cols > 0 => window.rows.push(character.to_string()),
            _ => {}
        }
    }

    /// Do something to the **windows** a bitmask names.
    fn bitmap(&mut self, mask: u8, each: impl Fn(&mut Window)) {
        for (index, window) in self.windows.iter_mut().enumerate() {
            if mask & (1 << index) != 0 {
                each(window);
            }
        }
    }

    /// The **service data** of the service one.
    fn service_data(&mut self, data: &[u8]) {
        let mut at = 0usize;
        while at < data.len() {
            let command = data[at];
            at += 1;
            let argument = |index: usize| data.get(index).copied().unwrap_or(0);
            match command {
                0x00 | 0x03 => {}
                0x08 => {
                    if let Some(row) = self.window().rows.last_mut() {
                        row.pop();
                    }
                }
                0x0c => self.window().rows = vec![String::new()],
                0x0d => {
                    let window = self.window();
                    if window.rows.len() < window.max_rows {
                        window.rows.push(String::new());
                    }
                }
                0x0e => {
                    if let Some(row) = self.window().rows.last_mut() {
                        row.clear();
                    }
                }
                0x10 => {
                    let value = argument(at);
                    at += 1;
                    match value {
                        0x00..=0x07 => {}
                        0x08..=0x0f => at += 1,
                        0x10..=0x17 => at += 2,
                        0x18..=0x1f => at += 3,
                        // The space and the two above it are the
                        // tree's own: the others are the G2 set.
                        0x20..=0x21 => self.put(' '),
                        0x22..=0x7f => {
                            if let Some((_, character)) = G2.iter().find(|(byte, _)| *byte == value) {
                                self.put(*character);
                            }
                        }
                        0x80..=0x87 => at += 4,
                        0x88..=0x8f => at += 5,
                        0x90..=0x9f => at += 1 + (argument(at) & 0x1f) as usize,
                        _ => {}
                    }
                }
                0x11..=0x17 => at += 1,
                0x18..=0x1f => at += 2,
                0x20..=0x7e => self.put(command as char),
                0x7f => self.put('♪'),
                0x80..=0x87 => self.current = (command - 0x80) as usize,
                0x88 => {
                    let mask = argument(at);
                    at += 1;
                    self.bitmap(mask, |window| window.rows = vec![String::new()]);
                }
                0x89 => {
                    let mask = argument(at);
                    at += 1;
                    self.bitmap(mask, |window| window.visible = true);
                }
                0x8a => {
                    let mask = argument(at);
                    at += 1;
                    self.bitmap(mask, |window| window.visible = false);
                }
                0x8b => {
                    let mask = argument(at);
                    at += 1;
                    self.bitmap(mask, |window| window.visible = !window.visible);
                }
                0x8c => {
                    let mask = argument(at);
                    at += 1;
                    self.bitmap(mask, |window| *window = Window::default());
                }
                0x8d => at += 1,
                0x8e | 0x8f | 0x93..=0x96 => {}
                0x90 | 0x92 => at += 2,
                0x91 => at += 3,
                0x97 => at += 4,
                0x98..=0x9f => {
                    let index = (command - 0x98) as usize;
                    let visible = argument(at) & 0x20 != 0;
                    let max_rows = (argument(at + 3) & 0x0f) as usize + 1;
                    let max_cols = (argument(at + 4) & 0x3f) as usize + 1;
                    let window = &mut self.windows[index.min(self.windows.len() - 1)];
                    if !window.defined {
                        *window =
                            Window { defined: true, visible, rows: vec![String::new()], max_rows, max_cols };
                    } else {
                        window.visible = visible;
                        window.max_rows = max_rows;
                        window.max_cols = max_cols;
                        window.rows.truncate(max_rows);
                        for row in window.rows.iter_mut() {
                            if let Some((at, _)) = row.char_indices().nth(max_cols) {
                                row.truncate(at);
                            }
                        }
                        if window.rows.is_empty() {
                            window.rows.push(String::new());
                        }
                    }
                    self.current = index;
                    at += 6;
                }
                0xa0..=0xff => self.put(command as char),
                _ => {}
            }
        }
    }
}

/// The **cues** of the service data, as the reader finds them.
///
/// The tree's own rule: the pairs are the DTVCC pairs of the frames,
/// and a file that carries only them is read with this decoder.
fn cues_708_of(pairs: &[(i64, bool, u8, u8)], rate: &Rate) -> Vec<Cue> {
    let mut decoder = Dec708::new();
    let mut last = 0i64;
    for (frame, start, first, second) in pairs {
        decoder.pair(*frame, *start, *first, *second);
        last = *frame + 1;
    }
    if let Some((start, text)) = decoder.open.take() {
        decoder.cues.push((start, last.max(start + 1), text));
    }
    decoder
        .cues
        .iter()
        .map(|(start, end, text)| Cue {
            start: rate.milliseconds_of(*start),
            end: rate.milliseconds_of(*end),
            text: text.clone(),
        })
        .collect()
}
