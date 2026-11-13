//! The **figures** of a file: the pictures in it, and what is said of
//! them.

use craftkit_sdk::Session;
use craftkit_sdk::serde_json::{Value as Json, json};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_pdf::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the PDF battery.
fn ecosystem() -> craftkit_sdk::Ecosystem {
    craftkit_sdk::Ecosystem::from_inventory().expect("the ecosystem is built")
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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-figures-{name}-{}", std::process::id()))
        .expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **tagged** file: a document with a heading and two pictures in it.
///
/// The second picture is a `/Illustration`, and the **role map** of the
/// tree says that an illustration is a figure.
fn a_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 5 0 R >>"),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] >>"),
        (4, "<< /Type /Annot /Subtype /Widget /Rect [0 0 10 10] /P 3 0 R >>"),
        (5, "<< /Type /StructTreeRoot /RoleMap << /Illustration /Figure >> /K 6 0 R >>"),
        (6, "<< /S /Document /Lang (en-GB) /K [7 0 R 8 0 R 9 0 R] >>"),
        (7, "<< /S /H1 /T (the heading) /P 6 0 R >>"),
        (8, "<< /S /Figure /Alt (a picture of a cat) /Pg 3 0 R /P 6 0 R >>"),
        (9, "<< /S /Illustration /Pg 3 0 R /P 6 0 R >>"),
    ] {
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

/// A file with **no structure tree** at all.
fn an_untagged_file() -> Vec<u8> {
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
    file
}

/// Load a file into the document.
fn load(session: &mut Session, name: &str, bytes: &[u8]) {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, bytes).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
}

/// The **figures** of the file, as the command reads them.
fn figures_of(session: &mut Session) -> Json {
    let outcome = session.execute("pdf.figure.read", &json!({})).expect("the figures are read");
    outcome.data.clone()
}

/// The **figures** of a tagged file are read.
#[test]
fn the_figures_of_a_tagged_file_are_read() {
    let mut session = session();
    load(&mut session, "tagged", &a_file());
    let figures = figures_of(&mut session);
    assert_eq!(figures["tagged"], json!(true), "the file is tagged");
    let figures = figures["figures"].as_array().expect("the figures are a list");
    assert_eq!(figures.len(), 2, "there are two figures: {figures:?}");
    assert_eq!(figures[0]["alt"], json!("a picture of a cat"), "the first has its text");
    assert_eq!(figures[0]["kind"], json!("Figure"), "and it is a figure");
    assert_eq!(figures[1]["alt"], json!(null), "the second has none");
    assert_eq!(figures[1]["kind"], json!("Illustration"), "and it is named as it is");
    assert!(figures[0]["page"].is_number(), "and it says which page it is on");
}

/// **The role map says what an illustration is.**
#[test]
fn the_role_map_says_what_an_illustration_is() {
    let mut session = session();
    load(&mut session, "roles", &a_file());
    let figures = figures_of(&mut session);
    let figures = figures["figures"].as_array().expect("the figures are a list");
    assert_eq!(figures.len(), 2, "the illustration is a figure too: {figures:?}");
}

/// **A file with no structure tree has no figures.**
#[test]
fn a_file_with_no_structure_tree_has_no_figures() {
    let mut session = session();
    load(&mut session, "untagged", &an_untagged_file());
    let figures = figures_of(&mut session);
    assert_eq!(figures["tagged"], json!(false), "the file is not tagged");
    assert_eq!(figures["figures"], json!([]), "and it has no figures");
}

/// **The alternative text of a figure is set.**
#[test]
fn the_alternative_text_of_a_figure_is_set() {
    let mut session = session();
    load(&mut session, "set", &a_file());
    let figures = figures_of(&mut session);
    let figure = figures["figures"][1]["id"].as_u64().expect("the figure has an id");
    session
        .execute("pdf.figure.alt.set", &json!({ "figure": figure, "alt": "a picture of a dog" }))
        .expect("the text is set");
    let figures = figures_of(&mut session);
    assert_eq!(figures["figures"][1]["alt"], json!("a picture of a dog"), "the text is said");
}

/// **An empty text takes the text away.**
#[test]
fn an_empty_text_takes_the_text_away() {
    let mut session = session();
    load(&mut session, "empty", &a_file());
    let figures = figures_of(&mut session);
    let figure = figures["figures"][0]["id"].as_u64().expect("the figure has an id");
    session
        .execute("pdf.figure.alt.set", &json!({ "figure": figure, "alt": "  " }))
        .expect("the text is taken away");
    let figures = figures_of(&mut session);
    assert_eq!(figures["figures"][0]["alt"], json!(null), "and the figure has none: {figures:?}");
}

/// **A node that is not a figure is refused.**
#[test]
fn a_node_that_is_not_a_figure_is_refused() {
    let mut session = session();
    load(&mut session, "refused", &a_file());
    // The **catalog** is not a figure.
    let catalog = session
        .document()
        .walk()
        .find(|node| {
            node.kind == "pdf.object" && node.data.get("object").and_then(|object| object.as_u64()) == Some(1)
        })
        .map(|node| node.id)
        .expect("the catalog is there");
    let refusal = session
        .execute("pdf.figure.alt.set", &json!({ "figure": catalog, "alt": "a picture" }))
        .expect_err("the node is refused");
    let message = refusal.to_string();
    assert!(message.contains("is not a figure"), "the refusal says so: {message}");
}
