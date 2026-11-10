//! The **appearance** of an annotation: the shape, the markup, the
//! line, the polygon, the ink, the caret, the text box and the mark of
//! a redaction.
//!
//! The tree's own: an annotation of the kind `Square` or `Circle` is a
//! **shape**, and a shape is **drawn** — the border of it and the fill
//! of it — and the drawing is an appearance, a form XObject that the
//! reader shows where the annotation is. An annotation of the kind
//! `Highlight`, `Underline`, `StrikeOut` or `Squiggly` is a **markup**,
//! and a markup is drawn over the text it covers. An annotation of the
//! kind `Line` is a **line**, and a line is drawn between its two
//! points, with its arrowheads. An annotation of the kind `Polygon` or
//! `PolyLine` is a **shape of corners**, and it is drawn through them
//! — with a **cloudy** border when the annotation says so. An
//! annotation of the kind `Ink` is a **hand**, and it is drawn through
//! the points of its strokes. An annotation of the kind `Caret` is a
//! **wedge**, and it is the mark that says "text was inserted here".
//! An annotation of the kind `FreeText` is a **text box**, and its own
//! text is drawn inside it. An annotation of the kind `Redact` is the
//! **mark of a redaction**, and the outline of what it covers is
//! drawn.
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
//!
//! The **line** is the tree's own line, and its rules are:
//!
//! - the **two points** of the line are the `/L` of the annotation,
//!   and a line without them is refused;
//! - a line has a **colour**, and one without a colour is refused;
//! - the **ends** of the line are the arrowheads, and they are the
//!   `/LE` of the annotation: two of them, and each one of the tree's
//!   own three — `None`, `OpenArrow` and `ClosedArrow` — and any other
//!   is refused;
//! - the **size of an arrowhead** is six points plus three times the
//!   width of the line, and the two wings of it lie at thirty degrees
//!   either side of the line;
//! - the **fill** of the line is the colour of the fill when the
//!   annotation says one, and the colour of the line otherwise, and a
//!   **closed** arrowhead is filled with it.
//!
//! The **polygon** is the tree's own polygon, and its rules are:
//!
//! - the **corners** of the shape are the `/Vertices` of the
//!   annotation, and there are an even number of them and at least
//!   two, and a shape without corners is refused;
//! - the **colour** of the line and the **fill** of the shape are the
//!   `/C` and the `/IC` of the annotation, and a polygon may have
//!   neither;
//! - a **cloudy border** is the tree's own `/BE /S /C`, and it is
//!   drawn as a row of **bumps** along the edges of the shape, and the
//!   **radius** of a bump is six points plus one and a half times the
//!   width of the line, and the **intensity** of the cloud (`/BE /I`)
//!   scales it and is between a half and two;
//! - a cloudy border is a **polygon** only: the tree draws it for
//!   nothing else, and one that is not a polygon is refused;
//! - the **ends** of a connected line are not drawn yet, and a
//!   connected line with an end that is not `None` is refused.
//!
//! The **ink** is the tree's own ink, and its rules are:
//!
//! - the **strokes** are the `/InkList` of the annotation, a list of
//!   lists of numbers, and each stroke is drawn on its own;
//! - the ink has a **colour**, and one without a colour is refused;
//! - a stroke of **one point** is a dot, and the tree draws it as the
//!   point and a hair along it, so that a reader shows a mark.
//!
//! The **caret** is the tree's own caret, and its rules are:
//!
//! - the caret is a **filled wedge**: two curved flanks that meet at
//!   the top centre of the rectangle, and the flanks are a tenth of
//!   the width of the rectangle from the sides and a fifth of its
//!   height from the bottom;
//! - the **colour** of the caret is the `/C` of the annotation, and
//!   the tree's own blue when the annotation says none.
//!
//! The **mark of a redaction** is the tree's own mark, and its rules
//! are:
//!
//! - the **quads** of the mark are the `/QuadPoints` of the
//!   annotation, and the **outline** of each of them is drawn: the
//!   fill comes when the redaction is applied, and not before;
//! - the **colour** of the mark is the `/C` of the annotation, and the
//!   tree's own red when the annotation says none.

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

/// The **size of an arrowhead** for a line of the width `w`: the tree's
/// own `arrow_size`.
fn the_arrow_size(width: f64) -> f64 {
    6.0 + 3.0 * width.max(0.0)
}

/// The **end of a line**: the tree's own `line_end`.
///
/// The arrowhead is drawn at the `tip`, and its two wings lie at
/// thirty degrees either side of the line that arrives at the tip.
fn the_line_end(out: &mut String, kind: &str, tip: (f64, f64), from: (f64, f64), width: f64) {
    if kind == "None" {
        return;
    }
    let (dx, dy) = (from.0 - tip.0, from.1 - tip.1);
    let length = dx.hypot(dy);
    if length == 0.0 {
        return;
    }
    let (ux, uy) = (dx / length, dy / length);
    let size = the_arrow_size(width);
    let (cos, sin) = (30f64.to_radians().cos(), 30f64.to_radians().sin());
    let a = (tip.0 + size * (ux * cos - uy * sin), tip.1 + size * (ux * sin + uy * cos));
    let b = (tip.0 + size * (ux * cos + uy * sin), tip.1 + size * (-ux * sin + uy * cos));
    let number = crate::watermark::number;
    let paint = if kind == "ClosedArrow" { "h B" } else { "S" };
    out.push_str(&format!(
        "{} {} m {} {} l {} {} l {paint}\n",
        number(a.0),
        number(a.1),
        number(tip.0),
        number(tip.1),
        number(b.0),
        number(b.1)
    ));
}

/// The **drawing** of a **line**: the tree's own line.
///
/// The two points of the line are the `/L` of the annotation, and the
/// ends of the line — the arrowheads — are the `/LE` of it.
fn the_line(entries: &[(String, Json)]) -> Option<(Vec<u8>, [f64; 4])> {
    let rect = numbers_of(entry(entries, "Rect")?);
    if rect.len() != 4 {
        return None;
    }
    let rect = [rect[0].min(rect[2]), rect[1].min(rect[3]), rect[0].max(rect[2]), rect[1].max(rect[3])];
    // The **two points** of the line.
    let points = numbers_of(entry(entries, "L")?);
    if points.len() != 4 {
        return None;
    }
    // A line has a **colour**, and one without a colour is refused.
    let colour = the_colour(entry(entries, "C")?).ok().flatten()?;
    // The **ends** of the line: the arrowheads. Two of them, and each
    // one of the tree's own three.
    let ends: Vec<String> = match entry(entries, "LE") {
        None => vec!["None".to_string(), "None".to_string()],
        Some(value) => {
            let list = value.as_array()?;
            let mut ends = Vec::new();
            for end in list {
                ends.push(end.as_str()?.to_string());
            }
            ends
        }
    };
    if ends.len() != 2 || ends.iter().any(|end| !matches!(end.as_str(), "None" | "OpenArrow" | "ClosedArrow"))
    {
        return None;
    }
    // The **fill** of the line: the colour of the fill when the
    // annotation says one, and the colour of the line otherwise — the
    // tree's own rule, and a closed arrowhead is filled with it.
    let fill = match entry(entries, "IC") {
        Some(value) => the_colour(value).ok().flatten()?,
        None => colour,
    };
    let width = the_width(entries);
    let number = crate::watermark::number;
    let mut out = String::new();
    out.push_str(&format!(
        "{} {} {} RG\n{} {} {} rg\n{} w 1 J 1 j\n{}",
        number(colour[0]),
        number(colour[1]),
        number(colour[2]),
        number(fill[0]),
        number(fill[1]),
        number(fill[2]),
        number(width),
        the_dash(entries)
    ));
    out.push_str(&format!(
        "{} {} m {} {} l S\n[] 0 d\n",
        number(points[0]),
        number(points[1]),
        number(points[2]),
        number(points[3])
    ));
    // The **arrowheads**: one at each end of the line.
    the_line_end(&mut out, &ends[0], (points[0], points[1]), (points[2], points[3]), width);
    the_line_end(&mut out, &ends[1], (points[2], points[3]), (points[0], points[1]), width);
    Some((out.into_bytes(), rect))
}

/// The **radius of a cloud**: the tree's own `cloud_radius` — six
/// points plus one and a half times the width of the line.
fn the_cloud_radius(width: f64) -> f64 {
    6.0 + 1.5 * width.max(0.0)
}

/// Whether the border of the annotation is **cloudy**: the tree's own
/// `/BE /S /C`.
fn the_cloudy(entries: &[(String, Json)]) -> bool {
    let Some(border) = entry(entries, "BE") else { return false };
    dictionary_of(border).iter().any(|(key, value)| key == "S" && value.as_str() == Some("C"))
}

/// The **cloud** of a border: a closed outline through the points, and
/// each edge of it is a row of half-circle bumps.
///
/// The tree's own `cloud_path`: the bumps bulge **outwards**, and the
/// side they bulge to is the one the outline turns to.
fn the_cloud_path(points: &[(f64, f64)], radius: f64) -> String {
    let number = crate::watermark::number;
    // The **area** of the outline: its sign says which way it turns.
    let area: f64 =
        points.iter().zip(points.iter().cycle().skip(1)).map(|(a, b)| a.0 * b.1 - b.0 * a.1).sum();
    let sign = if area >= 0.0 { 1.0 } else { -1.0 };
    let mut out = String::new();
    for (index, (a, b)) in points.iter().zip(points.iter().cycle().skip(1)).enumerate() {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let length = dx.hypot(dy);
        if index == 0 {
            out.push_str(&format!("{} {} m\n", number(a.0), number(a.1)));
        }
        if length < 1e-6 {
            continue;
        }
        let bumps = ((length / (2.0 * radius)).round() as usize).clamp(1, 10_000);
        let step = length / bumps as f64;
        let (ux, uy) = (dx / length, dy / length);
        let (nx, ny) = (sign * uy, -sign * ux);
        let h = 4.0 / 3.0 * step / 2.0;
        for bump in 0..bumps {
            let p0 = (a.0 + ux * step * bump as f64, a.1 + uy * step * bump as f64);
            let p1 = (a.0 + ux * step * (bump + 1) as f64, a.1 + uy * step * (bump + 1) as f64);
            out.push_str(&format!(
                "{} {} {} {} {} {} c\n",
                number(p0.0 + nx * h),
                number(p0.1 + ny * h),
                number(p1.0 + nx * h),
                number(p1.1 + ny * h),
                number(p1.0),
                number(p1.1)
            ));
        }
    }
    out.push_str("h\n");
    out
}

/// The **drawing** of a **polygon** and of the tree's **connected
/// lines**: the tree's own polygon branch.
///
/// The **corners** of the shape are the `/Vertices` of the annotation,
/// and a **cloudy** border is drawn as a row of bumps along its edges.
fn the_polygon(entries: &[(String, Json)], subtype: &str) -> Option<(Vec<u8>, [f64; 4])> {
    let rect = numbers_of(entry(entries, "Rect")?);
    if rect.len() != 4 {
        return None;
    }
    let rect = [rect[0].min(rect[2]), rect[1].min(rect[3]), rect[0].max(rect[2]), rect[1].max(rect[3])];
    // The **corners**: an even number of them, and at least two.
    let vertices = numbers_of(entry(entries, "Vertices")?);
    if vertices.len() < 4 || !vertices.len().is_multiple_of(2) {
        return None;
    }
    let points: Vec<(f64, f64)> =
        vertices.as_chunks::<2>().0.iter().map(|point| (point[0], point[1])).collect();
    let closed = subtype == "Polygon";
    // The **colour** of the line: a polygon may have none.
    let stroke = match entry(entries, "C") {
        Some(value) => the_colour(value).ok()?,
        None => None,
    };
    // The **fill**: a polygon may have one, and a connected line never
    // has.
    let fill = match entry(entries, "IC") {
        Some(value) => the_colour(value).ok()?,
        None => None,
    };
    // The tree does not draw the **ends** of a connected line yet, and
    // a connected line with an end that is not `None` is not drawn.
    if !closed && let Some(value) = entry(entries, "LE") {
        let unknown = match value.as_array() {
            None => true,
            Some(list) => list.iter().any(|end| end.as_str() != Some("None")),
        };
        if unknown {
            return None;
        }
    }
    let width = the_width(entries);
    let number = crate::watermark::number;
    let mut out = String::new();
    if let Some(fill) = fill {
        out.push_str(&format!("{} {} {} rg\n", number(fill[0]), number(fill[1]), number(fill[2])));
    }
    if let Some(stroke) = stroke {
        out.push_str(&format!(
            "{} {} {} RG\n{} w 1 J 1 j\n{}",
            number(stroke[0]),
            number(stroke[1]),
            number(stroke[2]),
            number(width),
            the_dash(entries)
        ));
    }
    if the_cloudy(entries) {
        // The **intensity** of the cloud, and the tree's own bounds of
        // it.
        let intensity = entry(entries, "BE")
            .map(dictionary_of)
            .and_then(|border| {
                border.iter().find(|(key, _)| key == "I").and_then(|(_, value)| value.as_f64())
            })
            .unwrap_or(1.0);
        let radius = the_cloud_radius(width) * intensity.clamp(0.5, 2.0);
        out.push_str(&the_cloud_path(&points, radius));
    } else {
        for (index, point) in points.iter().enumerate() {
            out.push_str(&format!(
                "{} {} {}\n",
                number(point.0),
                number(point.1),
                if index == 0 { "m" } else { "l" }
            ));
        }
        if closed {
            out.push_str("h\n");
        }
    }
    out.push_str(match (fill.is_some(), stroke.is_some() && width > 0.0) {
        (true, true) => "B\n",
        (true, false) => "f\n",
        (false, true) => "S\n",
        (false, false) => "n\n",
    });
    Some((out.into_bytes(), rect))
}

/// The **drawing** of the **ink**: the tree's own ink.
///
/// The strokes are the `/InkList` of the annotation, and each stroke
/// is drawn on its own.
fn the_ink(entries: &[(String, Json)]) -> Option<(Vec<u8>, [f64; 4])> {
    let rect = numbers_of(entry(entries, "Rect")?);
    if rect.len() != 4 {
        return None;
    }
    let rect = [rect[0].min(rect[2]), rect[1].min(rect[3]), rect[0].max(rect[2]), rect[1].max(rect[3])];
    // The ink has a **colour**, and one without a colour is refused.
    let colour = the_colour(entry(entries, "C")?).ok().flatten()?;
    // The **strokes**: a list of lists of numbers.
    let list = entry(entries, "InkList")?.as_array()?.clone();
    let width = the_width(entries);
    let number = crate::watermark::number;
    let mut out = String::new();
    out.push_str(&format!(
        "{} {} {} RG\n{} w 1 J 1 j\n",
        number(colour[0]),
        number(colour[1]),
        number(colour[2]),
        number(width)
    ));
    for stroke in &list {
        let points = numbers_of(stroke);
        let points: Vec<(f64, f64)> =
            points.as_chunks::<2>().0.iter().map(|point| (point[0], point[1])).collect();
        let Some(first) = points.first() else { continue };
        out.push_str(&format!("{} {} m\n", number(first.0), number(first.1)));
        if points.len() == 1 {
            // A stroke of one point is a dot: the tree draws it as a
            // point and a hair along.
            out.push_str(&format!("{} {} l\n", number(first.0 + 0.01), number(first.1)));
        }
        for point in &points[1..] {
            out.push_str(&format!("{} {} l\n", number(point.0), number(point.1)));
        }
        out.push_str("S\n");
    }
    Some((out.into_bytes(), rect))
}

/// The **drawing** of the **caret**: the tree's own caret.
///
/// The mark is a filled wedge: two curved flanks that meet at the top
/// centre of the rectangle.
fn the_caret(entries: &[(String, Json)]) -> Option<(Vec<u8>, [f64; 4])> {
    let numbers = numbers_of(entry(entries, "Rect")?);
    if numbers.len() != 4 {
        return None;
    }
    let rect = [
        numbers[0].min(numbers[2]),
        numbers[1].min(numbers[3]),
        numbers[0].max(numbers[2]),
        numbers[1].max(numbers[3]),
    ];
    // The **colour** of the caret: the tree's own blue when the
    // annotation says none.
    let colour = match entry(entries, "C") {
        Some(value) => the_colour(value).ok().flatten()?,
        None => [0.0, 0.47, 0.84],
    };
    let [x0, y0, x1, y1] = rect;
    let centre = (x0 + x1) / 2.0;
    let height = y1 - y0;
    let number = crate::watermark::number;
    let mut out = String::new();
    out.push_str(&format!("{} {} {} rg\n", number(colour[0]), number(colour[1]), number(colour[2])));
    out.push_str(&format!(
        "{} {} m\n{} {} {} {} {} {} c\n{} {} {} {} {} {} c\nh f\n",
        number(x0),
        number(y0),
        number(centre - (x1 - x0) * 0.1),
        number(y0 + height * 0.2),
        number(centre),
        number(y1 - height * 0.25),
        number(centre),
        number(y1),
        number(centre),
        number(y1 - height * 0.25),
        number(centre + (x1 - x0) * 0.1),
        number(y0 + height * 0.2),
        number(x1),
        number(y0)
    ));
    Some((out.into_bytes(), rect))
}

/// The **appearance of the text** of a text box: the tree's own
/// `parse_da`.
///
/// The `/DA` of the annotation holds the colour of the text and the
/// size of it in one string, and the tree reads the string: the three
/// numbers before an `rg` are the colour, the number before a `g` is
/// a grey, and the number before a `Tf` is the size.
fn the_da(value: Option<&Json>) -> ([f64; 3], f64) {
    let text = value.and_then(crate::fields::text_of).unwrap_or_default();
    let tokens: Vec<&str> = text.split_whitespace().collect();
    let mut colour = [0.0; 3];
    let mut size = 12.0;
    let number = |index: usize| tokens.get(index).and_then(|token| token.parse::<f64>().ok());
    for (index, token) in tokens.iter().enumerate() {
        match *token {
            "rg" if index >= 3 => {
                if let (Some(red), Some(green), Some(blue)) =
                    (number(index - 3), number(index - 2), number(index - 1))
                {
                    colour = [red, green, blue];
                }
            }
            "g" if index >= 1 => {
                if let Some(grey) = number(index - 1) {
                    colour = [grey; 3];
                }
            }
            "Tf" if index >= 1 => {
                if let Some(size_of_text) = number(index - 1).filter(|size| *size > 0.0) {
                    size = size_of_text;
                }
            }
            _ => {}
        }
    }
    (colour.map(|value| value.clamp(0.0, 1.0)), size.min(400.0))
}

/// The **drawing** of a **text box**: the tree's own free text.
///
/// The text of the box is the `/Contents` of the annotation, and the
/// colour of it and the size of it are the `/DA` of it.
fn the_freetext(entries: &[(String, Json)]) -> Option<(Vec<u8>, [f64; 4])> {
    let numbers = numbers_of(entry(entries, "Rect")?);
    if numbers.len() != 4 {
        return None;
    }
    let full = [
        numbers[0].min(numbers[2]),
        numbers[1].min(numbers[3]),
        numbers[0].max(numbers[2]),
        numbers[1].max(numbers[3]),
    ];
    // The **colour of the text** and the **size of it**: the tree's
    // own string.
    let (colour, size) = the_da(entry(entries, "DA"));
    // The **background** of the box.
    let background = match entry(entries, "C") {
        Some(value) => Some(the_colour(value).ok().flatten()?),
        None => None,
    };
    // The **width of the border**: the tree's own zero when the
    // annotation says nothing about a border.
    let border = if entry(entries, "BS").is_some() || entry(entries, "Border").is_some() {
        the_width(entries)
    } else {
        0.0
    };
    let number = crate::watermark::number;
    let mut rect = full;
    let mut out = String::new();
    // The **leader line** of a callout: the `/CL` of the annotation,
    // and the box is inside the `/RD` of it.
    if let Some(value) = entry(entries, "CL") {
        let points = numbers_of(value);
        if points.len() != 4 && points.len() != 6 {
            return None;
        }
        let inset = numbers_of(entry(entries, "RD")?);
        if inset.len() != 4 || inset.iter().any(|number| *number < 0.0) {
            return None;
        }
        rect = [full[0] + inset[0], full[1] + inset[1], full[2] - inset[2], full[3] - inset[3]];
        if rect[2] - rect[0] < 1.0 || rect[3] - rect[1] < 1.0 {
            return None;
        }
        let end = match entry(entries, "LE") {
            None => "None".to_string(),
            Some(value) => value.as_str()?.to_string(),
        };
        if !matches!(end.as_str(), "None" | "OpenArrow" | "ClosedArrow") {
            return None;
        }
        let line_width = border.max(0.5);
        let fill = background.unwrap_or([1.0; 3]);
        out.push_str(&format!(
            "{} {} {} RG\n{} {} {} rg\n{} w 1 J 1 j\n",
            number(colour[0]),
            number(colour[1]),
            number(colour[2]),
            number(fill[0]),
            number(fill[1]),
            number(fill[2]),
            number(line_width)
        ));
        for (index, point) in points.as_chunks::<2>().0.iter().enumerate() {
            out.push_str(&format!(
                "{} {} {}\n",
                number(point[0]),
                number(point[1]),
                if index == 0 { "m" } else { "l" }
            ));
        }
        out.push_str("S\n");
        the_line_end(&mut out, &end, (points[0], points[1]), (points[2], points[3]), line_width);
    }
    // The **background** of the box.
    if let Some(background) = background {
        out.push_str(&format!(
            "{} {} {} rg\n{} {} {} {} re f\n",
            number(background[0]),
            number(background[1]),
            number(background[2]),
            number(rect[0]),
            number(rect[1]),
            number(rect[2] - rect[0]),
            number(rect[3] - rect[1])
        ));
    }
    // The **border** of the box, inside it.
    if border > 0.0 {
        let half = border / 2.0;
        out.push_str(&format!(
            "{} {} {} RG\n{} w\n{} {} {} {} re S\n[] 0 d\n",
            number(colour[0]),
            number(colour[1]),
            number(colour[2]),
            number(border),
            number(rect[0] + half),
            number(rect[1] + half),
            number(rect[2] - rect[0] - border),
            number(rect[3] - rect[1] - border)
        ));
    }
    // The **text** of the box.
    let text = entry(entries, "Contents").and_then(crate::fields::text_of).unwrap_or_default();
    let pad = 2.0 + border;
    let width = (rect[2] - rect[0] - 2.0 * pad).max(1.0);
    let align = entry(entries, "Q").and_then(|value| value.as_f64()).unwrap_or(0.0);
    out.push_str(&format!(
        "{} {} {} {} re W n\nBT\n/Helv {} Tf\n{} {} {} rg\n",
        number(rect[0]),
        number(rect[1]),
        number(rect[2] - rect[0]),
        number(rect[3] - rect[1]),
        number(size),
        number(colour[0]),
        number(colour[1]),
        number(colour[2])
    ));
    let mut y = rect[3] - pad - size * 0.9;
    for line in crate::fieldadd::wrap(&text, size, width) {
        if y < rect[1] - size {
            break;
        }
        let line_width = crate::watermark::helvetica_width(&line, size);
        let x = match align as i64 {
            1 => rect[0] + pad + (width - line_width) / 2.0,
            2 => rect[2] - pad - line_width,
            _ => rect[0] + pad,
        };
        out.push_str(&format!("1 0 0 1 {} {} Tm ", number(x), number(y)));
        out.push_str(&crate::watermark::literal_bytes(&crate::watermark::win_ansi(&line)));
        out.push_str(" Tj\n");
        y -= size * 1.2;
    }
    out.push_str("ET\n");
    Some((out.into_bytes(), full))
}

/// The **drawing** of the **mark of a redaction**: the tree's own
/// redaction mark.
///
/// While the mark is there, the **outline** of each quad is drawn: the
/// fill comes when the redaction is applied.
fn the_redact(entries: &[(String, Json)]) -> Option<(Vec<u8>, [f64; 4])> {
    let numbers = numbers_of(entry(entries, "Rect")?);
    if numbers.len() != 4 {
        return None;
    }
    let rect = [
        numbers[0].min(numbers[2]),
        numbers[1].min(numbers[3]),
        numbers[0].max(numbers[2]),
        numbers[1].max(numbers[3]),
    ];
    // The **quads**: eight numbers to a quad, and a whole number of
    // quads.
    let quads = numbers_of(entry(entries, "QuadPoints")?);
    if quads.is_empty() || !quads.len().is_multiple_of(8) {
        return None;
    }
    // The **colour** of the mark: the tree's own red when the
    // annotation says none.
    let colour = match entry(entries, "C") {
        Some(value) => the_colour(value).ok().flatten()?,
        None => [0.89, 0.13, 0.13],
    };
    let number = crate::watermark::number;
    let mut out = String::new();
    out.push_str(&format!("{} {} {} RG\n1 w\n", number(colour[0]), number(colour[1]), number(colour[2])));
    for quad in quads.as_chunks::<8>().0 {
        out.push_str(&format!(
            "{} {} m {} {} l {} {} l {} {} l h S\n",
            number(quad[0]),
            number(quad[1]),
            number(quad[2]),
            number(quad[3]),
            number(quad[6]),
            number(quad[7]),
            number(quad[4]),
            number(quad[5])
        ));
    }
    Some((out.into_bytes(), rect))
}

/// The **drawing** of an annotation: the shape, the markup, the line,
/// the polygon, the ink, the caret, the text box or the mark.
fn the_drawing(entries: &[(String, Json)], subtype: &str) -> Option<(Vec<u8>, [f64; 4])> {
    // A **cloudy border** is the tree's own, and the tree draws it for
    // a polygon and for nothing else.
    if the_cloudy(entries) && subtype != "Polygon" {
        return None;
    }
    if MARKS.contains(&subtype) {
        the_markup(entries, subtype)
    } else if subtype == "Line" {
        the_line(entries)
    } else if subtype == "Polygon" || subtype == "PolyLine" {
        the_polygon(entries, subtype)
    } else if subtype == "Ink" {
        the_ink(entries)
    } else if subtype == "Caret" {
        the_caret(entries)
    } else if subtype == "FreeText" {
        the_freetext(entries)
    } else if subtype == "Redact" {
        the_redact(entries)
    } else {
        the_shape(entries, subtype)
    }
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
///   (`Square` or `Circle`), a **markup** (`Highlight`, `Underline`,
///   `StrikeOut` or `Squiggly`), a **line** (`Line`), a **shape of
///   corners** (`Polygon` or `PolyLine`), the **ink** (`Ink`) or the
///   **caret** (`Caret`), the **text box** (`FreeText`) or the **mark
///   of a redaction** (`Redact`) — and any other kind is refused;
/// - the **rectangle** of the shape is the one the annotation holds,
///   and the corners may come in any order;
/// - a **colour** is one number, three numbers or four numbers, and
///   anything else is refused;
/// - the **opacity** of the shape is between zero and one;
/// - a markup needs its **quads** and its **colour**, and one without
///   either is refused;
/// - a line needs its **two points** and its **colour**, and one
///   without either is refused, and the **ends** of it are the three
///   the tree knows;
/// - a polygon needs its **corners**, and a cloudy border is a polygon
///   only;
/// - the ink needs its **strokes** and its **colour**, and one without
///   either is refused;
/// - the caret needs its **rectangle**, and it has a colour whether or
///   not the annotation says one;
/// - the mark of a redaction needs its **quads**, and it has a colour
///   whether or not the annotation says one;
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
        let drawn = subtype == "Square"
            || subtype == "Circle"
            || subtype == "Line"
            || subtype == "Polygon"
            || subtype == "PolyLine"
            || subtype == "Ink"
            || subtype == "Caret"
            || subtype == "FreeText"
            || subtype == "Redact"
            || MARKS.contains(&subtype.as_str());
        if !drawn {
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
        let mut resources: Vec<(String, Json)> = Vec::new();
        if state {
            let mut entries = vec![
                ("Type".to_string(), json!("ExtGState")),
                ("CA".to_string(), json!(opacity)),
                ("ca".to_string(), json!(opacity)),
            ];
            if multiply {
                entries.insert(1, ("BM".to_string(), json!("Multiply")));
            }
            resources.push(("ExtGState".to_string(), json!({ "entries": crate::pairs_of(&entries) })));
        }
        if subtype == "FreeText" {
            // The **font** of the text: the tree's own Helvetica, under
            // the name the text of the box uses.
            resources.push((
                "Font".to_string(),
                json!({
                    "entries": [[
                        "Helv",
                        { "entries": [
                            ["Type", "Font"],
                            ["Subtype", "Type1"],
                            ["BaseFont", "Helvetica"],
                            ["Encoding", "WinAnsiEncoding"],
                        ] },
                    ]],
                }),
            ));
        }
        let resources = json!({ "entries": crate::pairs_of(&resources) });
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
