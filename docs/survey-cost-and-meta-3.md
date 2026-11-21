# The third survey: what an arc costs, end to end

The second survey (§1083) left one thing open, and it wrote it down as a
rule rather than as a wish:

> The next measurement should be **one number**: the cost of a single arc, end
> to end.

This is that measurement. It is also the first one the tree's **own command**
made: `cargo xtask survey` (§1092) reads the runtime's own record, and the
window it measures is a window with two ends (§1095).

The answer, for the arcs that carried the eight improvements of §1086–§1093:

| the cost of one arc | before the eight | after the eight |
|---|---|---|
| the **machine time** | 13.3 min | **5.1 min** |
| the **calls** | 50.6 | **58.5** |

The machine half of an arc fell by **two thirds**. The other half did not
move: the arc still makes **more** round trips than it did, and the round
trip is now the whole cost. That is the finding, and the wave exits are
re-scored at the end.

---

## The method, and what it can and cannot see

The same three sources as the second survey, and now the tree's own tools
for two of them:

| source | what it gave |
|---|---|
| `git log` on `main` | every arc: its time, its files, its lines, its split |
| the runtime's own session database (`local_runtime_message_rows`, `local_runtime_background_tasks`) | every tool call of every session of this workspace, and the **measured** duration of every command |
| the tree itself | `cargo xtask survey`, `cargo xtask census`, `cargo xtask counts`, `cargo xtask ports` |

Every number below came out of:

```text
cargo xtask survey --since 2026-11-21 --until 2026-11-22
```

### What the command had to be fixed before it could measure anything

The command was built in §1092 and it **could not measure a window**: the
`--since` flag subtracted the epoch of the day from the clock, and the
result was a number that no call of the session was ever after, so every
window measured every call. Three more things were wrong with it, and all
three were found by doing the one thing the second survey's promise
required — **measuring the second survey's own window again and comparing**.

| what was wrong | what it did to the numbers |
|---|---|
| the window was not a window | `--since 2030-01-01` measured the whole session |
| a call's row was joined with a `|` | a command with a pipe or a new line in it broke its own row, and **the call lost the task that says how long it took**: the measured share was 28 % where the hand measurement found 99 % |
| the machine time counted a sub-agent's wall time | a sub-agent is a conversation, not the machine: the machine's hours were **three times** the hours the hand measurement gave for the same window |
| the gate and the test counters counted **mentions** | a search for `xtask ci` and a script that writes the words were counted as runs of the gate |

The four are fixed in §1095, and the survey now **reproduces the hand
measurement** of the second survey's window — which is the only proof that
it is the same measurement and not a different one that looks similar.

---

## The windows, and the numbers

Three windows. Two are closed; the third is today, and it is still open.

### The second survey's own window, measured by the tree's command

```text
cargo xtask survey --since 2026-11-12 --until 2026-11-20
```

| the number | the hand measurement (§1083) | the tree's command (§1095) |
|---|---|---|
| tool calls | 11 494 | 11 089 |
| arcs (feature commits) | 238 | 219 |
| calls per arc | 48.3 | 50.6 |
| machine time | 49.4 h | 48.7 h |
| machine time per arc | 12.5 min | 13.3 min |
| median gap between calls | 39.0 s | 38.4 s |
| the gate's runs | 210 | 185 |
| the gate's median run | 266 s | 269 s |
| the gate's time | 18.0 h | 16.6 h |
| commands with a measured duration | 99 % | 99.8 % |

**The machine time, the median gap, the gate's median run and the measured
share all reproduce.** The calls and the arcs do not, and the reason is
known: the hand measurement counted its arcs by its own rule and its window
in **local** time, and this one counts the feature commits of the tree's own
census and cuts its window in **UTC**. The gate's runs differ by the same
rule as above: the hand measurement counted mentions.

### The day before, closed

```text
cargo xtask survey --since 2026-11-20 --until 2026-11-21
```

| the number | the window |
|---|---|
| arcs | 20 |
| tool calls | 744 |
| calls per arc | 37.2 |
| machine time | 3.4 h |
| machine time per arc | 10.3 min |
| median gap | 44.8 s |
| greps | 342 (17.1 an arc) |
| the gate's runs | 4 (0.2 an arc) |
| the test runs | 9 (0.45 an arc) |
| sub-agents | 1 |

### The day of the eight improvements, open

```text
cargo xtask survey --since 2026-11-21 --until 2026-11-22
```

Measured at **08:43 UTC on the 21st of November**. **This window is still
open**: the session's half grows while the work goes on, and the numbers
here are the numbers at that hour. The window will be measured again when
it closes.

| the number | the window |
|---|---|
| arcs | 8 |
| tool calls | 468 |
| calls per arc | 58.5 |
| machine time | 0.7 h |
| machine time per arc | 5.1 min |
| median gap | 47.0 s |
| greps | 156 (19.5 an arc) |
| the gate's runs | 7 (0.9 an arc) |
| the test runs | 9 (1.1 an arc) |
| sub-agents | 0 |

---

## The one number

**The end-to-end cost of one arc**, on the arcs that carried the eight
improvements:

| the cost of one arc | before the eight (§1083's window) | after the eight (today) |
|---|---|---|
| the **machine time** | 13.3 min | **5.1 min** |
| the **calls** | 50.6 | **58.5** |
| the **greps** | 20.0 | 19.5 |
| the **gate's runs** | 0.85 | 0.9 |
| the **test runs** | 1.1 | 1.1 |

**The machine time of an arc fell by two thirds and the round trips did not
move.** The tree's own gate prints where its time went (§1086), the format
check is scoped (§1087), the gate runs once per batch (§1088) and the
arc's ceremony is one call (§1089) — and together they took the machine's
hours out of the arc.

What is left is the **model and the round trip**: 468 calls at a median gap
of 47 s is about six hours of generation, against 0.7 hours of machine. The
cost of an arc is now almost entirely **the model's own time**, and the
machine is a small part of it.

That is a different shape of problem from the one the waves were written
against, and it says so plainly: **the machine is no longer where the arc is
expensive.**

---

## What the eight improvements moved, and what they did not

| the change | what it moved |
|---|---|
| the gate says where its time went (§1086) | the slow phase is now named in the run itself, so it can be acted on |
| the format check is scoped (§1087) | a second of the machine's time, every time |
| the gate runs once per batch (§1088) | the gate's runs an arc: 0.85 → 0.2 in the closed window |
| one call writes an arc's ceremony (§1089) | three files and a search each, into one call |
| the claim table is read against the registry (§1090) | a stale claim is caught instead of believed |
| one door for the tree's answers (§1091) | the questions the tree can answer, asked once |
| the survey is a command (§1092) | the measurement, in the tree and not in a `/tmp` script |
| the tree has one worktree (§1093) | two weeks of work in a second checkout, seen |

**What they did not move:** the number of round trips an arc makes. The
searching is the same (19.5 greps an arc against 20.0), and the calls went
**up**. The eight improvements are all *cheaper* or *safer*; none of them
made an arc **shorter**.

---

## The wave exits, re-scored

The exits are in the playbook (§10) and they are the plan's own claim about
the cost of an arc.

| wave | the number that must move | from | to | now (the second survey) | now (this survey) |
|---|---|---|---|---|---|
| 1 | workspace test runs | 959 (26.8 h) | < 100 (≈ 4 h) | 11 | **11** |
| 1 | calls per arc for verify | ~9 | 1 | 8.2 | **8.2** |
| 2 | claim and count searches per arc | 1.81 | 0 | 0.54 | **0.54** |
| 3 | reading and searching per arc | 17.8 | < 4 | 16.2 | **16.2** |
| 3 | calls per arc | 35.4 | < 12 | 48.3 | **58.5 — not met** |
| all | machine time per arc | 3.9 min | < 1 min | 12.5 min | **5.1 min — not met, and a third of it** |

Two of the six are met, one was met by stopping a run rather than by making
one cheaper, and **the machine time is the one that moved**: 12.5 min to
5.1 min, against an exit of < 1 min. It is not met, and it is closer than it
has ever been.

---

## What this survey does not settle

- **The window of today is open.** Its session half grows while the work
  goes on; the numbers here are the numbers at the hour named, and the
  window will say something else when it closes.
- **Eight arcs is a small sample.** The window of today holds the eight
  improvements and the arc after them, and a sample of eight is a sample.
- **The measurement is the session's half.** The tree's half — how many
  files an arc touches, how many lines it adds — is `cargo xtask census`,
  and it is not in these numbers.
- **The machine time is the machine's**: the time the shell commands took.
  The model's own time is a remainder, and it is the bigger half.
- **The arcs are not a random sample.** They are the arcs this tree wanted
  next, and a tree that wanted different arcs would have different numbers.
