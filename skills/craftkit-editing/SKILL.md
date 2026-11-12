---
name: craftkit-editing
description: Use when changing something in craftkit — adding a command, adding a crate or a battery, changing a parameter, or changing a file format. The checklist for a change that keeps the tree's rules.
---

# Editing craftkit

You are changing something in this tree. The tree's rules are the point; this
is the order that keeps them.

The repository root is the directory that holds `crates/`, `apps/` and
`xtask/`. Everything below is run from there.

---

## 1. Know what kind of change it is

| change | the recipe |
|---|---|
| a command, in a battery | `docs/recipes/add-a-command.md` |
| a new battery (a new plugin) | `docs/recipes/add-a-battery.md` |
| a new crate, and its layer | `docs/recipes/add-a-crate.md` |
| a command's parameters (a format and prose change) | `docs/recipes/change-a-command.md` |
| a crash | `docs/recipes/fix-a-crash.md` |

Read the recipe before you write anything. It has the exact paths and the
exact code.

---

## 2. The rules you are about to break if you are not careful

- **No panics in non-test code.** No `unwrap()`, `expect()`, `panic!`,
  `todo!`, `unimplemented!`, `unreachable!`. Every crate root carries the same
  deny list, in the same order, plus `#![forbid(unsafe_code)]`.
- **The ten crates that may hold `unsafe`** are named in
  `crates/craftkit-alloc/tests/gate.rs`, with one marked place each. An
  eleventh exception is a line in that file and a reason in it.
- **Everything is a command.** The behaviour goes in a command, not in a front
  end and not in a special case.
- **The layering holds.** Strictly lower layers only. A plugin may not depend
  on an application or on another plugin.
- **The front end is thin.** No `document_mut`, no `evaluators`, no `Plan` in
  a front end's state, no `undo`, no `redo`.
- **Refuse, do not clamp.** An out-of-range value is an error.
- **Every rule has a test.** A rule with no test is a habit.

---

## 3. The order of the work

1. **The rule, in one sentence.** If you cannot write it, the change is not
   ready.
2. **The test that breaks when the rule is broken.** Watch it fail.
3. **The code.**
4. **The prose.** The root `README.md` is the design: what the change does,
   what it refuses, and every public bound it introduces **named with the
   number the code has**.
5. **The commit.** One topic per commit:
   `feat(<scope>): <what it does> — §NNN`, then `docs: <what it does> — §NNN`.

---

## 4. The gates, before you commit

```bash
cargo test -p <the crate you changed>
cargo clippy --workspace -- -D warnings
cargo fmt --check
cargo test -p craftkit-gates
cargo xtask ci
```

`cargo xtask ci` is the whole outer gate and is the last thing you run.
`KNOWN-GAPS.md` §1 says what is true of it in this tree today.

**The machine is shared.** Check the load before a big build
(`cut -d' ' -f1 /proc/loadavg`), build with `nice -n 15`, and do not run a
suite that takes all the CPU for more than five minutes without asking. If a
job is expensive, put it in CI.

---

## 5. Before you commit, read this

- Did the change add a **path** that is not a command? (It should not have.)
- Does the new code **refuse** what it should, and **not clamp** it?
- Is every **public bound** named in the README with the same number?
- Does the layering table in `crates/craftkit-gates/src/lib.rs` list the new
  crate, and does `docs/architecture.md` agree with it?
- Is there a test that **fails** when the rule is broken?
- Is the commit **one topic**, with the prose in its own commit?
