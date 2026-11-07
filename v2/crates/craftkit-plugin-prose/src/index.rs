//! The **index** of a document.
//!
//! An index is the list of the **entries** a document has been marked
//! with. A mark is a fact of the **block** it is on — the tree's mark
//! is an inline object in the paragraph, and here it is the block's own
//! payload — and the **index of a document** is derived: it is the
//! marks of its blocks, gathered under the letters of the alphabet.
//!
//! # The rules
//!
//! - **A mark is a pair**: the entry and the subentry under it, and a
//!   mark with no subentry is the entry itself.
//! - **The same mark twice is one mark.** A block marked with an entry
//!   twice is a block marked with it once.
//! - **An empty entry takes the marks away.** An entry that says
//!   nothing is not an entry, and it takes every mark of the block
//!   with it.
//! - **The groups are the letters.** An entry that begins with a
//!   letter is under that letter, uppercased, and an entry that begins
//!   with anything else is under that character.
//! - **The letters come first.** The groups of the letters are in the
//!   alphabet's order, and the groups of the other characters come
//!   after them.
//!
//! # What it does not do
//!
//! - **The index is not laid out.** The tree's index is a field in the
//!   document, and it carries the **page** every entry is on. Here
//!   there is no layout, and the entry names the **block** it is on.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, NodeRef, Outcome, Session, Text};

/// The most characters an entry may have.
pub const MAX_ENTRY: usize = 200;

/// The **marks** of a block: the entries it is marked with.
pub fn marks_of(node: &craftkit_sdk::Node) -> Vec<Json> {
    let Some(marks) = node.data.get("marks").and_then(|marks| marks.as_array()) else {
        return Vec::new();
    };
    marks
        .iter()
        .filter_map(|mark| {
            let entry = mark.get("entry")?.as_str()?.to_string();
            let subentry =
                mark.get("subentry").and_then(|subentry| subentry.as_str()).map(|text| text.to_string());
            Some(json!({ "entry": entry, "subentry": subentry }))
        })
        .collect()
}

/// The **letter** an entry is under.
///
/// A letter is uppercased, and anything else is itself.
fn letter_of(entry: &str) -> String {
    let Some(first) = entry.chars().next() else { return String::new() };
    if first.is_alphabetic() { first.to_uppercase().to_string() } else { first.to_string() }
}

/// Whether a **letter** comes before another one.
///
/// The letters of the alphabet come first, and the other characters
/// come after them.
fn letter_is_before(one: &str, other: &str) -> bool {
    let one_is_a_letter = one.chars().all(|character| character.is_alphabetic()) && !one.is_empty();
    let other_is_a_letter = other.chars().all(|character| character.is_alphabetic()) && !other.is_empty();
    match (one_is_a_letter, other_is_a_letter) {
        (true, true) => one < other,
        (true, false) => true,
        (false, true) => false,
        (false, false) => one < other,
    }
}

/// One **entry** of an index: what it says, where it is, and the
/// subentries under it.
struct Entry {
    /// The entry itself.
    entry: String,
    /// The subentry under it, if it has one.
    subentry: String,
    /// The blocks it is marked on.
    blocks: Vec<u64>,
    /// The subentries under it, and the blocks they are on.
    subentries: Vec<(String, Vec<u64>)>,
}

/// The **index** of a document: the groups of the entries, by their
/// letter.
///
/// The groups are derived from the marks of the blocks, and the blocks
/// are walked in the order of the document.
pub fn index_of(document: &Document, document_node: u64) -> Vec<Json> {
    let Some(node) = document.node(document_node) else { return Vec::new() };
    // The **entries**: the entry, the blocks it is on, and the
    // subentries under it.
    let mut entries: Vec<Entry> = Vec::new();
    for block in node.children.iter().filter(|child| child.kind.starts_with("prose.block.")) {
        for mark in marks_of(block) {
            let Some(entry) = mark.get("entry").and_then(|entry| entry.as_str()) else { continue };
            let subentry = mark.get("subentry").and_then(|subentry| subentry.as_str()).unwrap_or_default();
            match entries.iter_mut().find(|held| held.entry == entry && held.subentry == subentry) {
                Some(held) => {
                    if !held.blocks.contains(&block.id) {
                        held.blocks.push(block.id);
                    }
                }
                None => {
                    let subentries: Vec<(String, Vec<u64>)> = if subentry.is_empty() {
                        Vec::new()
                    } else {
                        vec![(subentry.to_string(), vec![block.id])]
                    };
                    entries.push(Entry {
                        entry: entry.to_string(),
                        subentry: subentry.to_string(),
                        blocks: vec![block.id],
                        subentries,
                    });
                }
            }
        }
    }
    // The **groups**: the letter and the entries under it.
    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
    for (at, entry) in entries.iter().enumerate() {
        let letter = letter_of(&entry.entry);
        match groups.iter_mut().find(|(held, _)| *held == letter) {
            Some((_, held)) => held.push(at),
            None => groups.push((letter, vec![at])),
        }
    }
    groups.sort_by(|one, other| {
        if letter_is_before(&one.0, &other.0) {
            std::cmp::Ordering::Less
        } else if letter_is_before(&other.0, &one.0) {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    });
    let mut out: Vec<Json> = Vec::new();
    for (letter, held) in groups {
        // The **entries** of the group, in the alphabet's order.
        let mut sorted: Vec<usize> = held.clone();
        sorted.sort_by_key(|at| {
            let entry = &entries[*at].entry;
            (entry.to_lowercase(), entry.clone())
        });
        let listed: Vec<Json> = sorted
            .iter()
            .map(|at| {
                let entry = &entries[*at];
                let mut subentries = entry.subentries.clone();
                subentries.sort_by_key(|(subentry, _)| subentry.to_lowercase());
                let subentries: Vec<Json> = subentries
                    .iter()
                    .map(|(subentry, blocks)| json!({ "subentry": subentry, "blocks": blocks }))
                    .collect();
                json!({
                    "entry": entry.entry,
                    "subentry": entry.subentry,
                    "blocks": entry.blocks,
                    "subentries": subentries,
                })
            })
            .collect();
        out.push(json!({ "letter": letter, "entries": listed }));
    }
    out
}

/// **Mark** a block with an index **entry**.
///
/// The entry may have a **subentry** under it. An entry that says
/// nothing takes every mark of the block away.
#[derive(Command)]
#[command(id = "prose.index.mark", area = "prose", layer = "prose", icon = "tag", mutates, undoable)]
pub struct MarkIndex {
    /// The block.
    pub block: NodeRef,
    /// The entry.
    pub entry: Text,
    /// The subentry under the entry.
    pub subentry: Option<Text>,
}

impl MarkIndex {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.index.mark".to_string();
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
        let entry = self.entry.0.trim().to_string();
        let subentry =
            self.subentry.as_ref().map(|subentry| subentry.0.trim().to_string()).unwrap_or_default();
        if entry.chars().count() > MAX_ENTRY {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("an entry is at most {MAX_ENTRY} characters long"),
            });
        }
        // An entry that says **nothing** takes the marks away.
        if entry.is_empty() {
            let Some(node) = session.document_mut()?.node_mut(block) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no block `{block}`"),
                });
            };
            let had = marks_of(node).len();
            node.data["marks"] = Json::Array(Vec::new());
            return Ok(Outcome::changed(format!("the block `{block}` is not marked at all"))
                .with_data(json!({ "block": block, "taken": had })));
        }
        // The **marks** of the block: the mark is added, and the same
        // mark twice is one mark.
        let mut marks: Vec<Json> = marks_of(node);
        let held = marks.iter().any(|mark| {
            mark.get("entry").and_then(|entry| entry.as_str()) == Some(entry.as_str())
                && mark.get("subentry").and_then(|subentry| subentry.as_str()).unwrap_or_default()
                    == subentry.as_str()
        });
        if !held {
            marks.push(json!({ "entry": entry.clone(), "subentry": subentry.clone() }));
        }
        let Some(node) = session.document_mut()?.node_mut(block) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no block `{block}`"),
            });
        };
        node.data["marks"] = Json::Array(marks.clone());
        Ok(Outcome::changed(format!("the block `{block}` is marked `{entry}`")).with_data(json!({
            "block": block,
            "entry": entry,
            "subentry": subentry,
            "marks": marks.len(),
        })))
    }
}

/// **Read** the **index** of a document.
///
/// The index is **derived**: it is the marks of the blocks of the
/// document, gathered under the letters of the alphabet.
#[derive(Command)]
#[command(id = "prose.index.read", area = "prose", layer = "prose", icon = "list")]
pub struct ReadIndex {
    /// The document.
    pub document: NodeRef,
}

impl ReadIndex {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.index.read".to_string();
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
        let groups = index_of(session.document(), document);
        let entries: usize = groups
            .iter()
            .map(|group| {
                group.get("entries").and_then(|entries| entries.as_array()).map(Vec::len).unwrap_or(0)
            })
            .sum();
        Ok(Outcome::new(format!("the document `{document}` has {entries} indexed entry(ies)")).with_data(
            json!({
                "document": document,
                "groups": groups,
                "entries": entries,
            }),
        ))
    }
}
