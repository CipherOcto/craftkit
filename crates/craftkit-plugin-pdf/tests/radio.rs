//! The **radio button** that is put on a page.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-radio-{name}-{}", std::process::id()))
        .expect("the directory is made");
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

/// The **node** of an object number.
fn the_object(session: &Session, object: u64) -> Option<craftkit_sdk::Node> {
    session
        .document()
        .walk()
        .find(|node| node.data.get("object").and_then(|held| held.as_u64()) == Some(object))
        .cloned()
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

/// The **number** a reference names.
fn the_number(value: &craftkit_sdk::serde_json::Value) -> Option<u64> {
    value
        .get("ref")
        .and_then(|pair| pair.as_array())
        .and_then(|pair| pair.first())
        .and_then(|number| number.as_u64())
}

/// **A radio button is put on the page.** It is a widget, it is off,
/// and it names the group it belongs to.
#[test]
fn a_radio_button_is_put_on_the_page() {
    let mut session = session();
    let directory = directory("button");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.field.radio.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 200.0, 120.0, 220.0],
                "export": "Yes",
            }),
        )
        .expect("the radio button is added");
    assert!(outcome.changed);
    let object = outcome.data["object"].as_u64().expect("the object is there");
    let button = the_object(&session, object).expect("the button is there");
    let entries = entries_of(&button);
    assert_eq!(entry(&entries, "Type").and_then(|value| value.as_str()), Some("Annot"));
    assert_eq!(entry(&entries, "Subtype").and_then(|value| value.as_str()), Some("Widget"));
    assert_eq!(entry(&entries, "AS").and_then(|value| value.as_str()), Some("Off"));
    assert_eq!(entry(&entries, "F").and_then(|value| value.as_f64()), Some(4.0));
    // The **group** the button names.
    let group = the_number(entry(&entries, "Parent").expect("the group")).expect("the group is an object");
    assert_eq!(group, outcome.data["group_object"].as_u64().expect("the group is there"));
    // The **mark** of a radio button is a circle.
    let mark = dictionary_of(entry(&entries, "MK").expect("the colours"));
    assert_eq!(text_of(entry(&mark, "CA").expect("the style")), Some("l".to_string()));
}

/// **The group is made** — a field of its own, with the tree's flags,
/// and the form holds the group and not the button.
#[test]
fn the_group_is_made() {
    let mut session = session();
    let directory = directory("group");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.field.radio.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 200.0, 120.0, 220.0],
                "export": "Yes",
            }),
        )
        .expect("the radio button is added");
    assert_eq!(outcome.data["group"].as_str(), Some("Group1"));
    let group = the_object(&session, outcome.data["group_object"].as_u64().expect("the group"))
        .expect("the group is there");
    let entries = entries_of(&group);
    assert_eq!(entry(&entries, "FT").and_then(|value| value.as_str()), Some("Btn"));
    // The tree's own flags: the radio flag and the flag that says the
    // group cannot be turned off.
    assert_eq!(entry(&entries, "Ff").and_then(|value| value.as_f64()), Some(49152.0));
    assert_eq!(text_of(entry(&entries, "T").expect("the name")), Some("Group1".to_string()));
    assert_eq!(entry(&entries, "V").and_then(|value| value.as_str()), Some("Off"));
    assert_eq!(
        text_of(entry(&entries, "DA").expect("the default appearance")),
        Some("/ZaDb 0 Tf 0 g".to_string())
    );
    // The **kids** of the group: the button.
    let kids = entry(&entries, "Kids").and_then(|kids| kids.as_array().cloned()).expect("the kids");
    assert_eq!(kids.len(), 1);
    assert_eq!(the_number(&kids[0]), Some(10));
    // The **form** holds the group.
    let catalog = session
        .document()
        .walk()
        .find(|node| {
            entries_of(node).iter().any(|(key, value)| key == "Type" && value.as_str() == Some("Catalog"))
        })
        .cloned()
        .expect("the catalog is there");
    let catalog_entries = entries_of(&catalog);
    let acro = entry(&catalog_entries, "AcroForm").expect("the form");
    let form =
        the_object(&session, the_number(acro).expect("the form is an object")).expect("the form is there");
    let fields = entry(&entries_of(&form), "Fields")
        .and_then(|fields| fields.as_array().cloned())
        .expect("the fields");
    assert_eq!(fields.len(), 1);
    assert_eq!(the_number(&fields[0]), Some(outcome.data["group_object"].as_u64().expect("the group")));
}

/// **A second button joins the group** and no second group is made.
#[test]
fn a_second_button_joins_the_group() {
    let mut session = session();
    let directory = directory("join");
    let page = a_page(&mut session, &directory);
    let first = session
        .execute(
            "pdf.field.radio.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 200.0, 120.0, 220.0],
                "export": "Yes",
            }),
        )
        .expect("the radio button is added");
    let group_object = first.data["group_object"].as_u64().expect("the group");
    let second = session
        .execute(
            "pdf.field.radio.add",
            &json!({
                "page": page,
                "object": 20,
                "rect": [100.0, 300.0, 120.0, 320.0],
                "export": "No",
                "group": "Group1",
            }),
        )
        .expect("the radio button is added");
    assert_eq!(second.data["group_object"].as_u64().expect("the group"), group_object);
    // The group holds **two** kids now.
    let group = the_object(&session, group_object).expect("the group is there");
    let kids =
        entry(&entries_of(&group), "Kids").and_then(|kids| kids.as_array().cloned()).expect("the kids");
    assert_eq!(kids.len(), 2);
}

/// **The export value** may not be empty and may not be `Off`.
#[test]
fn a_bad_export_value_is_refused() {
    let mut session = session();
    let directory = directory("export");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute(
            "pdf.field.radio.add",
            &json!({ "page": page, "object": 10, "rect": [100.0, 200.0, 120.0, 220.0], "export": "Off" }),
        )
        .expect_err("the export value is refused");
    assert!(error.to_string().contains("export value"), "the error says why: {error}");
    let error = session
        .execute(
            "pdf.field.radio.add",
            &json!({ "page": page, "object": 10, "rect": [100.0, 200.0, 120.0, 220.0], "export": "  " }),
        )
        .expect_err("the export value is refused");
    assert!(error.to_string().contains("export value"), "the error says why: {error}");
}

/// **A group name with a period is refused,** and so is a name a
/// field that is not a group has.
#[test]
fn a_bad_group_name_is_refused() {
    let mut session = session();
    let directory = directory("badgroup");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute(
            "pdf.field.radio.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 200.0, 120.0, 220.0],
                "export": "Yes",
                "group": "a.b",
            }),
        )
        .expect_err("the name is refused");
    assert!(error.to_string().contains("period"), "the error says why: {error}");
    // A **text field** named `Group1` is not a group of radio buttons.
    session
        .execute(
            "pdf.field.add",
            &json!({ "page": page, "object": 10, "rect": [100.0, 400.0, 300.0, 440.0], "name": "Group1" }),
        )
        .expect("the field is added");
    let error = session
        .execute(
            "pdf.field.radio.add",
            &json!({
                "page": page,
                "object": 20,
                "rect": [100.0, 300.0, 120.0, 320.0],
                "export": "Yes",
                "group": "Group1",
            }),
        )
        .expect_err("the group is refused");
    assert!(error.to_string().contains("not a group"), "the error says why: {error}");
}

/// **The two drawings** of the button: the one that says it is off,
/// and the one that says it is on — and the on one is called the
/// export value.
#[test]
fn the_two_drawings_are_written() {
    let mut session = session();
    let directory = directory("drawings");
    let page = a_page(&mut session, &directory);
    let on = format!("{directory}/on.txt");
    let off = format!("{directory}/off.txt");
    let outcome = session
        .execute(
            "pdf.field.radio.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 200.0, 120.0, 220.0],
                "export": "Yes",
                "on_to": on,
                "off_to": off,
            }),
        )
        .expect("the radio button is added");
    let object = outcome.data["object"].as_u64().expect("the object is there");
    let button = the_object(&session, object).expect("the button is there");
    let appearances = dictionary_of(entry(&entries_of(&button), "AP").expect("the appearance"));
    let normal = dictionary_of(entry(&appearances, "N").expect("the normal appearance"));
    let states: Vec<String> = normal.iter().map(|(name, _)| name.clone()).collect();
    assert_eq!(states, vec!["Yes".to_string(), "Off".to_string()]);
    // The **mark** of a radio button is a circle.
    let bytes = std::fs::read_to_string(&on).expect("the drawing is there");
    assert!(bytes.contains(" c "), "the circle is drawn with curves: {bytes}");
}

/// **The page holds the button** as an annotation.
#[test]
fn the_page_holds_the_button() {
    let mut session = session();
    let directory = directory("annots");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.field.radio.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 200.0, 120.0, 220.0],
                "export": "Yes",
            }),
        )
        .expect("the radio button is added");
    let object = outcome.data["object"].as_u64().expect("the object is there");
    let page_node = session.document().node(page).cloned().expect("the page is there");
    let annots = entry(&entries_of(&page_node), "Annots")
        .and_then(|annots| annots.as_array().cloned())
        .expect("the annotations");
    assert_eq!(annots.len(), 1);
    assert_eq!(the_number(&annots[0]), Some(object));
}
