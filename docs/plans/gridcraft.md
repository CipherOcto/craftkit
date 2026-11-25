# The gridcraft plan: what the upstream has and the grid battery does not

This plan is a reading of the upstream spreadsheet surface against craftkit's grid battery, and a set of arcs for the gap between them. It is a plan for future work: nothing described here exists in this tree today.

## 0. The rule

Every capability the upstream has that craftkit's grid battery does not is either a command the battery gains, or a model the battery does not take on.

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

The LATER rows are each a new model or a new dependency — legacy workbooks and the other workbook formats, form controls, recorded macros, Python cells, equations, external queries — and none of them is a command the battery could simply gain.

The SKIP row is out of scope altogether: shapes, WordArt, text boxes and ink drawing belong to the shape and vector batteries.

## 2. The arcs

| arc | rule | from | to | module | command | test | section |
|---|---|---|---|---|---|---|---|
| **A** | a chart kind is a value the battery names, not a drawing the battery draws | `docs/plans/gridcraft.md` | `craftkit-plugin-grid` | `src/charts.rs` | `grid.chart.set` | `crates/craftkit-plugin-grid/tests/charts.rs` | §1233 |
| **B** | a summary is a named aggregate over a pivot's values | `docs/plans/gridcraft.md` | `craftkit-plugin-grid` | `src/pivot.rs` | `grid.pivot.set` | `crates/craftkit-plugin-grid/tests/pivot.rs` | §1234 |
| **C** | a table's look is a style the document carries | `docs/plans/gridcraft.md` | `craftkit-plugin-grid` | `src/table.rs` | `grid.table.style` | `crates/craftkit-plugin-grid/tests/table.rs` | §1235 |
| **D** | changing a chart's design is a command on the chart | `docs/plans/gridcraft.md` | `craftkit-plugin-grid` | `src/chart.rs` | `grid.chart.design` | `crates/craftkit-plugin-grid/tests/charts.rs` | §1236 |
| **E** | a validation list is searched before it is picked | `docs/plans/gridcraft.md` | `craftkit-plugin-grid` | `src/validation.rs` | `grid.validation.list` | `crates/craftkit-plugin-grid/tests/validation.rs` | §1237 |
| **F** | a view is a saved arrangement of the sheet | `docs/plans/gridcraft.md` | `craftkit-plugin-grid` | `src/views.rs` | `grid.view.split` | `crates/craftkit-plugin-grid/tests/views.rs` | §1238 |

Each arc is three edits and no more: a row in the port manifest (`xtask/src/ports.rs`), a claim row in the README, and a section in the README, whose sections are **§1233–§1238** in this order. The test file named in a row is written in the same arc that adds the row, and the rows name the files the arc will create.

## 3. The order, and what each arc ships

| arc | ships | cost |
|---|---|---|
| A | the chart kinds beyond the four the battery takes today | medium — a wider kind set, and every kind needs a picture that is not empty |
| B | distinct count beside the six aggregates the battery knows | small — one more aggregate over the same pivot model |
| C | table styles, banded rows and columns, and the header/first/last column marks | medium — a style value in the document, and a reader for it |
| D | chart type change, source-data re-pick, quick layout, and the series shape fields | medium — the design operations are the upstream's own operations, read one at a time |
| E | a searchable validation list | small — a list query over the rule the battery already holds |
| F | split panes and saved custom views beside the frozen panes it has | medium — the pane arrangement is a second layout model |

The order is A, B, C, D, E, F. Chart kinds come first because every other chart row — the design operations in particular — is meaningless over four kinds, and because the kinds are the visible gap. Pivot summaries and table styles follow: both are values the document already has room for, and neither changes the battery's shape. The chart design operations come after the kinds they operate on. The validation list and the saved views come last because they are the two rows that touch a door's behaviour rather than the document's data.

## 4. The tests

- **A** — every kind the battery accepts produces a chart that is not empty, and a kind outside the set is refused.
- **B** — distinct count over a pivot's values is right, and an aggregate the battery does not know is refused.
- **C** — a style set on a table is read back unchanged, banded rows alternate, and a style name that is not known is refused.
- **D** — changing a chart's type keeps its data, re-picking the source range changes it, and a quick layout is one command that can be undone.
- **E** — a search over a validation list finds the entries it should and refuses nothing it should not.
- **F** — a split pane and a saved view round-trip through the document, and a saved view that names no arrangement is refused.

## 5. What it does not do

- **Legacy 97-2003 `.xls` workbooks** — the compound-file container is in craftkit but the BIFF reader is not, and a reader is a new dependency rather than a command.
- **XLSB worksheet import** — a second workbook format, and the same kind of new dependency.
- **ODS worksheet import** — a third workbook format, same reason.
- **Form controls** — buttons, sliders and spin buttons are interactive objects, which is a new model rather than a missing command.
- **Recorded macros** — recording, listing, running, saving and renaming a script is a new model of stored behaviour.
- **Python cells** — a second language in the formula bar, whose results are stored in the file.
- **Equations** — a floating shape holding a source and a maths engine to lay it out, and craftkit has no maths crate to reach for.
- **Get & Transform** — queries, connections, refresh and reapply are about the outside world, and craftkit's batteries hold a document, not a connection.

Out of scope entirely: shapes, WordArt, text boxes and ink drawing, which belong to the shape and vector batteries.

## 6. The open questions

- The survey read the upstream's command ids but not each command's parameters, so a row marked "partial" may have a second parameter route that the survey did not see.
- The legacy-workbook absence is about what the survey could find, not a proven absence.
- The upstream's own gap list and roadmap were not read, so the survey does not know which gaps the upstream itself treats as open.
- The survey did not reconcile its table line by line with the tree's existing battery survey document.
