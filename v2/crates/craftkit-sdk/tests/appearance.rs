//! The **appearance**: how a thing looks.
//!
//! The appearance is the kit's own — not a plugin's payload — and it can
//! be **shared**: a thing points at the node that holds the colours, and
//! the colours are changed in one place.

use craftkit_sdk::Document;
use craftkit_sdk::document::{Appearance, BlendMode, Colour, Fill, Node, Stroke};

/// A colour, as a person writes it.
fn colour(text: &str) -> Colour {
    Colour::from_hex(text).expect("that is a colour")
}

/// A document with an **appearance** node and a thing that uses it.
fn a_document() -> (Document, u64, u64) {
    let mut document = Document::new("the look of it");
    let appearance = document.add_node("blue", "core.appearance", None);
    let fill = colour("#3366cc");
    if let Some(node) = document.node_mut(appearance) {
        node.appearance = Some(Appearance {
            fill: Fill::Solid(fill),
            stroke: Some(Stroke { colour: Colour::BLACK, width: 2.0, ..Default::default() }),
            blend: BlendMode::Normal,
        });
    }
    let shape = document.add_node("a shape", "geometry.rect", None);
    if let Some(node) = document.node_mut(shape) {
        node.appearance_from = Some(appearance);
    }
    (document, appearance, shape)
}

/// A thing that uses an appearance **resolves** it.
///
/// The document follows the reference: one hop, and no more.
#[test]
fn a_thing_that_uses_an_appearance_resolves_it() {
    let (document, _appearance, shape) = a_document();
    let node = document.node(shape).expect("the shape is there");
    let resolved = document.appearance_of(node).expect("the appearance is resolved");
    assert_eq!(resolved.fill, Fill::Solid(colour("#3366cc")));
    assert_eq!(resolved.stroke.map(|stroke| stroke.width), Some(2.0));
}

/// An appearance of its **own** wins over the one it points at.
///
/// A node that says what it looks like has said the last word.
#[test]
fn an_appearance_of_its_own_wins() {
    let (mut document, _appearance, shape) = a_document();
    let own = Appearance::filled(colour("#ff0000"));
    if let Some(node) = document.node_mut(shape) {
        node.appearance = Some(own.clone());
    }
    let node = document.node(shape).expect("the shape is there");
    assert_eq!(document.appearance_of(node), Some(own));
}

/// A node with **no** appearance has none.
///
/// It is not white and it is not black: it says nothing, and the caller
/// says what to do about that.
#[test]
fn a_node_with_no_appearance_has_none() {
    let mut document = Document::new("plain");
    let shape = document.add_node("a shape", "geometry.rect", None);
    let node = document.node(shape).expect("the shape is there");
    assert_eq!(document.appearance_of(node), None);
}

/// The appearance is **written and read**.
///
/// A document that keeps an appearance in the file keeps it in the file:
/// the field is one of the node's own, and a reader that does not know
/// it keeps it as one of the fields it does not understand.
#[test]
fn the_appearance_is_written_and_read() {
    let (document, _appearance, shape) = a_document();
    let node = document.node(shape).expect("the shape is there");
    let value = node.to_json();
    assert!(value.get("appearance_from").is_some(), "the reference is written: {value}");
    let read = Node::from_json(&value).expect("the node is read");
    assert_eq!(read.appearance_from, node.appearance_from, "the reference survives");
    assert_eq!(read.appearance, node.appearance, "and so does an appearance of its own");
}
