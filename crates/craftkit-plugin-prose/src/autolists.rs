//! The **list** a paragraph begins.
//!
//! A paragraph that begins with `1. ` or `* ` says what it is: it is
//! an item of a list, and the marker is not part of its words. The
//! tree does this as the user types — the moment the space after the
//! marker is typed, the paragraph becomes an item and the marker
//! goes away.
//!
//! # The rules
//!
//! - The **markers** are the tree's own: `* `, `- ` and `> ` make a
//!   bullet, and `1. `, `1) `, `a. `, `a) `, `A. `, `i. ` and `I. `
//!   make a numbered list. The two markers of two characters — `-> `
//!   and `=> ` — are not here: a bullet of this tree is one
//!   character.
//! - The **marker is taken away** from the words of the paragraph, and
//!   the **runs** are rewritten with it: the runs cover the words that
//!   are left.
//! - A **bullet** joins the bullet list of that kind that the
//!   document already has, so that a list of bullets is one list; a
//!   **numbered** list always starts a new one, as the tree's does.
//! - A paragraph that is **already in a list** is refused: a block is
//!   in one list.
//! - A paragraph that does **not begin** with a marker is refused by
//!   name.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Outcome, Relation, Session};

/// The **markers** and the kind of list each one makes: the tree's
/// own.
pub const MARKERS: [(&str, &str); 10] = [
    ("* ", "bullet"),
    ("- ", "-"),
    ("> ", ">"),
    ("1. ", "numbered"),
    ("1) ", "numberedParen"),
    ("a. ", "lowerLetterDot"),
    ("a) ", "lowerLetter"),
    ("A. ", "upperLetter"),
    ("i. ", "lowerRoman"),
    ("I. ", "outline"),
];

/// The **marker** a text begins with, and the kind of list it makes.
fn marker_of(text: &str) -> Option<(&'static str, &'static str)> {
    MARKERS.iter().find(|(marker, _)| text.starts_with(marker)).copied()
}

/// The **bullet** kinds: the ones that join a list that is there.
fn is_bullet(kind: &str) -> bool {
    kind == "bullet" || kind.chars().count() == 1
}

/// The **list** of a kind that the document already has.
fn list_of(session: &Session, document: u64, kind: &str) -> Option<u64> {
    session
        .document()
        .node(document)?
        .children
        .iter()
        .filter(|child| child.kind == "prose.list")
        .find(|child| child.data.get("kind").and_then(|kind| kind.as_str()) == Some(kind))
        .map(|child| child.id)
}

/// The **text** of a paragraph without its marker, and the **runs**
/// with it.
fn without_marker(text: &str, runs: &[Json], length: usize) -> (String, Vec<Json>) {
    let characters: Vec<char> = text.chars().collect();
    let rest: String = characters.iter().skip(length).collect();
    // Every boundary of the runs moves back by the length of the
    // marker.
    let moved = |offset: usize| -> usize { offset.saturating_sub(length) };
    let mut new_runs = Vec::new();
    let mut start = 0usize;
    for run in runs {
        let run_length = run.get("len").and_then(|len| len.as_u64()).unwrap_or(0) as usize;
        let end = start + run_length;
        let new_start = moved(start);
        let new_end = moved(end);
        let mut run = run.clone();
        run["len"] = json!(new_end.saturating_sub(new_start));
        if new_end > new_start {
            new_runs.push(run);
        }
        start = end;
    }
    (rest, new_runs)
}

commands! {
    /// **Make** the **list** a paragraph begins.
    ///
    /// The marker is taken away from the words and the paragraph becomes
    /// an item of a list of that kind.
    prose.block.list {
        area: "prose",
        layer: "prose",
        icon: "list",
        mutates,
        undoable,
        params: {
            /// The paragraph.
            block: NodeRef,
        },
        run: {
            let id = "prose.block.list".to_string();
            let block = self.block.0;
            let Some(node) = session.document().node(block).cloned() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{block}`"),
                });
            };
            if node.kind != "prose.block.paragraph" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{block}` is a `{}` and not a paragraph", node.kind),
                });
            }
            // The **document** the paragraph is in.
            let Some(document) = session
                .document()
                .walk()
                .find(|node| node.children.iter().any(|child| child.id == block))
                .map(|node| node.id)
            else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the block `{block}` is in no document"),
                });
            };
            // A paragraph that is **already in a list** is refused: a
            // block is in one list.
            if session
                .document()
                .relations
                .iter()
                .any(|relation| relation.kind == crate::numbering::NUMBERED_BY && relation.from == block)
            {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the paragraph `{block}` is already in a list"),
                });
            }
            let text = node.data.get("text").and_then(|text| text.as_str()).unwrap_or_default().to_string();
            let Some((marker, kind)) = marker_of(&text) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the paragraph does not begin with a marker: the markers are {}",
                        MARKERS.iter().map(|(marker, _)| *marker).collect::<Vec<_>>().join(", ")
                    ),
                });
            };
            let length = marker.chars().count();
            // The **list**: a bullet joins the list that is there, and a
            // numbered list always starts a new one.
            let list = match list_of(session, document, kind) {
                Some(list) if is_bullet(kind) => list,
                _ => session.document_mut()?.add_node_with_under(
                    Some(document),
                    kind.to_string(),
                    "prose.list",
                    None,
                    json!({ "kind": kind }),
                ),
            };
            let runs = node.data.get("runs").and_then(|runs| runs.as_array()).cloned().unwrap_or_default();
            let (text, runs) = without_marker(&text, &runs, length);
            let Some(node) = session.document_mut()?.node_mut(block) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{block}`"),
                });
            };
            node.data["text"] = json!(text.clone());
            node.data["runs"] = Json::Array(runs);
            let relation = Relation::with_payload(
                block,
                list,
                crate::numbering::NUMBERED_BY,
                json!({ "list": list, "level": 0 }),
            );
            session.document_mut()?.add_relation(relation);
            Ok(Outcome::changed(format!("the paragraph `{block}` is an item of the list `{list}`"))
                .with_data(json!({ "block": block, "list": list, "kind": kind, "text": text })))

        }
    }
}
