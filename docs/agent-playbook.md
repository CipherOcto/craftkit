# Agent playbook

You are working in this repository. This is the document that tells you how to
become useful in it in five minutes, what you must never break, how to check
that you did not, and when to stop and ask.

Read it once now. Read `docs/recipes/README.md` when you have a concrete task.

---

## The five-minute orientation

In this order. Stop when you have answered the question you came with.

1. **`crates/core/src/lib.rs`** — the base. `Value` (the one type that crosses
   every boundary), `Params` (the only way to read a parameter), `NodeId`,
   `IdCounter::MAX`, `MAX_MAGNITUDE`, `MAX_TEXT`, and `read_line_bounded` (the
   reader that applies a limit before the allocation). It also holds the
   numbers every reader of untrusted JSON derives its limit from
   (`MAX_JSON_TEXT_BYTES`), and it explains why the rest is shaped the way it
   is.
2. **`crates/model/src/document.rs`** — what a design *is*. `Document`,
   `Node`, `DocumentMeta`, `Node::set_defaults`, `validate`. Then
   `crates/model/src/property.rs` for the property tree. Plain data, no I/O.
3. **`crates/engine/src/session.rs`** — `Session::execute`. This
   one function is the heart of the product: it resolves the command, checks
   `enabled`, runs `deny_unknown`, snapshots the document, runs `run` under
   `catch_unwind`, and either pushes undo or restores the snapshot. Everything
   about panic safety, undo and the parameter contract is in it.
4. **`crates/engine/src/command.rs`** — the shape of a command: `CommandSpec`,
   `Area`, `ParamDoc`, `Icon`, and `ALL`. Then skim
   `crates/engine/src/commands.rs` — it is the manual, and its first part is
   worked examples; its tests are at the end of the file.
5. **`crates/io/src/format.rs` and `crates/io/src/format/pending.rs`** — what is
   saved, and how a format change is made. It is the rule.

Then, for whatever you are doing, the matching recipe:

| Task | Recipe |
| --- | --- |
| new behaviour on a document | [recipes/add-a-command.md](recipes/add-a-command.md) |
| a new field on something serialized | [recipes/add-a-saved-field.md](recipes/add-a-saved-field.md) |
| older designs must be rewritten | [recipes/change-the-file-format.md](recipes/change-the-file-format.md) |
| a command needs an icon | [recipes/add-an-icon.md](recipes/add-an-icon.md) |
| an agent needs a new capability over the socket | [recipes/add-a-control-channel-method.md](recipes/add-a-control-channel-method.md) |
| a genuinely new layer of code | [recipes/add-a-crate.md](recipes/add-a-crate.md) |
| something panicked | [recipes/fix-a-crash.md](recipes/fix-a-crash.md) |

**What this project is**, if you have not worked out from the code: a parametric
3D/CAD-style application where a document is a graph of nodes, each with a
property tree, some of whose properties are keyframed over an integer tick
timeline. Everything a person can do to a document is a command in one registry,
and that same registry is what scripts, the CLI, the control channel and the MCP
server call. That is the whole design idea: **the tool an agent calls is the
tool a person clicks**, so the two cannot drift.

---

## The invariants

Never break these. Each row is the rule, the one-line reason, and the check that
catches you.

### 1. Never panic in non-test code

`unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, `unreachable!`, `unsafe`.

**Why:** people trust this application with their designs, and a crash loses
their work. This outranks feature work.

**Check:** every crate root carries the deny block in
[architecture.md](architecture.md#the-deny-attribute-exactly) —

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

— and `cargo xtask lists` fails if one is missing or reordered. To check one
crate quickly: `cargo clippy --profile ci -p <crate> --all-targets -- -D warnings`.
The workspace lints do **not** cover the six clippy lints — copy the block from
`crates/io/src/lib.rs`; see [add-a-crate.md](recipes/add-a-crate.md) step 3.

**Tests may panic.** `clippy.toml` sets `allow-unwrap-in-tests`,
`allow-expect-in-tests`, `allow-panic-in-tests` and
`allow-indexing-slicing-in-tests` to `true`, so `#[cfg(test)]` code uses
`unwrap()` and indexing freely. Keep the deny attributes out of `cfg(test)` code
and inside `mod tests`.

### 2. Errors are `Result`, never a default

**Why:** a clamped or defaulted value is a surprise ten minutes later; an error
is a message someone can act on.

**Check:** your `run` returns `Result<Outcome, EngineError>`. If you find
yourself writing `unwrap_or(Default::default())` on a parameter, that is a
missing error.

### 3. Never index a parameter

**Why:** command parameters arrive as raw JSON from a control channel, a script,
or a design file. Indexing panics on a missing or out-of-range key, and the
caller gets an `internal` error instead of being told which key was wrong.

**Check:** read every parameter through `craftkit_core::Params` — `p.text`,
`p.bool`, `p.int`, `p.num`, `p.num_in`, `p.raw` — or through
`crates/engine/src/params.rs` (`node_by_id_or_name`, `value_for`, `kind_of`).
This is rule 4 in `crates/engine/src/lib.rs`, and it is not negotiable.

### 4. Every parameter you read is declared in `params`

**Why:** `Session::execute` runs `p.deny_unknown(&spec.param_names())` before
calling your `run`. A key you read but did not declare is **not ignored — it is
a refusal**, and it takes the whole command down with it.

**Check:** call your command with its own parameters in a test.
`every_parameter_is_documented_and_uniquely_named` does not catch this.

### 5. Every user-visible behaviour is a command

**Why:** the agent-facing surface and the human-facing surface must be the same
code, or one of them will be the one that is wrong.

**Check:** if a button does something, `engine.execute` with that command id
does the same thing. Programmatic calls never open a dialog: interactive work is
a different entry point, the `ui.*` methods, which the window answers and a
headless process refuses.

### 6. A format change is one new file

**Why:** two people editing the reader at once means one of them silently loses
their change.

**Check:** your diff to `crates/io/` is one new file in
`crates/io/src/format/pending/`, plus its `FILES` entry and its `apply` arm.
Nothing else. `cargo xtask ci` — see the note below.

### 7. Layering

**Why:** everything below the UI must be testable without a window, and the UI
must stay swappable.

**Check:** the layer table in `xtask`. A new crate gets a row, and the table
must name only crates that exist. `craftkit-core` is `Standalone` — no workspace
dependency at all. `craftkit-ui` is the first layer allowed `egui`; nothing
below it. See [add-a-crate.md](recipes/add-a-crate.md) step 4.

### 8. No commercial-CAD-derived data

**Why:** craftkit is independent. Behaviour is designed from public
documentation, general CAD conventions and user feedback.

**Check:** never drive, script or extract from a commercial CAD product — no
test data, no measurements, no ids, no command lists. Never read, copy or link
GPL/LGPL/AGPL code. Expected values in tests are analytic or come from an
open-source tool run as a separate program; never from a commercial one.

### 9. No third-party iconography

**Why:** it is a licence and provenance problem, and it is unrecoverable once the
diff lands.

**Check:** every icon is drawn in code from original coordinates in
`crates/ui/src/icons.rs`. No font, no downloaded SVG, no PNG traced from a
screenshot. A `Vec<u8>` of someone else's icon is a licence you do not have.
This is the most serious mistake available in this repo.

### 10. Never break `wasm32-unknown-unknown`

**Why:** the web build is a target, not an afterthought, and a dependency that
does not build for a browser takes it down silently.

**Check:** `cargo xtask ci` includes the wasm build. If a new dependency is not
wasm-clean, it does not go in a crate below the apps.

**Know what this does not do.** `std::fs`, `std::env`, `std::net`, `std::thread`
and `std::process` all *compile* for `wasm32-unknown-unknown` — `std` provides
stubs that fail at run time rather than being compiled out. So the gate proves
everything below the apps still **builds** for a browser, not that it still
**works** in one. Measured with `scripts/mutation-test-gates.sh`; the boundary is
written down in `xtask/src/wasm.rs`.

### 11. A user's bad parameter is a refusal, never an internal error

**Why:** the tree splits its failures in two — a **refusal** is a message someone
can act on, and an **internal** error is the kind the tree documents as "worth
a crash report and a bug". An agent branches on the kind. So a command that is
handed an empty name must say so, not mutate the document and let the
validator catch it afterwards: measured, `node.add { name: "" }` came back as
*"failed unexpectedly (the command left the document in a state the file format
refuses)"*. §92.

**Check:** your command refuses a bad parameter **before** it changes anything,
and the refusal is `EngineError::Param` — not `Internal`, and not "the document
does not validate afterwards". The engine's two backstops (§44's "changed
nothing", §50's "invalid document") are for **tree** bugs; a user who typed
something wrong must never reach them.

### 12. Say whether your command changed anything

**Why:** the engine compares the document with the snapshot it took before your
command. It cannot tell "you forgot to write" from "you wrote the value the
field already held" — and the second is ordinary: setting a property to the
value it already has. Measured, eight of the nine commands that could be asked
to do what they had already done were refused as *"failed unexpectedly"*. §93.

**Check:** take the answer from the model's write — `Node::set_muted`,
`Property::set_value`, `Document::set_range` and their siblings return whether
they changed anything — and pass it on with `Outcome::unchanged_if(!changed)`.
A command that always changes something needs none of this: the default is
"it changed something", which is the direction that keeps §44's bug check
alive.

### 13. `mutates: false` means you do not change the document

**Why:** the flag is published to agents in the command schema, and an agent
that reads it as *"safe to use as a probe"* has to be right. A command that
says it does not change the document and changes it is not a small mistake:
the write is not validated, is not undoable, and does not mark the project
dirty — so the design changes, the app does not know it has, and closing the
file loses the change with nothing having said the file was unsaved. Measured,
before the engine refused it: the command succeeded, the write landed, the
project was not dirty, there was no undo step, and the document was never
validated. §96.

**Check:** `mutates: false` and the document is left exactly as it was. The
engine now refuses and rolls back a command that breaks this, so you do not
have to notice it yourself — but a command that has to be refused to be
correct is a command that should not exist.

---

## How to verify a change

```bash
# everything, in the order the gates run
cargo xtask ci

# one crate, fast
cargo clippy --profile ci -p craftkit-engine -- -D warnings
cargo test -p craftkit-engine

# the whole suite, when you touched something shared
cargo test
```

`cargo xtask ci` is all of that in one command, plus the layering, asset,
shared-list, browser and wasm checks. The pieces are still worth knowing, because
when a gate fails you want the narrow one:

```bash
cargo fmt --all --check
cargo clippy --workspace --profile ci --all-targets -- -D warnings
cargo test --workspace
```

`profile.ci` exists so the suite is fast without going to full release: it keeps
`opt-level = 1` and overflow checks, and turns debug assertions off (they are
slow, and in a graphics context sometimes wrong).

**A change is not done until the tests pass and you have run the app.**

---

## How to see the result of a UI change

Never describe a UI change. Look at it.

```bash
# run the app with the sample design and a control channel
cargo run --release -p craftkit -- --sample --control PORT
# the token is in ~/.local/share/craftkit/control/PORT.token,
# or set CRAFTKIT_CONTROL_TOKEN yourself

# drive it with JSON lines on 127.0.0.1:PORT
{"id":1,"token":"…","method":"engine.execute","params":{"command":"node.add","params":{"name":"Gate","kind":"filter"}}}
{"id":2,"method":"ui.render","params":{"width":1180,"height":720}}
{"id":3,"method":"ui.click","params":{"name":"nodes.row.1.del"}}
```

`ui.render` answers with `png_base64`, `width`, `height`, the list of widget
names and a `rects` map from each name to `[x, y, width, height]` in the pixels
of that picture. Decode the base64 and look at the PNG; the rectangles say
where in it each widget is. §104.

**A click takes a name or a point.** `ui.click` with `{"name": "…"}` finds that
widget and clicks the middle of it; with `{"x": …, "y": …}` it clicks that
point. Giving both is refused rather than resolved — one of the two things you
said would have to be ignored, and which one is a guess. §104.

**`session.state` is how you read the document's parameters.** It answers with
the document's name, its **frame rate**, its **range** (in raw ticks — 254 016
000 000 ticks to the second), the playhead, whether there is unsaved work, the
nodes, the **undo depth** (and how much memory the undo stack holds) and the
plan. The frame rate used to be reachable only by *setting* it, which rewrote
the design; the window says "24 fps · 5.00s" in its header and nothing an
agent could ask said as much. §108.

**The undo stack is in the state as well.** `can_undo` and `can_redo` say
whether there is anything to undo; `undo_depth` says how many steps there
are, and `undo_bytes` says how much memory they hold. The state used to
carry only the two booleans — the depth was in `session.stats`, and the
memory was in nothing at all — while this file, the skill and the MCP's
`craftkit_status` description all said that the state carried the depth.
§111.

**`session.state` also carries the log**: the last 256 runs, each with its
id, whether it worked and what it said. "What has this session done?" used to
be answerable only as a *count* — `session.stats` said "3 commands, 1
failure" — and the commands themselves were in the tree the whole time. §109.

**`session.state` also carries the design's note** — the paragraph in the
file's `meta.comment`, whole. It used to be in the file and in the model and
nowhere else, so the only way to read a design's note was to open the file.
`doc.note` is how you write one (`{"text": "…"}`), an empty text clears it,
and a note with no `text` at all is refused rather than silently emptying the
note. The note is bounded at 4 096 bytes. §110.

**The action is in the method name, not in `params`.** `ui.render`,
`ui.screenshot` and `ui.click` each ask for exactly that; the bare `ui` takes
an `action` of `widgets`, `render`, `screenshot` or `click`. Naming both at
once — the method `ui.render` and an action of `click` — is refused rather than
resolved, because one of the two things you said would have to be ignored, and
which one is a guess. (Written here in words rather than as a JSON object on a
line of its own: `cargo xtask lists` reads one request-shaped object per line as
something to send, and this is the opposite of something to send.)

### What `ui.render` is, precisely

It draws the **current session state into a fresh egui context** — no window, no
GPU, no framebuffer read. Three things follow from that, and they are worth
knowing before you rely on it:

- **It works with no display at all.** `craftkit` started without a window prints
  that it could not open one and *keeps serving the control channel*, so an
  agent on a build machine gets the same capability as one at a desk.
- **It is not the window's pixels.** `ui.screenshot` is the same picture, and
  says so in its own `message`. The window's framebuffer cannot be read from a
  socket thread, and a tool that quietly handed you a different image under that
  name would be worse than one that tells you.
- **A click reports, it does not act.** `ui.click` replays a pointer event into
  a fresh context and answers with the name of the topmost widget at that point.
  Activating something is `engine.execute`, which is undoable and shows up in
  the log. Two ways to do everything is two ways to get it wrong.

`{"action":"widgets"}` returns the names **and where they are**: the names, a
`rects` map from each name to `[x, y, width, height]` in the pixels of the frame
the reply's `width` and `height` describe, and the size itself. That is the
cheapest way to find out whether the control you expect is on screen at all —
and, before §104, it was the only way to learn a name without also learning
where it was, which left a click with nothing to click.

**The screen has a state, and the state is the window's.** The shell remembers
which node is selected, and the properties panel draws that node's editors —
with nothing selected it draws *"Select a node."* and there is no editor on
the screen to name. The window and the control channel share that memory, so
the channel reports the screen the window is showing. Before §106 the channel
drew a shell of its own, always the empty one, so the properties panel was
never in the answer and no property editor could be looked up by name.

To ask about a screen with a node selected — without changing what the window
shows — send **`selected`**, the node's id:

```json
{"id":4,"method":"ui","params":{"action":"widgets","selected":1}}
```

The reply says which node it drew (`selected`), and a node that is not in the
document is refused rather than answered with an empty panel. Selecting a node
is a *screen*, not a command: it changes nothing in the document, so it is
not in the undo stack and there is no command for it. §106.

**Some of the widgets on the screen cannot be clicked.** A greyed-out Undo
button is on the screen, and pressing it does nothing. The reply carries a
**`disabled`** list of the names that are there but dead, so you do not have to
find out by trying. A click on one of them is **refused**, by name and by
point alike, and says so — before §107 a click by point answered *"that button
is at that point"* while a click by name said *"it is on screen but disabled"*,
and one question had two answers. §107.

Headless, through the CLI:

```bash
craftkit-cli run script.json --out part.step
craftkit-cli eval design.craftkit
craftkit-cli snapshot script.json --out shot.png
craftkit-cli commands
```

> These exist. `apps/craftkit-cli` is implemented, and the subcommands above are
> its real surface. `snapshot` rasterises in software — it walks egui's
> tessellated shapes, samples the font atlas for the text, and writes a PNG — so
> a frame can be produced on a build machine with no display and no GPU. That is
> how the picture a UI change needs gets produced.

The contract `crates/control` must satisfy is real and testable now:
`Method::parse`, `dispatch`, `token_path`. See
[add-a-control-channel-method.md](recipes/add-a-control-channel-method.md).

---

## How to add evidence for a change

If your change has any observable effect — a rendering, a UI, a model, a kernel
result, a backend behaviour — the change is not finished without **an actual
screenshot or a short recording from a small reproduction**.

- **Before and after** where the bug is visible.
- A small, public-safe example. Check for private designs, paths, account
  details or anything else sensitive before you attach it.
- Annotate what the reader should look at.
- Attach to the issue or the PR. Do not commit generated screenshots to the
  repository.

A flowchart or a Mermaid diagram is not evidence. Use them for architecture
where a picture of the app cannot make the point. A change with no observable
behaviour — a refactor, a rename, a doc — needs no evidence.

---

## Worked example: add a command that sets a node's `enabled` property

### The naming

Ids are **dotted, lower case, ours**, and the prefix says which area owns the
command: `node.*`, `prop.*`, `key.*`, `doc.*`, `playback.*`, `history.*`.
`Area::Debug` has no prefix and its ids name what they do — `diagnostics.list`,
`plan.evaluate`, `session.stats`. Never borrow another product's command id;
the ids are a public interface that scripts and agents depend on.

So: **`node.enable`**, label `"Enable node"`, `Area::Node`.

### Where each piece goes

| Piece | File | What |
| --- | --- | --- |
| the spec | `crates/engine/src/commands.rs`, in the `// ---- node ----` section, alphabetically | the whole `CommandSpec` |
| the registry entry | `crates/engine/src/command.rs`, the `ALL` list | `&NODE_ENABLE,` in `(area, id)` order, and `22` becomes `23` |
| the icon | `crates/ui/src/icons.rs` | the name in `ALL`, a `Painter` method, a `draw` arm |
| the behaviour | `run` in `commands.rs` | `Session::execute` does the rest |
| the test | `crates/engine/src/commands.rs`, `mod tests` | calls the command with its own parameters |
| the docs | nothing | the catalogue is generated from the registry |

### The spec

```rust
pub static NODE_ENABLE: CommandSpec = CommandSpec {
    id: "node.enable",
    label: "Enable node",
    area: Area::Node,
    icon: Icon("power"),
    shortcut: Some("E"),
    doc: "Enable or disable a node. A disabled node passes its level through untouched.",
    params: &[
        ParamDoc::either("node", Doc::NodeRef, "The node's id.", "node_name"),
        ParamDoc::opt("node_name", Doc::Text, "…or its name."),
        ParamDoc::opt("enabled", Doc::Bool, "True to enable, false to disable. Defaults to the opposite."),
    ],
    mutates: true,
    undoable: true,
    enabled: has_nodes,
    run: |s, p| {
        let id = node_by_id_or_name(s.document(), p)?;
        let node = node_mut(s, id, NODE_ENABLE.id)?;
        let current = find_mut(&mut node.props, "enabled")
            .and_then(|prop| prop.value.as_bool())
            .unwrap_or(true);
        let enabled = p.bool("enabled")?.unwrap_or(!current);
        let prop = find_mut(&mut node.props, "enabled").ok_or_else(|| EngineError::Param {
            id: NODE_ENABLE.id.to_string(),
            source: craftkit_core::ParamError::new("node", "this node has no `enabled` property"),
        })?;
        prop.value = craftkit_core::Value::Bool(enabled);
        Ok(Outcome::with(
            if enabled { "enabled" } else { "disabled" },
            serde_json::json!({ "id": id.get(), "enabled": enabled }),
        ))
    },
};
```

`NODE_MUTE` (`crates/engine/src/commands.rs:190`) is the closest existing
example and it is built from the same parts. `has_nodes`, `node_mut`,
`find_mut`, `node_by_id_or_name`, `Outcome::with` and `EngineError::Param` are
all already in scope in that file.

### Two things this example is teaching

- **`Icon("power")` does not exist yet.** You must add it to
  `crates/ui/src/icons.rs` in the same change, or
  `every_command_has_an_icon_this_set_can_draw` fails. That is the whole of
  [add-an-icon.md](recipes/add-an-icon.md).
- **`enabled` is declared, so the command works.** Had the spec declared `on`
  while the body read `enabled`, `deny_unknown` would refuse every request with
  `bad-parameter`, including correct ones.

### Verify

```bash
cargo test -p craftkit-engine
cargo test -p craftkit-ui
cargo clippy --profile ci -p craftkit-engine -- -D warnings
```

Then run the app, click the button, take a screenshot, and look at it.

---

## When to stop and ask

Stop and ask the owner — do not decide alone — when:

- **A new user-visible behaviour is ambiguous about what it should do to an
  existing design.** Reinterpreting what an existing command means changes every
  document someone has already made.
- **A format change needs older files *rejected*, not rewritten.** Pending steps
  rewrite; refusing is a different decision with different consequences for
  people's work.
- **A new crate, or a new layer number.** Layering is a whole-workspace
  decision.
- **A dependency is needed that is not already in `[workspace.dependencies]`.**
  Every dependency is a supply-chain decision; `deny.toml` enforces a licence
  allow-list and bans wildcard versions, and a new crate outside that list is a
  decision, not a formality.
- **Something needs to read GPL/LGPL/AGPL code or derive data from a commercial
  CAD product.** The answer is no, and there is no narrow version of the
  question that has a different answer.
- **An icon or other asset cannot obviously be drawn from original geometry.**
  Ask rather than ship and apologise.
- **The "never crash" rule appears to need an exception.** An `unsafe` block, a
  `forbid` downgraded to `deny`, a catch-all that swallows a panic. There is
  currently no exception. If you think one is needed, that is a conversation
  about the design.
- **Two changes want the same file and neither is obviously right.** In
  particular `crates/engine/src/command.rs`'s `ALL` array and
  `crates/io/src/format.rs`: resolve by re-reading both intents, never by
  discarding one.

Everything else is yours to decide. A command's parameter names, an icon's
shape, a test's wording, a helper's location — decide, write it down in a
comment, and move on.
