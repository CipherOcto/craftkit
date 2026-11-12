//! The **free numbers** of a file.
//!
//! An object number that is not in use is **free**. A number that is
//! taken away is freed and is not used again, and the file says so: the
//! cross-reference table has a row for it.

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

/// A directory of this test's own, so that the files do not mix.
fn directory(name: &str) -> String {
    let path = std::env::temp_dir().join(format!("craftkit-v2-free-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **file** whose table says the number two is free.
fn a_file_with_a_free_number() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 3 0 R >>"),
        (3, "<< /Type /Pages /Kids [4 0 R] /Count 1 >>"),
        (4, "<< /Type /Page /Parent 3 0 R /MediaBox [0 0 595 842] >>"),
    ] {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    file.extend_from_slice(b"xref\n0 5\n");
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    file.extend_from_slice(format!("{:010} 00000 n\r\n", offsets[0]).as_bytes());
    file.extend_from_slice(b"0000000000 00001 f\r\n");
    file.extend_from_slice(format!("{:010} 00000 n\r\n", offsets[1]).as_bytes());
    file.extend_from_slice(format!("{:010} 00000 n\r\n", offsets[2]).as_bytes());
    file.extend_from_slice(b"trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n");
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    file
}

/// An **object** with the number it has in the file.
fn an_object(session: &mut Session, number: u64, entries: craftkit_sdk::serde_json::Value) -> u64 {
    let outcome = session
        .execute("pdf.object.add", &json!({ "object": number, "entries": entries }))
        .expect("the object is added");
    outcome.data["id"].as_u64().expect("the object has an id")
}

/// The **free numbers** of the document.
fn free_numbers(session: &mut Session) -> Vec<(u64, u64)> {
    let outcome = session.execute("pdf.file.free", &json!({})).expect("the free numbers are read");
    outcome.data["free"]
        .as_array()
        .expect("the free numbers are a list")
        .iter()
        .map(|entry| {
            (
                entry["object"].as_u64().expect("the number is there"),
                entry["generation"].as_u64().expect("the generation is there"),
            )
        })
        .collect()
}

/// The **next number** of the document.
fn next_number(session: &mut Session) -> u64 {
    let outcome = session.execute("pdf.file.free", &json!({})).expect("the free numbers are read");
    outcome.data["next"].as_u64().expect("the next number is there")
}

/// The **entries** of a file's table.
fn entries_of(session: &mut Session, path: &str) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome = session.execute("pdf.file.xref", &json!({ "path": path })).expect("the table is read");
    outcome.data["entries"].as_array().expect("the entries are a list").clone()
}

/// The **trailer** of a file.
fn trailer_of(session: &mut Session, path: &str) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome = session.execute("pdf.file.xref", &json!({ "path": path })).expect("the table is read");
    outcome.data["trailer"].as_array().expect("the trailer is a list").clone()
}

/// A file that is **loaded** keeps the numbers it says are free.
#[test]
fn a_file_that_is_loaded_keeps_its_free_numbers() {
    let directory = directory("load");
    let mut session = session();
    let path = format!("{directory}/free.pdf");
    std::fs::write(&path, a_file_with_a_free_number()).expect("the file is written");
    session
        .execute("pdf.file.load", &json!({ "path": path, "into": format!("{directory}/streams") }))
        .expect("the file is loaded");
    let free = free_numbers(&mut session);
    assert_eq!(free.len(), 1, "one number is free: {free:?}");
    assert_eq!(free[0].0, 2, "the number two is the free one: {free:?}");
    assert_eq!(free[0].1, 1, "and it has the generation the file gave it: {free:?}");
}

/// An object that is **taken away** frees its number.
#[test]
fn an_object_that_is_taken_away_frees_its_number() {
    let mut session = session();
    let object = an_object(&mut session, 2, json!([["Type", "Page"]]));
    session.execute("pdf.object.remove", &json!({ "object": object })).expect("the object is taken away");
    let free = free_numbers(&mut session);
    assert_eq!(free.len(), 1, "one number is free: {free:?}");
    assert_eq!(free[0].0, 2, "the number of the object is free: {free:?}");
    assert_eq!(free[0].1, 1, "the generation is one more than it was: {free:?}");
}

/// A page that is **taken away** frees its number too.
#[test]
fn a_page_that_is_taken_away_frees_its_number() {
    let mut session = session();
    let first = an_object(&mut session, 1, json!([["Type", "Page"]]));
    let second = an_object(&mut session, 2, json!([["Type", "Page"]]));
    session.execute("pdf.page.remove", &json!({ "page": second })).expect("the page is taken away");
    let free = free_numbers(&mut session);
    assert_eq!(free.len(), 1, "one number is free: {free:?}");
    assert_eq!(free[0].0, 2, "the number of the page is free: {free:?}");
    let _ = first;
}

/// A free number is **not used again**.
#[test]
fn a_free_number_is_not_used_again() {
    let mut session = session();
    let object = an_object(&mut session, 2, json!([["Type", "Page"]]));
    session.execute("pdf.object.remove", &json!({ "object": object })).expect("the object is taken away");
    let refused = session.execute("pdf.object.add", &json!({ "object": 2, "entries": [] }));
    let message = match refused {
        Ok(_) => panic!("a free number is not used again"),
        Err(error) => error.to_string(),
    };
    assert!(message.contains("free"), "the refusal says why: {message}");
}

/// The **next number** is past the free ones: a free number is a
/// number something had.
#[test]
fn the_next_number_is_past_the_free_ones() {
    let mut session = session();
    let object = an_object(&mut session, 5, json!([["Type", "Page"]]));
    session.execute("pdf.object.remove", &json!({ "object": object })).expect("the object is taken away");
    assert_eq!(next_number(&mut session), 6, "the next number is past the free one");
}

/// The **table** that is written has a row for the free number, and
/// the row says it is free.
#[test]
fn the_table_has_a_row_for_the_free_number() {
    let directory = directory("table");
    let mut session = session();
    an_object(&mut session, 1, json!([["Type", "Catalog"]]));
    let object = an_object(&mut session, 2, json!([["Type", "Page"]]));
    session.execute("pdf.object.remove", &json!({ "object": object })).expect("the object is taken away");
    let out = format!("{directory}/out.pdf");
    session.execute("pdf.file.write", &json!({ "path": out.clone() })).expect("the file is written");
    let entries = entries_of(&mut session, &out);
    let row = entries
        .iter()
        .find(|entry| entry["object"].as_u64() == Some(2))
        .expect("the free number has a row: {entries:?}");
    assert_eq!(row["kind"].as_str(), Some("free"), "the row says it is free: {row:?}");
    assert_eq!(row["generation"].as_u64(), Some(1), "with the generation it has: {row:?}");
}

/// The number **zero** is free, and the table's first row is its own.
#[test]
fn the_number_zero_is_free() {
    let directory = directory("zero");
    let mut session = session();
    an_object(&mut session, 1, json!([["Type", "Catalog"]]));
    let out = format!("{directory}/out.pdf");
    session.execute("pdf.file.write", &json!({ "path": out.clone() })).expect("the file is written");
    let entries = entries_of(&mut session, &out);
    let row = entries
        .iter()
        .find(|entry| entry["object"].as_u64() == Some(0))
        .expect("the number zero has a row: {entries:?}");
    assert_eq!(row["kind"].as_str(), Some("free"), "the row says it is free: {row:?}");
}

/// The **size** of the file is the next number, and not the count of
/// its objects: a file with a free number in it has more numbers than
/// it has objects.
#[test]
fn the_size_of_the_file_is_the_next_number() {
    let directory = directory("size");
    let mut session = session();
    an_object(&mut session, 1, json!([["Type", "Catalog"]]));
    let object = an_object(&mut session, 2, json!([["Type", "Page"]]));
    session.execute("pdf.object.remove", &json!({ "object": object })).expect("the object is taken away");
    let out = format!("{directory}/out.pdf");
    session.execute("pdf.file.write", &json!({ "path": out.clone() })).expect("the file is written");
    let trailer = trailer_of(&mut session, &out);
    let size = trailer
        .iter()
        .find_map(|entry| {
            let pair = entry.as_array()?;
            let name = pair.first()?.as_str()?;
            if name.trim_start_matches('/') != "Size" {
                return None;
            }
            pair.get(1)?.as_f64().map(|size| size as u64)
        })
        .expect("the trailer says the size");
    assert_eq!(size, 3, "the size is the next number: {trailer:?}");
}

/// The file that is written is **read back** with its free number
/// still free.
#[test]
fn the_free_number_survives_the_round_trip() {
    let directory = directory("round");
    let mut session = session();
    an_object(&mut session, 1, json!([["Type", "Catalog"]]));
    let object = an_object(&mut session, 2, json!([["Type", "Page"]]));
    session.execute("pdf.object.remove", &json!({ "object": object })).expect("the object is taken away");
    let out = format!("{directory}/out.pdf");
    session.execute("pdf.file.write", &json!({ "path": out.clone() })).expect("the file is written");
    let mut other = Session::with_formats(
        ecosystem().commands,
        ecosystem().kinds,
        ecosystem().formats,
        ecosystem().migrations,
        ecosystem().evaluators,
    );
    other
        .execute("pdf.file.load", &json!({ "path": out, "into": format!("{directory}/again") }))
        .expect("the file is loaded again");
    let free = free_numbers(&mut other);
    assert_eq!(free.len(), 1, "one number is free: {free:?}");
    assert_eq!(free[0].0, 2, "and it is the same number: {free:?}");
}

/// A number that is **not** free is not free.
#[test]
fn a_number_that_is_not_free_is_not_free() {
    let mut session = session();
    an_object(&mut session, 1, json!([["Type", "Catalog"]]));
    assert!(free_numbers(&mut session).is_empty(), "nothing is free");
}
