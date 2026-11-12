//! **Taking away** a bin.
//!
//! A bin may be taken away, and what is **in** it goes with it: the
//! items of a bin are the bin's children, and a child goes with its
//! parent.
//!
//! But an item that a clip **plays** may not be taken away — not on
//! its own and not in a bin either: a clip that plays nothing is not
//! a clip. So the removal of a bin is refused when one of its items
//! is played, and the answer says which items are in the way.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session};

/// **Take away** a bin.
///
/// The items of the bin go with it, and the removal is refused when a
/// clip plays one of them.
#[derive(Command)]
#[command(id = "film.bin.remove", area = "film", layer = "film", icon = "minus", mutates, undoable)]
pub struct RemoveBin {
    /// The bin.
    pub bin: NodeRef,
}

impl RemoveBin {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.bin.remove".to_string();
        let bin = self.bin.0;
        let Some(node) = session.document().node(bin) else {
            return Err(EngineError::Failed { id: id.clone(), message: format!("there is no node `{bin}`") });
        };
        if node.kind != "film.bin" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{bin}` is a `{}` and not a bin", node.kind),
            });
        }
        // The **items** of the bin, and the clips that play them.
        let items: Vec<u64> =
            node.children.iter().filter(|child| child.kind == "film.item").map(|child| child.id).collect();
        let mut played: Vec<(u64, u64)> = Vec::new();
        for item in &items {
            for relation in session.document().relations_to(*item, Some("clip")) {
                played.push((*item, relation.from));
            }
        }
        if !played.is_empty() {
            let list: Vec<String> = played
                .iter()
                .map(|(item, clip)| format!("the item `{item}` is played by the clip `{clip}`"))
                .collect();
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "the bin `{bin}` holds {} item(s) that a clip plays ({}), and a clip that plays nothing is not a clip: take the clips away first",
                    played.len(),
                    list.join("; ")
                ),
            });
        }
        let name = node.name.clone();
        if !session.document_mut()?.remove_node(bin) {
            return Err(EngineError::Failed { id: id.clone(), message: format!("there is no node `{bin}`") });
        }
        Ok(Outcome::changed(format!("the bin `{name}` is gone")).with_data(json!({
            "bin": bin,
            "name": name,
            "items": items,
        })))
    }
}
