# craftkit

A clean-room, pure-Rust workspace for **parametric plans** — ordered nodes with
property trees, keyframes on an exact integer timebase, evaluated into a flat
plan you can reason about.

It is also a **scaffold**. Every part of it is meant to be read, copied and
re-purposed: the layering, the command registry, the panic boundaries, the
gates, the agent's way in. Nothing here is a product to be defended; it is a set
of decisions that are known to work, written down so they can be reused.

```bash
cargo xtask doctor   # can this machine build it?
cargo xtask ci       # is it green? seconds warm, minutes cold
cargo run --release -p craftkit -- --sample
```

Over 500 tests (`cargo test --workspace`). No image, font or audio file
anywhere in the tree.

---

## What it does

A plan is an ordered list of **nodes**. A node has a kind, a property tree, and
keyframes on any property:

| Kind | What it does |
|---|---|
| `Source` | introduces a value |
| `Filter` | transforms the value flowing through it |
| `Sink` | consumes the value |
| `Cue` | fires at a tick without carrying a value |
| `Comment` | says something to the person editing |

Evaluation walks the chain under `craftkit_kernel::guard` and produces a `Plan`
— a flat, comparable result, plus diagnostics. Time is an integer: `Tick` counts
254 016 000 000 ticks per second, so a frame boundary is exact rather than a
rounded float. **Not** "16.0 seconds"; `403_200_000` ticks, always the same
number twice.

## The shape

```text
crates/core     L0   ticks, values, ids, hostile parameters
crates/model    L1   the document: nodes, properties, keyframes
crates/kernel   L2   the numeric kernel, behind the panic guard
crates/eval     L3   document -> plan
crates/io       L3   the versioned .craftkit format
crates/engine   L4   the command registry and the Session
crates/control  L5   the loopback JSON-lines channel
crates/mcp      L5   the MCP server
crates/ui       L6   the egui shell, icons, automation names
apps/*             the desktop window, the CLI, the browser build
xtask               the gates
```

A crate may depend only on **strictly** lower layers, and nothing below L6 may
name a UI toolkit. `cargo xtask layers` checks both against every manifest, so
the boundary cannot erode one convenience import at a time.

## The four ideas worth copying

**Everything is a command.** There is exactly one registry —
`crates/engine/src/commands.rs` — and the toolbar, the scripts, the CLI, the
control channel and the MCP server all reach it. A panel never writes to the
document; it calls `session.execute`. That is what makes the thing an agent can
drive and a person can click the *same* surface, so a bug found by one is a bug
in both.

**Never index a parameter.** Every value that arrives from outside — a command
parameter, a `.craftkit` file, a line on the control channel — is read through a
typed, range-checked helper in `crates/core/src/params.rs`. An index that is out
of bounds is a panic, and this is a program whose whole promise is not crashing.

**Clone before edit, then one trust boundary.** `Session::execute` clones the
document, runs the command under `catch_unwind`, and on a panic restores the
snapshot and reports an error. A command that panicked is a command that did not
happen. Below that, `craftkit_kernel::guard` does the same for code craftkit
does not own.

**No assets, ever.** Every icon is vector paths in `crates/ui/src/icons.rs`,
drawn in code from original coordinates. There is no sprite sheet, no icon font,
no SVG. `cargo xtask assets` fails the build if anything embeds a non-source
file, and `ATTRIBUTION.md` is the ledger.

## Running it

### A window

```bash
cargo run --release -p craftkit -- --sample
cargo run --release -p craftkit -- --sample --control 4711   # and drive it
```

### Headless

```bash
cargo run -q -p craftkit-cli --bin craftkit-cli -- commands          # the catalogue
cargo run -q -p craftkit-cli --bin craftkit-cli -- run script.json    # a script of commands
cargo run -q -p craftkit-cli --bin craftkit-cli -- eval plan.craftkit # what it evaluates to
cargo run -q -p craftkit-cli --bin craftkit-cli -- snapshot script.json --out shot.png
cargo run -q -p craftkit-cli --bin craftkit-cli -- crashes list       # crash reports
```

`snapshot` rasterises in software: it walks the shape list, samples the font
atlas for text, and writes a PNG. Hard edges, centroid gradients, only the font
atlas uploaded — good enough to *look at* a frame from a build machine, not a
renderer.

### From an agent

```bash
cargo run -q -p craftkit-cli --bin craftkit-cli -- mcp     # MCP over stdio
```

One tool per command, named `craftkit_<id>`, with the schema generated from the
command's own parameter documentation — so the tool an agent sees cannot drift
from the command a person clicks.

### In a browser

```bash
cargo xtask wasm
wasm-bindgen --target web --out-dir apps/craftkit-web/dist \
    target/wasm32-unknown-unknown/release/craftkit-web.wasm
# serve apps/craftkit-web/ over HTTP — file:// will not do
```

Same engine, same commands, same shell. The plan is stashed in `localStorage`
as ordinary `.craftkit` text, so it moves between a browser and a desktop
unchanged. Saving is explicit: nothing is written without the button.

`craftkit-web --selftest` draws frames headlessly on a machine with no browser
at all, which is how the web build stays testable in CI.

## The gates

```bash
cargo xtask doctor   # blocking / degraded / info about this machine
cargo xtask ci       # fmt, assets, lists, layers, web, clippy, tests, wasm
```

| Gate | What it protects |
|---|---|
| `fmt` | one formatting, so diffs are about the change |
| `assets` | no embedded images; every file attributed; no stale attribution rows |
| `lists` | one deny attribute everywhere; the layer table matches the code *and* `cargo metadata`; sorted lists |
| `layers` | strictly-lower dependencies; no UI toolkit below L6 |
| `web` | the canvas id, the bundle path, and the service worker's safety rules |
| clippy | `-D clippy::all -D warnings`, every target |
| tests | every crate, every app, and the gates themselves |
| `wasm` | every library crate still builds for the browser |

A gate that cannot run says **skipped**, with a reason, in the middle of the
output. A skip is never printed as a pass.

`scripts/mutation-test-gates.sh` breaks each rule on purpose and checks that the
gate notices. A gate that cannot fail is worse than no gate, because it teaches
you to trust a check that has never checked anything — and this script found
three that had been quietly unable to fail. What it still cannot catch is in
[KNOWN-GAPS.md](KNOWN-GAPS.md), which is the first file to read when something
looks missing.

## Using it as a scaffold

1. Copy the tree.
2. Rename `craftkit` to your project (`craftkit-*` crates, `craftkit/N` format
   tag, `CRAFTKIT_*` environment variables, the control-channel folder).
3. Replace `crates/core` and `crates/model` with your domain. Keep the layer
   names, the error discipline and the deny attribute.
4. Keep `crates/engine`'s boundary, `xtask`, `skills/` and `docs/recipes/`.
   Those are the parts that keep working as the domain gets stranger.
5. Delete what you do not need — but read `AGENTS.md` first, because most of it
   is about the mistakes you have not made yet.

Start with `AGENTS.md` for the rules, `docs/architecture.md` for the shape,
`docs/agent-playbook.md` for driving it, and `docs/recipes/` for the task in
front of you.

## Licence

MIT OR Apache-2.0. See [ATTRIBUTION.md](ATTRIBUTION.md) for the full ledger:
there is no vendored third-party code, and no file asset of any kind.