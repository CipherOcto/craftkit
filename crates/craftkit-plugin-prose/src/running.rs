//! The **running heads** of a document: the header and the footer.
//!
//! A document is printed on pages, and a page may carry a **header**
//! at its top and a **footer** at its bottom: the running head of a
//! book, the page number of a report, the name of a document on
//! every sheet.
//!
//! The header and the footer are **facts of the document**, the way
//! the title is: they are what the document says it carries on every
//! page, and they are not blocks of the document — they do not stand
//! in the flow of the text.
//!
//! The rules:
//!
//! - an **empty** text **takes the running head away**: a document
//!   that has no header and a document whose header was taken away are
//!   the same document;
//! - a node that is **not a document** may not have a header or a
//!   footer;
//! - the text is **what it is**: the running head is not a
//!   paragraph, and it has no style of its own here.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Outcome, Session, Text};

/// The **header** of a document: what it carries at the top of every
/// page.
///
/// A document that carries nothing has no header, and the answer is
/// nothing and not an empty text that means the same thing.
pub fn header_of(document: &Node) -> Option<String> {
    running_of(document, "header")
}

/// The **footer** of a document: what it carries at the bottom of every
/// page.
pub fn footer_of(document: &Node) -> Option<String> {
    running_of(document, "footer")
}

/// A **running head** of a document, by its name.
fn running_of(document: &Node, key: &str) -> Option<String> {
    document
        .data
        .get(key)
        .and_then(|text| text.as_str())
        .map(|text| text.to_string())
        .filter(|text| !text.trim().is_empty())
}

commands! {
    /// **Say** what a document carries at the top of every page.
    prose.document.header {
        area: "prose",
        layer: "prose",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The document.

            document: NodeRef,
            /// The header's text.
            ///
            /// An empty text takes the header away.

            header: Text,
        },
        run: {

            set_running(session, self.document.0, "header", self.header.0.trim())

        }
    }
    /// **Say** what a document carries at the bottom of every page.
    prose.document.footer {
        area: "prose",
        layer: "prose",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The document.

            document: NodeRef,
            /// The footer's text.
            ///
            /// An empty text takes the footer away.

            footer: Text,
        },
        run: {

            set_running(session, self.document.0, "footer", self.footer.0.trim())

        }
    }
    /// **Read** the **header** of a document.
    prose.document.header.read {
        area: "prose",
        layer: "prose",
        icon: "list",
        params: {
            /// The document.

            document: NodeRef,
        },
        run: {

            read_running(session, self.document.0, "header")

        }
    }
    /// **Read** the **footer** of a document.
    prose.document.footer.read {
        area: "prose",
        layer: "prose",
        icon: "list",
        params: {
            /// The document.

            document: NodeRef,
        },
        run: {

            read_running(session, self.document.0, "footer")

        }
    }
}

/// Say what a document carries, at the top or at the bottom of every
/// page.
fn set_running(session: &mut Session, document: u64, key: &str, text: &str) -> Result<Outcome, EngineError> {
    let id = format!("prose.document.{key}");
    let Some(node) = session.document().node(document).cloned() else {
        return Err(EngineError::Failed {
            id: id.clone(),
            message: format!("there is no node `{document}`"),
        });
    };
    if node.kind != "prose.document" {
        return Err(EngineError::Failed {
            id: id.clone(),
            message: format!("the node `{document}` is a `{}` and not a document", node.kind),
        });
    }
    let Some(node) = session.document_mut()?.node_mut(document) else {
        return Err(EngineError::Failed {
            id: id.clone(),
            message: format!("there is no node `{document}`"),
        });
    };
    if text.is_empty() {
        // An **empty** value takes the fact away.
        if let Some(map) = node.data.as_object_mut() {
            map.remove(key);
        }
    } else {
        node.data[key] = json!(text);
    }
    Ok(Outcome::changed(if text.is_empty() {
        format!("the document `{document}` has no {key}")
    } else {
        format!("the document `{document}` carries a {key}")
    })
    .with_data(json!({ "document": document, key: text })))
}

/// Read what a document carries, at the top or at the bottom of every
/// page.
fn read_running(session: &Session, document: u64, key: &str) -> Result<Outcome, EngineError> {
    let id = format!("prose.document.{key}.read");
    let Some(node) = session.document().node(document) else {
        return Err(EngineError::Failed {
            id: id.clone(),
            message: format!("there is no node `{document}`"),
        });
    };
    if node.kind != "prose.document" {
        return Err(EngineError::Failed {
            id: id.clone(),
            message: format!("the node `{document}` is a `{}` and not a document", node.kind),
        });
    }
    let held = match key {
        "header" => header_of(node),
        _ => footer_of(node),
    };
    Ok(Outcome::new(match &held {
        Some(_) => format!("the document `{document}` carries a {key}"),
        None => format!("the document `{document}` has no {key}"),
    })
    .with_data(json!({
        "document": document,
        key: held.map(|text| json!(text)).unwrap_or(Json::Null),
    })))
}
