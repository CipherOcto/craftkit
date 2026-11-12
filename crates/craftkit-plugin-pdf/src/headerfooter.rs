//! The **header** and the **footer** of a page: the text that is
//! written at the top of the page and at the bottom of it.
//!
//! The tree's own: three slots at the top and three at the bottom —
//! the left, the centre and the right — and a text that may hold
//! **tokens**: the number of the page, the number of the pages, the
//! date and the **Bates number**.
//!
//! # The rules
//!
//! - The **text** of a header or a footer may not be empty, and a
//!   header or a footer with no text at all is **refused**.
//! - The **size** of the text is the tree's own: eight points, and
//!   never more than two hundred.
//! - The **margins** are the distances of the text from the edges of
//!   the page, and the tree's own are thirty-six points at the top and
//!   at the bottom and seventy-two at the left and at the right.
//! - The **slots**: the left text is at the left margin, the centre
//!   text is in the middle of the page and the right text is at the
//!   right margin — the tree's own.
//! - The **header** hangs below the top margin and the **footer**
//!   stands on the bottom margin.
//! - The **tokens** are the tree's own: `<<1>>` the number of the page,
//!   `<<n>>` the number of the pages, `<<1 of n>>`, `<<1/n>>`,
//!   `<<Page 1>>`, `<<Page 1 of n>>`, the dates `<<m/d/yy>>`,
//!   `<<m/d/yyyy>>`, `<<mm/dd/yy>>`, `<<mm/dd/yyyy>>`, `<<d/m/yy>>`,
//!   `<<d/m/yyyy>>`, `<<dd/mm/yyyy>>`, `<<yyyy-mm-dd>>`,
//!   `<<mmmm d, yyyy>>` and the **Bates number**
//!   `<<Bates Number#digits#start#prefix#suffix>>`.
//!
//! # What it does not settle
//!
//! - The **date** of the tree is the clock of the machine; here the
//!   caller says the day, because a file that says the day it was
//!   written is not the same file every time.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, Document, EngineError, NodeRef, Number, Outcome, Session, Text};

use crate::watermark::{box_of, helvetica_width, literal, number};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.page.header";

/// The **months**, as the tree writes them.
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// The **date** a file is stamped with: the year, the month and the day.
fn date_of(date: Option<&Json>) -> (i64, u32, u32) {
    let Some(date) = date else { return (1970, 1, 1) };
    let Some(list) = date.as_array() else { return (1970, 1, 1) };
    let number = |index: usize| list.get(index).and_then(Json::as_f64).map(|value| value as i64);
    let year = number(0).unwrap_or(1970);
    let month = number(1).unwrap_or(1).clamp(1, 12) as u32;
    let day = number(2).unwrap_or(1).max(1) as u32;
    (year, month, day)
}

/// The **Bates start** in a template, if it has one.
fn bates_start(template: &str) -> Option<u64> {
    let index = template.find("<<Bates Number")?;
    let rest = &template[index + 2..];
    let token = &rest[..rest.find(">>")?];
    token.split('#').nth(2).and_then(|start| start.parse().ok()).or(Some(1))
}

/// Expand the **tokens** of a template for the page `number` of `count`.
///
/// The tree's own tokens, and the tree's own spelling of them.
pub fn expand(template: &str, page_number: u32, count: usize, bates: u64, date: (i64, u32, u32)) -> String {
    let (year, month, day) = date;
    let mut out = String::new();
    let mut rest = template;
    while let Some(start) = rest.find("<<") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find(">>") else {
            out.push_str(&rest[start..]);
            return out;
        };
        let token = &after[..end];
        let page = page_number;
        let value = match token {
            "1" => page.to_string(),
            "n" => count.to_string(),
            "1 of n" => format!("{page} of {count}"),
            "1/n" => format!("{page}/{count}"),
            "Page 1" => format!("Page {page}"),
            "Page 1 of n" => format!("Page {page} of {count}"),
            "m/d" => format!("{month}/{day}"),
            "m/d/yy" => format!("{month}/{day}/{:02}", year.rem_euclid(100)),
            "m/d/yyyy" => format!("{month}/{day}/{year}"),
            "mm/dd/yy" => format!("{month:02}/{day:02}/{:02}", year.rem_euclid(100)),
            "mm/dd/yyyy" => format!("{month:02}/{day:02}/{year}"),
            "d/m/yy" => format!("{day}/{month}/{:02}", year.rem_euclid(100)),
            "d/m/yyyy" => format!("{day}/{month}/{year}"),
            "dd/mm/yyyy" => format!("{day:02}/{month:02}/{year}"),
            "yyyy-mm-dd" => format!("{year}-{month:02}-{day:02}"),
            "mmmm d, yyyy" => format!("{} {day}, {year}", MONTHS[(month.clamp(1, 12) - 1) as usize]),
            other if other.starts_with("Bates Number") => {
                // **Bates Number#digits#start#prefix#suffix**.
                let parts: Vec<&str> = other.split('#').collect();
                let digits =
                    parts.get(1).and_then(|digits| digits.parse::<usize>().ok()).unwrap_or(6).clamp(1, 15);
                format!("{}{:0digits$}{}", parts.get(3).unwrap_or(&""), bates, parts.get(4).unwrap_or(&""))
            }
            other => format!("<<{other}>>"),
        };
        out.push_str(&value);
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    out
}

/// The **slots** of a header or a footer: the left, the centre and the
/// right.
struct Slots {
    left: String,
    centre: String,
    right: String,
}

impl Slots {
    /// The **texts** of the slots, in the order of the tree.
    fn texts(&self) -> [&str; 3] {
        [&self.left, &self.centre, &self.right]
    }
}

/// The **settings** of a header or a footer.
struct Settings {
    size: f64,
    colour: [f64; 3],
    underline: bool,
    margins: [f64; 4],
    start_number: u32,
    date: (i64, u32, u32),
}

/// The **content** of a header or a footer: the tree's own operators.
///
/// The **header** hangs below the top margin and the **footer** stands
/// on the bottom margin, and the three slots are at the left, in the
/// middle and at the right.
fn content_of(
    slots: &Slots,
    settings: &Settings,
    header: bool,
    page_number: u32,
    count: usize,
    bates: u64,
    page_size: (f64, f64),
) -> String {
    let (width, height) = page_size;
    let mut out = String::new();
    out.push_str("q\n/Artifact <</Type /Pagination /Subtype /Header /PCMark /HeaderFooter>> BDC\n");
    out.push_str(&format!(
        "BT\n/PCHelv {} Tf\n{} {} {} rg\n",
        number(settings.size),
        number(settings.colour[0]),
        number(settings.colour[1]),
        number(settings.colour[2])
    ));
    let mut underlines = String::new();
    let [top, bottom, left, right] = settings.margins;
    for (slot, template) in slots.texts().into_iter().enumerate() {
        if template.trim().is_empty() {
            continue;
        }
        let lines: Vec<String> =
            expand(template, page_number, count, bates, settings.date).lines().map(str::to_string).collect();
        let line_height = settings.size * 1.2;
        for (index, line) in lines.iter().enumerate() {
            let text_width = helvetica_width(line, settings.size);
            let x = match slot % 3 {
                0 => left,
                1 => (width - text_width) / 2.0,
                _ => width - right - text_width,
            };
            let y = if header {
                height - top - settings.size * 0.8 - index as f64 * line_height
            } else {
                bottom + (lines.len() - 1 - index) as f64 * line_height
            };
            out.push_str(&format!("1 0 0 1 {} {} Tm {} Tj\n", number(x), number(y), literal(line)));
            if settings.underline {
                underlines.push_str(&format!(
                    "{} {} {} {} re f\n",
                    number(x),
                    number(y - settings.size * 0.15),
                    number(text_width),
                    number((settings.size * 0.06).max(0.4))
                ));
            }
        }
    }
    out.push_str("ET\n");
    if !underlines.is_empty() {
        out.push_str(&format!(
            "{} {} {} rg\n",
            number(settings.colour[0]),
            number(settings.colour[1]),
            number(settings.colour[2])
        ));
        out.push_str(&underlines);
    }
    out.push_str("EMC\nQ\n");
    out
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

/// The **pages** of the document: the nodes of them, in order.
fn page_nodes(document: &Document) -> Vec<u64> {
    document
        .walk()
        .filter(|node| {
            (node.kind == "pdf.object" || node.kind == "pdf.stream")
                && crate::entries_of(&node.data)
                    .iter()
                    .any(|(key, value)| key == "Type" && value.as_str() == Some("Page"))
        })
        .map(|node| node.id)
        .collect()
}

/// Draw a **header** or a **footer** on a page.
///
/// The tree's own: three slots, the tokens of the page number and the
/// date, and the tree's Helvetica.
fn draw(
    session: &mut Session,
    page: NodeRef,
    slots: Slots,
    settings: Settings,
    header: bool,
) -> Result<Outcome, EngineError> {
    if slots.texts().iter().all(|text| text.trim().is_empty()) {
        return Err(EngineError::Failed {
            id: COMMAND.to_string(),
            message: "the header or the footer has text".to_string(),
        });
    }
    if !settings.size.is_finite() || settings.size <= 0.0 || settings.size > 200.0 {
        return Err(EngineError::Failed {
            id: COMMAND.to_string(),
            message: "the size of the text is a number between zero and two hundred".to_string(),
        });
    }
    if !settings.margins.iter().all(|margin| margin.is_finite() && *margin >= 0.0) {
        return Err(EngineError::Failed {
            id: COMMAND.to_string(),
            message: "the margins are numbers that are not negative".to_string(),
        });
    }
    // The **page**.
    let Some(node) = session.document().node(page.0) else {
        return Err(EngineError::Failed {
            id: COMMAND.to_string(),
            message: format!("there is no node `{}`", page.0),
        });
    };
    let box_ = box_of(node);
    let (width, height) = (box_[2] - box_[0], box_[3] - box_[1]);
    // The **number** of the page and the number of the pages: the tree's
    // own rules — the number the reader sees, and every page of the
    // document.
    let pages = page_nodes(session.document());
    let index = pages.iter().position(|held| *held == page.0).unwrap_or(0);
    let count = pages.len();
    let page_number = settings.start_number + index as u32;
    // The **Bates number**: the start of the first template that has
    // one, and one more for each page.
    let bates = slots.texts().iter().find_map(|text| bates_start(text)).unwrap_or(1) + index as u64;
    let content = content_of(&slots, &settings, header, page_number, count, bates, (width, height));
    // The page's **content**: the header is drawn on top of it.
    let streams = content_streams(session.document(), node);
    let Some(stream) = streams.last() else {
        return Err(EngineError::Failed {
            id: COMMAND.to_string(),
            message: "the page has no content to draw on".to_string(),
        });
    };
    let stream_node = session.document().node(*stream).ok_or_else(|| EngineError::Failed {
        id: COMMAND.to_string(),
        message: format!("there is no node `{stream}`"),
    })?;
    let Some(path) = stream_node.stream.clone() else {
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
    let mut written = bytes.clone();
    written.extend_from_slice(content.as_bytes());
    std::fs::write(&path, &written).map_err(|error| EngineError::Failed {
        id: COMMAND.to_string(),
        message: format!("the page's content cannot be written: {error}"),
    })?;
    // The page's **resources**: the tree's Helvetica.
    if let Some(page_node) = session.document_mut()?.node_mut(page.0) {
        let mut entries = crate::entries_of(&page_node.data);
        let mut resources = entries
            .iter()
            .find(|(key, _)| key == "Resources")
            .map(|(_, value)| crate::entries_of(value))
            .unwrap_or_default();
        let mut fonts = resources
            .iter()
            .find(|(key, _)| key == "Font")
            .map(|(_, value)| crate::entries_of(value))
            .unwrap_or_default();
        fonts.retain(|(key, _)| key != "PCHelv");
        fonts.push((
            "PCHelv".to_string(),
            json!({ "entries": [
                ["Type", "Font"],
                ["Subtype", "Type1"],
                ["BaseFont", "Helvetica"],
                ["Encoding", "WinAnsiEncoding"],
            ] }),
        ));
        resources.retain(|(key, _)| key != "Font");
        resources.push((
            "Font".to_string(),
            json!({ "entries": fonts.iter().map(|(key, value)| json!([key, value])).collect::<Vec<Json>>() }),
        ));
        entries.retain(|(key, _)| key != "Resources");
        entries.push((
            "Resources".to_string(),
            json!({ "entries": resources.iter().map(|(key, value)| json!([key, value])).collect::<Vec<Json>>() }),
        ));
        page_node.data = json!({
            "object": page_node.data.get("object").cloned().unwrap_or(Json::Null),
            "entries": entries.iter().map(|(key, value)| json!([key, value])).collect::<Vec<Json>>(),
        });
    }
    Ok(Outcome::changed("the header or the footer is drawn on the page".to_string()).with_data(json!({
        "page": page.0,
        "number": page_number,
        "count": count,
        "bates": bates,
        "bytes": content.len(),
    })))
}

/// The **settings** of a header or a footer, as the caller gives them.
fn settings_of(
    size: Option<Number>,
    colour: Option<Text>,
    underline: Option<Bool>,
    margins: Option<Json>,
    start_number: Option<Number>,
    date: Option<Json>,
) -> Result<Settings, EngineError> {
    let size = size.map(|size| size.0).unwrap_or(8.0);
    let colour = match &colour {
        Some(colour) => {
            let parsed = craftkit_sdk::document::Colour::from_hex(colour.0.trim())
                .map_err(|message| EngineError::Failed { id: COMMAND.to_string(), message })?;
            let [r, g, b, _] = parsed.0;
            [f64::from(r), f64::from(g), f64::from(b)]
        }
        None => [0.0, 0.0, 0.0],
    };
    let underline = underline.map(|underline| underline.0).unwrap_or(false);
    let margins = match &margins {
        Some(margins) => {
            let Some(list) = margins.as_array() else {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "the margins are four numbers".to_string(),
                });
            };
            if list.len() != 4 {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "the margins are four numbers".to_string(),
                });
            }
            let mut out = [0.0; 4];
            for (index, margin) in list.iter().enumerate() {
                let Some(margin) = margin.as_f64() else {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "the margins are four numbers".to_string(),
                    });
                };
                out[index] = margin;
            }
            out
        }
        None => [36.0, 36.0, 72.0, 72.0],
    };
    let start_number = start_number.map(|number| number.0).unwrap_or(1.0);
    if !start_number.is_finite() || start_number < 0.0 || start_number.fract() != 0.0 {
        return Err(EngineError::Failed {
            id: COMMAND.to_string(),
            message: "the first number of the pages is a whole number".to_string(),
        });
    }
    Ok(Settings {
        size,
        colour,
        underline,
        margins,
        start_number: start_number as u32,
        date: date_of(date.as_ref()),
    })
}

commands! {
    /// **Draw** a header on a page: the text at the top of it.
    ///
    /// The tree's own: three slots, the tokens of the page number and the
    /// date, and the tree's Helvetica.
    pdf.page.header {
        area: "pdf",
        layer: "pdf",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The page the header is on.

            page: NodeRef,
            /// The text at the left of the header.

            left: Option<Text>,
            /// The text in the middle of the header.

            centre: Option<Text>,
            /// The text at the right of the header.

            right: Option<Text>,
            /// The size of the text: eight points is the tree's.

            size: Option<Number>,
            /// The colour of the text: black is the tree's.

            colour: Option<Text>,
            /// The text is **underlined**.

            underline: Option<Bool>,
            /// The margins: four numbers, the top, the bottom, the left and
            /// the right.

            margins: Option<Json>,
            /// The number the first page shows.

            start_number: Option<Number>,
            /// The date the header is stamped with: the year, the month and the
            /// day.

            date: Option<Json>,
        },
        run: {

            let slots = Slots {
                left: self.left.as_ref().map(|text| text.0.clone()).unwrap_or_default(),
                centre: self.centre.as_ref().map(|text| text.0.clone()).unwrap_or_default(),
                right: self.right.as_ref().map(|text| text.0.clone()).unwrap_or_default(),
            };
            let settings = settings_of(
                self.size,
                self.colour.clone(),
                self.underline,
                self.margins.clone(),
                self.start_number,
                self.date.clone(),
            )?;
            draw(session, self.page, slots, settings, true)

        }
    }
    /// **Draw** a footer on a page: the text at the bottom of it.
    ///
    /// The tree's own: three slots, the tokens of the page number and the
    /// date, and the tree's Helvetica.
    pdf.page.footer {
        area: "pdf",
        layer: "pdf",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The page the footer is on.

            page: NodeRef,
            /// The text at the left of the footer.

            left: Option<Text>,
            /// The text in the middle of the footer.

            centre: Option<Text>,
            /// The text at the right of the footer.

            right: Option<Text>,
            /// The size of the text: eight points is the tree's.

            size: Option<Number>,
            /// The colour of the text: black is the tree's.

            colour: Option<Text>,
            /// The text is **underlined**.

            underline: Option<Bool>,
            /// The margins: four numbers, the top, the bottom, the left and
            /// the right.

            margins: Option<Json>,
            /// The number the first page shows.

            start_number: Option<Number>,
            /// The date the footer is stamped with: the year, the month and the
            /// day.

            date: Option<Json>,
        },
        run: {

            let slots = Slots {
                left: self.left.as_ref().map(|text| text.0.clone()).unwrap_or_default(),
                centre: self.centre.as_ref().map(|text| text.0.clone()).unwrap_or_default(),
                right: self.right.as_ref().map(|text| text.0.clone()).unwrap_or_default(),
            };
            let settings = settings_of(
                self.size,
                self.colour.clone(),
                self.underline,
                self.margins.clone(),
                self.start_number,
                self.date.clone(),
            )?;
            draw(session, self.page, slots, settings, false)

        }
    }
}
