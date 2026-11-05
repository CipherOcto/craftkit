//! The **equation** of a document.
//!
//! An equation is a **block** whose text is not written down: it is
//! **derived** from the **parts** the equation is made of.
//!
//! A part is one of five:
//!
//! - a **text** — `{"kind": "text", "text": "x"}`;
//! - a **fraction** — `{"kind": "fraction", "top": …, "bottom": …}`;
//! - a **superscript** — `{"kind": "superscript", "base": …, "above": …}`;
//! - a **subscript** — `{"kind": "subscript", "base": …, "below": …}`;
//! - a **radical** — `{"kind": "radical", "inside": …}`.
//!
//! And the **linear form** — the text of the equation, the way it is
//! written on one line — is derived from them: `x`, `a/b`, `x^2`,
//! `x_1`, `√x`. A part that is put inside another is written in
//! **brackets**, so that the form says what it means: `(x^2)/y`.
//!
//! The rules:
//!
//! - a part that is **not one of the five** is refused, and so is a
//!   part that is **missing a piece** — a fraction with no bottom, a
//!   text that holds no text;
//! - the **depth** of an equation is bounded and the **length** of its
//!   text is bounded, because both come from outside;
//! - the text of an equation is **derived**, so it is **not
//!   replaced**: a replace would make the parts and the text disagree,
//!   and two things that disagree are one thing wrong.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Outcome, Session};

/// The most an equation may be **deep**.
pub const MAX_DEPTH: usize = 32;

/// The most an equation may have **parts**.
pub const MAX_PARTS: usize = 1024;

/// The most the **text** of an equation may be long.
pub const MAX_TEXT: usize = 4096;

/// The **kind** of a block, for the equations.
pub const EQUATION: &str = "prose.block.equation";

/// Whether a node is an **equation**.
pub fn is_equation(node: &Node) -> bool {
    node.kind == EQUATION
}

/// The **linear form** of a part, and whether it is a part at all.
///
/// The second answer is the number of the part, and it is counted as
/// the parts are read: an equation that is too big is refused before
/// it is written down.
fn linear_of(part: &Json, depth: usize, parts: &mut usize) -> Result<(String, bool), String> {
    if depth > MAX_DEPTH {
        return Err(format!("an equation is not more than {MAX_DEPTH} deep"));
    }
    *parts += 1;
    if *parts > MAX_PARTS {
        return Err(format!("an equation does not hold more than {MAX_PARTS} parts"));
    }
    let Some(map) = part.as_object() else {
        return Err("a part of an equation is a thing with a kind".to_string());
    };
    let kind = map.get("kind").and_then(|kind| kind.as_str()).unwrap_or_default();
    let mut child = |name: &str| -> Result<(String, bool), String> {
        let Some(child) = map.get(name) else {
            return Err(format!("a `{kind}` has no `{name}`"));
        };
        linear_of(child, depth + 1, parts)
    };
    Ok(match kind {
        "text" => {
            let Some(text) = map.get("text").and_then(|text| text.as_str()) else {
                return Err("a text of an equation holds text".to_string());
            };
            (text.to_string(), true)
        }
        "fraction" => {
            let (top, top_is_text) = child("top")?;
            let (bottom, bottom_is_text) = child("bottom")?;
            (
                format!(
                    "{}/{}",
                    if top_is_text { top } else { format!("({top})") },
                    if bottom_is_text { bottom } else { format!("({bottom})") }
                ),
                false,
            )
        }
        "superscript" => {
            let (base, base_is_text) = child("base")?;
            let (above, above_is_text) = child("above")?;
            (
                format!(
                    "{}^{}",
                    if base_is_text { base } else { format!("({base})") },
                    if above_is_text { above } else { format!("({above})") }
                ),
                false,
            )
        }
        "subscript" => {
            let (base, base_is_text) = child("base")?;
            let (below, below_is_text) = child("below")?;
            (
                format!(
                    "{}_{}",
                    if base_is_text { base } else { format!("({base})") },
                    if below_is_text { below } else { format!("({below})") }
                ),
                false,
            )
        }
        "radical" => {
            let (inside, inside_is_text) = child("inside")?;
            (format!("√{}", if inside_is_text { inside } else { format!("({inside})") }), false)
        }
        other => {
            return Err(format!(
                "`{other}` is not a part of an equation: it is text, a fraction, a superscript, a subscript or a radical"
            ));
        }
    })
}

/// The **text** of an equation, derived from its parts.
pub fn linear(parts: &Json) -> Result<String, String> {
    let mut count = 0usize;
    let (text, _) = linear_of(parts, 0, &mut count)?;
    if text.chars().count() > MAX_TEXT {
        return Err(format!("the text of an equation is at most {MAX_TEXT} characters long"));
    }
    Ok(text)
}

/// The **parts** of an equation, checked.
pub fn check(parts: &Json, id: &str) -> Result<String, EngineError> {
    linear(parts).map_err(|message| EngineError::Failed { id: id.to_string(), message })
}

/// **Put** an equation in a document.
///
/// The parts are the equation, and the text of the equation is
/// derived from them.
#[derive(Command)]
#[command(id = "prose.block.equation.add", area = "prose", layer = "prose", icon = "plus", mutates, undoable)]
pub struct AddEquation {
    /// The document it goes in.
    pub document: NodeRef,
    /// The **parts** of the equation: a text, a fraction, a
    /// superscript, a subscript or a radical.
    pub parts: Json,
}

impl AddEquation {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.block.equation.add".to_string();
        let text = check(&self.parts, &id)?;
        let Some(document) = session.document().node(self.document.0) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no document `{}`", self.document.0),
            });
        };
        if document.kind != "prose.document" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "the node `{}` is a `{}` and not a document",
                    self.document.0, document.kind
                ),
            });
        }
        let data = json!({ "parts": self.parts.clone(), "text": text });
        let block = session.document_mut()?.add_node_with_under(
            Some(self.document.0),
            "an equation".to_string(),
            EQUATION,
            None,
            data,
        );
        Ok(Outcome::changed(format!("added the equation `{text}`"))
            .with_data(json!({ "id": block, "kind": EQUATION, "text": text })))
    }
}

/// **Set** the parts of an equation.
///
/// The text of the equation is written with them, because the text is
/// the parts and the text is one thing.
#[derive(Command)]
#[command(id = "prose.equation.set", area = "prose", layer = "prose", icon = "pencil", mutates, undoable)]
pub struct SetEquation {
    /// The equation.
    pub block: NodeRef,
    /// The **parts** of the equation.
    pub parts: Json,
}

impl SetEquation {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.equation.set".to_string();
        let block = self.block.0;
        let Some(node) = session.document().node(block) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no block `{block}`"),
            });
        };
        if !is_equation(node) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{block}` is a `{}` and not an equation", node.kind),
            });
        }
        let text = check(&self.parts, &id)?;
        if let Some(node) = session.document_mut()?.node_mut(block) {
            node.data["parts"] = self.parts.clone();
            node.data["text"] = json!(text);
        }
        Ok(Outcome::changed(format!("the equation is `{text}` now"))
            .with_data(json!({ "block": block, "text": text })))
    }
}
