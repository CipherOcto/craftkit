//! The **name** of a clip.
//!
//! A clip may be **named**: the caller says what it is called, and the
//! name is the clip's own — it does not follow the clip when the clip
//! moves.
//!
//! A clip that has **not** been named is shown by the place it is at —
//! `clip at 120` — and that name is **derived**: it follows the clip
//! when the clip moves, and it is not written down as a name of its
//! own. So the name of a clip is a **fact** once it is given and a
//! **derivation** until then, and both are told apart by the payload:
//! a clip with a `name` has one, and a clip without has not.
//!
//! The tree's `clip.rename` renames an **item** or a **clip**, and it
//! renames a clip of the tree by its name and not by its place — the
//! tree's clip keeps its name when it is moved, and so does this one.
//!
//! # What the port found
//!
//! - **The tree's clip has a name and this tree's had none.** The
//!   tree's clip is an item of a track and it has a name, and the name
//!   is what the timeline shows. Here a clip was named by the place it
//!   was at, and every edit that moved it wrote the name again. The
//!   name is therefore a fact and a derivation, and the payload says
//!   which one it is.

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session, Text};

/// The name a clip has been **given**, if it has one.
pub fn given_name(data: &serde_json::Value) -> Option<String> {
    data.get("name")
        .and_then(|name| name.as_str())
        .filter(|name| !name.is_empty())
        .map(|name| name.to_string())
}

/// **Name** a clip.
///
/// The name is the clip's own, and it survives the clip being moved.
#[derive(Command)]
#[command(id = "film.clip.rename", area = "film", layer = "film", icon = "pencil", mutates, undoable)]
pub struct RenameClip {
    /// The clip.
    pub clip: NodeRef,
    /// What to call it.
    pub name: Text,
}

impl RenameClip {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.clip.rename".to_string();
        let clip = self.clip.0;
        let Some(node) = session.document().node(clip) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{clip}`"),
            });
        };
        if node.kind != "film.clip" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{clip}` is a `{}` and not a clip", node.kind),
            });
        }
        let name = self.name.0.trim().to_string();
        if name.is_empty() {
            return Err(EngineError::Failed { id: id.clone(), message: "a clip has a name".to_string() });
        }
        let start = node.data.get("start").and_then(|start| start.as_u64()).unwrap_or(0);
        if let Some(node) = session.document_mut()?.node_mut(clip) {
            node.data["name"] = serde_json::json!(name);
            crate::set_clip_name(node, start);
        }
        Ok(Outcome::changed(format!("the clip `{clip}` is called `{name}`"))
            .with_data(serde_json::json!({ "clip": clip, "name": name })))
    }
}
