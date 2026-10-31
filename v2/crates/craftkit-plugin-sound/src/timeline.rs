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

use crate::{ClipPayload, MarkerPayload, session_of};

/// The kind of a **track**.
const TRACK_KIND: &str = "sound.track";

/// The kind of a **playlist**.
const PLAYLIST_KIND: &str = "sound.playlist";

/// How tall one **row** is, in world units.
///
/// A track is a row, and a row is one world unit tall: the timeline's
/// world is measured in seconds across and in rows down.
pub const ROW_HEIGHT: f64 = 1.0;

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
