# The gap survey: what of the reference is still not in this tree

The port has been going for a hundred and thirty arcs, and the question
this document answers is the one the port exists to answer: **what of the
reference tree is still not in this tree, and why?**

The answer is short: **the reading surface is complete for this tree's
model.** What is left is a list of things this tree has decided not to
read, a list of things it has decided to read coarsely, and a list of
things nobody has asked for. None of the three is a gap that a port could
close.

**And the claim was checked, which is the only way a claim of that kind
is worth anything.** The check found two things: the **flags** of a file
were read as one spelling each, and a file's other spelling slipped past
(§1098); and the **quartile method** of a box and a whisker was not read
at all (§1099).

This is the evidence for that statement, and it is the evidence the *port
manifest* already holds: 240 rows, each one a rule that came from the
reference and landed in this tree, with the section that says what it
became.

---

## The method

Three sources, all of them the tree's own:

| source | what it gave |
|---|---|
| the reference's **model** (`crates/model/src/features.rs` and its two sub-modules) | the list of what a spreadsheet can hold: every table, chart, pivot, comment, condition, validation, protection, print setting and line fact it knows |
| the reference's **reader** (`crates/xlsx/src/`) | how each of those is read out of a file, and which of them are not read at all |
| this tree's **port manifest** (`xtask/src/ports.rs`) and the tree's **registry** | where each of them landed: 240 rules, 145 in the grid battery, 73 in the reader, 18 in the tree's own gate, and the rest in the crates under them |

The reference's own model is the spine of the table below, because it is
the reference's own answer to "what can a spreadsheet hold".

---

## The reading surface, feature by feature

| the reference's feature | this tree | where |
|---|---|---|
| cells: values, kinds, formulas | **read** | the reader (§905), the formula's own kinds (§1063–§1064) |
| shared formulas, array formulas, data tables | **read** | the reader (§905), the kinds of a formula |
| the number formats, the fonts, the fills, the borders, the alignments, the protections | **read** | the styles part (§906) |
| the named styles, the differential formats, the theme, the palette | **read** | the styles part, the theme |
| the dates: the 1900 and the 1904 systems | **read** | the reader |
| the shared strings, the rich text, the phonetic readings | **read** | the reader, the readings (§1031) |
| the checkboxes | **read** | the reader, the battery (§1029) |
| the rows and the columns: the size, the hidden mark, the outline, the collapsed mark, the custom size, the format | **read** | the reader (§1094) |
| the merges | **read** | the reader, the battery |
| the views: the gridlines, the headings, the zoom, the frozen panes, the mode, the right-to-left, the outline symbols, the zeros | **read** | the reader (§1081) |
| the protection of a sheet | **read** | the reader, the battery (§951) |
| the conditional formats, and the ones of the extension list | **read**, and the ones of the extension list are **reported** | the reader, the battery (§1017–§1035) |
| the data validations | **read** | the reader, the battery |
| the auto filter and the advanced filter | **read** | the reader, the battery (§1010–§1016) |
| the comments, the notes, the replies, the box and the corner of a note | **read** | the reader, the battery (§978) |
| the hyperlinks | **read** | the reader, the battery (§937) |
| the tables, their fields, their totals row, the way of a total, the label of the total | **read** | the reader (§454, §526–§527) |
| the formula of a calculated column, the unique name of a column, the XML a table is bound to | **read**, and the tree's tables have no place for them: they are **counted and said** | the reader (§1096) |
| the charts: the kinds, the series, the legend, the quartiles, the trendlines | **read** | the reader, the battery (§1072–§1073) |
| the chart extensions: the waterfall, the funnel, the treemap, the sunburst, the box and whisker, the histogram | **read** | the reader (§1073) |
| the **quartile method** of a box and a whisker | **not read** | the reference reads it and this tree does not (§1099) |
| the pictures | **read** | the reader, the battery (§1032) |
| the sparklines | **read** | the reader, the battery (§940) |
| the pivots: the fields, the groupings, the filters, the orders, the ways of showing the values, the grand totals | **read** | the reader, the battery (§1074–§1078) |
| the print: the paper, the orientation, the margins, the scale, the fit, the header and the footer, the breaks, the area, the titles, the gridlines, the headings, the centring, the marks, the edit ranges, the kinds of pages | **read** | the reader, the battery (§1079, §968–§969) |
| the defined names | **read** | the reader, the battery (§1049) |
| the calculation properties | **read** | the reader |
| the feature bags | **read** | the reader |
| the encrypted package: the CFB, the RC4, the AES | **read** | the reader, the crypto |
| the **shapes**, the **drawings**, the **ink** | **not read** | out of scope: a drawing is a drawing, and this tree draws its own |
| the **rich values** and the **linked data types** | **not read** | out of scope: the in-cell data of a newer file |
| the **slicers** | **not read — and the reference does not read them either** | the reference passes them through on the way out; this tree has slicers of its own, made by commands |
| the **query tables** | **not read — and the reference does not read them either** | a query table's definition hangs off the part that is rewritten |
| the **writer** | **not read** | out of scope: this tree reads, and the workbook is not written back |
| **XLSB**, **ODS**, **XLS**, **CSV** | **not read** | out of scope: this tree reads XLSX |

---

## What is left, and what it would take

Three lists, and none of them is a gap that a port could close.

### The reading this tree has decided to do coarsely

These are **decisions**, and each of them is written down where it was
made:

- **The chart kinds.** The reference knows twenty-five; this tree knows a
  handful of coarse ones, and a chart of a kind it does not have is read,
  named and **counted unread**. The tree's model is a document, and a
  chart in a document is a chart with a kind and a range.
- **The table's own filter.** The filters of this tree are the **sheet's**,
  so a table with a filter of its own keeps its table and loses its
  filter (§1096 and the note that says so).
- **The look of a table.** The banded rows, the banded columns and the
  emphasised first and last column are a **style**, and a style is a
  drawing (§454).
- **The pivot.** This tree's pivot gathers the rows by one field, spreads
  the groups over one and puts one together; a pivot of a file that has
  more of any of them is left out rather than half-carried (§932).

### The reading this tree has refused

These are **refusals**, and each one says what it would take:

- the **regular expressions** (`REGEXTEST`, `REGEXEXTRACT`,
  `REGEXREPLACE`) — `not supported yet: a regular expression`;
- the **cube functions** (`CUBESET`, `CUBEMEMBER`, `CUBEVALUE`, …) — `not
  supported yet: a cube`;
- the **spill** of a dynamic array (§1059), the **union** of ranges
  (§1060), the **arrays** and the **lambdas**;
- `NOW`, `TODAY` and `INFO`, which are refused on purpose: a file that
  says the time says the time of the file, and a reading that is asked
  for twice gives two answers.

### The reading nobody has asked for

The **XLSX writer**, the other **formats**, the **macro engine**, the
**drawings** and the **shapes**, the **in-cell images**, the **pivot
cache records** and `docProps/custom.xml`. These are not gaps in the
design: they are things this tree has not been asked for, and they are
named in `KNOWN-GAPS.md` so that nobody reads them as defects.

---

## What this survey does not settle

- **It is a survey of the reading surface, not of the behaviour.** The
  tree's commands — 251 in the grid battery — are the tree's own, and the
  reference's commands are not a target: the tree's rules win over the
  reference's rules.
- **"Read" means the reader reads it**, not that the tree's model keeps
  it. The facts the model has no place for are read, counted and reported
  (§1094, §1096), and that is the tree's own way of saying *this is not
  here*.
- **A kind the tree does not have is not an error.** The reader names it
  and counts it, and the tree does not refuse the file.
- **The survey is a reading, not a proof.** The manifest is the proof:
  240 rules, each one in the tree, each one with a section that says what
  it became — and the gate checks that every one of them is there.
