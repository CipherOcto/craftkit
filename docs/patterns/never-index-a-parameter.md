# Never index a parameter

## The problem

Command parameters arrive as `serde_json::Value`. A command is the one place in
craftkit where the input is genuinely hostile: it comes from a person's
keystrokes, from a script, from an agent guessing at an id, from a file written
by a build of a different vintage. Nothing about the shape is guaranteed, and
`serde_json::Value` indexes with `[]`.

```rust
// Never this.
let distance = params["distance"].as_f64().unwrap();
```

That line has three separate ways to panic and none of them are the caller's
fault in any way they can see: the key is absent, the key is a string, or the
key is a number that is not finite. Widened by JSON, `{"distance": 1e400}` is a
number `as_f64` cannot represent.

## The rule

**Never index a parameter object.** Every read goes through a method on
`craftkit_core::Params`; every method returns `Option` or `Result`; every
numeric read validates. This is the mechanism, not a style preference: it is
what lets a test fire every registered command at every documented parameter
name with hostile values and still pass.

Two supporting rules:

- **A non-object is an empty object.** `Params::new` wraps `null`, a number and
  a list as an empty view, so `execute(id, null)` behaves like `execute(id, {})`
  instead of panicking.
- **An unknown key is an error.** `Params::deny_unknown` is called by
  `Session::execute` with the command's declared parameter names, so a typo is
  a message naming the parameter rather than a command that silently does
  nothing.

## In the code

The readers, `crates/foundation/src/params.rs`:

```rust
/// A required number. Non-finite and out-of-range values are an error, not
/// a silent clamp, because a command parameter is something a person or an
/// agent just typed.
pub fn num(&self, key: &str) -> Result<Option<f64>, ParamError> {
    match self.get(key) {
        None | Some(Json::Null) => Ok(None),
        Some(Json::Number(n)) => {
            let v = n.as_f64().unwrap_or(f64::NAN);
            finite("parameter", v).map(Some).map_err(|e| ParamError {
                key: key.to_string(),
                detail: e.to_string(),
            })
        }
        Some(_) => Err(self.wrong_type(key, "number")),
    }
}

/// The raw JSON for a key, for the cases where a caller genuinely needs it.
/// Still bounded by the line-length limit upstream.
pub fn raw(&self, key: &str) -> Option<&'a Json> {
    self.get(key).filter(|v| !v.is_null())
}

fn get(&self, key: &str) -> Option<&'a Json> {
    // `.get()` on an object is the whole point of this module.
    self.obj.as_object().and_then(|m| m.get(key))
}
```

The call sites, `crates/engine/src/commands.rs`. Note that `required_text`
composes a bounded read with a domain rule ("not blank"), and each failure
names the parameter:

```rust
fn required_text(p: &Params<'_>, key: &str) -> Result<String, EngineError> {
    p.text(key, limits::MAX_NAME)
        .map_err(|e| EngineError::Param { id: String::new(), source: e })?
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| EngineError::Param {
            id: String::new(),
            source: craftkit_core::ParamError::new(key, "this parameter is required"),
        })
}
```

## What to read instead of indexing

| Need | Read |
|---|---|
| a number | `num`, `num_in(key, min, max, default)`, `int` (refuses a fractional value rather than truncating) |
| text | `text(key, max_bytes)` — the cap is applied at the read |
| a boolean | `bool` |
| a colour | `color` — `#rrggbb` or `[r, g, b]`, each channel range-checked |
| a list | `num_list(key, max_items)` |
| an instant | `raw`, then `Tick::try_from_seconds_f64` — see [`exact-time-not-float-seconds.md`](exact-time-not-float-seconds.md) |

## When NOT to use it

- **Inside `Params` itself.** `get` is the one function in the crate allowed to
  touch the map; that is what makes the rest of the crate auditable.
- **For a struct that is already typed and already bounded.** Reading a
  `Document` out of a validated `File` is `from_text`'s job, with its own size
  cap and its own `validate` call — not a per-field re-read.
- **To skip validation because "the caller already checked it".** The caller is
  usually an agent that did not. The cost of a redundant check is a few
  nanoseconds; the cost of a panic is a lost design.
- **For the JSON *output*.** `Outcome::with` builds `serde_json::json!` freely.
  Outgoing data is ours; incoming data is not.

## See also

- [`one-trust-boundary.md`](one-trust-boundary.md) — why a bad parameter must
  become an error, not a panic, before it reaches a command body.
- [`../architecture.md`](../architecture.md#the-never-crash-contract) — where
  validation lives, and why it is not re-checked downstream.
