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
//! The **clips** of the range, the **automation** of the range and the
//! **length** of the range. The tree's clipboard holds more: the
//! markers in it, the clip gain and the clip effects. The markers are
//! not here, and the section says why.

use craftkit_derive::commands;

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
    /// The **automation** of the range: the breakpoints of the track's
    /// lanes that were inside it, moved to the range's own head.
    pub(crate) automation: Vec<ClipboardLane>,
}

/// A **lane** in the clipboard.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ClipboardLane {
    /// Which parameter it moves.
    pub(crate) param: String,
    /// Its breakpoints, relative to the range's head.
    pub(crate) points: Vec<(i64, f64)>,
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
    let mut automation = Vec::new();
    for lane in value.get("automation").and_then(|lanes| lanes.as_array())? {
        let param = lane.get("param").and_then(|param| param.as_str()).unwrap_or("").to_string();
        let mut points = Vec::new();
        if let Some(list) = lane.get("points").and_then(|points| points.as_array()) {
            for point in list {
                let at = point.get("at").and_then(|at| at.as_i64());
                let value = point.get("value").and_then(|value| value.as_f64());
                if let (Some(at), Some(value)) = (at, value) {
                    points.push((at, value));
                }
            }
        }
        automation.push(ClipboardLane { param, points });
    }
    Some(Clipboard { length, clips, automation })
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
    let lanes: Vec<Json> = clipboard
        .automation
        .iter()
        .map(|lane| {
            let points: Vec<Json> = lane
                .points
                .iter()
                .map(|(at, value)| serde_json::json!({ "at": at, "value": value }))
                .collect();
            serde_json::json!({ "param": lane.param, "points": points })
        })
        .collect();
    data.insert(
        CLIPBOARD_KEY.to_string(),
        serde_json::json!({ "length": clipboard.length, "clips": clips, "automation": lanes }),
    );
    document.data = Json::Object(data);
}

/// The **lanes** of a range: the automation that was inside it.
fn lanes_of(session: &Session, track: u64, start: i64, end: i64) -> Vec<ClipboardLane> {
    crate::automation_of_range(session, track, start, end)
        .into_iter()
        .map(|(param, points)| ClipboardLane { param, points })
        .collect()
}

/// Take the **automation** of a range away: the breakpoints that fall
/// inside it are thrown away.
fn clear_automation(session: &mut Session, track: u64, start: i64, end: i64) -> Result<(), EngineError> {
    let lanes = crate::automation_of_range(session, track, start, end);
    for (param, _) in lanes {
        let Some(node) = lane_node(session, track, &param) else { continue };
        let Some(node) = session.document_mut()?.node_mut(node) else { continue };
        let Ok(mut payload) = crate::mixer::LanePayload::from_json(&node.data) else { continue };
        payload.points.retain(|(at, _)| *at < start || *at >= end);
        node.data = payload.to_json();
    }
    Ok(())
}

/// The **lane** of a track for a parameter, if it has one.
fn lane_node(session: &Session, track: u64, param: &str) -> Option<u64> {
    let document = session.document();
    for lane in document.walk().filter(|node| node.kind == "sound.lane") {
        if crate::timeline::parent_of(document, lane.id).map(|parent| parent.id) != Some(track) {
            continue;
        }
        let Ok(payload) = crate::mixer::LanePayload::from_json(&lane.data) else { continue };
        if payload.param == param {
            return Some(lane.id);
        }
    }
    None
}

/// Write the **automation** of a range on a track.
///
/// The breakpoints of the range are put at the place the material was
/// put, and the breakpoints that were **there** are taken away: the
/// tree's paste clears the range it covers before it writes, so that
/// a paste is a paste and not a sum of two curves.
fn paste_automation(
    session: &mut Session,
    track: u64,
    at: i64,
    clipboard: &Clipboard,
) -> Result<usize, EngineError> {
    let mut written = 0_usize;
    for lane in &clipboard.automation {
        if lane.points.is_empty() {
            continue;
        }
        let node = match lane_node(session, track, &lane.param) {
            Some(node) => node,
            None => {
                let payload = crate::mixer::LanePayload { param: lane.param.clone(), points: Vec::new() };
                session.document_mut()?.add_node_with_under(
                    Some(track),
                    lane.param.clone(),
                    "sound.lane",
                    None,
                    payload.to_json(),
                )
            }
        };
        let Some(node) = session.document_mut()?.node_mut(node) else { continue };
        let Ok(mut payload) = crate::mixer::LanePayload::from_json(&node.data) else { continue };
        let end = at + clipboard.length;
        payload.points.retain(|(point, _)| *point < at || *point >= end);
        payload.points.extend(lane.points.iter().map(|(point, value)| (at + point, *value)));
        payload.points.sort_by_key(|(at, _)| *at);
        node.data = payload.to_json();
        written += 1;
    }
    Ok(written)
}

commands! {
    /// **Copy** the material of a range of a track.
    ///
    /// The clips of the track that overlap the range are **trimmed** to it
    /// and copied into the clipboard, and the clipboard says how long the
    /// range was.
    ///
    /// A range with **no length** is refused: there is nothing to copy.
    sound.track.copy {
        area: "sound",
        layer: "sound",
        icon: "copy",
        mutates,
        undoable,
        menu: ("Edit", "Copy"),
        params: {
            /// The track whose material is copied.

            track: NodeRef,
            /// Where the range starts, in samples.

            start: Option<Number>,
            /// Where the range ends, in samples.

            end: Option<Number>,
        },
        run: {

            let id = "sound.track.copy".to_string();
            let (start, end) = crate::range_of(session, self.track.0, &id, self.start, self.end)?;
            let copies = crate::copies_of_range(session, self.track.0, start, end);
            let automation = lanes_of(session, self.track.0, start, end);
            // The **material** of a range is its clips **and** its
            // automation: a range with a curve in it and no clip in it has
            // something to copy.
            if copies.is_empty() && automation.is_empty() {
                return Ok(Outcome::new("there is no material in the range to copy")
                    .with_data(serde_json::json!({ "copied": 0, "length": end - start })));
            }
            let clipboard = Clipboard {
                length: end - start,
                clips: copies.into_iter().map(|(name, payload)| ClipboardClip { name, payload }).collect(),
                automation,
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
    sound.track.cut {
        area: "sound",
        layer: "sound",
        icon: "cut",
        mutates,
        undoable,
        menu: ("Edit", "Cut"),
        params: {
            /// The track whose material is cut.

            track: NodeRef,
            /// Where the range starts, in samples.

            start: Option<Number>,
            /// Where the range ends, in samples.

            end: Option<Number>,
        },
        run: {

            let id = "sound.track.cut".to_string();
            let (start, end) = crate::range_of(session, self.track.0, &id, self.start, self.end)?;
            let copies = crate::copies_of_range(session, self.track.0, start, end);
            let automation = lanes_of(session, self.track.0, start, end);
            if copies.is_empty() && automation.is_empty() {
                return Ok(Outcome::new("there is no material in the range to cut")
                    .with_data(serde_json::json!({ "cut": 0, "length": end - start })));
            }
            let clipboard = Clipboard {
                length: end - start,
                clips: copies.into_iter().map(|(name, payload)| ClipboardClip { name, payload }).collect(),
                automation,
            };
            let count = clipboard.clips.len();
            set_clipboard(session.document_mut()?, &clipboard);
            // The **removal**: the clips that are in the range are taken
            // away, and the ones that cross its edge keep the part outside
            // it.
            let removed = crate::clear_range_of(session, self.track.0, start, end);
            // The **automation** of the range goes with the material: a
            // cut takes the curve away as well.
            clear_automation(session, self.track.0, start, end)?;
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
    sound.track.paste {
        area: "sound",
        layer: "sound",
        icon: "paste",
        mutates,
        undoable,
        menu: ("Edit", "Paste"),
        params: {
            /// The track the material is pasted onto.

            track: NodeRef,
            /// Where the material is pasted, in samples.

            at: Option<Number>,
        },
        run: {

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
            if clipboard.clips.is_empty() && clipboard.automation.is_empty() {
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
            // The **automation** of the range is written on the track: the
            // lanes of the clipboard, at the place the material was put.
            let lanes = paste_automation(session, track, at, &clipboard)?;
            Ok(Outcome::changed(format!("pasted {} clip(s) onto the track `{track}`", made.len())).with_data(
                serde_json::json!({ "clips": made, "at": at, "length": clipboard.length, "lanes": lanes }),
            ))

        }
    }
    /// The **clipboard** of the document, read: how many clips it holds and
    /// how long the range was.
    sound.track.clipboard.read {
        area: "sound",
        layer: "sound",
        icon: "list",
        params: {
        },
        run: {

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
            // The **automation** of the range: the lanes that came with it.
            let automation: Vec<Json> = clipboard
                .automation
                .iter()
                .map(|lane| {
                    let points: Vec<Json> =
                        lane.points.iter().map(|(at, value)| serde_json::json!([at, value])).collect();
                    serde_json::json!({ "param": lane.param, "points": points })
                })
                .collect();
            Ok(Outcome::new(format!("the clipboard holds {} clip(s)", clips.len())).with_data(
                serde_json::json!({
                    "empty": false,
                    "length": clipboard.length,
                    "clips": clips,
                    "lanes": automation.len(),
                    "automation": automation,
                }),
            ))

        }
    }
}
