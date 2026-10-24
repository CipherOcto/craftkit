# Fix a crash

**When you need this** — someone says the application crashed, or a test
panicked, or a command returned `internal`, or you read a panic in the log. Not
for "this input gives the wrong answer"; for "this should not have been
possible".

**Before you start**

Nothing. Crashes are found when they are found. But read
[the never-crash rule](../agent-playbook.md#the-invariants) first, because the
fix is usually in that list rather than in the code that panicked — and
[docs/patterns/clone-before-edit.md](../patterns/clone-before-edit.md), which is
why the document was still intact when the panic was caught.

---

## What a crash leaves behind

The application writes a report when it catches a panic: the panic hook for a
panic that escapes, and the render path for a panic in the drawing code —
which the hook cannot reach, because the thread that draws holds the session's
lock (§76). A process that is *killed* writes nothing at all: catching a
signal needs `unsafe` FFI, and this tree forbids `unsafe`. So "the application
was killed" means "there is no report", and there is nothing to replay.

One file, one directory, one id:

```text
<folder>/<id>/
└── report.json    what happened, which command, the id and the timestamp,
                   the document as it was, and the command trace
```

**One file.** The folder used to hold three: the report, a copy of the design
and a copy of the steps. Nothing read the design's copy, and the replay's copy
was read by a door nothing called — so a report that arrived with the copies
disagreeing had **two answers**, and the tree noticed neither. §185.

The folder is `craftkit_engine::crash::report_dir()`:

1. `CRAFTKIT_CRASH_DIR`, if set;
2. else `$XDG_DATA_HOME/craftkit/crashes`;
3. else `~/.local/share/craftkit/crashes`;
4. else `<temp>/craftkit/crashes` in a container with no home.

Ids are `20250907-093012-7f3a` — `YYYYMMDD-HHMMSS` in **UTC**, then four hex
digits of sub-second position. They sort into the order the crashes happened,
and they say when it happened without anybody having to look them up. Two
crashes in the same second land on the same id only if their sub-second
positions agree, and when that happens the second report is filed **beside**
the first (`…-7f3a-2`) rather than on top of it: a crash report is the only
evidence a crash happened, and it is never destroyed by the next one. §58.
`sanitize` reduces any input to `[A-Za-z0-9_-]` before it is joined to a
path, so an id from a command line cannot escape the folder.

The file is written `0600` and the report's own folder is `0700`: a
report carries the design, so it is the most private thing this program
writes. A folder that was **already there** — `CRAFTKIT_CRASH_DIR` can name a
shared one — keeps the permissions its owner gave it; only what the tool
creates is the tool's to protect. §57. Sharing a report never needed the
file to be world-readable: `craftkit-cli crashes show` prints it.

---

## Steps

### 1. Find the report

```
ls -1t ~/.local/share/craftkit/crashes
```

```bash
craftkit-cli crashes list
craftkit-cli crashes show <id>
craftkit-cli crashes replay <id>
```

An agent on the MCP server reads the same reports with the `craftkit_crashes`
tool: `action` is `list`, `show` or `replay`, and `show` and `replay` need the
report's `id`. The three actions are the engine's — `crash::action_in` — so
the CLI, the control channel and the MCP server cannot answer differently
about the same report. §113.

The engine names exactly this surface: `EngineError::Unknown` says *"run
`craftkit-cli commands` for the list"*, and `crash::sanitize`'s doc comment says
`craftkit-cli crashes show …`. The CLI calls `crash::list()`, `crash::show(id)`
and `crash::reproduce(id)`, each of which has an `_in(dir, …)` variant so a test
can point at a temporary folder instead of the user's.

Reading the folder by hand still works — the layout is one file and a
`report.json` — and it is worth doing once, because it is what makes the format
readable rather than something the tool decides for you.

If the report is not on this machine, ask for `report.json`: it carries the
design, the base and the steps. There is nothing else to ask for. And give
that file to the tree with the door that takes a **file**:

```bash
craftkit-cli crashes show --file report.json      # read it
craftkit-cli crashes replay --file report.json    # replay it, and say whether
                                                 # it reproduced the design
```

The file does not have to be in the crash folder, and it does not have to be
named after its id: the door reads the file you were handed, with the tree's
one reader and the tree's one replay, so it cannot answer differently from
the folder's door. §195.

A report is **input**, not output — it arrives from somebody else's disk, so
the reader bounds what it will accept: `report.json` is
read only up to `MAX_REPORT_BYTES`, a replay runs at most
`MAX_COMMAND_STEPS` commands, and **both** of the report's documents — the
base it was replayed onto and the design it recorded — are validated the
way a design file is. A report that breaks one of those rules is **refused
with the reason**, not repaired and not half-run.

### 2. Read the report

`report.json` is a `CrashReport` (in `crates/engine/src/crash.rs`):

```rust exact
// crates/engine/src/crash.rs
pub struct CrashReport {
    pub id: String,
    pub unix: u64,
    pub command: String,     // which command was running, when one was
    pub message: String,
    pub design: craftkit_model::Document,   // the document at the moment of the crash
    pub base: craftkit_model::Document,     // the document the trace starts from
    pub replay: Vec<ReplayStep>,     // the commands, oldest first
    pub dropped: usize,              // commands the trace could not keep
    pub at: i64,                     // the playhead, in ticks
}
```

The block is marked `exact`, so `cargo xtask lists` compares it with the
source field by field: the same names, in the same order, with the same
types. It was not marked, and §72 changed two of the types — `design` and
`base` became documents rather than JSON trees — and this recipe went on
describing the old shape. A quote that says it is exact is checked; a quote
that does not is an illustration, and an illustration is allowed to drift.

`message` is the panic text. `command` is where it came from. If `command` is
empty the crash was not inside a command — look at the message and at the
design's size first.

`design` and `base` are **two** documents, and the distance between them is
the whole point. `base` is the state the session was in when the recorded
commands started; `design` is the state it crashed in. A reproduction applies
the commands to `base` and has to arrive at `design`.

`dropped` is how many commands the trace had to leave behind. The trace is
bounded — a session can run for days — so a report cannot always hold every
command, and a report that dropped some says so here rather than letting you
believe the replay is everything that happened.

### 3. Reproduce it, and check that it reproduced

The report's `replay` is `Vec<ReplayStep>`, each
`{ "command": …, "params": { … } }`. That is exactly `Session::execute`'s
signature. The reproduction starts from the
report's **base**, not from an empty session — the recorded commands are
relative to the document they ran against:

```rust
let mut fresh = Session::new("replay");
fresh.load_document(base);
for step in steps {
    fresh.execute(&step.command, &step.params).expect("replayed");
}
```

`craftkit-cli crashes replay <id>` does this and then **compares the result
with the design the report recorded**, printing either "the replay reproduced
the design in the report" or the ways in which it differs. That comparison is
the point: a replay that builds a different document and says nothing about it
is worse than no replay, because it looks like a reproduction.

The same door takes a report in a file: `craftkit-cli crashes replay --file
report.json` replays the file you were handed, with the same comparison and
the same words. §195.

`the_replay_rebuilds_the_document_that_crashed` (in `crates/engine/src/crash.rs`)
is the loop, and `a_replay_says_whether_it_reproduced_the_document` (in
`crates/engine/src/crash.rs`) is the verdict. Copy them; change the commands to
the ones from the report.

If a step fails rather than panics, the panic is downstream of that step — read
the failure message first. `craftkit-cli crashes replay <id>` lists the
failures, and says how many commands the report had to drop.

> **The report's `design` is the target, not the starting point.** The replay
> starts from `base` and has to arrive at `design`. When the two
> match, the report reproduces the crash; when they do not, the differences the
> replay lists are where to look. `dropped` says how much of the session the
> replay could not cover.

### 4. Find the actual cause

`Session::execute` already caught the panic
(it is in `crates/engine/src/session.rs`): the document was restored, the evaluator
invalidated, and the error surfaced as `EngineError::Internal` with the panic
text as `detail`. So the process did not die — what you are looking at is either

- a panic that *escaped* `execute` (a bug outside a `run` function — the
  evaluator, the loader, the renderer); or
- a refusal that is not a crash at all, wearing `internal` as a disguise; or
- the app's own panic hook firing on the way out.

Read `EngineError::kind` (in `crates/engine/src/error.rs`) for the `kind`
strings. `internal` is the one that means "something broke that should not
have", and it is the only kind that is a bug in this repo rather than in the
input.

### 5. Fix it, and fix it in the model

Prefer, in order:

1. **Return an error.** The input is hostile — a command parameter, a design
   file, a control-channel line. Name the parameter and say what was wrong.
2. **Bound it.** If the value has a natural range, use the existing cap:
   `limits::MAX_NAME`, `limits::MAX_NODES`, `limits::MAX_PATH_SEGMENTS`,
   `craftkit_core::MAX_MAGNITUDE`, `craftkit_core::MAX_TEXT`,
   `craftkit_core::ids::IdCounter::MAX`.
3. **Saturate, do not wrap.** `Tick`'s arithmetic is deliberately saturating for
   this reason (`crates/core/src/tick.rs:24`); follow that convention elsewhere.

Never: clamp silently, catch and continue, or turn the panic into a default.

### 6. Prove it

```
cargo test -p craftkit-engine
cargo xtask ci
```

---

## The test you must add

A regression test built from the actual report. Put it in
`crates/engine/src/crash.rs`'s existing `mod tests` when the crash was in the
engine; in the crate that owns the failing code otherwise. Use `temp_dir(tag)`
(in `crates/engine/src/crash.rs`) for any filesystem part — it gives each test
its own folder, named after it, so a failing run leaves evidence behind. Do
**not** set `CRAFTKIT_CRASH_DIR`: the environment is process-wide and every
test in the binary would race.

The shape, for the replay-onto-the-recorded-base case:

```rust
#[test]
fn a_replay_of_a_report_reproduces_the_crash() {
    let dir = temp_dir("regression-20250907-093012-7f3a");
    let mut original = Session::sample();
    original.execute("node.add", &json!({ "name": "Gate", "kind": "filter" })).expect("ok");
    // …the commands from the report, and the panic they used to cause…

    let report = build(&original, "node.enable", "index out of bounds: the len is 1 but the index is 1");
    write_to(&dir, &report).expect("written");

    let outcome = reproduce_in(&dir, &report.id).expect("replayed");
    assert!(outcome.reproduced, "the replay did not reproduce the crash: {:?}", outcome.differences);
}
```

`reproduce_in` starts from the report's base, runs the steps, and compares the
result with the recorded design — so the assertion *is* the claim. When you
want the document itself rather than the verdict, the loop underneath it is
three lines.

### Why it has to be built from the report

Three reasons, and each one is a real past failure mode:

1. **The replay proves the reproduction still reproduces.** A test that
   hand-writes the *symptom* rather than the *sequence* passes after the fix for
   the wrong reason, and the next person to touch the code cannot tell whether
   it still guards anything.
2. **The report pins the starting state.** If the crash needed a loaded
   document rather than an empty one, the report's `base` is it:

   ```rust
   let mut fresh = Session::new("replay");
   fresh.load_document(document_from_json(&report.base).expect("base"));
   ```

   `Session::load_document` (in `crates/engine/src/session.rs`) is the public
   entry point. A direct assignment to the document would work too — the
   evaluator's cache is content-addressed, so a restored document cannot be
   served a state belonging to a node it no longer has — but `load_document`
   also recomputes the plan, clears the dirty flag, and starts the command
   trace again from the document it loaded.
3. **It asserts the *absence* of a crash, not the presence of a value.** The
   test passes when every step returns `Ok`. That is the whole assertion, and
   `hostile_params_never_panic`
   (in `crates/engine/src/session.rs`) shows the general shape of it: the guard
   is that nothing panics, everything else is setup.

> **`craftkit-engine` does not depend on `craftkit-io`.** If your test wants to
> call `craftkit_io::load_text` on a saved `.craftkit` file rather than
> `serde_json::from_str` on the report's `design`, it cannot live in
> `crates/engine/src/crash.rs` — the dependency does not exist and adding it is
> a layering decision, not a test change. Use `serde_json::from_str` into
> `craftkit_model::Document` (the model derives `Deserialize` directly), or put
> the test in `crates/io`, which sits below the engine and can see both.

---

## The failure this recipe is designed to prevent

### The fix that does not fix the crash

The most common outcome is a `match` arm added to swallow the case:

```rust
// Do not do this.
_ => return Ok(Outcome::new("nothing to do")),
```

The test goes green because the crash is gone, and the panic has become an
`Outcome` saying nothing happened. The user loses the operation with no error
and no explanation. Worse, the *next* panic in the same function now takes the
same arm, so the second bug is invisible too.

The discipline that prevents it: the regression test asserts the *result*, not
just the absence of a panic. If the fixed behaviour is an error, assert
`is_err()` and assert the message names the parameter. If it is a clamped
value, assert the clamped value. A test that only says "did not panic" is a
test that will keep passing after the next regression.

### A crash "handled" at the wrong layer

`Session::execute` restores the document on both the error path and the panic
path — the `Session::restore` function in `crates/engine/src/session.rs`, which
both branches call. That makes the engine
robust — and it makes an *engine-internal* bug look like a command failure. A
`bad-parameter` for an input the engine's own model can represent is a bug in
the caller, not in the user.

When you investigate, check the `kind` before you fix anything:

| `kind` | Meaning |
| --- | --- |
| `unknown-command`, `disabled`, `bad-parameter`, `invalid-value` | the input was wrong; the behaviour is correct |
| `invalid-document`, `cannot-evaluate` | the document is in a state the model forbids |
| `kernel` | a wrapped third-party failure; `guard` turned a panic into an error |
| `internal` | **a bug** — either a panic that was recovered, or a refusal that should not have been a refusal |

Only `internal` and a `kernel` failure that should have been impossible are
crash fixes. Everything else is a refusal being reported correctly.

### Not filing the report at all

The report is written automatically. If you reproduce a crash from a log line
and there is no report folder, something ran a path that does not go through
`crash::record_in` — the app's hook or its exit path, not the engine. That is
its own bug: the next occurrence will cost the same investigation again, because
nothing recorded the sequence.
