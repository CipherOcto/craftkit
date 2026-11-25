# The pdfcraft plan: what the upstream has and the pdf battery does not

The upstream is a set of PDF libraries written in Rust, and this plan is about
the parts of it the `craftkit-plugin-pdf` battery does not yet reach. It is
fourteen arcs, and it reserves README sections **§1182 to §1195** — one
section per arc, in the order the arcs are written here.

## 0. The rule

Every arc here adds a command to the PDF battery, and every one of them is
written the way the tree writes a command: a named rule, a typed parameter,
and a test.

## 1. What the survey found

| capability | upstream path:line | craftkit's state | verdict |
|---|---|---|---|
| Find text across the document (matches per page) | `pdfcraft/crates/engine/src/find_replace.rs:264` | absent | PORT |
| Move pages (reorder, earlier/later) | `pdfcraft/crates/organize/src/lib.rs:529` | absent | PORT |
| Duplicate pages | `pdfcraft/crates/organize/src/lib.rs:565` | absent | PORT |
| Insert a blank page | `pdfcraft/crates/organize/src/lib.rs:609` | absent (the battery's `pdf.page.add` takes a raw object dictionary) | PORT |
| Reverse the page order | `pdfcraft/crates/organize/src/lib.rs:544` | absent | PORT |
| Insert pages taken from another file | `pdfcraft/crates/organize/src/import.rs:109` | absent | PORT |
| Link annotations (add, edit, list, delete) | `pdfcraft/crates/automation/src/links.rs` | absent (only `pdf.dests.read` exists) | PORT |
| Optional-content layers switched on and off | `pdfcraft/crates/engine/src/live_scripts.rs:163` | partial (`pdf.layers.read` reads the tree, nothing sets it) | PORT |
| Comment status, lock and replies | `pdfcraft/crates/automation/src/comments.rs` | partial (`pdf.comment.add`, `pdf.comment.read`, `pdf.comment.info`, `pdf.comment.flatten`; no status or lock) | PORT |
| Accessibility check of a document (PDF/UA style report) | `pdfcraft/crates/a11y/src/lib.rs:635` | absent (`pdf.structure.read` reads the tree only) | PORT |
| XFDF that carries comments, not only field values | `pdfcraft/crates/xfdf/src/xfdf.rs:701`, `:1094` | partial (`pdf.form.data.import`/`export` carry field values only, and say so) | PORT |
| Preflight checks for transparency, ICC output intent, spot colours | `pdfcraft/crates/preflight/src/transparency.rs`, `icc.rs`, `spot.rs` | partial (the battery's `pdf.preflight` checks declared parts, metadata, fonts, actions and annotations) | PORT |
| Page numbering / Bates stamping | `pdfcraft/crates/edit/src/marks.rs:379` | absent | PORT |
| Colour-space conversion of page content (gray, rgb, cmyk) | `pdfcraft/crates/edit/src/convert_colours.rs:417` | absent (`pdf.convert` is the PDF/A conversion, a different thing) | PORT |
| Export of the embedded images of a page | `pdfcraft/crates/engine/src/export.rs:86` | partial (`pdf.figure.read` describes figures, nothing exports them) | PORT |

The survey's finding is that the PDF battery is thin in two ways. It has
little **page management** — nothing reorders, duplicates, reverses or inserts
a page, and the one page-adding command takes a raw object dictionary rather
than a page a caller can think of. And it has readers without setters: the
battery can read destinations, layers, comments, the structure tree, figures
and form data, and in each of those cases there is no command that changes
what was read. A reader with no setter is half a capability.

Four of the fifteen rows are **partial** rather than absent, and each partial
says so in the battery's own terms — the layer tree can be read but not
switched, the comments can be read but not given a status, the XFDF carries
field values and says so, the preflight checks a declared set of parts and
nothing about colour. The partial rows are the strongest candidates for a
port, because the reading half is already in the tree and only the writing
half is missing.

The rows the survey marked **later** — extracting pages into new files, text
replace, running a document's scripts, signing, OCR, export to other
document formats, optimizing, print imposition, page enhancement, and
auto-tagging a structure tree — are not arcs now. They are named in §5 with
the reason each one waits.

The survey marked one row **skip**: measuring area, distance and perimeter on
a page, because craftkit measures in its vector and CAD batteries. That is
named in §5 too.

## 2. The arcs

An arc here is three edits and no more: a row in the port manifest
(`xtask/src/ports.rs`), a row in the README's claim table, and a section of
the README. The port manifest's rows take a document in this tree as their
`from`, so these arcs take **this plan** as their `from`: the plan is the
document the arcs come from, not the survey it follows.

| arc | rule | from | to | module | command | test | section |
|---|---|---|---|---|---|---|---|
| **A** | the document can be searched, and the search answers with the pages it matched on | `docs/plans/pdfcraft.md` | `craftkit-plugin-pdf` | `src/find.rs` | `pdf.find.text` | `crates/craftkit-plugin-pdf/tests/find.rs` | §1182 |
| **B** | a page can be moved, and the page order it lands in is the order that is written | `docs/plans/pdfcraft.md` | `craftkit-plugin-pdf` | `src/pageorder.rs` | `pdf.pages.move`, `pdf.pages.reverse` | `crates/craftkit-plugin-pdf/tests/pageorder.rs` | §1183 |
| **C** | a page can be duplicated, and the copy is a page of the document, not a reference to one | `docs/plans/pdfcraft.md` | `craftkit-plugin-pdf` | `src/pagecopy.rs` | `pdf.pages.duplicate` | `crates/craftkit-plugin-pdf/tests/pagecopy.rs` | §1184 |
| **D** | a blank page can be inserted, and it is inserted where it was asked for | `docs/plans/pdfcraft.md` | `craftkit-plugin-pdf` | `src/pageinsert.rs` | `pdf.page.blank.insert` | `crates/craftkit-plugin-pdf/tests/pageinsert.rs` | §1185 |
| **E** | pages of another document can be inserted, and they are inserted as pages of this one | `docs/plans/pdfcraft.md` | `craftkit-plugin-pdf` | `src/pageimport.rs` | `pdf.pages.import` | `crates/craftkit-plugin-pdf/tests/pageimport.rs` | §1186 |
| **F** | a link annotation is a thing the battery can add, read, change and remove | `docs/plans/pdfcraft.md` | `craftkit-plugin-pdf` | `src/links.rs` | `pdf.links.add`, `pdf.links.list`, `pdf.links.edit`, `pdf.links.delete` | `crates/craftkit-plugin-pdf/tests/links.rs` | §1187 |
| **G** | an optional-content layer can be switched on and off, and what the battery reads is what the document shows | `docs/plans/pdfcraft.md` | `craftkit-plugin-pdf` | `src/layers.rs` | `pdf.layers.set` | `crates/craftkit-plugin-pdf/tests/layers.rs` | §1188 |
| **H** | a comment can be given a status, locked, and answered | `docs/plans/pdfcraft.md` | `craftkit-plugin-pdf` | `src/commentstatus.rs` | `pdf.comment.status`, `pdf.comment.lock`, `pdf.comment.reply` | `crates/craftkit-plugin-pdf/tests/commentstatus.rs` | §1189 |
| **I** | a document can be checked for accessibility, and the check is a report | `docs/plans/pdfcraft.md` | `craftkit-plugin-pdf` | `src/a11y.rs` | `pdf.a11y.check` | `crates/craftkit-plugin-pdf/tests/a11y.rs` | §1190 |
| **J** | the XFDF the battery writes carries the comments as well as the field values | `docs/plans/pdfcraft.md` | `craftkit-plugin-pdf` | `src/xfdf.rs` | `pdf.xfdf.export`, `pdf.xfdf.import` | `crates/craftkit-plugin-pdf/tests/xfdf.rs` | §1191 |
| **K** | the preflight report says what the colours of the document are | `docs/plans/pdfcraft.md` | `craftkit-plugin-pdf` | `src/preflightcolour.rs` | `pdf.preflight.colour` | `crates/craftkit-plugin-pdf/tests/preflightcolour.rs` | §1192 |
| **L** | a page can be numbered, and the number is stamped on the page | `docs/plans/pdfcraft.md` | `craftkit-plugin-pdf` | `src/pagenumbers.rs` | `pdf.marks.number` | `crates/craftkit-plugin-pdf/tests/pagenumbers.rs` | §1193 |
| **M** | the colours of the page content can be converted, and the conversion is written into the document | `docs/plans/pdfcraft.md` | `craftkit-plugin-pdf` | `src/colourconvert.rs` | `pdf.colour.convert` | `crates/craftkit-plugin-pdf/tests/colourconvert.rs` | §1194 |
| **N** | the images a page carries can be exported, and what is exported is what the page holds | `docs/plans/pdfcraft.md` | `craftkit-plugin-pdf` | `src/imageexport.rs` | `pdf.images.export` | `crates/craftkit-plugin-pdf/tests/imageexport.rs` | §1195 |

**Arc B carries two commands because reversing the page order is a reordering
of the page order.** The survey lists moving pages and reversing them as two
rows, and they are two rows; they are one arc because they are one rule — the
page order is a thing the battery can write — and because they land in one
module and one test. Every other arc is one survey row, one module, one
command, one test.

Nothing here lands in two crates, so no arc needs two rows. The writer
(`craftkit-pdf-writer`) is not touched by this plan: every arc is a command
the PDF battery runs over a document it already opens, and the bytes on disk
are the writer's business. **Arc E is the one that could need it** — a page
taken from another file is written by the writer — and if that turns out to
be so, Arc E grows a second row and a second section, and §1186 stops being
the last section of the plan's range.

## 3. The order, and what each arc ships

| arc | ships | cost |
|---|---|---|
| A | the search: matches per page, with the page they were found on | medium — a text reader the battery does not have yet, and the matching rules are the work |
| B | the page order: move a page, and reverse the order | small — the page tree is already a tree, and both are permutations of it |
| C | the duplicated page | small — a copy of a page that already exists in the document |
| D | the blank page | small — a page with no content, placed where it was asked for |
| E | the inserted pages from another file | large — a second document open at once, and a copy across documents |
| F | the link annotations | medium — four commands over one annotation kind, and the link's target is a reference into the document |
| G | the switched optional-content layer | medium — a setter for a tree the battery can already read |
| H | the comment's status, lock and replies | medium — three commands, and a reply is a comment with a parent |
| I | the accessibility report | medium — the checks are many and each one is a rule of its own |
| J | the XFDF that carries comments | medium — two commands, and the format's comment half has to be read as well as written |
| K | the colour preflight | medium — three checks, and the output intent is a document-level fact |
| L | the page numbers and the Bates stamps | medium — a stamp is content, and the numbering has to be stated as a rule |
| M | the converted page content | large — the content stream is rewritten, and a wrong conversion is a wrong document |
| N | the exported images | medium — the images are decoded and written out, and a decoder is a thing the battery does not have |

The order is the survey's order, and it is roughly the order of the surface:
search and page management first, because they are what a caller asks for
most; the annotation and comment work next, because it is the middle of the
battery's existing surface; the reports and the conversions last, because
they are the heaviest and the least often wanted.

**Nothing ships before Arc A.** Arcs B through N are each a visible command
and each stands on its own, so unlike a plan whose early arcs are refactors,
this one has no invisible preparation: the first arc is already something an
agent can call. The plan is one batch, and the arcs are ordered for review,
not for dependency — with the exception of E, which waits on the open question
in §6 about holding two documents at once.

## 4. The tests

The house rule: **the crate's own rule has a test.** A rule with no test is a
habit. Each arc's test file is written in the same arc that adds the port
row.

| arc | the test that must exist |
|---|---|
| A | a term that is on page two is found on page two and not on page one; a term that is nowhere is reported as no matches; a term with no text at all is refused rather than matched against an empty page |
| B | a page moved earlier is earlier in the page tree; a page moved later is later; the pages that were not moved keep their place; reversing twice is the order it started with |
| C | the duplicated page holds the same content as the page it was copied from; the copy is a new page and changing it does not change the original |
| D | the blank page is inserted at the position it was asked for; a position past the end of the document is refused, not clamped |
| E | a page of the other document comes into this one as a page of this one; a document that cannot be opened is an error, not an empty insertion |
| F | a link added is listed; an edit changes what the list reports; a deleted link is not listed; a link to a target that does not exist is refused |
| G | a layer switched off is not in what the battery reads as visible; a layer switched on is; the state that was written is the state that is read back |
| H | a comment's status is read back as it was set; a locked comment refuses an edit; a reply is a comment with the one it answers as its parent |
| I | a document with an untagged figure is reported; a document that passes the check is reported as passing; the report names what it checked and what it found |
| J | the XFDF written carries the comments; the XFDF read brings the comments back, with their status and their replies |
| K | a document with a transparency group is reported; a document with no output intent is reported; a spot colour is reported |
| L | the numbers stamped on the pages are the numbers that were asked for, and the stamp is content of the page |
| M | a page converted to gray no longer carries rgb content; a converted page still renders the same shapes; a conversion that cannot be done is refused |
| N | the images exported are the images the page holds, and the count matches; a page with no images exports nothing rather than failing |

The **refusals** are the tests that matter. A page number past the end of the
document, a link to a target that does not exist, and a colour conversion
that cannot be done are the three places where the house rule — *refuse, do
not clamp* — and a comfortable port would disagree.

## 5. What it does not do

**Later work.** The survey marked these as later, and this plan does not
argue with it: extracting pages into new files; text replace; running a
document's scripts; signing a document; OCR; export to other document
formats; optimizing (reduce size); print imposition; page enhancement
(deskew, denoise, binarize); and auto-tagging a structure tree. Each of them
is a larger surface than the fourteen arcs above, several of them need a
decision this plan does not make, and one of them — text replace — the survey
called later on size rather than on design.

**Out of scope.** Measuring area, distance and perimeter on a page is not
ported: craftkit measures in its vector and CAD batteries, and a PDF battery
that measured would be a second place the same answer lives. The survey
marked this row skip, and the survey also said the verdict is the weakest row
it wrote — see §6.

**Not touched either.** The upstream's rendering, viewer and application
surfaces are front-end concerns with no place in a battery, and they were
deliberately not surveyed.

## 6. The open questions

The survey wrote down what it could not read or decide, and those are the
questions this plan does not answer.

- **Whether the upstream's text editing could be ported at all.** The
  "later" verdict for text replace is a size judgement, not a design one, so
  it is not settled by this plan.
- **Whether inserting pages from another file is really a port.** It needs a
  second document open at once, and the survey did not check whether the
  battery's session model can hold two. Arc E waits on this: what would
  settle it is a reading of the session model, and a decision about whether
  the second document is a parameter or a second session.
- **Whether craftkit has a measure capability elsewhere.** The survey only
  grepped the PDF battery and the image crate, so the "skip" verdict on
  measurement is the weakest row it wrote. What would settle it is a grep of
  the whole tree for the measure capability, and if the vector and CAD
  batteries do not cover it after all, the row is a PORT and not a SKIP.
- **Whether the PDF writer supports an incremental save.** The survey found
  no command id announcing it, but did not read the writer's source. What
  would settle it is a reading of `craftkit-pdf-writer`; if it does, some of
  these arcs get cheaper, because a write that is incremental does not
  rewrite the whole document.
- **The upstream's rendering, viewer and application surfaces** were
  deliberately not surveyed: they are front-end concerns with no place in a
  battery.
- **The upstream's PDF/X and Arlington validation and its linearization** are
  gaps in the upstream's own docs, so they are not candidates.
