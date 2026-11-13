//! The **frame** of a field: the background and the border of it, as
//! the look of the field says.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-lookframe-{name}-{}", std::process::id()))
        .expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file with a **text field** on a page, whose look is the given one.
fn a_file_with(mark: &str, line: &str) -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (
            1,
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] /DR << /Font << /Helv 6 0 R >> >> >> >>"
                .to_string(),
        ),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] \
             /Resources << /Font << /Helv 6 0 R >> >> /Annots [4 0 R] >>"
                .to_string(),
        ),
        (
            4,
            format!(
                "<< /Type /Annot /Subtype /Widget /FT /Tx /T (the name) /Rect [10 700 200 720] \
             /DA (/Helv 12 Tf 0 g) /V (Ada) /P 3 0 R {mark} {line} >>"
            ),
        ),
        (5, "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string()),
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

/// Load the file and give the **widget** it holds.
fn load(session: &mut Session, name: &str, mark: &str, line: &str) -> (u64, String) {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, a_file_with(mark, line)).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
    let widget = session
        .document()
        .walk()
        .find(|node| {
            node.data["entries"]
                .as_array()
                .map(|entries| {
                    entries.iter().any(|entry| {
                        let Some(pair) = entry.as_array() else { return false };
                        pair.first().and_then(|key| key.as_str()) == Some("Subtype")
                            && pair.get(1).and_then(|value| value.as_str()) == Some("Widget")
                    })
                })
                .unwrap_or(false)
        })
        .map(|node| node.id)
        .expect("the widget is there");
    (widget, directory)
}

/// The **drawing** of the widget.
fn the_drawing(session: &mut Session, name: &str, mark: &str, line: &str) -> String {
    let (widget, directory) = load(session, name, mark, line);
    let to = format!("{directory}/drawing.txt");
    session
        .execute("pdf.field.appearance.write", &json!({ "widget": widget, "to": to }))
        .expect("the appearance is written");
    std::fs::read_to_string(format!("{directory}/drawing.txt")).expect("the drawing is there")
}

/// **The background** is filled from the `/BG` of the look, and the
/// **border** is stroked from the `/BC` of it: the tree's own frame.
#[test]
fn the_background_and_the_border_are_drawn() {
    let mut session = session();
    let drawing =
        the_drawing(&mut session, "frame", "/MK << /BC [1 0 0] /BG [0.9 0.9 0.9] >>", "/BS << /W 2 /S /S >>");
    assert!(drawing.contains("0.9 0.9 0.9 rg\n0 0 190 20 re f\n"), "the background is filled: {drawing}");
    assert!(drawing.contains("1 0 0 RG\n2 w\n1 1 188 18 re S\n"), "the border is stroked: {drawing}");
}

/// **A field that says no colour** has no background and no border:
/// the tree's own rule.
#[test]
fn a_field_with_no_colour_has_no_frame() {
    let mut session = session();
    let drawing = the_drawing(&mut session, "noframe", "", "");
    assert!(!drawing.contains(" re f\n"), "there is no background: {drawing}");
    assert!(!drawing.contains(" re S\n"), "and no border: {drawing}");
    // But the text is still there.
    assert!(drawing.contains("(Ada) Tj"), "the value is drawn: {drawing}");
}

/// **A border of no width** is not stroked: the tree's own rule.
#[test]
fn a_border_of_no_width_is_not_stroked() {
    let mut session = session();
    let drawing = the_drawing(
        &mut session,
        "nowidth",
        "/MK << /BC [1 0 0] /BG [0.9 0.9 0.9] >>",
        "/BS << /W 0 /S /S >>",
    );
    assert!(!drawing.contains(" re S\n"), "there is no border: {drawing}");
    assert!(drawing.contains("0.9 0.9 0.9 rg\n"), "but the background is filled: {drawing}");
}

/// **A dashed border** is dashed: the tree's own rule.
#[test]
fn a_dashed_border_is_dashed() {
    let mut session = session();
    let drawing = the_drawing(&mut session, "dashed", "/MK << /BC [0 0 1] >>", "/BS << /W 1 /S /D >>");
    assert!(drawing.contains("[3] 0 d\n"), "the line is dashed: {drawing}");
    assert!(
        drawing.contains("0 0 1 RG\n1 w\n[3] 0 d\n0.5 0.5 189 19 re S\n"),
        "and the border is stroked: {drawing}"
    );
}

/// **An underline** is the bottom edge of the field and nothing else:
/// the tree's own rule.
#[test]
fn an_underline_is_the_bottom_edge() {
    let mut session = session();
    let drawing = the_drawing(&mut session, "underline", "/MK << /BC [0 0 1] >>", "/BS << /W 1 /S /U >>");
    assert!(
        drawing.contains("0 0 1 RG\n1 w\n0 0.5 m 190 0.5 l S\n"),
        "the bottom edge is stroked: {drawing}"
    );
    assert!(!drawing.contains(" re S\n"), "and there is no box: {drawing}");
}

/// **A grey** is drawn with the operator of a grey: the tree's own
/// rule.
#[test]
fn a_grey_is_drawn_as_a_grey() {
    let mut session = session();
    let drawing = the_drawing(&mut session, "grey", "/MK << /BC [0.5] /BG [0.25] >>", "/BS << /W 1 /S /S >>");
    assert!(drawing.contains("0.25 g\n0 0 190 20 re f\n"), "the background is a grey: {drawing}");
    assert!(drawing.contains("0.5 G\n1 w\n"), "and the border is a grey: {drawing}");
}
