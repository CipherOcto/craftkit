# AGENTS.md

Instructions for an agent working in this repository. Read this file first; it
is the short version of everything in `docs/`.

craftkit is a **scaffold**: a working, tested workspace meant to be copied and
re-purposed into an application or driven directly by an agent. It is not a
product. Everything here exists so that the next change — whether a person's or
yours — is cheap and safe.

This tree is **v2**. Its thesis is one sentence:

> A command is a struct whose fields are its parameters; the compiler writes the
> declaration, the reader and the registration; the registry is a **value**, and
> any crate linked in can put a command in it.

---

## Start every session here

1. `cargo xtask doctor` — can this machine build the project at all?
2. `cargo xtask ci` — is the tree green?
3. `cargo test -p craftkit-gates` — does the tree's **own** gate pass? This is
   the gate that knows about v2 (see §The two gates below).
4. `docs/architecture.md` — how the layers fit together, and why.
5. `docs/agent-playbook.md` — how to drive craftkit from a tool, and how to read
   the results.
6. `docs/recipes/` — the task you have is almost certainly one of these.
7. `KNOWN-GAPS.md` — what this project does not do, and what its gates cannot
   see. Read it before concluding that something is missing because nobody
   mentioned it.

Two skills cover most work and load themselves: `skills/craftkit-editing` when
you are changing something, `skills/craftkit-checking` when something is broken.

---

## The two gates

There are two gate systems in this tree, and they are not the same thing.

- **The tree's own gate** is `crates/craftkit-gates`. It is a crate with a
  function `check(root) -> Vec<Finding>`, and it knows this tree: it checks the
  house deny attribute, the layering, "a plugin is a plugin", the front end is
  thin, "the bounds are named in the prose", "a battery is a table" and "a test
  does not litter". Run it with `cargo test -p craftkit-gates`.
- **The outer gate** is `xtask/`, reached as `cargo xtask <command>`. Its
  commands are `doctor`, `ci`, `layers`, `assets`, `cleanroom`, `lists`, `web`,
  `wasm`, `deny`, `bundle` and `verify`. `cargo xtask ci` is the whole of it:
  fmt, assets, cleanroom, lists, layers, web, clippy, tests, wasm.
  **`cargo xtask ci` checks the packages a change can break** — the ones the
  working tree changed and the ones that depend on them — and it **names the
  packages it left out**. On a build machine (`CI` in the environment) it
  checks the whole tree, and `cargo xtask ci --all` asks for that anywhere.
  `cargo xtask verify --changed` is the same check on its own.

`KNOWN-GAPS.md` says what is true of each today. Read it before you trust
either one to be green.

---

## Never crash

This outranks every other rule here. craftkit holds a user's design; a crash
loses it, and the user's trust in everything else.

- **No panics in non-test code.** No `unwrap()`, `expect()`, `panic!`,
  `unreachable!`, `todo!`, `unimplemented!`, no `unsafe` (`forbid(unsafe_code)`).
  Every crate root carries exactly this deny list, in this order:

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

  It is multi-line because `rustfmt.toml` sets `max_width = 110`. The tree's own
  gate (`crates/craftkit-gates`) compares the lints and ignores the formatting,
  so either form passes — but the lints must be the same lints in the same
  order, because a deny list that varies between crates is a deny list nobody
  can reason about.

- **Ten crates deny unsafe instead of forbidding it, and they are named.** A
  shared library's entry point is found **by name**, and a function with a
  fixed name needs `#[unsafe(no_mangle)]`. `forbid` cannot be relaxed by an
  `allow` — it is a promise that can never be broken — so those crates carry
  `#![deny(unsafe_code)]` and **one** `#[allow(unsafe_code)]` on that one
  place. The list is one list, in `crates/craftkit-alloc/tests/gate.rs`:

  | crate | why |
  |---|---|
  | `craftkit-alloc` | the counting allocator: a global allocator is unsafe by definition |
  | `craftkit-ffi` | the plugin loader: opening a library and calling into it |
  | `craftkit-plugin-shared` | a shared library's entry point, which is found by name |
  | `craftkit-clap` | the CLAP host: opening a plugin and reading the entry it exports |
  | `craftkit-clap-fixture` | a reference CLAP plugin: the entry point is a data symbol, found by name |
  | `craftkit-vst3` | the VST3 host: opening a plugin and calling the factory it exports |
  | `craftkit-vst3-fixture` | a reference VST3 plugin: the entry point is a function, called by name |
  | `craftkit-au` | the AU host: opening a unit and calling the symbols it exports |
  | `craftkit-au-fixture` | a reference AU: the symbols are found by name |
  | `craftkit-wasm` | the browser door: the exports are found by name |

  The exception is the *attribute*, not the capability: each of those crates
  holds **exactly one** `allow(unsafe_code)`, and the only `unsafe impl` in the
  tree is the counting allocator's. An eleventh exception is a line in that
  file and a reason in it.

- **`clippy.toml` re-enables the unwrap lints in tests only.** A test may panic;
  nothing else may.

- **Errors are `Result<T, E>`** through the crate's error type and `?`. An
  unfinished feature returns an error — `"not supported yet: …"` — it never
  panics and never silently does nothing.

- **Input-derived numbers are hostile.** Parameters, `.craftkit` files and
  control-channel input all arrive from outside. Read them through
  `craftkit_sdk::Params`' typed helpers; never index. Check finiteness, bound
  sizes, and cap loop counts. `crates/craftkit-sdk/src/params.rs` is the list.

- **Two trust boundaries, in order:**
  1. `craftkit_sdk::guard` — turns a panic inside code craftkit does not own
     into an error.
  2. `Session::execute` — clones the document, runs the command, and on a
     panic **restores the pre-command document** and reports the failure. A
     command that panics is a command that did not happen.

- **Prove it.** Every crash fix lands with a regression test.
- **A test that needs a file asks the scratch.** `craftkit_scratch::file`
  and `craftkit_scratch::dir` hand out a path that is unique and is taken
  back when the thread that asked for it ends. A test may not name the
  temporary directory itself: the tree's own gate says so, and the file
  keeps the caller's own name.

---

## Everything is a command

User-visible behaviour = a `CommandSpec` in the registry. That one registry is
what the terminal door, the agent door, the browser door, the bridge, the
studio and the window all reach. There is no second path.

```text
id       dotted, lower case, no underscores: <area>.<thing>[.<variant>]
label    what the button says
area     the group the command is listed under
icon     a name; the drawing is the front end's business, and there is no
         icon file anywhere in the tree
params   every parameter, documented, with its default
enabled  a predicate over the session
run      the JSON form: (session, params) -> Result<Outcome, EngineError>
```

A command is declared with `#[derive(Command)]` and `#[command(...)]` in
`crates/craftkit-derive`, and its parameters are the **fields of a struct**:
the compiler writes the declaration, the reader and the registration. A
**battery** declares its commands as a **table** instead — `commands! { … }`
in `crates/craftkit-derive`, one row per command, and the compiler writes the
same struct, reader, registration and spec for every row. Both spellings
make the same registry; the table is what the tree's nine batteries use,
and the derive is what a single command uses. A plugin names its own items
with `craftkit_sdk::plugin_anchor!(...)`, so that the linker keeps them on
every target.

Adding behaviour anywhere else — a special case in a front end, a bespoke MCP
tool, a flag on the CLI — is the mistake this rule exists to prevent. Add a
command.

**Programmatic calls never open dialogs.** `Session::execute` runs the JSON
form. Only a front end's own entry point may ask a question.

---

## The rules of the modelling

These are the house rules of the model, and they are the point of the tree:

- **A node may have no place.** No location and no geometry is not an error;
  it is a node that is not on a canvas.
- **The document carries the payload; the tree is containment and relations.**
- **Derived values are derived — never stored.** If it can be computed from
  what is stored, it is not stored.
- **Refuse, do not clamp.** An out-of-range parameter is an error, not a
  silently clamped value.
- **"Nothing" is an option, not a zero.** An absent value and a value of zero
  are different things.
- **Unknown names are refused.** A name that is not known is an error.
- **Truthfulness beats tidiness.** A false note in a document is corrected in
  place, not quietly left.

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
- **Assets are absolutely essential.** craftkit contains **no iconography,
  images, fonts or artwork of any kind.** Every icon is drawn in code.
  `cargo xtask assets` fails the build if anything embeds a non-source file, or
  if `ATTRIBUTION.md` is missing, incomplete, or lists a file that is not in the
  tree.
- **Rust only.** The only non-Rust files in the tree are
  `apps/craftkit-wasm/index.html`, `apps/craftkit-wasm/browser-check.mjs` (the
  browser insists on JavaScript) and the CI workflow.
- **Layering is enforced.** L0 → L1 → L2 → L3 → L4 → L5; a crate may depend
  only on **strictly lower** layers, and the applications are exempt. The table
  is `crates/craftkit-gates/src/lib.rs` (`pub const LAYERS`), and the gate
  reads it. **A plugin may not depend on an app or on another plugin.**
- **The UI is thin.** A front end reads the kit's state and acts through the
  registry. A front end never writes to the document, never calls the
  evaluator, never keeps a value the evaluator computes. The gate checks it:
  `crates/craftkit-gates` refuses a front end that names `document_mut`,
  `evaluators`, `Plan`, `PlanItem`, `undo` or `redo`.
- **One set of camera controls.** The camera is `craftkit_sdk::camera::Camera`
  — one type, with its own caps (`MIN_ZOOM`, `MAX_ZOOM`) — and the rasteriser
  (`crates/craftkit-raster`) draws through it. A new canvas goes through that
  camera rather than hand-rolling its own input.

---

## The machine is shared

This is a **shared build machine**. Do not take all of its CPU, and do not
run a long or repeated suite without saying so first.

**Every rule in this section is about processes that run *here*, on this
machine.** It is the owner's machine and other people's too. **Remote work is
not covered by it**: CI runs on GitHub's runners, and so does any other remote
processing — a job there takes nothing from this machine at all, so it needs
no approval. **And that is where a heavy job belongs**: if something is
expensive, put it in CI rather than running it here.

- **Check the load before a big build** (`cut -d' ' -f1 /proc/loadavg`) and
  build with `nice -n 15`.
- **A test or suite that takes all the CPU for more than five minutes needs
  the owner's explicit approval first.** Ask, do not assume.
- **Do not repeat a long suite to look for a rare failure without asking.**
- **Prefer a bounded measurement to a brute-force loop.** A hundred cheap runs
  of one test binary beat a hundred runs of the whole `ci`.
- **A measurement you wrote yourself is a test, and the rule above covers it.**
  A scratch program is not a free exception: before running one, say how long
  it runs and how much memory it holds, and keep both **bounded** — a fixed
  iteration count, a fixed buffer, no read that can grow without limit.

---

## Before you commit

This **is** a git repository: branch `main` — the only working branch — a local
history, and the remote `git@github.com:CipherOcto/craftkit.git`. `git push`
goes there — and only there. Before every commit:

```text
cargo xtask ci          # fmt, assets, cleanroom, lists, layers, web, clippy, tests, wasm
                        # — over the packages the change can break; it says which it left out
cargo test -p craftkit-gates
```

Commit after every feature arc that builds, one topic per commit (see
`CONTRIBUTING.md`). Never commit `target*/`; `.gitignore` already excludes it.

**The gate is not an inner loop.** A whole-tree `cargo xtask ci --all` is a
heavy job: from a cold cache it is about **twenty-three minutes** on this
machine, and it rebuilds every test binary in the tree. This is a shared
machine, and the gate takes more of it than anything else in the tree.

- **Batch the work, then gate it once.** Make one big change, or one batch
  of them, and run the gate **once** for the batch. Do not run it after every
  small step, and do not run it several times a day to feel safer — the
  second run of the same tree says nothing the first did not.
- **While you work, use the cheap checks.** `cargo fmt`, `cargo clippy -p
  <crate>`, `cargo test -p <crate>` and `cargo test -p craftkit-gates` are
  seconds. The whole-tree gate is what runs before a commit and before a
  push, and it is run when there is a batch to put through it.
- **A green gate is a fact about a commit, not a running state.** It says
  *this* tree is green; it does not stay green while the tree keeps moving.

**`main` is the only working branch, and there is one worktree.**

- **A branch is never created without the owner's word.** If a piece of work
  wants a branch of its own, it is asked for — and an agent does not create
  one on its own account. A branch an agent made for itself is work the owner
  did not ask for, sitting somewhere the owner is not looking.
- **There is one worktree on disk, and it is `main`.** The tree is checked out
  once. A second checkout is a second place where the tree can be changed, and
  the second place is where a change goes that nobody reviews.
- **Every other branch is deleted**, local and remote, as soon as its work is
  on `main`. A merged branch is a pointer into the history and nothing else,
  and an unmerged one is work that is not in the tree yet.

---

## Delegation

A sub-agent is a round trip that costs money and returns an answer, so the
rule is about the **brief**: **a sub-agent gets a bounded, mechanical brief,
or it does not get spawned.** One survey per battery, with a written
contract; a question the tree can answer is not a brief; and reading what a
rule *means* stays with the reader. The rule is in
[`docs/agent-playbook.md`](docs/agent-playbook.md), and the gate checks that
it is still there.

---

## Where things are

```text
crates/craftkit-sdk       L2  the whole surface a plugin needs
crates/craftkit-derive    L0  the macro that writes the declaration, the reader, the registration
crates/craftkit-gates     L0  the tree's own gate
crates/craftkit-scratch   L0  the scratch a test works in: a path, and the removal of it
crates/craftkit-control   L3  the loopback JSON-lines channel
crates/craftkit-mcp       L3  the MCP server
crates/craftkit-plugin-*  L4  the batteries
apps/craftkit-repl        L5  the terminal door
apps/craftkit-mcp-server  L5  the agent door
apps/craftkit-wasm        L5  the browser door
apps/craftkit-window      L5  the window
apps/craftkit-bridge      L5  the bridge (JSON-RPC over stdio)
apps/craftkit-studio      L5  the studio: the ecosystem every front end builds
xtask                     the outer gate
```

The full layering table is in `docs/architecture.md`.
