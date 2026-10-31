//! The application's own commands.
//!
//! Nothing here is special. The commands in this module are declared exactly
//! as a plugin declares them — a struct, an attribute, a doc comment and a
//! `run` method — because the macro does not care which crate it is in.

use craftkit_derive::{Command, Panel};
use craftkit_sdk::{
    Bool, CommandRun, CommandSpec, Doc, Either, EngineError, NodeRef, Number, Outcome, ParamDoc, Rect,
    Session, Text,
};

/// Give the document a name.
///
/// A required parameter, read from a field, and the body never sees the
/// caller's values.
#[derive(Command)]
#[command(id = "doc.rename", area = "doc", icon = "pencil", menu("File"), mutates, undoable)]
pub struct DocRename {
    /// The new name.
    pub name: Text,
}

impl DocRename {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let name = self.name.0.clone();
        let changed = session.document().name != name;
        session.document_mut()?.name = name.clone();
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
#[command(id = "node.add", area = "node", icon = "plus", menu("Node"), mutates, undoable)]
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
        let id = session.document_mut()?.add_node_with_under(parent, name.clone(), kind.clone(), rect, data);
        Ok(Outcome::changed(format!("added a {kind} `{name}`"))
            .with_data(craftkit_sdk::serde_json::json!({ "id": id, "kind": kind })))
    }
}

/// **Use** an appearance: a node takes the colours of another node.
///
/// The reference is a **reference** and not a copy: the node points at
/// the appearance, and every node that points at the same one changes
/// when it changes. A document that copied it would have twenty places
/// to change one colour, and one of them would be forgotten.
#[derive(Command)]
#[command(id = "node.appearance", area = "node", icon = "pencil", menu("Node"), mutates, undoable)]
pub struct NodeAppearance {
    /// The node that takes the appearance.
    pub node: NodeRef,
    /// The node whose appearance it takes.
    pub appearance: NodeRef,
}

impl NodeAppearance {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let node = self.node.0;
        let appearance = self.appearance.0;
        if session.document().node(node).is_none() {
            return Err(EngineError::Failed {
                id: "node.appearance".to_string(),
                message: format!("there is no node `{node}`"),
            });
        }
        if session.document().node(appearance).is_none() {
            return Err(EngineError::Failed {
                id: "node.appearance".to_string(),
                message: format!("there is no node `{appearance}`"),
            });
        }
        if let Some(node) = session.document_mut()?.node_mut(node) {
            node.appearance = None;
            node.appearance_from = Some(appearance);
        }
        Ok(Outcome::changed(format!("the node `{node}` uses the appearance `{appearance}`"))
            .with_data(craftkit_sdk::serde_json::json!({ "id": node, "appearance": appearance })))
    }
}

/// **Move** a node.
///
/// A node that has **no place** cannot be moved: a clip in a sound
/// document is at a time and not at a place, and the command does not
/// invent one.
///
/// The node keeps its size: a move is a move, and a command that
/// changed the size as well would be two commands wearing one name.
#[derive(Command)]
#[command(id = "node.move", area = "node", icon = "move", menu("Node"), mutates, undoable)]
pub struct NodeMove {
    /// The node to move.
    pub node: NodeRef,
    /// Where it goes, across.
    pub x: Number,
    /// Where it goes, up.
    pub y: Number,
}

impl NodeMove {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = self.node.0;
        let Some(node) = session.document().node(id) else {
            return Err(EngineError::Failed {
                id: "node.move".to_string(),
                message: format!("there is no node `{id}`"),
            });
        };
        let Some(rect) = node.rect else {
            return Err(EngineError::Failed {
                id: "node.move".to_string(),
                message: format!("the node `{id}` has no place to be moved to"),
            });
        };
        let x = self.x.0;
        let y = self.y.0;
        if !x.is_finite() || !y.is_finite() {
            return Err(EngineError::Failed {
                id: "node.move".to_string(),
                message: "a place is two numbers".to_string(),
            });
        }
        let moved = Rect::new(x, y, rect.w, rect.h);
        if let Some(node) = session.document_mut()?.node_mut(id) {
            node.rect = Some(moved);
        }
        Ok(Outcome::changed(format!("moved the node `{id}`"))
            .with_data(craftkit_sdk::serde_json::json!({ "id": id, "x": x, "y": y })))
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
    menu("Node"),
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
        let Some(node) = session.document_mut()?.node_mut(id) else {
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
        menu: &["runtime"],
        params: ECHO_PARAMS,
        mutates: false,
        undoable: false,
        journal: false,
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
#[command(id = "node.group", area = "node", icon = "group", menu("Node"), mutates, undoable)]
pub struct NodeGroup {}

impl NodeGroup {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let selected = session.selected_ids();
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
        let group = session.document_mut()?.add_node("group", "core.group", rect);
        let mut moved = 0;
        for id in selected {
            let Some(node) = session.document_mut()?.take_node(id) else { continue };
            if let Some(parent) = session.document_mut()?.node_mut(group) {
                parent.add_child(node);
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
#[command(id = "doc.save", area = "doc", icon = "save", menu("File"), mutates, undoable)]
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
        // The document is checked against the kit's own bounds: a file
        // that cannot be read back is not a file.
        session
            .document()
            .check_bounds()
            .map_err(|message| EngineError::Failed { id: "doc.save".to_string(), message })?;
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
/// the file is read as it is; if it knows an older one, the file is
/// **migrated forward** first; if the file is from a **newer** program,
/// it is read with the newest reader here, the fields this program does
/// not understand are **kept**, and the program says so out loud.
/// The most bytes a **document** file may hold.
///
/// A document is a tree of nodes with payloads, and this is more than
/// any document needs: it is this program's number, and another program
/// may have another one.
const MAX_DOCUMENT_FILE_BYTES: craftkit_sdk::files::ByteLimit = 16 * 1024 * 1024;

/// The most bytes a **bundle** may hold.
///
/// A bundle carries the document and the files it needs — a sound, a
/// picture — so its limit is bigger than a document's.
const MAX_BUNDLE_FILE_BYTES: craftkit_sdk::files::ByteLimit = 256 * 1024 * 1024;

#[derive(Command)]
#[command(id = "doc.open", area = "doc", icon = "open", menu("File"), mutates, undoable)]
pub struct DocOpen {
    /// Where to read it from.
    pub path: Text,
}

impl DocOpen {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let path = self.path.0.clone();
        // The file is read **within a limit**: the limit is this
        // program's, and it is checked before the read.
        let text =
            craftkit_sdk::files::read_text_bounded(std::path::Path::new(&path), MAX_DOCUMENT_FILE_BYTES)
                .map_err(|message| EngineError::Failed { id: "doc.open".to_string(), message })?;
        let value: craftkit_sdk::serde_json::Value =
            craftkit_sdk::serde_json::from_str(&text).map_err(|error| EngineError::Failed {
                id: "doc.open".to_string(),
                message: format!("`{path}` is not a document: {error}"),
            })?;
        let (document, forward) =
            craftkit_sdk::format::read_document_noting(&value, session.formats(), session.migrations())
                .map_err(|message| EngineError::Failed {
                    id: "doc.open".to_string(),
                    message: format!("`{path}` was refused: {message}"),
                })?;
        let name = document.name.clone();
        let nodes = document.nodes.len();
        *session.document_mut()? = document;
        let message = match forward {
            Some(version) => format!(
                "opened `{path}` (written by a newer program, version {version}; the parts this \
                 program does not understand were kept)"
            ),
            None => format!("opened `{path}`"),
        };
        Ok(Outcome::changed(message)
            .with_data(craftkit_sdk::serde_json::json!({ "path": path, "name": name, "nodes": nodes })))
    }
}

/// Write the document and the files it needs into a **bundle**.
///
/// The bundle is the document and its files in one file: the assets the
/// document names, and the streams of its nodes. The files are read from
/// the directory they are relative to, and a file that is not there is
/// refused by name — a bundle that is missing a file is not a bundle.
#[derive(Command)]
#[command(id = "doc.save.bundle", area = "doc", icon = "save", menu("File"), mutates, undoable)]
pub struct DocSaveBundle {
    /// Where to write the bundle.
    pub path: Text,
    /// The directory the files are relative to.
    pub from: Text,
}

impl DocSaveBundle {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let path = self.path.0.clone();
        let from = self.from.0.clone();
        let mut files: Vec<craftkit_sdk::bundle::File> = Vec::new();
        let mut wanted: Vec<String> =
            session.document().assets.iter().map(|asset| asset.path.clone()).collect();
        for node in session.document().walk() {
            match node.stream.clone() {
                Some(stream) if !wanted.contains(&stream) => wanted.push(stream),
                _ => {}
            }
        }
        for name in wanted {
            let source = std::path::Path::new(&from).join(&name);
            // The file is read **within a limit**, like every other file
            // this program reads.
            let bytes = craftkit_sdk::files::read_bounded(&source, MAX_BUNDLE_FILE_BYTES)
                .map_err(|message| EngineError::Failed { id: "doc.save.bundle".to_string(), message })?;
            files.push(craftkit_sdk::bundle::File::new(name, bytes));
        }
        let bytes =
            craftkit_sdk::bundle::write(session.document(), files, session.formats(), session.migrations())
                .map_err(|message| EngineError::Failed {
                id: "doc.save.bundle".to_string(),
                message: format!("the bundle could not be written: {message}"),
            })?;
        std::fs::write(&path, bytes).map_err(|error| EngineError::Failed {
            id: "doc.save.bundle".to_string(),
            message: format!("`{path}` could not be written: {error}"),
        })?;
        let count = session.document().assets.len();
        Ok(Outcome::changed(format!("wrote `{path}` ({count} file(s))"))
            .with_data(craftkit_sdk::serde_json::json!({ "path": path, "assets": count })))
    }
}

/// Open a **bundle**: the document and the files it needs.
///
/// The files are written into the directory they are relative to, so that
/// the document's paths mean something on this machine as well.
#[derive(Command)]
#[command(id = "doc.open.bundle", area = "doc", icon = "open", menu("File"), mutates, undoable)]
pub struct DocOpenBundle {
    /// Where to read the bundle from.
    pub path: Text,
    /// The directory the files are relative to.
    pub into: Text,
}

impl DocOpenBundle {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let path = self.path.0.clone();
        let into = self.into.0.clone();
        // The bundle is read **within a limit**: a bundle may hold
        // media, so its limit is bigger than a document's, and it is
        // this program's number.
        let bytes = craftkit_sdk::files::read_bounded(std::path::Path::new(&path), MAX_BUNDLE_FILE_BYTES)
            .map_err(|message| EngineError::Failed { id: "doc.open.bundle".to_string(), message })?;
        let bundle = craftkit_sdk::bundle::read(&bytes, session.formats(), session.migrations()).map_err(
            |message| EngineError::Failed {
                id: "doc.open.bundle".to_string(),
                message: format!("`{path}` was refused: {message}"),
            },
        )?;
        for file in &bundle.files {
            let target = std::path::Path::new(&into).join(&file.path);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|error| EngineError::Failed {
                    id: "doc.open.bundle".to_string(),
                    message: format!("`{}` could not be made: {error}", parent.display()),
                })?;
            }
            std::fs::write(&target, &file.bytes).map_err(|error| EngineError::Failed {
                id: "doc.open.bundle".to_string(),
                message: format!("`{}` could not be written: {error}", target.display()),
            })?;
        }
        let name = bundle.document.name.clone();
        let nodes = bundle.document.nodes.len();
        let files = bundle.files.len();
        *session.document_mut()? = bundle.document;
        Ok(Outcome::changed(format!("opened `{path}` ({files} file(s))")).with_data(
            craftkit_sdk::serde_json::json!({
                "path": path,
                "name": name,
                "nodes": nodes,
                "files": files,
            }),
        ))
    }
}

/// Take a picture of the document.
///
/// This is the kit's **painting seam** at work: every kind of node draws
/// itself, and the picture is what it drew. Nothing here knows what a
/// rectangle is — the canvas is given the drawing, and the drawing comes
/// from the kind.
///
/// There is **no font** in this kit: it has the metrics of a typeface
/// and not its glyphs, so text is drawn as the boxes it would occupy.
/// The picture says *there is text here*, and not what it says.
#[derive(Command)]
#[command(id = "doc.snapshot", area = "doc", icon = "camera", menu("File"))]
pub struct DocSnapshot {
    /// Where to write the picture.
    pub path: Text,
    /// How wide the picture is, in pixels.
    pub width: Option<craftkit_sdk::Number>,
    /// How tall the picture is, in pixels.
    pub height: Option<craftkit_sdk::Number>,
    /// The directory the files are relative to.
    ///
    /// A node's **stream** is a name, and the name is relative to
    /// something: the folder the document was opened from, or the folder
    /// the bundle was written beside.
    pub from: Option<Text>,
}

impl DocSnapshot {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let path = self.path.0.clone();
        let width = self.width.map_or(800.0, |width| width.0);
        let height = self.height.map_or(600.0, |height| height.0);
        if !width.is_finite() || !height.is_finite() || width < 1.0 || height < 1.0 {
            return Err(EngineError::Failed {
                id: "doc.snapshot".to_string(),
                message: "the picture has no size".to_string(),
            });
        }
        let width = width as usize;
        let height = height as usize;
        // The camera is the session's own: one set of camera controls,
        // and the picture is seen through them.
        let camera = *session.camera();
        let mut raster = craftkit_raster::Raster::new(width, height, camera)
            .map_err(|message| EngineError::Failed { id: "doc.snapshot".to_string(), message })?;
        let document = session.document();
        for node in document.walk() {
            let Some(kind) = session.kinds().get(&node.kind) else { continue };
            (kind.paint)(node, document, &mut raster);
        }
        // The pictures are drawn too: the bytes are in the files, and the
        // decoder is the family's own.
        let mut skipped: Vec<String> = Vec::new();
        let mut drawn = 0;
        for node in session.document().walk() {
            let Some(stream) = node.stream.clone() else { continue };
            let Some(format) = node.data.get("format").and_then(|format| format.as_str()) else { continue };
            let Some(rect) = node.rect else { continue };
            let file = match &self.from {
                Some(from) => std::path::Path::new(&from.0).join(&stream),
                None => std::path::Path::new(&stream).to_path_buf(),
            };
            match draw_picture(&mut raster, &file, format, rect) {
                Ok(()) => drawn += 1,
                Err(message) => skipped.push(message),
            }
        }
        let png = raster.image().to_png();
        std::fs::write(&path, png).map_err(|error| EngineError::Failed {
            id: "doc.snapshot".to_string(),
            message: format!("`{path}` could not be written: {error}"),
        })?;
        Ok(Outcome::new(format!("wrote `{path}`")).with_data(craftkit_sdk::serde_json::json!({
            "path": path,
            "width": width,
            "height": height,
            "pictures": drawn,
            "skipped": skipped,
        })))
    }
}

/// Draw a picture into the picture.
///
/// The bytes are in the file, the pixels are in the decoder, and the
/// place is the node's own rectangle. A picture that cannot be decoded
/// is **reported** and not drawn: the snapshot is of the document, and a
/// broken picture is not a reason to refuse the whole of it.
fn draw_picture(
    raster: &mut craftkit_raster::Raster,
    file: &std::path::Path,
    format: &str,
    rect: craftkit_sdk::Rect,
) -> Result<(), String> {
    let bytes = craftkit_sdk::files::read_bounded(file, 64 * 1024 * 1024)?;
    match format {
        "jpeg" => {
            let mut limits = craftkit_jpeg::Limits::default();
            limits.max_pixels = 64 * 1024 * 1024;
            let decoded = craftkit_jpeg::decode(&bytes, &limits, None)
                .map_err(|error| format!("`{}` could not be decoded: {error}", file.display()))?;
            let samples: Vec<u8> = match &decoded.samples {
                craftkit_jpeg::Samples::U8(samples) => samples.clone(),
                craftkit_jpeg::Samples::U16(samples) => {
                    samples.iter().map(|sample| (sample >> 8) as u8).collect()
                }
            };
            raster.blit(&samples, decoded.width, decoded.height, decoded.components, rect);
            Ok(())
        }
        "png" => {
            let picture = craftkit_png::read(&bytes, &Default::default())
                .map_err(|error| format!("`{}` could not be decoded: {error}", file.display()))?;
            raster.blit(&picture.samples, picture.width, picture.height, picture.channels, rect);
            Ok(())
        }
        other => Err(format!("`{}` is a `{other}`, and this snapshot reads JPEG and PNG", file.display())),
    }
}
