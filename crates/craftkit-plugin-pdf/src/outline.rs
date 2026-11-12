//! The **outline**: the bookmarks of a PDF document.
//!
//! A PDF's outline is a tree of **items**: each one has a title, and
//! each one may have children. The file keeps that tree in a shape of
//! its own — a **list** of siblings, each item naming the next, and the
//! last naming none — and the root of the outline names the first of
//! the top-level items.
//!
//! This module reads and writes that tree, and it does it in the
//! battery's own way: the outline is a set of **objects** in the
//! document (each one an indirect object in the file), and the links
//! between them are **edges** of the kind `ref` — the same edges the
//! page tree is made of — which are also **values in the dictionaries**,
//! because the file is written from the dictionaries.
//!
//! Two numbers are **derived**, and not kept:
//!
//! - the **count** of an item is how many items are visible below it,
//!   and the sign says whether it is **open** (positive) or **closed**
//!   (negative) — the tree's rule, which is the rule of the file;
//! - the **count** of the root is how many items are visible at all.
//!
//! # What it does not do
//!
//! - **The outline is not moved.** An item may be added and read, and
//!   that is all: a bookmark that is moved is a bookmark whose siblings
//!   are relinked, and the relinking is the next arc.
//! - **The count is the visible count.** An item that is closed shows
//!   nothing below it, and the count says so — but an item is only
//!   closed because its count is negative, and nothing here makes an
//!   item closed.

use std::collections::{HashMap, HashSet};

use craftkit_derive::commands;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session, Text};

use crate::{a_reference, add_reference, entries_of, pairs_of, the_next_number, the_number_of};

/// How many items an outline may hold.
///
/// An outline comes from a file, and a file may be hostile: the count
/// is a bound, and a walk that stops is a walk that finishes.
const MAX_OUTLINE_ITEMS: usize = 4096;

/// How deep an outline may be.
const MAX_OUTLINE_DEPTH: usize = 64;

commands! {
    /// Put a **bookmark** in the outline.
    ///
    /// The bookmark is a new object with a **title**, and it hangs under a
    /// parent: the outline's root, or another bookmark. It is put at the
    /// end of its parent's list, and the list is relinked: the parent names
    /// it as its last, and the item that was last names it as its next.
    pdf.outline.add {
        area: "pdf",
        layer: "pdf",
        icon: "plus",
        mutates,
        undoable,
        params: {
            /// The bookmark's title.

            title: Text,
            /// The page the bookmark goes to.

            destination: Option<NodeRef>,
            /// The bookmark it goes under: the outline's root, or another
            /// bookmark.

            parent: Option<NodeRef>,
            /// The object number of the bookmark in the file.
            ///
            /// Leave it out and the next free number is used.

            object: Option<Number>,
        },
        run: {

            let id = "pdf.outline.add".to_string();
            let title = self.title.0.trim().to_string();
            if title.is_empty() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a bookmark has a title".to_string(),
                });
            }
            // The **page** the bookmark goes to.
            let destination = match self.destination {
                Some(page) => {
                    let node = session.document().node(page.0).ok_or_else(|| EngineError::Failed {
                        id: id.clone(),
                        message: format!("there is no node `{}`", page.0),
                    })?;
                    let page_number = the_number_of(node).ok_or_else(|| EngineError::Failed {
                        id: id.clone(),
                        message: format!("the node `{}` has no object number", page.0),
                    })?;
                    Some((page.0, page_number))
                }
                None => None,
            };
            // The **parent**: the outline's root, or another bookmark. The
            // root is made when the document has no outline yet — an
            // outline hangs from the catalog, and a document without a
            // catalog has nowhere to hang it.
            let (parent, root) = match self.parent {
                Some(parent) => {
                    if session.document().node(parent.0).is_none() {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("there is no node `{}`", parent.0),
                        });
                    }
                    let root = the_root_of(session, parent.0).ok_or_else(|| EngineError::Failed {
                        id: id.clone(),
                        message: format!("the node `{}` is not in an outline", parent.0),
                    })?;
                    (parent.0, root)
                }
                None => {
                    let root = match the_root(session) {
                        Some(root) => root,
                        None => {
                            let catalog = the_catalog(session).ok_or_else(|| EngineError::Failed {
                                id: id.clone(),
                                message: "the document has no catalog, and an outline hangs from it".to_string(),
                            })?;
                            let root_number = the_next_number(session.document(), 0);
                            let root_data = serde_json::json!({
                                "object": root_number,
                                "entries": pairs_of(&[("Type".to_string(), serde_json::json!("Outlines"))]),
                            });
                            let root =
                                session.document_mut()?.add_node_with("outline", "pdf.object", None, root_data);
                            // The **catalog** names the outline.
                            let reference = a_reference(session, root, 0, &id)?;
                            add_reference(session, catalog, "Outlines", reference, false)?;
                            let relation = craftkit_sdk::Relation::with_payload(
                                catalog,
                                root,
                                "ref",
                                serde_json::json!({ "key": "Outlines", "generation": 0 }),
                            );
                            session.document_mut()?.add_relation(relation);
                            root
                        }
                    };
                    (root, root)
                }
            };
            // The **object number** of the bookmark: a whole number of its
            // own, and one that nothing else has.
            //
            // It is worked out **after** the parent, because the outline
            // may have been made on the way: the root is an object of the
            // file too, and it takes a number of its own.
            let number = match self.object {
                Some(number) => {
                    let number = number.0;
                    if !number.is_finite() || number < 0.0 || number.fract() != 0.0 {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: "an object number is a whole number".to_string(),
                        });
                    }
                    let number = number as u64;
                    if number == 0 {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: "object numbers start at one".to_string(),
                        });
                    }
                    if session.document().walk().any(|node| the_number_of(node) == Some(number)) {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("the object {number} is already in the document"),
                        });
                    }
                    number
                }
                None => the_next_number(session.document(), 0),
            };
            let parent_number = the_number_of(session.document().node(parent).ok_or_else(|| {
                EngineError::Failed { id: id.clone(), message: format!("there is no node `{parent}`") }
            })?)
            .ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{parent}` has no object number"),
            })?;
            // The **bookmark** itself.
            let mut entries: Vec<(String, serde_json::Value)> = vec![
                ("Title".to_string(), serde_json::json!({ "string": title })),
                ("Parent".to_string(), serde_json::json!({ "ref": [parent_number, 0] })),
            ];
            if let Some((_, page_number)) = destination {
                entries.push(("Dest".to_string(), serde_json::json!([{ "ref": [page_number, 0] }, "Fit"])));
            }
            let data = serde_json::json!({ "object": number, "entries": pairs_of(&entries) });
            let item = session.document_mut()?.add_node_with(title.clone(), "pdf.object", None, data);
            // The **list** of the parent's children: the new bookmark goes
            // at the end of it, and the list is relinked.
            let children = the_children(session, parent);
            let last = children.last().copied();
            match last {
                Some(last) => {
                    let last_number = the_number_of(session.document().node(last).ok_or_else(|| {
                        EngineError::Failed { id: id.clone(), message: format!("there is no node `{last}`") }
                    })?)
                    .ok_or_else(|| EngineError::Failed {
                        id: id.clone(),
                        message: format!("the node `{last}` has no object number"),
                    })?;
                    // The item that was last names the new one as its next.
                    add_reference(session, last, "Next", serde_json::json!({ "ref": [number, 0] }), false)?;
                    let relation = craftkit_sdk::Relation::with_payload(
                        last,
                        item,
                        "ref",
                        serde_json::json!({ "key": "Next", "generation": 0 }),
                    );
                    session.document_mut()?.add_relation(relation);
                    // And the new one names the old last as its previous.
                    add_reference(session, item, "Prev", serde_json::json!({ "ref": [last_number, 0] }), false)?;
                    let relation = craftkit_sdk::Relation::with_payload(
                        item,
                        last,
                        "ref",
                        serde_json::json!({ "key": "Prev", "generation": 0 }),
                    );
                    session.document_mut()?.add_relation(relation);
                }
                None => {
                    // The parent had no children, and the new one is the
                    // first.
                    add_reference(session, parent, "First", serde_json::json!({ "ref": [number, 0] }), false)?;
                    let relation = craftkit_sdk::Relation::with_payload(
                        parent,
                        item,
                        "ref",
                        serde_json::json!({ "key": "First", "generation": 0 }),
                    );
                    session.document_mut()?.add_relation(relation);
                }
            }
            // The parent names the new one as its last.
            add_reference(session, parent, "Last", serde_json::json!({ "ref": [number, 0] }), false)?;
            let relation = craftkit_sdk::Relation::with_payload(
                parent,
                item,
                "ref",
                serde_json::json!({ "key": "Last", "generation": 0 }),
            );
            session.document_mut()?.add_relation(relation);
            // And the new one names its parent.
            let relation = craftkit_sdk::Relation::with_payload(
                item,
                parent,
                "ref",
                serde_json::json!({ "key": "Parent", "generation": 0 }),
            );
            session.document_mut()?.add_relation(relation);
            // A parent that was **closed** is opened: it has a child now,
            // and a child that cannot be seen is a child that is not there.
            if parent != root {
                let count = the_count(session, parent);
                let closed = count.map(|count| count <= 0).unwrap_or(true);
                if closed {
                    let node = session.document_mut()?.node_mut(parent);
                    if let Some(node) = node {
                        node.data = crate::with_entry(&node.data, "Count", serde_json::json!(1));
                    }
                }
            }
            // The **counts** are derived, and they are worked out again.
            recount(session, root)?;
            Ok(Outcome::changed(format!("added the bookmark `{title}`")).with_data(serde_json::json!({
                "id": item,
                "object": number,
                "title": title,
                "parent": parent,
                "root": root,
                "destination": destination.map(|(page, _)| page),
            })))

        }
    }
    /// Read the **outline**: the bookmarks, in the order they are in.
    ///
    /// A query: it does not change the document.
    pdf.outline.read {
        area: "pdf",
        layer: "pdf",
        icon: "list",
        params: {
            /// The root of the outline.
            ///
            /// Leave it out and the outline of the document's catalog is read.

            root: Option<NodeRef>,
        },
        run: {

            let id = "pdf.outline.read".to_string();
            let root = match self.root {
                Some(root) => {
                    if session.document().node(root.0).is_none() {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("there is no node `{}`", root.0),
                        });
                    }
                    root.0
                }
                None => the_root(session).ok_or_else(|| EngineError::Failed {
                    id: id.clone(),
                    message: "the document has no outline".to_string(),
                })?,
            };
            let items = the_tree(session, root, 0);
            let count = items.len();
            Ok(Outcome::new(format!("{count} bookmark(s)"))
                .with_data(serde_json::json!({ "root": root, "items": items })))

        }
    }
}

/// The **catalog** of the document: the object that says it is one.
fn the_catalog(session: &Session) -> Option<u64> {
    session
        .document()
        .walk()
        .find(|node| {
            node.kind == "pdf.object"
                && entries_of(&node.data)
                    .iter()
                    .any(|(key, value)| key == "Type" && value.as_str() == Some("Catalog"))
        })
        .map(|node| node.id)
}

/// The **root** of the document's outline: the object the catalog names
/// under `/Outlines`.
fn the_root(session: &Session) -> Option<u64> {
    let catalog = the_catalog(session)?;
    let node = session.document().node(catalog)?;
    let value =
        entries_of(&node.data).into_iter().find(|(key, _)| key == "Outlines").map(|(_, value)| value)?;
    the_node_of(session, &value)
}

/// The **root** of the outline a node is in.
///
/// The walk goes up the `/Parent` links, and it stops: a broken
/// outline is not followed for ever.
fn the_root_of(session: &Session, node: u64) -> Option<u64> {
    let mut current = node;
    let mut seen = HashSet::new();
    for _ in 0..MAX_OUTLINE_DEPTH {
        if !seen.insert(current) {
            return None;
        }
        if is_the_root(session, current) {
            return Some(current);
        }
        current = the_parent(session, current)?;
    }
    None
}

/// Whether a node is the **root** of an outline: it says it is one.
fn is_the_root(session: &Session, node: u64) -> bool {
    session
        .document()
        .node(node)
        .map(|node| {
            entries_of(&node.data)
                .iter()
                .any(|(key, value)| key == "Type" && value.as_str() == Some("Outlines"))
        })
        .unwrap_or(false)
}

/// The **parent** of an item: the one it names under `/Parent`.
fn the_parent(session: &Session, node: u64) -> Option<u64> {
    let value = the_value(session, node, "Parent")?;
    the_node_of(session, &value)
}

/// The **children** of a node: the list of its siblings' first, and then
/// the next of each.
///
/// The walk is cycle-safe: a broken outline is not followed for ever.
fn the_children(session: &Session, node: u64) -> Vec<u64> {
    let mut children = Vec::new();
    let mut seen = HashSet::new();
    let mut current = the_value(session, node, "First").and_then(|value| the_node_of(session, &value));
    while let Some(child) = current {
        if !seen.insert(child) || children.len() >= MAX_OUTLINE_ITEMS {
            break;
        }
        children.push(child);
        current = the_value(session, child, "Next").and_then(|value| the_node_of(session, &value));
    }
    children
}

/// The **count** of a node: what its dictionary says.
fn the_count(session: &Session, node: u64) -> Option<i64> {
    the_value(session, node, "Count").and_then(|value| the_whole(&value))
}

/// The **value** under a key of a node's dictionary.
fn the_value(session: &Session, node: u64, key: &str) -> Option<serde_json::Value> {
    let node = session.document().node(node)?;
    entries_of(&node.data).into_iter().find(|(name, _)| name == key).map(|(_, value)| value)
}

/// The **node** a reference names.
fn the_node_of(session: &Session, value: &serde_json::Value) -> Option<u64> {
    let number = value.get("ref").and_then(|pair| pair.as_array()).and_then(|pair| pair.first())?;
    let number = the_whole(number)?;
    if number < 0 {
        return None;
    }
    session.document().walk().find(|node| the_number_of(node) == Some(number as u64)).map(|node| node.id)
}

/// A **whole number**, from a value that may hold a float.
///
/// A number that came from a file's bytes is a float — the bytes are
/// read as bytes, and a number in them is a number and not an integer
/// until somebody says so — and a number that was made here is an
/// integer. Both are numbers, and both are read the same way.
fn the_whole(value: &serde_json::Value) -> Option<i64> {
    crate::whole_number_of(value)
}

/// Work the **counts** out again.
///
/// The count of a node is how many items are **visible** below it: an
/// item that is open shows its children, and a child that is closed
/// shows nothing. The sign says whether the node is open or closed, and
/// the root is always open.
fn recount(session: &mut Session, root: u64) -> Result<(), EngineError> {
    let tree = collect(session, root);
    // The **open** nodes: the sign of the count they have says so, and
    // the root is open whatever it says.
    let mut open: HashSet<u64> = HashSet::new();
    for (node, children) in &tree {
        if children.is_empty() {
            continue;
        }
        if *node == root || the_count(session, *node).map(|count| count > 0).unwrap_or(false) {
            open.insert(*node);
        }
    }
    // The **visible** count of every node, from the leaves up.
    let mut below: HashMap<u64, u64> = HashMap::new();
    for (node, children) in tree.iter().rev() {
        let mut visible = 0_u64;
        for child in children {
            visible += 1 + if open.contains(child) { below.get(child).copied().unwrap_or(0) } else { 0 };
        }
        below.insert(*node, visible);
    }
    // And the counts are written down.
    for (node, children) in &tree {
        if children.is_empty() {
            continue;
        }
        let visible = below.get(node).copied().unwrap_or(0) as i64;
        let count = if *node == root || open.contains(node) { visible } else { -visible };
        if let Some(entry) = session.document_mut()?.node_mut(*node) {
            entry.data = crate::with_entry(&entry.data, "Count", serde_json::json!(count));
        }
    }
    Ok(())
}

/// The **number** of the bookmarks of the file.
///
/// The tree's own rule: the items of the outline, and not the node
/// that holds them.
pub fn count_of(session: &Session) -> usize {
    let Some(root) = the_root(session) else { return 0 };
    collect(session, root).len().saturating_sub(1)
}

/// Every node of an outline, with its children: the tree, from the top.
fn collect(session: &Session, root: u64) -> Vec<(u64, Vec<u64>)> {
    let mut tree: Vec<(u64, Vec<u64>)> = Vec::new();
    let mut seen = HashSet::new();
    let mut stack: Vec<(u64, usize)> = vec![(root, 0)];
    while let Some((node, depth)) = stack.pop() {
        if depth > MAX_OUTLINE_DEPTH || tree.len() >= MAX_OUTLINE_ITEMS {
            continue;
        }
        if !seen.insert(node) {
            continue;
        }
        let children = the_children(session, node);
        tree.push((node, children.clone()));
        for child in children.into_iter().rev() {
            stack.push((child, depth + 1));
        }
    }
    tree
}

/// The **tree** of bookmarks, as the caller gets it.
fn the_tree(session: &Session, node: u64, depth: usize) -> Vec<serde_json::Value> {
    if depth > MAX_OUTLINE_DEPTH {
        return Vec::new();
    }
    let mut items = Vec::new();
    for child in the_children(session, node) {
        let node_data = session.document().node(child);
        let title = node_data
            .and_then(|node| {
                entries_of(&node.data).into_iter().find(|(key, _)| key == "Title").and_then(|(_, value)| {
                    value.get("string").and_then(|text| text.as_str()).map(|text| text.to_string())
                })
            })
            .unwrap_or_default();
        let destination = node_data.and_then(|node| {
            entries_of(&node.data).into_iter().find(|(key, _)| key == "Dest").and_then(|(_, value)| {
                value
                    .as_array()
                    .and_then(|list| list.first())
                    .and_then(|first| first.get("ref"))
                    .and_then(|pair| pair.as_array())
                    .and_then(|pair| pair.first())
                    .and_then(the_whole)
            })
        });
        let count = the_count(session, child);
        let mut item = serde_json::Map::new();
        item.insert("id".to_string(), serde_json::json!(child));
        item.insert("title".to_string(), serde_json::json!(title));
        item.insert("count".to_string(), serde_json::json!(count));
        item.insert("destination".to_string(), serde_json::json!(destination));
        item.insert("children".to_string(), serde_json::json!(the_tree(session, child, depth + 1)));
        items.push(serde_json::Value::Object(item));
    }
    items
}
