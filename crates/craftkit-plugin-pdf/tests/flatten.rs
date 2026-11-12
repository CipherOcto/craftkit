//! The **flattening** of a form: the fields of a form are drawn into
//! the pages they are on, and taken away.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Node, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_pdf::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the PDF battery.
fn ecosystem() -> Ecosystem {
    Ecosystem::from_inventory().expect("the ecosystem is built")
}

/// A session over the battery's own contributions.
fn session() -> Session {
    let ecosystem = ecosystem();
    Session::with_formats(
        ecosystem.commands,
        ecosystem.kinds,
        ecosystem.formats,
        ecosystem.migrations,
        ecosystem.evaluators,
    )
}

/// A **directory** of this test's own, so that the files do not mix.
fn directory(name: &str) -> String {
    let path = std::env::temp_dir().join(format!("craftkit-v2-flatten-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// The **content** of the page.
const PAGE_CONTENT: &str = "0 0 1 rg 10 10 100 100 re f\n";

/// The **content** of the appearance of a widget.
const APPEARANCE_CONTENT: &str = "0 0 0 rg 0 0 200 100 re f\n";

/// A file of PDF with a **form** on it: a page, a widget with an
/// appearance, a link and a form that holds the fields.
fn a_file(widget: &str, field: &str, fields: &str) -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    let bodies: Vec<(u32, String)> = vec![
        (1, "<< /Type /Catalog /Pages 2 0 R /AcroForm 9 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R /Annots [6 0 R 7 0 R] /Resources << >> >>"
                .to_string(),
        ),
        (
            4,
            format!("<< /Length {} >>\nstream\n{PAGE_CONTENT}endstream", PAGE_CONTENT.len()),
        ),
        (
            5,
            format!(
                "<< /Type /XObject /Subtype /Form /BBox [0 0 200 100] /Resources << >> /Length {} >>\nstream\n{APPEARANCE_CONTENT}endstream",
                APPEARANCE_CONTENT.len()
            ),
        ),
        (6, widget.to_string()),
        (7, "<< /Type /Annot /Subtype /Link /Rect [0 0 10 10] /P 3 0 R >>".to_string()),
        (8, field.to_string()),
        (9, format!("<< /Fields {fields} /DA (/Helv 0 Tf 0 g) >>")),
    ];
    for (number, body) in &bodies {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    file.extend_from_slice(b"xref\n0 10\n");
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(b"trailer\n<< /Size 10 /Root 1 0 R >>\nstartxref\n");
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    file
}

/// The **widget** of the tree's own: a widget with an appearance.
fn a_widget() -> &'static str {
    "<< /Type /Annot /Subtype /Widget /Rect [100 200 300 300] /AP << /N 5 0 R >> /P 3 0 R /F 4 /T (name) >>"
}

/// Load the file and give the **page**.
fn a_page(session: &mut Session, name: &str, widget: &str, field: &str, fields: &str) -> u64 {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, a_file(widget, field, fields)).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
    session
        .document()
        .walk()
        .find(|node| {
            node.data["entries"]
                .as_array()
                .map(|entries| {
                    entries.iter().any(|entry| {
                        let Some(pair) = entry.as_array() else { return false };
                        pair.first().and_then(|key| key.as_str()) == Some("Type")
                            && pair.get(1).and_then(|value| value.as_str()) == Some("Page")
                    })
                })
                .unwrap_or(false)
        })
        .map(|node| node.id)
        .expect("the page is there")
}

/// The **node** of an object number.
fn node_of(session: &Session, number: u64) -> Option<Node> {
    session
        .document()
        .walk()
        .find(|node| node.data.get("object").and_then(|number| number.as_u64()) == Some(number))
        .cloned()
}

/// The **entries** of the dictionary of a node.
fn entries_of(node: &Node) -> Vec<(String, craftkit_sdk::serde_json::Value)> {
    node.data["entries"]
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| {
                    let pair = entry.as_array()?;
                    Some((pair.first()?.as_str()?.to_string(), pair.get(1)?.clone()))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The **entries** of a dictionary that is a value.
fn dict_of(value: &craftkit_sdk::serde_json::Value) -> Vec<(String, craftkit_sdk::serde_json::Value)> {
    value["entries"]
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| {
                    let pair = entry.as_array()?;
                    Some((pair.first()?.as_str()?.to_string(), pair.get(1)?.clone()))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The **entry** of a dictionary, by its key.
fn entry<'a>(
    entries: &'a [(String, craftkit_sdk::serde_json::Value)],
    key: &str,
) -> Option<&'a craftkit_sdk::serde_json::Value> {
    entries.iter().find(|(name, _)| name == key).map(|(_, value)| value)
}

/// The **text** of every stream of the file: the content of the page
/// and the drawing that was put on it.
fn every_stream(session: &Session) -> String {
    let mut out = String::new();
    for node in session.document().walk() {
        if node.kind != "pdf.stream" {
            continue;
        }
        let Some(path) = node.stream.clone() else { continue };
        let Ok(bytes) = std::fs::read_to_string(&path) else { continue };
        out.push_str(&bytes);
        out.push('\n');
    }
    out
}

/// **The appearance of a widget is drawn and the widget is taken away.**
#[test]
fn the_appearance_of_a_widget_is_drawn_and_the_widget_is_taken_away() {
    let mut session = session();
    let page = a_page(&mut session, "drawn", a_widget(), "<< >>", "[6 0 R]");
    let outcome =
        session.execute("pdf.form.flatten", &json!({ "page": page })).expect("the form is flattened");
    assert_eq!(outcome.data["drawn"], json!(1), "one appearance is drawn: {outcome:?}");
    assert_eq!(outcome.data["taken"], json!(1), "one widget is taken: {outcome:?}");
    assert_eq!(outcome.data["left"], json!(0), "nothing is left: {outcome:?}");
    // The **drawing**: the matrix of the tree, and the tree's name.
    let content = every_stream(&session);
    assert!(content.contains("1 0 0 1 100 200 cm"), "the matrix is the tree's: {content}");
    assert!(content.contains("/PCFl0 Do"), "the appearance is drawn: {content}");
    // The **widget** is gone.
    assert!(node_of(&session, 6).is_none(), "the widget is taken away");
    // The **page** does not name it any more.
    let page_node = node_of(&session, 3).expect("the page is there");
    let annots = entry(&entries_of(&page_node), "Annots").and_then(|annots| annots.as_array()).cloned();
    let annots = annots.unwrap_or_default();
    assert_eq!(annots.len(), 1, "only the link is left: {annots:?}");
    // The **form** does not name it any more.
    let form = node_of(&session, 9).expect("the form is there");
    let fields = entry(&entries_of(&form), "Fields").and_then(|fields| fields.as_array()).cloned();
    let fields = fields.unwrap_or_default();
    assert!(fields.is_empty(), "the field is gone: {fields:?}");
}

/// **A hidden widget is left alone.**
#[test]
fn a_hidden_widget_is_left_alone() {
    let mut session = session();
    let hidden = "<< /Type /Annot /Subtype /Widget /Rect [100 200 300 300] /AP << /N 5 0 R >> /P 3 0 R /F 6 /T (name) >>";
    let page = a_page(&mut session, "hidden", hidden, "<< >>", "[6 0 R]");
    let outcome =
        session.execute("pdf.form.flatten", &json!({ "page": page })).expect("the form is flattened");
    assert_eq!(outcome.data["drawn"], json!(0), "nothing is drawn: {outcome:?}");
    assert_eq!(outcome.data["left"], json!(1), "the widget is left: {outcome:?}");
    assert!(node_of(&session, 6).is_some(), "the widget is still there");
}

/// **A widget with no appearance is left alone.**
#[test]
fn a_widget_with_no_appearance_is_left_alone() {
    let mut session = session();
    let bare = "<< /Type /Annot /Subtype /Widget /Rect [100 200 300 300] /P 3 0 R /F 4 /T (name) >>";
    let page = a_page(&mut session, "bare", bare, "<< >>", "[6 0 R]");
    let outcome =
        session.execute("pdf.form.flatten", &json!({ "page": page })).expect("the form is flattened");
    assert_eq!(outcome.data["drawn"], json!(0), "nothing is drawn: {outcome:?}");
    assert_eq!(outcome.data["left"], json!(1), "the widget is left: {outcome:?}");
    assert!(node_of(&session, 6).is_some(), "the widget is still there");
}

/// **The state of a box is the one that is drawn.**
#[test]
fn the_state_of_a_box_is_the_one_that_is_drawn() {
    let mut session = session();
    let box_ = "<< /Type /Annot /Subtype /Widget /Rect [100 200 300 300] /AP << /N << /Yes 5 0 R /Off 8 0 R >> >> /AS /Yes /P 3 0 R /F 4 /T (box) >>";
    let page = a_page(&mut session, "box", box_, "<< >>", "[6 0 R]");
    let outcome =
        session.execute("pdf.form.flatten", &json!({ "page": page })).expect("the form is flattened");
    assert_eq!(outcome.data["drawn"], json!(1), "one appearance is drawn: {outcome:?}");
    let page_node = node_of(&session, 3).expect("the page is there");
    let resources = entry(&entries_of(&page_node), "Resources").cloned().expect("the resources are there");
    let xobjects = entry(&dict_of(&resources), "XObject").cloned().expect("the objects are there");
    let name = entry(&dict_of(&xobjects), "PCFl0").cloned().expect("the appearance is there");
    assert_eq!(name["ref"][0], json!(5.0), "the state that is on is the one drawn: {name:?}");
}

/// **The matrix maps the box of the appearance onto the rectangle.**
#[test]
fn the_matrix_maps_the_box_of_the_appearance_onto_the_rectangle() {
    let mut session = session();
    let widget = "<< /Type /Annot /Subtype /Widget /Rect [100 200 300 400] /AP << /N 5 0 R >> /P 3 0 R /F 4 /T (name) >>";
    let page = a_page(&mut session, "matrix", widget, "<< >>", "[6 0 R]");
    session.execute("pdf.form.flatten", &json!({ "page": page })).expect("the form is flattened");
    let content = every_stream(&session);
    // The box is 200 wide and 100 high, and the rectangle is 200 wide
    // and 200 high: the scale is one and two.
    assert!(content.contains("1 0 0 2 100 200 cm"), "the matrix is the tree's: {content}");
}

/// **A field that is left without widgets is taken away from the form.**
#[test]
fn a_field_that_is_left_without_widgets_is_taken_away_from_the_form() {
    let mut session = session();
    let field = "<< /FT /Tx /T (name) /Kids [6 0 R] /P 3 0 R >>";
    let page = a_page(&mut session, "pruned", a_widget(), field, "[8 0 R]");
    let outcome =
        session.execute("pdf.form.flatten", &json!({ "page": page })).expect("the form is flattened");
    assert_eq!(outcome.data["drawn"], json!(1), "one appearance is drawn: {outcome:?}");
    assert_eq!(outcome.data["pruned"], json!(1), "the field is taken away: {outcome:?}");
    let form = node_of(&session, 9).expect("the form is there");
    let fields = entry(&entries_of(&form), "Fields").and_then(|fields| fields.as_array()).cloned();
    let fields = fields.unwrap_or_default();
    assert!(fields.is_empty(), "the form holds no field: {fields:?}");
}

/// **The file that is written holds the drawing and not the widget.**
#[test]
fn the_file_that_is_written_holds_the_drawing_and_not_the_widget() {
    let mut session = session();
    let directory = directory("written");
    let page = a_page(&mut session, "written", a_widget(), "<< >>", "[6 0 R]");
    session.execute("pdf.form.flatten", &json!({ "page": page })).expect("the form is flattened");
    let path = format!("{directory}/flat.pdf");
    session.execute("pdf.file.write", &json!({ "path": path })).expect("the file is written");
    let file = String::from_utf8_lossy(&std::fs::read(&path).expect("the file is there")).to_string();
    assert!(file.contains("/PCFl0 Do"), "the drawing is in the file: {file}");
    assert!(!file.contains("/Subtype /Widget"), "the widget is not in the file: {file}");
    assert!(file.contains("/PCFl0"), "the appearance is named: {file}");
}

/// **A link is left alone.**
#[test]
fn a_link_is_left_alone() {
    let mut session = session();
    let page = a_page(&mut session, "link", a_widget(), "<< >>", "[6 0 R]");
    session.execute("pdf.form.flatten", &json!({ "page": page })).expect("the form is flattened");
    assert!(node_of(&session, 7).is_some(), "the link is still there");
    let page_node = node_of(&session, 3).expect("the page is there");
    let annots = entry(&entries_of(&page_node), "Annots").and_then(|annots| annots.as_array()).cloned();
    let annots = annots.unwrap_or_default();
    assert_eq!(annots.len(), 1, "the link is still named: {annots:?}");
}
