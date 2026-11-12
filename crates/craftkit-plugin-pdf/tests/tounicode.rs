//! The **Unicode map** of a font.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-tounicode-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// The **CMap** a font carries: two characters, a range, and a range
/// with a list.
fn the_cmap() -> String {
    let mut cmap = String::new();
    cmap.push_str("/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n");
    cmap.push_str("1 begincodespacerange\n<00> <FF>\nendcodespacerange\n");
    cmap.push_str("2 beginbfchar\n<01> <0041>\n<02> <0042>\nendbfchar\n");
    cmap.push_str("1 beginbfrange\n<10> <12> <0058>\nendbfrange\n");
    cmap.push_str("1 beginbfrange\n<20> <22> [<0059> <005A> <005B>]\nendbfrange\n");
    cmap.push_str("endcmap\nend\nend\n");
    cmap
}

/// A file with a font that carries a **map** and a font that does not.
fn a_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    let cmap = the_cmap();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 4 0 R >> >> >>"
                .to_string(),
        ),
        (4, "<< /Type /Font /Subtype /Type1 /BaseFont /Custom /ToUnicode 5 0 R >>".to_string()),
        (5, format!("<< /Length {} >>\nstream\n{cmap}endstream", cmap.len())),
        (6, "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string()),
    ] {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    file.extend_from_slice(b"xref\n0 7\n");
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(b"trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n");
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    file
}

/// Load a file into the document, and give the **fonts** it holds.
fn load(session: &mut Session, name: &str) -> (u64, u64) {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, a_file()).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
    let fonts: Vec<u64> = session
        .document()
        .walk()
        .filter(|node| {
            node.data["entries"]
                .as_array()
                .map(|entries| {
                    entries.iter().any(|entry| {
                        let Some(pair) = entry.as_array() else { return false };
                        pair.first().and_then(|key| key.as_str()) == Some("Type")
                            && pair.get(1).and_then(|value| value.as_str()) == Some("Font")
                    })
                })
                .unwrap_or(false)
        })
        .map(|node| node.id)
        .collect();
    assert_eq!(fonts.len(), 2, "the file holds two fonts");
    (fonts[0], fonts[1])
}

/// The **map** of a font, as the command reads it.
fn map_of(session: &mut Session, font: u64) -> Json {
    let outcome = session.execute("pdf.font.unicode", &json!({ "font": font })).expect("the map is read");
    outcome.data.clone()
}

/// The map of a font is read: its characters and its ranges.
#[test]
fn the_map_of_a_font_is_read() {
    let mut session = session();
    let (font, _) = load(&mut session, "mapped");
    let map = map_of(&mut session, font);
    assert_eq!(map["has_map"], json!(true), "the font has a map");
    let entries = map["entries"].as_array().expect("the entries are a list");
    // Two characters, three codes of a range, and three codes of a
    // range with a list.
    assert_eq!(entries.len(), 8, "the map has eight entries");
    assert_eq!(entries[0]["code"], json!(1), "the first code is read");
    assert_eq!(entries[0]["text"], json!("A"), "and the character it stands for");
    assert_eq!(entries[1]["text"], json!("B"), "and the second one");
    assert_eq!(entries[2]["code"], json!(16), "the range is read");
    assert_eq!(entries[2]["text"], json!("X"), "and its text is given to every code");
    assert_eq!(entries[4]["text"], json!("X"), "and to the last of them too");
    assert_eq!(entries[5]["code"], json!(32), "the range with a list is read");
    assert_eq!(entries[5]["text"], json!("Y"), "and each code has its own text");
    assert_eq!(entries[7]["text"], json!("["), "and the last one too");
}

/// A font with **no map** has no Unicode.
#[test]
fn a_font_with_no_map_has_no_unicode() {
    let mut session = session();
    let (_, plain) = load(&mut session, "plain");
    let map = map_of(&mut session, plain);
    assert_eq!(map["has_map"], json!(false), "the font has no map");
    assert_eq!(map["count"], json!(0), "and no entries");
}
