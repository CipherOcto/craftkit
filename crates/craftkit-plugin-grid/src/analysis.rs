//! The **analysis reports**: a statistic of a range, written as a table.
//!
//! A report is read from a **range** of a sheet and written as a table
//! of **labels** and **values**. The table goes either to a **range**
//! of a sheet or to a **new sheet**.
//!
//! The report is **live**: the values are **formulas** over the range
//! the report is read from, so an edit of the data is an edit of the
//! report. The tree's own functions do the work — `AVERAGE`,
//! `STDEV.S`, `KURT`, `T.INV.2T` — and so the tree's own rules about
//! them hold: a spread of fewer than two numbers is `#DIV/0!`, and a
//! `k` that is not in the numbers is `#NUM!`.
//!
//! The **variables** of a report are the **columns** of the range, or
//! the **rows** of it. With `labels`, the first row (or the first
//! column) holds the **names** of the variables, and it is not part of
//! the numbers.
//!
//! The rules:
//!
//! - the range holds **numbers**; a cell that holds text is refused,
//!   because a statistic of a text is not a statistic;
//! - a cell that holds **nothing** is skipped — it is not a number and
//!   it is not a text;
//! - the report **refuses** to be written over cells that hold
//!   something, unless `overwrite` says it may.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Bool, EngineError, NodeRef, Number, Outcome, Session, Text};

use crate::formula::{Area, CellRef, Cells, Value};

/// The most cells one report may read.
const MAX_INPUT_CELLS: u64 = 1_000_000;

/// The most cells one report may write.
const MAX_OUTPUT_CELLS: u64 = 4_000_000;

/// The most variables in a correlation or a covariance matrix — the
/// work of one grows with its square.
const MAX_MATRIX_VARS: usize = 256;

/// One **variable** of a report: a column (or a row) of the input.
struct Variable {
    /// The **cell** that holds the name of the variable: the first
    /// cell of the column (or of the row).
    name_cell: CellRef,
    /// The **cells** of the variable: the numbers it is made of.
    area: Area,
}

/// The **qualified** name of a sheet, as a formula reads it: `Sheet1!`
/// or `'My Sheet'!`.
fn qualified(name: &str, id: &str) -> Result<String, EngineError> {
    if name.is_empty() {
        return Err(EngineError::Failed { id: id.to_string(), message: "the sheet has no name".to_string() });
    }
    if name.contains('\'') {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the name of the sheet `{name}` cannot be used in a formula"),
        });
    }
    let simple = name.chars().all(|character| character.is_ascii_alphanumeric() || character == '_');
    if simple { Ok(format!("{name}!")) } else { Ok(format!("'{name}'!")) }
}

/// The **absolute** address of an area, as a formula reads it:
/// `$A$2:$A$11`.
fn absolute(area: &Area) -> String {
    let first = CellRef { col: area.start.col, row: area.start.row };
    let last = CellRef { col: area.end.col, row: area.end.row };
    format!(
        "${}${}:${}${}",
        CellRef::column_text(first.col),
        first.row + 1,
        CellRef::column_text(last.col),
        last.row + 1
    )
}

/// The **cells** of a range that is one row or one column.
fn line_cells(area: &Area, id: &str) -> Result<Vec<CellRef>, EngineError> {
    if area.start.col == area.end.col {
        return Ok((area.start.row..=area.end.row).map(|row| CellRef { col: area.start.col, row }).collect());
    }
    if area.start.row == area.end.row {
        return Ok((area.start.col..=area.end.col).map(|col| CellRef { col, row: area.start.row }).collect());
    }
    Err(EngineError::Failed {
        id: id.to_string(),
        message: "the range must be one row or one column".to_string(),
    })
}

/// The **address** of a cell of a report, as a formula reads it:
/// `$D$2`.
fn address(col: u32, row: u32, origin: CellRef) -> String {
    format!("${}${}", CellRef::column_text(origin.col + col), origin.row + row + 1)
}

/// The **variables** of a range: its columns, or its rows.
///
/// With `labels`, the first cell of each variable names it, and it is
/// not one of the numbers.
fn variables(area: &Area, by_rows: bool, labels: bool) -> Vec<Variable> {
    let mut out = Vec::new();
    if by_rows {
        for row in area.start.row..=area.end.row {
            let first = if labels { area.start.col + 1 } else { area.start.col };
            out.push(Variable {
                name_cell: CellRef { col: area.start.col, row },
                area: Area { start: CellRef { col: first, row }, end: CellRef { col: area.end.col, row } },
            });
        }
    } else {
        for col in area.start.col..=area.end.col {
            let first = if labels { area.start.row + 1 } else { area.start.row };
            out.push(Variable {
                name_cell: CellRef { col, row: area.start.row },
                area: Area { start: CellRef { col, row: first }, end: CellRef { col, row: area.end.row } },
            });
        }
    }
    out
}

/// The **name** of a variable: what its first cell holds, or
/// `Column1`, `Column2`, … .
fn name_of(variable: &Variable, cells: &Cells, index: usize) -> String {
    if let Some(Value::Text(text)) = cells.get(variable.name_cell).and_then(|entry| entry.value.clone()) {
        let text = text.trim().to_string();
        if !text.is_empty() {
            return text;
        }
    }
    format!("Column{}", index + 1)
}

/// The **numbers** of a variable, and whether they are numbers at all.
///
/// A cell that holds **nothing** is skipped; a cell that holds
/// anything but a number is refused.
fn numbers_of(variable: &Variable, cells: &Cells, id: &str) -> Result<(), EngineError> {
    for row in variable.area.start.row..=variable.area.end.row {
        for col in variable.area.start.col..=variable.area.end.col {
            let cell = CellRef { col, row };
            match cells.get(cell).and_then(|entry| entry.value.clone()) {
                None | Some(Value::Empty) => continue,
                Some(Value::Number(_)) => continue,
                Some(_) => {
                    return Err(EngineError::Failed {
                        id: id.to_string(),
                        message: format!("the range holds text ({} isn't a number)", cell.to_text()),
                    });
                }
            }
        }
    }
    Ok(())
}

/// The **rows** of the summary of a variable: the labels and the
/// formulas over the variable's range.
fn summary_rows(range: &str) -> Vec<(String, String)> {
    vec![
        ("Mean".to_string(), format!("=AVERAGE({range})")),
        ("Standard Error".to_string(), format!("=STDEV.S({range})/SQRT(COUNT({range}))")),
        ("Median".to_string(), format!("=MEDIAN({range})")),
        ("Mode".to_string(), format!("=MODE({range})")),
        ("Standard Deviation".to_string(), format!("=STDEV.S({range})")),
        ("Sample Variance".to_string(), format!("=VAR.S({range})")),
        ("Kurtosis".to_string(), format!("=KURT({range})")),
        ("Skewness".to_string(), format!("=SKEW({range})")),
        ("Range".to_string(), format!("=MAX({range})-MIN({range})")),
        ("Minimum".to_string(), format!("=MIN({range})")),
        ("Maximum".to_string(), format!("=MAX({range})")),
        ("Sum".to_string(), format!("=SUM({range})")),
        ("Count".to_string(), format!("=COUNT({range})")),
    ]
}

/// The name of a report's sheet: the first one that is free.
fn a_free_name(taken: &[String], base: &str) -> String {
    for number in 1..1000 {
        let name = if number == 1 { base.to_string() } else { format!("{base}{number}") };
        if !taken.iter().any(|other| other.eq_ignore_ascii_case(&name)) {
            return name;
        }
    }
    base.to_string()
}

/// The **sheets** of the workbook a sheet is in, by name.
fn taken_names(session: &Session, sheet: u64) -> Vec<String> {
    session
        .document()
        .walk()
        .find(|node| node.children.iter().any(|child| child.id == sheet))
        .map(|node| {
            node.children
                .iter()
                .filter(|child| child.kind == "grid.sheet")
                .map(|child| child.name.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// The **workbook** a sheet is in.
fn workbook_of(session: &Session, sheet: u64) -> Option<u64> {
    session
        .document()
        .walk()
        .find(|node| node.children.iter().any(|child| child.id == sheet))
        .map(|node| node.id)
}

/// Where a report goes: a **range** of a sheet, or a **new sheet**.
enum Target {
    /// The report goes to the cells of a sheet, from a corner.
    Range { sheet: u64, at: CellRef },
    /// The report goes to a sheet of its own.
    Sheet { name: String },
}

/// The **target** of a report: the range of the sheet, or the new
/// sheet.
fn target_of(
    session: &Session,
    id: &str,
    sheet: u64,
    output: Option<&Text>,
    new_sheet: Option<&Text>,
    base: &str,
) -> Result<Target, EngineError> {
    match (output, new_sheet) {
        (Some(_), Some(_)) => Err(EngineError::Failed {
            id: id.to_string(),
            message: "the report goes to a range of a sheet or to a new sheet, not to both".to_string(),
        }),
        (Some(output), None) => {
            let text = output.0.trim();
            let cell = CellRef::parse(text).ok_or_else(|| EngineError::Failed {
                id: id.to_string(),
                message: format!("`{text}` is not a cell"),
            })?;
            Ok(Target::Range { sheet, at: cell })
        }
        (None, _) => {
            let taken = taken_names(session, sheet);
            let name = match new_sheet {
                Some(name) => {
                    let name = name.0.trim().to_string();
                    if name.is_empty() {
                        return Err(EngineError::Failed {
                            id: id.to_string(),
                            message: "the new sheet has no name".to_string(),
                        });
                    }
                    if taken.iter().any(|other| other.eq_ignore_ascii_case(&name)) {
                        return Err(EngineError::Failed {
                            id: id.to_string(),
                            message: format!("there is already a sheet called `{name}`"),
                        });
                    }
                    name
                }
                None => a_free_name(&taken, base),
            };
            Ok(Target::Sheet { name })
        }
    }
}

/// Writes a report into its place: a **range** of a sheet, or a **new
/// sheet**.
///
/// It returns the id and the name of the sheet the report is in.
#[allow(clippy::too_many_arguments)]
fn place_report(
    session: &mut Session,
    id: &str,
    target: Target,
    cells: &Cells,
    rows: u32,
    columns: u32,
    overwrite: bool,
    source_sheet: u64,
) -> Result<(u64, String), EngineError> {
    match target {
        Target::Range { sheet, at } => {
            write_into(session, id, sheet, at, cells, rows, columns, overwrite)?;
            let name = session.document().node(sheet).map(|node| node.name.clone()).unwrap_or_default();
            Ok((sheet, name))
        }
        Target::Sheet { name } => {
            let Some(workbook) = workbook_of(session, source_sheet) else {
                return Err(EngineError::Failed {
                    id: id.to_string(),
                    message: format!("the sheet `{source_sheet}` is not in a workbook"),
                });
            };
            let new_sheet = session.document_mut()?.add_node_with_under(
                Some(workbook),
                name.clone(),
                "grid.sheet",
                None,
                json!({
                    "cells": {},
                    "view": crate::view::SheetView::default().to_json(),
                    "zoom": 100,
                }),
            );
            if let Some(node) = session.document_mut()?.node_mut(new_sheet) {
                node.data["cells"] = crate::cells_into(cells);
            }
            Ok((new_sheet, name))
        }
    }
}

/// The **matrix** of a statistic of the variables of a range.
///
/// The names of the variables are in the first row and in the first
/// column, and the values are in the lower triangle of the square
/// they make.
fn matrix_cells(
    variables: &[(String, String)],
    diagonal: &dyn Fn(usize) -> Option<String>,
    off: &dyn Fn(usize, usize) -> String,
) -> Cells {
    let mut cells = Cells::new();
    for (index, (name, _)) in variables.iter().enumerate() {
        cells.set_value(CellRef { col: index as u32 + 1, row: 0 }, Value::Text(name.clone()));
        cells.set_value(CellRef { col: 0, row: index as u32 + 1 }, Value::Text(name.clone()));
    }
    for row in 0..variables.len() {
        for column in 0..=row {
            let cell = CellRef { col: column as u32 + 1, row: row as u32 + 1 };
            let formula = if row == column {
                match diagonal(row) {
                    Some(formula) => formula,
                    None => off(row, column),
                }
            } else {
                off(row, column)
            };
            cells.set_formula(cell, formula);
        }
    }
    cells
}

/// The **variables** of a range, with their names and their cells.
fn variables_of(area: &Area, by_rows: bool, labels: bool, cells: &Cells) -> Vec<(String, Area)> {
    variables(area, by_rows, labels)
        .iter()
        .enumerate()
        .map(|(index, variable)| (name_of(variable, cells, index), variable.area))
        .collect()
}

/// The **matrix** report: the correlation or the covariance of the
/// variables of a range.
#[allow(clippy::too_many_arguments)]
fn matrix_of(
    session: &mut Session,
    id: &str,
    kind: &str,
    sheet: u64,
    range: &str,
    output: Option<&Text>,
    new_sheet: Option<&Text>,
    labels: bool,
    by_rows: bool,
    overwrite: bool,
) -> Result<craftkit_sdk::serde_json::Value, EngineError> {
    let Some(node) = session.document().node(sheet).cloned() else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no sheet `{sheet}`"),
        });
    };
    if node.kind != "grid.sheet" {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the node `{sheet}` is not a sheet"),
        });
    }
    let area = crate::fill::area_at(range, id)?;
    if area.count() > MAX_INPUT_CELLS {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the range is too large (at most {MAX_INPUT_CELLS} cells)"),
        });
    }
    let cells = crate::cells_of(&node);
    let variables = variables_of(&area, by_rows, labels, &cells);
    if variables.len() > MAX_MATRIX_VARS {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("too many variables (at most {MAX_MATRIX_VARS})"),
        });
    }
    for (_, area) in &variables {
        for row in area.start.row..=area.end.row {
            for col in area.start.col..=area.end.col {
                match cells.get(CellRef { col, row }).and_then(|entry| entry.value.clone()) {
                    None | Some(Value::Empty) => continue,
                    Some(Value::Number(_)) => continue,
                    Some(_) => {
                        return Err(EngineError::Failed {
                            id: id.to_string(),
                            message: format!(
                                "the range holds text ({} isn't a number)",
                                CellRef { col, row }.to_text()
                            ),
                        });
                    }
                }
            }
        }
    }
    let target = target_of(session, id, sheet, output, new_sheet, kind)?;
    let prefix = match &target {
        Target::Range { sheet: target_sheet, .. } if *target_sheet == sheet => String::new(),
        _ => qualified(&node.name, id)?,
    };
    let ranges: Vec<(String, String)> =
        variables.iter().map(|(name, area)| (name.clone(), format!("{prefix}{}", absolute(area)))).collect();
    let correlation = kind == "correlation";
    let cells = matrix_cells(
        &ranges,
        &|index| {
            if correlation {
                Some("=1".to_string())
            } else {
                let range = &ranges[index].1;
                Some(format!("=COVARIANCE.P({range},{range})"))
            }
        },
        &|row, column| {
            let (x, y) = (&ranges[row].1, &ranges[column].1);
            if correlation { format!("=CORREL({x},{y})") } else { format!("=COVARIANCE.P({x},{y})") }
        },
    );
    let rows = ranges.len() as u32 + 1;
    let columns = rows;
    let (sheet_id, sheet_name) = place_report(session, id, target, &cells, rows, columns, overwrite, sheet)?;
    Ok(json!({
        "tool": kind,
        "sheet": sheet_id,
        "name": sheet_name,
        "range": format!("A1:{}", CellRef { col: columns - 1, row: rows - 1 }.to_text()),
        "rows": rows,
        "cols": columns,
        "variables": ranges.len(),
    }))
}

/// A **sample** of a report: a range that is one row or one column,
/// with its name and its numbers.
struct Sample {
    /// The **name** of the variable.
    name: String,
    /// The **cells** of the numbers of it.
    area: Area,
}

/// The **sample** of a range: one row or one column of numbers.
fn sample_of(
    area: &Area,
    labels: bool,
    cells: &Cells,
    fallback: &str,
    id: &str,
) -> Result<Sample, EngineError> {
    let mut line = line_cells(area, id)?;
    if line.is_empty() {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: "the input range is empty".to_string(),
        });
    }
    let name_cell = line[0];
    let name = match cells.get(name_cell).and_then(|entry| entry.value.clone()) {
        Some(Value::Text(text)) if labels && !text.trim().is_empty() => text.trim().to_string(),
        _ => fallback.to_string(),
    };
    if labels {
        line.remove(0);
    }
    if line.is_empty() {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: "the input range holds no numbers".to_string(),
        });
    }
    Ok(Sample { name, area: Area::new(line[0], line[line.len() - 1]) })
}

/// The **test** report: a t-test of two samples, or an F-test of two
/// variances.
#[allow(clippy::too_many_arguments)]
fn test_of(
    session: &mut Session,
    id: &str,
    kind: &str,
    sheet: u64,
    range1: &str,
    range2: &str,
    alpha: f64,
    difference: f64,
    output: Option<&Text>,
    new_sheet: Option<&Text>,
    labels: bool,
    overwrite: bool,
) -> Result<craftkit_sdk::serde_json::Value, EngineError> {
    let Some(node) = session.document().node(sheet).cloned() else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no sheet `{sheet}`"),
        });
    };
    if node.kind != "grid.sheet" {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the node `{sheet}` is not a sheet"),
        });
    }
    let cells = crate::cells_of(&node);
    let first = sample_of(&crate::fill::area_at(range1, id)?, labels, &cells, "Variable 1", id)?;
    let second = sample_of(&crate::fill::area_at(range2, id)?, labels, &cells, "Variable 2", id)?;
    for sample in [&first, &second] {
        for row in sample.area.start.row..=sample.area.end.row {
            for col in sample.area.start.col..=sample.area.end.col {
                match cells.get(CellRef { col, row }).and_then(|entry| entry.value.clone()) {
                    None | Some(Value::Empty) => continue,
                    Some(Value::Number(_)) => continue,
                    Some(_) => {
                        return Err(EngineError::Failed {
                            id: id.to_string(),
                            message: format!(
                                "the range holds text ({} isn't a number)",
                                CellRef { col, row }.to_text()
                            ),
                        });
                    }
                }
            }
        }
    }
    let target = target_of(session, id, sheet, output, new_sheet, kind)?;
    let origin = match &target {
        Target::Range { at, .. } => *at,
        Target::Sheet { .. } => CellRef { col: 0, row: 0 },
    };
    let prefix = match &target {
        Target::Range { sheet: target_sheet, .. } if *target_sheet == sheet => String::new(),
        _ => qualified(&node.name, id)?,
    };
    let one = format!("{prefix}{}", absolute(&first.area));
    let two = format!("{prefix}{}", absolute(&second.area));
    let mut cells = Cells::new();
    let title = match kind {
        "paired" => "t-Test: Paired Two Sample for Means",
        "equal" => "t-Test: Two-Sample Assuming Equal Variances",
        "unequal" => "t-Test: Two-Sample Assuming Unequal Variances",
        _ => "F-Test Two-Sample for Variances",
    };
    cells.set_value(CellRef { col: 0, row: 0 }, Value::Text(title.to_string()));
    cells.set_value(CellRef { col: 1, row: 2 }, Value::Text(first.name.clone()));
    cells.set_value(CellRef { col: 2, row: 2 }, Value::Text(second.name.clone()));
    // The **three rows** every test has.
    let mut row = 3;
    let line = |cells: &mut Cells, row: u32, label: &str, one: String, two: Option<String>| {
        cells.set_value(CellRef { col: 0, row }, Value::Text(label.to_string()));
        cells.set_formula(CellRef { col: 1, row }, one);
        if let Some(two) = two {
            cells.set_formula(CellRef { col: 2, row }, two);
        }
    };
    line(&mut cells, row, "Mean", format!("=AVERAGE({one})"), Some(format!("=AVERAGE({two})")));
    line(&mut cells, row + 1, "Variance", format!("=VAR.S({one})"), Some(format!("=VAR.S({two})")));
    line(&mut cells, row + 2, "Observations", format!("=COUNT({one})"), Some(format!("=COUNT({two})")));
    row += 3;
    if kind == "f" {
        // The **F-test**: the ratio of the two variances.
        let df_row = row;
        line(&mut cells, df_row, "df", format!("=COUNT({one})-1"), Some(format!("=COUNT({two})-1")));
        let f_row = df_row + 1;
        line(&mut cells, f_row, "F", format!("=VAR.S({one})/VAR.S({two})"), None);
        let f_cell = address(1, f_row, origin);
        let one_df = address(1, df_row, origin);
        let two_df = address(2, df_row, origin);
        line(
            &mut cells,
            f_row + 1,
            "P(F<=f) one-tail",
            format!(
                "=IF({f_cell}<1,F.DIST({f_cell},{one_df},{two_df},TRUE),F.DIST.RT({f_cell},{one_df},{two_df}))"
            ),
            None,
        );
        line(
            &mut cells,
            f_row + 2,
            "F Critical one-tail",
            format!("=IF({f_cell}<1,F.INV({alpha},{one_df},{two_df}),F.INV.RT({alpha},{one_df},{two_df}))"),
            None,
        );
        let rows = f_row + 3;
        let columns = 3;
        let (sheet_id, sheet_name) =
            place_report(session, id, target, &cells, rows, columns, overwrite, sheet)?;
        return Ok(json!({
            "tool": "fTest",
            "sheet": sheet_id,
            "name": sheet_name,
            "range": format!(
                "{}:{}",
                origin.to_text(),
                CellRef { col: origin.col + columns - 1, row: origin.row + rows - 1 }.to_text()
            ),
            "rows": rows,
            "cols": columns,
        }));
    }
    // The **t-tests**.
    let mut pooled_row = None;
    match kind {
        "paired" => {
            line(&mut cells, row, "Pearson Correlation", format!("=CORREL({one},{two})"), None);
        }
        "equal" => {
            pooled_row = Some(row);
            line(
                &mut cells,
                row,
                "Pooled Variance",
                format!(
                    "=((COUNT({one})-1)*VAR.S({one})+(COUNT({two})-1)*VAR.S({two}))/(COUNT({one})+COUNT({two})-2)"
                ),
                None,
            );
        }
        _ => {}
    }
    if kind == "paired" || kind == "equal" {
        row += 1;
    }
    cells.set_value(CellRef { col: 0, row }, Value::Text("Hypothesized Mean Difference".to_string()));
    cells.set_value(CellRef { col: 1, row }, Value::Number(difference));
    // The **degrees of freedom**.
    let df_row = row + 1;
    let df = match kind {
        "paired" => format!("=COUNT({one})-1"),
        "equal" => format!("=COUNT({one})+COUNT({two})-2"),
        _ => format!(
            "=ROUND(((VAR.S({one})/COUNT({one})+VAR.S({two})/COUNT({two}))^2)/((VAR.S({one})/COUNT({one}))^2/(COUNT({one})-1)+(VAR.S({two})/COUNT({two}))^2/(COUNT({two})-1)),0)"
        ),
    };
    line(&mut cells, df_row, "df", df, None);
    // The **t statistic**.
    let t_row = df_row + 1;
    let t = match kind {
        "paired" => format!(
            "=(AVERAGE({one})-AVERAGE({two})-{difference})/SQRT((VAR.S({one})+VAR.S({two})-2*CORREL({one},{two})*STDEV.S({one})*STDEV.S({two}))/COUNT({one}))"
        ),
        "equal" => {
            let pooled = address(1, pooled_row.unwrap_or(df_row), origin);
            format!(
                "=(AVERAGE({one})-AVERAGE({two})-{difference})/SQRT({pooled}*(1/COUNT({one})+1/COUNT({two})))"
            )
        }
        _ => format!(
            "=(AVERAGE({one})-AVERAGE({two})-{difference})/SQRT(VAR.S({one})/COUNT({one})+VAR.S({two})/COUNT({two}))"
        ),
    };
    line(&mut cells, t_row, "t Stat", t, None);
    let t_cell = address(1, t_row, origin);
    let df_cell = address(1, df_row, origin);
    line(&mut cells, t_row + 1, "P(T<=t) one-tail", format!("=T.DIST.RT(ABS({t_cell}),{df_cell})"), None);
    line(&mut cells, t_row + 2, "t Critical one-tail", format!("=T.INV(1-{alpha},{df_cell})"), None);
    line(&mut cells, t_row + 3, "P(T<=t) two-tail", format!("=T.DIST.2T(ABS({t_cell}),{df_cell})"), None);
    line(&mut cells, t_row + 4, "t Critical two-tail", format!("=T.INV.2T({alpha},{df_cell})"), None);
    let rows = t_row + 5;
    let columns = 3;
    let (sheet_id, sheet_name) = place_report(session, id, target, &cells, rows, columns, overwrite, sheet)?;
    Ok(json!({
        "tool": format!("tTest{kind}"),
        "sheet": sheet_id,
        "name": sheet_name,
        "range": format!(
            "{}:{}",
            origin.to_text(),
            CellRef { col: origin.col + columns - 1, row: origin.row + rows - 1 }.to_text()
        ),
        "rows": rows,
        "cols": columns,
    }))
}

/// The **report** of a set of variables: the cells of it, from its
/// own corner.
fn report_cells(variables: &[(String, Vec<(String, String)>)]) -> Cells {
    let mut cells = Cells::new();
    for (index, (name, rows)) in variables.iter().enumerate() {
        let column = index as u32 * 2;
        cells.set_value(CellRef { col: column, row: 0 }, Value::Text(name.clone()));
        for (row, (label, formula)) in rows.iter().enumerate() {
            let row = row as u32 + 2;
            cells.set_value(CellRef { col: column, row }, Value::Text(label.clone()));
            cells.set_formula(CellRef { col: column + 1, row }, formula.clone());
        }
    }
    cells
}

/// How many **rows** and **columns** a report has.
fn extent(variables: &[(String, Vec<(String, String)>)]) -> (u32, u32) {
    let rows = variables.iter().map(|(_, rows)| rows.len() as u32).max().unwrap_or(0) + 2;
    let columns = variables.len() as u32 * 2;
    (rows.max(1), columns.max(1))
}

/// Writes a report into the cells of a sheet, from a corner.
///
/// The cells that hold something are refused, unless the report may
/// be written over them.
#[allow(clippy::too_many_arguments)]
fn write_into(
    session: &mut Session,
    id: &str,
    sheet: u64,
    at: CellRef,
    report: &Cells,
    rows: u32,
    columns: u32,
    overwrite: bool,
) -> Result<(), EngineError> {
    let Some(node) = session.document().node(sheet) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no sheet `{sheet}`"),
        });
    };
    let cells = crate::cells_of(node);
    // The **occupied** cells of the report's rectangle.
    let mut occupied = Vec::new();
    for row in at.row..at.row.saturating_add(rows) {
        for col in at.col..at.col.saturating_add(columns) {
            let cell = CellRef { col, row };
            let entry = cells.get(cell);
            let holds = entry.map(|entry| entry.formula.is_some() || entry.value.is_some()).unwrap_or(false);
            if holds {
                occupied.push(cell);
            }
        }
    }
    if !occupied.is_empty() && !overwrite {
        let first = occupied[0].to_text();
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!(
                "the output range holds something (at `{first}`); pass `overwrite` to replace it"
            ),
        });
    }
    if let Some(node) = session.document_mut()?.node_mut(sheet) {
        let mut sheet_cells = crate::sheet_cells_mut(node);
        for (cell, entry) in report.iter() {
            let cell = CellRef { col: at.col + cell.col, row: at.row + cell.row };
            match (&entry.formula, &entry.value) {
                (Some(formula), _) => sheet_cells.set_formula(cell, formula.clone()),
                (None, Some(value)) => sheet_cells.set_value(cell, value.clone()),
                (None, None) => {}
            }
        }
        node.data["cells"] = crate::cells_into(&sheet_cells);
    }
    Ok(())
}

commands! {
    /// **Descriptive statistics** of a range, as a report.
    ///
    /// The report holds, for every variable of the range, the **mean**,
    /// the **standard error**, the **median**, the **mode**, the
    /// **standard deviation**, the **sample variance**, the
    /// **kurtosis**, the **skewness**, the **range**, the **minimum**,
    /// the **maximum**, the **sum** and the **count** — and, when they
    /// are asked for, the **largest** and the **smallest** values and
    /// the **confidence level** of the mean.
    ///
    /// The report is **live**: its values are formulas over the range,
    /// so an edit of the data is an edit of the report.
    ///
    /// The report goes to the **range** of the sheet — the top-left
    /// cell of it — or, without one, to a **new sheet** of its own.
    grid.analysis.descriptive {
        area: "grid",
        layer: "grid",
        icon: "list",
        mutates,
        undoable,
        params: {
            /// The sheet the data is on.
            sheet: NodeRef,
            /// The **range** of the data.
            range: Text,
            /// The **top-left cell** of the report, on the sheet. The
            /// report goes to a new sheet without it.
            output: Option<Text>,
            /// The **name** of the new sheet the report goes to.
            new_sheet: Option<Text>,
            /// Whether the first row (or the first column) holds the
            /// **names** of the variables.
            labels: Option<Bool>,
            /// Whether the variables are the **columns** (the default)
            /// or the **rows** of the range.
            grouped_by: Option<Text>,
            /// Whether the **summary** rows are written. Without them,
            /// one of `confidence`, `kth_largest` or `kth_smallest` is
            /// needed.
            summary: Option<Bool>,
            /// The **confidence level** of the mean: `0.95` or `95`.
            confidence: Option<Number>,
            /// The **k** of the largest value: the row `Largest(k)`.
            kth_largest: Option<Number>,
            /// The **k** of the smallest value: the row `Smallest(k)`.
            kth_smallest: Option<Number>,
            /// Whether the report may be written over cells that hold
            /// something.
            overwrite: Option<Bool>,
        },
        run: {
            let id = "grid.analysis.descriptive".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet).cloned() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            if node.kind != "grid.sheet" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{sheet}` is not a sheet"),
                });
            }
            // The **range** of the data.
            let area = crate::fill::area_at(&self.range.0, &id)?;
            if area.count() > MAX_INPUT_CELLS {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the range is too large (at most {MAX_INPUT_CELLS} cells)"),
                });
            }
            // The **variables**: the columns of the range, or its rows.
            let by_rows = match self.grouped_by.as_ref().map(|group| group.0.trim().to_ascii_lowercase()) {
                None => false,
                Some(ref text) if text.is_empty() || text == "columns" || text == "column" => false,
                Some(ref text) if text == "rows" || text == "row" => true,
                Some(ref text) => {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the variables are grouped by `columns` or by `rows`, not `{text}`"),
                    });
                }
            };
            let labels = self.labels.map(|labels| labels.0).unwrap_or(false);
            let cells = crate::cells_of(&node);
            let variables = variables(&area, by_rows, labels);
            for variable in &variables {
                numbers_of(variable, &cells, &id)?;
                if variable.area.start.row > variable.area.end.row
                    || variable.area.start.col > variable.area.end.col
                {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the variable at `{}` holds no numbers", variable.name_cell.to_text()),
                    });
                }
            }
            // The **options** of the report.
            let summary = self.summary.map(|summary| summary.0).unwrap_or(true);
            let confidence = match self.confidence {
                None => None,
                Some(number) => {
                    let value = number.0;
                    let value = if value > 0.0 && value < 1.0 { value * 100.0 } else { value };
                    if !(value > 0.0 && value < 100.0) {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: "the confidence level is between 0 and 100".to_string(),
                        });
                    }
                    Some(value)
                }
            };
            let kth_largest = match self.kth_largest {
                None => None,
                Some(number) => {
                    let value = number.0;
                    if !value.is_finite() || value < 1.0 {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: "the k of the largest value is from 1".to_string(),
                        });
                    }
                    Some(value.trunc() as u32)
                }
            };
            let kth_smallest = match self.kth_smallest {
                None => None,
                Some(number) => {
                    let value = number.0;
                    if !value.is_finite() || value < 1.0 {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: "the k of the smallest value is from 1".to_string(),
                        });
                    }
                    Some(value.trunc() as u32)
                }
            };
            if !summary && confidence.is_none() && kth_largest.is_none() && kth_smallest.is_none() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "choose at least one of `summary`, `confidence`, `kth_largest` or `kth_smallest`"
                        .to_string(),
                });
            }
            // Where the report goes.
            let target = target_of(
                session,
                &id,
                sheet,
                self.output.as_ref(),
                self.new_sheet.as_ref(),
                "Descriptive",
            )?;
            // The **formulas** of the report: over the range of the
            // data, named by the sheet it is on when the report is
            // written somewhere else.
            let prefix = match &target {
                Target::Range { sheet: target_sheet, .. } if *target_sheet == sheet => String::new(),
                _ => qualified(&node.name, &id)?,
            };
            let mut report: Vec<(String, Vec<(String, String)>)> = Vec::new();
            for (index, variable) in variables.iter().enumerate() {
                let range = format!("{prefix}{}", absolute(&variable.area));
                let mut rows = Vec::new();
                if summary {
                    rows.extend(summary_rows(&range));
                }
                if let Some(k) = kth_largest {
                    rows.push((format!("Largest({k})"), format!("=LARGE({range},{k})")));
                }
                if let Some(k) = kth_smallest {
                    rows.push((format!("Smallest({k})"), format!("=SMALL({range},{k})")));
                }
                if let Some(level) = confidence {
                    rows.push((
                        format!("Confidence Level({level:.1}%)"),
                        format!(
                            "=T.INV.2T(1-{level}/100,COUNT({range})-1)*STDEV.S({range})/SQRT(COUNT({range}))"
                        ),
                    ));
                }
                report.push((name_of(variable, &cells, index), rows));
            }
            let (rows, columns) = extent(&report);
            if u64::from(rows) * u64::from(columns) > MAX_OUTPUT_CELLS {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the report is too large".to_string(),
                });
            }
            let cells = report_cells(&report);
            let overwrite = self.overwrite.map(|overwrite| overwrite.0).unwrap_or(false);
            // The **write**.
            let (sheet_id, sheet_name, at) = match target {
                Target::Range { sheet: target_sheet, at } => {
                    write_into(session, &id, target_sheet, at, &cells, rows, columns, overwrite)?;
                    let name = session
                        .document()
                        .node(target_sheet)
                        .map(|node| node.name.clone())
                        .unwrap_or_default();
                    (target_sheet, name, at)
                }
                Target::Sheet { name } => {
                    let Some(workbook) = workbook_of(session, sheet) else {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("the sheet `{sheet}` is not in a workbook"),
                        });
                    };
                    let new_sheet = session.document_mut()?.add_node_with_under(
                        Some(workbook),
                        name.clone(),
                        "grid.sheet",
                        None,
                        json!({
                            "cells": {},
                            "view": crate::view::SheetView::default().to_json(),
                            "zoom": 100,
                        }),
                    );
                    if let Some(node) = session.document_mut()?.node_mut(new_sheet) {
                        node.data["cells"] = crate::cells_into(&cells);
                    }
                    (new_sheet, name, CellRef { col: 0, row: 0 })
                }
            };
            Ok(Outcome::changed(format!(
                "descriptive statistics of {} variable(s) into the sheet `{sheet_name}`",
                report.len()
            ))
            .with_data(json!({
                "tool": "descriptive",
                "sheet": sheet_id,
                "name": sheet_name,
                "range": format!("{}:{}", at.to_text(), CellRef { col: at.col + columns - 1, row: at.row + rows - 1 }.to_text()),
                "rows": rows,
                "cols": columns,
                "variables": report.len(),
            })))

        }
    }

    /// The **histogram** of a range: how many numbers fall in each bin.
    ///
    /// The report holds the **bins** and their **frequencies**, and,
    /// when they are asked for, the **cumulative** percentage and the
    /// **pareto** — the same bins, in the order of their frequencies.
    ///
    /// The report is **live**: the bins are formulas over the data —
    /// evenly spaced from the smallest number to the largest — and the
    /// frequencies are formulas over the bins.
    grid.analysis.histogram {
        area: "grid",
        layer: "grid",
        icon: "list",
        mutates,
        undoable,
        params: {
            /// The sheet the data is on.
            sheet: NodeRef,
            /// The **range** of the data: one row or one column.
            range: Text,
            /// The **top-left cell** of the report, on the sheet. The
            /// report goes to a new sheet without it.
            output: Option<Text>,
            /// The **name** of the new sheet the report goes to.
            new_sheet: Option<Text>,
            /// Whether the first cell of the range holds a **header**.
            labels: Option<Bool>,
            /// The **range** of the bin edges. Without it, the bins are
            /// evenly spaced from the smallest number to the largest.
            bins: Option<Text>,
            /// Whether the **pareto** is written beside the histogram.
            pareto: Option<Bool>,
            /// Whether the **cumulative** percentage is written.
            cumulative: Option<Bool>,
            /// Whether the report may be written over cells that hold
            /// something.
            overwrite: Option<Bool>,
        },
        run: {
            let id = "grid.analysis.histogram".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet).cloned() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            if node.kind != "grid.sheet" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{sheet}` is not a sheet"),
                });
            }
            let labels = self.labels.map(|labels| labels.0).unwrap_or(false);
            let cumulative = self.cumulative.map(|cumulative| cumulative.0).unwrap_or(false);
            let pareto = self.pareto.map(|pareto| pareto.0).unwrap_or(false);
            // The **data**: the cells of the range, without its header.
            let area = crate::fill::area_at(&self.range.0, &id)?;
            let mut data = line_cells(&area, &id)?;
            if labels && !data.is_empty() {
                data.remove(0);
            }
            if data.is_empty() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the range holds no numbers".to_string(),
                });
            }
            let cells = crate::cells_of(&node);
            for cell in &data {
                match cells.get(*cell).and_then(|entry| entry.value.clone()) {
                    None | Some(Value::Empty) => continue,
                    Some(Value::Number(_)) => continue,
                    Some(_) => {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("the range holds text ({} isn't a number)", cell.to_text()),
                        });
                    }
                }
            }
            // The **bins**: the edges of the intervals, or the range
            // that holds them.
            let mut bin_cells = Vec::new();
            let mut bin_range = None;
            if let Some(bins) = self.bins.as_ref() {
                let bin_area = crate::fill::area_at(&bins.0, &id)?;
                let mut cells_in = line_cells(&bin_area, &id)?;
                if labels && !cells_in.is_empty() {
                    cells_in.remove(0);
                }
                if cells_in.is_empty() {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "the bin range holds no numbers".to_string(),
                    });
                }
                for cell in &cells_in {
                    match cells.get(*cell).and_then(|entry| entry.value.clone()) {
                        None | Some(Value::Empty) => continue,
                        Some(Value::Number(_)) => continue,
                        Some(_) => {
                            return Err(EngineError::Failed {
                                id: id.clone(),
                                message: format!("the bin range holds text ({} isn't a number)", cell.to_text()),
                            });
                        }
                    }
                }
                bin_range = Some(Area::new(cells_in[0], cells_in[cells_in.len() - 1]));
                bin_cells = cells_in;
            }
            // Where the report goes.
            let target = target_of(
                session,
                &id,
                sheet,
                self.output.as_ref(),
                self.new_sheet.as_ref(),
                "Histogram",
            )?;
            let origin = match &target {
                Target::Range { at, .. } => *at,
                Target::Sheet { .. } => CellRef { col: 0, row: 0 },
            };
            // The **prefix**: the name of the sheet the data is on,
            // when the report is written somewhere else.
            let prefix = match &target {
                Target::Range { sheet: target_sheet, .. } if *target_sheet == sheet => String::new(),
                _ => qualified(&node.name, &id)?,
            };
            let data_range = format!("{prefix}{}", absolute(&Area::new(data[0], data[data.len() - 1])));
            // The **edges**: the bins of the histogram.
            let mut edges: Vec<String> = Vec::new();
            if let Some(bin_area) = bin_range {
                let bins_range = format!("{prefix}{}", absolute(&bin_area));
                for index in 0..bin_cells.len() {
                    edges.push(format!("=SMALL({bins_range},{})", index + 1));
                }
            } else {
                let count = data.len() as f64;
                let intervals = (count.sqrt().floor() as usize).clamp(1, 1000);
                for index in 0..=intervals {
                    edges.push(format!(
                        "=MIN({data_range})+(MAX({data_range})-MIN({data_range}))*{index}/{intervals}"
                    ));
                }
            }
            let rows = edges.len() as u32 + 2;
            let columns = 2 + u32::from(cumulative) + if pareto { 2 + u32::from(cumulative) } else { 0 };
            // The **main block**: the bins and their frequencies.
            let mut report = Cells::new();
            report.set_value(CellRef { col: 0, row: 0 }, Value::Text("Bin".to_string()));
            report.set_value(CellRef { col: 1, row: 0 }, Value::Text("Frequency".to_string()));
            if cumulative {
                report.set_value(CellRef { col: 2, row: 0 }, Value::Text("Cumulative %".to_string()));
            }
            for (index, edge) in edges.iter().enumerate() {
                let row = index as u32 + 1;
                report.set_formula(CellRef { col: 0, row }, edge.clone());
                let count = if index == 0 {
                    format!("=COUNTIFS({data_range},\"<=\"&{})", address(0, row, origin))
                } else {
                    format!(
                        "=COUNTIFS({data_range},\">\"&{},{data_range},\"<=\"&{})",
                        address(0, row - 1, origin),
                        address(0, row, origin)
                    )
                };
                report.set_formula(CellRef { col: 1, row }, count);
                if cumulative {
                    let total = format!("=SUM({}:{})/COUNT({data_range})", address(1, 1, origin), address(1, row, origin));
                    report.set_formula(CellRef { col: 2, row }, total);
                }
            }
            // The **last row**: the numbers that are past the last bin.
            let more = edges.len() as u32 + 1;
            report.set_value(CellRef { col: 0, row: more }, Value::Text("More".to_string()));
            report.set_formula(
                CellRef { col: 1, row: more },
                format!("=COUNTIFS({data_range},\">\"&{})", address(0, more - 1, origin)),
            );
            if cumulative {
                let total = format!("=SUM({}:{})/COUNT({data_range})", address(1, 1, origin), address(1, more, origin));
                report.set_formula(CellRef { col: 2, row: more }, total);
            }
            // The **pareto**: the same bins, in the order of their
            // frequencies. The order is the order of the frequencies
            // when the report is made; the values are live.
            if pareto {
                let offset = 2 + u32::from(cumulative);
                report.set_value(CellRef { col: offset, row: 0 }, Value::Text("Bin".to_string()));
                report.set_value(CellRef { col: offset + 1, row: 0 }, Value::Text("Frequency".to_string()));
                if cumulative {
                    report.set_value(
                        CellRef { col: offset + 2, row: 0 },
                        Value::Text("Cumulative %".to_string()),
                    );
                }
                // The **frequencies**, as the tree's own formulas compute
                // them.
                let mut book = crate::book_of(session.document());
                let report_sheet = match &target {
                    Target::Range { .. } => node.name.clone(),
                    Target::Sheet { name } => name.clone(),
                };
                match &target {
                    Target::Range { .. } => {
                        let mut merged = crate::cells_of(&node);
                        for (cell, entry) in report.iter() {
                            let cell = CellRef { col: origin.col + cell.col, row: origin.row + cell.row };
                            match (&entry.formula, &entry.value) {
                                (Some(formula), _) => merged.set_formula(cell, formula.clone()),
                                (None, Some(value)) => merged.set_value(cell, value.clone()),
                                (None, None) => {}
                            }
                        }
                        book.add(&report_sheet, merged);
                    }
                    Target::Sheet { name } => {
                        book.add(name, report.clone());
                    }
                }
                let mut frequencies: Vec<(f64, u32)> = Vec::new();
                for row in 1..=more {
                    let cell = CellRef { col: origin.col + 1, row: origin.row + row };
                    let value = book.value_of(&report_sheet, cell);
                    let frequency = match value {
                        Value::Number(number) => number,
                        _ => f64::NAN,
                    };
                    frequencies.push((frequency, row));
                }
                frequencies.sort_by(|left, right| {
                    right.0.total_cmp(&left.0).then(left.1.cmp(&right.1))
                });
                for (rank, (_frequency, row)) in frequencies.iter().enumerate() {
                    let target_row = rank as u32 + 1;
                    report.set_formula(CellRef { col: offset, row: target_row }, format!("={}", address(0, *row, origin)));
                    report.set_formula(CellRef { col: offset + 1, row: target_row }, format!("={}", address(1, *row, origin)));
                    if cumulative {
                        let total = format!(
                            "=SUM({}:{})/COUNT({data_range})",
                            address(offset + 1, 1, origin),
                            address(offset + 1, target_row, origin)
                        );
                        report.set_formula(CellRef { col: offset + 2, row: target_row }, total);
                    }
                }
            }
            // The **write**.
            let overwrite = self.overwrite.map(|overwrite| overwrite.0).unwrap_or(false);
            let (sheet_id, sheet_name) = match target {
                Target::Range { sheet: target_sheet, at } => {
                    write_into(session, &id, target_sheet, at, &report, rows, columns, overwrite)?;
                    let name = session
                        .document()
                        .node(target_sheet)
                        .map(|node| node.name.clone())
                        .unwrap_or_default();
                    (target_sheet, name)
                }
                Target::Sheet { name } => {
                    let Some(workbook) = workbook_of(session, sheet) else {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("the sheet `{sheet}` is not in a workbook"),
                        });
                    };
                    let new_sheet = session.document_mut()?.add_node_with_under(
                        Some(workbook),
                        name.clone(),
                        "grid.sheet",
                        None,
                        json!({
                            "cells": {},
                            "view": crate::view::SheetView::default().to_json(),
                            "zoom": 100,
                        }),
                    );
                    if let Some(node) = session.document_mut()?.node_mut(new_sheet) {
                        node.data["cells"] = crate::cells_into(&report);
                    }
                    (new_sheet, name)
                }
            };
            Ok(Outcome::changed(format!(
                "a histogram of {} number(s) into the sheet `{sheet_name}`",
                data.len()
            ))
            .with_data(json!({
                "tool": "histogram",
                "sheet": sheet_id,
                "name": sheet_name,
                "range": format!(
                    "{}:{}",
                    origin.to_text(),
                    CellRef { col: origin.col + columns - 1, row: origin.row + rows - 1 }.to_text()
                ),
                "rows": rows,
                "cols": columns,
                "bins": edges.len(),
            })))

        }
    }

    /// The **correlation** of the variables of a range, as a matrix.
    ///
    /// The report is a square: the names of the variables are in the
    /// first row and in the first column, and the correlation of each
    /// pair of them is in the square they make.
    ///
    /// The report is **live**: the values are formulas over the range,
    /// so an edit of the data is an edit of the report.
    grid.analysis.correlation {
        area: "grid",
        layer: "grid",
        icon: "list",
        mutates,
        undoable,
        params: {
            /// The sheet the data is on.
            sheet: NodeRef,
            /// The **range** of the data: a column is a variable.
            range: Text,
            /// The **top-left cell** of the report, on the sheet. The
            /// report goes to a new sheet without it.
            output: Option<Text>,
            /// The **name** of the new sheet the report goes to.
            new_sheet: Option<Text>,
            /// Whether the first row (or the first column) holds the
            /// **names** of the variables.
            labels: Option<Bool>,
            /// Whether the variables are the **columns** (the default)
            /// or the **rows** of the range.
            grouped_by: Option<Text>,
            /// Whether the report may be written over cells that hold
            /// something.
            overwrite: Option<Bool>,
        },
        run: {
            let id = "grid.analysis.correlation".to_string();
            let sheet = self.sheet.0;
            let labels = self.labels.map(|labels| labels.0).unwrap_or(false);
            let by_rows = match self.grouped_by.as_ref().map(|group| group.0.trim().to_ascii_lowercase()) {
                None => false,
                Some(ref text) if text.is_empty() || text == "columns" || text == "column" => false,
                Some(ref text) if text == "rows" || text == "row" => true,
                Some(ref text) => {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the variables are grouped by `columns` or by `rows`, not `{text}`"),
                    });
                }
            };
            let data = matrix_of(
                session,
                &id,
                "correlation",
                sheet,
                &self.range.0,
                self.output.as_ref(),
                self.new_sheet.as_ref(),
                labels,
                by_rows,
                self.overwrite.map(|overwrite| overwrite.0).unwrap_or(false),
            )?;
            Ok(Outcome::changed("the correlation of the variables".to_string()).with_data(data))

        }
    }

    /// The **covariance** of the variables of a range, as a matrix.
    ///
    /// The report is a square: the names of the variables are in the
    /// first row and in the first column, and the covariance of each
    /// pair of them is in the square they make.
    ///
    /// The report is **live**: the values are formulas over the range,
    /// so an edit of the data is an edit of the report.
    grid.analysis.covariance {
        area: "grid",
        layer: "grid",
        icon: "list",
        mutates,
        undoable,
        params: {
            /// The sheet the data is on.
            sheet: NodeRef,
            /// The **range** of the data: a column is a variable.
            range: Text,
            /// The **top-left cell** of the report, on the sheet. The
            /// report goes to a new sheet without it.
            output: Option<Text>,
            /// The **name** of the new sheet the report goes to.
            new_sheet: Option<Text>,
            /// Whether the first row (or the first column) holds the
            /// **names** of the variables.
            labels: Option<Bool>,
            /// Whether the variables are the **columns** (the default)
            /// or the **rows** of the range.
            grouped_by: Option<Text>,
            /// Whether the report may be written over cells that hold
            /// something.
            overwrite: Option<Bool>,
        },
        run: {
            let id = "grid.analysis.covariance".to_string();
            let sheet = self.sheet.0;
            let labels = self.labels.map(|labels| labels.0).unwrap_or(false);
            let by_rows = match self.grouped_by.as_ref().map(|group| group.0.trim().to_ascii_lowercase()) {
                None => false,
                Some(ref text) if text.is_empty() || text == "columns" || text == "column" => false,
                Some(ref text) if text == "rows" || text == "row" => true,
                Some(ref text) => {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the variables are grouped by `columns` or by `rows`, not `{text}`"),
                    });
                }
            };
            let data = matrix_of(
                session,
                &id,
                "covariance",
                sheet,
                &self.range.0,
                self.output.as_ref(),
                self.new_sheet.as_ref(),
                labels,
                by_rows,
                self.overwrite.map(|overwrite| overwrite.0).unwrap_or(false),
            )?;
            Ok(Outcome::changed("the covariance of the variables".to_string()).with_data(data))

        }
    }

    /// The **t-test** of two samples: a test of two means.
    ///
    /// The kind of the test is `paired`, `equal` (two samples assuming
    /// equal variances) or `unequal` (two samples assuming unequal
    /// variances).
    ///
    /// The report is **live**: the values are formulas over the two
    /// ranges, so an edit of the data is an edit of the report.
    grid.analysis.ttest {
        area: "grid",
        layer: "grid",
        icon: "list",
        mutates,
        undoable,
        params: {
            /// The sheet the data is on.
            sheet: NodeRef,
            /// The kind of the test: `paired`, `equal` or `unequal`.
            kind: Text,
            /// The **range** of the first sample: one row or one column.
            range1: Text,
            /// The **range** of the second sample: one row or one column.
            range2: Text,
            /// The **alpha** of the test: `0.05` by default.
            alpha: Option<Number>,
            /// The **hypothesized** difference of the means: `0` by
            /// default.
            difference: Option<Number>,
            /// The **top-left cell** of the report, on the sheet. The
            /// report goes to a new sheet without it.
            output: Option<Text>,
            /// The **name** of the new sheet the report goes to.
            new_sheet: Option<Text>,
            /// Whether the first cell of each range holds the **name**
            /// of the variable.
            labels: Option<Bool>,
            /// Whether the report may be written over cells that hold
            /// something.
            overwrite: Option<Bool>,
        },
        run: {
            let id = "grid.analysis.ttest".to_string();
            let sheet = self.sheet.0;
            let kind = match self.kind.0.trim().to_ascii_lowercase().as_str() {
                "paired" => "paired",
                "equal" => "equal",
                "unequal" => "unequal",
                ref text => {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the test is `paired`, `equal` or `unequal`, not `{text}`"),
                    });
                }
            };
            let alpha = match self.alpha {
                None => 0.05,
                Some(number) => {
                    let value = number.0;
                    if !(value > 0.0 && value < 1.0) {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: "the alpha is between 0 and 1".to_string(),
                        });
                    }
                    value
                }
            };
            let difference = self.difference.map(|number| number.0).unwrap_or(0.0);
            let data = test_of(
                session,
                &id,
                kind,
                sheet,
                &self.range1.0,
                &self.range2.0,
                alpha,
                difference,
                self.output.as_ref(),
                self.new_sheet.as_ref(),
                self.labels.map(|labels| labels.0).unwrap_or(false),
                self.overwrite.map(|overwrite| overwrite.0).unwrap_or(false),
            )?;
            Ok(Outcome::changed(format!("a t-test of two samples ({kind})")).with_data(data))

        }
    }

    /// The **F-test** of two samples: a test of two variances.
    ///
    /// The report is **live**: the values are formulas over the two
    /// ranges, so an edit of the data is an edit of the report.
    grid.analysis.ftest {
        area: "grid",
        layer: "grid",
        icon: "list",
        mutates,
        undoable,
        params: {
            /// The sheet the data is on.
            sheet: NodeRef,
            /// The **range** of the first sample: one row or one column.
            range1: Text,
            /// The **range** of the second sample: one row or one column.
            range2: Text,
            /// The **alpha** of the test: `0.05` by default.
            alpha: Option<Number>,
            /// The **top-left cell** of the report, on the sheet. The
            /// report goes to a new sheet without it.
            output: Option<Text>,
            /// The **name** of the new sheet the report goes to.
            new_sheet: Option<Text>,
            /// Whether the first cell of each range holds the **name**
            /// of the variable.
            labels: Option<Bool>,
            /// Whether the report may be written over cells that hold
            /// something.
            overwrite: Option<Bool>,
        },
        run: {
            let id = "grid.analysis.ftest".to_string();
            let sheet = self.sheet.0;
            let alpha = match self.alpha {
                None => 0.05,
                Some(number) => {
                    let value = number.0;
                    if !(value > 0.0 && value < 1.0) {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: "the alpha is between 0 and 1".to_string(),
                        });
                    }
                    value
                }
            };
            let data = test_of(
                session,
                &id,
                "f",
                sheet,
                &self.range1.0,
                &self.range2.0,
                alpha,
                0.0,
                self.output.as_ref(),
                self.new_sheet.as_ref(),
                self.labels.map(|labels| labels.0).unwrap_or(false),
                self.overwrite.map(|overwrite| overwrite.0).unwrap_or(false),
            )?;
            Ok(Outcome::changed("an F-test of two samples".to_string()).with_data(data))

        }
    }
}
