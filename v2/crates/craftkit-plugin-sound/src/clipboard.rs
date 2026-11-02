//! The **clipboard**: the material that was copied, waiting to be
//! pasted.
//!
//! The tree's editor has a clipboard: it copies the material of a
//! range, and pastes it somewhere else. This tree's battery had no
//! clipboard at all — there was no copy and no paste — and the copy
//! and the paste are here now.
//!
//! # Where the clipboard lives
//!
//! The clipboard is in the **document's payload**, under a key of its
//! own: the document is the only place this tree keeps a thing, and a
//! key of its own means that two plugins cannot collide by accident.
//!
//! It is **not** a node: a clipboard is not a thing in the session, it
//! is a thing that was copied out of it. The clips it holds are
//! **payloads** — the clip's own words and the source it plays — and
//! the paste makes new clips of them.
//!
//! # What the clipboard holds
//!
//! The **clips** of the range and the **length** of the range. The
//! tree's clipboard holds more: the automation of the range, the
//! markers in it, the clip gain and the clip effects. The automation
//! and the markers are not here, and the section says why.

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::serde_json::Value as Json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session};

use crate::ClipPayload;

/// The key the clipboard is kept under.
pub(crate) const CLIPBOARD_KEY: &str = "sound.clipboard";

/// What the **clipboard** holds.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Clipboard {
    /// How long the copied range was.
    pub(crate) length: i64,
    /// The clips of the range: each one is a payload and the source
    /// it plays.
    pub(crate) clips: Vec<ClipboardClip>,
}

/// A **clip** in the clipboard.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ClipboardClip {
    /// The name the clip had.
    pub(crate) name: String,
    /// The clip's own words.
    pub(crate) payload: ClipPayload,
}

/// The **clipboard** of the document, if it has one.
pub(crate) fn clipboard_of(document: &craftkit_sdk::Document) -> Option<Clipboard> {
    let value = document.data.get(CLIPBOARD_KEY)?;
    let length = value.get("length").and_then(|length| length.as_i64())?;
    let mut clips = Vec::new();
    for clip in value.get("clips").and_then(|clips| clips.as_array())? {
        let name = clip.get("name").and_then(|name| name.as_str()).unwrap_or("clip").to_string();
        let payload = ClipPayload::from_json(clip.get("payload")?).ok()?;
        clips.push(ClipboardClip { name, payload });
    }
    Some(Clipboard { length, clips })
}

/// Put the **clipboard** into the document.
pub(crate) fn set_clipboard(document: &mut craftkit_sdk::Document, clipboard: &Clipboard) {
    let clips: Vec<Json> = clipboard
        .clips
        .iter()
        .map(|clip| {
            serde_json::json!({
                "name": clip.name,
                "payload": clip.payload.to_json(),
            })
        })
        .collect();
    let mut data = match document.data.as_object() {
        Some(object) => object.clone(),
        None => serde_json::Map::new(),
    };
    data.insert(CLIPBOARD_KEY.to_string(), serde_json::json!({ "length": clipboard.length, "clips": clips }));
    document.data = Json::Object(data);
}

/// **Copy** the material of a range of a track.
///
/// The clips of the track that overlap the range are **trimmed** to it
/// and copied into the clipboard, and the clipboard says how long the
/// range was.
///
/// A range with **no length** is refused: there is nothing to copy.
#[derive(Command)]
#[command(
    id = "sound.track.copy",
    area = "sound",
    layer = "sound",
    icon = "copy",
    menu("Edit", "Copy"),
    mutates,
    undoable
)]
pub struct CopyRange {
    /// The track whose material is copied.
    pub track: NodeRef,
    /// Where the range starts, in samples.
    pub start: Option<Number>,
    /// Where the range ends, in samples.
    pub end: Option<Number>,
}

impl CopyRange {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.track.copy".to_string();
        let (start, end) = crate::range_of(session, self.track.0, &id, self.start, self.end)?;
        let copies = crate::copies_of_range(session, self.track.0, start, end);
        if copies.is_empty() {
            return Ok(Outcome::new("there is no material in the range to copy")
                .with_data(serde_json::json!({ "copied": 0, "length": end - start })));
        }
        let clipboard = Clipboard {
            length: end - start,
            clips: copies.into_iter().map(|(name, payload)| ClipboardClip { name, payload }).collect(),
        };
        let count = clipboard.clips.len();
        set_clipboard(session.document_mut()?, &clipboard);
        Ok(Outcome::changed(format!("copied {count} clip(s) into the clipboard"))
            .with_data(serde_json::json!({ "copied": count, "length": clipboard.length })))
    }
}

/// **Cut** the material of a range of a track.
///
/// The material is **copied** into the clipboard and then **taken
/// away**: the cut is a copy and a removal, and the removal is the
/// part that changes the document.
#[derive(Command)]
#[command(
    id = "sound.track.cut",
    area = "sound",
    layer = "sound",
    icon = "cut",
    menu("Edit", "Cut"),
    mutates,
    undoable
)]
pub struct CutRange {
    /// The track whose material is cut.
    pub track: NodeRef,
    /// Where the range starts, in samples.
    pub start: Option<Number>,
    /// Where the range ends, in samples.
    pub end: Option<Number>,
}

impl CutRange {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.track.cut".to_string();
        let (start, end) = crate::range_of(session, self.track.0, &id, self.start, self.end)?;
        let copies = crate::copies_of_range(session, self.track.0, start, end);
        if copies.is_empty() {
            return Ok(Outcome::new("there is no material in the range to cut")
                .with_data(serde_json::json!({ "cut": 0, "length": end - start })));
        }
        let clipboard = Clipboard {
            length: end - start,
            clips: copies.into_iter().map(|(name, payload)| ClipboardClip { name, payload }).collect(),
        };
        let count = clipboard.clips.len();
        set_clipboard(session.document_mut()?, &clipboard);
        // The **removal**: the clips that are in the range are taken
        // away, and the ones that cross its edge keep the part outside
        // it.
        let removed = crate::clear_range_of(session, self.track.0, start, end);
        Ok(Outcome::changed(format!("cut {count} clip(s) into the clipboard"))
            .with_data(serde_json::json!({ "cut": count, "removed": removed, "length": clipboard.length })))
    }
}

/// **Paste** the clipboard onto a track.
///
/// The material of the clipboard is laid on the track at the place the
/// caller gives: each clip of the clipboard becomes a **new** clip,
/// with the source it plays and everything it had.
///
/// A clipboard that is **empty** is refused: there is nothing to paste.
#[derive(Command)]
#[command(
    id = "sound.track.paste",
    area = "sound",
    layer = "sound",
    icon = "paste",
    menu("Edit", "Paste"),
    mutates,
    undoable
)]
pub struct PasteRange {
    /// The track the material is pasted onto.
    pub track: NodeRef,
    /// Where the material is pasted, in samples.
    pub at: Option<Number>,
}

impl PasteRange {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.track.paste".to_string();
        let track = self.track.0;
        let node = session.document().node(track).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("there is no node `{track}`"),
        })?;
        if node.kind != "sound.track" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{track}` is a `{}` and not a track", node.kind),
            });
        }
        let Some(clipboard) = clipboard_of(session.document()) else {
            return Err(EngineError::Failed {
                id,
                message: "the clipboard is empty: there is nothing to paste".to_string(),
            });
        };
        if clipboard.clips.is_empty() {
            return Err(EngineError::Failed {
                id,
                message: "the clipboard is empty: there is nothing to paste".to_string(),
            });
        }
        let at = match &self.at {
            Some(at) => crate::samples(at.0, "where the material is pasted")?,
            None => 0,
        };
        let playlist = crate::playlist_of(session, track, None)?;
        // The **clips**: each one of the clipboard is a new clip, with
        // its own name and its own place.
        let mut made = Vec::with_capacity(clipboard.clips.len());
        for clip in &clipboard.clips {
            let mut payload = clip.payload.clone();
            payload.start += at;
            let placed = session.document_mut()?.add_node_with_under(
                Some(playlist),
                clip.name.clone(),
                "sound.clip",
                None,
                payload.to_json(),
            );
            // The **source** relation goes with the clip: a clip that
            // plays nothing is a clip of nothing.
            if let Some(source) = payload.source {
                session.document_mut()?.add_relation(craftkit_sdk::Relation::new(placed, source, "source"));
            }
            made.push(placed);
        }
        Ok(Outcome::changed(format!("pasted {} clip(s) onto the track `{track}`", made.len()))
            .with_data(serde_json::json!({ "clips": made, "at": at, "length": clipboard.length })))
    }
}

/// The **clipboard** of the document, read: how many clips it holds and
/// how long the range was.
#[derive(Command)]
#[command(id = "sound.track.clipboard.read", area = "sound", layer = "sound", icon = "list")]
pub struct ReadClipboard {}

impl ReadClipboard {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let Some(clipboard) = clipboard_of(session.document()) else {
            return Ok(Outcome::new("the clipboard is empty")
                .with_data(serde_json::json!({ "empty": true, "length": 0, "clips": Vec::<u64>::new() })));
        };
        let clips: Vec<Json> = clipboard
            .clips
            .iter()
            .map(|clip| {
                serde_json::json!({
                    "name": clip.name,
                    "start": clip.payload.start,
                    "length": clip.payload.length,
                })
            })
            .collect();
        Ok(Outcome::new(format!("the clipboard holds {} clip(s)", clips.len()))
            .with_data(serde_json::json!({ "empty": false, "length": clipboard.length, "clips": clips })))
    }
}
