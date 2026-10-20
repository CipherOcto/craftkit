# craftkit documentation

Start with [`architecture.md`](architecture.md). It is the map: what craftkit
models, the layer table every crate obeys, the data flow, the two panic
boundaries, the undo model, the evaluation contract and the never-crash rules.

## These documents were reconciled against the built tree

Every document here was written before `xtask` existed and has since been
checked, line by line, against the code. That pass corrected four classes of
error, and they are worth naming because each one is the kind of thing a
document drifts into when it is written from a design rather than from a build:

- **Commands that did not exist yet.** Several recipes said a gate "is still
  being filled in". All seven gates are implemented and green.
- **Names from a different project.** `SOLIDCRAFT_*` environment variables where
  this project uses `CRAFTKIT_*`; a `commands.list` control method where the
  real one is `engine.describe`.
- **Claims the code no longer supports.** A recipe warned against a missing
  `#[serde(default)]` on `Property::hint` and `Meta::pending` — both of which
  were fixed, so the warning now describes a bug that is gone. It is kept, in
  the past tense, because the trap is still real for the next field.
- **Line numbers.** Every `file.rs:NNN` reference in this directory was
  re-anchored so that it points at the symbol the sentence names, and the symbol
  is named too. Line numbers rot; names do not.

If you change code that a document describes, change the document in the same
commit. `cargo xtask lists` enforces the two that can be checked mechanically —
the layer table in `architecture.md`, and the deny attribute in every crate root.

## Where to look

| Document | What it answers |
|---|---|
| [`architecture.md`](architecture.md) | How the crates fit together and why the boundaries are where they are |
| [`agent-playbook.md`](agent-playbook.md) | How to drive craftkit as an agent: commands, the control channel, MCP |
| [`patterns/`](patterns/) | The recurring rules, one file each — the things you must not do differently |
| [`recipes/`](recipes/) | Worked end-to-end tasks against real commands |
| [`design/`](design/) | The `.craftkit` file format and the plan model in detail |

## The patterns

Each file states a problem, the rule, an excerpt from the code that is currently
in the tree, and when the rule does *not* apply.

| Pattern | In one line |
|---|---|
| [Never index a parameter](patterns/never-index-a-parameter.md) | every parameter read goes through `Params`, which never indexes and always validates |
| [Everything is a command](patterns/everything-is-a-command.md) | one `CommandSpec` per user-visible behaviour; one `Session::execute` to run it |
| [Clone before edit](patterns/clone-before-edit.md) | a snapshot per command is the undo entry, the error rollback and the no-op check |
| [One trust boundary](patterns/one-trust-boundary.md) | two `catch_unwind` sites with distinct jobs, and no others |
| [Exact time, not float seconds](patterns/exact-time-not-float-seconds.md) | instants are `Tick`s; seconds are an input format, never a storage format |
| [A format change is one new file](patterns/format-changes.md) | one file in `format/pending/` per change; the release folds them in |

Two more ideas are documented where they are enforced rather than as separate
files: hostile input has exactly one owner (`Params` and `Value` validate; the
evaluator deliberately does not re-check — see
[architecture.md](architecture.md#the-never-crash-contract)), and a broken node
produces a diagnostic rather than an early return (see
[architecture.md](architecture.md#the-never-crash-contract)).

## How this fits together

The crate-level `//!` docs are the primary reference; these documents are the
narrative around them, and they do not repeat the API. When they disagree, the
code wins.

craftkit is an independent scaffold. Its structure is informed by publicly
documented conventions from a family of Rust creative applications, but no code,
asset, text or test data is taken from any of them, and craftkit is not affiliated
with them.

## Working on craftkit

Quality gates, in the order a change should pass them:

1. `cargo fmt --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `cargo xtask layers` — the layer table in
   [architecture.md](architecture.md#layers) is checked against every
   `Cargo.toml`
5. `cargo xtask ci` — all of the above, in one command

`xtask` is where every gate lives, including steps 4 and 5. It is an ordinary
workspace member — `.cargo/config.toml` aliases `cargo xtask` to
`run --quiet -p xtask --` — so it has its own tests, its own row in the layering
table, and no code outside the build.

`cargo xtask ci` runs all of the above, plus
`assets`, `lists`, `web` and the wasm check. A gate that cannot run says
**skipped**, with a reason; a skip is never printed as a pass.

Every crate and every application in the tree is implemented. `cargo test
--workspace` covers all of them.
