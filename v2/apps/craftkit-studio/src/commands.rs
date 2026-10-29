//! The application's own commands.
//!
//! Nothing here is special. The commands in this module are declared exactly
//! as a plugin declares them — a struct, an attribute, a doc comment and a
//! `run` method — because the macro does not care which crate it is in.

use craftkit_derive::Command;
use craftkit_sdk::{Bool, CommandSpec, Doc, Either, EngineError, NodeRef, Outcome, ParamDoc, Session, Text};

/// Give the document a name.
///
/// A required parameter, read from a field, and the body never sees the
/// caller's values.
#[derive(Command)]
#[command(id = "doc.rename", area = "doc", icon = "pencil", mutates, undoable)]
pub struct DocRename {
    /// The new name.
    pub name: Text,
}

impl DocRename {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let name = self.name.0.clone();
        let changed = session.document().name != name;
        session.document_mut().name = name.clone();
        Ok(Outcome::new(format!("renamed to `{name}`"))
            .with_data(craftkit_sdk::serde_json::json!({ "name": name }))
            .changed_if(changed))
    }
}

/// Add a node to the document.
#[derive(Command)]
#[command(id = "node.add", area = "node", icon = "plus", mutates, undoable)]
pub struct NodeAdd {
    /// The node's name.
    pub name: Text,
}

impl NodeAdd {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let name = self.name.0.clone();
        let id = session.document_mut().add_node(name.clone());
        Ok(Outcome::changed(format!("added `{name}`"))
            .with_data(craftkit_sdk::serde_json::json!({ "id": id, "name": name })))
    }
}

/// Mute or unmute a node.
///
/// The node may be given as an **id** or as a **name**: one parameter, two
/// spellings, and the type says so. The field is an `Either`, the
/// declaration says the pair is required and neither half is, and the body
/// is handed whichever one arrived.
#[derive(Command)]
#[command(
    id = "node.mute",
    area = "node",
    icon = "mute",
    mutates,
    undoable,
    enabled = has_nodes
)]
pub struct NodeMute {
    /// The node's id, or its name.
    #[param(or = "node_name")]
    pub node: Either<NodeRef, Text>,
    /// True to mute, false to unmute. The opposite of the node's own state
    /// when it is not given.
    pub muted: Option<Bool>,
}

impl NodeMute {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = match &self.node {
            Either::Left(NodeRef(id)) => *id,
            Either::Right(Text(name)) => match session.document().node_by_name(name) {
                Some(node) => node.id,
                None => {
                    return Err(EngineError::Failed {
                        id: "node.mute".to_string(),
                        message: format!("there is no node called `{name}`"),
                    });
                }
            },
        };
        let muted = match self.muted {
            Some(Bool(muted)) => muted,
            None => match session.document().node(id) {
                Some(node) => !node.muted,
                None => {
                    return Err(EngineError::Failed {
                        id: "node.mute".to_string(),
                        message: format!("there is no node with id {id}"),
                    });
                }
            },
        };
        let Some(node) = session.document_mut().node_mut(id) else {
            return Err(EngineError::Failed {
                id: "node.mute".to_string(),
                message: format!("there is no node with id {id}"),
            });
        };
        node.muted = muted;
        let message = if muted { "muted" } else { "unmuted" };
        Ok(Outcome::changed(message).with_data(craftkit_sdk::serde_json::json!({ "id": id })))
    }
}

/// Whether there is anything to mute.
fn has_nodes(session: &Session) -> bool {
    !session.document().nodes.is_empty()
}

/// The parameters of the command made at run time.
static ECHO_PARAMS: &[ParamDoc] = &[ParamDoc::required("text", Doc::Text, "What to say.")];

/// A command that exists only while the program runs.
///
/// There is no macro here: a command made at run time has no struct to
/// derive from, so it is written out. It reads its own parameter, because
/// nothing reads it for it — which is the price of a command that is not
/// there at compile time.
pub fn runtime_echo() -> CommandSpec {
    CommandSpec {
        id: "runtime.echo",
        label: "Echo",
        area: "runtime",
        icon: "echo",
        doc: "Say the text back.",
        shortcut: None,
        params: ECHO_PARAMS,
        mutates: false,
        undoable: false,
        enabled: craftkit_sdk::always,
        run: |_session, params| {
            let text = params.get("text").and_then(|value| value.as_str()).unwrap_or_default();
            match text {
                "" => Err(EngineError::Failed {
                    id: "runtime.echo".to_string(),
                    message: "there was no text".to_string(),
                }),
                text => Ok(Outcome::new(text.to_string())),
            }
        },
    }
}
