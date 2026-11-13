//! The **EPS** file of a drawing: the paths of the document, written as
//! PostScript.
//!
//! The tree's own writer: the **prolog** of the tree, its **path**
//! operators, its **ink** and its **fill** rules.
//!
//! # The rules
//!
//! - The file is an **EPS**: it begins with the version line, it says
//!   its **bounding box**, and it ends with the trailer.
//! - The **prolog** of the tree is written as it is: the short names
//!   of the operators — `m`, `l`, `c`, `h`, `f`, `f*`, `S`, `rg`, `k`,
//!   `g`, `w` — are bound once and used after.
//! - A **number** is written as the tree writes it: at most four
//!   decimals, and no trailing zeros; a number that is not finite is
//!   **zero**.
//! - The **fill** of a path is the tree's rule: `f` where the inside
//!   is the winding number, and `f*` where it is the even-odd rule.
//! - The **ink** is written in the space of the colour: `rg` for a
//!   colour of the screen, and `k` for one of the press.
//! - The **finish** of a line is the tree's own: the width, the end,
//!   the corner and the **dash** — the pattern in brackets, the offset
//!   after it — and a pattern that is not dashed is no dash at all.
//! - The **fill** of a path that holds a **gradient** is a **shading**
//!   of PostScript: the tree's own smooth shading, with the stops of
//!   the gradient as its function.
//!
//! # What it does not settle
//!
//! - The **pages** of the tree's writer are not here: this is one
//!   sheet, and it is the drawing.
//! - The **separations** of the tree — the spot colours and the
//!   overprinting — are not here: the ink of a path is the colour of
//!   the path.
//! - The **transparency** of the tree is not here: EPS has none, so a
//!   colour is written as it is and the opacity of a stop is left out.
//! - The file is in the **units of the document**, and the tree's file
//!   is in points.

use craftkit_derive::commands;

use craftkit_sdk::FormatValue;
use craftkit_sdk::document::{Cap, Colour, Fill, FillRule, Gradient, GradientGeometry, Join, Path, Segment};
use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Document, EngineError, Outcome, Text};

/// The name of the command, in the messages.
const COMMAND: &str = "vector.export.eps";

/// The **prolog** of the tree: the short names of the operators.
const PROLOG: &str = "/VCdict 64 dict def\n\
VCdict begin\n\
/bd {bind def} bind def\n\
/m {moveto} bd /l {lineto} bd /c {curveto} bd /h {closepath} bd\n\
/f {fill} bd /f* {eofill} bd /S {stroke} bd\n\
/W {clip newpath} bd /W* {eoclip newpath} bd\n\
/q {gsave} bd /Q {grestore} bd /cm {concat} bd\n\
/g {setgray} bd /rg {setrgbcolor} bd /k {setcmykcolor} bd\n\
/w {setlinewidth} bd /J {setlinecap} bd /j {setlinejoin} bd /M {setmiterlimit} bd /d {setdash} bd\n\
/op {setoverprint} bd\n\
end\n";

/// A **number** as PostScript writes it: the tree's own rule.
fn number(value: f64) -> String {
    if !value.is_finite() {
        return "0".to_string();
    }
    let value = if (value * 10_000.0).is_finite() { (value * 10_000.0).round() / 10_000.0 } else { value };
    if value == value.trunc() && value.abs() < 1e15 {
        return format!("{}", value as i64);
    }
    let text = format!("{value:.4}");
    let keep = text.trim_end_matches('0').trim_end_matches('.').len();
    text[..keep].to_string()
}

/// The **numbers** of a point, as PostScript writes them.
fn numbers(values: &[f64]) -> String {
    let mut out = String::new();
    for value in values {
        out.push_str(&number(*value));
        out.push(' ');
    }
    out
}

/// The **path** as PostScript writes it: the tree's own operators.
fn path_data(path: &Path) -> String {
    let mut out = String::new();
    for subpath in &path.subpaths {
        let Some(first) = subpath.segments.first() else { continue };
        let start = match first {
            Segment::Line { to } => *to,
            Segment::Cubic { c1, .. } => *c1,
        };
        out.push_str(&numbers(&[start.x, start.y]));
        out.push_str("m\n");
        for segment in &subpath.segments {
            match segment {
                Segment::Line { to } => {
                    out.push_str(&numbers(&[to.x, to.y]));
                    out.push_str("l\n");
                }
                Segment::Cubic { c1, c2, to } => {
                    out.push_str(&numbers(&[c1.x, c1.y, c2.x, c2.y, to.x, to.y]));
                    out.push_str("c\n");
                }
            }
        }
        if subpath.closed {
            out.push_str("h\n");
        }
    }
    out
}

/// The **fill** operator for a rule: the tree's own rule.
fn fill_op(rule: craftkit_sdk::document::FillRule) -> &'static str {
    match rule {
        craftkit_sdk::document::FillRule::NonZero => "f\n",
        craftkit_sdk::document::FillRule::EvenOdd => "f*\n",
    }
}

/// The **ink** of a colour: the tree's own rule — the screen or the
/// press.
fn ink(colour: Colour) -> String {
    let [r, g, b, _] = colour.0;
    format!("{}rg\n", numbers(&[f64::from(r), f64::from(g), f64::from(b)]))
}

/// The **end** of a line, as PostScript writes it: the tree's own
/// numbers.
fn cap(cap: Cap) -> &'static str {
    match cap {
        Cap::Butt => "0 J\n",
        Cap::Round => "1 J\n",
        Cap::Square => "2 J\n",
    }
}

/// The **corner** of a line, as PostScript writes it: the tree's own
/// numbers.
fn join(join: Join) -> &'static str {
    match join {
        Join::Miter => "0 j\n",
        Join::Round => "1 j\n",
        Join::Bevel => "2 j\n",
    }
}

/// The **components** of a colour, as PostScript writes them: three
/// numbers of the screen.
fn components(colour: Colour) -> String {
    let [r, g, b, _] = colour.0;
    numbers(&[f64::from(r), f64::from(g), f64::from(b)]).trim_end().to_string()
}

/// The **function** of a shading: the colours of the gradient, as
/// PostScript writes them.
///
/// The tree's own rule: one stretch of the gradient is an exponential
/// function between two colours, and more than one stretch is a
/// **stitching** function of them — each piece over the stretch of the
/// gradient that is its own.
fn function_of(gradient: &Gradient) -> String {
    let segments: Vec<(&craftkit_sdk::document::GradientStop, &craftkit_sdk::document::GradientStop)> =
        gradient
            .stops
            .windows(2)
            .filter(|pair| pair[1].offset > pair[0].offset)
            .map(|pair| (&pair[0], &pair[1]))
            .collect();
    match segments.as_slice() {
        [] => String::new(),
        [one] => format!(
            "<< /FunctionType 2 /Domain [0 1] /C0 [{}] /C1 [{}] /N 1 >>",
            components(one.0.colour),
            components(one.1.colour)
        ),
        many => {
            let mut functions = String::new();
            let mut bounds: Vec<String> = Vec::new();
            let mut encode: Vec<String> = Vec::new();
            for (index, (from, to)) in many.iter().enumerate() {
                functions.push_str(&format!(
                    "<< /FunctionType 2 /Domain [{} {}] /C0 [{}] /C1 [{}] /N 1 >>\n",
                    number(from.offset),
                    number(to.offset),
                    components(from.colour),
                    components(to.colour)
                ));
                if index > 0 {
                    bounds.push(number(from.offset));
                }
                encode.push("0 1".to_string());
            }
            format!(
                "<< /FunctionType 3 /Domain [0 1] /Functions [\n{functions}] /Bounds [{}] /Encode [{}] >>",
                bounds.join(" "),
                encode.join(" ")
            )
        }
    }
}

/// The **shading** of a gradient, as PostScript writes it: the tree's
/// own smooth shading.
///
/// A **linear** gradient is a shading of kind two and a **radial** one
/// is a shading of kind three, and both are extended both ways.
fn shading(gradient: &Gradient) -> String {
    let coords = match &gradient.geometry {
        GradientGeometry::Linear { from, to } => {
            format!("{} {} {} {}", number(from.x), number(from.y), number(to.x), number(to.y))
        }
        GradientGeometry::Radial { centre, radius } => format!(
            "{} {} 0 {} {} {}",
            number(centre.x),
            number(centre.y),
            number(centre.x),
            number(centre.y),
            number(*radius)
        ),
    };
    let kind = match gradient.kind {
        craftkit_sdk::document::GradientKind::Linear => 2,
        craftkit_sdk::document::GradientKind::Radial => 3,
    };
    format!(
        "<< /ShadingType {kind} /ColorSpace /DeviceRGB /Coords [{coords}] /Extend [true true]\n/Function {} >> shfill\n",
        function_of(gradient)
    )
}

/// The **fill** of a path that holds a gradient.
///
/// The tree's own rule: a gradient with no stretch in it — a stop and
/// nothing else — is the colour of its first stop, painted solid.
fn gradient_fill(gradient: &Gradient, rule: FillRule) -> String {
    let stretched = gradient.stops.windows(2).any(|pair| pair[1].offset > pair[0].offset);
    if stretched {
        return shading(gradient);
    }
    let Some(first) = gradient.stops.first() else { return String::new() };
    format!("{}\n{}", ink(first.colour), fill_op(rule))
}

/// The **dash** of a line, as PostScript writes it: the pattern in
/// brackets, the offset after it.
///
/// The tree's own rule: a pattern that is not dashed — a pattern with
/// no length in it — is **no dash at all**, and the line is whole.
fn dash(dash: Option<&crate::dashes::Dash>) -> String {
    let Some(dash) = dash else { return String::new() };
    if dash.pattern.is_empty() || !dash.pattern.iter().any(|length| *length > 0.0) {
        return String::new();
    }
    let pattern: Vec<String> = dash.pattern.iter().map(|length| number(*length)).collect();
    format!("[{}] {} d\n", pattern.join(" "), number(dash.offset))
}

/// The **file** of the drawing: the EPS of the document's paths.
pub fn the_eps(document: &Document) -> Result<(String, usize), EngineError> {
    let mut out = String::new();
    let mut count = 0usize;
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut body = String::new();
    for node in document.walk() {
        if node.kind != "vector.path" {
            continue;
        }
        let Ok(vector) = crate::VectorNode::from_json(&node.data) else { continue };
        // The **appearance** wins over the payload's own colours, the
        // way the tree draws it.
        let (fill, stroke) = match document.appearance_of(node) {
            Some(appearance) => (appearance.fill, appearance.stroke),
            None => (vector.fill.clone(), vector.stroke),
        };
        // The path is written **where it is**: the transform is baked
        // into it.
        let placed = vector.placed();
        let box_ = craftkit_vector_geom::bounds(&placed);
        if box_.x.is_finite() && box_.y.is_finite() {
            min_x = min_x.min(box_.x);
            min_y = min_y.min(box_.y);
            max_x = max_x.max(box_.x + box_.w);
            max_y = max_y.max(box_.y + box_.h);
        }
        // The **fill** of the path: a colour, or a **gradient**.
        match &fill {
            Fill::Solid(colour) => {
                body.push_str("newpath\n");
                body.push_str(&path_data(&placed));
                body.push_str(&ink(*colour));
                body.push_str(fill_op(placed.rule));
            }
            Fill::Gradient(gradient) => {
                body.push_str("newpath\n");
                body.push_str(&path_data(&placed));
                body.push_str(&gradient_fill(gradient, placed.rule));
            }
        }
        // The **stroke** of the path: the path is written again, for a
        // stroke is a path of its own.
        if let Some(stroke) = &stroke {
            body.push_str("newpath\n");
            body.push_str(&path_data(&placed));
            body.push_str(&numbers(&[stroke.width]));
            body.push_str("w\n");
            body.push_str(cap(stroke.cap));
            body.push_str(join(stroke.join));
            body.push_str(&dash(vector.dash.as_ref()));
            body.push_str(&ink(stroke.colour));
            body.push_str("S\n");
        }
        count += 1;
    }
    if count == 0 {
        return Err(EngineError::Failed {
            id: COMMAND.to_string(),
            message: "the document holds no path to write".to_string(),
        });
    }
    if !min_x.is_finite() || !max_x.is_finite() {
        return Err(EngineError::Failed {
            id: COMMAND.to_string(),
            message: "the paths of the document have no box".to_string(),
        });
    }
    let width = max_x - min_x;
    let height = max_y - min_y;
    out.push_str("%!PS-Adobe-3.0 EPSF-3.0\n");
    out.push_str(&format!("%%BoundingBox: 0 0 {} {}\n", width.ceil() as i64, height.ceil() as i64));
    out.push_str("%%EndComments\n");
    out.push_str("%%BeginProlog\n");
    out.push_str(PROLOG);
    out.push_str("%%EndProlog\n");
    out.push_str("%%Page: 1 1\n");
    out.push_str("q\n");
    // The world has Y down and the paper has Y up.
    out.push_str("1 0 0 -1 0 0 cm\n");
    out.push_str(&body);
    out.push_str("Q\n");
    out.push_str("showpage\n");
    out.push_str("%%Trailer\n%%EOF\n");
    Ok((out, count))
}

commands! {
    /// The **EPS file** of the drawing.
    vector.export.eps {
        area: "vector",
        layer: "vector",
        icon: "file",
        mutates,
        undoable,
        params: {
            /// The file to write.
            path: Text,
        },
        run: {
            let path = self.path.0.trim().to_string();
            if path.is_empty() {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "a file has a path".to_string(),
                });
            }
            let (file, count) = the_eps(session.document())?;
            std::fs::write(&path, file.as_bytes()).map_err(|error| EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("the file cannot be written: {error}"),
            })?;
            Ok(Outcome::changed(format!("`{path}` was written with {count} path(s)")).with_data(json!({
                "path": path,
                "count": count,
                "bytes": file.len(),
            })))

        }
    }
}
