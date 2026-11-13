//! The **theme** of a document.
//!
//! A theme is the pair of fonts a document uses — the **major** font
//! of the headings and the **minor** font of the body — and the
//! **accent colour** the headings are drawn in.
//!
//! The theme is **under** the styles: a style that says nothing of
//! its own takes the font of the theme, and a heading that says
//! nothing of its own takes the accent colour. So the theme is
//! **derived** into the character of every style, and a theme that
//! is changed is felt by every style that says nothing of its own.
//!
//! # The rules
//!
//! - a **font** is not nothing: a font that is empty is refused —
//!   the tree's rule for a font;
//! - a **colour** is `#rrggbb`, and an **empty** colour takes the
//!   accent away — the tree's rule for a colour;
//! - a part that is **not named** keeps what it was;
//! - a **heading** is a style whose name begins with `Heading`, and
//!   the `Title` — the tree's rule for what a heading is.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Node, NodeRef, Outcome, Text};

/// The **theme** of a document, as it is kept.
pub fn theme_of(node: &Node) -> Json {
    node.data
        .get("theme")
        .and_then(|theme| theme.as_object())
        .cloned()
        .map(Json::Object)
        .unwrap_or_else(|| Json::Object(craftkit_sdk::serde_json::Map::new()))
}

/// The **document** a style is in: the styles are the children of the
/// document.
pub fn document_of(document: &Document, style: u64) -> Option<u64> {
    document.walk().find(|node| node.children.iter().any(|child| child.id == style)).map(|node| node.id)
}

/// Whether a style is a **heading**: the tree's rule.
pub fn is_heading(style: &Node) -> bool {
    let name = style.name.as_str();
    name.starts_with("Heading") || name == "Title"
}

/// The **character** the theme gives a style: the font it does not
/// say of its own, and the accent colour a heading does not say.
pub fn character_of_theme(document: &Document, style: u64) -> Json {
    let mut character = craftkit_sdk::serde_json::Map::new();
    let Some(node) = document.node(style) else { return Json::Object(character) };
    let Some(document_id) = document_of(document, style) else { return Json::Object(character) };
    let Some(theme) = document.node(document_id).map(theme_of) else { return Json::Object(character) };
    let heading = is_heading(node);
    // The **font**: the major font of a heading, the minor font of
    // the body.
    let font = if heading { theme.get("major") } else { theme.get("minor") };
    if let Some(font) = font {
        character.insert("font".to_string(), font.clone());
    }
    // The **accent colour**: a heading that says no colour of its own
    // takes it.
    if heading && let Some(accent) = theme.get("accent") {
        character.insert("colour".to_string(), accent.clone());
    }
    Json::Object(character)
}

commands! {
    /// **Say** what the **theme** of a document is.
    ///
    /// A part that is not named keeps what it was, and an **empty**
    /// colour takes the accent away.
    prose.document.theme {
        area: "prose",
        layer: "prose",
        icon: "paint",
        mutates,
        undoable,
        params: {
            /// The document.
            document: NodeRef,
            /// The font of the headings.
            ///
            /// It is not nothing, and an empty font is refused.
            major: Option<Text>,
            /// The font of the body.
            minor: Option<Text>,
            /// The accent colour of the headings, as `#rrggbb`.
            ///
            /// An empty colour takes the accent away.
            accent: Option<Text>,
        },
        run: {
            let id = "prose.document.theme".to_string();
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
            let mut theme = theme_of(&node);
            if let Some(major) = &self.major {
                let major = major.0.trim();
                if major.is_empty() {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "a font is not nothing".to_string(),
                    });
                }
                theme["major"] = json!(major);
            }
            if let Some(minor) = &self.minor {
                let minor = minor.0.trim();
                if minor.is_empty() {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "a font is not nothing".to_string(),
                    });
                }
                theme["minor"] = json!(minor);
            }
            if let Some(accent) = &self.accent {
                let accent = accent.0.trim();
                if accent.is_empty() {
                    if let Some(map) = theme.as_object_mut() {
                        map.remove("accent");
                    }
                } else {
                    if let Err(why) = craftkit_sdk::Colour::from_hex(accent) {
                        return Err(EngineError::Failed { id: id.clone(), message: why });
                    }
                    theme["accent"] = json!(accent);
                }
            }
            let Some(node) = session.document_mut()?.node_mut(document) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no document `{document}`"),
                });
            };
            if theme.as_object().map(|theme| theme.is_empty()).unwrap_or(true) {
                if let Some(map) = node.data.as_object_mut() {
                    map.remove("theme");
                }
            } else {
                node.data["theme"] = theme.clone();
            }
            Ok(Outcome::changed(format!("the theme of the document `{document}` is said"))
                .with_data(json!({ "document": document, "theme": theme })))

        }
    }
}
