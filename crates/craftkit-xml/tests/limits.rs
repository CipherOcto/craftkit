//! The **limits of the xml crate**: the numbers it will not go past.
//!
//! The crate reads a part of XML, and the numbers it keeps are about
//! how deep a part may be and how many elements it may hold. A limit
//! that no test names is a limit nothing proves holds.

use craftkit_xml::{Error, MAX_DEPTH, MAX_XML_NODES, parse};

/// A part of XML of that many **nested** elements.
fn a_deep_part(depth: usize) -> Vec<u8> {
    let mut text = String::with_capacity(depth * 7);
    for _ in 0..depth {
        text.push_str("<a>");
    }
    for _ in 0..depth {
        text.push_str("</a>");
    }
    text.into_bytes()
}

/// A part of XML with that many **elements** in it.
fn a_wide_part(elements: usize) -> Vec<u8> {
    let mut text = String::with_capacity(elements * 4);
    text.push_str("<a>");
    for _ in 0..elements {
        text.push_str("<b/>");
    }
    text.push_str("</a>");
    text.into_bytes()
}

/// A part **deeper** than `MAX_DEPTH` is refused.
///
/// A part of XML that is nested deeper than the tree reads is a part
/// the tree does not read: the tree says how deep it goes.
#[test]
fn a_part_deeper_than_the_limit_is_refused() {
    let bytes = a_deep_part(MAX_DEPTH + 2);
    let error = parse(&bytes).expect_err("that part is too deep");
    match error {
        Error::TooLarge(message) => assert!(message.contains("deeper"), "the part is refused: {message}"),
        other => panic!("the part is refused: {other:?}"),
    }
}

/// A part with **more elements** than `MAX_XML_NODES` is refused.
#[test]
fn a_part_with_too_many_elements_is_refused() {
    let bytes = a_wide_part(MAX_XML_NODES + 2);
    let error = parse(&bytes).expect_err("that part holds too many elements");
    match error {
        Error::TooLarge(message) => assert!(message.contains("elements"), "the part is refused: {message}"),
        other => panic!("the part is refused: {other:?}"),
    }
}

/// A part **within** the limits is read, and its root is there.
#[test]
fn a_part_within_the_limits_is_read() {
    let bytes = a_deep_part(MAX_DEPTH - 2);
    let root = parse(&bytes).expect("the part is read");
    assert_eq!(root.name, "a", "the name of the root");
    assert_eq!(root.children.len(), 1, "and it has a child");
}
