//! The **XML reader** of the SDK.
//!
//! One reader for the whole tree: FCPXML, FCP7's `xmeml` and SVG are
//! all XML, and a kit with a reader in each of them has a reader that
//! reads one text three ways.

use craftkit_sdk::xml;

/// The reader reads the **elements**, the **attributes** and the
/// **text**.
#[test]
fn the_reader_reads_a_text() {
    let root = xml::parse(
        "<?xml version=\"1.0\"?>\n<!DOCTYPE fcpxml>\n<!-- a comment -->\n<fcpxml version=\"1.9\"><sequence name=\"the film\"><spine/></sequence></fcpxml>",
    )
    .expect("the text is read");
    assert_eq!(root.name, "fcpxml", "the root is the element");
    assert_eq!(root.attribute("version"), Some("1.9"), "and it has its attributes");
    let sequence = root.child("sequence").expect("there is a sequence");
    assert_eq!(sequence.attribute("name"), Some("the film"), "with its own attributes");
    assert!(sequence.child("spine").is_some(), "and its children");
}

/// The reader reads the **entities**, and refuses a text that is not
/// XML.
#[test]
fn the_reader_reads_the_entities_and_refuses_the_rest() {
    let root = xml::parse("<a name=\"the &amp; the &lt;beach&gt;\"/>").expect("the text is read");
    assert_eq!(root.attribute("name"), Some("the & the <beach>"), "the entities are read");
    assert!(xml::parse("<a name=\"unfinished>").is_err(), "a text that is not XML is refused");
    assert!(xml::parse("<a></b>").is_err(), "and so is a text that is closed wrongly");
    assert!(xml::parse("not xml at all").is_err(), "and so is a text with no element");
}

/// The reader keeps the **text** of an element, and reads a **number**
/// out of an attribute.
#[test]
fn the_reader_keeps_the_text() {
    let root = xml::parse("<a><b>the words</b></a>").expect("the text is read");
    let b = root.child("b").expect("there is a b");
    assert_eq!(b.text, "the words", "the text is kept");
    let root = xml::parse("<a width=\"12.5\"/>").expect("the text is read");
    assert_eq!(root.number("width"), Some(12.5), "and a number is read");
    assert_eq!(root.number("height"), None, "and one that is not there is not");
}

/// The reader is **strict**: an element that is closed by another is
/// refused, and a file that is too deep is refused.
#[test]
fn the_reader_is_strict() {
    assert!(xml::parse("<a><b></a>").is_err(), "a wrong closing is refused");
    let deep = format!("{}{}", "<a>".repeat(200), "</a>".repeat(200));
    assert!(xml::parse(&deep).is_err(), "and a file that is too deep is refused");
}

/// The reader finds an element **anywhere** below the one it is given.
#[test]
fn the_reader_finds_an_element() {
    let root = xml::parse("<a><b><c/></b><d/></a>").expect("the text is read");
    assert!(root.find("c").is_some(), "the element is found");
    assert_eq!(root.find_all("d").len(), 1, "and how many there are");
    assert!(root.find("z").is_none(), "and one that is not there is not found");
}
