# The architecture of craftkit

This is the v2 tree. One sentence says what it is:

> A command is a struct whose fields are its parameters; the compiler writes
> the declaration, the reader and the registration; the registry is a **value**,
> and any crate linked in can put a command in it.

Everything below is that sentence, unpacked. Where this document says a crate
is in a layer, the layer is the one in `crates/craftkit-gates/src/lib.rs`
(`pub const LAYERS`), which is the table the gate reads.

---

## 1. The idea: a list became a registry

The tree this replaces carried its commands in **one list**, in one crate.
That list was closed: adding a command meant editing the list, which meant
every application that wanted the command had to take the edit. The price was
paid by every application that re-purposed the scaffold.

Here the list is a **registry**, and it is open:

- a command is a **value** (`CommandSpec` in `crates/craftkit-sdk/src/command.rs`);
- the values are collected at run time from whatever the binary happens to
  contain — `Registry::from_inventory()` in
  `crates/craftkit-sdk/src/contribution.rs` walks the `inventory` collection
  that the linker filled;
- a crate that is linked in contributes its commands **without anybody naming
  them**.

So the link between a plugin and an application is the registry, and nothing
else. The application does not depend on the plugin's code, and the plugin does
not depend on the application.

---

## 2. The layers

A crate may depend only on crates in **strictly lower** layers. The
applications are exempt: they depend on everything, by design, and every real
layering mistake has been made in a library.

| layer | crates | what it may depend on |
|---|---|---|
| **L0** | `craftkit-alloc`, `craftkit-derive`, `craftkit-ffi-abi`, `craftkit-clap-abi`, `craftkit-vst3-abi`, `craftkit-au-abi`, `craftkit-gates`, `craftkit-link`, `craftkit-image`, `craftkit-jpeg`, `craftkit-deflate`, `craftkit-xml`, `craftkit-payload`, `craftkit-sound-time`, `craftkit-text-layout`, `craftkit-hyphen`, `craftkit-font`, `craftkit-wav`, `craftkit-audio`, `craftkit-midi`, `craftkit-plugins`, `craftkit-scratch` | nothing — the half that knows nothing about a document |
| **L1** | `craftkit-ffi`, `craftkit-clap`, `craftkit-clap-fixture`, `craftkit-vst3`, `craftkit-vst3-fixture`, `craftkit-au`, `craftkit-au-fixture`, `craftkit-png`, `craftkit-zip` | L0 — the doors, their reference fixtures, and the readers that build on the half |
| **L2** | `craftkit-sdk` | L0, L1 — the whole surface a plugin needs |
| **L3** | `craftkit-control`, `craftkit-mcp`, `craftkit-numeric`, `craftkit-raster`, `craftkit-vector-geom` | L0–L2 — the things above the SDK that are not plugins |
| **L4** | `craftkit-plugin-art`, `craftkit-plugin-cad`, `craftkit-plugin-clap`, `craftkit-plugin-film`, `craftkit-plugin-fx`, `craftkit-plugin-grid`, `craftkit-plugin-hello`, `craftkit-plugin-image`, `craftkit-plugin-layout`, `craftkit-plugin-numeric`, `craftkit-plugin-pdf`, `craftkit-plugin-prose`, `craftkit-plugin-shapes`, `craftkit-plugin-shared`, `craftkit-plugin-sound`, `craftkit-plugin-text`, `craftkit-plugin-vector`, `craftkit-plugin-vst3` | L0–L3 — the batteries, each one a document model of its own |
| **L5** | `craftkit-bridge`, `craftkit-hello-server`, `craftkit-mcp-server`, `craftkit-repl`, `craftkit-studio`, `craftkit-window`, `craftkit-wasm` | anything — the applications |

Two rules on top of the table:

- **A plugin may not depend on an application, and may not depend on another
  plugin.** The seam is the SDK, the macro and the libraries, and nothing else.
- **A dev dependency is exempt.** A door's loader tests need a shared library
  to open, and that library is a plugin; neither ships anything.

The gate is `crates/craftkit-gates`: `check(root) -> Vec<Finding>`. It reads
the manifests and the crate roots as text, which is what makes it fast and
hermetic — and what makes it a *reading* of the tree rather than the tree
itself. A manifest written in a form the reader does not know would be read as
no dependencies at all. That is a real limit of the gate, and
`KNOWN-GAPS.md` says so.

---

## 3. The seams

The SDK (`crates/craftkit-sdk`) is the whole surface a plugin needs, and it is
five things:

| | |
|---|---|
| `Params` | the caller's values, read by name and never indexed |
| `ParamKind` | a **type** that knows how to read itself and what it is |
| `CommandSpec` | one command: its id, its parameters, how to run it |
| `Registry` | the contributions, at run time, in any order, from any crate |
| `Session` | the document, the history and the one way to run a command |

### 3.1 The registry and the five contributions

Every kind of contribution is in the same registry, so a plugin can add more
than commands:

| Contribution | What it is |
|---|---|
| `CommandSpec` | something a person or an agent can do |
| `NodeKindSpec` | a kind of thing in the document |
| `ToolSpec` | a gesture that runs a command |
| `PanelSpec` | a window's worth of commands |
| `LayerSpec` | a group of contributions that depend on each other |

Two more are contributions of the format: `FormatSpec` (a version of a file
format) and `MigrationSpec` (a step from one version to the next). The format
and its migrations are described in §3.5.

`Ecosystem::from_inventory()` (`crates/craftkit-sdk/src/ecosystem.rs`) builds
all of them and then **checks them together**, because some mistakes are only
visible across the registries: a tool that names a command that is not there, a
panel that names a command that is not there, a contribution in a layer that
was not linked in, a migration that skips a version. None of those belongs to
one registry, so none of them can be checked by one registry. The ecosystem
**refuses** rather than fixing: a missing command is an error, not a warning.

### 3.2 The document

The document carries the **payload**; the tree is **containment and relations**.
A node may have **no place** — no location and no geometry is not an error.

Derived values are derived and never stored: if it can be worked out from what
is stored, it is not stored.

### 3.3 The session

`Session::execute(id, &params)` is the one way to run a command. It

- refuses an unknown id, a disabled command and an unknown parameter;
- **clones the document** before the command runs;
- runs the command under `catch_unwind` (`run_guarded`);
- **puts the document back** if the command failed or panicked, so a command
  that panics is a command that did not happen;
- refuses a command that changed the document when it promised not to
  (`mutates`), and refuses a document that grew past `MAX_DOCUMENT_BYTES`;
- records the command in the journal and, when it is undoable, in the history.

### 3.4 The evaluator

There are two kinds of evaluator, and both are contributions:

- a **node kind** evaluates **its own nodes** — it knows what a rectangle is,
  and the core does not;
- an **evaluator** evaluates **the whole document** — it may look at every
  node, count them, find the ones that overlap, and say something about the
  whole.

A plan is what the kinds say **and** what the evaluators say, in one value. An
evaluator that panics is skipped, and the plan says what it could not say.

### 3.5 The format

The format is a set of contributions: a **format version** says what a document
of that version looks like and how to read and write it; a **migration** says
how to move a document from one version to the next. `#[derive(Format)]` writes
the reader and the writer, and `#[derive(Migration)]` writes the step.

**A file is read with the version it was written with, and then migrated
forward.** Never with the current version: a file written by an old program has
to be readable by a new one.

---

## 4. How a plugin contributes commands

1. **Write the struct.** Its fields are the parameters:

   ```rust
   use craftkit_derive::Command;
   use craftkit_sdk::{Bool, EngineError, Outcome, Session, Text};

   #[derive(Command)]
   #[command(id = "hello.greet", area = "hello", icon = "hello")]
   pub struct HelloGreet {
       /// Who to greet.
       pub name: Text,
       /// Whether to shout.
       pub loudly: Option<Bool>,
   }

   impl HelloGreet {
       fn run(&self, _session: &mut Session) -> Result<Outcome, EngineError> {
           Ok(Outcome::new(format!("Hello, {}.", self.name.0)))
       }
   }
   ```

   The `label` is optional: with no `label` in the attribute, the macro takes
   it from the id. The `icon` is a **name**; the drawing is the front end's
   business, and the tree holds no icon file.

   **A battery writes a table instead.** The nine batteries of the tree declare
   their commands as **one table per file** — `commands! { … }` — and the
   compiler writes the same struct, reader, registration and spec for every
   row:

   ```rust
   use craftkit_derive::commands;
   use craftkit_sdk::{Bool, EngineError, Outcome, Text};

   commands! {
       /// Say hello to somebody.
       hello.greet {
           area: "hello",
           icon: "hello",
           params: {
               /// Who to greet.
               name: Text,
               /// Whether to shout.
               loudly: Option<Bool>,
           },
           run: {
               Ok(Outcome::new(format!("Hello, {}.", self.name.0)))
           }
       }
   }
   ```

   The row's **id** is the command's id, and the row's name is the id's own
   shape: `hello.greet` is `HelloGreet`. The two spellings make the **same
   registry** — `apps/craftkit-studio/tests/*_registry.rs` pins each
   battery's registry and says whether it changed.

2. **Implement it.** The command runs against the session and returns an
   outcome; an error is `Result`, never a panic.

3. **Register it.** The derive writes the registration, and the registration
   lands in the `inventory` collection. The application never names it. The
   plugin also names its own items with the SDK's anchor macro, so that the
   linker keeps them on every target:

   ```rust
   craftkit_sdk::plugin_anchor!(HelloGreet);
   ```

4. **The ecosystem checks it** when it is built: the id must be well formed,
   the layer must be one that was linked in, and every tool or panel that names
   this command must name a command that exists.

A battery is a plugin with a document model of its own: `craftkit-plugin-hello`
says what a greeting is, `craftkit-plugin-cad` says what a rectangle is, and
each of them adds commands, kinds and panels.

A plugin that is a **program** rather than a linked crate is a third door: it
is spawned, it registers over a wire, and its commands land in the same
registry (`craftkit_sdk::LoadedPlugin`, `craftkit_studio::load_plugin`).

---

## 5. The three doors

Every door reaches the **same** registry. There is no second path into a
design.

### 5.1 The terminal door — `apps/craftkit-repl`

A person at a prompt. The door has a handful of built-ins (`help`, `commands`,
`plan`, `memory`, `snapshot <path>`, `quit`) and runs any command by its id
with JSON parameters:

```text
hello.greet {"name": "world"}
```

The built-ins are commands of the kit with a friendlier shape: `plan` is
`doc.plan`, `memory` is `session.memory`, `snapshot` is `doc.snapshot`. The
door changes nothing itself — it asks, and it prints.

### 5.2 The agent door — `crates/craftkit-mcp` + `apps/craftkit-mcp-server`

MCP over stdio. **Every command in the registry is a tool of its own**, named
`craftkit_<id>` with the dots turned into underscores (`craftkit_hello_greet`),
because a tool name is an identifier and a dot is not one.

The server also answers `prompts/list`, `prompts/get`, `completion/complete`,
`resources/list`, `resources/templates/list` and `resources/read`.

`apps/craftkit-bridge` is the other agent door: JSON-RPC over stdio with
`initialize`, `tools/list`, `tools/call`, `resources/list`, `plan`, `journal`
and `menu`.

### 5.3 The browser door — `apps/craftkit-wasm`

The kit in a page. The browser insists on JavaScript, so this door has two
non-Rust files — `index.html` and `browser-check.mjs` — and they are the only
ones in the tree.

### 5.4 The control channel — `crates/craftkit-control`

Token-authenticated JSON lines over loopback, for driving a **running** program
(the window). Its methods are `engine.execute`, `engine.describe`,
`session.state`, `diagnostics.list`, `ping`, `shutdown`, `ui.render`,
`ui.click` and `ui.drag`. The wire format is in
`crates/craftkit-control/src/lib.rs`.

---

## 6. The front ends

| application | what it is |
|---|---|
| `apps/craftkit-window` | the window: an egui front end over the same registry |
| `apps/craftkit-studio` | the studio: the ecosystem every front end builds |
| `apps/craftkit-repl` | the terminal door |
| `apps/craftkit-mcp-server` | the agent door |
| `apps/craftkit-bridge` | the bridge: JSON-RPC over stdio |
| `apps/craftkit-wasm` | the browser door |
| `apps/craftkit-hello-server` | a small server, used as a fixture |

**The UI is thin.** A front end reads the kit's state and acts through the
registry. It never writes the document, never calls the evaluator, and never
keeps a value the evaluator computes. The gate checks it: `crates/craftkit-gates`
has `FRONT_ENDS` and `FRONT_END_FORBIDDEN` (`document_mut`, `evaluators`,
`Plan`, `PlanItem`, `undo`, `redo`) and refuses a front end that names them.

**One set of camera controls.** The camera is `craftkit_sdk::camera::Camera`:
one type, with its own caps (`MIN_ZOOM`, `MAX_ZOOM`), and the rasteriser
(`crates/craftkit-raster`) draws through it. A new canvas goes through that
camera rather than hand-rolling its own input.

---

## 7. The two gates

- **The tree's own gate** is `crates/craftkit-gates`. It checks the house deny
  attribute, the layering, "a plugin is a plugin", the front end is thin, "the
  bounds are named in the prose", "a battery is a table" (a battery
  declares its commands as rows of a table, and not as structs with
  `#[derive(Command)]`) and "a test does not litter" (a test may not name
  the temporary directory: the **scratch** is the way, and it hands a path
  back when the thread that asked for it ends). The rule that **the numbers
  agree with the prose**
  lives where the code is — `apps/craftkit-studio/tests/bounds.rs` — because
  it compares the README's number with the constant the compiler has already
  tied to it (§850). Run the gate with `cargo test -p craftkit-gates`.
- **The outer gate** is `xtask/` — `cargo xtask doctor`, `cargo xtask ci`, and
  the phases `ci` runs: fmt, assets, cleanroom, lists, layers, web, clippy,
  tests, wasm.

`KNOWN-GAPS.md` says what is true of each of them today, in this tree, without
exaggeration in either direction.
