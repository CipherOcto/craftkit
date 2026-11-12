//! The **boxes** of a page.
//!
//! A page has five boxes, and they are the geometry of the page:
//!
//! - the **media** box is the paper;
//! - the **crop** box is what is shown;
//! - the **bleed**, **trim** and **art** boxes are where the ink
//!   goes, where the page is cut, and where the artwork is.
//!
//! The tree's finding is that the boxes are a **chain**: the crop box
//! falls back to the media box, and the bleed, trim and art boxes fall
//! back to the crop box — and every one of them is **clipped** to the
//! media box, so a box that reaches outside the paper is cut back to
//! the paper. The media box itself falls back to the letter size, for
//! a page that says nothing at all.
//!
//! And the tree's finding is that a box may be **inherited**: the
//! media box and the crop box are among the keys a page may leave out
//! and take from its ancestor — the tree's own list — so a page in a
//! tree that says the size once says it once.
//!
//! # What it does not do
//!
//! - **The tree's margins are not here.** The tree's dialog takes
//!   margins inward from the media box as well as a rectangle; the
//!   rectangle is here and the margins are the same rectangle written
//!   another way.
//! - **The tree's pin of the default is not here**: the tree writes
//!   the media box out again when a crop box is removed, so that an
//!   inherited crop box does not come back.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{self, Value};
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session, Text};

/// The **boxes** of a page, in the tree's own order.
pub const BOXES: [&str; 5] = ["MediaBox", "CropBox", "BleedBox", "TrimBox", "ArtBox"];

/// The **box** a name says, if it is one.
///
/// The tree's rule: the name is the box's key without the `Box`, and
/// the case does not matter.
pub fn box_of(name: &str) -> Option<&'static str> {
    let wanted = name.trim().to_ascii_lowercase();
    let wanted = wanted.strip_suffix("box").unwrap_or(&wanted);
    BOXES.iter().find(|key| key.to_ascii_lowercase().starts_with(wanted) && wanted.len() >= 3).copied()
}

/// The letter size, in points: the tree's own default for a page that
/// says nothing about its size.
const LETTER: [f64; 4] = [0.0, 0.0, 612.0, 792.0];

/// The **rectangle** a value is, if it is one.
///
/// The tree's rule: four finite numbers, and the corners are put in
/// order — a rectangle whose lower corner is above its upper corner is
/// the same rectangle the other way up.
pub fn rect_of(value: &Value) -> Option<[f64; 4]> {
    let numbers: Vec<f64> = value.as_array()?.iter().filter_map(|number| number.as_f64()).collect();
    if numbers.len() != 4 || !numbers.iter().all(|number| number.is_finite()) {
        return None;
    }
    Some([
        numbers[0].min(numbers[2]),
        numbers[1].min(numbers[3]),
        numbers[0].max(numbers[2]),
        numbers[1].max(numbers[3]),
    ])
}

/// The **intersection** of two rectangles.
fn intersect(a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
    [a[0].max(b[0]), a[1].max(b[1]), a[2].min(b[2]), a[3].min(b[3])]
}

/// The **entries** of an object, as the document holds them.
fn entries_of(payload: &Value) -> Vec<(String, Value)> {
    let Some(list) = payload.get("entries").and_then(|entries| entries.as_array()) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|entry| {
            let pair = entry.as_array()?;
            let key = pair.first()?.as_str()?.to_string();
            let value = pair.get(1)?.clone();
            Some((key, value))
        })
        .collect()
}

/// The **entries** of an object, into the document.
fn entries_into(payload: &Value, entries: &[(String, Value)]) -> Value {
    serde_json::json!({
        "object": payload.get("object").cloned().unwrap_or(Value::Null),
        "entries": entries
            .iter()
            .map(|(key, value)| serde_json::json!([key, value]))
            .collect::<Vec<_>>(),
    })
}

/// The **parent** of a page.
fn parent_of(session: &Session, id: u64) -> Option<u64> {
    session
        .document()
        .relations
        .iter()
        .find(|relation| {
            relation.kind == "ref"
                && relation.from == id
                && relation
                    .payload
                    .get("key")
                    .and_then(|key| key.as_str())
                    .map(|key| key == "Parent")
                    .unwrap_or(false)
        })
        .map(|relation| relation.to)
}

/// The **keys** a page inherits.
const INHERITED: [&str; 2] = ["MediaBox", "CropBox"];

/// The box a page has, or takes from its ancestor.
///
/// The tree's rule: the nearest ancestor that has the box wins, and
/// the walk is bounded so that a document that says otherwise is not
/// walked for ever.
fn own_or_inherited(session: &Session, id: u64, key: &str) -> Option<Value> {
    let node = session.document().node(id)?;
    if let Some((_, value)) = entries_of(&node.data).iter().find(|(name, _)| name == key) {
        return Some(value.clone());
    }
    if !INHERITED.contains(&key) {
        return None;
    }
    let mut ancestor = parent_of(session, id);
    let mut steps = 0_usize;
    while let Some(current) = ancestor {
        steps += 1;
        if steps > 64 {
            break;
        }
        let Some(current_node) = session.document().node(current) else { break };
        if let Some((_, value)) = entries_of(&current_node.data).iter().find(|(name, _)| name == key) {
            return Some(value.clone());
        }
        ancestor = parent_of(session, current);
    }
    None
}

/// The **boxes** of a page, as they are: the media box, the crop box
/// and the three others, each one falling back and each one clipped.
pub fn boxes_of(session: &Session, id: u64) -> [Value; 5] {
    let media = own_or_inherited(session, id, "MediaBox").and_then(|value| rect_of(&value)).unwrap_or(LETTER);
    let crop = own_or_inherited(session, id, "CropBox")
        .and_then(|value| rect_of(&value))
        .map(|crop| intersect(crop, media))
        .unwrap_or(media);
    let child = |key: &str| -> Value {
        let own = session
            .document()
            .node(id)
            .and_then(|node| entries_of(&node.data).into_iter().find(|(name, _)| name == key))
            .and_then(|(_, value)| rect_of(&value));
        match own {
            Some(rect) => {
                Value::Array(intersect(rect, media).iter().map(|number| serde_json::json!(number)).collect())
            }
            None => Value::Array(crop.iter().map(|number| serde_json::json!(number)).collect()),
        }
    };
    [
        Value::Array(media.iter().map(|number| serde_json::json!(number)).collect()),
        Value::Array(crop.iter().map(|number| serde_json::json!(number)).collect()),
        child("BleedBox"),
        child("TrimBox"),
        child("ArtBox"),
    ]
}

/// The **boxes** of a page.
///
/// The command says all five, and where each of them came from: the
/// page's own dictionary, an ancestor, or the default.
#[derive(Command)]
#[command(id = "pdf.page.boxes", area = "pdf", layer = "pdf", icon = "list")]
pub struct PageBoxes {
    /// The page.
    pub page: NodeRef,
}

impl PageBoxes {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "pdf.page.boxes".to_string();
        let page = self.page.0;
        let Some(node) = session.document().node(page) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{page}`"),
            });
        };
        if node.kind != "pdf.object" && node.kind != "pdf.stream" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{page}` is a `{}` and not an object", node.kind),
            });
        }
        let boxes = boxes_of(session, page);
        let own: Vec<String> = entries_of(&node.data).into_iter().map(|(key, _)| key).collect();
        let mut from = serde_json::Map::new();
        for key in BOXES.iter() {
            let source = if own.iter().any(|name| name == key) {
                "own"
            } else if INHERITED.contains(key) && parent_of(session, page).is_some() {
                "inherited"
            } else {
                "default"
            };
            from.insert(key.to_string(), serde_json::json!(source));
        }
        Ok(Outcome::new(format!("the page `{page}` has {} box(es)", BOXES.len())).with_data(
            serde_json::json!({
                "id": page,
                "boxes": boxes,
                "from": from,
            }),
        ))
    }
}

/// Set a **box** of a page.
///
/// The box is a rectangle in points: four numbers, the lower left
/// corner first. The media box is the paper and the others are cut
/// back to it, so a box that reaches outside the paper is the paper.
///
/// A rectangle that is **not four finite numbers** is refused, and a
/// rectangle that would make the page **smaller than a point** is
/// refused: the tree's rule, and a page that cannot be seen is not a
/// page.
#[derive(Command)]
#[command(id = "pdf.page.box", area = "pdf", layer = "pdf", icon = "pencil", mutates, undoable)]
pub struct SetPageBox {
    /// The page.
    pub page: NodeRef,
    /// The box: `MediaBox`, `CropBox`, `BleedBox`, `TrimBox`, `ArtBox`.
    pub which: Text,
    /// The rectangle: four numbers, the lower left corner first.
    pub rect: serde_json::Value,
}

impl SetPageBox {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "pdf.page.box".to_string();
        let page = self.page.0;
        let wanted = self.which.0.clone();
        let key = box_of(&wanted).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("there is no box `{wanted}`: the boxes are {}", BOXES.join(", ")),
        })?;
        let Some(node) = session.document().node(page) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{page}`"),
            });
        };
        if node.kind != "pdf.object" && node.kind != "pdf.stream" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{page}` is a `{}` and not an object", node.kind),
            });
        }
        let rect = rect_of(&self.rect).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: "the rectangle is not four numbers".to_string(),
        })?;
        // The **media box** is the paper, and every other box is cut
        // back to it.
        let boxes = boxes_of(session, page);
        let media = rect_of(&boxes[0]).unwrap_or(LETTER);
        let rect = if key == "MediaBox" { rect } else { intersect(rect, media) };
        if rect[2] - rect[0] < 1.0 || rect[3] - rect[1] < 1.0 {
            return Err(EngineError::Failed {
                id,
                message: "the page would be smaller than a point".to_string(),
            });
        }
        let value = Value::Array(rect.iter().map(|number| serde_json::json!(number)).collect());
        let Some(node) = session.document_mut()?.node_mut(page) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{page}`"),
            });
        };
        let mut entries = entries_of(&node.data);
        match entries.iter_mut().find(|(name, _)| name == key) {
            Some(entry) => entry.1 = value.clone(),
            None => entries.push((key.to_string(), value.clone())),
        }
        node.data = entries_into(&node.data, &entries);
        Ok(Outcome::changed(format!("the `{key}` of the page `{page}` is set"))
            .with_data(serde_json::json!({ "id": page, "box": key, "rect": value })))
    }
}
