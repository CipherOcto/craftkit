//! Property tests: the promise, written as a generator.
//!
//! # Why this file exists
//!
//! The tree's rule is that hostile input is **refused**, not survived, and it
//! is checked by hand-written tests: a table of inputs somebody thought of.
//! A table is a list of guesses.
//!
//! This file is the same promise written as a generator — *any* bytes, *any*
//! JSON, *any* limit — so the inputs nobody thought of are covered too.
//!
//! # What is deliberately not here
//!
//! **A property that cannot fail.** `Session::execute` runs every command
//! under a `catch_unwind`, so "no command panics" is already structurally
//! true at that level, and a test of it would prove nothing about the code
//! underneath. The properties here are on the code *below* that boundary —
//! the readers and the parameter readers — where a panic is a real bug.
//!
//! And they are about the **bounds**, not about survival, because a bound is
//! a fact about the output that no amount of luck can fake: a reader that
//! keeps more than its limit has broken its promise whether or not it
//! crashed.
//!
//! # A property that depends on luck is a property that stops catching
//!
//! Where the rule is about a boundary, the generator **builds the boundary**
//! rather than hoping to land on it: a text of exactly the limit and one byte
//! past it, a list one item longer than the cap, a line one byte longer than
//! the limit.
//!
//! This was measured, not assumed. The text property was written first with
//! a random text and a random limit, and it caught a cap that was one byte
//! too large — **with a random seed**. With the fixed seed below it did not,
//! and the mutation was missed: the two numbers had to be drawn independently
//! for the case to be drawn at all. The generator now asks for the text's
//! length in terms of the limit, and the mutation is caught every time.
//!
//! # How many cases, and with which seed
//!
//! `proptest` runs 256 cases by default and shrinks a failure to the
//! smallest input that still breaks it. The generators are capped at 64
//! bytes: the properties are about the *shape* of an input, not its size,
//! and a cap keeps the suite quick.
//!
//! **The seed is fixed, and the failures are not written down.** A CI run
//! must never fail at random: the suites run from a fixed seed, so the same
//! inputs are tried every time and a red build is a real finding. Explore
//! further with `PROPTEST_CASES=5000 PROPTEST_RNG_SEED=<n>`, and keep what
//! you find as a test of its own.
//!
//! The failures are not persisted to a file. A file written by the tool is
//! not a list anybody reviews, and the tree's rule is that a list of things
//! has a reason beside each one. §175.

use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};
use serde_json::{Value as Json, json};

use craftkit_core::{MAX_MAGNITUDE, Params, ReadError, read_line_bounded, read_to_string_bounded};

/// The seed the suites run with, unless `PROPTEST_RNG_SEED` says otherwise.
const SEED: u64 = 0x5eed_9a7f;

/// How many cases each property runs, unless `PROPTEST_CASES` says otherwise.
const CASES: u32 = 256;

/// The configuration every property here runs with.
///
/// **Fixed seed, no persistence.** A random seed would make a CI run fail at
/// random, and a persisted failure file would be a list nobody reads.
///
/// `PROPTEST_CASES` and `PROPTEST_RNG_SEED` are honoured on top of this, by
/// `proptest` itself: the environment is applied to whatever the suite asks
/// for, so a developer can look further without editing anything.
fn config() -> Config {
    Config { failure_persistence: None, rng_seed: RngSeed::Fixed(SEED), cases: CASES, ..Config::default() }
}

/// The suites run from a fixed seed, and the failures are not written down.
///
/// **A test, not a comment.** A CI run must never fail at random, and the
/// way to hold that is to check the configuration rather than to write it
/// down where nobody reads it.
#[test]
fn the_property_suites_run_from_a_fixed_seed_and_write_nothing_down() {
    let c = config();
    assert_eq!(c.rng_seed, RngSeed::Fixed(SEED), "the suites must run from a fixed seed");
    assert!(c.failure_persistence.is_none(), "the failures must not be written to a file");
}

/// Any bytes, and any limit.
fn bytes_and_limit() -> impl Strategy<Value = (Vec<u8>, usize)> {
    (prop::collection::vec(any::<u8>(), 0..64), 0usize..64)
}

/// Any JSON value: the kinds a parameter can hold.
///
/// **Written here, because `proptest` does not generate them.** The crate has
/// no `serde_json` strategy, and a hand-written one is a few lines of
/// `prop_oneof!` rather than a dependency.
fn any_json() -> impl Strategy<Value = Json> {
    prop_oneof![
        Just(Json::Null),
        any::<bool>().prop_map(Json::Bool),
        any::<i64>().prop_map(|n| Json::Number(serde_json::Number::from(n))),
        any::<f64>().prop_filter("a finite number", |f| f.is_finite()).prop_map(|f| {
            match serde_json::Number::from_f64(f) {
                Some(n) => Json::Number(n),
                None => Json::Null,
            }
        }),
        ".*".prop_map(Json::String),
        prop::collection::vec(any::<i64>(), 0..4).prop_map(|items| {
            Json::Array(items.into_iter().map(|n| Json::Number(serde_json::Number::from(n))).collect())
        }),
    ]
}

/// A text of exactly `len` bytes, made of ASCII so that the byte length and
/// the character length are the same.
///
/// The point of the exactness is the boundary: a text whose length is *asked
/// for* lands on the limit and just past it, where a text of a random length
/// lands there by luck.
fn text_of_len(len: usize) -> String {
    let mut bytes: Vec<u8> = Vec::with_capacity(len);
    while bytes.len() < len {
        // A letter, so the text is not a run of spaces.
        bytes.push(b'a' + (bytes.len() % 26) as u8);
    }
    String::from_utf8(bytes).expect("ascii")
}

/// The first line's length, in bytes.
fn first_line_len(data: &[u8]) -> usize {
    data.iter().position(|b| *b == b'\n').unwrap_or(data.len())
}

proptest! {
    #![proptest_config(config())]

    /// A bounded line read answers with a line, with nothing, or with an
    /// error — and what it answers with is the input's own bytes.
    #[test]
    fn a_bounded_line_read_keeps_at_most_the_limit((data, max) in bytes_and_limit()) {
        let mut reader: &[u8] = &data;
        let out = read_line_bounded(&mut reader, max);
        match out {
            Ok(None) => {}
            Ok(Some(line)) => {
                prop_assert!(line.len() <= max, "kept {} bytes with a limit of {}", line.len(), max);
                prop_assert!(data.starts_with(line.as_bytes()), "the line is not the input's own bytes");
            }
            Err(e) => prop_assert!(
                matches!(e, ReadError::TooLong { .. } | ReadError::NotUtf8),
                "a slice cannot fail to read: {:?}",
                e
            ),
        }
    }

    /// The reader stops at the end of the line, whatever the limit was.
    ///
    /// This is the property that makes the reader usable by a server that
    /// carries on after refusing one line: it has to be left at the start of
    /// the next one.
    #[test]
    fn a_bounded_line_read_leaves_the_reader_at_the_next_line((data, max) in bytes_and_limit()) {
        let mut reader: &[u8] = &data;
        let _ = read_line_bounded(&mut reader, max);
        let first = first_line_len(&data);
        let expected: &[u8] = if first < data.len() { &data[first + 1..] } else { &data[data.len()..] };
        prop_assert_eq!(reader, expected, "the reader did not stop at the end of the line");
    }

    /// A line longer than the limit is refused, and the length in the error
    /// is the line's own length — not the limit, and not a guess.
    ///
    /// **The line's length is asked for, not stumbled on**: the generator
    /// makes a line of `max` bytes and of `max + 1` and of `max + 2`, so the
    /// boundary is always one of the cases. And **the line is sometimes
    /// terminated**: the reader counts a line two ways — the bytes before the
    /// newline, and the bytes of a line that ends the input — and a property
    /// that only had one of them would miss a break in the other.
    #[test]
    fn a_line_longer_than_the_limit_is_refused_with_its_true_length(
        max in 0usize..40,
        over in 0usize..3,
        terminated in any::<bool>(),
    ) {
        let mut data = text_of_len(max + over).into_bytes();
        if terminated {
            data.push(b'\n');
            data.extend_from_slice(b"and a tail");
        }
        let first = first_line_len(&data);
        prop_assume!(first > max);
        let mut reader: &[u8] = &data;
        match read_line_bounded(&mut reader, max) {
            Err(ReadError::TooLong { len, max: reported }) => {
                prop_assert_eq!(len, first, "the reported length is not the line's length");
                prop_assert_eq!(reported, max, "the reported limit is not the limit");
            }
            other => prop_assert!(false, "a line of {} bytes was not refused: {:?}", first, other),
        }
    }

    /// The whole-input reader answers with the input, or refuses it — and
    /// never with a truncated version of it.
    #[test]
    fn a_bounded_whole_read_answers_with_the_input_or_refuses_it((data, max) in bytes_and_limit()) {
        let out = read_to_string_bounded(&data[..], max);
        match out {
            Ok(got) => {
                prop_assert_eq!(got.as_bytes(), &data[..], "it did not answer with the whole input");
                prop_assert!(data.len() <= max, "kept {} bytes with a limit of {}", data.len(), max);
            }
            Err(ReadError::TooLong { len, max: reported }) => {
                prop_assert_eq!(len, max + 1, "the reported length is not the limit plus one");
                prop_assert_eq!(reported, max, "the reported limit is not the limit");
            }
            Err(ReadError::Io { .. }) => {
                // It refused because the bytes it read were not text. It reads
                // at most `max + 1` bytes, so that is the part that decides.
                let prefix = &data[..data.len().min(max + 1)];
                prop_assert!(
                    std::str::from_utf8(prefix).is_err(),
                    "the bytes it read were text"
                );
            }
            Err(e) => prop_assert!(false, "unexpected: {:?}", e),
        }
    }

    /// Any JSON value under a key is read or refused, and none of them makes
    /// a reader panic.
    ///
    /// The property is the absence of a panic: a `proptest` case fails when
    /// the body panics, so "no panic" is a result, not the absence of one.
    #[test]
    fn a_parameter_is_read_or_refused(value in any_json()) {
        let p = Params::new(&value);
        let _ = p.num("k");
        let _ = p.int("k");
        let _ = p.bool("k");
        let _ = p.text("k", 16);
        let _ = p.color("k");
        let _ = p.num_list("k", 4);
    }

    /// A text parameter is never longer than its limit, and is refused when
    /// it is.
    ///
    /// **The text's length is asked for, not stumbled on**: the generator
    /// makes a text of exactly the limit's length, and one and two bytes
    /// past it, so the boundary is always one of the cases. A text and a
    /// limit drawn independently hit the boundary only by luck, and this
    /// property was measured missing a mutation because of it.
    #[test]
    fn a_text_parameter_never_exceeds_its_limit(max in 0usize..40, over in 0usize..3) {
        let text = text_of_len(max + over);
        let value = json!({ "k": text });
        let p = Params::new(&value);
        match p.text("k", max) {
            Ok(Some(got)) => {
                prop_assert!(got.len() <= max, "kept {} bytes with a limit of {}", got.len(), max);
                prop_assert_eq!(got, text);
            }
            Ok(None) => prop_assert!(false, "the value was there and was not read"),
            Err(_) => prop_assert!(
                text.len() > max,
                "a text of {} bytes was refused with a limit of {}",
                text.len(),
                max
            ),
        }
    }

    /// A number is finite and within the magnitude the tree stores, and an
    /// integer is whole.
    ///
    /// **The rule is `MAX_MAGNITUDE`, not the number inside `int`.** A number
    /// is refused for being too big before `int` ever sees it, so the bound
    /// written in `int` cannot fire. The property is written against the
    /// bound that is really in force. §175.
    #[test]
    fn a_number_parameter_is_finite_and_an_integer_parameter_is_whole(n in prop::num::f64::NORMAL) {
        let value = json!({ "k": n });
        let p = Params::new(&value);
        let in_range = n.is_finite() && n.abs() <= MAX_MAGNITUDE;
        match p.num("k") {
            Ok(Some(v)) => {
                prop_assert!(v.is_finite(), "{} is not finite", v);
                prop_assert!(v.abs() <= MAX_MAGNITUDE, "{} is out of range", v);
            }
            Ok(None) => prop_assert!(false, "the number was there and was not read"),
            Err(_) => prop_assert!(!in_range, "a number in range was refused"),
        }
        match p.int("k") {
            Ok(Some(v)) => {
                prop_assert!(v.abs() < 9_000_000_000_000_000i64, "{} is out of range", v);
                prop_assert_eq!(v as f64, n, "the integer is not the number that was there");
            }
            Ok(None) => prop_assert!(false, "the number was there and was not read"),
            Err(_) => prop_assert!(
                !in_range || n.fract() != 0.0,
                "a whole number in range was refused"
            ),
        }
    }

    /// A list of numbers is all numbers, or it is refused.
    ///
    /// **The list's length is asked for**: the generator makes lists of two
    /// items either side of the cap, so the cap is always one of the cases.
    #[test]
    fn a_number_list_is_all_numbers_or_is_refused(items in prop::collection::vec(any_json(), 6..11)) {
        let value = json!({ "k": items });
        let p = Params::new(&value);
        match p.num_list("k", 8) {
            Ok(Some(list)) => {
                prop_assert!(list.len() <= 8, "kept {} items with a limit of 8", list.len());
                prop_assert_eq!(list.len(), items.len());
                for (i, v) in list.iter().enumerate() {
                    prop_assert!(v.is_finite(), "item {} is {}", i, v);
                    if let Some(expected) = items[i].as_f64() {
                        prop_assert_eq!(*v, expected, "item {} was read as {}", i, v);
                    }
                }
            }
            Ok(None) => prop_assert!(false, "the list was there and was not read"),
            Err(_) => {}
        }
    }

    /// A colour is three or four channels, and it is refused when a channel
    /// is out of range.
    ///
    /// **The channels it answers with are the ones it was given** — that is
    /// the property. Saying "every channel is in 0..=255" would be a
    /// comparison that cannot fail, because a channel *is* a `u8`: clippy
    /// refused the first version of this test for exactly that, and it was
    /// right.
    ///
    /// **The number of channels is asked for**: two, three, four or five, so
    /// the shape the reader accepts is always one of the cases.
    #[test]
    fn a_colour_parameter_is_three_or_four_channels_and_is_refused_when_one_is_out_of_range(
        count in 2usize..6,
        channel in 0u16..=300,
    ) {
        let channels: Vec<u16> = (0..count).map(|i| if i == 0 { channel } else { 0 }).collect();
        let value = json!({ "k": channels });
        let p = Params::new(&value);
        match p.color("k") {
            Ok(Some(colour)) => {
                prop_assert!(
                    channels.len() == 3 || channels.len() == 4,
                    "it answered for {} channels",
                    channels.len()
                );
                for c in channels.iter() {
                    prop_assert!(*c <= 255, "it answered for a channel of {}", c);
                }
                prop_assert_eq!(
                    [colour.r, colour.g, colour.b],
                    [channels[0] as u8, channels[1] as u8, channels[2] as u8],
                    "the channels are not the ones it was given"
                );
            }
            Ok(None) => prop_assert!(false, "the value was there and was not read"),
            Err(_) => {
                let out_of_range = (channels.len() != 3 && channels.len() != 4)
                    || channels.iter().any(|c| *c > 255);
                prop_assert!(out_of_range, "it refused a colour of {:?}", channels);
            }
        }
    }
}
