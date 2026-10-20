# Change the file format

**When you need this** — a design saved by an older craftkit cannot be read
correctly by this one, and *rewriting* it will fix it. A rename, a restructure,
a unit change, a re-typed value. If the old file simply needs an extra field
filled in with a default, use
[add-a-saved-field.md](add-a-saved-field.md) instead — that needs no pending
file.

This is the how-to.
[docs/patterns/format-changes.md](../patterns/format-changes.md) is the why,
and `crates/io/src/format/pending.rs` — which its own module docs point at — is
the rule in code. Read the pattern first if the trade-off is not obvious to you.

**Before you start**

1. Read `crates/io/src/format/pending.rs` top to bottom. It is the rule.
   Everything below is that rule written out as steps.
2. Prove the problem is real:

```
cargo test -p craftkit-io
```

then write the *old* JSON by hand into a test and watch it fail. Do not go
further until you have a failing test.

---

## The rule, in one line

**A change that alters what is saved adds one new file in
`crates/io/src/format/pending/`.** It does not edit `CURRENT`, it does not edit
the reader, and it does not touch an already-released version.

---

## Steps

### 1. Add exactly one file: `crates/io/src/format/pending/<name>.rs`

The name is `snake_case` — lower-case letters, digits and `_` — and it starts
with a letter, because the name is also a Rust module. The file, the `FILES`
entry and the `mod` are **the same word**. That is enforced by
`every_pending_name_is_a_plausible_file_name` and by
`the_files_on_disk_and_the_table_agree` (both in
`crates/io/src/format/pending.rs`).

> **This used to say three different things.** The name rule here allowed
> lower-case letters, digits and `-`; the `FILES` entry was kebab-case while
> the file was snake_case; and the pattern document's example used a name that
> is not a Rust identifier at all. A name Rust could compile was refused by the
> test, and a name the test accepted could not be compiled — so **no working
> change could satisfy the tree**. §81 has the measurements.

Pick a name for the *change*, not the field: `mute_flag_to_node_field.rs`, not
`muted.rs`.

A complete, minimal worked example — moving `Node::muted` out of the node
struct and onto a property named `enabled`, which is what a user-facing
"disabled" toggle actually means in this document model:

```rust
//! Move the mute flag out of the node and onto its `enabled` property.
//!
//! Until now a node carried `muted: bool` beside its property tree, and the
//! evaluator skipped the node when it was set. That made the flag invisible in
//! the property panel, so it could not be animated, keyframed or scripted.
//!
//! This step rewrites every node's `muted` flag as an `enabled` property with
//! the opposite value, and clears the flag, so the property is the only place
//! the state lives.
//!
//! Node ids and property ids are unchanged: a script written against this file
//! before the change keeps working, because a node is still addressed by id.

use craftkit_core::Value;
use craftkit_model::{PropPath, Property};

/// Rewrite a version-1 document into the shape version 2 expects.
pub fn step(doc: &mut craftkit_model::Document) {
    // Parsed once: the path grammar is `craftkit-model`'s, not ours.
    // `PropPath::parse` gives an `Option`, not a `Result`.
    let Some(enabled) = PropPath::parse("enabled") else { return };
    for node in &mut doc.nodes {
        let was_muted = node.muted;
        node.muted = false;
        if node.props.find(&enabled).is_some() {
            // A file that already has the property wins: it is the newer
            // statement about what this node should do.
            continue;
        }
        let next = node.props.nodes.iter().filter_map(|n| match n {
            craftkit_model::PropNode::Prop(p) => Some(p.id),
            _ => None,
        }).max().unwrap_or(0) + 1;
        node.props.push(Property::new(next, "enabled", Value::Bool(!was_muted)));
    }
}
```

Two things to copy from this skeleton:

- **The `//!` note says what changes, why, and what is deliberately left
  alone.** A reviewer deciding whether two pending changes conflict reads only
  those notes. "Node ids and property ids are unchanged" is the sentence that
  tells them the two changes are compatible.
- **`step` returns nothing and must not fail.** It is called from the loader
  after parsing, on a document that has already been read. If the rewrite
  cannot be expressed without error, say so in the `//!` note and refuse the
  older version in the reader instead — see step 5.

### 2. Declare the module

In `crates/io/src/format/pending.rs`, add the `mod` next to the `FILES` table
and the matching entry in `FILES`, sorted:

```rust
const FILES: &[&str] = &["mute_flag_to_node_field"];
```

and, above `FILES`:

```rust
mod mute_flag_to_node_field;
```

The file, the table entry and the `mod` are the same word, in `snake_case`.
`the_pending_list_is_sorted` (in `crates/io/src/format/pending.rs`) keeps the
table in order so a release is a `mv` and not a judgement call.

### 3. Wire the step into `apply`

`apply` exists for this and currently dispatches nothing: `step_for` is the
place a step is named, and it is empty. Replace its body with the match the
comment in it already sketches:

```rust
fn step_for(name: &str) -> Option<fn(&mut craftkit_model::Document)> {
    match name {
        "mute_flag_to_node_field" => Some(mute_flag_to_node_field::step),
        _ => None,
    }
}
```

`apply` collects the steps the names give and runs them in order, each
seeing what the previous one produced. That ordering is the contract; it is
why `FILES` is sorted. The runner is `run_steps`, and it has a test that
runs two synthetic steps and checks both the order and the chaining — §45.

### 4. Make sure `apply` is actually called

The loader calls it. `File::from_text` in `crates/io/src/format.rs` does:

```rust
let mut file: Self = serde_json::from_str(text).map_err(|e| FormatError::NotJson(e.to_string()))?;
file.check_with(&File::known_changes(steps))?;
// Pending rewrites run before validation, because a `step` is exactly
// the thing that fixes an old file's shape into this version's.
//
// Only the changes the file has **not** had: a file this build wrote
// already has them, and applying them again would rewrite it. §81.
let already = file.meta.pending.clone();
pending::apply_steps(&mut file.document, &already, steps);
file.document.validate().map_err(|e| FormatError::InvalidDocument(e.to_string()))?;
```

The order is the whole point: the document has to be a `Document` before it
can be stepped, and it has to be stepped *before* it is validated, or
validation runs against the old shape. The `already` list is the other half:
it is the file's own record of the changes it has had, and it is what keeps a
file this build wrote from being rewritten when it is read back.

The `check_with` line is the **other** direction, and it is the one a change
author has to know about. The file's list is not only a list of things to skip:
a file that names a change this build has never heard of was written by a
build whose shape this build does not know, and the reader refuses it rather
than dropping the parts it does not recognise. §95. So the reader asks two
questions of the same list — *what does the file still need* and *what has it
had that I do not have* — and a change author's name has to be in the table
for the file that carries it to be readable at all.

> **This note used to say the opposite.** It claimed that the loader did not
> call `apply` at all, and told the reader to add the call if it was "still
> missing". The call was there; the note was not. A note in a recipe is not
> a citation, so nothing noticed it going stale — §45.

### 5. Do not touch `CURRENT`

`pub const CURRENT: u32 = 1` (the `CURRENT` constant in `crates/io/src/format.rs`) names the version
*this build writes*. `Format::check` refuses anything with a higher number and accepts anything
lower. A pending change
does not move it. A release does.

If a pending step must be able to *refuse* an old document rather than guess,
that is a different change and it belongs in the reader, with a
`FormatError::TooOld` — say why in the PR.

### 6. Run the gates

```
cargo test -p craftkit-io -p craftkit-engine
cargo xtask ci
```

---

## How a release folds the change in

At version 1 there is nothing pending, and the tree shows no released version
directory yet:

> **Note** — the folding machinery exists. `docs/patterns/format-changes.md`
> is the pattern, `scripts/fold-format-pending.sh` is the release step, and
> there is no `v<N>/` directory under `crates/io/src/format/` yet, because
> nothing has been folded. The steps below are what the script does; run it
> with no arguments to see what it *would* do, and with `--apply` to do it.

> **This note used to say the opposite.** It claimed that the pattern
> document and the release script "do not exist in the tree yet", and that
> `ls scripts` is empty. All three were false: the document is in
> `docs/patterns/`, the script is in `scripts/`, and `scripts/` holds three
> of them. It was the same failure as the note in step 4 — a note about the
> tree, written once and never checked. §45.

The intent, from the module docs at
`crates/io/src/format/pending.rs` ("The release folds everything in this
directory into the next version and empties it"):

1. Take every `pending/*.rs` **in `names()` order** and concatenate them into
   `crates/io/src/format/v2/`, one `mod` per change, in the same order.
2. Make `v2::apply` what `pending::apply` is today: the same arms, calling the
   same `step` functions.
3. In the reader, dispatch on the version the file claims and call
   `v1::apply` for anything older. Files claiming `craftkit/1` go through the
   steps that were pending when 2 was cut; files claiming `craftkit/2` skip
   them.
4. Bump `CURRENT` to `2`. Only now do `Format::check` and `tag()` move.
5. Delete the pending directory's contents.

> **The script and this list do not agree, and the disagreement is open.**
> `scripts/fold-format-pending.sh` moves the files into
> `crates/io/src/format/pending/v2/` — not `crates/io/src/format/v2/` — and
> tells the releaser to put `FILES` back to `&[]` and `apply` back to "the
> loop with no arms". After that, **nothing calls a folded step**: the files
> are not compiled (there is no `mod v2;`), not called, and not read by
> anything. A design written before the change would then load without its
> rewrite unless the reader understands the older shape by hand.
>
> Which of the two is the intent — the script's history-only fold, or this
> list's version dispatch — is the owner's call, and `KNOWN-GAPS.md` §81
> records it as the open half. The script now says so out loud rather than
> letting the next release produce dead code quietly.

The reason for the ceremony is in the module docs and is worth repeating:
bumping the number and editing the reader in one diff means two people doing it
at once conflict in the one file everybody edits, and the losing change silently
disappears. One file per change cannot conflict that way.

---

## The test you must add

### A. The pending module's own test: the old file becomes the new one

Inside `crates/io/src/format/pending/<name>.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_muted_node_becomes_an_enabled_property_that_is_false() {
        let mut doc = craftkit_model::Document::empty("old");
        let id = doc.add_node("Echo", NodeKind::Filter).expect("a node");
        doc.get_mut(id).expect("the node").muted = true;

        step(&mut doc);

        let node = doc.get(id).expect("still one node");
        assert!(!node.muted, "the flag is gone");
        let path = craftkit_model::PropPath::parse("enabled").expect("a path");
        let prop = node.props.find(&path).expect("the property replaced the flag");
        assert_eq!(prop.value, craftkit_core::Value::Bool(false));
        assert!(doc.validate().is_ok(), "the rewritten document is legal");
    }
}
```

### B. The end-to-end test: a version-1 file loads through `craftkit-io`

In `crates/io/src/format.rs`'s test module, beside
`this_builds_own_file_round_trips`:

```rust
#[test]
fn a_file_written_before_the_pending_change_loads_and_is_rewritten() {
    let old = r#"{
        "format": "craftkit/1",
        "document": {
            "meta": { "name": "Old", "fps": 24, "start": 0, "end": 1270080000000 },
            "nodes": [
                { "id": 1, "name": "Echo", "kind": "filter",
                  "props": { "id": "props", "nodes": [] },
                  "muted": true }
            ]
        }
    }"#;
    let file = File::from_text(old).expect("a version-1 file loads");
    let node = file.document.get(craftkit_core::NodeId(1)).expect("the node survived");
    let path = craftkit_model::PropPath::parse("enabled").expect("a path");
    assert_eq!(node.props.find(&path).expect("the step ran").value, craftkit_core::Value::Bool(false));
}
```

### Why these two

- **A alone does not prove the loader calls the step.** `apply` is a function
  that nothing currently invokes (see step 4). A passes while the feature is
  dead.
- **B alone does not prove the rewrite is correct.** It only proves a file went
  in and a file came out; if the step were a no-op, B would pass as long as
  `enabled` already existed. A asserts the step does the work.

Also keep the pending module's own tests green — they are the ones that stop
the table rotting:

- `the_pending_list_is_sorted` (in `crates/io/src/format/pending.rs`)
- `every_pending_name_is_a_plausible_file_name` (in
  `crates/io/src/format/pending.rs`) — this is the one that says the name is a
  Rust module name, and it is what refused every name that could work before
  §81.
- `every_listed_change_is_declared_and_applied` and
  `every_pending_file_is_wired_into_apply` (in
  `crates/io/src/format/pending.rs`) — these two are the tripwires for the
  wiring. Both used to fail **on correct work**: the first asked for a
  `{name}::step(doc)` spelling no documented pattern produces, and the second
  asserted that the table was empty, so it fired the moment a change was
  pending. §81.

The end-to-end test also has a companion worth writing: a file this build
wrote must read back **unchanged**. The reader applies only the changes a file
has not had, and the file's header records which ones it has — so the
round-trip tests in `crates/io/src/format.rs` are what prove it.

---

## The failure this recipe is designed to prevent

### Editing the reader instead of adding a file

The tempting change is one diff: bump `CURRENT`, add a `match` arm on the old
version in `File::from_text`, done. It reads beautifully and passes every
existing test. Then two people do it at once — one renaming a field, one
retyping a value — and both edit `File::from_text` and `CURRENT`. Whoever
merges second resolves a conflict in the single most contended file in the
crate, has no way to tell which arm was theirs, and drops one of the two
rewrites. Nothing fails. Some number of people's designs silently load with one
change half-applied.

The test that catches this is not a test — it is the diff. If your change
touches `CURRENT`, `Format::check`, `File::from_text`, or anything already in
`crates/io/src/format/`, it is the wrong shape unless the release is what you
are doing.

### A `step` that is "clever"

A pending step runs on documents nobody controls: files from a build nobody
remembers, edited by hand, produced by a script that stopped running years ago.
A step that guesses is a step that produces a *different* document on every
run, silently. When a document does not fit the rewrite, refusing it loudly
(`FormatError::TooOld`, which `SaveError::is_recoverable` at
`crates/io/src/store.rs:43` already treats as recoverable) is better than
producing a plausible wrong answer.

### A step that renumbers something

Node ids and property ids are what scripts address. `Document::reseat_id_counter`
(`reseat_id_counter`, the last line of the reader in `crates/io/src/format.rs`) exists because a file must never hand out an id
that already belongs to something, and a pending step that invents ids
undermines exactly that. Ids survive renames and reorders; that is why
`Property` carries both `id` and `label` (`crates/model/src/property.rs:24`).
A format change that reassigns ids is a rewrite of everyone's scripts, not a
compatibility fix — raise it, do not ship it as a `pending` step.
