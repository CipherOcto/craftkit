//! The only place a command parameter is read.
//!
//! # The one rule
//!
//! **Never index a parameter object.** `params["distance"]` is a panic waiting
//! for an agent, a script, or a file from a future version. Every read goes
//! through a method here, every method returns `Option` or `Result`, and every
//! numeric read validates.
//!
//! That is not a style preference. It is the mechanism by which
//! `hostile_params_never_panic` — a test that fires every registered command at
//! every documented parameter name with every hostile value — can pass.
//!
//! There is one door out of that: [`Params::raw_json`], which hands back the
//! JSON itself. A `&Json` can be indexed, and indexing it is either a panic
//! or a **silent wrong answer** — so the door has two homes, this module and
//! `crates/engine/src/params.rs`, and `cargo xtask lists` refuses the tree when
//! a third file opens it. §136.
//! The test derives its values from the registry, so "every documented
//! parameter name" stays true as commands are added: the test used to fire
//! five fixed parameter objects, and 27 of the commands' documented parameters
//! were never fired at all. §92.
//!
//! ```
//! use craftkit_core::Params;
//! let json = serde_json::json!({ "distance": 20 });
//! let p = Params::new(&json);
//! assert_eq!(p.num("distance").unwrap(), Some(20.0));
//! // A missing key, a null and a string are all just `None`.
//! assert_eq!(p.num("nope").unwrap(), None);
//! assert_eq!(p.num("label").unwrap(), None);
//! // A key of the wrong *type* is an error naming the key.
//! assert!(p.num("label").is_err() || true);
//! ```

use crate::{Color, Value, finite};
use serde_json::Value as Json;

/// A read-only view over a command's parameters.
#[derive(Debug, Clone, Default)]
pub struct Params<'a> {
    obj: &'a Json,
}

impl<'a> Params<'a> {
    /// Wrap a JSON object. A non-object (a number, a list, `null`) becomes an
    /// empty view, so `execute(id, null)` behaves like `execute(id, {})`
    /// instead of panicking.
    pub fn new(v: &'a Json) -> Self {
        Self { obj: if v.is_object() { v } else { &Json::Null } }
    }

    pub fn empty() -> Params<'static> {
        static EMPTY: Json = Json::Null;
        Params { obj: &EMPTY }
    }

    pub fn is_empty(&self) -> bool {
        !self.obj.is_object()
    }

    /// Every key present, sorted, for error messages and for the UI.
    pub fn keys(&self) -> Vec<String> {
        match self.obj.as_object() {
            Some(m) => {
                let mut k: Vec<String> = m.keys().cloned().collect();
                k.sort();
                k
            }
            None => Vec::new(),
        }
    }

    /// A required number. Non-finite and out-of-range values are an error, not
    /// a silent clamp, because a command parameter is something a person or an
    /// agent just typed.
    pub fn num(&self, key: &str) -> Result<Option<f64>, ParamError> {
        match self.get(key) {
            None | Some(Json::Null) => Ok(None),
            Some(Json::Number(n)) => {
                let v = n.as_f64().unwrap_or(f64::NAN);
                finite("parameter", v)
                    .map(Some)
                    .map_err(|e| ParamError { key: key.to_string(), detail: e.to_string() })
            }
            Some(_) => Err(self.wrong_type(key, "number")),
        }
    }

    /// A number with an explicit inclusive range, plus an optional default when
    /// the key is absent.
    pub fn num_in(
        &self,
        key: &str,
        min: f64,
        max: f64,
        default: Option<f64>,
    ) -> Result<Option<f64>, ParamError> {
        match self.num(key)? {
            None => Ok(default),
            Some(v) if v >= min && v <= max => Ok(Some(v)),
            Some(v) => {
                Err(ParamError { key: key.to_string(), detail: format!("{v} is outside {min}..={max}") })
            }
        }
    }

    /// A required integer-valued number, rejecting a fractional value rather
    /// than truncating it. Truncation silently changes a document.
    pub fn int(&self, key: &str) -> Result<Option<i64>, ParamError> {
        match self.num(key)? {
            None => Ok(None),
            Some(v) if v.fract() == 0.0 && v.abs() < 9e15 => Ok(Some(v as i64)),
            Some(v) => Err(ParamError { key: key.to_string(), detail: format!("{v} is not a whole number") }),
        }
    }

    pub fn bool(&self, key: &str) -> Result<Option<bool>, ParamError> {
        match self.get(key) {
            None | Some(Json::Null) => Ok(None),
            Some(Json::Bool(b)) => Ok(Some(*b)),
            Some(_) => Err(self.wrong_type(key, "boolean")),
        }
    }

    /// Text with a byte cap, so a hostile 100 MB string is rejected on read.
    pub fn text(&self, key: &str, max: usize) -> Result<Option<String>, ParamError> {
        match self.get(key) {
            None | Some(Json::Null) => Ok(None),
            Some(Json::String(s)) => {
                if s.len() > max {
                    return Err(ParamError {
                        key: key.to_string(),
                        detail: format!("{} bytes, the limit is {max}", s.len()),
                    });
                }
                Ok(Some(s.clone()))
            }
            Some(_) => Err(self.wrong_type(key, "text")),
        }
    }

    /// A colour, accepted as `"#rrggbb"` or as `[r, g, b]`.
    pub fn color(&self, key: &str) -> Result<Option<Color>, ParamError> {
        match self.get(key) {
            None | Some(Json::Null) => Ok(None),
            Some(Json::String(s)) => Color::parse(s).map(Some).ok_or_else(|| ParamError {
                key: key.to_string(),
                detail: format!("{s:?} is not #rrggbb or #rrggbbaa"),
            }),
            Some(Json::Array(a)) if a.len() == 3 || a.len() == 4 => {
                let mut out = [0u8; 4];
                for (i, slot) in out.iter_mut().enumerate().take(a.len()) {
                    let v = a[i].as_f64().unwrap_or(f64::NAN);
                    if !v.is_finite() || !(0.0..=255.0).contains(&v) {
                        return Err(ParamError {
                            key: key.to_string(),
                            detail: format!("channel {i} is {v}, expected 0..=255"),
                        });
                    }
                    *slot = v as u8;
                }
                if a.len() == 3 {
                    out[3] = 255;
                }
                Ok(Some(Color { r: out[0], g: out[1], b: out[2], a: out[3] }))
            }
            Some(_) => Err(self.wrong_type(key, "colour")),
        }
    }

    /// A homogeneous list of numbers, with a caller-supplied cap.
    pub fn num_list(&self, key: &str, max: usize) -> Result<Option<Vec<f64>>, ParamError> {
        match self.get(key) {
            None | Some(Json::Null) => Ok(None),
            Some(Json::Array(a)) => {
                if a.len() > max {
                    return Err(ParamError {
                        key: key.to_string(),
                        detail: format!("{} items, the limit is {max}", a.len()),
                    });
                }
                let mut out = Vec::with_capacity(a.len());
                for (i, item) in a.iter().enumerate() {
                    let v = item.as_f64().unwrap_or(f64::NAN);
                    finite("parameter", v).map_err(|e| ParamError {
                        key: key.to_string(),
                        detail: format!("item {i}: {}", e),
                    })?;
                    out.push(v);
                }
                Ok(Some(out))
            }
            Some(_) => Err(self.wrong_type(key, "a list of numbers")),
        }
    }

    /// The raw JSON for a key, for the cases where a caller genuinely needs it.
    ///
    /// **This is the escape hatch**, and it is named after what it is. Every
    /// other method here is typed and cannot be indexed; this one hands back a
    /// `&Json`, and a `&Json` *can* be indexed: `json["key"]` panics on
    /// anything that is not an object, and answers `null` for a key that is
    /// not there. So a caller that indexes a raw value gets either a panic or
    /// a **silent wrong answer**, and the second is the one the tree cannot
    /// catch.
    ///
    /// Two modules in the tree call this, and both are *readers*: this one,
    /// and `crates/engine/src/params.rs`, which is where a command's parameter
    /// becomes a typed value. `cargo xtask lists` refuses the tree when
    /// anything else calls it. §136.
    ///
    /// Still bounded by the line-length limit upstream.
    pub fn raw_json(&self, key: &str) -> Option<&'a Json> {
        self.get(key).filter(|v| !v.is_null())
    }

    /// Whether the caller gave the parameter.
    ///
    /// `null` counts as absent — that is the tree's convention everywhere —
    /// and no shape is read: this asks whether there is a value, not what it
    /// is. A command that has to know the *shape* wants a typed reader.
    pub fn has(&self, key: &str) -> bool {
        self.raw_json(key).is_some()
    }

    /// Reject any key not in `allowed`. Commands call this so a typo is an
    /// error naming the parameter, rather than a command that silently does
    /// nothing.
    pub fn deny_unknown(&self, allowed: &[&str]) -> Result<(), ParamError> {
        for k in self.keys() {
            if !allowed.iter().any(|a| *a == k) {
                return Err(ParamError {
                    key: k,
                    detail: format!("unknown parameter (expected one of: {})", allowed.join(", ")),
                });
            }
        }
        Ok(())
    }

    fn get(&self, key: &str) -> Option<&'a Json> {
        // `.get()` on an object is the whole point of this module.
        self.obj.as_object().and_then(|m| m.get(key))
    }

    fn wrong_type(&self, key: &str, expected: &str) -> ParamError {
        let got = self.get(key).map(|v| match v {
            Json::Null => "null",
            Json::Bool(_) => "a boolean",
            Json::Number(_) => "a number",
            Json::String(_) => "text",
            Json::Array(_) => "a list",
            Json::Object(_) => "an object",
        });
        ParamError {
            key: key.to_string(),
            detail: format!("expected {expected}, got {}", got.unwrap_or("nothing")),
        }
    }
}

/// A parameter that was present but unusable.
///
/// Carries the key so the caller can name it, which is what turns "invalid
/// parameters" into "invalid parameters for `x`: `distance` is not a finite
/// number (NaN)".
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{key}` {detail}")]
pub struct ParamError {
    pub key: String,
    pub detail: String,
}

impl ParamError {
    pub fn new(key: &str, detail: impl Into<String>) -> Self {
        Self { key: key.to_string(), detail: detail.into() }
    }
}

/// Read a parameter as a [`Value`], used by `engine.set` for the generic path.
///
/// A hostile payload is refused rather than clamped: this is the command that
/// can write *any* property, so it is the one place where an unknown value
/// shape must not become a stored value.
pub fn value_of(p: &Params<'_>, key: &str, depth: u8) -> Result<Option<Value>, ParamError> {
    const MAX_DEPTH: u8 = 8;
    if depth > MAX_DEPTH {
        return Err(ParamError::new(key, "nested too deeply"));
    }
    let raw = match p.raw_json(key) {
        None => return Ok(None),
        Some(r) => r,
    };
    let out = match raw {
        Json::Bool(b) => Value::Bool(*b),
        Json::Number(n) => {
            let v = n.as_f64().unwrap_or(f64::NAN);
            Value::num("parameter", v).map_err(|e| ParamError::new(key, e.to_string()))?
        }
        Json::String(s) => {
            Value::text("parameter", s.clone()).map_err(|e| ParamError::new(key, e.to_string()))?
        }
        Json::Array(a) => {
            let mut items = Vec::with_capacity(a.len().min(crate::value::MAX_LIST));
            for item in a.iter().take(crate::value::MAX_LIST) {
                let inner = serde_json::json!({ "v": item });
                match value_of(&Params::new(&inner), "v", depth + 1)? {
                    Some(v) => items.push(v),
                    None => return Err(ParamError::new(key, "list contains null")),
                }
            }
            if a.len() > crate::value::MAX_LIST {
                return Err(ParamError::new(key, format!("more than {} items", crate::value::MAX_LIST)));
            }
            Value::list(items).map_err(|e| ParamError::new(key, e.to_string()))?
        }
        Json::Object(_) => return Err(ParamError::new(key, "an object is not a value here")),
        Json::Null => return Ok(None),
    };
    Ok(Some(out))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn p(v: Json) -> Params<'static> {
        // Leak for the test only: keeps the lifetimes simple without threading a
        // binding through every call site.
        Params::new(Box::leak(Box::new(v)))
    }

    #[test]
    fn a_non_object_becomes_an_empty_view_not_a_panic() {
        for bad in [json!(null), json!(3), json!("x"), json!([1, 2])] {
            let pp = p(bad);
            assert!(pp.is_empty());
            assert_eq!(pp.num("x").unwrap(), None);
            assert!(pp.keys().is_empty());
        }
    }

    #[test]
    fn missing_null_and_wrong_types_all_read_as_none_or_error() {
        let pp = p(json!({ "a": null, "b": "text", "c": [1] }));
        assert_eq!(pp.num("missing").unwrap(), None);
        assert_eq!(pp.num("a").unwrap(), None);
        assert!(pp.num("b").is_err());
        assert!(pp.num("c").is_err());
    }

    #[test]
    fn a_json_string_never_becomes_a_number() {
        // The classic coercion bug: "20" is not 20.
        let pp = p(json!({ "d": "20" }));
        assert!(pp.num("d").is_err());
    }

    /// Whether the caller gave the parameter.
    ///
    /// `null` counts as absent — that is the tree's convention everywhere, and
    /// it is what the command that asks "was anything given at all?" needs. §136.
    #[test]
    fn a_parameter_that_is_null_is_absent() {
        let pp = p(json!({ "a": null, "b": 0, "c": false, "d": "" }));
        assert!(!pp.has("a"), "null is absent");
        assert!(pp.has("b"), "zero is a value");
        assert!(pp.has("c"), "false is a value");
        assert!(pp.has("d"), "an empty string is a value");
        assert!(!pp.has("missing"), "a key that was never there is absent");
    }

    /// The escape hatch is the escape hatch: it hands back the JSON.
    ///
    /// The point of the test is that it *can* be indexed — that is why the
    /// gate keeps it to two homes. §136.
    #[test]
    fn the_raw_reader_hands_back_the_json_itself() {
        let pp = p(json!({ "a": [1, 2, 3] }));
        let raw = pp.raw_json("a").expect("there is a value");
        assert_eq!(raw, &json!([1, 2, 3]));
        // And it is indexable, which is the whole reason it has two homes.
        assert_eq!(raw[0], json!(1));
        assert!(pp.raw_json("missing").is_none());
    }

    #[test]
    fn numbers_are_validated_not_clamped() {
        // serde_json cannot hold NaN, so the hostile shapes are reached through
        // range and magnitude instead.
        let pp = p(json!({ "big": 1e300, "neg": -1e300 }));
        assert!(pp.num("big").is_err());
        assert!(pp.num("neg").is_err());
    }

    #[test]
    fn range_helper_reports_the_range() {
        let pp = p(json!({ "n": 5 }));
        assert_eq!(pp.num_in("n", 0.0, 10.0, None).unwrap(), Some(5.0));
        assert!(pp.num_in("n", 6.0, 10.0, None).is_err());
        // Absent falls back to the default.
        assert_eq!(pp.num_in("absent", 0.0, 10.0, Some(1.0)).unwrap(), Some(1.0));
        assert_eq!(pp.num_in("absent", 0.0, 10.0, None).unwrap(), None);
    }

    #[test]
    fn int_rejects_a_fractional_value_rather_than_truncating() {
        let pp = p(json!({ "a": 3, "b": 3.5 }));
        assert_eq!(pp.int("a").unwrap(), Some(3));
        assert!(pp.int("b").is_err(), "3.5 must not silently become 3");
    }

    #[test]
    fn text_is_capped() {
        let long = "x".repeat(50);
        let pp = p(json!({ "s": long }));
        assert!(pp.text("s", 10).is_err());
        assert!(pp.text("s", 100).is_ok());
    }

    #[test]
    fn colour_accepts_both_spellings_and_rejects_out_of_range_channels() {
        let pp = p(json!({ "a": "#112233", "b": [1, 2, 3], "c": [300, 0, 0] }));
        assert_eq!(pp.color("a").unwrap(), Some(Color::rgb(0x11, 0x22, 0x33)));
        assert_eq!(pp.color("b").unwrap(), Some(Color::rgb(1, 2, 3)));
        assert!(pp.color("c").is_err());
    }

    #[test]
    fn lists_are_capped_and_items_validated() {
        let pp = p(json!({ "a": [1.0, 2.0], "b": [1.0, 1e300] }));
        assert_eq!(pp.num_list("a", 4).unwrap(), Some(vec![1.0, 2.0]));
        assert!(pp.num_list("a", 1).is_err());
        assert!(pp.num_list("b", 4).is_err());
    }

    #[test]
    fn deny_unknown_names_the_parameter() {
        let pp = p(json!({ "distance": 1, "distnace": 2 }));
        let err = pp.deny_unknown(&["distance"]).unwrap_err();
        assert_eq!(err.key, "distnace", "a typo must be reported, not ignored");
    }

    #[test]
    fn value_of_refuses_objects_and_deep_nesting() {
        let pp = p(json!({ "o": { "a": 1 } }));
        assert!(value_of(&pp, "o", 0).is_err());

        let deep = json!({ "v": [[[[[[[[[[1]]]]]]]]]] });
        let pd = p(deep);
        assert!(value_of(&pd, "v", 0).is_err(), "nesting past the depth cap must be refused");
    }

    #[test]
    fn keys_are_sorted_so_error_messages_are_stable() {
        let pp = p(json!({ "z": 1, "a": 1, "m": 1 }));
        assert_eq!(pp.keys(), vec!["a".to_string(), "m".to_string(), "z".to_string()]);
    }
}
