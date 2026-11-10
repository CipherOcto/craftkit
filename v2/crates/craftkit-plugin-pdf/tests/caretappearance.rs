//! The **appearance** of the caret: the wedge of the inserted text.

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
    let path =
        std::env::temp_dir().join(format!("craftkit-v2-caretappearance-{name}-{}", std::process::id()));
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

/// A **caret** on a page: the node of it.
fn a_caret(session: &mut Session, directory: &str) -> u64 {
    let page = a_page(session, directory);
    let outcome = session
        .execute(
            "pdf.annot.caret.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 110.0, 720.0],
            }),
        )
        .expect("the caret is added");
    outcome.data["id"].as_u64().expect("the caret is there")
}

/// **The caret is drawn:** a filled wedge, two curved flanks that meet
/// at the top centre of the rectangle.
#[test]
fn the_caret_is_drawn() {
    let mut session = session();
    let directory = directory("caret");
    let caret = a_caret(&mut session, &directory);
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": caret, "to": to }))
        .expect("the caret is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    // The **tree's own** blue.
    assert!(bytes.contains("0 0.47 0.84 rg"), "the caret has its colour: {bytes}");
    // The wedge: the rectangle is ten wide and twenty high, so the
    // flanks are a tenth of the width from the sides and a fifth of
    // the height from the bottom.
    assert!(
        bytes.contains("100 700 m\n104 704 105 715 105 720 c\n105 715 106 704 110 700 c\nh f\n"),
        "the caret is drawn: {bytes}"
    );
}

/// **A caret without a colour** is the tree's own blue.
#[test]
fn a_caret_without_a_colour_is_the_trees_blue() {
    let mut session = session();
    let directory = directory("nocolour");
    let caret = a_caret(&mut session, &directory);
    session
        .execute("pdf.entry.remove", &json!({ "object": caret, "key": "C" }))
        .expect("the colour is taken away");
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": caret, "to": to }))
        .expect("the caret is drawn");
    let bytes = std::fs::read_to_string(&to).expect("the drawing is there");
    assert!(bytes.contains("0 0.47 0.84 rg"), "the caret is the tree's blue: {bytes}");
}

/// **A caret without its rectangle** is refused.
#[test]
fn a_caret_without_its_rectangle_is_refused() {
    let mut session = session();
    let directory = directory("norect");
    let caret = a_caret(&mut session, &directory);
    session
        .execute("pdf.entry.remove", &json!({ "object": caret, "key": "Rect" }))
        .expect("the rectangle is taken away");
    let to = format!("{directory}/drawing.txt");
    let error = session
        .execute("pdf.annot.appearance.write", &json!({ "annotation": caret, "to": to }))
        .expect_err("the caret is refused");
    assert!(error.to_string().contains("cannot be drawn"), "the error says why: {error}");
}
