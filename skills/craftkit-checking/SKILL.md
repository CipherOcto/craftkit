---
name: craftkit-checking
description: Use when something in craftkit is red or unproven — "the ci is red", a failing test or clippy error, a layers/assets/lists/wasm failure, an agent that must see a result as a screenshot, or a crash report to diagnose and reproduce.
---

# Checking craftkit

Something is broken, or you have to prove that something works. This is the order to
find out, and how to read what comes back.

The tree at `/mnt/data/mmacedoeu/work/tools/craftkit` is **not a git repository**
(`git rev-parse` fails). There is nothing to check out, bisect or push.

> **Invocation.** Every command below is `cargo xtask <cmd>`. That works because
> `.cargo/config.toml` defines `[alias] xtask = "run --quiet -p xtask --"`, so
> the build tool is an ordinary workspace member with its own tests and its own
> place in the layering table. If it ever fails with
> `error: no such command: xtask`, that alias file is missing — restore it rather
> than working around it everywhere.

---

## 1. Start with `doctor`

```
cargo xtask doctor
```

It reports what this **machine** can do, in three classes:

| Class | Meaning | Effect |
|---|---|---|
| `blocking` | the gates cannot be trusted here | exits non-zero; fix before reading anything else |
| `degraded` | something is missing that a gate silently needs | reported only — **not** a gate failure |
| `info` | present and working | nothing to do |

Observed on this machine right now — degraded, and nothing blocking:

```
── doctor
   [degraded] wasm-bindgen       the CLI is 0.2.89 but the wasm-bindgen crate is 0.2.129;
                           wasm-bindgen refuses a bundle whose schema versions differ
                           → cargo install wasm-bindgen-cli --version <the crate version> --force
   [info] node                   installed; `cargo xtask web` parses the service worker
   nothing blocking
```

A `degraded` finding is **not** a red gate. `wasm-bindgen` is the standing example:
doctor complains, `cargo xtask ci` still passes, and the damage only appears when
you try to produce a real browser bundle. Do not "fix" a degraded finding by
editing the tree.

A `blocking` finding is what a stale toolchain looks like — `rustc` older than
`rust-version` in `Cargo.toml`, which is **1.93**. That is a fact about the
machine, not a bug in the code: do not raise `rust-version` to silence it. And if
you ever do change the minimum, change `clippy.toml`'s `msrv` in the same commit,
or clippy will keep gating lint suggestions to a toolchain the project no longer
claims to support.

---

## 2. The gates

`ci` runs the gates in the table below, in that order, and **stops at the
first failure** (the `PHASES` constant in `xtask/src/ci.rs` — cite the symbol,
not a line number, the file moves). The table is the only copy of that list
in this file, and `cargo xtask lists` holds it against the constant — the
order as well as the names. §189.

Run one on its own while you work; run `ci` when you are done.

| Gate | What it protects | What it typically catches |
|---|---|---|
| `fmt` | formatting is not a reviewer's problem | an unformatted file after an edit |
| `anchors` | every anchor in `scripts/mutation-test-gates.sh` is in the tree, each names one place, the list of files a mutation may write to holds only files that are in the tree, every label is a literal the shell cannot change, every text a mutation writes is a literal too, the header names every way to run the script, the tree's documents name the list of files and not the script's array, the script checks its anchors before it starts, and every copy the script makes breaks the script in one place | an anchor that moved (the mutation would change nothing); an anchor that matches five places, so the swap breaks the wrong one; a file on the list that is not in the tree, so a mutation to it could not be put back; the same file on the list twice; a label the shell would rewrite, so the log says something the author did not write; a text the shell would rewrite, so the anchor the gate reads is not the anchor the shell writes; a header that misses a way to run the script; a document that names the array, so a path added to it is lost; a copy that breaks nothing, so the case tests nothing; a copy that breaks the wrong one of two places, so the case tests something else |
| `assets` | every icon is drawn in code; every file asset has a row in `ATTRIBUTION.md`; **nothing in the tree is written in another language** | a checked-in PNG/SVG/font, an `include_bytes!` of an image, an asset with no attribution row, **a `.py`/`.js`/`.sh` that is not one of the five declared exceptions** |
| `cleanroom` | **no product's name is in the tree outside prose** — the rule and the table of products are in `docs/clean-room.md` | a fixture, a test or a file name that carries another product's name; a product name in a string literal |
| `lists` | the lines many changes touch: the deny attribute, the layer table in `docs/architecture.md`, sorted `mod` groups, the crate roots, every bound the tree has named in the prose, and every field the model's fingerprint hashes | a new crate missing the deny attribute, a `TABLE` row with no matching doc row, an unsorted `mod` block, **a bound the tree has that no document names** (§181), **a field added to a type the fingerprint walks** (§80) |
| `layers` | `TABLE` is the layering rules | a dependency upward (`craftkit-core` naming `craftkit-io`), a UI crate below L6 |
| `web` | the two hand-written JavaScript files: the canvas id, the bundle path, the worker's precache shell, the service worker's three safety rules, and `node --check` | a rename that missed `index.html`/`sw.js`, an unregistered worker |
| `clippy` | `-D clippy::all -D warnings`, `--all-targets`, `--profile ci` | a warning you were going to ignore; a lint the house deny exists to catch |
| `tests` | behaviour, including `hostile_params_never_panic` and the properties in `crates/core/tests/properties.rs` and `crates/engine/tests/properties.rs` | a new command that panics on a hostile parameter; **an instant past the tree's range taken silently instead of refused** (§188) |
| `fuzz` | the three readers the tree has a writer for — the design file, the script and the crash report — handed broken files by `cargo xtask fuzz`: a crash or a hang is a bug, a refusal is the reader working | a reader that panics on a file it should have refused; a corpus that is not the tree's own format |
| `wasm` | every library crate and `craftkit-web` still build for `wasm32-unknown-unknown` | a new dependency that is not wasm-safe |

One more gate is **not** a phase of `ci`: `cargo xtask deny` checks the
dependency policy — licences, bans, sources and advisories — against
`deny.toml`. It needs `cargo-deny` and a network, so CI runs it as its own
job; the rule and the table of licences are in
[`docs/dependency-policy.md`](../../docs/dependency-policy.md).

Green output for each (observed):

```
── assets   ok: no embedded image assets, every asset is attributed, and nothing outside Rust crept in
── cleanroom ok: no product's name is in the tree outside prose
── lists    ok: 13 crate roots share one deny line, the layer table matches the docs, lists are sorted, …
── layers   craftkit-core L0 … craftkit-ui L6, apps + xtask exempt
── web      ok: the canvas, the bundle path, the worker's shell and its rules all agree
            ok: apps/craftkit-web/sw.js parses
```

### Reading a failure

The output is the compiler's and the checker's own words; there is no summary layer
to reinterpret. Two shapes to recognise:

```
── clippy
error: unused variable: `n`
   --> crates/core/src/lib.rs:1:9
```

That is clippy or rustc. Fix the code it points at; do not add `#[allow]` to make it
go away, unless the lint is wrong and the comment says why.

```
── lists
error: 2 shared-list problem(s):
   • crates/newthing/src/lib.rs: no deny attribute.
     Add this at the top of the file: …
   • docs/architecture.md disagrees with the code: …
```

That is a checker. Each bullet names the file, what was wanted and what was found.
The gate is the specification — when it and a doc disagree, the code is right.

**A skip is never a pass.** `wasm` prints `skipped` when the target is not installed
(`wasm::run` in `xtask/src/wasm.rs`), and `web` prints `skipped` when `node` is missing
(`web::run` in `xtask/src/web.rs`). Neither is a failure and neither is a pass. If a gate you
care about skipped, install the thing and run it again:

```
rustup target add wasm32-unknown-unknown
```

### A gate that passes has proved nothing on its own

A green gate is only evidence if you know it can go red. Four of this project's
gates were quietly unable to fail — a substring search that a rename defeated, a
check that existed only as a unit test, a table reader that could not see a row
glued onto the end of a line of prose, and `ci` itself, which reported running the
tests and the wasm check while doing neither.

So there is a second command, and it is the one that matters when you add or
change a gate:

```bash
scripts/mutation-test-gates.sh            # break each rule, require the gate to notice
scripts/mutation-test-gates.sh --self-test   # must exit NON-ZERO — it proves the script fails
scripts/mutation-test-gates.sh --check       # is the list of files a mutation may write to usable?
```

`--self-test` injects one synthetic case that is guaranteed to miss. It is
expected to fail; if it ever succeeds, the instrument itself has stopped working
and every result it gives is meaningless.

When you add a gate, add a line that breaks its rule and asserts it rejects the
tree. When you change a gate, the mutation that found the old bug is still in the
script — that is what stops it coming back. The script runs in CI as the
`mutation` job.

**And a rule about the script's own text needs its own door.** The script
cannot be edited while it runs — bash reads a script as it goes — so the list
of files a mutation may write to lives in `scripts/mutation-files.txt`, which
the script and `cargo xtask anchors` both read, and `cargo xtask anchors
--script PATH` checks a **copy** of the script. A copy is the only thing a
case can break. §187.

**And a copy's own anchors are checked before the run starts.** A `broken_copy`
looks its text up in the script and replaces the **first** place it finds, so
the rule for a copy is §184's rule for a `swap`: the text names **one** place,
apart from the call's own argument. A text the script does not hold, a text
whose first place is the call itself, and a text in two places are three ways
to write a copy that breaks nothing — or that breaks the wrong thing. All
three are found at the end of a run instead of at the start of one, and the
run then stops and every result below it is about nothing. §193.

### The one check that needs a display

```bash
DISPLAY=:1 scripts/live-window-check.sh   # 0 passed, 1 failed, 77 no display
scripts/live-window-check.sh --self-test  # the window picker, no display needed
```

CI has no display, so this cannot run there — and it is the check that found the
worst bug in the project: the window kept painting the frame from *before* a
command arrived over the control channel, while the engine had already moved on.
The engine reported five nodes, the screen showed three, the reply said `ok`, and
nothing anywhere complained.

It compares the window's own pixels before and after a command. If you change
anything about how the app repaints, run this. Note the exit code: **77 means it
did not run**, and that is deliberately not 0.

### The one check that cannot see a leak

egui only redraws when something asks it to, so a leak in the drawing path is
**invisible in a running window**: the process grows and the screen looks
perfectly fine. §66 found one that cost 221 MB an hour — the widget names were
`Box::leak`ed, once per widget per frame — and the only thing that saw it was a
probe that drove `craftkit_ui::frame` in a loop and watched the process's
resident pages.

If you touch a per-frame path — the shell's widgets, the rasteriser, anything
that runs once per frame — measure it, and measure it *twice*: a leak is linear
in the number of frames and allocator noise is not, so one run tells you
nothing and two runs tell you everything.

`crates/ui` forbids `unsafe`, so there is no counting global allocator to
write. The kernel's own answer is `/proc/self/statm`'s second field — the
process's resident pages, which is memory the allocator has not handed back —
and a leak is only visible against a baseline, so measure it in a process that
has done nothing else.

### The one check that cannot see a value that came from a file

A `craftkit_core::Value`'s invariants — a number is finite and within
`MAX_MAGNITUDE`, a text is within `MAX_TEXT`, a list is within `MAX_LIST` —
are kept by its **constructors**. A file is deserialized straight into the
model, so no constructor runs: before §73 a hand-written file could carry a
number a hundred million times past the limit, and the evaluator's comment
said "every number the tree stores is" in range.

The model now checks every value it reads (§73), so a file like that is
refused with *"the value of amount is out of range"* rather than believed.
Two things follow for anyone touching this tree:

- a new way for a value to arrive is a new **door**, and a door needs a
  check. The three doors are the constructors, the interpolator (§65) and
  deserialization;
- a **budget measured with an estimate** is only as good as the estimate.
  `Property::estimated_bytes` used to leave the property's own value out, so
  the undo stack's 64 MiB budget did not bind and the process grew to
  830 MB. If you add a thing a document can hold, count it.

### `wasm` and `wasm-bindgen`

`cargo xtask wasm` **builds** for `wasm32-unknown-unknown`
(`wasm::build` → `layers::check_wasm`). It does not bundle. The `wasm-bindgen` CLI is
what turns a `.wasm` into something a browser loads, and the CLI's version must
match the `wasm-bindgen` **crate** version in `Cargo.lock` or bundling fails with a
schema error. On this machine the CLI is 0.2.89 and the crate is 0.2.129 — building
works, bundling does not. Fix it on the machine, never in the tree:

```
cargo install wasm-bindgen-cli --version 0.2.129 --force
```

---

## 3. Driving the app, and seeing a result

Three surfaces, and they are not equal. Prefer the first.

### a. Headless — `craftkit-cli` (this is the one that works everywhere)

```
cargo run -q -p craftkit-cli -- commands
cargo run -q -p craftkit-cli -- commands --json
cargo run -q -p craftkit-cli -- run script.json --sample
cargo run -q -p craftkit-cli -- eval plan.craftkit --at 1.5
cargo run -q -p craftkit-cli -- snapshot script.json --out /tmp/shot.png
```

`run` takes a script of commands and **no `--out`** (see `parse` in
`apps/craftkit-cli/src/args.rs`); `snapshot` takes `--out` and also accepts
`--sample`. `snapshot` runs the script, renders one real frame of the real
shell through `craftkit_ui::raster::render` at a fixed size (see `snapshot` in
`apps/craftkit-cli/src/cli.rs`) and writes a PNG. Text rasterises as solid glyph
quads; everything else draws as the window draws it. **This is how you get
evidence for a UI or model change.**

Script format — the same shape as a crash report's **steps**, which is what
`craftkit-cli crashes replay --script` prints:

```json
{
  "name": "a ramp",
  "commands": [
    { "command": "node.add", "params": { "name": "In", "kind": "source", "amount": 1 } },
    { "command": "node.add", "params": { "name": "Out", "kind": "sink" } }
  ]
}
```

A script is capped at 1 MiB of text (`MAX_SCRIPT_BYTES` in
`apps/craftkit-cli/src/cli.rs`), and that cap is derived from a memory budget:
the parse of a script costs up to 32 times its size, so a 32 MiB script cost a
gigabyte of memory before the cap was derived (§64). A crash report whose
base document is too large to carry in a script says so and points at
`craftkit-cli crashes replay <id>`, which replays it directly.

### b. The window

```
cargo run --release -p craftkit -- --sample --control 4711
```

Flags: `--sample`, `--control PORT`, an optional `[FILE]`, `-h`, `-V`
(`parse_flags` in `apps/craftkit/src/main.rs`). An unreadable file still yields a usable session
rather than refusing to start.

### c. The control channel

```
cargo run --release -p craftkit -- --sample --control 4711
cat ~/.local/share/craftkit/control/4711.token
```

The token comes from `CRAFTKIT_CONTROL_TOKEN` when set, otherwise 32 bytes from the
OS written `0600` (`write_token` in `crates/control/src/token.rs`). `CRAFTKIT_CONTROL_DIR`
relocates the folder — useful for a launch script; it moves the token *file*, and a
directory that was already there keeps the permissions its owner gave it (§55). One JSON
object per line; the **first** request on a connection carries `"token"`; **request** lines
are capped at 1 MiB (`crates/control/src/protocol.rs:21`). That cap is a cap on *text*:
the parse of a line costs up to 32 times as much memory, so the number is derived from a
32 MiB memory budget rather than chosen on its own (`crates/core/src/lib.rs`). Replies are
not capped: a
`ui.render` reply is a base64 PNG of the frame, so read it by its newline rather than by a
fixed buffer. A frame may be 8192 on a side and 16 777 216 pixels in total
(`crates/ui/src/raster.rs`).

```json
{"id":1,"token":"…","method":"engine.describe"}
{"id":2,"method":"engine.execute","params":{"command":"node.add","params":{"name":"In","kind":"source"}}}
{"id":3,"method":"session.state"}
{"id":4,"method":"diagnostics.list"}
{"id":5,"method":"ping"}
```

Methods, exactly (`Method::all()` in `crates/control/src/protocol.rs`):
`engine.execute`, `engine.describe`,
`session.state`, `diagnostics.list`, `diagnostics.crash_reports`, `ping`, `shutdown`,
and `ui` (aliases `ui.click`, `ui.screenshot`, `ui.render`). There is no
`commands.list` — the catalogue comes from `engine.describe`.

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
memory was in nothing at all — while this file, the playbook and the MCP's
`craftkit_status` description all said that the state carried the depth.
§111.

**`session.state` also carries the log** — the last 256 runs, each with its
id, whether it worked and what it said. "What has this session done?" used to
be answerable only as a *count* (`session.stats` said "3 commands, 1
failure"); the commands themselves were in the tree the whole time. §109.

**`session.state` also carries the design's note** — the paragraph in the
file's `meta.comment`, whole. It used to be in the file and in the model and
nowhere else, so the only way to read it was to open the file; `doc.note` is
how you write one, and an empty text clears it. §110.

**`ui.*` works in the desktop app.** `craftkit-control` is layer 5 and
`craftkit-ui` is layer 6, so the channel does not draw: the host installs a
handler with `ServerOptions::with_ui` (`apps/craftkit/src/main.rs`'s `ServerOptions::with_ui`) and a
process without a UI leaves it unset, which is what answers `no-window`. The
handler draws a **fresh** frame into a new egui context via
`craftkit_ui::raster::render`, so it needs no GPU and no window:

```json
{"id":6,"token":"…","method":"ui.render","params":{"width":1180,"height":720}}
{"id":7,"token":"…","method":"ui","params":{"action":"widgets"}}
{"id":8,"token":"…","method":"ui.click","params":{"name":"nodes.row.1.del"}}
```

`ui.render` answers with `png_base64`, `width`, `height`, the list of widget
names and a `rects` map from each name to `[x, y, width, height]` in the pixels
of that picture. `ui.screenshot` is the same picture and says in its own
`message` that it
is a fresh render rather than the window's framebuffer — a socket thread cannot
read that. `ui.click` replays a pointer event into a fresh context and reports
which widget was at the point; it does **not** activate anything, because acting
is `engine.execute`.

**A click takes a name or a point.** `{"name": "nodes.row.1.del"}` finds that
widget and clicks the middle of it — which is how you find out *where* a widget
is, since the reply carries the point. `{"x": …, "y": …}` clicks that point.
Giving both is refused. §104.

**The problems panel publishes a name per problem.** `problems.item.0` is the
first problem the plan found and `problems.item.1` the second, so "what does
the screen say?" has an answer about the problems and not only about the
toolbar, the node rows and the property editors. §199.

**The `widgets` action says where the widgets are.** The reply carries the
names, a `rects` map from each name to `[x, y, width, height]` in the pixels
of the frame its `width` and `height` describe, and the size itself. Before
§104 it carried the names only, so a name could be found but not aimed at.

The action is spelled `{"method": "ui", "params": {"action": "widgets"}}` and
**not** `ui.widgets`: `widgets` is the one action with no method spelling of
its own, on purpose — a `ui.widgets` that the parser does not accept would be
a name that resolves to nothing. §104.

**The screen has a state, and the channel shares the window's.** The shell
remembers which node is selected, and the properties panel draws that node's
editors; with nothing selected it draws "Select a node." and there is no
editor on the screen to name. Before §106 the channel drew a shell of its own,
always the empty one, so `ui.widgets` never listed a property editor and
`ui.click {"name": "node.1.prop.amount"}` was refused with "there is no widget
named … on screen". To ask about a screen with a node selected, send
`"selected": <node id>`; the reply says which node it drew, and a node that is
not in the document is refused. §106.

**Some widgets are on the screen but cannot be clicked.** The reply carries a
`disabled` list of the names that are there but dead — a greyed-out Undo
button, for one. A click on one is refused, by name and by point alike, and
says so. Before §107 a click by point said "that button is at that point"
while a click by name said "it is on screen but disabled": one question, two
answers. §107.

**The method name is the action.** `ui.render`/`ui.screenshot`/`ui.click` each
mean themselves; only the bare `ui` reads `params.action`, and giving both a
method and a disagreeing `action` is refused. This is the shape that matters:
`{"method":"ui.click","params":{"x":412,"y":288}}` once rendered a 1180×720 PNG
and answered `rendered 1180x720`, `ok: true`, having clicked nothing.

Two things to check before reporting a `ui.*` problem: that the process was
launched by `apps/craftkit` (a process without a UI has no handler, and
`craftkit-cli` has no control channel at all), and that a panic in a host handler
now comes back as the error `handler-panicked` rather than dropping the
connection.

### MCP

```
cargo run -q -p craftkit-cli -- mcp
```

JSON-RPC 2.0 over stdio, one message per line. `initialize`, `notifications/initialized`,
`tools/list`, `tools/call`, `resources/list`, `resources/templates/list`, `resources/read`,
`prompts/list`, `prompts/get`, `completion/complete`, `ping`, `shutdown`. Tools are generated from the command
registry: `node.add` → `craftkit_node_add` (`crates/mcp/src/tools.rs:79`), plus three
built-ins, `craftkit_status`, `craftkit_commands` and `craftkit_crashes`. The last
one reads the crash reports this machine has written: `action` is `list`, `show` or
`replay`, and `show` and `replay` need the report's `id`. §113. A refused command
comes back with `isError: true` and a *successful* RPC — the two are different
failures.

A **resource** is the protocol's other half, and it *is* rather than *acts*: the open
design (`craftkit://design`), the command catalogue (`craftkit://commands`) and one
node of the design (`craftkit://node/n12`). The first two hold exactly what
`craftkit_status` and `craftkit_commands` hold, because they are built by the same
calls. A URI that names no resource is refused with `-32002`, not with "no such
method". §176.

A **prompt** is the protocol's third part, and it *says*: a templated message that
tells an agent what to do. Three of them — `add-node`, `set-property`, `key-property`
— and each one names a command (`node.add`, `prop.set`, `key.set`) and is built from
the registry's own words for it, so a prompt cannot describe a command differently
from the tree does. `prompts/get` needs the prompt's required arguments; a missing one
is refused with `-32602`. §178.

The **version** is negotiated in the handshake: the server answers the client's
revision when it speaks it, and one it speaks when it does not. It speaks
`2025-06-18` and `2024-11-05` — and *not* `2025-03-26`, which added JSON-RPC
batching that this server does not read. A client on `2024-11-05` is not offered
the `completions` capability, because that revision has never heard of it. §179.

`completion/complete` is the protocol's fourth method: the suggestions for one
argument, filtered by what has been typed. The reference is
`{"type":"ref/prompt","name":…}` or `{"type":"ref/resource","uri":…}`, and the
argument is `{"name":…,"value":…}`. The values come from the prompt's own
declaration or the template's own, so they follow the tree. §179.

---

## 4. Crashes

A crash report is the design plus the commands that produced it, written by the
desktop app into `craftkit_engine::crash`'s `report_dir()` (in
`crates/engine/src/crash.rs`) — `CRAFTKIT_CRASH_DIR` relocates it.

```
cargo run -q -p craftkit-cli -- crashes list
cargo run -q -p craftkit-cli -- crashes show ID
cargo run -q -p craftkit-cli -- crashes replay ID
```

And for a report that **arrived** from another machine — a `report.json` in your
hand, with no crash folder around it and no id to look it up by:

```
cargo run -q -p craftkit-cli -- crashes show --file report.json
cargo run -q -p craftkit-cli -- crashes replay --file report.json
```

The file door reads the file you were handed, with the tree's one reader and
the tree's one replay, so it cannot answer differently from the folder's door
(§195). The file does not have to be in the crash folder, and it does not have
to be named after its id.

And the tree can **write** one — before this, the only way to get a report onto
disk was to have a *window* crash, so a headless run left nothing behind:

```
cargo run -q -p craftkit-cli -- crashes write --out report.json
```

The report is of the **sample session** — the sample's three commands, the
document they ran on and the document they produced — so it carries a real
replay, and `crashes replay --file report.json` on it says it reproduced the
design. It is written with the tree's own writer, so the file holds the same
bytes a report in the crash folder holds (§200).

`crashes replay` prints what each step did and then **says whether the replay
reproduced the design the report recorded** — or lists the ways in which it
did not. Read that line before believing anything else: a replay that quietly
builds a different document is worse than no replay.

`crashes show ID` prints the report as JSON: `design` (the document at the
crash), `base` (the document the recorded commands start from) and `replay`
(the steps). A script for `snapshot` is `{ "name": …, "commands": [ … ] }`, and
the report's `replay` array is that `commands` array — but a report is **not**
a script: it also carries the design, which is its **target**, and that is what
`crashes replay --file` compares the result with.

A report is **input** — it arrives from another machine — and the reader
treats it as one: `design` and `base` are read as **documents**, so a report
whose design is not a document is refused by the format instead of being
loaded into memory first (§72). A refusal names the file and the reason, and
it costs the text of the file rather than thirty-two times the text. The file
format itself did not change: a report written by an older version still
reads.

**Who writes a report, and when.** The desktop app writes one when it catches
a panic: the panic hook for a panic that escapes, and the render path for a
panic in the drawing code — which the hook cannot reach, because the thread
that draws holds the session's lock (§76). A process that is *killed* writes
nothing: catching a signal needs `unsafe` FFI, and this tree forbids
`unsafe`. So "the application was killed" means "there is no report", and
there is nothing to replay — say so rather than looking for one.

When someone reports a crash, start from `crashes show ID`, reproduce with
`replay`, then fix it **with a regression test**. The engine registers a test-only
`test.panic` command (`test.panic` in `crates/engine/src/command.rs`) that panics on purpose and
is reachable only in test builds, so a test can exercise the real
restore-on-panic path in `Session::execute` rather than a mock of it. Model your
regression test on that.

A panic inside `run` is not a lost document: `execute` catches it, restores the
snapshot and reports `EngineError::Internal`.

---

## 5. Triage

| Symptom | First command | Likely cause |
|---|---|---|
| `error: no such command: xtask` | `cargo xtask ci` | `.cargo/config.toml` is missing its `[alias] xtask` entry |
| `doctor` exits non-zero before any gate | `cargo xtask doctor` | rustc older than `rust-version`; update the toolchain, do not edit `Cargo.toml` |
| `fmt` gate fails | `cargo fmt --all` | an edit left unformatted; the diff it prints is the fix |
| `assets` reports an embedded asset | `cargo xtask assets` | a checked-in image; draw it in `crates/ui/src/icons.rs` instead |
| `cleanroom` names a product | `cargo xtask cleanroom` | a name in a source file, a test or a file name; the rule allows the name in prose only — see `docs/clean-room.md` |
| `assets` reports an unattributed file | — | add a row to `ATTRIBUTION.md` naming file, what, and where it came from |
| `lists` says "no deny attribute" | `cargo xtask lists` | new crate root; copy the block from any other crate |
| `lists` says the deny is "not the house one" | — | right lints, wrong **set or order**; whitespace does not matter |
| `lists` says the layer table disagrees with the docs | — | `TABLE` in `xtask/src/layers.rs` and the table in `docs/architecture.md` must change together |
| `lists` says the fingerprint does not hash a field | — | the model grew a field and `crates/model/src/fingerprint.rs` does not read it; the evaluator's cache would serve a stale state (§80) |
| `layers` reports a dependency upward | `cargo xtask layers` | a lower layer naming a higher one; move the code down or invert it |
| `web` fails after a crate rename | — | `apps/craftkit-web/index.html` and `CANVAS_ID` in `xtask/src/web.rs` are two copies of one string |
| `wasm` prints `skipped` | `rustup target list --installed` | the target is missing; `rustup target add wasm32-unknown-unknown` |
| a browser bundle dies with a schema error | `wasm-bindgen --version` | CLI and crate versions differ; reinstall the CLI at the crate version |
| clippy: `unwrap_used` / `panic` in a `run` body | `cargo clippy -p craftkit-engine --all-targets` | a parameter read that bypassed `Params` |
| a command returns `EngineError::Internal` "changed nothing" | — | `mutates: true` on a command that does not write; or a value that compares equal |
| a test panics on a hostile value | `cargo test -p craftkit-engine hostile_params_never_panic` | an indexed or unwrapped parameter read |
| an undo entry does nothing | — | `undoable: true` without `mutates: true`; the snapshot is only pushed for a mutation |
| the app will not open a design | `cargo run -q -p craftkit-cli -- eval plan.craftkit` | the file format or a pending step; check `diagnostics.list` |
| a design is refused with "the value of X is out of range" | — | a hand-edited file carries a number past `MAX_MAGNITUDE`; a file is deserialized, so it is checked on the way in (§73) |
| everything is red after a half-finished edit | `cargo xtask lists && cargo xtask layers` | cheap gates first; they fail in seconds and name the file |

## 6. Before you call it verified

- [ ] `cargo xtask ci` green end to end, or every skip explained.
- [ ] No gate skipped silently — read the `skipped` lines.
- [ ] A crash fix has a regression test, built on the real restore path.
- [ ] A UI or model change has a PNG from `craftkit-cli snapshot` as evidence.
- [ ] Reported the tree's state honestly: this is not a git repository, so
      "committed" and "pushed" do not apply.