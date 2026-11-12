//! The **name** of a sheet.
//!
//! A sheet's name is not a label: it is the way a formula reaches it.
//! `Sheet1!A1` names the sheet `Sheet1`, and a name that is taken is a
//! name that means nothing.
//!
//! So a name has **rules**, and they are the tree's rules:
//!
//! - it is not **blank**;
//! - it is at most **31 characters**;
//! - it holds none of `:`, `\`, `/`, `?`, `*`, `[` or `]`;
//! - it does not **begin or end** with an apostrophe;
//! - it is **not taken** by another sheet.
//!
//! And the tree's name is **case-insensitive**: two sheets whose names
//! differ only in case are one name. Here the sheets of a **workbook**
//! are gathered into one book by their names, so a name is unique in
//! the workbook and not only in its own sheet — and the uniqueness is
//! the same, case and all.
//!
//! And **renaming** a sheet is not only renaming it: every formula
//! that names it follows, and every **name** that stands for a range in
//! it follows. A rename that left them behind would leave the
//! document reading a sheet that is not there.
//!
//! # What the port found
//!
//! - **The tree renames a sheet by rewriting the formulas.** The tree
//!   keeps a formula as a parsed expression and prints it again, so
//!   renaming is a change of the expression. Here a formula is **text**
//!   — the text the caller wrote — so renaming parses the text, names
//!   the sheet again and prints it. A formula that does not name the
//!   sheet is left exactly as it was: only the formulas that follow are
//!   printed again, and a printed formula is the printer's spelling of
//!   it (`A1 + A2`, where the caller wrote `A1+A2`).
//! - **The tree's sheet-qualified names are not here.** The tree's
//!   `Old!MyName` is one string and the printer adds the quotes; here a
//!   name stands for a range and the range names its sheet, so a name
//!   is renamed by renaming the **sheet** in the range and not the
//!   name itself.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session, Text};

/// The **longest** a sheet's name may be.
///
/// The tree's rule, and the bound of the format the tree writes.
const MAX_SHEET_NAME_CHARACTERS: usize = 31;

/// The characters a sheet's name may **not** hold.
///
/// The tree's list, and the reason they are not there: they are the
/// characters a formula writes a reference with.
const FORBIDDEN: [char; 7] = [':', '\\', '/', '?', '*', '[', ']'];

/// Whether a sheet's **name** may be used, and if it may not, why.
///
/// `taken` is the name of another sheet, if there is one: a name that
/// is taken is refused, and the tree's rule is that the comparison
/// **ignores case**.
pub fn why_not(name: &str, taken: Option<&str>) -> Option<String> {
    if name.is_empty() {
        return Some("a sheet has a name".to_string());
    }
    let count = name.chars().count();
    if count > MAX_SHEET_NAME_CHARACTERS {
        return Some(format!("a sheet's name is at most {MAX_SHEET_NAME_CHARACTERS} characters"));
    }
    if let Some(character) = name.chars().find(|character| FORBIDDEN.contains(character)) {
        return Some(format!("a sheet's name may not hold `{character}`"));
    }
    if name.starts_with('\'') || name.ends_with('\'') {
        return Some("a sheet's name may not begin or end with an apostrophe".to_string());
    }
    if let Some(taken) = taken
        && taken.eq_ignore_ascii_case(name)
    {
        return Some(format!("the sheet `{taken}` is already there"));
    }
    None
}

/// The names of the **other** sheets of the sheet's workbook, by name.
///
/// The uniqueness of a name is the tree's: it is unique **in the
/// workbook**, and two workbooks may hold sheets of one name.
fn other_sheet_names(session: &Session, sheet: u64) -> Vec<String> {
    let Some(node) = session.document().node(sheet) else { return Vec::new() };
    let Some(workbook) =
        session.document().walk().find(|parent| parent.children.iter().any(|child| child.id == sheet))
    else {
        return Vec::new();
    };
    workbook
        .children
        .iter()
        .filter(|child| child.kind == "grid.sheet" && child.id != node.id)
        .map(|child| child.name.clone())
        .collect()
}

/// The name of another sheet that **takes** this one, if one does.
fn taken_by(names: &[String], name: &str) -> Option<String> {
    names.iter().find(|other| other.eq_ignore_ascii_case(name)).cloned()
}

commands! {
    /// **Rename** a sheet.
    ///
    /// Every formula that names the sheet follows it, and every name of
    /// the workbook that stands for a range in it follows it too.
    grid.sheet.rename {
        area: "grid",
        layer: "grid",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The sheet to rename.

            sheet: NodeRef,
            /// Its new name.

            name: Text,
        },
        run: {

            let id = "grid.sheet.rename".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{sheet}`"),
                });
            };
            if node.kind != "grid.sheet" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{sheet}` is a `{}` and not a sheet", node.kind),
                });
            }
            let old = node.name.clone();
            let name = self.name.0.trim().to_string();
            let names = other_sheet_names(session, sheet);
            if let Some(reason) = why_not(&name, taken_by(&names, &name).as_deref()) {
                return Err(EngineError::Failed { id: id.clone(), message: reason });
            }
            // The formulas of every sheet of the document, and the names of
            // every workbook: they name the sheet, and they follow it.
            let mut renamed = 0usize;
            let sheets: Vec<u64> =
                session.document().walk().filter(|node| node.kind == "grid.sheet").map(|node| node.id).collect();
            for other in sheets {
                let Some(node) = session.document_mut()?.node_mut(other) else { continue };
                let mut cells = crate::cells_of(node);
                let changed = cells.rename_sheet(&old, &name);
                if changed > 0 {
                    renamed += changed;
                    node.data["cells"] = crate::cells_into(&cells);
                }
            }
            // The names of the workbook: a name stands for a range, and the
            // range names its sheet.
            let workbooks: Vec<u64> = session
                .document()
                .walk()
                .filter(|node| node.kind == "grid.workbook")
                .map(|node| node.id)
                .collect();
            let mut names_moved = 0usize;
            for workbook in workbooks {
                let Some(node) = session.document_mut()?.node_mut(workbook) else { continue };
                let Some(map) = node.data.get("names").and_then(|names| names.as_object()).cloned() else {
                    continue;
                };
                let mut changed = false;
                let mut moved = craftkit_sdk::serde_json::Map::new();
                for (name_of_name, refers_to) in map {
                    let Some(text) = refers_to.as_str() else {
                        moved.insert(name_of_name, refers_to);
                        continue;
                    };
                    let rewritten = rename_in_refers_to(text, &old, &name);
                    if rewritten != text {
                        changed = true;
                        names_moved += 1;
                    }
                    moved.insert(name_of_name, json!(rewritten));
                }
                if changed {
                    node.data["names"] = json!(moved);
                }
            }
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.name = name.clone();
            }
            Ok(Outcome::changed(format!("the sheet `{old}` is now called `{name}`")).with_data(json!({
                "sheet": sheet,
                "old_name": old,
                "name": name,
                "formulas": renamed,
                "names": names_moved,
            })))

        }
    }
}

/// The range of a **name** with the sheet renamed in it.
fn rename_in_refers_to(text: &str, old: &str, new: &str) -> String {
    let Some((sheet, range)) = text.rsplit_once('!') else { return text.to_string() };
    if !sheet.trim().eq_ignore_ascii_case(old) {
        return text.to_string();
    }
    format!("{new}!{range}")
}
