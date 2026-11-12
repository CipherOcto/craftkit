//! The **special** cells of a sheet: the cells that answer a question
//! asked of the sheet.
//!
//! The tree's own `edit.goToSpecial`, and the tree's own kinds:
//! **blanks**, **constants**, **formulas**, **comments**, **notes**,
//! **errors**, **numbers**, **text**, **visible**, **conditional
//! formats**, **data validation**, the **last cell** and the **current
//! region**.
//!
//! # The tree's rules
//!
//! - the **scope** is the range the question is asked about, and the
//!   **used range** of the sheet when no range is given;
//! - a kind that is not one of them is **refused by name**, and the
//!   refusal says what the kinds are;
//! - the cells are found in the order a reader reads them in: the rows
//!   first, and the columns in a row;
//! - the **last cell** is the far corner of the used range, and it is
//!   `A1` on a sheet that holds nothing;
//! - the **current region** is the block of data around a cell: it
//!   grows while the line next to it has something in it.
//!
//! # What the port found
//!
//! - **The tree's comment and its note are one thing and here they are
//!   two.** The tree keeps a note in the map of the comments and the
//!   *go to special* finds a note by asking for `comments` or for
//!   `notes` — the same answer twice. Here a comment is a thread and a
//!   note is the yellow box, and they are two kinds: a cell that has a
//!   comment is not a cell that has a note.
//! - **The tree's selection is the scope and this tree has none.** The
//!   tree asks of the sheet that is chosen and of the cells that are
//!   chosen in it. Here the sheet is named and the range is named, and
//!   a range that is not given is the used range of the sheet.
//! - **The tree's *current region* starts at the chosen cell.** Here
//!   the cell is named, and it is `A1` when it is not given.
//! - **The tree's *go to special* refuses when nothing is found.** Here
//!   the answer is a count of zero, as `grid.find` answers, because a
//!   question with no answer is an answer and not a failure.
//! - **The tree's names are the names here.** `conditionalFormats` is
//!   the tree's spelling and `conditional formats` and
//!   `conditional_formats` are the same kind: the spaces and the
//!   underscores are taken out before the kind is looked for, and the
//!   case does not matter.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Map, Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Outcome, Text};

use crate::formula::{Area, CellRef, Cells, Value};

/// The most cells one question looks at.
///
/// The tree's own bound is four million; this one is a million, and
/// the tree's is a bound on a rectangle of a sheet that is not walked
/// cell by cell as often as this one is.
const MAX_CELLS: u64 = 1_000_000;

/// The most cells one answer lists.
///
/// The count is always the whole truth; the list is bounded, and the
/// answer says so when it is short.
const MAX_HITS: usize = 1_000;

/// A **kind** of special cell: the tree's own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// The cells that hold nothing.
    Blanks,
    /// The cells that hold a value and not a formula.
    Constants,
    /// The cells that hold a formula.
    Formulas,
    /// The cells that hold a comment.
    Comments,
    /// The cells that hold a note.
    Notes,
    /// The cells whose value is an error.
    Errors,
    /// The cells whose value is a number.
    Numbers,
    /// The cells whose value is text.
    Text,
    /// The cells that are not hidden.
    Visible,
    /// The cells a conditional format is about.
    ConditionalFormats,
    /// The cells a validation rule is over.
    DataValidation,
    /// The far corner of the used range.
    LastCell,
    /// The block of data around a cell.
    CurrentRegion,
}

impl Kind {
    /// The **names** of the kinds: the tree's own, in the tree's own
    /// spelling.
    pub const NAMES: [&'static str; 13] = [
        "blanks",
        "constants",
        "formulas",
        "comments",
        "notes",
        "errors",
        "numbers",
        "text",
        "visible",
        "conditionalFormats",
        "dataValidation",
        "lastCell",
        "currentRegion",
    ];

    /// The **name** of the kind.
    pub fn name(self) -> &'static str {
        match self {
            Kind::Blanks => "blanks",
            Kind::Constants => "constants",
            Kind::Formulas => "formulas",
            Kind::Comments => "comments",
            Kind::Notes => "notes",
            Kind::Errors => "errors",
            Kind::Numbers => "numbers",
            Kind::Text => "text",
            Kind::Visible => "visible",
            Kind::ConditionalFormats => "conditionalFormats",
            Kind::DataValidation => "dataValidation",
            Kind::LastCell => "lastCell",
            Kind::CurrentRegion => "currentRegion",
        }
    }

    /// The kind a **name** says.
    ///
    /// The spaces, the underscores and the hyphens are taken out and
    /// the case is folded, so that the tree's own `conditionalFormats`
    /// and the words *conditional formats* are one kind and not three.
    pub fn parse(text: &str) -> Option<Kind> {
        let words: String =
            text.trim().to_lowercase().chars().filter(|character| character.is_alphanumeric()).collect();
        match words.as_str() {
            "blanks" => Some(Kind::Blanks),
            "constants" => Some(Kind::Constants),
            "formulas" => Some(Kind::Formulas),
            "comments" => Some(Kind::Comments),
            "notes" => Some(Kind::Notes),
            "errors" => Some(Kind::Errors),
            "numbers" => Some(Kind::Numbers),
            "text" => Some(Kind::Text),
            "visible" => Some(Kind::Visible),
            "conditionalformats" => Some(Kind::ConditionalFormats),
            "datavalidation" => Some(Kind::DataValidation),
            "lastcell" => Some(Kind::LastCell),
            "currentregion" => Some(Kind::CurrentRegion),
            _ => None,
        }
    }
}

/// The **current region** of a sheet: the block of data around a cell.
///
/// The tree's own rule: the region grows while the line next to it —
/// the diagonals included — has something in it, and it stops when
/// there is nothing on any of the four sides.
fn current_region(cells: &Cells, cell: CellRef) -> Area {
    // Whether a cell holds something.
    let filled = |cell: CellRef| match cells.get(cell) {
        Some(entry) => {
            entry.formula.is_some() || entry.value.as_ref().is_some_and(|value| *value != Value::Empty)
        }
        None => false,
    };
    // Whether a **row** has something in it between two columns.
    let row_has = |row: u32, first: u32, last: u32| (first..=last).any(|col| filled(CellRef { col, row }));
    // Whether a **column** has something in it between two rows.
    let column_has = |col: u32, first: u32, last: u32| (first..=last).any(|row| filled(CellRef { col, row }));
    let (mut top, mut bottom, mut left, mut right) = (cell.row, cell.row, cell.col, cell.col);
    loop {
        let before = (top, bottom, left, right);
        let (first_col, last_col) = (left.saturating_sub(1), right + 1);
        while top > 0 && row_has(top - 1, first_col, last_col) {
            top -= 1;
        }
        while row_has(bottom + 1, first_col, last_col) {
            bottom += 1;
        }
        let (first_row, last_row) = (top.saturating_sub(1), bottom + 1);
        while left > 0 && column_has(left - 1, first_row, last_row) {
            left -= 1;
        }
        while column_has(right + 1, first_row, last_row) {
            right += 1;
        }
        if (top, bottom, left, right) == before {
            break;
        }
    }
    Area::new(CellRef { col: left, row: top }, CellRef { col: right, row: bottom })
}

commands! {
    /// The **special** cells of a sheet.
    ///
    /// The tree's own `edit.goToSpecial`.
    grid.find.special {
        area: "grid",
        layer: "grid",
        icon: "list",
        params: {
            /// The sheet it is on.

            sheet: NodeRef,
            /// The kind of cell that is looked for: `blanks`, `constants`,
            /// `formulas`, `comments`, `notes`, `errors`, `numbers`, `text`,
            /// `visible`, `conditionalFormats`, `dataValidation`, `lastCell` or
            /// `currentRegion`.

            kind: Text,
            /// The range to look in. Without it, the **used range** of the
            /// sheet is looked in.

            range: Option<Text>,
            /// The cell the **current region** is worked out from: `A1` when
            /// it is not given.

            cell: Option<Text>,
        },
        run: {

            let id = "grid.find.special".to_string();
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
            // The **kind**: the tree refuses a kind it does not know, and
            // it says what the kinds are.
            let kind = Kind::parse(&self.kind.0).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "`{}` is not a kind: the kinds are {}",
                    self.kind.0.trim(),
                    Kind::NAMES.join(", ")
                ),
            })?;
            let name = node.name.clone();
            let cells = crate::cells_of(node);
            let book = crate::book_of(session.document());
            // The **scope**: the range the question is asked about, or the
            // used range of the sheet.
            let scope: Option<Area> = match &self.range {
                Some(text) => Some(crate::conditional::area_of(&text.0).ok_or_else(|| EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{}` is not a range of cells", text.0.trim()),
                })?),
                None => crate::reading::used_range(&cells),
            };
            // The **hits**, and the area they lie in.
            let (area, hits): (Option<Area>, Vec<(CellRef, Value)>) = match kind {
                // The **last cell**: the far corner of the used range, and
                // `A1` on a sheet that holds nothing.
                Kind::LastCell => {
                    let cell = scope.map(|area| area.end).unwrap_or(CellRef { col: 0, row: 0 });
                    let value = book.value_of(&name, cell);
                    (Some(Area::new(cell, cell)), vec![(cell, value)])
                }
                // The **current region**: the block of data around a cell.
                Kind::CurrentRegion => {
                    let cell = match &self.cell {
                        Some(text) => crate::cell_at(&text.0, &id)?,
                        None => CellRef { col: 0, row: 0 },
                    };
                    let region = current_region(&cells, cell);
                    let hits = (region.start.row..=region.end.row)
                        .flat_map(|row| (region.start.col..=region.end.col).map(move |col| CellRef { col, row }))
                        .map(|cell| {
                            let value = book.value_of(&name, cell);
                            (cell, value)
                        })
                        .collect();
                    (Some(region), hits)
                }
                // The other kinds: the cells of the scope, one by one.
                _ => {
                    let Some(area) = scope else {
                        return Ok(answer(sheet, &name, kind, None, Vec::new()));
                    };
                    if area.count() > MAX_CELLS {
                        return Err(EngineError::Failed {
                            id,
                            message: format!("the range is too large to look through ({MAX_CELLS} cells max)"),
                        });
                    }
                    // The **comments** and the **notes** of the sheet, and
                    // the rules that are about its cells.
                    let comments = crate::comments::comments_of(node);
                    let notes = crate::notes::notes_of(node);
                    let conditionals = crate::conditional::conditionals_of(node);
                    let validations = crate::validation::rules_of(node);
                    let hidden_rows = crate::hiding::hidden_rows(node);
                    let hidden_columns = crate::hiding::hidden_columns(node);
                    let mut hits: Vec<(CellRef, Value)> = Vec::new();
                    for row in area.start.row..=area.end.row {
                        for col in area.start.col..=area.end.col {
                            let cell = CellRef { col, row };
                            let entry = cells.get(cell);
                            // The **value** of the cell: what it says, and
                            // what it works out to when it holds a formula.
                            let value = match entry {
                                Some(entry) if entry.formula.is_some() => book.value_of(&name, cell),
                                Some(entry) => entry.value.clone().unwrap_or(Value::Empty),
                                None => Value::Empty,
                            };
                            let address = cell.to_text();
                            let found = match kind {
                                Kind::Blanks => {
                                    entry.is_none_or(|entry| entry.value.is_none() && entry.formula.is_none())
                                }
                                Kind::Constants => entry.is_some_and(|entry| {
                                    entry.formula.is_none()
                                        && entry.value.as_ref().is_some_and(|value| *value != Value::Empty)
                                }),
                                Kind::Formulas => entry.is_some_and(|entry| entry.formula.is_some()),
                                Kind::Comments => comments.contains_key(&address),
                                Kind::Notes => notes.contains_key(&address),
                                Kind::Errors => matches!(value, Value::Error(_)),
                                Kind::Numbers => matches!(value, Value::Number(_)),
                                Kind::Text => matches!(value, Value::Text(_)),
                                Kind::Visible => {
                                    !hidden_rows.contains(&(row as u64 + 1))
                                        && !hidden_columns.contains(&CellRef::column_text(col))
                                }
                                Kind::ConditionalFormats => conditionals.iter().any(|conditional| {
                                    crate::conditional::area_of(&conditional.range)
                                        .is_some_and(|area| area.covers(cell))
                                }),
                                Kind::DataValidation => validations.iter().any(|rule| {
                                    crate::conditional::area_of(&rule.range).is_some_and(|area| area.covers(cell))
                                }),
                                Kind::LastCell | Kind::CurrentRegion => false,
                            };
                            if found {
                                hits.push((cell, value));
                            }
                        }
                    }
                    (Some(area), hits)
                }
            };
            Ok(answer(sheet, &name, kind, area, hits))

        }
    }
}

/// The **answer** to the question: the cells, and how many there are.
///
/// The count is always the whole truth. The list is bounded, and the
/// answer says so when the list is short of the count.
fn answer(sheet: u64, name: &str, kind: Kind, area: Option<Area>, hits: Vec<(CellRef, Value)>) -> Outcome {
    let count = hits.len();
    let listed: Vec<Json> = hits
        .iter()
        .take(MAX_HITS)
        .map(|(cell, value)| json!({ "cell": cell.to_text(), "value": value.to_json() }))
        .collect();
    let mut data = Map::new();
    data.insert("sheet".to_string(), json!(sheet));
    data.insert("sheet_name".to_string(), json!(name));
    data.insert("kind".to_string(), json!(kind.name()));
    data.insert("count".to_string(), json!(count));
    data.insert("listed".to_string(), json!(listed.len()));
    data.insert("truncated".to_string(), json!(listed.len() < count));
    data.insert("hits".to_string(), Json::Array(listed));
    if let Some(area) = area {
        data.insert("area".to_string(), json!(area.to_text()));
    }
    Outcome::new(format!("{count} cell(s) are {kind}", kind = kind.name())).with_data(Json::Object(data))
}
