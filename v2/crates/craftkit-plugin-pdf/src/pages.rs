//! The **removal of a page**.
//!
//! A page is an object of the file, and taking a page away is not only
//! a matter of dropping a node: the **tree of pages** has to be right
//! afterwards, and everything that pointed at the page has to stop
//! pointing at it.
//!
//! So this module takes the page away and takes the references to it
//! away with it — the **edges** of the kind `ref` die with the node
//! (the SDK does that), and the **values in the dictionaries** are
//! taken away as well, because the file is written from the
//! dictionaries and not from the edges.
//!
//! And the **widgets** of the page leave the **form**: a widget is a
//! field of the form, and a field that points at a page that is not
//! there would keep it, with its content, in the file that is written.
//!
//! And the **number** of the page that is gone is **free**: it is not
//! used again, and the file says so.
//!
//! # What it does not do
//!
//! - **The tree of pages is not rebuilt.** The tree moves the
//!   attributes that a page inherits down into the pages that are left
//!   and takes them out of the root. Here the tree keeps the
//!   attributes where they are, and the pages that are left inherit
//!   them as they did.

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, Node, NodeRef, Outcome, Session};

use crate::{entries_of, pairs_of, the_number_of};

/// Take a **page** away.
///
/// The **last page** cannot go: a file with no pages is not a file.
///
/// The **destinations** that went to the page go with it: a bookmark or
/// a link that went to a page that is not there has no destination,
/// and half a destination is not a destination.
///
/// What the page **held** stays: an annotation of the page is an object
/// of its own, and it stays in the document — it is simply an object
/// that nothing refers to any more.
#[derive(Command)]
#[command(id = "pdf.page.remove", area = "pdf", layer = "pdf", icon = "trash", mutates, undoable)]
pub struct RemovePage {
    /// The page that is taken away.
    pub page: NodeRef,
}

impl RemovePage {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "pdf.page.remove".to_string();
        let page = self.page.0;
        let Some(node) = session.document().node(page) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{page}`"),
            });
        };
        if node.kind != "pdf.object" && node.kind != "pdf.stream" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{page}` is a `{}` and not an object", node.kind),
            });
        }
        let Some(number) = the_number_of(node) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{page}` has no object number"),
            });
        };
        if !is_a_page(node) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{page}` is not a page"),
            });
        }
        // The **pages** of the file: a file with no pages is not a
        // file, so the last page cannot go.
        let pages: Vec<u64> =
            session.document().walk().filter(|node| is_a_page(node)).map(|node| node.id).collect();
        if pages.len() <= 1 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "a file with no pages is not a file".to_string(),
            });
        }
        // The **parent** of the page: the tree of pages loses the page,
        // and the count of the children is derived from the children
        // that are left.
        let parent = parent_of(session, page);
        // The **references** to the page, in the dictionaries of the
        // objects that hold them. They are found in the dictionaries
        // and not in the edges, because the file is written from the
        // dictionaries.
        let holders: Vec<Holder> = session
            .document()
            .walk()
            .filter(|node| node.id != page && (node.kind == "pdf.object" || node.kind == "pdf.stream"))
            .filter_map(|node| {
                let entries = entries_of(&node.data);
                let (kept, gone) = without_the_page(session, &entries, number);
                if kept == entries && gone.is_empty() {
                    return None;
                }
                Some(Holder { id: node.id, kept, gone })
            })
            .collect();
        // The **widgets** of the page leave the **form**: a field that
        // points at a page that is not there would keep it, with its
        // content, in the file that is written.
        let widgets = drop_widgets(session, page)?;
        if !session.document_mut()?.remove_node(page) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{page}`"),
            });
        }
        let mut unlinked: Vec<serde_json::Value> = Vec::new();
        for widget in &widgets {
            unlinked.push(serde_json::json!({ "widget": widget, "form": true }));
        }
        for holder in holders {
            for key in &holder.gone {
                unlinked.push(serde_json::json!({ "object": holder.id, "key": key }));
            }
            if let Some(node) = session.document_mut()?.node_mut(holder.id) {
                node.data = serde_json::json!({
                    "object": node.data.get("object").cloned().unwrap_or(serde_json::Value::Null),
                    "entries": pairs_of(&holder.kept),
                });
            }
        }
        // The **count** of the children of the tree of pages: it is
        // derived, and it is derived from the children that are left.
        if let Some(parent) = parent {
            let children = session
                .document()
                .relations
                .iter()
                .filter(|relation| {
                    relation.kind == "ref"
                        && relation.from == parent
                        && relation.payload.get("key").and_then(|key| key.as_str()) == Some("Kids")
                })
                .count();
            if let Some(node) = session.document_mut()?.node_mut(parent) {
                node.data = crate::with_entry(&node.data, "Count", serde_json::json!(children));
            }
        }
        // The **number** of the page that is gone is **free**: it is
        // not used again, and the file says so.
        crate::free::free_number(session.document_mut()?, number, 0);
        Ok(Outcome::changed(format!("the page `{page}` is gone"))
            .with_data(serde_json::json!({ "id": page, "object": number, "unlinked": unlinked })))
    }
}

/// An object whose **dictionary** named the page that is gone.
struct Holder {
    /// The node that holds the dictionary.
    id: u64,
    /// The dictionary, with the references taken away.
    kept: Vec<(String, serde_json::Value)>,
    /// The keys that went.
    gone: Vec<String>,
}

/// The **widgets** of a page leave the **form**, and the page goes.
///
/// A widget is an annotation whose subtype is `Widget`, and a widget
/// is a **field** of the form: the form holds it, or the field it
/// belongs to holds it. The tree takes the widget out of the list that
/// holds it — out of the field's `Kids`, or out of the form's `Fields`
/// when the widget is a field itself — and a field left with no
/// children goes as well.
///
/// The widget **stays** in the document: it is an object of its own,
/// and an object that nothing refers to any more is still an object.
fn drop_widgets(session: &mut Session, page: u64) -> Result<Vec<u64>, EngineError> {
    // The **annotations** of the page, and the widgets among them.
    let Some(node) = session.document().node(page) else { return Ok(Vec::new()) };
    let annots: Vec<u64> = entries_of(&node.data)
        .into_iter()
        .find(|(key, _)| key == "Annots")
        .and_then(|(_, value)| value.as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(number_named)
        .collect();
    let mut widgets: Vec<u64> = Vec::new();
    for annot in annots {
        let Some(node) = session.document().walk().find(|node| the_number_of(node) == Some(annot)) else {
            continue;
        };
        let subtype = entries_of(&node.data)
            .into_iter()
            .find(|(key, _)| key == "Subtype")
            .and_then(|(_, value)| value.as_str().map(|subtype| subtype.to_string()));
        if subtype.as_deref() == Some("Widget") {
            widgets.push(annot);
        }
    }
    // Each widget leaves the list that holds it, and a field left with
    // no children goes as well.
    let mut gone: Vec<u64> = Vec::new();
    let mut pending: Vec<u64> = widgets.clone();
    while let Some(widget) = pending.pop() {
        let Some(node) = session.document().walk().find(|node| the_number_of(node) == Some(widget)) else {
            continue;
        };
        let parent = entries_of(&node.data)
            .into_iter()
            .find(|(key, _)| key == "Parent")
            .and_then(|(_, value)| number_named(&value));
        let holder = match parent {
            Some(parent) => {
                let empty = unlink_from(session, parent, "Kids", widget)?;
                if empty {
                    pending.push(parent);
                }
                Some(parent)
            }
            None => {
                unlink_from_form(session, widget)?;
                None
            }
        };
        gone.push(widget);
        let _ = holder;
    }
    Ok(gone)
}

/// Take a number out of a **list** of a dictionary, and say whether
/// the list is empty afterwards.
fn unlink_from(session: &mut Session, object: u64, key: &str, number: u64) -> Result<bool, EngineError> {
    let Some(found) = session.document().walk().find(|node| the_number_of(node) == Some(object)) else {
        return Ok(false);
    };
    let id = found.id;
    let entries = entries_of(&found.data);
    let Some((_, value)) = entries.iter().find(|(name, _)| name == key) else { return Ok(false) };
    let Some(list) = value.as_array() else { return Ok(false) };
    if !list.iter().any(|item| number_named(item) == Some(number)) {
        return Ok(false);
    }
    let rest: Vec<serde_json::Value> =
        list.iter().filter(|item| number_named(item) != Some(number)).cloned().collect();
    let empty = rest.is_empty();
    let kept: Vec<(String, serde_json::Value)> = entries
        .into_iter()
        .map(|(name, value)| if name == key { (name, serde_json::json!(rest)) } else { (name, value) })
        .collect();
    if let Some(node) = session.document_mut()?.node_mut(id) {
        node.data = serde_json::json!({
            "object": node.data.get("object").cloned().unwrap_or(serde_json::Value::Null),
            "entries": pairs_of(&kept),
        });
    }
    Ok(empty)
}

/// Take a number out of the **fields** of the form.
fn unlink_from_form(session: &mut Session, number: u64) -> Result<(), EngineError> {
    let Some(form) = the_form(session) else { return Ok(()) };
    unlink_from(session, form, "Fields", number)?;
    Ok(())
}

/// The **form** of the file: the object the catalog names in its
/// `AcroForm`, by its number.
fn the_form(session: &Session) -> Option<u64> {
    let catalog = session.document().walk().find(|node| {
        (node.kind == "pdf.object" || node.kind == "pdf.stream")
            && entries_of(&node.data)
                .iter()
                .any(|(key, value)| key == "Type" && value.as_str() == Some("Catalog"))
    })?;
    let form = entries_of(&catalog.data)
        .into_iter()
        .find(|(key, _)| key == "AcroForm")
        .and_then(|(_, value)| number_named(&value))?;
    // The **number** of the form: the lists hold numbers and not node
    // ids.
    Some(form)
}

/// Whether a node is a **page**: an object whose type is `Page`.
fn is_a_page(node: &Node) -> bool {
    (node.kind == "pdf.object" || node.kind == "pdf.stream")
        && entries_of(&node.data).iter().any(|(key, value)| key == "Type" && value.as_str() == Some("Page"))
}

/// The **parent** of a page: the node the page names in its `Parent`.
fn parent_of(session: &Session, page: u64) -> Option<u64> {
    let node = session.document().node(page)?;
    let number = entries_of(&node.data)
        .into_iter()
        .find(|(key, _)| key == "Parent")
        .and_then(|(_, value)| number_named(&value))?;
    session.document().walk().find(|node| the_number_of(node) == Some(number)).map(|node| node.id)
}

/// The entries of a dictionary, with the **page** taken away.
///
/// A key whose value names the page loses the key. A key whose value is
/// a **list** loses the one element that names the page, and the key
/// too when the list is empty afterwards — except the key of a
/// **destination**, which goes whole: a link that went to a page that
/// is not there has no destination, and half a destination is not a
/// destination. The tree's rule, and the same for an **action** that
/// goes to the page.
fn without_the_page(
    session: &Session,
    entries: &[(String, serde_json::Value)],
    number: u64,
) -> (Vec<(String, serde_json::Value)>, Vec<String>) {
    let mut kept: Vec<(String, serde_json::Value)> = Vec::with_capacity(entries.len());
    let mut gone: Vec<String> = Vec::new();
    for (key, value) in entries {
        // The **destination**: it goes whole.
        if key == "Dest" && dest_names(value, number) {
            gone.push(key.clone());
            continue;
        }
        // The **action**: it goes whole.
        if key == "A" && action_goes_to(session, value, number) {
            gone.push(key.clone());
            continue;
        }
        if names(value, number) {
            gone.push(key.clone());
            continue;
        }
        if let Some(list) = value.as_array() {
            let rest: Vec<serde_json::Value> =
                list.iter().filter(|item| !names(item, number)).cloned().collect();
            if rest.len() != list.len() {
                if rest.is_empty() {
                    gone.push(key.clone());
                    continue;
                }
                kept.push((key.clone(), serde_json::Value::Array(rest)));
                continue;
            }
        }
        kept.push((key.clone(), value.clone()));
    }
    (kept, gone)
}

/// Whether a **destination** names the page: a list whose first
/// element names it.
fn dest_names(value: &serde_json::Value, number: u64) -> bool {
    value.as_array().and_then(|list| list.first()).is_some_and(|first| names(first, number))
}

/// Whether an **action** goes to the page.
///
/// The action is a dictionary — its own object, or a dictionary inside
/// the one that holds it — and it goes to the page when it is a `GoTo`
/// and its destination names the page.
fn action_goes_to(session: &Session, value: &serde_json::Value, number: u64) -> bool {
    let Some(entries) = entries_of_value(session, value) else { return false };
    let kind = entries.iter().find(|(key, _)| key == "S").and_then(|(_, value)| value.as_str());
    if kind != Some("GoTo") {
        return false;
    }
    entries.iter().find(|(key, _)| key == "D").is_some_and(|(_, value)| dest_names(value, number))
}

/// The entries of a dictionary that is a **value**: its own object, or
/// a dictionary inside the one that holds it.
fn entries_of_value(
    session: &Session,
    value: &serde_json::Value,
) -> Option<Vec<(String, serde_json::Value)>> {
    if let Some(number) = number_named(value) {
        let node = session.document().walk().find(|node| the_number_of(node) == Some(number))?;
        return Some(entries_of(&node.data));
    }
    // A dictionary **inside** the one that holds it: a list of pairs.
    let list = value.as_array()?;
    let mut entries: Vec<(String, serde_json::Value)> = Vec::with_capacity(list.len());
    for entry in list {
        let pair = entry.as_array()?;
        if pair.len() != 2 {
            return None;
        }
        let key = pair.first()?.as_str()?.to_string();
        entries.push((key, pair.get(1)?.clone()));
    }
    Some(entries)
}

/// Whether a value names an object number.
///
/// A reference is a value of the shape `{"ref": [number, generation]}`.
fn names(value: &serde_json::Value, number: u64) -> bool {
    number_named(value) == Some(number)
}

/// The object number a value names, if it names one.
fn number_named(value: &serde_json::Value) -> Option<u64> {
    value
        .get("ref")
        .and_then(|pair| pair.as_array())
        .and_then(|pair| pair.first())
        .and_then(|number| number.as_u64())
}
