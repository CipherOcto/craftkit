//! `craftkit-plugin-layout` — the document model of a **page layout
//! application**.
//!
//! # What is ported
//!
//! A page layout document is not a tree either, and this time it is
//! neither a grid nor a flat list: it is a **set of spreads**, each with
//! **pages** and a flat, z-ordered list of **items**, plus **layers**,
//! **stories**, **styles** and an **asset table**.
//!
//! - a **document** — a node;
//! - a **spread** — a node, and the thing that holds pages and items;
//! - a **page** — a node, the spread's child;
//! - an **item** — a node of an **open kind**: `layout.item.frame`,
//!   `layout.item.graphic`, `layout.item.group`;
//! - a **layer** — a node, and an item is *on* a layer rather than in it;
//! - a **story** — a node: the text, as a string;
//! - an **asset** — a node whose **stream** is the file;
//! - the item's **layer** and the item's **story** — **relations**:
//!   edges that say something;
//! - the **pages** of a spread, and the **frames** of a story — derived.
//!
//! # The five things the port found
//!
//! - **The typed id fields are the edges.** The application writes
//!   `Item.story`, `Page.parent`, `Story.frames` and `Item.overrides` as
//!   hand-written id fields, and the survey says a universal kit would
//!   "type them once". Here they are **relations**: an edge with a
//!   payload and a kind that says what it is. That is the **fourth**
//!   time an edge has answered "what is this relationship?" — after the
//!   PDF's reference, the CAD's layer and the prose's style.
//! - **A relation has an inverse, and the inverse is derived.** The
//!   application stores `Story.frames` as a field, so the two ends of
//!   the same fact are stored twice and can disagree. Here the edge is
//!   stored **once**, from the item to the story, and the story's frames
//!   are **worked out** from the edges. That is a new kind of derived
//!   value: not a number and not a geometry, but a **relation**.
//! - **A closed union of contents is exactly what the open kind is for.**
//!   The application's item content is `Unassigned | Text | Graphic |
//!   Group`, matched exhaustively in the document, the composer, the
//!   renderer and every exporter — so a new kind of content is a change
//!   in five places at once. Here the **caller names the kind**, and the
//!   list is open.
//! - **The asset's bytes are in the package, not in the document.** The
//!   application keeps them beside the document and the document holds
//!   the id — which is this kit's **stream** (§25), the third port to
//!   land on it after the artist's stage and the prose's media.
//! - **A story is a string and its runs are intervals.** The second tree
//!   to say it, after the word processor: the text is stored **once**,
//!   and what formats it is said over the text rather than beside it.

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
use craftkit_sdk::{
    Document, EngineError, Node, NodeRef, Number, Outcome, Plan, PlanItem, Relation, Session, Text,
};

/// The layout layer.
///
/// It needs the core, and it says so.
#[derive(Layer)]
#[layer(id = "layout", name = "Layout", requires("core"))]
pub struct LayoutLayer;

/// A **document**: spreads, layers, stories and assets.
#[derive(NodeKind)]
#[node_kind(kind = "layout.document", label = "Document", layer = "layout", evaluates)]
pub struct DocumentKind;

impl DocumentKind {
    /// Not drawn.
    fn paint(_node: &Node, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a document **means**: what it holds.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        Some(json!({
            "spreads": node.children.iter().filter(|child| child.kind == "layout.spread").count(),
            "layers": node.children.iter().filter(|child| child.kind == "layout.layer").count(),
            "stories": node.children.iter().filter(|child| child.kind == "layout.story").count(),
            "assets": node.children.iter().filter(|child| child.kind == "layout.asset").count(),
        }))
    }
}

/// A **spread**: pages, and a flat list of items.
#[derive(NodeKind)]
#[node_kind(kind = "layout.spread", label = "Spread", layer = "layout", evaluates)]
pub struct SpreadKind;

impl SpreadKind {
    /// Not drawn.
    fn paint(_node: &Node, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a spread **means**: its pages and its items.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        Some(json!({
            "pages": node.children.iter().filter(|child| child.kind == "layout.page").count(),
            "items": node.children.iter().filter(|child| child.kind.starts_with("layout.item.")).count(),
        }))
    }
}

/// A **page**: one face of a spread.
#[derive(NodeKind)]
#[node_kind(kind = "layout.page", label = "Page", layer = "layout", evaluates)]
pub struct PageKind;

impl PageKind {
    /// Not drawn.
    fn paint(_node: &Node, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a page **means**: its size.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        Some(json!({
            "width": node.data.get("width").cloned().unwrap_or(Json::Null),
            "height": node.data.get("height").cloned().unwrap_or(Json::Null),
        }))
    }
}

/// A **layer**: a named place things are put.
#[derive(NodeKind)]
#[node_kind(kind = "layout.layer", label = "Layer", layer = "layout", evaluates)]
pub struct LayerKind;

impl LayerKind {
    /// Not drawn.
    fn paint(_node: &Node, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a layer **means**: its name and its colour.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        Some(json!({
            "name": node.name,
            "color": node.data.get("color").cloned().unwrap_or(Json::Null),
        }))
    }
}

/// A **story**: the text, stored once.
#[derive(NodeKind)]
#[node_kind(kind = "layout.story", label = "Story", layer = "layout", evaluates)]
pub struct StoryKind;

impl StoryKind {
    /// Not drawn.
    fn paint(_node: &Node, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a story **means**: its text, and how long it is.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        let text = node.data.get("text").and_then(|text| text.as_str()).unwrap_or_default();
        Some(json!({
            "text": text,
            "characters": text.chars().count(),
        }))
    }
}

/// An **asset**: a file the document places.
#[derive(NodeKind)]
#[node_kind(kind = "layout.asset", label = "Asset", layer = "layout", evaluates)]
pub struct AssetKind;

impl AssetKind {
    /// Not drawn.
    fn paint(_node: &Node, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What an asset **means**: the file it is, and the kind of file.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        Some(json!({
            "kind": node.data.get("kind").cloned().unwrap_or(Json::Null),
            "file": match node.stream.clone() {
                Some(path) => Json::String(path),
                None => Json::Null,
            },
        }))
    }
}

/// An **item**: a frame, a graphic or a group.
///
/// The kinds are open, and the **caller** names them: the application
/// has a closed union of contents, matched in five places at once.
macro_rules! item_kind {
    ($struct:ident, $kind:literal, $label:literal) => {
        /// A kind of item.
        #[derive(NodeKind)]
        #[node_kind(kind = $kind, label = $label, layer = "layout", evaluates)]
        pub struct $struct;

        impl $struct {
            /// Not drawn.
            fn paint(_node: &Node, _canvas: &mut dyn craftkit_sdk::Canvas) {}

            /// Not hit.
            fn hit(_node: &Node, _point: craftkit_sdk::Point) -> bool {
                false
            }

            /// What an item **means**: its shape, its layer and its story.
            ///
            /// The layer and the story are **edges**, and the edges are
            /// found by looking, not by a field that could disagree.
            fn evaluate(node: &Node, document: &Document) -> Option<Json> {
                Some(json!({
                    "shape": node.data.get("shape").cloned().unwrap_or(Json::Null),
                    "rect": node.data.get("rect").cloned().unwrap_or(Json::Null),
                    "layer": edge_name(document, node.id, "layout.on_layer"),
                    "story": edge_name(document, node.id, "layout.text"),
                }))
            }
        }
    };
}

item_kind!(FrameKind, "layout.item.frame", "Frame");
item_kind!(GraphicKind, "layout.item.graphic", "Graphic");
item_kind!(GroupKind, "layout.item.group", "Group");

/// The name of the thing an edge points at.
fn edge_name(document: &Document, from: u64, kind: &str) -> Json {
    let Some(to) = document
        .relations
        .iter()
        .find(|relation| relation.kind == kind && relation.from == from)
        .map(|relation| relation.to)
    else {
        return Json::Null;
    };
    match document.node(to) {
        Some(node) => Json::String(node.name.clone()),
        None => Json::Null,
    }
}

/// Put a **document** in the document.
#[derive(Command)]
#[command(id = "layout.document.add", area = "layout", layer = "layout", icon = "plus", mutates, undoable)]
pub struct AddDocument {
    /// What to call it.
    pub name: Text,
}

impl AddDocument {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let name = self.name.0.trim().to_string();
        if name.is_empty() {
            return Err(EngineError::Failed {
                id: "layout.document.add".to_string(),
                message: "a document has a name".to_string(),
            });
        }
        let id = session.document_mut().add_node_with_under(
            None,
            name.clone(),
            "layout.document",
            None,
            json!({}),
        );
        Ok(Outcome::changed(format!("added the document `{name}`"))
            .with_data(json!({ "id": id, "name": name })))
    }
}

/// Put a **spread** in a document.
#[derive(Command)]
#[command(id = "layout.spread.add", area = "layout", layer = "layout", icon = "plus", mutates, undoable)]
pub struct AddSpread {
    /// The document it goes in.
    pub document: NodeRef,
    /// What to call it.
    pub name: Text,
}

impl AddSpread {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let name = self.name.0.trim().to_string();
        if name.is_empty() {
            return Err(EngineError::Failed {
                id: "layout.spread.add".to_string(),
                message: "a spread has a name".to_string(),
            });
        }
        let document = self.document.0;
        if session.document().node(document).is_none() {
            return Err(EngineError::Failed {
                id: "layout.spread.add".to_string(),
                message: format!("there is no document `{document}`"),
            });
        }
        let id = session.document_mut().add_node_with_under(
            Some(document),
            name.clone(),
            "layout.spread",
            None,
            json!({}),
        );
        Ok(Outcome::changed(format!("added the spread `{name}`"))
            .with_data(json!({ "id": id, "name": name })))
    }
}

/// Put a **page** in a spread.
#[derive(Command)]
#[command(id = "layout.page.add", area = "layout", layer = "layout", icon = "plus", mutates, undoable)]
pub struct AddPage {
    /// The spread it goes in.
    pub spread: NodeRef,
    /// What to call it.
    pub name: Text,
    /// How wide it is.
    pub width: Option<Number>,
    /// How high it is.
    pub height: Option<Number>,
}

impl AddPage {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let name = self.name.0.trim().to_string();
        if name.is_empty() {
            return Err(EngineError::Failed {
                id: "layout.page.add".to_string(),
                message: "a page has a name".to_string(),
            });
        }
        let spread = self.spread.0;
        if session.document().node(spread).is_none() {
            return Err(EngineError::Failed {
                id: "layout.page.add".to_string(),
                message: format!("there is no spread `{spread}`"),
            });
        }
        let width = size(self.width.as_ref(), 210.0, "layout.page.add", "a width")?;
        let height = size(self.height.as_ref(), 297.0, "layout.page.add", "a height")?;
        let id = session.document_mut().add_node_with_under(
            Some(spread),
            name.clone(),
            "layout.page",
            None,
            json!({ "width": width, "height": height }),
        );
        Ok(Outcome::changed(format!("added the page `{name}`"))
            .with_data(json!({ "id": id, "name": name, "width": width, "height": height })))
    }
}

/// Put a **layer** in a document.
#[derive(Command)]
#[command(id = "layout.layer.add", area = "layout", layer = "layout", icon = "plus", mutates, undoable)]
pub struct AddLayer {
    /// The document it goes in.
    pub document: NodeRef,
    /// The layer's name.
    pub name: Text,
    /// Its colour, as a name or a number.
    pub color: Option<Text>,
}

impl AddLayer {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let name = self.name.0.trim().to_string();
        if name.is_empty() {
            return Err(EngineError::Failed {
                id: "layout.layer.add".to_string(),
                message: "a layer has a name".to_string(),
            });
        }
        let document = self.document.0;
        if session.document().node(document).is_none() {
            return Err(EngineError::Failed {
                id: "layout.layer.add".to_string(),
                message: format!("there is no document `{document}`"),
            });
        }
        let color = self.color.as_ref().map(|color| color.0.clone());
        let id = session.document_mut().add_node_with_under(
            Some(document),
            name.clone(),
            "layout.layer",
            None,
            json!({ "color": color }),
        );
        Ok(Outcome::changed(format!("added the layer `{name}`")).with_data(json!({ "id": id, "name": name })))
    }
}

/// Put a **story** in a document: the text, stored once.
#[derive(Command)]
#[command(id = "layout.story.add", area = "layout", layer = "layout", icon = "plus", mutates, undoable)]
pub struct AddStory {
    /// The document it goes in.
    pub document: NodeRef,
    /// What to call it.
    pub name: Text,
    /// The text of the story.
    pub text: Text,
}

impl AddStory {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let name = self.name.0.trim().to_string();
        if name.is_empty() {
            return Err(EngineError::Failed {
                id: "layout.story.add".to_string(),
                message: "a story has a name".to_string(),
            });
        }
        let document = self.document.0;
        if session.document().node(document).is_none() {
            return Err(EngineError::Failed {
                id: "layout.story.add".to_string(),
                message: format!("there is no document `{document}`"),
            });
        }
        let id = session.document_mut().add_node_with_under(
            Some(document),
            name.clone(),
            "layout.story",
            None,
            json!({ "text": self.text.0 }),
        );
        Ok(Outcome::changed(format!("added the story `{name}`")).with_data(json!({ "id": id, "name": name })))
    }
}

/// Put an **asset** in a document: a file the document places.
///
/// The file is the node's **stream**, so the bytes travel beside the
/// document in the package and nobody names a path twice.
#[derive(Command)]
#[command(id = "layout.asset.add", area = "layout", layer = "layout", icon = "plus", mutates, undoable)]
pub struct AddAsset {
    /// The document it goes in.
    pub document: NodeRef,
    /// What to call it.
    pub name: Text,
    /// Where the file is, relative to the document.
    pub path: Text,
    /// What kind of file it is: `image`, `eps`, `font`, …
    pub kind: Option<Text>,
}

impl AddAsset {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let name = self.name.0.trim().to_string();
        if name.is_empty() {
            return Err(EngineError::Failed {
                id: "layout.asset.add".to_string(),
                message: "an asset has a name".to_string(),
            });
        }
        let path = self.path.0.trim().to_string();
        if path.is_empty() {
            return Err(EngineError::Failed {
                id: "layout.asset.add".to_string(),
                message: "an asset says which file it is".to_string(),
            });
        }
        let document = self.document.0;
        if session.document().node(document).is_none() {
            return Err(EngineError::Failed {
                id: "layout.asset.add".to_string(),
                message: format!("there is no document `{document}`"),
            });
        }
        let kind = self.kind.as_ref().map(|kind| kind.0.clone()).unwrap_or_else(|| "file".to_string());
        let id = session.document_mut().add_node_with_under(
            Some(document),
            name.clone(),
            "layout.asset",
            None,
            json!({ "kind": kind }),
        );
        let Some(node) = session.document_mut().node_mut(id) else {
            return Err(EngineError::Failed {
                id: "layout.asset.add".to_string(),
                message: format!("the asset `{name}` is not in the document"),
            });
        };
        node.stream = Some(path.clone());
        Ok(Outcome::changed(format!("added the asset `{name}`"))
            .with_data(json!({ "id": id, "name": name, "path": path })))
    }
}

/// Put an **item** in a spread.
///
/// The **kind** is named by the caller: `layout.item.frame`,
/// `layout.item.graphic`, `layout.item.group`, or anything else that is
/// a well-formed id — because the list is open.
#[derive(Command)]
#[command(id = "layout.item.add", area = "layout", layer = "layout", icon = "plus", mutates, undoable)]
pub struct AddItem {
    /// The spread it goes in.
    pub spread: NodeRef,
    /// What to call it.
    pub name: Text,
    /// What kind of item it is.
    pub kind: Option<Text>,
    /// Its shape: `rectangle`, `ellipse`, `polygon`, `line`, `path`.
    pub shape: Option<Text>,
    /// Where it is, as `{x, y, width, height}`.
    pub rect: Option<Json>,
}

impl AddItem {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let name = self.name.0.trim().to_string();
        if name.is_empty() {
            return Err(EngineError::Failed {
                id: "layout.item.add".to_string(),
                message: "an item has a name".to_string(),
            });
        }
        let spread = self.spread.0;
        if session.document().node(spread).is_none() {
            return Err(EngineError::Failed {
                id: "layout.item.add".to_string(),
                message: format!("there is no spread `{spread}`"),
            });
        }
        let kind = match &self.kind {
            Some(kind) => kind.0.trim().to_string(),
            None => "layout.item.frame".to_string(),
        };
        if let Some(malformed) = craftkit_sdk::contribution::malformed_dotted_id(&kind) {
            return Err(EngineError::Failed { id: "layout.item.add".to_string(), message: malformed });
        }
        let shape =
            self.shape.as_ref().map(|shape| shape.0.clone()).unwrap_or_else(|| "rectangle".to_string());
        let rect = self.rect.clone().unwrap_or(Json::Null);
        let id = session.document_mut().add_node_with_under(
            Some(spread),
            name.clone(),
            &kind,
            None,
            json!({ "shape": shape, "rect": rect }),
        );
        Ok(Outcome::changed(format!("added the item `{name}`"))
            .with_data(json!({ "id": id, "name": name, "kind": kind })))
    }
}

/// Put an item **on a layer**.
///
/// The layer is not the item's parent — the item is in the spread, and
/// it is *on* a layer. So this is an **edge**, and it says what it is.
#[derive(Command)]
#[command(id = "layout.item.layer", area = "layout", layer = "layout", icon = "plus", mutates, undoable)]
pub struct PutOnLayer {
    /// The item.
    pub item: NodeRef,
    /// The layer.
    pub layer: NodeRef,
}

impl PutOnLayer {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let item = self.item.0;
        let layer = self.layer.0;
        let Some(node) = session.document().node(layer) else {
            return Err(EngineError::Failed {
                id: "layout.item.layer".to_string(),
                message: format!("there is no layer `{layer}`"),
            });
        };
        if node.kind != "layout.layer" {
            return Err(EngineError::Failed {
                id: "layout.item.layer".to_string(),
                message: format!("the node `{layer}` is not a layer"),
            });
        }
        let name = node.name.clone();
        let added = session.document_mut().add_relation(Relation::with_payload(
            item,
            layer,
            "layout.on_layer",
            json!({ "layer": name.clone() }),
        ));
        if !added {
            return Err(EngineError::Failed {
                id: "layout.item.layer".to_string(),
                message: format!("the item `{item}` is already on the layer `{name}`"),
            });
        }
        Ok(Outcome::changed(format!("put the item {item} on the layer `{name}`"))
            .with_data(json!({ "item": item, "layer": layer, "name": name })))
    }
}

/// Put a **story** in an item: the text the frame shows.
///
/// The story is not the item's parent and not its payload: it is an
/// **edge**, and it says which story. The story's own list of frames is
/// **derived** from these edges, so the two ends cannot disagree.
#[derive(Command)]
#[command(id = "layout.item.story", area = "layout", layer = "layout", icon = "plus", mutates, undoable)]
pub struct PutStory {
    /// The item.
    pub item: NodeRef,
    /// The story.
    pub story: NodeRef,
}

impl PutStory {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let item = self.item.0;
        let story = self.story.0;
        let Some(node) = session.document().node(story) else {
            return Err(EngineError::Failed {
                id: "layout.item.story".to_string(),
                message: format!("there is no story `{story}`"),
            });
        };
        if node.kind != "layout.story" {
            return Err(EngineError::Failed {
                id: "layout.item.story".to_string(),
                message: format!("the node `{story}` is not a story"),
            });
        }
        let name = node.name.clone();
        let added = session.document_mut().add_relation(Relation::with_payload(
            item,
            story,
            "layout.text",
            json!({ "story": name.clone() }),
        ));
        if !added {
            return Err(EngineError::Failed {
                id: "layout.item.story".to_string(),
                message: format!("the item `{item}` already shows the story `{name}`"),
            });
        }
        Ok(Outcome::changed(format!("put the story `{name}` in the item {item}"))
            .with_data(json!({ "item": item, "story": story, "name": name })))
    }
}

/// A size, from an optional number, with a default.
fn size(value: Option<&Number>, default: f64, id: &str, what: &str) -> Result<f64, EngineError> {
    match value {
        Some(size) if size.0.is_finite() && size.0 > 0.0 => Ok(size.0),
        Some(_) => {
            Err(EngineError::Failed { id: id.to_string(), message: format!("{what} is a number above zero") })
        }
        None => Ok(default),
    }
}

/// The evaluator: the **pages** of a spread.
///
/// They are derived: the spread's children, in order, and their sizes.
#[derive(Evaluator)]
#[evaluator(id = "layout.pages", label = "Pages", layer = "layout")]
pub struct PagesEvaluator;

impl PagesEvaluator {
    /// The pages of every spread.
    fn evaluate(document: &Document) -> Plan {
        let mut plan = Plan::new();
        for spread in document.walk().filter(|node| node.kind == "layout.spread") {
            let pages: Vec<Json> = spread
                .children
                .iter()
                .filter(|child| child.kind == "layout.page")
                .map(|page| {
                    json!({
                        "page": page.id,
                        "name": page.name,
                        "width": page.data.get("width").cloned().unwrap_or(Json::Null),
                        "height": page.data.get("height").cloned().unwrap_or(Json::Null),
                    })
                })
                .collect();
            plan.items.push(PlanItem {
                node: spread.id,
                kind: "layout.pages".to_string(),
                value: json!({ "pages": pages }),
            });
        }
        plan
    }
}

/// The evaluator: the **frames** of a story.
///
/// The application stores this list as a field of its own, beside the
/// other end of the same fact. Here the edge is stored **once**, and the
/// frames are **worked out** from it: a relation's inverse, derived.
#[derive(Evaluator)]
#[evaluator(id = "layout.frames", label = "Frames", layer = "layout")]
pub struct FramesEvaluator;

impl FramesEvaluator {
    /// The frames of every story.
    fn evaluate(document: &Document) -> Plan {
        let mut plan = Plan::new();
        for story in document.walk().filter(|node| node.kind == "layout.story") {
            let frames: Vec<Json> = document
                .relations
                .iter()
                .filter(|relation| relation.kind == "layout.text" && relation.to == story.id)
                .map(|relation| {
                    let name = document.node(relation.from).map(|node| node.name.clone()).unwrap_or_default();
                    json!({ "item": relation.from, "name": name })
                })
                .collect();
            plan.items.push(PlanItem {
                node: story.id,
                kind: "layout.frames".to_string(),
                value: json!({ "frames": frames }),
            });
        }
        plan
    }
}

/// The panel that shows the document.
#[derive(Panel)]
#[panel(
    id = "panel.layout",
    label = "Layout",
    layer = "layout",
    commands(
        "layout.document.add",
        "layout.spread.add",
        "layout.page.add",
        "layout.item.add",
        "layout.story.add"
    )
)]
pub struct LayoutPanel;

// The whole of the battery, on every target.
craftkit_sdk::plugin_anchor!(
    LayoutLayer,
    DocumentKind,
    SpreadKind,
    PageKind,
    LayerKind,
    StoryKind,
    AssetKind,
    FrameKind,
    GraphicKind,
    GroupKind,
    AddDocument,
    AddSpread,
    AddPage,
    AddLayer,
    AddStory,
    AddAsset,
    AddItem,
    PutOnLayer,
    PutStory,
    PagesEvaluator,
    FramesEvaluator,
    LayoutPanel
);
