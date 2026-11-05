//! The **character** of a style.
//!
//! A style is a name and what it applies to, and it carries the
//! **character** of the text it applies to: the font, the size, the
//! colour, and whether the text is bold, italic or underlined.
//!
//! A style may be **based on** another, and the character of a style is
//! the character of its base **with its own on top**: a style that
//! says nothing about the size has the size of its base, and a style
//! that says a size has its own.
//!
//! The rules:
//!
//! - a **colour** is the kit's colour, `#rrggbb`, and a text that is
//!   not one is refused;
//! - a **size** is a number that is finite and above zero, and one
//!   that is not is refused;
//! - a **font** is not nothing, and a font that is nothing is refused;
//! - the chain of styles is **bounded**, and a chain that comes back
//!   to itself stops: a cycle is not a hang.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, Node, NodeRef, Number, Outcome, Session, Text};

/// The most a chain of styles may be **long**.
pub const MAX_CHAIN: usize = 32;

/// The **character** a style carries.
pub fn character_of(style: &Node) -> Json {
    style.data.get("character").cloned().unwrap_or(Json::Object(craftkit_sdk::serde_json::Map::new()))
}

/// The **style** a block is on, if it is on one.
pub fn style_of_block(document: &craftkit_sdk::Document, block: u64) -> Option<Node> {
    let relation = document
        .relations
        .iter()
        .find(|relation| relation.kind == "prose.styled_by" && relation.from == block)?;
    document.node(relation.to).cloned()
}

/// The **chain** of a style, from the furthest base to the style
/// itself.
///
/// The walk is **bounded** and it stops at a style it has already
/// seen, so a cycle ends the chain instead of hanging the program.
pub(crate) fn chain_of(document: &craftkit_sdk::Document, style: u64) -> Vec<u64> {
    let mut chain: Vec<u64> = Vec::new();
    let mut seen: Vec<u64> = vec![style];
    let mut current = style;
    for _ in 0..MAX_CHAIN {
        let Some(relation) = document
            .relations
            .iter()
            .find(|relation| relation.kind == "prose.based_on" && relation.from == current)
        else {
            break;
        };
        let base = relation.to;
        chain.push(base);
        if seen.contains(&base) {
            break;
        }
        seen.push(base);
        current = base;
    }
    chain.reverse();
    chain.push(style);
    chain
}

/// The **character** of a style, with the character of its base under
/// it.
///
/// The chain is walked from the furthest base to the style itself, so
/// that the nearer style wins.
pub fn effective_character(document: &craftkit_sdk::Document, style: u64) -> Json {
    let mut character = craftkit_sdk::serde_json::Map::new();
    for id in chain_of(document, style) {
        let Some(node) = document.node(id) else { continue };
        let Some(map) = character_of(node).as_object().cloned() else { continue };
        for (key, value) in map {
            character.insert(key, value);
        }
    }
    Json::Object(character)
}

/// **Set** the character of a style.
///
/// The character that is said is set, and the character that is not
/// said stays as it was: a style that says the size and not the font
/// has the font it had.
#[derive(Command)]
#[command(id = "prose.style.character", area = "prose", layer = "prose", icon = "pencil", mutates, undoable)]
pub struct SetStyleCharacter {
    /// The style.
    pub style: NodeRef,
    /// The **font** of the text.
    pub font: Option<Text>,
    /// The **size** of the text.
    pub size: Option<Number>,
    /// The **colour** of the text, as `#rrggbb`.
    pub colour: Option<Text>,
    /// Whether the text is **bold**.
    pub bold: Option<Bool>,
    /// Whether the text is **italic**.
    pub italic: Option<Bool>,
    /// Whether the text is **underlined**.
    pub underline: Option<Bool>,
}

impl SetStyleCharacter {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.style.character".to_string();
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
        // The **character** that is being put on.
        let mut character = match character_of(node).as_object().cloned() {
            Some(map) => map,
            None => craftkit_sdk::serde_json::Map::new(),
        };
        if let Some(font) = &self.font {
            let font = font.0.trim().to_string();
            if font.is_empty() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a font is not nothing".to_string(),
                });
            }
            character.insert("font".to_string(), Json::String(font));
        }
        if let Some(size) = &self.size {
            let size = size.0;
            if !size.is_finite() || size <= 0.0 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("{size} is not a size: a size is a number above zero"),
                });
            }
            character.insert("size".to_string(), Json::from(size));
        }
        if let Some(colour) = &self.colour {
            let text = colour.0.trim().to_string();
            if let Err(why) = craftkit_sdk::Colour::from_hex(&text) {
                return Err(EngineError::Failed { id: id.clone(), message: why });
            }
            character.insert("colour".to_string(), Json::String(text));
        }
        if let Some(bold) = &self.bold {
            character.insert("bold".to_string(), Json::Bool(bold.0));
        }
        if let Some(italic) = &self.italic {
            character.insert("italic".to_string(), Json::Bool(italic.0));
        }
        if let Some(underline) = &self.underline {
            character.insert("underline".to_string(), Json::Bool(underline.0));
        }
        if character.is_empty() {
            return Err(EngineError::Failed {
                id,
                message: "the command says nothing to change".to_string(),
            });
        }
        if let Some(node) = session.document_mut()?.node_mut(style) {
            node.data["character"] = Json::Object(character);
        }
        Ok(Outcome::changed(format!("the character of the style `{style}` is set"))
            .with_data(json!({ "style": style })))
    }
}

/// **Read** the character of a style, with the character of its base
/// under it.
#[derive(Command)]
#[command(id = "prose.style.character.read", area = "prose", layer = "prose", icon = "list")]
pub struct ReadStyleCharacter {
    /// The style.
    pub style: NodeRef,
}

impl ReadStyleCharacter {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.style.character.read".to_string();
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
        let character = effective_character(session.document(), style);
        Ok(Outcome::new(format!("the style `{style}` has a character")).with_data(json!({
            "style": style,
            "character": character,
            "chain": chain_of(session.document(), style),
        })))
    }
}

/// **Read** the character of a block: the character of the style it is
/// on.
#[derive(Command)]
#[command(id = "prose.block.character", area = "prose", layer = "prose", icon = "list")]
pub struct ReadBlockCharacter {
    /// The block.
    pub block: NodeRef,
}

impl ReadBlockCharacter {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.block.character".to_string();
        let block = self.block.0;
        let Some(node) = session.document().node(block) else {
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
        let Some(style) = style_of_block(session.document(), block) else {
            return Ok(Outcome::new(format!("the block `{block}` is on no style")).with_data(json!({
                "block": block,
                "style": Json::Null,
                "character": Json::Object(craftkit_sdk::serde_json::Map::new()),
            })));
        };
        let character = effective_character(session.document(), style.id);
        Ok(Outcome::new(format!("the block `{block}` is on the style `{}`", style.name)).with_data(json!({
            "block": block,
            "style": style.id,
            "character": character,
        })))
    }
}
