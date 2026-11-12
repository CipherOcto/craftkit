//! The **accessibility** of a document.
//!
//! Some documents are hard to use: with a screen reader, with a
//! magnifier, or with a reader who cannot see the picture at all. The
//! **checker** says what is wrong, in the tree's own shape: an issue
//! has a **kind** — an `error`, a `warning` or a `tip` — the **rule**
//! it breaks, the **words** that say what is wrong, and the **place**
//! it is about.
//!
//! The rules that are here are the ones this tree can see:
//!
//! - **altText** — a picture that says nothing about itself. It is an
//!   **error**: a picture with no alt text is not there for the reader
//!   who cannot see it. A picture that is **decorative** is not asked
//!   for words, because it says nothing on purpose.
//! - **headingOrder** — a heading that is more than one level deeper
//!   than the heading before it. It is a **warning**: the outline has
//!   a gap in it.
//! - **blankParagraphs** — empty paragraphs in a row. It is a **tip**:
//!   spacing made with the return key.
//! - **linkText** — a link whose words are "click here". It is a
//!   **tip**: the words say nothing about where the link goes.
//! - **tableHeader** — a table of two or more rows whose first row is
//!   not a **header row**. It is a **warning**: nothing says what the
//!   columns are.
//! - **docTitle** — a document with no title. It is a **tip**: nothing
//!   says what the document is.
//! - **tableAltText** — a table that says nothing about itself. It is
//!   a **warning**: nothing says what the table is.
//! - **floating** — a picture that is not in the line of the text. It
//!   is a **tip**: a reader who cannot see the picture cannot tell
//!   where in the text it belongs.
//! - **contrast** — text that is hard to read. The measure is the
//!   **contrast ratio** of the WCAG 2: four and a half for normal text
//!   and three for large text. A ratio under three is a **warning**
//!   and the rest is a **tip**.
//!
//! The issues come first as **errors**, then as warnings, then as
//! tips, and the order of the document is kept within each kind.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Node, NodeRef, Outcome};

/// How many **empty paragraphs in a row** look like spacing made
/// with the return key.
const BLANK_RUN: usize = 3;

/// The most issues a document may report: a hostile document cannot
/// make the report endless.
const MAX_ISSUES: usize = 500;

/// The link words that say nothing about where the link goes.
const CLICK_HERE: &str = "click here";

/// The most **columns** of a table that are looked at: a hostile
/// table cannot make the report endless.
const COLUMNS: usize = 63;

/// Whether a **cell** is empty: nothing, or a text with nothing in
/// it.
fn is_empty(cell: &Json) -> bool {
    match cell {
        Json::Null => true,
        Json::String(text) => text.trim().is_empty(),
        _ => false,
    }
}

/// An **issue**: something that makes a document hard to use.
#[derive(Debug, Clone, PartialEq)]
pub struct Issue {
    /// How bad it is: `error`, `warning` or `tip`.
    pub kind: &'static str,
    /// The **rule** it breaks.
    pub rule: &'static str,
    /// What is wrong, in words.
    pub issue: String,
    /// The **node** it is about.
    pub place: u64,
}

impl Issue {
    /// The issue, as the caller gets it.
    pub fn to_json(&self) -> Json {
        json!({
            "kind": self.kind,
            "rule": self.rule,
            "issue": self.issue.clone(),
            "where": self.place,
        })
    }

    /// The rank of the kind: the errors come first.
    fn rank(&self) -> u8 {
        match self.kind {
            "error" => 0,
            "warning" => 1,
            _ => 2,
        }
    }
}

/// The **text** of a block.
fn text_of(node: &Node) -> String {
    node.data.get("text").and_then(|text| text.as_str()).unwrap_or_default().to_string()
}

/// One **run** of a block: the words it covers, and how it is drawn.
struct Run {
    /// The words the run covers.
    text: String,
    /// The target of the link the run carries.
    link: Option<String>,
    /// The colour of the text, as `#rrggbb`.
    colour: Option<String>,
    /// The size of the text.
    size: Option<f64>,
    /// Whether the text is bold.
    bold: bool,
}

/// The **runs** of a block, with the words they cover.
fn runs_of(node: &Node) -> Vec<Run> {
    let characters: Vec<char> = text_of(node).chars().collect();
    let Some(runs) = node.data.get("runs").and_then(|runs| runs.as_array()) else {
        return Vec::new();
    };
    let mut out: Vec<Run> = Vec::new();
    let mut at = 0usize;
    for run in runs {
        let length = run.get("len").and_then(|len| len.as_u64()).unwrap_or(0) as usize;
        let end = at.saturating_add(length).min(characters.len());
        let covered: String = characters[at.min(characters.len())..end].iter().collect();
        at = end;
        out.push(Run {
            text: covered,
            link: run.get("link").and_then(|link| link.as_str()).map(|link| link.to_string()),
            colour: run.get("colour").and_then(|colour| colour.as_str()).map(|colour| colour.to_string()),
            size: run.get("size").and_then(|size| size.as_f64()),
            bold: run.get("bold").and_then(|bold| bold.as_bool()).unwrap_or(false),
        });
    }
    out
}

/// The **issues** of a document: what is wrong with it.
pub fn issues_of(document: &Document, node: &Node) -> Vec<Issue> {
    let mut issues: Vec<Issue> = Vec::new();
    // The **pictures**: a picture that says nothing about itself.
    for media in node.children.iter().filter(|child| child.kind == "prose.media") {
        // A **decorative** picture says nothing on purpose, and it is
        // not asked for words.
        if crate::media::is_decorative(media) {
            continue;
        }
        if crate::media::alt_of(media).is_none() {
            issues.push(Issue {
                kind: "error",
                rule: "altText",
                issue: format!("The picture `{}` has no alternative text", media.name),
                place: media.id,
            });
        }
        // A picture that **floats** is not in the line of the text,
        // and the reading order is unclear.
        if crate::media::is_floating(media) {
            issues.push(Issue {
                kind: "tip",
                rule: "floating",
                issue: format!(
                    "The picture `{}` is not in line with the text, so its reading order is unclear",
                    media.name
                ),
                place: media.id,
            });
        }
    }
    // The **blocks**: the headings, the empty paragraphs and the links.
    let blocks: Vec<std::sync::Arc<Node>> =
        node.children.iter().filter(|child| child.kind.starts_with("prose.block.")).cloned().collect();
    let mut last_level: Option<u32> = None;
    let mut blank: Vec<u64> = Vec::new();
    for block in &blocks {
        // The **heading order**: a heading deeper than one level.
        if let Some(style) = crate::styles::style_of_block(document, block.id)
            && let Some(level) = crate::outline::effective_level(document, style.id)
        {
            if let Some(previous) = last_level
                && level > previous + 1
            {
                issues.push(Issue {
                    kind: "warning",
                    rule: "headingOrder",
                    issue: format!(
                        "A heading of level {level} follows a heading of level {previous}, and the outline has a gap in it"
                    ),
                    place: block.id,
                });
            }
            last_level = Some(level);
        }
        // The **empty paragraphs**: spacing made with the return key.
        let empty = block.kind == "prose.block.paragraph" && text_of(block).trim().is_empty();
        if empty {
            blank.push(block.id);
        } else {
            if blank.len() >= BLANK_RUN {
                issues.push(Issue {
                    kind: "tip",
                    rule: "blankParagraphs",
                    issue: format!("{} empty paragraphs in a row", blank.len()),
                    place: blank[0],
                });
            }
            blank.clear();
        }
        // The **tables**: a table that says nothing about itself.
        if block.kind == "prose.block.table" && crate::tables::alt_of(block).is_none() {
            issues.push(Issue {
                kind: "warning",
                rule: "tableAltText",
                issue: "The table has no alternative text, and nothing says what it is".to_string(),
                place: block.id,
            });
        }
        // The **tables**: the empty rows and the empty columns. A
        // table with nothing in it yet is being filled in, and not
        // used for spacing.
        if block.kind == "prose.block.table" {
            let rows = block.data.get("rows").and_then(|rows| rows.as_array()).cloned().unwrap_or_default();
            let grid: Vec<Vec<Json>> =
                rows.iter().map(|row| row.as_array().cloned().unwrap_or_default()).collect();
            let filled = grid.iter().flatten().any(|cell| !is_empty(cell));
            if grid.len() >= 2 && filled {
                for (index, row) in grid.iter().enumerate() {
                    if !row.is_empty() && row.iter().all(is_empty) {
                        issues.push(Issue {
                            kind: "warning",
                            rule: "blankRow",
                            issue: format!("Row {} of a table is empty: it looks like spacing", index + 1),
                            place: block.id,
                        });
                    }
                }
                let width = grid.iter().map(|row| row.len()).max().unwrap_or(0);
                if width >= 2 {
                    for column in 0..width.min(COLUMNS) {
                        let empty = grid.iter().all(|row| match row.get(column) {
                            Some(cell) => is_empty(cell),
                            None => true,
                        });
                        if empty {
                            issues.push(Issue {
                                kind: "warning",
                                rule: "blankColumn",
                                issue: format!(
                                    "Column {} of a table is empty: it looks like spacing",
                                    column + 1
                                ),
                                place: block.id,
                            });
                        }
                    }
                }
            }
        }
        // The **tables**: a table with two or more rows says what
        // its columns are in a header row.
        if block.kind == "prose.block.table"
            && crate::tables::row_count(block) >= 2
            && !crate::tables::has_a_header(block)
        {
            issues.push(Issue {
                kind: "warning",
                rule: "tableHeader",
                issue: "The table has no header row, and nothing says what its columns are".to_string(),
                place: block.id,
            });
        }
        // The **link words**: "click here" says nothing.
        for run in runs_of(block) {
            if run.link.is_none() {
                continue;
            }
            if run.text.trim().to_lowercase() == CLICK_HERE {
                issues.push(Issue {
                    kind: "tip",
                    rule: "linkText",
                    issue: "The words of a link say nothing about where it goes".to_string(),
                    place: block.id,
                });
            }
        }
        // The **contrast**: text that is hard to read.
        for run in runs_of(block) {
            if run.text.trim().is_empty() {
                continue;
            }
            let Some(text) = &run.colour else { continue };
            let Ok(colour) = craftkit_sdk::Colour::from_hex(text) else { continue };
            let ratio = crate::contrast::contrast_ratio(&colour, &craftkit_sdk::Colour::WHITE);
            let need = crate::contrast::needed(run.size, run.bold);
            if ratio < need {
                issues.push(Issue {
                    kind: if crate::contrast::is_poor(ratio) { "warning" } else { "tip" },
                    rule: "contrast",
                    issue: format!(
                        "The text is hard to read: the contrast is {ratio:.1}:1 and {need}:1 is needed"
                    ),
                    place: block.id,
                });
                // One issue for the block: the text is hard to read,
                // and saying it twice says it no better.
                break;
            }
        }
    }
    if blank.len() >= BLANK_RUN {
        issues.push(Issue {
            kind: "tip",
            rule: "blankParagraphs",
            issue: format!("{} empty paragraphs in a row", blank.len()),
            place: blank[0],
        });
    }
    // The **title** of the document: nothing says what it is.
    if crate::titles::title_of(node).is_none() {
        issues.push(Issue {
            kind: "tip",
            rule: "docTitle",
            issue: "The document has no title".to_string(),
            place: node.id,
        });
    }
    // The **errors** come first, then the warnings, then the tips.
    issues.sort_by_key(Issue::rank);
    issues.truncate(MAX_ISSUES);
    issues
}

commands! {
    /// **Check** a document: what makes it hard to use.
    ///
    /// A query: it does not change the document.
    prose.accessibility {
        area: "prose",
        layer: "prose",
        icon: "list",
        params: {
            /// The document.

            document: NodeRef,
        },
        run: {

            let id = "prose.accessibility".to_string();
            let document = self.document.0;
            let Some(node) = session.document().node(document).cloned() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no document `{document}`"),
                });
            };
            if node.kind != "prose.document" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{document}` is a `{}` and not a document", node.kind),
                });
            }
            let issues = issues_of(session.document(), &node);
            let count = issues.len();
            Ok(Outcome::new(format!("the document has {count} issue(s)")).with_data(json!({
                "document": document,
                "issues": issues.iter().map(Issue::to_json).collect::<Vec<_>>(),
                "count": count,
            })))

        }
    }
}
