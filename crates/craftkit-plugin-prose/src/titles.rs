//! The **title** of a document.
//!
//! A document with no title is a document that a reader cannot name:
//! in a list of files, in the window bar, and in the properties a
//! screen reader reads out. The title is a **fact of the document**,
//! and it is not the name of the node: the name is how the tree calls
//! it, and the title is what the document says it is.
//!
//! The rules:
//!
//! - an **empty** title **takes the title away**: a document that has
//!   no title and a document whose title was taken away are the same
//!   document;
//! - a node that is **not a document** may not have a title.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, Node, NodeRef, Outcome, Text};

/// The **title** of a document: what it says it is.
///
/// A document that says nothing has no title, and the answer is
/// nothing and not an empty text that means the same thing.
pub fn title_of(document: &Node) -> Option<String> {
    document
        .data
        .get("title")
        .and_then(|title| title.as_str())
        .map(|title| title.to_string())
        .filter(|title| !title.trim().is_empty())
}

commands! {
    /// **Say** what a document is.
    prose.document.title {
        area: "prose",
        layer: "prose",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The document.
            document: NodeRef,
            /// The title.
            ///
            /// An empty title takes the title away.
            title: Text,
        },
        run: {
            let id = "prose.document.title".to_string();
            let document = self.document.0;
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
            let title = self.title.0.trim().to_string();
            let node = session.document_mut()?.node_mut(document);
            let Some(node) = node else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{document}`"),
                });
            };
            if title.is_empty() {
                // An **empty** value takes the fact away.
                if let Some(map) = node.data.as_object_mut() {
                    map.remove("title");
                }
            } else {
                node.data["title"] = json!(title.clone());
            }
            Ok(Outcome::changed(if title.is_empty() {
                format!("the document `{document}` has no title")
            } else {
                format!("the document `{document}` is `{title}`")
            })
            .with_data(json!({ "document": document, "title": title })))

        }
    }
}
