//! The **slicer** of a table: a widget that filters a column of the
//! table by the values that are chosen.
//!
//! A slicer is a **thin front-end** over the filter of the sheet: the
//! values it holds are written into the sheet's filter as a **list of
//! values**, and the rows the filter hides are derived from it. A
//! slicer that holds every value filters nothing.
//!
//! The rules:
//!
//! - a slicer is bound to a **column of a table** of the sheet, and
//!   the column is one of the table's **fields**;
//! - the **values** of the slicer are the texts the cells of that
//!   column show, and they are **distinct** — the tree's own rule;
//! - a value that is not in the column is **refused** by name;
//! - a **single** click keeps that value and nothing else, and a click
//!   on the only value that is kept takes the filter away;
//! - a **multi** click adds the value to the ones that are kept, and
//!   a click on a value that is kept takes it away; and when every
//!   value is kept the filter is taken away, because a filter that
//!   keeps everything filters nothing;
//! - the filter of the sheet is the slicer's, and the rows it hides
//!   are **derived**.
//!
//! # What the port found
//!
//! - **The tree's slicer is a widget on the sheet.** The tree keeps
//!   the place of it, its width, its height and its caption, and the
//!   widget is drawn. Here a slicer is the **table, the column and
//!   the values that are kept**, and a widget is a drawing and a
//!   drawing is not a fact of the sheet.
//! - **The tree's slicer writes into the filter of the table's range.**
//!   The tree's filter holds a criterion for every column of its
//!   range, so a slicer adds to the filter and takes nothing away.
//!   Here a sheet has **one** filter, and the slicer writes it.
//! - **The tree's filter is over the range of the table and this one's
//!   is over the sheet.** The tree's filter keeps the heading row of
//!   the range and the rows outside it; here the heading row of the
//!   table is hidden with the rows that do not show the value. That is
//!   the difference the filter of a sheet already has (§347).
//! - **The tree's slicer is found on the sheet of the selection.** The
//!   tree's button acts on the active sheet; here the sheet is named,
//!   and the slicer is named by its **id**.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, Node, NodeRef, Number, Outcome, Session, Text};

use crate::formula::{CellRef, Cells};
use crate::tables::{Table, fields_of, tables_of};

/// The most **slicers** a sheet may hold.
const MAX_SLICERS: usize = 64;

/// A **slicer** of a sheet.
#[derive(Clone, Debug)]
pub struct Slicer {
    /// Its id.
    pub id: u64,
    /// The **table** it is bound to.
    pub table: String,
    /// The **column** of the table it filters.
    pub column: String,
    /// The values that are kept, and nothing when every value is.
    pub selected: Option<Vec<String>>,
}

impl Slicer {
    /// The slicer, as the sheet keeps it.
    fn to_json(&self) -> Json {
        json!({
            "id": self.id,
            "table": self.table,
            "column": self.column,
            "selected": match &self.selected {
                Some(selected) => json!(selected),
                None => Json::Null,
            },
        })
    }
}

/// The **slicers** of a sheet, as the sheet keeps them.
fn slicers_of(node: &Node) -> Vec<Slicer> {
    let mut slicers = Vec::new();
    let Some(list) = node.data.get("slicers").and_then(|slicers| slicers.as_array()) else {
        return slicers;
    };
    for slicer in list {
        let Some(id) = slicer.get("id").and_then(|id| id.as_u64()) else { continue };
        let Some(table) = slicer.get("table").and_then(|table| table.as_str()) else { continue };
        let Some(column) = slicer.get("column").and_then(|column| column.as_str()) else { continue };
        let selected = slicer.get("selected").and_then(|selected| selected.as_array()).map(|selected| {
            selected.iter().filter_map(|value| value.as_str().map(|value| value.to_string())).collect()
        });
        slicers.push(Slicer { id, table: table.to_string(), column: column.to_string(), selected });
    }
    slicers
}

/// The **slicers** of a sheet, as the sheet keeps them.
fn slicers_into(slicers: &[Slicer]) -> Json {
    json!(slicers.iter().map(|slicer| slicer.to_json()).collect::<Vec<_>>())
}

/// The **sheet** a command is about, or the error that says there is
/// no such sheet.
fn sheet_of(session: &Session, id: &str, sheet: u64) -> Result<Node, EngineError> {
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
    Ok(node)
}

/// The **table** of a name, and the error that says there is none.
fn table_of(tables: &[Table], name: &str, id: &str) -> Result<Table, EngineError> {
    tables.iter().find(|table| table.name.eq_ignore_ascii_case(name)).cloned().ok_or_else(|| {
        EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no table `{name}` on the sheet"),
        }
    })
}

/// The **column** of a table, and the error that says there is none.
fn column_of(fields: &[(CellRef, String)], column: &str, id: &str) -> Result<u32, EngineError> {
    fields.iter().find(|(_, name)| name.eq_ignore_ascii_case(column)).map(|(cell, _)| cell.col).ok_or_else(
        || EngineError::Failed { id: id.to_string(), message: format!("the table has no column `{column}`") },
    )
}

/// The **values** of a column of a table: the texts the cells show, and
/// they are distinct.
///
/// The tree's own rule: the blank cells are left out, and the values
/// are told apart without case.
fn values_of(cells: &Cells, table: &Table, column: u32) -> Vec<String> {
    let mut values: Vec<String> = Vec::new();
    for row in (table.range.start.row + 1)..=table.range.end.row {
        let cell = CellRef { col: column, row };
        let Some(entry) = cells.get(cell) else { continue };
        let Some(value) = &entry.value else { continue };
        let text = crate::formula::display_text(value);
        if text.is_empty() {
            continue;
        }
        if values.iter().any(|value| value.eq_ignore_ascii_case(&text)) {
            continue;
        }
        values.push(text);
    }
    values
}

/// The **id** of a slicer, as it is named.
fn id_of(slicer: Option<Number>, id: &str) -> Result<Option<u64>, EngineError> {
    let Some(slicer) = slicer else { return Ok(None) };
    let number = slicer.0;
    if !number.is_finite() || number < 1.0 || number.fract() != 0.0 || number > u64::MAX as f64 {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the id of a slicer is a whole number, and {number} is not one"),
        });
    }
    Ok(Some(number as u64))
}

/// The **slicer** a command is about: the one with that id, or the
/// last one of the sheet.
///
/// The tree's own rule: the slicer is named, and a command that names
/// none takes the last one.
fn slicer_of(slicers: &[Slicer], id: Option<u64>, wanted: &str) -> Result<Slicer, EngineError> {
    let found = match id {
        Some(id) => slicers.iter().find(|slicer| slicer.id == id).cloned(),
        None => slicers.last().cloned(),
    };
    found.ok_or_else(|| EngineError::Failed {
        id: wanted.to_string(),
        message: "there is no slicer on this sheet".to_string(),
    })
}

commands! {
    /// **Add** a slicer to a table of a sheet.
    ///
    /// The slicer is bound to a column of the table, and it starts with
    /// every value kept — and a filter that keeps everything filters
    /// nothing, so the sheet is not filtered yet.
    grid.slicer.add {
        area: "grid",
        layer: "grid",
        icon: "filter",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The **table** it is bound to.
            table: Text,
            /// The **column** of the table: the first one when it is not
            /// named.
            column: Option<Text>,
        },
        run: {
            let id = "grid.slicer.add".to_string();
            let sheet = self.sheet.0;
            let node = sheet_of(session, &id, sheet)?;
            let tables = tables_of(&node);
            let table = table_of(&tables, self.table.0.trim(), &id)?;
            let cells = crate::cells_of(&node);
            let fields =
                fields_of(&table, &cells).map_err(|message| EngineError::Failed { id: id.clone(), message })?;
            let column = match &self.column {
                Some(column) => column_of(&fields, column.0.trim(), &id)?,
                None => fields.first().map(|(cell, _)| cell.col).ok_or_else(|| EngineError::Failed {
                    id: id.clone(),
                    message: "the table has no columns".to_string(),
                })?,
            };
            let column_name = fields
                .iter()
                .find(|(cell, _)| cell.col == column)
                .map(|(_, name)| name.clone())
                .unwrap_or_default();
            let mut slicers = slicers_of(&node);
            if slicers.len() >= MAX_SLICERS {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("a sheet holds {MAX_SLICERS} slicers or fewer"),
                });
            }
            if slicers.iter().any(|slicer| {
                slicer.table.eq_ignore_ascii_case(&table.name) && slicer.column.eq_ignore_ascii_case(&column_name)
            }) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the column `{column_name}` of the table `{}` already has a slicer",
                        table.name
                    ),
                });
            }
            let next = slicers.iter().map(|slicer| slicer.id).max().unwrap_or(0) + 1;
            slicers.push(Slicer {
                id: next,
                table: table.name.clone(),
                column: column_name.clone(),
                selected: None,
            });
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["slicers"] = slicers_into(&slicers);
            }
            Ok(Outcome::changed(format!("the table `{}` has a slicer on its column `{column_name}`", table.name))
                .with_data(json!({
                    "sheet": sheet,
                    "slicer": next,
                    "table": table.name,
                    "column": column_name,
                })))

        }
    }

    /// **Read** a slicer: its values, and which of them are kept.
    grid.slicer.read {
        area: "grid",
        layer: "grid",
        icon: "list",
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// Which slicer: the last one when it is not named.
            slicer: Option<Number>,
        },
        run: {
            let id = "grid.slicer.read".to_string();
            let sheet = self.sheet.0;
            let node = sheet_of(session, &id, sheet)?;
            let slicer = slicer_of(&slicers_of(&node), id_of(self.slicer, &id)?, &id)?;
            let tables = tables_of(&node);
            let table = table_of(&tables, &slicer.table, &id)?;
            let cells = crate::cells_of(&node);
            let fields =
                fields_of(&table, &cells).map_err(|message| EngineError::Failed { id: id.clone(), message })?;
            let column = column_of(&fields, &slicer.column, &id)?;
            let values = values_of(&cells, &table, column);
            let items: Vec<Json> = values
                .iter()
                .map(|value| {
                    let selected = match &slicer.selected {
                        Some(selected) => selected.iter().any(|other| other.eq_ignore_ascii_case(value)),
                        None => true,
                    };
                    json!({ "value": value, "selected": selected })
                })
                .collect();
            Ok(Outcome::new(format!("the slicer holds {} value(s)", items.len())).with_data(json!({
                "sheet": sheet,
                "slicer": slicer.id,
                "table": slicer.table,
                "column": slicer.column,
                "items": items,
            })))

        }
    }

    /// **Toggle** a value of a slicer.
    ///
    /// The value is kept, and the rows that do not show it are hidden. A
    /// **multi** toggle adds the value to the ones that are kept and takes
    /// it away again, and a single toggle keeps that value and nothing
    /// else.
    grid.slicer.toggle {
        area: "grid",
        layer: "grid",
        icon: "filter",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// Which slicer: the last one when it is not named.
            slicer: Option<Number>,
            /// The value.
            value: Text,
            /// Whether the value is added to the ones that are kept.
            multi: Option<Bool>,
        },
        run: {
            let id = "grid.slicer.toggle".to_string();
            let sheet = self.sheet.0;
            let node = sheet_of(session, &id, sheet)?;
            let mut slicers = slicers_of(&node);
            let at = {
                let slicer = slicer_of(&slicers, id_of(self.slicer, &id)?, &id)?;
                slicers.iter().position(|other| other.id == slicer.id).ok_or_else(|| EngineError::Failed {
                    id: id.clone(),
                    message: "there is no slicer on this sheet".to_string(),
                })?
            };
            let slicer = slicers[at].clone();
            let tables = tables_of(&node);
            let table = table_of(&tables, &slicer.table, &id)?;
            let cells = crate::cells_of(&node);
            let fields =
                fields_of(&table, &cells).map_err(|message| EngineError::Failed { id: id.clone(), message })?;
            let column = column_of(&fields, &slicer.column, &id)?;
            let values = values_of(&cells, &table, column);
            let value = self.value.0.clone();
            if !values.iter().any(|other| other.eq_ignore_ascii_case(&value)) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{value}` is not in the column `{}`", slicer.column),
                });
            }
            let multi = self.multi.map(|multi| multi.0).unwrap_or(false);
            let selected = if multi {
                let mut kept = slicer.selected.clone().unwrap_or_else(|| values.clone());
                match kept.iter().position(|other| other.eq_ignore_ascii_case(&value)) {
                    Some(at) => {
                        kept.remove(at);
                    }
                    None => kept.push(value.clone()),
                }
                // Everything kept is the same as nothing filtered.
                if kept.len() == values.len()
                    && values.iter().all(|other| kept.iter().any(|kept| kept.eq_ignore_ascii_case(other)))
                {
                    None
                } else {
                    Some(kept)
                }
            } else {
                // A single click keeps that value alone, and a click on the
                // only value that is kept takes the filter away.
                match &slicer.selected {
                    Some(kept)
                        if kept.len() == 1 && kept.iter().any(|other| other.eq_ignore_ascii_case(&value)) =>
                    {
                        None
                    }
                    _ => Some(vec![value.clone()]),
                }
            };
            slicers[at].selected = selected.clone();
            let filter = match &selected {
                Some(kept) => json!({
                    "column": CellRef::column_text(column),
                    "values": kept,
                    "blanks": false,
                }),
                None => Json::Null,
            };
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["slicers"] = slicers_into(&slicers);
                node.data["filter"] = filter;
            }
            Ok(Outcome::changed(format!("the slicer of the column `{}` keeps {value}", slicer.column)).with_data(
                json!({
                    "sheet": sheet,
                    "slicer": slicer.id,
                    "column": slicer.column,
                    "selected": match &selected {
                        Some(kept) => json!(kept),
                        None => Json::Null,
                    },
                }),
            ))

        }
    }

    /// **Clear** a slicer: every value is kept, and the filter is taken
    /// away.
    grid.slicer.clear {
        area: "grid",
        layer: "grid",
        icon: "filter",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// Which slicer: the last one when it is not named.
            slicer: Option<Number>,
        },
        run: {
            let id = "grid.slicer.clear".to_string();
            let sheet = self.sheet.0;
            let node = sheet_of(session, &id, sheet)?;
            let mut slicers = slicers_of(&node);
            let at = {
                let slicer = slicer_of(&slicers, id_of(self.slicer, &id)?, &id)?;
                slicers.iter().position(|other| other.id == slicer.id).ok_or_else(|| EngineError::Failed {
                    id: id.clone(),
                    message: "there is no slicer on this sheet".to_string(),
                })?
            };
            let slicer = slicers[at].clone();
            slicers[at].selected = None;
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["slicers"] = slicers_into(&slicers);
                node.data["filter"] = Json::Null;
            }
            Ok(Outcome::changed(format!("the slicer of the column `{}` keeps every value", slicer.column))
                .with_data(json!({
                    "sheet": sheet,
                    "slicer": slicer.id,
                    "column": slicer.column,
                })))

        }
    }

    /// **Remove** a slicer, and the filter it wrote.
    grid.slicer.remove {
        area: "grid",
        layer: "grid",
        icon: "filter",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// Which slicer: the last one when it is not named.
            slicer: Option<Number>,
        },
        run: {
            let id = "grid.slicer.remove".to_string();
            let sheet = self.sheet.0;
            let node = sheet_of(session, &id, sheet)?;
            let mut slicers = slicers_of(&node);
            let slicer = slicer_of(&slicers, id_of(self.slicer, &id)?, &id)?;
            slicers.retain(|other| other.id != slicer.id);
            // The filter of the sheet is the slicer's when it is a list of
            // values on the column of the slicer, and it is taken away with
            // it.
            let tables = tables_of(&node);
            let table = table_of(&tables, &slicer.table, &id)?;
            let cells = crate::cells_of(&node);
            let fields =
                fields_of(&table, &cells).map_err(|message| EngineError::Failed { id: id.clone(), message })?;
            let column = column_of(&fields, &slicer.column, &id)?;
            let filter = node.data.get("filter").cloned();
            let mine = filter
                .as_ref()
                .and_then(|filter| filter.get("values"))
                .and_then(|values| values.as_array())
                .is_some()
                && filter
                    .as_ref()
                    .and_then(|filter| filter.get("column"))
                    .and_then(|column| column.as_str())
                    .and_then(CellRef::column_index)
                    == Some(column);
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["slicers"] = slicers_into(&slicers);
                if mine {
                    node.data["filter"] = Json::Null;
                }
            }
            Ok(Outcome::changed(format!("the slicer of the column `{}` is taken away", slicer.column)).with_data(
                json!({
                    "sheet": sheet,
                    "slicer": slicer.id,
                    "column": slicer.column,
                }),
            ))

        }
    }
}
