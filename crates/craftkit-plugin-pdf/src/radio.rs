//! The **radio button** that is put on a page.
//!
//! The tree's own: a **radio button** is a field of the kind `Btn` and
//! it is one of a **group** of them — the buttons of the group are
//! **exclusive**, and one of them is on at a time.
//!
//! The tree writes it with the conventions of Acrobat:
//!
//! - the **group** is a field of its own, with the name of the group
//!   — `Group1`, `Group2` and so on when the caller does not name it —
//!   and it holds the buttons as its **kids**;
//! - the group is **off** (`/V` is `Off`), it is **not** a toggle —
//!   the tree's own two flags, the radio flag and the flag that says
//!   the group cannot be turned off — and its default appearance is
//!   the tree's own, `/ZaDb 0 Tf 0 g`;
//! - each **button** is a widget — an annotation of the kind `Widget` —
//!   that names the group it belongs to (`/Parent`), and it is **off**
//!   as well (`/AS` is `Off`);
//! - each button has an **export value** — the value it holds when it
//!   is on — and the value may not be empty and may not be `Off`;
//! - the **mark** of a radio button is a **circle**, and the tree
//!   draws it as a path, so that no font of the file is needed.
//!
//! The rules of the tree, ported by name:
//!
//! - `ensure_form` — the form is made when the file has none, and it
//!   holds the **group** and not the buttons;
//! - `free_name` — the first `<base>N` that is free;
//! - `widget_dict` — the button is an annotation of the kind `Widget`;
//! - `check_box_states` — the **two** drawings of the button: the one
//!   that says it is off, and the one that says it is on.

use craftkit_derive::commands;

use craftkit_sdk::serde_json;
use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Text};

use crate::checkbox::{CheckStyle, the_mark};
use crate::fieldadd::{a_node, ensure_form, entry, free_name, put, the_entries, the_frame};
use crate::fields::{NO_TOGGLE_TO_OFF, RADIO};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.field.radio.add";

/// The **smallest** a radio button may be: the tree's own rule, four
/// points by four.
const SMALLEST: f64 = 4.0;

/// The **state** a radio button is off in: the tree's own.
const OFF: &str = "Off";

commands! {
    /// **Put** a **radio button** on a page.
    ///
    /// The button is a widget — an annotation of the kind `Widget` — and it
    /// is on the page the caller names, in the rectangle the caller gives.
    /// It belongs to a **group**, and the group is made when the file has
    /// no group of that name.
    ///
    /// The rules:
    ///
    /// - the **page** must be a page of the file;
    /// - the **rectangle** is four numbers, and the corners may come in
    ///   any order; a button smaller than four points by four is refused,
    ///   as the tree refuses it;
    /// - the **export value** — the value the button holds when it is on —
    ///   may not be empty and may not be `Off`, as the tree refuses it;
    /// - the **group** is named by the caller, or it is given the first
    ///   free name of the tree: `Group1`, `Group2` and so on; a name with
    ///   a period in it is refused, and so is a name a field that is not
    ///   a group of radio buttons has;
    /// - the file's **form** is made when the file has none, and it holds
    ///   the **group** and not the button;
    /// - the **two drawings** of the button are written when the caller
    ///   says where they go.
    pdf.field.radio.add {
        area: "pdf",
        layer: "pdf",
        icon: "plus",
        mutates,
        undoable,
        params: {
            /// The page the radio button is put on.
            page: NodeRef,
            /// The radio button's object number in the file.
            object: Number,
            /// The rectangle the radio button covers: four numbers.
            rect: serde_json::Value,
            /// The name of the **group** the button belongs to.
            ///
            /// Leave it out and the group is given the first free name of the
            /// tree: `Group1`, `Group2` and so on.
            group: Option<Text>,
            /// The **export value**: what the button holds when it is on.
            export: Text,
            /// The **style** of the mark: `Check`, `Circle`, `Cross`,
            /// `Diamond`, `Square` or `Star`.
            ///
            /// Leave it out and the mark is a circle, as the tree's is.
            style: Option<Text>,
            /// Where the bytes of the drawing that says the button is **on**
            /// are written.
            on_to: Option<Text>,
            /// Where the bytes of the drawing that says the button is **off**
            /// are written.
            off_to: Option<Text>,
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
                    message: "the radio button is too small".to_string(),
                });
            }
            // The **export value**: what the button holds when it is on.
            let export = self.export.0.trim().to_string();
            if export.is_empty() || export == OFF {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "a radio button needs an export value other than Off".to_string(),
                });
            }
            // The **style** of the mark: a circle, as the tree's is.
            let style = match &self.style {
                Some(text) => CheckStyle::parse(&text.0).ok_or_else(|| EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: format!(
                        "`{}` is not a style of a mark; the tree's own are Check, Circle, Cross, Diamond, Square and Star",
                        text.0
                    ),
                })?,
                None => CheckStyle::Circle,
            };
            // The **group**: the group the button belongs to, and the
            // fields the file already holds.
            let existing: Vec<String> =
                crate::fields::fields_of(session.document()).into_iter().map(|field| field.name).collect();
            let group_name = match self.group.as_ref() {
                Some(text) => {
                    let name = text.0.trim().to_string();
                    if name.is_empty() {
                        free_name(&existing, "Group")
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
                None => free_name(&existing, "Group"),
            };
            // A group that is already there is joined; a field that is not
            // a group of radio buttons and has that name is refused.
            let group = crate::fields::fields_of(session.document())
                .into_iter()
                .find(|field| field.name == group_name)
                .map(|field| (field.object, (field.flags as i64) & RADIO != 0));
            if let Some((_, is_radio)) = group
                && !is_radio
            {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: format!("a field named `{group_name}` already exists, and it is not a group"),
                });
            }
            // The **two drawings**: the bytes are made before the document
            // is changed, so that a drawing that cannot be written is
            // refused and nothing is changed.
            let mut drawings = None;
            match (&self.on_to, &self.off_to) {
                (Some(on), Some(off)) => {
                    let on = on.0.trim().to_string();
                    let off = off.0.trim().to_string();
                    if on.is_empty() || off.is_empty() {
                        return Err(EngineError::Failed {
                            id: COMMAND.to_string(),
                            message: "the drawings go somewhere".to_string(),
                        });
                    }
                    let frame = the_frame(width, height);
                    let mark = the_mark(style, width / 2.0, height / 2.0, width.min(height));
                    drawings = Some((on, off, format!("{frame}{mark}"), frame));
                }
                (None, None) => {}
                _ => {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "a radio button has two drawings, and both are written or neither".to_string(),
                    });
                }
            }
            // The **form**: it holds the group, and it is made when the
            // file has none.
            let form = ensure_form(session)?;
            // The **object number** of the button: a number nothing else
            // has.
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
            // The **group**: the node of the group, when the file has one
            // of that name, and the number of a new one.
            let (group_node, group_number) = match group {
                Some((object, _)) => {
                    let node = session
                        .document()
                        .walk()
                        .find(|node| node.data.get("object").and_then(|held| held.as_u64()) == Some(object))
                        .map(|node| node.id)
                        .ok_or_else(|| EngineError::Failed {
                            id: COMMAND.to_string(),
                            message: format!("the group `{group_name}` is not in the file"),
                        })?;
                    (node, object)
                }
                None => (0, crate::free::the_next_number(session.document(), 0)),
            };
            // The **button** itself: the tree's `widget_dict`, with the
            // radio button's own entries.
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
                            ["CA", json!({ "string": style.code() })],
                        ]
                    }),
                ),
                ("BS".to_string(), json!({ "entries": [ ["W", 1], ["S", "S"] ] })),
                ("AS".to_string(), json!(OFF)),
            ];
            let data = json!({ "object": number, "entries": crate::pairs_of(&entries) });
            let widget =
                session.document_mut()?.add_node_with(format!("field {number}"), "pdf.object", None, data);
            // The **group**: a new one is made when the file has none of
            // that name, and it holds the button as a kid.
            let made = group_node == 0;
            let group_node = if group_node == 0 {
                let entries: Vec<(String, Json)> = vec![
                    ("FT".to_string(), json!("Btn")),
                    ("Ff".to_string(), json!(RADIO | NO_TOGGLE_TO_OFF)),
                    ("T".to_string(), json!({ "string": group_name.clone() })),
                    ("V".to_string(), json!(OFF)),
                    ("DA".to_string(), json!({ "string": "/ZaDb 0 Tf 0 g" })),
                    ("Kids".to_string(), json!([])),
                ];
                let data = json!({ "object": group_number, "entries": crate::pairs_of(&entries) });
                let node = session.document_mut()?.add_node_with(
                    format!("field {group_number}"),
                    "pdf.object",
                    None,
                    data,
                );
                // The **form** holds the group.
                let mut fields = entry(&the_entries(session, form.node), "Fields")
                    .and_then(|fields| fields.as_array().cloned())
                    .unwrap_or_default();
                fields.push(json!({ "ref": [group_number, 0] }));
                put(session, form.node, "Fields", json!(fields))?;
                node
            } else {
                group_node
            };
            // The button **names the group**, and the group holds the
            // button.
            put(session, widget, "Parent", json!({ "ref": [group_number, 0] }))?;
            let mut kids = entry(&the_entries(session, group_node), "Kids")
                .and_then(|kids| kids.as_array().cloned())
                .unwrap_or_default();
            kids.push(json!({ "ref": [number, 0] }));
            put(session, group_node, "Kids", json!(kids))?;
            // The **page** holds the button as an annotation.
            let mut annots = entry(&the_entries(session, self.page.0), "Annots")
                .and_then(|annots| annots.as_array().cloned())
                .unwrap_or_default();
            annots.push(json!({ "ref": [number, 0] }));
            put(session, self.page.0, "Annots", json!(annots))?;
            // The **edges**: the page holds the button, the form holds the
            // group, the group holds the button, and the button names the
            // group.
            let mut edges: Vec<(u64, u64, &str)> = vec![(self.page.0, widget, "Annots")];
            if made {
                // The form holds the group, and it holds it once.
                edges.push((form.node, group_node, "Fields"));
            }
            edges.push((group_node, widget, "Kids"));
            edges.push((widget, group_node, "Parent"));
            for (from, to, key) in edges {
                let relation =
                    craftkit_sdk::Relation::with_payload(from, to, "ref", json!({ "key": key, "generation": 0 }));
                if !session.document_mut()?.add_relation(relation) {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: format!("the object `{from}` already refers to `{to}` under `{key}`"),
                    });
                }
            }
            // The **two drawings** of the button: the one that says it is
            // off, and the one that says it is on — the on one is called
            // the export value of the button.
            let mut appearance = None;
            let mut numbers: Vec<u64> = Vec::new();
            if let Some((on, off, on_bytes, off_bytes)) = drawings {
                let mut states: Vec<(String, Json)> = Vec::new();
                let mut streams: Vec<(String, u64)> = Vec::new();
                let first = crate::free::the_next_number(session.document(), 0);
                let second = crate::free::the_next_number(session.document(), first);
                numbers.push(first);
                numbers.push(second);
                let drawings = [(export.clone(), on, on_bytes), (OFF.to_string(), off, off_bytes)];
                for (index, (state, path, bytes)) in drawings.into_iter().enumerate() {
                    std::fs::write(&path, &bytes).map_err(|error| EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: format!("the drawing cannot be written to `{path}`: {error}"),
                    })?;
                    let number = numbers[index];
                    let entries = vec![
                        ("Type".to_string(), json!("XObject")),
                        ("Subtype".to_string(), json!("Form")),
                        ("BBox".to_string(), json!([0.0, 0.0, width, height])),
                    ];
                    let stream = a_node(session, number, "pdf.stream", entries)?;
                    if let Some(node) = session.document_mut()?.node_mut(stream) {
                        node.stream = Some(path.clone());
                    }
                    states.push((state.clone(), json!({ "ref": [number, 0] })));
                    streams.push((state.clone(), stream));
                    let relation = craftkit_sdk::Relation::with_payload(
                        widget,
                        stream,
                        "ref",
                        json!({ "key": "AP", "state": state, "generation": 0 }),
                    );
                    if !session.document_mut()?.add_relation(relation) {
                        return Err(EngineError::Failed {
                            id: COMMAND.to_string(),
                            message: format!("the button `{widget}` already refers to a drawing"),
                        });
                    }
                }
                put(session, widget, "AP", json!({ "entries": [ ["N", { "entries": states } ] ] }))?;
                appearance = Some(
                    streams
                        .into_iter()
                        .map(|(state, stream)| (state, json!(stream)))
                        .collect::<serde_json::Map<String, Json>>(),
                );
            }
            Ok(Outcome::changed(format!("added the radio button `{export}` to the page")).with_data(json!({
                "id": widget,
                "object": number,
                "page": self.page.0,
                "group": group_name,
                "group_id": group_node,
                "group_object": group_number,
                "export": export,
                "rect": rect,
                "style": style.label(),
                "code": style.code(),
                "form": form.node,
                "appearance": appearance,
                "drawings": numbers,
            })))

        }
    }
}
