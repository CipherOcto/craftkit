# The solidcraft plan: what the upstream has and this tree does not

The upstream is a parametric solid modeller: its document is a three-dimensional
model that a feature timeline rebuilds whenever a parameter changes. This tree
has no three-dimensional model at all, so the plan is small, and it says so
rather than pretending otherwise.

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
and they are named in §5.

Two of the rows are **partial**: the cad battery already records constraints
as facts and reports whether they hold, and it already has dimensions, tables
and a drawing. Neither is close enough to the upstream to be an arc today,
because both need a body to work on.

## 2. The arcs

| arc | rule | from | to | module | command | test | section |
|---|---|---|---|---|---|---|---|
| A | a document is written down without being asked, and the write is whole or nothing | `docs/plans/solidcraft.md` | `craftkit-sdk` | `src/autosave.rs` | `doc.autosave.write` | `crates/craftkit-sdk/tests/autosave.rs` | §1260 |
| B | the recoverable saves are listed, and each one is either kept or discarded | `docs/plans/solidcraft.md` | `craftkit-sdk` | `src/recovery.rs` | `doc.recovery.list` | `crates/craftkit-sdk/tests/recovery.rs` | §1261 |

**These two arcs are the only thing this tree can take from a solid modeller
today.** The upstream's autosave and its recovery list are properties of a
design file, and this tree already has a document per battery and a session
that runs commands against it, so both arcs are a service the session offers
rather than a model the tree has to grow first.

**And there are only two arcs because the rest of the upstream is a model, not
a command.** Every other row in §1 — the feature timeline, the solver, the
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

## 3. The order, and what each arc ships

| arc | ships | cost |
|---|---|---|
| A | the autosave — a document written to disk on its own, and a write that is never half a file | small — a bounded write of the document, and an atomic replace of the file |
| B | the recovery list — what can be recovered, and the discard | small — a listing of the saved copies, and one command that throws one away |

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

## 4. The tests

The house rule: the crate's own rule has a test.

| arc | the test that must exist |
|---|---|
| A | the autosave writes the document and the written document reads back the same; a write that is interrupted leaves the previous file whole, and never a half file; a path that cannot be written is refused rather than swallowed |
| B | the recovery list names every saved copy that is there and nothing else; a copy that is discarded is gone from the list, and the others are untouched; a copy that is missing is reported as missing rather than as an error that stops the listing |

## 5. What it does not do

The survey marked ten capabilities **LATER**, and this plan does not take
them. Each one is named here with the reason it is not now.

- **The parametric feature timeline and the solid model** — the tree has no
  three-dimensional document, so there is nothing to rebuild.
- **The solving constraint solver** — the cad battery records constraints as
  facts and reports whether they hold; a solver that moves geometry needs the
  solid model first.
- **Parameters and configurations with units** — craftkit has arithmetic but
  no units and no expression language, and both are a language of their own.
- **STEP import and export** — a neutral format for solids, and there are no
  solids in the tree to write.
- **IGES 5.3 import and export** — the same, for an older neutral format.
- **Mesh interchange, and the mesh-to-solid kernel** — a mesh format is easy
  and the kernel that turns a closed mesh back into a solid is not; the kernel
  is the whole of the work and it needs the solid model.
- **Assemblies, joints and motion studies** — components, occurrences,
  contacts, interference and mass properties are all properties of a
  three-dimensional assembly.
- **Sheet metal** — flanges, hems, unfolding and flat patterns are all
  operations on a solid.
- **Drawings taken from a solid** — the cad battery has dimensions, tables and
  a drawing, but nothing projects a three-dimensional body onto a sheet; that
  projection is the missing piece and it needs the body.
- **Materials, the appearance library and the render workspace** — a render
  workspace needs a scene and a body to light.
- **Three-dimensional navigation hardware** — the camera in the tree is a
  single planar view; six degrees of freedom needs a view in three
  dimensions.

Four further capabilities are **out of scope** rather than later, and the
reason is the same for each of them.

- **The headless command line and the agent server** — craftkit has the
  terminal door and the agent door already, and they drive the same registry.
- **The plugin runtime** — craftkit has the plugin door already, and a
  separately distributed module that adds commands is what it is for.
- **The upstream's own file container** — craftkit has its own document format
  per battery, and a second container would be a second format for the same
  thing.
- **Board and PCB files, and design-file signing** — a different domain
  entirely, and it belongs to a battery of its own rather than to this plan.

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
