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
//! - The **markers** of the sequence are markers of it: the `in` of a
//!   marker is where it starts, the `out` less the `in` is how long it
//!   is, and its colour is the colour it names — green when it names
//!   none.
//! - A **transition** is a `transitionitem` of a track: it joins the
//!   clip that ends in it and the clip that begins in it, and its
//!   **alignment** is the tree's own — `center`, `start` or `end` —
//!   worked out from where it sits when the file does not say.
//!
//! # What it does not do
//!
//! - The **effects**, the **links** and the **labels** of the
//!   document are not read, and the **bins** of it are not made.
//! - The **markers of a clip** are markers of that clip, and this tree
//!   has no marker of a clip: they are left out, and said. The
//!   markers of a **media** are markers of that media, and this tree
//!   has no marker of a media either.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Outcome, Relation, Session, Text};

use craftkit_sdk::xml::Element;

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
        let root = craftkit_sdk::xml::parse(&text).map_err(|message| EngineError::Failed {
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
        let mut left_out: Vec<String> = Vec::new();
        let Some(media) = sequence.child("media") else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "the file is not FCP7 XML: there is no sequence with media in it".to_string(),
            });
        };
        // The **markers** of the sequence.
        for marker in sequence.children_named("marker") {
            let start = number_of(marker.child("in")).unwrap_or(0);
            let end = number_of(marker.child("out")).unwrap_or(0);
            let duration = end.saturating_sub(start);
            let name = marker.child("name").map(|name| name.text.trim().to_string()).unwrap_or_default();
            let comment =
                marker.child("comment").map(|comment| comment.text.trim().to_string()).unwrap_or_default();
            // The **colour** the marker names, and green when it names
            // none.
            let color = marker
                .child("color")
                .and_then(|color| crate::labels::label_of(color.text.trim()))
                .unwrap_or("Green");
            session.document_mut()?.add_node_with_under(
                Some(sequence_id),
                name.clone(),
                "film.marker",
                None,
                json!({
                    "start": start,
                    "duration": duration,
                    "name": name,
                    "comment": comment,
                    "color": color,
                    "kind": "Comment",
                }),
            );
        }
        // The **markers of a clip** are markers of that clip, and this
        // tree has no marker of a clip: they are left out, and said.
        // The markers of a **media** are markers of that media, and
        // this tree has no marker of a media either.
        for clip_item in root.find_all("clipitem") {
            if clip_item.children_named("marker").next().is_none() {
                continue;
            }
            let name = clip_item.child("name").map(|name| name.text.trim().to_string()).unwrap_or_default();
            left_out.push(format!("the markers of the clip `{name}` are left out"));
        }
        for media in root.find_all("clip") {
            if media.children_named("marker").next().is_none() {
                continue;
            }
            let name = media.child("name").map(|name| name.text.trim().to_string()).unwrap_or_default();
            left_out.push(format!("the markers of the media `{name}` are left out"));
        }
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
                // The **clips** of the track, for the transitions to
                // name.
                let mut track_clips: Vec<(u64, u64, u64)> = Vec::new();
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
                    track_clips.push((clip, start, start + duration));
                }
                // The **transitions** of the track: each one joins the
                // clip that ends in it and the clip that begins in
                // it.
                for transition in track.children_named("transitionitem") {
                    let from_frame = number_of(transition.child("start")).unwrap_or(0);
                    let to_frame = number_of(transition.child("end")).unwrap_or(from_frame);
                    if to_frame <= from_frame {
                        continue;
                    }
                    // The **effect**: the name of the effect, or its
                    // id.
                    let effect_element = transition.child("effect");
                    let name = effect_element
                        .and_then(|effect| effect.child("name"))
                        .or_else(|| effect_element.and_then(|effect| effect.child("effectid")))
                        .map(|name| name.text.trim().to_string())
                        .unwrap_or_else(|| "Cross Dissolve".to_string());
                    let Some(effect) = crate::transitions::effect_of(&name) else {
                        left_out.push(format!(
                            "the transition `{name}` is not a transition this tree has, and it is left out"
                        ));
                        continue;
                    };
                    // The **alignment** the file says, in the tree's
                    // own words.
                    let alignment =
                        transition.attribute("alignment").unwrap_or("").trim().to_ascii_lowercase();
                    // The **clip** the transition comes from: the one
                    // that ends in it, and the one it comes into: the
                    // one that begins in it.
                    let mut from = track_clips
                        .iter()
                        .filter(|(_, _, end)| *end >= from_frame && *end <= to_frame)
                        .max_by_key(|(_, _, end)| *end)
                        .map(|(clip, _, end)| (*clip, *end));
                    let mut to = track_clips
                        .iter()
                        .filter(|(_, start, _)| *start >= from_frame && *start <= to_frame)
                        .min_by_key(|(_, start, _)| *start)
                        .map(|(clip, start, _)| (*clip, *start));
                    if alignment == "start-black" {
                        from = None;
                    }
                    if alignment == "end-black" {
                        to = None;
                    }
                    // A cut needs **both** sides to meet: when they do
                    // not, one of them is not part of the transition.
                    if let (Some((_, from_end)), Some((_, to_start))) = (from, to)
                        && from_end != to_start
                    {
                        if alignment == "start-black" || (alignment.is_empty() && to_start == from_frame) {
                            from = None;
                        } else {
                            to = None;
                        }
                    }
                    if from.is_none() && to.is_none() {
                        left_out.push(format!(
                            "the transition `{name}` has no clip next to it, and it is left out"
                        ));
                        continue;
                    }
                    // The **cut** the transition sits over: the end of
                    // the clip it comes from, or the start of the one
                    // it comes into.
                    let cut =
                        from.map(|(_, end)| end).or_else(|| to.map(|(_, start)| start)).unwrap_or(from_frame);
                    let align = match alignment.as_str() {
                        "center" => "center",
                        "start" | "start-black" => "start",
                        "end" | "end-black" => "end",
                        _ if cut == from_frame => "start",
                        _ if cut == to_frame => "end",
                        _ => "center",
                    };
                    let from_clip = from.map(|(clip, _)| clip);
                    let to_clip = to.map(|(clip, _)| clip);
                    session.document_mut()?.add_node_with_under(
                        Some(track_id),
                        format!("transition at {from_frame}"),
                        "film.transition",
                        None,
                        json!({
                            "from": from_clip,
                            "to": to_clip,
                            "start": from_frame,
                            "duration": to_frame - from_frame,
                            "align": align,
                            "effect": effect,
                        }),
                    );
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
                "left_out": left_out,
                "rate": { "num": rate.0, "den": rate.1 },
            })))
    }
}
