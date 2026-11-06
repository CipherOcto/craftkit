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
//!
//! And a field may have **children**: the fields it holds, and the
//! **widgets** it shows itself with — a kid with no name is not a
//! child but the field's appearance on a page.
//!
//! And a field may **inherit** from its parent: a field that does not
//! say what it is takes the kind of its parent, and a field that does
//! not say how it behaves takes the flags of its parent. A field that
//! says its own kind or its own flags **replaces** what it would have
//! inherited — the two are not added together.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, Document, EngineError, Node, NodeRef, Outcome, Session, Text};

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
    /// What the field is: the kind it says, or the kind it inherits.
    pub kind: FieldKind,
    /// The kind the file says for the field itself.
    pub own_kind: Option<FieldKind>,
    /// What the field holds now.
    pub value: Option<String>,
    /// The flags that say how the field behaves: the flags it says, or
    /// the flags it inherits.
    pub flags: f64,
    /// The flags the file says for the field itself.
    pub own_flags: Option<f64>,
    /// The options a choice offers: the values it holds.
    pub options: Vec<String>,
    /// The number of the object of the **parent** of the field.
    pub parent: Option<u64>,
    /// The **children** of the field: the fields it holds.
    pub children: Vec<u64>,
    /// The **widgets** of the field: the kids with no name, which are
    /// the field's appearance on a page.
    pub widgets: Vec<u64>,
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

/// The **flags** of a field, as the tree's own names.
///
/// The flags are a number, and the number is a list of facts about
/// how the field behaves: a field that is **read only**, a field that
/// must be **filled in**, a field that holds **many lines**, and so
/// on.
///
/// The tree gives one bit two meanings — the twenty-sixth is both
/// *the radio buttons are in unison* and *the text is rich* — and a
/// bit with two meanings says nothing, so it is left unnamed.
fn flag_names(flags: f64) -> Vec<&'static str> {
    let bits = flags as u32;
    let mut names: Vec<&'static str> = Vec::new();
    let mut add = |bit: u32, name: &'static str| {
        if bits & bit != 0 {
            names.push(name);
        }
    };
    add(1, "read-only");
    add(1 << 1, "required");
    add(1 << 12, "multiline");
    add(1 << 13, "password");
    add(1 << 14, "no-toggle-to-off");
    add(1 << 15, "radio");
    add(1 << 16, "push-button");
    add(1 << 17, "combo");
    add(1 << 18, "edit");
    add(1 << 19, "sort");
    add(1 << 20, "file-select");
    add(1 << 21, "multi-select");
    add(1 << 22, "do-not-spell-check");
    add(1 << 23, "do-not-scroll");
    add(1 << 24, "comb");
    add(1 << 26, "commit-on-selection-change");
    names
}

/// The **parent** of a node: the node its `/Parent` names.
fn parent_of<'a>(document: &'a Document, node: &Node) -> Option<&'a Node> {
    let entries = entries_of(node);
    let number = entry(&entries, "Parent").and_then(number_of)?;
    node_of_number(document, number)
}

/// The **kids** of a node: the entries its `/Kids` holds.
fn kids_entries_of(node: &Node) -> Vec<Json> {
    let entries = entries_of(node);
    entry(&entries, "Kids").and_then(|kids| kids.as_array()).cloned().unwrap_or_default()
}

/// The **object numbers** the kids of a node name.
fn kids_of(node: &Node) -> Vec<u64> {
    kids_entries_of(node).iter().filter_map(number_of).collect()
}

/// The **kids** of a node, with one **kid** put in or taken out.
fn kids_with(node: &Node, kid: u64, keep: bool) -> Vec<Json> {
    let mut kids = kids_entries_of(node);
    kids.retain(|entry| number_of(entry) != Some(kid));
    if keep {
        kids.push(json!({ "ref": [kid, 0] }));
    }
    kids
}

/// The **value** of a `/Kids` entry: the list of the kids, and
/// nothing at all when the list is empty — an empty value takes the
/// fact away.
fn kids_of_value(kids: Vec<Json>) -> Option<Json> {
    (!kids.is_empty()).then(|| json!(kids))
}

/// The **data** of a node, with one **entry** set — and an entry with
/// no value is taken away.
fn data_with(node: &Node, key: &str, value: Option<Json>) -> Json {
    let mut entries = entries_of(node);
    match value {
        Some(value) => match entries.iter_mut().find(|(name, _)| name == key) {
            Some(entry) => entry.1 = value,
            None => entries.push((key.to_string(), value)),
        },
        None => entries.retain(|(name, _)| name != key),
    }
    let mut data = node.data.clone();
    data["entries"] = json!(entries.iter().map(|(key, value)| json!([key, value])).collect::<Vec<_>>());
    data
}

/// The **object number** of a node.
fn object_of(node: &Node) -> u64 {
    node.data.get("object").and_then(|number| number.as_u64()).unwrap_or(0)
}

/// The **kind** a node inherits: the kind of the nearest ancestor that
/// says one, and the number of the object it comes from.
///
/// The chain is **bounded** and a **cycle** ends it, so that a field
/// that is its own ancestor still has a kind.
fn inherited_kind(document: &Document, node: &Node) -> Option<(FieldKind, u64)> {
    // A field that says its own kind inherits none.
    if entry(&entries_of(node), "FT").and_then(|kind| kind.as_str()).and_then(FieldKind::parse).is_some() {
        return None;
    }
    let mut seen: Vec<u64> = vec![object_of(node)];
    let mut current = parent_of(document, node);
    while let Some(parent) = current {
        if seen.len() > 64 {
            break;
        }
        let object = object_of(parent);
        if seen.contains(&object) {
            break;
        }
        seen.push(object);
        let entries = entries_of(parent);
        if let Some(kind) = entry(&entries, "FT").and_then(|kind| kind.as_str()).and_then(FieldKind::parse) {
            return Some((kind, object));
        }
        current = parent_of(document, parent);
    }
    None
}

/// The **flags** a node inherits: the flags of the nearest ancestor
/// that says them, and the number of the object they come from.
///
/// The chain is **bounded** and a **cycle** ends it.
fn inherited_flags(document: &Document, node: &Node) -> Option<(f64, u64)> {
    // A field that says its own flags inherits none.
    if entry(&entries_of(node), "Ff").and_then(|flags| flags.as_f64()).is_some() {
        return None;
    }
    let mut seen: Vec<u64> = vec![object_of(node)];
    let mut current = parent_of(document, node);
    while let Some(parent) = current {
        if seen.len() > 64 {
            break;
        }
        let object = object_of(parent);
        if seen.contains(&object) {
            break;
        }
        seen.push(object);
        let entries = entries_of(parent);
        if let Some(flags) = entry(&entries, "Ff").and_then(|flags| flags.as_f64()) {
            return Some((flags, object));
        }
        current = parent_of(document, parent);
    }
    None
}

/// The **fields** of a document, in the order the document has them.
pub fn fields_of(document: &Document) -> Vec<Field> {
    let mut fields: Vec<Field> = Vec::new();
    for node in document.walk() {
        if node.kind != "pdf.object" && node.kind != "pdf.stream" {
            continue;
        }
        let entries = entries_of(node);
        let own_kind = entry(&entries, "FT").and_then(|kind| kind.as_str()).and_then(FieldKind::parse);
        let own_flags = entry(&entries, "Ff").and_then(|flags| flags.as_f64());
        // A field that does not say what it is takes the kind of its
        // parent — and a node that says neither is not a field.
        let kind = match own_kind {
            Some(kind) => kind,
            None => {
                let has_name = entry(&entries, "T").and_then(text_of).is_some();
                if !has_name {
                    continue;
                }
                match inherited_kind(document, node) {
                    Some((kind, _)) => kind,
                    None => continue,
                }
            }
        };
        let flags = match own_flags {
            Some(flags) => flags,
            None => inherited_flags(document, node).map(|(flags, _)| flags).unwrap_or(0.0),
        };
        let object = node.data.get("object").and_then(|object| object.as_u64()).unwrap_or(0);
        let name = match qualified_name(document, node) {
            Some(name) => name,
            None => format!("object {object}"),
        };
        let parent = entry(&entries, "Parent").and_then(number_of);
        let value = entry(&entries, "V")
            .and_then(|value| text_of(value).or_else(|| value.as_str().map(|v| v.to_string())));
        let options = match entry(&entries, "Opt") {
            Some(options) => options_of(options),
            None => Vec::new(),
        };
        // The **kids**: a kid with a name is a field, and a kid with
        // no name is a widget — the field's appearance on a page.
        let mut children: Vec<u64> = Vec::new();
        let mut widgets: Vec<u64> = Vec::new();
        for kid in kids_of(node) {
            let Some(kid_node) = node_of_number(document, kid) else { continue };
            let kid_entries = entries_of(kid_node);
            if entry(&kid_entries, "T").and_then(text_of).is_some() {
                children.push(kid);
            } else {
                widgets.push(kid);
            }
        }
        fields.push(Field {
            object,
            name,
            kind,
            own_kind,
            value,
            flags,
            own_flags,
            options,
            parent,
            children,
            widgets,
        });
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
                    "own_kind": field.own_kind.map(|kind| json!(kind.name())).unwrap_or(Json::Null),
                    "value": field.value.clone().map(|value| json!(value)).unwrap_or(Json::Null),
                    "flags": field.flags,
                    "own_flags": field.own_flags.map(|flags| json!(flags)).unwrap_or(Json::Null),
                    "flag_names": flag_names(field.flags),
                    "options": field.options.clone(),
                    "parent": field.parent.map(|parent| json!(parent)).unwrap_or(Json::Null),
                    "children": field.children.clone(),
                    "widgets": field.widgets.clone(),
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

/// **Say** what a field **inherits** from its parents.
///
/// A field that does not say what it is takes the kind of its parent,
/// and a field that does not say how it behaves takes the flags of its
/// parent. A field that says its own kind or its own flags
/// **replaces** what it would have inherited — the two are not added
/// together.
///
/// The rules:
///
/// - a node that is **not a field** is refused;
/// - the chain of parents is **bounded**, and a **cycle** ends it, so
///   that a field that is its own ancestor still has an answer.
#[derive(Command)]
#[command(id = "pdf.field.inherit", area = "pdf", layer = "pdf", icon = "link")]
pub struct ReadInherited {
    /// The field.
    pub field: NodeRef,
}

impl ReadInherited {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "pdf.field.inherit".to_string();
        let field = self.field.0;
        let document = session.document();
        let Some(node) = document.node(field).cloned() else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{field}`"),
            });
        };
        let Some(found) = field_of(document, &node) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{field}` is not a field"),
            });
        };
        let kind = inherited_kind(document, &node);
        let flags = inherited_flags(document, &node);
        // The **chain**: the parents of the field, nearest first.
        let mut chain: Vec<u64> = Vec::new();
        let mut current = parent_of(document, &node);
        while let Some(parent) = current {
            if chain.len() > 64 {
                break;
            }
            let object = object_of(parent);
            if chain.contains(&object) {
                break;
            }
            chain.push(object);
            current = parent_of(document, parent);
        }
        Ok(Outcome::new(format!("the field `{field}` inherits from {} parent(s)", chain.len())).with_data(
            json!({
                "field": field,
                "name": found.name.clone(),
                "kind": kind.map(|(kind, _)| json!(kind.name())).unwrap_or(Json::Null),
                "kind_from": kind.map(|(_, object)| json!(object)).unwrap_or(Json::Null),
                "flags": flags.map(|(flags, _)| json!(flags)).unwrap_or(Json::Null),
                "flags_from": flags.map(|(_, object)| json!(object)).unwrap_or(Json::Null),
                "chain": chain,
            }),
        ))
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
/// - the parent must be a **field**;
/// - the field must be a field, or a node that says what it is
///   **called** — a field that has not said what it is yet is a field
///   once it is in one;
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
        // The field may be a field, or a node that says what it is
        // called: a field that has not said what it is yet becomes one
        // when it is put in a field.
        let found = field_of(session.document(), &node);
        if found.is_none() && entry(&entries_of(&node), "T").and_then(text_of).is_none() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{field}` has no name, and a field is named"),
            });
        }
        let document = session.document();
        match self.parent {
            None => {
                // The field has **no parent** any more.
                let old_parent = entry(&entries_of(&node), "Parent").and_then(number_of);
                let child = object_of(&node);
                let old_parent_node = old_parent.and_then(|object| node_of_number(document, object)).cloned();
                let mut entries = entries_of(&node);
                entries.retain(|(key, _)| key != "Parent");
                let data = json!({
                    "object": node.data.get("object").cloned().unwrap_or(Json::Null),
                    "entries": entries.iter().map(|(key, value)| json!([key, value])).collect::<Vec<_>>(),
                });
                if let Some(old) = &old_parent_node {
                    let kids = kids_with(old, child, false);
                    let data = data_with(old, "Kids", kids_of_value(kids));
                    if let Some(node) = session.document_mut()?.node_mut(old.id) {
                        node.data = data;
                    }
                }
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
                    if object == object_of(&node) {
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
                // The **old parent**: the field leaves it.
                let old_parent = entry(&entries_of(&node), "Parent").and_then(number_of);
                let child = object_of(&node);
                let old_parent_node = old_parent.and_then(|object| node_of_number(document, object)).cloned();
                // The **new parent**: the field joins it.
                let new_parent_node = Some(parent_node.clone());
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
                if let Some(old) = &old_parent_node
                    && old.data.get("object").and_then(|number| number.as_u64()) != Some(parent_field.object)
                {
                    let kids = kids_with(old, child, false);
                    let data = data_with(old, "Kids", kids_of_value(kids));
                    if let Some(node) = session.document_mut()?.node_mut(old.id) {
                        node.data = data;
                    }
                }
                if let Some(new) = &new_parent_node {
                    let kids = kids_with(new, child, true);
                    let data = data_with(new, "Kids", kids_of_value(kids));
                    if let Some(node) = session.document_mut()?.node_mut(parent.0) {
                        node.data = data;
                    }
                }
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

/// **Say** the **children** of a field.
///
/// A field may hold **children**: the fields it is a group of. The
/// file says them in its `/Kids`, and a kid with **no name** is not a
/// child but a **widget** — the field's appearance on a page.
///
/// The rules:
///
/// - a node that is **not a field** is refused;
/// - a kid that **names nothing** is a widget and not a child;
/// - a kid that is **not there** is not a child and not a widget: the
///   file names an object that is not in it.
#[derive(Command)]
#[command(id = "pdf.field.children", area = "pdf", layer = "pdf", icon = "list")]
pub struct ReadChildren {
    /// The field.
    pub field: NodeRef,
}

impl ReadChildren {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "pdf.field.children".to_string();
        let field = self.field.0;
        let document = session.document();
        let Some(node) = document.node(field).cloned() else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{field}`"),
            });
        };
        if field_of(document, &node).is_none() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{field}` is not a field"),
            });
        }
        let mut children: Vec<Json> = Vec::new();
        let mut widgets: Vec<u64> = Vec::new();
        for kid in kids_of(&node) {
            let Some(kid_node) = node_of_number(document, kid) else { continue };
            let kid_entries = entries_of(kid_node);
            let Some(name) = entry(&kid_entries, "T").and_then(text_of) else {
                // A kid with **no name** is a widget.
                widgets.push(kid);
                continue;
            };
            children.push(json!({
                "object": kid,
                "name": name,
                "field": qualified_name(document, kid_node).unwrap_or_else(|| name.clone()),
                "kind": entry(&kid_entries, "FT")
                    .and_then(|kind| kind.as_str())
                    .and_then(FieldKind::parse)
                    .map(|kind| json!(kind.name()))
                    .unwrap_or(Json::Null),
            }));
        }
        Ok(Outcome::new(format!("the field `{field}` holds {} child(ren)", children.len())).with_data(
            json!({
                "field": field,
                "children": children,
                "widgets": widgets,
            }),
        ))
    }
}

/// **Say** how a field behaves.
///
/// The flags of a field are a number, and two of them are the ones a
/// form is usually asked about: the field is **read only**, or the
/// field **must be filled in**. A flag that is not named here is left
/// as it was.
#[derive(Command)]
#[command(id = "pdf.field.flags", area = "pdf", layer = "pdf", icon = "pencil", mutates, undoable)]
pub struct SetFlags {
    /// The field.
    pub field: NodeRef,
    /// Whether the field is read only.
    pub read_only: Option<Bool>,
    /// Whether the field must be filled in.
    pub required: Option<Bool>,
}

impl SetFlags {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "pdf.field.flags".to_string();
        let field = self.field.0;
        let Some(node) = session.document().node(field).cloned() else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{field}`"),
            });
        };
        if field_of(session.document(), &node).is_none() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{field}` is not a field"),
            });
        }
        let entries = entries_of(&node);
        let mut flags = entry(&entries, "Ff").and_then(|flags| flags.as_f64()).unwrap_or(0.0);
        if !flags.is_finite() || flags < 0.0 || flags.fract() != 0.0 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "the flags of a field are a whole number".to_string(),
            });
        }
        let mut bits = flags as u32;
        if let Some(read_only) = &self.read_only {
            bits = if read_only.0 { bits | 1 } else { bits & !1 };
        }
        if let Some(required) = &self.required {
            bits = if required.0 { bits | (1 << 1) } else { bits & !(1 << 1) };
        }
        flags = f64::from(bits);
        let mut entries = entries;
        let value = json!(flags);
        match entries.iter_mut().find(|(key, _)| key == "Ff") {
            Some(entry) => entry.1 = value,
            None => entries.push(("Ff".to_string(), value)),
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
        Ok(Outcome::changed(format!("the flags of the field `{field}` are {flags}"))
            .with_data(json!({ "field": field, "flags": flags, "flag_names": flag_names(flags) })))
    }
}
