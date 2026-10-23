# Fuzzing the readers

`cargo xtask fuzz` breaks the files the tree reads on purpose and hands them to
the headless CLI. It is a phase of `cargo xtask ci`, so it runs on every push.

There are **two targets**, because the tree reads two kinds of file through a
door a person can walk through by accident:

| The target | The file | The child | The corpus is written by |
|---|---|---|---|
| `design` | a design | `craftkit-cli eval` | `craftkit-cli sample --out` |
| `script` | a script of commands | `craftkit-cli run` | `craftkit-cli script --out` |

**Both corpora are written by the tree's own program.** That is the rule, and
it is why a target cannot be added without a writer: a corpus written by hand
would be a second copy of the format, and it would drift — the fuzzer would be
breaking a format nothing else in the tree speaks. §182 gave the tree the
writer the script door was missing.

## Why the reader is the thing to fuzz

Every other input craftkit takes is checked before it is used: a parameter is
read through a reader that knows its type, and a command is given a value the
command asked for. **A design file is different.** It is the one thing a
person can hand the program by accident — a file from another program, a file
half-written by a crash, a file somebody edited in a text editor — and it goes
straight into the model.

So the reader is the widest door in the program, and it is the door the
fuzzer pushes on.

## What a run does

1. It writes a corpus of **valid** files, with the tree's own program: the
   design's seed comes from `craftkit-cli sample --out`, and the script's from
   `craftkit-cli script --out`.
2. It **checks** the seed: the same child that gets the mutants has to
   *accept* it. A corpus the reader turns away is a corpus that tests
   nothing, and a fuzzer whose corpus is rubbish is green for the wrong
   reason.
3. It breaks each one in one of seven ways — a bit turned over, the file cut
   short, a piece of another file spliced in, a piece repeated, a piece taken
   out, a number replaced by one the reader cannot hold, and the file's own
   shape broken with the file still valid JSON.
4. It hands each broken file to the target's child and reads the verdict.
5. It minimises every crash into `fuzz-out/findings/`, with a note beside it.

**The shape that is broken is the target's own.** A design's shape is its
document — more nodes than the model allows, a frame rate of zero, a frame
rate over the limit, an empty range, a range outside what a time can be, two
nodes with one id, a note longer than the model allows, and a document
replaced by something that is not one. A script's shape is its **list of
commands** — a step that is not an object, a step whose command is not a
string, a step whose parameters are not an object, a command the registry does
not have, a parameter past the tree's own magnitude limit, and a base that is
not a document.

That last one matters most for the script door: a script **carries
commands**, so a broken script is not only a broken file — it is a broken
*instruction*, and the instruction is aimed at the engine.

## What is a crash, and what is not

| The child | The verdict |
|---|---|
| died by a signal | **crash** |
| exited 101 — the code a Rust program uses when it dies of a panic | **crash** |
| printed `panicked at` | **crash** |
| ran longer than the timeout | **hang** |
| exited 0 | accepted — the file was read |
| exited 1 or 2 | refused — the file was turned away with a message |

**A refusal is the reader working.** Most of what the fuzzer produces is a
refusal, and a run in which every input was refused is a good run.

The third row is the one that is easy to miss. The engine runs every command
under `catch_unwind`, so a panic *inside a command* is turned into an
ordinary error and the process still exits cleanly. Only the message says
what happened. A reader that is "handled" by an error message is still a
reader that panicked.

## The run is reproducible

One fixed seed, one fixed list of mutations, one fixed order. Two runs try
the same inputs in the same order, so a crash found on a laptop is the same
crash on the build machine, and a red CI run can be reproduced by the person
who has to fix it.

```bash
cargo xtask fuzz                      # the same run `ci` does: both doors
cargo xtask fuzz --target script      # one door only
cargo xtask fuzz --iters 5000         # a longer hunt
cargo xtask fuzz --seed 0x1234        # a different hunt
cargo xtask fuzz --timeout 60         # more patience for a slow machine
cargo xtask fuzz --cli ./target/debug/craftkit-cli
```

## Reading a finding

A finding is two files, and the **target is in the name** — so the two doors'
findings cannot collide in one folder:

- `fuzz-out/findings/design-crash-00123.craftkit` — the **minimised** input.
  Small, because a crash in a two-thousand-byte file is a puzzle and a crash
  in a forty-byte file is a sentence.
- `fuzz-out/findings/design-crash-00123.note.json` — what happened: the
  mutation, the seed it came from, the exit code, and the tail of what the
  child said.

To see it for yourself:

```bash
./target/ci/craftkit-cli eval fuzz-out/findings/design-crash-00123.craftkit
```

## What the fuzzer does not cover

It is worth being exact about this, because a gate that is trusted for more
than it does is worse than no gate.

- **It is not a proof.** A clean run says the reader survived *these* inputs.
  It does not say the reader is correct, and it does not say there is no bug.
- **It does not fuzz the UI, the control channel or the MCP server.** Those
  take JSON over a channel, and the tree has property tests for them
  (§175, §177).
- **It does not fuzz the crash report.** The third file the tree reads has no
  headless writer yet: the only producer in the tree is the desktop app's
  panic hook, and the CLI's crash actions are list, show and replay. A corpus
  written by hand would be a second copy of the report format, so the door
  waits for its writer. §182.
- **It does not fuzz the writer.** The corpus is written by the writer, so the
  writer is exercised on every run — but only on the shapes the writer itself
  produces.
- **A hang is reported like a crash**, and it fails the run. The timeout is
  ten seconds for reading a file of a few kilobytes; the reader is bounded, so
  ten thousand times the work is already a hang.

## Why the xtask depends on nothing of the tree's own

The xtask is the tool that *checks* the tree. A checker that cannot be built
when the tree does not compile is a checker that is silent exactly when it is
needed — and the `lists` gate reads the tree as text precisely so that it can
say something about a broken tree.

So the xtask depends on `serde` and `serde_json` and on nothing else, and the
design comes from the tree's **program** rather than from the tree's
**library**. The first version of the fuzzer wrote its corpus in process,
with the xtask depending on `craftkit-io`; the mutation suite caught the
cost, in the one case that depends on the property the change took away.

## Why the child is a separate process

Two reasons, and both are reasons the check can fail at all.

A **hang** is only visible from outside: a program that loops forever cannot
time itself out, and a parent can kill it. And the **exit code is the
verdict**: running the reader in the fuzzer's own process would catch the
panic and carry on, and the one place a panic still shows — the reader,
before the session exists — is exactly the place that would have been hidden.

## The gate's own teeth

A fuzzer that has never found anything is a fuzzer nobody knows works. The
tree proves it can find a crash: `xtask/src/fuzz.rs` runs the whole loop
against a child that dies on purpose — the tree's own test binary, called by
name — and checks that a finding came out the other end. That test is in the
tree, and the mutation script breaks the loop to check the test notices.
