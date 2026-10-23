//! The commands' promise, written as a property.
//!
//! The tree's promise is that a hostile parameter is **refused**, not that it
//! crashes. The hand-written test (`hostile_params_never_panic`) fires a
//! fixed set of values at every parameter name; the names are complete,
//! because they come from the registry, and the values are guesses. §175.
//!
//! This file fires **generated** values at every parameter of every command,
//! and it looks at the answer rather than at the crash.
//!
//! # Looking through the guard
//!
//! `Session::execute` runs every command under a `catch_unwind`, so a panic
//! in a command does not reach the caller — the guard turns it into an
//! [`EngineError::Internal`]. That is why "no command panics" is a property
//! that cannot fail (§175): the guard has already made it true.
//!
//! **This property looks through the guard.** It forbids the *error the guard
//! produces*, so a panic that the guard caught is a failure of the property.
//! The guard hides the panic from the process; it cannot hide it from this.
//!
//! The other two kinds of `Internal` are forbidden with it, and they are
//! worth forbidding for the same reason: a command that leaves the document
//! in a state the file format refuses, and a command that says it does not
//! change the document and does, are both bugs that no caller can see.
//!
//! # What is deliberately not here
//!
//! **A property about the document that the engine already keeps.** The
//! engine restores the document when a command refuses, and the property
//! that says so is here — but it is a property about the *engine's* promise,
//! not about the commands, and it is checked by the engine's own tests. §177.

use craftkit_engine::{Doc, EngineError, Session, command};
use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};
use serde_json::{Map, Value as Json, json};

/// The seed the suites run from.
///
/// **Fixed**, so a CI run can never fail at random, and the failures are not
/// written to a file: a file the tool writes is a list nobody reviews.
/// §175's rule, and the family's practice. §177.
const SEED: u64 = 0x5eed_9a7f;

/// How many cases a suite runs.
const CASES: u32 = 256;

/// The configuration every suite here runs with.
fn config() -> Config {
    Config { failure_persistence: None, rng_seed: RngSeed::Fixed(SEED), cases: CASES, ..Config::default() }
}

/// A value of the right kind that the command will accept.
///
/// The point of it: only the parameter under test is hostile, so a refusal
/// that happens is about the hostile value and not about the others.
///
/// **The index matters.** The plausible value of the *n*th parameter of a
/// kind is different from the first one's, because two parameters of the same
/// kind are usually two ends of a range: `doc.range` takes a `start` and an
/// `end`, and if both were zero the command would refuse *"the end must be
/// after the start"* before it ever looked at the frame rate. The hostile
/// value would then never reach the code it is meant to test, and the
/// property would pass for the wrong reason. §175's lesson, the second time.
///
/// §177.
fn plausible(kind: Doc, index: usize) -> Json {
    match kind {
        Doc::Number => json!(1.0),
        Doc::Text => json!("Probe"),
        Doc::Bool => json!(true),
        Doc::Color => json!("#ffaa80"),
        Doc::NodeRef => json!(1),
        Doc::NodeKind => json!("filter"),
        // An instant, later for each parameter of this kind: a range's end
        // comes after its start, and a time is read at a rate.
        Doc::Time => json!(index as i64 * 1_000),
        Doc::PropertyPath => json!("amount"),
        Doc::Easing => json!("linear"),
    }
}

/// The hostile values for one parameter kind.
///
/// They are the values that reach the command rather than being turned away
/// by the reader: a node that does not exist, a path that names nothing, a
/// path that is deeper than the document, an instant far outside the range.
fn hostile(kind: Doc, index: usize) -> Json {
    match kind {
        Doc::Number => match index % 4 {
            0 => json!(0),
            1 => json!(-1),
            2 => json!(1e308),
            _ => json!(0.1),
        },
        Doc::Text => match index % 4 {
            0 => json!(""),
            1 => json!("   "),
            2 => json!("\u{0}\u{1}"),
            _ => json!("x".repeat(300)),
        },
        Doc::Bool => json!(index.is_multiple_of(2)),
        Doc::Color => match index % 3 {
            0 => json!("#000000"),
            1 => json!("#ffffffff"),
            _ => json!("#gggggg"),
        },
        Doc::NodeRef => match index % 5 {
            0 => json!(999_999),
            1 => json!(u64::MAX),
            2 => json!("n999999"),
            3 => json!("#999999"),
            _ => json!("no such node"),
        },
        Doc::NodeKind => match index % 3 {
            0 => json!("source"),
            1 => json!("comment"),
            _ => json!("not a kind"),
        },
        Doc::Time => match index % 4 {
            0 => json!(999_999_999_999i64),
            1 => json!(-999_999_999_999i64),
            2 => json!("999999999s"),
            _ => json!("-999999999s"),
        },
        Doc::PropertyPath => match index % 6 {
            0 => json!("no such property"),
            1 => json!("amount#999"),
            2 => json!("@999999"),
            3 => json!("#999999"),
            4 => json!("tint/deeper/still"),
            _ => json!("enabled#2"),
        },
        Doc::Easing => match index % 3 {
            0 => json!("linear"),
            1 => json!("step"),
            _ => json!("not an easing"),
        },
    }
}

/// The arguments of one call: every parameter of the command, the one under
/// test hostile and the rest plausible.
///
/// The hostile value of an **instant** is built, not drawn: it is the
/// plausible value of *another* instant parameter of the same command, so
/// that a range's two ends can be made equal — the boundary `doc.range`
/// refuses. A generator that hoped to land there would pass for the wrong
/// reason most of the time, and the mutation that allows an empty range
/// would be missed. §175's lesson, the second time. §177.
fn arguments(spec: &command::CommandSpec, under: &str, hostile_index: usize) -> Json {
    let times: Vec<&command::ParamDoc> = spec.params.iter().filter(|p| p.kind == Doc::Time).collect();
    let mut map = Map::new();
    let mut seen: usize = 0;
    let mut time_index: usize = 0;
    for param in spec.params {
        let value = if param.name == under {
            if param.kind == Doc::Time && !times.is_empty() {
                // The next instant parameter's plausible value, so the two ends
                // of a range can be made equal.
                let next = (time_index + 1) % times.len();
                plausible(Doc::Time, next)
            } else {
                hostile(param.kind, hostile_index)
            }
        } else {
            let value = plausible(param.kind, seen);
            seen += 1;
            value
        };
        if param.kind == Doc::Time {
            time_index += 1;
        }
        map.insert(param.name.to_string(), value);
    }
    Json::Object(map)
}

proptest! {
    #![proptest_config(config())]

    /// No command, with any generated parameter value, reports an internal
    /// error.
    ///
    /// An internal error is what the engine's guard produces when a command
    /// panics, and it is also what the engine produces when a command leaves
    /// the document in a state the file format refuses, or changes a document
    /// it said it would not change. All three are bugs, and all three are
    /// invisible to the caller — so the property looks at the error rather
    /// than at the absence of a crash.
    ///
    /// §177.
    #[test]
    fn no_command_reports_an_internal_error(
        command_index in 0usize..64,
        parameter_index in 0usize..16,
        hostile_index in 0usize..8,
    ) {
        let specs = command::all();
        let spec = specs[command_index % specs.len()];
        if spec.params.is_empty() {
            return Err(TestCaseError::reject("the command takes no parameters"));
        }
        let param = spec.params[parameter_index % spec.params.len()];
        let value = hostile(param.kind, hostile_index);
        let arguments = arguments(spec, param.name, hostile_index);

        let mut session = Session::sample();
        let outcome = session.execute(spec.id, &arguments);
        if let Err(e) = outcome {
            prop_assert!(
                !matches!(e, EngineError::Internal { .. }),
                "`{}` reported an internal error for {} = {value} (the call was {}): {e}",
                spec.id,
                param.name,
                arguments
            );
        }
    }

    /// A refused command leaves the document exactly as it was.
    ///
    /// The engine takes a snapshot before every command and puts it back when
    /// the command refuses — including when the command *panics*. The
    /// property is the other side of that promise: a half-applied edit is
    /// worse than a failed one, and a caller that reads the refusal must be
    /// able to trust that nothing changed.
    ///
    /// §177.
    #[test]
    fn a_refused_command_leaves_the_document_exactly_as_it_was(
        command_index in 0usize..64,
        parameter_index in 0usize..16,
        hostile_index in 0usize..8,
    ) {
        let specs = command::all();
        let spec = specs[command_index % specs.len()];
        if spec.params.is_empty() {
            return Err(TestCaseError::reject("the command takes no parameters"));
        }
        let param = spec.params[parameter_index % spec.params.len()];
        let arguments = arguments(spec, param.name, hostile_index);

        let mut session = Session::sample();
        let before = session.document().clone();
        let outcome = session.execute(spec.id, &arguments);
        if outcome.is_err() {
            prop_assert_eq!(
                session.document(),
                &before,
                "`{}` refused and changed the document anyway (the call was {})",
                spec.id,
                arguments
            );
        }
    }
}
