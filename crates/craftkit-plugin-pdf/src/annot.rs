//! The **flags** of an annotation.
//!
//! An annotation says how it behaves with its **flags**: whether it
//! prints, whether it is hidden, whether it may be turned. The PDF/A
//! rules say that an annotation must **print** and must not be
//! **hidden** — a reader who prints the file must get the notes, and
//! a note that is hidden is a note nobody sees.
//!
//! The flags of the format:
//!
//! - the **first** bit is *hidden*;
//! - the **second** bit is *print*;
//! - the **fifth** bit is *no view* — the annotation cannot be seen;
//! - the **seventh** bit is *locked* — its properties may not be
//!   changed.
//!
//! The rules:
//!
//! - a node that is **not an annotation** is refused;
//! - a flag that is **not said** leaves the annotation as it was, and
//!   a flag that is not named is left as it was.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Bool, EngineError, Node, NodeRef, Outcome, Session};

/// The flag of an annotation that is **hidden**.
pub const HIDDEN: i64 = 1 << 1;

/// The flag of an annotation that **prints**.
pub const PRINT: i64 = 1 << 2;

/// The flag of an annotation that **cannot be seen**.
pub const NO_VIEW: i64 = 1 << 5;

/// The flag of an annotation that is **locked**.
pub const LOCKED: i64 = 1 << 7;

/// The **flags** of an annotation: the number the file holds.
pub fn flags_of(node: &Node) -> i64 {
    let entries = crate::entries_of(&node.data);
    let flags = crate::preflight::entry(&entries, "F").and_then(|flags| flags.as_f64()).unwrap_or(0.0);
    if !flags.is_finite() || flags <= 0.0 {
        return 0;
    }
    flags as i64
}

/// Whether an annotation **prints**.
pub fn prints(node: &Node) -> bool {
    flags_of(node) & PRINT != 0
}

/// Whether an annotation is **hidden**.
pub fn is_hidden(node: &Node) -> bool {
    flags_of(node) & (HIDDEN | NO_VIEW) != 0
}

/// Make an annotation **print** and be **shown**.
///
/// The print flag is set and the hidden flags are taken away. It says
/// whether anything changed: an annotation that already prints and is
/// already shown is not changed.
pub fn show(node: &mut Node) -> bool {
    let entries = crate::entries_of(&node.data);
    let before = flags_of(node);
    let after = (before | PRINT) & !(HIDDEN | NO_VIEW);
    if before == after && entries.iter().any(|(key, _)| key == "F") {
        return false;
    }
    let mut entries = entries;
    let value = json!(after);
    match entries.iter_mut().find(|(key, _)| key == "F") {
        Some(entry) => entry.1 = value,
        None => entries.push(("F".to_string(), value)),
    }
    node.data = json!({
        "object": node.data.get("object").cloned().unwrap_or(craftkit_sdk::serde_json::Value::Null),
        "entries": entries.iter().map(|(key, value)| json!([key, value])).collect::<Vec<_>>(),
    });
    true
}

/// **Say** the **flags** of an annotation.
///
/// The flags that are asked about are said; the flags that are not
/// asked about are left as they were.
///
/// The rules:
///
/// - a node that is **not an annotation** is refused;
/// - a flag that is **not said** leaves the annotation as it was.
#[derive(Command)]
#[command(id = "pdf.annot.flags", area = "pdf", layer = "pdf", icon = "pencil", mutates, undoable)]
pub struct SetAnnotationFlags {
    /// The annotation.
    pub annotation: NodeRef,
    /// Whether the annotation prints.
    ///
    /// Leave it out and the annotation keeps the flag it has.
    pub print: Option<Bool>,
    /// Whether the annotation is hidden.
    ///
    /// Leave it out and the annotation keeps the flag it has.
    pub hidden: Option<Bool>,
}

impl SetAnnotationFlags {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "pdf.annot.flags".to_string();
        let annotation = self.annotation.0;
        let Some(node) = session.document().node(annotation).cloned() else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{annotation}`"),
            });
        };
        let entries = crate::entries_of(&node.data);
        if !crate::preflight::is_an_annotation(&entries) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{annotation}` is not an annotation"),
            });
        }
        let mut bits = flags_of(&node);
        if let Some(print) = &self.print {
            bits = if print.0 { bits | PRINT } else { bits & !PRINT };
        }
        if let Some(hidden) = &self.hidden {
            bits = if hidden.0 { bits | (HIDDEN | NO_VIEW) } else { bits & !(HIDDEN | NO_VIEW) };
        }
        let mut entries = entries;
        let value = json!(bits);
        match entries.iter_mut().find(|(key, _)| key == "F") {
            Some(entry) => entry.1 = value,
            None => entries.push(("F".to_string(), value)),
        }
        let data = json!({
            "object": node.data.get("object").cloned().unwrap_or(craftkit_sdk::serde_json::Value::Null),
            "entries": entries.iter().map(|(key, value)| json!([key, value])).collect::<Vec<_>>(),
        });
        if let Some(node) = session.document_mut()?.node_mut(annotation) {
            node.data = data;
        }
        Ok(Outcome::changed(format!("the annotation `{annotation}` has the flags {bits}")).with_data(json!({
            "annotation": annotation,
            "flags": bits,
            "print": bits & PRINT != 0,
            "hidden": bits & (HIDDEN | NO_VIEW) != 0,
        })))
    }
}
