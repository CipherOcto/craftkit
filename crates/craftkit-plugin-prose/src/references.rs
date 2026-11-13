//! The **cross-reference** of a document.
//!
//! A block may **reference** another: it stands for the text of a
//! block that carries a **bookmark**. The text it shows is
//! **derived** — worked out from the block it points at — and it is
//! never written down, so a reference cannot be out of date.
//!
//! The reference is an **edge**: from the block that references to the
//! block that is referenced, and the name of the bookmark is the
//! payload. The name is a fact of the document, and the edge is the
//! reference.
//!
//! The rules:
//!
//! - a **bookmark** that is not in the document is refused: a
//!   reference to a name that is not there points at nothing;
//! - a block that points at a block with **no text** shows **nothing**,
//!   and nothing is not a text of zero characters and a half;
//! - the text of a block that is **not** a reference is the text it
//!   stores.
//!
//! # The kinds of target
//!
//! A reference points at a **bookmark** — the tree's own default — or
//! at a **caption**:
//!
//! - `figure`, `table` and `equation` are the three labels of the
//!   tree, and `label:<Name>` is a label of the person's own;
//! - the target of a caption is its **number** or its **text**, and
//!   the text of the caption may begin with it;
//! - a caption that is pointed at is shown as its **text**, or as its
//!   **number** when the reference says so — and the number of a
//!   caption that names a chapter is the whole number, the chapter and
//!   the number together;
//! - a kind of target that is not one of the tree's is refused **by
//!   name**, and so is a way of showing a reference that is not one of
//!   the tree's two.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Outcome, Text};

/// The **kind** of the edge that says a block references another.
pub const REFERENCES: &str = "prose.references";

/// The **kind** of a block that references another.
pub const REFERENCE_KIND: &str = "prose.block.reference";

/// The block a reference **points at**.
pub fn target_of(document: &craftkit_sdk::Document, block: u64) -> Option<u64> {
    document
        .relations
        .iter()
        .find(|relation| relation.kind == REFERENCES && relation.from == block)
        .map(|relation| relation.to)
}

/// The **text** of a block.
///
/// For a reference it is the text of the block it points at, and it is
/// **derived**: it is worked out every time, and it is never stored.
/// A reference that shows a **number** shows the number of the caption
/// it points at.
pub fn text_of(document: &craftkit_sdk::Document, block: &Node) -> Json {
    if block.kind != REFERENCE_KIND {
        return block.data.get("text").cloned().unwrap_or(Json::Null);
    }
    let Some(target) = target_of(document, block.id) else { return Json::Null };
    let Some(node) = document.node(target) else { return Json::Null };
    if shows_a_number(block) {
        return number_of(document, block, target);
    }
    match crate::captions::caption_of(node) {
        Some(caption) => caption.get("text").cloned().unwrap_or(Json::Null),
        None => node.data.get("text").cloned().unwrap_or(Json::Null),
    }
}

/// Whether a reference shows the **number** of what it points at.
fn shows_a_number(block: &Node) -> bool {
    block.data.get("show").and_then(|show| show.as_str()) == Some("number")
}

/// The **number** of the caption a reference points at.
///
/// The tree's own rules: the number of a caption is derived from the
/// captions of the document, and a caption that names a chapter has
/// the whole number — the chapter and the number together. And the
/// tree's number of a caption is the **label and the number** together:
/// the tree reads it out of the text of the caption, where the label
/// and the number are one piece of text, and here they are two facts
/// of the caption, so they are put together the same way.
fn number_of(document: &craftkit_sdk::Document, block: &Node, target: u64) -> Json {
    let Some(node) = document_of(document, block.id) else { return Json::Null };
    let captions = crate::captions::captions_of(document, node);
    let Some(caption) = captions.as_array().and_then(|captions| {
        captions.iter().find(|caption| caption.get("block").and_then(|held| held.as_u64()) == Some(target))
    }) else {
        return Json::Null;
    };
    let label =
        caption.get("label").and_then(|label| label.as_str()).unwrap_or(crate::captions::DEFAULT_LABEL);
    let number = match caption.get("full").and_then(|full| full.as_str()) {
        Some(full) if !full.is_empty() => full.to_string(),
        _ => match caption.get("number").and_then(|number| number.as_str()) {
            Some(number) => number.to_string(),
            None => return Json::Null,
        },
    };
    Json::String(format!("{label} {number}"))
}

/// The **document** a block is in.
fn document_of(document: &craftkit_sdk::Document, block: u64) -> Option<&Node> {
    document
        .walk()
        .find(|node| node.kind == "prose.document" && node.children.iter().any(|child| child.id == block))
}

/// The block of a document that carries a **bookmark** of that name.
fn bookmarked(document: &craftkit_sdk::Document, parent: u64, name: &str) -> Option<u64> {
    document
        .node(parent)?
        .children
        .iter()
        .filter(|child| child.kind.starts_with("prose.block."))
        .find(|child| crate::bookmarks::bookmark_of(child).as_deref() == Some(name))
        .map(|child| child.id)
}

/// The **label** a kind of target names, if it names one.
///
/// The tree's own kinds: the three labels of the tree — `figure`,
/// `table` and `equation` — and `label:<Name>` for a label of the
/// person's own. A kind that is not one of them is refused by name.
fn label_of(to: &str, id: &str) -> Result<Option<String>, EngineError> {
    let to = to.trim();
    if to.is_empty() || to.eq_ignore_ascii_case("bookmark") {
        return Ok(None);
    }
    if let Some(label) = to.strip_prefix("label:") {
        return Ok(Some(crate::captions::label_of(label, id)?));
    }
    match to.to_ascii_lowercase().as_str() {
        "figure" => Ok(Some("Figure".to_string())),
        "table" => Ok(Some("Table".to_string())),
        "equation" => Ok(Some("Equation".to_string())),
        _ => Err(EngineError::Failed {
            id: id.to_string(),
            message: format!(
                "the target `{to}` is not one of this tree's: it is one of bookmark, figure, table, equation, label:<Name>"
            ),
        }),
    }
}

/// The **ways** a reference may show what it points at.
const SHOWS: [&str; 2] = ["text", "number"];

/// The way a reference shows what it points at: `text` when it is not
/// said, and a way that is not one of the tree's two is refused by
/// name.
fn show_of(show: Option<String>, id: &str) -> Result<String, EngineError> {
    let Some(show) = show else { return Ok("text".to_string()) };
    let show = show.to_ascii_lowercase();
    if !SHOWS.contains(&show.as_str()) {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!(
                "the way of showing `{show}` is not one of this tree's: it is one of text, number"
            ),
        });
    }
    Ok(show)
}

/// The **caption** of a document that a target names.
///
/// The tree's own rule: the target is the **number** of the caption
/// or its **text**, and the text of the caption may begin with it.
fn captioned(document: &craftkit_sdk::Document, parent: u64, label: &str, target: &str) -> Option<u64> {
    let node = document.node(parent)?;
    let captions = crate::captions::captions_of(document, node);
    let captions = captions.as_array()?;
    let caption = captions.iter().find(|caption| {
        let held =
            caption.get("label").and_then(|held| held.as_str()).unwrap_or(crate::captions::DEFAULT_LABEL);
        if held != label {
            return false;
        }
        let text = caption.get("text").and_then(|text| text.as_str()).unwrap_or_default();
        let number = caption.get("number").and_then(|number| number.as_str()).unwrap_or_default();
        let full = caption.get("full").and_then(|full| full.as_str()).unwrap_or_default();
        number == target || full == target || text == target || text.starts_with(target)
    })?;
    caption.get("block").and_then(|block| block.as_u64())
}

commands! {
    /// **Add** a cross-reference to a document.
    ///
    /// The reference is a block that stands for the text of the block that
    /// carries the bookmark, and its text is derived from that block.
    prose.block.reference.add {
        area: "prose",
        layer: "prose",
        icon: "plus",
        mutates,
        undoable,
        params: {
            /// The document it goes in.
            document: NodeRef,
            /// The **bookmark** it points at — or the **number** or the
            /// **text** of the caption it points at.
            target: Text,
            /// What it points at.
            ///
            /// `bookmark` — the default — `figure`, `table`, `equation`, or
            /// `label:<Name>` for a label of the person's own.
            to: Option<Text>,
            /// How it shows what it points at.
            ///
            /// `text` — the default — or `number`.
            show: Option<Text>,
        },
        run: {
            let id = "prose.block.reference.add".to_string();
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
            let name = self.target.0.trim().to_string();
            if name.is_empty() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a reference points at a bookmark, and a bookmark has a name".to_string(),
                });
            }
            let to = self.to.as_ref().map(|to| to.0.trim().to_string()).unwrap_or_default();
            let label = label_of(&to, &id)?;
            let show = show_of(self.show.as_ref().map(|show| show.0.trim().to_string()), &id)?;
            let (target, payload) = match &label {
                None => {
                    if show == "number" {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: "a bookmark has no number, and only a caption has one".to_string(),
                        });
                    }
                    let Some(target) = bookmarked(session.document(), document, &name) else {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("there is no bookmark `{name}` in the document"),
                        });
                    };
                    (target, json!({ "bookmark": name.clone() }))
                }
                Some(label) => {
                    let Some(target) = captioned(session.document(), document, label, &name) else {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("there is no caption of the label `{label}` that is `{name}`"),
                        });
                    };
                    (target, json!({ "caption": name.clone(), "label": label.clone() }))
                }
            };
            let block = session.document_mut()?.add_node_with_under(
                Some(document),
                name.clone(),
                REFERENCE_KIND,
                None,
                json!({ "target": name.clone(), "to": to, "show": show }),
            );
            session
                .document_mut()?
                .add_relation(craftkit_sdk::Relation::with_payload(block, target, REFERENCES, payload));
            Ok(Outcome::changed(format!("the document references `{name}`"))
                .with_data(json!({ "block": block, "target": target, "bookmark": name })))

        }
    }

    /// **Point** a cross-reference at another bookmark.
    ///
    /// The text of the reference changes with the block it points at, and
    /// the text it had is gone: a reference is never out of date.
    prose.block.reference.set {
        area: "prose",
        layer: "prose",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The reference.
            block: NodeRef,
            /// The **bookmark** it points at — or the **number** or the
            /// **text** of the caption it points at.
            target: Text,
            /// What it points at.
            ///
            /// `bookmark` — the default — `figure`, `table`, `equation`, or
            /// `label:<Name>` for a label of the person's own.
            to: Option<Text>,
            /// How it shows what it points at.
            ///
            /// `text` — the default — or `number`.
            show: Option<Text>,
        },
        run: {
            let id = "prose.block.reference.set".to_string();
            let block = self.block.0;
            let Some(node) = session.document().node(block).cloned() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no block `{block}`"),
                });
            };
            if node.kind != REFERENCE_KIND {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{block}` is a `{}` and not a reference", node.kind),
                });
            }
            let name = self.target.0.trim().to_string();
            if name.is_empty() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a reference points at a bookmark, and a bookmark has a name".to_string(),
                });
            }
            // The **document** the reference is in.
            let document = session
                .document()
                .walk()
                .find(|node| node.kind == "prose.document" && node.children.iter().any(|child| child.id == block))
                .map(|node| node.id);
            let Some(document) = document else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the reference `{block}` is in no document"),
                });
            };
            let to = self.to.as_ref().map(|to| to.0.trim().to_string()).unwrap_or_default();
            let label = label_of(&to, &id)?;
            let show = show_of(self.show.as_ref().map(|show| show.0.trim().to_string()), &id)?;
            let (target, payload) = match &label {
                None => {
                    if show == "number" {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: "a bookmark has no number, and only a caption has one".to_string(),
                        });
                    }
                    let Some(target) = bookmarked(session.document(), document, &name) else {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("there is no bookmark `{name}` in the document"),
                        });
                    };
                    (target, json!({ "bookmark": name.clone() }))
                }
                Some(label) => {
                    let Some(target) = captioned(session.document(), document, label, &name) else {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("there is no caption of the label `{label}` that is `{name}`"),
                        });
                    };
                    (target, json!({ "caption": name.clone(), "label": label.clone() }))
                }
            };
            session
                .document_mut()?
                .remove_relations(|relation| relation.kind == REFERENCES && relation.from == block);
            if let Some(node) = session.document_mut()?.node_mut(block) {
                node.data["target"] = json!(name.clone());
                node.data["to"] = json!(to);
                node.data["show"] = json!(show);
            }
            session
                .document_mut()?
                .add_relation(craftkit_sdk::Relation::with_payload(block, target, REFERENCES, payload));
            Ok(Outcome::changed(format!("the reference points at `{name}`"))
                .with_data(json!({ "block": block, "target": target, "bookmark": name })))

        }
    }

    /// **Read** the text of a block: the text it shows, which for a
    /// reference is the text of the block it points at.
    prose.block.reference.read {
        area: "prose",
        layer: "prose",
        icon: "list",
        params: {
            /// The block.
            block: NodeRef,
        },
        run: {
            let id = "prose.block.reference.read".to_string();
            let block = self.block.0;
            let Some(node) = session.document().node(block).cloned() else {
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
            let document = session.document();
            let text = text_of(document, &node);
            let target = target_of(document, block);
            Ok(Outcome::new(format!("the block `{block}` has a text")).with_data(json!({
                "block": block,
                "kind": node.kind,
                "text": text,
                "target": match target {
                    Some(target) => json!(target),
                    None => Json::Null,
                },
            })))

        }
    }
}
