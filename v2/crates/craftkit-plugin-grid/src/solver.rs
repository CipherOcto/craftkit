//! The **solver** of the grid: a linear program is solved.
//!
//! The tree's own solver: the **simplex** of the tree, with its two
//! phases, its bounds and its budget.
//!
//! # The rules
//!
//! - The **model** is numbers: the **costs** of the objective, the
//!   **rows** of the constraints and the **bounds** of the variables.
//!   The tree reads its model from the cells of a sheet, and here the
//!   model is given as numbers.
//! - A program is **minimised**: the solver of the tree minimises, and
//!   a program that is to be **maximised** is minimised with its costs
//!   turned about.
//! - The **bounds** of a variable may be left out, and a variable
//!   without a lower bound may go below zero — the tree's rule is
//!   that a bound is a bound and not a zero.
//! - The **budget** of the solver is a number of pivots, and a program
//!   that is not solved within it **stops** and says so: there is no
//!   answer rather than a wrong one.
//! - A program with **no solution** is **infeasible**, and a program
//!   that can grow without end is **unbounded**: both are said, and
//!   neither is a number.
//!
//! # What it does not settle
//!
//! - The tree's **integer** and **binary** variables, its **non-linear**
//!   solver and its **evolutionary** one are not here: this solves a
//!   linear program and nothing else.
//! - The tree's model is read from the **cells** of a sheet, and the
//!   tree writes the answer into them: here the model is given as
//!   numbers and the answer is said.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, Number, Outcome, Session};

/// The most **variables** a program may have.
pub const MAX_VARS: usize = 200;

/// The most **constraints** a program may have.
pub const MAX_CONSTRAINTS: usize = 1000;

/// The **budget** of the solver: the most pivots it may make.
///
/// The tree's own budget is a number of evaluations; here it is a
/// number of pivots, and it is a number the caller may set.
pub const MAX_PIVOTS: u64 = 2_000_000;

/// The **smallness** of a number, as the tree writes it.
const EPS: f64 = 1e-9;

/// The **sense** of a constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sense {
    /// The row is at most the right side.
    Le,
    /// The row is at least the right side.
    Ge,
    /// The row is the right side.
    Eq,
}

impl Sense {
    /// The **sense** a name asks for: the tree's own names.
    pub fn parse(text: &str) -> Option<Sense> {
        match text.trim().to_ascii_lowercase().as_str() {
            "<=" | "=<" | "le" | "≤" => Some(Sense::Le),
            ">=" | "=>" | "ge" | "≥" => Some(Sense::Ge),
            "=" | "==" | "eq" => Some(Sense::Eq),
            _ => None,
        }
    }
}

/// What a solve ended with.
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    /// The values of the variables and the objective, and whether the
    /// solver ran out of its budget.
    Solved(Vec<f64>, f64, bool),
    /// The program has no solution.
    Infeasible,
    /// The program can grow without end.
    Unbounded,
    /// The solver ran out of its budget.
    Limit,
}

/// A **column** of the transformed variables: the variable it stands
/// for and its sign.
#[derive(Debug, Clone, Copy)]
struct Map {
    var: usize,
    sign: f64,
}

/// A **row of the bounds**: the columns it names, its sense and its
/// right side.
type BoundRow = (Vec<(usize, f64)>, Sense, f64);

/// The transformed variables of a program.
struct VariableMap {
    columns: Vec<Map>,
    offset: Vec<f64>,
    bound_rows: Vec<BoundRow>,
}

/// The **columns** of the transformed variables.
fn map_variables(count: usize, low: &[f64], high: &[f64]) -> VariableMap {
    let mut columns: Vec<Map> = Vec::new();
    let mut offset = vec![0.0; count];
    let mut bound_rows: Vec<BoundRow> = Vec::new();
    for index in 0..count {
        let lower = low.get(index).copied().unwrap_or(f64::NEG_INFINITY);
        let upper = high.get(index).copied().unwrap_or(f64::INFINITY);
        if lower.is_finite() {
            if let Some(value) = offset.get_mut(index) {
                *value = lower;
            }
            columns.push(Map { var: index, sign: 1.0 });
            if upper.is_finite() {
                bound_rows.push((vec![(columns.len() - 1, 1.0)], Sense::Le, upper - lower));
            }
        } else if upper.is_finite() {
            if let Some(value) = offset.get_mut(index) {
                *value = upper;
            }
            columns.push(Map { var: index, sign: -1.0 });
        } else {
            columns.push(Map { var: index, sign: 1.0 });
            columns.push(Map { var: index, sign: -1.0 });
        }
    }
    VariableMap { columns, offset, bound_rows }
}

/// The rows of the program, over the transformed variables.
fn normalize_rows(
    rows: &[(Vec<f64>, Sense, f64)],
    bound_rows: Vec<BoundRow>,
    columns: &[Map],
    offset: &[f64],
) -> Vec<(Vec<f64>, Sense, f64)> {
    let width = columns.len();
    let mut transformed: Vec<(Vec<f64>, Sense, f64)> = Vec::with_capacity(rows.len() + bound_rows.len());
    for (coefficients, sense, rhs) in rows {
        let mut row = vec![0.0; width];
        for (index, mapping) in columns.iter().enumerate() {
            if let Some(value) = row.get_mut(index) {
                *value = coefficients.get(mapping.var).copied().unwrap_or(0.0) * mapping.sign;
            }
        }
        let shift: f64 = coefficients.iter().zip(offset).map(|(coefficient, base)| coefficient * base).sum();
        transformed.push((row, *sense, rhs - shift));
    }
    for (entries, sense, rhs) in bound_rows {
        let mut row = vec![0.0; width];
        for (index, value) in entries {
            if let Some(slot) = row.get_mut(index) {
                *slot = value;
            }
        }
        transformed.push((row, sense, rhs));
    }
    // A row with a **negative** right side is turned about, so that
    // every right side is at least zero.
    transformed
        .into_iter()
        .map(|(row, sense, rhs)| {
            if rhs < 0.0 {
                let sense = match sense {
                    Sense::Le => Sense::Ge,
                    Sense::Ge => Sense::Le,
                    Sense::Eq => Sense::Eq,
                };
                (row.iter().map(|value| -value).collect(), sense, -rhs)
            } else {
                (row, sense, rhs)
            }
        })
        .collect()
}

/// The **tableau** of the solver.
struct Tableau {
    rows: Vec<Vec<f64>>,
    basis: Vec<usize>,
    width: usize,
    decision_cols: usize,
    slack_cols: usize,
    artificial_cols: usize,
}

/// Build the **tableau** of the program.
fn build_tableau(
    normalized: &[(Vec<f64>, Sense, f64)],
    decision_cols: usize,
    slack_cols: usize,
    artificial_cols: usize,
) -> Tableau {
    let width = decision_cols + slack_cols + artificial_cols;
    let mut rows: Vec<Vec<f64>> = Vec::with_capacity(normalized.len());
    let mut basis = vec![0usize; normalized.len()];
    let (mut slack, mut artificial) = (decision_cols, decision_cols + slack_cols);
    for (index, (coefficients, sense, rhs)) in normalized.iter().enumerate() {
        let mut row = vec![0.0; width + 1];
        for (value, coefficient) in row.iter_mut().zip(coefficients) {
            *value = *coefficient;
        }
        let basic = match sense {
            Sense::Le => {
                if let Some(value) = row.get_mut(slack) {
                    *value = 1.0;
                }
                let basic = slack;
                slack += 1;
                basic
            }
            Sense::Ge => {
                if let Some(value) = row.get_mut(slack) {
                    *value = -1.0;
                }
                slack += 1;
                if let Some(value) = row.get_mut(artificial) {
                    *value = 1.0;
                }
                let basic = artificial;
                artificial += 1;
                basic
            }
            Sense::Eq => {
                if let Some(value) = row.get_mut(artificial) {
                    *value = 1.0;
                }
                let basic = artificial;
                artificial += 1;
                basic
            }
        };
        if let Some(value) = row.get_mut(width) {
            *value = *rhs;
        }
        if let Some(slot) = basis.get_mut(index) {
            *slot = basic;
        }
        rows.push(row);
    }
    Tableau { rows, basis, width, decision_cols, slack_cols, artificial_cols }
}

/// The **first phase** of the solver: the artificial columns are taken
/// out of the basis.
fn phase_one(
    tableau: &mut Tableau,
    normalized: &[(Vec<f64>, Sense, f64)],
    budget: &mut u64,
) -> Result<(), Verdict> {
    if tableau.artificial_cols == 0 {
        return Ok(());
    }
    let mut objective = vec![0.0; tableau.width + 1];
    for column in tableau.decision_cols + tableau.slack_cols..tableau.width {
        if let Some(value) = objective.get_mut(column) {
            *value = 1.0;
        }
    }
    for (index, row) in tableau.rows.iter().enumerate() {
        if tableau.basis.get(index).is_some_and(|basic| *basic >= tableau.decision_cols + tableau.slack_cols)
        {
            for (value, coefficient) in objective.iter_mut().zip(row) {
                *value -= coefficient;
            }
        }
    }
    let all = vec![true; tableau.width];
    if matches!(simplex(&mut tableau.rows, &mut tableau.basis, &mut objective, &all, budget), Step::Limit) {
        return Err(Verdict::Limit);
    }
    let scale = normalized.iter().map(|row| row.2.abs()).fold(1.0f64, f64::max);
    if -objective.get(tableau.width).copied().unwrap_or(0.0) > 1e-7 * scale {
        return Err(Verdict::Infeasible);
    }
    for index in 0..tableau.rows.len() {
        if tableau.basis.get(index).is_none_or(|basic| *basic < tableau.decision_cols + tableau.slack_cols) {
            continue;
        }
        let pivot_column = tableau.rows.get(index).and_then(|row| {
            (0..tableau.decision_cols + tableau.slack_cols)
                .find(|column| row.get(*column).is_some_and(|value| value.abs() > EPS))
        });
        if let Some(column) = pivot_column {
            pivot(&mut tableau.rows, &mut objective, index, column);
            if let Some(basic) = tableau.basis.get_mut(index) {
                *basic = column;
            }
        }
    }
    Ok(())
}

/// Pivot the tableau on `(row, column)`.
fn pivot(tableau: &mut [Vec<f64>], objective: &mut [f64], row: usize, column: usize) {
    let Some(prow) = tableau.get_mut(row) else { return };
    let value = prow.get(column).copied().unwrap_or(0.0);
    if value == 0.0 {
        return;
    }
    for cell in prow.iter_mut() {
        *cell /= value;
    }
    let prow = prow.clone();
    for (index, line) in tableau.iter_mut().enumerate() {
        if index == row {
            continue;
        }
        let factor = line.get(column).copied().unwrap_or(0.0);
        if factor != 0.0 {
            for (cell, other) in line.iter_mut().zip(&prow) {
                *cell -= factor * other;
            }
        }
    }
    let factor = objective.get(column).copied().unwrap_or(0.0);
    if factor != 0.0 {
        for (cell, other) in objective.iter_mut().zip(&prow) {
            *cell -= factor * other;
        }
    }
}

/// How a step of the solver ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    /// The program is solved.
    Optimal,
    /// The program can grow without end.
    Unbounded,
    /// The solver ran out of its budget.
    Limit,
}

/// One **step** of the simplex: the tree's own rule.
fn simplex(
    tableau: &mut [Vec<f64>],
    basis: &mut [usize],
    objective: &mut [f64],
    allowed: &[bool],
    budget: &mut u64,
) -> Step {
    let width = objective.len().saturating_sub(1);
    let cost = (tableau.len() as u64 + 1).saturating_mul(width as u64 + 1).max(1);
    let mut stalled = 0u32;
    loop {
        if *budget < cost {
            return Step::Limit;
        }
        *budget -= cost;
        // The **stall** of the solver: after so many steps without
        // moving, the columns are taken in Bland's order, so that the
        // solver cannot cycle.
        let bland = stalled > 50;
        let mut enter: Option<usize> = None;
        let mut best = -EPS;
        for (column, reduced) in objective.iter().take(width).enumerate() {
            if !allowed.get(column).copied().unwrap_or(false) || *reduced >= -EPS {
                continue;
            }
            if bland {
                enter = Some(column);
                break;
            }
            if *reduced < best {
                best = *reduced;
                enter = Some(column);
            }
        }
        let Some(column) = enter else { return Step::Optimal };
        let mut leave: Option<(usize, f64)> = None;
        for (index, row) in tableau.iter().enumerate() {
            let coefficient = row.get(column).copied().unwrap_or(0.0);
            if coefficient <= EPS {
                continue;
            }
            let ratio = row.get(width).copied().unwrap_or(0.0) / coefficient;
            let better = match leave {
                None => true,
                Some((held, held_ratio)) => {
                    ratio < held_ratio - 1e-12
                        || (ratio <= held_ratio + 1e-12 && basis.get(index) < basis.get(held))
                }
            };
            if better {
                leave = Some((index, ratio));
            }
        }
        let Some((row, ratio)) = leave else { return Step::Unbounded };
        stalled = if ratio <= EPS { stalled + 1 } else { 0 };
        pivot(tableau, objective, row, column);
        if let Some(basic) = basis.get_mut(row) {
            *basic = column;
        }
    }
}

/// **Solve** a linear program: minimise `costs · x` over the rows and
/// the bounds.
///
/// The tree's own rule: there is **no answer** rather than a wrong one,
/// so a program that has no solution is infeasible, a program that can
/// grow without end is unbounded, and a program that is not solved
/// within the budget stops.
pub fn solve_lp(
    costs: &[f64],
    rows: &[(Vec<f64>, Sense, f64)],
    low: &[f64],
    high: &[f64],
    budget: &mut u64,
) -> Verdict {
    if low.iter().zip(high).any(|(one, other)| one > other) {
        return Verdict::Infeasible;
    }
    // `x_j = offset + Σ sign · y_k`, with `y ≥ 0`.
    let VariableMap { columns, offset, bound_rows } = map_variables(costs.len(), low, high);
    let width = columns.len();
    let normalized = normalize_rows(rows, bound_rows, &columns, &offset);
    let slack_count = normalized.iter().filter(|row| row.1 != Sense::Eq).count();
    let artificial_count = normalized.iter().filter(|row| row.1 != Sense::Le).count();
    let mut tableau = build_tableau(&normalized, width, slack_count, artificial_count);
    if let Err(outcome) = phase_one(&mut tableau, &normalized, budget) {
        return outcome;
    }
    let Tableau { rows: mut table, mut basis, width, decision_cols, slack_cols, .. } = tableau;
    let is_artificial = |column: usize| column >= decision_cols + slack_cols && column < width;
    // The **second phase**: the costs of the program, and the
    // artificial columns may not come back.
    let mut cost = vec![0.0; width + 1];
    for (index, mapping) in columns.iter().enumerate() {
        if let Some(value) = cost.get_mut(index) {
            *value = costs.get(mapping.var).copied().unwrap_or(0.0) * mapping.sign;
        }
    }
    let mut objective = cost.clone();
    for (index, row) in table.iter().enumerate() {
        let basic_cost = basis.get(index).and_then(|basic| cost.get(*basic)).copied().unwrap_or(0.0);
        if basic_cost != 0.0 {
            for (value, coefficient) in objective.iter_mut().zip(row) {
                *value -= basic_cost * coefficient;
            }
        }
    }
    let allowed: Vec<bool> = (0..width).map(|column| !is_artificial(column)).collect();
    match simplex(&mut table, &mut basis, &mut objective, &allowed, budget) {
        Step::Limit => return Verdict::Limit,
        Step::Unbounded => return Verdict::Unbounded,
        Step::Optimal => {}
    }
    let mut values = vec![0.0; width];
    for (index, row) in table.iter().enumerate() {
        if let (Some(basic), Some(value)) = (basis.get(index), row.get(width))
            && let Some(slot) = values.get_mut(*basic)
        {
            *slot = *value;
        }
    }
    let mut solution = offset;
    for (index, mapping) in columns.iter().enumerate() {
        if let Some(slot) = solution.get_mut(mapping.var) {
            *slot += mapping.sign * values.get(index).copied().unwrap_or(0.0);
        }
    }
    let objective: f64 = costs.iter().zip(&solution).map(|(one, other)| one * other).sum();
    Verdict::Solved(solution, objective, true)
}

/// The **numbers** of a list: the tree's own rule is that a number that
/// is not a number is refused.
fn numbers_of(value: &Json, what: &str) -> Result<Vec<f64>, String> {
    let Some(list) = value.as_array() else {
        return Err(format!("the {what} is a list of numbers"));
    };
    let mut out: Vec<f64> = Vec::with_capacity(list.len());
    for number in list {
        let Some(number) = number.as_f64() else {
            return Err(format!("the {what} holds a number that is not a number"));
        };
        if !number.is_finite() {
            return Err(format!("the {what} holds a number that is not finite"));
        }
        out.push(number);
    }
    Ok(out)
}

/// Solve a **linear program**.
///
/// The model is **numbers**: the costs of the objective, the rows of
/// the constraints and the bounds of the variables. The answer is
/// **said**, and nothing is written down.
#[derive(Command)]
#[command(id = "grid.solver.solve", area = "grid", layer = "grid", icon = "check")]
pub struct SolveProgram {
    /// The costs of the objective: a list of numbers.
    pub objective: Option<Json>,
    /// The rows of the constraints: a list of
    /// `{coefficients, sense, rhs}`.
    pub constraints: Option<Json>,
    /// The bounds of the variables: a list of `[low, high]`, and
    /// `null` is a bound that is not there.
    pub bounds: Option<Json>,
    /// Whether the program is **maximised**: it is minimised when this
    /// is not given.
    pub maximise: Option<Bool>,
    /// The **budget** of the solver: the most pivots it may make.
    pub budget: Option<Number>,
}

impl SolveProgram {
    fn run(&self, _session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.solver.solve".to_string();
        let Some(objective) = &self.objective else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "the program has an objective".to_string(),
            });
        };
        let mut costs = numbers_of(objective, "objective")
            .map_err(|message| EngineError::Failed { id: id.clone(), message })?;
        if costs.is_empty() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "the objective has no variable in it".to_string(),
            });
        }
        if costs.len() > MAX_VARS {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "a program has {MAX_VARS} variables at the most, and this one has {}",
                    costs.len()
                ),
            });
        }
        // The **rows** of the constraints.
        let mut rows: Vec<(Vec<f64>, Sense, f64)> = Vec::new();
        if let Some(constraints) = &self.constraints {
            let Some(list) = constraints.as_array() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the constraints are a list of rows".to_string(),
                });
            };
            if list.len() > MAX_CONSTRAINTS {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "a program has {MAX_CONSTRAINTS} constraints at the most, and this one has {}",
                        list.len()
                    ),
                });
            }
            for row in list {
                let coefficients = row
                    .get("coefficients")
                    .ok_or_else(|| EngineError::Failed {
                        id: id.clone(),
                        message: "a constraint has no coefficients".to_string(),
                    })
                    .and_then(|value| {
                        numbers_of(value, "coefficients of a constraint")
                            .map_err(|message| EngineError::Failed { id: id.clone(), message })
                    })?;
                if coefficients.len() != costs.len() {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "a constraint has {} coefficient(s) and the objective has {} variable(s)",
                            coefficients.len(),
                            costs.len()
                        ),
                    });
                }
                let sense =
                    row.get("sense").and_then(Json::as_str).and_then(Sense::parse).ok_or_else(|| {
                        EngineError::Failed {
                            id: id.clone(),
                            message: "the sense of a constraint is `<=`, `>=` or `=`".to_string(),
                        }
                    })?;
                let rhs =
                    row.get("rhs").and_then(Json::as_f64).filter(|value| value.is_finite()).ok_or_else(
                        || EngineError::Failed {
                            id: id.clone(),
                            message: "the right side of a constraint is a number".to_string(),
                        },
                    )?;
                rows.push((coefficients, sense, rhs));
            }
        }
        // The **bounds** of the variables: without them a variable may
        // go below zero.
        let mut low = vec![f64::NEG_INFINITY; costs.len()];
        let mut high = vec![f64::INFINITY; costs.len()];
        if let Some(bounds) = &self.bounds {
            let Some(list) = bounds.as_array() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the bounds are a list of pairs".to_string(),
                });
            };
            if list.len() != costs.len() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "there are {} bound(s) and the objective has {} variable(s)",
                        list.len(),
                        costs.len()
                    ),
                });
            }
            for (index, bound) in list.iter().enumerate() {
                let Some(pair) = bound.as_array() else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "a bound is a pair of numbers".to_string(),
                    });
                };
                if pair.len() != 2 {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "a bound is a pair of numbers".to_string(),
                    });
                }
                if let Some(slot) = low.get_mut(index) {
                    *slot = match pair.first().and_then(Json::as_f64) {
                        Some(value) if value.is_finite() => value,
                        _ => f64::NEG_INFINITY,
                    };
                }
                if let Some(slot) = high.get_mut(index) {
                    *slot = match pair.get(1).and_then(Json::as_f64) {
                        Some(value) if value.is_finite() => value,
                        _ => f64::INFINITY,
                    };
                }
            }
        }
        let maximise = self.maximise.map(|flag| flag.0).unwrap_or(false);
        if maximise {
            for cost in costs.iter_mut() {
                *cost = -*cost;
            }
        }
        let mut budget = match self.budget.map(|value| value.0) {
            None => MAX_PIVOTS,
            Some(value) if !value.is_finite() || value < 1.0 || value > MAX_PIVOTS as f64 => {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the budget of the solver is between 1 and {MAX_PIVOTS} pivots"),
                });
            }
            Some(value) => value as u64,
        };
        let outcome = solve_lp(&costs, &rows, &low, &high, &mut budget);
        let (status, values, objective) = match outcome {
            Verdict::Solved(values, objective, _converged) => {
                let objective = if maximise { -objective } else { objective };
                ("optimal", values, objective)
            }
            Verdict::Infeasible => ("infeasible", Vec::new(), f64::NAN),
            Verdict::Unbounded => ("unbounded", Vec::new(), f64::NAN),
            Verdict::Limit => ("limit", Vec::new(), f64::NAN),
        };
        let values: Vec<Json> = values.iter().map(|value| json!(value)).collect();
        Ok(Outcome::new(format!("the program is {status}")).with_data(json!({
            "status": status,
            "objective": if objective.is_finite() { json!(objective) } else { Json::Null },
            "values": values,
        })))
    }
}
