//! The **transition** of a page.

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_pdf::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the pdf battery.
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

/// A directory of this test's own, so that the files do not mix.
fn directory(name: &str) -> String {
    let path = std::env::temp_dir().join(format!("craftkit-v2-transitions-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file whose page has the **transition** given.
fn a_file(transition: Option<&str>, duration: Option<f64>) -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    let mut page = String::from("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842]");
    if let Some(transition) = transition {
        page.push_str(&format!(" /Trans {transition}"));
    }
    if let Some(duration) = duration {
        page.push_str(&format!(" /Dur {duration}"));
    }
    page.push_str(" >>");
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (3, page),
    ] {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    file.extend_from_slice(b"xref\n0 4\n");
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(b"trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n");
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    file
}

/// Load a file into the document, and give the **page** it holds.
fn load(session: &mut Session, name: &str, transition: Option<&str>, duration: Option<f64>) -> u64 {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, a_file(transition, duration)).expect("the file is written");
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

/// The **transition** of the page, as the command reads it.
fn transition_of(session: &mut Session, page: u64) -> Json {
    let outcome =
        session.execute("pdf.page.transition", &json!({ "page": page })).expect("the transition is read");
    outcome.data.clone()
}

/// The transition of a page is read: the kind, how long it takes and
/// the way it moves.
#[test]
fn the_transition_of_a_page_is_read() {
    let mut session = session();
    let page = load(&mut session, "wipe", Some("<< /S /Wipe /D 1.5 /Dm /H /M /In /Di 90 >>"), None);
    let transition = transition_of(&mut session, page);
    let read = &transition["transition"];
    assert_eq!(read["kind"], json!("Wipe"), "the kind is read");
    assert_eq!(read["duration"], json!(1.5), "and how long it takes");
    assert_eq!(read["dimension"], json!("H"), "and the dimension it moves along");
    assert_eq!(read["direction"], json!("In"), "and the way it moves");
    assert_eq!(read["angle"], json!(90.0), "and the angle of the wipe");
}

/// A page with **no transition** has none, and a page may say how
/// long it is to be shown.
#[test]
fn a_page_with_no_transition_has_none() {
    let mut session = session();
    let page = load(&mut session, "none", None, Some(5.0));
    let transition = transition_of(&mut session, page);
    assert_eq!(transition["transition"], Json::Null, "the page has no transition");
    assert_eq!(transition["duration"], json!(5.0), "and it is shown for five seconds");
}

/// A kind the tree has **not heard of** is read as the file says it.
#[test]
fn a_kind_the_tree_has_not_heard_of_is_read_as_the_file_says_it() {
    let mut session = session();
    let page = load(&mut session, "unknown", Some("<< /S /Unheard >>"), None);
    let transition = transition_of(&mut session, page);
    assert_eq!(transition["transition"]["kind"], json!("Unheard"), "the name is the fact");
}

/// A transition that is **not a dictionary** is refused.
#[test]
fn a_transition_that_is_not_a_dictionary_is_refused() {
    let mut session = session();
    let page = load(&mut session, "broken", Some("(not a dictionary)"), None);
    let outcome = session.execute("pdf.page.transition", &json!({ "page": page }));
    assert!(outcome.is_err(), "a transition that is not a dictionary is refused");
    let message = outcome.err().map(|error| error.to_string()).unwrap_or_default();
    assert!(message.contains("not a dictionary"), "and it says why: {message}");
}
