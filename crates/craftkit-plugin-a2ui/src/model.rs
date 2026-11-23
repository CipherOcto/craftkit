//! The **model** — the surface's own data model, and where it lives.
//!
//! # It lives in the **document**
//!
//! The model is **the surface's**, and the surface is a node in the
//! document — so the model is in the document too, in the surface's own
//! data, under [`MODEL_KEY`].
//!
//! That is not a convenience. It is three properties at once:
//!
//! - the model is **undone** with the document, because undo puts the
//!   document back and the model is in it;
//! - the model is **per surface** and **per session**, and two sessions
//!   never share one; and
//! - the model is **not** a process-global, so a second surface in the
//!   same program does not see the first surface's values.
//!
//! The tree used to hold one model behind a global lock, and every
//! surface in every session shared it. That was a lock held across a
//! draw and a value that outlived the document it belonged to.
//!
//! # How a node finds its model
//!
//! A node's model is the **surface's** model, and the surface is an
//! **ancestor**: a component is a child of the surface it is drawn on.
//! So the model is found by walking up, and a node that is on no
//! surface has no model — and an empty model is what it sees.

use craftkit_a2ui::model::Model;
use craftkit_sdk::Node;
use craftkit_sdk::document::Document;
use craftkit_sdk::serde_json::{Map as JsonMap, Value as Json};

/// The **kind** of the node that holds a model.
pub const SURFACE_KIND: &str = "a2ui.surface";

/// The key the model's tree is held under, in the surface's own data.
pub const MODEL_KEY: &str = "model";

/// The model of the surface a node is on.
///
/// The node's **own** surface is the one above it: a component is a
/// child of its surface, and the surface is the first ancestor that is
/// a surface.
pub fn model_of(node: &Node, document: &Document) -> Model {
    let mut current = Some(node.id);
    while let Some(id) = current {
        let Some(found) = document.node(id) else { break };
        if found.kind == SURFACE_KIND {
            return model_of_surface(found);
        }
        current = document.parent_of(id).flatten();
    }
    Model::new()
}

/// The model a **surface** node holds.
pub fn model_of_surface(surface: &Node) -> Model {
    let root = match surface.data.get(MODEL_KEY) {
        Some(value) => value.clone(),
        None => Json::Object(JsonMap::new()),
    };
    Model::from_json(root)
}

/// The **data** a fresh surface carries: a model, and nothing else.
pub fn empty_data() -> Json {
    let mut data = JsonMap::new();
    data.insert(MODEL_KEY.to_string(), Json::Object(JsonMap::new()));
    Json::Object(data)
}

/// **Write** a value into a surface's model.
///
/// The model is written **in place**, in the surface's own data, and the
/// document is what holds it — so an undo takes the value back with the
/// rest of the document.
pub fn write(surface: &mut Node, path: &str, value: Json) -> Result<(), String> {
    let mut model = model_of_surface(surface);
    model.set(path, value)?;
    let mut data = match surface.data.clone() {
        Json::Object(map) => map,
        _ => JsonMap::new(),
    };
    data.insert(MODEL_KEY.to_string(), model.root().clone());
    surface.data = Json::Object(data);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use craftkit_sdk::serde_json::json;

    /// A surface node with an empty model.
    fn surface() -> Node {
        Node {
            id: 1,
            name: "s1".to_string(),
            kind: SURFACE_KIND.to_string(),
            rect: None,
            muted: false,
            hidden: false,
            locked: false,
            appearance: None,
            appearance_from: None,
            data: empty_data(),
            children: Vec::new(),
            unknown: Json::Null,
            stream: None,
        }
    }

    /// A value is written into the surface's own data.
    #[test]
    fn a_value_is_written_into_the_surfaces_data() {
        let mut node = surface();
        write(&mut node, "/title", json!("hello")).expect("the value is written");
        let model = model_of_surface(&node);
        assert_eq!(model.get_text("/title"), Some("hello".to_string()));
    }

    /// A path that is not a path is **refused**.
    #[test]
    fn a_path_that_is_not_a_path_is_refused() {
        let mut node = surface();
        let error = write(&mut node, "title", json!("hello")).expect_err("the path is refused");
        assert!(error.contains("not a path"), "{error}");
    }

    /// A surface with no data at all has an **empty** model.
    #[test]
    fn a_surface_with_no_data_has_an_empty_model() {
        let mut node = surface();
        node.data = Json::Null;
        assert!(model_of_surface(&node).get("/title").is_none());
    }
}
