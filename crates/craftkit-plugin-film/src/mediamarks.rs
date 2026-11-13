//! The **marks** of a media.
//!
//! A media may carry a **mark in** and a **mark out**, and the
//! material a clip plays is between them: the tree's media has marks,
//! and a clip of a marked media may not play outside it.
//!
//! And the marks are the **item's** facts — they belong to the media
//! and not to the clip, and every clip of that media plays between
//! them.
//!
//! # What the port found
//!
//! - **The tree's marks are the media's own and so are these.** The
//!   tree's item carries a `mark_in` and a `mark_out` in media time,
//!   and the tree's clip's `source_in` is in media time as well. Here
//!   the clip's `source_in` is in media time too — it has always been —
//!   and the marks say which part of that media may be played.
//! - **The tree's marks are optional and so are these.** A media
//!   without a mark in or a mark out is all of itself, and a mark in
//!   of nothing is the media's own beginning and a mark out of
//!   nothing is its end.
//!
//! # What it does not do
//!
//! - **The tree's marks may be set on the media and on the sequence.**
//!   The tree's sequence carries its own `mark_in` and `mark_out` for
//!   the media it plays, and they are the marks of the *sequence* and
//!   not of a media. Here the marks are the media's and a sequence's
//!   marks are not here.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome};

commands! {
    /// Put the **marks** on a media.
    ///
    /// The marks are the media's own frames: the material a clip plays is
    /// between them. Only what is named is changed, and a mark that is
    /// outside the media is refused.
    film.item.mark {
        area: "film",
        layer: "film",
        icon: "flag",
        mutates,
        undoable,
        params: {
            /// The media.
            item: NodeRef,
            /// The mark in: the first frame of the media that may be played.
            mark_in: Option<Number>,
            /// The mark out: the frame after the last one that may be played.
            mark_out: Option<Number>,
        },
        run: {
            let id = "film.item.mark".to_string();
            let item = self.item.0;
            let Some(node) = session.document().node(item) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{item}`"),
                });
            };
            if node.kind != "film.item" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{item}` is a `{}` and not an item", node.kind),
                });
            }
            let Some(length) = node.data.get("duration").and_then(|duration| duration.as_u64()) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the item `{item}` does not say how long it is"),
                });
            };
            let mark_in = frame(self.mark_in, &id)?;
            let mark_out = frame(self.mark_out, &id)?;
            if let Some(mark_in) = mark_in
                && mark_in > length
            {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the mark in is at frame {mark_in}, and the media is {length} frames long"),
                });
            }
            if let Some(mark_out) = mark_out
                && mark_out > length
            {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the mark out is at frame {mark_out}, and the media is {length} frames long"
                    ),
                });
            }
            let start = mark_in.or_else(|| node.data.get("mark_in").and_then(|mark| mark.as_u64()));
            let end = mark_out.or_else(|| node.data.get("mark_out").and_then(|mark| mark.as_u64()));
            if let (Some(start), Some(end)) = (start, end)
                && start > end
            {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the mark in is at frame {start} and the mark out is at frame {end}, and the first is after the second"
                    ),
                });
            }
            let mut data = node.data.clone();
            if let Some(mark_in) = mark_in {
                data["mark_in"] = json!(mark_in);
            }
            if let Some(mark_out) = mark_out {
                data["mark_out"] = json!(mark_out);
            }
            if let Some(node) = session.document_mut()?.node_mut(item) {
                node.data = data;
            }
            Ok(Outcome::changed(format!("the marks of the media `{item}` are set")).with_data(json!({
                "item": item,
                "mark_in": start,
                "mark_out": end,
                "length": length,
            })))

        }
    }
}

/// A frame of the media.
fn frame(value: Option<Number>, id: &str) -> Result<Option<u64>, EngineError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let frame = value.0;
    if !frame.is_finite() || frame < 0.0 || frame.fract() != 0.0 {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("a frame of the media is a whole number that is there: {frame}"),
        });
    }
    Ok(Some(frame as u64))
}
