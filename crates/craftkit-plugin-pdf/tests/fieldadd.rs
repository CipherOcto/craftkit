//! The **text field** that is put on a page.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-fieldadd-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// The **content** of the page.
const PAGE_CONTENT: &str = "0 0 1 rg 10 10 100 100 re f\n";

/// A file of PDF with a **page** on it.
fn a_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    let bodies: Vec<(u32, String)> = vec![
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R /Resources << >> >>"
                .to_string(),
        ),
        (4, format!("<< /Length {} >>\nstream\n{PAGE_CONTENT}endstream", PAGE_CONTENT.len())),
    ];
    for (number, body) in &bodies {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    file.extend_from_slice(b"xref\n0 5\n");
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(b"trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n");
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    file
}

/// Load the file and give the **page**.
fn a_page(session: &mut Session, directory: &str) -> u64 {
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, a_file()).expect("the file is written");
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

/// The **entries** of the dictionary of a node.
fn entries_of(node: &craftkit_sdk::Node) -> Vec<(String, craftkit_sdk::serde_json::Value)> {
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

/// The **entry** of a dictionary, by its key.
fn entry<'a>(
    entries: &'a [(String, craftkit_sdk::serde_json::Value)],
    key: &str,
) -> Option<&'a craftkit_sdk::serde_json::Value> {
    entries.iter().find(|(name, _)| name == key).map(|(_, value)| value)
}

/// The **node** of an id.
fn node_of(session: &Session, id: u64) -> Option<craftkit_sdk::Node> {
    session.document().node(id).cloned()
}

/// The **numbers** of a value.
fn numbers_of(value: &craftkit_sdk::serde_json::Value) -> Vec<f64> {
    value.as_array().map(|list| list.iter().filter_map(|item| item.as_f64()).collect()).unwrap_or_default()
}

/// The **text** of a value.
fn text_of(value: &craftkit_sdk::serde_json::Value) -> Option<String> {
    match value {
        craftkit_sdk::serde_json::Value::String(text) => Some(text.clone()),
        craftkit_sdk::serde_json::Value::Object(map) => {
            map.get("string").and_then(|text| text.as_str()).map(|text| text.to_string())
        }
        _ => None,
    }
}

/// The **pairs** of an inline dictionary.
fn dictionary_of(value: &craftkit_sdk::serde_json::Value) -> Vec<(String, craftkit_sdk::serde_json::Value)> {
    let Some(list) = value.get("entries").and_then(|entries| entries.as_array()) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|entry| {
            let pair = entry.as_array()?;
            Some((pair.first()?.as_str()?.to_string(), pair.get(1)?.clone()))
        })
        .collect()
}

/// The **catalog** of the file.
fn the_catalog(session: &Session) -> craftkit_sdk::Node {
    session
        .document()
        .walk()
        .find(|node| {
            entries_of(node).iter().any(|(key, value)| key == "Type" && value.as_str() == Some("Catalog"))
        })
        .cloned()
        .expect("the catalog is there")
}

/// The **form** of the file: the dictionary the catalog names.
fn the_form(session: &Session) -> Option<craftkit_sdk::Node> {
    let catalog = the_catalog(session);
    let entries = entries_of(&catalog);
    let value = entry(&entries, "AcroForm")?;
    let number = value.get("ref").and_then(|pair| pair.as_array()).and_then(|pair| pair.first())?.as_u64()?;
    the_field(session, number)
}

/// The **field** of an object number.
fn the_field(session: &Session, object: u64) -> Option<craftkit_sdk::Node> {
    session
        .document()
        .walk()
        .find(|node| node.data.get("object").and_then(|held| held.as_u64()) == Some(object))
        .cloned()
}

/// **A field is put on the page.** The widget says what the tree's
/// widget says: it is an annotation of the kind `Widget`, it has the
/// rectangle, it names the page, it has the flags, the colours and the
/// border, and it is a text field of twelve point Helvetica.
#[test]
fn a_field_is_put_on_the_page() {
    let mut session = session();
    let directory = directory("field");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.field.add",
            &json!({ "page": page, "object": 10, "rect": [100.0, 200.0, 300.0, 240.0] }),
        )
        .expect("the field is added");
    assert!(outcome.changed);
    let object = outcome.data["object"].as_u64().expect("the object is there");
    let field = the_field(&session, object).expect("the field is there");
    let entries = entries_of(&field);
    assert_eq!(entry(&entries, "Type").and_then(|value| value.as_str()), Some("Annot"));
    assert_eq!(entry(&entries, "Subtype").and_then(|value| value.as_str()), Some("Widget"));
    assert_eq!(numbers_of(entry(&entries, "Rect").expect("the rectangle")), vec![100.0, 200.0, 300.0, 240.0]);
    assert_eq!(
        entry(&entries, "P")
            .and_then(|value| value.get("ref"))
            .and_then(|pair| pair.as_array())
            .and_then(|pair| pair.first())
            .and_then(|number| number.as_u64()),
        Some(3)
    );
    assert_eq!(entry(&entries, "F").and_then(|value| value.as_f64()), Some(4.0));
    assert_eq!(entry(&entries, "FT").and_then(|value| value.as_str()), Some("Tx"));
    assert_eq!(
        text_of(entry(&entries, "DA").expect("the default appearance")),
        Some("/Helv 12 Tf 0 g".to_string())
    );
    assert_eq!(text_of(entry(&entries, "T").expect("the name")), Some("Text1".to_string()));
    // The **colours** and the **border**.
    let mark = dictionary_of(entry(&entries, "MK").expect("the colours"));
    assert_eq!(numbers_of(entry(&mark, "BC").expect("the border's colour")), vec![0.55, 0.55, 0.55]);
    assert_eq!(numbers_of(entry(&mark, "BG").expect("the background's colour")), vec![1.0, 1.0, 1.0]);
    let border = dictionary_of(entry(&entries, "BS").expect("the border"));
    assert_eq!(entry(&border, "W").and_then(|value| value.as_f64()), Some(1.0));
    assert_eq!(entry(&border, "S").and_then(|value| value.as_str()), Some("S"));
}

/// **The page holds the field.** The field is an annotation of the
/// page, and the page's `/Annots` says so.
#[test]
fn the_page_holds_the_field() {
    let mut session = session();
    let directory = directory("annots");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.field.add",
            &json!({ "page": page, "object": 10, "rect": [100.0, 200.0, 300.0, 240.0] }),
        )
        .expect("the field is added");
    let object = outcome.data["object"].as_u64().expect("the object is there");
    let node = node_of(&session, page).expect("the page is there");
    let annots = entry(&entries_of(&node), "Annots")
        .and_then(|annots| annots.as_array().cloned())
        .expect("the annotations");
    assert_eq!(annots.len(), 1);
    let number = annots[0]
        .get("ref")
        .and_then(|pair| pair.as_array())
        .and_then(|pair| pair.first())
        .and_then(|number| number.as_u64())
        .expect("the reference is there");
    assert_eq!(number, object);
}

/// **The form is made and holds the field.** The catalog names the
/// form, the form holds the fields, it says how a field's text looks
/// by default, and it holds the three standard fonts under the names
/// Acrobat uses.
#[test]
fn the_form_is_made_and_holds_the_field() {
    let mut session = session();
    let directory = directory("form");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.field.add",
            &json!({ "page": page, "object": 10, "rect": [100.0, 200.0, 300.0, 240.0] }),
        )
        .expect("the field is added");
    let object = outcome.data["object"].as_u64().expect("the object is there");
    let form = the_form(&session).expect("the form is there");
    let entries = entries_of(&form);
    // The **fields** of the form.
    let fields = entry(&entries, "Fields").and_then(|fields| fields.as_array().cloned()).expect("the fields");
    assert_eq!(fields.len(), 1);
    let number = fields[0]
        .get("ref")
        .and_then(|pair| pair.as_array())
        .and_then(|pair| pair.first())
        .and_then(|number| number.as_u64())
        .expect("the reference is there");
    assert_eq!(number, object);
    // The **default appearance** of the form.
    assert_eq!(
        text_of(entry(&entries, "DA").expect("the default appearance")),
        Some("/Helv 0 Tf 0 g".to_string())
    );
    // The **fonts** of the form.
    let resources = dictionary_of(entry(&entries, "DR").expect("the resources"));
    let fonts = dictionary_of(entry(&resources, "Font").expect("the fonts"));
    let names: Vec<String> = fonts.iter().map(|(name, _)| name.clone()).collect();
    assert_eq!(names, vec!["Helv".to_string(), "TiRo".to_string(), "Cour".to_string()]);
    // Each font is a font of the file, and it is the font the tree
    // names.
    for (name, value) in &fonts {
        let number = value
            .get("ref")
            .and_then(|pair| pair.as_array())
            .and_then(|pair| pair.first())
            .and_then(|number| number.as_u64())
            .expect("the font is an object");
        let font = the_field(&session, number).expect("the font is there");
        let font_entries = entries_of(&font);
        assert_eq!(entry(&font_entries, "Type").and_then(|value| value.as_str()), Some("Font"));
        assert_eq!(entry(&font_entries, "Subtype").and_then(|value| value.as_str()), Some("Type1"));
        assert_eq!(
            entry(&font_entries, "Encoding").and_then(|value| value.as_str()),
            Some("WinAnsiEncoding")
        );
        let base =
            entry(&font_entries, "BaseFont").and_then(|value| value.as_str()).unwrap_or_default().to_string();
        let expected = match name.as_str() {
            "Helv" => "Helvetica",
            "TiRo" => "Times-Roman",
            _ => "Courier",
        };
        assert_eq!(base, expected);
    }
}

/// **The name is free.** A field with no name is given the first free
/// name of the tree: `Text1`, then `Text2`.
#[test]
fn the_name_is_free() {
    let mut session = session();
    let directory = directory("names");
    let page = a_page(&mut session, &directory);
    let first = session
        .execute(
            "pdf.field.add",
            &json!({ "page": page, "object": 10, "rect": [100.0, 200.0, 300.0, 240.0] }),
        )
        .expect("the field is added");
    assert_eq!(first.data["name"].as_str(), Some("Text1"));
    let second = session
        .execute(
            "pdf.field.add",
            &json!({ "page": page, "object": 11, "rect": [100.0, 300.0, 300.0, 340.0] }),
        )
        .expect("the field is added");
    assert_eq!(second.data["name"].as_str(), Some("Text2"));
}

/// **A name with a period is refused,** and so is a name another field
/// has.
#[test]
fn a_bad_name_is_refused() {
    let mut session = session();
    let directory = directory("badname");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute(
            "pdf.field.add",
            &json!({ "page": page, "object": 10, "rect": [100.0, 200.0, 300.0, 240.0], "name": "a.b" }),
        )
        .expect_err("the name is refused");
    assert!(error.to_string().contains("period"), "the error says why: {error}");
    session
        .execute(
            "pdf.field.add",
            &json!({ "page": page, "object": 10, "rect": [100.0, 200.0, 300.0, 240.0], "name": "Name" }),
        )
        .expect("the field is added");
    let error = session
        .execute(
            "pdf.field.add",
            &json!({ "page": page, "object": 11, "rect": [100.0, 300.0, 300.0, 340.0], "name": "Name" }),
        )
        .expect_err("the name is refused");
    assert!(error.to_string().contains("already exists"), "the error says why: {error}");
}

/// **The corners may come in any order,** and a field that is too
/// small is refused.
#[test]
fn the_rectangle_is_normed_and_too_small_is_refused() {
    let mut session = session();
    let directory = directory("rect");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.field.add",
            &json!({ "page": page, "object": 10, "rect": [300.0, 240.0, 100.0, 200.0] }),
        )
        .expect("the field is added");
    let object = outcome.data["object"].as_u64().expect("the object is there");
    let field = the_field(&session, object).expect("the field is there");
    let rect = numbers_of(entry(&entries_of(&field), "Rect").expect("the rectangle"));
    assert_eq!(rect, vec![100.0, 200.0, 300.0, 240.0]);
    let error = session
        .execute(
            "pdf.field.add",
            &json!({ "page": page, "object": 11, "rect": [100.0, 200.0, 102.0, 202.0] }),
        )
        .expect_err("the field is refused");
    assert!(error.to_string().contains("too small"), "the error says why: {error}");
}

/// **A multiline field says so,** and it holds the text it was given.
#[test]
fn a_multiline_field_says_so() {
    let mut session = session();
    let directory = directory("multiline");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.field.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 200.0, 300.0, 280.0],
                "multiline": true,
                "value": "hello",
            }),
        )
        .expect("the field is added");
    let object = outcome.data["object"].as_u64().expect("the object is there");
    let field = the_field(&session, object).expect("the field is there");
    let entries = entries_of(&field);
    assert_eq!(entry(&entries, "Ff").and_then(|value| value.as_f64()), Some(4096.0));
    assert_eq!(text_of(entry(&entries, "V").expect("the value")), Some("hello".to_string()));
}

/// **The drawing of the field is written** where the caller says, and
/// the field names it.
#[test]
fn the_drawing_is_written() {
    let mut session = session();
    let directory = directory("drawing");
    let page = a_page(&mut session, &directory);
    let to = format!("{directory}/drawing.txt");
    let outcome = session
        .execute(
            "pdf.field.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 200.0, 300.0, 240.0],
                "value": "hello",
                "appearance_to": to,
            }),
        )
        .expect("the field is added");
    let object = outcome.data["object"].as_u64().expect("the object is there");
    let field = the_field(&session, object).expect("the field is there");
    // The **appearance**: the normal one, under `N`.
    let appearances = dictionary_of(entry(&entries_of(&field), "AP").expect("the appearance"));
    let normal = entry(&appearances, "N").expect("the normal appearance");
    let number = normal
        .get("ref")
        .and_then(|pair| pair.as_array())
        .and_then(|pair| pair.first())
        .and_then(|number| number.as_u64())
        .expect("the appearance is an object");
    let stream = the_field(&session, number).expect("the appearance is there");
    let entries = entries_of(&stream);
    assert_eq!(entry(&entries, "Type").and_then(|value| value.as_str()), Some("XObject"));
    assert_eq!(entry(&entries, "Subtype").and_then(|value| value.as_str()), Some("Form"));
    assert_eq!(numbers_of(entry(&entries, "BBox").expect("the box")), vec![0.0, 0.0, 200.0, 40.0]);
    // The **bytes**: the frame and the text.
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    assert!(bytes.contains("1 1 1 rg"), "the background is filled: {bytes}");
    assert!(bytes.contains("0.55 0.55 0.55 RG"), "the border is stroked: {bytes}");
    assert!(bytes.contains("(hello) Tj"), "the text is drawn: {bytes}");
}

/// **The drawing of a multiline field wraps** its text.
#[test]
fn the_drawing_of_a_multiline_field_wraps() {
    let mut session = session();
    let directory = directory("wrapped");
    let page = a_page(&mut session, &directory);
    let to = format!("{directory}/drawing.txt");
    session
        .execute(
            "pdf.field.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 200.0, 160.0, 280.0],
                "multiline": true,
                "value": "one two three four",
                "appearance_to": to,
            }),
        )
        .expect("the field is added");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    assert!(bytes.contains("(one two) Tj"), "the text is wrapped: {bytes}");
    assert!(bytes.contains("(three four) Tj"), "the text is wrapped: {bytes}");
}

/// **The object number is the field's own,** and a number that is
/// taken is refused.
#[test]
fn the_object_number_is_the_fields_own() {
    let mut session = session();
    let directory = directory("number");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute("pdf.field.add", &json!({ "page": page, "object": 3, "rect": [100.0, 200.0, 300.0, 240.0] }))
        .expect_err("the number is refused");
    assert!(error.to_string().contains("already in the document"), "the error says why: {error}");
    let error = session
        .execute("pdf.field.add", &json!({ "page": page, "object": 0, "rect": [100.0, 200.0, 300.0, 240.0] }))
        .expect_err("the number is refused");
    assert!(error.to_string().contains("start at one"), "the error says why: {error}");
}

/// **A node that is not a page** is refused.
#[test]
fn a_node_that_is_not_a_page_is_refused() {
    let mut session = session();
    let directory = directory("notapage");
    let page = a_page(&mut session, &directory);
    let catalog = the_catalog(&session);
    let error = session
        .execute(
            "pdf.field.add",
            &json!({ "page": catalog.id, "object": 10, "rect": [100.0, 200.0, 300.0, 240.0] }),
        )
        .expect_err("the catalog is refused");
    assert!(error.to_string().contains("not a page"), "the error says why: {error}");
    let _ = page;
}
