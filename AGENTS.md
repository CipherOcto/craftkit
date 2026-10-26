# AGENTS.md

Instructions for an agent working in this repository. Read this file first; it
is the short version of everything in `docs/`.

craftkit is a **scaffold**: a working, tested workspace meant to be copied and
re-purposed into an application or driven directly by an agent. It is not a
product. Everything here exists so that the next change — whether a person's or
yours — is cheap and safe.

---

## Start every session here

1. `cargo xtask doctor` — can this machine build the project at all?
2. `cargo xtask ci` — is the tree green?
3. `docs/architecture.md` — how the layers fit together, and why.
4. `docs/agent-playbook.md` — how to drive craftkit from a tool, and how to read
   the results.
5. `docs/recipes/` — the task you have is almost certainly one of these.
6. `KNOWN-GAPS.md` — what this project does not do, and what its gates cannot
   see. Read it before concluding that something is missing because nobody
   mentioned it.

Two skills cover most work and load themselves: `skills/craftkit-editing` when
you are changing something, `skills/craftkit-checking` when something is broken.

---

## Never crash

This outranks every other rule here. craftkit holds a user's plan; a crash
loses it, and the user's trust in everything else.

- **No panics in non-test code.** No `unwrap()`, `expect()`, `panic!`,
  `unreachable!`, `todo!`, `unimplemented!`, no `unsafe` (`forbid(unsafe_code)`).
  Every production crate root carries:

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

  It is multi-line because `rustfmt.toml` sets `max_width = 110` and the single
  line is 112 characters. `cargo xtask lists` normalises whitespace, case and the
  `clippy::` prefix, so either form passes — but the lints must be the same lints
  in the same order, because a deny list that varies between crates is a deny
  list nobody can reason about.

- **`clippy.toml` re-enables the unwrap lints in tests only.** A test may panic;
  nothing else may.

- **Errors are `Result<T, E>`** through the crate's error type and `?`. An
  unfinished feature returns an error — `"not supported yet: …"` — it never
  panics and never silently does nothing.

- **Input-derived numbers are hostile.** Command parameters, `.craftkit` files
  and control-channel input all arrive from outside. Read them through
  `craftkit_core::Params`' typed helpers; never index. The helpers are typed
  and a typed value cannot be indexed; the one door out is
  `Params::raw_json`, and it has two homes — `crates/core/src/params.rs` and
  `crates/engine/src/params.rs` — and `cargo xtask lists` refuses the tree
  when a third file opens it. Check finiteness, bound sizes, and cap loop
  counts. `crates/core/src/params.rs` is the list.

- **Two trust boundaries, in order:**
  1. `craftkit_kernel::guard` — turns a panic inside code craftkit does not own
     into `KernelError::Internal`.
  2. `Session::execute` — clones the document, runs the command under
     `catch_unwind`, and on a panic **restores the pre-command document** and
     reports `EngineError::Internal`. A command that panics is a command that
     did not happen.

- **Prove it.** Every crash fix lands with a regression test. The engine has a
  test-only `test.panic` command that exercises the real restore path; use it.

---

## Everything is a command

User-visible behaviour = a `CommandSpec` in `crates/engine/src/commands.rs`.
That one registry is what the toolbar, the dialogs, scripts, the CLI, the
control channel and the MCP server all reach. There is no second path.

```text
id       dotted, lower case, no underscores: <area>.<thing>[.<variant>]
         e.g. solid.extrude, sketch.rectangle.two_point, doc.range
label    what the button says
area     toolbar tab and panel
icon     from crates/ui/src/icons.rs — drawn in code, never a file
shortcut optional
params   every parameter, documented, with its default
enabled  a predicate over the session
run      the JSON form: (session, params) -> Result<Outcome, EngineError>
```

Adding behaviour anywhere else — a special case in the UI, a bespoke MCP tool, a
flag on the CLI — is the mistake this rule exists to prevent. Add a command.

The rule is structural, not a promise. `Session`'s public methods are the
registry (`execute`), the things that read a design, and the two that let the
host open a file; a public method that changed a design would be a second
path, and `cargo xtask lists` refuses the tree when one appears. The front
ends are measured, and they honour it: the desktop app, the web app, the CLI,
the control channel and the MCP server all reach the registry and nothing else
that changes a design. §135.

**Programmatic calls never open dialogs.** `engine.execute` runs the JSON form.
Only a front end's own entry point may ask a question — in this tree that is
`craftkit_ui::shell::Shell::start`, which collects the parameters a command
needs and then runs it. §118.

---

## Non-negotiables

- **Independent.** Behaviour is designed from public specifications, general
  conventions and user feedback. No commercial CAD product's code, icons,
  artwork, help text, tooltips, templates, test data, measurements or generated
  files. Nothing is copied from another project's source, and **no commercial
  product is run** — not launched, scripted, automated, screenshotted or
  measured, and not compared against. The rule and the table of products are in
  [`docs/clean-room.md`](docs/clean-room.md); `cargo xtask cleanroom` checks
  the tree against it, and fails with the file, the line and the name.
- **Assets are absolutely essential.** craftkit contains **no iconography, images,
  fonts or artwork of any kind.** Every UI icon is drawn in code from original
  coordinates in `crates/ui/src/icons.rs`. `cargo xtask assets` fails the build
  if anything embeds a non-source file, and if `ATTRIBUTION.md` is missing,
  incomplete, or lists a file that is not in the tree.
- **Rust only, and it is now checked.** Millimetres and radians internally; Z is
  up. `cargo xtask assets` rejects any file in another language that is not in
  `FOREIGN_SOURCE_EXCEPTIONS` (`xtask/src/assets.rs`). That list has five
  entries, each with its reason, and it is not only the two JavaScript files the
  browser insists on — it also holds `scripts/mutation-test-gates.sh`,
  `scripts/fold-format-pending.sh` and `.github/workflows/ci.yml`. An earlier
  version of this file claimed the JavaScript was the *only* exception, which was
  untrue when it was written, and nothing noticed until a `.py` helper inside
  `crates/ui` left `cargo xtask ci` green.
- **Layering is enforced.** L0 `core` → L1 `model` → L2 `kernel` → L3 `eval`,
  `io` → L4 `engine` → L5 `control`, `mcp` → L6 `ui` → the apps. A crate may
  depend only on **strictly** lower layers, and nothing below L6 may name a UI
  toolkit. `cargo xtask layers` checks it against every `Cargo.toml`.
- **The UI is thin.** Panels read engine state and act through
  `session.execute`. A panel never writes to the document, never calls the
  evaluator, never caches a value the evaluator computes. All three are
  checked: `cargo xtask lists` refuses a shell that names the evaluator, one
  that puts a document into the session, and one whose state holds a value the
  evaluator computes. `crates/ui` depends on `egui` and on no other UI
  toolkit — no windowing, no renderer — which is what lets the same panels run
  in a window, headlessly, and in a browser.
- **One set of camera controls.** This tree has no canvas and no camera: the
  window draws a node graph, a timeline and a properties panel. If a canvas is
  added, it must have **one** set of camera controls — one module that owns
  them, with its own caps — and every canvas must go through that module
  rather than hand-rolling its own input.

---

## The machine is shared

This is a **shared build machine**. Do not take all of its CPU, and do not
run a long or repeated suite without saying so first.

- **Check the load before a big build** (`cut -d' ' -f1 /proc/loadavg`) and
  build with `nice -n 15`.
- **A test or suite that takes all the CPU for more than five minutes needs
  the owner's explicit approval first.** Ask, do not assume.
- **Do not repeat a long suite to look for a rare failure without asking.**
  One run of the mutation suite is twenty-odd minutes; twenty of them is not
  a measurement, it is a way of occupying a machine that other people are
  using.
- **Prefer a bounded measurement to a brute-force loop.** A hundred cheap
  runs of one test binary beat a hundred runs of the whole `ci`, and a
  measurement of the *number* beats a repetition of the *suite*.

## Before you commit

This **is** a git repository: branch `main`, a local history, and the remote
`git@github.com:CipherOcto/craftkit.git` (the owner's decision, 2026-10-25).
`git push` goes there — and only there. Before every commit:

```text
cargo xtask ci          # fmt, anchors, assets, cleanroom, lists, layers, web, clippy, tests, fuzz, wasm
```

Commit after every feature arc that builds, one topic per commit (see
`CONTRIBUTING.md`). Never commit `target*/`; `.gitignore` already excludes it.

---

## Lists many changes add to

These collide constantly. Keep them mechanical, and let the gate arbitrate:

- the **deny attribute** in every crate root — one set of lints, identical
  everywhere (`cargo xtask lists`);
- the **layering table** in `xtask/src/layers.rs` *and* the matching table in
  `docs/architecture.md` — they are compared, so change them together
  (`cargo xtask lists`);
- the **workspace `members` list** in `Cargo.toml`, sorted;
- **`mod` groups**, sorted;
- a **file-format change**, which is one new file at
  `crates/io/src/format/pending/<name>.rs`. Never edit `FORMAT`, `JSON_STEPS`,
  the history in `format.rs`, or a `v<N>/` folder.

## Design file format

`.craftkit` files carry a version tag (`craftkit/1`). To change what is saved,
add **one** file under `crates/io/src/format/pending/`, with a `//!` note
saying what it changes and why, and a `pub fn step` if older designs must be
rewritten. The build works out the number. A release folds the pending changes
into `v<N>/` with `scripts/fold-format-pending.sh` — that is a release step, not
a change step.

---

## Driving craftkit

Every entry point reaches the same 23 commands.

```bash
# list them, with every parameter
cargo run -q -p craftkit-cli --bin craftkit-cli -- commands

# headless: run a script, evaluate a design, render a frame
cargo run -q -p craftkit-cli --bin craftkit-cli -- run script.json --out plan.step
cargo run -q -p craftkit-cli --bin craftkit-cli -- eval plan.craftkit
cargo run -q -p craftkit-cli --bin craftkit-cli -- snapshot script.json --out shot.png

# a window, with a control channel for you to drive it
cargo run --release -p craftkit -- --sample --control 4711
```

The control channel is token-authenticated JSON lines over loopback; the token
is in `~/.local/share/craftkit/control/<PORT>.token` unless `CRAFTKIT_CONTROL_TOKEN`
says otherwise. Its methods are `engine.execute`, `engine.describe`,
`session.state`, `diagnostics.list`, `diagnostics.crash_reports`, `ping`,
`shutdown` and `ui` (`ui.click`, `ui.screenshot`, `ui.render`). `docs/agent-playbook.md` has the wire format, and `crates/control/src/protocol.rs`
is where the methods are defined.

`craftkit-cli mcp` speaks MCP over stdio, with one tool per command named
`craftkit_<id>`. That is how an agent should drive this project: not by editing
files, but by running commands.

---

## Where things are

```text
crates/core     L0  ticks, values, ids, hostile parameters
crates/model    L1  the document: nodes, properties, keyframes
crates/kernel   L2  the numeric kernel, behind the panic guard
crates/eval     L3  document -> plan
crates/io       L3  the versioned .craftkit format
crates/engine   L4  the command registry and the Session
crates/control  L5  the loopback JSON-lines channel
crates/mcp      L5  the MCP server
crates/ui       L6  the egui shell, icons, automation names
apps/craftkit       the desktop window
apps/craftkit-cli   the headless CLI, renderer and MCP host
apps/craftkit-web   the wasm build (index.html + sw.js + Rust)
xtask               the gates
```

## Reporting

Report what actually happened: what passed, what failed, what was skipped and
why, and what you did not check. A claim of success you have not verified is
worse than an honest gap, because it stops the next person looking.