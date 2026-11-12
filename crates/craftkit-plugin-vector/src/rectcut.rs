//! The **rectangle cut**: a shape cut to a rectangle.
//!
//! The tree's Rectangle Cut crops the shapes it is given to a
//! rectangle: the part of the shape that is **inside** the rectangle
//! is kept, and the part that is outside is taken away. A shape that
//! lies wholly inside the rectangle is left as it was, and a shape that
//! lies wholly outside is taken away whole.
//!
//! The rules are the tree's own:
//!
//! - the rectangle is **four numbers** — a place and a width and a
//!   height — or **two corners**, and one malformed number is no
//!   rectangle at all;
//! - the rectangle must have a **width and a height**;
//! - the numbers must be **within** the tree's own bound;
//! - a shape that is **wholly inside** the rectangle is left as it
//!   was;
//! - a shape that is **wholly outside** it is taken away;
//! - what is left is the shape **cropped** to the rectangle, and the
//!   shape keeps its holes.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Outcome, Path, Point, Rect, Segment, SubPath, Text};

use craftkit_vector_geom as geom;

use crate::{vector_of, write_back};

/// The **largest** number a rectangle may hold, on either axis.
///
/// The tree's own: ten million.
const MAX_COORD: f64 = 1e7;

/// The **smallest** width or height a rectangle may have.
const MIN_SIDE: f64 = 1e-9;

commands! {
    /// Cut a closed shape to a rectangle.
    vector.path.cut.rect {
        area: "vector",
        layer: "vector",
        icon: "path",
        mutates,
        undoable,
        menu: ("Path", "Cut by rectangle"),
        params: {
            /// The path.

            path: NodeRef,
            /// The rectangle: `[x, y, width, height]`.

            rect: Option<Json>,
            /// A **corner** of the rectangle, when it is not given as four
            /// numbers.

            from: Option<Json>,
            /// The **other** corner.

            to: Option<Json>,
            /// What to call the shape that is left.

            name: Option<Text>,
        },
        run: {

            let id = "vector.path.cut.rect".to_string();
            let rect = read_rect(&self.rect, &self.from, &self.to)
                .map_err(|message| EngineError::Failed { id: id.clone(), message })?;
            let vector = vector_of(session, self.path.0)?;
            if vector.path.subpaths.iter().any(|contour| !contour.closed) {
                return Err(EngineError::Failed {
                    id,
                    message: "the rectangle cuts closed shapes, and this path has an open contour".to_string(),
                });
            }
            if vector.path.is_empty() {
                return Err(EngineError::Failed { id, message: "the path has no shape to cut".to_string() });
            }
            let bounds = geom::bounds(&vector.path);
            // A shape that lies **wholly inside** the rectangle is left as
            // it was.
            if rect.holds(bounds) {
                return Ok(Outcome::new("the shape is already inside the rectangle")
                    .with_data(json!({ "id": self.path.0 })));
            }
            let cropped = geom::boolean(&vector.path, &rect.path(), geom::BoolOp::Intersect)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error.to_string() })?;
            if cropped.is_empty() {
                // A shape that lies **wholly outside** the rectangle is
                // taken away whole.
                session.document_mut()?.remove_node(self.path.0);
                return Ok(
                    Outcome::changed("the shape is taken away").with_data(json!({ "removed": self.path.0 }))
                );
            }
            let mut vector = vector;
            vector.path = cropped;
            if let Some(name) = &self.name
                && let Some(node) = session.document_mut()?.node_mut(self.path.0)
            {
                node.name = name.0.clone();
            }
            write_back(session, self.path.0, &vector)?;
            Ok(Outcome::changed("the shape is cut to the rectangle").with_data(json!({ "id": self.path.0 })))

        }
    }
}

/// The **rectangle** the caller named.
struct Rectangle {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

impl Rectangle {
    /// Whether a **box** lies wholly inside the rectangle.
    fn holds(&self, bounds: Rect) -> bool {
        bounds.x >= self.x - MIN_SIDE
            && bounds.y >= self.y - MIN_SIDE
            && bounds.x + bounds.w <= self.x + self.w + MIN_SIDE
            && bounds.y + bounds.h <= self.y + self.h + MIN_SIDE
    }

    /// The rectangle as a **shape**: four sides and the corners.
    fn path(&self) -> Path {
        let corners = [
            Point::new(self.x, self.y),
            Point::new(self.x + self.w, self.y),
            Point::new(self.x + self.w, self.y + self.h),
            Point::new(self.x, self.y + self.h),
        ];
        let segments: Vec<Segment> = corners
            .windows(2)
            .map(|pair| Segment::Line { to: pair[1] })
            .chain(std::iter::once(Segment::Line { to: corners[0] }))
            .collect();
        Path::single(SubPath { start: corners[0], segments, closed: true })
    }
}

/// The rectangle, as the caller gave it: four numbers, or two corners.
fn read_rect(rect: &Option<Json>, from: &Option<Json>, to: &Option<Json>) -> Result<Rectangle, String> {
    if let Some(rect) = rect {
        let numbers = numbers_of(rect, 4).ok_or_else(|| {
            "the rectangle is [x, y, width, height], and all four numbers are needed".to_string()
        })?;
        let (x, y, w, h) = (numbers[0], numbers[1], numbers[2], numbers[3]);
        let rect = Rectangle { x, y, w: w.abs(), h: h.abs() };
        check(&rect)?;
        return Ok(rect);
    }
    match (from, to) {
        (Some(from), Some(to)) => {
            let first = read_point(from)?;
            let second = read_point(to)?;
            let rect = Rectangle {
                x: first.x.min(second.x),
                y: first.y.min(second.y),
                w: (second.x - first.x).abs(),
                h: (second.y - first.y).abs(),
            };
            check(&rect)?;
            Ok(rect)
        }
        _ => Err("the rectangle is [x, y, width, height], or two corners".to_string()),
    }
}

/// A rectangle that **may** be cut with.
fn check(rect: &Rectangle) -> Result<(), String> {
    let numbers = [rect.x, rect.y, rect.w, rect.h];
    if !numbers.iter().all(|value| value.is_finite() && value.abs() <= MAX_COORD) {
        return Err("the rectangle's numbers are not within the bound".to_string());
    }
    if rect.w <= MIN_SIDE || rect.h <= MIN_SIDE {
        return Err("the rectangle must have a width and a height".to_string());
    }
    Ok(())
}

/// The **numbers** of a list, when there are exactly as many as are
/// wanted and every one of them is a number.
fn numbers_of(value: &Json, count: usize) -> Option<Vec<f64>> {
    let list = value.as_array()?;
    if list.len() != count {
        return None;
    }
    list.iter().map(|item| item.as_f64()).collect()
}

/// A **point**, as the caller gave it.
fn read_point(value: &Json) -> Result<Point, String> {
    let (x, y) = match value {
        Json::Array(pair) => {
            (pair.first().and_then(|value| value.as_f64()), pair.get(1).and_then(|value| value.as_f64()))
        }
        Json::Object(table) => {
            (table.get("x").and_then(|value| value.as_f64()), table.get("y").and_then(|value| value.as_f64()))
        }
        _ => return Err("the corner is not a point".to_string()),
    };
    match (x, y) {
        (Some(x), Some(y)) if x.is_finite() && y.is_finite() => Ok(Point::new(x, y)),
        _ => Err("the corner has no numbers in it".to_string()),
    }
}
