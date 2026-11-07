//! The **DXF** read back: a drawing file made into a drawing.
//!
//! DXF is a **text** of group codes: every other line is the value of
//! the code above it. This module reads one and makes the drawing it
//! names.
//!
//! # The rules
//!
//! - The file is read through a **bound**: at most **sixteen
//!   mebibytes**.
//! - A file that is **not DXF** is refused: a line that is not a
//!   group code is not a DXF file, and so is a file that ends in the
//!   middle of a tag.
//! - The **layers** of the drawing are the `LAYER` records of the
//!   file's tables, with their colour indices.
//! - The **entities** are the records of the file's `ENTITIES`
//!   section, and they go into the block the tree names
//!   `*Model_Space` — the tree's own name for the block a drawing's
//!   entities live in.
//! - The **numbers** of a file are the file's own numbers: the tree
//!   does not scale them on the way in, and the unit a file declares
//!   is a fact of the file, not of the drawing.
//! - An entity that names a layer the file does not define gets that
//!   layer **made**, because an entity on a layer the drawing does
//!   not have is an entity on nothing.
//! - An entity this tree cannot read is **skipped**, and the answer
//!   says what it skipped.
//!
//! # What it does not do
//!
//! - The **blocks**, the **inserts**, the **text**, the **dimensions**
//!   and the **hatches** of a file are not read, and the **handles**
//!   are not kept.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Outcome, Relation, Session, Text};

/// The most a DXF file may be, in bytes.
const MAX_BYTES: u64 = 16 * 1024 * 1024;

/// The **name** of the block a drawing's entities live in: the tree's
/// own name.
const MODEL_SPACE: &str = "*Model_Space";

/// The name of the command, in the messages.
const COMMAND: &str = "cad.import.dxf";

/// One **tag** of a DXF file: a code and its value.
#[derive(Clone)]
struct Tag {
    /// The group code.
    code: i64,
    /// The value of the code.
    value: String,
}

/// The **tags** of a file, read from its text.
///
/// Every other line is a value: a line that is not a group code is
/// not a DXF file.
fn tags_of(text: &str) -> Result<Vec<Tag>, String> {
    let lines: Vec<&str> = text.lines().map(|line| line.trim()).filter(|line| !line.is_empty()).collect();
    let mut tags = Vec::with_capacity(lines.len() / 2);
    let mut at = 0usize;
    while at < lines.len() {
        let code =
            lines[at].parse::<i64>().map_err(|_| format!("the line `{}` is not a group code", lines[at]))?;
        let Some(value) = lines.get(at + 1) else {
            return Err("the file ends in the middle of a tag".to_string());
        };
        tags.push(Tag { code, value: (*value).to_string() });
        at += 2;
    }
    Ok(tags)
}

/// The **sections** of a file, by name.
fn sections_of(tags: &[Tag]) -> Vec<(String, Vec<Tag>)> {
    let mut sections = Vec::new();
    let mut at = 0usize;
    while at < tags.len() {
        let is_section = tags[at].code == 0 && tags[at].value == "SECTION";
        if !is_section {
            at += 1;
            continue;
        }
        let name =
            tags.get(at + 1).filter(|tag| tag.code == 2).map(|tag| tag.value.clone()).unwrap_or_default();
        let mut body = Vec::new();
        let mut cursor = at + 2;
        while cursor < tags.len() {
            let ends =
                tags[cursor].code == 0 && (tags[cursor].value == "ENDSEC" || tags[cursor].value == "EOF");
            if ends {
                break;
            }
            body.push(tags[cursor].clone());
            cursor += 1;
        }
        sections.push((name, body));
        at = cursor + 1;
    }
    sections
}

/// The **records** of a section: each one starts with a tag of code
/// zero.
fn records_of(body: &[Tag]) -> Vec<(String, Vec<Tag>)> {
    let mut records: Vec<(String, Vec<Tag>)> = Vec::new();
    let mut current: Option<(String, Vec<Tag>)> = None;
    for tag in body {
        if tag.code == 0 {
            if let Some(record) = current.take() {
                records.push(record);
            }
            current = Some((tag.value.clone(), Vec::new()));
            continue;
        }
        if let Some((_, tags)) = current.as_mut() {
            tags.push(tag.clone());
        }
    }
    if let Some(record) = current.take() {
        records.push(record);
    }
    records
}

/// The **layers** of a file: its `LAYER` records, with their colour
/// indices.
fn layers_of(body: &[Tag]) -> Vec<(String, Option<i64>)> {
    let mut layers = Vec::new();
    for (kind, tags) in records_of(body) {
        if kind != "LAYER" {
            continue;
        }
        let Some(name) = tags.iter().find(|tag| tag.code == 2).map(|tag| tag.value.clone()) else {
            continue;
        };
        let color = tags.iter().find(|tag| tag.code == 62).and_then(|tag| tag.value.parse::<i64>().ok());
        layers.push((name, color));
    }
    layers
}

/// The **number** a tag holds, when it holds one.
fn number_of(tags: &[Tag], code: i64) -> Option<f64> {
    tags.iter().find(|tag| tag.code == code).and_then(|tag| tag.value.trim().parse::<f64>().ok())
}

/// The **point** a pair of tags holds.
fn point_of(tags: &[Tag], x: i64, y: i64) -> Option<(f64, f64)> {
    let x = number_of(tags, x)?;
    let y = number_of(tags, y)?;
    if !x.is_finite() || !y.is_finite() {
        return None;
    }
    Some((x, y))
}

/// The **name** of a tag, when it holds one.
fn text_of(tags: &[Tag], code: i64) -> Option<String> {
    tags.iter().find(|tag| tag.code == code).map(|tag| tag.value.clone())
}

/// The **geometry** of an entity, as this tree holds it.
///
/// The answer is the kind of the entity and its geometry, and `None`
/// when this tree cannot read the entity.
fn geometry_of(kind: &str, tags: &[Tag]) -> Option<(String, Json)> {
    match kind {
        "LINE" => {
            let from = point_of(tags, 10, 20)?;
            let to = point_of(tags, 11, 21)?;
            Some((
                "cad.entity.line".to_string(),
                json!({
                    "from": { "x": from.0, "y": from.1 },
                    "to": { "x": to.0, "y": to.1 },
                }),
            ))
        }
        "CIRCLE" => {
            let center = point_of(tags, 10, 20)?;
            let radius = number_of(tags, 40)?;
            if !radius.is_finite() || radius <= 0.0 {
                return None;
            }
            Some((
                "cad.entity.circle".to_string(),
                json!({ "center": { "x": center.0, "y": center.1 }, "radius": radius }),
            ))
        }
        "ARC" => {
            let center = point_of(tags, 10, 20)?;
            let radius = number_of(tags, 40)?;
            if !radius.is_finite() || radius <= 0.0 {
                return None;
            }
            let mut start = number_of(tags, 50)?;
            let mut end = number_of(tags, 51)?;
            if !start.is_finite() || !end.is_finite() {
                return None;
            }
            // The **degrees** of a file are **radians** here, and an
            // arc goes **counter-clockwise**: an end that is before
            // the start is the next turn round.
            start = start.to_radians();
            end = end.to_radians();
            if end <= start {
                end += std::f64::consts::TAU;
            }
            Some((
                "cad.entity.arc".to_string(),
                json!({
                    "center": { "x": center.0, "y": center.1 },
                    "radius": radius,
                    "start": start,
                    "end": end,
                }),
            ))
        }
        "LWPOLYLINE" => {
            let mut points: Vec<Json> = Vec::new();
            let mut x: Option<f64> = None;
            for tag in tags {
                if tag.code == 10 {
                    x = tag.value.trim().parse::<f64>().ok();
                    continue;
                }
                if tag.code == 20 {
                    let Some(x) = x else { continue };
                    let Ok(y) = tag.value.trim().parse::<f64>() else { continue };
                    if !x.is_finite() || !y.is_finite() {
                        continue;
                    }
                    points.push(json!({ "x": x, "y": y }));
                }
            }
            if points.len() < 2 {
                return None;
            }
            // The **flags**: the first bit says the polyline is
            // closed.
            let closed = number_of(tags, 70).map(|flags| flags as i64 & 1 == 1).unwrap_or(false);
            Some(("cad.entity.polyline".to_string(), json!({ "points": points, "closed": closed })))
        }
        _ => None,
    }
}

/// **Read** a DXF file and make the **drawing** it names.
///
/// The layers of the file are layers of the drawing, and the entities
/// of the file are entities of its model space.
#[derive(Command)]
#[command(id = "cad.import.dxf", area = "cad", layer = "cad", icon = "file", mutates, undoable)]
pub struct ImportDxf {
    /// The file to read.
    pub path: Text,
    /// What to call the drawing.
    ///
    /// Without it, the name of the file.
    pub name: Option<Text>,
}

impl ImportDxf {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let path = self.path.0.trim().to_string();
        if path.is_empty() {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "a file has a path".to_string(),
            });
        }
        let text = craftkit_sdk::files::read_text_bounded(std::path::Path::new(&path), MAX_BYTES).map_err(
            |message| EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("the file cannot be read: {message}"),
            },
        )?;
        let tags = tags_of(&text).map_err(|reason| EngineError::Failed {
            id: COMMAND.to_string(),
            message: format!("the file is not a DXF file: {reason}"),
        })?;
        let sections = sections_of(&tags);
        if sections.is_empty() {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "the file is not a DXF file: it holds no section".to_string(),
            });
        }
        // The **name** of the drawing.
        let name = match &self.name {
            Some(name) if !name.0.trim().is_empty() => name.0.trim().to_string(),
            _ => std::path::Path::new(&path)
                .file_stem()
                .map(|stem| stem.to_string_lossy().to_string())
                .unwrap_or_else(|| "the drawing".to_string()),
        };
        // The **drawing**, its **layers** and its **block**.
        let drawing = session.document_mut()?.add_node_under(None, name.clone(), "cad.drawing", None);
        let mut layers: Vec<(String, u64)> = Vec::new();
        for (layer_name, color) in sections
            .iter()
            .find(|(name, _)| name == "TABLES")
            .map(|(_, body)| layers_of(body))
            .unwrap_or_default()
        {
            let color = color.map(|color| color.to_string());
            let layer = session.document_mut()?.add_node_with_under(
                Some(drawing),
                layer_name.clone(),
                "cad.layer",
                None,
                json!({ "color": color }),
            );
            layers.push((layer_name, layer));
        }
        let block = session.document_mut()?.add_node_with_under(
            Some(drawing),
            MODEL_SPACE.to_string(),
            "cad.block",
            None,
            json!({}),
        );
        // The **entities**.
        let mut entities = 0usize;
        let mut skipped: Vec<String> = Vec::new();
        let mut counts: Vec<(String, usize)> = Vec::new();
        for (kind, tags) in sections
            .iter()
            .find(|(name, _)| name == "ENTITIES")
            .map(|(_, body)| records_of(body))
            .unwrap_or_default()
        {
            let Some((entity_kind, geometry)) = geometry_of(&kind, &tags) else {
                skipped.push(kind.clone());
                continue;
            };
            let plain = entity_kind.trim_start_matches("cad.entity.").to_string();
            let count = match counts.iter_mut().find(|(name, _)| *name == plain) {
                Some((_, count)) => {
                    *count += 1;
                    *count
                }
                None => {
                    counts.push((plain.clone(), 1));
                    1
                }
            };
            let name = format!("{plain} {count}");
            let entity = session.document_mut()?.add_node_with_under(
                Some(block),
                name,
                &entity_kind,
                None,
                json!({ "geometry": geometry }),
            );
            // The **layer** of the entity: a relation, as it is
            // everywhere else in this tree.
            if let Some(layer_name) = text_of(&tags, 8) {
                let layer = match layers.iter().find(|(name, _)| *name == layer_name) {
                    Some((_, layer)) => *layer,
                    None => {
                        let layer = session.document_mut()?.add_node_with_under(
                            Some(drawing),
                            layer_name.clone(),
                            "cad.layer",
                            None,
                            json!({ "color": Json::Null }),
                        );
                        layers.push((layer_name.clone(), layer));
                        layer
                    }
                };
                let relation = Relation::with_payload(
                    entity,
                    layer,
                    craftkit_sdk::layer::ON_LAYER,
                    json!({ "layer": layer_name }),
                );
                session.document_mut()?.add_relation(relation);
            }
            entities += 1;
        }
        let units = sections
            .iter()
            .find(|(name, _)| name == "HEADER")
            .and_then(|(_, body)| {
                body.iter()
                    .find(|tag| tag.code == 9 && tag.value.trim() == "$INSUNITS")
                    .and_then(|_| body.iter().find(|tag| tag.code == 70))
            })
            .and_then(|tag| tag.value.trim().parse::<i64>().ok());
        Ok(Outcome::changed(format!("the file `{path}` made {entities} entit(ies)")).with_data(json!({
            "path": path,
            "drawing": drawing,
            "block": block,
            "layers": layers.len(),
            "entities": entities,
            "units": units,
            "skipped": skipped,
        })))
    }
}
