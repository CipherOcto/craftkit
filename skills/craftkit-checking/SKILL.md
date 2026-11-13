---
name: craftkit-checking
description: Use when something in craftkit is red or unproven — a failing test, a clippy error, a gate that says no, a command that refuses, or a crash to diagnose and reproduce.
---

# Checking craftkit

Something is broken, or you have to prove that something works. This is the
order to find out, and how to read what comes back.

The repository root is the directory that holds `crates/`, `apps/` and
`xtask/`.

---

## 1. Read the failure before you run anything

| the failure | what it is |
|---|---|
| `cargo xtask doctor` | the machine, not the code: the toolchain, the targets, the tools |
| a failing `cargo test` | the tree's own rules; the test's name says which rule |
| a clippy error | the house deny list, or a lint the tree turned on |
| the gates' gate | `cargo test -p craftkit-gates`: the deny attribute, the layering, the plugin rule, the thin front end, the bounds in the prose, the battery that is a table |
| `cargo xtask cleanroom` | a prohibited product's name outside prose |
| `cargo xtask assets` | a file that is not attributed, or an asset embedded in code |
| a command that **refuses** | the tree's own rule, working |
| a panic | a crash; see `docs/recipes/fix-a-crash.md` |

---

## 2. The order

1. **The machine.** `cargo xtask doctor` says whether this machine can build
   the project at all. A failure that is really the machine looks like a
   failure of everything else.
2. **The narrowest thing.** `cargo test -p <the crate>` before the whole
   suite. A hundred cheap runs of one test binary beat a hundred runs of
   everything.
3. **The tree's own gate.** `cargo test -p craftkit-gates`. It is fast, and
   it is the gate that knows this tree.
4. **The whole gate.** `cargo xtask ci` — fmt, assets, cleanroom, lists,
   layers, web, clippy, tests, wasm. `KNOWN-GAPS.md` §1 says what this gate
   does in this tree today; read it before you believe a failure of it.

**The machine is shared.** Check the load before a big build
(`cut -d' ' -f1 /proc/loadavg`), build with `nice -n 15`, and do not run a
suite that takes all the CPU for more than five minutes without asking. One
run of the long suite is a CI job, not a local habit.

---

## 3. How to read the results

- **A refusal is a result.** `refused: <why>` is the tree's rule working. Do
  not "fix" it by sending a value in range unless the rule says the value
  was wrong.
- **A panic is caught, not ignored.** `Session::execute` puts the document
  back and reports `EngineError::Internal`. The document is as it was: the
  command did not happen.
- **The gate's finding names the file and the line.** It is a reading of the
  tree, and a finding is a place to look.
- **The plan carries failures.** An evaluator that panicked is skipped, and
  the plan says what it could not say. A plan with a failure is missing
  something; it is not a pass.
- **A skipped gate is not a green gate.** A gate that could not run says so.

---

## 4. Driving the kit to see the result

`docs/agent-playbook.md` has the whole thing. The short version:

```bash
cargo run -p craftkit-repl
```

```text
commands
hello.greet {"name": "world"}
```

The terminal door runs a command by its id with JSON parameters. The agent
door is `cargo run -p craftkit-mcp-server` (MCP over stdio, one tool per
command). The control channel drives a program that is **already running**:

```json
{"id": 1, "method": "engine.execute", "params": {"command": "hello.greet", "params": {"name": "world"}}}
```

The methods are `engine.execute`, `engine.describe`, `session.state`,
`diagnostics.list`, `ping`, `shutdown`, `ui.render`, `ui.click` and
`ui.drag`.

---

## 5. When you cannot find the cause

- `KNOWN-GAPS.md` — what this tree does not do, and what its gates cannot
  see. A missing feature and a broken gate look the same from the outside.
- `docs/architecture.md` — the layers and the seams: a command that is not
  found is usually a crate that is not linked in.
- The **journal** (`session.state`, or `diagnostics.list`) says what ran and
  what was given.
