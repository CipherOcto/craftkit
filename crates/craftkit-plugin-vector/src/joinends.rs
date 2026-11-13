//! The **joining** of two open contours.
//!
//! The tree's Join joins two open paths into one: the ends that are
//! joined meet, and the joined point may be made a **corner** or
//! **smooth**.
//!
//! The rules are the tree's own:
//!
//! - the two contours must be **open**;
//! - the ends that are joined are the ones **named**, or the **nearest**
//!   pair when none are named;
//! - when the two ends are already **one place**, the curves are joined
//!   without a jump: the handle that leaves the join is the one that
//!   left the second contour;
//! - when they are two places, a **straight side** joins them;
//! - the joined point may be a **corner** — its handles come back to
//!   it — or **smooth**, by the tree's **neighbours'** rule.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Point, Segment, SubPath, Text};

use crate::convert::smooth_by_neighbours;
use crate::{reverse_subpath, vector_of, write_back};

/// How close two points must be to be **one place**.
pub(crate) const SAME_PLACE: f64 = 1e-6;

commands! {
    /// Join two open contours of a path into one.
    vector.path.join.ends {
        area: "vector",
        layer: "vector",
        icon: "path",
        mutates,
        undoable,
        menu: ("Path", "Points"),
        params: {
            /// The path.
            path: NodeRef,
            /// Which contour of the path, counting from zero.
            first: Number,
            /// The other contour, counting from zero.
            second: Number,
            /// Which end of each contour is joined: `["first"|"last", "first"|"last"]`.
            ///
            /// The **nearest** pair when it is left out.
            ends: Option<Json>,
            /// The joined point: `corner` or `smooth`.
            to: Option<Text>,
        },
        run: {
            let id = "vector.path.join.ends".to_string();
            let kind = match self.to.as_ref().map(|to| to.0.as_str()) {
                None | Some("corner") => Some(false),
                Some("smooth") => Some(true),
                Some(other) => {
                    return Err(EngineError::Failed {
                        id,
                        message: format!("the joined point is `corner` or `smooth`, not `{other}`"),
                    });
                }
            };
            let first = crate::index_of(Some(self.first), "the first contour")
                .map_err(|reason| EngineError::Failed { id: id.clone(), message: reason })?;
            let second = crate::index_of(Some(self.second), "the second contour")
                .map_err(|reason| EngineError::Failed { id: id.clone(), message: reason })?;
            if first == second {
                return Err(EngineError::Failed {
                    id,
                    message: "a contour cannot be joined to itself: `vector.path.close` closes one".to_string(),
                });
            }
            let mut vector = vector_of(session, self.path.0)?;
            if vector.path.subpaths.len() <= first || vector.path.subpaths.len() <= second {
                return Err(EngineError::Failed {
                    id,
                    message: format!(
                        "the path has {} contours and not a contour {first} and a contour {second}",
                        vector.path.subpaths.len()
                    ),
                });
            }
            let (one, two) = (vector.path.subpaths[first].clone(), vector.path.subpaths[second].clone());
            if one.closed || two.closed {
                return Err(EngineError::Failed { id, message: "the join needs open paths".to_string() });
            }
            // The ends that are joined: the ones named, or the nearest pair.
            let (reverse_one, reverse_two) = match &self.ends {
                Some(ends) => {
                    read_ends(ends).map_err(|message| EngineError::Failed { id: id.clone(), message })?
                }
                None => nearest_ends(&one, &two),
            };
            let joined = join_contours(&one, &two, reverse_one, reverse_two, kind);
            // The joined contour takes the place of the first, and the
            // second is gone.
            let mut subpaths = vector.path.subpaths.clone();
            if first < second {
                subpaths[first] = joined;
                subpaths.remove(second);
            } else {
                subpaths.remove(second);
                subpaths[first - 1] = joined;
            }
            vector.path.subpaths = subpaths;
            write_back(session, self.path.0, &vector)?;
            Ok(Outcome::changed("the paths are one now")
                .with_data(json!({ "id": self.path.0, "contour": first.min(second) })))

        }
    }
}

/// The contour that is two contours **joined**: the ends that meet,
/// and the joined point's kind.
///
/// The tree's own rules: when the two ends are already **one place**
/// the curves are joined without a jump, and when they are two places
/// a **straight side** joins them.
pub(crate) fn join_contours(
    one: &SubPath,
    two: &SubPath,
    reverse_one: bool,
    reverse_two: bool,
    kind: Option<bool>,
) -> SubPath {
    let one = if reverse_one { reverse_subpath(one) } else { one.clone() };
    let two = if reverse_two { reverse_subpath(two) } else { two.clone() };
    let here = one.end();
    let there = two.start;
    let mut segments: Vec<Segment> = one.segments.clone();
    let junction = segments.len();
    if distance(here, there) < SAME_PLACE {
        segments.extend(two.segments.clone());
    } else {
        segments.push(Segment::Line { to: there });
        segments.extend(two.segments.clone());
    }
    let mut joined = SubPath { start: one.start, segments, closed: false };
    if let Some(smooth) = kind {
        if smooth {
            smooth_by_neighbours(&mut joined, junction);
        } else {
            make_corner(&mut joined, junction, here);
        }
    }
    joined
}

/// The joined point is a **corner**: its handles come back to it.
fn make_corner(subpath: &mut SubPath, point: usize, at: Point) {
    if point > 0 {
        let index = point - 1;
        let end = subpath.segments[index].end();
        let c2 = match &subpath.segments[index] {
            Segment::Line { .. } => end,
            Segment::Cubic { c2, .. } => *c2,
        };
        subpath.segments[index] = Segment::Cubic { c1: at, c2, to: end };
    }
    if point < subpath.segments.len() {
        let index = point;
        let end = subpath.segments[index].end();
        let c2 = match &subpath.segments[index] {
            Segment::Line { .. } => end,
            Segment::Cubic { c2, .. } => *c2,
        };
        subpath.segments[index] = Segment::Cubic { c1: at, c2, to: end };
    }
}

/// The ends that are joined: the ones **named**.
fn read_ends(value: &Json) -> Result<(bool, bool), String> {
    let Some(list) = value.as_array() else {
        return Err("the ends are two of `first` and `last`".to_string());
    };
    if list.len() != 2 {
        return Err("the ends are two of `first` and `last`".to_string());
    }
    let first = list[0].as_str().unwrap_or_default();
    let second = list[1].as_str().unwrap_or_default();
    if !matches!(first, "first" | "last") || !matches!(second, "first" | "last") {
        return Err("the ends are two of `first` and `last`".to_string());
    }
    Ok((first == "first", second == "last"))
}

/// The **nearest** pair of ends: the tree's own rule.
fn nearest_ends(one: &SubPath, two: &SubPath) -> (bool, bool) {
    let candidates = [
        (distance(one.end(), two.start), false, false),
        (distance(one.end(), two.end()), false, true),
        (distance(one.start, two.start), true, false),
        (distance(one.start, two.end()), true, true),
    ];
    candidates
        .iter()
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, reverse_one, reverse_two)| (*reverse_one, *reverse_two))
        .unwrap_or((false, false))
}

/// How far two points are **apart**.
pub(crate) fn distance(a: Point, b: Point) -> f64 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}
