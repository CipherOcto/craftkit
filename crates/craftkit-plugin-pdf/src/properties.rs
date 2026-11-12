//! The **properties** of a field: the tree's own General tab.
//!
//! The tree's editor has a **dialog** for a field, and its first tab
//! is the **General** one: the name of the field, the text that is
//! shown when the reader rests on it, whether the field may be read
//! only, whether it must be filled in, whether it holds more than one
//! line, how many characters it holds, what it offers, where its text
//! sits, what the **reset** of the form restores, and whether its
//! properties may be changed at all.
//!
//! The rules of the tree, ported by name:
//!
//! - `set_props` — the **locked** field takes only **unlocking**: a
//!   field whose properties are locked refuses every change but the
//!   unlocking, as the tree refuses it;
//! - the **name** may not be empty and may not hold a period, and a
//!   name another field has is refused — and the name that is set is
//!   the **last** part of the name, the part the field has of its own;
//! - the **tooltip** is taken away when it is empty;
//! - the **options** of a choice may not be empty;
//! - the **alignment** is 0 (left), 1 (centre) or 2 (right);
//! - the **multiline** flag is the flag of a text field and of nothing
//!   else.
//!
//! The read-only flag and the required flag are the flags of the field
//! and are set by `pdf.field.flags`.

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, Document, EngineError, Node, NodeRef, Number, Outcome, Session, Text};

use crate::annot::LOCKED;
use crate::fields::fields_of;

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.field.properties.set";

/// The **multiline** flag: the tree's own, the thirteenth bit of the
/// flags of a field.
const MULTILINE: i64 = 1 << 12;

/// The **entries** of a node's dictionary.
fn entries_of(node: &Node) -> Vec<(String, Json)> {
    crate::entries_of(&node.data)
}

/// The **entry** of a dictionary, by its key.
fn entry<'a>(entries: &'a [(String, Json)], key: &str) -> Option<&'a Json> {
    entries.iter().find(|(name, _)| name == key).map(|(_, value)| value)
}

/// The **node** an object number names.
fn node_of_number(document: &Document, number: u64) -> Option<&Node> {
    document.walk().find(|node| node.data.get("object").and_then(|found| found.as_u64()) == Some(number))
}

/// The **flags** of a node: the number the file holds.
fn flags_of(node: &Node) -> i64 {
    entry(&entries_of(node), "F").and_then(|flags| flags.as_f64()).unwrap_or(0.0) as i64
}

/// Whether a field is **locked**: its properties may not be changed.
///
/// A field is locked when one of its **widgets** says so — and a field
/// that is its own widget says so itself.
pub(crate) fn is_locked(document: &Document, node: &Node) -> bool {
    if flags_of(node) & LOCKED != 0 {
        return true;
    }
    let kids = entry(&entries_of(node), "Kids").and_then(|kids| kids.as_array().cloned()).unwrap_or_default();
    for kid in kids {
        let Some(number) = crate::number_of(&kid) else { continue };
        let Some(kid_node) = node_of_number(document, number) else { continue };
        if flags_of(kid_node) & LOCKED != 0 {
            return true;
        }
    }
    false
}

/// The **options** of a field: a list of texts.
///
/// The list comes from outside, so it is read and checked: a thing that
/// is not a text is refused, and not taken as an empty text.
fn the_options(value: &serde_json::Value) -> Result<Vec<String>, EngineError> {
    let Some(list) = value.as_array() else {
        return Err(EngineError::Failed {
            id: COMMAND.to_string(),
            message: "the options of a field are a list of texts".to_string(),
        });
    };
    let mut out: Vec<String> = Vec::with_capacity(list.len());
    for item in list {
        let Some(text) = item.as_str() else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "the options of a field are a list of texts".to_string(),
            });
        };
        out.push(text.to_string());
    }
    Ok(out)
}

/// **Set** the **properties** of a field.
///
/// The properties that are not said are left as they were.
///
/// The rules:
///
/// - the node must be a **field** of a form;
/// - a **locked** field takes only unlocking: a field whose
///   properties are locked refuses every change but the unlocking, and
///   so it is refused here;
/// - the **name** may not be empty and may not hold a period, and a
///   name another field has is refused;
/// - the **tooltip** is taken away when it is empty;
/// - the **options** are a list of texts, and a choice may not be
///   left with no option at all;
/// - the **alignment** is 0 (left), 1 (centre) or 2 (right);
/// - the **multiline** flag is the flag of a text field, and a field
///   that is not a text field refuses it;
/// - the **maximum length** is a whole number of characters, and zero
///   means that there is no limit.
#[derive(Command)]
#[command(id = "pdf.field.properties.set", area = "pdf", layer = "pdf", icon = "pencil", mutates, undoable)]
pub struct SetProperties {
    /// The field.
    pub field: NodeRef,
    /// The **name** of the field: the part it has of its own.
    pub name: Option<Text>,
    /// The **text** shown when the reader rests on the field.
    ///
    /// An empty text takes the tooltip away.
    pub tooltip: Option<Text>,
    /// The **most characters** the field holds.
    ///
    /// Zero means that there is no limit.
    pub max_len: Option<Number>,
    /// The **options** the field offers: a list of texts.
    pub options: Option<serde_json::Value>,
    /// Where the **text** of the field sits: 0 (left), 1 (centre) or 2
    /// (right).
    pub quadding: Option<Number>,
    /// The **value** the reset of the form restores.
    ///
    /// An empty text takes it away.
    pub default_value: Option<Text>,
    /// Whether the **properties** of the field may be changed.
    pub locked: Option<Bool>,
    /// Whether the field holds **more than one line** of text.
    ///
    /// A field that is not a text field refuses it.
    pub multiline: Option<Bool>,
}

impl SetProperties {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let field = self.field.0;
        let Some(node) = session.document().node(field).cloned() else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("there is no node `{field}`"),
            });
        };
        if crate::fields::field_of(session.document(), &node).is_none() {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("the node `{field}` is not a field"),
            });
        }
        // A **locked** field takes only unlocking.
        if is_locked(session.document(), &node) && self.locked != Some(Bool(false)) {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "the field is locked: unlock it first".to_string(),
            });
        }
        // The **name**: the last part of the name the field has.
        let mut new_name = None;
        if let Some(text) = &self.name {
            let name = text.0.trim().to_string();
            if name.is_empty() || name.contains('.') {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "field names can't be empty or contain a period".to_string(),
                });
            }
            let taken = fields_of(session.document()).into_iter().any(|field| field.name == name);
            if taken {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: format!("a field named `{name}` already exists"),
                });
            }
            new_name = Some(name);
        }
        // The **options**: a list of texts, and a choice may not be
        // left with no option at all.
        let options = match &self.options {
            Some(value) => {
                let options = the_options(value)?;
                if options.is_empty()
                    && matches!(
                        crate::fields::field_of(session.document(), &node).map(|found| found.kind),
                        Some(crate::fields::FieldKind::Choice)
                    )
                {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "a list needs at least one option".to_string(),
                    });
                }
                Some(options)
            }
            None => None,
        };
        // The **alignment**: 0, 1 or 2.
        let quadding = match self.quadding.as_ref().map(|value| value.0) {
            Some(value) => {
                if !value.is_finite() || value.fract() != 0.0 || !(0.0..=2.0).contains(&value) {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "alignment is 0 (left), 1 (centre) or 2 (right)".to_string(),
                    });
                }
                Some(value as i64)
            }
            None => None,
        };
        // The **multiline** flag is the flag of a text field.
        let multiline = match self.multiline.as_ref().map(|flag| flag.0) {
            Some(flag) => {
                let kind = crate::fields::field_of(session.document(), &node).map(|found| found.kind);
                if kind != Some(crate::fields::FieldKind::Text) {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "only a text field holds more than one line of text".to_string(),
                    });
                }
                Some(flag)
            }
            None => None,
        };
        // The **maximum length**: a whole number of characters, and
        // zero means that there is no limit.
        let max_len = match self.max_len.as_ref().map(|value| value.0) {
            Some(value) => {
                if !value.is_finite() || value < 0.0 || value.fract() != 0.0 {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "the most characters a field holds is a whole number".to_string(),
                    });
                }
                Some(value as u64)
            }
            None => None,
        };
        // The **entries** of the field, with the properties put in.
        let mut entries = entries_of(&node);
        let put = |entries: &mut Vec<(String, Json)>, key: &str, value: Json| match entries
            .iter_mut()
            .find(|(name, _)| name == key)
        {
            Some(entry) => entry.1 = value,
            None => entries.push((key.to_string(), value)),
        };
        let take = |entries: &mut Vec<(String, Json)>, key: &str| {
            entries.retain(|(name, _)| name != key);
        };
        if let Some(name) = &new_name {
            put(&mut entries, "T", json!({ "string": name.clone() }));
        }
        if let Some(tooltip) = &self.tooltip {
            if tooltip.0.is_empty() {
                take(&mut entries, "TU");
            } else {
                put(&mut entries, "TU", json!({ "string": tooltip.0.clone() }));
            }
        }
        if let Some(max_len) = max_len {
            if max_len == 0 {
                take(&mut entries, "MaxLen");
            } else {
                put(&mut entries, "MaxLen", json!(max_len));
            }
        }
        if let Some(options) = &options {
            put(
                &mut entries,
                "Opt",
                json!(options.iter().map(|option| json!({ "string": option })).collect::<Vec<_>>()),
            );
        }
        if let Some(quadding) = quadding {
            put(&mut entries, "Q", json!(quadding));
        }
        if let Some(default_value) = &self.default_value {
            if default_value.0.is_empty() {
                take(&mut entries, "DV");
            } else {
                put(&mut entries, "DV", json!({ "string": default_value.0.clone() }));
            }
        }
        if let Some(multiline) = multiline {
            let mut flags = entry(&entries, "Ff").and_then(|flags| flags.as_f64()).unwrap_or(0.0) as i64;
            if multiline {
                flags |= MULTILINE;
            } else {
                flags &= !MULTILINE;
            }
            put(&mut entries, "Ff", json!(flags));
        }
        // The **lock** is on the widgets of the field — and a field
        // that is its own widget is locked itself.
        let mut locked = None;
        if let Some(lock) = self.locked.map(|lock| lock.0) {
            // The **widgets** of the field: the kids that have no name
            // of their own.
            let kids = entry(&entries, "Kids").and_then(|kids| kids.as_array().cloned()).unwrap_or_default();
            let mut widgets: Vec<u64> = Vec::new();
            for kid in &kids {
                let Some(number) = crate::number_of(kid) else { continue };
                let Some(kid_node) = node_of_number(session.document(), number) else { continue };
                if entry(&entries_of(kid_node), "T").is_some() {
                    continue;
                }
                widgets.push(number);
            }
            if widgets.is_empty() {
                // The field **is** its own widget: the lock is in its
                // own dictionary.
                let mut flags = flags_of(&node);
                if lock {
                    flags |= LOCKED;
                } else {
                    flags &= !LOCKED;
                }
                put(&mut entries, "F", json!(flags));
            }
            for widget in widgets {
                let Some(widget_node) = node_of_number(session.document(), widget) else { continue };
                let id = widget_node.id;
                let mut widget_entries = entries_of(widget_node);
                let mut flags = flags_of(widget_node);
                if lock {
                    flags |= LOCKED;
                } else {
                    flags &= !LOCKED;
                }
                put(&mut widget_entries, "F", json!(flags));
                let data = json!({
                    "object": widget_node.data.get("object").cloned().unwrap_or(Json::Null),
                    "entries": widget_entries.iter().map(|(key, value)| json!([key, value])).collect::<Vec<_>>(),
                });
                if let Some(node) = session.document_mut()?.node_mut(id) {
                    node.data = data;
                }
            }
            locked = Some(lock);
        }
        // The **field** itself.
        let data = json!({
            "object": node.data.get("object").cloned().unwrap_or(Json::Null),
            "entries": entries.iter().map(|(key, value)| json!([key, value])).collect::<Vec<_>>(),
        });
        if let Some(node) = session.document_mut()?.node_mut(field) {
            node.data = data;
        }
        Ok(Outcome::changed(format!("the field `{field}` has its properties")).with_data(json!({
            "field": field,
            "name": new_name,
            "tooltip": self.tooltip.as_ref().map(|tooltip| tooltip.0.clone()),
            "max_len": max_len,
            "options": options,
            "quadding": quadding,
            "default_value": self.default_value.as_ref().map(|value| value.0.clone()),
            "locked": locked,
            "multiline": multiline,
        })))
    }
}
