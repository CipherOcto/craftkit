//! The **check box** that is put on a page.
//!
//! The tree's own: a **check box** is a field of the kind `Btn` that
//! is either **on** or **off**, and it says which it is in two places:
//! the **value** (`/V`) is the state it is in, and the **appearance
//! state** (`/AS`) is the state the page shows.
//!
//! The tree writes it with the conventions of Acrobat:
//!
//! - the **name** of the field is `Check Box1`, `Check Box2` and so on
//!   — the first name that is free;
//! - the field starts **off** (`/V` and `/AS` are `Off`), and the state
//!   it is on in is called `Yes`;
//! - the **mark** that says the box is on is one of **six** shapes —
//!   the check, the circle, the cross, the diamond, the square and the
//!   star — and the file says which one in the **caption** of the
//!   field (`/MK /CA`), with the code of the glyph of the Zapf
//!   Dingbats;
//! - the mark is **drawn as a path** and not as a glyph, so that no
//!   font of the file is needed to show it.
//!
//! The rules of the tree, ported by name:
//!
//! - `ensure_form` — the form is made when the file has none, and it
//!   holds the field;
//! - `free_name` — the first `<base>N` that is free;
//! - `widget_dict` — the field is an annotation of the kind `Widget`;
//! - `check_box_states` — the **two** drawings of the field: the one
//!   that says it is off, and the one that says it is on.

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session, Text};

use crate::fieldadd::{a_node, ensure_form, entry, free_name, put, the_entries, the_frame};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.field.checkbox.add";

/// The **smallest** a check box may be: the tree's own rule, four
/// points by four.
const SMALLEST: f64 = 4.0;

/// The **state** a check box is off in: the tree's own.
const OFF: &str = "Off";

/// The **state** a check box is on in: the tree's own, `Yes`.
const ON: &str = "Yes";

/// The **style** of the mark of a check box.
///
/// The tree's own six: the check, the circle, the cross, the diamond,
/// the square and the star.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckStyle {
    /// The check: a tick.
    Check,
    /// The circle: a dot.
    Circle,
    /// The cross: two strokes.
    Cross,
    /// The diamond.
    Diamond,
    /// The square.
    Square,
    /// The star.
    Star,
}

impl CheckStyle {
    /// Every style, in the tree's own order.
    pub const ALL: [CheckStyle; 6] = [
        CheckStyle::Check,
        CheckStyle::Circle,
        CheckStyle::Cross,
        CheckStyle::Diamond,
        CheckStyle::Square,
        CheckStyle::Star,
    ];

    /// The **code** of the style: the glyph of the Zapf Dingbats that
    /// the tree writes into the caption of the field.
    pub fn code(self) -> &'static str {
        match self {
            CheckStyle::Check => "4",
            CheckStyle::Circle => "l",
            CheckStyle::Cross => "8",
            CheckStyle::Diamond => "u",
            CheckStyle::Square => "n",
            CheckStyle::Star => "H",
        }
    }

    /// The **label** of the style: what the tree's own menu says.
    pub fn label(self) -> &'static str {
        match self {
            CheckStyle::Check => "Check",
            CheckStyle::Circle => "Circle",
            CheckStyle::Cross => "Cross",
            CheckStyle::Diamond => "Diamond",
            CheckStyle::Square => "Square",
            CheckStyle::Star => "Star",
        }
    }

    /// The style a **label** names — the tree's own labels, and the
    /// tree's own codes.
    pub fn parse(label: &str) -> Option<CheckStyle> {
        let label = label.trim();
        CheckStyle::ALL
            .into_iter()
            .find(|style| style.label().eq_ignore_ascii_case(label) || style.code() == label)
    }
}

/// The **mark** of a check box: the shape that says it is on.
///
/// The tree's own `check_mark`: the shape is drawn as a path — a
/// stroke, a filled polygon or a filled circle — and not as a glyph.
fn the_mark(style: CheckStyle, cx: f64, cy: f64, s: f64) -> String {
    let number = crate::watermark::number;
    // A filled polygon through the points.
    let polygon = |points: &[(f64, f64)]| {
        let mut out = String::from("0 g\n");
        for (index, (x, y)) in points.iter().enumerate() {
            out.push_str(&format!("{} {} {} ", number(*x), number(*y), if index == 0 { "m" } else { "l" }));
        }
        out.push_str("h f\n");
        out
    };
    let line = number((s * 0.1).max(1.0));
    match style {
        CheckStyle::Check => format!(
            "0 G\n{line} w 1 J 1 j\n{} {} m {} {} l {} {} l S\n",
            number(cx - s * 0.28),
            number(cy),
            number(cx - s * 0.08),
            number(cy - s * 0.22),
            number(cx + s * 0.3),
            number(cy + s * 0.25)
        ),
        CheckStyle::Cross => {
            let d = s * 0.25;
            format!(
                "0 G\n{line} w 1 J\n{} {} m {} {} l S\n{} {} m {} {} l S\n",
                number(cx - d),
                number(cy - d),
                number(cx + d),
                number(cy + d),
                number(cx - d),
                number(cy + d),
                number(cx + d),
                number(cy - d)
            )
        }
        CheckStyle::Square => {
            let d = s * 0.22;
            format!(
                "0 g\n{} {} {} {} re f\n",
                number(cx - d),
                number(cy - d),
                number(2.0 * d),
                number(2.0 * d)
            )
        }
        CheckStyle::Diamond => {
            let d = s * 0.3;
            polygon(&[(cx, cy + d), (cx + d, cy), (cx, cy - d), (cx - d, cy)])
        }
        CheckStyle::Star => {
            // Five points: the outer and the inner radius alternate,
            // and the first point is straight up.
            let (outer, inner) = (s * 0.32, s * 0.32 * 0.382);
            let points: Vec<(f64, f64)> = (0..10)
                .map(|index| {
                    let angle = std::f64::consts::FRAC_PI_2 + index as f64 * std::f64::consts::PI / 5.0;
                    let radius = if index % 2 == 0 { outer } else { inner };
                    (cx + radius * angle.cos(), cy + radius * angle.sin())
                })
                .collect();
            polygon(&points)
        }
        CheckStyle::Circle => {
            let r = s * 0.25;
            let k = 0.5523 * r;
            format!(
                "0 g\n{} {} m {} {} {} {} {} {} c {} {} {} {} {} {} c {} {} {} {} {} {} c {} {} {} {} {} {} c f\n",
                number(cx + r),
                number(cy),
                number(cx + r),
                number(cy + k),
                number(cx + k),
                number(cy + r),
                number(cx),
                number(cy + r),
                number(cx - k),
                number(cy + r),
                number(cx - r),
                number(cy + k),
                number(cx - r),
                number(cy),
                number(cx - r),
                number(cy - k),
                number(cx - k),
                number(cy - r),
                number(cx),
                number(cy - r),
                number(cx + k),
                number(cy - r),
                number(cx + r),
                number(cy - k),
                number(cx + r),
                number(cy)
            )
        }
    }
}

/// **Put** a **check box** on a page.
///
/// The field is a widget — an annotation of the kind `Widget` — and it
/// is on the page the caller names, in the rectangle the caller gives.
/// It starts **off**, and its **mark** is the tree's own check unless
/// the caller asks for another one.
///
/// The rules:
///
/// - the **page** must be a page of the file;
/// - the **rectangle** is four numbers, and the corners may come in
///   any order; a check box smaller than four points by four is
///   refused, as the tree refuses it;
/// - the **name** is the first free `Check Box` name when the caller
///   does not give one; a name with a period in it is refused, and so
///   is a name another field has;
/// - the **style** is one of the tree's six — the check, the circle,
///   the cross, the diamond, the square and the star — and a style
///   that is not one of them is refused;
/// - the file's **form** is made when the file has none, and it holds
///   the field;
/// - the **two drawings** of the field — the one that says it is off
///   and the one that says it is on — are written when the caller says
///   where they go; this tree keeps the bytes of a stream in a file of
///   their own.
#[derive(Command)]
#[command(id = "pdf.field.checkbox.add", area = "pdf", layer = "pdf", icon = "plus", mutates, undoable)]
pub struct AddCheckBox {
    /// The page the check box is put on.
    pub page: NodeRef,
    /// The check box's object number in the file.
    pub object: Number,
    /// The rectangle the check box covers: four numbers.
    pub rect: serde_json::Value,
    /// The name of the check box.
    ///
    /// Leave it out and the check box is given the first free name of
    /// the tree: `Check Box1`, `Check Box2` and so on.
    pub name: Option<Text>,
    /// The **style** of the mark: `Check`, `Circle`, `Cross`,
    /// `Diamond`, `Square` or `Star`.
    ///
    /// Leave it out and the mark is a check.
    pub style: Option<Text>,
    /// Where the bytes of the drawing that says the box is **on** are
    /// written.
    pub on_to: Option<Text>,
    /// Where the bytes of the drawing that says the box is **off** are
    /// written.
    pub off_to: Option<Text>,
}

impl AddCheckBox {
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
        // The **object number** of the check box.
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
                message: "the check box is too small".to_string(),
            });
        }
        // The **style** of the mark.
        let style = match &self.style {
            Some(text) => CheckStyle::parse(&text.0).ok_or_else(|| EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!(
                    "`{}` is not a style of a mark; the tree's own are Check, Circle, Cross, Diamond, Square and Star",
                    text.0
                ),
            })?,
            None => CheckStyle::Check,
        };
        // The **name**.
        let existing: Vec<String> =
            crate::fields::fields_of(session.document()).into_iter().map(|field| field.name).collect();
        let name = match self.name.as_ref() {
            Some(text) => {
                let name = text.0.trim().to_string();
                if name.is_empty() {
                    free_name(&existing, "Check Box")
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
            None => free_name(&existing, "Check Box"),
        };
        if existing.contains(&name) {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("a field named `{name}` already exists"),
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
                    message: "a check box has two drawings, and both are written or neither".to_string(),
                });
            }
        }
        // The **form**: it holds the field, and it is made when the
        // file has none. The form and its fonts are objects of their
        // own, and they are made before the field, so that the field
        // holds the number the caller gave it.
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
        // The **check box** itself: the tree's `widget_dict`, with the
        // check box's own entries.
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
            ("FT".to_string(), json!("Btn")),
            ("V".to_string(), json!(OFF)),
            ("AS".to_string(), json!(OFF)),
            ("DA".to_string(), json!({ "string": "/ZaDb 0 Tf 0 g" })),
            ("T".to_string(), json!({ "string": name.clone() })),
        ];
        let data = json!({ "object": number, "entries": crate::pairs_of(&entries) });
        let widget =
            session.document_mut()?.add_node_with(format!("field {number}"), "pdf.object", None, data);
        // The numbers of the **two drawings**: the streams that hold
        // them. They are numbers of their own, above the field's.
        let mut numbers: Vec<u64> = Vec::new();
        if drawings.is_some() {
            let first = crate::free::the_next_number(session.document(), 0);
            let second = crate::free::the_next_number(session.document(), first);
            numbers.push(first);
            numbers.push(second);
        }
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
        // The **two drawings** of the field: the one that says it is
        // off, and the one that says it is on.
        let mut appearance = None;
        if let Some((on, off, on_bytes, off_bytes)) = drawings {
            let mut states: Vec<(String, Json)> = Vec::new();
            let mut streams: Vec<(String, u64)> = Vec::new();
            let drawings = [(ON, on, on_bytes), (OFF, off, off_bytes)];
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
                states.push((state.to_string(), json!({ "ref": [number, 0] })));
                streams.push((state.to_string(), stream));
                // The **edge**: the field names the drawing, and says
                // which state it is.
                let relation = craftkit_sdk::Relation::with_payload(
                    widget,
                    stream,
                    "ref",
                    json!({ "key": "AP", "state": state, "generation": 0 }),
                );
                if !session.document_mut()?.add_relation(relation) {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: format!("the field `{widget}` already refers to a drawing"),
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
            // The **numbers** of the drawings are said too: they are
            // numbers of the file, and the caller may want them.
        }
        Ok(Outcome::changed(format!("added the check box `{name}` to the page")).with_data(json!({
            "id": widget,
            "object": number,
            "page": self.page.0,
            "name": name,
            "rect": rect,
            "style": style.label(),
            "code": style.code(),
            "form": form.node,
            "appearance": appearance,
            "drawings": numbers,
        })))
    }
}
