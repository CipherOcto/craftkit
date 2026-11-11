//! **Reading** a cell.
//!
//! A cell may be read whole: the answer is everything the cell holds
//! and everything that is said about it.
//!
//! - the **value** it holds, and the **type** of that value — and a
//!   cell that holds a formula has the value its formula computes,
//!   because the value is **derived**;
//! - the **formula** it holds, if it holds one;
//! - the **text** it shows — the value written with the format of the
//!   cell and with the conditional format that matches it;
//! - the **format** and the **style** it carries;
//! - the **look** of the cell: its alignment, its font, its fill and
//!   the lock it is under — the tree's cell carries all of them in
//!   its style, and here the cell carries them itself;
//! - the **note** and the **link** that are on it;
//! - the **merge** that covers it, if it is covered by one.
//!
//! The tree's `cell.get` says the same things, and says more: the
//! tree's cell may hold a **picture**, and the tree's cell may be
//! part of a **spilled** range.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session, Text};

use crate::formula::Value;

/// The **type** of a value, as the tree says it.
fn kind_of(value: &Value) -> &'static str {
    match value {
        Value::Empty => "empty",
        Value::Number(_) => "number",
        Value::Text(_) => "text",
        Value::Bool(_) => "boolean",
        Value::Error(_) => "error",
    }
}

/// **Read** a cell whole.
#[derive(Command)]
#[command(id = "grid.cell.get", area = "grid", layer = "grid", icon = "list")]
pub struct ReadCell {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// The cell's address.
    pub cell: Text,
}

impl ReadCell {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.cell.get".to_string();
        let sheet = self.sheet.0;
        let cell = crate::cell_at(&self.cell.0, &id)?;
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
        let name = node.name.clone();
        let cells = crate::cells_of(node);
        let entry = cells.get(cell).cloned();
        // The **value**: the one the cell holds, or the one its formula
        // computes.
        let value = match &entry {
            Some(entry) => match (&entry.value, &entry.formula) {
                (Some(value), _) => value.clone(),
                (None, Some(_)) => crate::book_of(session.document()).value_of(&name, cell),
                (None, None) => Value::Empty,
            },
            None => Value::Empty,
        };
        // The **look** the conditional formats of the sheet give the
        // cell: the format and the fill, each of them from the rule
        // that is seen first and says something about it.
        let looks = crate::conditional::looks_for(node, cell, &value);
        // The **text** the cell shows: the value written with the
        // format of the cell — the conditional format that matches it
        // first, and the format of the style it names otherwise.
        let format = match &looks.format {
            Some(code) => crate::format::Format::parse(code).unwrap_or(crate::format::Format::General),
            None => crate::format_of(session, sheet, &cells, cell),
        };
        let text = format.text(&value, crate::system_of(session, sheet));
        // The **note** and the **link** that are on the cell.
        let notes = crate::notes::notes_of(node);
        let links = crate::links::links_of(node);
        let address = cell.to_text();
        let note = notes.get(&address);
        let link = links.get(&address);
        // The **merge** that covers the cell, if it is covered by one.
        let merges = crate::merges::merges_of(node);
        let merge = crate::merges::covering(&merges, cell);
        // The **look** of the cell: its alignment, its font and its
        // fill, and the lock it is under. The lock is the cell's own
        // word, then the style it names, and then the sheet; and the
        // alignment and the font are the cell's own, and then the
        // style's.
        let locks = crate::style_locks(session, sheet);
        let locked = crate::protection::is_locked(node, &locks, cell);
        let styles = crate::styles_of(session, sheet);
        let style = entry
            .as_ref()
            .and_then(|entry| entry.style.as_deref())
            .and_then(|name| styles.iter().find(|style| style.name == name));
        Ok(Outcome::new(format!("{name}!{address} holds {}", kind_of(&value))).with_data(json!({
            "sheet": sheet,
            "cell": address,
            "value": value.to_json(),
            "type": kind_of(&value),
            "formula": entry.as_ref().and_then(|entry| entry.formula.clone()),
            "text": text,
            "format": entry.as_ref().and_then(|entry| entry.format.clone()),
            "style": entry.as_ref().and_then(|entry| entry.style.clone()),
            "note": match note {
                Some(note) => json!({ "text": note.text, "author": note.author }),
                None => Json::Null,
            },
            "link": match link {
                Some(link) => json!({ "target": link.target, "tooltip": link.tooltip }),
                None => Json::Null,
            },
            "merge": match merge {
                Some(area) => json!(area.to_text()),
                None => Json::Null,
            },
            "align": match entry.as_ref().and_then(|entry| entry.align) {
                Some(align) => align.to_json(),
                None => match style.and_then(|style| style.align) {
                    Some(align) => align.to_json(),
                    None => Json::Null,
                },
            },
            "font": match entry.as_ref().and_then(|entry| entry.font.as_ref()) {
                Some(font) => font.to_json(),
                None => match style.and_then(|style| style.font.as_ref()) {
                    Some(font) => font.to_json(),
                    None => Json::Null,
                },
            },
            // The **fill**: the rule's fill is on top of the cell's
            // own, as the rule is on top of the cell.
            "fill": match looks.fill.clone().or_else(|| entry.as_ref().and_then(|entry| entry.fill.clone())) {
                Some(fill) => json!(fill),
                None => Json::Null,
            },
            "locked": locked,
        })))
    }
}
