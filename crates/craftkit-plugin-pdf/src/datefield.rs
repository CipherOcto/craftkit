//! The **date field** that is put on a page.
//!
//! The tree's own: a **date field** is a text field that holds a day,
//! and the reader is helped with it by the **actions** of Acrobat —
//! the format of the day and the keystrokes that are allowed — which
//! are **JavaScript** that only Acrobat's own engine runs.
//!
//! The tree writes it with the conventions of Acrobat:
//!
//! - the **name** of the field is `Date1`, `Date2` and so on — the
//!   first name that is free;
//! - the field is a text field (`/FT /Tx`) of **twelve point
//!   Helvetica** (`/DA`);
//! - the field carries **two additional actions** (`/AA`): the one
//!   that **formats** the day and the one that **keystrokes** it, and
//!   both of them are the tree's own scripts, and both of them name
//!   the tree's own format — `mm/dd/yyyy`.
//!
//! The rules of the tree, ported by name:
//!
//! - `ensure_form` — the form is made when the file has none, and it
//!   holds the field;
//! - `free_name` — the first `<base>N` that is free;
//! - `widget_dict` — the field is an annotation of the kind `Widget`;
//! - `field_appearance` — the **drawing** of the field: the frame and
//!   the text it holds.

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session, Text};

use crate::fieldadd::{a_node, ensure_form, entry, free_name, put, the_entries};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.field.date.add";

/// The **smallest** a date field may be: the tree's own rule, four
/// points by four.
const SMALLEST: f64 = 4.0;

/// The **format** of the day: the tree's own, month, day and year.
const FORMAT: &str = "mm/dd/yyyy";

/// The **script** of the tree that **formats** the day.
fn the_format_script() -> String {
    format!("AFDate_FormatEx(\"{FORMAT}\");")
}

/// The **script** of the tree that **keystrokes** the day.
fn the_keystroke_script() -> String {
    format!("AFDate_KeystrokeEx(\"{FORMAT}\");")
}

/// **Put** a **date field** on a page.
///
/// The field is a widget — an annotation of the kind `Widget` — and it
/// is on the page the caller names, in the rectangle the caller gives.
/// It is a text field that carries the tree's own **actions**, which
/// format the day and check the keys that are typed into it.
///
/// The rules:
///
/// - the **page** must be a page of the file;
/// - the **rectangle** is four numbers, and the corners may come in
///   any order; a field smaller than four points by four is refused,
///   as the tree refuses it;
/// - the **name** is the first free `Date` name when the caller does
///   not give one; a name with a period in it is refused, and so is a
///   name another field has;
/// - the file's **form** is made when the file has none, and it holds
///   the field;
/// - the **drawing** of the field is written when the caller says
///   where the bytes go.
#[derive(Command)]
#[command(id = "pdf.field.date.add", area = "pdf", layer = "pdf", icon = "plus", mutates, undoable)]
pub struct AddDate {
    /// The page the date field is put on.
    pub page: NodeRef,
    /// The date field's object number in the file.
    pub object: Number,
    /// The rectangle the date field covers: four numbers.
    pub rect: serde_json::Value,
    /// The name of the date field.
    ///
    /// Leave it out and the date field is given the first free name of
    /// the tree: `Date1`, `Date2` and so on.
    pub name: Option<Text>,
    /// The day the field holds.
    pub value: Option<Text>,
    /// Where the bytes of the field's drawing are written.
    pub appearance_to: Option<Text>,
}

impl AddDate {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
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
                message: "the date field is too small".to_string(),
            });
        }
        // The **name**.
        let existing: Vec<String> =
            crate::fields::fields_of(session.document()).into_iter().map(|field| field.name).collect();
        let name = match self.name.as_ref() {
            Some(text) => {
                let name = text.0.trim().to_string();
                if name.is_empty() {
                    free_name(&existing, "Date")
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
            None => free_name(&existing, "Date"),
        };
        if existing.contains(&name) {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("a field named `{name}` already exists"),
            });
        }
        // The **day** the field holds.
        let value = self.value.as_ref().map(|value| value.0.clone()).unwrap_or_default();
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
            let Some(bytes) = crate::fieldadd::the_drawing(rect, &value, false) else {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "the day cannot be written".to_string(),
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
        // The **date field** itself: the tree's `widget_dict`, with the
        // date field's own entries.
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
            ("FT".to_string(), json!("Tx")),
            ("DA".to_string(), json!({ "string": "/Helv 12 Tf 0 g" })),
            (
                "AA".to_string(),
                json!({
                    "entries": [
                        ["F", { "entries": [ ["S", "JavaScript"], ["JS", { "string": the_format_script() }] ] }],
                        ["K", { "entries": [ ["S", "JavaScript"], ["JS", { "string": the_keystroke_script() }] ] }],
                    ]
                }),
            ),
            ("T".to_string(), json!({ "string": name.clone() })),
        ];
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
        Ok(Outcome::changed(format!("added the date field `{name}` to the page")).with_data(json!({
            "id": widget,
            "object": number,
            "page": self.page.0,
            "name": name,
            "rect": rect,
            "value": value,
            "format": FORMAT,
            "form": form.node,
            "appearance": appearance,
        })))
    }
}
