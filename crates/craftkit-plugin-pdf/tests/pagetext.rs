//! The **text** of a page.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-pagetext-{name}-{}", std::process::id()))
        .expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file whose page has the **content** given.
fn a_file(content: &str) -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R >>".to_string()),
        (4, format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len())),
    ] {
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

/// Load a file into the document, and give the **page** it holds.
fn load(session: &mut Session, name: &str, content: &str) -> u64 {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, a_file(content)).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
    the_page(session)
}

/// The **page** of the document: the node whose type is `Page`.
fn the_page(session: &Session) -> u64 {
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

/// The **text** of the page, as the command reads it.
fn text_of(session: &mut Session, page: u64) -> Json {
    let outcome = session.execute("pdf.page.text", &json!({ "page": page })).expect("the text is read");
    outcome.data.clone()
}

/// The text a page shows is read, piece by piece.
#[test]
fn the_text_of_a_page_is_read() {
    let mut session = session();
    let page = load(&mut session, "plain", "BT /F1 12 Tf (Hello) Tj (world) Tj ET");
    let text = text_of(&mut session, page);
    let pieces = text["pieces"].as_array().expect("the pieces are a list");
    assert_eq!(pieces.len(), 2, "the page shows two pieces");
    assert_eq!(pieces[0]["text"], json!("Hello"), "the first piece is read");
    assert_eq!(pieces[0]["operator"], json!("Tj"), "and the operator that showed it is named");
    assert_eq!(pieces[0]["encoding"], json!("latin-1"), "and the bytes were read as Latin-1");
    assert_eq!(pieces[1]["text"], json!("world"), "the second piece is read");
    assert_eq!(text["text"], json!("Helloworld"), "and the text is the two of them");
    assert_eq!(text["streams"], json!(1), "the text came from one stream");
}

/// A **number** in a `TJ` array is the space between two words.
#[test]
fn a_number_in_an_array_is_the_space_between_words() {
    let mut session = session();
    let page = load(&mut session, "spaced", "BT [(wor) -300 (ld)] TJ ET");
    let text = text_of(&mut session, page);
    let pieces = text["pieces"].as_array().expect("the pieces are a list");
    assert_eq!(pieces.len(), 1, "the array is one piece");
    assert_eq!(pieces[0]["text"], json!("wor ld"), "and the number is a space");
    assert_eq!(pieces[0]["operator"], json!("TJ"), "the operator is named");
}

/// A string of **UTF-16** is read as UTF-16.
#[test]
fn a_string_of_utf16_is_read_as_utf16() {
    let mut session = session();
    let page = load(&mut session, "utf16", "BT <FEFF00480065006C006C006F> Tj ET");
    let text = text_of(&mut session, page);
    let pieces = text["pieces"].as_array().expect("the pieces are a list");
    assert_eq!(pieces.len(), 1, "there is one piece");
    assert_eq!(pieces[0]["text"], json!("Hello"), "the text is read");
    assert_eq!(pieces[0]["encoding"], json!("utf-16be"), "and the bytes were read as UTF-16");
}

/// Text shown **outside** a text object is not text.
#[test]
fn text_shown_outside_a_text_object_is_not_text() {
    let mut session = session();
    let page = load(&mut session, "loose", "(loose) Tj BT (inside) Tj ET");
    let text = text_of(&mut session, page);
    let pieces = text["pieces"].as_array().expect("the pieces are a list");
    assert_eq!(pieces.len(), 1, "only the text inside the text object is read");
    assert_eq!(pieces[0]["text"], json!("inside"), "and it is the text inside");
}

/// A page with **no content** has no text, and that is not a failure.
#[test]
fn a_page_with_no_content_has_no_text() {
    let mut session = session();
    let directory = directory("empty");
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>"),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] >>"),
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
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, file).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
    let page = the_page(&session);
    let text = text_of(&mut session, page);
    assert_eq!(text["pieces"].as_array().map(Vec::len), Some(0), "the page shows nothing");
    assert_eq!(text["text"], json!(""), "and there is no text");
}

/// A node that is **not a page** is refused.
#[test]
fn a_node_that_is_not_a_page_is_refused() {
    let mut session = session();
    let page = load(&mut session, "refused", "BT (x) Tj ET");
    let other = session
        .document()
        .walk()
        .find(|node| node.id != page && node.kind == "pdf.object")
        .map(|node| node.id)
        .expect("there is another object");
    let outcome = session.execute("pdf.page.text", &json!({ "page": other }));
    assert!(outcome.is_err(), "the object is not a page and the command refuses it");
    let message = outcome.err().map(|error| error.to_string()).unwrap_or_default();
    assert!(message.contains("not a page"), "and it says why: {message}");
}
