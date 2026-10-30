//! `craftkit-text-layout` — **line breaking** on the box/glue/penalty model.
//!
//! # What this is
//!
//! A paragraph is broken into lines. The classic way to do it is Knuth
//! and Plass's model: the paragraph is a sequence of **items** —
//!
//! - a **box**: fixed material, a word or a glyph. Never a break;
//! - **glue**: elastic space. A break opportunity when it follows a box;
//! - a **penalty**: a break opportunity with a cost. A hyphenation point
//!   is a penalty that adds a hyphen's width when it is taken.
//!
//! — and the breaker chooses the set of breaks that makes the whole
//! paragraph look best: no line too loose, no line too tight, and no two
//! lines of very different looseness next to each other.
//!
//! This crate is that breaker: an **optimal** one (the whole paragraph
//! is minimised at once) and a **greedy** one (each line takes as much
//! as fits), and a way to ask which one produced the lines.
//!
//! # Why it is a library and not a plugin
//!
//! Because the kit has no **font**. A line breaker needs the advance
//! of every glyph, and an advance comes from a typeface, and this tree
//! has no typeface and no asset of any kind (the outer tree's rule: no
//! artwork, no fonts, nothing that is not drawn in code).
//!
//! So the widths come from **the caller**: the library takes items with
//! widths already in them, and the battery that uses it asks the caller
//! for a measure and an advance. That is the honest shape, and it is
//! also the useful one: the moment a font arrives, it supplies the
//! widths and nothing else changes.
//!
//! # The units
//!
//! The widths are **in the caller's units**. The kit's unit is the
//! millimetre, so a caller in the kit passes millimetres; the library
//! does not care, and it says so.
//!
//! # What is deliberately not here
//!
//! - **A varying measure.** One paragraph has one measure, the width of
//!   its column. A first line that is narrower (an indent, a drop cap)
//!   makes the optimal breaker quadratic in the *number of lines* as
//!   well as in the number of breaks, and that is not built yet.
//! - **A font.** There is none to have: see above.
//! - **Hyphenation.** The breaker knows a **hyphenation point** — a
//!   flagged penalty — but it does not find them. Finding them is a
//!   dictionary's business.
//!
//! # The input is hostile
//!
//! Widths, costs and measures arrive from outside. A width that is
//! not a number, a measure that is negative: the breaker **refuses**
//! them rather than producing lines that are nonsense. The refusal is
//! a value, not a panic.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

use std::ops::Range;

/// A penalty at least this high forbids a break; one at most its
/// negative forces a break.
pub const INFINITE_PENALTY: f64 = 10_000.0;

/// The cost of an emergency break: high, so that a paragraph takes no
/// more of them than it must, but legal.
pub const EMERGENCY_PENALTY: f64 = 5_000.0;

/// What an [`Item`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Fixed material — a word, a glyph. Never a break.
    Box,
    /// Elastic space. A break when it follows a box.
    Glue,
    /// A break opportunity with a cost.
    Penalty,
}

/// One item of a paragraph.
///
/// The constructors are the way in, and the fields are public so that a
/// caller can read them back — a breaker is a function, and its result
/// has to say what it did.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Item {
    /// What this item is.
    pub kind: Kind,
    /// The natural width, in the caller's units. A penalty's width is
    /// what a line broken here **gains** — a hyphen's width.
    pub width: f64,
    /// How much this item may be stretched.
    pub stretch: f64,
    /// How much this item may be shrunk.
    pub shrink: f64,
    /// A penalty's cost. [`INFINITE_PENALTY`] forbids the break; a
    /// negative cost of [`INFINITE_PENALTY`] or less forces it.
    pub cost: f64,
    /// A penalty: a break that was flagged — a hyphenation point.
    /// Consecutive flagged breaks cost [`Params::flagged_demerits`].
    pub flagged: bool,
    /// A penalty: an automatic hyphenation point, which a caller may
    /// keep out of the last fraction of a line.
    pub hyphenation: bool,
}

impl Item {
    /// A box of `width`.
    pub const fn boxed(width: f64) -> Item {
        Item {
            kind: Kind::Box,
            width,
            stretch: 0.0,
            shrink: 0.0,
            cost: 0.0,
            flagged: false,
            hyphenation: false,
        }
    }

    /// Glue of natural `width`, with `stretch` and `shrink`.
    pub const fn glue(width: f64, stretch: f64, shrink: f64) -> Item {
        Item { kind: Kind::Glue, width, stretch, shrink, cost: 0.0, flagged: false, hyphenation: false }
    }

    /// Glue of no width and infinite stretch: it takes all the leftover
    /// space of the line it is in.
    pub const fn fill() -> Item {
        Item {
            kind: Kind::Glue,
            width: 0.0,
            stretch: f64::INFINITY,
            shrink: 0.0,
            cost: 0.0,
            flagged: false,
            hyphenation: false,
        }
    }

    /// A break opportunity costing `cost`, adding `width` when taken.
    pub const fn penalty(width: f64, cost: f64, flagged: bool) -> Item {
        Item { kind: Kind::Penalty, width, stretch: 0.0, shrink: 0.0, cost, flagged, hyphenation: false }
    }

    /// A forced break: the line ends here.
    pub const fn forced() -> Item {
        Item::penalty(0.0, -INFINITE_PENALTY, false)
    }

    /// A break that is never taken.
    pub const fn no_break() -> Item {
        Item::penalty(0.0, INFINITE_PENALTY, false)
    }

    /// A hyphenation point: a flagged break that adds a hyphen's width.
    pub fn hyphen(width: f64) -> Item {
        Item { hyphenation: true, ..Item::penalty(width, 50.0, true) }
    }

    /// An emergency break inside a word too wide for any line.
    pub fn emergency() -> Item {
        Item {
            kind: Kind::Penalty,
            width: 0.0,
            stretch: 0.0,
            shrink: 0.0,
            cost: EMERGENCY_PENALTY,
            flagged: false,
            hyphenation: false,
        }
    }

    /// Is this a forced break?
    pub fn is_forced(&self) -> bool {
        self.kind == Kind::Penalty && self.cost <= -INFINITE_PENALTY
    }
}

/// Ragged (unjustified) setting.
///
/// A ragged line keeps its spaces and is rated by the space it leaves
/// at the end, against a **rag**: a stretch in width units, plus a
/// fraction of the measure.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rag {
    /// The stretch every ragged line is rated against.
    pub stretch: f64,
    /// The stretch every ragged line is rated against, as a fraction of
    /// its measure.
    pub width_fraction: f64,
}

impl Default for Rag {
    fn default() -> Self {
        Rag { stretch: 0.0, width_fraction: 1.0 / 6.0 }
    }
}

/// How lines are rated and chosen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Params {
    /// The largest badness a line may have. Badness is `100·|r|³`, where
    /// `r` is the line's adjustment ratio, so a line that has to stretch
    /// by more than about half again is beyond it.
    pub max_badness: f64,
    /// When a line cannot be fitted at all, take it anyway: the
    /// paragraph is broken, and the overfull line is reported.
    pub overfull: bool,
    /// Added to every line's badness before squaring.
    pub line_penalty: f64,
    /// Added when two lines in a row end at flagged breaks.
    pub flagged_demerits: f64,
    /// Added when two neighbouring lines are of very different looseness.
    pub fitness_demerits: f64,
    /// The most flagged breaks in a row (0 = no limit). A hard
    /// constraint: a break that would exceed it is not taken.
    pub hyphen_limit: usize,
    /// A line that is longer than its measure by at most this fits.
    pub overrun: f64,
    /// Ragged setting; `None` justifies.
    pub rag: Option<Rag>,
}

impl Default for Params {
    fn default() -> Self {
        Params {
            max_badness: 10_000.0,
            overfull: false,
            line_penalty: 10.0,
            flagged_demerits: 3000.0,
            fitness_demerits: 3000.0,
            hyphen_limit: 0,
            overrun: 1e-6,
            rag: None,
        }
    }
}

/// The number of fitness classes: tight, decent, loose, very loose.
const CLASSES: usize = 4;

/// The badness of a line that cannot be set at all: too long and with
/// nothing to shrink, or too short and with nothing to stretch.
///
/// It is large on purpose: such a line is legal only when the caller
/// allows it, and then it is worse than any number of lines that fit.
const TERRIBLE_BADNESS: f64 = 10_000.0;

/// One line chosen by a breaker.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Line {
    /// The first item of the line.
    pub start: usize,
    /// The break that ends the line: the items `start..end` are the
    /// line's material. The item at `end` is the break itself.
    pub end: usize,
    /// The first item of the next line; the glue and the penalties after
    /// a break are discarded.
    pub next: usize,
    /// The line's natural width, plus the break's width when the break
    /// is a penalty — a hyphen's width.
    pub width: f64,
    /// The measure the line was measured against.
    pub measure: f64,
    /// The adjustment ratio: positive stretches, negative shrinks.
    pub ratio: f64,
    /// The width the line leaves over: the leftover of a justified
    /// line, the rag of a ragged one, and the overfull of a line that
    /// does not fit (negative).
    pub excess: f64,
    /// The line ended at a flagged break — a hyphenation point.
    pub flagged: bool,
    /// The line ended at a forced break or at the end of the paragraph.
    pub forced: bool,
}

/// Which breaker produced a paragraph's lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    /// The optimal breaker: the whole paragraph was minimised at once.
    Optimal,
    /// The greedy breaker: each line took as much as fit. Used when the
    /// optimal breaker could not fit the paragraph, or when the
    /// paragraph is too long for it.
    Greedy,
}

/// The lines of a paragraph.
#[derive(Clone, Debug, PartialEq)]
pub struct Breaks {
    /// The lines, in order.
    pub lines: Vec<Line>,
    /// Which breaker produced them.
    pub method: Method,
}

/// Why a paragraph could not be broken.
#[derive(Clone, Debug, PartialEq)]
pub enum LayoutError {
    /// An item's width, stretch or shrink was not a number.
    NotFinite {
        /// Which item it was.
        index: usize,
    },
    /// The measure was not a number, or was negative.
    BadMeasure,
    /// The paragraph had more break opportunities than the optimal
    /// breaker will look at. The greedy breaker is used instead.
    TooLong {
        /// How many there were.
        breaks: usize,
    },
}

impl std::fmt::Display for LayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LayoutError::NotFinite { index } => {
                write!(f, "item {index} has a width that is not a number")
            }
            LayoutError::BadMeasure => write!(f, "the measure is not a number"),
            LayoutError::TooLong { breaks } => {
                write!(f, "the paragraph has {breaks} break opportunities, which is too many")
            }
        }
    }
}

impl std::error::Error for LayoutError {}

/// The most break opportunities the optimal breaker will look at.
///
/// The optimal breaker is quadratic in the number of breaks, and a
/// paragraph with ten thousand of them would take a while. The greedy
/// breaker is used past this number, and the paragraph says so.
pub const MAX_BREAKS: usize = 5_000;

/// A number that is a number.
fn finite(value: f64) -> bool {
    value.is_finite()
}

/// The prefix sums of a paragraph: the width, the stretch and the
/// shrink of the boxes and the glue before each item.
struct Sums {
    width: Vec<f64>,
    stretch: Vec<f64>,
    shrink: Vec<f64>,
}

impl Sums {
    fn new(items: &[Item]) -> Sums {
        let mut width = Vec::with_capacity(items.len() + 1);
        let mut stretch = Vec::with_capacity(items.len() + 1);
        let mut shrink = Vec::with_capacity(items.len() + 1);
        let (mut w, mut s, mut sh) = (0.0, 0.0, 0.0);
        for item in items {
            width.push(w);
            stretch.push(s);
            shrink.push(sh);
            if item.kind != Kind::Penalty {
                w += item.width;
                s += item.stretch;
                sh += item.shrink;
            }
        }
        width.push(w);
        stretch.push(s);
        shrink.push(sh);
        Sums { width, stretch, shrink }
    }

    /// The material of a line that starts after the break at `from` and
    /// ends at the break `to`.
    ///
    /// `from` is `None` for the first line of a paragraph. The item at
    /// `from` is the break itself and is not part of the line; the glue
    /// and the penalties right after it are discarded, as they are after
    /// every break. The item at `to` is the break too: when it is a
    /// glue, the space at the break is dropped, and when it is a
    /// penalty, its width — a hyphen's — is added to the line.
    fn line(&self, items: &[Item], from: Option<usize>, to: usize) -> (f64, f64, f64) {
        let mut start = match from {
            Some(from) => from + 1,
            None => 0,
        };
        while start < items.len() && items[start].kind != Kind::Box {
            start += 1;
        }
        let end = to.min(items.len());
        let mut width = self.width[end] - self.width[start];
        let stretch = self.stretch[end] - self.stretch[start];
        let shrink = self.shrink[end] - self.shrink[start];
        if let Some(item) = items.get(end)
            && item.kind == Kind::Penalty
        {
            width += item.width;
        }
        (width, stretch, shrink)
    }
}

/// Is `b` a break opportunity?
fn is_break(items: &[Item], b: usize) -> bool {
    match items.get(b) {
        None => true,
        Some(item) => match item.kind {
            Kind::Box => false,
            Kind::Penalty => item.cost < INFINITE_PENALTY,
            Kind::Glue => b > 0 && items.get(b - 1).is_some_and(|before| before.kind == Kind::Box),
        },
    }
}

/// The first item of the line after a break at `b`: the glue and the
/// penalties are discarded up to the next box or forced break.
fn next_start(items: &[Item], b: usize) -> usize {
    let mut index = b + 1;
    while let Some(item) = items.get(index) {
        if item.kind == Kind::Box || item.is_forced() {
            break;
        }
        index += 1;
    }
    index
}

/// The fitness class of a line with the given ratio.
fn fitness_class(ratio: f64) -> usize {
    if ratio < -0.5 {
        0
    } else if ratio <= 0.5 {
        1
    } else if ratio <= 1.0 {
        2
    } else {
        3
    }
}

/// How a line fits its measure.
struct Fit {
    ratio: f64,
    excess: f64,
    badness: f64,
    class: usize,
    feasible: bool,
}

/// Measure a line against its measure.
///
/// `last` says that the line ends the paragraph: its leftover space is
/// not a fault, because the paragraph's last line is not justified.
fn fit(width: f64, stretch: f64, shrink: f64, measure: f64, params: &Params, last: bool) -> Fit {
    let excess = measure - width;
    if last && excess >= 0.0 {
        // The last line takes the leftover: it is not stretched, and the
        // space it leaves is not a fault.
        return Fit { ratio: 0.0, excess, badness: 0.0, class: 1, feasible: true };
    }
    if let Some(rag) = params.rag {
        // Ragged: the space is the rag, and the line is rated against it.
        if excess < 0.0 {
            // Too long: a ragged line is never shrunk.
            if params.overfull {
                let ratio = -overfull_ratio(-excess, shrink, measure);
                let badness = 100.0 * ratio.abs().powi(3);
                return Fit { ratio, excess, badness, class: fitness_class(ratio), feasible: true };
            }
            return Fit { ratio: -1.0, excess, badness: f64::INFINITY, class: 0, feasible: false };
        }
        let unit = rag.stretch + rag.width_fraction * measure;
        let ratio = if unit > 0.0 { excess / unit } else { 0.0 };
        let badness = 100.0 * ratio.powi(3);
        return Fit { ratio, excess, badness, class: fitness_class(ratio), feasible: true };
    }
    if excess >= 0.0 {
        if stretch > 0.0 {
            let ratio = excess / stretch;
            let badness = 100.0 * ratio.powi(3);
            if badness > params.max_badness {
                return Fit { ratio, excess, badness, class: fitness_class(ratio), feasible: false };
            }
            return Fit { ratio, excess, badness, class: fitness_class(ratio), feasible: true };
        }
        // Nothing to stretch with: the line fits only if it is within
        // the overrun.
        if excess <= params.overrun {
            return Fit { ratio: 0.0, excess, badness: 0.0, class: 1, feasible: true };
        }
        // Underfull and unable to stretch: legal only if overfull lines
        // are allowed, and then it is a bad line.
        if params.overfull {
            return Fit { ratio: 0.0, excess, badness: TERRIBLE_BADNESS, class: 1, feasible: true };
        }
        return Fit { ratio: 0.0, excess, badness: f64::INFINITY, class: 1, feasible: false };
    }
    // Too long: shrink it, if there is anything to shrink.
    let over = -excess;
    if shrink > 0.0 {
        let ratio = -over / shrink;
        if ratio < -1.0 {
            if params.overfull {
                let badness = 100.0 * ratio.abs().powi(3);
                return Fit { ratio, excess, badness, class: fitness_class(ratio), feasible: true };
            }
            return Fit { ratio, excess, badness: f64::INFINITY, class: 0, feasible: false };
        }
        let badness = 100.0 * ratio.powi(3);
        if badness > params.max_badness {
            return Fit { ratio, excess, badness, class: fitness_class(ratio), feasible: false };
        }
        return Fit { ratio, excess, badness, class: fitness_class(ratio), feasible: true };
    }
    if params.overfull {
        let ratio = -overfull_ratio(over, shrink, measure);
        let badness = 100.0 * ratio.abs().powi(3);
        return Fit { ratio, excess, badness, class: fitness_class(ratio), feasible: true };
    }
    Fit { ratio: -1.0, excess, badness: f64::INFINITY, class: 0, feasible: false }
}

/// The ratio of a line that does not fit and has nothing to shrink.
///
/// It is rated against the measure, so that a line that is far too long
/// is worse than one that is a little too long.
fn overfull_ratio(over: f64, shrink: f64, measure: f64) -> f64 {
    let unit = if shrink > 0.0 {
        shrink
    } else if measure > 0.0 {
        measure
    } else {
        1.0
    };
    -(over / unit)
}

/// The state of a path: the last line's fitness class, and the number
/// of flagged breaks in a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct State {
    class: usize,
    flagged: usize,
}

/// The best way to reach a break: its demerits, and where it came from.
#[derive(Clone, Copy, Debug)]
struct Best {
    demerits: f64,
    /// The break it came from, as an index into the candidates.
    from: Option<usize>,
    /// The slot that break's own state was in.
    prev_slot: Option<usize>,
    /// The state of the line that ended here.
    state: Option<State>,
}

/// Break a paragraph into lines.
///
/// The **optimal** breaker minimises the whole paragraph at once: it
/// chooses the set of breaks that makes the fewest demerits. When it
/// cannot fit the paragraph — or the paragraph has more break
/// opportunities than [`MAX_BREAKS`] — the **greedy** breaker is used,
/// and [`Breaks::method`] says which one ran.
///
/// The measure is the width of the column, in the caller's units.
pub fn break_paragraph(items: &[Item], measure: f64, params: &Params) -> Result<Breaks, LayoutError> {
    check_items(items)?;
    check_measure(measure)?;
    if items.is_empty() {
        return Ok(Breaks { lines: Vec::new(), method: Method::Optimal });
    }
    match optimal(items, measure, params) {
        Ok(Some(lines)) => Ok(Breaks { lines, method: Method::Optimal }),
        Ok(None) => Ok(Breaks { lines: greedy(items, measure, params), method: Method::Greedy }),
        Err(error) => Err(error),
    }
}

/// Refuse a paragraph whose items are not numbers.
fn check_items(items: &[Item]) -> Result<(), LayoutError> {
    for (index, item) in items.iter().enumerate() {
        if !finite(item.width) || !finite(item.stretch) || !finite(item.shrink) || !finite(item.cost) {
            return Err(LayoutError::NotFinite { index });
        }
    }
    Ok(())
}

/// Refuse a measure that is not a measure.
fn check_measure(measure: f64) -> Result<(), LayoutError> {
    if !finite(measure) || measure < 0.0 {
        return Err(LayoutError::BadMeasure);
    }
    Ok(())
}

/// The optimal breaker: the whole paragraph minimised at once.
///
/// It is a dynamic program over the break opportunities. Each break
/// keeps the best path to it, for each fitness class of the line that
/// ended there — and, when there is a limit on hyphenation, for each
/// number of flagged breaks in a row.
fn optimal(items: &[Item], measure: f64, params: &Params) -> Result<Option<Vec<Line>>, LayoutError> {
    let breaks: Vec<usize> = (0..=items.len()).filter(|b| is_break(items, *b)).collect();
    if breaks.len() > MAX_BREAKS {
        return Err(LayoutError::TooLong { breaks: breaks.len() });
    }
    let ctx = Context { items, sums: Sums::new(items), breaks: &breaks, measure, params };
    let mut lines = Vec::new();
    // The segments of the paragraph: a forced break ends one.
    let mut start = 0usize;
    loop {
        let forced = (start..items.len()).find(|index| items[*index].is_forced());
        let (end, last) = match forced {
            Some(index) => (index, index + 1 == items.len()),
            None => (items.len(), true),
        };
        let segment = break_segment(&ctx, start, end, last)?;
        if segment.is_empty() && end > start {
            // The segment has material and no way to break it: the
            // greedy breaker is better than losing the material.
            return Ok(None);
        }
        lines.extend(segment);
        let Some(index) = forced else { break };
        start = index + 1;
    }
    if lines.is_empty() {
        return Ok(None);
    }
    Ok(Some(lines))
}

/// What a segment is broken against: the paragraph, its sums, its break
/// opportunities, the measure and the parameters.
struct Context<'a> {
    items: &'a [Item],
    sums: Sums,
    breaks: &'a [usize],
    measure: f64,
    params: &'a Params,
}

/// Break one segment of a paragraph: the items from `start` to `end`,
/// where `end` is a forced break or the end of the paragraph.
///
/// `last` says that the segment ends the paragraph, so its last line is
/// not justified.
fn break_segment(ctx: &Context<'_>, start: usize, end: usize, last: bool) -> Result<Vec<Line>, LayoutError> {
    let items = ctx.items;
    let sums = &ctx.sums;
    let breaks = ctx.breaks;
    let measure = ctx.measure;
    let params = ctx.params;
    // The break opportunities of this segment.
    let candidates: Vec<usize> = breaks.iter().copied().filter(|b| *b > start && *b <= end).collect();
    if candidates.is_empty() {
        return Ok(Vec::new());
    }
    let flagged_cap = if params.hyphen_limit == 0 { 0 } else { params.hyphen_limit };
    let mut best: Vec<Vec<Option<Best>>> = vec![vec![None; CLASSES * (flagged_cap + 1)]; candidates.len()];
    // The first line of the segment: it starts at the segment's start.
    let first_from = if start == 0 { None } else { Some(start - 1) };
    for (position, candidate) in candidates.iter().enumerate() {
        let (width, stretch, shrink) = sums.line(items, first_from, *candidate);
        let is_last = last && *candidate == end;
        let fit = fit(width, stretch, shrink, measure, params, is_last);
        if !fit.feasible {
            continue;
        }
        let demerits = (params.line_penalty + fit.badness).powi(2);
        let flagged = usize::from(items.get(*candidate).is_some_and(|item| item.flagged));
        let slot = fit.class * (flagged_cap + 1) + flagged.min(flagged_cap);
        best[position][slot] = Some(Best {
            demerits,
            from: None,
            prev_slot: None,
            state: Some(State { class: fit.class, flagged }),
        });
    }
    // Every later line: from a break that came before.
    for position in 1..candidates.len() {
        let candidate = candidates[position];
        let flagged_here = items.get(candidate).is_some_and(|item| item.flagged);
        for previous in 0..position {
            let from = candidates[previous];
            let (width, stretch, shrink) = sums.line(items, Some(from), candidate);
            let is_last = last && candidate == end;
            let fit = fit(width, stretch, shrink, measure, params, is_last);
            if !fit.feasible {
                continue;
            }
            let line_demerits = (params.line_penalty + fit.badness).powi(2);
            for slot in 0..best[previous].len() {
                let Some(state) = best[previous][slot] else { continue };
                let Some(previous_state) = state.state else { continue };
                let flagged = if flagged_here {
                    if previous_state.flagged > 0 { previous_state.flagged + 1 } else { 1 }
                } else {
                    0
                };
                if flagged_cap > 0 && flagged > flagged_cap {
                    continue;
                }
                let mut demerits = state.demerits + line_demerits;
                if flagged_here && previous_state.flagged > 0 {
                    demerits += params.flagged_demerits;
                }
                if previous_state.class.abs_diff(fit.class) > 1 {
                    demerits += params.fitness_demerits;
                }
                let new_class = fit.class;
                let target = new_class * (flagged_cap + 1) + flagged.min(flagged_cap);
                let replace = match best[position][target] {
                    Some(current) => demerits < current.demerits,
                    None => true,
                };
                if replace {
                    best[position][target] = Some(Best {
                        demerits,
                        from: Some(previous),
                        prev_slot: Some(slot),
                        state: Some(State { class: new_class, flagged }),
                    });
                }
            }
        }
    }
    // The end of the segment: the best path that reaches it.
    let Some(end_position) = candidates.iter().position(|c| *c == end) else {
        return Ok(Vec::new());
    };
    let mut chosen: Option<usize> = None;
    for slot in 0..best[end_position].len() {
        let Some(candidate_best) = best[end_position][slot] else { continue };
        let better = match chosen {
            None => true,
            Some(current) => match best[end_position][current] {
                Some(current_best) => candidate_best.demerits < current_best.demerits,
                None => true,
            },
        };
        if better {
            chosen = Some(slot);
        }
    }
    let Some(chosen) = chosen else { return Ok(Vec::new()) };
    // Walk the path back.
    let mut path = Vec::new();
    let mut position = end_position;
    let mut slot = chosen;
    loop {
        path.push(candidates[position]);
        let Some(state) = best[position][slot] else { break };
        let Some(from) = state.from else { break };
        let Some(prev_slot) = state.prev_slot else { break };
        slot = prev_slot;
        position = from;
    }
    path.reverse();
    // And forward into the lines.
    let mut lines = Vec::new();
    let mut previous = first_from;
    for candidate in path.iter() {
        let (width, stretch, shrink) = sums.line(items, previous, *candidate);
        let is_last = last && *candidate == end;
        let fit = fit(width, stretch, shrink, measure, params, is_last);
        let mut start_index = match previous {
            Some(previous) => previous + 1,
            None => start,
        };
        while start_index < items.len() && items[start_index].kind != Kind::Box {
            start_index += 1;
        }
        lines.push(Line {
            start: start_index,
            end: *candidate,
            next: next_start(items, *candidate),
            width,
            measure,
            ratio: fit.ratio,
            excess: fit.excess,
            flagged: items.get(*candidate).is_some_and(|item| item.flagged),
            forced: items.get(*candidate).is_some_and(Item::is_forced) || *candidate == items.len(),
        });
        previous = Some(*candidate);
    }
    Ok(lines)
}

/// The greedy breaker: each line takes as much as fits.
///
/// It never fails: a word that no line can hold is put on a line of its
/// own, and the line is reported as overfull. The limit on hyphenation
/// is honoured, as it is in the optimal breaker.
pub fn greedy(items: &[Item], measure: f64, params: &Params) -> Vec<Line> {
    if items.is_empty() {
        return Vec::new();
    }
    let sums = Sums::new(items);
    let mut lines = Vec::new();
    let mut previous: Option<usize> = None;
    let mut flagged_run = 0usize;
    let mut start = 0usize;
    loop {
        let mut chosen: Option<usize> = None;
        let mut first: Option<usize> = None;
        for candidate in start..=items.len() {
            if !is_break(items, candidate) {
                continue;
            }
            let flagged = items.get(candidate).is_some_and(|item| item.flagged);
            let run = if flagged { if flagged_run > 0 { flagged_run + 1 } else { 1 } } else { 0 };
            if params.hyphen_limit > 0 && run > params.hyphen_limit {
                continue;
            }
            if first.is_none() {
                first = Some(candidate);
            }
            let (width, _, _) = sums.line(items, previous, candidate);
            if width <= measure + params.overrun {
                chosen = Some(candidate);
            }
        }
        let Some(candidate) = chosen.or(first) else { break };
        let (width, stretch, shrink) = sums.line(items, previous, candidate);
        let is_last = candidate == items.len();
        let fit = fit(width, stretch, shrink, measure, params, is_last);
        let mut start_index = match previous {
            Some(previous) => previous + 1,
            None => 0,
        };
        while start_index < items.len() && items[start_index].kind != Kind::Box {
            start_index += 1;
        }
        let flagged = items.get(candidate).is_some_and(|item| item.flagged);
        lines.push(Line {
            start: start_index,
            end: candidate,
            next: next_start(items, candidate),
            width,
            measure,
            ratio: fit.ratio,
            excess: fit.excess,
            flagged,
            forced: items.get(candidate).is_some_and(Item::is_forced) || is_last,
        });
        flagged_run = if flagged { if flagged_run > 0 { flagged_run + 1 } else { 1 } } else { 0 };
        if candidate >= items.len() {
            break;
        }
        previous = Some(candidate);
        start = candidate + 1;
    }
    lines
}

/// The runs of boxes that no line can hold: the words that are wider
/// than the measure, even shrunk.
///
/// A caller that hyphenates adds [`Item::hyphen`] inside them.
pub fn overlong(items: &[Item], measure: f64) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut run: Option<(usize, usize, f64)> = None;
    for (index, item) in items.iter().enumerate() {
        match item.kind {
            Kind::Box => {
                let width = item.width - item.shrink;
                run = Some(match run {
                    Some((first, _, total)) => (first, index, total + width),
                    None => (index, index, width),
                });
            }
            Kind::Penalty if item.cost >= INFINITE_PENALTY => {}
            Kind::Glue | Kind::Penalty => {
                if let Some((first, last, total)) = run.take()
                    && total > measure
                {
                    out.push(first..last + 1);
                }
            }
        }
    }
    if let Some((first, last, total)) = run.take()
        && total > measure
    {
        out.push(first..last + 1);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A paragraph of words of the given widths, separated by glue.
    fn words(widths: &[f64]) -> Vec<Item> {
        let mut items = Vec::new();
        for (index, width) in widths.iter().enumerate() {
            if index > 0 {
                items.push(Item::glue(1.0, 3.0, 1.0));
            }
            items.push(Item::boxed(*width));
        }
        items
    }

    /// The words on each line, by their index in the paragraph.
    fn lines_of(items: &[Item], lines: &[Line]) -> Vec<Vec<usize>> {
        lines
            .iter()
            .map(|line| {
                let mut out = Vec::new();
                let mut index = line.start;
                while index < line.end {
                    if items[index].kind == Kind::Box {
                        out.push(index);
                    }
                    index += 1;
                }
                out
            })
            .collect()
    }

    /// The demerits of a set of lines, by the same measure the breaker
    /// used.
    fn demerits(lines: &[Line], params: &Params) -> f64 {
        lines
            .iter()
            .map(|line| {
                let badness = 100.0 * line.ratio.abs().powi(3);
                (params.line_penalty + badness).powi(2)
            })
            .sum()
    }

    #[test]
    fn a_short_paragraph_is_one_line() {
        let items = words(&[10.0, 10.0]);
        let breaks = break_paragraph(&items, 100.0, &Params::default()).expect("the paragraph");
        assert_eq!(breaks.method, Method::Optimal);
        assert_eq!(breaks.lines.len(), 1);
        assert_eq!(lines_of(&items, &breaks.lines), vec![vec![0, 2]]);
    }

    #[test]
    fn a_long_paragraph_is_broken_into_lines() {
        let items = words(&[10.0, 10.0, 10.0, 10.0, 10.0]);
        let breaks = break_paragraph(&items, 25.0, &Params::default()).expect("the paragraph");
        assert_eq!(breaks.method, Method::Optimal);
        assert_eq!(breaks.lines.len(), 3);
        // Every word is on exactly one line.
        let mut all = Vec::new();
        for line in lines_of(&items, &breaks.lines) {
            all.extend(line);
        }
        assert_eq!(all, vec![0, 2, 4, 6, 8]);
    }

    #[test]
    fn the_optimal_breaker_is_never_worse_than_the_greedy_one() {
        let items = words(&[10.0, 10.0, 10.0, 10.0, 4.0]);
        let params = Params::default();
        let optimal = break_paragraph(&items, 24.0, &params).expect("the paragraph");
        let greedy_lines = greedy(&items, 24.0, &params);
        if optimal.method == Method::Optimal {
            assert!(
                demerits(&optimal.lines, &params) <= demerits(&greedy_lines, &params),
                "the optimal breaker minimises the demerits of the paragraph"
            );
        }
    }

    #[test]
    fn a_line_is_never_overfull_when_it_can_be_shrunk() {
        let items = words(&[10.0, 10.0, 10.0]);
        let breaks = break_paragraph(&items, 25.0, &Params::default()).expect("the paragraph");
        for line in &breaks.lines {
            assert!(line.ratio >= -1.0, "a line is never shrunk past its shrink: {:?}", line.ratio);
        }
    }

    #[test]
    fn a_word_with_no_line_is_reported_as_overlong() {
        let items = words(&[10.0, 40.0, 10.0]);
        let runs = overlong(&items, 30.0);
        assert_eq!(runs, vec![2..3]);
    }

    #[test]
    fn a_hyphenation_point_is_a_break() {
        let mut items = words(&[10.0, 10.0]);
        items.push(Item::hyphen(2.0));
        items.push(Item::boxed(10.0));
        let breaks = break_paragraph(&items, 22.0, &Params::default()).expect("the paragraph");
        assert_eq!(breaks.lines.len(), 2);
        assert!(breaks.lines[0].flagged, "the first line ended at the hyphenation point");
    }

    #[test]
    fn a_forced_break_ends_a_line() {
        let mut items = words(&[10.0, 10.0]);
        items.push(Item::forced());
        items.push(Item::boxed(10.0));
        let breaks = break_paragraph(&items, 25.0, &Params::default()).expect("the paragraph");
        assert_eq!(breaks.lines.len(), 2);
        assert!(breaks.lines[0].forced, "the first line ended at the forced break");
    }

    #[test]
    fn a_ragged_paragraph_is_not_stretched() {
        let items = words(&[10.0, 10.0, 10.0]);
        let params = Params { rag: Some(Rag::default()), ..Params::default() };
        let breaks = break_paragraph(&items, 40.0, &params).expect("the paragraph");
        assert_eq!(breaks.lines.len(), 1);
        // The leftover is the rag, and the line is not stretched.
        assert!(breaks.lines[0].excess > 0.0, "the line leaves room");
        assert_eq!(breaks.lines[0].ratio, 0.0);
    }

    #[test]
    fn a_width_that_is_not_a_number_is_refused() {
        let mut items = words(&[10.0]);
        items[0].width = f64::NAN;
        let error = break_paragraph(&items, 100.0, &Params::default()).expect_err("that is not a width");
        assert!(matches!(error, LayoutError::NotFinite { index: 0 }));
    }

    #[test]
    fn a_measure_that_is_not_a_number_is_refused() {
        let items = words(&[10.0, 10.0]);
        let error = break_paragraph(&items, f64::NAN, &Params::default()).expect_err("that is not a measure");
        assert!(matches!(error, LayoutError::BadMeasure));
    }

    #[test]
    fn a_negative_measure_is_refused() {
        let items = words(&[10.0, 10.0]);
        let error = break_paragraph(&items, -1.0, &Params::default()).expect_err("that is not a measure");
        assert!(matches!(error, LayoutError::BadMeasure));
    }

    #[test]
    fn an_empty_paragraph_has_no_lines() {
        let breaks = break_paragraph(&[], 100.0, &Params::default()).expect("the paragraph");
        assert!(breaks.lines.is_empty());
    }

    #[test]
    fn the_hyphen_limit_is_a_hard_constraint() {
        // Four words, each with a hyphenation point and an ordinary
        // break. With a limit of one, two hyphenation points may not
        // follow each other.
        let mut items = Vec::new();
        for _ in 0..4 {
            items.push(Item::boxed(20.0));
            items.push(Item::glue(0.0, 0.5, 0.0));
            items.push(Item::hyphen(2.0));
        }
        items.push(Item::boxed(20.0));
        let params = Params { hyphen_limit: 1, ..Params::default() };
        let breaks = break_paragraph(&items, 22.0, &params).expect("the paragraph");
        let mut in_a_row = 0;
        for line in &breaks.lines {
            if line.flagged {
                in_a_row += 1;
                assert!(in_a_row <= 1, "two flagged breaks in a row is over the limit");
            } else {
                in_a_row = 0;
            }
        }
    }

    #[test]
    fn the_lines_do_not_overlap_and_cover_the_paragraph() {
        let items = words(&[10.0, 10.0, 10.0, 10.0, 10.0, 10.0]);
        let breaks = break_paragraph(&items, 25.0, &Params::default()).expect("the paragraph");
        let mut next = 0usize;
        for line in &breaks.lines {
            assert!(line.start >= next, "the lines do not go backwards");
            assert!(line.end > line.start, "every line has material");
            next = line.end;
        }
    }

    #[test]
    fn a_paragraph_that_nothing_fits_falls_back_to_the_greedy_breaker() {
        let items = words(&[100.0, 100.0]);
        let breaks = break_paragraph(&items, 10.0, &Params::default()).expect("the paragraph");
        assert_eq!(breaks.method, Method::Greedy);
        assert_eq!(breaks.lines.len(), 2);
    }

    #[test]
    fn the_overfull_option_lets_a_line_through() {
        let items = words(&[100.0, 10.0]);
        let params = Params { overfull: true, ..Params::default() };
        let breaks = break_paragraph(&items, 10.0, &params).expect("the paragraph");
        assert_eq!(breaks.lines.len(), 2);
        assert!(breaks.lines[0].excess < 0.0, "the line is overfull");
    }
}
