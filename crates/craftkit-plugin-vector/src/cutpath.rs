//! The **cutting** of a path.
//!
//! The tree's scissors cut a path at a point: a **closed** path opens
//! there, and an **open** one becomes one path per piece. The shape
//! does not change — the pieces are the sides that were there.
//!
//! The rules are the tree's own:
//!
//! - the cut is at a **point of the contour**, and the points are
//!   counted from zero, as they are everywhere in this tree;
//! - a **closed** contour opens at the cut, and it is one piece whose
//!   ends meet where the cut was;
//! - an **open** contour is cut into two, and its **ends** are not cut:
//!   a piece with no side has no shape;
//! - a path of **one contour** becomes **one path per piece** — the
//!   first keeps the node and the others are new nodes beside it — and
//!   the pieces of a path of several contours **stay contours of it**.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{
    EngineError, FormatValue, NodeRef, Number, Outcome, Path, Point, Segment, Session, SubPath,
};

use crate::VectorNode;
use crate::{vector_of, write_back};

/// Cut a path at a **point**.
///
/// The path is cut at one point of one of its contours, and the pieces
/// are the sides that were there: the shape does not change.
#[derive(Command)]
#[command(
    id = "vector.path.cut",
    area = "vector",
    layer = "vector",
    icon = "path",
    menu("Path", "Cut"),
    mutates,
    undoable
)]
pub struct CutPath {
    /// The path.
    pub path: NodeRef,
    /// Which contour of the path, counting from zero. The first one if
    /// it is left out.
    pub contour: Option<Number>,
    /// The point of the contour to cut at, counting from zero.
    pub point: Number,
}

impl CutPath {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "vector.path.cut".to_string();
        let mut vector = vector_of(session, self.path.0)?;
        let contour = self.contour.as_ref().map(|contour| contour.0).unwrap_or(0.0);
        if !contour.is_finite() || contour < 0.0 || contour.fract() != 0.0 {
            return Err(EngineError::Failed {
                id,
                message: "a contour is counted from zero, in whole numbers".to_string(),
            });
        }
        let index = contour as usize;
        let Some(contour) = vector.path.subpaths.get(index) else {
            return Err(EngineError::Failed { id, message: format!("the path has no contour {index}") });
        };
        let point = self.point.0;
        if !point.is_finite() || point < 0.0 || point.fract() != 0.0 {
            return Err(EngineError::Failed {
                id,
                message: "a point is counted from zero, in whole numbers".to_string(),
            });
        }
        let pieces = cut(contour, point as usize)
            .map_err(|message| EngineError::Failed { id: id.clone(), message })?;
        let whole = vector.path.subpaths.len() == 1;
        let mut ids: Vec<u64> = Vec::new();
        if whole {
            // One path per piece: the first keeps the node, and the
            // others are new nodes beside it.
            let mut pieces = pieces.into_iter();
            let Some(first) = pieces.next() else {
                return Err(EngineError::Failed { id, message: "the cut left no path".to_string() });
            };
            vector.path = Path { subpaths: vec![first], rule: vector.path.rule };
            write_back(session, self.path.0, &vector)?;
            ids.push(self.path.0);
            let name = session.document().node(self.path.0).map(|node| node.name.clone()).unwrap_or_default();
            for piece in pieces {
                ids.push(add_piece(session, &name, &vector, Path::single(piece))?);
            }
        } else {
            // The pieces stay contours of the path.
            let mut subpaths = vector.path.subpaths.clone();
            subpaths.splice(index..=index, pieces);
            vector.path = Path { subpaths, rule: vector.path.rule };
            write_back(session, self.path.0, &vector)?;
            ids.push(self.path.0);
        }
        Ok(Outcome::changed("the path is cut").with_data(json!({ "ids": ids })))
    }
}

/// Put a **piece** of a cut path in the document, and give back its id.
///
/// The piece is the path it was cut from, with the shape of the piece:
/// the same name, the same paint and the same place.
fn add_piece(session: &mut Session, name: &str, vector: &VectorNode, path: Path) -> Result<u64, EngineError> {
    let mut piece = vector.clone();
    piece.path = path;
    let rect = craftkit_vector_geom::bounds(&piece.placed());
    let id =
        session.document_mut()?.add_node_with(name.to_string(), "vector.path", Some(rect), piece.to_json());
    Ok(id)
}

/// The **pieces** a contour is cut into.
///
/// A **closed** contour opens at the cut: it is one piece whose ends
/// meet where the cut was. An **open** contour is cut into two, and its
/// ends are not cut, because a piece with no side has no shape.
fn cut(contour: &SubPath, point: usize) -> Result<Vec<SubPath>, String> {
    let count = contour.segments.len();
    if count == 0 {
        return Err("the contour has no side to cut".to_string());
    }
    let mut sides: Vec<Segment> = contour.segments.clone();
    if contour.closed {
        // The closing side of a closed contour — the straight line back
        // to its first point — is part of the shape, and it becomes a
        // side of its own when the contour is opened.
        sides.push(Segment::Line { to: contour.start });
    }
    if contour.closed {
        if point > count {
            return Err(format!("the contour has {count} sides and not a point {point}"));
        }
        // The contour opens at the cut: one piece, going round from the
        // cut and back to it.
        let mut rotated: Vec<Segment> = Vec::with_capacity(sides.len());
        for index in 0..sides.len() {
            rotated.push(sides[(point + index) % sides.len()]);
        }
        let start = point_at(contour, point);
        return Ok(vec![SubPath { start, segments: rotated, closed: false }]);
    }
    if point == 0 || point >= count {
        return Err("an open contour may not be cut at its end".to_string());
    }
    let first = SubPath { start: contour.start, segments: contour.segments[..point].to_vec(), closed: false };
    let second = SubPath {
        start: point_at(contour, point),
        segments: contour.segments[point..].to_vec(),
        closed: false,
    };
    Ok(vec![first, second])
}

/// The **point** of a contour, counting from zero.
///
/// The first point is where the contour starts, and every other point is
/// where the side before it ends.
fn point_at(contour: &SubPath, index: usize) -> Point {
    match index {
        0 => contour.start,
        index => contour.segments.get(index - 1).map(|side| side.end()).unwrap_or(contour.start),
    }
}
