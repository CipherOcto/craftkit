//! The **signature field** that is put on a page.
//!
//! The tree's own: a **signature field** is a field of the kind `Sig`
//! and it is a box on a page that the reader **signs** — a signature is
//! the reader's own mark, and this tree does not make one.
//!
//! The tree writes it with the conventions of Acrobat:
//!
//! - the **name** of the field is `Signature1`, `Signature2` and so on
//!   — the first name that is free;
//! - the field is a field of the kind `Sig`;
//! - the field has a **thin grey border** and a **white background**,
//!   and its **drawing** is the empty box — the frame and nothing else,
//!   so that the reader may sign in it.
//!
//! The rules of the tree, ported by name:
//!
//! - `ensure_form` — the form is made when the file has none, and it
//!   holds the field;
//! - `free_name` — the first `<base>N` that is free;
//! - `widget_dict` — the field is an annotation of the kind `Widget`;
//! - `empty_box` — the **drawing** of the field: the frame.

use craftkit_derive::commands;

use craftkit_sdk::serde_json;
use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Text};

use crate::fieldadd::{a_node, ensure_form, entry, free_name, put, the_entries, the_frame};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.field.signature.add";

/// The **smallest** a signature field may be: the tree's own rule, four
/// points by four.
const SMALLEST: f64 = 4.0;

commands! {
    /// **Put** a **signature field** on a page.
    ///
    /// The field is a widget — an annotation of the kind `Widget` — and it
    /// is on the page the caller names, in the rectangle the caller gives.
    /// Its **drawing** is the empty box the reader signs in.
    ///
    /// The rules:
    ///
    /// - the **page** must be a page of the file;
    /// - the **rectangle** is four numbers, and the corners may come in
    ///   any order; a field smaller than four points by four is refused,
    ///   as the tree refuses it;
    /// - the **name** is the first free `Signature` name when the caller
    ///   does not give one; a name with a period in it is refused, and so
    ///   is a name another field has;
    /// - the file's **form** is made when the file has none, and it holds
    ///   the field;
    /// - the **drawing** of the field is written when the caller says
    ///   where the bytes go.
    pdf.field.signature.add {
        area: "pdf",
        layer: "pdf",
        icon: "plus",
        mutates,
        undoable,
        params: {
            /// The page the signature field is put on.
            page: NodeRef,
            /// The signature field's object number in the file.
            object: Number,
            /// The rectangle the signature field covers: four numbers.
            rect: serde_json::Value,
            /// The name of the signature field.
            ///
            /// Leave it out and the signature field is given the first free
            /// name of the tree: `Signature1`, `Signature2` and so on.
            name: Option<Text>,
            /// Where the bytes of the field's drawing are written.
            appearance_to: Option<Text>,
        },
        run: {
            // The **page**.
            let page = session.document().node(self.page.0).ok_or_else(|| EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("there is no node `{}`", self.page.0),
            })?;
            if page.kind != "pdf.object" && page.kind != "pdf.stream" {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: format!("the node `{}` is a `{}` and not an object", self.page.0, page.kind),
                });
            }
            let is_page = crate::entries_of(&page.data)
                .iter()
                .any(|(key, value)| key == "Type" && value.as_str() == Some("Page"));
            if !is_page {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: format!("the node `{}` is not a page", self.page.0),
                });
            }
            let page_number = page.data.get("object").and_then(|number| number.as_u64()).ok_or_else(|| {
                EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: format!("the node `{}` has no object number", self.page.0),
                }
            })?;
            // The **object number** of the field.
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
            // The **rectangle**: the corners may come in any order.
            let rect = crate::the_rect(&self.rect, COMMAND)?;
            let width = rect[2] - rect[0];
            let height = rect[3] - rect[1];
            if width < SMALLEST || height < SMALLEST {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "the signature field is too small".to_string(),
                });
            }
            // The **name**.
            let existing: Vec<String> =
                crate::fields::fields_of(session.document()).into_iter().map(|field| field.name).collect();
            let name = match self.name.as_ref() {
                Some(text) => {
                    let name = text.0.trim().to_string();
                    if name.is_empty() {
                        free_name(&existing, "Signature")
                    } else {
                        if name.contains('.') {
                            return Err(EngineError::Failed {
                                id: COMMAND.to_string(),
                                message: "a field name cannot hold a period".to_string(),
                            });
                        }
                        name
                    }
                }
                None => free_name(&existing, "Signature"),
            };
            if existing.contains(&name) {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: format!("a field named `{name}` already exists"),
                });
            }
            // The **drawing**: the empty box, the frame and nothing else.
            let mut drawing = None;
            if let Some(to) = &self.appearance_to {
                let path = to.0.trim().to_string();
                if path.is_empty() {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "the drawing goes somewhere".to_string(),
                    });
                }
                let bytes = the_frame(width, height).into_bytes();
                drawing = Some((path, bytes));
            }
            // The **form**: it holds the field, and it is made when the
            // file has none.
            let form = ensure_form(session)?;
            // The **object number** of the field: a number nothing else has.
            if session
                .document()
                .walk()
                .any(|node| node.data.get("object").and_then(|held| held.as_u64()) == Some(number))
            {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: format!("the object {number} is already in the document"),
                });
            }
            // The **signature field** itself: the tree's `widget_dict`,
            // with the signature field's own entries.
            let entries: Vec<(String, Json)> = vec![
                ("Type".to_string(), json!("Annot")),
                ("Subtype".to_string(), json!("Widget")),
                ("Rect".to_string(), json!(rect)),
                ("P".to_string(), json!({ "ref": [page_number, 0] })),
                ("F".to_string(), json!(4)),
                (
                    "MK".to_string(),
                    json!({
                        "entries": [
                            ["BC", json!([0.55, 0.55, 0.55])],
                            ["BG", json!([1.0, 1.0, 1.0])],
                        ]
                    }),
                ),
                ("BS".to_string(), json!({ "entries": [ ["W", 1], ["S", "S"] ] })),
                ("FT".to_string(), json!("Sig")),
                ("T".to_string(), json!({ "string": name.clone() })),
            ];
            let data = json!({ "object": number, "entries": crate::pairs_of(&entries) });
            let widget =
                session.document_mut()?.add_node_with(format!("field {number}"), "pdf.object", None, data);
            // The number of the **drawing**: the stream that holds it.
            let appearance_number = match &drawing {
                Some(_) => crate::free::the_next_number(session.document(), 0),
                None => 0,
            };
            // The **form** holds the field, and so does the **page**.
            let mut fields = entry(&the_entries(session, form.node), "Fields")
                .and_then(|fields| fields.as_array().cloned())
                .unwrap_or_default();
            fields.push(json!({ "ref": [number, 0] }));
            put(session, form.node, "Fields", json!(fields))?;
            let mut annots = entry(&the_entries(session, self.page.0), "Annots")
                .and_then(|annots| annots.as_array().cloned())
                .unwrap_or_default();
            annots.push(json!({ "ref": [number, 0] }));
            put(session, self.page.0, "Annots", json!(annots))?;
            // The **edges**: the page holds the field, and the form holds
            // it too.
            for (from, to, key) in [(self.page.0, widget, "Annots"), (form.node, widget, "Fields")] {
                let relation =
                    craftkit_sdk::Relation::with_payload(from, to, "ref", json!({ "key": key, "generation": 0 }));
                if !session.document_mut()?.add_relation(relation) {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: format!("the object `{from}` already refers to `{to}` under `{key}`"),
                    });
                }
            }
            // The **drawing** of the field: the empty box.
            let mut appearance = None;
            if let Some((path, bytes)) = drawing {
                std::fs::write(&path, &bytes).map_err(|error| EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: format!("the drawing cannot be written to `{path}`: {error}"),
                })?;
                let entries = vec![
                    ("Type".to_string(), json!("XObject")),
                    ("Subtype".to_string(), json!("Form")),
                    ("BBox".to_string(), json!([0.0, 0.0, width, height])),
                ];
                let stream = a_node(session, appearance_number, "pdf.stream", entries)?;
                if let Some(node) = session.document_mut()?.node_mut(stream) {
                    node.stream = Some(path.clone());
                }
                put(session, widget, "AP", json!({ "entries": [ ["N", { "ref": [appearance_number, 0] } ] ] }))?;
                let relation = craftkit_sdk::Relation::with_payload(
                    widget,
                    stream,
                    "ref",
                    json!({ "key": "AP", "generation": 0 }),
                );
                if !session.document_mut()?.add_relation(relation) {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: format!("the field `{widget}` already refers to its drawing"),
                    });
                }
                appearance = Some(stream);
            }
            Ok(Outcome::changed(format!("added the signature field `{name}` to the page")).with_data(json!({
                "id": widget,
                "object": number,
                "page": self.page.0,
                "name": name,
                "rect": rect,
                "form": form.node,
                "appearance": appearance,
            })))

        }
    }
}
