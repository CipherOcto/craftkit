//! The **ink** on a page: the tree's own freehand strokes.
//!
//! The tree's own: an **ink** is an annotation whose `/InkList` is a
//! list of **strokes**, and a stroke is a list of **points** — the
//! points the hand went through. The ink is drawn by the tree when
//! the annotation is made; here the annotation is made here and the
//! drawing is the appearance command's own work.
//!
//! # The rules
//!
//! - The **strokes** are strokes of points, and each point is two
//!   numbers, and a number that is not a number is **refused**.
//! - **Every stroke empty** is **refused**: the tree's own rule, and
//!   an ink with no points in it marks nothing.
//! - The **width** of the ink is the tree's own two points when the
//!   caller says nothing, and a width below zero is the tree's own
//!   zero.
//! - The **colour** of the ink is the tree's own blue when the caller
//!   says nothing, and it is three numbers between zero and one.
//! - The **rectangle** of the annotation is the **bounds** of all the
//!   points, grown by half the width of the ink and by one point more:
//!   the tree's own rectangle.
//! - The **ink list** of the annotation is the list of the strokes, and
//!   a stroke is a flat list of numbers — the tree's own `/InkList`.
//!
//! # What it does not settle
//!
//! - The **drawing** of the ink is not here: it is the appearance
//!   command's own work.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.annot.ink.add";

/// The tree's own **width** of the ink: two points.
const WIDTH: f64 = 2.0;

/// The tree's own **colour** of the ink: the blue.
const COLOUR: [f64; 3] = [0.0, 0.4, 0.87];

/// The **numbers** a value holds.
fn numbers_of(value: &Json) -> Vec<f64> {
    value.as_array().map(|list| list.iter().filter_map(|item| item.as_f64()).collect()).unwrap_or_default()
}

/// The **strokes** a value holds: each of them a list of points of two
/// numbers.
fn the_strokes(value: &Json) -> Option<Vec<Vec<[f64; 2]>>> {
    let list = value.as_array()?;
    let mut strokes = Vec::new();
    for stroke in list {
        let points = stroke.as_array()?;
        let mut points_of_stroke = Vec::new();
        for point in points {
            let numbers = numbers_of(point);
            if numbers.len() != 2 || numbers.iter().any(|number| !number.is_finite()) {
                return None;
            }
            points_of_stroke.push([numbers[0], numbers[1]]);
        }
        strokes.push(points_of_stroke);
    }
    Some(strokes)
}

/// **Put** **ink** on a page.
#[derive(Command)]
#[command(id = "pdf.annot.ink.add", area = "pdf", layer = "pdf", icon = "pencil", mutates, undoable)]
pub struct AddInk {
    /// The page the ink is on.
    pub page: NodeRef,
    /// The object number the annotation has in the file.
    pub object: Number,
    /// The strokes of the ink: lists of points of two numbers.
    pub strokes: Json,
    /// The colour of the ink: three numbers.
    pub colour: Option<Json>,
    /// The width of the ink, in points.
    pub width: Option<Number>,
}

impl AddInk {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
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
        // The **strokes**: lists of points of two numbers.
        let Some(strokes) = the_strokes(&self.strokes) else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "the strokes of the ink are lists of points of two numbers".to_string(),
            });
        };
        // **Every stroke empty** is refused: the tree's own rule.
        if strokes.iter().all(|stroke| stroke.is_empty()) {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "the ink has no points in it".to_string(),
            });
        }
        // The **width**: the tree's own two points when nothing is
        // said, and never less than zero.
        let width = match self.width {
            Some(width) if !width.0.is_finite() => {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "the width of the ink is a number".to_string(),
                });
            }
            Some(width) => width.0.max(0.0),
            None => WIDTH,
        };
        // The **colour**: the tree's own blue when nothing is said.
        let colour = match &self.colour {
            Some(colour) => {
                let numbers = numbers_of(colour);
                if numbers.len() != 3 || numbers.iter().any(|number| !number.is_finite()) {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "the colour of the ink is three numbers".to_string(),
                    });
                }
                if numbers.iter().any(|number| !(0.0..=1.0).contains(number)) {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "the colour of the ink is between zero and one".to_string(),
                    });
                }
                [numbers[0], numbers[1], numbers[2]]
            }
            None => COLOUR,
        };
        // The **bounds** of all the points.
        let points: Vec<[f64; 2]> = strokes.iter().flatten().copied().collect();
        let left = points.iter().map(|point| point[0]).fold(f64::INFINITY, f64::min);
        let bottom = points.iter().map(|point| point[1]).fold(f64::INFINITY, f64::min);
        let right = points.iter().map(|point| point[0]).fold(f64::NEG_INFINITY, f64::max);
        let top = points.iter().map(|point| point[1]).fold(f64::NEG_INFINITY, f64::max);
        // The **rectangle**: the bounds of the points, grown by half the
        // width and by one point more.
        let pad = width / 2.0 + 1.0;
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
        // The **annotation** itself: the tree's own ink.
        let list: Vec<Vec<f64>> = strokes
            .iter()
            .map(|stroke| stroke.iter().flat_map(|point| [point[0], point[1]]).collect())
            .collect();
        let entries: Vec<(String, Json)> = vec![
            ("Type".to_string(), json!("Annot")),
            ("Subtype".to_string(), json!("Ink")),
            ("Rect".to_string(), json!(rect)),
            ("C".to_string(), json!(colour)),
            ("InkList".to_string(), json!(list)),
            ("BS".to_string(), json!({ "entries": [ ["W", width], ["S", "S"] ] })),
            ("P".to_string(), json!({ "ref": [page_number, 0] })),
        ];
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
            "subtype": "Ink",
            "rect": rect,
            "strokes": strokes,
            "colour": colour,
            "width": width,
        })))
    }
}
