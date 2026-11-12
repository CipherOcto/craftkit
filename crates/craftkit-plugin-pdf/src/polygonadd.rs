//! The **polygon** on a page: the tree's own polygon and its connected
//! lines.
//!
//! The tree's own: a **polygon** is an annotation whose **vertices**
//! are the corners of a shape, and a **connected line** — the tree's
//! `PolyLine` — is the same thing left open. The border of the tree's
//! polygon may be **cloudy**, and the cloud is the tree's own: the
//! intent says `PolygonCloud` and the border says `/BE /S /C`.
//!
//! # The rules
//!
//! - The **kind** is one of the tree's own two: `polygon` and
//!   `polyline`, and a kind that is not one of them is **refused**.
//! - The **vertices** are points, and each point is two numbers, and a
//!   number that is not a number is **refused**.
//! - A **polygon** needs **three** points and a **connected line**
//!   needs **two**: the tree's own rules, and a shape with fewer
//!   corners is not a shape.
//! - The **bounds** of the vertices must be at least a point wide and a
//!   point high for a polygon, and the larger of the two at least a
//!   point for a connected line: the tree's own rules.
//! - The **width** of the line is the tree's own two points when the
//!   caller says nothing, and a width below zero is the tree's own
//!   zero.
//! - The **colour** is the tree's own red when the caller says
//!   nothing, and it is three numbers between zero and one.
//! - The **fill** of a polygon is the colour of the fill when the
//!   caller says one, and there is none when the caller says nothing.
//! - The **rectangle** of the annotation is the **bounds** of the
//!   vertices, grown by half the width of the line and by one point
//!   more — and by one and a half **cloud radii** when the border is
//!   cloudy, the tree's own rectangle.
//! - The **radius of a cloud** is six points plus one and a half times
//!   the width of the line: the tree's own `cloud_radius`.
//! - A **cloud** is a polygon only: a connected line with a cloudy
//!   border is **refused**, the tree's own rule.
//!
//! # What it does not settle
//!
//! - The **drawing** of the polygon is not here: it is the appearance
//!   command's own work.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, NodeRef, Number, Outcome, Session, Text};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.annot.polygon.add";

/// The tree's own **width** of a line: two points.
const WIDTH: f64 = 2.0;

/// The tree's own **colour** of a line: the red.
const COLOUR: [f64; 3] = [0.89, 0.13, 0.13];

/// The **radius of a cloud**: the tree's own `cloud_radius` — six
/// points plus one and a half times the width of the line.
fn the_cloud_radius(width: f64) -> f64 {
    6.0 + 1.5 * width.max(0.0)
}

/// The **numbers** a value holds.
fn numbers_of(value: &Json) -> Vec<f64> {
    value.as_array().map(|list| list.iter().filter_map(|item| item.as_f64()).collect()).unwrap_or_default()
}

/// The **points** a value holds: each of them two numbers.
fn the_points(value: &Json) -> Option<Vec<[f64; 2]>> {
    let list = value.as_array()?;
    let mut points = Vec::new();
    for point in list {
        let numbers = numbers_of(point);
        if numbers.len() != 2 || numbers.iter().any(|number| !number.is_finite()) {
            return None;
        }
        points.push([numbers[0], numbers[1]]);
    }
    Some(points)
}

/// A **colour**: three numbers between zero and one.
fn the_colour(value: &Json) -> Result<[f64; 3], EngineError> {
    let numbers = numbers_of(value);
    if numbers.len() != 3 || numbers.iter().any(|number| !number.is_finite()) {
        return Err(EngineError::Failed {
            id: COMMAND.to_string(),
            message: "a colour is three numbers".to_string(),
        });
    }
    if numbers.iter().any(|number| !(0.0..=1.0).contains(number)) {
        return Err(EngineError::Failed {
            id: COMMAND.to_string(),
            message: "a colour is between zero and one".to_string(),
        });
    }
    Ok([numbers[0], numbers[1], numbers[2]])
}

/// **Put** a **polygon** on a page.
#[derive(Command)]
#[command(id = "pdf.annot.polygon.add", area = "pdf", layer = "pdf", icon = "pencil", mutates, undoable)]
pub struct AddPolygon {
    /// The page the polygon is on.
    pub page: NodeRef,
    /// The object number the annotation has in the file.
    pub object: Number,
    /// The kind of shape: `polygon` or `polyline`.
    pub kind: Text,
    /// The corners of the shape: points of two numbers.
    pub vertices: Json,
    /// The colour of the line: three numbers.
    pub colour: Option<Json>,
    /// The width of the line, in points.
    pub width: Option<Number>,
    /// The colour of the fill of a polygon: three numbers.
    pub fill: Option<Json>,
    /// Whether the border of a polygon is **cloudy**.
    pub cloud: Option<Bool>,
}

impl AddPolygon {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        // The **kind**: one of the tree's own two.
        let kind = self.kind.0.trim().to_ascii_lowercase();
        let closed = match kind.as_str() {
            "polygon" => true,
            "polyline" => false,
            _ => {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: format!(
                        "`{}` is not a kind of polygon: it is `polygon` or `polyline`",
                        self.kind.0.trim()
                    ),
                });
            }
        };
        // The **page**: a page of the document.
        let Some(page) = session.document().node(self.page.0) else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("there is no node `{}`", self.page.0),
            });
        };
        let page_number = crate::the_number_of(page).ok_or_else(|| EngineError::Failed {
            id: COMMAND.to_string(),
            message: format!("the node `{}` has no object number", self.page.0),
        })?;
        // The **vertices**: points of two numbers.
        let Some(points) = the_points(&self.vertices) else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "the corners of a shape are points of two numbers".to_string(),
            });
        };
        // A polygon needs **three** points and a connected line needs
        // **two**: the tree's own rules.
        let least = if closed { 3 } else { 2 };
        if points.len() < least {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!(
                    "a {} needs at least {least} points",
                    if closed { "polygon" } else { "connected line" }
                ),
            });
        }
        // The **width**: the tree's own two points when nothing is
        // said, and never less than zero.
        let width = match self.width {
            Some(width) if !width.0.is_finite() => {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "the width of a line is a number".to_string(),
                });
            }
            Some(width) => width.0.max(0.0),
            None => WIDTH,
        };
        // The **colour**: the tree's own red when nothing is said.
        let colour = match &self.colour {
            Some(colour) => the_colour(colour)?,
            None => COLOUR,
        };
        // The **fill** of a polygon.
        let fill = match &self.fill {
            Some(fill) => Some(the_colour(fill)?),
            None => None,
        };
        // The **cloud** is a polygon only.
        let cloud = self.cloud.as_ref().map(|flag| flag.0).unwrap_or(false);
        if cloud && !closed {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "a connected line has no cloudy border".to_string(),
            });
        }
        // The **bounds** of the vertices: the tree's own rule, and the
        // bounds of a shape that is too small are refused.
        let left = points.iter().map(|point| point[0]).fold(f64::INFINITY, f64::min);
        let bottom = points.iter().map(|point| point[1]).fold(f64::INFINITY, f64::min);
        let right = points.iter().map(|point| point[0]).fold(f64::NEG_INFINITY, f64::max);
        let top = points.iter().map(|point| point[1]).fold(f64::NEG_INFINITY, f64::max);
        if closed {
            if right - left < 1.0 || top - bottom < 1.0 {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "a polygon is at least a point wide and a point high".to_string(),
                });
            }
        } else if (right - left).max(top - bottom) < 1.0 {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "a connected line is at least a point long".to_string(),
            });
        }
        // The **rectangle**: the bounds of the vertices, grown by half
        // the width and by one point more, and by one and a half cloud
        // radii when the border is cloudy.
        let mut pad = width / 2.0 + 1.0;
        if cloud {
            pad += 1.5 * the_cloud_radius(width);
        }
        let rect = [left - pad, bottom - pad, right + pad, top + pad];
        // The **object number** of the annotation: a whole number, and
        // one that nothing else has.
        let object = self.object.0;
        if !object.is_finite() || object < 0.0 || object.fract() != 0.0 {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "an object number is a whole number".to_string(),
            });
        }
        let number = object as u64;
        if number == 0 {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "object numbers start at one".to_string(),
            });
        }
        if session.document().walk().any(|node| crate::the_number_of(node) == Some(number)) {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("the object {number} is already in the document"),
            });
        }
        // The page's own **annotations**, before the new one is made.
        let annots = crate::entries_of(&page.data)
            .into_iter()
            .find(|(key, _)| key == "Annots")
            .and_then(|(_, value)| value.as_array().cloned())
            .unwrap_or_default();
        // The **annotation** itself: the tree's own polygon.
        let flat: Vec<f64> = points.iter().flat_map(|point| [point[0], point[1]]).collect();
        let mut entries: Vec<(String, Json)> = vec![
            ("Type".to_string(), json!("Annot")),
            ("Subtype".to_string(), json!(if closed { "Polygon" } else { "PolyLine" })),
            ("Rect".to_string(), json!(rect)),
            ("C".to_string(), json!(colour)),
            ("Vertices".to_string(), json!(flat)),
            ("BS".to_string(), json!({ "entries": [ ["W", width], ["S", "S"] ] })),
            ("P".to_string(), json!({ "ref": [page_number, 0] })),
        ];
        if let Some(fill) = fill {
            entries.push(("IC".to_string(), json!(fill)));
        }
        if cloud {
            entries.push(("IT".to_string(), json!("PolygonCloud")));
            entries.push(("BE".to_string(), json!({ "entries": [ ["S", "C"], ["I", 1] ] })));
        }
        let data = json!({ "object": number, "entries": entries });
        let name = format!("annotation {number}");
        let annot = session.document_mut()?.add_node_with(name.clone(), "pdf.object", None, data);
        // The page **names** the annotation.
        let mut annots = annots;
        annots.push(json!({ "ref": [number, 0] }));
        if let Some(node) = session.document_mut()?.node_mut(self.page.0) {
            node.data = crate::with_entry(&node.data, "Annots", json!(annots));
        }
        // And the **edge**: the page names the annotation.
        let relation = craftkit_sdk::Relation::with_payload(
            self.page.0,
            annot,
            "ref",
            json!({ "key": "Annots", "generation": 0 }),
        );
        if !session.document_mut()?.add_relation(relation) {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("the page already refers to the annotation `{annot}`"),
            });
        }
        Ok(Outcome::changed(format!("added the `{name}` to the page")).with_data(json!({
            "id": annot,
            "object": number,
            "page": self.page.0,
            "subtype": if closed { "Polygon" } else { "PolyLine" },
            "kind": kind,
            "rect": rect,
            "vertices": points,
            "colour": colour,
            "width": width,
            "fill": fill,
            "cloud": cloud,
        })))
    }
}
