//! The **file attachment** on a page: the tree's own attachment.
//!
//! The tree's own: a **file attachment** is an annotation that carries
//! a file with it. The file itself is an **embedded file** — a stream
//! of its own — and a **file specification** says what it is called
//! and where its bytes are.
//!
//! # The rules
//!
//! - The **name** of the attached file must not be empty: the tree's
//!   own rule, and a file with no name is a file nobody can find.
//! - The **point** is two numbers, and the annotation covers the box
//!   of twenty points that hangs from it: the tree's own box.
//! - The **icon** is one of the tree's own four — `PushPin`,
//!   `Paperclip`, `Graph` and `Tag` — and the pin is the tree's own
//!   when the caller says nothing.
//! - The **colour** of the attachment is the tree's own blue when the
//!   caller says nothing, and it is three numbers between zero and
//!   one.
//! - The **bytes** of the file are read and are **bounded**: the file
//!   is read whole or not at all, and the answer says how many bytes
//!   there were.
//! - The **embedded file** is a stream of its own, and it says its
//!   **type** and its **size**: the tree's own keys.
//! - The **file specification** is an object of its own, and it says
//!   the **name** of the file — the plain one and the Unicode one —
//!   and, under `/EF`, which object holds the bytes.
//! - The annotation **names** the file specification, and the file
//!   specification names the embedded file: the edges of the document
//!   say it as well as the dictionary.
//!
//! # What it does not settle
//!
//! - The **drawing** of the attachment is not here: it is the
//!   appearance command's own work.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Text};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.annot.attachment.add";

/// The tree's own **box** of an attachment: twenty points.
const SIZE: f64 = 20.0;

/// The tree's own **colour** of an attachment: the blue.
const COLOUR: [f64; 3] = [0.0, 0.47, 0.84];

/// The tree's own **flags** of an attachment: it prints, it does not
/// zoom and it does not turn.
const FLAGS: i64 = 28;

/// An **icon** of an attachment: the tree's own four.
pub struct Icon {
    /// The name the file gives it.
    pub name: &'static str,
}

/// The tree's own **four** icons.
pub const ICONS: [Icon; 4] =
    [Icon { name: "PushPin" }, Icon { name: "Paperclip" }, Icon { name: "Graph" }, Icon { name: "Tag" }];

/// The **icon** the caller names: one of the tree's own four.
pub fn icon_of(name: &str) -> Option<&'static Icon> {
    let name = name.trim();
    ICONS.iter().find(|icon| icon.name.eq_ignore_ascii_case(name))
}

/// The **numbers** a value holds.
fn numbers_of(value: &Json) -> Vec<f64> {
    value.as_array().map(|list| list.iter().filter_map(|item| item.as_f64()).collect()).unwrap_or_default()
}

commands! {
    /// **Put** a **file attachment** on a page.
    pdf.annot.attachment.add {
        area: "pdf",
        layer: "pdf",
        icon: "plus",
        mutates,
        undoable,
        params: {
            /// The page the attachment is on.

            page: NodeRef,
            /// The object number the annotation has in the file.

            object: Number,
            /// The point the attachment hangs from: two numbers.

            at: Json,
            /// The name of the attached file.

            name: Text,
            /// The file that is attached.

            path: Text,
            /// Where the bytes of the attachment are written.

            to: Text,
            /// The icon of the attachment: `PushPin`, `Paperclip`, `Graph` or
            /// `Tag`.

            icon: Option<Text>,
            /// The colour of the attachment: three numbers.

            colour: Option<Json>,
        },
        run: {

            // The **name** of the attached file: the tree's own rule.
            let name = self.name.0.trim().to_string();
            if name.is_empty() {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "the attached file needs a name".to_string(),
                });
            }
            // The **point**: two numbers.
            let numbers = numbers_of(&self.at);
            if numbers.len() != 2 || numbers.iter().any(|number| !number.is_finite()) {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "the point of the attachment is two numbers".to_string(),
                });
            }
            let at = [numbers[0], numbers[1]];
            // The **icon**: one of the tree's own four.
            let icon = match &self.icon {
                Some(icon) => {
                    let Some(icon) = icon_of(&icon.0) else {
                        return Err(EngineError::Failed {
                            id: COMMAND.to_string(),
                            message: format!(
                                "`{}` is not an icon of an attachment: they are PushPin, Paperclip, Graph and Tag",
                                icon.0.trim()
                            ),
                        });
                    };
                    icon.name
                }
                None => "PushPin",
            };
            // The **colour**: the tree's own blue when nothing is said.
            let colour = match &self.colour {
                Some(colour) => {
                    let numbers = numbers_of(colour);
                    if numbers.len() != 3 || numbers.iter().any(|number| !number.is_finite()) {
                        return Err(EngineError::Failed {
                            id: COMMAND.to_string(),
                            message: "the colour of an attachment is three numbers".to_string(),
                        });
                    }
                    if numbers.iter().any(|number| !(0.0..=1.0).contains(number)) {
                        return Err(EngineError::Failed {
                            id: COMMAND.to_string(),
                            message: "the colour of an attachment is between zero and one".to_string(),
                        });
                    }
                    [numbers[0], numbers[1], numbers[2]]
                }
                None => COLOUR,
            };
            // The **bytes** of the file: read whole or not at all.
            let source = self.path.0.trim();
            if source.is_empty() {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "the attached file has a path".to_string(),
                });
            }
            let limits = crate::filters::Limits::default();
            let bytes = craftkit_sdk::files::read_bounded(std::path::Path::new(source), limits.max_input as u64)
                .map_err(|message| EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: format!("the attached file cannot be read: {message}"),
                })?;
            // Where the bytes are written.
            let to = self.to.0.trim().to_string();
            if to.is_empty() {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "the bytes of the attachment go somewhere".to_string(),
                });
            }
            std::fs::write(&to, &bytes).map_err(|error| EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("the bytes of the attachment cannot be written to `{to}`: {error}"),
            })?;
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
            // The **rectangle**: the box of twenty points that hangs from
            // the point.
            let rect = [at[0], at[1] - SIZE, at[0] + SIZE, at[1]];
            // The **annotation** itself: the tree's own attachment.
            let entries: Vec<(String, Json)> = vec![
                ("Type".to_string(), json!("Annot")),
                ("Subtype".to_string(), json!("FileAttachment")),
                ("Rect".to_string(), json!(rect)),
                ("C".to_string(), json!(colour)),
                ("Name".to_string(), json!(icon)),
                ("F".to_string(), json!(FLAGS)),
                ("P".to_string(), json!({ "ref": [page_number, 0] })),
            ];
            let data = json!({ "object": number, "entries": entries });
            let annotation_name = format!("annotation {number}");
            let annot = session.document_mut()?.add_node_with(annotation_name.clone(), "pdf.object", None, data);
            // The **embedded file**: a stream of its own, and the numbers
            // of the document go after the caller's own.
            let file_number = crate::free::the_next_number(session.document(), 0);
            let file_entries: Vec<(String, Json)> = vec![
                ("Type".to_string(), json!("EmbeddedFile")),
                ("Params".to_string(), json!({ "entries": [ ["Size", bytes.len()] ] })),
            ];
            let file_data = json!({ "object": file_number, "entries": file_entries });
            let file = session.document_mut()?.add_node_with(
                format!("object {file_number}"),
                "pdf.stream",
                None,
                file_data,
            );
            if let Some(node) = session.document_mut()?.node_mut(file) {
                node.stream = Some(to.clone());
            }
            // The **file specification**: an object of its own.
            let spec_number = crate::free::the_next_number(session.document(), 0);
            let spec_entries: Vec<(String, Json)> = vec![
                ("Type".to_string(), json!("Filespec")),
                ("F".to_string(), json!({ "string": name.clone() })),
                ("UF".to_string(), json!({ "string": name.clone() })),
                (
                    "EF".to_string(),
                    json!({
                        "entries": [
                            ["F", { "ref": [file_number, 0] }],
                            ["UF", { "ref": [file_number, 0] }],
                        ],
                    }),
                ),
            ];
            let spec_data = json!({ "object": spec_number, "entries": spec_entries });
            let spec = session.document_mut()?.add_node_with(
                format!("object {spec_number}"),
                "pdf.object",
                None,
                spec_data,
            );
            // The annotation **names** the file specification.
            let mut entries = crate::entries_of(
                &session
                    .document()
                    .node(annot)
                    .ok_or_else(|| EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: format!("there is no node `{annot}`"),
                    })?
                    .data,
            );
            entries.push(("FS".to_string(), json!({ "ref": [spec_number, 0] })));
            let data = json!({
                "object": number,
                "entries": crate::pairs_of(&entries),
            });
            if let Some(node) = session.document_mut()?.node_mut(annot) {
                node.data = data;
            }
            // The page **names** the annotation.
            let mut annots = annots;
            annots.push(json!({ "ref": [number, 0] }));
            if let Some(node) = session.document_mut()?.node_mut(self.page.0) {
                node.data = crate::with_entry(&node.data, "Annots", json!(annots));
            }
            // And the **edges**: the page names the annotation, the
            // annotation names the file specification, and the file
            // specification names the embedded file.
            for (from, to, key) in [(self.page.0, annot, "Annots"), (annot, spec, "FS"), (spec, file, "EF")] {
                let relation =
                    craftkit_sdk::Relation::with_payload(from, to, "ref", json!({ "key": key, "generation": 0 }));
                if !session.document_mut()?.add_relation(relation) {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: format!("the object `{from}` already refers to `{to}` under `{key}`"),
                    });
                }
            }
            Ok(Outcome::changed(format!("added the `{annotation_name}` to the page")).with_data(json!({
                "id": annot,
                "object": number,
                "page": self.page.0,
                "subtype": "FileAttachment",
                "rect": rect,
                "at": at,
                "icon": icon,
                "colour": colour,
                "name": name,
                "path": source,
                "to": to,
                "bytes": bytes.len(),
                "file": file,
                "file_object": file_number,
                "specification": spec,
                "specification_object": spec_number,
            })))

        }
    }
}
