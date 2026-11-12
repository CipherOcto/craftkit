//! The **flattening** of a file: the **fields** of a form and the
//! **comments** of a file are drawn into the pages they are on, and
//! taken away.
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
//! - a **comment** is an annotation that is **none of a link, a widget
//!   and a popup**, and it is the comment that is flattened;
//! - the **appearance** is the `/AP` of the annotation: the `/N` of it
//!   is the appearance that is drawn, and when the `/N` holds the
//!   **states** of a box, the `/AS` of the annotation says which state
//!   is drawn;
//! - the appearance must be a **form** — a stream of the subtype
//!   `Form` — and it must have a **bounding box**: an appearance that
//!   has neither cannot be placed, and it is left alone;
//! - an annotation that is **hidden** — the flag `Hidden` or the flag
//!   `NoView` — is left alone;
//! - the drawn appearance is put on the page as an **XObject** of the
//!   page's resources, under the tree's own name: `PCFl0`, `PCFl1`,
//!   and so on;
//! - the annotation is **taken away**, and the references that named it
//!   go with it: the `/Annots` of the page, the `/Kids` of the field
//!   and the `/Fields` of the form;
//! - a **field that is left without widgets** is taken away from the
//!   form as well;
//! - the **popup** of a comment that is flattened goes with it, and so
//!   does a **reply** to it — the tree's own rule.
//!
//! # What it does not settle
//!
//! - The **comments** of a file and its **fields** are flattened by
//!   two commands here, and by one in the tree.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Node, NodeRef, Outcome, Session};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.form.flatten";

/// The flag of an annotation that is **hidden**.
const HIDDEN: i64 = 2;

/// The flag of an annotation that **cannot be seen**.
const NO_VIEW: i64 = 32;

/// What is being **flattened**.
#[derive(Clone, Copy, PartialEq)]
pub enum What {
    /// The **fields** of the form: the widgets.
    Fields,
    /// The **comments** of the file: the annotations that are notes.
    Comments,
}

/// What a flattening did.
pub struct Report {
    /// The number of appearances that were drawn.
    pub drawn: usize,
    /// The number of annotations that were taken away.
    pub taken: usize,
    /// The number of annotations that were left alone.
    pub left: usize,
    /// The number of fields that were taken away from the form.
    pub pruned: usize,
    /// What was flattened, one entry for each annotation.
    pub flattened: Vec<Json>,
}

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

/// The **appearance** of an annotation: the number of the stream that
/// is drawn for it.
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

/// Take an **annotation** away: the node goes, and the references that
/// named it go with it.
fn take_away(session: &mut Session, annotation: u64, number: u64) {
    let holders: Vec<(u64, Vec<(String, Json)>)> = session
        .document()
        .walk()
        .filter(|node| node.id != annotation && (node.kind == "pdf.object" || node.kind == "pdf.stream"))
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
        document.remove_node(annotation);
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

/// The **annotations** of a page, in the order the file holds them.
fn annots_of(document: &Document, page: u64) -> Vec<(u64, String)> {
    let Some(node) = document.node(page) else { return Vec::new() };
    let entries = crate::entries_of(&node.data);
    let Some(annots) = entry(&entries, "Annots").and_then(|annots| annots.as_array()).cloned() else {
        return Vec::new();
    };
    let mut out: Vec<(u64, String)> = Vec::new();
    for annot in annots {
        let Some(node) = crate::preflight::node_of(document, &annot) else { continue };
        let subtype = crate::entries_of(&node.data)
            .into_iter()
            .find(|(key, _)| key == "Subtype")
            .and_then(|(_, value)| value.as_str().map(|text| text.to_string()))
            .unwrap_or_default();
        out.push((node.id, subtype));
    }
    out
}

/// The annotations that go with a comment that is **flattened**: the
/// **popup** of the comment and the **replies** to it.
///
/// The tree's own: a popup whose parent is a comment that is gone goes
/// with it, and so does a reply to a comment that is gone.
fn gone_with(document: &Document, annots: &[(u64, String)], gone: &mut Vec<u64>) {
    for (annot, subtype) in annots {
        if gone.contains(annot) {
            continue;
        }
        let Some(node) = document.node(*annot) else { continue };
        let entries = crate::entries_of(&node.data);
        let names = |key: &str| {
            entry(&entries, key).and_then(crate::number_of).is_some_and(|number| gone.contains(&number))
        };
        if (subtype == "Popup" && names("Parent")) || names("IRT") {
            gone.push(*annot);
        }
    }
}

/// The **pages** of the file, in the order of the tree of pages.
pub(crate) fn pages_of(document: &Document) -> Vec<u64> {
    let Some(catalog) = crate::preflight::the_catalog(document) else { return Vec::new() };
    let catalog_entries = crate::entries_of(&catalog.data);
    let Some(pages) = entry(&catalog_entries, "Pages") else { return Vec::new() };
    let Some(node) = crate::preflight::node_of(document, pages) else { return Vec::new() };
    let mut out: Vec<u64> = Vec::new();
    walk_pages(document, node.id, 0, &mut out);
    out
}

/// Walk the **tree of pages**, and take the pages of it in order.
fn walk_pages(document: &Document, node: u64, depth: usize, out: &mut Vec<u64>) {
    if depth >= crate::comments::MAX_DEPTH {
        return;
    }
    let Some(current) = document.node(node) else { return };
    let entries = crate::entries_of(&current.data);
    let is_page = entries.iter().any(|(key, value)| key == "Type" && value.as_str() == Some("Page"));
    if is_page {
        out.push(node);
        return;
    }
    let Some(kids) = entry(&entries, "Kids").and_then(|kids| kids.as_array()).cloned() else {
        return;
    };
    for kid in kids {
        let Some(kid_node) = crate::preflight::node_of(document, &kid) else { continue };
        let id = kid_node.id;
        walk_pages(document, id, depth + 1, out);
    }
}

/// **Flatten** the fields or the comments of the pages.
pub(crate) fn flatten_pages(session: &mut Session, pages: &[u64], what: What) -> Result<Report, EngineError> {
    let mut report = Report { drawn: 0, taken: 0, left: 0, pruned: 0, flattened: Vec::new() };
    // The annotations that were **not** drawn, and the ones that were
    // **taken away**: an annotation that is left is one that is still
    // there when the flattening is done.
    let mut skipped: Vec<u64> = Vec::new();
    let mut removed: Vec<u64> = Vec::new();
    for page in pages {
        let annots = annots_of(session.document(), *page);
        // The **annotations** that are flattened, and the drawing of
        // each.
        let mut drawings: Vec<(u64, u64, u64, [f64; 6])> = Vec::new();
        for (annot, subtype) in &annots {
            let wanted = match what {
                What::Fields => subtype == "Widget",
                What::Comments => !matches!(subtype.as_str(), "Link" | "Widget" | "Popup"),
            };
            if !wanted {
                continue;
            }
            let Some(node) = session.document().node(*annot) else { continue };
            let Some(number) = crate::the_number_of(node) else { continue };
            let entries = crate::entries_of(&node.data);
            if flags_of(&entries) & (HIDDEN | NO_VIEW) != 0 {
                // A **hidden** annotation is left alone.
                skipped.push(*annot);
                continue;
            }
            let Some(rect) = rect_of(&entries) else {
                skipped.push(*annot);
                continue;
            };
            let Some(appearance) = appearance_of(session.document(), &entries) else {
                skipped.push(*annot);
                continue;
            };
            let Some(stream) = node_of_number(session.document(), appearance) else {
                skipped.push(*annot);
                continue;
            };
            let stream_entries = crate::entries_of(&stream.data);
            if entry(&stream_entries, "Subtype").and_then(|kind| kind.as_str()) != Some("Form") {
                skipped.push(*annot);
                continue;
            }
            let Some(bbox) = bbox_of(&stream_entries) else {
                skipped.push(*annot);
                continue;
            };
            let Some(matrix) = placement(bbox, matrix_of(&stream_entries), rect) else {
                skipped.push(*annot);
                continue;
            };
            drawings.push((*annot, number, appearance, matrix));
        }
        if drawings.is_empty() {
            continue;
        }
        // The **content** of the page: the appearances are drawn on top
        // of it, in a stream of their own.
        let mut content =
            String::from("/Artifact <</Type /Pagination /Subtype /Form /PCMark /Flattened>> BDC\n");
        let mut xobjects: Vec<(String, Json)> = Vec::new();
        for (_, _, appearance, matrix) in &drawings {
            let name = format!("PCFl{}", report.drawn);
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
            report.drawn += 1;
        }
        content.push_str("EMC\n");
        // The **stream** of the page that holds the drawing.
        let number = crate::free::the_next_number(session.document(), 0);
        let Some(path) = a_path_beside(session, *page, number) else {
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
            &session.document().node(*page).map(|node| node.data.clone()).unwrap_or(Json::Null),
        );
        let contents = match entry(&page_entries, "Contents") {
            Some(Json::Array(list)) => list.clone(),
            Some(value) => vec![value.clone()],
            None => Vec::new(),
        };
        let mut contents = contents;
        contents.push(json!({ "ref": [number as f64, 0.0] }));
        if let Some(node) = session.document_mut()?.node_mut(*page) {
            node.data = crate::with_entry(&node.data, "Contents", json!(contents));
        }
        // The page's **resources**: the appearances are XObjects.
        let page_entries = crate::entries_of(
            &session.document().node(*page).map(|node| node.data.clone()).unwrap_or(Json::Null),
        );
        let resources = entry(&page_entries, "Resources").cloned().unwrap_or(Json::Null);
        let mut xobject_value =
            entry(&dict_of(session.document(), &resources), "XObject").cloned().unwrap_or(Json::Null);
        for (name, reference) in &xobjects {
            xobject_value = put_in_value(session.document_mut()?, &xobject_value, name, reference.clone());
        }
        let resources = put_in_value(session.document_mut()?, &resources, "XObject", xobject_value);
        if let Some(node) = session.document_mut()?.node_mut(*page) {
            node.data = crate::with_entry(&node.data, "Resources", resources);
        }
        // The **annotations** are taken away.
        let mut gone: Vec<u64> = drawings.iter().map(|(annot, _, _, _)| *annot).collect();
        if what == What::Comments {
            gone_with(session.document(), &annots, &mut gone);
        }
        for (annot, number, _, _) in &drawings {
            report.flattened.push(json!({
                "page": page,
                "annotation": annot,
                "object": number,
            }));
        }
        for annot in gone {
            let Some(node) = session.document().node(annot).cloned() else { continue };
            let Some(number) = crate::the_number_of(&node) else { continue };
            take_away(session, annot, number);
            removed.push(annot);
            report.taken += 1;
        }
    }
    // The annotations that are **left** are the ones that were not
    // drawn and that are still there.
    report.left = skipped.iter().filter(|annot| !removed.contains(annot)).count();
    // The **fields that are left without widgets** go away too.
    report.pruned = prune(session.document_mut()?);
    Ok(report)
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

/// The **pages** that are asked for: the page that is said, or every
/// page of the file.
fn the_pages(session: &Session, page: Option<NodeRef>) -> Result<Vec<u64>, EngineError> {
    let document = session.document();
    let pages = pages_of(document);
    match page {
        Some(page) => {
            let Some(node) = document.node(page.0) else {
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
            Ok(vec![page.0])
        }
        None => Ok(pages),
    }
}

commands! {
    /// **Flatten** the form of a file: the fields are drawn into the pages
    /// they are on, and taken away.
    ///
    /// The tree's own: the appearance of a widget is drawn with the matrix
    /// of ISO 32000-2 §12.5.5, and the widget is taken away.
    pdf.form.flatten {
        area: "pdf",
        layer: "pdf",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The page that is flattened; every page of the file when it is
            /// not said.

            page: Option<NodeRef>,
        },
        run: {

            let pages = the_pages(session, self.page)?;
            let report = flatten_pages(session, &pages, What::Fields)?;
            Ok(Outcome::changed(format!("{} appearance(s) drawn", report.drawn)).with_data(json!({
                "drawn": report.drawn,
                "taken": report.taken,
                "left": report.left,
                "pruned": report.pruned,
                "flattened": report.flattened,
            })))

        }
    }
    /// **Flatten** the comments of a file: they are drawn into the pages
    /// they are on, and taken away.
    ///
    /// The tree's own: the appearance of a comment is drawn with the matrix
    /// of ISO 32000-2 §12.5.5, and the comment is taken away — with its
    /// popup and its replies.
    pdf.comment.flatten {
        area: "pdf",
        layer: "pdf",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The page that is flattened; every page of the file when it is
            /// not said.

            page: Option<NodeRef>,
        },
        run: {

            let pages = the_pages(session, self.page)?;
            let report = flatten_pages(session, &pages, What::Comments)?;
            Ok(Outcome::changed(format!("{} appearance(s) drawn", report.drawn)).with_data(json!({
                "drawn": report.drawn,
                "taken": report.taken,
                "left": report.left,
                "pruned": report.pruned,
                "flattened": report.flattened,
            })))

        }
    }
}
