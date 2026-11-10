//! The **appearance** of a shape: the square and the circle.
//!
//! The tree's own: an annotation of the kind `Square` or `Circle` is a
//! **shape**, and a shape is **drawn** — the border of it and the fill
//! of it — and the drawing is an appearance, a form XObject that the
//! reader shows where the annotation is.
//!
//! The rules of the tree, ported by name:
//!
//! - `build` — the drawing is made from the **dictionary** of the
//!   annotation: the **rectangle** it covers, the **colour** of its
//!   line (`/C`), the **colour** of its fill (`/IC`), the **width** of
//!   its line (`/BS /W`) and its **dashes** (`/BS /D`), and the
//!   **opacity** of it (`/CA`);
//! - `color` — a colour is one number (a grey), three numbers (red,
//!   green and blue) or four (cyan, magenta, yellow and black), and
//!   four numbers are turned into the red, the green and the blue of
//!   the screen;
//! - `border_width` — the width of the line is the width the
//!   dictionary says, or the third number of the old `/Border`, and
//!   one point when the dictionary says nothing;
//! - `dash` — a **dashed** line has the dashes the dictionary says,
//!   and three points when it says nothing;
//! - the line is **inset** by half its width, so that the line is
//!   drawn inside the rectangle and not over its edge.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Outcome, Session, Text};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.annot.appearance.write";

/// The **entries** of a node's dictionary.
fn entries_of(node: &Node) -> Vec<(String, Json)> {
    crate::entries_of(&node.data)
}

/// The **entry** of a dictionary, by its key.
fn entry<'a>(entries: &'a [(String, Json)], key: &str) -> Option<&'a Json> {
    entries.iter().find(|(name, _)| name == key).map(|(_, value)| value)
}

/// The **numbers** a value holds.
fn numbers_of(value: &Json) -> Vec<f64> {
    value.as_array().map(|list| list.iter().filter_map(|item| item.as_f64()).collect()).unwrap_or_default()
}

/// The **pairs** of an inline dictionary.
fn dictionary_of(value: &Json) -> Vec<(String, Json)> {
    let Some(list) = value.get("entries").and_then(|entries| entries.as_array()) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|entry| {
            let pair = entry.as_array()?;
            Some((pair.first()?.as_str()?.to_string(), pair.get(1)?.clone()))
        })
        .collect()
}

/// A **colour** of a shape: the red, the green and the blue.
///
/// The tree's own `color`: one number is a grey, three numbers are
/// red, green and blue, and four numbers are cyan, magenta, yellow and
/// black — which are turned into the red, the green and the blue of
/// the screen.
fn the_colour(value: &Json) -> Result<Option<[f64; 3]>, EngineError> {
    let numbers = numbers_of(value);
    if numbers.len() != value.as_array().map(|list| list.len()).unwrap_or(numbers.len()) {
        return Err(EngineError::Failed {
            id: COMMAND.to_string(),
            message: "a colour is one number, three numbers or four numbers".to_string(),
        });
    }
    if numbers.iter().any(|number| !number.is_finite()) {
        return Err(EngineError::Failed {
            id: COMMAND.to_string(),
            message: "a colour is made of numbers that are numbers".to_string(),
        });
    }
    let colour = match numbers.as_slice() {
        [] => None,
        [grey] => Some([*grey; 3]),
        [red, green, blue] => Some([*red, *green, *blue]),
        [cyan, magenta, yellow, black] => Some([
            (1.0 - cyan) * (1.0 - black),
            (1.0 - magenta) * (1.0 - black),
            (1.0 - yellow) * (1.0 - black),
        ]),
        _ => {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "a colour is one number, three numbers or four numbers".to_string(),
            });
        }
    };
    Ok(colour)
}

/// The **width** of the line of a shape: the tree's own
/// `border_width`.
fn the_width(entries: &[(String, Json)]) -> f64 {
    if let Some(border) = entry(entries, "BS") {
        let border = dictionary_of(border);
        if let Some(width) = entry(&border, "W").and_then(|width| width.as_f64()) {
            return width.max(0.0);
        }
    }
    // The old `/Border`: three numbers, and the third is the width.
    if let Some(border) = entry(entries, "Border") {
        let numbers = numbers_of(border);
        if numbers.len() >= 3 {
            return numbers[2].max(0.0);
        }
    }
    1.0
}

/// The **dashes** of a line: the tree's own `dash`.
fn the_dash(entries: &[(String, Json)]) -> String {
    let Some(border) = entry(entries, "BS") else { return String::new() };
    let border = dictionary_of(border);
    if entry(&border, "S").and_then(|style| style.as_str()) != Some("D") {
        return String::new();
    }
    let dashes = entry(&border, "D").map(numbers_of).unwrap_or_default();
    let dashes: Vec<f64> = if dashes.is_empty() || dashes.iter().any(|dash| !dash.is_finite() || *dash < 0.0)
    {
        vec![3.0]
    } else {
        dashes
    };
    format!(
        "[{}] 0 d\n",
        dashes.iter().map(|dash| crate::watermark::number(*dash)).collect::<Vec<_>>().join(" ")
    )
}

/// An **ellipse**, as the tree draws it: four curves.
fn the_ellipse(x0: f64, y0: f64, x1: f64, y1: f64) -> String {
    let k = 0.552_284_75;
    let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let (rx, ry) = ((x1 - x0) / 2.0, (y1 - y0) / 2.0);
    let (ox, oy) = (rx * k, ry * k);
    let number = crate::watermark::number;
    let mut out = format!("{} {} m\n", number(cx + rx), number(cy));
    for (a, b, c, d, e, f) in [
        (cx + rx, cy + oy, cx + ox, cy + ry, cx, cy + ry),
        (cx - ox, cy + ry, cx - rx, cy + oy, cx - rx, cy),
        (cx - rx, cy - oy, cx - ox, cy - ry, cx, cy - ry),
        (cx + ox, cy - ry, cx + rx, cy - oy, cx + rx, cy),
    ] {
        out.push_str(&format!(
            "{} {} {} {} {} {} c\n",
            number(a),
            number(b),
            number(c),
            number(d),
            number(e),
            number(f)
        ));
    }
    out.push_str("h\n");
    out
}

/// The **drawing** of a shape: the tree's own square and circle.
///
/// The rectangle is the one of the annotation, the corners in order,
/// and the line is inset by half its width.
fn the_drawing(entries: &[(String, Json)], subtype: &str) -> Option<(Vec<u8>, [f64; 4])> {
    let rect = numbers_of(entry(entries, "Rect")?);
    if rect.len() != 4 {
        return None;
    }
    let rect = [rect[0].min(rect[2]), rect[1].min(rect[3]), rect[0].max(rect[2]), rect[1].max(rect[3])];
    let fill = match entry(entries, "IC") {
        Some(value) => the_colour(value).ok()?,
        None => None,
    };
    let stroke = match entry(entries, "C") {
        Some(value) => the_colour(value).ok()?,
        None => None,
    };
    // The **opacity** of the shape.
    let opacity = match entry(entries, "CA").and_then(|value| value.as_f64()) {
        Some(value) if !value.is_finite() || !(0.0..=1.0).contains(&value) => return None,
        Some(value) => value,
        None => 1.0,
    };
    let width = the_width(entries);
    let mut out = String::new();
    if opacity < 1.0 {
        out.push_str("/GS0 gs\n");
    }
    if fill.is_none() && stroke.is_none() {
        return Some((out.into_bytes(), rect));
    }
    let inset = width / 2.0;
    let (x0, y0, x1, y1) = (rect[0] + inset, rect[1] + inset, rect[2] - inset, rect[3] - inset);
    if let Some(fill) = fill {
        out.push_str(&format!(
            "{} {} {} rg\n",
            crate::watermark::number(fill[0]),
            crate::watermark::number(fill[1]),
            crate::watermark::number(fill[2])
        ));
    }
    if let Some(stroke) = stroke {
        out.push_str(&format!(
            "{} {} {} RG\n{} w\n{}",
            crate::watermark::number(stroke[0]),
            crate::watermark::number(stroke[1]),
            crate::watermark::number(stroke[2]),
            crate::watermark::number(width),
            the_dash(entries)
        ));
    }
    if subtype == "Square" {
        out.push_str(&format!(
            "{} {} {} {} re\n",
            crate::watermark::number(x0),
            crate::watermark::number(y0),
            crate::watermark::number(x1 - x0),
            crate::watermark::number(y1 - y0)
        ));
    } else {
        out.push_str(&the_ellipse(x0, y0, x1, y1));
    }
    out.push_str(match (fill.is_some(), stroke.is_some() && width > 0.0) {
        (true, true) => "B\n",
        (true, false) => "f\n",
        (false, true) => "S\n",
        (false, false) => "n\n",
    });
    Some((out.into_bytes(), rect))
}

/// **Write** the **appearance** of a shape.
///
/// The appearance is a **form XObject** — a little drawing — and the
/// annotation's `/AP` names it under the key `N`. The bytes of the
/// drawing are written to a file of their own, beside the document.
///
/// The rules:
///
/// - the annotation must be a shape — an annotation of the kind
///   `Square` or `Circle` — and any other kind is refused;
/// - the **rectangle** of the shape is the one the annotation holds,
///   and the corners may come in any order;
/// - a **colour** is one number, three numbers or four numbers, and
///   anything else is refused;
/// - the **opacity** of the shape is between zero and one.
#[derive(Command)]
#[command(id = "pdf.annot.appearance.write", area = "pdf", layer = "pdf", icon = "pencil", mutates, undoable)]
pub struct WriteShape {
    /// The shape.
    pub annotation: NodeRef,
    /// Where the bytes of the drawing are written.
    pub to: Text,
}

impl WriteShape {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let annotation = self.annotation.0;
        let Some(node) = session.document().node(annotation).cloned() else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("there is no node `{annotation}`"),
            });
        };
        let entries = entries_of(&node);
        let subtype =
            entry(&entries, "Subtype").and_then(|subtype| subtype.as_str()).unwrap_or_default().to_string();
        if subtype != "Square" && subtype != "Circle" {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("the appearance of an annotation of the kind `{subtype}` is not drawn here"),
            });
        }
        let path = self.to.0.trim().to_string();
        if path.is_empty() {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "the drawing goes somewhere".to_string(),
            });
        }
        let Some((bytes, rect)) = the_drawing(&entries, &subtype) else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("the shape `{annotation}` cannot be drawn"),
            });
        };
        std::fs::write(&path, &bytes).map_err(|error| EngineError::Failed {
            id: COMMAND.to_string(),
            message: format!("the drawing cannot be written to `{path}`: {error}"),
        })?;
        // The **appearance**: a form XObject, and the resources it
        // needs — the graphics state of the opacity.
        let opacity = entry(&entries, "CA").and_then(|value| value.as_f64()).unwrap_or(1.0);
        let resources = if opacity < 1.0 {
            json!({
                "entries": [
                    [
                        "ExtGState",
                        {
                            "entries": [
                                ["Type", "ExtGState"],
                                ["CA", opacity],
                                ["ca", opacity],
                            ]
                        },
                    ],
                ]
            })
        } else {
            json!({ "entries": [] })
        };
        let number = crate::free::the_next_number(session.document(), 0);
        let entries = vec![
            ("Type".to_string(), json!("XObject")),
            ("Subtype".to_string(), json!("Form")),
            ("FormType".to_string(), json!(1)),
            ("BBox".to_string(), json!(rect)),
            ("Resources".to_string(), resources),
        ];
        let data = json!({ "object": number, "entries": crate::pairs_of(&entries) });
        let stream =
            session.document_mut()?.add_node_with(format!("object {number}"), "pdf.stream", None, data);
        if let Some(node) = session.document_mut()?.node_mut(stream) {
            node.stream = Some(path.clone());
        }
        // The **annotation** names the drawing.
        let mut entries = crate::entries_of(
            &session
                .document()
                .node(annotation)
                .ok_or_else(|| EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: format!("there is no node `{annotation}`"),
                })?
                .data,
        );
        let normal = json!({ "entries": [ ["N", { "ref": [number, 0] } ] ] });
        match entries.iter_mut().find(|(key, _)| key == "AP") {
            Some(entry) => entry.1 = normal,
            None => entries.push(("AP".to_string(), normal)),
        }
        let data = json!({
            "object": session.document().node(annotation).and_then(|node| node.data.get("object").cloned()).unwrap_or(Json::Null),
            "entries": crate::pairs_of(&entries),
        });
        if let Some(node) = session.document_mut()?.node_mut(annotation) {
            node.data = data;
        }
        // The **edge**: the annotation refers to the drawing.
        let relation = craftkit_sdk::Relation::with_payload(
            annotation,
            stream,
            "ref",
            json!({ "key": "AP", "generation": 0 }),
        );
        if !session.document_mut()?.add_relation(relation) {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("the annotation `{annotation}` already refers to its drawing"),
            });
        }
        Ok(Outcome::changed(format!("the shape `{annotation}` has an appearance")).with_data(json!({
            "annotation": annotation,
            "appearance": stream,
            "object": number,
            "path": path,
            "bytes": bytes.len(),
            "rect": rect,
            "opacity": opacity,
        })))
    }
}
