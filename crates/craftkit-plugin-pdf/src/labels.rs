//! The **labels** of the pages.
//!
//! A PDF file may give its pages **labels** — the names a reader sees
//! beside them: `i`, `ii`, `iii` for the front matter and `1`, `2`,
//! `3` for the rest. The labels live in a **number tree** hanging from
//! the page tree, and the tree says which page a run of labels begins
//! at and what the run looks like.
//!
//! # What the port found
//!
//! - **The tree's labels are a thing of their own** — a list of
//!   entries, each with a style, a prefix and the number it starts
//!   at, and the tree's own file format has a section for them. Here
//!   the labels are the **PDF's own** `/PageLabels`: a number tree
//!   with the entries the format says.
//! - **The tree's labels may be read back and the file's may not**,
//!   here: the labels are written into the file and the file is the
//!   place they live. A command that reads them back would be a
//!   second copy of them, and a second copy is a copy that can be
//!   wrong.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{self, json};
use craftkit_sdk::{EngineError, NodeRef, Outcome};

use crate::{a_reference, add_reference, pairs_of, the_next_number};

/// The **styles** a label may have.
///
/// `D` is arabic, `R` and `r` are roman, and `A` and `a` are letters.
const STYLES: [&str; 5] = ["D", "R", "r", "A", "a"];

/// One **run** of labels: the page it begins at and how it is written.
struct Run {
    /// The page the run begins at, counted from one.
    page: u64,
    /// How the numbers are written.
    style: String,
    /// The text before the number.
    prefix: Option<String>,
    /// The number the run starts at.
    start: u64,
}

/// The **runs** of labels, as the caller gives them.
fn runs_of(labels: &serde_json::Value, id: &str) -> Result<Vec<Run>, EngineError> {
    let Some(list) = labels.as_array() else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: "the labels of the pages are a list".to_string(),
        });
    };
    if list.is_empty() {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: "a file has labels or no labels, and a list of none is no labels".to_string(),
        });
    }
    let mut runs: Vec<Run> = Vec::with_capacity(list.len());
    for label in list {
        let page = label.get("page").and_then(|page| page.as_u64()).ok_or_else(|| EngineError::Failed {
            id: id.to_string(),
            message: "a label says which page it begins at".to_string(),
        })?;
        if page == 0 {
            return Err(EngineError::Failed {
                id: id.to_string(),
                message: "the pages are counted from one".to_string(),
            });
        }
        let style = label.get("style").and_then(|style| style.as_str()).unwrap_or("D").to_string();
        if !STYLES.contains(&style.as_str()) {
            return Err(EngineError::Failed {
                id: id.to_string(),
                message: format!("`{style}` is not a style: the styles are {}", STYLES.join(", ")),
            });
        }
        let prefix = match label.get("prefix") {
            None => None,
            Some(prefix) => {
                let prefix = prefix.as_str().ok_or_else(|| EngineError::Failed {
                    id: id.to_string(),
                    message: "the prefix of a label is text".to_string(),
                })?;
                Some(prefix.to_string())
            }
        };
        let start = label.get("start").and_then(|start| start.as_u64()).unwrap_or(1);
        if start == 0 {
            return Err(EngineError::Failed {
                id: id.to_string(),
                message: "the numbers of a label start at one".to_string(),
            });
        }
        runs.push(Run { page, style, prefix, start });
    }
    // The runs are **in order**: the first is the page they begin at
    // and the next is later. A number tree is sorted, and a tree that
    // is not sorted is not a tree.
    for pair in runs.windows(2) {
        if pair[1].page <= pair[0].page {
            return Err(EngineError::Failed {
                id: id.to_string(),
                message: format!(
                    "the labels are in order: the page {} comes before the page {}",
                    pair[0].page, pair[1].page
                ),
            });
        }
    }
    Ok(runs)
}

commands! {
    /// Give the **pages** of a document their labels.
    pdf.page.labels {
        area: "pdf",
        layer: "pdf",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The pages: the object that holds them.
            pages: NodeRef,
            /// The labels: a list of `{"page": 1, "style": "D", "prefix": "A-",
            /// "start": 1}`.
            labels: serde_json::Value,
        },
        run: {
            let id = "pdf.page.labels".to_string();
            let pages = self.pages.0;
            let Some(node) = session.document().node(pages) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{pages}`"),
                });
            };
            let is_pages = crate::objects::pairs_of(&node.data)
                .iter()
                .any(|(key, value)| key == "Type" && value.as_str() == Some("Pages"));
            if !is_pages {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{pages}` is not the pages of a document"),
                });
            }
            let runs = runs_of(&self.labels, &id)?;
            // The **number tree**: an object of the file that holds the
            // runs, and the pages name it.
            let number = the_next_number(session.document(), 0);
            let mut nums: Vec<serde_json::Value> = Vec::with_capacity(runs.len());
            for run in &runs {
                let mut entries: Vec<(String, serde_json::Value)> = vec![("S".to_string(), json!(run.style))];
                if let Some(prefix) = &run.prefix {
                    entries.push(("P".to_string(), json!({ "string": prefix })));
                }
                entries.push(("St".to_string(), json!(run.start)));
                // A **pair**: the page the run begins at and the run
                // itself, written one after the other.
                nums.push(json!({
                    "pair": [
                        run.page - 1,
                        { "entries": pairs_of(&entries) },
                    ],
                }));
            }
            let tree_data = json!({
                "object": number,
                "entries": pairs_of(&[("Nums".to_string(), json!(nums))]),
            });
            let tree = session.document_mut()?.add_node_with("the page labels", "pdf.object", None, tree_data);
            // The **pages** name the tree.
            let reference = a_reference(session, tree, 0, &id)?;
            add_reference(session, pages, "PageLabels", reference, false)?;
            Ok(Outcome::changed(format!(
                "the pages of the document `{}` have {} run(s) of labels",
                pages,
                runs.len()
            ))
            .with_data(json!({ "pages": pages, "tree": tree, "number": number, "runs": runs.len() })))

        }
    }
}
