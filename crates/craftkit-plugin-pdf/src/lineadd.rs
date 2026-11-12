//! The **line** on a page: the tree's own line.
//!
//! The tree's own: a **line** is an annotation that joins two points,
//! and it may have an **arrow** at the end of it. The line is drawn by
//! the tree when the annotation is made; here the annotation is made
//! here and the drawing is the appearance command's own work.
//!
//! # The rules
//!
//! - The **points** are two points — the one the line starts at and
//!   the one it ends at — and each point is two numbers, and a number
//!   that is not a number is **refused**.
//! - A line **shorter than a point** is **refused**: the tree's own
//!   rule, and a line a reader cannot see is not a line.
//! - The **width** of the line is the tree's own two points when the
//!   caller says nothing, and a width that is not a number is
//!   refused; a width below zero is the tree's own zero.
//! - The **colour** of the line is the tree's own red when the caller
//!   says nothing, and it is three numbers between zero and one, and a
//!   number outside that is **refused**.
//! - The **rectangle** of the annotation is the **bounds** of the two
//!   points, grown by half the width of the line, by the **size of
//!   the arrow** when the line has one, and by one point more — the
//!   tree's own rectangle, so that the line and its arrow are inside
//!   it.
//! - The **size of the arrow** is six points plus three times the
//!   width of the line: the tree's own `arrow_size`.
//! - The **line** is the two points — the `/L` of the annotation — and
//!   an **arrow** is the tree's own: the `/LE` of the annotation says
//!   `None` at the start and `OpenArrow` at the end.
//!
//! # What it does not settle
//!
//! - The **drawing** of the line is not here: it is the appearance
//!   command's own work.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, NodeRef, Number, Outcome};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.annot.line.add";

/// The tree's own **width** of a line: two points.
const WIDTH: f64 = 2.0;

/// The tree's own **colour** of a line: the red.
const COLOUR: [f64; 3] = [0.89, 0.13, 0.13];

/// The **size of the arrow**: the tree's own `arrow_size` — six points
/// plus three times the width of the line.
fn the_arrow_size(width: f64) -> f64 {
    6.0 + 3.0 * width.max(0.0)
}

/// The **numbers** a value holds.
fn numbers_of(value: &Json) -> Vec<f64> {
    value.as_array().map(|list| list.iter().filter_map(|item| item.as_f64()).collect()).unwrap_or_default()
}

/// A **point**: two numbers.
fn a_point(value: &Json) -> Option<[f64; 2]> {
    let numbers = numbers_of(value);
    if numbers.len() != 2 || numbers.iter().any(|number| !number.is_finite()) {
        return None;
    }
    Some([numbers[0], numbers[1]])
}

commands! {
    /// **Put** a **line** on a page.
    pdf.annot.line.add {
        area: "pdf",
        layer: "pdf",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The page the line is on.

            page: NodeRef,
            /// The object number the annotation has in the file.

            object: Number,
            /// The point the line starts at: two numbers.

            from: Json,
            /// The point the line ends at: two numbers.

            to: Json,
            /// Whether the line has an **arrow** at the end of it.

            arrow: Option<Bool>,
            /// The colour of the line: three numbers.

            colour: Option<Json>,
            /// The width of the line, in points.

            width: Option<Number>,
        },
        run: {

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
            // The **points**: two points, and each is two numbers.
            let Some(from) = a_point(&self.from) else {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "the point the line starts at is two numbers".to_string(),
                });
            };
            let Some(to) = a_point(&self.to) else {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "the point the line ends at is two numbers".to_string(),
                });
            };
            // A line **shorter than a point** is refused: the tree's own
            // rule.
            if (from[0] - to[0]).hypot(from[1] - to[1]) < 1.0 {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "a line is at least a point long".to_string(),
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
            // The **colour**: the tree's own red when nothing is said, and
            // three numbers between zero and one.
            let colour = match &self.colour {
                Some(colour) => {
                    let numbers = numbers_of(colour);
                    if numbers.len() != 3 || numbers.iter().any(|number| !number.is_finite()) {
                        return Err(EngineError::Failed {
                            id: COMMAND.to_string(),
                            message: "the colour of a line is three numbers".to_string(),
                        });
                    }
                    if numbers.iter().any(|number| !(0.0..=1.0).contains(number)) {
                        return Err(EngineError::Failed {
                            id: COMMAND.to_string(),
                            message: "the colour of a line is between zero and one".to_string(),
                        });
                    }
                    [numbers[0], numbers[1], numbers[2]]
                }
                None => COLOUR,
            };
            // The **arrow**: the tree's own, at the end of the line.
            let arrow = self.arrow.as_ref().map(|flag| flag.0).unwrap_or(false);
            // The **rectangle**: the bounds of the two points, grown by half
            // the width, by the size of the arrow when there is one, and by
            // one point more.
            let left = from[0].min(to[0]);
            let bottom = from[1].min(to[1]);
            let right = from[0].max(to[0]);
            let top = from[1].max(to[1]);
            let pad = width / 2.0 + if arrow { the_arrow_size(width) } else { 0.0 } + 1.0;
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
            // The **annotation** itself: the tree's own line.
            let mut entries: Vec<(String, Json)> = vec![
                ("Type".to_string(), json!("Annot")),
                ("Subtype".to_string(), json!("Line")),
                ("Rect".to_string(), json!(rect)),
                ("C".to_string(), json!(colour)),
                ("L".to_string(), json!([from[0], from[1], to[0], to[1]])),
                ("BS".to_string(), json!({ "entries": [ ["W", width], ["S", "S"] ] })),
                ("P".to_string(), json!({ "ref": [page_number, 0] })),
            ];
            if arrow {
                entries.push(("LE".to_string(), json!(["None", "OpenArrow"])));
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
                "subtype": "Line",
                "rect": rect,
                "from": from,
                "to": to,
                "arrow": arrow,
                "colour": colour,
                "width": width,
            })))

        }
    }
}
