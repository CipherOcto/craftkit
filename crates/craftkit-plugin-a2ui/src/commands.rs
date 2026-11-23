//! The **five commands** — a surface is a command, and nothing else.
//!
//! # What a surface is
//!
//! A **surface** is a node in the document that a component tree is
//! drawn into. Making one, drawing onto it, taking it away, writing a
//! value into it and running one of its actions are **five commands** —
//! and the tree's rule is that every change to the document is a
//! command, so an a2ui agent has the same undo, the same refusals and
//! the same journal as a person.
//!
//! # The layout is **derived**, never stored
//!
//! The rectangles of a component tree are worked out **when the tree is
//! applied**, from the tree's own weights and the layout's rules. They
//! are the children's **places**, and a place is set by the command that
//! creates the node — so nothing about the arrangement is stored twice.
//!
//! # A value is **mirrored**, not written
//!
//! `a2ui.data.set` writes the model, and it **mirrors** the value into
//! craftkit **by issuing a command** — never by writing the document
//! behind the door's back.

use craftkit_a2ui::layout::{Align, Child, Direction, Justify, arrange};
use craftkit_sdk::serde_json::Value as Json;
use craftkit_sdk::{Node, Outcome, Rect, Session, Text};

/// The **kind** of a surface: a node the tree draws into.
pub const SURFACE_KIND: &str = "a2ui.surface";

/// The **kind** of a mirror: the node a value is mirrored into.
pub const MIRROR_KIND: &str = "a2ui.data";

/// The command's own id, in an error.
const COMMAND: &str = "a2ui";

/// The **width** of a surface, when nothing says.
const DEFAULT_WIDTH: f64 = 200.0;

/// The **height** of a surface, when nothing says.
const DEFAULT_HEIGHT: f64 = 24.0;

/// An **error** the command failed on purpose.
fn failed(message: String) -> craftkit_sdk::EngineError {
    craftkit_sdk::EngineError::Failed { id: COMMAND.to_string(), message }
}

craftkit_derive::commands! {
    /// Make a surface, and the node it draws into.
    a2ui.surface.create {
        label: "Create an a2ui surface",
        area: "a2ui",
        mutates,
        undoable,
        params: {
            /// The surface's name.
            surface_id: Text,
        },
        run: {
            let id = session.document_mut()?.add_node(
                self.surface_id.0.clone(),
                SURFACE_KIND,
                Some(Rect::new(0.0, 0.0, DEFAULT_WIDTH, DEFAULT_HEIGHT)),
            );
            Ok(Outcome::changed(format!("the surface {} is {}", self.surface_id.0, id)))
        }
    }

    /// Draw a component tree onto a surface.
    a2ui.surface.apply {
        label: "Apply an a2ui component tree",
        area: "a2ui",
        mutates,
        undoable,
        params: {
            /// The surface the tree is drawn onto.
            surface_id: Text,
            /// The component tree.
            components: craftkit_sdk::serde_json::Value,
        },
        run: {
            let surface = find_surface(session, &self.surface_id.0)?;
            let children = components_of(&self.components);
            let placed = place(session, &surface, &children)?;
            Ok(Outcome::changed(format!(
                "the surface {} holds {} component(s)",
                self.surface_id.0,
                placed
            )))
        }
    }

    /// Remove a surface's nodes.
    a2ui.surface.delete {
        label: "Delete an a2ui surface",
        area: "a2ui",
        mutates,
        undoable,
        params: {
            /// The surface to remove.
            surface_id: Text,
        },
        run: {
            let surface = find_surface(session, &self.surface_id.0)?;
            let mut removed = 0;
            for child in surface.children.clone() {
                if session.document_mut()?.remove_node(child.id) {
                    removed += 1;
                }
            }
            Ok(Outcome::changed(format!(
                "the surface {} lost {} node(s)",
                self.surface_id.0,
                removed
            )))
        }
    }

    /// Write a value into the data model, and mirror it into craftkit.
    a2ui.data.set {
        label: "Set an a2ui data value",
        area: "a2ui",
        mutates,
        undoable,
        params: {
            /// The surface the value belongs to.
            surface_id: Text,
            /// Where the value goes. The empty path is the whole model.
            path: Text,
            /// The value.
            value: craftkit_sdk::serde_json::Value,
        },
        run: {
            let path = self.path.0.clone();
            let written = crate::model::with_model(|model| model.set(&path, self.value.clone()));
            if let Err(error) = written {
                return Err(failed(error));
            }
            // The **mirror**: the value is in craftkit as a node of its
            // own, and it is put there by a **command**.
            let mirrored = mirror(session, &self.surface_id.0, &path, &self.value)?;
            Ok(Outcome::changed(format!(
                "the value at `{path}` is mirrored in {mirrored}"
            )))
        }
    }

    /// Run a component's action.
    a2ui.component.invoke {
        label: "Invoke an a2ui component",
        area: "a2ui",
        mutates,
        undoable,
        params: {
            /// The surface the component is on.
            surface_id: Text,
            /// The component's name.
            component_id: Text,
            /// The action: an event, or a call.
            action: craftkit_sdk::serde_json::Value,
        },
        run: {
            // The **surface** has to be there: an action on a surface that
            // is not there is an action on nothing.
            let _ = find_surface(session, &self.surface_id.0)?;
            let name = action_name(&self.action)?;
            // **Local first**: the layer's own table, and the agent only
            // as a fallback. And a name in neither table is **refused**.
            if let Some(command) = craftkit_a2ui::functions::command_of(&name) {
                let args = action_args(&self.action);
                session.execute(command, &args)?;
                return Ok(Outcome::changed(format!(
                    "the action `{name}` of the component {} ran",
                    self.component_id.0
                )));
            }
            if craftkit_a2ui::functions::by_name(&name).is_some() {
                // An **agent-side** function: the layer does not
                // implement it, and the door forwards it. The action
                // travels in the outcome's **data**, so the door does not
                // have to read it back out of a message.
                let mut action = craftkit_sdk::serde_json::Map::new();
                action.insert("name".to_string(), craftkit_sdk::serde_json::json!(name));
                if let Some(user_message) = self.action.get("userMessage") {
                    action.insert("userMessage".to_string(), user_message.clone());
                }
                action.insert("surfaceId".to_string(), craftkit_sdk::serde_json::json!(self.surface_id.0));
                action.insert("sourceComponentId".to_string(), craftkit_sdk::serde_json::json!(self.component_id.0));
                action.insert(
                    "context".to_string(),
                    self.action.get("context").cloned().unwrap_or_else(|| craftkit_sdk::serde_json::json!({})),
                );
                return Ok(Outcome::new(format!(
                    "the action `{name}` of the component {} is the agent's",
                    self.component_id.0
                ))
                .with_data(craftkit_sdk::serde_json::json!({ "action": action })));
            }
            Err(failed(format!(
                "the action `{name}` is in neither the layer's table nor the agent-side table"
            )))
        }
    }
}

/// The surface with a name, if the document has one.
fn find_surface(session: &mut Session, name: &str) -> Result<Node, craftkit_sdk::EngineError> {
    let document = session.document();
    let found = document.nodes().find(|node| node.kind == SURFACE_KIND && node.name == name).cloned();
    match found {
        Some(node) => Ok(node),
        None => Err(failed(format!("there is no surface called `{name}`"))),
    }
}

/// The **children** of a component tree.
///
/// The tree is a2ui's shape: a component, or a list of components.
fn components_of(tree: &Json) -> Vec<Json> {
    match tree {
        Json::Array(items) => items.clone(),
        Json::Object(_) => vec![tree.clone()],
        _ => Vec::new(),
    }
}

/// The **kind** of a component, if the tree names one.
fn kind_of(component: &Json) -> Option<String> {
    component.get("component").and_then(Json::as_str).map(|name| name.to_string())
}

/// The **weight** of a component, if it says one.
fn weight_of(component: &Json) -> Option<f64> {
    let weight = component.get("weight")?;
    match weight {
        Json::Number(number) => number.as_f64(),
        Json::String(text) => text.parse().ok(),
        _ => None,
    }
}

/// The **place** of every component in the tree.
///
/// The arrangement is worked out **here**, from the tree's own weights,
/// and it is **not stored**: the rectangles are the children's places,
/// and a place is set by the command that creates the node.
fn place(
    session: &mut Session,
    surface: &Node,
    children: &[Json],
) -> Result<usize, craftkit_sdk::EngineError> {
    let box_ = surface.rect.unwrap_or_default();
    let mut laid: Vec<Child> = Vec::with_capacity(children.len());
    let mut kinds: Vec<Option<String>> = Vec::with_capacity(children.len());
    for component in children {
        let kind = kind_of(component);
        if let Some(name) = &kind
            && !is_known(name)
        {
            // A name the layer **refuses** says why it is refused, and a
            // name it simply does not draw is not a component at all.
            return Err(failed(match craftkit_a2ui::defs::refusal_of(name) {
                Some(reason) => format!("the component `{name}` is refused: {reason}"),
                None => format!("the component `{name}` is not one this layer draws"),
            }));
        }
        laid.push(Child { weight: weight_of(component), size: DEFAULT_WIDTH, cross: DEFAULT_HEIGHT });
        kinds.push(kind);
    }
    let arrangement = arrange(Direction::Horizontal, Justify::Start, Align::Stretch, box_.w, box_.h, &laid)
        .map_err(failed)?;

    let mut count = 0;
    for (index, component) in children.iter().enumerate() {
        let Some(name) = kinds[index].clone() else { continue };
        // The node's **kind** is craftkit's: `a2ui.text`. The a2ui name
        // is `Text`, and a kind is dotted and lower case.
        let kind = format!("a2ui.{}", name.to_lowercase());
        let rect = arrangement.rects[index];
        let data = component.clone();
        // The component is a **child of the surface**, so that the
        // surface's nodes are the ones it holds — and so that deleting
        // the surface deletes them.
        session.document_mut()?.add_node_with_under(Some(surface.id), name, kind, Some(rect), data);
        count += 1;
    }
    Ok(count)
}

/// Whether the layer draws a component.
fn is_known(kind: &str) -> bool {
    matches!(
        kind,
        "Text"
            | "Image"
            | "Icon"
            | "Row"
            | "Column"
            | "List"
            | "Card"
            | "Divider"
            | "Button"
            | "CheckBox"
            | "TextField"
    )
}

/// The **mirror** of a value: a node of its own, holding the value.
fn mirror(
    session: &mut Session,
    surface_id: &str,
    path: &str,
    value: &Json,
) -> Result<u64, craftkit_sdk::EngineError> {
    let name = if path.is_empty() { surface_id.to_string() } else { path.to_string() };
    let data = craftkit_sdk::serde_json::json!({ "path": path, "value": value });
    let id = session.document_mut()?.add_node_with_under(None, name, MIRROR_KIND, None, data);
    Ok(id)
}

/// The **name** of an action.
///
/// An a2ui action is either an **event** — an object with a `name` — or a
/// **function call** — an object with a `@call`. Both are read here, and
/// anything else is **refused**.
fn action_name(action: &Json) -> Result<String, craftkit_sdk::EngineError> {
    if let Some(name) = action.get("name").and_then(Json::as_str) {
        return Ok(name.to_string());
    }
    if let Some(name) = action.get("@call").and_then(Json::as_str) {
        return Ok(name.to_string());
    }
    Err(failed("the action names no function and no event".to_string()))
}

/// The **arguments** of an action.
fn action_args(action: &Json) -> Json {
    action.get("args").cloned().unwrap_or(Json::Null)
}
