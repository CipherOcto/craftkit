//! The **anchors** of a path.
//!
//! The tree's path is a run of anchors, and each anchor is a point and
//! two handles. This module is the port of that: the reader of the
//! anchors, the contours the anchors make, and the command that sets
//! the anchors of a path that is already there.
//!
//! The rules are the tree's own:
//!
//! - an anchor is a point and two handles, and a handle that is not
//!   given — or given as `null` — is the point itself;
//! - a side whose two handles are both their points is a **line**, and
//!   any other side is a **curve** through them;
//! - a **closed** contour has one more side than it has anchors: the
//!   side that comes back to the first anchor;
//! - anything **malformed** in an anchor is refused rather than left
//!   out or given a default, because a default handle changes the
//!   shape of the path.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Outcome, Path, Point, Segment, SubPath};

use crate::{closing_segment, vector_of, write_back};

/// The most **anchors** one path may hold.
///
/// The bound is this battery's: a path of a hundred thousand anchors is
/// a document nobody can open, and a caller that wants one says so and
/// is refused by name.
const MAX_ANCHORS: usize = 10_000;

/// An **anchor** of a path: a point and its two handles.
pub(crate) struct Anchor {
    /// Where the anchor is.
    pub(crate) point: Point,
    /// The handle the curve that arrives at the anchor comes by.
    pub(crate) h_in: Point,
    /// The handle the curve that leaves the anchor goes by.
    pub(crate) h_out: Point,
}

/// The **anchors** of a path, as the caller gave them.
///
/// The count of the anchors already in the path is given, so that the
/// bound is on the path and not on one list.
pub(crate) fn read_anchors(value: &Json, used: usize) -> Result<Vec<Anchor>, String> {
    let Some(list) = value.as_array() else {
        return Err("the anchors are not a list".to_string());
    };
    if used + list.len() > MAX_ANCHORS {
        return Err(format!("a path may have at most {MAX_ANCHORS} anchors"));
    }
    let mut anchors: Vec<Anchor> = Vec::with_capacity(list.len());
    for (index, item) in list.iter().enumerate() {
        anchors.push(read_anchor(item, index)?);
    }
    Ok(anchors)
}

/// One anchor, as the caller gave it.
///
/// An anchor is `{x, y}`, with an optional `in` and an optional `out`
/// handle, each a pair of numbers. Anything malformed is **refused**:
/// an anchor that was left out or given a default would change the
/// shape of the path, and the tree's own rule says the same thing.
pub(crate) fn read_anchor(value: &Json, index: usize) -> Result<Anchor, String> {
    let Some(table) = value.as_object() else {
        return Err(format!("the anchor {index} is not an object of `x` and `y`"));
    };
    let point = Point::new(number(table.get("x"), index, "x")?, number(table.get("y"), index, "y")?);
    let h_in = handle(table.get("in"), point, index, "in")?;
    let h_out = handle(table.get("out"), point, index, "out")?;
    // The tree keeps an anchor's **kind** — smooth or corner — and this
    // tree has nowhere to keep it: a path here is a run of segments, and
    // a segment says nothing about the point it leaves. A corner is the
    // default and is said by saying nothing; a **smooth** anchor is
    // refused rather than quietly forgotten, and the command that makes
    // a point smooth is the way to have one.
    match table.get("smooth") {
        None | Some(Json::Bool(false)) => {}
        Some(Json::Bool(true)) => {
            return Err(format!(
                "the anchor {index} is smooth, and a path here does not keep an anchor's kind: \
                 make the point smooth with `vector.path.smooth` instead"
            ));
        }
        Some(_) => return Err(format!("the anchor {index} has a `smooth` that is not true or false")),
    }
    Ok(Anchor { point, h_in, h_out })
}

/// The **contour** the anchors make.
///
/// A side with no handle on either end is a straight line, and any
/// other side is a curve through the two handles. A closed contour has
/// one more side than it has anchors: the side that comes back to the
/// first anchor.
pub(crate) fn contour(anchors: &[Anchor], closed: bool) -> SubPath {
    let count = anchors.len();
    let sides = if closed { count } else { count.saturating_sub(1) };
    let mut segments: Vec<Segment> = Vec::with_capacity(sides);
    for index in 0..sides {
        let from = &anchors[index % count];
        let to = &anchors[(index + 1) % count];
        if from.h_out == from.point && to.h_in == to.point {
            segments.push(Segment::Line { to: to.point });
        } else {
            segments.push(Segment::Cubic { c1: from.h_out, c2: to.h_in, to: to.point });
        }
    }
    SubPath { start: anchors.first().map(|anchor| anchor.point).unwrap_or_default(), segments, closed }
}

/// One **contour**, as the caller gave it.
///
/// A contour is a list of anchors and whether it is closed. The count
/// of the anchors comes back with it, so that the bound is on the path
/// and not on one contour.
pub(crate) fn read_contour(value: &Json, used: usize) -> Result<(SubPath, usize), String> {
    let Some(table) = value.as_object() else {
        return Err("a contour is not an object of `anchors` and `closed`".to_string());
    };
    let Some(anchors) = table.get("anchors") else {
        return Err("a contour needs an `anchors` list".to_string());
    };
    let anchors = read_anchors(anchors, used)?;
    if anchors.len() < 2 {
        return Err("a contour needs at least two anchors".to_string());
    }
    let closed = match table.get("closed") {
        None | Some(Json::Null) => false,
        Some(Json::Bool(flag)) => *flag,
        Some(_) => return Err("a contour's `closed` must be true or false".to_string()),
    };
    let count = anchors.len();
    Ok((contour(&anchors, closed), count))
}

commands! {
    /// Set the **anchors** of a path.
    ///
    /// The path's contours are the ones the anchors describe: the tree's
    /// own command, and the tree's own rules about the anchors.
    vector.path.anchors.set {
        area: "vector",
        layer: "vector",
        icon: "path",
        mutates,
        undoable,
        menu: ("Path", "Anchors"),
        params: {
            /// The path.

            path: NodeRef,
            /// The contours of the path: each a list of anchors and whether it
            /// is closed.

            subpaths: Json,
        },
        run: {

            let id = "vector.path.anchors.set".to_string();
            let Some(list) = self.subpaths.as_array() else {
                return Err(EngineError::Failed { id, message: "the subpaths are not a list".to_string() });
            };
            if list.is_empty() {
                return Err(EngineError::Failed { id, message: "a path needs at least one contour".to_string() });
            }
            let mut subpaths: Vec<SubPath> = Vec::with_capacity(list.len());
            let mut used = 0usize;
            for (index, item) in list.iter().enumerate() {
                let (contour, count) = read_contour(item, used).map_err(|message| EngineError::Failed {
                    id: id.clone(),
                    message: format!("the contour {index}: {message}"),
                })?;
                used += count;
                subpaths.push(contour);
            }
            // The anchors are the whole of the path's shape, and the **fill
            // rule** is not part of them: it does not change.
            let mut vector = vector_of(session, self.path.0)?;
            vector.path = Path { subpaths, rule: vector.path.rule };
            write_back(session, self.path.0, &vector)?;
            Ok(Outcome::changed(format!("the anchors of `{}` are set", self.path.0))
                .with_data(json!({ "id": self.path.0 })))

        }
    }
    /// Put a point in the **middle of every side** of a path.
    ///
    /// The tree's own command: an anchor is added at the middle of each
    /// side, so that the path may be edited in finer pieces. The sides
    /// keep their shape — the split is the tree's own, and the curve is
    /// the same curve.
    vector.path.anchors.middle {
        area: "vector",
        layer: "vector",
        icon: "path",
        mutates,
        undoable,
        menu: ("Path", "Anchors"),
        params: {
            /// The path.

            path: NodeRef,
        },
        run: {

            let id = "vector.path.anchors.middle".to_string();
            let mut vector = vector_of(session, self.path.0)?;
            let mut subpaths: Vec<SubPath> = Vec::with_capacity(vector.path.subpaths.len());
            for contour in &vector.path.subpaths {
                let mut contour = contour.clone();
                // A closed contour may leave its **closing edge** out, and
                // the tree's closed contour has it in: the edge is a side
                // of its own here too, and it is split like the others.
                if contour.closed && closing_segment(&contour).is_none() {
                    contour.segments.push(Segment::Line { to: contour.start });
                }
                // Every side is split in the middle, from the last to the
                // first, so that the sides that are not split yet keep
                // their places.
                for side in (0..contour.segments.len()).rev() {
                    contour = craftkit_vector_geom::split_segment(&contour, side, 0.5)
                        .map_err(|error| EngineError::Failed { id: id.clone(), message: error.to_string() })?;
                }
                subpaths.push(contour);
            }
            vector.path = Path { subpaths, rule: vector.path.rule };
            write_back(session, self.path.0, &vector)?;
            Ok(Outcome::changed("the path has a point in the middle of every side")
                .with_data(json!({ "id": self.path.0 })))

        }
    }
}

/// A handle of an anchor.
///
/// A handle that is not given, or given as `null`, is the anchor's own
/// point: the tree's own rule, and a handle that is the point says
/// "there is no handle here".
fn handle(value: Option<&Json>, point: Point, index: usize, name: &str) -> Result<Point, String> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(point);
    };
    let Some(pair) = value.as_array() else {
        return Err(format!("the anchor {index} has a `{name}` that is not a pair of numbers"));
    };
    if pair.len() != 2 {
        return Err(format!("the anchor {index} has a `{name}` that is not a pair of numbers"));
    }
    let x = pair[0].as_f64().filter(|number| number.is_finite());
    let y = pair[1].as_f64().filter(|number| number.is_finite());
    match (x, y) {
        (Some(x), Some(y)) => Ok(Point::new(x, y)),
        _ => Err(format!("the anchor {index} has a `{name}` that is not a pair of numbers")),
    }
}

/// A number of an anchor.
///
/// A number that is missing, or that is not a number, or that is not
/// finite, is refused: the tree's own rule about numbers that come in
/// from outside.
fn number(value: Option<&Json>, index: usize, name: &str) -> Result<f64, String> {
    match value.and_then(|value| value.as_f64()) {
        Some(number) if number.is_finite() => Ok(number),
        _ => Err(format!("the anchor {index} has a `{name}` that is not a number")),
    }
}
