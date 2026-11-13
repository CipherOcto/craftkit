//! The **bookmarks** of a layout document.
//!
//! A bookmark is a **name** for a page of a document, and the
//! bookmarks of a document are its own list.
//!
//! # The rules
//!
//! These are the tree's own rules.
//!
//! - A bookmark names a **page**, and the page is counted from one.
//!   A page that is not there is refused by name.
//! - The **name** of a bookmark is not empty, and a bookmark that is
//!   added without one is called `Bookmark N`, where N is the number
//!   of the bookmarks there are and one.
//! - A bookmark may be **renamed** and **deleted**.
//!
//! # What it does not do
//!
//! - **The tree's bookmark has children**, and the tree's bookmark is
//!   a tree of them. No command of the tree fills those children, and
//!   here a bookmark has none.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Number, Outcome, Session, Text};

/// The most a bookmark's **name** may have.
pub const MAX_NAME: usize = 256;

/// The **pages** of a document, in the order they stand in.
fn pages_of(document: &craftkit_sdk::Document, document_node: u64) -> Vec<u64> {
    let Some(node) = document.node(document_node) else { return Vec::new() };
    let mut pages = Vec::new();
    for spread in node.children.iter().filter(|child| child.kind == "layout.spread") {
        for page in spread.children.iter().filter(|child| child.kind == "layout.page") {
            pages.push(page.id);
        }
    }
    pages
}

/// The **bookmarks** of a document, in the order they were made.
fn bookmarks_of(node: &Node) -> Vec<(u64, String, i64)> {
    node.children
        .iter()
        .filter(|child| child.kind == "layout.bookmark")
        .map(|child| {
            let name = child.data.get("name").and_then(|name| name.as_str()).unwrap_or_default().to_string();
            let page = child.data.get("page").and_then(|page| page.as_i64()).unwrap_or(0);
            (child.id, name, page)
        })
        .collect()
}

/// The **bookmark** a command is about.
fn the_bookmark<'a>(session: &'a Session, command: &str, bookmark: u64) -> Result<&'a Node, EngineError> {
    let Some(node) = session.document().node(bookmark) else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("there is no bookmark `{bookmark}`"),
        });
    };
    if node.kind != "layout.bookmark" {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the node `{bookmark}` is a `{}` and not a bookmark", node.kind),
        });
    }
    Ok(node)
}

commands! {
    /// **Add** a bookmark to a document.
    ///
    /// The bookmark names a page, and the page is counted from one. The
    /// name is `Bookmark N` when it is not given.
    layout.bookmark.add {
        area: "layout",
        layer: "layout",
        icon: "tag",
        mutates,
        undoable,
        params: {
            /// The document.
            document: NodeRef,
            /// The name of the bookmark.
            name: Option<Text>,
            /// The page it names, counted from one.
            page: Option<Number>,
        },
        run: {
            let command = "layout.bookmark.add";
            let document = self.document.0;
            let Some(node) = session.document().node(document).cloned() else {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: format!("there is no document `{document}`"),
                });
            };
            // The **pages** of the document: the page of a bookmark is one
            // of them, counted from one.
            let pages = pages_of(session.document(), document);
            if pages.is_empty() {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: format!("the document `{document}` has no page"),
                });
            }
            let wanted = match &self.page {
                Some(page) => {
                    let number = page.0;
                    if !number.is_finite() || number.fract() != 0.0 || number < 1.0 {
                        return Err(EngineError::Failed {
                            id: command.to_string(),
                            message: format!("a page is a whole number from one, and {number} is not one"),
                        });
                    }
                    number as usize
                }
                None => 1,
            };
            if wanted > pages.len() {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: format!("there is no page {wanted}"),
                });
            }
            // The **name**: `Bookmark N` when it is not given.
            let given = self.name.as_ref().map(|name| name.0.trim().to_string()).filter(|name| !name.is_empty());
            let name = match given {
                Some(name) => {
                    if name.chars().count() > MAX_NAME {
                        return Err(EngineError::Failed {
                            id: command.to_string(),
                            message: format!("the name of a bookmark is not longer than {MAX_NAME} characters"),
                        });
                    }
                    name
                }
                None => format!("Bookmark {}", bookmarks_of(&node).len() + 1),
            };
            let id = session.document_mut()?.add_node_with_under(
                Some(document),
                name.clone(),
                "layout.bookmark",
                None,
                json!({ "name": name, "page": wanted }),
            );
            Ok(Outcome::changed(format!("added the bookmark `{name}`")).with_data(json!({
                "id": id,
                "document": document,
                "name": name,
                "page": wanted,
            })))

        }
    }

    /// **Rename** a bookmark.
    layout.bookmark.rename {
        area: "layout",
        layer: "layout",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The bookmark.
            bookmark: NodeRef,
            /// Its new name.
            name: Text,
        },
        run: {
            let command = "layout.bookmark.rename";
            let bookmark = self.bookmark.0;
            the_bookmark(session, command, bookmark)?;
            let name = self.name.0.trim().to_string();
            if name.is_empty() {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: "a bookmark has a name".to_string(),
                });
            }
            if name.chars().count() > MAX_NAME {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: format!("the name of a bookmark is not longer than {MAX_NAME} characters"),
                });
            }
            if let Some(node) = session.document_mut()?.node_mut(bookmark) {
                node.name = name.clone();
                node.data["name"] = Json::String(name.clone());
            }
            Ok(Outcome::changed(format!("the bookmark is `{name}`"))
                .with_data(json!({ "bookmark": bookmark, "name": name })))

        }
    }

    /// **Delete** a bookmark.
    layout.bookmark.delete {
        area: "layout",
        layer: "layout",
        icon: "minus",
        mutates,
        undoable,
        params: {
            /// The bookmark.
            bookmark: NodeRef,
        },
        run: {
            let command = "layout.bookmark.delete";
            let bookmark = self.bookmark.0;
            the_bookmark(session, command, bookmark)?;
            session.document_mut()?.remove_node(bookmark);
            Ok(Outcome::changed(format!("the bookmark `{bookmark}` is gone"))
                .with_data(json!({ "bookmark": bookmark })))

        }
    }

    /// The **bookmarks** of a document: the names and the pages they name.
    layout.document.bookmarks {
        area: "layout",
        layer: "layout",
        icon: "list",
        params: {
            /// The document.
            document: NodeRef,
        },
        run: {
            let command = "layout.document.bookmarks";
            let document = self.document.0;
            let Some(node) = session.document().node(document) else {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: format!("there is no document `{document}`"),
                });
            };
            let bookmarks: Vec<Json> = bookmarks_of(node)
                .into_iter()
                .map(|(id, name, page)| json!({ "id": id, "name": name, "page": page }))
                .collect();
            Ok(Outcome::new(format!("the document `{document}` holds {} bookmark(s)", bookmarks.len()))
                .with_data(json!({
                    "document": document,
                    "bookmarks": bookmarks,
                    "count": bookmarks.len(),
                })))

        }
    }
}
