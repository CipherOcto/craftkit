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

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, Node, NodeRef, Outcome, Session, Text};

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
