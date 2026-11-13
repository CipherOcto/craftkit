//! The **drop-down** that is put on a page.
//!
//! The tree's own: a **drop-down** is a field of the kind `Ch` that
//! offers a **list of options** and holds one of them — and it may be
//! **editable**, which means the reader may write something that is not
//! one of the options.
//!
//! The tree writes it with the conventions of Acrobat:
//!
//! - the **name** of the field is `Dropdown1`, `Dropdown2` and so on —
//!   the first name that is free;
//! - the field is a **drop-down** — the tree's own eighteenth bit of the
//!   flags of a field — and it may be **editable** as well, which is
//!   the tree's own nineteenth bit;
//! - the **options** are the **display texts** of the field, and the
//!   value it holds is the **export value** of the option — the tree's
//!   own rule; an option may carry an **export value** and a **display
//!   text** of its own, and the tree's own file holds the pair when the
//!   two are not the same;
//! - the field's text is **twelve point Helvetica** (`/DA`).
//!
//! The rules of the tree, ported by name:
//!
//! - `ensure_form` — the form is made when the file has none, and it
//!   holds the field;
//! - `free_name` — the first `<base>N` that is free;
//! - `widget_dict` — the field is an annotation of the kind `Widget`;
//! - `field_appearance` — the **drawing** of the field: the frame and
//!   the text it holds.

use craftkit_derive::commands;

use craftkit_sdk::serde_json;
use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, NodeRef, Number, Outcome, Text};

use crate::fieldadd::{a_node, ensure_form, entry, free_name, put, the_entries, the_frame};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.field.combo.add";

/// The **smallest** a drop-down may be: the tree's own rule, four
/// points by four.
const SMALLEST: f64 = 4.0;

/// The **flag** of a drop-down: the tree's own, the eighteenth bit of
/// the flags of a field.
pub const COMBO: i64 = 1 << 17;

/// The **flag** of a drop-down that may be **edited**: the tree's own,
/// the nineteenth bit of the flags of a field.
pub const EDIT: i64 = 1 << 18;

/// The **options** of a field: a list of texts.
///
/// The list comes from outside, so it is read and checked: a thing that
/// is not a text is refused, and not taken as an empty text.
///
/// The tree's own: an option is an **export value** and a **display
/// text**, and they are the same thing unless the caller says
/// otherwise — so an option may be a text, or a pair of two texts.
fn the_options(value: &serde_json::Value) -> Result<Vec<(String, String)>, EngineError> {
    let Some(list) = value.as_array() else {
        return Err(EngineError::Failed {
            id: COMMAND.to_string(),
            message: "the options of a drop-down are a list of texts".to_string(),
        });
    };
    let mut out: Vec<(String, String)> = Vec::with_capacity(list.len());
    for item in list {
        if let Some(text) = item.as_str() {
            out.push((text.to_string(), text.to_string()));
            continue;
        }
        // A **pair**: the export value and the display text.
        let Some(pair) = item.as_array() else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "the options of a drop-down are a list of texts".to_string(),
            });
        };
        if pair.len() != 2 {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "an option of a drop-down is a text or a pair of two texts".to_string(),
            });
        }
        let (Some(export), Some(display)) = (pair[0].as_str(), pair[1].as_str()) else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "an option of a drop-down is a text or a pair of two texts".to_string(),
            });
        };
        out.push((export.to_string(), display.to_string()));
    }
    Ok(out)
}

/// The **options** as the file holds them: a text when the export value
/// and the display text are the same, and a **pair** of the two when
/// they are not: the tree's own rule.
fn the_options_of_the_file(options: &[(String, String)]) -> Vec<Json> {
    options
        .iter()
        .map(|(export, display)| {
            if export == display {
                json!({ "string": display })
            } else {
                json!([{ "string": export }, { "string": display }])
            }
        })
        .collect()
}

/// The **drawing** of a drop-down: the frame and the text it holds.
///
/// The tree's own `field_appearance`: the text of a choice is the text
/// of the option that is chosen, and it is drawn as the text of a text
/// field is drawn.
fn the_drawing(rect: [f64; 4], value: &str) -> Option<Vec<u8>> {
    let width = rect[2] - rect[0];
    let height = rect[3] - rect[1];
    let mut out = the_frame(width, height);
    if value.is_empty() {
        return Some(out.into_bytes());
    }
    let text = crate::appearance::the_string(value)?;
    let size = 12.0;
    let y = (height - 0.925 * size) / 2.0 + 0.207 * size;
    out.push_str(&format!(
        "BT\n/Helv {} Tf\n0 g\n1 0 0 1 {} {} Tm\n({text}) Tj\nET\n",
        crate::watermark::number(size),
        crate::watermark::number(3.0),
        crate::watermark::number(y)
    ));
    Some(out.into_bytes())
}

commands! {
    /// **Put** a **drop-down** on a page.
    ///
    /// The field is a widget — an annotation of the kind `Widget` — and it
    /// is on the page the caller names, in the rectangle the caller gives.
    /// It offers the **options** the caller lists.
    ///
    /// The rules:
    ///
    /// - the **page** must be a page of the file;
    /// - the **rectangle** is four numbers, and the corners may come in
    ///   any order; a drop-down smaller than four points by four is
    ///   refused, as the tree refuses it;
    /// - the **options** are a list of texts, and a list that holds
    ///   something that is not a text is refused; an option may be a
    ///   **pair** of two texts — the export value and the display text;
    /// - the **name** is the first free `Dropdown` name when the caller
    ///   does not give one; a name with a period in it is refused, and so
    ///   is a name another field has;
    /// - the file's **form** is made when the file has none, and it holds
    ///   the field;
    /// - the **drawing** of the field is written when the caller says
    ///   where the bytes go.
    pdf.field.combo.add {
        area: "pdf",
        layer: "pdf",
        icon: "plus",
        mutates,
        undoable,
        params: {
            /// The page the drop-down is put on.
            page: NodeRef,
            /// The drop-down's object number in the file.
            object: Number,
            /// The rectangle the drop-down covers: four numbers.
            rect: serde_json::Value,
            /// The name of the drop-down.
            ///
            /// Leave it out and the drop-down is given the first free name of
            /// the tree: `Dropdown1`, `Dropdown2` and so on.
            name: Option<Text>,
            /// The **options** the drop-down offers: a list of texts, or of
            /// pairs of two texts — the **export value** and the **display
            /// text** of the option.
            options: serde_json::Value,
            /// Whether the reader may write something that is not one of the
            /// options.
            editable: Option<Bool>,
            /// The text the drop-down holds.
            value: Option<Text>,
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
                    message: "the drop-down is too small".to_string(),
                });
            }
            // The **options**.
            let options = the_options(&self.options)?;
            // The **name**.
            let existing: Vec<String> =
                crate::fields::fields_of(session.document()).into_iter().map(|field| field.name).collect();
            let name = match self.name.as_ref() {
                Some(text) => {
                    let name = text.0.trim().to_string();
                    if name.is_empty() {
                        free_name(&existing, "Dropdown")
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
                None => free_name(&existing, "Dropdown"),
            };
            if existing.contains(&name) {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: format!("a field named `{name}` already exists"),
                });
            }
            // The **value** the field holds: it must be one of the options,
            // unless the field may be edited — and then anything may be
            // written in it. The tree's own rule: the caller may give the
            // **export value** or the **display text**, and the field holds
            // the export value.
            let editable = self.editable.as_ref().map(|flag| flag.0).unwrap_or(false);
            let given = self.value.as_ref().map(|value| value.0.clone()).unwrap_or_default();
            let value = if given.is_empty() {
                String::new()
            } else {
                let found = options.iter().find(|(export, display)| *export == given || *display == given);
                match found {
                    Some((export, _)) => export.clone(),
                    None if editable => given.clone(),
                    None => {
                        return Err(EngineError::Failed {
                            id: COMMAND.to_string(),
                            message: format!("`{given}` is not one of the options of the drop-down"),
                        });
                    }
                }
            };
            // The **display text** of the option that is chosen: the tree's
            // own rule — the reader sees the display text and the file
            // holds the export value.
            let display = options
                .iter()
                .find(|(export, _)| *export == value)
                .map(|(_, display)| display.clone())
                .unwrap_or_else(|| value.clone());
            // The **drawing**: the bytes are made before the document is
            // changed, so that a text that cannot be written is refused and
            // nothing is changed.
            let mut drawing = None;
            if let Some(to) = &self.appearance_to {
                let path = to.0.trim().to_string();
                if path.is_empty() {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "the drawing goes somewhere".to_string(),
                    });
                }
                let Some(bytes) = the_drawing(rect, &display) else {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "the text of the drop-down cannot be written".to_string(),
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
            // The **drop-down** itself: the tree's `widget_dict`, with the
            // choice's own entries.
            let mut entries: Vec<(String, Json)> = vec![
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
                ("FT".to_string(), json!("Ch")),
                ("DA".to_string(), json!({ "string": "/Helv 12 Tf 0 g" })),
                ("Opt".to_string(), json!(the_options_of_the_file(&options))),
                ("T".to_string(), json!({ "string": name.clone() })),
            ];
            let mut flags = COMBO;
            if editable {
                flags |= EDIT;
            }
            entries.push(("Ff".to_string(), json!(flags)));
            if !value.is_empty() {
                entries.push(("V".to_string(), json!({ "string": value.clone() })));
            }
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
            // The **drawing** of the field.
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
            Ok(Outcome::changed(format!("added the drop-down `{name}` to the page")).with_data(json!({
                "id": widget,
                "object": number,
                "page": self.page.0,
                "name": name,
                "rect": rect,
                "options": options.iter().map(|(_, display)| display.clone()).collect::<Vec<_>>(),
                "exports": options.iter().map(|(export, _)| export.clone()).collect::<Vec<_>>(),
                "display": display,
                "editable": editable,
                "value": value,
                "form": form.node,
                "appearance": appearance,
            })))

        }
    }
}
