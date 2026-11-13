//! The **cues** of an MCC file.
//!
//! A MacCaption MCC file carries the caption data of a frame in every
//! line, and the pairs of the field one of a distribution packet are
//! the CEA-608 pairs of the caption. The rules are the tree's own.
//!
//! The files of this test are **built by hand** from the rules of the
//! format, because the tree's writer of an MCC file is not here: it
//! writes the CEA-708 service data beside the CEA-608 pairs, and that
//! is not here either.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_film::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the film battery.
fn ecosystem() -> Ecosystem {
    Ecosystem::from_inventory().expect("the ecosystem is built")
}

/// A session over the battery's own contributions.
fn session() -> Session {
    let ecosystem = ecosystem();
    Session::with_formats(
        ecosystem.commands,
        ecosystem.kinds,
        ecosystem.formats,
        ecosystem.migrations,
        ecosystem.evaluators,
    )
}

/// A file of this test's own, so that two tests do not fight over one
/// file.
fn a_file(name: &str, text: &str) -> String {
    let directory = craftkit_scratch::dir(&format!("craftkit-v2-mcc-read-{}", std::process::id()))
        .expect("the directory is made");
    let path = directory.join(name);
    std::fs::write(&path, text).expect("the file is written");
    path.to_string_lossy().to_string()
}

/// The **cues** of a file, as the answer holds them.
fn cues_of(session: &mut Session, path: &str) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome = session
        .execute("film.captions.read", &json!({ "path": path, "format": "mcc" }))
        .expect("the file is read");
    outcome.data["cues"].as_array().expect("the cues are a list").clone()
}

/// The **warnings** of a file, as the answer holds them.
fn warnings_of(session: &mut Session, path: &str) -> Vec<String> {
    let outcome = session
        .execute("film.captions.read", &json!({ "path": path, "format": "mcc" }))
        .expect("the file is read");
    outcome.data["warnings"]
        .as_array()
        .expect("the warnings are a list")
        .iter()
        .filter_map(|warning| warning.as_str().map(str::to_string))
        .collect()
}

/// The **parity** of a byte: the odd parity of the format.
fn parity(byte: u8) -> u8 {
    let byte = byte & 0x7f;
    if byte.count_ones().is_multiple_of(2) { byte | 0x80 } else { byte }
}

/// The **bytes** of a caption distribution packet for one frame, with
/// the CEA-608 pair of the field one.
fn distribution(seq: u16, pair: Option<(u8, u8)>) -> Vec<u8> {
    let mut data: Vec<u8> = Vec::new();
    match pair {
        Some((first, second)) => data.extend_from_slice(&[0xfc, parity(first), parity(second)]),
        None => data.extend_from_slice(&[0xfc, 0x80, 0x80]),
    }
    data.extend_from_slice(&[0xfd, 0x80, 0x80]);
    while data.len() < 60 {
        data.extend_from_slice(&[0xfa, 0x00, 0x00]);
    }
    let len = 7 + 2 + data.len() + 4;
    let [high, low] = seq.to_be_bytes();
    let mut cdp = vec![0x96, 0x69, len as u8, 0x4f, 0x43, high, low, 0x72, 0xe0 | 20];
    cdp.extend_from_slice(&data);
    cdp.extend_from_slice(&[0x74, high, low]);
    let sum: u32 = cdp.iter().map(|byte| *byte as u32).sum();
    cdp.push(((256 - sum % 256) % 256) as u8);
    let mut out = vec![0x61, 0x01, cdp.len() as u8];
    out.extend_from_slice(&cdp);
    let ancillary: u32 = out.iter().map(|byte| *byte as u32).sum();
    out.push((ancillary & 0xff) as u8);
    out
}

/// The **text** of the bytes of a line: the hexadecimal of the format
/// with its abbreviations.
fn abbreviate(bytes: &[u8]) -> String {
    const ABBREV: [(char, &[u8]); 7] = [
        ('P', &[0xfb, 0x80, 0x80]),
        ('Q', &[0xfc, 0x80, 0x80]),
        ('R', &[0xfd, 0x80, 0x80]),
        ('S', &[0x96, 0x69]),
        ('T', &[0x61, 0x01]),
        ('U', &[0xe1, 0x00, 0x00, 0x00]),
        ('Z', &[0x00]),
    ];
    let mut out = String::new();
    let mut at = 0;
    'outer: while at < bytes.len() {
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

/// The **timecode** of a frame of the format, at twenty-nine and
/// ninety-seven with the drop-frame mark.
fn timecode(frame: i64) -> String {
    let base = 30i64;
    let drop = 2i64;
    let mut n = frame;
    let per_minute = base * 60 - drop;
    let per_ten = per_minute * 10 + drop;
    let ten = n / per_ten;
    let rest = n % per_ten;
    n += drop * 9 * ten;
    if rest > drop {
        n += drop * ((rest - drop) / per_minute);
    }
    format!("{:02}:{:02}:{:02};{:02}", n / (base * 3600), (n / (base * 60)) % 60, (n / base) % 60, n % base)
}

/// The **file** of a caption: the pairs of the field one, one to a
/// line, and the lines of the header.
fn a_file_of(pairs: &[(i64, Option<(u8, u8)>)]) -> String {
    let mut out = String::from("File Format=MacCaption_MCC V1.0\n\nTime Code Rate=30DF\n\n");
    for (at, (frame, pair)) in pairs.iter().enumerate() {
        let data = abbreviate(&distribution(at as u16, *pair));
        out.push_str(&format!("{}\t{}\n\n", timecode(*frame), data));
    }
    out
}

/// The **pairs** of the caption of the tree's own test: the load, the
/// text and the end of the caption.
fn the_pairs() -> Vec<(i64, Option<(u8, u8)>)> {
    let mut pairs: Vec<(i64, Option<(u8, u8)>)> = Vec::new();
    for (at, pair) in [
        (0x14u8, 0x20u8),
        (0x14, 0x20),
        (0x14, 0x2e),
        (0x14, 0x2e),
        (0x14, 0x50),
        (0x14, 0x50),
        (0x48, 0x45),
        (0x4c, 0x4c),
        (0x4f, 0x80),
        (0x14, 0x2f),
        (0x14, 0x2f),
    ]
    .into_iter()
    .enumerate()
    {
        pairs.push((20 + at as i64, Some(pair)));
    }
    // The caption is cleared at the frame ninety.
    pairs.push((90, Some((0x14, 0x2c))));
    pairs
}

/// A **caption** is read: it is loaded, it is shown, and it is
/// cleared.
#[test]
fn a_caption_is_read_from_an_mcc_file() {
    let mut session = session();
    let path = a_file("popon.mcc", &a_file_of(&the_pairs()));
    let cues = cues_of(&mut session, &path);
    assert_eq!(cues.len(), 1, "there is one caption: {cues:?}");
    assert_eq!(cues[0]["text"], json!("HELLO"), "and it says HELLO: {cues:?}");
    // The caption is shown at the frame twenty-nine — the end of the
    // caption is the tenth pair — and it is cleared at the frame
    // ninety.
    assert_eq!(cues[0]["start"], json!(967), "it begins at its frame: {cues:?}");
    assert_eq!(cues[0]["end"], json!(3003), "and it ends at its own: {cues:?}");
}

/// The **abbreviations** of the format are read: a line whose data is
/// written with them holds the same caption as a line whose data is
/// written in hexadecimal.
#[test]
fn the_abbreviations_of_the_format_are_read() {
    let mut session = session();
    let pairs = the_pairs();
    let mut text = String::from("File Format=MacCaption_MCC V1.0\n\nTime Code Rate=30DF\n\n");
    for (at, (frame, pair)) in pairs.iter().enumerate() {
        let data = abbreviate(&distribution(at as u16, *pair));
        assert!(data.contains('T'), "the header of the packet is abbreviated: {data}");
        text.push_str(&format!("{}\t{}\n\n", timecode(*frame), data));
    }
    let path = a_file("abbreviated.mcc", &text);
    let cues = cues_of(&mut session, &path);
    assert_eq!(cues.len(), 1, "there is one caption: {cues:?}");
    assert_eq!(cues[0]["text"], json!("HELLO"), "and it says HELLO: {cues:?}");
}

/// A file that does not say what it is is **refused by name**.
#[test]
fn a_file_that_is_not_one_is_refused_by_name() {
    let mut session = session();
    let path = a_file("not-mcc.mcc", "Scenarist_SCC V1.0\n\n00:00:00;00\t9420\n");
    let error = session
        .execute("film.captions.read", &json!({ "path": path, "format": "mcc" }))
        .expect_err("the file is refused");
    let message = error.to_string();
    assert!(message.contains("MacCaption"), "the format is named: {message}");
}

/// A line whose **data** is not one is a **warning**: the file is read
/// and the line is left out.
#[test]
fn a_line_whose_data_is_not_one_is_a_warning() {
    let mut session = session();
    let mut text = a_file_of(&the_pairs());
    text.push_str("00:00:05;00\tnot data at all\n\n");
    let path = a_file("bad-data.mcc", &text);
    let cues = cues_of(&mut session, &path);
    assert_eq!(cues.len(), 1, "the caption of the file is still read: {cues:?}");
    let warnings = warnings_of(&mut session, &path);
    assert_eq!(warnings.len(), 1, "there is one warning: {warnings:?}");
    assert!(warnings[0].contains("caption data"), "and it says what is wrong: {warnings:?}");
}

/// A file that carries **no caption data** at all — neither the field
/// one nor the service data — has no cues here, and it says so.
///
/// The CEA-708 service data **is** read now (§1473): the warning of
/// this file is about the data it does not have, and not about the
/// service data it leaves out.
#[test]
fn a_file_of_no_caption_data_says_so() {
    let mut session = session();
    let text = a_file_of(&[(20, None), (21, None)]);
    let path = a_file("only-708.mcc", &text);
    let cues = cues_of(&mut session, &path);
    assert!(cues.is_empty(), "there is no caption: {cues:?}");
    let warnings = warnings_of(&mut session, &path);
    assert_eq!(warnings.len(), 1, "there is one warning: {warnings:?}");
    assert!(warnings[0].contains("no caption data"), "and it says what is not there: {warnings:?}");
}

/// The **rate** of the timecode is the one the header names, and a rate
/// that is not one is refused by name and with the line it is on.
#[test]
fn the_rate_of_the_timecode_is_the_one_the_header_names() {
    let mut session = session();
    let text = "File Format=MacCaption_MCC V1.0\n\nTime Code Rate=60DF\n\n00:00:01;00\tnot data\n\n";
    let path = a_file("rate.mcc", text);
    let error = session
        .execute("film.captions.read", &json!({ "path": path, "format": "mcc" }))
        .expect("the file is read");
    assert!(
        error.data["warnings"].as_array().is_some_and(|warnings| !warnings.is_empty()),
        "the bad line is a warning: {error:?}"
    );
    let text = "File Format=MacCaption_MCC V1.0\n\nTime Code Rate=12\n\n00:00:01;00\tnot data\n\n";
    let path = a_file("bad-rate.mcc", text);
    let error = session
        .execute("film.captions.read", &json!({ "path": path, "format": "mcc" }))
        .expect_err("the rate is refused");
    let message = error.to_string();
    assert!(message.contains("time code rate"), "the rate is named: {message}");
}
