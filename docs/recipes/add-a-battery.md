# Recipe: add a battery

A **battery** is a plugin: a crate that adds commands, kinds, tools and panels
to an application, and that the application links in **without naming it**.

The batteries in this tree are `crates/craftkit-plugin-*`: `art`, `cad`,
`clap`, `film`, `fx`, `grid`, `hello`, `image`, `layout`, `numeric`, `pdf`,
`prose`, `shapes`, `shared`, `sound`, `text`, `vector`, `vst3`.

---

## 1. Make the crate

```text
crates/craftkit-plugin-<name>/
├── Cargo.toml
└── src/
    └── lib.rs
```

`crates/craftkit-plugin-<name>/Cargo.toml`:

```toml
[package]
name = "craftkit-plugin-<name>"
description = "<one line: the document model of what>"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[dependencies]
# The SDK is the whole surface a plugin needs, and the macro writes the
# declaration, the reader and the registration.
craftkit-sdk = { workspace = true }
craftkit-derive = { workspace = true }
# Libraries are allowed. **Other plugins are not.**
```

The crate root carries the house deny attribute and `#![forbid(unsafe_code)]`
— see [`add-a-crate.md`](add-a-crate.md).

---

## 2. Say what the battery's document is

A battery is a **document model of its own**. Write, in the crate's own
`//!` notes:

- what a node of this kind **is**;
- what it **carries** (the payload) and what it **relates** to (the tree);
- what is **derived** and never stored;
- what the battery **refuses**.

The rules of the modelling are the tree's rules, and they are the same for
every battery:

- a node may have **no place** — no location and no geometry is not an error;
- the document carries the **payload**, the tree is **containment and
  relations**;
- **derived values are derived** and never stored;
- **refuse, do not clamp** — an out-of-range parameter is an error;
- **"nothing" is an option, not a zero**;
- **unknown names are refused**.

---

## 3. Write the contributions

Four kinds of contribution live in the registry, and a battery may add any of
them:

| Contribution | What it is |
|---|---|
| a **command** | something a person or an agent can do — see [`add-a-command.md`](add-a-command.md) |
| a **node kind** | a kind of thing in the document, with its own evaluation |
| a **tool** | a gesture that runs a command |
| a **panel** | a window's worth of commands |

The kinds, tools and panels are declared with the derive macros in
`crates/craftkit-derive` (`NodeKind`, `Tool`, `Panel`, `Evaluator`, `Layer`),
and the **layer** is what groups contributions that depend on each other. The
shape of the declarations, from `crates/craftkit-plugin-shapes/src/lib.rs`:

```rust
/// The geometry layer.
#[derive(Layer)]
#[layer(id = "geometry", name = "Geometry", requires("core"))]
pub struct GeometryLayer;

/// A rectangle.
#[derive(NodeKind)]
#[node_kind(kind = "geometry.rect", label = "Rectangle", layer = "geometry", evaluates)]
pub struct RectKind;

/// The rectangle tool: a gesture that runs a command.
#[derive(Tool)]
#[tool(id = "tool.rect", label = "Rectangle", layer = "geometry", icon = "rect", command = "geometry.rectangle")]
pub struct RectTool;

/// The panel that shows the shapes.
#[derive(Panel)]
#[panel(id = "panel.shapes", label = "Shapes", layer = "geometry", commands("geometry.rectangle"))]
pub struct ShapesPanel;
```

A tool names a **command that exists** and a panel names **commands that
exist**: the ecosystem checks both when it is built, and refuses a program that
starts with a tool that cannot run.

Every item the battery contributes is named in the anchor:

```rust
craftkit_sdk::plugin_anchor!(HelloGreet, HelloCount);
```

---

## 4. Give it a layer

A battery is **L4** in `crates/craftkit-gates/src/lib.rs`:

```rust
("craftkit-plugin-<name>", 4),
```

The table is sorted by layer and then by name.

**A plugin may not depend on an application, and may not depend on another
plugin.** The seam is the SDK, the macro and the libraries, and nothing else.
The gate checks it.

---

## 5. Link it into an application

The **dependency list is the plugin list**. An application names a plugin by
depending on it and nothing else: `apps/craftkit-studio/build.rs` reads the
manifest, finds the plugin dependencies, and writes the references that keep
them in the program.

In `apps/craftkit-studio/Cargo.toml`, an **optional** dependency and a
feature:

```toml
[dependencies]
craftkit-plugin-<name> = { path = "../../crates/craftkit-plugin-<name>", optional = true }

[features]
# The <name> battery. Off by default: a program that wants <what> says so.
<name> = ["dep:craftkit-plugin-<name>"]
```

And in the application that wants it, the feature is turned on:

```toml
craftkit-studio = { path = "../craftkit-studio", features = ["<name>"] }
```

(`apps/craftkit-mcp-server/Cargo.toml` turns **every** battery on, so that the
tool list an agent sees is the whole kit.)

---

## 6. Test it

- A battery's test needs a session: `Ecosystem::from_inventory()` and
  `Session::with_formats(...)` — see
  [`add-a-command.md`](add-a-command.md) §3.
- A test that opens a shared library needs the fixture crate as a **dev**
  dependency; a dev dependency is exempt from the layering and from the
  plugin rule.
- Every rule the battery states has a test that fails when the rule is broken.

---

## 7. Run it

```bash
cargo test -p craftkit-plugin-<name>
cargo test -p craftkit-studio --features <name>
cargo clippy --workspace -- -D warnings
cargo fmt --check
cargo test -p craftkit-gates
```

---

## 8. Say it in the README

The root `README.md` is the design, and a battery is a chapter of it: what it
models, how the model maps onto the tree's own, and what it refuses. Any
**public bound** it introduces must be named in the README **with the same
number the code has**.

`docs/architecture.md` holds the layering table; the two tables must agree.

---

## 9. Commit

One topic per commit:

```text
feat(<name>): the battery — §NNN
docs: the battery — §NNN
```
