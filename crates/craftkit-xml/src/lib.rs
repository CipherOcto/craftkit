//! `craftkit-xml` — the **XML reader**: a part of a file, read as
//! elements.
//!
//! # What this is
//!
//! An XML part is a tree of **elements**: each of them a name, some
//! attributes, some children and some text. This crate reads that:
//! it gives back the tree, and it gives back the parts of it that a
//! caller asks for — a child by name, an attribute by name, the text
//! of an element.
//!
//! # Why it is a library and not a plugin
//!
//! Because an XML part is an XML part: the reader reads bytes and
//! gives back elements, and it knows nothing about a document, a
//! canvas or the kit. A workbook reader (§904) is what puts a
//! workbook into a document, and it reads the parts of a package with
//! this.
//!
//! # The file is hostile
//!
//! An XML part came from somewhere else, so it may be a bomb: a part
//! of endless empty elements fits in a small file and takes gigabytes
//! of tree. The reader **refuses** what it cannot make sense of, and
//! it refuses **within limits**: the tree may be **256** deep and hold
//! **1 000 000** elements, and a part that says more is refused.
//!
//! # What this does not do
//!
//! - It does not **write** XML. A writer is a different program.
//! - It does not **validate** the XML: a part that does not match the
//!   schema is the caller's business and not this reader's.
//! - It does not keep the **prefixes** of the names: the name of an
//!   element is its local name, and the namespace of a part is the
//!   part's own.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

/// The deepest an element may be.
pub const MAX_DEPTH: usize = 256;

/// The most elements one part may hold.
pub const MAX_XML_NODES: usize = 1_000_000;

/// Why an XML part could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The part is not XML, or is broken in a way that cannot be read.
    Malformed(String),
    /// The part asks for more than the reader allows.
    TooLarge(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Malformed(message) => write!(f, "malformed xml: {message}"),
            Error::TooLarge(message) => write!(f, "limit exceeded: {message}"),
        }
    }
}

impl std::error::Error for Error {}

/// An **element**: a name, some attributes, some children and some
/// text.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct El {
    /// The **local** name of the element: the prefix, if it has one, is
    /// not kept.
    pub name: String,
    /// The attributes, by their local names.
    pub attrs: Vec<(String, String)>,
    /// The children of the element.
    pub children: Vec<El>,
    /// The **direct** text of the element: the text that is not in a
    /// child.
    pub text: String,
}

impl El {
    /// The value of an attribute.
    pub fn attr(&self, key: &str) -> Option<&str> {
        self.attrs.iter().find(|(name, _)| name == key).map(|(_, value)| value.as_str())
    }

    /// The value of an attribute, as a number.
    pub fn attr_u32(&self, key: &str) -> Option<u32> {
        self.attr(key).and_then(|value| value.trim().parse::<u32>().ok())
    }

    /// The value of an attribute, as a number.
    pub fn attr_i64(&self, key: &str) -> Option<i64> {
        self.attr(key).and_then(|value| value.trim().parse::<i64>().ok())
    }

    /// The value of an attribute, as a number.
    ///
    /// A number that is not finite is not a number.
    pub fn attr_f64(&self, key: &str) -> Option<f64> {
        self.attr(key).and_then(|value| value.trim().parse::<f64>().ok()).filter(|value| value.is_finite())
    }

    /// The value of an attribute, as a **boolean**: `1`, `true`, `on`
    /// and `t` are true, and `0`, `false`, `off` and `f` are false.
    pub fn attr_bool(&self, key: &str) -> Option<bool> {
        match self.attr(key)?.trim() {
            "1" | "true" | "on" | "t" => Some(true),
            "0" | "false" | "off" | "f" => Some(false),
            _ => None,
        }
    }

    /// The value of an attribute, as a boolean, with a default.
    pub fn flag(&self, key: &str, default: bool) -> bool {
        self.attr_bool(key).unwrap_or(default)
    }

    /// The **first** child of that name.
    pub fn child(&self, name: &str) -> Option<&El> {
        self.children.iter().find(|child| child.name == name)
    }

    /// The children of that name.
    pub fn kids<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a El> + 'a {
        self.children.iter().filter(move |child| child.name == name)
    }

    /// Follows a **path** of child names.
    pub fn path(&self, names: &[&str]) -> Option<&El> {
        let mut element = self;
        for name in names {
            element = element.child(name)?;
        }
        Some(element)
    }

    /// The `val` attribute of a child — the shape a spreadsheet part
    /// uses most.
    pub fn child_val(&self, name: &str) -> Option<&str> {
        self.child(name).and_then(|child| child.attr("val"))
    }
}

/// Read an XML part, and give back its **root**.
pub fn parse(bytes: &[u8]) -> Result<El, Error> {
    let text = to_utf8(bytes);
    let mut parser = Parser { text: &text, at: 0, stack: Vec::new(), root: None, kept: 0 };
    parser.run()?;
    parser.root.ok_or_else(|| Error::Malformed("the part holds no element".to_string()))
}

/// The **reader** of a part.
struct Parser<'a> {
    /// The part, as text.
    text: &'a str,
    /// Where the reader is.
    at: usize,
    /// The elements that are open.
    stack: Vec<El>,
    /// The root of the part, when it has been read.
    root: Option<El>,
    /// How many elements the tree holds.
    kept: usize,
}

impl Parser<'_> {
    /// Read the part, to its end.
    fn run(&mut self) -> Result<(), Error> {
        while self.at < self.text.len() {
            if self.text[self.at..].starts_with('<') {
                self.markup()?;
            } else {
                let (text, taken) = self.text(self.at)?;
                self.at += taken;
                if let Some(element) = self.stack.last_mut() {
                    element.text.push_str(&text);
                }
            }
        }
        // An element that was never closed is closed here: a part that
        // ends in the middle of an element is read as far as it goes.
        while let Some(element) = self.stack.pop() {
            self.finish(element)?;
        }
        Ok(())
    }

    /// Read a piece of **markup**: a comment, a declaration, a CDATA
    /// section, an end tag or an element.
    fn markup(&mut self) -> Result<(), Error> {
        let rest = &self.text[self.at..];
        if let Some(body) = rest.strip_prefix("<!--") {
            let end =
                body.find("-->").ok_or_else(|| Error::Malformed("a comment is not closed".to_string()))?;
            self.at += 4 + end + 3;
            return Ok(());
        }
        if let Some(body) = rest.strip_prefix("<![CDATA[") {
            let end = body
                .find("]]>")
                .ok_or_else(|| Error::Malformed("a CDATA section is not closed".to_string()))?;
            let text = &body[..end];
            if let Some(element) = self.stack.last_mut() {
                element.text.push_str(text);
            }
            self.at += 9 + end + 3;
            return Ok(());
        }
        if let Some(body) = rest.strip_prefix("<?") {
            let end =
                body.find("?>").ok_or_else(|| Error::Malformed("a declaration is not closed".to_string()))?;
            self.at += 2 + end + 2;
            return Ok(());
        }
        if let Some(body) = rest.strip_prefix("</") {
            let end =
                body.find('>').ok_or_else(|| Error::Malformed("an end tag is not closed".to_string()))?;
            let name = local_name(body[..end].trim());
            self.at += 2 + end + 1;
            // An end tag **closes** the element it names: the element is
            // finished, and so is every element that was still open
            // inside it.
            let Some(depth) = self.stack.iter().rposition(|element| element.name == name) else {
                // An end tag that closes nothing is ignored: the tree's
                // own rule, and a part written by a program that closed
                // an element twice is still worth reading.
                return Ok(());
            };
            while self.stack.len() > depth {
                let element = self
                    .stack
                    .pop()
                    .ok_or_else(|| Error::Malformed("an element is not closed".to_string()))?;
                self.finish(element)?;
            }
            return Ok(());
        }
        if rest.starts_with("<!") {
            // A **doctype**: it may hold a subset in brackets.
            let mut at = 2;
            let mut depth = 0usize;
            while at < rest.len() {
                match rest.as_bytes()[at] {
                    b'[' => depth += 1,
                    b']' => depth = depth.saturating_sub(1),
                    b'>' if depth == 0 => break,
                    _ => {}
                }
                at += 1;
            }
            let at = at + 1;
            self.at += at;
            return Ok(());
        }
        self.element()
    }

    /// Read an **element**: its name, its attributes, and what it
    /// holds.
    fn element(&mut self) -> Result<(), Error> {
        let rest = &self.text[self.at + 1..];
        let name_end = rest
            .find(|character: char| character.is_whitespace() || character == '>' || character == '/')
            .unwrap_or(rest.len());
        let name = local_name(&rest[..name_end]);
        if name.is_empty() {
            return Err(Error::Malformed("an element has no name".to_string()));
        }
        let mut at = self.at + 1 + name_end;
        let mut attrs: Vec<(String, String)> = Vec::new();
        let mut empty = false;
        loop {
            // The **space** before the next thing is not a thing.
            let rest = &self.text[at..];
            at += rest.len() - rest.trim_start().len();
            let rest = &self.text[at..];
            if rest.starts_with("/>") {
                at += 2;
                empty = true;
                break;
            }
            if rest.starts_with('>') {
                at += 1;
                break;
            }
            if rest.is_empty() {
                return Err(Error::Malformed("an element is not closed".to_string()));
            }
            // An **attribute**: a name, an equals sign and a value.
            let name_end = rest
                .find(|character: char| character.is_whitespace() || character == '=' || character == '>')
                .unwrap_or(rest.len());
            let attribute = local_name(&rest[..name_end]);
            if attribute.is_empty() {
                return Err(Error::Malformed("an attribute has no name".to_string()));
            }
            at += name_end;
            let rest = &self.text[at..];
            at += rest.len() - rest.trim_start().len();
            if !self.text[at..].starts_with('=') {
                return Err(Error::Malformed(format!("the attribute {attribute} has no value")));
            }
            at += 1;
            let rest = &self.text[at..];
            at += rest.len() - rest.trim_start().len();
            let rest = &self.text[at..];
            let Some(quote) = rest.chars().next() else {
                return Err(Error::Malformed(format!("the attribute {attribute} has no value")));
            };
            if quote != '"' && quote != '\'' {
                return Err(Error::Malformed(format!("the attribute {attribute} is not quoted")));
            }
            let value_start = at + 1;
            let Some(end) = self.text[value_start..].find(quote) else {
                return Err(Error::Malformed(format!("the attribute {attribute} is not closed")));
            };
            let value = decode(&self.text[value_start..value_start + end]);
            at = value_start + end + 1;
            attrs.push((attribute, value));
        }
        self.at = at;
        let element = El { name, attrs, children: Vec::new(), text: String::new() };
        if empty {
            self.finish(element)?;
        } else {
            if self.stack.len() >= MAX_DEPTH {
                return Err(Error::TooLarge(format!("the part is deeper than {MAX_DEPTH}")));
            }
            self.stack.push(element);
        }
        Ok(())
    }

    /// An element is **finished**: it goes to its parent, or it is the
    /// root.
    fn finish(&mut self, element: El) -> Result<(), Error> {
        match self.stack.last_mut() {
            Some(parent) => {
                self.kept += 1;
                if self.kept > MAX_XML_NODES {
                    return Err(Error::TooLarge(format!(
                        "the part holds more than {MAX_XML_NODES} elements"
                    )));
                }
                parent.children.push(element);
                Ok(())
            }
            None => {
                if self.root.is_none() {
                    self.root = Some(element);
                    Ok(())
                } else {
                    // A second root: the part is not one tree, and the
                    // tree that came first is the one that is read.
                    Ok(())
                }
            }
        }
    }

    /// The **text** from here to the next markup.
    fn text(&self, at: usize) -> Result<(String, usize), Error> {
        let rest = &self.text[at..];
        let end = rest.find('<').unwrap_or(rest.len());
        Ok((decode(&rest[..end]), end))
    }
}

/// The **local** name of a name: the part after the colon.
fn local_name(name: &str) -> String {
    match name.rsplit_once(':') {
        Some((_, local)) => local.to_string(),
        None => name.to_string(),
    }
}

/// The **text** of a piece of XML, with its entities read.
fn decode(text: &str) -> String {
    if !text.contains('&') {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at + 1..];
        let Some(end) = rest.find(';') else {
            // A `&` that is not an entity is kept as it is: the tree's
            // own rule, and a part that has one is still worth reading.
            out.push('&');
            break;
        };
        let entity = &rest[..end];
        rest = &rest[end + 1..];
        match entity {
            "amp" => out.push('&'),
            "lt" => out.push('<'),
            "gt" => out.push('>'),
            "quot" => out.push('"'),
            "apos" => out.push('\''),
            _ => {
                let code = if let Some(hex) = entity.strip_prefix("#x").or_else(|| entity.strip_prefix("#X"))
                {
                    u32::from_str_radix(hex, 16).ok()
                } else if let Some(decimal) = entity.strip_prefix('#') {
                    decimal.parse::<u32>().ok()
                } else {
                    None
                };
                match code.and_then(char::from_u32) {
                    Some(character) => out.push(character),
                    None => {
                        out.push('&');
                        out.push_str(entity);
                        out.push(';');
                    }
                }
            }
        }
    }
    out.push_str(rest);
    out
}

/// The part as **text**: UTF-8, or UTF-16 in either order, with or
/// without a byte-order mark.
fn to_utf8(bytes: &[u8]) -> String {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(rest).into_owned();
    }
    let utf16 = |little: bool, bytes: &[u8]| -> String {
        let units = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| if little { u16::from_le_bytes(*pair) } else { u16::from_be_bytes(*pair) });
        char::decode_utf16(units).map(|result| result.unwrap_or('\u{FFFD}')).collect()
    };
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        return utf16(true, rest);
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        return utf16(false, rest);
    }
    String::from_utf8_lossy(bytes).into_owned()
}
