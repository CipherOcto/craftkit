//! The **mark of Fill and Sign** on a page: the tree's own check, cross,
//! dot and line.
//!
//! The tree's own: a **mark** is an annotation that says a hand has
//! been here — the check, the cross, the dot and the line that a form
//! is signed with. The tree's own name for each of them is in the
//! file: `PCCheck`, `PCCross`, `PCDot` and `PCLine`.
//!
//! # The rules
//!
//! - The **mark** is one of the tree's own four — `check`, `cross`,
//!   `dot` and `line` — and a mark that is not one of them is
//!   **refused**.
//! - The **rectangle** is four numbers, the corners may come in any
//!   order, and it must be at least a point wide and a point high:
//!   the tree's own rule.
//! - The **colour** of the mark is the tree's own black when the
//!   caller says nothing, and it is three numbers between zero and
//!   one.
//! - The **width** of the line of the mark is the tree's own one and
//!   a half points, and it is never less than zero.
//! - The **name** of the annotation is the tree's own: `PCCheck`,
//!   `PCCross`, `PCDot` or `PCLine`.
//!
//! # What it does not settle
//!
//! - The **drawing** of the mark is not here: it is the appearance
//!   command's own work.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Text};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.annot.mark.add";

/// The tree's own **width** of a mark: one and a half points.
const WIDTH: f64 = 1.5;

/// The tree's own **colour** of a mark: the black.
const COLOUR: [f64; 3] = [0.0, 0.0, 0.0];

/// A **mark** of Fill and Sign: the tree's own four.
pub struct Mark {
    /// The name the file gives it.
    pub name: &'static str,
    /// The text the tree's editor shows for it.
    pub label: &'static str,
    /// The short name it is asked for by.
    pub said: &'static str,
}

/// The tree's own **four** marks.
pub const MARKS: [Mark; 4] = [
    Mark { name: "PCCheck", label: "Checkmark", said: "check" },
    Mark { name: "PCCross", label: "Cross", said: "cross" },
    Mark { name: "PCDot", label: "Dot", said: "dot" },
    Mark { name: "PCLine", label: "Line", said: "line" },
];

/// The **mark** the caller names: the tree's own four, said as the
/// tree's editor says them or as the file says them.
pub fn mark_of(name: &str) -> Option<&'static Mark> {
    let name = name.trim();
    MARKS.iter().find(|mark| {
        mark.name == name || mark.said.eq_ignore_ascii_case(name) || mark.label.eq_ignore_ascii_case(name)
    })
}

/// The **numbers** a value holds.
fn numbers_of(value: &Json) -> Vec<f64> {
    value.as_array().map(|list| list.iter().filter_map(|item| item.as_f64()).collect()).unwrap_or_default()
}

commands! {
    /// **Put** a **mark** on a page.
    pdf.annot.mark.add {
        area: "pdf",
        layer: "pdf",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The page the mark is on.

            page: NodeRef,
            /// The object number the annotation has in the file.

            object: Number,
            /// The rectangle the mark covers: four numbers.

            rect: Json,
            /// The mark: `check`, `cross`, `dot` or `line`.

            mark: Text,
            /// The colour of the mark: three numbers.

            colour: Option<Json>,
        },
        run: {

            // The **mark**: one of the tree's own four.
            let Some(mark) = mark_of(&self.mark.0) else {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: format!(
                        "`{}` is not a mark: they are check, cross, dot and line",
                        self.mark.0.trim()
                    ),
                });
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
            // The **rectangle**: four numbers, and the corners may come in
            // any order.
            let numbers = numbers_of(&self.rect);
            if numbers.len() != 4 || numbers.iter().any(|number| !number.is_finite()) {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "the rectangle of a mark is four numbers".to_string(),
                });
            }
            let rect = [
                numbers[0].min(numbers[2]),
                numbers[1].min(numbers[3]),
                numbers[0].max(numbers[2]),
                numbers[1].max(numbers[3]),
            ];
            // A mark that is too small is refused: the tree's own rule.
            if rect[2] - rect[0] < 1.0 || rect[3] - rect[1] < 1.0 {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "a mark is at least a point wide and a point high".to_string(),
                });
            }
            // The **colour**: the tree's own black when nothing is said.
            let colour = match &self.colour {
                Some(colour) => {
                    let numbers = numbers_of(colour);
                    if numbers.len() != 3 || numbers.iter().any(|number| !number.is_finite()) {
                        return Err(EngineError::Failed {
                            id: COMMAND.to_string(),
                            message: "the colour of a mark is three numbers".to_string(),
                        });
                    }
                    if numbers.iter().any(|number| !(0.0..=1.0).contains(number)) {
                        return Err(EngineError::Failed {
                            id: COMMAND.to_string(),
                            message: "the colour of a mark is between zero and one".to_string(),
                        });
                    }
                    [numbers[0], numbers[1], numbers[2]]
                }
                None => COLOUR,
            };
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
            // The **annotation** itself: the tree's own mark.
            let entries: Vec<(String, Json)> = vec![
                ("Type".to_string(), json!("Annot")),
                ("Subtype".to_string(), json!("Stamp")),
                ("Rect".to_string(), json!(rect)),
                ("C".to_string(), json!(colour)),
                ("Name".to_string(), json!(mark.name)),
                ("BS".to_string(), json!({ "entries": [ ["W", WIDTH], ["S", "S"] ] })),
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
                "subtype": "Stamp",
                "rect": rect,
                "mark": mark.name,
                "label": mark.label,
                "colour": colour,
                "width": WIDTH,
            })))

        }
    }
}
