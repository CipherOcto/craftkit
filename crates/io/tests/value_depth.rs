//! The deepest value the tree accepts must survive the tree's own format.
//!
//! **Why this is a test and not a note.** §260 measured it: the tree's value
//! bound was 64, the tree's own file could be read back only to a value
//! nested **59** deep, and so the tree could **save a design it could not
//! open**. The file was on the disk and the tree refused it — with the
//! reader's own message, "recursion limit exceeded", which says nothing
//! about the tree's own bound.
//!
//! The bound is now measured on the **file**, not on the loader's bare value
//! (`craftkit_core::MAX_VALUE_DEPTH`), and this test is what keeps the two
//! in step: if the document's shape ever nests deeper, or the reader's limit
//! moves, this test fails instead of the tree quietly writing designs that
//! cannot be opened.

use craftkit_core::{MAX_VALUE_DEPTH, Value};
use craftkit_model::{Document, NodeKind, PropGroup, Property};

/// A value nested `levels` list levels deep.
fn nested(levels: usize) -> Value {
    let mut value = Value::Num(1.0);
    for _ in 0..levels {
        value = Value::list(vec![value]).expect("a value");
    }
    value
}

/// A document with one property holding `value`.
fn document_with(value: Value) -> Document {
    let mut document = Document::empty("depth");
    let id = document.add_node("In", NodeKind::Source).expect("a node");
    let mut group = PropGroup::default();
    group.nodes.push(craftkit_model::PropNode::Prop(Property::new(1, "v", value)));
    document.get_mut(id).expect("the node").props = group;
    document
}

/// The deepest value the tree accepts is written and read back.
#[test]
fn the_deepest_value_the_tree_accepts_round_trips() {
    let document = document_with(nested(MAX_VALUE_DEPTH));
    let text = craftkit_io::save_text(&document).expect("the design is written");
    let read = craftkit_io::load_text(&text).expect("the design is read back");
    assert_eq!(read.nodes.len(), 1, "the document came back");
}

/// A value one level deeper cannot even be **built**: the tree's own
/// constructor refuses it, so the tree cannot write one.
#[test]
fn a_value_past_the_bound_is_refused_by_the_tree() {
    let mut value = Value::Num(1.0);
    for _ in 0..MAX_VALUE_DEPTH {
        value = Value::list(vec![value]).expect("a value");
    }
    let outcome = Value::list(vec![value]);
    assert!(outcome.is_err(), "the tree built a value past its bound: {outcome:?}");
}
