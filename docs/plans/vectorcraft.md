# The vectorcraft plan: what the upstream has and the vector battery does not

This plan is a survey of one upstream tree against the vector battery, written
before any of it is built. It names fifteen arcs of future work, in two waves,
and says plainly what the upstream has that the battery does not. The sections
reserved for this plan are **§1226 through §1336**.

## 0. The rule

Every arc serves one rule: **a capability the upstream has is a command in
the battery that owns it, and a capability the upstream does not have is
not.**

## 1. What the survey found

The survey read the upstream at `/mnt/data/mmacedoeu/work/tools/vectorcraft` and
compared it with `craftkit-plugin-vector` as it stands. Nine capabilities are
marked PORT: they are commands to port, and they are the **first wave**. Six
are marked LATER: they are whole models rather than commands, and they are
the **second wave**.

| capability | upstream path:line | craftkit's state | verdict |
|---|---|---|---|
| Clipping mask | `crates/engine/src/cmd/object/commands.rs:204` | absent | PORT |
| Pathfinder over a whole selection | `crates/engine/src/cmd/pathops.rs:30` | partial (unite/subtract/intersect/exclude take two paths only) | PORT |
| Duplicate a path | `crates/engine/src/cmd/edit.rs:233` | absent (copies exist only as a by-product of grid/radial/mirror) | PORT |
| Align objects | `crates/engine/src/cmd/object/commands.rs:267` | absent | PORT |
| Distribute objects | `crates/engine/src/cmd/object/commands.rs:276` | absent | PORT |
| Rotate, reflect and shear about a point | `crates/engine/src/cmd/object/commands.rs:33` | partial (only a raw matrix transform) | PORT |
| Reusable symbol instances | `crates/engine/src/cmd/brushsym/symbols.rs:294` | absent | PORT |
| Arrowheads on stroke ends | `crates/engine/src/cmd/arrowheads.rs:45` | absent | PORT |
| Export to PDF | `crates/engine/src/cmd/fileio/pdf.rs:127` | absent (SVG and EPS only) | PORT |
| Live paint (paint the faces of a shape) | `crates/engine/src/cmd/buildcmds.rs:85` | absent | LATER |
| Image trace | `crates/engine/src/cmd/buildcmds.rs:157` | absent | LATER |
| Gradient mesh | `crates/engine/src/cmd/live/mesh.rs:13` | absent | LATER |
| Object patterns (tiled fill) | `crates/engine/src/cmd/patterncmds.rs:33` | absent | LATER |
| Slices (export objects apart) | `crates/engine/src/cmd/slices.rs:26` | absent | LATER |
| Envelope / mesh distort | `crates/engine/src/cmd/live/commands.rs:153` | absent | LATER |

The nine PORT rows are the arcs of §2, in the order the survey gave them. Each
one is a command the battery can grow without changing a model: a mask, a
pathfinder over a selection, a duplicate, an align, a distribute, a transform
about a point, a symbol instance, an arrowhead, a PDF export.

The six LATER rows are the second wave: §2 names them as arcs 10 to 15, and §5
says what each of them needs.

The later work is the live paint, the image trace, the gradient mesh, the
object patterns, the slices and the envelope distort. Each of those is a model
before it is a command — a mesh, a raster, a fill — and each of them is an arc
of the second wave, whose rule says what the model is.

## 2. The arcs

Each arc is three edits: a row in the port manifest, a row in the README's claim
table, and a section in the README. The sections reserved for this plan are
**§1226 through §1336**, one per arc, in the order below.

| arc | rule | from | to | module | command | test | section |
|---|---|---|---|---|---|---|---|
| **1** | a clipping mask is a node on the document, not a flag on a shape | `docs/plans/vectorcraft.md` | `craftkit-plugin-vector` | `src/mask.rs` | `vector.mask.clip` | `crates/craftkit-plugin-vector/tests/mask.rs` | §1226 |
| **2** | the pathfinder takes a whole selection, not two paths | `docs/plans/vectorcraft.md` | `craftkit-plugin-vector` | `src/pathops.rs` | `vector.pathops.selection` | `crates/craftkit-plugin-vector/tests/pathops.rs` | §1227 |
| **3** | a path can be duplicated on its own, not only as a by-product | `docs/plans/vectorcraft.md` | `craftkit-plugin-vector` | `src/duplicate.rs` | `vector.path.duplicate` | `crates/craftkit-plugin-vector/tests/duplicate.rs` | §1228 |
| **4** | objects align to one another, not to the origin | `docs/plans/vectorcraft.md` | `craftkit-plugin-vector` | `src/align.rs` | `vector.align.objects` | `crates/craftkit-plugin-vector/tests/align.rs` | §1229 |
| **5** | objects distribute along a span, evenly | `docs/plans/vectorcraft.md` | `craftkit-plugin-vector` | `src/distribute.rs` | `vector.distribute.objects` | `crates/craftkit-plugin-vector/tests/distribute.rs` | §1230 |
| **6** | a transform is about a point: rotate, reflect and shear | `docs/plans/vectorcraft.md` | `craftkit-plugin-vector` | `src/transform.rs` | `vector.transform.rotate`, `vector.transform.reflect`, `vector.transform.shear` | `crates/craftkit-plugin-vector/tests/transform.rs` | §1231 |
| **7** | a symbol is a reusable instance, and the instance is what is on the canvas | `docs/plans/vectorcraft.md` | `craftkit-plugin-vector` | `src/symbols.rs` | `vector.symbol.instance` | `crates/craftkit-plugin-vector/tests/symbols.rs` | §1232 |
| **8** | a stroke may carry arrowheads at its ends | `docs/plans/vectorcraft.md` | `craftkit-plugin-vector` | `src/arrowheads.rs` | `vector.stroke.arrowheads` | `crates/craftkit-plugin-vector/tests/arrowheads.rs` | §1233 |
| **9** | the battery exports PDF as well as SVG and EPS | `docs/plans/vectorcraft.md` | `craftkit-plugin-vector` | `src/pdf.rs` | `vector.export.pdf` | `crates/craftkit-plugin-vector/tests/pdf.rs` | §1234 |

These arcs are nine because the survey marked nine capabilities PORT, and each
one is a command with a test and a section. Arc 2 is the one that changes an
existing command's shape — the pathfinder already takes two paths, and the
survey's finding is that the upstream's takes a whole selection — so it is the
only arc that touches behaviour the battery already has. Arc 6 is the only arc
with three command ids, because rotate, reflect and shear are three commands
sharing one model.

**The second wave: the whole upstream.**

| arc | rule | from | to | module | command | test | section |
|---|---|---|---|---|---|---|---|
| **10** | a path's faces are painted, and each face carries its own fill | `docs/plans/vectorcraft.md` | `craftkit-plugin-vector` | `src/livepaint.rs` | `vector.paint.live` | `crates/craftkit-plugin-vector/tests/livepaint.rs` | §1331 |
| **11** | a picture is traced into paths, and the picture is read, not copied | `docs/plans/vectorcraft.md` | `craftkit-plugin-vector` | `src/imagetrace.rs` | `vector.trace.image` | `crates/craftkit-plugin-vector/tests/imagetrace.rs` | §1332 |
| **12** | a gradient is a mesh, and the mesh is a node of its own | `docs/plans/vectorcraft.md` | `craftkit-plugin-vector` | `src/gradientmesh.rs` | `vector.fill.gradientmesh` | `crates/craftkit-plugin-vector/tests/gradientmesh.rs` | §1333 |
| **13** | a fill may name other objects, and the named objects are the tile | `docs/plans/vectorcraft.md` | `craftkit-plugin-vector` | `src/patterns.rs` | `vector.fill.pattern` | `crates/craftkit-plugin-vector/tests/patterns.rs` | §1334 |
| **14** | the document may be cut into slices, and each slice is written out on its own | `docs/plans/vectorcraft.md` | `craftkit-plugin-vector` | `src/slices.rs` | `vector.export.slices` | `crates/craftkit-plugin-vector/tests/slices.rs` | §1335 |
| **15** | a shape is bent through a mesh, and the mesh does the bending | `docs/plans/vectorcraft.md` | `craftkit-plugin-vector` | `src/envelope.rs` | `vector.transform.envelope` | `crates/craftkit-plugin-vector/tests/envelope.rs` | §1336 |

The second wave is the whole upstream: every capability the survey found is an
arc, and nothing is held back. The survey's "later" verdicts were judgements
about size — a mesh, a raster and a fill are bigger than a command — and not
about scope. Each of these six arcs brings a **new model** into the battery,
and so each of them says what the model is: a face inside a path, a mesh, a
fill that names other objects, a slice of the document, and a picture read from
the image battery. The rule in each row is the model, stated in one sentence,
because the model is the thing the arc adds and the command is the thing it
uses.

## 3. The order, and what each arc ships

| arc | ships | cost |
|---|---|---|
| 1 | a clipping mask, and the command that applies it | small — one node kind and one command |
| 2 | a pathfinder over a selection | medium — the existing two-path pathfinder is generalised, and its tests move with it |
| 3 | duplicating one path | small — a copy that is a node in its own right |
| 4 | aligning objects | small — arithmetic over node bounds |
| 5 | distributing objects | small — arithmetic over node bounds, with a spacing rule |
| 6 | rotate, reflect and shear about a point | medium — a transform model, and three commands |
| 7 | symbol instances | medium — a definition and an instance, and the instance is derived |
| 8 | arrowheads on stroke ends | medium — arrowhead geometry and the stroke end it sits on |
| 9 | PDF export | large — a PDF writer is a format, not a function |
| 10 | live paint: the faces of a path, each with its own fill | large — a face model inside a path, and the fill that paints it |
| 11 | image trace: a picture turned into paths | large — a tracing algorithm, and a picture read from the image battery |
| 12 | gradient mesh: a gradient carried by a mesh | large — a mesh of its own, beside the geometry the battery has |
| 13 | object patterns: a fill that names other objects | large — a fill that references the document, so the document becomes part of a fill |
| 14 | slices: the document cut apart for export | large — an export that writes many files, and the cutting that decides them |
| 15 | envelope distort: a shape bent through a mesh | large — the mesh of arc 12, and the bending it does |

The order follows the survey's order, and the survey's order is the order in
which the capabilities depend on each other: the mask and the pathfinder come
first because they decide what a path *is*; duplicate, align and distribute are
arrangements of nodes that already exist; the transform and the symbol come
after them, because a transform about a point and a reusable instance both need
the node kinds the earlier arcs settled; the arrowhead is a property of a
stroke; and the PDF export is last, because it writes out whatever the earlier
arcs made.

The second wave follows the first, and every arc in it is large — each brings a
model rather than a command, so each is a change to what the document holds.
The order inside the wave is the survey's order, with one dependency written
into it: the envelope distort comes after the gradient mesh, because it bends a
shape through the same kind of mesh.

## 4. The tests

One test file per arc, named in the arc's row above, and each one lands with
the arc it belongs to.

- **1** — a mask clips what is inside it and leaves what is outside; a mask with no target is refused.
- **2** — the pathfinder over a selection of three paths returns one path; two paths still work; a selection of one path is refused.
- **3** — a duplicated path is equal to its original and has its own identity; editing one does not edit the other.
- **4** — aligning two objects to each other's left edge puts them level; the first object in the selection is the one that stays put.
- **5** — distributing three objects over a span spaces them evenly; a span with fewer than two objects is refused.
- **6** — rotating by a quarter turn about a point moves a node the way the geometry says; reflecting about a point leaves the point fixed; shearing about a point moves the point not at all.
- **7** — a symbol instance is derived from its definition; changing the definition changes the instance; the instance is on the canvas and the definition is not.
- **8** — an arrowhead sits on the end of a stroke and points along it; a stroke with no arrowheads is unchanged.
- **9** — the exported PDF holds the document's pages; an empty document is refused rather than exported empty.
- **10** — a live-painted face takes the paint it was given and keeps its own fill; a face with no paint is refused.
- **11** — tracing a picture returns paths, and the paths are the trace and not the picture; a picture that cannot be read is refused.
- **12** — a gradient mesh paints a gradient across its mesh; a mesh with fewer than two rows or columns is refused.
- **13** — a pattern fill tiles the objects it names; a fill naming an object that is not on the canvas is refused.
- **14** — slices cut the document apart and each slice is written out on its own; a slice naming nothing is refused.
- **15** — an envelope bends a shape through its mesh; a shape with no envelope is refused.

## 5. What it does not do

Nothing is refused. Every row the survey held back is an arc of the second
wave, and the six models they bring are named here so the plan reads whole: a
face inside a path (live paint), a picture read from the image battery (image
trace), a mesh beside the geometry (gradient mesh), a fill that names other
objects (object patterns), a cut of the document (slices), and a mesh a shape
is bent through (envelope distort).

One line is worth keeping from the first reading of the survey: the image
trace's raster is the **image battery's** picture, and the vector battery's arc
reads it. The trace is a vector arc, and the picture is not copied into it.

## 6. The open questions

No upstream crate depends on craftkit, and that is the whole of the dependency
relation. But the survey did not read every upstream crate, so a capability
sitting in one it never opened may already be ported somewhere else; the
survey's "absent" verdicts are about the battery as the survey found it, not a
proof.

The battery's command list was extracted by pattern rather than from a registry
dump, so the list may be missing a command the battery already has.

The upstream's interface crates were not read, so the upstream's own way of
naming these capabilities is unknown.

The dimension-style, hatch-pattern and live-paint samples give no true sizes,
so nothing about scale can be carried across from them.

The survey did not decide whether the pathfinder boolean should take a whole
selection or stay two-path. It flagged the gap and left the answer open;
arc 2 assumes the selection, and that assumption is a question for whoever
picks the arc up.

The upstream's own gap list and roadmap were not read, so this plan does not
know which of the fifteen capabilities the upstream itself considers unfinished.

The second wave leaves open the shape of its six models: what a face is inside
a path, what a mesh holds, what a pattern fill names, how a slice is cut, and
where the trace's picture comes from. Each arc's rule says what the model is,
and the exact form of each model is the first thing whoever picks the arc up
has to settle.
