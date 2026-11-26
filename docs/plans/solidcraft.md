# The solidcraft plan: what the upstream has and this tree does not

The upstream is a parametric solid modeller: its document is a three-dimensional
model that a feature timeline rebuilds whenever a parameter changes. The family
is **fully ported**: every capability the upstream has is in scope, and any rule
that would have excluded one is changed so that it is not. This plan holds
**14 arcs** and reserves **§1262 to §1371**.

The first wave is the autosave and the recovery list — the two arcs a tree
without a three-dimensional model can honestly carry today. The second wave is
the whole upstream: the solid model itself, the solver, the parameters, the
neutral formats, the mesh kernel, the assemblies, the sheet metal, the drawings
taken from a solid, the materials and the render workspace, the three-dimensional
navigation, and the board files.

## 0. The rule

Everything this tree takes from a solid modeller is a command, and never a
special case in a front end.

## 1. What the survey found

The survey compared the upstream's capabilities against this tree, and read
both. Its findings are the rows below.

| capability | upstream path:line | craftkit's state | verdict |
|---|---|---|---|
| Autosave and crash recovery of a design, with recovery list and discard | `docs/features.md:436` | absent | PORT |
| Parametric feature timeline that rebuilds on every parameter change | `crates/doc/src/document.rs:949`, `docs/features.md:147` | absent (craftkit has no 3-D model at all) | LATER |
| Sketch profiles with a solving constraint solver | `docs/capabilities.md:28`, `crates/sketch/src/equation.rs` | partial — the cad battery records constraints as facts and reports whether they hold, but never moves geometry | LATER |
| Parameters and configurations with unit-aware expressions and ordered rules | `crates/doc/src/config.rs:41`, `docs/features.md:336` | partial — craftkit has arithmetic, no units and no expression language | LATER |
| STEP import and export | `docs/capabilities.md:60`, `docs/features.md:404` | absent | LATER |
| IGES 5.3 import and export | `docs/capabilities.md:62` | absent | LATER |
| Mesh interchange (3MF, STL, OBJ, GLB) plus reconstruction of a closed mesh as a parametric solid | `docs/capabilities.md:62`, `docs/features.md:757` | absent | LATER |
| Assemblies: components, occurrences, joints, motion studies, contact sets, interference, mass properties | `crates/doc/src/document.rs:3269`, `crates/doc/src/joints.rs:528`, `docs/features.md:312` | absent | LATER |
| Sheet metal: flanges, hems, fold/unfold, flat patterns, bend tables, DXF outlines | `docs/capabilities.md:53`, `docs/features.md:242` | absent | LATER |
| Drawings taken from a 3-D model: projected views, dimensions, balloons, BOM tables | `docs/capabilities.md:14`, `docs/features.md:556`, `ROADMAP.md:34` | partial — the cad battery has dimensions, tables and a drawing, but nothing projects a three-dimensional body onto a sheet | LATER |
| Materials and appearance library plus a render workspace with lighting and image export | `docs/capabilities.md:78`, `docs/features.md:896` | absent | LATER |
| 6-DOF 3-D navigation hardware | `crates/space-mouse/`, `docs/features.md:1053` | absent | LATER |
| Headless CLI and MCP server driving the very same command registry | `docs/features.md:404`, `crates/mcp-server/` | present — craftkit has the agent door and the terminal door | SKIP |
| Plugin runtime where a separately distributed module adds commands | `README.md`, `crates/craft-plugin-wasm/` | present — craftkit has the plugin door | SKIP |
| Its own design-file container as a versioned envelope | `crates/format/src/container.rs` | no home | SKIP |
| Board and PCB file import and design-file signing | `docs/features.md:404` | absent | SKIP |

One row is a **PORT**, and it is the row about a design being written down
without being asked and read back after a crash. That row is **the only thing
this tree can take from a solid modeller today**, and it is taken as two arcs:
the autosave, and the recovery list.

The rest of the upstream is a **model**, not a command. A feature timeline
that rebuilds on every parameter change needs a three-dimensional document; the
tree has no such document, so there is nothing for the timeline to rebuild.
The sketch solver, the unit-aware expressions, the neutral interchange
formats, the mesh-to-solid kernel, the assemblies and their joints, the sheet
metal, the drawings taken from a solid, the materials and the render
workspace, and the three-dimensional navigation hardware are all later work,
and they are named in §2 as the second wave.

Two of the rows are **partial**: the cad battery already records constraints
as facts and reports whether they hold, and it already has dimensions, tables
and a drawing. Neither is close enough to the upstream to be an arc before the
body exists, because both need a body to work on.

## 2. The arcs

| arc | rule | from | to | module | command | test | section |
|---|---|---|---|---|---|---|---|
| A | a document is written down without being asked, and the write is whole or nothing | `docs/plans/solidcraft.md` | `craftkit-sdk` | `src/autosave.rs` | `doc.autosave.write` | `crates/craftkit-sdk/tests/autosave.rs` | §1262 |
| B | the recoverable saves are listed, and each one is either kept or discarded | `docs/plans/solidcraft.md` | `craftkit-sdk` | `src/recovery.rs` | `doc.recovery.list` | `crates/craftkit-sdk/tests/recovery.rs` | §1263 |

**These two arcs are the only thing this tree can take from a solid modeller
today.** The upstream's autosave and its recovery list are properties of a
design file, and this tree already has a document per battery and a session
that runs commands against it, so both arcs are a service the session offers
rather than a model the tree has to grow first.

**And the first wave is two arcs because the rest of the upstream is a model,
not a command.** Every other row in §1 — the feature timeline, the solver, the
parameters and units, the interchange formats, the mesh kernel, the
assemblies, the sheet metal, the drawings from a solid, the materials and the
render workspace, the navigation hardware — is a capability that needs a
three-dimensional model in the document before it can be a command at all. An
arc that landed one of those today would land a command with nothing to
operate on, and the house rule is that a command is the whole of a
user-visible behaviour.

The two arcs land in the SDK, because the autosave belongs to the **session**
and not to an application: every door gets it, and the recovery list is read
by whichever door a person is looking at. Arc A writes; Arc B reads what was
written and lets it be kept or discarded.

**The second wave: the whole upstream.**

| arc | rule | from | to | module | command | test | section |
|---|---|---|---|---|---|---|---|
| C | a body is the result of a list of features, rebuilt from that list and not stored as a shape | `docs/plans/solidcraft.md` | `craftkit-plugin-solid` | `src/body.rs` | `solid.body.rebuild` | `crates/craftkit-plugin-solid/tests/body.rs` | §1360 |
| D | a sketch profile's constraints are solved, and the geometry moves to satisfy them | `docs/plans/solidcraft.md` | `craftkit-plugin-solid` | `src/sketch.rs` | `solid.sketch.solve` | `crates/craftkit-plugin-solid/tests/sketch.rs` | §1361 |
| E | a parameter carries its unit, and a configuration is a set of parameters applied in order | `docs/plans/solidcraft.md` | `craftkit-plugin-solid` | `src/params.rs` | `solid.config.evaluate` | `crates/craftkit-plugin-solid/tests/params.rs` | §1362 |
| F | a body is written as STEP and read back as STEP | `docs/plans/solidcraft.md` | `craftkit-plugin-solid` | `src/step.rs` | `solid.step.import`, `solid.step.export` | `crates/craftkit-plugin-solid/tests/step.rs` | §1363 |
| G | a body is written as IGES 5.3 and read back as IGES 5.3 | `docs/plans/solidcraft.md` | `craftkit-plugin-solid` | `src/iges.rs` | `solid.iges.import`, `solid.iges.export` | `crates/craftkit-plugin-solid/tests/iges.rs` | §1364 |
| H | the mesh containers are read into one neutral mesh model, and the solid battery reads that model | `docs/plans/solidcraft.md` | `craftkit-mesh` | `src/lib.rs` | `mesh.container.read` | `crates/craftkit-mesh/tests/mesh.rs` | §1365 |
| H | a closed mesh is turned back into a body that a feature list can rebuild | `docs/plans/solidcraft.md` | `craftkit-plugin-solid` | `src/kernel.rs` | `solid.mesh.reconstruct` | `crates/craftkit-plugin-solid/tests/kernel.rs` | §1365 |
| I | an assembly is a list of occurrences of components, and it carries its joints, its studies, its contacts, its interference and its mass properties | `docs/plans/solidcraft.md` | `craftkit-plugin-solid` | `src/assembly.rs` | `solid.assembly.build` | `crates/craftkit-plugin-solid/tests/assembly.rs` | §1366 |
| J | a sheet is folded and unfolded, and the flat pattern carries its bend table and its outline | `docs/plans/solidcraft.md` | `craftkit-plugin-solid` | `src/sheet.rs` | `solid.sheet.unfold` | `crates/craftkit-plugin-solid/tests/sheet.rs` | §1367 |
| K | a drawing is a projection of a body onto a sheet, and the cad battery holds the drawing, the dimension and the table | `docs/plans/solidcraft.md` | `craftkit-plugin-cad` | `src/projection.rs` | `solid.drawing.project` | `crates/craftkit-plugin-cad/tests/projection.rs` | §1368 |
| L | a render workspace is a scene and a light over a body, and a picture is the image battery's | `docs/plans/solidcraft.md` | `craftkit-plugin-image` | `src/render.rs` | `image.render.export` | `crates/craftkit-plugin-image/tests/render.rs` | §1369 |
| M | a six-degree camera is beside the planar one, and both are one camera type with its own caps | `docs/plans/solidcraft.md` | `craftkit-sdk` | `src/camera.rs` | `camera.sixdof.orbit` | `crates/craftkit-sdk/tests/camera.rs` | §1370 |
| N | a board file is read into the cad battery's board | `docs/plans/solidcraft.md` | `craftkit-plugin-cad` | `src/board.rs` | `cad.board.import` | `crates/craftkit-plugin-cad/tests/board.rs` | §1371 |

**The second wave is the whole upstream, and it is a second wave because it
needs a document the tree does not have.** The first wave's two arcs are a
service the session offers; this wave grows the **model** they and everything
else operate on. The survey's "later" verdicts were judgements about **size**,
not about **scope**: each row was too large for a first arc, and none of them
was out of scope. So every one of them is an arc here, in the order the
upstream's own document has them — the model, then what the model is drawn
with, then the neutral formats, then the assemblies and the sheet metal, then
the drawings and the pictures and the navigation.

**And this wave brings the tree its first three-dimensional document.** Arc C
lands a **new battery**, `craftkit-plugin-solid`, and an arc that adds a
battery carries **three more edits** on top of its own: the workspace's
`Cargo.toml`, the row in the layer table, and the row in `docs/architecture.md`.
Everything that needs a body lands in that battery; the four rows that do
not land there — the drawings (K), the render workspace (L), the navigation
(M) and the board reader (N) — land in the battery that already owns the
drawing, the picture and the
camera, because a second answer in the wrong battery is a second truth.

**Arc H is two rows because it is two crates, and only one of them is
written here.** The mesh containers are the **shared library's** arc — it is
the craft-libs plan's arc (`docs/plans/craft-libs.md`), and this plan names
it only so that the solid battery has something to read; it is not duplicated
here. The kernel that turns a closed mesh back into a parametric solid is the
solid battery's own work, and it is the hard half of the row.

## 3. The order, and what each arc ships

| arc | ships | cost |
|---|---|---|
| A | the autosave — a document written to disk on its own, and a write that is never half a file | small — a bounded write of the document, and an atomic replace of the file |
| B | the recovery list — what can be recovered, and the discard | small — a listing of the saved copies, and one command that throws one away |
| C | the solid battery and the body — a feature list, and a body rebuilt from it | large — a new crate, a new document type, and the geometry kernel under it |
| D | the solving constraint solver — a sketch whose geometry moves | large — it needs the body and the solver together |
| E | parameters, units and configurations — an expression language with units | large — it needs the body's parameters to exist |
| F | STEP import and export | large — a neutral format, and it needs the body |
| G | IGES 5.3 import and export | large — the same, for an older format |
| H | the mesh containers and the mesh-to-solid kernel | large — the containers are the shared library's arc; the kernel is the work |
| I | assemblies, joints, studies, contacts, interference and mass properties | large — all of it is a property of a three-dimensional assembly |
| J | sheet metal — flanges, hems, unfolding, flat patterns, bend tables, outlines | large — every operation is on a solid |
| K | the projection a drawing was missing | large — it needs the body to project |
| L | materials, appearances and the render workspace | large — it needs a scene and a body to light |
| M | the six-degree camera | medium — a camera with its own caps, beside the planar one |
| N | board and PCB file import | medium — a reader into the cad battery |

**Arc A comes first because Arc B reads what Arc A writes.** The recovery list
is a list of autosaved copies, so there is nothing to list until the autosave
has written one. Arc B then adds the discard, because a recovery list whose
entries can never be thrown away is a list that grows for ever, and a growing
list is a leak.

**And the two arcs are deliberately small.** Neither one needs a solid: the
autosave writes the document the tree already has, and the recovery list
reports on it. That is what makes them the right first arcs — they are the
parts of a modeller's crash safety that a tree without a three-dimensional
model can honestly carry.

**The second wave is one wave because the solid model is large and everything
that needs it is large.** Arc C is the model; the arcs after it are the
capabilities the model carries, and each of them is large for the same reason
— it is work on a three-dimensional body, not on a record. The small-ish
rows in the wave are the ones that need no body at all: the navigation, which
is a camera, the board reader, which is a file format, and the mesh
containers, which belong to the shared library. And Arc C is still
first among them, because a command with nothing to operate on is not a
command.

## 4. The tests

The house rule: the crate's own rule has a test.

| arc | the test that must exist |
|---|---|
| A | the autosave writes the document and the written document reads back the same; a write that is interrupted leaves the previous file whole, and never a half file; a path that cannot be written is refused rather than swallowed |
| B | the recovery list names every saved copy that is there and nothing else; a copy that is discarded is gone from the list, and the others are untouched; a copy that is missing is reported as missing rather than as an error that stops the listing |
| C | the body is the result of the feature list and not the shape: the same list rebuilds the same body, and changing a feature changes the body; a feature list that cannot be built is refused, not half-built; the body is never stored as a shape |
| D | a sketch whose constraints hold is solved to the geometry it describes; a sketch whose constraints cannot all hold is **refused**, and the solver does not move the geometry to satisfy some of them and drop the rest |
| E | an expression with units of different kinds is refused; a configuration applies its parameters in order, and the last rule that sets a parameter is the one that holds; a parameter that is out of its bound is refused rather than clamped |
| F | a body written as STEP reads back as the same body; a STEP file that is not a body is refused; an export that cannot represent the body is refused rather than written with a hole in it |
| G | a body written as IGES 5.3 reads back as the same body; an IGES file of the wrong version is refused |
| H | every mesh container the upstream names is read into the one neutral mesh model; a closed mesh becomes a body whose feature list rebuilds it, and an open mesh is refused; a mesh that is not closed is not silently closed |
| I | an assembly holds its occurrences and its joints, and its motion study moves the occurrences it names; a joint that cannot be satisfied is refused; the interference check reports a real overlap and the mass properties match the geometry |
| J | a sheet unfolds into its flat pattern and folds back into the same body; the bend table names every bend and its allowance; the flat pattern carries its outline and the outline reads back as the sheet |
| K | a body projects onto a sheet as the views the drawing names, and the projection is the drawing's own; a dimension on a projected view measures the body and not the projection; a body that cannot be projected is refused |
| L | the scene holds its bodies and its lights, and the exported picture is the picture of that scene; a scene with no light is refused rather than rendered black; a material that is not in the appearance library is refused |
| M | the camera holds its own caps, and a zoom past a cap is refused rather than clamped; the six-degree camera keeps the planar one's behaviour where the two agree; a rotation with no reference frame is refused |
| N | a board file is read into the cad battery's board; a file that is not a board is refused; the board that is read back is the board that was in the file |

## 5. What it does not do

Almost nothing is refused now. Three things are, and each is refused for a
reason of the house's shape.

- **The upstream's own design-file container** is refused: craftkit has its
  own document format per battery, and a second container for the same thing
  would be a second place the same answer lives.
- **Design-file signing** is not here: it is the **pdf plan's** arc
  (`docs/plans/pdfcraft.md`, the signing row of its second wave), because one
  answer lives in one place.
- **The headless command line and the agent server, and the plug-in runtime**,
  are **parity**: craftkit has the terminal door, the agent door and the
  plug-in door already, and they drive the same registry. Parity is not a gap
  and not an arc.

Everything else the survey marked **LATER** is an arc in §2, and the survey's
**SKIP** row about board files is split: the board reader is an arc, and the
signing is the pdf plan's.

## 6. The open questions

- **The survey did not read the upstream's geometry kernel.** Its size
  estimates for the solid model come from the documentation and the feature
  list, and not from the kernel's code; the kernel is the part of the work
  nobody has costed.
- **The survey read the cad battery's constraint and library modules, but not
  all nineteen of its modules.** It cannot say whether some three-dimensional
  notion hides in the curve, measurement or array modules, and if one does,
  the later verdicts change.
- **The upstream's recent work is solid-modelling work**, so it inherits the
  later verdict: the newest capability in the upstream is the one this plan
  cannot take.
- **The proposed command ids follow the tree's `<area>.<verb>.<thing>`
  shape, but none of them exists today.** The names here are proposals, and a
  different spelling may be the right one once the arcs are written.

**The second wave leaves two shapes open.** The first is the **shape of the
solid model**: whether the feature list is a flat list of features or a tree
of them, whether a body carries its own parameters or reads them from the
configuration, and what a body is when it is empty — the same question the
tree already asks of a node with no place. The second is the **shape of the new
battery**: what `craftkit-plugin-solid` holds that is the solid battery's own,
and what it shares with the cad, image and mesh batteries, given that a
plugin may not depend on another plugin. Both are decided when Arc C is
written, and both are the first thing a reader of that arc will ask.
