# Contributing

This tree is a **scaffold**: a working, tested workspace meant to be copied and
re-purposed, or driven directly by an agent. Contributions here are usually
small, and they are usually about **making the kit do one more thing**.

---

## 1. The rules that are not negotiable

These are the tree's rules. A contribution that breaks one of them is not a
contribution; it is a rewrite.

- **Never crash.** No `unwrap()`, `expect()`, `panic!`, `todo!`,
  `unimplemented!` or `unreachable!` in non-test code, and no `unsafe` outside
  the ten named crates. Every crate root carries the same deny list, in the
  same order.
- **Everything is a command.** A command is a struct with
  `#[derive(Command)]`; the fields are the parameters. Behaviour added
  anywhere else — a special case in a front end, a bespoke tool, a flag on a
  command line — is the mistake this rule exists to prevent.
- **Independent.** No commercial product's material, and no commercial
  product run, measured or compared against. `docs/clean-room.md` has the
  rule and the table.
- **No assets.** No iconography, images, fonts or artwork: everything is drawn
  in code, and `cargo xtask assets` fails the build if anything embeds a
  non-source file.
- **Rust only.** The only non-Rust files are `apps/craftkit-wasm/index.html`,
  `apps/craftkit-wasm/browser-check.mjs` and the CI workflow.
- **The layering holds.** A crate may depend only on crates in **strictly
  lower** layers. A plugin may not depend on an application or on another
  plugin.
- **The UI is thin.** A front end reads the kit's state and acts through the
  registry. It never writes the document, never calls the evaluator, never
  keeps a value the evaluator computes.
- **The rules of the modelling.** A node may have no place; the document
  carries the payload and the tree is containment and relations; derived
  values are derived and never stored; **refuse, do not clamp**; "nothing" is
  an option and not a zero; unknown names are refused.
- **Every rule has a test.** A rule with no test is a habit.

---

## 2. Before you start

```bash
cargo xtask doctor        # can this machine build the project at all?
cargo test -p craftkit-gates   # is the tree's own gate green?
```

Read `AGENTS.md`, then `docs/architecture.md` and the recipe that matches
your task under `docs/recipes/`.

**This is a shared machine.** Check the load before a big build
(`cut -d' ' -f1 /proc/loadavg`), build with `nice -n 15`, and do not run a
suite that takes all the CPU for more than five minutes without asking. If a
job is expensive, put it in CI rather than running it here.

---

## 3. The gates that must be green

| Gate | What it checks |
|---|---|
| `cargo fmt --check` | the formatting |
| `cargo clippy --workspace -- -D warnings` | the lints, including the house deny list |
| `cargo test` | the tests, one per rule |
| `cargo test -p craftkit-gates` | the tree's own gate: the deny attribute, the layering, "a plugin is a plugin", the thin front end, the bounds in the prose |
| `cargo xtask ci` | the whole outer gate — see `KNOWN-GAPS.md` §1 for what it does in this tree today |
| `cargo xtask cleanroom` | no prohibited product's name outside prose |
| `cargo xtask assets` | no un-attributed file, and no embedded asset |

`cargo xtask ci` is the last thing you run before you commit.

---

## 4. One topic per commit

A commit is one idea. If the message needs an "and", it is two commits.

The message form is:

```text
feat(<scope>): <what it does> — §NNN
docs: <what it does> — §NNN
```

- `feat(<scope>)` for a change that makes the kit do something, where
  `<scope>` is the battery, the crate or the area it belongs to;
- `docs` for the prose that goes with it — the README's section, in its own
  commit, because the prose and the code are reviewed by different eyes;
- `fix(<scope>)` for a crash fix, and `test(<scope>)` for a test that lands
  on its own;
- `§NNN` is the number of the README section the work belongs to. The README
  is the design and the sections are numbered; a commit that changes
  behaviour points at the section that describes it.
- The message may not contain a **backtick** or a shell metacharacter
  (`$(`, `&&`, `||`, a redirect, a leading `!`): the repository's
  `commit-msg` hook refuses them, because a message that is pasted into a
  shell can run what it contains. Write `MAX_NODES`, not `` `MAX_NODES` ``.

Examples from this tree's history:

```text
feat(cad): the table of a drawing — §1600
docs(v2): the table of a drawing — §1600
fix(sound): the clip that read past its end — §812
```

Commit after every arc that builds. Never commit `target*/`; `.gitignore`
already excludes it.

---

## 5. What a review will look for

- **The rule, and the test that breaks when the rule is broken.** A feature
  with no failing test is a feature nobody checked.
- **Refusals, not clamps.** An out-of-range value is an error. A value that is
  silently brought into range is a bug that shows up later, somewhere else.
- **Hostile input.** Everything from outside is bounded, checked and finite.
- **The prose.** The README says what the change does and what it refuses, and
  every public bound it introduces is named there **with the number the code
  has**.
- **No new path.** The command is the way; the front ends reach it and
  nothing else does.

---

## 6. The clean room, once more

Behaviour is designed from public specifications, general conventions and
user feedback. No commercial product's code, icons, artwork, help text,
tooltips, templates, test data, measurements or generated files — and no
commercial product is launched, scripted, screenshotted, measured or compared
against. `docs/clean-room.md` has the table of products, and
`cargo xtask cleanroom` checks the tree against it.

---

## 7. Where things are

```text
crates/craftkit-sdk       L2  the whole surface a plugin needs
crates/craftkit-derive    L0  the macro that writes the declaration, the reader, the registration
crates/craftkit-gates     L0  the tree's own gate
crates/craftkit-control   L3  the loopback JSON-lines channel
crates/craftkit-mcp       L3  the MCP server
crates/craftkit-plugin-*  L4  the batteries
apps/craftkit-repl        L5  the terminal door
apps/craftkit-mcp-server  L5  the agent door
apps/craftkit-wasm        L5  the browser door
apps/craftkit-window      L5  the window
xtask                     the outer gate
```

`docs/architecture.md` has the layering table and the seams;
`docs/agent-playbook.md` has how to drive the kit; `KNOWN-GAPS.md` has what
the tree does not do and what its gates cannot see.
