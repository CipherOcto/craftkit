//! The **stroke drawn by hand**.
//!
//! The tree's Pencil draws a stroke by hand and **fits** it to the
//! samples: the jitter is dropped, the sharp turns stay corners, and the
//! rest is a spline through the samples that is then simplified.
//!
//! The rules are the tree's own:
//!
//! - a sample closer than a **third of the fidelity** to the last one
//!   kept is dropped — the jitter — and the **last sample always ends
//!   the stroke**;
//! - a turn of **more than a hundred degrees** is a **corner**, and a
//!   corner has no handles;
//! - the stroke is a **spline through the samples** — the Catmull-Rom
//!   one: the handles at a point are a sixth of the way to the point
//!   before it and the point after it;
//! - the spline is **simplified** at the fidelity.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, Number, Outcome, Path, Point, Session, Text};

use crate::add_path;
use crate::anchors::Anchor;

/// The least **fidelity** a stroke may be fitted at.
const MIN_FIDELITY: f64 = 0.01;

/// The most **fidelity** a stroke may be fitted at.
const MAX_FIDELITY: f64 = 100.0;

/// The **fidelity** a stroke is fitted at when the caller names none.
const DEFAULT_FIDELITY: f64 = 1.5;

/// Draw a **stroke by hand** in the document.
///
/// The stroke is the tree's own: the samples are fitted, the jitter is
/// dropped, the sharp turns stay corners, and the result is simplified.
#[derive(Command)]
#[command(
    id = "vector.path.freehand",
    area = "vector",
    layer = "vector",
    icon = "path",
    menu("Create"),
    mutates,
    undoable
)]
pub struct Freehand {
    /// The samples of the stroke: `[[x, y], …]` or `[{x, y}, …]`.
    pub points: Json,
    /// The **fidelity** the stroke is fitted at: how far the stroke may
    /// be from the samples. 1.5 if it is not given.
    pub fidelity: Option<Number>,
    /// Whether the stroke is fitted at all.
    ///
    /// It is, unless this says otherwise: a stroke that is not fitted
    /// is the samples joined by straight sides.
    pub fit: Option<Bool>,
    /// Whether the stroke is closed.
    pub closed: Option<Bool>,
    /// What to call it.
    pub name: Option<Text>,
    /// The colour it is filled with, as `#rrggbb`.
    pub fill: Option<Text>,
}

impl Freehand {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "vector.path.freehand".to_string();
        let fidelity = self.fidelity.as_ref().map(|fidelity| fidelity.0).unwrap_or(DEFAULT_FIDELITY);
        if !fidelity.is_finite() || fidelity < MIN_FIDELITY || fidelity > MAX_FIDELITY {
            return Err(EngineError::Failed {
                id,
                message: format!(
                    "a fidelity is a number between {MIN_FIDELITY} and {MAX_FIDELITY}, and \
                     the stroke is fitted at it"
                ),
            });
        }
        let samples =
            read_samples(&self.points).map_err(|message| EngineError::Failed { id: id.clone(), message })?;
        let closed = self.closed.map(|closed| closed.0).unwrap_or(false);
        let fit = self.fit.map(|fit| fit.0).unwrap_or(true);
        let path = stroke(&samples, fidelity, closed, fit);
        if path.is_empty() {
            return Err(EngineError::Failed { id, message: "the stroke has nothing in it".to_string() });
        }
        // The tree's own name for a stroke drawn by hand.
        let name = match &self.name {
            Some(name) => name.0.clone(),
            None => "Pencil".to_string(),
        };
        let node = add_path(session, name, path, self.fill.as_ref())?;
        Ok(Outcome::changed(format!("added a stroke `{node}`")).with_data(json!({ "id": node })))
    }
}

/// The **samples** of a stroke, as the caller gave them.
///
/// A sample is `[x, y]` or `{x, y}`. The tree's Pencil also takes the
/// pen's **pressure** as a third number, and keeps it on the stroke;
/// this tree has nowhere to keep it, so a sample that carries one is
/// refused rather than quietly losing it.
fn read_samples(value: &Json) -> Result<Vec<Point>, String> {
    let Some(list) = value.as_array() else {
        return Err("the samples are not a list".to_string());
    };
    let mut samples: Vec<Point> = Vec::with_capacity(list.len());
    for (index, item) in list.iter().enumerate() {
        let (x, y, pressure) = match item {
            Json::Array(pair) => {
                let x = pair.first().and_then(|value| value.as_f64());
                let y = pair.get(1).and_then(|value| value.as_f64());
                let pressure = pair.get(2);
                (x, y, pressure)
            }
            Json::Object(table) => {
                let x = table.get("x").and_then(|value| value.as_f64());
                let y = table.get("y").and_then(|value| value.as_f64());
                let pressure = table.get("pressure");
                (x, y, pressure)
            }
            _ => return Err(format!("the sample {index} is not a point")),
        };
        let (Some(x), Some(y)) = (x, y) else {
            return Err(format!("the sample {index} has no numbers in it"));
        };
        if !x.is_finite() || !y.is_finite() {
            return Err(format!("the sample {index} has no numbers in it"));
        }
        if pressure.is_some() {
            return Err(format!(
                "the sample {index} carries the pen's pressure, and a stroke here keeps none: \
                 a stroke is fitted to its samples and nothing else"
            ));
        }
        samples.push(Point::new(x, y));
    }
    Ok(samples)
}

/// The **stroke** the samples make.
///
/// The tree's own rules: the jitter is dropped, the sharp turns stay
/// corners, the rest is a spline through the samples, and the spline
/// is simplified at the fidelity.
fn stroke(samples: &[Point], fidelity: f64, closed: bool, fit: bool) -> Path {
    if !fit {
        // The samples joined by straight sides.
        let anchors: Vec<Anchor> =
            samples.iter().map(|point| Anchor { point: *point, h_in: *point, h_out: *point }).collect();
        return Path::single(crate::anchors::contour(&anchors, closed));
    }
    let kept = keep_samples(samples, fidelity);
    let anchors = spline(&marked(&kept), closed);
    let path = Path::single(crate::anchors::contour(&anchors, closed && kept.len() > 2));
    // The spline is simplified at the fidelity: the points a flat line
    // would serve are taken away. A simplification that leaves nothing
    // is not used, and the spline is kept as it was.
    let simplified = craftkit_vector_geom::simplify(&path, fidelity);
    let keeps = simplified.subpaths.first().map(|contour| !contour.segments.is_empty()).unwrap_or(false);
    if keeps { simplified } else { path }
}

/// The samples that are **kept**: the jitter is dropped.
///
/// A sample closer than a third of the fidelity to the last one kept is
/// dropped, and the last sample always ends the stroke.
fn keep_samples(samples: &[Point], fidelity: f64) -> Vec<Point> {
    let min = (fidelity * 0.3).max(MIN_FIDELITY * 0.3);
    let (Some(first), Some(last)) = (samples.first(), samples.last()) else {
        return Vec::new();
    };
    let mut kept: Vec<Point> = vec![*first];
    for sample in samples.iter().skip(1) {
        if kept.last().is_some_and(|kept| distance(*kept, *sample) >= min) {
            kept.push(*sample);
        }
    }
    if kept.len() > 1 {
        if let Some(kept) = kept.last_mut() {
            *kept = *last;
        }
    } else if samples.len() > 1 {
        kept.push(*last);
    }
    kept
}

/// The samples, each with whether it is a **corner**.
///
/// A turn of more than a hundred degrees is a corner, and a corner has
/// no handles.
fn marked(samples: &[Point]) -> Vec<(Point, bool)> {
    let count = samples.len();
    (0..count)
        .map(|index| {
            let point = samples[index];
            let corner = if index == 0 || index + 1 == count {
                false
            } else {
                let before = samples[index - 1];
                let after = samples[index + 1];
                let (ax, ay) = (point.x - before.x, point.y - before.y);
                let (bx, by) = (after.x - point.x, after.y - point.y);
                let dot = ax * bx + ay * by;
                let lengths = (ax * ax + ay * ay).sqrt() * (bx * bx + by * by).sqrt();
                dot / lengths.max(1e-12) < -0.2
            };
            (point, corner)
        })
        .collect()
}

/// The **spline through the samples**: the Catmull-Rom one.
///
/// The handles at a point are a sixth of the way to the point before it
/// and the point after it, and a corner or an end of an open stroke has
/// no handles at all.
fn spline(samples: &[(Point, bool)], closed: bool) -> Vec<Anchor> {
    let count = samples.len();
    let mut anchors: Vec<Anchor> = Vec::with_capacity(count);
    for index in 0..count {
        let (point, corner) = samples[index];
        let ends = !closed && (index == 0 || index + 1 == count);
        if corner || ends || count < 3 {
            anchors.push(Anchor { point, h_in: point, h_out: point });
            continue;
        }
        let previous = samples[(index + count - 1) % count].0;
        let next = samples[(index + 1) % count].0;
        let tangent = Point::new((next.x - previous.x) / 6.0, (next.y - previous.y) / 6.0);
        if (tangent.x * tangent.x + tangent.y * tangent.y).sqrt() < 1e-9 {
            anchors.push(Anchor { point, h_in: point, h_out: point });
            continue;
        }
        anchors.push(Anchor {
            point,
            h_in: Point::new(point.x - tangent.x, point.y - tangent.y),
            h_out: Point::new(point.x + tangent.x, point.y + tangent.y),
        });
    }
    anchors
}

/// How far two points are apart.
fn distance(one: Point, other: Point) -> f64 {
    (one.x - other.x).hypot(one.y - other.y)
}
