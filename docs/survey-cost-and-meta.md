# Where the v2 work went, and what could go faster

A survey of the v2 work: **825 porting arcs** on `exp/meta-v2` and the **one**
arc of the swap, from 2026-10-20 to 2026-11-11. Its purpose is to answer one
question with numbers rather than memory: **where did the time go, and which
of those places are structural?**

## The method, and what it can and cannot see

Three sources, all of them the tree's own:

| source | what it gave |
|---|---|
| `git log` on `exp/meta-v2` and `exp/v2-root` | 826 arcs, their gaps, their files, their lines |
| the runtime's own session database (`local_runtime_message_rows`, `local_runtime_background_tasks`) | every tool call, its timestamp, and the **measured** duration of every command |
| the tree itself | what the tree does about counts, claims, bounds and registries |

Two caveats, stated plainly.

The **command durations are measured**, not estimated: the runtime starts every
shell command as a tracked background task and records when it ended. Where a
command had no tracked duration it is excluded, which is 4 318 calls — mostly
the instant ones (`write`, `edit`, `read`).

The **split between model time and machine time is an estimate**. A tool call
costs the session the time until the *next* call is issued; that interval holds
the command's execution, the model's generation, the round trip, and any
idle. I can subtract measured execution, but the rest is one bucket. So:
**machine time is exact; model time is a remainder.**

## The shape of the work

| measure | value |
|---|---|
| arcs | 826 (825 ports + 1 swap arc) |
| days | 22 |
| arcs per day | 37.5 |
| median gap between arcs | **10.0 min** |
| p90 gap between arcs | 29.3 min |
| mean gap between arcs | 21.7 min |
| files touched per arc | median 3, mean 4.1 |
| lines per arc | median 411, mean 466 |
| line split per arc | **62 % code, 37 % test, 0.4 % prose** |

The arcs are small and uniform. That is the design working: a port is "find the
rule, write it, test it, say so". The median arc moves three files and changes
four hundred lines.

Per scope the cost is almost flat — grid 12.5 min, vector 13.9 min, cad 11.6,
pdf 11.4, film 11.4, prose 9.2, layout 9.3, sound 8.3, fx 6.4. **The battery
does not decide the cost; the shape of the arc does.** That is a useful fact:
it means a shortcut that helps one battery helps all of them.

## Three budgets

The session ran for **331.9 hours** of wall clock, over 29 467 tool calls.

```
machine time (measured)     53.2 h     23 668 shell commands
  cargo                        33.3 h   63 % of all machine time
  search, read, git, scripts   20.0 h
model + round trip           ~156 h     29 467 calls x 19.1 s median gap
idle, waiting, compaction    ~123 h     61 gaps over 15 min, 92 over 5 min
```

The middle line is the one that matters. The median gap between two tool calls
is **19.1 seconds**, and at that rate 29 467 calls cost 156 hours of pure
round trip before a single line of Rust is written.

**The cost driver is the number of round trips, not the compiling.**

## The five findings

### 1. Half of every arc is spent reading the tree to find a convention

Of the 35.4 tool calls an arc makes, **17.8 (50.3 %) are searches and reads**:

| per arc | calls | share of all calls |
|---|---|---|
| search the v2 tree | 11.10 | 31.4 % |
| search the reference tree | 3.73 | 10.6 % |
| search the README and the claim rows | 1.81 | 5.1 % |
| read a file | 0.97 | 2.7 % |

Those 15 223 search commands break down by intent:

| what the search was for | commands | share |
|---|---|---|
| read a region of a file | 4 514 | 29.7 % |
| find the function or type to extend | 3 739 | 24.6 % |
| find the README claim or count row | 1 837 | 12.1 % |
| search the reference tree | 1 818 | 11.9 % |
| find the test conventions | 1 465 | 9.6 % |

Read that again. **Two thirds of all searching is looking for something that is
already in the tree** — an example command, a place to put a new one, the
convention for a test, the row of the claim table. None of that is a design
question. All of it is a **lookup question**, and the tree can answer it.

### 2. The verification is about thirty times more expensive than it needs to be

| kind of test run | runs | hours | median |
|---|---|---|---|
| one package (`cargo test -p craftkit-plugin-grid`) | 2 908 | 3.7 h | **0.8 s** |
| the whole workspace (`cargo test --workspace`) | 959 | **26.8 h** | 27 s |

The workspace runs are where the time is: **448 runs longer than 30 s hold 94 %
of the 26.8 hours**, 108 runs longer than 5 minutes hold 59 %, and the worst
took 24 minutes. A single battery's change was being verified by the whole
workspace, over and over — 959 times across 826 arcs.

The three-part recipe (`cargo fmt --all && cargo clippy --workspace ... &&
cargo test --workspace --all-features`) was run as one line **658 times**,
9.1 hours in total, and the same command line appears **111 times** in the
history exactly as written.

The machine is shared and heavily loaded, so this is also the polite thing to
stop doing.

### 3. The ceremony is hand-maintained while the truth is already a value

The README carries a **713-row claim table** and a count per battery, and both
are kept by hand. The counts test checks the README against the tree by
**scanning the source text for the string `#[command(`**
(`apps/craftkit-studio/tests/counts.rs`).

But the tree already has the truth as a value, at link time:

```rust
// crates/craftkit-sdk/src/command.rs:128
inventory::collect!(crate::contribution::Registration<CommandSpec>);

// crates/craftkit-sdk/src/contribution.rs — the registry reads it back
Registry::from_inventory() -> Result<Self, RegistryError>
```

Every command is already a `CommandSpec` static with its id, area, label,
params and flags, registered into an inventory the linker fills. The count of
the grid battery **is** `registry.of_area("grid").len()`. The README says 188
because a human counted `#[command(` with `grep`.

This is the house rule applied to the house's own book: *derived values are
derived — never stored*. The claim table is stored. The count is stored.

The same is true of the bounds. `crates/craftkit-gates/src/numbers.rs` parses
constants out of Rust source with a regex and then greps the prose for the
number, to check that the two agree. Both halves of that check are available
to the compiler.

### 4. The port's reference work is invisible to the tree

3.73 searches per arc go to the family trees (`gridcraft`, `soundcraft`,
`pdfcraft`…). This is the genuinely irreducible part of the work — someone has
to understand what the rule *means* before it can be ported. But the *result*
of that understanding is not recorded anywhere in the tree.

There is no artefact that says: "this rule came from `gridcraft`'s
`PivotTable`, it landed in `craftkit-plugin-grid/src/pivots.rs`, and it is
tested by `tests/pivots.rs`". The port is only legible in the commit history,
in prose, one arc at a time. **The tree cannot check its own ports.**

### 5. The delegation was mostly noise

41 sub-agents were spawned. Their combined wall time was 8.0 hours, their own
shell time 0.9 hours, and **four errored and two were aborted**. Thirty-six of
them were one-off "survey battery X" explorations, each answered by re-reading
a tree the parent could read directly. The parent spent **1.66 calls per arc
polling** for results that mostly said "no gap found".

Delegation paid for itself in three cases (the clippy range rewrite, the door
fallbacks, the standard's documentation) — 6 of 41. That is a 15 % hit rate on
a mechanism that costs a round trip every time it is used.

## The shortcuts, in order of payoff

Each is measured against the arc it would replace, not against a wish. The
savings are two kinds and the column says which: **machine hours** come from
the measured durations, **calls** come from the counted calls per arc and are
the round-trip budget.

| # | the shortcut | what it removes | the estimate |
|---|---|---|---|
| **S1** | verify **only the package that changed** during the arc; run the workspace suite once at the arc's end, or in CI | 959 workspace test runs | **26.8 h → ~4 h** of machine time; the median run goes 27 s → 0.8 s |
| **S2** | one command for the whole recipe: `cargo xtask verify --package <pkg>` (fmt-check, clippy, test) | 658 hand-written three-line recipes | **~1 300 calls** (2 per recipe) ≈ 7 h of round trip |
| **S3** | generate the claim row, the per-battery count and the README block **from the registry** | the 1.81 searches per arc for claim and count rows, and the `grep`-based counts test | **~1 500 calls**; the counts stop being a claim that can be stale |
| **S4** | a **port manifest**: one row per ported rule — name, reference path, landing site, test — checked by the gate | the 3.73 reference searches per arc become one lookup; the tree can check its own ports | **~3 000 calls**; the ports become auditable |
| **S5** | a **scaffold command**: `cargo xtask new-command --battery grid --id grid.foo` writes the command struct, the test skeleton, the claim row and the README section from the tree's own conventions | part of the 11.10 searches per arc for "where does this go and how is it written here" | **~5 000 calls** |
| **S6** | **fewer, bigger queries**: one `xtask find` that answers "where does command X live", "what owns area Y", "how is a command written in battery Z" | the rest of the 11.10 per-arc searches | **~4 000 calls** |
| **S7** | **stop the one-off surveys**: one written contract per battery, not 36 ad-hoc ones | 1.66 polls per arc, 35 wasted spawns | **~1 400 calls** |

S5 and S6 divide the same 11.1 searches between them, so their call counts
overlap with S3 and S4; they are **not** additive. Taken together, S1 through
S7 touch roughly 9 000 of the 29 467 calls — a third of the round-trip budget
— and two thirds of the machine time in the workspace test runs.

S1 and S2 are mechanical and pay for themselves in a day. S3 through S6 are the
meta-programming work, and they are where the tree's own thesis points.

## Where the meta-programming headroom is

The tree already made the key move once: **a command is a struct whose fields
are its parameters**, and `#[derive(Command)]` writes the declaration, the
reader, the registration and the anchor. That is the pattern to extend, not
replace.

### M1 — the derive should write the *facts*, not only the code

Every `CommandSpec` already carries the id, the area, the label, the icon, the
parameter documentation and the flags. The claim table is a **projection** of
exactly that. Today it is written by hand into the README, one row at a time,
663 rows, and every row can drift.

If the claim table were generated from the registry — `xtask claims`, printing
the table, and the README carrying it as a generated block that the gate
checks against the registry — then:

- the table can never be stale, because it is the registry;
- the per-battery counts become `registry.of_area(area).len()`;
- the counts test stops counting the string `#[command(` and starts asking
  the tree what it has;
- and the README stops carrying a number that a human has to keep right.

This is the single highest-leverage change, and it is **small**: the derive
already emits everything needed.

### M2 — the gate should read data, not text

`numbers_agree_with_the_prose` and `bounds_named_in_the_prose` both work by
scanning source and prose with regexes and comparing. Both facts are available
to the compiler:

- a bound named in the prose can be a `const` whose value is asserted against a
  parsed literal, or a doc-test;
- the layering, the deny list and the "a plugin is a plugin" rule can be
  checked by a **build script** that reads the manifest graph, rather than by
  parsing `Cargo.toml` as text.

The text scanners are not wrong; they are *late* and *fragile*. A compiler
check fails at the point of the mistake, with the line number.

### M3 — a battery as a *declaration*, not as Rust

This is the big one, and it is the one that would "completely change today's
project" — so it deserves to be stated with its cost attached.

The nine batteries are structurally identical: a plugin crate, a module per
feature, one `#[derive(Command)]` struct per rule, one test file per feature, an
anchor. The **semantics** of a ported rule live in the reference tree, and the
v2 code is mostly that semantics written out again. If the rule were *declared*
— its inputs, its outputs, its refusals — and the compiler wrote the struct,
the reader, the registration, the test skeleton and the claim row, then a port
would be a **row in a table** rather than 411 lines of Rust.

The tree's own `craftkit-derive` crate already has `expand_node_kind`,
`expand_tool`, `expand_panel`, `expand_evaluator` and `expand_layer` — five
kinds of contribution, five macros. A sixth kind, "a rule", is a natural
extension of what is there.

**The honest cost:** the v2 tree's batteries are deliberately hand-written Rust
whose rules can be *read*. A declarative table would trade that readability
for the speed of the port. The house values a rule you can read over a rule you
can only run. So M3 is not obviously right — it is a trade, and it should be
made deliberately, one battery at a time, not wholesale.

### M4 — the test scaffolding, written by the macro

37 % of every arc's lines are tests, and much of that is repeated shape:
build a document, execute a command, assert an outcome, name the failure. The
tests are also where the ceremony lives — the anchor line, the session setup,
the parameter JSON.

A `assert_command!`-style macro could write the shape, leaving the arc to
write only the assertion that is specific to the rule. That is a smaller
promise than M3 and keeps the readable body of the test.

## What this survey does not settle

1. **The model-time share is an estimate.** The machine times are measured; the
   round-trip figure is a median applied to a call count. If the true latency
   distribution is heavy-tailed in a way the median hides, the proportion
   shifts — though the direction does not: fewer calls is cheaper either way.

2. **The reference-tree searches are not all removable.** The 3.73 per arc are
   the *semantic* work of a port. A manifest (S4) removes the *lookup* of
   where the rule was, not the reading of what it means. If the rule is subtle,
   it will be read again whatever the tooling says.

3. **The 30× test ratio is a median, not a law.** A workspace test after a
   change to `craftkit-sdk` is genuinely a workspace test. The shortcut is to
   scale the scope to the blast radius of the change, not to forbid the
   workspace run.

4. **M3 is a trade, not a win.** It is listed because it is the largest lever,
   not because the measurements say it should be taken. The measurements say
   the *ceremony* is the waste; they do not say that hand-written rule bodies
   are.

5. **The sub-agent hit rate is one session's.** 41 spawns is a small sample,
   and the three that paid off were the ones given a bounded, mechanical
   brief — the clippy rewrite, the fallbacks, the documentation. The lesson
   may be "delegate the mechanical, not the survey" rather than "delegate
   less".

## One paragraph, for the record

The v2 work was fast because it was uniform: 826 arcs, ten minutes each, four
hundred lines each, and the same shape every time. It was slow in exactly one
way, and the way is visible in the numbers: **50 % of every arc went into
finding out what the tree already knew.** The compiling was never the
problem — 53 hours of machine time across three weeks, and a quarter of that
was the whole workspace being rebuilt to check one battery. The tree already
holds its own facts as values, in a link-time inventory, written by a derive
macro that already emits the declaration, the reader and the registration.
The next arc is not another battery feature. **The next arc is teaching the
derive to write the claims, and the gate to read the registry instead of the
prose.**
