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
//! (`craftkit_foundation::MAX_VALUE_DEPTH`), and this test is what keeps the two
//! in step: if the document's shape ever nests deeper, or the reader's limit
//! moves, this test fails instead of the tree quietly writing designs that
//! cannot be opened.

use craftkit_foundation::{MAX_VALUE_DEPTH, Value};
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

/// A design too deep to read is **not written**.
///
/// §287: the writer bounded the **size** of a file and the reader bounded
/// the **depth**, and the two limits lived in different crates — so the
/// writer produced files the tree's own reader refused. Eight nested
/// property groups and the deepest value the tree accepts are legal to the
/// model and unreadable to the reader.
#[test]
fn a_design_that_is_too_deep_to_read_is_not_written() {
    let document = document_in_groups(nested(MAX_VALUE_DEPTH), 8);
    document.validate().expect("the model accepts this design");
    let err = craftkit_io::save_text(&document).expect_err("the writer refuses what the reader refuses");
    assert!(err.to_string().contains("nests"), "the refusal does not say what it refused: {err}");
}

/// And a file that arrived from elsewhere is refused in the tree's own words.
///
/// The parser's own message read like a malformed file — "the file is not
/// valid JSON" — and the file *was* valid JSON; the tree refused its own
/// depth. §287.
#[test]
fn a_file_too_deep_to_read_is_refused_in_the_tree_s_own_words() {
    let deep = format!("{}1{}", "[".repeat(200), "]".repeat(200));
    let err = craftkit_io::load_text(&deep).expect_err("the reader refuses it");
    let message = err.to_string();
    assert!(message.contains("nests"), "the refusal does not say what it refused: {message}");
    assert!(!message.contains("not valid JSON"), "a deep file is not a malformed file: {message}");
}

/// A document with one property holding `value`, inside `groups` nested groups.
fn document_in_groups(value: Value, groups: usize) -> Document {
    let mut document = Document::empty("depth");
    let id = document.add_node("In", NodeKind::Source).expect("a node");
    let mut group = PropGroup::new("props", "Properties");
    group.nodes.push(craftkit_model::PropNode::Prop(Property::new(1, "v", value)));
    // Each group is a container of its own, so the file nests deeper per
    // group.
    for depth in 0..groups {
        let mut outer = PropGroup::new(format!("g{depth}"), format!("Group {depth}"));
        outer.nodes.push(craftkit_model::PropNode::Group(group));
        group = outer;
    }
    document.get_mut(id).expect("the node").props = group;
    document
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
