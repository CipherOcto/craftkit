//! The **aggregates of a condition**.
//!
//! Six functions count or add what meets a condition:
//!
//! - `COUNTIFS(criteria_range1, criteria1, ...)` — the **count** of
//!   the cells that meet **every** condition;
//! - `SUMIFS(sum_range, criteria_range1, criteria1, ...)` — the
//!   **sum** of the cells that stand opposite the cells that meet
//!   every condition;
//! - `AVERAGEIFS(average_range, criteria_range1, criteria1, ...)` —
//!   the **average** of them;
//! - `MAXIFS(max_range, ...)` and `MINIFS(min_range, ...)` — the
//!   **largest** and the **smallest** of them;
//! - `AVERAGEIF(range, criteria, [average_range])` — the average of
//!   the cells that meet **one** condition.
//!
//! # The condition
//!
//! The condition is the tree's condition (the module `criteria`), and
//! it is a **value**: a text condition is a condition written out, and
//! a number or a boolean is the value itself. The wildcards are the
//! tree's, and so is the comparison of text without case.
//!
//! # The ranges
//!
//! Every range of the family is **the same shape**: the range of the
//! values and every range of the conditions are read cell by cell,
//! one beside the other, and a range that is not the shape of the
//! first is an error (`#VALUE!`) and not a shorter read.
//!
//! `AVERAGEIF` is the odd one: it takes **one** condition, and the
//! range of the values it averages may be **sized like** the range it
//! is averaged over and is anchored at its own top left cell — the
//! tree's rule for the sum range of `SUMIF`.
//!
//! # What the port found
//!
//! - **The tree's long criterion is cut and this tree's is refused.**
//!   The tree refuses a text criterion of more than two hundred and
//!   fifty-five characters when there is **one** condition, and **cuts
//!   it to its first two hundred and fifty-five characters** when
//!   there are two or more. Here the bound is the tree's, and so is
//!   the cut — a criterion of two pairs is matched by its first two
//!   hundred and fifty-five characters and not refused.
//! - **The tree's `AVERAGEIFS` of nothing is a division by zero.**
//!   The tree's `AVERAGEIFS` of no cells is `#DIV/0!`, and its
//!   `MAXIFS` and `MINIFS` of no cells are zero. So are they here.
//! - **The tree's `AVERAGEIF` is not `AVERAGEIFS` with one pair.**
//!   The tree's `AVERAGEIF` reads its values from a range that may
//!   be sized like the range it is averaged over, and `AVERAGEIFS`
//!   refuses a range that is not the same shape. The two are
//!   different rules here too.

use crate::criteria::Condition;
use crate::formula::{Area, Book, CellRef, Expr, Value};
pub const NAMES: &[&str] = &["COUNTIFS", "SUMIFS", "AVERAGEIFS", "MAXIFS", "MINIFS", "AVERAGEIF"];

/// A **pair** of a condition: a range of cells and the condition the
/// cells of it may meet.
struct Pair {
    /// The sheet the range is on.
    sheet: String,
    /// The range of cells.
    area: Area,
    /// The condition.
    condition: Condition,
}

/// What an aggregate does with the numbers it gathers.
#[derive(Clone, Copy)]
enum Aggregate {
    /// It adds them.
    Sum,
    /// It averages them.
    Average,
    /// It takes the largest.
    Max,
    /// It takes the smallest.
    Min,
}

impl Book {
    /// The **aggregate functions**.
    pub(crate) fn aggregates(
        &self,
        sheet: &str,
        name: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, CellRef)>,
    ) -> Value {
        match name {
            "COUNTIFS" => self.countifs(sheet, arguments, being),
            "SUMIFS" => self.ifs(sheet, arguments, being, Aggregate::Sum),
            "AVERAGEIFS" => self.ifs(sheet, arguments, being, Aggregate::Average),
            "MAXIFS" => self.ifs(sheet, arguments, being, Aggregate::Max),
            "MINIFS" => self.ifs(sheet, arguments, being, Aggregate::Min),
            "AVERAGEIF" => self.averageif(sheet, arguments, being),
            _ => Value::Error(format!("there is no function `{name}`")),
        }
    }

    /// The tree's `COUNTIFS`: the **count** of the cells that meet
    /// every condition.
    ///
    /// The first range of conditions is the **shape**: every other
    /// range of conditions is read cell by cell beside it, and a
    /// range that is not that shape is an error.
    fn countifs(&self, sheet: &str, arguments: &[Expr], being: &mut Vec<(String, CellRef)>) -> Value {
        let pairs = match self.pairs(sheet, arguments, being, 0) {
            Ok(pairs) => pairs,
            Err(message) => return Value::Error(message),
        };
        let Some(first) = pairs.first() else {
            return Value::Error("#VALUE!".to_string());
        };
        let shape = shape_of(&first.area);
        for pair in &pairs {
            if shape_of(&pair.area) != shape {
                return Value::Error("#VALUE!".to_string());
            }
        }
        let mut count = 0.0;
        for row in 0..shape.0 {
            for col in 0..shape.1 {
                if self.meets_every(&pairs, row, col, being) {
                    count += 1.0;
                }
            }
        }
        Value::Number(count)
    }

    /// The tree's `SUMIFS`, `AVERAGEIFS`, `MAXIFS` and `MINIFS`: the
    /// aggregate of the cells that meet every condition.
    ///
    /// The first argument is the **range of the values**, and it is
    /// the shape: every range of the conditions is read cell by cell
    /// beside it.
    fn ifs(
        &self,
        sheet: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, CellRef)>,
        aggregate: Aggregate,
    ) -> Value {
        let Some(first) = arguments.first() else {
            return Value::Error("#VALUE!".to_string());
        };
        let (values_sheet, values_area) = match self.area_of(sheet, first) {
            Ok(area) => area,
            Err(message) => return Value::Error(message),
        };
        let pairs = match self.pairs(sheet, arguments, being, 1) {
            Ok(pairs) => pairs,
            Err(message) => return Value::Error(message),
        };
        let shape = shape_of(&values_area);
        for pair in &pairs {
            if shape_of(&pair.area) != shape {
                return Value::Error("#VALUE!".to_string());
            }
        }
        let mut numbers: Vec<f64> = Vec::new();
        for row in 0..shape.0 {
            for col in 0..shape.1 {
                if !self.meets_every(&pairs, row, col, being) {
                    continue;
                }
                let cell = CellRef {
                    col: values_area.start.col + col as u32,
                    row: values_area.start.row + row as u32,
                };
                match self.value_of_guarded(&values_sheet, cell, being) {
                    Value::Number(number) => numbers.push(number),
                    Value::Error(message) => return Value::Error(message),
                    _ => {}
                }
            }
        }
        gathered(aggregate, &numbers)
    }

    /// The tree's `AVERAGEIF`: the **average** of the cells that meet
    /// one condition.
    ///
    /// The range of the values may be **sized like** the range that
    /// is averaged over and is anchored at its own top left cell, so
    /// a range that is smaller is read past its own end.
    fn averageif(&self, sheet: &str, arguments: &[Expr], being: &mut Vec<(String, CellRef)>) -> Value {
        if arguments.len() < 2 || arguments.len() > 3 {
            return Value::Error("#VALUE!".to_string());
        }
        let (range_sheet, range) = match self.area_of(sheet, &arguments[0]) {
            Ok(area) => area,
            Err(message) => return Value::Error(message),
        };
        let criterion = self.evaluate(sheet, &arguments[1], being);
        let condition = match Condition::parse_argument(&criterion) {
            Ok(condition) => condition,
            Err(message) => return Value::Error(message),
        };
        let values = match arguments.get(2) {
            Some(argument) => match self.area_of(sheet, argument) {
                Ok((name, area)) => Some((name, area)),
                Err(message) => return Value::Error(message),
            },
            None => None,
        };
        let shape = shape_of(&range);
        let mut numbers: Vec<f64> = Vec::new();
        for row in 0..shape.0 {
            for col in 0..shape.1 {
                let cell = CellRef { col: range.start.col + col as u32, row: range.start.row + row as u32 };
                let value = self.value_of_guarded(&range_sheet, cell, being);
                if !condition.matches(&value) {
                    continue;
                }
                let target = match &values {
                    Some((name, area)) => {
                        let cell =
                            CellRef { col: area.start.col + col as u32, row: area.start.row + row as u32 };
                        self.value_of_guarded(name, cell, being)
                    }
                    None => value,
                };
                match target {
                    Value::Number(number) => numbers.push(number),
                    Value::Error(message) => return Value::Error(message),
                    _ => {}
                }
            }
        }
        gathered(Aggregate::Average, &numbers)
    }

    /// The **pairs** of a list of conditions: a range and the
    /// condition it is read with.
    ///
    /// The list is read from `first`, and it is a list of **pairs**:
    /// a range, a condition, a range, a condition. A list that is not
    /// made of pairs is an error.
    ///
    /// A list of **one** pair refuses a text criterion over the
    /// bound, and a list of two or more **cuts** it — the tree's rule.
    fn pairs(
        &self,
        sheet: &str,
        arguments: &[Expr],
        being: &mut Vec<(String, CellRef)>,
        first: usize,
    ) -> Result<Vec<Pair>, String> {
        let rest = arguments.get(first..).unwrap_or(&[]);
        if rest.is_empty() || rest.len() % 2 != 0 {
            return Err("#VALUE!".to_string());
        }
        let one_pair = rest.len() == 2;
        let mut pairs = Vec::new();
        for chunk in rest.chunks(2) {
            let (Some(range), Some(criterion)) = (chunk.first(), chunk.get(1)) else {
                return Err("#VALUE!".to_string());
            };
            let (name, area) = self.area_of(sheet, range)?;
            let value = self.evaluate(sheet, criterion, being);
            let condition = if one_pair {
                Condition::parse_argument(&value)?
            } else {
                Condition::parse_argument_truncated(&value)
            };
            pairs.push(Pair { sheet: name, area, condition });
        }
        Ok(pairs)
    }

    /// Whether the cell at the **place** meets **every** condition.
    fn meets_every(
        &self,
        pairs: &[Pair],
        row: usize,
        col: usize,
        being: &mut Vec<(String, CellRef)>,
    ) -> bool {
        pairs.iter().all(|pair| {
            let cell =
                CellRef { col: pair.area.start.col + col as u32, row: pair.area.start.row + row as u32 };
            let value = self.value_of_guarded(&pair.sheet, cell, being);
            pair.condition.matches(&value)
        })
    }
}

/// The **shape** of a range: its rows and its columns.
fn shape_of(area: &Area) -> (usize, usize) {
    ((area.end.row - area.start.row + 1) as usize, (area.end.col - area.start.col + 1) as usize)
}

/// What an aggregate makes of the **numbers** it gathered.
fn gathered(aggregate: Aggregate, numbers: &[f64]) -> Value {
    match aggregate {
        Aggregate::Sum => Value::Number(numbers.iter().sum()),
        Aggregate::Average => {
            if numbers.is_empty() {
                return Value::Error("#DIV/0!".to_string());
            }
            Value::Number(numbers.iter().sum::<f64>() / numbers.len() as f64)
        }
        // The tree's `MAXIFS` and `MINIFS` of **nothing** are zero,
        // and of something are that something — a negative number
        // included.
        Aggregate::Max => match numbers.iter().copied().reduce(f64::max) {
            Some(largest) => Value::Number(largest),
            None => Value::Number(0.0),
        },
        Aggregate::Min => match numbers.iter().copied().reduce(f64::min) {
            Some(smallest) => Value::Number(smallest),
            None => Value::Number(0.0),
        },
    }
}
