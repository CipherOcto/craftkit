//! The **moving** of many points at once.
//!
//! The tree's Move Anchors moves the anchors that are chosen by a
//! **delta** — the distance of a drag — and the handles of an anchor
//! move with it: an anchor carries its two handles.
//!
//! The rules are the tree's own:
//!
//! - the move is a **delta**: two numbers, across and down;
//! - the numbers must be **numbers**;
//! - every point named must be **there**;
//! - the handles move with the point they belong to.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Point};

use crate::convert::read_points;
use crate::{move_point, vector_of, write_back};

commands! {
    /// Move many points of a path by one delta.
    vector.path.move.points {
        area: "vector",
        layer: "vector",
        icon: "path",
        mutates,
        undoable,
        menu: ("Path", "Points"),
        params: {
            /// The path.
            path: NodeRef,
            /// Which contour of the path, counting from zero. The first one if
            /// it is left out.
            contour: Option<Number>,
            /// The points, counting from zero: a list of whole numbers.
            points: Json,
            /// How far across the points go.
            dx: Number,
            /// How far down the points go.
            dy: Number,
        },
        run: {
            let id = "vector.path.move.points".to_string();
            let (dx, dy) = (self.dx.0, self.dy.0);
            if !dx.is_finite() || !dy.is_finite() {
                return Err(EngineError::Failed {
                    id,
                    message: "the move is two numbers, across and down".to_string(),
                });
            }
            let points =
                read_points(&self.points).map_err(|message| EngineError::Failed { id: id.clone(), message })?;
            let mut vector = vector_of(session, self.path.0)?;
            let contour = crate::index_of(self.contour, "the contour")
                .map_err(|reason| EngineError::Failed { id: id.clone(), message: reason })?;
            if points.is_empty() {
                return Ok(Outcome::new("no point was named")
                    .with_data(json!({ "id": self.path.0, "contour": contour })));
            }
            for point in &points {
                let Some(subpath) = vector.path.subpaths.get(contour) else {
                    return Err(EngineError::Failed {
                        id,
                        message: format!("the path has no contour {contour}"),
                    });
                };
                let count = subpath.segments.len() + 1;
                if *point >= count {
                    return Err(EngineError::Failed {
                        id,
                        message: format!("the contour has {count} points and not a point {point}"),
                    });
                }
                let from = if *point == 0 { subpath.start } else { subpath.segments[*point - 1].end() };
                let to = Point::new(from.x + dx, from.y + dy);
                move_point(&mut vector.path, contour, *point, to)
                    .map_err(|reason| EngineError::Failed { id: id.clone(), message: reason })?;
            }
            write_back(session, self.path.0, &vector)?;
            Ok(Outcome::changed("the points have moved")
                .with_data(json!({ "id": self.path.0, "contour": contour, "points": points })))

        }
    }
}
