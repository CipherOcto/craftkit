//! The **DXF** written out: a drawing made into a drawing file.
//!
//! This is the other half of the reader of §625: the same codes, the
//! same order and the same rules, read back into the same drawing.
//!
//! # The rules
//!
//! - The file is a DXF: the `HEADER`, the `TABLES` with the layers,
//!   the `ENTITIES` and the `EOF`.
//! - The **version** is the tree's own — `AC1032` — and the **units**
//!   are millimetres, which is what the tree's new drawings say.
//! - The **colour** of a layer is its index, and a layer that is off
//!   has a **negative** colour: the tree's own way of saying it.
//! - The **flags** of a layer are the tree's own: one is frozen, four
//!   is locked.
//! - The **angles** of an arc are **degrees**, because a file counts
//!   in degrees and this tree counts in radians.
//! - An entity this tree cannot write is **left out**, and the answer
//!   says what it left out.
//!
//! # What it does not do
//!
//! - The **blocks** of a drawing are not written as blocks: the
//!   entities of one block — the model space, or the first block of a
//!   drawing that has no model space — are the entities of the file.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Outcome, Session, Text};

/// The name of the command, in the messages.
const COMMAND: &str = "cad.export.dxf";

/// The **version** a file says: the tree's own.
const VERSION: &str = "AC1032";

/// The **units** a file says: millimetres, which is what the tree's
/// new drawings say.
const UNITS: i64 = 4;

/// The **colour** of a layer that says nothing: the tree's own.
const DEFAULT_COLOR: i64 = 7;

/// The **number** a geometry holds.
fn number_of(geometry: &Json, name: &str) -> Option<f64> {
    geometry.get(name).and_then(|value| value.as_f64()).filter(|value| value.is_finite())
}

/// The **point** a geometry holds.
fn point_of(geometry: &Json, name: &str) -> Option<(f64, f64)> {
    let point = geometry.get(name)?;
    let x = point.get("x")?.as_f64()?;
    let y = point.get("y")?.as_f64()?;
    if !x.is_finite() || !y.is_finite() {
        return None;
    }
    Some((x, y))
}

/// The **lines** of the file, as they are written.
fn lines_of(out: &mut Vec<String>, code: i64, value: impl Into<String>) {
    out.push(code.to_string());
    out.push(value.into());
}

/// The **layers** of a drawing, as the table of a file.
///
/// The answer is the lines and how many layers there are.
fn layers_of(drawing: &Node) -> (Vec<String>, usize) {
    let mut out: Vec<String> = Vec::new();
    let mut count = 0usize;
    for layer in drawing.children.iter().filter(|child| child.kind == "cad.layer") {
        let color = layer
            .data
            .get("color")
            .and_then(|color| color.as_str())
            .and_then(|color| color.trim().parse::<i64>().ok())
            .unwrap_or(DEFAULT_COLOR);
        let on = layer.data.get("on").and_then(|on| on.as_bool()).unwrap_or(true);
        let frozen = layer.data.get("frozen").and_then(|frozen| frozen.as_bool()).unwrap_or(false);
        let locked = layer.data.get("locked").and_then(|locked| locked.as_bool()).unwrap_or(false);
        let mut flags = 0i64;
        if frozen {
            flags |= 1;
        }
        if locked {
            flags |= 4;
        }
        let linetype = layer
            .data
            .get("linetype")
            .and_then(|linetype| linetype.as_str())
            .filter(|linetype| !linetype.trim().is_empty())
            .unwrap_or("Continuous");
        lines_of(&mut out, 0, "LAYER");
        lines_of(&mut out, 2, layer.name.clone());
        lines_of(&mut out, 70, flags.to_string());
        lines_of(&mut out, 62, if on { color } else { -color }.to_string());
        lines_of(&mut out, 6, linetype);
        count += 1;
    }
    (out, count)
}

/// The **entities** of a block, as the section of a file.
///
/// The answer is the lines, how many entities they hold and the
/// entities that were left out.
fn entities_of(document: &craftkit_sdk::Document, block: &Node) -> (Vec<String>, usize, Vec<String>) {
    let mut out: Vec<String> = Vec::new();
    let mut count = 0usize;
    let mut left_out: Vec<String> = Vec::new();
    for entity in block.children.iter() {
        let geometry = entity.data.get("geometry").cloned().unwrap_or(Json::Null);
        // The **layer** of the entity: the relation says it.
        let layer = document
            .relations_from(entity.id, Some(craftkit_sdk::layer::ON_LAYER))
            .first()
            .and_then(|relation| document.node(relation.to))
            .map(|layer| layer.name.clone())
            .unwrap_or_else(|| "0".to_string());
        match entity.kind.as_str() {
            "cad.entity.line" => {
                let (Some(from), Some(to)) = (point_of(&geometry, "from"), point_of(&geometry, "to")) else {
                    left_out.push(entity.name.clone());
                    continue;
                };
                lines_of(&mut out, 0, "LINE");
                lines_of(&mut out, 8, layer);
                lines_of(&mut out, 10, format!("{}", from.0));
                lines_of(&mut out, 20, format!("{}", from.1));
                lines_of(&mut out, 30, "0.0");
                lines_of(&mut out, 11, format!("{}", to.0));
                lines_of(&mut out, 21, format!("{}", to.1));
                lines_of(&mut out, 31, "0.0");
            }
            "cad.entity.circle" => {
                let (Some(center), Some(radius)) =
                    (point_of(&geometry, "center"), number_of(&geometry, "radius"))
                else {
                    left_out.push(entity.name.clone());
                    continue;
                };
                lines_of(&mut out, 0, "CIRCLE");
                lines_of(&mut out, 8, layer);
                lines_of(&mut out, 10, format!("{}", center.0));
                lines_of(&mut out, 20, format!("{}", center.1));
                lines_of(&mut out, 30, "0.0");
                lines_of(&mut out, 40, format!("{}", radius));
            }
            "cad.entity.arc" => {
                let (Some(center), Some(radius), Some(start), Some(end)) = (
                    point_of(&geometry, "center"),
                    number_of(&geometry, "radius"),
                    number_of(&geometry, "start"),
                    number_of(&geometry, "end"),
                ) else {
                    left_out.push(entity.name.clone());
                    continue;
                };
                lines_of(&mut out, 0, "ARC");
                lines_of(&mut out, 8, layer);
                lines_of(&mut out, 10, format!("{}", center.0));
                lines_of(&mut out, 20, format!("{}", center.1));
                lines_of(&mut out, 30, "0.0");
                lines_of(&mut out, 40, format!("{}", radius));
                lines_of(&mut out, 50, format!("{}", start.to_degrees()));
                lines_of(&mut out, 51, format!("{}", end.to_degrees()));
            }
            "cad.entity.polyline" => {
                let Some(points) = geometry.get("points").and_then(|points| points.as_array()) else {
                    left_out.push(entity.name.clone());
                    continue;
                };
                let mut vertices: Vec<(f64, f64)> = Vec::new();
                for point in points {
                    let Some(x) = point.get("x").and_then(|x| x.as_f64()) else { continue };
                    let Some(y) = point.get("y").and_then(|y| y.as_f64()) else { continue };
                    if !x.is_finite() || !y.is_finite() {
                        continue;
                    }
                    vertices.push((x, y));
                }
                if vertices.len() < 2 {
                    left_out.push(entity.name.clone());
                    continue;
                }
                let closed = geometry.get("closed").and_then(|closed| closed.as_bool()).unwrap_or(false);
                lines_of(&mut out, 0, "LWPOLYLINE");
                lines_of(&mut out, 8, layer);
                lines_of(&mut out, 90, vertices.len().to_string());
                lines_of(&mut out, 70, if closed { "1" } else { "0" });
                for (x, y) in vertices {
                    lines_of(&mut out, 10, format!("{}", x));
                    lines_of(&mut out, 20, format!("{}", y));
                }
            }
            _ => left_out.push(entity.name.clone()),
        }
        count += 1;
    }
    (out, count, left_out)
}

/// **Write** a drawing as a **DXF file**.
///
/// The file is written where the caller says, and the answer says how
/// many entities it holds and what it left out.
#[derive(Command)]
#[command(id = "cad.export.dxf", area = "cad", layer = "cad", icon = "file")]
pub struct ExportDxf {
    /// Where to write the file.
    pub path: Text,
    /// The drawing to write.
    pub drawing: NodeRef,
}

impl ExportDxf {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let path = self.path.0.trim().to_string();
        if path.is_empty() {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "a file has a path".to_string(),
            });
        }
        let drawing_id = self.drawing.0;
        let Some(drawing) = session.document().node(drawing_id).cloned() else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("there is no drawing `{drawing_id}`"),
            });
        };
        if drawing.kind != "cad.drawing" {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("the node `{drawing_id}` is a `{}` and not a drawing", drawing.kind),
            });
        }
        // The **block** the entities come from: the model space, or
        // the first block of a drawing that has no model space.
        let blocks: Vec<std::sync::Arc<Node>> =
            drawing.children.iter().filter(|child| child.kind == "cad.block").cloned().collect();
        let model = blocks.iter().find(|block| block.name == "*Model_Space");
        let chosen = model.or_else(|| blocks.first());
        let (entities, entity_count, left_out) = match chosen {
            Some(block) => entities_of(session.document(), block),
            None => (Vec::new(), 0, Vec::new()),
        };
        // The **file**.
        let mut out: Vec<String> = Vec::new();
        lines_of(&mut out, 0, "SECTION");
        lines_of(&mut out, 2, "HEADER");
        lines_of(&mut out, 9, "$ACADVER");
        lines_of(&mut out, 1, VERSION);
        lines_of(&mut out, 9, "$INSUNITS");
        lines_of(&mut out, 70, UNITS.to_string());
        lines_of(&mut out, 0, "ENDSEC");
        lines_of(&mut out, 0, "SECTION");
        lines_of(&mut out, 2, "TABLES");
        lines_of(&mut out, 0, "TABLE");
        lines_of(&mut out, 2, "LAYER");
        let (layers, layer_count) = layers_of(&drawing);
        lines_of(&mut out, 70, layer_count.to_string());
        out.extend(layers);
        lines_of(&mut out, 0, "ENDTAB");
        lines_of(&mut out, 0, "ENDSEC");
        lines_of(&mut out, 0, "SECTION");
        lines_of(&mut out, 2, "ENTITIES");
        out.extend(entities);
        lines_of(&mut out, 0, "ENDSEC");
        lines_of(&mut out, 0, "EOF");
        let text = out.join("\n") + "\n";
        std::fs::write(&path, text.as_bytes()).map_err(|error| EngineError::Failed {
            id: COMMAND.to_string(),
            message: format!("the file cannot be written to `{path}`: {error}"),
        })?;
        Ok(Outcome::new(format!("wrote {path}")).with_data(json!({
            "path": path,
            "layers": layer_count,
            "entities": entity_count,
            "left_out": left_out,
            "bytes": text.len(),
        })))
    }
}
