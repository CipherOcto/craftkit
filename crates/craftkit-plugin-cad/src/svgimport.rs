//! The **SVG** read back: a picture file made into a drawing.
//!
//! This is the tree's own way of bringing a picture in: the shapes of
//! the file become entities of a drawing, the page of the file becomes
//! the origin of the drawing, and the Y of the page is turned over —
//! a page has its Y down and this tree's world has it up.
//!
//! # The rules
//!
//! - The file is read through a **bound**: at most **sixteen
//!   mebibytes**.
//! - The **units** are the tree's own: 96 pixels to the inch, 72
//!   points to the inch, 6 picas to the inch, 25.4 millimetres to the
//!   inch, and 10 millimetres to the centimetre. A number with no
//!   unit is a number of **user units**, and the page's own size is
//!   in **pixels** when it says nothing.
//! - The **page** is the root's `viewBox`, and the **size** of the
//!   page is the root's `width`: the millimetres a user unit holds
//!   are the width in millimetres over the width of the view box.
//! - The **origin** is the bottom left of the page, and the Y of the
//!   page is turned over.
//! - The **shapes** that are read are the tree's own: `line`,
//!   `circle`, `ellipse`, `rect`, `polyline`, `polygon` and `path`,
//!   with the path commands `M L H V C S Q T Z`, absolute and
//!   relative.
//! - A **curve** is a run of straight segments: the drawing model of
//!   this tree has no curve of its own to put one in, and a polyline
//!   is what a curve becomes.
//! - The **entities** go into the block the tree names `*Model_Space`,
//!   and they are on a layer of their own.
//! - An element this tree does not read is **left out**, and the
//!   answer says what it left out.
//!
//! # What it does not do
//!
//! - An **arc** in a path is not read: a path that holds one is left
//!   out, and the answer says so.
//! - A **transform** is not applied: an element that carries one is
//!   left out, and the answer says so.
//! - The **fills**, the **gradients** and the **styles** are not read:
//!   the drawing model of this tree draws outlines.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Outcome, Relation, Text};

/// The most an SVG file may be, in bytes.
const MAX_BYTES: u64 = 16 * 1024 * 1024;

/// The most **elements** a file may hold.
const MAX_ELEMENTS: usize = 50_000;

/// The most **points** a file may hold, over every shape of it.
const MAX_POINTS: usize = 100_000;

/// The most elements may be **deep**.
const MAX_DEPTH: usize = 32;

/// The **steps** a curve is flattened into.
const CURVE_STEPS: usize = 16;

/// The **name** of the block a drawing's entities live in: the tree's
/// own name.
const MODEL_SPACE: &str = "*Model_Space";

/// The name of the command, in the messages.
const COMMAND: &str = "cad.import.svg";

/// The **millimetres** a unit holds.
fn unit_in_mm(unit: &str) -> Option<f64> {
    match unit {
        "mm" => Some(1.0),
        "cm" => Some(10.0),
        "in" => Some(25.4),
        "pt" => Some(25.4 / 72.0),
        "pc" => Some(25.4 / 6.0),
        "px" => Some(25.4 / 96.0),
        _ => None,
    }
}

/// A **number** and the **unit** it is in.
fn number_with_unit(text: &str) -> Option<(f64, Option<String>)> {
    let text = text.trim();
    let cut = text
        .find(|character: char| {
            !(character.is_ascii_digit() || matches!(character, '.' | '-' | '+' | 'e' | 'E'))
        })
        .unwrap_or(text.len());
    let (number, unit) = text.split_at(cut);
    let value: f64 = number.trim().parse().ok()?;
    if !value.is_finite() {
        return None;
    }
    let unit = unit.trim();
    let unit = if unit.is_empty() { None } else { Some(unit.to_string()) };
    Some((value, unit))
}

/// A **length** in millimetres, when the unit is one this tree knows.
///
/// A number with no unit is a number of **pixels**, because that is
/// what the size of a page is when it says nothing.
fn length_in_mm(text: &str) -> Option<f64> {
    let (value, unit) = number_with_unit(text)?;
    match unit {
        None => Some(value * 25.4 / 96.0),
        Some(unit) => unit_in_mm(&unit).map(|millimetres| value * millimetres),
    }
}

/// A **coordinate** in user units: a number with no unit is a number
/// of user units, and a number with a unit is a length in millimetres
/// over the scale of the page.
fn coordinate_in_user(text: &str, scale: f64) -> Option<f64> {
    let (value, unit) = number_with_unit(text)?;
    match unit {
        None => Some(value),
        Some(unit) => {
            let millimetres = unit_in_mm(&unit)?;
            if !scale.is_finite() || scale <= 0.0 {
                return None;
            }
            Some(millimetres / scale)
        }
    }
}

/// One **operation** of a path.
///
/// Every operation says whether it is **relative**: the small letter
/// of a command is the same command from the point the path is at.
#[derive(Clone, Copy)]
enum Op {
    /// A move to a point.
    Move(bool, f64, f64),
    /// A line to a point.
    Line(bool, f64, f64),
    /// A line to a point on the horizontal.
    Horizontal(bool, f64),
    /// A line to a point on the vertical.
    Vertical(bool, f64),
    /// A curve with two controls.
    Cubic(bool, f64, f64, f64, f64, f64, f64),
    /// A curve with one control, the other one reflected.
    SmoothCubic(bool, f64, f64, f64, f64),
    /// A curve with one control.
    Quad(bool, f64, f64, f64, f64),
    /// A curve with one control, reflected.
    SmoothQuad(bool, f64, f64),
    /// The end of the run.
    Close,
}

/// The **number** at that place, and where the text goes on.
fn number_at(chars: &[char], at: usize) -> (Option<f64>, usize) {
    let mut cursor = at;
    if matches!(chars.get(cursor), Some('+') | Some('-')) {
        cursor += 1;
    }
    let start = cursor;
    while matches!(chars.get(cursor), Some(char) if char.is_ascii_digit() || *char == '.') {
        cursor += 1;
    }
    if cursor == start {
        return (None, at + 1);
    }
    // An exponent, when there is one.
    if matches!(chars.get(cursor), Some('e') | Some('E')) {
        let mark = cursor;
        cursor += 1;
        if matches!(chars.get(cursor), Some('+') | Some('-')) {
            cursor += 1;
        }
        let digits = cursor;
        while matches!(chars.get(cursor), Some(char) if char.is_ascii_digit()) {
            cursor += 1;
        }
        if cursor == digits {
            cursor = mark;
        }
    }
    let text: String = chars[at..cursor].iter().collect();
    match text.parse::<f64>() {
        Ok(value) if value.is_finite() => (Some(value), cursor),
        _ => (None, at + 1),
    }
}

/// The **numbers** of a text, in order.
fn numbers_of(text: &str) -> Vec<f64> {
    let chars: Vec<char> = text.chars().collect();
    let mut numbers: Vec<f64> = Vec::new();
    let mut at = 0usize;
    while at < chars.len() {
        let character = chars[at];
        if character.is_ascii_digit() || matches!(character, '+' | '-' | '.') {
            let (value, next) = number_at(&chars, at);
            at = next;
            if let Some(value) = value {
                numbers.push(value);
            }
            continue;
        }
        at += 1;
    }
    numbers
}

/// The **operation** a command and its numbers make, when they are
/// enough of them.
fn take_op(command: char, pending: &mut Vec<f64>) -> Option<Op> {
    let relative = command.is_ascii_lowercase();
    let needed = match command {
        'M' | 'm' | 'L' | 'l' | 'T' | 't' => 2,
        'H' | 'h' | 'V' | 'v' => 1,
        'C' | 'c' => 6,
        'S' | 's' => 4,
        'Q' | 'q' => 4,
        _ => return None,
    };
    if pending.len() < needed {
        return None;
    }
    let args = pending.drain(..needed).collect::<Vec<f64>>();
    let op = match command {
        'M' | 'm' => Op::Move(relative, args[0], args[1]),
        'L' | 'l' => Op::Line(relative, args[0], args[1]),
        'T' | 't' => Op::SmoothQuad(relative, args[0], args[1]),
        'H' | 'h' => Op::Horizontal(relative, args[0]),
        'V' | 'v' => Op::Vertical(relative, args[0]),
        'C' | 'c' => Op::Cubic(relative, args[0], args[1], args[2], args[3], args[4], args[5]),
        'S' | 's' => Op::SmoothCubic(relative, args[0], args[1], args[2], args[3]),
        'Q' | 'q' => Op::Quad(relative, args[0], args[1], args[2], args[3]),
        _ => return None,
    };
    Some(op)
}

/// The **operations** of a path, and whether it holds an arc.
fn ops_of(text: &str) -> (Vec<Op>, bool) {
    let chars: Vec<char> = text.chars().collect();
    let mut ops: Vec<Op> = Vec::new();
    let mut pending: Vec<f64> = Vec::new();
    let mut command = ' ';
    let mut arc = false;
    let mut at = 0usize;
    while at < chars.len() {
        let character = chars[at];
        if character.is_ascii_alphabetic() {
            command = character;
            at += 1;
            pending.clear();
            if matches!(command, 'z' | 'Z') {
                ops.push(Op::Close);
            }
            continue;
        }
        if character.is_ascii_digit() || matches!(character, '+' | '-' | '.') {
            let (value, next) = number_at(&chars, at);
            at = next;
            let Some(value) = value else { continue };
            if matches!(command, 'a' | 'A') {
                // An **arc** is not read: its seven numbers are taken
                // and the path is refused.
                arc = true;
                pending.push(value);
                if pending.len() == 7 {
                    pending.clear();
                }
                continue;
            }
            pending.push(value);
            while let Some(op) = take_op(command, &mut pending) {
                let move_op = matches!(op, Op::Move(..));
                ops.push(op);
                if move_op {
                    // The numbers of a move are the numbers of a line
                    // after it: the rule of every path.
                    command = if command == 'M' { 'L' } else { 'l' };
                }
            }
            continue;
        }
        at += 1;
    }
    (ops, arc)
}

/// One **run** of a path: its points, and whether it is closed.
struct Run {
    /// The points of the run.
    points: Vec<(f64, f64)>,
    /// Whether the run comes back to where it began.
    closed: bool,
}

/// The **point** of a curve at that place.
fn cubic_at(from: (f64, f64), c1: (f64, f64), c2: (f64, f64), to: (f64, f64), t: f64) -> (f64, f64) {
    let u = 1.0 - t;
    let a = u * u * u;
    let b = 3.0 * u * u * t;
    let c = 3.0 * u * t * t;
    let d = t * t * t;
    (a * from.0 + b * c1.0 + c * c2.0 + d * to.0, a * from.1 + b * c1.1 + c * c2.1 + d * to.1)
}

/// The **runs** of a path, when it holds no arc.
fn runs_of(text: &str) -> Result<Vec<Run>, String> {
    let (ops, arc) = ops_of(text);
    if arc {
        return Err("a path with an arc".to_string());
    }
    let mut runs: Vec<Run> = Vec::new();
    let mut current = (0.0, 0.0);
    let mut start = (0.0, 0.0);
    let mut control: Option<(f64, f64)> = None;
    let mut quad_control: Option<(f64, f64)> = None;
    for op in ops {
        match op {
            Op::Move(relative, x, y) => {
                let point = if relative { (current.0 + x, current.1 + y) } else { (x, y) };
                runs.push(Run { points: vec![point], closed: false });
                current = point;
                start = point;
                control = None;
                quad_control = None;
            }
            Op::Line(relative, x, y) => {
                let point = if relative { (current.0 + x, current.1 + y) } else { (x, y) };
                if let Some(run) = runs.last_mut() {
                    run.points.push(point);
                }
                current = point;
                control = None;
                quad_control = None;
            }
            Op::Horizontal(relative, x) => {
                let point = if relative { (current.0 + x, current.1) } else { (x, current.1) };
                if let Some(run) = runs.last_mut() {
                    run.points.push(point);
                }
                current = point;
                control = None;
                quad_control = None;
            }
            Op::Vertical(relative, y) => {
                let point = if relative { (current.0, current.1 + y) } else { (current.0, y) };
                if let Some(run) = runs.last_mut() {
                    run.points.push(point);
                }
                current = point;
                control = None;
                quad_control = None;
            }
            Op::Cubic(relative, x1, y1, x2, y2, x, y) => {
                let (c1, c2, to) = if relative {
                    (
                        (current.0 + x1, current.1 + y1),
                        (current.0 + x2, current.1 + y2),
                        (current.0 + x, current.1 + y),
                    )
                } else {
                    ((x1, y1), (x2, y2), (x, y))
                };
                if let Some(run) = runs.last_mut() {
                    for step in 1..=CURVE_STEPS {
                        run.points.push(cubic_at(current, c1, c2, to, step as f64 / CURVE_STEPS as f64));
                    }
                }
                current = to;
                control = Some(c2);
                quad_control = None;
            }
            Op::SmoothCubic(relative, x2, y2, x, y) => {
                let (c2, to) = if relative {
                    ((current.0 + x2, current.1 + y2), (current.0 + x, current.1 + y))
                } else {
                    ((x2, y2), (x, y))
                };
                let c1 = match control {
                    Some(control) => (2.0 * current.0 - control.0, 2.0 * current.1 - control.1),
                    None => current,
                };
                if let Some(run) = runs.last_mut() {
                    for step in 1..=CURVE_STEPS {
                        run.points.push(cubic_at(current, c1, c2, to, step as f64 / CURVE_STEPS as f64));
                    }
                }
                current = to;
                control = Some(c2);
                quad_control = None;
            }
            Op::Quad(relative, x1, y1, x, y) => {
                let (q, to) = if relative {
                    ((current.0 + x1, current.1 + y1), (current.0 + x, current.1 + y))
                } else {
                    ((x1, y1), (x, y))
                };
                let c1 =
                    (current.0 + 2.0 / 3.0 * (q.0 - current.0), current.1 + 2.0 / 3.0 * (q.1 - current.1));
                let c2 = (to.0 + 2.0 / 3.0 * (q.0 - to.0), to.1 + 2.0 / 3.0 * (q.1 - to.1));
                if let Some(run) = runs.last_mut() {
                    for step in 1..=CURVE_STEPS {
                        run.points.push(cubic_at(current, c1, c2, to, step as f64 / CURVE_STEPS as f64));
                    }
                }
                current = to;
                control = None;
                quad_control = Some(q);
            }
            Op::SmoothQuad(relative, x, y) => {
                let to = if relative { (current.0 + x, current.1 + y) } else { (x, y) };
                let q = match quad_control {
                    Some(control) => (2.0 * current.0 - control.0, 2.0 * current.1 - control.1),
                    None => current,
                };
                let c1 =
                    (current.0 + 2.0 / 3.0 * (q.0 - current.0), current.1 + 2.0 / 3.0 * (q.1 - current.1));
                let c2 = (to.0 + 2.0 / 3.0 * (q.0 - to.0), to.1 + 2.0 / 3.0 * (q.1 - to.1));
                if let Some(run) = runs.last_mut() {
                    for step in 1..=CURVE_STEPS {
                        run.points.push(cubic_at(current, c1, c2, to, step as f64 / CURVE_STEPS as f64));
                    }
                }
                current = to;
                control = None;
                quad_control = Some(q);
            }
            Op::Close => {
                if let Some(run) = runs.last_mut() {
                    run.closed = true;
                }
                current = start;
                control = None;
                quad_control = None;
            }
        }
    }
    // A run with one point is not a shape.
    runs.retain(|run| run.points.len() >= 2);
    Ok(runs)
}

/// One **shape** of a picture, in the file's own units.
enum Shape {
    /// A line from one point to another.
    Line { from: (f64, f64), to: (f64, f64) },
    /// A circle.
    Circle { center: (f64, f64), radius: f64 },
    /// An ellipse.
    Ellipse { center: (f64, f64), rx: f64, ry: f64 },
    /// A run of points.
    Polyline { points: Vec<(f64, f64)>, closed: bool },
}

/// The **shapes** of an element, when it holds any.
fn shapes_of(element: &craftkit_sdk::xml::Element, scale: f64) -> Vec<Shape> {
    let coordinate = |name: &str| element.attribute(name).and_then(|text| coordinate_in_user(text, scale));
    match element.name.as_str() {
        "line" => {
            let (Some(x1), Some(y1), Some(x2), Some(y2)) =
                (coordinate("x1"), coordinate("y1"), coordinate("x2"), coordinate("y2"))
            else {
                return Vec::new();
            };
            vec![Shape::Line { from: (x1, y1), to: (x2, y2) }]
        }
        "circle" => {
            let (Some(cx), Some(cy), Some(radius)) = (coordinate("cx"), coordinate("cy"), coordinate("r"))
            else {
                return Vec::new();
            };
            if radius <= 0.0 {
                return Vec::new();
            }
            vec![Shape::Circle { center: (cx, cy), radius }]
        }
        "ellipse" => {
            let (Some(cx), Some(cy), Some(rx), Some(ry)) =
                (coordinate("cx"), coordinate("cy"), coordinate("rx"), coordinate("ry"))
            else {
                return Vec::new();
            };
            if rx <= 0.0 || ry <= 0.0 {
                return Vec::new();
            }
            vec![Shape::Ellipse { center: (cx, cy), rx, ry }]
        }
        "rect" => {
            let (Some(width), Some(height)) = (coordinate("width"), coordinate("height")) else {
                return Vec::new();
            };
            if width <= 0.0 || height <= 0.0 {
                return Vec::new();
            }
            let x = coordinate("x").unwrap_or(0.0);
            let y = coordinate("y").unwrap_or(0.0);
            vec![Shape::Polyline {
                points: vec![(x, y), (x + width, y), (x + width, y + height), (x, y + height)],
                closed: true,
            }]
        }
        "polyline" | "polygon" => {
            let Some(text) = element.attribute("points") else { return Vec::new() };
            let numbers = numbers_of(text);
            let mut points = Vec::new();
            for pair in numbers.as_chunks::<2>().0 {
                if pair[0].is_finite() && pair[1].is_finite() {
                    points.push((pair[0], pair[1]));
                }
            }
            if points.len() < 2 {
                return Vec::new();
            }
            vec![Shape::Polyline { points, closed: element.name == "polygon" }]
        }
        _ => Vec::new(),
    }
}

/// The **shapes** of a file, and what was left out of it.
fn shapes_of_tree(
    element: &craftkit_sdk::xml::Element,
    scale: f64,
    shapes: &mut Vec<Shape>,
    left_out: &mut Vec<String>,
    depth: usize,
    counted: &mut usize,
) {
    if depth > MAX_DEPTH {
        left_out.push("a deep nest".to_string());
        return;
    }
    for child in element.children.iter() {
        *counted += 1;
        if *counted > MAX_ELEMENTS {
            left_out.push("too many elements".to_string());
            return;
        }
        if child.attribute("transform").is_some() {
            left_out.push("a transform".to_string());
            continue;
        }
        match child.name.as_str() {
            "g" | "svg" => shapes_of_tree(child, scale, shapes, left_out, depth + 1, counted),
            "path" => {
                let Some(text) = child.attribute("d") else { continue };
                match runs_of(text) {
                    Ok(runs) => shapes.extend(
                        runs.into_iter()
                            .map(|run| Shape::Polyline { points: run.points, closed: run.closed }),
                    ),
                    Err(reason) => left_out.push(reason),
                }
            }
            "line" | "circle" | "ellipse" | "rect" | "polyline" | "polygon" => {
                shapes.extend(shapes_of(child, scale));
            }
            "text" | "tspan" => left_out.push("text".to_string()),
            "image" => left_out.push("an image".to_string()),
            "use" => left_out.push("a use".to_string()),
            "defs" | "style" | "title" | "desc" | "metadata" | "symbol" | "marker" => {
                left_out.push(child.name.clone());
            }
            other => left_out.push(other.to_string()),
        }
    }
}

/// The **page** of a file: its box and the millimetres a user unit
/// holds.
fn page_of(root: &craftkit_sdk::xml::Element) -> (Option<(f64, f64, f64, f64)>, f64) {
    let box_of = root.attribute("viewBox").and_then(|text| {
        let numbers = numbers_of(text);
        match numbers.as_slice() {
            [x, y, width, height]
                if width.is_finite() && height.is_finite() && *width > 0.0 && *height > 0.0 =>
            {
                Some((*x, *y, *width, *height))
            }
            _ => None,
        }
    });
    // The **size** of the page: the root's width in millimetres over
    // the width of the view box.
    let width_mm = root.attribute("width").and_then(length_in_mm);
    let height_mm = root.attribute("height").and_then(length_in_mm);
    let scale = match (box_of, width_mm, height_mm) {
        (Some((_, _, width, _)), Some(millimetres), _) if millimetres > 0.0 => millimetres / width,
        (Some((_, _, _, height)), _, Some(millimetres)) if millimetres > 0.0 => millimetres / height,
        _ => 25.4 / 96.0,
    };
    (box_of, scale)
}

/// The **extents** of a set of shapes, in the file's own units.
fn extents_of(shapes: &[Shape]) -> Option<(f64, f64, f64, f64)> {
    let mut points: Vec<(f64, f64)> = Vec::new();
    for shape in shapes {
        match shape {
            Shape::Line { from, to } => {
                points.push(*from);
                points.push(*to);
            }
            Shape::Circle { center, radius } => {
                points.push((center.0 - radius, center.1 - radius));
                points.push((center.0 + radius, center.1 + radius));
            }
            Shape::Ellipse { center, rx, ry } => {
                points.push((center.0 - rx, center.1 - ry));
                points.push((center.0 + rx, center.1 + ry));
            }
            Shape::Polyline { points: vertices, .. } => points.extend(vertices.iter().copied()),
        }
    }
    let min_x = points.iter().map(|point| point.0).fold(f64::INFINITY, f64::min);
    let max_x = points.iter().map(|point| point.0).fold(f64::NEG_INFINITY, f64::max);
    let min_y = points.iter().map(|point| point.1).fold(f64::INFINITY, f64::min);
    let max_y = points.iter().map(|point| point.1).fold(f64::NEG_INFINITY, f64::max);
    if min_x.is_finite() && max_x.is_finite() && min_y.is_finite() && max_y.is_finite() {
        Some((min_x, min_y, max_x, max_y))
    } else {
        None
    }
}

commands! {
    /// **Read** an SVG file and make the **drawing** it names.
    ///
    /// The shapes of the file are entities of the drawing's model space,
    /// and the page of the file is the origin of the drawing.
    cad.import.svg {
        area: "cad",
        layer: "cad",
        icon: "file",
        mutates,
        undoable,
        params: {
            /// The file to read.

            path: Text,
            /// What to call the drawing.
            ///
            /// Without it, the name of the file.

            name: Option<Text>,
        },
        run: {

            let path = self.path.0.trim().to_string();
            if path.is_empty() {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "a file has a path".to_string(),
                });
            }
            let text = craftkit_sdk::files::read_text_bounded(std::path::Path::new(&path), MAX_BYTES).map_err(
                |message| EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: format!("the file cannot be read: {message}"),
                },
            )?;
            let root = craftkit_sdk::xml::parse(&text).map_err(|reason| EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("the file is not an SVG file: {reason}"),
            })?;
            if root.name != "svg" {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: format!("the file is not an SVG file: its root is a `{}`", root.name),
                });
            }
            let (page, scale) = page_of(&root);
            let mut shapes: Vec<Shape> = Vec::new();
            let mut left_out: Vec<String> = Vec::new();
            let mut counted = 0usize;
            shapes_of_tree(&root, scale, &mut shapes, &mut left_out, 0, &mut counted);
            // The **box** of the drawing: the page of the file, or the
            // extents of its shapes.
            let (min_x, top) = match page {
                Some((x, y, _width, height)) => (x, y + height),
                None => match extents_of(&shapes) {
                    Some((min_x, _min_y, _max_x, max_y)) => (min_x, max_y),
                    None => (0.0, 0.0),
                },
            };
            let map = |x: f64, y: f64| json!({ "x": (x - min_x) * scale, "y": (top - y) * scale });
            // The **name** of the drawing.
            let name = match &self.name {
                Some(name) if !name.0.trim().is_empty() => name.0.trim().to_string(),
                _ => std::path::Path::new(&path)
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().to_string())
                    .unwrap_or_else(|| "the drawing".to_string()),
            };
            // The **drawing**, its **layer** and its **block**.
            let drawing = session.document_mut()?.add_node_under(None, name.clone(), "cad.drawing", None);
            let layer = session.document_mut()?.add_node_with_under(
                Some(drawing),
                "the picture".to_string(),
                "cad.layer",
                None,
                json!({ "color": Json::Null }),
            );
            let block = session.document_mut()?.add_node_with_under(
                Some(drawing),
                MODEL_SPACE.to_string(),
                "cad.block",
                None,
                json!({}),
            );
            // The **entities**.
            let mut entities = 0usize;
            let mut points = 0usize;
            let mut counts: Vec<(String, usize)> = Vec::new();
            for shape in &shapes {
                let (kind, geometry) = match shape {
                    Shape::Line { from, to } => {
                        ("line", json!({ "from": map(from.0, from.1), "to": map(to.0, to.1) }))
                    }
                    Shape::Circle { center, radius } => {
                        ("circle", json!({ "center": map(center.0, center.1), "radius": radius * scale }))
                    }
                    Shape::Ellipse { center, rx, ry } => {
                        // The **major axis** is the longer of the two, and
                        // the ratio is the shorter over the longer.
                        let (major, ratio) = if rx >= ry {
                            (json!({ "x": rx * scale, "y": 0.0 }), ry / rx)
                        } else {
                            (json!({ "x": 0.0, "y": ry * scale }), rx / ry)
                        };
                        (
                            "ellipse",
                            json!({
                                "center": map(center.0, center.1),
                                "major": major,
                                "ratio": ratio,
                                "start": 0.0,
                                "end": std::f64::consts::TAU,
                            }),
                        )
                    }
                    Shape::Polyline { points: vertices, closed } => {
                        points += vertices.len();
                        if points > MAX_POINTS {
                            left_out.push("too many points".to_string());
                            break;
                        }
                        let mapped: Vec<Json> = vertices.iter().map(|(x, y)| map(*x, *y)).collect();
                        ("polyline", json!({ "points": mapped, "closed": closed }))
                    }
                };
                let count = match counts.iter_mut().find(|(name, _)| *name == kind) {
                    Some((_, count)) => {
                        *count += 1;
                        *count
                    }
                    None => {
                        counts.push((kind.to_string(), 1));
                        1
                    }
                };
                let entity = session.document_mut()?.add_node_with_under(
                    Some(block),
                    format!("{kind} {count}"),
                    format!("cad.entity.{kind}"),
                    None,
                    json!({ "geometry": geometry }),
                );
                let relation = Relation::with_payload(
                    entity,
                    layer,
                    craftkit_sdk::layer::ON_LAYER,
                    json!({ "layer": "the picture" }),
                );
                session.document_mut()?.add_relation(relation);
                entities += 1;
            }
            // What was left out is said once each: a file may hold a
            // thousand of the same thing, and a thousand names is noise.
            left_out.sort();
            left_out.dedup();
            Ok(Outcome::changed(format!("the file `{path}` made {entities} entit(ies)")).with_data(json!({
                "path": path,
                "drawing": drawing,
                "block": block,
                "layer": layer,
                "entities": entities,
                "left_out": left_out,
                "scale": scale,
            })))

        }
    }
}
