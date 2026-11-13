//! The **names** of a workbook.
//!
//! A name is a **word** that stands for a range of cells: `Sales` for
//! the numbers in `Sheet1!A1:B4`. A formula may use the name where it
//! would use the range, and the range is what the formula reads — so a
//! name is a way of saying the same thing twice.
//!
//! The tree's rules for a name are the ones here:
//!
//! - it is at most **255 characters**;
//! - it begins with a **letter**, an underscore or a backslash;
//! - the rest of it is letters, digits, underscores, dots,
//!   backslashes and question marks;
//! - it does **not look like a cell address** (`A1`, `$A$1`), because
//!   a name that looks like a cell is a cell;
//! - it does **not look like an R1C1 reference** (`R1C1`), for the
//!   same reason.
//!
//! And two more, because this tree has them and the tree does not:
//!
//! - it is not `TRUE` or `FALSE`, because those are the constants and
//!   a name that is one of them could never be reached;
//! - it names a range in a sheet that **exists**, and the sheet is
//!   named in the range itself — so a name is never a name for "the
//!   sheet it was written on", and it never moves.
//!
//! A name is **workbook**-wide: every sheet of the workbook may use
//! it. The tree has a name on a sheet and a name on the workbook, and
//! the sheet's own name wins; here there is one kind of name and one
//! place it lives.
//!
//! # What the port found
//!
//! - **The tree keeps its names in a hidden stash of the file.** The
//!   tree's names are kept in the workbook's own list of defined
//!   names, and they are written out in the file's format. Here they
//!   are in the workbook's **payload**, which is where every other
//!   fact of a grid lives.
//! - **The tree renames a name in the formulas that use it.** The
//!   tree's `rename_defined_name` goes through every formula of every
//!   sheet and rewrites the word. Here a name is written with
//!   `grid.name.set` and taken away with `grid.name.delete`, and a
//!   name that is **renamed** — that is, taken away and written again
//!   under another word — leaves the formulas that used it reading a
//!   name that is not there. That is an error in the cell, and the
//!   error says which name is missing.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Text};

use crate::formula::{Anchor, Area};

/// The **longest** a name may be.
const MAX_NAME_CHARACTERS: usize = 255;

/// Whether a name may be **used**, and if it may not, why.
///
/// The tree's rule, and the two this tree adds.
pub fn why_not(name: &str) -> Option<String> {
    if name.is_empty() {
        return Some("a name is not nothing".to_string());
    }
    if name.chars().count() > MAX_NAME_CHARACTERS {
        return Some(format!("a name is at most {MAX_NAME_CHARACTERS} characters"));
    }
    let mut characters = name.chars();
    let first = characters.next().unwrap_or(' ');
    if !(first.is_alphabetic() || first == '_' || first == '\\') {
        return Some(format!(
            "a name begins with a letter, an underscore or a backslash, and `{name}` begins with `{first}`"
        ));
    }
    for character in characters {
        if !(character.is_alphanumeric()
            || character == '_'
            || character == '.'
            || character == '\\'
            || character == '?')
        {
            return Some(format!("`{character}` is not a character a name may hold"));
        }
    }
    if Anchor::parse(name).is_some() {
        return Some(format!("`{name}` looks like a cell, and a name that looks like a cell is a cell"));
    }
    if looks_like_r1c1(name) {
        return Some(format!("`{name}` looks like an R1C1 reference, and a name may not"));
    }
    if name.eq_ignore_ascii_case("true") || name.eq_ignore_ascii_case("false") {
        return Some(format!(
            "`{name}` is one of the constants, and a name that is one of them could never be reached"
        ));
    }
    None
}

/// Whether a word looks like an **R1C1 reference**: `R1C1`.
fn looks_like_r1c1(name: &str) -> bool {
    let Some(rest) = name.strip_prefix('R') else {
        return false;
    };
    let Some((row, column)) = rest.split_once('C') else {
        return false;
    };
    !row.is_empty()
        && !column.is_empty()
        && row.chars().all(|character| character.is_ascii_digit())
        && column.chars().all(|character| character.is_ascii_digit())
}

/// The **sheet** and the **area** a range names, if it names one.
///
/// The range names its sheet: `Sheet1!A1:B4` or `Sheet1!A1`.
pub fn parse_refers_to(text: &str) -> Result<(String, Area), String> {
    let trimmed = text.trim();
    let Some((sheet, range)) = trimmed.rsplit_once('!') else {
        return Err(format!(
            "a range names its sheet: `{trimmed}` does not name one, and `Sheet1!A1:B4` does"
        ));
    };
    let sheet = sheet.trim().to_string();
    if sheet.is_empty() {
        return Err(format!("`{trimmed}` does not name a sheet"));
    }
    let range = range.trim();
    let area = match range.split_once(':') {
        Some((first, second)) => {
            let Some(first) = Anchor::parse(first) else {
                return Err(format!("`{first}` is not a cell"));
            };
            let Some(second) = Anchor::parse(second) else {
                return Err(format!("`{second}` is not a cell"));
            };
            Area::new(first.cell, second.cell)
        }
        None => {
            let Some(only) = Anchor::parse(range) else {
                return Err(format!("`{range}` is not a cell"));
            };
            Area::new(only.cell, only.cell)
        }
    };
    Ok((sheet, area))
}

/// The **area** of a range, written back the way it is written.
fn area_to_text(area: &Area) -> String {
    if area.start == area.end {
        area.start.to_text()
    } else {
        format!("{}:{}", area.start.to_text(), area.end.to_text())
    }
}

commands! {
    /// Write a **name** into a workbook.
    ///
    /// The name stands for a range of cells in a sheet, and the range names
    /// its sheet: `Sheet1!A1:B4`.
    grid.name.set {
        area: "grid",
        layer: "grid",
        icon: "tag",
        mutates,
        undoable,
        params: {
            /// The workbook.
            workbook: NodeRef,
            /// The name.
            name: Text,
            /// The range it stands for: `Sheet1!A1:B4`.
            refers_to: Text,
        },
        run: {
            let id = "grid.name.set".to_string();
            let workbook = self.workbook.0;
            let Some(node) = session.document().node(workbook) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{workbook}`"),
                });
            };
            if node.kind != "grid.workbook" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{workbook}` is a `{}` and not a workbook", node.kind),
                });
            }
            let name = self.name.0.trim().to_string();
            if let Some(reason) = why_not(&name) {
                return Err(EngineError::Failed { id: id.clone(), message: reason });
            }
            let (sheet, area) = parse_refers_to(&self.refers_to.0)
                .map_err(|message| EngineError::Failed { id: id.clone(), message })?;
            let sheets: Vec<String> = session
                .document()
                .walk()
                .filter(|node| node.kind == "grid.sheet")
                .map(|node| node.name.clone())
                .collect();
            if !sheets.contains(&sheet) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`: the sheets are {}", sheets.join(", ")),
                });
            }
            let refers_to = format!("{sheet}!{}", area_to_text(&area));
            let mut names = node.data.get("names").cloned().unwrap_or(json!({}));
            if !names.is_object() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the names of the workbook `{workbook}` are not a group"),
                });
            }
            let replacing = names.get(&name).is_some();
            names[&name] = json!(refers_to.clone());
            if let Some(node) = session.document_mut()?.node_mut(workbook) {
                node.data["names"] = names;
            }
            Ok(Outcome::changed(if replacing {
                format!("the name `{name}` now stands for {refers_to}")
            } else {
                format!("the name `{name}` stands for {refers_to}")
            })
            .with_data(json!({ "workbook": workbook, "name": name, "refers_to": refers_to })))

        }
    }

    /// Take a **name** away from a workbook.
    grid.name.delete {
        area: "grid",
        layer: "grid",
        icon: "trash",
        mutates,
        undoable,
        params: {
            /// The workbook.
            workbook: NodeRef,
            /// The name.
            name: Text,
        },
        run: {
            let id = "grid.name.delete".to_string();
            let workbook = self.workbook.0;
            let Some(node) = session.document().node(workbook) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{workbook}`"),
                });
            };
            if node.kind != "grid.workbook" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{workbook}` is a `{}` and not a workbook", node.kind),
                });
            }
            let name = self.name.0.trim().to_string();
            let mut names = node.data.get("names").cloned().unwrap_or(json!({}));
            if names.get(&name).is_none() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no name `{name}` in the workbook `{workbook}`"),
                });
            }
            if let Some(names) = names.as_object_mut() {
                names.remove(&name);
            }
            if let Some(node) = session.document_mut()?.node_mut(workbook) {
                node.data["names"] = names;
            }
            Ok(Outcome::changed(format!("the name `{name}` is taken away"))
                .with_data(json!({ "workbook": workbook, "name": name })))

        }
    }
}
