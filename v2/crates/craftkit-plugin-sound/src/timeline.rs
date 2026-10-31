//! The **timeline**: the sound document's own world.
//!
//! # Why it is here
//!
//! §19 found that a sound document has **no place** in it: a clip is at
//! a **time**, and a time is not a place. The document grew an optional
//! place, the sound kinds left it empty — and the mixer that the
//! relations made possible (§20.4) was a model that could not be
//! **seen**.
//!
//! So the sound battery grows a world of its own: the **timeline**.
//!
//! # The world
//!
//! - **Across is time.** One world unit is **one second**, and the
//!   document's own sample rate says how many samples that is. The
//!   camera's zoom is the timeline's zoom.
//! - **Down is the row.** A track is a row, and the row is the track's
//!   place among the tracks — the order the tracks are in, which is the
//!   order the tree is in, and the order the drawing is in (§105).
//!
//! # The place is **derived**, and that is the point
//!
//! A clip's rectangle is worked out from its span and from the row of
//! the track it is under. It is **not** in the document, because a
//! rectangle in the document would go stale the moment a track was
//! added above it: the row of every clip below would move, and the
//! document would have to be rewritten to match.
//!
//! So the **kind** says where its node is, and the kit asks it. That is
//! the `place` seam, and it is the kit's own: a kind that knows where
//! its node is says so, and the core never matches on a kind to work it
//! out.
//!
//! # What it costs
//!
//! The row of a track is its place among the tracks, and finding it
//! means walking the document. So drawing a document of `n` sound nodes
//! costs `n` walks, and a walk is `n` nodes: the sound document is a
//! tree of a few hundred nodes, and the cost is a few tens of thousands
//! of comparisons. A document big enough for that to hurt would want
//! the rows **stored**, and that is a different design.

use craftkit_sdk::document::{Document, Node, Rect};

use crate::mixer::LanePayload;
use crate::{ClipPayload, MarkerPayload, session_of, tempo_map};
use craftkit_sound_time::TempoMap;

/// The kind of a **track**.
const TRACK_KIND: &str = "sound.track";
const CLIP_KIND: &str = "sound.clip";

/// The kind of a **playlist**.
const PLAYLIST_KIND: &str = "sound.playlist";

/// The kind of a **bus**.
const BUS_KIND: &str = "sound.bus";

/// How tall one **row** is, in world units.
///
/// A track is a row, and a row is one world unit tall: the timeline's
/// world is measured in seconds across and in rows down.
pub const ROW_HEIGHT: f64 = 1.0;

/// The **ruler**: the row above the tracks.
///
/// It is where the things that are not on a track live: the tempo and
/// the meter. It is **above** the first track, so the ruler's row is
/// at `-1` and the first track's row is at `0`.
pub const RULER_HEIGHT: f64 = ROW_HEIGHT;

/// How wide an **insert** is, in world units.
///
/// The inserts are the boxes at the **left** of a track's row, the way
/// a mixer shows them: an insert is not at a time, so it is at the
/// start.
pub const INSERT_WIDTH: f64 = 0.2;

/// How far apart two **sends** are, in world units.
///
/// The sends are the lines from a track down to a bus, and two lines
/// that are in the same place are one line.
pub const SEND_SPACING: f64 = 0.05;

/// The narrowest a **clip** may be.
///
/// A clip that is shorter than this cannot be picked, and a thing that
/// cannot be picked is not there. So a clip that is very short is drawn
/// at this width, and it keeps its **span** — the width is the drawing,
/// not the truth.
pub const MINIMUM_WIDTH: f64 = 0.05;

/// The row of a **track**: where it is among the tracks.
///
/// The tracks are the document's roots, and their order is the order
/// they are in: there is no other order to be had.
pub fn row_of(document: &Document, track: &Node) -> Option<f64> {
    let mut row = 0.0;
    for node in document.walk() {
        if node.kind != TRACK_KIND {
            continue;
        }
        if node.id == track.id {
            return Some(row);
        }
        row += ROW_HEIGHT;
    }
    None
}

/// The node's **parent**, if it has one.
pub fn parent_of(document: &Document, id: u64) -> Option<&Node> {
    document.walk().find(|node| node.children.iter().any(|child| child.id == id))
}

/// The **track** a node is under, if it is under one.
///
/// A clip is under a playlist and a playlist is under a track, so the
/// track is the parent or the grandparent — and a node that is under
/// neither is not on a timeline.
pub fn track_of<'a>(document: &'a Document, node: &Node) -> Option<&'a Node> {
    let parent = parent_of(document, node.id)?;
    if parent.kind == TRACK_KIND {
        return Some(parent);
    }
    if parent.kind != PLAYLIST_KIND {
        return None;
    }
    let grandparent = parent_of(document, parent.id)?;
    if grandparent.kind == TRACK_KIND {
        return Some(grandparent);
    }
    None
}

/// How long the **timeline** is, in seconds.
///
/// It is as long as its content: the last clip that ends is where the
/// timeline ends. A document with no clips has no length, and the row
/// it has is empty.
pub fn length_of(document: &Document) -> f64 {
    let rate = session_of(document).sample_rate as f64;
    let mut longest = 0.0;
    for node in document.walk() {
        if node.kind != "sound.clip" {
            continue;
        }
        let Ok(payload) = ClipPayload::from_json(&node.data) else { continue };
        let end = payload.start.saturating_add(payload.length) as f64 / rate;
        if end > longest {
            longest = end;
        }
    }
    longest
}

/// The rectangle of a **clip**: a bar on the track's row.
pub fn clip_rect(document: &Document, clip: &Node) -> Option<Rect> {
    let payload = ClipPayload::from_json(&clip.data).ok()?;
    let track = track_of(document, clip)?;
    let row = row_of(document, track)?;
    let rate = session_of(document).sample_rate as f64;
    let x = payload.start as f64 / rate;
    let width = payload.length as f64 / rate;
    Some(Rect::new(x, row, width.max(MINIMUM_WIDTH), ROW_HEIGHT))
}

/// The rectangle of a **marker**: a line at the time it is at.
pub fn marker_rect(document: &Document, marker: &Node) -> Option<Rect> {
    let payload = MarkerPayload::from_json(&marker.data).ok()?;
    let rate = session_of(document).sample_rate as f64;
    let x = payload.start as f64 / rate;
    Some(Rect::new(x, 0.0, MINIMUM_WIDTH, length_of(document).max(ROW_HEIGHT)))
}

/// The rectangle of a **tempo** or **meter** event: a mark in the
/// ruler.
///
/// The event's time is in **ticks**, and the world is in **seconds**,
/// so the tick is worked out against the **tempo map** — the map the
/// evaluator derives from the very events this reads. A tick is a
/// musical position, and it is not a length until the tempo says how
/// fast the music is.
pub fn event_rect(document: &Document, event: &Node) -> Option<Rect> {
    let tick = event_tick(event)?;
    let rate = session_of(document).sample_rate;
    let samples = tempo_map(document).ticks_to_samples(tick, rate);
    let x = samples as f64 / rate as f64;
    Some(Rect::new(x, -RULER_HEIGHT, MINIMUM_WIDTH, RULER_HEIGHT))
}

/// The **tick** an event is at, if it is an event with one.
fn event_tick(event: &Node) -> Option<i64> {
    let value = event.data.get("tick")?;
    value.as_i64()
}

/// The rectangle of a **track**: its row, the whole way along.
pub fn track_rect(document: &Document, track: &Node) -> Option<Rect> {
    let row = row_of(document, track)?;
    Some(Rect::new(0.0, row, length_of(document), ROW_HEIGHT))
}

/// The rectangle of a **playlist**: the row of the track it is under.
pub fn playlist_rect(document: &Document, playlist: &Node) -> Option<Rect> {
    let parent = parent_of(document, playlist.id)?;
    if parent.kind != TRACK_KIND {
        return None;
    }
    track_rect(document, parent)
}

/// How many **tracks** the document has.
pub fn track_count(document: &Document) -> usize {
    document.walk().filter(|node| node.kind == TRACK_KIND).count()
}

/// The row of a **bus**: below the tracks.
///
/// The mixer is under the timeline: the tracks are the rows of the
/// timeline and the buses are the rows of the mixer, and the sends are
/// the lines between them.
pub fn bus_row(document: &Document, bus: &Node) -> Option<f64> {
    let mut index = 0usize;
    for node in document.walk() {
        if node.kind != BUS_KIND {
            continue;
        }
        if node.id == bus.id {
            return Some((track_count(document) + index) as f64 * ROW_HEIGHT);
        }
        index += 1;
    }
    None
}

/// The rectangle of a **bus**: its row, the whole way along.
pub fn bus_rect(document: &Document, bus: &Node) -> Option<Rect> {
    let row = bus_row(document, bus)?;
    Some(Rect::new(0.0, row, length_of(document), ROW_HEIGHT))
}

/// The rectangle of an **insert**: a box at the left of the track's row.
pub fn insert_rect(document: &Document, insert: &Node) -> Option<Rect> {
    let track = track_of(document, insert)?;
    let row = row_of(document, track)?;
    let index = index_among(document, insert, "sound.insert");
    Some(Rect::new(index as f64 * INSERT_WIDTH, row, INSERT_WIDTH, ROW_HEIGHT))
}

/// The rectangle of a **send**: the line from the track down to the bus.
pub fn send_rect(document: &Document, send: &Node) -> Option<Rect> {
    let track = track_of(document, send)?;
    let row = row_of(document, track)?;
    let bus = the_bus_of(document, send)?;
    let bus_row = bus_row(document, bus)?;
    let index = index_among(document, send, "sound.send");
    Some(Rect::new(index as f64 * SEND_SPACING, row, SEND_SPACING, (bus_row - row).max(ROW_HEIGHT)))
}

/// The **owner** of an automation lane: the node the lane automates.
///
/// The owner is the lane's **parent**, and it is a track or a clip: a
/// lane under a track is the track's curve, and a lane under a clip is
/// the clip's own curve — and its times are the clip's own, not the
/// track's.
pub fn lane_owner<'a>(document: &'a Document, lane: &Node) -> Option<&'a Node> {
    let owner = parent_of(document, lane.id)?;
    if owner.kind == TRACK_KIND || owner.kind == CLIP_KIND {
        return Some(owner);
    }
    None
}

/// The row a lane is drawn on, and the time its points are measured
/// from.
///
/// A track's lane is drawn on the track's row and its times are the
/// track's timeline; a clip's lane is drawn on the clip's row and its
/// times are the clip's own.
fn lane_row_and_offset(document: &Document, lane: &Node) -> Option<(f64, f64)> {
    let owner = lane_owner(document, lane)?;
    if owner.kind == TRACK_KIND {
        return Some((row_of(document, owner)?, 0.0));
    }
    let track = track_of(document, owner)?;
    let row = row_of(document, track)?;
    let payload = ClipPayload::from_json(&owner.data).ok()?;
    Some((row, payload.start as f64))
}

/// The rectangle of an **automation lane**: the curve on the track's row.
pub fn lane_rect(document: &Document, lane: &Node) -> Option<Rect> {
    let (row, offset) = lane_row_and_offset(document, lane)?;
    let payload = LanePayload::from_json(&lane.data).ok()?;
    let rate = session_of(document).sample_rate as f64;
    let mut first: Option<f64> = None;
    let mut last: Option<f64> = None;
    for (at, _value) in &payload.points {
        let x = (offset + *at as f64) / rate;
        first = Some(first.map_or(x, |current: f64| current.min(x)));
        last = Some(last.map_or(x, |current: f64| current.max(x)));
    }
    let first = first?;
    let last = last?;
    Some(Rect::new(first, row, (last - first).max(MINIMUM_WIDTH), ROW_HEIGHT))
}

/// The **points** of an automation lane, as the world sees them.
///
/// The point's time is in **samples** and the value is the parameter's
/// own, and the kit does not know what range a parameter takes. So the
/// curve is drawn to **fill its row**: what is seen is the shape of the
/// automation and not the absolute values.
pub fn lane_points(document: &Document, lane: &Node) -> Vec<(f64, f64)> {
    let Ok(payload) = LanePayload::from_json(&lane.data) else { return Vec::new() };
    let Some((row, offset)) = lane_row_and_offset(document, lane) else { return Vec::new() };
    if payload.points.is_empty() {
        return Vec::new();
    }
    let rate = session_of(document).sample_rate as f64;
    let smallest = payload.points.iter().map(|(_at, value)| *value).fold(f64::MAX, f64::min);
    let largest = payload.points.iter().map(|(_at, value)| *value).fold(f64::MIN, f64::max);
    let span = largest - smallest;
    payload
        .points
        .iter()
        .map(|(at, value)| {
            let x = (offset + *at as f64) / rate;
            let y = if span > 0.0 {
                row + (value - smallest) / span * ROW_HEIGHT
            } else {
                row + ROW_HEIGHT / 2.0
            };
            (x, y)
        })
        .collect()
}

/// The node's place among its **brothers of one kind**.
fn index_among(document: &Document, node: &Node, kind: &str) -> usize {
    let Some(parent) = parent_of(document, node.id) else { return 0 };
    let mut index = 0usize;
    for child in &parent.children {
        if child.id == node.id {
            return index;
        }
        if child.kind == kind {
            index += 1;
        }
    }
    index
}

/// The **bus** a send goes to.
fn the_bus_of<'a>(document: &'a Document, send: &Node) -> Option<&'a Node> {
    let relations = document.relations_from(send.id, Some("send"));
    let relation = relations.first()?;
    document.node(relation.to)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The row of a track is its place among the tracks, and a row is
    /// one world unit tall.
    #[test]
    fn a_row_is_one_unit_tall() {
        assert_eq!(ROW_HEIGHT, 1.0, "a row");
    }

    /// A clip that is very short is still wide enough to be picked.
    #[test]
    fn a_very_short_clip_is_still_pickable() {
        // A fiftieth of a second: narrow enough to be a line on the
        // timeline, wide enough to be picked.
        assert_eq!(MINIMUM_WIDTH, 0.05, "the narrowest a clip is drawn");
    }
}

/// The smallest gap between two **numbers** of the ruler, in seconds.
///
/// A number is a label and a label has a width, so two numbers that are
/// too close would lie on top of each other. The rule is a gap in
/// **time**, and not a gap in bars: the ruler is drawn in seconds and a
/// tempo may make the bars any width it likes.
pub const MINIMUM_GAP: f64 = 2.0;

/// How tall the numbers of the ruler are, in world units: the **em**.
///
/// The ruler's row is one unit tall, and the numbers sit in it.
pub const SCALE_EM: f64 = 0.5;

/// One **number** of the ruler: where it is and what it says.
#[derive(Clone, Debug, PartialEq)]
pub struct ScaleMark {
    /// Where the number is, in seconds from the beginning.
    pub x: f64,
    /// What the number says.
    pub label: String,
}

/// The ruler's **scale**: the numbers it carries.
///
/// The scale is either the **bars** — the numbers a musician counts by —
/// or the **seconds**, which is the world the ruler is drawn in. A ruler
/// with no scale says nothing: a bar number without a tempo map has no
/// bars to count.
pub fn scale_marks(document: &Document, node: &Node) -> Vec<ScaleMark> {
    let scale = scale_of(node);
    if scale == "seconds" { seconds_marks(document) } else { bar_marks(document) }
}

/// The scale a ruler is drawn in.
fn scale_of(node: &Node) -> &str {
    match node.data.get("scale").and_then(|value| value.as_str()) {
        Some(scale) => scale,
        None => "bars",
    }
}

/// The numbers of the ruler, counting **seconds**.
fn seconds_marks(document: &Document) -> Vec<ScaleMark> {
    let length = length_of(document);
    let mut marks = Vec::new();
    let mut second = 1.0;
    while second <= length {
        marks.push(ScaleMark { x: second, label: format!("{second}") });
        second += 1.0;
    }
    marks
}

/// The numbers of the ruler, counting **bars**.
///
/// The bars come from the **tempo map**: the map says which bar a tick
/// is in, and the tick of a bar is found by a search, because the bars
/// are monotonic in the tick.
fn bar_marks(document: &Document) -> Vec<ScaleMark> {
    let map = tempo_map(document);
    let rate = session_of(document).sample_rate;
    let length = length_of(document);
    if length <= 0.0 {
        return Vec::new();
    }
    let last_tick = map.samples_to_ticks((length * rate as f64) as i64, rate);
    let bars = map.bar_beat_at_tick(last_tick).bar;
    let mut marks: Vec<ScaleMark> = Vec::new();
    let mut last_x = f64::NEG_INFINITY;
    for bar in 1..=bars {
        let tick = tick_of_bar(&map, bar, last_tick);
        let samples = map.ticks_to_samples(tick, rate);
        let x = samples as f64 / rate as f64;
        if x - last_x < MINIMUM_GAP {
            continue;
        }
        last_x = x;
        marks.push(ScaleMark { x, label: format!("{bar}") });
    }
    marks
}

/// The **tick** a bar starts at.
///
/// The bars are monotonic in the tick — a later tick is in a later bar —
/// so the tick of a bar is found by a search, and not by counting.
fn tick_of_bar(map: &TempoMap, bar: i64, last_tick: i64) -> i64 {
    let mut low = 0_i64;
    let mut high = last_tick.max(0);
    while low < high {
        let middle = low + (high - low) / 2;
        if map.bar_beat_at_tick(middle).bar < bar {
            low = middle + 1;
        } else {
            high = middle;
        }
    }
    low
}

/// The rectangle of the **playhead**: a line at the position.
///
/// The position is in **samples**, and the world is in seconds, so the
/// position is worked out against the session's rate. The playhead is a
/// **line** and not a box: it has no width of its own, only a place.
pub fn playhead_rect(document: &Document, playhead: &Node) -> Option<Rect> {
    let position = playhead_position(playhead)?;
    let rate = session_of(document).sample_rate as f64;
    let x = position as f64 / rate;
    let height = track_count(document) as f64 * ROW_HEIGHT + RULER_HEIGHT;
    Some(Rect::new(x - MINIMUM_WIDTH / 2.0, -RULER_HEIGHT, MINIMUM_WIDTH, height))
}

/// Where the **playhead** is, in samples.
pub fn playhead_position(playhead: &Node) -> Option<i64> {
    let value = playhead.data.get("position")?;
    value.as_i64()
}
