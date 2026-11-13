//! A caption file of **any format that is read** may be imported as a
//! caption track.
//!
//! The formats are the tree's own: a file that may be read may be
//! imported, and the reading is the one of the reader.

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

/// A **sequence** that runs at **25** frames a second.
fn a_sequence(session: &mut Session) -> u64 {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the sequence", "bin": bin, "item": 1, "duration": 100 }))
        .expect("the sequence is added");
    let sequence = outcome.data["id"].as_u64().expect("the sequence has an id");
    session
        .execute("film.sequence.rate.set", &json!({ "sequence": sequence, "rate": "25" }))
        .expect("the rate is set");
    sequence
}

/// A file of this test's own, so that two tests do not fight over one
/// file.
fn a_file(name: &str, text: &str) -> String {
    let directory =
        craftkit_scratch::dir(&format!("craftkit-v2-captionimportformats-{}", std::process::id()))
            .expect("the directory is made");
    let path = directory.join(name);
    std::fs::write(&path, text).expect("the file is written");
    path.to_string_lossy().to_string()
}

/// The **cues** of a track, as the answer holds them.
fn cues_of(session: &mut Session, track: u64) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome =
        session.execute("film.caption.read", &json!({ "track": track })).expect("the cues are read");
    outcome.data["cues"].as_array().expect("the cues are a list").clone()
}

/// An **SCC file** is imported as a track: the caption of the file is
/// the cue of the track, snapped to the frames of the sequence.
#[test]
fn an_scc_file_is_imported_as_a_track() {
    let mut session = session();
    let sequence = a_sequence(&mut session);
    let path = a_file(
        "the-captions.scc",
        "Scenarist_SCC V1.0\n\n00:00:00;20\t9420 9420 94ae 94ae 94d0 94d0 c845 4c4c 4f80 942f 942f\n\n00:00:03;00\t942c 942c\n",
    );
    let outcome = session
        .execute("film.caption.import", &json!({ "sequence": sequence, "path": path, "format": "scc" }))
        .expect("the file is imported");
    let track = outcome.data["track"].as_u64().expect("the track has an id");
    assert_eq!(outcome.data["cues"], json!(1), "and one caption was imported: {outcome:?}");
    let cues = cues_of(&mut session, track);
    assert_eq!(cues.len(), 1, "the caption is there: {cues:?}");
    assert_eq!(cues[0]["text"], json!("HELLO"), "and it is the caption of the file: {cues:?}");
    // The caption is shown at the frame twenty-nine of the format, and
    // it is cleared at the frame ninety: they are the frames of the
    // sequence.
    assert_eq!(cues[0]["start"], json!(24), "and it begins at its frame: {cues:?}");
    assert_eq!(cues[0]["end"], json!(75), "and it ends at its own: {cues:?}");
}

/// The **parity** of a byte: the odd parity of the format.
fn parity(byte: u8) -> u8 {
    let byte = byte & 0x7f;
    if byte.count_ones().is_multiple_of(2) { byte | 0x80 } else { byte }
}

/// The **bytes** of a caption distribution packet for one frame, with
/// the CEA-608 pair of the field one.
fn distribution(seq: u16, pair: (u8, u8)) -> Vec<u8> {
    let mut data: Vec<u8> = vec![0xfc, parity(pair.0), parity(pair.1)];
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

/// An **MCC file** is imported as a track: the caption of the file is
/// the cue of the track.
#[test]
fn an_mcc_file_is_imported_as_a_track() {
    let mut session = session();
    let sequence = a_sequence(&mut session);
    let mut text = String::from("File Format=MacCaption_MCC V1.0\n\nTime Code Rate=30DF\n\n");
    // The **pairs** of a pop-on caption, one to a line.
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
        let bytes = distribution(at as u16, pair);
        let data: String = bytes.iter().map(|byte| format!("{byte:02X}")).collect();
        text.push_str(&format!("00:00:00;{:02}\t{data}\n", 20 + at));
    }
    let path = a_file("the-captions.mcc", &text);
    let outcome = session
        .execute("film.caption.import", &json!({ "sequence": sequence, "path": path, "format": "mcc" }))
        .expect("the file is imported");
    let track = outcome.data["track"].as_u64().expect("the track has an id");
    let cues = cues_of(&mut session, track);
    assert_eq!(cues.len(), 1, "the caption is there: {cues:?}");
    assert_eq!(cues[0]["text"], json!("HELLO"), "and it is the caption of the file: {cues:?}");
}

/// A format that is **not one** is refused by name, and the formats
/// that may be imported are named.
#[test]
fn a_format_that_is_not_one_is_refused_by_name() {
    let mut session = session();
    let sequence = a_sequence(&mut session);
    let path = a_file("the-captions.txt", "not a caption file");
    let error = session
        .execute("film.caption.import", &json!({ "sequence": sequence, "path": path, "format": "nope" }))
        .expect_err("the format is refused");
    let message = error.to_string();
    assert!(message.contains("nope"), "the format is named: {message}");
    assert!(message.contains("scc"), "and the formats are named: {message}");
    assert!(message.contains("mcc"), "and the formats are named: {message}");
}
