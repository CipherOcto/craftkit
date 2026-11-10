//! The **flattening** of a form: the fields of a form are drawn into
//! the pages they are on, and the fields are taken away.
//!
//! The tree's own: an annotation's **appearance** is drawn into the
//! page with the matrix of ISO 32000-2 §12.5.5 — the appearance's
//! **bounding box**, transformed by its `/Matrix`, mapped onto the
//! `/Rect` of the annotation — and the annotation is taken away. A
//! hidden annotation is left alone, and a link is kept.
//!
//! # The rules
//!
//! - a **widget** — an annotation of the subtype `Widget` — is the
//!   appearance of a field on a page, and it is the widget that is
//!   flattened;
//! - the **appearance** is the `/AP` of the widget: the `/N` of it is
//!   the appearance that is drawn, and when the `/N` holds the
//!   **states** of a box, the `/AS` of the widget says which state is
//!   drawn;
//! - the appearance must be a **form** — a stream of the subtype
//!   `Form` — and it must have a **bounding box**: an appearance that
//!   has neither cannot be placed, and it is left alone;
//! - a widget that is **hidden** — the flag `Hidden` or the flag
//!   `NoView` — is left alone;
//! - the drawn appearance is put on the page as an **XObject** of the
//!   page's resources, under the tree's own name: `PCFl0`, `PCFl1`,
//!   and so on;
//! - the widget is **taken away**, and the references that named it go
//!   with it: the `/Annots` of the page, the `/Kids` of the field and
//!   the `/Fields` of the form;
//! - a **field that is left without widgets** is taken away from the
//!   form as well.
//!
//! # What it does not settle
//!
//! - The **comments** of a file are not flattened: the tree flattens
//!   the comments as well, and takes their pop-ups and their replies
//!   with them.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Node, NodeRef, Outcome, Session};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.form.flatten";

/// The flag of an annotation that is **hidden**.
const HIDDEN: i64 = 2;

/// The flag of an annotation that **cannot be seen**.
const NO_VIEW: i64 = 32;

/// The **node** of an object number.
fn node_of_number(document: &Document, number: u64) -> Option<&Node> {
    document.walk().find(|node| crate::the_number_of(node) == Some(number))
}

/// The **entry** of a dictionary, by its key.
fn entry<'a>(entries: &'a [(String, Json)], key: &str) -> Option<&'a Json> {
    entries.iter().find(|(name, _)| name == key).map(|(_, value)| value)
}

/// The **numbers** of a list of numbers.
fn numbers_of(value: &Json) -> Option<Vec<f64>> {
    let list = value.as_array()?;
    let mut out = Vec::with_capacity(list.len());
    for item in list {
        let number = item.as_f64()?;
        if !number.is_finite() {
            return None;
        }
        out.push(number);
    }
    Some(out)
}

/// The **rectangle** of an annotation: two corners in order.
fn rect_of(entries: &[(String, Json)]) -> Option<[f64; 4]> {
    corners(entry(entries, "Rect")?)
}

/// The **bounding box** of an appearance: two corners in order.
fn bbox_of(entries: &[(String, Json)]) -> Option<[f64; 4]> {
    corners(entry(entries, "BBox")?)
}

/// The **corners** of a box: two corners in order.
fn corners(value: &Json) -> Option<[f64; 4]> {
    let numbers = numbers_of(value)?;
    if numbers.len() != 4 {
        return None;
    }
    Some([
        numbers[0].min(numbers[2]),
        numbers[1].min(numbers[3]),
        numbers[0].max(numbers[2]),
        numbers[1].max(numbers[3]),
    ])
}

/// The **matrix** of the appearance: the tree's own, and the identity
/// when the appearance names none.
fn matrix_of(entries: &[(String, Json)]) -> [f64; 6] {
    let numbers = entry(entries, "Matrix").and_then(numbers_of).filter(|numbers| numbers.len() == 6);
    match numbers {
        Some(numbers) => [numbers[0], numbers[1], numbers[2], numbers[3], numbers[4], numbers[5]],
        None => [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
    }
}

/// The **matrix** that maps the appearance's bounding box onto the
/// rectangle of the annotation.
///
/// The tree's own: the box is transformed by the matrix of the
/// appearance, the **corners** of the box are what is measured, and
/// the box is scaled into the rectangle — a box that is empty in one
/// direction cannot be placed, and there is no matrix for it.
fn placement(bbox: [f64; 4], matrix: [f64; 6], rect: [f64; 4]) -> Option<[f64; 6]> {
    let points = [(bbox[0], bbox[1]), (bbox[2], bbox[1]), (bbox[0], bbox[3]), (bbox[2], bbox[3])]
        .map(|(x, y)| (matrix[0] * x + matrix[2] * y + matrix[4], matrix[1] * x + matrix[3] * y + matrix[5]));
    let (mut x0, mut x1) = (f64::MAX, f64::MIN);
    let (mut y0, mut y1) = (f64::MAX, f64::MIN);
    for (x, y) in points {
        x0 = x0.min(x);
        x1 = x1.max(x);
        y0 = y0.min(y);
        y1 = y1.max(y);
    }
    if x1 - x0 < 1e-9 || y1 - y0 < 1e-9 {
        return None;
    }
    let (sx, sy) = ((rect[2] - rect[0]) / (x1 - x0), (rect[3] - rect[1]) / (y1 - y0));
    Some([sx, 0.0, 0.0, sy, rect[0] - x0 * sx, rect[1] - y0 * sy])
}

/// The **appearance** of a widget: the number of the stream that is
/// drawn for it.
fn appearance_of(document: &Document, entries: &[(String, Json)]) -> Option<u64> {
    let appearances = entry(entries, "AP")?;
    // The `/AP` is a dictionary: written in the file, or named by a
    // reference.
    let appearances = if let Some(number) = crate::number_of(appearances) {
        crate::entries_of(&node_of_number(document, number)?.data)
    } else {
        crate::objects::pairs_of(appearances)
    };
    let normal = entry(&appearances, "N")?;
    // The `/N` is a stream, or the **states** of a box.
    if let Some(number) = crate::number_of(normal) {
        return Some(number);
    }
    let states = crate::objects::pairs_of(normal);
    let state = entry(entries, "AS")?.as_str()?;
    crate::number_of(entry(&states, state)?)
}

/// The **flags** of an annotation: the number the file holds.
fn flags_of(entries: &[(String, Json)]) -> i64 {
    entry(entries, "F")
        .and_then(|flags| flags.as_f64())
        .filter(|flags| flags.is_finite() && *flags > 0.0)
        .map(|flags| flags as i64)
        .unwrap_or(0)
}

/// Put an **entry** in the dictionary of a node.
fn put_in_node(document: &mut Document, id: u64, key: &str, value: Json) {
    let Some(node) = document.node(id) else { return };
    let mut entries = crate::entries_of(&node.data);
    let object = node.data.get("object").cloned().unwrap_or(Json::Null);
    match entries.iter_mut().find(|(name, _)| name == key) {
        Some(found) => found.1 = value,
        None => entries.push((key.to_string(), value)),
    }
    if let Some(node) = document.node_mut(id) {
        node.data = json!({ "object": object, "entries": crate::pairs_of(&entries) });
    }
}

/// Put an **entry** in a dictionary that a value holds, and say what
/// the value is afterwards.
///
/// A dictionary that is named by a reference is a node of its own,
/// and the entry goes into that node; a dictionary that is written in
/// the file is a value, and the entry goes into the value.
fn put_in_value(document: &mut Document, value: &Json, key: &str, entry: Json) -> Json {
    if let Some(number) = crate::number_of(value) {
        if let Some(node) = node_of_number(document, number) {
            let id = node.id;
            put_in_node(document, id, key, entry);
        }
        return value.clone();
    }
    let mut entries = crate::objects::pairs_of(value);
    match entries.iter_mut().find(|(name, _)| name == key) {
        Some(found) => found.1 = entry,
        None => entries.push((key.to_string(), entry)),
    }
    json!({ "entries": crate::pairs_of(&entries) })
}

/// The **dictionary** a value holds: the entries of it, whether it is
/// written in the file or named by a reference.
fn dict_of(document: &Document, value: &Json) -> Vec<(String, Json)> {
    if let Some(number) = crate::number_of(value) {
        return node_of_number(document, number)
            .map(|node| crate::entries_of(&node.data))
            .unwrap_or_default();
    }
    crate::objects::pairs_of(value)
}

/// Take a **widget** away: the node goes, and the references that
/// named it go with it.
fn take_away(session: &mut Session, widget: u64, number: u64) {
    let holders: Vec<(u64, Vec<(String, Json)>)> = session
        .document()
        .walk()
        .filter(|node| node.id != widget && (node.kind == "pdf.object" || node.kind == "pdf.stream"))
        .filter_map(|node| {
            let entries = crate::entries_of(&node.data);
            let kept = crate::remove::without(&entries, number);
            if kept == entries {
                return None;
            }
            Some((node.id, kept))
        })
        .collect();
    for (id, kept) in holders {
        let object = session
            .document()
            .node(id)
            .and_then(|node| node.data.get("object").cloned())
            .unwrap_or(Json::Null);
        if let Ok(document) = session.document_mut()
            && let Some(node) = document.node_mut(id)
        {
            node.data = json!({ "object": object, "entries": crate::pairs_of(&kept) });
        }
    }
    if let Ok(document) = session.document_mut() {
        document.remove_node(widget);
        crate::free::free_number(document, number, 0);
    }
}

/// The **fields** of the form of the file: the numbers of the objects
/// the `/Fields` of the form holds.
fn fields_of(document: &Document) -> Vec<u64> {
    let Some(catalog) = crate::preflight::the_catalog(document) else { return Vec::new() };
    let catalog_entries = crate::entries_of(&catalog.data);
    let Some(form) = entry(&catalog_entries, "AcroForm") else { return Vec::new() };
    let form = dict_of(document, form);
    let Some(fields) = entry(&form, "Fields").and_then(|fields| fields.as_array()) else {
        return Vec::new();
    };
    fields.iter().filter_map(crate::number_of).collect()
}

/// Take the **fields that are left without widgets** away from the
/// form.
fn prune(document: &mut Document) -> usize {
    let fields = fields_of(document);
    let mut gone: Vec<u64> = Vec::new();
    for number in &fields {
        let Some(node) = node_of_number(document, *number) else { continue };
        let entries = crate::entries_of(&node.data);
        let has_kids = entry(&entries, "Kids").is_some();
        let is_widget = entry(&entries, "Subtype").and_then(|kind| kind.as_str()) == Some("Widget");
        if !has_kids && !is_widget {
            gone.push(*number);
        }
    }
    if gone.is_empty() {
        return 0;
    }
    let Some(catalog) = crate::preflight::the_catalog(document) else { return 0 };
    let catalog_id = catalog.id;
    let catalog_entries = crate::entries_of(&catalog.data);
    let Some(form) = entry(&catalog_entries, "AcroForm").cloned() else { return 0 };
    let form_entries = dict_of(document, &form);
    let Some(fields) = entry(&form_entries, "Fields").and_then(|fields| fields.as_array()).cloned() else {
        return 0;
    };
    let kept: Vec<Json> = fields
        .iter()
        .filter(|field| !crate::number_of(field).is_some_and(|number| gone.contains(&number)))
        .cloned()
        .collect();
    let value = json!(kept);
    // The `/AcroForm` is a node of its own, or a dictionary in the
    // catalog.
    if crate::number_of(&form).is_some() {
        let Some(number) = crate::number_of(&form) else { return 0 };
        let Some(node) = node_of_number(document, number) else { return 0 };
        let id = node.id;
        put_in_node(document, id, "Fields", value);
    } else {
        let form = put_in_value(document, &form, "Fields", value);
        if let Some(node) = document.node_mut(catalog_id) {
            let data = crate::with_entry(&node.data, "AcroForm", form);
            node.data = data;
        }
    }
    gone.len()
}

/// **Flatten** the form of a file: the fields are drawn into the pages
/// they are on, and taken away.
///
/// The tree's own: the appearance of a widget is drawn with the matrix
/// of ISO 32000-2 §12.5.5, and the widget is taken away.
#[derive(Command)]
#[command(id = "pdf.form.flatten", area = "pdf", layer = "pdf", icon = "pencil", mutates, undoable)]
pub struct FlattenForm {
    /// The page that is flattened; every page of the file when it is
    /// not said.
    pub page: Option<NodeRef>,
}

impl FlattenForm {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        // The **pages**.
        let pages: Vec<u64> = match self.page {
            Some(page) => {
                let Some(node) = session.document().node(page.0) else {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: format!("there is no node `{}`", page.0),
                    });
                };
                let entries = crate::entries_of(&node.data);
                if !entries.iter().any(|(key, value)| key == "Type" && value.as_str() == Some("Page")) {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: format!("the node `{}` is not a page", page.0),
                    });
                }
                vec![page.0]
            }
            None => session
                .document()
                .walk()
                .filter(|node| {
                    node.kind == "pdf.object"
                        && crate::entries_of(&node.data)
                            .iter()
                            .any(|(key, value)| key == "Type" && value.as_str() == Some("Page"))
                })
                .map(|node| node.id)
                .collect(),
        };
        let mut drawn = 0_usize;
        let mut taken = 0_usize;
        let mut left = 0_usize;
        let mut flattened: Vec<Json> = Vec::new();
        for page in pages {
            let Some(page_node) = session.document().node(page).cloned() else { continue };
            let page_entries = crate::entries_of(&page_node.data);
            let annots = entry(&page_entries, "Annots")
                .and_then(|annots| annots.as_array())
                .cloned()
                .unwrap_or_default();
            // The **widgets** of the page, and the drawing of each.
            let mut drawings: Vec<(u64, u64, u64, [f64; 6])> = Vec::new();
            for annot in annots {
                let Some(number) = crate::number_of(&annot) else { continue };
                let Some(node) = node_of_number(session.document(), number) else { continue };
                let entries = crate::entries_of(&node.data);
                if entry(&entries, "Subtype").and_then(|kind| kind.as_str()) != Some("Widget") {
                    // Only the **fields** are flattened.
                    continue;
                }
                if flags_of(&entries) & (HIDDEN | NO_VIEW) != 0 {
                    // A **hidden** widget is left alone.
                    left += 1;
                    continue;
                }
                let Some(rect) = rect_of(&entries) else {
                    left += 1;
                    continue;
                };
                let Some(appearance) = appearance_of(session.document(), &entries) else {
                    left += 1;
                    continue;
                };
                let Some(stream) = node_of_number(session.document(), appearance) else {
                    left += 1;
                    continue;
                };
                let stream_entries = crate::entries_of(&stream.data);
                if entry(&stream_entries, "Subtype").and_then(|kind| kind.as_str()) != Some("Form") {
                    left += 1;
                    continue;
                }
                let Some(bbox) = bbox_of(&stream_entries) else {
                    left += 1;
                    continue;
                };
                let Some(matrix) = placement(bbox, matrix_of(&stream_entries), rect) else {
                    left += 1;
                    continue;
                };
                drawings.push((node.id, number, appearance, matrix));
            }
            if drawings.is_empty() {
                continue;
            }
            // The **content** of the page: the appearances are drawn
            // on top of it, in a stream of their own.
            let mut content =
                String::from("/Artifact <</Type /Pagination /Subtype /Form /PCMark /Flattened>> BDC\n");
            let mut xobjects: Vec<(String, Json)> = Vec::new();
            for (_, _, appearance, matrix) in drawings.iter() {
                let name = format!("PCFl{drawn}");
                content.push_str(&format!(
                    "q {} {} {} {} {} {} cm /{name} Do Q\n",
                    crate::watermark::number(matrix[0]),
                    crate::watermark::number(matrix[1]),
                    crate::watermark::number(matrix[2]),
                    crate::watermark::number(matrix[3]),
                    crate::watermark::number(matrix[4]),
                    crate::watermark::number(matrix[5])
                ));
                xobjects.push((name, json!({ "ref": [*appearance as f64, 0.0] })));
                drawn += 1;
            }
            content.push_str("EMC\n");
            // The **stream** of the page that holds the drawing.
            let number = crate::free::the_next_number(session.document(), 0);
            let Some(path) = a_path_beside(session, page, number) else {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "the file has no streams to put the drawing beside".to_string(),
                });
            };
            std::fs::write(&path, content.as_bytes()).map_err(|error| EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("the drawing cannot be written: {error}"),
            })?;
            let data = json!({ "object": number, "entries": [["Length", content.len()]] });
            let name = format!("stream {number}");
            let stream = session.document_mut()?.add_node_with(name, "pdf.stream", None, data);
            if let Some(node) = session.document_mut()?.node_mut(stream) {
                node.stream = Some(path);
            }
            // The page's **contents**: the drawing is put after them.
            let page_entries = crate::entries_of(
                &session.document().node(page).map(|node| node.data.clone()).unwrap_or(Json::Null),
            );
            let contents = match entry(&page_entries, "Contents") {
                Some(Json::Array(list)) => list.clone(),
                Some(value) => vec![value.clone()],
                None => Vec::new(),
            };
            let mut contents = contents;
            contents.push(json!({ "ref": [number as f64, 0.0] }));
            if let Some(node) = session.document_mut()?.node_mut(page) {
                node.data = crate::with_entry(&node.data, "Contents", json!(contents));
            }
            // The page's **resources**: the appearances are XObjects.
            let page_entries = crate::entries_of(
                &session.document().node(page).map(|node| node.data.clone()).unwrap_or(Json::Null),
            );
            let resources = entry(&page_entries, "Resources").cloned().unwrap_or(Json::Null);
            let mut xobject_value =
                entry(&dict_of(session.document(), &resources), "XObject").cloned().unwrap_or(Json::Null);
            for (name, reference) in &xobjects {
                xobject_value =
                    put_in_value(session.document_mut()?, &xobject_value, name, reference.clone());
            }
            let resources = put_in_value(session.document_mut()?, &resources, "XObject", xobject_value);
            if let Some(node) = session.document_mut()?.node_mut(page) {
                node.data = crate::with_entry(&node.data, "Resources", resources);
            }
            // The **widgets** are taken away.
            for (widget, object, _, _) in drawings {
                take_away(session, widget, object);
                taken += 1;
                flattened.push(json!({ "page": page, "widget": widget, "object": object }));
            }
        }
        // The **fields that are left without widgets** go away too.
        let pruned = prune(session.document_mut()?);
        Ok(Outcome::changed(format!("{drawn} appearance(s) drawn")).with_data(json!({
            "drawn": drawn,
            "taken": taken,
            "left": left,
            "pruned": pruned,
            "flattened": flattened,
        })))
    }
}

/// A **path** for a new stream: beside the streams the file already
/// has.
fn a_path_beside(session: &Session, page: u64, stream: u64) -> Option<String> {
    let page_entries = crate::entries_of(&session.document().node(page)?.data);
    let contents = entry(&page_entries, "Contents")?.clone();
    let contents = match contents {
        Json::Array(list) => list,
        other => vec![other],
    };
    for value in contents {
        let Some(number) = crate::number_of(&value) else { continue };
        let Some(node) = node_of_number(session.document(), number) else { continue };
        let Some(path) = node.stream.clone() else { continue };
        let directory = std::path::Path::new(&path).parent()?.to_path_buf();
        return Some(directory.join(format!("flattened-{stream}.bin")).to_string_lossy().to_string());
    }
    None
}
