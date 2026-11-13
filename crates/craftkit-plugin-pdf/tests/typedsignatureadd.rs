//! The **typed signature** on a page: the tree's own signature of
//! filled outlines.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-typedsignatureadd-{name}-{}", std::process::id()))
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

/// The **contours** of a signature: a square and a smaller square
/// inside it.
fn the_contours() -> Vec<Vec<Vec<f64>>> {
    vec![
        vec![vec![0.0, 0.0], vec![1.0, 0.0], vec![1.0, 1.0], vec![0.0, 1.0]],
        vec![vec![0.25, 0.25], vec![0.75, 0.25], vec![0.75, 0.75], vec![0.25, 0.75]],
    ]
}

/// **A typed signature is put on a page:** the name of it and its
/// outlines.
#[test]
fn a_typed_signature_is_put_on_the_page() {
    let mut session = session();
    let directory = directory("signature");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.annot.typedsignature.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 300.0, 740.0],
                "contours": the_contours(),
            }),
        )
        .expect("the signature is added");
    assert!(outcome.changed);
    let annot = outcome.data["id"].as_u64().expect("the signature is there");
    let node = session.document().node(annot).cloned().expect("the signature is there");
    let entries = entries_of(&node);
    // The **subtype** of the tree's typed signature is a stamp.
    assert_eq!(entry(&entries, "Subtype"), Some(&json!("Stamp")), "the subtype is the tree's");
    // The **name** of it is the tree's own.
    assert_eq!(entry(&entries, "Name"), Some(&json!("PCTypedSignature")), "the name is the tree's");
    // The **outlines** are the contours, and each one is a flat list of
    // numbers.
    assert_eq!(
        entry(&entries, "PCOutline"),
        Some(&json!([
            [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0],
            [0.25, 0.25, 0.75, 0.25, 0.75, 0.75, 0.25, 0.75]
        ])),
        "the outlines are the tree's"
    );
    // And the **colour** is the tree's own black.
    assert_eq!(entry(&entries, "C"), Some(&json!([0.0, 0.0, 0.0])), "the colour is the tree's");
    assert_eq!(outcome.data["contours"], json!(2), "both contours are kept");
}

/// **A contour of fewer than three points** is dropped: the tree's own
/// rule.
#[test]
fn a_short_contour_is_dropped() {
    let mut session = session();
    let directory = directory("short");
    let page = a_page(&mut session, &directory);
    let outcome = session
        .execute(
            "pdf.annot.typedsignature.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 300.0, 740.0],
                "contours": [[[0.0, 0.0], [1.0, 1.0]], [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]],
            }),
        )
        .expect("the signature is added");
    assert_eq!(outcome.data["contours"], json!(1), "the short contour is dropped");
    let annot = outcome.data["id"].as_u64().expect("the signature is there");
    let node = session.document().node(annot).cloned().expect("the signature is there");
    let entries = entries_of(&node);
    assert_eq!(
        entry(&entries, "PCOutline"),
        Some(&json!([[0.0, 0.0, 1.0, 0.0, 1.0, 1.0]])),
        "only the contour of three points is kept"
    );
}

/// **A point that is not between zero and one** is refused: the
/// tree's own normalisation.
#[test]
fn a_point_outside_the_normalisation_is_refused() {
    let mut session = session();
    let directory = directory("outside");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute(
            "pdf.annot.typedsignature.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 300.0, 740.0],
                "contours": [[[0.0, 0.0], [1.5, 0.0], [1.0, 1.0]]],
            }),
        )
        .expect_err("the signature is refused");
    assert!(error.to_string().contains("between zero and one"), "the error says why: {error}");
}

/// **A rectangle that is too small** is refused: the tree's own rule.
#[test]
fn a_rectangle_that_is_too_small_is_refused() {
    let mut session = session();
    let directory = directory("small");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute(
            "pdf.annot.typedsignature.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 100.5, 700.0],
                "contours": the_contours(),
            }),
        )
        .expect_err("the signature is refused");
    assert!(error.to_string().contains("a point wide"), "the error says why: {error}");
}

/// **A colour that is not three numbers** is refused.
#[test]
fn a_colour_that_is_not_three_numbers_is_refused() {
    let mut session = session();
    let directory = directory("colour");
    let page = a_page(&mut session, &directory);
    let error = session
        .execute(
            "pdf.annot.typedsignature.add",
            &json!({
                "page": page,
                "object": 10,
                "rect": [100.0, 700.0, 300.0, 740.0],
                "contours": the_contours(),
                "colour": [0.0, 0.0],
            }),
        )
        .expect_err("the signature is refused");
    assert!(error.to_string().contains("three numbers"), "the error says why: {error}");
}
