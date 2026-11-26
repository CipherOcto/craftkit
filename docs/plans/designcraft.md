# The designcraft plan: what the upstream has and the layout and prose batteries do not

The upstream is a page-layout engine written in Rust, and this plan is about
the parts of it that the `craftkit-plugin-layout` battery and the
`craftkit-plugin-prose` battery do not yet reach. It is thirteen arcs, and it
reserves README sections **§1283 to §1295** — one section per arc, in the
order the arcs are written here.

## 0. The rule

Every arc here adds a command to one of the two batteries, and every one of
them is written the way the tree writes a command: a named rule, a typed
parameter, and a test.

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
converting text to a table, and a book — is named with its reason in §5.

**The out-of-scope rows** are the upstream's file formats, export and print
surfaces; selection, undo, redo, copy and paste, which the core owns; the
upstream's scripting, library, font and preference surfaces; ink separations;
colour conversion, which the vector battery owns; paths, which the vector
battery owns; caret-level typing, which the text battery owns; and the
upstream's page transitions, which are a rendering surface. They are named
with their reason in §5.

## 2. The arcs

An arc here is three edits and no more: a row in the port manifest
(`xtask/src/ports.rs`), a row in the README's claim table, and a section of
the README. The port manifest's rows take a document in this tree as their
`from`, so these arcs take **this plan** as their `from`: the plan is the
document the arcs come from, not the survey it follows.

| arc | rule | from | to | module | command | test | section |
|---|---|---|---|---|---|---|---|
| **A** | a spread is duplicated into a new spread whose pages and items are copies, not the same nodes | `docs/plans/designcraft.md` | `craftkit-plugin-layout` | `src/pages.rs` | `layout.spread.duplicate` | `crates/craftkit-plugin-layout/tests/spreadduplicate.rs` | §1283 |
| **B** | a page's own facts — the index it sits at in its spread and its width and height — may be changed after the fact, and an index outside the spread is refused | `docs/plans/designcraft.md` | `craftkit-plugin-layout` | `src/pages.rs` | `layout.page.insert`, `layout.page.set` | `crates/craftkit-plugin-layout/tests/pagefacts.rs` | §1284 |
| **C** | a layer's own facts — its name, its colour, its copies and which one is active — are facts of the layer and may be changed | `docs/plans/designcraft.md` | `craftkit-plugin-layout` | `src/layers.rs` | `layout.layer.set`, `layout.layer.duplicate`, `layout.layer.active` | `crates/craftkit-plugin-layout/tests/layerfacts.rs` | §1285 |
| **D** | the guides of a spread are readable, and a column guide is a guide on a page, so moving it is a guide move with a page | `docs/plans/designcraft.md` | `craftkit-plugin-layout` | `src/guides.rs` | `layout.guide.list`, `layout.page.guide.move` | `crates/craftkit-plugin-layout/tests/guidelist.rs` | §1286 |
| **E** | an item's rectangle holds its position and its name is its own fact, and both may be changed after the item exists | `docs/plans/designcraft.md` | `craftkit-plugin-layout` | `src/items.rs` | `layout.item.move`, `layout.item.rename` | `crates/craftkit-plugin-layout/tests/itemfacts.rs` | §1287 |
| **F** | aligning puts the items on one edge or one centre line of their common box, and distributing spaces them evenly between the first and the last | `docs/plans/designcraft.md` | `craftkit-plugin-layout` | `src/order.rs` | `layout.item.align`, `layout.item.distribute` | `crates/craftkit-plugin-layout/tests/itemplacement.rs` | §1288 |
| **G** | swapping exchanges the two paint facts of the item | `docs/plans/designcraft.md` | `craftkit-plugin-layout` | `src/paint.rs` | `layout.item.paint.swap` | `crates/craftkit-plugin-layout/tests/paintswap.rs` | §1289 |
| **H** | stepping and repeating writes copies of the item along a direction and a count | `docs/plans/designcraft.md` | `craftkit-plugin-layout` | `src/items.rs` | `layout.item.repeat` | `crates/craftkit-plugin-layout/tests/itemrepeat.rs` | §1290 |
| **I** | an item may say what it is, and the saying is a fact of the item | `docs/plans/designcraft.md` | `craftkit-plugin-layout` | `src/items.rs` | `layout.item.alt` | `crates/craftkit-plugin-layout/tests/itemalt.rs` | §1291 |
| **J** | a story's text and length are derived, and they are read rather than stored twice | `docs/plans/designcraft.md` | `craftkit-plugin-layout` | `src/items.rs` | `layout.story.get` | `crates/craftkit-plugin-layout/tests/storyget.rs` | §1292 |
| **K** | a table's own measurements — a row's height, a column's width, and the even division of the space — are facts of the table | `docs/plans/designcraft.md` | `craftkit-plugin-prose` | `src/tables.rs` | `prose.table.row.height`, `prose.table.column.width`, `prose.table.distribute` | `crates/craftkit-plugin-prose/tests/tablemeasure.rs` | §1293 |
| **L** | placeholder text fills a frame so its length can be seen | `docs/plans/designcraft.md` | `craftkit-plugin-prose` | `src/paragraph.rs` | `prose.block.placeholder.fill` | `crates/craftkit-plugin-prose/tests/placeholder.rs` | §1294 |
| **M** | the rulers, the zero point and the preferences are facts of the document, and the front end reads them | `docs/plans/designcraft.md` | `craftkit-plugin-layout` | `src/view.rs` | `layout.ruler.set`, `layout.view.zero.set`, `layout.preference.set` | `crates/craftkit-plugin-layout/tests/view.rs` | §1295 |

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

The order is the order of the surface, from the outside in: the spread, the
page, the layer, the guides, then the item — its own facts, its placement,
its paint, its repetition, its alt text — and then the two things that are
read rather than written, the story and the table's measurements, with the
placeholder text last because it depends on a frame and a length.

**Arc M comes at the very end**, because it is about how the document is
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

**The refusals** are the tests that matter. An index outside a spread, a
size of zero, a page that has no such guide, a set of one item to align, a
count of zero to repeat, and a length of zero to fill are the places where the
house rule — *refuse, do not clamp* — and a comfortable port would disagree.

## 5. What it does not do

**Later work.** These are named by the survey and are not arcs now.

- **The parent spread as an entity of its own** is later because the layout
  battery's spread is a flat container, and an entity with its own per-page
  overrides is a change to the model before it is a change to the battery.
- **Section and chapter numbering** is later because a section is a range of
  pages with a number of its own, and the tree has no such range.
- **Alternate and liquid layouts** are later because a layout that changes
  with the page it is on is a derived fact the tree does not compute.
- **A shuffled spread** is later because it is an arrangement rule over
  whole spreads, not a command about one spread.
- **A shear, a flip, and clearing transformations** are later because the
  tree's items carry a rotation and a rectangle, and a transform stack is a
  third shape.
- **Inner shadow, glow, bevel, satin and feathering** are later because the
  layout battery's shadow and radius facts are page-level, and these are
  per-item effects.
- **Text wrap, fitting content to a frame, gridifying a frame and threading
  text frames** are later because each is a layout engine over a frame, and
  the tree has no frame layout yet.
- **Kerning, OpenType features, vertical writing and story direction** are
  later because they are typography machinery, and the upstream's colour,
  typography and font machinery has no counterpart in any of the batteries.
- **Cell and table styles** are later because a style is a named set of
  facts, and the tree's tables hold rows, columns and cells.
- **Colour swatches and composite fonts** are later because both are
  resources a document refers to, and the tree has no resource table.
- **Annotation notes** are later because a note that can be split is a
  document-level editorial structure.
- **Conditional visibility, form fields, XML tagging, preflight as a rule set
  and multi-state objects** are later because each is a rule evaluated
  somewhere other than at the moment of a command.
- **Flex layout, anchored objects and a primary text frame** are later
  because each is a structural relationship the tree's model does not have.
- **Converting text to a table, and a book** are later because both rewrite
  the document's structure rather than a fact of one thing.

**Out of scope.** These are not this plan's to carry, and no arc touches
them.

- **The upstream's file formats, export and print surfaces** are out of
  scope because they are a reader and a writer for another format, and the
  craftkit tree has none.
- **Selection, undo, redo, copy and paste** are out of scope because the
  core owns them, and a battery that added its own would be a second path.
- **The upstream's scripting, library and font surfaces** are out of scope
  because they are a host application's furniture, not rules about a
  document. The **preference** part of that row is not out of scope: the
  owner has said the saved view and preference state belongs in the
  document, and **Arc M** carries it.
- **Ink separations** are out of scope because they are a print surface.
- **Colour conversion** is out of scope because the vector battery owns it.
- **Paths** are out of scope because the vector battery owns them.
- **Caret-level typing** is out of scope because the text battery owns it.
- **The upstream's page transitions** are out of scope because they are a
  rendering surface, and this tree's front ends do not carry one.

## 6. The open questions

The survey wrote down what it could not read or decide, and those are the
questions this plan does not answer.

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
