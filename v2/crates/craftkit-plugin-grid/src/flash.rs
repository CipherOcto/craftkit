//! The **flash fill** of a column.
//!
//! The tree's `flash_fill`: the column to the **left** of the cell is
//! the source, the rows that hold a value in the target column are the
//! **examples**, and what the examples say is a **program** — a
//! sequence of pieces — that is then run on the rows that are empty.
//!
//! The program is not a pattern of numbers and not a list: it is a
//! little program of its own, and the tree learns it from the examples
//! and refuses to run one that does not explain all of them.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{self, json};
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session, Text};

use crate::formula::{CellRef, Cells, Value};

/// **Flash fill** a column: learn from the rows that are filled and
/// fill the rows that are not.
///
/// The cell names the column to fill, and the column to its **left**
/// is the source. The rows that hold a value in the target column are
/// the **examples**.
///
/// A column with **no examples** is refused: there is nothing to learn
/// from, and a command that quietly does nothing is a command that
/// answers a question nobody asked. So is a column whose examples do
/// not share a program.
#[derive(Command)]
#[command(id = "grid.flash.fill", area = "grid", layer = "grid", icon = "plus", mutates, undoable)]
pub struct FlashFill {
    /// The sheet.
    pub sheet: NodeRef,
    /// The cell whose column is filled.
    pub at: Text,
}

impl FlashFill {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.flash.fill".to_string();
        let sheet = self.sheet.0;
        let at = crate::cell_at(&self.at.0, &id)?;
        if at.col == 0 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "the first column has nothing beside it to learn from".to_string(),
            });
        }
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
        let cells = crate::cells_of(node);
        // The **date system** of the workbook: the text the cells show
        // is the text they show in that calendar.
        let system = crate::system_of(session, sheet);
        let source = CellRef { col: at.col - 1, row: at.row };
        // The **region**: the rows the source column holds, above the
        // cell and below it.
        let mut top = at.row;
        while top > 0 && !is_empty(&cells, CellRef { col: source.col, row: top - 1 }) {
            top -= 1;
        }
        let mut bottom = at.row;
        while !is_empty(&cells, CellRef { col: source.col, row: bottom + 1 }) {
            bottom += 1;
        }
        // The **examples** are the rows that are filled, and the **todo**
        // are the rows that are not. The tree reads the text the cells
        // **show** — a date as it is written and not as a number.
        let mut examples: Vec<(String, String)> = Vec::new();
        let mut todo: Vec<(CellRef, String)> = Vec::new();
        for row in top..=bottom {
            let from = shown_text(session, sheet, &cells, CellRef { col: source.col, row }, system);
            let to = CellRef { col: at.col, row };
            if is_empty(&cells, to) {
                todo.push((to, from));
            } else {
                examples.push((from, shown_text(session, sheet, &cells, to, system)));
            }
        }
        if examples.is_empty() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "there is nothing to learn from: the column has no filled rows".to_string(),
            });
        }
        if todo.is_empty() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "there is nothing to fill: every row of the column is filled".to_string(),
            });
        }
        let Some(program) = learn(&examples) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "the filled rows do not share a pattern this tree can learn".to_string(),
            });
        };
        // The **format** of the column: the tree's fill takes the style
        // of the top of the column it fills.
        let format = cells.format(CellRef { col: at.col, row: top }).map(|code| code.to_string());
        // A cell that a **merge** covers may not be written to.
        if let Some(node) = session.document().node(sheet) {
            let cells: Vec<crate::formula::CellRef> = todo.iter().map(|(cell, _)| *cell).collect();
            crate::merges::refuse_covered(node, &cells, "grid.flash.fill")?;
        }
        let mut written: Vec<serde_json::Value> = Vec::new();
        if let Some(node) = session.document_mut()?.node_mut(sheet) {
            let mut cells = crate::sheet_cells_mut(node);
            for (cell, from) in &todo {
                let Some(out) = program.apply(from) else { continue };
                cells.set_value(*cell, Value::Text(out.clone()));
                if let Some(format) = &format {
                    cells.set_format(*cell, Some(format.clone()));
                }
                written.push(json!({ "cell": cell.to_text(), "value": out }));
            }
            node.data["cells"] = crate::cells_into(&cells);
        }
        let count = written.len();
        Ok(Outcome::changed(format!("filled {count} cell(s) of the column"))
            .with_data(json!({ "sheet": sheet, "at": at.to_text(), "cells": written })))
    }
}

/// Whether a cell holds **nothing**.
///
/// A cell that holds a **formula** holds something: its value is
/// computed, not written, so an empty value is not an empty cell. A
/// flash fill that wrote over a formula would throw it away, and the
/// tree's rule — the row is *filled* — is about what the cell shows.
fn is_empty(cells: &Cells, cell: CellRef) -> bool {
    match cells.get(cell) {
        None => true,
        Some(entry) => {
            if entry.formula.is_some() {
                return false;
            }
            match &entry.value {
                None | Some(Value::Empty) => true,
                Some(_) => false,
            }
        }
    }
}

/// The **shown text** of a cell: its value with its format.
///
/// The format is the one the cell is **written** with, so a cell that
/// names a style is written with the style's format.
fn shown_text(
    session: &Session,
    sheet: u64,
    cells: &Cells,
    cell: CellRef,
    system: crate::dates::System,
) -> String {
    let value = cells.get(cell).and_then(|entry| entry.value.clone()).unwrap_or(Value::Empty);
    let format = crate::format_of(session, sheet, cells, cell);
    format.text(&value, system)
}

/// A **piece** of a program.
#[derive(Debug, Clone, PartialEq)]
enum Piece {
    /// Text that is written out as it is.
    Lit(String),
    /// A **token** of the source, by its index, with a case.
    Tok(i32, Case),
    /// The **first letter** of a token of the source, with a case.
    Initial(i32, Case),
}

/// The **case** a piece is written in.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Case {
    /// As it is.
    Same,
    /// In capitals.
    Upper,
    /// In small letters.
    Lower,
    /// In capitals, and the rest in small letters.
    Proper,
}

impl Case {
    /// The **cases** the tree tries, in its order.
    const ALL: [Case; 4] = [Case::Same, Case::Upper, Case::Lower, Case::Proper];
}

/// The **tokens** of a text: the words and the numbers in it.
///
/// The tree's rule: a token is a run of letters and digits, and an
/// apostrophe is part of the word it is in.
fn tokens(text: &str) -> Vec<String> {
    text.split(|character: char| !character.is_alphanumeric() && character != '\'')
        .filter(|token| !token.is_empty())
        .map(|token| token.to_string())
        .collect()
}

/// A piece of text, in a case.
fn apply_case(text: &str, case: Case) -> String {
    match case {
        Case::Same => text.to_string(),
        Case::Upper => text.to_uppercase(),
        Case::Lower => text.to_lowercase(),
        Case::Proper => {
            let mut out = String::new();
            for (index, character) in text.chars().enumerate() {
                if index == 0 {
                    out.extend(character.to_uppercase());
                } else {
                    out.extend(character.to_lowercase());
                }
            }
            out
        }
    }
}

/// A **program**: the pieces that turn a source into its target.
#[derive(Debug, Clone)]
struct Program(Vec<Piece>);

impl Program {
    /// What the program says about a source.
    fn apply(&self, source: &str) -> Option<String> {
        let tokens = tokens(source);
        let token = |index: i32| -> Option<&String> {
            if index >= 0 {
                tokens.get(index as usize)
            } else {
                tokens.len().checked_sub(index.unsigned_abs() as usize).and_then(|at| tokens.get(at))
            }
        };
        let mut out = String::new();
        for piece in &self.0 {
            match piece {
                Piece::Lit(text) => out.push_str(text),
                Piece::Tok(index, case) => out.push_str(&apply_case(token(*index)?, *case)),
                Piece::Initial(index, case) => {
                    out.push_str(&apply_case(&token(*index)?.chars().next()?.to_string(), *case))
                }
            }
        }
        Some(out)
    }
}

/// The program that explains **every** example.
///
/// The tree's rule: the candidates of the first example are tried, and
/// the first one that explains them all is the program.
fn learn(examples: &[(String, String)]) -> Option<Program> {
    let (source, target) = examples.first()?;
    synthesize(source, target)
        .into_iter()
        .find(|program| examples.iter().all(|(from, to)| program.apply(from).as_deref() == Some(to.as_str())))
}

/// The **candidates** for one example.
///
/// The tree's rule: the target is cut into pieces greedily — the
/// longest token that starts it, and the rest as text — and the whole
/// thing is tried twice, once with the tokens counted from the front
/// and once from the back.
fn synthesize(source: &str, target: &str) -> Vec<Program> {
    let tokens = tokens(source);
    let count = tokens.len() as i32;
    let mut programs = Vec::new();
    for prefer_negative in [false, true] {
        let mut pieces: Vec<Piece> = Vec::new();
        let mut rest = target;
        let mut guard = 0;
        while !rest.is_empty() && guard < 64 {
            guard += 1;
            // The longest token, in any case, that starts the rest.
            let mut best: Option<(usize, Piece)> = None;
            for (index, token) in tokens.iter().enumerate() {
                let index = if prefer_negative { index as i32 - count } else { index as i32 };
                for case in Case::ALL {
                    let candidate = apply_case(token, case);
                    if !candidate.is_empty()
                        && rest.starts_with(&candidate)
                        && best.as_ref().is_none_or(|(length, _)| candidate.len() > *length)
                    {
                        best = Some((candidate.len(), Piece::Tok(index, case)));
                    }
                }
            }
            // The **first letters**, when no token fits: "J. Smith".
            if best.as_ref().is_none_or(|(length, _)| *length <= 1) {
                for (index, token) in tokens.iter().enumerate() {
                    let index = if prefer_negative { index as i32 - count } else { index as i32 };
                    let Some(letter) = token.chars().next() else { continue };
                    for case in [Case::Same, Case::Upper, Case::Lower] {
                        let candidate = apply_case(&letter.to_string(), case);
                        if rest.starts_with(&candidate) && best.is_none() {
                            best = Some((candidate.len(), Piece::Initial(index, case)));
                        }
                    }
                }
            }
            match best {
                Some((length, piece)) => {
                    pieces.push(piece);
                    rest = rest.get(length..).unwrap_or("");
                }
                None => {
                    let character = rest.chars().next().unwrap_or(' ');
                    match pieces.last_mut() {
                        Some(Piece::Lit(text)) => text.push(character),
                        _ => pieces.push(Piece::Lit(character.to_string())),
                    }
                    rest = rest.get(character.len_utf8()..).unwrap_or("");
                }
            }
        }
        programs.push(Program(pieces));
    }
    programs
}
