# The gridcraft plan: what the upstream has and the grid battery does not

This plan is a reading of the upstream spreadsheet surface against craftkit's grid battery, and a set of arcs for the gap between them. It is a plan for future work: nothing described here exists in this tree today. The plan holds **fifteen arcs** and reserves the README sections **§1235 to §1345**.

## 0. The rule

Every capability the upstream has is ported: it is a command the grid battery
gains, or a command another battery gains, or a model one of them takes on.

## 1. What the survey found

| capability | upstream path:line | craftkit's state | verdict |
|---|---|---|---|
| Chart kinds beyond four (stacked, area, scatter, bubble, radar, histogram, waterfall, funnel, treemap, sunburst, box-whisker, stock, combo) | `gridcraft/crates/model/src/features.rs:493` | partial — the battery accepts only `column, bar, line, pie` | PORT |
| Pivot value summaries, incl. distinct count | `gridcraft/crates/engine/src/cmd/pivot/params.rs:23` | partial — the battery knows six aggregates, no distinct count | PORT |
| Table styles and banding (style name, banded rows/columns, header/first/last column) | `gridcraft/crates/engine/src/cmd/insert.rs:70`, `crates/engine/src/catalog.rs:296` | absent — the battery's table commands are create/read/rename/resize/totals/fields only | PORT |
| Chart design operations: change type, pick source data, quick layout, series colours/outline, shape fill | `gridcraft/crates/engine/src/cmd/extra.rs:74`, `crates/engine/src/cmd/extra/chart.rs:177` | partial — `grid.chart.set` has kind/range/legend/colours, but no type change, no data-range re-pick, no quick layout | PORT |
| Searchable dropdown for validation lists | `gridcraft/crates/engine/src/cmd/data/validation.rs:292` | partial — the battery has list rules but no search of the list | PORT |
| Split panes and saved custom views | `gridcraft/crates/engine/src/cmd/extra.rs:175-176` | partial — the battery has frozen panes, no split, no saved views | PORT |
| Legacy 97-2003 `.xls` workbooks (values, formulas, number formats) | `gridcraft/crates/xlsx/src/xls.rs:1` | absent — `craftkit-xlsx` has the compound-file container but no BIFF reader | LATER |
| XLSB worksheet import (saved values, not formulas) | `gridcraft/crates/xlsx/src/xlsb.rs:1` | absent | LATER |
| ODS worksheet import | `gridcraft/crates/xlsx/src/ods.rs:1` | absent | LATER |
| Form controls: buttons, sliders, spin buttons, and pressing them | `gridcraft/crates/engine/src/cmd/controls.rs`, `crates/engine/src/cmd/insert.rs` | absent | LATER |
| Recorded scripts (macros): record, list, run, save, rename | `gridcraft/crates/engine/src/cmd/macros.rs:27,45` | absent | LATER |
| Python cells (Python mode in the formula bar, results stored in the file) | `gridcraft/crates/engine/src/cmd/python.rs:17` | absent | LATER |
| Equations (floating shape holding source, laid out by a maths engine) | `gridcraft/crates/engine/src/cmd/equation.rs:1-6` | absent — no maths crate in craftkit | LATER |
| Get & Transform: queries, connections, refresh all, reapply | `gridcraft/crates/engine/src/cmd/extra.rs:232`, `crates/engine/src/catalog.rs:198` | absent | LATER |
| Shapes, WordArt, text boxes and ink drawing | `gridcraft/crates/engine/src/cmd/insert/objects.rs`, `crates/engine/src/cmd/draw.rs:11,17` | absent from the grid battery, but craftkit has shape, vector and fx batteries | SKIP |

The shape of the survey is worth saying plainly: the battery is ahead of the upstream on the core spreadsheet surface, so what is missing is chart depth, then summary and style richness, then file formats, then the heavier features that are new models rather than missing commands.

The six PORT rows are therefore the plan's work: chart kinds, pivot summaries, table styles, chart design operations, the searchable validation list, and split panes with saved views.

The LATER rows are each a new model or a new dependency — legacy workbooks and the other workbook formats, form controls, recorded macros, Python cells, equations, external queries — and each of them is an arc of the second wave below rather than a command the battery could simply gain.

The SKIP row is not out of scope: the shapes, the WordArt and the text boxes are the vector battery's work and the ink drawing is the image battery's, and each of them is an arc in that battery.

## 2. The arcs

| arc | rule | from | to | module | command | test | section |
|---|---|---|---|---|---|---|---|
| **A** | a chart kind is a value the battery names, not a drawing the battery draws | `docs/plans/gridcraft.md` | `craftkit-plugin-grid` | `src/charts.rs` | `grid.chart.set` | `crates/craftkit-plugin-grid/tests/charts.rs` | §1235 |
| **B** | a summary is a named aggregate over a pivot's values | `docs/plans/gridcraft.md` | `craftkit-plugin-grid` | `src/pivot.rs` | `grid.pivot.set` | `crates/craftkit-plugin-grid/tests/pivot.rs` | §1236 |
| **C** | a table's look is a style the document carries | `docs/plans/gridcraft.md` | `craftkit-plugin-grid` | `src/table.rs` | `grid.table.style` | `crates/craftkit-plugin-grid/tests/table.rs` | §1237 |
| **D** | changing a chart's design is a command on the chart | `docs/plans/gridcraft.md` | `craftkit-plugin-grid` | `src/chart.rs` | `grid.chart.design` | `crates/craftkit-plugin-grid/tests/charts.rs` | §1238 |
| **E** | a validation list is searched before it is picked | `docs/plans/gridcraft.md` | `craftkit-plugin-grid` | `src/validation.rs` | `grid.validation.list` | `crates/craftkit-plugin-grid/tests/validation.rs` | §1239 |
| **F** | a view is a saved arrangement of the sheet | `docs/plans/gridcraft.md` | `craftkit-plugin-grid` | `src/views.rs` | `grid.view.split` | `crates/craftkit-plugin-grid/tests/views.rs` | §1240 |

Each arc is three edits and no more: a row in the port manifest (`xtask/src/ports.rs`), a claim row in the README, and a section in the README, whose sections are **§1235–§1240** in this order. The test file named in a row is written in the same arc that adds the row, and the rows name the files the arc will create.

**The second wave: the whole upstream.**

| arc | rule | from | to | module | command | test | section |
|---|---|---|---|---|---|---|---|
| **G** | a legacy workbook is read by the shared reader, and the battery's own command hands the file to it — the reader is the **craft-libs plan's** arc (`docs/plans/craft-libs.md`), the command is the battery's, and the same answer does not live in two places | `docs/plans/gridcraft.md` | `craftkit-plugin-grid` | `src/xls.rs` | `grid.open.xls` | `crates/craftkit-plugin-grid/tests/xls.rs` | §1337 |
| **H** | the binary workbook is read by the shared reader, and the battery's own command hands the file to it — the reader is the **craft-libs plan's** arc, the command is the battery's, and the same answer does not live in two places | `docs/plans/gridcraft.md` | `craftkit-plugin-grid` | `src/xlsb.rs` | `grid.open.xlsb` | `crates/craftkit-plugin-grid/tests/xlsb.rs` | §1338 |
| **I** | the OpenDocument workbook is read by the shared reader, and the battery's own command hands the file to it — the reader is the **craft-libs plan's** arc, the command is the battery's, and the same answer does not live in two places | `docs/plans/gridcraft.md` | `craftkit-plugin-grid` | `src/ods.rs` | `grid.open.ods` | `crates/craftkit-plugin-grid/tests/ods.rs` | §1339 |
| **J** | a form control is a button, a slider or a spin button, and pressing it runs the command it carries | `docs/plans/gridcraft.md` | `craftkit-plugin-grid` | `src/controls.rs` | `grid.control.press` | `crates/craftkit-plugin-grid/tests/controls.rs` | §1340 |
| **K** | a recorded macro is stored behaviour, and the battery records it, lists it, runs it, saves it and renames it | `docs/plans/gridcraft.md` | `craftkit-plugin-grid` | `src/macros.rs` | `grid.macro.record` | `crates/craftkit-plugin-grid/tests/macros.rs` | §1341 |
| **L** | a Python cell is a second language in the formula bar, and its result is stored in the file | `docs/plans/gridcraft.md` | `craftkit-plugin-grid` | `src/python.rs` | `grid.python.eval` | `crates/craftkit-plugin-grid/tests/python.rs` | §1342 |
| **M** | an equation is a floating shape holding a source, and the maths engine lays that source out | `docs/plans/gridcraft.md` | `craftkit-maths` | `src/lib.rs` | `maths.equation.layout` | `crates/craftkit-maths/tests/equation.rs` | §1343 |
| **M** | the battery's commands place the equation the maths engine laid out | `docs/plans/gridcraft.md` | `craftkit-plugin-grid` | `src/equation.rs` | `grid.equation.add` | `crates/craftkit-plugin-grid/tests/equation.rs` | §1343 |
| **N** | a connection is a named link to another file on disk with the value it last read, and a refresh re-reads that file; the tree fetches nothing from the network, and a link whose file is gone is refused by name | `docs/plans/gridcraft.md` | `craftkit-plugin-grid` | `src/query.rs` | `grid.query.refresh` | `crates/craftkit-plugin-grid/tests/query.rs` | §1344 |
| **O** | a shape, a WordArt and a text box are the vector battery's ink | `docs/plans/gridcraft.md` | `craftkit-plugin-vector` | `src/shapes.rs` | `vector.shape.add`, `vector.wordart.add`, `vector.textbox.add` | `crates/craftkit-plugin-vector/tests/shapes.rs` | §1345 |
| **O** | an ink drawing is the image battery's ink | `docs/plans/gridcraft.md` | `craftkit-plugin-image` | `src/ink.rs` | `image.ink.draw` | `crates/craftkit-plugin-image/tests/ink.rs` | §1345 |

The second wave is the whole upstream: every row the survey held back is an arc above, and the rows that belonged to another battery are arcs **in** that battery — the shapes and the ink are not dropped, they are ported there. The survey's "later" verdicts were judgements about **size** and not about **scope**: each of those rows is a new model or a new dependency, and a new model is an arc like any other. The three workbook arcs (G, H, I) share one reader, and that reader is the **craft-libs plan's** arc; the reader is a shared library, the command that hands the file to it is the battery's, and the same answer does not live in two places. The equations arc (M) lands in two crates and holds one section: the maths engine is a new crate, and the battery's commands that use it are the battery's own. The shapes arc (O) lands in two crates and holds one section too, for the same reason.

Each of the second wave's arcs is three edits and no more, like the first wave's: a row in the port manifest, a claim row in the README, and a section in the README, whose sections are **§1337–§1345** in this order. Arc **M** adds a crate, and that is three edits of its own on top of the arc's three: the `Cargo.toml`, the row in the layer table, and the row in `docs/architecture.md`.

## 3. The order, and what each arc ships

| arc | ships | cost |
|---|---|---|
| A | the chart kinds beyond the four the battery takes today | medium — a wider kind set, and every kind needs a picture that is not empty |
| B | distinct count beside the six aggregates the battery knows | small — one more aggregate over the same pivot model |
| C | table styles, banded rows and columns, and the header/first/last column marks | medium — a style value in the document, and a reader for it |
| D | chart type change, source-data re-pick, quick layout, and the series shape fields | medium — the design operations are the upstream's own operations, read one at a time |
| E | a searchable validation list | small — a list query over the rule the battery already holds |
| F | split panes and saved custom views beside the frozen panes it has | medium — the pane arrangement is a second layout model |
| G | the battery's own command that reads a legacy workbook, over the shared reader | medium — the command is small; the reader is the craft-libs plan's arc |
| H | the battery's own command that reads a binary workbook, over the shared reader | medium — the command is small; the reader is the craft-libs plan's arc |
| I | the battery's own command that reads an OpenDocument workbook, over the shared reader | medium — the command is small; the reader is the craft-libs plan's arc |
| J | form controls: buttons, sliders and spin buttons, and pressing them | medium — an interactive object the sheet holds |
| K | recorded macros: recording, listing, running, saving and renaming | large — stored behaviour the document carries |
| L | Python cells: a second language in the formula bar, with the result stored in the file | large — a second language in the battery |
| M | the maths engine and the battery's commands that place an equation | large — a new crate, and its own three edits |
| N | Get & Transform: queries, connections, refresh all and reapply | medium — a link to a file on disk, and the value it last read |
| O | the shapes, the WordArt and the text boxes in the vector battery, and the ink in the image battery | medium — two crates, one section |

The order is A, B, C, D, E, F. Chart kinds come first because every other chart row — the design operations in particular — is meaningless over four kinds, and because the kinds are the visible gap. Pivot summaries and table styles follow: both are values the document already has room for, and neither changes the battery's shape. The chart design operations come after the kinds they operate on. The validation list and the saved views come last because they are the two rows that touch a door's behaviour rather than the document's data.

The second wave's order is G, H, I, J, K, L, M, N, O. The three workbook readers come first because they share one reader and the commands over it are thin; the form controls follow, then the recorded macros and the Python cells, which are the two heaviest models; the maths engine comes with the equations, the queries and the connections follow, and the shapes and the ink come last because they land in the other batteries.

## 4. The tests

- **A** — every kind the battery accepts produces a chart that is not empty, and a kind outside the set is refused.
- **B** — distinct count over a pivot's values is right, and an aggregate the battery does not know is refused.
- **C** — a style set on a table is read back unchanged, banded rows alternate, and a style name that is not known is refused.
- **D** — changing a chart's type keeps its data, re-picking the source range changes it, and a quick layout is one command that can be undone.
- **E** — a search over a validation list finds the entries it should and refuses nothing it should not.
- **F** — a split pane and a saved view round-trip through the document, and a saved view that names no arrangement is refused.
- **G** — a legacy workbook is read through the shared reader, and a file that is not a legacy workbook is refused by name.
- **H** — a binary workbook is read through the shared reader, and a file that is not a binary workbook is refused by name.
- **I** — an OpenDocument workbook is read through the shared reader, and a file that is not one is refused by name.
- **J** — a control is placed, pressed, and the command it carries runs; a control that carries no command is refused.
- **K** — a macro is recorded, listed, run, saved and renamed, and a name that is not known is refused.
- **L** — a Python cell's result is stored in the file and read back unchanged, and a cell whose script fails is refused with the reason.
- **M** — the maths engine lays an equation's source out, the battery places the result, and a source that does not parse is refused.
- **N** — a connection names a file on disk and holds the value it last read; a refresh re-reads that file, and a link whose file is gone is refused by name.
- **O** — a shape, a WordArt, a text box and an ink drawing each land in the battery that owns them, and each is drawn and read back unchanged.

## 5. What it does not do

**Nothing in the upstream's spreadsheet surface is refused.** Every row the survey held back is an arc above, and the rows that belonged to another battery are arcs **in** that battery.

One line to keep in the prose: the maths engine and the three workbook readers are named as **dependencies** of their arcs, and the tree fetches nothing from the network — a dictionary, a corpus or a price is a file on disk or it does not exist.

## 6. The open questions

- The survey read the upstream's command ids but not each command's parameters, so a row marked "partial" may have a second parameter route that the survey did not see.
- The legacy-workbook absence is about what the survey could find, not a proven absence.
- The upstream's own gap list and roadmap were not read, so the survey does not know which gaps the upstream itself treats as open.
- The survey did not reconcile its table line by line with the tree's existing battery survey document.

The second wave leaves the **shape of the new models** open. A form control, a recorded macro, a Python cell and an equation are four new things the document may hold, and the survey says what each of them does but not what each of them is made of — which fields, which values, and where in the document they sit. That is the question the arcs answer as they land, and it is the one thing the survey could not answer.
