//! Evaluation: `(document, instant)` → [`Plan`].

use craftkit_core::{NodeId, Tick};
use craftkit_kernel::{KernelError, apply_gain};
use craftkit_model::{Document, Node, NodeKind, PropGroup, PropNode, limits};

use crate::plan::{Diagnostic, Diagnostics, NodeState, Plan, Resolved};

/// Why an evaluation could not be attempted at all.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EvalError {
    #[error("the document has {count} nodes, more than the limit of {max}", count = .0, max = crate::MAX_NODES)]
    TooManyNodes(usize),
    #[error("evaluation failed below the kernel boundary: {0}")]
    Internal(#[from] KernelError),
}

/// What the level starts at, before the first node.
pub const START_LEVEL: f64 = 1.0;

/// The largest level the evaluator carries forward.
///
/// A filter chain is user arithmetic: `gain` typed a hundred times is a
/// perfectly reasonable thing to do and an easy way to produce an infinity.
/// Saturating keeps such a chain *sounding* wrong instead of poisoning every
/// later value with a `NaN`. The clamp itself lives in
/// [`craftkit_kernel::apply_gain`], so there is exactly one copy of the rule.
pub const MAX_LEVEL: f64 = craftkit_core::MAX_MAGNITUDE;

/// One cached node state with the fingerprint that justifies keeping it.
#[derive(Debug, Clone)]
struct CacheEntry {
    /// Hash of the node as it was when this state was computed.
    node_fp: u64,
    /// Whether any of its properties are animated. A node with no keyframes
    /// produces the same state at every instant, so it survives a time change.
    animated: bool,
    state: NodeState,
}

/// Holds the previous evaluation so the next one can do less work.
///
/// The cache is a `Vec`, not a `HashMap`: the node list is ordered and reuse is
/// inherently a prefix property, so an ordered list is both smaller and easier
/// to reason about.
#[derive(Debug, Default)]
pub struct Evaluator {
    entries: Vec<CacheEntry>,
    cached_at: Option<Tick>,
    /// Instrumentation: how many node evaluations the last call avoided.
    last_reused: usize,
}

impl Evaluator {
    pub fn new() -> Self {
        Self::default()
    }

    /// How many node evaluations the last call reused.
    pub fn reused_last_time(&self) -> usize {
        self.last_reused
    }

    /// Forget everything.
    ///
    /// The engine calls this when it restores an undo entry, because a restore
    /// puts a different document in place.
    ///
    /// It is belt and braces rather than a correctness requirement. The cache
    /// is content-addressed: an entry is reused only when the node it was
    /// computed from still hashes the same, so a restored document cannot be
    /// served a state belonging to a node it no longer has. Clearing is
    /// cheaper than relying on that, and it makes the invariant explicit
    /// rather than argued for. `KNOWN-GAPS.md` §40 has the measurement and
    /// the test that pins the property down.
    pub fn invalidate(&mut self) {
        self.entries.clear();
        self.cached_at = None;
        self.last_reused = 0;
    }

    /// Evaluate `doc` at `at`, reusing whatever it can.
    ///
    /// # The reuse rule
    ///
    /// Walk the node list from the front. Entry `i` is reused when **both**:
    ///
    /// 1. the fingerprint of `doc.nodes[i]` equals the one recorded with its
    ///    cached state, and
    /// 2. either the instant is unchanged, or that node has no animated
    ///    properties.
    ///
    /// The first entry that fails stops the walk. Everything after it is
    /// evaluated afresh even where it looks unchanged, and that is deliberate:
    /// the level entering node `i+1` comes from node `i`, so a change anywhere
    /// in the prefix can change every later state. Stopping is cheaper to
    /// *reason about* than a sound-but-fiddly per-node cache, and for the edits
    /// people actually make — change one value, scrub the playhead — it is also
    /// the faster one.
    ///
    /// # Why the fingerprint is a structural hash
    ///
    /// It used to be a hash of the `serde_json` form, because a hand-written
    /// structural hash has to be updated by hand every time the model grows a
    /// field, and a forgotten update is a **silent wrong answer**: a stale
    /// state the engine believes. Hashing the serialisation cannot drift.
    ///
    /// That was the right trade until the profile showed up. The serialisation
    /// of a node with 200 000 keyframes is 12 MiB of JSON, and hashing it cost
    /// 52 ms where the evaluation it was meant to save cost 7 µs — so a fully
    /// reused evaluation cost exactly as much as a full one, and the cache was
    /// 7 400 times more expensive than the work it avoided. §53.
    ///
    /// The fingerprint is now `Node::fingerprint`, in the model, next to the
    /// data it hashes, and the drift it risks is held down by tests in the
    /// same file that change each part of the node on its own, and by a gate
    /// that reads the tree: a field the model grows and the hash forgets is a
    /// gate failure, not a stale state. §80.
    pub fn evaluate(&mut self, doc: &Document, at: Tick) -> Result<Plan, EvalError> {
        if doc.nodes.len() > limits::MAX_NODES {
            return Err(EvalError::TooManyNodes(doc.nodes.len()));
        }
        let same_time = self.cached_at == Some(at);

        let mut diagnostics = Diagnostics::default();
        let mut nodes: Vec<NodeState> = Vec::with_capacity(doc.nodes.len());
        // One fingerprint per node, computed once.
        //
        // It used to be computed twice — once to decide whether the cached
        // state could be reused, and again when the cache was rebuilt — and
        // each one was a full serialisation of the node's properties. On a
        // document with 200 000 keyframes that was 52 ms each, where the
        // evaluation the cache was meant to save cost 7 µs. §53.
        let mut fingerprints: Vec<u64> = Vec::with_capacity(doc.nodes.len());
        let mut level = START_LEVEL;
        let mut reused = 0usize;
        // Until something fails to match, we are still inside the reusable prefix.
        let mut at_start_of_prefix = true;

        for node in &doc.nodes {
            let idx = nodes.len();
            let fp = node.fingerprint();
            fingerprints.push(fp);
            let reusable = at_start_of_prefix
                && self.entries.get(idx).is_some_and(|e| e.node_fp == fp && (same_time || !e.animated));

            if reusable {
                // `is_some_and` above proved it is there.
                let entry = self.entries.get(idx).cloned();
                match entry {
                    Some(e) => {
                        level = e.state.level_out;
                        nodes.push(e.state);
                        reused += 1;
                    }
                    None => {
                        at_start_of_prefix = false;
                    }
                }
                continue;
            }
            at_start_of_prefix = false;

            let state = eval_node(node, level, at, &mut diagnostics);
            level = state.level_out;
            nodes.push(state);
        }

        self.entries = nodes
            .iter()
            .zip(doc.nodes.iter())
            .zip(fingerprints.iter())
            .map(|((state, node), fp)| CacheEntry {
                node_fp: *fp,
                animated: is_animated(&node.props),
                state: state.clone(),
            })
            .collect();
        self.cached_at = Some(at);
        self.last_reused = reused;

        // A keyframe outside the design's range can never be reached, and the
        // value between two keys is interpolated **across** it — so a key
        // past the end of the design changes every instant of the design,
        // and a key before its start is interpolated towards from the first
        // frame. Nothing else says so. §119.
        //
        // This is outside the reuse branch on purpose: the range belongs to
        // the document, not to a node, so a node whose fingerprint has not
        // changed can still be outside the range the document now has.
        for node in &doc.nodes {
            for (path, prop) in node.props.flatten() {
                for message in keyframes_outside_the_range(&path, prop, doc.meta.start, doc.meta.end) {
                    diagnostics.push(Diagnostic::warn(Some(node.id), &node.name, message));
                }
            }
        }

        Ok(Plan { at, nodes, diagnostics, reused, evaluated: doc.nodes.len() - reused })
    }
}

/// The warning a keyframe outside the design's range earns, if there is one.
///
/// The model's own rule is that the value **holds** before the first key and
/// after the last, and that the value between two keys is interpolated — and
/// both of those rules are about instants the design may not contain. A
/// keyframe outside the range is therefore legal, and it is not a mistake the
/// model should refuse: it is a fact about the design that the person who
/// made it may not know. §119.
fn keyframes_outside_the_range(
    path: &craftkit_model::path::PropPath,
    prop: &craftkit_model::Property,
    start: Tick,
    end: Tick,
) -> Vec<String> {
    let mut out = Vec::new();
    let keys = prop.keys.keys();
    let (Some(first), Some(last)) = (keys.first(), keys.last()) else { return out };
    // **Both** ends, not one: a property with a key before the start and a
    // key after the end is outside the range at both ends, and reporting only
    // the first would hide the second — which is the one that changes every
    // instant of the design.
    if first.time < start {
        out.push(format!(
            "the keyframes of `{path}` reach back to {:.3}s, before the design starts at {:.3}s",
            first.time.seconds_f64(),
            start.seconds_f64()
        ));
    }
    if last.time > end {
        out.push(format!(
            "the keyframes of `{path}` reach {:.3}s, past the end of the design ({:.3}s)",
            last.time.seconds_f64(),
            end.seconds_f64()
        ));
    }
    out
}

/// Evaluate one node, given the level entering it.
///
/// `apply_gain` is the only arithmetic performed, and it is the kernel's: a
/// boundary around the *risky operation*, not around the whole evaluation, so
/// the cost of catching a panic is paid once per node rather than once per frame
/// — and so the seam stays visible when an untrusted node implementation is
/// added later.
fn eval_node(node: &Node, level_in: f64, at: Tick, diag: &mut Diagnostics) -> NodeState {
    let props = resolve_props(&node.props, at);
    let mut level_out = level_in;

    if !node.muted && !node.kind.is_passive() {
        match node.kind {
            // A source *sets* the level, so a missing or wrong `amount` leaves
            // it exactly as it was rather than multiplying by a guess.
            NodeKind::Source => {
                // The amount is in range because every number the tree stores
                // is — the model checks a file's values when it reads them,
                // and a command's values when they are built (§73) — and
                // because the number *between* two keys is clamped to the
                // same range: the interpolator is a second door into the
                // value, and it was the one that was not checked. §65.
                if let Ok(amount) = amount_of(&props, diag, node) {
                    level_out = amount;
                }
            }
            NodeKind::Filter => {
                if let Ok(amount) = amount_of(&props, diag, node) {
                    match apply_gain(level_in, amount) {
                        Ok(v) => level_out = v,
                        Err(e) => {
                            // A refusal here means the incoming level was
                            // already non-finite. That cannot happen while
                            // every level comes from `apply_gain` — except for
                            // a source's, which is its `amount` and is in
                            // range for the reason above. Report the refusal
                            // and pass the level through rather than inventing
                            // a number, so a future bug is visible and not
                            // silent.
                            diag.push(Diagnostic::error(
                                Some(node.id),
                                &node.name,
                                format!("the level could not be scaled: {e}"),
                            ));
                        }
                    }
                }
            }
            NodeKind::Sink | NodeKind::Cue | NodeKind::Comment => {}
        }
    }

    NodeState {
        id: node.id,
        name: node.name.clone(),
        kind: node.kind,
        muted: node.muted,
        level_in,
        level_out,
        props,
    }
}

/// The `amount` property, with the rules for what a missing or wrong one means.
///
/// `Ok(())`-free on purpose: the `Err` case has already pushed its diagnostic,
/// so the caller only has to decide what value to carry instead of formatting
/// the same message twice.
fn amount_of(props: &[Resolved], diag: &mut Diagnostics, node: &Node) -> Result<f64, ()> {
    let Some(r) = props.iter().find(|p| p.path == "amount") else {
        diag.push(Diagnostic::warn(Some(node.id), &node.name, "has no `amount` property; using 1.0"));
        return Ok(1.0);
    };
    match r.value.as_num() {
        Some(v) if v.is_finite() => Ok(v),
        Some(v) => {
            diag.push(Diagnostic::error(
                Some(node.id),
                &node.name,
                format!("`amount` is {v}, which is not a number; using 1.0"),
            ));
            Err(())
        }
        None => {
            diag.push(Diagnostic::error(
                Some(node.id),
                &node.name,
                format!("`amount` is a {}, not a number; using 1.0", r.value.type_name()),
            ));
            Err(())
        }
    }
}

/// Sample every property at `at`, sorted by path.
///
/// The paths come from the model's own [`PropGroup::flatten`], not from a walk
/// written here. That is the whole point: a command addresses a property by the
/// path the plan reports, and two code paths that each spell a path out their
/// own way are two ways to be wrong.
fn resolve_props(group: &PropGroup, at: Tick) -> Vec<Resolved> {
    let mut out: Vec<Resolved> = group
        .flatten()
        .into_iter()
        .map(|(path, prop)| Resolved {
            path: path.to_string(),
            id: prop.id,
            value: prop.value_at(at),
            animated: prop.is_animated(),
        })
        .collect();
    // Sorting makes two plans for the same instant compare equal regardless of
    // the order the properties happen to sit in the tree.
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

/// Whether any property below this group has keyframes.
fn is_animated(group: &PropGroup) -> bool {
    fn walk(nodes: &[PropNode], depth: usize) -> bool {
        if depth > limits::MAX_DEPTH {
            return false;
        }
        nodes.iter().any(|n| match n {
            PropNode::Prop(p) => p.is_animated(),
            PropNode::Group(g) => walk(&g.nodes, depth + 1),
        })
    }
    walk(&group.nodes, 0)
}

/// Evaluate once, without keeping a cache.
///
/// For tests, the CLI's one-shot mode, and anywhere the reuse would not pay.
pub fn evaluate_once(doc: &Document, at: Tick) -> Result<Plan, EvalError> {
    Evaluator::new().evaluate(doc, at)
}

/// Look a node up in a plan, for a command's result.
pub fn state_of(plan: &Plan, id: NodeId) -> Option<&NodeState> {
    plan.state(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use craftkit_core::{FrameRate, Tick, Value};
    use craftkit_model::{Document, Easing, Key, Keyframes, Property};

    fn doc_with(spec: &[(NodeKind, &str, f64)]) -> Document {
        let mut d = Document::empty("test");
        for (kind, name, amount) in spec {
            let id = d.add_node(*name, *kind).expect("id");
            let node = d.get_mut(id).expect("just added");
            node.props.push(Property::new(1, "amount", Value::num("amount", *amount).expect("value")));
        }
        d
    }

    fn first_prop(doc: &mut Document, id: NodeId) -> &mut Property {
        let node = doc.get_mut(id).expect("node");
        match node.props.nodes.first_mut() {
            Some(PropNode::Prop(p)) => p,
            other => panic!("expected a property, found {other:?}"),
        }
    }

    /// A keyframe outside the design's range is a warning, not a silence.
    ///
    /// §119: a key past the end of the design changes **every instant** of
    /// the design — the value between two keys is interpolated across it —
    /// and the playhead can never reach the key itself. Nothing said so.
    #[test]
    fn a_keyframe_outside_the_design_is_a_warning() {
        let mut d = doc_with(&[(NodeKind::Source, "src", 1.0)]);
        let end = d.meta.end;
        let past_the_end = Tick::from_raw(end.raw() + 1_000_000_000);
        let prop = first_prop(&mut d, NodeId(1));
        let mut keys = Keyframes::new();
        let one = Value::num("amount", 1.0).expect("value");
        let two = Value::num("amount", 2.0).expect("value");
        keys.set(Key::new(end, one, Easing::Linear).expect("key")).expect("set");
        keys.set(Key::new(past_the_end, two, Easing::Linear).expect("key")).expect("set");
        prop.keys = keys;

        let plan = evaluate_once(&d, Tick::ZERO).expect("plan");
        assert_eq!(plan.diagnostics.items.len(), 1, "{:?}", plan.diagnostics);
        let message = &plan.diagnostics.items[0].message;
        assert!(message.contains("past the end of the design"), "{message}");
        assert!(message.contains("amount"), "{message}");
    }

    /// A keyframe at both ends outside the range is reported at both ends.
    ///
    /// The one that matters is the end *after* the design: it changes every
    /// instant of the design. Reporting only the first would hide it. §119.
    #[test]
    fn a_keyframe_outside_the_design_is_reported_at_both_ends() {
        let mut d = doc_with(&[(NodeKind::Source, "src", 1.0)]);
        let start = d.meta.start;
        let end = d.meta.end;
        let before = Tick::from_raw(start.raw() - 1_000_000_000);
        let after = Tick::from_raw(end.raw() + 1_000_000_000);
        let prop = first_prop(&mut d, NodeId(1));
        let mut keys = Keyframes::new();
        let one = Value::num("amount", 1.0).expect("value");
        let two = Value::num("amount", 2.0).expect("value");
        keys.set(Key::new(before, one, Easing::Linear).expect("key")).expect("set");
        keys.set(Key::new(after, two, Easing::Linear).expect("key")).expect("set");
        prop.keys = keys;

        let plan = evaluate_once(&d, Tick::ZERO).expect("plan");
        assert_eq!(plan.diagnostics.items.len(), 2, "{:?}", plan.diagnostics);
        let messages: Vec<&str> = plan.diagnostics.items.iter().map(|d| d.message.as_str()).collect();
        assert!(messages.iter().any(|m| m.contains("before the design starts")), "{messages:?}");
        assert!(messages.iter().any(|m| m.contains("past the end of the design")), "{messages:?}");
    }

    /// A keyframe inside the design is not a warning.
    ///
    /// The warning is worth nothing if it fires on every design. §119.
    #[test]
    fn a_keyframe_inside_the_design_is_not_a_warning() {
        let mut d = doc_with(&[(NodeKind::Source, "src", 1.0)]);
        let start = d.meta.start;
        let end = d.meta.end;
        let prop = first_prop(&mut d, NodeId(1));
        let mut keys = Keyframes::new();
        let one = Value::num("amount", 1.0).expect("value");
        let two = Value::num("amount", 2.0).expect("value");
        keys.set(Key::new(start, one, Easing::Linear).expect("key")).expect("set");
        keys.set(Key::new(end, two, Easing::Linear).expect("key")).expect("set");
        prop.keys = keys;

        let plan = evaluate_once(&d, Tick::ZERO).expect("plan");
        assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    }

    #[test]
    fn a_source_sets_the_level_and_a_filter_scales_it() {
        let d = doc_with(&[
            (NodeKind::Source, "src", 2.0),
            (NodeKind::Filter, "gain", 3.0),
            (NodeKind::Filter, "trim", 0.5),
        ]);
        let plan = evaluate_once(&d, Tick::ZERO).expect("plan");
        assert_eq!(plan.state(NodeId(1)).expect("src").level_out, 2.0);
        assert_eq!(plan.state(NodeId(2)).expect("gain").level_out, 6.0);
        assert_eq!(plan.state(NodeId(3)).expect("trim").level_out, 3.0);
        assert!((plan.output() - 3.0).abs() < 1e-9);
        assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    }

    /// The level a source sets is in range, even when its amount is animated.
    ///
    /// A source takes its `amount` directly rather than through the kernel's
    /// clamping `apply_gain`, so the amount is the level. The amount is a
    /// number the tree checked — unless it is *between* two keys, and there
    /// the interpolator was the one that was not. Measured: a source whose
    /// animated amount overshot took the level to 1 497 942 087, outside the
    /// 1 000 000 000 the tree allows a number to be. §65.
    #[test]
    fn an_animated_amount_never_takes_the_level_out_of_range() {
        let mut d = Document::empty("test");
        let id = d.add_node("src", NodeKind::Source).expect("id");
        let mut keys = Keyframes::new();
        // The first key's easing is the one used across the span, and a
        // bezier with an overshoot in y is a checked, legal easing.
        keys.set(
            Key::new(
                Tick::from_frames_i64(0, FrameRate(24)),
                Value::num("a", craftkit_core::MAX_MAGNITUDE).expect("value"),
                Easing::Bezier { out_x: 0.5, out_y: 2.0 },
            )
            .expect("key"),
        )
        .expect("insert");
        keys.set(
            Key::new(
                Tick::from_frames_i64(24, FrameRate(24)),
                Value::num("b", -(craftkit_core::MAX_MAGNITUDE)).expect("value"),
                Easing::Linear,
            )
            .expect("key"),
        )
        .expect("insert");
        let mut prop = Property::new(1, "amount", Value::num("amount", 0.0).expect("value"));
        prop.keys = keys;
        d.get_mut(id).expect("node").props.push(prop);

        for f in 0..=24 {
            let at = Tick::from_frames_i64(f, FrameRate(24));
            let plan = evaluate_once(&d, at).expect("plan");
            let level = plan.state(id).expect("src").level_out;
            assert!(
                level.abs() <= craftkit_core::MAX_MAGNITUDE,
                "frame {f} took the level to {level}, which is outside the range"
            );
        }
    }

    #[test]
    fn a_sink_reads_the_level_without_changing_it() {
        let d = doc_with(&[(NodeKind::Source, "src", 4.0), (NodeKind::Sink, "out", 9.0)]);
        let plan = evaluate_once(&d, Tick::ZERO).expect("plan");
        assert_eq!(plan.state(NodeId(2)).expect("sink").level_out, 4.0);
    }

    #[test]
    fn a_cue_carries_animation_without_changing_the_level() {
        let d = doc_with(&[(NodeKind::Source, "src", 4.0), (NodeKind::Cue, "mark", 100.0)]);
        let plan = evaluate_once(&d, Tick::ZERO).expect("plan");
        assert_eq!(plan.state(NodeId(2)).expect("cue").level_out, 4.0);
    }

    #[test]
    fn a_muted_node_passes_the_level_through_untouched() {
        let mut d = doc_with(&[(NodeKind::Source, "src", 4.0), (NodeKind::Filter, "gain", 100.0)]);
        d.get_mut(NodeId(2)).expect("node").muted = true;
        let plan = evaluate_once(&d, Tick::ZERO).expect("plan");
        let state = plan.state(NodeId(2)).expect("gain");
        assert_eq!(state.level_out, 4.0);
        assert!(state.muted);
    }

    #[test]
    fn a_runaway_chain_saturates_instead_of_producing_an_infinity() {
        // `Value::num` already refuses a value outside ±MAX_MAGNITUDE, so a
        // runaway cannot be *stored*: it has to be produced by repetition. That
        // is the real threat model — a legal chain of legal numbers that
        // multiplies its way past the world.
        let big: f64 = 1e9;
        let mut d = Document::empty("runaway");
        let spec =
            std::iter::once((NodeKind::Source, big)).chain(std::iter::repeat_n((NodeKind::Filter, big), 40));
        for (i, (kind, amount)) in spec.enumerate() {
            let id = d.add_node(format!("n{i}"), kind).expect("id");
            let node = d.get_mut(id).expect("just added");
            node.props.push(Property::new(
                1,
                "amount",
                Value::num("amount", amount).expect("within the magnitude limit"),
            ));
        }
        let plan = evaluate_once(&d, Tick::ZERO).expect("plan");
        assert!(plan.output().is_finite(), "the plan leaked a non-finite level");
        assert!((plan.output() - MAX_LEVEL).abs() < 1e-6, "level was {}", plan.output());
    }

    #[test]
    fn a_value_outside_the_magnitude_limit_cannot_even_be_stored() {
        // The defence that makes the test above necessary. If this ever starts
        // succeeding, the runaway case stops being reachable and the clamping
        // rule needs a new home.
        assert!(Value::num("amount", 1e300).is_err());
        assert!(Value::num("amount", f64::NAN).is_err());
    }

    #[test]
    fn a_non_numeric_amount_is_reported_and_does_not_stop_the_plan() {
        let mut d = doc_with(&[(NodeKind::Source, "src", 1.0), (NodeKind::Sink, "out", 1.0)]);
        // Force the type mismatch a hand-edited file would produce.
        let node = d.get_mut(NodeId(1)).expect("node");
        node.props.nodes[0] = PropNode::Prop(Property::new(1, "amount", Value::Bool(true)));
        let plan = evaluate_once(&d, Tick::ZERO).expect("the plan still evaluates");
        assert!(plan.diagnostics.has_errors());
        assert_eq!(plan.nodes.len(), 2, "evaluation must continue past a bad node");
        assert!(plan.diagnostics.items[0].render().contains("amount"));
    }

    #[test]
    fn a_missing_amount_warns_rather_than_failing() {
        let mut d = Document::empty("t");
        let _ = d.add_node("bare", NodeKind::Source);
        let plan = evaluate_once(&d, Tick::ZERO).expect("plan");
        assert_eq!(plan.diagnostics.error_count(), 0);
        assert_eq!(plan.diagnostics.items.len(), 1);
        assert_eq!(plan.output(), 1.0);
    }

    #[test]
    fn a_keyframed_property_is_sampled_at_the_instant() {
        let mut d = doc_with(&[(NodeKind::Source, "src", 1.0)]);
        let p = first_prop(&mut d, NodeId(1));
        p.keys = Keyframes::new();
        p.keys
            .set(Key::new(Tick::ZERO, Value::num("v", 0.0).expect("v"), Easing::Linear).expect("k"))
            .expect("set");
        p.keys
            .set(
                Key::new(
                    Tick::from_frames_i64(24, FrameRate(24)),
                    Value::num("v", 10.0).expect("v"),
                    Easing::Linear,
                )
                .expect("k"),
            )
            .expect("set");

        let at_end = Tick::from_frames_i64(24, FrameRate(24));
        let plan = evaluate_once(&d, at_end).expect("plan");
        assert_eq!(plan.state(NodeId(1)).expect("src").level_out, 10.0);
        let resolved = plan.state(NodeId(1)).expect("src").props.first().expect("prop");
        assert!(resolved.animated, "the resolved property should be marked animated");
    }

    #[test]
    fn editing_the_last_node_reuses_the_whole_prefix() {
        let mut d = doc_with(&[
            (NodeKind::Source, "src", 2.0),
            (NodeKind::Filter, "a", 2.0),
            (NodeKind::Filter, "b", 2.0),
            (NodeKind::Filter, "c", 2.0),
        ]);
        let mut ev = Evaluator::new();
        let first = ev.evaluate(&d, Tick::ZERO).expect("plan");
        assert_eq!((first.reused, first.evaluated), (0, 4));

        first_prop(&mut d, NodeId(4)).value = Value::num("v", 5.0).expect("v");
        let second = ev.evaluate(&d, Tick::ZERO).expect("plan");
        assert_eq!((second.reused, second.evaluated), (3, 1), "the prefix should have been reused");
        assert_eq!(second.output(), 40.0);
        assert_eq!(ev.reused_last_time(), 3);
    }

    #[test]
    fn editing_the_first_node_reuses_nothing() {
        let mut d = doc_with(&[(NodeKind::Source, "src", 2.0), (NodeKind::Filter, "a", 2.0)]);
        let mut ev = Evaluator::new();
        let _ = ev.evaluate(&d, Tick::ZERO).expect("plan");
        first_prop(&mut d, NodeId(1)).value = Value::num("v", 9.0).expect("v");
        let second = ev.evaluate(&d, Tick::ZERO).expect("plan");
        assert_eq!(second.reused, 0, "a changed prefix invalidates everything after it");
        assert_eq!(second.output(), 18.0);
    }

    #[test]
    fn a_source_sets_the_level_rather_than_multiplying_it() {
        // The level entering a source is whatever came before it. If a source
        // multiplied instead, the second source in a chain would be scaled by
        // the first and `amount` would mean something different depending on
        // where in the chain the node sat.
        //
        // This rule was anchored, but only inside
        // `reordering_nodes_is_not_served_from_the_cache` — a test named for the
        // cache. It is asserted here under its own name so that a change to the
        // cache test cannot quietly drop the only coverage of what a source
        // does.
        let d = doc_with(&[(NodeKind::Source, "a", 2.0), (NodeKind::Source, "b", 5.0)]);
        let mut ev = Evaluator::new();
        let plan = ev.evaluate(&d, Tick::ZERO).expect("plan");
        assert_eq!(plan.state(NodeId(1)).expect("a").level_out, 2.0);
        assert_eq!(plan.state(NodeId(2)).expect("b").level_out, 5.0, "b must not be 2.0 × 5.0");
        assert_eq!(plan.output(), 5.0);
    }

    #[test]
    fn the_chain_starts_at_unity_so_one_gain_is_one_gain() {
        // A chain of gains that all say 1.0 leaves the level where it started.
        // That is the whole reason [`START_LEVEL`] is one: `gain` typed once
        // means a real gain, rather than a fraction of an arbitrary constant.
        assert_eq!(START_LEVEL, 1.0, "the chain starts at one");
        let d = doc_with(&[(NodeKind::Filter, "a", 1.0), (NodeKind::Filter, "b", 1.0)]);
        let mut ev = Evaluator::new();
        let plan = ev.evaluate(&d, Tick::ZERO).expect("plan");
        assert_eq!(plan.output(), START_LEVEL);
    }

    #[test]
    fn reordering_nodes_is_not_served_from_the_cache() {
        let mut d = doc_with(&[(NodeKind::Source, "a", 2.0), (NodeKind::Source, "b", 5.0)]);
        let mut ev = Evaluator::new();
        let _ = ev.evaluate(&d, Tick::ZERO).expect("plan");
        // Swap the two nodes. Their contents are unchanged, but what flows
        // through position 0 changed, so a content-only fingerprint would lie.
        d.nodes.swap(0, 1);
        let second = ev.evaluate(&d, Tick::ZERO).expect("plan");
        assert_eq!(second.reused, 0, "position is part of a node's identity here");
        // A source *sets* the level, so the plan's output is the last node's,
        // which after the swap is the one that was first.
        assert_eq!(second.state(NodeId(2)).expect("b").level_out, 5.0);
        assert_eq!(second.output(), 2.0);
    }

    #[test]
    fn scrubbing_time_reuses_nodes_that_have_no_keyframes() {
        let mut d = doc_with(&[(NodeKind::Source, "static", 2.0), (NodeKind::Filter, "moving", 3.0)]);
        first_prop(&mut d, NodeId(2))
            .keys
            .set(Key::new(Tick::ZERO, Value::num("v", 1.0).expect("v"), Easing::Linear).expect("k"))
            .expect("set");
        let mut ev = Evaluator::new();
        let _ = ev.evaluate(&d, Tick::ZERO).expect("plan");

        let later = Tick::from_seconds_f64(1.0);
        let second = ev.evaluate(&d, later).expect("plan");
        assert_eq!(second.reused, 1, "the static head should survive a time change");
        assert_eq!(second.evaluated, 1);
    }

    #[test]
    fn scrubbing_time_recomputes_an_animated_node() {
        let mut d = doc_with(&[(NodeKind::Source, "src", 1.0)]);
        let p = first_prop(&mut d, NodeId(1));
        p.keys = Keyframes::new();
        p.keys
            .set(Key::new(Tick::ZERO, Value::num("v", 0.0).expect("v"), Easing::Linear).expect("k"))
            .expect("set");
        p.keys
            .set(
                Key::new(Tick::from_seconds_f64(1.0), Value::num("v", 7.0).expect("v"), Easing::Linear)
                    .expect("k"),
            )
            .expect("set");

        let mut ev = Evaluator::new();
        assert_eq!(ev.evaluate(&d, Tick::ZERO).expect("plan").output(), 0.0);
        let later = ev.evaluate(&d, Tick::from_seconds_f64(1.0)).expect("plan");
        assert_eq!(later.output(), 7.0);
        assert_eq!(later.reused, 0);
    }

    #[test]
    fn a_node_whose_keyframes_changed_is_not_served_from_the_cache() {
        // Nothing in the tree checked this. The other cache tests change a
        // property's *value* or move a node; none of them changed a
        // *keyframe*, which is the part of a node the fingerprint has the
        // most to get right — and the part a hand-written hash is most likely
        // to forget. §53.
        let mut d = doc_with(&[(NodeKind::Source, "src", 1.0)]);
        let p = first_prop(&mut d, NodeId(1));
        p.keys = Keyframes::new();
        p.keys
            .set(Key::new(Tick::ZERO, Value::num("v", 0.0).expect("v"), Easing::Linear).expect("k"))
            .expect("set");
        let mut ev = Evaluator::new();
        assert_eq!(ev.evaluate(&d, Tick::ZERO).expect("plan").output(), 0.0);

        // Now the key at zero moves: the value at zero changes with it.
        let p = first_prop(&mut d, NodeId(1));
        p.keys.clear();
        p.keys
            .set(Key::new(Tick::ZERO, Value::num("v", 3.0).expect("v"), Easing::Linear).expect("k"))
            .expect("set");

        let after = ev.evaluate(&d, Tick::ZERO).expect("plan");
        assert_eq!(after.reused, 0, "a changed keyframe must invalidate the cache");
        assert_eq!(after.output(), 3.0, "the plan must show the new value");
    }

    /// A changed **easing** is a changed node.
    ///
    /// The other cache tests change a property's value or a keyframe's value.
    /// None of them changed a keyframe's *easing*, and that is the part a
    /// hand-written hash forgets most easily: the easing is not the value, it
    /// is the curve between two values, so dropping it from the hash leaves
    /// the cache serving a state computed with the old curve.
    ///
    /// Measured: removing `hash_easing` from `Node::fingerprint` left every
    /// test in the workspace green, and the plan below kept the old curve.
    /// §80.
    #[test]
    fn a_changed_easing_is_not_served_from_the_cache() {
        let mut d = doc_with(&[(NodeKind::Source, "src", 1.0)]);
        {
            let p = first_prop(&mut d, NodeId(1));
            p.keys = Keyframes::new();
            p.keys
                .set(Key::new(Tick::ZERO, Value::num("v", 0.0).expect("v"), Easing::Linear).expect("k"))
                .expect("set");
            p.keys
                .set(
                    Key::new(
                        Tick::from_frames_i64(24, FrameRate(24)),
                        Value::num("v", 1.0).expect("v"),
                        Easing::Linear,
                    )
                    .expect("k"),
                )
                .expect("set");
        }
        // A quarter of the way across the span, where linear and smoothstep
        // are far enough apart to tell apart.
        let at = Tick::from_frames_i64(6, FrameRate(24));
        let mut ev = Evaluator::new();
        let first = ev.evaluate(&d, at).expect("plan");
        assert!((first.output() - 0.25).abs() < 1e-9, "the linear sample was {}", first.output());

        // Change only the curve. The keys, their times and their values are
        // untouched.
        {
            let p = first_prop(&mut d, NodeId(1));
            let time = p.keys.keys().first().expect("a key").time;
            p.keys.set_ease(time, Easing::EaseInOut).expect("the easing is a valid one");
        }

        let second = ev.evaluate(&d, at).expect("plan");
        assert_eq!(second.reused, 0, "a changed easing must invalidate the cache");
        // `EaseInOut` is smoothstep: 0.25² × (3 − 2 × 0.25) = 0.15625.
        assert!(
            (second.output() - 0.15625).abs() < 1e-4,
            "the plan still shows the old curve: {}",
            second.output()
        );
    }

    #[test]
    fn an_unchanged_document_is_entirely_a_cache_hit() {
        let d = doc_with(&[(NodeKind::Source, "src", 2.0), (NodeKind::Filter, "a", 2.0)]);
        let mut ev = Evaluator::new();
        let first = ev.evaluate(&d, Tick::ZERO).expect("plan");
        let again = ev.evaluate(&d, Tick::ZERO).expect("plan");
        assert_eq!((again.reused, again.evaluated), (2, 0));
        assert_eq!(again.output(), first.output(), "reuse must not change the answer");
        assert_eq!(again.nodes, first.nodes, "a reused state must equal a fresh one");
    }

    #[test]
    fn invalidating_forces_a_full_re_evaluation() {
        let d = doc_with(&[(NodeKind::Source, "src", 2.0), (NodeKind::Filter, "a", 2.0)]);
        let mut ev = Evaluator::new();
        let first = ev.evaluate(&d, Tick::ZERO).expect("plan");
        ev.invalidate();
        let fresh = ev.evaluate(&d, Tick::ZERO).expect("plan");
        assert_eq!(fresh.reused, 0);
        assert_eq!(fresh.nodes, first.nodes, "reuse must not change the answer");
    }

    #[test]
    fn growing_the_document_evaluates_only_the_new_tail() {
        let mut d = doc_with(&[(NodeKind::Source, "src", 2.0), (NodeKind::Filter, "a", 2.0)]);
        let mut ev = Evaluator::new();
        let _ = ev.evaluate(&d, Tick::ZERO).expect("plan");
        let _ = d.add_node("b", NodeKind::Filter).expect("id");
        let after = ev.evaluate(&d, Tick::ZERO).expect("plan");
        assert_eq!(after.reused, 2);
        assert_eq!(after.evaluated, 1);
        assert_eq!(after.output(), 4.0);
    }

    #[test]
    fn removing_a_node_does_not_reuse_a_tail_that_moved() {
        let mut d = doc_with(&[
            (NodeKind::Source, "a", 2.0),
            (NodeKind::Filter, "b", 2.0),
            (NodeKind::Filter, "c", 2.0),
        ]);
        let mut ev = Evaluator::new();
        let _ = ev.evaluate(&d, Tick::ZERO).expect("plan");
        assert!(d.remove(NodeId(1)), "removes by id");
        let after = ev.evaluate(&d, Tick::ZERO).expect("plan");
        assert_eq!(after.nodes.len(), 2);
        assert_eq!(after.output(), 4.0, "the level must follow the surviving nodes");
    }

    #[test]
    fn an_empty_document_evaluates_to_an_empty_plan() {
        let d = Document::empty("nothing");
        let plan = evaluate_once(&d, Tick::ZERO).expect("plan");
        assert!(plan.nodes.is_empty());
        assert_eq!(plan.output(), 0.0);
        assert_eq!((plan.reused, plan.evaluated), (0, 0));
    }

    #[test]
    fn diagnostics_are_capped_and_the_overflow_is_counted() {
        let mut d = Diagnostics::default();
        for _ in 0..5 {
            d.push_capped(Diagnostic::warn(None, "n", "x"), 3);
        }
        assert_eq!(d.items.len(), 3);
        assert_eq!(d.dropped, 2);
        assert!(d.to_string().contains("2 further"));
    }

    #[test]
    fn a_plan_summarizes_for_a_status_bar() {
        let d = doc_with(&[(NodeKind::Source, "src", 1.0)]);
        let plan = evaluate_once(&d, Tick::from_seconds_f64(0.5)).expect("plan");
        let s = plan.summary();
        assert!(s.contains("0.500"), "{s}");
    }
}
