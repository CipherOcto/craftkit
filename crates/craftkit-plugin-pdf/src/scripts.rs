//! The **scripts** of a field.
//!
//! A field may carry small scripts: the file holds them on the field's
//! **additional actions** — the `/AA` dictionary — and each event of
//! that dictionary holds one:
//!
//! - `K` — the **keystroke** event;
//! - `F` — the **format** event, how the field's text is shown;
//! - `V` — the **validate** event, what the field accepts;
//! - `C` — the **calculate** event, how the field's value is worked
//!   out.
//!
//! The scripts are **JavaScript**, and this tree reads the few calls it
//! knows and does not pretend to run the rest. The reading of a call's
//! **arguments** is here, because every one of those scripts is a call
//! with arguments.
//!
//! The rules:
//!
//! - an action may be **named** by a reference or **written** where it
//!   is used, and both are read the same way;
//! - a script this tree does not know is **kept as it is** and
//!   reported as unsupported.

use craftkit_sdk::serde_json::Value as Json;
use craftkit_sdk::{Document, Node};

/// The **arguments** of a call: the text of each one.
pub fn arguments_of(script: &str, name: &str) -> Option<Vec<String>> {
    let at = script.find(name)?;
    let open = script[at..].find('(')? + at;
    let mut depth = 0usize;
    let mut quoted = false;
    let mut escaped = false;
    for (index, character) in script[open..].char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match character {
            '\\' if quoted => escaped = true,
            '"' => quoted = !quoted,
            '(' if !quoted => depth += 1,
            ')' if !quoted => {
                depth -= 1;
                if depth == 0 {
                    let inside = &script[open + 1..open + index];
                    return Some(split_arguments(inside));
                }
            }
            _ => {}
        }
    }
    None
}

/// The **arguments** of a call, split at the commas that are not
/// inside a string or a bracket.
pub fn split_arguments(text: &str) -> Vec<String> {
    let mut arguments: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    let mut quoted = false;
    let mut escaped = false;
    for character in text.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            continue;
        }
        match character {
            '\\' if quoted => {
                current.push(character);
                escaped = true;
            }
            '"' => {
                quoted = !quoted;
                current.push(character);
            }
            '(' | '[' if !quoted => {
                depth += 1;
                current.push(character);
            }
            ')' | ']' if !quoted => {
                depth = depth.saturating_sub(1);
                current.push(character);
            }
            ',' if !quoted && depth == 0 => {
                arguments.push(current.trim().to_string());
                current = String::new();
            }
            _ => current.push(character),
        }
    }
    if !current.trim().is_empty() {
        arguments.push(current.trim().to_string());
    }
    arguments
}

/// The **text** a string holds: the file's own spelling of it.
pub fn text_of(argument: &str) -> Option<String> {
    let trimmed = argument.trim();
    if !trimmed.starts_with('"') || !trimmed.ends_with('"') || trimmed.len() < 2 {
        return None;
    }
    let inside = &trimmed[1..trimmed.len() - 1];
    let mut text = String::new();
    let mut escaped = false;
    for character in inside.chars() {
        if escaped {
            match character {
                'n' => text.push('\n'),
                'r' => text.push('\r'),
                't' => text.push('\t'),
                other => text.push(other),
            }
            escaped = false;
            continue;
        }
        if character == '\\' {
            escaped = true;
            continue;
        }
        text.push(character);
    }
    Some(text)
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
fn text_of_value(value: &Json) -> Option<String> {
    match value {
        Json::String(text) => Some(text.clone()),
        Json::Object(map) => map.get("string").and_then(|text| text.as_str()).map(|text| text.to_string()),
        _ => None,
    }
}

/// The **node** an object number names.
fn node_of_number(document: &Document, number: u64) -> Option<&Node> {
    document.walk().find(|node| node.data.get("object").and_then(|found| found.as_u64()) == Some(number))
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

/// The **script** of a field: the text of the action its event names.
///
/// The event is the key the additional actions hold it under — `K`,
/// `F`, `V` or `C`.
pub fn script_of(document: &Document, node: &Node, event: &str) -> Option<String> {
    let entries = entries_of(node);
    let actions = entry(&entries, "AA")?;
    let actions = actions.get("entries")?.as_array()?;
    let chosen = actions.iter().find_map(|pair| {
        let pair = pair.as_array()?;
        if pair.first()?.as_str()? != event {
            return None;
        }
        pair.get(1)
    })?;
    // The action may be the dictionary itself, or a reference to it.
    let action = match crate::number_of(chosen) {
        Some(number) => {
            let action = node_of_number(document, number)?;
            entries_of(action)
        }
        None => dictionary_of(chosen)?,
    };
    let script = entry(&action, "JS")?;
    text_of_value(script)
}
