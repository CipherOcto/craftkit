//! The **comment** on a cell.
//!
//! A comment is a **thread**: a first comment, the replies to it and
//! whether the thread is **resolved**. It is the tree's *comment* —
//! the thread in the margin of a cell — and it is not the tree's
//! *note*, which is the yellow box in the corner of a cell (see the
//! module `notes`).
//!
//! A comment belongs to the **sheet** and names a cell, and that is
//! where it is kept: the tree keeps its comments in a map of the
//! sheet and not in the cell, because a cell in the tree is a value
//! and a formula and a style, and a comment is none of those three.
//!
//! # The rules
//!
//! - A comment is **text**, and it may say who wrote it. There is no
//!   user name in this tree, so the author is what is given and it is
//!   kept when it is not given again.
//! - Writing a comment on a cell that **has one edits it**: the text
//!   is the new text, the author is kept unless a new one is given,
//!   and the replies and the resolved flag stay. The tree's own rule.
//! - A **note replaces a comment** and a comment replaces a note: the
//!   tree's own rule. A cell carries the one or the other.
//! - A **reply** joins the thread of the cell. A cell that has no
//!   comment has no thread to join, and that is refused by name.
//! - A thread may be **resolved**, and unresolved again.
//! - The **whole cell cleared** takes the comment with it — the
//!   tree's *clear all* takes the comment of the cell with it — and
//!   the *clear contents* does not.

use std::collections::BTreeMap;

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Outcome, Text};

/// A **reply** in a thread: the text of it, and who wrote it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reply {
    /// The text of the reply.
    pub text: String,
    /// Who wrote it, when that is known.
    pub author: Option<String>,
}

/// A **comment**: a thread on a cell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Comment {
    /// The text of the comment.
    pub text: String,
    /// Who wrote it, when that is known.
    pub author: Option<String>,
    /// The replies to it, in the order they were written.
    pub replies: Vec<Reply>,
    /// Whether the thread is resolved.
    pub resolved: bool,
}

/// The **comments** of a sheet: by the address of the cell they are on.
pub fn comments_of(node: &Node) -> BTreeMap<String, Comment> {
    let mut comments = BTreeMap::new();
    let Some(map) = node.data.get("comments").and_then(|comments| comments.as_object()) else {
        return comments;
    };
    for (address, comment) in map {
        let Some(text) = comment.get("text").and_then(|text| text.as_str()) else { continue };
        let author =
            comment.get("author").and_then(|author| author.as_str()).map(|author| author.to_string());
        let mut replies = Vec::new();
        if let Some(list) = comment.get("replies").and_then(|replies| replies.as_array()) {
            for reply in list {
                let Some(text) = reply.get("text").and_then(|text| text.as_str()) else { continue };
                let author =
                    reply.get("author").and_then(|author| author.as_str()).map(|author| author.to_string());
                replies.push(Reply { text: text.to_string(), author });
            }
        }
        let resolved = comment.get("resolved").and_then(|resolved| resolved.as_bool()).unwrap_or(false);
        comments.insert(address.to_string(), Comment { text: text.to_string(), author, replies, resolved });
    }
    comments
}

/// The comments of a sheet, as they are kept.
pub fn comments_into(comments: &BTreeMap<String, Comment>) -> Json {
    let mut map = craftkit_sdk::serde_json::Map::new();
    for (address, comment) in comments {
        let mut one = craftkit_sdk::serde_json::Map::new();
        one.insert("text".to_string(), Json::String(comment.text.clone()));
        if let Some(author) = &comment.author {
            one.insert("author".to_string(), Json::String(author.clone()));
        }
        one.insert("resolved".to_string(), Json::Bool(comment.resolved));
        let replies: Vec<Json> = comment
            .replies
            .iter()
            .map(|reply| {
                let mut one = craftkit_sdk::serde_json::Map::new();
                one.insert("text".to_string(), Json::String(reply.text.clone()));
                if let Some(author) = &reply.author {
                    one.insert("author".to_string(), Json::String(author.clone()));
                }
                Json::Object(one)
            })
            .collect();
        one.insert("replies".to_string(), Json::Array(replies));
        map.insert(address.clone(), Json::Object(one));
    }
    Json::Object(map)
}

/// The **comment** of a cell, as it is read.
pub fn comment_of(comment: &Comment) -> Json {
    let mut one = craftkit_sdk::serde_json::Map::new();
    one.insert("text".to_string(), Json::String(comment.text.clone()));
    one.insert(
        "author".to_string(),
        match &comment.author {
            Some(author) => Json::String(author.clone()),
            None => Json::Null,
        },
    );
    one.insert("resolved".to_string(), Json::Bool(comment.resolved));
    let replies: Vec<Json> = comment
        .replies
        .iter()
        .map(|reply| {
            let mut one = craftkit_sdk::serde_json::Map::new();
            one.insert("text".to_string(), Json::String(reply.text.clone()));
            one.insert(
                "author".to_string(),
                match &reply.author {
                    Some(author) => Json::String(author.clone()),
                    None => Json::Null,
                },
            );
            Json::Object(one)
        })
        .collect();
    one.insert("replies".to_string(), Json::Array(replies));
    Json::Object(one)
}

commands! {
    /// Put a **comment** on a cell.
    ///
    /// A comment that is already on the cell is **edited**: the text is
    /// the new text, the author is kept unless a new one is given, and the
    /// replies and the resolved flag stay. A **note** on the cell goes
    /// away: the tree's own rule, that a note replaces a comment and a
    /// comment replaces a note.
    grid.cell.comment.set {
        area: "grid",
        layer: "grid",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The cell's address.
            cell: Text,
            /// The text of the comment.
            text: Text,
            /// Who wrote it.
            author: Option<Text>,
        },
        run: {
            let id = "grid.cell.comment.set".to_string();
            let sheet = self.sheet.0;
            let cell = crate::cell_at(&self.cell.0, &id)?;
            let text = self.text.0.clone();
            if text.is_empty() {
                return Err(EngineError::Failed { id: id.clone(), message: "a comment holds text".to_string() });
            }
            crate::notes::sheet_of(session, sheet, &id)?;
            let author = self.author.as_ref().map(|author| author.0.clone()).filter(|author| !author.is_empty());
            let Some(node) = session.document_mut()?.node_mut(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            let mut comments = comments_of(node);
            let address = cell.to_text();
            // The **author** of an edited comment is kept unless a new one
            // is given: the tree's own rule.
            let author = author.or_else(|| comments.get(&address).and_then(|comment| comment.author.clone()));
            // The **replies** and the **resolved** flag of an edited
            // comment stay: the tree's own rule.
            let held = comments.get(&address);
            let replies = held.map(|comment| comment.replies.clone()).unwrap_or_default();
            let resolved = held.map(|comment| comment.resolved).unwrap_or(false);
            comments.insert(address.clone(), Comment { text, author: author.clone(), replies, resolved });
            node.data["comments"] = comments_into(&comments);
            // A **note replaces a comment**: the note on the cell goes away.
            let mut notes = crate::notes::notes_of(node);
            let took_note = notes.remove(&address).is_some();
            node.data["notes"] = crate::notes::notes_into(&notes);
            let who = match &author {
                Some(author) => format!(" by {author}"),
                None => String::new(),
            };
            Ok(Outcome::changed(format!("wrote a comment on {address}{who}")).with_data(json!({
                "sheet": sheet,
                "cell": address,
                "text": comments.get(&address).map(|comment| comment.text.clone()),
                "author": author,
                "took_note": took_note,
            })))

        }
    }

    /// **Reply** to the comment of a cell.
    ///
    /// The reply joins the thread of the cell. A cell that has no comment
    /// has no thread to join, and that is refused by name.
    grid.cell.comment.reply {
        area: "grid",
        layer: "grid",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The cell's address.
            cell: Text,
            /// The text of the reply.
            text: Text,
            /// Who wrote it.
            author: Option<Text>,
        },
        run: {
            let id = "grid.cell.comment.reply".to_string();
            let sheet = self.sheet.0;
            let cell = crate::cell_at(&self.cell.0, &id)?;
            let text = self.text.0.clone();
            if text.is_empty() {
                return Err(EngineError::Failed { id: id.clone(), message: "a reply holds text".to_string() });
            }
            crate::notes::sheet_of(session, sheet, &id)?;
            let author = self.author.as_ref().map(|author| author.0.clone()).filter(|author| !author.is_empty());
            let Some(node) = session.document_mut()?.node_mut(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            let mut comments = comments_of(node);
            let address = cell.to_text();
            let Some(comment) = comments.get_mut(&address) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no comment in {address}"),
                });
            };
            comment.replies.push(Reply { text: text.clone(), author: author.clone() });
            let replies = comment.replies.len();
            node.data["comments"] = comments_into(&comments);
            Ok(Outcome::changed(format!("replied to the comment on {address}")).with_data(json!({
                "sheet": sheet,
                "cell": address,
                "text": text,
                "author": author,
                "replies": replies,
            })))

        }
    }

    /// **Resolve** the comment of a cell, or leave it unresolved again.
    grid.cell.comment.resolve {
        area: "grid",
        layer: "grid",
        icon: "check",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The cell's address.
            cell: Text,
            /// Whether the thread is resolved: yes, unless it is said no.
            resolved: Option<craftkit_sdk::Bool>,
        },
        run: {
            let id = "grid.cell.comment.resolve".to_string();
            let sheet = self.sheet.0;
            let cell = crate::cell_at(&self.cell.0, &id)?;
            crate::notes::sheet_of(session, sheet, &id)?;
            let resolved = self.resolved.map(|flag| flag.0).unwrap_or(true);
            let Some(node) = session.document_mut()?.node_mut(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            let mut comments = comments_of(node);
            let address = cell.to_text();
            let Some(comment) = comments.get_mut(&address) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no comment in {address}"),
                });
            };
            comment.resolved = resolved;
            node.data["comments"] = comments_into(&comments);
            Ok(Outcome::changed(format!(
                "{} the comment on {address}",
                if resolved { "resolved" } else { "unresolved" }
            ))
            .with_data(json!({ "sheet": sheet, "cell": address, "resolved": resolved })))

        }
    }

    /// The **comment** of a cell: the thread and its replies.
    grid.cell.comment.get {
        area: "grid",
        layer: "grid",
        icon: "list",
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The cell's address.
            cell: Text,
        },
        run: {
            let id = "grid.cell.comment.get".to_string();
            let sheet = self.sheet.0;
            let cell = crate::cell_at(&self.cell.0, &id)?;
            crate::notes::sheet_of(session, sheet, &id)?;
            let Some(node) = session.document().node(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            let comments = comments_of(node);
            let address = cell.to_text();
            let comment = comments.get(&address);
            Ok(Outcome::new(match &comment {
                Some(_) => format!("{address} holds a comment"),
                None => format!("{address} holds no comment"),
            })
            .with_data(json!({
                "sheet": sheet,
                "cell": address,
                "comment": match comment {
                    Some(comment) => comment_of(comment),
                    None => Json::Null,
                },
            })))

        }
    }

    /// Take the **comment** away from a cell.
    ///
    /// A cell that holds no comment is not refused: there is nothing to
    /// take away, and the answer says so.
    grid.cell.comment.delete {
        area: "grid",
        layer: "grid",
        icon: "minus",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The cell's address.
            cell: Text,
        },
        run: {
            let id = "grid.cell.comment.delete".to_string();
            let sheet = self.sheet.0;
            let cell = crate::cell_at(&self.cell.0, &id)?;
            crate::notes::sheet_of(session, sheet, &id)?;
            let Some(node) = session.document_mut()?.node_mut(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            let mut comments = comments_of(node);
            let address = cell.to_text();
            let was_there = comments.remove(&address).is_some();
            node.data["comments"] = comments_into(&comments);
            let outcome = if was_there {
                Outcome::changed(format!("took the comment off {address}"))
            } else {
                Outcome::new(format!("{address} held no comment"))
            };
            Ok(outcome.with_data(json!({ "sheet": sheet, "cell": address, "was_there": was_there })))

        }
    }
}
