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

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, Outcome, Path, Point, Segment, Session, SubPath, Text};

use crate::add_path;
use crate::svgimport::read_path;

/// The most **anchors** one path may hold.
///
/// The tree's own bound is for reading a file, and it is very large; a
/// path made by hand is not read from a file, and the bound here is the
/// one that keeps a command from making a document nobody can open.
const MAX_ANCHORS: usize = 10_000;

/// Put a path in the document, made by hand.
///
/// The path is made one of two ways: the **path data** says what the
/// path is, or the **anchors** do. The tree's own command takes either,
/// and this one does the same.
#[derive(Command)]
#[command(
    id = "vector.path.create",
    area = "vector",
    layer = "vector",
    icon = "path",
    menu("Create"),
    mutates,
    undoable
)]
pub struct CreatePath {
    /// The path data, in the SVG spelling: the moves, the lines and the
    /// curves of the path. An **arc** is refused.
    pub d: Option<Text>,
    /// The anchors of the path: a list of `{x, y}`, each with an
    /// optional `in` and `out` handle, each a pair of numbers.
    pub anchors: Option<Json>,
    /// Whether the path is closed.
    ///
    /// It says so for the **anchors**; with path data the data itself
    /// says whether the path is closed, as it does in the tree.
    pub closed: Option<Bool>,
    /// What to call it.
    pub name: Option<Text>,
    /// The colour it is filled with, as `#rrggbb`.
    pub fill: Option<Text>,
}

impl CreatePath {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
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

/// An **anchor** of a path: a point and its two handles.
struct Anchor {
    /// Where the anchor is.
    point: Point,
    /// The handle the curve that arrives at the anchor leaves by.
    h_in: Point,
    /// The handle the curve that leaves the anchor goes by.
    h_out: Point,
}

/// The path a list of anchors holds.
///
/// The anchors are the tree's own: a point, and a handle on each side
/// of it, and a handle that is not given is the point itself. The
/// segments between them are the tree's own too: a side whose two
/// handles are both their points is a **line**, and any other side is a
/// **curve** through them.
///
/// A **closed** path has one more side than it has anchors: the side
/// that comes back to the first anchor, which is what the tree's own
/// closed contour is.
fn path_from_anchors(value: &Json, closed: bool) -> Result<Path, String> {
    let Some(list) = value.as_array() else {
        return Err("the anchors are not a list".to_string());
    };
    if list.is_empty() {
        return Err("a path needs at least one anchor".to_string());
    }
    if list.len() > MAX_ANCHORS {
        return Err(format!("a path may have at most {MAX_ANCHORS} anchors"));
    }
    let mut anchors: Vec<Anchor> = Vec::with_capacity(list.len());
    for (index, item) in list.iter().enumerate() {
        anchors.push(anchor_from_json(item, index)?);
    }
    if anchors.len() < 2 {
        return Err("a path needs at least two anchors".to_string());
    }
    let count = if closed { anchors.len() } else { anchors.len() - 1 };
    let mut segments: Vec<Segment> = Vec::with_capacity(count);
    for index in 0..count {
        let from = &anchors[index % anchors.len()];
        let to = &anchors[(index + 1) % anchors.len()];
        // A side with no handle on either end is a straight line.
        if from.h_out == from.point && to.h_in == to.point {
            segments.push(Segment::Line { to: to.point });
        } else {
            segments.push(Segment::Cubic { c1: from.h_out, c2: to.h_in, to: to.point });
        }
    }
    Ok(Path::single(SubPath { start: anchors[0].point, segments, closed }))
}

/// One anchor, as the caller gave it.
///
/// An anchor is `{x, y}`, with an optional `in` and an optional `out`
/// handle, each a pair of numbers. Anything malformed is **refused**:
/// an anchor that was left out or given a default would change the
/// shape of the path, and the tree's own rule says the same thing.
fn anchor_from_json(value: &Json, index: usize) -> Result<Anchor, String> {
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
