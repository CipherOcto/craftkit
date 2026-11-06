//! `craftkit-plugin-prose` — the document model of a **word processor**.
//!
//! # What is ported
//!
//! A word processor's document is not a tree either, and this time it
//! is not a grid: it is a **flat list of blocks**, a **side vector** of
//! styles, and a table of media.
//!
//! - a **document** — a node;
//! - a **block** — a node of an **open kind**: `prose.block.paragraph`
//!   and `prose.block.table`;
//! - a **paragraph's text** — the block's **payload**;
//! - a **paragraph's runs** — the **payload** too: a run is an
//!   **interval over the text**, and the text it covers is **derived**;
//! - a **comment** — the **payload** of the document too, by the block
//!   it is on: a comment is numerous and holds nothing (the module
//!   `comments`);
//! - a **list** — a **node**, and the blocks that are in it name it
//!   with a **relation**: the **number** of a block is derived, and a
//!   block that is moved is renumbered (the module `numbering`);
//! - the **page** of a document — a fact of the document: its size and
//!   its margins, in points, and whether it is a **landscape** one is
//!   derived (the module `page`);
//! - a **link** in a run — the target the run points at, and whether
//!   that target **may be opened** is derived: the tree's rule, that a
//!   link the system follows is a web link or a mail link and nothing
//!   else (the module `links`);
//! - **finding** and **replacing** in a block — a find is a reading
//!   and a replace rewrites the text and the runs together (the module
//!   `finding`);
//! - the **cells** of a table — a grid of values, read and written a
//!   cell at a time, with rows and columns going in and coming out
//!   (the module `tables`);
//! - the **resolution** of a revision — it may be accepted or
//!   rejected, and the block's text and runs are rewritten together
//!   (the module `resolving`);
//! - a **revision** in a run — that it was inserted, deleted or
//!   reformatted, and by whom: the tree's three kinds, and the words
//!   of a revision are derived (the module `revisions`);
//! - a **footnote** — a **node**, and the run that points at it names
//!   it: the **number** of a footnote is derived, and it is where the
//!   footnote is referenced (the module `footnotes`);
//! - a **bookmark** — a name for a place in a document, on a block:
//!   the **bookmarks of a document** are derived, and a link to a
//!   place that is there may be opened (the module `bookmarks`);
//! - a **style** — a node, and the side vector is the document's
//!   children;
//! - a **paragraph's style** — a **relation**: an edge that says
//!   something;
//! - a **media** — a node whose **stream** is the file: the bytes
//!   travel beside the document in the bundle;
//! - the **blocks** — derived, in order, by the evaluator.
//!
//! And a document can be **edited**: what a paragraph says may be
//! changed, a block may be taken away, and a block may be moved to
//! another place in the document (the module `editing`).
//!
//! # The five things the port found
//!
//! - **A word processor's document is a flat list, and the kit's tree
//!   fits it exactly.** The blocks are a sequence and not a nesting, so
//!   the tree is a list of children and **the order is the tree's
//!   order**. That is the third shape a document has had here — a grid,
//!   a flat store and a list — and the tree took the first and the
//!   third without a word being said.
//! - **A run is an interval, and the text it covers is derived.** The
//!   application stores a paragraph's text **once**, as a string, and a
//!   run is a **length** over that string plus the formatting. So the
//!   run does not carry the words: it says how many of them it covers.
//!   Here the run is an interval in the payload and the **words it
//!   covers are worked out every time**. That is the fourth instance
//!   of "derived values are derived", and the first one where the
//!   derived thing is **text**.
//! - **What is numerous is in the payload, and what is few is a node.**
//!   A paragraph's **runs** are numerous — one per word, sometimes more
//!   — so they are in the payload, beside the text they format. A
//!   **style** is few, so a style is a node. The grid's cells and the
//!   CAD's geometry said the same thing, and now it is a **rule** with
//!   four instances behind it.
//! - **The rule has a limit, and this port found it.** A table cell in
//!   the application holds a **nested list of blocks** — and a thing
//!   that holds a tree cannot be a value in a payload. So the rule is
//!   not "what is numerous is in the payload" but the sharper one:
//!   **what is numerous and holds nothing is in the payload; what
//!   holds a tree is a node.** The grid's cells hold numbers, the CAD's
//!   geometry holds numbers, a paragraph's runs hold formatting — and
//!   all three are payload. A cell that holds paragraphs is not, and
//!   the battery says so rather than pretending.
//! - **A style is a node and a paragraph's style is an edge.** The
//!   third time an edge has answered "what is this relationship?" —
//!   after the PDF's reference and the CAD's layer — and the third time
//!   the answer has been the same.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

use craftkit_derive::{Command, Evaluator, Layer, NodeKind, Panel};
use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Node, NodeRef, Outcome, Plan, PlanItem, Relation, Session, Text};

mod accessibility;
mod bookmarks;
mod characters;
mod comments;
pub mod contrast;
mod editing;
mod equations;
mod finding;
mod footnotes;
mod links;
mod media;
mod numbering;
mod outline;
mod page;
mod paragraph;
mod references;
mod resolving;
mod revisions;
mod styles;
mod tables;
mod titles;

pub use accessibility::CheckAccessibility;
pub use bookmarks::{DeleteBookmark, ReadBookmarks, SetBookmark};
pub use characters::FormatRun;
pub use comments::{DeleteComment, GetComment, SetComment};
pub use editing::{
    MoveBlock, RemoveBlock, SetAlignment, SetIndent, SetLineSpacing, SetParagraphText, SetSpacing,
};
pub use equations::{AddEquation, SetEquation};
pub use finding::{Find, Replace};
pub use footnotes::{AddFootnote, ReadFootnotes};
pub use links::ReadLinks;
pub use media::SetAlt;
pub use numbering::{AddList, ReadNumber, SetNumbering};
pub use outline::{ReadOutline, ReadStyleLevel, SetStyleLevel};
pub use page::{GetPage, SetPage};
pub use paragraph::{ReadBlockParagraph, ReadStyleParagraph, SetStyleParagraph};
pub use references::{AddReference, ReadReference, SetReference};
pub use resolving::{AcceptAllRevisions, AcceptRevision, RejectAllRevisions, RejectRevision};
pub use revisions::ReadRevisions;
pub use styles::{ReadBlockCharacter, ReadStyleCharacter, SetStyleCharacter};
pub use tables::{AddColumn, AddRow, GetCell, RemoveColumn, RemoveRow, SetCell, SetHeader};
pub use titles::SetTitle;

/// The prose layer.
///
/// It needs the core, and it says so.
#[derive(Layer)]
#[layer(id = "prose", name = "Prose", requires("core"))]
pub struct ProseLayer;

/// A **document**: a list of blocks, a vector of styles, a table of
/// media.
#[derive(NodeKind)]
#[node_kind(kind = "prose.document", label = "Document", layer = "prose", evaluates)]
pub struct DocumentKind;

impl DocumentKind {
    /// Not drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a document **means**: its blocks, its styles and its media,
    /// and what it says it is.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        Some(json!({
            "title": crate::titles::title_of(node).map(|title| json!(title)).unwrap_or(Json::Null),
            "blocks": node.children.iter().filter(|child| child.kind.starts_with("prose.block.")).count(),
            "styles": node.children.iter().filter(|child| child.kind == "prose.style").count(),
            "media": node.children.iter().filter(|child| child.kind == "prose.media").count(),
        }))
    }
}

/// A **block**: a paragraph or a table.
///
/// The kinds are open: a new kind of block is a new kind, and nothing
/// else moves.
macro_rules! block_kind {
    ($struct:ident, $kind:literal, $label:literal) => {
        /// A kind of block.
        #[derive(NodeKind)]
        #[node_kind(kind = $kind, label = $label, layer = "prose", evaluates)]
        pub struct $struct;

        impl $struct {
            /// Not drawn.
            fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

            /// Not hit.
            fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
                false
            }

            /// What a block **means**: its text, its runs and the words
            /// its runs cover.
            ///
            /// The words are **derived**: the text is stored once and a
            /// run says how many of them it covers.
            fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
                let text = text_of(node);
                Some(json!({
                    "text": node.data.get("text").cloned().unwrap_or(Json::Null),
                    "runs": runs_with_text(node),
                    "words": words_of(&text),
                }))
            }
        }
    };
}

block_kind!(ParagraphKind, "prose.block.paragraph", "Paragraph");

/// A **table**: a grid of values.
#[derive(NodeKind)]
#[node_kind(kind = "prose.block.table", label = "Table", layer = "prose", evaluates)]
pub struct TableKind;

impl TableKind {
    /// Not drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a table **means**: its grid, and whether its first row is
    /// a header row.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        let rows = node.data.get("rows").and_then(|rows| rows.as_array()).cloned().unwrap_or_default();
        let columns = rows.first().and_then(|row| row.as_array()).map(|row| row.len()).unwrap_or(0);
        Some(json!({
            "rows": rows,
            "columns": columns,
            "header": crate::tables::has_a_header(node),
        }))
    }
}

/// An **equation**: a block whose text is derived from its parts.
///
/// The parts are the equation, and the text is what they are written
/// as on one line. Both are kept, and they are written together, so
/// that they cannot disagree.
#[derive(NodeKind)]
#[node_kind(kind = "prose.block.equation", label = "Equation", layer = "prose", evaluates)]
pub struct EquationKind;

impl EquationKind {
    /// Not drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What an equation **means**: its text and its parts.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        Some(json!({
            "text": node.data.get("text").cloned().unwrap_or(Json::Null),
            "parts": node.data.get("parts").cloned().unwrap_or(Json::Null),
        }))
    }
}

/// A **cross-reference**: a block that stands for the text of a
/// block that carries a bookmark.
///
/// The text is **derived** — worked out from the block it points at —
/// and it is never written down, so a reference cannot be out of
/// date.
#[derive(NodeKind)]
#[node_kind(kind = "prose.block.reference", label = "Reference", layer = "prose", evaluates)]
pub struct ReferenceKind;

impl ReferenceKind {
    /// Not drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a reference **means**: the text it shows and the block it
    /// points at.
    fn evaluate(node: &Node, document: &Document) -> Option<Json> {
        let text = crate::references::text_of(document, node);
        let target = crate::references::target_of(document, node.id);
        Some(json!({
            "text": text,
            "target": match target {
                Some(target) => json!(target),
                None => Json::Null,
            },
        }))
    }
}

/// A **style**: a named look, and the side vector of the document.
#[derive(NodeKind)]
#[node_kind(kind = "prose.style", label = "Style", layer = "prose", evaluates)]
pub struct StyleKind;

impl StyleKind {
    /// Not drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a style **means**: its name, its kind, and the chain it is
    /// based on.
    ///
    /// The chain is **derived**: it is walked every time, and a chain
    /// that comes back to itself stops — the walk is bounded, so a
    /// cycle is not a hang.
    fn evaluate(node: &Node, document: &Document) -> Option<Json> {
        Some(json!({
            "name": node.name,
            "style_kind": node.data.get("style_kind").cloned().unwrap_or(Json::Null),
            "based_on": based_on_name(document, node.id),
            "chain": style_chain(document, node.id),
        }))
    }
}

/// A **footnote**: a note that belongs to the foot of the page.
#[derive(NodeKind)]
#[node_kind(kind = "prose.footnote", label = "Footnote", layer = "prose", evaluates)]
pub struct FootnoteKind;

impl FootnoteKind {
    /// Not drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a footnote **means**: what it says.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        Some(json!({
            "text": node.data.get("text").cloned().unwrap_or(Json::Null),
        }))
    }
}

/// A **list**: a kind of list, and the blocks that are in it.
#[derive(NodeKind)]
#[node_kind(kind = "prose.list", label = "List", layer = "prose", evaluates)]
pub struct ListKind;

impl ListKind {
    /// Not drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a list **means**: its kind, and the blocks that are in it.
    ///
    /// The blocks are **derived**: they are the blocks of the document
    /// that name the list.
    fn evaluate(node: &Node, document: &Document) -> Option<Json> {
        let blocks: Vec<u64> = document
            .relations
            .iter()
            .filter(|relation| {
                relation.kind == crate::numbering::NUMBERED_BY
                    && relation.payload.get("list").and_then(|list| list.as_u64()) == Some(node.id)
            })
            .map(|relation| relation.from)
            .collect();
        Some(json!({
            "kind": node.data.get("kind").cloned().unwrap_or(Json::Null),
            "blocks": blocks,
        }))
    }
}

/// A **media**: a file the document uses.
#[derive(NodeKind)]
#[node_kind(kind = "prose.media", label = "Media", layer = "prose", evaluates)]
pub struct MediaKind;

impl MediaKind {
    /// Not drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a media **means**: the file it is, and what it says
    /// about itself.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        Some(json!({
            "file": match node.stream.clone() {
                Some(path) => Json::String(path),
                None => Json::Null,
            },
            "alt": crate::media::alt_of(node).map(|alt| json!(alt)).unwrap_or(Json::Null),
        }))
    }
}

/// The text of a block, as a string.
fn text_of(node: &Node) -> String {
    node.data.get("text").and_then(|text| text.as_str()).unwrap_or_default().to_string()
}

/// The words of a text: the text split on whitespace.
fn words_of(text: &str) -> Vec<String> {
    text.split_whitespace().map(|word| word.to_string()).collect()
}

/// The runs of a block, each with the words it covers.
///
/// A run is an **interval**: a length over the text. The words it
/// covers are **derived**, and a run that runs past the end of the text
/// covers what is there and stops.
fn runs_with_text(node: &Node) -> Json {
    let text = text_of(node);
    let chars: Vec<char> = text.chars().collect();
    let Some(runs) = node.data.get("runs").and_then(|runs| runs.as_array()) else {
        return Json::Array(Vec::new());
    };
    let mut out = Vec::new();
    let mut at = 0usize;
    for run in runs {
        let length = run.get("len").and_then(|len| len.as_u64()).unwrap_or(0) as usize;
        let end = at.saturating_add(length).min(chars.len());
        let covered: String = chars[at.min(chars.len())..end].iter().collect();
        at = end;
        out.push(json!({
            "len": length,
            "bold": run.get("bold").cloned().unwrap_or(Json::Bool(false)),
            "italic": run.get("italic").cloned().unwrap_or(Json::Bool(false)),
            "underline": run.get("underline").cloned().unwrap_or(Json::Bool(false)),
            "colour": run.get("colour").cloned().unwrap_or(Json::Null),
            "size": run.get("size").cloned().unwrap_or(Json::Null),
            "link": run.get("link").cloned().unwrap_or(Json::Null),
            "revision": run.get("revision").cloned().unwrap_or(Json::Null),
            "footnote": run.get("footnote").cloned().unwrap_or(Json::Null),
            "text": covered,
        }));
    }
    Json::Array(out)
}

/// The name of the style a style is based on.
fn based_on_name(document: &Document, style: u64) -> Json {
    match relation_to(document, style, "prose.based_on") {
        Some(node) => Json::String(node.name.clone()),
        None => Json::Null,
    }
}

/// The chain of styles a style is based on, nearest first.
///
/// The walk is **bounded** and it stops at a style it has already seen,
/// so a cycle ends the chain instead of hanging the program.
fn style_chain(document: &Document, style: u64) -> Json {
    let mut chain: Vec<Json> = Vec::new();
    let mut seen: Vec<u64> = vec![style];
    let mut current = style;
    for _ in 0..32 {
        let Some(node) = relation_to(document, current, "prose.based_on") else { break };
        chain.push(Json::String(node.name.clone()));
        if seen.contains(&node.id) {
            break;
        }
        seen.push(node.id);
        current = node.id;
    }
    Json::Array(chain)
}

/// The node a relation points at, if the relation is there.
fn relation_to(document: &Document, from: u64, kind: &str) -> Option<Node> {
    let to = document
        .relations
        .iter()
        .find(|relation| relation.kind == kind && relation.from == from)
        .map(|relation| relation.to)?;
    document.node(to).cloned()
}

/// Put a **document** in the document.
#[derive(Command)]
#[command(id = "prose.document.add", area = "prose", layer = "prose", icon = "plus", mutates, undoable)]
pub struct AddDocument {
    /// What to call it.
    pub name: Text,
}

impl AddDocument {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let name = self.name.0.trim().to_string();
        if name.is_empty() {
            return Err(EngineError::Failed {
                id: "prose.document.add".to_string(),
                message: "a document has a name".to_string(),
            });
        }
        let id = session.document_mut()?.add_node_with_under(
            None,
            name.clone(),
            "prose.document",
            None,
            json!({}),
        );
        Ok(Outcome::changed(format!("added the document `{name}`"))
            .with_data(json!({ "id": id, "name": name })))
    }
}

/// Put a **style** in a document.
#[derive(Command)]
#[command(id = "prose.style.add", area = "prose", layer = "prose", icon = "plus", mutates, undoable)]
pub struct AddStyle {
    /// The document it goes in.
    pub document: NodeRef,
    /// The style's name.
    pub name: Text,
    /// What the style applies to: `paragraph`, `character`, `table` or
    /// `numbering` — or anything else, because the list is open.
    pub style_kind: Option<Text>,
    /// The style it is based on.
    pub based_on: Option<NodeRef>,
}

impl AddStyle {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let name = self.name.0.trim().to_string();
        if name.is_empty() {
            return Err(EngineError::Failed {
                id: "prose.style.add".to_string(),
                message: "a style has a name".to_string(),
            });
        }
        let document = self.document.0;
        if session.document().node(document).is_none() {
            return Err(EngineError::Failed {
                id: "prose.style.add".to_string(),
                message: format!("there is no document `{document}`"),
            });
        }
        let style_kind = self
            .style_kind
            .as_ref()
            .map(|kind| kind.0.trim().to_string())
            .unwrap_or_else(|| "paragraph".to_string());
        if style_kind.is_empty() {
            return Err(EngineError::Failed {
                id: "prose.style.add".to_string(),
                message: "a style says what it applies to".to_string(),
            });
        }
        let based_on = match self.based_on {
            Some(base) => {
                let Some(node) = session.document().node(base.0) else {
                    return Err(EngineError::Failed {
                        id: "prose.style.add".to_string(),
                        message: format!("there is no style `{}`", base.0),
                    });
                };
                if node.kind != "prose.style" {
                    return Err(EngineError::Failed {
                        id: "prose.style.add".to_string(),
                        message: format!("the node `{}` is not a style", base.0),
                    });
                }
                Some(base.0)
            }
            None => None,
        };
        let id = session.document_mut()?.add_node_with_under(
            Some(document),
            name.clone(),
            "prose.style",
            None,
            json!({ "style_kind": style_kind }),
        );
        if let Some(base) = based_on {
            let base_name = session.document().node(base).map(|node| node.name.clone()).unwrap_or_default();
            session.document_mut()?.add_relation(Relation::with_payload(
                id,
                base,
                "prose.based_on",
                json!({ "based_on": base_name }),
            ));
        }
        Ok(Outcome::changed(format!("added the style `{name}`")).with_data(json!({ "id": id, "name": name })))
    }
}

/// Put a **media** in a document: a file the document uses.
///
/// The file is the node's **stream**, so the bytes travel beside the
/// document in the bundle and nobody names a path twice.
#[derive(Command)]
#[command(id = "prose.media.add", area = "prose", layer = "prose", icon = "plus", mutates, undoable)]
pub struct AddMedia {
    /// The document it goes in.
    pub document: NodeRef,
    /// What to call it.
    pub name: Text,
    /// Where the file is, relative to the document.
    pub path: Text,
}

impl AddMedia {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let name = self.name.0.trim().to_string();
        if name.is_empty() {
            return Err(EngineError::Failed {
                id: "prose.media.add".to_string(),
                message: "a media has a name".to_string(),
            });
        }
        let path = self.path.0.trim().to_string();
        if path.is_empty() {
            return Err(EngineError::Failed {
                id: "prose.media.add".to_string(),
                message: "a media says which file it is".to_string(),
            });
        }
        let document = self.document.0;
        if session.document().node(document).is_none() {
            return Err(EngineError::Failed {
                id: "prose.media.add".to_string(),
                message: format!("there is no document `{document}`"),
            });
        }
        let id = session.document_mut()?.add_node_with_under(
            Some(document),
            name.clone(),
            "prose.media",
            None,
            json!({}),
        );
        let Some(node) = session.document_mut()?.node_mut(id) else {
            return Err(EngineError::Failed {
                id: "prose.media.add".to_string(),
                message: format!("the media `{name}` is not in the document"),
            });
        };
        node.stream = Some(path.clone());
        Ok(Outcome::changed(format!("added the media `{name}`"))
            .with_data(json!({ "id": id, "name": name, "path": path })))
    }
}

/// Put a **paragraph** in a document.
#[derive(Command)]
#[command(
    id = "prose.block.paragraph.add",
    area = "prose",
    layer = "prose",
    icon = "plus",
    mutates,
    undoable
)]
pub struct AddParagraph {
    /// The document it goes in.
    pub document: NodeRef,
    /// What it says.
    pub text: Text,
}

impl AddParagraph {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        add_block(
            session,
            self.document.0,
            "prose.block.paragraph",
            json!({ "text": self.text.0, "runs": [] }),
            "prose.block.paragraph.add",
        )
    }
}

/// Put a **table** in a document.
///
/// The rows are in the **payload**: a cell holds a value and not a
/// tree. A cell that holds blocks is a different thing, and this
/// battery does not pretend to have one.
#[derive(Command)]
#[command(id = "prose.block.table.add", area = "prose", layer = "prose", icon = "plus", mutates, undoable)]
pub struct AddTable {
    /// The document it goes in.
    pub document: NodeRef,
    /// The rows, each a list of cells.
    pub rows: Option<Json>,
}

impl AddTable {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let rows = match &self.rows {
            Some(rows) => rows.clone(),
            None => Json::Array(Vec::new()),
        };
        if !rows.is_array() {
            return Err(EngineError::Failed {
                id: "prose.block.table.add".to_string(),
                message: "the rows are a list".to_string(),
            });
        }
        // A table is a **grid**: every row as long as every other row.
        {
            let mut width: Option<usize> = None;
            for row in rows.as_array().unwrap_or(&Vec::new()) {
                let Some(cells) = row.as_array() else {
                    return Err(EngineError::Failed {
                        id: "prose.block.table.add".to_string(),
                        message: "a row of a table is a list of cells".to_string(),
                    });
                };
                match width {
                    None => width = Some(cells.len()),
                    Some(width) if width != cells.len() => {
                        return Err(EngineError::Failed {
                            id: "prose.block.table.add".to_string(),
                            message: "the rows of a table are the same length".to_string(),
                        });
                    }
                    Some(_) => {}
                }
            }
        }
        add_block(
            session,
            self.document.0,
            "prose.block.table",
            json!({ "text": "", "runs": [], "rows": rows }),
            "prose.block.table.add",
        )
    }
}

/// Put a **block** in a document.
fn add_block(
    session: &mut Session,
    document: u64,
    kind: &str,
    data: Json,
    id: &str,
) -> Result<Outcome, EngineError> {
    let Some(node) = session.document().node(document) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no document `{document}`"),
        });
    };
    if node.kind != "prose.document" {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the node `{document}` is not a document"),
        });
    }
    let name = match kind {
        "prose.block.paragraph" => "a paragraph",
        _ => "a table",
    };
    let block =
        session.document_mut()?.add_node_with_under(Some(document), name.to_string(), kind, None, data);
    Ok(Outcome::changed(format!("added {name}")).with_data(json!({ "id": block, "kind": kind })))
}

/// Put a **style** on a block.
///
/// The style is not the block's parent — the block is in the document —
/// and it is not in the block's payload either: it is a **relation**, an
/// edge that says which style.
#[derive(Command)]
#[command(id = "prose.block.style", area = "prose", layer = "prose", icon = "plus", mutates, undoable)]
pub struct PutOnStyle {
    /// The block.
    pub block: NodeRef,
    /// The style.
    pub style: NodeRef,
}

impl PutOnStyle {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let block = self.block.0;
        let style = self.style.0;
        let Some(node) = session.document().node(style) else {
            return Err(EngineError::Failed {
                id: "prose.block.style".to_string(),
                message: format!("there is no style `{style}`"),
            });
        };
        if node.kind != "prose.style" {
            return Err(EngineError::Failed {
                id: "prose.block.style".to_string(),
                message: format!("the node `{style}` is not a style"),
            });
        }
        let name = node.name.clone();
        let added = session.document_mut()?.add_relation(Relation::with_payload(
            block,
            style,
            "prose.styled_by",
            json!({ "style": name.clone() }),
        ));
        if !added {
            return Err(EngineError::Failed {
                id: "prose.block.style".to_string(),
                message: format!("the block `{block}` is already in the style `{name}`"),
            });
        }
        Ok(Outcome::changed(format!("put the block {block} in the style `{name}`"))
            .with_data(json!({ "block": block, "style": style, "name": name })))
    }
}

/// Set the **runs** of a block: the formatting of its text.
///
/// A run is an **interval**: it says how many of the text's characters
/// it covers, and the characters it covers are **derived**. The runs
/// are numerous — one per word, sometimes more — so they are in the
/// block's payload, and the text they format is beside them.
#[derive(Command)]
#[command(id = "prose.block.runs", area = "prose", layer = "prose", icon = "list", mutates, undoable)]
pub struct SetRuns {
    /// The block.
    pub block: NodeRef,
    /// The runs: a list of `{len, bold, italic, underline, colour,
    /// size}`.
    pub runs: Json,
}

impl SetRuns {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let block = self.block.0;
        let Some(runs) = self.runs.as_array() else {
            return Err(EngineError::Failed {
                id: "prose.block.runs".to_string(),
                message: "the runs are a list".to_string(),
            });
        };
        if runs.len() > 100_000 {
            return Err(EngineError::Failed {
                id: "prose.block.runs".to_string(),
                message: "a block does not hold a hundred thousand runs".to_string(),
            });
        }
        let Some(node) = session.document().node(block) else {
            return Err(EngineError::Failed {
                id: "prose.block.runs".to_string(),
                message: format!("there is no block `{block}`"),
            });
        };
        let text = text_of(node);
        let characters = text.chars().count();
        let mut covered = 0usize;
        for run in runs {
            let Some(length) = run.get("len").and_then(|len| len.as_u64()) else {
                return Err(EngineError::Failed {
                    id: "prose.block.runs".to_string(),
                    message: "a run says how many characters it covers".to_string(),
                });
            };
            let length = length as usize;
            covered = covered.saturating_add(length);
            if covered > characters {
                return Err(EngineError::Failed {
                    id: "prose.block.runs".to_string(),
                    message: "the runs cover more characters than the text has".to_string(),
                });
            }
            // A run may carry a **revision**: it was inserted, it was
            // deleted, or its format was changed — and by whom.
            if let Some(revision) = run.get("revision") {
                crate::revisions::check(revision, "prose.block.runs")?;
            }
            // A run may carry a **footnote**: the note it points at.
            if let Some(footnote) = run.get("footnote") {
                let Some(footnote) = footnote.as_u64() else {
                    return Err(EngineError::Failed {
                        id: "prose.block.runs".to_string(),
                        message: "a footnote is a note".to_string(),
                    });
                };
                let found = session.document().node(footnote);
                match found {
                    Some(node) if node.kind == "prose.footnote" => {}
                    Some(node) => {
                        return Err(EngineError::Failed {
                            id: "prose.block.runs".to_string(),
                            message: format!("the node `{footnote}` is a `{}` and not a footnote", node.kind),
                        });
                    }
                    None => {
                        return Err(EngineError::Failed {
                            id: "prose.block.runs".to_string(),
                            message: format!("there is no footnote `{footnote}`"),
                        });
                    }
                }
            }
            // A run may carry a **link**: the target it points at.
            // The target is a text and it is bounded; whether it may
            // be opened is said by the reading of the links.
            if let Some(link) = run.get("link") {
                let Some(target) = link.as_str() else {
                    return Err(EngineError::Failed {
                        id: "prose.block.runs".to_string(),
                        message: "a link is a target".to_string(),
                    });
                };
                if target.chars().count() > crate::links::MAX_TARGET {
                    return Err(EngineError::Failed {
                        id: "prose.block.runs".to_string(),
                        message: format!("a target is at most {} characters long", crate::links::MAX_TARGET),
                    });
                }
            }
        }
        let Some(node) = session.document_mut()?.node_mut(block) else {
            return Err(EngineError::Failed {
                id: "prose.block.runs".to_string(),
                message: format!("there is no block `{block}`"),
            });
        };
        node.data["runs"] = Json::Array(runs.clone());
        Ok(Outcome::changed(format!("set the runs of the block {block}"))
            .with_data(json!({ "block": block, "runs": runs.len() })))
    }
}

/// The evaluator: the **blocks** of a document, in order.
///
/// The order is the tree's order, the style of a block is the edge that
/// says it, and the words a run covers are worked out from the text.
/// All three are **derived**.
#[derive(Evaluator)]
#[evaluator(id = "prose.blocks", label = "Blocks", layer = "prose")]
pub struct BlocksEvaluator;

impl BlocksEvaluator {
    /// The blocks of every document.
    fn evaluate(document: &Document) -> Plan {
        let mut plan = Plan::new();
        for prose in document.walk().filter(|node| node.kind == "prose.document") {
            let mut blocks: Vec<Json> = Vec::new();
            for block in prose.children.iter().filter(|child| child.kind.starts_with("prose.block.")) {
                let style = document
                    .relations
                    .iter()
                    .find(|relation| relation.kind == "prose.styled_by" && relation.from == block.id)
                    .map(|relation| relation.payload.get("style").cloned().unwrap_or(Json::Null));
                blocks.push(json!({
                    "block": block.id,
                    "kind": block.kind,
                    "text": crate::references::text_of(document, block),
                    "runs": runs_with_text(block),
                    "style": style,
                }));
            }
            plan.items.push(PlanItem {
                node: prose.id,
                kind: "prose.blocks".to_string(),
                value: json!({ "blocks": blocks }),
            });
        }
        plan
    }
}

/// The panel that shows the document.
#[derive(Panel)]
#[panel(
    id = "panel.prose",
    label = "Prose",
    layer = "prose",
    commands("prose.document.add", "prose.style.add", "prose.media.add", "prose.block.paragraph.add")
)]
pub struct ProsePanel;

// The whole of the battery, on every target.
craftkit_sdk::plugin_anchor!(
    ProseLayer,
    DocumentKind,
    ParagraphKind,
    TableKind,
    EquationKind,
    ReferenceKind,
    StyleKind,
    MediaKind,
    AddDocument,
    AddStyle,
    AddMedia,
    SetAlt,
    SetTitle,
    SetHeader,
    CheckAccessibility,
    AddParagraph,
    AddTable,
    AddEquation,
    SetEquation,
    PutOnStyle,
    SetRuns,
    SetStyleCharacter,
    SetStyleParagraph,
    SetStyleLevel,
    ReadStyleLevel,
    ReadOutline,
    AddReference,
    SetReference,
    ReadReference,
    ReadStyleParagraph,
    ReadBlockParagraph,
    ReadStyleCharacter,
    ReadBlockCharacter,
    FormatRun,
    SetParagraphText,
    RemoveBlock,
    MoveBlock,
    SetAlignment,
    SetIndent,
    SetSpacing,
    SetLineSpacing,
    AcceptRevision,
    RejectRevision,
    Find,
    Replace,
    GetCell,
    SetCell,
    AddRow,
    RemoveRow,
    AddColumn,
    RemoveColumn,
    AcceptAllRevisions,
    RejectAllRevisions,
    BlocksEvaluator,
    ProsePanel
);
