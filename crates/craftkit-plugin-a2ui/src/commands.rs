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
//! # The model is **in the document**
//!
//! The surface's **data model** is held in the surface's own data, so
//! it is undone with the document and it is not shared with another
//! session or another surface. A value that is written is a value in
//! the document, and an undo takes it back.
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

use craftkit_sdk::serde_json::{Map as JsonMap, Value as Json, json};
use craftkit_sdk::{Node, Outcome, Rect, Session, Text};

/// The **kind** of a surface: a node the tree draws into.
pub const SURFACE_KIND: &str = "a2ui.surface";

/// The **kind** of a mirror: the node a value is mirrored into.
pub const MIRROR_KIND: &str = "a2ui.data";

/// The command's own id, in an error.
const COMMAND: &str = "a2ui";

/// The **width** of a surface, when nothing says.
const DEFAULT_WIDTH: f64 = 320.0;

/// The **height** of a surface, when nothing says.
const DEFAULT_HEIGHT: f64 = 200.0;

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
            // A surface's id is **unique for the renderer's lifetime**,
            // and a second surface with the same name is a second
            // surface wearing the first one's name: it is refused.
            if find_surface(session, &self.surface_id.0).is_ok() {
                return Err(failed(format!(
                    "there is already a surface called `{}`",
                    self.surface_id.0
                )));
            }
            let id = session.document_mut()?.add_node_with_under(
                None,
                self.surface_id.0.clone(),
                SURFACE_KIND,
                Some(Rect::new(0.0, 0.0, DEFAULT_WIDTH, DEFAULT_HEIGHT)),
                crate::model::empty_data(),
            );
            Ok(Outcome::changed(format!("the surface {} is {}", self.surface_id.0, id)))
        }
    }

    /// Draw a component tree onto a surface.
    ///
    /// The tree **replaces** what the surface held: a2ui says a tree may
    /// be sent again to update the surface, and an update that kept the
    /// old tree as well would draw it twice.
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
            // The tree is **checked** before anything is changed: a tree
            // that does not fit is refused, and the surface keeps what
            // it had.
            let tree = crate::tree::Tree::parse(&self.components).map_err(failed)?;
            let mut ids: Vec<(String, u64)> = Vec::new();
            // The old tree goes first: the surface holds one tree.
            clear(session, &surface)?;
            let root = tree.root();
            let root_id = build(session, Some(surface.id), root, &tree, &mut ids)?;
            let box_ = surface.rect.unwrap_or(Rect::new(0.0, 0.0, DEFAULT_WIDTH, DEFAULT_HEIGHT));
            if let Some(node) = session.document_mut()?.node_mut(root_id) {
                node.rect = Some(box_);
            }
            place(session, &tree, &ids, root, box_)?;
            Ok(Outcome::changed(format!(
                "the surface {} holds {} component(s)",
                self.surface_id.0,
                ids.len()
            )))
        }
    }

    /// Remove a surface, and everything it holds.
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
            // The surface goes **with** what it holds: the surface's
            // node, its components and its model are one thing, and
            // taking the surface away takes them all.
            let removed = session.document_mut()?.remove_node(surface.id);
            Ok(Outcome::changed(format!(
                "the surface {} is {}",
                self.surface_id.0,
                if removed { "gone" } else { "not there" }
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
            let surface = find_surface(session, &self.surface_id.0)?;
            // The model is the **surface's**, and it is in the document:
            // so an undo takes the value back with the rest.
            {
                let node = session
                    .document_mut()?
                    .node_mut(surface.id)
                    .ok_or_else(|| failed(format!("there is no surface called `{}`", self.surface_id.0)))?;
                crate::model::write(node, &self.path.0, self.value.clone()).map_err(failed)?;
            }
            // The **mirror**: the value is in craftkit as a node of its
            // own, and it is put there by a **command**.
            let mirrored = mirror(session, surface.id, &self.path.0, &self.value)?;
            Ok(Outcome::changed(format!(
                "the value at `{}` is mirrored in {mirrored}",
                self.path.0
            )))
        }
    }

    /// Run a component's action.
    ///
    /// The action is a2ui's own shape: an **event** goes back to the
    /// agent, and a **function call** runs here — the layer's own
    /// functions, or the agent's.
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
            /// The action: an event, or a function call.
            action: craftkit_sdk::serde_json::Value,
        },
        run: {
            // The **surface** has to be there: an action on a surface that
            // is not there is an action on nothing.
            let surface = find_surface(session, &self.surface_id.0)?;
            // And so does the **component**: an action of a component
            // that is not on the surface is an action of nothing.
            if !has_component(session, &surface, &self.component_id.0) {
                return Err(failed(format!(
                    "there is no component called `{}` on the surface `{}`",
                    self.component_id.0, self.surface_id.0
                )));
            }
            let model = crate::model::model_of_surface(&surface);
            match action_of(&self.action)? {
                Action::Event { name, user_message, context } => {
                    // An **event** is the agent's: the door puts it on
                    // the wire. And the **bindings** in it are
                    // resolved first — the protocol says the context
                    // that goes to the agent is the resolved one.
                    let user_message = match user_message {
                        Some(value) => Some(
                            crate::props::text_of(&model, &value).ok_or_else(|| {
                                failed("the user message of the action names a path that is not there".to_string())
                            })?,
                        ),
                        None => None,
                    };
                    let context = resolve_context(&model, &context)?;
                    let mut event = JsonMap::new();
                    event.insert("name".to_string(), json!(name));
                    if let Some(message) = user_message {
                        event.insert("userMessage".to_string(), json!(message));
                    }
                    event.insert("context".to_string(), context);
                    event.insert("surfaceId".to_string(), json!(self.surface_id.0));
                    event.insert("sourceComponentId".to_string(), json!(self.component_id.0));
                    return Ok(Outcome::new(format!(
                        "the action of the component {} is the agent's",
                        self.component_id.0
                    ))
                    .with_data(json!({ "action": Json::Object(event) })));
                }
                Action::Call { name, args } => {
                    if let Some(command) = craftkit_a2ui::functions::command_of(&name) {
                        // The **local** function: it is a craftkit
                        // command, and the arguments are translated
                        // from a2ui's names to craftkit's.
                        let translated = translate(&name, &args)?;
                        // And it is **one undo step**: the action and
                        // the command it runs are one thing to undo.
                        session.batch(|session| session.execute(command, &translated))?;
                        return Ok(Outcome::changed(format!(
                            "the action `{name}` of the component {} ran",
                            self.component_id.0
                        )));
                    }
                    if craftkit_a2ui::functions::by_name(&name).is_some() {
                        // An **agent-side** function: the layer does not
                        // implement it, and the door asks the agent to.
                        // The **call**, in a2ui's own shape: the name and
                        // the arguments, as the protocol carries them.
                        let mut function = JsonMap::new();
                        function.insert("@call".to_string(), json!(name));
                        function.insert("args".to_string(), args);
                        let mut call = JsonMap::new();
                        call.insert("surfaceId".to_string(), json!(self.surface_id.0));
                        call.insert("callFunction".to_string(), Json::Object(function));
                        return Ok(Outcome::new(format!(
                            "the function `{name}` of the component {} is the agent's",
                            self.component_id.0
                        ))
                        .with_data(json!({ "callAgentFunction": Json::Object(call) })));
                    }
                    Err(failed(format!(
                        "the function `{name}` is in neither the layer's table nor the agent-side table"
                    )))
                }
            }
        }
    }
}

/// An **action**: an event, or a function call.
///
/// a2ui's own shape: `{"event": {...}}` or `{"functionCall": {...}}`.
enum Action {
    /// An **event**: the agent's.
    Event {
        /// The name of the event.
        name: String,
        /// What the user did, in a line.
        user_message: Option<Json>,
        /// The context that goes with it.
        context: Json,
    },
    /// A **function call**: the layer's, or the agent's.
    Call {
        /// The name of the function.
        name: String,
        /// The arguments, as they were sent.
        args: Json,
    },
}

/// Read an **action**, in a2ui's own shape.
fn action_of(action: &Json) -> Result<Action, craftkit_sdk::EngineError> {
    if let Some(event) = action.get("event") {
        let name = event
            .get("name")
            .and_then(Json::as_str)
            .filter(|name| !name.is_empty())
            .ok_or_else(|| failed("the event names nothing".to_string()))?
            .to_string();
        let user_message = event.get("userMessage").cloned();
        let context = event.get("context").cloned().unwrap_or(Json::Object(JsonMap::new()));
        return Ok(Action::Event { name, user_message, context });
    }
    if let Some(call) = action.get("functionCall") {
        let name = call
            .get("@call")
            .and_then(Json::as_str)
            .filter(|name| !name.is_empty())
            .ok_or_else(|| failed("the function call names no function".to_string()))?
            .to_string();
        let args = call.get("args").cloned().unwrap_or(Json::Object(JsonMap::new()));
        return Ok(Action::Call { name, args });
    }
    Err(failed("the action is neither an event nor a function call".to_string()))
}

/// The **context** of an event, with its **bindings** resolved.
///
/// The protocol says the context that goes to the agent is the
/// resolved one. A binding that names a path that is not there is
/// **refused**: an event with a hole in it is not an event.
fn resolve_context(
    model: &craftkit_a2ui::model::Model,
    context: &Json,
) -> Result<Json, craftkit_sdk::EngineError> {
    let Some(object) = context.as_object() else {
        return Err(failed("the context of an action is not an object".to_string()));
    };
    let mut out = JsonMap::new();
    for (key, value) in object {
        let resolved = if craftkit_a2ui::bindings::Dynamic::is_binding(value) {
            match craftkit_a2ui::bindings::resolve(model, value) {
                Ok(craftkit_a2ui::bindings::Resolved::Value(value)) => value,
                Ok(craftkit_a2ui::bindings::Resolved::Call { name, .. }) => {
                    return Err(failed(format!("the context of the action calls `{name}`")));
                }
                Err(error) => return Err(failed(error)),
            }
        } else {
            value.clone()
        };
        out.insert(key.clone(), resolved);
    }
    Ok(Json::Object(out))
}

/// The **arguments** of a call, in craftkit's own names.
///
/// a2ui says `surfaceId` and craftkit says `surface_id`, and the
/// layer's table says which is which — so the names are **translated**,
/// not guessed. An argument the table does not name is **refused**.
fn translate(name: &str, args: &Json) -> Result<Json, craftkit_sdk::EngineError> {
    let Some(pairs) = craftkit_a2ui::functions::arguments_of(name) else {
        return Err(failed(format!("the function `{name}` takes no arguments")));
    };
    let Some(object) = args.as_object() else {
        return Err(failed(format!("the arguments of `{name}` are not an object")));
    };
    let mut out = JsonMap::new();
    for (key, value) in object {
        let Some((_, craftkit_name)) = pairs.iter().find(|(a2ui, _)| a2ui == key) else {
            return Err(failed(format!("`{key}` is not an argument of `{name}`")));
        };
        out.insert((*craftkit_name).to_string(), value.clone());
    }
    Ok(Json::Object(out))
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

/// Whether the surface holds a **component** with a name.
fn has_component(session: &mut Session, surface: &Node, id: &str) -> bool {
    let document = session.document();
    document
        .walk()
        .any(|node| node.kind != SURFACE_KIND && node.name == id && under(document, node.id, surface.id))
}

/// Whether a node is **under** another one.
fn under(document: &craftkit_sdk::Document, id: u64, parent: u64) -> bool {
    let mut current = document.parent_of(id).flatten();
    while let Some(id) = current {
        if id == parent {
            return true;
        }
        current = document.parent_of(id).flatten();
    }
    false
}

/// Take the surface's **components** away, and leave the rest.
///
/// The surface's **mirrors** stay: a value is not part of the tree, and
/// an update of the tree does not unwrite a value.
fn clear(session: &mut Session, surface: &Node) -> Result<usize, craftkit_sdk::EngineError> {
    let children: Vec<u64> = surface
        .children
        .iter()
        .filter(|child| crate::tree::is_component_kind(&child.kind))
        .map(|child| child.id)
        .collect();
    let mut removed = 0;
    for id in children {
        if session.document_mut()?.remove_node(id) {
            removed += 1;
        }
    }
    Ok(removed)
}

/// Make the **nodes** of a tree: a node for each component, and the
/// children under their parent.
fn build(
    session: &mut Session,
    parent: Option<u64>,
    component: &crate::tree::Component,
    tree: &crate::tree::Tree,
    ids: &mut Vec<(String, u64)>,
) -> Result<u64, craftkit_sdk::EngineError> {
    let kind = crate::tree::component_kind(&component.kind);
    let id = session.document_mut()?.add_node_with_under(
        parent,
        component.id.clone(),
        kind,
        None,
        component.json.clone(),
    );
    ids.push((component.id.clone(), id));
    for child in tree.children_of(component).map_err(failed)? {
        build(session, Some(id), child, tree, ids)?;
    }
    Ok(id)
}

/// **Place** the children of a component, inside the component's own
/// place.
///
/// The places are worked out by the layer's layout, from the tree's
/// own weights — and they are **derived**, never stored: a place is
/// set on the node, and it is worked out again the next time the tree
/// is applied.
fn place(
    session: &mut Session,
    tree: &crate::tree::Tree,
    ids: &[(String, u64)],
    component: &crate::tree::Component,
    box_: Rect,
) -> Result<(), craftkit_sdk::EngineError> {
    let children = tree.children_of(component).map_err(failed)?;
    if children.is_empty() {
        return Ok(());
    }
    let rects = crate::tree::places(tree, component, box_, &children).map_err(failed)?;
    for (child, rect) in children.iter().zip(rects.iter()) {
        let Some((_, id)) = ids.iter().find(|(name, _)| name == &child.id) else {
            return Err(failed(format!("the component `{}` was not made", child.id)));
        };
        if let Some(node) = session.document_mut()?.node_mut(*id) {
            node.rect = Some(*rect);
        }
        place(session, tree, ids, child, *rect)?;
    }
    Ok(())
}

/// The **mirror** of a value: a node of its own, holding the value.
///
/// One node per path: the mirror is the value **now**, and not a
/// history of the values that were.
fn mirror(
    session: &mut Session,
    surface_id: u64,
    path: &str,
    value: &Json,
) -> Result<u64, craftkit_sdk::EngineError> {
    let name = if path.is_empty() { surface_id.to_string() } else { path.to_string() };
    let data = json!({ "path": path, "value": value });
    let existing = session
        .document()
        .walk()
        .find(|node| node.kind == MIRROR_KIND && node.name == name)
        .map(|node| node.id);
    match existing {
        Some(id) => {
            if let Some(node) = session.document_mut()?.node_mut(id) {
                node.data = data;
            }
            Ok(id)
        }
        None => {
            Ok(session.document_mut()?.add_node_with_under(Some(surface_id), name, MIRROR_KIND, None, data))
        }
    }
}
