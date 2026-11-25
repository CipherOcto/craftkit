# The vectorcraft proof of concept plan: what it names and this tree does not do

This plan is the landing site for two arcs that come out of a survey of a
proof-of-concept sketch in the upstream project. The sketch is a self-contained
experiment on a local branch, **uncommitted**, and it does not build on this
machine. So the plan names what the sketch is worth, says plainly that the
sketch proves nothing about rendering, hit testing, path geometry, a stable
plugin interface or the migration of real files, and reserves two README
sections for the work that is worth doing here.

## 0. The rule

A file format is reached through a registry keyed by its extension, and the
lists of formats a front end offers are read from that registry rather than
written down.

## 1. What the survey found

| capability | upstream path:line | craftkit's state | verdict |
|---|---|---|---|
| Importers/exporters as a registry keyed by file extension, so open/save lists come from the plugins | `v2-poc/crates/vc2-core/src/io.rs:58-140` | absent (craftkit has per-format commands such as `cad.import.svg`, `grid.export.csv`, and no extension-keyed registry; the window has no file dialogs at all) | PORT |
| A plugin owns a pointer gesture (drag) that ends in a command | `v2-poc/crates/vc2-core/src/tool.rs:22-60` | partial (the tool derive names a command only; the drag lives in the front end) | LATER |
| A typed body per node kind, parsed by the plugin that owns the kind | `v2-poc/crates/vc2-core/src/node.rs:17-101` | partial (node data is JSON in the document; the plugin reads it) | LATER |
| A widget model built from the panel spec | `v2-poc/crates/vc2-ui/src/lib.rs:1-107` | unsure (the window builds panel rows from a panel spec, but the survey did not trace whether the widgets are generic or per panel) | LATER |
| One projection generating menus, toolbar, tool schemas and node schemas | `v2-poc/crates/vc2-core/src/projection.rs:16-180` | partial (the projection has menu, schema, JSON schema, catalog, entries and manifest, but no node-schema or toolbar projection) | LATER |
| A generic contribution registry with id, duplicate and malformed-id checks | `v2-poc/crates/vc2-core/src/command.rs:99-165` | present | SKIP |
| Plugin-owned painting and hit test per node kind | `v2-poc/crates/vc2-core/src/render.rs:38-113` | present | SKIP |
| Layer graph checked at start-up, not in a `match` | `v2-poc/crates/vc2-core/src/ecosystem.rs:92-181` | present | SKIP |
| Out-of-process plugin registering the same spec after start-up | `v2-poc/crates/vc2-plugin-host/src/lib.rs:1-359` | present | SKIP |
| Format and migration version chain with a hole check | `v2-poc/crates/vc2-core/src/format.rs:79-141` | present | SKIP |
| Undo only when a command mutated, was undoable and really changed | `v2-poc/README.md:44` | present | SKIP |
| Crash guard: a panicking command rolls the document back | `v2-poc/README.md:45` | present | SKIP |
| A raw-JSON parameter kind in the SDK | `v2-poc/README.md:35` | present | SKIP |
| A software raster backend behind a canvas seam | `v2-poc/crates/vc2-render/src/lib.rs:1-151` | present | SKIP |
| A geometry crate the core never names | `v2-poc/crates/vc2-geometry/src/lib.rs:1-256` | present | SKIP |

One row is a PORT, and it is the only one: the registry of importers and
exporters, keyed by file extension. Everything else is either later work or
work this tree has already done.

The proof of concept is best read as a catalogue of patterns this tree has
already absorbed. The contribution registry with its id, duplicate and
malformed-id checks, the derived painting and hit test, the layer graph
checked at start-up, the out-of-process plugin registering the same spec, the
format and migration chain with its hole check, the undo rule, the crash guard,
the raw-JSON parameter kind, the software rasteriser behind a canvas seam and
the separated geometry crate are all present here already, in this tree's own
shape. Reading the proof against this tree is how the survey decided the
verdicts, and most of the proof's table is already this tree's table.

What the proof is **not** is a base to build on. It is a sketch, on a local
branch, uncommitted, whose own README says it proves no real rendering, no hit
testing, no path geometry, no stable plugin interface and no migration of real
files. Its workspace points at a checkout that does not exist on this
machine, so it does not build here at all: whether it compiles, and whether
its tests pass, is unknown. The arcs below are therefore **new work in this
tree**, informed by what the sketch names, and not a port of its code.

## 2. The arcs

An arc is three edits: a port-manifest row in `xtask/src/ports.rs`, a claim
row in the README's claim table, and a README section. The two arcs below
reserve **§1270** and **§1271**, in that order, after the README's §1181.

| arc | rule | from | to | module | command | test | section |
|---|---|---|---|---|---|---|---|
| **1** | the importer registry is keyed by file extension, and the open list is read from it | `docs/plans/vectorcraft-poc.md` | `craftkit-sdk` | `src/importers.rs` | `file.import` | `crates/craftkit-sdk/tests/importers.rs` | §1270 |
| **2** | the exporter registry is keyed by file extension, and the save list is read from it | `docs/plans/vectorcraft-poc.md` | `craftkit-sdk` | `src/exporters.rs` | `file.export` | `crates/craftkit-sdk/tests/exporters.rs` | §1271 |

These two arcs because the one PORT row is a single idea with two halves, and
the halves do not land together: the importers answer "what can I open", the
exporters answer "what can I save", and each is a registry of its own with a
test of its own. The two land in the SDK, because a registry that a plugin
fills and a front end reads is L2 work, and the front ends read it without
knowing which plugin wrote it.

The rest of the proof is not arcs. The rows marked LATER — a plugin owning its
own gesture, a typed body per node kind, the widget model, one projection
answering every view — are named in §5 and §6 with the reason each is not
now, and they are not arcs because the tree does not yet know enough about the
window's panels to judge the widget model, and because a gesture and a typed
body are changes to what the tree already does rather than gaps in it. The
rows marked SKIP are work this tree has already done, in this tree's own
shape, and re-doing them here would be a copy, not an arc.

## 3. The order, and what each arc ships

| arc | ships | cost |
|---|---|---|
| 1 | the importer registry in the SDK, keyed by extension, with the existing per-format import commands registered in it | medium — a new registry, the registrations of the commands that already exist, and a new test file |
| 2 | the exporter registry in the SDK, keyed by extension, with the existing per-format export commands registered in it | medium — the same shape as arc 1, on the other side of the file |

Arc 1 lands before arc 2, and arc 2 is written after arc 1's registry exists
so that the two share one idea — one key, one lookup, one refusal for an
unknown extension — and so that the second arc is a copy of the first with the
direction reversed, not a new design. Nothing in the tree changes for a user
until the two are in: the registries are read by the doors, and the doors read
them once the second arc is there.

## 4. The tests

- Arc 1: `crates/craftkit-sdk/tests/importers.rs` — the registry lists every
  registered importer and nothing else, an extension that is registered twice
  is refused, a malformed extension is refused, and an unknown extension is
  refused rather than faked.
- Arc 2: `crates/craftkit-sdk/tests/exporters.rs` — the registry lists every
  registered exporter and nothing else, an extension that is registered twice
  is refused, a malformed extension is refused, and an unknown extension is
  refused rather than faked.

## 5. What it does not do

The later work, named and not done now:

- **A plugin owning its own pointer gesture.** The tool derive names a
  command; the drag lives in the front end. Moving it would change what the
  front end does, and the survey did not trace how much of the window's
  input handling would move with it.
- **A typed body per node kind.** Node data is JSON in the document today, and
  the plugin reads it. A typed body per kind is a change to the document's
  shape, and nothing in the survey says which kinds would be typed first.
- **The widget model built from the panel spec.** The window builds panel rows
  from a panel spec, but the survey did not trace whether those rows are
  generic or per panel, so the model cannot be judged and a wrong judgement
  would be a wrong arc.
- **One projection answering every view.** The projection has menu, schema,
  JSON schema, catalog, entries and manifest, but no node-schema or toolbar
  projection. Adding those is a change to the projection's contract, not a
  gap, and it wants its own survey.

The work this tree has already done, and that the proof therefore does not
contribute: the generic contribution registry with id, duplicate and
malformed-id checks; the plugin-owned painting and hit test per node kind; the
layer graph checked at start-up rather than in a `match`; the out-of-process
plugin registering the same spec after start-up; the format and migration
version chain with its hole check; the undo rule that fires only when a
command mutated, was undoable and really changed; the crash guard that rolls
the document back when a command panics; the raw-JSON parameter kind in the
SDK; the software raster backend behind a canvas seam; and the geometry crate
the core never names. Each is present here already, and re-doing it would be
a copy of this tree's own code under another name.

## 6. The open questions

- The proof of concept cannot be built or run here — its workspace points at a
  checkout that does not exist on this machine — so whether it compiles at all,
  and whether its tests pass, is unknown. Every verdict in §1 is therefore a
  verdict about the tree, read against the sketch's text.
- The survey did not trace how far the window's panel rows are generic, so the
  widget model could not be judged, and the LATER row for it says so rather
  than guessing.
- The specification's own SDK fixes and conformance suite were not read, so
  the proof's behaviour is known only from its own README and its source.
