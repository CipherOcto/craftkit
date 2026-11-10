//! The **group** of radio buttons, and the button of it that is on.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-radiogroup-{name}-{}", std::process::id()));
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

/// The **node** of an object number.
fn the_object(session: &Session, object: u64) -> Option<craftkit_sdk::Node> {
    session
        .document()
        .walk()
        .find(|node| node.data.get("object").and_then(|held| held.as_u64()) == Some(object))
        .cloned()
}

/// The **number** a reference names.
fn the_number(value: &craftkit_sdk::serde_json::Value) -> Option<u64> {
    value
        .get("ref")
        .and_then(|pair| pair.as_array())
        .and_then(|pair| pair.first())
        .and_then(|number| number.as_u64())
}

/// The **buttons** of a group: the two of them, with their drawings.
fn a_group(session: &mut Session, directory: &str) -> (u64, u64, u64) {
    let page = a_page(session, directory);
    let first = session
        .execute(
            "pdf.field.radio.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 200.0, 120.0, 220.0],
                "export": "Yes",
                "on_to": format!("{directory}/yes-on.txt"),
                "off_to": format!("{directory}/yes-off.txt"),
            }),
        )
        .expect("the radio button is added");
    let group = first.data["group_object"].as_u64().expect("the group");
    session
        .execute(
            "pdf.field.radio.add",
            &json!({
                "page": page,
                "object": 20,
                "rect": [100.0, 300.0, 120.0, 320.0],
                "export": "No",
                "group": "Group1",
                "on_to": format!("{directory}/no-on.txt"),
                "off_to": format!("{directory}/no-off.txt"),
            }),
        )
        .expect("the radio button is added");
    (page, group, first.data["object"].as_u64().expect("the object"))
}

/// **The group has the states of its buttons,** and a value that is
/// not one of them is refused.
#[test]
fn the_group_has_the_states_of_its_buttons() {
    let mut session = session();
    let directory = directory("states");
    let (_, group, _) = a_group(&mut session, &directory);
    let node = the_object(&session, group).expect("the group is there");
    let group_id = node.id;
    let error = session
        .execute("pdf.field.set", &json!({ "field": group_id, "value": "Maybe" }))
        .expect_err("the value is refused");
    assert!(error.to_string().contains("not one of the states"), "the error says why: {error}");
}

/// **The button of the group that is on holds the value,** and every
/// other button of the group is off.
#[test]
fn the_button_that_is_on_holds_the_value() {
    let mut session = session();
    let directory = directory("on");
    let (_, group, first) = a_group(&mut session, &directory);
    let node = the_object(&session, group).expect("the group is there");
    let group_id = node.id;
    session
        .execute("pdf.field.set", &json!({ "field": group_id, "value": "Yes" }))
        .expect("the value is put");
    // The **group** holds the value.
    let group = the_object(&session, group).expect("the group is there");
    assert_eq!(entry(&entries_of(&group), "V").and_then(|value| value.as_str()), Some("Yes"));
    // The **button** that has that state is on.
    let button = the_object(&session, first).expect("the button is there");
    let entries = entries_of(&button);
    assert_eq!(entry(&entries, "V").and_then(|value| value.as_str()), Some("Yes"));
    assert_eq!(entry(&entries, "AS").and_then(|value| value.as_str()), Some("Yes"));
    // The **other** button of the group is off.
    let other = the_object(&session, 20).expect("the other button is there");
    let entries = entries_of(&other);
    assert_eq!(entry(&entries, "V").and_then(|value| value.as_str()), Some("Off"));
    assert_eq!(entry(&entries, "AS").and_then(|value| value.as_str()), Some("Off"));
}

/// **One button of the group is on at a time:** turning another one on
/// turns the first one off.
#[test]
fn only_one_button_of_the_group_is_on() {
    let mut session = session();
    let directory = directory("one");
    let (_, group, first) = a_group(&mut session, &directory);
    let node = the_object(&session, group).expect("the group is there");
    let group_id = node.id;
    session
        .execute("pdf.field.set", &json!({ "field": group_id, "value": "Yes" }))
        .expect("the value is put");
    session.execute("pdf.field.set", &json!({ "field": group_id, "value": "No" })).expect("the value is put");
    // The **first** button is off now.
    let button = the_object(&session, first).expect("the button is there");
    let entries = entries_of(&button);
    assert_eq!(entry(&entries, "V").and_then(|value| value.as_str()), Some("Off"));
    assert_eq!(entry(&entries, "AS").and_then(|value| value.as_str()), Some("Off"));
    // The **second** one is on.
    let other = the_object(&session, 20).expect("the other button is there");
    let entries = entries_of(&other);
    assert_eq!(entry(&entries, "V").and_then(|value| value.as_str()), Some("No"));
    assert_eq!(entry(&entries, "AS").and_then(|value| value.as_str()), Some("No"));
    // The group holds the value of the button that is on.
    let group = the_object(&session, group).expect("the group is there");
    assert_eq!(entry(&entries_of(&group), "V").and_then(|value| value.as_str()), Some("No"));
}

/// **The group is turned off,** and every button of it is off.
#[test]
fn the_group_is_turned_off() {
    let mut session = session();
    let directory = directory("off");
    let (_, group, first) = a_group(&mut session, &directory);
    let node = the_object(&session, group).expect("the group is there");
    let group_id = node.id;
    session
        .execute("pdf.field.set", &json!({ "field": group_id, "value": "Yes" }))
        .expect("the value is put");
    session
        .execute("pdf.field.set", &json!({ "field": group_id, "value": "Off" }))
        .expect("the value is put");
    let button = the_object(&session, first).expect("the button is there");
    let entries = entries_of(&button);
    assert_eq!(entry(&entries, "V").and_then(|value| value.as_str()), Some("Off"));
    assert_eq!(entry(&entries, "AS").and_then(|value| value.as_str()), Some("Off"));
    let other = the_object(&session, 20).expect("the other button is there");
    let entries = entries_of(&other);
    assert_eq!(entry(&entries, "AS").and_then(|value| value.as_str()), Some("Off"));
}

/// **An empty value takes the value away,** and every button of the
/// group is off.
#[test]
fn an_empty_value_takes_the_value_away() {
    let mut session = session();
    let directory = directory("empty");
    let (_, group, first) = a_group(&mut session, &directory);
    let node = the_object(&session, group).expect("the group is there");
    let group_id = node.id;
    session
        .execute("pdf.field.set", &json!({ "field": group_id, "value": "Yes" }))
        .expect("the value is put");
    session
        .execute("pdf.field.set", &json!({ "field": group_id, "value": "" }))
        .expect("the value is taken away");
    let group = the_object(&session, group).expect("the group is there");
    assert!(entry(&entries_of(&group), "V").is_none(), "the group holds nothing");
    let button = the_object(&session, first).expect("the button is there");
    let entries = entries_of(&button);
    assert_eq!(entry(&entries, "AS").and_then(|value| value.as_str()), Some("Off"));
}

/// **The kids of the group** are the buttons, and the group names them.
#[test]
fn the_kids_of_the_group_are_the_buttons() {
    let mut session = session();
    let directory = directory("kids");
    let (_, group, first) = a_group(&mut session, &directory);
    let node = the_object(&session, group).expect("the group is there");
    let kids = entry(&entries_of(&node), "Kids").and_then(|kids| kids.as_array().cloned()).expect("the kids");
    assert_eq!(kids.len(), 2);
    assert_eq!(the_number(&kids[0]), Some(first));
    // The button **names** the group it belongs to.
    let button = the_object(&session, first).expect("the button is there");
    let button_entries = entries_of(&button);
    let parent = entry(&button_entries, "Parent").expect("the group");
    assert_eq!(the_number(parent), Some(group));
}
