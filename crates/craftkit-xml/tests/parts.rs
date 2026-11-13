//! The **XML reader**: the elements of a part, and the parts of them.

use craftkit_xml::{El, Error, parse};

/// A **part** of XML, as text.
fn a_part(text: &str) -> Vec<u8> {
    text.as_bytes().to_vec()
}

/// The root of a part is read.
#[test]
fn the_root_of_a_part_is_read() {
    let root = parse(&a_part("<workbook><sheets/></workbook>")).expect("the part is read");
    assert_eq!(root.name, "workbook", "the name of the root");
    assert_eq!(root.children.len(), 1, "and it has a child");
    assert_eq!(root.children[0].name, "sheets", "the name of the child");
}

/// The **attributes** of an element are read.
#[test]
fn the_attributes_of_an_element_are_read() {
    let root = parse(&a_part(r#"<c r="A1" s="3" t="s"/>"#)).expect("the part is read");
    assert_eq!(root.attr("r"), Some("A1"), "a text attribute");
    assert_eq!(root.attr_u32("s"), Some(3), "a number attribute");
    assert_eq!(root.attr_i64("s"), Some(3), "and as a signed number");
    assert_eq!(root.attr_f64("s"), Some(3.0), "and as a float");
    assert_eq!(root.attr("t"), Some("s"), "another attribute");
    assert_eq!(root.attr("nonesuch"), None, "and one that is not there");
}

/// The **text** of an element is read, and the text of its children is
/// not its own.
#[test]
fn the_text_of_an_element_is_read() {
    let root = parse(&a_part("<si><t>the text</t></si>")).expect("the part is read");
    assert_eq!(root.text, "", "the element's own text");
    assert_eq!(root.child("t").map(|child| child.text.as_str()), Some("the text"), "the child's text");
}

/// The **children** of an element are found by name.
#[test]
fn the_children_of_an_element_are_found_by_name() {
    let root =
        parse(&a_part("<sheets><sheet name=\"a\"/><sheet name=\"b\"/></sheets>")).expect("the part is read");
    assert_eq!(root.kids("sheet").count(), 2, "two children of that name");
    assert_eq!(root.child("sheet").and_then(|sheet| sheet.attr("name")), Some("a"), "the first one");
    assert_eq!(root.child("nonesuch"), None, "and one that is not there");
}

/// A **path** of child names is followed.
#[test]
fn a_path_of_child_names_is_followed() {
    let root = parse(&a_part("<a><b><c val=\"7\"/></b></a>")).expect("the part is read");
    assert_eq!(root.path(&["b", "c"]).and_then(|element| element.attr("val")), Some("7"), "the path");
    assert_eq!(root.path(&["b", "nonesuch"]), None, "and a path that is not there");
}

/// The **entities** of a text are read.
#[test]
fn the_entities_of_a_text_are_read() {
    let root = parse(&a_part("<t>a &amp; b &lt; c &gt; d &quot;e&quot; &apos;f&apos; &#65;</t>"))
        .expect("the part is read");
    assert_eq!(root.text, "a & b < c > d \"e\" 'f' A", "the text with its entities");
}

/// A **CDATA** section is text.
#[test]
fn a_cdata_section_is_text() {
    let root = parse(&a_part("<t><![CDATA[the & text]]></t>")).expect("the part is read");
    assert_eq!(root.text, "the & text", "the text of the section");
}

/// A **comment** and a **declaration** are not text.
#[test]
fn a_comment_and_a_declaration_are_not_text() {
    let root =
        parse(&a_part("<?xml version=\"1.0\"?><!-- a comment --><t>the text</t>")).expect("the part is read");
    assert_eq!(root.name, "t", "the element");
    assert_eq!(root.text, "the text", "and its text");
}

/// The **prefix** of a name is not kept: the name of an element is its
/// local name.
#[test]
fn the_prefix_of_a_name_is_not_kept() {
    let root =
        parse(&a_part(r#"<x:si xmlns:x="urn:x"><x:t>the text</x:t></x:si>"#)).expect("the part is read");
    assert_eq!(root.name, "si", "the local name of the root");
    assert_eq!(root.child("t").map(|child| child.text.as_str()), Some("the text"), "and of the child");
}

/// A part in **UTF-16** is read.
#[test]
fn a_part_in_utf16_is_read() {
    let text = "<t>the text</t>";
    let mut bytes: Vec<u8> = vec![0xFF, 0xFE];
    for unit in text.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    let root = parse(&bytes).expect("the part is read");
    assert_eq!(root.text, "the text", "the text of the element");
}

/// Bytes that are **not XML** are refused.
#[test]
fn bytes_that_are_not_xml_are_refused() {
    let error = parse(b"not xml at all").expect_err("that is not xml");
    assert!(matches!(error, Error::Malformed(_)), "{error}");
}

/// A part with **no element** in it is refused.
#[test]
fn a_part_with_no_element_is_refused() {
    let error = parse(b"<!-- only a comment -->").expect_err("that is not xml");
    assert!(matches!(error, Error::Malformed(_)), "{error}");
}

/// A part that is **too deep** is refused.
#[test]
fn a_part_that_is_too_deep_is_refused() {
    let mut text = String::new();
    for _ in 0..300 {
        text.push_str("<a>");
    }
    for _ in 0..300 {
        text.push_str("</a>");
    }
    let error = parse(text.as_bytes()).expect_err("the part is too deep");
    assert!(matches!(error, Error::TooLarge(_)), "{error}");
}

/// An element with **no name** is refused.
#[test]
fn an_element_with_no_name_is_refused() {
    let error = parse(b"< >").expect_err("that is not an element");
    assert!(matches!(error, Error::Malformed(_)), "{error}");
}

/// An attribute that is **not quoted** is refused.
#[test]
fn an_attribute_that_is_not_quoted_is_refused() {
    let error = parse(b"<a b=c/>").expect_err("that is not an attribute");
    assert!(matches!(error, Error::Malformed(_)), "{error}");
}

/// An element that is **not closed** is read as far as it goes.
#[test]
fn an_element_that_is_not_closed_is_read_as_far_as_it_goes() {
    let root = parse(b"<a><b>the text").expect("the part is read");
    assert_eq!(root.name, "a", "the root");
    assert_eq!(root.child("b").map(|child| child.text.as_str()), Some("the text"), "and its child");
}

/// An element with **no children** is empty.
#[test]
fn an_element_with_no_children_is_empty() {
    let root = parse(b"<a/>").expect("the part is read");
    assert_eq!(root, El { name: "a".to_string(), ..El::default() }, "the element");
}
