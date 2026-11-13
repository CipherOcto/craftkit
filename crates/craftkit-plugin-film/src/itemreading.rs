//! **Reading** a media item.
//!
//! A media item may be read whole: the answer is everything the item
//! holds and everything that is said about it.
//!
//! The facts of an item are its **name**, the **id** it has in the
//! editor, the **path** of the media, how long it is in **frames**,
//! whether it was made **offline** on purpose, and the **label** it
//! carries.
//!
//! Two of the facts of the answer are **derived** and are not written
//! down: the **bin** the item is in is its place in the tree, and the
//! **clips** that play it are the edges that name it.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Outcome};

commands! {
    /// Read a **media item**.
    film.item.get {
        area: "film",
        layer: "film",
        icon: "list",
        params: {
            /// The item to read.
            item: NodeRef,
        },
        run: {
            let id = "film.item.get".to_string();
            let item = self.item.0;
            let Some(node) = session.document().node(item) else {
                return Err(EngineError::Failed { id, message: format!("there is no item `{item}`") });
            };
            if node.kind != "film.item" {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the node `{item}` is a `{}` and not an item", node.kind),
                });
            }
            let document = session.document();
            // The **bin** the item is in: its place in the tree.
            let bin = document
                .walk()
                .find(|node| node.children.iter().any(|child| child.id == item))
                .map(|node| node.id);
            // The **clips** that play the item: the edges that name it.
            let clips: Vec<u64> =
                document.relations_to(item, Some("clip")).iter().map(|relation| relation.from).collect();
            let path = node.data.get("path").cloned().unwrap_or(Json::Null);
            let label = node.data.get("label").cloned().unwrap_or(Json::Null);
            let item_id = node.data.get("item").cloned().unwrap_or(Json::Null);
            let duration = node.data.get("duration").cloned().unwrap_or(Json::Null);
            let offline = node.data.get("offline").cloned().unwrap_or(Json::Null);
            // The **generator**: what the item plays when it is not a file.
            let generator = node.data.get("generator").cloned().unwrap_or(Json::Null);
            Ok(Outcome::new(format!("the item `{item}`")).with_data(json!({
                "id": item,
                "name": node.name,
                "item": item_id,
                "path": path,
                "duration": duration,
                "offline": offline,
                "generator": generator,
                "label": label,
                "bin": bin,
                "clips": clips,
            })))

        }
    }
}
