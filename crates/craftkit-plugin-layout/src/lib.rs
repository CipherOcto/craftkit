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

mod articles;
mod bleed;
mod bookmarks;
mod columns;
mod facing;
mod grouping;
mod guides;
mod items;
mod layers;
mod linked;
mod links;
mod margins;
mod order;
mod pages;
mod paint;
mod parents;
mod radii;
mod rotation;
mod shadows;
mod sides;
mod styles;

use craftkit_derive::{Evaluator, Layer, NodeKind, Panel, commands};
use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Node, NodeRef, Number, Outcome, Plan, PlanItem, Relation, Text};

pub use articles::{
    LayoutArticleAdd, LayoutArticleDelete, LayoutArticleNew, LayoutArticleOptions, LayoutArticleRemove,
    LayoutDocumentArticles,
};
pub use bleed::LayoutDocumentBleed;
pub use bookmarks::{LayoutBookmarkAdd, LayoutBookmarkDelete, LayoutBookmarkRename, LayoutDocumentBookmarks};
pub use columns::LayoutPageColumns;
pub use facing::LayoutPageFacing;
pub use grouping::{LayoutItemGroup, LayoutItemUngroup};
pub use guides::{
    LayoutGuideAdd, LayoutGuideCreate, LayoutGuideDelete, LayoutGuideDeleteAll, LayoutGuideMove,
    LayoutPageGuides,
};
pub use items::{
    LayoutItemHide, LayoutItemLock, LayoutItemOpacity, LayoutItemScale, LayoutItemShow, LayoutItemUnlock,
};
pub use layers::{
    LayoutLayerDelete, LayoutLayerHide, LayoutLayerLock, LayoutLayerMerge, LayoutLayerMove,
    LayoutLayerRename, LayoutLayerShow, LayoutLayerUnlock, LayoutLayerUnusedDelete,
};
pub use linked::{
    LayoutDocumentLinks as ReadStoryLinks, LayoutStoryLink, LayoutStorySet, LayoutStoryUnlink,
    LayoutStoryUpdate,
};
pub use links::{LayoutItemLink, LayoutItemPageLink, LayoutLinks};
pub use margins::LayoutPageMargin;
pub use order::{LayoutItemBack, LayoutItemBackward, LayoutItemForward, LayoutItemFront, LayoutItemList};
pub use pages::{LayoutPageDelete, LayoutPageMove};
pub use paint::{LayoutItemFill, LayoutItemStroke};
pub use parents::{LayoutPageParent, LayoutPageParentClear};
pub use radii::LayoutItemRadius;
pub use rotation::LayoutItemRotate;
pub use shadows::LayoutItemShadow;
pub use sides::LayoutPageSide;
pub use styles::{LayoutItemStyle, LayoutStyleAdd, LayoutStyleFill, LayoutStyleStroke};

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
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a document **means**: what it holds.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        Some(json!({
            "spreads": node.children.iter().filter(|child| child.kind == "layout.spread").count(),
            "layers": node.children.iter().filter(|child| child.kind == "layout.layer").count(),
            "stories": node.children.iter().filter(|child| child.kind == "layout.story").count(),
            "assets": node.children.iter().filter(|child| child.kind == "layout.asset").count(),
            "bleed": crate::bleed::bleed_of(node).to_json(),
        }))
    }
}

/// A **style**: a named set of paint facts that items may use.
///
/// A style may be **based on** another, and the base's paint sits
/// under the style's own.
#[derive(NodeKind)]
#[node_kind(kind = "layout.style", label = "Style", layer = "layout", evaluates)]
pub struct StyleKind;

impl StyleKind {
    /// Not drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a style **means**: its paint, and the paint of its base
    /// under it.
    fn evaluate(node: &Node, document: &Document) -> Option<Json> {
        Some(json!({
            "fill": crate::styles::effective_fill(document, node.id),
            "stroke": crate::styles::effective_stroke(document, node.id),
            "base": crate::styles::chain_of(document, node.id)
                .first()
                .filter(|base| **base != node.id)
                .map(|base| json!(base))
                .unwrap_or(Json::Null),
        }))
    }
}

/// A **bookmark**: a name for a page of a document.
#[derive(NodeKind)]
#[node_kind(kind = "layout.bookmark", label = "Bookmark", layer = "layout", evaluates)]
pub struct BookmarkKind;

impl BookmarkKind {
    /// A bookmark is not drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Whether a point is inside a bookmark: never.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a bookmark **means**: its name and the page it names.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        Some(json!({
            "name": node.data.get("name").cloned().unwrap_or(Json::Null),
            "page": node.data.get("page").cloned().unwrap_or(Json::Null),
        }))
    }
}

/// An **article**: a named list of items in a reading order.
#[derive(NodeKind)]
#[node_kind(kind = "layout.article", label = "Article", layer = "layout", evaluates)]
pub struct ArticleKind;

impl ArticleKind {
    /// An article is not drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Whether a point is inside an article: never.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What an article **means**: its name, its items and whether it is
    /// exported.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        Some(json!({
            "name": node.data.get("name").cloned().unwrap_or(Json::Null),
            "items": node.data.get("items").cloned().unwrap_or(Json::Null),
            "export": node.data.get("export").and_then(|export| export.as_bool()).unwrap_or(true),
        }))
    }
}

/// A **guide**: a line that is drawn on a page.
#[derive(NodeKind)]
#[node_kind(kind = "layout.guide", label = "Guide", layer = "layout", evaluates)]
pub struct GuideKind;

impl GuideKind {
    /// A guide is not drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Whether a point is inside a guide: never.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a guide **means**: how it lies and where it stands.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        Some(json!({
            "orientation": node.data.get("orientation").cloned().unwrap_or(Json::Null),
            "position": node.data.get("position").cloned().unwrap_or(Json::Null),
            "spread": node.data.get("spread").and_then(|spread| spread.as_bool()).unwrap_or(false),
            "liquid": node.data.get("liquid").and_then(|liquid| liquid.as_bool()).unwrap_or(false),
        }))
    }
}

/// A **spread**: pages, and a flat list of items.
#[derive(NodeKind)]
#[node_kind(kind = "layout.spread", label = "Spread", layer = "layout", evaluates)]
pub struct SpreadKind;

impl SpreadKind {
    /// Not drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
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
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a page **means**: its size.
    fn evaluate(node: &Node, document: &Document) -> Option<Json> {
        let bleed = match crate::bleed::bleed_box_of(document, node) {
            Some((x, y, width, height)) => json!({ "x": x, "y": y, "width": width, "height": height }),
            None => Json::Null,
        };
        let margin_box = match crate::parents::margin_box_of(document, node.id) {
            Some((x, y, width, height)) => json!({ "x": x, "y": y, "width": width, "height": height }),
            None => Json::Null,
        };
        Some(json!({
            "width": node.data.get("width").cloned().unwrap_or(Json::Null),
            "height": node.data.get("height").cloned().unwrap_or(Json::Null),
            "bleed": bleed,
            "margin": crate::parents::margins_of(document, node.id).to_json(),
            "side": crate::sides::side_of(node).name(),
            "spread": crate::facing::spread_of(document, node.id).map(|spread| json!(spread)).unwrap_or(Json::Null),
            "facing": crate::facing::facing_of(document, node.id).map(|facing| json!(facing)).unwrap_or(Json::Null),
            "inside": crate::sides::inside_outside(node).0,
            "outside": crate::sides::inside_outside(node).1,
            "margin_box": margin_box,
            "parent": crate::parents::parent_of(node).map(|parent| json!(parent)).unwrap_or(Json::Null),
            "columns": crate::parents::columns_of(document, node.id).to_json(),
            "column_boxes": crate::parents::column_boxes_of(document, node.id)
                .map(|boxes| json!(boxes
                    .into_iter()
                    .map(|(x, y, width, height)| json!({ "x": x, "y": y, "width": width, "height": height }))
                    .collect::<Vec<_>>()))
                .unwrap_or(Json::Null),
        }))
    }
}

/// A **layer**: a named place things are put.
#[derive(NodeKind)]
#[node_kind(kind = "layout.layer", label = "Layer", layer = "layout", evaluates)]
pub struct LayerKind;

impl LayerKind {
    /// Not drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
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
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
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
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
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
            fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

            /// Not hit.
            fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
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
                    "layer": edge_name(document, node.id, craftkit_sdk::layer::ON_LAYER),
                    "story": edge_name(document, node.id, "layout.text"),
                    "opacity": crate::items::opacity_of(node),
                    "rotation": crate::rotation::rotation_of(node),
                    "radians": crate::rotation::radians_of(node),
                    "turned": crate::rotation::turned_json(node),
                    "fill": crate::styles::effective_paint(document, node).0,
                    "stroke": crate::styles::effective_paint(document, node).1,
                    "own_fill": crate::paint::fill_of(node),
                    "own_stroke": crate::paint::stroke_of(node),
                    "style": edge_name(document, node.id, "layout.styled_by"),
                    "radius": crate::radii::radius_of(node)
                        .map(|radius| json!(radius))
                        .unwrap_or(Json::Null),
                    "corners": crate::radii::corners_json(node),
                    "shadow": crate::shadows::shadow_json(node),
                    "locked": crate::items::is_locked(node),
                    "hidden": node.muted,
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

commands! {
    /// Put a **document** in the document.
    layout.document.add {
        area: "layout",
        layer: "layout",
        icon: "plus",
        mutates,
        undoable,
        example,
        params: {
            /// What to call it.
            name: Text,
        },
        run: {
            let name = self.name.0.trim().to_string();
            if name.is_empty() {
                return Err(EngineError::Failed {
                    id: "layout.document.add".to_string(),
                    message: "a document has a name".to_string(),
                });
            }
            let id = session.document_mut()?.add_node_with_under(
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
    layout.spread.add {
        area: "layout",
        layer: "layout",
        icon: "plus",
        mutates,
        undoable,
        params: {
            /// The document it goes in.
            document: NodeRef,
            /// What to call it.
            name: Text,
        },
        run: {
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
            let id = session.document_mut()?.add_node_with_under(
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
    layout.page.add {
        area: "layout",
        layer: "layout",
        icon: "plus",
        mutates,
        undoable,
        params: {
            /// The spread it goes in.
            spread: NodeRef,
            /// What to call it.
            name: Text,
            /// How wide it is.
            width: Option<Number>,
            /// How high it is.
            height: Option<Number>,
        },
        run: {
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
            let id = session.document_mut()?.add_node_with_under(
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
    layout.layer.add {
        area: "layout",
        layer: "layout",
        icon: "plus",
        mutates,
        undoable,
        params: {
            /// The document it goes in.
            document: NodeRef,
            /// The layer's name.
            name: Text,
            /// Its colour, as a name or a number.
            color: Option<Text>,
        },
        run: {
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
            let id = session.document_mut()?.add_node_with_under(
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
    layout.story.add {
        area: "layout",
        layer: "layout",
        icon: "plus",
        mutates,
        undoable,
        params: {
            /// The document it goes in.
            document: NodeRef,
            /// What to call it.
            name: Text,
            /// The text of the story.
            text: Text,
        },
        run: {
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
            let id = session.document_mut()?.add_node_with_under(
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
    layout.asset.add {
        area: "layout",
        layer: "layout",
        icon: "plus",
        mutates,
        undoable,
        params: {
            /// The document it goes in.
            document: NodeRef,
            /// What to call it.
            name: Text,
            /// Where the file is, relative to the document.
            path: Text,
            /// What kind of file it is: `image`, `eps`, `font`, …
            kind: Option<Text>,
        },
        run: {
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
            let id = session.document_mut()?.add_node_with_under(
                Some(document),
                name.clone(),
                "layout.asset",
                None,
                json!({ "kind": kind }),
            );
            let Some(node) = session.document_mut()?.node_mut(id) else {
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
    layout.item.add {
        area: "layout",
        layer: "layout",
        icon: "plus",
        mutates,
        undoable,
        params: {
            /// The spread it goes in.
            spread: NodeRef,
            /// What to call it.
            name: Text,
            /// What kind of item it is.
            kind: Option<Text>,
            /// Its shape: `rectangle`, `ellipse`, `polygon`, `line`, `path`.
            shape: Option<Text>,
            /// Where it is, as `{x, y, width, height}`.
            rect: Option<Json>,
        },
        run: {
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
            let id = session.document_mut()?.add_node_with_under(
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
    layout.item.layer {
        area: "layout",
        layer: "layout",
        icon: "plus",
        mutates,
        undoable,
        params: {
            /// The item.
            item: NodeRef,
            /// The layer.
            layer: NodeRef,
        },
        run: {
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
            // A **locked** item may not be edited, and a layer is an edit.
            let Some(item_node) = session.document().node(item).cloned() else {
                return Err(EngineError::Failed {
                    id: "layout.item.layer".to_string(),
                    message: format!("there is no item `{item}`"),
                });
            };
            crate::items::may_edit(session, &item_node, "layout.item.layer")?;
            let name = node.name.clone();
            let added = session.document_mut()?.add_relation(Relation::with_payload(
                item,
                layer,
                craftkit_sdk::layer::ON_LAYER,
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
    layout.item.story {
        area: "layout",
        layer: "layout",
        icon: "plus",
        mutates,
        undoable,
        params: {
            /// The item.
            item: NodeRef,
            /// The story.
            story: NodeRef,
        },
        run: {
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
            // A **locked** item may not be edited, and a story is an edit.
            let Some(item_node) = session.document().node(item).cloned() else {
                return Err(EngineError::Failed {
                    id: "layout.item.story".to_string(),
                    message: format!("there is no item `{item}`"),
                });
            };
            crate::items::may_edit(session, &item_node, "layout.item.story")?;
            let name = node.name.clone();
            let added = session.document_mut()?.add_relation(Relation::with_payload(
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
    StyleKind,
    StoryKind,
    AssetKind,
    FrameKind,
    GraphicKind,
    GroupKind,
    LayoutDocumentAdd,
    LayoutDocumentBleed,
    LayoutSpreadAdd,
    LayoutPageAdd,
    LayoutPageColumns,
    LayoutPageMargin,
    LayoutPageParent,
    LayoutPageParentClear,
    LayoutPageSide,
    LayoutPageFacing,
    LayoutLayerAdd,
    LayoutStoryAdd,
    LayoutAssetAdd,
    LayoutItemAdd,
    LayoutItemOpacity,
    LayoutItemHide,
    LayoutItemShow,
    LayoutItemLock,
    LayoutItemUnlock,
    LayoutItemScale,
    LayoutItemRotate,
    LayoutItemRadius,
    LayoutItemShadow,
    LayoutItemList,
    LayoutItemFront,
    LayoutItemBack,
    LayoutItemForward,
    LayoutItemBackward,
    LayoutItemLayer,
    LayoutItemStory,
    LayoutItemLink,
    LayoutItemFill,
    LayoutItemStroke,
    LayoutStyleAdd,
    LayoutStyleFill,
    LayoutStyleStroke,
    LayoutItemStyle,
    LayoutItemPageLink,
    LayoutLinks,
    PagesEvaluator,
    FramesEvaluator,
    LayoutPanel
);
