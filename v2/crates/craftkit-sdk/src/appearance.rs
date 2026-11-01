//! The **appearance**: how a thing looks.
//!
//! # What it is
//!
//! A node's appearance: the colour it is filled with, and the line on
//! its outline. It is the **kit's own** — a path, a rectangle and a
//! layer can all have one, and two of them can have the *same* one.
//!
//! # Why an appearance is a node
//!
//! Because a **reference** is the only way a document keeps twenty
//! shapes the same colour and changes them all at once. The appearance
//! node holds the values, and the thing that uses one says which node
//! it uses. A copy would be twenty places to change one colour, and one
//! of them would be forgotten.
//!
//! # Who follows the reference
//!
//! The **kind**, when it draws: the paint is given the document, and a
//! kind that cannot see the document cannot follow a reference — which
//! would make every value in the document a copy.

use craftkit_derive::{Command, NodeKind};

use crate::document::{Canvas, Colour, Document, Fill, Node, Point, Stroke};
use crate::format::FormatValue;
use crate::serde_json::{Value as Json, json};
use crate::{Appearance, EngineError, NodeRef, Number, Outcome, Session, Text};

/// An **appearance**: the colours a thing is drawn with.
///
/// It draws nothing and is hit by nothing: an appearance is not a
/// thing, it is how a thing looks.
#[derive(NodeKind)]
#[node_kind(kind = "core.appearance", label = "Appearance", layer = "core", evaluates)]
pub struct AppearanceKind;

impl AppearanceKind {
    /// Not drawn.
    fn paint(_node: &Node, _document: &Document, _canvas: &mut dyn Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _document: &Document, _point: Point) -> bool {
        false
    }

    /// What an appearance **means**: the colours it carries.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        let appearance = node.appearance.clone()?;
        Some(json!({
            "fill": appearance.fill.to_json(),
            "stroke": appearance.stroke.map(|stroke| stroke.to_json()),
        }))
    }
}

/// **Add** an appearance.
///
/// The colours are written as `#rrggbb`, the way a person writes them.
#[derive(Command)]
#[command(id = "core.appearance.add", area = "core", icon = "plus", menu("Appearance"), mutates, undoable)]
pub struct AddAppearance {
    /// Its name.
    pub name: Text,
    /// The colour it is filled with, as `#rrggbb`.
    pub fill: Option<Text>,
    /// The colour of the line on its outline, as `#rrggbb`.
    pub stroke: Option<Text>,
    /// How wide that line is.
    pub stroke_width: Option<Number>,
}

impl AddAppearance {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let name = self.name.0.trim().to_string();
        if name.is_empty() {
            return Err(EngineError::Failed {
                id: "core.appearance.add".to_string(),
                message: "an appearance has a name".to_string(),
            });
        }
        let appearance = appearance_of(
            self.fill.as_ref(),
            self.stroke.as_ref(),
            self.stroke_width.as_ref(),
            None,
            "core.appearance.add",
        )?;
        let id = session.document_mut()?.add_node(name.clone(), "core.appearance", None);
        if let Some(node) = session.document_mut()?.node_mut(id) {
            node.appearance = Some(appearance);
        }
        Ok(Outcome::changed(format!("added the appearance `{name}`"))
            .with_data(json!({ "id": id, "name": name })))
    }
}

/// **Change** an appearance.
///
/// The parts that are not named are **left alone**: a command that
/// changed the outline when it was asked to change the fill would be a
/// command that changed two things.
#[derive(Command)]
#[command(id = "core.appearance.set", area = "core", icon = "pencil", menu("Appearance"), mutates, undoable)]
pub struct SetAppearance {
    /// The appearance node.
    pub node: NodeRef,
    /// The colour it is filled with, as `#rrggbb`.
    pub fill: Option<Text>,
    /// The colour of the line on its outline, as `#rrggbb`.
    pub stroke: Option<Text>,
    /// How wide that line is.
    pub stroke_width: Option<Number>,
}

impl SetAppearance {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = self.node.0;
        let Some(node) = session.document().node(id) else {
            return Err(EngineError::Failed {
                id: "core.appearance.set".to_string(),
                message: format!("there is no node `{id}`"),
            });
        };
        let current = node.appearance.clone();
        let appearance = appearance_of(
            self.fill.as_ref(),
            self.stroke.as_ref(),
            self.stroke_width.as_ref(),
            current,
            "core.appearance.set",
        )?;
        let shown = appearance.to_json();
        if let Some(node) = session.document_mut()?.node_mut(id) {
            node.appearance = Some(appearance);
        }
        Ok(Outcome::changed(format!("changed the appearance `{id}`"))
            .with_data(json!({ "id": id, "appearance": shown })))
    }
}

/// The appearance the parameters describe.
///
/// The parts that are not named are **left alone**, and a part that is
/// named replaces the one it replaces — a command that changed two
/// things at once would be a command nobody could undo in one step.
fn appearance_of(
    fill: Option<&Text>,
    stroke: Option<&Text>,
    stroke_width: Option<&Number>,
    current: Option<Appearance>,
    id: &str,
) -> Result<Appearance, EngineError> {
    // The **current** appearance: the fill is taken out of it, and the
    // stroke is copied, because the fill is not a copy any more — a
    // gradient is a list of stops.
    let current_stroke = current.as_ref().and_then(|appearance| appearance.stroke);
    let current_fill = current.map(|appearance| appearance.fill);
    let fill = match fill {
        Some(fill) => Fill::Solid(colour_of(fill, id)?),
        None => current_fill.unwrap_or(Fill::Solid(Colour::BLACK)),
    };
    let stroke = match stroke {
        Some(stroke) => Some(Stroke {
            colour: colour_of(stroke, id)?,
            width: stroke_width.map(|width| width.0).unwrap_or(1.0),
            ..Default::default()
        }),
        None => current_stroke,
    };
    Ok(Appearance { fill, stroke })
}

/// A colour, as a person writes it.
fn colour_of(text: &Text, id: &str) -> Result<Colour, EngineError> {
    Colour::from_hex(&text.0).map_err(|message| EngineError::Failed { id: id.to_string(), message })
}
