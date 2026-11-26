# The plans: one per project in the family tree

**What this directory is.** Each file here is a **plan**: it names the arcs
that would bring one project in the family tree into this tree's batteries.
There is one plan per project, because each project is one battery's worth of
ground, and a battery's worth of ground is best read on its own.

**A plan is not work.** Nothing here has been built. The plans reserve README
sections and name the rule each arc would carry; the port manifest, the claim
table and the README sections are written **when the arc lands**, not before.
A plan that reserves a section it never fills is a promise, and a promise in a
plan is not a lie — but it is a promise.

**Where the plans came from.** Each plan's §1 is a **survey**: a read of the
upstream project's own command registry against this tree's battery, one row
per capability, with the upstream's `path:line` and the battery's own state.
The surveys were read-only and bounded, and each says what it could not read
or decide (§6 of each plan).

---

## The plans

| plan | the upstream | the battery | arcs | README sections |
|---|---|---|---|---|
| [`pdfcraft.md`](pdfcraft.md) | the document-format project | `craftkit-plugin-pdf`, and the cad battery for the measure | 25 | §1184–§1307 |
| [`wordcraft.md`](wordcraft.md) | the prose project | `craftkit-plugin-prose` | 17 | §1198–§1312 |
| [`effectcraft.md`](effectcraft.md) | the compositing project | `craftkit-plugin-fx`, the vector, text and film batteries, and the shared library | 19 | §1210–§1325 |
| [`cadcraft.md`](cadcraft.md) | the drawing project | `craftkit-plugin-cad` | 15 | §1216–§1330 |
| [`vectorcraft.md`](vectorcraft.md) | the vector project | `craftkit-plugin-vector` | 15 | §1226–§1336 |
| [`gridcraft.md`](gridcraft.md) | the spreadsheet project | `craftkit-plugin-grid`, the vector and image batteries, and the shared library | 15 | §1235–§1345 |
| [`artcraft.md`](artcraft.md) | the video-editor project | `craftkit-plugin-art`, and the image, film and vector batteries | 18 | §1241–§1351 |
| [`craft-libs.md`](craft-libs.md) | the shared format and widget libraries | eight new library crates and the batteries that call them | 17 | §1253–§1359 |
| [`solidcraft.md`](solidcraft.md) | the solid modeller | a new `craftkit-plugin-solid`, and the cad, image and sdk | 14 | §1262–§1371 |
| [`a2ui-spec.md`](a2ui-spec.md) | the a2ui specification | `craftkit-a2ui`, `craftkit-plugin-a2ui`, `apps/craftkit-a2ui-server` | 10 | §1264–§1373 |
| [`soundcraft.md`](soundcraft.md) | the audio project | `craftkit-plugin-sound`, the film battery and the sdk | 38 | §1272–§1399 |
| [`designcraft.md`](designcraft.md) | the page-layout project | `craftkit-plugin-layout`, `craftkit-plugin-prose`, and the vector, image, text and pdf batteries | 33 | §1284–§1419 |

**Two hundred and thirty-six arcs, two hundred and thirty-six reserved
sections**, from §1184 to §1419, with no gap and no repeat.

**Each plan holds two waves.** The **first wave** is the survey's *port*
rows: the gaps that are commands over facts the batteries already hold. The
**second wave** is the rest of the upstream — the rows the survey called
*later* and the rows it called *skip* — and it is now **all of it**. The
owner has said the family is to be fully ported, and any rule that held a
capability back is changed so that it is not. A row that belongs to another
battery is an arc **in** that battery, and a row that needs a model is an arc
that brings the model.

**One plan was withdrawn.** The vector project's proof of concept had a plan
of two arcs — an importer and an exporter registry keyed by file extension.
The owner has said that project's work is **finished** and that it is out of
scope, so the plan is gone and its two sections are **not** reserved: §1272
and §1273 are the sound plan's first two arcs, and no section anywhere points
at a proof of concept that will not be worked.

**One plan sits beside an older one.** `a2ui-spec.md` is about the
**specification's** own conformance — its messages, its checks, its bindings —
and it sits beside [`../a2ui-compat-layer.md`](../a2ui-compat-layer.md),
which is about **this tree's** a2ui-compatible surface. They meet at the door
and at the layer, and neither replaces the other.

**Two plans share the prose battery.** `wordcraft.md` plans the prose gaps the
prose project shows; `designcraft.md` plans the prose gaps the page-layout
project shows. The four gaps they share — the character facts of a run, the
link on a run, the tab stops of a paragraph, the sort of a table — are
planned **once**, in `wordcraft.md`, and `designcraft.md` says so.

---

## The shape every plan has

An arc is **three edits and no more**:

1. a row in the port manifest (`xtask/src/ports.rs`),
2. a row in the README's claim table,
3. a section of the README.

And the row lands **with the test**: a port row fails the gate until the file
it names exists, so each arc's row names the test file that arc will create.

The manifest's fields are `rule`, `from`, `to`, `module`, `command`, `test`,
`section`. In these plans `from` is the plan itself — the document the arc
comes from — and `section` is the reserved number above.

**A plan that adds a crate carries three more edits**: the workspace's
`Cargo.toml`, the row in the layer table, and the row in
`docs/architecture.md`. Six of the craft-libs arcs add a crate, and the plan
says so.

---

## The order across the plans

The order is a **recommendation**, not a schedule. The owner decides what
lands.

| # | plan | why here |
|---|---|---|
| 1 | craft-libs | the shared libraries come first: the mesh containers, the picture formats and the settings layer are what later plans ask for, and a format that lands twice is a format that drifts |
| 2 | pdfcraft | the largest set of gaps in a battery that already has its model — no new model is needed, and every arc is a rule over a page tree the battery holds |
| 3 | wordcraft | the prose battery's gaps are parameters over facts it already carries, and the desktop-publishing plan leans on them |
| 4 | designcraft | after the prose plan, because it shares a battery and repeats four of its gaps |
| 5 | gridcraft | the chart and pivot depth is everyday work over a model that is already there |
| 6 | artcraft | the video-editor battery is thin in verbs and facts, and both are cheap |
| 7 | cadcraft | the drawing battery's entity kinds are cheap to add, and the shape-changing ones are not |
| 8 | vectorcraft | the vector battery's geometry is in good shape; what it lacks is what is laid on top of paths |
| 9 | effectcraft | the compositing battery's gaps are small and well-defined — a mask's facts, a layer's timing, a tangent, an expression's errors |
| 10 | soundcraft | the audio battery is nearly at parity; what is left is a track's own facts, a range's own operations and the session's own view |
| 11 | a2ui-spec | the specification's conformance work sits on top of the compatible surface, so it lands after that plan's arcs |
| 12 | solidcraft | last on purpose: the solid model is a new battery, and it is the largest thing on the list |

**The second waves come after the first.** Within every plan the order is
the same: the first wave is the gaps that are commands over facts the
batteries already hold, and the second wave is the rest of the upstream —
the new models, the new dependencies, and the capabilities that moved to
another battery. The second waves are the heavy half, and they are the half
that changes what a battery *is*: a grid battery with a maths engine and a
macros model, a layout battery with a flex layout and a book, a tree with a
three-dimensional document in it.

**What the order does not say.** It does not say that a battery's arcs land
together, and it does not say that a plan's arcs land at all. The rule the
tree already holds applies to each of the two hundred and thirty-six arcs
separately: an arc lands when it is written, tested and green.

---

## The rules that bind all of them

Every arc in every plan is bound by the same house rules, and a plan that
breaks one of them is wrong:

- **Refuse, do not clamp.** An out-of-range parameter is an error. Where a
  knob has a clamp of its own (the repeat count, the zoom, the rate), the
  clamp holds **and the answer says it held**.
- **Unknown names are refused.** A name that is not known is an error.
- **Derived values are derived.** A value that can be computed is not stored.
- **The UI is thin.** A front end reads the kit's state and acts through the
  registry. A new command is a new command in a battery, never a special case
  in a front end.
- **No panics.** No `unwrap`, no `expect`, no `panic`, anywhere outside a test.
- **The clean room.** No commercial product's name, and no other project's
  code: a plan ports **behaviour**, and cites the upstream's `path:line`.

---

## What is refused

Very little is refused now, and every refusal is named in the plan that holds
it, with the reason. The whole of it, gathered:

- **A front end's own surfaces.** The upstream's rendering, viewer and
  application windows, and the page transitions: they are not rules about a
  document. The tree's front ends draw their own.
- **A print surface.** The score printing: the tree's answer to printing is
  the PDF battery's write.
- **A second container for the same thing.** The upstream's own design-file
  container: craftkit has its own document format per battery.
- **A client's for a vendor's service.** Mail, cloud, presence and chat,
  e-sign and the AI-provider abstractions: the kit makes no outbound call
  and holds no credential, so a client for somebody else's service would be
  a client for nothing. What the tree keeps instead is the **model** those
  abstractions carry — a message, a chat, a signature, a model's request —
  and where a battery holds it, it is that battery's arc.
- **What another project already owns.** Design-file signing is the PDF
  plan's arc, the foreign design containers are the craft-libs plan's arcs,
  the mesh containers are the craft-libs plan's arc, and the seven
  basic-catalogue components are the a2ui compatibility plan's arcs. One
  answer lives in one place, and a plan says where.

**What is not refused.** Every other capability the family has is an arc in
one of the twelve plans — 236 arcs in all — and the second wave of each plan
is where the hard ones are: the models that do not exist yet (a solid body,
a face inside a path, a mesh gradient, a cell with a border, a table style),
the dependencies that must be added (the maths engine, the colour
management, the video encoder, the office formats), and the capabilities
that belong to another battery.

---

## The decisions the owner has taken

Five questions were put to the owner with these plans, and five answers came
back. They are recorded here because each one changed a plan.

1. **No releases yet.** The tree has one workflow and it is CI. The open
   question in `KNOWN-GAPS.md` §8 — O10, the contracts — keeps its trigger:
   the contracts are written when the tree has a **release** to write them
   for, and there is no release to write them for now. Nothing else in the
   plans waits on it.
2. **The view and the options are in scope.** The saved view and option
   state — the ruler, the counter, the saved views, the clip display
   options, the scrolling, solo and focus modes, the rulers, the zero point
   and the preferences — is a **fact of the document**, and the front end
   reads it and applies it. This is three arcs the plans did not have: two
   in the sound plan and one in the page-layout plan. The house rule is not
   broken by it: the **fact** lives in the document, and a front end that
   writes the fact is still refused by the gate.
3. **The a2ui door may change.** The specification's plan touches the
   door's own message shapes, and the owner has said that is allowed: the
   door is this tree's contract with an agent, and the contract is the
   owner's to change.
4. **The vector project's proof of concept is finished and out of scope.**
   Its plan is withdrawn, its two sections are not reserved, and nothing
   else in the plans waits on it.
5. **The family is to be fully ported.** Every capability the family has is
   in scope, and any rule that held one back is changed so that it is not.
   The plans' **second waves** are that decision: 123 arcs that were held
   back as "later" or "skip" and are now arcs, in the battery that owns
   them. What is refused is now short, and each refusal is named with its
   reason in the plan that holds it — the list is in "What is refused"
   above.
