# Contributing

craftkit is a scaffold, so the contribution here is rarely a feature. It is
usually one of:

- a **decision** made explicit and checked, so the next change does not have to
  re-derive it;
- a **gate** that turns a mistake someone will actually make into a failure at
  the moment they make it;
- a **recipe**, so the next agent does not have to work out the same sequence.

Those three are worth more here than features, because they are what survives
being copied into somebody else's project.

## Before you start

```bash
cargo xtask doctor    # can this machine build it?
cargo xtask ci        # seconds on a warm tree, minutes cold
```

Then read `AGENTS.md`. It is the rules; this file is the process.

## The shape of a change

**One topic.** One change, one branch, one commit when it builds. A change that
adds a command, changes the file format and refactors the evaluator is three
changes, and the one that gets reviewed properly is the one that did one thing.

**Land the gate before the gate.** If you add a rule, add the check in the same
change. A rule nobody enforces is a comment; a check with no rule is a gate that
blocks people for no stated reason.

**A recipe is part of the change.** If your change makes an existing recipe
wrong, fix the recipe in the same commit. Documentation that describes a
workflow you have just changed is worse than no documentation.

## Style

The rules are in `AGENTS.md`. What they are *for* is worth restating once.

```rust
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]
```

at the top of every production crate root, and:

```rust
match params.num("distance", 10.0) {
    Ok(distance) => { /* … */ }
    Err(e) => return Err(e.into()),
}
```

rather than `params["distance"].as_f64().unwrap_or(10.0)`.

The first says *this may not panic*. The second says *this cannot panic even if
the input is hostile*. The second is the one that matters, and it is why
`crates/foundation/src/params.rs` exists as a separate crate instead of a helper
function: it is the boundary, and putting it at layer 0 means nothing below it
can get at the raw JSON by accident.

Formatting is `cargo fmt` with `rustfmt.toml` (`max_width = 110`,
`use_small_heuristics = "Max"`). That is why the deny attribute above is
multi-line: it is 112 characters on one line, and rustfmt reflows it. Paste the
block as written, then let `cargo fmt` have it.

`cargo xtask lists` compares the attribute after normalising whitespace, case
and the `clippy::` prefix, so the one-line and multi-line forms both pass — but
the lints must match in **order**, and they must be all six.

Comments explain **why**, in prose, for the person who arrives in six months —
not what the next line does.

## Adding things

- **A command**: `crates/engine/src/commands.rs`, plus tests. Nothing else may
  change a document. See `docs/recipes/add-a-command.md`.
- **A saved field**: one new file at `crates/io/src/format/pending/<name>.rs`.
  Never edit `FORMAT`, `JSON_STEPS`, the history in `format.rs`, or a `v<N>/`
  folder. See `docs/recipes/change-the-file-format.md`.
- **An icon**: draw it. `crates/ui/src/icons.rs` is a list of line segments and
  arcs; add one and draw it. Do not add a file.
- **A crate**: pick its layer, add the deny attribute, add it to
  `xtask/src/layers.rs::TABLE` *and* to the table in `docs/architecture.md` in
  the same commit (`cargo xtask lists` compares them), keep `members` sorted,
  and add it to the table in `AGENTS.md` and `README.md` if it is a library
  crate.
- **A control-channel method or an MCP tool**: read
  `docs/recipes/add-a-control-channel-method.md` first. The short answer is that
  a control method is usually one match arm and nothing else, because the
  commands already exist.

## Adding a gate

A gate earns its place by preventing a mistake that has happened, or that is
obvious enough that it will. Write it in `xtask/src/`, in the house style — no
`unwrap`, errors as `Result`, because a gate that panics on a malformed manifest
is a gate that lies.

Then:

1. **Unit-test the rules**, not the file reading. The interesting part is the
   comparison, and it should be testable without a repository on disk — see
   `xtask/src/layers.rs`, where `check` takes a model and `from_metadata` builds
   it.
2. **Add a test asserting the rule against this repository**, so the gate cannot
   quietly stop checking anything (`the_project_ships_no_image_assets`,
   `the_real_architecture_table_matches_the_code_table`).
3. **Make it skip, not pass, when it cannot run** — with a reason, printed where
   it cannot be missed.
4. **Register it** in `xtask/src/ci.rs::PHASES` if it belongs in `ci`, and in
   `USAGE`. There is a test that the usage text and the phase list agree, and
   `PHASES` is the only list: `ci::run` iterates it, so a phase you add there is
   one that runs, and a phase you remove is a failing test rather than a silent
   omission.

Order matters in `ci`: the cheap file-reading gates come first so a formatting
slip is reported in a second rather than after a compile.

A gate returns `Outcome::Ran` or `Outcome::Skipped`, and `ci`'s closing line is
built from those outcomes rather than written out by hand. Keep it that way. The
hand-written version listed `tests` and `wasm` on a build where the call that ran
them had been deleted, so `cargo xtask ci` reported green without running either.

### Show the gate that fails

A gate nobody has ever seen fail is a gate nobody can vouch for. When you add
one, add a line to `scripts/mutation-test-gates.sh` that breaks the rule and
asserts the gate rejects the tree. Four gates in this project had been quietly
unable to fail — a substring search, a check that existed only as a unit test, a
table reader that could not see a row glued onto the end of a line of prose, and
`ci` itself, which reported gates it had not run — and that script found all
four. The script found a fifth in itself: it printed its missed mutations in red
and exited 0.

Give each mutation its own line and its own label. Three of them once shared a
single check, so the label named one of the three things it actually verified.

The script refuses to run a mutation that does not apply, compares checksums at
the end so it cannot leave the tree changed, and exits non-zero when a gate
misses. All three exist because it got all three wrong once. It also counts the
cases it runs rather than dividing by a hardcoded total, so the ratio it prints
cannot drift away from the checks it actually did.

**A mutation's target file must also be in `scripts/mutation-files.txt`.** That
file is the list: the script reads it on every run, and the array it fills is
rebuilt from it each time, so a path added to the script's own array is lost
before the first case runs. The snapshot, the restore and the checksum all walk
that one list, so a mutation against a file that is not on it is restored by
nothing and checked by nothing:
the tree keeps the broken source and the run still prints "Tree unchanged" and
exits 0. That happened once, and the next ordinary `cargo test` was what noticed
it — not the script that made the mess. `swap` and `append` now refuse a target
that is not listed, so you will be told rather than left to remember.

`scripts/mutation-test-gates.sh` runs in CI as the `mutation` job of
`.github/workflows/mutation.yml` — its own workflow, because it takes about
an hour and a push must not cancel it (§252). It runs nightly, on demand
(`workflow_dispatch`) and on every pull request; the fast gates run on every
push. `--self-test` asserts the script itself still fails when a gate stops
catching. Run both locally before you claim a new gate works:

```bash
scripts/mutation-test-gates.sh --self-test   # must exit non-zero
scripts/mutation-test-gates.sh               # must exit zero, tree unchanged
```

## Running the real window

One check cannot run in CI, because CI has no display: `scripts/live-window-check.sh`
drives the real binary on a real display and compares the window's own pixels
before and after a command arrives over the control channel. It found the defect
in `KNOWN-GAPS.md` §21 — the window used to keep painting a frame from before
the command, while the engine had already moved on.

```bash
DISPLAY=:1 scripts/live-window-check.sh   # 0 passed, 1 failed, 77 no display
```

It exits **77**, not 0, when it cannot run. A check that reports success where it
could not run is the same defect it exists to catch, so there is no default that
looks like a pass. Captures are left in `target/live-window-check/` — the check
keeps its evidence, because a check that deletes what it saw is one nobody can
look at afterwards.

It measures **this application's** window, matched by the pid that owns it
(`_NET_WM_PID`), because a second instance has a window with the same name and
the check used to measure whichever the X server listed first — reporting
*"the engine changed and the window did not"* about a window that had never
received the commands (`KNOWN-GAPS.md` §56). The part that decides which
window to measure is pure, so it has a self-test that needs no display:

```bash
scripts/live-window-check.sh --self-test   # 0 when the window picker still holds
```

## Never break the browser

Everything below the apps must build for `wasm32-unknown-unknown`; `cargo xtask
wasm` checks it and `cargo xtask ci` runs it. It catches a dependency that does
not build for the browser, and a `cfg` branch that stopped compiling.

It does **not** catch `std::fs`, `std::net`, `std::thread` or `std::env`: all of
those compile for `wasm32-unknown-unknown` and fail at run time instead. This was
measured, not assumed — see `scripts/mutation-test-gates.sh` and `KNOWN-GAPS.md`.
Nothing replaces opening the page.

## Honest reporting

Say what passed, what failed, what was skipped and why, and what you did not
check. A claim of success you have not verified is worse than an admitted gap,
because it stops the next person from looking. This applies to agents exactly as
it applies to people.

## What is not accepted

- A panicking path in non-test code, however unlikely.
- A dependency from a higher layer to a lower-numbered one.
- An image, font, audio or model file anywhere in the tree.
- A UI toolkit below L6.
- A behaviour that lives outside the command registry.
- A rule without a gate.
- Another product's code, artwork, icons, text, or data — commercial or not.
  Behaviour comes from public specifications and general conventions.