//! The application's own commands.
//!
//! Nothing here is special. The commands in this module are declared exactly
//! as a plugin declares them — a struct, an attribute, a doc comment and a
//! `run` method — because the macro does not care which crate it is in.

use craftkit_derive::{Command, Panel};
use craftkit_sdk::{
    Bool, CommandRun, CommandSpec, Doc, Either, EngineError, NodeRef, Outcome, ParamDoc, Rect, Session, Text,
};

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
///
/// The node's **kind** is a parameter, and the command refuses a kind that
/// is not linked in: the kinds are a registry, and the document may only
/// hold kinds the program knows how to draw.
#[derive(Command)]
#[command(id = "node.add", area = "node", icon = "plus", mutates, undoable)]
pub struct NodeAdd {
    /// The node's name.
    pub name: Text,
    /// The kind of thing it is.
    pub kind: Text,
    /// The node to put it in, if it goes in one.
    pub parent: Option<NodeRef>,
    /// Where the node is, if it is somewhere.
    ///
    /// A rectangle: `{"x": 0, "y": 0, "w": 10, "h": 10}`. A node in a
    /// document that has no space — a clip in a sound document — has none,
    /// and the command does not invent one.
    pub rect: Option<craftkit_sdk::serde_json::Value>,
    /// The kind's own data.
    ///
    /// The payload: what the kind means, in the kind's own shape. The
    /// command does not read it.
    pub data: Option<craftkit_sdk::serde_json::Value>,
}

impl NodeAdd {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let kind = self.kind.0.clone();
        if session.kinds().get(&kind).is_none() {
            return Err(EngineError::Failed {
                id: "node.add".to_string(),
                message: format!("there is no kind called `{kind}`"),
            });
        }
        let name = self.name.0.clone();
        let parent = self.parent.map(|parent| parent.0);
        if let Some(parent) = parent
            && session.document().node(parent).is_none()
        {
            return Err(EngineError::Failed {
                id: "node.add".to_string(),
                message: format!("there is no node `{parent}`"),
            });
        }
        let rect: Option<Rect> = match &self.rect {
            Some(value) => match craftkit_sdk::FormatValue::from_json(value) {
                Ok(rect) => Some(rect),
                Err(message) => {
                    return Err(EngineError::Failed {
                        id: "node.add".to_string(),
                        message: format!("the node's place is not a rectangle: {message}"),
                    });
                }
            },
            None => None,
        };
        let data = self.data.clone().unwrap_or(craftkit_sdk::serde_json::Value::Null);
        let id = session.document_mut().add_node_with_under(parent, name.clone(), kind.clone(), rect, data);
        Ok(Outcome::changed(format!("added a {kind} `{name}`"))
            .with_data(craftkit_sdk::serde_json::json!({ "id": id, "kind": kind })))
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
        capability: "command",
        layer: "core",
        icon: "echo",
        doc: "Say the text back.",
        shortcut: None,
        params: ECHO_PARAMS,
        mutates: false,
        undoable: false,
        enabled: craftkit_sdk::always,
        run: CommandRun::Dynamic(std::sync::Arc::new(|_session, params| {
            let text = params.get("text").and_then(|value| value.as_str()).unwrap_or_default();
            match text {
                "" => Err(EngineError::Failed {
                    id: "runtime.echo".to_string(),
                    message: "there was no text".to_string(),
                }),
                text => Ok(Outcome::new(text.to_string())),
            }
        })),
    }
}

/// Put the selected nodes in a group.
///
/// The **selection** is the session's state, and this is what a person
/// does with it: pick two things, and put them in a box. The group is a
/// node, and the nodes go into it.
#[derive(Command)]
#[command(id = "node.group", area = "node", icon = "group", mutates, undoable)]
pub struct NodeGroup {}

impl NodeGroup {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let selected = session.selection().to_vec();
        if selected.len() < 2 {
            return Err(EngineError::Failed {
                id: "node.group".to_string(),
                message: "select two nodes or more, and then group them".to_string(),
            });
        }
        // The group's box is the box of what is in it — and a node that
        // has no place contributes nothing to it. A group of sound clips
        // is a group with no box, and that is right: a clip is at a time.
        let mut bounds: Option<Rect> = None;
        for id in &selected {
            let Some(node) = session.document().node(*id) else { continue };
            let Some(rect) = node.rect else { continue };
            bounds = Some(match bounds {
                None => rect,
                Some(bounds) => Rect::new(
                    bounds.x.min(rect.x),
                    bounds.y.min(rect.y),
                    (bounds.w + bounds.x).max(rect.x + rect.w) - bounds.x,
                    (bounds.h + bounds.y).max(rect.y + rect.h) - bounds.y,
                ),
            });
        }
        let rect = bounds;
        let group = session.document_mut().add_node("group", "core.group", rect);
        let mut moved = 0;
        for id in selected {
            let Some(node) = session.document_mut().take_node(id) else { continue };
            if let Some(parent) = session.document_mut().node_mut(group) {
                parent.children.push(node);
                moved += 1;
            }
        }
        Ok(Outcome::changed(format!("grouped {moved} node(s)"))
            .with_data(craftkit_sdk::serde_json::json!({ "id": group, "grouped": moved })))
    }
}

/// The panel that shows the document's nodes.
///
/// A panel is **data**: an id, a label and the commands it shows. The
/// ecosystem checks that every command it names is there, so a panel cannot
/// point at a command that does not exist.
#[derive(Panel)]
#[panel(id = "panel.nodes", label = "Nodes", layer = "core", commands("node.add", "node.mute", "node.group"))]
pub struct NodesPanel;

/// Save the document to a file.
///
/// The **newest** format writes it, and the file says which format wrote
/// it, so a program that only knows an older one can still read it — after
/// the steps.
#[derive(Command)]
#[command(id = "doc.save", area = "doc", icon = "save", mutates, undoable)]
pub struct DocSave {
    /// Where to write it.
    pub path: Text,
}

impl DocSave {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let path = self.path.0.clone();
        let Some(format) = craftkit_sdk::format::newest_format(session.formats(), "craftkit") else {
            return Err(EngineError::Failed {
                id: "doc.save".to_string(),
                message: "this program has no format to write".to_string(),
            });
        };
        let value = craftkit_sdk::format::write_document(session.document(), format);
        let text =
            craftkit_sdk::serde_json::to_string_pretty(&value).map_err(|error| EngineError::Failed {
                id: "doc.save".to_string(),
                message: format!("the document could not be written: {error}"),
            })?;
        std::fs::write(&path, text).map_err(|error| EngineError::Failed {
            id: "doc.save".to_string(),
            message: format!("`{path}` could not be written: {error}"),
        })?;
        Ok(Outcome::changed(format!("wrote `{path}`"))
            .with_data(craftkit_sdk::serde_json::json!({ "path": path, "format": format.tag })))
    }
}

/// Open a document from a file.
///
/// The file says which format wrote it. If this program knows that version,
/// the file is read as it is; if it knows a newer one, the file is
/// **migrated forward** first; if it knows an older one, the file is
/// refused, because this program does not know what it is.
#[derive(Command)]
#[command(id = "doc.open", area = "doc", icon = "open", mutates, undoable)]
pub struct DocOpen {
    /// Where to read it from.
    pub path: Text,
}

impl DocOpen {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let path = self.path.0.clone();
        let text = std::fs::read_to_string(&path).map_err(|error| EngineError::Failed {
            id: "doc.open".to_string(),
            message: format!("`{path}` could not be read: {error}"),
        })?;
        let value: craftkit_sdk::serde_json::Value =
            craftkit_sdk::serde_json::from_str(&text).map_err(|error| EngineError::Failed {
                id: "doc.open".to_string(),
                message: format!("`{path}` is not a document: {error}"),
            })?;
        let document = craftkit_sdk::format::read_document(&value, session.formats(), session.migrations())
            .map_err(|message| EngineError::Failed {
            id: "doc.open".to_string(),
            message: format!("`{path}` was refused: {message}"),
        })?;
        let name = document.name.clone();
        let nodes = document.nodes.len();
        *session.document_mut() = document;
        Ok(Outcome::changed(format!("opened `{path}`"))
            .with_data(craftkit_sdk::serde_json::json!({ "path": path, "name": name, "nodes": nodes })))
    }
}
