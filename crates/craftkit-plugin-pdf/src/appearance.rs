//! The **appearance** of a field.
//!
//! A field that is shown on a page has an **appearance**: the stream
//! that draws it. The appearance is a **form XObject** — a little
//! drawing that says what the field looks like — and the widget's
//! dictionary names it under the key `N` of its appearance dictionary
//! (`/AP`).
//!
//! An appearance may be a **set of states**: a button has one drawing
//! for the state it is off in and another for the state it is on in,
//! and the dictionary holds one stream per state. The field's value
//! (`/V`) says which state it is in.
//!
//! And a field may have **no appearance at all**: the form may say
//! that the reader is to make the appearances (`NeedAppearances`), and
//! then the field has nothing to show and the reader draws it.
//!
//! The rules:
//!
//! - the appearance is read as the file says it: the **box** it is
//!   drawn in (`/BBox`), the **matrix** that places it (`/Matrix`)
//!   and the **operators** of its stream;
//! - a field that says nothing about its appearance has none, and the
//!   answer says so.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Node, NodeRef, Outcome, Text};

/// The **appearance** of a field: what the file says about it.
#[derive(Debug, Clone, PartialEq)]
pub struct Appearance {
    /// The states the appearance has, when it has more than one.
    pub states: Vec<String>,
    /// The state the field is in, when the appearance has states.
    pub state: Option<String>,
    /// The box the appearance is drawn in.
    pub box_: Option<Vec<f64>>,
    /// The matrix that places the appearance.
    pub matrix: Option<Vec<f64>>,
    /// The operations of the appearance's stream.
    pub operations: Vec<Json>,
}

/// The **entries** of a node's dictionary.
fn entries_of(node: &Node) -> Vec<(String, Json)> {
    crate::entries_of(&node.data)
}

/// The **entry** of a dictionary, by its key.
fn entry<'a>(entries: &'a [(String, Json)], key: &str) -> Option<&'a Json> {
    entries.iter().find(|(name, _)| name == key).map(|(_, value)| value)
}

/// The **text** a value holds: the file's own spelling of a text.
fn text_of(value: &Json) -> Option<String> {
    match value {
        Json::String(text) => Some(text.clone()),
        Json::Object(map) => map.get("string").and_then(|text| text.as_str()).map(|text| text.to_string()),
        _ => None,
    }
}

/// The **numbers** a value holds.
fn numbers_of(value: &Json) -> Option<Vec<f64>> {
    let list = value.as_array()?;
    Some(list.iter().filter_map(|number| number.as_f64()).collect())
}

/// The **dictionary** a value holds: the pairs of an inline
/// dictionary.
fn dictionary_of(value: &Json) -> Option<Vec<(String, Json)>> {
    let list = value.get("entries")?.as_array()?;
    Some(
        list.iter()
            .filter_map(|entry| {
                let pair = entry.as_array()?;
                let key = pair.first()?.as_str()?.to_string();
                Some((key, pair.get(1)?.clone()))
            })
            .collect(),
    )
}

/// The **node** an object number names.
fn node_of_number(document: &Document, number: u64) -> Option<&Node> {
    document.walk().find(|node| node.data.get("object").and_then(|found| found.as_u64()) == Some(number))
}

/// The **appearance** of a widget: the drawing the file holds for it.
pub fn appearance_of(document: &Document, node: &Node) -> Option<Appearance> {
    let entries = entries_of(node);
    let appearances = entry(&entries, "AP")?;
    // The **normal** appearance: the one that is drawn.
    let normal =
        dictionary_of(appearances)?.iter().find(|(key, _)| key == "N").map(|(_, value)| value.clone())?;
    // The appearance is either one stream, or a set of **states**.
    let states: Vec<(String, Json)> = dictionary_of(&normal).unwrap_or_default();
    let chosen: Option<Json> = if states.is_empty() {
        Some(normal.clone())
    } else {
        // The field's value says which state it is in.
        let value = entry(&entries, "V").and_then(text_of);
        value
            .and_then(|value| states.iter().find(|(name, _)| *name == value).map(|(_, held)| held.clone()))
            .or_else(|| states.first().map(|(_, held)| held.clone()))
    };
    let stream = match chosen {
        Some(value) => match crate::number_of(&value) {
            Some(number) => node_of_number(document, number)?.clone(),
            None => {
                // An appearance written where it is used is not a
                // stream this tree can read.
                return Some(Appearance {
                    states: states.iter().map(|(name, _)| name.clone()).collect(),
                    state: None,
                    box_: None,
                    matrix: None,
                    operations: Vec::new(),
                });
            }
        },
        None => {
            return Some(Appearance {
                states: states.iter().map(|(name, _)| name.clone()).collect(),
                state: None,
                box_: None,
                matrix: None,
                operations: Vec::new(),
            });
        }
    };
    let stream_entries = entries_of(&stream);
    let box_ = entry(&stream_entries, "BBox").and_then(numbers_of);
    let matrix = entry(&stream_entries, "Matrix").and_then(numbers_of);
    // The **operators** of the appearance: what it says to draw.
    let operations = match crate::decoded_bytes(&stream, None, None, false, "pdf.field.appearance") {
        Ok(decoded) => match crate::content::operations(&decoded.bytes) {
            Ok(operations) => operations.iter().map(|operation| operation.to_json()).collect(),
            Err(_) => Vec::new(),
        },
        Err(_) => Vec::new(),
    };
    let state = entry(&entries, "V").and_then(text_of);
    Some(Appearance {
        states: states.iter().map(|(name, _)| name.clone()).collect(),
        state,
        box_,
        matrix,
        operations,
    })
}

commands! {
    /// **Read** the **appearance** of a field.
    ///
    /// A query: it does not change the document. A field that says nothing
    /// about its appearance has none, and the answer says so.
    pdf.field.appearance {
        area: "pdf",
        layer: "pdf",
        icon: "list",
        params: {
            /// The field to read.
            field: NodeRef,
        },
        run: {
            let id = "pdf.field.appearance".to_string();
            let document = session.document();
            let node = document.node(self.field.0).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{}`", self.field.0),
            })?;
            let name = crate::fields::qualified_name(document, node);
            let Some(appearance) = appearance_of(document, node) else {
                return Ok(Outcome::new("the field has no appearance").with_data(json!({
                    "field": name.clone().map(|name| json!(name)).unwrap_or(Json::Null),
                    "has_appearance": false,
                    "states": Vec::<Json>::new(),
                    "state": Json::Null,
                    "box": Json::Null,
                    "matrix": Json::Null,
                    "operations": Vec::<Json>::new(),
                })));
            };
            Ok(Outcome::new("the field has an appearance").with_data(json!({
                "field": name.clone().map(|name| json!(name)).unwrap_or(Json::Null),
                "has_appearance": true,
                "states": appearance.states.clone(),
                "state": appearance.state.clone().map(|state| json!(state)).unwrap_or(Json::Null),
                "box": appearance.box_.clone().map(|values| json!(values)).unwrap_or(Json::Null),
                "matrix": appearance.matrix.clone().map(|values| json!(values)).unwrap_or(Json::Null),
                "operations": appearance.operations.clone(),
            })))

        }
    }

    /// **Write** the **appearance** of a field.
    ///
    /// The appearance is a **form XObject** — a little drawing — and the
    /// widget's `/AP` names it under the key `N`. The bytes of the
    /// drawing are written to a file of their own, beside the document,
    /// and the node of the appearance says which file they are in.
    ///
    /// The rules:
    ///
    /// - the widget must have a **rectangle**, for the drawing to be
    ///   drawn in;
    /// - the **value** is the one the widget holds, or the one it
    ///   inherits, and a widget that holds none is refused;
    /// - the **size** and the **font** come from the default appearance
    ///   (`/DA`), and a field that says none is refused;
    /// - the font must be **in the file** — in the page's resources or in
    ///   the form's — and a font that is not there is refused;
    /// - the text is drawn at the **top left** of the box, two points
    ///   from its edge, and the drawing is clipped to the box;
    /// - the **frame** of the field is drawn from its **look**: the
    ///   background is filled from the `/BG` and the border is stroked from
    ///   the `/BC` and the `/BS` — the tree's own `frame` — and a field
    ///   that says no colour has no background and no border.
    pdf.field.appearance.write {
        area: "pdf",
        layer: "pdf",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The widget the appearance is written for.
            widget: NodeRef,
            /// Where the bytes of the appearance are written.
            to: Text,
        },
        run: {
            let id = "pdf.field.appearance.write".to_string();
            let widget = self.widget.0;
            let Some(node) = session.document().node(widget) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{widget}`"),
                });
            };
            let entries = entries_of(node);
            let subtype = entry(&entries, "Subtype").and_then(|subtype| subtype.as_str());
            if subtype != Some("Widget") {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{widget}` is not a widget"),
                });
            }
            let Some((width, height)) = box_of(node) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the widget `{widget}` has no box to be drawn in"),
                });
            };
            let Some(value) = value_of(session.document(), node) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the widget `{widget}` holds no value to draw"),
                });
            };
            let Some(da) = da_of(session.document(), node) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the widget `{widget}` says nothing about how its text is drawn"),
                });
            };
            let Some((font, size)) = font_and_size(&da) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the default appearance `{da}` says no font and no size"),
                });
            };
            let Some(font_number) = font_number_of(session.document(), node, &font) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the font `{font}` is not in the file"),
                });
            };
            let Some(drawing) = the_drawing(&font, size, width, height, &value) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the value of the widget `{widget}` cannot be written"),
                });
            };
            // The **frame** of the widget: the background and the border of
            // it, as the tree's own `frame` draws them.
            let frame = the_frame_of(session.document(), &entries, width, height);
            let mut bytes = frame.into_bytes();
            bytes.extend_from_slice(&drawing);
            let path = self.to.0.clone();
            if path.trim().is_empty() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the drawing goes somewhere".to_string(),
                });
            }
            std::fs::write(&path, &bytes).map_err(|error| EngineError::Failed {
                id: id.clone(),
                message: format!("the drawing cannot be written to `{path}`: {error}"),
            })?;
            // The **appearance**: a form XObject with the box, and the
            // font it draws with.
            let number = crate::free::the_next_number(session.document(), 0);
            let font_resource = json!({ "entries": [ [ font.clone(), { "ref": [font_number, 0] } ] ] });
            let resources = json!({ "entries": [ [ "Font", font_resource ] ] });
            let appearance = json!({
                "object": number,
                "entries": [
                    ["Type", "XObject"],
                    ["Subtype", "Form"],
                    ["BBox", [0.0, 0.0, width, height]],
                    ["Resources", resources],
                ],
            });
            let name = format!("object {number}");
            let stream = session.document_mut()?.add_node_with(name, "pdf.stream", None, appearance);
            if let Some(node) = session.document_mut()?.node_mut(stream) {
                node.stream = Some(path.clone());
            }
            // The widget's **appearance**: the normal one, under `N`.
            let normal = json!({ "entries": [ ["N", { "ref": [number, 0] } ] ] });
            let mut entries = entries_of(session.document().node(widget).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{widget}`"),
            })?);
            match entries.iter_mut().find(|(key, _)| key == "AP") {
                Some(entry) => entry.1 = normal,
                None => entries.push(("AP".to_string(), normal)),
            }
            let data = json!({
                "object": session.document().node(widget).and_then(|node| node.data.get("object").cloned()).unwrap_or(Json::Null),
                "entries": entries.iter().map(|(key, value)| json!([key, value])).collect::<Vec<_>>(),
            });
            if let Some(node) = session.document_mut()?.node_mut(widget) {
                node.data = data;
            }
            // The **edge**: the widget refers to the appearance, and the
            // file is written from the edges as well as from the
            // dictionaries.
            let relation = craftkit_sdk::Relation::with_payload(
                widget,
                stream,
                "ref",
                json!({ "key": "AP", "generation": 0 }),
            );
            if !session.document_mut()?.add_relation(relation) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the widget `{widget}` already refers to the appearance"),
                });
            }
            Ok(Outcome::changed(format!("the widget `{widget}` has an appearance")).with_data(json!({
                "widget": widget,
                "appearance": stream,
                "object": number,
                "path": path,
                "bytes": bytes.len(),
                "font": font,
                "size": size,
                "box": [0.0, 0.0, width, height],
            })))

        }
    }
}

/// The most **deep** a chain of parents is walked.
const MAX_PARENTS: usize = 8;

/// The **value** a widget holds: its own, or the one it inherits.
///
/// The chain is **bounded** and a field that is its own ancestor ends
/// it, so that a file that says a field is its own parent does not
/// walk for ever.
fn value_of(document: &Document, node: &Node) -> Option<String> {
    let mut current = Some(node.id);
    let mut depth = 0usize;
    while let Some(id) = current {
        if depth >= MAX_PARENTS {
            return None;
        }
        let node = document.node(id)?;
        let entries = entries_of(node);
        if let Some(value) = entry(&entries, "V").and_then(text_of) {
            return Some(value);
        }
        current = entry(&entries, "Parent")
            .and_then(crate::number_of)
            .and_then(|number| node_of_number(document, number))
            .map(|node| node.id);
        depth += 1;
    }
    None
}

/// The **default appearance** of a widget: the `/DA` it says, or the
/// one it inherits.
fn da_of(document: &Document, node: &Node) -> Option<String> {
    let mut current = Some(node.id);
    let mut depth = 0usize;
    while let Some(id) = current {
        if depth >= MAX_PARENTS {
            return None;
        }
        let node = document.node(id)?;
        let entries = entries_of(node);
        if let Some(da) = entry(&entries, "DA").and_then(text_of) {
            return Some(da);
        }
        current = entry(&entries, "Parent")
            .and_then(crate::number_of)
            .and_then(|number| node_of_number(document, number))
            .map(|node| node.id);
        depth += 1;
    }
    None
}

/// The **font** and the **size** a default appearance says.
///
/// A default appearance is a list of things to set: `/Helv 12 Tf 0 g`
/// says the font is `Helv` and the size is twelve, and the number
/// before the `Tf` is the size and the name before it is the font.
fn font_and_size(da: &str) -> Option<(String, f64)> {
    let tokens: Vec<&str> = da.split_whitespace().collect();
    let at = tokens.iter().position(|token| *token == "Tf")?;
    if at < 2 {
        return None;
    }
    let font = tokens[at - 2].strip_prefix('/')?.to_string();
    let size = tokens[at - 1].parse::<f64>().ok()?;
    if !size.is_finite() || size <= 0.0 {
        return None;
    }
    Some((font, size))
}

/// The **box** of a widget: its width and its height.
fn box_of(node: &Node) -> Option<(f64, f64)> {
    let entries = entries_of(node);
    let numbers = numbers_of(entry(&entries, "Rect")?)?;
    if numbers.len() != 4 {
        return None;
    }
    let width = (numbers[2] - numbers[0]).abs();
    let height = (numbers[3] - numbers[1]).abs();
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return None;
    }
    Some((width, height))
}

/// The **dictionary** a value holds: the pairs of an object or of an
/// inline dictionary.
fn dictionary_of_value(document: &Document, value: &Json) -> Option<Vec<(String, Json)>> {
    if let Some(number) = crate::number_of(value) {
        return Some(entries_of(node_of_number(document, number)?));
    }
    dictionary_of(value)
}

/// The **font** a name names: the number of the object of the font.
///
/// The font is looked for in the page the widget is on, and then in
/// the form's resources — the place a form keeps the fonts it uses.
fn font_number_of(document: &Document, node: &Node, name: &str) -> Option<u64> {
    let entries = entries_of(node);
    // The **page** the widget is on.
    if let Some(page) =
        entry(&entries, "P").and_then(crate::number_of).and_then(|n| node_of_number(document, n))
    {
        let page_entries = entries_of(page);
        if let Some(resources) =
            entry(&page_entries, "Resources").and_then(|r| dictionary_of_value(document, r))
            && let Some(found) = font_in(document, &resources, name)
        {
            return Some(found);
        }
    }
    // The **form's** resources.
    let catalog = crate::preflight::the_catalog(document)?;
    let catalog_entries = entries_of(catalog);
    let form = entry(&catalog_entries, "AcroForm").and_then(crate::number_of)?;
    let form = node_of_number(document, form)?;
    let form_entries = entries_of(form);
    let resources = entry(&form_entries, "DR").and_then(|r| dictionary_of_value(document, r))?;
    font_in(document, &resources, name)
}

/// The **font** a name names, in a dictionary of resources.
fn font_in(document: &Document, resources: &[(String, Json)], name: &str) -> Option<u64> {
    let fonts = entry(resources, "Font")?;
    let fonts = dictionary_of_value(document, fonts)?;
    let font = entry(&fonts, name)?;
    crate::number_of(font)
}

/// The **text** of a value, as a PDF string writes it.
///
/// The characters that are not letters are written as the octal the
/// standard encoding gives them, and a character that has no byte is
/// refused: a value that cannot be written is not written.
pub(crate) fn the_string(value: &str) -> Option<String> {
    let mut out = String::new();
    for character in value.chars() {
        match character {
            '(' | ')' | '\\' => {
                out.push('\\');
                out.push(character);
            }
            ' '..='~' => out.push(character),
            other => {
                let byte = other as u32;
                if byte > 0xff {
                    return None;
                }
                out.push_str(&format!("\\{:03o}", byte));
            }
        }
    }
    Some(out)
}

/// The **drawing** of a field: the bytes of its appearance.
fn the_drawing(font: &str, size: f64, width: f64, height: f64, value: &str) -> Option<Vec<u8>> {
    let text = the_string(value)?;
    let drawing =
        format!("q\n1 1 {width} {height} re W n\nBT\n/{font} {size} Tf\n0 g\n2 2 Td\n({text}) Tj\nET\nQ\n");
    Some(drawing.into_bytes())
}

/// The **colour** a value holds, as PostScript writes it.
///
/// The tree's own `color_array`: one number is a grey, three are red,
/// green and blue, and four are cyan, magenta, yellow and black.
fn the_colour_of(value: &Json) -> Option<String> {
    let numbers = numbers_of(value)?;
    let number = crate::watermark::number;
    match numbers.as_slice() {
        [grey] => Some(format!("{} g", number(*grey))),
        [red, green, blue] => Some(format!("{} {} {} rg", number(*red), number(*green), number(*blue))),
        [cyan, magenta, yellow, black] => {
            Some(format!("{} {} {} {} k", number(*cyan), number(*magenta), number(*yellow), number(*black)))
        }
        _ => None,
    }
}

/// The **stroke** of a colour: the tree's own `stroke_op`.
fn the_stroke_of(colour: &str) -> String {
    match colour.rsplit_once(' ') {
        Some((numbers, "g")) => format!("{numbers} G"),
        Some((numbers, "rg")) => format!("{numbers} RG"),
        Some((numbers, "k")) => format!("{numbers} K"),
        _ => "0 G".to_string(),
    }
}

/// The **frame** of a field: the background and the border of it.
///
/// The tree's own `frame`: the background is filled from the `/MK` and
/// the border is stroked from the `/MK` and the `/BS` — and a field
/// that says no colour has no background and no border, which is the
/// tree's own rule.
fn the_frame_of(document: &Document, entries: &[(String, Json)], width: f64, height: f64) -> String {
    let number = crate::watermark::number;
    let mut out = String::new();
    // The **mark** of the widget: the colours of it.
    let mark =
        entry(entries, "MK").and_then(|value| dictionary_of_value(document, value)).unwrap_or_default();
    // The **background**: the tree's own `/BG`.
    if let Some(background) = entry(&mark, "BG").and_then(the_colour_of) {
        out.push_str(&format!("{}\n0 0 {} {} re f\n", background, number(width), number(height)));
    }
    // The **line**: its width and its style.
    let line =
        entry(entries, "BS").and_then(|value| dictionary_of_value(document, value)).unwrap_or_default();
    let border_width = entry(&line, "W").and_then(|value| value.as_f64()).unwrap_or(1.0).max(0.0);
    // The **border**: the tree's own `/BC`, and a field that says no
    // colour has no border.
    let Some(border) = entry(&mark, "BC").and_then(the_colour_of) else {
        return out;
    };
    if border_width <= 0.0 {
        return out;
    }
    let style = entry(&line, "S").and_then(|value| value.as_str()).unwrap_or("S").to_string();
    if style == "U" {
        // **Underline**: only the bottom edge.
        out.push_str(&format!(
            "{}\n{} w\n0 {} m {} {} l S\n",
            the_stroke_of(&border),
            number(border_width),
            number(border_width / 2.0),
            number(width),
            number(border_width / 2.0)
        ));
        return out;
    }
    let dashed = style == "D";
    out.push_str(&format!(
        "{}\n{} w\n{}{} {} {} {} re S\n[] 0 d\n",
        the_stroke_of(&border),
        number(border_width),
        if dashed { "[3] 0 d\n" } else { "" },
        number(border_width / 2.0),
        number(border_width / 2.0),
        number(width - border_width),
        number(height - border_width)
    ));
    if style == "B" || style == "I" {
        // **Beveled**: a light top-left and a darker bottom-right inside
        // the border; **inset**: a grey top-left and a light grey
        // bottom-right. The tree's own.
        let (top, bottom) = if style == "B" { ("1 g", "0.5 g") } else { ("0.5 g", "0.75 g") };
        let (x0, y0, x1, y1) = (border_width, border_width, width - border_width, height - border_width);
        let k = border_width;
        out.push_str(&format!(
            "{top}\n{} {} m {} {} l {} {} l {} {} l {} {} l {} {} l f\n{bottom}\n{} {} m {} {} l {} {} l {} {} l {} {} l {} {} l f\n",
            number(x0),
            number(y0),
            number(x0),
            number(y1),
            number(x1),
            number(y1),
            number(x1 - k),
            number(y1 - k),
            number(x0 + k),
            number(y1 - k),
            number(x0 + k),
            number(y0 + k),
            number(x1),
            number(y1),
            number(x1),
            number(y0),
            number(x0),
            number(y0),
            number(x0 + k),
            number(y0 + k),
            number(x1 - k),
            number(y0 + k),
            number(x1 - k),
            number(y1 - k)
        ));
    }
    out
}
