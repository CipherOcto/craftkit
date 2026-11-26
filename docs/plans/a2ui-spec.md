# The a2ui specification plan: what the specification requires and the door does not do

This plan is about the **specification's own conformance**: the messages it
requires a renderer to answer, the checks it requires a renderer to apply, the
bindings it requires a renderer to carry, and the rules it forbids a renderer
from breaking. It **sits beside** `docs/a2ui-compat-layer.md`, which is the
plan for the compatible surface, and it does not replace it. The two meet at
the **door** — `apps/craftkit-a2ui-server` — and at the **layer** —
`crates/craftkit-a2ui` and `crates/craftkit-plugin-a2ui` — and nothing else.
Nothing in this file says that any of it exists today: every arc below is
future work. The plan holds **ten arcs** — the first wave in §1264–§1271 and
the second wave in §1372–§1373 — and reserves **§1264 to §1373**.

## 0. The rule

Every message the specification requires a renderer to answer is answered, and
every rule it requires a renderer to apply is applied.

## 1. What the survey found

| capability | upstream path:line | craftkit's state | verdict |
|---|---|---|---|
| `updateDataModel` replaces the value at a path | `specification/v1_0/docs/a2ui_protocol.md:258` | partial (the door reads `contents` and always writes the whole model; the command `a2ui.data.set` does take a path) | PORT |
| `callRendererFunction` carries a nested `callFunction`, a `catalogId` and a `functionCallId` | `specification/v1_0/docs/a2ui_protocol.md:300` | partial (the door takes a flat `name`/`args`; `catalogId` and `functionCallId` are ignored) | PORT |
| Check rules (`@checks`) evaluated, `ValidationResult`, a failing check disables a button | `specification/v1_0/docs/a2ui_protocol.md:1112` | absent (the layer strips `@checks` and evaluates none) | PORT |
| Two-way binding: typing writes into the local model | `specification/v1_0/docs/a2ui_protocol.md:986` | absent (no command carries a user's input into the model) | PORT |
| `rendererFunctionResponse` after every renderer-side call | `specification/v1_0/docs/a2ui_protocol.md:302`, `:351` | absent (the door answers with a JSON-RPC result) | PORT |
| `metadata.extensions` must not be rejected | `specification/v1_0/docs/a2ui_protocol.md:1647` | absent (the door refuses such a surface) | PORT |
| Renderer-to-agent `error` message (surface faults) | `specification/v1_0/docs/a2ui_protocol.md:1556` | absent (only JSON-RPC errors are emitted) | PORT |
| Template child lists: children generated from a data-model path, with `@index` | `specification/v1_0/json/common_types.json:78` | absent (the layer refuses a template list) | PORT |
| UAX #31 identifier rules and the reserved `@` namespace | `specification/v1_0/docs/a2ui_protocol.md:1256-1275` | partial (reserved `@` functions are refused; agent-supplied names are not checked) | LATER |
| The catalogue's own `allowedParents` composition constraints | `specification/v1_0/test/cases/composition_constraints.json:1` | partial (craftkit uses its own parent rules) | LATER |
| Single-message instantiation | `specification/v1_0/docs/a2ui_protocol.md:191` | present | SKIP |
| `sendDataModel`: whole model in the metadata of every message | `specification/v1_0/docs/a2ui_protocol.md:1095` | present | SKIP |
| `action` and `callAgentFunction` messages | `specification/v1_0/docs/a2ui_protocol.md:1442`, `:1474` | present | SKIP |
| Capabilities handshake, inline catalogues | `specification/v1_0/docs/a2ui_protocol.md:1612-1640` | present | SKIP |
| Seven basic-catalog components craftkit refuses | `specification/v1_0/docs/a2ui_protocol.md:1233-1250` | absent, deliberately | SKIP |

Eight rows are **PORT**: eight capabilities the specification requires and the
tree does not deliver. They are not all the same kind of work. Three of them
are about the **data model** and the **functions** the layer owns, two are
about the **binding** between a surface and its model, and three are about the
**door's own replies**.

Two of the eight are marked *partial* rather than absent, and the difference
matters. The door already knows what a data-model update and a renderer
function call are, but it handles them in the flat, convenient way rather than
the way the specification words them. An arc that changed nothing but the
wording would be dishonest; an arc that changes the shape is real work.

The verdicts are the survey's, and they are the reason this plan exists: the
**PORT** rows are the first wave, the **LATER** rows are the second wave below,
and the **SKIP** rows are work that is already done or work the tree does on
purpose.

## 2. The arcs

An arc is three edits: a row in the port manifest, a row in the README's claim
table, and a README section. This plan reserves **§1264–§1271** — eight
sections, one per arc, in the order below — and **§1372–§1373** for the second
wave that follows the first table.

| arc | rule | from | to | module | command | test | section |
|---|---|---|---|---|---|---|---|
| **A** | a data-model update replaces the value at a path, and nothing else in the model changes | `docs/plans/a2ui-spec.md` | `craftkit-a2ui` | `src/model.rs` | `craftkit-a2ui::model::set` | `crates/craftkit-a2ui/tests/model.rs` | §1264 |
| **B** | a renderer function call carries its nested `callFunction`, its `catalogId` and its `functionCallId` | `docs/plans/a2ui-spec.md` | `craftkit-a2ui` | `src/functions.rs` | `craftkit-a2ui::functions::declaration` | `crates/craftkit-a2ui/tests/functions.rs` | §1265 |
| **C** | the check rules are evaluated, and a failing check disables its control | `docs/plans/a2ui-spec.md` | `craftkit-a2ui` | `src/checks.rs` | `craftkit-a2ui::checks::evaluate` | `crates/craftkit-a2ui/tests/checks.rs` | §1266 |
| **D** | typing writes the user's input into the local model, at the bound path | `docs/plans/a2ui-spec.md` | `craftkit-a2ui` | `src/bindings.rs` | `craftkit-a2ui::bindings::write` | `crates/craftkit-a2ui/tests/bindings.rs` | §1267 |
| **E** | every renderer-side call is answered with a `rendererFunctionResponse` | `docs/plans/a2ui-spec.md` | `apps/craftkit-a2ui-server` | `src/lib.rs` | `a2ui.component.invoke` | `apps/craftkit-a2ui-server/tests/protocol.rs` | §1268 |
| **F** | `metadata.extensions` is carried through, not refused | `docs/plans/a2ui-spec.md` | `apps/craftkit-a2ui-server` | `src/lib.rs` | `a2ui.surface.create` | `apps/craftkit-a2ui-server/tests/door.rs` | §1269 |
| **G** | a fault in a surface is sent to the agent as an `error` message | `docs/plans/a2ui-spec.md` | `apps/craftkit-a2ui-server` | `src/lib.rs` | `a2ui.surface.apply` | `apps/craftkit-a2ui-server/tests/stream.rs` | §1270 |
| **H** | a template child list is generated from a data-model path, and `@index` names the item | `docs/plans/a2ui-spec.md` | `craftkit-plugin-a2ui` | `src/tree.rs` | `a2ui.surface.apply` | `crates/craftkit-plugin-a2ui/tests/tree.rs` | §1271 |

**Four arcs land in the layer and four in the door.** Arcs A, B, C and D are
the layer's own work: the model, the function table, the check rules and the
bindings. Arcs E, F and G are the door's: what it answers with, what it
accepts, and how it says that something went wrong. Arc H is the plugin's, and
it is the one arc that changes how a surface is built rather than how it is
described.

**The second wave: the whole upstream.**

| arc | rule | from | to | module | command | test | section |
|---|---|---|---|---|---|---|---|
| **I** | a name that does not fit the specification's identifier rules is refused by name, and a refusal is an error and not a renamed thing | `docs/plans/a2ui-spec.md` | `craftkit-a2ui` | `src/names.rs` | `a2ui.name.check` | `crates/craftkit-a2ui/tests/names.rs` | §1372 |
| **I** | the door applies the identifier rules to every name an agent supplies | `docs/plans/a2ui-spec.md` | `apps/craftkit-a2ui-server` | `src/lib.rs` | `a2ui.name.apply` | `apps/craftkit-a2ui-server/tests/names.rs` | §1372 |
| **J** | the catalogue says what may hold what, and the tree's own parent rules are replaced by the catalogue's table rather than kept beside it | `docs/plans/a2ui-spec.md` | `craftkit-a2ui` | `src/catalog.rs` | `a2ui.catalog.parents` | `crates/craftkit-a2ui/tests/parents.rs` | §1373 |

**What the second wave is.** The first wave carries the specification's
**messages** — what a renderer is told and what it answers. The second wave
carries the specification's own **rules**: what a name may be
(`specification/v1_0/docs/a2ui_protocol.md:1256-1275`) and what may hold what
(`specification/v1_0/test/cases/composition_constraints.json:1`). The house
rule applies to these as much as to anything else: **a rule about a name is a
command like any other**, so it is declared, registered and reachable through
the same registry as the rest. Arc I is the identifier rules and the reserved
`@` namespace, applied to the names an **agent** supplies; it lands in the
crate that owns the door's own rules and in the door that applies them, and
the two rows share one section. Arc J is the catalogue's own
`allowedParents` composition constraints, and it lands in the layer beside the
catalogue it constrains.

**And the other plan stays.** `docs/a2ui-compat-layer.md` owns the
compatible surface — the catalogue, the components, the door's commands. This
plan adds the specification's own demands **on top of** it, at the same two
places: the layer and the door. Where the two would touch — the door's reply
to a call, the door's reading of a surface's metadata — this plan's arc goes
first, and the other plan's arc follows it.

## 3. The order, and what each arc ships

| arc | ships | cost |
|---|---|---|
| A | a model that is written **at a path**, so an update touches one value | medium — the write path, the door's whole-model write, and the model's own tests |
| B | the call shape the specification words: nested, identified, catalogued | medium — the function table's shape, the door's reader, and the declaration the catalogue publishes |
| C | the check rules evaluated, with a `ValidationResult` of their own | large — a rule language, an evaluator, and a control that a failing rule disables |
| D | the binding that runs both ways: a value in, a typed value out | medium — the write side of the binding, the command that carries it, and the model's undo |
| E | the answer to a call, in the specification's own message | small — one reply shape and the test that names it |
| F | extensions carried through the door instead of refused | small — one accepted field and the test that shows it arrives |
| G | a fault sent as a message the agent reads | small — one message shape on top of the existing fault path |
| H | a template child list, generated per item, with `@index` | medium — a second branch of the tree, a binding of the item, and the layout that follows |
| I | the identifier rules, checked against every name an agent supplies | medium — the rule itself in the layer, the door's application of it, and the refusal that names the offending name |
| J | the catalogue's own `allowedParents` table, in one place | medium — the table, the check that reads it, and the tree's own parent rules taken out of their old place |

**The order is A, B, C, D, E, F, G, H, I, J.** The layer comes before the door
because the door answers in the layer's shapes: a reply that names a call
(Arc E) means nothing until the call itself is shaped the way the
specification words it (Arc B). C and D are independent of A and B and of each
other, but they share the model, so they follow it.

**The second wave is last.** I and J are the specification's own rules, and a
rule is applied to what the arcs before it build: the names an agent supplies
(Arc I) and the parents the catalogue allows (Arc J) are only settled once the
messages they arrive in are settled.

**E, F and G are the cheap three**, and they are the three a renderer would
notice first: a call with no answer, a payload with no home, a fault with no
message. They are last because a reply built on an unfinished model is a reply
to the wrong thing.

**C is the expensive one.** It is a rule language, not a field, and it is the
arc most likely to change shape while it is written — which is why it sits
after the two arcs that give it something to evaluate.

## 4. The tests

The house rule: the crate's own rule has a test.

- **A** — a set at a path changes that value and leaves the rest of the model alone; a path that does not exist is refused; a set is undoable.
- **B** — a call's JSON carries the nested `callFunction`, the `catalogId` and the `functionCallId`; a call without a `functionCallId` is refused; the catalogue publishes the same shape the door reads.
- **C** — a rule that holds evaluates to a passing `ValidationResult`; a rule that fails evaluates to a failing one; a failing rule disables its button; a malformed rule is refused rather than ignored.
- **D** — a typed value is written into the model at the bound path; a binding that is not two-way is refused; the write is undoable and rolls back when it fails.
- **E** — a call that the door answers is followed by a `rendererFunctionResponse` carrying the same `functionCallId`; a call the door refuses is not.
- **F** — a surface whose metadata carries `extensions` is accepted, and the extensions reach the surface; an absent `extensions` is still accepted.
- **G** — a surface fault reaches the agent as an `error` message naming the surface; a refusal that is not a fault stays a refusal.
- **H** — a template child list is generated once per item of the bound path, with `@index` naming the item; a path that does not resolve to a list is refused; the reserved `@` names are still refused.
- **I** — a name that fits the specification's identifier rules is accepted; a name that does not is refused **by name**, and the refusal is an error rather than a renamed thing; the reserved `@` namespace is refused for agent-supplied names as well as for the tree's own.
- **J** — the catalogue's `allowedParents` table decides what may hold what; a component placed under a parent the table does not allow is refused; and the tree's own parent rules are replaced by that table rather than kept beside it, so there is one table of parents in one place.

## 5. What it does not do

**Nothing in the specification's surface is refused.** The two rows the survey
held back are the second wave's arcs above: the identifier rules and the
reserved `@` namespace are **Arc I**, and the catalogue's own `allowedParents`
composition constraints are **Arc J**. What the survey marked **LATER** is no
longer deferred; it is work.

**The five rows the survey marked as present are parity, not gaps.** The
single-message instantiation, the whole model in the metadata of every message,
the `action` and `callAgentFunction` messages, and the capabilities handshake
with the inline catalogue: the door already does them. They are named here as
parity, and they are not arcs.

**The seven basic-catalogue components craftkit refuses are the other plan's
work.** `docs/a2ui-compat-layer.md` owns them. They are refused on purpose
here, and the refusal is the house rule: an unknown component is refused and
not faked, and a component that was half-implemented would be faked. Making
them real is that plan's arcs, not this one's — so they are **owned
elsewhere**, and are not refused forever.

## 6. The open questions

**The owner has allowed the door to change.** This plan touches the door's
own message shapes — the `updateDataModel` path, the `callRendererFunction`
envelope, the `rendererFunctionResponse` and the surface `error` message —
and the door is this tree's contract with an agent. The owner has said that
the contract may change, so the arcs below are not held for a decision about
the contract. What is still open is the **shape**: whether the door keeps
JSON-RPC as its transport and carries the protocol's messages inside it, or
answers in the protocol's messages directly, is a design choice this plan
does not settle.

**What the second wave leaves open is the shape of the rules themselves.**
Arcs I and J say that the identifier rules and the `allowedParents` table are
applied; they do not settle how those rules are written down — where the
identifier rules live in the layer, whether the `allowedParents` table is
read from the catalogue at run time or generated from it, and what the tree
does with a parent rule the catalogue does not mention. Those are design
choices this plan does not settle.

**The specification's conformance suite and its per-language SDK directories
were not read.** Recent upstream commits in either may have a protocol-level
effect the survey did not verify, so the arcs above are written against the
protocol document alone.

**The upstream's proposals directory was listed, not read.** A proposal is not
the current specification, so nothing in it is counted here — but a proposal
that has since been adopted would change the shape of an arc.

**The refused components were judged from the layer's own definitions file
alone.** That file says what the layer refuses; it does not say what the
specification requires of those components, so the SKIP verdict for them rests
on a one-sided reading.

**Whether the layer ever evaluates a local `@call` at bind time was not traced
in every bind site.** If some sites do and some do not, Arc D is smaller than
it looks here, and its test is narrower.
