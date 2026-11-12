//! The **revisions** of a block.
//!
//! A run may be marked as a **revision**: it was **inserted**, it was
//! **deleted**, or its **format** was changed — and by whom.
//!
//! The tree's kinds are the tree's own: an insertion, a deletion and a
//! format change. A revision has an **author**, and the tree has a
//! **date** for it as well; the date here is the text the caller
//! gives, because this tree has no clock of its own.
//!
//! And the words of a revision are **derived**: they are the words the
//! run covers.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session};

/// The kinds of revision the tree has.
pub const KINDS: [&str; 3] = ["insert", "delete", "format"];

/// The most characters an author's name may have.
pub const MAX_AUTHOR: usize = 255;

/// The most characters a date may have.
pub const MAX_DATE: usize = 64;

/// The kind a name says, if it says one this tree has.
pub fn kind_of(name: &str) -> Option<&'static str> {
    KINDS.iter().find(|kind| kind.eq_ignore_ascii_case(name.trim())).copied()
}

/// Check the **revision** of a run, and say what is wrong with it if
/// something is.
///
/// A revision is an object with a **kind** — one of the three — and an
/// **author**: a revision nobody made is not a revision. The **date**
/// is optional.
pub fn check(revision: &Json, id: &str) -> Result<(), EngineError> {
    let Some(kind) = revision.get("kind").and_then(|kind| kind.as_str()) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: "a revision says what kind it is".to_string(),
        });
    };
    if kind_of(kind).is_none() {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no revision `{kind}`: the revisions are {}", KINDS.join(", ")),
        });
    }
    let Some(author) = revision.get("author").and_then(|author| author.as_str()) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: "a revision has an author".to_string(),
        });
    };
    if author.trim().is_empty() {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: "a revision has an author".to_string(),
        });
    }
    if author.chars().count() > MAX_AUTHOR {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("an author is at most {MAX_AUTHOR} characters long"),
        });
    }
    if let Some(date) = revision.get("date") {
        let Some(date) = date.as_str() else {
            return Err(EngineError::Failed { id: id.to_string(), message: "a date is a text".to_string() });
        };
        if date.chars().count() > MAX_DATE {
            return Err(EngineError::Failed {
                id: id.to_string(),
                message: format!("a date is at most {MAX_DATE} characters long"),
            });
        }
    }
    Ok(())
}

/// The **revisions** of a block.
///
/// The answer is every run that carries a revision, with its kind, its
/// author, its date and the **words** it covers.
#[derive(Command)]
#[command(id = "prose.block.revisions", area = "prose", layer = "prose", icon = "list")]
pub struct ReadRevisions {
    /// The block.
    pub block: NodeRef,
}

impl ReadRevisions {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.block.revisions".to_string();
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
        let mut revisions: Vec<Json> = Vec::new();
        let mut at = 0usize;
        let runs = node.data.get("runs").and_then(|runs| runs.as_array()).cloned().unwrap_or_default();
        for (index, run) in runs.iter().enumerate() {
            let length = run.get("len").and_then(|len| len.as_u64()).unwrap_or(0) as usize;
            let end = at.saturating_add(length).min(characters.len());
            let covered: String = characters[at.min(characters.len())..end].iter().collect();
            at = end;
            let Some(revision) = run.get("revision") else { continue };
            revisions.push(json!({
                "run": index,
                "kind": revision.get("kind").cloned().unwrap_or(Json::Null),
                "author": revision.get("author").cloned().unwrap_or(Json::Null),
                "date": revision.get("date").cloned().unwrap_or(Json::Null),
                "len": length,
                "text": covered,
            }));
        }
        Ok(Outcome::new(format!("the block `{block}` has {} revision(s)", revisions.len())).with_data(
            json!({
                "block": block,
                "revisions": revisions,
            }),
        ))
    }
}
