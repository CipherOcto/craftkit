//! The **images of a form**: the pictures a form draws.
//!
//! A form that the region hits is written again as a new object, and
//! the images it draws are treated as the images of the page are: an
//! image the region only **hits** has its covered pixels cleared, and
//! an image the region **covers** is taken away.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-formimages-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file whose page draws a **form**, and the form draws an **image**.
///
/// The image is four by four grey pixels, and every one of them is
/// white.
fn a_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    let content = "q 100 0 0 100 0 0 cm /Fm1 Do Q";
    let form = "q 1 0 0 1 0 0 cm /Im1 Do Q";
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R \
             /Resources << /XObject << /Fm1 6 0 R >> >> >>"
                .to_string(),
        ),
        (4, format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len())),
    ] {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    // The **image**: four by four grey pixels, and every one of them is
    // white.
    offsets.push(file.len());
    file.extend_from_slice(
        b"5 0 obj\n<< /Type /XObject /Subtype /Image /Width 4 /Height 4 \
          /ColorSpace /DeviceGray /BitsPerComponent 8 /Length 16 >>\nstream\n",
    );
    file.extend_from_slice(&[0xff; 16]);
    file.extend_from_slice(b"\nendstream\nendobj\n");
    // The **form**: it draws the image, and it has resources of its
    // own.
    offsets.push(file.len());
    file.extend_from_slice(
        format!(
            "6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 1 1] \
             /Resources << /XObject << /Im1 5 0 R >> >> /Length {} >>\nstream\n{form}\nendstream\nendobj\n",
            form.len()
        )
        .as_bytes(),
    );
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

/// Load a file into the document, and give the **page** it holds.
fn load(session: &mut Session, name: &str) -> u64 {
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

/// The **streams** of the document, and the bytes of each of them.
fn streams_of(session: &Session) -> Vec<Vec<u8>> {
    let mut out: Vec<Vec<u8>> = Vec::new();
    for node in session.document().walk() {
        if node.kind != "pdf.stream" {
            continue;
        }
        let Some(path) = node.stream.clone() else { continue };
        out.push(std::fs::read(&path).unwrap_or_default());
    }
    out
}

/// The **new form**: the stream of the form that was written again.
///
/// The form draws in its own space, which the page scales by a
/// hundred: the form's own transform is the one that is not the
/// page's.
fn the_new_form(streams: &[Vec<u8>]) -> String {
    streams
        .iter()
        .filter_map(|bytes| String::from_utf8(bytes.clone()).ok())
        .find(|text| text.contains("1 0 0 1 0 0 cm") && !text.contains("/Im1"))
        .unwrap_or_else(|| panic!("there is a new form among {streams:?}"))
}

/// The **XObjects** of the new form: the names it draws.
fn the_new_form_xobjects(session: &Session) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for node in session.document().walk() {
        let Some(number) = node.data.get("object").and_then(|number| number.as_u64()) else { continue };
        if number <= 7 {
            continue;
        }
        let Some(entries) = node.data.get("entries").and_then(|entries| entries.as_array()) else { continue };
        let Some(resources) = entries.iter().find_map(|entry| {
            let pair = entry.as_array()?;
            if pair.first().and_then(|key| key.as_str()) == Some("Resources") { pair.get(1) } else { None }
        }) else {
            continue;
        };
        let Some(xobjects) = resources.get("entries").and_then(|entries| entries.as_array()) else {
            continue;
        };
        for entry in xobjects {
            let Some(pair) = entry.as_array() else { continue };
            if pair.first().and_then(|key| key.as_str()) == Some("XObject")
                && let Some(list) =
                    pair.get(1).and_then(|value| value.get("entries")).and_then(|entries| entries.as_array())
            {
                for item in list {
                    if let Some(pair) = item.as_array()
                        && let Some(name) = pair.first().and_then(|name| name.as_str())
                    {
                        out.push(name.to_string());
                    }
                }
            }
        }
    }
    out
}

/// The image of a form the region only **hits** has its pixels
/// cleared, and it is drawn under a new name.
#[test]
fn the_image_of_a_form_has_its_pixels_cleared() {
    let mut session = session();
    let page = load(&mut session, "hits");
    let outcome = session
        .execute("pdf.redact.drawings", &json!({ "page": page, "rect": [0, 0, 50, 50] }))
        .expect("the image is cleared");
    assert_eq!(outcome.data["cleared"], json!(1), "one image is cleared: {outcome:?}");
    assert_eq!(outcome.data["forms"], json!(1), "and the form is written again: {outcome:?}");
    let streams = streams_of(&session);
    // The **new form** draws the image under a **new name**.
    let form = the_new_form(&streams);
    assert!(form.contains("PCRedacted1"), "the image is drawn under a new name: {form}");
    assert!(!form.contains("/Im1"), "and not under the old one: {form}");
    // The **pixels** of the new image: the four under the region are
    // black, and the other twelve are white.
    let cleared = streams
        .iter()
        .find(|bytes| bytes.len() == 16 && bytes.contains(&0))
        .cloned()
        .expect("the new image is there");
    let mut expected = vec![0xff; 16];
    for at in [8usize, 9, 12, 13] {
        expected[at] = 0;
    }
    assert_eq!(cleared, expected, "and the covered pixels are black: {cleared:?}");
    // The **resources** of the new form: the image is drawn under the
    // new name, and the old name is not among them.
    let xobjects = the_new_form_xobjects(&session);
    assert!(xobjects.contains(&"PCRedacted1".to_string()), "the new name is there: {xobjects:?}");
    assert!(!xobjects.contains(&"Im1".to_string()), "and the old one is not: {xobjects:?}");
    // The **object** the file had is untouched: the form that shared
    // it still draws it.
    let original = streams
        .iter()
        .find(|bytes| bytes.len() == 16 && bytes.iter().all(|byte| *byte == 0xff))
        .cloned()
        .expect("the original image is there");
    assert_eq!(original, vec![0xff; 16], "and it is as it was: {original:?}");
}

/// The image of a form the region **covers** is taken away.
#[test]
fn the_image_of_a_form_the_region_covers_is_taken_away() {
    let mut session = session();
    let page = load(&mut session, "covered");
    let outcome = session
        .execute("pdf.redact.drawings", &json!({ "page": page, "rect": [0, 0, 100, 100] }))
        .expect("the image is taken away");
    assert_eq!(outcome.data["forms"], json!(1), "the form is written again: {outcome:?}");
    assert_eq!(outcome.data["cleared"], json!(0), "and nothing is cleared: {outcome:?}");
    let streams = streams_of(&session);
    let form = the_new_form(&streams);
    assert!(!form.contains("Do"), "and the image is not drawn: {form}");
    // The **resources** of the new form: the image that is taken away
    // is not among them.
    let xobjects = the_new_form_xobjects(&session);
    assert!(!xobjects.contains(&"Im1".to_string()), "and it is not among the resources: {xobjects:?}");
}

/// A region that does not touch the image of a form leaves it alone.
#[test]
fn a_region_that_does_not_touch_the_image_leaves_it_alone() {
    let mut session = session();
    let page = load(&mut session, "apart");
    let outcome = session
        .execute("pdf.redact.drawings", &json!({ "page": page, "rect": [300, 300, 320, 320] }))
        .expect("the region is read");
    assert_eq!(outcome.data["cleared"], json!(0), "nothing is cleared: {outcome:?}");
    assert_eq!(outcome.data["forms"], json!(0), "and the form is left as it was: {outcome:?}");
}
