# The second survey: the same measurements, nine months of tree later

This is the **second run** of the survey in `docs/survey-cost-and-meta.md`. That
document is the **baseline**: it is the numbers the plan was written against,
the gate checks it still holds them (`xtask/src/lists.rs` looks for `35.4`,
`26.8` and `959` in it), and this document does **not** touch it.

The question is the same one, asked again: **where did the time go, and which
of those places are structural?** The method is the same, so the two runs can
be compared. The baseline covered 2026-10-20 → 2026-11-11; this one covers
2026-11-12 → 2026-11-20 (nine days) and, for the comparison, the whole
history to date.

## The method, and what it can and cannot see

Three sources, all of them the tree's own:

| source | what it gave |
|---|---|
| `git log` on `main` | every arc: its time, its files, its lines, its split |
| the runtime's own session database (`local_runtime_message_rows`, `local_runtime_background_tasks`) | every tool call of every session whose workspace is this tree, and the **measured** duration of every command |
| the tree itself | `cargo xtask census`, `cargo xtask counts`, `cargo xtask ports` |

The caveats are the baseline's, unchanged:

- **Command durations are measured**, not estimated — the runtime starts every
  shell command as a tracked background task and records when it ended. A
  command with no tracked duration is excluded (90 % of the baseline window's
  commands have one; 99 % of this window's).
- **The split between model time and machine time is a remainder.** A tool
  call costs the session the time until the *next* call is issued; measured
  execution is subtracted from that interval and the rest is one bucket:
  generation, round trip and idle. **Machine time is exact; model time is a
  remainder.**
- **An arc is a commit that names a section** (`feat(...) — §NNN`). The tree's
  own census counts 2 160 such commits; the *feature* commits — the ones that
  carry the code — number 1 067. Both are given below, because the gap
  between them is the documentation ceremony, and the ceremony is part of the
  cost.

### One number did not reproduce exactly

The baseline's mean gap between arcs was 21.7 min; re-measured over the same
window it is 37.6 min. The median (10.0 → 10.2) and the p90 (29.3 → 33.4)
reproduce; the mean does not, because a mean over gaps that include overnight
pauses is a statement about how often the machine was left alone. This survey
reports the median and the p90 as the load-bearing numbers, and says so.

## The shape of the work

| measure | baseline (10-20 → 11-11) | this run (11-12 → 11-20) |
|---|---|---|
| arcs (feature commits) | 829 | 238 |
| days | 22 | 9 |
| arcs per day | 37.7 | **26.4** |
| median gap between arcs | 10.2 min | **25.4 min** |
| p90 gap between arcs | 33.4 min | 96.4 min |
| files touched per arc | median 3, mean 4.2 | median 5, mean 9.5 |
| lines added per arc | median 399, mean 454 | median 232, mean 597 |
| line split per arc | 61.7 % code, 37.7 % test, 0.6 % prose | 80.3 % code, 19.6 % test, 0.1 % prose |

The arcs got **smaller and slower**. The median arc now moves five files and
adds 232 lines, against three files and 399 lines before; and it takes twice
as long between one arc and the next.

That combination is a shape, not a mystery: more files, fewer lines, more time.
The work that remains in this tree is not a sequence of self-contained rules —
it is a set of rules that each touch a reader in one crate, an import in
another, a test in a third, a registry snapshot in a fourth and the claim
table in the README. The median arc touches five files because the arc now
has five places to land. The port manifest (§853) says which, but it says it
*after* the fact.

## The three budgets

The nine days of this window:

```
machine time (measured)     49.4 h     8 530 shell commands
  cargo                        33.0 h   67 % of all machine time
  xtask ci (the gate)          18.0 h   210 runs, median 266 s
  search, read, git, scripts   16.4 h
model + round trip           ~124 h     11 494 calls x 39.0 s median gap
idle, waiting                  ~9 h      26 gaps over 15 min, 44 over 5 min
```

Compare the same nine-day shape against the baseline window's fourteen:

| | baseline (14 d) | this run (9 d) |
|---|---|---|
| tool calls | 33 498 | 11 494 |
| calls per arc | 40.4 | **48.3** |
| machine time | 54.5 h | 49.4 h |
| machine time per arc | 3.9 min | **12.5 min** |
| median gap between calls | 17.1 s | **39.0 s** |
| model + round trip | ~159.5 h | ~124.4 h |
| shell commands with a measured duration | 90 % | 99 % |

**The cost driver moved.** The baseline's finding was that the cost was the
number of round trips, not the compiling. That still holds — but the round
trip got twice as long (17.1 s → 39.0 s per call) while the call count per
arc got *worse*, and the machine time per arc got three times worse. The
baseline's own wave exit was "machine time per arc 3.9 min → < 1 min". This
run is at 12.5 min.

## The five findings, re-measured

### 1. Reading the tree is still half the arc

Search and read commands, per arc:

| per arc | baseline | this run |
|---|---|---|
| `rg`/`grep` commands | 19.10 | 19.34 |
| of which in the reference tree | 5.07 | **3.81** |
| README and claim-row searches | 0.62 | 0.54 |
| `read` tool calls | 1.28 | 1.80 |

The reference-tree searches fell (the manifest and the LSP answer "where is
this" without a search), but the total searching did not: the arcs now search
the tree they are in **more** often, not less. Wave 3's exit was "reading and
searching per arc 17.8 → < 4". It did not happen.

### 2. The verification moved; it did not get cheaper

| | baseline | this run |
|---|---|---|
| single-package test runs | 3 183 (9.7 h, median 0.9 s) | 1 189 (8.9 h, median 3.5 s) |
| **whole-workspace test runs** | **1 089 (29.7 h, median 24 s)** | **11 (2.2 h)** |
| runs longer than 30 s | 554, holding 89 % of test hours | 147, holding 87 % |
| three-part recipe runs | 709 (9.2 h) | 113 (2.1 h) |
| verification per arc | 4.3 min | **10.0 min** |

Wave 1's exit — "workspace test runs 959 → < 100" — is **met, and by a wide
margin**: the whole workspace is tested eleven times in nine days, against
1 089 times in fourteen days before. The workspace run is gone.

But the cost did not go away; it **moved into `cargo xtask ci`**. The gate ran
210 times in this window — 0.93 times per arc, at a median of 266 s — and
took 18.0 h. `cargo xtask ci` checks the packages a change can break and
names the ones it left out, which is the right idea; run **once per arc**, it
is the same mistake wearing a better hat: one whole-tree rebuild to check one
battery. The house rule says "batch the work, then gate it once", and the
practice was one gate per arc.

**The honest reading:** the wave exit was written as "workspace test runs",
and it was answered literally. The number it meant to move — the machine time
an arc costs — went the other way.

### 3. The ceremony is still hand-maintained, and the truth is still a value

| | baseline | this run |
|---|---|---|
| claim rows in the README | 713 | **928** |
| commands in the tree | — | **1 024** (grid 251) |
| the counts test | grepped `#[command(` | reads the registry |
| a port manifest | **did not exist** | **224 rows, checked by the gate** |

This is the one finding the tree has **answered properly**. The manifest
exists (`xtask/src/ports.rs`), the gate checks that every row's landing site,
command and test are in the tree and that every row has a claim row in the
README, and the counts come from the registry rather than from a `grep`.

What has not changed is the *cost* of the ceremony: the claim rows are still
written by hand, 928 of them, and finding them still costs searches (0.54 per
arc, against wave 2's exit of 0).

### 4. The port is now legible — and the manifest is forward-only

The baseline's finding was that the tree could not check its own ports. It can
now: 224 rows, each naming the rule, the reference path, the crate, the
module, the command, the test and the section, and `cargo xtask ports --check`
reads the whole thing against the tree.

The manifest is **forward-only** by decision: it records the rules that were
ported after it existed. What of `gridcraft` is not here yet is still a
question a person answers by reading the reference tree.

### 5. The delegation stopped

| | baseline | this run |
|---|---|---|
| sub-agent spawns | 46 | **4** |
| of which failed or were cancelled | 6 | 0 |
| polls for results | 1 512 | 1 060 |

Wave 1's exit — "the sub-agents that paid for themselves" — is met in the
crudest possible way: the mechanism was almost entirely stopped. Four spawns
in nine days, all of them bounded and mechanical (a survey, a survey, a
change, a fix). The baseline's 41 spawns paid off 6 times; this run's 4
spawns were 4 for 4.

## The wave exits, scored

The exits are in `docs/agent-playbook.md` §10.

| wave | the number | from | to | measured now | |
|---|---|---|---|---|---|
| 1 | workspace test runs | 959 | < 100 | **11** | met |
| 1 | verify commands per arc | ~9 | 1 | 8.2 | **not met** |
| 2 | claim and count searches per arc | 1.81 | 0 | 0.54 | partly met |
| 3 | reading and searching per arc | 17.8 | < 4 | 19.3 | **not met** |
| 3 | calls per arc | 35.4 | < 12 | 48.3 | **not met** |
| all | machine time per arc | 3.9 min | < 1 min | 12.5 min | **not met** |

One of six. The one that was met was met by *removing* a run rather than by
making one cheaper, and the run that was removed was replaced by another run
of the same kind.

## What the tree says about itself

`cargo xtask census`, measured by the tree:

| | value |
|---|---|
| commits | 2 175, of which 2 160 name a section |
| the median arc | 110 lines over 2 files |
| the gap between two arcs | 1 minute |
| the whole history | 913 521 lines |
| the commands | 1 024 (grid 251, sound 144, pdf 119, film 113, prose 110, layout 73, vector 67, cad 55, fx 47, art 11, text 10, clap 8, vst3 6, numeric 5, hello 2, image 2, shapes 1) |

The census counts every commit that names a section; the feature commits are
1 067 of them, and the rest are the documentation half of each arc. The
census's median arc of 110 lines over 2 files is the *documentation* arc; the
feature arc is 232 lines over 5 files.

## The shortcuts, re-scored

The baseline's S1–S7, with what the tree now holds:

| # | the shortcut | the tree's answer |
|---|---|---|
| **S1** | verify only the package that changed | **built** (`cargo xtask verify --package/--changed`) and **used** — but the practice is `xtask ci` per arc, which is the whole tree |
| **S2** | one command for the whole recipe | **built** (`cargo xtask verify`) — 113 hand-written three-part recipes left, down from 709 |
| **S3** | generate the claim row and the count from the registry | **half built** — the counts come from the registry; the 928 claim rows are still prose, by decision |
| **S4** | a port manifest, checked by the gate | **built** — 224 rows, and the gate reads it |
| **S5** | a scaffold command that writes the arc | **built** (`cargo xtask brief`) |
| **S6** | fewer, bigger queries (`xtask find`, `xtask example`, `xtask digest`) | **built** — and the reference-tree searches per arc fell from 5.07 to 3.81 |
| **S7** | stop the one-off surveys | **adopted** — 4 spawns in nine days, against 46 in fourteen |

Every shortcut on the list now exists in the tree. **The number did not follow
the shortcut into the tree**, with one exception: the reference-tree lookup
fell, and the searches that took its place did not.

That is the finding of this survey, and it is uncomfortable enough to state
plainly: **the wave exits were written against the wrong number.** They named
*kinds of call* ("workspace test runs", "claim and count searches"), and the
tree was reorganised to have fewer of those kinds — while the arc got more
expensive on every measure that counts.

## What this survey does not settle

1. **The model-time share is still a remainder.** The machine times are
   measured; the round trip is a median applied to a call count. The
   direction is safe — fewer calls is cheaper either way — but the
   proportions are estimates.

2. **The gate is not the workspace run.** `cargo xtask ci` checks the packages
   a change can break and says which it left out. It is a better command than
   `cargo test --workspace`, and it is still a whole-tree build when it runs.
   Whether the 18.0 h is "the same waste in new clothes" or "the same work
   done correctly" is a question about the blast radius of each arc, and this
   survey cannot answer it per arc.

3. **One window is one window.** Nine days, 238 arcs, one machine, one
   battery family (grid). The baseline's per-scope spread (6.4–13.9 min) says
   the battery does not decide the cost; this window does not have enough
   scope to repeat that test.

4. **The sub-agent sample is four.** The baseline's lesson was "delegate the
   mechanical, not the survey". Four spawns cannot confirm or refute it; they
   are consistent with it.

5. **The claim table is prose, and prose is the owner's.** The tree now
   checks that every claim cites a section that exists and that every port is
   claimed. It cannot check that the claim is *true*. That half is still a
   person's reading.

## One paragraph, for the record

The tree got its tools: the port manifest, the registry-derived counts, the
verify command, the brief, the find, the digest. Nine days of work used them,
and the arc got **more** expensive — 12.5 minutes of machine time against
3.9, 48.3 calls against 35.4, and a whole-tree gate 0.93 times per arc where
the plan wanted a package check. One of the six wave exits was met, and it
was met by stopping a run rather than by making one cheaper. The lesson is
not that the tools did not work — the reference-tree lookup fell, and the
delegation stopped — but that the exits were phrased as **kinds of call**
rather than as **cost per arc**, so the tree could satisfy them while the
arc got heavier. The next measurement should be one number: **the machine
time and the calls of a single arc, end to end**, and every change should be
judged by whether that number moves.

## How to repeat this

```text
# the tree's half
cargo xtask census
cargo xtask counts
cargo xtask ports --check

# the git half
git log --format='%H|%ad|%s' --date=iso --numstat

# the session half — the runtime's own tables
#   local_runtime_message_rows      every assistant message and its tool_calls
#   local_runtime_background_tasks  every tracked command and when it ended
# a tool call's measured duration is ended_at_ms - created_at_ms of its task
```

The sessions are found by their workspace:

```sql
select session_id from local_runtime_sessions
 where workspace_dir like '%craftkit%';
```

and the calls, with the duration of each command:

```sql
select json_extract(j.value,'$.tool_name')  as tool,
       json_extract(j.value,'$.tool_call_args') as args,
       t.created_at_ms, t.ended_at_ms
  from local_runtime_message_rows m,
       json_each(m.data_json,'$.tool_calls') j,
       local_runtime_background_tasks t
 where t.task_id = json_extract(j.value,'$.tool_call_result_data')
                   -> '$.details' ->> 'task_id';
```
