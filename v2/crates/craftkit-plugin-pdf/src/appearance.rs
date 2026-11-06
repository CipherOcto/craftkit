//! The **appearance** of a field.
//!
//! A field that is shown on a page has an **appearance**: the stream
//! that draws it. The appearance is a **form XObject** — a little
//! drawing that says what the field looks like — and the widget's
//! dictionary names it under the key `N` of its appearance dictionary
//! (`/AP`).
//!
//! An appearance may be a **set of states**: a button has one drawing
//! for the state it is off in and another for the state it is on in,
//! and the dictionary holds one stream per state. The field's value
//! (`/V`) says which state it is in.
//!
//! And a field may have **no appearance at all**: the form may say
//! that the reader is to make the appearances (`NeedAppearances`), and
//! then the field has nothing to show and the reader draws it.
//!
//! The rules:
//!
//! - the appearance is read as the file says it: the **box** it is
//!   drawn in (`/BBox`), the **matrix** that places it (`/Matrix`)
//!   and the **operators** of its stream;
//! - a field that says nothing about its appearance has none, and the
//!   answer says so.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Node, NodeRef, Outcome, Session};

/// The **appearance** of a field: what the file says about it.
#[derive(Debug, Clone, PartialEq)]
pub struct Appearance {
    /// The states the appearance has, when it has more than one.
    pub states: Vec<String>,
    /// The state the field is in, when the appearance has states.
    pub state: Option<String>,
    /// The box the appearance is drawn in.
    pub box_: Option<Vec<f64>>,
    /// The matrix that places the appearance.
    pub matrix: Option<Vec<f64>>,
    /// The operations of the appearance's stream.
    pub operations: Vec<Json>,
}

/// The **entries** of a node's dictionary.
fn entries_of(node: &Node) -> Vec<(String, Json)> {
    crate::entries_of(&node.data)
}

/// The **entry** of a dictionary, by its key.
fn entry<'a>(entries: &'a [(String, Json)], key: &str) -> Option<&'a Json> {
    entries.iter().find(|(name, _)| name == key).map(|(_, value)| value)
}

/// The **text** a value holds: the file's own spelling of a text.
fn text_of(value: &Json) -> Option<String> {
    match value {
        Json::String(text) => Some(text.clone()),
        Json::Object(map) => map.get("string").and_then(|text| text.as_str()).map(|text| text.to_string()),
        _ => None,
    }
}

/// The **numbers** a value holds.
fn numbers_of(value: &Json) -> Option<Vec<f64>> {
    let list = value.as_array()?;
    Some(list.iter().filter_map(|number| number.as_f64()).collect())
}

/// The **dictionary** a value holds: the pairs of an inline
/// dictionary.
fn dictionary_of(value: &Json) -> Option<Vec<(String, Json)>> {
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

/// The **node** an object number names.
fn node_of_number(document: &Document, number: u64) -> Option<&Node> {
    document.walk().find(|node| node.data.get("object").and_then(|found| found.as_u64()) == Some(number))
}

/// The **appearance** of a widget: the drawing the file holds for it.
pub fn appearance_of(document: &Document, node: &Node) -> Option<Appearance> {
    let entries = entries_of(node);
    let appearances = entry(&entries, "AP")?;
    // The **normal** appearance: the one that is drawn.
    let normal =
        dictionary_of(appearances)?.iter().find(|(key, _)| key == "N").map(|(_, value)| value.clone())?;
    // The appearance is either one stream, or a set of **states**.
    let states: Vec<(String, Json)> = dictionary_of(&normal).unwrap_or_default();
    let chosen: Option<Json> = if states.is_empty() {
        Some(normal.clone())
    } else {
        // The field's value says which state it is in.
        let value = entry(&entries, "V").and_then(text_of);
        value
            .and_then(|value| states.iter().find(|(name, _)| *name == value).map(|(_, held)| held.clone()))
            .or_else(|| states.first().map(|(_, held)| held.clone()))
    };
    let stream = match chosen {
        Some(value) => match crate::number_of(&value) {
            Some(number) => node_of_number(document, number)?.clone(),
            None => {
                // An appearance written where it is used is not a
                // stream this tree can read.
                return Some(Appearance {
                    states: states.iter().map(|(name, _)| name.clone()).collect(),
                    state: None,
                    box_: None,
                    matrix: None,
                    operations: Vec::new(),
                });
            }
        },
        None => {
            return Some(Appearance {
                states: states.iter().map(|(name, _)| name.clone()).collect(),
                state: None,
                box_: None,
                matrix: None,
                operations: Vec::new(),
            });
        }
    };
    let stream_entries = entries_of(&stream);
    let box_ = entry(&stream_entries, "BBox").and_then(numbers_of);
    let matrix = entry(&stream_entries, "Matrix").and_then(numbers_of);
    // The **operators** of the appearance: what it says to draw.
    let operations = match crate::decoded_bytes(&stream, None, None, false, "pdf.field.appearance") {
        Ok(decoded) => match crate::content::operations(&decoded.bytes) {
            Ok(operations) => operations.iter().map(|operation| operation.to_json()).collect(),
            Err(_) => Vec::new(),
        },
        Err(_) => Vec::new(),
    };
    let state = entry(&entries, "V").and_then(text_of);
    Some(Appearance {
        states: states.iter().map(|(name, _)| name.clone()).collect(),
        state,
        box_,
        matrix,
        operations,
    })
}

/// **Read** the **appearance** of a field.
///
/// A query: it does not change the document. A field that says nothing
/// about its appearance has none, and the answer says so.
#[derive(Command)]
#[command(id = "pdf.field.appearance", area = "pdf", layer = "pdf", icon = "list")]
pub struct ReadAppearance {
    /// The field to read.
    pub field: NodeRef,
}

impl ReadAppearance {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "pdf.field.appearance".to_string();
        let document = session.document();
        let node = document.node(self.field.0).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("there is no node `{}`", self.field.0),
        })?;
        let name = crate::fields::qualified_name(document, node);
        let Some(appearance) = appearance_of(document, node) else {
            return Ok(Outcome::new("the field has no appearance").with_data(json!({
                "field": name.clone().map(|name| json!(name)).unwrap_or(Json::Null),
                "has_appearance": false,
                "states": Vec::<Json>::new(),
                "state": Json::Null,
                "box": Json::Null,
                "matrix": Json::Null,
                "operations": Vec::<Json>::new(),
            })));
        };
        Ok(Outcome::new("the field has an appearance").with_data(json!({
            "field": name.clone().map(|name| json!(name)).unwrap_or(Json::Null),
            "has_appearance": true,
            "states": appearance.states.clone(),
            "state": appearance.state.clone().map(|state| json!(state)).unwrap_or(Json::Null),
            "box": appearance.box_.clone().map(|values| json!(values)).unwrap_or(Json::Null),
            "matrix": appearance.matrix.clone().map(|values| json!(values)).unwrap_or(Json::Null),
            "operations": appearance.operations.clone(),
        })))
    }
}
