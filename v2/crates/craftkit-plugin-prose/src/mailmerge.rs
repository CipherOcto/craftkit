//! The **mail merge** of a document: the fields of a document filled
//! with the rows of a data source.
//!
//! The tree's own: a **merge field** is a field whose instruction is
//! `MERGEFIELD`, the name of the column follows it, and the value of
//! the column is what the field holds.
//!
//! # The rules
//!
//! - The **name** of a merge field is the word after `MERGEFIELD` in
//!   the instruction of the field, and the word may be in **quotes**.
//!   The name of the instruction is **not** case-sensitive, and the
//!   name of the column is not either.
//! - A column that the data source does not have is an **empty**
//!   value: the tree's rule, and the rule here.
//! - The **address block** is the tree's own: the title, the first
//!   name, the last name, the company, the address, the city, the
//!   state, the postal code and the country — the parts that the
//!   columns hold, in that order, and the empty parts are left out.
//! - The columns are matched by their **names**, and the tree's map
//!   of the parts of an address is the tree's own: `FirstName`,
//!   `First` and `GivenName` are the first name; `LastName`, `Last`
//!   and `Surname` are the last name; and so on.
//!
//! # What it does not settle
//!
//! - The merge does not write a **document**: it says the text of
//!   every row, and the tree writes a document of its own.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Outcome, Session};

/// The name of the command, in the messages.
const COMMAND: &str = "prose.merge";

/// The **name** of a merge field, from the instruction of the field.
///
/// The tree's own rule: the kind of the field ends at a space or at a
/// quote, so `MERGEFIELDX` is another field, and the name that
/// follows may be in quotes.
pub fn mergefield_name(code: &str) -> Option<String> {
    let text = code.trim();
    let end =
        text.find(|character: char| character.is_whitespace() || character == '"').unwrap_or(text.len());
    let (kind, rest) = text.split_at(end);
    if !kind.eq_ignore_ascii_case("MERGEFIELD") {
        return None;
    }
    let rest = rest.trim();
    let name = match rest.strip_prefix('"') {
        Some(quoted) => quoted.split('"').next().unwrap_or(""),
        None => rest.split_whitespace().next().unwrap_or(""),
    };
    Some(name.to_string())
}

/// The **text** of a value of the data source.
fn text_of(value: &Json) -> String {
    match value {
        Json::String(text) => text.clone(),
        Json::Null => String::new(),
        other => other.to_string(),
    }
}

/// The **value** of a column in a row.
fn value_of(headers: &[String], row: &[Json], name: &str) -> String {
    let Some(index) = headers.iter().position(|held| held.eq_ignore_ascii_case(name)) else {
        return String::new();
    };
    row.get(index).map(text_of).unwrap_or_default()
}

/// The **columns** that hold the parts of an address: the tree's own
/// map.
pub fn address_map(headers: &[String]) -> Json {
    let find = |names: &[&str]| {
        headers
            .iter()
            .find(|held| names.iter().any(|name| held.replace([' ', '_'], "").eq_ignore_ascii_case(name)))
            .cloned()
    };
    json!({
        "firstName": find(&["FirstName", "First", "GivenName"]),
        "lastName": find(&["LastName", "Last", "Surname", "FamilyName"]),
        "company": find(&["Company", "Organization", "CompanyName"]),
        "address": find(&["Address", "AddressLine1", "Address1", "Street"]),
        "city": find(&["City", "Town"]),
        "state": find(&["State", "Province", "Region"]),
        "postal": find(&["PostalCode", "ZIP", "ZipCode", "Postcode"]),
        "country": find(&["Country"]),
        "title": find(&["Title", "CourtesyTitle"]),
    })
}

/// The **address block** of a row: the tree's own order, and the empty
/// parts are left out.
pub fn address_block(headers: &[String], row: &[Json]) -> String {
    let map = address_map(headers);
    let part = |name: &str| {
        map.get(name).and_then(Json::as_str).map(|column| value_of(headers, row, column)).unwrap_or_default()
    };
    let name = [part("title"), part("firstName"), part("lastName")]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let city_line = [
        part("city"),
        [part("state"), part("postal")]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" "),
    ]
    .into_iter()
    .filter(|part| !part.is_empty())
    .collect::<Vec<_>>()
    .join(", ");
    [name, part("company"), part("address"), city_line, part("country")]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The **text** of a block with its merge fields filled in.
fn merged_text(node: &craftkit_sdk::Node, headers: &[String], row: &[Json]) -> String {
    let text = node.data.get("text").and_then(|text| text.as_str()).unwrap_or_default();
    let characters: Vec<char> = text.chars().collect();
    // The **fields**, from the last to the first: the offsets of the
    // fields that follow are not moved by the ones before them.
    let mut fields: Vec<(usize, usize, String)> = crate::fields::fields_of(node)
        .iter()
        .filter_map(|field| {
            let code = field.get("code").and_then(Json::as_str)?;
            let name = mergefield_name(code)?;
            let start = field.get("start").and_then(Json::as_u64).unwrap_or(0) as usize;
            let end = field.get("end").and_then(Json::as_u64).unwrap_or(0) as usize;
            Some((start.min(characters.len()), end.min(characters.len()).max(start), name))
        })
        .collect();
    fields.sort_by_key(|field| std::cmp::Reverse(field.0));
    let mut out: Vec<char> = characters;
    for (start, end, name) in fields {
        let value = value_of(headers, row, &name);
        let value: Vec<char> = value.chars().collect();
        out.splice(start..end, value);
    }
    out.into_iter().collect()
}

/// **Merge** the fields of a document with the rows of a data source.
///
/// The tree's own: a merge field is a field whose instruction is
/// `MERGEFIELD`, and its value is the value of the column of that
/// name.
#[derive(Command)]
#[command(id = "prose.merge", area = "prose", layer = "prose", icon = "list")]
pub struct Merge {
    /// The names of the columns of the data source.
    pub headers: Option<Json>,
    /// The rows of the data source: a list of lists of values.
    pub rows: Json,
}

impl Merge {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let headers: Vec<String> = match &self.headers {
            Some(headers) => {
                let Some(list) = headers.as_array() else {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "the names of the columns are a list".to_string(),
                    });
                };
                list.iter().map(|header| header.as_str().map(str::to_string).unwrap_or_default()).collect()
            }
            None => Vec::new(),
        };
        let Some(rows) = self.rows.as_array() else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "the rows of the data source are a list of lists".to_string(),
            });
        };
        // The **blocks** of the document.
        let blocks: Vec<u64> = session
            .document()
            .walk()
            .filter(|node| node.kind.starts_with("prose.block."))
            .map(|node| node.id)
            .collect();
        let mut merged: Vec<Json> = Vec::new();
        for (index, row) in rows.iter().enumerate() {
            let Some(row) = row.as_array() else {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "the rows of the data source are a list of lists".to_string(),
                });
            };
            let texts: Vec<Json> = blocks
                .iter()
                .filter_map(|block| session.document().node(*block))
                .map(|node| {
                    let text = merged_text(node, &headers, row);
                    json!({ "block": node.id, "kind": node.kind.clone(), "text": text })
                })
                .collect();
            merged.push(json!({
                "row": index + 1,
                "blocks": texts,
                "address": address_block(&headers, row),
            }));
        }
        // The **fields** the document holds that are merge fields: the
        // tree says which they are, and so does this one.
        let mut names: Vec<String> = Vec::new();
        for block in blocks {
            let Some(node) = session.document().node(block) else { continue };
            for field in crate::fields::fields_of(node) {
                let Some(code) = field.get("code").and_then(Json::as_str) else { continue };
                let Some(name) = mergefield_name(code) else { continue };
                if !names.contains(&name) {
                    names.push(name);
                }
            }
        }
        Ok(Outcome::new(format!("{} row(s) merged", merged.len())).with_data(json!({
            "headers": headers,
            "rows": merged,
            "count": merged.len(),
            "fields": names,
        })))
    }
}
