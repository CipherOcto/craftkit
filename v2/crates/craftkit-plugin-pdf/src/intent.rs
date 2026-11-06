//! The **output intent** of a file.
//!
//! A PDF/A file says what the colours on its pages mean with an
//! **output intent**: a dictionary that names an ICC profile. The
//! profile is the file's own colour space, and it is **built in
//! code** — this tree holds no profile in its assets, and the bytes
//! are made from the sRGB primaries of IEC 61966-2-1, adapted to the
//! D50 profile connection space.
//!
//! The **rules**:
//!
//! - the profile is a **version 2** ICC profile: a header of a
//!   hundred and twenty-eight bytes, a table of tags, and the tags
//!   themselves;
//! - the tags begin on **four-byte boundaries**, and the table says
//!   where each one is and how long it is;
//! - the profile is **uncompressed**, and the file says how many
//!   colour components it has: `N 3`;
//! - the intent says what it is with the name the file uses: `/S` is
//!   `GTS_PDFA1`, and `/Type` is `OutputIntent`.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, Outcome, Session, Text};

/// The **name** the file gives the PDF/A output intent.
const PDFA: &str = "GTS_PDFA1";

/// The **identifier** of the output condition: the sRGB profile.
const SRGB: &str = "sRGB IEC61966-2-1";

/// A number as the ICC format holds it: a signed 16.16 fixed point.
fn s15f16(value: f64) -> [u8; 4] {
    ((value * 65536.0).round() as i32).to_be_bytes()
}

/// A **colour** as the ICC format holds it: three fixed-point numbers.
fn xyz(x: f64, y: f64, z: f64) -> Vec<u8> {
    let mut tag = b"XYZ \0\0\0\0".to_vec();
    tag.extend(s15f16(x));
    tag.extend(s15f16(y));
    tag.extend(s15f16(z));
    tag
}

/// A **description** of the profile: the ICC version 2 form.
fn description(text: &str) -> Vec<u8> {
    let mut tag = b"desc\0\0\0\0".to_vec();
    let ascii: Vec<u8> = text.bytes().chain(std::iter::once(0)).collect();
    tag.extend((ascii.len() as u32).to_be_bytes());
    tag.extend(&ascii);
    // No Unicode description and no ScriptCode description.
    tag.extend([0_u8; 4 + 4]);
    tag.extend([0_u8; 2 + 1 + 67]);
    tag
}

/// A **text** of the profile: the ICC version 2 form.
fn text(text: &str) -> Vec<u8> {
    let mut tag = b"text\0\0\0\0".to_vec();
    tag.extend(text.bytes());
    tag.push(0);
    tag
}

/// The **tone curve** of the profile: the sRGB curve, sampled.
fn curve() -> Vec<u8> {
    const POINTS: usize = 1024;
    let mut tag = b"curv\0\0\0\0".to_vec();
    tag.extend((POINTS as u32).to_be_bytes());
    for point in 0..POINTS {
        let value = point as f64 / (POINTS - 1) as f64;
        let linear = if value <= 0.04045 { value / 12.92 } else { ((value + 0.055) / 1.055).powf(2.4) };
        tag.extend(((linear * 65535.0).round() as u16).to_be_bytes());
    }
    tag
}

/// The **bytes** of the sRGB profile.
///
/// The profile is a display profile of the sRGB primaries, adapted to
/// the D50 profile connection space, with the sRGB tone curve sampled
/// at a thousand and twenty-four points.
pub fn profile() -> Vec<u8> {
    let tags: Vec<(&[u8; 4], Vec<u8>)> = vec![
        (b"desc", description("sRGB (craftkit)")),
        (b"cprt", text("No copyright, use freely")),
        (b"wtpt", xyz(0.9642, 1.0, 0.8249)),
        (b"rXYZ", xyz(0.4361, 0.2225, 0.0139)),
        (b"gXYZ", xyz(0.3851, 0.7169, 0.0971)),
        (b"bXYZ", xyz(0.1431, 0.0606, 0.7141)),
        (b"rTRC", curve()),
    ];
    // The green and the blue share the red's tone curve.
    let count = tags.len() + 2;
    let table_len = 4 + count * 12;
    let mut offset = 128 + table_len;
    let mut table = (count as u32).to_be_bytes().to_vec();
    let mut data: Vec<u8> = Vec::new();
    let mut tone = (0_usize, 0_usize);
    for (signature, body) in &tags {
        let len = body.len();
        table.extend(*signature);
        table.extend((offset as u32).to_be_bytes());
        table.extend((len as u32).to_be_bytes());
        if *signature == b"rTRC" {
            tone = (offset, len);
        }
        data.extend(body);
        // The tags begin on **four-byte boundaries**.
        let pad = (4 - len % 4) % 4;
        data.extend(std::iter::repeat_n(0_u8, pad));
        offset += len + pad;
    }
    for signature in [b"gTRC", b"bTRC"] {
        table.extend(signature);
        table.extend((tone.0 as u32).to_be_bytes());
        table.extend((tone.1 as u32).to_be_bytes());
    }
    let size = 128 + table.len() + data.len();
    let mut header: Vec<u8> = Vec::with_capacity(size);
    header.extend((size as u32).to_be_bytes());
    header.extend(b"\0\0\0\0"); // The preferred CMM: none.
    header.extend([2, 0x10, 0, 0]); // The version: 2.1.
    header.extend(b"mntr");
    header.extend(b"RGB ");
    header.extend(b"XYZ ");
    header.extend([0_u8; 12]); // The date: none.
    header.extend(b"acsp");
    header.extend(b"\0\0\0\0"); // The platform: none.
    header.extend([0_u8; 4]); // The flags: none.
    header.extend([0_u8; 4]); // The manufacturer: none.
    header.extend([0_u8; 4]); // The model: none.
    header.extend([0_u8; 8]); // The attributes: none.
    header.extend([0, 0, 0, 0]); // The rendering intent: perceptual.
    header.extend(s15f16(0.9642));
    header.extend(s15f16(1.0));
    header.extend(s15f16(0.8249));
    header.extend([0_u8; 4]); // The creator: none.
    header.resize(128, 0);
    header.extend(table);
    header.extend(data);
    header
}

/// **Write** the **output intent** of a file.
///
/// The intent names an ICC profile — the sRGB one, built in code — and
/// the catalog names the intent. The profile's bytes go in a file of
/// their own, because that is where the bytes of a stream live here.
///
/// The rules:
///
/// - a path that is **empty** is refused;
/// - a file with **no catalog** is refused: there is nothing to name
///   the intent.
#[derive(Command)]
#[command(id = "pdf.intent.write", area = "pdf", layer = "pdf", icon = "file", mutates, undoable)]
pub struct WriteIntent {
    /// Where the profile's **bytes** are written.
    pub path: Text,
}

impl WriteIntent {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "pdf.intent.write".to_string();
        let path = self.path.0.trim().to_string();
        if path.is_empty() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "the profile's bytes go somewhere".to_string(),
            });
        }
        let Some(catalog) = crate::preflight::the_catalog(session.document()).cloned() else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "the file has no catalog".to_string(),
            });
        };
        // The **profile**: the bytes of the sRGB profile.
        let bytes = profile();
        if let Some(parent) = std::path::Path::new(&path).parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).map_err(|error| EngineError::Failed {
                id: id.clone(),
                message: format!("the profile's bytes have nowhere to go: {error}"),
            })?;
        }
        std::fs::write(&path, &bytes).map_err(|error| EngineError::Failed {
            id: id.clone(),
            message: format!("the profile's bytes cannot be written: {error}"),
        })?;
        // The **profile object**: the stream the profile is in.
        let profile_number = crate::the_next_number(session.document(), 0);
        let data = json!({ "object": profile_number, "entries": json!([["N", 3]]) });
        let profile = session.document_mut()?.add_node_with(
            format!("profile {profile_number}"),
            "pdf.stream",
            None,
            data,
        );
        if let Some(node) = session.document_mut()?.node_mut(profile) {
            node.stream = Some(path.clone());
        }
        // The **intent**: the dictionary that names the profile.
        let intent_number = crate::the_next_number(session.document(), profile_number);
        let entries = json!([
            ["Type", "OutputIntent"],
            ["S", PDFA],
            ["OutputConditionIdentifier", SRGB],
            ["Info", SRGB],
            ["DestOutputProfile", { "ref": [profile_number, 0] }],
        ]);
        let data = json!({ "object": intent_number, "entries": entries });
        let intent = session.document_mut()?.add_node_with(
            format!("output intent {intent_number}"),
            "pdf.object",
            None,
            data,
        );
        // The **catalog** names the intent, and the intents it already
        // had are kept.
        let mut catalog_entries = crate::entries_of(&catalog.data);
        let mut intents = catalog_entries
            .iter()
            .find(|(key, _)| key == "OutputIntents")
            .and_then(|(_, value)| value.as_array().cloned())
            .unwrap_or_default();
        intents.push(json!({ "ref": [intent_number, 0] }));
        match catalog_entries.iter_mut().find(|(key, _)| key == "OutputIntents") {
            Some(entry) => entry.1 = json!(intents),
            None => catalog_entries.push(("OutputIntents".to_string(), json!(intents))),
        }
        let data = json!({
            "object": catalog.data.get("object").cloned().unwrap_or(craftkit_sdk::serde_json::Value::Null),
            "entries": catalog_entries.iter().map(|(key, value)| json!([key, value])).collect::<Vec<_>>(),
        });
        if let Some(node) = session.document_mut()?.node_mut(catalog.id) {
            node.data = data;
        }
        // And the **edges**: the catalog names the intent, and the
        // intent names the profile.
        for (from, to, key) in [(catalog.id, intent, "OutputIntents"), (intent, profile, "DestOutputProfile")]
        {
            let payload = json!({ "key": key, "generation": 0 });
            let relation = craftkit_sdk::Relation::with_payload(from, to, "ref", payload);
            session.document_mut()?.add_relation(relation);
        }
        Ok(Outcome::changed(format!("the file says its colours are {SRGB}")).with_data(json!({
            "intent": intent,
            "object": intent_number,
            "profile": profile,
            "profile_object": profile_number,
            "path": path,
            "bytes": bytes.len(),
        })))
    }
}
