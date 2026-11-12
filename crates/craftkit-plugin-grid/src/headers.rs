//! The **header** and the **footer** of a print.
//!
//! A printed page may carry a header above it and a footer below it,
//! and each of them is a text with **codes** in it: the page number
//! (`&P`), the number of pages (`&N`), the name of the sheet (`&A`),
//! and the sections the text stands in — left (`&L`), centre (`&C`)
//! and right (`&R`).
//!
//! A sheet may print a **different** header on its first page, and a
//! **different** one on its even pages. Here that is not a pair of
//! flags: the kind of page that has a header of its own is the kind
//! that **has** one, and a kind that has none falls back on the
//! header of the odd pages.
//!
//! The rules:
//!
//! - a **code** that this tree does not print is refused: `&D` is the
//!   date of the machine and `&F` is the name of the file it was
//!   printed from, and a printed sheet is not about the machine;
//! - a text that is **too long** is refused;
//! - a **page kind** that is not `odd`, `even` or `first` is refused.
//!
//! # What the port found
//!
//! - **The tree keeps two flags** — different first page, different
//!   odd and even pages — beside the six texts, so an even header
//!   that is set but not switched on is a header that is not printed.
//!   Here the texts are the facts and the flags are gone.
//! - **The tree's header and footer carry a font** (`&"Arial,Bold"`) and
//!   a size, and the drawing honours them. Here a section says whether
//!   it is bold and whether it is italic, and a font is not a fact a
//!   printed sheet has here.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, Node, NodeRef, Outcome, Session, Text};

use crate::printing::sheet_of;

/// The **most** characters a header or a footer may hold: the tree's
/// own bound.
pub const MAX_TEXT: usize = 10_000;

/// The **kind** of page a header or a footer is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageKind {
    /// The pages that are not the first and not even — the odd pages.
    Odd,
    /// The even pages.
    Even,
    /// The first page of the sheet.
    First,
}

impl PageKind {
    /// The **name** of the kind.
    pub fn name(self) -> &'static str {
        match self {
            PageKind::Odd => "odd",
            PageKind::Even => "even",
            PageKind::First => "first",
        }
    }

    /// The kind a name says, if it is one of the three.
    pub fn parse(text: &str) -> Option<PageKind> {
        match text.trim().to_ascii_lowercase().as_str() {
            "odd" | "all" => Some(PageKind::Odd),
            "even" => Some(PageKind::Even),
            "first" => Some(PageKind::First),
            _ => None,
        }
    }

    /// The kinds, in the order they are kept.
    pub const KINDS: [PageKind; 3] = [PageKind::Odd, PageKind::Even, PageKind::First];
}

/// A **section** of a header or a footer: its text, and whether it is
/// bold and whether it is italic.
#[derive(Debug, Clone, Default)]
pub struct Section {
    /// The text of the section.
    pub text: String,
    /// Whether the text is bold.
    pub bold: bool,
    /// Whether the text is italic.
    pub italic: bool,
}

/// The three **sections** of a header or a footer.
#[derive(Debug, Clone, Default)]
pub struct Sections {
    /// The section at the left.
    pub left: Section,
    /// The section in the centre.
    pub centre: Section,
    /// The section at the right.
    pub right: Section,
}

impl Sections {
    /// The sections, as they are reported.
    pub fn to_json(&self) -> craftkit_sdk::serde_json::Value {
        let section = |section: &Section| json!({ "text": section.text, "bold": section.bold, "italic": section.italic });
        json!({
            "left": section(&self.left),
            "centre": section(&self.centre),
            "right": section(&self.right),
        })
    }
}

/// The **code** a character after an ampersand is, and what it says.
///
/// The codes that name the machine — the date, the time, the file and
/// the path — are not here: a printed sheet is about the document, and
/// a date is the clock of the machine that printed it.
fn code_of(character: char) -> Option<&'static str> {
    match character.to_ascii_uppercase() {
        'L' => Some("the left section"),
        'C' => Some("the centre section"),
        'R' => Some("the right section"),
        'P' => Some("the page number"),
        'N' => Some("the number of pages"),
        'A' => Some("the name of the sheet"),
        'B' => Some("bold"),
        'I' => Some("italic"),
        '&' => Some("an ampersand"),
        _ => None,
    }
}

/// **Check** a header or a footer: every code in it must be one this
/// tree prints.
pub fn check(text: &str) -> Result<(), String> {
    let characters: Vec<char> = text.chars().take(MAX_TEXT + 1).collect();
    if characters.len() > MAX_TEXT {
        return Err(format!("a header or a footer is at most {MAX_TEXT} characters long"));
    }
    let mut index = 0usize;
    while index < characters.len() {
        if characters[index] != '&' {
            index += 1;
            continue;
        }
        match characters.get(index + 1) {
            None => return Err("a header or a footer may not end with an ampersand".to_string()),
            Some(character) => {
                if code_of(*character).is_none() {
                    return Err(format!(
                        "`&{}` is not a code this tree prints: it may be a section, a page, a count, a name, bold, italic or an ampersand",
                        character.to_ascii_uppercase()
                    ));
                }
                index += 2;
            }
        }
    }
    Ok(())
}

/// The **header** or the **footer** of a sheet, for the three kinds of
/// page.
pub fn texts_of(node: &Node, name: &str) -> [Option<String>; 3] {
    let stored = node.data.get(name);
    let read = |kind: PageKind| -> Option<String> {
        stored
            .and_then(|stored| stored.get(kind.name()))
            .and_then(|text| text.as_str())
            .map(|text| text.to_string())
    };
    [read(PageKind::Odd), read(PageKind::Even), read(PageKind::First)]
}

/// The index of a kind in the three kinds.
fn kind_of_index(kind: PageKind) -> usize {
    match kind {
        PageKind::Odd => 0,
        PageKind::Even => 1,
        PageKind::First => 2,
    }
}

/// The **kind** of page a page number is: the first page is the first
/// when it has a header of its own, and an even page is even when it
/// has one of its own.
pub fn kind_of(node: &Node, page: usize) -> PageKind {
    let (header, footer) = (texts_of(node, "print_header"), texts_of(node, "print_footer"));
    if page == 1 && (header[2].is_some() || footer[2].is_some()) {
        return PageKind::First;
    }
    if page.is_multiple_of(2) && (header[1].is_some() || footer[1].is_some()) {
        return PageKind::Even;
    }
    PageKind::Odd
}

/// The **text** of the header of a page, and of its footer: the kind
/// of the page decides, and a kind that has none falls back on the
/// odd pages.
fn text_for(node: &Node, name: &str, page: usize) -> Option<String> {
    let texts = texts_of(node, name);
    let kind = kind_of(node, page);
    match kind {
        PageKind::Odd => texts[0].clone(),
        PageKind::Even => texts[1].clone().or_else(|| texts[0].clone()),
        PageKind::First => texts[2].clone().or_else(|| texts[0].clone()),
    }
}

/// **Expand** a header or a footer into its three sections, for a page.
///
/// The page number and the number of pages are the ones of the page
/// being printed, and the name is the name of the sheet.
pub fn expand(spec: &str, page: usize, pages: usize, sheet: &str) -> Sections {
    let mut out = Sections::default();
    let characters: Vec<char> = spec.chars().take(MAX_TEXT).collect();
    let mut current = 1usize;
    let mut index = 0usize;
    while index < characters.len() {
        let character = characters[index];
        if character != '&' {
            section_of(&mut out, current).text.push(character);
            index += 1;
            continue;
        }
        let Some(&code) = characters.get(index + 1) else { break };
        index += 2;
        match code.to_ascii_uppercase() {
            'L' => current = 0,
            'C' => current = 1,
            'R' => current = 2,
            'P' => {
                let mut number = page as i64;
                // `&P+3` and `&P-3`: the page number, moved.
                if let Some(&sign) = characters.get(index)
                    && (sign == '+' || sign == '-')
                {
                    let digits: String = characters[index + 1..]
                        .iter()
                        .take_while(|digit| digit.is_ascii_digit())
                        .take(6)
                        .collect();
                    if let Ok(moved) = digits.parse::<i64>() {
                        number = if sign == '+' { number + moved } else { number - moved };
                        index += 1 + digits.len();
                    }
                }
                section_of(&mut out, current).text.push_str(&number.to_string());
            }
            'N' => section_of(&mut out, current).text.push_str(&pages.to_string()),
            'A' => section_of(&mut out, current).text.push_str(sheet),
            'B' => {
                let section = section_of(&mut out, current);
                section.bold = !section.bold;
            }
            'I' => {
                let section = section_of(&mut out, current);
                section.italic = !section.italic;
            }
            '&' => section_of(&mut out, current).text.push('&'),
            _ => {}
        }
    }
    out
}

/// The **section** a number stands for: 0 is the left, 1 the centre and
/// 2 the right.
fn section_of(sections: &mut Sections, index: usize) -> &mut Section {
    match index {
        0 => &mut sections.left,
        2 => &mut sections.right,
        _ => &mut sections.centre,
    }
}

/// The **header** and the **footer** of a page, as they are printed.
pub fn of_page(node: &Node, page: usize, pages: usize) -> (Sections, Sections) {
    let sheet = node.name.as_str();
    let header = text_for(node, "print_header", page).unwrap_or_default();
    let footer = text_for(node, "print_footer", page).unwrap_or_default();
    (expand(&header, page, pages, sheet), expand(&footer, page, pages, sheet))
}

/// The header and the footer of a sheet, as they are kept.
pub fn to_json(node: &Node) -> craftkit_sdk::serde_json::Value {
    let texts = |name: &str| {
        let kept = texts_of(node, name);
        json!({
            "odd": kept[0].clone(),
            "even": kept[1].clone(),
            "first": kept[2].clone(),
        })
    };
    json!({ "header": texts("print_header"), "footer": texts("print_footer") })
}

/// **Set** the header of a sheet — or take it away.
///
/// The text may hold the codes of a printed page: `&P` is the page
/// number, `&N` the number of pages, `&A` the name of the sheet, and
/// `&L`, `&C` and `&R` the three sections the text stands in. An
/// empty text takes the header away.
#[derive(Command)]
#[command(id = "grid.print.header", area = "grid", layer = "grid", icon = "page", mutates, undoable)]
pub struct SetHeader {
    /// The sheet.
    pub sheet: NodeRef,
    /// The kind of page: `odd`, `even` or `first`. Without it, the odd
    /// pages are meant.
    pub page: Option<Text>,
    /// The text of the header.
    pub text: Text,
}

impl SetHeader {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        set(session, "grid.print.header", "print_header", self.sheet.0, &self.page, &self.text.0)
    }
}

/// **Set** the footer of a sheet — or take it away.
///
/// The codes are the codes of the header: `&P` is the page number,
/// `&N` the number of pages, `&A` the name of the sheet, and `&L`,
/// `&C` and `&R` the three sections the text stands in.
#[derive(Command)]
#[command(id = "grid.print.footer", area = "grid", layer = "grid", icon = "page", mutates, undoable)]
pub struct SetFooter {
    /// The sheet.
    pub sheet: NodeRef,
    /// The kind of page: `odd`, `even` or `first`. Without it, the odd
    /// pages are meant.
    pub page: Option<Text>,
    /// The text of the footer.
    pub text: Text,
}

impl SetFooter {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        set(session, "grid.print.footer", "print_footer", self.sheet.0, &self.page, &self.text.0)
    }
}

/// **Set** a header or a footer of a sheet.
fn set(
    session: &mut Session,
    id: &str,
    name: &str,
    sheet: u64,
    page: &Option<Text>,
    text: &str,
) -> Result<Outcome, EngineError> {
    let node = sheet_of(session, id, sheet)?;
    let kind = match page {
        Some(page) => match PageKind::parse(&page.0) {
            Some(kind) => kind,
            None => {
                return Err(EngineError::Failed {
                    id: id.to_string(),
                    message: format!(
                        "`{}` is not a kind of page: it is `odd`, `even` or `first`",
                        page.0.trim()
                    ),
                });
            }
        },
        None => PageKind::Odd,
    };
    if let Err(refusal) = check(text) {
        return Err(EngineError::Failed { id: id.to_string(), message: refusal });
    }
    let mut kept = texts_of(&node, name);
    kept[kind_of_index(kind)] = if text.trim().is_empty() { None } else { Some(text.to_string()) };
    if let Some(node) = session.document_mut()?.node_mut(sheet) {
        let mut group = json!({});
        for (index, kind) in PageKind::KINDS.iter().enumerate() {
            group[kind.name()] = match &kept[index] {
                Some(text) => json!(text),
                None => craftkit_sdk::serde_json::Value::Null,
            };
        }
        node.data[name] = group;
    }
    Ok(Outcome::changed(format!("the {name} of the sheet is set")).with_data(json!({
        "sheet": sheet,
        "page": kind.name(),
        "text": text,
    })))
}
