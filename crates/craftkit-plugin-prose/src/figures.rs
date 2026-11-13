//! The **table of figures** of a document.
//!
//! A table of figures is the list of the **captions** that carry one
//! label: the figures of a document, the tables of a document, the
//! equations of a document. The tree's table of figures is a field in
//! the document and it is rebuilt when the document changes; here it
//! is **derived**: it is read out of the captions, and it cannot be out
//! of date.
//!
//! # The rules
//!
//! - The **label** names the captions the table is of. It is the
//!   tree's label, `Figure`, when it is not given, and a label that
//!   says nothing is that label too.
//! - The **identifier** of a label is the tree's: a label holds no
//!   quote, no backslash and no control character, it is at most
//!   **40** characters long, and its words are joined with an
//!   underscore — the tree joins a multi-word caption label with
//!   underscores, so `Photo Plate` and `Photo_Plate` are one label.
//! - The **entries** are the captions that carry the label, in the
//!   order of the document.
//! - The **number** of an entry is the number of its caption, and a
//!   caption that names a chapter has the whole number — the chapter
//!   and the number together.
//! - The **entry** shows the label and the number, and the text of
//!   the caption after them. A table of figures asked not to carry
//!   the label shows the text of the caption alone, which is the
//!   tree's own rule.
//!
//! # What it does not do
//!
//! - **The table is not laid out.** The tree's table of figures ends
//!   every entry with the **page** the caption is on, and it has a
//!   tab leader and a right-aligned page number. There is no layout
//!   here, and so there is no page to show.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, NodeRef, Outcome, Text};

/// The longest **label** of a table of figures: the tree's.
pub const MAX_LABEL: usize = 40;

/// The label a table of figures has when it is not given one: the
/// tree's.
pub const DEFAULT_LABEL: &str = "Figure";

/// The **identifier** of a label: the tree's own.
///
/// A label holds no quote, no backslash and no control character, it
/// is at most forty characters long, and its words are joined with an
/// underscore. A label that says nothing after that is `Figure`.
pub fn ident_of(label: &str) -> String {
    let cleaned: String = label
        .chars()
        .filter(|character| !matches!(character, '"' | '\\') && !character.is_control())
        .take(MAX_LABEL)
        .collect();
    let ident = cleaned.split_whitespace().collect::<Vec<_>>().join("_");
    if ident.is_empty() { DEFAULT_LABEL.to_string() } else { ident }
}

/// The **entries** of a table of figures: the captions that carry the
/// label, in the order of the document.
///
/// The entries are derived from the captions of the document, and the
/// number of each entry is the number of its caption.
pub fn entries_of(
    document: &craftkit_sdk::Document,
    document_node: u64,
    ident: &str,
    include_label: bool,
) -> Vec<Json> {
    let Some(node) = document.node(document_node) else { return Vec::new() };
    let captions = crate::captions::captions_of(document, node);
    let Some(captions) = captions.as_array() else { return Vec::new() };
    let mut entries = Vec::new();
    for caption in captions {
        let label = caption.get("label").and_then(|label| label.as_str()).unwrap_or(DEFAULT_LABEL);
        if ident_of(label) != ident {
            continue;
        }
        // The **number** of the caption: the whole number when the
        // caption names a chapter, and the number itself otherwise.
        let number = match caption.get("full").and_then(|full| full.as_str()) {
            Some(full) if !full.is_empty() => full.to_string(),
            _ => match caption.get("number").and_then(|number| number.as_str()) {
                Some(number) => number.to_string(),
                None => String::new(),
            },
        };
        let text = caption.get("text").and_then(|text| text.as_str()).unwrap_or_default().to_string();
        // The **entry**: the label and the number, and the text of the
        // caption after them. A table asked not to carry the label
        // shows the text alone.
        let entry = if include_label {
            let mut entry = format!("{label} {number}");
            if !text.is_empty() {
                entry.push(' ');
                entry.push_str(&text);
            }
            entry
        } else {
            text.clone()
        };
        entries.push(json!({
            "block": caption.get("block").cloned().unwrap_or(Json::Null),
            "label": label,
            "number": number,
            "text": text,
            "entry": entry,
        }));
    }
    entries
}

commands! {
    /// **Read** the **table of figures** of a document: the captions that
    /// carry one label.
    prose.document.figures {
        area: "prose",
        layer: "prose",
        icon: "list",
        params: {
            /// The document.
            document: NodeRef,
            /// The label of the captions: `Figure` when it is not given.
            label: Option<Text>,
            /// Whether the entries carry the label: yes, unless it is said
            /// no.
            include_label: Option<Bool>,
        },
        run: {
            let id = "prose.document.figures".to_string();
            let document = self.document.0;
            let Some(node) = session.document().node(document) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{document}`"),
                });
            };
            if node.kind != "prose.document" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{document}` is a `{}` and not a document", node.kind),
                });
            }
            // The **label** of the table, and its **identifier**: the
            // tree's own.
            let label = self.label.as_ref().map(|label| label.0.as_str()).unwrap_or(DEFAULT_LABEL);
            let ident = ident_of(label);
            let include_label = self.include_label.map(|flag| flag.0).unwrap_or(true);
            let entries = entries_of(session.document(), document, &ident, include_label);
            Ok(Outcome::new(format!(
                "the table of figures `{ident}` of the document `{document}` has {} entries",
                entries.len()
            ))
            .with_data(json!({
                "document": document,
                "label": ident,
                "includeLabel": include_label,
                "entries": entries,
            })))

        }
    }
}
