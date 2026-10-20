//! The operation the kernel actually performs: walking a chain of nodes and
//! carrying one value through them.
//!
//! This exists so the boundary has something real to guard. A guard with nothing
//! behind it is untested scaffolding; here, a panic deep in the walk is a real
//! failure mode (a cycle, an absurd length, an arithmetic overflow in code that
//! will one day become a foreign implementation).

use super::{KernelError, guard};

/// How many nodes one call may walk. A cycle is refused rather than detected by
/// a step counter deep in a loop, so the limit is checked up front where the
/// error can still name the cause.
pub const MAX_STEPS: usize = 64;

/// One node's contribution to the chain.
#[derive(Debug, Clone, PartialEq)]
pub struct ChainStep {
    /// The node's name, for diagnostics.
    pub name: String,
    /// Whether the node is active. A muted node is skipped but still counted,
    /// because a document full of muted nodes is not a smaller document.
    pub enabled: bool,
    /// How much this node multiplies the value by.
    pub gain: f64,
    /// How far this node shifts the value.
    pub offset: f64,
}

impl ChainStep {
    /// Build a step, refusing a gain or offset the domain cannot use.
    pub fn new(name: impl Into<String>, enabled: bool, gain: f64, offset: f64) -> Result<Self, KernelError> {
        let bad = |what: &'static str, v: f64| KernelError::Refused { what, detail: format!("{v}") };
        if !gain.is_finite() {
            return Err(bad("gain", gain));
        }
        if !offset.is_finite() {
            return Err(bad("offset", offset));
        }
        Ok(Self { name: name.into(), enabled, gain, offset })
    }
}

/// The immutable input to a walk.
#[derive(Debug, Clone, PartialEq)]
pub struct ChainInput {
    pub start: f64,
    pub steps: Vec<ChainStep>,
}

impl ChainInput {
    /// Check the input *before* walking it, so a bad document is refused with a
    /// message about the document rather than about a half-finished walk.
    pub fn validated(self) -> Result<Self, KernelError> {
        if !self.start.is_finite() {
            return Err(KernelError::Refused { what: "start value", detail: format!("{}", self.start) });
        }
        if self.steps.len() > MAX_STEPS {
            return Err(KernelError::Refused {
                what: "chain length",
                detail: format!(
                    "{} nodes, the limit is {MAX_STEPS} (a chain that long is usually a cycle)",
                    self.steps.len()
                ),
            });
        }
        Ok(self)
    }
}

/// Walk the chain: `((start + offset₀) * gain₀ + offset₁) * gain₁ + …`
///
/// **This does not saturate; it refuses.** A value that leaves the finite world
/// stops the walk with an error naming the node it happened at. That is a
/// deliberate difference from [`apply_gain`], which clamps at
/// [`craftkit_core::MAX_MAGNITUDE`] instead — see that function for why the two
/// differ.
///
/// This doc previously claimed "saturating arithmetic throughout, so a long chain
/// of large gains reaches a limit rather than an infinity", and the function did
/// the opposite. `overflow_stops_the_walk_with_a_named_node`, twenty lines below,
/// asserts the refusal. A comment that contradicts the test beneath it is worse
/// than no comment, because it is believed.
///
/// The returned **trace** holds the start value followed by one value per step,
/// each recorded *after* that step was applied (so a muted step repeats the
/// previous value). Recording the input side of every step as well would double
/// the allocation for a series the editor only ever draws from its tail.
pub fn evaluate_chain(input: ChainInput) -> Result<Vec<f64>, KernelError> {
    // Inside the boundary, not merely next to it: this is the walk a future
    // foreign implementation would replace, and the boundary has to be where
    // that swap happens rather than one frame out from it.
    guard("evaluating the chain", || evaluate_chain_inner(input))
}

fn evaluate_chain_inner(input: ChainInput) -> Result<Vec<f64>, KernelError> {
    let input = input.validated()?;
    let mut acc = input.start;
    let mut trace = Vec::with_capacity(input.steps.len() + 1);
    trace.push(acc);
    for step in &input.steps {
        if step.enabled {
            acc = (acc + step.offset).mul_add(step.gain, 0.0);
        }
        // A value that has left the finite world cannot be made finite again by
        // multiplying it by something, so stop here with a real error.
        if !acc.is_finite() {
            return Err(KernelError::Failed {
                what: "chain evaluation",
                detail: format!("the value became {} at node `{}`", acc, step.name),
            });
        }
        trace.push(acc);
    }
    Ok(trace)
}

/// The one arithmetic operation the plan evaluator is allowed to perform.
///
/// `level * gain`, saturated at [`craftkit_core::MAX_MAGNITUDE`].
///
/// It exists as a separate entry point rather than as a `ChainStep` because the
/// evaluator applies it **per node, per frame**, and allocating a `Vec` of
/// steps for a single multiply would put the dominant cost of every
/// re-evaluation in the allocator.
///
/// **It saturates and [`evaluate_chain`] does not**, and that is intentional
/// rather than an oversight left behind. This runs per node per frame, in the
/// middle of drawing: a runaway gain must not throw away the frame, and
/// "sounding wrong" is the outcome the project wants from a hundred `gain`
/// parameters. `evaluate_chain` is a whole-document walk that a person asked for
/// as a unit, so an infinity there is information — the walk stops and says which
/// node produced it. One doc comment used to claim the two rules were identical
/// and that there was "exactly one copy" of the rule. There are two, and they
/// are different on purpose.
///
/// The `guard` is deliberate even though the arithmetic below is total: this is
/// the seam where an untrusted node implementation would be plugged in. Put the
/// boundary *around* the operation rather than around the evaluator, so the
/// cost is paid per risky call instead of per frame.
pub fn apply_gain(level: f64, gain: f64) -> Result<f64, KernelError> {
    guard("applying a gain", || {
        if !level.is_finite() || !gain.is_finite() {
            return Err(KernelError::Refused {
                what: "gain",
                detail: format!("level {level} and gain {gain} must both be finite"),
            });
        }
        let v = level * gain;
        if v.is_nan() {
            // Only reachable as 0 * inf, which `is_finite` above already excludes.
            // Kept so a future compiler change cannot turn this into a silent NaN.
            return Err(KernelError::Refused {
                what: "gain",
                detail: "the product is not a number".to_string(),
            });
        }
        Ok(v.clamp(-craftkit_core::MAX_MAGNITUDE, craftkit_core::MAX_MAGNITUDE))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_gain_scales_the_level() {
        assert_eq!(apply_gain(2.0, 3.0).expect("ok"), 6.0);
    }

    #[test]
    fn a_runaway_gain_saturates_instead_of_producing_an_infinity() {
        let got = apply_gain(1e300, 1e300).expect("saturates, it does not fail");
        assert!(got.is_finite());
        assert_eq!(got, craftkit_core::MAX_MAGNITUDE);
    }

    #[test]
    fn a_negative_gain_is_allowed_because_the_domain_allows_it() {
        assert_eq!(apply_gain(4.0, -0.5).expect("ok"), -2.0);
    }

    #[test]
    fn a_non_finite_input_is_refused_with_both_numbers_in_the_message() {
        let err = apply_gain(1.0, f64::NAN).expect_err("refused");
        let msg = err.to_string();
        assert!(msg.contains("gain"), "{msg}");
        assert!(msg.contains("finite"), "{msg}");
    }

    fn step(name: &str, gain: f64, offset: f64) -> ChainStep {
        ChainStep::new(name, true, gain, offset).expect("valid step")
    }

    #[test]
    fn an_empty_chain_returns_the_start() {
        let r = evaluate_chain(ChainInput { start: 3.0, steps: vec![] }).expect("ok");
        assert_eq!(r, vec![3.0]);
    }

    #[test]
    fn the_trace_holds_the_start_then_one_value_per_step() {
        let r =
            evaluate_chain(ChainInput { start: 0.0, steps: vec![step("a", 2.0, 1.0), step("b", 3.0, 0.0)] })
                .expect("ok");
        // (0 + 1) * 2 = 2, then 2 * 3 = 6 — one recorded value per step.
        assert_eq!(r, vec![0.0, 2.0, 6.0]);
    }

    #[test]
    fn the_trace_index_i_is_the_value_after_step_i() {
        let r =
            evaluate_chain(ChainInput { start: 10.0, steps: vec![step("a", 2.0, 1.0), step("b", 0.5, 4.0)] })
                .expect("ok");
        // index 0 = start, index i = value after step i-1.
        assert_eq!(r.len(), 3);
        assert_eq!(r[1], 22.0); // (10 + 1) * 2
        assert_eq!(r[2], 13.0); // (22 + 4) * 0.5
    }

    #[test]
    fn a_muted_node_is_skipped_but_still_traced() {
        let mut disabled = step("a", 100.0, 0.0);
        disabled.enabled = false;
        let r = evaluate_chain(ChainInput { start: 5.0, steps: vec![disabled, step("b", 2.0, 0.0)] })
            .expect("ok");
        assert_eq!(r, vec![5.0, 5.0, 10.0]);
    }

    #[test]
    fn a_non_finite_gain_or_offset_is_refused_at_construction() {
        assert!(ChainStep::new("a", true, f64::NAN, 0.0).is_err());
        assert!(ChainStep::new("a", true, f64::INFINITY, 0.0).is_err());
        assert!(ChainStep::new("a", true, 1.0, f64::NAN).is_err());
    }

    #[test]
    fn an_over_long_chain_is_refused_and_the_message_says_why() {
        let steps: Vec<ChainStep> = (0..MAX_STEPS + 1).map(|i| step(&format!("n{i}"), 1.0, 0.0)).collect();
        let err = evaluate_chain(ChainInput { start: 0.0, steps }).expect_err("refused");
        let msg = err.to_string();
        assert!(msg.contains("cycle"), "the message should point at the likely cause: {msg}");
    }

    #[test]
    fn the_two_arithmetic_paths_disagree_on_overflow_on_purpose() {
        // `apply_gain` saturates and refuses to throw away a frame;
        // `evaluate_chain` refuses and names the node. Both are deliberate.
        //
        // This test exists so that "make them the same" is a decision somebody
        // makes and writes down, rather than a difference that drifts into an
        // accident — which is what the two doc comments managed to do, claiming
        // a single shared rule that never existed.
        let saturated = apply_gain(1e300, 1e300).expect("apply_gain saturates rather than failing");
        assert_eq!(saturated, craftkit_core::MAX_MAGNITUDE);

        let refused = evaluate_chain(ChainInput {
            start: 1.0,
            steps: vec![step("a", 1e300, 0.0), step("b", 1e300, 0.0)],
        })
        .expect_err("the chain refuses rather than saturating");
        assert!(refused.to_string().contains("b"), "{}", refused);
    }

    #[test]
    fn overflow_stops_the_walk_with_a_named_node() {
        let steps = vec![step("huge", 1e300, 0.0), step("next", 1e300, 0.0)];
        let err = evaluate_chain(ChainInput { start: 1.0, steps }).expect_err("must not produce infinity");
        let msg = err.to_string();
        assert!(msg.contains("next"), "the failing node should be named: {msg}");
    }

    #[test]
    fn a_non_finite_start_is_refused() {
        assert!(evaluate_chain(ChainInput { start: f64::NAN, steps: vec![] }).is_err());
    }

    #[test]
    fn evaluation_is_deterministic() {
        let input = || ChainInput { start: 1.5, steps: vec![step("a", 2.0, 0.25), step("b", 0.5, -1.0)] };
        let first = evaluate_chain(input()).expect("ok");
        for _ in 0..8 {
            assert_eq!(evaluate_chain(input()).expect("ok"), first, "evaluation is not deterministic");
        }
    }
}
