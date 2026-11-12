//! The **typed signature** on a page: the tree's own signature of
//! filled outlines.
//!
//! The tree's own: a **typed signature** is the outline of a name that
//! a person typed — the name is written in a script, and what is put
//! on the page is not the text but the **filled outline** of it: the
//! **contours** of the letters, each one a list of points.
//!
//! The tree's contours are **normalised**: every point is between
//! zero and one, and the y grows upwards, so that the outline can be
//! laid over any rectangle. The tree makes those contours from the
//! outlines of a font it carries; here the **caller** says them, and
//! the tree's own rule that they are normalised is the rule here.
//!
//! # The rules
//!
//! - The **rectangle** is four numbers, the corners may come in any
//!   order, and it must be at least a point wide and a point high: the
//!   tree's own rule.
//! - The **contours** are a list of contours, and a contour is a list
//!   of points of two numbers each.
//! - A contour of **fewer than three points** is **dropped**: the
//!   tree's own rule.
//! - Every point must be **between zero and one**: the tree's own
//!   normalisation, and a point that is not is **refused**.
//! - The **name** of the annotation is the tree's own:
//!   `PCTypedSignature`, and the **colour** is the tree's own black
//!   when the caller says none.
//!
//! # What it does not settle
//!
//! - The **drawing** of the signature is not here: it is the
//!   appearance command's own work.
//! - The tree's own **script outlines** are not here: they come from
//!   the outlines of a font the tree carries, and this tree carries no
//!   fonts, so the contours are the caller's.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.annot.typedsignature.add";

/// The tree's own **colour** of a typed signature: the black.
const COLOUR: [f64; 3] = [0.0, 0.0, 0.0];

/// The **numbers** a value holds.
fn numbers_of(value: &Json) -> Vec<f64> {
    value.as_array().map(|list| list.iter().filter_map(|item| item.as_f64()).collect()).unwrap_or_default()
}

/// The **contours** of the signature: each one a list of points, and
/// each point two numbers between zero and one.
///
/// A contour of fewer than three points is dropped: the tree's own
/// rule.
fn the_contours(value: &Json) -> Result<Vec<Vec<f64>>, EngineError> {
    let Some(list) = value.as_array() else {
        return Err(EngineError::Failed {
            id: COMMAND.to_string(),
            message: "the contours of a typed signature are a list of contours".to_string(),
        });
    };
    let mut contours: Vec<Vec<f64>> = Vec::new();
    for contour in list {
        let Some(points) = contour.as_array() else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "a contour of a typed signature is a list of points".to_string(),
            });
        };
        let mut flat: Vec<f64> = Vec::new();
        for point in points {
            let numbers = numbers_of(point);
            if numbers.len() != 2 {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "a point of a typed signature is two numbers".to_string(),
                });
            }
            if numbers.iter().any(|number| !number.is_finite() || !(0.0..=1.0).contains(number)) {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "a point of a typed signature is between zero and one".to_string(),
                });
            }
            flat.push(numbers[0]);
            flat.push(numbers[1]);
        }
        // A contour of fewer than three points is dropped: the tree's
        // own rule.
        if flat.len() > 4 {
            contours.push(flat);
        }
    }
    Ok(contours)
}

/// **Put** a **typed signature** on a page.
#[derive(Command)]
#[command(
    id = "pdf.annot.typedsignature.add",
    area = "pdf",
    layer = "pdf",
    icon = "pencil",
    mutates,
    undoable
)]
pub struct AddTypedSignature {
    /// The page the signature is on.
    pub page: NodeRef,
    /// The object number the annotation has in the file.
    pub object: Number,
    /// The rectangle the signature covers: four numbers.
    pub rect: Json,
    /// The contours of the signature: each one a list of points.
    pub contours: Json,
    /// The colour of the signature: three numbers.
    pub colour: Option<Json>,
}

impl AddTypedSignature {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        // The **rectangle**: four numbers, and the corners may come in
        // any order.
        let numbers = numbers_of(&self.rect);
        if numbers.len() != 4 || numbers.iter().any(|number| !number.is_finite()) {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "the rectangle of a typed signature is four numbers".to_string(),
            });
        }
        let rect = [
            numbers[0].min(numbers[2]),
            numbers[1].min(numbers[3]),
            numbers[0].max(numbers[2]),
            numbers[1].max(numbers[3]),
        ];
        // A rectangle that is too small is refused: the tree's own rule.
        if rect[2] - rect[0] < 1.0 || rect[3] - rect[1] < 1.0 {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "a typed signature is at least a point wide and a point high".to_string(),
            });
        }
        // The **contours** of the signature.
        let contours = the_contours(&self.contours)?;
        // The **colour**: the tree's own black when nothing is said.
        let colour = match &self.colour {
            Some(colour) => {
                let numbers = numbers_of(colour);
                if numbers.len() != 3 || numbers.iter().any(|number| !number.is_finite()) {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "the colour of a typed signature is three numbers".to_string(),
                    });
                }
                if numbers.iter().any(|number| !(0.0..=1.0).contains(number)) {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "the colour of a typed signature is between zero and one".to_string(),
                    });
                }
                [numbers[0], numbers[1], numbers[2]]
            }
            None => COLOUR,
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
        // The **annotation** itself: the tree's own typed signature.
        let entries: Vec<(String, Json)> = vec![
            ("Type".to_string(), json!("Annot")),
            ("Subtype".to_string(), json!("Stamp")),
            ("Rect".to_string(), json!(rect)),
            ("Name".to_string(), json!("PCTypedSignature")),
            ("C".to_string(), json!(colour)),
            ("PCOutline".to_string(), json!(contours)),
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
            "contours": contours.len(),
            "colour": colour,
        })))
    }
}
