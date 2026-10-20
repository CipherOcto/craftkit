# Add a command

**When you need this** — any behaviour a person or an agent should be able to
trigger against a document. Adding a node, setting a property, keying a
property, evaluating the plan. If it changes or reads the document, it is a
command. If it opens a dialog, it is *not* a command; see
[the playbook](../agent-playbook.md#the-invariants), invariant 5, and
[docs/patterns/everything-is-a-command.md](../patterns/everything-is-a-command.md)
for the rule itself.

**Before you start**

```
cargo test -p craftkit-engine
```

is green. If it is not, fix that first — you will not be able to tell your own
breakage from the one you inherited.

---

## Steps

### 1. Write the spec in `crates/engine/src/commands.rs`

Find the section for your area — the file is divided by
`// ---- node ----`-style banner comments (the `// ---- node ----`-style banner in `crates/engine/src/commands.rs`
for `node`). Put the new `pub static` in alphabetical position within it, so a
reviewer reading top to bottom sees a sorted manual.

`node.mute` (`NODE_MUTE` in `crates/engine/src/commands.rs`) is the closest template for
anything that addresses a node and flips a boolean:

```rust
pub static NODE_MUTE: CommandSpec = CommandSpec {
    id: "node.mute",
    label: "Mute node",
    area: Area::Node,
    icon: Icon("mute"),
    shortcut: Some("M"),
    doc: "Mute or unmute a node. A muted node passes the level through untouched.",
    params: &[
        ParamDoc::req("node", Doc::NodeRef, "The node's id."),
        ParamDoc::opt("node_name", Doc::Text, "…or its name."),
        ParamDoc::opt("muted", Doc::Bool, "True to mute, false to unmute. Defaults to the opposite."),
    ],
    mutates: true,
    undoable: true,
    enabled: has_nodes,
    run: |s, p| {
        let id = node_by_id_or_name(s.document(), p)?;
        let muted = match p.bool("muted")? {
            Some(b) => b,
            None => !node_mut(s, id, NODE_MUTE.id)?.muted,
        };
        node_mut(s, id, NODE_MUTE.id)?.muted = muted;
        Ok(Outcome::with(
            if muted { "muted" } else { "unmuted" },
            serde_json::json!({ "id": id.get(), "muted": muted }),
        ))
    },
};
```

Rules the surrounding code already enforces and you must match:

- **The id is ours.** Dotted, lower case, ASCII and `.` only, and if
  `area.prefix()` is `Some` the id starts with it (`node.`, `prop.`, `key.`,
  `doc.`, `playback.`, `history.`). `Area::Debug` has no prefix, so its ids
  name what they do — `diagnostics.list`, `plan.evaluate`, `session.stats`.
- **The first parameter is the subject** and must be required if any parameter
  is. See `every_parameter_is_documented_and_uniquely_named`.
- **Read every parameter through `Params`.** `p.text`, `p.bool`, `p.int`,
  `p.num`, `p.num_in`, `p.raw`. Never `p.as_object().unwrap()["x"]`.
- **Return an error, do not clamp.** `doc.range` (`DOC_RANGE` in `crates/engine/src/commands.rs`)
  returns a `EngineError::Param` naming the parameter and the range rather than
  silently substituting `24`.
- **Mutate through `Session`.** `node_mut(s, id, ID.id)?` borrows the document
  through `s`; never cache a `&mut Document` across another call.
- **Return `Outcome::with(message, data)`** so a log line, the CLI and an agent
  all read the same thing.

### 2. Declare the icon name

`Icon("…")` is a **name, not a drawing** (`crates/engine/src/command.rs:99`).
If the name is not one the UI already draws, see
[add-an-icon.md](add-an-icon.md) and do it in the same change — otherwise the
UI's icon test fails and you will not know why.

### 3. Decide `mutates` and `undoable`

These are two different questions and they have two different answers.

| Field | Question | Set `false` when |
| --- | --- | --- |
| `mutates` | Does a *successful* run change the document? | The command only reads — `plan.evaluate`, `diagnostics.list`, `session.stats`, `playback.seek` (`mutates: false, undoable: true`). |
| `undoable` | Should a successful run push an undo entry? | The command restores state by itself, or is not a document edit. |

`history.undo` and `history.redo` are `mutates: true, undoable: false`. The doc
comment on `CommandSpec::undoable` (`crates/engine/src/command.rs:148`) explains
why in full: pushing the restore as a new step makes undo appear to do nothing
and clears the redo branch with its own undo.

`Session::execute` treats these as follows
(it is in `crates/engine/src/session.rs`):

- `mutates: true` and the document is byte-identical afterwards returns
  `EngineError::Internal` with *"the command declared that it mutates the
  document but changed nothing"*. A command that forgot to write anything is a
  bug this catches for you.
- `mutates: true` sets `dirty`, pushes history if `undoable`, and recomputes.
- `mutates: false` does none of that, so a read-only command cannot dirty the
  project or grow the undo stack.

### 4. Set `enabled`

`enabled: fn(&Session) -> bool`. Use the existing helpers in
the helpers section at the bottom of `crates/engine/src/commands.rs`:
`always` or `has_nodes`. If a new command
needs a new condition, add it there.

### 5. Register it in `ALL`

`crates/engine/src/command.rs:219`:

```rust
static ALL: [&CommandSpec; 21] = [
    …
    &NODE_MUTE,
    &NODE_ENABLE,   // new
    &NODE_REMOVE,
    …
];
```

Two things at once, both required:

1. **Bump the declared length** from `21` to `22`. The array is a `static` with
   an explicit length so that `all()` returns `&'static [&'static CommandSpec]`
   rather than a reference to a temporary. The declared length is also what makes
   "added the spec, forgot the table" a compile error instead of a silently
   truncated catalogue.
2. **Insert in `(area, id)` order.** `the_catalogue_is_sorted_and_free_of_duplicates`
   compares each entry's `(a.area, a.id)` with the next and asserts `key < next`.
   `Area` derives `Ord` in declaration order — `Document, Node, Property,
   Keyframes, Playback, History, Debug` — so `Area::Node` entries sort by id
   string: `node.add`, `node.duplicate`, `node.kind`, `node.move`, `node.mute`,
   `node.remove`, `node.rename`.

### 6. Run the gates

```
cargo test -p craftkit-engine
cargo clippy --profile ci -p craftkit-engine -- -D warnings
cargo xtask ci
```

---

## The test you must add

At least one test in the `mod tests` block at the bottom of
`crates/engine/src/commands.rs` — or in `crates/engine/src/session.rs` if the
behaviour is about the session rather than the document. Name it for the
behaviour, not for the function.

```rust
// In `crates/engine/src/commands.rs`, inside `mod tests`.
use crate::session::Session;
use craftkit_core::NodeId;
use craftkit_model::PropPath;
use serde_json::json;

/// The `enabled` property of a node, as it stands now.
fn enabled_of(s: &Session, id: u64) -> Option<bool> {
    let path = PropPath::parse("enabled").ok()?;
    s.node(NodeId(id))?.props.find(&path)?.value.as_bool()
}

#[test]
fn a_node_can_be_disabled_and_re_enabled() {
    let mut s = Session::sample();
    let id = s
        .execute("node.add", &json!({ "name": "Gate", "kind": "filter" }))
        .expect("added")
        .data["id"]
        .as_u64()
        .expect("node.add returns the id in its data");
    assert_eq!(enabled_of(&s, id), Some(true), "a new node starts enabled");

    s.execute("node.enable", &json!({ "node": id, "enabled": false })).expect("disabled");
    assert_eq!(enabled_of(&s, id), Some(false));

    // No `enabled` parameter means the opposite of what it is now.
    s.execute("node.enable", &json!({ "node": id })).expect("toggled");
    assert_eq!(enabled_of(&s, id), Some(true));

    // And the mistake that makes the command unusable rather than merely wrong.
    assert!(
        s.execute("node.enable", &json!({ "node": id, "enabeld": false })).is_err(),
        "an undeclared parameter must be refused"
    );
}
```

### Why these specific tests

Several existing tests already cover the mistakes, and each fails in a
different, expensive way. Do not rely on them alone — they are catalogue-wide
checks that tell you *which command* is wrong, not *why*.

| Mistake | Test that catches it | What it looks like if you get it wrong |
| --- | --- | --- |
| Spec written but not added to `ALL` | The compiler, via the declared array length | A clean build where the command simply does not exist. |
| `ALL` entry out of order | `the_catalogue_is_sorted_and_free_of_duplicates` | Every later PR conflicts on this array. |
| Id not dotted lower case, or not matching its area | `every_id_is_dotted_lower_case_and_starts_with_its_area` | The CLI listing and the MCP tool list drift out of sync with the ids scripts use. |
| A parameter read but not declared in `params` | `every_parameter_is_documented_and_uniquely_named` *if* you notice — but see below | **The command is unusable.** |
| First parameter optional while another is required | `every_parameter_is_documented_and_uniquely_named` | An agent cannot tell what the command operates on. |
| Empty `doc`, empty `help`, empty `label` | `every_id_is_dotted_lower_case_and_starts_with_its_area` and `every_parameter_is_documented_and_uniquely_named` | The MCP tool description and the CLI `--help` say nothing. |
| `mutates: true` on a read-only command | `every_read_only_command_leaves_the_document_alone` (in `crates/engine/src/session.rs`) if it is used as a probe | Every probe dirties the document and pushes undo. |
| `mutates: true` but the run writes nothing | `Session::execute` itself | `EngineError::Internal` on every call. |
| `undoable: true` on an undo/redo command | `undo_then_redo_returns_to_the_same_state` (in `crates/engine/src/session.rs`) | Undo appears to do nothing; redo is gone. |
| Panics on hostile or empty input | `hostile_params_never_panic` (in `crates/engine/src/session.rs`) | A crash for every caller who omits a parameter — which is every caller who got the id slightly wrong. |

---

## The failure this recipe is designed to prevent

### Forgetting a parameter in `params` does not degrade — it disables the command

This is the one that costs the most time, so read it twice.

`Session::execute` runs this before it ever calls your `run`
(it is in `crates/engine/src/session.rs`):

```rust
let p = Params::new(params);
p.deny_unknown(&spec.param_names())
    .map_err(|source| EngineError::Param { id: id.to_string(), source })?;
```

`deny_unknown` compares the keys actually present against
`CommandSpec::param_names()`, which is derived from the `params` table. A key
you read but did not declare is **not an ignored extra** — it is a refusal.
The caller gets `bad-parameter` naming the unknown key, every time, including
when they pass only the parameters you documented.

So a `node.enable` that reads `p.bool("on")` but documents `enabled` returns
`bad-parameter` for `{"node":1,"on":true}`, *and* for `{"node":1}` if `on` were
required, *and* an agent that correctly sends `{"node":1,"enabled":true}` gets
`bad-parameter: unknown parameter 'enabled'`. Nothing works, nothing panics,
nothing looks like a bug in your `run` function, and the parameter table looks
perfectly reasonable. The existing test `every_parameter_is_documented_and_uniquely_named`
only checks that what you declared has help text and a unique name — it does
not check that what you *read* is declared. That gap is why your own test has
to actually call the command with its own parameters.

### The other three worth naming

**Out-of-order `ALL`.** The array is the one file every command change edits.
It is hand-written on purpose — `crates/engine/src/command.rs:205` says deriving
it from a macro "would save three lines and lose the ability to read the
catalogue top to bottom, which is how a reviewer checks that a new command has
a sensible neighbour and no duplicate id". Unsorted, it produces a merge
conflict in almost every concurrent command PR, and the resolution is always
guesswork.

**Inventing a command id that copies another product's vocabulary.** The ids
are a public interface: scripts, the MCP tool list and the control channel all
use them. Borrowing another application's id would make craftkit's tools look
like theirs and would tie the naming to a product craftkit does not and will
not depend on.

**Adding behaviour to the UI instead of to a command.** If the button works but
`engine.execute` cannot do it, the agent-facing half of the product does not
exist. The button should call
`Session::execute("node.enable", …)` — there is no second implementation.
