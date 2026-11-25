//! The **cuts of the pdf battery**: the places where it stops early and
//! says so.
//!
//! A reader that stops at a bound must say that it stopped: the outline
//! of a file is read to a depth and to a number of items, and the
//! bookmarks below the depth and beyond the number are in the file and
//! not in the answer. Every cut of this battery is named here, and the
//! bound it stops at is named in **code**, so that the proof and the
//! bound cannot drift apart.
//!
//! The outline of a file is the only way to reach the depth: a bookmark
//! may not be put under a bookmark that is already at the bound, so a
//! tree deeper than the bound comes from a file and not from the
//! commands.

use craftkit_plugin_pdf::{MAX_OUTLINE_DEPTH, MAX_OUTLINE_ITEMS};
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
    let path =
        craftkit_scratch::dir(&format!("craftkit-v2-pdf-outline-{name}")).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file of PDF whose outline is a **chain** of `depth` bookmarks.
///
/// Each bookmark is under the one before it, and the first of them is
/// under the root of the outline.
fn a_file_with_a_chain(name: &str, depth: usize) -> String {
    let mut bodies: Vec<(u32, String)> = vec![
        (1, "<< /Type /Catalog /Pages 2 0 R /Outlines 3 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [] /Count 0 >>".to_string()),
        (3, "<< /Type /Outlines /First 4 0 R >>".to_string()),
    ];
    for index in 0..depth {
        let number = 4 + index as u32;
        let parent = number - 1;
        let mut body = format!("<< /Title (the item {index}) /Parent {parent} 0 R");
        if index + 1 < depth {
            body.push_str(&format!(" /First {} 0 R", number + 1));
        }
        body.push_str(" >>");
        bodies.push((number, body));
    }
    a_file(name, bodies)
}

/// A file of PDF whose outline has `count` **siblings**.
///
/// Each bookmark is under the root of the outline, and they are one
/// another's next.
fn a_file_with_siblings(name: &str, count: usize) -> String {
    let mut bodies: Vec<(u32, String)> = vec![
        (1, "<< /Type /Catalog /Pages 2 0 R /Outlines 3 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [] /Count 0 >>".to_string()),
        (3, format!("<< /Type /Outlines /First 4 0 R /Last {} 0 R >>", 3 + count as u32)),
    ];
    for index in 0..count {
        let number = 4 + index as u32;
        let mut body = format!("<< /Title (the item {index}) /Parent 3 0 R");
        if index + 1 < count {
            body.push_str(&format!(" /Next {} 0 R", number + 1));
        }
        body.push_str(" >>");
        bodies.push((number, body));
    }
    a_file(name, bodies)
}

/// A file of PDF with the **objects** it holds.
fn a_file(name: &str, bodies: Vec<(u32, String)>) -> String {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in &bodies {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    let size = bodies.len() + 1;
    file.extend_from_slice(format!("xref\n0 {size}\n").as_bytes());
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n").as_bytes());
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    std::fs::write(&path, file).expect("the file is written");
    path
}

/// The **outline** of a file, as the reader gives it.
fn the_outline(session: &mut Session, path: &str) -> craftkit_sdk::serde_json::Value {
    let into = format!("{path}.streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
    let outcome = session.execute("pdf.outline.read", &json!({})).expect("the outline is read");
    outcome.data.clone()
}

/// The **depth** of the tree the reader gave: the levels of it, the item
/// itself counted as one.
fn depth_of(item: &craftkit_sdk::serde_json::Value) -> usize {
    let children = item["children"].as_array().expect("the item has children");
    1 + children.iter().map(depth_of).max().unwrap_or(0)
}

/// An outline **deeper** than `MAX_OUTLINE_DEPTH` is cut, and the reader
/// says so.
///
/// The file holds bookmarks below the depth the reader reads, and the
/// answer holds none of them: the answer says that it stopped.
#[test]
fn an_outline_deeper_than_the_bound_is_cut_and_the_reader_says_so() {
    let mut session = session();
    let path = a_file_with_a_chain("thedeepoutline", MAX_OUTLINE_DEPTH + 2);
    let data = the_outline(&mut session, &path);
    assert_eq!(data["truncated"], json!(true), "the reader says it stopped: {data:?}");
    let items = data["items"].as_array().expect("there are items");
    assert_eq!(items.len(), 1, "the first item is there: {data:?}");
    assert_eq!(depth_of(&items[0]), MAX_OUTLINE_DEPTH + 1, "the tree stops at the bound: {data:?}");
}

/// An outline of **less depth** than the bound is not cut, and the
/// reader says so.
///
/// The other half of the test above: a bound that is always reached
/// proves nothing, because it cannot tell a cut from a whole answer.
#[test]
fn an_outline_within_the_bound_is_not_cut() {
    let mut session = session();
    let path = a_file_with_a_chain("theshallowoutline", 3);
    let data = the_outline(&mut session, &path);
    assert_eq!(data["truncated"], json!(false), "nothing was cut: {data:?}");
    let items = data["items"].as_array().expect("there are items");
    assert_eq!(items.len(), 1, "the first item is there: {data:?}");
    assert_eq!(depth_of(&items[0]), 3, "the whole tree is there: {data:?}");
}

/// The **list** of the children of an item is cut at
/// `MAX_OUTLINE_ITEMS`, and the reader says so.
///
/// The tree of a file may be wide as well as deep, and a tree that is
/// wider than the bound is cut as well.
#[test]
fn the_list_of_the_children_is_cut_at_the_bound() {
    let mut session = session();
    let path = a_file_with_siblings("thewideoutline", MAX_OUTLINE_ITEMS + 1);
    let data = the_outline(&mut session, &path);
    let items = data["items"].as_array().expect("there are items");
    assert_eq!(items.len(), MAX_OUTLINE_ITEMS, "the list is cut at the bound: {data:?}");
    assert_eq!(data["truncated"], json!(true), "the reader says it stopped: {data:?}");
}

/// The **hidden report** of a file whose outline is deeper than the
/// bound says that it stopped.
///
/// The report counts the bookmarks of the file, and the count is a count
/// of a cut tree when the outline is deeper than the reader reads: the
/// report says so, because a count of less than the file holds is a
/// count the reader has to be told about.
#[test]
fn the_hidden_report_says_when_the_bookmarks_are_cut() {
    let mut session = session();
    let depth = MAX_OUTLINE_DEPTH + 2;
    let path = a_file_with_a_chain("thedeepebookmarks", depth);
    the_outline(&mut session, &path);
    let outcome = session.execute("pdf.hidden.scan", &json!({})).expect("the file is scanned");
    assert_eq!(outcome.data["truncated"], json!(true), "the report says it stopped: {outcome:?}");
    let categories = outcome.data["categories"].as_array().expect("there are categories");
    let bookmarks = categories
        .iter()
        .find(|category| category["category"] == json!("Bookmarks"))
        .expect("there is a category for the bookmarks");
    let count = bookmarks["count"].as_u64().expect("the category has a count");
    assert!(count < depth as u64, "the count is a count of a cut tree: {outcome:?}");
}

/// The **hidden report** of a file whose outline is within the bound is
/// not cut, and it says so.
#[test]
fn the_hidden_report_of_a_whole_outline_is_not_cut() {
    let mut session = session();
    let path = a_file_with_a_chain("thewholebookmarks", 3);
    the_outline(&mut session, &path);
    let outcome = session.execute("pdf.hidden.scan", &json!({})).expect("the file is scanned");
    assert_eq!(outcome.data["truncated"], json!(false), "nothing was cut: {outcome:?}");
    let categories = outcome.data["categories"].as_array().expect("there are categories");
    let bookmarks = categories
        .iter()
        .find(|category| category["category"] == json!("Bookmarks"))
        .expect("there is a category for the bookmarks");
    let count = bookmarks["count"].as_u64().expect("the category has a count");
    assert_eq!(count, 3, "the count is the count of the bookmarks: {outcome:?}");
}
