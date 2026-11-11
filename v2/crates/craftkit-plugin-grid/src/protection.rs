//! The **protection** of a sheet.
//!
//! A sheet may be **protected**. Then two things are true of it:
//!
//! - its **cells are locked**: a command that changes what a cell
//!   holds — its value or its formula — refuses;
//! - the things that may still be done to it are the ones the
//!   protection **says**: the sorting, the filtering and the
//!   formatting. A command that is not allowed says so and does
//!   nothing.
//!
//! The tree's protection says the same things — the tree's protection
//! carries a flag for the sorting, the filtering, the formatting, the
//! inserting, the deleting and the selecting — and the tree's cells
//! are locked by their own style, which is locked unless the style
//! says otherwise.
//!
//! The tree's protection may carry a **password**, and it stores the
//! hash of it and never the password. Here there is no password: a
//! sheet may be unprotected by anyone who may change it, and that is
//! what the tree's protection is when it has no password.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Bool, EngineError, Node, NodeRef, Outcome, Session, Text};

use crate::formula::CellRef;

/// The **protection** of a sheet: what may be done to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Protection {
    /// Whether the **sorting** is allowed.
    pub sort: bool,
    /// Whether the **filtering** is allowed.
    pub filter: bool,
    /// Whether the **formatting** is allowed.
    pub format: bool,
    /// Whether the **cells** are locked.
    pub locked: bool,
}

impl Protection {
    /// The protection of a sheet, if it has one.
    pub fn of(node: &Node) -> Option<Protection> {
        let protection = node.data.get("protection")?;
        if protection.is_null() {
            return None;
        }
        let flag = |name: &str| protection.get(name).and_then(|value| value.as_bool()).unwrap_or(false);
        Some(Protection { sort: flag("sort"), filter: flag("filter"), format: flag("format"), locked: true })
    }

    /// The protection, as it is kept.
    pub fn to_json(self) -> craftkit_sdk::serde_json::Value {
        json!({
            "sort": self.sort,
            "filter": self.filter,
            "format": self.format,
            "locked": self.locked,
        })
    }
}

/// The message of a refusal: what is not allowed, and how to allow it.
fn refused(what: &str, id: &str) -> EngineError {
    EngineError::Failed {
        id: id.to_string(),
        message: format!(
            "{what} is not allowed on this protected sheet: unprotect it with grid.sheet.unprotect"
        ),
    }
}

/// The **edit ranges** of a sheet: the ranges that stay editable on a
/// protected sheet.
pub fn edit_ranges_of(node: &Node) -> Vec<crate::formula::Area> {
    let mut areas = Vec::new();
    let Some(list) = node.data.get("edit_ranges").and_then(|ranges| ranges.as_array()) else {
        return areas;
    };
    for range in list {
        let Some(text) = range.as_str() else { continue };
        let Some((_, area)) = crate::merges::parse_area(text) else { continue };
        areas.push(area);
    }
    areas
}

/// The **lock** of a cell: what it says of itself, and then what the
/// style it names says.
///
/// The cell's own word comes first: a cell that says of itself that
/// it is unlocked says more than the style it names. The style's word
/// comes next: the tree's cell is locked by its style. And a cell that
/// says nothing and names a style that says nothing is locked by the
/// sheet.
pub fn lock_of(node: &Node, locks: &[(String, Option<bool>)], cell: CellRef) -> Option<bool> {
    let entry = node.data.get("cells").and_then(|cells| cells.get(cell.to_text()));
    if let Some(locked) = entry.and_then(|entry| entry.get("locked")).and_then(|locked| locked.as_bool()) {
        return Some(locked);
    }
    let name = entry.and_then(|entry| entry.get("style")).and_then(|style| style.as_str())?;
    locks.iter().find(|(style, _)| style.eq_ignore_ascii_case(name)).and_then(|(_, locked)| *locked)
}

/// Refuse a change to the **contents** of a cell of a locked sheet.
///
/// A sheet that is not protected, or a protection that does not lock
/// the cells, says nothing.
///
/// A cell may be **unlocked of its own**, and then it may be changed
/// even though the sheet is protected: the tree's own rule, and the
/// tree's cells are locked by their own style — and here the style may
/// say so too.
pub fn refuse_locked(
    node: &Node,
    locks: &[(String, Option<bool>)],
    cells: &[CellRef],
    id: &str,
) -> Result<(), EngineError> {
    let Some(protection) = Protection::of(node) else { return Ok(()) };
    if !protection.locked {
        return Ok(());
    }
    // A cell in an **edit range** may be changed even though the sheet
    // is protected: the tree's own rule.
    let editable = |cell: CellRef| edit_ranges_of(node).iter().any(|area| area.covers(cell));
    let unlocked = |cell: CellRef| lock_of(node, locks, cell) == Some(false);
    let cell = match cells.iter().find(|cell| !unlocked(**cell) && !editable(**cell)) {
        Some(cell) => cell.to_text(),
        None => return Ok(()),
    };
    Err(EngineError::Failed {
        id: id.to_string(),
        message: format!(
            "the cell {cell} is locked: the sheet is protected, and a protected sheet's cells may not be changed"
        ),
    })
}

/// Refuse the **sorting** of a protected sheet that does not allow it.
pub fn refuse_sort(node: &Node, id: &str) -> Result<(), EngineError> {
    match Protection::of(node) {
        Some(protection) if !protection.sort => Err(refused("sorting", id)),
        _ => Ok(()),
    }
}

/// Refuse the **filtering** of a protected sheet that does not allow it.
pub fn refuse_filter(node: &Node, id: &str) -> Result<(), EngineError> {
    match Protection::of(node) {
        Some(protection) if !protection.filter => Err(refused("filtering", id)),
        _ => Ok(()),
    }
}

/// Refuse the **formatting** of a protected sheet that does not allow it.
pub fn refuse_format(node: &Node, id: &str) -> Result<(), EngineError> {
    match Protection::of(node) {
        Some(protection) if !protection.format => Err(refused("formatting", id)),
        _ => Ok(()),
    }
}

/// **Protect** a sheet.
///
/// The protection says what may still be done to the sheet: the
/// sorting, the filtering and the formatting. What is not said is not
/// allowed, and the cells of the sheet are locked.
#[derive(Command)]
#[command(id = "grid.sheet.protect", area = "grid", layer = "grid", icon = "lock", mutates, undoable)]
pub struct ProtectSheet {
    /// The sheet to protect.
    pub sheet: NodeRef,
    /// Whether the **sorting** is still allowed.
    pub sort: Option<Bool>,
    /// Whether the **filtering** is still allowed.
    pub filter: Option<Bool>,
    /// Whether the **formatting** is still allowed.
    pub format: Option<Bool>,
}

impl ProtectSheet {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.sheet.protect".to_string();
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
        let protection = Protection {
            sort: self.sort.map(|flag| flag.0).unwrap_or(false),
            filter: self.filter.map(|flag| flag.0).unwrap_or(false),
            format: self.format.map(|flag| flag.0).unwrap_or(false),
            locked: true,
        };
        let Some(node) = session.document_mut()?.node_mut(sheet) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no sheet `{sheet}`"),
            });
        };
        node.data["protection"] = protection.to_json();
        Ok(Outcome::changed(format!("protected the sheet `{}`", node.name)).with_data(json!({
            "sheet": sheet,
            "protection": protection.to_json(),
        })))
    }
}

/// **Unprotect** a sheet: the protection is gone.
#[derive(Command)]
#[command(id = "grid.sheet.unprotect", area = "grid", layer = "grid", icon = "unlock", mutates, undoable)]
pub struct UnprotectSheet {
    /// The sheet to unprotect.
    pub sheet: NodeRef,
}

impl UnprotectSheet {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.sheet.unprotect".to_string();
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
        };
        let was_there = node.data.get("protection").is_some();
        let Some(node) = session.document_mut()?.node_mut(sheet) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no sheet `{sheet}`"),
            });
        };
        node.data["protection"] = craftkit_sdk::serde_json::Value::Null;
        Ok(Outcome::changed(format!("unprotected the sheet `{}`", node.name)).with_data(json!({
            "sheet": sheet,
            "was_there": was_there,
        })))
    }
}

/// **Lock** a cell, or let it be changed.
///
/// The tree's Lock Cell: a button, and a button pressed twice does the
/// opposite.
///
/// A lock matters only when the sheet is **protected**: a cell that is
/// unlocked may be changed even then, and a cell that is locked may
/// not. A cell that says nothing follows the sheet.
#[derive(Command)]
#[command(id = "grid.cell.lock", area = "grid", layer = "grid", icon = "lock", mutates, undoable)]
pub struct LockCell {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// The cell's address.
    pub cell: Text,
    /// Whether the cell is locked: the opposite of what it is when
    /// this is not named.
    pub locked: Option<Bool>,
}

impl LockCell {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.cell.lock".to_string();
        let sheet = self.sheet.0;
        let cell = crate::cell_at(&self.cell.0, &id)?;
        let Some(node) = session.document_mut()?.node_mut(sheet) else {
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
        crate::protection::refuse_format(node, &id)?;
        let cells = crate::cells_of(node);
        let now = cells.locked(cell);
        let locked = match self.locked {
            Some(locked) => locked.0,
            // A button pressed twice does the opposite: the tree's own
            // rule.
            None => now == Some(false),
        };
        let mut cells = crate::sheet_cells_mut(node);
        cells.set_locked(cell, Some(locked));
        node.data["cells"] = crate::cells_into(&cells);
        Ok(Outcome::changed(format!(
            "the cell {} is {}",
            cell.to_text(),
            if locked { "locked" } else { "unlocked" }
        ))
        .with_data(json!({
            "sheet": sheet,
            "cell": cell.to_text(),
            "locked": locked,
        })))
    }
}

/// The most **edit ranges** a sheet may hold.
const MAX_EDIT_RANGES: usize = 64;

/// **Set** the **edit ranges** of a sheet: the ranges that stay
/// editable while the sheet is protected.
///
/// The tree's Allow Edit Ranges: the ranges may only be set while the
/// sheet is **not protected** — the tree's own rule.
#[derive(Command)]
#[command(id = "grid.sheet.edit.set", area = "grid", layer = "grid", icon = "lock", mutates, undoable)]
pub struct SetEditRanges {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// The **ranges** that stay editable: `["A1:B2", "D4"]`.
    pub ranges: craftkit_sdk::serde_json::Value,
}

impl SetEditRanges {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.sheet.edit.set".to_string();
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
        // The ranges may only be set while the sheet is not protected:
        // the tree's own rule.
        if Protection::of(node).is_some() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "the sheet `{sheet}` is protected: unprotect it before changing its edit ranges"
                ),
            });
        }
        let Some(given) = self.ranges.as_array() else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "the edit ranges of a sheet are a list of ranges".to_string(),
            });
        };
        if given.len() > MAX_EDIT_RANGES {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("a sheet holds {MAX_EDIT_RANGES} edit ranges or fewer"),
            });
        }
        let mut list: Vec<String> = Vec::with_capacity(given.len());
        for range in given {
            let Some(text) = range.as_str() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "an edit range is a range of cells".to_string(),
                });
            };
            if crate::merges::parse_area(text).is_none() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{text}` is not a range of cells"),
                });
            }
            list.push(text.trim().to_string());
        }
        if let Some(node) = session.document_mut()?.node_mut(sheet) {
            node.data["edit_ranges"] = json!(list);
        }
        Ok(Outcome::changed(format!("the sheet `{sheet}` has {} edit range(s)", list.len())).with_data(
            json!({
                "sheet": sheet,
                "ranges": list,
            }),
        ))
    }
}

/// **Read** the **edit ranges** of a sheet: the ranges that stay
/// editable while the sheet is protected.
#[derive(Command)]
#[command(id = "grid.sheet.edit.read", area = "grid", layer = "grid", icon = "lock")]
pub struct ReadEditRanges {
    /// The sheet it is in.
    pub sheet: NodeRef,
}

impl ReadEditRanges {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.sheet.edit.read".to_string();
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
        let ranges: Vec<String> = edit_ranges_of(node)
            .iter()
            .map(|area| format!("{}:{}", area.start.to_text(), area.end.to_text()))
            .collect();
        Ok(Outcome::new(format!("the sheet `{sheet}` has {} edit range(s)", ranges.len())).with_data(json!({
            "sheet": sheet,
            "ranges": ranges,
        })))
    }
}
