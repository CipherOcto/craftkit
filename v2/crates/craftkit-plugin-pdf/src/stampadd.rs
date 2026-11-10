//! The **stamp** on a page: the tree's own stamps.
//!
//! The tree's own: a **stamp** is an annotation that says something
//! about the page — that it is approved, that it is a draft, that it
//! wants a signature. The tree has **twenty two** of them, and they
//! come in three **groups**: the standard business ones, the ones
//! that ask for a signature, and the **dynamic** ones, which carry a
//! line that says who did it and when.
//!
//! # The rules
//!
//! - The **kind** is one of the tree's own twenty two, named as the
//!   file names it — `Approved`, `PCSignHere` and the rest — and a
//!   kind that is not one of them is **refused**.
//! - The **rectangle** is four numbers, the corners may come in any
//!   order, and it must be at least a point wide and a point high:
//!   the tree's own rule.
//! - The **colour** of the stamp is the tree's own colour for the
//!   kind — green for an approval, red for a refusal or a
//!   restriction, orange for a signature and blue for the rest — and
//!   the caller may say another one, and it is three numbers between
//!   zero and one.
//! - The **name** of the annotation is the tree's own: the standard
//!   stamp names of the file where one exists, and `PC` and the name
//!   of the kind where one does not.
//! - The stamp says it is the tree's own with `/PCStamp`, and the
//!   **line** that says who did it is `/PCByLine` — the tree's own
//!   keys.
//!
//! # What it does not settle
//!
//! - The **drawing** of the stamp is not here: it is the appearance
//!   command's own work.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session, Text};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.annot.stamp.add";

/// The **group** of a stamp: the tree's own three.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Group {
    /// The standard business ones.
    Business,
    /// The ones that ask for a signature.
    SignHere,
    /// The ones that carry a line that says who did it.
    Dynamic,
}

/// A **kind** of stamp: the tree's own twenty two.
pub struct Kind {
    /// The name the file gives it.
    pub name: &'static str,
    /// The text on the stamp.
    pub label: &'static str,
    /// The group it is in.
    pub group: Group,
    /// The colour of the stamp.
    pub colour: [f64; 3],
}

/// The tree's own **twenty two** stamps.
pub const KINDS: [Kind; 22] = [
    Kind { name: "Approved", label: "APPROVED", group: Group::Business, colour: [0.13, 0.55, 0.13] },
    Kind { name: "Completed", label: "COMPLETED", group: Group::Business, colour: [0.13, 0.55, 0.13] },
    Kind { name: "Confidential", label: "CONFIDENTIAL", group: Group::Business, colour: [0.80, 0.10, 0.10] },
    Kind { name: "Draft", label: "DRAFT", group: Group::Business, colour: [0.10, 0.30, 0.70] },
    Kind { name: "Final", label: "FINAL", group: Group::Business, colour: [0.13, 0.55, 0.13] },
    Kind { name: "ForComment", label: "FOR COMMENT", group: Group::Business, colour: [0.10, 0.30, 0.70] },
    Kind {
        name: "ForPublicRelease",
        label: "FOR PUBLIC RELEASE",
        group: Group::Business,
        colour: [0.10, 0.30, 0.70],
    },
    Kind {
        name: "InformationOnly",
        label: "INFORMATION ONLY",
        group: Group::Business,
        colour: [0.10, 0.30, 0.70],
    },
    Kind { name: "NotApproved", label: "NOT APPROVED", group: Group::Business, colour: [0.80, 0.10, 0.10] },
    Kind {
        name: "NotForPublicRelease",
        label: "NOT FOR PUBLIC RELEASE",
        group: Group::Business,
        colour: [0.80, 0.10, 0.10],
    },
    Kind {
        name: "PCPreliminaryResults",
        label: "PRELIMINARY RESULTS",
        group: Group::Business,
        colour: [0.10, 0.30, 0.70],
    },
    Kind { name: "PCVoid", label: "VOID", group: Group::Business, colour: [0.80, 0.10, 0.10] },
    Kind { name: "PCAccepted", label: "ACCEPTED", group: Group::SignHere, colour: [0.13, 0.55, 0.13] },
    Kind { name: "PCInitialHere", label: "INITIAL HERE", group: Group::SignHere, colour: [0.85, 0.35, 0.05] },
    Kind { name: "PCRejected", label: "REJECTED", group: Group::SignHere, colour: [0.80, 0.10, 0.10] },
    Kind { name: "PCSignHere", label: "SIGN HERE", group: Group::SignHere, colour: [0.85, 0.35, 0.05] },
    Kind { name: "PCWitness", label: "WITNESS", group: Group::SignHere, colour: [0.85, 0.35, 0.05] },
    Kind { name: "PCDynApproved", label: "APPROVED", group: Group::Dynamic, colour: [0.13, 0.55, 0.13] },
    Kind {
        name: "PCDynConfidential",
        label: "CONFIDENTIAL",
        group: Group::Dynamic,
        colour: [0.80, 0.10, 0.10],
    },
    Kind { name: "PCDynReceived", label: "RECEIVED", group: Group::Dynamic, colour: [0.13, 0.55, 0.13] },
    Kind { name: "PCDynReviewed", label: "REVIEWED", group: Group::Dynamic, colour: [0.13, 0.55, 0.13] },
    Kind { name: "PCDynRevised", label: "REVISED", group: Group::Dynamic, colour: [0.10, 0.30, 0.70] },
];

/// The **kind** of stamp the caller names.
pub fn kind_of(name: &str) -> Option<&'static Kind> {
    KINDS.iter().find(|kind| kind.name == name.trim())
}

/// The **numbers** a value holds.
fn numbers_of(value: &Json) -> Vec<f64> {
    value.as_array().map(|list| list.iter().filter_map(|item| item.as_f64()).collect()).unwrap_or_default()
}

/// **Put** a **stamp** on a page.
#[derive(Command)]
#[command(id = "pdf.annot.stamp.add", area = "pdf", layer = "pdf", icon = "pencil", mutates, undoable)]
pub struct AddStamp {
    /// The page the stamp is on.
    pub page: NodeRef,
    /// The object number the annotation has in the file.
    pub object: Number,
    /// The rectangle the stamp covers: four numbers.
    pub rect: Json,
    /// The kind of stamp: one of the tree's own twenty two.
    pub kind: Text,
    /// The line that says who did it.
    pub by: Option<Text>,
    /// The colour of the stamp: three numbers.
    pub colour: Option<Json>,
}

impl AddStamp {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        // The **kind**: one of the tree's own twenty two.
        let Some(kind) = kind_of(&self.kind.0) else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!(
                    "`{}` is not a kind of stamp: it is one of {}",
                    self.kind.0.trim(),
                    KINDS.iter().map(|kind| kind.name).collect::<Vec<_>>().join(", ")
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
                message: "the rectangle of a stamp is four numbers".to_string(),
            });
        }
        let rect = [
            numbers[0].min(numbers[2]),
            numbers[1].min(numbers[3]),
            numbers[0].max(numbers[2]),
            numbers[1].max(numbers[3]),
        ];
        // A stamp that is too small is refused: the tree's own rule.
        if rect[2] - rect[0] < 1.0 || rect[3] - rect[1] < 1.0 {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "a stamp is at least a point wide and a point high".to_string(),
            });
        }
        // The **colour**: the tree's own colour for the kind when
        // nothing is said.
        let colour = match &self.colour {
            Some(colour) => {
                let numbers = numbers_of(colour);
                if numbers.len() != 3 || numbers.iter().any(|number| !number.is_finite()) {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "the colour of a stamp is three numbers".to_string(),
                    });
                }
                if numbers.iter().any(|number| !(0.0..=1.0).contains(number)) {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "the colour of a stamp is between zero and one".to_string(),
                    });
                }
                [numbers[0], numbers[1], numbers[2]]
            }
            None => kind.colour,
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
        // The **annotation** itself: the tree's own stamp.
        let mut entries: Vec<(String, Json)> = vec![
            ("Type".to_string(), json!("Annot")),
            ("Subtype".to_string(), json!("Stamp")),
            ("Rect".to_string(), json!(rect)),
            ("C".to_string(), json!(colour)),
            ("Name".to_string(), json!(kind.name)),
            ("PCStamp".to_string(), json!(true)),
            ("P".to_string(), json!({ "ref": [page_number, 0] })),
        ];
        if let Some(by) = &self.by {
            entries.push(("PCByLine".to_string(), json!({ "string": by.0 })));
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
            "kind": kind.name,
            "label": kind.label,
            "group": match kind.group {
                Group::Business => "business",
                Group::SignHere => "signhere",
                Group::Dynamic => "dynamic",
            },
            "colour": colour,
        })))
    }
}
