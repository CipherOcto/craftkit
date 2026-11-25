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
| [`craft-libs.md`](craft-libs.md) | the shared format and widget libraries | several — six new library crates and three batteries | 9 | §1251–§1259 |
| [`pdfcraft.md`](pdfcraft.md) | the document-format project | `craftkit-plugin-pdf` | 14 | §1182–§1195 |
| [`wordcraft.md`](wordcraft.md) | the prose project | `craftkit-plugin-prose` | 12 | §1196–§1207 |
| [`effectcraft.md`](effectcraft.md) | the compositing project | `craftkit-plugin-fx` | 6 | §1208–§1213 |
| [`cadcraft.md`](cadcraft.md) | the drawing project | `craftkit-plugin-cad` | 10 | §1214–§1223 |
| [`vectorcraft.md`](vectorcraft.md) | the vector project | `craftkit-plugin-vector` | 9 | §1224–§1232 |
| [`gridcraft.md`](gridcraft.md) | the spreadsheet project | `craftkit-plugin-grid` | 6 | §1233–§1238 |
| [`artcraft.md`](artcraft.md) | the video-editor project | `craftkit-plugin-art` | 12 | §1239–§1250 |
| [`solidcraft.md`](solidcraft.md) | the solid modeller | `craftkit-sdk` | 2 | §1260–§1261 |
| [`a2ui-spec.md`](a2ui-spec.md) | the a2ui specification | `craftkit-a2ui`, `craftkit-plugin-a2ui`, `apps/craftkit-a2ui-server` | 8 | §1262–§1269 |
| [`vectorcraft-poc.md`](vectorcraft-poc.md) | the vector project's proof of concept | `craftkit-sdk` | 2 | §1270–§1271 |
| [`soundcraft.md`](soundcraft.md) | the audio project | `craftkit-plugin-sound` | 10 | §1272–§1281 |
| [`designcraft.md`](designcraft.md) | the page-layout project | `craftkit-plugin-layout`, `craftkit-plugin-prose` | 12 | §1282–§1293 |

**One hundred and twelve arcs, one hundred and twelve reserved sections**, from §1182 to
§1293, with no gap and no repeat.

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
| 10 | soundcraft | the audio battery is nearly at parity; what is left is a track's own facts and a range's own operations |
| 11 | a2ui-spec | the specification's conformance work sits on top of the compatible surface, so it lands after that plan's arcs |
| 12 | vectorcraft-poc | the importer and exporter registries are small, and the window has no file dialogs yet to use them |
| 13 | solidcraft | last on purpose: the solid model is a new battery, and it is the largest thing on the list |

**What the order does not say.** It does not say that a battery's arcs land
together, and it does not say that a plan's arcs land at all. The rule the
tree already holds applies to each of the one hundred and twelve arcs separately: an arc
lands when it is written, tested and green.

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

## What is not planned

Each plan's §5 names the rows it holds back, and the reason. Across the
thirteen plans the held-back work is of three kinds:

- **New models, not missing commands.** A parametric solid model, a table
  style model, a direction on a paragraph, a mesh gradient, a media library,
  a scripting language. These are batteries of their own, and a plan that
  pretended otherwise would be a promise it cannot keep.
- **New dependencies.** The office document formats, the layered image
  format, the camera raw containers, the other audio codecs, the colour
  management model.
- **Another battery's work.** Media and rendering belong to the film battery,
  paths belong to the vector battery, caret typing belongs to the text
  battery, plugin discovery belongs to the plugin batteries.

---

## The decisions that are the owner's

Three things in these plans are not the tree's to decide, and each plan says
where it touches them:

1. **A release.** The tree has one workflow and no release; the trigger for
   the open question in `KNOWN-GAPS.md` §5 is a release, and until then the
   work lands as it lands.
2. **The view preferences.** The sound survey classified the upstream's view
   and options families as out of scope, on the tree's own rule that a front
   end reads state and never writes the document. If saved view preferences
   belong in the document, several of those rows become ports.
3. **The extension of the plain-text door.** The a2ui specification's plan
   touches the door's own message shapes, and the door is this tree's
   contract with an agent — a change there is a change to the contract.
