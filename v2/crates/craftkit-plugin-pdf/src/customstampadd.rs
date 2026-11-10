//! The **custom stamp** on a page: the tree's own stamp of a picture.
//!
//! The tree's own: a **custom stamp** is a stamp whose artwork is a
//! **picture that is already in the document** — an image, or a form —
//! and the stamp is that picture, laid over a rectangle.
//!
//! # The rules
//!
//! - The **rectangle** is four numbers, the corners may come in any
//!   order, and it must be at least a point wide and a point high: the
//!   tree's own rule.
//! - The **picture** is an object of the document, and the stamp refers
//!   to it: the tree's own `/PCPicture`.
//! - The **name** of the annotation is the tree's own: `PCCustom` and
//!   then the letters and the digits of the name the caller gave — the
//!   tree's own cleaning of it — and a name with no letters or digits
//!   in it is **refused**.
//! - Whether the picture is an **image** is the tree's own `/PCPictureImage`.
//! - The **size** of the picture is two numbers, and they must be
//!   greater than zero: the tree's own `/PCPictureSize`, which a
//!   picture that is **not** an image needs, because it is the size
//!   the form is mapped to.
//!
//! # What it does not settle
//!
//! - The **drawing** of the stamp is not here: it is the appearance
//!   command's own work.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, NodeRef, Number, Outcome, Session, Text};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.annot.customstamp.add";

/// The **numbers** a value holds.
fn numbers_of(value: &Json) -> Vec<f64> {
    value.as_array().map(|list| list.iter().filter_map(|item| item.as_f64()).collect()).unwrap_or_default()
}

/// The **clean name** of the stamp: the letters and the digits of the
/// name the caller gave: the tree's own cleaning of it.
fn the_clean_name(name: &str) -> String {
    name.chars().filter(|letter| letter.is_ascii_alphanumeric()).collect()
}

/// **Put** a **custom stamp** on a page.
#[derive(Command)]
#[command(id = "pdf.annot.customstamp.add", area = "pdf", layer = "pdf", icon = "pencil", mutates, undoable)]
pub struct AddCustomStamp {
    /// The page the stamp is on.
    pub page: NodeRef,
    /// The object number the annotation has in the file.
    pub object: Number,
    /// The rectangle the stamp covers: four numbers.
    pub rect: Json,
    /// The name of the stamp.
    pub name: Text,
    /// The picture the stamp is: an object of the document.
    pub picture: NodeRef,
    /// Whether the picture is an **image** and not a form.
    pub image: Option<Bool>,
    /// The size of the picture: two numbers.
    pub size: Option<Json>,
}

impl AddCustomStamp {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        // The **rectangle**: four numbers, and the corners may come in
        // any order.
        let numbers = numbers_of(&self.rect);
        if numbers.len() != 4 || numbers.iter().any(|number| !number.is_finite()) {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "the rectangle of a custom stamp is four numbers".to_string(),
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
                message: "a custom stamp is at least a point wide and a point high".to_string(),
            });
        }
        // The **name**: the tree's own cleaning of it.
        let clean = the_clean_name(&self.name.0);
        if clean.is_empty() {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "the name of a custom stamp has letters or digits in it".to_string(),
            });
        }
        // The **picture**: an object of the document.
        let Some(picture) = session.document().node(self.picture.0) else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("there is no node `{}`", self.picture.0),
            });
        };
        let picture_number = crate::the_number_of(picture).ok_or_else(|| EngineError::Failed {
            id: COMMAND.to_string(),
            message: format!("the node `{}` has no object number", self.picture.0),
        })?;
        // Whether the picture is an **image**.
        let image = matches!(self.image, Some(Bool(true)));
        // The **size** of the picture: two numbers, and they must be
        // greater than zero — the tree's own `/PCPictureSize`, which a
        // picture that is not an image needs.
        let size = match &self.size {
            Some(size) => {
                let numbers = numbers_of(size);
                if numbers.len() != 2 || numbers.iter().any(|number| !number.is_finite()) {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "the size of a picture is two numbers".to_string(),
                    });
                }
                if numbers[0] <= 0.0 || numbers[1] <= 0.0 {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "the size of a picture is greater than zero".to_string(),
                    });
                }
                Some([numbers[0], numbers[1]])
            }
            None => {
                // A picture that is **not** an image is a form, and the
                // size is the size it is mapped to: the tree's own rule.
                if image {
                    None
                } else {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "a picture that is not an image needs its size".to_string(),
                    });
                }
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
        // The **annotation** itself: the tree's own custom stamp.
        let entries: Vec<(String, Json)> = vec![
            ("Type".to_string(), json!("Annot")),
            ("Subtype".to_string(), json!("Stamp")),
            ("Rect".to_string(), json!(rect)),
            ("Name".to_string(), json!(format!("PCCustom{clean}"))),
            ("PCPicture".to_string(), json!({ "ref": [picture_number, 0] })),
            ("PCPictureImage".to_string(), json!(image)),
            ("P".to_string(), json!({ "ref": [page_number, 0] })),
        ];
        let mut entries = entries;
        // The **size** of the picture is the tree's own `/PCPictureSize`,
        // and it is there when the picture has one.
        if let Some(size) = size {
            entries.push(("PCPictureSize".to_string(), json!(size)));
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
            "subtype": "Stamp",
            "rect": rect,
            "name": format!("PCCustom{clean}"),
            "picture": picture_number,
            "image": image,
            "size": size,
        })))
    }
}
