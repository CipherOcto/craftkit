//! Addressing anything inside a document.
//!
//! # The grammar
//!
//! A path is a `/`-separated list of segments. Each segment is one of:
//!
//! | Segment | Means |
//! |---|---|
//! | `@<id>` | the property with that stable id, wherever it sits |
//! | `#<n>` | the n-th child, **1-based** |
//! | `name#<n>` | the n-th child whose group id or label matches (case-insensitive) |
//! | `name` | shorthand for `name#1` |
//!
//! Examples:
//!
//! ```text
//! position          the first property called `position`
//! transform/x       the `x` property inside the `transform` group
//! effects/#2/amount the second effect's `amount`
//! @1f4a             the property whose id is 0x1f4a, wherever it is
//! ```
//!
//! # What a path may name
//!
//! A path names things that **exist**. An id names a property, and a position
//! names a child — neither can be the name of something that is not there
//! yet. So a path that *creates* a property may only spell names:
//!
//! - `prop.set { path: "trigger/lead" }` creates the group `trigger` if it is
//!   not there, and the property `lead` inside it.
//! - `prop.set { path: "@1f4a" }` **changes** the property with that id. If
//!   there is no such property, it is refused — ids are the model's to
//!   assign, not the caller's.
//! - `prop.set { path: "effects/#2/amount" }` changes the `amount` of the
//!   second child of `effects`. If there is no second child, it is refused —
//!   a new property has no position yet.
//!
//! The engine used to read every segment as a literal name and build the
//! tree from that: `effects/#2/amount` made a group whose id was literally
//! `#2`, and `@1f4a` made a property labelled `1f4a`. Neither could be read
//! back by the path that created it. §85.
//!
//! # What a name may be
//!
//! A **name** — a property's label or a group's id — becomes a path segment.
//! So a name the grammar reads as something else is a name the grammar
//! cannot write:
//!
//! | Name | The grammar reads it as | So it cannot be a name |
//! |---|---|---|
//! | `a/b` | two segments | `/` separates segments |
//! | `@5` | an id | `@` marks an id |
//! | `#2` | a position | `#` marks a position |
//!
//! The tree refuses such a name rather than publish a path that lies.
//! Measured: a file with the label `a/b` used to load, the listing published
//! the path `a/b`, and `prop.set { path: "a/b" }` **created a second
//! property** instead of changing the one on screen. §88.
//!
//! A name the grammar *can* write with an explicit occurrence is fine:
//! `a#b` and `C#` are published as `a#b#1` and `C##1`, which read back as
//! exactly those names — the grammar's own way of saying "this name, the
//! first one".
//!
//! # What it deliberately does not do
//!
//! It does not split on `.`. An earlier design did, and that is precisely why
//! this one does not: an effect instance's group id is its effect id
//! (`fx.blur.gaussian`), which contains dots, so a dot-splitting parser could
//! never name it. Instances are addressed positionally instead — `effects/#2` —
//! and the command that creates one returns the path it created.
//!
//! # Stability
//!
//! A path is **not stable across a reorder**. An id is. Commands that a script
//! will call repeatedly should record the id; commands a person types should use
//! the path. `PropPath` says which it is.
//!
//! # How long a path may be
//!
//! At most [`limits::MAX_PATH_SEGMENTS`] segments, and the property tree is
//! bounded by the same number: a property deeper than a path can name is a
//! property nothing can reach, so the tree is refused rather than quietly
//! truncated. §85.
//!
//! And each segment is a **name**, so it is at most [`limits::MAX_NAME`]
//! bytes — in every spelling. The length rule used to be only in the plain
//! one, so the grammar refused a name of 256 bytes and accepted the same name
//! written with an explicit occurrence, `name#1`. A name the model refuses is
//! a name no path may spell, and the spelling that skipped the rule was the
//! one the listing used for a long group id. §97.
//!
//! So the longest path the grammar can express is
//! [`limits::MAX_PATH_BYTES`] bytes, and that is the number the commands
//! carry: a path the tree publishes is a path a command accepts.
//!
//! That sentence was false once, and the number was the reason. §253: the
//! budget counted a segment as a name and one digit, but the tree's own
//! spelling of a name the grammar cannot write plainly is `name#1` — two
//! bytes more — so the longest path the tree could publish was 4 127 bytes
//! and every command refused it. The budget is now the length of the longest
//! path the **grammar** can express, and the grammar's own spelling is
//! bounded by the same rule the position spelling has always had.

use crate::limits;
use craftkit_core::NodeId;
use serde::{Deserialize, Serialize};
use std::fmt;

/// One segment of a [`PropPath`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "k", content = "v", rename_all = "snake_case")]
pub enum PathSeg {
    /// A stable property id, hexadecimal.
    Uid(u64),
    /// The n-th child, 1-based.
    Index(usize),
    /// The n-th child with this group id or label, 1-based.
    Match(String, usize),
}

/// A parsed property path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PropPath(Vec<PathSeg>);

impl PropPath {
    /// Parse a path. Returns `None` for anything unusable, so a bad path is an
    /// ordinary error rather than a panic — the caller turns it into a message
    /// naming the command and the path.
    ///
    /// A path longer than [`limits::MAX_PATH_SEGMENTS`] is refused: it would
    /// name a property deeper than the tree may be, and a property that deep
    /// is one nothing can reach. §85.
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('/').filter(|p| !p.is_empty()).collect();
        if parts.is_empty() || parts.len() > limits::MAX_PATH_SEGMENTS {
            return None;
        }
        let mut segs = Vec::with_capacity(parts.len());
        for part in parts {
            segs.push(parse_seg(part)?);
        }
        Some(Self(segs))
    }

    /// Build a path from segments. Used by `flatten`, which knows the
    /// position of every node it walked.
    pub fn from_segs(segs: Vec<PathSeg>) -> Self {
        Self(segs)
    }

    pub fn segs(&self) -> &[PathSeg] {
        &self.0
    }

    pub fn is_stable(&self) -> bool {
        // A path is stable only if every segment is an id: a single positional
        // segment can be renamed out from under it.
        !self.0.is_empty() && self.0.iter().all(|s| matches!(s, PathSeg::Uid(_)))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

fn parse_seg(part: &str) -> Option<PathSeg> {
    if let Some(hex) = part.strip_prefix('@') {
        // `from_str_radix` on a bounded length, so a path cannot be a long hex
        // digit string that is slow to parse.
        if hex.is_empty() || hex.len() > 16 {
            return None;
        }
        return u64::from_str_radix(hex, 16).ok().map(PathSeg::Uid);
    }
    if let Some(digits) = part.strip_prefix('#') {
        // `#0` is invalid: indexing is 1-based, and silently accepting 0 would
        // make a typo mean "the node before the first one".
        if digits.is_empty() || digits.len() > 6 {
            return None;
        }
        let n: usize = digits.parse().ok()?;
        return (n >= 1).then_some(PathSeg::Index(n));
    }
    // `name#n` — split on the LAST `#`, so a name may itself contain one.
    match part.rsplit_once('#') {
        Some((name, digits)) if !name.is_empty() => {
            // The same length rule as the plain spelling below. It used to be
            // missing here, so the grammar was inconsistent with itself: it
            // refused a name of 256 bytes and accepted the same name written
            // with an explicit occurrence — `name#1`. A name the model would
            // refuse is a name no path may spell. §97.
            if name.len() > limits::MAX_NAME {
                return None;
            }
            // The occurrence's digits are a number, and a number the grammar
            // writes is a number it can count: the same six digits the
            // position spelling has always had. Without this the grammar
            // accepted a segment twenty bytes longer than the budget assumed,
            // and a path of them was longer than any command would carry. §253.
            if digits.is_empty() || digits.len() > limits::MAX_PATH_INDEX_DIGITS {
                return None;
            }
            let n: usize = digits.parse().ok()?;
            (n >= 1).then(|| PathSeg::Match(name.to_string(), n))
        }
        _ => {
            if part.is_empty() || part.len() > limits::MAX_NAME {
                return None;
            }
            Some(PathSeg::Match(part.to_string(), 1))
        }
    }
}

/// Whether a name is written as it is.
///
/// A name is a property's label or a group's id, and a path segment is
/// written in the grammar: `@1f4a`, `#2`, `name#2`, `name`. So a name the
/// grammar reads as something else is a name the grammar cannot write: `a/b`
/// is two segments, `@5` is an id, `#2` is a position, and `C#` is nothing at
/// all.
///
/// The check is a **round trip**, not a list of characters: the name is parsed
/// as a segment and has to come back as the same plain name. A list of
/// forbidden characters would be a second copy of the grammar, and it would rot
/// the moment the grammar grows a form. §88.
pub fn is_written_as_it_is(name: &str) -> bool {
    matches!(parse_seg(name), Some(PathSeg::Match(parsed, 1)) if parsed == name)
}

/// The shortest spelling of the `n`th `name` that parses back to it.
///
/// A name that is written as it is needs no help. A name the grammar would
/// read as something else — `a#b`, `C#` — is written with an explicit
/// occurrence, which is how the grammar already says "this name, the first
/// one": `a#b#1`, `C#1`. §88.
pub fn segment_spelling(name: &str, n: usize) -> String {
    if n > 1 || !is_written_as_it_is(name) { format!("{name}#{n}") } else { name.to_string() }
}

/// Whether a name can be written as one path segment at all.
///
/// A name that cannot be written names a property **nothing can reach**: the
/// listing would publish a path that means something else, and a command
/// using that path would change — or create — something else. The tree refuses
/// such a name rather than publish a path that lies. §88.
pub fn is_writable(name: &str) -> bool {
    if name.contains('/') {
        return false;
    }
    if is_written_as_it_is(name) {
        return true;
    }
    // The occurrence spelling is the last way to write a name.
    matches!(parse_seg(&format!("{name}#1")), Some(PathSeg::Match(parsed, 1)) if parsed == name)
}

/// Why a name cannot be written as one path segment, if it cannot.
///
/// The reason is asked of the grammar rather than kept in a list beside it.
pub fn why_not_writable(name: &str) -> Option<String> {
    if is_writable(name) {
        return None;
    }
    if name.contains('/') {
        return Some("`/` separates the segments of a path".to_string());
    }
    if name.starts_with('@') {
        return Some("`@` marks an id, and an id is not a name".to_string());
    }
    if name.starts_with('#') {
        return Some("`#` marks a position, and a position is not a name".to_string());
    }
    Some("the grammar cannot write this name".to_string())
}

impl fmt::Display for PropPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for seg in &self.0 {
            if !first {
                write!(f, "/")?;
            }
            first = false;
            match seg {
                PathSeg::Uid(id) => write!(f, "@{id:x}")?,
                PathSeg::Index(n) => write!(f, "#{n}")?,
                // The spelling is the one that parses back: a name the grammar
                // cannot write plainly is written with an explicit occurrence.
                // §88.
                PathSeg::Match(name, n) => write!(f, "{}", segment_spelling(name, *n))?,
            }
        }
        Ok(())
    }
}

/// Address a **node** in the document. Nodes are addressed by index in the
/// ordered list, which is why a reorder is visible to a user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeRef {
    /// The n-th node, 0-based. Nodes are ordered, so this is meaningful.
    Index(usize),
    Id(NodeId),
}

impl NodeRef {
    pub fn parse(s: &str) -> Option<Self> {
        if let Some(rest) = s.strip_prefix('#') {
            let n: usize = rest.parse().ok()?;
            return Some(NodeRef::Index(n));
        }
        if let Some(rest) = s.strip_prefix("n") {
            let n: u64 = rest.parse().ok()?;
            return Some(NodeRef::Id(NodeId(n)));
        }
        None
    }
}

impl fmt::Display for NodeRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeRef::Index(n) => write!(f, "#{n}"),
            NodeRef::Id(id) => write!(f, "{id}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_segment_kind_parses() {
        assert_eq!(PropPath::parse("@1f4a").unwrap().segs(), [PathSeg::Uid(0x1f4a)]);
        assert_eq!(PropPath::parse("#2").unwrap().segs(), [PathSeg::Index(2)]);
        assert_eq!(PropPath::parse("position").unwrap().segs(), [PathSeg::Match("position".into(), 1)]);
        assert_eq!(PropPath::parse("position#3").unwrap().segs(), [PathSeg::Match("position".into(), 3)]);
        assert_eq!(
            PropPath::parse("transform/x").unwrap().segs(),
            [PathSeg::Match("transform".into(), 1), PathSeg::Match("x".into(), 1)]
        );
    }

    #[test]
    fn dots_are_legal_in_a_segment() {
        // This is the whole reason the parser does not split on `.`.
        let p = PropPath::parse("fx.blur.gaussian/amount").unwrap();
        assert_eq!(
            p.segs(),
            [PathSeg::Match("fx.blur.gaussian".into(), 1), PathSeg::Match("amount".into(), 1)]
        );
    }

    #[test]
    fn zero_is_rejected_because_indexing_is_one_based() {
        assert!(PropPath::parse("#0").is_none());
        assert!(PropPath::parse("name#0").is_none());
    }

    #[test]
    fn empty_and_oversized_paths_are_rejected() {
        assert!(PropPath::parse("").is_none());
        assert!(PropPath::parse("///").is_none());
        let deep: String =
            std::iter::repeat_n("a", limits::MAX_PATH_SEGMENTS + 1).collect::<Vec<_>>().join("/");
        assert!(PropPath::parse(&deep).is_none());
    }

    /// A name that is too long is too long in every spelling.
    ///
    /// §97: the grammar refused a name of 256 bytes and accepted the same name
    /// written with an explicit occurrence — `name#1` — because the length
    /// rule was only in the plain branch. A name the model refuses is a name
    /// no path may spell, and the spelling that skipped the rule was the one
    /// the listing used for a long group id.
    #[test]
    fn a_name_too_long_is_refused_in_either_spelling() {
        let long = "a".repeat(limits::MAX_NAME + 1);
        assert!(PropPath::parse(&long).is_none(), "the plain spelling must be refused");
        assert!(PropPath::parse(&format!("{long}#1")).is_none(), "the occurrence spelling too");

        let at_the_limit = "a".repeat(limits::MAX_NAME);
        assert!(PropPath::parse(&at_the_limit).is_some(), "a name at the limit is a name");
        assert!(PropPath::parse(&format!("{at_the_limit}#1")).is_some());
    }

    #[test]
    fn a_hostile_uid_segment_is_refused_rather_than_parsed() {
        assert!(PropPath::parse("@").is_none());
        assert!(PropPath::parse("@zzzz").is_none());
        assert!(PropPath::parse(&format!("@{}", "f".repeat(17))).is_none());
        assert!(PropPath::parse(&format!("#{}", "9".repeat(7))).is_none());
    }

    #[test]
    fn a_name_may_contain_a_hash_when_it_is_the_last_one() {
        let p = PropPath::parse("a#b#2").unwrap();
        assert_eq!(p.segs(), [PathSeg::Match("a#b".into(), 2)]);
    }

    #[test]
    fn display_round_trips_through_parse() {
        for s in ["@1f4a", "#2", "position", "position#3", "transform/x", "fx.blur.gaussian/amount"] {
            let p = PropPath::parse(s).unwrap();
            assert_eq!(p.to_string(), s, "display did not round trip for {s}");
            assert_eq!(PropPath::parse(&p.to_string()).unwrap(), p);
        }
    }

    #[test]
    fn stability_is_reported_honestly() {
        assert!(PropPath::parse("@1").unwrap().is_stable());
        assert!(!PropPath::parse("@1/position").unwrap().is_stable());
        assert!(!PropPath::parse("position").unwrap().is_stable());
    }

    #[test]
    fn node_refs_parse_both_spellings() {
        assert_eq!(NodeRef::parse("#0"), Some(NodeRef::Index(0)));
        assert_eq!(NodeRef::parse("n12"), Some(NodeRef::Id(NodeId(12))));
        assert_eq!(NodeRef::parse("garbage"), None);
        assert_eq!(NodeRef::Index(3).to_string(), "#3");
    }

    /// A name the grammar cannot write is refused.
    ///
    /// A name is a label or a group id, and it becomes a path segment. A name
    /// the grammar reads as something else — `a/b` is two segments, `@5` an
    /// id, `#2` a position — would be published as a path that means
    /// something else, and a command using it would change something else.
    /// §88.
    #[test]
    fn a_name_the_grammar_cannot_write_is_refused() {
        assert!(is_writable("amount"));
        assert!(is_writable("a@b"));
        assert!(is_writable("fx.blur.gaussian"));
        // These are written with an explicit occurrence, because the plain
        // spelling would be read as something else.
        assert!(is_writable("a#b"));
        assert!(is_writable("C#"));
        assert!(is_writable("amount#2"));

        assert!(!is_writable("a/b"));
        assert!(!is_writable("@5"));
        assert!(!is_writable("#2"));
        assert!(!is_writable(""));
    }

    /// The reason a name is refused is asked of the grammar, not kept beside
    /// it in a list. §88.
    #[test]
    fn the_reason_a_name_is_refused_comes_from_the_grammar() {
        assert_eq!(why_not_writable("amount"), None);
        assert!(why_not_writable("a/b").is_some_and(|why| why.contains("separates")));
        assert!(why_not_writable("@5").is_some_and(|why| why.contains("id")));
        assert!(why_not_writable("#2").is_some_and(|why| why.contains("position")));
    }

    /// A name is published as the shortest spelling that parses back.
    ///
    /// A name the grammar cannot write plainly is written with an explicit
    /// occurrence — which is how the grammar already says "this name, the
    /// first one". §88.
    #[test]
    fn a_name_is_published_as_the_shortest_spelling_that_parses_back() {
        assert_eq!(segment_spelling("amount", 1), "amount");
        assert_eq!(segment_spelling("amount", 2), "amount#2");
        assert_eq!(segment_spelling("a#b", 1), "a#b#1");
        assert_eq!(segment_spelling("C#", 1), "C##1");
        for (name, n) in [("amount", 1), ("amount", 2), ("a#b", 1), ("C#", 1), ("a#b", 2)] {
            let spelling = segment_spelling(name, n);
            let parsed = PropPath::parse(&spelling).unwrap_or_else(|| panic!("{spelling} parses"));
            assert_eq!(parsed.segs(), [PathSeg::Match(name.into(), n)], "{spelling}");
        }
    }

    /// The display of a path is the one that parses back — always.
    #[test]
    fn display_round_trips_through_parse_for_names_the_grammar_cannot_write_plainly() {
        for name in ["a#b", "C#", "amount#2"] {
            let p = PropPath::from_segs(vec![PathSeg::Match(name.to_string(), 1)]);
            let text = p.to_string();
            let parsed = PropPath::parse(&text).unwrap_or_else(|| panic!("{text} parses"));
            assert_eq!(parsed, p, "{text}");
        }
    }
}
