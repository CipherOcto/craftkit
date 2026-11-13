//! The **link** on a cell.
//!
//! A cell may carry a **link**: a target that something else may open.
//! The target may be
//!
//! - a **place in this workbook** — `Sheet2!A1` — and then the link is
//!   **inside**;
//! - a **place outside** — a web address, a `mailto:`, a file — and
//!   then the link is **outside**.
//!
//! And the link may carry a **tooltip**: the text that is shown when
//! the mouse rests on it.
//!
//! The link belongs to the **sheet** and names a cell, as the tree's
//! hyperlink does and for the tree's reason: a cell in the tree is a
//! value and a formula and a style, and a link is none of those three.
//! So a link **survives** the cell being written to, and it goes only
//! when the link is taken away or when the **whole cell** is cleared.

use std::collections::BTreeMap;

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Outcome, Session, Text};

/// A **link**: where it goes, and what it says on the way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    /// The target: a place in this workbook, or a place outside it.
    pub target: String,
    /// The text that is shown when the mouse rests on it.
    pub tooltip: Option<String>,
}

/// The **links** of a sheet: by the address of the cell they are on.
pub fn links_of(node: &Node) -> BTreeMap<String, Link> {
    let mut links = BTreeMap::new();
    let Some(map) = node.data.get("links").and_then(|links| links.as_object()) else {
        return links;
    };
    for (address, link) in map {
        let Some(target) = link.get("target").and_then(|target| target.as_str()) else { continue };
        let tooltip = link.get("tooltip").and_then(|tooltip| tooltip.as_str()).map(|text| text.to_string());
        links.insert(address.to_string(), Link { target: target.to_string(), tooltip });
    }
    links
}

/// The links of a sheet, as they are kept.
pub fn links_into(links: &BTreeMap<String, Link>) -> Json {
    let mut map = craftkit_sdk::serde_json::Map::new();
    for (address, link) in links {
        let mut one = craftkit_sdk::serde_json::Map::new();
        one.insert("target".to_string(), Json::String(link.target.clone()));
        if let Some(tooltip) = &link.tooltip {
            one.insert("tooltip".to_string(), Json::String(tooltip.clone()));
        }
        map.insert(address.clone(), Json::Object(one));
    }
    Json::Object(map)
}

/// Whether a **target** is a place in this workbook.
///
/// A target that names a sheet of this workbook and a cell in it is a
/// place **inside**; everything else — a web address, a `mailto:`, a
/// file — is outside. The tree's flag says whether the target is
/// outside, and the tree reads it from the file it was written in; here
/// it is **derived**, because this tree knows the sheets it has.
pub fn is_inside(target: &str, sheets: &[String]) -> bool {
    let Some((sheet, cell)) = target.split_once('!') else { return false };
    let sheet = sheet.trim();
    let sheet = sheet.strip_prefix('\'').unwrap_or(sheet);
    let sheet = sheet.strip_suffix('\'').unwrap_or(sheet);
    if !sheets.iter().any(|name| name.eq_ignore_ascii_case(sheet)) {
        return false;
    }
    crate::formula::CellRef::parse(cell.trim()).is_some()
}

/// The sheet a command is about, or the error that says there is no
/// such sheet.
fn sheet_of(session: &Session, sheet: u64, id: &str) -> Result<(), EngineError> {
    let Some(node) = session.document().node(sheet) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no sheet `{sheet}`"),
        });
    };
    if node.kind != "grid.sheet" {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the node `{sheet}` is a `{}` and not a sheet", node.kind),
        });
    }
    Ok(())
}

/// The **sheets** of the workbook, by name.
fn sheet_names(session: &Session) -> Vec<String> {
    session.document().walk().filter(|node| node.kind == "grid.sheet").map(|node| node.name.clone()).collect()
}

commands! {
    /// Put a **link** on a cell.
    ///
    /// A link that is already on the cell is **edited**: the target is the
    /// new target and the tooltip is kept unless a new one is given.
    grid.cell.link.set {
        area: "grid",
        layer: "grid",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The cell's address.
            cell: Text,
            /// Where it goes: a place in this workbook, or a place outside it.
            target: Text,
            /// The text that is shown when the mouse rests on it.
            tooltip: Option<Text>,
        },
        run: {
            let id = "grid.cell.link.set".to_string();
            let sheet = self.sheet.0;
            let cell = crate::cell_at(&self.cell.0, &id)?;
            let target = self.target.0.trim().to_string();
            if target.is_empty() {
                return Err(EngineError::Failed { id: id.clone(), message: "a link goes somewhere".to_string() });
            }
            sheet_of(session, sheet, &id)?;
            let tooltip =
                self.tooltip.as_ref().map(|tooltip| tooltip.0.clone()).filter(|tooltip| !tooltip.is_empty());
            let Some(node) = session.document_mut()?.node_mut(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            let mut links = links_of(node);
            let address = cell.to_text();
            let tooltip = tooltip.or_else(|| links.get(&address).and_then(|link| link.tooltip.clone()));
            links.insert(address.clone(), Link { target: target.clone(), tooltip: tooltip.clone() });
            node.data["links"] = links_into(&links);
            let inside = is_inside(&target, &sheet_names(session));
            Ok(Outcome::changed(format!("put a link on {address}")).with_data(json!({
                "sheet": sheet,
                "cell": address,
                "target": target,
                "tooltip": tooltip,
                "inside": inside,
            })))

        }
    }

    /// The **link** on a cell.
    grid.cell.link.get {
        area: "grid",
        layer: "grid",
        icon: "list",
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The cell's address.
            cell: Text,
        },
        run: {
            let id = "grid.cell.link.get".to_string();
            let sheet = self.sheet.0;
            let cell = crate::cell_at(&self.cell.0, &id)?;
            sheet_of(session, sheet, &id)?;
            let Some(node) = session.document().node(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            let links = links_of(node);
            let address = cell.to_text();
            let link = links.get(&address);
            let inside = link.map(|link| is_inside(&link.target, &sheet_names(session)));
            Ok(Outcome::new(match link {
                Some(_) => format!("{address} holds a link"),
                None => format!("{address} holds no link"),
            })
            .with_data(json!({
                "sheet": sheet,
                "cell": address,
                "link": match link {
                    Some(link) => json!({
                        "target": link.target,
                        "tooltip": link.tooltip,
                        "inside": inside.unwrap_or(false),
                    }),
                    None => Json::Null,
                },
            })))

        }
    }

    /// Take the **link** away from a cell.
    ///
    /// A cell that holds no link is not refused: there is nothing to take
    /// away, and the answer says so.
    grid.cell.link.delete {
        area: "grid",
        layer: "grid",
        icon: "minus",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The cell's address.
            cell: Text,
        },
        run: {
            let id = "grid.cell.link.delete".to_string();
            let sheet = self.sheet.0;
            let cell = crate::cell_at(&self.cell.0, &id)?;
            sheet_of(session, sheet, &id)?;
            let Some(node) = session.document_mut()?.node_mut(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            let mut links = links_of(node);
            let address = cell.to_text();
            let was_there = links.remove(&address).is_some();
            node.data["links"] = links_into(&links);
            let outcome = if was_there {
                Outcome::changed(format!("took the link off {address}"))
            } else {
                Outcome::new(format!("{address} held no link"))
            };
            Ok(outcome.with_data(json!({
                "sheet": sheet,
                "cell": address,
                "was_there": was_there,
            })))

        }
    }
}
