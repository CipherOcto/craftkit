//! The **text field** that is put on a page.
//!
//! The tree's own: a **field** is a thing the reader may write in, and
//! a **text field** is the one that takes text. The tree writes it
//! with the conventions of Acrobat:
//!
//! - the **name** of the field is `Text1`, `Text2` and so on — the
//!   first name that is free — and a name the caller gives is used as
//!   it is;
//! - a name that holds a **period** is refused, and so is a name
//!   another field has;
//! - the field is **twelve point Helvetica** (`/DA`), and it may be a
//!   **multiline** field — the thirteenth bit of the flags of a field;
//! - the field has a **thin grey border** and a **white background**
//!   (`/MK` and `/BS`);
//! - the file's **form** (`/AcroForm`) holds the field, and the page
//!   holds it as an **annotation** (`/Annots`).
//!
//! The rules of the tree, ported by name:
//!
//! - `ensure_form` — the form is made when the file has none: the
//!   catalogue names it (`/AcroForm`), it holds the **fields**
//!   (`/Fields`), it says how a field's text looks by default
//!   (`/DA`) and it holds the **resources** the fields use (`/DR`);
//! - `free_name` — the first `<base>N` that is free, where a name
//!   that **begins** with it and a period is that name too;
//! - `widget_dict` — the field is an annotation of the kind `Widget`;
//! - `field_appearance` — the drawing of the field: the frame and the
//!   text, the text wrapped when the field is a multiline one.
//!
//! What is **not** here: the tree's other kinds of field — the check
//! box, the radio button, the drop-down, the list box, the button,
//! the date and the signature.

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, NodeRef, Number, Outcome, Session, Text};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.field.add";

/// The **multiline** flag of a field: the tree's own, the thirteenth
/// bit of the flags of a field.
pub const MULTILINE: i64 = 1 << 12;

/// The **smallest** a field may be: the tree's own rule, four points
/// by four.
const SMALLEST: f64 = 4.0;

/// The **size** of the text of a new field: the tree's own, twelve
/// points.
const SIZE: f64 = 12.0;

/// The **padding** of the text of a field: the tree's own, two points
/// and the width of the border.
const PADDING: f64 = 2.0;

/// The **width** of the border of a field: the tree's own, a thin one.
const BORDER: f64 = 1.0;

/// The **colour** of the border of a field: the tree's own, a grey.
const BORDER_COLOUR: [f64; 3] = [0.55, 0.55, 0.55];

/// The **colour** of the background of a field: the tree's own, a
/// white.
const BACKGROUND_COLOUR: [f64; 3] = [1.0, 1.0, 1.0];

/// The **standard fonts** of a form: the tree's own — the three names
/// Acrobat's Appearance tab offers, and the font each one is.
const FONTS: [(&str, &str); 3] = [("Helv", "Helvetica"), ("TiRo", "Times-Roman"), ("Cour", "Courier")];

/// The **entry** of a dictionary, by its key.
pub(crate) fn entry<'a>(entries: &'a [(String, Json)], key: &str) -> Option<&'a Json> {
    entries.iter().find(|(name, _)| name == key).map(|(_, value)| value)
}

/// The **pairs** of an inline dictionary.
fn dictionary_of(value: &Json) -> Vec<(String, Json)> {
    let Some(list) = value.get("entries").and_then(|entries| entries.as_array()) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|entry| {
            let pair = entry.as_array()?;
            let key = pair.first()?.as_str()?.to_string();
            Some((key, pair.get(1)?.clone()))
        })
        .collect()
}

/// The **entries** of a node's dictionary.
pub(crate) fn the_entries(session: &Session, node: u64) -> Vec<(String, Json)> {
    session.document().node(node).map(|node| crate::entries_of(&node.data)).unwrap_or_default()
}

/// A **node** of the document, with the entries it holds.
pub(crate) fn a_node(
    session: &mut Session,
    number: u64,
    kind: &str,
    entries: Vec<(String, Json)>,
) -> Result<u64, EngineError> {
    let data = json!({ "object": number, "entries": crate::pairs_of(&entries) });
    Ok(session.document_mut()?.add_node_with(format!("object {number}"), kind, None, data))
}

/// Put an **entry** in a node's dictionary.
pub(crate) fn put(session: &mut Session, node: u64, key: &str, value: Json) -> Result<(), EngineError> {
    if let Some(node) = session.document_mut()?.node_mut(node) {
        let data = crate::with_entry(&node.data, key, value);
        node.data = data;
    }
    Ok(())
}

/// The **dictionary** a value holds: the pairs of a node, or the pairs
/// of an inline dictionary.
fn the_dictionary(session: &Session, value: &Json) -> Vec<(String, Json)> {
    match crate::number_of(value) {
        Some(_) => crate::preflight::node_of(session.document(), value)
            .map(|node| crate::entries_of(&node.data))
            .unwrap_or_default(),
        None => dictionary_of(value),
    }
}

/// The **first free** name of a field.
///
/// The tree's own rule: the first `<base>N` that no field has, where
/// a field whose name **begins** with the name and a period has that
/// name too.
pub(crate) fn free_name(existing: &[String], base: &str) -> String {
    for number in 1u64.. {
        let candidate = format!("{base}{number}");
        let taken =
            existing.iter().any(|held| *held == candidate || held.starts_with(&format!("{candidate}.")));
        if !taken {
            return candidate;
        }
    }
    base.to_string()
}

/// The **form** of the file: the dictionary the catalogue names.
pub(crate) struct TheForm {
    /// The node the form is.
    pub(crate) node: u64,
    /// The object number of the form.
    pub(crate) number: u64,
    /// The **fonts** of the form: the name and the value that names
    /// the font.
    pub(crate) fonts: Vec<(String, Json)>,
}

/// The **form** of the file, made when the file has none.
///
/// The tree's own `ensure_form`: the catalogue names the form, the
/// form holds the fields, the form says how a field's text looks by
/// default, and the form holds the resources the fields use — the
/// three standard fonts, under the names Acrobat uses.
pub(crate) fn ensure_form(session: &mut Session) -> Result<TheForm, EngineError> {
    let catalog = crate::preflight::the_catalog(session.document()).map(|node| node.id).ok_or_else(|| {
        EngineError::Failed {
            id: COMMAND.to_string(),
            message: "the file has no catalogue, and a form hangs on the catalogue".to_string(),
        }
    })?;
    let catalog_entries = the_entries(session, catalog);
    // The form is an **object** of its own: the tree's own rule — a
    // form that is written where it is used becomes an object, and
    // the catalogue names it.
    let (node, number) = match entry(&catalog_entries, "AcroForm").cloned() {
        Some(value) => match crate::number_of(&value) {
            Some(number) => {
                let node = crate::preflight::node_of(session.document(), &value)
                    .map(|node| node.id)
                    .ok_or_else(|| EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "the file names a form that is not in the file".to_string(),
                    })?;
                (node, number)
            }
            None => {
                let base = dictionary_of(&value);
                let number = crate::free::the_next_number(session.document(), 0);
                let node = a_node(session, number, "pdf.object", base)?;
                put(session, catalog, "AcroForm", json!({ "ref": [number, 0] }))?;
                (node, number)
            }
        },
        None => {
            let number = crate::free::the_next_number(session.document(), 0);
            let node = a_node(session, number, "pdf.object", Vec::new())?;
            put(session, catalog, "AcroForm", json!({ "ref": [number, 0] }))?;
            (node, number)
        }
    };
    let entries = the_entries(session, node);
    // The **fields** of the form, and its **default appearance**.
    if entry(&entries, "Fields").is_none() {
        put(session, node, "Fields", json!([]))?;
    }
    if entry(&entries, "DA").is_none() {
        put(session, node, "DA", json!({ "string": "/Helv 0 Tf 0 g" }))?;
    }
    // The **fonts** of the form: the three the tree's editor offers.
    let mut fonts: Vec<(String, Json)> = match entry(&the_entries(session, node), "DR") {
        Some(dr) => match entry(&the_dictionary(session, dr), "Font") {
            Some(font) => the_dictionary(session, font),
            None => Vec::new(),
        },
        None => Vec::new(),
    };
    let mut added = false;
    for (name, base) in FONTS {
        if fonts.iter().any(|(held, _)| held == name) {
            continue;
        }
        // The font is an **object** of its own: the tree writes the
        // three fonts where they are used, and here a font is a thing
        // the file holds, so that the drawing of a field can name it.
        let number = crate::free::the_next_number(session.document(), 0);
        let entries = vec![
            ("Type".to_string(), json!("Font")),
            ("Subtype".to_string(), json!("Type1")),
            ("BaseFont".to_string(), json!(base)),
            ("Encoding".to_string(), json!("WinAnsiEncoding")),
        ];
        a_node(session, number, "pdf.object", entries)?;
        fonts.push((name.to_string(), json!({ "ref": [number, 0] })));
        added = true;
    }
    if added {
        let map = json!({
            "entries": fonts.iter().map(|(name, value)| json!([name, value])).collect::<Vec<_>>(),
        });
        // The **resources** of the form: the fonts, and whatever else
        // the file already held there.
        let resources = match entry(&the_entries(session, node), "DR") {
            Some(dr) => {
                let mut entries = the_dictionary(session, dr);
                match entries.iter_mut().find(|(held, _)| held == "Font") {
                    Some(entry) => entry.1 = map,
                    None => entries.push(("Font".to_string(), map)),
                }
                json!({ "entries": entries.iter().map(|(key, value)| json!([key, value])).collect::<Vec<_>>() })
            }
            None => json!({ "entries": [ ["Font", map] ] }),
        };
        put(session, node, "DR", resources)?;
    }
    Ok(TheForm { node, number, fonts })
}

/// The **lines** a text is drawn in: the text wrapped at a width.
///
/// The tree's own rule: the text is broken at the spaces, and a word
/// that does not fit on a line of its own is left long.
pub(crate) fn wrap(text: &str, size: f64, width: f64) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for paragraph in text.split('\n') {
        let mut line = String::new();
        for word in paragraph.split(' ') {
            let candidate = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
            if !line.is_empty() && crate::watermark::helvetica_width(&candidate, size) > width {
                lines.push(line);
                line = word.to_string();
            } else {
                line = candidate;
            }
        }
        lines.push(line);
    }
    lines
}

/// A **colour**, as PostScript writes it.
fn colour(colour: [f64; 3]) -> String {
    colour.iter().map(|value| crate::watermark::number(*value)).collect::<Vec<_>>().join(" ")
}

/// The **frame** of a field: the background and the border.
///
/// The tree's own `frame`: the background is filled over the whole box
/// and the border is stroked inside it.
pub(crate) fn the_frame(width: f64, height: f64) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{} rg\n0 0 {} {} re f\n",
        colour(BACKGROUND_COLOUR),
        crate::watermark::number(width),
        crate::watermark::number(height)
    ));
    out.push_str(&format!(
        "{} RG\n{} w\n{} {} {} {} re S\n",
        colour(BORDER_COLOUR),
        crate::watermark::number(BORDER),
        crate::watermark::number(BORDER / 2.0),
        crate::watermark::number(BORDER / 2.0),
        crate::watermark::number(width - BORDER),
        crate::watermark::number(height - BORDER)
    ));
    out
}

/// The **drawing** of a text field: the frame and the text.
///
/// The tree's own `field_appearance`: the background is filled, the
/// border is stroked, and the text is drawn in the box — on one line
/// and centred in it, or from the top down and wrapped when the field
/// is a multiline one.
pub(crate) fn the_drawing(rect: [f64; 4], value: &str, multiline: bool) -> Option<Vec<u8>> {
    let width = rect[2] - rect[0];
    let height = rect[3] - rect[1];
    let pad = PADDING + BORDER;
    let inner = (width - 2.0 * pad).max(1.0);
    let mut out = the_frame(width, height);
    // The **text**.
    out.push_str(&format!(
        "/Tx BMC\nq\n{} {} {} {} re W n\nBT\n/Helv {} Tf\n0 g\n",
        crate::watermark::number(BORDER),
        crate::watermark::number(BORDER),
        crate::watermark::number(width - 2.0 * BORDER),
        crate::watermark::number(height - 2.0 * BORDER),
        crate::watermark::number(SIZE)
    ));
    if !value.is_empty() {
        if multiline {
            let mut y = height - pad - SIZE * 0.85;
            for line in wrap(value, SIZE, inner) {
                if y < -SIZE {
                    break;
                }
                let text = crate::appearance::the_string(&line)?;
                out.push_str(&format!(
                    "1 0 0 1 {} {} Tm ({text}) Tj\n",
                    crate::watermark::number(pad),
                    crate::watermark::number(y)
                ));
                y -= SIZE * 1.15;
            }
        } else {
            let text = crate::appearance::the_string(value)?;
            // Helvetica's ascent and descent, centred in the box.
            let y = (height - 0.925 * SIZE) / 2.0 + 0.207 * SIZE;
            out.push_str(&format!(
                "1 0 0 1 {} {} Tm ({text}) Tj\n",
                crate::watermark::number(pad),
                crate::watermark::number(y)
            ));
        }
    }
    out.push_str("ET\nQ\nEMC\n");
    Some(out.into_bytes())
}

/// **Put** a **text field** on a page.
///
/// The field is a widget — an annotation of the kind `Widget` — and it
/// is on the page the caller names, in the rectangle the caller gives.
///
/// The rules:
///
/// - the **page** must be a page of the file;
/// - the **rectangle** is four numbers, and the corners may come in
///   any order; a field smaller than four points by four is refused,
///   as the tree refuses it;
/// - the **name** is the first free `Text` name when the caller does
///   not give one; a name with a period in it is refused, and so is a
///   name another field has;
/// - the file's **form** is made when the file has none, and it holds
///   the field;
/// - the **drawing** of the field is written when the caller says
///   where the bytes go — this tree keeps the bytes of a stream in a
///   file of their own.
#[derive(Command)]
#[command(id = "pdf.field.add", area = "pdf", layer = "pdf", icon = "plus", mutates, undoable)]
pub struct AddField {
    /// The page the field is put on.
    pub page: NodeRef,
    /// The field's object number in the file.
    pub object: Number,
    /// The rectangle the field covers: four numbers.
    pub rect: serde_json::Value,
    /// The name of the field.
    ///
    /// Leave it out and the field is given the first free name of the
    /// tree: `Text1`, `Text2` and so on.
    pub name: Option<Text>,
    /// Whether the field holds more than one line of text.
    pub multiline: Option<Bool>,
    /// The text the field holds.
    pub value: Option<Text>,
    /// Where the bytes of the field's drawing are written.
    ///
    /// Leave it out and the field is put on the page with no drawing
    /// of its own.
    pub appearance_to: Option<Text>,
}

impl AddField {
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
                message: "the field is too small".to_string(),
            });
        }
        // The **name**.
        let existing: Vec<String> =
            crate::fields::fields_of(session.document()).into_iter().map(|field| field.name).collect();
        let name = match self.name.as_ref() {
            Some(text) => {
                let name = text.0.trim().to_string();
                if name.is_empty() {
                    free_name(&existing, "Text")
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
            None => free_name(&existing, "Text"),
        };
        if existing.contains(&name) {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("a field named `{name}` already exists"),
            });
        }
        // The **field** itself: the tree's `widget_dict`, with the
        // text field's own entries.
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
                        ["BC", json!(BORDER_COLOUR)],
                        ["BG", json!(BACKGROUND_COLOUR)],
                    ]
                }),
            ),
            ("BS".to_string(), json!({ "entries": [ ["W", 1], ["S", "S"] ] })),
            ("FT".to_string(), json!("Tx")),
            ("DA".to_string(), json!({ "string": "/Helv 12 Tf 0 g" })),
            ("T".to_string(), json!({ "string": name.clone() })),
        ];
        let multiline = self.multiline.as_ref().map(|flag| flag.0).unwrap_or(false);
        if multiline {
            entries.push(("Ff".to_string(), json!(MULTILINE)));
        }
        if let Some(value) = &self.value {
            entries.push(("V".to_string(), json!({ "string": value.0.clone() })));
        }
        // The **form**: it holds the field, and it is made when the
        // file has none. The form and its fonts are objects of their
        // own, and they are made before the field, so that the field
        // holds the number the caller gave it.
        let form = ensure_form(session)?;
        // The **drawing** of the field: the bytes are made before the
        // document is changed, so that a text that cannot be written
        // is refused and nothing is changed.
        let mut drawing = None;
        if let Some(to) = &self.appearance_to {
            let path = to.0.trim().to_string();
            if path.is_empty() {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "the drawing goes somewhere".to_string(),
                });
            }
            let value = self.value.as_ref().map(|value| value.0.as_str()).unwrap_or("");
            let Some(bytes) = the_drawing(rect, value, multiline) else {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "the text of the field cannot be written".to_string(),
                });
            };
            drawing = Some((path, bytes));
        }
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
        let data = json!({ "object": number, "entries": crate::pairs_of(&entries) });
        let widget =
            session.document_mut()?.add_node_with(format!("field {number}"), "pdf.object", None, data);
        // The number of the **drawing**: the stream that holds it. It
        // is a number of its own, above the field's.
        let appearance_number = match &drawing {
            Some(_) => crate::free::the_next_number(session.document(), 0),
            None => 0,
        };
        let mut fields = entry(&the_entries(session, form.node), "Fields")
            .and_then(|fields| fields.as_array().cloned())
            .unwrap_or_default();
        fields.push(json!({ "ref": [number, 0] }));
        put(session, form.node, "Fields", json!(fields))?;
        // The **page** holds the field as an annotation.
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
            let number = appearance_number;
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
            let stream = a_node(session, number, "pdf.stream", entries)?;
            if let Some(node) = session.document_mut()?.node_mut(stream) {
                node.stream = Some(path.clone());
            }
            put(session, widget, "AP", json!({ "entries": [ ["N", { "ref": [number, 0] } ] ] }))?;
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
        Ok(Outcome::changed(format!("added the field `{name}` to the page")).with_data(json!({
            "id": widget,
            "object": number,
            "page": self.page.0,
            "name": name,
            "rect": rect,
            "multiline": multiline,
            "form": form.node,
            "form_object": form.number,
            "appearance": appearance,
        })))
    }
}
