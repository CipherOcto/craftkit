//! The **table of authorities** of a document.
//!
//! A table of authorities is the list of the **authorities** a document
//! has been marked with: the cases, the statutes and the other things a
//! document cites. The tree's table is a field in the document and it
//! is rebuilt when the document changes; here it is **derived**: it is
//! read out of the marks, and it cannot be out of date.
//!
//! # The rules
//!
//! These are the tree's own rules.
//!
//! - An **entry** is the citation of the authority, and it says
//!   something: an entry that says nothing is refused.
//! - The **quotes** are taken out of the entry, as the tree takes them
//!   out of the citation it marks.
//! - The **same entry twice is one entry.** The authorities of a
//!   document are gathered by their entry, and the blocks an entry is
//!   marked on are listed once each.
//! - The entries are **sorted**, as the tree sorts them.
//!
//! # What it does not do
//!
//! - **The table is not laid out.** The tree's table ends every entry
//!   with the **page** it is on, and it has a tab leader and a
//!   right-aligned page number. There is no layout here, and so there
//!   is no page to show.
//! - **The tree's table has categories** — cases, statutes and the
//!   other kinds of authority — and the tree's mark does not say which
//!   one it is. There is no category here either.
//! - **The tree's collation** is the order of the language, and it is
//!   not here: the entries are compared as their text is.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Outcome, Text};

/// The most characters an entry may have.
pub const MAX_ENTRY: usize = 200;

/// The **marks** of a block: the authorities it is marked with.
pub fn authorities_of(node: &craftkit_sdk::Node) -> Vec<String> {
    node.data
        .get("authorities")
        .and_then(|marks| marks.as_array())
        .map(|marks| marks.iter().filter_map(|mark| mark.as_str().map(|mark| mark.to_string())).collect())
        .unwrap_or_default()
}

/// The **entry** a mark carries: the citation, without its quotes.
fn entry_of(entry: &str) -> String {
    entry.replace('"', "")
}

/// The **authorities** of a document: the entries, gathered, and the
/// blocks they are on.
///
/// The entries are derived from the marks of the blocks, and the blocks
/// are walked in the order of the document.
pub fn authorities_of_document(document: &craftkit_sdk::Document, document_node: u64) -> Vec<Json> {
    let Some(node) = document.node(document_node) else { return Vec::new() };
    // The **entries**: the entry and the blocks it is marked on.
    let mut entries: Vec<(String, Vec<u64>)> = Vec::new();
    for block in node.children.iter().filter(|child| child.kind.starts_with("prose.block.")) {
        for entry in authorities_of(block) {
            match entries.iter_mut().find(|(held, _)| *held == entry) {
                Some((_, blocks)) => {
                    if !blocks.contains(&block.id) {
                        blocks.push(block.id);
                    }
                }
                None => entries.push((entry, vec![block.id])),
            }
        }
    }
    // The **sort**: the entries as their text is, and then the text
    // itself, so that two entries of the same letters keep a place of
    // their own.
    entries.sort_by(|one, other| {
        let one_lower = one.0.to_lowercase();
        let other_lower = other.0.to_lowercase();
        one_lower.cmp(&other_lower).then_with(|| one.0.cmp(&other.0))
    });
    entries.into_iter().map(|(entry, blocks)| json!({ "entry": entry, "blocks": blocks })).collect()
}

commands! {
    /// **Mark** a block with an authority.
    prose.authority.mark {
        area: "prose",
        layer: "prose",
        icon: "tag",
        mutates,
        undoable,
        params: {
            /// The block.

            block: NodeRef,
            /// The citation of the authority.

            entry: Text,
        },
        run: {

            let id = "prose.authority.mark".to_string();
            let block = self.block.0;
            let Some(node) = session.document().node(block) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no block `{block}`"),
                });
            };
            if !node.kind.starts_with("prose.block.") {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{block}` is a `{}` and not a block", node.kind),
                });
            }
            // The **entry**: the citation, without its quotes, and it says
            // something.
            let entry = entry_of(self.entry.0.trim());
            if entry.is_empty() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "an authority says what it is".to_string(),
                });
            }
            if entry.chars().count() > MAX_ENTRY {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("an entry is at most {MAX_ENTRY} characters long"),
                });
            }
            // The **mark**: the same entry twice on one block is one mark.
            let mut marks = authorities_of(node);
            let added = !marks.contains(&entry);
            if added {
                marks.push(entry.clone());
            }
            if let Some(node) = session.document_mut()?.node_mut(block) {
                node.data["authorities"] = json!(marks);
            }
            Ok(Outcome::changed(format!("the block `{block}` is marked with `{entry}`")).with_data(json!({
                "block": block,
                "entry": entry,
                "added": added,
            })))

        }
    }
    /// The **table of authorities** of a document: the entries it has been
    /// marked with, and the blocks they are on.
    prose.document.authorities {
        area: "prose",
        layer: "prose",
        icon: "list",
        params: {
            /// The document.

            document: NodeRef,
        },
        run: {

            let command = "prose.document.authorities";
            let document = self.document.0;
            if session.document().node(document).is_none() {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: format!("there is no document `{document}`"),
                });
            }
            let authorities = authorities_of_document(session.document(), document);
            Ok(Outcome::new(format!("the document `{document}` holds {} authority(ies)", authorities.len()))
                .with_data(json!({
                    "document": document,
                    "authorities": authorities,
                    "count": authorities.len(),
                })))

        }
    }
}
