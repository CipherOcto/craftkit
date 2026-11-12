//! The **height** of the text of a page.
//!
//! The box of a piece of text is where it is: from the descent of the
//! font to its ascent, in the page's own space.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-textheight-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file whose page has the **content** given, and a **font** given.
fn a_file(content: &str, font: &str) -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>"
                .to_string(),
        ),
        (4, format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len())),
        (5, font.to_string()),
    ] {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    file.extend_from_slice(b"xref\n0 6\n");
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(b"trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n");
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    file
}

/// Load a file into the document, and give the **page** it holds.
fn load(session: &mut Session, name: &str, content: &str, font: &str) -> u64 {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, a_file(content, font)).expect("the file is written");
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

/// The **pieces** of text the page shows.
fn pieces_of(session: &mut Session, page: u64) -> Vec<Json> {
    let outcome = session.execute("pdf.page.text", &json!({ "page": page })).expect("the text is read");
    outcome.data["pieces"].as_array().expect("the pieces are a list").clone()
}

/// The **box** of a piece: the left, the bottom, the right and the top
/// of it.
fn box_of(piece: &Json) -> Vec<f64> {
    piece["box"].as_array().expect("the box is a list").iter().filter_map(|number| number.as_f64()).collect()
}

/// The **box** of the text is the box of the font: from the descent to
/// the ascent, in the page's own space.
#[test]
fn the_box_of_the_text_is_the_box_of_the_font() {
    let mut session = session();
    let font = "<< /Type /Font /Subtype /TrueType /BaseFont /Helvetica /FirstChar 65 /Widths [500] \
                /FontDescriptor << /Ascent 800 /Descent -200 >> >>";
    let page = load(&mut session, "box", "BT /F1 12 Tf 1 0 0 1 100 200 Tm (A) Tj ET", font);
    let pieces = pieces_of(&mut session, page);
    assert_eq!(pieces.len(), 1, "the page shows one piece: {pieces:?}");
    let box_ = box_of(&pieces[0]);
    assert_eq!(box_.len(), 4, "the box has four sides: {pieces:?}");
    // The ascent is eight tenths of an em and the descent two, at
    // twelve points: the text stands from 197.6 to 209.6.
    assert!((box_[0] - 100.0).abs() < 1e-9, "the box begins where the text does: {pieces:?}");
    assert!((box_[1] - 197.6).abs() < 1e-9, "and it reaches below the line: {pieces:?}");
    assert!((box_[3] - 209.6).abs() < 1e-9, "and above it: {pieces:?}");
    assert!((box_[2] - 106.0).abs() < 1e-9, "and it is as wide as the text: {pieces:?}");
}

/// A font that says **nothing** has the tree's own metrics: an
/// ascent of `0.9` and a descent of `-0.25`.
#[test]
fn a_font_that_says_nothing_has_the_tree_s_own_metrics() {
    let mut session = session();
    let font = "<< /Type /Font /Subtype /TrueType /BaseFont /Helvetica /FirstChar 65 /Widths [500] >>";
    let page = load(&mut session, "silent", "BT /F1 12 Tf 1 0 0 1 100 200 Tm (A) Tj ET", font);
    let pieces = pieces_of(&mut session, page);
    let box_ = box_of(&pieces[0]);
    assert!((box_[1] - 197.0).abs() < 1e-9, "the descent is a quarter of an em: {pieces:?}");
    assert!((box_[3] - 210.8).abs() < 1e-9, "and the ascent nine tenths: {pieces:?}");
    // The fonts of the page say the same.
    let outcome = session.execute("pdf.page.fonts", &json!({ "page": page })).expect("the fonts are read");
    let fonts = outcome.data["fonts"].as_array().expect("the fonts are a list").clone();
    assert_eq!(fonts.len(), 1, "the page names one font: {fonts:?}");
    assert!(
        (fonts[0]["ascent"].as_f64().unwrap_or_default() - 0.9).abs() < 1e-9,
        "and the ascent: {fonts:?}"
    );
    assert!(
        (fonts[0]["descent"].as_f64().unwrap_or_default() + 0.25).abs() < 1e-9,
        "and the descent: {fonts:?}"
    );
}

/// A font of **type three** says it in its own box: the top of the
/// box is the ascent and the bottom the descent.
#[test]
fn a_font_of_type_three_says_it_in_its_own_box() {
    let mut session = session();
    let font = "<< /Type /Font /Subtype /Type3 /BaseFont /Helvetica /FirstChar 65 /Widths [500] \
                /FontBBox [0 0 1000 800] /FontMatrix [0.001 0 0 0.001 0 0] >>";
    let page = load(&mut session, "type3", "BT /F1 12 Tf 1 0 0 1 100 200 Tm (A) Tj ET", font);
    let pieces = pieces_of(&mut session, page);
    let box_ = box_of(&pieces[0]);
    // The box is a thousand tall and eight hundred up, drawn by a
    // matrix of a thousandth: the ascent is eight tenths, and the
    // descent is the tenth the tree's own rule gives it.
    assert!((box_[1] - 198.8).abs() < 1e-9, "the descent is a tenth of an em: {pieces:?}");
    assert!((box_[3] - 209.6).abs() < 1e-9, "and the ascent is eight tenths: {pieces:?}");
}

/// The place of an **array** of text is where the run begins, and not
/// where it ends.
#[test]
fn the_place_of_an_array_is_where_the_run_begins() {
    let mut session = session();
    let font = "<< /Type /Font /Subtype /TrueType /BaseFont /Helvetica /FirstChar 65 /Widths [500] \
                /FontDescriptor << /Ascent 800 /Descent -200 >> >>";
    let page = load(&mut session, "array", "BT /F1 12 Tf 1 0 0 1 100 200 Tm [(A) -100 (A)] TJ ET", font);
    let pieces = pieces_of(&mut session, page);
    assert_eq!(pieces.len(), 1, "the page shows one piece: {pieces:?}");
    assert_eq!(pieces[0]["x"], json!(100.0), "the run begins where the matrix says: {pieces:?}");
    let box_ = box_of(&pieces[0]);
    assert!((box_[0] - 100.0).abs() < 1e-9, "and the box begins there too: {pieces:?}");
}
