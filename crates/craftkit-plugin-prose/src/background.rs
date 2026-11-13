//! The **background** of a page.
//!
//! The page a document is printed on may be painted with a **colour**,
//! and it may carry a **watermark**: a text drawn over it, across it
//! or upright.
//!
//! # The rules
//!
//! - **A colour is `#rrggbb`**, and a colour that is not one is
//!   refused by name.
//! - **An empty value takes the fact away.** A page with no colour is
//!   a page that is not painted, and a document with no watermark is
//!   a document that has none.
//! - **A colour that is not said leaves the fact as it was.** A
//!   watermark that is given a new text and no colour keeps the colour
//!   it has.
//! - **The watermark's text is at most two hundred characters**, and a
//!   longer text is refused rather than shortened.
//! - **The watermark of the tree is diagonal and grey**: a watermark
//!   that is made and given neither keeps the tree's two facts.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, NodeRef, Outcome, Session, Text};

/// The most characters a watermark's text may hold.
const MOST_CHARACTERS: usize = 200;

/// The colour of the tree's own watermark: a pale grey.
const GREY: &str = "#c0c0c0";

/// The **document** a background belongs to.
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

/// A **colour**, if it is one.
fn colour_of(text: &str, id: &str) -> Result<String, EngineError> {
    let colour = text.trim().to_string();
    if let Err(why) = craftkit_sdk::Colour::from_hex(&colour) {
        return Err(EngineError::Failed { id: id.to_string(), message: why });
    }
    Ok(colour)
}

/// The **colour** of the page of a document, if it has one.
pub fn background_of(node: &craftkit_sdk::Node) -> Option<String> {
    node.data.get("page_colour").and_then(|colour| colour.as_str()).map(|colour| colour.to_string())
}

/// The **watermark** of a document, if it has one.
pub fn watermark_of(node: &craftkit_sdk::Node) -> Option<Json> {
    let watermark = node.data.get("watermark")?;
    let text = watermark.get("text").and_then(|text| text.as_str())?.to_string();
    if text.is_empty() {
        return None;
    }
    Some(json!({
        "text": text,
        "diagonal": watermark.get("diagonal").and_then(|diagonal| diagonal.as_bool()).unwrap_or(true),
        "colour": watermark.get("colour").and_then(|colour| colour.as_str()).unwrap_or(GREY),
    }))
}

commands! {
    /// **Paint** the page of a document with a **colour**.
    ///
    /// An **empty** colour takes the colour away, and the page is then a
    /// page that is not painted.
    prose.page.background {
        area: "prose",
        layer: "prose",
        icon: "paint",
        mutates,
        undoable,
        params: {
            /// The document.
            document: NodeRef,
            /// The colour of the page, as `#rrggbb`.
            ///
            /// An empty colour takes the colour of the page away.
            colour: Text,
        },
        run: {
            let id = "prose.page.background".to_string();
            let document = self.document.0;
            the_document(session, &id, document)?;
            let text = self.colour.0.trim();
            if text.is_empty() {
                let Some(node) = session.document_mut()?.node_mut(document) else {
                    return Err(EngineError::Failed {
                        id,
                        message: format!("there is no document `{document}`"),
                    });
                };
                node.data.as_object_mut().map(|map| map.remove("page_colour"));
                return Ok(Outcome::changed(format!("the page of the document `{document}` is not painted"))
                    .with_data(json!({ "document": document, "colour": Json::Null })));
            }
            let colour = colour_of(text, &id)?;
            let Some(node) = session.document_mut()?.node_mut(document) else {
                return Err(EngineError::Failed { id, message: format!("there is no document `{document}`") });
            };
            node.data["page_colour"] = json!(colour);
            Ok(Outcome::changed(format!("the page of the document `{document}` is painted {colour}"))
                .with_data(json!({ "document": document, "colour": colour })))

        }
    }

    /// **Put** a **watermark** on the page of a document.
    ///
    /// The text is what the watermark says, and an **empty** text takes
    /// the watermark away. A colour that is not said leaves the colour of
    /// the watermark as it was, and a watermark that is made and given
    /// neither is the tree's: diagonal and grey.
    prose.page.watermark {
        area: "prose",
        layer: "prose",
        icon: "paint",
        mutates,
        undoable,
        params: {
            /// The document.
            document: NodeRef,
            /// What the watermark says.
            ///
            /// An empty text takes the watermark away.
            text: Text,
            /// Whether the watermark runs **across** the page.
            diagonal: Option<Bool>,
            /// The colour of the watermark, as `#rrggbb`.
            colour: Option<Text>,
        },
        run: {
            let id = "prose.page.watermark".to_string();
            let document = self.document.0;
            the_document(session, &id, document)?;
            let text = self.text.0.trim().to_string();
            if text.is_empty() {
                let Some(node) = session.document_mut()?.node_mut(document) else {
                    return Err(EngineError::Failed {
                        id,
                        message: format!("there is no document `{document}`"),
                    });
                };
                node.data.as_object_mut().map(|map| map.remove("watermark"));
                return Ok(Outcome::changed(format!("the document `{document}` has no watermark"))
                    .with_data(json!({ "document": document, "watermark": Json::Null })));
            }
            if text.chars().count() > MOST_CHARACTERS {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("a watermark says at most {MOST_CHARACTERS} characters"),
                });
            }
            // The **watermark** as it is now: a colour that is not said
            // leaves the colour as it was.
            let held = session.document().node(document).and_then(watermark_of);
            let colour = match &self.colour {
                Some(colour) => colour_of(&colour.0, &id)?,
                None => held
                    .as_ref()
                    .and_then(|watermark| watermark.get("colour"))
                    .and_then(|colour| colour.as_str())
                    .map(|colour| colour.to_string())
                    .unwrap_or_else(|| GREY.to_string()),
            };
            let diagonal = match &self.diagonal {
                Some(diagonal) => diagonal.0,
                None => held
                    .as_ref()
                    .and_then(|watermark| watermark.get("diagonal"))
                    .and_then(|diagonal| diagonal.as_bool())
                    .unwrap_or(true),
            };
            let watermark = json!({ "text": text, "diagonal": diagonal, "colour": colour });
            let Some(node) = session.document_mut()?.node_mut(document) else {
                return Err(EngineError::Failed { id, message: format!("there is no document `{document}`") });
            };
            node.data["watermark"] = watermark.clone();
            Ok(Outcome::changed(format!("the document `{document}` has a watermark")).with_data(json!({
                "document": document,
                "watermark": watermark,
            })))

        }
    }
}
