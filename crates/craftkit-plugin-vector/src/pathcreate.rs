//! The **making** of a path by hand.
//!
//! The tree's pen makes a path out of one of two things: a run of path
//! data written in the SVG spelling, or a list of anchors with their
//! handles. This module is the port of that: one command that takes
//! either and puts the path in the document.
//!
//! The two ways are the tree's own, and the tree's rules about them are
//! the rules here:
//!
//! - the **path data** is read by the reader this crate already has for
//!   SVG files, so there is one reader of path data in this tree and not
//!   two; and an **arc** is refused, as it is there, because this tree
//!   does not write a curve it cannot say;
//! - an **anchor** is a point and two handles, and a handle that is not
//!   given is the point itself: the tree's own rule;
//! - anything **malformed** in an anchor is refused rather than left out
//!   or replaced by a default, because a default handle changes the
//!   shape of the path.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, Outcome, Path, Text};

use crate::add_path;
use crate::anchors::{contour, read_anchors};
use crate::svgimport::read_path;

commands! {
    /// Put a path in the document, made by hand.
    ///
    /// The path is made one of two ways: the **path data** says what the
    /// path is, or the **anchors** do. The tree's own command takes either,
    /// and this one does the same.
    vector.path.create {
        area: "vector",
        layer: "vector",
        icon: "path",
        mutates,
        undoable,
        menu: ("Create"),
        params: {
            /// The path data, in the SVG spelling: the moves, the lines and the
            /// curves of the path. An **arc** is refused.

            d: Option<Text>,
            /// The anchors of the path: a list of `{x, y}`, each with an
            /// optional `in` and `out` handle, each a pair of numbers.

            anchors: Option<Json>,
            /// Whether the path is closed.
            ///
            /// It says so for the **anchors**; with path data the data itself
            /// says whether the path is closed, as it does in the tree.

            closed: Option<Bool>,
            /// What to call it.

            name: Option<Text>,
            /// The colour it is filled with, as `#rrggbb`.

            fill: Option<Text>,
        },
        run: {

            let id = "vector.path.create".to_string();
            let path = match (&self.d, &self.anchors) {
                (Some(_), Some(_)) => {
                    return Err(EngineError::Failed {
                        id,
                        message: "give `d` or `anchors`, not both: they are two ways of saying one path"
                            .to_string(),
                    });
                }
                (Some(data), None) => {
                    read_path(&data.0).map_err(|message| EngineError::Failed { id: id.clone(), message })?
                }
                (None, Some(anchors)) => {
                    path_from_anchors(anchors, self.closed.map(|closed| closed.0).unwrap_or(false))
                        .map_err(|message| EngineError::Failed { id: id.clone(), message })?
                }
                (None, None) => {
                    return Err(EngineError::Failed {
                        id,
                        message: "give `d` or `anchors`: a path is made one of the two ways".to_string(),
                    });
                }
            };
            if path.is_empty() {
                return Err(EngineError::Failed { id, message: "the path has nothing in it".to_string() });
            }
            // The tree's own name for a path made by hand.
            let name = match &self.name {
                Some(name) => name.0.clone(),
                None => "Pen".to_string(),
            };
            let node = add_path(session, name, path, self.fill.as_ref())?;
            Ok(Outcome::changed(format!("added a path `{node}`")).with_data(json!({ "id": node })))

        }
    }
}

/// The path a list of anchors holds.
///
/// The anchors are the tree's own, and the contour they make is the
/// tree's own too: a side whose two handles are both their points is a
/// **line**, and any other side is a **curve** through them.
fn path_from_anchors(value: &Json, closed: bool) -> Result<Path, String> {
    let anchors = read_anchors(value, 0)?;
    if anchors.len() < 2 {
        return Err("a path needs at least two anchors".to_string());
    }
    Ok(Path::single(contour(&anchors, closed)))
}
