//! The **appearance** of an annotation: the shape and the markup.
//!
//! The tree's own: an annotation of the kind `Square` or `Circle` is a
//! **shape**, and a shape is **drawn** — the border of it and the fill
//! of it — and the drawing is an appearance, a form XObject that the
//! reader shows where the annotation is. An annotation of the kind
//! `Highlight`, `Underline`, `StrikeOut` or `Squiggly` is a **markup**,
//! and a markup is drawn over the text it covers.
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
//!
//! The **markup** is the tree's own markup branch, and its rules are:
//!
//! - the **quads** (`/QuadPoints`) are the lines of the text the
//!   markup covers, and there are **eight** numbers to a quad — the
//!   upper left, the upper right, the lower left and the lower right —
//!   and a number of quads that is not a whole number of quads is
//!   refused;
//! - a markup has a **colour**, and a markup without one is refused;
//! - a **highlight** is a filled quadrilateral over the text, and it
//!   **multiplies** over the text — the tree's own rule, and the file
//!   says so with the blend mode `/Multiply`;
//! - an **underline** and a **strikeout** are a **line** across the
//!   text: the underline lies eight percent of the way up the quad,
//!   the strikeout forty two percent, and the line is seven percent
//!   of the height of the text wide, and never less than half a point
//!   and never more than three;
//! - a **squiggly** is a **zigzag** along the bottom of the quad: the
//!   step is a quarter of the height of the text and never less than
//!   point and a half, the zigzag is five percent of the height wide
//!   and never less than half a point and never more than two, and
//!   the rise of each step is six percent of the height of the text
//!   and never less than three quarters of a point.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Outcome, Session, Text};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.annot.appearance.write";

/// The kinds of **markup**, the tree's own four.
const MARKS: [&str; 4] = ["Highlight", "Underline", "StrikeOut", "Squiggly"];

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
        (cx - rx, cy - oy, cx - ox, cy - ry, cx - rx, cy - ry),
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
fn the_shape(entries: &[(String, Json)], subtype: &str) -> Option<(Vec<u8>, [f64; 4])> {
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
    let width = the_width(entries);
    let mut out = String::new();
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

/// The **drawing** of a **markup**: the tree's own markup branch.
///
/// A quad is eight numbers — the upper left, the upper right, the
/// lower left and the lower right — and the text of a quad is the
/// height between its top and its bottom.
fn the_markup(entries: &[(String, Json)], subtype: &str) -> Option<(Vec<u8>, [f64; 4])> {
    let rect = numbers_of(entry(entries, "Rect")?);
    if rect.len() != 4 {
        return None;
    }
    let rect = [rect[0].min(rect[2]), rect[1].min(rect[3]), rect[0].max(rect[2]), rect[1].max(rect[3])];
    // The **quads**: eight numbers to a quad, and a whole number of
    // quads.
    let quads = numbers_of(entry(entries, "QuadPoints")?);
    if quads.is_empty() || !quads.len().is_multiple_of(8) {
        return None;
    }
    // A markup has a **colour**, and one without a colour is refused.
    let colour = the_colour(entry(entries, "C")?).ok().flatten()?;
    let number = crate::watermark::number;
    let mut out = String::new();
    for quad in quads.as_chunks::<8>().0 {
        let point = |index: usize| (quad[index * 2], quad[index * 2 + 1]);
        let (p1, p2, p3, p4) = (point(0), point(1), point(2), point(3));
        // The **height** of the text of the quad.
        let height = (p1.0 - p3.0).hypot(p1.1 - p3.1);
        // A point a fraction `t` of the way from the bottom edge to
        // the top edge.
        let at = |bottom: (f64, f64), top: (f64, f64), t: f64| {
            (bottom.0 + (top.0 - bottom.0) * t, bottom.1 + (top.1 - bottom.1) * t)
        };
        match subtype {
            "Highlight" => {
                out.push_str(&format!(
                    "{} {} {} rg\n",
                    number(colour[0]),
                    number(colour[1]),
                    number(colour[2])
                ));
                out.push_str(&format!(
                    "{} {} m {} {} l {} {} l {} {} l h f\n",
                    number(p1.0),
                    number(p1.1),
                    number(p2.0),
                    number(p2.1),
                    number(p4.0),
                    number(p4.1),
                    number(p3.0),
                    number(p3.1)
                ));
            }
            "Underline" | "StrikeOut" => {
                let line_width = (height * 0.07).clamp(0.5, 3.0);
                let fraction = if subtype == "Underline" { 0.08 } else { 0.42 };
                let (from, to) = (at(p3, p1, fraction), at(p4, p2, fraction));
                out.push_str(&format!(
                    "{} {} {} RG\n{} w\n{} {} m {} {} l S\n",
                    number(colour[0]),
                    number(colour[1]),
                    number(colour[2]),
                    number(line_width),
                    number(from.0),
                    number(from.1),
                    number(to.0),
                    number(to.1)
                ));
            }
            _ => {
                // The **squiggly**: a zigzag along the bottom of the
                // quad.
                let line_width = (height * 0.05).clamp(0.5, 2.0);
                let rise = (height * 0.06).max(0.75);
                let length = (p4.0 - p3.0).hypot(p4.1 - p3.1);
                let step = (height / 4.0).max(1.5);
                let steps = (length / step).ceil().clamp(1.0, 10_000.0) as usize;
                out.push_str(&format!(
                    "{} {} {} RG\n{} w 1 j\n",
                    number(colour[0]),
                    number(colour[1]),
                    number(colour[2]),
                    number(line_width)
                ));
                for index in 0..=steps {
                    let fraction = index as f64 / steps as f64;
                    let base = (p3.0 + (p4.0 - p3.0) * fraction, p3.1 + (p4.1 - p3.1) * fraction);
                    let up = if index % 2 == 0 { rise * 2.0 } else { 0.0 };
                    let (ux, uy) = if height > 0.0 {
                        ((p1.0 - p3.0) / height, (p1.1 - p3.1) / height)
                    } else {
                        (0.0, 1.0)
                    };
                    out.push_str(&format!(
                        "{} {} {}\n",
                        number(base.0 + ux * up),
                        number(base.1 + uy * up),
                        if index == 0 { "m" } else { "l" }
                    ));
                }
                out.push_str("S\n");
            }
        }
    }
    Some((out.into_bytes(), rect))
}

/// The **drawing** of an annotation: the shape or the markup.
fn the_drawing(entries: &[(String, Json)], subtype: &str) -> Option<(Vec<u8>, [f64; 4])> {
    if MARKS.contains(&subtype) { the_markup(entries, subtype) } else { the_shape(entries, subtype) }
}

/// **Write** the **appearance** of an annotation.
///
/// The appearance is a **form XObject** — a little drawing — and the
/// annotation's `/AP` names it under the key `N`. The bytes of the
/// drawing are written to a file of their own, beside the document.
///
/// The rules:
///
/// - the annotation must be one this command draws — a **shape**
///   (`Square` or `Circle`) or a **markup** (`Highlight`,
///   `Underline`, `StrikeOut` or `Squiggly`) — and any other kind is
///   refused;
/// - the **rectangle** of the shape is the one the annotation holds,
///   and the corners may come in any order;
/// - a **colour** is one number, three numbers or four numbers, and
///   anything else is refused;
/// - the **opacity** of the shape is between zero and one;
/// - a markup needs its **quads** and its **colour**, and one without
///   either is refused;
/// - a **highlight multiplies** over the text, the tree's own rule,
///   and the file says so with the blend mode.
#[derive(Command)]
#[command(id = "pdf.annot.appearance.write", area = "pdf", layer = "pdf", icon = "pencil", mutates, undoable)]
pub struct WriteAnnotation {
    /// The annotation.
    pub annotation: NodeRef,
    /// Where the bytes of the drawing are written.
    pub to: Text,
}

impl WriteAnnotation {
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
        if subtype != "Square" && subtype != "Circle" && !MARKS.contains(&subtype.as_str()) {
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
        // The **opacity** of the shape, and the tree's own rule that a
        // highlight **multiplies** over the text.
        let opacity = match entry(&entries, "CA").and_then(|value| value.as_f64()) {
            Some(value) if !value.is_finite() || !(0.0..=1.0).contains(&value) => {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "the opacity of an annotation is between zero and one".to_string(),
                });
            }
            Some(value) => value,
            None => 1.0,
        };
        let multiply = subtype == "Highlight";
        let Some((bytes, rect)) = the_drawing(&entries, &subtype) else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("the annotation `{annotation}` cannot be drawn"),
            });
        };
        // The **graphics state**: the opacity, and the blend mode of a
        // highlight.
        let state = opacity < 1.0 || multiply;
        let mut bytes = bytes;
        if state {
            let mut prefix = b"/GS0 gs\n".to_vec();
            prefix.append(&mut bytes);
            bytes = prefix;
        }
        std::fs::write(&path, &bytes).map_err(|error| EngineError::Failed {
            id: COMMAND.to_string(),
            message: format!("the drawing cannot be written to `{path}`: {error}"),
        })?;
        // The **appearance**: a form XObject, and the resources it
        // needs — the graphics state of the opacity.
        let resources = if state {
            let mut entries = vec![
                ("Type".to_string(), json!("ExtGState")),
                ("CA".to_string(), json!(opacity)),
                ("ca".to_string(), json!(opacity)),
            ];
            if multiply {
                entries.insert(1, ("BM".to_string(), json!("Multiply")));
            }
            json!({
                "entries": [
                    [
                        "ExtGState",
                        { "entries": crate::pairs_of(&entries) },
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
        Ok(Outcome::changed(format!("the annotation `{annotation}` has an appearance")).with_data(json!({
            "annotation": annotation,
            "appearance": stream,
            "object": number,
            "path": path,
            "bytes": bytes.len(),
            "rect": rect,
            "opacity": opacity,
            "multiply": multiply,
        })))
    }
}
