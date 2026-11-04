//! **Reading** a project.
//!
//! A project may be read: the answer is the **bins**, the **items**
//! in them, and the **clips** that play them.
//!
//! Everything in the answer is **derived**: the bins are the nodes of
//! the tree, the items are the nodes under them, and the clips that
//! play an item are the edges that name it. None of it is written
//! down, and so none of it can be wrong.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Outcome, Session};

/// The **project**: the bins, the items, and the clips that play them.
#[derive(Command)]
#[command(id = "film.project", area = "film", layer = "film", icon = "list")]
pub struct ReadProject {}

impl ReadProject {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let document = session.document();
        // The **bins**: the nodes that hold the items.
        let mut bins: Vec<Json> = Vec::new();
        for bin in document.walk().filter(|node| node.kind == "film.bin") {
            let items: Vec<Json> = bin
                .children
                .iter()
                .filter(|child| child.kind == "film.item")
                .map(|child| json!({ "id": child.id, "name": child.name }))
                .collect();
            bins.push(json!({ "id": bin.id, "name": bin.name, "items": items }));
        }
        // The **items**: what they are, and what plays them.
        let mut items: Vec<Json> = Vec::new();
        for item in document.walk().filter(|node| node.kind == "film.item") {
            let clips: Vec<u64> =
                document.relations_to(item.id, Some("clip")).iter().map(|relation| relation.from).collect();
            let tracks: Vec<u64> = item
                .children
                .iter()
                .filter(|child| child.kind == "film.track")
                .map(|child| child.id)
                .collect();
            let bin = document
                .walk()
                .find(|node| node.kind == "film.bin" && node.children.iter().any(|child| child.id == item.id))
                .map(|node| node.id);
            items.push(json!({
                "id": item.id,
                "name": item.name,
                "item": item.data.get("item").and_then(|item| item.as_u64()),
                "path": item.data.get("path").cloned().unwrap_or(Json::Null),
                "duration": item.data.get("duration").and_then(|duration| duration.as_u64()).unwrap_or(0),
                "offline": item.data.get("offline").and_then(|offline| offline.as_bool()).unwrap_or(false),
                "bin": bin,
                "tracks": tracks,
                "clips": clips,
            }));
        }
        let count = items.len();
        Ok(Outcome::new(format!("{count} item(s) in {} bin(s)", bins.len())).with_data(json!({
            "bins": bins,
            "items": items,
        })))
    }
}
