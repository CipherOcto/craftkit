//! **Finding** a cell.
//!
//! A sheet may be **searched**: the answer is the cells whose text
//! holds what was looked for.
//!
//! The tree's rule, in the tree's own words:
//!
//! - the **case** does not matter unless the search says it does, and
//!   when it does not matter both sides are folded;
//! - a search that holds a **wildcard** is a wildcard search: `*` is
//!   any run of characters and `?` is any one character, and the
//!   escape is `~`. Such a search looks for the pattern **inside** the
//!   text unless the search says the **whole cell** must match;
//! - a search without a wildcard looks for the text **inside** the
//!   cell's text, and for the whole text when the whole cell must
//!   match.
//!
//! And what is searched is the **text the cell shows**, unless the
//! search asks for the **formulas** — and then a cell that holds a
//! formula is searched by its formula and a cell that holds a value is
//! searched by its value.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, NodeRef, Outcome, Session, Text};

use crate::formula::{CellRef, Cells, Value};

/// Whether a **text** holds what was looked for.
///
/// The tree's rule: the case does not matter unless it is asked for,
/// a wildcard search is a wildcard search, and a search without a
/// wildcard looks for the text inside the cell's text.
pub(crate) fn holds(cell: &str, what: &str, case: bool, whole: bool) -> bool {
    let (cell, what) =
        if case { (cell.to_string(), what.to_string()) } else { (cell.to_lowercase(), what.to_lowercase()) };
    if what.contains(['*', '?']) {
        let pattern = if whole { what } else { format!("*{what}*") };
        return crate::formula::wildcard_with_case(&pattern, &cell, case);
    }
    if whole { cell == what } else { cell.contains(&what) }
}

/// The **text** of a cell: the text it shows, or its formula.
pub(crate) fn text_of(
    session: &Session,
    sheet: u64,
    name: &str,
    cells: &Cells,
    cell: CellRef,
    formulas: bool,
) -> String {
    let entry = cells.get(cell);
    if formulas && let Some(formula) = entry.and_then(|entry| entry.formula.clone()) {
        return formula;
    }
    let value = match entry {
        Some(entry) => match (&entry.value, &entry.formula) {
            (Some(value), _) => value.clone(),
            (None, Some(_)) => crate::book_of(session.document()).value_of(name, cell),
            (None, None) => Value::Empty,
        },
        None => Value::Empty,
    };
    // The **format** the cell is written with: the conditional format
    // that matches it, if there is one, and otherwise the format of the
    // cell itself.
    let node = session.document().node(sheet);
    let format = match node {
        Some(node) => match crate::conditional::format_for(node, cell, &value) {
            Some(code) => crate::format::Format::parse(&code).unwrap_or(crate::format::Format::General),
            None => crate::format_of(session, sheet, cells, cell),
        },
        None => crate::format::Format::General,
    };
    format.text(&value, crate::system_of(session, sheet))
}

/// **Find** the cells whose text holds what was looked for.
#[derive(Command)]
#[command(id = "grid.find", area = "grid", layer = "grid", icon = "list")]
pub struct FindCells {
    /// The sheet to look in. Without it, the whole workbook is looked
    /// in.
    pub sheet: Option<NodeRef>,
    /// What to look for.
    pub what: Text,
    /// Whether the **case** matters.
    pub match_case: Option<Bool>,
    /// Whether the **whole cell** must match.
    pub whole_cell: Option<Bool>,
    /// Whether to look in the **formulas** rather than in the values.
    pub formulas: Option<Bool>,
}

impl FindCells {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.find".to_string();
        let what = self.what.0.clone();
        if what.is_empty() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "there is nothing to look for".to_string(),
            });
        }
        // The **sheets** that are looked in: the one that was named, or
        // every sheet of the workbook.
        let sheets: Vec<(u64, String)> = match self.sheet {
            Some(sheet) => {
                let Some(node) = session.document().node(sheet.0) else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("there is no sheet `{}`", sheet.0),
                    });
                };
                if node.kind != "grid.sheet" {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the node `{}` is a `{}` and not a sheet", sheet.0, node.kind),
                    });
                }
                vec![(sheet.0, node.name.clone())]
            }
            None => session
                .document()
                .walk()
                .filter(|node| node.kind == "grid.sheet")
                .map(|node| (node.id, node.name.clone()))
                .collect(),
        };
        let case = self.match_case.map(|flag| flag.0).unwrap_or(false);
        let whole = self.whole_cell.map(|flag| flag.0).unwrap_or(false);
        let formulas = self.formulas.map(|flag| flag.0).unwrap_or(false);
        let mut hits: Vec<Json> = Vec::new();
        for (sheet, name) in sheets {
            let Some(node) = session.document().node(sheet) else { continue };
            let cells = crate::cells_of(node);
            let mut found: Vec<(u32, u32, Json)> = Vec::new();
            for (cell, _) in cells.iter() {
                let text = text_of(session, sheet, &name, &cells, cell, formulas);
                if holds(&text, &what, case, whole) {
                    found.push((
                        cell.row,
                        cell.col,
                        json!({
                            "sheet": sheet,
                            "sheet_name": name,
                            "cell": cell.to_text(),
                            "text": text,
                        }),
                    ));
                }
            }
            // The hits are in the order a reader reads them in: the rows
            // first, and the columns in a row.
            found.sort_by_key(|(row, col, _)| (*row, *col));
            hits.extend(found.into_iter().map(|(_, _, hit)| hit));
        }
        let count = hits.len();
        Ok(Outcome::new(format!("{count} cell(s) hold `{what}`")).with_data(json!({
            "what": what,
            "count": count,
            "hits": hits,
        })))
    }
}
