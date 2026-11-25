//! The **filter** of a sheet.
//!
//! A filter is a **column** and a **criterion**, and the criterion is
//! an operation and a value: `B` is `>` `10`, and the rows whose `B`
//! is not more than ten are the rows the filter **hides**.
//!
//! A filter may also be **groups** of criteria: the columns of a
//! group are judged together and the groups are judged one by one, and
//! a row is hidden when it matches **none** of them. A column filter
//! is one group of one criterion.
//!
//! And the rows a filter hides are **derived**: they are worked out
//! from the cells each time they are asked for, and they are not
//! written down. A row hidden **by hand** (§345) is a fact of the
//! sheet and a row hidden by a **filter** is a consequence of one —
//! and the tree's `SUBTOTAL` tells them apart: the rows a filter
//! hides are left out whatever the function number, and the rows
//! hidden by hand are left out only in the hundred-and-more form.
//!
//! The tree's criterion is a pair of strings — an operation and a
//! value — and the value may hold the tree's `*` wildcard: `*abc*` is
//! "contains abc" and `abc*` is "begins with abc".
//!
//! And the criterion has four **operations on the text** of a cell
//! beside the four comparisons: `beginsWith`, `endsWith`, `contains`
//! and `notContains`. The words are the words there, and not
//! wildcards.
//!
//! And four operations that name a cell by **what it holds** and not
//! by a value: `isEmpty`, `isNotEmpty`, `isError` and `isNotError`.
//! They compare with nothing, so their criterion has **no value** — an
//! absent value and an empty one are different things, and the tree
//! refuses the one that does not belong.
//!
//! And a filter may judge the **colour** of a cell: the fill behind
//! its text, or the colour of its text. That is a filter of its own
//! (`grid.filter.colour`), because a colour is not a value.
//!
//! And the number a comparison operation is compared with is read by
//! the tree's **own rule of numbers** — the rule `VALUE` reads a text
//! by — so a currency sign, thousands separators and a date are
//! numbers here too.
//!
//! # What the port found
//!
//! - **The tree's filter has many kinds of criterion and this one has
//!   two.** The tree's criterion may be a list of values, a pair of
//!   comparisons joined by and or or, a top-ten, an above-the-average
//!   or a colour. Here a criterion is **one** operation and **one**
//!   value — the pair of comparisons — and the **list of values** is a
//!   filter of its own (`grid.filter.values`); the other kinds are
//!   commands of their own when they come — and §1012 says otherwise:
//!   the **top ten** is here (`grid.filter.top10`), and the
//!   above-the-average and the colours are not — and §1013 says
//!   otherwise as well: the **above-the-average** is here
//!   (`grid.filter.average`), and the colours are not.
//! - **The tree's filter is over a range and this one's is over a
//!   column.** The tree's filter has a range and a criterion for each
//!   column of it. Here a sheet has **one** filter, and it is either
//!   over one column (`grid.filter.set`), over one column and a
//!   **list of values** (`grid.filter.values`) or over **groups** of
//!   criteria (`grid.filter.groups.set`) — the columns of a group are
//!   judged together and the groups are judged one by one. The rows it
//!   hides are the rows of the sheet either way.
//! - **The tree's comparison is Excel's and this one's is simpler.**
//!   The tree compares two numbers rounded to fifteen significant
//!   digits, treats an empty cell as zero, as `""` or as `FALSE`
//!   depending on the other side, and orders values of different kinds
//!   by rank. Here the comparison operations compare **numbers** — an
//!   empty cell is zero — and the equality operations compare the
//!   **text** of the cell, without case, with the tree's `*`.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, NodeRef, Number, Outcome, Text};

use crate::formula::{CellRef, Criterion, FILTER_OPS};

commands! {
    /// **Filter** a sheet by a column.
    ///
    /// The criterion is an operation and a value: `=` `!=` `<` `<=` `>` or
    /// `>=`, and a value that may hold the tree's `*`.
    grid.filter.set {
        area: "grid",
        layer: "grid",
        icon: "filter",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// Which column it filters: `A`, `B`, `AA`.
            column: Text,
            /// The operation: `=`, `!=`, `<`, `<=`, `>` or `>=`, or one
            /// of the four on the **text** of a cell: `beginsWith`,
            /// `endsWith`, `contains` or `notContains`, or one of the
            /// four that name a cell by **what it holds**: `isEmpty`,
            /// `isNotEmpty`, `isError` or `isNotError`.
            op: Text,
            /// The value it compares with. A comparison operation needs a
            /// **number**, and the value may hold the tree's `*`. An
            /// operation that names a cell by what it holds compares
            /// with nothing, and it takes no value.
            value: Option<Text>,
        },
        run: {
            let id = "grid.filter.set".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet) else {
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
            crate::protection::refuse_filter(node, &id)?;
            let column = self.column.0.trim().to_uppercase();
            if CellRef::column_index(&column).is_none() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{column}` is not a column: a column is `A`, `B` or `AA`"),
                });
            };
            let op = self.op.0.trim().to_string();
            if !Criterion::op_is_known(&op) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "there is no operation `{op}`: the operations are {}",
                        FILTER_OPS.join(", ")
                    ),
                });
            }
            let value = match Criterion::value_of(&op, self.value.as_ref().map(|value| value.0.as_str())) {
                Ok(value) => value,
                Err(message) => return Err(EngineError::Failed { id: id.clone(), message }),
            };
            // The **number** the operation compares with is read by the
            // tree's own rule of numbers — the rule `VALUE` reads a text
            // by — so a currency sign, thousands separators and a date
            // are numbers.
            let number = crate::textvalue::number_of(&value, crate::system_of(session, sheet));
            if (op == "<" || op == "<=" || op == ">" || op == ">=") && number.is_none() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the operation `{op}` compares numbers, and `{value}` is not one"),
                });
            }
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["filter"] = json!({
                    "column": column,
                    "op": op,
                    "value": value,
                });
            }
            Ok(Outcome::changed(format!("the sheet `{sheet}` is filtered by the column {column}"))
                .with_data(json!({ "sheet": sheet, "column": column, "op": op, "value": value })))

        }
    }

    /// **Filter** a sheet by the **highest** (or the lowest) values of a
    /// column.
    ///
    /// The **threshold** is the count-th highest number of the column —
    /// or the count-th lowest, for the lowest — and a row is kept when
    /// its number is at the threshold or above it. A cell that does not
    /// hold a number is not kept.
    ///
    /// The count is a count of rows — ten of them by default — or a
    /// **percent** of the numbers of the column, and it is rounded up.
    grid.filter.top10 {
        area: "grid",
        layer: "grid",
        icon: "filter",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// Which column it filters: `A`, `B`, `AA`.
            column: Text,
            /// How many rows are kept: ten of them by default.
            count: Option<Number>,
            /// Whether the count is a **percent** of the numbers of the
            /// column.
            percent: Option<Bool>,
            /// Whether the **lowest** values are kept.
            bottom: Option<Bool>,
        },
        run: {
            let id = "grid.filter.top10".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            if node.kind != "grid.sheet" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{sheet}` is a `{}` and not a sheet", node.kind),
                });
            }
            crate::protection::refuse_filter(node, &id)?;
            let column = self.column.0.trim().to_uppercase();
            if CellRef::column_index(&column).is_none() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{column}` is not a column: a column is `A`, `B` or `AA`"),
                });
            };
            let count = self.count.map(|count| count.0).unwrap_or(10.0);
            if !count.is_finite() || count < 1.0 || count.fract() != 0.0 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the count of a top-ten filter is a whole number of rows, and `{count}` is not one"),
                });
            }
            let count = count as u32;
            let percent = self.percent.map(|percent| percent.0).unwrap_or(false);
            if percent && count > 100 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the percent of a top-ten filter is a hundred at most, and this one is {count}"),
                });
            }
            let bottom = self.bottom.map(|bottom| bottom.0).unwrap_or(false);
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["filter"] = json!({
                    "column": column,
                    "top10": {
                        "count": count,
                        "percent": percent,
                        "bottom": bottom,
                    },
                });
            }
            Ok(Outcome::changed(format!("the sheet `{sheet}` is filtered by the column {column}"))
                .with_data(json!({
                    "sheet": sheet,
                    "column": column,
                    "count": count,
                    "percent": percent,
                    "bottom": bottom,
                })))

        }
    }

    /// **Filter** a sheet by the rows that are **above** the average of
    /// a column — or below it.
    ///
    /// The **average** is the average of the numbers of the column, and
    /// a row is kept when its number is above it. A cell that does not
    /// hold a number is not kept.
    grid.filter.average {
        area: "grid",
        layer: "grid",
        icon: "filter",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// Which column it filters: `A`, `B`, `AA`.
            column: Text,
            /// Whether the rows **above** the average are kept; the rows
            /// below it are kept when they are not.
            above: Option<Bool>,
        },
        run: {
            let id = "grid.filter.average".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            if node.kind != "grid.sheet" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{sheet}` is a `{}` and not a sheet", node.kind),
                });
            }
            crate::protection::refuse_filter(node, &id)?;
            let column = self.column.0.trim().to_uppercase();
            if CellRef::column_index(&column).is_none() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{column}` is not a column: a column is `A`, `B` or `AA`"),
                });
            };
            let above = self.above.map(|above| above.0).unwrap_or(true);
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["filter"] = json!({
                    "column": column,
                    "average": { "above": above },
                });
            }
            Ok(Outcome::changed(format!("the sheet `{sheet}` is filtered by the column {column}"))
                .with_data(json!({ "sheet": sheet, "column": column, "above": above })))

        }
    }

    /// **Filter** a sheet by the **colour** of its cells.
    ///
    /// The colour is the cell's **own**: the fill behind its text, or
    /// the colour of its text. A cell that carries no such colour is
    /// not kept.
    grid.filter.colour {
        area: "grid",
        layer: "grid",
        icon: "filter",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// Which column it filters: `A`, `B`, `AA`.
            column: Text,
            /// The colour: `#rrggbb`.
            colour: Text,
            /// Whether the **colour of the text** is judged, and not the
            /// fill behind it.
            font: Option<Bool>,
        },
        run: {
            let id = "grid.filter.colour".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            if node.kind != "grid.sheet" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{sheet}` is a `{}` and not a sheet", node.kind),
                });
            }
            crate::protection::refuse_filter(node, &id)?;
            let column = self.column.0.trim().to_uppercase();
            if CellRef::column_index(&column).is_none() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{column}` is not a column: a column is `A`, `B` or `AA`"),
                });
            };
            let Some(Some(colour)) = crate::colours::colour_of(&self.colour.0) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "`{}` is not a colour: a colour is `#rrggbb`",
                        self.colour.0.trim()
                    ),
                });
            };
            let font = self.font.map(|font| font.0).unwrap_or(false);
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["filter"] = json!({
                    "column": column,
                    "colour": colour,
                    "font": font,
                });
            }
            Ok(Outcome::changed(format!("the sheet `{sheet}` is filtered by the column {column}"))
                .with_data(json!({ "sheet": sheet, "column": column, "colour": colour, "font": font })))

        }
    }

    /// Take the **filter** away from a sheet.
    grid.filter.clear {
        area: "grid",
        layer: "grid",
        icon: "filter",
        mutates,
        undoable,
        params: {
            /// The sheet.
            sheet: NodeRef,
        },
        run: {
            let id = "grid.filter.clear".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet) else {
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
            crate::protection::refuse_filter(node, &id)?;
            if node.data.get("filter").is_none() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the sheet `{sheet}` is not filtered"),
                });
            }
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["filter"] = json!(null);
            }
            Ok(Outcome::changed(format!("the filter of the sheet `{sheet}` is taken away"))
                .with_data(json!({ "sheet": sheet })))

        }
    }

    /// **Filter** a sheet by **groups** of criteria.
    ///
    /// The columns of a group are judged **together** — a row matches the
    /// group when it matches every criterion of it — and the groups are
    /// judged one by one: a row is hidden when it matches **none** of
    /// them.
    ///
    /// The groups are a list of lists of criteria, and a criterion says
    /// which **column** it judges, the **operation** and the **value**:
    /// `{"column": "B", "op": ">", "value": "10"}`.
    grid.filter.groups.set {
        area: "grid",
        layer: "grid",
        icon: "filter",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The **groups** of criteria: a list of lists of
            /// `{"column": "B", "op": ">", "value": "10"}`.
            groups: craftkit_sdk::serde_json::Value,
        },
        run: {
            let id = "grid.filter.groups.set".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet) else {
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
            crate::protection::refuse_filter(node, &id)?;
            let Some(groups) = self.groups.as_array() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the groups of a filter are a list".to_string(),
                });
            };
            if groups.is_empty() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a filter has a group of criteria or more".to_string(),
                });
            }
            // Every **criterion** of every **group**: the column it
            // judges, the operation and the value.
            let mut read: Vec<Json> = Vec::with_capacity(groups.len());
            for group in groups {
                let Some(criteria) = group.as_array() else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "a group of criteria is a list".to_string(),
                    });
                };
                if criteria.is_empty() {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "a group of criteria has a criterion or more".to_string(),
                    });
                }
                let mut read_group: Vec<Json> = Vec::with_capacity(criteria.len());
                for criterion in criteria {
                    let column = criterion.get("column").and_then(|column| column.as_str()).ok_or_else(|| {
                        EngineError::Failed {
                            id: id.clone(),
                            message: "a criterion says which column it judges".to_string(),
                        }
                    })?;
                    let column = column.trim().to_uppercase();
                    if CellRef::column_index(&column).is_none() {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("`{column}` is not a column: a column is `A`, `B` or `AA`"),
                        });
                    }
                    let op = criterion.get("op").and_then(|op| op.as_str()).unwrap_or("=").trim().to_string();
                    if !Criterion::op_is_known(&op) {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!(
                                "there is no operation `{op}`: the operations are {}",
                                FILTER_OPS.join(", ")
                            ),
                        });
                    }
                    let value = criterion.get("value").and_then(|value| value.as_str()).unwrap_or("").to_string();
                    if (op == "<" || op == "<=" || op == ">" || op == ">=")
                        && value.trim().parse::<f64>().is_err()
                    {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("the operation `{op}` compares numbers, and `{value}` is not one"),
                        });
                    }
                    read_group.push(json!({ "column": column, "op": op, "value": value }));
                }
                read.push(Json::Array(read_group));
            }
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["filter"] = json!({ "groups": read });
            }
            Ok(Outcome::changed(format!("the sheet `{sheet}` is filtered by {} group(s)", groups.len()))
                .with_data(json!({ "sheet": sheet, "groups": read })))

        }
    }

    /// The **rows** a sheet's filter hides.
    ///
    /// The rows are **derived**: they are worked out from the cells each
    /// time they are asked for.
    grid.filter.rows {
        area: "grid",
        layer: "grid",
        icon: "list",
        params: {
            /// The sheet.
            sheet: NodeRef,
        },
        run: {
            let id = "grid.filter.rows".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet) else {
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
            let book = crate::book_of(session.document());
            let name = node.name.clone();
            let rows = book.filtered_rows(&name);
            let count = rows.len();
            Ok(Outcome::new(format!("{count} row(s) are hidden by the filter")).with_data(json!({
                "sheet": sheet,
                "rows": rows,
            })))

        }
    }

    /// **Filter** a sheet by a **list of values**.
    ///
    /// The tree's own list filter: the rows whose cell shows one of the
    /// values are kept, and the others are hidden. The comparison is of
    /// the **text** the cell shows, and it is without case — the tree's
    /// own rule.
    ///
    /// An empty cell is kept when **blanks** is asked for, and hidden when
    /// it is not: the tree's own rule.
    grid.filter.values {
        area: "grid",
        layer: "grid",
        icon: "filter",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// Which column it filters: `A`, `B`, `AA`.
            column: Text,
            /// The **values** that are kept: a list of texts.
            values: craftkit_sdk::serde_json::Value,
            /// Whether an empty cell is kept.
            blanks: Option<Bool>,
        },
        run: {
            let id = "grid.filter.values".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet) else {
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
            crate::protection::refuse_filter(node, &id)?;
            let column = self.column.0.trim().to_uppercase();
            if CellRef::column_index(&column).is_none() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{column}` is not a column: a column is `A`, `B` or `AA`"),
                });
            }
            let Some(values) = self.values.as_array() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the values of a filter are a list".to_string(),
                });
            };
            if values.len() > MAX_FILTER_VALUES {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "a filter holds {MAX_FILTER_VALUES} values or fewer, and this one holds {}",
                        values.len()
                    ),
                });
            }
            let mut read: Vec<String> = Vec::with_capacity(values.len());
            for value in values {
                let Some(value) = value.as_str() else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "a value of a filter is text".to_string(),
                    });
                };
                if value.chars().count() > MAX_VALUE_CHARS {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("a value of a filter holds {MAX_VALUE_CHARS} characters or fewer"),
                    });
                }
                read.push(value.to_string());
            }
            let blanks = self.blanks.map(|blanks| blanks.0).unwrap_or(false);
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["filter"] = json!({
                    "column": column,
                    "values": read,
                    "blanks": blanks,
                });
            }
            Ok(Outcome::changed(format!("the sheet `{sheet}` is filtered by the column {column}")).with_data(
                json!({
                    "sheet": sheet,
                    "column": column,
                    "values": read,
                    "blanks": blanks,
                }),
            ))

        }
    }
}

/// The most **values** a list filter may hold.
const MAX_FILTER_VALUES: usize = 1024;

/// The most **characters** one value of a list filter may hold.
const MAX_VALUE_CHARS: usize = 255;
