//! The **endnotes** of a document.
//!
//! An endnote is a note that belongs to the **end** of the document,
//! where a footnote belongs to the foot of the page. The text that
//! points at it is a **reference** — a run that names the endnote —
//! and the **number** of an endnote is **derived**: it is where the
//! endnote is referenced in the document, and it is counted **apart**
//! from the footnotes, as the tree counts them apart.
//!
//! The rules:
//!
//! - An endnote is a **node**, of the kind `prose.endnote`, and a
//!   paragraph may point at it.
//! - A run that carries an **endnote** points at an endnote: a run
//!   that names a node that is not an endnote is refused.
//! - The **number** of an endnote is derived, and the endnotes are
//!   counted apart from the footnotes: an endnote that is the first
//!   endnote is `1` even when three footnotes came before it.

use craftkit_derive::{Command, NodeKind};

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Node, NodeRef, Outcome, Session, Text};

/// The most characters an endnote may have.
pub const MAX_TEXT: usize = 32_767;

/// An **endnote**: a note that belongs to the end of the document.
#[derive(NodeKind)]
#[node_kind(kind = "prose.endnote", label = "Endnote", layer = "prose", evaluates)]
pub struct EndnoteKind;

impl EndnoteKind {
    /// Not drawn.
    fn paint(_node: &Node, _document: &Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What an endnote **means**: what it says.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        Some(json!({ "text": node.data.get("text").cloned().unwrap_or(Json::Null) }))
    }
}

/// Put an **endnote** in a document.
///
/// The endnote is a **node**: it is few, and a paragraph may point at
/// it.
#[derive(Command)]
#[command(id = "prose.endnote.add", area = "prose", layer = "prose", icon = "plus", mutates, undoable)]
pub struct AddEndnote {
    /// The document it goes in.
    pub document: NodeRef,
    /// What it says.
    pub text: Text,
}

impl AddEndnote {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.endnote.add".to_string();
        let document = self.document.0;
        let Some(node) = session.document().node(document) else {
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
        let text = self.text.0.clone();
        if text.chars().count() > MAX_TEXT {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("an endnote is at most {MAX_TEXT} characters long"),
            });
        }
        let endnote = session.document_mut()?.add_node_with_under(
            Some(document),
            "an endnote".to_string(),
            "prose.endnote",
            None,
            json!({ "text": text.clone() }),
        );
        Ok(Outcome::changed(format!("the document `{document}` has an endnote"))
            .with_data(json!({ "id": endnote, "document": document, "text": text })))
    }
}

/// **Read** the endnotes of a block.
///
/// The answer is every run that points at an endnote, with the
/// **number** of the endnote — derived, and counted apart from the
/// footnotes — and what it says.
#[derive(Command)]
#[command(id = "prose.block.endnotes", area = "prose", layer = "prose", icon = "list")]
pub struct ReadEndnotes {
    /// The block.
    pub block: NodeRef,
}

impl ReadEndnotes {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.block.endnotes".to_string();
        let block = self.block.0;
        let Some(node) = session.document().node(block) else {
            return Err(EngineError::Failed { id, message: format!("there is no block `{block}`") });
        };
        if !node.kind.starts_with("prose.block.") {
            return Err(EngineError::Failed {
                id,
                message: format!("the node `{block}` is a `{}` and not a block", node.kind),
            });
        }
        let text = node.data.get("text").and_then(|text| text.as_str()).unwrap_or_default().to_string();
        let characters: Vec<char> = text.chars().collect();
        let document = session.document();
        let numbers = crate::footnotes::numbers(document);
        let mut endnotes: Vec<Json> = Vec::new();
        let mut at = 0usize;
        let runs = node.data.get("runs").and_then(|runs| runs.as_array()).cloned().unwrap_or_default();
        for (index, run) in runs.iter().enumerate() {
            let length = run.get("len").and_then(|len| len.as_u64()).unwrap_or(0) as usize;
            let end = at.saturating_add(length).min(characters.len());
            let covered: String = characters[at.min(characters.len())..end].iter().collect();
            at = end;
            let Some(endnote) = run.get("endnote").and_then(|endnote| endnote.as_u64()) else { continue };
            let said =
                document.node(endnote).and_then(|node| node.data.get("text").cloned()).unwrap_or(Json::Null);
            let number = numbers.get(&endnote).copied().map(Json::from).unwrap_or(Json::Null);
            endnotes.push(json!({
                "run": index,
                "endnote": endnote,
                "number": number,
                "text": said,
                "reference": covered,
            }));
        }
        Ok(Outcome::new(format!("the block `{block}` has {} endnote(s)", endnotes.len())).with_data(json!({
            "block": block,
            "endnotes": endnotes,
        })))
    }
}
