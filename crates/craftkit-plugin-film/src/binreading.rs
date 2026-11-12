//! **Reading** a bin.
//!
//! A bin may be read whole: the answer is everything the bin holds and
//! everything that is said about it.
//!
//! The facts of a bin are its **name**, the **items** in it and the
//! **bins** in it. A bin in a bin is a bin like any other, and the
//! tree's bin holds both kinds of child in one list.
//!
//! One fact of the answer is **derived**: the **parent** of a bin is
//! the bin that holds it, and the tree's bin has no parent either —
//! it is found by walking.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Outcome};

commands! {
    /// Read a **bin**.
    film.bin.get {
        area: "film",
        layer: "film",
        icon: "list",
        params: {
            /// The bin to read.

            bin: NodeRef,
        },
        run: {

            let id = "film.bin.get".to_string();
            let bin = self.bin.0;
            let Some(node) = session.document().node(bin) else {
                return Err(EngineError::Failed { id, message: format!("there is no bin `{bin}`") });
            };
            if node.kind != "film.bin" {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the node `{bin}` is a `{}` and not a bin", node.kind),
                });
            }
            // The **items** and the **bins** the bin holds.
            let items: Vec<u64> =
                node.children.iter().filter(|child| child.kind == "film.item").map(|child| child.id).collect();
            let bins: Vec<u64> =
                node.children.iter().filter(|child| child.kind == "film.bin").map(|child| child.id).collect();
            // The **parent** of the bin: the bin that holds it.
            let parent = session
                .document()
                .walk()
                .find(|node| node.children.iter().any(|child| child.id == bin))
                .map(|node| node.id);
            Ok(Outcome::new(format!("the bin `{bin}`")).with_data(json!({
                "id": bin,
                "name": node.name,
                "items": items,
                "bins": bins,
                "parent": parent,
            })))

        }
    }
}
