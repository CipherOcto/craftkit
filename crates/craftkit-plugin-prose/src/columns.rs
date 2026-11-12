//! The **columns** of a page.
//!
//! A page may be printed in **columns**: two columns side by side, or
//! three, or twelve. Between them there is a **space**, and there may
//! be a **line** drawn down the middle.
//!
//! # The rules
//!
//! - The **count** of the columns is from **one** to **twelve**. The
//!   tree refuses a count outside that range, and so does this tree.
//! - The **space** between the columns is in **points**, from **zero**
//!   to **three hundred**. The tree clamps a space to that range and
//!   says nothing; here a space outside it is refused.
//! - The **separator** is a line drawn between the columns.
//! - The **width of a column** is **derived**: the room the text has,
//!   less the spaces between the columns, shared out. Columns that do
//!   not fit on the page are refused, and the page is left as it was.
//! - The columns are a **fact of the document**, like the borders and
//!   the background, and they are read with the page.
//!
//! # What the port found
//!
//! - **The tree's columns belong to a section, and this tree has no
//!   sections.** The tree's command asks where the columns go:
//!   `section`, `document`, `forward` or `selection` — and a
//!   selection of text becomes a section of its own, with a
//!   continuous break at each end. Here the columns are a fact of the
//!   document, and there is nowhere else for them to go.
//! - **The tree clamps the space and this tree refuses it.** The
//!   tree's `layout.pageSetup` says that a gap is clamped to
//!   zero–three hundred points and the count to one–twelve; the
//!   `layout.columns` command itself refuses a count outside one to
//!   twelve and clamps the space. Here both are refused by name.
//! - **The tree's default space is thirty-six points** — half an inch
//!   — and that is the space a page has here when it is not given
//!   one.
//! - **The width of a column is derived in both trees, but the tree
//!   also lets it be given.** The tree's dialog takes the widths of
//!   unequal columns, and the tree's presets — a wide column and a
//!   narrow one — make two columns that are not alike. Here every
//!   column is the same width, and that width is worked out from the
//!   page and the space.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, Node, NodeRef, Number, Outcome, Session};

/// The most columns a page may have.
pub const MAX_COLUMNS: u64 = 12;

/// The **space** between the columns, in points, when it is not given.
pub const DEFAULT_SPACE: f64 = 36.0;

/// The most **space** there may be between the columns, in points.
pub const MAX_SPACE: f64 = 300.0;

/// The **columns** of a page, as they are kept.
pub fn stored_of(node: &Node) -> Option<Json> {
    node.data.get("page_columns").and_then(|columns| columns.as_object()).cloned().map(Json::Object)
}

/// The **columns** of a page, as they are read: the count, the space,
/// the separator, and the **width** of a column, which is derived.
pub fn columns_of(node: &Node) -> Json {
    let stored = stored_of(node);
    let count = stored
        .as_ref()
        .and_then(|columns| columns.get("count"))
        .and_then(|count| count.as_u64())
        .unwrap_or(1);
    let space = stored
        .as_ref()
        .and_then(|columns| columns.get("space"))
        .and_then(|space| space.as_f64())
        .filter(|space| space.is_finite())
        .unwrap_or(DEFAULT_SPACE);
    let separator = stored
        .as_ref()
        .and_then(|columns| columns.get("separator"))
        .and_then(|separator| separator.as_bool())
        .unwrap_or(false);
    let width = width_of(node, count, space);
    json!({
        "count": count,
        "space": space,
        "separator": separator,
        "width": width,
    })
}

/// The **width of a column**, in points, or nothing when the columns
/// do not fit on the page.
fn width_of(node: &Node, count: u64, space: f64) -> Json {
    let page = crate::page::Page::of(node);
    let room = page.width - page.left - page.right;
    let gaps = (count.saturating_sub(1)) as f64 * space;
    let width = (room - gaps) / count as f64;
    if !width.is_finite() || width <= 0.0 {
        return Json::Null;
    }
    json!(width)
}

/// The **document** the columns belong to.
fn the_document(session: &Session, command: &str, document: u64) -> Result<(), EngineError> {
    let Some(node) = session.document().node(document) else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("there is no document `{document}`"),
        });
    };
    if node.kind != "prose.document" {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the node `{document}` is a `{}` and not a document", node.kind),
        });
    }
    Ok(())
}

/// The **space** between the columns, in points.
fn space_of(value: f64, id: &str) -> Result<f64, EngineError> {
    if !value.is_finite() {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("a space of points is a number that is there: {value}"),
        });
    }
    if !(0.0..=MAX_SPACE).contains(&value) {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("a space is from zero to {MAX_SPACE} points, and {value} is not"),
        });
    }
    Ok(value)
}

/// **Set** the **columns** of the page of a document.
///
/// The count is from one to twelve, the space is in points, and the
/// separator is a line between the columns.
#[derive(Command)]
#[command(id = "prose.page.columns", area = "prose", layer = "prose", icon = "rect", mutates, undoable)]
pub struct SetColumns {
    /// The document.
    pub document: NodeRef,
    /// How many columns the page has, from one to twelve.
    pub count: Number,
    /// The space between the columns, in points.
    ///
    /// It is thirty-six points when it is not given.
    pub space: Option<Number>,
    /// Whether a line is drawn between the columns.
    pub separator: Option<Bool>,
}

impl SetColumns {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.page.columns".to_string();
        let document = self.document.0;
        the_document(session, &id, document)?;
        let count = self.count.0;
        if !count.is_finite() || count.fract() != 0.0 || count < 1.0 || count > MAX_COLUMNS as f64 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "a page has from one to {MAX_COLUMNS} columns, and {count} is not one of them"
                ),
            });
        }
        let count = count as u64;
        let Some(node) = session.document().node(document) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no document `{document}`"),
            });
        };
        let held = stored_of(node);
        let space = match self.space {
            Some(space) => space_of(space.0, &id)?,
            None => held
                .as_ref()
                .and_then(|columns| columns.get("space"))
                .and_then(|space| space.as_f64())
                .filter(|space| space.is_finite())
                .unwrap_or(DEFAULT_SPACE),
        };
        let separator = match self.separator {
            Some(separator) => separator.0,
            None => held
                .as_ref()
                .and_then(|columns| columns.get("separator"))
                .and_then(|separator| separator.as_bool())
                .unwrap_or(false),
        };
        // The columns must **fit**: a column that is not wide enough is
        // refused, and the page is left as it was.
        if width_of(node, count, space).is_null() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("{count} columns with a space of {space} points do not fit on the page"),
            });
        }
        let columns = json!({
            "count": count,
            "space": space,
            "separator": separator,
        });
        let Some(node) = session.document_mut()?.node_mut(document) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no document `{document}`"),
            });
        };
        node.data["page_columns"] = columns.clone();
        Ok(Outcome::changed(format!("the page of the document `{document}` has {count} columns"))
            .with_data(json!({ "document": document, "columns": columns })))
    }
}
