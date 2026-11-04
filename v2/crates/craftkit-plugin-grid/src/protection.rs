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
use craftkit_sdk::{Bool, EngineError, Node, NodeRef, Outcome, Session};

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

/// Refuse a change to the **contents** of a cell of a locked sheet.
///
/// A sheet that is not protected, or a protection that does not lock
/// the cells, says nothing.
pub fn refuse_locked(node: &Node, cells: &[CellRef], id: &str) -> Result<(), EngineError> {
    let Some(protection) = Protection::of(node) else { return Ok(()) };
    if !protection.locked {
        return Ok(());
    }
    let cell = match cells.first() {
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
