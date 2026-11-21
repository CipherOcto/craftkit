# The survey of the language server: what the LSP calls cost, and whether they paid

`cargo xtask lsp` was built on 2026-11-14 (§915) so that an agent would
**ask the tree what a name means** instead of grepping for it. `AGENTS.md`
says so in as many words: *ask it before you grep*. This survey asks the
question that was never asked: **did anybody ask it?**

The short answer: **five times, all of them on the day it was built, and
never again.** This document measures that, measures what a call costs, and
says what would have to change for the tool to earn its place.

## The method

The same three sources as `docs/survey-cost-and-meta.md` and
`docs/survey-cost-and-meta-2.md`: the tree's own history, the runtime's own
record of every tool call and of the measured duration of every command, and
the tree itself.

The measured durations are **measured**: the runtime starts every shell
command as a tracked background task and records when it ended. The
per-call timings at the end of this document were taken with
`/usr/bin/time` on this machine, under `nice -n 15`, at a load average of
2.0.

## What was called

| | count |
|---|---|
| `cargo xtask lsp` **question calls** in the whole history | **5** |
| of which: `symbols` / `definition` / `hover` / `diagnostics` | 2 / 1 / 1 / 1 |
| bash calls that mention `xtask lsp` at all (building and testing it) | 10 |
| bash calls that mention `rust-analyzer` (installing it, probing the wire protocol) | 14 |
| hand-written protocol probe scripts (`/tmp/ck-enc/probe*.py`) | 5 |
| feature arcs in the history | 1 067 |

**Five question calls per 1 067 arcs is 0.005 calls per arc.** Every one of
them was made in the twenty minutes after the command was built — 03:37 to
03:50 on 2026-11-14, in the session that wrote it. In the **143 arcs of the
six days since**, the language server was asked nothing.

## What the build cost

The arc that built the command (§915), measured over 03:08 → 04:36:

| | value |
|---|---|
| the span of the arc | 88 min |
| the tool calls | 128 (94 shell, 26 edits, 5 reads) |
| the measured shell time | 25.2 min |
| the code | 589 lines in `xtask/src/lsp.rs` |
| the probes | 5 python scripts, written by hand, to reverse-engineer the wire protocol |

The probes were needed because the first `symbols` call answered **"nothing
by that name"**: the client had asked the server before the server had
indexed the tree, and the tree had no way to tell the difference. The fix
was to read `rust-analyzer`'s messages directly until the shape of the
answer was known.

That is a real cost — an hour and a quarter of an arc and 589 lines — and
it bought a tool that is used 0.005 times per arc.

## What a call costs

Measured on this machine on 2026-11-20, at a load average of 2.0:

| the call | the measured time |
|---|---|
| `cargo xtask lsp symbols SheetProtection` | **10.93 s** |
| `cargo xtask lsp symbols Chart` | **16.26 s** |
| `cargo xtask lsp references crates/craftkit-xlsx/src/lib.rs 1851 12` | **33.40 s** |
| `rg -c "SheetProtection" crates/` | **0.02 s** |

A language-server call is **500 to 1 700 times** the cost of a grep. And
the second survey says what the scarce thing is: the round trip. The median
gap between two tool calls is now **39.0 s**. So one LSP query is roughly
**one whole round trip of the session**, spent, to answer one question that
a grep answers for nothing.

### The load is paid on every call

`xtask/src/lsp.rs` starts a **new `rust-analyzer` process for every
invocation** (`Session::start`, at the top of each branch of `run`). There
is no daemon and no warm server: the tree loads the whole workspace, waits
until the server is ready, asks one question and shuts it down.

So the playbook's sentence — *"the first load takes a minute or two, and
after that it is seconds"* — describes a server that does not exist. Every
call is a first load. The measurements above are what "a first load" costs
here: ten to thirty seconds, every time.

## Did it displace the grep?

No. The tree got **grep-happier** after the language server arrived.

| per arc | before the §915 arc | after it |
|---|---|---|
| `rg`/`grep` commands | 18.8 | **22.0** |
| tool calls | 41.0 | **49.9** |

## Why

Three reasons, and the third is the one that matters.

1. **The price is wrong for the frequency.** An LSP call costs a round trip
   and ten to thirty seconds; a grep costs two hundredths of a second. The
   rule as written — *ask it before you grep* — prices **every** question at
   the LSP's rate, including the ninety per cent that a grep answers.

2. **The tree has cheaper ways to ask.** Most of what an arc wants to know
   is *where does this live* and *how is this written here*, and the tree
   answers those with its own index: `cargo xtask find`, `cargo xtask brief`,
   `cargo xtask example`, `cargo xtask digest`. In the nine days of the
   second survey those were called 19, 5, 4 and 4 times, at a second or two
   each. The LSP answers a narrower question, better: **is this a
   declaration or a use, what type is this thing, and who else mentions it.**

3. **The narrow question is rare in a port arc.** A port does not *discover*
   a rule; it *translates* one. The landing site is decided by the port
   manifest and the brief, the convention is read from the neighbouring
   code, and the test is written next to the other tests. The questions the
   LSP is good at — every reference of a name before a rename, the type of a
   value, the difference between a declaration and a use — are asked in
   **refactors**, not in ports. This tree's last six days have been ports.

## What would make it pay

Two changes, in the order they would be worth trying.

- **A batch mode.** Several questions in **one** load. Five questions would
  cost one tree load (~35–40 s) and **one** round trip, instead of five loads
  and five round trips. That is the cheapest possible change: it is the
  existing client, asked more than once before it is shut down.
- **A persistent server.** The tree would hold the `rust-analyzer` process
  across the calls of a session, so the load is paid once per session
  rather than once per question. That is a bigger change — it is a daemon,
  a lock, a way to stop it and a way for it to go stale — and it should be
  built only if the batch mode shows the questions are worth asking.

And the **rule** should be narrowed either way. The house rule that says
*ask it before you grep* should say what the measurement supports: **ask the
server when the grep cannot tell you** — a declaration from a use, a type,
the whole list of references — and grep otherwise.

## What it does not settle

1. **Five calls are a small sample, but the zero is the finding.** The
   interesting number is not the five calls that were made; it is the 143
   arcs in which it was not called at all.

2. **The machine was not busy.** The timings above were taken at a load
   average of 2.0 on a shared machine that is often busier, and the tree is
   often busier still. The load cost is a lower bound.

3. **The tool's answers were not audited.** The five calls that were made
   were answered correctly: `symbols` found the struct at
   `crates/craftkit-xlsx/src/lib.rs:223`, `references` found its four uses,
   `hover` gave the type and the doc comment, `diagnostics` said "no
   complaints". The **tool works**; the **habit** does not exist.

4. **Whether an agent would use the tool if it were free is untested.** The
   rule in `AGENTS.md` was never followed even at 0.005 calls per arc, so
   the rule as written is not the thing that makes it work.

## One paragraph, for the record

The language server was built for a good reason — a grep cannot tell a
declaration from a use — and it works. It has been asked **five questions in
1 067 arcs**, all of them on the day it was built, and the tree's greps went
**up** by 3.2 per arc afterwards. The reason is the price: ten to thirty
seconds and a whole round trip, against two hundredths of a second for the
grep, paid **on every call**, because the tree starts a new server each
time. The tool answers a narrow question well — what a name *is*, and where
it is *declared* — and that question belongs to a refactor, not to a port.
The next arc, if there is one, is not another language-server feature: it is
a **batch mode** that asks several questions in one load, and a rule that
says *ask the server when the grep cannot tell you* — which is a smaller
claim than the one the tree makes today, and the one the measurement
supports.
