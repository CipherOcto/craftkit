//! The **parts** of a shape: an outer contour and the holes in it.
//!
//! The tree's `components` takes a path that a boolean has left and
//! says how many **shapes** are in it: a contour inside another one is
//! a **hole** in it, and the hole travels with the shape it is in.
//!
//! The rules are the tree's own:
//!
//! - the **depth** of a contour is how many other contours hold it;
//! - a contour of an **even** depth is a shape, and one of an **odd**
//!   depth is a hole in the shape above it;
//! - a hole belongs to the **smallest** shape that holds it and is one
//!   level above it;
//! - a path of one contour is one part, and a path of none is no parts.

use craftkit_sdk::{Path, Point, SubPath};

use craftkit_vector_geom as geom;

/// The **tolerance** the contours are flattened with, to ask whether
/// one is inside another.
const FLAT_TOLERANCE: f64 = 0.05;

/// The parts of a path, one shape each: an outer contour and the holes
/// in it.
pub fn components(path: &Path) -> Vec<Path> {
    let closed: Vec<&SubPath> =
        path.subpaths.iter().filter(|sub| sub.closed && !sub.segments.is_empty()).collect();
    if closed.len() <= 1 {
        return if path.is_empty() { Vec::new() } else { vec![path.clone()] };
    }
    let flats: Vec<Vec<Point>> = closed.iter().map(|sub| geom::flatten(sub, FLAT_TOLERANCE)).collect();
    let samples: Vec<Point> = flats.iter().map(|flat| sample_of(flat)).collect();
    let count = closed.len();
    let holds = |outer: usize, inner: usize| winding(&flats[outer], samples[inner]) != 0;
    let depth: Vec<usize> =
        (0..count).map(|i| (0..count).filter(|&j| j != i && holds(j, i)).count()).collect();
    let areas: Vec<f64> = closed.iter().map(|sub| geom::area(&Path::single((*sub).clone())).abs()).collect();
    let outers: Vec<usize> = (0..count).filter(|&i| depth[i].is_multiple_of(2)).collect();
    let mut groups: Vec<Vec<SubPath>> = outers.iter().map(|&outer| vec![closed[outer].clone()]).collect();
    for hole in (0..count).filter(|&i| !depth[i].is_multiple_of(2)) {
        let best = outers
            .iter()
            .copied()
            .enumerate()
            .filter(|&(_, outer)| depth[outer] + 1 == depth[hole] && holds(outer, hole))
            .min_by(|a, b| areas[a.1].total_cmp(&areas[b.1]));
        if let Some((index, _)) = best {
            groups[index].push(closed[hole].clone());
        }
    }
    groups.into_iter().map(|subpaths| Path { subpaths, rule: path.rule }).collect()
}

/// A **point** of a contour, to ask about: the middle of its first
/// side.
fn sample_of(flat: &[Point]) -> Point {
    match (flat.first(), flat.get(1)) {
        (Some(first), Some(second)) => Point::new((first.x + second.x) / 2.0, (first.y + second.y) / 2.0),
        (Some(first), None) => *first,
        _ => Point::new(0.0, 0.0),
    }
}

/// How many times a **polygon** goes round a point.
///
/// The rule of a winding number: an edge that passes under the point
/// and turns left adds one, and an edge that passes over it and turns
/// right takes one away. A point inside a contour is inside it when the
/// number is not zero — the tree's own rule.
fn winding(flat: &[Point], point: Point) -> i32 {
    let count = flat.len();
    if count < 3 {
        return 0;
    }
    let mut total = 0;
    for index in 0..count {
        let a = flat[index];
        let b = flat[(index + 1) % count];
        let turn = (b.x - a.x) * (point.y - a.y) - (b.y - a.y) * (point.x - a.x);
        if a.y <= point.y {
            if b.y > point.y && turn > 0.0 {
                total += 1;
            }
        } else if b.y <= point.y && turn < 0.0 {
            total -= 1;
        }
    }
    total
}
