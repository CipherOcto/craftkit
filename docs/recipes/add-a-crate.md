# Add a crate

**When you need this** — a genuinely new layer of code, with its own
dependency direction and its own reason to exist. If the new code can live in an
existing module, it is a new module, not a new crate. A crate costs a
`Cargo.toml`, a deny block, a layer row and a test that checks the layering
holds forever.

**Before you start**

```
cargo xtask ci
```

is green. Do not add a crate on top of a broken tree — the layer check will
report your crate's violations against a baseline you cannot trust.

---

## Step 0: do you actually need a crate?

Ask whether the new code has a *different set of things it is allowed to
depend on*. If it does not, it belongs in `crates/engine/src/` and costs
nothing.

The existing dependency graph, from the manifests:

```text
core          (serde, serde_json)                    no craftkit-* at all
model         core
kernel        core
eval          core, model, kernel
io            core, model
engine        core, model, eval, kernel
control       core, engine
mcp           core, model, engine
ui            core, model, engine, eval, egui
apps/*        eframe + whatever they need            exempt from layering
xtask         serde, serde_json                      build tooling
```

A new crate is justified when it needs a level nobody else has — a second
renderer that must not be able to see the UI, a headless evaluator that must
not be able to see the filesystem, a test kit allowed to depend on everything.

---

## Steps

### 1. Create the directory and the manifest

```
mkdir -p crates/<name>/src
```

`crates/<name>/Cargo.toml`. Copy `crates/io/Cargo.toml` and change the name and
description:

```toml
[package]
name = "craftkit-<name>"
description = "<one line: what this crate is for>"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true

[lints]
workspace = true

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
log = { workspace = true }
craftkit-core = { workspace = true }
craftkit-model = { workspace = true }
```

Every field is `workspace = true`. Never pin a version in a crate manifest —
one place decides versions.

`[lints] workspace = true` is not optional. It is what applies
`[workspace.lints.rust] unsafe_code = "forbid"`.

### 2. Wire it into the workspace

`members` in the root `Cargo.toml` is already a glob:

```toml
members = [
    "crates/*",
    "apps/*",
    "xtask",
]
```

A new directory under `crates/` is picked up with no edit. **But** anything
another crate will depend on needs an entry in `[workspace.dependencies]`,
which *is* a hand-written list:

```toml
# --- internal -------------------------------------------------------------
craftkit-core = { path = "crates/core" }
craftkit-model = { path = "crates/model" }
craftkit-eval = { path = "crates/eval" }
craftkit-kernel = { path = "crates/kernel" }
craftkit-io = { path = "crates/io" }
craftkit-engine = { path = "crates/engine" }
craftkit-control = { path = "crates/control" }
craftkit-mcp = { path = "crates/mcp" }
craftkit-ui = { path = "crates/ui" }
craftkit-<name> = { path = "crates/<name>" }     # keep the list in this order
```

Keep the internal list in the existing order — it reads bottom-up, and it is
the same list every concurrent PR touches.

### 3. Put the deny attributes on the crate root

First line of `crates/<name>/src/lib.rs`, after the module docs:

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
line is 112 characters: `cargo fmt` reflows it every time. Paste the block as
written and let the formatter have it.

`unsafe_code` is also forbid-forced by `[workspace.lints.rust]`, and the six
clippy lints *are* in the workspace table, so a crate that forgets the
attribute gets them anyway. But the attribute is still required, and
`cargo xtask lists` checks it: the comparison normalises whitespace, case and
the `clippy::` prefix, so the one-line and multi-line forms both pass — but the
six lints must be present, in that order.

To see the exact block to paste:

```
cargo xtask lists
```

The failure message prints it.

`clippy.toml` is what makes this tolerable: `allow-unwrap-in-tests`,
`allow-expect-in-tests`, `allow-panic-in-tests` and
`allow-indexing-in-tests` are all `true`, so `#[cfg(test)]` code may use
`unwrap()` and index freely. Production code may not.

### 4. Add the layer row

The table is `xtask/src/layers.rs::TABLE`, and `cargo xtask layers` enforces it
against every `Cargo.toml` in the workspace. It names **full package names**:

```rust
pub const TABLE: &[(&str, Class)] = &[
    // L0 — values and units. No workspace dependency at all.
    ("craftkit-core", Class::Layer(0)),
    // L1 — the document: nodes, properties, keyframes.
    ("craftkit-model", Class::Layer(1)),
    // L2 — the numeric kernel, behind the panic guard.
    ("craftkit-kernel", Class::Layer(2)),
    // L3 — two ways to get a value out of a document: evaluate it, or load it.
    ("craftkit-eval", Class::Layer(3)),
    ("craftkit-io", Class::Layer(3)),
    // L4 — the command registry and the session that runs it.
    ("craftkit-engine", Class::Layer(4)),
    // L5 — the two ways an agent reaches the engine.
    ("craftkit-control", Class::Layer(5)),
    ("craftkit-mcp", Class::Layer(5)),
    // L6 — the shell. The only library crate that may name egui.
    ("craftkit-ui", Class::Layer(6)),
    // The binaries and the build tool. Exempt.
    ("craftkit", Class::Exempt),
    ("craftkit-cli", Class::Exempt),
    ("craftkit-web", Class::Exempt),
    ("xtask", Class::Exempt),
];
```

Two constraints are not negotiable:

- **`craftkit-core` is L0**, permitted to depend on no workspace crate at all.
  Nothing below it exists.
- **`craftkit-ui` is the first layer allowed a UI toolkit** (`UI_MIN_LAYER = 6`).
  `egui`, `eframe`, `egui-wgpu`, `wgpu`, `winit`, `rfd` and `accesskit*` may not
  appear below it. The apps are `Class::Exempt`, which is why an app can have a
  window while `craftkit-ui` cannot — and why a value crate that needs a `Vec2`
  needs its own tiny vector type rather than `egui`'s.

The rule that matters:

> **The table must name only crates that exist, and every crate that exists must
> have a row.** A row for a crate that does not exist is a rule that silently
> checks nothing. A crate with no row has no layer, and therefore no
> constraints at all — it can reach `craftkit-ui` and `egui` from anywhere.

`cargo xtask layers` reports both, so you do not have to remember to check.

### 4b. Add the row to the documentation too

`docs/architecture.md` carries the same table, and `cargo xtask lists` compares
the two **and** checks its "depends on" column against `cargo metadata`. Add the
row in both files in the same commit, or the gate fails and the documentation
drifts.

### 5. Verify

```
cargo build -p craftkit-<name>
cargo clippy --profile ci -p craftkit-<name> -- -D warnings
cargo test -p craftkit-<name>
cargo xtask ci
```

---

## The test you must add

Two kinds, and both are required.

### A. A real test of the crate

Not a smoke test. Whatever the crate exists to do must be tested at its own
level, and the test must fail if the crate is stubbed out.

### B. A test that the crate is what it claims

If the crate exists to enforce a boundary, the boundary is the thing to test:

```rust
#[test]
fn this_crate_does_not_depend_on_the_ui() {
    // A dependency edge here would let the renderer reach a window, which is
    // the one thing it must never do.
    let tree = std::process::Command::new(env!("CARGO"))
        .args(["tree", "-p", env!("CARGO_PKG_NAME"), "--edges", "normal", "-q"])
        .output()
        .expect("cargo tree runs");
    let out = String::from_utf8_lossy(&tree.stdout);
    assert!(!out.contains("craftkit-ui"), "{out}");
    assert!(!out.contains("egui"), "{out}");
}
```

Only write this if it is worth having — it shells out, so it is slower than the
rest of the suite, and `cargo xtask layers` is the better version of the same
check. Do write the equivalent if the crate's whole reason for existing is a
restriction: a "headless" crate that can see `egui` is not headless, and nothing
else in the tree will say so.

---

## The failure this recipe is designed to prevent

### The crate root without the deny line

This is the one that happens, because nothing enforces it. The comment at
`Cargo.toml:48` says the workspace lints exist "so a new crate that forgets the
attribute still gets it" — and they do not. `[workspace.lints.clippy]` sets
`dbg_macro` and `todo` only. A new crate whose `lib.rs` forgets the `deny`
line compiles, passes clippy, passes `cargo xtask ci`, and carries an
`unwrap()` in a `run` function that a control-channel request can reach. The
panic is caught by `Session::execute`'s `catch_unwind` and turned into
`EngineError::Internal`, so the user sees an error and nobody ever finds out it
was a panic.

That is the whole failure mode of the "never crash" rule in this repo: not a
visible crash, but a panic that has been laundered into an ordinary error and
is therefore invisible.

### The layer row that names nothing

The layer check is only as good as its table. Two failure shapes, opposite:

- **A row for a crate that does not exist.** The rule is satisfied by nothing,
  and it will keep being satisfied as long as the name stays wrong. A layer
  table is a set of names matched against `cargo metadata`; a name that
  resolves to nothing is a check that never fires.
- **A crate with no row.** It has no layer, so it has no constraints, and it can
  depend on `craftkit-ui` and `egui` from anywhere — which puts a window
  dependency inside L0 and breaks every guarantee the layering exists to give
  (everything below L6 testable headlessly; the UI crate swappable).

So when you touch the table, check both directions: every row resolves, and
every crate on disk has one.

### An `unsafe` block "just this once"

`#![forbid(unsafe_code)]` cannot be overridden from inside the crate — that is
the difference between `forbid` and `deny`. If you need `unsafe`, the crate
cannot have it, and the design has to change. Do not "temporarily" switch
`forbid` to `deny` to get a build green; that diff is the first step of a
dependency on unverifiable memory safety, and it is the one change in this repo
with no safe rollback.

### A dependency added "just for the tests"

`[dev-dependencies]` may reach anything — that is the point of a test kit. But
a *normal* dependency added for a test pulls the crate into the shipped binary
and into every other crate that depends on it. If a dependency is only used by
`#[cfg(test)]`, it belongs under `[dev-dependencies]`, and the layer table
should not have to move because of it.
