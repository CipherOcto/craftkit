//! The **alt text** of a picture.
//!
//! A picture that says nothing about itself cannot be read by a
//! screen reader, and the text that stands for it is the picture's
//! **alt text**: the words that say what the picture is.
//!
//! The rules:
//!
//! - a node that is **not a media** may not have alt text: the text
//!   stands for a picture, and a paragraph is not a picture;
//! - an **empty** alt text **takes the text away**: a picture that has
//!   no text and a picture whose text was taken away are the same
//!   picture, and the second is the first after the fact.
//!
//! And a picture may be **decorative**: it says nothing about itself
//! on purpose — it is a rule, a flourish, a mark a reader may skip —
//! and a decorative picture is not asked for words, because words
//! would be wrong.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Bool, EngineError, Node, NodeRef, Outcome, Session, Text};

/// The **alt text** of a media: what it says about itself.
///
/// A media that says nothing has no alt text, and the answer is
/// nothing — not an empty text that means the same thing.
pub fn alt_of(media: &Node) -> Option<String> {
    media
        .data
        .get("alt")
        .and_then(|alt| alt.as_str())
        .map(|alt| alt.to_string())
        .filter(|alt| !alt.trim().is_empty())
}

/// Whether a picture is **decorative**: it says nothing about itself
/// on purpose.
pub fn is_decorative(media: &Node) -> bool {
    media.data.get("decorative").and_then(|decorative| decorative.as_bool()).unwrap_or(false)
}

/// **Say** that a picture is **decorative**.
///
/// A decorative picture is one that says nothing about itself **on
/// purpose**: it is a rule, a flourish, a mark that a reader may
/// skip. A screen reader reads it out as nothing, and the checker
/// does not ask for words that would be wrong.
///
/// The rules:
///
/// - a node that is **not a picture** is refused;
/// - a picture that is **not said** to be decorative keeps the fact it
///   has — the words "not said" leave the picture as it was.
#[derive(Command)]
#[command(id = "prose.media.decorative", area = "prose", layer = "prose", icon = "pencil", mutates, undoable)]
pub struct SetDecorative {
    /// The picture.
    pub media: NodeRef,
    /// Whether the picture is decorative.
    ///
    /// Leave it out and the picture keeps the fact it has.
    pub decorative: Option<Bool>,
}

impl SetDecorative {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.media.decorative".to_string();
        let media = self.media.0;
        let Some(node) = session.document().node(media).cloned() else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{media}`"),
            });
        };
        if node.kind != "prose.media" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{media}` is a `{}` and not a picture", node.kind),
            });
        }
        let mut data = node.data.clone();
        match &self.decorative {
            Some(decorative) if decorative.0 => {
                data["decorative"] = json!(true);
            }
            Some(_) => {
                // A picture that is **not** decorative is not decorative:
                // the fact is taken away.
                if let Some(map) = data.as_object_mut() {
                    map.remove("decorative");
                }
            }
            None => {}
        }
        let decorative = data.get("decorative").and_then(|decorative| decorative.as_bool()).unwrap_or(false);
        if let Some(node) = session.document_mut()?.node_mut(media) {
            node.data = data;
        }
        Ok(Outcome::changed(if decorative {
            format!("the picture `{media}` is decorative")
        } else {
            format!("the picture `{media}` is not decorative")
        })
        .with_data(json!({ "media": media, "decorative": decorative })))
    }
}

/// **Say** what a picture is.
#[derive(Command)]
#[command(id = "prose.media.alt", area = "prose", layer = "prose", icon = "pencil", mutates, undoable)]
pub struct SetAlt {
    /// The picture.
    pub media: NodeRef,
    /// The text that stands for it.
    ///
    /// An empty text takes the alt text away.
    pub alt: Text,
}

impl SetAlt {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.media.alt".to_string();
        let media = self.media.0;
        let Some(node) = session.document().node(media).cloned() else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{media}`"),
            });
        };
        if node.kind != "prose.media" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{media}` is a `{}` and not a picture", node.kind),
            });
        }
        let alt = self.alt.0.trim().to_string();
        let node = session.document_mut()?.node_mut(media);
        let Some(node) = node else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{media}`"),
            });
        };
        if alt.is_empty() {
            // An **empty** value takes the fact away.
            if let Some(map) = node.data.as_object_mut() {
                map.remove("alt");
            }
        } else {
            node.data["alt"] = json!(alt.clone());
        }
        Ok(Outcome::changed(if alt.is_empty() {
            format!("the picture `{media}` says nothing about itself")
        } else {
            format!("the picture `{media}` says what it is")
        })
        .with_data(json!({ "media": media, "alt": alt })))
    }
}
