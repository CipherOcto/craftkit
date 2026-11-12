//! `craftkit-payload` — reading a node's **payload** without trusting it.
//!
//! # What this is
//!
//! A node's payload is a **JSON value that came from outside**: from a
//! file, from a script, from another program. The kit's rule is that
//! input-derived numbers are hostile — "check finiteness, bound sizes,
//! and cap loop counts" — and the core enforces that rule for a
//! command's **parameters**.
//!
//! It does not enforce it for a **payload**. The core has no opinion
//! about what a payload holds: that is the plugin's business, and the
//! plugin reads the payload itself. Which means that every plugin that
//! reads one has to write its own bounds — and a plugin that forgets
//! is a plugin that will read a table of a million entries into a map
//! because a file said so.
//!
//! This crate is the **mechanism** that every such plugin needs, and it
//! is a **library** rather than a change to the core, for the same
//! reason the typed value is (§39, §52): the kit's core has no opinion
//! about a payload's *shape*, and the *shape* is the plugin's. What is
//! not the plugin's is the **size**: how many entries, how long a key,
//! how large a number. Those are the same for everybody.
//!
//! # What the reader does **not** check
//!
//! **Finiteness.** A JSON number is always finite: serde_json refuses a
//! number that is out of range and writes a float that is not finite as
//! `null`. The kit's rule asks for a finiteness check on every number
//! that came from outside — and for a **payload** the format has
//! already made it. What the reader checks is the **magnitude**, which
//! the format does not know about.
//!
//! # The two kinds of refusal
//!
//! - a **bound** is not negotiable. A table with too many entries is
//!   refused, and there is no "lossy" way round it: the bound is the
//!   point;
//! - a **shape** is the plugin's. A table whose value is not a number
//!   may be refused outright ([`Payload::table`]) or **skipped**
//!   ([`Payload::table_lossy`]), and the choice is the plugin's — it
//!   depends on whether the reader must keep what it does not
//!   understand.
//!
//! # The limits are the plugin's
//!
//! [`Limits`] is a value, and the plugin chooses it. The defaults are
//! the ones a document can carry comfortably; a plugin with a bigger
//! table to read says so.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

use serde_json::Value as Json;

/// The limits a reader holds a payload to.
///
/// Every limit is a **count** or a **magnitude**, and every one of them
/// exists because the value came from outside.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Limits {
    /// The most entries in one table or list.
    pub entries: usize,
    /// The most characters in a key.
    pub key_length: usize,
    /// The most characters in a string.
    pub text_length: usize,
    /// The largest magnitude of a number.
    pub value: f64,
}

impl Default for Limits {
    fn default() -> Self {
        Limits { entries: 1024, key_length: 64, text_length: 4096, value: 1_000_000.0 }
    }
}

/// Why a value was refused.
#[derive(Clone, Debug, PartialEq)]
pub enum Refusal {
    /// There is no value under that name.
    NotFound {
        /// The name that was asked for.
        name: String,
    },
    /// The value is not of the shape that was asked for.
    NotA {
        /// The name that was asked for.
        name: String,
        /// What was wanted.
        wanted: &'static str,
    },
    /// There are more entries than the limit allows.
    TooManyEntries {
        /// The name that was asked for.
        name: String,
        /// The limit.
        limit: usize,
    },
    /// A key is longer than the limit allows.
    KeyTooLong {
        /// The name that was asked for.
        name: String,
        /// The limit.
        limit: usize,
    },
    /// A string is longer than the limit allows.
    TextTooLong {
        /// The name that was asked for.
        name: String,
        /// The limit.
        limit: usize,
    },
    /// A number is larger than the limit allows.
    TooLarge {
        /// The name that was asked for.
        name: String,
        /// The limit.
        limit: f64,
    },
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refusal::NotFound { name } => write!(f, "there is no `{name}`"),
            Refusal::NotA { name, wanted } => write!(f, "`{name}` is not {wanted}"),
            Refusal::TooManyEntries { name, limit } => {
                write!(f, "`{name}` holds more than {limit} entries")
            }
            Refusal::KeyTooLong { name, limit } => {
                write!(f, "a key of `{name}` is longer than {limit} characters")
            }
            Refusal::TextTooLong { name, limit } => {
                write!(f, "a string of `{name}` is longer than {limit} characters")
            }
            Refusal::TooLarge { name, limit } => {
                write!(f, "a number of `{name}` is larger than {limit}")
            }
        }
    }
}

impl std::error::Error for Refusal {}

/// A payload, read within limits.
#[derive(Clone, Copy, Debug)]
pub struct Payload<'a> {
    value: &'a Json,
    limits: Limits,
}

impl<'a> Payload<'a> {
    /// Read a payload within the given limits.
    pub fn new(value: &'a Json, limits: Limits) -> Payload<'a> {
        Payload { value, limits }
    }

    /// Read a payload within the **default** limits.
    pub fn of(value: &'a Json) -> Payload<'a> {
        Payload::new(value, Limits::default())
    }

    /// The limits this reader holds to.
    pub fn limits(&self) -> Limits {
        self.limits
    }

    /// The value under a name, if it is there.
    fn get(&self, name: &str) -> Result<&'a Json, Refusal> {
        self.value.get(name).ok_or_else(|| Refusal::NotFound { name: name.to_string() })
    }

    /// A **number** under a name.
    ///
    /// It has to be a number, it has to be finite, and it has to be
    /// within the limit.
    pub fn number(&self, name: &str) -> Result<f64, Refusal> {
        let value = self.get(name)?;
        let Some(number) = value.as_f64() else {
            return Err(Refusal::NotA { name: name.to_string(), wanted: "a number" });
        };
        if number.abs() > self.limits.value {
            return Err(Refusal::TooLarge { name: name.to_string(), limit: self.limits.value });
        }
        Ok(number)
    }

    /// A **string** under a name.
    ///
    /// It has to be a string, and it has to be within the limit.
    pub fn text(&self, name: &str) -> Result<&'a str, Refusal> {
        let value = self.get(name)?;
        let Some(text) = value.as_str() else {
            return Err(Refusal::NotA { name: name.to_string(), wanted: "a string" });
        };
        if text.chars().count() > self.limits.text_length {
            return Err(Refusal::TextTooLong { name: name.to_string(), limit: self.limits.text_length });
        }
        Ok(text)
    }

    /// A **boolean** under a name.
    pub fn boolean(&self, name: &str) -> Result<bool, Refusal> {
        let value = self.get(name)?;
        value.as_bool().ok_or_else(|| Refusal::NotA { name: name.to_string(), wanted: "a boolean" })
    }

    /// A **list** under a name, within the limit.
    pub fn list(&self, name: &str) -> Result<Vec<&'a Json>, Refusal> {
        let value = self.get(name)?;
        let Some(list) = value.as_array() else {
            return Err(Refusal::NotA { name: name.to_string(), wanted: "a list" });
        };
        if list.len() > self.limits.entries {
            return Err(Refusal::TooManyEntries { name: name.to_string(), limit: self.limits.entries });
        }
        Ok(list.iter().collect())
    }

    /// A list of **words** under a name: a list of strings, within the
    /// limits.
    ///
    /// Every word has to be a string, and every word has to be within
    /// the text limit.
    pub fn words(&self, name: &str) -> Result<Vec<&'a str>, Refusal> {
        let list = self.list(name)?;
        let mut out = Vec::with_capacity(list.len());
        for value in list {
            let Some(word) = value.as_str() else {
                return Err(Refusal::NotA { name: name.to_string(), wanted: "a list of strings" });
            };
            if word.chars().count() > self.limits.text_length {
                return Err(Refusal::TextTooLong { name: name.to_string(), limit: self.limits.text_length });
            }
            out.push(word);
        }
        Ok(out)
    }

    /// A **table** under a name: a map of keys to numbers, within the
    /// limits.
    ///
    /// Every value has to be a number, and every key has to be within
    /// the key limit. A value that is not a number refuses the table.
    pub fn table(&self, name: &str) -> Result<Vec<(&'a str, f64)>, Refusal> {
        let table = self.table_shape(name)?;
        let mut out = Vec::with_capacity(table.len());
        for (key, value) in table {
            let Some(number) = value.as_f64() else {
                return Err(Refusal::NotA { name: name.to_string(), wanted: "a table of numbers" });
            };
            if number.abs() > self.limits.value {
                return Err(Refusal::TooLarge { name: name.to_string(), limit: self.limits.value });
            }
            out.push((key.as_str(), number));
        }
        Ok(out)
    }

    /// A **table** under a name, within the limits, keeping what it does
    /// not understand.
    ///
    /// The bounds still bite: a table with too many entries is refused,
    /// and a key that is too long is refused. But an entry whose value
    /// is not a number is **skipped**, and the reader carries on.
    pub fn table_lossy(&self, name: &str) -> Result<Vec<(&'a str, f64)>, Refusal> {
        let table = self.table_shape(name)?;
        let mut out = Vec::with_capacity(table.len());
        for (key, value) in table {
            let Some(number) = value.as_f64() else { continue };
            if number.abs() > self.limits.value {
                continue;
            }
            out.push((key.as_str(), number));
        }
        Ok(out)
    }

    /// The table itself: a map, within the bounds on its size and its
    /// keys.
    fn table_shape(&self, name: &str) -> Result<Vec<(&'a String, &'a Json)>, Refusal> {
        let value = self.get(name)?;
        let Some(table) = value.as_object() else {
            return Err(Refusal::NotA { name: name.to_string(), wanted: "a table" });
        };
        if table.len() > self.limits.entries {
            return Err(Refusal::TooManyEntries { name: name.to_string(), limit: self.limits.entries });
        }
        let mut out = Vec::with_capacity(table.len());
        for (key, value) in table {
            if key.chars().count() > self.limits.key_length {
                return Err(Refusal::KeyTooLong { name: name.to_string(), limit: self.limits.key_length });
            }
            out.push((key, value));
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A payload with a number, a string and a table.
    fn payload() -> Json {
        json!({
            "count": 3.0,
            "name": "a name",
            "flag": true,
            "table": { "a": 1.0, "b": 2.0 },
            "words": ["one", "two"],
        })
    }

    #[test]
    fn a_number_is_read() {
        let value = payload();
        let payload = Payload::of(&value);
        assert_eq!(payload.number("count").expect("a number"), 3.0);
    }

    #[test]
    fn a_name_that_is_not_there_is_refused() {
        let value = payload();
        let payload = Payload::of(&value);
        assert!(matches!(payload.number("missing"), Err(Refusal::NotFound { .. })));
    }

    #[test]
    fn a_number_that_is_not_a_number_is_refused() {
        let value = json!({ "count": "three" });
        let payload = Payload::of(&value);
        assert!(matches!(payload.number("count"), Err(Refusal::NotA { .. })));
    }

    #[test]
    fn a_json_number_is_always_finite() {
        // The format has already done the finiteness check: serde_json
        // refuses a number that is out of range and writes a float that
        // is not finite as null. So the reader never sees one, and the
        // limit that matters is the magnitude.
        assert!(serde_json::from_str::<Json>("1e400").is_err(), "the parser refuses it");
        assert_eq!(serde_json::Value::from(f64::INFINITY), Json::Null, "and the writer nulls it");
        let value = json!({ "count": 1e308 });
        let payload = Payload::of(&value);
        assert!(matches!(payload.number("count"), Err(Refusal::TooLarge { .. })));
    }

    #[test]
    fn a_number_that_is_too_large_is_refused() {
        let value = json!({ "count": 1e9 });
        let payload = Payload::of(&value);
        assert!(matches!(payload.number("count"), Err(Refusal::TooLarge { .. })));
    }

    #[test]
    fn a_string_is_read() {
        let value = payload();
        let payload = Payload::of(&value);
        assert_eq!(payload.text("name").expect("a string"), "a name");
    }

    #[test]
    fn a_string_that_is_too_long_is_refused() {
        let value = json!({ "name": "x".repeat(5000) });
        let payload = Payload::of(&value);
        assert!(matches!(payload.text("name"), Err(Refusal::TextTooLong { .. })));
    }

    #[test]
    fn a_boolean_is_read() {
        let value = payload();
        let payload = Payload::of(&value);
        assert!(payload.boolean("flag").expect("a boolean"));
    }

    #[test]
    fn a_table_is_read() {
        let value = payload();
        let payload = Payload::of(&value);
        let table = payload.table("table").expect("a table");
        assert_eq!(table.len(), 2);
        assert!(table.contains(&("a", 1.0)));
    }

    #[test]
    fn a_table_with_too_many_entries_is_refused() {
        let mut table = serde_json::Map::new();
        for index in 0..2000 {
            table.insert(format!("k{index}"), json!(1.0));
        }
        let value = json!({ "table": Json::Object(table) });
        let payload = Payload::of(&value);
        assert!(matches!(payload.table("table"), Err(Refusal::TooManyEntries { .. })));
        // And the lossy reading refuses it too: a bound is a bound.
        assert!(matches!(payload.table_lossy("table"), Err(Refusal::TooManyEntries { .. })));
    }

    #[test]
    fn a_key_that_is_too_long_is_refused() {
        let value = json!({ "table": { "x".repeat(100): 1.0 } });
        let payload = Payload::of(&value);
        assert!(matches!(payload.table("table"), Err(Refusal::KeyTooLong { .. })));
    }

    #[test]
    fn the_lossy_table_keeps_what_it_does_not_understand() {
        let value = json!({ "table": { "a": 1.0, "b": "two", "c": 3.0 } });
        let payload = Payload::of(&value);
        let table = payload.table_lossy("table").expect("a table");
        assert_eq!(table.len(), 2, "the entry that is not a number is skipped");
        assert!(table.contains(&("a", 1.0)));
        assert!(table.contains(&("c", 3.0)));
    }

    #[test]
    fn the_strict_table_refuses_what_it_does_not_understand() {
        let value = json!({ "table": { "a": 1.0, "b": "two" } });
        let payload = Payload::of(&value);
        assert!(matches!(payload.table("table"), Err(Refusal::NotA { .. })));
    }

    #[test]
    fn words_are_read() {
        let value = payload();
        let payload = Payload::of(&value);
        assert_eq!(payload.words("words").expect("words"), vec!["one", "two"]);
    }

    #[test]
    fn a_list_that_is_not_a_list_is_refused() {
        let value = json!({ "words": "one two" });
        let payload = Payload::of(&value);
        assert!(matches!(payload.words("words"), Err(Refusal::NotA { .. })));
    }

    #[test]
    fn a_list_with_too_many_entries_is_refused() {
        let words: Vec<&str> = (0..2000).map(|_| "word").collect();
        let value = json!({ "words": words });
        let payload = Payload::of(&value);
        assert!(matches!(payload.words("words"), Err(Refusal::TooManyEntries { .. })));
    }

    #[test]
    fn the_limits_are_the_callers() {
        let value = json!({ "table": { "a": 1.0, "b": 2.0 } });
        let limits = Limits { entries: 1, ..Limits::default() };
        let payload = Payload::new(&value, limits);
        assert!(matches!(payload.table("table"), Err(Refusal::TooManyEntries { .. })));
    }
}
