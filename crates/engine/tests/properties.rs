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

use craftkit_core::{MAX_INSTANT_TICKS, MAX_MAGNITUDE, MAX_TEXT, MIN_INSTANT_TICKS, TICKS_PER_SECOND};
use craftkit_engine::{Doc, EngineError, Session, command, params::PAST_THE_RANGE};
use craftkit_model::limits::{MAX_FRAME_RATE, MAX_NAME, MAX_PATH_BYTES};
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

/// The value that is **past the tree's range**, for the kinds that have one.
///
/// **Derived from the tree's own limits**, so the value cannot drift away
/// from the rule it is testing: a change to `MAX_MAGNITUDE` moves the value
/// with it. A kind whose hostility is its *shape* rather than its size — a
/// colour, a kind, an easing — has no such value, and says so with `None`.
/// §188.
fn past_the_range(kind: Doc) -> Option<Json> {
    match kind {
        Doc::Number => Some(json!(MAX_MAGNITUDE + 1.0)),
        Doc::Text => Some(json!("x".repeat(MAX_TEXT + 1))),
        Doc::Time => an_instant_past_the_range().first().cloned(),
        Doc::NodeRef => Some(json!(u64::MAX)),
        Doc::PropertyPath => Some(json!("x".repeat(MAX_PATH_BYTES + 1))),
        Doc::Bool | Doc::Color | Doc::NodeKind | Doc::Easing => None,
    }
}

/// One second past the tree's range, in each of the spellings a time has.
///
/// The values are **derived from the tree's own bound**: the tree's range for
/// an instant is the range a reply can carry, and that bound is one number
/// of ticks — so the seconds are the bound in seconds, the ticks are the
/// bound in ticks, and the frames are the bound in the frames of the
/// **largest** rate the tree allows, which is the rate at which the bound in
/// frames is the smallest and the value is therefore past the range at every
/// rate a document can have.
///
/// Both ends of the range are there, and both signs of each spelling: a
/// bound that is only checked on one side is half a bound. §188.
fn an_instant_past_the_range() -> Vec<Json> {
    let seconds = MAX_INSTANT_TICKS / TICKS_PER_SECOND + 1;
    let before = MIN_INSTANT_TICKS / TICKS_PER_SECOND - 1;
    let frames = MAX_INSTANT_TICKS / (TICKS_PER_SECOND / i128::from(MAX_FRAME_RATE)) + 1;
    let ticks = MAX_INSTANT_TICKS + 1;
    // Twice the bound, because the end of the range is not a number an
    // `f64` holds exactly: one tick past it would be read back as the bound.
    let ticks_before = MIN_INSTANT_TICKS * 2;
    vec![
        json!(seconds as f64),
        json!(seconds.to_string()),
        json!(format!("{frames}f")),
        json!(format!("{ticks}t")),
        json!(before as f64),
        json!(before.to_string()),
        json!(format!("-{frames}f")),
        json!(format!("{ticks_before}t")),
    ]
}

/// The hostile values for one parameter kind.
///
/// They are the values that reach the command rather than being turned away
/// by the reader — and they are **the tree's own limits, walked**: a number
/// at `MAX_MAGNITUDE` and one past it, a text at `MAX_TEXT` and one past it,
/// an instant past the same bound in **each** of the three spellings, and a
/// fraction of a frame, which is not a count of frames.
///
/// The limits are read from the tree, not written here, so the generator
/// cannot drift away from the rules it is testing: a change to
/// `MAX_MAGNITUDE` moves the value with it. §188.
fn hostile_values(kind: Doc) -> Vec<Json> {
    match kind {
        Doc::Number => vec![
            json!(0),
            json!(-1),
            json!(1e308),
            json!(0.1),
            json!(MAX_MAGNITUDE),
            json!(MAX_MAGNITUDE + 1.0),
            json!(-MAX_MAGNITUDE),
            json!(-MAX_MAGNITUDE - 1.0),
            json!(f64::MIN_POSITIVE),
            json!(5e-324),
        ],
        Doc::Text => vec![
            json!(""),
            json!("   "),
            json!("\u{0}\u{1}"),
            json!("x".repeat(300)),
            json!("x".repeat(MAX_NAME + 1)),
            json!("x".repeat(MAX_TEXT + 1)),
            json!("a\u{0}b"),
            json!("\u{1f600}"),
            json!("\u{feff}x"),
        ],
        Doc::Bool => vec![json!(true), json!(false)],
        Doc::Color => vec![
            json!("#000000"),
            json!("#ffffffff"),
            json!("#gggggg"),
            json!("#fff"),
            json!("ffffff"),
            json!("#FFFFFF"),
            json!(""),
        ],
        Doc::NodeRef => vec![
            json!(999_999),
            json!(u64::MAX),
            json!("n999999"),
            json!("#999999"),
            json!("no such node"),
            json!(-1),
            json!(1.5),
            json!(null),
        ],
        Doc::NodeKind => vec![
            json!("source"),
            json!("comment"),
            json!("not a kind"),
            json!(""),
            json!("Filter"),
            json!("filter "),
        ],
        Doc::Time => {
            let mut values = vec![
                json!(999_999_999_999i64),
                json!(-999_999_999_999i64),
                json!("999999999s"),
                json!("-999999999s"),
                // A fraction of a frame is not a count of frames.
                json!("1.5f"),
                json!("0.5t"),
                // A body with no number in it.
                json!("f"),
                json!("t"),
                // A count with a space in it, and a negative one.
                json!("12 f"),
                json!("-5f"),
            ];
            // The three spellings, each with an instant past the tree's
            // range. The seconds spelling has always refused these; the
            // other two took the nearest representable instant and said
            // nothing. §188.
            values.extend(an_instant_past_the_range());
            values
        }
        Doc::PropertyPath => vec![
            json!("no such property"),
            json!("amount#999"),
            json!("@999999"),
            json!("#999999"),
            json!("tint/deeper/still"),
            json!("enabled#2"),
            json!(""),
            json!("a/"),
            json!("/a"),
            json!("a//b"),
            json!("a".repeat(MAX_NAME + 1)),
            json!("x".repeat(MAX_PATH_BYTES + 1)),
        ],
        Doc::Easing => {
            vec![json!("linear"), json!("step"), json!("not an easing"), json!(""), json!("LINEAR")]
        }
    }
}

/// One hostile value of one kind.
fn hostile(kind: Doc, index: usize) -> Json {
    let values = hostile_values(kind);
    values[index % values.len()].clone()
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
    let kind = spec.params.iter().find(|p| p.name == under).map(|p| p.kind);
    let Some(kind) = kind else { return Json::Object(Map::new()) };
    let times: Vec<&command::ParamDoc> = spec.params.iter().filter(|p| p.kind == Doc::Time).collect();
    let mut map = Map::new();
    let mut seen: usize = 0;
    let mut time_index: usize = 0;
    for param in spec.params {
        let value = if param.name == under {
            if kind == Doc::Time && !times.is_empty() {
                // The next instant parameter's plausible value, so the two ends
                // of a range can be made equal.
                let next = (time_index + 1) % times.len();
                plausible(Doc::Time, next)
            } else {
                hostile(kind, hostile_index)
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

/// The arguments of one call, with a **given** value under test.
///
/// The value is passed in rather than drawn, so that a sweep can fire every
/// value of a kind at every parameter of every command — a random draw is a
/// draw, and a value the generator holds may never be chosen. §188.
///
/// **The value under test is the value.** Nothing is swapped for it and
/// nothing is drawn for it: a refusal that happens for another reason —
/// the end of a range before its start — would satisfy a weaker version of
/// the property and prove nothing. The equal-ends trick lives in
/// [`arguments`], and the two are **different hostile values**: one makes a
/// range's ends equal on purpose, the other is simply out of range, and only
/// the second one reaches the reader. §188.
///
/// **The alternative of the parameter under test is left out.** A parameter
/// with an alternative is one parameter in two spellings (§116), and the
/// reader tries the first spelling first: a call that gives both never
/// reaches the second, so a hostile value fired at the second spelling would
/// never reach the reader, and the property would pass for the wrong reason.
///
/// The pair is named from **one** side only — `node` says its alternative is
/// `node_name`, and `node_name` says nothing — so the other half has to be
/// found by looking the other way as well. Getting this wrong was measured:
/// with only one side left out, eleven commands accepted a hostile name
/// because the reader never got to it.
fn arguments_with(spec: &command::CommandSpec, under: &str, value: Json) -> Json {
    let other = spec
        .params
        .iter()
        .find(|p| p.name == under)
        .and_then(|p| p.alternative)
        .or_else(|| spec.params.iter().find(|p| p.alternative == Some(under)).map(|p| p.name));
    let mut map = Map::new();
    let mut seen: usize = 0;
    for param in spec.params {
        if Some(param.name) == other {
            continue;
        }
        let value = if param.name == under {
            value.clone()
        } else {
            let value = plausible(param.kind, seen);
            seen += 1;
            value
        };
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

/// Every command, every parameter, every hostile value of that kind.
///
/// The property above draws its cases — with a fixed seed, so a CI run can
/// never fail at random — but a draw is a draw: a value the generator holds
/// may never be chosen, and "the generator walks the tree's range" would be a
/// claim nothing backs. This walks all of it, so the claim is the suite's.
///
/// It checks both of the tree's promises about a command: no internal error,
/// and a refused command leaves the document exactly as it was. §188.
///
/// **And it checks that the generator really walks the range.** The value
/// that is past the tree's range has to be one of the values the generator
/// holds — a generator that stopped holding it would pass every other
/// assertion here, and the range would never be walked at all. §188.
#[test]
fn every_command_at_every_parameter_with_every_hostile_value() {
    let mut fired = 0usize;
    let mut expected = 0usize;
    for spec in command::all() {
        for param in spec.params {
            let values = hostile_values(param.kind);
            expected += values.len();
            if let Some(past) = past_the_range(param.kind) {
                assert!(
                    values.contains(&past),
                    "the generator does not hold the value past the tree's range for {}: {past}",
                    param.kind.json_name()
                );
            }
            for value in values {
                fired += 1;
                let arguments = arguments_with(spec, param.name, value.clone());
                let mut session = Session::sample();
                let before = session.document().clone();
                let outcome = session.execute(spec.id, &arguments);
                if let Err(e) = outcome {
                    assert!(
                        !matches!(e, EngineError::Internal { .. }),
                        "`{}` reported an internal error for {} = {value} (the call was {}): {e}",
                        spec.id,
                        param.name,
                        arguments
                    );
                    assert_eq!(
                        session.document(),
                        &before,
                        "`{}` refused and changed the document anyway (the call was {})",
                        spec.id,
                        arguments
                    );
                }
            }
        }
    }
    assert_eq!(
        fired, expected,
        "the sweep fired {fired} cases and expected {expected}; it is not firing every value at every parameter"
    );
    assert!(fired > 100, "the sweep fired only {fired} cases; the generator is not walking");
}

/// A value past the tree's range is refused, at every parameter.
///
/// The tree's limits are the rules the readers keep, and this is the
/// property that says they keep them: a number past `MAX_MAGNITUDE`, a text
/// past `MAX_TEXT`, a node id past the last id the tree can hand out, a path
/// past the longest path — each is refused, at every parameter of every
/// command that takes one, in every command's own words.
///
/// The values come from the tree's own constants, so the property cannot
/// drift away from the limits it is checking. §188.
#[test]
fn a_value_past_the_trees_range_is_refused_at_every_parameter() {
    let mut fired = 0usize;
    for spec in command::all() {
        for param in spec.params {
            let Some(value) = past_the_range(param.kind) else { continue };
            fired += 1;
            let arguments = arguments_with(spec, param.name, value.clone());
            let mut session = Session::sample();
            let outcome = session.execute(spec.id, &arguments);
            assert!(
                outcome.is_err(),
                "`{}` accepted {value} for {}, which is past the tree's range (the call was {arguments})",
                spec.id,
                param.name
            );
        }
    }
    assert!(fired > 0, "no parameter has a range, so the property tested nothing");
}

/// An instant past the tree's range is refused, in **every** spelling.
///
/// The seconds spelling has always refused an instant past
/// `MAX_MAGNITUDE` seconds. The frame and tick spellings did not: they took
/// the nearest representable instant and said nothing, so the same hostile
/// value got three different answers depending on how it was spelled. §188.
///
/// The refusal has to be about the **range of the time**: a command that
/// refused the call for another reason — the end of a range before its
/// start — would satisfy a weaker version of this property and prove
/// nothing. The tree says the same words for every spelling, so the
/// property can look for them.
#[test]
fn an_instant_past_the_trees_range_is_refused_in_every_spelling() {
    let mut fired = 0usize;
    for spec in command::all() {
        for param in spec.params.iter().filter(|p| p.kind == Doc::Time) {
            for value in an_instant_past_the_range() {
                fired += 1;
                let arguments = arguments_with(spec, param.name, value.clone());
                let mut session = Session::sample();
                let error = match session.execute(spec.id, &arguments) {
                    Ok(_) => {
                        panic!("`{}` accepted {value} for {} (the call was {arguments})", spec.id, param.name)
                    }
                    Err(e) => e,
                };
                assert!(
                    error.to_string().contains(PAST_THE_RANGE),
                    "`{}` refused {value} for {}, but not because it was past the tree's range: {error}",
                    spec.id,
                    param.name
                );
            }
        }
    }
    assert!(fired > 0, "no command takes an instant, so the property tested nothing");
}

/// A fraction of a count is refused.
///
/// A frame is a count and a tick is a count, and the tree's rule is that
/// counts have to be exact: `1.5f` used to be truncated to one frame, which
/// is a different instant than the one that was asked for, and nothing said
/// so. §188.
///
/// The seconds spelling is **not** in this property: `1.5` seconds is a
/// perfectly good instant, and a reader that refused it would be refusing
/// the tree's own example.
#[test]
fn a_fraction_of_a_count_is_refused() {
    let fractions = [json!("1.5f"), json!("0.5t"), json!("2.25F"), json!("1.5T")];
    let mut fired = 0usize;
    for spec in command::all() {
        for param in spec.params.iter().filter(|p| p.kind == Doc::Time) {
            for value in fractions.clone() {
                fired += 1;
                let arguments = arguments_with(spec, param.name, value.clone());
                let mut session = Session::sample();
                let error = match session.execute(spec.id, &arguments) {
                    Ok(_) => {
                        panic!("`{}` accepted {value} for {} (the call was {arguments})", spec.id, param.name)
                    }
                    Err(e) => e,
                };
                assert!(
                    error.to_string().contains("is not a time"),
                    "`{}` refused {value} for {}, but not because a count has to be whole: {error}",
                    spec.id,
                    param.name
                );
            }
        }
    }
    assert!(fired > 0, "no command takes an instant, so the property tested nothing");
}
