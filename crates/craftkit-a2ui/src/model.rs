//! The **data model** — a2ui's own tree, addressed by JSON-Pointer-ish
//! paths.
//!
//! # What this is
//!
//! a2ui's protocol says that the **local data model is the single source
//! of truth** for a binding (`specification/v1_0/docs/a2ui_protocol.md:999`).
//! So the layer holds a tree of values, and a binding — `{"@path":
//! "/title"}` — is a **path** into it.
//!
//! # What it is not
//!
//! It is **not** a copy of craftkit's document. craftkit's document is
//! craftkit's own state — the geometry, the commands — and the layer
//! **mirrors** a value into it by issuing a **command**. The mirrored
//! value is an *effect*, and it is never read as authority.
//!
//! # The paths
//!
//! A path is a **JSON Pointer** (`/title`, `/user/name`), which is a
//! string of `/`-separated steps, each one an object key or an array
//! index. The steps are **decoded** the way RFC 6901 says: `~1` is `/`
//! and `~0` is `~`.

use craftkit_sdk::serde_json::{Map as JsonMap, Value as Json};

/// The layer's data model: a tree of values, addressed by path.
#[derive(Debug, Clone, Default)]
pub struct Model {
    /// The tree's root.
    root: Json,
}

impl Model {
    /// An empty model.
    pub fn new() -> Self {
        Model { root: Json::Object(JsonMap::<String, Json>::new()) }
    }

    /// The whole tree, as it is.
    pub fn root(&self) -> &Json {
        &self.root
    }

    /// The value at a **path**, if there is one.
    ///
    /// A path that is not there is `None` — and the caller **refuses** it,
    /// because a binding to a missing path is a missing value and the
    /// house rule is that an unknown name is refused.
    pub fn get(&self, path: &str) -> Option<&Json> {
        let mut current = &self.root;
        for step in steps_of(path) {
            current = match current {
                Json::Object(map) => map.get(&step)?,
                Json::Array(items) => {
                    let index: usize = step.parse().ok()?;
                    items.get(index)?
                }
                _ => return None,
            };
        }
        Some(current)
    }

    /// The value at a path, as a **string**.
    ///
    /// A string is a string; a number and a boolean are **their own
    /// text**; anything else is `None`, because there is no honest text
    /// for an object.
    pub fn get_text(&self, path: &str) -> Option<String> {
        match self.get(path)? {
            Json::String(text) => Some(text.clone()),
            Json::Number(number) => Some(number.to_string()),
            Json::Bool(value) => Some(value.to_string()),
            _ => None,
        }
    }

    /// The value at a path, as a **boolean**.
    pub fn get_bool(&self, path: &str) -> Option<bool> {
        match self.get(path)? {
            Json::Bool(value) => Some(*value),
            Json::String(text) => match text.as_str() {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            },
            _ => None,
        }
    }

    /// The value at a path, as a **number**.
    pub fn get_number(&self, path: &str) -> Option<f64> {
        match self.get(path)? {
            Json::Number(number) => number.as_f64(),
            Json::String(text) => text.parse().ok(),
            _ => None,
        }
    }

    /// **Write** the value at a path, creating what is missing.
    ///
    /// This is the model only — the **mirror** into craftkit is a
    /// command, and the caller issues it.
    ///
    /// A path that cannot be written — a step through a **scalar**, say —
    /// is an error: the model is not re-shaped to fit a path.
    pub fn set(&mut self, path: &str, value: Json) -> Result<(), String> {
        let steps = steps_of(path);
        if steps.is_empty() {
            self.root = value;
            return Ok(());
        }
        let mut current = &mut self.root;
        for step in &steps[..steps.len() - 1] {
            current = match current {
                Json::Object(map) => {
                    map.entry(step.clone()).or_insert_with(|| Json::Object(JsonMap::<String, Json>::new()))
                }
                Json::Array(items) => {
                    let index: usize = step.parse().map_err(|_| {
                        format!("the path `{path}` steps through a list by a name that is not a number")
                    })?;
                    let length = items.len();
                    items.get_mut(index).ok_or_else(|| {
                        format!("the path `{path}` steps through a list at {index}, and it has {length}")
                    })?
                }
                _ => {
                    return Err(format!("the path `{path}` steps through a value that is not a tree"));
                }
            };
        }
        let last = &steps[steps.len() - 1];
        match current {
            Json::Object(map) => {
                map.insert(last.clone(), value);
                Ok(())
            }
            Json::Array(items) => {
                let index: usize = last
                    .parse()
                    .map_err(|_| format!("the path `{path}` names a list entry that is not a number"))?;
                let length = items.len();
                match items.get_mut(index) {
                    Some(slot) => {
                        *slot = value;
                        Ok(())
                    }
                    None if index == length => {
                        // The **end** of the list: the value is appended.
                        items.push(value);
                        Ok(())
                    }
                    None => Err(format!(
                        "the path `{path}` names a list entry at {index}, and the list has {length}"
                    )),
                }
            }
            _ => Err(format!("the path `{path}` steps through a value that is not a tree")),
        }
    }
}

/// The **steps** of a path.
///
/// A JSON Pointer: a string of `/`-separated steps, each one decoded the
/// way RFC 6901 says — `~1` is `/` and `~0` is `~`.
fn steps_of(path: &str) -> Vec<String> {
    if path.is_empty() {
        return Vec::new();
    }
    let trimmed = path.strip_prefix('/').unwrap_or(path);
    if trimmed.is_empty() {
        return Vec::new();
    }
    trimmed.split('/').map(|step| step.replace("~1", "/").replace("~0", "~")).collect()
}

/// Whether a path is **well formed**.
///
/// A path is a JSON Pointer, and a malformed one is refused before it is
/// used: a path that is not a path would silently address nothing.
pub fn is_path(path: &str) -> bool {
    // A path is well formed when it is empty (the root) or it starts with
    // a `/`. Everything else — a step with no separator, a `~` that is
    // not followed by `0` or `1` — is refused.
    if path.is_empty() {
        return true;
    }
    if !path.starts_with('/') {
        return false;
    }
    let bytes = path.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'~' {
            match bytes.get(index + 1) {
                Some(b'0') | Some(b'1') => index += 2,
                _ => return false,
            }
            continue;
        }
        index += 1;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use craftkit_sdk::serde_json::json;

    /// An empty model has nothing in it.
    #[test]
    fn an_empty_model_has_nothing_in_it() {
        let model = Model::new();
        assert!(model.get("/title").is_none());
        assert!(model.get_text("/title").is_none());
    }

    /// A value is written and read back.
    #[test]
    fn a_value_is_written_and_read_back() {
        let mut model = Model::new();
        model.set("/title", json!("hello")).expect("the value is written");
        assert_eq!(model.get_text("/title"), Some("hello".to_string()));
        assert_eq!(model.get("/title"), Some(&json!("hello")));
    }

    /// A **nested** path is walked.
    #[test]
    fn a_nested_path_is_walked() {
        let mut model = Model::new();
        model.set("/user/name", json!("ada")).expect("the value is written");
        assert_eq!(model.get_text("/user/name"), Some("ada".to_string()));
    }

    /// A path that is not there is `None`.
    #[test]
    fn a_path_that_is_not_there_is_none() {
        let model = Model::new();
        assert!(model.get("/missing").is_none());
        assert!(model.get("/missing/deeper").is_none());
    }

    /// A list is walked by its index.
    #[test]
    fn a_list_is_walked_by_its_index() {
        let mut model = Model::new();
        model.set("/items", json!([])).expect("the list is written");
        model.set("/items/0", json!("first")).expect("the value is written");
        model.set("/items/1", json!("second")).expect("the value is written");
        assert_eq!(model.get_text("/items/0"), Some("first".to_string()));
        assert_eq!(model.get_text("/items/1"), Some("second".to_string()));
        assert!(model.get("/items/2").is_none(), "and there is no third");
    }

    /// A step through a **scalar** is refused.
    #[test]
    fn a_step_through_a_scalar_is_refused() {
        let mut model = Model::new();
        model.set("/title", json!("hello")).expect("the value is written");
        let error = model.set("/title/deeper", json!("x")).expect_err("the step is refused");
        assert!(error.contains("not a tree"), "{error}");
    }

    /// A path that names a list entry that is not there is refused.
    #[test]
    fn a_path_that_names_nothing_is_refused() {
        let mut model = Model::new();
        model.set("/items", json!(["first"])).expect("the list is written");
        let error = model.set("/items/5", json!("x")).expect_err("the step is refused");
        assert!(error.contains("list has"), "{error}");
    }

    /// The **escapes** are the JSON Pointer's.
    #[test]
    fn the_escapes_are_the_json_pointers() {
        let mut model = Model::new();
        model.set("/a~1b", json!("slash")).expect("the value is written");
        assert_eq!(model.get_text("/a~1b"), Some("slash".to_string()));
        model.set("/c~0d", json!("tilde")).expect("the value is written");
        assert_eq!(model.get_text("/c~0d"), Some("tilde".to_string()));
    }

    /// A malformed path is refused.
    #[test]
    fn a_malformed_path_is_refused() {
        assert!(is_path(""), "the root is a path");
        assert!(is_path("/title"), "a path is a path");
        assert!(!is_path("title"), "a step with no separator is not");
        assert!(!is_path("/a~2b"), "and a bad escape is not");
    }

    /// A number and a boolean read as their own text.
    #[test]
    fn a_number_and_a_boolean_read_as_their_own_text() {
        let mut model = Model::new();
        model.set("/count", json!(3)).expect("the value is written");
        model.set("/on", json!(true)).expect("the value is written");
        assert_eq!(model.get_text("/count"), Some("3".to_string()));
        assert_eq!(model.get_text("/on"), Some("true".to_string()));
        assert_eq!(model.get_number("/count"), Some(3.0));
        assert_eq!(model.get_bool("/on"), Some(true));
    }
}
