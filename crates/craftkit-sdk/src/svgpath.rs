//! The **reading** of SVG path data.
//!
//! Path data is the text of a `d` attribute: a list of commands and
//! numbers that says where a line goes. This module reads that text into
//! a [`Path`] — the contours and the segments the tree draws.
//!
//! The commands are the ones a file may write:
//!
//! | command | what it says |
//! |---|---|
//! | `M` | a move |
//! | `L` | a line |
//! | `H` | a horizontal line |
//! | `V` | a vertical line |
//! | `C` | a cubic curve |
//! | `S` | a smooth cubic curve |
//! | `Q` | a quadratic curve |
//! | `T` | a smooth quadratic curve |
//! | `A` | an elliptical arc |
//! | `Z` | a close |
//!
//! Every command has a **relative** spelling — the lower-case letter —
//! and the relative spelling says the same thing *from the pen*, the
//! point the drawing is at.
//!
//! Two shapes are **not** drawn but are stored as curves: a quadratic
//! curve is stored as the cubic it is, and so is an elliptical arc,
//! which is split into cubics of at most ninety degrees each.

use crate::document::{FillRule, Path, Point, Segment, SubPath};

/// One **piece** of an arc: a line or a curve.
#[derive(Debug, Clone, PartialEq)]
enum ArcPiece {
    /// A straight line, from an arc whose radii are nothing.
    Line(Point),
    /// A cubic curve.
    Cubic(Point, Point, Point),
}

/// The most segments one elliptical arc is split into.
///
/// The split is one cubic per at most ninety degrees of sweep, and an
/// arc sweeps at most three hundred and sixty degrees, so the count is
/// four at the most. The cap is here so that a hostile file cannot
/// ask for more work than this.
const MAX_ARC_SEGMENTS: usize = 4;

/// A **token** of path data.
#[derive(Debug, Clone, PartialEq)]
enum Token {
    /// A command: the letter that says what to do.
    Command(char),
    /// A number.
    Number(f64),
    /// A **flag**: an arc's `large-arc` and `sweep`.
    Flag(bool),
}

/// The **tokens** of path data.
///
/// The tokenizer knows the **command** it is under, because an arc's
/// flags are **single characters** and not numbers: a file writes
/// `A 1 1 0 0110 0`, and the `011` is a flag, a flag and a number — and
/// a number reader would have read it as one number of one hundred and
/// eleven.
fn read_tokens(data: &str) -> Result<Vec<Token>, String> {
    let bytes: Vec<char> = data.chars().collect();
    let mut at = 0usize;
    let mut tokens: Vec<Token> = Vec::new();
    // The **command** the numbers are under, and **how many** of them
    // have been read: an arc's flags are its fourth and its fifth.
    let mut command: Option<char> = None;
    let mut index = 0usize;
    while at < bytes.len() {
        let byte = bytes[at];
        if byte.is_whitespace() || byte == ',' {
            at += 1;
            continue;
        }
        if byte.is_ascii_alphabetic() {
            tokens.push(Token::Command(byte));
            command = Some(byte.to_ascii_uppercase());
            index = 0;
            at += 1;
            continue;
        }
        // A **flag**: a single `0` or a single `1`.
        if command == Some('A') && (index == 3 || index == 4) {
            if byte != '0' && byte != '1' {
                return Err(format!("the path data has the flag `{byte}`, and a flag is a `0` or a `1`"));
            }
            tokens.push(Token::Flag(byte == '1'));
            at += 1;
            index += 1;
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
        index += 1;
        if let Some(letter) = command {
            let count = parameter_count(letter);
            if count > 0 {
                index %= count;
            }
        }
    }
    Ok(tokens)
}

/// How many numbers a **command** takes.
///
/// The count is what says where an arc's flags are: they are its fourth
/// and its fifth.
fn parameter_count(letter: char) -> usize {
    match letter {
        'M' | 'L' | 'T' => 2,
        'H' | 'V' => 1,
        'C' => 6,
        'S' | 'Q' => 4,
        'A' => 7,
        _ => 0,
    }
}

/// A **point** that stays in numbers.
///
/// The numbers in path data are finite, but the **sum** of two of them
/// need not be: a relative move of a very large number overflows, and a
/// point that is not a number is not a point.
fn point(x: f64, y: f64) -> Result<Point, String> {
    if !x.is_finite() || !y.is_finite() {
        return Err("the path data holds a point that does not stay in numbers".to_string());
    }
    Ok(Point::new(x, y))
}

/// The **path** a path data holds.
///
/// The commands are the ones a file may write: a move, a line, a
/// horizontal or vertical line, a curve, a smooth curve, a quadratic
/// curve, a smooth quadratic curve, an arc and a close.
///
/// # Errors
///
/// The path data that holds no path at all is refused, and so is the
/// path data that holds a command this tree does not read, or a number
/// that is not a number, or a number that is not finite, or a command
/// with **not enough numbers** — a path that is cut short is refused
/// and not read as far as it goes.
pub fn read_path(data: &str) -> Result<Path, String> {
    let tokens = read_tokens(data)?;
    let mut at = 0usize;
    let mut subpaths: Vec<SubPath> = Vec::new();
    let mut current: Option<SubPath> = None;
    let mut pen = Point::new(0.0, 0.0);
    let mut start = Point::new(0.0, 0.0);
    // The **control point** of the last curve, for a smooth curve.
    let mut last_cubic: Option<Point> = None;
    let mut last_quad: Option<Point> = None;
    let mut command: Option<char> = None;
    // A path **begins** with a move: the file says so, and a path that
    // begins with a line has no place to begin the line from.
    let mut begun = false;

    let number = |at: &mut usize, letter: char| -> Result<f64, String> {
        match tokens.get(*at) {
            Some(Token::Number(number)) => {
                *at += 1;
                Ok(*number)
            }
            _ => Err(format!("the path data holds the command `{letter}` and not enough numbers")),
        }
    };
    let flag = |at: &mut usize, letter: char| -> Result<bool, String> {
        match tokens.get(*at) {
            Some(Token::Flag(flag)) => {
                *at += 1;
                Ok(*flag)
            }
            _ => Err(format!("the path data holds the command `{letter}` and not enough numbers")),
        }
    };

    while at < tokens.len() {
        // A new **command**, or the numbers of the one that is going.
        if let Some(Token::Command(letter)) = tokens.get(at) {
            command = Some(*letter);
            at += 1;
        }
        let Some(letter) = command else {
            return Err("the path data holds a number and not a command".to_string());
        };
        let relative = letter.is_lowercase();
        let letter = letter.to_ascii_uppercase();
        if !begun && letter != 'M' {
            return Err(format!(
                "the path data begins with the command `{letter}`, and a path begins with a move"
            ));
        }
        match letter {
            'M' => {
                let x = number(&mut at, letter)?;
                let y = number(&mut at, letter)?;
                let point = if relative { point(pen.x + x, pen.y + y)? } else { point(x, y)? };
                if let Some(subpath) = current.take() {
                    subpaths.push(subpath);
                }
                start = point;
                pen = point;
                current = Some(SubPath { start: point, segments: Vec::new(), closed: false });
                begun = true;
                // The **rest** of the numbers of a move are lines.
                command = Some(if relative { 'l' } else { 'L' });
                last_cubic = None;
                last_quad = None;
            }
            'L' => {
                let x = number(&mut at, letter)?;
                let y = number(&mut at, letter)?;
                let point = if relative { point(pen.x + x, pen.y + y)? } else { point(x, y)? };
                push_line(&mut current, &mut pen, point);
                last_cubic = None;
                last_quad = None;
            }
            'H' => {
                let x = number(&mut at, letter)?;
                let point = if relative { point(pen.x + x, pen.y)? } else { point(x, pen.y)? };
                push_line(&mut current, &mut pen, point);
                last_cubic = None;
                last_quad = None;
            }
            'V' => {
                let y = number(&mut at, letter)?;
                let point = if relative { point(pen.x, pen.y + y)? } else { point(pen.x, y)? };
                push_line(&mut current, &mut pen, point);
                last_cubic = None;
                last_quad = None;
            }
            'C' => {
                let x1 = number(&mut at, letter)?;
                let y1 = number(&mut at, letter)?;
                let x2 = number(&mut at, letter)?;
                let y2 = number(&mut at, letter)?;
                let x = number(&mut at, letter)?;
                let y = number(&mut at, letter)?;
                let (c1, c2, to) = if relative {
                    (
                        point(pen.x + x1, pen.y + y1)?,
                        point(pen.x + x2, pen.y + y2)?,
                        point(pen.x + x, pen.y + y)?,
                    )
                } else {
                    (point(x1, y1)?, point(x2, y2)?, point(x, y)?)
                };
                push_cubic(&mut current, &mut pen, c1, c2, to);
                last_cubic = Some(c2);
                last_quad = None;
            }
            'S' => {
                let x2 = number(&mut at, letter)?;
                let y2 = number(&mut at, letter)?;
                let x = number(&mut at, letter)?;
                let y = number(&mut at, letter)?;
                let c1 = match last_cubic {
                    Some(previous) => point(2.0 * pen.x - previous.x, 2.0 * pen.y - previous.y)?,
                    None => pen,
                };
                let (c2, to) = if relative {
                    (point(pen.x + x2, pen.y + y2)?, point(pen.x + x, pen.y + y)?)
                } else {
                    (point(x2, y2)?, point(x, y)?)
                };
                push_cubic(&mut current, &mut pen, c1, c2, to);
                last_cubic = Some(c2);
                last_quad = None;
            }
            'Q' => {
                let x1 = number(&mut at, letter)?;
                let y1 = number(&mut at, letter)?;
                let x = number(&mut at, letter)?;
                let y = number(&mut at, letter)?;
                let (control, to) = if relative {
                    (point(pen.x + x1, pen.y + y1)?, point(pen.x + x, pen.y + y)?)
                } else {
                    (point(x1, y1)?, point(x, y)?)
                };
                let (c1, c2) = quadratic_as_cubic(pen, control, to);
                push_cubic(&mut current, &mut pen, c1, c2, to);
                last_quad = Some(control);
                last_cubic = None;
            }
            'T' => {
                let x = number(&mut at, letter)?;
                let y = number(&mut at, letter)?;
                let to = if relative { point(pen.x + x, pen.y + y)? } else { point(x, y)? };
                let control = match last_quad {
                    Some(previous) => point(2.0 * pen.x - previous.x, 2.0 * pen.y - previous.y)?,
                    None => pen,
                };
                let (c1, c2) = quadratic_as_cubic(pen, control, to);
                push_cubic(&mut current, &mut pen, c1, c2, to);
                last_quad = Some(control);
                last_cubic = None;
            }
            'A' => {
                let rx = number(&mut at, letter)?;
                let ry = number(&mut at, letter)?;
                let rotation = number(&mut at, letter)?;
                // The **flags**: a single `0` or a single `1`, and not a
                // number — the file may write them without a separator,
                // and a number reader would swallow the next number with
                // them.
                let large = flag(&mut at, letter)?;
                let sweep = flag(&mut at, letter)?;
                let x = number(&mut at, letter)?;
                let y = number(&mut at, letter)?;
                let to = if relative { point(pen.x + x, pen.y + y)? } else { point(x, y)? };
                let pieces = arc_as_cubics(pen, to, rx, ry, rotation, large, sweep)?;
                for piece in pieces {
                    match piece {
                        ArcPiece::Line(end) => push_line(&mut current, &mut pen, end),
                        ArcPiece::Cubic(c1, c2, end) => push_cubic(&mut current, &mut pen, c1, c2, end),
                    }
                }
                // An arc is not a curve a **smooth** curve may lean on.
                last_cubic = None;
                last_quad = None;
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
fn push_line(current: &mut Option<SubPath>, pen: &mut Point, to: Point) {
    if let Some(subpath) = current.as_mut() {
        subpath.segments.push(Segment::Line { to });
    }
    *pen = to;
}

/// Add a **curve** to the path being built.
fn push_cubic(current: &mut Option<SubPath>, pen: &mut Point, c1: Point, c2: Point, to: Point) {
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

/// An **elliptical arc** as the cubics it is.
///
/// The arc is turned into the centre of its ellipse and the two angles
/// it runs through, and each piece of at most ninety degrees is then
/// written as one cubic — the same four thirds of a tangent the
/// quadratic curve uses.
///
/// # Errors
///
/// An arc whose numbers do not make an ellipse is refused: a radius
/// that is not a number, a point that is not finite.
fn arc_as_cubics(
    from: Point,
    to: Point,
    rx: f64,
    ry: f64,
    rotation: f64,
    large_arc: bool,
    sweep: bool,
) -> Result<Vec<ArcPiece>, String> {
    // The radii of an ellipse are **lengths**: a negative one is the
    // same ellipse the other way round.
    let mut rx = rx.abs();
    let mut ry = ry.abs();
    // A radius of nothing is not an ellipse: the arc is a **line**.
    if rx == 0.0 || ry == 0.0 {
        return Ok(vec![ArcPiece::Line(to)]);
    }
    // The arc starts and ends at the same place: there is nothing to
    // draw, and the pen does not move.
    if from.x == to.x && from.y == to.y {
        return Ok(Vec::new());
    }
    let phi = rotation.to_radians();
    let (sin_phi, cos_phi) = phi.sin_cos();

    // The **half** of the distance between the two ends, turned into
    // the ellipse's own axes.
    let dx = (from.x - to.x) / 2.0;
    let dy = (from.y - to.y) / 2.0;
    let x1 = cos_phi * dx + sin_phi * dy;
    let y1 = -sin_phi * dx + cos_phi * dy;

    // The radii are **scaled up** when they are too small to reach the
    // two ends, which is what a file's arc means.
    let squared = x1 * x1 / (rx * rx) + y1 * y1 / (ry * ry);
    if squared > 1.0 {
        let scale = squared.sqrt();
        rx *= scale;
        ry *= scale;
    }

    // The **centre** of the ellipse.
    let rx2 = rx * rx;
    let ry2 = ry * ry;
    let numerator = rx2 * ry2 - rx2 * y1 * y1 - ry2 * x1 * x1;
    let denominator = rx2 * y1 * y1 + ry2 * x1 * x1;
    if denominator <= 0.0 {
        return Err("the path data holds an arc that has no ellipse".to_string());
    }
    let coefficient = (numerator / denominator).max(0.0).sqrt();
    let sign = if large_arc != sweep { 1.0 } else { -1.0 };
    let cxp = sign * coefficient * rx * y1 / ry;
    let cyp = -sign * coefficient * ry * x1 / rx;
    let cx = cos_phi * cxp - sin_phi * cyp + (from.x + to.x) / 2.0;
    let cy = sin_phi * cxp + cos_phi * cyp + (from.y + to.y) / 2.0;

    // The **angles** the arc runs through.
    //
    // The angles are in the **ellipse's own** frame — the frame the radii
    // are measured in — and the point on the ellipse at an angle is
    // `(cx + rx·cos, cy + ry·sin)` turned by the rotation. So the angle
    // of a point is `atan2(y, x)`: the **y** first.
    let ux = (x1 - cxp) / rx;
    let uy = (y1 - cyp) / ry;
    let vx = (-x1 - cxp) / rx;
    let vy = (-y1 - cyp) / ry;
    let theta = uy.atan2(ux);
    let mut delta = vy.atan2(vx) - theta;
    if !sweep && delta > 0.0 {
        delta -= std::f64::consts::TAU;
    } else if sweep && delta < 0.0 {
        delta += std::f64::consts::TAU;
    }

    // The arc is split into pieces of at most ninety degrees.
    let count = (delta.abs() / std::f64::consts::FRAC_PI_2).ceil().max(1.0) as usize;
    let count = count.min(MAX_ARC_SEGMENTS);
    let step = delta / count as f64;

    let mut segments: Vec<ArcPiece> = Vec::with_capacity(count);
    for piece in 0..count {
        let start = theta + step * piece as f64;
        let end = start + step;
        let alpha = 4.0 / 3.0 * (step / 4.0).tan();
        let p0 = ellipse_point(cx, cy, rx, ry, sin_phi, cos_phi, start);
        let p3 = ellipse_point(cx, cy, rx, ry, sin_phi, cos_phi, end);
        let d0 = ellipse_tangent(rx, ry, sin_phi, cos_phi, start);
        let d1 = ellipse_tangent(rx, ry, sin_phi, cos_phi, end);
        let c1 = Point::new(p0.x + alpha * d0.x, p0.y + alpha * d0.y);
        let c2 = Point::new(p3.x - alpha * d1.x, p3.y - alpha * d1.y);
        for point in [p0, c1, c2, p3] {
            if !point.x.is_finite() || !point.y.is_finite() {
                return Err("the path data holds an arc that does not stay in numbers".to_string());
            }
        }
        segments.push(ArcPiece::Cubic(c1, c2, p3));
    }
    Ok(segments)
}

/// The **point** of an ellipse at an angle.
fn ellipse_point(cx: f64, cy: f64, rx: f64, ry: f64, sin_phi: f64, cos_phi: f64, angle: f64) -> Point {
    let (sin, cos) = angle.sin_cos();
    let x = rx * cos;
    let y = ry * sin;
    Point::new(cx + cos_phi * x - sin_phi * y, cy + sin_phi * x + cos_phi * y)
}

/// The **tangent** of an ellipse at an angle, for one radian.
fn ellipse_tangent(rx: f64, ry: f64, sin_phi: f64, cos_phi: f64, angle: f64) -> Point {
    let (sin, cos) = angle.sin_cos();
    let x = -rx * sin;
    let y = ry * cos;
    Point::new(cos_phi * x - sin_phi * y, sin_phi * x + cos_phi * y)
}
