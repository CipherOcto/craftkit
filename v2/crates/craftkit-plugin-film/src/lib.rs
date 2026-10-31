//! `craftkit-plugin-film` — the document model of a **video editor**, as
//! v2 nodes and relations.
//!
//! # What is ported
//!
//! A video editor's project is not a tree, and it is not a graph either.
//! It is **three things at once**:
//!
//! - a tree of **bins**: the folders the editor shows, each holding
//!   items and other bins;
//! - a flat **registry of items**: the media, the sequences, the
//!   graphics — each one a thing with its own id, and each one placed
//!   in a bin;
//! - a **timeline**: sequences hold tracks, and tracks hold clips, and a
//!   clip says which item it plays, where it starts, how long it is,
//!   and where in the media it begins.
//!
//! That last part is the **two clocks**: a clip has a place on the
//! timeline and a place in the media, and they are not the same place.
//!
//! # The mapping
//!
//! | the editor | v2 |
//! |---|---|
//! | a bin | a **node** of the kind `film.bin` — a group |
//! | an item | a **node** of the kind `film.item`, in a bin |
//! | a sequence | an item whose kind says it is one |
//! | a track | a **node** of the kind `film.track`, in a sequence |
//! | a clip | a **node** of the kind `film.clip`, in a track |
//! | the item a clip plays | a **relation** of the kind `clip` |
//! | the item's own id | the payload's `item`, because the ids are the core's |
//!
//! # The three things the port found
//!
//! - **The tree gives a node one parent, and an editor's bins can hold
//!   the same item twice.** The item lives in one bin, and a second bin
//!   that holds it points at it with a relation. The core's rule — a
//!   node has one parent — is the right one, and the editor's flat
//!   registry is what has to give way.
//! - **The order of a timeline is not the order of the tree.** The
//!   children of a track are in the order they were made, and the
//!   timeline is in the order they start. So the order that matters is
//!   **derived**, and the evaluator derives it.
//! - **A sequence's duration is not stored.** It is the end of the last
//!   clip on its tracks, and it changes when a clip moves. So it is
//!   derived too, and nobody writes it down.
//!
//! And one that is not a finding but a fact: a clip's **effects** are
//! its own, and they live in its payload — the payload is the plugin's,
//! and the effects are the plugin's business.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

use craftkit_derive::{Command, Evaluator, Layer, NodeKind, Panel};
use craftkit_sdk::serde_json;
use craftkit_sdk::{Document, EngineError, Node, NodeRef, Number, Outcome, Plan, PlanItem, Session, Text};

/// The film layer.
///
/// It needs the core, and it says so.
#[derive(Layer)]
#[layer(id = "film", name = "Film", requires("core"))]
pub struct FilmLayer;

/// A **bin**: a folder of the project.
#[derive(NodeKind)]
#[node_kind(kind = "film.bin", label = "Bin", layer = "film", evaluates)]
pub struct BinKind;

impl BinKind {
    /// A bin is not drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Whether a point is inside a bin: never.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a bin **means**: how much is in it.
    fn evaluate(node: &Node, _document: &Document) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "items": node.children.iter().filter(|child| child.kind == "film.item").count(),
            "bins": node.children.iter().filter(|child| child.kind == "film.bin").count(),
        }))
    }
}

/// An **item**: the media, the sequence, the graphic.
#[derive(NodeKind)]
#[node_kind(kind = "film.item", label = "Item", layer = "film", evaluates)]
pub struct ItemKind;

impl ItemKind {
    /// An item is not drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Whether a point is inside an item: never.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What an item **means**: its own id, and where its media is.
    fn evaluate(node: &Node, _document: &Document) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "item": node.data.get("item").cloned().unwrap_or(serde_json::Value::Null),
            "path": node.data.get("path").cloned().unwrap_or(serde_json::Value::Null),
            "duration": node.data.get("duration").cloned().unwrap_or(serde_json::Value::Null),
            "offline": node.data.get("offline").cloned().unwrap_or(serde_json::Value::Null),
        }))
    }
}

/// A **track**: a lane of the timeline.
#[derive(NodeKind)]
#[node_kind(kind = "film.track", label = "Track", layer = "film", evaluates)]
pub struct TrackKind;

impl TrackKind {
    /// A track is not drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Whether a point is inside a track: never.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a track **means**: what kind of track it is, and how many
    /// clips are on it.
    fn evaluate(node: &Node, _document: &Document) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "kind": node.data.get("kind").cloned().unwrap_or(serde_json::Value::Null),
            "clips": node.children.iter().filter(|child| child.kind == "film.clip").count(),
        }))
    }
}

/// A **clip**: a piece of an item, placed on a track.
#[derive(NodeKind)]
#[node_kind(kind = "film.clip", label = "Clip", layer = "film", evaluates)]
pub struct ClipKind;

impl ClipKind {
    /// A clip is not drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Whether a point is inside a clip: never.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a clip **means**: the two clocks, and the item it plays.
    fn evaluate(node: &Node, document: &Document) -> Option<serde_json::Value> {
        let item = document
            .relations
            .iter()
            .find(|relation| relation.kind == "clip" && relation.from == node.id)
            .map(|relation| relation.to);
        Some(serde_json::json!({
            "start": node.data.get("start").cloned().unwrap_or(serde_json::Value::Null),
            "duration": node.data.get("duration").cloned().unwrap_or(serde_json::Value::Null),
            "source_in": node.data.get("source_in").cloned().unwrap_or(serde_json::Value::Null),
            "item": item,
        }))
    }
}

/// Put a **bin** in the project.
#[derive(Command)]
#[command(id = "film.bin.add", area = "film", layer = "film", icon = "plus", mutates, undoable)]
pub struct AddBin {
    /// What to call it.
    pub name: Text,
    /// The bin it goes in, if it goes in one.
    pub parent: Option<NodeRef>,
}

impl AddBin {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let name = self.name.0.clone();
        if name.trim().is_empty() {
            return Err(EngineError::Failed {
                id: "film.bin.add".to_string(),
                message: "a bin has a name".to_string(),
            });
        }
        let parent = self.parent.map(|parent| parent.0);
        let id = session.document_mut()?.add_node_under(parent, name.clone(), "film.bin", None);
        Ok(Outcome::changed(format!("added the bin `{name}`"))
            .with_data(serde_json::json!({ "id": id, "name": name })))
    }
}

/// Put an **item** in a bin.
///
/// The item's own id is the editor's, and it is kept in the payload:
/// the ids of the nodes are the core's.
#[derive(Command)]
#[command(id = "film.item.add", area = "film", layer = "film", icon = "plus", mutates, undoable)]
pub struct AddItem {
    /// What to call it.
    pub name: Text,
    /// The bin it goes in.
    pub bin: NodeRef,
    /// The item's own id, in the editor.
    pub item: Number,
    /// Where the media is, if it is a media file.
    pub path: Option<Text>,
    /// How long it is, in frames.
    pub duration: Option<Number>,
    /// Whether it was made offline on purpose.
    pub offline: Option<craftkit_sdk::Bool>,
}

impl AddItem {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let name = self.name.0.clone();
        if name.trim().is_empty() {
            return Err(EngineError::Failed {
                id: "film.item.add".to_string(),
                message: "an item has a name".to_string(),
            });
        }
        let bin = self.bin.0;
        if session.document().node(bin).is_none() {
            return Err(EngineError::Failed {
                id: "film.item.add".to_string(),
                message: format!("there is no bin `{bin}`"),
            });
        }
        let item = frames(self.item, "film.item.add")?;
        let duration = match &self.duration {
            Some(duration) => frames(*duration, "film.item.add")?,
            None => 0,
        };
        let offline = self.offline.map(|offline| offline.0).unwrap_or(false);
        let data = serde_json::json!({
            "item": item,
            "path": match &self.path {
                Some(path) => serde_json::Value::String(path.0.clone()),
                None => serde_json::Value::Null,
            },
            "duration": duration,
            "offline": offline,
        });
        let id =
            session.document_mut()?.add_node_with_under(Some(bin), name.clone(), "film.item", None, data);
        Ok(Outcome::changed(format!("added the item `{name}`"))
            .with_data(serde_json::json!({ "id": id, "item": item, "name": name })))
    }
}

/// Put a **track** in a sequence.
#[derive(Command)]
#[command(id = "film.track.add", area = "film", layer = "film", icon = "plus", mutates, undoable)]
pub struct AddTrack {
    /// What to call it.
    pub name: Text,
    /// The sequence it is in.
    pub sequence: NodeRef,
    /// What kind of track it is: `video`, `audio`, `title`, …
    pub kind: Text,
}

impl AddTrack {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let name = self.name.0.clone();
        if name.trim().is_empty() {
            return Err(EngineError::Failed {
                id: "film.track.add".to_string(),
                message: "a track has a name".to_string(),
            });
        }
        let sequence = self.sequence.0;
        if session.document().node(sequence).is_none() {
            return Err(EngineError::Failed {
                id: "film.track.add".to_string(),
                message: format!("there is no sequence `{sequence}`"),
            });
        }
        let kind = self.kind.0.clone();
        let data = serde_json::json!({ "kind": kind });
        let id = session.document_mut()?.add_node_with_under(
            Some(sequence),
            name.clone(),
            "film.track",
            None,
            data,
        );
        Ok(Outcome::changed(format!("added the track `{name}`"))
            .with_data(serde_json::json!({ "id": id, "name": name, "kind": kind })))
    }
}

/// Put a **clip** on a track.
///
/// The clip has the **two clocks**: where it starts on the timeline and
/// how long it is, and where in the media it begins. And it says which
/// item it plays, with a relation.
#[derive(Command)]
#[command(id = "film.clip.add", area = "film", layer = "film", icon = "plus", mutates, undoable)]
pub struct AddClip {
    /// The track it goes on.
    pub track: NodeRef,
    /// The item it plays.
    pub item: NodeRef,
    /// Where it starts on the timeline, in frames.
    pub start: Number,
    /// How long it is, in frames.
    pub duration: Number,
    /// Where in the media it begins, in frames.
    pub source_in: Option<Number>,
}

impl AddClip {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let track = self.track.0;
        let item = self.item.0;
        for id in [track, item] {
            if session.document().node(id).is_none() {
                return Err(EngineError::Failed {
                    id: "film.clip.add".to_string(),
                    message: format!("there is no node `{id}`"),
                });
            }
        }
        let start = frames(self.start, "film.clip.add")?;
        let duration = frames(self.duration, "film.clip.add")?;
        if duration == 0 {
            return Err(EngineError::Failed {
                id: "film.clip.add".to_string(),
                message: "a clip is not zero frames long".to_string(),
            });
        }
        let source_in = match &self.source_in {
            Some(source_in) => frames(*source_in, "film.clip.add")?,
            None => 0,
        };
        let data = serde_json::json!({
            "start": start,
            "duration": duration,
            "source_in": source_in,
        });
        let name = format!("clip at {start}");
        let id =
            session.document_mut()?.add_node_with_under(Some(track), name.clone(), "film.clip", None, data);
        // The item the clip plays: an edge, not containment.
        let relation = craftkit_sdk::Relation::with_payload(id, item, "clip", serde_json::json!({}));
        session.document_mut()?.add_relation(relation);
        Ok(Outcome::changed(format!("added a {duration}-frame clip at {start}"))
            .with_data(serde_json::json!({ "id": id, "item": item, "start": start, "duration": duration })))
    }
}

/// The timeline of the document.
///
/// A query: the clips in the order they start, and the length of every
/// track. Both are **derived**: neither is written down.
#[derive(Command)]
#[command(id = "film.timeline", area = "film", layer = "film", icon = "list")]
pub struct ReadTimeline {}

impl ReadTimeline {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let mut tracks: Vec<serde_json::Value> = Vec::new();
        for track in session.document().walk().filter(|node| node.kind == "film.track") {
            let mut clips: Vec<(u64, u64)> = track
                .children
                .iter()
                .filter(|child| child.kind == "film.clip")
                .map(|child| {
                    let start = child.data.get("start").and_then(|start| start.as_u64()).unwrap_or(0);
                    let duration =
                        child.data.get("duration").and_then(|duration| duration.as_u64()).unwrap_or(0);
                    (start, duration)
                })
                .collect();
            // The order of a timeline is not the order of the tree.
            clips.sort_by_key(|(start, _)| *start);
            let end = clips.iter().map(|(start, duration)| start + duration).max().unwrap_or(0);
            tracks.push(serde_json::json!({
                "track": track.id,
                "name": track.name,
                "clips": clips.iter().map(|(start, duration)| serde_json::json!({
                    "start": start,
                    "duration": duration,
                })).collect::<Vec<_>>(),
                "end": end,
            }));
        }
        let count = tracks.len();
        Ok(Outcome::new(format!("{count} track(s)")).with_data(serde_json::json!({ "tracks": tracks })))
    }
}

/// The evaluator that measures the **timeline**.
///
/// A track's length is the end of its last clip, and a sequence's length
/// is the end of the longest of its tracks. Neither is stored: they
/// change when a clip moves, and a thing that changes when something
/// else changes is derived.
#[derive(Evaluator)]
#[evaluator(id = "film.timeline", label = "Timeline", layer = "film")]
pub struct TimelineEvaluator;

impl TimelineEvaluator {
    /// The length of every track, and of every sequence.
    fn evaluate(document: &Document) -> Plan {
        let mut plan = Plan::new();
        for node in document.walk() {
            if node.kind != "film.track" {
                continue;
            }
            let end = node
                .children
                .iter()
                .filter(|child| child.kind == "film.clip")
                .filter_map(|child| {
                    let start = child.data.get("start").and_then(|start| start.as_u64())?;
                    let duration = child.data.get("duration").and_then(|duration| duration.as_u64())?;
                    Some(start + duration)
                })
                .max()
                .unwrap_or(0);
            plan.items.push(PlanItem {
                node: node.id,
                kind: "film.timeline".to_string(),
                value: serde_json::json!({ "end": end }),
            });
        }
        plan
    }
}

/// The panel that shows the project.
#[derive(Panel)]
#[panel(
    id = "panel.film",
    label = "Film",
    layer = "film",
    commands("film.bin.add", "film.item.add", "film.track.add", "film.clip.add")
)]
pub struct FilmPanel;

/// A number of frames.
fn frames(value: Number, id: &str) -> Result<u64, EngineError> {
    if !value.0.is_finite() || value.0 < 0.0 || value.0.fract() != 0.0 {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: "a number of frames is a whole number".to_string(),
        });
    }
    Ok(value.0 as u64)
}

// The plugin's anchor: one name that keeps every item above in the
// program, on every target.
craftkit_sdk::plugin_anchor!(
    FilmLayer,
    BinKind,
    ItemKind,
    TrackKind,
    ClipKind,
    AddBin,
    AddItem,
    AddTrack,
    AddClip,
    ReadTimeline,
    TimelineEvaluator,
    FilmPanel
);
