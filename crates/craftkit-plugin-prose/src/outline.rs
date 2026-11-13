//! The **outline** of a document.
//!
//! A style may carry a **level**: the level of a heading, from one to
//! nine. A block on a style with a level is a **heading**, and the
//! headings of a document in the order they stand are its **outline**.
//!
//! The level is a fact of the style, and the outline is **derived**:
//! it is worked out from the blocks and their styles every time it is
//! asked for, so an outline cannot be out of date with the document.
//!
//! The rules:
//!
//! - a level that is **not a whole number from one to nine** is
//!   refused;
//! - a style with **no level** is not a heading, and a block on a
//!   style with no level is not in the outline;
//! - the level of a style is the level of its **base** when it has
//!   none of its own — the same bargain as the character (§468) and
//!   the paragraph (§473) — so a style based on a heading is a
//!   heading, and the chain is **bounded**;
//! - the text of a heading is the text the block **shows**, so a
//!   heading that is a cross-reference shows what it references.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Number, Outcome};

use crate::styles::chain_of;

/// The **least** a level may be.
pub const MIN_LEVEL: u32 = 1;

/// The **most** a level may be.
pub const MAX_LEVEL: u32 = 9;

/// The **level** a style carries of its own.
pub fn level_of(style: &Node) -> Option<u32> {
    style.data.get("level").and_then(|level| level.as_u64()).map(|level| level as u32)
}

/// The **level** of a style, with the level of its base under it.
///
/// The chain is walked from the furthest base to the style itself, so
/// that the nearer style wins.
pub fn effective_level(document: &craftkit_sdk::Document, style: u64) -> Option<u32> {
    let mut level = None;
    for id in chain_of(document, style) {
        let Some(node) = document.node(id) else { continue };
        if let Some(found) = level_of(node) {
            level = Some(found);
        }
    }
    level
}

commands! {
    /// **Set** the level of a style — or take it away.
    ///
    /// The level of a style is the level of its base when it has none of
    /// its own, so a style based on a heading is a heading.
    prose.style.level {
        area: "prose",
        layer: "prose",
        icon: "list",
        mutates,
        undoable,
        params: {
            /// The style.
            style: NodeRef,
            /// The level: a whole number from 1 to 9. Without it, the style
            /// has no level of its own.
            level: Option<Number>,
        },
        run: {
            let id = "prose.style.level".to_string();
            let style = self.style.0;
            let Some(node) = session.document().node(style) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no style `{style}`"),
                });
            };
            if node.kind != "prose.style" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{style}` is a `{}` and not a style", node.kind),
                });
            }
            let level = match &self.level {
                Some(level) => {
                    let value = level.0;
                    if !value.is_finite() || value.fract() != 0.0 {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("a level is a whole number: {value} is not one"),
                        });
                    }
                    if value < f64::from(MIN_LEVEL) || value > f64::from(MAX_LEVEL) {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!(
                                "a level is a whole number from {MIN_LEVEL} to {MAX_LEVEL}: {value} is not one"
                            ),
                        });
                    }
                    Some(value as u32)
                }
                None => None,
            };
            if let Some(node) = session.document_mut()?.node_mut(style) {
                match level {
                    Some(level) => node.data["level"] = json!(level),
                    None => {
                        if let Some(map) = node.data.as_object_mut() {
                            map.remove("level");
                        }
                    }
                }
            }
            Ok(Outcome::changed(format!("the level of the style `{style}` is set"))
                .with_data(json!({ "style": style, "level": level })))

        }
    }

    /// **Read** the level of a style, with the level of its base under
    /// it.
    prose.style.level.read {
        area: "prose",
        layer: "prose",
        icon: "list",
        params: {
            /// The style.
            style: NodeRef,
        },
        run: {
            let id = "prose.style.level.read".to_string();
            let style = self.style.0;
            let Some(node) = session.document().node(style) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no style `{style}`"),
                });
            };
            if node.kind != "prose.style" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{style}` is a `{}` and not a style", node.kind),
                });
            }
            let level = effective_level(session.document(), style);
            Ok(Outcome::new(format!("the style `{style}` has a level")).with_data(json!({
                "style": style,
                "level": match level {
                    Some(level) => json!(level),
                    None => Json::Null,
                },
            })))

        }
    }

    /// **Read** the outline of a document: its headings, in the order
    /// they stand.
    ///
    /// The outline is **derived**: it is worked out from the blocks and
    /// their styles every time it is asked for.
    prose.document.outline {
        area: "prose",
        layer: "prose",
        icon: "list",
        params: {
            /// The document.
            document: NodeRef,
        },
        run: {
            let id = "prose.document.outline".to_string();
            let document = self.document.0;
            let Some(node) = session.document().node(document).cloned() else {
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
            let mut outline: Vec<Json> = Vec::new();
            for block in node.children.iter().filter(|child| child.kind.starts_with("prose.block.")) {
                let Some(style) = crate::styles::style_of_block(session.document(), block.id) else {
                    continue;
                };
                let Some(level) = effective_level(session.document(), style.id) else { continue };
                outline.push(json!({
                    "block": block.id,
                    "style": style.id,
                    "style_name": style.name,
                    "level": level,
                    "text": crate::references::text_of(session.document(), block),
                }));
            }
            Ok(Outcome::new(format!("the document has {} heading(s)", outline.len())).with_data(json!({
                "document": document,
                "outline": outline,
                "count": outline.len(),
            })))

        }
    }
}
