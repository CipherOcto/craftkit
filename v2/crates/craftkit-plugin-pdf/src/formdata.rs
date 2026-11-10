//! The **data of a form**: the values of its fields, written as a file
//! and read back from one.
//!
//! The tree's own formats: an **XFDF** file — the XML of Adobe's
//! interchange format — and a **table** of the values.
//!
//! # The rules
//!
//! - The fields of a form are the fields of its **AcroForm**, and the
//!   **qualified name** of a field is the name of it and the names of
//!   its parents, joined with a dot.
//! - The **XFDF** file is a **tree**: the name of a field is split at
//!   its dots, and every part is a field of its own, so that a file
//!   says `who.first` as a field of a field.
//! - The **table** of the values is two lines: the names of the fields
//!   and their values, and a value that holds a comma or a quote is
//!   **quoted**.
//! - A file that is **not one** is refused, and a field that the file
//!   names and the form does not have is **left out**: the tree's own
//!   rule is that the file is read and what it does not match is not
//!   written.
//!
//! # What it does not settle
//!
//! - The **comments** of the tree's XFDF are not here: this carries the
//!   values of the fields and not the annotations.
//! - The **FDF file** of the tree is here and is **written**: it is a
//!   file of PDF syntax, and this tree writes files of PDF syntax
//!   (`src/fdf.rs`). It is not **read** back yet.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Node, Outcome, Session, Text};

/// The **formats** the data may be written as.
pub const FORMATS: [&str; 4] = ["xfdf", "xml", "csv", "fdf"];

/// The **name** of the command, in the messages.
const COMMAND: &str = "pdf.form.data.export";

/// The most **characters** a file of data may hold.
const MAX_CHARS: usize = 8 * 1024 * 1024;

/// A **field** of a form, with the value it holds.
#[derive(Debug, Clone)]
pub struct Entry {
    /// The qualified name of the field.
    pub name: String,
    /// The value it holds.
    pub value: Option<String>,
}

/// The **fields** of the form, with the values they hold.
pub fn data_of(document: &Document) -> Vec<Entry> {
    crate::fields::fields_of(document)
        .into_iter()
        .map(|field| Entry { name: field.name, value: field.value })
        .collect()
}

/// The **escape** of the text of an XML file: the tree's own rule.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\r' => out.push_str("&#13;"),
            character if (character as u32) < 0x20 && character != '\n' && character != '\t' => {}
            character => out.push(character),
        }
    }
    out
}

/// A **node** of the tree of the fields of an XFDF file.
#[derive(Debug, Default)]
struct FieldNode {
    values: Vec<String>,
    kids: Vec<(String, FieldNode)>,
}

impl FieldNode {
    /// The **child** of that name, and the child is made when it is not
    /// there.
    fn child(&mut self, name: &str) -> &mut FieldNode {
        let index = match self.kids.iter().position(|(held, _)| held == name) {
            Some(index) => index,
            None => {
                self.kids.push((name.to_string(), FieldNode::default()));
                self.kids.len() - 1
            }
        };
        &mut self.kids[index].1
    }
}

/// The **XFDF** file of the data.
///
/// The tree's own rule: the names of the fields are split at their
/// dots, and every part is a field of its own.
pub fn xfdf_of(entries: &[Entry], file: &str) -> String {
    let mut root = FieldNode::default();
    for entry in entries {
        let mut node = &mut root;
        for part in entry.name.split('.') {
            node = node.child(part);
        }
        node.values = match &entry.value {
            Some(value) => vec![value.clone()],
            None => Vec::new(),
        };
    }
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<xfdf xmlns=\"http://ns.adobe.com/xfdf/\" xml:space=\"preserve\">\n");
    out.push_str("<fields>\n");
    for (name, node) in &root.kids {
        write_node(name, node, &mut out);
    }
    out.push_str("</fields>\n");
    out.push_str(&format!("<f href=\"{}\"/>\n", escape(file)));
    out.push_str("</xfdf>\n");
    out
}

/// Write a **field** of the XFDF file, and the fields under it.
fn write_node(name: &str, node: &FieldNode, out: &mut String) {
    out.push_str(&format!("<field name=\"{}\">\n", escape(name)));
    for value in &node.values {
        out.push_str(&format!("<value>{}</value>\n", escape(value)));
    }
    for (kid, child) in &node.kids {
        write_node(kid, child, out);
    }
    out.push_str("</field>\n");
}

/// The **XML** table of the data: the tree's own format.
pub fn xml_of(entries: &[Entry]) -> String {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<fields xmlns:xfdf=\"http://ns.adobe.com/xfdf-transition/\">\n");
    for entry in entries {
        let value = entry.value.clone().unwrap_or_default();
        out.push_str(&format!("<{}>{}</{}>\n", xml_name(&entry.name), escape(&value), xml_name(&entry.name)));
    }
    out.push_str("</fields>\n");
    out
}

/// The **name** of an element for a field name: the tree's own rule —
/// letters, digits, `_`, `-` and `.`, and a name that does not begin
/// with a letter begins with an underscore.
fn xml_name(name: &str) -> String {
    let mut out: String = name
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || matches!(character, '_' | '-' | '.') { character } else { '_' }
        })
        .collect();
    if !out.chars().next().is_some_and(|character| character.is_alphabetic() || character == '_') {
        out.insert(0, '_');
    }
    out
}

/// The **CSV** table of the data: the tree's own rule.
pub fn csv_of(entries: &[Entry]) -> String {
    let quote = |text: &str| {
        if text.contains([',', '"', '\n']) {
            format!("\"{}\"", text.replace('"', "\"\""))
        } else {
            text.to_string()
        }
    };
    let names: Vec<String> = entries.iter().map(|entry| quote(&entry.name)).collect();
    let values: Vec<String> =
        entries.iter().map(|entry| quote(entry.value.as_deref().unwrap_or(""))).collect();
    format!("{}\n{}\n", names.join(","), values.join(","))
}

/// The **values** of an XFDF file: the qualified name of each field and
/// the values it holds.
///
/// The tree's own rule: the names of the fields are the names of the
/// elements, and the name of a field of a field is the two joined with
/// a dot.
pub fn values_of(text: &str) -> Result<Vec<(String, Vec<String>)>, String> {
    let root = craftkit_sdk::xml::parse(text)?;
    let Some(fields) = root.child("fields") else {
        return Err("the file has no fields in it".to_string());
    };
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    read_fields(fields, "", &mut out);
    Ok(out)
}

/// The **fields** of an element of the XFDF file.
fn read_fields(element: &craftkit_sdk::xml::Element, prefix: &str, out: &mut Vec<(String, Vec<String>)>) {
    for child in &element.children {
        if child.name != "field" {
            continue;
        }
        let name = child.attribute("name").unwrap_or_default();
        let qualified = if prefix.is_empty() { name.to_string() } else { format!("{prefix}.{name}") };
        let values: Vec<String> = child
            .children
            .iter()
            .filter(|value| value.name == "value")
            .map(|value| value.text.clone())
            .collect();
        out.push((qualified.clone(), values));
        read_fields(child, &qualified, out);
    }
}

/// **Set** the value of a field.
///
/// The tree's own rules: the value must be one the field **can hold**,
/// and an empty value **takes the value away**.
fn set_value(session: &mut Session, node: &Node, value: &str) -> Result<(), String> {
    let document = session.document();
    let found = crate::fields::field_of(document, node).ok_or("the node is not a field")?;
    // A **button** that is **off** holds no state: the tree's own rule.
    let off = found.kind == crate::fields::FieldKind::Button && value == "Off";
    match found.kind {
        crate::fields::FieldKind::Text => {}
        crate::fields::FieldKind::Choice => {
            if !value.is_empty() && !found.options.iter().any(|option| option == value) {
                return Err(format!("`{value}` is not one of the options of the field"));
            }
        }
        crate::fields::FieldKind::Signature => {}
        crate::fields::FieldKind::Button => {
            let states = crate::fields::states_of(document, node);
            if !value.is_empty() && !off && !states.iter().any(|state| state == value) {
                return Err(format!("`{value}` is not one of the states of the field"));
            }
        }
    }
    let mut entries = crate::entries_of(&node.data);
    if value.is_empty() || off {
        entries.retain(|(key, _)| key != "V");
    } else {
        // The **value** of a **button** is a **state**, and a state is a
        // **name**; every other field holds a **string**.
        let held = match found.kind {
            crate::fields::FieldKind::Button => Json::String(value.to_string()),
            _ => json!({ "string": value }),
        };
        match entries.iter_mut().find(|(key, _)| key == "V") {
            Some(entry) => entry.1 = held,
            None => entries.push(("V".to_string(), held)),
        }
    }
    let data = json!({
        "object": node.data.get("object").cloned().unwrap_or(Json::Null),
        "entries": entries.iter().map(|(key, value)| json!([key, value])).collect::<Vec<_>>(),
    });
    let id = node.id;
    let Some(node) = session.document_mut().map_err(|error| error.to_string())?.node_mut(id) else {
        return Err(format!("there is no node `{id}`"));
    };
    node.data = data;
    Ok(())
}

/// The **fields** of a file: the tree's own rule — a field of the form
/// that the file names.
fn field_node(document: &Document, name: &str) -> Option<Node> {
    let field = crate::fields::fields_of(document).into_iter().find(|field| field.name == name)?;
    document
        .walk()
        .find(|node| node.data.get("object").and_then(|number| number.as_u64()) == Some(field.object))
        .cloned()
}

/// **Write** the **data of a form** as a file.
///
/// The data is **derived**: it is read from the form every time it is
/// asked for.
#[derive(Command)]
#[command(id = "pdf.form.data.export", area = "pdf", layer = "pdf", icon = "file", mutates, undoable)]
pub struct ExportFormData {
    /// The file to write.
    pub path: Text,
    /// The format of the file: `xfdf` when it is not given.
    pub format: Option<Text>,
    /// The file the data came from: it is written in the XFDF file.
    pub file: Option<Text>,
}

impl ExportFormData {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = COMMAND.to_string();
        let path = self.path.0.trim().to_string();
        if path.is_empty() {
            return Err(EngineError::Failed { id: id.clone(), message: "a file has a path".to_string() });
        }
        let named = self.format.as_ref().map(|format| format.0.trim().to_ascii_lowercase());
        let format = match named.as_deref() {
            None | Some("") | Some("xfdf") => "xfdf",
            Some("xml") => "xml",
            Some("csv") => "csv",
            Some("fdf") => "fdf",
            Some(other) => {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "`{other}` is not a format of the data of a form: the formats are {}",
                        FORMATS.join(", ")
                    ),
                });
            }
        };
        let entries = data_of(session.document());
        let file = self.file.as_ref().map(|file| file.0.trim().to_string()).unwrap_or_default();
        let (written, count) = match format {
            "xml" => (xml_of(&entries).into_bytes(), entries.len()),
            "csv" => (csv_of(&entries).into_bytes(), entries.len()),
            "fdf" => {
                let fields = crate::fields::fields_of(session.document());
                crate::fdf::fdf_of(&fields, &file)
            }
            _ => (xfdf_of(&entries, &file).into_bytes(), entries.len()),
        };
        std::fs::write(&path, &written).map_err(|error| EngineError::Failed {
            id: id.clone(),
            message: format!("the file cannot be written: {error}"),
        })?;
        Ok(Outcome::changed(format!("`{path}` was written with {count} field(s)")).with_data(json!({
            "path": path,
            "format": format,
            "count": count,
            "bytes": written.len(),
        })))
    }
}

/// **Read** the **data of a form** from a file, and write it into the
/// form.
///
/// The tree's own rule: a field the file names and the form does not
/// have is **left out**, and a field that takes its value is counted.
#[derive(Command)]
#[command(id = "pdf.form.data.import", area = "pdf", layer = "pdf", icon = "file", mutates, undoable)]
pub struct ImportFormData {
    /// The file to read.
    pub path: Text,
}

impl ImportFormData {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "pdf.form.data.import".to_string();
        let path = self.path.0.trim().to_string();
        if path.is_empty() {
            return Err(EngineError::Failed { id: id.clone(), message: "a file has a path".to_string() });
        }
        let metadata = std::fs::metadata(&path).map_err(|error| EngineError::Failed {
            id: id.clone(),
            message: format!("the file cannot be read: {error}"),
        })?;
        if metadata.len() as usize > MAX_CHARS {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the file is more than {MAX_CHARS} bytes long"),
            });
        }
        let bytes = std::fs::read(&path).map_err(|error| EngineError::Failed {
            id: id.clone(),
            message: format!("the file cannot be read: {error}"),
        })?;
        // The **FDF file** is a file of PDF syntax, and the rest are
        // files of text: the file says which it is.
        let values: Vec<(String, Vec<String>)> = if bytes.starts_with(b"%FDF") {
            crate::fdf::values_of(&bytes)
                .map_err(|message| EngineError::Failed { id: id.clone(), message })?
                .into_iter()
                .map(|(name, value)| (name, value.into_iter().collect()))
                .collect()
        } else {
            let text = String::from_utf8_lossy(&bytes).to_string();
            values_of(&text).map_err(|message| EngineError::Failed { id: id.clone(), message })?
        };
        let mut taken = 0usize;
        let mut left_out: Vec<String> = Vec::new();
        for (name, values) in values {
            let Some(node) = field_node(session.document(), &name) else {
                left_out.push(name);
                continue;
            };
            let value = values.first().cloned().unwrap_or_default();
            match set_value(session, &node, &value) {
                Ok(()) => taken += 1,
                Err(_) => left_out.push(name),
            }
        }
        Ok(Outcome::changed(format!("{taken} field(s) took their value")).with_data(json!({
            "path": path,
            "count": taken,
            "left_out": left_out,
        })))
    }
}
