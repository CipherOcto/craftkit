//! The **name** of a thing in the project.
//!
//! A media item may be **named**, and a bin may be **named**. The
//! name is the thing's own: it does not follow the thing, and it is
//! not derived from it.
//!
//! The tree's `clip.rename` renames an **item** or a **clip** with one
//! command. Here they are two commands — one command per thing — and
//! the rule they share is the tree's: a name that is **nothing** is
//! refused, because a thing with no name is not a thing that can be
//! found.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Text};

/// The name a thing may be **called**, or the error that says it may
/// not.
fn a_name(name: &str, what: &str, id: &str) -> Result<String, EngineError> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(EngineError::Failed { id: id.to_string(), message: format!("{what} has a name") });
    }
    Ok(name)
}

commands! {
    /// **Name** a media item.
    film.item.rename {
        area: "film",
        layer: "film",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The item.

            item: NodeRef,
            /// What to call it.

            name: Text,
        },
        run: {

            let id = "film.item.rename".to_string();
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
            let name = a_name(&self.name.0, "an item", &id)?;
            if let Some(node) = session.document_mut()?.node_mut(item) {
                node.name = name.clone();
            }
            Ok(Outcome::changed(format!("the item `{item}` is called `{name}`"))
                .with_data(json!({ "item": item, "name": name })))

        }
    }
    /// **Name** a bin.
    film.bin.rename {
        area: "film",
        layer: "film",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The bin.

            bin: NodeRef,
            /// What to call it.

            name: Text,
        },
        run: {

            let id = "film.bin.rename".to_string();
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
            let name = a_name(&self.name.0, "a bin", &id)?;
            if let Some(node) = session.document_mut()?.node_mut(bin) {
                node.name = name.clone();
            }
            Ok(Outcome::changed(format!("the bin `{bin}` is called `{name}`"))
                .with_data(json!({ "bin": bin, "name": name })))

        }
    }
}
