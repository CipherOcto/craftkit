//! The **model of a series in time**.
//!
//! The tree has four functions that fit a model to a series of values
//! and forecast from it:
//!
//! - `FORECAST.ETS(target_date, values, timeline, [seasonality],
//!   [data_completion], [aggregation])` — the value of the model at a
//!   date;
//! - `FORECAST.ETS.CONFINT(target_date, values, timeline,
//!   [confidence_level], [seasonality], [data_completion],
//!   [aggregation])` — the **half-width** of the confidence interval
//!   around it;
//! - `FORECAST.ETS.SEASONALITY(values, timeline, [data_completion],
//!   [aggregation])` — the **length of the season** the model found;
//! - `FORECAST.ETS.STAT(values, timeline, statistic_type,
//!   [seasonality], [data_completion], [aggregation])` — a statistic
//!   of the model.
//!
//! # The model
//!
//! The tree's model: **additive** error, **additive** trend and
//! **additive** seasonality — the state space of a book of forecasting
//! — with the three smoothing parameters worked out by trying to make
//! the one-step errors as small as possible.
//!
//! # What the port found
//!
//! - **The tree's model is fitted, and a fitted model is not a
//!   value**: the three smoothing parameters are found by a search, and
//!   the search finds a model and not an answer. The tree's own
//!   application does not say how it fits, so the tree's numbers and
//!   this tree's numbers can differ in the last digits of a noisy
//!   series — and they do not differ at all on an exact one, a
//!   straight line or a season that repeats exactly.

use crate::formula::{Book, CellRef, Expr, Value};

/// Whether a name is one of the **ETS functions**.
pub fn is_ets_function(name: &str) -> bool {
    matches!(name, "FORECAST.ETS" | "FORECAST.ETS.CONFINT" | "FORECAST.ETS.SEASONALITY" | "FORECAST.ETS.STAT")
}

/// The most points a series may hold.
const MAX_POINTS: usize = 200_000;

/// The longest season the tree supports.
const MAX_SEASON: usize = 8_760;

/// The share of a series that may be missing.
const MAX_MISSING: f64 = 0.3;

/// The values the level and the trend are fitted to.
const INIT_POINTS: usize = 10;

/// The steps of the search for the smoothing parameters.
const NM_ITERATIONS: usize = 400;

/// The **seasonality** of a model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Season {
    /// The season is found from the series itself.
    Detect,
    /// There is no season.
    None,
    /// The season is of this length.
    Fixed(usize),
}

/// What is done with two values of the **same** time stamp.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Aggregate {
    Average,
    Count,
    CountA,
    Max,
    Median,
    Min,
    Sum,
}

/// The **options** of a model.
#[derive(Clone, Copy, Debug)]
struct Options {
    seasonality: Season,
    /// `true`: missing points are interpolated; `false`: they are zero.
    interpolate: bool,
    aggregate: Aggregate,
}

impl Default for Options {
    fn default() -> Options {
        Options { seasonality: Season::Detect, interpolate: true, aggregate: Aggregate::Average }
    }
}

/// The **axis** of a series: how a time stamp maps to a place.
#[derive(Clone, Copy, Debug)]
enum Axis {
    /// A constant numeric step.
    Linear { t0: f64, step: f64 },
    /// Whole-day dates a constant number of calendar months apart.
    Months { month0: i64, step: i64, day: Option<u32> },
}

/// The **month** a date is in, and the **day** of the month.
fn month_parts(system: crate::dates::System, serial: f64) -> Option<(i64, f64)> {
    let date = crate::dates::from_serial(serial.floor(), system)?;
    if date.day == 0 {
        return None;
    }
    Some((i64::from(date.year) * 12 + i64::from(date.month) - 1, f64::from(date.day)))
}

impl Axis {
    /// The **place** of a time stamp: zero is the first point.
    fn position(&self, system: crate::dates::System, t: f64) -> Option<f64> {
        match *self {
            Axis::Linear { t0, step } => Some((t - t0) / step),
            Axis::Months { month0, step, day } => {
                let date = crate::dates::from_serial(t.floor(), system)?;
                if date.day == 0 {
                    return None;
                }
                let month = i64::from(date.year) * 12 + i64::from(date.month) - 1;
                let length = f64::from(crate::dates::days_in_month(date.year, date.month));
                if length <= 0.0 {
                    return None;
                }
                let anchor = day.map_or(length, |day| f64::from(day).min(length));
                let offset = (f64::from(date.day) - anchor + (t - t.floor())) / length;
                Some(((month - month0) as f64 + offset) / step as f64)
            }
        }
    }
}

/// The **greatest common divisor** of two numbers.
fn gcd(mut left: i64, mut right: i64) -> i64 {
    while right != 0 {
        let rest = left % right;
        left = right;
        right = rest;
    }
    left.abs()
}

/// The axis of a sorted set of time stamps.
///
/// The tree's rule: a constant numeric step — with the points that are
/// missing allowed — or, for whole-day dates, a constant number of
/// calendar months, all on the same day of the month or all at the end
/// of their month.
fn detect_axis(system: crate::dates::System, times: &[f64]) -> Option<Axis> {
    let (&first, &second) = match (times.first(), times.get(1)) {
        (Some(a), Some(b)) => (a, b),
        _ => return None,
    };
    let step = times
        .windows(2)
        .map(|pair| pair.get(1).copied().unwrap_or(second) - pair.first().copied().unwrap_or(first))
        .fold(f64::INFINITY, f64::min);
    if step.is_finite() && step > 0.0 {
        let even = times.windows(2).all(|pair| match (pair.first(), pair.get(1)) {
            (Some(left), Some(right)) => {
                let ratio = (right - left) / step;
                (ratio - ratio.round()).abs() <= 1e-6
            }
            _ => true,
        });
        if even {
            return Some(Axis::Linear { t0: first, step });
        }
    }
    // Calendar months.
    let mut months = Vec::with_capacity(times.len());
    let mut same_day: Option<u32> = None;
    let mut all_same = true;
    let mut all_end = true;
    for &time in times {
        if time.fract() != 0.0 {
            return None;
        }
        let date = crate::dates::from_serial(time, system)?;
        if date.day == 0 {
            return None;
        }
        months.push(i64::from(date.year) * 12 + i64::from(date.month) - 1);
        match same_day {
            None => same_day = Some(date.day),
            Some(day) if day != date.day => all_same = false,
            _ => {}
        }
        if date.day != crate::dates::days_in_month(date.year, date.month) {
            all_end = false;
        }
    }
    let day = if all_same {
        same_day
    } else if all_end {
        None
    } else {
        return None;
    };
    let mut divisor = 0i64;
    for pair in months.windows(2) {
        if let (Some(left), Some(right)) = (pair.first(), pair.get(1)) {
            let step = right - left;
            if step <= 0 {
                return None;
            }
            divisor = gcd(divisor, step);
        }
    }
    let month0 = month_parts(system, first)?.0;
    if divisor <= 0 {
        return None;
    }
    Some(Axis::Months { month0, step: divisor, day })
}

/// An evenly spaced series, built from the values and the timeline.
#[derive(Clone, Debug)]
struct Series {
    /// The values, one for each place on the axis.
    y: Vec<f64>,
    axis: Axis,
    /// The step of the timeline, in the units of the timeline.
    step: f64,
}

impl Series {
    /// The **place** of a time stamp after the last point: zero is the
    /// last point and one is the next step.
    fn horizon(&self, system: crate::dates::System, t: f64) -> Option<f64> {
        let place = self.axis.position(system, t)?;
        let horizon = place - (self.y.len() as f64 - 1.0);
        if !horizon.is_finite() || horizon < -1e-9 {
            return None;
        }
        Some(horizon.max(0.0))
    }
}

/// The **one** value of a group of values of the same time stamp.
fn aggregate_of(kind: Aggregate, values: &mut [f64]) -> Option<f64> {
    let count = values.len() as f64;
    match kind {
        Aggregate::Count | Aggregate::CountA => Some(count),
        _ if values.is_empty() => None,
        Aggregate::Average => Some(values.iter().sum::<f64>() / count),
        Aggregate::Sum => Some(values.iter().sum()),
        Aggregate::Max => values.iter().copied().reduce(f64::max),
        Aggregate::Min => values.iter().copied().reduce(f64::min),
        Aggregate::Median => {
            values.sort_by(f64::total_cmp);
            let middle = values.len() / 2;
            let high = values.get(middle).copied()?;
            if values.len() % 2 == 1 {
                Some(high)
            } else {
                Some((values.get(middle.checked_sub(1)?).copied()? + high) / 2.0)
            }
        }
    }
}

/// The **fault** of a series that cannot be built.
///
/// The tree's rule: the two sets of numbers must be of the same
/// length — that is `#N/A` and not `#NUM!` — a value that is not a
/// number is `#VALUE!`, and an error in a cell is the error of that
/// cell.
#[derive(Clone, Debug)]
enum Fault {
    /// The two sets of numbers are not of the same length.
    NotAvailable,
    /// The series does not make a model.
    NotNumber,
    /// A value is not a number.
    NotValue,
    /// The **error** of a cell, carried through.
    Cell(String),
}

impl Fault {
    /// The **message** a fault is reported with.
    fn message(&self) -> String {
        match self {
            Fault::NotAvailable => "#N/A".to_string(),
            Fault::NotNumber => "#NUM!".to_string(),
            Fault::NotValue => "#VALUE!".to_string(),
            Fault::Cell(message) => message.clone(),
        }
    }
}

/// The series, built from the values and the timeline.
///
/// The tree's rule: the two must be of the same length, the time
/// stamps are sorted, the values of one time stamp are **aggregated**,
/// the points that are missing are **completed** and there may not be
/// too many of them.
fn series_of(
    system: crate::dates::System,
    values: &[Value],
    timeline: &[Value],
    options: &Options,
) -> Result<Series, Fault> {
    if values.len() != timeline.len() {
        return Err(Fault::NotAvailable);
    }
    if values.len() > MAX_POINTS {
        return Err(Fault::NotNumber);
    }
    let mut points: Vec<(f64, Option<f64>)> = Vec::with_capacity(values.len());
    for (time, value) in timeline.iter().zip(values.iter()) {
        if let Value::Error(message) = time {
            return Err(Fault::Cell(message.clone()));
        }
        if let Value::Error(message) = value {
            return Err(Fault::Cell(message.clone()));
        }
        let number = match value {
            Value::Number(number) => Some(*number),
            Value::Empty => None,
            _ => return Err(Fault::NotValue),
        };
        let stamp = match time {
            Value::Number(number) if number.is_finite() => *number,
            // A **blank** row is ignored.
            Value::Empty if number.is_none() => continue,
            _ => return Err(Fault::NotValue),
        };
        points.push((stamp, number));
    }
    points.sort_by(|left, right| left.0.total_cmp(&right.0));
    let mut times: Vec<f64> = points.iter().map(|point| point.0).collect();
    times.dedup();
    if times.len() < 2 {
        return Err(Fault::NotNumber);
    }
    let Some(axis) = detect_axis(system, &times) else {
        return Err(Fault::NotNumber);
    };
    let (&first, &last) = match (times.first(), times.last()) {
        (Some(a), Some(b)) => (a, b),
        _ => return Err(Fault::NotNumber),
    };
    let Some(span) = axis.position(system, last).map(|span| span.round()) else {
        return Err(Fault::NotNumber);
    };
    if !(span >= 1.0 && span < MAX_POINTS as f64) {
        return Err(Fault::NotNumber);
    }
    let length = span as usize + 1;
    // The values, gathered by place.
    let mut slots: Vec<Option<f64>> = vec![None; length];
    let mut group: Vec<f64> = Vec::new();
    let mut group_at: Option<usize> = None;
    for &(stamp, value) in &points {
        let Some(place) = axis.position(system, stamp).map(|place| place.round()) else {
            return Err(Fault::NotNumber);
        };
        if !(place >= 0.0 && place < length as f64) {
            return Err(Fault::NotNumber);
        }
        let place = place as usize;
        if group_at != Some(place) {
            if let Some(at) = group_at
                && let Some(slot) = slots.get_mut(at)
            {
                *slot = aggregate_of(options.aggregate, &mut group);
            }
            group.clear();
            group_at = Some(place);
        }
        if let Some(number) = value {
            group.push(number);
        }
    }
    if let Some(at) = group_at
        && let Some(slot) = slots.get_mut(at)
    {
        *slot = aggregate_of(options.aggregate, &mut group);
    }
    let known = slots.iter().filter(|slot| slot.is_some()).count();
    if known == 0 || (length - known) as f64 > MAX_MISSING * length as f64 {
        return Err(Fault::NotNumber);
    }
    let y = if options.interpolate {
        interpolate(&slots)
    } else {
        slots.iter().map(|slot| slot.unwrap_or(0.0)).collect()
    };
    let step = match axis {
        Axis::Linear { step, .. } => step,
        Axis::Months { .. } => (last - first) / span,
    };
    Ok(Series { y, axis, step })
}

/// The missing points become the straight line between their known
/// neighbours.
fn interpolate(slots: &[Option<f64>]) -> Vec<f64> {
    let mut out = Vec::with_capacity(slots.len());
    let mut known = slots.iter().enumerate().filter_map(|(at, slot)| slot.map(|value| (at, value)));
    let mut next = known.next();
    let mut previous: Option<(usize, f64)> = None;
    for (at, slot) in slots.iter().enumerate() {
        if let Some(value) = slot {
            out.push(*value);
            previous = Some((at, *value));
            next = known.next();
            continue;
        }
        out.push(match (previous, next) {
            (Some((before, before_value)), Some((after, after_value))) => {
                before_value + (after_value - before_value) * (at - before) as f64 / (after - before) as f64
            }
            (Some((_, value)), None) | (None, Some((_, value))) => value,
            (None, None) => 0.0,
        });
    }
    out
}

/// The **residuals** of the least-squares line through a series.
fn detrend(y: &[f64]) -> Vec<f64> {
    let (intercept, slope) = line(y.iter().enumerate().map(|(at, value)| (at as f64, *value)));
    y.iter().enumerate().map(|(at, value)| value - (intercept + slope * at as f64)).collect()
}

/// The **intercept** and the **slope** of the least-squares line
/// through some points.
fn line(points: impl Iterator<Item = (f64, f64)> + Clone) -> (f64, f64) {
    let (mut count, mut sum_x, mut sum_y) = (0.0, 0.0, 0.0);
    for (x, y) in points.clone() {
        count += 1.0;
        sum_x += x;
        sum_y += y;
    }
    if count == 0.0 {
        return (0.0, 0.0);
    }
    let (mean_x, mean_y) = (sum_x / count, sum_y / count);
    let (mut cross, mut square) = (0.0, 0.0);
    for (x, y) in points {
        cross += (x - mean_x) * (y - mean_y);
        square += (x - mean_x) * (x - mean_x);
    }
    let slope = if square > 0.0 { cross / square } else { 0.0 };
    (mean_y - slope * mean_x, slope)
}

/// The **season** the series shows, and zero when it shows none.
///
/// The tree's rule: the **autocorrelation** of the series with its
/// trend taken off, and the highest peak above the white-noise bound
/// at a lag of at most half the series.
fn detect_season(y: &[f64]) -> usize {
    let count = y.len();
    if count < 4 {
        return 0;
    }
    let residuals = detrend(y);
    let variance: f64 = residuals.iter().map(|value| value * value).sum();
    let scale: f64 = y.iter().map(|value| value * value).sum::<f64>();
    if !(variance.is_finite() && variance > 1e-20 * scale.max(f64::MIN_POSITIVE)) {
        return 0;
    }
    let max_lag = (count / 2).min(MAX_SEASON);
    let acf: Vec<f64> = (0..=max_lag.saturating_add(1).min(count.saturating_sub(1)))
        .map(|lag| {
            residuals.iter().skip(lag).zip(residuals.iter()).map(|(left, right)| left * right).sum::<f64>()
                / variance
        })
        .collect();
    let bound = 2.0 / (count as f64).sqrt();
    let mut best = 0usize;
    let mut best_r = bound;
    for lag in 2..=max_lag {
        let (Some(&previous), Some(&here)) = (acf.get(lag - 1), acf.get(lag)) else { break };
        let next = acf.get(lag + 1).copied().unwrap_or(f64::NEG_INFINITY);
        if here > previous && here >= next && here > best_r {
            best = lag;
            best_r = here;
        }
    }
    best
}

/// The **initial states** of a model.
#[derive(Clone, Debug)]
struct Init {
    level: f64,
    trend: f64,
    /// The seasonal state, one for each place of the season.
    season: Vec<f64>,
}

/// The initial states of the tree's model.
fn initial_states(y: &[f64], season: usize) -> Init {
    let count = y.len();
    let mut seasonal = vec![0.0; season];
    if season >= 2 {
        if count >= 2 * season {
            // The **classical decomposition** of the first seasons: the
            // centred moving average is the trend.
            let length = count.min(4 * season).max(2 * season);
            let window = &y[..length.min(count)];
            let mut prefix = Vec::with_capacity(window.len() + 1);
            prefix.push(0.0);
            let mut total = 0.0;
            for value in window {
                total += value;
                prefix.push(total);
            }
            let sum = |from: usize, to: usize| {
                prefix.get(to).copied().unwrap_or(0.0) - prefix.get(from).copied().unwrap_or(0.0)
            };
            let half = season / 2;
            let mut totals = vec![0.0; season];
            let mut counts = vec![0usize; season];
            for at in half..window.len().saturating_sub(half) {
                let average = if season % 2 == 1 {
                    sum(at - half, at + half + 1) / season as f64
                } else {
                    let ends = window.get(at - half).copied().unwrap_or(0.0)
                        + window.get(at + half).copied().unwrap_or(0.0);
                    (sum(at - half + 1, at + half) + ends / 2.0) / season as f64
                };
                if let (Some(total), Some(count), Some(value)) =
                    (totals.get_mut(at % season), counts.get_mut(at % season), window.get(at))
                {
                    *total += value - average;
                    *count += 1;
                }
            }
            for ((state, total), count) in seasonal.iter_mut().zip(&totals).zip(&counts) {
                *state = if *count > 0 { total / *count as f64 } else { 0.0 };
            }
        } else {
            // Under two seasons: the deviations from a straight line.
            let residuals = detrend(y);
            let mut counts = vec![0usize; season];
            for (at, value) in residuals.iter().enumerate() {
                if let (Some(state), Some(count)) =
                    (seasonal.get_mut(at % season), counts.get_mut(at % season))
                {
                    *state += value;
                    *count += 1;
                }
            }
            for (state, count) in seasonal.iter_mut().zip(&counts) {
                if *count > 0 {
                    *state /= *count as f64;
                }
            }
        }
        let mean = seasonal.iter().sum::<f64>() / season as f64;
        for state in &mut seasonal {
            *state -= mean;
        }
    }
    let taken = count.min(INIT_POINTS);
    let adjusted = y.iter().take(taken).enumerate().map(|(at, value)| {
        let state = if season >= 2 { seasonal.get(at % season).copied().unwrap_or(0.0) } else { 0.0 };
        ((at + 1) as f64, value - state)
    });
    let (level, trend) = line(adjusted);
    Init { level, trend, season: seasonal }
}

/// The three smoothing parameters.
type Params = [f64; 3];

/// The **state** of a model after the filter has run.
#[derive(Clone, Debug)]
struct State {
    level: f64,
    trend: f64,
    season: Vec<f64>,
    sse: f64,
}

/// The filter of the tree's model, run over the series.
fn filter(y: &[f64], init: &Init, params: Params, mut errors: Option<&mut Vec<f64>>) -> State {
    let [alpha, beta, gamma] = params;
    let season = init.season.len();
    let (mut level, mut trend) = (init.level, init.trend);
    let mut seasonal = init.season.clone();
    let mut sse = 0.0;
    for (at, value) in y.iter().enumerate() {
        let slot = if season > 0 { seasonal.get_mut(at % season) } else { None };
        let state = slot.as_ref().map_or(0.0, |state| **state);
        let error = value - (level + trend + state);
        if let Some(slot) = slot {
            *slot = state + gamma * error;
        }
        level = level + trend + alpha * error;
        trend += alpha * beta * error;
        sse += error * error;
        if let Some(errors) = errors.as_deref_mut() {
            errors.push(error);
        }
    }
    State { level, trend, season: seasonal, sse }
}

/// The **sum of the squared errors** of a set of parameters.
fn objective(y: &[f64], init: &Init, params: Params) -> f64 {
    let sse = filter(y, init, params, None).sse;
    if sse.is_finite() { sse } else { f64::INFINITY }
}

/// The parameters, held inside the box of the model.
fn clamp(params: Params, active: usize) -> Params {
    let mut out = [0.0; 3];
    for (index, (out, value)) in out.iter_mut().zip(params).enumerate() {
        *out = if index < active { value.clamp(0.0, 1.0) } else { 0.0 };
    }
    out
}

/// The parameters that make the errors as small as possible.
///
/// The tree's way: a **coarse grid** first, and then the simplex of
/// **Nelder and Mead** from the best point of the grid.
fn optimise(y: &[f64], init: &Init) -> Params {
    let active = if init.season.is_empty() { 2 } else { 3 };
    let cost = |params: Params| objective(y, init, clamp(params, active));
    const GRID: [f64; 5] = [0.05, 0.25, 0.5, 0.75, 0.95];
    let mut best: (Params, f64) = ([0.5, 0.1, 0.1], f64::INFINITY);
    for alpha in GRID {
        for beta in GRID {
            for gamma in if active == 3 { &GRID[..] } else { &GRID[..1] } {
                let params = clamp([alpha, beta, *gamma], active);
                let value = cost(params);
                if value < best.1 {
                    best = (params, value);
                }
            }
        }
    }
    let mut simplex: Vec<(Params, f64)> = vec![best];
    for index in 0..active {
        let mut params = best.0;
        if let Some(value) = params.get_mut(index) {
            *value = if *value > 0.5 { *value - 0.1 } else { *value + 0.1 };
        }
        let params = clamp(params, active);
        simplex.push((params, cost(params)));
    }
    let lerp = |left: Params, right: Params, at: f64| -> Params {
        let mut out = [0.0; 3];
        for ((out, left), right) in out.iter_mut().zip(left).zip(right) {
            *out = left + at * (right - left);
        }
        clamp(out, active)
    };
    for _ in 0..NM_ITERATIONS {
        simplex.sort_by(|left, right| left.1.total_cmp(&right.1));
        let (&lowest, &highest) = match (simplex.first(), simplex.last()) {
            (Some(a), Some(b)) => (a, b),
            _ => break,
        };
        let second = simplex.len().checked_sub(2).and_then(|at| simplex.get(at)).map_or(highest.1, |s| s.1);
        let size = simplex
            .iter()
            .map(|s| s.0.iter().zip(lowest.0).map(|(a, b)| (a - b).abs()).fold(0.0, f64::max))
            .fold(0.0, f64::max);
        if (highest.1 - lowest.1).abs() <= 1e-12 * lowest.1.abs() + 1e-300 || size < 1e-7 {
            break;
        }
        let count = simplex.len().saturating_sub(1).max(1) as f64;
        let mut centre = [0.0; 3];
        for s in simplex.iter().take(simplex.len().saturating_sub(1)) {
            for (value, x) in centre.iter_mut().zip(s.0) {
                *value += x / count;
            }
        }
        let reflected = lerp(centre, highest.0, -1.0);
        let reflected_cost = cost(reflected);
        let replacement = if reflected_cost < lowest.1 {
            let expanded = lerp(centre, highest.0, -2.0);
            let expanded_cost = cost(expanded);
            Some(if expanded_cost < reflected_cost {
                (expanded, expanded_cost)
            } else {
                (reflected, reflected_cost)
            })
        } else if reflected_cost < second {
            Some((reflected, reflected_cost))
        } else {
            let contracted = lerp(centre, highest.0, 0.5);
            let contracted_cost = cost(contracted);
            if contracted_cost < highest.1 { Some((contracted, contracted_cost)) } else { None }
        };
        match replacement {
            Some(value) => {
                if let Some(last) = simplex.last_mut() {
                    *last = value;
                }
            }
            None => {
                for s in simplex.iter_mut().skip(1) {
                    let params = lerp(lowest.0, s.0, 0.5);
                    *s = (params, cost(params));
                }
            }
        }
    }
    simplex.sort_by(|left, right| left.1.total_cmp(&right.1));
    clamp(simplex.first().map_or(best.0, |s| s.0), active)
}

/// A **fitted** model.
#[derive(Clone, Debug)]
struct Fit {
    alpha: f64,
    beta: f64,
    gamma: f64,
    season: usize,
    count: usize,
    state: State,
    errors: Vec<f64>,
}

/// The model, fitted to a series.
fn fit(y: &[f64], season: usize) -> Fit {
    let season = if season >= 2 { season } else { 0 };
    let init = initial_states(y, season);
    let params = optimise(y, &init);
    let mut errors = Vec::with_capacity(y.len());
    let state = filter(y, &init, params, Some(&mut errors));
    let [alpha, beta, gamma] = params;
    Fit { alpha, beta, gamma, season, count: y.len(), state, errors }
}

impl Fit {
    /// The value of the model `k` steps after the last point.
    fn at(&self, k: f64) -> f64 {
        let state = if self.season > 0 {
            let index = ((self.count as f64 - 1.0) + k).rem_euclid(self.season as f64);
            self.state.season.get(index as usize).copied().unwrap_or(0.0)
        } else {
            0.0
        };
        self.state.level + k * self.state.trend + state
    }

    /// The **forecast** `h` steps after the last point.
    fn forecast(&self, h: f64) -> f64 {
        let low = h.floor();
        let at = self.at(low);
        if h == low { at } else { at + (h - low) * (self.at(low + 1.0) - at) }
    }

    /// The **standard deviation** of the forecast error `h` steps
    /// ahead.
    fn forecast_sd(&self, h: f64) -> f64 {
        let variance = self.state.sse / self.count.max(1) as f64;
        let k = (h.ceil().max(1.0)) - 1.0;
        let (alpha, beta, gamma) = (self.alpha, self.alpha * self.beta, self.gamma);
        let mut sum = k * alpha * alpha
            + alpha * beta * k * (k + 1.0)
            + beta * beta * k * (k + 1.0) * (2.0 * k + 1.0) / 6.0;
        if self.season > 0 {
            let season = self.season as f64;
            let q = (k / season).floor();
            sum += 2.0 * gamma * (q * alpha + beta * season * q * (q + 1.0) / 2.0) + gamma * gamma * q;
        }
        (variance * (1.0 + sum)).max(0.0).sqrt()
    }
}

impl Book {
    /// The **ETS functions**.
    pub(crate) fn ets(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, CellRef)>,
    ) -> Value {
        let system = self.system();
        match name {
            "FORECAST.ETS" => {
                let target = match self.number_at(sheet, arguments, 0, being) {
                    Ok(number) => number,
                    Err(message) => return Value::Error(message),
                };
                let Some(options) = self.ets_options(sheet, arguments, 3, 4, 5, being) else {
                    return Value::Error("#NUM!".to_string());
                };
                let (series, season) = match self.ets_series(sheet, arguments, 1, 2, &options, being) {
                    Ok(found) => found,
                    Err(fault) => return Value::Error(fault.message()),
                };
                let Some(horizon) = series.horizon(system, target) else {
                    return Value::Error("#NUM!".to_string());
                };
                Value::Number(fit(&series.y, season).forecast(horizon))
            }
            "FORECAST.ETS.CONFINT" => {
                let target = match self.number_at(sheet, arguments, 0, being) {
                    Ok(number) => number,
                    Err(message) => return Value::Error(message),
                };
                let level = match self.optional_number(sheet, arguments, 3, being) {
                    Ok(Some(number)) => number,
                    Ok(None) => 0.95,
                    Err(message) => return Value::Error(message),
                };
                if !(level > 0.0 && level < 1.0) {
                    return Value::Error("#NUM!".to_string());
                }
                let Some(options) = self.ets_options(sheet, arguments, 4, 5, 6, being) else {
                    return Value::Error("#NUM!".to_string());
                };
                let (series, season) = match self.ets_series(sheet, arguments, 1, 2, &options, being) {
                    Ok(found) => found,
                    Err(fault) => return Value::Error(fault.message()),
                };
                let Some(horizon) = series.horizon(system, target) else {
                    return Value::Error("#NUM!".to_string());
                };
                let z = crate::special::norm_inv((1.0 + level) / 2.0);
                Value::Number(z * fit(&series.y, season).forecast_sd(horizon))
            }
            "FORECAST.ETS.SEASONALITY" => {
                let Some(options) = self.ets_options(sheet, arguments, usize::MAX, 2, 3, being) else {
                    return Value::Error("#NUM!".to_string());
                };
                let (_series, season) = match self.ets_series(sheet, arguments, 0, 1, &options, being) {
                    Ok(found) => found,
                    Err(fault) => return Value::Error(fault.message()),
                };
                Value::Number(season as f64)
            }
            _ => {
                // `FORECAST.ETS.STAT`: a statistic of the model.
                let kind = match self.number_at(sheet, arguments, 2, being) {
                    Ok(number) => number,
                    Err(message) => return Value::Error(message),
                };
                if !kind.is_finite() || !(1.0..9.0).contains(&kind.trunc()) {
                    return Value::Error("#NUM!".to_string());
                }
                let kind = kind.trunc() as u8;
                let Some(options) = self.ets_options(sheet, arguments, 3, 4, 5, being) else {
                    return Value::Error("#NUM!".to_string());
                };
                let (series, season) = match self.ets_series(sheet, arguments, 0, 1, &options, being) {
                    Ok(found) => found,
                    Err(fault) => return Value::Error(fault.message()),
                };
                if kind == 8 {
                    return Value::Number(series.step);
                }
                let y = &series.y;
                let fitted = fit(y, season);
                let count = fitted.errors.len().max(1) as f64;
                let mae = fitted.errors.iter().map(|error| error.abs()).sum::<f64>() / count;
                let value = match kind {
                    1 => fitted.alpha,
                    2 => fitted.beta,
                    3 => fitted.gamma,
                    4 => {
                        let naive: Vec<f64> = y
                            .windows(2)
                            .filter_map(|pair| Some((pair.get(1)? - pair.first()?).abs()))
                            .collect();
                        let scale = naive.iter().sum::<f64>() / naive.len().max(1) as f64;
                        if scale == 0.0 {
                            return Value::Error("#DIV/0!".to_string());
                        }
                        mae / scale
                    }
                    5 => {
                        let terms: Vec<f64> = y
                            .iter()
                            .zip(&fitted.errors)
                            .filter_map(|(value, error)| {
                                let denominator = (value.abs() + (value - error).abs()) / 2.0;
                                (denominator > 0.0).then(|| error.abs() / denominator)
                            })
                            .collect();
                        terms.iter().sum::<f64>() / terms.len().max(1) as f64
                    }
                    6 => mae,
                    7 => (fitted.errors.iter().map(|error| error * error).sum::<f64>() / count).sqrt(),
                    _ => return Value::Error("#NUM!".to_string()),
                };
                Value::Number(value)
            }
        }
    }

    /// The **options** of a model, and `None` when one of them is not
    /// a number.
    fn ets_options(
        &self,
        sheet: &str,
        arguments: &[Expr],
        season: usize,
        completion: usize,
        aggregation: usize,
        being: &mut Vec<(String, CellRef)>,
    ) -> Option<Options> {
        let mut options = Options::default();
        if season != usize::MAX {
            let seasonality = match self.optional_number(sheet, arguments, season, being) {
                Ok(Some(number)) => {
                    if !number.is_finite() {
                        return None;
                    }
                    match number.trunc() as i64 {
                        1 => Season::Detect,
                        0 => Season::None,
                        length if (2..=MAX_SEASON as i64).contains(&length) => Season::Fixed(length as usize),
                        _ => return None,
                    }
                }
                Ok(None) => Season::Detect,
                Err(_) => return None,
            };
            options.seasonality = seasonality;
        }
        options.interpolate = match self.optional_number(sheet, arguments, completion, being) {
            Ok(Some(number)) => {
                if !number.is_finite() {
                    return None;
                }
                match number.trunc() as i64 {
                    1 => true,
                    0 => false,
                    _ => return None,
                }
            }
            Ok(None) => true,
            Err(_) => return None,
        };
        options.aggregate = match self.optional_number(sheet, arguments, aggregation, being) {
            Ok(Some(number)) => {
                if !number.is_finite() {
                    return None;
                }
                match number.trunc() as i64 {
                    0 | 1 => Aggregate::Average,
                    2 => Aggregate::Count,
                    3 => Aggregate::CountA,
                    4 => Aggregate::Max,
                    5 => Aggregate::Median,
                    6 => Aggregate::Min,
                    7 => Aggregate::Sum,
                    _ => return None,
                }
            }
            Ok(None) => Aggregate::Average,
            Err(_) => return None,
        };
        Some(options)
    }

    /// The **series** a model is fitted to, and the length of its
    /// season.
    fn ets_series(
        &self,
        sheet: &str,
        arguments: &[Expr],
        values: usize,
        timeline: usize,
        options: &Options,
        being: &mut Vec<(String, CellRef)>,
    ) -> Result<(Series, usize), Fault> {
        let Some(values_argument) = arguments.get(values) else {
            return Err(Fault::NotValue);
        };
        let Some(timeline_argument) = arguments.get(timeline) else {
            return Err(Fault::NotValue);
        };
        let Ok(values) = self.values_of_cells(sheet, values_argument, being) else {
            return Err(Fault::NotValue);
        };
        let Ok(timeline) = self.values_of_cells(sheet, timeline_argument, being) else {
            return Err(Fault::NotValue);
        };
        let series = series_of(self.system(), &values, &timeline, options)?;
        let season = match options.seasonality {
            Season::Detect => detect_season(&series.y),
            Season::None => 0,
            Season::Fixed(length) => length,
        };
        Ok((series, season))
    }
}
