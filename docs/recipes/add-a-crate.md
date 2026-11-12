# Recipe: add a crate

A crate is a directory, a manifest, and **a layer**. The layer is not a
formality: the gate reads the table, and a crate that is not in it is a
finding.

---

## 1. Make the crate

```text
crates/craftkit-<name>/
├── Cargo.toml
└── src/
    └── lib.rs
```

### `crates/craftkit-<name>/Cargo.toml`

```toml
[package]
name = "craftkit-<name>"
description = "<one line: what it is>"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[dependencies]
# Only what it needs. The layering is the point: a crate may depend only
# on crates in strictly lower layers.
craftkit-sdk = { workspace = true }
```

The workspace's `members` list is `["crates/*", "apps/*", "xtask"]`, so **a new
crate under `crates/` is a member already** — no edit to the root
`Cargo.toml`'s `members` is needed.

If the crate is one other crates will name, add it to the workspace's
`[workspace.dependencies]` as well, with a version:

```toml
craftkit-<name> = { path = "crates/craftkit-<name>", version = "0.2.0" }
```

### `crates/craftkit-<name>/src/lib.rs`

The crate root carries the **house deny attribute**, in this order, and the
`forbid`:

```rust
//! `craftkit-<name>` — <what it is>.
//!
//! <why it exists, and what it knows nothing about>.

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

`crates/craftkit-gates` compares the lints and ignores the formatting, so one
line and six lines are the same six lints — but the **order** must be the
same, because a deny list that varies between crates is a deny list nobody can
reason about.

**If your crate must hold `unsafe`**, it does not `forbid`: it `deny`s, and it
carries exactly one `#[allow(unsafe_code)]` on the one place that needs it.
The crates that may do that are **named in one list**, in
`crates/craftkit-alloc/tests/gate.rs`, with a reason each. An eleventh
exception is a line in that file and a reason in it.

---

## 2. Give it a layer

The layering table is `crates/craftkit-gates/src/lib.rs`:

```rust
pub const LAYERS: &[(&str, u8)] = &[
    ...
    ("craftkit-<name>", 3),
    ...
];
```

The table is sorted **by layer and then by name**, so that the table itself is
reviewable as a diff. The layers are:

| layer | what belongs there |
|---|---|
| L0 | the half that knows nothing about a document: the macro, the ABIs, the allocator gate, the libraries that are arithmetic over plain numbers |
| L1 | the doors: opening a shared library and calling the table in it, and their reference fixtures |
| L2 | `craftkit-sdk`, the whole surface a plugin needs |
| L3 | the things above the SDK that are not plugins: the control channel, the MCP server, the numeric library, the rasteriser, the vector geometry |
| L4 | the batteries: the plugins |
| L5 | the applications — **exempt** from the layering rule |

A crate may depend only on crates in **strictly lower** layers. A **plugin**
(L4, named `craftkit-plugin-*`) may not depend on an application, and may not
depend on another plugin. A **dev** dependency is exempt: a door's loader
tests need a shared library to open, and that library is a plugin.

---

## 3. Write it, with tests

- The crate's own rule has a test. A rule with no test is a habit.
- Tests may panic; nothing else may.
- No `unwrap`, no `expect`, no `panic!`, no `todo!` outside tests.
- Input from outside is **hostile**: read it through typed helpers, check
  finiteness, bound sizes, cap loop counts.

---

## 4. Run it

```bash
cargo test -p craftkit-<name>
cargo clippy -p craftkit-<name> -- -D warnings
cargo fmt --check
cargo test -p craftkit-gates
```

The gate is the one that will tell you the layer is wrong: it reports a crate
that depends on a **higher** layer, on the **same** layer, or one that is not
in the table at all.

---

## 5. Say it in the README and in the docs

- The root `README.md` is the design: a new crate belongs in the battery's
  story, and any **public bound** it introduces (`MAX_*`) must be **named in
  the README with the same number the code has**.
- `docs/architecture.md` holds the layering table as a markdown table; it is
  the tree's own copy of the gate's table, and the two must agree.

---

## 6. Commit

One topic per commit:

```text
feat(<name>): <what it does> — §NNN
docs: <what it does> — §NNN
```
