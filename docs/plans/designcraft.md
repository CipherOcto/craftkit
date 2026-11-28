# The designcraft plan: what the upstream has and the layout and prose batteries do not

The upstream is a page-layout engine written in Rust, and this plan is about
the parts of it that the `craftkit-plugin-layout` battery and the
`craftkit-plugin-prose` battery do not yet reach. It is thirty-three arcs, and
it reserves README sections **§1285 to §1420** — one section per arc, in the
order the arcs are written here. The first wave is thirteen arcs and takes
§1285 to §1297; the second wave is twenty arcs and takes §1401 to §1420.

## 0. The rule

Every arc here adds a command to the battery that owns the capability —
the layout battery, the prose battery, and the batteries a row moved to — and
every one of them is written the way the tree writes a command: a named rule,
a typed parameter, and a test.

## 1. What the survey found

| capability | upstream path:line | craftkit's state | verdict |
|---|---|---|---|
| Insert pages at a position | `crates/engine/src/cmd/layout/registry.rs:62` | partial — `layout.page.add` exists but only appends | PORT |
| Duplicate a spread | `crates/engine/src/cmd/layout/registry.rs:167` | absent | PORT |
| Page size and page options | `crates/engine/src/cmd/layout/registry.rs:601`, `:634` | partial — width and height only at add time | PORT |
| Layer options (colour, name, state) | `crates/engine/src/cmd/layout/registry.rs:782` | partial — the layer can be renamed, and its colour is set at add time | PORT |
| Duplicate a layer | `crates/engine/src/cmd/layout/registry.rs:773` | absent | PORT |
| The active layer | `crates/engine/src/cmd/layout/registry.rs:905` | partial — the layer's visibility and lock exist, but no active layer | PORT |
| List the guides of a spread | `crates/engine/src/cmd/guides.rs:85` | absent | PORT |
| Move a column guide | `crates/engine/src/cmd/layout/registry.rs:417` | absent | PORT |
| Move an object | `crates/engine/src/cmd/object.rs:57` | absent — the rectangle is set only at add time | PORT |
| Rename an object | `crates/engine/src/cmd/object.rs:671` | absent | PORT |
| Align objects | `crates/engine/src/cmd/object.rs:722` | absent | PORT |
| Distribute objects | `crates/engine/src/cmd/object.rs:731` | absent | PORT |
| Swap fill and stroke | `crates/engine/src/cmd/object.rs:240` | absent | PORT |
| Step and repeat | `crates/engine/src/cmd/object.rs:675` | absent | PORT |
| Alt text on an item | `crates/engine/src/cmd/object.rs:898` | partial — the prose battery has alt text, the layout item does not | PORT |
| Read a story | `crates/engine/src/cmd/text.rs:200` | partial — the story node evaluates to its text and length, but there is no command | PORT |
| Fill with placeholder text | `crates/engine/src/cmd/text.rs:214` | absent | PORT |
| Row height and column width | `crates/engine/src/cmd/table/registry.rs:317`, `:326` | absent | PORT |
| Distribute rows and columns evenly | `crates/engine/src/cmd/table/registry.rs:344`, `:353` | absent | PORT |

Nineteen rows, and every one of them is a gap: **absent** or **partial**, and
never a reader that is already there with no setter. Two of them are partial
in the same way — a fact that can be written once, at creation, and never
afterwards. A page's size and a layer's colour are both set at add time and
cannot be changed; the tree's rule that a fact of a thing is a fact of that
thing and may be changed is not yet honoured for either.

**One row is close to parity, and it is named here so the arc does not
mistake it.** The survey called the guide move absent, but the layout battery
already carries `layout.guide.add`, `layout.guide.create` and
`layout.guide.move`. What the upstream row adds is the **guide as a guide on
a page** — a column guide moved on a named page — and the survey also found
no way to **read** the guides of a spread at all. The guide arc therefore
carries a listing and a page-scoped move, and it does not rebuild a command
that already exists.

**The survey's own five sentences** are condensed here. The upstream has a
complete page-layout arrangement grammar — parent spreads with per-page
overrides, alternate and liquid layouts, flex layout, section numbering and a
shuffled spread — and the layout battery has only the flat parts of it: a
spread, pages, items, layers and guides. The upstream's transform and effect
vocabulary is far wider than the item facts: move, resize about an anchor,
shear, flip, align, distribute, inner shadow, glow, bevel, satin, feathering,
text wrap, fitting, gridify, and step and repeat. The upstream's editorial
apparatus is deeper than the prose battery's: annotation notes that can be
split, an index generator, cross-reference formats, a table of contents with
page numbers, captions with a numbering policy, and a data merge over a table
of records. The upstream's colour, typography and font machinery has no
counterpart in any of the batteries. And the upstream's interactive and
structural layer is either absent from craftkit or deliberately out of scope.

**The later work the survey named** — the parent spread as an entity of its
own, section and chapter numbering, alternate and liquid layouts, a shuffled
spread, a shear, a flip, clearing transformations, inner shadow, glow, bevel,
satin, feathering, text wrap, fitting content to a frame, gridifying a frame,
threading text frames, kerning, OpenType features, vertical writing, story
direction, cell and table styles, colour swatches, composite fonts, annotation
notes, conditional visibility, form fields, XML tagging, preflight as a rule
set, multi-state objects, flex layout, anchored objects, a primary text frame,
converting text to a table, and a book — is the **second wave** of arcs, and
every one of those rows is an arc in §2.

**The out-of-scope rows** are the upstream's file formats, and the upstream's
page transitions, which are a rendering surface. **Selection, undo, redo, copy
and paste** are rows at parity, because the core owns them. Every other row
the survey set aside — export, ink separations, colour conversion, paths,
caret-level typing and the rest — is an arc of the second wave, and each one
lands in the battery that owns the capability. The refusals and the parity
rows are named with their reason in §5.

## 2. The arcs

An arc here is three edits and no more: a row in the port manifest
(`xtask/src/ports.rs`), a row in the README's claim table, and a section of
the README. The port manifest's rows take a document in this tree as their
`from`, so these arcs take **this plan** as their `from`: the plan is the
document the arcs come from, not the survey it follows.

| arc | rule | from | to | module | command | test | section |
|---|---|---|---|---|---|---|---|
| **A** | a spread is duplicated into a new spread whose pages and items are copies, not the same nodes | `docs/plans/designcraft.md` | `craftkit-plugin-layout` | `src/pages.rs` | `layout.spread.duplicate` | `crates/craftkit-plugin-layout/tests/spreadduplicate.rs` | §1285 |
| **B** | a page's own facts — the index it sits at in its spread and its width and height — may be changed after the fact, and an index outside the spread is refused | `docs/plans/designcraft.md` | `craftkit-plugin-layout` | `src/pages.rs` | `layout.page.insert`, `layout.page.set` | `crates/craftkit-plugin-layout/tests/pagefacts.rs` | §1286 |
| **C** | a layer's own facts — its name, its colour, its copies and which one is active — are facts of the layer and may be changed | `docs/plans/designcraft.md` | `craftkit-plugin-layout` | `src/layers.rs` | `layout.layer.set`, `layout.layer.duplicate`, `layout.layer.active` | `crates/craftkit-plugin-layout/tests/layerfacts.rs` | §1287 |
| **D** | the guides of a spread are readable, and a column guide is a guide on a page, so moving it is a guide move with a page | `docs/plans/designcraft.md` | `craftkit-plugin-layout` | `src/guides.rs` | `layout.guide.list`, `layout.page.guide.move` | `crates/craftkit-plugin-layout/tests/guidelist.rs` | §1288 |
| **E** | an item's rectangle holds its position and its name is its own fact, and both may be changed after the item exists | `docs/plans/designcraft.md` | `craftkit-plugin-layout` | `src/items.rs` | `layout.item.move`, `layout.item.rename` | `crates/craftkit-plugin-layout/tests/itemfacts.rs` | §1289 |
| **F** | aligning puts the items on one edge or one centre line of their common box, and distributing spaces them evenly between the first and the last | `docs/plans/designcraft.md` | `craftkit-plugin-layout` | `src/order.rs` | `layout.item.align`, `layout.item.distribute` | `crates/craftkit-plugin-layout/tests/itemplacement.rs` | §1290 |
| **G** | swapping exchanges the two paint facts of the item | `docs/plans/designcraft.md` | `craftkit-plugin-layout` | `src/paint.rs` | `layout.item.paint.swap` | `crates/craftkit-plugin-layout/tests/paintswap.rs` | §1291 |
| **H** | stepping and repeating writes copies of the item along a direction and a count | `docs/plans/designcraft.md` | `craftkit-plugin-layout` | `src/items.rs` | `layout.item.repeat` | `crates/craftkit-plugin-layout/tests/itemrepeat.rs` | §1292 |
| **I** | an item may say what it is, and the saying is a fact of the item | `docs/plans/designcraft.md` | `craftkit-plugin-layout` | `src/items.rs` | `layout.item.alt` | `crates/craftkit-plugin-layout/tests/itemalt.rs` | §1293 |
| **J** | a story's text and length are derived, and they are read rather than stored twice | `docs/plans/designcraft.md` | `craftkit-plugin-layout` | `src/items.rs` | `layout.story.get` | `crates/craftkit-plugin-layout/tests/storyget.rs` | §1294 |
| **K** | a table's own measurements — a row's height, a column's width, and the even division of the space — are facts of the table | `docs/plans/designcraft.md` | `craftkit-plugin-prose` | `src/tables.rs` | `prose.table.row.height`, `prose.table.column.width`, `prose.table.distribute` | `crates/craftkit-plugin-prose/tests/tablemeasure.rs` | §1295 |
| **L** | placeholder text fills a frame so its length can be seen | `docs/plans/designcraft.md` | `craftkit-plugin-prose` | `src/paragraph.rs` | `prose.block.placeholder.fill` | `crates/craftkit-plugin-prose/tests/placeholder.rs` | §1296 |
| **M** | the rulers, the zero point and the preferences are facts of the document, and the front end reads them | `docs/plans/designcraft.md` | `craftkit-plugin-layout` | `src/view.rs` | `layout.ruler.set`, `layout.view.zero.set`, `layout.preference.set` | `crates/craftkit-plugin-layout/tests/view.rs` | §1297 |

**The nineteen rows are grouped into twelve rules, and the grouping is by
subject, not by command.** The spread's own facts are the duplication of a
spread, so **Arc A** is that one row. The page's own facts are the position
of a page in its spread and the size of a page, so **Arc B** carries the
insert row and the size row. The layer's own facts are its name and colour,
its copies, and which layer is active, so **Arc C** carries the layer-options
row, the layer-duplication row and the active-layer row. The guides are one
group, so **Arc D** carries the listing row and the column-guide-move row.
The item's own facts are its rectangle and its name, so **Arc E** carries the
move row and the rename row. The item's placement among other items is its
own rule, so **Arc F** carries the align row and the distribute row. The
item's paint is a rule of its own, so **Arc G** carries the swap row. The
item's repetition is a rule of its own, so **Arc H** carries the step-and-
repeat row. The item's alt text is a rule of its own, so **Arc I** carries
the alt-text row. The story's reading is a rule of its own, so **Arc J**
carries the story row. The table's own measurements are one rule, so **Arc
K** carries the row-height and column-width row and the even-distribution row.
And placeholder text is a rule of its own, so **Arc L** carries the
placeholder row.

**The sibling prose plan already carries four of the prose gaps, and this
plan does not repeat them.** The prose battery's plan at
`docs/plans/wordcraft.md` already plans the **character facts of a run**
(`prose.run.format`, extended), the **link on a run** (`prose.block.link.add`),
the **tab stops of a paragraph** (`prose.block.stops.set`) and the **sort of a
table** (`prose.table.sort`). None of those four is an arc here, and none of
the thirteen arcs above touches the same ground: this plan's prose work is the
table's measurements and the placeholder text, which that plan does not have.

**Eleven arcs land in the layout battery and two in the prose battery.** An arc
that lands in two crates would need two port rows, and none here does.

**The second wave: the whole upstream.**

| arc | rule | from | to | module | command | test | section |
|---|---|---|---|---|---|---|---|
| **N1** | a spread may have a parent spread, and a page of the parent may override the parent's own facts | `docs/plans/designcraft.md` | `craftkit-plugin-layout` | `src/parents.rs` | `layout.parent.set`, `layout.parent.override.set` | `crates/craftkit-plugin-layout/tests/parentoverrides.rs` | §1401 |
| **N2** | a section is a range of pages that carries a number of its own | `docs/plans/designcraft.md` | `craftkit-plugin-layout` | `src/sections.rs` | `layout.section.add`, `layout.section.number.set` | `crates/craftkit-plugin-layout/tests/sections.rs` | §1402 |
| **N3** | a layout may change with the page it is on, and the page says which layout it takes | `docs/plans/designcraft.md` | `craftkit-plugin-layout` | `src/layouts.rs` | `layout.alternate.set`, `layout.liquid.set` | `crates/craftkit-plugin-layout/tests/alternate.rs` | §1403 |
| **N4** | the pages of a spread may be arranged by a rule that holds over whole spreads | `docs/plans/designcraft.md` | `craftkit-plugin-layout` | `src/shuffle.rs` | `layout.spread.shuffle.set` | `crates/craftkit-plugin-layout/tests/shufflespread.rs` | §1404 |
| **N5** | an item's shear, its flip and its stack of transformations are facts of the item, and the stack may be cleared | `docs/plans/designcraft.md` | `craftkit-plugin-layout` | `src/transform.rs` | `layout.item.shear.set`, `layout.item.flip.set`, `layout.item.transform.clear` | `crates/craftkit-plugin-layout/tests/transforms.rs` | §1405 |
| **N6** | the effects on an item — inner shadow, glow, bevel, satin and feathering — are facts of the item | `docs/plans/designcraft.md` | `craftkit-plugin-layout` | `src/effects.rs` | `layout.item.inner.shadow.set`, `layout.item.glow.set`, `layout.item.bevel.set`, `layout.item.satin.set`, `layout.item.feather.set` | `crates/craftkit-plugin-layout/tests/effects.rs` | §1406 |
| **N7** | the text in a frame may be wrapped in it, fitted to it, laid on a grid, and threaded from one frame to the next | `docs/plans/designcraft.md` | `craftkit-plugin-layout` | `src/frametext.rs` | `layout.frame.wrap.set`, `layout.frame.fit.set`, `layout.frame.grid.set`, `layout.frame.thread.set` | `crates/craftkit-plugin-layout/tests/frametext.rs` | §1407 |
| **N8** | the typography of a run — its kerning, its font features, its vertical writing and its story direction — is a fact of the run | `docs/plans/designcraft.md` | `craftkit-plugin-layout` | `src/typography.rs` | `layout.text.kerning.set`, `layout.text.feature.set`, `layout.text.vertical.set`, `layout.text.direction.set` | `crates/craftkit-plugin-layout/tests/typography.rs` | §1408 |
| **N9** | a cell's style and a table's style are named sets of facts that the table refers to | `docs/plans/designcraft.md` | `craftkit-plugin-prose` | `src/styles.rs` | `prose.table.style.set`, `prose.table.cell.style.set` | `crates/craftkit-plugin-prose/tests/tablestyles.rs` | §1409 |
| **N10** | a colour swatch and a composite font are resources that the document refers to | `docs/plans/designcraft.md` | `craftkit-plugin-layout` | `src/swatches.rs` | `layout.swatch.add`, `layout.font.composite.add` | `crates/craftkit-plugin-layout/tests/swatches.rs` | §1410 |
| **N11** | an annotation note is a note on the document, and a note may be split into parts | `docs/plans/designcraft.md` | `craftkit-plugin-prose` | `src/notes.rs` | `prose.note.add`, `prose.note.split` | `crates/craftkit-plugin-prose/tests/annotationnotes.rs` | §1411 |
| **N12** | visibility, form fields, tagging, preflight rules and object states are rules the document holds and that are evaluated when they are read | `docs/plans/designcraft.md` | `craftkit-plugin-layout` | `src/rules.rs` | `layout.rule.visibility.add`, `layout.form.field.add`, `layout.tag.apply`, `layout.preflight.rule.add`, `layout.item.state.set` | `crates/craftkit-plugin-layout/tests/ruleset.rs` | §1412 |
| **N13** | a frame may lay its children out by flex, an item may be anchored to a frame, and one text frame is the primary one | `docs/plans/designcraft.md` | `craftkit-plugin-layout` | `src/flex.rs` | `layout.frame.flex.set`, `layout.item.anchor.set`, `layout.frame.primary.set` | `crates/craftkit-plugin-layout/tests/flexanchor.rs` | §1413 |
| **N14** | text may be converted into a table, and a document may be assembled into a book | `docs/plans/designcraft.md` | `craftkit-plugin-layout` | `src/convert.rs` | `layout.text.to.table`, `layout.book.build` | `crates/craftkit-plugin-layout/tests/texttotable.rs` | §1414 |
| **N15** | a document may be exported to another format | `docs/plans/designcraft.md` | `craftkit-plugin-pdf` | `src/convert.rs` | `pdf.document.export` | `crates/craftkit-plugin-pdf/tests/documentexport.rs` | §1415 |
| **N16** | ink may be separated into its separations | `docs/plans/designcraft.md` | `craftkit-plugin-image` | `src/separations.rs` | `image.ink.separate` | `crates/craftkit-plugin-image/tests/inkseparations.rs` | §1416 |
| **N17** | the colour of a path may be converted between colour spaces | `docs/plans/designcraft.md` | `craftkit-plugin-vector` | `src/colour.rs` | `vector.colour.convert` | `crates/craftkit-plugin-vector/tests/colourconvert.rs` | §1417 |
| **N18** | a path is a shape of its own, and the facts of a path are facts of the path | `docs/plans/designcraft.md` | `craftkit-plugin-vector` | `src/pathcreate.rs` | `vector.path.create`, `vector.path.set` | `crates/craftkit-plugin-vector/tests/paths.rs` | §1418 |
| **N19** | text may be typed at the caret | `docs/plans/designcraft.md` | `craftkit-plugin-text` | `src/lib.rs` | `text.caret.type` | `crates/craftkit-plugin-text/tests/carettype.rs` | §1419 |
| **N20** | a font the document refers to is a resource of the document, and a run's font is a fact of the run | `docs/plans/designcraft.md` | `craftkit-plugin-prose` | `src/characters.rs` | `prose.font.add`, `prose.run.font.set` | `crates/craftkit-plugin-prose/tests/fonts.rs` | §1420 |

**The second wave is the whole upstream.** The first wave took the gaps the
survey could point at; this wave takes every capability the upstream has, so
that nothing in it is left behind. **The survey's "later" verdicts were
judgements about size and not about scope** — it said a thing was later
because the model it wanted was a large one, and the owner has now decided
that the large model is in scope.

**A row that belongs to another battery is an arc in that battery.** The paths
are the vector battery's, the ink is the image battery's, the caret typing is
the text battery's, and the export is the PDF battery's — because the PDF
battery writes files, and the reader and the writer for the format are the
craft-libs plan's arcs. **None of them is dropped.** Twelve of the twenty
arcs land in the layout battery, three in the prose battery, two in the
vector battery, and one each in the PDF, image and text batteries. As in the
first wave, no arc lands in two crates, so no arc needs two port rows.

## 3. The order, and what each arc ships

| arc | ships | cost |
|---|---|---|
| A | the duplicated spread | medium — a spread is copied, and every page and item on it is copied rather than shared |
| B | the page's position and size | medium — an index into a spread, and a size that is checked against zero |
| C | the layer's name, colour, copies and the active one | medium — four facts, and the active layer is a single slot on the session |
| D | the guides of a spread, read and moved on a page | small — the guides are already facts of a page; the listing and the page scope are the new part |
| E | the item's rectangle and name | small — both are already facts of an item |
| F | aligning and distributing | medium — a common box over a set of items, and an even division of the space between the first and the last |
| G | the swapped fill and stroke | small — two facts of one item, exchanged |
| H | the repeated item | medium — copies are written along a direction and a count, and each copy is a node of its own |
| I | the alt text on an item | small — one string on an item, and the prose battery's alt-text model is the shape to copy |
| J | the read story | small — the text and the length are derived already, and the command only reads them |
| K | the table's measurements | medium — a height, a width, and an even division that is stated in one number |
| L | the placeholder text | medium — the fill is chosen to a length, and the length has to be the one the frame asks for |
| M | the rulers, the zero point and the preferences | small — three fields of the document that the front end reads and applies |
| N1 | the parent spread and its per-page overrides | medium — a second level of spread, and an override that is resolved per page |
| N2 | the sections and their numbers | medium — a range of pages is a new thing to hold, and a number of its own is a fact of it |
| N3 | the alternate and liquid layouts | medium — a layout that depends on the page it is on is a derived fact |
| N4 | the shuffled spread | medium — a rule over whole spreads, and the pages it moves |
| N5 | the shear, the flip and the cleared transformations | medium — a transform stack on an item, and a way to empty it |
| N6 | the inner shadow, the glow, the bevel, the satin and the feathering | medium — five per-item effects, each its own facts |
| N7 | the wrap, the fit, the grid and the threading of text frames | large — a layout engine over a frame, and text that moves between frames |
| N8 | the kerning, the font features, the vertical writing and the story direction | medium — four facts of a run, and the machinery under them |
| N9 | the cell and table styles | medium — a named set of facts, and the table's reference to it |
| N10 | the colour swatches and the composite fonts | medium — two kinds of resource, and the document's table of them |
| N11 | the annotation notes and the split note | medium — a document-level note, and a way to divide it |
| N12 | the visibility rules, the form fields, the tagging, the preflight rules and the object states | large — five kinds of rule held in the document and evaluated when they are read |
| N13 | the flex layout, the anchored objects and the primary text frame | large — a layout engine over a frame, an anchoring relation, and one frame that is special |
| N14 | the text turned into a table, and the book | medium — both rewrite the document's structure rather than a fact of one thing |
| N15 | the export of a document to another format | medium — the format's reader and writer, which the craft-libs plan carries |
| N16 | the ink separations | medium — a picture per separation, and the rules that divide the ink |
| N17 | the colour conversion | small — a conversion between colour spaces, and the space it converts to |
| N18 | the paths | medium — a path's own facts, and the geometry that holds them |
| N19 | the caret-level typing | medium — the text of a frame written one caret position at a time |
| N20 | the font resources and a run's font | medium — a resource the document holds, and the run that refers to it |

The order is the order of the surface, from the outside in: the spread, the
page, the layer, the guides, then the item — its own facts, its placement,
its paint, its repetition, its alt text — and then the two things that are
read rather than written, the story and the table's measurements, with the
placeholder text last because it depends on a frame and a length.

**Arc M comes at the very end of the first wave**, because it is about how the document is
*looked at* and not about what it holds, and because the owner asked for it
by name: the saved view and preference state belongs in the document, and the
front end reads it and applies it.

**Nothing ships before Arc A**, and every arc from A on is a visible command
that stands on its own. There is no invisible preparation in this plan, so
the plan is one batch and the arcs are ordered for review rather than for
dependency.

## 4. The tests

The house rule: **the crate's own rule has a test.** A rule with no test is
a habit. Each arc's test file is written in the same arc that adds the port
row.

| arc | the test that must exist |
|---|---|
| A | the duplicated spread holds copies: changing a page in the copy leaves the original alone; a spread that is not in the document is refused |
| B | a page inserted at an index sits at that index and the pages after it move; an index outside the spread is refused, not clamped; a width or a height of zero or less is refused; a size set on a page is the size the page reports |
| C | the layer's name and colour are read back as they were set; a duplicated layer holds copies of the items that were on it; setting a second active layer makes it the active one and the other is not; a layer that is not in the document is refused |
| D | the guides of a spread are listed as the spread holds them; a column guide moved on a page is at the position it was moved to; a move on a page that has no such guide is refused |
| E | the item's rectangle is the rectangle that was written; the name is the name that was set; a move to a position off the page is refused rather than clamped |
| F | the aligned items share one edge or one centre line of their common box; the distributed items are evenly spaced between the first and the last; a set of one item is refused rather than padded |
| G | the fill and the stroke are exchanged, and an item with no stroke is refused rather than given one |
| H | the copies are written along the direction and to the count that were asked for; a count of zero or less is refused; each copy is a node of its own and editing one leaves the others alone |
| I | the alt text is read back as it was set; an item with no alt text reports none, not an empty string |
| J | the story's text and length are the ones the node derives, and the same story read twice gives the same answer |
| K | a row's height and a column's width are read back as they were set; the distribution says the even number it divided to, and the rows or columns are that even; a height or a width of zero or less is refused |
| L | the frame is filled with text of the length that was asked for; a length of zero or less is refused; the fill does not change the frame's own facts |
| M | the rulers, the zero point and the preference are read back as they were set; a preference the tree does not know is refused by name; the front end reads them and does not write them |
| N1 | a page of a parent spread reads the parent's facts until it overrides them, and then reads its own; an override on a page that is not in the parent is refused |
| N2 | a section holds the range of pages it was given, and the number it was given; a range that runs backwards is refused, not reordered |
| N3 | the layout a page takes is the one that page says, and a page that says nothing takes the spread's; a layout the tree does not know is refused by name |
| N4 | the pages of a shuffled spread are the pages the rule put in that order; a rule that names a page that is not in the spread is refused |
| N5 | the shear and the flip are read back as they were set, and clearing the transformations leaves the item with none rather than with none written |
| N6 | each of the five effects is read back as it was set; an item with no effect of that kind reports none, not an empty one |
| N7 | the text in a wrapped frame is wrapped at the frame's width; the fitted text fits; the gridded frame puts the text on the grid; threaded text continues in the next frame; a frame with no text to thread is refused |
| N8 | the kerning, the features, the vertical writing and the direction are read back as they were set; a feature the font does not have is refused by name |
| N9 | a cell's style is the style it was given, and the style holds the facts it was declared with; a style the table does not have is refused by name |
| N10 | a swatch and a composite font are read back as they were added, and the document refers to them by name; a second resource of the same name is refused |
| N11 | a note is read back as it was written, and a split note gives the parts it was split into; splitting a note that has one part is refused |
| N12 | a visibility rule that does not hold hides the item, and one that holds shows it; a form field is read back as it was added; a tag is applied to the item it names; a preflight rule reports the rule it breaks; a multi-state item reports the state it is in; a rule that names something that is not in the document is refused |
| N13 | the children of a flex frame are placed as the flex rule says; an anchored item stays with the frame it is anchored to; the primary text frame is the one that was made primary, and only one is |
| N14 | the converted table holds the text that was converted; the book holds the documents it was built from, in the order they were given; a text that cannot be converted is refused |
| N15 | the exported document is the document that was exported, and the format it was exported to is the format that was asked for; a format the tree cannot write is refused by name |
| N16 | the separations are the separations the ink divides into, and each is a picture of its own; ink with one colour does not divide into more than one separation |
| N17 | the converted colour is the colour of the source under the target space, and a space the tree does not know is refused by name |
| N18 | a path's points are the points it was given, and its facts are read back as they were set; a path with fewer than two points is refused |
| N19 | the typed text is the text that was typed, and the caret is where it was typed at; typing at a caret that is not in the document is refused |
| N20 | the font the document holds is the font it was added as, and a run's font is the font it was given; a run that refers to a font the document does not hold is refused by name |

**The refusals** are the tests that matter. An index outside a spread, a
size of zero, a page that has no such guide, a set of one item to align, a
count of zero to repeat, and a length of zero to fill are the places where the
house rule — *refuse, do not clamp* — and a comfortable port would disagree.

## 5. What it does not do

**Refused.** These are the only things this plan does not carry, and no arc
touches them.

- **The upstream's own file formats** are refused because craftkit has its own
  document format per battery, and a reader for another tool's private
  container is a library the craft-libs plan names — where it is a design
  container, `docs/plans/craft-libs.md` holds it, and it is not duplicated
  here. Exporting a document is a different matter, and **Arc N15** carries
  it.
- **The page transitions** are refused because a transition is a rendering
  surface, and the tree's front ends do not carry one.

**Parity.** One row is at parity, and it is named so that no arc mistakes it
for a gap.

- **Selection, undo, redo, copy and paste** are at parity, because the core
  owns them.

Everything else that the survey set aside is now an arc above, and the
second wave's table in §2 is the whole of it.

## 6. The open questions

The survey wrote down what it could not read or decide, and those are the
questions this plan does not answer.

**The second wave leaves one thing open, and it is the shape of the new
models.** The arcs in §2 say what each command does, but they do not say what
a parent spread, a section, a transform stack, a frame layout, a named style,
a resource, a rule set or a path is in the tree's model — what those things
are made of, and where they live. Several of them are new kinds of thing in
the layout battery's document, and the shape they take decides what the
commands above can be. That is the question the first arc of the second wave
has to answer.

- **A capability marked absent may be there as data rather than as a
  command.** The survey was confined to the upstream's command directory, so
  a capability it called absent may exist there in some other form. What
  would settle it is a reading of the upstream's data layer against the same
  contract.
- **The rule the survey states is its reading of the interface.** For most
  rows it read the declared interface rather than the handler bodies, so the
  rule in each row may not be the rule the upstream actually enforces.
  What would settle it is a reading of the handler bodies.
- **The upstream's rows carry menu, shortcut, journal and undoable flags,
  and this tree's table does not.** The survey assumed those are front-end
  concerns, but a shortcut layer is a real user-visible capability it did not
  map. What would settle it is a decision about whether the tree grows such a
  layer at all.
- **Whether the prose battery or the grid battery is the right home for the
  upstream's page tables is unsettled**, because the upstream's cells hold
  text while the prose battery's cells hold values. What would settle it is a
  reading of the grid battery's cell model against the prose battery's.
- **The survey did not audit the art, image, audio and shape batteries**, so
  "absent" here means absent from the batteries it was asked about.
- **What a preference is, is not settled.** The owner has brought the view
  and preference state into scope, and the survey's row names the upstream's
  preference surface without reading it. What would settle it is a reading of
  the upstream's preference list, and the answer decides whether Arc M is
  three fields or a table of named preferences.
