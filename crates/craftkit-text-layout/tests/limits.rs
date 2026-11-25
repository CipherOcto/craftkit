//! The **limits of the line breaker**: the paragraphs it will not break
//! optimally.
//!
//! The optimal breaker is quadratic in the number of break opportunities,
//! so there is a number of them past which it is not used, and a limit
//! that no test names is a limit nothing proves holds.

use craftkit_text_layout::{Item, LayoutError, MAX_BREAKS, Method, Params, break_paragraph};

/// A paragraph of `count` words, each one ten wide, separated by glue.
///
/// Every glue follows a box, so every glue is a break opportunity, and
/// the end of the paragraph is one more: a paragraph of `count` words
/// has `count` break opportunities.
fn words(count: usize) -> Vec<Item> {
    let mut items = Vec::with_capacity(count * 2);
    for index in 0..count {
        if index > 0 {
            items.push(Item::glue(1.0, 3.0, 1.0));
        }
        items.push(Item::boxed(10.0));
    }
    items
}

/// A short paragraph is broken by the **optimal** breaker.
///
/// This is the other half of the tests below: a cap that refuses
/// everything is not a cap, it is a wall.
#[test]
fn a_paragraph_within_the_limit_is_broken_optimally() {
    let items = words(10);
    let breaks = break_paragraph(&items, 100.0, &Params::default()).expect("the paragraph is broken");
    assert_eq!(breaks.method, Method::Optimal, "the optimal breaker ran");
    assert!(!breaks.lines.is_empty(), "and it made lines");
}

/// A paragraph of **more break opportunities** than `MAX_BREAKS` is
/// refused.
///
/// The tree does **not** fall back to the greedy breaker here: the
/// optimal breaker refuses the paragraph with
/// [`LayoutError::TooLong`] and says how many break opportunities it
/// found. This test names what the tree does, and it is the truth of
/// the cap: past `MAX_BREAKS` there is no optimal break, and the caller
/// is told so.
#[test]
fn a_paragraph_over_the_limit_is_refused() {
    let items = words(MAX_BREAKS + 1);
    let error = break_paragraph(&items, 100.0, &Params::default()).expect_err("that paragraph is too long");
    match error {
        LayoutError::TooLong { breaks } => {
            assert!(breaks > MAX_BREAKS, "the count is past the bound: {breaks}");
            assert_eq!(breaks, MAX_BREAKS + 1, "and the count is the one that was built");
        }
        other => panic!("the error names the count: {other:?}"),
    }
}

/// A paragraph of **exactly** `MAX_BREAKS` break opportunities is
/// broken: the bound is the number the optimal breaker will look at, and
/// the number itself is still broken.
#[test]
fn a_paragraph_at_the_limit_is_broken() {
    let items = words(MAX_BREAKS);
    let breaks = break_paragraph(&items, 100.0, &Params::default()).expect("the paragraph is broken");
    assert_eq!(breaks.method, Method::Optimal, "the optimal breaker ran");
}

/// The breaker **says which one ran**: a paragraph the optimal breaker
/// cannot fit is broken by the greedy one, and the result says so.
///
/// This is the mechanism `MAX_BREAKS` sits in: the result of a break is
/// a value, and the value carries the method that produced it.
#[test]
fn the_greedy_breaker_says_it_ran() {
    let items = words(3);
    // A measure too narrow for the middle word: no line can hold it.
    let breaks = break_paragraph(&items, 15.0, &Params::default()).expect("the paragraph is broken");
    assert_eq!(breaks.method, Method::Greedy, "the greedy breaker ran");
    assert!(!breaks.lines.is_empty(), "and it made lines");
}
