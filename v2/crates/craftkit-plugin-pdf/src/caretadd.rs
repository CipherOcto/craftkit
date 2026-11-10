//! The **caret** on a page: the tree's own inserted-text mark.
//!
//! The tree's own: a **caret** is an annotation that says "text was
//! inserted here", and it is drawn as a filled wedge. The tree's own
//! symbol for it is `None` — the mark itself is the drawing — and the
//! text of the insertion is the contents of the annotation.
//!
//! # The rules
//!
//! - The **rectangle** is four numbers, the corners may come in any
//!   order, and it must be at least a point wide and a point high:
//!   the tree's own rule.
//! - The **colour** of the caret is the tree's own blue when the
//!   caller says nothing, and it is three numbers between zero and
//!   one.
//! - The **symbol** of the caret is the tree's own: the `/Sy` of the
//!   annotation says `None`, and the mark is the drawing of it.
//!
//! # What it does not settle
//!
//! - The **drawing** of the caret is not here: it is the appearance
//!   command's own work.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session, Text};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.annot.caret.add";

/// The tree's own **colour** of a caret: the blue.
const COLOUR: [f64; 3] = [0.0, 0.47, 0.84];

/// The **numbers** a value holds.
fn numbers_of(value: &Json) -> Vec<f64> {
    value.as_array().map(|list| list.iter().filter_map(|item| item.as_f64()).collect()).unwrap_or_default()
}

/// **Put** a **caret** on a page.
#[derive(Command)]
#[command(id = "pdf.annot.caret.add", area = "pdf", layer = "pdf", icon = "pencil", mutates, undoable)]
pub struct AddCaret {
    /// The page the caret is on.
    pub page: NodeRef,
    /// The object number the annotation has in the file.
    pub object: Number,
    /// The rectangle the caret covers: four numbers.
    pub rect: Json,
    /// The colour of the caret: three numbers.
    pub colour: Option<Json>,
    /// The text that was inserted.
    pub contents: Option<Text>,
}

impl AddCaret {
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
        // The **rectangle**: four numbers, and the corners may come in
        // any order.
        let numbers = numbers_of(&self.rect);
        if numbers.len() != 4 || numbers.iter().any(|number| !number.is_finite()) {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "the rectangle of a caret is four numbers".to_string(),
            });
        }
        let rect = [
            numbers[0].min(numbers[2]),
            numbers[1].min(numbers[3]),
            numbers[0].max(numbers[2]),
            numbers[1].max(numbers[3]),
        ];
        // A caret that is too small is refused: the tree's own rule.
        if rect[2] - rect[0] < 1.0 || rect[3] - rect[1] < 1.0 {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "a caret is at least a point wide and a point high".to_string(),
            });
        }
        // The **colour**: the tree's own blue when nothing is said.
        let colour = match &self.colour {
            Some(colour) => {
                let numbers = numbers_of(colour);
                if numbers.len() != 3 || numbers.iter().any(|number| !number.is_finite()) {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "the colour of a caret is three numbers".to_string(),
                    });
                }
                if numbers.iter().any(|number| !(0.0..=1.0).contains(number)) {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "the colour of a caret is between zero and one".to_string(),
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
        // The **annotation** itself: the tree's own caret.
        let mut entries: Vec<(String, Json)> = vec![
            ("Type".to_string(), json!("Annot")),
            ("Subtype".to_string(), json!("Caret")),
            ("Rect".to_string(), json!(rect)),
            ("C".to_string(), json!(colour)),
            ("Sy".to_string(), json!("None")),
            ("P".to_string(), json!({ "ref": [page_number, 0] })),
        ];
        if let Some(contents) = &self.contents {
            entries.push(("Contents".to_string(), json!(contents.0)));
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
            "subtype": "Caret",
            "rect": rect,
            "colour": colour,
        })))
    }
}
