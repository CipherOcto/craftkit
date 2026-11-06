//! The **fields** of a form.
//!
//! A form is made of **fields**: the places where a person writes.
//! Each field says what it is — the entry `/FT` — and what it holds:
//!
//! - a **text** field (`Tx`) holds text;
//! - a **button** field (`Btn`) holds a **state** — the box that is
//!   ticked, or the button that is pressed;
//! - a **choice** field (`Ch`) holds one of its **options** — the
//!   list it offers.
//!
//! And a field says what it is called (`/T`), what it holds now
//! (`/V`), and the **flags** that say how it behaves (`/Ff`).
//!
//! The rules:
//!
//! - a field's value must be one the field **can hold**: a choice
//!   holds one of its options, and a button holds one of its states —
//!   and a value the field cannot hold is **refused**, not put in
//!   anyway;
//! - a field **without** a name is named by the number of its object,
//!   so that it can be asked about.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Node, NodeRef, Outcome, Session, Text};

/// The **kind** of a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    /// A field that holds text: `Tx`.
    Text,
    /// A field that holds a state: `Btn`.
    Button,
    /// A field that holds one of its options: `Ch`.
    Choice,
}

impl FieldKind {
    /// The kind, from the name the file says.
    pub fn parse(name: &str) -> Option<FieldKind> {
        match name.trim() {
            "Tx" => Some(FieldKind::Text),
            "Btn" => Some(FieldKind::Button),
            "Ch" => Some(FieldKind::Choice),
            _ => None,
        }
    }

    /// The name the file says.
    pub fn name(self) -> &'static str {
        match self {
            FieldKind::Text => "Tx",
            FieldKind::Button => "Btn",
            FieldKind::Choice => "Ch",
        }
    }
}

/// A **field** of a form.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    /// The number of the object the field is.
    pub object: u64,
    /// What the field is called.
    pub name: String,
    /// What the field is.
    pub kind: FieldKind,
    /// What the field holds now.
    pub value: Option<String>,
    /// The flags that say how the field behaves.
    pub flags: f64,
    /// The options a choice offers: the values it holds.
    pub options: Vec<String>,
    /// The number of the object of the **parent** of the field.
    pub parent: Option<u64>,
}

/// The **text** a value holds: the file's own spelling of a text.
pub fn text_of(value: &Json) -> Option<String> {
    match value {
        Json::String(text) => Some(text.clone()),
        Json::Object(map) => map.get("string").and_then(|text| text.as_str()).map(|text| text.to_string()),
        _ => None,
    }
}

/// The **object number** a reference names.
fn number_of(value: &Json) -> Option<u64> {
    value.get("ref").and_then(|pair| pair.as_array())?.first()?.as_u64()
}

/// The **node** an object number names.
fn node_of_number(document: &Document, number: u64) -> Option<&Node> {
    document.walk().find(|node| node.data.get("object").and_then(|found| found.as_u64()) == Some(number))
}

/// The **qualified name** of a field: its own name and the names of
/// its parents, joined with a dot.
///
/// The chain is **bounded** and a **cycle** ends it, so that a field
/// that is its own ancestor still has a name.
pub fn qualified_name(document: &Document, node: &Node) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut seen: Vec<u64> = Vec::new();
    let mut current = Some(node.id);
    while let Some(id) = current {
        if parts.len() > 64 {
            break;
        }
        let Some(found) = document.node(id) else { break };
        let object = found.data.get("object").and_then(|number| number.as_u64()).unwrap_or(0);
        if seen.contains(&object) {
            break;
        }
        seen.push(object);
        let entries = entries_of(found);
        if let Some(name) = entry(&entries, "T").and_then(text_of) {
            parts.push(name);
        }
        current = entry(&entries, "Parent")
            .and_then(number_of)
            .and_then(|number| node_of_number(document, number))
            .map(|parent| parent.id);
    }
    parts.reverse();
    (!parts.is_empty()).then(|| parts.join("."))
}

/// The **entries** of a node's dictionary.
fn entries_of(node: &Node) -> Vec<(String, Json)> {
    crate::entries_of(&node.data)
}

/// The **entry** of a dictionary, by its key.
fn entry<'a>(entries: &'a [(String, Json)], key: &str) -> Option<&'a Json> {
    entries.iter().find(|(name, _)| name == key).map(|(_, value)| value)
}

/// The **options** of a choice: the values it may hold.
///
/// An option is either a text, or a pair — the value and the text
/// that is shown for it.
fn options_of(value: &Json) -> Vec<String> {
    let Some(list) = value.as_array() else { return Vec::new() };
    let mut options: Vec<String> = Vec::new();
    for option in list {
        if let Some(text) = text_of(option) {
            options.push(text);
        } else if let Some(pair) = option.as_array()
            && let Some(first) = pair.first()
            && let Some(text) = text_of(first)
        {
            options.push(text);
        }
    }
    options
}

/// The **pairs** of a dictionary that is a value: the key and the
/// value, in the order the file had them.
fn pairs_of(value: &Json) -> Vec<(String, Json)> {
    let Some(list) = value.get("entries").and_then(|entries| entries.as_array()) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|pair| {
            let pair = pair.as_array()?;
            let key = pair.first()?.as_str()?.to_string();
            let value = pair.get(1)?.clone();
            Some((key, value))
        })
        .collect()
}

/// The **states** of a button: the names of its appearances.
///
/// A button that has no appearance has no states, and it may not be
/// pressed.
fn states_of(node: &Node) -> Vec<String> {
    let entries = entries_of(node);
    let Some(appearance) = entry(&entries, "AP") else { return Vec::new() };
    let Some((_, normal)) = pairs_of(appearance).into_iter().find(|(key, _)| key == "N") else {
        return Vec::new();
    };
    pairs_of(&normal).into_iter().map(|(key, _)| key).collect()
}

/// The **fields** of a document, in the order the document has them.
pub fn fields_of(document: &Document) -> Vec<Field> {
    let mut fields: Vec<Field> = Vec::new();
    for node in document.walk() {
        if node.kind != "pdf.object" && node.kind != "pdf.stream" {
            continue;
        }
        let entries = entries_of(node);
        let Some(kind) = entry(&entries, "FT").and_then(|kind| kind.as_str()).and_then(FieldKind::parse)
        else {
            continue;
        };
        let object = node.data.get("object").and_then(|object| object.as_u64()).unwrap_or(0);
        let name = match qualified_name(document, node) {
            Some(name) => name,
            None => format!("object {object}"),
        };
        let parent = entry(&entries, "Parent").and_then(number_of);
        let value = entry(&entries, "V")
            .and_then(|value| text_of(value).or_else(|| value.as_str().map(|v| v.to_string())));
        let flags = entry(&entries, "Ff").and_then(|flags| flags.as_f64()).unwrap_or(0.0);
        let options = match entry(&entries, "Opt") {
            Some(options) => options_of(options),
            None => Vec::new(),
        };
        fields.push(Field { object, name, kind, value, flags, options, parent });
    }
    fields
}

/// The **field** a node is: what it says about itself.
fn field_of(document: &Document, node: &Node) -> Option<Field> {
    let fields = fields_of(document);
    let object = node.data.get("object").and_then(|object| object.as_u64())?;
    fields.into_iter().find(|field| field.object == object)
}

/// **Read** the fields of a form.
///
/// A query: it does not change the document.
#[derive(Command)]
#[command(id = "pdf.field.read", area = "pdf", layer = "pdf", icon = "list")]
pub struct ReadFields {
    /// The field to read.
    ///
    /// Leave it out and every field of the document is read.
    pub field: Option<NodeRef>,
}

impl ReadFields {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "pdf.field.read".to_string();
        let document = session.document();
        let mut fields = fields_of(document);
        if let Some(field) = self.field {
            let node = document.node(field.0).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{}`", field.0),
            })?;
            let one = field_of(document, node).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{}` is not a field", field.0),
            })?;
            fields = vec![one];
        }
        let count = fields.len();
        let listed: Vec<Json> = fields
            .iter()
            .map(|field| {
                json!({
                    "object": field.object,
                    "name": field.name.clone(),
                    "kind": field.kind.name(),
                    "value": field.value.clone().map(|value| json!(value)).unwrap_or(Json::Null),
                    "flags": field.flags,
                    "options": field.options.clone(),
                    "parent": field.parent.map(|parent| json!(parent)).unwrap_or(Json::Null),
                })
            })
            .collect();
        Ok(Outcome::new(format!("the form has {count} field(s)")).with_data(json!({
            "fields": listed,
            "count": count,
        })))
    }
}

/// **Fill** a field of a form.
#[derive(Command)]
#[command(id = "pdf.field.set", area = "pdf", layer = "pdf", icon = "pencil", mutates, undoable)]
pub struct SetField {
    /// The field.
    pub field: NodeRef,
    /// What it holds now.
    ///
    /// An empty text takes the value away.
    pub value: Text,
}

impl SetField {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "pdf.field.set".to_string();
        let field = self.field.0;
        let Some(node) = session.document().node(field).cloned() else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{field}`"),
            });
        };
        let Some(found) = field_of(session.document(), &node) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{field}` is not a field"),
            });
        };
        let value = self.value.0.clone();
        // The value must be one the field **can hold**.
        match found.kind {
            FieldKind::Text => {}
            FieldKind::Choice => {
                if !value.is_empty() && !found.options.contains(&value) {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "`{value}` is not one of the options of the field: the options are {}",
                            if found.options.is_empty() {
                                "none".to_string()
                            } else {
                                found.options.join(", ")
                            }
                        ),
                    });
                }
            }
            FieldKind::Button => {
                let states = states_of(&node);
                if !value.is_empty() && !states.contains(&value) {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "`{value}` is not one of the states of the field: the states are {}",
                            if states.is_empty() { "none".to_string() } else { states.join(", ") }
                        ),
                    });
                }
            }
        }
        let mut entries = entries_of(&node);
        if value.is_empty() {
            // An **empty** value takes the value away.
            entries.retain(|(key, _)| key != "V");
        } else {
            let value_json = json!({ "string": value.clone() });
            match entries.iter_mut().find(|(key, _)| key == "V") {
                Some(entry) => entry.1 = value_json,
                None => entries.push(("V".to_string(), value_json)),
            }
        }
        let data = json!({
            "object": node.data.get("object").cloned().unwrap_or(Json::Null),
            "entries": entries.iter().map(|(key, value)| json!([key, value])).collect::<Vec<_>>(),
        });
        let Some(node) = session.document_mut()?.node_mut(field) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{field}`"),
            });
        };
        node.data = data;
        Ok(Outcome::changed(format!("the field `{field}` holds `{value}`"))
            .with_data(json!({ "field": field, "value": value })))
    }
}

/// **Put** a field in a field.
///
/// A field may have a **parent**: the field it belongs to. The name
/// of a field is then the name of its parent and its own, joined with
/// a dot — the tree's fully qualified name.
///
/// The **rules**:
///
/// - the field and the parent must both be **fields**;
/// - a field may not be **its own** parent;
/// - a parent that is **below** the field would make a **loop**, and
///   a loop is refused.
#[derive(Command)]
#[command(id = "pdf.field.parent", area = "pdf", layer = "pdf", icon = "link", mutates, undoable)]
pub struct SetParent {
    /// The field.
    pub field: NodeRef,
    /// The field it belongs to.
    ///
    /// Leave it out and the field has no parent.
    pub parent: Option<NodeRef>,
}

impl SetParent {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "pdf.field.parent".to_string();
        let field = self.field.0;
        let Some(node) = session.document().node(field).cloned() else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{field}`"),
            });
        };
        let Some(found) = field_of(session.document(), &node) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{field}` is not a field"),
            });
        };
        let document = session.document();
        match self.parent {
            None => {
                // The field has **no parent** any more.
                let mut entries = entries_of(&node);
                entries.retain(|(key, _)| key != "Parent");
                let data = json!({
                    "object": node.data.get("object").cloned().unwrap_or(Json::Null),
                    "entries": entries.iter().map(|(key, value)| json!([key, value])).collect::<Vec<_>>(),
                });
                let Some(node) = session.document_mut()?.node_mut(field) else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("there is no node `{field}`"),
                    });
                };
                node.data = data;
                Ok(Outcome::changed(format!("the field `{field}` has no parent now"))
                    .with_data(json!({ "field": field, "parent": Json::Null })))
            }
            Some(parent) => {
                if parent.0 == field {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the field `{field}` may not be its own parent"),
                    });
                }
                let Some(parent_node) = document.node(parent.0).cloned() else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("there is no node `{}`", parent.0),
                    });
                };
                let Some(parent_field) = field_of(document, &parent_node) else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the node `{}` is not a field", parent.0),
                    });
                };
                // The parent may not be **below** the field: that would
                // be a loop.
                let mut current = Some(parent_field.object);
                let mut seen: Vec<u64> = Vec::new();
                while let Some(object) = current {
                    if seen.contains(&object) {
                        break;
                    }
                    seen.push(object);
                    if object == found.object {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!(
                                "the field `{}` is below the field `{field}`, and that would be a loop",
                                parent.0
                            ),
                        });
                    }
                    let Some(node) = node_of_number(document, object) else { break };
                    let entries = entries_of(node);
                    current = entry(&entries, "Parent").and_then(number_of);
                }
                let mut entries = entries_of(&node);
                let value = json!({ "ref": [parent_field.object, 0] });
                match entries.iter_mut().find(|(key, _)| key == "Parent") {
                    Some(entry) => entry.1 = value,
                    None => entries.push(("Parent".to_string(), value)),
                }
                let data = json!({
                    "object": node.data.get("object").cloned().unwrap_or(Json::Null),
                    "entries": entries.iter().map(|(key, value)| json!([key, value])).collect::<Vec<_>>(),
                });
                let Some(node) = session.document_mut()?.node_mut(field) else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("there is no node `{field}`"),
                    });
                };
                node.data = data;
                Ok(Outcome::changed(format!("the field `{field}` is in the field `{}`", parent.0))
                    .with_data(json!({ "field": field, "parent": parent.0 })))
            }
        }
    }
}
