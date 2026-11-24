# What a2ui's patterns would give craftkit

**Version 5** — rewritten under an adversarial review loop that ran until a
round came back clean. **The log is the authority**: every round, every
finding and every fix is in `survey-a2ui-review.md`, and the
round count below is not maintained here. A read of `../a2ui` (patterns only;
`conformance/` out of scope) against the tree at
`/mnt/data/mmacedoeu/work/tools/craftkit`.

---

## 0. What this document is, and what it is not

It is a **transfer guide**: a reading of a2ui's patterns against craftkit's
shape, and the list of things a downstream application that wants
agent-composed UI would need craftkit to grow.

It is **not** a defect list in the `KNOWN-GAPS.md` sense — it does not claim
the tree is broken. `AGENTS.md` calls craftkit *"a scaffold: a working, tested
workspace meant to be copied and re-purposed into an application or driven
directly by an agent."* Nothing in the tree says craftkit is missing an
"agent-facing half"; `KNOWN-GAPS.md` lists the link, the `Either`, the plan
and the small document model, and this is not among them. The tree already
has working agent doors.

§3 does find *facts about the tree as it stands* — including **three that
belong in `KNOWN-GAPS.md`** (§3.5). They are stated as facts, and where one is
a defect rather than an opportunity, §3.5 says so.

So the honest question is not *"what is craftkit missing?"* but:

> **Which craftkit application would want an agent to choose its commands and
> pre-bind their arguments — and what would the tree have to offer it?**

That question has no answer inside this repository, and §6 says so plainly
rather than assuming one.

---

## 1. The ten a2ui patterns, stripped of their skin

| # | Pattern | Where it lives in a2ui | One line |
|---|---------|------------------------|----------|
| P1 | **The catalog** — a JSON-Schema document naming the components the agent may use, owned by the *host* | `catalogs/basic/v1/catalog.json`, `specification/v1_0/json/catalog_definition.json` | The agent's vocabulary is data, versioned, and the app's |
| P2 | **Capability negotiation** — renderer advertises `supportedCatalogIds`, agent advertises the same; the intersection is what the model is told | `specification/v1_0/json/renderer_capabilities.json`, `agent_capabilities.json` | The model is never offered a thing the host cannot show |
| P3 | **Surface + streaming protocol** — `createSurface` / `updateComponents` / `updateDataModel` / `deleteSurface` | `specification/v1_0/json/agent_to_renderer.json` | UI is a sequence of data messages |
| P4 | **Data model + bindings** — a path-addressed data model; every dynamic prop is a `DynamicValue` — literal, or a `DataBinding` (which is the path form: `{"@path": …}`), or a `FunctionCall`; writes go back to the path | `specification/v1_0/json/common_types.json` (`DynamicValue`, `DataBinding`, `ChildList`) | The UI and the state are one model, bound by path |
| P5 | **Actions** — a prop is either an `event` (to the agent) or a `functionCall` (local, emits nothing) | `common_types.json` `Action` | Interaction has exactly two shapes |
| P6 | **Resolution + placeholders** — a flat component map resolves to a live tree; an unresolved reference is a `pending` node upgraded *in place*; unknown type → `unknown-type`; cycle → `cyclic` | `blueprints/features/node_resolution.blueprint.md` | Progressive arrival is a state, not an error |
| P7 | **Inference format as a strategy** — `DirectJson`, `Express` (ANTLR grammar + compiler + decompiler + streaming parser), `Elemental`, `Atom` | `specification/inference_formats/express/Express.g4` | How you *ask* is separable from what UI *is* |
| P8 | **Catalog → skill compilation** — catalog and format rules compile into a `SKILL.md` | `blueprints/features/skill_generator.blueprint.md`, `skills/a2ui/SKILL.md` | The agent's manual is generated from the vocabulary |
| P9 | **Eval as a scoring funnel** — dataset → solver → schema validity → deterministic assertions → model-graded intent; `pass@k`; validation-error retry; streaming parse must agree with whole-text parse | `eval/DESIGN.md`, `eval/main.py` | "Does the model emit valid UI" is measured |
| P10 | **Spec-driven blueprints** — module / feature / codebase blueprints, a frontmatter validator, a compliance checker | `blueprints/**` | Contract, optional feature and implementation conformance are three documents |

---

## 2. What craftkit already has (verified)

| a2ui | craftkit | note |
|------|----------|------|
| P1 catalog | `CommandSpec` (`crates/craftkit-sdk/src/command.rs:71-137`) + `ParamDoc` (`crates/craftkit-sdk/src/error.rs:187-204`) + `Doc` (`crates/craftkit-sdk/src/params.rs:34-50`) | A typed, documented vocabulary |
| P1 (compiled) | `crates/craftkit-derive/src/command.rs:383-431` writes the spec, the reader and the registration; `commands!` synthesises the same (macro at `crates/craftkit-derive/src/lib.rs:129-131`, expander at `crates/craftkit-derive/src/commands.rs:389`) | The catalog is **compiled from types**, so it cannot drift from them — a hand-written catalog can, and does |
| P2 negotiation | `Contribution::capability()` (`crates/craftkit-sdk/src/contribution.rs:53`), `Registry::of_capability` / `of_area` (`:203-222`) | The grouping exists; the negotiation does not |
| P3 protocol | `Session::execute` (`crates/craftkit-sdk/src/session.rs:890`); six doors and two transport libraries over one registry (§3.3) | One registry, several transports |
| P4 data model | `Document` (`crates/craftkit-sdk/src/document.rs:1591-1644`) — `nodes`, `relations`, opaque `Node.data`; the **plan** (`crates/craftkit-sdk/src/plan.rs:60-69`) | A derived-value surface exists; **no path language** |
| P5 actions | `CommandSpec::run` — `CommandRun::Static` or `Dynamic` | A command *is* an action with parameters |
| P6 resolution | `Projection` (`crates/craftkit-sdk/src/ecosystem.rs:392-398`) — *"what a front end shows and what an agent is told"* | The projection is where a2ui's resolver's job would sit: the views of one registry |
| P7 formats | `craftkit-mcp`'s `schema_of` (`crates/craftkit-mcp/src/lib.rs:439-488`) emits real JSON Schema per command | One format exists |
| P8 skills | `skills/craftkit-editing/SKILL.md`, `skills/craftkit-checking/SKILL.md` — a **routing table** to `docs/recipes/` | The recipes are what can drift; the checking skill also quotes one live command id |
| P9 evals | none | gap — but see §4's **O9** (the posture decision comes first) |
| P10 blueprints | "arcs" + `cargo xtask new-arc` + `crates/craftkit-gates` | Same instinct, different vocabulary |

The load-bearing line for everything below is `ecosystem.rs:392-398`:

> **One source.** A menu, a toolbar, a schema, a chord map and a document are
> five views of the same registry, and this is where they are made.

That is a2ui's catalog, in craftkit's own words. The missing piece is that a
**catalog** is not among the five views.

---

## 3. The five findings that matter — and the three defects

### 3.1 The vocabulary has four shapes, and no canonical view

Four mechanisms describe the same commands today:

| mechanism | built from | params? | linked-in plugin commands? | run-time (subprocess) commands? |
|---|---|---|---|---|
| `xtask` index (`xtask/src/index.rs:129` `build`, `:172`, `:294`, `:468`; the record at `:32-58`) | **source text** — `commands!` tables, `#[command(…)]` attributes and macros parsed out of the files | no | **yes** — its scope *is* the plugin crates (`battery_names`, `xtask/src/index.rs:89-104`), so `hello.greet` and every other linked-in plugin command is in it | **no** — it reads source text, and a run-time command is written by hand |
| MCP `tools/list` (`crates/craftkit-mcp/src/lib.rs:303-305`; the tool JSON at `:115-123`, the schema at `:439-488`) | the registry, **taken once** at `Server::new` (`:142-154`) | yes (JSON Schema) | yes | **no** — the list is fixed at startup, and `tools/call` refuses anything outside it (`:323-325`) |
| bridge `tools/list` (`apps/craftkit-bridge/src/lib.rs:183-201`) | the **live** ecosystem projection | yes (craftkit's descriptor) | yes | **no** — it never loads one; *if* one were loaded later it would be **listed and then refused**, because the bridge calls a **snapshot** session (`:122`) |
| plugin catalogue (`crates/craftkit-sdk/src/plugin.rs:242-262`; the parameter descriptor at `:130-138`) | the plugin process | yes | — it *is* a plugin's own declaration | yes |

And the doors that show **no parameters at all**: the repl's `help`
(`apps/craftkit-repl/src/lib.rs:136-142` prints `area / id / doc`), the wasm
page (*"Type a command's id — `help` lists them"*), and control's
`engine.describe` (`crates/craftkit-control/src/lib.rs:663-684`).

The **xtask index** cannot see a run-time command at all: it reads source
text, and `KNOWN-GAPS.md` says *"A run-time command is written out by hand"*.
It **does** see the linked-in plugins' commands — its scope *is* the plugin
crates (`battery_names`, `xtask/src/index.rs:89-104`), parsed out of their
`#[command(…)]` attributes.

**The MCP door misses them too, for a different reason.** It builds its tool
list **once**, at `Server::new` (`crates/craftkit-mcp/src/lib.rs:142-154`), and
`tools/call` refuses anything outside that list (`:323-325`). Both doors take
`ecosystem.session()`, which **clones** the registries
(`crates/craftkit-sdk/src/ecosystem.rs:322-330`), while a run-time command is
registered into the **ecosystem's** registry
(`apps/craftkit-studio/src/lib.rs:62-68`).

Neither agent app ever loads a subprocess plugin — both build their ecosystem
from the inventory (`apps/craftkit-mcp-server/src/main.rs:45`,
`apps/craftkit-bridge/src/main.rs:17`) — so the run-time case is one the tree
does not currently reach. But the shape is there, and it is where O6 bites:
**if a door loaded a plugin, the MCP door would neither list the command nor
run it, and the bridge would list it and then refuse it.**

### 3.2 Six doors, and the gate knows about one

**The doors.** One list, used everywhere in this document:

| door | crate | who looks at it | transport |
|---|---|---|---|
| the terminal door | `craftkit-repl` (apps) | a person | a line of text |
| the window | `craftkit-window` (apps) | a person | a native window |
| the studio | `craftkit-studio` (apps) | a person | a native application |
| the agent door | `craftkit-mcp-server` (apps) over `craftkit-mcp` (L3 lib) | an agent | stdio JSON-RPC |
| the bridge | `craftkit-bridge` (apps) | an agent | stdio JSON-RPC |
| the browser door | `craftkit-wasm` (apps) | a person | four wasm exports over a memory boundary |

`craftkit-mcp` and `craftkit-control` are not doors — they are the transports
two of the doors use. `craftkit-control` has a row of its own in §3.3;
`craftkit-mcp` appears inside the agent door's row, because it is what that
door runs.

**The gate knows about one door.**

```rust
pub const FRONT_ENDS: &[&str] = &["craftkit-window"];   // crates/craftkit-gates/src/lib.rs:274
```

The rule "a front end is thin" — one of `AGENTS.md`'s non-negotiables — reaches
**one** of the six. The other five are not classified, so the rule does not
apply to them.

**And widening the list is not the one-liner it looks like.** It is one line
*of const*, and then:

- **`craftkit-bridge` fails immediately.** `apps/craftkit-bridge/src/lib.rs:231`
  reads `self.ecosystem.evaluators` in code, and `evaluators` is in
  `FRONT_END_FORBIDDEN` (`:295`). The rule reads **code**, not strings and not
  comments (`:292-294`), so a JSON key built from a field access is a hit.
- **`craftkit-studio` fails harder, and two of its hits are real defects.**
  `apps/craftkit-studio/src/main.rs:170` calls `session.undo()` directly —
  the gate's own words: *"`undo` and `redo` are the history's own doors … and
  **everything** is a command"* (`:285-287`). And
  `apps/craftkit-studio/src/main.rs:36` reads `ecosystem.evaluators.ids()` —
  *"`evaluators` is the evaluator's own registry: a front end that reaches it
  is a front end that evaluates the document itself"* (`:280-281`). The
  studio's other 24 hits are `document_mut` inside command **implementations**
  (`apps/craftkit-studio/src/commands.rs`) — legitimate under the house rules,
  but indistinguishable to a crate-wide text rule, because the gate scans
  `src/*.rs` and does not parse.

  **Four findings in all, not twenty-six.** The gate emits one finding per
  *(file, name)* pair (`crates/craftkit-gates/src/lib.rs:636-651`), so the
  studio yields three (`commands.rs`/`document_mut`, `main.rs`/`evaluators`,
  `main.rs`/`undo`) and the bridge one (`lib.rs`/`evaluators`). The 24 is the
  number of `document_mut` **occurrences**.
- **`craftkit-repl`, `craftkit-mcp-server`, `craftkit-wasm`,
  `craftkit-window`, `craftkit-mcp` and `craftkit-control` are clean** under
  the rule as written.

So the honest statement is not "one line":

> **One line, plus a decision on which six, plus fixing `craftkit-bridge`'s
> `evaluators`, plus — if the studio is in — either scoping the rule so a
> command's own implementation is exempt, or rewriting the studio's
> twenty-four `document_mut` calls.**

**Which of these are defects today?** The studio's two are, and they are in
§3.5. The **bridge's `evaluators` is a violation only once the bridge is
classified as a front end** — which is the very decision this section is
about — so it is a *cost of widening the list*, not a defect that exists
today. The two studio ones belong in `KNOWN-GAPS.md` now; all three do once
the classification is settled.

**Since this was written.** The six doors are **classified** (README §1116):
the gate's `FRONT_ENDS` names them all, the **transports** (`craftkit-mcp`,
`craftkit-control`) are not doors, and the **studio** is a **host** whose
entry point is the front end (`FRONT_END_FILES`). The bridge's two doors are
gone — the plan is the `doc.plan` command, and the evaluator's ids are the
**manifest**'s — and the rule names `evaluate` as well. The **parity matrix**
is written too ([`door-parity.md`](door-parity.md)): the six doors side by
side, checked by hand.

### 3.3 The mid-run cycle exists in two transports, and the `Supplier` seam in none

The SDK has a complete, deliberately agent-shaped prompt seam:

- `crates/craftkit-sdk/src/prompt.rs` — *"a `Supplier`: where the answers come
  from — a person, **an agent**, a script, a file"* (`:26`), plus `MapSupplier`
  (`:79`) and the `Supplier` trait (`:67`);
- `Session::execute_with` (`session.rs:766`) — *"run a command, and ask the
  host for the parameters that are missing before it runs"*
  (`crates/craftkit-sdk/src/prompt.rs:28-29`);
- `Session::ask` / `answer` / `pending` / `cancel` (`session.rs:823`, `:852`,
  `:842`, `:885`) — a command that asks mid-run is **stopped** with the
  document restored, and re-runs when answered.

But **no door's runtime code calls `execute_with`**, and the mid-run answer
cycle is implemented in **two** transport libraries:

| door | `execute` | mid-run answer cycle | `Supplier` |
|---|---|---|---|
| terminal (`craftkit-repl`) | ✔ (`:236`) | ✘ | ✘ |
| window (`craftkit-window`) | ✔ (`:286`) | ✘ in the window itself — but it runs a `craftkit-control` server (`apps/craftkit-window/src/main.rs:121-132`), which does implement it | ✘ |
| studio (`craftkit-studio`) | ✔ | ✘ | ✘ |
| agent door (`craftkit-mcp-server` → `craftkit-mcp`) | ✔ (`craftkit-mcp/src/lib.rs:339`) | ✔ (`:326-337`, calls `session.answer`) | ✘ |
| bridge (`craftkit-bridge`) | ✔ (`:211`) | **✘** | ✘ |
| browser (`craftkit-wasm`, which runs the repl's logic) | ✔ | ✘ | ✘ |
| — control transport (`craftkit-control`) | ✔ (`:709`) | ✔ (`:702-710`) | ✘ |

`craftkit-studio`'s own library says it plainly: it *"calls
`Session::execute` and nothing else"*
(`apps/craftkit-studio/src/lib.rs:21`) — but the studio already has a
**tested** `Supplier` implementation: `apps/craftkit-studio/tests/prompt.rs:31`
defines one and calls `execute_with` six times (`:45`–`:116`). The seam is
understood and proven in that tree; it is only unused at runtime.

a2ui's equivalent is the `renderer_to_agent` function-call message and its
`agentFunctionResponse` reply — a bidirectional call **in the protocol**, not a
host-side convention.

### 3.4 The plan has no address, and a front end must not resolve it

The plan (`crates/craftkit-sdk/src/plan.rs:60-69`) is `{items, failures}`, and
each item is `{node, kind, value}` — a derived-value surface with a natural
address. But:

- there is **no path language for addressing a value** anywhere in the SDK
  (`path` appears as an *asset* path and as a *menu* path —
  `CommandSpec::menu`, `crates/craftkit-sdk/src/command.rs:90-97` — but neither
  addresses a value);
- the gate forbids a front end's **code** from naming the plan's types:

```rust
pub const FRONT_END_FORBIDDEN: &[&str] =
    &["document_mut", "evaluators", "Plan", "PlanItem", "undo", "redo"];
```

*"a front end that holds one is a front end that keeps a value the evaluator
computes"* (`crates/craftkit-gates/src/lib.rs:282-284`) — but the same doc
comment is careful about what that means:

> *"The plan itself is not forbidden: the session hands it out, and a front
> end may read it. What it may not do is **keep** it."*
> (`crates/craftkit-gates/src/lib.rs:289-290`)

That is the shape of the constraint, and it is sharper than "never expose a
`Plan`". The plan reaches a front end as **JSON** — through the `doc.plan`
command (`crates/craftkit-sdk/src/plan.rs:162`), whose outcome carries
`Plan::to_json` (`plan.rs:78`) — never as a `Plan` value the front end can
name. And the front end may **read** it but must not **keep** it: a derived
value is derived, and a stored one goes stale.

So a binding a panel draws must be **re-read** when it is drawn, must arrive
as plain JSON, and must not be cached by the front end. The SDK-side
resolution produces the value; the front end displays it and drops it.

**The rule is about front ends, not about the SDK.** What the gate forbids is
a *front end* naming a `Plan`; a **library** is not a front end, and O11's
compat layer resolves its own tree's paths outside the SDK without breaking
anything. What nothing may do is hand a front end a `Plan` — which is the
rule this section states, and the rule O11 keeps.

**And the tree's own window already breaks that rule** — see §3.5. The gate
misses it because `FRONT_END_FORBIDDEN` names `evaluators`, `Plan` and
`PlanItem`, while the window uses `evaluate()` and `PlanRow`.

### 3.5 Three defects found along the way

These are not opportunities. They are the three house-rule violations the
tree has today, and **§6.1** is where they are actionable — the file, the
rule it breaks and how you know it is done.

They are house-rule violations that exist **today**,
independent of anything a2ui has to do with it. Each belongs in
`KNOWN-GAPS.md`, and each is a *defect* rather than a feature — which is a
different kind of work, and one nobody has to approve.

| # | where | what it does | the rule it breaks |
|---|---|---|---|
| 1 | `apps/craftkit-studio/src/main.rs:170` | `match session.undo()` | *"…`undo` and `redo` are the history's own doors … and **everything** is a command"* (`crates/craftkit-gates/src/lib.rs:285-287`) |
| 2 | `apps/craftkit-studio/src/main.rs:36` | `ecosystem.evaluators.ids()` | *"`evaluators` is the evaluator's own registry: a front end that reaches it is a front end that evaluates the document itself"* (`:280-281`) |
| 3 | `apps/craftkit-window/src/lib.rs:201-209` | `pub fn plan(&self) -> Vec<PlanRow>` calls `self.session().evaluate()` and **keeps** the derived values | *"A front end … never calls the evaluator, never keeps a value the evaluator computes"* (`AGENTS.md`) |

Number 3 is the sharpest, because `craftkit-window` is the **one** crate the
gate already trusts (`FRONT_ENDS`, `:274`) — and the gate misses it: the
forbidden list names `evaluators`, `Plan` and `PlanItem`, while the window
writes `evaluate()` and `PlanRow`. The gate reads **names**, and a name can
be avoided without the behaviour being avoided. That is a limit of the rule
as written, not a gap in the report.

**All three are repaired** (README §1115): the studio undoes through
`history.undo`, the studio's evaluators line is gone, and the window has
no rows of its own — it asks the `doc.plan` command. The gate now reads
the studio's **entry point** (`FRONT_END_FILES`) and names `evaluate` as
well. What is left of this section is the **bridge**, which is not
classified yet (`KNOWN-GAPS.md` §4).

### 3.6 The renderable catalogue — the seam that plays a2ui's catalog

There is a **second catalogue** in the tree, and it is the one a2ui's catalog
actually resembles. It is not mentioned anywhere above, because the search
started at the registry.

**A kind is a catalogue entry that knows how to draw itself.**

- `Ecosystem.kinds` is a `Registry<NodeKindSpec>`
  (`crates/craftkit-sdk/src/ecosystem.rs:116`).
- `NodeKindSpec` (`crates/craftkit-sdk/src/document.rs:2409-2440`) carries
  `kind`, `label`, `layer`, a `paint` function, a `hit` function, and what
  the node *means* in the plan.
- `#[derive(NodeKind)]` (`crates/craftkit-derive/src/lib.rs:141`) writes the
  spec and submits it to the **link-time inventory**
  (`crates/craftkit-derive/src/seams.rs:200-215`), so a kind joins the
  registry the moment its crate is linked in — `Registry::from_inventory()`
  (`crates/craftkit-sdk/src/contribution.rs:235-244`).
- **The tree has 99 kinds** across 17 families: `sound.*` (16), `art.*` (15),
  `layout.*` (13), `cad.*` (11), `prose.*` (10), `film.*` (8), `core.*`,
  `fx.*`, `grid.*` (4 each), `numeric.*`, `pdf.*`, `text.*` (3 each), and one
  each for `geometry.rect`, `vector.path`, `image.picture`, `clap.plugin` and
  `vst3.plugin`. (Counted as **distinct kind names in the shipped crates**:
  every `#[node_kind(…)]` attribute under `crates/` — **76** distinct,
  because `geometry.rect` is declared three times (twice in the derive
  crate's doc and test, once for real) — **plus** every invocation of the
  `art_kind!`, `track_kind!`, `entity_kind!`, `item_kind!` and `block_kind!`
  macro shims, which expand to the same attribute and are **23** more. A
  grep of the literal attributes alone finds 76 and **undercounts by 23**.
  One further kind exists only in an integration test (`test.panic.kind`,
  `apps/craftkit-studio/tests/guard.rs:14`) and is not shipped.)
- `Projection::manifest()` already publishes them: `"node kind {kind} in
  {layer} — {label}"` (`crates/craftkit-sdk/src/ecosystem.rs:571-573`).

**The painting seam.** `crates/craftkit-raster/src/lib.rs:5`: *"The kit has a
**painting seam** — every kind of node knows how it is painted, and the core
never matches on a kind."* `Session::paint` walks the visible nodes and calls
`(kind.paint)(node, document, canvas)` for each
(`crates/craftkit-sdk/src/session.rs:394-399`); `paint_view` adds the
selection's chrome (`:409-412`).

**That is a2ui's catalog role, exactly.** a2ui's catalog is a closed,
host-owned set of renderable entries with a name and a label, that a
renderer maps onto real widgets. craftkit's kind registry is a closed,
host-owned set of renderable entries with a name and a label, that a canvas
maps onto real drawing. The difference is only **who chooses**: a2ui's model
chooses the entry; craftkit's document contains it, and the model chooses
*commands*.

**The canvases — the renderers.** The `Canvas` trait
(`crates/craftkit-sdk/src/document.rs:776-801`) has four methods: `rect`,
`text`, `path`, `picture`. Two implementors exist:

| backend | what it is | where |
|---|---|---|
| `RecordingCanvas` | writes down what it was asked to draw — the spike's backend, enough to *test* a kind and not enough to *see* one | `crates/craftkit-sdk/src/document.rs:813` |
| `Raster` | RGBA pixels with a background; the camera's zoom and offset applied; a PNG writer over it | `crates/craftkit-raster/src/raster.rs:282` |

The window is a `Raster`: `session.paint_view(&mut raster)`
(`apps/craftkit-window/src/lib.rs:462-467`, `:512-513`).

**And two things the render story does not have.**

1. **The browser door cannot render.** `apps/craftkit-wasm` runs the terminal
   door's logic and prints text; there is no canvas behind it. An
   agent-composed surface in a browser would need one first.
2. **The PDF writer is off the seam.** `crates/craftkit-pdf-writer` has its
   own `Page`, `Rect`, `Point` and `Font`, and **no mention of `Canvas`
   anywhere in the crate** — yet the seam's own doc comment says *"A backend
   that draws a document may be a raster, a PDF writer or a recorder"*
   (`crates/craftkit-sdk/src/document.rs:794-799`). The comment describes a
   third implementor that does not exist.

**What this changes.** §4's O7 says *"craftkit has no widgets"*, and for
**UI** widgets that is true — there is no `Button` and no `TextField`. But
craftkit has a **catalogue of 99 renderable kinds** and a **single seam**
they all paint through, and that is the a2ui-shaped thing that already
exists. O11 is built on it.

---

## 4. What a2ui would give craftkit — scoped correctly

### O1 · A catalog in the SDK (L2)

**Opportunity.** One canonical, serialisable view of the registry: every
command, tool, panel and kind, with parameters as JSON Schema, plus
`capability`, `area`, `mutates`, `undoable`. Every door serves *that*.

**Three of the four mechanisms in §3.1 collapse into one** — the MCP
`tools/list`, the bridge `tools/list` and the plugin catalogue line. The
`xtask` index does **not** (see below), so it stays a second reader of the
same truth.

**The shape, so it is not a design decision left to the reader:**

```rust
// crates/craftkit-sdk/src/ecosystem.rs, beside Projection::schema
pub fn catalog(&self) -> serde_json::Value
```

returning one document:

```json
{ "commands": [
    { "id": "…", "area": "…", "label": "…", "capability": "…",
      "mutates": true, "undoable": false,
      "params": [ { "name": "…", "required": true, "schema": { … } } ] }
  ],
  "tools":  [ { "id": "…", "label": "…", "icon": "…",
                "shortcut": null, "command": "…" } ],
  "panels": [ { "id": "…", "label": "…", "commands": [ "…" ] } ],
  "kinds":  [ { "kind": "…", "label": "…" } ] }
```

**The tools and the panels are in it because craftkit's UI is made of them.**
A `ToolSpec` is *a gesture that runs a command* and a `PanelSpec` is *a
window's worth of commands* (`crates/craftkit-sdk/src/seams.rs:28-41`,
`:64-72`) — they are the **verbs**, and the commands are what the verbs run.
A view that listed the commands but not the gestures that run them would be
half the vocabulary, and the half a renderer needs.

**And the a2ui components are in it — as kinds.** The catalogue a2ui's agent
generates against is a **component** catalogue, and craftkit's components are
the **kinds** — so the kinds are in this view, and craftkit's *a2ui* components
are **eleven of those kinds** (`a2ui.text` and so on). So they appear in this
view **as kinds**, with a label and no schema.

What is separate is the **a2ui catalogue**: the a2ui-**shaped** projection of
the catalogue entries, with each component's **published** schema — a
projection that lives in the a2ui layer, not in the SDK (see
[`a2ui-compat-layer.md`](a2ui-compat-layer.md) §2.6). The two views are **not
disjoint**: the overlap is exactly the eleven a2ui kinds, and they differ in
**shape** and **reader**, not in membership. Two projections, and neither is
written by hand.

**No `enabled` field — and that is not an oversight.** `CommandSpec::enabled`
is `fn(&Session) -> bool` (`crates/craftkit-sdk/src/command.rs:71-137`),
evaluated against a live session (`crates/craftkit-sdk/src/session.rs:903`),
and a `Projection` holds only the ecosystem — there is no session in scope. So
availability cannot be a field of a registry-level view at all.
`Projection::manifest()` omits it for exactly this reason
(`crates/craftkit-sdk/src/ecosystem.rs:563-589`). **"Nothing" is not a
zero:** a command that is unavailable in *this* session is not a command with
`enabled: false`; it is a command whose availability the **session** answers
when the command is run.

**First consumer:** `crates/craftkit-mcp/src/lib.rs:439-488`, whose
`schema_of` already builds the per-command JSON Schema — the catalog
generalises it rather than replacing it.

**It should carry the kinds too.** `Projection::manifest()` already
publishes the kind catalogue alongside the commands
(`crates/craftkit-sdk/src/ecosystem.rs:571-573`), and the catalog's shape
above already has a `"kinds"` array. A view that lists commands but not
kinds is half the vocabulary — and the kinds are the half a renderer needs
(§3.6).

**Landing site.** Two options, and the choice is a constraint rather than a
preference.

*In the SDK* (L2), beside `Projection::schema`
(`crates/craftkit-sdk/src/ecosystem.rs:525-558`). Layering-clean —
`craftkit-mcp` and `craftkit-control` are L3
(`crates/craftkit-gates/src/lib.rs:219-220`) and may depend on L2 — and best
for "one source", since the projection is already where the views are made.
The cost is that the SDK's plugin-facing surface grows a method plugins do
not need; it is a *view*, not a capability, so nothing is exposed that was
not already there.

*A new L3 crate* (`craftkit-catalog`). **This does not work as first
written.** The catalog crate would depend on the SDK (L2) and so must itself
be L3+, but `craftkit-mcp` and `craftkit-control` are already L3 and may
depend only on **strictly lower** layers (`AGENTS.md`; enforced at
`crates/craftkit-gates/src/lib.rs:466`). An L3 crate depending on an L3 crate
fails the inner gate. This option therefore requires **promoting
`craftkit-mcp` and `craftkit-control` to L4** — a change to the layer table,
which the outer gate checks against the manifests.

So: **the SDK**, unless the tree wants the layer table moved.

**And `xtask` stays out of it.** The `xtask` index is a *source-text* reader,
and the tree keeps it that way deliberately: *"It reads the tree as text,
because a checker that cannot be built when the tree does not compile is a
checker that is silent exactly when it is needed"* (`KNOWN-GAPS.md`). Pulling
`xtask` into `craftkit-catalog` would give the outer gate a dependency on the
tree's own code — exactly what it avoids. The `xtask` index and the runtime
catalog stay **two readers of one truth**, and the gap between them is worth a
gate, not a merge.

**Cost.** Small in the SDK: one method, three call sites (`craftkit-mcp`,
`apps/craftkit-bridge`, and the plugin catalogue in
`crates/craftkit-sdk/src/plugin.rs:130-262`). Larger in a new crate: a
layer-table change as well.

**Acceptance test.** `cargo test -p craftkit-gates` — the inner gate — plus a
test asserting that **both doors' `tools/list` are generated from the one
`Projection::catalog()`**, rather than each built by hand. Not that they
carry the same *set*: they do not today, and should not. The MCP server
builds its tools from `session.registry().all()` — commands only
(`crates/craftkit-mcp/src/lib.rs:142-154`) — while the bridge builds its list
from `projection.menu(&["command", "effect", "tool"])` — commands, effects and
tools (`apps/craftkit-bridge/src/lib.rs:183-201`). O1 makes the **shape**
uniform; O2 then prunes each door's contents **on purpose**, and the parity
that must hold is behavioural, not enumerative (O4.2). (Adding a method to the
SDK also means `cargo xtask ci`, because the SDK is depended on by
everything.)

**Note.** a2ui's catalog is hand-written JSON Schema, versioned by hand —
the catalog versions under `catalogs/` are `v0_9`, `v0_9_1` and `v1`, the
specification carries `v0_8` … `v1_0`, and the manual for changing a catalog
is `specification/v0_8/docs/custom_catalog_changes.md`. craftkit's is
**compiled from the types**. Take
the *role* of the catalog; keep craftkit's mechanism. Do not reverse it.

### O2 · Prune the catalog per door

**Opportunity.** A door advertises what it can show (capabilities, areas), and
the catalog it serves is pruned to that.

**Honest size.** a2ui's negotiation works by **advertisement**: the renderer and the agent
each publish a list of catalog ids, and the *effective* catalog is the
intersection of the two — the agent is prompted with the components both
sides named. (The schemas advertise; nothing in the protocol rejects a
component id outside the intersection at the resolver level, whose documented
fallback for an unknown type is the `unknown-type` placeholder.) craftkit's
capabilities are groups of *verbs* and **every door can render every
command** — the window draws
commands by area, the bridge lists them all. So there is no intersection with
teeth.

This is therefore **prompt hygiene and relevance**, not a trust boundary: it
shrinks the prompt and stops offering the user commands they cannot reach.
Worth doing; smaller than the analogy suggests.

**What it prunes.** The **agent-facing** view only: the catalog an agent door
serves. It does not change what a door can enumerate or run — the registry
stays whole, and every door may still reach every command.

**The pruned sets are expected to differ.** The MCP and bridge lists differ
already (commands only, versus commands + effects + tools), and O2 makes that
difference deliberate rather than accidental. The parity that must hold is
**behavioural** — the same refusals, the same rollback — and that is O4.2's
job, not O1's.

**Cost.** Small — `Registry::of_capability` / `of_area` already exist.

### O3 · Check the recipes, not the skills

**Opportunity.** The *editing* skill names **no command id** — it routes to
five recipes in `docs/recipes/`. The *checking* skill quotes one:
`hello.greet`, twice, as a worked example
(`skills/craftkit-checking/SKILL.md:115`, `:123`) — a command that belongs to
a plugin and could be renamed. What can drift is therefore the **recipes**
(the paths they name, the conventions they state) **and the quoted ids**.

**The link half is already done.** `xtask`'s prose check already covers
`docs` (`PROSE` at `xtask/src/lists.rs:55`), walks it recursively, and
fails on a broken link — so "the paths they name" is checked today. What is
**not** checked is **the claims**: that a recipe still says what the tree
does. That is the new work, and it is a judgement rather than a link, so it
needs a stated rule, not a regex.

So the useful check is not "generate a command list" but **"every path and
claim a recipe makes still holds"** — a link-and-claim check over
`docs/recipes/*.md`, in the spirit of the outer gate's existing
"every document link in the prose points at a file that is in the tree".

**Cost.** Small, and it fits the existing `xtask` rather than adding a
concept — but only the *claims* half is new.

### O4 · A door-parity matrix, and the classification that makes it possible

**Opportunity, in two steps.**

1. **Classify the doors.** Widen `FRONT_ENDS` — but see §3.2: it is *not*
   one line. It is one line, plus fixing `craftkit-bridge`'s `evaluators`,
   plus deciding what to do about the studio's twenty-four `document_mut`
   calls inside command implementations. The likely shape is a rule that
   exempts a command's **own implementation** — which means changing the
   gate, not just the const.
2. **Assert parity.** A parity matrix **built on** the door table in §3.2 —
   every door enumerates the registry, refuses an unknown command the same
   way, rejects an unknown parameter the same way, rolls back a
   `mutates = false` command that changed the document, and handles a
   mid-run question the same way. **That matrix does not exist yet**: §3.3's
   table records only which doors have which mechanism, and the parity
   matrix is the thing to be written.

Step 2 is a test harness across **the six doors and the two transport
libraries they use** — a line of text, a native window, a native
application, two stdio JSON-RPC doors, a TCP JSON-lines transport, and four
wasm exports over a memory boundary. That is a **project**, not a task, and
the honest first cut is the matrix itself, written as a table and checked by
hand, before it becomes code.

**Why it matters.** The tree's own rule says a front end reads the engine's
state and acts through the registry. A matrix is how that rule stops being a
text grep.

### O5 · The model picks commands and pre-binds their arguments

**The corrected opportunity.** Not "the agent lays out a UI tree". craftkit's
UI vocabulary is `ToolSpec` (*a gesture that runs a command*) and `PanelSpec`
(*a window's worth of commands*) — `crates/craftkit-sdk/src/seams.rs:30`,
`:66`. These are **verbs**. There is no catalogue of visual components to
compose from, and the window already draws commands by area and panels by
battery. **A verb list is not a layout**, and a2ui's value — the model
choosing *widgets* — does not transfer.

What does transfer is narrow and real:

1. the model **selects commands** from the registry, in an order, with
   labels — which is `Projection::menu_tree` (`ecosystem.rs:464`) with a
   different author;
2. the model **pre-binds arguments** — which is `prompt::Supplier`, already
   written and already documented as an agent affordance.

So the landing is: a `Supplier` implementation over the model's own output,
a door that calls `execute_with` instead of `execute`, and the mid-run cycle
that exists in **two** transport libraries (`craftkit-mcp` and
`craftkit-control`); of the six doors, only the **agent door** uses it
directly — the window reaches it only through the `craftkit-control` server
it runs, and does not use the cycle in its own code (§3.3).

**This is not a new concept.** It is an existing seam used by more doors. And
the seam is where the tree already put it: `prompt.rs` says the prompt is *"a
**host** concern, not a command concern"* — so a `Supplier` belongs in the
host, and the house rule that forbids behaviour in a front end does not
forbid it here.

**Cost.** Small, if scoped this way. Large, if scoped as a "surface".

### O6 · A registry that can be reconciled

`Registry<T>` (`crates/craftkit-sdk/src/contribution.rs:139-222`) can
`register` and read; it cannot remove or replace. Commands can be *added at
run time* (`CommandRun::Dynamic`, from a subprocess plugin —
`crates/craftkit-sdk/src/plugin.rs:414-529`), and a plugin loads **after** a
front end has already rendered.

a2ui's P6 answers the mirror-image problem (a component arrives late) with
the placeholder-upgrade rule. craftkit's version:

> A panel holds **command ids**. When the registry changes, the panel is
> reconciled against it, and it is **not** rebuilt.
>
> *(This is the rule as a2ui states it; the direction that can actually happen
> in craftkit is narrower — see below.)*

**The sharpest case for it.** A run-time command is **unusable** from the two
agent doors (§3.1): the MCP door neither lists nor runs it, and the bridge
lists it and then refuses it when called. A catalog built from the **live**
ecosystem — rather than from a session's snapshot — would be the first view
that both **sees and can run** a command registered after start-up; the
bridge's live listing already sees one, but refuses it. That is the whole of
O6's argument, and it does not depend on "disappeared" ever being possible.

**The direction that exists.** `Registry<T>` is grow-only, so the only
direction that can happen today is **"appeared"**. There is no "disappeared":
a command cannot be removed from the registry, so the reconciliation case
reduces to *a command that was not there when the panel was built is now
there*. And if a panel holds an id that the registry does not have, the
house rule applies — *"Unknown names are refused"* — so a stale id is an
**error**, not something to drop silently.

**What this is not.** It is not "the natural v3 of the tree's thesis". The
thesis (`AGENTS.md:14`) is *"the registry is a **value**, and any crate linked
in can put a command in it"* — "value" there means *a value you can hold and
inspect*, not *immutable*. A grow-only `Vec` is an implementation of that
thesis, not a violation of it. What the panel needs is a **version counter
and a reconciliation step**, which is a smaller and more honest claim.

**Cost.** Medium.

### O7 · Paths that resolve to plain values, inside the SDK

**Opportunity.** Give the derived-value surface an address — the plan's
`{node, kind}` shape is the natural one — and let a panel bind to it.

**The transfer test, stated once.** a2ui's *path/binding* half transfers; its
*component tree* half does not (see O5). The reason is specific: a path
addresses a **value**, and craftkit already has values with an address (the
plan's `{node, kind}`). A component tree addresses a **widget**, and craftkit
has no **UI** widgets — no `Button`, no `TextField`. What craftkit has is a
catalogue of **99 renderable kinds** (§3.6), and a model that chooses
*commands* rather than kinds. The test applied throughout this document is:
*does the thing being borrowed address a value, or compose widgets?* — and
where the answer is "compose widgets", the tree's own answer is the kind
catalogue and the painting seam, not a new vocabulary.

**The constraint that shapes it (§3.4).** A front end may *read* the plan
and must not *keep* it. So the resolution happens **in the SDK** and arrives
as plain JSON: a path resolves to a value, exactly as `Node.data` is a plain
value the core does not interpret. The front end displays the value and drops
it; it re-reads rather than caching, because a kept derived value is a stale
one.

**Cost.** Large, and it touches the SDK's core data model. Its own arc.

### O8 · Declare the bounds that are already enforced

`ParamDoc` carries `name`, `kind`, `required`, `help`, `alternative`,
`type_name`, `unit` — but **no default and no range**, even though the SDK
knows both: `Params`' helpers bound text (`MAX_TEXT_CHARACTERS`), JSON depth
and node count, and `NodeRef` is a `u64`.

Putting the bounds **in the catalog** means the model is **told** the bound
before it proposes a value, rather than being refused after — which is the
whole difference, and which is not the same as preventing it.

**The catalog derives them; it never restates them.** The numbers already
live in the SDK's constants. A catalog that *declared* its own copy would be
a second place they live — against the "one source" discipline. So the
catalog **reads** the constants and emits them.

**But the one-home rule is a convention here, not something a gate holds.**
The inner gate's prose check (`crates/craftkit-gates/src/lib.rs:816-826`)
collects only `pub const MAX_*` whose name ends in `BYTES`, `MEMORY` or
`STEPS` (`public_bounds`, `:851-867`) — and the tree has exactly three of
those: `MAX_DOCUMENT_BYTES`, `MAX_SESSION_BYTES`, `MAX_HISTORY_BYTES`.
`MAX_TEXT_CHARACTERS`, `MAX_JSON_NODES`, `MAX_JSON_DEPTH` and `MAX_NODES` are
**not** covered by it. So for the bounds O8 is about, "one home" is a
convention the review upholds, not a rule a gate enforces — which is worth
knowing before the catalog starts depending on it.

**Honest scope — smaller than it sounds.** The SDK knows **ranges** but knows
**no defaults at all**: there is no default field on `ParamDoc` or on
`CommandSpec`, and no default mechanism in the derive's command path. And of
the ranges it knows, only one is usable:

| kind | bound the SDK knows | already in the schema? |
|---|---|---|
| `Doc::Text` | `MAX_TEXT_CHARACTERS` = 65 536 (`params.rs:206`) | no — **new** |
| `Doc::NodeRef` | a `u64` | yes — `"minimum": 0` (`craftkit-mcp/src/lib.rs:447`) |
| `Doc::Number` | none | no |
| `Doc::Bool`, `Doc::NodeKind` | none | no |
| `Doc::Json` | `MAX_JSON_NODES` = 1 000 000, `MAX_JSON_DEPTH` = 64 (`params.rs:293`, `:296`) | bounded by shape, not meaning — nothing an author can act on |

(The document's own node ceiling is a different and much smaller bound —
`MAX_NODES` is 10 000, `crates/craftkit-sdk/src/document.rs:1310`.)

So O8, as it survives, is **one `maxLength` in one schema**. That is worth
doing and it is not an opportunity in its own right.

**It is done** (README §1117). The SDK's one generator reads
`MAX_TEXT_CHARACTERS` and emits it as `maxLength` — the number is read, not
copied, so the schema and the reader cannot disagree.

**Cost.** Small, and it rides on O1.

### O9 · Evals — a dependency decision first, not an arc

a2ui P9 is the most *reusable* idea in that repo, and craftkit has nothing
comparable: no measurement of whether a model emits *correct* craftkit
commands; the first evidence today is a runtime refusal.

The design maps almost perfectly, with one substitution that makes it better:

> **The scorer must be the engine.** a2ui's stage 1 is "did the payload
> validate against the schema". craftkit's stage 1 should be *"did
> `Session::execute` accept it"* — using craftkit's own `Params` reader,
> `deny_unknown` and bounds. Then the eval cannot drift from the engine,
> because it **is** the engine.

**But it is not an arc in this tree yet, and the wall is the tree's posture,
not its policy.** The workspace has **no network dependencies** — no
`reqwest`, `hyper`, `tokio`, `ureq` or `async-std` in any manifest — and
**nothing outbound**: the only sockets are the control channel's own, on
loopback (`TcpListener::bind` at `crates/craftkit-control/src/lib.rs:441`,
`TcpStream::connect` at `:492` and `:793`). The tree has never spoken to a
host it does not own. `docs/dependency-policy.md` is *permissive* about a
provider SDK (only strong copyleft is refused; crates.io is required;
advisories must be named), so the policy is not the obstacle.

The obstacles are two: the tree's **no-outbound-network posture**, and the
absence of a secret-management story for a2ui's **encrypted datasets**
(Transcrypt, against base-model contamination).

So: a **posture decision first**, then a design. It belongs in CI on
GitHub's runners (`AGENTS.md`: heavy work belongs there), but the posture is
the wall, not the cost.

### O10 · Contracts — only if they replace prose

a2ui P10 keeps three documents apart: the module blueprint (stable
interface), the feature blueprint (optional, dated), the codebase blueprint
(this repo's compliance, pinned to a commit hash). craftkit keeps one — a
3.4 MB README that is a build log.

The opportunity is to extract the **stable contracts** (the shape of
`CommandSpec`, the session's guarantees, the layering) into `docs/contracts/`.

**The constraint.** A second document holding the same facts is a **second
source** — against the tree's own discipline (*"One source … five views of the
same registry"*, `ecosystem.rs:392-398`) and the outer gate's existing check
that prose links resolve. Extraction must **replace** prose, not sit beside
it.

**And one thing may not move.** The inner gate reads the bounds out of
`root/README.md` (`crates/craftkit-gates/src/lib.rs:820`) and fails with "a
bound no document names" if a number is not there. So the README must keep
the bounds, or the gate must be widened to read the new document — which is
itself a change to the gate. The extraction is therefore **not** free, and it
is not a pure move.

**Cost.** High, and documentation-shaped. Do it when the contracts have
stopped moving.

### O11 · An a2ui-compatible surface, as a layer that adds nothing

**The question.** Can craftkit accept an a2ui surface — the protocol, the
component catalogue, the data model, the bindings — **without** limiting
craftkit's own UI capabilities, or the rendering libraries underneath it?

**The answer is yes, and the shape is fixed by two facts already in the
tree.**

**Fact one: a kind is added by linking a crate in.** `#[derive(NodeKind)]`
writes the spec and submits it to the link-time inventory
(`crates/craftkit-derive/src/seams.rs:200-215`). Nothing in the core, the
rasteriser, the window or the SDK changes when a kind is added — so a compat
layer can contribute kinds **as a plugin would**, which is the tree's own
answer to "a new thing that renders".

**Fact two: there is one seam, not six.** a2ui has six renderers — React,
Lit, Angular, markdown-it, Flutter and SwiftUI — and **four shared
catalogues** (`basic`, `mcp`, `minimal`, `vegachart`), of which the basic
one is *"shared by renderers and `@a2ui/agent`"* (`a2ui/README.md:131`). Its
negotiation is over which **sets** of catalogue ids each side supports
(`supportedCatalogIds`), not one catalogue per renderer.

craftkit has **one** `Canvas` trait and **one** kind registry, and every
backend draws every kind — so the effective catalogue is the whole of it,
always. craftkit already has a2ui's answer to "the sides support different
things", in the form of one seam (`AGENTS.md`: *"One set of camera
controls"*). **The compat layer does not need a catalogue per renderer,
because craftkit does not have per-renderer vocabularies.**

**So the layer is one crate, at L4, and it is one-way.**

*(**Superseded in scope** by [`a2ui-compat-layer.md`](a2ui-compat-layer.md),
which is the implementation plan and is the one that is true: three crates
rather than one, eleven components rather than six, a new derive macro, and
changes to the SDK. The argument below — that the layer adds nothing and
limits nothing — is unchanged and is the reason the plan is worth doing.)*

| a2ui | the compat layer | craftkit |
|---|---|---|
| the protocol's messages (`createSurface`, `updateComponents`, `updateDataModel`, `deleteSurface`, `callRendererFunction`, `agentFunctionResponse`) | one door that speaks them | — |
| the component catalogue | a **catalogue generated from the compat layer's own kinds** — `a2ui.text`, `a2ui.column`, `a2ui.button`, `a2ui.textfield`, `a2ui.image` | each is a kind with a painter, like every other kind |
| the data model and its bindings | a small JSON tree **inside the layer**, with the paths resolved by the layer — the truth for an a2ui binding | values are **mirrored** into craftkit by issuing **commands**, never written |
| an action (`event` / `functionCall`) | a **`functionCall` is resolved local-first** (§5.1 of the plan) and a name in the layer's function table is a craftkit **command id**; an `event` is forwarded to the agent | `Session::execute` — the same door, the same refusals |
| a renderer adapter | nothing: the compat kinds paint themselves | the ordinary `Canvas` seam |

**Why this limits nothing.**

- **The 99 kinds stay.** The compat layer *adds* eleven components; it
  replaces none. `layout.spread` and `sound.track` are still there, still
  painted by their own painters.
- **The renderers are untouched.** The compat kinds paint through the
  ordinary `Canvas` seam, so the raster, the window and any future backend
  draw them **with no change at all**. Nothing goes near the rasteriser, the
  camera, the font metrics or the layout engine.
- **The data model is kept in step by commands, never by writing.** The layer
  resolves a2ui's bindings against its own tree and mirrors a value into
  craftkit by issuing a **command**; it never writes the document. a2ui's own
  rule is that *"the local Data Model is the single source of truth"*, and
  craftkit's is that derived values are never stored — so the layer's model is
  the truth for an **a2ui binding**, craftkit's document is the truth for
  **craftkit's state**, and the mirrored value is an **effect** that is
  re-derived, never read as authority.
  And **the core does not depend on a2ui**: the **SDK** knows nothing about
  it, the **derive** gains one a2ui-specific macro that emits no a2ui type,
  and a binary without the plugin has no `a2ui.*` kinds at all. (The absolute
  form — "craftkit never learns that a2ui exists" — is **false**, because the
  components are craftkit kinds named `a2ui.text` and so on, and the registry
  will carry that name.)
- **The layer resolves paths itself, and that is allowed.** The rule in §3.4
  is that a **front end** may not hold a `Plan`; the compat layer is a
  library, not a front end, so its own path helper is not the O7 work and
  does not depend on it. What it may not do — and does not — is hand a front
  end a plan value.
- **A function call is a command — when the layer has one.** An a2ui `Button`
  carries an **`action`**, and an action is one of two things
  (`common_types.json`'s `Action`): an agent-side **`event`**, or a
  renderer-side **`functionCall`**. A `functionCall` is resolved the way the
  protocol says — **local first**, the agent only as a fallback
  (`a2ui_protocol.md:1482-1486`) — and the layer's local implementation is a
  **function table** from an a2ui function name to a craftkit **command id**.
  A name in that table runs through `Session::execute`, so an a2ui surface
  gets craftkit's undo, its journal, its `mutates` rules and its refusals —
  for free, because it used the same door. The `event` is **not** a command:
  it is the agent's own vocabulary, and the layer forwards it to the agent as
  a2ui's `action` message.

**Cost.** *(As first estimated, before the plan existed:)* a crate at L4, six
kinds, two or three commands, and a path-resolution helper of perhaps two
hundred lines. Nothing in the SDK changes; nothing in the raster changes;
nothing in the window changes.

*(Corrected by [`a2ui-compat-layer.md`](a2ui-compat-layer.md): the plan needs
**three** crates (L3, L4 and an app), **eleven** components, **one new derive
macro**, and it **does** change **three** crates — the SDK (a `CatalogEntry`
contribution and the schema mapping moved in from `craftkit-mcp`), the derive
crate (the new macro), and **the vector plugin** (the SVG path-data reader
moves to L2, because the layer cannot reach it where it is, and it gains an
**arc**, which a2ui's `svgPath` may carry). Nothing in the raster and nothing
in the window changes, and that still holds.)*

**What it does not do.** It does not expose craftkit's 99 kinds as a2ui
components — only its **eleven** a2ui components are, and the other
eighty-eight are not. Exposing them all would be a second catalogue, and the
one a2ui's negotiation exists to reconcile. If an application wants `layout.spread` to be
addressable as an a2ui component, that is a **declaration** in the
application, not a change to craftkit.

**And the honest caveat.** a2ui's model chooses *widgets*; craftkit's model
chooses *commands*. A compat layer makes an a2ui-speaking agent able to
drive craftkit, and it does **not** make craftkit's own kinds choosable by
a model. The second thing would be a different idea, and §3.6 says why it
does not transfer.

**Done when.** A crate at L4 contributes `a2ui.text` and `a2ui.column`; an
a2ui `createSurface` with a `Column` of two `Text`s produces two craftkit
nodes and one group, drawn by `session.paint_view` into a `Raster` with no
change to the raster or the window; and an a2ui `Button` with a
`functionCall` whose name is in the layer's function table runs that command
through `Session::execute` and refuses it the way every other door refuses
it, while a name that is in neither the table nor the catalogue's `functions`
is refused rather than forwarded.

---

## 5. What not to copy

1. **Unknown → placeholder.** a2ui resolves a component whose type has no
   catalog entry to a **placeholder node** in state `unknown-type` and
   dispatches `UNKNOWN_COMPONENT_TYPE`
   (`blueprints/features/node_resolution.blueprint.md`); the adapter renders
   nothing in its place. craftkit's rule — *"Unknown names are refused"* — is
   stronger, and should win: an unknown name is an error, not a blank region. (a2ui's *pending* state is
   different in kind: "not here **yet**", and the craftkit analogue — a
   registry changing under a live panel, O6 — is the one place where
   upgrade-in-place is right.)
2. **A hand-written catalog.** a2ui's catalogs are hand-written JSON Schema,
   versioned by hand — `v0_9`, `v0_9_1` and `v1` under `catalogs/`, `v0_8` …
   `v1_0` under `specification/`, and a hand-written guide to changing one at
   `specification/v0_8/docs/custom_catalog_changes.md`. craftkit's are
   compiled from the types, which is why the derive macro exists. Take the
   role, keep the mechanism.
3. **A protocol with its own version ladder.** a2ui carried v0.8 → v0.9 →
   v0.9.1 → v1.0 with per-version adapters. craftkit already has a version
   mechanism — the format chain and migrations, checked by the ecosystem
   (`crates/craftkit-sdk/src/ecosystem.rs`) — and should use it rather than
   grow a second.
4. **a2ui's nouns.** `surface`, `dataPath`, `instanceId`, `ResolvedBinding`
   are a2ui's vocabulary. craftkit's are `Session`, `Document`, `Node`,
   `Relation`, `Plan`, `Outcome`. Borrow the *shape*, not the words.
5. **a2ui's plurality of catalogues.** a2ui carries **four shared
   catalogues** and a negotiation over which sets each side supports. craftkit
   has **one** `Canvas` trait and **one** kind registry (§3.6), so every
   backend draws every kind and the effective catalogue is always the whole
   of it. A compat layer that grew a catalogue per renderer would be
   reintroducing the problem craftkit's single seam already solves.

And the conformance suite is out of scope by instruction — but note *why* that
suits craftkit: craftkit's equivalent of conformance is `crates/craftkit-gates`
plus `cargo xtask ci`, and O4's parity matrix is how craftkit buys back the
part that matters — *the doors agree with each other* — without importing a
language-agnostic corpus.

---

## 6. Two groups, and the order

### 6.1 Defects — these wait for nobody

Three house-rule violations exist today (§3.5). They are not opportunities,
they are not a2ui's business, and they need no product decision. Each is
small, each has a test, and each is the kind of thing the tree's own
`KNOWN-GAPS.md` is for.

| # | the change | the file | how you know it is done |
|---|------------|----------|--------------------|
| D1 | the studio undoes through the **registry**, not through `session.undo()` | `apps/craftkit-studio/src/main.rs:170` | a studio test asserting the studio never names `undo`/`redo` — **not** the gate, which does not look at the studio at all |
| D2 | the studio's `evaluators` line is dropped or read through a projection method — `projection().manifest()` already names every evaluator (`crates/craftkit-sdk/src/ecosystem.rs:577-578`) | `apps/craftkit-studio/src/main.rs:36` | a studio test asserting the studio never names `evaluators` |
| D3 | the window's `plan()` stops calling the evaluator and **keeps** derived values — it reads them the way the rest of the tree does | `apps/craftkit-window/src/lib.rs:201-209` | `cargo test -p craftkit-gates` — the window **is** in `FRONT_ENDS`, so the gate sees it; and the forbidden list needs `evaluate`/`PlanRow` added, not only `Plan` |

**Why D1 and D2 have no gate test:** `FRONT_ENDS` holds only
`craftkit-window` (`:274`), so `front_ends_are_thin` never looks at the
studio. The gate's own test —
`crates/craftkit-gates/tests/tree.rs:22` (`the_tree_is_clean`), run by
`cargo test -p craftkit-gates` — passes today *and* after D1 and D2 are
fixed, which makes it a **vacuous** acceptance test for those two.

It becomes the acceptance test for **D3** as soon as the forbidden list
names `evaluate` and `PlanRow` — the window is already in `FRONT_ENDS`, so
only the list has to grow — and for **D1 and D2** only once **O4.1** puts
the studio in it.

**All three are done** (README §1115). **D1**: the studio runs
`history.undo`, a command like any other. **D2**: the `evaluators` line is
dropped, and the manifest carries every evaluator with its layer and its
line about itself. **D3**: the window has no `plan()` and no `PlanRow`; it
asks the `doc.plan` command and draws the text it says.

**The acceptance is the gate, not a studio test.** `FRONT_END_FILES` puts
the studio's **entry point** under the rule — the studio's library is a
command book, and a command's body may open the document — and the
forbidden list now names `evaluate`. It does not name `PlanRow`: there is
no `PlanRow` in the tree to name. Two gate tests carry the new scope: a
host whose entry point undoes itself is caught, and a host's library is
not a front end.

### 6.2 Opportunities — these wait for a product question

| # | Opportunity | Cost, honestly | Serves §7's claim? | **Done when** |
|---|-------------|------|------|------|
| 1 | **O1** the catalog, in the SDK | **small** — one method, three call sites | **yes** | `Projection::catalog()` exists, and a test asserts that both doors' `tools/list` are **derived from it** rather than built by hand (the two lists differ on purpose — see O2) |
| 2 | **O2** prune the agent-facing view | **small**, and only once O1 exists | **yes** | the agent-facing list equals the pruned set, asserted by a test |
| 3 | **O5** model picks commands, pre-binds params | **small, *if* scoped this way**; large if scoped as a "surface" | **yes** | a door calls `execute_with` with a `Supplier` fed by the model's own output |
| 4 | **O8** bounds derived into the catalog | **small** — in practice one `maxLength` | **yes** | the text bound appears in the schema the agent is told — **done** (README §1117): `maxLength`, read from the SDK's own `MAX_TEXT_CHARACTERS` |
| 5 | **O4.1** classify the doors | **not one line** — see §3.2: a decision, the bridge's `evaluators`, the studio's hits, and a change to the rule itself | no | `FRONT_ENDS` names the doors, and `cargo test -p craftkit-gates` is green with them in it — **done** (README §1116) |
| 6 | **O4.2** the parity matrix | **a project, not a task** — six doors and two transport libraries; the honest first cut is a table checked by hand | no | the table exists, and every row has been checked by hand — **done** ([`door-parity.md`](door-parity.md)): the six doors side by side, cell by cell, with the line each cell was read in |
| 7 | **O3** recipe **claim** check | **small**, but it is a judgement, not a link — the link half is already done by `cargo xtask lists` (`xtask/src/lists.rs:55`) | no | a named claim in a recipe is checked against the tree, and a broken claim is reported |
| 8 | **O6** registry reconciliation | **medium**, and only the "appeared" direction exists | no | the registry carries a version counter and a panel reconciles against it |
| 9 | **O7** paths resolving inside the SDK | **large**, and architecture-defining | no | a path resolves to a plain JSON value inside the SDK, and no front end names a `Plan` |
| 10 | **O11** the a2ui compat layer — **scope superseded by [`a2ui-compat-layer.md`](a2ui-compat-layer.md)** | **medium** — three crates (L3, L4, an app), eleven components, one new derive macro, **the SVG path-data reader promoted to L2** (and it learns arcs), and **changes to the SDK, to the derive crate and to the vector plugin**; **no** change to the raster or the window | **yes** | yes — an a2ui-speaking agent can drive craftkit |
| — | **O9** evals | **blocked on a posture decision** — the tree has no outbound network and no secret-management story; that is a decision memo, not an opportunity | measures the claim | — |
| — | **O10** contracts | **high**, and the README must keep the bounds; do it when the contracts have stopped moving | no | — |

**Blockers a reader will meet in the first hour**, beyond the costs above:

- **Adding a crate to the tree is three edits, not one**: the `Cargo.toml`,
  the layer table in `crates/craftkit-gates/src/lib.rs` (`LAYERS`), and
  `docs/architecture.md` — which `cargo xtask lists` checks against the gate's
  table. Any new crate (the rejected `craftkit-catalog`, a parity harness)
  carries all three.
- **The SDK's public surface is the plugin API.** A new public method on
  `craftkit-sdk` is a semver-visible addition to the surface plugins compile
  against, even though it is only a *view*.
- **A zero-network tree cannot fetch a model**, so O9 needs **two**
  approvals — a posture decision and a credential story — not one.

### 6.3 The recommendation, and the open question

**Start with the three defects (§6.1).** They wait for no product decision,
they are small, each has a test that fails before it and passes after, and
they are real.

**Then, if the target is an agent-driven application**, the coherent package
is **O1, O2, O5, O8 and O11**: one vocabulary, pruned for the agent, with the
bounds derived into it, the model's own output feeding the existing
`Supplier` seam, and — if the agent speaks a2ui — a compat layer that adds
**eleven components** and changes the SDK (a `CatalogEntry` contribution and
the schema mapping moved in from `craftkit-mcp`) **and the vector plugin** (the
SVG path-data reader moves to L2 and learns arcs), and **nothing else** in the
tree. O3, O4, O6, O7 and O10 are adjacent (§7);
**O9 is adjacent as well, and blocked** on the posture decision.

**The open question, still unanswered.** Which application wants an agent to
choose its commands? craftkit is a scaffold, and the answer decides whether
O1–O2, O5 and O8 are worth doing at all:

- **If the target is an agent-driven application** (a studio, a bridge, a
  product that wants an agent to drive it), O1, O2, O5 and O8 are the
  package.
- **If the target is a person at a window**, the window already builds a form
  from `spec.params` (`apps/craftkit-window/src/lib.rs:219-286`) and the agent
  doors already work; most of this document is unnecessary.
- **If the target is a downstream scaffold**, the right output of this
  exercise is a **recipe** (`docs/recipes/`) describing the pattern, and no
  code at all.

That question is not answered inside this repository, and this document does
not assume an answer.

## 7. What serves the claim, and the one-line version

a2ui's transferable idea is that **an agent's vocabulary must be a schema the
host owns**. Of the eleven opportunities, five instantiate that claim
directly:

- **O1** — one canonical view of the registry, that the model can be told
  about;
- **O2** — the view the model is given, pruned to what it can use;
- **O5** — the model filling that schema in, through the seam that already
  exists;
- **O8** — the schema saying what the bounds are, so the model is *told*
  rather than refused;
- **O11** — an a2ui-speaking agent driving craftkit through the layer, with
  its own a2ui-shaped catalogue **over the ordinary `Canvas` seam**, and no
  change to the seam or to anything else.

The other six are **adjacent**, and are marked as such rather than smuggled
in as part of the same finding:

- **O3** and **O10** are about prose, not vocabulary;
- **O6** is about a registry that grows under a live panel;
- **O7** borrows a2ui's *binding* half — a path addresses a value — not its
  vocabulary half;
- **O4** is about the doors agreeing with each other;
- **O9** measures the claim.

**The one-line version.** a2ui's transferable idea is that an agent's
vocabulary must be a schema the host owns. craftkit already compiles a
vocabulary — from its types, so it cannot drift — and already has the prompt
seam for an agent to fill it in. What it does not have is **one canonical
view** of that vocabulary for the doors to serve.

The second thing is the **three house-rule violations the tree has today**
(§3.5, §6.1): the studio calls `undo` directly, the studio reaches for the
evaluator's registry, and the window calls the evaluator and keeps what it
computed. Those are defects, and they wait for nobody.

**Door *parity* is a separate matter** — that is O4, and it waits for a
product question like the rest of the adjacent six.
