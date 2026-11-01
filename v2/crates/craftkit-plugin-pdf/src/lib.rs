//! `craftkit-plugin-pdf` — the **object model** of a PDF document, as v2
//! nodes and relations.
//!
//! # What is ported
//!
//! A PDF file is not a tree. It is an **object graph**: a set of
//! indirect objects, each one a **dictionary** of key and value pairs,
//! some of which are **streams** of bytes, and some of which are
//! **references** to other objects. Two objects may refer to the same
//! object, and an object may refer to one that is defined later in the
//! file.
//!
//! That is the part this battery ports — the **object model** (ISO
//! 32000-2 §7.3). The file around it comes in pieces: the
//! **cross-reference table** is read (the module `xref`), the
//! **objects** are read (the module `objects`), the **loader** puts a
//! file into the document — its objects as nodes, its references as
//! relations, and the bytes of its streams into files of their own,
//! beside the document — and the **writer** (the module `writer`) puts
//! the document back out as a file. What is still not here is the
//! **encryption** and the **object stream** on the way out: the file
//! that is written is a plain file.
//!
//! One part of the file is here as well, because it is not the file but
//! the **bytes** in it: the **filters** — the ways a stream's bytes may
//! be encoded — are in the module `filters`, the **predictors** — the
//! steps made after the encoding — are in the module `predictors`, and
//! the command `pdf.stream.decode` undoes both.
//!
//! And the **page tree** is here: a page may have a parent, and the
//! four attributes a page inherits — the media box, the resources, the
//! crop box and the rotation — are found by walking up the tree.
//!
//! # The mapping
//!
//! | the object model | v2 |
//! |---|---|
//! | an indirect object | a **node**, of the kind `pdf.object` or `pdf.stream` |
//! | its dictionary | the node's **payload** |
//! | its stream | the node's **stream** — the bytes, beside the document |
//! | a reference to another object | a **relation** of the kind `ref` |
//! | the object number | the payload's `object`, because the ids are the core's |
//!
//! # The three things the port found
//!
//! - **An edge has to be able to say something.** A reference says which
//!   *key* it is under and which *generation* of the object it names, and
//!   two references of the same kind say different things. The relation
//!   grew a **payload** (the eighth version of the format), and the
//!   reference puts the key and the generation in it.
//! - **A dictionary is ordered, and JSON is not.** A JSON object does not
//!   keep the order of its keys, so the dictionary is a **list of pairs**:
//!   `[["Type", "Page"], ["MediaBox", [0, 0, 595, 842]]]`. The pairs are
//!   in the order the file had them, which is what a rewritten file has
//!   to keep for the diff to be readable.
//! - **A reference may point forward.** A relation names two nodes that
//!   are *in the document*, so the graph is built in **two passes**: the
//!   objects first, and the references after. That is a plugin's business,
//!   and it is the price of the core's rule.
//!
//! And one that is not a finding but a fact: a PDF **name** and a PDF
//! **string** are *bytes*, and the payload is JSON. A name that is not
//! text is base64 in the payload, and the door is the SDK's.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

mod content;
mod filters;
mod loader;
mod objects;
mod predictors;
mod writer;
mod xref;

use craftkit_derive::{Command, Evaluator, Layer, NodeKind, Panel};
use craftkit_sdk::serde_json;
use craftkit_sdk::{Document, EngineError, Node, NodeRef, Number, Outcome, Plan, PlanItem, Session, Text};

/// The PDF layer.
///
/// It needs the core, and it says so.
#[derive(Layer)]
#[layer(id = "pdf", name = "PDF", requires("core"))]
pub struct PdfLayer;

/// An indirect object: a dictionary.
#[derive(NodeKind)]
#[node_kind(kind = "pdf.object", label = "PDF object", layer = "pdf", evaluates)]
pub struct ObjectKind;

impl ObjectKind {
    /// A PDF object is **not drawn**.
    ///
    /// There is no canvas in this tree, and a plugin that drew one would
    /// be a second set of camera controls (§ the README of the tree).
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Whether a point is inside an object: never.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What the object **means**: its number and its keys.
    fn evaluate(node: &Node, _document: &Document) -> Option<serde_json::Value> {
        let payload = Payload::of(node)?;
        Some(serde_json::json!({
            "object": payload.object,
            "keys": payload.keys(),
        }))
    }
}

/// An indirect object that has a stream.
#[derive(NodeKind)]
#[node_kind(kind = "pdf.stream", label = "PDF stream", layer = "pdf", evaluates)]
pub struct StreamKind;

impl StreamKind {
    /// A stream is not drawn either.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Whether a point is inside a stream: never.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What the stream **means**: its number, its keys, and the fact that
    /// it has bytes.
    fn evaluate(node: &Node, _document: &Document) -> Option<serde_json::Value> {
        let payload = Payload::of(node)?;
        Some(serde_json::json!({
            "object": payload.object,
            "keys": payload.keys(),
            "bytes": node.stream.is_some(),
        }))
    }
}

/// What a PDF node's payload says.
///
/// The payload is the battery's own: the object number, and the
/// dictionary as a list of pairs.
struct Payload {
    /// The object's number in the file.
    object: u64,
    /// The dictionary: key and value, in the order the file had them.
    entries: Vec<(String, serde_json::Value)>,
}

impl Payload {
    /// The payload of a node, if it is one of this battery's.
    fn of(node: &Node) -> Option<Payload> {
        if node.kind != "pdf.object" && node.kind != "pdf.stream" {
            return None;
        }
        let object = node.data.get("object").and_then(|object| object.as_u64())?;
        let entries = node.data.get("entries").and_then(|entries| entries.as_array())?;
        let mut pairs = Vec::with_capacity(entries.len());
        for entry in entries {
            let pair = entry.as_array()?;
            let key = pair.first()?.as_str()?.to_string();
            let value = pair.get(1)?.clone();
            pairs.push((key, value));
        }
        Some(Payload { object, entries: pairs })
    }

    /// The keys of the dictionary, in order.
    fn keys(&self) -> Vec<String> {
        self.entries.iter().map(|(key, _)| key.clone()).collect()
    }
}

/// Put an indirect object in the document.
#[derive(Command)]
#[command(id = "pdf.object.add", area = "pdf", layer = "pdf", icon = "plus", mutates, undoable)]
pub struct AddObject {
    /// The object's number in the file.
    pub object: Number,
    /// The dictionary: a list of `[key, value]` pairs, in order.
    pub entries: serde_json::Value,
}

impl AddObject {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let object = self.object.0;
        if !object.is_finite() || object < 0.0 || object.fract() != 0.0 {
            return Err(EngineError::Failed {
                id: "pdf.object.add".to_string(),
                message: "an object number is a whole number".to_string(),
            });
        }
        let number = object as u64;
        if number == 0 {
            return Err(EngineError::Failed {
                id: "pdf.object.add".to_string(),
                message: "object numbers start at one".to_string(),
            });
        }
        let entries = check_entries(&self.entries)?;
        let data = serde_json::json!({ "object": number, "entries": entries });
        let name = format!("object {number}");
        let id = session.document_mut()?.add_node_with(name.clone(), "pdf.object", None, data);
        Ok(Outcome::changed(format!("added the `{name}`"))
            .with_data(serde_json::json!({ "id": id, "object": number })))
    }
}

/// Put an indirect object that has a **stream** in the document.
///
/// The bytes are not in the document: they are in the file beside it, and
/// the node says which file is its stream.
#[derive(Command)]
#[command(id = "pdf.stream.add", area = "pdf", layer = "pdf", icon = "plus", mutates, undoable)]
pub struct AddStream {
    /// The object's number in the file.
    pub object: Number,
    /// The dictionary: a list of `[key, value]` pairs, in order.
    pub entries: serde_json::Value,
    /// Where the bytes are, relative to the document.
    pub stream: Text,
}

impl AddStream {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let object = self.object.0;
        if !object.is_finite() || object < 0.0 || object.fract() != 0.0 {
            return Err(EngineError::Failed {
                id: "pdf.stream.add".to_string(),
                message: "an object number is a whole number".to_string(),
            });
        }
        let number = object as u64;
        if number == 0 {
            return Err(EngineError::Failed {
                id: "pdf.stream.add".to_string(),
                message: "object numbers start at one".to_string(),
            });
        }
        let entries = check_entries(&self.entries)?;
        let path = self.stream.0.clone();
        if path.trim().is_empty() {
            return Err(EngineError::Failed {
                id: "pdf.stream.add".to_string(),
                message: "a stream has a path".to_string(),
            });
        }
        let data = serde_json::json!({ "object": number, "entries": entries });
        let name = format!("object {number}");
        let id = session.document_mut()?.add_node_with(name.clone(), "pdf.stream", None, data);
        if let Some(node) = session.document_mut()?.node_mut(id) {
            node.stream = Some(path.clone());
        }
        Ok(Outcome::changed(format!("added the `{name}` with its stream"))
            .with_data(serde_json::json!({ "id": id, "object": number, "stream": path })))
    }
}

/// **Decode** a stream's bytes.
///
/// A PDF's stream may be **encoded**: the `/Filter` entry names the way
/// and the bytes in the file are the encoded ones. The command reads
/// the file the node names, undoes the filter and writes what was
/// inside it to another file.
///
/// The node is not changed: it still names the **encoded** bytes,
/// because that is what the PDF file holds, and the decoded bytes are
/// what the caller asked for.
#[derive(Command)]
#[command(id = "pdf.stream.decode", area = "pdf", layer = "pdf", icon = "file", menu("PDF", "Streams"))]
pub struct DecodeStream {
    /// The stream whose bytes are decoded.
    pub stream: NodeRef,
    /// The filter to undo: `FlateDecode`, `LZWDecode`,
    /// `ASCIIHexDecode`, `ASCII85Decode` or `RunLengthDecode`.
    ///
    /// Leave it out and the **dictionary** names it, which is where a
    /// PDF file says which filter a stream is in.
    pub filter: Option<Text>,
    /// The **predictor** to undo: the number the PDF gives it — `1` is
    /// none, `2` is the TIFF one and `10` to `15` are the PNG ones.
    ///
    /// Leave it out and the **dictionary** names it, in
    /// `/DecodeParms`. The other parameters of the predictor — the
    /// columns, the colours and the bits — come from the dictionary
    /// whatever this says.
    pub predictor: Option<Number>,
    /// Where to write the decoded bytes.
    pub to: Text,
}

impl DecodeStream {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = self.stream.0;
        let Some(node) = session.document().node(id) else {
            return Err(EngineError::Failed {
                id: "pdf.stream.decode".to_string(),
                message: format!("there is no node `{id}`"),
            });
        };
        if node.kind != "pdf.stream" {
            return Err(EngineError::Failed {
                id: "pdf.stream.decode".to_string(),
                message: format!("the node `{id}` is a `{}` and not a stream", node.kind),
            });
        }
        let to = self.to.0.clone();
        if to.trim().is_empty() {
            return Err(EngineError::Failed {
                id: "pdf.stream.decode".to_string(),
                message: "the decoded bytes go somewhere".to_string(),
            });
        }
        let decoded = decoded_bytes(
            node,
            self.filter.as_ref().map(|filter| filter.0.as_str()),
            self.predictor.as_ref().map(|predictor| predictor.0),
            true,
            "pdf.stream.decode",
        )?;
        std::fs::write(&to, &decoded.bytes).map_err(|error| EngineError::Failed {
            id: "pdf.stream.decode".to_string(),
            message: format!("the decoded bytes cannot be written to `{to}`: {error}"),
        })?;
        let from = node.stream.clone().unwrap_or_default();
        let count = decoded.bytes.len();
        Ok(Outcome::new(format!("decoded {count} bytes")).with_data(serde_json::json!({
            "id": id,
            "from": from,
            "to": to,
            "filters": decoded.filters,
            "predictor": decoded.predictor.map(|kind| format!("{kind:?}")),
            "bytes": count,
        })))
    }
}

/// The **decoded bytes** of a stream node.
///
/// The bytes are read from the file the node names, the **filter** is
/// undone — the one the caller names, or the one the dictionary names —
/// and the **predictor** is undone after it.
fn decoded_bytes(
    node: &Node,
    filter: Option<&str>,
    predictor: Option<f64>,
    require_filter: bool,
    command: &str,
) -> Result<Decoded, EngineError> {
    let id = node.id;
    let Some(path) = node.stream.clone() else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the stream `{id}` has no file"),
        });
    };
    // The **filter**: the one the command was given, or the one the
    // dictionary names.
    let names = match filter {
        Some(name) => vec![name.to_string()],
        None => match filters::filter_of(&entries_of(&node.data)) {
            Some(names) => names,
            // A **content stream** is usually not encoded at all, and
            // its bytes are the content; a stream that is being
            // decoded must say what it is in.
            None if require_filter => {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: format!("the stream `{id}` does not name a filter"),
                });
            }
            None => Vec::new(),
        },
    };
    let limits = filters::Limits::default();
    let bytes = craftkit_sdk::files::read_bounded(std::path::Path::new(&path), limits.max_input as u64)
        .map_err(|message| EngineError::Failed {
            id: command.to_string(),
            message: format!("the stream cannot be read: {message}"),
        })?;
    let decoded = filters::decode_named(&names, &bytes, &limits).map_err(|error| EngineError::Failed {
        id: command.to_string(),
        message: format!("the stream cannot be decoded: {error:?}"),
    })?;
    // The **predictor**: the step that was made after the encoding.
    // The command may say which one it is, and the parameters come
    // from the dictionary either way.
    let entries = entries_of(&node.data);
    let base = predictors::spec_of(&entries).map_err(|error| EngineError::Failed {
        id: command.to_string(),
        message: format!("the predictor cannot be read: {error:?}"),
    })?;
    let spec = match (predictor, base) {
        (None, None) => None,
        (None, Some(spec)) => Some(spec),
        (Some(number), base) => {
            let number = number.round() as i64;
            let Some(predictor) = predictors::Predictor::by_number(number) else {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: format!("the predictor {number} is not one of the kit's"),
                });
            };
            let mut spec = base.unwrap_or_default();
            spec.predictor = predictor;
            Some(spec)
        }
    };
    let decoded = match spec {
        Some(spec) => predictors::undo(&decoded, &spec).map_err(|error| EngineError::Failed {
            id: command.to_string(),
            message: format!("the predictor cannot be undone: {error:?}"),
        })?,
        None => decoded,
    };
    let kind = spec.map(|spec| spec.predictor);
    Ok(Decoded { bytes: decoded, filters: names, predictor: kind })
}

/// The **decoded bytes** of a stream, and what was done to them.
struct Decoded {
    /// The bytes, after the filter and the predictor.
    bytes: Vec<u8>,
    /// The filters that were undone.
    filters: Vec<String>,
    /// The predictor that was undone, if there was one.
    predictor: Option<predictors::Predictor>,
}

/// Read the **operators** of a content stream.
///
/// A page's content is a stream of operands and operators, and the
/// command says what the stream says: the operators in the order the
/// stream has them, with the operands each one was given.
#[derive(Command)]
#[command(id = "pdf.content.operators", area = "pdf", layer = "pdf", icon = "list")]
pub struct ContentOperators {
    /// The stream whose bytes are read.
    pub stream: NodeRef,
    /// The filter to undo, when the dictionary does not name one.
    pub filter: Option<Text>,
}

impl ContentOperators {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = self.stream.0;
        let Some(node) = session.document().node(id) else {
            return Err(EngineError::Failed {
                id: "pdf.content.operators".to_string(),
                message: format!("there is no node `{id}`"),
            });
        };
        if node.kind != "pdf.stream" {
            return Err(EngineError::Failed {
                id: "pdf.content.operators".to_string(),
                message: format!("the node `{id}` is a `{}` and not a stream", node.kind),
            });
        }
        let decoded = decoded_bytes(
            node,
            self.filter.as_ref().map(|filter| filter.0.as_str()),
            None,
            false,
            "pdf.content.operators",
        )?;
        let operations = content::operations(&decoded.bytes).map_err(|error| EngineError::Failed {
            id: "pdf.content.operators".to_string(),
            message: format!("the content cannot be read: {error:?}"),
        })?;
        let count = operations.len();
        let list: Vec<serde_json::Value> = operations.iter().map(content::Operation::to_json).collect();
        Ok(Outcome::new(format!("{count} operation(s)"))
            .with_data(serde_json::json!({ "id": id, "operations": list })))
    }
}

/// Put a **page** in the document.
///
/// A page is an indirect object whose dictionary says `/Type /Page`, and
/// it may have a **parent**: the page is in a tree, and the tree is
/// what a PDF file has instead of a list of pages.
#[derive(Command)]
#[command(id = "pdf.page.add", area = "pdf", layer = "pdf", icon = "plus", mutates, undoable)]
pub struct AddPage {
    /// The object's number in the file.
    pub object: Number,
    /// The page's own dictionary: a list of `[key, value]` pairs, in
    /// order.
    pub entries: Option<serde_json::Value>,
    /// The page's parent, if it has one.
    pub parent: Option<NodeRef>,
}

impl AddPage {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let object = self.object.0;
        if !object.is_finite() || object < 0.0 || object.fract() != 0.0 {
            return Err(EngineError::Failed {
                id: "pdf.page.add".to_string(),
                message: "an object number is a whole number".to_string(),
            });
        }
        let number = object as u64;
        if number == 0 {
            return Err(EngineError::Failed {
                id: "pdf.page.add".to_string(),
                message: "object numbers start at one".to_string(),
            });
        }
        let mut entries = match &self.entries {
            Some(entries) => check_entries(entries)?,
            None => Vec::new(),
        };
        // The page says what it is: the type is the page's own.
        let kind = entries.iter().find_map(|entry| {
            let pair = entry.as_array()?;
            if pair.first()?.as_str()? == "Type" { Some(pair.get(1)?.clone()) } else { None }
        });
        match kind {
            Some(value) if value.as_str() != Some("Page") => {
                return Err(EngineError::Failed {
                    id: "pdf.page.add".to_string(),
                    message: "a page's type is `Page`".to_string(),
                });
            }
            Some(_) => {}
            None => entries.push(serde_json::json!(["Type", "Page"])),
        }
        let parent = self.parent.map(|parent| parent.0);
        if let Some(parent) = parent {
            let node = session.document().node(parent).ok_or_else(|| EngineError::Failed {
                id: "pdf.page.add".to_string(),
                message: format!("there is no node `{parent}` in the document"),
            })?;
            if node.kind != "pdf.object" && node.kind != "pdf.stream" {
                return Err(EngineError::Failed {
                    id: "pdf.page.add".to_string(),
                    message: format!("the node `{parent}` is a `{}` and not an object", node.kind),
                });
            }
        }
        let data = serde_json::json!({ "object": number, "entries": entries });
        let name = format!("page {number}");
        let id = session.document_mut()?.add_node_with(name.clone(), "pdf.object", None, data);
        if let Some(parent) = parent {
            // The page names its parent and the parent names the page:
            // the tree is two edges, as the file has it.
            for (from, to, key) in [(id, parent, "Parent"), (parent, id, "Kids")] {
                let payload = serde_json::json!({ "key": key, "generation": 0 });
                let relation = craftkit_sdk::Relation::with_payload(from, to, "ref", payload);
                if !session.document_mut()?.add_relation(relation) {
                    return Err(EngineError::Failed {
                        id: "pdf.page.add".to_string(),
                        message: format!("the page `{id}` is already under `{parent}`"),
                    });
                }
            }
        }
        Ok(Outcome::changed(format!("added the `{name}`"))
            .with_data(serde_json::json!({ "id": id, "object": number, "parent": parent })))
    }
}

/// The **attributes** of a page, and the ones it inherits.
///
/// A page may leave out what a page does not have to say: the **media
/// box**, the **resources**, the **crop box** and the **rotation** are
/// inherited from the nearest ancestor that has them. The command says
/// what the page has, and where each thing came from.
#[derive(Command)]
#[command(id = "pdf.page.attributes", area = "pdf", layer = "pdf", icon = "list")]
pub struct PageAttributes {
    /// The page.
    pub page: NodeRef,
}

/// The keys a page **inherits**.
const INHERITED: [&str; 4] = ["Resources", "MediaBox", "CropBox", "Rotate"];

impl PageAttributes {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = self.page.0;
        let node = session.document().node(id).ok_or_else(|| EngineError::Failed {
            id: "pdf.page.attributes".to_string(),
            message: format!("there is no node `{id}`"),
        })?;
        let own = entries_of(&node.data);
        let mut attributes = serde_json::Map::new();
        let mut inherited = serde_json::Map::new();
        for key in INHERITED {
            if let Some((_, value)) = own.iter().find(|(name, _)| name == key) {
                attributes.insert(key.to_string(), value.clone());
                continue;
            }
            // The **ancestors**: the nearest one that has it wins.
            let mut ancestor = parent_of(session, id);
            let mut steps = 0_usize;
            let mut found = None;
            while let Some(current) = ancestor {
                steps += 1;
                if steps > MAX_ANCESTORS {
                    break;
                }
                let Some(current_node) = session.document().node(current) else { break };
                let entries = entries_of(&current_node.data);
                if let Some((_, value)) = entries.iter().find(|(name, _)| name == key) {
                    found = Some((value.clone(), current));
                    break;
                }
                ancestor = parent_of(session, current);
            }
            if let Some((value, from)) = found {
                attributes.insert(key.to_string(), value);
                inherited.insert(key.to_string(), serde_json::json!(from));
            }
        }
        Ok(Outcome::new(format!("{} attribute(s)", attributes.len())).with_data(serde_json::json!({
            "id": id,
            "attributes": attributes,
            "inherited": inherited,
        })))
    }
}

/// The **parent** of a page.
fn parent_of(session: &Session, id: u64) -> Option<u64> {
    session
        .document()
        .relations
        .iter()
        .find(|relation| {
            relation.kind == "ref"
                && relation.from == id
                && relation
                    .payload
                    .get("key")
                    .and_then(|key| key.as_str())
                    .map(|key| key == "Parent")
                    .unwrap_or(false)
        })
        .map(|relation| relation.to)
}

/// How many **ancestors** a page may have.
///
/// A page tree is a tree, and a tree has no loop in it; the bound is
/// there so that a document that says otherwise is not walked for ever.
const MAX_ANCESTORS: usize = 64;

/// Read the **objects** of an object stream.
///
/// An object stream is a stream that holds a file's objects, so that
/// they may be compressed together. The command reads them: each one's
/// number and what it says.
#[derive(Command)]
#[command(id = "pdf.objstm.read", area = "pdf", layer = "pdf", icon = "list")]
pub struct ReadObjectStream {
    /// The stream whose objects are read.
    pub stream: NodeRef,
}

impl ReadObjectStream {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = self.stream.0;
        let Some(node) = session.document().node(id) else {
            return Err(EngineError::Failed {
                id: "pdf.objstm.read".to_string(),
                message: format!("there is no node `{id}`"),
            });
        };
        if node.kind != "pdf.stream" {
            return Err(EngineError::Failed {
                id: "pdf.objstm.read".to_string(),
                message: format!("the node `{id}` is a `{}` and not a stream", node.kind),
            });
        }
        let entries = entries_of(&node.data);
        // The **count** and the **first**: what the dictionary says.
        let count =
            entries.iter().find(|(key, _)| key == "N").and_then(|(_, value)| value.as_u64()).ok_or_else(
                || EngineError::Failed {
                    id: "pdf.objstm.read".to_string(),
                    message: "the object stream does not say how many objects it has".to_string(),
                },
            )?;
        let first =
            entries.iter().find(|(key, _)| key == "First").and_then(|(_, value)| value.as_u64()).ok_or_else(
                || EngineError::Failed {
                    id: "pdf.objstm.read".to_string(),
                    message: "the object stream does not say where the first object begins".to_string(),
                },
            )?;
        let decoded = decoded_bytes(node, None, None, false, "pdf.objstm.read")?;
        let objects =
            objects::object_stream(&decoded.bytes, count as usize, first as usize).map_err(|error| {
                EngineError::Failed {
                    id: "pdf.objstm.read".to_string(),
                    message: format!("the object stream cannot be read: {error:?}"),
                }
            })?;
        let list: Vec<serde_json::Value> = objects
            .iter()
            .map(|object| serde_json::json!({ "object": object.number, "value": object.value }))
            .collect();
        let count = list.len();
        Ok(Outcome::new(format!("{count} object(s)"))
            .with_data(serde_json::json!({ "id": id, "objects": list })))
    }
}

/// Read the **cross-reference table** of a file.
///
/// The table is the file's own index of itself: it says where each
/// object is. The command reads it and reports the entries and the
/// trailer.
#[derive(Command)]
#[command(id = "pdf.file.xref", area = "pdf", layer = "pdf", icon = "list")]
pub struct ReadXref {
    /// The file to read.
    pub path: Text,
}

impl ReadXref {
    fn run(&self, _session: &mut Session) -> Result<Outcome, EngineError> {
        let path = self.path.0.clone();
        if path.trim().is_empty() {
            return Err(EngineError::Failed {
                id: "pdf.file.xref".to_string(),
                message: "a file has a path".to_string(),
            });
        }
        let limits = filters::Limits::default();
        let bytes = craftkit_sdk::files::read_bounded(std::path::Path::new(&path), limits.max_input as u64)
            .map_err(|message| EngineError::Failed {
            id: "pdf.file.xref".to_string(),
            message: format!("the file cannot be read: {message}"),
        })?;
        let table = xref::read(&bytes).map_err(|error| EngineError::Failed {
            id: "pdf.file.xref".to_string(),
            message: format!("the table cannot be read: {error:?}"),
        })?;
        let entries: Vec<serde_json::Value> = table
            .entries
            .iter()
            .map(|entry| {
                serde_json::json!({
                    "object": entry.number,
                    "offset": entry.offset,
                    "generation": entry.generation,
                    "kind": match entry.kind {
                        xref::Kind::InFile => "in-file",
                        xref::Kind::Free => "free",
                        xref::Kind::Compressed => "compressed",
                    },
                })
            })
            .collect();
        let trailer: Vec<serde_json::Value> =
            table.trailer.iter().map(|(key, value)| serde_json::json!([key, value])).collect();
        let count = entries.len();
        Ok(Outcome::new(format!("{count} entry(ies)"))
            .with_data(serde_json::json!({ "path": path, "entries": entries, "trailer": trailer })))
    }
}

/// Load a PDF **file** into the document.
///
/// The file's own index says where its objects are, and the loader
/// reads them: every object becomes a **node**, and every reference
/// becomes a **relation**. The bytes of a stream go into a file of
/// their own, beside the document, because the bytes are not the
/// document.
#[derive(Command)]
#[command(id = "pdf.file.load", area = "pdf", layer = "pdf", icon = "file", mutates, undoable)]
pub struct LoadFile {
    /// The file to read.
    pub path: Text,
    /// Where the **streams' bytes** go: a directory of this file's own.
    pub into: Text,
}

impl LoadFile {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let path = self.path.0.clone();
        if path.trim().is_empty() {
            return Err(EngineError::Failed {
                id: "pdf.file.load".to_string(),
                message: "a file has a path".to_string(),
            });
        }
        let into = self.into.0.clone();
        if into.trim().is_empty() {
            return Err(EngineError::Failed {
                id: "pdf.file.load".to_string(),
                message: "the streams' bytes go somewhere".to_string(),
            });
        }
        let limits = filters::Limits::default();
        let bytes = craftkit_sdk::files::read_bounded(std::path::Path::new(&path), limits.max_input as u64)
            .map_err(|message| EngineError::Failed {
            id: "pdf.file.load".to_string(),
            message: format!("the file cannot be read: {message}"),
        })?;
        let objects = loader::load(&bytes).map_err(|error| EngineError::Failed {
            id: "pdf.file.load".to_string(),
            message: format!("the file cannot be loaded: {error:?}"),
        })?;
        std::fs::create_dir_all(&into).map_err(|error| EngineError::Failed {
            id: "pdf.file.load".to_string(),
            message: format!("the streams' bytes have nowhere to go: {error}"),
        })?;
        // The **first pass**: the objects. A reference may point
        // forward, so every object is in the document before any
        // reference is made.
        let mut nodes: Vec<(u64, u64)> = Vec::with_capacity(objects.len());
        for object in &objects {
            let entries: Vec<serde_json::Value> =
                object.entries.iter().map(|(key, value)| serde_json::json!([key, value])).collect();
            let data = serde_json::json!({ "object": object.number, "entries": entries });
            let has_stream = object.stream.is_some();
            let name = if has_stream {
                format!("stream {}", object.number)
            } else {
                format!("object {}", object.number)
            };
            let kind = if has_stream { "pdf.stream" } else { "pdf.object" };
            let id = session.document_mut()?.add_node_with(name, kind, None, data);
            if let Some(stream) = &object.stream {
                // The **bytes** are in a file of their own.
                let stream_path = format!("{into}/{}.bin", object.number);
                std::fs::write(&stream_path, stream).map_err(|error| EngineError::Failed {
                    id: "pdf.file.load".to_string(),
                    message: format!("the stream's bytes cannot be written: {error}"),
                })?;
                if let Some(node) = session.document_mut()?.node_mut(id) {
                    node.stream = Some(stream_path);
                }
            }
            nodes.push((object.number, id));
        }
        // The **second pass**: the references.
        let mut relations = 0_usize;
        for object in &objects {
            let Some((_, from)) = nodes.iter().find(|(number, _)| *number == object.number) else {
                continue;
            };
            for reference in &object.references {
                let Some((_, to)) = nodes.iter().find(|(number, _)| *number == reference.number) else {
                    continue;
                };
                let payload = serde_json::json!({
                    "key": reference.key,
                    "generation": reference.generation,
                });
                let relation = craftkit_sdk::Relation::with_payload(*from, *to, "ref", payload);
                if session.document_mut()?.add_relation(relation) {
                    relations += 1;
                }
            }
        }
        let count = objects.len();
        Ok(Outcome::changed(format!("loaded {count} object(s)")).with_data(serde_json::json!({
            "path": path,
            "into": into,
            "objects": count,
            "relations": relations,
        })))
    }
}

/// Write the document out as a PDF **file**.
///
/// The objects of the document become the objects of a file, and the
/// file says where each one is.
#[derive(Command)]
#[command(id = "pdf.file.write", area = "pdf", layer = "pdf", icon = "file", mutates, undoable)]
pub struct WriteFile {
    /// Where to write the file.
    pub path: Text,
    /// The number of the object that is the file's **root**.
    ///
    /// Leave it out and the object whose type is `Catalog` is the root.
    pub root: Option<Number>,
}

impl WriteFile {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let path = self.path.0.clone();
        if path.trim().is_empty() {
            return Err(EngineError::Failed {
                id: "pdf.file.write".to_string(),
                message: "a file has a path".to_string(),
            });
        }
        // The **objects**: every node of the battery, with its number
        // and its dictionary.
        let mut objects: Vec<writer::Object> = Vec::new();
        for node in session.document().walk() {
            if node.kind != "pdf.object" && node.kind != "pdf.stream" {
                continue;
            }
            let Some(number) = node.data.get("object").and_then(|number| number.as_u64()) else {
                continue;
            };
            let entries = entries_of(&node.data);
            // The **bytes** of a stream are in the file they came from.
            let stream = match &node.stream {
                Some(path) => {
                    let limits = filters::Limits::default();
                    let bytes = craftkit_sdk::files::read_bounded(
                        std::path::Path::new(path),
                        limits.max_input as u64,
                    )
                    .map_err(|message| EngineError::Failed {
                        id: "pdf.file.write".to_string(),
                        message: format!("the stream's bytes cannot be read: {message}"),
                    })?;
                    Some(bytes)
                }
                None => None,
            };
            objects.push(writer::Object { number, entries, stream });
        }
        if objects.is_empty() {
            return Err(EngineError::Failed {
                id: "pdf.file.write".to_string(),
                message: "the document holds no objects to write".to_string(),
            });
        }
        // The **root**: the one that was named, or the catalog.
        let root = match &self.root {
            Some(root) => {
                let number = root.0;
                if !number.is_finite() || number < 0.0 || number.fract() != 0.0 {
                    return Err(EngineError::Failed {
                        id: "pdf.file.write".to_string(),
                        message: "an object number is a whole number".to_string(),
                    });
                }
                number as u64
            }
            None => objects
                .iter()
                .find(|object| {
                    object
                        .entries
                        .iter()
                        .any(|(key, value)| key == "Type" && value.as_str() == Some("Catalog"))
                })
                .map(|object| object.number)
                .ok_or_else(|| EngineError::Failed {
                    id: "pdf.file.write".to_string(),
                    message: "the document holds no catalog to be the root of the file".to_string(),
                })?,
        };
        let file = writer::write(&objects, root).map_err(|error| EngineError::Failed {
            id: "pdf.file.write".to_string(),
            message: format!("the file cannot be written: {error:?}"),
        })?;
        std::fs::write(&path, &file).map_err(|error| EngineError::Failed {
            id: "pdf.file.write".to_string(),
            message: format!("the file cannot be written to `{path}`: {error}"),
        })?;
        Ok(Outcome::changed(format!("wrote {} object(s)", objects.len()))
            .with_data(serde_json::json!({ "path": path, "root": root, "bytes": file.len() })))
    }
}

/// Say that one object refers to another.
///
/// The reference says which **key** it is under and which **generation**
/// of the object it names, and the relation carries both in its payload.
#[derive(Command)]
#[command(id = "pdf.ref.add", area = "pdf", layer = "pdf", icon = "link", mutates, undoable)]
pub struct AddReference {
    /// The object the reference is in.
    pub from: NodeRef,
    /// The key the reference is under.
    pub key: Text,
    /// The object the reference names.
    pub to: NodeRef,
    /// The generation of the object it names, if it is not the first.
    pub generation: Option<Number>,
}

impl AddReference {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let from = self.from.0;
        let to = self.to.0;
        for id in [from, to] {
            if session.document().node(id).is_none() {
                return Err(EngineError::Failed {
                    id: "pdf.ref.add".to_string(),
                    message: format!("there is no object `{id}` in the document"),
                });
            }
        }
        if from == to {
            return Err(EngineError::Failed {
                id: "pdf.ref.add".to_string(),
                message: format!("the object `{from}` does not refer to itself"),
            });
        }
        let generation = match &self.generation {
            Some(generation) if generation.0.is_finite() && generation.0 >= 0.0 => generation.0 as u64,
            Some(_) => {
                return Err(EngineError::Failed {
                    id: "pdf.ref.add".to_string(),
                    message: "a generation is a whole number".to_string(),
                });
            }
            None => 0,
        };
        let key = self.key.0.clone();
        if key.is_empty() {
            return Err(EngineError::Failed {
                id: "pdf.ref.add".to_string(),
                message: "a reference is under a key".to_string(),
            });
        }
        let payload = serde_json::json!({ "key": key, "generation": generation });
        let relation = craftkit_sdk::Relation::with_payload(from, to, "ref", payload.clone());
        if !session.document_mut()?.add_relation(relation) {
            return Err(EngineError::Failed {
                id: "pdf.ref.add".to_string(),
                message: format!("the object `{from}` already refers to `{to}`"),
            });
        }
        Ok(Outcome::changed(format!("`{from}` refers to `{to}` under `{key}`"))
            .with_data(serde_json::json!({ "from": from, "to": to, "payload": payload })))
    }
}

/// The references of the document.
///
/// A query: it does not change the document.
#[derive(Command)]
#[command(id = "pdf.refs", area = "pdf", layer = "pdf", icon = "list")]
pub struct ListReferences {}

impl ListReferences {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let refs: Vec<serde_json::Value> = session
            .document()
            .relations
            .iter()
            .filter(|relation| relation.kind == "ref")
            .map(|relation| {
                serde_json::json!({
                    "from": relation.from,
                    "to": relation.to,
                    "payload": relation.payload,
                })
            })
            .collect();
        let count = refs.len();
        Ok(Outcome::new(format!("{count} reference(s)")).with_data(serde_json::json!({ "references": refs })))
    }
}

/// The evaluator that measures the **object graph**.
///
/// It says how many references point at each object — the in-degree of
/// the graph, which is **derived**: it is never stored, and it changes
/// when a reference is made or unmade.
#[derive(Evaluator)]
#[evaluator(id = "pdf.graph", label = "Object graph", layer = "pdf")]
pub struct GraphEvaluator;

impl GraphEvaluator {
    /// Count the references that point at each object.
    fn evaluate(document: &Document) -> Plan {
        let mut plan = Plan::new();
        for node in document.walk() {
            if node.kind != "pdf.object" && node.kind != "pdf.stream" {
                continue;
            }
            let Some(payload) = Payload::of(node) else { continue };
            let incoming = document
                .relations
                .iter()
                .filter(|relation| relation.kind == "ref" && relation.to == node.id)
                .count();
            plan.items.push(PlanItem {
                node: node.id,
                kind: "pdf.graph".to_string(),
                value: serde_json::json!({
                    "object": payload.object,
                    "keys": payload.keys().len(),
                    "referenced_by": incoming,
                }),
            });
        }
        plan
    }
}

/// The panel that shows the objects.
#[derive(Panel)]
#[panel(id = "panel.pdf", label = "PDF objects", layer = "pdf", commands("pdf.object.add", "pdf.ref.add"))]
pub struct PdfPanel;

/// Check the dictionary, and give it back.
///
/// The dictionary is a **list of pairs**, because a JSON object does not
/// keep the order of its keys and a PDF dictionary does.
fn check_entries(entries: &serde_json::Value) -> Result<Vec<serde_json::Value>, EngineError> {
    let Some(list) = entries.as_array() else {
        return Err(EngineError::Failed {
            id: "pdf.object.add".to_string(),
            message: "a dictionary is a list of `[key, value]` pairs".to_string(),
        });
    };
    for entry in list {
        let Some(pair) = entry.as_array() else {
            return Err(EngineError::Failed {
                id: "pdf.object.add".to_string(),
                message: "a dictionary is a list of `[key, value]` pairs".to_string(),
            });
        };
        if pair.len() != 2 {
            return Err(EngineError::Failed {
                id: "pdf.object.add".to_string(),
                message: "a pair is a key and a value".to_string(),
            });
        }
        if pair[0].as_str().is_none() {
            return Err(EngineError::Failed {
                id: "pdf.object.add".to_string(),
                message: "a key is text".to_string(),
            });
        }
    }
    Ok(list.clone())
}

/// The **dictionary** of an object, as the pairs it holds.
///
/// The dictionary is a list of `[key, value]` pairs, and the pairs are
/// in the order the file had them.
fn entries_of(payload: &serde_json::Value) -> Vec<(String, serde_json::Value)> {
    let Some(list) = payload.get("entries").and_then(|entries| entries.as_array()) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|entry| {
            let pair = entry.as_array()?;
            let key = pair.first()?.as_str()?.to_string();
            let value = pair.get(1)?.clone();
            Some((key, value))
        })
        .collect()
}

// The plugin's anchor: one name that keeps every item above in the
// program, on every target.
craftkit_sdk::plugin_anchor!(
    PdfLayer,
    ObjectKind,
    StreamKind,
    AddObject,
    AddStream,
    AddPage,
    PageAttributes,
    DecodeStream,
    ContentOperators,
    ReadObjectStream,
    ReadXref,
    LoadFile,
    WriteFile,
    AddReference,
    ListReferences,
    GraphEvaluator,
    PdfPanel
);
