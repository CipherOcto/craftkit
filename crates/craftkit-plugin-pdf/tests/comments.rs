//! The **comments** of a file: the annotations that are notes.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-comments-{name}-{}", std::process::id()))
        .expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// The **content** of the page.
const PAGE_CONTENT: &str = "0 0 1 rg 10 10 100 100 re f\n";

/// A file of PDF with **comments** on its page: a link, a widget, a
/// note, a highlight, a free text and a reply.
fn a_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    let bodies: Vec<(u32, String)> = vec![
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R /Annots [5 0 R 6 0 R 7 0 R 8 0 R 9 0 R 10 0 R] /Resources << >> >>"
                .to_string(),
        ),
        (
            4,
            format!("<< /Length {} >>\nstream\n{PAGE_CONTENT}endstream", PAGE_CONTENT.len()),
        ),
        (5, "<< /Type /Annot /Subtype /Link /Rect [0 0 10 10] >>".to_string()),
        (6, "<< /Type /Annot /Subtype /Widget /Rect [0 0 10 10] /FT /Tx >>".to_string()),
        (
            7,
            "<< /Type /Annot /Subtype /Text /Rect [320 720 300 700] /T (Ada) /Contents (A note) /M (D:20260101120000) /NM (note-1) /C [1 0 0] >>"
                .to_string(),
        ),
        (
            8,
            "<< /Type /Annot /Subtype /Highlight /Rect [10 100 200 120] /QuadPoints [10 100 200 100 10 120 200 120] /T (Bo) /Contents (marked) >>"
                .to_string(),
        ),
        (
            9,
            "<< /Type /Annot /Subtype /FreeText /Rect [10 300 200 340] /DA (/Helv 12 Tf 0 1 0 rg) >>"
                .to_string(),
        ),
        (
            10,
            "<< /Type /Annot /Subtype /Text /Rect [10 500 30 520] /IRT 7 0 R /F 128 /T (Cy) /Contents (A reply) >>"
                .to_string(),
        ),
    ];
    for (number, body) in &bodies {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    file.extend_from_slice(b"xref\n0 11\n");
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(b"trailer\n<< /Size 11 /Root 1 0 R >>\nstartxref\n");
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    file
}

/// Load the file and give the **page**.
fn a_page(session: &mut Session, name: &str) -> u64 {
    let directory = directory(name);
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

/// The **comments** of the file.
fn comments(session: &mut Session) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome = session.execute("pdf.comment.read", &json!({})).expect("the comments are read");
    outcome.data["comments"].as_array().cloned().expect("the comments are there")
}

/// **A link and a widget are not comments.**
#[test]
fn a_link_and_a_widget_are_not_comments() {
    let mut session = session();
    a_page(&mut session, "kinds");
    let comments = comments(&mut session);
    assert_eq!(comments.len(), 4, "the four notes are there: {comments:?}");
    let kinds: Vec<String> =
        comments.iter().map(|comment| comment["subtype"].as_str().unwrap_or_default().to_string()).collect();
    assert!(!kinds.iter().any(|kind| kind == "Link"), "the link is not a comment: {kinds:?}");
    assert!(!kinds.iter().any(|kind| kind == "Widget"), "the widget is not a comment: {kinds:?}");
}

/// **A comment says who wrote it and what it says.**
#[test]
fn a_comment_says_who_wrote_it_and_what_it_says() {
    let mut session = session();
    a_page(&mut session, "says");
    let comments = comments(&mut session);
    let note = comments.iter().find(|comment| comment["name"] == json!("note-1")).expect("the note is there");
    assert_eq!(note["author"], json!("Ada"), "the author is said: {note:?}");
    assert_eq!(note["contents"], json!("A note"), "the contents are said: {note:?}");
    assert_eq!(note["modified"], json!("D:20260101120000"), "the day is said: {note:?}");
}

/// **The rectangle of a comment is two corners in order.**
#[test]
fn the_rectangle_of_a_comment_is_two_corners_in_order() {
    let mut session = session();
    a_page(&mut session, "rect");
    let comments = comments(&mut session);
    let note = comments.iter().find(|comment| comment["name"] == json!("note-1")).expect("the note is there");
    assert_eq!(note["rect"], json!([300.0, 700.0, 320.0, 720.0]), "the rectangle is in order: {note:?}");
}

/// **The colour of a free text is the colour of its appearance.**
#[test]
fn the_colour_of_a_free_text_is_the_colour_of_its_appearance() {
    let mut session = session();
    a_page(&mut session, "colour");
    let comments = comments(&mut session);
    let free = comments
        .iter()
        .find(|comment| comment["subtype"] == json!("FreeText"))
        .expect("the free text is there");
    assert_eq!(free["colour"], json!([0.0, 1.0, 0.0]), "the colour is the tree's: {free:?}");
    let note = comments.iter().find(|comment| comment["name"] == json!("note-1")).expect("the note is there");
    assert_eq!(note["colour"], json!([1.0, 0.0, 0.0]), "and the note keeps its own: {note:?}");
}

/// **The quads of a comment are eight numbers at a time.**
#[test]
fn the_quads_of_a_comment_are_eight_numbers_at_a_time() {
    let mut session = session();
    a_page(&mut session, "quads");
    let comments = comments(&mut session);
    let marked = comments
        .iter()
        .find(|comment| comment["subtype"] == json!("Highlight"))
        .expect("the highlight is there");
    let quads = marked["quads"].as_array().expect("the quads are there");
    assert_eq!(quads.len(), 1, "there is one quad: {quads:?}");
    assert_eq!(quads[0].as_array().map(|quad| quad.len()), Some(8), "and it is eight numbers: {quads:?}");
}

/// **A reply says which comment it answers, and a locked comment says
/// it is locked.**
#[test]
fn a_reply_says_which_comment_it_answers() {
    let mut session = session();
    a_page(&mut session, "reply");
    let comments = comments(&mut session);
    let reply = comments.iter().find(|comment| comment["author"] == json!("Cy")).expect("the reply is there");
    assert_eq!(reply["in_reply_to"], json!("note-1"), "the reply says what it answers: {reply:?}");
    assert_eq!(reply["locked"], json!(true), "the reply is locked: {reply:?}");
}

/// **The list is sorted from the top of the page down.**
#[test]
fn the_list_is_sorted_from_the_top_of_the_page_down() {
    let mut session = session();
    a_page(&mut session, "sorted");
    let comments = comments(&mut session);
    let tops: Vec<f64> = comments
        .iter()
        .map(|comment| {
            comment["rect"]
                .as_array()
                .and_then(|rect| rect.get(3))
                .and_then(|top| top.as_f64())
                .unwrap_or(0.0)
        })
        .collect();
    let mut sorted = tops.clone();
    sorted.sort_by(|left, right| right.total_cmp(left));
    assert_eq!(tops, sorted, "the comments come from the top down: {tops:?}");
}

/// **A page that is not a page of the file is refused.**
#[test]
fn a_page_that_is_not_a_page_of_the_file_is_refused() {
    let mut session = session();
    a_page(&mut session, "refused");
    let refusal =
        session.execute("pdf.comment.read", &json!({ "page": 9999 })).expect_err("the page is refused");
    let message = refusal.to_string();
    assert!(message.contains("page"), "the refusal says so: {message}");
}
