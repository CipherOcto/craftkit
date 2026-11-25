# The cadcraft plan: what the upstream has and the cad battery does not

The upstream is a drawing engine whose commands cover entities, dimensions,
modification, regions, tables and export. It feeds the `craftkit-plugin-cad`
battery, the one battery in this tree that draws. This plan names **ten arcs**,
one per gap the survey found, and reserves README sections **§1214 to §1223**,
one section per arc, in the order the arcs are written here.

## 0. The rule

Every capability the upstream has that the cad battery lacks is ported as a
command in the battery, and never as a special case in a front end.

## 1. What the survey found

The survey compared the upstream's command surface against the cad battery's,
and it read both. Its findings are the rows below.

| capability | upstream path:line | craftkit's state | verdict |
|---|---|---|---|
| Dimension kinds beyond length (aligned, angular, radius, diameter) | `crates/engine/src/cmd/annotate.rs:39` | partial (only length is measured) | PORT |
| Block attributes | `crates/engine/src/cmd/blocks.rs:54` | absent | PORT |
| Rectangle entity | `crates/engine/src/cmd/draw.rs:62` | absent | PORT |
| Polygon entity | `crates/engine/src/cmd/draw.rs:67` | absent | PORT |
| Spline entity | `crates/engine/src/cmd/draw.rs:95` | absent | PORT |
| Trim and extend | `crates/engine/src/cmd/modify.rs:89` | absent | PORT |
| Offset of a curve | `crates/engine/src/cmd/modify.rs:84` | absent | PORT |
| Boolean region operations (union, subtract, intersect) | `crates/engine/src/cmd/inquiry/regions.rs:22` | absent | PORT |
| Table columns | `crates/engine/src/cmd/table.rs:36` | partial (rows only) | PORT |
| Plot to PDF / export PDF | `crates/engine/src/cmd/layout.rs:83` | absent | PORT |

The survey read the upstream's command modules and compared them with the
battery the tree has today. Its finding is that the battery draws lines,
circles, arcs, ellipses and polylines, measures them, moves and removes them,
and reads and writes its tables by row — while the upstream also measures in
more than one dimension kind, carries attributes on blocks, draws three entity
kinds the battery has no shape for, trims, extends and offsets, combines
regions, has table columns, and writes a PDF.

The rows marked **LATER** in the survey — hatch and patterned fill, multiline
text, multileader, dimension styles, fillet and chamfer — are **later work**.
They are named in §5 and are not arcs of this plan.

## 2. The arcs

| arc | rule | from | to | module | command | test | section |
|---|---|---|---|---|---|---|---|
| A | a dimension carries a kind, and only length is the default | `docs/plans/cadcraft.md` | `craftkit-plugin-cad` | `src/dimensions.rs` | `cad.dimension.add` | `crates/craftkit-plugin-cad/tests/dimensions.rs` | §1214 |
| B | a block carries attributes, and an attribute has a value | `docs/plans/cadcraft.md` | `craftkit-plugin-cad` | `src/groups.rs` | `cad.block.attribute.add` | `crates/craftkit-plugin-cad/tests/blocks.rs` | §1215 |
| C | a rectangle is a drawn entity with its own geometry | `docs/plans/cadcraft.md` | `craftkit-plugin-cad` | `src/curves.rs` | `cad.entity.rect.add` | `crates/craftkit-plugin-cad/tests/curves.rs` | §1216 |
| D | a polygon is a drawn entity with its own geometry | `docs/plans/cadcraft.md` | `craftkit-plugin-cad` | `src/polygons.rs` | `cad.entity.polygon.add` | `crates/craftkit-plugin-cad/tests/polygons.rs` | §1217 |
| E | a spline is a drawn entity with its own geometry | `docs/plans/cadcraft.md` | `craftkit-plugin-cad` | `src/splines.rs` | `cad.entity.spline.add` | `crates/craftkit-plugin-cad/tests/splines.rs` | §1218 |
| F | a curve is trimmed or extended by a command, never by editing its geometry | `docs/plans/cadcraft.md` | `craftkit-plugin-cad` | `src/modify.rs` | `cad.entity.trim` | `crates/craftkit-plugin-cad/tests/modify.rs` | §1219 |
| G | a curve is offset by a command, and the offset is the requested distance | `docs/plans/cadcraft.md` | `craftkit-plugin-cad` | `src/offset.rs` | `cad.entity.offset` | `crates/craftkit-plugin-cad/tests/offset.rs` | §1220 |
| H | regions are combined by a command, and the result is a region | `docs/plans/cadcraft.md` | `craftkit-plugin-cad` | `src/regions.rs` | `cad.region.boolean` | `crates/craftkit-plugin-cad/tests/regions.rs` | §1221 |
| I | a table has columns, and a column is inserted and removed like a row | `docs/plans/cadcraft.md` | `craftkit-plugin-cad` | `src/tables.rs` | `cad.table.insert.column` | `crates/craftkit-plugin-cad/tests/tables.rs` | §1222 |
| J | a drawing is written as a PDF by the battery's own export door | `docs/plans/cadcraft.md` | `craftkit-plugin-cad` | `src/pdfexport.rs` | `cad.export.pdf` | `crates/craftkit-plugin-cad/tests/pdfexport.rs` | §1223 |

These ten are the gaps the survey found and called **PORT**, and they are the
ones a drawing user meets first: the entities that cannot be drawn, the
operations that cannot be applied, the table shape that is only half there,
and the one export the battery lacks. Arc A and Arc I are **partial** gaps —
the battery already measures length and already has tables by row — so those
two arcs extend what is there rather than add beside it; the other eight are
absent today and land as new commands.

## 3. The order, and what each arc ships

| arc | ships | cost |
|---|---|---|
| A | the dimension kinds — aligned, angular, radius, diameter — on the dimension command | medium — four kinds, and each needs its own measurement and text placement |
| B | block attributes, read and written | medium — an attribute is a value on a block, and the value needs a home in the document |
| C | the rectangle entity | small — a closed, four-cornered shape |
| D | the polygon entity | medium — a closed shape with an arbitrary number of sides |
| E | the spline entity | medium — a curve through control points, which needs its own geometry |
| F | trim and extend | medium — the intersections must be computed, not approximated |
| G | the offset of a curve | medium — the offset is a new curve, and it must be a curve the rest of the battery understands |
| H | the boolean region operations | large — three operations, each a region algorithm |
| I | table columns | small — the table already has rows, and a column is the same shape of edit |
| J | the PDF export | medium — a document format is written from the drawing, and nothing in the tree writes one today |

The order is the order of the survey's rows, and it is also the order of
increasing consequence: the entity arcs come first because every later arc
operates on entities, and the entity kinds must exist before a spline can be
trimmed or a region can be combined. Arc A comes first because the dimension
kinds are the partial gap closest to complete, and closing it is the cheapest
way to see that the arc shape works. Arc H comes last of the geometry work
because it is the largest, and Arc I and Arc J come after it because a table's
columns and a PDF export do not depend on the geometry work at all and are
better shipped once the battery is stable.

## 4. The tests

The house rule: the crate's own rule has a test.

| arc | the test that must exist |
|---|---|
| A | each of the four dimension kinds measures what its name says, and a kind the battery does not know is refused |
| B | an attribute's value is read back unchanged, and an attribute on a block that is not there is refused |
| C | a rectangle's four corners are the ones asked for, and its area is the rectangle's |
| D | a polygon's edges close, and a polygon with too few points is refused |
| E | a spline passes through its control points, and a degenerate control set is refused |
| F | a trim removes the part that was asked to go, and a trim that meets no curve is refused |
| G | an offset curve stands the requested distance from the original, and a zero distance is refused |
| H | a union of two overlapping regions is one region, a subtract removes what it should, and an intersect keeps only the overlap |
| I | a column's cells are the column's, and inserting and removing a column leaves the rest of the table alone |
| J | the exported file is a PDF, and an empty drawing is refused rather than written as a broken file |

## 5. What it does not do

The survey marked five capabilities **LATER**, and this plan does not take
them.

- **Hatch and patterned fill** — the upstream's fill is a pattern library and
  a fill entity; this is a new model, not a missing command.
- **Multiline text** — the upstream's form carries paragraphs; this is a new
  model for text, not a missing command.
- **Multileader** — the upstream's form needs leader geometry and corner
  intersection; this is new geometry work.
- **Dimension styles** — the upstream's dimension styles are a variable list
  whose true size the survey could not read; this is a new model for styling.
- **Fillet and chamfer** — the upstream's needs corner intersection and arc
  geometry, neither of which the battery has today.

Each of the five is a new model or needs geometry the battery does not have,
which is why the survey put them behind the ten that are ports. They are named
here so that their absence is a decision and not an oversight.

## 6. The open questions

- No upstream crate depends on craftkit, so no upstream capability is already
  supplied by craftkit — but the survey did not read every upstream crate, so
  a capability living in one it never opened may already be ported elsewhere.
- The survey extracted craftkit's command lists by pattern rather than from a
  registry dump, so a command defined in an unusual shape could be missing
  from the list it compared against.
- The upstream's dimension-style variable list and its hatch pattern library
  were sampled rather than read whole, so their true sizes are unknown.
- The upstream's user-interface crates were not read, so behaviour that
  exists only in the interface may not be in the engine commands the table
  lists.
- The upstream's own gap list was only partly read, so the ranking of the
  upstream's own priorities is only partly known.

What would settle them: a registry dump of the tree's commands settles the
second, a full read of the upstream's style variables and pattern library
settles the third, and reading the upstream's interface crates and the rest
of its gap list settles the fourth and the fifth. The first is settled by
reading the upstream crates the survey did not open — which is also what would
confirm that none of the ten arcs lands twice.
