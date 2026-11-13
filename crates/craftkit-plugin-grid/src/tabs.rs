//! The **tabs** of a workbook: the sheets that are hidden, and the
//! colour of a sheet's tab.
//!
//! A sheet may be **hidden**, and a hidden sheet is still a sheet: it
//! is there, with its cells, and it is not in the way.
//!
//! The tree's finding is that a workbook must keep **one visible
//! sheet**: the tree's command refuses to hide the last one, because a
//! workbook with no sheet to look at is not a workbook — and hiding
//! the sheet that is being looked at moves the looking to another
//! sheet, the tree's own rule, and not to nothing.
//!
//! And a sheet's **tab colour** is a colour and not a name: the tree's
//! command takes a hex colour and `none`, and a colour that is not a
//! colour is refused.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session, Text};

/// The **sheets** of a workbook, and which of them are visible.
pub(crate) fn sheets_of(session: &Session, workbook: u64) -> Vec<(u64, bool)> {
    let Some(node) = session.document().node(workbook) else { return Vec::new() };
    node.children
        .iter()
        .filter(|child| child.kind == "grid.sheet")
        .map(|child| {
            (child.id, !child.data.get("hidden").and_then(|hidden| hidden.as_bool()).unwrap_or(false))
        })
        .collect()
}

/// The **workbook** a sheet is in, if it is in one.
pub(crate) fn workbook_of(session: &Session, sheet: u64) -> Option<u64> {
    session
        .document()
        .walk()
        .find(|node| node.children.iter().any(|child| child.id == sheet))
        .map(|node| node.id)
}

commands! {
    /// **Hide** a sheet.
    ///
    /// The last **visible** sheet of a workbook may not be hidden: the
    /// tree's command refuses it, because a workbook with no sheet to look
    /// at is not a workbook.
    grid.sheet.hide {
        area: "grid",
        layer: "grid",
        icon: "eye",
        mutates,
        undoable,
        params: {
            /// The sheet to hide.
            sheet: NodeRef,
        },
        run: {
            let id = "grid.sheet.hide".to_string();
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
            let Some(workbook) = workbook_of(session, sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the sheet `{sheet}` is in no workbook"),
                });
            };
            let visible = sheets_of(session, workbook).iter().filter(|(_, shown)| *shown).count();
            if visible <= 1 {
                return Err(EngineError::Failed {
                    id,
                    message: "a workbook must keep one visible sheet".to_string(),
                });
            }
            let Some(node) = session.document_mut()?.node_mut(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            node.data["hidden"] = json!(true);
            Ok(Outcome::changed(format!("hid the sheet `{sheet}`")).with_data(json!({ "sheet": sheet })))

        }
    }

    /// **Unhide** a sheet.
    grid.sheet.unhide {
        area: "grid",
        layer: "grid",
        icon: "eye",
        mutates,
        undoable,
        params: {
            /// The sheet to unhide.
            sheet: NodeRef,
        },
        run: {
            let id = "grid.sheet.unhide".to_string();
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
            let Some(node) = session.document_mut()?.node_mut(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            node.data["hidden"] = json!(false);
            Ok(Outcome::changed(format!("unhid the sheet `{sheet}`")).with_data(json!({ "sheet": sheet })))

        }
    }

    /// The **colour** of a sheet's tab.
    ///
    /// The colour is a hex colour — `#RRGGBB` — and `none` takes it away.
    /// A colour that is not a colour is **refused**: the tree's command
    /// takes a colour and not a word.
    grid.sheet.color {
        area: "grid",
        layer: "grid",
        icon: "paint",
        mutates,
        undoable,
        params: {
            /// The sheet.
            sheet: NodeRef,
            /// The colour: `#FF0000`, or `none`.
            color: Text,
        },
        run: {
            let id = "grid.sheet.color".to_string();
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
            let wanted = self.color.0.trim().to_string();
            let colour = if wanted.eq_ignore_ascii_case("none") || wanted.eq_ignore_ascii_case("auto") {
                None
            } else {
                Some(hex_colour(&wanted).ok_or_else(|| EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{wanted}` is not a colour"),
                })?)
            };
            let Some(node) = session.document_mut()?.node_mut(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            match &colour {
                Some(colour) => node.data["tab_color"] = json!(colour),
                None => node.data["tab_color"] = json!(null),
            }
            Ok(Outcome::changed(format!("the tab of the sheet `{sheet}` is coloured"))
                .with_data(json!({ "sheet": sheet, "color": colour })))

        }
    }
}

/// The colour a hex string says, as the tree's own three numbers.
///
/// The tree's rule: six hex digits, and nothing else. A string that
/// is not six hex digits is not a colour.
fn hex_colour(text: &str) -> Option<String> {
    let body = text.strip_prefix('#')?;
    if body.len() != 6 || !body.chars().all(|character| character.is_ascii_hexdigit()) {
        return None;
    }
    Some(format!("#{body}"))
}
