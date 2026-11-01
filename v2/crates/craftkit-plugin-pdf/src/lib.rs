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
//! 32000-2 §7.3), not the file: the cross-reference table, the
//! incremental save and the parser are the *file*, and a v2 document
//! is not a PDF file.
//!
//! One part of the file is here as well, because it is not the file but
//! the **bytes** in it: the **filters** — the ways a stream's bytes may
//! be encoded — are in the module `filters`, the **predictors** — the
//! steps made after the encoding — are in the module `predictors`, and
//! the command `pdf.stream.decode` undoes both.
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

mod filters;
mod predictors;

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
        let Some(path) = node.stream.clone() else {
            return Err(EngineError::Failed {
                id: "pdf.stream.decode".to_string(),
                message: format!("the stream `{id}` has no file"),
            });
        };
        // The **filter**: the one the command was given, or the one the
        // dictionary names.
        let names = match &self.filter {
            Some(name) => vec![name.0.clone()],
            None => filters::filter_of(&entries_of(&node.data)).ok_or_else(|| EngineError::Failed {
                id: "pdf.stream.decode".to_string(),
                message: format!("the stream `{id}` does not name a filter"),
            })?,
        };
        let to = self.to.0.clone();
        if to.trim().is_empty() {
            return Err(EngineError::Failed {
                id: "pdf.stream.decode".to_string(),
                message: "the decoded bytes go somewhere".to_string(),
            });
        }
        let limits = filters::Limits::default();
        let bytes = craftkit_sdk::files::read_bounded(std::path::Path::new(&path), limits.max_input as u64)
            .map_err(|message| EngineError::Failed {
            id: "pdf.stream.decode".to_string(),
            message: format!("the stream cannot be read: {message}"),
        })?;
        let decoded =
            filters::decode_named(&names, &bytes, &limits).map_err(|error| EngineError::Failed {
                id: "pdf.stream.decode".to_string(),
                message: format!("the stream cannot be decoded: {error:?}"),
            })?;
        // The **predictor**: the step that was made after the encoding.
        // The command may say which one it is, and the parameters come
        // from the dictionary either way.
        let entries = entries_of(&node.data);
        let base = predictors::spec_of(&entries).map_err(|error| EngineError::Failed {
            id: "pdf.stream.decode".to_string(),
            message: format!("the predictor cannot be read: {error:?}"),
        })?;
        let spec = match (&self.predictor, base) {
            (None, None) => None,
            (None, Some(spec)) => Some(spec),
            (Some(number), base) => {
                let number = number.0.round() as i64;
                let Some(predictor) = predictors::Predictor::by_number(number) else {
                    return Err(EngineError::Failed {
                        id: "pdf.stream.decode".to_string(),
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
                id: "pdf.stream.decode".to_string(),
                message: format!("the predictor cannot be undone: {error:?}"),
            })?,
            None => decoded,
        };
        std::fs::write(&to, &decoded).map_err(|error| EngineError::Failed {
            id: "pdf.stream.decode".to_string(),
            message: format!("the decoded bytes cannot be written to `{to}`: {error}"),
        })?;
        Ok(Outcome::new(format!("decoded {} bytes", decoded.len())).with_data(serde_json::json!({
            "id": id,
            "from": path,
            "to": to,
            "filters": names,
            "predictor": spec.map(|spec| format!("{:?}", spec.predictor)),
            "bytes": decoded.len(),
        })))
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
    DecodeStream,
    AddReference,
    ListReferences,
    GraphEvaluator,
    PdfPanel
);
