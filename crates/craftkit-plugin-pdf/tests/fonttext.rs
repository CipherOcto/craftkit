//! The **text** of a page read through the **map** of its font.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-fonttext-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// The **CMap** the font carries: two codes and the characters they
/// stand for.
fn the_cmap() -> String {
    let mut cmap = String::new();
    cmap.push_str("/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n");
    cmap.push_str("1 begincodespacerange\n<00> <FF>\nendcodespacerange\n");
    cmap.push_str("2 beginbfchar\n<01> <0058>\n<02> <0059>\nendbfchar\n");
    cmap.push_str("endcmap\nend\nend\n");
    cmap
}

/// A file with a font that carries a **map** and a font that does not.
fn a_file(content: &str) -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    let cmap = the_cmap();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R \
             /Resources << /Font << /F1 5 0 R /F2 7 0 R >> >> >>"
                .to_string(),
        ),
        (4, format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len())),
        (5, "<< /Type /Font /Subtype /Type1 /BaseFont /Mapped /ToUnicode 6 0 R >>".to_string()),
        (6, format!("<< /Length {} >>\nstream\n{cmap}endstream", cmap.len())),
        (7, "<< /Type /Font /Subtype /Type1 /BaseFont /Plain >>".to_string()),
    ] {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    file.extend_from_slice(b"xref\n0 8\n");
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(b"trailer\n<< /Size 8 /Root 1 0 R >>\nstartxref\n");
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

/// The **fonts** of the page, as the command reads them.
fn fonts_of(session: &mut Session, page: u64) -> Json {
    let outcome = session.execute("pdf.page.fonts", &json!({ "page": page })).expect("the fonts are read");
    outcome.data.clone()
}

/// The text of a page is read **through the map** of the font it is
/// shown in.
#[test]
fn the_text_of_a_page_is_read_through_the_map_of_its_font() {
    let mut session = session();
    let page = load(&mut session, "mapped", "BT /F1 12 Tf (\\001\\002) Tj ET");
    let text = text_of(&mut session, page);
    let pieces = text["pieces"].as_array().expect("the pieces are a list");
    assert_eq!(pieces.len(), 1, "the page shows one piece");
    assert_eq!(pieces[0]["text"], json!("XY"), "the codes are read through the map");
    assert_eq!(pieces[0]["encoding"], json!("unicode"), "and the answer says so");
    assert_eq!(pieces[0]["font"], json!("F1"), "and the piece says which font it is shown in");
    assert_eq!(text["text"], json!("XY"), "and the text of the page is the characters");
}

/// A font with **no map** leaves the text as the bytes it is.
#[test]
fn a_font_with_no_map_leaves_the_text_as_the_bytes_it_is() {
    let mut session = session();
    let page = load(&mut session, "plain", "BT /F2 12 Tf (\\001\\002) Tj ET");
    let text = text_of(&mut session, page);
    let pieces = text["pieces"].as_array().expect("the pieces are a list");
    assert_eq!(pieces.len(), 1, "the page shows one piece");
    assert_eq!(pieces[0]["encoding"], json!("latin-1"), "the bytes are read as they are");
    assert_eq!(pieces[0]["text"], json!("\u{1}\u{2}"), "and the codes stand for themselves");
    assert_eq!(pieces[0]["font"], json!("F2"), "and the piece still says which font it is in");
}

/// A code the map does **not know** is read as the byte it is.
#[test]
fn a_code_the_map_does_not_know_is_read_as_the_byte_it_is() {
    let mut session = session();
    let page = load(&mut session, "unknown", "BT /F1 12 Tf (\\001\\002\\003) Tj ET");
    let text = text_of(&mut session, page);
    let pieces = text["pieces"].as_array().expect("the pieces are a list");
    assert_eq!(pieces[0]["text"], json!("XY\u{3}"), "the two known codes are read and the third is not");
    assert_eq!(pieces[0]["encoding"], json!("unicode"), "and the piece was read through the map");
}

/// The **fonts** of a page are read: the names it gives them and the
/// maps they carry.
#[test]
fn the_fonts_of_a_page_are_read() {
    let mut session = session();
    let page = load(&mut session, "fonts", "BT /F1 12 Tf (\\001) Tj ET");
    let fonts = fonts_of(&mut session, page);
    let listed = fonts["fonts"].as_array().expect("the fonts are a list");
    assert_eq!(listed.len(), 2, "the page names two fonts");
    assert_eq!(listed[0]["name"], json!("F1"), "the first is named");
    assert_eq!(listed[0]["base"], json!("Mapped"), "and the font behind it is named");
    assert_eq!(listed[0]["has_map"], json!(true), "and it carries a map");
    assert_eq!(listed[0]["codes"], json!(2), "with two codes in it");
    assert_eq!(listed[1]["name"], json!("F2"), "the second is named");
    assert_eq!(listed[1]["has_map"], json!(false), "and it carries no map");
    assert_eq!(listed[1]["codes"], json!(0), "and has no code in it");
}

/// The text under a **marked content** is read through the map of the
/// font too.
#[test]
fn the_text_under_a_marked_content_is_read_through_the_map() {
    let mut session = session();
    let content = "/Span /MC0 BDC\nBT /F1 12 Tf (\\001\\002) Tj ET\nEMC\n";
    let directory = directory("marked");
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    let cmap = the_cmap();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 8 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R \
             /Resources << /Font << /F1 5 0 R >> /Properties << /MC0 << /MCID 0 >> >> >> \
             /StructParents 0 >>"
                .to_string(),
        ),
        (4, format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len())),
        (5, "<< /Type /Font /Subtype /Type1 /BaseFont /Mapped /ToUnicode 6 0 R >>".to_string()),
        (6, format!("<< /Length {} >>\nstream\n{cmap}endstream", cmap.len())),
        (7, "<< /Type /StructTreeRoot /K 9 0 R >>".to_string()),
        (8, "<< /S /Document /Lang (en-GB) /Pg 3 0 R /K 9 0 R >>".to_string()),
        (9, "<< /S /H1 /T (the heading) /Pg 3 0 R /K 0 >>".to_string()),
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
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, file).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
    let outcome = session.execute("pdf.marked.read", &json!({})).expect("the content is read");
    let marked = outcome.data["marked"].as_array().expect("the elements are a list");
    let heading = marked.iter().find(|element| element["kind"] == json!("H1")).expect("the heading is there");
    assert_eq!(heading["text"], json!("XY"), "the text under it is read through the map");
}
