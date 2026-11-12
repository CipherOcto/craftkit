//! The **writing** of a drawing as an SVG file.
//!
//! A drawing is a set of paths, and an **SVG file** is a way of
//! writing a drawing down: a text file that says where each path is
//! and what it is painted with. The paths are written as
//! `<path>` elements, and the colours as the attributes of the
//! elements.
//!
//! The rules:
//!
//! - the **box** of the drawing is the box of all its paths, and the
//!   file says it in its `width` and `height`;
//! - the world has **Y up** and an SVG file has Y down, so the paths
//!   are written in a group that **turns them over**, and the box of
//!   the file is the box of the world turned over with them;
//! - the **transform** of a path is **baked** into it: the path is
//!   written where it is, and not with a matrix on it;
//! - a **gradient** is written as a gradient in the file's `defs`, and
//!   the path that is filled with it names it;
//! - a drawing with **no path** is refused: a file with nothing in it
//!   is not a drawing.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{
    Cap, Colour, Document, EngineError, Fill, FillRule, FormatValue, GradientGeometry, Join, Outcome, Path,
    Rect, Segment, Text,
};

use craftkit_vector_geom as geom;

/// The name of the command, in the messages.
const COMMAND: &str = "vector.export.svg";

/// A **number**, as a file writes it: three decimals at most, and
/// nothing after the point when there is nothing after it.
fn number(value: f64) -> String {
    if !value.is_finite() {
        return "0".to_string();
    }
    let rounded = (value * 1000.0).round() / 1000.0;
    let text = format!("{rounded:.3}");
    let text = text.trim_end_matches('0').trim_end_matches('.').to_string();
    if text.is_empty() || text == "-" {
        return "0".to_string();
    }
    text
}

/// A **colour**, as a file writes it: `#rrggbb`.
fn hex(colour: Colour) -> String {
    let channel = |value: f32| {
        let value = value.clamp(0.0, 1.0) * 255.0;
        format!("{:02x}", value.round() as u32)
    };
    format!("#{}{}{}", channel(colour.0[0]), channel(colour.0[1]), channel(colour.0[2]))
}

/// The **opacity** of a colour, as a file writes it.
fn alpha(colour: Colour) -> f64 {
    f64::from(colour.0[3].clamp(0.0, 1.0))
}

/// The **data** of a path: where it goes and what it does.
fn data(path: &Path) -> String {
    let mut out = String::new();
    for subpath in &path.subpaths {
        out.push_str(&format!("M {} {}", number(subpath.start.x), number(subpath.start.y)));
        for segment in &subpath.segments {
            match segment {
                Segment::Line { to } => {
                    out.push_str(&format!(" L {} {}", number(to.x), number(to.y)));
                }
                Segment::Cubic { c1, c2, to } => {
                    out.push_str(&format!(
                        " C {} {} {} {} {} {}",
                        number(c1.x),
                        number(c1.y),
                        number(c2.x),
                        number(c2.y),
                        number(to.x),
                        number(to.y)
                    ));
                }
            }
        }
        if subpath.closed {
            out.push_str(" Z");
        }
    }
    out
}

/// The **rule** that says what is inside a path.
fn rule(rule: FillRule) -> &'static str {
    match rule {
        FillRule::NonZero => "nonzero",
        FillRule::EvenOdd => "evenodd",
    }
}

/// The **corner** of a stroke, as a file writes it.
fn join(join: Join) -> &'static str {
    match join {
        Join::Miter => "miter",
        Join::Round => "round",
        Join::Bevel => "bevel",
    }
}

/// The **end** of a stroke, as a file writes it.
fn cap(cap: Cap) -> &'static str {
    match cap {
        Cap::Butt => "butt",
        Cap::Round => "round",
        Cap::Square => "square",
    }
}

/// One **path** of the drawing, as the file writes it.
struct Written {
    /// The element itself.
    element: String,
    /// The gradients the file needs for it.
    gradients: Vec<String>,
}

/// The **gradient** a fill is, as the file writes it.
fn gradient(fill: &Fill, transform: &geom::Matrix, opacity: f64, index: usize) -> Option<String> {
    let Fill::Gradient(gradient) = fill else { return None };
    let id = format!("gradient-{index}");
    let mut stops = String::new();
    for stop in &gradient.stops {
        stops.push_str(&format!(
            "<stop offset=\"{}\" stop-color=\"{}\" stop-opacity=\"{}\"/>",
            number(stop.offset),
            hex(stop.colour),
            number(alpha(stop.colour) * opacity)
        ));
    }
    let body = match gradient.geometry {
        GradientGeometry::Linear { from, to } => {
            let from = transform.apply(from);
            let to = transform.apply(to);
            format!(
                "<linearGradient id=\"{id}\" gradientUnits=\"userSpaceOnUse\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\">{stops}</linearGradient>",
                number(from.x),
                number(from.y),
                number(to.x),
                number(to.y)
            )
        }
        GradientGeometry::Radial { centre, radius } => {
            let centre = transform.apply(centre);
            format!(
                "<radialGradient id=\"{id}\" gradientUnits=\"userSpaceOnUse\" cx=\"{}\" cy=\"{}\" r=\"{}\">{stops}</radialGradient>",
                number(centre.x),
                number(centre.y),
                number(radius)
            )
        }
    };
    Some(body)
}

/// The **paths** of the document, as the file writes them.
fn written(document: &Document) -> Vec<(Written, Rect)> {
    let mut out: Vec<(Written, Rect)> = Vec::new();
    let mut gradients = 0usize;
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
        let mut attributes = format!("d=\"{}\"", data(&placed));
        attributes.push_str(&format!(" fill-rule=\"{}\"", rule(placed.rule)));
        // The **fill**.
        let mut defs: Vec<String> = Vec::new();
        match &fill {
            Fill::Solid(colour) => {
                attributes.push_str(&format!(" fill=\"{}\"", hex(*colour)));
                let opacity = alpha(*colour) * vector.opacity;
                if opacity < 1.0 {
                    attributes.push_str(&format!(" fill-opacity=\"{}\"", number(opacity)));
                }
            }
            Fill::Gradient(_) => {
                gradients += 1;
                if let Some(body) = gradient(&fill, &vector.transform, vector.opacity, gradients) {
                    let id = body
                        .split("id=\"")
                        .nth(1)
                        .and_then(|rest| rest.split('"').next())
                        .unwrap_or_default()
                        .to_string();
                    attributes.push_str(&format!(" fill=\"url(#{id})\""));
                    defs.push(body);
                }
            }
        }
        // The **stroke**.
        match &stroke {
            Some(stroke) => {
                attributes.push_str(&format!(" stroke=\"{}\"", hex(stroke.colour)));
                attributes.push_str(&format!(" stroke-width=\"{}\"", number(stroke.width)));
                attributes.push_str(&format!(" stroke-linejoin=\"{}\"", join(stroke.join)));
                attributes.push_str(&format!(" stroke-linecap=\"{}\"", cap(stroke.cap)));
                let opacity = alpha(stroke.colour) * vector.opacity;
                if opacity < 1.0 {
                    attributes.push_str(&format!(" stroke-opacity=\"{}\"", number(opacity)));
                }
            }
            None => attributes.push_str(" stroke=\"none\""),
        }
        // The **dash**.
        if let Some(dash) = &vector.dash {
            let pattern: Vec<String> = dash.run().iter().map(|length| number(*length)).collect();
            if !pattern.is_empty() {
                attributes.push_str(&format!(" stroke-dasharray=\"{}\"", pattern.join(" ")));
                attributes.push_str(&format!(" stroke-dashoffset=\"{}\"", number(dash.offset)));
            }
        }
        let box_ = geom::bounds(&placed);
        out.push((Written { element: format!("<path {attributes}/>"), gradients: defs }, box_));
    }
    out
}

/// The **file** of the drawing: the SVG of the document's paths.
pub fn the_svg(document: &Document) -> Result<(String, usize), EngineError> {
    let paths = written(document);
    if paths.is_empty() {
        return Err(EngineError::Failed {
            id: COMMAND.to_string(),
            message: "the document holds no path to write".to_string(),
        });
    }
    // The **box** of the drawing: the box of all its paths.
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for (_, box_) in &paths {
        if !box_.x.is_finite() || !box_.y.is_finite() || !box_.w.is_finite() || !box_.h.is_finite() {
            continue;
        }
        min_x = min_x.min(box_.x);
        min_y = min_y.min(box_.y);
        max_x = max_x.max(box_.x + box_.w);
        max_y = max_y.max(box_.y + box_.h);
    }
    if !min_x.is_finite() || !max_x.is_finite() {
        return Err(EngineError::Failed {
            id: COMMAND.to_string(),
            message: "the paths of the document have no box".to_string(),
        });
    }
    let width = max_x - min_x;
    let height = max_y - min_y;
    // The **defs**: the gradients the file needs.
    let mut defs: Vec<String> = Vec::new();
    for (written, _) in &paths {
        for gradient in &written.gradients {
            defs.push(gradient.clone());
        }
    }
    let mut file = String::new();
    file.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    file.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"{} {} {} {}\">\n",
        number(width),
        number(height),
        number(min_x),
        number(-max_y),
        number(width),
        number(height)
    ));
    if !defs.is_empty() {
        file.push_str("<defs>\n");
        for gradient in &defs {
            file.push_str(gradient);
            file.push('\n');
        }
        file.push_str("</defs>\n");
    }
    // The world has Y up and the file has Y down, so the paths are
    // turned over.
    file.push_str("<g transform=\"scale(1,-1)\">\n");
    for (written, _) in &paths {
        file.push_str(&written.element);
        file.push('\n');
    }
    file.push_str("</g>\n</svg>\n");
    Ok((file, paths.len()))
}

commands! {
    /// **Write** a drawing as an **SVG file**.
    ///
    /// The file is written where the caller says, and the answer says how
    /// many paths it holds and what box they are in.
    vector.export.svg {
        area: "vector",
        layer: "vector",
        icon: "file",
        params: {
            /// Where to write the file.

            path: Text,
        },
        run: {

            let path = self.path.0.clone();
            if path.trim().is_empty() {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "a file has a path".to_string(),
                });
            }
            let (file, count) = the_svg(session.document())?;
            std::fs::write(&path, file.as_bytes()).map_err(|error| EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("the file cannot be written to `{path}`: {error}"),
            })?;
            Ok(Outcome::new(format!("wrote {count} path(s)")).with_data(json!({
                "path": path,
                "paths": count,
                "bytes": file.len(),
            })))

        }
    }
}
