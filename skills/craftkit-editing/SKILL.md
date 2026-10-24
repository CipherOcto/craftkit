---
name: craftkit-editing
description: Use when changing craftkit's Rust code or a .craftkit design — adding or editing a command, a property, a crate, an icon, a file-format field, or a command's run/enabled body — and before claiming such a change is finished.
---

# Editing craftkit

craftkit is a scaffold: a parametric plan document that evaluates to a `Plan`, one
command registry, and six ways to reach it. You are editing it, or re-purposing it
into an application.

Read this before your first edit. It is the set of invariants that are easy to break
and expensive to notice later.

---

## 1. The map

| Crate | Layer | What lives there |
|---|---|---|
| `crates/core` | L0 | `Tick`, `Value`, `NodeId`, **`Params`** — the only place a parameter is read |
| `crates/model` | L1 | `Document`, `Node`, `PropGroup`, `PropPath`, `Keyframes` |
| `crates/kernel` | L2 | the numeric ops, behind the panic guard |
| `crates/eval` | L3 | document → `Plan` |
| `crates/io` | L3 | the `.craftkit` file: `format.rs`, `format/pending/` |
| `crates/engine` | L4 | `CommandSpec`, the 21-command registry, `Session::execute`, undo, crash reports |
| `crates/control` | L5 | the loopback JSON-lines channel |
| `crates/mcp` | L5 | MCP tools over stdio |
| `crates/ui` | L6 | `shell`, `icons`, `automation`, `theme`, `raster` — the only crate that may name egui |
| `apps/*`, `xtask` | exempt | the binaries and the gates |

The layer table is `TABLE` in `xtask/src/layers.rs`. It is the rule, and
`docs/architecture.md` must agree with it.

---

## 2. Everything is a command

**A new user-visible behaviour is a `CommandSpec` in `crates/engine/src/commands.rs`
plus tests.** The toolbar, `engine.describe`, the CLI, the control channel, a script
and the MCP server all reach the same `Session::execute`. If you add behaviour
outside a command, it has no undo entry, no log line, no crash reproduction and no
schema — and the project's central claim becomes false.

### The id rule

`<area>.<thing>[.<variant>]` — dotted, lower case, ASCII, **no underscores**, no
uppercase, no abbreviations borrowed from another product. Area prefixes come from
`Area::prefix` (`crates/engine/src/command.rs:52`):

| `Area` | prefix | panel label |
|---|---|---|
| `Document` | `doc` | Document |
| `Node` | `node` | Nodes |
| `Property` | `prop` | Properties |
| `Keyframes` | `key` | Keyframes |
| `Playback` | `playback` | Playback |
| `History` | `history` | History |
| `Debug` | *(none)* | Debug |

`Debug` deliberately has no prefix: `plan.evaluate`, `diagnostics.list` and
`session.stats` are cross-cutting, and a shared prefix would name them after the
panel instead of after what they do.

The 23 ids today, sorted by (area, id) — the registry's own order
(`crates/engine/src/command.rs`):

```text
doc.note        node.add        prop.reset      key.clear      playback.seek
doc.range       node.duplicate  prop.set        key.ease       history.redo
doc.rename      node.kind                       key.list       history.undo
                node.move                       key.remove     diagnostics.list
                node.mute                       key.set       plan.evaluate
                node.remove                                    session.stats
                node.rename
```

### The shape of a `CommandSpec`

`crates/engine/src/command.rs`, in `CommandSpec`:

| Field | Type | Notes |
|---|---|---|
| `id` | `&'static str` | the rule above; the registry is sorted and duplicate-free |
| `label` | `&'static str` | what a button and an undo entry say |
| `area` | `Area` | decides the panel and the id prefix |
| `icon` | `Icon` | `Icon(pub &'static str)` — **a name, not a drawing** (in `Icon`) |
| `shortcut` | `Option<&'static str>` | the chord the command is advertised with; the tree binds no keys yet |
| `params` | `&'static [ParamDoc]` | `ParamDoc { name, kind: Doc, required, help, alternative }` — `alternative` names the parameter that may be given **instead**, and then the pair is "one of the two" |
| `asks` | `&'static [&'static str]` | the parameters a person is asked for when the command is started from the interface; a name in it must be one of `params`, and every parameter the command cannot do without must be in it (§118) |
| `doc` | `&'static str` | one line, used by `commands --json` and the MCP description |
| `mutates` | `bool` | does it change the document |
| `undoable` | `bool` | does a success push an undo entry |
| `enabled` | `fn(&Session) -> bool` | availability right now |
| `run` | `fn(&mut Session, &Params<'_>) -> Result<Outcome, EngineError>` | the whole behaviour |

`ALL` must stay sorted (in `command.rs`, in `ALL`). `get` (same file)
looks a shipped command up; `lookup` (same file) also resolves the test-only
`test.panic` in test builds.

### Adding a command, in order

1. Add the spec to `ALL` in `crates/engine/src/commands.rs`, in sorted-by-id position.
2. Read every parameter with `Params` (section 3). Never index.
3. If it changes the document: `mutates: true`, and `undoable: true` unless the
   change should not be undoable (say why in a comment).
4. Write `enabled` from real state — `has_nodes` (`crates/engine/src/commands.rs`) or
   `node_mut` (in `crates/engine/src/commands.rs`) style helpers. A command that is always enabled and then
   fails is worse than one that greys out.
5. Add the icon **name** to `crates/ui/src/icons.rs` if the set has no such shape
   (section 6). `every_command_has_an_icon_this_set_can_draw`
   (`icons.rs:252`) fails otherwise.
6. Tests, in the same file: the happy path, a hostile parameter value, and one
   refusal. See `docs/recipes/add-a-command.md`.
7. Re-run the gates (section 10).

Nothing else needs touching. The CLI catalogue, the MCP tool list, `engine.describe`
and the docs' parameter tables are all generated from `CommandSpec` / `ParamDoc`.

---

## 3. Never index a parameter

`crates/core/src/params.rs` is the only place a command parameter is read.

```rust
// never
let distance = params["distance"].as_f64();          // panics on a typo
// always
let Some(distance) = p.num("distance")? else { return Ok(no_change()) };
```

Parameters arrive from the control channel, from MCP, from scripts and from files —
all hostile. `hostile_params_never_panic` in the engine fires every registered
command at every documented parameter name with every hostile value; a single
`[i]` or `.unwrap()` in a `run` body makes that test fail.

| Reader | Line | Use it for |
|---|---|---|
| `num(key)` | `crates/core/src/params.rs` | a number; missing / null / string → `Ok(None)` |
| `num_in(key, lo, hi)` | `crates/core/src/params.rs` | a number the caller has already decided bounds for |
| `int(key)` | `crates/core/src/params.rs` | a whole number; **refuses** a fraction rather than truncating |
| `bool(key)` | `crates/core/src/params.rs` | a flag |
| `text(key, max)` | `crates/core/src/params.rs` | a string with a byte cap, so a 100 MB value is refused on read |
| `color(key)` | `crates/core/src/params.rs` | `"#rrggbb"` or `[r, g, b]` |
| `num_list(key, max)` | `crates/core/src/params.rs` | a homogeneous list with a caller-supplied cap |
| `has(key)` | `crates/core/src/params.rs` | was the parameter given at all? `null` counts as absent |
| `raw_json(key)` | `crates/core/src/params.rs` | the raw `Json`, when you must interpret it yourself — **the two reader modules only**; the gate refuses a third (§136) |
| `deny_unknown(allowed)` | `crates/core/src/params.rs` | reject a misspelled key instead of ignoring it |

`Session::execute` already calls `deny_unknown` for every command
(in `crates/engine/src/session.rs`), so an agent's typo is already loud — do not repeat it by hand.

For a time, use the engine's helper rather than converting yourself
(`commands.rs:~800`): a JSON **number is seconds**, and a string may carry a suffix
— `"1.5s"` seconds, `"45f"` frames, `"120"` raw ticks. Internally it is always an
exact integer `Tick`; seconds are an input format, never a storage format.

---

## 4. Clone before edit; one trust boundary

`Session::execute` (in `crates/engine/src/session.rs`) is the only place a
document changes, and it is deliberately boring:

```text
lookup id            unknown id  -> EngineError::Unknown
enabled?             disabled    -> EngineError::Disabled
Params::new + deny_unknown         -> EngineError::Param
let snapshot = self.doc.clone()
catch_unwind(|| (spec.run)(self, &p))
  Ok(Ok(outcome))  -> mutates? compare with snapshot, push undo, mark dirty
  Ok(Err(e))       -> restore snapshot
  Err(panic)       -> restore snapshot, report EngineError::Internal
```

Three consequences:

- **New mutation goes in `run`, never around the boundary.** Do not add a second
  `catch_unwind`, do not write to the document before `execute`, do not "optimise"
  the clone away.
- **The clone is unconditional** — read-only commands clone too — so the restore
  path is a single assignment with no branch.
- **A refused command leaves nothing half-applied.** If your `run` mutates and then
  returns `Err`, the rollback undoes the whole thing; write `run` as if it were
  transactional, because it is.

`Session::load_document` (in `crates/engine/src/session.rs`) exists for
loading a file, not for commands. It clears the undo stack and the log — they
belong to the document that was there before — and starts the command trace
again, so a crash report replays onto the document it was made from.

---

## 5. `mutates` and `undoable` are different questions

| | `mutates: true` | `undoable: true` |
|---|---|---|
| asks | does this change the document? | should a success be undoable? |
| effect | marks the session dirty and recomputes | pushes the snapshot onto the undo stack |
| must the document actually change? | **yes** | no |

A command that declares `mutates: true` and changes nothing is **not** a silent
no-op: `execute` compares the document with the snapshot and returns
`EngineError::Internal` — "declared that it mutates the document but changed
nothing" (in `Session::execute`). That is the mechanism that catches the common
mistake, a command that declares `mutates: true` and forgets to write.

So: set `mutates: true` only when a successful run really does change something;
set `undoable: false` for a mutation that must not be undoable (and say why);
set both `false` for `plan.evaluate`, `diagnostics.list`, `session.stats`.

A read-only command that marks the project dirty is the other half of the same bug.

---

## 6. Icons are drawn in code

`Icon("name")` is a name. `crates/ui/src/icons.rs` turns names into strokes, from
original coordinates in `0.0..=1.0`, using `line`, `path`, `circle`, `disc`, `rect`.

- **Never an image file.** No PNG, SVG, font glyph or traced screenshot — from any
  source, commercial or free. `cargo xtask assets` fails on an embedded
  asset and on an unattributed one.
- To add a name: add it to `ALL` (`icons.rs:29`) **in sorted position**
  (`the_name_list_is_sorted_and_free_of_duplicates`, in `crates/ui/src/icons.rs`), then write the
  painter method. Lower-case ASCII with `-`, dotted names allowed
  (`every_name_is_dotted_lower_case`, in `crates/ui/src/icons.rs`).
- `mute` (in `icons.rs`, in `Painter`) is the shortest complete example. `docs/recipes/add-an-icon.md`
  is the full recipe.

---

## 7. A file-format change is exactly one new file

Never edit `crates/io/src/format.rs` (`CURRENT`, the reader, the writer) and never
touch another change's file.

1. Create `crates/io/src/format/pending/<name>.rs`, lower case with dashes.
2. Start it with a `//!` note: what is now saved, what an older file gets, and that
   the last release asks for an update instead of meeting it.
3. Add `pub fn step(doc: &mut Document)` **only if older files must be rewritten**.
4. Add `mod <name>;` **and** a line in `FILES` (in `crates/io/src/format/pending.rs`).
   `the_pending_list_is_sorted` keeps the table honest.
5. Leave `apply` (in `crates/io/src/format/pending.rs`) to call the new step in order — one arm per change in `step_for`.

`FILES` is empty today: nothing is pending at format version 1
(`format.rs:33`). See `docs/recipes/change-the-file-format.md`.

---

## 8. Adding a crate

1. Pick the layer from `TABLE` (`xtask/src/layers.rs:64`). Depend only on **strictly
   lower** layers.
2. `crates/ui` (L6) is the first layer allowed to name a UI crate
   (`UI_CRATES` and `UI_MIN_LAYER`, in `xtask/src/layers.rs`). No egui / eframe /
   winit / wgpu /
   rfd below it.
3. Put both attributes in the crate root, at the top of `src/lib.rs`:

```rust
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]
```

4. Add the crate to `TABLE` **and** to the table in `docs/architecture.md`, in the
   same change. `cargo xtask lists` fails if the two disagree
   (`the_real_architecture_table_matches_the_code_table`).
5. `members` in the root `Cargo.toml` is globs (`apps/*`, `crates/*`, `xtask`) — a
   new directory under either is picked up automatically; a new *top-level* member
   needs a line, and it must stay sorted.
6. Every crate root must carry the deny attribute. The check normalises whitespace
   and case and ignores the `clippy::` prefix, so **formatting does not matter** —
   but the **set of lints and their order do** (`a_deny_in_a_different_order_is_not_the_house_one`,
   `xtask/src/lists.rs`).

---

## 9. Editing a `.craftkit` design

- Property paths are **label-based**: `amount`, `trigger/lead`, `position#2` for the
  second sibling with that label (the grammar is in the module doc of
  `crates/model/src/path.rs`; the listing is `PropGroup::flatten` in
  `crates/model/src/property.rs`). `PropGroup::id` is the stable name; the label
  is what a path matches.
- Segments: `@<hex id>` (stable across a reorder), `#<n>` (n-th child, 1-based),
  `name#<n>`, and `name` as shorthand for `name#1`. `.` never separates; `/` does.
- **A path names things that exist.** An id and a position name a property or a
  child that is already there, so they can *change* a property but not *create*
  one: `prop.set` with `@1f4a` or `effects/#2/amount` is refused when there is
  no such property or child. Only names may create (`trigger/lead` makes the
  group and the property). The model creates properties, in `PropGroup::insert`.
- A path is at most `MAX_PATH_SEGMENTS` segments, and the property tree is
  bounded by the same number: a property deeper than a path can name is one
  nothing can reach, so such a tree is refused rather than truncated. §85.
- **A name is written as it is.** A label or group id that contains `/`, or
  begins with `@` (an id) or `#` (a position), cannot be a path segment, so
  the tree refuses it. A name the grammar can only write with an explicit
  occurrence — `a#b`, `C#` — is published as `a#b#1`, `C##1`. §88.
- Instants are integers of `Tick`. A path is not stable across a reorder; a property
  id is.
- To change a design by hand, prefer a command over a text edit — the file format is
  for storage, and every behaviour is a command.

---

## 10. Pre-flight, before you start

```
cargo xtask doctor      # can this machine build it at all?
cargo xtask lists       # are the shared tables currently green?
cargo test -p craftkit-engine     # is the layer you are touching green?
```

These are `cargo xtask <cmd>`. `.cargo/config.toml` aliases it to
`run --quiet -p xtask --`, so the build tool is an ordinary workspace member with
its own tests and its own row in the layering table rather than a script outside
the build.

Before you finish writing:

- [ ] Every new behaviour is a `CommandSpec` with tests — not a special case in a caller.
- [ ] Every parameter read goes through `Params`; no `[]`, no `.unwrap()`, no `[i]`.
- [ ] A format change is one new file in `crates/io/src/format/pending/`.
- [ ] A new icon is drawn in `crates/ui/src/icons.rs`; no image file.
- [ ] A new crate has both attributes, a `TABLE` row and a matching
      `docs/architecture.md` row.
- [ ] Unsorted lists you touched stayed sorted.

## 11. Done

```
cargo fmt --all
cargo xtask ci
```

`ci` runs fmt, anchors, assets, cleanroom, lists, layers, web, clippy
`-D warnings --all-targets`, tests, fuzz and the wasm check. It stops at the
first failure. The list is the one in the checking skill's table, and a gate
holds it there.

- [ ] `cargo xtask ci` is green end to end.
- [ ] The deny attribute is present in any new or touched crate root, with the
      house set and order (formatting is free).
- [ ] The layer table in `docs/architecture.md` matches `TABLE` for anything you added.
- [ ] `hostile_params_never_panic` still passes if you touched a `run` body.
- [ ] Nothing was committed. **This tree is not a git repository**
      (`git rev-parse` fails), so there is nothing to push; leave the files on disk.
- [ ] Never commit `target*/`. Do not add generated output or a lock-file churn to a
      change that did not need it.