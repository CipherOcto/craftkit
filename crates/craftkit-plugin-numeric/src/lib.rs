//! `craftkit-plugin-numeric` — the **typed value**, as a plugin.
//!
//! # What this is
//!
//! §39 asked what a typed value would look like when it arrived, and the
//! answer was that it arrives as a **plugin** and a **library**, and not
//! as a change to the core. This is that answer, built:
//!
//! - `crates/craftkit-numeric` is the **library**: a [`Length`] in
//!   millimetres and an [`Angle`] in radians, and the arithmetic over
//!   them;
//! - this crate is the **plugin**: the kinds of thing that have a
//!   length or an angle, the commands that make them, and the
//!   evaluation that says what they measure.
//!
//! # What it proves
//!
//! - **The core has no unit, and it never will.** A length in a
//!   document is a **number** in a payload, and the number is
//!   millimetres because *this plugin* says so. The core reads the
//!   payload as JSON and knows nothing about it.
//! - **The typed value is not a command parameter.** The kinds a
//!   parameter may take are the **core's** — text, number, bool, a node
//!   reference — and a plugin cannot add one. So a command takes a
//!   number and a **unit name**, and the conversion happens in the
//!   command's body, in the library. That is the one thing that would
//!   need the core to change, and it is named here rather than worked
//!   around silently.
//! - **The other units are derived.** A length is stored once, in
//!   millimetres, and the centimetres, the inches and the points are
//!   worked out every time by the evaluation.
//!
//! # What is not here
//!
//! No unit is in the core, no unit is in the document format, and no
//! command parameter is typed. What is here is a plugin that says what
//! a number means — which is the whole of §39's answer.

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
use craftkit_numeric::{Angle, Length, NumericError};
use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Node, NodeRef, Number, Outcome, Plan, PlanItem, Session};

/// The numeric layer.
///
/// It needs the core, and it says so.
#[derive(Layer)]
#[layer(id = "numeric", name = "Numeric", requires("core"))]
pub struct NumericLayer;

/// A **length**: a number of millimetres.
#[derive(NodeKind)]
#[node_kind(kind = "numeric.length", label = "Length", layer = "numeric", evaluates)]
pub struct LengthKind;

impl LengthKind {
    /// Not drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a length **means**: its millimetres, and the other units
    /// they are.
    ///
    /// The other units are **derived**: the millimetres are stored once.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        let millimetres = node.data.get("millimetres").and_then(|value| value.as_f64()).unwrap_or(0.0);
        let Ok(length) = Length::from_number(millimetres) else { return None };
        Some(json!({
            "millimetres": length.as_millimetres(),
            "centimetres": length.as_centimetres(),
            "metres": length.as_metres(),
            "inches": length.as_inches(),
            "points": length.as_points(),
        }))
    }
}

/// An **angle**: a number of radians.
#[derive(NodeKind)]
#[node_kind(kind = "numeric.angle", label = "Angle", layer = "numeric", evaluates)]
pub struct AngleKind;

impl AngleKind {
    /// Not drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What an angle **means**: its radians, its degrees, and the angle
    /// turned into a turn.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        let radians = node.data.get("radians").and_then(|value| value.as_f64()).unwrap_or(0.0);
        let Ok(angle) = Angle::from_number(radians) else { return None };
        Some(json!({
            "radians": angle.as_radians(),
            "degrees": angle.as_degrees(),
            "normalized": angle.normalized().as_radians(),
        }))
    }
}

/// A **box**: two lengths, and the area they make.
#[derive(NodeKind)]
#[node_kind(kind = "numeric.box", label = "Box", layer = "numeric", evaluates)]
pub struct BoxKind;

impl BoxKind {
    /// Not drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a box **means**: its two lengths, and the area.
    ///
    /// The area is **derived**, and it is the library's arithmetic and
    /// not the core's.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        let width = node.data.get("width_mm").and_then(|value| value.as_f64()).unwrap_or(0.0);
        let height = node.data.get("height_mm").and_then(|value| value.as_f64()).unwrap_or(0.0);
        let (Ok(width), Ok(height)) = (Length::from_number(width), Length::from_number(height)) else {
            return None;
        };
        let area = width.scale(height.as_millimetres()).ok()?;
        Some(json!({
            "width_mm": width.as_millimetres(),
            "height_mm": height.as_millimetres(),
            "area_mm2": area.as_millimetres(),
        }))
    }
}

/// Put a **length** in the document.
///
/// The parameter is a **length**, not a number: the library's own type.
/// A caller is told the unit in the schema — the number is in
/// millimetres — and a string with a unit (`2.5cm`) is read as well, so
/// that a person does not have to convert in their head.
#[derive(Command)]
#[command(id = "numeric.length.add", area = "numeric", layer = "numeric", icon = "plus", mutates, undoable)]
pub struct AddLength {
    /// How long it is: a number of millimetres, or a string with a unit
    /// (`2.5cm`).
    pub value: Length,
}

impl AddLength {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let length = self.value;
        let id = session.document_mut()?.add_node_with_under(
            None,
            format!("{} mm", length.as_millimetres()),
            "numeric.length",
            None,
            json!({ "millimetres": length.as_millimetres() }),
        );
        Ok(Outcome::changed(format!("a length of {} mm", length.as_millimetres()))
            .with_data(json!({ "id": id, "millimetres": length.as_millimetres() })))
    }
}

/// Put an **angle** in the document.
#[derive(Command)]
#[command(id = "numeric.angle.add", area = "numeric", layer = "numeric", icon = "plus", mutates, undoable)]
pub struct AddAngle {
    /// How far it turns: a number of radians, or a string with a unit
    /// (`180deg`).
    pub value: Angle,
}

impl AddAngle {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let angle = self.value;
        let id = session.document_mut()?.add_node_with_under(
            None,
            format!("{} rad", angle.as_radians()),
            "numeric.angle",
            None,
            json!({ "radians": angle.as_radians() }),
        );
        Ok(Outcome::changed(format!("an angle of {} rad", angle.as_radians()))
            .with_data(json!({ "id": id, "radians": angle.as_radians() })))
    }
}

/// Put a **box** in the document: two lengths in millimetres.
#[derive(Command)]
#[command(id = "numeric.box.add", area = "numeric", layer = "numeric", icon = "plus", mutates, undoable)]
pub struct AddBox {
    /// How wide it is: a number of millimetres, or a string with a unit.
    pub width: Length,
    /// How high it is: a number of millimetres, or a string with a unit.
    pub height: Length,
}

impl AddBox {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let width = self.width;
        let height = self.height;
        let id = session.document_mut()?.add_node_with_under(
            None,
            format!("{} by {} mm", width.as_millimetres(), height.as_millimetres()),
            "numeric.box",
            None,
            json!({ "width_mm": width.as_millimetres(), "height_mm": height.as_millimetres() }),
        );
        Ok(Outcome::changed(format!("a box of {} by {} mm", width.as_millimetres(), height.as_millimetres()))
            .with_data(
                json!({ "id": id, "width_mm": width.as_millimetres(), "height_mm": height.as_millimetres() }),
            ))
    }
}

/// **Scale** a length: the library's arithmetic, written back.
///
/// The core cannot do this: it has no unit and no type, so it cannot
/// tell a length from a factor. Here the length is read as a
/// millimetre count, the library does the arithmetic, and the result is
/// written back as a millimetre count.
#[derive(Command)]
#[command(id = "numeric.length.scale", area = "numeric", layer = "numeric", icon = "list", mutates, undoable)]
pub struct ScaleLength {
    /// The length.
    pub length: NodeRef,
    /// What to multiply it by.
    pub factor: Number,
}

impl ScaleLength {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = self.length.0;
        let Some(node) = session.document().node(id) else {
            return Err(EngineError::Failed {
                id: "numeric.length.scale".to_string(),
                message: format!("there is no length `{id}`"),
            });
        };
        if node.kind != "numeric.length" {
            return Err(EngineError::Failed {
                id: "numeric.length.scale".to_string(),
                message: format!("the node `{id}` is not a length"),
            });
        }
        let millimetres = node.data.get("millimetres").and_then(|value| value.as_f64()).unwrap_or(0.0);
        let length =
            Length::from_number(millimetres).map_err(|error| refused("numeric.length.scale", error))?;
        let scaled = length.scale(self.factor.0).map_err(|error| refused("numeric.length.scale", error))?;
        let Some(node) = session.document_mut()?.node_mut(id) else {
            return Err(EngineError::Failed {
                id: "numeric.length.scale".to_string(),
                message: format!("there is no length `{id}`"),
            });
        };
        node.data["millimetres"] = json!(scaled.as_millimetres());
        node.name = format!("{} mm", scaled.as_millimetres());
        Ok(Outcome::changed(format!("scaled the length to {} mm", scaled.as_millimetres()))
            .with_data(json!({ "id": id, "millimetres": scaled.as_millimetres() })))
    }
}

/// The **total** of some lengths: the library's arithmetic again.
#[derive(Command)]
#[command(id = "numeric.length.total", area = "numeric", layer = "numeric", icon = "plus", mutates, undoable)]
pub struct TotalLengths {
    /// The lengths, in millimetres.
    pub lengths: Json,
}

impl TotalLengths {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let Some(lengths) = self.lengths.as_array() else {
            return Err(EngineError::Failed {
                id: "numeric.length.total".to_string(),
                message: "the lengths are a list".to_string(),
            });
        };
        if lengths.len() > 100_000 {
            return Err(EngineError::Failed {
                id: "numeric.length.total".to_string(),
                message: "a total does not hold a hundred thousand lengths".to_string(),
            });
        }
        let mut total = Length::zero();
        for value in lengths {
            let Some(millimetres) = value.as_f64() else {
                return Err(EngineError::Failed {
                    id: "numeric.length.total".to_string(),
                    message: "a length is a number".to_string(),
                });
            };
            let length =
                Length::from_number(millimetres).map_err(|error| refused("numeric.length.total", error))?;
            total = total.checked_add(length).map_err(|error| refused("numeric.length.total", error))?;
        }
        let id = session.document_mut()?.add_node_with_under(
            None,
            format!("{} mm", total.as_millimetres()),
            "numeric.length",
            None,
            json!({ "millimetres": total.as_millimetres() }),
        );
        Ok(Outcome::changed(format!("a total of {} mm", total.as_millimetres()))
            .with_data(json!({ "id": id, "millimetres": total.as_millimetres() })))
    }
}

/// The evaluator: what the numeric things **measure**.
///
/// The stored number is millimetres or radians, and every other unit is
/// **derived**.
#[derive(Evaluator)]
#[evaluator(id = "numeric.measures", label = "Measures", layer = "numeric")]
pub struct MeasuresEvaluator;

impl MeasuresEvaluator {
    /// The measures of every numeric node.
    fn evaluate(document: &Document) -> Plan {
        let mut plan = Plan::new();
        for node in document.walk() {
            if !node.kind.starts_with("numeric.") {
                continue;
            }
            let value = match node.kind.as_str() {
                "numeric.length" => Length::from_number(
                    node.data.get("millimetres").and_then(|value| value.as_f64()).unwrap_or(0.0),
                )
                .ok()
                .map(|length| {
                    json!({
                        "millimetres": length.as_millimetres(),
                        "centimetres": length.as_centimetres(),
                        "metres": length.as_metres(),
                        "inches": length.as_inches(),
                        "points": length.as_points(),
                    })
                }),
                "numeric.angle" => Angle::from_number(
                    node.data.get("radians").and_then(|value| value.as_f64()).unwrap_or(0.0),
                )
                .ok()
                .map(|angle| {
                    json!({
                        "radians": angle.as_radians(),
                        "degrees": angle.as_degrees(),
                        "normalized": angle.normalized().as_radians(),
                    })
                }),
                "numeric.box" => {
                    let width = Length::from_number(
                        node.data.get("width_mm").and_then(|value| value.as_f64()).unwrap_or(0.0),
                    )
                    .ok();
                    let height = Length::from_number(
                        node.data.get("height_mm").and_then(|value| value.as_f64()).unwrap_or(0.0),
                    )
                    .ok();
                    match (width, height) {
                        (Some(width), Some(height)) => {
                            width.scale(height.as_millimetres()).ok().map(|area| {
                                json!({
                                    "width_mm": width.as_millimetres(),
                                    "height_mm": height.as_millimetres(),
                                    "area_mm2": area.as_millimetres(),
                                })
                            })
                        }
                        _ => None,
                    }
                }
                _ => None,
            };
            let Some(value) = value else { continue };
            plan.items.push(PlanItem { node: node.id, kind: "numeric.measures".to_string(), value });
        }
        plan
    }
}

/// A number that the library refused, said the way this kit says it.
fn refused(id: &str, error: NumericError) -> EngineError {
    EngineError::Failed { id: id.to_string(), message: error.to_string() }
}

/// The panel that shows the measures.
#[derive(Panel)]
#[panel(
    id = "panel.numeric",
    label = "Numeric",
    layer = "numeric",
    commands("numeric.length.add", "numeric.angle.add", "numeric.box.add")
)]
pub struct NumericPanel;

// The whole of the battery, on every target.
craftkit_sdk::plugin_anchor!(
    NumericLayer,
    LengthKind,
    AngleKind,
    BoxKind,
    AddLength,
    AddAngle,
    AddBox,
    ScaleLength,
    TotalLengths,
    MeasuresEvaluator,
    NumericPanel
);
