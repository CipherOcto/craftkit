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
//!   who cannot see it.
//! - **headingOrder** — a heading that is more than one level deeper
//!   than the heading before it. It is a **warning**: the outline has
//!   a gap in it.
//! - **blankParagraphs** — empty paragraphs in a row. It is a **tip**:
//!   spacing made with the return key.
//! - **linkText** — a link whose words are "click here". It is a
//!   **tip**: the words say nothing about where the link goes.
//!
//! The issues come first as **errors**, then as warnings, then as
//! tips, and the order of the document is kept within each kind.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Node, NodeRef, Outcome, Session};

/// How many **empty paragraphs in a row** look like spacing made
/// with the return key.
const BLANK_RUN: usize = 3;

/// The most issues a document may report: a hostile document cannot
/// make the report endless.
const MAX_ISSUES: usize = 500;

/// The link words that say nothing about where the link goes.
const CLICK_HERE: &str = "click here";

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

/// The **runs** of a block, with the words they cover.
fn runs_of(node: &Node) -> Vec<(String, Option<String>)> {
    let characters: Vec<char> = text_of(node).chars().collect();
    let Some(runs) = node.data.get("runs").and_then(|runs| runs.as_array()) else {
        return Vec::new();
    };
    let mut out: Vec<(String, Option<String>)> = Vec::new();
    let mut at = 0usize;
    for run in runs {
        let length = run.get("len").and_then(|len| len.as_u64()).unwrap_or(0) as usize;
        let end = at.saturating_add(length).min(characters.len());
        let covered: String = characters[at.min(characters.len())..end].iter().collect();
        at = end;
        let target = run.get("link").and_then(|link| link.as_str()).map(|link| link.to_string());
        out.push((covered, target));
    }
    out
}

/// The **issues** of a document: what is wrong with it.
pub fn issues_of(document: &Document, node: &Node) -> Vec<Issue> {
    let mut issues: Vec<Issue> = Vec::new();
    // The **pictures**: a picture that says nothing about itself.
    for media in node.children.iter().filter(|child| child.kind == "prose.media") {
        if crate::media::alt_of(media).is_none() {
            issues.push(Issue {
                kind: "error",
                rule: "altText",
                issue: format!("The picture `{}` has no alternative text", media.name),
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
        // The **link words**: "click here" says nothing.
        for (covered, target) in runs_of(block) {
            if target.is_none() {
                continue;
            }
            if covered.trim().to_lowercase() == CLICK_HERE {
                issues.push(Issue {
                    kind: "tip",
                    rule: "linkText",
                    issue: "The words of a link say nothing about where it goes".to_string(),
                    place: block.id,
                });
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
    // The **errors** come first, then the warnings, then the tips.
    issues.sort_by_key(Issue::rank);
    issues.truncate(MAX_ISSUES);
    issues
}

/// **Check** a document: what makes it hard to use.
///
/// A query: it does not change the document.
#[derive(Command)]
#[command(id = "prose.accessibility", area = "prose", layer = "prose", icon = "list")]
pub struct CheckAccessibility {
    /// The document.
    pub document: NodeRef,
}

impl CheckAccessibility {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
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
