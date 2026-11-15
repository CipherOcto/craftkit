//! The **review** of a workbook: what is in it, and what a reader may
//! not be able to read.
//!
//! Two readings: the **statistics** of a workbook — how many sheets,
//! how many cells, how many formulas, how many tables, how many
//! charts — and its **accessibility**: the issues that a reader with a
//! screen reader, or a reader who cannot see a picture, would meet.
//!
//! The rules:
//!
//! - the statistics are **counted**, not estimated: a cell with a
//!   formula and no value is a cell with a formula, and a cell with
//!   nothing is not a cell at all;
//! - the accessibility is a **list of issues**, and a workbook with no
//!   issues has an empty list and not a refusal.

use craftkit_derive::commands;

use craftkit_sdk::Outcome;
use craftkit_sdk::serde_json::{Value as Json, json};

/// The **sheets** of the document, with what is in them.
fn sheets_of(session: &craftkit_sdk::Session) -> Vec<(&craftkit_sdk::Node, Json)> {
    let mut sheets = Vec::new();
    for node in session.document().walk() {
        if node.kind != "grid.sheet" {
            continue;
        }
        let cells = crate::cells_of(node);
        let mut with_data = 0;
        let mut formulas = 0;
        for (_, entry) in cells.iter() {
            if entry.formula.is_some() {
                formulas += 1;
            }
            if entry.value.is_some() || entry.formula.is_some() {
                with_data += 1;
            }
        }
        let tables = crate::tables::tables_of(node).len();
        let charts = node.children.iter().filter(|child| child.kind == "grid.chart").count();
        let comments = crate::comments::comments_of(node).len();
        let notes = crate::notes::notes_of(node).len();
        let merges = crate::merges::merges_of(node).len();
        let pictures = crate::pictures::pictures_of(node).len();
        sheets.push((
            node,
            json!({
                "name": node.name,
                "cells": with_data,
                "formulas": formulas,
                "tables": tables,
                "charts": charts,
                "comments": comments,
                "notes": notes,
                "merges": merges,
                "pictures": pictures,
            }),
        ));
    }
    sheets
}

commands! {
    /// The **statistics** of a workbook: how much is in it.
    ///
    /// The answer holds the count of every **sheet** — the cells with
    /// something in them, the formulas, the tables, the charts, the
    /// comments, the notes, the merges and the pictures — and the count
    /// of the whole **document**.
    grid.workbook.stats {
        area: "grid",
        layer: "grid",
        icon: "list",
        params: {
        },
        run: {
            let sheets = sheets_of(session);
            let mut cells = 0;
            let mut formulas = 0;
            let mut tables = 0;
            let mut charts = 0;
            let mut comments = 0;
            let mut notes = 0;
            let mut merges = 0;
            let mut pictures = 0;
            let mut list = Vec::new();
            for (_, one) in &sheets {
                cells += one["cells"].as_u64().unwrap_or(0);
                formulas += one["formulas"].as_u64().unwrap_or(0);
                tables += one["tables"].as_u64().unwrap_or(0);
                charts += one["charts"].as_u64().unwrap_or(0);
                comments += one["comments"].as_u64().unwrap_or(0);
                notes += one["notes"].as_u64().unwrap_or(0);
                merges += one["merges"].as_u64().unwrap_or(0);
                pictures += one["pictures"].as_u64().unwrap_or(0);
                list.push(one.clone());
            }
            // The **names** of the workbooks.
            let names: usize = session
                .document()
                .walk()
                .filter(|node| node.kind == "grid.workbook")
                .map(|node| {
                    node.data
                        .get("names")
                        .and_then(|names| names.as_object())
                        .map(|names| names.len())
                        .unwrap_or(0)
                })
                .sum();
            let workbooks = session
                .document()
                .walk()
                .filter(|node| node.kind == "grid.workbook")
                .count();
            let count = list.len();
            Ok(Outcome::new(format!("the statistics of {count} sheet(s)")).with_data(json!({
                "sheets": list,
                "workbook": {
                    "workbooks": workbooks,
                    "sheets": count,
                    "cells": cells,
                    "formulas": formulas,
                    "tables": tables,
                    "charts": charts,
                    "comments": comments,
                    "notes": notes,
                    "merges": merges,
                    "pictures": pictures,
                    "names": names,
                },
            })))

        }
    }

    /// The **accessibility** of a workbook: the issues a reader may
    /// meet with it.
    ///
    /// The answer is a **list of issues**: a picture with no
    /// **alternative text**, a sheet with **merged cells**, and a sheet
    /// whose name is still the **default** one.
    grid.workbook.accessibility {
        area: "grid",
        layer: "grid",
        icon: "check",
        params: {
        },
        run: {
            let mut issues = Vec::new();
            for (node, _) in sheets_of(session) {
                // The **pictures** with no alternative text.
                for (id_of, picture) in crate::pictures::pictures_of(node) {
                    let alt = picture.get("alt").and_then(|alt| alt.as_str()).unwrap_or("");
                    if alt.trim().is_empty() {
                        issues.push(json!({
                            "sheet": node.name,
                            "issue": "missing alternative text",
                            "object": id_of,
                        }));
                    }
                }
                // The **merged cells**: a reader cannot read a table
                // that is not a table.
                let merges = crate::merges::merges_of(node).len();
                if merges > 0 {
                    issues.push(json!({
                        "sheet": node.name,
                        "issue": "merged cells",
                        "count": merges,
                    }));
                }
                // The **default name**: a name that says nothing.
                if node.name.starts_with("Sheet") {
                    issues.push(json!({
                        "sheet": node.name,
                        "issue": "default sheet name",
                    }));
                }
            }
            let count = issues.len();
            Ok(Outcome::new(format!("{count} issue(s)")).with_data(json!({
                "issues": issues,
            })))

        }
    }
}
