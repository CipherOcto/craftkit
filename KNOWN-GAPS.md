# Known gaps

What this tree does **not** do, and what its gates cannot see. This file is the
honest list: a gap written down is a gap somebody can close, and a gap nobody
wrote down is a gap somebody will rediscover as a surprise.

Nothing here is a code gap unless it says so. Some of these are things this
tree has not been asked for, and they are here so that nobody reads them as
defects.

---

## 1. The outer gate and the inner gate check different halves

The tree has **two** gates, and they do not overlap.

- The **inner** gate is `crates/craftkit-gates`. It knows about the crates: the
  house deny attribute, the layering, what a plugin may depend on, the bounds
  the README names, and that a battery declares its commands as rows of a
  table. It runs as `cargo test -p craftkit-gates`.
- The **outer** gate is `xtask/`. It knows about the **repository**: the layer
  table in `docs/architecture.md` and the inner gate's own table agree, every
  gate the tree names is actually run, every document link in the prose points
  at a file that is in the tree, the `mod` groups are sorted, every crate is a
  member of the workspace, the xtask holds itself to the same deny list as the
  crates, and every rule the v1 tree had and this one does not is **named**
  with the reason. It runs as `cargo xtask ci`.

Two gates means two places to look, and that is a cost. It is paid on purpose:
the inner gate is the tree's own, and it can be run by anybody who has the
tree, without the repository's tooling; the outer gate is the repository's,
and it knows the things the tree cannot know about itself — its own
documentation, its own workflow, and its own history.

**What the outer gate cannot see.** It reads the tree as text, because a
checker that cannot be built when the tree does not compile is a checker that
is silent exactly when it is needed. So it does not run cargo to ask what a
crate depends on; the inner gate does that, by reading the manifests, and the
outer gate checks that the inner gate's *table* and the *document* agree.

## 2. What the tree's own gate cannot see

The gate in `crates/craftkit-gates` is a **reading** of the tree, not the tree
itself, and it has four honest limits:

- **It reads text, not cargo.** It parses the manifests and the crate roots.
  That is what makes it fast and hermetic, and what makes a manifest written
  in a form the reader does not know read as *no dependencies at all*.
- **The table of layers is written by hand.** A new crate is a two-step change:
  the crate, and the table. A table generated from the manifests cannot catch a
  manifest that is wrong.
- **It does not hold the list of crates that may hold `unsafe`.** That list is
  in `crates/craftkit-alloc/tests/gate.rs`, which reads the whole workspace and
  says so.
- **A bound whose number cannot be worked out is not checked.** The rule
  "the numbers agree with the prose" evaluates the expressions; a bound
  written in a way the evaluator does not know is silently not checked, and
  the gate says nothing rather than guessing.

---

## 3. What the other gates cannot see

- **The clean-room gate checks names, not copying.** Copied code does not
  carry the product's name, so a gate that checked that would be a gate that
  could never pass. The rest is the review, and the table in
  `docs/clean-room.md` is what the reviewer reads.
- **The assets gate checks the tree's files and their attribution.** It cannot
  check that a drawing in code is a drawing and not a copy.
- **The dependency policy checks the licences a manifest claims.** It cannot
  check that anybody read them.
- **The deny attribute is a lint, not a proof.** `forbid(unsafe_code)` holds
  the code; a `no_mangle` that the platform looks up by name is the one
  exception, and it is named, in one list, with one marked place each.

---

## 4. What the design honestly does not do

These are the README's own honest notes, and they are still true of the tree:

- **The linker is part of the design.** The anchor macro is what keeps a
  plugin in the binary. Without it, the plugin's contributions are missing and
  nothing says so.
- **A run-time command is written out by hand.** A command made while the
  program runs has no struct to derive from, so it reads its own parameter.
- **The `Either` is a compromise.** One parameter with two spellings of
  different kinds cannot be a single typed value, so the command is handed
  whichever half arrived and decides what to do with it.
- **A subprocess plugin's metadata is leaked, once.** A command's labels are
  `&'static str` and a subprocess plugin's labels arrive at run time, so they
  are leaked when the plugin is loaded (`crates/craftkit-sdk/src/plugin.rs`).
  The leak is bounded by the number of commands the plugin has, and once each.
- **A subprocess plugin is not checked by the compiler.** Its catalogue is
  data, and a mistake in it is refused when the plugin is loaded.
- **The wire protocol is a decision, not a derivation.** Newline-delimited
  JSON was chosen because it is trivial to write in any language.
- **The bridge is a proof, not a product.** It speaks JSON-RPC over stdio
  because that is the cheapest way to show a second front end over the same
  registry.
- **The plan is a value, not a drawing.** Nothing renders it, and nothing
  checks it against a rule.
- **The third door does not exist on the web.** A subprocess plugin needs a
  process, and `wasm32-unknown-unknown` has none.
- **The document model is still small.** The *seam* is the thing being
  demonstrated: a version is a contribution, a step is a contribution, and the
  chain is checked. A document worth versioning is a different piece of work.

---

## 5. Things this tree has not been asked for

Not gaps in the design — things nobody asked for:

- **There is no toolbar.** The window draws the commands by area and the
  panels a battery contributes, and a toolbar is not part of it.
- **The plugin formats are a shim** over a door that is built.
- **The AU's own battery is CoreFoundation's**, and CoreFoundation is macOS.
- **The code mutation is CI-only**, by the machine's rule, so no claim in the
  README has been measured by mutation. The v2 gates are: the tests, clippy,
  fmt, the workspace-scanning gate that holds the list of allowed `unsafe`,
  and the tree's own gate — which is measured by being pointed at trees that
  are broken on purpose.

---

## 6. Things the README has claimed and the tree has since changed

The README is a build log, written in order, and some of its early sentences
have been overtaken by later sections. The ones that matter to a reader who
comes to the tree cold:

- **"There is no numeric core."** There is now `craftkit-numeric`, a typed
  value (a length in millimetres, an angle in radians), at L3, because a
  length can be a command's parameter and that means speaking the SDK's own
  types.
- **"The old tree's format is not here."** It is still not here: v2's format
  is a **contribution**, with migrations that are contributions, and a version
  chain the ecosystem checks. The old tree's one-file-per-change discipline
  belongs to a tree that is not this one.
- **"The front ends are not built."** The terminal door, the agent door
  (`apps/craftkit-mcp-server`), the bridge, the browser door and the window
  are all built, and all of them reach the same registry.

When the README and the code disagree, the **code** is right, and this file
records the difference rather than the README's history.
