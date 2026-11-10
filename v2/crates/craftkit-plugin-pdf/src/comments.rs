//! The **comments** of a file: the annotations that are notes.
//!
//! The tree's own: a **comment** is an annotation that is **none of a
//! link, a widget and a popup** — a link is not a note, a widget is
//! the appearance of a field, and a popup is the window of a note
//! rather than the note itself.
//!
//! # The rules
//!
//! - the **pages** are the pages of the tree of pages, in the order
//!   the tree holds them;
//! - the **annotations** of a page are the ones the `/Annots` of the
//!   page holds, in the order the file holds them, and the **index**
//!   of an annotation is its place in that list — a link counts as a
//!   place, even though it is not a comment;
//! - a comment says:
//!   - the **page** it is on, and its **index** on that page;
//!   - the **subtype** — the kind of note it is;
//!   - the **author** (`/T`), the **contents** (`/Contents`), the
//!     **day** it was changed (`/M`) and its **name** (`/NM`);
//!   - the comment it **answers** (`/IRT` — the name of that one);
//!   - its **rectangle** (`/Rect`), which is two corners in order;
//!   - its **colour** (`/C`);
//!   - its **quads** (`/QuadPoints`), which are the lines of text it
//!     lies over, eight numbers at a time;
//!   - whether it is **locked** (the flag `Locked`);
//!   - its **intent** (`/IT`) — what it means, when it says.
//! - the **colour** of a comment that is written free (`FreeText`) is
//!   the colour of its **default appearance** (`/DA`) — the tree's own
//!   rule, and the rule here: the three numbers before the `rg` of
//!   the appearance.
//! - the **list** is **sorted**: by the page, and then from the **top
//!   of the page down** — the comment whose rectangle reaches highest
//!   comes first.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Outcome, Session};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.comment.read";

/// How deep the tree of pages may be walked.
pub(crate) const MAX_DEPTH: usize = 64;

/// The **entry** of a dictionary, by its key.
fn entry<'a>(entries: &'a [(String, Json)], key: &str) -> Option<&'a Json> {
    entries.iter().find(|(name, _)| name == key).map(|(_, value)| value)
}

/// The **pages** of the file, in the order of the tree of pages.
fn pages_of(document: &Document) -> Vec<u64> {
    let Some(catalog) = crate::preflight::the_catalog(document) else { return Vec::new() };
    let catalog_entries = crate::entries_of(&catalog.data);
    let Some(pages) = entry(&catalog_entries, "Pages") else { return Vec::new() };
    let Some(node) = crate::preflight::node_of(document, pages) else { return Vec::new() };
    let mut out: Vec<u64> = Vec::new();
    walk_pages(document, node.id, 0, &mut out);
    out
}

/// Walk the **tree of pages**, and take the pages of it in order.
fn walk_pages(document: &Document, node: u64, depth: usize, out: &mut Vec<u64>) {
    if depth >= MAX_DEPTH {
        return;
    }
    let Some(current) = document.node(node) else { return };
    let entries = crate::entries_of(&current.data);
    let is_page = entries.iter().any(|(key, value)| key == "Type" && value.as_str() == Some("Page"));
    if is_page {
        out.push(node);
        return;
    }
    let Some(kids) = entry(&entries, "Kids").and_then(|kids| kids.as_array()).cloned() else {
        return;
    };
    for kid in kids {
        let Some(kid_node) = crate::preflight::node_of(document, &kid) else { continue };
        let id = kid_node.id;
        walk_pages(document, id, depth + 1, out);
    }
}

/// The **annotations** of a page, in the order the file holds them.
fn annots_of(document: &Document, page: u64) -> Vec<u64> {
    let Some(node) = document.node(page) else { return Vec::new() };
    let entries = crate::entries_of(&node.data);
    let Some(annots) = entry(&entries, "Annots").and_then(|annots| annots.as_array()).cloned() else {
        return Vec::new();
    };
    let mut out: Vec<u64> = Vec::new();
    for annot in annots {
        let Some(node) = crate::preflight::node_of(document, &annot) else { continue };
        out.push(node.id);
    }
    out
}

/// The **numbers** of a list of numbers.
fn numbers_of(value: &Json) -> Vec<f64> {
    value
        .as_array()
        .map(|list| list.iter().map(|item| item.as_f64().unwrap_or(0.0)).collect())
        .unwrap_or_default()
}

/// The **rectangle** of a comment: two corners in order, and the
/// origin when the file says nothing that is a rectangle.
fn rect_of(entries: &[(String, Json)]) -> [f64; 4] {
    let numbers = entry(entries, "Rect").map(numbers_of).unwrap_or_default();
    if numbers.len() != 4 {
        return [0.0; 4];
    }
    [
        numbers[0].min(numbers[2]),
        numbers[1].min(numbers[3]),
        numbers[0].max(numbers[2]),
        numbers[1].max(numbers[3]),
    ]
}

/// The **colour** of a comment.
///
/// The tree's own: the `/C` of the comment, and for a comment that is
/// written free — a `FreeText` — the colour of its **default
/// appearance**: the three numbers that come before the `rg` of the
/// `/DA`.
fn colour_of(entries: &[(String, Json)], subtype: &str) -> Option<[f64; 3]> {
    let numbers = entry(entries, "C").map(numbers_of).unwrap_or_default();
    if numbers.len() == 3 {
        return Some([numbers[0], numbers[1], numbers[2]]);
    }
    if subtype != "FreeText" {
        return None;
    }
    let appearance = entry(entries, "DA").and_then(crate::fields::text_of)?;
    let parts: Vec<&str> = appearance.split_whitespace().collect();
    let at = parts.iter().position(|part| *part == "rg")?;
    if at < 3 {
        return None;
    }
    let number = |index: usize| parts[index].parse::<f64>().unwrap_or(0.0);
    Some([number(at - 3), number(at - 2), number(at - 1)])
}

/// The **comment** an annotation answers: the name of the comment it
/// is a reply to.
fn reply_of(document: &Document, entries: &[(String, Json)]) -> Option<String> {
    let reply = entry(entries, "IRT")?;
    let node = crate::preflight::node_of(document, reply)?;
    let reply_entries = crate::entries_of(&node.data);
    entry(&reply_entries, "NM").and_then(crate::fields::text_of)
}

/// The **flags** of a comment: the number the file holds.
fn flags_of(entries: &[(String, Json)]) -> i64 {
    entry(entries, "F")
        .and_then(|flags| flags.as_f64())
        .filter(|flags| flags.is_finite() && *flags > 0.0)
        .map(|flags| flags as i64)
        .unwrap_or(0)
}

/// **Read** the comments of a file.
///
/// The tree's own: the comments are the annotations that are none of a
/// link, a widget and a popup, and the list is sorted by the page and
/// then from the top of the page down.
#[derive(Command)]
#[command(id = "pdf.comment.read", area = "pdf", layer = "pdf", icon = "list")]
pub struct ReadComments {
    /// The page whose comments are read; every page of the file when
    /// it is not said.
    pub page: Option<craftkit_sdk::NodeRef>,
}

impl ReadComments {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let document = session.document();
        let pages = pages_of(document);
        // The **pages** that are asked for.
        let wanted: Vec<(usize, u64)> = match self.page {
            Some(page) => {
                let Some(index) = pages.iter().position(|held| *held == page.0) else {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: format!("the node `{}` is not a page of the file", page.0),
                    });
                };
                vec![(index, page.0)]
            }
            None => pages.iter().copied().enumerate().collect(),
        };
        let mut comments: Vec<Json> = Vec::new();
        for (page_index, page) in wanted {
            for (index, annot) in annots_of(document, page).iter().enumerate() {
                let Some(node) = document.node(*annot) else { continue };
                let entries = crate::entries_of(&node.data);
                let Some(subtype) = entry(&entries, "Subtype").and_then(|kind| kind.as_str()) else {
                    continue;
                };
                if matches!(subtype, "Link" | "Widget" | "Popup") {
                    continue;
                }
                let rect = rect_of(&entries);
                let quads: Vec<Vec<f64>> = entry(&entries, "QuadPoints")
                    .map(numbers_of)
                    .unwrap_or_default()
                    .chunks(8)
                    .map(|chunk| chunk.to_vec())
                    .collect();
                comments.push(json!({
                    "page": page_index,
                    "index": index,
                    "id": node.id,
                    "subtype": subtype,
                    "author": entry(&entries, "T").and_then(crate::fields::text_of),
                    "contents": entry(&entries, "Contents").and_then(crate::fields::text_of),
                    "modified": entry(&entries, "M").and_then(crate::fields::text_of),
                    "name": entry(&entries, "NM").and_then(crate::fields::text_of),
                    "in_reply_to": reply_of(document, &entries),
                    "rect": rect,
                    "colour": colour_of(&entries, subtype),
                    "quads": quads,
                    "locked": flags_of(&entries) & crate::annot::LOCKED != 0,
                    "intent": entry(&entries, "IT").and_then(|intent| intent.as_str()),
                }));
            }
        }
        // The **order** of the tree's own: by the page, and then from
        // the top of the page down.
        comments.sort_by(|left, right| {
            let page = left["page"].as_u64().cmp(&right["page"].as_u64());
            if page != std::cmp::Ordering::Equal {
                return page;
            }
            let top = |comment: &Json| {
                comment["rect"].as_array().and_then(|rect| rect.get(3)).and_then(Json::as_f64).unwrap_or(0.0)
            };
            top(right).total_cmp(&top(left))
        });
        let count = comments.len();
        Ok(Outcome::new(format!("the file holds {count} comment(s)")).with_data(json!({
            "comments": comments,
            "count": count,
        })))
    }
}
