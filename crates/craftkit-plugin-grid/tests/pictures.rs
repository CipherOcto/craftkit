//! The **picture** of a cell: a file beside the document and a size
//! inside it.

use craftkit_sdk::document::Colour;
use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_grid::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the grid battery.
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

/// A **workbook** with a **sheet** in it.
fn a_sheet(session: &mut Session) -> u64 {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    outcome.data["id"].as_u64().expect("the sheet has an id")
}

/// A **PNG file** of that many pixels across and down.
///
/// It is written with the tree's own writer, so the tree carries no
/// artwork and the fixture is made here and not shipped.
fn a_png_file(name: &str, width: usize, height: usize) -> String {
    let mut image = craftkit_raster::Image::new(width, height, Colour::WHITE);
    image.blend(0, 0, Colour::BLACK, craftkit_sdk::BlendMode::Normal);
    let path =
        craftkit_scratch::file(&format!("craftkit-v2-grid-picture-{name}.png")).expect("the scratch is made");
    std::fs::write(&path, image.to_png()).expect("the picture is written");
    path.to_string_lossy().to_string()
}

/// The **picture** of a cell, as it is read.
fn picture_of(session: &mut Session, sheet: u64, cell: &str) -> craftkit_sdk::serde_json::Value {
    let outcome = session
        .execute("grid.picture.read", &json!({ "sheet": sheet, "cell": cell }))
        .expect("the picture is read");
    outcome.data.clone()
}

/// **A picture may be set, and it is read back.**
#[test]
fn a_picture_may_be_set() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let path = a_png_file("set", 4, 3);
    session
        .execute(
            "grid.picture.set",
            &json!({ "sheet": sheet, "cell": "B2", "path": path, "alt": "a square" }),
        )
        .expect("the picture is set");
    let read = picture_of(&mut session, sheet, "B2");
    assert_eq!(read["picture"]["width"], json!(4));
    assert_eq!(read["picture"]["height"], json!(3));
    assert_eq!(read["picture"]["format"], json!("png"));
    assert_eq!(read["picture"]["alt"], json!("a square"));
}

/// **The size is measured from the head of the file.**
#[test]
fn the_size_is_measured_from_the_head() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let path = a_png_file("measured", 5, 2);
    session
        .execute("grid.picture.set", &json!({ "sheet": sheet, "cell": "B2", "path": path }))
        .expect("the picture is set");
    let read = picture_of(&mut session, sheet, "B2");
    assert_eq!(read["pixels"], json!(10), "five across and two down is ten pixels");
    assert_eq!(read["aspect"], json!(2.5));
}

/// **The bytes are in the file, and the document says where.**
#[test]
fn the_bytes_are_in_the_file() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let path = a_png_file("bytes", 2, 2);
    session
        .execute("grid.picture.set", &json!({ "sheet": sheet, "cell": "B2", "path": path.clone() }))
        .expect("the picture is set");
    let read = picture_of(&mut session, sheet, "B2");
    assert_eq!(read["picture"]["path"], json!(path));
    assert!(std::path::Path::new(&path).exists(), "the file is there and the document does not hold it");
}

/// **A file that is not a picture is refused.**
#[test]
fn a_file_that_is_not_a_picture_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let path =
        craftkit_scratch::file("craftkit-v2-grid-picture-not-a-picture.png").expect("the scratch is made");
    std::fs::write(&path, b"not a picture").expect("the file is written");
    let outcome = session.execute(
        "grid.picture.set",
        &json!({ "sheet": sheet, "cell": "B2", "path": path.to_string_lossy() }),
    );
    assert!(outcome.is_err(), "a file that is not a picture is refused");
}

/// **A file that is not there is refused.**
#[test]
fn a_file_that_is_not_there_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome = session
        .execute("grid.picture.set", &json!({ "sheet": sheet, "cell": "B2", "path": "/nowhere/at/all.png" }));
    assert!(outcome.is_err(), "a file that is not there is refused");
}

/// **A cell that is covered by a merge is refused.**
#[test]
fn a_cell_that_is_covered_by_a_merge_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let path = a_png_file("merged", 2, 2);
    session
        .execute("grid.merge.set", &json!({ "sheet": sheet, "range": "B2:C3" }))
        .expect("the cells are merged");
    let outcome = session
        .execute("grid.picture.set", &json!({ "sheet": sheet, "cell": "C3", "path": path }))
        .expect_err("a covered cell is refused");
    assert!(outcome.to_string().contains("merge"), "{outcome}");
}

/// **A picture may be taken away.**
#[test]
fn a_picture_may_be_taken_away() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let path = a_png_file("removed", 2, 2);
    session
        .execute("grid.picture.set", &json!({ "sheet": sheet, "cell": "B2", "path": path }))
        .expect("the picture is set");
    session
        .execute("grid.picture.remove", &json!({ "sheet": sheet, "cell": "B2" }))
        .expect("the picture is taken away");
    let outcome = session.execute("grid.picture.read", &json!({ "sheet": sheet, "cell": "B2" }));
    assert!(outcome.is_err(), "the cell holds no picture");
}

/// **Every picture of the sheet may be read at once.**
#[test]
fn every_picture_of_the_sheet_may_be_read() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let path = a_png_file("all", 2, 2);
    for cell in ["B2", "C3"] {
        session
            .execute("grid.picture.set", &json!({ "sheet": sheet, "cell": cell, "path": path.clone() }))
            .expect("the picture is set");
    }
    let outcome =
        session.execute("grid.picture.read", &json!({ "sheet": sheet })).expect("the pictures are read");
    let pictures = outcome.data["pictures"].as_array().expect("the pictures are a list");
    assert_eq!(pictures.len(), 2);
}
