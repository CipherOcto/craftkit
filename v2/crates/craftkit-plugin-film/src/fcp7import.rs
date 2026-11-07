//! The **FCP7** read back: an `xmeml` document made into a sequence.
//!
//! FCP7 XML is the dialect of a final cut of the seven: a document of
//! **sequences**, **tracks** and **clip items**, with the media of a
//! clip item in a **file**. This module reads one and makes the
//! sequence it names.
//!
//! # The rules
//!
//! - The file is read through a **bound**: at most **sixteen
//!   mebibytes**.
//! - A file that is **not FCP7 XML** is refused: its root is not
//!   `xmeml`, or it holds no **sequence** with media — the tree's own
//!   refusals.
//! - The **rate** of the sequence is the rate of the document: the
//!   `timebase` of its rate, and a rate that is **NTSC** is the
//!   tree's own rational one — `30000/1001` and the rest.
//! - A **video** track is a video track and an **audio** track is an
//!   audio one, and a clip item of one is a clip of that track.
//! - The **media** of a clip item is an **item** of a bin, with the
//!   name and the file of the `file` element, and the **clip** points
//!   at it — the same shape as a clip made by hand.
//! - The **start** of a clip is the `start` of its clip item, its
//!   **length** is the `out` less the `in`, and the **source** is the
//!   `in`.
//!
//! # What it does not do
//!
//! - The **markers**, the **effects**, the **links**, the **labels**
//!   and the **transitions** of the document are not read, and the
//!   **bins** of it are not made.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Outcome, Relation, Session, Text};

use crate::xml::Element;

/// The most an FCP7 file may be, in bytes.
const MAX_BYTES: u64 = 16 * 1024 * 1024;

/// The rate a sequence with **no rate** runs at: the tree's own.
const DEFAULT_RATE: (i64, i64) = (25, 1);

/// The **rate** a `rate` element says.
///
/// The `timebase` is the frames of a second, and a rate that is
/// **NTSC** is the tree's own rational one: `24000/1001` and the
/// rest.
fn rate_of(element: Option<&Element>) -> (i64, i64) {
    let Some(element) = element else { return DEFAULT_RATE };
    let Some(timebase) =
        element.child("timebase").and_then(|timebase| timebase.text.trim().parse::<i64>().ok())
    else {
        return DEFAULT_RATE;
    };
    let ntsc =
        element.child("ntsc").map(|ntsc| ntsc.text.trim().eq_ignore_ascii_case("TRUE")).unwrap_or(false);
    if ntsc {
        return match timebase {
            24 => (24000, 1001),
            30 => (30000, 1001),
            60 => (60000, 1001),
            120 => (120000, 1001),
            _ => (timebase, 1),
        };
    }
    if timebase > 0 { (timebase, 1) } else { DEFAULT_RATE }
}

/// The **number** an element holds, when it holds one.
fn number_of(element: Option<&Element>) -> Option<u64> {
    element.and_then(|element| element.text.trim().parse::<u64>().ok())
}

/// The **files** of the document, by their id.
fn files_of(root: &Element) -> Vec<(String, String, String, u64)> {
    let mut files = Vec::new();
    let mut stack: Vec<&Element> = root.children.iter().rev().collect();
    while let Some(element) = stack.pop() {
        if element.name == "file"
            && let Some(id) = element.attribute("id")
        {
            let path = element.child("pathurl").map(|path| path.text.trim().to_string()).unwrap_or_default();
            let path = path.strip_prefix("file://").unwrap_or(&path).to_string();
            let name = element
                .child("name")
                .map(|name| name.text.trim().to_string())
                .unwrap_or_else(|| "the media".to_string());
            let duration = number_of(element.child("duration")).unwrap_or(0);
            files.push((id.to_string(), name, path, duration));
        }
        for child in element.children.iter().rev() {
            stack.push(child);
        }
    }
    files
}

/// The **sequence** of the document: the first one that holds media.
fn sequence_of(root: &Element) -> Option<&Element> {
    let mut stack: Vec<&Element> = root.children.iter().rev().collect();
    while let Some(element) = stack.pop() {
        if element.name == "sequence" && element.child("media").is_some() {
            return Some(element);
        }
        for child in element.children.iter().rev() {
            stack.push(child);
        }
    }
    None
}

/// **Read** an FCP7 document and make the **sequence** it names.
///
/// The media of the document are items, and the clip items of its
/// tracks are clips on the tracks of the sequence.
#[derive(Command)]
#[command(id = "film.import.fcp7", area = "film", layer = "film", icon = "plus", mutates, undoable)]
pub struct ImportFcp7 {
    /// The file to read.
    pub path: Text,
    /// What to call the sequence.
    ///
    /// Without it, the name of the sequence in the document.
    pub name: Option<Text>,
}

impl ImportFcp7 {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.import.fcp7".to_string();
        let path = self.path.0.trim().to_string();
        if path.is_empty() {
            return Err(EngineError::Failed { id: id.clone(), message: "a file has a path".to_string() });
        }
        let bytes =
            craftkit_sdk::files::read_bounded(std::path::Path::new(&path), MAX_BYTES).map_err(|message| {
                EngineError::Failed { id: id.clone(), message: format!("the file cannot be read: {message}") }
            })?;
        let text = String::from_utf8(bytes).map_err(|_| EngineError::Failed {
            id: id.clone(),
            message: "the file is not a text".to_string(),
        })?;
        let root = crate::xml::parse(&text).map_err(|message| EngineError::Failed {
            id: id.clone(),
            message: format!("the file is not FCP7 XML: {message}"),
        })?;
        if root.name != "xmeml" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the file is not FCP7 XML: its root is `{}` and not `xmeml`", root.name),
            });
        }
        let Some(sequence) = sequence_of(&root) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "the file is not FCP7 XML: there is no sequence with media in it".to_string(),
            });
        };
        let files = files_of(&root);
        let rate = rate_of(sequence.child("rate"));
        // The **name** of the sequence.
        let name = match &self.name {
            Some(name) if !name.0.trim().is_empty() => name.0.trim().to_string(),
            _ => sequence
                .child("name")
                .map(|name| name.text.trim().to_string())
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| {
                    std::path::Path::new(&path)
                        .file_stem()
                        .map(|stem| stem.to_string_lossy().to_string())
                        .unwrap_or_else(|| "the sequence".to_string())
                }),
        };
        // The **bin** the media goes in, and the **bin** the sequence
        // goes in: a sequence is not a media.
        let bin = session.document_mut()?.add_node_under(None, format!("{name} media"), "film.bin", None);
        let sequences =
            session.document_mut()?.add_node_under(None, format!("{name} sequences"), "film.bin", None);
        let sequence_id = session.document_mut()?.add_node_with_under(
            Some(sequences),
            name.clone(),
            "film.item",
            None,
            json!({
                "item": 0,
                "path": Json::Null,
                "duration": 0,
                "offline": false,
                "rate": { "num": rate.0, "den": rate.1 },
            }),
        );
        let mut end = 0u64;
        let mut tracks = 0usize;
        let mut clips = 0usize;
        let mut number = 0u64;
        let Some(media) = sequence.child("media") else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "the file is not FCP7 XML: there is no sequence with media in it".to_string(),
            });
        };
        for (kind, name) in [("video", "the video"), ("audio", "the audio")] {
            let Some(section) = media.child(kind) else { continue };
            for track in section.children_named("track") {
                let track_id = session.document_mut()?.add_node_with_under(
                    Some(sequence_id),
                    name.to_string(),
                    "film.track",
                    None,
                    json!({ "kind": kind }),
                );
                tracks += 1;
                for clip_item in track.children_named("clipitem") {
                    let source_in = number_of(clip_item.child("in")).unwrap_or(0);
                    let source_out = number_of(clip_item.child("out")).unwrap_or(0);
                    let duration = source_out.saturating_sub(source_in);
                    let start = number_of(clip_item.child("start")).unwrap_or(0);
                    if duration == 0 {
                        continue;
                    }
                    // The **media** of the clip item: the file it
                    // names.
                    let reference = clip_item.child("file").and_then(|file| file.attribute("id"));
                    let media_item = files.iter().find(|(id, _, _, _)| Some(id.as_str()) == reference);
                    number += 1;
                    let item = session.document_mut()?.add_node_with_under(
                        Some(bin),
                        media_item.map(|(_, name, _, _)| name.clone()).unwrap_or_else(|| {
                            clip_item
                                .child("name")
                                .map(|name| name.text.trim().to_string())
                                .unwrap_or_default()
                        }),
                        "film.item",
                        None,
                        json!({
                            "item": number,
                            "path": match media_item {
                                Some((_, _, path, _)) if !path.is_empty() => Json::String(path.clone()),
                                _ => Json::Null,
                            },
                            "duration": media_item.map(|(_, _, _, duration)| *duration).unwrap_or(0),
                            "offline": media_item.is_none(),
                        }),
                    );
                    let clip_name = clip_item
                        .child("name")
                        .map(|name| name.text.trim().to_string())
                        .filter(|name| !name.is_empty())
                        .unwrap_or_else(|| "a clip".to_string());
                    let clip = session.document_mut()?.add_node_with_under(
                        Some(track_id),
                        clip_name,
                        "film.clip",
                        None,
                        json!({
                            "start": start,
                            "duration": duration,
                            "source_in": source_in,
                            "speed": 1.0,
                            "link": Json::Null,
                        }),
                    );
                    let relation = Relation::with_payload(clip, item, "clip", json!({}));
                    session.document_mut()?.add_relation(relation);
                    clips += 1;
                    end = end.max(start + duration);
                }
            }
        }
        // The **length** of the sequence is what its tracks hold.
        if let Some(node) = session.document_mut().ok().and_then(|document| document.node_mut(sequence_id)) {
            node.data["duration"] = Json::from(end);
        }
        Ok(Outcome::changed(format!("the document of `{path}` made {tracks} track(s) and {clips} clip(s)"))
            .with_data(json!({
                "path": path,
                "bin": bin,
                "sequence_bin": sequences,
                "sequence": sequence_id,
                "tracks": tracks,
                "clips": clips,
                "rate": { "num": rate.0, "den": rate.1 },
            })))
    }
}
