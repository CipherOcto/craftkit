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
//! crop box and the rotation — are found by walking up the tree. The
//! tree says itself in the **dictionary** as well as in the edges: a
//! page names its parent, the parent names its children, and the count
//! of the children is worked out from the edges and not kept.
//!
//! And the **rotation** of a page is here: `/Rotate` says by how many
//! degrees, and a positive number turns the page clockwise. The
//! degrees are a multiple of ninety — the tree's command truncates
//! what it is given, and here it is refused — and the number is taken
//! back into the circle, so a page turned four times is a page not
//! turned at all (the module `rotation`).
//!
//! And the **boxes** of a page are here: the media box, the crop box,
//! the bleed box, the trim box and the art box — a chain, where each
//! falls back to the one before it and each is cut back to the media
//! box (the module `boxes`).
//!
//! And the **annotation** is here: an annotation is an object of its
//! own — a kind, a rectangle, and somewhere to go — and the page names
//! it in its `/Annots`, which is an edge, the same edge the page tree
//! is made of.
//!
//! And an object may be **taken away**: the references that named it go
//! with it, because a file whose objects name things that are not there
//! is not a file (the module `remove`).
//!
//! And a **page** may be taken away: the tree of pages is right
//! afterwards, the count of the children is derived, and the
//! destinations that went to the page go with it (the module `pages`).
//!
//! And an object has a **number**, and one number is one object: two
//! objects may not share one, and a number that is changed takes every
//! reference to the object with it (the module `number`).
//!
//! And the **outline** is here: the bookmarks of the document, a tree of
//! items with titles and destinations. The tree is a **list** of
//! siblings in the file — each item names the next — and the counts are
//! **derived**: how many items are visible below an item, and the sign
//! saying whether it is open or closed.
//!
//! And the **free numbers** are here: a number that is not in use is
//! **free**, a number whose object is taken away is freed and is not
//! used again, and the file says so — the table has a row for the
//! number and the generation it gets if it is used again (the module
//! `free`).
//!
//! # The mapping
//!
//! | the object model | v2 |
//! |---|---|
//! | an indirect object | a **node**, of the kind `pdf.object` or `pdf.stream` |
//! | its dictionary | the node's **payload** |
//! | its stream | the node's **stream** — the bytes, beside the document |
//! | a reference to another object | a **relation** of the kind `ref` — and a **value** in the dictionary, because the file is written from the dictionary |
//! | the object number | the payload's `object`, because the ids are the core's |
//! | an outline item | an object of its own, with a `/Title` and a `/Dest` |
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
//!   to keep for the diff to be readable. And because a JSON array is
//!   one thing, a dictionary *inside* a dictionary is a list of pairs
//!   under the name `entries` — the name the payload already uses for
//!   the dictionary of an object, so that there is one spelling for a
//!   dictionary and not two.
//! - **A reference may point forward.** A relation names two nodes that
//!   are *in the document*, so the graph is built in **two passes**: the
//!   objects first, and the references after. That is a plugin's business,
//!   and it is the price of the core's rule.
//!
//! And one that is not a finding but a fact: a PDF **name** and a PDF
//! **string** are *bytes*, and the payload is JSON. A name that is not
//! text is base64 in the payload, and the door is the SDK's.
//!
//! The **price** of the spellings is the same price: a PDF dictionary
//! with a key called `entries` whose value is a list of pairs is read
//! as a dictionary and written as one.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

mod aes;
mod annot;
mod annotappearance;
mod appearance;
mod attachments;
mod background;
mod boxes;
mod calculate;
mod caretadd;
mod checkbox;
mod combobox;
mod comments;
mod compare;
mod content;
mod convert;
mod crypt;
mod datefield;
mod decrypt;
mod destinations;
mod encrypt;
mod fdf;
mod fieldadd;
mod fields;
mod figures;
mod filters;
mod flatten;
mod fonts;
mod form;
mod format;
mod formdata;
mod free;
mod freetextadd;
mod headerfooter;
mod hidden;
mod image;
mod imagefield;
mod information;
mod inkadd;
mod intent;
mod javascript;
mod labels;
mod lineadd;
mod listbox;
mod loader;
mod look;
mod marked;
mod marks;
mod markup;
mod metadata;
mod metrics;
mod number;
mod objects;
mod optional;
mod outline;
mod pagefonts;
mod pages;
mod pagetext;
mod patterns;
mod polygonadd;
mod predictors;
mod preflight;
mod properties;
mod pushbutton;
mod radio;
mod redact;
mod redactmark;
mod region;
mod remove;
mod revision6;
mod rotation;
mod scripts;
mod sha2;
mod signaturefield;
mod signatures;
mod structure;
mod tabs;
mod tounicode;
mod trailer;
mod transitions;
mod validation;
mod watermark;
mod widths;
mod writer;
mod xref;

use craftkit_derive::{Command, Evaluator, Layer, NodeKind, Panel};
use craftkit_sdk::serde_json;
use craftkit_sdk::{Document, EngineError, Node, NodeRef, Number, Outcome, Plan, PlanItem, Session, Text};

pub use annot::SetAnnotationFlags;
pub use annotappearance::WriteAnnotation;
pub use appearance::{ReadAppearance, WriteAppearance};
pub use attachments::ReadAttachments;
pub use background::AddBackground;
pub use boxes::{PageBoxes, SetPageBox};
pub use calculate::{ReadCalculatedValue, ReadCalculation};
pub use caretadd::AddCaret;
pub use checkbox::AddCheckBox;
pub use combobox::AddCombo;
pub use comments::{AddComment, CommentInfo, ReadComments};
pub use compare::{CompareFiles, MarkDifferences};
pub use convert::Convert;
pub use datefield::AddDate;
pub use destinations::ReadDestinations;
pub use encrypt::ReadEncryption;
pub use fieldadd::AddField;
pub use fields::{ReadChildren, ReadFields, ReadInherited, ReadWidgets, SetField, SetFlags, SetParent};
pub use figures::{ReadFigures, SetFigureAlt};
pub use flatten::{FlattenComments, FlattenForm};
pub use fonts::ReadFonts;
pub use form::ReadForm;
pub use format::ReadFormat;
pub use formdata::{ExportFormData, ImportFormData};
pub use freetextadd::AddFreeText;
pub use headerfooter::{AddFooter, AddHeader};
pub use imagefield::AddImage;
pub use information::{ReadInformation, SetInformation};
pub use inkadd::AddInk;
pub use intent::WriteIntent;
pub use javascript::ReadScripts;
pub use labels::SetPageLabels;
pub use lineadd::AddLine;
pub use listbox::AddList;
pub use look::SetLook;
pub use marked::ReadMarked;
pub use marks::RemoveMarks;
pub use markup::AddMarkup;
pub use metadata::WriteMetadata;
pub use number::SetNumber;
pub use optional::ReadLayers;
pub use outline::{AddOutline, ReadOutline};
pub use pages::RemovePage;
pub use pagetext::ReadPageText;
pub use patterns::{FindPattern, ReadPatterns};
pub use polygonadd::AddPolygon;
pub use preflight::Preflight;
pub use properties::SetProperties;
pub use pushbutton::AddButton;
pub use radio::AddRadio;
pub use redactmark::AddRedactMark;
pub use remove::RemoveObject;
pub use rotation::{ReadRotation, RotatePage};
pub use signaturefield::AddSignature;
pub use signatures::ReadSignatures;
pub use structure::ReadStructure;
pub use tabs::{ReadTabOrder, SetTabs};
pub use tounicode::ReadFontUnicode;
pub use trailer::ReadTrailer;
pub use transitions::ReadTransition;
pub use validation::ReadValidation;
pub use watermark::AddWatermark;

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

/// The **information** of a document: the object the trailer names.
#[derive(NodeKind)]
#[node_kind(kind = "pdf.info", label = "PDF information", layer = "pdf", evaluates)]
pub struct InformationKind;

impl InformationKind {
    /// The information is not drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Whether a point is inside it: never.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What the information **means**: the object it names.
    fn evaluate(node: &Node, _document: &Document) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "object": node.data.get("object").cloned().unwrap_or(serde_json::Value::Null),
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
        if let Some(other) = number::holder_of(session, number, None) {
            return Err(EngineError::Failed {
                id: "pdf.object.add".to_string(),
                message: format!("the number `{number}` is the number of the object `{other}`"),
            });
        }
        // A number that is **free** is not used again: it was the
        // number of an object that was taken away, and the file still
        // says the number is free.
        if free::the_free(session.document()).iter().any(|entry| entry.number == number) {
            return Err(EngineError::Failed {
                id: "pdf.object.add".to_string(),
                message: format!("the number `{number}` is free, and a free number is not used again"),
            });
        }
        let data = serde_json::json!({ "object": number, "entries": entries });
        let name = format!("object {number}");
        let id = session.document_mut()?.add_node_with(name.clone(), "pdf.object", None, data);
        Ok(Outcome::changed(format!("added the `{name}`"))
            .with_data(serde_json::json!({ "id": id, "object": number })))
    }
}

/// **Set** a key of an object's dictionary.
///
/// The dictionary is the payload, and a payload that cannot be changed
/// is a payload that was wrong the moment it was written. The key is
/// **added** when it is not there and **changed** when it is, and the
/// order of the dictionary is kept: a key that is there keeps its
/// place, and a key that is new goes at the end.
///
/// A node that is **not an object** is refused, and so is an empty
/// key.
#[derive(Command)]
#[command(id = "pdf.entry.set", area = "pdf", layer = "pdf", icon = "pencil", mutates, undoable)]
pub struct SetEntry {
    /// The object.
    pub object: NodeRef,
    /// The key.
    pub key: Text,
    /// The value.
    pub value: serde_json::Value,
}

impl SetEntry {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "pdf.entry.set".to_string();
        let object = self.object.0;
        let Some(node) = session.document().node(object) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{object}`"),
            });
        };
        if node.kind != "pdf.object" && node.kind != "pdf.stream" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{object}` is a `{}` and not an object", node.kind),
            });
        }
        let key = self.key.0.trim();
        if key.is_empty() {
            return Err(EngineError::Failed { id: id.clone(), message: "a key is text".to_string() });
        }
        let mut entries = entries_of(&node.data);
        let value = self.value.clone();
        match entries.iter_mut().find(|(name, _)| name == key) {
            Some(entry) => entry.1 = value,
            None => entries.push((key.to_string(), value)),
        }
        let data = serde_json::json!({
            "object": node.data.get("object").cloned().unwrap_or(serde_json::Value::Null),
            "entries": entries
                .iter()
                .map(|(key, value)| serde_json::json!([key, value]))
                .collect::<Vec<_>>(),
        });
        let Some(node) = session.document_mut()?.node_mut(object) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{object}`"),
            });
        };
        node.data = data;
        Ok(Outcome::changed(format!("the object `{object}` has the key `{key}`"))
            .with_data(serde_json::json!({ "id": object, "key": key })))
    }
}

/// **Take away** a key of an object's dictionary.
///
/// A key that is **not there** is refused: a key that was never there
/// is not a key that was taken away.
#[derive(Command)]
#[command(id = "pdf.entry.remove", area = "pdf", layer = "pdf", icon = "minus", mutates, undoable)]
pub struct RemoveEntry {
    /// The object.
    pub object: NodeRef,
    /// The key.
    pub key: Text,
}

impl RemoveEntry {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "pdf.entry.remove".to_string();
        let object = self.object.0;
        let Some(node) = session.document().node(object) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{object}`"),
            });
        };
        if node.kind != "pdf.object" && node.kind != "pdf.stream" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{object}` is a `{}` and not an object", node.kind),
            });
        }
        let key = self.key.0.trim();
        let mut entries = entries_of(&node.data);
        let before = entries.len();
        entries.retain(|(name, _)| name != key);
        if entries.len() == before {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the object `{object}` has no key `{key}`"),
            });
        }
        let data = serde_json::json!({
            "object": node.data.get("object").cloned().unwrap_or(serde_json::Value::Null),
            "entries": entries
                .iter()
                .map(|(key, value)| serde_json::json!([key, value]))
                .collect::<Vec<_>>(),
        });
        let Some(node) = session.document_mut()?.node_mut(object) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{object}`"),
            });
        };
        node.data = data;
        Ok(Outcome::changed(format!("the object `{object}` has no key `{key}`"))
            .with_data(serde_json::json!({ "id": object, "key": key })))
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
        if let Some(other) = number::holder_of(session, number, None) {
            return Err(EngineError::Failed {
                id: "pdf.stream.add".to_string(),
                message: format!("the number `{number}` is the number of the object `{other}`"),
            });
        }
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

/// Write the **content** of a stream back.
///
/// The content of a stream is read and written again in the kit's own
/// form: the operands, each one followed by a space, and then the
/// operator and a newline. The **inline images** are kept whole, so a
/// stream that is read and written again is the stream it was.
///
/// A stream that is **filtered** is refused: its bytes are not the
/// content, and writing them back would change what the file says.
#[derive(Command)]
#[command(id = "pdf.content.write", area = "pdf", layer = "pdf", icon = "file", mutates, undoable)]
pub struct ContentWrite {
    /// The stream whose content is written.
    pub stream: NodeRef,
}

impl ContentWrite {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = self.stream.0;
        let Some(node) = session.document().node(id) else {
            return Err(EngineError::Failed {
                id: "pdf.content.write".to_string(),
                message: format!("there is no node `{id}`"),
            });
        };
        if node.kind != "pdf.stream" {
            return Err(EngineError::Failed {
                id: "pdf.content.write".to_string(),
                message: format!("the node `{id}` is a `{}` and not a stream", node.kind),
            });
        }
        // A **filtered** stream's bytes are not the content.
        if let Some(names) = filters::filter_of(&entries_of(&node.data)) {
            return Err(EngineError::Failed {
                id: "pdf.content.write".to_string(),
                message: format!("the stream is filtered by {names:?}, and its bytes are not the content"),
            });
        }
        let Some(path) = node.stream.clone() else {
            return Err(EngineError::Failed {
                id: "pdf.content.write".to_string(),
                message: format!("the stream `{id}` has no file"),
            });
        };
        let limits = filters::Limits::default();
        let bytes = craftkit_sdk::files::read_bounded(std::path::Path::new(&path), limits.max_input as u64)
            .map_err(|message| EngineError::Failed {
            id: "pdf.content.write".to_string(),
            message: format!("the stream's bytes cannot be read: {message}"),
        })?;
        let operations = content::operations(&bytes).map_err(|error| EngineError::Failed {
            id: "pdf.content.write".to_string(),
            message: format!("the content cannot be read: {error:?}"),
        })?;
        let written = content::write_operations(&operations);
        std::fs::write(&path, &written).map_err(|error| EngineError::Failed {
            id: "pdf.content.write".to_string(),
            message: format!("the content cannot be written: {error}"),
        })?;
        Ok(Outcome::changed(format!("{} operation(s) written", operations.len())).with_data(
            serde_json::json!({
                "id": id,
                "operations": operations.len(),
                "bytes": written.len(),
            }),
        ))
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
            // And the **dictionary** says it too: the file is written
            // from the dictionary, and a tree that is only in the
            // document is a tree the file does not have.
            let to_parent = a_reference(session, parent, 0, "pdf.page.add")?;
            add_reference(session, id, "Parent", to_parent, false)?;
            let to_page = a_reference(session, id, 0, "pdf.page.add")?;
            add_reference(session, parent, "Kids", to_page, true)?;
            // The **count** of the children is derived: it is how many
            // of them there are.
            let kids = session
                .document()
                .relations
                .iter()
                .filter(|relation| {
                    relation.kind == "ref"
                        && relation.from == parent
                        && relation.payload.get("key").and_then(|key| key.as_str()) == Some("Kids")
                })
                .count();
            if let Some(node) = session.document_mut()?.node_mut(parent) {
                node.data = with_entry(&node.data, "Count", serde_json::json!(kids));
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

/// The smallest an **annotation** may be.
///
/// The tree's editor refuses a link whose area is less than two
/// points, because a link that cannot be clicked is not a link. The
/// bound is the tree's own, and it is here for the same reason: a
/// thing that cannot be picked is not there.
const MINIMUM_ANNOTATION: f64 = 2.0;

/// Put an **annotation** on a page.
///
/// An annotation is an object of its own: a dictionary with a type, a
/// kind, a rectangle, and — for a link — somewhere to go. The page
/// names it in its `/Annots`, and that naming is an **edge**, the same
/// edge the page tree is made of.
///
/// The kind is a **name** and not a closed list: the tree's editor has
/// a list of the annotations it draws, and keeps the others as they
/// are. Here nothing matches on the kind at all — the annotation says
/// what it is, and a third party may add a kind of its own.
///
/// Where a link goes is either a **page of this document** — the
/// `/Dest` — or a **web address** — the `/A` action, which is an
/// object of its own, so that the action may be anything at all.
#[derive(Command)]
#[command(id = "pdf.annot.add", area = "pdf", layer = "pdf", icon = "plus", mutates, undoable)]
pub struct AddAnnotation {
    /// The page the annotation is on.
    pub page: NodeRef,
    /// The annotation's object number in the file.
    pub object: Number,
    /// The annotation's kind: `Link`, `Text`, `FreeText`, `Square`,
    /// `Circle`, `Highlight`, `Popup`, `Widget` or `Stamp`.
    pub subtype: Text,
    /// The rectangle the annotation covers: four numbers.
    pub rect: serde_json::Value,
    /// The annotation's text.
    pub contents: Option<Text>,
    /// The page a link goes to.
    pub destination: Option<NodeRef>,
    /// Where a link goes that is not in this document.
    pub uri: Option<Text>,
}

impl AddAnnotation {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "pdf.annot.add".to_string();
        // The **page**: an object whose type is `Page`.
        let page = session.document().node(self.page.0).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("there is no node `{}`", self.page.0),
        })?;
        if page.kind != "pdf.object" && page.kind != "pdf.stream" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{}` is a `{}` and not an object", self.page.0, page.kind),
            });
        }
        let kind = entries_of(&page.data)
            .into_iter()
            .find(|(key, _)| key == "Type")
            .and_then(|(_, value)| value.as_str().map(|kind| kind.to_string()));
        if kind.as_deref() != Some("Page") {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{}` is not a page", self.page.0),
            });
        }
        let page_number = the_number_of(page).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("the node `{}` has no object number", self.page.0),
        })?;
        // The page's own **annotations**, before the new one is made.
        let annots = entries_of(&page.data)
            .into_iter()
            .find(|(key, _)| key == "Annots")
            .and_then(|(_, value)| value.as_array().cloned())
            .unwrap_or_default();
        // The **object number** of the annotation: a whole number, and
        // one that nothing else has.
        let object = self.object.0;
        if !object.is_finite() || object < 0.0 || object.fract() != 0.0 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "an object number is a whole number".to_string(),
            });
        }
        let number = object as u64;
        if number == 0 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "object numbers start at one".to_string(),
            });
        }
        if session.document().walk().any(|node| the_number_of(node) == Some(number)) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the object {number} is already in the document"),
            });
        }
        // The **kind**: a name.
        let subtype = a_name(&self.subtype.0, &id)?;
        // The **rectangle**: four numbers, and the corners may come in
        // any order — the tree puts them the right way round.
        let rect = the_rect(&self.rect, &id)?;
        // Where it goes: one place, and one place only.
        if self.destination.is_some() && self.uri.is_some() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "an annotation goes to one place".to_string(),
            });
        }
        let mut entries: Vec<(String, serde_json::Value)> = vec![
            ("Type".to_string(), serde_json::json!("Annot")),
            ("Subtype".to_string(), serde_json::json!(subtype)),
            ("Rect".to_string(), serde_json::json!(rect)),
        ];
        if let Some(contents) = &self.contents {
            entries.push(("Contents".to_string(), serde_json::json!({ "string": contents.0 })));
        }
        // The **page** the annotation is on, and the **print** flag: the
        // tree's editor prints its links, and so does this one.
        entries.push(("P".to_string(), serde_json::json!({ "ref": [page_number, 0] })));
        entries.push(("F".to_string(), serde_json::json!(4)));
        // Where it goes.
        let mut destination = None;
        let mut action = None;
        if let Some(target) = self.destination {
            let node = session.document().node(target.0).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{}`", target.0),
            })?;
            let target_number = the_number_of(node).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{}` has no object number", target.0),
            })?;
            // The **destination**: the page, and the way it is shown.
            entries.push(("Dest".to_string(), serde_json::json!([{ "ref": [target_number, 0] }, "Fit"])));
            destination = Some(target.0);
        }
        if let Some(uri) = &self.uri {
            let uri = uri.0.trim().to_string();
            if uri.is_empty() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a web address is not empty".to_string(),
                });
            }
            if !uri.is_ascii() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a web address is 7-bit ASCII".to_string(),
                });
            }
            // The **action** is an object of its own: the tree's editor
            // writes it inside the annotation, and the tree's action is
            // a closed list of three — a page, a web address, and the
            // things it does not edit. Here the action is an object, and
            // the annotation refers to it, so that a third party may
            // make one of its own.
            let action_number = the_next_number(session.document(), number);
            let action_entries: Vec<(String, serde_json::Value)> = vec![
                ("S".to_string(), serde_json::json!("URI")),
                ("URI".to_string(), serde_json::json!({ "string": uri })),
            ];
            let action_data = serde_json::json!({
                "object": action_number,
                "entries": pairs_of(&action_entries),
            });
            let action_name = format!("action {action_number}");
            let action_id =
                session.document_mut()?.add_node_with(action_name, "pdf.object", None, action_data);
            entries.push(("A".to_string(), serde_json::json!({ "ref": [action_number, 0] })));
            action = Some(action_id);
        }
        // The **annotation** itself.
        let data = serde_json::json!({ "object": number, "entries": pairs_of(&entries) });
        let name = format!("annotation {number}");
        let annot = session.document_mut()?.add_node_with(name.clone(), "pdf.object", None, data);
        // The page **names** the annotation: the `/Annots` is an array
        // of the annotations, and the array grows.
        let mut annots = annots;
        annots.push(serde_json::json!({ "ref": [number, 0] }));
        if let Some(node) = session.document_mut()?.node_mut(self.page.0) {
            node.data = with_entry(&node.data, "Annots", serde_json::json!(annots));
        }
        // And the **edges**: the page names the annotation, and the
        // annotation names where it goes.
        let edges: Vec<(u64, u64, &str)> = vec![
            (self.page.0, annot, "Annots"),
            (annot, destination.unwrap_or(annot), "Dest"),
            (annot, action.unwrap_or(annot), "A"),
        ];
        for (from, to, key) in edges {
            if from == to && key != "Dest" {
                continue;
            }
            if key == "Dest" && destination.is_none() {
                continue;
            }
            if key == "A" && action.is_none() {
                continue;
            }
            let payload = serde_json::json!({ "key": key, "generation": 0 });
            let relation = craftkit_sdk::Relation::with_payload(from, to, "ref", payload);
            if !session.document_mut()?.add_relation(relation) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the object `{from}` already refers to `{to}` under `{key}`"),
                });
            }
        }
        Ok(Outcome::changed(format!("added the `{name}` to the page")).with_data(serde_json::json!({
            "id": annot,
            "object": number,
            "page": self.page.0,
            "subtype": subtype,
            "rect": rect,
            "destination": destination,
            "action": action,
        })))
    }
}

/// The **annotations** of a page.
///
/// A query: it does not change the document. The page's `/Annots` are
/// the edges the page holds, and each one is an object of its own.
#[derive(Command)]
#[command(id = "pdf.page.annotations", area = "pdf", layer = "pdf", icon = "list")]
pub struct PageAnnotations {
    /// The page.
    pub page: NodeRef,
}

impl PageAnnotations {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "pdf.page.annotations".to_string();
        if session.document().node(self.page.0).is_none() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{}`", self.page.0),
            });
        }
        let mut annotations: Vec<serde_json::Value> = Vec::new();
        for relation in session.document().relations.iter().filter(|relation| {
            relation.kind == "ref"
                && relation.from == self.page.0
                && relation
                    .payload
                    .get("key")
                    .and_then(|key| key.as_str())
                    .map(|key| key == "Annots")
                    .unwrap_or(false)
        }) {
            let Some(node) = session.document().node(relation.to) else { continue };
            let entries = entries_of(&node.data);
            let mut annotation = serde_json::Map::new();
            annotation.insert("id".to_string(), serde_json::json!(node.id));
            annotation
                .insert("object".to_string(), serde_json::json!(the_number_of(node).unwrap_or_default()));
            for key in ["Subtype", "Rect", "Contents"] {
                if let Some((_, value)) = entries.iter().find(|(name, _)| name == key) {
                    annotation.insert(key.to_string(), value.clone());
                }
            }
            // The **flags**: whether the annotation prints, and whether
            // it is hidden.
            annotation.insert("flags".to_string(), serde_json::json!(crate::annot::flags_of(node)));
            annotation.insert("print".to_string(), serde_json::json!(crate::annot::prints(node)));
            annotation.insert("hidden".to_string(), serde_json::json!(crate::annot::is_hidden(node)));
            if let Some(where_) = where_it_goes(session.document(), &entries) {
                annotation.insert("goes_to".to_string(), where_);
            }
            annotations.push(serde_json::Value::Object(annotation));
        }
        let count = annotations.len();
        Ok(Outcome::new(format!("{count} annotation(s)"))
            .with_data(serde_json::json!({ "id": self.page.0, "annotations": annotations })))
    }
}

/// Where an annotation **goes**: a page of this document, or a web
/// address.
fn where_it_goes(document: &Document, entries: &[(String, serde_json::Value)]) -> Option<serde_json::Value> {
    // The **action** first: it is the one that is used when both are
    // there, and the tree says so.
    if let Some((_, action)) = entries.iter().find(|(key, _)| key == "A") {
        let number = number_of(action)?;
        let node = document.walk().find(|node| the_number_of(node) == Some(number))?;
        let uri =
            entries_of(&node.data).into_iter().find(|(key, _)| key == "URI").and_then(|(_, value)| {
                value.get("string").and_then(|text| text.as_str()).map(|text| text.to_string())
            })?;
        return Some(serde_json::json!({ "uri": uri }));
    }
    let (_, dest) = entries.iter().find(|(key, _)| key == "Dest")?;
    let number = number_of(dest.as_array().and_then(|list| list.first())?)?;
    Some(serde_json::json!({ "page": number }))
}

/// The value of a **reference**: the number of the object it names, and
/// its generation.
fn a_reference(
    session: &Session,
    to: u64,
    generation: u64,
    id: &str,
) -> Result<serde_json::Value, EngineError> {
    let node = session.document().node(to).ok_or_else(|| EngineError::Failed {
        id: id.to_string(),
        message: format!("there is no object `{to}` in the document"),
    })?;
    let number = the_number_of(node).ok_or_else(|| EngineError::Failed {
        id: id.to_string(),
        message: format!("the object `{to}` has no object number"),
    })?;
    Ok(serde_json::json!({ "ref": [number, generation] }))
}

/// Put a **reference** in an object's dictionary, under a key.
///
/// A key that holds a **list** of references grows, and a key that
/// holds one reference is set.
///
/// The reference is two things at once: an edge in the document and a
/// value in the dictionary — and the file is written from the
/// dictionary, so a reference that is only an edge is a reference the
/// file does not have.
fn add_reference(
    session: &mut Session,
    from: u64,
    key: &str,
    value: serde_json::Value,
    list: bool,
) -> Result<(), EngineError> {
    let current = session.document().node(from).and_then(|node| {
        entries_of(&node.data).into_iter().find(|(name, _)| name == key).map(|(_, value)| value)
    });
    let value = match (list, current) {
        (true, Some(serde_json::Value::Array(mut list))) => {
            list.push(value);
            serde_json::Value::Array(list)
        }
        (true, _) => serde_json::json!([value]),
        (false, _) => value,
    };
    if let Some(node) = session.document_mut()?.node_mut(from) {
        node.data = with_entry(&node.data, key, value);
    }
    Ok(())
}

/// The **object number** of a node, if it has one.
/// The **whole number** a value holds.
///
/// A file's numbers are numbers, and a whole number may be written as
/// a fraction — the loader keeps the number as the file wrote it, and
/// a number in the bytes is a number and not an integer until
/// somebody says so. So a whole number is read as a number and taken
/// as the whole one it is, and a number that is not a whole one is
/// not one.
pub fn whole_number_of(value: &serde_json::Value) -> Option<i64> {
    let number = value.as_f64()?;
    if !number.is_finite() || number.fract() != 0.0 {
        return None;
    }
    Some(number as i64)
}

/// The **object number** a reference names.
///
/// The number in a reference is a number, and a file's object number
/// is a whole one — but a file may write it as a fraction, and the
/// loader keeps the number as the file wrote it. So the number is
/// read as a number and taken as a whole one, and a number that is
/// not a whole one names no object.
pub fn number_of(value: &serde_json::Value) -> Option<u64> {
    let number = value.get("ref")?.as_array()?.first()?.as_f64()?;
    if !number.is_finite() || number < 0.0 || number.fract() != 0.0 {
        return None;
    }
    Some(number as u64)
}

fn the_number_of(node: &Node) -> Option<u64> {
    node.data.get("object").and_then(|number| number.as_u64())
}

/// The next **free** object number.
///
/// A number nothing else has: the file says which object an action is,
/// and the file's objects are numbered. A number that is **free** is a
/// number something had, so the next number is past it.
fn the_next_number(document: &Document, after: u64) -> u64 {
    free::the_next_number(document, after)
}

/// A **name**, from a text a caller sent.
///
/// A name may not be empty, and may not hold the characters a name may
/// not hold: the spaces, and the delimiters that end a name.
fn a_name(text: &str, id: &str) -> Result<String, EngineError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(EngineError::Failed { id: id.to_string(), message: "a name is not empty".to_string() });
    }
    if trimmed.chars().any(|character| character.is_whitespace() || "()<>[]{}/%".contains(character)) {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("`{trimmed}` is not a name: a name may not hold a space or a delimiter"),
        });
    }
    Ok(trimmed.to_string())
}

/// The **rectangle** of an annotation, from a value a caller sent.
///
/// Four numbers, and the corners may come in any order: the tree puts
/// them the right way round, and so does this one.
fn the_rect(value: &serde_json::Value, id: &str) -> Result<[f64; 4], EngineError> {
    let Some(list) = value.as_array() else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: "a rectangle is four numbers".to_string(),
        });
    };
    if list.len() != 4 {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("a rectangle is four numbers, and this one is {}", list.len()),
        });
    }
    let mut numbers: Vec<f64> = Vec::with_capacity(4);
    for item in list {
        let Some(number) = item.as_f64() else {
            return Err(EngineError::Failed {
                id: id.to_string(),
                message: "a rectangle is four numbers".to_string(),
            });
        };
        if !number.is_finite() {
            return Err(EngineError::Failed {
                id: id.to_string(),
                message: "a rectangle is four numbers, and one of them is not".to_string(),
            });
        }
        numbers.push(number);
    }
    let rect = [
        numbers[0].min(numbers[2]),
        numbers[1].min(numbers[3]),
        numbers[0].max(numbers[2]),
        numbers[1].max(numbers[3]),
    ];
    if rect[2] - rect[0] < MINIMUM_ANNOTATION || rect[3] - rect[1] < MINIMUM_ANNOTATION {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!(
                "the annotation is too small to be picked: it is less than {MINIMUM_ANNOTATION} points across"
            ),
        });
    }
    Ok(rect)
}

/// The **pairs** of a dictionary, as the payload holds them.
fn pairs_of(entries: &[(String, serde_json::Value)]) -> serde_json::Value {
    serde_json::Value::Array(entries.iter().map(|(key, value)| serde_json::json!([key, value])).collect())
}

/// A node's payload with an **entry** set.
///
/// The entry is put where it was, and at the end if it was not there.
fn with_entry(data: &serde_json::Value, key: &str, value: serde_json::Value) -> serde_json::Value {
    let mut entries = entries_of(data);
    match entries.iter_mut().find(|(name, _)| name == key) {
        Some(entry) => entry.1 = value,
        None => entries.push((key.to_string(), value)),
    }
    serde_json::json!({ "object": data.get("object").cloned().unwrap_or(serde_json::Value::Null), "entries": pairs_of(&entries) })
}

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
        let count = entries
            .iter()
            .find(|(key, _)| key == "N")
            .and_then(|(_, value)| whole_number_of(value))
            .and_then(|number| u64::try_from(number).ok())
            .ok_or_else(|| EngineError::Failed {
                id: "pdf.objstm.read".to_string(),
                message: "the object stream does not say how many objects it has".to_string(),
            })?;
        let first = entries
            .iter()
            .find(|(key, _)| key == "First")
            .and_then(|(_, value)| whole_number_of(value))
            .and_then(|number| u64::try_from(number).ok())
            .ok_or_else(|| EngineError::Failed {
                id: "pdf.objstm.read".to_string(),
                message: "the object stream does not say where the first object begins".to_string(),
            })?;
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
        let file = loader::load(&bytes).map_err(|error| EngineError::Failed {
            id: "pdf.file.load".to_string(),
            message: format!("the file cannot be loaded: {error:?}"),
        })?;
        let objects = file.objects;
        // The **trailer**: the file's own last word, kept in the
        // document's payload so that the document still says what the
        // file said.
        let trailer: Vec<serde_json::Value> =
            file.trailer.iter().map(|(key, value)| serde_json::json!([key, value])).collect();
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
            let data = serde_json::json!({ "object": object.number, "generation": object.generation, "entries": entries });
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
        // The **trailer** of the file goes into the document's own
        // payload: it is the file's last word and not an object of
        // the file.
        if !trailer.is_empty() {
            let document = session.document_mut()?;
            document.data["pdf.trailer"] = serde_json::json!({ "entries": trailer });
        }
        // The **free numbers** of the file: the numbers the file says
        // are not in use. They are kept, because a number that is free
        // is a number something had.
        let free = file.free;
        if !free.is_empty() {
            let document = session.document_mut()?;
            free::set_the_free(document, &free);
        }
        Ok(Outcome::changed(format!("loaded {count} object(s)")).with_data(serde_json::json!({
            "path": path,
            "into": into,
            "objects": count,
            "relations": relations,
            "free": free.len(),
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
        // The **information** of the file: the object the trailer
        // names, when the document has one.
        let info = session
            .document()
            .walk()
            .find(|node| node.kind == "pdf.info")
            .and_then(|node| node.data.get("object").and_then(|object| object.as_u64()))
            .and_then(|object| session.document().node(object))
            .and_then(|node| node.data.get("object").and_then(|number| number.as_u64()));
        let free = free::the_free(session.document());
        let trailer = writer::Trailer { root, info, encrypt: None, identifier: None };
        let file = writer::write(&objects, &free, &trailer).map_err(|error| EngineError::Failed {
            id: "pdf.file.write".to_string(),
            message: format!("the file cannot be written: {error:?}"),
        })?;
        std::fs::write(&path, &file).map_err(|error| EngineError::Failed {
            id: "pdf.file.write".to_string(),
            message: format!("the file cannot be written to `{path}`: {error}"),
        })?;
        Ok(Outcome::changed(format!("wrote {} object(s)", objects.len())).with_data(serde_json::json!({
            "path": path,
            "root": root,
            "bytes": file.len(),
            "free": free.len(),
        })))
    }
}

/// The **free numbers** of the file.
///
/// A number that is not in use is **free**, and a free number is not
/// used again: it was the number of an object that was taken away,
/// and the file still says the number is free.
///
/// The **next number** is past them all, because a free number is a
/// number something had.
#[derive(Command)]
#[command(id = "pdf.file.free", area = "pdf", layer = "pdf", icon = "list")]
pub struct ReadFree {
    /// The number to start from: the next number is past this one as
    /// well as past the free ones. Leave it out and the next number
    /// is past the file's own numbers.
    pub after: Option<Number>,
}

impl ReadFree {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "pdf.file.free".to_string();
        let mut after = 0_u64;
        if let Some(number) = &self.after {
            if !number.0.is_finite() || number.0 < 0.0 || number.0.fract() != 0.0 {
                return Err(EngineError::Failed {
                    id,
                    message: "an object number is a whole number".to_string(),
                });
            }
            after = number.0 as u64;
        }
        let free = free::the_free(session.document());
        let numbers: Vec<serde_json::Value> = free
            .iter()
            .map(|entry| serde_json::json!({ "object": entry.number, "generation": entry.generation }))
            .collect();
        let next = free::the_next_number(session.document(), after);
        Ok(Outcome::new(format!("the file has {} free number(s)", free.len()))
            .with_data(serde_json::json!({ "free": numbers, "count": free.len(), "next": next })))
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
    /// Whether the key holds a **list** of references that grows, rather
    /// than one reference that is set.
    ///
    /// A page tree's `/Kids` is a list and a page's `/Parent` is not.
    pub list: Option<craftkit_sdk::Bool>,
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
        // The reference is in the **dictionary** as well: the file is
        // written from the dictionary, and an edge that is only in the
        // document is an edge the file does not have.
        let value = a_reference(session, to, generation, "pdf.ref.add")?;
        let list = matches!(self.list, Some(craftkit_sdk::Bool(true)));
        add_reference(session, from, &key, value, list)?;
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
#[panel(
    id = "panel.pdf",
    label = "PDF objects",
    layer = "pdf",
    commands(
        "pdf.object.add",
        "pdf.object.remove",
        "pdf.object.number",
        "pdf.ref.add",
        "pdf.annot.add",
        "pdf.page.annotations",
        "pdf.outline.add",
        "pdf.outline.read"
    )
)]
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
    InformationKind,
    AddObject,
    AddField,
    AddCheckBox,
    AddRadio,
    AddButton,
    AddCombo,
    AddList,
    AddDate,
    AddSignature,
    AddImage,
    AddLine,
    AddPolygon,
    AddInk,
    AddCaret,
    AddFreeText,
    AddRedactMark,
    SetProperties,
    SetLook,
    WriteAnnotation,
    Preflight,
    ReadFonts,
    ReadTrailer,
    Convert,
    ReadFields,
    CompareFiles,
    MarkDifferences,
    AddMarkup,
    AddWatermark,
    AddBackground,
    RemoveMarks,
    ReadComments,
    AddComment,
    CommentInfo,
    FlattenComments,
    FlattenForm,
    AddHeader,
    AddFooter,
    ReadInherited,
    ReadChildren,
    ReadWidgets,
    ReadTabOrder,
    SetTabs,
    WriteMetadata,
    WriteIntent,
    SetAnnotationFlags,
    ReadEncryption,
    ReadCalculation,
    ReadCalculatedValue,
    ReadForm,
    ReadAppearance,
    ReadValidation,
    ReadFormat,
    SetField,
    SetFlags,
    SetParent,
    AddStream,
    SetInformation,
    ReadInformation,
    ReadDestinations,
    ReadSignatures,
    ReadStructure,
    ReadPageText,
    ReadAttachments,
    ReadLayers,
    WriteAppearance,
    ReadMarked,
    ReadTransition,
    ReadScripts,
    ReadFontUnicode,
    SetPageLabels,
    RemoveObject,
    SetNumber,
    SetEntry,
    RemoveEntry,
    AddPage,
    RemovePage,
    PageAttributes,
    DecodeStream,
    ContentOperators,
    ReadObjectStream,
    ReadXref,
    LoadFile,
    WriteFile,
    AddReference,
    ListReferences,
    AddOutline,
    ReadOutline,
    GraphEvaluator,
    PdfPanel
);
