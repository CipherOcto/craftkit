//! The **XMP metadata** of a file.

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

/// The **node** of an object, by its number.
fn node_of(session: &Session, object: u64) -> u64 {
    session
        .document()
        .walk()
        .find(|node| node.data.get("object").and_then(|number| number.as_u64()) == Some(object))
        .map(|node| node.id)
        .expect("the object is there")
}

/// An **object** with a dictionary.
fn add_object(session: &mut Session, object: u64, entries: Json) -> u64 {
    session
        .execute("pdf.object.add", &json!({ "object": object, "entries": entries }))
        .expect("the object is added");
    node_of(session, object)
}

/// A **directory** for the bytes of this test.
fn a_directory(name: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("craftkit-v2-metadata-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path
}

/// A **file** with a catalog and an information dictionary.
fn a_file(session: &mut Session) -> u64 {
    let catalog = add_object(session, 2, json!([["Type", "Catalog"]]));
    let information = add_object(
        session,
        3,
        json!([
            ["Title", { "string": "Report" }],
            ["Author", { "string": "A & B" }],
            ["CreationDate", { "string": "D:20240105093000Z" }],
        ]),
    );
    session.execute("pdf.info.set", &json!({ "object": information })).expect("the information is said");
    catalog
}

/// The **packet** of the file, as the command writes it.
fn write_the_packet(session: &mut Session, directory: &std::path::Path, level: &str) -> String {
    let path = directory.join("metadata.xml");
    session
        .execute("pdf.metadata.write", &json!({ "path": path.to_string_lossy(), "level": level }))
        .expect("the packet is written");
    std::fs::read_to_string(&path).expect("the packet is there")
}

/// The **clauses** the preflight reports.
fn clauses_of(session: &mut Session, level: &str) -> Vec<String> {
    let outcome = session.execute("pdf.preflight", &json!({ "level": level })).expect("the checker runs");
    outcome.data["issues"]
        .as_array()
        .expect("the issues are a list")
        .iter()
        .filter_map(|issue| issue["clause"].as_str().map(|clause| clause.to_string()))
        .collect()
}

/// The packet says the **level** the file is.
#[test]
fn the_packet_says_the_level_the_file_is() {
    let mut session = session();
    a_file(&mut session);
    let directory = a_directory("level");
    let packet = write_the_packet(&mut session, &directory, "a2b");
    assert!(packet.contains("<pdfaid:part>2</pdfaid:part>"), "the packet says the part: {packet}");
    assert!(packet.contains("<pdfaid:conformance>B</pdfaid:conformance>"), "and the conformance");
}

/// The packet carries the **information** the file has.
#[test]
fn the_packet_carries_the_information_the_file_has() {
    let mut session = session();
    a_file(&mut session);
    let directory = a_directory("information");
    let packet = write_the_packet(&mut session, &directory, "a2b");
    assert!(packet.contains("<dc:title>"), "the packet has a title: {packet}");
    assert!(packet.contains("Report"), "and the title is the one the file has");
    assert!(packet.contains("A &amp; B"), "and the words are escaped");
    assert!(
        packet.contains("<xmp:CreateDate>2024-01-05T09:30:00Z</xmp:CreateDate>"),
        "and the date is an XMP date"
    );
}

/// The **catalog** names the metadata.
#[test]
fn the_catalog_names_the_metadata() {
    let mut session = session();
    let catalog = a_file(&mut session);
    let directory = a_directory("catalog");
    write_the_packet(&mut session, &directory, "a2b");
    let node = session.document().node(catalog).expect("the catalog is there");
    let entries = node.data["entries"].clone();
    let metadata = entries
        .as_array()
        .expect("the entries are a list")
        .iter()
        .find_map(|entry| {
            let pair = entry.as_array()?;
            if pair.first()?.as_str()? != "Metadata" {
                return None;
            }
            Some(pair.get(1)?.clone())
        })
        .expect("the catalog has a Metadata");
    let number = metadata
        .get("ref")
        .and_then(|pair| pair.as_array())
        .and_then(|pair| pair.first())
        .and_then(|number| number.as_u64());
    assert!(number.is_some(), "the catalog names the metadata: {metadata}");
}

/// The **checker** stops saying the file has no metadata.
#[test]
fn the_checker_stops_saying_the_file_has_no_metadata() {
    let mut session = session();
    a_file(&mut session);
    assert!(clauses_of(&mut session, "a2b").contains(&"6.6.2.1".to_string()), "the checker says so");
    let directory = a_directory("checker");
    write_the_packet(&mut session, &directory, "a2b");
    let clauses = clauses_of(&mut session, "a2b");
    assert!(!clauses.contains(&"6.6.2.1".to_string()), "the checker is quiet: {clauses:?}");
    assert!(!clauses.contains(&"6.6.4".to_string()), "and the file says what it is: {clauses:?}");
}

/// A level that is **not a level** is refused.
#[test]
fn a_level_that_is_not_a_level_is_refused() {
    let mut session = session();
    a_file(&mut session);
    let directory = a_directory("level-refused");
    let path = directory.join("metadata.xml");
    let refused = session
        .execute("pdf.metadata.write", &json!({ "path": path.to_string_lossy(), "level": "a9z" }))
        .expect_err("the level is refused");
    assert!(refused.to_string().contains("is not a level"), "the refusal says why: {refused}");
}

/// A path that is **empty** is refused.
#[test]
fn a_path_that_is_empty_is_refused() {
    let mut session = session();
    a_file(&mut session);
    let refused = session
        .execute("pdf.metadata.write", &json!({ "path": "  ", "level": "a2b" }))
        .expect_err("the path is refused");
    assert!(refused.to_string().contains("bytes go somewhere"), "the refusal says why: {refused}");
}

/// A file with **no catalog** is refused.
#[test]
fn a_file_with_no_catalog_is_refused() {
    let mut session = session();
    let directory = a_directory("no-catalog");
    let path = directory.join("metadata.xml");
    let refused = session
        .execute("pdf.metadata.write", &json!({ "path": path.to_string_lossy(), "level": "a2b" }))
        .expect_err("the file is refused");
    assert!(refused.to_string().contains("no catalog"), "the refusal says why: {refused}");
}
