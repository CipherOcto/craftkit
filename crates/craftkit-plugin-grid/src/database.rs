//! The **database functions**: a table of records and a table of
//! criteria.
//!
//! A **database** is a table: its first row holds the **labels** of
//! its columns, and every row after it is a **record**. A **criteria**
//! table is a table too: its first row holds the labels of its
//! columns, and every row after it is a **condition**.
//!
//! - a condition row's cells are **and**ed together: every one of
//!   them must hold;
//! - a record matches when **any** of the condition rows holds;
//! - a criteria cell that is **empty** holds always;
//! - a criteria cell that is **plain text** is a condition that the
//!   value **begins with** — as the tree's advanced filter does it —
//!   so `P` matches `Pear` and `Peach`.
//!
//! The **field** is the column the function speaks about: a **label**
//! of the database's first row, or a **number** that says which
//! column it is, counting from one.
//!
//! # What the port found
//!
//! - **The tree's criteria cell of plain text is a "begins with"
//!   pattern** and not an equality, and this tree's is the same: the
//!   text is turned into the text and a star, and the wildcard
//!   matcher of this crate does the rest.
//! - **The tree's criteria of a number is a number and not a
//!   pattern**: a cell holding the number `5` matches the number
//!   five and not the text that begins with a five. Here the same,
//!   and the number parser of the criteria module is the one.
//! - **The tree's criteria cell of a formula under a label that names
//!   no field is a computed criterion** and the host evaluates it for
//!   every record. This tree does not do that: a label that names no
//!   field is a condition that **cannot hold**, so a record does not
//!   match that condition row. The rule is refused and not guessed.
//! - **The tree's `DGET` counts the records that have a value** and
//!   not the records that match: a record whose field is empty does
//!   not count, and so a table with two matching records and one of
//!   them empty has one value and not an error. Here the same.

use crate::criteria::{Condition, compare_text, number_text};
use crate::formula::Value;
pub(crate) const NAMES: &[&str] = &[
    "DSUM", "DAVERAGE", "DCOUNT", "DCOUNTA", "DGET", "DMAX", "DMIN", "DPRODUCT", "DVAR", "DVARP", "DSTDEV",
    "DSTDEVP",
];

/// A **table** of values: rows of cells.
pub(crate) type Table = Vec<Vec<Value>>;

/// The **column** a label names in a table's first row.
fn column_of(header: &[Value], label: &str) -> Option<usize> {
    let label = label.trim();
    header.iter().position(|cell| match cell {
        Value::Text(text) => compare_text(text.trim(), label).is_eq(),
        Value::Number(_) => crate::texts::value_text(cell).is_ok_and(|text| text == label),
        _ => false,
    })
}

/// The **column** a field names: a label or a number.
fn field_of(header: &[Value], field: &Value) -> Result<Option<usize>, String> {
    match field {
        Value::Number(number) => {
            let index = number.trunc();
            if !index.is_finite() || index < 1.0 || index > header.len() as f64 {
                return Err("#VALUE!".to_string());
            }
            Ok(Some(index as usize - 1))
        }
        Value::Text(text) => column_of(header, text).map(Some).ok_or_else(|| "#VALUE!".to_string()),
        Value::Bool(_) => Err("#VALUE!".to_string()),
        Value::Error(message) => Err(message.clone()),
        Value::Empty => Ok(None),
    }
}

/// The **condition** a criteria cell is.
fn condition_of(value: &Value) -> Option<Condition> {
    match value {
        Value::Empty => None,
        Value::Text(text) => {
            let plain = !text.starts_with(['=', '<', '>'])
                && number_text(text).is_none()
                && !text.eq_ignore_ascii_case("TRUE")
                && !text.eq_ignore_ascii_case("FALSE");
            if plain && !text.ends_with('*') {
                Some(Condition::parse(&Value::Text(format!("{text}*"))))
            } else {
                Some(Condition::parse(value))
            }
        }
        other => Some(Condition::parse(other)),
    }
}

/// The **rows** of the database that match the criteria.
///
/// The rows are the ones after the header, and their numbers are
/// their places in the table.
pub(crate) fn matching_records(database: &Table, criteria: &Table) -> Vec<usize> {
    let header = database.first().map(Vec::as_slice).unwrap_or(&[]);
    let labels = criteria.first().map(Vec::as_slice).unwrap_or(&[]);
    // Every condition row: the conditions of one row.
    let mut rows: Vec<Vec<(Option<usize>, Condition)>> = Vec::new();
    for row in criteria.iter().skip(1) {
        let mut conditions = Vec::new();
        for (index, cell) in row.iter().enumerate() {
            let Some(condition) = condition_of(cell) else { continue };
            let column = labels.get(index).and_then(|label| match label {
                Value::Text(text) => column_of(header, text),
                Value::Number(_) => {
                    crate::texts::value_text(label).ok().and_then(|text| column_of(header, &text))
                }
                _ => None,
            });
            conditions.push((column, condition));
        }
        rows.push(conditions);
    }
    let all_match = rows.is_empty();
    (1..database.len())
        .filter(|record| {
            all_match
                || rows.iter().any(|conditions| {
                    conditions.iter().all(|(column, condition)| match column {
                        Some(column) => {
                            condition.matches(database[*record].get(*column).unwrap_or(&Value::Empty))
                        }
                        // A label that names no field cannot hold.
                        None => false,
                    })
                })
        })
        .collect()
}

/// The **numbers** of a field among the records that match.
fn field_numbers(database: &Table, records: &[usize], field: usize) -> Vec<f64> {
    records
        .iter()
        .filter_map(|record| match database.get(*record).and_then(|row| row.get(field)) {
            Some(Value::Number(number)) => Some(*number),
            _ => None,
        })
        .collect()
}

/// The **sum of the squares** and the **mean** of a set of numbers.
fn variance(numbers: &[f64], sample: bool) -> Result<f64, String> {
    let count = numbers.len() as f64;
    let least = if sample { 2.0 } else { 1.0 };
    if count < least {
        return Err("#DIV/0!".to_string());
    }
    let mean = numbers.iter().sum::<f64>() / count;
    let squares: f64 = numbers.iter().map(|number| (number - mean) * (number - mean)).sum();
    Ok(squares / if sample { count - 1.0 } else { count })
}

/// Call a **database function**.
pub(crate) fn call(
    name: &str,
    database: &Table,
    criteria: &Table,
    field: Option<&Value>,
) -> Result<Value, String> {
    let header = database.first().map(Vec::as_slice).unwrap_or(&[]);
    let field = match field {
        Some(field) => field_of(header, field)?,
        None => None,
    };
    let records = matching_records(database, criteria);
    let numbers = match field {
        Some(field) => field_numbers(database, &records, field),
        None => Vec::new(),
    };
    let value = match name {
        "DSUM" => numbers.iter().sum::<f64>(),
        "DAVERAGE" => {
            if numbers.is_empty() {
                return Err("#DIV/0!".to_string());
            }
            numbers.iter().sum::<f64>() / numbers.len() as f64
        }
        "DCOUNT" => match field {
            Some(field) => records
                .iter()
                .filter(|record| {
                    matches!(database.get(**record).and_then(|row| row.get(field)), Some(Value::Number(_)))
                })
                .count() as f64,
            None => records.len() as f64,
        },
        "DCOUNTA" => match field {
            Some(field) => records
                .iter()
                .filter(|record| {
                    !matches!(
                        database.get(**record).and_then(|row| row.get(field)),
                        None | Some(Value::Empty)
                    )
                })
                .count() as f64,
            None => records.len() as f64,
        },
        "DGET" => {
            let Some(field) = field else { return Err("#VALUE!".to_string()) };
            let mut found = records.iter().filter_map(|record| database[*record].get(field));
            let mut values = found.by_ref().filter(|value| {
                !matches!(value, Value::Empty) && !matches!(value, Value::Text(text) if text.is_empty())
            });
            match (values.next(), values.next()) {
                (None, _) => return Err("#VALUE!".to_string()),
                (Some(value), None) => return Ok(value.clone()),
                _ => return Err("#NUM!".to_string()),
            }
        }
        "DMAX" => numbers.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        "DMIN" => numbers.iter().copied().fold(f64::INFINITY, f64::min),
        "DPRODUCT" => {
            if numbers.is_empty() {
                0.0
            } else {
                numbers.iter().product()
            }
        }
        "DVAR" => variance(&numbers, true)?,
        "DVARP" => variance(&numbers, false)?,
        "DSTDEV" => variance(&numbers, true)?.sqrt(),
        "DSTDEVP" => variance(&numbers, false)?.sqrt(),
        _ => return Err("#VALUE!".to_string()),
    };
    // A maximum of nothing is nothing, and not a number that is not
    // there: the tree's maximum of no numbers is zero.
    let value = match name {
        "DMAX" if numbers.is_empty() => 0.0,
        "DMIN" if numbers.is_empty() => 0.0,
        _ => value,
    };
    Ok(Value::Number(value))
}
