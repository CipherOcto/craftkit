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
  the README names, that a battery declares its commands as rows of a table,
  that every test file is a test the tree runs, and that a test does not
  litter. It runs as `cargo test -p craftkit-gates`.
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
- **The table of layers is written by hand.** A new crate is a change to the
  crate **and** to the table, and the outer gate keeps a second table that has
  to agree with the first: the tree's own (`crates/craftkit-gates`), the outer
  one (`xtask/src/layers.rs`) and the architecture document. A table generated
  from the manifests cannot catch a manifest that is wrong.
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

## 4. The doors are classified; the matrix is kept by the gate, and the doors are compared by a program

The tree has **six doors**, and the rule "a front end is thin" now reaches
all six (README §1116): the terminal door, the window, the browser door,
the agent door, the bridge and the a2ui door. The **transports** — the MCP
library and the control channel — are not doors, and the **studio** is a
**host**: its entry point is a front end, and its library is a command
book.

**The parity matrix is kept, not kept by hand.** The table is written — the
six doors side by side, cell by cell — and **every cell that says
something names the test that proves it**
([`docs/door-parity.md`](docs/door-parity.md)). The outer gate reads the
matrix and checks two things: that a cell which says something names a
test, and that the test it names is **there** — a file under a `tests`
directory, with a function of that name in it. A named test does not drift
when the code moves, and a line does.

What is still **not** checked is the *cells* themselves — that is, the gate
does not read a door and compare what it does with what the table says, and
it does not check that a named test is *about* the cell it is in.

**The doors are compared with each other, though.** The parity harness
(`apps/craftkit-parity`, §1128) drives five of the six doors with the same
five questions and says where they differ: the same commands listed, the
same message for the same command, the same plan published, the same word
for the undo, and every door naming the command it refused. It is the harness the survey asked for — a crate of its own and
**four** tables to change, not three: the tree's own layer table, the
outer gate's table, the architecture document and the README. The
**a2ui door** is not in it, because it speaks a2ui's own vocabulary and
there is nothing to compare.

What is left is a **cell** read by a person, and that is **not** a defect:
nothing in the tree is broken by a table that drifts, and the cells that
matter most are each proved by a test the tree runs.

**And the bridge's protocol changed** when the bridge became thin:
`resources/list` carries the **manifest** now and not the evaluator's ids,
because the manifest is the door that names every evaluator and the
registry is not a door at all. The bridge is a **proof** and not a
product, and a proof that changes shape is cheap to change now and would
not be later.

---

## 5. What the design honestly does not do

These are the README's own honest notes, and they are still true of the tree:

- **The linker is part of the design.** The anchor macro is what keeps a
  plugin in the binary. Without it, the plugin's contributions are missing and
  nothing says so.
- **A run-time command is declared, and there is no macro for it.** The
  SDK writes the fields and the reader (§1120), and what is left is the
  leak: a run-time command's names are `&'static str` and a name that
  arrives at run time cannot be static, so each one is leaked once.
- **A run-time command's declaration is checked against the call, and not
  against its body.** The **kind** is checked: the session refuses a value
  that is not of the kind the command declared, so a command that declares
  a text and reads a number is refused at the door (§1124). A compiled
  command is not refused this way — the compiler writes the reader and the
  declaration from the same struct — **unless** a kind says it takes a
  string and its own reader does not, which is a convention on the kind
  and not a promise of the compiler (§1129). The **required** flag is checked
  against the **call** (§1126): a parameter the declaration says is
  required and the caller did not give is refused. What is **not** checked
  is the flag against the **body**: a body that ignores a parameter it
  declared is not visible to the engine. The **other spelling** of a parameter with two
  carries its own kind now, and the schema publishes each spelling with
  its own shape (§1125) — and it carries it on **every** path: the
  hand-written declaration, the a2ui derive and the subprocess plugin all
  say what each spelling takes (§1129).
- **`mutates` and `undoable` are two flags, and the derive refuses only
  one combination.** A command that is undoable and does **not** change
  the document is refused by the macro — there is nothing to undo. A
  command that changes the document and says nothing about undoing is
  **allowed**: it changes the document and leaves no history, and the
  session judges it by the document and not by the flags. The two flags
  are consistent, and a run-time command declares both the same way.
- **A test file that is a copy of another is caught; a copy that differs
  by a byte is not.** The rule compares the bytes, and an empty test file
  is not caught either — it carries no test, so a copy of one is
  nothing (§1130). The rule reads the crates and the **xtask**, and not
  anything else.
- **A session built by hand is not filtered by layer.** `Session::register`
  refuses a command whose layer is not linked in, and every door builds its
  session with the ecosystem's layers — but a session made from a registry
  holds what it was given, and nothing checks the layer of a command that
  was already there when the session was built. The doors do not do this:
  they ask the ecosystem for the session.
- **The `Either` is a compromise.** One parameter with two spellings of
  different kinds cannot be a single typed value, so the command is handed
  whichever half arrived and decides what to do with it.
- **A subprocess plugin's metadata is leaked, once.** A command's labels are
  `&'static str` and a subprocess plugin's labels arrive at run time, so they
  are leaked when the plugin is loaded (`crates/craftkit-sdk/src/plugin.rs`).
  The leak is bounded by the number of commands the plugin has, and once each.
- **A subprocess plugin is not checked by the compiler.** Its catalogue is
  data, and a mistake in it is refused when the plugin is loaded. What
  the catalogue now carries is each parameter's kind, its leniency and a
  pair's second kind (§1129), so the host checks the plugin's **call**
  against the plugin's own **declaration** — and the plugin's body is
  still the plugin's business.
- **A kind's `ACCEPTS_TEXT` is a convention, not a guarantee.** The
  compiler writes a compiled command's declaration and reader from the same
  struct, but the constant that says "this kind takes a string as well"
  is the kind author's own word, and a kind that says it and whose reader
  does not take one is refused at the door (§1129).
- **JSON Schema's "integer" admits `3.0`.** The reader takes a whole
  number, and the schema's word for that is `integer`; a caller that sends
  `3.0` passes the schema and is refused by the reader (§1129).
- **A test's scratch comes back when its thread ends, and not before.** A
  test that panics takes its thread down with it, and the harness takes the
  process down with it, so the scratch of a test that panicked may be left
  behind. Before §873 the tree's tests left **everything** behind, so this is
  a smaller hole and not a closed one.
- **The outer gate's own scratch is left behind on purpose.** The `xtask`
  writes what a test wrote into `/tmp/craftkit-xtask-tests` and leaves it
  there, so that a person can look at it; the folder is emptied when the
  next run owns the name, and the name carries the process id, so a folder
  of an older run is still there. The tree's seventh rule ("a test does not
  litter") reads the crates of the tree and not the `xtask`, which depends
  on nothing of the tree's own.
- **The wire protocol is a decision, not a derivation.** Newline-delimited
  JSON was chosen because it is trivial to write in any language.
- **The bridge is a proof, not a product.** It speaks JSON-RPC over stdio
  because that is the cheapest way to show a second front end over the same
  registry.
- **The plan is a value, and the window draws it.** The window shows the
  plan's items — the node, the kind and the value — and the plan itself
  is the **command's** work (`doc.plan`) and not the door's. What nothing
  does is **check** a plan against a rule: the items are what the
  command said, and there is no rule that says what a plan must hold.
- **The third door does not exist on the web.** A subprocess plugin needs a
  process, and `wasm32-unknown-unknown` has none.
- **The document model is still small.** The *seam* is the thing being
  demonstrated: a version is a contribution, a step is a contribution, and the
  chain is checked. A document worth versioning is a different piece of work.

---

## 6. Things this tree has not been asked for

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

## 7. Things the README has claimed and the tree has since changed

The README is a build log, written in order, and some of its early sentences
have been overtaken by later sections. The ones that matter to a reader who
comes to the tree cold:

- **"A commit points at the section that describes it."** `CONTRIBUTING.md`
  says the message ends with the README section's number — `feat(cad): the
  table of a drawing — §1600` — and 542 of the tree's 602 commits carry
  one. The **twenty commits of the a2ui arc** carry none: they say
  `feat(v2): <what it does>` and name what they touch instead. The rule is
  still the house's, and it is followed from here; the history is not
  rewritten.

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
- **"The plan is a value, not a drawing."** The **window draws it** now:
  the plan's items — the node, the kind and the value — are on the
  window's own panel, and the plan is still the command's work
  (`doc.plan`) and not the door's. What nothing does is **check** a plan
  against a rule (§5).
- **"The two spellings of a parameter are two kinds."** That was true
  for the **compiled** command and not for the rest: the declaration
  carried the second spelling's kind in one builder, and the hand-written
  declaration, the a2ui derive and the subprocess plugin did not use it
  (§1125). It is true on **every** path now: a pair says what each
  spelling takes, the plugin's catalogue carries it, and the schema
  publishes the pair as required (§1129).

When the README and the code disagree, the **code** is right, and this file
records the difference rather than the README's history.

---

## 8. What the survey left open

The a2ui survey read this tree against a2ui and found opportunities as
well as defects. The defects are repaired and the opportunities that need
no product decision are taken (`docs/survey-a2ui.md` §6). What is left
**waits for a question nobody has answered**, and it is written down here
so that it is not rediscovered as a surprise:

- **O6 — the registry reconciles against itself.** The registry grows
  while a program runs, and a panel that was drawn before a plugin
  arrived does not know about it. The remedy the survey proposed was a
  **version counter** and a reconciliation step. The **sharpest case**
  was closed first: a session can gain a command while it runs, and every
  door reads its list from the session it runs the commands in, so a door
  never offers a command the session does not have (§1121). The **panel's
  side** is closed too (§1127): a panel is a battery's **contribution** —
  a fixed set of the commands that battery brought — and the commands
  that are not a panel's are the kit's own, read from the session every
  time the window draws. A command that arrives while the program runs is
  a button with the rest, and a version counter would be a way of
  noticing what the window already notices.
- **O7 — a path resolves to a plain value, inside the SDK.** The a2ui
  layer resolves its own paths and the door gets plain values, and the
  gate refuses a front end that names the plan — so the *constraint*
  holds. The general API in the SDK does not exist, and it is **large**
  and architecture-defining: it touches the SDK's own data model.
- **O9 — evals.** The tree has no outbound network and no
  secret-management story, so an eval that calls a model needs **two**
  decisions — a posture and a credential story — and not one. That is a
  memo, not an arc.
- **O10 — contracts.** The bounds are in the README and the code, and
  the contracts would be a second place they live. **High**, and the
  honest moment to write them is when they have stopped moving.
