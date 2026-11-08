//! The **content stream**: what a page says to draw.
//!
//! A page's content is a stream of **operands** and **operators**: the
//! operands are the numbers, the names, the strings and the arrays that
//! come before an operator, and the operator says what to do with
//! them. `10 20 m` moves the pen to the point (10, 20), and
//! `100 100 200 200 re` draws a rectangle.
//!
//! This module **reads** a content stream: it says what the stream
//! says, in the order the stream says it. It does not draw it — the
//! drawing of a PDF page is the evaluator's business and not this
//! module's.
//!
//! # What it does not do
//!
//! - **The inline images are kept whole.** A stream may hold an image
//!   between `BI` and `EI`, and the bytes between are the image and
//!   not a list of operands; the operation keeps the dictionary that
//!   describes it and the bytes of it, so that a stream that is read
//!   and written again is the stream it was.
//! - **The operators are not carried out.** What a `re` means is the
//!   evaluator's; this module says that a `re` is there.
//!
//! # The writer
//!
//! The operations of a stream may be **written back** to bytes, by the
//! tree's own rule: the operands come first, each one followed by a
//! space, and then the operator and a newline. A **name** is written
//! as a name and a **string** as a string, because the two are two
//! things: the reader tells them apart, and the writer keeps them
//! apart.

use craftkit_sdk::serde_json::{Map, Value as Json};

/// A **matrix**: `a b c d e f`, the affine map of a PDF.
///
/// The row-vector convention of the tree: a point is a row, the
/// matrix is multiplied on the right, and `then` is the order things
/// happen in — `self` first, and then the other.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix(pub [f64; 6]);

impl Matrix {
    /// The matrix that changes nothing.
    pub const IDENTITY: Matrix = Matrix([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

    /// The **matrix** six numbers make, when there are six of them.
    pub fn from_numbers(numbers: &[f64]) -> Option<Matrix> {
        if numbers.len() != 6 {
            return None;
        }
        Some(Matrix([numbers[0], numbers[1], numbers[2], numbers[3], numbers[4], numbers[5]]))
    }

    /// `self` and then `then`: the order things happen in.
    pub fn then(&self, then: &Matrix) -> Matrix {
        let [a, b, c, d, e, f] = self.0;
        let [a2, b2, c2, d2, e2, f2] = then.0;
        Matrix([
            a * a2 + b * c2,
            a * b2 + b * d2,
            c * a2 + d * c2,
            c * b2 + d * d2,
            e * a2 + f * c2 + e2,
            e * b2 + f * d2 + f2,
        ])
    }

    /// Where a **point** ends up after this matrix.
    ///
    /// The tree's own rule: the row-vector convention of the
    /// specification, so a point is a row and the matrix is on the
    /// left of it.
    pub fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        let [a, b, c, d, e, f] = self.0;
        (a * x + c * y + e, b * x + d * y + f)
    }

    /// The matrix **inverted**: the transform that undoes it.
    ///
    /// The tree's own rule: the row-vector convention of the
    /// specification, so the inverse of a matrix that turns nothing is
    /// the matrix that turns nothing, and a matrix that cannot be
    /// undone — one that turns a square into a line — has no inverse.
    pub fn invert(&self) -> Option<Matrix> {
        let [a, b, c, d, e, f] = self.0;
        let det = a * d - c * b;
        if det == 0.0 || !det.is_finite() {
            return None;
        }
        Some(Matrix([d / det, -b / det, -c / det, a / det, (c * f - d * e) / det, (b * e - a * f) / det]))
    }

    /// The **box** a rectangle takes after this matrix.
    ///
    /// The tree's own rule: the four corners of the rectangle are
    /// turned, and the box that holds them is the answer — a matrix
    /// that turns turns the box with it.
    pub fn bbox(&self, rectangle: [f64; 4]) -> [f64; 4] {
        let [x0, y0, x1, y1] = rectangle;
        let points = [self.apply(x0, y0), self.apply(x1, y0), self.apply(x0, y1), self.apply(x1, y1)];
        let mut box_ = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
        for (x, y) in points {
            box_[0] = box_[0].min(x);
            box_[1] = box_[1].min(y);
            box_[2] = box_[2].max(x);
            box_[3] = box_[3].max(y);
        }
        box_
    }
}

/// Why a content stream could not be **read**.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The bytes are not what a content stream is.
    Malformed(String),
}

/// One **operation**: an operator and the operands it was given.
#[derive(Debug, Clone, PartialEq)]
pub struct Operation {
    /// The operator's name, as the stream spells it.
    pub operator: String,
    /// The operands that came before it.
    pub operands: Vec<Json>,
    /// The **bytes** of the inline image of the operation, when it
    /// has one: the dictionary that describes it is its operands.
    pub inline: Option<Vec<u8>>,
}

impl Operation {
    /// The operation, as the caller gets it.
    pub fn to_json(&self) -> Json {
        let mut map = Map::new();
        map.insert("operator".to_string(), Json::String(self.operator.clone()));
        map.insert("operands".to_string(), Json::Array(self.operands.clone()));
        map.insert("inline".to_string(), Json::Bool(self.inline.is_some()));
        Json::Object(map)
    }
}

/// The **operators** a content stream may hold.
///
/// The list is the specification's (ISO 32000-2 §8.9), and an operator
/// that is not on it is refused by name: a stream that says something
/// the kit does not know is not a stream the kit should read half of.
const OPERATORS: &[&str] = &[
    "b", "B", "B*", "b*", "BDC", "BI", "BMC", "BT", "BX", "c", "cm", "CS", "cs", "d", "d0", "d1", "Do", "DP",
    "EI", "EMC", "ET", "EX", "f", "F", "f*", "G", "g", "gs", "h", "i", "ID", "j", "J", "K", "k", "l", "m",
    "M", "MP", "n", "q", "Q", "re", "RG", "rg", "ri", "s", "S", "SC", "sc", "SCN", "scn", "sh", "T*", "Tc",
    "Td", "TD", "Tf", "Tj", "TJ", "TL", "Tm", "Tr", "Ts", "Tw", "Tz", "v", "w", "W", "W*", "y", "'", "\"",
];

/// Write the **operations** of a content stream back to bytes.
///
/// The tree's own rule: the operands come first, each one followed by
/// a space, and then the operator and a newline. A **name** is written
/// as a name and a **string** as a string, and the two are two things.
pub fn write_operations(operations: &[Operation]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    for operation in operations {
        if let Some(image) = &operation.inline {
            out.extend_from_slice(b"BI");
            for operand in &operation.operands {
                out.push(b' ');
                write_operand(operand, &mut out);
            }
            out.extend_from_slice(b" ID ");
            out.extend_from_slice(image);
            out.extend_from_slice(b"\nEI\n");
            continue;
        }
        for operand in &operation.operands {
            write_operand(operand, &mut out);
            out.push(b' ');
        }
        out.extend_from_slice(operation.operator.as_bytes());
        out.push(b'\n');
    }
    out
}

/// Write one **operand** of a content stream.
fn write_operand(value: &Json, out: &mut Vec<u8>) {
    match value {
        Json::Null => out.extend_from_slice(b"null"),
        Json::Bool(flag) => {
            if *flag {
                out.extend_from_slice(b"true");
            } else {
                out.extend_from_slice(b"false");
            }
        }
        Json::Number(number) => {
            // A number is an **integer** when it is one: the tree's
            // own rule.
            let value = number.as_f64().unwrap_or_default();
            if value.fract() == 0.0 && value.abs() < 1e15 {
                out.extend_from_slice(format!("{}", value as i64).as_bytes());
            } else {
                out.extend_from_slice(format!("{value}").as_bytes());
            }
        }
        // A **name**: it carries its own slash, and the bytes that
        // may not be in one are escaped.
        Json::String(name) => {
            let text = name.strip_prefix('/').unwrap_or(name);
            out.push(b'/');
            out.extend_from_slice(crate::writer::escape_name(text).as_bytes());
        }
        Json::Array(list) => {
            out.push(b'[');
            for item in list {
                write_operand(item, out);
                out.push(b' ');
            }
            out.push(b']');
        }
        Json::Object(map) => {
            if let Some(Json::String(text)) = map.get("string") {
                // A **string**: its bytes are its characters.
                write_string(text, out);
                return;
            }
            if let Some(Json::Array(pairs)) = map.get("entries") {
                // A **dictionary**: the pairs it holds, each one a
                // key and a value.
                out.extend_from_slice(b"<<");
                for pair in pairs {
                    out.push(b' ');
                    if let Json::Array(pair) = pair {
                        for item in pair {
                            write_operand(item, out);
                            out.push(b' ');
                        }
                    }
                }
                out.extend_from_slice(b">>");
                return;
            }
            out.extend_from_slice(b"null");
        }
    }
}

/// Write a **string**: the bytes of it, with the characters that may
/// not be in one escaped.
fn write_string(text: &str, out: &mut Vec<u8>) {
    out.push(b'(');
    for character in text.chars() {
        let byte = character as u32 as u8;
        match byte {
            b'(' => out.extend_from_slice(b"\\("),
            b')' => out.extend_from_slice(b"\\)"),
            b'\\' => out.extend_from_slice(b"\\\\"),
            b'\n' => out.extend_from_slice(b"\\n"),
            b'\r' => out.extend_from_slice(b"\\r"),
            b'\t' => out.extend_from_slice(b"\\t"),
            other => out.push(other),
        }
    }
    out.push(b')');
}

/// Read the **operations** of a content stream.
pub fn operations(bytes: &[u8]) -> Result<Vec<Operation>, Error> {
    let mut reader = Reader { bytes, at: 0 };
    let mut operations: Vec<Operation> = Vec::new();
    let mut operands: Vec<Json> = Vec::new();
    loop {
        let token = reader.token()?;
        let Some(token) = token else { break };
        match token {
            Token::Value(value) => operands.push(value),
            Token::Operator(name) => {
                if name == "ID" {
                    // The **inline image**: the bytes until `EI` are
                    // the image and not a list of operands, and they
                    // belong to the `BI` before them. The operands read
                    // since that `BI` are the dictionary of the image.
                    let image = reader.inline_image()?;
                    let dictionary = std::mem::take(&mut operands);
                    match operations.last_mut() {
                        Some(operation) if operation.operator == "BI" => {
                            operation.operands.extend(dictionary);
                            operation.inline = Some(image);
                        }
                        _ => {
                            return Err(Error::Malformed(
                                "the inline image has no `BI` before it".to_string(),
                            ));
                        }
                    }
                    continue;
                }
                operations.push(Operation {
                    operator: name,
                    operands: std::mem::take(&mut operands),
                    inline: None,
                });
            }
        }
    }
    if !operands.is_empty() {
        return Err(Error::Malformed("the stream ends with operands and no operator".to_string()));
    }
    Ok(operations)
}

/// A **token** of a content stream.
enum Token {
    /// An operand.
    Value(Json),
    /// An operator.
    Operator(String),
}

/// The reader of a content stream.
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    /// The next token, or `None` at the end of the stream.
    fn token(&mut self) -> Result<Option<Token>, Error> {
        self.skip_space();
        let Some(byte) = self.bytes.get(self.at).copied() else {
            return Ok(None);
        };
        let token = match byte {
            b'/' => Token::Value(self.name()?),
            b'(' => Token::Value(self.literal_string()?),
            b'<' => {
                if self.bytes.get(self.at + 1) == Some(&b'<') {
                    Token::Value(self.dictionary()?)
                } else {
                    Token::Value(self.hex_string()?)
                }
            }
            b'[' => Token::Value(self.array()?),
            b']' => return Err(Error::Malformed("a `]` with no `[`".to_string())),
            b'{' | b'}' => return Err(Error::Malformed("a content stream has no procedures".to_string())),
            b'0'..=b'9' | b'+' | b'-' | b'.' => {
                if let Some(number) = self.number()? {
                    Token::Value(number)
                } else {
                    self.keyword()?
                }
            }
            _ => self.keyword()?,
        };
        Ok(Some(token))
    }

    /// The **space** and the comments, which say nothing.
    fn skip_space(&mut self) {
        loop {
            let Some(byte) = self.bytes.get(self.at).copied() else { return };
            match byte {
                b' ' | b'\t' | b'\n' | b'\r' | 0x0c | 0x00 => self.at += 1,
                b'%' => {
                    while let Some(byte) = self.bytes.get(self.at).copied() {
                        self.at += 1;
                        if byte == b'\n' || byte == b'\r' {
                            break;
                        }
                    }
                }
                _ => return,
            }
        }
    }

    /// A **name**: `/Name`.
    fn name(&mut self) -> Result<Json, Error> {
        self.at += 1;
        let mut bytes: Vec<u8> = Vec::new();
        while let Some(byte) = self.bytes.get(self.at).copied() {
            if is_space(byte) || is_delimiter(byte) {
                break;
            }
            self.at += 1;
            if byte == b'#' {
                // The **escape**: `#20` is the byte that the two
                // digits stand for.
                let high = self.bytes.get(self.at).copied();
                let low = self.bytes.get(self.at + 1).copied();
                match (high, low) {
                    (Some(high), Some(low)) => match (hex(high), hex(low)) {
                        (Some(high), Some(low)) => {
                            bytes.push(high * 16 + low);
                            self.at += 2;
                        }
                        _ => return Err(Error::Malformed("a name has a bad escape".to_string())),
                    },
                    _ => return Err(Error::Malformed("a name has a bad escape".to_string())),
                }
            } else {
                bytes.push(byte);
            }
        }
        Ok(Json::String(format!("/{}", String::from_utf8_lossy(&bytes))))
    }

    /// A **literal string**: `(text)`.
    ///
    /// The bytes are kept as they are, and each one is a character: a
    /// PDF string is a sequence of bytes and not text, and the kit does
    /// not pretend otherwise.
    fn literal_string(&mut self) -> Result<Json, Error> {
        self.at += 1;
        let mut bytes: Vec<u8> = Vec::new();
        let mut depth = 1_usize;
        while let Some(byte) = self.bytes.get(self.at).copied() {
            self.at += 1;
            match byte {
                b'\\' => {
                    let Some(next) = self.bytes.get(self.at).copied() else {
                        return Err(Error::Malformed("a string ends in an escape".to_string()));
                    };
                    self.at += 1;
                    match next {
                        b'n' => bytes.push(b'\n'),
                        b'r' => bytes.push(b'\r'),
                        b't' => bytes.push(b'\t'),
                        b'b' => bytes.push(0x08),
                        b'f' => bytes.push(0x0c),
                        b'(' => bytes.push(b'('),
                        b')' => bytes.push(b')'),
                        b'\\' => bytes.push(b'\\'),
                        b'0'..=b'7' => {
                            // The **octal**: up to three digits.
                            let mut value = u32::from(next - b'0');
                            for _ in 0..2 {
                                let Some(digit) = self.bytes.get(self.at).copied() else { break };
                                if !(b'0'..=b'7').contains(&digit) {
                                    break;
                                }
                                value = value * 8 + u32::from(digit - b'0');
                                self.at += 1;
                            }
                            bytes.push((value & 0xff) as u8);
                        }
                        other => bytes.push(other),
                    }
                }
                b'(' => {
                    depth += 1;
                    bytes.push(byte);
                }
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        // A **string** is a string and not a name: the
                        // payload says so, as it does everywhere else
                        // in the kit.
                        let text: String = bytes.iter().map(|byte| *byte as char).collect();
                        return Ok(Json::Object({
                            let mut map = Map::new();
                            map.insert("string".to_string(), Json::String(text));
                            map
                        }));
                    }
                    bytes.push(byte);
                }
                other => bytes.push(other),
            }
        }
        Err(Error::Malformed("a string is never closed".to_string()))
    }

    /// A **hex string**: `<48656C6C6F>`.
    fn hex_string(&mut self) -> Result<Json, Error> {
        self.at += 1;
        let mut digits: Vec<u8> = Vec::new();
        while let Some(byte) = self.bytes.get(self.at).copied() {
            self.at += 1;
            if byte == b'>' {
                break;
            }
            if is_space(byte) {
                continue;
            }
            let Some(value) = hex(byte) else {
                return Err(Error::Malformed("a hex string has a bad digit".to_string()));
            };
            digits.push(value);
        }
        if digits.len() % 2 == 1 {
            // An odd number of digits: the last one is half a byte,
            // and the specification pads it with a zero.
            digits.push(0);
        }
        let bytes: Vec<u8> = digits.chunks(2).map(|pair| pair[0] * 16 + pair[1]).collect();
        let text: String = bytes.iter().map(|byte| *byte as char).collect();
        Ok(Json::Object({
            let mut map = Map::new();
            map.insert("string".to_string(), Json::String(text));
            map
        }))
    }

    /// A **dictionary**: `<< /Key value >>`.
    fn dictionary(&mut self) -> Result<Json, Error> {
        self.at += 2;
        let mut pairs: Vec<Json> = Vec::new();
        loop {
            self.skip_space();
            if self.bytes.get(self.at) == Some(&b'>') && self.bytes.get(self.at + 1) == Some(&b'>') {
                self.at += 2;
                return Ok(Json::Object({
                    let mut map = Map::new();
                    map.insert("entries".to_string(), Json::Array(pairs));
                    map
                }));
            }
            if self.at >= self.bytes.len() {
                return Err(Error::Malformed("a dictionary is never closed".to_string()));
            }
            let key = self.name()?;
            let Some(value) = self.token()? else {
                return Err(Error::Malformed("a dictionary's key has no value".to_string()));
            };
            let Token::Value(value) = value else {
                return Err(Error::Malformed("a dictionary's key has no value".to_string()));
            };
            pairs.push(Json::Array(vec![key, value]));
        }
    }

    /// An **array**: `[1 2 3]`.
    fn array(&mut self) -> Result<Json, Error> {
        self.at += 1;
        let mut values: Vec<Json> = Vec::new();
        loop {
            self.skip_space();
            match self.bytes.get(self.at) {
                Some(b']') => {
                    self.at += 1;
                    return Ok(Json::Array(values));
                }
                None => return Err(Error::Malformed("an array is never closed".to_string())),
                _ => {}
            }
            let Some(token) = self.token()? else {
                return Err(Error::Malformed("an array is never closed".to_string()));
            };
            match token {
                Token::Value(value) => values.push(value),
                Token::Operator(name) => {
                    return Err(Error::Malformed(format!("the operator `{name}` is inside an array")));
                }
            }
        }
    }

    /// A **number**, if the bytes are one.
    fn number(&mut self) -> Result<Option<Json>, Error> {
        let start = self.at;
        if matches!(self.bytes.get(self.at), Some(b'+') | Some(b'-')) {
            self.at += 1;
        }
        let mut seen_dot = false;
        let mut seen_digit = false;
        while let Some(byte) = self.bytes.get(self.at).copied() {
            match byte {
                b'0'..=b'9' => {
                    seen_digit = true;
                    self.at += 1;
                }
                b'.' if !seen_dot => {
                    seen_dot = true;
                    self.at += 1;
                }
                _ => break,
            }
        }
        if !seen_digit {
            self.at = start;
            return Ok(None);
        }
        let text = String::from_utf8_lossy(&self.bytes[start..self.at]).to_string();
        let Ok(number) = text.parse::<f64>() else {
            return Err(Error::Malformed(format!("the number `{text}` is not a number")));
        };
        if !number.is_finite() {
            return Err(Error::Malformed(format!("the number `{text}` is not finite")));
        }
        let Some(value) = craftkit_sdk::serde_json::Number::from_f64(number) else {
            return Err(Error::Malformed(format!("the number `{text}` is not finite")));
        };
        Ok(Some(Json::Number(value)))
    }

    /// A **keyword**: an operator.
    fn keyword(&mut self) -> Result<Token, Error> {
        let start = self.at;
        while let Some(byte) = self.bytes.get(self.at).copied() {
            if is_space(byte) || is_delimiter(byte) {
                break;
            }
            self.at += 1;
        }
        if start == self.at {
            return Err(Error::Malformed("the stream has a byte that says nothing".to_string()));
        }
        let name = String::from_utf8_lossy(&self.bytes[start..self.at]).to_string();
        if !OPERATORS.contains(&name.as_str()) {
            return Err(Error::Malformed(format!("the operator `{name}` is not one of the kit's")));
        }
        Ok(Token::Operator(name))
    }

    /// The bytes of an **inline image**.
    ///
    /// The tree's own rule: one whitespace byte separates `ID` from the
    /// data, and the data ends at whitespace + `EI` + (whitespace or a
    /// delimiter or the end of the stream). The `EI` is consumed with
    /// the data, so the reader never sees it as an operator of its own.
    fn inline_image(&mut self) -> Result<Vec<u8>, Error> {
        let begin = (self.at + 1).min(self.bytes.len());
        let mut at = begin;
        while at + 2 <= self.bytes.len() {
            let before = at.checked_sub(1).and_then(|at| self.bytes.get(at)).copied();
            let after = self.bytes.get(at + 2).copied();
            let closed = self.bytes[at] == b'E'
                && self.bytes.get(at + 1) == Some(&b'I')
                && before.is_none_or(is_space)
                && after.is_none_or(|byte| is_space(byte) || is_delimiter(byte));
            if closed {
                let end = if at > begin && before.is_some_and(is_space) { at - 1 } else { at };
                let image = self.bytes[begin..end].to_vec();
                self.at = at + 2;
                return Ok(image);
            }
            at += 1;
        }
        Err(Error::Malformed("the inline image is never closed".to_string()))
    }
}

/// Whether a byte is **space**.
fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | 0x0c | 0x00)
}

/// Whether a byte **ends** a token.
fn is_delimiter(byte: u8) -> bool {
    matches!(byte, b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%')
}

/// The **value** of a hex digit.
fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
