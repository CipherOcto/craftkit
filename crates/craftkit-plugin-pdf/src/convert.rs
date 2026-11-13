//! The **convert** of a PDF file.
//!
//! The tree's preflight says what a file breaks, and the tree's
//! `convert` **fixes** what it can: the actions a PDF/A file may not
//! have, the flags that ask a viewer to do the work of the file, and
//! the metadata that says what the file is. What it cannot fix stays,
//! and the report says what stays.
//!
//! What is here is what this tree can do to a **document** without
//! writing anything new:
//!
//! - the **additional actions** (`/AA`) are taken away from every
//!   dictionary that has them;
//! - the **forbidden actions** — the actions of the tree's own list —
//!   are taken away, and so are the entries that point at them;
//! - the **NeedAppearances** flag is taken away: it asks the viewer
//!   to make the appearances of the form, and a PDF/A file must not
//!   ask;
//! - the **images** are asked for nothing: the interpolation is
//!   cleared, and the alternates and the OPI information go away.
//!
//! What is **not** here is what needs something this tree does not
//! have, or what another command does better:
//!
//! - the **encryption** is not taken away: the tree decrypts a file
//!   when it opens it, and this tree cannot, so a file whose strings
//!   are still encrypted would be broken by the taking;
//! - the **metadata** is not written: a PDF/A file identifies itself
//!   in an XMP packet, and the packet is written by its own command
//!   (§536) — the bytes of a stream are bytes in a file, and the
//!   conversion does not name a file;
//! - the **output intent** is not written: it names an ICC profile,
//!   and the profile is written by its own command (§537).
//!
//! So the report says what was fixed and what is left, and the
//! preflight's own answer is the second list.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Node, Outcome, Text};

use crate::preflight::{self, Level};

/// The **object number** a reference names.
fn number_of(value: &Json) -> Option<u64> {
    crate::number_of(value)
}

/// The **entries** of a node's dictionary, without the key.
fn without(entries: &[(String, Json)], key: &str) -> Vec<(String, Json)> {
    entries.iter().filter(|(name, _)| name != key).cloned().collect()
}

/// Take a **key** away from a node's dictionary.
///
/// The key is taken away and the dictionary is written again, keeping
/// the order of the keys that are left.
fn take_away(node: &mut Node, key: &str) -> bool {
    let entries = crate::entries_of(&node.data);
    if !entries.iter().any(|(name, _)| name == key) {
        return false;
    }
    let entries = without(&entries, key);
    node.data = json!({
        "object": node.data.get("object").cloned().unwrap_or(Json::Null),
        "entries": entries
            .iter()
            .map(|(key, value)| json!([key, value]))
            .collect::<Vec<_>>(),
    });
    true
}

/// The **nodes** of the document that are objects of the file.
fn the_objects(document: &Document) -> Vec<u64> {
    document
        .walk()
        .filter(|node| node.kind == "pdf.object" || node.kind == "pdf.stream")
        .map(|node| node.id)
        .collect()
}

commands! {
    /// **Convert** a PDF file: fix what can be fixed, and say what is
    /// left.
    ///
    /// A mutation: it changes the document.
    pdf.convert {
        area: "pdf",
        layer: "pdf",
        icon: "check",
        mutates,
        undoable,
        params: {
            /// The level to check against: `2b` or `3b`.
            ///
            /// Without it, the rules that need a level are not checked.
            level: Option<Text>,
        },
        run: {
            let id = "pdf.convert".to_string();
            let level = match &self.level {
                None => None,
                Some(level) => Some(Level::from_id(&level.0).ok_or_else(|| EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{}` is not a level: a level is `2b` or `3b`", level.0.trim()),
                })?),
            };
            let mut fixed: Vec<String> = Vec::new();
            // The **additional actions**: they go away from every
            // dictionary that has them.
            let mut actions = 0_usize;
            for node in the_objects(session.document()) {
                let Some(found) = session.document().node(node).cloned() else { continue };
                let entries = preflight::entries_of(&found);
                if !entries.iter().any(|(name, _)| name == "AA") {
                    continue;
                }
                if let Some(found) = session.document_mut()?.node_mut(node) {
                    take_away(found, "AA");
                }
                session.document_mut()?.remove_relations(|relation| {
                    relation.from == node
                        && relation.kind == "ref"
                        && relation.payload.get("key").and_then(|key| key.as_str()).is_some_and(|key| key == "AA")
                });
                actions += 1;
            }
            if actions > 0 {
                fixed.push(format!("{actions}: removed the additional actions"));
            }
            // The entries that pointed at them.
            for node in the_objects(session.document()) {
                let Some(found) = session.document().node(node).cloned() else { continue };
                let entries = preflight::entries_of(&found);
                let Some(value) = preflight::entry(&entries, "A") else { continue };
                let Some(number) = number_of(value) else { continue };
                let gone = session.document().walk().find(|other| {
                    other.id != node && other.data.get("object").and_then(|n| n.as_u64()) == Some(number)
                });
                let Some(gone) = gone else { continue };
                let entries = preflight::entries_of(gone);
                if preflight::forbidden_action(&entries).is_none() {
                    continue;
                }
                if let Some(found) = session.document_mut()?.node_mut(node) {
                    take_away(found, "A");
                }
                session.document_mut()?.remove_relations(|relation| {
                    relation.from == node
                        && relation.kind == "ref"
                        && relation.payload.get("key").and_then(|key| key.as_str()).is_some_and(|key| key == "A")
                });
            }
            // The **forbidden actions**: the objects themselves go away,
            // and so do the entries that point at them.
            let mut forbidden = 0_usize;
            for node in the_objects(session.document()) {
                let Some(found) = session.document().node(node).cloned() else { continue };
                let entries = preflight::entries_of(&found);
                if preflight::forbidden_action(&entries).is_some() {
                    session.document_mut()?.remove_node(node);
                    forbidden += 1;
                }
            }
            if forbidden > 0 {
                fixed.push(format!("{forbidden}: removed the forbidden actions"));
            }
            // The **images**: the interpolate flag is cleared, and the
            // alternates and the OPI information go away.
            let mut images = 0_usize;
            for node in the_objects(session.document()) {
                let Some(found) = session.document().node(node).cloned() else { continue };
                let entries = preflight::entries_of(&found);
                if preflight::name_of(&entries, "Subtype").as_deref() != Some("Image") {
                    continue;
                }
                let mut changed = false;
                if let Some(found) = session.document_mut()?.node_mut(node) {
                    changed |= take_away(found, "Alternates");
                    changed |= take_away(found, "OPI");
                    // The **interpolate flag**: it is cleared, not taken
                    // away.
                    let mut entries = crate::entries_of(&found.data);
                    if let Some(entry) = entries.iter_mut().find(|(key, _)| key == "Interpolate")
                        && entry.1.as_bool().unwrap_or(false)
                    {
                        entry.1 = json!(false);
                        found.data = json!({
                            "object": found.data.get("object").cloned().unwrap_or(Json::Null),
                            "entries": entries
                                .iter()
                                .map(|(key, value)| json!([key, value]))
                                .collect::<Vec<_>>(),
                        });
                        changed = true;
                    }
                }
                if changed {
                    images += 1;
                }
            }
            if images > 0 {
                fixed.push(format!("{images}: cleared the image interpolation and the alternates"));
            }
            // The **annotations**: they print, and they are shown. A
            // popup is left alone: it is the window of another annotation
            // and not a note of its own.
            let mut annotations = 0_usize;
            for node in the_objects(session.document()) {
                let Some(found) = session.document().node(node).cloned() else { continue };
                let entries = preflight::entries_of(&found);
                if !preflight::is_an_annotation(&entries) {
                    continue;
                }
                if preflight::name_of(&entries, "Subtype").as_deref() == Some("Popup") {
                    continue;
                }
                if let Some(found) = session.document_mut()?.node_mut(node)
                    && crate::annot::show(found)
                {
                    annotations += 1;
                }
            }
            if annotations > 0 {
                fixed.push(format!("{annotations}: set the annotations to print and show"));
            }
            // The **NeedAppearances** flag: it asks the viewer to make the
            // appearances of the form.
            let mut appearances = 0_usize;
            let acroform = preflight::the_catalog(session.document())
                .map(preflight::entries_of)
                .and_then(|entries| preflight::entry(&entries, "AcroForm").cloned());
            if let Some(value) = acroform {
                let target = number_of(&value).and_then(|number| {
                    session
                        .document()
                        .walk()
                        .find(|node| node.data.get("object").and_then(|n| n.as_u64()) == Some(number))
                        .map(|node| node.id)
                });
                if let Some(target) = target {
                    // The form is an **object** of the file.
                    let taken = match session.document_mut()?.node_mut(target) {
                        Some(found) => take_away(found, "NeedAppearances"),
                        None => false,
                    };
                    if taken {
                        appearances += 1;
                    }
                } else if let Some(catalog) = preflight::the_catalog(session.document()).map(|node| node.id) {
                    // The form is a dictionary **inside** the catalog: the
                    // flag is taken away from the dictionary itself.
                    if let Some(found) = session.document_mut()?.node_mut(catalog) {
                        let entries = crate::entries_of(&found.data);
                        if let Some((_, form)) = entries.iter().find(|(key, _)| key == "AcroForm") {
                            let pairs = crate::entries_of(form);
                            let kept = without(&pairs, "NeedAppearances");
                            if kept.len() != pairs.len() {
                                let value = json!({
                                    "entries": kept
                                        .iter()
                                        .map(|(key, value)| json!([key, value]))
                                        .collect::<Vec<_>>(),
                                });
                                let mut new_entries = entries.clone();
                                if let Some(entry) = new_entries.iter_mut().find(|(key, _)| key == "AcroForm") {
                                    entry.1 = value;
                                }
                                found.data = json!({
                                    "object": found.data.get("object").cloned().unwrap_or(Json::Null),
                                    "entries": new_entries
                                        .iter()
                                        .map(|(key, value)| json!([key, value]))
                                        .collect::<Vec<_>>(),
                                });
                                appearances += 1;
                            }
                        }
                    }
                }
            }
            if appearances > 0 {
                fixed.push("cleared NeedAppearances".to_string());
            }
            // What is **left**: the preflight's own answer.
            let remaining = preflight::issues(session.document(), level);
            let count = remaining.len();
            Ok(Outcome::changed(format!("the file was converted and {count} rule(s) are left")).with_data(
                json!({
                    "fixed": fixed,
                    "remaining": remaining
                        .iter()
                        .map(|issue| json!({
                            "clause": issue.clause,
                            "message": issue.message.clone(),
                            "fixable": issue.fixable,
                        }))
                        .collect::<Vec<_>>(),
                    "count": count,
                }),
            ))

        }
    }
}
