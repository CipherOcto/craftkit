//! The **push button** that is put on a page.
//!
//! The tree's own: a **push button** is a field of the kind `Btn` that
//! does nothing by itself — it carries an **action**, which is a thing
//! this tree does not make here — and it shows a **caption**.
//!
//! The tree writes it with the conventions of Acrobat:
//!
//! - the **name** of the field is `Button1`, `Button2` and so on —
//!   the first name that is free;
//! - the field is a **push button** — the tree's own seventeenth bit of
//!   the flags of a field;
//! - the **caption** is in the **caption** of the field (`/MK /CA`),
//!   and the background is the tree's own **light grey**;
//! - the caption is **centred** in the button, and its size is the
//!   tree's own: six tenths of the height of the button, and never
//!   less than four points and never more than fourteen.
//!
//! The rules of the tree, ported by name:
//!
//! - `ensure_form` — the form is made when the file has none, and it
//!   holds the field;
//! - `free_name` — the first `<base>N` that is free;
//! - `widget_dict` — the field is an annotation of the kind `Widget`;
//! - `button_appearance` — the **drawing** of the field: the frame and
//!   the caption, centred.

use craftkit_derive::commands;

use craftkit_sdk::serde_json;
use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Text};

use crate::fieldadd::{a_node, ensure_form, entry, free_name, put, the_entries, the_frame};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.field.button.add";

/// The **smallest** a button may be: the tree's own rule, four points
/// by four.
const SMALLEST: f64 = 4.0;

/// The **flag** of a push button: the tree's own, the seventeenth bit
/// of the flags of a field.
pub const PUSH_BUTTON: i64 = 1 << 16;

/// The **background** of a push button: the tree's own, a light grey.
const BACKGROUND: [f64; 3] = [0.86, 0.86, 0.86];

/// The **drawing** of a push button: the frame and the caption.
///
/// The tree's own `button_appearance`: the caption is **centred** in
/// the box, and its size is six tenths of the height of the box, and
/// never less than four points and never more than fourteen.
fn the_drawing(rect: [f64; 4], caption: &str) -> Option<Vec<u8>> {
    let width = rect[2] - rect[0];
    let height = rect[3] - rect[1];
    let mut out = the_frame(width, height);
    if caption.is_empty() {
        return Some(out.into_bytes());
    }
    let text = crate::appearance::the_string(caption)?;
    let size = ((height - 4.0) * 0.6).clamp(4.0, 14.0);
    let text_width = crate::watermark::helvetica_width(caption, size);
    out.push_str(&format!(
        "BT\n/Helv {} Tf\n0 g\n1 0 0 1 {} {} Tm\n({text}) Tj\nET\n",
        crate::watermark::number(size),
        crate::watermark::number((width - text_width) / 2.0),
        crate::watermark::number((height - size * 0.7) / 2.0)
    ));
    Some(out.into_bytes())
}

commands! {
    /// **Put** a **push button** on a page.
    ///
    /// The field is a widget — an annotation of the kind `Widget` — and it
    /// is on the page the caller names, in the rectangle the caller gives.
    /// It carries a **caption**, and the caption is what its drawing shows.
    ///
    /// The rules:
    ///
    /// - the **page** must be a page of the file;
    /// - the **rectangle** is four numbers, and the corners may come in
    ///   any order; a button smaller than four points by four is refused,
    ///   as the tree refuses it;
    /// - the **name** is the first free `Button` name when the caller does
    ///   not give one; a name with a period in it is refused, and so is a
    ///   name another field has;
    /// - the file's **form** is made when the file has none, and it holds
    ///   the field;
    /// - the **drawing** of the button is written when the caller says
    ///   where the bytes go.
    pdf.field.button.add {
        area: "pdf",
        layer: "pdf",
        icon: "plus",
        mutates,
        undoable,
        params: {
            /// The page the button is put on.
            page: NodeRef,
            /// The button's object number in the file.
            object: Number,
            /// The rectangle the button covers: four numbers.
            rect: serde_json::Value,
            /// The name of the button.
            ///
            /// Leave it out and the button is given the first free name of the
            /// tree: `Button1`, `Button2` and so on.
            name: Option<Text>,
            /// The **caption** of the button: the text it shows.
            caption: Text,
            /// Where the bytes of the button's drawing are written.
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
            // The **object number** of the button.
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
                    message: "the button is too small".to_string(),
                });
            }
            // The **name**.
            let existing: Vec<String> =
                crate::fields::fields_of(session.document()).into_iter().map(|field| field.name).collect();
            let name = match self.name.as_ref() {
                Some(text) => {
                    let name = text.0.trim().to_string();
                    if name.is_empty() {
                        free_name(&existing, "Button")
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
                None => free_name(&existing, "Button"),
            };
            if existing.contains(&name) {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: format!("a field named `{name}` already exists"),
                });
            }
            // The **caption** and the **drawing**: the bytes are made before
            // the document is changed, so that a caption that cannot be
            // written is refused and nothing is changed.
            let caption = self.caption.0.clone();
            let mut drawing = None;
            if let Some(to) = &self.appearance_to {
                let path = to.0.trim().to_string();
                if path.is_empty() {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "the drawing goes somewhere".to_string(),
                    });
                }
                let Some(bytes) = the_drawing(rect, &caption) else {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "the caption of the button cannot be written".to_string(),
                    });
                };
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
            // The **button** itself: the tree's `widget_dict`, with the
            // push button's own entries.
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
                            ["BG", json!(BACKGROUND)],
                            ["CA", json!({ "string": caption.clone() })],
                        ]
                    }),
                ),
                ("BS".to_string(), json!({ "entries": [ ["W", 1], ["S", "S"] ] })),
                ("FT".to_string(), json!("Btn")),
                ("Ff".to_string(), json!(PUSH_BUTTON)),
                ("DA".to_string(), json!({ "string": "/Helv 0 Tf 0 g" })),
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
            // The **drawing** of the button.
            let mut appearance = None;
            if let Some((path, bytes)) = drawing {
                std::fs::write(&path, &bytes).map_err(|error| EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: format!("the drawing cannot be written to `{path}`: {error}"),
                })?;
                let font = form
                    .fonts
                    .iter()
                    .find(|(name, _)| name == "Helv")
                    .map(|(_, value)| value.clone())
                    .unwrap_or(Json::Null);
                let resources = json!({ "entries": [ ["Font", { "entries": [ ["Helv", font] ] } ] ] });
                let entries = vec![
                    ("Type".to_string(), json!("XObject")),
                    ("Subtype".to_string(), json!("Form")),
                    ("BBox".to_string(), json!([0.0, 0.0, width, height])),
                    ("Resources".to_string(), resources),
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
            Ok(Outcome::changed(format!("added the button `{name}` to the page")).with_data(json!({
                "id": widget,
                "object": number,
                "page": self.page.0,
                "name": name,
                "caption": caption,
                "rect": rect,
                "form": form.node,
                "appearance": appearance,
            })))

        }
    }
}
