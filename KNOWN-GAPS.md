# Known gaps

What this tree does **not** do, and what its gates cannot see. This file is the
honest list: a gap written down is a gap somebody can close, and a gap nobody
wrote down is a gap somebody will rediscover as a surprise.

Nothing here is a code gap unless it says so. Some of these are things this
tree has not been asked for, and they are here so that nobody reads them as
defects.

---

## 1. The outer gate and the inner gate check different halves

The tree has **two** gates, and they do not overlap.

- The **inner** gate is `crates/craftkit-gates`. It knows about the crates: the
  house deny attribute, the layering, what a plugin may depend on, the bounds
  the README names, that a battery declares its commands as rows of a table,
  that every test file is a test the tree runs, and that a test does not
  litter. It runs as `cargo test -p craftkit-gates`.
- The **outer** gate is `xtask/`. It knows about the **repository**: the layer
  table in `docs/architecture.md` and the inner gate's own table agree, every
  gate the tree names is actually run, every document link in the prose points
  at a file that is in the tree, the `mod` groups are sorted, every crate is a
  member of the workspace, the xtask holds itself to the same deny list as the
  crates, and every rule the v1 tree had and this one does not is **named**
  with the reason. It runs as `cargo xtask ci`.

Two gates means two places to look, and that is a cost. It is paid on purpose:
the inner gate is the tree's own, and it can be run by anybody who has the
tree, without the repository's tooling; the outer gate is the repository's,
and it knows the things the tree cannot know about itself — its own
documentation, its own workflow, and its own history.

**What the outer gate cannot see.** It reads the tree as text, because a
checker that cannot be built when the tree does not compile is a checker that
is silent exactly when it is needed. So it does not run cargo to ask what a
crate depends on; the inner gate does that, by reading the manifests, and the
outer gate checks that the inner gate's *table* and the *document* agree.

## 2. What the tree's own gate cannot see

The gate in `crates/craftkit-gates` is a **reading** of the tree, not the tree
itself, and it has four honest limits:

- **It reads text, not cargo.** It parses the manifests and the crate roots.
  That is what makes it fast and hermetic, and what makes a manifest written
  in a form the reader does not know read as *no dependencies at all*.
- **The table of layers is written by hand.** A new crate is a change to the
  crate **and** to the table, and the outer gate keeps a second table that has
  to agree with the first: the tree's own (`crates/craftkit-gates`), the outer
  one (`xtask/src/layers.rs`) and the architecture document. A table generated
  from the manifests cannot catch a manifest that is wrong.
- **It does not hold the list of crates that may hold `unsafe`.** That list is
  in `crates/craftkit-alloc/tests/gate.rs`, which reads the whole workspace and
  says so.
- **A bound whose number cannot be worked out is not checked.** The rule
  "the numbers agree with the prose" evaluates the expressions; a bound
  written in a way the evaluator does not know is silently not checked, and
  the gate says nothing rather than guessing.

---

## 3. What the other gates cannot see

- **The clean-room gate checks names, not copying.** Copied code does not
  carry the product's name, so a gate that checked that would be a gate that
  could never pass. The rest is the review, and the table in
  `docs/clean-room.md` is what the reviewer reads.
- **The assets gate checks the tree's files and their attribution.** It cannot
  check that a drawing in code is a drawing and not a copy.
- **The dependency policy checks the licences a manifest claims.** It cannot
  check that anybody read them.
- **The deny attribute is a lint, not a proof.** `forbid(unsafe_code)` holds
  the code; a `no_mangle` that the platform looks up by name is the one
  exception, and it is named, in one list, with one marked place each.
- **The digest checks the README's last section.** The house's four
  headings came with §62, and every section from §115 on has all four of
  them; the fifteen sections before that are the old tree's, and the
  digest does not go back for them. A section written in the middle of
  the README would not be checked either — and sections are written at
  the end.

---

## 4. The doors are classified; the matrix is kept by the gate, and the doors are compared by a program

The tree has **six doors**, and the rule "a front end is thin" now reaches
all six (README §1116): the terminal door, the window, the browser door,
the agent door, the bridge and the a2ui door. The **transports** — the MCP
library and the control channel — are not doors, and the **studio** is a
**host**: its entry point is a front end, and its library is a command
book.

**The parity matrix is kept, not kept by hand.** The table is written — the
six doors side by side, cell by cell — and **every cell that says
something names the test that proves it**
([`docs/door-parity.md`](docs/door-parity.md)). The outer gate reads the
matrix and checks two things: that a cell which says something names a
test, and that the test it names is **there** — a file under a `tests`
directory, with a function of that name in it. A named test does not drift
when the code moves, and a line does.

What is still **not** checked by the gate is the *cells* themselves — that
is, the gate does not read a door and compare what it does with what the
table says, and it does not check that a named test is *about* the cell it
is in. The **harness** checks the marks (§1132); the gate does not.

**The doors are compared with each other, though.** The parity harness
(`apps/craftkit-parity`, §1128) drives five of the six doors with the same
five questions and says where they differ: the same commands listed, the
same message for the same command, the same plan published, the same word
for the undo, and every door naming the command it refused. It is the harness the survey asked for — a crate of its own and
**four** tables to change, not three: the tree's own layer table, the
outer gate's table, the architecture document and the README. The
**a2ui door** is not in it, because it speaks a2ui's own vocabulary and
there is nothing to compare.

**And the doors are compared with the session** (§1131): the harness asks
a session of its own the same questions, because the doors are compared
with each other and the **first** door is the anchor — so two doors that
are wrong in the same way agree with each other, and the session is what
says otherwise.

**And the matrix's marks are live** (§1132): the harness reads
`docs/door-parity.md` and compares each cell's **mark** with what the
door did, so a cell that says a door publishes a plan and the door
published none is a difference that names its line. And a door that does
not prune its list must list **everything** the session has: the
**bridge** — which runs only what it lists — is the one door that prunes.

What is left is the **words** in a cell read by a person, and that is
**not** a defect: the harness knows the tick and the dash and not whether
"✓ as a command" and "✓ the size only" are the right words, and the three
columns it does not ask about — the schema, the journal and the document's
tree — are still a reader's job. Nothing in the tree is broken by a table
that drifts, and the cells that matter most are each proved by a test the
tree runs.

**And the bridge's protocol changed** when the bridge became thin:
`resources/list` carries the **manifest** now and not the evaluator's ids,
because the manifest is the door that names every evaluator and the
registry is not a door at all. The bridge is a **proof** and not a
product, and a proof that changes shape is cheap to change now and would
not be later.

---

## 5. What the design honestly does not do

These are the README's own honest notes, and they are still true of the tree:

- **The linker is part of the design.** The anchor macro is what keeps a
  plugin in the binary. Without it, the plugin's contributions are missing and
  nothing says so.
- **A run-time command is declared, and there is no macro for it.** The
  SDK writes the fields and the reader (§1120), and what is left is the
  leak: a run-time command's names are `&'static str` and a name that
  arrives at run time cannot be static, so each one is leaked once.
- **A run-time command's declaration is checked against the call, and not
  against its body.** The **kind** is checked: the session refuses a value
  that is not of the kind the command declared, so a command that declares
  a text and reads a number is refused at the door (§1124). A compiled
  command is not refused this way — the compiler writes the reader and the
  declaration from the same struct — **unless** a kind says it takes a
  string and its own reader does not, which is a convention on the kind
  and not a promise of the compiler (§1129). The **required** flag is checked
  against the **call** (§1126): a parameter the declaration says is
  required and the caller did not give is refused. What is **not** checked
  is the flag against the **body**: a body that ignores a parameter it
  declared is not visible to the engine. The **other spelling** of a parameter with two
  carries its own kind now, and the schema publishes each spelling with
  its own shape (§1125) — and it carries it on **every** path: the
  hand-written declaration, the a2ui derive and the subprocess plugin all
  say what each spelling takes (§1129).
- **`mutates` and `undoable` are two flags, and the derive refuses only
  one combination.** A command that is undoable and does **not** change
  the document is refused by the macro — there is nothing to undo. A
  command that changes the document and says nothing about undoing is
  **allowed**: it changes the document and leaves no history, and the
  session judges it by the document and not by the flags. The two flags
  are consistent, and a run-time command declares both the same way.
- **A test file that is a copy of another is caught; a copy that differs
  by a byte is not.** The rule compares the bytes, and an empty test file
  is not caught either — it carries no test, so a copy of one is
  nothing (§1130). The rule reads the crates and the **xtask**, and not
  anything else.
- **A command that no test names is caught** (§1133): the outer gate reads
  every command the tree declares — the batteries' rows through the index,
  which knows the macros they are written with, and the kit's own from the
  crates that are not batteries — and says the ones that no test names.
  The rule is about the **name**: a test behind a feature flag still names
  the command, and a test that runs every command in a loop names none.
- **A kind that no test names is caught** (§1134): the same rule, read over
  the tree's **kinds** — a node kind or an evaluator — and a kind is named
  rather than run, so the rule is even weaker than the command's: a kind
  written about in a string is a kind the gate counts, and a kind that is
  created and read in a test is a kind the gate cannot tell from one that
  is only written about.
- **One name, one number — the private caps too** (§1160): the rule
  read the **published** caps and not the private ones, and the private
  ones were where the clashes were: the tree's published caps were
  already one name and one number each, and twenty-three private groups
  were not — the film battery's three `MAX_SIZE`, the grid battery's
  five `MAX_CELLS` and two `MAX_POINTS`, the pdf battery's five
  `MAX_DEPTH`, and more. The rule now reads every cap of a crate. The
  private caps stay private: the rule holds them to one name and one
  number and to nothing else, and nothing asks a test to prove them.
- **The rule asks for the name in the code** (§1159): a cap named in a
  comment is a name, and a cap named in the **code** of a test is a cap
  the test uses. Sixty-nine of the tree's caps were named in prose and
  in no code, because the batteries' caps lived in private modules and
  a test could not use one: the test said "a title over `MAX_PART` is
  refused" and typed 1 025 where the bound was 1 024, so a change of
  the bound would have left the test proving nothing. The batteries
  publish their caps now, the tests use them, and the rule reads the
  code view of the tests.
- **Five names that were two caps each are one cap each** (§1159): the
  prose battery had a `MAX_LABEL` for its captions and one for its
  figures, an `MAX_ENTRY` for its authorities and one for its index, and
  a `MAX_LEVEL` for its contents and one for its outline; the film
  battery had a `MAX_TEXT` and a `MAX_CUES` twice each. They are named
  apart now, and the tree keeps **one hundred and eleven** caps under
  one hundred and eleven names — the count of §1156 was the count of
  *names*, and five of them were two caps each.
- **One name, one number — the whole tree** (§1169): the rule of §1144
  and §1160 was **inside** a crate, and the tree published ten names
  that two crates published each: `MAX_DEPTH` in three, and nine more in
  two each. The §62 promise that two crates may not publish one bound's
  name was broken by §1159 — the batteries published their caps, and the
  cap registry never saw them, because a cap is not a registered bound.
  The rule now holds the **whole tree**: a published cap's name belongs
  to one crate, the gate says the ones that do not, and ten names are
  named apart. What stays: a **private** cap may still share a name with
  another crate's — it is not a promise, and nobody may rely on it.
- **The rule holds every crate to its own caps** (§1158): the rule of
  §1141 held the **kit's** limits to a test anywhere in the tree, and
  §1145–§1156 brought every other crate's caps under a test by hand.
  The rule now asks **each crate** for the proof of **its own** caps —
  its test files and the test parts of its own source — and the promise
  bounds follow the same rule. The tree was already ready: every
  crate's caps were named by that crate's own tests, so the rule was
  made strict without changing a behaviour. The other rules still ask
  the whole tree: a battery's command may be run by the studio's test,
  and that is the proof the tree wants.
- **The blind spot is closed** (§1157): the rule that reads the tree's
  caps — and its commands, kinds, migrations, panels, tools, evaluators,
  layers, formats and promise bounds — read the test **files** of the
  tree and not the `#[cfg(test)]` modules of its source, because it read
  the production code and could not tell a unit test from the code
  around it. The reader already knew the difference — it has to, to cut
  the tests out — and the rule now asks it for the other half: a cap
  named only by a unit test is no longer invisible to the tree's own
  gate. What stays is the weakness the tree already knows: a name in a
  comment is a name, and a name in a string is a name, and the unit
  tests widen the surface the rule looks at.
- **Every published cap in the tree is named by a test** (§1154–§1156,
  §1158, §1159): every published cap in the tree is named by a test, and
  **how many** there are is written once, in the README's counted block,
  which the gate checks (§1180). The last four were the two of the fixtures
  and the two of the kit: the fixtures' caps were named by their own
  unit tests and by nothing else, and the kit's two were proved by the
  **studio's** tests and by the **window's** — a proof in another
  crate's house. The kit has a test of its own for both now.
- **The rule that reads the tree's caps had a blind spot** (§1156): it
  looked in the `tests` of a crate and not in the `#[cfg(test)]` modules
  of its source, because it read the production code and could not tell a
  unit test from the code around it. A cap that only a unit test names
  was invisible to it — the two fixtures' caps were. The blind spot is
  closed (§1157): the reader hands back the tests as well as the
  production code. What stays is the weakness the tree already knows: a
  name in a comment is a name, and the unit tests widen the surface the
  rule looks at.
- **The camera's zoom is a third deliberate clamp** (§1156): a zoom
  above `MAX_ZOOM` is clamped, and the camera says so in its own
  words. The tree now has a test of its own for it. The art battery's
  rate and the sound battery's repeat count are the other two. **§1177
  settled the half of it that was open**: the camera says when it held
  a zoom (`Camera::set_zoom` returns it and `view.restore` answers
  `clamped`), and the camera's note about a zoom that is not a number
  was wrong — the code moved the camera to one, and the test passed
  because the camera was already at one. It does not move for it now.
- **The midi crate's caps are named** (§1152): the tracks of a file, the
  events of a file and the velocity of a note are each named by a test
  beside the crate. The **budget of the events** is one budget for the
  whole file and not one per track, so a file of many tracks is refused
  for its events and not for its tracks. The operation that changes how
  hard a note is struck said in one breath that a velocity outside the
  range is refused and that the tree clamps it; the tree refuses, and
  the doc says so now.
- **The zip crate's caps are named; two of them cannot be reached**
  (§1153): the size of a part is proved by a package that claims a
  part of more than the bound, and the tree reads none of it. The
  **number of the entries** is a number of sixteen bits in the record
  at the end of the directory, so the tree's bound of two hundred
  thousand is above the count the format itself can hold and can never
  be reached — a package with more entries keeps its numbers in the
  **ZIP64** record, and this reader refuses that one. The **total** a
  package may inflate to is a gigabyte of real bytes, and the tree
  counts the bytes it has read: the bound is reached only by reading a
  gigabyte, and that is not a test the tree runs. A bound that cannot
  bite is a bound that cannot be relied on, and the tree says so rather
  than changing the crate's behaviour to make it reachable — the same
  choice the kit's own unreachable bound got in §1141.
- **The audio crate's detectors say when they stop** (§1151): the
  onsets, the transients and the notes of a sound are a reading — the
  items that were found and whether the detector stopped at its bound —
  and the sound battery's answers carry the flag. A pitch **above** the
  detector's bound was given as a **wrong** pitch, an octave below the
  tone, because the search started at the bound and could not see the
  period of such a tone; the search sees it now and refuses it, which
  is what the crate's own words say ("a frame that cannot carry a
  pitch has no pitch rather than a wrong one").
- **A bound that cannot be reached** (§1151): the loudness meter holds
  state for sixty-four channels and the tree's own reader of a sound
  refuses more than **eight** channels, so the meter's bound cannot be
  reached through the tree. The sound battery refuses a material with
  too many channels rather than measuring it with fewer, and the guard
  is there for a reader that is less strict. Nothing says which of the
  two bounds is the right one.
- **A cap that no test can reach** (§1151): the pitch detector reads
  two million frames of two thousand samples each, and the sound it
  would take to reach that is larger than the machine that runs the
  test. The test names the cap and says what the tree reads; it cannot
  say what the tree would read. The same is true of `MAX_RESUMES` in
  the kit (§1141).
- **The xlsx reader's bounds now say what they do** (§1150): the bags
  of a file, the outline of a row and the columns of a sheet are read
  against a bound, and the file is told when the bound bites — the
  rule of §1146 and §1147, held to the xlsx reader. Three of the six
  bounds already said so; the other three were silent. The **bags were
  never read at all**: the reader asked for a relation of the kind
  `FeaturePropertyBag` and the kind of a relation is the last word of
  its type, which is written `featurePropertyBag`; the name is
  compared without regard to the case of its letters now, and the
  checkboxes of a file are read for the first time.
- **A cut is still a cut** (§1150): the outline of a row deeper than
  `MAX_OUTLINE` is cut to the limit and the file is told, and the
  columns of a sheet past its width are not read and the file is told.
  The file is read, not refused: the answer holds the truth of what
  was read, and the truth of what was not is in the warnings beside it.
- **The batteries' caps are published and named** (§1149, §1148,
  §1158, §1159): **every** published cap in the tree is named by a test
  beside the crate that keeps it. What §1148 and §1149 recorded — that
  fifty and thirty-two published caps in seventeen crates were named by
  no test — was closed by §1158 and §1159, which published the
  batteries' caps and asked each crate for the proof of its own. The
  caps a battery keeps **inside itself** are not the tree's rules: the
  rule holds a private cap to one name and one number and to nothing
  else (§1160). **How many** each battery publishes and keeps is written
  once, in the README's counted block, and the gate checks it (§1180).
  What stays is the rule's own weakness: the rule is about the **name**,
  so a cap named in a comment is a cap it counts, and a cap that
  **cuts** rather than refuses is named by the same test as one that
  refuses.
- **An address past the sheet was a cell** (§1149): a sheet is sixteen
  thousand columns wide and a million rows tall, and the reader of an
  address did not check either, so `XFE1` and `A1048577` were cells of
  a sheet that has neither. The address is refused now. The caps of the
  sheet's size are read in the **formula** path, and the two paths did
  not agree.
- **Two docs of the grid battery said the tree clamps** (§1149): the two
  caps of a fraction's denominator said "the tree clamps to it", and the
  tree refuses — a denominator with too many digits is not a fraction,
  and a code that is not a fraction is not a format. The docs say what
  the tree does now. A cap that is read only on one path is a cap with
  two doors: the digits of a **free** denominator are bounded by one cap
  and the value of a **fixed** one by the other.
- **The prose battery's caps are named, and so are the other
  batteries'** (§1148, §1158, §1159): every published cap of every
  battery is named by a test beside the battery. The outer gate's rule
  reaches only the **kit** (§1141), so nothing held a battery to it when
  §1148 was written; §1158 and §1159 gave every crate the rule and its
  own proof, and no published cap in the tree is now named by no test.
  **How many** each battery has is written once, in the README's
  counted block, and the gate checks it (§1180). The rule is about the
  **name**, and some of those caps cut rather than refuse — the chain
  of the styles is cut at `MAX_STYLE_CHAIN`, and a label is truncated at
  `MAX_LABEL` — so naming them is not the same as refusing them.
- **A source's title had no bound** (§1148): the parts of a source were
  bounded — the author 256 characters, the year 32, the tag 64, the
  other parts 1 024 — and the title was whatever a reader sent. The
  title is bounded now, at `MAX_PART`, and the test that claimed it was
  bounded found the hole because it did not pass.
- **The readers of the pdf battery now say when they stop** (§1146,
  §1147): the JavaScript name tree, the structure tree, the marked
  content, the figures, the comments and the attachments each read their
  tree to a depth, and each now **says** that it stopped — the
  JavaScript in the list of what it skipped, the other five in a
  `truncated` field of their answer. The flag is a flag and not a note:
  it says that the reader stopped and not *what* was not read. The
  image's pixels and the outline's items are caps that skip silently and
  have not been looked at.
- **One name, one number** (§1144): the `lists` phase reads the caps
  of every crate and says a name that is declared with two different
  numbers — the same rule the kit's own bound registry holds between
  crates, held inside one. The rule is about the **public** caps, and it
  compares the numbers as they are written: two caps that hold the same
  number and spell it differently are the same number, but nothing
  checks that the two are the same kind. The rule reads the **public**
  caps only, so a private cap that shares a name with a public one is
  invisible to it: the editing battery's `MAX_INDENT` was 1 584 points
  and the paragraph's was 10 000, and the private one is
  `MAX_EDIT_INDENT` now (§1148). The same happened in the grid
  battery, whose private caps for the text of a cell and of a word were
  both `MAX_TEXT` beside a published `MAX_TEXT` of ten thousand: they
  are `MAX_CELL_TEXT` and `MAX_WORDS_TEXT` now, and the prose
  battery's private `MAX_LINE_SPACING` is `MAX_EDIT_LINE_SPACING`
  (§1149).
- **The same name means two different numbers** (§1143): the prose
  battery keeps `MAX_SPACE` — ten thousand points in the paragraph and
  three hundred in the columns — and `MAX_LINE_SPACING`, a hundred in
  the paragraph and fifteen hundred and eighty-four in the editing. The
  kit's own bound registry refuses two crates publishing one name,
  because a reader would have no way to say which is which; the
  batteries' caps are private and the registry never sees them, so the
  confusion is still there. A test that names `MAX_SPACE` does not say
  which limit it means, and nothing in the tree holds a battery to one
  name, one number.
- **The tree clamps in two places, and both are deliberate** (§1142,
  §1145): the art battery **clamps** a rate outside the domain's bounds,
  and the sound battery **clamps** a repeat count into its bounds. The
  house rule says an out-of-range parameter is an error and not a
  silently clamped value, and these are the two places the tree does
  not do it. Both are documented in the code itself — the rate is "the
  application's rule" and the count is "a knob and not a fact about the
  material" — and both are recorded here rather than changed: a clamp
  the tree's own documentation promises is a decision, and changing it
  is the owner's. A test now says what each knob does. **§1177 settled
  the half of it that was open**: the clamps stay, and each of them now
  answers `clamped` when it held its value, so a clamp is no longer
  silent. What stays open is whether a knob should be a refusal at all,
  and the owner has decided that it is not.
- **A cap in a private module was textually public and practically
  private** (§1142): the batteries' caps were `pub const` inside private
  modules, so the outer gate read the text and saw a promise while no
  caller could name the number. The rule of §1141 therefore reached the
  **kit** and not the batteries. The batteries publish their caps at
  their roots now (§1159), and a **private** cap is a private cap: the
  rule holds it to one name and one number and to nothing else
  (§1160), and nothing asks a test to prove it.
- **A limit of the kit that no test names is caught** (§1141): the same
  rule, read over the **kit's own public limits** — the constants whose
  names say they are one, `MAX_…` and `…_LIMIT` — and the rule is about
  the **name**, so a limit named in a comment is a limit the gate counts.
  The rule is about the **kit**: the batteries keep their own caps, and
  the rule does not reach them yet.
- **The answer bound cannot bite** (§1141): the kit keeps a bound on the
  number of questions a command may ask and a bound on the number of
  times it may be answered, and the two are the **same number**. Every
  question is one answer, the answer bound is checked first, and the
  question bound never refuses anything. A limit that cannot bite is a
  limit that cannot be relied on; the tree says so rather than changing
  the kit's behaviour to make it reachable.
- **A bound that no test names is caught** (§1139): the same rule, read over
  the tree's **promise bounds** — the ones whose name says what they
  bound, `MAX_…_BYTES`, `MAX_…_MEMORY`, `MAX_…_STEPS` — and the rule is
  about the **name**. A cap on a count of entries is the program's own
  business and is not a promise, and the tree's own gate makes the same
  distinction. The rule says the number is named by a test; it does not
  say that the code refuses what is over it.
- **An evaluator that no test names is caught** (§1138): the same rule, read
  over the tree's **evaluators** — the gestures about the whole document —
  and the rule is about the **name**, so an evaluator named in a comment
  is an evaluator the gate counts. An evaluator is also a **kind**, so the
  kinds' rule held it to account twice; the evaluator's own rule is what
  holds the kit's own node count, which is written as a static and is not
  a kind at all.
- **A layer that no test names is caught** (§1138): the same rule, read
  over the tree's **layers** — the groups of contributions that depend on
  each other — and the rule is about the **name**. The core layer is
  written as a static and is read as one.
- **A format that no test names is caught** (§1138): the same rule, read
  over the tree's **formats** — the versions of the document format — and
  a format's name is its **tag**, not a word of its own.
- **The reader of the layers, formats and evaluators is a reader too**
  (§1138): it reads a static that writes its specification as
  `id: "…"` or `tag: "…"`, and it reads the **production code** only. A
  static that writes its name another way is a specification the gate
  cannot see, and nothing says so. A **mention** of a specification is
  not one either: the reader reads the block that opens after the name,
  and a type annotation that names a specification — the kit's own
  session holds a registry of evaluators and says so in a type — is not
  a specification. The reader said otherwise once, and the gate reported
  an evaluator and a layer that are not there (§1140).
- **Every evaluator is proved beside the battery that owns it** (§1138):
  all eighteen of the tree's evaluators are proved by a test beside the
  battery that owns them. What the proof says is the **answer** — the
  area, the count, the end of a track — and not that the evaluator
  answers every document it could be given.
- **A tool that no test names is caught** (§1137): the same rule, read over
  the tree's **tools** — a gesture that runs a command — and the rule is
  about the **name**, so a tool named in a comment is a tool the gate
  counts. The rule does not say that the gesture works: a tool that names
  a command that is there can still be a gesture that does nothing
  useful, and nothing checks the pointer or the keyboard.
- **The reader that finds the tools is a reader too** (§1137): it reads the
  tools a battery declares, where the attribute is at the start of its
  line, and it reads the **production code** only — so a tool written in a
  test is not a tool the tree has. The derive crate's own test holds a
  tool declaration in a string, and a reader that counted it would hold
  the tree to account for a tool that does not ship.
- **A panel that no test names is caught** (§1136): the same rule, read
  over the tree's **panels** — a battery's own window of commands — and
  the rule is about the **name**, so a panel named in a comment is a panel
  the gate counts.
- **A panel shows only its own layer's commands** (§1136): the rule that a
  plugin may not depend on another plugin is now a rule about **data** as
  well as about Cargo. The rule is about the **layer**, not the crate: a
  panel may show a command of a layer it requires, and two batteries that
  shared a layer would be two batteries the rule could not tell apart.
- **A migration that no test names is caught** (§1135): the same rule, read
  over the tree's **migrations** — the steps from one version of a file
  format to the next — and the rule is about the **name**, so a step
  named in a comment is a step the gate counts. The rule does not say that
  a document of every version survives being written and read back with
  everything that version can carry.
- **The reader that finds the kinds is a reader too** (§1134): it reads the
  kinds a battery writes **through a macro** — the macro's pattern says
  which argument the kind's id is, and the call says what it is — and it
  reads the production code only, so that a kind written in a test is not
  a kind the tree has. A kind declared in a way the reader does not see is
  a kind the gate cannot hold to account, and the gate's own tests are the
  only thing that says the reader sees what it says it sees.
- **A session built by hand is not filtered by layer.** `Session::register`
  refuses a command whose layer is not linked in, and every door builds its
  session with the ecosystem's layers — but a session made from a registry
  holds what it was given, and nothing checks the layer of a command that
  was already there when the session was built. The doors do not do this:
  they ask the ecosystem for the session.
- **The `Either` is a compromise.** One parameter with two spellings of
  different kinds cannot be a single typed value, so the command is handed
  whichever half arrived and decides what to do with it.
- **A subprocess plugin's metadata is leaked, once.** A command's labels are
  `&'static str` and a subprocess plugin's labels arrive at run time, so they
  are leaked when the plugin is loaded (`crates/craftkit-sdk/src/plugin.rs`).
  The leak is bounded by the number of commands the plugin has, and once each.
- **A subprocess plugin is not checked by the compiler.** Its catalogue is
  data, and a mistake in it is refused when the plugin is loaded. What
  the catalogue now carries is each parameter's kind, its leniency and a
  pair's second kind (§1129), so the host checks the plugin's **call**
  against the plugin's own **declaration** — and the plugin's body is
  still the plugin's business.
- **A kind's `ACCEPTS_TEXT` is a convention, not a guarantee.** The
  compiler writes a compiled command's declaration and reader from the same
  struct, but the constant that says "this kind takes a string as well"
  is the kind author's own word, and a kind that says it and whose reader
  does not take one is refused at the door (§1129).
- **JSON Schema's "integer" admits `3.0`.** The reader takes a whole
  number, and the schema's word for that is `integer`; a caller that sends
  `3.0` passes the schema and is refused by the reader (§1129).
- **A test's scratch comes back when its thread ends, and not before.** A
  test that panics takes its thread down with it, and the harness takes the
  process down with it, so the scratch of a test that panicked may be left
  behind. Before §873 the tree's tests left **everything** behind, so this is
  a smaller hole and not a closed one.
- **The outer gate's own scratch is left behind on purpose.** The `xtask`
  writes what a test wrote into `/tmp/craftkit-xtask-tests` and leaves it
  there, so that a person can look at it; the folder is emptied when the
  next run owns the name, and the name carries the process id, so a folder
  of an older run is still there. The tree's seventh rule ("a test does not
  litter") reads the crates of the tree and not the `xtask`, which depends
  on nothing of the tree's own.
- **The wire protocol is a decision, not a derivation.** Newline-delimited
  JSON was chosen because it is trivial to write in any language.
- **The bridge is a proof, not a product.** It speaks JSON-RPC over stdio
  because that is the cheapest way to show a second front end over the same
  registry.
- **The plan is a value, and the window draws it.** The window shows the
  plan's items — the node, the kind and the value — and the plan itself
  is the **command's** work (`doc.plan`) and not the door's. What nothing
  does is **check** a plan against a rule: the items are what the
  command said, and there is no rule that says what a plan must hold.
- **The third door does not exist on the web.** A subprocess plugin needs a
  process, and `wasm32-unknown-unknown` has none.
- **The document model is still small.** The *seam* is the thing being
  demonstrated: a version is a contribution, a step is a contribution, and the
  chain is checked. A document worth versioning is a different piece of work.
- **The survey of the tree's bounds (§1170) named the stops that did not
  say so, and the arcs after it have taken them.** Fifteen batteries
  keep two hundred and twenty-three bounds and the kit and the
  libraries seventy-seven more; every one of them was read with a
  written contract, and these are the ones that did not say that they
  stopped — with the arc that took each of them:
  - **pdf** — §1172 fixed the two walks: `pdf.page.attributes` stops
    the walk up `/Parent` at `MAX_ANCESTORS` (sixty-four) and
    `pdf.form.flatten` and `pdf.comment.flatten` stop the walk over the
    pages at `MAX_DEPTH` (sixty-four), and all three answer
    `truncated`.
  - **prose** — §1172 fixed the chain: `prose.style.character.read` and
    `prose.block.character` stop the chain of base styles at
    `MAX_STYLE_CHAIN` (thirty-two), and both answer `chain_truncated`.
  - **grid** — the `SUMIF` family cuts a criterion at
    `MAX_CRITERION_UNITS` (two hundred and fifty-five) before it
    matches, where the single-criterion case refuses it: the code says
    so in its own words and the tests prove it, so it is a rule and not
    an accident. The **solver** answered **"optimal"** when its branch
    and bound stopped at the bound of the tree before it had proved the
    answer was the best one — that was a lie, and §1171 fixed it: a
    solution that was not proved is "feasible" and the answer carries
    `converged`. The **goal** now says when its search stopped
    (`truncated`), though the search halves its range and so comes
    within the bound long before it is reached. Two of the survey's
    claims were **wrong**: a group's level cannot be deeper than the
    tree's own (the commands refuse it and the xlsx reader cuts it
    with a word), and a cell's spill cannot reach `MAX_SPILL` (the text
    runs over the columns of the page it is drawn on, and a page holds
    about eight columns).
  - **fx** — §1172 fixed it: `fx.time.step` holds the frame at
    `MAX_FRAME` and answers `clamped`, the same shape as the film
    battery's playhead, which §1170 fixed.
  - **text** — §1175 fixed it: a metric run longer than `MAX_RUN`
    (eight characters) is kept and never used, and the measurement of a
    paragraph now answers `runs_ignored` — the number of the metric's
    runs that are too long to be used. The run is still kept rather than
    refused, and that is the tree's rule and not an accident: the bound
    is on the work and not on the table.
  - **vector** — §1174 fixed it: `vector.path.erase` samples a contour
    at no more than `MAX_VECTOR_SAMPLES` (four thousand and ninety-six)
    however fine the eraser is asked to be, and the geometry is coarser
    than the caller asked for. Both of the battery's erasers answer
    `clamped` now, and the clamp is reported only when there is
    something beyond the bound. What is still true: the samples are
    **held** rather than the erase refused, so a caller who needs a
    finer walk than the bound allows cannot have one — and the answer
    says so.
  - **image** — §1173 fixed it: the head of a picture is read up to
    `MAX_IMAGE_HEAD_BYTES` (a mebibyte), the reader says whether the file
    is longer than the bound, `image.add` and `image.measure` answer
    `head_truncated`, and a refusal of a head that was cut names the
    bound. The **grid** battery reads a head the same way and says so
    too (`MAX_GRID_HEAD_BYTES`), and the **a2ui** plugin reads a picture
    up to `MAX_A2UI_PICTURE_BYTES` and does not read one beyond it.
  - **The deliberate departures stay deliberate, and they say so**:
    §1177 fixed the three of them — the art battery's rate, the sound
    battery's repeat count and the camera's zoom are held rather than
    refused, and each of them now says that it held its value. What
    stays is the shape: a knob's domain is the knob's, and "as fast as
    the tree allows" is a request that means something.
- **A reader that stops at a bound must say that it stopped, and the
  batteries say it.** The pdf and xlsx readers and the audio crate's
  detectors say it now (§1146, §1147, §1150, §1151); the grid battery
  says it too (§1161, §1163, §1164), the prose battery's accessibility
  check says it (§1162), the sound battery says it when it cuts a name
  (§1165), the pdf battery's outline and its hidden report say it
  (§1166, §1168), the prose battery's table of figures says it (§1167),
  and the film battery says it when it holds a playhead or cuts a
  caption file's pairs (§1170). The **kit's own head reader** says it
  now, and so do the three callers of it: the image battery, the grid
  battery and the a2ui plugin (§1173). The **vector** battery's erasers
  say it too, though theirs is not a read but a walk (§1174). The **rest**
  of the batteries were surveyed in §1170 with a written contract, and
  the stops they do not say are named in the list above; **none** of them
  is open now — the text battery's metric says how many of its runs it
  ignored (§1175). Nothing *checks* a battery for a cut that is silent:
  the next one to be found is found by a reader, not by the tree.

---

## 6. Things this tree has not been asked for

Not gaps in the design — things nobody asked for:

- **There is no toolbar.** The window draws the commands by area and the
  panels a battery contributes, and a toolbar is not part of it.
- **The plugin formats are a shim** over a door that is built.
- **The AU's own battery is CoreFoundation's**, and CoreFoundation is macOS.
- **The code mutation is CI-only**, by the machine's rule, so no claim in the
  README has been measured by mutation. The v2 gates are: the tests, clippy,
  fmt, the workspace-scanning gate that holds the list of allowed `unsafe`,
  and the tree's own gate — which is measured by being pointed at trees that
  are broken on purpose.

---

## 7. Things the README has claimed and the tree has since changed

The README is a build log, written in order, and some of its early sentences
have been overtaken by later sections. The ones that matter to a reader who
comes to the tree cold:

- **"A commit points at the section that describes it."** `CONTRIBUTING.md`
  says the message ends with the README section's number — `feat(cad): the
  table of a drawing — §1600`. Of the tree's **2 597** commits, **2 317**
  carry one in their subject, and **280** do not. Those are the tree's
  early history, before the sections were numbered (**215** of them, in
  three runs between the twentieth and the twenty-ninth of October), and
  the arc that moved the v2 tree to the repository root and read a2ui
  against it (**65** of them, between the eleventh and the twenty-fourth
  of November). Those name what they touch instead —
  `feat(v2): <what it does>`. `CONTRIBUTING.md` §4 now records them as a
  **dated exception**: the commits that are already in the tree are
  accepted, the history is not rewritten, and a commit made from now on
  names its section. The rule is still the house's; what is relaxed is
  the reading of the past, not the rule itself.

- **"There is no numeric core."** There is now `craftkit-numeric`, a typed
  value (a length in millimetres, an angle in radians), at L3, because a
  length can be a command's parameter and that means speaking the SDK's own
  types.
- **"The old tree's format is not here."** It is still not here: v2's format
  is a **contribution**, with migrations that are contributions, and a version
  chain the ecosystem checks. The old tree's one-file-per-change discipline
  belongs to a tree that is not this one.
- **"The front ends are not built."** The terminal door, the agent door
  (`apps/craftkit-mcp-server`), the bridge, the browser door and the window
  are all built, and all of them reach the same registry.
- **"The plan is a value, not a drawing."** The **window draws it** now:
  the plan's items — the node, the kind and the value — are on the
  window's own panel, and the plan is still the command's work
  (`doc.plan`) and not the door's. What nothing does is **check** a plan
  against a rule (§5).
- **"The two spellings of a parameter are two kinds."** That was true
  for the **compiled** command and not for the rest: the declaration
  carried the second spelling's kind in one builder, and the hand-written
  declaration, the a2ui derive and the subprocess plugin did not use it
  (§1125). It is true on **every** path now: a pair says what each
  spelling takes, the plugin's catalogue carries it, and the schema
  publishes the pair as required (§1129).
- **"Eleven crates keep a published cap."** That was true when §1155
  wrote it, and it stopped being true when §1159 published the
  batteries' caps: the tree keeps a published cap in **twenty-six**
  crates, and §1173 made two more of them — the image battery and the
  a2ui plugin — and gave the grid battery a second one. The claim row
  now says twenty-seven, and the count of the tree's caps is in the
  README's counted block, which the gate checks (§1180). The row was
  corrected in place; the history of the claim is not in the README.

- **"The README's sections stop at §1180."** They did, and the **plans**
  reserve the ones after it: `docs/plans/` holds one plan per project in
  the family tree, and between them they reserve **§1185 to §1420** for
  **236 arcs** that have **not been built**. The reservation is a promise
  about numbering, not a claim that the work exists — the README has no
  such sections yet, and the port manifest has no rows for them either.
  A plan that is not worked becomes a plan that is wrong, and the
  numbering it holds is the thing to give up first. **One plan was
  withdrawn** — the vector project's proof of concept — and its two
  sections went with it: §1272 and §1273 are the sound plan's first two
  arcs, and a section that no plan holds is a section nobody will fill.

When the README and the code disagree, the **code** is right, and this file
records the difference rather than the README's history.

---

## 8. What the survey left open

The a2ui survey read this tree against a2ui and found opportunities as
well as defects. The defects are repaired and the opportunities that need
no product decision are taken (`docs/survey-a2ui.md` §6). What was left
**waited for a question nobody had answered**. The owner has answered
them, and the answers are here with their reasons, so that a reader who
disagrees knows what to argue with:

- **O6 — the registry reconciles against itself: closed, and the version
  counter is not wanted.** The **sharpest case** was closed first: a
  session can gain a command while it runs, and every door reads its
  list from the session it runs the commands in, so a door never offers
  a command the session does not have (§1121). The **panel's side** is
  closed too (§1127): a panel is a battery's **contribution** — a fixed
  set of the commands that battery brought — and the commands that are
  not a panel's are the kit's own, read from the session every time the
  window draws. A command that arrives while the program runs is a
  button with the rest. The survey's remedy was a **version counter**
  and a reconciliation step, and the reason it is not wanted is that it
  would be a way of *noticing* what the window already notices — while
  adding a public concept to the SDK's contribution model. The registry
  is **grow-only**, so "disappeared" cannot happen, and a stale id is an
  **error** under the house's own rule rather than something to drop
  quietly. What would reopen this is a registry that can **shrink**: a
  plugin that unloads. The tree has no such thing.
- **O7 — a path resolves to a plain value, inside the SDK: closed as not
  wanted.** The a2ui layer resolves its own paths and the door gets
  plain values, and the gate refuses a front end that names the plan — so
  the *constraint* the survey cared about holds. A **general** API would
  have to say what a path may resolve to, and that is a decision about
  the data model: the tree's own rule is that the document carries the
  payload and the tree is containment and relations, and an API that
  resolves any path to any value is a **second way** to read the
  document — the thing the tree refuses everywhere else ("one source,
  five views"). The cost was **large** and the benefit is unproven: the
  one surface that wanted it has its own resolution, and a second
  surface that needed one would find the SDK's API *and* that layer's,
  and there would be two ways to do one thing. What would reopen this is
  a **second** surface that must resolve paths; then the SDK would have
  one way to do it rather than two.
- **O9 — evals: closed with a posture, and the engine is the scorer.** The
  tree's posture is **no outbound network** and **no credentials in the
  kit**, and that is a property of a scaffold meant to be copied: a tree
  whose every copy can reach a host is a tree that every copy must be
  trusted with, and a tree that holds a credential is a tree whose
  copies must all be given one. The survey's own best idea survives the
  decision — *the scorer must be the engine* — and the engine is already
  here: a host program asks a model for commands, runs them through
  `Session::execute`, and scores what the engine refused. The **model
  call belongs to the host**, not to the kit, and the kit's part is the
  engine. A program that wants an eval adds its own command or its own
  plugin; the tree's rule that everything user-visible is a command is
  what makes that possible without a special door. What would reopen
  this is a tree that is a **product** rather than a scaffold, and that
  is a decision about what this tree is.
- **O10 — contracts: still open, and the trigger is named.** The bounds
  are in the README and the code, and the contracts would be a second
  place they live. The extraction must **replace** prose and not sit
  beside it, and the inner gate reads the bounds out of the README, so
  the extraction is not a pure move. The honest moment to write them is
  when they have stopped moving — and they have not: four arcs of this
  session changed the public surface, and the tree's published caps went
  from a hundred and thirty to a hundred and thirty-five. The **trigger**
  is a release, and the tree has no release: there is one workflow in the
  tree and it is CI. So the first step is a release workflow, and the
  contracts follow it. **The owner has been asked and has said there are no
  releases yet**, so the trigger stands and nothing waits on it: the
  question is not open for a decision, it is open for a day.
