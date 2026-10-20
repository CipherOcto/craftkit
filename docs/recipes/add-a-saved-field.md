# Add a saved field

**When you need this** — you are adding a field to a struct that derives
`Serialize`/`Deserialize` and is written to a `.craftkit` file, and files
already on disk must keep loading.

**Before you start**

```
cargo test -p craftkit-model -p craftkit-io
```

is green. Confirm the field is genuinely *saved* — if it is `#[serde(skip)]`,
or the struct never reaches `File::document`, this recipe does not apply.

---

## Steps

### 1. Decide the default first, not the field

The default is a product decision: what should a design written before this
field existed show? Two shapes only.

| You want older files to | Use |
| --- | --- |
| See a sensible value | `#[serde(default)]` and give the type a `Default` |
| See an explicit, non-default value | `#[serde(default = "fn_name")]` |

The canonical example is already in the tree —
`DocumentMeta::comment` (`crates/model/src/document.rs`, `DocumentMeta`):

```rust exact
// crates/model/src/document.rs
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentMeta {
    pub name: String,
    pub fps: u32,
    pub start: Tick,
    pub end: Tick,
    #[serde(default)]
    pub comment: String,
}
```

`Meta` (in `crates/io/src/format.rs`) shows the other shape, for a field whose sensible
default is not `Default::default()`:

```rust
/// The default for [`Meta::written_at`], so a file written before the field
/// existed still loads.
#[serde(default = "zero")]
pub written_at: i128,
```

If the default is non-trivial, write the function next to the struct with a
comment saying what it is for, exactly as `zero()` does.

### 2. Add the field, then the `Default` impl

`DocumentMeta` has a hand-written `Default`
(`Default for DocumentMeta` in `crates/model/src/document.rs`) listing every
field. A new field is a compile error there until you add it — which is the
point: the compiler tells you the one place you would otherwise forget.

```rust
impl Default for DocumentMeta {
    fn default() -> Self {
        Self {
            name: "Untitled".to_string(),
            fps: 24,
            start: Tick::ZERO,
            end: Tick::from_seconds_f64(5.0),
            comment: String::new(),
        }
    }
}
```

The same applies to any constructor that builds the struct field by field.
Grep for it:

```
rg -n "Self \{" crates/model/src/document.rs
```

### 3. Bound anything unbounded

A saved field is hostile input on the way back in. If it is a string, a number
or a collection, cap it with the limit that already exists rather than
inventing a new one:

```
rg -n "pub const MAX" crates/model/src/lib.rs
```

That prints `craftkit_model::limits`: `MAX_NODES` (10 000), `MAX_DEPTH` (32),
`MAX_PROPERTIES` (50 000), `MAX_KEYS` (10 000), `MAX_PATH_SEGMENTS` (16),
`MAX_NAME` (255) and `MAX_DOCUMENT_BYTES` (`32 * 1024 * 1024` bytes). For text
and numbers, the
caps are `craftkit_core::MAX_TEXT` (`1 << 16`) and
`craftkit_core::MAX_MAGNITUDE` (`1e9`). Reference these caps in the doc
comment so the next reader knows the bound is deliberate.

**A cap in the setter is not a cap on the file.** A saved field is read back by
deserializing it, and no constructor runs on the way in — so the field's own
constructor keeps the invariant only for the values that arrive through it. The
model's validation has to check the field too, or a hand-edited file carries
what the setter would have refused: `craftkit_core::Value::check` is what a
value in a document is held to, and the model calls it for every property and
every key (§73).

Every limit in that table is enforced somewhere in the tree, and
`cargo xtask lists` says so: a limit nobody enforces is a wish, and a wish in a
table of limits is worse than no table (§74).

`Property` in `crates/model/src/property.rs:24` is the model to copy — `id`,
`keys` and `locked` all carry `#[serde(default)]` with a comment saying why.

### 4. Make it reachable from a command, or do not add it

A field nobody can set is a field that is always its default. If it is
user-visible, add the command in the same change
([add-a-command.md](add-a-command.md)); if it is derived, compute it and say so
in the doc comment.

### 5. Verify the older file still loads

```
cargo test -p craftkit-model -p craftkit-io -p craftkit-engine
```

---

## The test you must add

Two tests, in two places. Both are required; each catches something the other
cannot see.

### A. The round-trip test, next to the struct

`crates/model/src/document.rs` already has the shape. Extend
`documents_round_trip_through_json_and_keep_the_id_counter`
(in `crates/model/src/document.rs`) or add a sibling:

```rust
#[test]
fn a_document_from_before_the_comment_field_existed_still_loads() {
    // Exactly the JSON a file written before `comment` was added contains.
    // Ticks are 254_016_000_000 per second, so 5 s is 1_270_080_000_000.
    let old = r#"{
        "meta": { "name": "Old", "fps": 24, "start": 0, "end": 1270080000000 },
        "nodes": []
    }"#;
    let doc: Document = serde_json::from_str(old).expect("an older file loads");
    assert_eq!(doc.meta.comment, "", "the default is what a person sees");
    assert!(doc.validate().is_ok());
}
```

The `expect("an older file loads")` is the whole point of the test: with no
`#[serde(default)]` it fails here with a serde *"missing field `comment`"*, and
every `.craftkit` file anyone has ever saved stops opening.

### B. The file-level round trip, in `crates/io`

`crates/io/src/store.rs` has `a_document_round_trips_through_a_file`. Add a
sibling that goes through the real header, because `craftkit-io` is the layer
that sees the file rather than the struct:

```rust
#[test]
fn a_new_field_survives_a_save_and_load() {
    let mut doc = sample();
    doc.meta.comment = "check the taper".to_string();
    let text = save_text(&doc).expect("serialize");
    let back = load_text(&text).expect("parse");
    assert_eq!(back.meta.comment, "check the taper");
}
```

### Why these two and not one

The model-level test proves the *struct* tolerates a missing key. The io-level
test proves the value survives the whole path — `File::to_text`,
`File::from_text`, `Format::check`, `Document::validate` and
`Document::reseat_id_counter`. `File::from_text` (in `crates/io/src/format.rs`) is where a hostile or
truncated file is capped at `MAX_FILE_BYTES`, the tag is checked, and the id
counter is reseated; a field that round-trips through `serde_json` alone can
still be rejected by any of those.

---

## The failure this recipe is designed to prevent

### A missing `#[serde(default)]` is not a deprecation warning

It is a hard break, in both directions, and it is invisible in review because
the new code reads fine:

- **Old files stop loading.** serde reports *"missing field `comment`"*, and
  `File::from_text` turns that into `FormatError::NotJson`. The user's design is
  still on disk. It simply no longer opens, and the message names a struct
  field rather than anything a person can act on.
- **Nothing in the test suite catches it by default.** Every test in the tree
  builds its fixture with the *current* struct, so every one of them writes the
  new field. The suite is 100% green on a change that makes every real file
  unopenable. Only a test that hand-writes the *old* JSON sees it.

### The shape that bit this tree once

`Property::hint` and `Meta::pending` both lacked `#[serde(default)]` when this
recipe was first written, even though `PropHint` derives `Default` with
`#[default] Number`. Neither was reachable: both fields had existed since
version 1, so every real file already carried them. The test suite was 100%
green on a change that would have made every real file unopenable — only a test
that hand-writes the *old* JSON sees it.

Both now carry `#[serde(default)]`, and both carry a comment saying why. Copy
that, not the shape that was there before: a defaulted field on a new type is
also **not** detectable by any test that uses the type's own constructor, because
the constructor always fills it in.

If you add a field and are unsure whether old files still load, the test to write
is the one that builds the previous version's JSON by hand and feeds it to
`craftkit_io::load_text`.

### And the reverse: a field with a default but no `validate` rule

`#[serde(default)]` makes the field *loadable*; it does not make it *legal*. If
a document must satisfy an invariant — names unique, node count under
`MAX_NODES`, property tree under `MAX_PATH_SEGMENTS` — a hand-edited or old
file can now produce a value that violates it, and `File::from_text` will
refuse the whole file with `FormatError::InvalidDocument`. If your field can be
invalid, add the rule to `Document::validate` in the same change and cover it
with a test that hands `validate` a document containing the bad value.

If the field changes what is *written* in a way an older file's contents cannot
express — a rename, a restructure, a unit — this recipe is not enough. Use
[change-the-file-format.md](change-the-file-format.md).
