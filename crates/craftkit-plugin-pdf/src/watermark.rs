//! The **watermark** of a page: a text drawn over it.
//!
//! The tree's own: a watermark is a piece of page content that is
//! marked as a **pagination artefact**, drawn with the tree's
//! Helvetica, turned about the centre of the page.
//!
//! # The rules
//!
//! - The watermark has **text**, and text that is empty is refused.
//! - The **size** of the text is the size that is asked for; when it
//!   is not asked for, it is the tree's own: half the diagonal of the
//!   page, over the width of the widest line, and never less than six
//!   points and never more than three hundred.
//! - The **opacity** is between zero and one, and an opacity outside
//!   that is **refused**.
//! - The **rotation** is in degrees, and the tree's is forty-five.
//! - The watermark is drawn **behind** the content of the page or
//!   **on top** of it, and the tree's default is on top.
//! - The text is measured with the tree's own **Helvetica** widths, so
//!   that each line is centred and the lines are stacked as a block.
//!
//! # What it does not settle
//!
//! - The **picture** watermark of the tree is not here: this is a text
//!   watermark, and the picture is a file and an image.
//! - The **background** of the tree is not here: that is a colour or a
//!   picture behind the whole page.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, Document, EngineError, NodeRef, Number, Outcome, Session, Text};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.page.watermark";

/// The **width** of a text in the tree's Helvetica.
///
/// The tree's own table: the widths of the characters, in the units of
/// a thousandth of an em.
pub(crate) fn helvetica_width(text: &str, size: f64) -> f64 {
    let units: f64 = text
        .chars()
        .map(|character| match character {
            ' ' | 'i' | 'j' | 'l' | '\'' | '!' | '|' | '.' | ',' | ':' | ';' | 'I' => 260.0,
            'f' | 't' | 'r' | '(' | ')' | '[' | ']' | '/' | '-' | '"' => 333.0,
            'm' => 833.0,
            'w' => 722.0,
            'M' => 833.0,
            'W' => 944.0,
            'J' | 'c' | 'k' | 's' | 'v' | 'x' | 'y' | 'z' => 500.0,
            '0'..='9' | 'a'..='z' | '$' | '#' | '?' | '_' => 556.0,
            'A'..='Z' => 680.0,
            '@' => 1015.0,
            _ if character.is_whitespace() => 260.0,
            _ => 584.0,
        })
        .sum();
    units * size / 1000.0
}

/// A **number** as PostScript writes it: the tree's own rule of at most
/// four decimals.
pub(crate) fn number(value: f64) -> String {
    let text = format!("{value:.4}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text.is_empty() { "0".to_string() } else { text.to_string() }
}

/// The **text** of a line, as PostScript writes it: a literal string.
pub(crate) fn literal(text: &str) -> String {
    let mut out = String::from("(");
    for character in text.chars() {
        match character {
            '(' | ')' | '\\' => {
                out.push('\\');
                out.push(character);
            }
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            other => out.push(other),
        }
    }
    out.push(')');
    out
}

/// The **bytes** of a text in WinAnsiEncoding: the tree's own
/// `win_ansi`.
///
/// The characters that WinAnsi has are the ones of ISO 32000-2
/// Annex D, and a character that it does not have becomes a question
/// mark.
pub(crate) fn win_ansi(text: &str) -> Vec<u8> {
    text.chars()
        .map(|character| match character {
            '\u{20}'..='\u{7e}' => character as u8,
            '\u{a0}'..='\u{ff}' => character as u32 as u8,
            '\u{20ac}' => 0x80,
            '\u{201a}' => 0x82,
            '\u{201e}' => 0x84,
            '\u{2026}' => 0x85,
            '\u{2018}' => 0x91,
            '\u{2019}' => 0x92,
            '\u{201c}' => 0x93,
            '\u{201d}' => 0x94,
            '\u{2022}' => 0x95,
            '\u{2013}' => 0x96,
            '\u{2014}' => 0x97,
            '\u{2122}' => 0x99,
            '\t' => b' ',
            _ => b'?',
        })
        .collect()
}

/// The **bytes** of a line, as PostScript writes it: a literal string.
pub(crate) fn literal_bytes(bytes: &[u8]) -> String {
    let mut out = String::from("(");
    for byte in bytes {
        match byte {
            b'(' | b')' | b'\\' => {
                out.push('\\');
                out.push(*byte as char);
            }
            b'\r' => out.push_str("\\r"),
            b'\n' => out.push_str("\\n"),
            other => out.push(*other as char),
        }
    }
    out.push(')');
    out
}

/// The **box** of the page: the media box, or the crop box when the
/// page has one.
pub(crate) fn box_of(page: &craftkit_sdk::Node) -> [f64; 4] {
    for key in ["CropBox", "MediaBox"] {
        if let Some((_, value)) = crate::entries_of(&page.data).into_iter().find(|(held, _)| held == key)
            && let Some(rect) = crate::boxes::rect_of(&value)
        {
            return rect;
        }
    }
    [0.0, 0.0, 595.0, 842.0]
}

/// The **size** of a watermark that is given none: the tree's own rule
/// — half the diagonal of the page, over the width of the widest line,
/// and never less than six points and never more than three hundred.
fn size_of(text: &str, width: f64, height: f64) -> f64 {
    let widest = text.lines().map(|line| helvetica_width(line, 1.0)).fold(0.0, f64::max).max(0.01);
    ((width * width + height * height).sqrt() * 0.5 / widest).clamp(6.0, 300.0)
}

/// The **content** of the watermark: the tree's own operators.
fn content_of(
    text: &str,
    size: f64,
    rotation: f64,
    colour: [f64; 3],
    opacity: f64,
    width: f64,
    height: f64,
) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let size = if size > 0.0 { size } else { size_of(text, width, height) };
    let (sine, cosine) = rotation.to_radians().sin_cos();
    let (centre_x, centre_y) = (width / 2.0, height / 2.0);
    let mut out = String::new();
    // The tree's own marking: the watermark is a **pagination
    // artefact**, and the page's content is not disturbed by it.
    out.push_str("q\n/Artifact <</Type /Pagination /Subtype /Watermark /PCMark /Watermark>> BDC\n");
    out.push_str(&format!("/PCGS{} gs\n", (opacity * 100.0).round() as i64));
    out.push_str(&format!(
        "{} {} {} {} {} {} cm\n",
        number(cosine),
        number(sine),
        number(-sine),
        number(cosine),
        number(centre_x),
        number(centre_y)
    ));
    out.push_str(&format!(
        "BT\n/PCHelv {} Tf\n{} {} {} rg\n",
        number(size),
        number(colour[0]),
        number(colour[1]),
        number(colour[2])
    ));
    let line_height = size * 1.15;
    let total = line_height * (lines.len() as f64 - 1.0);
    for (index, line) in lines.iter().enumerate() {
        let text_width = helvetica_width(line, size);
        let x = -text_width / 2.0;
        let y = total / 2.0 - index as f64 * line_height - size * 0.35;
        out.push_str(&format!("1 0 0 1 {} {} Tm {} Tj\n", number(x), number(y), literal(line)));
    }
    out.push_str("ET\nEMC\nQ\n");
    out
}

/// The **resources** of the page: the tree's Helvetica and the graphics
/// state of the opacity.
fn resources_of(opacity: f64) -> Json {
    json!({
        "Font": {
            "entries": [[
                "PCHelv",
                { "entries": [
                    ["Type", "Font"],
                    ["Subtype", "Type1"],
                    ["BaseFont", "Helvetica"],
                    ["Encoding", "WinAnsiEncoding"],
                ] },
            ]],
        },
        "ExtGState": {
            "entries": [[
                format!("PCGS{}", (opacity * 100.0).round() as i64),
                { "entries": [
                    ["Type", "ExtGState"],
                    ["CA", opacity],
                    ["ca", opacity],
                ] },
            ]],
        },
    })
}

/// The **streams** of the page's content: the nodes of them, in the
/// order the content is in.
fn content_streams(document: &Document, page: &craftkit_sdk::Node) -> Vec<u64> {
    let entries = crate::entries_of(&page.data);
    let Some((_, value)) = entries.into_iter().find(|(key, _)| key == "Contents") else {
        return Vec::new();
    };
    let mut out: Vec<u64> = Vec::new();
    let mut push = |value: Json| {
        // The **numbers** of the file are numbers, and a reference is
        // a pair of them.
        let Some(number) = value
            .get("ref")
            .and_then(|reference| reference.get(0))
            .and_then(Json::as_f64)
            .filter(|number| number.is_finite() && number.fract() == 0.0 && *number >= 0.0)
            .map(|number| number as u64)
        else {
            return;
        };
        if let Some(node) = document
            .walk()
            .find(|node| crate::the_number_of(node) == Some(number) && node.kind == "pdf.stream")
        {
            out.push(node.id);
        }
    };
    match value {
        Json::Array(list) => {
            for item in list {
                push(item.clone());
            }
        }
        other => push(other.clone()),
    }
    out
}

/// **Draw** a watermark on a page: a text, turned about the centre of
/// the page, in the tree's own grey.
#[derive(Command)]
#[command(id = "pdf.page.watermark", area = "pdf", layer = "pdf", icon = "pencil", mutates, undoable)]
pub struct AddWatermark {
    /// The page the watermark is on.
    pub page: NodeRef,
    /// The text of the watermark.
    pub text: Text,
    /// The size of the text: the tree's own size is used when it is
    /// not given.
    pub size: Option<Number>,
    /// The rotation of the text, in degrees: forty-five is the tree's.
    pub rotation: Option<Number>,
    /// The opacity of the watermark, between zero and one: a half is
    /// the tree's.
    pub opacity: Option<Number>,
    /// The colour of the text: the tree's grey.
    pub colour: Option<Text>,
    /// The watermark is drawn **behind** the content of the page.
    pub behind: Option<Bool>,
}

impl AddWatermark {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let text = self.text.0.clone();
        if text.trim().is_empty() {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "the watermark has text".to_string(),
            });
        }
        let size = self.size.map(|size| size.0).unwrap_or(0.0);
        if !size.is_finite() || size < 0.0 {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "the size of the watermark is a number that is not negative".to_string(),
            });
        }
        let rotation = self.rotation.map(|rotation| rotation.0).unwrap_or(45.0);
        if !rotation.is_finite() {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "the rotation of the watermark is a number".to_string(),
            });
        }
        let opacity = self.opacity.map(|opacity| opacity.0).unwrap_or(0.5);
        if !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "the opacity of the watermark is between zero and one".to_string(),
            });
        }
        let colour = match &self.colour {
            Some(colour) => {
                let colour = colour.0.trim();
                let parsed = craftkit_sdk::document::Colour::from_hex(colour)
                    .map_err(|message| EngineError::Failed { id: COMMAND.to_string(), message })?;
                let [r, g, b, _] = parsed.0;
                [f64::from(r), f64::from(g), f64::from(b)]
            }
            None => [0.6, 0.6, 0.6],
        };
        let behind = self.behind.map(|behind| behind.0).unwrap_or(false);
        // The **page**.
        let Some(page) = session.document().node(self.page.0) else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("there is no node `{}`", self.page.0),
            });
        };
        let box_ = box_of(page);
        let (width, height) = (box_[2] - box_[0], box_[3] - box_[1]);
        // The **size**: the one that is asked for, and the tree's own
        // when it is not asked for.
        let size = if size > 0.0 { size } else { size_of(&text, width, height) };
        // The **content** of the watermark.
        let content = content_of(&text, size, rotation, colour, opacity, width, height);
        // The **page's content**: the watermark goes in front of it or
        // behind it, and the page's content is a list of streams or one
        // stream.
        let streams = content_streams(session.document(), page);
        let Some(stream) = (if behind { streams.first() } else { streams.last() }) else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "the page has no content to draw the watermark on".to_string(),
            });
        };
        let node = session.document().node(*stream).ok_or_else(|| EngineError::Failed {
            id: COMMAND.to_string(),
            message: format!("there is no node `{stream}`"),
        })?;
        let Some(path) = node.stream.clone() else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("the stream `{stream}` has no file"),
            });
        };
        let limits = crate::filters::Limits::default();
        let bytes = craftkit_sdk::files::read_bounded(std::path::Path::new(&path), limits.max_input as u64)
            .map_err(|message| EngineError::Failed {
            id: COMMAND.to_string(),
            message: format!("the page's content cannot be read: {message}"),
        })?;
        let mut written = Vec::with_capacity(bytes.len() + content.len());
        if behind {
            written.extend_from_slice(content.as_bytes());
            written.extend_from_slice(&bytes);
        } else {
            written.extend_from_slice(&bytes);
            written.extend_from_slice(content.as_bytes());
        }
        std::fs::write(&path, &written).map_err(|error| EngineError::Failed {
            id: COMMAND.to_string(),
            message: format!("the page's content cannot be written: {error}"),
        })?;
        // The page's **resources**: the tree's Helvetica and the
        // graphics state of the opacity.
        let resources = resources_of(opacity);
        if let Some(page) = session.document_mut()?.node_mut(self.page.0) {
            let mut entries = crate::entries_of(&page.data);
            let merged = match entries.iter().find(|(key, _)| key == "Resources") {
                Some((_, value)) => {
                    let mut held = crate::entries_of(value);
                    for (key, value) in crate::entries_of(&resources) {
                        match held.iter_mut().find(|(held_key, _)| *held_key == key) {
                            Some(entry) => entry.1 = value,
                            None => held.push((key, value)),
                        }
                    }
                    json!({ "entries": held.iter().map(|(key, value)| json!([key, value])).collect::<Vec<Json>>() })
                }
                None => resources,
            };
            entries.retain(|(key, _)| key != "Resources");
            entries.push(("Resources".to_string(), merged));
            page.data = json!({
                "object": page.data.get("object").cloned().unwrap_or(Json::Null),
                "entries": entries.iter().map(|(key, value)| json!([key, value])).collect::<Vec<Json>>(),
            });
        }
        Ok(Outcome::changed("the watermark is drawn on the page".to_string()).with_data(json!({
            "page": self.page.0,
            "text": text,
            "size": size,
            "rotation": rotation,
            "opacity": opacity,
            "colour": colour,
            "behind": behind,
            "bytes": content.len(),
        })))
    }
}
