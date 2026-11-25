# The wordcraft plan: what the upstream has and the prose battery does not

The upstream is a set of document libraries written in Rust, and this plan is
about the parts of it the `craftkit-plugin-prose` battery does not yet reach.
It is twelve arcs, and it reserves README sections **§1196 to §1207** — one
section per arc, in the order the arcs are written here.

## 0. The rule

Every arc here adds a command to the prose battery, and every one of them is
written the way the tree writes a command: a named rule, a typed parameter,
and a test.

## 1. What the survey found

| capability | upstream path:line | craftkit's state | verdict |
|---|---|---|---|
| A switch that turns the recording of tracked changes on and off | `crates/engine/src/cmd/review.rs:48` | absent — the battery stores revisions but nothing turns recording on | PORT |
| Making a hyperlink on a range of text | `crates/engine/src/cmd/insert.rs:58` | absent — `prose.block.links` only reads the links of a block | PORT |
| Character formatting beyond bold/italic/underline/colour/size (strike, super/subscript, caps, small caps, highlight, hidden, character spacing and scale) | `crates/engine/src/cmd/format.rs:57`, `crates/engine/src/cmd/format.rs:92` | partial — `prose.run.format` takes only bold, italic, underline, colour and size | PORT |
| A table formula (`=SUM(ABOVE)`) in a cell | `crates/engine/src/cmd/table.rs:201` | absent | PORT |
| Sorting a table by a column | `crates/engine/src/cmd/table.rs:197` | absent | PORT |
| Paragraph tab stops with leaders | `crates/engine/src/cmd/para.rs:221` | absent | PORT |
| Paragraph borders and paragraph shading | `crates/engine/src/cmd/para.rs:202`, `crates/engine/src/cmd/para.rs:189` | absent — the paragraph facts the battery has are alignment, indents, spacing and line spacing | PORT |
| Page and column breaks | `crates/engine/src/cmd/page.rs:33` | absent | PORT |
| A drop cap on the first paragraph | `crates/engine/src/cmd/insert.rs:130` | absent | PORT |
| Line numbering down the page | `crates/engine/src/cmd/page.rs:35` | absent | PORT |
| Right-to-left text direction | `crates/engine/src/cmd/para.rs:58` | absent — no direction fact anywhere in the battery | LATER |
| Table styles, cell borders and cell shading | `crates/engine/src/cmd/table.rs:73`, `:94`, `:99` | absent — the battery's table model has rows, columns, cells, a header flag and alt text only | LATER |
| The mail-merge workflow: choose recipients, write rules, finish the merge | `crates/engine/src/cmd/mailings.rs:75`, `:136`, `:207` | partial — `prose.merge` fills merge fields and says the text of each row, and writes no document | LATER |
| Spelling and grammar proofing, and a thesaurus | `crates/engine/src/cmd/review.rs:82`, `:141` | absent | LATER |
| Reading and writing `.docx` (and `.odt`, `.rtf`, `.md`, `.tex`) | `crates/docx/src/`, `crates/formats/src/` | absent — the whole craftkit tree has no docx/odt/rtf reader or writer | LATER |

The survey's finding is that the prose battery has readers without setters in
two of the rows, and the two failures are of different kinds. It can **read**
the links of a block and the characters of a run, and it can **hold** tracked
revisions, but in each of those cases there is no command that changes what
was read or recorded: `prose.block.links` only reads, `prose.run.format` takes
only bold, italic, underline, colour and size, and nothing turns the recording
of revisions on. A reader with no setter is half a capability.

The rest of the ten port rows are **absent** rather than partial, and they
fall into three groups: the paragraph and the page — tab stops with leaders,
paragraph borders, paragraph shading, page and column breaks, a drop cap, and
line numbering; the table — a formula in a cell and sorting by a column; and
the two character-formatting arcs, which split the survey's one row into the
facts that mark a run and the facts that change a run's width.

The rows the survey marked **later** — right-to-left text direction, table
styles and cell borders and cell shading, the mail-merge workflow, spelling
and grammar proofing and a thesaurus, and reading and writing office document
formats — are not arcs now. They are named in §5 with the reason each one
waits.

## 2. The arcs

An arc here is three edits and no more: a row in the port manifest
(`xtask/src/ports.rs`), a row in the README's claim table, and a section of
the README. The port manifest's rows take a document in this tree as their
`from`, so these arcs take **this plan** as their `from`: the plan is the
document the arcs come from, not the survey it follows.

| arc | rule | from | to | module | command | test | section |
|---|---|---|---|---|---|---|---|
| **A** | the recording of tracked changes can be switched on and off, and what the battery stores follows the switch | `docs/plans/wordcraft.md` | `craftkit-plugin-prose` | `src/revisions.rs` | `prose.revision.record` | `crates/craftkit-plugin-prose/tests/revisionrecord.rs` | §1196 |
| **B** | a range of text can carry a hyperlink, and the link is a thing the block can be read back for | `docs/plans/wordcraft.md` | `craftkit-plugin-prose` | `src/links.rs` | `prose.block.link.add` | `crates/craftkit-plugin-prose/tests/linkadd.rs` | §1197 |
| **C** | a run carries the character facts that mark it — strike, super/subscript, caps, small caps, highlight and hidden — and each one is read back as it was set | `docs/plans/wordcraft.md` | `craftkit-plugin-prose` | `src/characters.rs` | `prose.run.format.extended` | `crates/craftkit-plugin-prose/tests/charextended.rs` | §1198 |
| **D** | a run carries the character facts that change its width — character spacing and scale — and the width the battery reports is the width the facts give | `docs/plans/wordcraft.md` | `craftkit-plugin-prose` | `src/characters.rs` | `prose.run.spacing` | `crates/craftkit-plugin-prose/tests/charspacing.rs` | §1199 |
| **E** | a cell can hold a table formula, and the formula is read back as the formula it is | `docs/plans/wordcraft.md` | `craftkit-plugin-prose` | `src/tables.rs` | `prose.table.formula` | `crates/craftkit-plugin-prose/tests/tableformula.rs` | §1200 |
| **F** | a table can be sorted by a column, and the rows come back in the order the sort names | `docs/plans/wordcraft.md` | `craftkit-plugin-prose` | `src/tables.rs` | `prose.table.sort` | `crates/craftkit-plugin-prose/tests/tablesort.rs` | §1201 |
| **G** | a paragraph's tab stops carry leaders, and a leader is drawn where the stop was asked for | `docs/plans/wordcraft.md` | `craftkit-plugin-prose` | `src/stops.rs` | `prose.block.stops.set` | `crates/craftkit-plugin-prose/tests/stopleaders.rs` | §1202 |
| **H** | a paragraph can be given borders, and the borders are read back as they were written | `docs/plans/wordcraft.md` | `craftkit-plugin-prose` | `src/borders.rs` | `prose.block.paragraph.border` | `crates/craftkit-plugin-prose/tests/paragraphborder.rs` | §1203 |
| **I** | a paragraph can be shaded, and the shading is read back as it was written | `docs/plans/wordcraft.md` | `craftkit-plugin-prose` | `src/background.rs` | `prose.block.paragraph.shading` | `crates/craftkit-plugin-prose/tests/paragraphshading.rs` | §1204 |
| **J** | a page break and a column break can be placed, and the break is where it was asked for | `docs/plans/wordcraft.md` | `craftkit-plugin-prose` | `src/page.rs` | `prose.page.break` | `crates/craftkit-plugin-prose/tests/pagebreak.rs` | §1205 |
| **K** | the first paragraph of a page can carry a drop cap, and the cap is a fact of the paragraph | `docs/plans/wordcraft.md` | `craftkit-plugin-prose` | `src/paragraph.rs` | `prose.block.paragraph.dropcap` | `crates/craftkit-plugin-prose/tests/dropcap.rs` | §1206 |
| **L** | a page can be numbered line by line, and the numbering starts and ends where it was asked to | `docs/plans/wordcraft.md` | `craftkit-plugin-prose` | `src/pagenumbers.rs` | `prose.page.linenumber` | `crates/craftkit-plugin-prose/tests/linenumber.rs` | §1207 |

**Two survey rows become two arcs each, and that is why there are twelve.**
The survey's character-formatting row names two kinds of fact: the ones that
mark a run (strike, super/subscript, caps, small caps, highlight, hidden) and
the ones that change a run's width (character spacing and scale). Those are
two rules, and the second one is the rule that reaches the measurement of a
run's width — so it is an arc of its own, in the same module, with its own
test. The paragraph row is the same kind of split: borders and shading are two
facts, and they land in two modules, because the battery already keeps the
borders of a page and the background of a page in two places.

**Every other arc is one survey row, one module, one command, one test.**
Nothing here lands in two crates, so no arc needs two rows: the whole plan is
in the prose battery.

## 3. The order, and what each arc ships

| arc | ships | cost |
|---|---|---|
| A | the switch that turns the recording of revisions on and off | small — one fact on the session, and the revisions the battery already stores follow it |
| B | the hyperlink on a range of text | medium — a range of a run is addressed, and the target is a reference into the document |
| C | the character facts that mark a run | medium — six facts, each of them a flag the run carries |
| D | the character facts that change a run's width | medium — two facts, and the width the battery reports has to be computed from them |
| E | the table formula in a cell | medium — the formula is parsed and the result it gives is a value, not a string |
| F | the sorted table | medium — a comparison rule over the cells of a column, and the rows come back in a new order |
| G | the tab stops with leaders | small — the stop is already a fact of a paragraph; the leader is a second fact on it |
| H | the paragraph borders | small — the border is a fact on a paragraph, and the battery already draws page borders |
| I | the paragraph shading | small — a fill colour on a paragraph, and the battery already knows page backgrounds |
| J | the page and column breaks | medium — a break is a position in the flow, and the columns are a page fact the battery already holds |
| K | the drop cap | medium — a cap belongs to a paragraph and to the page it sits on, so the two have to agree |
| L | the line numbering down the page | medium — the count depends on the page's layout, and the battery does not lay a page out yet |

The order is the survey's order, and it is roughly the order of the surface:
the two reader-without-setter rows first, because they are the half-capabilities
the survey found; the character and table work next, because they are the
middle of the battery's existing surface; and the page and paragraph work
last, because they are the heaviest and the least often asked for.

**Nothing ships before Arc A.** Every arc from A on is a visible command and
each stands on its own, so unlike a plan whose early arcs are refactors, this
one has no invisible preparation: the first arc is already something an agent
can call. The plan is one batch, and the arcs are ordered for review, not for
dependency.

## 4. The tests

The house rule: **the crate's own rule has a test.** A rule with no test is a
habit. Each arc's test file is written in the same arc that adds the port row.

| arc | the test that must exist |
|---|---|
| A | with recording off, an edit stores no revision; with recording on, it stores one; the state that was set is the state that is read back |
| B | a link added to a range is listed by `prose.block.links`; a range that is not in the block is refused; a link with no target is refused |
| C | each of the six facts is read back as it was set; a fact that was not set is absent, not false; a fact set to a value the run does not take is refused |
| D | a run with spacing set reports the width that spacing gives; a run with scale set reports the width that scale gives; a run with neither reports the width it had before |
| E | a formula is read back as the formula it is; a cell whose formula names a column that is not there is refused, not clamped; a cell with no formula reports no value |
| F | the rows come back in the order the sort names; a column that is not in the table is refused; sorting by a column of equal cells keeps the rows the order they had |
| G | a stop with a leader reports the leader; a stop without one reports none; a stop past the margin is refused |
| H | the borders are read back as they were written; a border with an impossible weight is refused |
| I | the shading is read back as it was written; a shading with no colour is refused rather than read as no shading |
| J | a page break is where it was asked for; a column break is where it was asked for; a break past the end of the flow is refused |
| K | the drop cap is a fact of the paragraph it was set on; a drop cap on a paragraph that is not the first of its page is refused |
| L | the numbering counts the lines it counts; the count starts where it was asked to start and ends where it was asked to end |

The **refusals** are the tests that matter. A formula naming a column that is
not there, a stop past the margin, a drop cap on a paragraph that is not the
first of its page, and a break past the end of the flow are the places where
the house rule — *refuse, do not clamp* — and a comfortable port would
disagree.

## 5. What it does not do

**Later work.** The survey marked these as later, and this plan does not argue
with it. **Right-to-left text direction** is later because the survey found no
direction fact anywhere in the battery, and a direction is not one field but a
decision about every paragraph, every table and every run in the document —
it is a port of the whole model, not of a command. **Table styles, cell
borders and cell shading** are later because the battery's table model holds
rows, columns, cells, a header flag and alt text only; giving a cell a border
means a cell becomes a thing with a shape, and that is a change to the model
before it is a change to the battery. **The mail-merge workflow** — choosing
recipients, writing the rules, finishing the merge — is later because
`prose.merge` fills merge fields and says the text of each row, and a workflow
that ends in a written document is a document the battery has to be able to
write, which is the next item on this list. **Spelling and grammar proofing,
and a thesaurus** are later because a proofing tool needs a dictionary the
tree does not carry and cannot fetch, and a thesaurus needs a corpus. **Reading
and writing `.docx`** (and `.odt`, `.rtf`, `.md`, `.tex`) is later because it
is a reader and a writer for another format, not a rule in the battery: the
whole craftkit tree has no such reader or writer, and a format port is a
project, not an arc.

## 6. The open questions

The survey wrote down what it could not read or decide, and those are the
questions this plan does not answer.

- **Whether a capability in a crate the survey never opened is already
  ported elsewhere.** The survey read the upstream's index documents and
  command lists, but not every upstream crate; a capability living in a crate
  it never opened may already be ported elsewhere in craftkit. What would
  settle it is a reading of the remaining upstream crates against the tree.
- **What craftkit's other batteries already hold.** The survey did not verify
  what the shapes, vector, text, film and pdf batteries hold, so a capability
  this plan calls absent may be covered by one of them. What would settle it
  is a survey of those batteries with the same written contract.
- **Whether craftkit's core offers anything of these already.** Whether the
  core — not only the battery — already offers anything about page breaks,
  drop caps, tab stops, line numbering, track-changes switching or character
  spacing is **unsure**: the survey grepped the battery crate only. What
  would settle it is a grep of the core and the SDK, and an arc that finds a
  core capability is a different arc than the one written here.
- **Whether a capability listed only in the upstream's parity documents is in
  either table.** The upstream's file-format and layout parity documents were
  not read, so a capability listed only there is in neither the port rows nor
  the later rows of this plan.
