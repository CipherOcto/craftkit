//! The **trendline** of a chart: the line that best fits its values.
//!
//! The tree's own maths: a **least-squares fit** of the points of a
//! series, and the **moving average** of them.
//!
//! # The rules
//!
//! - The **kinds** of fit are the tree's own: `linear`, `exponential`,
//!   `logarithmic`, `power`, `polynomial` and `moving_average`.
//! - An **exponential**, a **logarithmic** and a **power** fit are
//!   fits of **transformed** data: the logarithm of `y` against `x`,
//!   `y` against the logarithm of `x`, and the logarithm of `y`
//!   against the logarithm of `x`. A point where the transform is not
//!   defined — a number that is not positive — is **not used** by the
//!   fit, and the command says how many points were left out.
//! - A fit needs **more points than the degree** of the polynomial,
//!   and points whose `x` are **not all the same**.
//! - The **order** of a polynomial is between two and six, and the
//!   **period** of a moving average is between two and two hundred and
//!   fifty-five: a number outside is **refused**, and not made to fit.
//! - A fit that cannot be found is **refused**: the tree's rule is
//!   that there is no fit rather than a wrong one.
//!
//! # What it does not settle
//!
//! - The tree's **set intercept** — a trendline that is told to pass
//!   through a given point — is not here.
//! - The tree **stores** a trendline on the chart. Here the fit is
//!   **derived**: it is worked out from the cells of the sheet every
//!   time it is asked for, and the chart keeps no trace of it.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Text};

use crate::charts::{Series, area_of, data_of, kind_of_chart, sheet_of};

/// The **kinds** of fit there are.
pub const KINDS: [&str; 6] =
    ["linear", "exponential", "logarithmic", "power", "polynomial", "moving_average"];

/// The **least** order a polynomial may have.
pub const MIN_ORDER: u64 = 2;

/// The **most** order a polynomial may have.
pub const MAX_ORDER: u64 = 6;

/// The **least** period a moving average may have.
pub const MIN_PERIOD: u64 = 2;

/// The **most** period a moving average may have.
pub const MAX_PERIOD: u64 = 255;

/// A **kind of fit**.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trend {
    /// A straight line.
    Linear,
    /// A line of the logarithm of `y`.
    Exponential,
    /// A line of the logarithm of `x`.
    Logarithmic,
    /// A line of both logarithms.
    Power,
    /// A polynomial of an order.
    Polynomial(usize),
    /// The average of the last points of each point.
    MovingAverage(usize),
}

impl Trend {
    /// The **name** of the kind of fit.
    pub fn name(&self) -> String {
        match self {
            Trend::Linear => "linear".to_string(),
            Trend::Exponential => "exponential".to_string(),
            Trend::Logarithmic => "logarithmic".to_string(),
            Trend::Power => "power".to_string(),
            Trend::Polynomial(order) => format!("polynomial {order}"),
            Trend::MovingAverage(period) => format!("moving average of {period}"),
        }
    }

    /// Whether the fit takes the **logarithm** of `x`.
    fn log_x(&self) -> bool {
        matches!(self, Trend::Logarithmic | Trend::Power)
    }

    /// Whether the fit takes the **logarithm** of `y`.
    fn log_y(&self) -> bool {
        matches!(self, Trend::Exponential | Trend::Power)
    }

    /// The **degree** of the fit: the moving average has none.
    fn degree(&self) -> Option<usize> {
        match self {
            Trend::Linear | Trend::Exponential | Trend::Logarithmic | Trend::Power => Some(1),
            Trend::Polynomial(order) => Some(*order),
            Trend::MovingAverage(_) => None,
        }
    }
}

/// The **kind of fit** a name asks for.
///
/// The tree's own names, and the tree's own bounds: a number outside
/// them is refused, and not made to fit.
pub fn trend_of(name: &str, order: Option<Number>, period: Option<Number>) -> Result<Trend, String> {
    let wanted = name.trim().to_ascii_lowercase();
    match wanted.as_str() {
        "linear" => Ok(Trend::Linear),
        "exponential" => Ok(Trend::Exponential),
        "logarithmic" | "log" => Ok(Trend::Logarithmic),
        "power" => Ok(Trend::Power),
        "polynomial" => {
            let order = order.ok_or_else(|| "a polynomial needs its order".to_string())?;
            let order = order.0;
            if !order.is_finite() || order.fract() != 0.0 {
                return Err("the order of a polynomial is a whole number".to_string());
            }
            let order = order as u64;
            if !(MIN_ORDER..=MAX_ORDER).contains(&order) {
                return Err(format!(
                    "the order of a polynomial is between {MIN_ORDER} and {MAX_ORDER}, and {order} is not"
                ));
            }
            Ok(Trend::Polynomial(order as usize))
        }
        "moving_average" | "movingaverage" => {
            let period = period.ok_or_else(|| "a moving average needs its period".to_string())?;
            let period = period.0;
            if !period.is_finite() || period.fract() != 0.0 {
                return Err("the period of a moving average is a whole number".to_string());
            }
            let period = period as u64;
            if !(MIN_PERIOD..=MAX_PERIOD).contains(&period) {
                return Err(format!(
                    "the period of a moving average is between {MIN_PERIOD} and {MAX_PERIOD}, and {period} is not"
                ));
            }
            Ok(Trend::MovingAverage(period as usize))
        }
        _ => Err(format!("`{wanted}` is not a kind of fit: it is one of {}", KINDS.join(", "))),
    }
}

/// A **fit**: the coefficients of the line, and how good it is.
#[derive(Debug, Clone)]
pub struct Fit {
    /// The kind of fit.
    pub kind: Trend,
    /// The coefficients `c0..=cd` of `y = Σ cₖ·xᵏ`, for the equation.
    pub coef: Vec<f64>,
    /// The **centre** of the `x` of the points: the fit is solved on
    /// centred and scaled `x`, and this is the centre it used.
    center: f64,
    /// The **scale** of the `x` of the points.
    scale: f64,
    /// The same polynomial in `u = (x - centre) / scale`.
    a: Vec<f64>,
    /// How much of the spread of the `y` the fit explains: `None` when
    /// it is not defined, which is when every `y` is the same.
    pub r2: Option<f64>,
}

impl Fit {
    /// The fit at `x`: `None` outside its own domain — an `x` that is
    /// not positive for a fit of a logarithm — or when the answer is
    /// not a number.
    pub fn eval(&self, x: f64) -> Option<f64> {
        let x = if self.kind.log_x() {
            if x > 0.0 {
                x.ln()
            } else {
                return None;
            }
        } else {
            x
        };
        let u = (x - self.center) / self.scale;
        let value = self.a.iter().rev().fold(0.0, |acc, c| acc * u + c);
        let value = if self.kind.log_y() { value.exp() } else { value };
        value.is_finite().then_some(value)
    }

    /// The **equation** of the fit, as the tree writes it: `y = 2x + 1`.
    pub fn equation(&self) -> String {
        let c0 = self.coef.first().copied().unwrap_or(0.0);
        let c1 = self.coef.get(1).copied().unwrap_or(0.0);
        match self.kind {
            Trend::Exponential => format!("y = {}e^({}x)", num_text(c0.exp()), num_text(c1)),
            Trend::Power => format!("y = {}x^{}", num_text(c0.exp()), num_text(c1)),
            Trend::Logarithmic => terms(&[(c1, "ln(x)"), (c0, "")]),
            _ => {
                let parts: Vec<(f64, &str)> = self
                    .coef
                    .iter()
                    .enumerate()
                    .rev()
                    .map(|(power, c)| (*c, POWERS.get(power).copied().unwrap_or("x")))
                    .collect();
                terms(&parts)
            }
        }
    }
}

/// The **powers** of `x` the equations are written with.
const POWERS: [&str; 7] = ["", "x", "x²", "x³", "x⁴", "x⁵", "x⁶"];

/// `y = a·t1 ± b·t2 …`, without the terms whose coefficient is zero.
fn terms(parts: &[(f64, &str)]) -> String {
    let mut out = String::from("y = ");
    let mut first = true;
    for (coefficient, term) in parts {
        let text = num_text(coefficient.abs());
        if text == "0" {
            continue;
        }
        let negative = *coefficient < 0.0;
        if first {
            if negative {
                out.push('-');
            }
        } else {
            out.push_str(if negative { " - " } else { " + " });
        }
        // A coefficient of one is left out of a power of `x`: `y = x² - x + 1`.
        if text != "1" || term.is_empty() {
            out.push_str(&text);
        }
        out.push_str(term);
        first = false;
    }
    if first {
        out.push('0');
    }
    out
}

/// A coefficient as the tree writes it: up to four decimals, and in
/// scientific form when it is very small or very large.
pub fn num_text(value: f64) -> String {
    if !value.is_finite() {
        return "#NUM!".to_string();
    }
    let size = value.abs();
    if size == 0.0 {
        return "0".to_string();
    }
    if !(1.0e-4..1.0e11).contains(&size) {
        let text = format!("{value:.4E}");
        let (mantissa, exponent) = text.split_once('E').unwrap_or((text.as_str(), "0"));
        let mantissa = trim_zeros(mantissa);
        let exponent: i32 = exponent.parse().unwrap_or(0);
        return format!("{mantissa}E{}{:02}", if exponent < 0 { '-' } else { '+' }, exponent.unsigned_abs());
    }
    let text = trim_zeros(&format!("{value:.4}"));
    if text == "-0" { "0".to_string() } else { text }
}

/// The zeros at the end of a number are not written.
fn trim_zeros(text: &str) -> String {
    if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        text.to_string()
    }
}

/// **Fit** a kind of fit to the points.
///
/// The tree's own rule: there is **no fit** rather than a wrong one, so
/// the points that cannot be used are left out, and when what is left
/// cannot carry a fit there is none.
pub fn fit(kind: Trend, points: &[(f64, f64)]) -> Option<Fit> {
    let degree = kind.degree()?;
    let data: Vec<(f64, f64)> = points
        .iter()
        .filter(|(x, y)| {
            x.is_finite() && y.is_finite() && (!kind.log_x() || *x > 0.0) && (!kind.log_y() || *y > 0.0)
        })
        .map(|(x, y)| {
            let x = if kind.log_x() { x.ln() } else { *x };
            let y = if kind.log_y() { y.ln() } else { *y };
            (x, y)
        })
        .collect();
    if data.len() < 2 || data.len() <= degree {
        return None;
    }
    let count = data.len() as f64;
    let center = data.iter().map(|point| point.0).sum::<f64>() / count;
    let scale = data.iter().map(|point| (point.0 - center).abs()).fold(0.0f64, f64::max);
    if !(scale.is_finite() && scale > 0.0) {
        return None;
    }
    // The **normal equations** over the powers of the centred and
    // scaled `x`.
    let size = degree + 1;
    let mut matrix = vec![vec![0.0f64; size]; size];
    let mut right = vec![0.0f64; size];
    for (x, y) in &data {
        let u = (x - center) / scale;
        let powers: Vec<f64> = (0..=degree).map(|power| u.powi(power as i32)).collect();
        for (row, first) in powers.iter().enumerate() {
            if let Some(slot) = right.get_mut(row) {
                *slot += first * y;
            }
            for (column, second) in powers.iter().enumerate() {
                if let Some(slot) = matrix.get_mut(row).and_then(|row| row.get_mut(column)) {
                    *slot += first * second;
                }
            }
        }
    }
    let solution = solve(matrix, right)?;
    let coef = expand(&solution, center, scale)?;
    let fit = Fit { kind, coef, center, scale, a: solution, r2: None };
    let mean = data.iter().map(|point| point.1).sum::<f64>() / count;
    let mut residual = 0.0f64;
    let mut total = 0.0f64;
    for (x, y) in &data {
        let u = (x - center) / scale;
        let predicted = fit.a.iter().rev().fold(0.0, |acc, c| acc * u + c);
        residual += (y - predicted).powi(2);
        total += (y - mean).powi(2);
    }
    let r2 = (total > 0.0 && total.is_finite() && residual.is_finite()).then(|| 1.0 - residual / total);
    Some(Fit { r2, ..fit })
}

/// Solve the normal equations: elimination with partial pivoting.
///
/// `None` when the system is (nearly) singular: the tree's rule is
/// that there is no fit rather than a wrong one.
fn solve(mut matrix: Vec<Vec<f64>>, mut right: Vec<f64>) -> Option<Vec<f64>> {
    let size = right.len();
    let big = matrix.iter().flatten().fold(0.0f64, |most, value| most.max(value.abs()));
    if !(big.is_finite() && big > 0.0) {
        return None;
    }
    for column in 0..size {
        let pivot = (column..size).max_by(|left, right| {
            let one = matrix.get(*left).and_then(|row| row.get(column)).map_or(0.0, |value| value.abs());
            let other = matrix.get(*right).and_then(|row| row.get(column)).map_or(0.0, |value| value.abs());
            one.total_cmp(&other)
        })?;
        matrix.swap(column, pivot);
        right.swap(column, pivot);
        let head = *matrix.get(column)?.get(column)?;
        if !head.is_finite() || head.abs() <= big * 1.0e-13 {
            return None;
        }
        let row = matrix.get(column)?.clone();
        let value = *right.get(column)?;
        for line in column + 1..size {
            let row_mut = matrix.get_mut(line)?;
            let factor = *row_mut.get(column)? / head;
            for (cell, other) in row_mut.iter_mut().zip(&row).skip(column) {
                *cell -= factor * other;
            }
            *right.get_mut(line)? -= factor * value;
        }
    }
    let mut out = vec![0.0f64; size];
    for row in (0..size).rev() {
        let line = matrix.get(row)?;
        let sum: f64 = (row + 1..size)
            .map(|column| line.get(column).copied().unwrap_or(0.0) * out.get(column).copied().unwrap_or(0.0))
            .sum();
        let head = *line.get(row)?;
        let value = (right.get(row)? - sum) / head;
        *out.get_mut(row)? = value;
    }
    out.iter().all(|value| value.is_finite()).then_some(out)
}

/// The coefficients in `x` of `Σ aₖ·((x - centre) / scale)ᵏ`.
fn expand(a: &[f64], centre: f64, scale: f64) -> Option<Vec<f64>> {
    let mut out = vec![0.0f64; a.len()];
    for (power, coefficient) in a.iter().enumerate() {
        let lead = coefficient / scale.powi(power as i32);
        let mut binomial = 1.0f64;
        for step in 0..=power {
            if let Some(slot) = out.get_mut(step) {
                *slot += lead * binomial * (-centre).powi((power - step) as i32);
            }
            binomial = binomial * (power - step) as f64 / (step + 1) as f64;
        }
    }
    out.iter().all(|value| value.is_finite()).then_some(out)
}

/// The **moving average** of the points: the point `i` is the average
/// of the last `period` of the `y`.
///
/// The tree's own rule: there are no points when there are fewer
/// points than the period.
pub fn moving_average(points: &[(f64, f64)], period: usize) -> Vec<(f64, f64)> {
    let period = period.max(1);
    let points: Vec<(f64, f64)> =
        points.iter().copied().filter(|(x, y)| x.is_finite() && y.is_finite()).collect();
    points
        .windows(period)
        .filter_map(|window| {
            window.last().map(|(x, _)| (*x, window.iter().map(|point| point.1).sum::<f64>() / period as f64))
        })
        .collect()
}

/// The **points** of a series: the `x` of a point is the number of its
/// category when the category is a number, and the number of the
/// category otherwise, counting from one.
pub fn points_of(series: &Series, categories: &[String]) -> Vec<(f64, f64)> {
    let mut out: Vec<(f64, f64)> = Vec::new();
    for (index, value) in series.values.iter().enumerate() {
        let Some(value) = value else { continue };
        let x = categories
            .get(index)
            .and_then(|category| category.trim().parse::<f64>().ok())
            .unwrap_or(index as f64 + 1.0);
        out.push((x, *value));
    }
    out
}

commands! {
    /// **The trendline of a chart**: the fit of a kind of fit to the
    /// values of its series.
    ///
    /// The fit is **derived**: it is worked out from the cells of the
    /// sheet every time it is asked for.
    grid.chart.trendline {
        area: "grid",
        layer: "grid",
        icon: "chart",
        params: {
            /// The chart.
            chart: NodeRef,
            /// The kind of fit: `linear`, `exponential`, `logarithmic`,
            /// `power`, `polynomial` or `moving_average`.
            kind: Text,
            /// The order of a polynomial: between two and six.
            order: Option<Number>,
            /// The period of a moving average: between two and two hundred and
            /// fifty-five.
            period: Option<Number>,
            /// The name of one series: every series is fitted when it is not
            /// said.
            series: Option<Text>,
        },
        run: {
            let id = "grid.chart.trendline".to_string();
            let chart = self.chart.0;
            let Some(node) = session.document().node(chart).cloned() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no chart `{chart}`"),
                });
            };
            if node.kind != "grid.chart" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{chart}` is a `{}` and not a chart", node.kind),
                });
            }
            let kind = trend_of(&self.kind.0, self.order, self.period)
                .map_err(|message| EngineError::Failed { id: id.clone(), message })?;
            let sheet = sheet_of(session, &id, chart)?;
            let Some(area) = area_of(&node) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the chart `{chart}` reads no range"),
                });
            };
            let (categories, series) = data_of(&sheet, &area);
            let wanted = self.series.as_ref().map(|name| name.0.trim().to_string());
            let mut fitted: Vec<Json> = Vec::new();
            for one in &series {
                if let Some(wanted) = &wanted
                    && one.name.trim() != wanted
                {
                    continue;
                }
                let points = points_of(one, &categories);
                let used = points.len();
                let entry = match kind {
                    Trend::MovingAverage(period) => {
                        let average = moving_average(&points, period);
                        json!({
                            "name": one.name,
                            "kind": kind.name(),
                            "equation": Json::Null,
                            "r_squared": Json::Null,
                            "points": average
                                .iter()
                                .map(|(x, y)| json!({ "x": x, "y": y }))
                                .collect::<Vec<Json>>(),
                            "used": used,
                            "skipped": used - average.len(),
                        })
                    }
                    _ => match fit(kind, &points) {
                        Some(fit) => {
                            // The **points the fit uses**, and the value of
                            // the fit at each of them.
                            let used_points: Vec<(f64, f64)> = points
                                .iter()
                                .copied()
                                .filter(|(x, y)| {
                                    x.is_finite()
                                        && y.is_finite()
                                        && (!kind.log_x() || *x > 0.0)
                                        && (!kind.log_y() || *y > 0.0)
                                })
                                .collect();
                            let fitted_points: Vec<Json> = used_points
                                .iter()
                                .filter_map(|(x, _)| fit.eval(*x).map(|y| json!({ "x": x, "y": y })))
                                .collect();
                            json!({
                                "name": one.name,
                                "kind": kind.name(),
                                "equation": fit.equation(),
                                "r_squared": match fit.r2 {
                                    Some(value) => json!(value),
                                    None => Json::Null,
                                },
                                "points": used_points
                                    .iter()
                                    .map(|(x, y)| json!({ "x": x, "y": y }))
                                    .collect::<Vec<Json>>(),
                                "fitted": fitted_points,
                                "used": used_points.len(),
                                "skipped": used - used_points.len(),
                            })
                        }
                        None => {
                            return Err(EngineError::Failed {
                                id: id.clone(),
                                message: format!("the series `{}` cannot carry a {} fit", one.name, kind.name()),
                            });
                        }
                    },
                };
                fitted.push(entry);
            }
            if wanted.is_some() && fitted.is_empty() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the chart `{chart}` has no series of that name"),
                });
            }
            Ok(Outcome::new(format!("the chart `{chart}` is fitted by a {}", kind.name())).with_data(json!({
                "chart": chart,
                "kind": kind.name(),
                "chart_kind": kind_of_chart(&node),
                "series": fitted,
            })))

        }
    }
}
