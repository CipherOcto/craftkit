//! The **link** of an item.
//!
//! An item may carry a **link**: a target it points at. The target is
//! either **outside** the document — a web link or a mail link — or
//! **inside** it: a page of the same document.
//!
//! The link of an item is **one** thing, so setting one takes the
//! other away: an item that points at a web page does not also point
//! at a page of the document.
//!
//! The rules are the tree's own:
//!
//! - a link that the system may **follow** is a **web link** or a
//!   **mail link** — `http`, `https` and `mailto` — and nothing else.
//!   A script, a `data:` URL, a `file:` path, a link to **another
//!   machine** — a protocol-relative `//host` or a UNC `\\host` — and
//!   a relative link are all links that the system does not follow,
//!   and each is refused for its own reason;
//! - a link to a **page** must name a page of the **same document**:
//!   a page of another document is a link to a document that is not
//!   here.
//!
//! # What the port found
//!
//! - **The tree's link has a source and a destination**, and the
//!   source is a selection — a piece of text or a set of frames. Here
//!   the source is the **item** and the destination is one of two
//!   kinds, and the two kinds are two commands because they are two
//!   things.
//! - **The tree's link to a page is a page number**, and a page number
//!   is a fact of the machine's layout: here a page is a **node**, and
//!   the link to it is an **edge**.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Outcome, Session, Text};

use crate::items::{item_of, may_edit};

/// The most characters a target may have.
pub const MAX_TARGET: usize = 2048;

/// The **schemes** a link that may be opened has.
pub const OPENABLE: [&str; 3] = ["http", "https", "mailto"];

/// The **kind** of the edge that says an item points at a page.
pub const LINKS_TO: &str = "layout.links_to";

/// Whether a target **may be opened**.
///
/// The tree's rule, exactly: the scheme is what is before the first
/// colon, and it is one of the three.
pub fn may_be_opened(target: &str) -> bool {
    let Some((scheme, _)) = target.split_once(':') else { return false };
    OPENABLE.iter().any(|openable| scheme.eq_ignore_ascii_case(openable))
}

/// Why a target may **not** be opened, if it may not.
///
/// Each kind of target is refused for its own reason, because a
/// refusal that does not say why is a refusal the caller cannot act
/// on.
pub fn why_not(target: &str) -> Option<String> {
    if may_be_opened(target) {
        return None;
    }
    if target.chars().count() > MAX_TARGET {
        return Some(format!("a target is at most {MAX_TARGET} characters long"));
    }
    let scheme = target.split_once(':').map(|(scheme, _)| scheme.to_ascii_lowercase());
    match scheme.as_deref() {
        Some("javascript") | Some("vbscript") => {
            Some("a script is not a link that may be opened".to_string())
        }
        Some("data") => Some("a data link is not a link that may be opened".to_string()),
        Some("file") => Some("a file is not a link that may be opened".to_string()),
        _ => {
            // A link to **another machine**: a protocol-relative
            // reference or a UNC path. Following it would send the
            // user's own credentials.
            let cleaned: String = target.chars().filter(|c| !c.is_ascii_control() && *c != ' ').collect();
            if cleaned.replace('\\', "/").starts_with("//") {
                Some("a link to another machine is not a link that may be opened".to_string())
            } else if scheme.is_none() {
                Some("a relative link is not a link that may be opened".to_string())
            } else {
                Some("a link that is not a web link or a mail link may not be opened".to_string())
            }
        }
    }
}

/// The **target** an item points at outside the document.
pub fn target_of(item: &Node) -> Option<String> {
    item.data
        .get("link")
        .and_then(|link| link.get("target"))
        .and_then(|target| target.as_str())
        .map(|target| target.to_string())
}

/// The **page** an item points at, if it points at one.
pub fn page_of(document: &craftkit_sdk::Document, item: u64) -> Option<u64> {
    document.relations_from(item, Some(LINKS_TO)).first().map(|relation| relation.to)
}

/// The **pages** of a document.
fn pages_of(session: &Session, document: u64) -> Vec<u64> {
    let Some(node) = session.document().node(document) else { return Vec::new() };
    node.children
        .iter()
        .filter(|child| child.kind == "layout.spread")
        .flat_map(|spread| spread.children.iter())
        .filter(|child| child.kind == "layout.page")
        .map(|child| child.id)
        .collect()
}

/// The **items** of a document.
fn items_of(document: &Node) -> Vec<std::sync::Arc<Node>> {
    document
        .children
        .iter()
        .filter(|child| child.kind == "layout.spread")
        .flat_map(|spread| spread.children.iter())
        .filter(|child| child.kind.starts_with("layout.item."))
        .cloned()
        .collect()
}

/// The **document** an item is in, if it is in one.
fn document_of(session: &Session, item: u64) -> Option<u64> {
    session
        .document()
        .walk()
        .find(|node| {
            node.kind == "layout.document"
                && node.children.iter().any(|spread| {
                    spread.kind == "layout.spread" && spread.children.iter().any(|child| child.id == item)
                })
        })
        .map(|node| node.id)
}

commands! {
    /// **Set** the link of an item: a target outside the document.
    ///
    /// The target must be one that may be opened — a web link or a mail
    /// link — and an empty target takes the link away.
    layout.item.link {
        area: "layout",
        layer: "layout",
        icon: "link",
        mutates,
        undoable,
        params: {
            /// The item.

            item: NodeRef,
            /// The target: `https://example.com` or `mailto:ann@example.com`.
            /// Without it, the link is taken away.

            target: Text,
        },
        run: {

            let id = "layout.item.link".to_string();
            let item = self.item.0;
            let node = item_of(session, &id, item)?;
            may_edit(session, &node, &id)?;
            let target = self.target.0.trim().to_string();
            if target.is_empty() {
                session
                    .document_mut()?
                    .remove_relations(|relation| relation.kind == LINKS_TO && relation.from == item);
                if let Some(node) = session.document_mut()?.node_mut(item)
                    && let Some(map) = node.data.as_object_mut()
                {
                    map.remove("link");
                }
                return Ok(Outcome::changed(format!("the item `{item}` has no link"))
                    .with_data(json!({ "item": item, "target": Json::Null })));
            }
            if let Some(why) = why_not(&target) {
                return Err(EngineError::Failed { id: id.clone(), message: why });
            }
            session
                .document_mut()?
                .remove_relations(|relation| relation.kind == LINKS_TO && relation.from == item);
            if let Some(node) = session.document_mut()?.node_mut(item) {
                node.data["link"] = json!({ "target": target });
            }
            Ok(Outcome::changed(format!("the item `{item}` points at `{target}`"))
                .with_data(json!({ "item": item, "target": target })))

        }
    }
    /// **Set** the link of an item to a **page** of the document.
    ///
    /// The page must be a page of the **same document**: a page of another
    /// document is a link to a document that is not here. Without a page,
    /// the link to a page is taken away.
    layout.item.page.link {
        area: "layout",
        layer: "layout",
        icon: "link",
        mutates,
        undoable,
        params: {
            /// The item.

            item: NodeRef,
            /// The page it points at. Without it, the link to a page is taken
            /// away.

            page: Option<NodeRef>,
        },
        run: {

            let id = "layout.item.page.link".to_string();
            let item = self.item.0;
            let node = item_of(session, &id, item)?;
            may_edit(session, &node, &id)?;
            let Some(page) = self.page.map(|page| page.0) else {
                session
                    .document_mut()?
                    .remove_relations(|relation| relation.kind == LINKS_TO && relation.from == item);
                if let Some(node) = session.document_mut()?.node_mut(item)
                    && let Some(map) = node.data.as_object_mut()
                {
                    map.remove("link");
                }
                return Ok(Outcome::changed(format!("the item `{item}` points at no page"))
                    .with_data(json!({ "item": item, "page": Json::Null })));
            };
            let Some(page_node) = session.document().node(page).cloned() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no page `{page}`"),
                });
            };
            if page_node.kind != "layout.page" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{page}` is a `{}` and not a page", page_node.kind),
                });
            }
            // The page must be in the **same document** as the item.
            let Some(document) = document_of(session, item) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the item `{item}` is in no document"),
                });
            };
            let same = pages_of(session, document).contains(&page);
            if !same {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the page `{page}` is not in the document of the item `{item}`"),
                });
            }
            let name = page_node.name.clone();
            session
                .document_mut()?
                .remove_relations(|relation| relation.kind == LINKS_TO && relation.from == item);
            if let Some(node) = session.document_mut()?.node_mut(item)
                && let Some(map) = node.data.as_object_mut()
            {
                map.remove("link");
            }
            let added = session.document_mut()?.add_relation(craftkit_sdk::Relation::with_payload(
                item,
                page,
                LINKS_TO,
                json!({ "page": name.clone() }),
            ));
            if !added {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the item `{item}` already points at the page `{name}`"),
                });
            }
            Ok(Outcome::changed(format!("the item `{item}` points at the page `{name}`"))
                .with_data(json!({ "item": item, "page": page, "name": name })))

        }
    }
    /// **Read** the links of a document.
    ///
    /// The answer is every item that carries a link: the target it points
    /// at, or the page it points at, and whether the target **may be
    /// opened**.
    layout.links {
        area: "layout",
        layer: "layout",
        icon: "link",
        params: {
            /// The document.

            document: NodeRef,
        },
        run: {

            let id = "layout.links".to_string();
            let document = self.document.0;
            let Some(node) = session.document().node(document) else {
                return Err(EngineError::Failed { id, message: format!("there is no document `{document}`") });
            };
            if node.kind != "layout.document" {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the node `{document}` is a `{}` and not a document", node.kind),
                });
            }
            let document_node = node.clone();
            let mut links: Vec<Json> = Vec::new();
            for item in items_of(&document_node) {
                let page = page_of(session.document(), item.id);
                let target = target_of(&item);
                if target.is_none() && page.is_none() {
                    continue;
                }
                let mut link = json!({ "item": item.id, "name": item.name });
                if let Some(target) = &target {
                    link["kind"] = json!("target");
                    link["target"] = json!(target);
                    link["may_be_opened"] = json!(may_be_opened(target));
                } else if let Some(page) = page {
                    let name = session.document().node(page).map(|node| node.name.clone());
                    link["kind"] = json!("page");
                    link["page"] = json!(page);
                    link["page_name"] = match name {
                        Some(name) => json!(name),
                        None => Json::Null,
                    };
                    link["may_be_opened"] = json!(true);
                }
                links.push(link);
            }
            Ok(Outcome::new(format!("the document has {} link(s)", links.len())).with_data(json!({
                "document": document,
                "links": links,
                "count": links.len(),
            })))

        }
    }
}
