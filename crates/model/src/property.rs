//! The property tree.
//!
//! Every node in the document owns a [`PropGroup`]: a two-level tree of named
//! values, some of which are animated. It is the same shape the command API
//! addresses, so there is exactly one addressing scheme to learn.
//!
//! ```text
//! node "Delay"
//! ├── group "trigger"
//! │   ├── prop "enabled"   bool
//! │   └── prop "lead"      number  (animated)
//! └── group "output"
//!     └── prop "amount"    number
//! ```
//!
//! # The invariant
//!
//! **Every property the tree holds is named by exactly one path, and that path
//! is one the model's own parser accepts.**
//!
//! Three parts of this crate used to disagree about that, and each
//! disagreement lost a property somewhere. The listing ([`PropGroup::flatten`])
//! published paths the parser ([`PropPath::parse`]) refused; the read side
//! ([`PropGroup::find`]) and the write side ([`PropGroup::find_mut`]) resolved
//! a uid differently, so a nested property could be read by `@5` and not
//! written by it; and the listing counted name occurrences exactly while the
//! resolver counted them case-insensitively, so the published path of
//! `amount` could resolve to a property called `Amount`. §85.
//!
//! The rules that hold it together:
//!
//! - **One name-matching rule** ([`node_matches`]): a group's id and a
//!   property's label both match case-insensitively, everywhere.
//! - **The tree is as deep as a path is long**: a property tree deeper than
//!   [`limits::MAX_PATH_SEGMENTS`] is refused by [`PropGroup::validate`]
//!   rather than quietly truncated by the listing.
//! - **The model creates properties too** ([`PropGroup::insert`]): a path that
//!   names something that cannot exist yet — an id, a position — is refused
//!   rather than turned into a different thing.
//! - **A name is written as it is**: a label or a group id the grammar cannot
//!   write — one that contains the separator, or begins with the character
//!   that marks an id or a position — is refused, because the listing would
//!   publish a path that means something else. §88.

use crate::keyframe::{Key, Keyframes};
use crate::limits;
use crate::path::{PathSeg, PropPath};
use craftkit_core::{CoreError, Value};
use serde::{Deserialize, Serialize};

/// One animatable or settable value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Property {
    /// A stable id, unique within the node. Survives rename and reorder, which
    /// is what makes it worth having alongside the label.
    #[serde(default)]
    pub id: u64,
    /// The display name, and the name a path matches on.
    pub label: String,
    /// The value when nothing is animating it.
    pub value: Value,
    /// Animated keys, empty when the property is static.
    #[serde(default)]
    pub keys: Keyframes,
    /// Whether a user may change this. A locked property is greyed in the UI and
    /// refused by `property.set`, so the rule lives in the document rather than
    /// in every caller.
    #[serde(default)]
    pub locked: bool,
    /// What the UI shows. Advisory, but the engine never relies on it.
    /// How the UI should edit this property.
    ///
    /// Defaulted: a file written before hints existed has no reason to fail
    /// loading over one.
    #[serde(default)]
    pub hint: PropHint,
}

/// How the UI should present a property.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum PropHint {
    #[default]
    Number,
    /// A number with a slider; carries the range so the UI and any validator
    /// agree instead of drifting apart.
    Slider {
        min: f64,
        max: f64,
    },
    Toggle,
    Text,
    Color,
    /// A number with a unit shown next to it.
    Scaled {
        min: f64,
        max: f64,
        step: f64,
    },
}

impl Property {
    pub fn new(id: u64, label: impl Into<String>, value: Value) -> Self {
        // Compute the hint from a reference, so `value` is still owned below.
        let hint = match &value {
            Value::Bool(_) => PropHint::Toggle,
            Value::Num(_) => PropHint::Number,
            Value::Text(_) => PropHint::Text,
            Value::Color(_) => PropHint::Color,
            _ => PropHint::Number,
        };
        Self { id, label: label.into(), value, keys: Keyframes::new(), locked: false, hint }
    }

    pub fn with_hint(mut self, hint: PropHint) -> Self {
        self.hint = hint;
        self
    }

    pub fn locked(mut self) -> Self {
        self.locked = true;
        self
    }

    /// The value at `time`: the animated one if there are keys, otherwise the
    /// static one. **This is the only way a caller should read a property.**
    pub fn value_at(&self, time: craftkit_core::Tick) -> Value {
        self.keys.sample(time).unwrap_or_else(|| self.value.clone())
    }

    pub fn is_animated(&self) -> bool {
        !self.keys.is_empty()
    }

    pub fn matches(&self, name: &str) -> bool {
        self.label.eq_ignore_ascii_case(name)
    }

    /// A cheap, conservative estimate of the memory this property holds.
    ///
    /// Never under-estimates: a budget a document can lie about is not a
    /// budget. §52.
    ///
    /// **The property's own value counts.** It did not, and the sentence above
    /// was false because of it: a property whose value was eight megabytes of
    /// text was estimated at a hundred bytes, so the document budget and the
    /// undo budget both said the document was tiny while it held a gigabyte.
    /// The keyframes were counted — §52's mutation even says "that is where a
    /// document's memory goes" — but the value was not, and a value is where
    /// a hostile document's memory goes. §73.
    pub fn estimated_bytes(&self) -> usize {
        std::mem::size_of::<Property>()
            + self.label.capacity()
            + self.value.estimated_bytes()
            + self
                .keys
                .keys()
                .iter()
                .map(|k| std::mem::size_of::<Key>() + k.value.estimated_bytes())
                .sum::<usize>()
    }
}

/// A node in the property tree: either a nested group or a leaf.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PropNode {
    Group(PropGroup),
    Prop(Property),
}

/// Whether a child's name matches `name`.
///
/// **One rule, one place.** The resolver, the listing and the creator all ask
/// this question, and they used to ask it three different ways: the resolver
/// matched case-insensitively, the listing counted occurrences
/// case-sensitively, and the engine's own insertion used neither. Two sibling
/// properties called `Amount` and `amount` were the smallest case that showed
/// it — the listing published `amount` for the second one, and the resolver
/// sent `amount` to the first. §85.
fn node_matches(node: &PropNode, name: &str) -> bool {
    match node {
        PropNode::Group(g) => g.id.eq_ignore_ascii_case(name),
        PropNode::Prop(p) => p.matches(name),
    }
}

/// Resolve one segment against a node list.
fn step_of<'a>(nodes: &'a [PropNode], seg: &PathSeg) -> Option<&'a PropNode> {
    let n = nodes;
    match seg {
        PathSeg::Index(i) => n.get(i.checked_sub(1)?),
        PathSeg::Uid(_) => None,
        PathSeg::Match(name, want) => {
            let mut seen = 0usize;
            for node in n {
                if node_matches(node, name) {
                    seen += 1;
                    if seen == *want {
                        return Some(node);
                    }
                }
            }
            None
        }
    }
}

/// Resolve one segment against a node list, mutably.
fn step_mut<'a>(nodes: &'a mut [PropNode], seg: &PathSeg) -> Option<&'a mut PropNode> {
    let idx = match seg {
        PathSeg::Uid(_) => return None,
        other => index_of_seg(nodes, other)?,
    };
    nodes.get_mut(idx)
}

/// The 0-based index a segment names, or `None`.
fn index_of_seg(nodes: &[PropNode], seg: &PathSeg) -> Option<usize> {
    match seg {
        PathSeg::Index(i) => i.checked_sub(1).filter(|i| *i < nodes.len()),
        PathSeg::Uid(id) => nodes.iter().position(|n| matches!(n, PropNode::Prop(p) if p.id == *id)),
        PathSeg::Match(name, want) => {
            let mut seen = 0usize;
            for (i, node) in nodes.iter().enumerate() {
                if node_matches(node, name) {
                    seen += 1;
                    if seen == *want {
                        return Some(i);
                    }
                }
            }
            None
        }
    }
}

/// Depth-first search for a property id anywhere in a slice of nodes.
///
/// Free function rather than a method so the borrow of `self` does not conflict
/// with the borrow of the node being returned.
fn find_uid_in(nodes: &[PropNode], id: u64) -> Option<&PropNode> {
    for n in nodes {
        match n {
            PropNode::Prop(p) if p.id == id => return Some(n),
            PropNode::Group(g) => {
                if let Some(found) = find_uid_in(&g.nodes, id) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}

/// Depth-first search for a property id anywhere in a slice of nodes,
/// mutably.
///
/// The mutable twin of [`find_uid_in`], and it exists because the two halves of
/// the model disagreed: `find` looked for a uid **anywhere below** the group
/// it was addressing, and `find_mut` looked only among that group's direct
/// children. A property nested one group down could therefore be *read* by
/// `@5` and not *written* by `@5` — and `prop.set`, which writes, answered
/// "there is no such property" by inventing a new one. §85.
fn find_uid_in_mut(nodes: &mut [PropNode], id: u64) -> Option<&mut Property> {
    for node in nodes.iter_mut() {
        match node {
            PropNode::Prop(p) if p.id == id => return Some(p),
            PropNode::Group(g) => {
                if let Some(found) = find_uid_in_mut(&mut g.nodes, id) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}

/// A named group of properties, possibly nested.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PropGroup {
    /// The stable id used in paths. Distinct from the label, which is for
    /// humans and may be duplicated; this may not.
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub nodes: Vec<PropNode>,
}

/// A path segment naming `label`, numbered if this is not its first use among
/// its **siblings**.
///
/// Siblings, not the whole tree: `PathSeg::Match(name, n)` counts the matching
/// children of one group, so a counter that ran across levels would emit a
/// `#2` no resolver could ever satisfy. (A group id repeated anywhere in the
/// tree is a different problem, and `validate` rejects it.)
///
/// The count is **case-insensitive**, because the resolver's is: it matches a
/// group id or a property label with `eq_ignore_ascii_case`. A counter that
/// compared exactly would number `Amount` and `amount` as two firsts, and the
/// second one's published path would resolve to the first. §85.
fn occurrence(label: String, seen: &mut Vec<String>) -> PathSeg {
    let count = seen.iter().filter(|s| s.eq_ignore_ascii_case(&label)).count() + 1;
    seen.push(label.clone());
    PathSeg::Match(label, count)
}

impl PropGroup {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self { id: id.into(), label: label.into(), nodes: Vec::new() }
    }

    pub fn with_prop(mut self, p: Property) -> Self {
        self.push(p);
        self
    }

    pub fn with_group(mut self, g: PropGroup) -> Self {
        self.push_group(g);
        self
    }

    pub fn push(&mut self, p: Property) -> &mut Self {
        self.nodes.push(PropNode::Prop(p));
        self
    }

    pub fn push_group(&mut self, g: PropGroup) -> &mut Self {
        self.nodes.push(PropNode::Group(g));
        self
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Total properties anywhere below here.
    pub fn count_props(&self) -> usize {
        self.nodes
            .iter()
            .map(|n| match n {
                PropNode::Prop(_) => 1,
                PropNode::Group(g) => g.count_props(),
            })
            .sum()
    }

    /// A cheap, conservative estimate of the memory this group holds.
    ///
    /// Never under-estimates: a budget a document can lie about is not a
    /// budget. §52.
    pub fn estimated_bytes(&self) -> usize {
        std::mem::size_of::<PropGroup>()
            + self.id.capacity()
            + self.label.capacity()
            + self.nodes.capacity() * std::mem::size_of::<PropNode>()
            + self
                .nodes
                .iter()
                .map(|n| match n {
                    PropNode::Prop(p) => p.estimated_bytes(),
                    PropNode::Group(g) => g.estimated_bytes(),
                })
                .sum::<usize>()
    }

    /// Total nesting depth below here; a leaf group is depth 1.
    pub fn depth(&self) -> usize {
        self.nodes
            .iter()
            .map(|n| match n {
                PropNode::Prop(_) => 1,
                PropNode::Group(g) => 1 + g.depth(),
            })
            .max()
            .unwrap_or(0)
    }

    /// Check the invariants that keep a hostile or corrupt document from
    /// reaching an allocation or an ambiguity.
    pub fn validate(&self) -> Result<(), CoreError> {
        let mut prop_ids: Vec<u64> = Vec::new();
        self.check(0, &mut prop_ids, &mut 0)?;
        // Duplicate property ids make `@` addressing ambiguous, which is worse
        // than a missing property: it silently resolves to the wrong one.
        prop_ids.sort_unstable();
        if prop_ids.windows(2).any(|w| w[0] == w[1]) {
            return Err(CoreError::LimitReached { limit: "duplicate property id" });
        }
        Ok(())
    }

    fn check(&self, depth: usize, prop_ids: &mut Vec<u64>, total: &mut usize) -> Result<(), CoreError> {
        // The property tree is as deep as a path can be.
        //
        // A property inside a group at depth `d` is named by a path of `d + 1`
        // segments, and the parser accepts at most `MAX_PATH_SEGMENTS` of them.
        // A tree deeper than that holds properties that **no path can name**:
        // the listing used to stop at the limit and drop the rest silently,
        // while `validate` — this function — called the tree good. Measured: a
        // tree of seventeen groups validated, and `flatten` returned no
        // properties at all for it. §85.
        //
        // So the depth is bounded by the *path* limit, not by `MAX_DEPTH`:
        // `MAX_DEPTH` is how deep the **node graph** may be (the evaluator's
        // rule), and a property tree that is deeper than a path is a tree
        // whose properties are unreachable.
        if depth >= limits::MAX_PATH_SEGMENTS {
            return Err(CoreError::LimitReached { limit: "property depth" });
        }
        // A group id must be unique **among its siblings**.
        //
        // Two groups with the same id under one parent would make `#2` and
        // `dup#1` ambiguous about which one they name — that is the rule, and
        // it is a real one.
        //
        // The same id at *different* depths is not ambiguous, and the path
        // scheme says so in as many words: `flatten` counts the occurrences
        // of a name **per level**, because *"`trigger/x` and
        // `trigger/trigger/y` are not competing for the same name space at
        // all"*. A command can create exactly that — `prop.set` with the path
        // `a/trigger/lead` and then `trigger/lead` — and the old, tree-wide
        // rule called the result invalid, so the engine could write a file
        // that its own reader refused. §50.
        //
        // The comparison is case-insensitive, because the resolver's is
        // (`index_of_seg` matches a group id with `eq_ignore_ascii_case`):
        // two siblings called `Trigger` and `trigger` would always resolve to
        // the first of them, which is exactly the ambiguity the rule is for.
        let mut siblings: Vec<&str> = Vec::new();
        for node in &self.nodes {
            if let PropNode::Group(g) = node
                && siblings.iter().any(|id| id.eq_ignore_ascii_case(&g.id))
            {
                return Err(CoreError::LimitReached { limit: "duplicate group id" });
            }
            if let PropNode::Group(g) = node {
                siblings.push(&g.id);
            }
        }

        for node in &self.nodes {
            match node {
                PropNode::Prop(p) => {
                    *total += 1;
                    if *total > limits::MAX_PROPERTIES {
                        return Err(CoreError::LimitReached { limit: "properties per node" });
                    }
                    if p.label.is_empty() || p.label.len() > limits::MAX_NAME {
                        return Err(CoreError::TooLong {
                            what: "property label".to_string(),
                            len: p.label.len(),
                            max: limits::MAX_NAME,
                        });
                    }
                    // A label is a path segment, so it has to be one the
                    // grammar can write.
                    //
                    // A file can carry a label the grammar reads as something
                    // else — `a/b` is two segments, `@5` is an id, `#2` is a
                    // position — and the tree used to accept it: the listing
                    // published a path that meant something else, and a
                    // command using that path **created a second property**
                    // instead of changing the one on screen. §88.
                    if let Some(why) = crate::path::why_not_writable(&p.label) {
                        return Err(CoreError::BadPath(format!(
                            "the property label `{}` cannot be written as a path: {why}",
                            p.label
                        )));
                    }
                    // The value, and the values of the keys.
                    //
                    // These are the invariants the *constructors* keep, and a
                    // file is not built by a constructor: it is deserialized
                    // straight into the model, so a file can carry a number a
                    // hundred million times past the limit, a text past the
                    // text limit, or a list past the list limit. The
                    // evaluator's comment says "every number the tree stores
                    // is" in range — this is where that becomes true rather
                    // than merely intended. §73.
                    p.value.check(&p.label)?;
                    for key in p.keys.keys() {
                        key.value.check(&p.label)?;
                    }
                    prop_ids.push(p.id);
                }
                PropNode::Group(g) => {
                    // A group's id is a path segment, so it has to be one the
                    // grammar can write. A group whose id is `a/b` would be
                    // listed as `a/b/…` — a path that names two segments — and
                    // nothing could reach the properties inside it. §88.
                    if let Some(why) = crate::path::why_not_writable(&g.id) {
                        return Err(CoreError::BadPath(format!(
                            "the group id `{}` cannot be written as a path: {why}",
                            g.id
                        )));
                    }
                    g.check(depth + 1, prop_ids, total)?;
                }
            }
        }
        Ok(())
    }

    /// Resolve a path to a property, read-only.
    ///
    /// A segment that is a bare `@id` searches the **whole subtree**, which is
    /// what makes an id a durable reference: a script that recorded `@1f4a`
    /// keeps working after the property is renamed or moved.
    pub fn find(&self, path: &PropPath) -> Option<&Property> {
        let segs = path.segs();
        let last = segs.last()?;
        let mut group: &PropGroup = self;

        for seg in &segs[..segs.len() - 1] {
            match step_of(&group.nodes, seg)? {
                PropNode::Group(g) => group = g,
                PropNode::Prop(_) => return None,
            }
        }

        // The last segment may be a uid anywhere below the current group.
        if let PathSeg::Uid(id) = last {
            let found = find_uid_in(&group.nodes, *id)?;
            return match found {
                PropNode::Prop(p) => Some(p),
                PropNode::Group(_) => None,
            };
        }
        match step_of(&group.nodes, last)? {
            PropNode::Prop(p) => Some(p),
            PropNode::Group(_) => None,
        }
    }

    /// Resolve a path to a property, mutably.
    ///
    /// Written out separately from [`PropGroup::find`] rather than shared: the
    /// read-only version can hand back a borrow from anywhere in the tree, but
    /// the mutable version can only walk downward, and forcing one shape on both
    /// produces a helper that is wrong for one of them.
    ///
    /// **It resolves the same paths `find` does.** A uid in the last position
    /// names the property with that id *anywhere below* the group being
    /// addressed — the rule the read-only half uses — so a script that recorded
    /// an id keeps working whether it reads the property or writes it. §85.
    pub fn find_mut(&mut self, path: &PropPath) -> Option<&mut Property> {
        let last = path.segs().last()?;
        let mut group: &mut PropGroup = self;
        // Every segment but the last names a group on the way down.
        for seg in &path.segs()[..path.segs().len() - 1] {
            let next = step_mut(&mut group.nodes, seg)?;
            group = match next {
                PropNode::Group(g) => g,
                PropNode::Prop(_) => return None,
            };
        }
        // The last segment may be a uid anywhere below the current group.
        if let PathSeg::Uid(id) = last {
            return find_uid_in_mut(&mut group.nodes, *id);
        }
        let idx = index_of_seg(&group.nodes, last)?;
        match group.nodes.get_mut(idx)? {
            PropNode::Prop(p) => Some(p),
            PropNode::Group(_) => None,
        }
    }

    /// Every property below here as `(path, &Property)`, for the UI, the
    /// evaluator and the dumps.
    ///
    /// The paths are **label-based** — `amount`, `trigger/lead` — with an
    /// occurrence count appended when a label repeats among its siblings
    /// (`amount#2`). Two properties with the same label is a thing a person can
    /// do by accident, so the path must still resolve to exactly one of them,
    /// and `PropPath::parse` resolves `name#2` the same way.
    ///
    /// An earlier version emitted `#1`, `#2` by position instead. That is
    /// unambiguous, and it is unusable: nobody types `#2`, and it made the tree
    /// have a second addressing scheme alongside the label paths commands take
    /// and the ones `Resolved.path` reports. One scheme, spelled the way a
    /// person would spell it.
    pub fn flatten(&self) -> Vec<(PropPath, &Property)> {
        let mut out = Vec::new();
        self.walk(&mut Vec::new(), &mut out);
        out
    }

    fn walk<'a>(&'a self, prefix: &mut Vec<PathSeg>, out: &mut Vec<(PropPath, &'a Property)>) {
        // Counted per level: `amount` and `amount#2` are siblings, while
        // `trigger/x` and `trigger/trigger/y` are not competing for the same
        // name space at all.
        let mut seen: Vec<String> = Vec::new();
        for node in &self.nodes {
            match node {
                PropNode::Prop(p) => {
                    let mut segs = prefix.clone();
                    segs.push(occurrence(p.label.clone(), &mut seen));
                    out.push((PropPath::from_segs(segs), p));
                }
                PropNode::Group(g) => {
                    let id = g.id.clone();
                    let seg = occurrence(id, &mut seen);
                    prefix.push(seg);
                    // Descend only while the path is still one the parser
                    // accepts.
                    //
                    // A property inside a group at depth `d` is named by a path
                    // of `d + 1` segments, so the last group that may be
                    // walked into is the one at depth `MAX_PATH_SEGMENTS - 1`.
                    // The bound used to be `<=`, which listed a path of
                    // `MAX_PATH_SEGMENTS + 1` segments — a path the parser
                    // refuses — and the next group down was dropped without a
                    // word. §85.
                    if prefix.len() < limits::MAX_PATH_SEGMENTS {
                        g.walk(prefix, out);
                    }
                    prefix.pop();
                }
            }
        }
    }

    /// Create a property at `path`, making the groups it needs on the way.
    ///
    /// **This is the only way to make a property.** The engine used to split
    /// the path string on `/` itself and use each piece as a literal group id,
    /// which meant it could build a tree the model's grammar could not
    /// describe: `effects/#2/amount` made a group whose id was literally
    /// `#2`, and `@5` made a property labelled `5`. Neither could be read
    /// back by the path that created it, so `prop.set` reported success and
    /// left a property nothing could see. §85.
    ///
    /// So the rules are the model's own:
    ///
    /// - A segment that names a **label** names a group (on the way down) or a
    ///   property (the last segment). Groups are created as needed; the
    ///   property is created with the label the segment spells.
    /// - A segment that names an **id** (`@5`) names a property that already
    ///   exists. There is no such property to create — ids are the model's to
    ///   assign — so it is refused.
    /// - A segment that names a **position** (`#2`) names a child that already
    ///   exists. A new property has no position yet, so it is refused.
    ///
    /// The path must already be one the grammar accepts: `PropPath::parse` is
    /// the gate, and a path it refuses never reaches here.
    pub fn insert(&mut self, path: &PropPath, id: u64, value: Value) -> Result<(), CoreError> {
        let segs = path.segs();
        let Some((last, rest)) = segs.split_last() else {
            return Err(CoreError::BadPath("a path needs at least one segment".to_string()));
        };
        let mut current = self;
        for seg in rest {
            current = current.group_for(seg)?;
        }
        let label = match last {
            PathSeg::Match(name, want) => {
                let have = current.matches_named(name);
                if have + 1 == *want {
                    name.clone()
                } else if have < *want {
                    return Err(CoreError::BadPath(format!(
                        "`{}` names the {} `{name}`, and this group holds {have}",
                        segment_text(name, *want),
                        ordinal(*want)
                    )));
                } else {
                    // The name is taken. Say by what.
                    let idx = index_of_seg(&current.nodes, last)
                        .ok_or_else(|| CoreError::BadPath(format!("`{name}` is not here")))?;
                    return Err(match current.nodes.get(idx) {
                        Some(PropNode::Group(_)) => {
                            CoreError::BadPath(format!("`{name}` is a group, not a property"))
                        }
                        _ => CoreError::BadPath(format!("there is already a property called `{name}` here")),
                    });
                }
            }
            PathSeg::Uid(id) => {
                return Err(CoreError::BadPath(format!(
                    "`@{id:x}` names a property by its id, and there is no property with that id to create"
                )));
            }
            PathSeg::Index(n) => {
                return Err(CoreError::BadPath(format!(
                    "`#{n}` names the {} child of its group, and a new property has no position yet",
                    ordinal(*n)
                )));
            }
        };
        current.push(Property::new(id, label, value));
        Ok(())
    }

    /// The group a segment names, created if it does not exist yet.
    fn group_for(&mut self, seg: &PathSeg) -> Result<&mut PropGroup, CoreError> {
        match seg {
            PathSeg::Uid(id) => Err(CoreError::BadPath(format!(
                "`@{id:x}` names a property by its id; a property is a leaf, so it cannot name a group"
            ))),
            PathSeg::Index(n) => Err(CoreError::BadPath(format!(
                "`#{n}` names the {} child of this group, and a new group has no position yet",
                ordinal(*n)
            ))),
            PathSeg::Match(name, want) => {
                let have = self.matches_named(name);
                if have + 1 == *want {
                    // The group this segment names does not exist yet: make it.
                    self.push_group(PropGroup::new(name.clone(), name.clone()));
                    return match self.nodes.last_mut() {
                        Some(PropNode::Group(g)) => Ok(g),
                        _ => Err(CoreError::BadPath("the group was created but is not a group".to_string())),
                    };
                }
                if have < *want {
                    return Err(CoreError::BadPath(format!(
                        "`{}` names the {} `{name}`, and this group holds {have}",
                        segment_text(name, *want),
                        ordinal(*want)
                    )));
                }
                // It exists. It has to be a group: a property is a leaf.
                let idx = index_of_seg(&self.nodes, seg)
                    .ok_or_else(|| CoreError::BadPath(format!("`{name}` is not here")))?;
                match self.nodes.get_mut(idx) {
                    Some(PropNode::Group(g)) => Ok(g),
                    Some(PropNode::Prop(_)) => {
                        Err(CoreError::BadPath(format!("`{name}` is a property, not a group")))
                    }
                    None => Err(CoreError::BadPath(format!("`{name}` is not here"))),
                }
            }
        }
    }

    /// How many children here are named `name`.
    ///
    /// The same rule the resolver uses — case-insensitively, and counting a
    /// group's id and a property's label alike — so "how many are there" and
    /// "which one is the *n*th" cannot disagree. §85.
    fn matches_named(&self, name: &str) -> usize {
        self.nodes.iter().filter(|n| node_matches(n, name)).count()
    }
}

/// How a segment is spelled, the way [`PropPath`] spells it.
fn segment_text(name: &str, want: usize) -> String {
    if want == 1 { name.to_string() } else { format!("{name}#{want}") }
}

/// A number as an ordinal, the way a person writes it: 1st, 2nd, 3rd, 4th.
fn ordinal(n: usize) -> String {
    let suffix = match n % 100 {
        11..=13 => "th",
        _ => match n % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        },
    };
    format!("{n}{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use craftkit_core::{FrameRate, Tick, Value};

    const FPS: FrameRate = FrameRate(24);

    fn tree() -> PropGroup {
        PropGroup::new("root", "root")
            .with_group(
                PropGroup::new("trigger", "Trigger")
                    .with_prop(Property::new(1, "enabled", Value::Bool(true)))
                    .with_prop(Property::new(2, "lead", Value::num("t", 1.5).unwrap())),
            )
            .with_group(
                PropGroup::new("output", "Output")
                    .with_prop(Property::new(3, "amount", Value::num("t", 0.5).unwrap()))
                    .with_prop(Property::new(4, "tint", Value::Color(craftkit_core::Color::rgb(1, 2, 3)))),
            )
    }

    fn get(g: &PropGroup, path: &str) -> Value {
        let p = PropPath::parse(path).expect("test path");
        g.find(&p).expect("found").value_at(Tick::ZERO)
    }

    /// A tree of `groups` nested groups with one property at the bottom.
    fn nest(groups: usize) -> PropGroup {
        let mut root = PropGroup::new("root", "root");
        let mut current = &mut root;
        for i in 0..groups {
            current.push_group(PropGroup::new(format!("g{i}"), format!("g{i}")));
            current = match current.nodes.last_mut() {
                Some(PropNode::Group(g)) => g,
                _ => panic!("the group was pushed"),
            };
        }
        current.push(Property::new(1, "leaf", Value::num("t", 1.0).unwrap()));
        root
    }

    #[test]
    fn a_leaf_is_found_by_label() {
        assert_eq!(get(&tree(), "trigger/lead"), Value::Num(1.5));
        assert_eq!(get(&tree(), "output/tint"), Value::Color(craftkit_core::Color::rgb(1, 2, 3)));
    }

    #[test]
    fn label_matching_is_case_insensitive() {
        assert_eq!(get(&tree(), "TRIGGER/Lead"), Value::Num(1.5));
    }

    #[test]
    fn a_missing_path_is_none_not_a_panic() {
        for bad in ["nope", "trigger/nope", "trigger", "output/amount/deeper", "#9", "@999"] {
            let p = PropPath::parse(bad).expect("parses");
            assert!(tree().find(&p).is_none(), "{bad} should not resolve");
        }
    }

    #[test]
    fn a_uid_resolves_from_anywhere_in_the_tree() {
        assert_eq!(get(&tree(), "@3"), Value::Num(0.5));
        assert_eq!(get(&tree(), "trigger/@2"), Value::Num(1.5));
    }

    #[test]
    fn index_segments_are_one_based_and_relative_to_their_parent() {
        assert_eq!(get(&tree(), "#1/enabled"), Value::Bool(true));
        assert_eq!(get(&tree(), "#2/amount"), Value::Num(0.5));
    }

    #[test]
    fn duplicate_labels_are_disambiguated_by_position() {
        let g = PropGroup::new("r", "r")
            .with_prop(Property::new(1, "x", Value::num("t", 1.0).unwrap()))
            .with_prop(Property::new(2, "x", Value::num("t", 2.0).unwrap()));
        assert_eq!(get(&g, "x#1"), Value::Num(1.0));
        assert_eq!(get(&g, "x#2"), Value::Num(2.0));
        assert_eq!(get(&g, "#1"), Value::Num(1.0));
    }

    #[test]
    fn find_mut_reaches_the_same_property_as_find() {
        let mut g = tree();
        let p = PropPath::parse("output/amount").unwrap();
        g.find_mut(&p).unwrap().value = Value::num("t", 9.0).unwrap();
        assert_eq!(get(&g, "output/amount"), Value::Num(9.0));
    }

    #[test]
    fn find_mut_reports_none_for_a_group() {
        let mut g = tree();
        let p = PropPath::parse("output").unwrap();
        assert!(g.find_mut(&p).is_none(), "a group is not a property");
    }

    /// The listing and the parser must agree about how deep a property tree
    /// may be.
    ///
    /// They did not, and a property was lost in the gap. The listing stopped
    /// one group too late — publishing a path of `MAX_PATH_SEGMENTS + 1`
    /// segments, which the parser refuses — and the group after that was
    /// dropped without a word, while `validate` called the tree good.
    /// Measured: a tree of sixteen groups published a path the parser would
    /// not accept, and a tree of seventeen groups published **no paths at
    /// all** and still validated. §85.
    #[test]
    fn every_property_the_tree_holds_is_named_by_a_path_the_parser_accepts() {
        // The deepest tree a path can name: fifteen groups, and a property at
        // the bottom of them — a path of sixteen segments, the most the
        // parser accepts.
        for groups in 1..limits::MAX_PATH_SEGMENTS {
            let g = nest(groups);
            assert!(g.validate().is_ok(), "{groups} group(s) is a legal tree");
            let flat = g.flatten();
            assert_eq!(flat.len(), g.count_props(), "{groups} group(s): the listing dropped a property");
            for (path, prop) in &flat {
                let text = path.to_string();
                assert!(
                    PropPath::parse(&text).is_some(),
                    "{groups} group(s): the listing published `{text}`, which the parser refuses"
                );
                assert_eq!(
                    g.find(path).map(|f| f.id),
                    Some(prop.id),
                    "{groups} group(s): `{text}` does not resolve to the property it names"
                );
            }
        }
        // And the deepest legal tree uses every segment the parser has.
        let deepest = nest(limits::MAX_PATH_SEGMENTS - 1);
        let text = deepest.flatten()[0].0.to_string();
        assert_eq!(text.split('/').count(), limits::MAX_PATH_SEGMENTS, "{text}");
    }

    /// A tree deeper than a path can name is **refused**, not truncated.
    ///
    /// The alternative is a document that holds a property nothing can see:
    /// the file keeps it, the UI does not list it, and the evaluator does not
    /// read it. §85.
    #[test]
    fn a_tree_deeper_than_a_path_is_refused_rather_than_truncated() {
        let g = nest(limits::MAX_PATH_SEGMENTS);
        let err = g.validate().expect_err("a tree deeper than a path must be refused");
        assert!(err.to_string().contains("property depth"), "{err}");
    }

    /// A path must name the property it is published for.
    ///
    /// The listing counted name occurrences exactly while the resolver counted
    /// them case-insensitively, so two siblings called `Amount` and `amount`
    /// were both published as a first: `amount` resolved to the *first* one,
    /// and the second property had no path of its own. §85.
    #[test]
    fn a_path_names_the_property_the_listing_published_it_for() {
        let g = PropGroup::new("r", "r")
            .with_prop(Property::new(1, "Amount", Value::num("t", 1.0).unwrap()))
            .with_prop(Property::new(2, "amount", Value::num("t", 2.0).unwrap()));
        let flat = g.flatten();
        assert_eq!(flat.len(), 2, "both properties are listed");
        let paths: Vec<String> = flat.iter().map(|(p, _)| p.to_string()).collect();
        assert_eq!(paths, vec!["Amount".to_string(), "amount#2".to_string()], "{paths:?}");
        for (path, prop) in &flat {
            assert_eq!(
                g.find(path).map(|f| f.id),
                Some(prop.id),
                "`{}` resolved to the wrong property",
                path
            );
        }
    }

    /// The read side and the write side must resolve a uid the same way.
    ///
    /// `find` looked for a uid *anywhere below* the group it was addressing
    /// and `find_mut` looked only among that group's direct children, so a
    /// nested property could be read by `@5` and not written by it. The engine
    /// writes, so `prop.set @5` answered "no such property" by inventing
    /// another one. §85.
    #[test]
    fn the_read_side_and_the_write_side_resolve_a_uid_the_same_way() {
        let mut g = PropGroup::new("r", "r").with_group(PropGroup::new("a", "a").with_prop(Property::new(
            7,
            "b",
            Value::num("t", 1.0).unwrap(),
        )));
        let by_uid = PropPath::parse("@7").expect("parses");
        assert_eq!(g.find(&by_uid).map(|p| p.id), Some(7), "the read side finds it");
        assert_eq!(g.find_mut(&by_uid).map(|p| p.id), Some(7), "the write side finds it too");
    }

    /// A path that names something that cannot exist yet is refused.
    ///
    /// An id names a property that already exists, and a position names a
    /// child that already exists; neither can be the name of a new property.
    /// The engine used to invent one — `@5` made a property labelled `5` — and
    /// the property it made could not be read back by the path that made it.
    /// §85.
    #[test]
    fn a_path_that_names_an_id_or_a_position_cannot_create_anything() {
        let mut g = PropGroup::new("r", "r");
        let by_uid = PropPath::parse("@5").expect("parses");
        let err = g.insert(&by_uid, 1, Value::num("t", 1.0).unwrap()).expect_err("refused");
        assert!(err.to_string().contains("id"), "{err}");

        let by_index = PropPath::parse("#2").expect("parses");
        let err = g.insert(&by_index, 1, Value::num("t", 1.0).unwrap()).expect_err("refused");
        assert!(err.to_string().contains("position"), "{err}");

        assert!(g.is_empty(), "nothing was created: {g:?}");
    }

    /// A property created by the model is readable by the path that made it.
    #[test]
    fn a_created_property_is_readable_by_the_path_that_created_it() {
        let mut g = PropGroup::new("r", "r");
        let path = PropPath::parse("a/b/c").expect("parses");
        g.insert(&path, 1, Value::num("t", 3.0).unwrap()).expect("insert");
        assert_eq!(g.find(&path).map(|p| p.value.clone()), Some(Value::Num(3.0)));
        // And it is in the listing, under that path.
        let listed = g.flatten();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].0.to_string(), "a/b/c");
    }

    /// A property the tree already holds is not created twice.
    #[test]
    fn creating_a_property_that_is_already_there_is_refused() {
        let mut g = PropGroup::new("r", "r").with_prop(Property::new(1, "a", Value::Num(1.0)));
        let path = PropPath::parse("a").expect("parses");
        let err = g.insert(&path, 2, Value::Num(2.0)).expect_err("refused");
        assert!(err.to_string().contains("already"), "{err}");
        assert_eq!(g.count_props(), 1);
    }

    /// A label the grammar cannot write is refused.
    ///
    /// A file can carry one: it is deserialized straight into the model, so
    /// nothing checked it on the way in. The tree used to accept it, and the
    /// listing published a path that meant something else — measured: a file
    /// with the label `a/b` loaded, published the path `a/b`, and
    /// `prop.set { path: "a/b" }` **created a second property** instead of
    /// changing the one on screen. §88.
    #[test]
    fn a_label_the_grammar_cannot_write_is_refused() {
        for label in ["a/b", "@5", "#2"] {
            let g = PropGroup::new("r", "r").with_prop(Property::new(1, label, Value::Num(1.0)));
            let err = g.validate().expect_err("refused");
            assert!(err.to_string().contains("cannot be written as a path"), "{label}: {err}");
        }
    }

    /// A group id the grammar cannot write is refused, for the same reason.
    #[test]
    fn a_group_id_the_grammar_cannot_write_is_refused() {
        let g = PropGroup::new("r", "r").with_group(PropGroup::new("a/b", "a/b").with_prop(Property::new(
            1,
            "leaf",
            Value::Num(1.0),
        )));
        let err = g.validate().expect_err("refused");
        assert!(err.to_string().contains("cannot be written as a path"), "{err}");
    }

    /// Every property the tree holds is named by a path that names **it**.
    ///
    /// The listing publishes a path; the path has to be one the grammar can
    /// read, and it has to read as *this* property. A name the grammar cannot
    /// write plainly is published with an explicit occurrence, so the path
    /// still names the property it was published for. §88.
    #[test]
    fn every_property_is_named_by_a_path_that_names_it() {
        let g = PropGroup::new("r", "r")
            .with_prop(Property::new(1, "C#", Value::Num(1.0)))
            .with_prop(Property::new(2, "a#b", Value::Num(2.0)))
            .with_prop(Property::new(3, "amount", Value::Num(3.0)))
            .with_prop(Property::new(4, "amount", Value::Num(4.0)));
        let flat = g.flatten();
        assert_eq!(flat.len(), 4);
        for (path, prop) in &flat {
            let text = path.to_string();
            let parsed = PropPath::parse(&text).unwrap_or_else(|| panic!("{text} parses"));
            assert_eq!(g.find(&parsed).map(|f| f.id), Some(prop.id), "{text}");
        }
    }

    #[test]
    fn validate_accepts_a_good_tree_and_rejects_duplicate_ids() {
        assert!(tree().validate().is_ok());
        let mut g = tree();
        g.nodes.push(PropNode::Prop(Property::new(1, "dup", Value::Bool(false))));
        assert!(g.validate().is_err(), "duplicate property ids must be refused");
    }

    /// A group id may repeat at a different depth.
    ///
    /// The two groups are not competing for the same name space: `trigger/x`
    /// and `trigger/trigger/y` are different paths, and `flatten` counts the
    /// occurrences of a name **per level** for exactly that reason. The rule
    /// that forbade it was stricter than its own reason, and a command could
    /// cross it — so the engine could write a file its reader refused. §50.
    #[test]
    fn a_group_id_may_repeat_at_a_different_depth() {
        let g = PropGroup::new("root", "root")
            .with_group(PropGroup::new("trigger", "Trigger").with_prop(Property::new(
                1,
                "lead",
                Value::num("t", 1.0).unwrap(),
            )))
            .with_group(PropGroup::new("a", "a").with_group(
                PropGroup::new("trigger", "Trigger").with_prop(Property::new(
                    2,
                    "lead",
                    Value::num("t", 2.0).unwrap(),
                )),
            ));
        assert!(g.validate().is_ok(), "the same id at two depths is not ambiguous: {g:?}");
        // And both are still reachable by their own path.
        assert_eq!(get(&g, "trigger/lead"), Value::Num(1.0));
        assert_eq!(get(&g, "a/trigger/lead"), Value::Num(2.0));
    }

    /// Two siblings with the same id *are* ambiguous, and the comparison is
    /// the resolver's: the resolver matches a group id case-insensitively, so
    /// `Trigger` and `trigger` under one parent would always resolve to the
    /// first of them.
    #[test]
    fn two_siblings_with_the_same_id_are_refused() {
        let exact = PropGroup::new("root", "root")
            .with_group(PropGroup::new("trigger", "Trigger"))
            .with_group(PropGroup::new("trigger", "Trigger again"));
        assert!(exact.validate().is_err(), "two siblings with one id must be refused");

        let cased = PropGroup::new("root", "root")
            .with_group(PropGroup::new("Trigger", "Trigger"))
            .with_group(PropGroup::new("trigger", "trigger"));
        assert!(cased.validate().is_err(), "the resolver matches ids case-insensitively");
    }

    #[test]
    fn validate_rejects_a_duplicate_group_id() {
        let mut g = PropGroup::new("a", "a");
        g.push_group(PropGroup::new("x", "X"));
        g.push_group(PropGroup::new("x", "X again"));
        assert!(g.validate().is_err());
    }

    #[test]
    fn validate_rejects_a_too_deep_tree() {
        let mut g = PropGroup::new("d0", "d0");
        for i in 1..(limits::MAX_DEPTH + 2) {
            g = PropGroup::new(format!("d{i}"), format!("d{i}")).with_group(g);
        }
        assert!(g.validate().is_err());
    }

    #[test]
    fn flatten_produces_paths_that_resolve_back() {
        let g = tree();
        let flat = g.flatten();
        assert!(flat.len() >= 4);
        for (path, _) in &flat {
            assert!(g.find(path).is_some(), "flattened path {path} does not resolve");
        }
    }

    #[test]
    fn an_animated_property_wins_over_its_static_value() {
        let mut p = Property::new(1, "amount", Value::num("t", 0.0).unwrap());
        let mut keys = crate::keyframe::Keyframes::new();
        keys.set(
            crate::keyframe::Key::new(
                Tick::ZERO,
                Value::num("t", 0.0).unwrap(),
                crate::keyframe::Easing::Linear,
            )
            .unwrap(),
        )
        .unwrap();
        keys.set(
            crate::keyframe::Key::new(
                Tick::from_frames_i64(10, FPS),
                Value::num("t", 1.0).unwrap(),
                crate::keyframe::Easing::Linear,
            )
            .unwrap(),
        )
        .unwrap();
        p.keys = keys;
        assert!(p.is_animated());
        assert_eq!(p.value_at(Tick::from_frames_i64(10, FPS)), Value::Num(1.0));
        // Past the last key it holds, so the property is defined everywhere.
        assert_eq!(p.value_at(Tick::from_frames_i64(20, FPS)), Value::Num(1.0));
    }

    #[test]
    fn a_locked_property_is_marked_so_the_ui_and_the_engine_agree() {
        let p = Property::new(1, "x", Value::num("t", 0.0).unwrap()).locked();
        assert!(p.locked);
    }

    #[test]
    fn flatten_produces_paths_that_are_written_the_way_a_person_would_write_them() {
        let mut group = PropGroup::new("props", "Properties");
        group.push(Property::new(1, "amount", Value::Num(1.0)));
        group.push_group(PropGroup::new("trigger", "Trigger").with_prop(Property::new(
            2,
            "lead",
            Value::Num(0.0),
        )));

        let paths: Vec<String> = group.flatten().into_iter().map(|(p, _)| p.to_string()).collect();
        assert_eq!(paths, vec!["amount".to_string(), "trigger/lead".to_string()]);
        // And every one of them resolves back to the property it names.
        for (path, prop) in group.flatten() {
            assert_eq!(group.find(&path).map(|f| f.id), Some(prop.id), "{path} did not resolve back");
        }
    }

    #[test]
    fn a_duplicated_label_gets_a_numbered_path_that_still_resolves() {
        let mut group = PropGroup::new("props", "Properties");
        group.push(Property::new(1, "amount", Value::Num(1.0)));
        group.push(Property::new(2, "amount", Value::Num(2.0)));

        let paths: Vec<String> = group.flatten().into_iter().map(|(p, _)| p.to_string()).collect();
        assert_eq!(paths, vec!["amount".to_string(), "amount#2".to_string()]);
        // The first is `amount`, the second is `amount#2`, and neither is
        // ambiguous — which a bare position path would have been.
        assert_eq!(group.find(&PropPath::parse("amount").expect("parses")).map(|p| p.id), Some(1));
        assert_eq!(group.find(&PropPath::parse("amount#2").expect("parses")).map(|p| p.id), Some(2));
    }

    #[test]
    fn a_nested_group_may_repeat_its_parents_name() {
        // Occurrence counting is per level, so a group called `trigger` inside a
        // group called `trigger` gets a plain path rather than an unresolvable
        // `#2`.
        let mut group = PropGroup::new("props", "Properties");
        let mut nested = PropGroup::new("trigger", "Trigger");
        nested.push_group(PropGroup::new("trigger", "Inner").with_prop(Property::new(
            1,
            "x",
            Value::Num(1.0),
        )));
        group.push_group(nested);
        let paths: Vec<String> = group.flatten().into_iter().map(|(p, _)| p.to_string()).collect();
        assert_eq!(paths, vec!["trigger/trigger/x".to_string()]);
        for (path, _) in group.flatten() {
            assert!(group.find(&path).is_some(), "{path} did not resolve back");
        }
    }

    /// The estimate has to count the property's own value.
    ///
    /// It did not, and the budget was measured with it: a document whose
    /// properties held megabytes of text was estimated at kilobytes, so the
    /// document budget and the undo budget both said the document was tiny
    /// while it held a gigabyte. §73.
    #[test]
    fn the_size_estimate_counts_the_property_value() {
        let small = Property::new(1, "label", Value::Text("x".into()));
        let big = Property::new(2, "label", Value::Text("x".repeat(64 * 1024)));
        let small_bytes = small.estimated_bytes();
        let big_bytes = big.estimated_bytes();
        let text_bytes = big.value.estimated_bytes();
        assert!(
            big_bytes >= text_bytes,
            "a property holding {text_bytes} bytes of value was estimated at {big_bytes}"
        );
        assert!(big_bytes > small_bytes + text_bytes - 1024, "{big_bytes} vs {small_bytes}");
    }

    /// A value the constructors would refuse is refused when it is read.
    ///
    /// A file is deserialized straight into the model, so the constructors
    /// never run — and the invariants they keep are the ones the evaluator
    /// relies on. §73.
    #[test]
    fn a_value_the_constructors_would_refuse_is_refused() {
        let mut group = PropGroup::new("props", "Properties");

        // A number past the limit: the constructors refuse it.
        group.push(Property::new(1, "amount", Value::Num(1e300)));
        let err = group.validate().expect_err("a number past the limit");
        assert!(err.to_string().contains("amount"), "{err}");

        // A text past the limit.
        let mut group = PropGroup::new("props", "Properties");
        group.push(Property::new(1, "label", Value::Text("x".repeat(70_000))));
        assert!(group.validate().is_err(), "a text past the limit");

        // A list past the limit.
        let mut group = PropGroup::new("props", "Properties");
        group.push(Property::new(1, "list", Value::List(vec![Value::Num(1.0); 5_000])));
        assert!(group.validate().is_err(), "a list past the limit");

        // And the good ones still pass.
        let mut group = PropGroup::new("props", "Properties");
        group.push(Property::new(1, "amount", Value::Num(1.0)));
        group.push(Property::new(2, "label", Value::Text("x".into())));
        assert!(group.validate().is_ok());
    }
}
