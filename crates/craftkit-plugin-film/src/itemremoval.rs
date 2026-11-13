//! **Taking away** a media item.
//!
//! A media item may be taken away — the tree's *clear*, the tree's
//! `project.delete`.
//!
//! And here the item may be taken away only when **no clip plays it**:
//! a clip that plays nothing is not a clip. The tree removes such an
//! item and leaves the clip behind, and the tree has an *offline* flag
//! for a media file that is not there; here the removal is refused and
//! the answer says which clips are in the way, so that they may be
//! taken away first.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Outcome};

commands! {
    /// **Take away** a media item.
    ///
    /// The item is gone when no clip plays it.
    film.item.remove {
        area: "film",
        layer: "film",
        icon: "minus",
        mutates,
        undoable,
        params: {
            /// The item.
            item: NodeRef,
        },
        run: {
            let id = "film.item.remove".to_string();
            let item = self.item.0;
            let Some(node) = session.document().node(item) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{item}`"),
                });
            };
            if node.kind != "film.item" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{item}` is a `{}` and not an item", node.kind),
                });
            }
            // The **clips** that play the item: the edges that name it.
            let clips: Vec<u64> = session
                .document()
                .relations_to(item, Some("clip"))
                .iter()
                .map(|relation| relation.from)
                .collect();
            if !clips.is_empty() {
                let list: Vec<String> = clips.iter().map(|clip| format!("`{clip}`")).collect();
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the item `{item}` is played by {} clip(s) ({}), and a clip that plays nothing is not a clip: take the clips away first",
                        clips.len(),
                        list.join(", ")
                    ),
                });
            }
            let name = node.name.clone();
            if !session.document_mut()?.remove_node(item) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{item}`"),
                });
            }
            Ok(Outcome::changed(format!("the item `{name}` is gone")).with_data(json!({
                "item": item,
                "name": name,
            })))

        }
    }
}
