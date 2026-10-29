//! `craftkit-plugin-shapes` — a plugin that adds **document semantics**.
//!
//! # This is the interesting plugin
//!
//! The greeting plugin adds commands: things a person or an agent can *do*.
//! This one adds a **kind of thing to the document**, and everything that
//! follows from a kind:
//!
//! - a **layer**, `geometry`, which the other contributions are in;
//! - a **node kind**, `geometry.rect`, which knows how it is painted and
//!   what is inside it;
//! - a **tool**, `tool.rect`, which is a gesture that runs a command;
//! - a **command**, `geometry.rectangle`, which puts one in the document.
//!
//! # The core does not know any of this
//!
//! There is no `match` on a kind anywhere in the SDK. The core draws a node
//! by asking the kind's registry, and hit-tests it by asking too. A kind
//! that this plugin adds is drawn by this plugin, and a document that
//! contains one can be saved, loaded and edited by a program that has never
//! heard of it.

use craftkit_derive::{Command, Evaluator, Layer, NodeKind, Panel, Tool};
use craftkit_sdk::{
    Canvas, Colour, Document, EngineError, Node, Number, Outcome, Point, Rect, Session, Text,
};

// The plugin's own JSON value type. It uses the SDK's re-export, so its
// dependency list is the SDK and the macro — and nothing else.
use craftkit_sdk::serde_json;
use craftkit_sdk::{Plan, PlanItem};

/// The geometry layer.
///
/// It needs the core, so it says so, and the ecosystem checks the graph
/// when the program starts.
#[derive(Layer)]
#[layer(id = "geometry", name = "Geometry", requires("core"))]
pub struct GeometryLayer;

/// A rectangle.
#[derive(NodeKind)]
#[node_kind(kind = "geometry.rect", label = "Rectangle", layer = "geometry", evaluates)]
pub struct RectKind;

impl RectKind {
    /// How a rectangle is drawn.
    fn paint(node: &Node, canvas: &mut dyn Canvas) {
        let fill = if node.muted { Colour::NONE } else { Colour::WHITE };
        canvas.rect(node.rect, fill);
    }

    /// Whether a point is inside a rectangle.
    fn hit(node: &Node, point: Point) -> bool {
        node.rect.contains(point)
    }

    /// What a rectangle **means**, in the plan.
    ///
    /// The kind's own evaluation: the core does not know what a rectangle
    /// is, so it asks, and gets an area and a size back.
    fn evaluate(node: &Node, _document: &Document) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "area": node.rect.w * node.rect.h,
            "width": node.rect.w,
            "height": node.rect.h,
        }))
    }
}

/// The rectangle tool.
///
/// A tool is a gesture that runs a command. It is not a command: it has no
/// parameters of its own, it has an icon, and it says which command it
/// runs.
#[derive(Tool)]
#[tool(
    id = "tool.rect",
    label = "Rectangle",
    layer = "geometry",
    icon = "rect",
    command = "geometry.rectangle"
)]
pub struct RectTool;

/// Put a rectangle in the document.
#[derive(Command)]
#[command(id = "geometry.rectangle", area = "geometry", layer = "geometry", icon = "rect", mutates, undoable)]
pub struct AddRectangle {
    /// Where it starts, across.
    pub x: Number,
    /// Where it starts, down.
    pub y: Number,
    /// How wide it is.
    pub width: Number,
    /// How tall it is.
    pub height: Number,
    /// What to call it.
    pub name: Option<Text>,
}

impl AddRectangle {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let rect = Rect::new(self.x.0, self.y.0, self.width.0, self.height.0);
        let name = match &self.name {
            Some(name) => name.0.clone(),
            None => format!("rectangle {}", rect.w),
        };
        let id = session.document_mut().add_node(name.clone(), "geometry.rect", rect);
        Ok(Outcome::changed(format!("added a rectangle `{name}`"))
            .with_data(craftkit_sdk::serde_json::json!({ "id": id, "kind": "geometry.rect" })))
    }
}

/// The panel that shows the shapes.
///
/// A plugin's panel, in the plugin's layer, naming the plugin's command.
#[derive(Panel)]
#[panel(id = "panel.shapes", label = "Shapes", layer = "geometry", commands("geometry.rectangle"))]
pub struct ShapesPanel;

/// The evaluator that measures the document's rectangles.
///
/// An **evaluator** is a contribution like every other: it has an id, it
/// has a layer, and the ecosystem checks the layer is there. It says
/// something about the **whole document**, which is what a kind cannot do:
/// a kind may only speak about its own nodes.
#[derive(Evaluator)]
#[evaluator(id = "geometry.totalarea", label = "Total area", layer = "geometry")]
pub struct AreaEvaluator;

impl AreaEvaluator {
    /// Add up the area of every rectangle.
    fn evaluate(document: &Document) -> Plan {
        let mut plan = Plan::new();
        let area: f64 = document
            .nodes
            .iter()
            .filter(|node| node.kind == "geometry.rect")
            .map(|node| node.rect.w * node.rect.h)
            .sum();
        plan.items.push(PlanItem {
            node: 0,
            kind: "geometry.totalarea".to_string(),
            value: serde_json::json!({ "area": area }),
        });
        plan
    }
}
