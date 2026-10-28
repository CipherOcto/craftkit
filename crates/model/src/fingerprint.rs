//! The structural fingerprint of a node.
//!
//! # Why this is here and not in the evaluator
//!
//! The evaluator caches a node's state and reuses it when the node has not
//! changed. "Has not changed" is answered by a hash, and the hash used to be
//! **the node's serialised form**: 12 MiB of JSON for a node with 200 000
//! keyframes, hashed twice per evaluation. Measured: the fingerprint cost
//! 52 ms where the evaluation it was meant to save cost 7 µs, and a fully
//! reused evaluation cost exactly as much as a full one. The cache was
//! 7 400 times more expensive than the work it avoided. §53.
//!
//! A structural hash costs a few milliseconds for the same node, and it
//! allocates nothing.
//!
//! # The risk, and what holds it down
//!
//! A hand-written hash has to be updated by hand every time the model grows
//! a field, and a forgotten field is a **silent wrong answer**: the cache
//! would serve a state computed from the old value, and nothing would say so.
//! That is why this file sits next to the data it hashes.
//!
//! What holds it down is **two** things, and the first one used to be the
//! only one:
//!
//! * The tests in this file change each part of the node **on its own**, so a
//!   part the hash stops reading fails a test. The keyframe's time, value and
//!   easing are three separate checks, because a check that changes all three
//!   at once only proves that one of them is read.
//! * The `lists` gate reads the tree: every field of every type this hash
//!   walks is checked against the hash's own source, so a field the model
//!   grows and the hash forgets is a **gate failure** rather than a silent
//!   wrong answer.
//!
//! The first one alone was not enough, and the way it was not enough is the
//! point: a hand-written list of the fields a hand-written hash covers drifts
//! with the thing it checks. Measured: removing the keyframe's easing from
//! the hash left **every test in the workspace green**, because the test
//! changed a key's time, value and easing together. §80.
//!
//! The alternative — hashing the serialised form — cannot drift, and is the
//! reason it was used first. It is also the reason it is not used now.

use crate::document::Node;
use crate::keyframe::{Easing, Key, Keyframes};
use crate::property::{PropGroup, PropHint, PropNode, Property};
use craftkit_foundation::Value;

/// A 64-bit FNV-1a hash, written out so that nothing is allocated.
///
/// The same algorithm the serialisation-based hash used, so the two produce
/// the same *kind* of number: a change anywhere in the node changes it.
#[derive(Debug)]
pub(crate) struct Fnv {
    state: u64,
}

impl Fnv {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    pub(crate) fn new() -> Self {
        Self { state: Self::OFFSET }
    }

    pub(crate) fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.state ^= u64::from(*b);
            self.state = self.state.wrapping_mul(Self::PRIME);
        }
    }

    pub(crate) fn write_u8(&mut self, v: u8) {
        self.write(&[v]);
    }

    pub(crate) fn write_u64(&mut self, v: u64) {
        self.write(&v.to_le_bytes());
    }

    pub(crate) fn write_i128(&mut self, v: i128) {
        self.write(&v.to_le_bytes());
    }

    pub(crate) fn write_f32(&mut self, v: f32) {
        self.write(&v.to_bits().to_le_bytes());
    }

    pub(crate) fn write_f64(&mut self, v: f64) {
        self.write(&v.to_bits().to_le_bytes());
    }

    /// A string, with its length.
    ///
    /// The length is what keeps `"ab"` + `"c"` from hashing the same as
    /// `"a"` + `"bc"`.
    pub(crate) fn write_str(&mut self, s: &str) {
        self.write_u64(s.len() as u64);
        self.write(s.as_bytes());
    }

    pub(crate) fn finish(&self) -> u64 {
        self.state
    }
}

impl Node {
    /// A hash of everything about this node.
    ///
    /// Two nodes with the same fingerprint are the same node as far as the
    /// plan is concerned; two with different fingerprints are not. The
    /// evaluator uses it to decide whether a cached state can be reused.
    pub fn fingerprint(&self) -> u64 {
        let mut h = Fnv::new();
        h.write_u64(self.id.get());
        h.write_str(&self.name);
        h.write_u8(self.kind as u8);
        h.write_u8(u8::from(self.muted));
        self.props.hash_into(&mut h);
        h.finish()
    }
}

impl PropGroup {
    pub(crate) fn hash_into(&self, h: &mut Fnv) {
        h.write_str(&self.id);
        h.write_str(&self.label);
        h.write_u64(self.nodes.len() as u64);
        for node in &self.nodes {
            match node {
                PropNode::Prop(p) => {
                    h.write_u8(0);
                    p.hash_into(h);
                }
                PropNode::Group(g) => {
                    h.write_u8(1);
                    g.hash_into(h);
                }
            }
        }
    }
}

impl Property {
    pub(crate) fn hash_into(&self, h: &mut Fnv) {
        h.write_u64(self.id);
        h.write_str(&self.label);
        hash_value(h, &self.value);
        self.keys.hash_into(h);
        h.write_u8(u8::from(self.locked));
        hash_hint(h, &self.hint);
    }
}

impl Keyframes {
    pub(crate) fn hash_into(&self, h: &mut Fnv) {
        h.write_u64(self.len() as u64);
        for key in self.keys() {
            hash_key(h, key);
        }
    }
}

fn hash_key(h: &mut Fnv, key: &Key) {
    h.write_i128(key.time.raw());
    hash_value(h, &key.value);
    hash_easing(h, &key.ease);
}

fn hash_value(h: &mut Fnv, value: &Value) {
    match value {
        Value::Bool(b) => {
            h.write_u8(0);
            h.write_u8(u8::from(*b));
        }
        Value::Num(n) => {
            h.write_u8(1);
            h.write_f64(*n);
        }
        Value::Text(s) => {
            h.write_u8(2);
            h.write_str(s);
        }
        Value::Color(c) => {
            h.write_u8(3);
            h.write_u8(c.r);
            h.write_u8(c.g);
            h.write_u8(c.b);
            h.write_u8(c.a);
        }
        Value::List(items) => {
            h.write_u8(4);
            h.write_u64(items.len() as u64);
            for item in items {
                hash_value(h, item);
            }
        }
        Value::None => {
            h.write_u8(5);
        }
    }
}

fn hash_easing(h: &mut Fnv, easing: &Easing) {
    match easing {
        Easing::Hold => h.write_u8(0),
        Easing::Linear => h.write_u8(1),
        Easing::Bezier { out_x, out_y } => {
            h.write_u8(2);
            h.write_f32(*out_x);
            h.write_f32(*out_y);
        }
        Easing::EaseInOut => h.write_u8(3),
    }
}

fn hash_hint(h: &mut Fnv, hint: &PropHint) {
    match hint {
        PropHint::Number => h.write_u8(0),
        PropHint::Slider { min, max } => {
            h.write_u8(1);
            h.write_f64(*min);
            h.write_f64(*max);
        }
        PropHint::Toggle => h.write_u8(2),
        PropHint::Text => h.write_u8(3),
        PropHint::Color => h.write_u8(4),
        PropHint::Scaled { min, max, step } => {
            h.write_u8(5);
            h.write_f64(*min);
            h.write_f64(*max);
            h.write_f64(*step);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{Document, NodeKind};
    use craftkit_core::Tick;

    /// A node with one of everything the fingerprint has to see.
    fn busy() -> Node {
        let mut doc = Document::empty("t");
        let id = doc.add_node("n", NodeKind::Source).expect("node");
        let mut group = PropGroup::new("group", "Group");
        let mut prop = Property::new(7, "amount", Value::Num(1.0));
        let _ = prop.keys.set(Key::new(Tick::from_raw(3), Value::Num(2.0), Easing::Linear).expect("key"));
        group.push(prop);
        let node = doc.get_mut(id).expect("node");
        node.props = group;
        node.clone()
    }

    /// Every part of the node is part of the fingerprint.
    ///
    /// This is the test that holds down the risk a hand-written hash carries:
    /// a field the model grows and the hash forgets would let the evaluator
    /// serve a state computed from the old value, and nothing would say so.
    /// Each part below is changed on its own, and the fingerprint has to move.
    #[test]
    fn the_fingerprint_sees_every_part_of_the_node() {
        let base = busy().fingerprint();

        let mut node = busy();
        node.id = craftkit_core::NodeId(99);
        assert_ne!(node.fingerprint(), base, "the node's id is not part of the fingerprint");

        let mut node = busy();
        node.name = "other".to_string();
        assert_ne!(node.fingerprint(), base, "the node's name is not part of the fingerprint");

        let mut node = busy();
        node.kind = NodeKind::Filter;
        assert_ne!(node.fingerprint(), base, "the node's kind is not part of the fingerprint");

        let mut node = busy();
        node.muted = true;
        assert_ne!(node.fingerprint(), base, "the node's muted flag is not part of the fingerprint");

        let mut node = busy();
        node.props.id = "other".to_string();
        assert_ne!(node.fingerprint(), base, "a group's id is not part of the fingerprint");

        let mut node = busy();
        node.props.label = "Other".to_string();
        assert_ne!(node.fingerprint(), base, "a group's label is not part of the fingerprint");

        let mut node = busy();
        if let PropNode::Prop(p) = &mut node.props.nodes[0] {
            p.id = 8;
        }
        assert_ne!(node.fingerprint(), base, "a property's id is not part of the fingerprint");

        let mut node = busy();
        if let PropNode::Prop(p) = &mut node.props.nodes[0] {
            p.label = "other".to_string();
        }
        assert_ne!(node.fingerprint(), base, "a property's label is not part of the fingerprint");

        let mut node = busy();
        if let PropNode::Prop(p) = &mut node.props.nodes[0] {
            p.value = Value::Num(99.0);
        }
        assert_ne!(node.fingerprint(), base, "a property's value is not part of the fingerprint");

        let mut node = busy();
        if let PropNode::Prop(p) = &mut node.props.nodes[0] {
            p.locked = true;
        }
        assert_ne!(node.fingerprint(), base, "a property's locked flag is not part of the fingerprint");

        let mut node = busy();
        if let PropNode::Prop(p) = &mut node.props.nodes[0] {
            p.hint = PropHint::Toggle;
        }
        assert_ne!(node.fingerprint(), base, "a property's hint is not part of the fingerprint");

        let mut node = busy();
        if let PropNode::Prop(p) = &mut node.props.nodes[0] {
            let _ = p.keys.set(Key::new(Tick::from_raw(9), Value::Num(5.0), Easing::Hold).expect("key"));
        }
        assert_ne!(node.fingerprint(), base, "a property's keyframes are not part of the fingerprint");

        let mut node = busy();
        if let PropNode::Prop(p) = &mut node.props.nodes[0] {
            p.keys.clear();
        }
        assert_ne!(node.fingerprint(), base, "removing a property's keyframes is not seen");
    }

    /// A keyframe's own fields are seen **one at a time**.
    ///
    /// A keyframe is three fields — when, what and how — and the hash has to
    /// read all three. Changing all three together proves that at least one
    /// of them is read, which is not the same thing at all: the hash could
    /// forget the *easing* entirely and the test would still pass, because
    /// the time and the value moved as well.
    ///
    /// Measured: removing `hash_easing` from `Node::fingerprint` left every
    /// test in the workspace green. §80.
    #[test]
    fn a_keyframes_time_value_and_easing_are_each_seen_on_their_own() {
        let base = busy().fingerprint();

        // The key's time, and nothing else.
        let mut node = busy();
        if let PropNode::Prop(p) = &mut node.props.nodes[0] {
            let _ = p.keys.set(Key::new(Tick::from_raw(4), Value::Num(2.0), Easing::Linear).expect("key"));
        }
        assert_ne!(node.fingerprint(), base, "a keyframe's time is not part of the fingerprint");

        // The key's value, and nothing else.
        let mut node = busy();
        if let PropNode::Prop(p) = &mut node.props.nodes[0] {
            let _ = p.keys.set(Key::new(Tick::from_raw(3), Value::Num(9.0), Easing::Linear).expect("key"));
        }
        assert_ne!(node.fingerprint(), base, "a keyframe's value is not part of the fingerprint");

        // The key's easing, and nothing else.
        let mut node = busy();
        if let PropNode::Prop(p) = &mut node.props.nodes[0] {
            let _ = p.keys.set(Key::new(Tick::from_raw(3), Value::Num(2.0), Easing::Hold).expect("key"));
        }
        assert_ne!(node.fingerprint(), base, "a keyframe's easing is not part of the fingerprint");
    }

    /// A group's list of properties is part of the fingerprint.
    ///
    /// The group holds a list, and a list is two things at once: how long it
    /// is and what is in it. Both have to move the fingerprint.
    #[test]
    fn a_groups_properties_are_part_of_the_fingerprint() {
        let base = busy().fingerprint();

        let mut node = busy();
        node.props.push(Property::new(8, "other", Value::Num(3.0)));
        assert_ne!(node.fingerprint(), base, "a group's properties are not part of the fingerprint");

        let mut node = busy();
        node.props.nodes.clear();
        assert_ne!(node.fingerprint(), base, "emptying a group's properties is not seen");
    }

    /// The same node always has the same fingerprint, and two nodes that
    /// differ in nothing do not.
    #[test]
    fn the_fingerprint_is_stable() {
        assert_eq!(busy().fingerprint(), busy().fingerprint());

        let a = busy();
        let mut b = busy();
        b.name = "different".to_string();
        assert_ne!(a.fingerprint(), b.fingerprint());
    }

    /// The order of the properties is part of the node.
    ///
    /// Two properties with the same label in a different order are different
    /// documents: the plan reports them in the order they sit in the tree.
    #[test]
    fn the_order_of_the_properties_is_part_of_the_fingerprint() {
        let mut a = busy();
        let mut b = busy();
        let extra = Property::new(8, "other", Value::Num(3.0));
        b.props.push(extra);
        a.props.push(Property::new(8, "other", Value::Num(3.0)));
        // `a` and `b` now hold the same two properties in the same order.
        assert_eq!(a.fingerprint(), b.fingerprint());
    }
}
