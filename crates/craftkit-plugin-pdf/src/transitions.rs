//! The **transition** of a page: how it comes in.
//!
//! A page may say how it is to be **shown**: a wipe, a dissolve, a
//! box that opens, a page that pushes the one before it. The
//! transition is in the page's `/Trans` — a dictionary with the **kind**
//! of the transition (`/S`), how long it takes (`/D`), the way it
//! moves (`/M`), the **dimension** it moves along (`/Dm`) and the
//! **angle** of a wipe (`/Di`).
//!
//! And a page may say how long it is to be **shown** before the
//! reader turns it: that is the `/Dur` of the page, in seconds.
//!
//! The rules:
//!
//! - a page with **no transition** has none, and the answer says so;
//! - the kind is read **as the file says it**: a file may name a
//!   transition this tree has not heard of, and the name is the
//!   fact;
//! - the **duration** is in seconds, and so is the duration of the
//!   page;
//! - a `/Trans` that is **not a dictionary** is refused: the page
//!   says something this tree cannot read, and saying nothing is
//!   better than saying half of it.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Outcome, Session};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.page.transition";

/// The **transition** of a page: how it comes in.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Transition {
    /// The **kind** of the transition, as the file says it.
    pub kind: Option<String>,
    /// How long the transition takes, in seconds.
    pub duration: Option<f64>,
    /// The **way** it moves: `In`, `Out` or nothing.
    pub direction: Option<String>,
    /// The **dimension** it moves along: `H`, `V` or nothing.
    pub dimension: Option<String>,
    /// The **angle** of a wipe, in degrees.
    pub angle: Option<f64>,
}

impl Transition {
    /// The transition, as the answer gives it.
    fn to_json(&self) -> Json {
        json!({
            "kind": self.kind.clone().map(|kind| json!(kind)).unwrap_or(Json::Null),
            "duration": self.duration.map(|duration| json!(duration)).unwrap_or(Json::Null),
            "direction": self.direction.clone().map(|direction| json!(direction)).unwrap_or(Json::Null),
            "dimension": self.dimension.clone().map(|dimension| json!(dimension)).unwrap_or(Json::Null),
            "angle": self.angle.map(|angle| json!(angle)).unwrap_or(Json::Null),
        })
    }
}

/// The **entries** of a node's dictionary.
fn entries_of(node: &craftkit_sdk::Node) -> Vec<(String, Json)> {
    crate::entries_of(&node.data)
}

/// The **entry** of a dictionary, by its key.
fn entry<'a>(entries: &'a [(String, Json)], key: &str) -> Option<&'a Json> {
    entries.iter().find(|(name, _)| name == key).map(|(_, value)| value)
}

/// The **transition** of a page, and how long the page is to be shown.
///
/// A page that says nothing about a transition has none.
pub fn transition_of(
    document: &Document,
    page: u64,
) -> Result<(Option<Transition>, Option<f64>), EngineError> {
    let Some(node) = document.node(page) else {
        return Err(EngineError::Failed {
            id: COMMAND.to_string(),
            message: format!("there is no node `{page}`"),
        });
    };
    let entries = entries_of(node);
    // The **duration** of the page: how long it is to be shown.
    let duration = entry(&entries, "Dur").and_then(|dur| dur.as_f64());
    // The **transition**: the page's own dictionary.
    let Some(value) = entry(&entries, "Trans").cloned() else {
        return Ok((None, duration));
    };
    let Some(entries) = dictionary_of(document, &value) else {
        return Err(EngineError::Failed {
            id: COMMAND.to_string(),
            message: format!("the transition of the page `{page}` is not a dictionary"),
        });
    };
    let transition = Transition {
        kind: entry(&entries, "S").and_then(|kind| kind.as_str()).map(String::from),
        duration: entry(&entries, "D").and_then(|duration| duration.as_f64()),
        direction: entry(&entries, "M").and_then(|direction| direction.as_str()).map(String::from),
        dimension: entry(&entries, "Dm").and_then(|dimension| dimension.as_str()).map(String::from),
        angle: entry(&entries, "Di").and_then(|angle| angle.as_f64()),
    };
    Ok((Some(transition), duration))
}

/// The **pairs** of a dictionary: the one the page holds, or one that
/// is written where it is used.
fn dictionary_of(document: &Document, value: &Json) -> Option<Vec<(String, Json)>> {
    if let Some(node) = crate::preflight::node_of(document, value) {
        return Some(crate::entries_of(&node.data));
    }
    let list = value.get("entries")?.as_array()?;
    Some(
        list.iter()
            .filter_map(|entry| {
                let pair = entry.as_array()?;
                let key = pair.first()?.as_str()?.to_string();
                Some((key, pair.get(1)?.clone()))
            })
            .collect(),
    )
}

/// **Read** the **transition** of a page.
///
/// A query: it does not change the document. A page with no transition
/// has none, and the answer says so.
#[derive(Command)]
#[command(id = "pdf.page.transition", area = "pdf", layer = "pdf", icon = "list")]
pub struct ReadTransition {
    /// The page to read.
    pub page: craftkit_sdk::NodeRef,
}

impl ReadTransition {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let page = self.page.0;
        let (transition, duration) = transition_of(session.document(), page)?;
        Ok(Outcome::new(match &transition {
            Some(_) => format!("the page `{page}` has a transition"),
            None => format!("the page `{page}` has no transition"),
        })
        .with_data(json!({
            "page": page,
            "transition": transition.map(|transition| transition.to_json()).unwrap_or(Json::Null),
            "duration": duration.map(|duration| json!(duration)).unwrap_or(Json::Null),
        })))
    }
}
