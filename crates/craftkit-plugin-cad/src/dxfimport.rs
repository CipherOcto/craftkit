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
//! - The **blocks** of the file's `BLOCKS` section are blocks of the
//!   drawing, and their entities are entities of their block. The
//!   model's own record is the model space, and its entities are the
//!   entities of the `ENTITIES` section — the tree's own writer puts
//!   them there — so they are read only when the file has none of
//!   its own.
//! - An **insert** is an instance of a block, and the instance and
//!   its definition are two things: the reference between them is an
//!   **edge**, and it says which block is inserted.
//! - The **numbers** of a file are the file's own numbers: the tree
//!   does not scale them on the way in, and the unit a file declares
//!   is a fact of the file, not of the drawing.
//! - An entity that names a layer the file does not define gets that
//!   layer **made**, because an entity on a layer the drawing does
//!   not have is an entity on nothing.
//! - An entity this tree cannot read is **skipped**, and the answer
//!   says what it skipped. So is an insert of a block the file does
//!   not define: an insert of nothing is an insert of nothing.
//!
//! # What it does not do
//!
//! - The **text**, the **dimensions**, the **hatches** and the
//!   **splines** of a file are not read, and the **handles** are not
//!   kept.

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

/// One **record** of a section: its kind and its tags.
type Record = (String, Vec<Tag>);

/// The **blocks** of a file: its `BLOCK` records, with the entities
/// that are in them.
fn blocks_of(body: &[Tag]) -> Vec<(String, Vec<Record>)> {
    let records = records_of(body);
    let mut blocks: Vec<(String, Vec<Record>)> = Vec::new();
    let mut at = 0usize;
    while at < records.len() {
        let (kind, tags) = &records[at];
        if kind != "BLOCK" {
            at += 1;
            continue;
        }
        let name = text_of(tags, 2).unwrap_or_default();
        let mut entities: Vec<Record> = Vec::new();
        let mut cursor = at + 1;
        while cursor < records.len() && records[cursor].0 != "ENDBLK" {
            entities.push(records[cursor].clone());
            cursor += 1;
        }
        blocks.push((name, entities));
        at = cursor + 1;
    }
    blocks
}

/// The **state** of an import: the layers, the blocks and the counts
/// of what has been read.
struct Import {
    /// The layers of the drawing, by name.
    layers: Vec<(String, u64)>,
    /// The blocks of the drawing, by name.
    blocks: Vec<(String, u64)>,
    /// The entities that have been read, by kind.
    counts: Vec<(String, usize)>,
}

/// **Add** an entity of a file to a block of the drawing.
///
/// The layer of the entity is the layer the file names, and a layer
/// the file does not define is **made**.
fn add_entity(
    session: &mut Session,
    import: &mut Import,
    drawing: u64,
    block: u64,
    kind: &str,
    tags: &[Tag],
) -> Result<Option<u64>, EngineError> {
    let Some((entity_kind, geometry)) = geometry_of(kind, tags) else {
        return Ok(None);
    };
    let plain = entity_kind.trim_start_matches("cad.entity.").to_string();
    // An **insert** names the block it inserts, and the block is
    // found by its name.
    let inserts = if entity_kind == "cad.entity.insert" {
        let Some(name) = text_of(tags, 2) else { return Ok(None) };
        let Some((_, target)) = import.blocks.iter().find(|(block, _)| *block == name) else {
            return Ok(None);
        };
        Some((*target, name))
    } else {
        None
    };
    let count = match import.counts.iter_mut().find(|(name, _)| *name == plain) {
        Some((_, count)) => {
            *count += 1;
            *count
        }
        None => {
            import.counts.push((plain.clone(), 1));
            1
        }
    };
    let entity = session.document_mut()?.add_node_with_under(
        Some(block),
        format!("{plain} {count}"),
        &entity_kind,
        None,
        json!({ "geometry": geometry }),
    );
    // The **layer** of the entity: a relation, as it is everywhere
    // else in this tree.
    if let Some(layer_name) = text_of(tags, 8) {
        let layer = match import.layers.iter().find(|(name, _)| *name == layer_name) {
            Some((_, layer)) => *layer,
            None => {
                let layer = session.document_mut()?.add_node_with_under(
                    Some(drawing),
                    layer_name.clone(),
                    "cad.layer",
                    None,
                    json!({ "color": Json::Null }),
                );
                import.layers.push((layer_name.clone(), layer));
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
    // The **block** an insert inserts: an edge, and it says which.
    if let Some((target, name)) = inserts {
        let relation = Relation::with_payload(entity, target, "cad.inserts", json!({ "block": name }));
        session.document_mut()?.add_relation(relation);
    }
    Ok(Some(entity))
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
        "INSERT" => {
            // An **insert** is an instance of a block, and where it
            // is put is the point of the record.
            let at = point_of(tags, 10, 20)?;
            Some(("cad.entity.insert".to_string(), json!({ "at": { "x": at.0, "y": at.1 } })))
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
        let mut import = Import { layers: Vec::new(), blocks: Vec::new(), counts: Vec::new() };
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
            import.layers.push((layer_name, layer));
        }
        let block = session.document_mut()?.add_node_with_under(
            Some(drawing),
            MODEL_SPACE.to_string(),
            "cad.block",
            None,
            json!({}),
        );
        import.blocks.push((MODEL_SPACE.to_string(), block));
        let entity_records = sections
            .iter()
            .find(|(name, _)| name == "ENTITIES")
            .map(|(_, body)| records_of(body))
            .unwrap_or_default();
        // The **blocks** of the file. They are made first, so that an
        // insert may name one of them.
        let file_blocks = sections
            .iter()
            .find(|(name, _)| name == "BLOCKS")
            .map(|(_, body)| blocks_of(body))
            .unwrap_or_default();
        let mut made_blocks: Vec<(u64, Vec<Record>)> = Vec::new();
        for (name, block_entities) in &file_blocks {
            if name == MODEL_SPACE {
                // The model's own record is the model space, and the
                // entities of the model space are the entities of the
                // `ENTITIES` section — the tree's own writer puts
                // them there. A file that keeps them in the record
                // instead is read when it has none of its own.
                if entity_records.is_empty() {
                    made_blocks.push((block, block_entities.clone()));
                }
                continue;
            }
            let made = session.document_mut()?.add_node_with_under(
                Some(drawing),
                name.clone(),
                "cad.block",
                None,
                json!({}),
            );
            import.blocks.push((name.clone(), made));
            made_blocks.push((made, block_entities.clone()));
        }
        // The **entities** of the file's `ENTITIES` section.
        let mut entities = 0usize;
        let mut skipped: Vec<String> = Vec::new();
        for (kind, tags) in &entity_records {
            match add_entity(session, &mut import, drawing, block, kind, tags)? {
                Some(_) => entities += 1,
                None => skipped.push(kind.clone()),
            }
        }
        // The **entities** of the blocks.
        for (made, block_entities) in &made_blocks {
            for (kind, tags) in block_entities {
                match add_entity(session, &mut import, drawing, *made, kind, tags)? {
                    Some(_) => entities += 1,
                    None => skipped.push(kind.clone()),
                }
            }
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
            "layers": import.layers.len(),
            "blocks": import.blocks.len(),
            "entities": entities,
            "units": units,
            "skipped": skipped,
        })))
    }
}
