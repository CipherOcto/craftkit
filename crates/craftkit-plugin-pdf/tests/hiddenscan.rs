//! The **hidden information** of a file.
//!
//! The nine categories the tree counts are counted here.

use craftkit_sdk::serde_json::json;
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
    let path = std::env::temp_dir().join(format!("craftkit-v2-hiddenscan-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file that hides a little of everything.
fn a_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (
            1,
            "<< /Type /Catalog /Pages 2 0 R /Info 7 0 R /OpenAction 8 0 R /Outlines 9 0 R \
             /Names << /JavaScript << /Names [(Doc) 10 0 R] >> /EmbeddedFiles << /Names [(data.txt) 11 0 R] >> >> \
             /OCProperties << /OCGs [12 0 R] /D << /OFF [12 0 R] >> >> >>"
                .to_string(),
        ),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Annots [4 0 R 5 0 R 6 0 R] >>".to_string(),
        ),
        (4, "<< /Type /Annot /Subtype /Link /Rect [10 10 20 20] >>".to_string()),
        (5, "<< /Type /Annot /Subtype /Text /Rect [30 30 40 40] /Contents (a note) >>".to_string()),
        (
            6,
            "<< /Type /Annot /Subtype /FileAttachment /Rect [50 50 60 60] >>".to_string(),
        ),
        (7, "<< /Title (Hello) /Author (Someone) >>".to_string()),
        (8, "<< /Type /Action /S /GoTo >>".to_string()),
        (9, "<< /Type /Outlines /First 13 0 R /Last 13 0 R /Count 1 >>".to_string()),
        (10, "<< /Length 12 >>\nstream\napp.alert(1)\nendstream".to_string()),
        (11, "<< /Length 5 /Type /EmbeddedFile >>\nstream\nhello\nendstream".to_string()),
        (12, "<< /Type /OCG /Name (The layer) >>".to_string()),
        (13, "<< /Title (Chapter one) /Parent 9 0 R >>".to_string()),
    ] {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    file.extend_from_slice(b"xref\n0 14\n");
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(b"trailer\n<< /Size 14 /Root 1 0 R /Info 7 0 R >>\nstartxref\n");
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    file
}

/// Load a file into the document.
fn load(session: &mut Session, name: &str) {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, a_file()).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
}

/// The **counts** of the hidden information.
fn counts_of(session: &mut Session) -> Vec<(String, u64)> {
    let outcome = session.execute("pdf.hidden.scan", &json!({})).expect("the file is scanned");
    outcome.data["categories"]
        .as_array()
        .expect("the categories are a list")
        .iter()
        .map(|category| {
            (
                category["category"].as_str().unwrap_or_default().to_string(),
                category["count"].as_u64().unwrap_or_default(),
            )
        })
        .collect()
}

/// The **nine categories** are named, in the tree's own order.
#[test]
fn the_nine_categories_are_named_in_the_trees_own_order() {
    let mut session = session();
    load(&mut session, "names");
    let counts = counts_of(&mut session);
    let names: Vec<String> = counts.iter().map(|(name, _)| name.clone()).collect();
    assert_eq!(
        names,
        vec![
            "Metadata",
            "Attachments",
            "Comments",
            "FormFields",
            "HiddenText",
            "HiddenLayers",
            "Bookmarks",
            "LinksActionsScripts",
            "PrivateData",
        ],
        "the categories are the tree's own: {names:?}"
    );
}

/// What the file hides is **counted**: the information, the
/// attachments, the comments, the layer that is off, the bookmark, the
/// link, the action and the script.
#[test]
fn what_the_file_hides_is_counted() {
    let mut session = session();
    load(&mut session, "counted");
    let counts = counts_of(&mut session);
    let find = |name: &str| {
        counts.iter().find(|(category, _)| category == name).map(|(_, count)| *count).unwrap_or_default()
    };
    assert_eq!(find("Metadata"), 2, "the two entries of the information: {counts:?}");
    assert_eq!(find("Attachments"), 2, "the file inside and the attachment: {counts:?}");
    assert_eq!(find("Comments"), 1, "the note on the page: {counts:?}");
    assert_eq!(find("FormFields"), 0, "the file has no form: {counts:?}");
    assert_eq!(find("HiddenText"), 0, "the file hides no text: {counts:?}");
    assert_eq!(find("HiddenLayers"), 1, "the layer that is off: {counts:?}");
    assert_eq!(find("Bookmarks"), 1, "the one bookmark: {counts:?}");
    assert_eq!(find("LinksActionsScripts"), 3, "the link, the action and the script: {counts:?}");
    assert_eq!(find("PrivateData"), 0, "the file holds no private data: {counts:?}");
}

/// **One category** may be asked for.
#[test]
fn one_category_may_be_asked_for() {
    let mut session = session();
    load(&mut session, "one");
    let outcome =
        session.execute("pdf.hidden.scan", &json!({ "category": "Comments" })).expect("the category is read");
    let categories = outcome.data["categories"].as_array().expect("the categories are a list");
    assert_eq!(categories.len(), 1, "one category is answered: {categories:?}");
    assert_eq!(categories[0]["category"], json!("Comments"), "and it is the one asked for: {categories:?}");
    assert_eq!(categories[0]["count"], json!(1), "and its count: {categories:?}");
}

/// A category that is **not one of the tree's** is refused.
#[test]
fn a_category_that_is_not_one_of_the_trees_is_refused() {
    let mut session = session();
    load(&mut session, "refused");
    let error = session
        .execute("pdf.hidden.scan", &json!({ "category": "Nonsense" }))
        .expect_err("there is no such category");
    assert!(error.to_string().contains("Nonsense"), "the refusal says why: {error}");
}
