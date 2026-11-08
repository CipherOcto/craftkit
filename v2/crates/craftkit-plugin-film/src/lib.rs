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
//! And a sequence has a **playhead**: the frame it is looked at, and
//! the playhead is a fact of the sequence and not of the window that
//! happens to be open (the module `playhead`).
//!
//! And a sequence has **edit points** — the places where its clips
//! begin and end — and the playhead walks them, and the **razor** cuts
//! the clip that is under the playhead (the module `edits`).
//!
//! And a track has **settings**: it may be enabled, muted or soloed,
//! it may have a volume and a pan, and it may carry a label — and the
//! pan outside its bounds is refused and not clamped (the module
//! `tracks`).
//!
//! And a **media item** may carry a **label** as well: the tree's
//! `edit.label` labels the items of the project and the clips of the
//! sequence with one command and one list of colours, and here the
//! label of a thing is a thing of its own and the tree's list is
//! shared by both (the module `labels`).
//!
//! And a clip may carry a **label**: one of the tree's own sixteen
//! colours, and the clips that carry the same one are a **group** — the
//! group is derived from the labels and not written down (the module
//! `labels`).
//!
//! And a clip may be **trimmed**: one of its two edges moves, and a
//! trim that would cross a bound is **refused** and says which bound
//! stopped it — the tree clamps the same trim and says nothing (the
//! module `trim`).
//!
//! And a clip may be **moved**: along its track or to another one, and
//! a move that would land on another clip is **refused** and names the
//! clip that would have been cut — the tree cuts it and says nothing
//! about the cut (the module `moving`).
//!
//! And a clip may be **inserted**: a gap opens at the place and the
//! clips that are there are pushed along, so that nothing is cut — the
//! tree's second mode of the move (the module `inserting`).
//!
//! And a clip may be **taken away**: the edge that says which item it
//! played goes with it, and the gap it leaves is a gap — the tree's
//! delete, and not its ripple delete (the module `removal`).
//!
//! And a clip may be **rippled** away: the gap closes and the clips
//! that were after it move up, and the ripple is refused when the time
//! the clip took is not free (the module `rippling`).
//!
//! And a **range** of a track may be **lifted**: the clips that are in
//! it go, and the gap they leave is a gap — the tree's lift (the module
//! `lifting`). And the same range may be **extracted**: the same
//! removal, and the gap closes.
//!
//! And a clip may play its media **backwards**: the direction is a
//! property of the clip and not of the media, and it changes what the
//! clip's edges consume — a reversed clip's head is the *end* of its
//! media — and what a split of it does (the module `reversing`).
//!
//! And a track may be **locked**: a locked track cannot be edited, and
//! every command that would edit it is refused — the tree's locked
//! track, and the tree's `unlocked_track` (the module `locking`).
//!
//! And a track may carry a **transition**: a thing that sits over the
//! cut between two clips that are next to each other, with a duration
//! and an alignment on the cut, and it may not leave the clips it
//! joins — the tree's transitions (the module `transitions`).
//!
//! And a clip may play its media **faster or slower** than it is: the
//! speed is the third thing that joins the two clocks, and the media a
//! clip plays is its length multiplied by its speed (the module
//! `speed`).
//!
//! And two clips may be **linked**: the same thing on two tracks — a
//! picture and its sound — and a move of one moves the other (the
//! module `linking`).
//!
//! And a clip may **hold** a frame: it plays its media up to a point in
//! the media and then holds that frame to the end of the clip, so its
//! material is **shorter** than the clip (the module `hold`).
//!
//! And a clip may be **slipped**: it stays where it is on the timeline
//! and how long it is, and it shows a **different part of its media** —
//! and a slip that would run past the end of the media is **refused**
//! and says how long the media is, where the tree clamps it and says
//! nothing (the module `slipping`).
//!
//! And a **cut** may be **rolled**: the place where two clips meet
//! moves, the left clip's tail grows and the right clip's head shrinks,
//! and the media follows — and a roll that would cross a bound is
//! **refused** and says which bound stopped it, where the tree clamps
//! it and says nothing (the module `rolling`).
//!
//! And a clip may be **slid** between its neighbours: it moves and the
//! clips around it give way, so that the span they share stays
//! without a gap — and a slide that would cross a bound is **refused**
//! and says which bound stopped it (the module `sliding`).
//!
//! And a clip may be **stretched**: it gets longer or shorter and its
//! speed changes the other way, so that it plays the **same material**
//! — and a stretch that would cross a bound is **refused** and says
//! which bound stopped it (the module `stretching`).
//!
//! And a sequence may carry **markers**: named places on its timeline,
//! with a comment, a colour and a kind (the module `markers`).
//!
//! And a **media** may carry **marks**: a mark in and a mark out, and
//! the material a clip plays is between them (the module `mediamarks`).
//!
//! And a **media item** may be **read**: its name, its own id, the
//! path of the media, its length in frames, whether it was made
//! offline on purpose and its label — and the bin it is in and the
//! clips that play it, which are derived (the module `itemreading`).
//!
//! And a **clip** may be read whole: where it starts, how long it is,
//! where in the media it begins, how fast it plays and what it plays —
//! and the track, the sequence and the item it belongs to, which are
//! derived (the module `clipreading`).
//!
//! And a **track** may be read whole: its name, its kind and whether it
//! is locked — and its clips in the order they start, its length and
//! its sequence, which are derived (the module `trackreading`).
//!
//! And a **bin** may be read whole: its name, the items in it, the
//! bins in it, and the bin it is in, which is derived (the module
//! `binreading`).
//!
//! And a **sequence** may carry a **rate**: how many frames there are
//! in a second — 25, or the tree's own rational rates — and the length
//! of a sequence in seconds is derived from the frames and the rate
//! (the module `rates`).
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

mod ale;
mod aleimport;
mod anchors;
mod binreading;
mod binremoval;
mod captions;
mod captiontracks;
mod clipreading;
mod edits;
mod edl;
mod edlimport;
mod enabling;
mod fcp7import;
mod fcpxml;
mod fcpxmlimport;
mod gaps;
mod hold;
mod inserting;
mod itemnaming;
mod itemreading;
mod itemremoval;
mod labels;
mod lifting;
mod linking;
mod locking;
mod markers;
mod mediamarks;
mod moving;
mod naming;
mod nesting;
mod otio;
mod otioimport;
mod paths;
mod playhead;
mod project;
mod rates;
mod removal;
mod replacing;
mod reversing;
mod rippling;
mod rolling;
mod sliding;
mod slipping;
pub mod speed;
mod splitting;
mod stl;
mod stretching;
mod trackreading;
mod trackremoval;
mod tracks;
mod transitions;
mod trim;
mod workarea;

pub use ale::ExportAle;
pub use aleimport::ImportAle;
pub use anchors::SynchroniseClip;
pub use binreading::ReadBin;
pub use binremoval::RemoveBin;
pub use captions::ReadCaptions;
pub use captiontracks::{
    AddCaption, AddCaptionTrack, ImportCaptions, ReadCaptions as ReadCaptionTrack, RemoveCaption,
};
pub use clipreading::ReadClip;
pub use edits::{NextEdit, PrevEdit, Razor};
pub use edl::ExportEdl;
pub use edlimport::ImportEdl;
pub use enabling::EnableClip;
pub use fcp7import::ImportFcp7;
pub use fcpxml::ExportFcpxml;
pub use fcpxmlimport::ImportFcpxml;
pub use gaps::CloseGap;
pub use hold::{DeleteHold, SetHold};
pub use inserting::InsertClip;
pub use itemnaming::{RenameBin, RenameItem};
pub use itemreading::ReadItem;
pub use itemremoval::RemoveItem;
pub use labels::{DeleteItemLabel, DeleteLabel, ItemLabelGroup, LabelGroup, SetItemLabel, SetLabel};
pub use lifting::LiftRange;
pub use linking::{LinkClips, UnlinkClip};
pub use locking::LockTrack;
pub use markers::{AddMarker, ClearMarkers, MarkerKind, RemoveMarker, SetMarker};
pub use mediamarks::SetMarks;
pub use moving::MoveClip;
pub use naming::RenameClip;
pub use otio::ExportOtio;
pub use otioimport::ImportOtio;
pub use playhead::{GetPlayhead, GoToEnd, GoToStart, SetPlayhead, StepPlayhead};
pub use project::ReadProject;
pub use rates::{GetRate, SetRate};
pub use removal::RemoveClip;
pub use replacing::ReplaceMedia;
pub use reversing::ReverseClip;
pub use rippling::RippleClip;
pub use rolling::RollCut;
pub use sliding::SlideClip;
pub use slipping::SlipClip;
pub use speed::SetSpeed;
pub use splitting::SplitClip;
pub use stretching::StretchClip;
pub use trackreading::ReadTrack;
pub use trackremoval::RemoveTrack;
pub use tracks::{EnableTrack, MuteTrack, RenameTrack, SetPan, SetTrackLabel, SetVolume, SoloTrack};
pub use transitions::{AddTransition, RemoveTransition, SetTransition, TransitionKind};
pub use trim::TrimClip;

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

/// A **caption track**: the cues of a sequence.
#[derive(NodeKind)]
#[node_kind(kind = "film.caption.track", label = "Caption Track", layer = "film", evaluates)]
pub struct CaptionTrackKind;

impl CaptionTrackKind {
    /// A caption track is not drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Whether a point is inside a caption track: never.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a caption track **means**: its format, its language and how
    /// many cues it holds.
    fn evaluate(node: &Node, _document: &Document) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "format": node.data.get("format").cloned().unwrap_or(serde_json::Value::Null),
            "language": node.data.get("language").cloned().unwrap_or(serde_json::Value::Null),
            "cues": node.children.iter().filter(|child| child.kind == "film.caption").count(),
        }))
    }
}

/// A **cue**: the text of a caption and the two times it is on the
/// screen.
#[derive(NodeKind)]
#[node_kind(kind = "film.caption", label = "Caption", layer = "film", evaluates)]
pub struct CaptionKind;

impl CaptionKind {
    /// A cue is not drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Whether a point is inside a cue: never.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a cue **means**: the two times and the text.
    fn evaluate(node: &Node, _document: &Document) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "start": node.data.get("start").cloned().unwrap_or(serde_json::Value::Null),
            "end": node.data.get("end").cloned().unwrap_or(serde_json::Value::Null),
            "text": node.data.get("text").cloned().unwrap_or(serde_json::Value::Null),
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
            "enabled": crate::enabling::is_enabled(&node.data),
            "name": crate::naming::given_name(&node.data),
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
        // A **locked** track cannot be edited, and a clip on it is an
        // edit.
        if is_locked(session, track) {
            return Err(locked_error("film.clip.add", track));
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
            "speed": 1.0,
            "link": serde_json::Value::Null,
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
            // A clip that does not **play** is on the track and it takes
            // its place there, and it is not played: the count of the
            // clips that play is **derived**.
            let playing = track
                .children
                .iter()
                .filter(|child| child.kind == "film.clip" && crate::enabling::is_enabled(&child.data))
                .count();
            tracks.push(serde_json::json!({
                "track": track.id,
                "name": track.name,
                "clips": clips.iter().map(|(start, duration)| serde_json::json!({
                    "start": start,
                    "duration": duration,
                })).collect::<Vec<_>>(),
                "end": end,
                "playing": playing,
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
    commands(
        "film.bin.add",
        "film.item.add",
        "film.track.add",
        "film.clip.add",
        "film.clip.trim",
        "film.clip.move"
    )
)]
pub struct FilmPanel;

/// The name a clip is shown by.
///
/// A clip that the caller has **named** keeps that name, and a clip
/// that has not been named is shown by the place it is at — `clip at
/// 120`. The name of a clip is therefore a **fact** once it is given
/// and a **derivation** until then, and a clip that is moved keeps the
/// name it was given.
pub fn clip_name_of(data: &serde_json::Value, start: u64) -> String {
    match data.get("name").and_then(|name| name.as_str()) {
        Some(name) if !name.is_empty() => name.to_string(),
        _ => format!("clip at {start}"),
    }
}

/// Put the **name** of a clip on a node, and say which way it went.
///
/// The node's name is what the document shows, and it follows the
/// clip's own name when there is one.
pub fn set_clip_name(node: &mut craftkit_sdk::Node, start: u64) {
    node.name = clip_name_of(&node.data, start);
}

/// The payload of a clip with its **clocks** set.
///
/// The rest of the payload is kept: a clip may carry more than its
/// three clocks — whether it plays backwards is one of those things —
/// and taking it away would be taking away something the caller did
/// not ask about.
pub fn clocks_of(
    data: &serde_json::Value,
    start: u64,
    duration: u64,
    source_in: &serde_json::Value,
) -> serde_json::Value {
    let mut new = data.clone();
    if !new.is_object() {
        new = serde_json::json!({});
    }
    new["start"] = serde_json::json!(start);
    new["duration"] = serde_json::json!(duration);
    new["source_in"] = source_in.clone();
    new
}

/// A **neighbour** of a clip on its own track: which clip it is,
/// where it starts and where it ends.
#[derive(Clone, Copy)]
pub struct Neighbour {
    /// Which clip it is.
    pub id: u64,
    /// Where it starts on the timeline.
    pub start: u64,
    /// Where it ends on the timeline.
    pub end: u64,
}

/// The **neighbours** of a clip on its own track: the clip before it
/// and the clip after it.
///
/// A neighbour is the clip whose end is the nearest before the clip's
/// start, and the clip whose start is the nearest after the clip's
/// end. A **gap** is not a neighbour: a clip with a gap between it and
/// the next one is still the next one.
pub fn neighbours_of(session: &Session, clip: u64) -> (Option<Neighbour>, Option<Neighbour>) {
    let mut before: Option<Neighbour> = None;
    let mut after: Option<Neighbour> = None;
    let Some(node) = session.document().node(clip) else {
        return (before, after);
    };
    let start = node.data.get("start").and_then(|start| start.as_u64()).unwrap_or(0);
    let duration = node.data.get("duration").and_then(|duration| duration.as_u64()).unwrap_or(0);
    let end = start + duration;
    let Some(track) = track_of(session, clip) else {
        return (before, after);
    };
    let Some(track_node) = session.document().node(track) else {
        return (before, after);
    };
    for child in track_node.children.iter().filter(|child| child.kind == "film.clip") {
        if child.id == clip {
            continue;
        }
        let Some(other_start) = child.data.get("start").and_then(|start| start.as_u64()) else {
            continue;
        };
        let Some(duration) = child.data.get("duration").and_then(|duration| duration.as_u64()) else {
            continue;
        };
        let other_end = other_start + duration;
        if other_end <= start && before.map(|neighbour| other_end > neighbour.end).unwrap_or(true) {
            before = Some(Neighbour { id: child.id, start: other_start, end: other_end });
        } else if other_start >= end && after.map(|neighbour| other_start < neighbour.start).unwrap_or(true) {
            after = Some(Neighbour { id: child.id, start: other_start, end: other_end });
        }
    }
    (before, after)
}

/// How long the **media** a clip plays is.
///
/// The item the clip plays is a relation, and the item's own length is
/// in its payload.
pub fn media_length_of(session: &Session, clip: u64) -> Option<u64> {
    let item = session
        .document()
        .relations
        .iter()
        .find(|relation| relation.kind == "clip" && relation.from == clip)
        .map(|relation| relation.to)?;
    let node = session.document().node(item)?;
    let duration = node.data.get("duration")?.as_u64()?;
    if duration == 0 { None } else { Some(duration) }
}

/// The **media** a clip may play: the range of the media's frames that
/// a clip may play, and the media's own length.
///
/// The tree's media carries a **mark in** and a **mark out**, and the
/// material a clip plays is between them. Here the marks are the
/// item's own facts, and a media without them is all of itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediaRange {
    /// The first frame of the media a clip may play.
    pub start: u64,
    /// The frame after the last one a clip may play.
    pub end: u64,
    /// How long the whole media is.
    pub length: u64,
    /// Whether the media has marks of its own.
    pub marked: bool,
}

impl MediaRange {
    /// What the media allows, in words.
    pub fn bound(&self) -> String {
        if self.marked {
            format!(
                "the media is {} frames long and its marks are {} and {}",
                self.length, self.start, self.end
            )
        } else {
            format!("the media is {} frames long", self.length)
        }
    }
}

/// The **media** a clip may play: the range of its frames, if it is
/// known.
pub fn media_range_of(session: &Session, clip: u64) -> Option<MediaRange> {
    let item = session
        .document()
        .relations
        .iter()
        .find(|relation| relation.kind == "clip" && relation.from == clip)
        .map(|relation| relation.to)?;
    let node = session.document().node(item)?;
    let length = node.data.get("duration")?.as_u64()?;
    if length == 0 {
        return None;
    }
    let mark_in = node.data.get("mark_in").and_then(|mark| mark.as_u64());
    let mark_out = node.data.get("mark_out").and_then(|mark| mark.as_u64());
    let marked = mark_in.is_some() || mark_out.is_some();
    let start = mark_in.unwrap_or(0);
    let end = mark_out.unwrap_or(length).min(length);
    Some(MediaRange { start, end, length, marked })
}

/// The **sequence** a clip is in: the item the track is in.
pub fn sequence_of(session: &Session, clip: u64) -> Option<u64> {
    let track = track_of(session, clip)?;
    session
        .document()
        .walk()
        .find(|node| node.children.iter().any(|child| child.id == track))
        .map(|node| node.id)
}

/// The **track** a clip is on.
pub fn track_of(session: &Session, clip: u64) -> Option<u64> {
    session
        .document()
        .walk()
        .find(|node| node.kind == "film.track" && node.children.iter().any(|child| child.id == clip))
        .map(|node| node.id)
}

/// Whether a track is **locked**: a locked track cannot be edited.
pub fn is_locked(session: &Session, track: u64) -> bool {
    session
        .document()
        .node(track)
        .and_then(|node| node.data.get("locked").and_then(|locked| locked.as_bool()))
        .unwrap_or(false)
}

/// The refusal a **locked** track gives.
pub fn locked_error(id: &str, track: u64) -> EngineError {
    EngineError::Failed { id: id.to_string(), message: format!("the track `{track}` is locked") }
}

/// Whether a clip plays its media **backwards**.
pub fn is_reversed(data: &serde_json::Value) -> bool {
    data.get("reverse").and_then(|reverse| reverse.as_bool()).unwrap_or(false)
}

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
    ExportEdl,
    ImportEdl,
    ExportAle,
    ImportAle,
    ExportFcpxml,
    ImportFcpxml,
    ImportFcp7,
    ExportOtio,
    ImportOtio,
    ReadTimeline,
    TrimClip,
    SetPlayhead,
    GetPlayhead,
    NextEdit,
    PrevEdit,
    Razor,
    GoToStart,
    GoToEnd,
    StepPlayhead,
    MoveClip,
    InsertClip,
    SplitClip,
    RemoveClip,
    ReadProject,
    RemoveBin,
    RemoveItem,
    RenameBin,
    RenameItem,
    RippleClip,
    LiftRange,
    TransitionKind,
    MarkerKind,
    AddMarker,
    SetMarker,
    RemoveMarker,
    ClearMarkers,
    SetSpeed,
    LinkClips,
    UnlinkClip,
    SetHold,
    SynchroniseClip,
    DeleteHold,
    AddTransition,
    SetTransition,
    RemoveTransition,
    ReverseClip,
    LockTrack,
    RemoveTrack,
    EnableTrack,
    MuteTrack,
    SoloTrack,
    SetVolume,
    SetPan,
    SetTrackLabel,
    RenameTrack,
    EnableClip,
    RenameClip,
    SetItemLabel,
    DeleteItemLabel,
    ItemLabelGroup,
    ReplaceMedia,
    CloseGap,
    SlipClip,
    SlideClip,
    SetMarks,
    StretchClip,
    RollCut,
    TimelineEvaluator,
    FilmPanel
);
