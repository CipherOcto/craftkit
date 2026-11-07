//! The **reading** of an SVG file.
//!
//! An SVG file is a way of writing a drawing down, and this module
//! reads it back: the paths it holds, with the colours they are
//! painted with and the place they are put.
//!
//! The rules:
//!
//! - a **pixel** is 25.4/96 of a millimetre: an SVG file counts in
//!   pixels and the tree counts in millimetres, and the file is
//!   scaled into the tree's units;
//! - the world has **Y up** and an SVG file has Y down, so the
//!   drawing is **turned over** and its bottom left corner stands at
//!   the tree's origin;
//! - a **transform** is the place the drawing is put, and the
//!   transform of a group is the transform of everything in it;
//! - an **element this tree cannot read** — a shape, a text, a
//!   gradient — is **skipped**, and the answer says what it skipped;
//!   and a file with **no path** in it is refused.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{
    Colour, EngineError, Fill, FillRule, FormatValue, Outcome, Path, Point, Segment, Session, Text,
};

use craftkit_vector_geom as geom;

/// The name of the command, in the messages.
const COMMAND: &str = "vector.import.svg";

/// A **pixel**, in millimetres.
const PIXEL: f64 = 25.4 / 96.0;

/// The most bytes an SVG file may hold.
const MAX_BYTES: u64 = 16 * 1024 * 1024;

/// One **element** of an SVG file.
#[derive(Debug, Clone, PartialEq, Default)]
struct Element {
    /// The name of the element.
    name: String,
    /// Its attributes, in the order the file had them.
    attributes: Vec<(String, String)>,
    /// The elements inside it.
    children: Vec<Element>,
}

impl Element {
    /// The **attribute** of the element, by its name.
    fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes.iter().find(|(key, _)| key == name).map(|(_, value)| value.as_str())
    }

    /// The **number** of an attribute.
    fn number(&self, name: &str) -> Option<f64> {
        self.attribute(name).and_then(|value| value.trim().parse::<f64>().ok())
    }
}

/// The most **deep** a file may nest its elements.
const MAX_DEPTH: usize = 64;

/// The **elements** of an SVG file, read from its text.
///
/// The reader is small and says what it cannot read: a file that is
/// not XML is refused, and the text between the elements is ignored —
/// this tree reads the drawing and not the words around it.
fn read_elements(text: &str) -> Result<Vec<Element>, String> {
    let bytes: Vec<char> = text.chars().collect();
    let mut at = 0usize;
    read_level(&bytes, &mut at, 0)
}

/// The **elements** at one level of the file.
fn read_level(bytes: &[char], at: &mut usize, depth: usize) -> Result<Vec<Element>, String> {
    if depth > MAX_DEPTH {
        return Err("the file nests its elements deeper than the tree reads".to_string());
    }
    let mut elements: Vec<Element> = Vec::new();
    loop {
        // The next **tag**.
        while let Some(byte) = bytes.get(*at) {
            if *byte == '<' {
                break;
            }
            *at += 1;
        }
        if *at >= bytes.len() {
            break;
        }
        match bytes.get(*at + 1) {
            // A **comment**, a **declaration** or a **doctype**: not the
            // drawing.
            Some('!') | Some('?') => {
                let end = if bytes.get(*at + 1) == Some(&'?') { "?>" } else { ">" };
                *at = match find(bytes, *at, end) {
                    Some(at) => at + end.chars().count(),
                    None => bytes.len(),
                };
                continue;
            }
            // The **closing** tag of the level above.
            Some('/') => {
                *at = match find(bytes, *at, ">") {
                    Some(at) => at + 1,
                    None => bytes.len(),
                };
                break;
            }
            _ => {}
        }
        let (mut element, closed) = read_tag(bytes, at)?;
        if !closed {
            element.children = read_level(bytes, at, depth + 1)?;
        }
        elements.push(element);
    }
    Ok(elements)
}

/// One **tag**: its name, its attributes, and whether it closes
/// itself.
fn read_tag(bytes: &[char], at: &mut usize) -> Result<(Element, bool), String> {
    let mut cursor = *at + 1;
    let name = read_name(bytes, &mut cursor);
    if name.is_empty() {
        return Err("a tag has no name".to_string());
    }
    let mut attributes: Vec<(String, String)> = Vec::new();
    let mut closed = false;
    loop {
        skip_space(bytes, &mut cursor);
        match bytes.get(cursor) {
            None => return Err(format!("the tag `{name}` is never closed")),
            Some('>') => {
                cursor += 1;
                break;
            }
            Some('/') if bytes.get(cursor + 1) == Some(&'>') => {
                cursor += 2;
                closed = true;
                break;
            }
            Some('/') => {
                cursor += 1;
            }
            Some(_) => {
                let key = read_name(bytes, &mut cursor);
                if key.is_empty() {
                    cursor += 1;
                    continue;
                }
                skip_space(bytes, &mut cursor);
                if bytes.get(cursor) != Some(&'=') {
                    attributes.push((key, String::new()));
                    continue;
                }
                cursor += 1;
                skip_space(bytes, &mut cursor);
                let value = read_value(bytes, &mut cursor);
                attributes.push((key, value));
            }
        }
    }
    *at = cursor;
    Ok((Element { name, attributes, children: Vec::new() }, closed))
}

/// Where a **text** is, from a place onwards.
fn find(bytes: &[char], from: usize, text: &str) -> Option<usize> {
    let needle: Vec<char> = text.chars().collect();
    let mut at = from;
    while at + needle.len() <= bytes.len() {
        if bytes[at..at + needle.len()] == needle[..] {
            return Some(at);
        }
        at += 1;
    }
    None
}

/// The **space** of a text.
fn skip_space(bytes: &[char], at: &mut usize) {
    while let Some(byte) = bytes.get(*at) {
        if byte.is_whitespace() {
            *at += 1;
            continue;
        }
        break;
    }
}

/// The **name** of a tag or of an attribute.
fn read_name(bytes: &[char], at: &mut usize) -> String {
    let mut name = String::new();
    while let Some(byte) = bytes.get(*at) {
        if byte.is_whitespace() || *byte == '=' || *byte == '>' || *byte == '/' {
            break;
        }
        name.push(*byte);
        *at += 1;
    }
    name
}

/// The **value** of an attribute: a text in quotes.
fn read_value(bytes: &[char], at: &mut usize) -> String {
    let quote = match bytes.get(*at) {
        Some('"') => '"',
        Some('\'') => '\'',
        _ => {
            let mut value = String::new();
            while let Some(byte) = bytes.get(*at) {
                if byte.is_whitespace() || *byte == '>' {
                    break;
                }
                value.push(*byte);
                *at += 1;
            }
            return value;
        }
    };
    *at += 1;
    let mut value = String::new();
    while let Some(byte) = bytes.get(*at) {
        if *byte == quote {
            *at += 1;
            break;
        }
        value.push(*byte);
        *at += 1;
    }
    // The **entities** a file may write.
    value.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"")
}

/// A **token** of path data.
#[derive(Debug, Clone, PartialEq)]
enum Token {
    /// A command: the letter that says what to do.
    Command(char),
    /// A number.
    Number(f64),
}

/// The **tokens** of path data.
fn read_tokens(data: &str) -> Result<Vec<Token>, String> {
    let bytes: Vec<char> = data.chars().collect();
    let mut at = 0usize;
    let mut tokens: Vec<Token> = Vec::new();
    while at < bytes.len() {
        let byte = bytes[at];
        if byte.is_whitespace() || byte == ',' {
            at += 1;
            continue;
        }
        if byte.is_ascii_alphabetic() {
            tokens.push(Token::Command(byte));
            at += 1;
            continue;
        }
        let start = at;
        if bytes[at] == '+' || bytes[at] == '-' {
            at += 1;
        }
        let mut seen_dot = false;
        let mut seen_digit = false;
        while let Some(byte) = bytes.get(at) {
            match byte {
                '0'..='9' => {
                    seen_digit = true;
                    at += 1;
                }
                '.' if !seen_dot => {
                    seen_dot = true;
                    at += 1;
                }
                'e' | 'E' => {
                    at += 1;
                    if matches!(bytes.get(at), Some('+') | Some('-')) {
                        at += 1;
                    }
                }
                _ => break,
            }
        }
        if !seen_digit {
            return Err(format!("the path data has a `{}` that says nothing", byte));
        }
        let text: String = bytes[start..at].iter().collect();
        let Ok(number) = text.parse::<f64>() else {
            return Err(format!("the path data has the number `{text}` and it is not a number"));
        };
        if !number.is_finite() {
            return Err(format!("the path data has the number `{text}` and it is not finite"));
        }
        tokens.push(Token::Number(number));
    }
    Ok(tokens)
}

/// The **path** a path data holds.
///
/// The commands are the ones a file may write: a move, a line, a
/// horizontal or vertical line, a curve, a smooth curve, a quadratic
/// curve, a smooth quadratic curve and a close. An **arc** is not
/// read, and a file that holds one is refused: an arc is a curve, and
/// this tree does not write a curve it cannot say.
fn read_path(data: &str) -> Result<Path, String> {
    let tokens = read_tokens(data)?;
    let mut at = 0usize;
    let mut subpaths: Vec<craftkit_sdk::SubPath> = Vec::new();
    let mut current: Option<craftkit_sdk::SubPath> = None;
    let mut pen = Point::new(0.0, 0.0);
    let mut start = Point::new(0.0, 0.0);
    // The **control point** of the last curve, for a smooth curve.
    let mut last_cubic: Option<Point> = None;
    let mut last_quad: Option<Point> = None;
    let mut command: Option<char> = None;

    let number = |at: &mut usize| -> Option<f64> {
        match tokens.get(*at) {
            Some(Token::Number(number)) => {
                *at += 1;
                Some(*number)
            }
            _ => None,
        }
    };

    while at < tokens.len() {
        // A new **command**, or the numbers of the one that is going.
        if let Some(Token::Command(letter)) = tokens.get(at) {
            command = Some(*letter);
            at += 1;
        }
        let Some(letter) = command else {
            return Err("the path data starts with a number".to_string());
        };
        let relative = letter.is_lowercase();
        let letter = letter.to_ascii_uppercase();
        match letter {
            'M' => {
                let Some(x) = number(&mut at) else { break };
                let Some(y) = number(&mut at) else { break };
                let point = if relative { Point::new(pen.x + x, pen.y + y) } else { Point::new(x, y) };
                if let Some(subpath) = current.take() {
                    subpaths.push(subpath);
                }
                start = point;
                pen = point;
                current = Some(craftkit_sdk::SubPath { start: point, segments: Vec::new(), closed: false });
                // The **rest** of the numbers of a move are lines.
                command = Some(if relative { 'l' } else { 'L' });
                last_cubic = None;
                last_quad = None;
            }
            'L' => {
                let Some(x) = number(&mut at) else { break };
                let Some(y) = number(&mut at) else { break };
                let point = if relative { Point::new(pen.x + x, pen.y + y) } else { Point::new(x, y) };
                push_line(&mut current, &mut pen, point);
                last_cubic = None;
                last_quad = None;
            }
            'H' => {
                let Some(x) = number(&mut at) else { break };
                let point = if relative { Point::new(pen.x + x, pen.y) } else { Point::new(x, pen.y) };
                push_line(&mut current, &mut pen, point);
                last_cubic = None;
                last_quad = None;
            }
            'V' => {
                let Some(y) = number(&mut at) else { break };
                let point = if relative { Point::new(pen.x, pen.y + y) } else { Point::new(pen.x, y) };
                push_line(&mut current, &mut pen, point);
                last_cubic = None;
                last_quad = None;
            }
            'C' => {
                let Some(x1) = number(&mut at) else { break };
                let Some(y1) = number(&mut at) else { break };
                let Some(x2) = number(&mut at) else { break };
                let Some(y2) = number(&mut at) else { break };
                let Some(x) = number(&mut at) else { break };
                let Some(y) = number(&mut at) else { break };
                let (c1, c2, to) = if relative {
                    (
                        Point::new(pen.x + x1, pen.y + y1),
                        Point::new(pen.x + x2, pen.y + y2),
                        Point::new(pen.x + x, pen.y + y),
                    )
                } else {
                    (Point::new(x1, y1), Point::new(x2, y2), Point::new(x, y))
                };
                push_cubic(&mut current, &mut pen, c1, c2, to);
                last_cubic = Some(c2);
                last_quad = None;
            }
            'S' => {
                let Some(x2) = number(&mut at) else { break };
                let Some(y2) = number(&mut at) else { break };
                let Some(x) = number(&mut at) else { break };
                let Some(y) = number(&mut at) else { break };
                let c1 = match last_cubic {
                    Some(previous) => Point::new(2.0 * pen.x - previous.x, 2.0 * pen.y - previous.y),
                    None => pen,
                };
                let (c2, to) = if relative {
                    (Point::new(pen.x + x2, pen.y + y2), Point::new(pen.x + x, pen.y + y))
                } else {
                    (Point::new(x2, y2), Point::new(x, y))
                };
                push_cubic(&mut current, &mut pen, c1, c2, to);
                last_cubic = Some(c2);
                last_quad = None;
            }
            'Q' => {
                let Some(x1) = number(&mut at) else { break };
                let Some(y1) = number(&mut at) else { break };
                let Some(x) = number(&mut at) else { break };
                let Some(y) = number(&mut at) else { break };
                let (control, to) = if relative {
                    (Point::new(pen.x + x1, pen.y + y1), Point::new(pen.x + x, pen.y + y))
                } else {
                    (Point::new(x1, y1), Point::new(x, y))
                };
                let (c1, c2) = quadratic_as_cubic(pen, control, to);
                push_cubic(&mut current, &mut pen, c1, c2, to);
                last_quad = Some(control);
                last_cubic = None;
            }
            'T' => {
                let Some(x) = number(&mut at) else { break };
                let Some(y) = number(&mut at) else { break };
                let to = if relative { Point::new(pen.x + x, pen.y + y) } else { Point::new(x, y) };
                let control = match last_quad {
                    Some(previous) => Point::new(2.0 * pen.x - previous.x, 2.0 * pen.y - previous.y),
                    None => pen,
                };
                let (c1, c2) = quadratic_as_cubic(pen, control, to);
                push_cubic(&mut current, &mut pen, c1, c2, to);
                last_quad = Some(control);
                last_cubic = None;
            }
            'Z' => {
                if let Some(mut subpath) = current.take() {
                    subpath.closed = true;
                    subpaths.push(subpath);
                }
                pen = start;
                last_cubic = None;
                last_quad = None;
                command = None;
            }
            'A' => return Err("the path data holds an arc, and this tree does not read arcs".to_string()),
            other => {
                return Err(format!(
                    "the path data holds the command `{other}` and this tree does not read it"
                ));
            }
        }
    }
    if let Some(subpath) = current.take() {
        subpaths.push(subpath);
    }
    if subpaths.is_empty() {
        return Err("the path data holds no path".to_string());
    }
    Ok(Path { subpaths, rule: FillRule::NonZero })
}

/// Add a **line** to the path being built.
fn push_line(current: &mut Option<craftkit_sdk::SubPath>, pen: &mut Point, to: Point) {
    if let Some(subpath) = current.as_mut() {
        subpath.segments.push(Segment::Line { to });
    }
    *pen = to;
}

/// Add a **curve** to the path being built.
fn push_cubic(current: &mut Option<craftkit_sdk::SubPath>, pen: &mut Point, c1: Point, c2: Point, to: Point) {
    if let Some(subpath) = current.as_mut() {
        subpath.segments.push(Segment::Cubic { c1, c2, to });
    }
    *pen = to;
}

/// A **quadratic curve** as the cubic it is.
fn quadratic_as_cubic(from: Point, control: Point, to: Point) -> (Point, Point) {
    let c1 = Point::new(from.x + 2.0 / 3.0 * (control.x - from.x), from.y + 2.0 / 3.0 * (control.y - from.y));
    let c2 = Point::new(to.x + 2.0 / 3.0 * (control.x - to.x), to.y + 2.0 / 3.0 * (control.y - to.y));
    (c1, c2)
}

/// The **transform** of an element, as the file writes it.
///
/// The transforms are the ones a file may write: a move, a scale, a
/// turn and a matrix. A transform this tree does not know is refused:
/// a drawing that is put somewhere else is a different drawing.
fn read_transform(text: &str) -> Result<geom::Matrix, String> {
    let mut matrix = geom::Matrix::identity();
    let mut at = 0usize;
    let bytes: Vec<char> = text.chars().collect();
    while at < bytes.len() {
        while at < bytes.len() && (bytes[at].is_whitespace() || bytes[at] == ',') {
            at += 1;
        }
        if at >= bytes.len() {
            break;
        }
        let start = at;
        while at < bytes.len() && bytes[at].is_ascii_alphabetic() {
            at += 1;
        }
        let name: String = bytes[start..at].iter().collect();
        if name.is_empty() {
            return Err(format!("the transform holds a `{}` that says nothing", bytes[at]));
        }
        if bytes.get(at) != Some(&'(') {
            return Err(format!("the transform holds `{name}` and it has no numbers"));
        }
        at += 1;
        let mut numbers: Vec<f64> = Vec::new();
        while let Some(byte) = bytes.get(at) {
            if *byte == ')' {
                at += 1;
                break;
            }
            if byte.is_whitespace() || *byte == ',' {
                at += 1;
                continue;
            }
            let number_start = at;
            if *byte == '+' || *byte == '-' {
                at += 1;
            }
            let mut seen_digit = false;
            while let Some(byte) = bytes.get(at) {
                match byte {
                    '0'..='9' => {
                        seen_digit = true;
                        at += 1;
                    }
                    '.' => {
                        at += 1;
                    }
                    _ => break,
                }
            }
            if !seen_digit {
                return Err(format!("the transform holds `{name}` and one of its numbers is not a number"));
            }
            let text: String = bytes[number_start..at].iter().collect();
            let Ok(number) = text.parse::<f64>() else {
                return Err(format!("the transform holds the number `{text}` and it is not a number"));
            };
            numbers.push(number);
        }
        let step = match name.as_str() {
            "translate" => {
                let dx = numbers.first().copied().unwrap_or(0.0);
                let dy = numbers.get(1).copied().unwrap_or(0.0);
                geom::Matrix::translate(dx, dy)
            }
            "scale" => {
                let sx = numbers.first().copied().unwrap_or(1.0);
                let sy = numbers.get(1).copied().unwrap_or(sx);
                geom::Matrix { a: sx, b: 0.0, c: 0.0, d: sy, e: 0.0, f: 0.0 }
            }
            "rotate" => {
                let angle = numbers.first().copied().unwrap_or(0.0);
                let radians = angle.to_radians();
                let (sin, cos) = radians.sin_cos();
                let turn = geom::Matrix { a: cos, b: sin, c: -sin, d: cos, e: 0.0, f: 0.0 };
                match (numbers.get(1), numbers.get(2)) {
                    (Some(cx), Some(cy)) => {
                        let centre = Point::new(*cx, *cy);
                        let to = geom::Matrix::translate(-centre.x, -centre.y);
                        let back = geom::Matrix::translate(centre.x, centre.y);
                        to.then(&turn).then(&back)
                    }
                    _ => turn,
                }
            }
            "matrix" => {
                if numbers.len() != 6 {
                    return Err("a matrix of a transform holds six numbers".to_string());
                }
                geom::Matrix {
                    a: numbers[0],
                    b: numbers[1],
                    c: numbers[2],
                    d: numbers[3],
                    e: numbers[4],
                    f: numbers[5],
                }
            }
            other => return Err(format!("the transform holds `{other}` and this tree does not read it")),
        };
        matrix = matrix.then(&step);
    }
    Ok(matrix)
}

/// The **colour** a file writes.
///
/// The colours a file may write: the short form and the long form of
/// a hexadecimal colour, the `rgb` form, a name, and `none` — which
/// is not a colour at all, and is the absence of one.
fn read_colour(text: &str) -> Option<Colour> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    if let Some(hex) = text.strip_prefix('#') {
        let digits: Vec<char> = hex.chars().collect();
        let value = |digit: char| digit.to_digit(16).map(|digit| digit as f32 / 15.0);
        return match digits.len() {
            3 => {
                let red = value(digits[0])?;
                let green = value(digits[1])?;
                let blue = value(digits[2])?;
                Some(Colour([red, green, blue, 1.0]))
            }
            6 => {
                let pair = |at: usize| {
                    let high = digits[at].to_digit(16)? as f32 / 255.0;
                    let low = digits[at + 1].to_digit(16)? as f32 / 255.0;
                    Some(high * 16.0 + low)
                };
                Some(Colour([pair(0)?, pair(2)?, pair(4)?, 1.0]))
            }
            _ => None,
        };
    }
    if let Some(rest) = text.strip_prefix("rgb(") {
        let rest = rest.strip_suffix(')')?;
        let parts: Vec<f64> = rest.split(',').filter_map(|part| part.trim().parse::<f64>().ok()).collect();
        if parts.len() != 3 {
            return None;
        }
        return Some(Colour([
            (parts[0] / 255.0).clamp(0.0, 1.0) as f32,
            (parts[1] / 255.0).clamp(0.0, 1.0) as f32,
            (parts[2] / 255.0).clamp(0.0, 1.0) as f32,
            1.0,
        ]));
    }
    let named = match text.to_ascii_lowercase().as_str() {
        "black" => [0.0, 0.0, 0.0],
        "white" => [1.0, 1.0, 1.0],
        "red" => [1.0, 0.0, 0.0],
        "green" => [0.0, 0.5019608, 0.0],
        "blue" => [0.0, 0.0, 1.0],
        "yellow" => [1.0, 1.0, 0.0],
        "cyan" => [0.0, 1.0, 1.0],
        "magenta" => [1.0, 0.0, 1.0],
        "gray" | "grey" => [0.5019608, 0.5019608, 0.5019608],
        "orange" => [1.0, 0.6470588, 0.0],
        "purple" => [0.5019608, 0.0, 0.5019608],
        _ => return None,
    };
    Some(Colour([named[0], named[1], named[2], 1.0]))
}

/// One **path** of the file, as the tree holds it.
struct Brought {
    /// The path, in the units of the file.
    path: Path,
    /// The transform that puts it in the tree's world.
    transform: geom::Matrix,
    /// The fill.
    fill: Option<Colour>,
    /// The stroke.
    stroke: Option<(Colour, f64)>,
    /// The opacity of the path.
    opacity: f64,
}

/// The **paths** of an SVG file, and the elements it holds that this
/// tree does not read.
fn bring(elements: &[Element], transform: geom::Matrix, out: &mut Vec<Brought>, skipped: &mut Vec<String>) {
    for element in elements {
        let own = match element.attribute("transform") {
            Some(text) => match read_transform(text) {
                Ok(matrix) => matrix,
                Err(reason) => {
                    skipped.push(format!("{}: {reason}", element.name));
                    continue;
                }
            },
            None => geom::Matrix::identity(),
        };
        let here = own.then(&transform);
        match element.name.as_str() {
            "path" => {
                let Some(data) = element.attribute("d") else {
                    skipped.push("path: it has no data".to_string());
                    continue;
                };
                let mut path = match read_path(data) {
                    Ok(path) => path,
                    Err(reason) => {
                        skipped.push(format!("path: {reason}"));
                        continue;
                    }
                };
                let rule = match element.attribute("fill-rule") {
                    Some(rule) if rule.eq_ignore_ascii_case("evenodd") => FillRule::EvenOdd,
                    _ => FillRule::NonZero,
                };
                path.rule = rule;
                let fill = match element.attribute("fill") {
                    Some("none") => None,
                    Some(text) => match read_colour(text) {
                        Some(colour) => Some(colour),
                        None => {
                            skipped.push(format!("path: the fill `{text}` is not a colour"));
                            None
                        }
                    },
                    None => Some(Colour::BLACK),
                };
                let stroke = match element.attribute("stroke") {
                    Some("none") | None => None,
                    Some(text) => match read_colour(text) {
                        Some(colour) => {
                            let width = element.number("stroke-width").unwrap_or(1.0);
                            Some((colour, width))
                        }
                        None => {
                            skipped.push(format!("path: the stroke `{text}` is not a colour"));
                            None
                        }
                    },
                };
                let opacity = element.number("opacity").unwrap_or(1.0).clamp(0.0, 1.0);
                out.push(Brought { path, transform: here, fill, stroke, opacity });
            }
            "g" | "svg" => bring(&element.children, here, out, skipped),
            other => skipped.push(other.to_string()),
        }
    }
}

/// The **box** of an SVG file: the area it draws in.
///
/// The box is the `viewBox` when the file has one, and the `width`
/// and the `height` otherwise. A file that says neither has no box,
/// and the drawing is put where the file says it is.
fn read_box(root: &Element) -> Option<(f64, f64, f64, f64)> {
    if let Some(view) = root.attribute("viewBox") {
        let numbers: Vec<f64> = view
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|part| !part.is_empty())
            .filter_map(|part| part.parse::<f64>().ok())
            .collect();
        if numbers.len() == 4 {
            return Some((numbers[0], numbers[1], numbers[2], numbers[3]));
        }
    }
    let width = root.number("width")?;
    let height = root.number("height")?;
    Some((0.0, 0.0, width, height))
}

/// **Read** an **SVG file** into the document.
///
/// The paths of the file become paths of the document, in the tree's
/// millimetres, with the drawing's bottom left corner at the tree's
/// origin.
#[derive(Command)]
#[command(id = "vector.import.svg", area = "vector", layer = "vector", icon = "file", mutates, undoable)]
pub struct ImportSvg {
    /// The file to read.
    pub path: Text,
    /// The name to give the paths.
    pub name: Option<Text>,
}

impl ImportSvg {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let path = self.path.0.clone();
        if path.trim().is_empty() {
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
        let roots = read_elements(&text).map_err(|reason| EngineError::Failed {
            id: COMMAND.to_string(),
            message: format!("the file is not an SVG file: {reason}"),
        })?;
        let Some(root) = roots.iter().find(|element| element.name == "svg") else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "the file holds no `svg` element".to_string(),
            });
        };
        // The **box** of the file, and the turn of the world.
        let (x, y, width, height) = read_box(root).unwrap_or((0.0, 0.0, 0.0, 0.0));
        let mapping =
            geom::Matrix { a: PIXEL, b: 0.0, c: 0.0, d: -PIXEL, e: -x * PIXEL, f: (y + height) * PIXEL };
        let mut brought: Vec<Brought> = Vec::new();
        let mut skipped: Vec<String> = Vec::new();
        bring(&root.children, mapping, &mut brought, &mut skipped);
        if brought.is_empty() {
            // The file holds **no path**: and the answer says what it
            // skipped, so that a file this tree cannot read says why.
            let reason = if skipped.is_empty() { String::new() } else { format!(": {}", skipped.join("; ")) };
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("the file holds no path to read{reason}"),
            });
        }
        // The **paths** into the document.
        let mut ids: Vec<u64> = Vec::with_capacity(brought.len());
        for (at, path) in brought.into_iter().enumerate() {
            let name = match &self.name {
                Some(name) => format!("{} {}", name.0, at + 1),
                None => format!("path {}", at + 1),
            };
            let fill = match path.fill {
                Some(colour) => Fill::Solid(colour),
                None => Fill::Solid(Colour::NONE),
            };
            let stroke = path.stroke.map(|(colour, width)| craftkit_sdk::Stroke {
                colour,
                width,
                join: craftkit_sdk::Join::Miter,
                cap: craftkit_sdk::Cap::Butt,
            });
            let vector = crate::VectorNode {
                path: path.path,
                fill,
                stroke,
                blend: craftkit_sdk::BlendMode::Normal,
                transform: path.transform,
                opacity: path.opacity,
                dash: None,
            };
            let rect = geom::bounds(&vector.placed());
            let id = session.document_mut()?.add_node_with(name, "vector.path", Some(rect), vector.to_json());
            ids.push(id);
        }
        let count = ids.len();
        Ok(Outcome::changed(format!("read {count} path(s)")).with_data(json!({
            "ids": ids,
            "paths": count,
            "skipped": skipped,
            "box": [x, y, width, height],
        })))
    }
}
