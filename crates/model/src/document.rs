//! The document.
//!
//! Plain data, no I/O, no behaviour beyond validation. That is not laziness —
//! it is what makes undo a snapshot, the file format a `serde_json` call, and
//! the evaluator a pure function of `(document, instant)`.
//!
//! # One id space
//!
//! Nodes and documents draw from a single monotonic [`IdCounter`]. An id is
//! therefore unique across the whole document, so a log line can name any object
//! without a kind prefix, and copying an id between two collections cannot
//! collide. `Document::validate` checks uniqueness.

use crate::limits;
use crate::property::{PropGroup, Property};
use craftkit_core::{IdCounter, NodeId, Tick};
use craftkit_foundation::{CoreError, Value};
use serde::{Deserialize, Serialize};

/// What a node does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    /// Reads a value from somewhere. A leaf of the graph.
    Source,
    /// Changes a value on the way through.
    Filter,
    /// Writes a value out.
    Sink,
    /// Holds an animation, with no other effect.
    Cue,
    /// A note. Ignored by the evaluator.
    Comment,
}

impl NodeKind {
    pub fn label(self) -> &'static str {
        match self {
            NodeKind::Source => "source",
            NodeKind::Filter => "filter",
            NodeKind::Sink => "sink",
            NodeKind::Cue => "cue",
            NodeKind::Comment => "comment",
        }
    }

    /// Whether the evaluator does anything with it.
    pub fn is_passive(self) -> bool {
        matches!(self, NodeKind::Comment)
    }

    /// Every kind, for the UI's add menu.
    pub fn all() -> &'static [NodeKind] {
        &[NodeKind::Source, NodeKind::Filter, NodeKind::Sink, NodeKind::Cue, NodeKind::Comment]
    }
}

/// One node in the plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub name: String,
    pub kind: NodeKind,
    /// The node's own property tree.
    pub props: PropGroup,
    /// Whether the evaluator skips it.
    pub muted: bool,
}

impl Node {
    pub fn new(id: NodeId, name: impl Into<String>, kind: NodeKind) -> Self {
        Self { id, name: name.into(), kind, props: PropGroup::new("props", "Properties"), muted: false }
    }

    /// Why a node's name cannot be used, if it cannot.    ///
    /// **One rule, in one place, next to the type that has it.** The
    /// constructor, the validator and the command all ask this, so they cannot
    /// disagree about what a name is.
    ///
    /// They did disagree, and the tree had three answers to one question.
    /// Measured: `node.add { name: "" }` was *accepted* by the command and
    /// then refused by the document validator — so the user's empty name came
    /// back as the tree's own unexpected failure, of the kind the tree
    /// documents as "worth a crash report and a bug". `node.add { name: " " }`
    /// was accepted outright, while `node.rename { name: " " }` refused the
    /// same name. §92.
    ///
    /// The length is counted in **bytes**, because that is the unit every
    /// other text limit in the tree is counted in — `Params::text`,
    /// `Value::check`, a property's label, a path segment. The engine had its
    /// own copy of the limit, counted in characters, which could never fire:
    /// the parameter had already been refused by the byte count.
    pub fn why_not_a_name(name: &str) -> Option<String> {
        if name.trim().is_empty() {
            return Some("a name must not be empty".to_string());
        }
        if name.len() > limits::MAX_NAME {
            return Some(format!("{} bytes, the limit is {}", name.len(), limits::MAX_NAME));
        }
        None
    }

    /// A node with the standard property set a user expects to be able to set.
    pub fn with_defaults(mut self, enabled: bool, amount: f64) -> Result<Self, CoreError> {
        self.set_defaults(enabled, amount)?;
        Ok(self)
    }

    /// Give the node a new name. Returns whether it changed.
    ///
    /// Writing the name a node already has leaves the document *identical*,
    /// and the engine cannot tell that from a command that forgot to write.
    /// So the write says whether it changed anything, and the command passes
    /// that on. §93.
    pub fn rename(&mut self, name: impl Into<String>) -> bool {
        let name = name.into();
        if self.name == name {
            return false;
        }
        self.name = name;
        true
    }

    /// Change what the node does. Returns whether it changed.
    pub fn set_kind(&mut self, kind: NodeKind) -> bool {
        if self.kind == kind {
            return false;
        }
        self.kind = kind;
        true
    }

    /// Mute or unmute the node. Returns whether it changed.
    pub fn set_muted(&mut self, muted: bool) -> bool {
        if self.muted == muted {
            return false;
        }
        self.muted = muted;
        true
    }

    /// The same, in place.
    ///
    /// Separate from [`Node::with_defaults`] because a command usually holds a
    /// `&mut Node` borrowed out of a document. Moving the node out to call the
    /// builder would need a clone of the whole property tree, which for a
    /// command that only wants to add four properties is absurd.
    pub fn set_defaults(&mut self, enabled: bool, amount: f64) -> Result<(), CoreError> {
        self.props.push(Property::new(1, "enabled", Value::Bool(enabled)));
        self.props.push(
            Property::new(2, "amount", Value::num("amount", amount)?)
                .with_hint(crate::property::PropHint::Slider { min: 0.0, max: 1.0 }),
        );
        self.props.push(Property::new(
            3,
            "tint",
            Value::Color(craftkit_foundation::Color::rgb(140, 170, 210)),
        ));
        self.props.push(Property::new(4, "label", Value::text("label", String::new())?));
        Ok(())
    }

    /// A cheap, conservative estimate of the memory this node holds.
    ///
    /// Never under-estimates: a budget a document can lie about is not a
    /// budget. §52.
    pub fn estimated_bytes(&self) -> usize {
        std::mem::size_of::<Node>() + self.name.capacity() + self.props.estimated_bytes()
    }
}

/// Document-level settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentMeta {
    pub name: String,
    pub fps: u32,
    /// The first instant in the document.
    pub start: Tick,
    /// One past the last instant.
    pub end: Tick,
    #[serde(default)]
    pub comment: String,
}

impl Default for DocumentMeta {
    fn default() -> Self {
        Self {
            name: "Untitled".to_string(),
            fps: 24,
            start: Tick::ZERO,
            end: Tick::from_seconds_f64(5.0),
            comment: String::new(),
        }
    }
}

impl DocumentMeta {
    pub fn frame_rate(&self) -> craftkit_core::FrameRate {
        craftkit_core::FrameRate(self.fps)
    }
    pub fn duration(&self) -> Tick {
        self.end.sub_ticks(self.start.raw())
    }

    /// The frames the design has: its first frame and its last instant.
    ///
    /// **The range, not the duration.** A design that starts one second into
    /// the timeline and ends five seconds in has frames 24..=120 at 24 fps,
    /// and its *duration* is 96 frames — a number that says how long the
    /// design is and nothing about where it is. A timeline built on the
    /// duration cannot show the design's first frame and cannot reach its
    /// last one. §115.
    ///
    /// The end is included because the engine's own rule for the playhead is
    /// the closed range `[start, end]`: `playback.seek` clamps to it, and the
    /// range here is the same rule rather than a second one.
    pub fn frame_range(&self) -> std::ops::RangeInclusive<i64> {
        let rate = self.frame_rate();
        let start = self.start.frames_i64(rate);
        // A range shorter than one frame has no last frame of its own; the
        // design still has a place to stand.
        let end = self.end.frames_i64(rate).max(start);
        start..=end
    }
}

/// A whole document: ordered nodes plus settings.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub meta: DocumentMeta,
    pub nodes: Vec<Node>,
    #[serde(skip)]
    next_id: IdCounter,
}

impl Document {
    pub fn new(name: impl Into<String>) -> Self {
        let mut d = Self {
            meta: DocumentMeta { name: name.into(), ..Default::default() },
            nodes: Vec::new(),
            next_id: IdCounter::new(1),
        };
        // A document with nothing in it is not a document: start with one node
        // so the UI, the evaluator and every example have something to show.
        if let Ok(id) = d.add_node("Start", NodeKind::Source) {
            // `with_defaults` only fails on a bad number, and 1.0 is not a bad
            // number — but the document must be usable even so, so an unexpected
            // failure leaves a node without properties rather than propagating
            // out of a constructor.
            if let Some(n) = d.get_mut(id)
                && let Ok(ready) = n.clone().with_defaults(true, 1.0)
            {
                *n = ready;
            }
        }
        d
    }

    pub fn empty(name: impl Into<String>) -> Self {
        Self {
            meta: DocumentMeta { name: name.into(), ..Default::default() },
            nodes: Vec::new(),
            next_id: IdCounter::new(1),
        }
    }

    /// Give the document a new name. Returns whether it changed.
    ///
    /// Writing the name the document already has leaves it *identical*, and
    /// the engine cannot tell that from a command that forgot to write. So
    /// the write says whether it changed anything. §93.
    pub fn set_name(&mut self, name: impl Into<String>) -> bool {
        let name = name.into();
        if self.meta.name == name {
            return false;
        }
        self.meta.name = name;
        true
    }

    /// Set the document's note. Returns whether it changed.
    ///
    /// The note is the paragraph that says why the design is the way it is.
    /// It used to have no setter at all: the field was in the model and in
    /// the file and there was no way to write it, so a note could only be
    /// made by editing the file by hand. §110.
    ///
    /// An empty note is a note that says nothing, and it is legal: it is how
    /// a note is cleared.
    pub fn set_comment(&mut self, comment: impl Into<String>) -> bool {
        let comment = comment.into();
        if self.meta.comment == comment {
            return false;
        }
        self.meta.comment = comment;
        true
    }

    /// Set the document's range and rate. Returns whether it changed.
    pub fn set_range(&mut self, start: Tick, end: Tick, fps: u32) -> bool {
        if self.meta.start == start && self.meta.end == end && self.meta.fps == fps {
            return false;
        }
        self.meta.start = start;
        self.meta.end = end;
        self.meta.fps = fps;
        true
    }

    /// Append a node and return it, so the caller can keep configuring it.
    ///
    /// Returns `Err` rather than a default value when the document is full: a
    /// document at its limit should refuse the edit, not silently drop it.
    pub fn add_node(&mut self, name: impl Into<String>, kind: NodeKind) -> Result<NodeId, CoreError> {
        if self.nodes.len() >= limits::MAX_NODES {
            return Err(CoreError::LimitReached { limit: "nodes per document" });
        }
        let name = name.into();
        // The name is checked here, where the rule lives, so a caller cannot
        // add a node the validator would refuse. §92.
        if let Some(why) = Node::why_not_a_name(&name) {
            return Err(CoreError::BadData(format!("the node name `{name}` cannot be used: {why}")));
        }
        // The id comes from the counter, and the counter refuses when it can no
        // longer promise an id that is not already in use. Two nodes that
        // share an id are a document the file format refuses, so a duplicate
        // here would not be a small wrong answer: the command would be
        // refused for a reason nobody could act on. §137.
        let Some(raw) = self.next_id.next() else {
            return Err(CoreError::BadData(format!(
                "this document's node ids have reached the ceiling of {}, so a new node cannot \
                 be given an id of its own",
                craftkit_core::IdCounter::MAX
            )));
        };
        let id = NodeId(raw);
        self.nodes.push(Node::new(id, name, kind));
        Ok(id)
    }

    pub fn remove(&mut self, id: NodeId) -> bool {
        let before = self.nodes.len();
        self.nodes.retain(|n| n.id != id);
        self.nodes.len() != before
    }

    pub fn index_of(&self, id: NodeId) -> Option<usize> {
        self.nodes.iter().position(|n| n.id == id)
    }

    pub fn get(&self, id: NodeId) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    /// Find a node by its name, first match wins.
    pub fn find_by_name(&self, name: &str) -> Option<&Node> {
        self.nodes.iter().find(|n| n.name.eq_ignore_ascii_case(name))
    }

    /// Move a node within the order. The timeline is the document's spine, so
    /// this is a first-class operation rather than a generic `reorder`.
    ///
    /// An index past the end is **refused**, not clamped. Clamping here was the
    /// original behaviour and it is wrong twice over: the caller is told the node
    /// moved to `99` when it moved to the end, and a script that computes an
    /// index has a bug that the clamp hides instead of reporting.
    /// Move a node to another position. Returns whether it changed.
    ///
    /// Moving a node to the position it is already at changes nothing, and
    /// the engine cannot tell that from a command that forgot to write. §93.
    pub fn move_to(&mut self, id: NodeId, index: usize) -> Result<bool, CoreError> {
        let from = self.index_of(id).ok_or(CoreError::LimitReached { limit: "unknown node" })?;
        let to = index.min(self.nodes.len().saturating_sub(1));
        if index > to {
            return Err(CoreError::OutOfRange {
                what: "index".to_string(),
                value: index.to_string(),
                min: "0".to_string(),
                max: to.to_string(),
            });
        }
        if from == to {
            return Ok(false);
        }
        let node = self.nodes.remove(from);
        self.nodes.insert(to, node);
        Ok(true)
    }

    /// The id the next added node will get, without taking it.
    pub fn peek_next_id(&self) -> NodeId {
        NodeId(self.next_id.peek())
    }

    /// Repoint the counter above every id in the document.
    ///
    /// Called on load so a hand-edited file cannot reuse an id. This is the
    /// single most important line of defence against a corrupt or hostile file:
    /// without it, `add_node` can hand out an id that is already in use and the
    /// next `get` returns the wrong node.
    pub fn reseat_id_counter(&mut self) {
        let mut c = IdCounter::new(1);
        for n in &self.nodes {
            c.observe(n.id.get());
        }
        self.next_id = c;
    }

    /// A cheap, conservative estimate of the memory this document holds.
    ///
    /// Never under-estimates: a budget a document can lie about is not a
    /// budget. This is what `MAX_DOCUMENT_BYTES` is measured against, and it
    /// is what the undo stack's budget is measured in. §52.
    pub fn estimated_bytes(&self) -> usize {
        std::mem::size_of::<Document>()
            + self.meta.name.capacity()
            + self.meta.comment.capacity()
            + self.nodes.capacity() * std::mem::size_of::<Node>()
            + self.nodes.iter().map(|n| n.estimated_bytes()).sum::<usize>()
    }

    /// Check every invariant that must hold before the document is trusted.
    pub fn validate(&self) -> Result<(), DocError> {
        // A frame rate has two rules: it must not be zero, because the ticks
        // per frame divide by it, and it must not be faster than the model
        // allows. The first lives in the core, next to the type; the second is
        // `MAX_FRAME_RATE` — the same number the `doc.range` command uses, so
        // a document can never hold a rate no command can set. §89.
        if craftkit_core::FrameRate::new(self.meta.fps).is_err() || self.meta.fps > limits::MAX_FRAME_RATE {
            return Err(DocError::BadMeta(format!(
                "frame rate {} is not usable: it must be between 1 and {}",
                self.meta.fps,
                limits::MAX_FRAME_RATE
            )));
        }
        if self.meta.start >= self.meta.end {
            return Err(DocError::BadMeta(format!(
                "the document is empty: start {} is not before end {}",
                self.meta.start, self.meta.end
            )));
        }
        // The note has a bound, so that it can be carried whole. A note with
        // no bound is a note nobody can read: it was in the file and in the
        // model and in nothing else. §110.
        if self.meta.comment.len() > limits::MAX_COMMENT_BYTES {
            return Err(DocError::BadMeta(format!(
                "the note is {} bytes long: it must be at most {}",
                self.meta.comment.len(),
                limits::MAX_COMMENT_BYTES
            )));
        }
        // The range has to be one a reply can carry.
        //
        // The file format is wider than the model: a bare integer in a file is
        // read straight into an `i128`, so a file can name an instant that no
        // JSON number holds. A document like that would make every reply that
        // mentions its playhead fail inside `json!` — where the `unwrap` is
        // invisible to the lint — so the reader refuses it instead of loading
        // it. §60.
        if self.meta.start < craftkit_core::Tick::MIN || self.meta.end > craftkit_core::Tick::MAX {
            return Err(DocError::BadMeta(format!(
                "the document's range is outside what a time can be: {}..{}",
                self.meta.start, self.meta.end
            )));
        }
        // The document has to fit in memory, not just in the file.
        //
        // Every other limit here is a count — nodes, properties, keyframes —
        // and the product of those counts was never written down. The engine
        // clones the whole document for every command and keeps 128 of them
        // for undo, so a document of 12 MiB cost 1.5 GiB. A byte budget is
        // the only limit that says what the memory actually is. §52.
        let bytes = self.estimated_bytes();
        if bytes > limits::MAX_DOCUMENT_BYTES {
            return Err(DocError::TooLarge { bytes, limit: limits::MAX_DOCUMENT_BYTES });
        }
        if self.nodes.len() > limits::MAX_NODES {
            return Err(DocError::LimitReached("nodes per document"));
        }
        let mut seen: Vec<NodeId> = self.nodes.iter().map(|n| n.id).collect();
        seen.sort_unstable();
        if seen.windows(2).any(|w| w[0] == w[1]) {
            return Err(DocError::DuplicateId("two nodes share an id"));
        }
        for n in &self.nodes {
            if let Some(why) = Node::why_not_a_name(&n.name) {
                return Err(DocError::BadMeta(format!("node {} has an unusable name: {why}", n.id)));
            }
            n.props.validate().map_err(|e| DocError::Invalid(format!("node {}: {e}", n.id)))?;
        }
        Ok(())
    }

    /// A short description of the document for the UI title bar.
    pub fn summary(&self) -> String {
        let anim =
            self.nodes.iter().filter(|n| n.props.flatten().iter().any(|(_, p)| p.is_animated())).count();
        format!("{} nodes · {anim} animated · {}s", self.nodes.len(), self.meta.duration().seconds_f64())
    }
}

/// Why a document is not usable.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DocError {
    #[error("bad metadata: {0}")]
    BadMeta(String),
    #[error("two objects share an id: {0}")]
    DuplicateId(&'static str),
    #[error("limit reached: {0}")]
    LimitReached(&'static str),
    #[error("invalid: {0}")]
    Invalid(String),
    /// The document is too big to hold.
    ///
    /// §52: the other limits here are counts, and a count does not say how
    /// much memory the thing counted holds.
    #[error("the document is {bytes} bytes, and the limit is {limit}")]
    TooLarge { bytes: usize, limit: usize },
}

impl craftkit_foundation::Snapshot for Document {
    /// **The document's own size.** §278: the undo stack asks its snapshots
    /// what they cost, and the document answers with the number it has
    /// always answered with.
    fn estimated_bytes(&self) -> usize {
        Document::estimated_bytes(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_document_is_valid_and_has_something_in_it() {
        let d = Document::new("test");
        assert!(d.validate().is_ok(), "{:?}", d.validate());
        assert_eq!(d.nodes.len(), 1);
    }

    /// The design's frames are its range, not its duration.
    ///
    /// §115: a design that starts one second in has frames 24..=120 at 24 fps
    /// and a duration of 96 frames. The two numbers are not the same, and a
    /// timeline built on the duration cannot show the design's first frame
    /// and cannot reach its last.
    #[test]
    fn the_designs_frames_are_its_range_and_not_its_duration() {
        let meta = DocumentMeta {
            start: craftkit_core::Tick::from_seconds_f64(1.0),
            end: craftkit_core::Tick::from_seconds_f64(5.0),
            fps: 24,
            ..DocumentMeta::default()
        };
        assert_eq!(*meta.frame_range().start(), 24, "the design's first frame");
        assert_eq!(*meta.frame_range().end(), 120, "the design's last frame");
        assert_eq!(meta.duration().frames_i64(meta.frame_rate()), 96, "the design's duration");
        // A design that starts at zero has the frames its duration says.
        let plain = DocumentMeta::default();
        assert_eq!(*plain.frame_range().start(), 0);
        assert_eq!(*plain.frame_range().end(), 120);
    }

    /// A document over the byte limit is refused.
    ///
    /// §52: every other limit here is a **count**, and the product of the
    /// counts was never written down — 10 000 nodes × 50 000 properties ×
    /// 10 000 keyframes is a number no machine has. The engine clones the
    /// whole document for every command and keeps 128 of them, so this is the
    /// limit that says what the memory is.
    #[test]
    fn a_document_over_the_byte_limit_is_refused() {
        let doc = over_the_byte_limit();
        let err = doc.validate().expect_err("refused");
        assert!(matches!(err, DocError::TooLarge { .. }), "{err}");
        assert!(err.to_string().contains("the limit is"), "{err}");
    }

    /// A range no reply could carry is refused.
    ///
    /// The file format is wider than the model: a bare integer in a file is
    /// read straight into an `i128`, so a file can name an instant that no
    /// JSON number holds. A document like that would make every reply that
    /// mentions its playhead fail inside `json!`. §60.
    ///
    /// The tick cannot be built with `from_raw` — that saturates to the
    /// model's bounds, which is the point — so it arrives the way it really
    /// arrives: by being read out of a file.
    #[test]
    fn a_document_whose_range_is_outside_what_a_time_can_be_is_refused() {
        let raw = r#"{"meta":{"name":"Too far out","fps":24,"start":18446744073709551616,"end":18446744073709551617},"nodes":[]}"#;
        let doc: Document = serde_json::from_str(raw).expect("the tick is read straight into an i128");
        assert_eq!(doc.meta.start.raw(), u64::MAX as i128 + 1, "the file really did carry it");
        let err = doc.validate().expect_err("refused");
        assert!(err.to_string().contains("outside what a time can be"), "{err}");

        // And one that is inside is accepted.
        let mut ok = Document::new("Fine");
        ok.meta.start = craftkit_core::Tick::ZERO;
        ok.meta.end = craftkit_core::Tick::from_raw(u64::MAX as i128);
        ok.validate().expect("a range inside the bounds is fine");
    }

    /// A note over the byte limit is refused, and one at the limit is not.
    ///
    /// §110: the note had **no** bound at all, and no way out either — it
    /// was in the file and in the model and in nothing else, so the only way
    /// to read a design's note was to open the file that held it. A bound is
    /// what makes the note carryable: the state can hand it over whole, and a
    /// file with a longer one is refused rather than half-kept.
    #[test]
    fn a_note_over_the_byte_limit_is_refused() {
        let mut doc = Document::new("With a note");
        doc.meta.comment = "n".repeat(limits::MAX_COMMENT_BYTES);
        doc.validate().expect("a note at the limit is a note");

        doc.meta.comment = "n".repeat(limits::MAX_COMMENT_BYTES + 1);
        let err = doc.validate().expect_err("refused");
        assert!(err.to_string().contains("the note is"), "{err}");
        assert!(err.to_string().contains(&limits::MAX_COMMENT_BYTES.to_string()), "{err}");

        // And the setter writes it, and says whether it changed anything.
        let mut doc = Document::new("With a note");
        assert!(doc.set_comment("why the design is the way it is"), "the note was written");
        assert_eq!(doc.meta.comment, "why the design is the way it is");
        assert!(!doc.set_comment("why the design is the way it is"), "the same note is not a change");
        assert!(doc.set_comment(""), "an empty note clears the note");
        assert!(doc.meta.comment.is_empty());
    }

    /// The estimate grows with the content, and a small document is nowhere
    /// near the limit.
    ///
    /// The estimate is what the budget is measured against, so it has to move
    /// when the document does.
    #[test]
    fn the_estimate_grows_with_the_document() {
        let small = Document::new("t");
        assert!(small.estimated_bytes() < limits::MAX_DOCUMENT_BYTES, "a new document is not big");
        let mut bigger = small.clone();
        let id = bigger.add_node("n", NodeKind::Source).expect("node");
        let mut prop = Property::new(1, "p", Value::Num(1.0));
        for t in 0..1000 {
            let _ = prop.keys.set(
                crate::keyframe::Key::new(
                    craftkit_core::Tick::from_raw(t),
                    Value::Num(t as f64),
                    crate::keyframe::Easing::Linear,
                )
                .expect("key"),
            );
        }
        bigger.get_mut(id).expect("node").props.push(prop);
        assert!(
            bigger.estimated_bytes() > small.estimated_bytes(),
            "a thousand keyframes did not move the estimate: {} vs {}",
            bigger.estimated_bytes(),
            small.estimated_bytes()
        );
    }

    /// A document that is over the byte limit, and still inside every count.
    ///
    /// One node, 60 properties, 10 000 keyframes on each: inside
    /// `MAX_PROPERTIES` and `MAX_KEYS`, and over `MAX_DOCUMENT_BYTES`.
    fn over_the_byte_limit() -> Document {
        let mut doc = Document::empty("Big");
        let id = doc.add_node("n", NodeKind::Source).expect("node");
        let mut group = PropGroup::new("props", "Properties");
        for p in 0..60u64 {
            let mut prop = Property::new(p + 1, format!("p{p}"), Value::Num(1.0));
            for t in 0..10_000i128 {
                let _ = prop.keys.set(
                    crate::keyframe::Key::new(
                        craftkit_core::Tick::from_raw(t),
                        Value::Num(t as f64),
                        crate::keyframe::Easing::Linear,
                    )
                    .expect("key"),
                );
            }
            group.push(prop);
        }
        doc.get_mut(id).expect("just added").props = group;
        doc
    }

    #[test]
    fn ids_are_unique_and_monotonic() {
        let mut d = Document::empty("t");
        let a = d.add_node("a", NodeKind::Source).unwrap();
        let b = d.add_node("b", NodeKind::Filter).unwrap();
        assert_ne!(a, b);
        assert!(b.get() > a.get());
        assert!(d.validate().is_ok());
    }

    #[test]
    fn reseating_the_counter_repairs_a_hand_edited_duplicate_id() {
        let mut d = Document::empty("t");
        let a = d.add_node("a", NodeKind::Source).unwrap();
        d.add_node("b", NodeKind::Filter).unwrap();
        // Simulate a corrupt file: give two nodes the same id.
        d.nodes[1].id = a;
        assert!(d.validate().is_err());
        d.reseat_id_counter();
        // The duplicate is still there — validate is what catches it — but the
        // next id is above everything, so a repair can finish the job.
        assert!(d.peek_next_id() > a);
    }

    #[test]
    fn add_node_refuses_at_the_limit_rather_than_truncating() {
        let mut d = Document::empty("t");
        for i in 0..limits::MAX_NODES {
            d.add_node(format!("n{i}"), NodeKind::Source).unwrap();
        }
        assert_eq!(d.nodes.len(), limits::MAX_NODES);
        assert!(d.add_node("one too many", NodeKind::Source).is_err());
    }

    /// A document whose ids have reached the ceiling refuses a new node.
    ///
    /// §137: the id counter used to *saturate*, which meant it handed out its
    /// last id for ever. A file carrying that id therefore made every later
    /// `node.add` mint a node that shared an id with one already there — and
    /// two nodes that share an id are a document the file format refuses, so
    /// the command failed with the tree's own unexpected error, for ever.
    #[test]
    fn a_document_whose_ids_reached_the_ceiling_refuses_a_new_node() {
        let mut d = Document::empty("t");
        // What a file from another machine can carry: one node, with the
        // last id the counter can reach.
        d.nodes.push(Node::new(NodeId(craftkit_core::IdCounter::MAX), "Only", NodeKind::Source));
        d.reseat_id_counter();

        let err = d.add_node("Second", NodeKind::Filter).expect_err("the ids are exhausted");
        assert!(err.to_string().contains("ceiling"), "{err}");
        assert_eq!(d.nodes.len(), 1, "no node was added");
        // And the document is still one the file format would accept.
        assert!(d.validate().is_ok(), "the document is still valid");
    }

    #[test]
    fn an_over_long_name_is_refused() {
        let mut d = Document::empty("t");
        let long = "x".repeat(limits::MAX_NAME + 1);
        assert!(d.add_node(long, NodeKind::Source).is_err());
    }

    #[test]
    fn move_to_reorders_and_refuses_an_index_past_the_end() {
        let mut d = Document::empty("t");
        let a = d.add_node("a", NodeKind::Source).unwrap();
        d.add_node("b", NodeKind::Filter).unwrap();
        let c = d.add_node("c", NodeKind::Sink).unwrap();
        d.move_to(c, 0).unwrap();
        assert_eq!(d.nodes[0].id, c);

        // An index past the end is refused, not clamped: a caller told "moved to
        // 9" when it moved to the end is a caller with a bug that never finds
        // out.
        let err = d.move_to(a, 9_999).expect_err("refused");
        assert!(err.to_string().contains("index"), "{err}");

        // The last valid index still works, and the document is unchanged by
        // the refusal.
        let before: Vec<NodeId> = d.nodes.iter().map(|n| n.id).collect();
        d.move_to(a, d.nodes.len() - 1).expect("the last index is valid");
        assert_eq!(d.nodes[d.nodes.len() - 1].id, a);
        assert_eq!(d.nodes.len(), before.len());

        assert!(d.move_to(NodeId(9_999), 0).is_err(), "an unknown node is refused");
    }

    #[test]
    fn an_empty_time_range_is_refused() {
        let mut d = Document::new("t");
        d.meta.end = d.meta.start;
        assert!(matches!(d.validate(), Err(DocError::BadMeta(_))));
    }

    #[test]
    fn an_unusable_frame_rate_is_refused() {
        let mut d = Document::new("t");
        d.meta.fps = 0;
        assert!(d.validate().is_err());
        // And one above the model's limit. The limit is the same number the
        // `doc.range` command enforces, so a document can never hold a rate
        // no command can set. §89.
        d.meta.fps = limits::MAX_FRAME_RATE + 1;
        assert!(d.validate().is_err());
        // The limit itself is fine.
        d.meta.fps = limits::MAX_FRAME_RATE;
        assert!(d.validate().is_ok());
    }

    #[test]
    fn a_node_with_a_broken_property_tree_is_refused() {
        let mut d = Document::new("t");
        let id = d.add_node("bad", NodeKind::Filter).unwrap();
        let n = d.get_mut(id).unwrap();
        n.props.push_group(PropGroup::new("dup", "dup"));
        n.props.push_group(PropGroup::new("dup", "dup again"));
        assert!(d.validate().is_err());
    }

    #[test]
    fn remove_reports_whether_anything_went() {
        let mut d = Document::new("t");
        let id = d.add_node("gone", NodeKind::Comment).unwrap();
        assert!(d.remove(id));
        assert!(!d.remove(id));
    }

    #[test]
    fn documents_round_trip_through_json_and_keep_the_id_counter() {
        let mut d = Document::new("round trip");
        d.add_node("second", NodeKind::Filter).unwrap();
        let j = serde_json::to_string(&d).unwrap();
        let mut back: Document = serde_json::from_str(&j).unwrap();
        back.reseat_id_counter();
        assert_eq!(back.nodes.len(), d.nodes.len());
        assert_eq!(back.meta.name, "round trip");
        assert!(back.validate().is_ok());
        // The restored document must not hand out an id that already exists.
        let fresh = back.add_node("fresh", NodeKind::Source).unwrap();
        assert!(back.get(fresh).is_some());
    }

    // ------------------------------------------------------------------ §92 --

    /// The name rule is the model's, and it is asked in one place.
    ///
    /// Measured, before the rule was shared: `node.add { name: "" }` was
    /// accepted by the command and refused by the validator — so the user's
    /// empty name came back as the tree's own unexpected failure — and
    /// `node.add { name: " " }` was accepted while `node.rename` refused the
    /// same name. §92.
    #[test]
    fn the_name_rule_says_what_a_name_may_not_be() {
        assert!(Node::why_not_a_name("Camera").is_none());
        assert!(Node::why_not_a_name("Camera 2").is_none());
        let empty = Node::why_not_a_name("").expect("an empty name is not a name");
        assert!(empty.contains("empty"), "{empty}");
        let blank = Node::why_not_a_name("   ").expect("a blank name is not a name");
        assert!(blank.contains("empty"), "{blank}");
        let long = Node::why_not_a_name(&"x".repeat(limits::MAX_NAME + 1)).expect("too long");
        assert!(long.contains("the limit is"), "{long}");
        // The length is counted in bytes, like every other text limit in the
        // tree — a name of 200 two-byte characters is 400 bytes.
        assert!(Node::why_not_a_name(&"é".repeat(200)).is_some());
        assert!(Node::why_not_a_name(&"é".repeat(100)).is_none());
    }

    /// The constructor refuses a name the validator would refuse.
    ///
    /// Otherwise a command could add a node the document cannot hold, and the
    /// *validator* would be the thing that noticed — after the damage. §92.
    #[test]
    fn the_constructor_refuses_a_name_the_validator_would_refuse() {
        let mut d = Document::empty("names");
        for bad in ["", "   ", &"x".repeat(limits::MAX_NAME + 1)] {
            let err = d.add_node(bad, NodeKind::Source).expect_err("refused");
            assert!(err.to_string().contains("cannot be used"), "{err}");
            assert!(d.nodes.is_empty(), "a refused name still added a node");
        }
        // And a good one still works.
        d.add_node("Camera", NodeKind::Source).expect("a good name");
        assert_eq!(d.nodes.len(), 1);
    }

    /// A file carrying a node with an unusable name is refused.
    ///
    /// The document is deserialized straight into the model, so the rule is
    /// checked rather than kept — the same shape as §91's keyframes. §92.
    #[test]
    fn a_document_with_an_unusable_node_name_is_refused() {
        let raw = r#"{"meta":{"name":"n","fps":24,"start":0,"end":100},"nodes":[{"id":1,"name":"","kind":"source","props":{"id":"props","label":"Properties","nodes":[]},"muted":false}]}"#;
        let doc: Document = serde_json::from_str(raw).expect("the file is readable");
        let err = doc.validate().expect_err("refused");
        assert!(err.to_string().contains("unusable name"), "{err}");
    }

    // ------------------------------------------------------------------ §93 --

    /// A write that changes nothing says so.
    ///
    /// The engine compares the document with the snapshot it took before the
    /// command, and cannot tell "the command forgot to write" from "the
    /// command wrote the value the field already held". So the write says
    /// whether it changed anything, and the command passes that on. §93.
    #[test]
    fn a_write_that_changes_nothing_says_so() {
        let mut d = Document::empty("plan");
        let id = d.add_node("Camera", NodeKind::Source).expect("a node");
        let node = d.get_mut(id).expect("the node");

        assert!(node.rename("Camera 2"), "a new name is a change");
        assert!(!node.rename("Camera 2"), "the same name is not a change");
        assert!(node.set_kind(NodeKind::Filter), "a new kind is a change");
        assert!(!node.set_kind(NodeKind::Filter), "the same kind is not a change");
        assert!(node.set_muted(true), "muting is a change");
        assert!(!node.set_muted(true), "muting twice is not a change");

        assert!(d.set_name("other"), "a new name is a change");
        assert!(!d.set_name("other"), "the same name is not a change");

        let range = (d.meta.start, d.meta.end, d.meta.fps);
        assert!(d.set_range(range.0, range.1, range.2 + 1), "a new rate is a change");
        let range = (d.meta.start, d.meta.end, d.meta.fps);
        assert!(!d.set_range(range.0, range.1, range.2), "the same range is not a change");

        // And moving a node to where it already is changes nothing.
        let second = d.add_node("Second", NodeKind::Source).expect("a node");
        assert!(d.move_to(second, 0).expect("the node is there"), "a move is a change");
        assert!(!d.move_to(second, 0).expect("the node is there"), "a move to where it is is not a change");
    }
}
