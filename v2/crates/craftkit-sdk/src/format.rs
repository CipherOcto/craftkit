//! The file format: versions and migrations, as contributions.
//!
//! # The problem this solves
//!
//! The tree v2 replaces changes its format by adding **one file per
//! change**, and the change has to be wired in four places: the file, the
//! table of pending changes, the `mod` declaration and the match that says
//! which step applies. The ceremony is deliberate — it is what lets two
//! people change the format at once without a merge conflict — and it exists
//! because the format is **hand-written**.
//!
//! Here the format is a set of **contributions**:
//!
//! - a **format version** says what a document of that version looks like,
//!   and how to read and write it;
//! - a **migration** says how to move a document from one version to the
//!   next.
//!
//! So a change is one file and one derive, and the wiring is the list of
//! contributions — which the link fills.
//!
//! # The rule that matters
//!
//! **A file is read with the version it was written with, and then migrated
//! forward.** Never with the current version: a file written by an old
//! program has to be readable by a new one, and the only way to know that is
//! to carry the version and to have the steps that bridge it.
//!
//! # What is derived
//!
//! The version of a format is a fact about the **types**, not a number
//! somebody remembers to bump: a `#[derive(Format)]` writes the reader and
//! the writer, and a `#[derive(Migration)]` writes the step.

use serde_json::Value as Json;

use crate::contribution::{Contribution, Registry};
use crate::document::Document;

/// A value that can be written to a file and read back.
///
/// The model's types implement it, and a plugin's own types implement it too
/// — which is the point: a plugin that adds a kind of node has a body type,
/// and the body type has to be in the file.
pub trait FormatValue: Sized {
    /// The value as it is written.
    fn to_json(&self) -> Json;
    /// The value as it is read.
    fn from_json(value: &Json) -> Result<Self, String>;
    /// The value a field that may be **absent** takes when it is absent.
    ///
    /// A field is marked absent-tolerant by `#[format(default)]`, and the
    /// derive reads the file with this value in place of a missing one.
    ///
    /// It is `null` for most things and an **empty list** for a list,
    /// because a document that does not have a list of relations has no
    /// relations and a document that does not have a name has no name.
    fn default_value() -> Json {
        Json::Null
    }
}

impl FormatValue for String {
    fn to_json(&self) -> Json {
        Json::String(self.clone())
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        match value.as_str() {
            Some(text) => Ok(text.to_string()),
            None => Err("expected a string".to_string()),
        }
    }
}

impl FormatValue for f64 {
    fn to_json(&self) -> Json {
        Json::from(*self)
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        value.as_f64().ok_or_else(|| "expected a number".to_string())
    }
}

impl FormatValue for u64 {
    fn to_json(&self) -> Json {
        Json::from(*self)
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        value.as_u64().ok_or_else(|| "expected a whole number".to_string())
    }
}

impl FormatValue for bool {
    fn to_json(&self) -> Json {
        Json::Bool(*self)
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        value.as_bool().ok_or_else(|| "expected true or false".to_string())
    }
}

impl<T: FormatValue> FormatValue for Vec<T> {
    fn to_json(&self) -> Json {
        Json::Array(self.iter().map(|item| item.to_json()).collect())
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        let array = value.as_array().ok_or_else(|| "expected a list".to_string())?;
        array.iter().map(T::from_json).collect()
    }

    /// A list that is not there is an empty list.
    fn default_value() -> Json {
        Json::Array(Vec::new())
    }
}

impl<T: FormatValue> FormatValue for Option<T> {
    fn to_json(&self) -> Json {
        match self {
            Some(value) => value.to_json(),
            None => Json::Null,
        }
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        if value.is_null() {
            return Ok(None);
        }
        T::from_json(value).map(Some)
    }
}

/// A **JSON value** is its own format.
///
/// The payload of a node is a JSON value, and the document's payload is a
/// JSON value, and the file carries them as they are: the core does not
/// read them and does not write them in any other shape.
impl FormatValue for Json {
    fn to_json(&self) -> Json {
        self.clone()
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        Ok(value.clone())
    }
}

/// The value at a name, or the reason there is none.
///
/// The reader the derive macro writes needs this: a field that is not in the
/// file is an error, and the error names the field.
pub fn field<'a>(value: &'a Json, name: &str) -> Result<&'a Json, String> {
    match value.get(name) {
        Some(found) => Ok(found),
        None => Err(format!("there is no `{name}` in the value")),
    }
}

/// The fields of a value that the reader does **not** understand.
///
/// This is what a forward-compatible reader is: the fields it knows are
/// read, and the fields it does not know are **kept**, so that a file
/// written by a newer program does not come back smaller.
///
/// The **envelope** is not one of them: the format and the version belong
/// to the file and not to the document, so they are never kept as
/// content.
pub fn keep_unknown(value: &Json, known: &[&str]) -> Json {
    let Some(object) = value.as_object() else { return Json::Null };
    let mut kept = serde_json::Map::new();
    for (name, field) in object {
        if known.contains(&name.as_str()) || name == "format" || name == "version" {
            continue;
        }
        kept.insert(name.clone(), field.clone());
    }
    // Nothing kept is **nothing**: an empty object and a null are two
    // shapes for the same fact, and a document that went through a
    // program that keeps nothing must not look different from one that
    // did not.
    if kept.is_empty() {
        return Json::Null;
    }
    Json::Object(kept)
}

/// Write the fields a reader did not understand back into a value.
///
/// The fields the writer knows win: a kept field is by definition one the
/// writer does not know, and a file that has both is a file whose known
/// field is the truth.
pub fn merge_unknown(value: &mut Json, unknown: &Json) {
    let Some(kept) = unknown.as_object() else { return };
    let Some(object) = value.as_object_mut() else { return };
    for (name, field) in kept {
        if object.contains_key(name) {
            continue;
        }
        object.insert(name.clone(), field.clone());
    }
}

/// One version of the format.
///
/// The tag is what a file carries: `craftkit/2`. The version is what the
/// migrations count in.
#[derive(Clone, Copy)]
pub struct FormatSpec {
    /// The tag a file of this version carries.
    pub tag: &'static str,
    /// The version number.
    pub version: u32,
    /// The layer that owns it.
    pub layer: &'static str,
    /// How a document of this version is read.
    pub read: fn(&Json) -> Result<Document, String>,
    /// How a document of this version is written.
    pub write: fn(&Document) -> Json,
}

impl std::fmt::Debug for FormatSpec {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("FormatSpec").field("tag", &self.tag).field("version", &self.version).finish()
    }
}

impl Contribution for FormatSpec {
    fn id(&self) -> &'static str {
        self.tag
    }

    fn capability(&self) -> &'static str {
        "format"
    }

    fn layer(&self) -> &'static str {
        self.layer
    }

    /// A format's id is a tag, not an area and a thing.
    ///
    /// The tag is `<family>/<version>`, and the family is a word and the
    /// version is a number. It is not `<area>.<thing>`, because a format is
    /// not somewhere on a screen.
    fn malformed_id(id: &str) -> Option<String> {
        malformed_tag_id(id)
    }
}

/// One step: how a document moves from one version to the next.
#[derive(Clone, Copy)]
pub struct MigrationSpec {
    /// What the step is called.
    pub id: &'static str,
    /// The family it belongs to: the format family it steps through.
    pub family: &'static str,
    /// The version it reads.
    pub from: u32,
    /// The version it writes.
    pub to: u32,
    /// The layer that owns it.
    pub layer: &'static str,
    /// The step itself: it changes the value in place.
    pub apply: fn(&mut Json) -> Result<(), String>,
}

impl std::fmt::Debug for MigrationSpec {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MigrationSpec")
            .field("id", &self.id)
            .field("from", &self.from)
            .field("to", &self.to)
            .finish()
    }
}

impl Contribution for MigrationSpec {
    fn id(&self) -> &'static str {
        self.id
    }

    fn capability(&self) -> &'static str {
        "migration"
    }

    fn layer(&self) -> &'static str {
        self.layer
    }
}

// The link collects formats and migrations here.
inventory::collect!(crate::contribution::Registration<FormatSpec>);
inventory::collect!(crate::contribution::Registration<MigrationSpec>);

/// The family a tag belongs to: the part before the slash.
pub fn family_of(tag: &str) -> &str {
    match tag.split_once('/') {
        Some((family, _)) => family,
        None => tag,
    }
}

/// Whether a tag is well formed: `<family>/<version>`.
///
/// The family is a word in lower case and the version is a number.
pub fn malformed_tag_id(id: &str) -> Option<String> {
    let Some((family, version)) = id.split_once('/') else {
        return Some("a format's id is `<family>/<version>`".to_string());
    };
    if family.is_empty() || !family.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()) {
        return Some("a format's family is a word, in lower case".to_string());
    }
    if version.is_empty() || !version.chars().all(|c| c.is_ascii_digit()) {
        return Some("a format's version is a number".to_string());
    }
    None
}

/// Take the **payload** off every relation in a value.
///
/// A version that does not know about the payload of a relation must not
/// write it: the payload is the eighth version's field, and a file of the
/// seventh version that has one is a file of the seventh version with
/// something in it that the seventh version does not know.
fn without_payloads(value: &mut Json) {
    let Some(relations) = value.get_mut("relations").and_then(|relations| relations.as_array_mut()) else {
        return;
    };
    for relation in relations.iter_mut() {
        if let Some(object) = relation.as_object_mut() {
            object.remove("payload");
        }
    }
}

/// Take the **stream** off every node in a value.
///
/// A version that does not know about streams must not write them: the
/// stream is the seventh version's field, and a file of the sixth version
/// that has one is a file of the sixth version with something in it that
/// the sixth version does not know.
fn without_streams(value: &mut Json) {
    let Some(nodes) = value.get_mut("nodes").and_then(|nodes| nodes.as_array_mut()) else {
        return;
    };
    for node in nodes.iter_mut() {
        if let Some(object) = node.as_object_mut() {
            object.remove("stream");
        }
    }
}

/// Write a document in the first version: no `muted` flag on a node.
///
/// A node was either there or not, and there was nothing to say about it.
pub fn write_v1(document: &Document) -> Json {
    let mut value = document.to_json();
    if let Some(nodes) = value.get_mut("nodes").and_then(|nodes| nodes.as_array_mut()) {
        for node in nodes.iter_mut() {
            if let Some(object) = node.as_object_mut() {
                object.remove("muted");
                object.remove("children");
            }
        }
    }
    without_streams(&mut value);
    let mut written = serde_json::json!({
        "format": "craftkit/1",
        "version": 1,
        "name": value.get("name").cloned().unwrap_or(Json::Null),
        "nodes": value.get("nodes").cloned().unwrap_or(Json::Array(Vec::new())),
    });
    merge_unknown(&mut written, &document.unknown);
    written
}

/// Write a document in the second version: every node says whether it is
/// muted.
pub fn write_v2(document: &Document) -> Json {
    let mut value = serde_json::json!({
        "format": "craftkit/2",
        "version": 2,
        "name": document.name,
        "nodes": document.nodes.iter().map(|node| node.to_json()).collect::<Vec<_>>(),
    });
    if let Some(nodes) = value.get_mut("nodes").and_then(|nodes| nodes.as_array_mut()) {
        for node in nodes.iter_mut() {
            if let Some(object) = node.as_object_mut() {
                object.remove("children");
            }
        }
    }
    without_streams(&mut value);
    merge_unknown(&mut value, &document.unknown);
    value
}

/// Write a document in the third version: a node may hold other nodes.
///
/// The version before had a list and no groups. This one has a tree, and
/// the step below gives a file that does not have it the empty group each
/// node would have had.
pub fn write_v3(document: &Document) -> Json {
    let mut value = serde_json::json!({
        "format": "craftkit/3",
        "version": 3,
        "name": document.name,
        "nodes": document.nodes.iter().map(|node| node.to_json()).collect::<Vec<_>>(),
    });
    without_streams(&mut value);
    merge_unknown(&mut value, &document.unknown);
    value
}

/// The fourth version of the document format.
///
/// A node may have no place, and the document carries a payload.
pub fn write_v4(document: &Document) -> Json {
    let mut value = serde_json::json!({
        "format": "craftkit/4",
        "version": 4,
        "name": document.name,
        "data": document.data,
        "nodes": document.nodes.iter().map(|node| node.to_json()).collect::<Vec<_>>(),
    });
    without_streams(&mut value);
    merge_unknown(&mut value, &document.unknown);
    value
}

/// The fifth version of the document format.
///
/// The document grew **relations**: typed edges between the nodes, for
/// the things a tree cannot say — a clip that plays a source, a track
/// that feeds a bus, a track that is under a controller.
pub fn write_v5(document: &Document) -> Json {
    let mut value = serde_json::json!({
        "format": "craftkit/5",
        "version": 5,
        "name": document.name,
        "data": document.data,
        "nodes": document.nodes.iter().map(|node| node.to_json()).collect::<Vec<_>>(),
        "relations": document.relations.iter().map(|relation| relation.to_json()).collect::<Vec<_>>(),
    });
    without_streams(&mut value);
    without_payloads(&mut value);
    merge_unknown(&mut value, &document.unknown);
    value
}

/// The sixth version of the document format.
///
/// The document grew **assets**: the files it needs, by reference. A
/// document that points at audio, or at a font, or at a picture, now
/// says so in one place, so that a program can tell what is missing
/// before it starts editing.
pub fn write_v6(document: &Document) -> Json {
    let mut value = serde_json::json!({
        "format": "craftkit/6",
        "version": 6,
        "name": document.name,
        "data": document.data,
        "nodes": document.nodes.iter().map(|node| node.to_json()).collect::<Vec<_>>(),
        "relations": document.relations.iter().map(|relation| relation.to_json()).collect::<Vec<_>>(),
        "assets": document.assets.iter().map(|asset| asset.to_json()).collect::<Vec<_>>(),
    });
    without_streams(&mut value);
    without_payloads(&mut value);
    merge_unknown(&mut value, &document.unknown);
    value
}

/// The seventh version of the document format.
///
/// A node may have a **stream**: bytes that belong to the node and are
/// not JSON. The bytes travel beside the document, in a bundle, and the
/// node says which file is its stream.
///
/// This is the second payload field, and it is cheaper than it was
/// expected to be: the bundle already carried files, so the cost is one
/// field and one version, and **no base64 in the file at all**.
pub fn write_v7(document: &Document) -> Json {
    let mut value = serde_json::json!({
        "format": "craftkit/7",
        "version": 7,
        "name": document.name,
        "data": document.data,
        "nodes": document.nodes.iter().map(|node| node.to_json()).collect::<Vec<_>>(),
        "relations": document.relations.iter().map(|relation| relation.to_json()).collect::<Vec<_>>(),
        "assets": document.assets.iter().map(|asset| asset.to_json()).collect::<Vec<_>>(),
    });
    // The payload of a relation is the **eighth** version's field, and a
    // file of the seventh version must not carry a field the seventh
    // version does not know.
    without_payloads(&mut value);
    merge_unknown(&mut value, &document.unknown);
    value
}

/// The first version of the document format.
pub static FORMAT_V1: FormatSpec =
    FormatSpec { tag: "craftkit/1", version: 1, layer: "core", read: Document::from_json, write: write_v1 };

/// The second version of the document format.
///
/// It adds the `muted` flag to a node. The flag is written by every version
/// after the first, and the step below adds it to a file that does not have
/// it.
pub static FORMAT_V2: FormatSpec =
    FormatSpec { tag: "craftkit/2", version: 2, layer: "core", read: Document::from_json, write: write_v2 };

/// The third version of the document format.
///
/// It lets a node hold other nodes: the document is a tree, and a node
/// with children is a group.
pub static FORMAT_V3: FormatSpec =
    FormatSpec { tag: "craftkit/3", version: 3, layer: "core", read: Document::from_json, write: write_v3 };

inventory::submit! {
    crate::contribution::Registration { item: &FORMAT_V1 }
}

inventory::submit! {
    crate::contribution::Registration { item: &FORMAT_V2 }
}

inventory::submit! {
    crate::contribution::Registration { item: &FORMAT_V3 }
}

/// The fourth version of the document format.
///
/// Two things changed, and both of them were found by the port of a sound
/// application:
///
/// - a node may have **no place**. The first three versions said that
///   every node has a rectangle, and a node in a sound document is at a
///   time rather than at a point in a picture;
/// - the document carries a **payload** of its own, because a sound
///   session's sample rate and timecode belong to the session and not to
///   any one node.
pub static FORMAT_V4: FormatSpec =
    FormatSpec { tag: "craftkit/4", version: 4, layer: "core", read: Document::from_json, write: write_v4 };

inventory::submit! {
    crate::contribution::Registration { item: &FORMAT_V4 }
}

/// The fifth version of the document format.
///
/// It adds the relations between the nodes.
pub static FORMAT_V5: FormatSpec =
    FormatSpec { tag: "craftkit/5", version: 5, layer: "core", read: Document::from_json, write: write_v5 };

inventory::submit! {
    crate::contribution::Registration { item: &FORMAT_V5 }
}

/// The sixth version of the document format.
///
/// It adds the files the document needs.
pub static FORMAT_V6: FormatSpec =
    FormatSpec { tag: "craftkit/6", version: 6, layer: "core", read: Document::from_json, write: write_v6 };

inventory::submit! {
    crate::contribution::Registration { item: &FORMAT_V6 }
}

/// The step from the first version to the second.
///
/// It gives every node the flag the second version writes.
#[derive(crate::Migration)]
#[migration(id = "craftkit.1to2", family = "craftkit", from = 1, to = 2, layer = "core")]
pub struct V1ToV2;

impl V1ToV2 {
    /// The step itself.
    fn apply(value: &mut Json) -> Result<(), String> {
        let Some(nodes) = value.get_mut("nodes").and_then(|nodes| nodes.as_array_mut()) else {
            return Ok(());
        };
        for node in nodes.iter_mut() {
            if let Some(object) = node.as_object_mut() {
                object.entry("muted").or_insert(Json::Bool(false));
            }
        }
        Ok(())
    }
}

/// The step from the second version to the third.
///
/// It gives every node the children the third version writes. A file of
/// the second version is a list, and a list is a tree whose nodes have
/// nothing in them.
#[derive(crate::Migration)]
#[migration(id = "craftkit.2to3", family = "craftkit", from = 2, to = 3, layer = "core")]
pub struct V2ToV3;

impl V2ToV3 {
    /// The step itself.
    fn apply(value: &mut Json) -> Result<(), String> {
        let Some(nodes) = value.get_mut("nodes").and_then(|nodes| nodes.as_array_mut()) else {
            return Ok(());
        };
        for node in nodes.iter_mut() {
            if let Some(object) = node.as_object_mut() {
                object.entry("children").or_insert(Json::Array(Vec::new()));
            }
        }
        Ok(())
    }
}

/// The step from the third version to the fourth.
///
/// It says what the fourth version says: a node may have no place, and
/// the document carries a payload. A file of the third version has a place
/// on every node and no payload, so the step gives the document an empty
/// payload and leaves the places where they are.
#[derive(crate::Migration)]
#[migration(id = "craftkit.3to4", family = "craftkit", from = 3, to = 4, layer = "core")]
pub struct V3ToV4;

impl V3ToV4 {
    /// The step itself.
    fn apply(value: &mut Json) -> Result<(), String> {
        if let Some(object) = value.as_object_mut() {
            object.entry("data").or_insert(Json::Null);
        }
        Ok(())
    }
}

/// The step from the fourth version to the fifth.
///
/// It gives the document the relations the fifth version writes. A file
/// of the fourth version has none, and none is what it gets.
#[derive(crate::Migration)]
#[migration(id = "craftkit.4to5", family = "craftkit", from = 4, to = 5, layer = "core")]
pub struct V4ToV5;

impl V4ToV5 {
    /// The step itself.
    fn apply(value: &mut Json) -> Result<(), String> {
        if let Some(object) = value.as_object_mut() {
            object.entry("relations").or_insert(Json::Array(Vec::new()));
        }
        Ok(())
    }
}

/// The step from the fifth version to the sixth.
///
/// It gives the document the assets the sixth version writes. A file of
/// the fifth version names no files, and none is what it gets.
#[derive(crate::Migration)]
#[migration(id = "craftkit.5to6", family = "craftkit", from = 5, to = 6, layer = "core")]
pub struct V5ToV6;

impl V5ToV6 {
    /// The step itself.
    fn apply(value: &mut Json) -> Result<(), String> {
        if let Some(object) = value.as_object_mut() {
            object.entry("assets").or_insert(Json::Array(Vec::new()));
        }
        Ok(())
    }
}

/// The seventh version of the document format.
///
/// It adds the **stream** of a node: the file its bytes are in.
pub static FORMAT_V7: FormatSpec =
    FormatSpec { tag: "craftkit/7", version: 7, layer: "core", read: Document::from_json, write: write_v7 };

inventory::submit! {
    crate::contribution::Registration { item: &FORMAT_V7 }
}

/// The step from the sixth version to the seventh.
///
/// It gives every node the stream the seventh version writes. A file of
/// the sixth version has no stream on any node, and none is what it gets.
#[derive(crate::Migration)]
#[migration(id = "craftkit.6to7", family = "craftkit", from = 6, to = 7, layer = "core")]
pub struct V6ToV7;

impl V6ToV7 {
    /// The step itself.
    fn apply(value: &mut Json) -> Result<(), String> {
        let Some(nodes) = value.get_mut("nodes").and_then(|nodes| nodes.as_array_mut()) else {
            return Ok(());
        };
        for node in nodes.iter_mut() {
            if let Some(object) = node.as_object_mut() {
                object.entry("stream").or_insert(Json::Null);
            }
        }
        Ok(())
    }
}

/// The eighth version of the document format.
///
/// A relation carries a **payload**: what the edge says, besides its
/// kind. The port of a PDF application is what found the gap: a
/// reference says which key it is under, and two edges of the same kind
/// can say different things.
pub fn write_v8(document: &Document) -> Json {
    let mut value = serde_json::json!({
        "format": "craftkit/8",
        "version": 8,
        "name": document.name,
        "data": document.data,
        "nodes": document.nodes.iter().map(|node| node.to_json()).collect::<Vec<_>>(),
        "relations": document.relations.iter().map(|relation| relation.to_json()).collect::<Vec<_>>(),
        "assets": document.assets.iter().map(|asset| asset.to_json()).collect::<Vec<_>>(),
    });
    // The payload of a relation is the eighth version's field, and this
    // is the version that has it: it is written here, and the seventh
    // version's writer takes it off again.
    merge_unknown(&mut value, &document.unknown);
    value
}

/// The eighth version of the document format.
pub static FORMAT_V8: FormatSpec =
    FormatSpec { tag: "craftkit/8", version: 8, layer: "core", read: Document::from_json, write: write_v8 };

inventory::submit! {
    crate::contribution::Registration { item: &FORMAT_V8 }
}

/// The step from the seventh version to the eighth.
///
/// It gives every relation the payload the eighth version writes. A file
/// of the seventh version has none, and none is what it gets.
#[derive(crate::Migration)]
#[migration(id = "craftkit.7to8", family = "craftkit", from = 7, to = 8, layer = "core")]
pub struct V7ToV8;

impl V7ToV8 {
    /// The step itself.
    fn apply(value: &mut Json) -> Result<(), String> {
        let Some(relations) = value.get_mut("relations").and_then(|relations| relations.as_array_mut())
        else {
            return Ok(());
        };
        for relation in relations.iter_mut() {
            if let Some(object) = relation.as_object_mut() {
                object.entry("payload").or_insert(Json::Null);
            }
        }
        Ok(())
    }
}

/// The newest format of a family, if the program has one.
pub fn newest_format<'a>(formats: &'a Registry<FormatSpec>, family: &str) -> Option<&'a FormatSpec> {
    formats.all().iter().filter(|format| family_of(format.tag) == family).max_by_key(|format| format.version)
}

/// The document, as the newest format of a family writes it.
pub fn write_document(document: &Document, format: &FormatSpec) -> Json {
    (format.write)(document)
}

/// The document, in the **oldest** format of a family that can say
/// everything the document says.
///
/// The file is written at the lowest version that **loses nothing**, so
/// that a program which knows only that version can read it. That is the
/// rule the family's page layout application follows (§51): it writes
/// the oldest version that still represents the document.
///
/// How the version is chosen: the document is written at each version of
/// the family, from the oldest to the newest, and read back. The first
/// version whose round trip gives the same document back is the one.
/// Nothing is declared and nothing is configured — the versions are
/// tried, and the one that keeps everything wins.
///
/// The **bytes** a node's stream points at are not part of the
/// comparison: they travel beside the document in the bundle, and no
/// version of the document file can lose them.
///
/// `None` means that no version of the family round trips — which
/// should not happen, and which the caller may answer by writing at the
/// newest version instead.
pub fn write_document_oldest(
    document: &Document,
    formats: &Registry<FormatSpec>,
    migrations: &Registry<MigrationSpec>,
    family: &str,
) -> Option<(Json, u32)> {
    let mut candidates: Vec<&FormatSpec> =
        formats.all().iter().filter(|format| family_of(format.tag) == family).collect();
    candidates.sort_by_key(|format| format.version);
    for format in candidates {
        let written = write_document(document, format);
        let Ok(read_back) = read_document(&written, formats, migrations) else { continue };
        if same_document(document, &read_back) {
            return Some((written, format.version));
        }
    }
    None
}

/// Whether two documents say the same thing.
///
/// Everything is compared, the **payload of a relation** included: the
/// eighth version writes it and the seventh version does not, so a
/// document whose relations say something needs the eighth version, and
/// the comparison is what says so.
fn same_document(left: &Document, right: &Document) -> bool {
    left.to_json() == right.to_json()
}

/// Read a document, migrating it forward if it was written by an older
/// program.
///
/// The file carries the tag of the version it was written with, and the
/// steps that bridge it are contributions. A file whose version is the
/// newest one is read as it is; a file whose version is older is
/// **migrated first**; a file whose version is **newer** is read with the
/// newest reader this program has, and the fields that reader does not
/// understand are **kept** — see [`read_document_noting`].
pub fn read_document(
    value: &Json,
    formats: &Registry<FormatSpec>,
    migrations: &Registry<MigrationSpec>,
) -> Result<Document, String> {
    read_document_noting(value, formats, migrations).map(|(document, _)| document)
}

/// Read a document, and say what the reader had to say about it.
///
/// The second value is the **version the file was written with**, when
/// that version is newer than anything this program knows. A program that
/// reads a file from the future is doing something it should say out
/// loud: it read the fields it knows, and it kept the rest, but it cannot
/// know that a field it *does* know has not changed its meaning.
///
/// The steps that bridge the versions run **only** for a file that is
/// older than this program: a file from the future is not brought
/// forward, because bringing it forward would need steps this program
/// does not have.
pub fn read_document_noting(
    value: &Json,
    formats: &Registry<FormatSpec>,
    migrations: &Registry<MigrationSpec>,
) -> Result<(Document, Option<u32>), String> {
    let tag = value
        .get("format")
        .and_then(|tag| tag.as_str())
        .ok_or_else(|| "the file does not say which format it is".to_string())?;
    let family = family_of(tag);
    let Some(newest) = newest_format(formats, family) else {
        return Err(format!("this program does not know the format `{family}`"));
    };
    let known = formats.all().iter().find(|format| format.tag == tag).map(|format| format.version);
    let written = match known {
        Some(version) => version,
        // A tag this program has not seen. The number in the tag says how
        // far ahead the file is.
        None => {
            let version =
                version_of(tag).ok_or_else(|| format!("this program does not know the format `{tag}`"))?;
            if version <= newest.version {
                return Err(format!("this program does not know the format `{tag}`"));
            }
            version
        }
    };
    if written > newest.version {
        // A file from a newer program. It is read with the newest reader
        // here, and what that reader does not understand is kept.
        let document = (newest.read)(value)?;
        return Ok((document, Some(written)));
    }

    // The steps, in order. Each one is a version forward, and the ecosystem
    // has already refused a chain with a hole in it.
    let mut value = value.clone();
    let mut version = written;
    while version < newest.version {
        let step = migrations
            .all()
            .iter()
            .find(|step| step.family == family && step.from == version)
            .ok_or_else(|| format!("there is no step from version {version} of `{family}`"))?;
        (step.apply)(&mut value)?;
        version = step.to;
    }
    Ok(((newest.read)(&value)?, None))
}

/// The version a tag says, if the tag says one.
///
/// The tag is `<family>/<version>`, and the version is a number. A tag
/// that does not say one is not a tag of a format this program knows.
pub fn version_of(tag: &str) -> Option<u32> {
    let (_, version) = tag.split_once('/')?;
    version.parse::<u32>().ok()
}
