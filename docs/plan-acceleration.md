# The acceleration plan

How to take the bottlenecks the survey found and remove them, in two levers:

1. **Meta-programming** — make the compiler write the facts, so that nobody
   has to keep them by hand.
2. **Context engineering** — make the key information arrive **in one call**,
   so that nobody has to go looking for it.

The survey is the evidence; this is the work. Its baselines are the numbers
that were measured on 2026-11-12, and every wave ends with the same numbers
measured again.

| baseline | value |
|---|---|
| arcs | 826, median 10.0 min each |
| tool calls per arc | 35.4 (29 467 in total) |
| reading and searching per arc | 17.8 calls (50.3 % of all calls) |
| machine time | 53.2 h, of which 26.8 h is 959 workspace test runs |
| round trip | 29 467 calls × 19.1 s median ≈ 156 h |

**The target**: an arc that costs **6–8 calls** instead of 35.4, and **2 h of
machine time** instead of 53.2. That is the whole aim; everything below is a
way of getting there.

---

## Part 0 — the two levers, and why they are the same thing

The survey's finding 1 says half of every arc went into *finding out what the
tree already knew*. The finding said the tree knew these things in three
places, and each place has a lever:

| what the tree knows | where it knows it now | the lever |
|---|---|---|
| what a command is, and what it takes | `CommandSpec`, written by the derive | **meta** — the derive already writes it |
| which commands exist, in which area | the link-time inventory | **meta** — `Registry::from_inventory()` |
| the counts, and the claim rows | the README, by hand | **meta** — the counts are derivable; the prose is not |
| the conventions of a battery | the battery's source, by reading | **context** — one call, not eleven |
| where a rule was ported from | nowhere | **context** — a manifest |
| the reference rule's meaning | the reference tree | **irreducible** — this is the port |

The last row is the one that is not on the list of things to fix. Reading what
a rule *means* in `gridcraft` is the work of a port. Everything else is
bookkeeping, and bookkeeping is what a compiler is for.

---

## Part 1 — the bottlenecks, ranked

| # | bottleneck | measured | wave |
|---|---|---|---|
| B1 | the workspace test is run to check one battery | 26.8 h, 959 runs | 1 |
| B2 | the verify recipe is hand-written three times per arc | 658 recipes, ~1 300 calls | 1 |
| B3 | the counts and the claim rows are kept by hand | 1.81 searches per arc; `counts.rs` greps the source | 2 |
| B4 | the gate reads the tree as text | `numbers.rs`, `bounds_named_in_the_prose` | 2, 4 |
| B5 | the conventions of a battery are found by reading | 11.1 searches per arc | 3 |
| B6 | the ports are not recorded anywhere | 3.73 reference searches per arc, unauditable | 3 |
| B7 | the rules are re-found every time, in every battery | 9.6 % of searches | 3 |
| B8 | the delegation is unpriced | 41 spawns, 6 paid off | 1 (policy), 3 |

---

## Part 2 — Wave 1: the machine (arcs §845–§847)

**Goal: stop paying workspace-scale prices for battery-scale changes.**

### §845 — `xtask verify` — one command, scoped to the change

The recipe `cargo fmt --all && cargo clippy --workspace ... && cargo test
--workspace --all-features` appears 658 times in the history, 111 times
verbatim. Replace it with one command that knows the blast radius:

```
cargo xtask verify                       # the whole tree: the gate's job
cargo xtask verify --package craftkit-plugin-grid
cargo xtask verify --changed             # the packages the diff touches
```

`--changed` is the one that matters: it reads the diff against `HEAD` and
verifies only the packages it touches, plus the packages that depend on them
(reverse dependencies from the manifest graph). A change to
`craftkit-plugin-grid` verifies grid and its dependants, not all 51 crates.

*Acceptance*: `xtask verify --changed` runs the same phases the gate runs
(fmt, clippy, test) restricted to the affected packages; the median runtime
for a one-battery change is **under 30 seconds**, against 27 s median for the
workspace *plus* the clippy and fmt phases it replaced. A test proves the
scope: a change to a battery selects that battery and its dependants, and a
change to `craftkit-sdk` selects everything.

### §846 — the gate runs the scoped verification, in the order that fails first

The gate currently runs `fmt`, then `clippy`, then `test`, on the whole tree.
Three phases, one blast radius. Change it to: **fmt-check** (cheap, catches
most), then **clippy** (medium), then **test** — each scoped to the changed
packages in the local gate, and the whole tree in CI, where the machine is
free.

The house rule that the gate is honest is preserved: a scoped run that cannot
verify something says so. The gate's output must name what it verified and
what it did not.

*Acceptance*: `cargo xtask ci` in a dirty tree verifies the changed packages
and says which ones it skipped and why; the CI workflow runs the whole tree
once, unchanged. A test asserts the skip list is reported.

### §847 — the delegation is priced

Write down the rule that was only ever implicit: **a sub-agent gets a bounded,
mechanical brief or it does not get spawned.** The three that paid off (the
clippy range rewrite, the door fallbacks, the standard's documentation) were
exactly that. The 36 one-off "survey battery X" explorations were not, and
cost 1.66 polls per arc.

*Acceptance*: a rule in `docs/agent-playbook.md` and in `AGENTS.md`: one
survey per battery with a written contract; no spawning for a question that
one `xtask find` answers. A test in the xtask that checks the rule is stated
in the prose (the house's "the bounds are named in the prose" pattern,
applied to a rule of conduct).

**Wave 1 payoff**: ~26.8 h → ~4 h of machine time; ~1 300 calls removed; the
shared machine stops being hammered.

---

## Part 3 — Wave 2: the facts (arcs §848–§850)

**Goal: the tree stops keeping numbers by hand.**

### §848 — the counts come from the registry

The README says the grid battery carries 188 commands, and
`apps/craftkit-studio/tests/counts.rs` checks that by counting the string
`#[command(` in the source. But every command is already a `CommandSpec` in a
link-time inventory, and `Registry::from_inventory()` reads it back.

Change the test to ask the tree:

```rust
let registry = Registry::from_inventory()?;   // the truth
let count = registry.of_area("grid").len();   // one small addition: of_area
```

(`Registry` has `of_capability` today and `all()`; `of_area` is one more
filter, and it is the filter the counts need.) Change the README's numbers to
be **generated** into a marked block by `cargo xtask claims`, which the same
test then checks. A hand-edited number inside a generated block fails; a
stale number cannot survive.

*Acceptance*: the counts test reads the registry and not the source text; the
README's nine battery counts live in a generated block; editing a count by
hand fails the test. The MCP tool count (1 045 today) comes from the same
place, so the README and the server cannot disagree.

### §849 — the claim table gains a checked index

The claim table (§48, 713 rows) is **prose** — `| the claim | the truth today
| where |` — and prose stays prose; a compiler cannot write "the truth today".
But two of its three columns can be made true by machine:

- the **where** column is a § number, and every § number in the table must
  exist as a section in the README;
- every port arc's § must have a row, so a port cannot land without saying
  what it claims.

That is a gate, not a generator: the prose is written, and the table's
*index* is checked. The README's own claim — "a tree that keeps saying what it
has not built is a tree whose claims can be believed" — gets its teeth.

*Acceptance*: the gate fails when a § in the table has no section, and when a
port's § has no row.

### §850 — the bounds are checked where the compiler is

`crates/craftkit-gates/src/numbers.rs` parses constants out of Rust source
with a regex and then greps the prose for the number, to check that the two
agree. Two moves:

- the **numeric** half becomes a compile-time assertion — a `const` that must
  equal the value the prose names, checked by the compiler with a line
  number;
- the **prose** half stays a text check, because only text can say "ten
  thousand" — but it stops re-parsing the code, because the code now hands it
  the value.

*Acceptance*: the gate reads the constants from the compiled values rather
than from a regex over the source; a mismatch names the constant, the file
and the line.

**Wave 2 payoff**: ~1 500 calls removed; the README's numbers can no longer
drift; the gate stops being a text scanner in the places where a compiler
can do better.

---

## Part 4 — Wave 3: the context (arcs §851–§855)

**Goal: the key information arrives in one call.**

This is the wave that attacks the 17.8 reading and searching calls per arc
directly. The principle: **every question an arc asks about the tree should
have exactly one answer, and that answer should be one call.**

### §851 — `xtask find` — the registry answers the questions

One command, many questions:

```
cargo xtask find id grid.pivot.set      # where does this command live?
cargo xtask find area grid              # what is in this area?
cargo xtask find capability command     # what has this capability?
cargo xtask find module grid            # which files make up this battery?
cargo xtask find claim §1600            # what does the README say about this?
```

The first four are answered from the registry and the manifest graph — facts,
not text. The fifth reads the README's own index. The answer names the file,
the line, and the *shape* of what is there.

*Acceptance*: every question above is answered by one call; the answers are
derived from the tree's own structures, not from a text search; a test proves
the answers agree with the registry.

### §852 — the battery carries its own example

The single biggest search cost is "how is a command written in *this*
battery?". The answer exists in the battery's source — but finding it costs
three greps and a read.

Put the answer **in the code**, where it is reviewable, and mark it:

```rust
#[derive(Command)]
#[command(id = "grid.pivot.set", label = "Set a pivot", example)]
pub struct SetPivot { … }
```

The derive records the example; `cargo xtask example --battery grid` prints
the canonical command, the canonical test, the module list and the naming
rules of that battery, in one call — and because the example is *in the
source*, it cannot go stale. It is read by a human too.

*Acceptance*: every battery has exactly one marked example; `xtask example`
prints it; a test fails when a battery has none.

### §854 — the arc brief: everything one arc needs, in one call

The brief is the whole of Wave 3 in one artefact. Before writing a command,
the arc asks for its brief:

```
cargo xtask brief --battery grid --id grid.pivot.set
```

and gets, in one output:

- the **rule**: its name, its meaning, and where it was ported from (the
  manifest);
- the **landing site**: the module and the file, and the neighbouring commands
  already there;
- the **conventions**: the battery's example command and example test (§852),
  the naming rules, the refusal style, the doc-comment style;
- the **claims**: the claim row to add, the § number to use, the current
  counts of that battery and of the tree;
- the **commands**: the exact scoped verify to run, and the exact gate to
  run at the end.

That is the eleven greps, replaced by one call whose output is a page.

*Acceptance*: the brief contains every item above; a test asserts that each
section is present and non-empty, so the brief cannot quietly lose a part.

### §853 — the port manifest: the tree remembers where each rule came from

Today a port leaves no trace. The manifest is that trace, and it is data:

```toml
[[ports]]
rule = "the pivots of a sheet"
from  = "gridcraft/crates/sheets/src/pivot.rs"
to    = "craftkit-plugin-grid"
module = "src/pivots.rs"
command = "grid.pivot.set"
test  = "tests/pivots.rs"
section = "§1600"
```

It is written once per port, and it earns its keep three times over:

- the **brief** can name the reference path, so the reference search is one
  lookup instead of three greps;
- the **gate** can check that every ported rule still has a command and a
  test, so a port cannot rot;
- the **survey of what is left** becomes a query: which rules of the family
  trees have no row here.

*Acceptance*: the gate fails when a manifest row names a command that is not
in the registry, or a test file that does not exist; `xtask ports` prints the
rules of a family tree that have not been ported.

### §855 — the conventions of the house, in one place

Two digests, both generated and both small:

- `cargo xtask digest --battery grid` — the battery's conventions (§852
  plus the module list and the test layout);
- `cargo xtask digest --house` — the rules that an arc keeps re-learning:
  the deny list, the refusal style, the naming rules, the four headings of a
  README section, the claim row's shape, the commit-message rules (the
  hook's ban on backticks and metacharacters), the two-commit rule.

The house digest is the answer to the 9.6 % of searches that looked for test
conventions, and to the repeated loss of time to the commit-msg hook's
format.

*Acceptance*: both digests print in one call; the house digest's content is
checked against the tree — every rule it states is found in `AGENTS.md`,
`CONTRIBUTING.md` or the recipes, and a rule that has gone is a failure.

**Wave 3 payoff**: ~9 000 calls removed (the reading and searching, 50.3 % of
the budget); the reference search falls from 3.73 calls to a lookup.

---

## Part 5 — Wave 4: the deeper meta (arcs §856+, optional, priced)

These are the bigger levers. Each is a **trade**, and each is worth doing
only if the two before it have landed and been measured.

### §856 — the test shape, written by a macro

37 % of every arc's lines are tests, and much of that is the same shape: build
a document, execute a command, assert the outcome, name the failure. A macro
writes the shape; the arc writes the assertion that is specific to the rule.

```rust
assert_command!(grid.pivot.set, |s| s.execute("grid.pivot.set", params), |o| …);
```

### §857 — the gate as a build script

The layering, the plugin rule and the deny list are facts about the manifest
graph. A **build script** can read the graph and fail at build time, with the
crate's name, instead of a text scan at gate time.

### §858 — a battery as a declaration

The largest lever, and the one that would most change the tree: the nine
batteries are structurally identical, and the semantics of a rule are
declarable — inputs, outputs, refusals. If the rule were **declared** and the
compiler wrote the struct, the reader, the registration, the test skeleton
and the claim row, a port would be a row in a table rather than 411 lines of
Rust.

**The cost, honestly**: the house values a rule you can *read*. A declaration
table trades that for the speed of the port. `craftkit-derive` already has
six kinds of contribution macro (`expand_node_kind`, `expand_tool`,
`expand_panel`, `expand_evaluator`, `expand_layer`, and the command itself),
so the machinery is close — but this wave should be done **one battery at a
time, and measured**, not taken wholesale.

---

## Part 6 — how we will know

The survey measured the cost of making the tree. The tree can measure **its
own** cost, and should — the census is itself an xtask command:

```
cargo xtask census
```

It reports, **from the git history**: the arcs, the files and the lines each
one touched, the gap between them, the commands per battery, and the
commands per file. That half is a property of the tree, and it is the half
that generalises: it says how big an arc is, whatever model wrote it.

The other half — the calls, the round trips, the machine time — is a property
of **the session that made the tree**, and it is measured where that session
is recorded: the runtime's own database. That is not the tree's business, so
it is not in the tree; the procedure lives in `docs/agent-playbook.md`, and
the survey's numbers are the baseline it is read against.

The survey's numbers become a **baseline in the repository**, and each wave's
exit is a number:

| wave | the number that must move | from | to |
|---|---|---|---|
| 1 | workspace test runs | 959 (26.8 h) | < 100 (≈ 4 h) |
| 1 | calls per arc for verify | ~9 | 1 |
| 2 | claim and count searches per arc | 1.81 | 0 |
| 3 | reading and searching per arc | 17.8 | < 4 |
| 3 | calls per arc | 35.4 | < 12 |
| all | machine time per arc | 3.9 min | < 1 min |

The census is the honest half of the plan: a shortcut that does not move the
number is a shortcut that did not happen.

---

## Part 7 — the risks, and what this plan does not settle

1. **The brief can become a second source of truth.** The brief is generated
   from the tree; the moment it is edited by hand it is a second truth. Rule:
   the brief is generated, always, and the gate checks the generation.

2. **The manifest is work.** 826 arcs' worth of ports would be 826 rows,
   written by hand, if done retroactively. It must be written **forward**
   only — each new arc adds its row — and the gate checks the rows that
   exist, not the rows that do not. Retrofitting 826 rows is not an arc; it
   is a project, and it is not worth it.

3. **The registry is not the whole truth.** The registry knows the commands;
   it does not know which reference rule each one came from, nor whether the
   semantics were faithful. That is what the manifest is for, and the
   manifest is a human's promise, not a machine's fact.

4. **The reference reading is irreducible.** 3.73 searches per arc go to the
   family trees. The manifest turns "where is it?" into a lookup; it does not
   turn "what does it mean?" into a lookup. If the rule is subtle, it will be
   read again — and that is the port, not overhead.

5. **The workspace test is sometimes right.** A change to `craftkit-sdk` wants
   a workspace test. The plan scales the scope to the blast radius; it does
   not forbid the whole tree.

6. **Wave 4 is a trade, not a win**, and §858 in particular trades the
   readability of hand-written rule bodies for the speed of the port. The
   house may reasonably refuse it. That is a decision for the owner of the
   tree, not for the census.

7. **The census measures a model, not a tree.** The round-trip figures
   describe how *this* session worked. They are the right baseline for this
   session's successors and the wrong baseline for a different model. The
   machine-time figures are about the tree and do not have that weakness.

---

## Part 8 — the order, and the first three days

| order | arc | what it buys | rough cost |
|---|---|---|---|
| 1 | §845 `xtask verify` | the machine stops paying workspace prices | 1 day |
| 2 | §851 `xtask find` | one call answers the tree's questions | half a day |
| 3 | §852 the battery's example | the biggest search cost, gone | half a day |
| 4 | §854 the arc brief | the whole arc's context, in one page | 1 day |
| 5 | §848 the counts from the registry | the README cannot drift | 1 day |
| 6 | §846 the gate, scoped | the local gate is cheap | half a day |
| 7 | §853 the port manifest | the ports become auditable | 1 day |
| 8 | §849 the claim index | a port cannot land unclaimed | half a day |
| 9 | §850 the bounds, compiled | the gate stops re-parsing code | 1 day |
| 10 | §855 the house digest | the rules stop being re-learned | half a day |
| 11 | §847 the delegation is priced | fewer, better sub-agents | half a day |
| 12 | §856 the test macro | 37 % of the lines, shorter | 1 day |
| 13 | §857 the gate as a build script | the layering fails at build time | 1 day |
| 14 | §858 the battery as a declaration | the largest lever, and a trade | weeks |

The first three arcs (§845, §851, §852) are deliberately the ones that
need **no new concepts**: a scoped verify, a lookup command, and a marker on
an example that already exists. They take the two largest costs out of the
budget — the machine time and the searching — before anything is designed
that has to be understood.

The wave 3 arcs (§852, §854) are the ones that change the shape of an
arc: instead of *reading the tree to find out how it works*, the arc is
*told*. That is the whole of the plan in one sentence, and it is why the plan
is a context-engineering plan before it is a meta-programming one — the
compiler is only asked to do the part that is genuinely mechanical.
