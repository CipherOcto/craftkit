# Known gaps

What this project does **not** do, what it claims to do and does not, and what
a gate cannot see. Every entry says how it was found and how you can check it
again, because a gap list with no evidence is a list of opinions.

The entries marked **fixed** were found by this audit and have been repaired;
they are kept because "we looked and here is what we found" is more useful than
"everything is fine", and because the mutation that found each one is still in
`scripts/mutation-test-gates.sh` so it cannot come back unnoticed.

Run the audit yourself:

```bash
scripts/mutation-test-gates.sh     # break each rule, check the gate notices
cargo xtask doctor                 # what this machine cannot do
cargo llvm-cov --workspace --summary-only --profile ci   # §14
```

---

## 1. Gates that could not fail — fixed

The most valuable kind of gap, because a gate that cannot fail teaches people to
trust a check that has never checked anything.

| Gate | The mutation it missed | Now |
|---|---|---|
| `web` | Renaming the canvas element's `id` while the name survived in the stylesheet and a comment. The check was `html.contains(CANVAS_ID)` — a substring search. | `canvas_element_is_correctly_identified` parses the opening tag and its `id` attribute. A second rule checks the stylesheet has a rule for it. |
| `lists` | An unsorted workspace `members` list. The check existed only as a unit test, so `cargo xtask lists` printed "lists are sorted" about a list it never looked at. | The gate reads and checks the list. |
| `assets` | An `ATTRIBUTION.md` row glued onto the end of a prose line. The document had **no trailing newline**, so appending a row produced a line that does not begin with `\|`, and the table reader correctly refused to see it. | Added `mentioned_files`: a name mentioned anywhere but claimed by no row is reported as malformed. The missing newline in `ATTRIBUTION.md` is fixed. |
| `ci` | Deleting the one call that ran the tests and the wasm check. `cargo xtask ci` printed **"ci green — …, tests, wasm"** and exited 0 without running either. §2 has the whole story. | One `PHASES` list that `run` iterates, a closing line built from the phases that ran, and two tests that pin both. |

Verified by re-running the mutation script: **every case caught**, and the one
that is not is the wasm limitation below, which is deliberate and is printed as
a `known` line rather than hidden.

This paragraph deliberately carries **no number of its own**. It used to say
"28 of 29 caught", which was true on the day it was written and was **ninety
cases out of date** by the time the next section was added — a hand-written
count of a thing that grows is stale by construction. §71.

The number in that line is computed by counting the cases the script runs, not
written by hand. It used to be `caught / caught + 1`, which quietly assumed
there was exactly one un-catchable case; two labels also claimed to test one
thing while three mutations were applied under a single check. Both are fixed,
because a gate that reports a wrong ratio is the same species of gate as a gate
that cannot fail — and `cargo xtask lists` now checks that the ratio is still
computed, rather than written out.

## 2. `cargo xtask ci` reported gates it had not run — fixed

The worst thing found, and the reason §1 exists.

`ci::run` ran the six cheap gates from the `PHASES` constant, and then called
`tests_and_wasm(root)?` separately, after the loop. Its closing line was a
string literal:

```rust
println!("\nci green in {:.1}s — fmt, assets, lists, layers, web, clippy, tests, wasm.", …);
```

Delete that one call and the result is a green CI run that ran neither the tests
nor the wasm check, and **says it ran both**. This was verified by doing exactly
that, not by reading the code and suspecting: exit code 0, "ci green", and the
full list of phases in the summary.

Nothing noticed, for three separate reasons, and each of them had to be fixed:

1. **No test ran `ci::run`.** `xtask/src/ci.rs` was the least-covered file in the
   workspace at 41.03%. The three tests it had asserted over the *constant*
   `PHASES` — which was accurate — and never over the function that used it.
2. **The summary was a claim, not a record.** It could name any phase and be
   believed. It is now built from the outcomes.
3. **The mutation script did not touch `ci`.** It broke six gates and never the
   one that runs them.

Fixed: `PHASES` now holds all eight phases and `run` iterates that one list, so
a dropped phase is a failing `assert_eq!` rather than a silent omission. Gates
return `Outcome::Ran` or `Outcome::Skipped`, and `ci::summary` builds the closing
line from them — a phase that could not run is named as skipped and is **not**
listed among the ones that ran. `the_closing_line_names_what_actually_ran` pins
that, and both mutations are in the script.

## 3. The audit's own instrument could not fail — fixed

`scripts/mutation-test-gates.sh` is a gate too, and it had the defect the whole
exercise exists to find: **it printed its failures and exited 0.**

Found by making it fail. Replacing every gate in it with a command that always
succeeds produced eighteen red `MISSED` lines, the correct explanation, "Tree
unchanged" — and exit code 0. In CI that is a green build with the instrument
shouting inside it, which is worse than having no instrument, because the red
text is on a line people learn to scroll past.

It is a section of its own rather than a row in §1 because §1 is about the
project's gates and this one is about the thing that audits them. The same
sentence covers both: *a gate that cannot fail teaches people to trust a check
that has never checked anything.* This one taught it.

Fixed: the exit status is now part of the result. A run with a missed mutation
exits 1, and the reporting lives in a `finish` function rather than at the end of
the script body. `--self-test` then asserts the negative cheaply — it injects one
synthetic case that is guaranteed to miss, drives it through the real
`expect_catch` and the real `finish`, and must exit non-zero:

```console
$ scripts/mutation-test-gates.sh --self-test; echo $?
  MISSED  synthetic: a gate that never rejects
1 of 1 gate(s) MISSED their mutation:
A gate that cannot fail is worse than no gate. Fix the gate, or the check.
1
```

The workflow asserts that this command fails, so restoring the old behaviour
turns CI red on that line before the full run can be believed.

## 4. A security boundary with no test on it — fixed

The most severe thing this audit found, and it is not a gate at all.

The control channel is a JSON endpoint on a TCP socket that runs engine
commands. `ServerOptions::allow_remote` decides whether it binds loopback or
`0.0.0.0` — every interface on the machine. The field's own documentation says
what that means: *"a control channel on a routable interface is remote code
execution with a JSON syntax."*

**The default had no test.** There was a test named
`the_listener_is_loopback_only_by_default`, and it passed a hardcoded `false` to
`bind()` rather than the default from `ServerOptions::new`. So it asserted that
the function treats `false` as loopback, and said nothing about what the program
actually does. Flipping the default to `true` — one character — left all 41
control tests green:

```console
$ cargo test -p craftkit-control
test result: ok. 41 passed; 0 failed
```

A test named after a property that nothing checks is the most expensive kind of
test, because it is read as proof.

What was *not* wrong, checked while looking: the token itself. The token file is
written `0600` inside a `0700` directory, and there is a test asserting the
mode. Rejecting a wrong token, a missing token, and a token presented after the
first request are all covered. So the token was never the weak link — the socket
address was, and the socket address was the one thing with no test.

(This paragraph also said the permissions were set **before** the content rather
than after. It was not — the *directory*'s were, and the *file*'s were not. The
file was created with the umask, which measured `0664`, and restricted
afterwards, so the secret was on disk in a world-readable file for the length of
one call. §55 found it. The sentence here was wrong when it was written, and it
was wrong in exactly the way §53 describes: a section that checks a mechanism
and then describes it in detail is still a *description*, and it can be wrong
the same way a test can be green.)

Fixed with two tests:

- `the_default_is_loopback_and_the_default_is_what_binds` reads the default from
  `ServerOptions::new` and binds with it, so the property is about the program
  rather than about the function.
- `nothing_in_the_tree_turns_the_control_channel_into_a_remote_one` reads every
  `.rs` file under `crates/`, `apps/` and `xtask/`, so the flag being set where
  the server is built fails the build. That is the realistic way the boundary
  goes, not a flipped default.

Two things went wrong while writing the second one, both worth recording because
they are the failure mode of a tree-scanning test:

1. It failed on a clean tree, because its own source contained the string it was
   searching for. The needle is now assembled at run time.
2. Then it passed on a probe that set the flag with `=`, because it only knew the
   `: ` spelling a struct literal uses. Both spellings are searched now, and
   both are in `scripts/mutation-test-gates.sh`.

**The residual, stated rather than hidden:** the `allow_remote == true` branch of
`bind()` is still never executed by a test, because the only honest way to test
it is to bind a routable interface and look at the result. That is the one branch
this audit chose to document instead of fix. The flag is off by default, nothing
in the tree sets it, and the tree-wide test now fails the build if that changes.

## 5. A format migration nobody ran — fixed

The design file format is the one thing in this project that, going wrong,
produces wrong values rather than an error.

`AGENTS.md`, `CONTRIBUTING.md` and four docs all state the rule: a change that
alters what is saved **adds one file** under `crates/io/src/format/pending/`, and
the release folds them in. A pending file holds a `step` that rewrites a document
written by the previous version. It only runs if the file is also listed in the
`FILES` table in `pending.rs` — and nothing checked that against the disk.

Writing `pending/probe.rs` and running the suite:

```console
$ cargo test -p craftkit-io
test result: ok. 24 passed; 0 failed
```

All four pending tests passed with a migration sitting in the directory that
`names()` never returns and `apply` never calls. A design saved by an older
version would load without its rewrite. Not an error — just wrong values, which
is the failure mode nobody looks for.

The reason is worth recording, because it is the same shape as §4:

- `every_pending_file_is_wired_into_apply` asserted `FILES.is_empty()`. A test
  named after a property, checking a constant.
- It cannot see the filesystem at all, so it is blind to exactly the direction
  that happens in practice — you write the migration file first and register it
  second, if at all.
- The one direction it *does* cover (a name with no file) failed on purpose, as a
  standing reminder to wire `apply` when the first change lands.

Fixed with two tests:

- `the_files_on_disk_and_the_table_agree` reads the directory and compares it
  with `names()` in both directions, and its failure message names both sides.
- `every_listed_change_is_declared_and_applied` closes the last hole in the loop:
  a listed file with no `mod` declaration is not compiled at all, so a mistake in
  it cannot even fail to build, and a declared file never called from `apply`
  compiles perfectly and does nothing. This one is vacuous while the list is
  empty — the normal state between releases — and is a tripwire for the first
  real change.

**Two durable facts about this mechanism, stated because they are easy to be
surprised by:**

1. `pending/` is empty today, and **git does not track empty directories**. A
   fresh clone has no `pending/` at all. The new test treats a missing directory
   as empty rather than failing, because that is a real state and the table is
   correctly empty for it — but the directory itself is not durable, and whoever
   adds the first change creates it.
2. The whole mechanism has therefore never run. Every part of it — `step`,
   `FILES`, `apply`, the fold script — is exercised only by its own tests and
   never by an actual format change. The first one is the first time any of it
   meets reality.

## 6. "Rust only" was never enforced, and never true — fixed

`AGENTS.md` states it as absolute: *"Rust only. The single exception is
JavaScript the browser requires."*

Nothing checked it, and the statement was already untrue. The tree holds **five**
files that are not Rust:

```text
apps/craftkit-web/index.html      the module script the page imports
apps/craftkit-web/sw.js           a service worker; browsers only run these as JS
scripts/mutation-test-gates.sh    the gate instrument
scripts/fold-format-pending.sh    runs once, by hand, at a release
.github/workflows/ci.yml          continuous integration is YAML
```

`cargo xtask assets` looked like it should have caught this and did not, which is
the interesting part. It is the gate that owns "what may exist in this tree", it
walks every file, and it rejects anything that embeds a file — but it recognises
a file by extension, and `ASSET_EXTENSIONS` is binary and media formats only. A
script is not an asset.

So the check saw every one of those files, every time, and classified all of them
as "not my problem". Measured, not suspected:

```console
$ printf 'print("x")\n' > crates/ui/build_helper.py
$ printf 'const x = 1;\n' > apps/craftkit-web/extra.js
$ printf '#!/bin/sh\necho hi\n' > crates/core/deploy.sh
$ cargo xtask ci
ci green in 2.3s — fmt, assets, lists, layers, web, clippy, tests, wasm.
```

A Python build helper inside `crates/ui`, a stray JavaScript file in the web app
and a deploy script inside `crates/core`, and the gate that exists to police the
tree said nothing about any of them.

Fixed in the same gate, because that is where the walk already was:

- `FOREIGN_SOURCE_EXTENSIONS` — twenty-six extensions that mean "code, in
  another language".
- `FOREIGN_SOURCE_EXCEPTIONS` — the five files above, each with its reason. A row
  without a reason is a permission slip that looks like a decision, so
  `every_declared_exception_is_a_real_file_with_a_reason` requires both a
  non-empty reason and a file that is actually there.
- `walk` takes a predicate, so one traversal serves both rules.

One detail worth recording, because it is the kind of thing that makes a rule
quietly off on one platform: `walk` yields whatever `strip_prefix` gives, which on
Windows is backslash-separated. Comparing that against a table of `/` paths would
have made **every exception a false negative on Windows** — and CI runs
`windows-latest`. `relative_name` normalises.

`AGENTS.md` now says what is true: the rule is checked, the list has five
entries, and it was never the two the file claimed.

## 7. Ninety citations, none of them checked — fixed

For a project whose premise is that agents read prose and act on it, the prose
is the product. So: every `file.rs:12` in every document and every skill, and
whether any of them still points where it claims.

**90 citations. 5 provably wrong, and nothing had ever looked.**

| Where | Cites | Actually points at |
|---|---|---|
| `skills/craftkit-checking/SKILL.md` | `xtask/src/wasm.rs:41-48` | the import block, in a paragraph about the skip |
| `skills/craftkit-checking/SKILL.md` | `xtask/src/web.rs:163-167` | a `fallback` check, in a paragraph about `node` being missing |
| `skills/craftkit-checking/SKILL.md` | `wasm.rs:58` | a lone `}` |
| `skills/craftkit-checking/SKILL.md` | `apps/craftkit/src/main.rs:187` | blank lines |
| `skills/craftkit-editing/SKILL.md` | `lists.rs:492` | blank lines |
| `docs/recipes/change-the-file-format.md` | `crates/io/src/format.rs:213` | a closing brace, cited for `reseat_id_counter` |
| `docs/recipes/add-a-control-channel-method.md` | `crates/control/src/server.rs:523` | a closing brace |

**Part of that drift was caused by this audit.** Every one of those files was
edited in the course of the last four rounds, which moved the lines below the
ones cited. A citation is a claim about a file, and the audit made six of them
false without noticing. That is the honest version of the finding: this is not a
mistake somebody made once, it is the normal decay rate of an unchecked citation.

The checking skill was worse than stale. It described the `assets` gate without
mentioning the Rust-only rule added in §6, quoted gate output that no longer
matched, cited `xtask/src/ci.rs:109` for a phase list that had moved to a
`PHASES` constant, and — the worst of it — **never mentioned
`scripts/mutation-test-gates.sh` at all**. Zero mentions in both skills. The one
workflow `CONTRIBUTING.md` calls its first rule, and the one CI now runs as a
job, was invisible to the agents the skills exist to serve.

Fixed in three parts:

1. **The seven wrong citations now name symbols** — `wasm::run`, `web::run`,
   `reseat_id_counter`, `a_deny_in_a_different_order_is_not_the_house_one`.
   A symbol still means something next year; a line number does not.
2. **`cargo xtask lists` checks them**, via `citation_drift`. It reports a
   citation whose line is blank, whose line range is past the end of the file, or
   whose file is not in the tree.
3. **The checking skill documents the instrument**, including that
   `--self-test` is *expected to exit non-zero*.

**What the check deliberately does not do.** It cannot tell that
`crates/engine/src/session.rs:229` should be about `catch_unwind` and now is
about something else, because the line is real code either way. "What the
citation should have said" is a judgement about prose and no tool can make it. So
the gate covers the mechanically detectable half — a citation pointing at
nothing — and `KNOWN-GAPS.md` says plainly that the other half is a reviewer's
job.

Writing that scanner found three bugs in itself, all of the same kind: `.rs:` is
four bytes so digits start at `dot + 4`, not `dot + 3`; the name has to be
sliced to `dot + 3` to include the extension; and the range separator has to be
stepped over, not merely detected, or `read_number` is looking at a `-`. It is
hand-written for the reason `args.rs` is: the pattern is small, and this build
tool has no dependency that is not already in the tree.

## 8. Three of six easings had no answer anyone checked — fixed

Every other finding in this document is one of two things: a check that cannot
fail, or prose that no longer matches the code. This is neither. It is a
**computation nothing checked the answer to**, and it was found by asking a
question the other eleven findings never asked: *what does the engine actually
compute, and who knows whether it is right?*

The timeline interpolates keyframes through an easing curve. Six easings are
supported — `hold`, `linear`, `easeInOut`, `easeIn` and `easeOut`, plus
aliases. Two of them are a cubic bezier solved by bisection, which is the most
numerically involved code in the project and the kind that can be subtly wrong
without ever being obviously broken: an iteration count that is one short, a
solver that gives up on a non-monotonic curve, an endpoint that drifts.

So: replace `easeInOut` and both bezier presets with a linear ramp, and count the
failures across the whole workspace.

```console
$ cargo test --workspace --no-fail-fast
passed: 542  failed: 1
```

**One failure, and it was not about the thing that was broken.** It came from
`hold_does_not_interpolate` — the only easing with a numeric test. `easeInOut`,
`easeIn` and `easeOut` could return linear values forever and the suite would
stay green.

Only `linear` was properly anchored, and it is anchored well: `a_keyframe_sorts
_itself_and_samples_in_between` asserts 0, 5 and 10 at the quarter, middle and
end of a 0→10 ramp, which is the correct answer by hand. That test set the
standard the other five were not held to.

Fixed with four tests carrying hand-derived values, so the oracle never calls
its own subject:

- `ease_in_out_is_smoothstep_and_not_linear` — `t²(3−2t)` gives 0.15625, 0.5 and
  0.84375 at a quarter, the middle and three quarters.
- `ease_in_out_is_symmetric_about_the_middle` — the property, independent of the
  constants.
- `the_two_bezier_presets_solve_their_own_curve` — values obtained by solving
  `x(t) = x` and reading `y(t)` from the *definition* of each preset:
  `easeIn` 0.093465 / 0.315357 / 0.621862, `easeOut` 0.386805 / 0.686461 /
  0.897053, plus the shape check that `easeIn` is always below linear and
  `easeOut` always above it.
- `every_easing_pins_both_ends_and_stays_inside_the_unit_range` — endpoints,
  range and monotonicity for all six, which is what a broken *solver* breaks
  before a broken *curve* does.

The same mutation now fails two tests instead of one, and the script keeps it
that way.

Worth recording honestly: the endpoint test was wrong on its first run, because
it assumed `hold` ends at 1. `hold` holds the first key's value, so it ends at
0 — which is what the word means. The test was asserting the wrong thing
beautifully, and the only reason it was caught is that it was written against
the implementation's behaviour rather than its documentation.

**What this opens up.** This is the first finding here about *answers* rather
than *checks*. It also produced a prediction, and §9 records what happened when
that prediction was tested: this section guessed that the evaluator's semantics
had no independent oracle either, and it was wrong — every rule is mutation
covered. It also guessed wrong about the shape of the thing, expecting a graph
with fan-in and cycle detection, when evaluation is a sequential fold over the
node list. Coverage of numeric code says it ran, not that it was right, so the
question is worth asking of every computation in the tree.

## 9. The evaluator's rules were anchored, but by the wrong tests — fixed

§8 ended by pointing at this as the next thing to check: the evaluator's graph
topology is well covered and, by the standard applied to the easings, might have
no independent oracle either.

**It does.** Three semantic mutations, three catches:

| Mutation | Caught by |
|---|---|
| A `Source` multiplies instead of setting the level | `reordering_nodes_is_not_served_from_the_cache` |
| Muting a node stops bypassing it | `a_muted_node_passes_the_level_through_untouched` |
| The chain starts at 0.5 instead of unity | `removing_a_node_does_not_reuse_a_tail_that_moved` |

The evaluator is sound, and that is the finding — a verified negative, which is
worth recording precisely because §8 predicted otherwise and was wrong.

But look at which tests caught them. Two of the three are named for the **cache**,
not for the rule they happen to assert. The second cache test even carries the
comment *"A source sets the level, so the plan's output is the last node's"*
inside a test about fingerprint invalidation. So:

- A source setting rather than multiplying was one careless edit to an unrelated
  test away from having no coverage at all.
- Nothing in the test names tells a reader that either rule is anchored.

An oracle you cannot see is one you will not protect. Fixed by giving each rule a
test named after it — `a_source_sets_the_level_rather_than_multiplying_it` and
`the_chain_starts_at_unity_so_one_gain_is_one_gain` — and by putting both mutations
in the script. Each now fails a test whose name says what it protects.

`hold` and mute were already anchored by correctly-named tests, so they were left
alone.

**The method, since it is the reusable part.** Asking "what does this compute, and
who knows whether it is right?" is a different question from "can this check
fail?", and it takes thirty seconds per rule to answer by mutation. It found §8's
three unanchored easings, and here it cleared five rules and made two of the
remaining oracles legible. Negative results are worth running on purpose: the
rules that turn out to be fine are now known to be fine rather than merely
assumed to be.

## 10. Two doc comments described a saturation rule that never existed — fixed

The kernel has two ways to do arithmetic, and both have a doc comment describing
a saturation rule they share:

- `apply_gain` — `level * gain`, **clamped** at `MAX_MAGNITUDE`. This is what the
  plan evaluator calls, per node, per frame.
- `evaluate_chain` — a whole-document walk with offsets and a trace. Its comment
  said *"Saturating arithmetic throughout, so a long chain of large gains
  reaches a limit rather than an infinity."*

`apply_gain`'s comment said the rule *"is identical to the chain's, and lives here
so there is exactly one copy of it."*

**Neither is true.** `evaluate_chain` does not saturate; it stops and returns an
error naming the node where the value went non-finite. And that is not an
oversight — it is asserted, twenty lines below the comment, by
`overflow_stops_the_walk_with_a_named_node`, which feeds two `1e300` gains and
requires `expect_err`. Two doc comments in one file describing a single rule,
contradicted by a test in the same file, and nothing had ever noticed.

This is the whole audit in miniature: a claim, written down, that nothing checks.
It is also the closest thing to a contradiction anyone can construct — a comment
that disagrees with the test sitting beneath it is worse than no comment, because
it is *believed*.

The behaviour is fine and deliberate. The two paths want different things:

- `apply_gain` runs inside drawing. A runaway gain must not throw away a frame,
  and "sounding wrong" is the intended outcome of a hundred `gain` parameters.
- `evaluate_chain` is a walk a person asked for as a unit. An infinity there is
  information, and naming the node that produced it is more useful than a clamp.

Fixed by correcting both comments to say what actually happens and why the two
differ, and by adding `the_two_arithmetic_paths_disagree_on_overflow_on_purpose`
— which asserts both behaviours side by side, so that *making them the same*
becomes a decision somebody writes down rather than a difference that drifts into
an accident.

**Context, not a defect:** `evaluate_chain` has no caller anywhere in the
workspace. It is a public, tested extension point — the kernel is the seam where
an untrusted node implementation would be plugged in, and this is the walk such
an implementation would replace — but today only its own tests run it. Worth
knowing before someone reads it as the engine's own path; it is not.

## 11. The browser page was a 404, and every gate was green — fixed

This section said *"nothing here replaces opening the page."* A browser was
available, so it was opened. **The page did not work, and no compile check in the
project could tell.**

**The defect.** `index.html` imports `./dist/craftkit_web.js`. `wasm-bindgen`
emits `<whatever name it is handed>.js`, and left to itself it uses the crate's
— dashes and all. So it wrote `craftkit-web.js`, and every load of the deployed
page 404'd on its own JavaScript and rendered the "craftkit did not start"
fallback.

Measured, served and loaded:

```console
$ curl -o /dev/null -w '%{http_code}\n' http://127.0.0.1:8099/dist/craftkit_web.js
404
$ google-chrome --headless --dump-dom http://127.0.0.1:8099/index.html
  <div id="fallback" role="alert" data-shown="true">
    <h1>craftkit did not start</h1>
```

**Three things made it invisible, and each is a separate gap:**

1. **The bundling step did not exist.** `cargo xtask wasm` only *checked* that
   the crates compile for the target. Nothing in the repository turned a `.wasm`
   into the `dist/` the page imports — so a developer following the instructions
   could not produce a working page at all.
2. **`index.html` told them to run `cargo xtask wasm` "to run this."** A
   confident instruction to a command that does not produce the thing.
3. **The `web` gate asserted a falsehood.** Its error message read *"craftkit-web
   produces `craftkit_web.js` (the dashes become underscores)"*. `wasm-bindgen`
   does not do that. The gate checked that the *string* appeared in `index.html`
   and called that the check, so the page and the bundler could disagree
   indefinitely.

The second half of this entry is still true and still worth reading: a lower
layer can reach for `std::fs` or `std::env`, pass `cargo xtask wasm`, and fail in
a browser, because those calls *compile* for `wasm32-unknown-unknown`. That limit
is unchanged. What changed is that something now produces the bundle and
something now opens the page.

**Fixed, all four ways:**

- `cargo xtask bundle` exists: it builds the release `wasm`, runs `wasm-bindgen`
  with `--out-name` set to one constant, and prints the files it wrote. When the
  CLI is missing or its version does not match the `wasm-bindgen` crate, it
  prints a **skip with a reason** — never a pass.
- `wasm::BUNDLE_NAME` is the single name. `web::bundle_path()` derives the page's
  import path from it, and `cargo xtask web` checks the page against the derived
  path, so a rename on either side is a failing gate rather than a 404.
- `index.html` says `cargo xtask bundle`, and says why `wasm` is not it.
- CI installs a **version-matched** `wasm-bindgen-cli`, produces the bundle, and
  then requires that the file the page imports is one that job just produced.

Verified after the fix, in the same browser that had shown the failure — the
fallback is gone, the canvas is laid out at 1180×633, both bundle files are
fetched with 200, and the shell renders: the sample's Level/Taper/Out in the
Nodes panel, the playback and history controls, the stash buttons, and "opened
the sample plan; nothing was saved here yet".

**The instrument caught my own fix being incomplete.** The first version of this
repair kept a second hardcoded copy of the name in `web.rs` and had the check
read *that* one, so renaming `BUNDLE_NAME` alone changed nothing. The new mutation
— *"the bundler is told to emit a name the page does not import"* — reported
MISSED on its first run, which is the instrument working exactly as intended.

And fixing that exposed a bug in the instrument itself: `restore()` used
`cp -a`, which preserves the snapshot's timestamp, so cargo could not see that a
source file had changed and kept the **mutated binary**. A gate after a mutation
run was reporting on a build that was no longer on disk. `restore()` now touches
what it restores. That one produced a failure no source in the tree could
explain, which is how it was found.

## 12. The crash-report pipeline had no producer — fixed

`craftkit-engine::crash` was fully implemented and tested: `record`, `list`,
`show`, `replay`, atomic writes, a design snapshot, a replayable step list. The
CLI reads the folder. The control channel exposes `diagnostics.crash_reports`.

**Nothing in any binary ever wrote to it.** There was no `std::panic::set_hook`
anywhere in the tree, and no caller of `crash::record` outside its own tests —
while `apps/craftkit/src/main.rs` claimed in its own module docs that it "adds
wiring, a panic hook and a few flags". So the documented recovery workflow —
"when someone says the app crashed, look there first" — could not happen.

Fixed: the desktop app installs a hook that records a report before the process
dies, using `try_lock` (a hook that waits for a lock held by a dead thread is a
hang that looks like a freeze) and taking the default hook so the panic still
prints. It is proven by `a_panic_hook_writes_a_report_a_person_can_act_on`,
which panics a real thread and asserts the report names the message, the command
that was running, a non-empty replay list, and the document as it was.

Worth knowing: a panic inside a *command* still never reaches this hook, because
`Session::execute` catches it first. That is the design — the common case is a
recovered error, and the hook is for the panics that escape anyway.

## 13. A skip that could be mistaken for a pass — fixed

`cargo xtask doctor` reports `blocking` / `degraded` / `info`. A `degraded`
finding is **not** a gate failure, and the standing example is `wasm-bindgen`:
the CLI here is 0.2.89 against a 0.2.129 crate, so the `.wasm` builds and
`cargo xtask wasm` is green, but `wasm-bindgen` would refuse to produce a bundle.

The risk is not the finding; it is that somebody reads "cargo xtask wasm passed"
and concludes the browser build works. Measured on this machine, before the fix:

```console
$ cargo xtask wasm
   ok: every library crate and craftkit-web build for wasm32-unknown-unknown
$ cargo xtask bundle
   skipped: the browser bundle — the CLI is 0.2.89 but the wasm-bindgen crate
   is 0.2.129; wasm-bindgen refuses a bundle whose schema versions differ
```

`wasm` said nothing. The two commands disagreed and neither told the reader
which of them decided the outcome.

**What was fixed.** `wasm::run` now asks the same question `bundle` asks and
prints the answer when the browser build is out of reach:

```
── wasm
   ok: every library crate and craftkit-web build for wasm32-unknown-unknown
   note: that is a compile check. It writes nothing, and the browser bundle
         cannot be built here: the CLI is 0.2.89 but the wasm-bindgen crate is
         0.2.129; wasm-bindgen refuses a bundle whose schema versions differ
         `cargo xtask wasm` is green; the page's JavaScript does not exist yet.
```

A note and not a skip, deliberately: the compile check *did* run and *did* pass,
so `Outcome::Skipped` would be its own kind of lie and would put a passing phase
in the closing line's "skipped, not passed" list. It is also silent when the
bundler works, so a machine that can build the page is not told it cannot — three
tests pin that symmetry, including the quiet case.

**The obvious fix was measured and rejected.** Adding `bundle` to `ci`'s phases
would make the gate actually attempt the browser build, which is the structural
answer and the one this section wanted. It costs, on a cold cache:

```console
$ time cargo xtask bundle        # with a correctly matched CLI
real  2,644s
```

Forty-four minutes inside a gate that currently takes seconds. That is not a
price worth paying for a sentence that can be printed instead, so the structural
fix was declined and recorded rather than quietly adopted on principle. The cost
is stated because the next person will think of the same idea.

**What is still not enforced.** The mutation covers `post_build_note`, which is
pure so that it can be tested without a `wasm-bindgen` installed — a test that
depended on the machine's CLI would pass in one checkout and fail in another,
reporting the machine instead of the tree. Deleting the *call* in `run` would
not be caught by it. That is the same honest limitation §21 has on the other
side of the layer boundary, and it is not dressed up as covered.

## 14. Doc-tests: effectively none — fixed for the two that matter

This started as *"One doc-test in the whole workspace."* Every crate had heavy
`//!` and `///` prose and no runnable example, so nothing could rot — and nothing
demonstrated that the documented shapes were the real ones.

Two places mattered, and both are now assertions rather than prose.

**The `.craftkit` format** (`craftkit_io::save_text` and `load_text`). Three
doc-tests pin exactly what `docs/design/file-format.md` claims, checked against
what the serialiser actually emits rather than against the document:

- `format` is the string `"craftkit/1"`, not a number.
- A value is adjacently tagged — `{"t": "num", "v": 2.0}`.
- A property node is externally tagged, so the variant name is the key: the leaf
  is under `Prop`, not inlined.
- A hint is externally tagged with a capitalised name — `{"Slider": {…}}`, not
  `{"slider": {…}}`.

And two that check the *reading* side, which is where a hand-written file actually
goes wrong:

- A complete file typed by hand parses. This is the case the document says was
  correct only because it was written after probing `craftkit-cli eval` and
  reading four successive error messages.
- A bare `2.0` where a value belongs is refused with a message containing
  **"expected adjacently tagged enum Value"** — the exact sentence the document
  quotes. That assertion is what makes the quoted error a fact rather than a
  recollection.

**The command schemas** (`craftkit_engine::catalogue`). The catalogue is what the
MCP tool list and the playbook are generated from, so its contract now lives in a
doc-test: every command has a dotted lower-case id, a non-empty label, a doc
string and an area, and every parameter has a name, a type, a `required` flag and
help text. Verified not vacuous — blanking one label fails with
`` `doc.rename` has no label ``.

**One small thing found on the way.** `PropHint` was public but not re-exported
from `craftkit-model`'s root alongside `Property`, `PropNode` and `PropGroup`, so
a library consumer had to reach into `craftkit_model::property` to name the type.
It is exported now. That is the shape of the whole audit: a thing that is
*available* but not reachable the way everything around it is.

The rest of the prose is still unexampled, and that is a deliberate stopping
point rather than an oversight: a doc-test is worth writing where a reader would
otherwise have to trust a shape, and the format and the command registry are the
only two places that happens.

## 15. Coverage is a diagnostic, not a gate — and that is now a decision

Measured three times as the audit went, on a tree that changed under it:

```text
                       first    second   now
TOTAL  lines            21356     21533     22372
       missed            3106      2911      2960
       line cover       85.46%    86.48%    86.77%
       region cover     86.09%    87.09%    87.24%
       function cover   84.32%    85.47%    85.83%
```

**The shape of the misses is the finding. The total never was.** 85% sounds fine
and means nothing on its own; the same 85% hid a CI run that never executed its
own test suite (§2). What mattered was *which* files sat at the bottom, and the
two that sat lowest were the two that mattered:

| File | First | Now |
|---|---|---|
| `xtask/src/ci.rs` — the gate that runs the gates | 41.03% | 50.00% |
| `apps/craftkit-cli/src/cli.rs` — the headless surface | 37.97% | 78.05% |
| `xtask/src/lib.rs` — command dispatch | 64.80% | 63.78% |
| `xtask/src/doctor.rs` — the machine report | 71.50% | 71.50% |
| `crates/ui/src/shell.rs` — widget layout | 81.64% | 81.85% |
| `crates/model/src/lib.rs` | 100.00% | 100.00% |

**A coverage gate was recommended here, and is deliberately not being added.**
That recommendation was wrong, and the work that followed is what showed it:

- The rule it proposed — *the gate that runs the gates and the CLI an agent
  drives are both covered* — is **already enforced, and more strongly**, by
  `scripts/mutation-test-gates.sh`. A mutation proves a line is reached *and*
  that reaching it changes the outcome. A coverage percentage proves only the
  first. Two mutations sit in `ci.rs` and seven in `cli.rs`.
- `ci.rs`'s remaining 50% **cannot** be raised by a test. `fmt`, `clippy`,
  `tests` and `wasm` shell out to cargo; a unit test that called them would run
  the gate from inside the gate. Any threshold low enough to pass there is low
  enough to mean nothing, and any threshold high enough to be worth having
  cannot be met.

So the honest rule is not a number. It is the one already in
`CONTRIBUTING.md`: *when you add or change a gate, break its rule and require the
gate to notice.* Coverage is what you look at to decide **where** to point that
rule, and this table is what it was used for — it found the CLI dispatcher and
`ci.rs`, both of which turned out to be real.

**What the number will never tell you**, even at 100%: that a line *executed* is
not a line that was *asserted about*; that `fn main` entry points read 0.00%;
and that the rasteriser's 97% says nothing about whether a frame looks right.
Those are §16 and §8's business, not a percentage's.

## 16. The software rasteriser is a rasteriser, not a renderer

`craftkit-ui`'s rasteriser walks egui's tessellated shapes and samples the font
atlas, so text rasterises. Its stated limits:

- **Hard edges.** egui's geometry is the only anti-aliasing and there is none
  beyond it, so a frame looks crisper and slightly jaggy beside the window.
- **Gradients at the triangle centroid.** A theme that adds one will band.
- **Only the font atlas is uploaded.** A texture a document loaded would draw as
  a flat rectangle.

It exists so a frame can be produced on a machine with no display and no GPU,
which is what makes `ui.render` and `snapshot` work in CI. It is not a
comparison tool: to know what the window really looks like, open the window.

## 17. `ui.click` reports, it does not act

`ui.click` replays a pointer event into a fresh context and answers with the
name of the topmost widget at that point. It does not activate anything —
`engine.execute` does, and that is undoable, logged and audited.

This is deliberate, but it is a gap against what an agent might expect from the
name. It was also, for a while, a lie in the only place an agent looks: the reply
said ``clicked `toolbar.node.duplicate` `` for a click that activates nothing.
§32 has the measurement and the fix.

The second half of this section used to say the UI was "the only place with no
end-to-end test on a real display", and that this was because CI has no display.
**Having a display showed that this was not a testing gap but a real defect:**
the window never repainted after a command, so §17's `ui.click` was not the only
thing an agent could be misled by. See §21, and `scripts/live-window-check.sh`
for the end-to-end check that now exists and what it can and cannot prove from
CI.

## 18. Scope boundaries, not defects

Stated so nobody discovers them as surprises:

- **No `save`/`open` commands.** The engine registry has 21 commands and none of
  them touch the filesystem; the desktop app calls `craftkit-io` directly, the
  way the CLI does. Everything is a command applies to *behaviour*, and file I/O
  is the host's business. The cost is that an agent cannot persist a plan through
  the command registry — it must use the web stash or the CLI.
- **No undo for `prop.reset` and friends beyond what `undoable` declares.** The
  distinction is documented in `AGENTS.md` and enforced by
  `every_read_only_command_leaves_the_document_alone`.
- **The web build's stash is one plan.** `STASH_KEY` is a single fixed slot.
  Losing it loses the plan, and the panel says so in the open.
- **The software rasteriser's `Frame` borrows the session.** Rendering cannot
  run concurrently with a command in the same session, which is why `ui.render`
  takes the session lock for the duration.
- **`Cli::run`'s `mcp` arm is no longer the untested one.** It used to be, and
  the stated reason — that it blocks on stdio — was not a reason. It is driven
  end to end now, over a real pipe, and doing so found a defect on the first
  run. See §24.

## 19. Not a git repository

There is no history, no remote and no branch. Nothing was committed, and
`AGENTS.md` tells agents not to push. `.gitignore` is written and ready.

## 20. Licence texts are not vendored

`LICENSE` names MIT OR Apache-2.0 and links the canonical texts rather than
transcribing them: a paraphrase of a licence is not a licence, and a
transcription that differs in one word is worse than none. The copying
instructions in `README.md` say to add the two files.

## 21. The window did not follow the document — fixed

§17 said the UI was "the only place with no end-to-end test on a real display".
That was true, and having a display turned it into the largest defect found in
this audit after §11.

**The defect.** A command that arrives over the control channel runs on a socket
thread. It mutates the session — which is behind a `Mutex` the frame loop never
sees — and nothing tells egui a frame is due. eframe paints on input events, and
a command over a socket is not one. So the window kept showing the frame it had
already painted. Measured on a real X display, after two `node.add` commands:

```console
$ python3 drive.py $PORT '[{"id":3,"method":"engine.execute",
                           "params":{"command":"session.stats","params":{}}}]'
id=3 ok=True 2 command(s), 0 failure(s)
   data: {"commands":2,"failures":0,"nodes":5,"undo_depth":2}

$ DISPLAY=:1 import -window 0x4600005 /tmp/after.png   # the window's own pixels
```

`nodes: 5`. The window showed **three** nodes, "3 nodes saved". Nothing errored,
no command failed, and the reply said `ok`. The engine had moved and the window
was showing it a picture of the past. That is the worst shape a defect can have:
it is invisible to every check in the project and actively misleading to the one
consumer the app is built for — an agent that drives the app and looks at it.

**The fix.** `craftkit-control` gained an extension point beside the `ui.*` one:

- `ServerOptions::with_after(handler)` — called after a request that can change
  the document, *after* the reply is sent and *after* the session lock is
  released, so a handler is free to read the document it just changed.
- `apps/craftkit` publishes its `eframe::egui::Context` into a slot on the first
  frame and passes `request_repaint` as that handler. The channel starts before
  the window does, so the slot is empty for the first few frames and for the
  whole life of a headless process; an empty slot is a no-op, and there is a
  test for exactly that.

It fires for `engine.execute` and `ui.*` only. `engine.describe` in a loop must
not repaint a window, and two tests pin that.

**There is no unit test for the call itself, and there cannot be one.** egui
offers no observable that separates "a repaint was requested" from "a repaint is
pending" on a context with no window behind it. This was measured, not assumed:
`has_requested_repaint` is already `true` on a headless context after the first
pass (`outstanding` is re-armed at the end of every pass), and the wake-up
callback in `set_request_repaint_callback` fires only when the delay is
*lowered*, which requires the idle state only a real integration loop reaches.
Both were tried; both were measured to be true before and after a request. A
test asserting such a flag would have looked like evidence and proved nothing,
which is the failure mode §1 is about.

So the check measures the thing that matters instead:
**`scripts/live-window-check.sh`**. It runs the real binary on a real display,
drives it over the control channel, captures the window's own pixels before and
after, and requires the two to differ by more than 1000 pixels — plus, as an
independent oracle, that the engine's own node count moved. Exit 77 for skip,
never 0: CI has no display, and a check that quietly reported success there would
be the same lie as the bug.

Proven to fail on the unfixed tree, and to pass on the fixed one:

| Tree | Engine says | Differing pixels | Exit |
|---|---|---|---|
| `.with_after(…)` removed | 7 nodes | **0** | **1** |
| as committed | 7 nodes | **11290** | **0** |

The 0 is the whole point. Anything that reports success while the window is
showing stale pixels is the defect it was written to catch.

**What CI can and cannot run.** The policy half — *which requests announce
themselves* — is a normal test in `craftkit-control` and is mutation-tested, so
it runs everywhere. The mechanism half needs a display and is a local check,
documented as one. That split is deliberate: the part CI can see is enforced in
CI, and the part it cannot is not dressed up as if it were.

**A hole in the fix, found by reading the fix.** The first version announced the
change *after* sending the reply:

```rust
let response = answer(&request, session.as_ref(), ui);
if send(&mut writer, &response).is_err() {
    break;                      // <-- returns without ever announcing
}
if matches!(method, Method::Execute | Method::Ui) && let Some(h) = after { h() }
```

`answer` has already changed the document by then. Whether the reply reaches the
client is a fact about the socket and says nothing about the document, so a
client that disconnected mid-command left the window showing the frame from
before it — silently and permanently, because nothing had gone wrong anywhere.
The announcement now happens between the two.

This ordering is **not** mutation-tested, and it cannot be: making a send fail
deterministically needs a socket that is reliably dead on the first write, and
Linux gives you a buffered write plus an RST instead. It was found by reading the
five lines the previous paragraph was about, which is the same method as §22 and
the same reason it is worth writing down.

**One completeness check.** A `Mutex<Session>` is shared in exactly two places
in this tree: the desktop app, where a window and the control channel share it,
and `craftkit-mcp`, which runs in its own process with no window. So §21's fix
covers every path that can move a session a window is drawing, and the sweep
turned up no third case to miss.

---

## 22. Two copies of "can this machine build the page?", and they disagreed

Found while fixing §13, by asking the same question of the next piece of code:
*what does this compute, and is anyone checking the answer?*

`cargo xtask doctor` and `cargo xtask bundle` each decided on their own whether
`wasm-bindgen` could produce a bundle. Same inputs, same tool, **two
implementations of one rule** — which is how §11's `BUNDLE_NAME` bug happened,
and the instrument caught that one too.

They did not agree. Both read the crate version out of `Cargo.lock` with
`wasm_bindgen_crate_version`, which returns `Option`. When it came back `None`,
the two halves did opposite things:

- `wasm::bundler_blocker` used `?` on the `Option`, so `None` meant **"no
  blocker"** — and `cargo xtask bundle` would carry on and run a CLI it had
  checked against nothing.
- `doctor` matched `(None, _)` and filed an **`info`** finding, the lowest
  severity there is.

So the one state in which the machine knows nothing was the state it reported as
knowing nothing, most quietly. `Cargo.lock` is hand-parsed for the
`name = "wasm-bindgen"` entry; the day cargo reformats that file, or the lockfile
is unreadable, the failure mode is a bundle built against an unchecked tool.

There was also a literal doing nothing at all. `doctor` compared the CLI against
a hardcoded `"0.2.0"` and then discarded the answer with a `_` in the pattern, so
the number looked like a minimum-version policy and did nothing whatsoever.

**The fix.** One function, `doctor::bundler_status(cli, crate_version)`, answers
`Ready` or `Blocked(reason)`, and both callers use it. The rule it now states is
the one this document keeps arguing for:

> "I could not read the version I need to compare against" is not "they match."

An unknown is `Blocked`, in both halves, with a sentence that names the CLI
version the reader has to go and fix. Three tests cover it as a pure function —
no `wasm-bindgen` has to be installed for them to mean anything — and the
mutation puts the unsafe reading back to prove they bite.

**Recorded as a method note rather than a triumph.** This defect could not be
found by running anything: on this machine `Cargo.lock` parses, the CLI is
0.2.89, and every command behaved correctly. It was found by reading two
functions that should have been one and asking why both existed. That is the
question that found §8's unanchored easings and §9's misnamed tests too, and it
is the one that no mutation and no failing run will ever ask on your behalf.

## 23. The instrument's whitelist let a mutation corrupt the tree, silently

Found because §22's own mutation did this to me, four minutes after I added it.

`scripts/mutation-test-gates.sh` protects the tree three ways: it takes a
`FILES` snapshot up front, `restore`s the whole list after every case, and ends
by `fingerprint`ing the list to prove it changed nothing. All three walk the
same array.

I added a mutation against `xtask/src/doctor.rs` and **did not add that file to
`FILES`.** So:

- `snapshot()` never copied it — nothing to restore *from*.
- `restore()`'s `cp -a "$SNAPSHOT/$f" "$f" 2>/dev/null` failed, silently, by
  design.
- `fingerprint()` never looked at it.

The mutation ran, the gate caught it, the run printed **"Tree unchanged"** and
**exited 0** — while the tree kept the broken source. The header of that script
claims *"the whole mutable surface is restored between cases"*, and for that one
file that was false.

What noticed was the next ordinary `cargo xtask ci`: a test asserting the §22
fix failed with a message about the old behaviour being back. Not the script
that made the mess. That is the worst way for this to be caught, because the
script's whole claim is that it is the thing that notices.

**The fix is not just the missing line.** Adding `xtask/src/doctor.rs` to `FILES`
repairs this instance; the next person adds a mutation against a file not on the
list and gets the same silent corruption. So `swap` and `append` now call
`guarded`, which refuses any target outside `FILES` and says why:

```
MUTATION TARGET IS NOT SNAPSHOTTED
  README.md

  It is not in FILES, so snapshot() never copied it, restore() cannot put it
  back and fingerprint() never looks at it. Add it to FILES first.
```

Verified by adding a probe mutation against `README.md` and running the suite:
exit 1, the refusal printed, and `README.md` was not touched.

**Why this one matters more than the rest of §1's list.** The other gaps were
gates that could not fail. This one is the *instrument that exists to prove gates
can fail*, and it reported a clean tree it had not looked at. A whitelist that
is consulted by the writers and not by the readers is a list that can fall behind
the thing it constrains, and nothing in a passing run would ever say so.

## 24. `shutdown` said ok and kept serving

§18 listed this as the last untested surface: *"`Cli::run`'s `mcp` arm is the
one subcommand no test executes."* The stated reason was that it blocks on
stdio. That is not a reason a test cannot drive it — a pipe is exactly what
tests do — and driving it found a real defect immediately.

**The defect.** `serve` opened with *"Serve MCP over stdio until the input ends
or `shutdown` arrives."* The loop ended when the input ended. `shutdown`
dispatched to `Response::result(id, json!({}))` and carried on. Measured,
holding stdin open so that only `shutdown` could possibly have ended it:

```console
$ { echo '{"jsonrpc":"2.0","id":1,"method":"shutdown"}'
    echo '{"jsonrpc":"2.0","id":2,"method":"ping"}'
    echo '{"jsonrpc":"2.0","id":3,"method":"ping"}'
    sleep 4; } | craftkit-cli mcp
{"jsonrpc":"2.0","id":1,"result":{}}
{"jsonrpc":"2.0","id":2,"result":{}}
{"jsonrpc":"2.0","id":3,"result":{}}
```

The server answered a stop request with success and then served two more
requests. That is the failure mode `crates/engine`'s own `key.remove` doc warns
about — *"a command that reports success without changing anything is how an
agent concludes it has done something it has not"* — in the one place where an
agent decides whether the process is still there.

The two servers also disagreed: the control channel's `shutdown` really does
break its accept loop. Two servers, one name, two meanings.

**The fix.** `handle_line` could not report "and stop", because it returns only
a reply. So the parse now produces an `Answer { reply, stop }`, `handle_line` is
a thin wrapper over it (so every existing protocol test keeps testing exactly
one function), and `serve` breaks on `stop` — **after** sending the
acknowledgement, because the client that asked to stop still deserves to hear
that it was heard.

**The stop has to travel separately from the reply**, and that is not a detail:
a `shutdown` sent as a *notification* gets no reply by design, so if the signal
were folded into the reply it would be lost exactly when a client is shutting
down cleanly. A test pins that, and a mutation puts it back to prove the test
notices.

**What is now covered.** `apps/craftkit-cli/tests/mcp_stdio.rs` spawns the real
binary and drives a real pipe: a tool call that changes the document and is
confirmed by an independent `status` call, refusal codes kept apart from
protocol codes, malformed input that does not kill the server, and `shutdown`
ending it. Unlike the window in §21, **these run in CI** — stdio needs no
display. §18's last untested surface is closed.

**A verified negative, and one thing left alone.** Every other walker in the tree
— `assets::walk`, `assets::collect`, `collect_rust` — already recursed. The
shallow one was `prose_files`, and only `prose_files`.

Left in place deliberately: five documents assert "21 commands", and 21 is
correct today. That is a rot risk, not a defect, and the project's own rule —
*"don't hardcode counts or timings in prose; they rot; point at the command"* —
is what argues against them. There is no gate for it, and adding one would mean
parsing digits out of sentences, which is the substring-checking this document
has spent twenty-six sections objecting to. Recorded here so that adding
command 22 is a moment somebody notices five sentences, rather than a moment
five sentences quietly become wrong.

## 25. §11's own falsehood survived in the error box

§11 fixed the browser page: nothing produced the bundle, `index.html` named a
file `wasm-bindgen` never wrote, and a gate's error message asserted something
false. The authoring instructions in `index.html` were corrected to say
`cargo xtask bundle`.

**One copy of that sentence was not corrected.** Line 102 of the same file —
the `<pre>` inside `#fallback`, the box drawn when the page fails to start —
still read:

```
1. The bundle is missing.        Run:  cargo xtask wasm
```

Served with no `dist/` so the import must fail, and read in a real browser:

```console
$ google-chrome --headless --dump-dom http://127.0.0.1:8123/index.html
  <pre>1. The bundle is missing.        Run:  cargo xtask wasm
```

`cargo xtask wasm` only checks that the crates compile for the browser. It
writes nothing. So the one instruction a reader sees **when the page is
already broken** told them to run the command that cannot fix it — and, having
just failed, they would run it, see it pass, and conclude the page was fine.

This is §11 arriving one file later. The lesson of §11 was "one constant, not
two spellings", and the fix for it left a second spelling in the same file
where nobody was looking, because the thing being corrected was prose and prose
does not fail to compile.

**The fix, and the rule that keeps it fixed.** The gate now reads the failure
box:

```
• apps/craftkit-web/index.html: the failure box tells the reader to run
  `cargo xtask wasm`, which only checks that the crates compile and writes
  nothing.
   `cargo xtask bundle` is the command that produces the file this page imports.
```

It matches the sentence, not the bare word `wasm` — `wasm` appears in this page
legitimately, in the comment that explains *why* `bundle` is the right command.
A check that matched the word would be the substring gate §1 is about; this one
matches the instruction, and a mutation puts `wasm` back to prove it bites.

**The method note.** This was found by grepping for a command name that §11 had
already established was wrong — not by running anything, because nothing was
broken. `cargo xtask web` was green throughout, exactly as it had been in §11.
The question that found it is the same one that found §22: *which other places
does this same fact appear in?* It is cheap to ask and it does not need a
failure to ask it.

## 26. A gate that read three documents and called itself exhaustive

§25's method note — *which other places does this same fact appear in?* — was
aimed at one string. It was applied to the whole documentation tree, and two
things came back.

**A recipe named a command that does not exist.** `docs/recipes/add-a-command.md`
listed the live `Area::Node` entries in the order you must insert into:

```
node.add, node.duplicate, node.enable, node.kind, node.move, node.mute,
node.remove, node.rename
```

`node.enable` has never been in the registry. The catalogue has 21 commands and
this is not one of them; the nearest is `node.mute`. An agent reading that
sentence learns where a real command lives and invents a real-looking id that
fails on the first call.

**The citation gate had never opened a subdirectory.** This is the larger one.
§7's `citation_drift` is the check whose closing line reads *"every `file.rs:12`
in prose points at something"*. Its walker read `docs/*.md` — **three files** —
and stopped. It never descended into `docs/recipes/`, `docs/design/` or
`docs/patterns/`, which hold **fifteen** documents and **49** line-number
citations:

```rust
} else if path.is_dir()
    && let Ok(dir) = std::fs::read_dir(&path)
{
    for e in dir.flatten() {          // <- one level, and no recursion
```

So a walk that inspected three documents out of eighteen reported, in bold
confidence, that every citation in prose was checked. It did not say otherwise
anywhere. Making it recurse found the drift immediately — **five stale
citations** in `docs/recipes/add-a-control-channel-method.md`, two of them
pointing past the end of a 450-line file:

```
docs/recipes/add-a-control-channel-method.md: cites
  `crates/control/src/server.rs:496`, outside a 450-line file
```

All five were replaced by the symbol already sitting beside them, which is the
rule the gate itself prints: *"Cite the symbol, not the line number."* A sixth,
in `skills/craftkit-editing/SKILL.md`, went stale *during this work* because my
own edit moved `lists.rs` — which is the citation-drift problem happening to me
in real time.

**A second check was added for command ids.** `cargo xtask lists` now refuses a
document that names a `<area>.<name>` the registry has never heard of. It reads
the ids from `crates/engine/src/commands.rs` rather than linking
`craftkit-engine`, because xtask deliberately depends on nothing but `serde`
and a build tool should not have to be the thing it builds.

Three dotted names in the prose are not commands and are allowed by name, with
reasons: `session.state` and `diagnostics.crash_reports` are control-channel
methods, `session.execute` is the Rust method every channel calls.

**The one exemption worth arguing about.** `node.enable` is also the command
`docs/agent-playbook.md` and `docs/recipes/add-a-command.md` work through
*adding* — the gate cannot tell prose that describes a command from prose that
proposes one. So `PROPOSED_IN` exempts that id in those two files **and nowhere
else**. The first attempt exempted the id globally, and the mutation caught the
thickness of that idea immediately: the fabricated id reappeared in the recipe
and the gate said nothing. An exemption scoped to the id would have left the
exact place this defect was found unguarded.

**A mutation that could not be written, and why.** The obvious second case — break
the recursion and require the gate to notice — does not exist. Removing the
recursion genuinely changes what the gate can see, so the gate passing is then
*correct*, and no mutation can honestly assert otherwise. The fabricated-id case
does the job instead: it plants its defect in a subdirectory, so if the walk is
flattened again it stops being caught, the run reports MISSED and exits
non-zero. That is the instrument doing the one thing it is for. Writing the
other case anyway would have been a gate asserting that it is worse than it is.

## 27. The crash pipeline had a tested producer and an untested consumer

§24 established a habit: map the surfaces, then drive them. Six CLI subcommands,
one integration test. So: `commands`, `run`, `eval`, `snapshot` and `crashes`
were all exercised by hand for the first time, and the crash folder came back
empty.

**The gap.** `crates/engine/src/crash.rs` is well covered — reports round-trip
through temporary folders, a corrupt report is refused by name, two reports in
the same second get different ids. The panic hook in `apps/craftkit` has its own
test. All of that is the **producer**.

Nothing covered `craftkit-cli crashes` — the command a person is told to run
first when they say the app crashed, and the one the skills tell an agent to
run. It had never been executed against a real report on any machine, because
the folder only fills when something crashes, and nothing had.

**It was already testable.** `CRAFTKIT_CRASH_DIR` relocates the folder, which
means the whole pipeline can run in CI without writing into anybody's home.
`apps/craftkit-cli/tests/crash_pipeline.rs` now records a report from a session
that has done real work, then drives the real binary through `list`, `show` and
`replay`, asserting the document and the steps come back out and run again in a
fresh session. Two smaller properties are pinned alongside: an empty folder says
*"no crash reports in …"* rather than printing nothing that reads like a
folder that could not be opened, and an unknown id is refused by both `show`
and `replay`.

**The mutation is the interesting one.** It removes the `CRAFTKIT_CRASH_DIR`
override, and it fails with a message that names the real folder the CLI fell
back to:

```
`list` did not name the report:
  "no crash reports in /home/…/.local/share/craftkit/crashes"
```

That test is what makes the override load-bearing rather than merely present —
without it the pipeline cannot be tested at all, and nothing would say so.

**A measurement lesson, recorded because it bit twice in this document already.**
A combined shell pipeline reported "47 tests passed" when the workspace run had
**573**. The suite was green both times; the number was an artefact of reading
a partially-flushed stream through a pipe. Re-running the count on its own gave
573, exit 0, 26 test binaries, 0 failures. A number that changes when you
measure it a second way is a measurement, not a fact.
## 28. The tool that exists to be driven by an agent hid its one input format

Found by doing what §27 describes — driving the CLI by hand — and getting this
on the first attempt:

```console
$ craftkit-cli run script.json
craftkit-cli: script.json is not a script: invalid type: map, expected a string
```

An array of `{"command", "params"}` is the shape an agent would reach for, and
it is refused. The real shape is an **object** with a `commands` array. Which is
fine to refuse — it refuses, by name, with exit 1 — and not fine to have to
discover.

**It was documented.** `apps/craftkit-cli/src/main.rs` carries the shape in its
module documentation, with a worked example, under `## The script format`. So
the claim "it is undocumented" would have been wrong.

The claim that survives is narrower and is the real finding:

- `craftkit-cli --help` does not show it.
- **Every error message ends by printing `USAGE`, and `USAGE` does not show it.**
- `skills/craftkit-checking/SKILL.md:194` tells an agent to run
  `craftkit-cli run script.json` without ever saying what one is.

So the format was written down in the one place in that file a reader of the
tool never reaches. This is a program whose stated purpose is being driven by an
agent, and the first thing that agent has to write is invisible from the
program.

`USAGE` now carries it, and the example is a real one:

```
SCRIPT FORMAT (`run` and `snapshot`):
    {"name": "optional",
     "commands": [{"command": "node.add",
                    "params": {"name": "Box", "kind": "filter"}}]}
    `params` may be omitted and means no parameters. `crashes replay <ID>`
    prints this same shape.
```

Verified the way a user would: piped out of `--help`, stripped of its indent,
written to a file, run.

```console
$ craftkit-cli run pasted.json
1/1 node.add: added filter `Box`
optional — 1 step(s), 1 node(s)
```

**Two copies, then one.** The test was first written against a second constant
holding the example — which is §11's "two spellings" bug, introduced in the same
commit as the fix for a documentation bug about duplicated facts. A test
comparing them failed immediately, on line wrapping, and the constant was
deleted: the test now reads the example out of `USAGE` and parses it, so there
is exactly one source and it is the text that gets printed.

**And the mutation nearly passed.** The first version rewrote the heading to
`SCRIPT FORMAT IS A SECRET`, which does not bite — the test's guard is a
`contains("SCRIPT FORMAT")`, so it matched the mangled text happily. §1's
substring disease, caught the day it was written by checking rather than
assuming. The mutation now removes the heading outright.

## 29. Prose names test symbols, and nothing checks those either

Found by sweeping every backticked identifier in `skills/` against the source —
the same question §25 asked, one level of indirection further. Sixteen
candidates; seven did not match a `fn`, `struct`, `enum`, `const`, `static` or
`mod`, and six of those were correct: `catch_unwind`, `unwrap_used`,
`png_base64` and three MCP tool names.

The seventh was a real inexactness, in three places at once:

```
skills/craftkit-editing/SKILL.md:237   `pending_list_is_sorted` keeps the table honest.
docs/patterns/format-changes.md:32      `pending_list_is_sorted` test keeps the table honest.
crates/io/src/format/pending.rs:43      `pending_list_is_sorted` test keeps the table honest.
```

The test is `the_pending_list_is_sorted`. Two other lines in the *same pattern
document* spell it correctly, so the file had drifted against itself. All three
are fixed.

**Why this is small, and why it is still worth writing down.** A reader who
pastes `pending_list_is_sorted` into a `cargo test` filter gets a green run,
because cargo matches substrings and `the_pending_list_is_sorted` contains it.
A reader who greps for `fn pending_list_is_sorted` gets nothing. So the failure
is quiet in one direction and loud in the other.

**No gate, deliberately.** `cargo xtask lists` checks `file.rs:12` citations and
command ids; extending it to every backticked snake_case name in prose would
have to distinguish `the_pending_list_is_sorted` from `catch_unwind` and from
`craftkit_node_add`, and every rule I could write for that distinction is a
heuristic that would eventually flag something real. A gate that cries wolf is
worse than the absence of one, which is §1 stated the other way round. The
finding is recorded so the next sweep knows the shape of it.

## 30. Every surface, driven by hand — and what that found

Twenty-nine of these sections list defects. This one lists the opposite result,
because the absence of findings is only worth knowing if somebody went looking.

**Method.** Enumerate the surfaces a user or an agent can touch, then drive each
one for real rather than reading the test that claims to cover it. §24's `mcp`
and §27's crash folder both came out of this and both had a defect waiting.

| Surface | How it was driven | Result |
|---|---|---|
| `craftkit-cli commands` | run | correct — 21 ids |
| `craftkit-cli run` | hand-written script | correct |
| `craftkit-cli eval` | the format doc's own example | correct — and the doc's claim ("one node, 12 frames at 24 fps, no diagnostics") is exactly what it prints |
| `craftkit-cli snapshot` | script → PNG | correct; the frame renders the nodes |
| `craftkit-cli crashes` | see §27 | **defect found** |
| `craftkit-cli mcp` | see §24 | **defect found** |
| the desktop window | `--sample`, control channel | **defect found** — §21 |
| the desktop window opening a file | a hand-written `.craftkit`, real display | correct |
| the browser page | headless Chrome, bundle present and absent | correct after §11 and §25 |

**`eval` against hostile files.** Seven malformed inputs — truncated, empty,
not JSON, an array, missing fields — all refused, exit 1, no panic. The version
gate is the interesting one, and it is well made:

| `format` | what it says |
|---|---|
| `craftkit/2` | "this file is format version 2, and this build reads up to 1. Open it with a newer craftkit" |
| `craftkit/0` | "this file is format version 0 or less" |
| `craftkit/banana` | "this file is format version 0 or less" — unparseable is refused, not guessed |
| absent | "this is not a craftkit file: the `format` field is missing" |

That last behaviour is the one the format document stakes its safety on, and it
holds.

**Opening a file on a real display.** A four-node plan written by hand, opened
by `craftkit FILE`, on Xorg with hardware GL: the window shows the plan's name
from the file, all four nodes with their kinds, and the status bar reads
**"4 nodes saved"** — not "unsaved", because a document that was opened rather
than edited is clean. That is a small thing to get right and it is the sort of
thing that quietly stops being true.

**What this sweep did not reach, and cannot.**

- The repaint *mechanism* on the real display — §21's instrument covers the
  behaviour, not the call that causes it.
- The `after` hook's call site in `serve` — §21 records why it has no test.
- ~~Real-browser *interaction*.~~ **Done — §31.** It needed a DevTools-protocol
  client, not more reading, and it found a defect on the first real click.
- `craftkit-web`'s stash (`localStorage`), which §18 lists as a single fixed
  slot and which nothing here exercised.

**The honest summary.** Four defects came out of driving things (§21, §24, §27,
and §11 before them). This pass found none new, and the surfaces it covered now
have an answer either way rather than an assumption. The next useful question is
not "what else is untested" — it is the one at the bottom of this table.

## 31. The browser's save button printed the whole plan, and nothing read it

§30 ended by saying real-browser *interaction* was the thing the sweep had not
reached. It was also the thing most likely to pay, so this is what it paid.

**How.** No websocket library is installed, so the DevTools-protocol client is
about sixty lines: an HTTP upgrade, and a frame codec where the client masks and
the server does not. Chrome is started with `--remote-debugging-port` and the
page is driven with `Input.dispatchMouseEvent`.

**Validating the instrument first, which is the part that mattered.** The first
click on "Save in this browser" did nothing. Before calling that a defect I
clicked "Add node" instead — and a node appeared. So the clicking worked and the
*coordinate* was wrong: I had read it off a screenshot of a different browser
instance, whose viewport is 1180×760, while this one is 1180×**617**. Save was
at y=576 here and y=714 there, so the click had gone off the bottom of the
screen. Had I not checked, this section would have reported a stash that does
not work.

**The defect.** Once the click landed:

```rust
format!("saved {text} bytes under “{STASH_KEY}”")
```

`{text}` where `{text.len()}` was meant. So the note was not a count, it was the
**entire document** — 7462 bytes of JSON, inside a sentence claiming to be a
count. The bottom panel grew to hold it, pushed the toolbar off the top of the
window, and left the note reading `saved {…} bytes`, which is not a number and
not a sentence.

The correct form was **twelve lines below**, in the "Copy the plan as text"
button: `format!("copied {} bytes to the clipboard", text.len())`. One
interpolation short, in the only place in the tree where that mistake was made.

**Why nothing caught it.** Nothing read the note. Every test in the web crate
covers the stash *behaviour* — write, read, forget, a broken stash falling back
to the sample — and all of those were green and stayed green while the panel
grew a hundred times taller than it should. The property that was broken is a
property of a **sentence**: it must be short. No amount of testing the storage
would have found it, and no gate would, because the note is only wrong when
something is actually stashed.

`saved_note` is now its own function and the test asserts the note is under 80
characters and contains no fragment of the plan — not merely that it mentions
one. Reinstating `{text}` fails both tests.

**Verified where it broke.** Bundle rebuilt, and the same click in the same
browser:

```
saved 7462 bytes under "craftkit/plan"
```

One line, panel back at the bottom, toolbar intact, four nodes.

**A positive result worth having.** The round trip itself works, and had never
been exercised: save, reload the page, and the app comes back from the browser
alone — *"reopened "Sample plan" from this browser"*, four nodes, status `saved`.
§18 calls the single-slot stash the riskiest thing in the web build, and it is
the part that is now known to work rather than assumed to.

**The rest of the row, driven for real — three verified negatives.** The defect
was in one button's sentence, but the other three produce sentences too, and
nothing had ever read them. Each was clicked against the same live page with a
cleared `localStorage`:

| Clicked | localStorage after | Note shown | Verdict |
|---|---|---|---|
| Restore, nothing stashed | `[]` | `there is nothing saved here yet` | correct |
| Save in this browser | `["craftkit/plan"]` | `saved 7462 bytes under "craftkit/plan"` | correct (§31) |
| Forget | `[]` | `the saved plan is gone; what is on screen is not` | correct, and honest about the difference |
| Copy the plan as text | (nothing) | `copied 5667 bytes to the clipboard` | correct |

Every screenshot has the note on one line at the bottom with the toolbar intact,
which is the property §31 broke.

The copy row needed the instrument checked too. `navigator.clipboard.readText()`
is refused in this headless Chrome — `NotAllowedError: Read permission denied`,
and `Browser.grantPermissions` did not lift it — so *reading the clipboard back
is not available here*. Rather than report an unverifiable success, I wrapped
`writeText` in the page and recorded what it was called with and how its promise
settled:

```
{"len":5667,"head":"{\n  \"format\": \"craftkit/1\",\n  \"meta\": {\n    \"written_by\": \"craftkit 0.","ok":true}
```

5667 bytes, the promise resolved, and the head is the plan. So the button copied
what it claims to copy. Note the honest limit of this: it proves the write
succeeded, **not** that the bytes are retrievable by a later paste — that needs a
document with clipboard read permission, which this browser will not grant.

Also worth recording, because it looks like an inconsistency and is not: the
save note counts 7462 bytes and the copy note 5667. Both come from the same
`stash_text`, and the difference is the plan itself — four nodes when the page
was reloaded from the stash, three after edits in the session. Same function,
same document, different moment.

---

## 32. `ui.click` rendered instead of clicking, and said `clicked` when it did

§17 recorded that `ui.click` reports rather than acts, and filed the enforcement
column as *nothing*. Reading it again, the interesting part was not the design
but the fact that the one place an agent reads — the reply — contradicted it.
Two defects, found by driving the running app rather than by reading.

**One. The method name was decoration.** `Method::parse` maps `ui`, `ui.click`,
`ui.screenshot` and `ui.render` all to `Method::Ui`, and the host then read the
action from `params.action`, defaulting to `render`. The method the caller
actually typed was never read. So the request the docs show:

```json
{"id":3,"method":"ui.click","params":{"x":400,"y":12}}
```

answered `rendered 1180x720`, `ok: true`, `data.png_base64` — a full frame, and
no click. And symmetrically, `{"method":"ui.render","params":{"action":"click"}}`
*did* click. Measured against the live app:

```
id=1 ok=True  message='rendered 1180x720'          <- asked to click
id=2 ok=True  message='clicked `toolbar.node.duplicate`'   <- asked to render
```

Both `ok: true`. Nothing errored anywhere.

The worst part is that the documentation had been written around the accident:
`docs/agent-playbook.md` and `skills/craftkit-checking/SKILL.md` both showed
`{"method":"ui.render","params":{"action":"click"}}` — the one spelling that
worked, for the wrong reason — and the recipe called the four names "aliases".

**Two. The reply claimed an action that does not happen.** With the action
finally reaching the click, the reply was:

```
clicked `toolbar.node.duplicate`
```

Three of those in a row against a live app, and the document was byte-identical
afterwards: 3 nodes, `dirty: false`, before and after. §17 says a click is a hit
test; the sentence said otherwise; an agent has every reason to believe the button
was pressed. The control crate's own refusal message states the principle the app
broke ten lines away — *"a silent success here is how an agent comes to believe
it clicked something."*

**Fixed.** `Request::ui_action` in `crates/control/src/protocol.rs` decides from
the **method name**, consulting `params.action` only for the bare `ui`, and
refuses a method and an action that disagree rather than guessing which one to
ignore. The click reply is `click_message`, which says what happened:

```
`toolbar.node.duplicate` is at that point; a click reports where it landed and
does not activate anything — run it with `engine.execute`
```

The name is still in `data.hit`, which is where a machine should read it.

**A lesson worth keeping, because I walked into it.** The first version of the
mutation case filtered on `ui_action` and the test that catches the mutation was
called `the_method_name_decides_what_a_ui_request_does` — so the filter ran one
unrelated test, which passed, and the case would have shipped as "caught" while
catching nothing. It only showed up because the run was done by hand first and
the output was read: `1 passed; 48 filtered out`. A mutation case is a claim that
a gate bites; running it once by hand is the only way to know the claim is true.

**The examples are now a gate.** `Request::ui_action` is locked by a mutation,
but the docs' *examples* were prose and nothing parsed them — so the spelling
this section is about was teachable. `cargo xtask lists` now reads every
request-shaped line in `docs/` and `skills/`: it must be JSON, name a method the
protocol accepts, and not contradict itself.

The method and action names are **read out of `crates/control/src/protocol.rs`'s
own match arms**, not listed here. A second list is §22's defect waiting to
happen, so `wire_names` takes the names from where they are defined and refuses
to run if it finds none — a parser that matched nothing would otherwise check the
examples against an empty set and pass, which is the failure mode this document
keeps writing about.

**Its first finding was in prose written ten minutes earlier.** The playbook
explained the refusal with a copyable counter-example,
`{"method":"ui.render","params":{"action":"click"}}`, and the gate reported it —
correctly, by its own rule, and wrongly in effect: a request-shaped object on a
line of its own is something to send, and that one was something *not* to send.
The rule is kept and the prose was changed to describe the bad request in words,
with a note saying why. That is the honest resolution rather than a special case
for "unless the line looks like a warning", which is the kind of heuristic that
cries wolf.

Seven examples are checked. What the gate cannot do is judge whether an example
is a *sensible* request, or prove it behaves as documented — a wrong coordinate
in a click example is still a well-formed request. That is still found by a
person running it.

---

## 33. Nothing in the tree can save a design

Found by asking §32's question of the next surface: not *can this check fail*,
but *what does this do, and is anyone relying on it*. `apps/craftkit` opens a
design, so the obvious next question is where a design goes when you have edited
it.

**Answer: nowhere.** Measured three ways.

```console
$ grep -rn "craftkit_io::save" --include=*.rs apps/ crates/
apps/craftkit-cli/src/cli.rs:452:        craftkit_io::save(&file, session.document()).expect(…);
```

One caller in the whole workspace, at `cli.rs:452`, and `#[cfg(test)]` starts at
line 269 — so it is a **test**. There is no production call to `craftkit_io::save`
anywhere in craftkit.

- The desktop app calls `craftkit_io::load` for `--file` and never `save`. There
  is no Save control; the shell's status bar is where saving would announce
  itself, and it says `unsaved` / `saved`, never `save`.
- The CLI's `--out` writes a **PNG**: `snapshot <script.json> --out <file.png>`.
  `run` has no `--out` for a design at all.
- The web build writes `localStorage`, not a file.

Worth being precise about which half is missing, because it changes who has to do
what. `craftkit-io`'s **write path is finished**: `save`, `save_text`, a
`SaveError` with a path and a recoverability judgement in every variant, and
doc-tests that assert the text it produces. It has no production caller. So this
is not a missing capability in the library — it is a missing **wire-up**, and
whichever way the owner decides to do it, the format layer needs no work.

**The only signal that work is being lost** is in `on_exit`:

```rust
if session.is_dirty() {
    log::warn!("exiting with unsaved changes");
}
```

which goes to the `Stderr` logger installed at `main.rs:555`. Launched from a
terminal, a person might see it. Launched from a desktop environment — a double
click, a `.desktop` file, a dock icon — stderr goes to the journal and nobody
sees it. So the ordinary way to run this app loses every edit silently, and the
extraordinary way loses them with one line on a console that has already closed.

**§18 gets this wrong, and it is worth correcting because it is the sentence an
agent would rely on.** It says:

> the desktop app calls `craftkit-io` directly, the way the CLI does

The app calls `load`. It has no `save` to call, and neither does the CLI. The
paragraph reads as though the host writes files, and the cost it names — *"an
agent cannot persist a plan through the command registry"* — understates the
position: an agent cannot persist a plan **at all**, through any interface,
unless it writes the format itself.

**This is the pending owner decision, not a bug to fix here.** Item 6 of the
suggested order already asks whether `save`/`open` belongs in the command
registry, and this section sharpens what that decision actually costs. The options
are not equivalent: a `doc.save` command is one registry entry plus a file
dialog, and it makes saving reachable by every surface for free; a host-side
shortcut is one binding plus a dialog and leaves agents unable to save; a
recovery file is invisible until the day it is needed. What must not happen is
what happens today, which is that all three look implemented because the format
round-trips in tests and the app *can* open a file.

What is cheap and is fixed here: nothing. This is recorded rather than acted on,
deliberately, because every version of "fix it" is a product decision the owner
has not made — and §18's own argument, that file I/O is the host's business, is
an argument about *where* the code lives, not a reason the capability can be
absent.

---

## 34. `AGENTS.md` sent every agent to a file that does not exist

Found by reading `AGENTS.md` to check whether §32's change had made its control-
channel description wrong. It had not — but the paragraph below that one says:

> `docs/control-protocol.md` has the wire format.

That file has never existed. `docs/` holds `agent-playbook.md`, `architecture.md`,
`README.md` and three directories. So the one document every agent is told to
consult, for the one thing §32 changed, is a dead reference — in the file that
introduces the channel in the first place.

**The wider finding is the gate.** `citation_drift` (§7, §26) reads every
`file.rs:12` in prose and checks it points at something. It found seven stale
citations across `docs/` and `skills/` when it was extended to recurse. It does
**not** look at document-to-document references, which are citations too and are
the ones a reader actually follows.

Measured before fixing anything, so the claim is a count and not a story:

```console
$ python3 scan.py     # every backticked `*.md` in docs/, skills/, AGENTS.md,
                      # README.md, CONTRIBUTING.md
markdown references in prose: 58, broken: 1
  AGENTS.md:204 -> docs/control-protocol.md
```

Fifty-eight references, one broken. Both ways a reader tries one are tried — next
to the referring document, and from the repository root — because both spellings
are in use in this tree.

**Fixed both ways.** The sentence now points at `docs/agent-playbook.md` and at
`crates/control/src/protocol.rs`, and `broken_document_links` in `cargo xtask
lists` checks every `\`something.md\`` in prose, resolving both ways. The gate
found one broken reference the moment it existed — `AGENTS.md`'s, before I fixed
it — and the mutation puts it back:

```
caught  prose: a document points at a document that is not in the tree
```

The honest limit is the same one `citation_drift` states: this checks that the
target exists, never that it says what the sentence claims. Fifty-seven other
references resolve to real files, and at least one of them is probably about
something that has since changed.

---

## 35. Nothing checked that a command's effect survives the file

Found by putting §33's question to the format instead of the application: if
`.craftkit` is the only persistence there is, does everything a command does
reach it?

**Nothing checked it.** The round-trip tests that existed were at the *model*
level — `documents_round_trip_through_json_and_keep_the_id_counter` proves the
`Document` type survives serde. That is not the same claim. A command that
writes to a field the format does not carry would pass every test in the tree,
because the type round-trips perfectly; only the *command's* effect is lost.

**The measurement.** One command at a time: run it, save the document, load it
back, and compare. Fifteen candidates, and the first version of the probe was
worthless in a way I had to catch — it reported success for `playback.seek`,
because the playhead is **session state** and the document never had it. A
round-trip assertion over a command that changed nothing proves nothing, so the
test now asserts that the command *changed* the document first, and says so
when it did not.

With that in place: **fourteen commands change the document and every one of
them survives the file**, byte for byte. `doc.rename`, `doc.range`, `node.add`,
`node.duplicate`, `node.rename`, `node.kind`, `node.move`, `node.mute`,
`node.remove`, `prop.set`, `prop.reset`, `key.set`, `key.ease`, `key.remove`,
`key.clear`. A verified negative, and a useful one: the format is not missing
anything the registry writes.

**What was missing was the *check*, and the list.** The test is
`apps/craftkit-cli/tests/command_effects_survive_the_file.rs`, and it has a
completeness half that matters more than the round-trip half: every command in
the registry has to be either in the table or in `NOT_IN_THE_FILE` with a reason
recorded. A new command is then a *decision* — is this in the file? — rather
than an oversight nobody notices until a design loses an edit.

It lives in an app rather than in `craftkit-io` because it needs both crates and
`craftkit-io` is a *lower* layer than `craftkit-engine`; a dev-dependency
upwards would be a layering violation that `cargo xtask layers` correctly
reports. The statement is an application-level one — "the file the application
writes is the file it read" — so an application-level test is where it belongs.

**A small omission, found on the way.** The format document has a section
called *What is not in the file*, and it lists the plan, the id counter and
anything derived. It did not list the **playhead** or the **undo history**,
which are also not in any file and are the two absences a person actually
notices: a reopened design starts at its first frame with nothing to undo. The
section now says so, and a test asserts it keeps saying so.

**What it still cannot check.** Whether the *reloaded* document means the same
thing to the evaluator — that is `plan.evaluate` on both sides, and the plan is
computed, not stored. This test compares documents, not behaviour.

---

## 36. Thirteen wrong citations, and the gate could not see one of them

Found by asking `citation_drift` (§7, §26) the question it had never been asked:
*what does it actually read?* It reads `name.rs:12` — a file name and a line
number. It does not read a **bare** line number, and this tree had twenty-four
of those.

**The bare form.** `` (`:330`) `` means "the line I named a sentence ago" — the
file was mentioned just above, and the number is a short way back to it. It is a
real convention, and it is also a citation with no owner.

Measured, by resolving each one the way a reader would — to the nearest `.rs`
the document has named:

```console
bare citations resolved: 19, stale: 4, unresolved: 0
```

Four flagged, and then the interesting part. Checking each against the symbol it
claims to point at found **thirteen wrong out of twenty-four**, in four
different ways:

| How it was wrong | Example |
|---|---|
| The line had moved | `add-an-icon.md` cited `icons.rs:300` for a test now at 286 |
| The line had moved a lot | `add-a-command.md` cited `:353` and `:365` for a test now at 390 |
| **The file was wrong** | `add-a-control-channel-method.md` cited `:446` for a test that is in `server.rs`, not `protocol.rs` |
| The number was right, the file was wrong | `change-the-file-format.md` cited `:72` for a test in `format/pending.rs`, read as `format.rs` |

The third kind is the worst, because a bare citation *cannot* be checked by
anyone: the sentence before it names two files, and which one the number means
depends on which one you happened to notice last.

**And the explicit ones were wrong too.** `add-a-command.md` cited
`crates/engine/src/command.rs:312` for a test now at 348. `citation_drift`
passed it, because 312 is real code — it is the gap §34 named, now with a
number on it: of thirteen wrong citations, the mechanical rule sees **four**.
The other nine point at code that exists and says something else, which no tool
can know is wrong.

**One of them was mine.** `add-a-control-channel-method.md` cited `:254` for
`every_method_round_trips_through_its_own_name`; §32's `ui_action` pushed it to
389, and 254 is now the middle of my own new function. The same thing happened
to `protocol.rs:261` in §32, and `citation_drift` caught that one because it had
landed on a lone brace. A citation is a promise to keep a file in step.

**The fix is not to check the bare form — it is to stop using it.** A citation
that names a *symbol* rots into a wrong reading at worst; one that names a
*line* rots into a wrong *place*, and a wrong place sends somebody to the wrong
file. So:

- All twenty-four bare citations are gone. Where a symbol was already named, the
  number went with it. Where it was not — `mute` is the shortest icon example —
  the file is named instead.
- The nine wrong explicit citations are corrected: `command.rs:312` → the
  symbol, `session.rs:262`/`:273` → `263` and `271` (the two
  `self.doc = snapshot;` lines, which is what the sentence says they are),
  `icons.rs:156` → `152` (the start of `fn mute`).
- `cargo xtask lists` now **refuses** a bare line citation, with the file to
  name in the message. The rule is mechanical and complete, so it cannot rot:
  a bare number with no file is an error, full stop.

The gate found four more bare citations that my own inventory had missed,
including two in a list form — `` (`UI_CRATES`, `:93`; `UI_MIN_LAYER`, `:96`) ``
— which a hand-written pattern had not matched. The gate is the better
instrument; it is now the one in charge of this.

**What this says about the rest of the citations.** The nine wrong explicit
citations are still there, corrected by hand, and they can rot again. The rule
that would catch them is "the cited line must contain the symbol the sentence
names" — which needs the sentence to be machine-readable, and it is not. The
honest position is the one §34 took: the gate checks that a citation points at
*something*, and a person checks that it points at the *right* thing. The
difference is that the bare form no longer adds a second way to be wrong.

---

## 37. The README's layer table was decoration

Found by asking the same question of the layer table that §36 asked of the
citations: *what does this check, and what does it leave out?*

`cargo xtask lists` compares the layering table in `docs/architecture.md` with
the table in `xtask/src/layers.rs`, and it has done so since §7. The README has
a **second copy of the same table**, in a different shape:

```text
crates/core     L0   ticks, values, ids, hostile parameters
crates/kernel   L2   the numeric kernel, behind the panic guard
...
```

and **nothing compared it with anything**. The two agree today — measured, not
assumed — but the agreement is a coincidence of two people having been careful,
and the README is the front door of the project. This is §22's shape one layer
down: one rule, two spellings, one of them enforced and the other decorative.

**Fixed.** `readme_layers` reads the README's rows, resolves each directory to
its package through the directory's own `Cargo.toml`, and compares the layer
with the code's. Both directions are checked, because either one alone is a
half-check:

```console
$ cargo xtask lists        # README says crates/kernel is L3; the code says L2
$ cargo xtask lists        # README's layer table does not mention craftkit-io (L3)
```

**Why the direction that matters is the one people forget.** A README that says
L3 for a crate the code puts at L2 is a wrong fact. A README that *omits* a
crate is a table that rots the moment a crate is added, and it rots quietly:
the new crate is invisible in the front door of the project until somebody goes
looking. The second failure is the more likely one, and it is the one the
"missing crate" direction catches.

**And the measurement that made it worth doing.** The two tables agreed, so
this section is a *verified negative* plus a gate. That is the honest shape of
this kind of finding: the defect is not that the README was wrong, it is that
nothing would have said so.

---

## 38. The CLI's help and its parser were two lists that nobody compared

§28 found that `craftkit-cli --help` never showed the script format — the one
thing anybody must know to use the tool — and fixed it. Reading that section
again raised the obvious next question: **the help is a hand-written string and
the subcommands are match arms.** What keeps them in step?

Nothing.

```rust
// apps/craftkit-cli/src/args.rs
match first.as_str() {
    "help" | "-h" | "--help" => …,
    "commands" => …,
    "run" => …,
    "eval" => …,
    "snapshot" => …,
    "crashes" => …,
    "mcp" => …,
    other => Err(UsageError::UnknownSubcommand(other.to_string())),
}
```

Seven arms, seven lines in `USAGE`, and no test anywhere compared them. They
agree today — measured, not assumed — but the failure mode is one-sided and
silent: a subcommand added to the parser and not to the help **exists and is
undiscoverable**, and the help is the only place an agent looks. §28's defect
was the help hiding the script format; this is the help hiding a whole
subcommand.

**Fixed by deriving one from the other.** The test reads the subcommand names
out of `args.rs`'s own source — the `match first.as_str()` block, found by
walking the braces, taking the first quoted word of each arm and skipping the
ones that start with `-` — and requires the help to mention every one of them.
The other direction is checked too, because a help that lists a command the
parser refuses is a lie of the same kind.

```console
$ # a subcommand added to the parser and not to the help
the parser accepts `export` and the help does not mention it:
  ["commands", "run", "eval", "snapshot", "crashes", "mcp", "help"]
```

**A note on the mechanism, because it is the same lesson as §22 and §36.** The
names are *read out of the parser*, not listed a second time in the test. A
test that listed them would have been a third copy, and would have needed its
own check — the thing this document has now found seven times. The scan is
hand-written, like every other parser in this tree, and it is bounded: it
counts braces, so it cannot run past the end of the match it is reading.

---

## 39. The part of §29 that *could* be gated, and was not

§29 swept the backticked names in `skills/` against the source and found three
places naming `pending_list_is_sorted` when the test is called
`the_pending_list_is_sorted`. It fixed them and then wrote:

> Prose-cited test symbols could be gated, and deliberately are not: every rule
> that separates `the_pending_list_is_sorted` from `catch_unwind` is a
> heuristic that would eventually flag something real.

That reasoning is right about the *general* rule, and this section is the
exception it did not see: **when the sentence names the file, the check is
anchored and stops being a heuristic.**

The sweep of the directories §29 did not cover found the case:

```text
docs/recipes/add-an-icon.md:5
yet. `every_command_has_an_icon_this_set_can_draw`
(the `every_command_has_an_icon_it_can_draw` test in `crates/ui/src/icons.rs`)
fails the moment you write `Icon("power")`
```

A recipe naming a test **that does not exist**, in the same sentence as the
right one — the parenthetical is an older name for the same test, left behind
when it was renamed. The reader is sent to look for a test that was never
there, and the sentence contradicts itself.

**What is checkable, and what is not.** The general rule — "every backticked
snake_case name in prose is a symbol in this tree" — cries wolf, because
`catch_unwind`, `unwrap_used`, `png_base64`, `serde_json` and `as_f64` are all
in prose and none of them is in this tree. But the *anchored* rule — "this
symbol is in this file" — does not:

| Claim | Verdict |
|---|---|
| `` `a_request_round_trips` (in `crates/mcp/src/rpc.rs`) `` | true |
| `` `the_pending_list_is_sorted` (in `crates/io/src/format/pending.rs`) `` | true |
| `` `every_command_has_an_icon_it_can_draw` test in `crates/ui/src/icons.rs` `` | **false** |

Eleven such claims in the tree, two false, both the same stale name.

**The rule, precisely.** A backticked symbol *immediately* followed by a
backticked `*.rs` path is a claim that the symbol is in that file — "immediately"
meaning the next backticked thing, in the same paragraph, with nothing much in
between. Two refinements were needed, and both came from the gate finding a
false positive of its own:

- The first version paired a test with a file named **three sections later**,
  because "the next backticked thing" crossed a paragraph boundary. A claim is
  about the file it names, so the two must be in the same breath: no blank
  line, and a gap of at most forty characters. The longest gap in a real claim
  is ten.
- The symbol must look like a test name — `every_`, `the_`, `a_`, `an_` — so
  `catch_unwind` is not a claim and the rule does not cry wolf.

**Fixed and gated.** The two stale names are gone (the correct one was already
in the sentence), and `symbol_file_claims` is in `cargo xtask lists` with a
mutation that puts the stale name back.

**What §29's verdict was, restated honestly.** §29 was right that the general
rule cannot be gated, and this section is the part of it that can. The two
statements are not in conflict: a *claim with a file* is a fact about this
tree; a *bare name* is a name from somewhere, and the tree cannot tell where
from.

**The general rule was counted, and every false positive is a quote.** In the
hunt that followed §113, the rule was measured against the tree: every
test-shaped name — `the_`, `a_`, `an_`, `every_`, `each_`, lower case, no
digits — in this file, in `docs/`, in `skills/`, in `AGENTS.md` and in
`README.md` that is **not** a `fn` in the tree. There are six:

| Name | Where | What it is |
|---|---|---|
| `a_change_this_build_has_never_heard_of` | this file, in a quoted error | a message, not a test |
| `a_trace_line_that_is_not_json_still_replays_as_a_step` | this file | the **old** name of a test this file is talking about |
| `an_older_file_is_read` | this file | the old name, quoted next to the new one |
| `every_command_has_an_icon_it_can_draw` | this file | the false claim §29 found, quoted as the example |
| `the_method_name_decides_what_a_ui_request_does` | this file | another old name, in a lesson about a filter that matched the wrong test |
| `a_test` | `docs/recipes/add-an-icon.md` | a placeholder: "a test *of that name* enforces it" |

**Six out of six are the prose doing its job.** Five are quotes of names the
file is *talking about* — a renamed test, a message in a quoted block — and
one is a placeholder. A gate on the general rule would have to flag all six,
and five of the six would be the gate crying wolf at prose that is correct.
§29's verdict, confirmed by a second count: **a bare name is a name from
somewhere**, and the ones that are not in the tree are the ones the tree is
quoting.

---

## 40. A pattern document promised a safety property the code did not have

`docs/patterns/clone-before-edit.md` is the document an agent reads before
touching the engine. It describes the rollback, and it described it in a
sentence that was false:

> **`evaluator.invalidate()` accompanies every restore.** The cache belongs to the
> document that produced it, and a restore puts a different document in place.

Read literally, that says: without the call, the cache would serve states
computed from a document that no longer exists. It says the same thing, in
code, on the method itself:

```rust
/// The engine calls this when it restores an undo entry: the cache belongs
/// to the document that produced it, and after a restore that document is
/// not the one in the cache.
pub fn invalidate(&mut self) {
```

And on `Session::replace_document`:

> Public because a document loaded from disk has to reset the evaluator's
> cache too, and a cache that survived a load would serve states computed
> from the *previous* document.

**Measured, the claim is false.** The evaluator's cache is content-addressed:
an entry is reused only when the node it was computed from still hashes the
same, and the walk stops at the first entry that does not match. A restored
document cannot be served a state belonging to a node it no longer has. The
removal of every `invalidate()` call in the engine changes nothing:

| Tree | Tests | Result |
|---|---|---|
| As it stands | 604 | 604 passed, 0 failed |
| The `self.evaluator.invalidate();` in `crates/engine/src/session.rs` removed | 604 | 604 passed, 0 failed |

(The call was in three places before the rollback was folded into one
function; the measurement was made again afterwards, on the one call that
was left. The count is 604 because the measurement was taken before §43 added
two tests — the number is what the tree held at the time, and §41 is why a
number in a sentence needs a gate.)

The call is not *wrong* — clearing is cheaper than relying on a property, and
it makes the invariant explicit. But the *reason* it was given was wrong, and
a false reason in the document an agent reads is worse than no reason: it
teaches a reader to believe something untrue about the safety of the rollback.

**The count was the thing that rotted.** The same document said "the four
restore statements" — and, in three other places, "one", "two" and "three".
The two rollback branches each spelled out the same four statements, so the
document had to keep a count in step with two copies of the same code, by
hand. A number in a sentence is not a citation, and `citation_drift` only
reads `file.rs:12`; nothing in the tree could see the count drift.

**Fixed by quoting instead of counting.** The rollback is now one function:

```rust
fn restore(&mut self, doc: craftkit_model::Document, dirty: bool) {
    self.doc = doc;
    self.evaluator.invalidate();
    self.recompute();
    self.dirty = dirty;
}
```

Both branches call it, and `replace_document` calls it too, so "the same
statements" is true by construction rather than by agreement. The document
quotes the function instead of counting it, and the quote is **exact** — a
fenced block whose info string says `exact` is a claim that it is the source,
word for word.

**And that claim is gated.** `quoted_functions_match_the_source` is in
`cargo xtask lists`: every function an `exact` block quotes must be the
function in the file the block names, line for line, in both directions. A
line added to the function and not to the document is caught, and so is a line
in the document that the function does not have. Two mutations, one for each
direction.

The marker is deliberate. Most blocks in the tree are *illustrations* with
elisions in them, and pretending otherwise would make the check cry wolf —
which is §29's lesson applied to §39's rule: anchor the check or do not make
it.

**The property is now tested, and the test bites.** The claim that a restore
cannot serve a stale state had no test at the engine's level, where the
restore happens. `a_restore_cannot_serve_a_state_from_the_document_it_replaced`
changes the document twice, puts the first change back **without** clearing
the cache, and compares the result with a plan computed by a session that has
never seen any of the three documents.

**A wrong prediction, recorded.** The first version of that test restored the
*original* document and **passed with the fingerprint broken**. The reason is
worth keeping: with a broken fingerprint the cache is frozen at the first
evaluation, and the document it was frozen at was the one being restored, so
the stale state and the correct state were the same state. The test now
restores a document that differs from the first evaluation, and it fails when
the fingerprint stops distinguishing nodes.

The mutation that proves it is the sharpest one in the tree: it removes the
node's **name** from the fingerprint. Every test in `craftkit-eval` still
passes — they all change values and positions — and only the engine's own
test sees it, because it is the only one that renames a node and then puts
the document back. That is a coverage difference the mutation script found
and no reading of the code would have.

### The false alarm, which was worth having

`cargo xtask lists` reported:

```text
docs/patterns/clone-before-edit.md: line 94 points at `KNOWN-GAPS.md`, which is not in the tree
```

The file is `KNOWN-GAPS.md`. The document said `KNOW-GAPS.md` — a typo, in a
sentence added minutes earlier. The gate was right and the typo was mine.

What happened next is the part worth recording. `stat`, `File::open` and
`read` all found the file; `metadata()` inside the xtask process returned
`ENOENT`; `read_dir` of the same parent listed it with its full size. The
rule "a document that is not in the tree is not in the directory either" was
added as a fallback, with a doc comment asserting that "`stat` intermittently
answered 'no such file' for a document that `read_dir` listed". **That
comment was false**, and it was written into the build tool in the same hour
it was believed. It is now deleted, and `in_the_tree` asks the question one
way again.

The lesson is not "check your spelling" — it is that a workaround built on an
unverified belief is a new false claim, and it is written into the code
rather than into a conversation. A gate that says "this is not here" deserves
to be doubted *once*, carefully, by looking at the name it was given.

---

## 41. Two documents disagreed about one file, and both were wrong

§40 found a number in a sentence that had rotted: "the four restore
statements". It fixed that one by quoting the code instead of counting it,
and it left a question behind: **how many other numbers are in this tree's
prose, and how many of them are true?**

The sweep looked for the shapes a number takes in a sentence — "N lines",
"N commands", "N areas", "N rules", "N tests", "the tests start at line N",
"N lines of code" — and checked each against the tree.

**Fifteen claims. Four wrong.**

| Claim | File | True? |
|---|---|---|
| "153 lines" — `crates/core/src/lib.rs` | `docs/agent-playbook.md` | yes |
| "its first ~900 lines" — `crates/engine/src/commands.rs` | `docs/agent-playbook.md` | yes (an approximation) |
| "the tests start at line 957" | `docs/agent-playbook.md` | yes |
| "21 commands across seven areas" | `docs/patterns/everything-is-a-command.md` | yes |
| "the two commands with `undoable: false`" | `docs/architecture.md` | yes |
| "the three panic boundaries" | `docs/architecture.md` | yes |
| "the six clippy lints" | `docs/agent-playbook.md` | yes |
| "rule 4 in `crates/engine/src/lib.rs`" | `docs/agent-playbook.md` | yes |
| "the four rules" — `crates/control/src/lib.rs` | `docs/recipes/add-a-control-channel-method.md` | yes |
| "Over 500 tests" | `README.md` | yes (604) |
| "the three applications" | `docs/architecture.md` | yes |
| **"110 lines" — `crates/io/src/format/pending.rs`** | `docs/agent-playbook.md` | **no — it is 174** |
| **"98 lines" — the same file** | `docs/recipes/change-the-file-format.md` | **no — it is 174** |
| **"the same three lines" — the rollback** | `docs/architecture.md` | **no — it was four statements** |
| **"lines 196-282", "87 lines" — `Session::execute`** | `docs/agent-playbook.md` | **no — but §40 broke it, not time** |

The last row is this document's own doing and is here because a finding that
flatters the finder is the one most worth checking. §40 folded the rollback
into a function, and `Session::execute` went from 196-282 to 196-276, so a
line range that was correct an hour ago is now wrong. It is counted as
wrong because it *is* wrong, and it is fixed the way §36 says line
citations are fixed: the symbol is named and the range is dropped. The other
three had rotted on their own.

**The two wrong counts were about the same file, and they disagreed with each
other.** One document said 110 lines, another said 98, and the file is 174.
Neither was a typo: both were numbers written once, when the file was a
different length, and both documents were *correct when written*. The recipe
tells an agent to read that file "top to bottom" and calls it "the rule" —
so the one number that would have told the agent how much reading it was
being asked for was the number that was wrong.

The rollback count was wrong in the same way, and it is §40's finding seen
from the other side: "the same three lines" for a block of **four**. It had
been wrong for a while.

**Why the citation gate did not see them.** §36 removed the bare line
*citations* — `` `:123` `` — and taught the tree to cite symbols. A line
*count* is not a citation: it names no line, so `citation_drift` has nothing
to resolve, and the rule that replaced it ("cite the symbol") says nothing
about how long a file is. The numbers were left behind by a fix that was
right about its own subject.

**Fixed, and gated.** The four wrong claims are corrected, and
`prose_line_counts_match_the_files` is in `cargo xtask lists`. The rule is
anchored the way §39's is: **the nearest backticked `*.rs` path before the
number, in the same paragraph, is the file the number is about.** Two forms
count, and only two:

- "N lines" — the file is N lines long. `~900` does not count: an
  approximation is not a claim that can be wrong.
- "the tests start at line N" — the file's test module begins at that line.

Two mutations, one for each form. The second is the sharp one: it moves the
line the document names to a line that holds something else, and the gate
reports what is actually there.

**What is not gated, and why.** The counts written as *words* — "the same
three lines", "the six clippy lints" — are not mechanically locatable, and a
rule that guessed at them would cry wolf. The two that were wrong are fixed
anyway, and §40's answer applies to them: quote the code, do not count it.
The "lines 196-282" claim was a line citation in disguise, so it went the
way §36 says line citations go: the symbol is named, and the range is not.

### A note on the two that *were* right

Eleven of the fifteen claims were true, including the ones that look like
they should not be: "21 commands across seven areas" (the registry really
does hold 21 commands in 7 areas), "the three panic boundaries" (the
architecture document counts the control server's boundary, which is easy to
forget), and "the tests start at line 957".

They were true by luck, not by anything. That is the whole point of the
sweep: a number in a sentence is a claim that is either checked or not, and
until now none of them was.

### The safety limits, which are the numbers that matter

The same sweep was pointed at the *constants*: the hard limits that bound a
hostile file, written down in three documents so that an agent coding against
them uses the right ones. **Twenty claims, all twenty true** — `MAX_NODES`
10 000, `MAX_DEPTH` 32, `MAX_PROPERTIES` 50 000, `MAX_KEYS` 10 000,
`MAX_PATH_SEGMENTS` 16, `MAX_NAME` 255, `MAX_EDGES` 20 000, `MAX_STEPS` 64,
`MAX_LINE` `1 << 20`, `MAX_TEXT` `1 << 16`, `MAX_ENTRIES` 128.

True by luck. Change `MAX_NODES` and the architecture table keeps saying
10 000, and the table is what an agent reads.

`prose_constants_match_the_code` is in `cargo xtask lists`. Two shapes:

- a constant and its value side by side — `` `MAX_DEPTH` 32 `` — wherever
  that pair appears;
- a constant and its value in **different cells of a table row**, which is
  how the pattern document writes the cost bound.

Two mutations, one for each shape. The message names the file, because two
constants can share a name: the engine's history keeps 128 entries and its
log keeps 256, and a claim that matches neither is reported against both.

---

## 42. The rollback is described in five places, and three of them were wrong

§40 found that `docs/patterns/clone-before-edit.md` gave a false reason for
why `Evaluator::invalidate` exists, and fixed it — in the document and in the
method's own doc comment. It also folded the rollback into one function,
`Session::restore`, so the two branches could not drift apart.

That left a question the fix did not answer: **where else is the rollback
described?** The answer was three more places, and two of them were wrong.

| Place | What it said | True? |
|---|---|---|
| `crates/eval/src/evaluate.rs` — `invalidate`'s doc comment | "the cache belongs to the document that produced it" | **no** — §40 fixed it |
| `docs/patterns/clone-before-edit.md` | the same reason, in a bullet | **no** — §40 fixed it |
| `docs/architecture.md`, in "Incremental evaluation" | "because the cache belongs to the document that produced it" | **no** — fixed here |
| `docs/recipes/fix-a-crash.md`, in "The test you must add" | "`replace_document` invalidates the evaluator's cache, **which a direct assignment would not**" | **no** — fixed here |
| `docs/recipes/fix-a-crash.md`, in "A crash handled at the wrong layer" | "the two `self.doc = snapshot;` lines in `crates/engine/src/session.rs`" | **no** — §40 deleted those lines |
| `skills/craftkit-editing/SKILL.md`, in the `Session::execute` map | "restore snapshot, invalidate, recompute" | **no** — it is one call now |

**All six were wrong.** The false reason — "the cache belongs to the
document that produced it" — was written once and copied three times, and
each copy looked authoritative. A reader of `docs/architecture.md` would
learn that the cache is *owned* by a document, and from there the "a direct
assignment would not" sentence in the crash recipe reads as obvious.

They are all wrong for one reason, and §40 measured it: the cache is
content-addressed, so a restored document cannot be served a state belonging
to a node it no longer has.

**The lesson is not "be careful".** It is that **a fact stated in N places
is a fact with N chances to rot, and the gates check one of them.** §40
fixed the two places it had been sent to. The other three were found by
asking a question with a mechanical answer — *where else is this described?*
— and that question is now a gate.

**The gate: code that prose attributes to a file.** A claim is a *pair*: a
backticked piece of code and the backticked `*.rs` path it is about, next to
each other in the same paragraph. Two forms count:

- `Type::method` — the method must be in that file. **Eleven live claims**,
  all true.
- a statement, ending in `;` — the statement must be a *line* of that file.
  **One claim existed, and it was false.**

`attributed_code_is_in_the_file` is in `cargo xtask lists`, with a mutation
for each form. The statement mutation puts back the exact sentence that
started this section, and the gate says:

```text
docs/recipes/fix-a-crash.md: line 251 says `self.doc = snapshot;` is in
`crates/engine/src/session.rs`, and it is not
```

**What else the sweep found, and did not.** The same question — *where else
is this described?* — was asked of the rollback's **line citations**, and the
answer was better than expected. `skills/craftkit-editing/SKILL.md` maps
`Session::execute` with six line numbers, and **all six still point at the
right lines** after §40's refactor moved the function's tail. Only the
*words* beside one of them were stale: "restore snapshot, invalidate,
recompute" described three statements where there is now one call. A line
number that is right and a description that is wrong is the most annoying
combination there is, because the reader checks the line, sees that it is
right, and believes the words.

The lesson from that is the one §36 already taught, applied to a place §36
did not look: **cite the symbol, and say what it does.**

---

## 43. Two of the four statements in the rollback were pinned by no test

§40 folded the rollback into one function:

```rust
fn restore(&mut self, doc: craftkit_model::Document, dirty: bool) {
    self.doc = doc;
    self.evaluator.invalidate();
    self.recompute();
    self.dirty = dirty;
}
```

**A change to a function is a change to its contract, and the contract here
was not written down anywhere.** So the next question was mechanical: *for
each statement in it, is there a test that fails when the statement is
deleted?* Four statements, four deletions, one full test run each:

| Deleted | Tests that fail |
|---|---|
| `self.doc = doc;` | 4 |
| `self.evaluator.invalidate();` | 0 — the cache is content-addressed; §40 |
| `self.recompute();` | **0** |
| `self.dirty = dirty;` | **0** |

**Two of the four statements could be deleted and the whole workspace would
still be green.**

### The plan

`self.recompute()` is what puts the *plan* back — the thing the window draws,
the status bar reads and `plan.evaluate` returns. Without it, a document that
arrives from outside keeps the plan of whatever was there before.

A document arrives from outside in three places: the desktop app, the web app
and the CLI all call `Session::replace_document` when they open a file. That
is not a command, so `execute`'s success path never runs, and **nothing else
recomputes the plan**. A file that had just been opened would have shown the
previous document's plan.

That is §21's bug in a new place: the engine says one thing and the screen
says another, with `ok: true` on the reply and nothing reporting a problem.
§21 found it by looking at a real window; it survived every test because the
window was the only witness.

`loading_a_document_re_evaluates_the_plan` now pins it.

### The dirty flag

`self.dirty = dirty;` restores the flag, and **no command sets it** — the
engine sets it only after a command succeeds. So the statement is a
no-op in every path that exists today, and the pattern doc gave it a reason
that was false: *"restoring only the document would leave a save prompt for
changes that were rolled back"*. It would not: the flag was never set.

It is still worth keeping — it is a precaution against the next command that
does mark the project dirty — and it is now pinned by
`a_restore_puts_the_dirty_flag_back`. A precaution nobody checks is a
comment.

### The half-applied command

There was no way to write the first test, and the reason is worth recording:
**the plan is recomputed on the success path too**, so a command that is
refused before it changes anything never shows the difference between the two
recomputes. Every failure the tree had was refused early.

So the tree needed a command that changes the document and *then* fails.
`test.half_apply` is one: it mutes the filter — which changes the plan's
output — and then refuses. It is registered only in test builds, beside
`test.panic`, and it is the only way to reach the state where the document
and the plan disagree.

`a_failed_command_leaves_the_plan_as_it_was` uses it.

### The lesson

**"The rollback restores the document" is not the contract. The contract is
"the rollback restores the document *and the plan* and the dirty flag", and
two thirds of it was untested.** The doc said the document; the code did
three things; the tests checked one.

The generalisable instrument is cheap and it is the one to reach for after
any change to a small, load-bearing function: **delete each statement in
turn and watch the tests.** A function of four statements is a contract of
four claims, and a claim with no test is a comment that happens to compile.

The four new tests are proven to bite: delete the statement each one
guards and it fails.

---

## 44. The check that catches a command lying about itself was never run

§43's instrument — *delete each statement and watch the tests* — was pointed
at the next most load-bearing function in the tree, `Session::execute`.
Eleven statements, eleven full test runs:

| Statement | Tests that fail, before this section |
|---|---|
| `if self.doc == snapshot` — the no-op check | **0** |
| `self.restore(snapshot, before_dirty)` in that branch | **0** |
| `self.history.push(snapshot, spec.label)` | 5 |
| `self.dirty = true` | 1 |
| `self.recompute()` | 2 |
| `self.log.record(id, true, …)` | 1 |
| `self.restore(snapshot, before_dirty)` on the error path | 1 |
| `self.log.record(id, false, …)` on the error path | **0** |
| `self.restore(snapshot, before_dirty)` on the panic path | 1 |
| `self.log.record(id, false, …)` on the panic path | **0** |
| `let e = e.with_command(id)` | 1 |

**Four statements had no test, and three of them are promises the documents
make.**

### The no-op check

```rust
if self.doc == snapshot {
    // A command that declares `mutates: true` and writes nothing is a bug —
    // almost always one that forgot to write — and would otherwise put a
    // no-op step on the undo stack.
```

The engine compares the document with the snapshot it took before the
command, and refuses a command that claims to change the document and does
not. The pattern document promises it in a table row, the architecture
document promises it, and the code says it in a comment.

**No command in the tree ever did it.** Every command that declares
`mutates: true` writes something, so the check never fired, so no test
reached it, so deleting it changed nothing. A guard against a bug that has
not happened yet is exactly the kind of thing that quietly stops being a
guard.

`test.noop` is the command that does it: it declares `mutates: true` and
changes nothing. `a_command_that_says_it_changes_the_document_and_does_not_is_an_error`
now pins the refusal, the message, the untouched document, the absent undo
entry and the clean dirty flag.

### The dirty flag, again

§43 found that `self.dirty = dirty;` in the restore is a no-op, because no
command sets the flag. The same was true of the branch above, for the same
reason — and the reason was *the same reason twice*: the engine sets the
flag only after a command succeeds, so nothing can change it between the
capture and the restore.

`test.dirty_noop` breaks the circle: it marks the project dirty, leaves the
document alone, and succeeds. The engine refuses it as a no-op, and the
rollback has to put the flag back — which is the first time in this tree's
history that the dirty flag has been anything but a constant.

`a_command_that_marks_the_project_dirty_and_changes_nothing_leaves_the_flag_alone`
pins it, and the branch now goes through `Session::restore` like the other
two, so there is one place where the session is put back and one place where
the flag is restored.

### Two log entries

The log records what ran. Three things can end a command badly — an unknown
id, a refusal, a panic — and the log test used the first: an unknown id
fails in the *lookup*, before the snapshot, before the rollback, before the
log entry. So the two entries that matter most were never written by a test.

`a_refused_command_is_logged_as_a_failure` and
`a_panicking_command_is_logged_as_a_failure` now write them.

### The lesson

§43's lesson was "a change to a function is a change to its contract, and
the contract was written down nowhere". This is the same finding one level
up: **the contract was written down in a table row, and the table row was
not a test.** "A command declaring `mutates: true` that writes nothing is an
`EngineError::Internal`" reads like a guarantee. It was a *hope*, held by a
branch that no test had ever entered.

The instrument generalises further than §43 said: it is not only the
statements of a function that need checking, but the *branches* — a branch
that no test enters is a branch that does not work. Four of the eleven
statements above were in branches that nothing had ever reached.

`Session::execute` is now fully pinned: every statement in it, deleted,
fails a test. (The test count is not written here on purpose — §41 is why a
number in a sentence needs a gate, and this one has none.)

---

## 45. The pending-change mechanism had never run

The format-change recipe is the one document in the tree that a *future*
change depends on: it tells whoever bumps the version how to write the
migration that rewrites an old design. It is also the document that had gone
stale in four separate ways, all of the same shape — a statement about the
tree that was written once and then never checked again:

1. a note claiming the loader does not call `apply` (it does);
2. four test citations pointing at the wrong lines (all four);
3. a note claiming the folding machinery does not exist (it does);
4. a line count of `pending.rs` in two other documents (it grew).

None of the four is a `file.rs:12` citation, so §36's rules saw none of
them. Each was prose about the tree.

### The loader calls `apply`. The recipe says it does not.

The recipe's step 4 was a blockquote:

> **Note — the loader does not call `apply` yet.** In the tree as of this
> writing, `grep -rn "pending::apply" crates/` returns nothing … Add that
> call as part of this change if it is still missing.

The grep returns one line: `crates/io/src/format.rs:203`. The call is there,
in exactly the place the note says it belongs — between the parse and the
validation, because a document has to be a `Document` before it can be
stepped and has to be stepped before it is validated.

**A note in a recipe is not a citation**, so nothing noticed it going stale.
§36's rules cover `file.rs:12`; a paragraph that says "the loader does not
do X" is prose about behaviour, and prose about behaviour was unchecked until
this section.

### The mechanism had never executed

`pending::apply` was a skeleton:

```rust
pub fn apply(doc: &mut craftkit_model::Document) {
    let _ = doc;
    for name in names() {
        // Each pending module's `step` is called here as it is added…
        let _ = name;
    }
}
```

`FILES` has been empty since version 1 — the module's own docs say so — so
`apply` had run **zero** times. The module promises that "`step` functions
run in order, each seeing what the previous one produced", and nothing
tested the order or the chaining, because there was nothing to run.

The first real format change would have been the first execution of the
mechanism, in the one place a silent failure is expensive: a design written
by an older build, loaded by a newer one, quietly not rewritten.

`apply` now collects the steps the names give and hands them to `run_steps`,
and `run_steps` is tested with two synthetic steps — one that renames a node
and one that checks it can see the rename. The order is the contract, and
the second test proves it by showing that the *last* step wins.

### Four citations of a test, all four stale

The recipe and the skills cite tests by line:

| Citation | The test is at |
|---|---|
| `crates/io/src/format/pending.rs:89` — `every_pending_name_is_a_plausible_file_name` | line 236 |
| `crates/io/src/store.rs:138` — `a_document_round_trips_through_a_file` | line 238 |
| `crates/engine/src/crash.rs:325` — `the_replay_rebuilds_the_document_that_crashed` | line 316 |
| `crates/io/src/format/pending.rs:44` — `the_pending_list_is_sorted` | line 96 |

**All four were wrong, by 9, 52, 100 and 147 lines.** `citation_drift` did
not see them, because it only asks whether the line is *in* the file and
points at something — and a doc comment is something.

The fix is §36's rule, applied to the last citations that had it:
`citations_of_tests_point_at_the_test` is in `cargo xtask lists`, and it
checks a citation **paired with a test-shaped name** — next to it, in the
same paragraph, in either order. A citation of a file's *region* — a struct
field, a block of code — is not a claim about one symbol, and the tree has
plenty of those, so the rule is deliberately narrow.

### The folding machinery exists. The recipe said it did not.

The same note was duplicated further down the recipe, under "How a release
folds the change in". It read:

> **Note** — the pattern document and the release script "do not exist in the
> tree yet", and `ls scripts` is empty.

All three clauses were false. `docs/patterns/format-changes.md` is in the
tree. `scripts/fold-format-pending.sh` is in the tree. `scripts/` holds
three scripts. The only true part of the sentence was the one it did not
make: there is no `v<N>/` directory under `crates/io/src/format/`, because
nothing has been folded.

A note is not a citation, and a note *about the tree* is the least
self-correcting kind of prose there is: the author had to look at the tree
to write it, and never had to look again.

### The gate caught my own stale count

While writing this section, the pending module grew from 174 lines to 245
and the line count in two documents went stale — **and §41's gate said so
within the same minute**, naming both documents and the true number. That is
the whole argument for gating a number rather than trusting it: the person who
made the change is the person most likely not to notice.

---

## 46. The instrument's own guard did not stop the run

`scripts/mutation-test-gates.sh` is the tree's instrument: it breaks each
rule on purpose and requires the gate to notice. Its own header is explicit
about the failure mode it has already suffered:

> **A mutation that does not apply.** Three of the mutations below silently
> did nothing once because an anchor string had moved. The output still
> looked plausible — a gate was reported MISSED that had never been handed a
> broken tree. So every mutation goes through `swap`, which exits with an
> error if the text it is looking for is not there. A mutation that changes
> nothing is a bug in this script, not a result.

**`swap` did exit with an error. Nobody was listening.**

`swap` is a shell function whose last command is a python heredoc. Python
exited non-zero; the function returned non-zero; and *no caller checked*,
because `swap` and `expect_catch` are two separate lines of a flat script
and nothing connects them. The run continued, the gate was handed an
**unmutated** tree, the gate passed, and the case was reported as a missed
gate — which is a statement about the gate, when it is a statement about this
file.

The run that exposed it was this session's own: §45 grew the pending module,
the line count in two documents went 174 → 245, and the §41 mutation's
anchor — `174 lines for the second one` — moved with it. The output said:

```text
  looking for: '174 lines for the second one'
  This is a bug in scripts/mutation-test-gates.sh: the anchor moved.
  A mutation that changes nothing makes every result below meaningless.
  MISSED  prose: a line count in prose is not the file's length
```

**The loud message was right there, and the line under it said the opposite.**
I read "1 of 67 gates MISSED their mutation" as a finding about a gate and
started looking for the gate that had stopped catching. The gate was fine.
The script had stopped noticing.

### Fixed, and the fix is itself tested

`swap` now stops the run:

```bash
if ! python3 - "$1" "$2" "$3" <<'PYSWAP'
…
PYSWAP
then
    printf 'THE MUTATION DID NOT APPLY, SO THE RUN IS VOID.\n' >&2
    exit 1
fi
```

and `--self-test` has a second half that drives a deliberately non-applying
mutation and fails if the run is *not* stopped. The first half proves a
missed mutation fails the script; the second proves a broken mutation fails
it too. With the guard removed, the self-test exits 2 and says so.

### The lesson

**A guard that prints a loud message and then lets the run continue is worse
than no guard.** The message is read as a warning and the result under it as
a finding, and the finding is about the wrong thing. The instrument's own
failure modes have to be *fatal*, not advisory — and they have to be tested,
because a guard that only prints is a guard that only prints.

This is §2's shape one level down. There, `cargo xtask ci` printed "ci
green — …, tests, wasm" and exited 0 without running the tests: a check
that *reported* rather than *stopped*. Here, a check printed the truth and
let the run carry on to print a falsehood.

---

## 47. The crash report replayed into a different document, and said nothing

A crash report exists to be a **reproduction**: the design as it was, and the
commands that produced it. The engine's module comment says so —
*"**reproduction**: the design as it was, and the commands that produced it"* —
and `docs/recipes/fix-a-crash.md` is built on the same claim.

The report had the design and the commands. It did not have the **starting
point**, and it did not have a way to say "this replay did not reproduce".

**Four separate ways existed for the report to be wrong about the session it
claims to reproduce. None of them said a word.** The first three made the
*replay* land somewhere else; the fourth made the *session* itself wrong, and
the report faithfully recorded a document nobody had.

### The measurements

Each of these was measured with a probe against the real engine, not reasoned
about.

**1. A long parameter became an empty one.** The trace was a line of text —
`format!("{id} {}", crate::log::compact(params))` — and `compact` cut it at
200 characters. A node name of 250 characters (the limit is
`limits::MAX_NAME`, 255) produced the line

```text
node.add {"kind":"source","name":"xxx…
```

which is not JSON. `parse_trace` parsed it, failed, and fell back to
`serde_json::Value::Object(Default::default())` — **an empty parameter
object**. The replay then ran `node.add` with *no parameters at all*, and
`node.add` without a name succeeds and invents one. The replayed document had
a node called `filter 1` where the crashed one had a 250-character name.

The fallback was not an accident; it was pinned by a test called
`a_trace_line_that_is_not_json_still_replays_as_a_step`, which asserted
`params == {}` as the *correct* behaviour.

**2. The trace was a ring of sixty-four.** `MAX_TRACE` was 64, and
`execute` dropped the oldest line when the ring was full. After 70
`node.add` commands the trace held 64; the replay rebuilt 64 nodes where the
session had 70, and its first node was `n6` where the original's was `n0`.
The report did not say that six commands were missing.

**3. Opening a document left the trace behind.** `replace_document` — the
path every file-open went through — restored the document and nothing else. A
session that opened a document with five nodes in it and then ran one command
had a trace of exactly one line. Replaying that line into a fresh session
produced **one** node where the report recorded **five**. The file's contents
were simply not there, and the report claimed to reproduce the session that
had them.

**4. Opening a document left the undo stack behind.** Measured this session: a
session that had added a node and then opened a three-node document still had
the old snapshot on its undo stack. The first Ctrl+Z answered *"undid Add
node"* and left **zero** nodes — the empty document, not the file's. The undo
stack belonged to the document that had been there before.

**A verified negative, and the reason the other four survived.** Undo and
redo *do* replay correctly: `node.add A`, `node.add B`, `history.undo` gives
one node in the original and one node in the replay, equal. The reason is
that the undo and redo commands are themselves in the trace, so the replay's
own undo stack does the same work. That accident is why the trace looked
right in every test anybody wrote: the sessions those tests used had never
opened a file, never used a long name, and never run more than sixty-four
commands.

### The fix

**The trace is structured, so it cannot be lossy.** `TraceStep` is a command
id and the parameters *as they were given* — a `serde_json::Value`, not text.
There is nothing to truncate and nothing to parse back, so the `{}` fallback
is gone rather than fixed, and `log::compact` is gone with it. The test that
pinned the fallback is gone with that too.

**The trace is bounded by bytes, and it says what it dropped.** `MAX_TRACE` is
now `MAX_TRACE_BYTES`, and the report carries `dropped`: the number of
commands the bound made it leave behind. A partial replay now says it is
partial.

**The report carries the base.** `CrashReport` has a `base` field: the
document the recorded commands were relative to. The replay starts there, so
a session that opened a file replays *onto that file*. The report also
carries the playhead, because the plan depends on it.

**The replay has a verdict.** `crash::reproduce` starts from the base, runs
the steps, and then **compares the result with the design the report
recorded**. It answers `reproduced`, or it lists the differences — the node
that is named differently, the id that does not match, the count that is
short. `craftkit-cli crashes replay <id>` prints the verdict; the control
channel's `diagnostics.crash_reports` returns `reproduced` and `differences`.

**Opening a document is a distinct operation.** `Session::load_document` is
what the three file-open sites call now. It clears the undo stack and the log
— they belong to the document that was there before — and restarts the trace
from the document it loaded. `Session::replace_document` stays as the
restore-only path that undo and redo use.

**And the reproduction can be rendered.** `craftkit-cli crashes replay <id>
--script` prints the replay as a script *with its base*, and the script
format learned a `base` field, so `snapshot` can draw the crash that
happened. This is the workflow `skills/craftkit-checking/SKILL.md` claimed
and the CLI did not have.

### One detail worth keeping

The base document is read with `crash::document_from_json`, which calls
`reseat_id_counter` — and it has to. A document's id counter is **derived**
from the ids in it, not stored, so a document read back from JSON hands out
id 0 for its first new node. The first version of the replay failed with
*"there is no node with id 2"*: the base was empty, the counter had been
reset, the replay's first node took id 0, and every later step named the
wrong node. The rule is already written down in the file reader
(`crates/io/src/format.rs`), which is where the model keeps it. Every place
that reads a document now reseats it — checked, not assumed.

### The lesson

**A silent divergence is worse than an error.** Every one of these four
defects could have been an error — "the replay needs a document it does not
have", "these parameters were truncated", "this report covers 64 of your 70
commands". None of them was, because the pipeline's contract was "run these
commands and you have the design". It is not enough for the commands to run.
They have to *arrive*.

This is the same shape as §32, one layer down. There, `ui.click` answered
`clicked` when it had only rendered; here, `crashes replay` answered "N
step(s) replayed" when it had rebuilt something else. A message that claims
more than happened is the defect, whatever the message is about.

### What now holds it

Seven mutations in `scripts/mutation-test-gates.sh`, each of which breaks one
of the properties above and requires a test to notice:

| Mutation | Caught by |
|---|---|
| the verdict always says "reproduced" | `a_replay_says_whether_it_reproduced_the_document` |
| the replay starts from an empty session | `a_session_that_opened_a_document_replays_onto_it` |
| the trace keeps the parameters | `the_trace_keeps_the_parameters_exactly` |
| opening a document does not restart the trace | `opening_a_document_starts_the_trace_again` |
| opening a document keeps the old undo stack | `opening_a_document_clears_the_undo_stack` |
| the CLI calls a divergence a reproduction | `a_replay_that_does_not_match_the_report_says_so` |
| a dropped command is not counted | `a_long_session_reports_how_many_commands_it_dropped` |

---

## 48. The crash report was read as trusted input, and it was executed

§47 made the crash report honest. It also made it **input**: the replay now
starts from a document the report carries and runs the report's commands. And
`docs/recipes/fix-a-crash.md` has always said where reports come from — when
the crash happened on somebody else's machine, it asks for the report file —
so a report is expected to arrive from **another machine**. The file the app
writes and the file the CLI reads are the same file, and only the writer's
assumptions had ever been checked.

### Three measurements

**1. There was no size bound.** The tree has two other readers of the same
kind, and both cap themselves:

| Reader | Bound | Where |
|---|---|---|
| a design file | 64 MiB | `crates/io/src/format.rs` — *"a hostile or truncated file must not reach `serde_json::from_str` with a megabyte of input"* |
| a script file | 32 MiB | `apps/craftkit-cli/src/cli.rs` |
| **a crash report** | **none** | `crates/engine/src/crash.rs` |

Measured: a **104 857 690-byte** `report.json` was read in **105 ms** without
a word of complaint.

**2. The base was not validated.** The design reader refuses a document whose
nodes share an id — *"two objects share an id: two nodes share an id"* — and
the crash reader accepted the same document. The consequence is not
theoretical: a step that named the duplicated id (`node.mute {"node": 1}`)
**muted the first of the two nodes**, and the replay reported no failure.

**3. The report is executed, and the cost is quadratic.** Every command clones
the document, so a replay of `n` commands that each add a node copies O(n²)
nodes. Measured, with the real engine:

| steps | time | per step |
|---|---|---|
| 500 | 0.82 s | 1.63 ms |
| 1 000 | 3.25 s | 3.25 ms |
| 2 000 | 13.13 s | 6.56 ms |
| 4 000 | 55.58 s | 13.90 ms |

Each doubling of the steps doubles the per-step cost, which is the signature
of the square. A report big enough to hold two million steps — and 64 MiB
holds about two million — would run for **years**.

### What §47 already did for it

The wrong-node edit of measurement 2 was **not silent**, and that is worth
recording: §47's verdict compared the replayed document with the recorded one
and answered `reproduced: false`, with the differences listed — *"node 1 is `A`
in the report and `Level` in the replay"*. The verdict is what turned a
corrupt report from a wrong answer into a reported one.

It did not make the reader safe, and it did not make the *cost* bounded.

### The fix

- **`MAX_REPORT_BYTES`**, 64 MiB — the design reader's own number, for the
  same kind of file, and the reader now applies it **while reading**: `take`
  cuts the file off at the limit, so a ten-gigabyte report costs one buffer
  rather than ten gigabytes of memory.
- **`MAX_REPLAY_STEPS`**, 20 000 — the trace a session keeps is bounded at
  about ten thousand steps, so a report with more was not written by this
  tree. The check happens **before the first step runs**, because the cost is
  in the running.
- **The base is validated**, exactly as the design reader validates a design.
  A corrupt base is refused with the reason it is corrupt.

### The lesson

**A file the tool writes is not the file the tool reads.** The report is
written by the desktop app, which knows exactly what it put in; it is read by
the CLI, the control channel and the MCP server, which know nothing about
whoever wrote it. The reader is the side that has to cope with anything — and
it was the side with no bounds and no validation, in a tree where the other
two readers have both.

The three measurements have one shape: each of them is a place where the
*writer* was trusted. The trace was trusted to be short (§47's ring), the
parameters were trusted to be small (§47's truncation), and now the report is
trusted to be small, well-formed and cheap. Trusting the writer is fine until
the file leaves the machine — and this one is meant to.

### What now holds it

Three more mutations, each breaking one bound:

| Mutation | Caught by |
|---|---|
| the report file is read with no bound | `a_report_larger_than_the_limit_is_refused` |
| the replay runs as many steps as the report holds | `a_replay_with_too_many_steps_is_refused_before_it_runs` |
| a corrupt base is replayed anyway | `a_report_with_a_corrupt_base_is_refused` |

---

## 49. Every limit in the tree was checked after the bytes were already in memory

The tree is careful about hostile input. `crates/control/src/protocol.rs` says
it best:

> A cap is not politeness. Without one, one line from a client that has gone
> wrong fills memory until the process is killed — and this process holds the
> user's document.

That cap is there, that reasoning is there, and the memory was spent anyway.
The cap was checked in the **parser**: the line was read, the file was read,
and only then did the code look at how long it was. By the time the refusal
was written, the allocation it was meant to prevent had already happened.

### The measurements

Three readers, each measured with the real program and the real input, before
and after:

| Reader | Input | Refused? | Peak RSS before | Peak RSS after |
|---|---|---|---|---|
| the design file (`craftkit_io::load`) | a 300 MiB file | yes | **300 MiB** | 64 MiB |
| the MCP server (`stdin`) | a 200 MiB line | yes | **203 MiB** | 7.7 MiB |
| the control channel (a socket) | a 64 MiB line | yes | **66 MiB** | 3.4 MiB |

Every refusal was **correct and specific**:

- *"the line is 209715208 bytes; the limit is 4194304"*
- *"the file is 314572800 bytes, and the limit is 67108864"*

The cap did its job. It just did it after the memory was gone.

### The fix

One implementation, in the layer whose own module comment says it holds *"the
readers that turn hostile input into typed values"*: `craftkit_core::readers`.

- **`read_to_string_bounded`** — reads an input whole and refuses one over the
  limit. The reader is *cut off* at the limit (`take`), so the input is never
  read into memory in the first place.
- **`read_file_bounded`** — the same, for a file, and it reports the file's
  own size.
- **`read_line_bounded`** — reads one line, keeps at most `max` bytes of it,
  and drops the rest. It still reads the rest of the line, so a server that
  carries on after refusing one stays in step with its client.

Five call sites now use them: the design file, the script file, the crash
report, the MCP server and the control channel — and the crash report, which
§48 had already fixed on its own.

### The lesson

**A cap in the parser is not a cap on the memory.** The distinction is the
whole difference between *"the file is 300 MiB and the limit is 64 MiB"* —
written after reading 300 MiB — and refusing the file without reading it. The
first is a message; the second is a limit.

There is a second lesson, and it is the one that cost the tree four readers:
**the reasoning was right and the placement was wrong, in four places at
once.** The comment above `MAX_LINE` describes the failure *exactly* — a line
from a client that has gone wrong filling memory until the process is killed —
and the code did that anyway, because the check sat one function too far
down. A cap written next to the parser reads like the parser's business. It
is the reader's business.

### What now holds it

Two mutations, and one test that can see the property exactly:

| Mutation | Caught by |
|---|---|
| the bounded read is not bounded | `a_bounded_read_never_asks_for_more_than_the_limit` |
| a long line is kept instead of dropped | `the_bytes_kept_never_exceed_the_limit` |

The first test uses a reader that counts the bytes it hands out, so it can
assert that the reader was **never asked** for more than the limit — which is
the property, and which no amount of peak-memory measuring could pin down
reliably.

The second one was **missed** the first time, and the miss is worth
recording. The mutation broke a branch that decided whether the bytes were
dropped — and the test still passed, because the *error* was the same either
way. The difference was in the memory, and the test was looking at the error.
The mutation now moves the property the test can see: the size of the buffer
the reader fills. A test that cannot fail is a test that has not been asked
the right question, and the instrument saying so is the whole point of
running it.

---

## 50. The engine could write a file its own reader refused

The writer and the reader are two functions in one crate, a few lines apart:
`craftkit_io::save_text` and `craftkit_io::load_text`. One turns a document
into text; the other turns text back into a document. The reader **validates**
what it gets — it has to, because the text may come from a file on disk or
from a browser's storage. The writer did not.

So the tree could produce a document that was fine in memory and meaningless
on disk. Two ordinary commands could each do it.

### The measurements

Both measured with the real engine, by reverting the four fixes and running the
real code — not by reading it.

**The group id that had to be unique in the whole tree.** Two commands:

```
prop.set  { node: 1, path: "a/trigger/lead", value: 1 }   →  Ok
prop.set  { node: 1, path: "trigger/lead", value: 2 }     →  Ok
```

The first creates a group `a` holding a group `trigger`; the second creates a
group `trigger` at the top level. Two groups, the same id, two depths. Both
commands reported success.

Then the document was asked whether it was valid, and it said no:

```
Err(Invalid("node n1: limit reached: duplicate group id"))
```

Then the writer was asked to save it, and it saved it:

```
WROTE 7134 bytes
```

And the reader was handed those 7 134 bytes back:

```
the document in this file is not valid: invalid: node n1: limit reached: duplicate group id
```

A file the app wrote, that the app could not open. The web app stashes through
the same two functions (`apps/craftkit-web/src/main.rs` — `stash_text` calls
`save_text`, `restore` calls `load_text`), so the same plan, in a browser, was
a reload away from being lost.

**A range of zero.** One command:

```
doc.range { start: 0, end: 0 }   →  Ok("range 0.000s..0.000s at 24 fps")
```

and the document it left behind:

```
Err(BadMeta("the document is empty: start 0t is not before end 0t"))
```

A successful command, a plan that cannot be saved, and no hint that the two
things are related.

### The rule was stricter than its reason

The rule that refused the first document was the model's own, and its comment
explains it:

> A group id must be unique, because two groups with the same id make `#2` and
> `dup#1` ambiguous about which one they name.

Two groups with the same id **under one parent**. The old rule did not say
that — it compared every group id in the whole tree. And the path scheme
disagrees with it in as many words, in the same file:

> `flatten` counts the occurrences of a name **per level**, because *"`trigger/x`
> and `trigger/trigger/y` are not competing for the same name space at all"*.

So the rule was stricter than the reason it gave, and `prop.set` — which
creates groups one level at a time — could cross it on purpose. A rule stricter
than its stated reason, reachable by an ordinary command, is not a rule. It is
a trap, and the trap was in the direction that loses work.

### The fix

Three changes, at three levels, because each of them closes a different hole.

**The model** — the duplicate-group-id rule is now scoped to **siblings**, and
compares case-insensitively, because the resolver's does
(`index_of_seg` matches with `eq_ignore_ascii_case`): two siblings called
`Trigger` and `trigger` would always resolve to the first, which is exactly
the ambiguity the rule exists for.

**The writer** — `save_text` validates the document before writing it. A file
that cannot be loaded back is worse than no file at all: it looks like a save,
it takes the name the plan should have, and the refusal comes later, when the
work is already out of sight.

**The engine** — `Session::execute` validates after every mutating command and
rolls the snapshot back if the document no longer validates. The check is in
one place rather than in each command, so the next command cannot forget it.

And `doc.range` refuses `end <= start` with a parameter error, before it can
leave an empty document behind.

### What the backstop alone would have done

Worth measuring, because it is the difference between *losing work* and *being
inconvenienced*. With the backstop in place and the old rule still in the
model, the second `prop.set` is refused:

```
Err(Internal { id: "prop.set", detail: "the command left the document in a state the file
              format refuses: invalid: node n1: limit reached: duplicate group id" })
```

and the document is rolled back — `validate` is `Ok(())` afterwards, and
`doc.range { start: 0, end: 0 }` is refused for the same reason.

So the backstop alone stops the bad file. But it also **refuses a legitimate
command**: the user's `trigger/lead` is rejected because of a rule that is
stricter than its reason. That is why the model was fixed too, and not just
the writer.

After the fix, the same two commands succeed, the document validates, the file
is 7 134 bytes — **the same size as the broken one** — and it reads back.

### The lesson

**A file the tool writes is not the file the tool reads.** They are two
functions with two sets of assumptions, and the gap between them is where the
work disappears. The reader validates because it cannot trust the input; the
writer did not validate because it *did* trust the input — it had just built
it. Trusting your own output is the most expensive kind of trust.

The second lesson is older and sharper: **a model's rule must not be stricter
than its stated reason.** The reason existed to stop `#2` from being
ambiguous, and it was enforcing something else — a uniqueness constraint the
scheme had already decided not to have. The check was not wrong about
ambiguity; it was answering a question nobody had asked.

### What now holds it

Five mutations, and six tests that can see the difference:

| Mutation | Caught by |
|---|---|
| a command may leave a document the file reader refuses | `leaves_a_document_the_file_would_refuse` |
| two groups with one id are allowed among siblings | `two_siblings_with_the_same_id_are_refused` |
| the same group id at two depths is refused again | `a_group_id_may_repeat_at_a_different_depth` |
| a group is created under a name the path does not name | `a_group_name_at_two_depths_survives_the_file` |
| the writer writes a document the reader would refuse | `the_writer_refuses_a_document_the_reader_would_refuse` |

The third of those restores the **old rule exactly** — the group ids threaded
back through the recursion, the check tree-wide again — so it is not a
hypothetical mutation but the defect itself, put back for the test to catch.

The third of those was **missed** the first time, and the miss is the useful
part. The mutation changed the group's *label* and left its *id* alone — and
the test still passed, because the resolver matches on the id. A mutation that
changes the wrong thing measures nothing, and the script reported it as
*caught* only because the mutation had broken something else. The mutation now
moves the id, and the test fails with the group's real structure in the
message:

```
the top-level group is missing: PropGroup { id: "props", …,
  Group(PropGroup { id: "a#2", label: "a", nodes: [Group(PropGroup { id: "trigger#2", …
```

---

## 51. The writer had no limit, and the reader did

§49 gave the reader a limit — 64 MiB, applied *while* reading. The writer
never learned about it. So the tree could write a file that the tree could not
open: §50's shape, one level up, and §50's fix did not touch it, because §50
was about a document being *invalid* and this is about it being *too big*.

### The measurements

A document the model's own limits allow: one node, 32 properties, 10 000
keyframes on each. `MAX_PROPERTIES` is 50 000 per node and `MAX_KEYS` is
10 000 per property, so this is inside every limit the tree has. Serialised,
it is **74 820 332 bytes**.

Then:

```
save      -> Ok(())      // 74 820 332 bytes on disk
load      -> the file is 74820332 bytes, and the limit is 67108864
load_text -> the file is 74820332 bytes, and the limit is 67108864
```

A file the crate wrote, that the crate could not open. The desktop app has no
save path yet (§33), so the reachable surface today is the web app's stash —
which uses the same `save_text` — and there the browser's own storage quota
hid the defect by accident. A 74 MiB stash is refused by a browser long
before it reaches the disk; a 74 MiB file is refused by nothing at all.

### The fix

**`File::to_text` now writes *through* a limit.** A `LimitedWriter` refuses
any chunk past `MAX_FILE_BYTES`, so the text is never more than the limit in
memory and the file is never bigger than the reader will take. `to_text_bounded(max)`
is the general form; `to_text` uses `MAX_FILE_BYTES`.

The refusal is a new error variant, `FormatError::TooLargeToWrite`, because
the message has to be true: there is no file yet, and *"the file is N bytes"*
about a file that was never written is the kind of small lie this tree does
not tell. It says:

```
the file would be more than 67108865 bytes, and the limit is 67108864
```

The count is a **lower bound**, and the wording says so: the writer stopped
there rather than finishing the text to find out how big it really was.

`save_text` inherits the refusal, `save` inherits it, and **no file is
written** — the refusal happens before the temporary file is created, so
there is nothing to clean up and nothing half-written:

```
save_text -> the file would be more than 67108865 bytes, and the limit is 67108864
save      -> the file would be more than 67108865 bytes, and the limit is 67108864
file on disk -> None
```

A document that *is* under the limit still saves and loads, because a limit
that refuses everything is not a limit, it is a wall. There is a test for
that half too.

### The lesson

**A limit belongs to both ends of a pipe.** §49 taught that a limit checked
after the bytes are in memory is not a limit at all. This is the same lesson
from the other end: a limit on one end of a pipe is a limit on *both*, or it
is a half-measure that only ever surprises the side that has to cope with
it. The reader's 64 MiB cap was never a safety feature — it was a rule the
writer was breaking, every time it wrote a big enough plan.

And the second lesson is about the shape of the work: **the writer and the
reader are one contract with two halves.** §50 fixed the validity half — the
writer must not write a document the reader refuses — and §51 fixed the size
half. Two defects, one contract, and the second was found only because the
first had made the contract explicit. A defect you have already written down
the rule for is much easier to find than one you have not.

### What is still open

The writer now refuses a document too big to save, so no work is lost to a
file that cannot be opened. But the **engine can still hold** such a
document: nothing stops a command from growing the plan past what can be
written, and the engine keeps 128 copies of it for undo. That is §52, and it
is a different kind of limit — a budget rather than a cap.

### What now holds it

Two mutations, and three tests:

| Mutation | Caught by |
|---|---|
| the writer writes a file too large to read back | `a_document_too_large_to_read_back_is_not_written` |
| the writer keeps the bytes it refused | `the_writer_keeps_no_more_than_the_limit` |

The second one is the interesting one, and it is §49's lesson turned into a
mutation. The first version of the fix serialised the whole text and checked
its length afterwards — correct, and useless: the 74 MiB had already been
built. The mutation keeps the chunk the writer refused, and the test says
exactly what it saw:

```
the writer kept 16 bytes of a 10-byte limit
```

---

## 52. The undo stack kept 128 copies of a document nobody had measured

The history module says what its design costs:

> The cost is a clone per mutating command on a document of at most
> `MAX_NODES` nodes — bounded, predictable, and profiled rather than guessed.

It was none of the three. `MAX_NODES` is a count of nodes, not a number of
bytes, and the product of the model's counts — 10 000 nodes × 50 000
properties × 10 000 keyframes — is a number no machine has. Nothing had ever
measured it, because measuring it was not something the tree did.

### The measurements

A document the model's own limits allow: one node, 20 properties, 10 000
keyframes on each — 200 000 keyframes, about 12 MiB of memory. Then 128
mutating commands, which is what the history is there for.

| | before | after |
|---|---|---|
| resident size with the document loaded | 25 748 KiB | 25 752 KiB |
| resident size after 128 mutating commands | **1 638 844 KiB** (1.56 GiB) | **88 560 KiB** |
| undo steps kept | 128 | 3 |
| resident size after undoing all 128 | 1 664 436 KiB | 112 784 KiB |

**1.56 GiB** for a 12 MiB plan, and **undoing every step did not give it
back** — the snapshots moved to the redo stack, so "undo everything" freed
nothing at all. The one thing a person reaches for when the app is heavy was
the one thing that did not help.

### The fix

Three parts, because "the document is too big" is three different questions.

**An estimate.** `Document::estimated_bytes()` walks the document and counts
what it holds: the structures, the strings' *capacities* rather than their
lengths, and every keyframe and value. It never under-estimates — a budget a
document can lie about is not a budget. For the 12 MiB document it says
19 207 535 bytes, about 1.5× the real thing: conservative, and the direction
that matters.

**A limit on the document.** `limits::MAX_DOCUMENT_BYTES` (32 MiB) is checked
in `Document::validate()`, which is the check the engine already runs after
every mutating command (§50) and the check the reader and the writer already
run. So a command that grows the plan past the limit is *refused and rolled
back*, a file that holds one is refused, and the number is one number in one
place.

**A budget on the history.** `MAX_HISTORY_BYTES` (64 MiB) bounds the undo and
redo stacks *together*, in bytes. The count of steps (`MAX_ENTRIES`) is still
there for the small-document case, where 128 steps cost nothing; when the
document is large the budget binds first and the oldest steps go. Which is
what a budget means: the stack keeps as much undo as it can afford.

> **§68: this paragraph was the intention, and the code did not do it.** The
> budget counted the memory the stacks hold together, and the one operation
> that *frees* memory — throwing the redo stack away when a new edit arrives
> — did not give it back. So the budget was enforced against a phantom, and
> on a full-size document it emptied the undo stack: undo stopped working
> after twenty-one edit/undo cycles. A section that verifies a mechanism and
> then describes it is still a description, and this one described a mechanism
> that was not finished.

### The lesson

**A count is not a bound on memory.** Every limit in the model was a count,
and each one was correct about what it counted. What none of them said was
what the thing counted *held* — and the history multiplied that silence by
128. The limit that was missing was not a bigger limit on nodes or
keyframes; it was a limit in the unit the resource is actually spent in.

The second lesson is about the sentence: *"bounded, predictable, and profiled
rather than guessed"* was a claim about work that had been done. It had not
been. A comment that says a thing was profiled is a promise to the next
reader, and the only way to keep it is to profile it — which takes one
measurement, and takes ten minutes.

### What is still open

The engine still clones the whole document for **every** command, including
the read-only ones, because the snapshot is what restores the design if a
command fails or panics. Measured on the 19 MiB document: the clone is
**205 µs**, and twenty cheap commands through the engine take 38 ms — so
about 1.9 ms of engine overhead per command.

> **§103 corrected the last two sentences of that paragraph.** "That is the
> price of the restore guarantee" pointed at the clone, which is a seventh of
> the overhead: a command walks the whole document **four** times — the
> snapshot, the comparison, the validation and the size estimate — and the
> other three were added after this section was written and never measured.
> On a document at the model's limits a mutating command costs **23.8 ms**,
> which is more than a frame at sixty frames a second. §103 has the
> measurement and names the four.

That is the price of the restore guarantee, and it is the right price: a
snapshot cannot be subtly wrong, which is the whole reason the design is
snapshots rather than inverted commands. It is also only a seventh of the
overhead, and §103 has the rest.

The 108 ms that first showed up in this section was **not** the clone. It was
`plan.evaluate` doing a full, uncached evaluation of the whole document on
every call — which turned out to be §53's subject, and which now costs 5 ms
instead of 102 ms.

The fix that would remove the clone's 205 µs is copy-on-write: share the
property tree between the snapshot and the live document and clone only the
branch a command writes. That is a redesign of the model's ownership rules,
not a patch, and it is the kind of change that should be a decision rather
than an accident. It is recorded here rather than done.

### What now holds it

Three mutations, and three tests:

| Mutation | Caught by |
|---|---|
| a document over the byte limit is accepted | `a_document_over_the_byte_limit_is_refused` |
| the size estimate does not count the content | `the_history_keeps_its_memory_within_a_budget` |
| the undo stack ignores its memory budget | `the_history_keeps_its_memory_within_a_budget` |

The second one was **missed** the first time, and the miss is the useful
part. The mutation zeroed `Value::estimated_bytes` — so the estimate
under-counted — and the test still passed, because an under-estimate is still
an estimate and the budget still bound. The count that decides the budget is
the keyframes' count, and that is what the mutation moves now. The test says
what it saw:

```
the budget should have bound this before the count did: 128 steps
```

---

## 53. The evaluator's cache cost 7 400 times more than the work it avoided

The evaluator keeps the last plan and reuses the states of nodes that have
not changed. "Has not changed" is answered by a **fingerprint** — a hash of
the node — and the fingerprint used to be *the node's serialised form*:
`serde_json::to_string(&node.props)`, hashed.

The code said why, and it was right at the time:

> A hand-written structural hash would be faster, and would have to be
> updated by hand every time the model grows a field. A forgotten update is a
> **silent wrong answer**: a stale state the engine believes. Hashing the
> `serde_json` form cannot drift. Correctness first; if this ever shows up in
> a profile, move it to a single `Node::fingerprint` and keep it there.

This is that profile.

### The measurements

One node, 20 properties, 10 000 keyframes on each — 200 000 keyframes, a
document the model's own limits allow.

| | before | after |
|---|---|---|
| the work the cache avoids (`resolve_props`) | 7 µs | 7 µs |
| the cost of the cache (one fingerprint) | 52.1 ms | **5.0 ms** |
| a cold evaluation | 101.6 ms | **5.0 ms** |
| a warm evaluation — everything reused, nothing evaluated | 102.6 ms | **5.0 ms** |

The two lines that matter are the last two. A **warm** evaluation, where
every node was reused and *no node was evaluated at all*, cost exactly as
much as a cold one — because the fingerprint was computed **twice** per
evaluation (once to decide whether the state could be reused, once again when
the cache was rebuilt) and each one was a full JSON serialisation of the
node's 12 MiB of properties.

So the cache cost **7 400 times** more than the work it avoided, and a cache
that costs more than it saves is not a cache; it is a tax on every
evaluation, taken whether or not anything was reused.

### The fix

Two changes, and the tree had already decided both.

**The fingerprint is structural.** `Node::fingerprint` now lives in the
model, next to the data it hashes, and walks the fields directly: no
allocation, no JSON, 5 ms instead of 52 ms. The comment above it had already
named this as the answer — *"if this ever shows up in a profile, move it to a
single `Node::fingerprint` and keep it there"* — and this is that profile.

**The fingerprint is computed once.** The cache rebuild reuses the
fingerprint the reuse check already computed.

The evaluation of that document is **20 times faster**, and the cost is now
in the right place: a plan of 40 ordinary nodes evaluates in 68 µs, and a
warm evaluation of the same plan is 33 µs, so the cache does what it was
written to do.

### The risk, and what holds it down

A hand-written hash is the thing the old comment was afraid of, and the
fear was right: a field the model grows and the hash forgets would let the
evaluator serve a state computed from the old value — a silent wrong answer,
with nothing anywhere reporting a problem.

So the risk is held down by a test that sits next to the hash:
`the_fingerprint_sees_every_part_of_the_node` changes each part of a node in
turn — the id, the name, the kind, the muted flag, a group's id and label, a
property's id, label, value, locked flag, hint and keyframes — and requires
the fingerprint to move every time.

And the part nothing checked at all: **no test in the tree changed a
*keyframe* and then re-evaluated.** The other cache tests change a property's
value or move a node; the keyframes are the part of a node a hand-written
hash is most likely to forget, and they were the part with no test. There is
one now:
`a_node_whose_keyframes_changed_is_not_served_from_the_cache`.

### The lesson

**Measure the thing you are optimising against the thing you are paying.**
The cache was written to avoid re-evaluating nodes, and it was measured
against nothing; the profile it asked for took ten minutes and showed that
the mechanism cost 7 400 times the work. A cache that has never been timed
is a belief.

And the second lesson is about the comment: it said *"if this ever shows up
in a profile"* — and the profile was the only thing that could have told the
tree the trade had gone bad. The comment was not wrong; it was **waiting for
a measurement that nobody had gone to take**. The engine's own promise, in
§52, was the same shape: *"bounded, predictable, and profiled rather than
guessed"*.

### What now holds it

Three mutations, and the two that already existed moved with the code:

| Mutation | Caught by |
|---|---|
| the fingerprint stops looking at the node's properties | the evaluator's own tests |
| the fingerprint stops looking at the keyframes | `a_node_whose_keyframes_changed_is_not_served_from_the_cache` |
| the node's name stops being part of the fingerprint | `a_restore_cannot_serve_a_state_from_the_document_it_replaced` |

The middle one is new, and it is the one that matters: it is the mutation
the old code could not express, because a serialisation-based fingerprint
*cannot* forget the keyframes. The risk arrived with the fix, and so did the
test that holds it down.

### The prose was still describing the old fingerprint

Two documents said the fingerprint was *"FNV-1a over the document's serde
form"*: `docs/design/file-format.md` and `docs/architecture.md`. The second
went further and explained why — *"Hashing the serialization cannot drift"* —
which is the argument the fix had just given up on purpose.

Nothing noticed. `cargo xtask lists` checks that prose **points at** code that
exists — a file, a line, a symbol, a test — and both documents pointed at
things that all still exist. What they no longer *said* was not checked,
because checking what prose says is the heuristic this tree does not do
(§29). The only thing that found it was reading the documents after changing
the code, which is a thing a person does and a gate does not.

So: **a gate that checks that prose points at code is not a gate that checks
that prose describes code.** The two are different questions, and only the
first one has a cheap mechanical answer. The second one still depends on
somebody reading the document after the code moves — which is why the
documents are part of the change and not part of the aftermath.

---

## 54. The second instance took the first one's token file

The control channel is how an agent drives the running application. The token
is not passed out of band: the app writes it to a file, and the agent reads
the file. The file is the promise — *this port answers to this token* — and
the app wrote it **before** it knew whether it held the port.

### The measurements

Two instances, the second one on the port the first one already had:

```text
$ craftkit --sample --control 47766        # the first instance
control channel listening on 127.0.0.1:47766
token in /home/…/craftkit/control/47766.token

$ craftkit --sample --control 47766        # the second instance
control channel: could not bind 127.0.0.1:47766: Address already in use (os error 98)
$ echo $?
1
```

The second instance failed, said so, and exited. And it had already
overwritten the first instance's token file on its way to the bind:

```text
A's token:    e776f56747069df6…
token file:   5d170a412cb47724…   (changed: YES)
```

So the first instance was still running, its window was still there, and the
token in the file was the **second** instance's — a token belonging to a
process that had already exited. An agent that did what the documentation
told it to do — read the file, connect, send — got:

```text
control channel: rejected a connection with a bad token
```

…from an application that was working perfectly, on a port that was open.
The failure looks like a security rejection, so the first thing anybody
reasons about is *"is someone else using my token?"* — and the answer is no:
the token is the app's own, written by a copy of the app that quit.

### The fix

**The channel is bound before the token is published.** The order is the
whole contract: a promise is not made for a port this process does not hold.
`craftkit_control::serve_published` does both, in that order, and the app
calls it.

The file is also named for the port that was **bound**, not the one that was
asked for. With `--control 0` the operating system picks the port, and the
old code wrote a file called `0.token` — a promise about nothing.

After the fix, the same two instances leave the first one reachable:

```text
token file: e776f56747069df6…   (changed: NO)
the running app with the token from the file ->
  {"id":1,"ok":true,"message":"0 command(s), 0 failure(s)","data":{…,"nodes":3,…}}
```

### The escape hatch that did not exist

The reason no test caught this is worth recording, because it is the more
general failure.

`token_path`'s own comment said:

> `CRAFTKIT_CONTROL_DIR` overrides it, which is what lets a test run entirely
> inside a temporary directory.

It cannot. Setting an environment variable is `unsafe` in edition 2024, and
the crate forbids `unsafe` — so **no test in that crate can use the escape
hatch its own documentation advertises.** The mechanism was described, it
worked for a launch script, and it was not reachable from a test. The test
that would have caught the takeover could not have been written the way the
documentation said to write it, so it was not written.

So the fix also adds what a test can actually use: `ServerOptions::with_token_dir`,
a per-server token directory. The environment variable is for *launching* the
app; the option is for *testing* it, and the comment now says so.

### The lesson

**A promise is made after the thing it is about has happened.** The token
file says "this port answers to this token", and it was written before anyone
knew whether this process held the port. Every ordering that matters in this
tree is the same ordering: the crash report is written when the process
dies, the document is validated before it is written, the file is validated
before it is loaded, the bound is taken before the token is published.

And the second lesson is about the escape hatch: **a documented mechanism
that cannot be reached is worse than no mechanism**, because it explains why
the test was not written. The comment was not wrong about the environment
variable — it was wrong about *who could set it*, and that difference is
exactly the difference between "there is a way to test this" and "there is
not".

### What now holds it

One mutation, and one test:

| Mutation | Caught by |
|---|---|
| the token is published before the port is bound | `a_second_instance_does_not_take_the_first_ones_token_file` |

The mutation is the old order, and the test says what it saw:

```
assertion `left == right` failed: the second instance took the first one's token file
```

---

## 55. The tool changed the permissions of a directory it did not create

The control channel's token file is written by `craftkit_control::write_token`.
The file's mode is `0600`. The **directory's** mode is `0700` — and the code
that set it never asked whether the tool had made that directory.

### The measurements

`CRAFTKIT_CONTROL_DIR` relocates the token file. It is documented in three
places, and every one of them says only *where the file goes*:

- `skills/craftkit-checking/SKILL.md`: "`CRAFTKIT_CONTROL_DIR` relocates the
  folder — useful for a launch script."
- `docs/recipes/add-a-control-channel-method.md`: "`CRAFTKIT_CONTROL_DIR`
  relocates the token file."
- `crates/control/src/token.rs`: the variable's own doc comment.

Point it at a directory that already exists:

```text
$ mkdir -p /tmp/shared && chmod 755 /tmp/shared
$ CRAFTKIT_CONTROL_DIR=/tmp/shared craftkit --sample --control 47772
control channel listening on 127.0.0.1:47772
token in /tmp/shared/47772.token

$ stat -c '%a %A' /tmp/shared
700 drwx------
```

The directory went from `0755` to `0700`. The application printed nothing
about it, logged nothing about it, and returned success.

With a `setgid` bit — the usual mark of a directory whose contents belong to
a group — the bit goes too:

```text
before: 2775 drwxrwsr-x
after:  0700 drwx------
```

`0700` means *the owner, and nobody else*. On a shared machine every other
user's access to that directory, and to everything already in it, ends at that
moment — because of a file the user asked to be written somewhere. The
directory was not the tool's to restrict.

### The coupling: the file's own privacy was never what protected the secret

`write_token`'s own comment said:

> The permissions are set *before* the content is written. Doing it afterwards
> leaves a window in which another local process can read the token.

That was true of the **directory** and false of the **file**. The code was:

```rust
std::fs::write(path, body)?;                      // the secret goes in here
if let Err(e) = restrict(path, false) { … }       // and is locked down here
```

and `std::fs::write` creates its file with the process's umask. Measured on
this account, in the directory the tool writes to:

```text
umask-visible mode of a freshly created file: 0664
```

So the secret was on disk in a world-readable file between two adjacent
calls. The window is short — a watcher polling the file's mode never caught
it in 200 000 attempts — but the promise in the comment was not kept, and
what actually protected the token was the directory's `0700`.

**That is why the tool felt entitled to change the directory.** The file's own
mode was not doing the work, so the directory had to. Fixing the directory
without fixing the file would have left the secret in a world-readable file;
fixing the file removes the reason to touch the directory at all. The two
defects are one defect seen from two sides.

### The fix

Two rules, and they are separable.

**The file is private.** It is created `0600` from the moment it exists —
`OpenOptions::mode(0o600)` — and the mode is *checked* before the secret goes
into it, so a file that is not private is one the tool refuses to write
rather than one it writes and repairs. It is then moved into place, so the
published path is never a half-written file, and a token file left over from
a previous launch is replaced rather than kept.

**The directory is left alone unless the tool made it.** `write_token`
records whether the parent existed before it created it, and only restricts
the ones it created.

After the fix, the same directory keeps its permissions and the token is
still private:

```text
before: 0755 drwxr-xr-x
after:  0755 drwxr-xr-x
-rw------- 1 … 47776.token
```

and the application still answers with the token in the file:

```json
{"id":1,"ok":true,"message":"added filter `Probe`","data":{"id":4,"kind":"filter","name":"Probe"}}
```

### The lesson

**A tool that keeps a secret safe by changing something it does not own will
eventually change something it must not.** The directory was not the tool's
to restrict; the file was. The two got confused because the file's own
protection was not doing its job, and the confusion was invisible, because
the outcome — a private token — was right. The wrongness only showed up on
the one input where "the right outcome for the wrong reason" is not the same
as "the right outcome": a directory the user named.

The second lesson is smaller and sharper: **a promise in a comment is a
mechanism.** "The permissions are set before the content is written" was a
statement about a mechanism that did not exist. The comment was describing a
different function — `restrict` on the *directory* — and a reader (including
me, until I measured it) took it as a description of the file. The rule from
§42 applies here as much as it does to the rollback: a comment that names a
mechanism has to name the mechanism that is actually there.

### What now holds it

Four mutations, and the tests they bite:

| Mutation | Caught by |
|---|---|
| the directory is restricted whether or not the tool made it | `a_directory_that_was_already_there_keeps_its_permissions` |
| the token file is created `0644` | `the_token_file_is_private_in_a_shared_directory` |
| the tool's own directory is created `0755` | `a_directory_the_tool_creates_is_private` |
| a token file that already exists is not replaced | `a_token_file_left_over_from_a_previous_launch_is_replaced` |

One of them has to be honest about what it cannot show. The token file's mode
is `0600` before the fix and `0600` after it, so **no mutation can catch the
difference in the file's final state** — the difference is in the state
*before* the fix finished, which no test can reach. What the mutations lock
is the directory rule, which is where the observable harm was. The file-side
guarantee is a promise kept, not a promise proved.

---

## 56. The check that watches the window could watch the wrong window

`scripts/live-window-check.sh` is the only thing in the tree that can see
whether the window follows the document. Everything else about the repaint is
unobservable from a test — that was measured, and it is why the script exists
(§21). It starts the real binary on a real display, drives it over the
control channel, and compares the window's own pixels before and after.

It found the window by name.

### The measurement

The window it found looked like this:

```text
$ DISPLAY=:1 xwininfo -root -tree | grep '"craftkit"'
        0x3e00005 "craftkit": ("craftkit" "craftkit")  1180x720+10+45  +106+114
```

and the id was pulled out with

```bash
grep -o '0x[0-9a-f]\+'
```

which matched **two** things on that line:

```text
$ … | grep '"craftkit"' | grep -o '0x[0-9a-f]\+'
0x3e00005
0x720
```

`0x720` is not a window. It is the *width*: the geometry on the same line
says `1180x720+10+45`, and `0x720` is the tail of it. Only the first id was
used, so this did not bite by itself — but it shows the extraction was
reading a line rather than a field.

The name was the real problem. **A second instance of the application has a
window with the same name**, and the script took whichever the X server
listed first.

### What it cost

A leftover instance — one of several, from something else that happened to be
running — was enough:

```text
live-window: window 0x3e00005            # the *other* instance's window
engine: 3 nodes before, 7 after, 0 failure(s)
live-window: 0 differing pixels (< 1000)
live-window: the engine changed and the window did not — this is the stale-window bug
```

The check reported the bug it exists to find, with the diagnosis attached.
The code was fine. The check was watching a window that had never received
the commands.

This is the failure the script's own comment claimed to have avoided:

> The window, found by name rather than assumed — a stale id from a previous
> run would capture whatever is there now, which is exactly the kind of quiet
> wrong answer this project keeps paying for.

It did avoid a *hard-coded* id. It did not avoid the window it actually had
to avoid: **another instance's**.

The false direction is the dangerous one. A wrong window that happens to
change — because something else drove *it* — makes the check report a
**pass** while the window under test stayed stale.

### The fix

An X window carries `_NET_WM_PID`: the process that owns it. Measured:

```text
window 0x4400005 -> pid 53939
```

and 53939 was the application's own pid. So the check now finds the window
**by pid**:

- every window named `craftkit`, with the pid that owns it;
- the one whose pid is the app's is the app's window;
- one candidate and no way to tell whose it is → use it, because there is
  nothing to confuse it with;
- more than one candidate and none of them ours → **refuse**, rather than
  guess.

"Not ours" is not "not there": the app's window is mapped a moment after the
process starts, so an early look sees only the other instance's. The check
waits; only the timeout turns that into a failure.

The ids are read as the *first field* of the line now, not as every
hex-looking token on it, so the geometry stays out.

### The instrument checks itself

The part of the script that decides which window to measure is pure: it reads
"id pid" pairs and writes one id. So it can be tested without a display, a
build or an application:

```text
$ ./scripts/live-window-check.sh --self-test
  ok    the window whose pid is ours wins
  ok    a lone window with no pid is used
  ok    a window that belongs to another process is refused
  ok    several windows and none of them ours is refused
  ok    no window at all means wait
live-window: self-test ok
```

The same situation, before and after — a second instance running, the code
untouched:

| | window chosen | result |
|---|---|---|
| the old search | the other instance's | 0 differing pixels — **FAIL**, *"this is the stale-window bug"* |
| the new search | this app's (pid 53939) | 11 290 differing pixels — **pass** |

### The lesson

**A gate that cannot tell its instances apart measures one of them and
reports on it.** The name was the only identity the check had, and a name is
not an identity. The window's pid is.

And the smaller lesson, which cost an hour: **the pid of a backgrounded
command is not always the pid of the program.** In a shell, `cmd &` gives
the pid of the job; when the command is compound (`cd … && cmd &`), that is
the *subshell's* pid and the program underneath is a different process.
Measured here: the shell reported 52537, whose `/proc/52537/comm` was `bash`,
while the application was 52539 — and the window carried 52539. The script
backgrounds a simple command, so `$!` is the application's own pid, and the
check now prints it: `window 0x4400005 (pid 53939)`. **A gate that names the
thing it measured is a gate whose answer can be checked** — and this one had
been answering about a window it could not identify.

### What now holds it

One mutation, and the self-test that bites:

| Mutation | Caught by |
|---|---|
| the check stops matching the window by pid | `live-window-check.sh --self-test` |

The mutation is the old rule — *any candidate with a pid will do* — and the
self-test says what it saw:

```
FAIL  the window whose pid is ours wins — expected status 0 output 0xb, got status 0 output 0xa
```

---

## 57. The crash report wrote the design world-readable

A crash report holds the reproduction: the design as it was, and the command
trace that produced it. It is written automatically, on every unclean exit,
into a folder the application chooses. It is the most private thing this
program writes, and it was written with the process's umask and nothing else.

### The measurements

The default folder, on this account:

```text
default crash folder: /home/…/.local/share/craftkit/crashes
after:  0775
report folder 0775
  report.json  0664
  design.json  0664
  replay.json  0664
```

And the parent it lands in:

```text
drwxrwxr-x 3 mmacedoeu mmacedoeu 4096 … /home/mmacedoeu/.local/share/craftkit
```

`0775` and `0664` are world-readable. `~/.local/share/craftkit` is `0755`, so
the path is traversable too. **On a shared machine, any local user could read
the design of any project that crashed** — every node name, every property
value, every keyframe:

```text
design.json holds 5066 bytes and names the document: {
  "meta": { "name": "Sample plan", "fps": 24, … },
  "nodes": [ { "id": 1, "kind": "source", "name": "Level", "props": { … } } ]
```

And `replay.json` holds the command trace **with its parameters** — §27 made
that decision deliberately, and it is the right one: a reproduction is
worthless without the parameters. It is also a list of every name and path
the document ever carried.

Nothing about this was visible from inside the program. The report was
written, the app died, and the folder sat there.

### The fix

The same rule as §55, applied to the place that matters more.

**The files are created private.** `write_atomic` opens its temporary file
with `OpenOptions::mode(0o600)` — private from the moment it exists — and
renames it into place. It used to be `std::fs::write`, which creates its file
with the umask: measured `0664` on this account, so the design was on disk in
a world-readable file between two adjacent calls.

**The report's own folder is `0700`.** It is created by this call, so it is
this tool's to protect.

**A folder that was already there keeps its permissions.** `CRAFTKIT_CRASH_DIR`
can name one that other people use too — a shared CI artifact directory, a
team folder — and §55's rule applies unchanged: only a folder the tool made
is the tool's to make private.

After the fix, at the same default path:

```text
default crash folder: /home/…/.local/share/craftkit/crashes
after:  0700
report folder 0700
  report.json  0600
  design.json  0600
  replay.json  0600
```

The report is still readable by the person it is for. `craftkit-cli crashes
show` prints it to stdout, so **sharing a report never needed the file to be
world-readable** — a copy carries the user's own umask with it.

### The lesson

**The most private thing a program writes should be the thing it writes when
something goes wrong.** The design is copied to disk automatically, without
being asked for, at exactly the moment nobody is thinking about who else is
on the machine. Every other write in this tree is a file the user chose a
place for; this one is not.

The second lesson is about repetition. §55 found the *launch token* changing
the permissions of a directory it did not own. The crash folder had no rule
at all — and it holds a far bigger prize. **A rule that appears once is a
coincidence; a rule that appears twice is a convention.** The second
appearance is what turns "the token file is private" into "what this program
writes is private", and it took a second measurement to notice that the
convention did not exist yet.

### What now holds it

Three mutations, and the tests they bite:

| Mutation | Caught by |
|---|---|
| the report's folder is left readable by other users | `a_crash_report_is_not_readable_by_other_users` |
| the design is written readable by other users | `a_crash_report_is_not_readable_by_other_users` |
| a crash folder the tool did not create is made private anyway | `a_crash_folder_that_was_already_there_keeps_its_permissions` |

---

## 58. The crash report's id was not the id the code's own type promised

`CrashReport::id` has one line of documentation:

```rust
/// A sortable identifier: `20250907-093012-7f3a`.
pub id: String,
```

and `docs/recipes/fix-a-crash.md` says the same thing. The code produced
something else.

### The measurements

The id was built like this:

```rust
let mixed = unix ^ (salt << 20) ^ nanos;
let secs = unix % 100_000_000;
format!("{secs:08}-{:04x}", mixed & 0xffff)
```

which gives:

```text
id the code produces: 92491655-8d92
id the docs promise: 20250907-093012-7f3a
```

No date, no time of day, no milliseconds — a number that happens to be
seconds-since-the-epoch, taken modulo a hundred million.

**And it was not sortable**, which the documentation says it is. The first
part is `unix % 100_000_000`, so it restarts every 1 157 days:

```text
a crash one second before the wrap: 99999999-d1ff
a crash one second after  the wrap: 00000000-d200
string order puts the LATER one first: true
```

`list_in` sorts the ids as strings, so once a crash folder crossed that
boundary the newest reports were listed first and the oldest last — exactly
backwards, and wrong for the reports a person is looking for.

### The worse half: a second crash destroyed the first

The id's low bits come from the sub-second clock, so two crashes in the same
second can land on the same id. And `write_to` created the folder with
`create_dir_all` and wrote the three files into it — so a second report with
the same id **overwrote the first**.

Measured, 200 crashes in one second:

```text
200 crashes in one second -> 198 report folders, 3 229 182 bytes
```

Two reports were gone. And deliberately:

```text
two crashes with the same id -> 1 folder(s): ["20250907-093012-7f3a"]
the folder now holds: the SECOND crash
the first crash's report is gone: true
```

A crash report is the only evidence that a crash happened. It is written by
the panic hook, on every unclean exit, with no idea whether another one is
about to arrive. **The second crash took the first one's evidence.**

### And a test was relying on it

The control crate's crash-replay test plants a report whose recorded design
does not match what its commands build, so the replay can be seen to say so.
It planted that report by **writing it over the honest one** and then asking
for the id — which only worked because a second report with the same id
overwrote the first.

```rust
let mut lying = report.clone();
lying.design = serde_json::json!({ "meta": { "name": "lies" }, "nodes": [] });
craftkit_engine::crash::write_to(&dir, &lying).expect("the report is written");
let request = Request::new(10, Method::Crashes)
    .with_params(json!({ "action": "replay", "id": lying.id.clone() }));
```

The test failed the moment the defect was fixed, because the honest report
was still there to be replayed. It now writes the lying report as its own
report, under its own name. **A test that sets up its scenario with the
defect cannot be the test that catches it** — and this one had been quietly
certifying the defect all along.

### The fix

**The id is the documented one.** `YYYYMMDD-HHMMSS-xxxx`, in **UTC** so the
same crash gets the same id on every machine, with four hex digits of
sub-second position so two crashes in the same second usually differ and the
ids sort into the order things happened.

The date arithmetic is written out — the tree has no date library and this is
arithmetic, not a dependency. The epoch is shifted to 0000-03-01, which puts
the leap day at the end of a year and makes every month the same length; the
era is 400 years wide because 400 years hold exactly 146 097 days. The result
is pinned against dates anybody can check:

| Unix seconds | UTC |
|---|---|
| 0 | 1970-01-01 00:00:00 |
| 951 782 400 | 2000-02-29 00:00:00 |
| 1 234 567 890 | 2009-02-13 23:31:30 |
| 1 709 164 800 | 2024-02-29 00:00:00 |
| 4 102 444 800 | 2100-01-01 00:00:00 |

**A report is never overwritten.** If the folder is already there, the report
is filed beside it — `…-7f3a-2` — and the id inside `report.json` is made to
match the folder it is in, so the report and its name never disagree.

After the fix, the same 200 crashes:

```text
200 crashes in one second -> 200 report folders
first id: 20261020-102425-0088
last  id: 20261020-102425-ffb4
the report's id field: "20261020-102425-0088"
the folder's name:      20261020-102425-0088
```

### What is still open

The crash folder is now private and complete, and it is still **the only
unbounded writer in the tree**. The undo stack has a byte budget (§52), the
trace has one, the command log has a count — and the crash folder has
neither. Measured: a report of the sample document is 16 309 bytes (the
design is 5 066 of them, and `report.json` carries the base document too), so
a crash loop — an application that panics on launch, relaunched by a
supervisor — writes a full copy of the design every time and nothing ever
prunes it. Measured, 200 crashes in one second:

```text
198 report folders, 3 229 182 bytes total (16 309 bytes per report)
  report.json     11 241 bytes
  design.json      5 066 bytes
  replay.json          2 bytes
```

(Two of the two hundred reports were lost to an id collision — §58 — so the
folder held 198 of them.) Nothing in the tree deletes a report, and that is
deliberate for now: deleting somebody's evidence is a decision, not a
default. It is recorded here rather than left as a surprise.

### The lesson

**An identifier that says nothing is not an identifier.** `92491655-8d92` is
opaque: it does not say when the crash happened, and it is not the format the
type beside it documents. The code and its own documentation had been
disagreeing, and nothing in the tree compares them.

The second half is the one that costs something: **an append-only folder must
not be writable twice.** Every other writer in this tree is careful about
overwriting — §35 checks that a command's effect survives the file, §47 checks
that a replay lands on the right document, §50 checks that the writer refuses
a document the reader would refuse. The crash folder was the one place where
a second write silently destroyed the first, and it was the place where the
evidence mattered most.

### What now holds it

Three mutations, and the tests they bite:

| Mutation | Caught by |
|---|---|
| the id stops carrying a date | `the_id_is_the_documented_format` |
| the date is a day out | `the_date_is_right_for_a_known_day` |
| a second crash destroys the first crash's report | `a_second_crash_does_not_destroy_the_first` |

---

## 59. The frame size was capped per side, and the area was not

`ui.render` and `ui.screenshot` draw a frame and answer with it as a PNG. The
size is hostile input like any other — it arrives from an agent, a script or a
file — and `crates/ui/src/raster.rs` says so:

> `width` and `height` arrive from an agent, a script or a file, and a zero or
> a hundred-thousand would either divide by zero or **allocate until the
> process dies**.

There was a cap. It was on each side:

```rust
pub const MIN_SIZE: usize = 64;
pub const MAX_SIZE: usize = 8192;
```

### The measurements

8192 on each side is allowed. 8192 on each side is **67 108 864 pixels**, and
a frame holds four bytes per pixel.

```text
1180x720:  50.8ms  png     251 212 bytes  peak RSS  10 136 KiB
2048x2048: 223.7ms  png   1 100 579 bytes  peak RSS  24 812 KiB
4096x4096: 940.1ms  png   3 005 688 bytes  peak RSS  77 536 KiB
8192x8192: 3.52s    png  11 091 593 bytes  peak RSS 301 028 KiB
```

**301 MB of resident memory**, for one picture of a three-node sample
document. The pixel buffer alone is 268 435 456 bytes. On the build machines
this program is meant to run on — the app's own documentation calls itself
*"a usable headless host"* on a build machine — that is the allocation the
limit's own words were about.

In the running application, over the control channel:

```text
the render finished at 6.34 s, reply 14 789 463 bytes
```

**14 789 463 bytes on one line.** The protocol caps a *request* line at 1 MiB
(`MAX_LINE`), and the reply was 14 times that.

And the app was not answering anything else while it drew:

```text
a session.stats issued 1.0 s into the render came back after 6.27 s
```

The session is behind a mutex, and the shell holds it for the whole frame —
it has to, because a panel can run a command through the shell. So one
request from one agent froze the application for six seconds and a half.

### The fix

**A limit on each side is not a limit on the whole.** The memory a frame costs
is its *pixels*, so the budget is in pixels:

```rust
pub const MAX_PIXELS: usize = 16_777_216;   // 4096 × 4096
```

16 777 216 keeps every frame the tool draws itself, a retina-sized one and a
4K one; it refuses the one that hurts. The per-side cap stays, so a wide
frame is still allowed — 8192 × 2048 is inside the budget.

After the fix, in the running application:

```text
8192x8192: refused in 0.01s — a frame may be at most 16777216 pixels; asked for 8192x8192, which is 67108864
4096x4096: ok in 1.86s, reply 4008255 bytes
```

The refusal is immediate and names both the limit and what was asked for, and
the largest reply the channel can produce is now 4 MB rather than 14.8.

The documentation is corrected too. `skills/craftkit-checking/SKILL.md` said
*"lines are capped at 1 MiB"*, and `docs/recipes/add-a-control-channel-method.md`
said *"one JSON object per line, in and out"* next to `MAX_LINE` — both of
which read as though the limit applied to replies as well. It does not: a
reply may be a picture. Both now say **request** lines, and say how to read a
reply that is 4 MB of base64.

### What is still open

The render still happens while the session is locked, so the application is
unresponsive for as long as a frame takes. The pixel budget cut the worst case
from 6.3 s to 1.9 s; it did not remove it. The reason it cannot simply be
moved out of the lock is structural: `Shell` holds `&mut Session` because a
panel may run a command through it, so a render is not a read. Making the
headless render path a read — a shell that cannot act — would let the frame
be drawn from a snapshot outside the lock. That is a change to the UI crate's
shape, not a cap, and it is recorded here rather than pretended away.

### The lesson

**A limit in the wrong unit is not a limit.** The cap was expressed in *sides*
because that is how the request names the size — `width` and `height` — and
the memory is in *pixels*. Every other limit in this tree is expressed in the
unit it actually bounds: bytes for a file (§49), bytes for a document (§52),
bytes for the undo stack (§52), pixels for a frame. This one was expressed in
the unit the *caller* used and the memory did not.

And the smaller half, which is the same lesson from the other side: **a
limit that is not stated in the direction it applies is not a limit.** The
channel bounded its reads and said nothing about its writes, and the prose
that described the channel could not tell a reader which was which.

### What now holds it

One mutation, and the test that bites:

| Mutation | Caught by |
|---|---|
| a frame's area stops being bounded | `a_frame_that_is_too_big_in_pixels_is_refused` |

The mutation makes the check unreachable, and the test then renders the frame
it was supposed to be refused — which takes 7.5 seconds and 301 MB. The gate
notices the missing refusal, and the cost notices it too.

---

## 60. A tick could be wider than a reply, and the panic was inside a macro

`Tick` is the type every time in this program is stored as. It is an `i128`,
and its arithmetic **saturates**: the pattern document promises that a
hostile or nonsensical time *"becomes the nearest representable time; it never
wraps, **never panics**, and never produces a time in the past from an
addition"*.

It panicked.

### The measurements

The `time` parameter is a number of seconds, and `craftkit_core::finite`
accepts anything up to `MAX_MAGNITUDE` — 10^9 seconds. One second is
254 016 000 000 ticks, so the largest accepted time is **2.54016 × 10^20
ticks**.

A reply is JSON, and a JSON number holds an `i64` or a `u64` and nothing
else: at most **1 844 674 407 370 955 161** ticks. So the parameter's range
reaches far past what a reply can carry, and the boundary is at

```text
1 844 674 407 370 955 161 / 254 016 000 000 = 72 623.7 seconds
```

Every accepted time above 72 623.7 seconds produced a tick no reply could
hold:

```text
key.set with a hostile time:
  time 72624:   ok: keyed `amount` at 72624.000s
  time 100000:  ok: keyed `amount` at 100000.000s
  time 100000000: Err("internal"): `key.set` failed unexpectedly (called `Result::unwrap()` on an `Err` value: Error("number out of range", line: 0, column: 0)); your document is unchanged
  time 18446744073709551616t: Err("internal"): … (the same)
  time -99999999999999999999t: Err("internal"): … (the same)
doc.range {"end":1000000000.0,"start":0} -> Err("internal"): `doc.range` failed unexpectedly (called `Result::unwrap()` on an `Err` value: …); your document is unchanged
```

The user is told *"failed unexpectedly"* and shown a serde error with a line
and a column. Nothing says *time*, nothing says *too large*, and nothing says
which parameter.

Two things were working: the document was unchanged (the rollback held) and
the application did not crash (`Session::execute`'s `catch_unwind` held). The
command panicked, the boundary caught it, and the user got a message about a
`Result` in a macro they have never heard of.

### Where the panic was, and why the lint did not see it

The reply was built with `serde_json::json!`:

```rust
serde_json::json!({ "node": node, "path": path, "time": time.raw(), "value": value })
```

`json!` converts an interpolated expression with `serde_json::to_value(…)` and
**`.unwrap()`s it — inside the macro expansion**. The house deny is
`#![deny(clippy::unwrap_used, …)]`, and clippy cannot see an `unwrap` that
exists only after a macro expands. So the rule that exists to stop exactly
this was, for this case, not running.

There were **seven** places in the tree that put a tick into a `json!`:
`commands.rs` (three), `control/src/server.rs`, `engine/src/session.rs`,
`io/src/format.rs` and `engine/src/crash.rs`. And one of them had already hit
the wall and worked around it locally:

```rust
at: i64::try_from(session.at().raw()).unwrap_or(0),
```

The crash reporter had to clamp the playhead to an `i64` so its own report
could be written. That is the fingerprint of a problem nobody had named.

### The other half: the format is wider than the model

The file format can hold **more** than the model would. A bare integer in a
file is read straight into an `i128` — `serde_json` has a path for that — so
a file can name an instant that no JSON number holds. Measured, before the
fix: a file whose `start` was 18446744073709551616 was written, read back
and **loaded**, and the document validated. That document's playhead could
not be reported by any reply.

### The fix

**The model's tick range is the range a reply can hold.** `Tick::MIN` is
`i64::MIN` and `Tick::MAX` is `u64::MAX` — the integers a JSON number holds.
They were `i128::MIN / 4` and `i128::MAX / 4`, which is 2.3 × 10^18 times
wider.

So saturation now lands on a representable value, the promise the pattern
document makes is true again, and the panic is unreachable:

```text
key.set with a hostile time:
  time 72624:   ok: keyed `amount` at 72624.000s
  time 100000000: ok: keyed `amount` at 72620402.155s
  time 1000000000: Err("internal"): `key.set` failed unexpectedly (the command declared that it mutates the document but changed nothing); your document is unchanged
```

No panic, anywhere. (The second message is the no-op detector doing its job:
the time saturated onto the key that was already there, so the document
really did not change. It is a confusing message for a hostile input, and it
is recorded below.)

**And the reader refuses what the model will not hold.** A file whose range
is outside the model's range is refused, by name:

```text
refused: the document in this file is not valid: bad metadata: the document's range is outside what a time can be: 18446744073709551616t..18446744073709551617t
```

The writer refuses the same document, so §50's contract holds for ticks too:
the two are one contract with two halves.

### What is still open

Two things, both recorded rather than pretended away.

**The parameter's range is wider than the tick's.** `time` accepts 10^9
seconds; a tick can carry 72 623.7. So a hostile time saturates rather than
being refused, which is the documented contract ("an out-of-range time is
clamped, not an error to handle at every call site") — but the message the
user gets when the clamped time collides with an existing key is *"the command
declared that it mutates the document but changed nothing"*, which does not
mention time at all. A cap of 72 623 seconds on the `time` parameter would
make it honest, and would also refuse a timeline longer than twenty hours,
which might be a real thing somebody wants.

**Six `json!` sites carry a tick** and now rely on the model's range being
the reply's range. That is true, and it is true for a reason nobody wrote
down at the site. A helper that carried a tick safely — a `Tick` that knows
how to become a `Value` — would make the six sites independent of the bound
and would put the rule in one place.

### The lesson

**A saturation has to land somewhere the rest of the system can carry.** The
arithmetic was right: it saturated, it did not wrap, it did not go negative.
It saturated into a value that nothing else in the program could hold, and
the program had no way to say so.

The second lesson is the one that should change how the tree is written:
**a lint cannot see inside a macro.** `json!` is used everywhere in this tree
to build replies, and every `json!` that interpolates a `u64` larger than
`i64::MAX`, or an `i128` at all, carries a hidden `unwrap`. The house deny
was written to stop exactly that and could not. The answer is not to remove
the deny — it is to know which macros hide one, and to keep the values that
go into them inside the range the macro can carry.

And the third is the one about this file: **the promise was in a pattern
document, and the pattern document was right.** *"never panics"* was written
down, checked by nobody, and was false for six years. §10 found two doc
comments in one file describing a saturation rule that did not exist; this is
the same shape again, one level up: a pattern document describing a rule the
code did not keep.

### What now holds it

Three mutations, and the tests they bite:

| Mutation | Caught by |
|---|---|
| a tick can be wider than a reply can carry | `a_hostile_time_does_not_panic_the_command_that_reports_it` |
| a document whose range no reply can carry is accepted | `a_document_whose_range_is_outside_what_a_time_can_be_is_refused` |
| a file whose range is too far out is read back | `a_file_whose_range_is_too_far_out_is_refused` |

The suite caught one mistake of mine along the way, which is worth recording
because it is the kind that would otherwise have been filed as a gap in the
code. The second and third rows share a mutation — the same line in
`crates/model/src/document.rs` — and I wrote them as one `swap` followed by
two `expect_catch` calls. But `expect_catch` **restores the tree after every
case**, so the second gate ran against an unmutated tree, passed, and was
reported as MISSED. The run exited non-zero and said which gate. One swap
serves exactly one gate; the rule is now in the script's header, and the
script has a check that finds any gate without a mutation of its own.

---

## 61. The file said when it was written, and never said anything

The smallest finding in this file, and the one that took the least measuring.

### The measurements

Every design file carries a header:

```json
"meta": { "written_by": "craftkit 0.1.0", "written_at": 0, "pending": [] },
```

and the field's own doc comment said:

```rust
/// The tick at which it was written. Ticks, never seconds — see
/// `craftkit-core`.
pub written_at: i128,
```

It is **always zero**. `Meta::now()` writes `written_at: zero()`, and nothing
in the tree reads the field at all — before the fix, the field and its
neighbours read like this:

```text
$ grep -rn written_at --include=*.rs .
crates/io/src/format.rs:106:    pub written_at: i128,
crates/io/src/format.rs:114:/// The default for [`Meta::written_at`], so a file written before the field
crates/io/src/format.rs:124:            written_at: zero(),
crates/io/src/store.rs:126:///   "meta": { "written_by": "a person", "written_at": 0, "pending": [] },
crates/io/src/store.rs:153:///   "meta": { "written_by": "a person", "written_at": 0, "pending": [] },
```

So a file says it was written at tick zero, and claims that zero is a time.

### Why the zero is right

Because a file's bytes have to depend on the document and on nothing else.
Saving the same plan twice has to give the same file, or every save is a
change and a saved plan does not diff cleanly — and there is a test for
exactly that:

```rust
fn the_text_form_is_stable_so_saves_diff_cleanly() {
    let doc = sample();
    let a = save_text(&doc).expect("text");
    let b = save_text(&doc).expect("text");
    assert_eq!(a, b);
```

A clock reading in that header would break it on every save. The `craftkit-io`
crate has no clock in it at all — no `SystemTime`, no `UNIX_EPOCH` — which is
not an accident but a consequence: the crate that writes the file does not
know what time it is.

**So the behaviour is right and the doc comment is wrong.** And the reason it
is right is written down in neither the comment nor the format's design
document, which shows `"written_at": 0` in its example without a word about
it. The next person to read that comment would reasonably "fix" the field to
carry a write time, and the only thing that would stop them is a failing
test whose name does not mention the field.

### The fix

The comment now says what the field is, and why it cannot be what it is
called:

```rust
/// **Always zero.** Reserved.
///
/// The field is called `written_at` and the format is a tick count, but it
/// does not carry a write time, and it must not: a file's bytes have to
/// depend on the document and on nothing else, so that saving the same
/// plan twice gives the same file and a saved plan diffs cleanly.
```

### The lesson

**A field's name is a promise, and a doc comment that repeats the name is not
a description.** `written_at` says what the field is for and `written_at:
0` says it is not there. The two were in the same struct, one line apart, and
nothing noticed — because a reader who already knows the format reads the
name and skips the comment, and a reader who does not know it reads the
comment and believes it.

This is the third instance of the shape §53 named: prose that points at real
code and describes it wrongly. §53's two documents described the old
fingerprint correctly and in detail; §4 claimed a permission was set before
the content when it was set after; and here a one-line comment described a
field that does not exist. **The common thread is that the prose was written
by someone reading the code, and reading the code is not the same as
measuring it.** §62 is the fourth, and it is the same shape again: a counter
whose own doc comment said *"since the process started"* while the storage
three lines below it was a `thread_local!` cell.

### What now holds it

A test, not a mutation — and that is the right instrument here:

| Property | Held by |
|---|---|
| saving the same document twice gives the same bytes | `the_text_form_is_stable_so_saves_diff_cleanly` |

There is no mutation to write, because the thing that could break it — a
clock in the header — cannot be introduced by changing one line: the crate
that writes the file does not know what time it is. The property is held by
the absence of a mechanism, and the test is what says so.

---

## 62. The counter that reports recoveries counted only the thread that asked

The kernel keeps two counters side by side. One of them was wired to the wrong wire.

### The measurements

`crates/kernel/src/lib.rs` keeps two numbers:

- `DEPTH` — how deep the current thread is inside a guarded call. `guarded()`
  reads it.
- `RECOVERED` — how many panics the boundary has caught. `recovered_count()`
  reads it.

The second one existed for a stated purpose, written on the counter itself:

```rust
/// Panics recovered by this boundary since the process started. Exposed so
/// the engine can tell "the kernel recovered from something" from "nothing
/// unusual happened" without parsing a log.
static RECOVERED: Cell<u64> = const { Cell::new(0) };
```

**Since the process started.** It was a `thread_local!` cell, so it counted
since the *thread* started.

Measured with a probe against the real code — one panic recovered on a worker
thread, then the count read on the main thread:

```text
main thread before          : 0
worker thread recovered     : true (is_internal=true)
worker thread after         : 1
main thread after           : 0
main - worker               : -1
```

And the engine's statistic — the thing the counter was for — after that
recovery:

```text
stats on main thread        : Ok(Object {"commands": Number(0), "failures": Number(0),
                                          "kernel_recoveries": Number(0), "nodes": Number(0),
                                          "undo_depth": Number(0)})
```

`kernel_recoveries: 0`, one recovery into the bargain.

### The two threads are never the same thread

In the running app the counter and the work are on different threads:

- The **UI thread** evaluates the document. `Shell::node_list` asks the session
  for its plan, the session evaluates, and every filter node goes through
  `apply_gain`, which is wrapped in the kernel's `guard`.
- The **control channel's thread** answers `session.stats`. The channel spawns
  one thread per connection and names it `craftkit-control`.

Measured in the running application, with a control connection open:

```text
=== threads while a control connection is open ===
  craftkit             tid=35293     <- the main thread: the window, the frame, the evaluation
  craftkit             tid=35294
  ...
  craftkit-contro      tid=35783     <- the control channel's connection thread
{"id":1,"ok":true,"message":"1 command(s), 0 failure(s)","data":{"commands":1,"failures":0,
 "kernel_recoveries":0,"nodes":3,"undo_depth":0}}
```

(The kernel truncates a thread's name to fifteen characters, which is why
`craftkit-control` reads as `craftkit-contro`.)

So the counter could only ever report recoveries that happened on the thread
doing the reading, and the reading is not where the work is.

### A negative result worth keeping

**In the tree as it stands the counter can never move at all.** The only
guarded operation the application calls is `apply_gain`, and its arithmetic is
total: it refuses a non-finite input and saturates its output. `evaluate_chain`
and `measure_run` are called by their own tests and by nothing else.

That does not make the defect hypothetical, and it is worth saying why. The
kernel crate exists so that an untrusted implementation can be plugged in
later — `crates/kernel/src/ops.rs` says the guard is "the seam where an
untrusted node implementation would be plugged in", and the crate's own module
doc says the rule is "if a call can panic and its code is not in this
workspace, it goes through `guard`". The counter is the tripwire for that day,
and it was wired to the wrong wire.

**A statistic that cannot move is not evidence of health** — and a statistic
that could only move on the reading thread would not have been evidence of
health either.

### The fix

`RECOVERED` is now a process-wide atomic. `DEPTH` stays thread-local, because
depth *is* a per-thread question: "am I inside a guarded call?" is about this
thread's stack, while "has the kernel recovered from a panic?" is about the
process. The two counters answer different questions, and giving them the same
scope would have been wrong in one direction or the other.

```rust
static RECOVERED: AtomicU64 = AtomicU64::new(0);
```

The increment saturates rather than wraps, so the number cannot quietly go
back to zero and read as "nothing has ever gone wrong".

The command's own doc string now says what the number is, because the other
counters in that reply are per-session and this one is not:

```rust
doc: "Counters for the running session: commands, failures, undo depth, kernel recoveries (process-wide).",
```

### The lesson

**A counter's scope is part of its meaning.** `recovered_count()` had a number,
a name and a doc comment, and the three did not agree about which process the
number was counting.

There is a second lesson, and it is the one §53 named: the doc comment said
*"since the process started"*, and it was not. That is the **fourth** instance
of prose that points at real code and describes it wrongly — after §53's
fingerprint, §4's permissions and §61's `written_at`. The common thread is
that the prose was written by someone reading the code, and reading the code
is not the same as measuring it: the doc comment and the `thread_local!` were
three lines apart, in the same block, and the block read as though it were one
thing.

The two counters sit next to each other and look interchangeable. They are not:
one asks about a thread and the other asks about a process. **Two numbers next
to each other are not the same question, and a statistic is only as good as the
scope it was measured in.**

### What now holds it

| Property | Held by |
|---|---|
| A recovery on one thread is counted on another | `a_recovery_on_one_thread_is_counted_on_another` |
| The session statistics count a recovery from another thread | `the_session_statistics_count_a_recovery_that_happened_on_another_thread` |
| A guarded call on another thread does not guard this one | `being_guarded_on_one_thread_does_not_guard_another` |

and two mutations in `scripts/mutation-test-gates.sh`:

- the counter goes back to being thread-local, and the kernel test catches it;
- the statistics stop reading the counter, and the engine test catches it.

There is no screenshot, and there could not be one: nothing on screen changes.
The defect is in which thread counts, and its whole effect is on a number in a
JSON reply. The evidence above is a JSON reply, a thread listing and a probe.

---

## 63. The token comparison lost every length difference of 256

The one place in the tree that decides whether a peer may talk to the
application folded a length difference into a byte.

### The measurements

`crates/control/src/token.rs` compares the presented token with the launch
token:

```rust
pub fn matches(&self, presented: &str) -> bool {
    let a = self.0.as_bytes();
    let b = presented.as_bytes();
    let mut diff = (a.len() ^ b.len()) as u8;
    let n = a.len().max(b.len());
    for i in 0..n {
        diff |= a.get(i).copied().unwrap_or(0) ^ b.get(i).copied().unwrap_or(0);
    }
    diff == 0
}
```

`diff` is a `u8` because it accumulates *byte* differences. The length
difference is not a byte — it is a count of bytes, and it was folded into the
same accumulator with `as u8`. A difference that is a multiple of 256 does not
fit, and the truncation threw it away.

Measured against the **running application**, over the real control channel,
with the token the application had itself written:

```text
the token is 64 characters long

  the token itself          -> ACCEPTED
  a wrong token             -> refused
  the token + 1 NUL         -> refused
  the token + 255 NULs      -> refused
  the token + 256 NULs      -> ACCEPTED
  the token + 257 NULs      -> refused
  the token + 512 NULs      -> ACCEPTED
  the token + 1024 NULs     -> ACCEPTED
```

A scan of every padding from 0 to 1024 bytes gave exactly:

```text
accepted NUL-padding lengths from 0..1024:
  [0, 256, 512, 768, 1024]

every accepted length is a multiple of 256: True
the multiples of 256 up to 1024 are all accepted: True
```

**The rule is exact: a presented token is accepted when it is the token, or
when it is the token followed by a multiple of 256 NUL bytes.**

### Why NUL bytes are the ones that work

The loop treats a byte that is not there as zero:

```rust
diff |= a.get(i).copied().unwrap_or(0) ^ b.get(i).copied().unwrap_or(0));
```

so a NUL byte and a missing byte are the same value to it. That is fine on its
own — the *length* is what has to catch it — and the length is exactly what
the truncation lost.

The NULs have to be written as `\u0000`, because the channel's own parser
refuses a raw control character inside a string. That is no obstacle: every
JSON encoder writes a NUL that way, so the padding arrives as an ordinary
JSON string.

### What it cost, honestly

The acceptance set is `{token} ∪ {token + 256k NUL bytes}`. A peer still has
to know the token, so **this is not a way in for somebody who does not have
it** — the secret is still required, in full, as a prefix. What it is, is a
comparison that answers *"yes"* to a string that is not the token, in the one
place the project calls a promise, and a `Token` that compares equal to a
different `Token`.

That is worth having measured and fixed rather than argued about, because the
next version of this comparison might be the one where the extra bytes are not
required to be NUL. The property that was broken is *"this and nothing else"*,
and it was broken by a truncation rather than by a decision.

### The fix

The accumulator is a `usize` now:

```rust
let mut diff = a.len() ^ b.len();
let n = a.len().max(b.len());
for i in 0..n {
    diff |= usize::from(a.get(i).copied().unwrap_or(0) ^ b.get(i).copied().unwrap_or(0));
}
diff == 0
```

`a.len() ^ b.len()` is zero only when the lengths are equal, and the `|=`
below can never clear a bit it did not set, so a length difference of **any**
size survives to the end. The comparison is still constant-time over the
longer of the two, which is what it was before; it is now also exact.

Measured again against the running application, with the fix in:

```text
=== after the fix ===
  the token itself            -> ACCEPTED
  the token +     1 NULs      -> refused
  the token +   255 NULs      -> refused
  the token +   256 NULs      -> refused
  the token +   257 NULs      -> refused
  the token +   512 NULs      -> refused
  the token +  1024 NULs      -> refused
  the token +  4096 NULs      -> refused
```

### The lesson

**A difference measured in the wrong unit is not a difference.** The length of
a string is a count of bytes; `diff` held eight bits of it; and the part that
did not fit was discarded without a word. This is §59's shape in a new place —
a limit in the unit it does not bound — except that here the unit was not a
limit but an *accumulator*, and a byte accumulator was asked to hold a count.

There is a second lesson, and it is the sharper one. The comparison was made
constant-time, and the module's own justification for that was:

> Constant-time. Not because an attacker on loopback can usefully time a
> 128-bit comparison, but because the alternative is one `==` that someone
> eventually copies into a path where it matters.

**The `==` it replaced was exact.** The rewrite for a non-functional property
silently dropped a functional one, and the argument for making the rewrite —
"the alternative is a `==`" — is an argument for a comparison that was
*right*. Nobody wrote down the trade, because the trade was not noticed.

**A rewrite for a non-functional property must keep the functional one.** A
constant-time comparison has to be *constant-time and exact*, and a test that
checks that one wrong token is refused has not checked the space of tokens
that are accepted. The property to test is "this and nothing else", and it has
to be said in those words.

### What now holds it

| Property | Held by |
|---|---|
| A token with padding on the end is not a token | `a_token_with_trailing_nuls_is_not_a_token` |
| A token is not accepted because its own tail is NULs | `a_token_is_not_accepted_because_its_tail_is_nuls` |
| The channel refuses a connection that presents a padded token | `a_connection_with_a_padded_token_is_closed` |

and two mutations in `scripts/mutation-test-gates.sh`:

- the accumulator goes back to a `u8`, and the token's own test catches it;
- the length is not folded in at all, and the channel's test catches it.

There is no screenshot, and there could not be one: nothing on screen changes.
The evidence above is a JSON reply, a scan of a thousand padding lengths, and
a pair of tests.

---

## 64. The limit that bounded the read was credited with bounding the memory

Three readers of untrusted JSON had three unrelated limits, and the largest
of them bought a gigabyte of memory.

### The measurements

The tree's limits are applied **while reading** — §49's fix, and a good one.
The reader keeps at most `max` bytes of the line and drops the rest, so a
peer that sends a gigabyte without a newline costs a buffer.

But the line is then **parsed**, and the parse is what costs memory. The
limits were written as if reading and parsing were the same thing:

- `crates/control/src/protocol.rs` — `MAX_LINE`, 1 MiB;
- `crates/mcp/src/rpc.rs` — `MAX_MESSAGE`, 4 MiB;
- `apps/craftkit-cli/src/cli.rs` — the script limit, 32 MiB.

And four places said so in as many words. `crates/io/src/format.rs`:

> It is applied **while reading**, in [`crate::store::load`], so that a file
> of any size costs at most this much memory — a cap checked after the read
> has already paid for what it was meant to prevent.

`crates/core/src/readers.rs`:

> so a line of any length costs at most `max` bytes of memory

Measured against the real tools, with the real limits:

```text
=== craftkit-cli run, a 32 000 063-byte script ===
Maximum resident set size (kbytes): 1034584        <- 1 010 MiB

=== craftkit-cli mcp, a 2 600 064-byte message ===
Maximum resident set size (kbytes): 87024          <- 85 MiB
```

**A 32 MiB script made the tool use a gigabyte of memory.** A scan of the
shape confirmed the ratio: the worst case is a JSON array of small numbers,
because a number is two bytes of text and a parsed value is thirty-two bytes
— it has to carry its own type — and the vector holding the list can be
twice its length.

```text
  1000 keys: serialised        12781 bytes, in memory about       83890 bytes -> 6.6x
```

The text limit was not wrong about the *read*. It was wrong about what it
was for.

### Why the document path was the exception

A design file is read under a 64 MiB cap, and the document it holds is
measured against `MAX_DOCUMENT_BYTES` — 32 MiB of memory — by
`Document::validate`. That is §52's work, and it is why the document path
is the one place in the tree where the bytes of a file and the bytes of what
it holds are two different numbers with the *tighter* one on the memory.

**The tree already knew how to do this and had done it once.** The three raw
JSON readers were the ones that had not.

### The fix

One number, in `crates/core/src/lib.rs`, and every reader of untrusted JSON
derives its limit from it:

```rust
pub const JSON_EXPANSION: usize = 32;
pub const MAX_JSON_MEMORY_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_JSON_TEXT_BYTES: usize = MAX_JSON_MEMORY_BYTES / JSON_EXPANSION;
```

- the control channel's line limit: 1 MiB, unchanged, now derived;
- the MCP server's message limit: 4 MiB → **1 MiB**;
- the CLI's script limit: 32 MiB → **1 MiB**.

Measured again with the fix in place:

```text
=== craftkit-cli run, the same 32 000 063-byte script ===
craftkit-cli: could not read /tmp/bigscript2.json: the input is 32000063 bytes; the limit is 1048576
Maximum resident set size (kbytes): 4136

=== a script just under the new limit: 1 040 063 bytes ===
Maximum resident set size (kbytes): 36836          <- 36 MiB, a factor of 35
```

**The worst case fell from 1 010 MiB to 36 MiB.** The number that is left is
the expansion the constant names, and the constant is checked against the
real size of a parsed value rather than against a number somebody typed:

```rust
let worst = std::mem::size_of::<serde_json::Value>();
assert!(JSON_EXPANSION >= worst, "…");
```

so if `serde_json`'s value ever grows, the text limit has to be re-derived
with it.

Two smaller things came with it:

- `document_from_json` takes the value **by value**. It used to take it by
  reference and clone it, and the clone was a second copy of a structure that
  can be a gigabyte.
- `replay_as_script` refuses to print a replay script that is too big to be
  read back, and says which command replays it directly. Printing a script
  that could not be run, or printing one without its base, would both have
  been worse than saying so.

And the four claims that the read was the memory are corrected, in the four
places they were made.

### The lesson

**A cap on the input is not a cap on the memory.** §49 fixed *when* the caps
are applied — while reading, not after — and the prose written in the same
edit went one step further and said the read *was* the memory. It was not:
reading is cheap and parsing is not, and the two differ by a factor that
depends on the shape of the input.

**A limit in the wrong unit is not a limit** — which the tree has now learned
twice, in two different units: §59's pixels and §63's bytes. The third time,
the unit was not wrong but *incomplete*: the number was right, and it was
answering a question nobody had asked.

**Three unrelated numbers are three unrelated decisions.** 1 MiB, 4 MiB and
32 MiB were each chosen for a different reader and none of them was written
down as what it was for, so nobody could see that the largest one was
thirty-two times looser than the others. One derived number and one budget
is the whole fix, and the test that keeps it honest is three lines long.

**The path that had already solved it was the evidence.** The document path
measures its own memory (§52) and the raw JSON paths did not, and the
difference between them was the size of the gap.

### What now holds it

| Property | Held by |
|---|---|
| A text limit fits the memory budget once the parse has expanded it | `the_json_text_limit_fits_the_memory_budget` |
| The expansion covers the real size of a parsed value | `the_expansion_covers_the_size_of_a_parsed_value` |
| The control channel's line limit is the tree's limit | `the_line_limit_is_the_tree_s_json_limit` |
| The MCP server's message limit is the tree's limit | `the_message_limit_is_the_tree_s_json_limit` |
| The CLI's script limit is the tree's limit | `the_script_limit_is_the_tree_s_json_limit` |
| A script over the limit is refused, and the message says what the limit is | `an_oversized_script_is_refused_with_the_limit_in_the_message` |

and two mutations in `scripts/mutation-test-gates.sh`:

- the MCP server's message limit goes back to being a number of its own, and
  its test catches it;
- the CLI's script limit goes back to being a number of its own, and its test
  catches it.

There is no screenshot, and there could not be one: nothing on screen changes.
The evidence above is a pair of measurements before and after, and a pair of
tests.

---

## 65. The number between two keys was never checked

The tree checks every number that enters it. The number *between* two numbers
was not one of them.

### The measurements

`craftkit_core::Value` documents its invariants:

```rust
/// Invariants the constructors maintain:
///
/// - `Num` is always finite and within [`crate::MAX_MAGNITUDE`].
```

and `MAX_MAGNITUDE` says why the range exists at all:

> A stored `f64` is written to a JSON file and read back by a future version
> of the program. Past this magnitude, the arithmetic that follows stops being
> meaningful long before it stops being finite, so the reader refuses it.

So every stored number is within ±1 000 000 000. The keyframes of an animated
property are stored numbers, so they are. But the number *between* two keys is
not stored — it is computed — and `blend` built it directly:

```rust
(V::Num(x), V::Num(y)) => V::Num(x + (y - x) * t),
```

`V::Num` is the variant, not the constructor. The constructor is `Value::num`,
and that is the one that refuses an out-of-range number.

The `t` in that expression is the *eased* progress, and an easing is allowed
to overshoot on purpose — `Easing::checked` says so:

> out_y is deliberately allowed outside 0..=1: that is how an ease-out
> overshoots, and clamping it would make every easing preset a lie.

So the curve overshoots, and the value overshot with it. Measured, through the
real evaluator, on a document whose two keys are ±`MAX_MAGNITUDE` and whose
first key carries a checked bezier that overshoots:

```text
MAX_MAGNITUDE = 1000000000
frame   0: largest resolved |value| = 1000000000,   level_out = 1000000000    (outside the range: false)
frame   8: largest resolved |value| =  885352521,   level_out =  885352521    (outside the range: false)
frame  12: largest resolved |value| = 1331603073,   level_out = 1331603073    (outside the range: true)
frame  16: largest resolved |value| = 1497942087,   level_out = 1497942087    (outside the range: true)
frame  20: largest resolved |value| = 1398236998,   level_out = 1398236998    (outside the range: true)
frame  24: largest resolved |value| = 1000000000,   level_out = 1000000000    (outside the range: false)
```

**A number 1.5 times the limit the tree allows a number to be**, in the plan
that the window draws and that `plan.evaluate` hands to an agent.

### And a second claim, in the same place, that was false

The evaluator's own comment on a filter node's refusal said:

> A refusal here means the incoming level was already non-finite, which cannot
> happen while every level comes from `apply_gain`.

**Not every level comes from `apply_gain`.** A *source* node takes its `amount`
directly:

```rust
NodeKind::Source => {
    if let Ok(amount) = amount_of(&props, diag, node) {
        level_out = amount;
    }
}
```

which is right — a source sets the level rather than scaling it — and it means
the source's level is its amount, unchecked, out of range, at 1 497 942 087.

So the hole was in the value *and* in the sentence about the value, and the
sentence was what would have stopped anybody looking.

### The fix

The interpolation clamps to the range the constructors hold to:

```rust
(V::Num(x), V::Num(y)) => {
    V::Num((x + (y - x) * t).clamp(-craftkit_core::MAX_MAGNITUDE, craftkit_core::MAX_MAGNITUDE))
}
```

The colour branch already clamped to 0..=255; the number branch did not, and
the number branch is the one the tree has a limit for.

**The overshoot is still an overshoot.** Clamping the *value* does not flatten
the *curve*: an ordinary value is nowhere near the limit, so the shape of the
easing is untouched — and there is a test that says so, because "clamp the
value" and "clamp the curve" are different decisions and only one of them was
made.

And the evaluator's comment now says what is true: a source's level comes from
its amount, and that amount is in range because every stored number is and
because the number between two keys is clamped to the same range.

### The lesson

**An interpolation is a second door into the value, and only one of them was
checked.** The tree checks every number that *enters* — through a constructor,
through a reader, through a command parameter. The number between two of them
*leaves*, and it was built by a line that did not go through the constructor
that holds the invariant. The invariant was written down, and it was true of
every constructor and false of the type.

**An overshoot in the curve is not a licence for the value to leave the
range.** The overshoot is deliberate — it is what an ease-out *is* — and the
range is also deliberate. Both can be honoured at once: the curve may go past
its endpoints, and the number it produces stays a number the tree can store,
write and read back.

**And the sentence was the thing that would have stopped the search.** "Every
level comes from `apply_gain`" is the kind of claim that saves an afternoon —
when it is true. It was false here, and it was false in the same file as the
bug, three lines above it. §62's lesson, again, in a new place: prose that
points at real code and describes it wrongly. That is the **sixth** instance
— after §53's fingerprint, §4's permissions, §61's `written_at`, §62's
recovery counter and §64's read.

### What now holds it

| Property | Held by |
|---|---|
| An overshooting easing stays inside the range a stored number may take | `an_overshooting_easing_stays_inside_the_range_a_stored_number_may_take` |
| The clamp does not flatten the overshoot of an ordinary value | `the_clamp_does_not_flatten_the_overshoot_of_an_ordinary_value` |
| An animated amount never takes the level out of range | `an_animated_amount_never_takes_the_level_out_of_range` |

and two mutations in `scripts/mutation-test-gates.sh`:

- the interpolation stops clamping, and the model's test catches it;
- the interpolation stops clamping, and the evaluator's test catches it.

There is no screenshot, and there could not be one: the number is out of range
in a plan, and a plan is drawn as a level bar that clamps it for the screen
anyway. The evidence above is a measurement through the real evaluator and a
pair of tests.

---

## 66. The widget names leaked a kilobyte a frame

Every widget that matters carries a name, so an agent can find it:
`toolbar.node.add`, `nodes.row.4`, `node.7.prop.amount`. The name is how an
agent says "that button" without guessing a pixel.

The names were `&'static str`:

```rust
pub struct AutomationId(pub &'static str);

pub fn toolbar(command_id: &str) -> Self {
    AutomationId(Box::leak(format!("toolbar.{command_id}").into_boxed_str()))
}
```

`Box::leak` is the only way to get a `&'static str` without paying for it — and
"without paying for it" is the whole problem. The box is never freed. The
process keeps it for as long as it runs.

**And the names are built every frame.** Not once a session: every frame, the
toolbar builds one name per command (21 of them), each node row builds two,
each property editor one, each keyframe row one. `Box::leak` in a per-frame
path is not a small leak; it is a *rate*.

### The measurements

Drawn with the real `craftkit_ui::frame`, in a loop, against the real sample
session (probe at `/tmp/probe66`):

```text
frames drawn      : 1000      RSS growth 1052 kB   -> 1077 bytes/frame
second run: 3000 frames, 3168 kB, 1081 bytes per frame      (linear -> a leak, not noise)
per second @60fps : 63 kB
per hour @60fps   : 221 MB
```

The second run is the one that matters. Noise does not scale with the number
of frames, and 1081 bytes a frame over 3 000 frames is 1081 bytes a frame over
1 000. It is linear, so it is a leak and not a measurement of the allocator.

**221 MB an hour**, for a window that is simply open and nobody is touching.

### Why nobody saw it

**The app does not repaint on its own.** egui redraws on demand — on input, on
a change, when a frame was asked for — so a leak in the egui path only shows
up if the draw loop is *driven*. A person using the app produces a few frames a
second at most, so the process grows by a few kilobytes a second and the
window looks perfectly healthy.

The raster path (`ui.render`, `ui.click`) does draw frames on demand, and it
runs at the same rate — but that path is driven by an agent, and an agent
closes the connection when it is done.

So the measurement had to be made by driving the frame loop directly. That is
why this section has a probe and not a screenshot: **the leak has no
appearance.** There is nothing to photograph and nothing to compare.

### The fix

The name owns its bytes:

```rust
pub struct AutomationId(pub String);
```

and the four builders use `format!` instead of `Box::leak`.

**The identity rule did not change, and did not need to.** egui identifies a
widget by an `Id`, and an `Id` hashes the *bytes* it is given, not the address
of the string that holds them: the old code hashed a `&'static str` and the new
code hashes the same bytes out of a `String`. So the same widget keeps the
same identity across frames. This is worth saying out loud because the
alternative was one line away and would have been a disaster: an id built from
the *pointer* would make every widget a different widget every frame — the
focus would be lost, a scroll would jump, a dragged widget would drop the
drag — and every one of those would look like a different bug.

One consequence: a lookup could no longer answer with a borrowed `&str` that
points into something which dies. The click handler used to answer with a name
borrowed from the registry, and the registry is a local that dies at the end of
the frame. The answer is owned now (`Click.hit: Option<String>`), and the
control channel's `ui.click` reply carries the owned name.

### The gate

`#![forbid(unsafe_code)]` is the tree's rule and it is the right one. The
honest way to count a program's allocations is a counting global allocator,
and that needs `unsafe impl GlobalAlloc` — which this crate may not write. So
the gate measures the kernel's own answer instead:

- `/proc/self/statm`'s second field is this process's resident pages: memory
  the allocator has not handed back, which is what a leak *is*;
- the measurement runs in a **child process**, because a leak is only visible
  against a baseline, and a baseline is only worth anything in a process that
  has done nothing else.

Measured on the fixed tree: **0 bytes** for 200 000 names × 4 builders. With
the leak back: **25 411 584 bytes** — twenty-four times the 1 MiB limit the
gate sets.

The gate is Linux-only, and the reason is stated rather than hidden: there is
no portable equivalent of `/proc/self/statm` without a new dependency or an
`unsafe` call, and the crate forbids `unsafe`.

### The lesson

**A `Box::leak` in a per-frame path is a leak.** `Box::leak` is the only way
to make a `&'static str` without paying for it, and the price is not paid at
all: it is paid by the process, forever, at the rate the frame runs. The
`&'static str` was convenient — it is `Copy`, and the registry's lookups were
written against one — and the convenience was the bug.

**A name that is stable and a name that is owned are two different questions.**
The old design answered "stable" with immortality, and immortality also
answered "owned", so the two looked like one question. They are not: the
identity comes from the *contents* of the name, and the contents can be owned.

**The app does not repaint on its own, so a leak in the egui path is
invisible in the running app.** Not slow, not subtle — *invisible*. A person
cannot see it and a screenshot cannot show it, because the symptom is not
pixels, it is the absence of pixels the process will not give back. The
evidence for this one is a probe, and the probe had to drive the draw loop
itself.

### What now holds it

| Property | Held by |
|---|---|
| A widget name owns its bytes | `a_widget_name_owns_its_bytes` |
| Building a widget name does not cost the process memory | `building_a_widget_name_does_not_leak_memory` |

and one mutation in `scripts/mutation-test-gates.sh`:

- the widget names leak their bytes again — and the gate measures 25 411 584
  bytes against a limit of 1 048 576.

There is no screenshot, and there could not be one: the leak has no appearance.
The evidence is a measurement of the process's own resident pages, before and
after, in a process that did nothing else.

---

## 67. The service worker precached a bundle that nothing checked

The browser build has exactly two hand-written files: `index.html` and `sw.js`.
Both of them hold a copy of the bundle's name, and the name is the one thing
that decides whether the page works.

The page imports it:

```html
const { default: init } = await import("./dist/craftkit_web.js");
```

and the service worker precaches it:

```js
const SHELL = [
  "./",
  "./index.html",
  "./sw.js",
  "./dist/craftkit_web.js",
  "./dist/craftkit_web_bg.wasm",
];
```

`cargo xtask web` checked the **page's** copy against the one constant:

```rust
pub fn bundle_path() -> String {
    format!("./dist/{}.js", crate::wasm::BUNDLE_NAME)
}
```

and it never looked at the **worker's** copy. The two were the same string
because somebody typed them the same way, and the gate's job — which is
exactly this — was to make sure they stayed the same.

### The measurements

**Rename the worker's copy** — the two `dist/` paths in `sw.js` become
`craftkit_web_v2.js` and `craftkit_web_v2_bg.wasm`, files that do not exist:

```text
── web
   ok: the canvas, the bundle path and the worker's rules all agree
   ok: apps/craftkit-web/sw.js parses
```

Green. And the gate's own sentence says the two agree.

**Rename the constant** — `wasm::BUNDLE_NAME` becomes `craftkit_web2`, so
*both* files now name a file that will not exist:

```text
error: 1 problem(s) in the browser build:
   • apps/craftkit-web/index.html: no import of `./dist/craftkit_web2.js`.
```

One problem. The page is reported. The worker is not — and the worker is the
one that decides what is in the cache when the network is gone.

### What it costs

The service worker's job is to have the application ready when the network is
not there. It gets there in two ways, and this breaks one of them:

- **The precache.** The install step fetches each entry of `SHELL` and stores
  it. A name that does not exist fails, and the failure is swallowed:

  ```js
  SHELL.map((url) => cache.add(url).catch((e) => console.warn("not cached", url, e)))
  ```

  A `console.warn` is not a report. Nobody reads the console of a page they
  are trying to use.

- **The runtime cache.** Every same-origin 200 is stored as it goes past. This
  one usually saves the day — but **not on the first visit**. The page registers
  the worker *after* it has imported the bundle:

  ```js
  const { default: init } = await import("./dist/craftkit_web.js");
  await init();
  // ... and only then:
  navigator.serviceWorker.register("./sw.js");
  ```

  so the very first request for the bundle never went through the worker and
  was never put in the Cache API. The precache is the only thing that puts it
  there on a first visit.

So the shape of the failure is: **the page is cached and the application is
not**. A user who goes offline gets the page — and the page cannot start,
because the module it imports is not in the cache and the network is gone. The
page's own failure box says *"craftkit did not start"*, which is §11's fix
doing its job, and the reason is in the console.

And it is **intermittent**, which is worse than a clean breakage: once the
worker has been controlling the page for a visit, the runtime cache has the
bundle and offline works. So the symptom is "offline is flaky", and the cause
is a line in a list that nobody looks at twice.

### The fix

The worker has a copy of the bundle's name, and now that copy is derived from
the same constant as the page's:

```rust
/// The path the bundler writes the wasm module to.
///
/// The bundle is two files: the JavaScript module the page imports and the
/// wasm behind it. `wasm-bindgen --out-name` writes both, and the second one
/// is named after the first. §67.
pub fn wasm_path() -> String {
    format!("./dist/{}_bg.wasm", crate::wasm::BUNDLE_NAME)
}
```

**Both** files have to be in the shell — the JavaScript module the page
imports and the wasm behind it — and the shell is **parsed**, not searched
for:

```rust
pub fn worker_shell(worker: &str) -> Vec<String> { … }
pub fn worker_caches_the_bundle(worker: &str) -> bool { … }
```

Parsing matters for the reason the canvas id has its own function: a
substring search over the whole worker stays true when the bundle's name
survives in a comment, and a check that a comment can satisfy is not a check.
A line that is only a comment is not an entry either.

The gate's success message now says what it checks:

```text
ok: the canvas, the bundle path, the worker's shell and its rules all agree
```

and it is true.

One more thing: the gate's own test fixture used to spell the bundle's name
out, so a rename of the constant would have made the *fixture* wrong rather
than the gate. It is built from the derived paths now.

### The lesson

**Two copies of one name: check both, or the gate is a claim about one of
them.** The page and the worker each held the bundle's name. The gate checked
one of them and reported on the agreement of two. A gate that checks half of
what it claims is worse than one that checks none, because the message says
"all agree".

**A gate's success message is a claim about the tree, and it has to be
true.** *"the canvas, the bundle path and the worker's rules all agree"* was
false: the bundle path in the worker disagreed with the page's, and the gate
had no way to know. §62's lesson, in a new place — the sentence that stops
somebody looking was the sentence the gate printed.

**A warning swallowed by a `.catch` is not a report.** The install step
prints `console.warn("not cached", …)` and carries on, which is the right
behaviour for a missing file and the wrong behaviour for a *wrong name*. The
difference between the two is not visible at the point where the two are
handled, so it has to be checked somewhere else — which is what the gate is
for.

### What now holds it

| Property | Held by |
|---|---|
| The wasm path is derived from the bundler's name | `the_wasm_path_is_derived_from_the_bundlers_name` |
| A worker that precaches a different bundle is caught | `a_worker_that_precaches_a_different_bundle_is_caught` |
| A bundle named only in a comment is not in the shell | `a_bundle_named_only_in_a_comment_is_not_in_the_shell` |
| The worker in the tree precaches the bundle the page imports | `the_worker_in_the_tree_precaches_the_bundle_the_page_imports` |

and one mutation in `scripts/mutation-test-gates.sh`:

- the worker precaches a bundle that does not exist — and `cargo xtask web`
  reports it.

There is no screenshot: the failure is a page that cannot start when the
network is gone, which needs a browser with the network switched off. The
evidence is the gate's own output, before and after, and the mutation.

---

## 68. The undo budget counted the memory it had just given back

§52 put a budget on the undo stack, because 128 snapshots of a 12 MiB document
cost 1.5 GiB. The budget is the right shape: it bounds the memory rather than
the number of steps.

A budget is only a budget if it counts what is really held.

### The accounting

The history holds two stacks and one number:

```rust
pub struct History {
    undo: Vec<Entry>,
    redo: Vec<Entry>,
    /// The memory both stacks hold together.
    bytes: usize,
}
```

Every place that lets go of an entry gives its memory back. `pop_undo` and
`pop_redo` move an entry from one stack to the other and the total does not
change; `drop_oldest_undo` subtracts; `clear` sets the count to zero.

One place did not:

```rust
pub fn push(&mut self, doc: Document, label: &str) {
    self.redo.clear();
    …
}
```

`push` is where a **new** edit arrives, and a new edit makes the undone branch
unreachable — so the redo stack is thrown away. `self.redo.clear()` frees that
memory, and the budget kept counting it.

So the number grew by the size of a discarded branch on every edit after an
undo, and the budget was then enforced against a phantom: it dropped undo
steps to make room for memory that no longer existed.

### The measurements

**The history on its own**, with a full-size document (10 000 nodes, measured
at 3 905 077 bytes) and the budget of 67 108 864 bytes:

```text
history: after 21 edit/undo cycles the undo stack is EMPTY
history: the budget says 66339399 bytes (1.0x the budget) for stacks holding nothing
```

Twenty-one cycles. And the number the budget reports — 66 339 399 bytes, 99%
of the whole budget — is for **two empty stacks**.

**The real session**, with the same document and the shape a person actually
produces (a few edits, then undo them all, then a few more):

```text
session: round  1: the undo depth after 8 edits was   8 (it should be 8), and 8 of them could be undone
session: round  2: the undo depth after 8 edits was   8 (it should be 8), and 8 of them could be undone
session: round  3: the undo depth after 8 edits was   5 (it should be 8), and 5 of them could be undone
session: undo has already lost steps in round 3.
```

**Three rounds of eight edits, and three of the eight steps are already
gone.** The steps are not reported as lost: the undo stack is simply shorter,
and the oldest steps are the ones that disappear — which is exactly what the
budget is *supposed* to do when the document is large, so nothing looks wrong.

And the end of it, which is the part a person would meet:

```text
history: after 21 edit/undo cycles the undo stack is EMPTY
```

**Undo stops working.** Not "undo is slower", not "undo is shallower": the
command answers *"there is nothing to undo"*, and the person has no way to
know that the stack was emptied by a budget counting memory it had already
given back.

### The fix

The discarded branch gives its memory back at the moment it is discarded:

```rust
/// Forget the redo branch, and give its memory back.
///
/// The budget counts the memory the two stacks hold *together*, so a stack
/// that is emptied has to be subtracted from it — the same way
/// [`drop_oldest_undo`] does.
fn clear_redo(&mut self) {
    for entry in self.redo.drain(..) {
        self.bytes = self.bytes.saturating_sub(entry.bytes);
    }
}
```

With it, the same probe says:

```text
history: cycle  25: undo steps kept  21, the budget says     66339399 bytes
history: cycle  50: undo steps kept  21, the budget says     66339399 bytes
history: cycle 125: undo steps kept  21, the budget says     66339399 bytes
session: round 20: the undo depth after 8 edits was   8 (it should be 8), and 8 of them could be undone
```

Twenty-one steps, and the budget says 66 339 399 bytes — which is exactly
21 × 3 159 019, the size of the twenty-one copies the stacks hold. The
number and the stacks agree, and the undo depth holds for twenty rounds.

**The number is now the sum of what is held**, which is the whole point of a
budget.

### A note on the unit

The 3 159 019 is not the 3 905 077 of the document the copies were made from,
and that is not a bug: a copy carries no spare capacity, so it really is
smaller than the document it was copied from. The budget counts the copies it
holds, and 21 copies really do cost 66 MB. It is worth knowing, because it is
the kind of difference that makes a test's expectation wrong for a reason that
has nothing to do with the test.

### The lesson

**A budget is only a budget if it counts what is really held.** The number was
right in every individual operation and wrong in the only operation that
mattered — the one that *frees* memory. Every other path gave the memory
back; the one that threw a whole stack away did not.

**A budget that is too pessimistic is a bug too, and it hides as a missing
feature.** The symptom was not "the app used too much memory" — it was "undo
does nothing", which reads as a missing feature and not as an accounting
error. The budget was doing its job perfectly: it was enforcing a number that
was wrong.

**The stack that grows is not the one that leaks.** The redo stack is the
smaller of the two, and it is the one that is thrown away — so the leak is in
the *removal*, and it only shows up after an undo. A budget that is only ever
tested by pushing never sees it.

### What now holds it

| Property | Held by |
|---|---|
| The budget counts the memory the stacks hold | `the_budget_counts_the_memory_the_stacks_hold` |
| A discarded redo branch gives its memory back | `a_discarded_redo_branch_gives_its_memory_back` |
| Undo still works after many edits that discard a redo branch | `undo_still_works_after_many_edits_that_discard_a_redo_branch` |

and two mutations in `scripts/mutation-test-gates.sh`:

- the undo budget counts the memory it has just freed — and the accounting
  test catches it;
- undo stops working once the redo branches pile up — and the symptom test
  catches it.

There is no screenshot, and there could not be one: the bug is that a
*feature* is missing. The evidence is the depth of the undo stack before and
after, through the real session, and the number the budget reports for two
empty stacks.

---

## 69. The failure counter counted the last 256 commands and called them the session

The command log is a bounded ring, on purpose:

```rust
/// How many log entries are kept.
///
/// Bounded, because a long session can run for days and the log exists to
/// explain the last few minutes, not to be a file.
pub const MAX_ENTRIES: usize = 256;
```

That is a good design: the log is for explaining *the last few minutes*, and a
session that runs for days must not grow a log for every minute of it.

The counters were taken over that ring.

### The measurements

Three hundred refusals and one success, through the real session:

```text
300 commands were refused, and one succeeded.
the reply says: 256 command(s), 255 failure(s)
the data says:  {"commands":256,"failures":255,"kernel_recoveries":0,"nodes":1,"undo_depth":1}

the log holds 256 entries
```

**Three hundred and one commands ran. The reply says 256.**

And the two numbers are wrong in a way that is worse than being wrong by a
little: they are wrong by *exactly the amount that has fallen out of the
ring*, and they stop moving there. A session that has run ten thousand
commands reports 256, and a session that has run ten thousand failures reports
255. The number is not a little stale — it is **frozen**.

The doc on the count said what it meant:

```rust
/// How many commands failed since the session opened.
pub fn failures(&self) -> usize {
    self.entries.iter().filter(|e| !e.ok).count()
}
```

"Since the session opened", over a ring of the last 256 commands. The doc was
false, and it was false in the direction that makes the number look like a
total — which is exactly why nobody looked.

And the command count was the ring's *length*:

```rust
format!("{} command(s), {} failure(s)", s.log().entries().len(), s.log().failures()),
```

`session.stats` is the command an agent asks to find out what the session has
done. It answered with the size of the ring.

### The fix

The log keeps two counters that are **not** bounded, because the questions
they answer are not bounded:

```rust
pub struct LogBook {
    entries: Vec<LogEntry>,
    seq: u64,
    /// How many commands have failed since the session opened.
    ///
    /// Counted here rather than derived from `entries`, because the entries are
    /// a bounded ring of the last [`MAX_ENTRIES`] commands: a count taken over
    /// the ring is a count of the last 256 commands, and it was reported as
    /// "since the session opened". §69.
    failures: usize,
}
```

`seq` was already there — it numbers the entries, and it counts every command
— and it was simply not being asked. Now it is:

```rust
/// How many commands have run since the session opened.
///
/// Every command, not only the ones still in the ring. §69.
pub fn total(&self) -> u64 {
    self.seq
}
```

and the statistics read the totals:

```rust
format!("{} command(s), {} failure(s)", s.log().total(), s.log().failures()),
```

The same probe, after:

```text
300 commands were refused, and one succeeded.
the reply says: 301 command(s), 300 failure(s)
the data says:  {"commands":301,"failures":300,"kernel_recoveries":0,"nodes":1,"undo_depth":1}

the log holds 256 entries
```

**301 commands, 300 failures, and the ring still holds 256 entries.** The
bounded part is bounded; the count is not.

### The lesson

**A bounded store and a count over it are two different things.** The ring
answers *"what happened just now?"*. A counter answers *"how much has
happened?"*. One of them is allowed to be bounded and the other is not, and
the failure count was the wrong one of the two.

**A frozen number is worse than a wrong one.** A number that is off by a
little gets corrected by the next reading. A number that stops at 256 and
stays there looks like a *total* — it is round, it is stable, and it is the
kind of number a person stops questioning. §61's `written_at` was the same
shape: a field that always said zero.

**A counter's scope is part of its meaning, and this is the second time.** §62
found a counter that counted only the thread that asked while its doc said
*"since the process started"*. This one counted only the last 256 commands
while its doc said *"since the session opened"*. Both were true as counts and
false as claims, and in both cases the claim was the only thing wrong.

### What now holds it

| Property | Held by |
|---|---|
| The failure count is every failure, not only the ones in the ring | `the_failure_count_is_every_failure_and_not_only_the_ones_in_the_ring` |
| The command total is every command, not only the ones in the ring | `the_total_is_every_command_and_not_only_the_ones_in_the_ring` |
| Clearing forgets the failures too | `clearing_forgets_the_failures_too` |
| The statistics count every command | `the_session_statistics_count_every_command_and_not_only_the_last_two_hundred_and_fifty_six` |

and two mutations in `scripts/mutation-test-gates.sh`:

- the failure counter counts only the commands in the ring — and the log's own
  test catches it;
- the statistics count only the commands in the ring — and the command's own
  test catches it.

There is no screenshot: the numbers are in a JSON reply. The evidence is the
reply itself, before and after, through the real session.

---

## 70. The trace's budget was in bytes of text, and the trace holds the parsed value

§52 bounded the undo stack in bytes, because a count of steps is not a bound
on memory. The command trace was bounded the same way, with the same
reason:

```rust
/// How many bytes of command trace a session keeps for a crash report.
///
/// Bounded because the trace is in memory for the whole session — but bounded
/// by **size**, not by a count of commands.
pub const MAX_TRACE_BYTES: usize = 256 * 1024;
```

The reason is memory. The unit is text.

### What the trace holds

A trace step is the command id and the parameters **as they were given**:

```rust
pub struct TraceStep {
    command: String,
    params: serde_json::Value,
}
```

and the size of a step was measured by serialising it:

```rust
fn step_size(step: &TraceStep) -> usize {
    serde_json::to_string(step).map(|s| s.len()).unwrap_or(0)
}
```

So the budget counted **bytes of JSON text**, while what the trace holds is the
**parsed** value — and §64 already measured what that costs:

> a JSON number is two bytes of text and thirty-two bytes of memory

The tree had the constant for this. `craftkit_core::JSON_EXPANSION` is 32, and
§64's rule is written down: *"a limit written in bytes of text has to be read
as a limit in bytes of memory thirty-two times larger."*

### The measurements

A payload of 40 000 numbers — the text of one step, and the trace's whole
budget:

```text
the trace holds 1 step(s)
the trace's text is      228958 bytes
the budget is            262144 bytes
the process grew by      3632 kB
so the memory per byte of text is 16.2x
```

**229 KB of text, 3.6 MB of memory.** The budget held, and the memory it was
supposed to bound was sixteen times bigger.

And the budget could not bound even *that*:

```text
the payload is 2688926 bytes of JSON text
the command was refused: true
the trace holds 1 step(s)
the trace's text is 2688958 bytes
the budget is 262144 bytes
the process grew by      15252 kB
the budget was exceeded by 2426782 bytes
```

**One step of 2.7 MB of text — ten times the budget — was kept whole, and it
cost 15 MB of memory.** The reason is the loop that keeps the trace inside
its budget:

```rust
while self.trace.len() > 1 && self.trace_bytes > crate::MAX_TRACE_BYTES {
```

`len() > 1` — the loop drops the *oldest* step and keeps the last one, because
the last one is the command that was running when the crash happened. So a
step that is bigger than the whole budget cannot be dropped: there is nothing
else to drop, and the one that is there is the one that matters.

And it is kept for the rest of the session, because the trace is the
reproduction — it is not written off at the end of a command.

### The fix

**Two things, because there are two defects.**

The budget now says what it bounds, and is derived from the memory it may
hold — the same shape §64 gave the readers of untrusted JSON:

```rust
/// How much memory the command trace may hold.
///
/// The trace is in memory for the whole session, and what it holds is the
/// **parsed** parameters of every command — not the text they arrived as.
pub const MAX_TRACE_MEMORY_BYTES: usize = 8 * 1024 * 1024;

/// How many bytes of command trace a session keeps for a crash report.
pub const MAX_TRACE_BYTES: usize = MAX_TRACE_MEMORY_BYTES / craftkit_core::JSON_EXPANSION;
```

The number is the same — 8 MiB at the tree's own expansion is 256 KiB of
text — but what it *means* is now written down.

**And a step that does not fit is refused**, before the document is cloned
and before the step is copied:

```rust
if step_bytes > crate::MAX_TRACE_BYTES {
    let err = EngineError::Param {
        id: id.to_string(),
        source: craftkit_core::ParamError::new(
            "params",
            format!(
                "they are {step_bytes} bytes of JSON, and the crash report's trace keeps at \
                 most {} — a command that cannot be recorded cannot be reproduced",
                crate::MAX_TRACE_BYTES
            ),
        ),
    };
    self.log.record(id, false, &err.to_string());
    return Err(err);
}
```

The same probe, after:

```text
the command was refused: true
the trace holds 0 step(s)
the process grew by      2748 kB
```

The trace holds nothing, and the process grew by the caller's own payload
instead of by a second copy of it that would have lived for the rest of the
session.

**Refusing the command, rather than dropping the step, is the point.** A
command that cannot be recorded cannot be reproduced — so it does not run, and
the crash report is complete. Dropping the step instead would have left the
report unable to reproduce the very command that crashed.

Every command a person or a script can legitimately run is far inside the
budget: a property's text is capped at 64 KiB by the model, and the budget is
256 KiB. The refusal is for the input nobody meant to send.

One consequence worth writing down: a crash report written *before* this rule
can hold a step the engine would now refuse, so `crashes replay` reports that
step as a failure rather than pretending it ran.

### The lesson

**A budget written in bytes of text is a budget in bytes of memory that
many times larger.** §64 found this for the readers of untrusted JSON and
gave the tree a constant for it. The trace was the one place that still
counted text and called it memory — and the constant was already there, one
crate away.

**A loop that cannot drop the last step cannot bound it either.** The trace
keeps the newest step on purpose — it is the one that was running when the
crash happened — and that deliberate exception meant a step bigger than the
whole budget was kept whole. A bound with an exception is not a bound.

**The exception and the bound have to be checked separately.** "We drop the
oldest until it fits" is only a bound if what is left *fits*, and the code
that says so has to say it about the last step too.

### What now holds it

| Property | Held by |
|---|---|
| A command whose parameters do not fit in the trace is refused | `a_command_whose_parameters_do_not_fit_in_the_trace_is_refused` |
| The trace never holds more than its budget | `the_trace_never_holds_more_than_its_budget` |
| The trace's budget is derived from the memory budget | `the_trace_budget_is_derived_from_the_memory_budget` |

and two mutations in `scripts/mutation-test-gates.sh`:

- a command too big for the trace is not refused — and the engine's own test
  catches it;
- the trace budget is a number of its own again — and the derivation test
  catches it.

There is no screenshot: the defect is a number of bytes in memory. The
evidence is the process's own resident pages before and after, and the size
of the trace.

---

## 71. The evidence file's own numbers were ninety cases out of date

This file is the record of what the audit found, and it ends with a list of the
ways prose has lied. It had three counters of its own, and all three were
wrong.

### The measurements

**§1's verification claim.** The first section says what re-running the
mutation script proves:

> Verified by re-running the mutation script: **28 of 29 caught**, and the
> nineteenth is the wasm limitation below

The script ran **122** cases by the time §71 was written. So the claim was
ninety-three cases out of date, and "the nineteenth" pointed at a case that had
moved. The claim is not *approximately* right: it says the instrument has
twenty-nine rules, and the instrument has a hundred and twenty-two.

And the paragraph immediately below it states the rule the file broke:

> The number in that line is computed by counting the cases the script runs,
> not written by hand.

**The counters in the last section.** Item 8 of "Suggested order" counts the
prose-drift instances and says how many sections they were found in:

> Six instances in sixty-six sections …
> Seven instances in sixty-seven sections.
> Eight instances in sixty-eight sections.

Three sentences, written on three different days, each one stale the moment the
next section was added — and each stale by exactly one, which is how you can
tell they were written by hand rather than computed.

**Nothing checked any of it.** `cargo xtask lists` reads the prose in `docs/`,
`skills/`, `AGENTS.md`, `README.md` and `CONTRIBUTING.md` — and
`KNOWN-GAPS.md` is not among them, which is defensible (this file is a
working document) but it means the file that tracks prose drift has no gate on
its own prose.

### The fix

**The document carries no number of its own.** §1 now says what the run
proves without putting a count in it:

> Verified by re-running the mutation script: **every case caught**, and the
> one that is not is the wasm limitation below …

and says why:

> This paragraph deliberately carries **no number of its own**. It used to say
> "28 of 29 caught", which was true on the day it was written and was **ninety
> cases out of date** by the time the next section was added — a hand-written
> count of a thing that grows is stale by construction.

The counters in item 8 lost their section counts for the same reason. The
*number of instances* stayed — nine is a fact about the findings, and the
person who finds the tenth writes it down — but the "in N sections" clause was
a number about a document that grows, and it went stale every time.

**The script's ratio is now checked, not asserted.** The sentence "computed by
counting the cases the script runs, not written by hand" is a claim about the
instrument, and it was the one claim in §1 that nothing enforced. `cargo xtask
lists` now reads the script and checks it:

```rust
pub fn mutation_ratio_is_computed(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let Some(index) = lines.iter().position(|l| l.contains("%d/%d caught")) else {
        return vec!["the mutation script prints no closing ratio at all".to_string()];
    };
    let arguments = lines.get(index + 1).copied().unwrap_or_default();
    …
}
```

The ratio must come from `"${#CAUGHT[@]}"` and `"$CASES"` — the arrays the
run filled in — and a literal fails.

### The one thing that cannot be done here

**This gate has no mutation case, and the reason is the medium rather than the
will.** bash reads a script as it goes, so a file that changes under a running
script is a corrupted run: the shell's read offset no longer matches the file,
and the run reads the wrong bytes. Every other mutation in the suite writes to
a file that is not the script doing the writing — `scripts/live-window-check.sh`
is mutated because it is *invoked*, not because it is *running*. The
instrument cannot be mutated by itself.

So the check is proved the way the tree proves a check that cannot be
mutated: with a **fabricated copy**. The test makes a script whose ratio is
written by hand and requires the check to reject it — the same shape as §26's
fabricated-id case.

That is a smaller guarantee than a mutation, and it says so here rather than
letting the matrix imply otherwise.

### The lesson

**A number written by hand about a thing that grows is stale by
construction.** The script's ratio is a fact about a run; the run prints it.
A copy of it in a document is a snapshot of a run, and a snapshot that is not
labelled as one is a claim.

**The file that tracks prose drift has prose drift in its own counters** — and
that is worth more than the individual numbers. §8 asks "how is prose kept
true?", and the answer it was reaching for is now visible from the inside: the
counters were kept true by *writing them down again*, which is the process
the question is about.

**A gate that cannot be mutated must say so.** There is exactly one gate in the
tree that is proved by a fabricated copy rather than by a mutation, and it is
the gate about the instrument that cannot be mutated while it runs. Saying that
is part of the result.

### What now holds it

| Property | Held by |
|---|---|
| The instrument's closing ratio is computed, not written by hand | `the_mutation_script_computes_its_own_closing_ratio` |
| A ratio written by hand is caught | `a_closing_ratio_written_by_hand_is_caught` |

and **no mutation**, for the reason above: the instrument cannot be mutated
while it is running.

There is no screenshot, and there could not be one: the defect is a number in
a document. The evidence is the script's own output against the document's
claim — 122 cases against "28 of 29".

---

## 72. The crash report reader parsed sixty-four megabytes of text into a tree

§48 put a bound on a crash report's **file**: at most 64 MiB, read through the
shared bounded reader, so a huge file costs one buffer rather than one huge
allocation. The bound was on the **text**. What the reader did with the text
was a different question, and nobody had asked it.

### What the reader held

```rust
pub struct CrashReport {
    /// The document at the moment of the crash.
    pub design: serde_json::Value,
    /// The document the trace starts from.
    pub base: serde_json::Value,
    …
}
```

A report is **input**: `docs/recipes/fix-a-crash.md` asks for `report.json`
from whoever had the crash, so the file arrives from another machine. §48
said exactly that, and capped the file — and the cap is what made the
question sharp, because the file was then parsed into a `serde_json::Value`,
and §64 had already measured what a value costs:

> a JSON number is two bytes of text and thirty-two bytes of memory

So the 64 MiB the reader accepted was a **two gigabyte** budget. The limit
written to keep a hostile file out was the thing that let the hostile file in:
a report of sixty-six megabytes cost a gigabyte and a quarter of memory to
read, on the machine of the person who had the crash.

### The measurements

Before the change, with `/usr/bin/time -v` on the real `craftkit-cli crashes
show`:

```text
a 59 200 132-byte report:    804 028 kB of resident memory
a 66 000 101-byte report:  1 260 292 kB of resident memory, 2.37 s
```

The second report's design was thirty-three million numbers — a design that is
not a design, in a file that is a valid report. Nothing refused it, because a
tree holds anything.

After the change, the same file and the same command:

```text
a 66 000 101-byte report:     67 796 kB of resident memory, 0.04 s
```

Eighteen times less memory, fifty-nine times less time — and the report is
now **refused**, with a message that names the file and the reason.

The gate's own numbers, on a smaller hostile report of 32 000 178 bytes
(sixteen million numbers where the nodes should be):

```text
read as a document:  the process peaked at  35 381 248 bytes, and the report was refused
read as a tree:      the process peaked at 547 860 480 bytes
```

And on a **legitimate** report — 2 003 nodes, 3 895 915 bytes of text:

```text
read as a document:    9 986 048 bytes of resident memory, 12.8 ms
read as a tree:       30 064 640 bytes of growth,      54.9 ms
```

A tree costs three times as much on an honest report and seventeen times as
much on a hostile one. The difference is the hostile one: a tree has to be
**finished** before anybody can say the file is not a document, and a document
is refused at the first number.

### The fix

**The report carries documents, not trees.**

```rust
pub struct CrashReport {
    /// The document at the moment of the crash.
    pub design: craftkit_model::Document,
    /// The document the trace starts from.
    pub base: craftkit_model::Document,
    …
}
```

The file format does not change: a document's serialization and a value's are
the same shape, so a report the old tree wrote still reads — with one
difference in the bytes, because a value's map is sorted and a document's is
not. `a_report_the_old_tree_wrote_still_reads` writes a report through a
value and reads it back, because a bug report collected on one version has to
be readable on the next.

The comparison the replay makes changed with it. It used to compare the
*serialized forms*, which is why it had to skip the id counter by hand; it now
compares the two documents:

```rust
if recorded.meta == replayed.meta && recorded.nodes == replayed.nodes {
    return Vec::new();
}
```

and the counter is not part of a document's serialization at all — `next_id` is
`#[serde(skip)]`, and the reader reseats it (§48), so two documents with the
same nodes and the same settings are the same document whatever their counters
say.

**And the reader has a budget that says what it bounds.** The cap on the text
was already there; what was missing was the number that says what the text
costs:

```rust
pub fn report_memory_budget(text_bytes: usize) -> usize {
    text_bytes.saturating_add(2 * craftkit_model::limits::MAX_DOCUMENT_BYTES)
}
```

The report's design is a document, and one document may hold
`MAX_DOCUMENT_BYTES` — thirty-two mebibytes. Reading the report costs the
text as well, because the text is held while it is parsed, and the reader's
own room on top of that. So the budget is the text plus twice the document's:
one for the document, one for the reader. A reader that starts building a tree
crosses it long before the tree is finished.

### What now holds it

| Property | Held by |
|---|---|
| A report whose design is not a document is refused | `a_report_whose_design_is_not_a_document_is_refused` |
| A report the old tree wrote still reads | `a_report_the_old_tree_wrote_still_reads` |
| Reading a report costs no more than its text and one document | `reading_a_report_costs_no_more_than_its_text_and_one_document` |

and one mutation in `scripts/mutation-test-gates.sh`:

- the report is read into a JSON tree before it is read as a report — the old
  shape exactly, and it **compiles**, because the report's own types did not
  change, so only a measurement can catch it. Measured: the peak goes from
  35 381 248 bytes to 547 860 480.

The measurement is of the **peak**, in a child process that has done nothing
else, because that is what a reader's cost is. It is Linux-only: a counting
global allocator would say the same thing everywhere, but the crate forbids
`unsafe` and `unsafe impl GlobalAlloc` is unsafe, so one cannot be written —
the same wall §66 found.

There is no screenshot: the defect is a number of bytes in memory. The
evidence is the process's own resident pages, before and after, on both
shapes of the same read.

### The lesson

**A report is input, not output.** Everything else in the crash folder is
written by this tree and is therefore whatever this tree made it. The report
is the one file in the folder that arrives from somewhere else, and it was the
one file read into the most expensive representation available. An input is
read into the type it claims to be.

**A cap on the text is not a cap on the memory.** §64 found this for the
readers of untrusted JSON, §70 found it for the command trace, and this is
the third: the report's cap was on bytes of text and the parse was
thirty-two times bigger. The constant that says so —
`craftkit_core::JSON_EXPANSION` — was one crate away the whole time, and the
reader that needed it most was the one that could kill the machine of the
person who had the crash.

**The type of a field is a budget.** `serde_json::Value` reads like a
neutral container and is not one: it costs thirty-two bytes for every two
bytes of text, it holds anything, and it makes "is this file a report?"
answerable only after the whole file has been read. A field typed as the
model's own document is a different kind of decision: it says what the file
must be, and it makes the reader's cost proportional to the text.

---

## 73. The size estimate did not count a property's value, and a file was a third door into it

Two defects, one cause: a value can be made in three ways, and only two of
them were checked.

### The three doors

`craftkit_core::Value` documents its own invariants:

> - `Num` is always finite and within [`crate::MAX_MAGNITUDE`].
> - `Text` is always within [`crate::MAX_TEXT`].
> - `List` is always within `MAX_LIST` entries.

and the doc says which doors exist:

> The constructors are not the only way a value is made: a value can also be
> *interpolated* between two of them …

Two doors: the constructors, and the interpolator. §65 found the second one
and closed it — the interpolator now clamps to the same range, because an
interpolation "is not one of the numbers the tree checked".

**The third door is deserialization**, and it was the widest one. A file is
deserialized straight into the model: `Document` → `Node` → `Property` →
`Value`, and no constructor runs on the way in. The invariants above were
kept by three constructors and by nothing else.

So a hand-written file could put into a property:

| Value | The constructors say | The file said |
|---|---|---|
| `{"t":"num","v":1e300}` | `MAX_MAGNITUDE` is 1 × 10⁹ | **accepted** |
| `{"t":"text","v":"x" × 8 388 608}` | `MAX_TEXT` is 65 536 | **accepted** |
| `{"t":"list","v":[… 10 000 items …]}` | `MAX_LIST` is 4 096 | **accepted** |

All three measured, through the real reader (`craftkit_io::load_text`).

And the evaluator's comment says, of the number it uses:

> The amount is in range because every number the tree stores is …

That was false. A file stores numbers the tree never checked.

### The second defect: the estimate

The same hole was open in the budget. §52 gave the document a byte budget and
the undo stack a byte budget, and both are measured with the document's own
size estimate — whose contract is:

> Never under-estimates: a budget a document can lie about is not a budget.

The estimate counted a property's **label** and its **keyframes**. It did not
count the property's **value**:

```rust
pub fn estimated_bytes(&self) -> usize {
    std::mem::size_of::<Property>()
        + self.label.capacity()
        + self.keys.keys().iter().map(|k| size_of::<Key>() + k.value.estimated_bytes()).sum::<usize>()
}
```

And §52's own mutation says why it was missed the first time:

> The count that matters is the keyframes' — that is where a document's memory
> goes.

That sentence was true of an honest document and false of a hostile one, and
it is the sentence that stopped the looking.

### The measurements

A file with one hundred properties, each holding a text of 64 KiB — every
value **inside** its own limit, so nothing but the estimate was wrong:

```text
the file is 6 561 564 bytes of text
the document holds      30 258 bytes by its own estimate
the values really hold 6 556 800 bytes
```

**Two hundred and seventeen times short.** The document budget (32 MiB) and
the undo budget (64 MiB) were both measured with that number.

The consequence, through the real session — two hundred commands, each
pushing a snapshot onto the undo stack:

```text
after 200 commands the undo stack holds 128 step(s)
the undo budget is 67108864
the process grew by 870211584 bytes
```

**Eight hundred and thirty megabytes**, against a budget of sixty-four, and
the stack was at its maximum of 128 steps: the budget never bound, because
the number it was measuring was a lie.

### The fix

**The value is counted.**

```rust
pub fn estimated_bytes(&self) -> usize {
    std::mem::size_of::<Property>()
        + self.label.capacity()
        + self.value.estimated_bytes()
        + self.keys.keys().iter().map(|k| size_of::<Key>() + k.value.estimated_bytes()).sum::<usize>()
}
```

**And a deserialized value is held to the same rules.** `craftkit_core` grows
a check, and the model calls it for every value in a document:

```rust
pub fn check(&self, what: &str) -> Result<(), crate::CoreError> { … }
```

```rust
p.value.check(&p.label)?;
for key in p.keys.keys() {
    key.value.check(&p.label)?;
}
```

The reader refuses the file instead of believing it:

```text
the document in this file is not valid: invalid: node n1:
the value of amount is out of range: 1e300 (allowed -1000000000..=1000000000)
```

and the budget binds again:

```text
the document holds  6 587 058 bytes by its own estimate
after 200 commands the undo stack holds 9 step(s)
the process grew by 80019456 bytes
```

**Ten times less memory, and the stack holds nine steps instead of a hundred
and twenty-eight.**

### The sentences

Four sentences were wrong, and they are the reason the defect survived:

- the estimate's own doc — *"Never under-estimates"* — with the value not
  counted;
- §52's mutation comment — *"the count that matters is the keyframes'"*;
- the evaluator — *"every number the tree stores is"* in range;
- the interpolator — *"the keys are numbers the tree checked"*.

All four are now true, and all four were false in the same way: they said
what the tree *did* rather than what it *checked*.

### What now holds it

| Property | Held by |
|---|---|
| The estimate counts a property's value | `the_size_estimate_counts_the_property_value` |
| A value that breaks an invariant is caught | `a_value_that_breaks_an_invariant_is_caught` |
| A document with a value the constructors would refuse is invalid | `a_value_the_constructors_would_refuse_is_refused` |
| A file whose value is out of range is refused | `a_file_whose_value_is_out_of_range_is_refused` |

and two mutations in `scripts/mutation-test-gates.sh`:

- the size estimate does not count the property's value — and the estimate
  test catches it;
- a value that breaks an invariant is accepted — and the file reader's test
  catches it.

There is no screenshot: the defect is a number of bytes in memory and a
number in a file. The evidence is the process's own resident pages and the
reader's own refusal.

### The lesson

**An invariant kept by a constructor is only kept on the paths that call the
constructor.** The doc said "the constructors maintain" — which was honest —
and everything downstream read it as "the tree maintains". A file does not
call a constructor, and a file is the only way a document arrives from
outside.

**When you find the doors, count them.** §65 found the interpolator and
wrote it down as *the* second door. Writing down a door you have closed is
how the next one gets missed: the sentence "the constructors are not the only
way" became "the constructors and the interpolator", and the third was
neither guessed nor counted.

**A budget is measured with an estimate, and the estimate is part of the
budget.** §52's contract — "never under-estimates" — was not false; it was
*unimplemented*, and the sentence above it was the implementation. The
mutation that should have caught it zeroed the keyframes' count, which is
where an honest document's memory is; the value's count, where a hostile
document's is, was never in the mutation because nobody had looked.

---

## 74. The limits table had a limit that nothing enforced

`crates/model/src/lib.rs` has a module called `limits`, and two documents
quote it as the tree's list of safety limits:

> | Hard limits | `craftkit_model::limits`: `MAX_NODES` 10 000, `MAX_DEPTH` 32,
> `MAX_PROPERTIES` 50 000, `MAX_KEYS` 10 000, `MAX_PATH_SEGMENTS` 16,
> `MAX_NAME` 255, `MAX_EDGES` 20 000 …

— `docs/architecture.md`, and `docs/recipes/add-a-saved-field.md`, which tells
an agent adding a field to *use* the limits in that table.

One of them was not a limit at all.

```rust
/// Edges the reverse index may hold.
pub const MAX_EDGES: usize = 20_000;
```

### The measurement

A count of the tree's own limits, and where each one is used:

| Limit | Used in code |
|---|---|
| `MAX_NODES` | 4 places |
| `MAX_DEPTH` | 2 |
| `MAX_PROPERTIES` | 1 |
| `MAX_KEYS` | 2 |
| `MAX_PATH_SEGMENTS` | 2 |
| `MAX_NAME` | 6 |
| `MAX_DOCUMENT_BYTES` | 4 |
| **`MAX_EDGES`** | **0** |

Every limit in the tree was checked the same way — a name, then every place
the name appears in the Rust sources — and `MAX_EDGES` appeared in exactly
one: its own declaration.

**There is no reverse index.** `Document::index_of` is a scan:

```rust
pub fn index_of(&self, id: NodeId) -> Option<usize> {
    self.nodes.iter().position(|n| n.id == id)
}
```

The constant bounded a thing that does not exist, and the thing it was
supposed to bound — a node lookup — is bounded already, by `MAX_NODES`.

### Why it mattered

The number was wrong in a way that is easy to act on. An agent reading the
architecture table is told the tree caps something at 20 000; the recipe
tells it to cap a new field with a limit *from that table*. A cap taken from
a limit that enforces nothing is a cap in a comment: the file it protects
still reads whatever it holds.

And the sentence is a false one, which is the shape §53, §61, §62, §64, §65,
§67, §69, §71 and §73 all found: **"Edges the reverse index may hold"**
describes an index the tree does not have.

### The fix

**The limit is gone.** A table of limits should list limits that are
enforced, and the lookup it was meant to bound is bounded by `MAX_NODES`.

**And the table now says so.** `cargo xtask lists` reads the table and checks
that every constant in it is used in code somewhere else in the tree:

```rust
pub fn limits_are_enforced(root: &Path) -> Result<Vec<String>, String> { … }
```

A mention in a comment is not enforcement, and neither is a mention in prose,
so both are left out: the check reads the code, with the comments removed.

The check is proved the way the tree proves a check that reads a file it
cannot change — the §26 shape — with a **fabricated copy**: a limits table
with a constant in it that nothing uses, and no code that uses it. The check
has to report it, and the same tree with the limit used has to be clean.

### What now holds it

| Property | Held by |
|---|---|
| Every limit in the model's table is enforced in code | `every_limit_in_the_table_is_enforced` |
| A limit nobody enforces is caught | `a_limit_nobody_enforces_is_caught` |
| A limit only a comment mentions is still caught | `a_limit_only_a_comment_mentions_is_still_caught` |

and one mutation in `scripts/mutation-test-gates.sh`:

- the limits table grows a limit nobody enforces — and `cargo xtask lists`
  catches it.

### The honest limit of the gate

**It is one-sided.** The check says every limit in the table is enforced; it
does not say every limit *named in prose* is in the table. A document that
quotes a limit the code no longer has is not caught by this check — the
existing prose check compares a named constant's **value** with the code's,
and a constant the code does not have has no value to compare.

That is a real gap and it is written down rather than papered over: the two
documents that quoted `MAX_EDGES` were updated by hand, and the next one
would not be caught. The general shape — *a name in prose that the code does
not have* — is the same one §26 closed for commands, and it is a bigger
change than this section's.

### The lesson

**A limit in a table of limits is a claim that something enforces it.** The
table is the tree's answer to "what bounds this?", and an entry in it that
bounds nothing makes the answer wrong in the way that is easiest to act on:
the number is there, it is plausible, and using it feels like using the
system.

**A check that reads the tree can be proved with a copy of the tree.** The
mutation script cannot mutate the file it is running, and the same is true
of a check that reads the tree's own source: the honest proof is a
fabricated tree with the defect in it. §71 said so for the script's own
ratio; this is the same shape for a check that reads the limits table.

**A constant nobody uses is a sentence, and the sentence has a name.** The
way this was found was counting: every limit in the tree, and where each one
is used. The count was one line of `grep` per limit, and it found a limit
that had been in the table, quoted in two documents, for as long as the
documents had been quoting it.

---

## 75. The crash recipe quoted a report that no longer existed

§72 changed the crash report's shape: `design` and `base` became
`craftkit_model::Document` instead of `serde_json::Value`. The engine's own
documentation was updated in the same change. **The recipe was not.**

`docs/recipes/fix-a-crash.md` — the document somebody opens when their
application crashed — quoted the whole struct:

```rust
pub struct CrashReport {
    pub id: String,
    pub unix: u64,
    pub command: String,     // which command was running, when one was
    pub message: String,
    pub design: serde_json::Value,   // the document at the moment of the crash
    pub base: serde_json::Value,     // the document the trace starts from
    pub replay: Vec<ReplayStep>,     // the commands, oldest first
    pub dropped: usize,              // commands the trace could not keep
    pub at: i64,                     // the playhead, in ticks
}
```

Two of those fields were wrong. The recipe went on describing a report that
the tree could not produce, and it is the *recipe*: the one place a person is
told what a crash report looks like.

### Why nothing noticed

Because the block was not marked. §40 built the contract for this in the
tree's own words:

> A fenced block whose info string says `exact` is a claim that it is the
> source, word for word, and this check holds that claim: every function such
> a block quotes must be the function in the file the block names, line for
> line, in both directions.

The recipe's block said nothing of the kind, so it was an *illustration* —
and an illustration is allowed to drift. That is the right default: most
blocks in the tree are illustrations with elisions in them, and a gate that
held them all would cry wolf.

The hole was that **only functions were held to the contract**. A type is
quoted as often as a function is, and a struct's shape rots the same way:
`design` became a document and the quote still said a JSON tree.

### The fix

**The quote is right, and it says it is exact.**

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

**And a struct is held to the contract too.** The check that §40 built now
compares the fields of a quoted struct with the fields of the struct in the
source — the same names, in the same order, with the same types — and it
reads **every** prose file, not only the pattern documents, because the file
that broke it was a recipe:

```rust
pub fn quoted_code_matches_the_source(root: &Path) -> Result<Vec<String>, String> { … }
```

Three other blocks that already matched the source are marked `exact` too —
`CommandSpec` in the command pattern, `DocumentMeta` in the saved-field
recipe, and `Tick` in the time pattern — so the contract covers four quotes
rather than one. `Tick` is a **tuple** struct, and a tuple struct has no
field names: the comparison is on the types, in order, because the type is
the whole claim (`Tick(i128)` is why the tree's time is exact).

The check is proved the way the tree proves a check that reads the tree: with
a **fabricated copy**. A document with an exact block quoting a struct, and a
source file with the struct in it — the quote's type drifted, then fixed.

### What now holds it

| Property | Held by |
|---|---|
| A quoted struct that drifted is caught | `a_quoted_struct_that_drifted_is_caught` |
| A quoted field the source does not have is caught | `a_quoted_field_the_source_does_not_have_is_caught` |

and one mutation in `scripts/mutation-test-gates.sh`:

- an exact quote of a struct is not the struct in the source — and
  `cargo xtask lists` catches it.

### The honest limit of the gate

**Only a block that says `exact` is held to it.** An unmarked quote is an
illustration and may drift — and one of them must: the control-channel
recipe shows a `Method` enum with one variant and an elision, as an example
of a change to make, and the variant it names does not exist yet. Marking
that one exact would be a lie, so it is not marked.

So the gate holds the quotes that *claim* to be exact. What it cannot do is
notice a quote that should have been exact and is not — the same one-sidedness
§74's gate has, and the same answer: a rule, not a check.

### The lesson

**This tree caused this drift, one section after writing the rule about it.**
§73's lesson was "when you find the doors, count them", and item 8 has said
since §53 that *"a change to a mechanism re-reads the documents that describe
it"*. §72 changed a type and did not re-read the one document that quotes it
— in the same stretch, by the same hand, while writing that rule down for
the eleventh time. **The twelfth instance is the one the tree made itself**
(§74 was the eleventh), and that is worth more than the ones before it: the
rule was not wrong, it was not adopted.

**A quote that says it is exact is checked; a quote that does not is an
illustration.** The difference between the two is a word in the fence, and
the word is a contract. §40 already had that contract for functions; §75
extends it to types, because a struct's shape rots exactly as a function's
body does, and the recipe that broke was quoting a struct.

**A type is quoted as often as a function.** The tree's gate covered one of
the two and not the other, and the uncovered one is the one a *recipe* uses:
a recipe shows you a shape so you can write against it, and a shape that is
two refactorings out of date is worse than no shape at all.

---

## 76. The crash report was not written for the crash the report is for

The panic hook's own comment says what it is for:

> This is for the panics that escape anyway — a panic on the render thread, a
> panic in a widget, a poisoned lock turned into an unwrap somewhere we do
> not own.

**A panic in the drawing code is the most likely crash there is.** And it
wrote nothing at all.

### The measurement

The hook takes the session with `try_lock`, and the reason is sound:

> **It must not block for long.** The session may be locked by the thread that
> panicked. `try_lock` is used rather than `lock` for exactly that reason: a
> hook that waits for a lock held by a dead thread is a hang that looks like a
> freeze.

And the render path holds that lock for the length of a frame:

```rust
let Ok(mut session) = self.session.lock() else { … };
let _ = craftkit_ui::frame(ctx, &mut session, &tokens);
```

So a panic in a widget happens **while the lock is held**, the hook's
`try_lock` fails, and the hook writes nothing. Measured, with the real hook
and a real panic on a thread that held the lock:

```text
reports written: 0
```

The report exists for the case the hook cannot serve. The document the user
was working on was there, in the session, one lock away.

### The promises

Three sentences promised a report for crashes that get none:

- the engine's module doc: *"the engine writes both, automatically, on every
  unclean exit"*;
- the same file: *"Called from the panic handler and from the unclean-exit
  path"*;
- the recipe: *"The engine writes a report on every unclean exit and from the
  panic hook."*

**There is no unclean-exit path.** The only writer in the tree is the
desktop app's panic hook: a process that is killed, or that loses the power,
writes nothing, and there is no code that could notice — catching a signal
needs `unsafe` FFI, and this tree forbids `unsafe` (`#![forbid(unsafe_code)]`).

### The fix

**The render path catches its own panics**, while the session is still in
hand, and writes the report from there:

```rust
fn draw_guarded(
    session: &mut Session,
    dir: Option<&std::path::Path>,
    draw: impl FnOnce(&mut Session),
) -> Result<(), String> { … }
```

The application survives it too: a panic in a widget is a bug, not a reason to
lose the work, and the report is the evidence.

**And the promises are corrected.** The engine's doc, and the recipe, now say
what is true: a report is written when a panic is *caught* — by the hook, or
by the render path — and a process that is killed writes nothing.

### What now holds it

| Property | Held by |
|---|---|
| A panic in the drawing code writes a report | `a_panic_in_the_drawing_code_writes_a_report` |
| A frame that does not panic writes no report | `a_frame_that_does_not_panic_writes_no_report` |

and one mutation in `scripts/mutation-test-gates.sh`:

- a panic in the drawing code writes no report — and the app's own test
  catches it.

### Two smaller things this found

**A `&Box<dyn Any>` is not a `&dyn Any`.** The message out of a caught panic
came out as "a panic with no message", because

```rust
message_from_payload(&payload)      // the box, not the value inside it
```

coerces to a trait object of the **box**, and the downcast inside the box
never matches. `payload.as_ref()` reaches the value. The same function, called
with two types that look the same, and one of them silently wrong.

**The panic hook is process-global, so two tests that panic interfere.** The
hook writes a report for *any* panic, including one another test caught, so a
test that counts the reports in its folder can see a stranger's — and worse,
the hook that ran for one test's panic was the one *another* test had
installed, with that test's folder, so the report landed in the wrong
directory. The suite was flaky before this section, and the new test made it
fire. The tests that touch the hook now take a lock, and the hook test counts
the reports that are *its* panic, by message.

### The lesson

**A lock held for the length of a frame is a lock held for the length of a
crash.** The hook's `try_lock` is right — a blocking lock would deadlock on
the panicking thread — and the conclusion that follows from it was not drawn:
if the crashing thread holds the lock, the crash report has to be written by
the crashing thread, before it lets go.

**The case a document names is the case to test.** The hook's comment named
"a panic in a widget" as the reason it exists, and the test suite never
produced one. The test that was missing was not hard; it was the shape of
the real thing — a panic on a thread that holds the lock.

**"On every unclean exit" is a promise about a path that does not exist.**
Three sentences said it, and the tree has exactly one writer. A promise about
a path is a promise about code, and the code was not there. This is the
thirteenth prose-drift instance, and the first one that was a promise about
*behaviour* rather than about a value.

---

## 77. The prose counted the commands by hand

§71 found the evidence file's own numbers ninety cases out of date, and drew
a rule from it:

> **A number written by hand about a thing that grows is stale by
> construction.**

The tree has a thing that grows — the command registry — and three documents
counted it by hand:

> The registry currently holds 21 commands across seven areas — `document`,
> `node`, `property`, `keyframes`, `playback`, `history`, `debug` — from
> `doc.rename` to `session.stats`.

— `docs/patterns/everything-is-a-command.md`, and:

> Every entry point reaches the same 21 commands.

— `AGENTS.md`.

All the numbers are right today: the registry has 21 commands and 7 areas.
They are wrong the moment a command is added, and **nothing would have said
so**: the tree's gate checks that a document never names a command the
registry lacks (§26), and that is the wrong direction — a *missing* name is
caught, a *count* is not.

The third was the sharpest, because it was not even a claim about the
registry's size — it was a piece of *advice*:

> | the registry entry | `crates/engine/src/command.rs:219` | `&NODE_ENABLE,` in
> `(area, id)` order, and `21` becomes `22` |

An agent reading the playbook is told to add a row and to bump a number that
is written down three files away. The row does not carry a number; the array
just grows. The line now says "the array grows by one".

The `Area` enum's variants are the areas, so "7 areas" is a number about a
thing that grows too, and it was written in words — which no check can see.

### The fix

**The numbers are checked against the code.**

```rust
pub fn counts_in_prose_match_the_code(root: &Path) -> Result<Vec<String>, String> { … }
```

The registry's commands are read from the source — the same way §26's check
reads them, and for the same reason: the build tool should not have to be the
thing it builds — and the areas are the `Area` enum's variants.

A number in prose is a count when it is a run of digits, in its own token,
immediately before the noun: "21 commands", "7 areas". A number inside a
longer token — "v1.21 commands" — is not a count, and the check says so
rather than crying wolf.

**And the areas are written in digits**, because a number in words is
invisible to a check that reads digits. That is the honest limit of this
gate, and it is written down: "seven areas" would pass unchecked, so the
prose says "7 areas" and the gate can see it.

### What now holds it

| Property | Held by |
|---|---|
| The numbers in the prose are the code's numbers | `the_numbers_in_the_prose_are_the_codes_numbers` |
| A count in prose that is wrong is caught | `a_count_in_prose_that_is_wrong_is_caught` |
| A number inside a token is not a count | `a_number_inside_a_token_is_not_a_count` |

and one mutation in `scripts/mutation-test-gates.sh`:

- the number of commands is not the registry's number — and `cargo xtask lists`
  catches it.

### The lesson

**A check in one direction does not cover the other.** §26's check says a
document may not name a command that does not exist; it says nothing about
how many commands exist, and a count is the same kind of claim as a name. The
gate was not wrong — it was *one-sided*, and the one side it did not cover is
the side that rots.

**The rule from §71 was written about a document and not applied to the
code.** The rule is general — a number about a growing thing — and the tree
had two instances of it, in the two documents an agent reads first. Writing
the rule down is not the same as obeying it, and §71's own lesson was that
the file broke the rule two lines below the stale number.

**A number in words is a number no check can see.** "seven areas" reads
better than "7 areas", and it hides. The fix was not to make the check
understand words; it was to write the number the way a check can read it, and
to say which numbers the gate cannot see.

**And a count can hide in a shape the check does not look for.** The check
reads a number immediately before the word "commands"; the playbook's "21
becomes 22" is a number with no noun at all ✗ — and it was found by reading
the document, not by the check. The check sees the counts it can see; the
others are a reading.

---

## 78. The pattern document said "two boundaries, and no more", and the tree had four

`docs/patterns/one-trust-boundary.md` is the tree's rule for catching
panics, and it is unambiguous:

> **Two boundaries, with distinct jobs, and no more.**
>
> | Boundary | Catches | Restores | Answers with |
> |---|---|---|---|
> | `craftkit_kernel::guard`, … | panics in code craftkit does not own | nothing | `KernelError::Internal` |
> | `Session::execute` | everything else … | the pre-command document snapshot | `EngineError::Internal` |

and, further down:

> If a `catch_unwind` appears in production code outside the two boundaries
> above, it is a bug in the design, not a safety net.

**The tree had three such boundaries before §76, and four after it.** The
count, from the source:

| File | `catch_unwind` | In the document? |
|---|---|---|
| `crates/kernel/src/lib.rs` | the guard | yes |
| `crates/engine/src/session.rs` | `Session::execute` | yes |
| `crates/control/src/server.rs` | the `ui.*` handler | **no** |
| `apps/craftkit/src/main.rs` | the render path (§76) | **no** |

The third is not an accident and not a mistake: the code argues for it at the
call site —

> The handler is host code, not ours, running with the session locked.
> Without a boundary here a panic in it would drop the connection with no
> reply at all, and an agent would be left guessing whether its request had
> arrived.

— and the *document* never heard of it. The fourth is §76's own: the render
path catches a panic in the drawing code, because the panic hook cannot (the
thread that draws holds the session's lock).

So the tree's rule was a **false sentence twice over**: it said "two" when
there were three, and it said "no more" about a rule that had already been
broken by a well-argued boundary.

The document was also wrong about a third thing, in the same file:

> `evaluate_chain` is a separate entry point that walks a whole chain; it
> validates its input up front and returns an ordinary `KernelError::Failed`
> for anything that leaves the finite world, and **it is not inside a guard**.

It is inside a guard — `crates/kernel/src/ops.rs:93`:

```rust
guard("evaluating the chain", || evaluate_chain_inner(input))
```

with a comment saying why the boundary is *there* rather than one frame out
from it.

### The fix

**The document lists the boundaries that exist**, each with its job and its
reason — and the two that are not about a foreign implementation say so:

> The last two are here because of a **lock**, and neither of them is a
> general safety net.

**And the tree is held to the document.** `cargo xtask lists` reads every
Rust file, and:

- a file with a `catch_unwind` in its production code must be a declared
  boundary;
- a declared boundary must be named in the pattern document.

A fifth boundary has to be written down before it is written.

The scan skips the tests (a test may catch a panic to assert that it was
caught ✓), the comments (a comment that says `catch_unwind` is not a call
✓), and the string literals — which is not a detail: the check's own needle
is a string, and a check that flagged itself would be a check nobody trusts
✓.

### What now holds it

| Property | Held by |
|---|---|
| Every boundary that catches a panic is declared | `every_boundary_that_catches_a_panic_is_declared` |
| An undeclared boundary is caught | `an_undeclared_boundary_is_caught` |

and one mutation in `scripts/mutation-test-gates.sh`:

- a boundary that catches a panic is not declared — and `cargo xtask lists`
  catches it.

### The lesson

**A rule with a number in it is a rule that rots.** "Two boundaries, and no
more" is a *count*, and a count about a thing that grows is stale by
construction — §71's lesson, again, in a different document. The third
boundary was not a mistake: it was better than no boundary, and it was
argued for in the code. What was missing was the *bookkeeping*, and a
document that says "no more" is the last place a reader looks for a third.

**A rule that is broken by a good change is a rule that is wrong.** §76
added a boundary because the tree needed one, and the document — which
forbids the very thing the change was for — was not re-read. The honest
order is the other way round: a change that adds a boundary has to say why
it is not a bug in the design, in the place that says what a bug in the
design looks like.

**A check that reads a document can hold the code to it.** The document was
not wrong because nobody looked hard enough; it was wrong because "two
boundaries" is a claim no one could check. Now it is a list, and the list
is checked both ways: a boundary the document does not know about, and a
boundary the document names that is not in the tree.

**This is the fourteenth prose-drift instance, and the second one this tree
caused itself** — the first was §75. Both times, the sentence that was wrong
was the one that described the thing being changed.

---

## 79. The format document's example file was never loaded by anything

`docs/design/file-format.md` is the document that describes the `.craftkit`
file, and it opens with a promise:

> Everything here was read out of the code and checked by loading a
> hand-written file, so if it disagrees with `crates/io/src/format.rs` or
> `crates/model/src/`, the code is right and this file is out of date.

and then it shows a file, and says:

> That is a complete, loadable file. `craftkit-cli eval` on it reports one
> node, 12 frames at 24 fps, and no diagnostics.

**Nothing checked it.** The document's own words — "checked by loading a
hand-written file" — describe a *process* that nobody was doing.

The claims are true today. Measured, with the real CLI on the document's own
example:

```text
left where it was
1 node(s), 12 frame(s) at 24 fps
```

and the diagnostics are empty. But three claims about a hand-written file, in
the one document that describes the format, with no gate behind them, is the
same shape as every other gap in this file: **a promise the tree does not
keep**.

And it nearly stopped being true. §73 changed what a file may contain — the
values in it are now held to the same limits the constructors keep — and the
example is a file with values in it. It survived that change by luck: its
numbers are small. A file with a value past the limit would have been
refused, and the document would have gone on saying "that is a complete,
loadable file".

### The fix

**The example is loaded, and the three claims are checked.**

```rust
#[test]
fn the_formats_example_file_is_what_the_document_says() {
    // the first json block in the document
    // load it
    // one node; twelve frames at 24 fps; no diagnostics
}
```

The test lives beside the other test about this document — the one that
checks the "what is *not* in the file" section — because it is the same kind
of thing: a statement about the document, checked against the code.

**And the document's value table now says what the reader enforces.** §73
made the reader hold a file's values to the constructors' limits, and the
table said only "a finite number" — which is what a number a hundred million
times past the limit *is*. It now says what the reader does, and says that a
file like that is refused.

### What now holds it

| Property | Held by |
|---|---|
| The format's example file is what the document says | `the_formats_example_file_is_what_the_document_says` |

and one mutation in `scripts/mutation-test-gates.sh`:

- the format's example file is not what the document says — and the CLI's own
  test catches it.

### The lesson

**A document that shows a file should load it.** The example is the only part
of the format's documentation a reader can *run*, and it was the only part
with nothing behind it. Everything else in the document is a description,
which a reader can check against the code by reading; the example is a
*claim*, and claims are what gates are for.

**"Checked by hand" is a process, and a process is not a check.** The
document says its contents were "checked by loading a hand-written file" —
which was true, once, by somebody. §53's lesson, again: a statement about
what somebody did is a description, and it rots the moment the thing changes.

**The example is the part of the documentation that breaks first.** A
paragraph about the format can be wrong in a way that only misleads; an
example that no longer loads is a *broken instruction*, and the reader who
follows it gets an error instead of a plan. §73 was one rule change away
from turning this document's example into an error message.

---

## 80. The guard against the fingerprint forgetting a field was a list of the same fields

The evaluator's cache is content-addressed: a node's state is reused only when
the node still hashes the same. That hash — `Node::fingerprint` — is
**hand-written**, because hashing the serialised form cost 52 ms where the
evaluation it saved cost 7 µs (§53). The price of a hand-written hash is
stated in the file that holds it:

> A hand-written hash has to be updated by hand every time the model grows a
> field, and a forgotten field is a **silent wrong answer**: the cache would
> serve a state computed from the old value, and nothing would say so.

A forgotten field is not a slow path or a wrong error message. It is the engine
serving a **stale answer** and calling it current.

### The measurement

One line was removed from `crates/model/src/fingerprint.rs`:

```rust
fn hash_key(h: &mut Fnv, key: &Key) {
    h.write_i128(key.time.raw());
    hash_value(h, &key.value);
    hash_easing(h, &key.ease);   // ← removed
}
```

and then the whole workspace was tested:

```text
cargo test --workspace --profile ci
workspace test exit=0
```

**Every test passed.** Not "the interesting ones" — every one of them, in every
crate, including the evaluator's own cache tests, the engine's restore test and
the model's fingerprint test.

The consequence is not hypothetical. A keyframe's easing is the *curve* between
two values, and a document with two keys and an easing between them evaluates
to a different number when the easing changes. With the line gone, the cache
does not see the change: the plan keeps the curve from before the edit, and the
engine reports it as current.

### Why nothing noticed

The file's own doc said:

> why `the_fingerprint_sees_every_part_of_the_node` checks each part in turn —
> a test that fails if a part of the node stops being part of the hash.

That sentence was false for the keyframe's parts, and the reason is in the
test:

```rust
let _ = p.keys.set(Key::new(Tick::from_raw(9), Value::Num(5.0), Easing::Hold).expect("key"));
```

A keyframe is three fields — **when**, **what** and **how**. The test changed
all three at once, so it proved that *at least one* of them was read. The hash
could forget the easing entirely and the fingerprint would still move, because
the time and the value moved as well — and the same was true of the time and
of the value.

That is one half of the gap. The other half is worse, because it is structural:
**the guard was a hand-written list of the same fields the hand-written hash
covers.** A list like that drifts with the thing it checks — the two are
written by the same reading of the same code, and a field the model grows is
added to neither, or to both by the same forgetting. The test could not have
caught the forgetting it shared.

And the claim was not in one place. The same sentence — *"a test beside the
hash checks that every part of a node is part of it"* — was in
`docs/architecture.md` and in `docs/design/file-format.md` as well. A reader
checking one of the three would have found the claim repeated, true-looking,
and not the gap under it.

### The fix

**Three things, each holding down a different half.**

1. **The model test changes one field at a time.** The keyframe's time, value
   and easing are now three separate checks, and a group's list of properties
   is checked for both its length and its contents. `the_fingerprint_sees_every_part_of_the_node`
   says "each part below is changed on its own" — now it is true.

2. **A new gate reads the tree.** `cargo xtask lists` grew
   `fingerprint_sees_every_field`: every field of every type the fingerprint
   walks is checked against the hash's **own source**. A field the model grows
   and the hash forgets is a gate failure, not a silent wrong answer, and the
   gate says which field:

   ```text
   • the fingerprint does not hash `Property`'s field `unremembered`
     (crates/model/src/fingerprint.rs)
   ```

   The types it checks are **derived**, not listed: the hash's blocks are found
   by their shape (a method called `fingerprint` or `hash_into`, a free
   function that hashes one type), and the fields come from the struct
   definitions. There is no second list to keep in step.

3. **The evaluator gets a behavioural test.** The mechanism is worth less than
   the consequence, so the consequence is asserted too:
   `a_changed_easing_is_not_served_from_the_cache` changes only the easing and
   requires the cache to miss.

### What now holds it

| Property | Held by |
|---|---|
| A keyframe's time, value and easing are each seen on their own | `craftkit-model` test |
| A group's properties are seen, and emptying it is seen | `craftkit-model` test |
| Every field the fingerprint walks is a field the hash reads | `cargo xtask lists` |
| A changed easing is not served from the cache | `craftkit-eval` test |

and two mutations in `scripts/mutation-test-gates.sh`:

- the fingerprint forgets the keyframe's easing — and the evaluator's test
  catches it;
- the fingerprint forgets a field the model has — and the `lists` gate catches
  it.

### The honest limit of the gate

The check sees that a field is **read**. It cannot see that the read reaches
the hash: a hash that read `self.id` and then hashed something else would
satisfy it. The name is the evidence, and a name is weaker than a value.

Two kinds of type are not covered by the field check, both on purpose:

* An **enum** is matched exhaustively by the hash, so a new variant is a
  compile error rather than a silent wrong answer.
* A **tuple struct** is reached through an accessor — `Keyframes` is
  `Keyframes(Vec<Key>)` — and the check reads fields, not accessors. What it
  holds is checked through `Key`.

### The lesson

**A guard that is a hand-written list of the same thing it checks drifts with
it.** The list and the hash are written by the same reading of the same code,
and they are forgotten together. A guard has to be **derived** from the thing
it guards, or it is a second copy with the same blind spot.

**A check that changes three things at once proves that one of them is
read.** This is the whole gap, and it is a shape that recurs: §53's own
fingerprint test changed a key's time, value and easing together and passed
for a hash that read only the time. "Changed on its own" is a stronger
requirement than "changed", and it is the one that matches the claim.

**The claim in the doc was the cheapest evidence available, and it was
false.** The file said a test would fail if a part of the node stopped being
part of the hash. Reading that sentence takes a second; running the mutation
takes a minute. The sentence was written by somebody reading the code, and
every other sentence in this list that was wrong was written the same way.

---

## 81. The first real format change could not be made: three guards forbade the state the machinery exists for

The house rule is one sentence long — *"a change that saves something new in a
design adds ONE file, `crates/doc/src/format/pending/<name>.rs`"* — and the
machinery for it is real: a directory, a table, a `step` function, a fold
script, a pattern document and a recipe.

§81 made the change. **The suite went red, seven times, and three of the
failures were the guards themselves firing on correct work.** Two of them said
something false about the code while they did it.

### The measurement

The recipe (`docs/recipes/change-the-file-format.md`) is the longest document
in the tree about this, so it is what was followed: one file in `pending/`, a
`FILES` entry, a `mod`, a `step`, and the reader left alone.

```text
cargo test -p craftkit-io
test format::pending::tests::every_pending_file_is_wired_into_apply ... FAILED
test format::pending::tests::every_listed_change_is_declared_and_applied ... FAILED
test format::pending::tests::applying_nothing_leaves_the_document_alone ... FAILED
test format::pending::tests::the_files_on_disk_and_the_table_agree ... FAILED
test format::tests::this_builds_own_file_round_trips ... FAILED
test store::tests::a_document_the_reader_can_read_back_is_written ... FAILED
test store::tests::a_document_round_trips_through_a_file ... FAILED
test result: FAILED. 28 passed; 7 failed; 0 ignored; 0 measured; 0 filtered out
```

And the recipe's own worked example **does not compile**:

```text
error[E0308]: mismatched types
  --> crates/io/src/format/pending/mute_flag_to_node_field.rs:20:9
   |
20 |     let Ok(enabled) = PropPath::parse("enabled") else { return };
   |     ^^^^^^^^^^^   -------------------------- this expression has type `Option<PropPath>`
   |                    expected `Option<PropPath>`, found `Result<_, _>`
```

`PropPath::parse` returns an `Option`. The recipe says `Result`. The example
is not compiled by anything — a `rust` block in a markdown file is not a
doctest — so it sat there wrong.

The release script refused the change too:

```text
$ ./scripts/fold-format-pending.sh
pending format changes (1):
  • mute_flag_to_node_field
error: mute_flag_to_node_field is not listed in crates/io/src/format/pending.rs; add it to FILES and to apply() first.
fold dry-run exit=1
```

### Why nothing noticed

**The naming was three different things in three places.**

| Where | What it said the name is |
|---|---|
| the recipe, step 1 | the file is `snake_case`, the `FILES` entry is kebab-case |
| the pattern document, step 3 | `"example-name" => example_name::step(doc)` |
| `every_pending_name_is_a_plausible_file_name` | lower-case letters, digits and `-` |

Rust module names are `snake_case`, and `mod example-name;` does not compile.
So the rule accepted a name that could not be built and refused the names that
could — and following the recipe exactly (`"mute-flag-to-node-field"` in the
table, `mute_flag_to_node_field.rs` on disk) failed
`the_files_on_disk_and_the_table_agree`, which wants them to be *the same
word*.

**The guards forbade the state instead of checking it.**

```rust
assert!(
    FILES.is_empty(),
    "there are pending changes ({}); call each one's `step` from `apply`",
    FILES.join(", ")
);
```

That is a tripwire wired to the wrong side: it fires the moment a change
exists, and its advice is advice the person had already taken. And:

```rust
assert!(
    source.contains(&format!("{name}::step(doc)")),
    "pending/{name}.rs is declared but `apply` never calls its `step`, so the \
     migration does not run and an older design loads without its rewrite"
);
```

The recipe's wiring is `Some(mute_flag_to_node_field::step)` and the pattern
document's is `name => name::step(doc)`. Neither is the string the test looks
for, and the second one is not valid Rust under the first one's naming. So
the test's message — *"`apply` never calls its `step`"* — was **false**: `apply`
did call it.

**And the reader rewrote files it had just written.** `pending::apply` ran
every step on every file, including a file this build wrote — so the file did
not read back as it was written, which is what the three round-trip failures
were. The file's own header records which changes it has had
(`Meta::pending`), and the reader **never read it**: a field in `Format`
nobody reads, in a file whose doc says *"Every field in [`Format`] is here
because some command reads it. A field nobody reads is a field that will
drift."*

### The fix

**One name, everywhere.** The file, the `FILES` entry and the `mod` are the
same word in `snake_case`. The name rule now says what it means, and what it
means is what Rust requires.

**Guards that ask questions.**

| Guard | Before | After |
|---|---|---|
| `every_pending_file_is_wired_into_apply` | the table must be **empty** | every listed change **has a step** (`step_for(name).is_some()`) |
| `every_listed_change_is_declared_and_applied` | the source contains `{name}::step(doc)` | the source contains `{name}::step` — every shape of the wiring |
| `every_pending_name_is_a_plausible_file_name` | lower-case, digits and `-` | `snake_case`, and it starts with a letter, so it is a module |
| `applying_nothing_leaves_the_document_alone` | `apply` leaves the document alone | renamed `applying_no_steps_leaves_the_document_alone`: the *empty-step* case |

**The reader gives a file only the changes it has not had.** `Meta::pending`
is read now, and `apply` takes the file's list. A file this build wrote
records every change this build has, so it is not rewritten when it is read
back.

**And the rule is testable *now*.** `File::from_text_with` and
`pending::apply_steps` take the steps as an argument, so a test can read a
file with a step of its own and see which steps the file is given. The test
`a_file_is_given_only_the_changes_it_has_not_had` is not vacuous: it drives
the reader with a synthetic step and a header that names it.

**And a number in prose had to go.** The recipe and the playbook both said
how many lines `crates/io/src/format/pending.rs` was, and the tree's
line-count gate kept those numbers true — which meant that **every format
change had to edit two documents** to keep a number right. The one-file rule
exists because two people changing the format at once must not be editing the
same file; a line count in a recipe puts them both in it, and every format
change ends in a merge conflict over a number that carries no information the
reader needs. The numbers are gone. "It is the rule" is the claim; the count
was decoration.

The gate itself stays — `prose_line_counts_match_the_files` still catches a
wrong count, and its own tests (a fabricated tree with a wrong count, and one
with a number inside a token) prove it — but the tree no longer has a count
for it to catch, and that is the point: a checked number that every change has
to update is a number in the way.

**After the fix**, the same change — snake_case throughout — gives:

```text
cargo test -p craftkit-io
test result: ok. 37 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

and the fold script's dry run exits 0. `cargo test --workspace` exits 0 with
the change in place, and so does `cargo xtask ci` — **the first time the tree
has ever carried a pending change at all**, and the first time the whole
machinery has ever run.

### What now holds it

| Property | Held by |
|---|---|
| The name is one word, and it is a Rust module | `every_pending_name_is_a_plausible_file_name` |
| The table and the directory agree | `the_files_on_disk_and_the_table_agree` |
| Every listed change has a step, and is declared | two `pending.rs` tests |
| A file is given only the changes it has not had | `a_file_is_given_only_the_changes_it_has_not_had` |
| The changes a file still needs are worked out | `a_file_that_has_had_a_change_is_not_had_it_again` |
| A file this build wrote reads back unchanged | the three round-trip tests |
| A line count in prose is the file's length | `cargo xtask lists` — with no count left to catch, on purpose |

and two mutations in `scripts/mutation-test-gates.sh`: the reader forgetting
what the file has had, and the list of changes a file still needs.

### The open half — the release, and it is the owner's

The script folds a change into `crates/io/src/format/pending/v<N>/`, empties
`FILES` and puts `apply` back to "the loop with no arms". After that **nothing
calls a folded step**: the files are not compiled (there is no `mod v2;`), not
called, and **nothing in the tree reads a `v<N>/` folder**. The recipe
describes the other half — `crates/io/src/format/v2/` and a reader that
dispatches on the version the file claims.

So a design written before a change would load **without its rewrite** after
the release, unless the reader is edited by hand to understand the older
shape — which is the one-diff edit the whole scheme exists to avoid.

Which of the two is the intent is a decision about the release, and the release
is the owner's. The script now says so out loud instead of letting the next
release produce dead code quietly, and the question is item 15 below.

### The lesson

**A tripwire wired to the wrong side fires on the work it is meant to
protect.** "The list must be empty" is not a check that the wiring is right;
it is a check that nothing has happened yet.

**A guard that quotes a spelling asks for one spelling.** The test wanted
`{name}::step(doc)`, the recipe wrote `Some(name::step)`, and the pattern
document wrote a third thing — and the test's message claimed the step was
never called, when it was. A guard should look for what all the right answers
have in common.

**A name is one thing, and three documents were free to disagree about it.**
The file, the table entry and the module were allowed to be different words,
and the release script compared one of them with another.

**The field that records the state is not read by the code that needs it.**
`Meta::pending` was written on every save and read on no save, in a file whose
own doc says a field nobody reads is a field that will drift.

**A guard that cannot be exercised is not a guard.** The whole machinery had
never run: the list had been empty since version 1 (§45), the guards were
vacuous, and the first real change was the first execution — of code whose
rules three documents described differently.

---

## 82. The control channel's method list was written out by hand in three places, and one of them was wrong

The control channel is how an agent drives the application. Its **methods**
are the wire contract: `engine.execute`, `engine.describe`, `session.state`,
`diagnostics.list`, `diagnostics.crash_reports`, `ping`, `shutdown` and `ui`
(`ui.click`, `ui.screenshot`, `ui.render`).

Those eight names are written down in the code, in `Method::as_str`. They were
also written down **by hand**, in three documents.

### The measurement

| Where | What it says |
|---|---|
| `crates/control/src/protocol.rs` | the eight, in `Method::as_str` |
| `docs/recipes/add-a-control-channel-method.md` | a table of all eight ✓ |
| `skills/craftkit-checking/SKILL.md` | "Methods, **exactly**": all eight ✓ |
| `AGENTS.md` | **five of the eight** ✗ |

`AGENTS.md` said:

> Its methods are `engine.execute`, `engine.describe`, `session.state`,
> `diagnostics.list` and `ui` (`ui.click`, `ui.screenshot`, `ui.render`).

`diagnostics.crash_reports`, `ping` and `shutdown` were missing. The sentence
has no "and a few more" in it — it reads as the whole list, and it is the
document every agent reads first.

The skill's list was complete, and it cited its source:

> Methods, exactly (`protocol.rs:59-67`): …

Lines 59 to 67 of `protocol.rs` are the tail of `as_str` and the head of
`parse` — a fragment, not the list. The list runs from 53 to 60.

### Why nothing noticed

**A list in prose is a second copy of the code.** Three copies of the method
names, written at three times, by three hands — and nothing compared any of
them. §77 found the same shape in a *count* ("21 commands"), §80 found it in a
*list of fields*, and this is the same thing again in a list of *names*: a
hand-written copy of something the code already knows.

**The one citation that could have been checked was not.** §39's check covers
citations that name a **test** — "`the_fingerprint_is_stable` asserts it" —
because a test name is a name that can be looked up. A citation that names
*lines* (`protocol.rs:59-67`) names a place, and nothing checked places: the
lines it pointed at were a fragment of the thing it claimed to point at.

**The one list that could have been compared was not.** The recipe's table is
a table: eight rows, one name each. A table is comparable. Nothing compared
it.

### The fix

A new `lists` check, `control_methods_are_listed`, with two rules:

* **The recipe's table is exactly the code's methods** — in both directions.
  A method the code has and the table does not is a gap; a method the table
  has and the code does not is a name an agent would send and get an error
  for.
* **A paragraph that names more than half of the methods names them all.**
  The threshold is what keeps this from firing on a paragraph that mentions a
  method in passing: `docs/agent-playbook.md` names three in its examples and
  is not talking about the set.

The names come from the code — `Method::as_str`'s arms — so the list is
derived, not copied, and there is no second list of methods to keep in step.

And the prose is corrected: `AGENTS.md` names all eight, and the skill's
citation now names the **symbol** (`Method::all()` in
`crates/control/src/protocol.rs`) rather than a line range — and a symbol
citation *is* checked, by the check that a symbol claimed to be in a file is
in it.

The check finds the old `AGENTS.md` on the real tree:

```text
• AGENTS.md: line 200 names 5 of the 8 methods and not `diagnostics.crash_reports`
• AGENTS.md: line 200 names 5 of the 8 methods and not `ping`
• AGENTS.md: line 200 names 5 of the 8 methods and not `shutdown`
```

### What now holds it

| Property | Held by |
|---|---|
| The recipe's table is the code's method list, both ways | `cargo xtask lists` |
| A paragraph that names most of the methods names them all | `cargo xtask lists` |
| The tree's method list is right today | `the_control_methods_are_listed_in_the_prose` |

and a mutation in `scripts/mutation-test-gates.sh`: a method taken out of
`AGENTS.md`'s list.

### The lesson

**A list in prose is a copy of the code, and a copy rots.** Three copies of
eight names, one of them wrong, and no comparison anywhere. The fix is not
"write the list more carefully" — it is to make the copy *derived*, or to
check it.

**A citation that names a place is a promise nobody keeps.** `protocol.rs:
59-67` was the shape §39 does not check, and it was wrong in the way a
placeholder citation always is: it points at *something* near the thing.

**A threshold is what lets a check about prose be a check.** "Every paragraph
must name every method" would fire on every document that mentions one. "A
paragraph that names more than half of them is talking about the set" fires
on exactly the sentences that claim to be the list.

---

## 83. The registry advertised seven keyboard shortcuts, and the tree bound none

`CommandSpec` has a `shortcut` field, and the house rules list it as part of
the command's shape: id, label, area, icon, **shortcut**, parameters, doc,
enabled. Seven commands declare a chord.

### The measurement

```text
Ctrl+N   Del   F2   M   Ctrl+D   Ctrl+Z   Ctrl+Shift+Z
```

All seven are unique ✓. And **nothing binds them**:

```text
$ grep -rn "key_pressed\|consume_key\|Keyboard\|hotkey" crates/ui/src crates/engine/src apps/craftkit/src
(no matches)
```

The UI crate is 2496 lines and has no key handling at all — no `Key::` check,
no `key_pressed`, no shortcut parser. The only `egui::Key` in the tree is a
synthetic event in a test. The application is the same: pressing `Ctrl+Z` in
the window does nothing.

What the tree *says* about those chords:

| Where | What it says | The truth |
|---|---|---|
| `CommandSpec::shortcut`'s doc | "The key chord, in the form **the UI's shortcut parser** understands." | there is no shortcut parser ✗ |
| `skills/craftkit-editing/SKILL.md` | "key chord the UI's parser understands" | the same claim ✗ |
| the MCP tool description | "**Keyboard: Ctrl+Z**" | the app binds no keys ✗ |
| `docs/architecture.md` | "The UI toolbar, **a keyboard shortcut**, a CLI script, the control channel and the MCP tool all land on step 3" | one of those five front ends does not exist ✗ |

The MCP line is the one with teeth. A tool description is what an agent reads
to decide what to tell a person, and "Keyboard: Ctrl+Z" reads as *send them
here*. The person presses `Ctrl+Z` and nothing happens, and the tree has
promised otherwise.

### Why nothing noticed

**The field had exactly one reader, and the reader was the advertisement.**
`spec.shortcut` is read in one place — the MCP description — and the thing
that read it never checked whether the thing it read was true. The
advertisement was exercised; the *claim* was not.

**A specification cannot fail a test.** There is no implementation to be
wrong: the chords are notes to self, and a note to self is not a fact about
the tree. So the tree could say "the UI's shortcut parser" for as long as it
liked — the parser was not there, and the sentence about it was the only
place the tree claimed otherwise.

### The fix

**The documentation says what is true.** The chord is the chord the command is
*advertised* with; the tree binds no keys; the MCP description repeats it and
says so.

**The advertisement stops claiming a binding.**

```text
Intended keyboard: Ctrl+Z (this build binds no keys)
```

**And the specification is checked.** A new test in the engine:
`the_advertised_shortcuts_are_unique_and_well_formed` — no two commands on
one chord, and every chord is `[Ctrl+][Shift+]<key>`, with `Ctrl` before
`Shift`. A specification with two commands on one chord, or a chord nobody
could parse, is a specification nobody could implement.

A second test, in the MCP crate: a tool description that mentions a chord
also says the build binds no keys.

### What now holds it

| Property | Held by |
|---|---|
| No two commands advertise the same chord | `the_advertised_shortcuts_are_unique_and_well_formed` |
| Every chord is `[Ctrl+][Shift+]<key>` | the same |
| A tool description does not claim a binding | `a_tool_description_does_not_claim_a_key_the_build_does_not_bind` |

and a mutation in `scripts/mutation-test-gates.sh`: two commands advertising
one chord.

### The open half — the binding, and it is the owner's

The chords are a specification, and a specification is not a feature. Binding
them is a decision with questions in it: which commands get one, what a chord
does about a command that needs parameters (`Ctrl+N` is `node.add`, which
needs a name and a kind — does it open a dialog, like a toolbar click?), and
what happens when two commands want the same chord. The tree's rule is that
only a toolbar click starts an interactive tool, and a key press is a
toolbar click in another shape — so the rule has an answer, but the *product*
does not yet.

So the decision is recorded as item 16 below: **bind the chords, or drop the
field.** What is not on the table is the middle state §83 found — a chord in
the registry, advertised to agents as a keyboard shortcut, bound to nothing.

### The lesson

**A specification can lie in the tree's voice.** The doc said "the UI's
shortcut parser", and the tree has no shortcut parser. A specification has no
implementation to fail a test, so the sentences *about* it are the only place
the claim lives — and they were not claims anybody checked.

**An advertisement is a promise.** "Keyboard: Ctrl+Z" in a tool description is
the tree telling an agent to send a person to a key. The tree has to be able
to answer for every word it publishes, and the honest word here was "intended".

**A field with one reader needs the reader to check something.** The chord
was read to be repeated, and repeating a claim is not the same as checking it.

---

## 84. The history's memory budget was enforced on push and not on undo

§52 gave the undo and redo stacks a **memory** budget, because a count of
entries is not a bound on memory. §68 found that the budget counted memory
that had been freed. §84 found that the budget was not checked on half the
paths that hold memory.

### The measurement

`push` checks:

```rust
while self.bytes + bytes > MAX_HISTORY_BYTES {
    if self.undo.is_empty() { break; }
    self.drop_oldest_undo();
}
```

`pop_undo` — the path a person takes when they undo — did not:

```rust
pub fn pop_undo(&mut self, current: Document) -> Option<Document> {
    let entry = self.undo.pop()?;
    self.bytes -= entry.bytes;
    let bytes = current.estimated_bytes();
    self.bytes += bytes;
    self.redo.push(Entry { label: entry.label.clone(), doc: current, bytes });
    Some(entry.doc)
}
```

An undo moves the **current** document into the redo stack. The current
document is not the snapshot it replaces: it is whatever the person has been
editing, and it can be **larger**. So the two stacks could hold more than the
budget allows, and the budget's own report said so:

```text
the history held 67936429 bytes, and the budget is 67108864
```

67 936 429 bytes against a 67 108 864-byte budget — an overshoot of 827 565
bytes in the measurement, and up to the size of the whole current document
(32 MiB, the model's own limit) in the worst case: a history holding 64 MiB of
snapshots, undone into a 32 MiB document, holds 96 MiB against a 64 MiB
budget.

### Why nothing noticed

**The budget had a test, and the test exercised the path that checked it.**
`the_history_keeps_its_memory_within_a_budget` pushes and pushes ✓ — it never
undoes with a document bigger than the one it replaced ✓.

**Both stacks count together, so the arithmetic looked right.** `self.bytes`
is correct at every line — the number reported *is* what the history holds.
What was missing was the *check*, not the count. §68 was the same shape: the
count was right and the check was not.

### The fix

The undo and redo paths trim now, through one helper:

```rust
fn trim_to_budget(&mut self) {
    while self.bytes > MAX_HISTORY_BYTES {
        if !self.undo.is_empty() {
            self.drop_oldest_undo();
        } else if !self.redo.is_empty() {
            self.drop_oldest_redo();
        } else {
            break;
        }
    }
}
```

The undo steps go first, because the redo entry that was just pushed is the
newest state there is and the oldest undo step is the one a person is least
likely to want — which is the same rule `push` already followed ("Drop the
oldest, not the newest: recent steps are the ones a person is likely to
want").

### What now holds it

| Property | Held by |
|---|---|
| The history stays inside its budget, whoever moves a document | `undoing_does_not_take_the_history_over_its_budget` |
| The budget counts what the stacks hold | `the_budget_counts_the_memory_the_stacks_hold` |

and a mutation in `scripts/mutation-test-gates.sh`: the undo path not
trimming.

### The lesson

**A budget has to be enforced on every path that holds memory.** The budget
was checked where the memory arrives and not where the memory *moves*, and a
move can enlarge what is held.

**A test that exercises one path proves that path.** The budget test pushed
138 documents and never undid; the undo path was untested against the budget,
and a budget with one checked path is a budget with a hole in it.

---

## 85. A property had several names, and the model could not agree on which one it had

§83 found that the tree advertised shortcuts it did not bind. §85 found
something worse in the one place every command goes through: **the model had
several ways to name a property, and they disagreed with each other.** A
property could be read by a path that could not write it, listed under a path
that pointed at a different property, and created under a path that named
nothing at all.

The command that showed it is one line:

```text
prop.set { node: 1, path: "@5", value: 42 }   →   Ok("`@5` = 42")
```

where property 5 was the nested property `a/b`. It reported success, and it
did something else.

### The measurement

**1. The read side and the write side resolved an id differently.** The model's
two resolvers are one screen apart:

```rust
// find — the read side
PathSeg::Uid(_) => None,          // …except in the last position, where
                                  // find_uid_in searches the whole subtree
// find_mut — the write side
let idx = index_of_seg(&group.nodes, last)?;
// …and index_of_seg's uid arm:
//     PathSeg::Uid(id) => nodes.iter().position(|n| matches!(n, PropNode::Prop(p) if p.id == *id)),
//     …the direct children only
```

Measured on a document where property 5 sat one group down:

```text
find("@5")     = Some((5, "b", "1"))
find_mut("@5") = None
```

So the property could be **read** by `@5` and not **written** by it. The engine
writes, so `prop.set @5` fell through to "there is no such property" and
**invented** one. Before the fix:

```text
prop.set { node: 1, path: "@5", value: 42 }  →  Ok("`@5` = 42")
flat after: [("enabled", 1, "true"), ("amount", 2, "1"), ("tint", 3, "#8caad2"),
             ("label", 4, ""), ("a/b", 5, "1"), ("5", 6, "42")]
```

The nested property kept its old value — the path named the wrong thing — and
the root gained a second property labelled `5`. The command reported success.
After the fix the same command sets the property the path names and creates
nothing:

```text
prop.set { node: 1, path: "@5", value: 42 }  →  Ok("`@5` = 42")
flat after: [("enabled", 1, "true"), ("amount", 2, "1"), ("tint", 3, "#8caad2"),
             ("label", 4, ""), ("a/b", 5, "42")]
```

**2. The listing and the parser disagreed about how deep a property tree may
be.** The listing stopped at the path limit; the validator did not. The
boundary, measured group by group:

| groups | `flatten()` gives | the deepest path | the parser accepts it | `validate()` |
|---|---|---|---|---|
| 15 | 1 property | 16 segments | yes | ok |
| 16 | 1 property | **17 segments** | **no** | ok |
| 17 | **0 properties** | — | — | **ok** |

Three things in one table. A tree of sixteen groups published a path the
parser refuses. A tree of seventeen groups published **nothing at all** — and
`validate` called it good. That tree is a document that holds a property the
file keeps, the UI does not list, and the evaluator never reads.

**3. The listing counted names exactly; the resolver counted them loosely.**
Two sibling properties called `Amount` and `amount`:

```text
validate()   = Ok(())
flat         = "Amount" (id 1), "amount" (id 2)
find("amount") = Some((1, "Amount"))
```

The second property's published path resolved to the **first** one, so the
second had no name of its own.

**4. The engine had a second path parser.** The engine's own comment claimed
otherwise, twenty-seven lines above the helper it sat on — and the same file
had a parser of its own:

```rust
/// Find a property by path, using the model's own grammar rather than a
/// hand-rolled split.
///
/// One addressing scheme, one implementation. A second parser here would be a
/// second thing to keep in step with `craftkit-model`.
fn find_mut<'a>(group: &'a mut PropGroup, path: &str) -> Option<&'a mut Property> { … }
```

and further down the file:

```rust
fn insert_at(group: &mut PropGroup, path: &str, prop: Property, id_of: &'static str) -> … {
    let segs: Vec<&str> = path.split('/').map(str::trim).filter(|s| !s.is_empty()).collect();
    …
    current.push_group(PropGroup::new(*seg, *seg));   // every segment is a literal id
```

So `prop.set` read every segment as a literal name:

```text
prop.set { node: 1, path: "effects/#2/amount", value: 7 }  →  Ok
   … made a group whose id was literally "#2"
find("effects/#2/amount") = None          ← the property it just made cannot be read back

prop.set { node: 1, path: "@99", value: 1 }  →  Ok
   … made a property labelled "99" (id 6)

prop.set { node: 1, path: "g0/g1/…/g19/leaf", value: 1 }  →  Ok
   … made twenty groups and a property
   flatten()               = 4 properties      ← the new one is not among them
   PropPath::parse(the path) = None             ← and no path can name it
   the file mentions "leaf" = true              ← and the file keeps it
```

Setting that last path a second time made a **second** `leaf`: the lookup
still failed, so the insertion ran again. The file grew by a property that
nothing could read, and the command said `Ok` both times.

### Why nothing noticed

**The model's own test asked a weaker question.** There was one, and it was
about exactly this:

```rust
fn flatten_produces_paths_that_resolve_back() {
    for (path, _) in &flat {
        assert!(g.find(path).is_some(), "flattened path {path} does not resolve");
    }
}
```

*Some* property, not *the* property. A path that resolves to the wrong one
passes. The name of the test says what it meant to check, and it did not check
that.

**Three pieces of code had three ideas of what a path is, and no test asked
them to agree.** The listing (`flatten`), the parser (`PropPath::parse`) and
the validator (`PropGroup::validate`) each had their own idea, and the only
relationship between them was the *number* `MAX_PATH_SEGMENTS` — used by the
first two, ignored by the third, which used `MAX_DEPTH` instead. Two limits
for two different trees is fine; two limits for one tree is a hole.

**The engine's second parser was invisible because it was inside a command.**
`prop.set` was tested with the paths a person types — `amount`, `custom/thing`.
Nothing tested a path the grammar cannot describe, because nothing in the tree
said such a path was possible.

**A comment can be a promise nobody keeps.** The claim "one addressing scheme,
one implementation" sat directly above a helper that used the model's
grammar — and directly above a second parser, twenty-seven lines further
down, that did not. §82's lesson (cite the symbol, not the line) applies to
code as well: the sentence was true of the function it sat on and false of the
file it sat in.

### The fix

**One name-matching rule, in one place.** A group's id and a property's label
both match case-insensitively, and every part of the model asks that one
function:

```rust
fn node_matches(node: &PropNode, name: &str) -> bool {
    match node {
        PropNode::Group(g) => g.id.eq_ignore_ascii_case(name),
        PropNode::Prop(p) => p.matches(name),
    }
}
```

The listing's occurrence counter now counts the way the resolver counts, so a
published path points at the property it names.

**The tree is as deep as a path is long.** The property tree is bounded by
`MAX_PATH_SEGMENTS`, and a tree deeper than that is **refused** rather than
truncated:

```rust
if depth >= limits::MAX_PATH_SEGMENTS {
    return Err(CoreError::LimitReached { limit: "property depth" });
}
```

`MAX_DEPTH` is now what it always should have been — the depth of the **node
graph**, which is the evaluator's rule.

**What this costs.** A property tree deeper than fifteen groups used to be
legal, and a file holding one is now **refused** by the reader rather than
loaded with its deepest properties invisible. That is the direction §50 set
("the writer must not write what the reader refuses"), and the writer agrees:
the model refuses to build such a tree. No file this tree has ever written has
one — the engine could not make one without a path of more than sixteen
segments, and that path was refused — but a file made by hand, or by an older
build that allowed it, will not open. The honest trade is a refused file over
a file that holds a property nothing can reach.

**The write side resolves an id the way the read side does.** `find_mut` looks
for a uid anywhere below the group it is addressing, so a property that can be
read by `@5` can be written by `@5`.

**The model creates properties.** The engine's hand-rolled parser is gone; the
model owns the grammar, so the model owns the creation:

```rust
pub fn insert(&mut self, path: &PropPath, id: u64, value: Value) -> Result<(), CoreError>
```

A path that names an **id** or a **position** names something that already
exists, so it can change a property but not create one, and it is refused:

```text
prop.set { node: 1, path: "@99", value: 7 }
  → `path` `@99` names a property by its id, and there is no property with that id to create
prop.set { node: 1, path: "effects/#2/amount", value: 7 }
  → `path` `#2` names the 2nd child of this group, and a new group has no position yet
prop.set { node: 1, path: "g0/…/g15/leaf", value: 1 }
  → `path` `…` is not a path this build accepts: a path has at most 16 segments, and every
    segment must name a property or a group
```

### What now holds it

| Property | Held by |
|---|---|
| Every property the tree holds is named by a path the parser accepts | `every_property_the_tree_holds_is_named_by_a_path_the_parser_accepts` |
| A tree deeper than a path is refused, not truncated | `a_tree_deeper_than_a_path_is_refused_rather_than_truncated` |
| A path names the property the listing published it for | `a_path_names_the_property_the_listing_published_it_for` |
| The read side and the write side resolve an id the same way | `the_read_side_and_the_write_side_resolve_a_uid_the_same_way` |
| A path that names an id or a position cannot create anything | `a_path_that_names_an_id_or_a_position_cannot_create_anything`, `an_id_that_names_no_property_cannot_create_anything`, `a_path_that_names_a_position_cannot_create_anything` |
| A property set by its id is the property the id names | `a_property_set_by_its_id_is_the_property_the_id_names` |
| A path too long to name is refused | `a_path_too_long_to_name_is_refused` |
| A created property is readable by the path that created it | `a_created_property_is_readable_by_the_path_that_created_it` (model and engine) |

and six mutations in `scripts/mutation-test-gates.sh`: the property tree
deeper than a path, the listing counting names exactly, the write side
resolving an id differently, an id naming a property that can be created, a
path longer than the tree is deep, and a group created under a name the path
does not name.

### The lesson

**One thing, one implementation — and check the claim is true of the file, not
of the function.** "One addressing scheme, one implementation" was true of
`find_mut` and false of `commands.rs`.

**A path is a name, and the code that resolves a name and the code that
publishes one have to be the same code.** Here they were three: the listing,
the parser and the validator, plus a fourth in the engine. Every one of them
had its own idea, and no test asked them to agree.

**"Is it there?" is a weaker question than "is it the right one?"** The model's
test asked the first and the bug was the second.

**A command that reports success has done what it said.** `prop.set @5`
reported "`@5` = 42" and set a different property. The rule the tree already
has — a refused command leaves nothing half-applied — did not cover it,
because the command did not fail: it succeeded at the wrong thing.

**Prose drift, instance nineteen.** The agent playbook said "the tests start at
line 960" about `crates/engine/src/commands.rs`, and §85's edits moved them.
The line is now the place — "its tests are at the end of the file" — and the
gate that checks the old shape still catches a document that names a line.

---

## 86. The live-window check read the token the last run left behind

§85's verification ran `scripts/live-window-check.sh` and it failed, with a
complaint about the control channel:

```text
live-window: window 0x5000005 (pid 57692)
Traceback (most recent call last):
  File "<stdin>", line 6, in <module>
IndexError: list index out of range
live-window: FAIL — the commands over the control channel did not all succeed
```

The driver had sent one request and received **nothing at all**: the server
closed the connection without a reply. The check had nothing to do with the
window, and the failure said nothing about the window.

### The measurement

The token file **outlives the process that wrote it**. The control server
replaces it on the next launch and never removes it — the tree says so in
`crates/control/src/token.rs`:

> the token file outlives the process that wrote it

and the script waited for the file to *exist*:

```bash
TOKEN_FILE="$HOME/.local/share/craftkit/control/$PORT.token"
for _ in $(seq 1 60); do
    [ -s "$TOKEN_FILE" ] && break
    …
done
TOKEN=$(cat "$TOKEN_FILE")
```

The file from the previous run is still there, non-empty, **before this app
has written anything** — so the wait was satisfied at once and the script
sent the *previous* run's token. The server refused it and closed the
connection, and the driver read an empty reply.

The port is fixed (`CRAFTKIT_LIVE_PORT`, default 47711), so the file is
almost always there from the last run: the check fails on the first run after
another one and passes on the ones after that, when the app has already
replaced the file. **A check that fails for a reason that has nothing to do
with what it checks is worse than no check**: it teaches people to re-run it
and to ignore the failures, and it is exactly the §1 failure in the other
direction.

Reproduced on demand, with the stale file planted by hand:

```bash
printf '%s\n' "$(head -c 32 /dev/urandom | od -An -tx1 | tr -d ' \n')" \
    > ~/.local/share/craftkit/control/47711.token
./scripts/live-window-check.sh
```

### The fix

The wait is for the file to hold a token that is **not** the one that was
already there:

```bash
wait_for_token() { # path old-token pid [tries]
    local path="$1" old="$2" pid="$3" tries="${4:-60}"
    local now
    for _ in $(seq 1 "$tries"); do
        if [ -s "$path" ]; then
            now=$(cat "$path")
            if [ "$now" != "$old" ]; then
                printf '%s' "$now"
                return 0
            fi
        fi
        kill -0 "$pid" 2>/dev/null || return 2
        sleep 0.5
    done
    return 1
}
```

and the script's self-test now has two cases for it, because a fix to a check
that is not itself checked is a wish:

| Case | Says |
|---|---|
| `a token left over from the last run is not this app's` | the wait takes the *new* token, not the one that was there |
| `a token that never changes is not waited for` | a file that never changes is not a token, and the wait gives up |

### What now holds it

| Property | Held by |
|---|---|
| The check reads this app's token, not the last run's | two `live-window-check.sh --self-test` cases |
| The check fails loudly when the app never writes a token | `wait_for_token`'s status 1 and 2 |

and a mutation in `scripts/mutation-test-gates.sh`: the wait taking any token
in the file.

### The lesson

**A file that outlives its writer is not evidence of the writer.** The token
file is the app's, but between one run and the next it holds a token that
belongs to a process that is gone. "The file exists" and "the file is *this
app's*" are different claims, and the script only asked the first one.

**A check that fails for the wrong reason is a check that gets ignored.** The
first run of the live-window check after §85 failed, and the obvious reading
was "my change broke the control channel". It had not: three later runs
passed with the same numbers, and the cause turned out to be a file left in
`~/.local/share`. Finding that out took a measurement; without one it would
have been a coin flip.

**The self-test is the only part of a shell script that can be checked
without a display**, so a rule the script enforces belongs there. §56 put the
window-picking rule in the self-test; §86 puts the token rule next to it.

---

## 87. The end-to-end MCP test threw the server's own words away

§86 was a check that failed for the wrong reason. This is the other half of
it: a check that cannot say **why** it failed.

### The measurement

One run of `cargo xtask ci` failed in the tests phase:

```text
error: test failed, to rerun pass `-p craftkit-cli --test mcp_stdio`
```

and the test's own message was:

```text
the server keeps its end open: Os { code: 32, kind: BrokenPipe, message: "Broken pipe" }
```

Which says the pipe broke. It does not say **that the server had already
died**, what it printed before it died, or how it exited — because the test
threw all of that away:

```rust
let mut child = Command::new(BIN)
    .arg("mcp")
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::null())          // ← the server's words, into nothing
    .spawn()
```

and nothing ever asked the child for its exit status. So the failure said:
*something* broke, and the person reading it had to guess.

The failure did not reproduce: **15** runs of that test on its own and **3**
runs of the whole workspace were green, and the machine was under memory
pressure at the time (17.5 GB of 32 GB used, right after a forty-minute
build-heavy run), which is the likeliest reason — a spawn that failed, or a
child that was killed. **That is a guess, and it is recorded as one.** What
is not a guess is that the next occurrence would have said exactly as little.

### The fix

The server's stderr goes to a file, and every failure path asks the child what
became of it:

```rust
/// What the server said, and what became of it.
fn complaint(&mut self) -> String {
    let status = self.child.try_wait().ok().flatten();
    let said = std::fs::read_to_string(&self.log).unwrap_or_default();
    let said = said.trim();
    if said.is_empty() {
        format!("the server said nothing; its status is {status:?}")
    } else {
        format!("the server said: {said}; its status is {status:?}")
    }
}
```

A **file**, not a pipe: a pipe nobody reads fills up, and then the server
blocks writing to it — which would look exactly like the hang this file
exists to catch.

The same message now comes out of a write that fails, because that is the
first thing to fail when the server is gone:

```text
the server stopped listening (Broken pipe (os error 32)) — the server said nothing;
its status is Some(ExitStatus(unix_wait_status(9)))
```

which says what the old one did not: the server was **killed**, and it said
nothing on the way out.

### What now holds it

| Property | Held by |
|---|---|
| The end-to-end MCP test says what the server said and how it exited | `complaint` in `apps/craftkit-cli/tests/mcp_stdio.rs`, used by `send` and `expect_reply` |
| The server's stderr is captured rather than discarded | `Stdio::from(err)` in `Server::start` |

There is no mutation for this one, and the matrix says so: the property only
shows itself **when the test fails**, so a mutation that removed the capture
would leave every test green. A check that cannot fail is §1's subject; a
check whose *diagnostics* cannot fail is the same thing one level down, and
the honest entry is *nothing* rather than a mutation that would measure
nothing.

### The lesson

**The tree's own rule about error messages applies to the checks too.** The
engine's error type says it: "an error message is not a shrug". A test that
fails with "a reply to the call" is a shrug.

**A check that cannot say why it failed gets re-run and believed less.** The
failure here looked like a product defect — the MCP server had died — and
the honest answer was "I do not know, and the test threw the evidence away".

**Record the guess as a guess.** The likeliest cause is the machine, not the
tree; saying so is not the same as saying so is the cause.

---

## 88. A property's name could be something the path grammar cannot write

§85 fixed the model's disagreement about how a property is named. It did not
close the door through which a **name** enters the tree: a file can carry a
label or a group id that the grammar reads as something else, and the tree
accepted it.

### The measurement

A file, written by hand, with one property whose label is `a/b`:

```text
the file loaded
validate()   = Ok(())
the listing published the path "a/b"
PropPath::parse("a/b") = Some   ← it parses — as two segments
… and it resolves to: None      ← and to nothing at all
```

So the listing published a path that named **no property**, and the property
it was published for was unreachable by any command. The whole family, each
measured the same way:

| Label | the file loads | the listing publishes | the parser reads it as | resolves to |
|---|---|---|---|---|
| `a/b` | yes | `a/b` | two segments | **nothing** |
| `@5` | yes | `@5` | an id | **nothing** |
| `#2` | yes | `#2` | a position | **nothing** |
| `amount#2` | yes | `amount#2` | the *second* `amount` | **the wrong property** |
| `C#` | yes | `C#` | — | **the parser refuses it** |
| `a@b` | yes | `a@b` | the name `a@b` | the property ✓ |
| `ok` | yes | `ok` | the name `ok` | the property ✓ |

And the command's answer, with the file's property in the tree:

```text
--- before: [("a/b", 1, "2")]
=== prop.set a/b -> Ok("`a/b` = 99")
--- after:  [("a/b", 1, "2"), ("a/b", 2, "99")]
--- validate: Ok(())
```

**The command reported success and did something else.** The property the path
named still holds `2`; a *second* property was created, and it publishes the
same path `a/b`. The node now has two properties with one name, the UI shows
two rows with the same path, and the plan has two entries with the same path.
Editing the row on screen would create another one each time.

### Why nothing noticed

**§85's invariant was checked on trees the model built.** The new tests
covered the depth, the id and the case, and every one of them built its tree
from names the grammar can write. A name that the grammar cannot write was
never in a test — because nothing in the tree said such a name was possible.

**The reader validates the document, and the document said this was fine.**
`PropGroup::check` refused an empty label, an oversized one and a duplicate
group id, and it had no rule about a name the *path grammar* cannot write.
The validator and the grammar were two pieces of code about the same thing,
and only one of them knew about the grammar.

**A file is deserialized straight into the model.** Nothing constructs a
property when it is read — the label arrives as text and is accepted — so a
file is the one place where an arbitrary name can appear. §73 made the same
argument for numbers ("a file is not built by a constructor: it is
deserialized straight into the model"); the rule was not carried across to
names.

### The fix

**A name must be writable, and the check asks the grammar.**

```rust
pub fn is_written_as_it_is(name: &str) -> bool {
    matches!(parse_seg(name), Some(PathSeg::Match(parsed, 1)) if parsed == name)
}
```

The name is parsed as a segment and has to come back as the same plain name.
**A round trip, not a list of characters**: a list of forbidden characters
would be a second copy of the grammar, and it would rot the moment the
grammar grows a form — §80's lesson, learned twice.

A name that cannot be written plainly can still be written with an explicit
occurrence — the grammar's own way of saying "this name, the first one" — so
`a#b` and `C#` are published as `a#b#1` and `C##1`, and those read back as
exactly those names. Only three shapes are refused, and the reason is asked of
the grammar rather than kept beside it:

| Name | Why it is refused |
|---|---|
| `a/b` | `/` separates the segments of a path |
| `@5` | `@` marks an id, and an id is not a name |
| `#2` | `#` marks a position, and a position is not a name |

The tree refuses such a name in [`PropGroup::validate`], so **the reader
refuses the file** — with the name and the reason in the message:

```text
the document in this file is not valid: invalid: node n1: the property label `a/b`
cannot be written as a path: `/` separates the segments of a path
```

and the engine refuses to work with such a document too, so a command cannot
create the second property that used to appear:

```text
prop.set { node: 1, path: "a/b", value: 99 }
  → the command left the document in a state the file format refuses: invalid: node n1:
    the property label `a/b` cannot be written as a path: `/` separates the segments
    of a path; your document is unchanged
```

### What now holds it

| Property | Held by |
|---|---|
| A name the grammar cannot write is refused | `a_name_the_grammar_cannot_write_is_refused`, `a_label_the_grammar_cannot_write_is_refused`, `a_group_id_the_grammar_cannot_write_is_refused` |
| The reason a name is refused comes from the grammar | `the_reason_a_name_is_refused_comes_from_the_grammar` |
| A name is published as the shortest spelling that parses back | `a_name_is_published_as_the_shortest_spelling_that_parses_back`, `display_round_trips_through_parse_for_names_the_grammar_cannot_write_plainly` |
| Every property is named by a path that names **it** | `every_property_is_named_by_a_path_that_names_it` |
| A file carrying such a name is refused | `a_file_carrying_a_name_the_grammar_cannot_write_is_refused` |
| A path the grammar cannot read is refused | `a_path_the_grammar_cannot_read_is_refused` |

and three mutations in `scripts/mutation-test-gates.sh`: a property label the
grammar cannot write being accepted anyway, a group id the grammar cannot
write being accepted anyway, and a name being published as a spelling that
does not parse back.

### The lesson

**A fix that closes one door and leaves the next one open is half a fix.**
§85's invariant was "every property is named by exactly one path" — and it was
true of every tree the model built, and false of every tree a file could
carry. The hole was not in the rule; it was in the *input*.

**The rule and the thing that checks it must know about each other.** The
validator refused duplicate group ids because the *resolver* would be
ambiguous; it did not refuse a name the *grammar* cannot write because
nobody connected those two facts. A validator that checks one rule and not
the neighbouring one is a validator with a hole the size of the gap between
the rules.

**Ask the grammar, do not keep a copy of it.** "A name may not contain `/`,
`@` or `#`" is a second copy of the grammar, and it would be wrong the moment
the grammar grew a form. "The name must come back from the parser as itself"
cannot be wrong that way.

**What a command reports is what it did.** `prop.set { path: "a/b" }` said
"`a/b` = 99" and created a different property. §85 found the same shape in
the id form; §88 found it in the label form. Two of the same bug is a
pattern, and the pattern is worth more than either instance.

---

## 89. The frame rate's ceiling was written five times, and the two that decided anything disagreed

§88 found a name the grammar could not write. §89 found a **number** written
five times, with two different values — and the two copies that decided
anything were not the two that decided everything.

### The measurement

The ceiling was in five places:

| Where | What it said |
|---|---|
| `crates/core/src/tick.rs` — `FrameRate::new` | refuses 0 and **anything above 1000** |
| `crates/engine/src/commands.rs` — `doc.range` | refuses anything outside **1..=240** |
| the same command's parameter doc | "1..=240" |
| `crates/ui/src/shell.rs` — a comment | "validated to 1..=240 by `doc.range`" |
| `docs/design/file-format.md`, `docs/patterns/exact-time-not-float-seconds.md` | "refuses 0 and > 1000" |

So the **reader** accepted a thousand frames per second and the **command**
accepted two hundred and forty. Measured, rate by rate:

| fps | the reader | the command |
|---|---|---|
| 240 | loads | accepted |
| **241** | **loads** | **refused** — "241 is not between 1 and 240" |
| **500** | **loads** | **refused** |
| **1000** | **loads** | **refused** |
| 1001 | refused | refused |

And the sharpest symptom, on a document that carries 500 fps:

```text
doc.range { fps: 500 }  →  Err(`doc.range` rejected its parameters: `fps` 500 is not
                             between 1 and 240)
```

**The command cannot set the rate the document already has.** A file could
carry a frame rate that no command could ever set — not even to the value it
already had.

### Why nothing noticed

**Nothing compared the two.** The two limits sat in different crates, for
different reasons — "a rate of zero divides by zero" (the core) and "how fast
may a plan run" (the command) — and each was defensible on its own. What was
not defensible was that the *document* answered to the first and the
*command* to the second, and the two answers overlapped only up to 240.

**The number was in prose three times, and no gate covered it.** The §74 gate
compares a **named** constant in prose with the code — `` `MAX_NODES` 10 000 ``
— and the frame rate's ceiling was never a named constant, so "refuses 0 and
anything above 1000" was prose nobody could check.

**The UI's comment recorded the wrong number**, and the UI is where a person
would look: it said the rate is "validated to 1..=240 by `doc.range`" — which
was true of the command and false of the reader, and false of the file the
panel was drawing.

### The fix

**One limit, in the model's table**, where the document's other limits live:

```rust
// crates/model/src/lib.rs
/// The fastest a document may run.
pub const MAX_FRAME_RATE: u32 = 1_000;
```

**The core keeps the check that has to live next to the type** — a rate of
zero divides by zero — and the *policy* lives in the model:

```rust
// crates/core/src/tick.rs
pub const fn new(fps: u32) -> Result<Self, crate::CoreError> {
    if fps == 0 {
        return Err(crate::CoreError::LimitReached { limit: "frame rate" });
    }
    Ok(Self(fps))
}
```

**The command uses the model's number**, not a copy of it:

```rust
if f < 1 || f > i64::from(limits::MAX_FRAME_RATE) {
    return Err(EngineError::Param {
        …
        format!("{f} is not between 1 and {}", limits::MAX_FRAME_RATE),
    });
}
```

The comparison is on the `i64` the parameter is, **not on a cast of it**:
casting first would let 4 294 967 297 become one frame per second.

**The prose stops quoting the number.** The parameter doc says "Frame rate in
frames per second" — like every other parameter doc — and the range is in the
error message, which is where a caller meets it. The two documents and the
UI's comment now name the symbol instead of the value.

**The value is 1000**, the reader's: no file that opens today stops opening.
Whether 1000 or 240 is the right ceiling is the owner's; it is now *one*
number in *one* place, so the answer is a one-line change.

### What now holds it

| Property | Held by |
|---|---|
| The command can set any rate a document may carry | `the_command_can_set_any_rate_a_document_may_carry` |
| A document may not carry a rate above the model's | `an_unusable_frame_rate_is_refused` |
| A rate of zero is refused where the type is | `frame_rate_zero_is_rejected_rather_than_dividing_by_it` |

and two mutations in `scripts/mutation-test-gates.sh`: the command's ceiling
not being the model's, and the model accepting a rate the command refuses.

### The lesson

**Two limits for one thing is a hole; two limits for one thing in *different
crates* is a hole with a plausible-looking shape.** The core's 1000 and the
command's 240 each looked like a deliberate choice about a different
question, and neither was wrong on its own. What was wrong was that the
*document* was governed by one and the *command* by the other.

**A number in prose is only safe if it is a number the code can be asked
about.** The §74 gate works by naming the constant; a limit that is written
out in five places and named in none of them is five chances to be wrong, and
the tree's own gate could not see a single one of them.

**A comment that records a number is a copy of a number.** The UI's comment
was the most misleading of the five: it was true of the command, false of the
reader, and written in the place a person would look to find out what the
file in front of them was allowed to say.

---

## 90. The tree promises exact frame boundaries and does not deliver them for every rate it accepts

§89's measurement had a second number in it: the tree's own tick rate does
not divide by every frame rate it accepts.

### The measurement

`TICKS_PER_SECOND` is 254 016 000 000, and a frame is that divided by the
rate — an **integer** division:

```text
fps   24: ticks per frame = 10584000000 (remainder  0 of 24) — exact
fps   25: ticks per frame = 10160640000 (remainder  0 of 25) — exact
fps   30: ticks per frame =  8467200000 (remainder  0 of 30) — exact
fps  240: ticks per frame =  1058400000 (remainder  0 of 240) — exact
fps   11: ticks per frame =  23092363636 (remainder 4 of 11) — TRUNCATED
fps   13: ticks per frame =  19539692307 (remainder 9 of 13) — TRUNCATED
fps   17: ticks per frame =  14942117647 (remainder 1 of 17) — TRUNCATED
```

and the error is not a rounding of one frame — it **grows**:

```text
fps 11: after 1000 frames the boundary is 363 ticks early
fps 13: after 1000 frames the boundary is 692 ticks early
```

A tick is 3.9 ns, so that is 1.4 µs after ninety seconds at 11 fps — and
fifty-six milliseconds after an hour. The playhead's frame number is visible,
so the drift is visible too. (§115 made that sentence true: until then the
frame number was on no screen at all — the slider's value was hidden and the
formatter that would have written it down never ran.)

The claim that is not kept is the tree's own, in the module that is called
"Exact time":

> Because it is a power of two times an integer, conversions between ticks and
> frame boundaries — including the 29.97 fps NTSC drop-frame timebase — are
> exact integer arithmetic with **no accumulated rounding**.

For a rate that divides the tick rate, that is true. For a rate that does not,
the conversion is a truncation and the rounding accumulates.

### Why nothing noticed

**The rates people use divide evenly.** 24, 25, 30, 50, 60, 120, 240 — every
one of them is exact, so the promise holds for every document anyone has
actually made. The tree has no test that says so, and no test that would have
noticed if it stopped being true.

**Nothing in the tree asks whether a rate is exact.** The tick rate is a
power of two times an integer; whether a *rate* divides it is a fact the
model could state and never did.

### The fix

**The model can now say whether a rate is exact**, and the answer is derived
rather than listed:

```rust
/// Whether a rate divides the tick rate exactly.
///
/// A frame boundary is exact only when the rate divides `TICKS_PER_SECOND`
/// with no remainder. Otherwise the ticks per frame are truncated, and the
/// error grows with the number of frames. §90.
pub const fn is_exact(self) -> bool {
    TICKS_PER_SECOND % self.0 as i128 == 0
}
```

**The promise is corrected to what the tree keeps.** The tick is exact — that
is the tree's real claim, and it is what "Exact time" means: a time is a
count, not a float. The *frame boundary* is exact for the rates that divide
the tick rate, and is short by less than a tick per frame for the others,
which is why the error grows. The document says so, in the place that
promised it.

**The owner's question stays open**, because it is a product question and not
a bug: *should a document be allowed a rate whose frame boundaries are not
exact?* If not, the rule is already derived — `FrameRate::is_exact` — and the
reader and the command can refuse those rates with it. The answer is
recorded in "Suggested order" below.

### What now holds it

| Property | Held by |
|---|---|
| Whether a rate is exact is a fact the model can state | `FrameRate::is_exact` |
| The tree's own default rate is exact | `the_default_rate_divides_the_tick_rate_exactly` |
| A frame boundary at an exact rate is the exact time | `an_exact_rate_places_a_frame_boundary_exactly` |
| The promise in prose is the promise the code keeps | the module doc of `crates/core/src/tick.rs` |

### The lesson

**"Exact time" is a claim about the tick, and the tick is exact.** A frame
boundary is a *derived* quantity, and a derived quantity inherits the limits
of the representation it is derived from. The module said "exact time" and
meant it; the paragraph underneath it over-claimed by one step.

**A promise that holds for every rate anyone uses is not a promise.** It is a
lucky sample. The tree's rates divide evenly, so the tree looked exact — and
nothing ever asked whether that was a property of the *rates* or of the
*users*.

**A drift that grows is a different animal from a rounding.** Rounding is
bounded; truncation in a conversion is not. The distinction is worth one
sentence in the code that does the conversion.

---

## 91. The model checked a keyframe's value and none of the other things it promised about a keyframe

§73 made the model check the numbers a file carries. It checked a keyframe's
**value** — and stopped there.

### The measurement

A keyframe is three things: a **time**, a **value** and an **easing**. A
keyframe *list* has three more rules: it is **sorted**, **unique**, and
within `MAX_KEYS`. The model keeps all of them when it *builds* a list. A
**file** does not build one — it is deserialized straight into the model — so
the rules were never kept on the way in, and (until this was fixed) never
*checked* either.

Seven cases, each written by hand into a file and loaded through the real
reader. Before the fix, **every one of them loaded, and every one of them
validated `Ok`**:

| What the file carried | What the model says about it | What the reader did |
|---|---|---|
| keys at 500 and 0 — **out of order** | "a sorted, unique list" | loaded it |
| **two keys at the same time** | "a sorted, unique list" | loaded it |
| a time of `18446744073709551616` | `Tick::from_raw` clamps; the tick's own doc says "the reader refuses such a file" | loaded it |
| a time of `-99999999999999999999` | the same | loaded it |
| a bezier with `out_x: 5.0` | `Easing::checked()` refuses it | loaded it |
| a bezier with `out_x: 1e40` (which is `inf` in `f32`) | `Easing::checked()` refuses it | loaded it |
| **10 005 keys** on one property | the limit is 10 000 | loaded it |

The worst of them is not a tidiness problem. An **unsorted** list does not
sample to an untidy answer — it samples to the **wrong one**:

```text
keys at 0 (value 2.0) and 500 (value 1.0), sampled at tick 50
  sorted list   -> 2.0 + (1.0 - 2.0) x (50/500) = 1.9   <- right
  unsorted list -> 1.0                               <- what the file gave
```

The key that is *earliest in time* is never reached: the list holds from its
first element, so the animation starts at the **later** key's value and the
earlier one is dead. Nothing said so; the document validated.

### Why nothing noticed

**Nothing ever built a broken list.** `Keyframes::set` sorts, replaces and
bounds the list, so every list the tree makes is correct. The bug was
therefore *unreachable* from the tree's own API — and every test in the tree
built its lists through that API.

**The one field that was checked was the field that is hard to get wrong.** A
number past the limit is a number; the tests for §73 wrote one. A *time* and
an *easing* are different kinds of thing, and the check never grew to them.

**The reader checked everything else.** The document's frame rate is checked
(§89), its range is checked (§60), its properties are checked (§73, §88) —
and its keyframes were the one part that was not. The check was not missing
*from* the tree; it was missing *from one field*.

### The fix

**One place knows what a keyframe list must be.** `Keyframes::check` lives
next to the type that has the rules, and the model's property check calls it:

```rust
pub fn check(&self, what: &str) -> Result<(), CoreError>
```

It checks the count, every key's time against the range a reply could carry,
every key's easing against `Easing::checked`, every key's value (§73), and
the list's own two rules — sorted, and no two keys at the same time. The
message names the property, so a file with many properties says which one is
wrong.

**The door the easing comes through is the model's.** `Keyframes::keys_mut`
is gone: it handed out `&mut [Key]`, which is a way to break the list's
rules without the tree knowing. `key.ease` now goes through
`Keyframes::set_ease`, which checks the easing before it stores it — so an
easing the model's own constructor refuses cannot enter by the back door
either.

**The reader refuses such a file, and the engine keeps refusing it.** The
model's check is called by `Document::validate`, which the reader calls
(§50's rule, §73's precedent) and which the engine calls after **every**
command (§50). So a file carrying a broken list is refused at the door, and a
command that broke one would be rolled back.

### What now holds it

| Property | Held by |
|---|---|
| A keyframe list is sorted, unique and within the limit | `Keyframes::check` |
| A keyframe's time is one a reply could carry | `Keyframes::check` |
| A keyframe's easing is one the model accepts | `Keyframes::check`, `Easing::checked` |
| A file carrying a broken list is refused | `a_file_carrying_broken_keyframes_is_refused` |
| A property with broken keyframes is refused | `a_property_with_broken_keyframes_is_refused` |
| An unsorted list samples to the wrong value | `an_unsorted_list_samples_to_the_wrong_value` |
| A command cannot break the list's rules | `Keyframes::set_ease`, and no `keys_mut` |

and eight mutations in `scripts/mutation-test-gates.sh`:

- the model's property check does not check the keyframe list at all;
- a list that is not sorted is accepted;
- two keys at the same time are accepted;
- a keyframe on a time no reply could carry is accepted;
- an easing the model refuses is accepted;
- a bezier control point outside 0..=1 is accepted;
- a property may carry more keyframes than the limit;
- `set_ease` stores an easing the model refuses.

### The lesson

**A check that grows one field at a time needs a reason to grow the next.**
§73 checked the value because the value was the thing a file could get
wrong. The time and the easing were the same kind of thing and nobody wrote
them down.

**"The constructor keeps it" is not the same as "the tree keeps it".** The
rules a constructor keeps are exactly the rules a *file* breaks, because a
file does not go through a constructor. The tree had a rule about sorted
keyframes and no rule about *checking* them.

**A wrong answer is worse than no answer.** An unsorted list does not fail
loudly and it does not look wrong: it animates, smoothly, to the wrong
values. The refusal is worth more than the tidiness.

**The escape hatch was the hole.** `keys_mut` existed for one caller, and
one caller is enough to make an invariant unenforceable. Removing it was
cheaper than documenting it.

---

## 92. The tree had three answers to "what is a name?", and a user's bad parameter was reported as a crash

§91 was about a validator that checked one field of a thing and not the
others. This is the same shape one level up: **a rule that three places kept,
differently.**

### The measurement

The tree splits its failures in two, and the split is load-bearing:

> Whether this is a crash rather than a refusal. The distinction drives
> everything downstream: an internal error is worth a crash report and a bug, a
> refusal is worth a better message.

So a command that is handed an empty name must refuse it — with a **param**
error, before it changes anything. Two commands did not:

```text
node.add { "name": "" }
  -> kind=internal
  -> `node.add` failed unexpectedly (the command left the document in a state
     the file format refuses: bad metadata: node n4 has an unusable name);
     your document is unchanged
```

The command *accepted* the empty name, added the node, and the **document
validator** — the backstop — refused it afterwards. The user's empty name came
back as the tree's own unexpected failure, of the kind the tree documents as
worth a bug report. An agent branches on that kind, so a typo became a crash.

And the two commands that take a name did not agree:

```text
node.add    { "name": "   " }  -> ACCEPTED: added source `   `
node.rename { "name": "   " }  -> refused: `name` this parameter is required
```

The same name, two commands, two answers.

A third case, of the same family:

```text
doc.range {}
  -> kind=internal
  -> `doc.range` failed unexpectedly (the command declared that it mutates the
     document but changed nothing); your document is unchanged
```

Asking a command to change nothing is the **user's** mistake. It set every
value to what it already was, changed nothing, and hit §44's backstop — which
is an internal error, because a command that declares that it mutates and does
not is supposed to be a tree bug.

And a message that points at the wrong mistake:

```text
node.rename { "node": 1, "name": "" }
  -> `name` this parameter is required
```

The parameter **was** given. It was empty.

### Why nothing noticed

**The name rule was written five times, and the copies were not equal.**

| where | rule | unit |
|---|---|---|
| `Params::text` | at most 255 | bytes |
| `Document::add_node` | at most 255 | bytes |
| `Document::validate` | not empty, at most 255 | bytes |
| `check_name` (the engine) | at most 255 | **characters** |
| `required_text` (the engine) | not empty, not blank | — |

The engine's character count could never fire: the parameter had already been
refused by the byte count. It was a copy of a rule, in the wrong unit, that
was dead code.

**The backstops were doing the commands' job.** §44's "changed nothing" and
§50's "invalid document" are *backstops* — they exist for a command that
breaks a rule the model keeps. A command that never checked the rule in the
first place makes them the *primary* path, and they report in the wrong
category.

**The test that promised to catch this fired the wrong things.** The module
that holds the parameter reader says:

> `hostile_params_never_panic` — a test that fires every registered command at
> every documented parameter name with every hostile value

Measured, it fires **five fixed parameter objects**:

```text
21 commands x 5 hostile objects = 105 firings
  47 reach a command's body
  58 are stopped by `deny_unknown` before the command's own code runs
27 documented parameters are never fired with a hostile value at all
  (node.add.name, node.add.kind, doc.range.fps, key.ease.easing, ...)
```

Firing every documented parameter name with every hostile value finds **no
panic** — and finds all four of the defects above.

**One of the five copies of the name rule could not fire, and that was how it
was found.** A mutation that removed `required_text`'s blank check changed
**nothing** that any test could see, because `check_name` — which asks the
model — refused the same name one line later. The engine's own character
count was the same story: the parameter had already been refused by the byte
count. A copy of a rule that can never fire is not a safety net; it is a
second thing to keep in step, and the mutation suite is what says so.

**And fixing this blinded a gate.** The full mutation run after the fix
reported one missed case: *"a command may leave a document the file reader
refuses"*. The case removed the engine's backstop — the check that refuses a
command which leaves a document the reader would refuse — and it used to be
caught by `node.add { name: "" }`, which was the only real command that could
leave an invalid document. Now that the command refuses an empty name, **no
real command can**, so the backstop is reachable only through the test-only
command that exists to reach it. The case now names that command. A gate that
stops biting because the bug it watched is gone is not a gate that has
nothing left to do — it is a gate that has been pointed at nothing.

### The fix

**The model owns what a name is.** One rule, one place, next to the type that
has it:

```rust
impl Node {
    pub fn why_not_a_name(name: &str) -> Option<String>
}
```

Not empty, not blank, at most `MAX_NAME` **bytes** — the unit every other text
limit in the tree is counted in. The constructor (`Document::add_node`), the
validator (`Document::validate`) and the command (`check_name`) all ask it, so
they cannot disagree.

**A bad parameter is refused before the document changes.** `check_name` now
asks the model's rule and returns a **param** error naming the parameter, so
the command never reaches the backstop with the user's mistake in hand.

**A command asked to change nothing says so.** `doc.range` with no `start`,
`end` or `fps` — including a `null`, which means "leave this alone" — is
refused with a param error, not caught by §44's backstop.

**A parameter that was given but is empty says that.** `required_text` tells
"required" (it was not there) from "must not be empty" (it was there).

**The hostile test now does what it says.** The values are derived from the
registry, so every documented parameter is fired with every hostile shape, and
a second test asserts that the coverage holds — a command that documents a
parameter nobody fires is a parameter nobody has checked.

### What now holds it

| Property | Held by |
|---|---|
| A name is not empty, not blank and within the limit | `Node::why_not_a_name` |
| The constructor refuses a name the validator would refuse | `the_constructor_refuses_a_name_the_validator_would_refuse` |
| A file carrying an unusable node name is refused | `a_file_carrying_an_unusable_node_name_is_refused` |
| An empty name is a refusal, not an internal failure | `an_empty_node_name_is_a_refusal_not_an_internal_failure` |
| Every command that takes a name agrees about a name | `every_command_that_takes_a_name_agrees_about_what_a_name_is` |
| A command asked to change nothing is a refusal | `a_range_command_with_nothing_to_change_is_a_refusal` |
| A parameter that was given but is empty says so | `a_parameter_that_was_given_but_is_empty_says_so` |
| Every documented parameter is fired with a hostile value | `every_documented_parameter_is_fired_with_a_hostile_value` |

### The lesson

**A backstop is not a substitute for a check.** §44 and §50 are the tree's
safety net, and they report in the category that means "a bug". A command
that leans on them has turned the user's typo into a crash report.

**Three copies of a rule are three rules.** The name rule was written five
times, in two units, with three different answers — and the copy that was
*wrong in the unit* was also the copy that could never fire, so it looked
harmless.

**A test's promise is part of the test.** "Every documented parameter name"
was a sentence in a module doc, and the test below it fired five hard-coded
objects. The sentence was the *specification*; nothing checked that the code
met it. Deriving the values from the registry is what makes the sentence true
and keeps it true as commands are added.

**A test can be right and still be weak.** The hostile test passed. It was
also missing 27 parameters' worth of coverage — and the four defects it
should have found were all *refusals*, not panics, so a test that only
asserts "it did not panic" would not have seen them either.

---

## 93. The engine could not tell "the command forgot to write" from "the command wrote what was already there"

§92 fixed the *kind* of a refusal. This is the next layer: the engine
refused commands that had done nothing wrong at all.

### The measurement

§44 put a backstop in the engine: a command that declares `mutates: true`
and changes nothing is a bug — usually one that forgot to write — so the
engine refuses it and rolls the document back.

The check compares the document with the snapshot taken before the command.
That comparison cannot tell two things apart:

```text
a command that forgot to write        -> the document is identical
a command that wrote the value a      -> the document is identical
field already held
```

The second is not a bug. It is what "set the amount to 1.0" means when the
amount is 1.0. Measured, **eight of the nine** commands that can be asked to
do what they have already done were refused:

```text
prop.set    the value a property already had   -> internal
prop.reset  a property with no keyframes        -> internal
key.clear   a property with no keyframes        -> internal
node.mute   a node that is already unmuted      -> internal
node.kind   the kind a node already has         -> internal
node.rename the name a node already has         -> internal
doc.rename  the name the document already has   -> internal
doc.range   the range the document already has  -> internal
```

Each with the same message:

> `prop.set` failed unexpectedly (the command declared that it mutates the
> document but changed nothing); your document is unchanged

"Failed unexpectedly" is the tree's **internal** classification — the kind
the tree documents as "worth a crash report and a bug". An agent branches on
the kind. So the ordinary operation "set this to what it already is" was
reported as the tree's own unexpected failure.

And the comment on the test-only command that exists to reach the check said
the opposite of the truth:

> No command in the tree does that, so nothing ever reached the check.

Eight commands reached it every day.

### Why nothing noticed

**No test asked a command to do what it had already done.** Every test in
the tree changed something. The backstop only fires on a *no-op*, so a suite
that never asks for one never sees it.

**The check was right and the comparison was wrong.** §44's rule is sound: a
command that claims to mutate and does not is a bug. What the engine could
not do was tell that case from the other one.

**§92 made it worse before it made it better.** The fix for §92 — a command
that is handed an empty name refusing it — removed the only *real* command
that reached the backstop with an invalid document behind it. The full
mutation run after §92 reported one missed case, and it was §50's: the
backstop's removal had stopped being visible.

### The fix

**The command says whether it changed anything.** `Outcome` gains a `changed`
flag, and the engine believes it:

```rust
let changed = self.doc != snapshot;
if !changed && outcome.changed {
    // the command says it wrote and the document says otherwise: a bug
}
```

The default is `true`, which is the direction that keeps §44's check alive:
a command that forgot to write says nothing, so it is still caught. A
command that may legitimately change nothing says so — through
`Outcome::unchanged` — when it did not.

**The write says, not the command.** The model owns the rule, as it does
everywhere else: `Node::rename`, `Node::set_kind`, `Node::set_muted`,
`Property::set_value`, `Document::set_name`, `Document::set_range` and
`Document::move_to` all return whether they changed anything. The command
passes that on, so there is no place where a command has to remember to
compare two values by hand.

**A no-op is a success, and it leaves no trace.** It does not put a step on
the undo stack, it does not mark the project dirty, and it does not fail. The
document is the one that was there before, so it is already valid.

### What now holds it

| Property | Held by |
|---|---|
| A command asked to do what it has already done succeeds | `a_command_asked_to_do_what_it_has_already_done_succeeds` |
| Keying and easing what is already there succeeds | `keying_and_easing_what_is_already_there_succeeds` |
| A command that claims to change something and does not is still refused | `a_command_that_claims_to_change_something_and_does_not_is_still_refused` |
| A no-op puts nothing on the undo stack | `a_command_asked_to_do_what_it_has_already_done_succeeds` |
| A write that changes nothing says so | `a_write_that_changes_nothing_says_so`, `writing_a_value_a_property_already_holds_says_it_changed_nothing` |

and six mutations in `scripts/mutation-test-gates.sh`:

- a command asked to do what it has already done is a crash;
- a command that changed nothing is not caught;
- a no-op puts a step on the undo stack;
- muting a node that is already muted is a crash;
- setting a property to the value it already has is a crash;
- a write that changes nothing does not say so.

### The lesson

**A check that compares a *result* cannot tell two different *acts*.** The
document before and after are the same in both cases; the difference is
whether the command wrote. The check had to move to where the knowledge is.

**The default must be the strict side.** If "the command changed something"
were the default, a command that forgot to write would pass the check
silently. The default says "it changed something", so the bug is caught and
the *honest* case has to opt out. A rule that is opt-out by default fails
safe.

**Eight commands reached a check every day, and the comment said none did.**
The comment was written when the check was new and the commands had not yet
been exercised. The comment was a description of a code that had never been
run against those commands — §65's shape, nineteen sections earlier.

**A no-op is not a failure, and it is not a change.** It is a success with
nothing to undo. The engine had three states for "the document is the same"
and used the wrong one.

---

## 94. The catalogue told every agent that four commands were undoable, and none of them could be

§93 fixed what happens when a command changes nothing. This is the other half
of the same flag: what the tree *says* about undoing.

### The measurement

`CommandSpec` has two flags, and the second one's own comment is its
definition:

```rust
/// Whether a successful run pushes an undo entry.
pub undoable: bool,
```

The engine pushes the entry inside the `mutates` branch:

```rust
if spec.mutates {
    …
    if spec.undoable { self.history.push(snapshot, spec.label); }
}
```

So a command that does **not** change the document has nothing to undo, and
cannot push an undo entry however it is declared. Four commands declared it
anyway:

```text
playback.seek      mutates=false, undoable=true
plan.evaluate      mutates=false, undoable=true
diagnostics.list   mutates=false, undoable=true
session.stats      mutates=false, undoable=true
```

Measured, none of them pushes an entry, and none of them can:

```text
playback.seek      undo depth 0 -> 0
plan.evaluate      undo depth 0 -> 0
diagnostics.list   undo depth 0 -> 0
session.stats      undo depth 0 -> 0
node.add           undo depth 0 -> 1     (a command that does mutate)
```

And the flag is **published**. The command schema — the JSON that
`engine.describe` returns, that `commands --json` prints and that the MCP
server turns into a tool description — carries `"undoable": true` for all
four. An agent reading the catalogue is told that `playback.seek` is undoable.

The history only ever stored the **document**, so the playhead was never
undoable either: the claim could not be made true by changing the flag.

### Why nothing noticed

**The flag was read in one place and written in twenty-one.** Nothing checked
the *pair*. A command's `mutates` and `undoable` were two literals next to
each other, and the relationship between them was in the engine, not in the
data.

**Every command that mattered was consistent.** The fifteen mutating commands
were `mutates: true, undoable: true` (apart from the two history commands,
which are false on purpose), so the flag looked right everywhere anyone
looked.

**The prose described the flag's *purpose*, not its *rule*.** The pattern
document said the right thing — *"`mutates` decides whether the snapshot
becomes an undo entry"* — and the field's comment said the wrong thing. Two
sentences about one flag, one of them true.

### The fix

**The data is honest.** The four commands are `undoable: false`, which is
what they have always been in fact.

**The rule is in the field's own comment**, next to the data it constrains:
only meaningful together with `mutates`; a command that does not change the
document has nothing to undo.

**A test keeps it that way.** `no_command_that_does_not_mutate_is_undoable`
walks the registry and refuses the combination — so a new command cannot
repeat the mistake, and the rule is in one check rather than in a reviewer's
memory.

**The prose that named the old state is corrected.** `docs/architecture.md`
said "`history.undo` and `history.redo` are the two commands with
`undoable: false`". There are six now, and the sentence says why.

### What now holds it

| Property | Held by |
|---|---|
| A command that does not mutate is never undoable | `no_command_that_does_not_mutate_is_undoable` |
| The flag's comment says what the flag means | the comment on `CommandSpec::undoable` |
| The catalogue does not claim what the engine cannot do | the same test, through the registry |

and one mutation in `scripts/mutation-test-gates.sh`: a command that does not
mutate is undoable anyway.

### The lesson

**A flag and the rule that constrains it belong in the same place.** Two
literals next to each other invite the reader to assume they are independent.
They are not.

**A published flag is a promise.** The schema goes to agents, and the MCP
server turns it into a tool description. A flag that says `true` where the
engine can never do the thing is a false promise made to every reader at once
— and it cost nothing to fix, because the truth was already what the engine
did.

**A flag whose value is always the same for a subset of the data is a second
copy of another flag.** `undoable` was `mutates` for fifteen commands, `true`
for six, and meaningless for four. The meaningful values are the exceptions,
and the exceptions are what need checking.

## 95. The reader looked at the changes a file still needed, and never at the ones it had

§81 gave the pending mechanism a rule for the changes a file still **needs**.
This is the other direction, and it is the one that decides whether the reader
understands a file at all.

### The measurement

The file format promises this:

> A reader refuses a version it does not know rather than guessing, because a
> guessed read of a newer file silently corrupts a design.

But a format change that alters what is saved does **not** bump the version —
that is the whole point of the pending scheme. So the version cannot be the
only thing that says whether this build understands a file. The other thing is
the file's own `meta.pending` list: the changes that had been applied to it.

The reader's rule was one-sided. `pending::apply_steps` works out what the
file still needs and applies it:

```rust
fn missing<'a>(names: &[&'a str], already: &[String]) -> Vec<&'a str> {
    names.iter().copied().filter(|name| !already.iter().any(|a| a == name)).collect()
}
```

The file's list is used to **subtract** from this build's list, and that is all
it is ever used for. Nothing ever asks the other question — *which changes has
the file had that this build has never heard of?* — because a subtraction
cannot ask it.

Measured, on a file at the current version whose header names a change this
build has never heard of:

```text
PROBE loaded: yes
PROBE nickname survived the round trip: false
```

The file loaded without a word of complaint. Its node carried a field this
build has never heard of, the field was dropped on the way in, and the file
written back had no trace of it — so the next save **loses the data**. The
file's own header was rewritten too: the saved file says `"pending": []`, so
the file's record of its own history became a lie as well.

The version tag could not catch it, and could not have: the file said
`craftkit/1`, exactly like every file this build writes. A pending change
does not bump the version, so the tag says nothing about the shape.

### Why nothing noticed

**The rule was one-sided by construction.** `missing` is a subtraction, and a
subtraction never looks at what it subtracts from. The name of the function
says what it was for, and nothing else was ever written.

**There was nothing to be wrong about.** The pending list has been empty since
version 1 (§45), so the whole mechanism has never run: a file written by this
build names no changes, and this build has no changes to name. The reader
could not be wrong about a change that did not exist.

**The reader had no way to say it.** There was no error to give: a build that
has never heard of a change has nothing to compare the name against, so
silence was the only thing it could produce.

**And the tree has never read an older file.** Version 1 is the first version
there is, so there is no version before the current one to write — and the
test that claimed to read an older file was tagging the file with version 1,
which is the *current* version. It was reading a file of its own age and
calling it an older one. "Open a design written by an earlier craftkit" has
never happened, and cannot until version 2 exists.

### The fix

**The other direction is a function of its own.** `pending::unknown_names` is
the mirror of `missing`: it works out what the file has had that this build
does not have, and it is a pure function that a test can exercise while there
is still nothing pending.

**A refusal that names the change.** `FormatError::UnknownChange` says which
change the file names and that this build has never heard of it. The file is
refused, loudly, instead of being read as a design this build understands:

```text
this file has had the format change `a_change_this_build_has_never_heard_of`,
and this build has never heard of it. It was written by a build with a format
change this build does not have; open it with that build.
```

**The check is in the reader.** `File::check` refuses such a file, and
`File::check_with` does the same against a known set — so the seam that §81
needs, a test that drives the reader with a step of its own, can say that the
file's header names a change this reader *does* have.

**The rule is in one pure function.** `unknown_changes` takes the file's
version, this build's version, the file's list and this build's list, and
answers with the names that may not be read. That is what makes the older-
version half testable at all: the tree has no version older than the current
one, so the rule is asked with the two versions it is about — 1 and 2 — rather
than with a file that cannot be written yet.

**An earlier file is still read.** A change folded into a later version is
not in this build's pending list any more, so a file from before the fold
names a change this build cannot find — and refusing every such file would
refuse every design written before the last release. What a folded change *is*
is still the open question it was, and it is recorded rather than guessed at.

**The test that lied says so.** `an_older_file_is_read` is now
`a_file_at_the_current_version_is_read`, and its comment says why the tree has
no older file to read.

### What now holds it

| Property | Held by |
|---|---|
| The changes a file has had that this build does not have are worked out | `the_changes_a_file_has_that_this_build_does_not_have_are_worked_out` |
| A file at the current version naming an unknown change is refused | `a_file_that_names_a_change_this_build_has_never_heard_of_is_refused` |
| A file naming a change this build has is read | `a_file_that_names_a_change_this_build_has_is_read` |
| An earlier file may name a change that was folded | `an_earlier_file_may_name_a_change_that_was_folded` |

and four mutations in `scripts/mutation-test-gates.sh`: the other direction is
never worked out, the refusal does not fire, the known set loses the seam's
steps, and the message does not name the change.

### The lesson

**A rule that only runs one way is half a rule.** The reader asked what a file
needed and never what it had. Both questions are about the same list, and a
list that is only ever subtracted from cannot tell a file it understands from
a file it does not.

**A promise about the future needs a way to be wrong.** "A reader refuses what
it does not understand" was true about the *version* and untrue about the
*shape*, and the shape was the part a pending change moved. A scheme that
deliberately does not bump the version has to say what does.

**The mechanism that has never run is the one with no evidence.** §45 found
that the pending steps had never executed. The same is true of the check
around them: with an empty list there is nothing to be wrong about, and a
reader that cannot be wrong is a reader nobody has tested.

**A test can be named for a thing it does not do.** `an_older_file_is_read`
read a file of the current version, because there is no other version to read.
The name was wrong in a way that hid the gap: the tree had never opened a
design from an earlier craftkit, and the test that said it had was reading its
own.

---

## 96. Every backstop the engine had was on one side of the `mutates` flag

The engine's `mutates` flag says that a command does not change the document.
The engine has three backstops for a command that breaks a rule: §44's *"the
command declared that it mutates the document but changed nothing"*, §50's
*"the command left the document in a state the file format refuses"*, and the
panic rollback.

Every one of them is inside `if spec.mutates { … }`. That is where the undo
entry and the dirty flag live, so that is where the checks went with them.

The other side of the flag — a command that says it does **not** change the
document and changes it — had nothing at all.

### The measurement

With a test-only command that declares `mutates: false` and renames a node:

```text
PROBE result: Ok("renamed the node and claimed to be read-only")
PROBE name before: "Taper" after: "renamed"
PROBE dirty: false
PROBE can_undo: false
```

The command **succeeded**. The write **landed**. The project was **not** marked
dirty, there was **no** undo step, and the document was never validated.

And with a test-only command that declares `mutates: false` and empties the
range:

```text
PROBE invalidate result: Ok("the document is now invalid")
PROBE document valid now: false
PROBE dirty after invalid: false
```

The session was left holding a document that `Document::validate` refuses — a
design that **cannot be written to a file**, because the writer refuses it —
and the app would have drawn it.

The consequences are not hypothetical. The project is not marked dirty, so
closing the file loses the change, and nothing ever said the file was unsaved.
The change is not undoable. And a design the file reader refuses is a design
the app cannot open again.

### Why nothing noticed

**The flag's promise was kept by a test, not by a rule.**
`every_read_only_command_leaves_the_document_alone` proves that the four
commands in the registry do not write. That is a fact about *today's*
commands, checked when the tree is built — not something the engine applies.
Delete the engine's check and that test still passes, because the four
commands still do not write.

**Every backstop was written for the `mutates: true` side.** They live in the
same branch because that is where the undo entry and the dirty flag live, and
nobody asked what the branch does for everybody else.

**And the flag is published.** `mutates` is in the command schema, in
`commands --json` and in the MCP tool description. An agent reads
`mutates: false` as *"safe to use as a probe"* — the read-only commands are
exactly the ones an agent is told to use as a probe. The flag was a promise
with no backstop behind it.

### The fix

**The engine compares the document before and after every command**, not only
for the mutating ones, and refuses a command that declared it does not mutate
and changed the document — with the document rolled back, and the refusal
saying which of the two backstops it was.

**The cost is nothing.** The snapshot is already taken, for every command,
mutating or not:

> Clone before edit. Taken for *every* command, mutating or not, so the
> restore path below is a single unconditional assignment.

The comparison is the same one the `mutates` branch already makes. The clone
that the read-only commands paid for, for a restore that never came, now buys
the evidence.

**The flag's own doc says it is a promise the engine keeps, on both sides**,
and the read-only table in the tree's tests says the same thing the rule 13
of the playbook says.

### What now holds it

| Property | Held by |
|---|---|
| A command that declares it does not mutate and writes is refused | `a_command_that_declares_it_does_not_mutate_and_writes_is_refused` |
| A command that declares it does not mutate cannot leave an invalid document | `a_command_that_declares_it_does_not_mutate_cannot_leave_an_invalid_document` |

and three mutations in `scripts/mutation-test-gates.sh`: the check does not
fire, the write is not rolled back, and the refusal does not say what the
command did.

### The lesson

**A flag is a promise, and a promise needs a backstop on both sides.** The
engine had a rule for a command that lied upwards — it said it changed
something and did not — and none for a command that lied downwards. The
downwards one is the dangerous one: it is the one an agent trusts.

**A test is a fact about today; a backstop is a rule.** The tree had a test
that the read-only commands do not write, and it was green, and the flag was
still a promise the engine did not keep. The test was the right thing to have
and it was not the thing that was missing.

**A rule that lives in a branch covers the branch.** Every backstop the engine
had was written where the undo entry and the dirty flag were, and the branch
was read as "the mutating commands" rather than as "the commands that
declared they change something". A check that lives inside a condition is
only about the cases the condition selects.

**And the mutation anchor was a lesson of its own.** The first anchor for the
rollback case was one line, and `grep` said it matched once — because `grep`
counts *lines* and the anchor matched as a *substring* of the more deeply
indented restores in the same file. `swap` replaces the first match, so the
mutation landed on the §44 branch and the case never bit. The anchor now runs
over two lines, and the check behind it counts substrings.

---

## 97. The tree published a property path that no command would accept

A path is made of names. A name is at most 255 bytes. So a path is *longer*
than a name — and three layers of the tree had three different answers about
how long a path may be.

### The measurement

| Layer | Its rule | Measured |
|---|---|---|
| the grammar | a segment's name ≤ 255 bytes — in the **plain** spelling only | `parse("a"×256)` → refused; `parse("a"×256 + "#1")` → **accepted** |
| the commands | a path parameter ≤ 255 bytes | `prop.set` with a 309-byte path → refused: *"path 309 bytes, the limit is 255"* |
| the model | a group's id: **no rule at all** | a file with a 30 000-byte group id → read, validated, and the listing published a path of 30 009 bytes |

Measured on files, before the fix, each one read and **validated**:

| The group's id | Read? | The path the listing published | A command could carry it? |
|---|---|---|---|
| 255 bytes | yes | 262 bytes | **no** |
| 256 bytes | yes | 265 bytes | **no** |
| 300 bytes | yes | 309 bytes | **no** |
| 30 000 bytes | yes | 30 009 bytes | **no** |

So the tree published a path it refused to accept — and it did so for a
document the model itself was happy with. A group's id of 255 bytes is a
perfectly legal name; the path to the property inside it is 262 bytes, and
every command that takes a path said *"the limit is 255"*.

And the asymmetry inside the model was plain: the **property label** has had
the 255-byte rule all along — *"property label is 256 characters, the limit is
255"* — and the **group's id**, the other name in the same tree, had none.

### Why nothing noticed

**A path is a name in most people's heads.** The commands' cap was
`MAX_NAME` — the *name* limit — applied to a path, which is made of names. No
document said a path may be longer than a name, so nobody wrote the number
down.

**Two questions about the same string, and only one of them had an answer.**
§88 asked whether a group id is *writable* — whether the grammar can spell it
— and the answer was a round trip through the grammar. Nobody asked the other
question: whether it is *short*.

**The listing is the tree's own view of its data**, so a path it publishes
looks right by construction. §88's lesson was that a published path can mean
something else; this time the published path could not be *sent*.

**And the grammar was inconsistent with itself.** It refused a name of 256
bytes and accepted the same name written `name#1` — because the length rule
was only in the plain branch. That spelling is exactly the one the listing
uses for a long group id, so the hole and the symptom were the same thing.

### The fix

**The model owns the rule.** A group's id is a name, so it obeys the name
rule: not empty, and at most `MAX_NAME` bytes — the same rule the property
label obeys. The reader refuses a file that breaks it.

**The grammar is consistent with itself.** The occurrence spelling obeys the
same length rule as the plain one, so a name the model refuses is a name no
path may spell.

**A path parameter is capped at the grammar's longest path.** The commands
read a path with their own reader now — `required_path` — and the cap is
`MAX_PATH_BYTES`: sixteen segments of 255 bytes and the slashes between them,
4 095 bytes. A name parameter still has a name's limit. So a path the tree
publishes is a path a command accepts.

**And the error message said "characters" where it counted bytes.** The
length rule is in bytes, everywhere; the message said otherwise.

### What now holds it

| Property | Held by |
|---|---|
| A group id longer than a name is refused | `a_group_id_longer_than_a_name_is_refused` |
| The longest path the grammar can express fits the path budget | `the_longest_path_the_grammar_can_express_fits_in_the_path_budget` |
| A name too long is refused in either spelling | `a_name_too_long_is_refused_in_either_spelling` |
| A path longer than a name is still carried | `a_path_longer_than_a_name_is_still_carried` |
| A file with a group id longer than a name is refused | `a_file_with_a_group_id_longer_than_a_name_is_refused` |

and four mutations in `scripts/mutation-test-gates.sh`: the model accepts a
long group id, the grammar accepts a long name in the occurrence spelling, the
commands do not carry a path the grammar can express, and the reader does not
validate what it read.

### The lesson

**A name, a path and a parameter are three different lengths.** The tree had
one number for all three, and it was the smallest of them. The rule was
"everything textual is at most 255 bytes", and a path is not one thing.

**Two questions about the same string need two answers.** "Can the grammar
spell this name?" and "Is this name short enough?" are different questions, and
a rule that answers one of them says nothing about the other. §88 answered
the first for every name; the second was answered for one name out of three.

**A published path is a promise too.** §88's rule was that a published path
must not mean something else. This one is that it must be *sendable* — and a
tree that lists what it will not accept is a tree whose listing cannot be
followed.

---

## 98. The automation registry scanned everything it had published, for every widget

The registry that lets an agent find a widget by name is rebuilt **every
frame**. And for every widget published, it checked whether that name had
already been published — by scanning the whole list.

So a frame cost the square of the number of widgets in it.

### The measurement

The node list publishes two names per node (the row and its level bar), so a
document at the model's own limit of ten thousand nodes publishes twenty
thousand names. Measured, publishing those names:

| Nodes | Names published | Before | After | Per name, before |
|---|---|---|---|---|
| 100 | 200 | 51 µs | 47 µs | 255 ns |
| 1 000 | 2 000 | 3.4 ms | 0.39 ms | 1.7 µs |
| 5 000 | 10 000 | 78.7 ms | 1.9 ms | 7.9 µs |
| 10 000 | 20 000 | **377.9 ms** | **6.9 ms** | 18.9 µs |

The per-name cost is the tell: it grows with the number of names already
published, because each one walks the list looking for a duplicate. A
thousand nodes cost 3.4 ms a frame; five thousand cost 79 ms — twelve frames a
second — and ten thousand, which the model accepts, cost **378 ms a frame**.

And it is every frame: the registry is rebuilt from scratch each time, so the
cost is paid again on every repaint.

### Why nothing noticed

**The check is one line and looks free.** "Have I published this name before?"
is a question about a list of names, and a list of names is small — until it
is twenty thousand of them, and each question walks it.

**The registry is not where anybody looked.** The tree's performance work went
into the *evaluator* (§53 measured a cache that cost 7 400 times the work it
avoided) and into the *document's* memory budget (§52). The registry is the
agent's window on the window: it is rebuilt every frame and nobody measures it,
because it is not on the document and it does not touch the model.

**And the numbers a small document hides are the numbers that matter.** At a
hundred nodes the registry costs 51 µs — a rounding error next to egui's own
layout. The curve only shows up at the size the model *allows*.

### The fix

**The duplicate check is a lookup.** The registry keeps a set of the names it
has published this frame, so "have I seen this name?" is a hash lookup rather
than a walk. The list keeps its order, because the order is what "topmost" and
"first wins" mean.

The copy of the name the set keeps is the price, and it is the same order as
the allocation the caller has already made for it: 6.9 ms for twenty thousand
names, against 378 ms.

### What now holds it

| Property | Held by |
|---|---|
| Publishing a widget does not cost more than the widgets before it | `publishing_a_widget_does_not_cost_more_than_the_widgets_before_it` |

and one mutation in `scripts/mutation-test-gates.sh`: the duplicate check goes
back to being a scan.

The test is **relative** on purpose: a second batch of five thousand names
must not cost twice as much as the first. Linear costs the same; quadratic
costs three times as much, because the second batch walks a list twice as
long. So the test does not depend on how fast the machine is — and the
mutation that puts the scan back measures 3.7× and fails.

### The lesson

**A duplicate check is a lookup.** The tree has one of those rules everywhere
else — a set of the ids it has seen, a map of the names it knows — and here the
same question was answered by walking a list, in a loop that runs once per
widget, on every frame.

**The cost of a frame is not the cost of the work.** A frame does a lot of
small things: a name per widget, a rectangle per widget, a check per widget.
None of them is expensive. All of them together are the frame, and the frame
is what a person feels.

**A limit is a promise about the size of a document.** The model allows ten
thousand nodes. The registry cost 378 ms a frame at that size, so the promise
was "a document the model accepts is a document the window can draw". It was
not true, and nothing measured the gap between the two.

## 99. The level bar wore a property's name, and the registry wore the first frame's

The registry that lets an agent find a widget by name is rebuilt **every frame**.
Its own doc says so, and so does the shell's.

It was not.

Three things were wrong with it, and they are one thing: **a name is a promise about
one widget, in one frame.**

### The measurement

A property may be called `$level` — the grammar writes it plainly, the model accepts
it, and the tree's own fixture does it
(`apps/craftkit-cli/tests/command_effects_survive_the_file.rs` sets `path: "$level"`
on node 1). The node list published the **level bar** as `node.1.prop.$level`, which
is how a property named `$level` spells its own name. The properties panel
published the property's **editor** as `node.1.prop.$level`.

One name, two widgets. The first publication wins, so the property's editor was
**not in the registry at all**. Measured, one frame:

```
PROBE names containing $level: ["node.1.prop.$level", "node.2.prop.$level", "node.3.prop.$level"]
PROBE known node.1.prop.$level: Found { rect: [[54.3 61.0] - [54.3 71.0]], enabled: true }
PROBE row rect:               Found { rect: [[8.0 57.0] - [46.3 75.0]], enabled: true }
PROBE amount rect:            Found { rect: [[9808.0 100.0] - [10009.0 118.0]], enabled: true }
```

The name `node.1.prop.$level` resolved to a rectangle at x = 54 — the node list,
beside the row — while the property's editor lives in the properties panel at
x = 9808. An agent asking for the property's editor was answered with the level
bar's rectangle, in the wrong panel, and the property had no editor to click. The
name was there, the lookup succeeded, and the widget behind it was not the one that
was asked for.

And the rectangle it resolved to was `[[54.3 61.0] - [54.3 71.0]]` — **zero wide**.
The level bar published its *fill*, the coloured part, whose width *is* the level. At
level zero that is a rectangle with no area at all: a name an agent can look up and
never click.

The third defect is in the registry itself. It was never cleared, so it grew for as
long as the shell lived, and "the first publication wins" meant "the first **frame**
wins". Measured, with a node selected and then deselected:

| | entries in the registry | `node.1.prop.amount` |
|---|---|---|
| frame 1, node selected | 31 | Found, at `[[9808.0 100.0] - [10009.0 118.0]]` |
| frame 2, nothing selected | 31 | **still Found**, at the same rectangle |

The properties panel drew nothing in the second frame — the editor was gone from the
screen — and the registry still offered it. That is exactly what the type's own doc
promised would not happen: *"Rebuilt every frame and dropped after, so a widget that
disappears does not linger in the registry and let a click land on stale
coordinates."*

### Why nothing noticed

**A property may be called `$level`, and the tree's own fixture does it.** So the
name was not hypothetical: it is a property a person can create with one command.

**Nothing published a name twice, so nothing had a reason to check.** "First wins"
was there because two *buttons* might want one name. There was no counter, no log
and no record: the second widget was dropped, and the registry answered for it with
the first widget's rectangle. The failure was invisible by construction.

**The doc said the opposite of the code.** `Automation`'s doc promised a per-frame
registry; the shell never cleared it. The promise sat in the place a reader would
look, which is why it survived.

**A count could not see it.** The obvious test — "one editor per property" — passes
while the bug is there: the level bar was *wearing* the property's name, so the
numbers matched. A test that counts what a collision can impersonate is not a test.

### The fix

**A name is one widget.** The registry records the loser of a name
(`Automation::collisions()`), so a collision is something that can be refused rather
than swallowed.

**The level bar has a name of its own.** `AutomationId::level_bar(id)` is
`node.<id>.level` — not a property's name, because it is not a property.

**The registry is rebuilt each frame.** `Shell::show` clears it, so the names and the
rectangles belong to the frame that was drawn.

**The level bar publishes its track**, not its fill: a widget has an area.

### What now holds it

| Property | Held by |
|---|---|
| A level bar and a property never share a name | `a_level_bar_and_a_property_never_share_a_name` |
| A name two widgets want is recorded as a collision | `a_name_two_widgets_want_is_recorded_as_a_collision` |
| A frame publishes no name twice | `a_frame_publishes_no_name_twice` |
| Every property of the selected node has an editor | `every_property_of_the_selected_node_has_an_editor` |
| A property named after the level bar keeps its own editor | `a_property_named_after_the_level_bar_keeps_its_own_editor` |
| A level bar publishes a rectangle with an area | `a_level_bar_publishes_a_rectangle_with_an_area` |
| The registry holds the frame that was drawn | `the_registry_holds_the_frame_that_was_drawn` |

and four mutations in `scripts/mutation-test-gates.sh`: the level bar wears a
property's name again; the level bar publishes the fill again; a collision is
swallowed again; the registry is not rebuilt each frame again.

`crates/ui/src/shell.rs` joined `FILES` for this, because a mutation outside `FILES`
is a mutation the script cannot put back.

The §66 mutation's anchor had to move: it spanned the whole block of name
constructors, and the level bar's constructor now sits among them. It is anchored on
one constructor now, which is where it belongs.

### The lesson

**A name is a promise about one widget.** Two widgets wanting one name is not a tie
to be broken; it is a collision, and the loser's widget is simply *not there*.

**A lookup that succeeds is not an answer.** The name was present, the registry
answered, and the answer was about a different widget in a different panel. The
registry cannot tell an agent that — only the publisher can, so the publisher
records it.

**The doc is not the code.** `Automation`'s doc promised a registry that was rebuilt
every frame; the shell never cleared it. The promise sat in the place a reader would
look.

**A count cannot see a collision.** The intruder wore the victim's name, so the
numbers matched. What sees it is a *record of the conflict*, not a count of the
things.

## 100. The shell drew buttons the registry could not name

The registry's own doc makes a claim: *"Every widget that matters carries an
[`AutomationId`] — a stable dotted name like `toolbar.node.add` or
`nodes.row.4`."*

The node list draws, in every row: the node's name, a level bar, a **mute**
button and a **delete** button. The first two were published. The buttons were
not — and neither were the playhead slider or the two "key at the playhead"
fields.

One name was worse than missing: the **clear keys** button was published as
`node.<id>.key.<path>`, which is the name of a *keyframe row* — a widget the
shell does not draw.

### The measurement

The registry for the sample document, with one node selected, before the fix:
**31 names**, and then nothing:

```
PROBE len: 31
PROBE bar: Found { rect: [[54.3 61.0] - [118.3 71.0]], enabled: true }
PROBE at (119.3, 66.0): None
PROBE at (127.3, 66.0): None
PROBE at (135.3, 66.0): None
...
PROBE at (271.3, 66.0): None
```

Every point to the right of the level bar answers *nothing* — and that is
exactly where the two buttons are. The picture says otherwise: rendering the
shell at 1200 × 400 shows a **mute** and a **del** button on every one of the
three rows, and the playhead handle on the timeline, and the registry names
none of them.

After the fix the same scan answers:

```
PROBE len: 39
PROBE at (127.3, 66.0): Some("nodes.row.1.mute")
PROBE at (183.3, 66.0): Some("nodes.row.1.del")
PROBE at (223.3, 66.0): Some("timeline.playhead")
```

### Why nothing noticed

**The registry was never asked about a button.** The tests asked *"is this
command's button there?"* — and every command's button is in the toolbar,
which is fully published. The node list's own buttons were never in the set of
things anybody looked for. A registry that covers the toolbar completely and
the rest of the screen not at all still passes every question of the first
kind.

**A name that promises a widget that is not there fails quietly.**
`node.1.key.amount` resolves: the name is in the registry, the lookup
succeeds, and what it points at is a button labelled "clear keys". Nothing
about the lookup is wrong; the *promise* is.

**The picture is the evidence nobody looked at.** A rendering of the node list
has six buttons in it. The registry is a text structure, and nobody rendered
the two side by side.

### The fix

Every widget the shell draws that somebody might click now carries a name:

| Widget | Name |
|---|---|
| the mute button in a row | `nodes.row.<id>.mute` |
| the delete button in a row | `nodes.row.<id>.del` |
| the playhead slider | `timeline.playhead` |
| the path field of "key at the playhead" | `properties.key.path` |
| the value field of "key at the playhead" | `properties.key.value` |
| the clear-keys button | `node.<id>.keys.<path>.clear` |

The last one is a **rename**, not an addition: `node.<id>.key.<path>` said
"the keyframe row", and there is no such row. A button that has a command
behind it still needs a name — the registry is about the *screen*, not about
the commands.

### What now holds it

| Property | Held by |
|---|---|
| Every node row has its buttons named | `every_node_row_has_its_buttons_named` |
| The timeline has a name | `the_timeline_has_a_name` |
| The key fields have names | `the_key_fields_have_names` |
| Every published name answers at its own centre | `every_published_name_answers_at_its_own_centre` |
| A name never promises a widget that is not drawn | `a_name_never_promises_a_widget_that_is_not_drawn` |

and five mutations in `scripts/mutation-test-gates.sh`: the row's buttons are
not published again; the clear-keys button promises a row again; the playhead
is not published again; the key path field is not published again; and a
widget is published with another widget's rectangle.

`every_published_name_answers_at_its_own_centre` is the general shape of the
bug: **the registry and the screen can disagree**. A name whose widget is
somewhere else, or is covered by another, points at the wrong thing — and the
lookup succeeds while it does.

### The lesson

**A registry of widgets is a claim about the screen.** The claim is checkable
only against the screen, and the screen is a picture while the registry is a
list of names. §99 taught that the registry and the shell can disagree about
*one* name; this is the same disagreement at the scale of the whole panel.

**A name that promises nothing is worse than no name.** A missing name says
"there is no such widget". A name that promises a row that is not there says
"there is one, over there" — and the agent goes looking.

**Coverage is a set, and a set that is asked about in one direction is not
covered.** Every question anybody asked about the registry was about the
toolbar. The buttons in the node list had never been on anybody's list.

## 101. A timing gate that failed when the machine was busy

§98 made the automation registry's duplicate check a lookup and wrote a test
to prove it: the second batch of five thousand names must not cost twice as
much as the first. The test's own comment said *"The test is **relative**, so
it does not depend on how fast the machine is"*.

It did not depend on how fast the machine was. It depended on how **busy** the
machine was, and it failed about four times in ten when it was.

### The measurement

Sixteen processes spinning, and the same correct code measured twice:

| Round | First batch | Second batch | Ratio |
|---|---|---|---|
| 0 | 51.4 ms | 2.0 ms | 0.04 |
| 1 | 155.9 ms | 1.8 ms | 0.01 |
| 3 | 1.5 ms | 106.2 ms | **72.0** |
| 4 | 1.5 ms | 87.4 ms | **59.4** |
| 5 | 103.7 ms | 1.9 ms | 0.02 |

Two rounds of the *same* code, differing by a factor of seven thousand. The
first batch is the one that got the CPU, or it did not; the second batch is
the one that did.

So the test, run ten times under that load, on code that is **correct**:

```
1: the second batch took 42.4x as long as the first   FAILED
2: ok      3: ok      4: ok      5: ok      6: ok
7: the second batch took  4.6x as long as the first   FAILED
8: ok      9: the second batch took  3.3x as long as the first   FAILED
10: the second batch took 44.2x as long as the first  FAILED

OLD TEST FAILURES: 4 / 10
```

And the failure it reported was **the mutation's own message**: *"the
registry is scanning what it has already published"*. A gate that reports the
wrong reason is worse than a gate that fails to run: it teaches people that
the message means something.

### Why nothing noticed

**The machine was quiet.** A gate that only misbehaves when something else
is running is a gate that is right on the developer's desk and wrong in CI,
where a build, a test binary and the runner all compete for the same cores.

**The comment promised the wrong thing.** "Does not depend on how fast the
machine is" is true of a *relative* measurement — and the ratio did not depend
on the machine's speed. It depended on the machine's **load**, which the
comment never mentioned and which the measurement never controlled for. The
sentence was reassuring and wrong, which is the §64 shape: a sentence about
the property the code did not have.

**It failed once, and was put down to the machine.** The first §100 run of
`cargo xtask ci` failed on exactly this test, on a tree where nothing had
changed. Re-running it passed. The comfortable reading is "the machine was
busy" — which is *true*, and is also the bug.

### The fix

**Measure the least disturbed round, not the last one.** The test now runs
eight rounds and keeps the **fastest** measurement of each side. A preempted
round says nothing about the code; the fastest round is the one that was
least disturbed.

The same ten runs under the same load, with the fix:

```
NEW TEST FAILURES: 0 / 10
```

And the mutation is still caught — under the same load, with the scan put
back:

```
the second batch of 5000 widgets took 3.8x as long as the first   FAILED
the second batch of 5000 widgets took 2.9x as long as the first   FAILED
the second batch of 5000 widgets took 3.6x as long as the first   FAILED
```

The quadratic implementation's *fastest* round is still three times its
first batch, because the quadratic work is the thing being measured and it
dominates any scheduling noise. The margin is the same; the noise is gone.

### What now holds it

| Property | Held by |
|---|---|
| A timing gate does not fail when the machine is busy | `publishing_a_widget_does_not_cost_more_than_the_widgets_before_it` |

and the §98 mutation — the scan put back — still fails it, under load, every
time.

There is no new mutation here, and the reason is worth saying: **a mutation
that breaks the *test* rather than the production code is not a mutation.**
The §98 case already covers the rule; what was wrong was the instrument, not
the rule.

### The lesson

**A measurement needs a rule for what to keep.** The test kept one number,
and one number from a preempted round is noise. The rule that works here is
the one every benchmark already knows: **keep the fastest run**, because the
fastest run is the one that was not interrupted.

**"Does not depend on how fast the machine is" is not "does not depend on the
machine".** The relative measurement was right about speed and blind to load.
A sentence about what a measurement does *not* depend on is a claim about the
world, and it needs the same care as the measurement.

**A gate that fails at random is a gate that trains people to ignore it.**
The failure mode here was not a missed mutation — it was a *false* one, with
a message that named a bug that was not there. The cost of that is not the
false failure; it is that the next time the gate says the same thing, nobody
will believe it.

**This tree has exactly one timing-sensitive gate, and now it is not
sensitive.** The kernel's `measure` and the CI's own clock report times; they
do not assert on them.

## 102. A mutation that did not compile was counted as a caught mutation

The mutation script is this tree's evidence that its gates work. Its report
says *N of M caught*, in green, and it says nothing about **why** a case was
caught.

Five of them were caught by a **build failure**.

### The measurement

`expect_catch` asked one question: did the gate exit non-zero? Yes → "caught".

A tree that does not compile has no test to run. The gate exits non-zero
because *cargo* failed, not because the rule was broken — so the case says
"caught" while the rule it names was never tested at all.

Measured, by teaching the script a third outcome, **BROKE**:

```
5 of 194 mutation(s) broke the build, so no gate ran:
   • io: a bounded read is not bounded
   • ci: the tests and wasm phases vanish from the run
   • ci: a skipped phase is counted as one that ran
   • wasm: a cfg(wasm32) branch stopped compiling
   • engine: a documented parameter is never fired with a hostile value
```

They are not all the same kind of thing, and one of them is not a defect at
all:

**"a bounded read is not bounded"** removed `reader.take(max as u64 + 1)`.
`take` consumes its receiver **by value**, so without it the binding `reader`
is never mutated — and `reader.read_to_string(&mut out)` needs `&mut reader`.
The tree did not compile. The case was "caught" because the crate would not
build.

**"the tests and wasm phases vanish from the run"** deleted two elements of
`PHASES`, which leaves the `tests` and `wasm::run` functions unreferenced.
`cargo xtask ci` runs clippy with `-D warnings`, so the dead functions are
refused and the ci gate fails — at the **clippy** phase, before the test that
pins the phase list ever runs.

**"a skipped phase is counted as one that ran"** had the same shape:
`let mut skipped` is never mutated, so `unused_mut` is refused, and the ci
gate fails before it reaches the tests. And this one is the worst of the
five: **the rule it names was not protected by anything at all.** The xtask's
tests check `summary()`, which takes the two lists as *arguments* — so no
test could see what the *loop* did with a skipped phase. The case has been
reporting "caught" since §2 — the case is from §2's own work — and what it was
catching was its own broken build.

**"a cfg(wasm32) branch stopped compiling"** is *supposed* to break the
build: the gate is `cargo xtask wasm`, and a wasm32 branch that no longer
compiles is exactly what that gate exists to notice. Here a broken build
**is** the catch.

**"a documented parameter is never fired with a hostile value"** replaced the
head of a chain with two statements, which left the chain's type
unconstrained — so the tree needed a type annotation it did not have.

### Why nothing noticed

**The question was the exit code.** One question, one bit: did the gate fail?
A build failure and a rule failure are the same bit, and the bit is all the
script looked at.

**"Caught" was printed in green.** The report's own colours said the rule was
protected. Five cases in green that protected nothing — and the report's
closing line counted them.

**A mutation that breaks the build is easy to write by accident.** Removing
a line is the easiest way to mutate anything, and whether it compiles depends
on the line: `reader.take(...)` consumes its receiver, `let mut` wants a
mutation, a chain wants an annotation.

### The fix

**Three outcomes, not two.** caught, MISSED, and **BROKE** — and a BROKE case
fails the run like a MISSED one, with a message that says what to do: *"A
mutation that does not compile tests nothing."*

**The exception is explicit.** A gate whose subject *is* a compilation says
so — `expect_catch "…" "cargo xtask wasm" "yes"` — so the one case that must
break the build still counts, and the reason is written down at the call
site rather than guessed at.

**Five mutations repaired, so that each tests its rule:**

| Case | Repaired to |
|---|---|
| a bounded read is not bounded | `take(u64::MAX)` — the limit is gone, the code compiles |
| the tests and wasm phases vanish | gated by the **test** that pins the phase list, not by the whole `ci` run, which refuses the dead code first |
| a skipped phase is counted as one that ran | the rule is now a **function** (`record`) with its own test, and the mutation points at it |
| a documented parameter is never fired | the chain keeps its type |
| a cfg(wasm32) branch stopped compiling | unchanged: the compile failure *is* the catch, and it now says so |

**And the self-test has a third half**: a gate that prints what cargo prints
for a compilation failure must be classified BROKE, not caught. The self-test
is the negative of the whole exercise, and this is the third way it can be
wrong.

### What now holds it

| Property | Held by |
|---|---|
| A mutation that does not compile is not a catch | the mutation script's third outcome, and its self-test's third half |
| A skipped phase is not counted as one that ran | `a_phase_that_could_not_run_is_not_counted_as_one_that_ran` |
| The phase list is the documented one | `the_phases_are_the_documented_ones_and_come_first` |

and five mutations repaired so that each breaks **production code** rather
than the build.

### The lesson

**A gate that failed is not the same as a gate that failed for the right
reason.** §101 was the same shape in the other direction: a gate that failed
when it should not have. Both are the same mistake — the harness asked a
question that could not tell the two situations apart.

**A mutation must break the code, not the build.** The build is not the
rule; the build is the thing that runs the rule. A mutation that stops the
rule from running has not tested it, and it is not a catch.

**"Caught" is a claim, and green is a colour.** The report said "caught" and
printed it in green, and for five cases the sentence was false. A report that
says *why* it caught something is worth more than a report that says only
that it did.

**A rule with no test of its own is protected by nothing.** The skipped-phase
case had a gate, a mutation and a green tick, and the rule was untested —
because the mutation that broke it had broken the build instead, and the
build failing is exactly what a gate notices. The rule was moved into a
function of its own so that it could be pinned, and the pinning is now a
test with a name.

**And the harness's own rules have to be written down where they are
obeyed.** The script's header warned against editing a file in `FILES` while
it runs. It did not warn against editing **the script**, which is worse: bash
reads a script incrementally, so an edit in the middle leaves the shell
reading at a byte offset that is no longer a command, and the run stops with
*"THE MUTATION DID NOT APPLY, SO THE RUN IS VOID"*. The run says so — it is
honest — but the run is gone, and it is gone after the twenty minutes it
took to get there. It happened, in this section's own measurement. The
header says it now.

## 103. The engine walks the whole document four times a command, and the tree had measured one of them

§52 measured the engine's per-command overhead once, and wrote a sentence about
it:

> Measured on the 19 MiB document: the clone is **205 µs**, and twenty cheap
> commands through the engine take 38 ms — so about 1.9 ms of engine overhead
> per command. **That is the price of the restore guarantee**, and it is the
> right price.

The clone is 205 µs. The overhead is 1.9 ms. The sentence calls the whole of it
the price of the restore guarantee — and the restore guarantee is the clone,
which is a **tenth** of it.

The other three quarters were never measured, because nobody thought to ask
what they were.

### The measurement

A document at the model's limits — one node, 32 properties, 10 000 keyframes on
each, **30 730 191** bytes estimated, inside `MAX_DOCUMENT_BYTES` (32 MiB) and
inside every count the model has. Measured on the fastest of five rounds each
(§101's rule), on an idle machine:

| Step | Cost | What it is for |
|---|---|---|
| the snapshot | **3.1 ms** | the document is copied, so a failed or panicking command can put it back |
| the comparison | **1.9 ms** | has the command changed anything? (§93, §96) |
| the validation | **5.3 ms** | is the document still one the file reader would accept? (§50) |
| the estimate | **1.2 ms** | how many bytes the undo stack is holding (§52) |
| **a read-only command** | **6.7 ms** | the first two, plus the command's own work |
| **a mutating command** | **23.8 ms** | all four, and a re-evaluation of the plan |

Four walks over the whole document, for every command, on a document the model
accepts. The mutating command's 23.8 ms is **more than a frame** at sixty
frames a second.

The four are not the same kind of thing, and that matters for what could be
done about them:

- The **snapshot** is the restore guarantee. It is the price of being able to
  put the design back, and copy-on-write would be a redesign of the model's
  ownership rules — item 7 in the list below, and still open.
- The **comparison** and the **validation** are *backstops*: they catch a
  command that lies about what it did (§93, §96) and a command that leaves a
  document the reader would refuse (§50). They are the reason the tree can
  say "a command cannot leave a broken design".
- The **estimate** is the undo stack's byte accounting (§52). It is only
  expensive on a document big enough for the budget to matter, and on such a
  document it is what keeps the budget honest.

### Why nothing noticed

**The one measurement that existed was about the clone.** §52 was looking at
the undo stack, and the clone was the cost it could see from there. The
comparison and the validation were added *after* §52 — §93, §96 and §50 — and
nobody went back to the sentence that said what a command costs.

**The number was right and the attribution was wrong.** "About 1.9 ms of engine
overhead per command" was true. "That is the price of the restore guarantee"
was not: the restore guarantee is 205 µs of it. A sentence can be half right
in a way that is worse than being wrong, because the wrong half is the one
that gets quoted.

**A cost that is invisible at the sizes people use is invisible.** A document
with ten nodes and a hundred properties validates in microseconds. The four
walks only show up at the size the model *allows*, which is the same shape as
§98: the numbers a small document hides are the numbers that matter.

### The fix

**The sentence is corrected, and the four walks are named.** §52's "what is
still open" now says what a command costs and says which part of it is the
restore guarantee.

**The numbers are in the code**, on `Session::execute` — the place a reader
who is about to make a command slower will actually look. §52's lesson was
that a comment saying a thing was profiled is a promise to the next reader,
and a promise has to be kept; this one is kept by measurement and not by
confidence.

**The open question is recorded rather than answered.** The two backstops cost
7.2 ms of the 23.8 ms, and they are safety. Whether a document at the model's
byte limit should be *editable* — the byte limit is a storage limit, and
nothing in the tree says what an interaction limit would be — is a decision,
and it is item 18 below.

### What now holds it

| Property | Held by |
|---|---|
| What a command costs is measured, and says which part of it is the restore | `Session::execute`'s doc comment, and §52's numbers |

No new mutation: the rules this section is about — the snapshot, the
comparison, the validation and the budget — are already mutation-tested, and
this section measures them rather than changing them.

### The lesson

**A measurement of one part of a cost is not a measurement of the cost.** The
clone was measured because it was the thing being looked at. Three other walks
sat beside it, in the same function, and were not.

**A sentence about a cost can be half right in the worst way.** "1.9 ms per
command" was true; "that is the price of the restore guarantee" pointed
everybody at the cheapest seventh of it, and the sentence is what a reader
would quote.

**The cost of a command is not the cost of the work it does.** The work is a
property write. The cost is four passes over the design, and the design is as
large as the model allows.

**And a limit is a promise about more than storage.** `MAX_DOCUMENT_BYTES` says
a design may be 32 MiB. It does not say what that design costs to edit, and at
that size a single command costs more than a frame.

## 104. The control channel published the names of the widgets and not where they were

The registry exists so that an agent can find a widget. §99 gave the level bar
a name of its own; §100 named the buttons the shell drew and had never
published. Both of those were about **names**.

None of them was about **positions** — and a name with no position behind it is
a name an agent cannot act on.

### The measurement

The running app, over the control channel:

```
ui.widgets keys: ['widgets']
ui.widgets widgets[:6]: ['node.1.level', 'node.2.level', 'node.3.level',
                         'nodes.row.1', 'nodes.row.2', 'nodes.row.3']
ui.widgets has rectangles? False
ui.click by name -> False | `x` and `y` are required, and must be finite numbers
ui.click by point -> True  | nothing is at that point
```

So the channel answered "these thirty-four widgets are on screen" and nothing
else. `ui.click` wanted pixels, and the pixels were the one thing the channel
would not say. An agent that had been told "the delete button of row 3 is
`nodes.row.3.del`" — which is exactly what §100 made true — had no way to find
out where that button was, and so no way to click it.

The registry knew: `Automation::rect(name)` has answered with a widget's
rectangle since the registry was written. Nothing published it.

### Why nothing noticed

**The names were the interesting half, and they were published.** §99 and §100
were both about the *set* of names — a name that collides, a name that is
missing. The position of a name was nobody's question.

**A click by coordinates looks like it works.** `ui.click {"x": 120, "y": 240}`
answers "nothing is at that point" or "…is at that point", and a caller who
guessed well is satisfied. The caller who guessed *badly* gets an answer that
reads like a fact about the screen.

**The PNG was the fallback, and it is not a coordinate system.** An agent can
look at the rendered picture and see the buttons, but the picture is pixels, and
the agent has to guess where in it a button is, and the click's coordinates are
in the frame's pixels — the same pixels, but only if the agent happened to
render at the size the click assumes.

### The fix

**A click takes a name.** `ui.click {"name": "nodes.row.1.del"}` finds the
widget and clicks the middle of it. The reply carries the point it landed on,
so the same call answers "where is this widget?" as well as "what is at this
point?" — the click was always a report (§17), and now the report can be asked
for by name.

**`ui.widgets` says where the widgets are.** The reply carries the names, a
`rects` map from each name to `[x, y, width, height]`, and the `width` and
`height` those pixels are in — so an agent can build its own map of the screen
and aim at anything on it.

**Giving both is refused**, like a method and an action that disagree: one of
the two things the caller said would have to be ignored, and which one is a
guess.

**A widget that is on screen but disabled is not one that is not there.** The
refusal says so, rather than "there is no widget named X" for a button that is
right there.

### What now holds it

| Property | Held by |
|---|---|
| A click by name lands on that widget | `a_click_by_name_lands_on_that_widget` |
| A name that is not on screen is refused | `a_click_by_a_name_that_is_not_there_is_refused` |
| A disabled widget is not the same as one that is not there | `a_disabled_widget_is_not_the_same_as_one_that_is_not_there` |
| A click with neither a name nor a point is refused | `a_click_with_neither_a_name_nor_a_point_is_refused` |
| A click with both a name and a point is refused | `a_click_with_both_a_name_and_a_point_is_refused` |
| The widgets reply says where the widgets are | `the_widgets_reply_says_where_the_widgets_are` |

and two mutations in `scripts/mutation-test-gates.sh`: a click by name lands
somewhere else, and the published rectangles have no width.

### The lesson

**A name is not an address.** The registry answers "what is called X?" and the
channel published exactly that — and a caller holding a name still could not
point at it. §99 and §100 fixed names; this fixes the other half.

**"Where is it?" and "what is it?" are different questions, and a registry
that only answers the second is half a registry.** The rectangle was there the
whole time, in the same struct, one method away.

**A capability that needs a guess is not a capability.** `ui.click` took
pixels, and the pixels were not published, so the only way to use it was to
guess — and a guess that happens to be right is indistinguishable from a
click that worked.

---

## 105. The window drew a fresh shell every frame, and threw the shell's memory away with it

§99, §100 and §104 were all about the **registry**: the names of the widgets and
where they were. Every one of them was about a thing the shell *publishes*.

None of them was about the thing the shell *remembers*.

### The measurement

The desktop app's `Application` held two things: the session, and the slot where
the egui context is published. Its `update` called `craftkit_ui::frame`, which
built a **new shell** for the frame, drew it, and handed back the shell's state:

```rust
let _ = craftkit_ui::frame(ctx, session, &tokens);
```

The tuple was thrown away. So the shell's state — the selection — lived for the
length of one frame and was dropped at the end of it.

The properties panel asks for the selection and, when there is none, draws
*"Select a node."* and returns. The click on a node row set the selection
*during* the frame; the next frame had a new shell, so the selection was gone
and the panel said "Select a node." again. Measured:

```
selected after the click:        Some(NodeId(1))
selected in the next frame:      None
selected in the carried frame:   Some(NodeId(1))
```

The two pictures are the app's own rendering, before and after, at 1200×500:

* **before** — `/tmp/probe105-before.png`: the right-hand panel reads
  *"Properties / Select a node."* This is what every frame of the window said,
  no matter what the person clicked.
* **after** — `/tmp/probe105-after.png`: the row *Level* is highlighted in the
  node list, and the panel shows *"Level · source"*, the *enabled* checkbox,
  the *amount* slider, the *label* field and the "key at the playhead" field.

**The browser had the same bug with a different face.** The web app *had* a
state field, documented as *"Kept between frames by the host"*, and it
overwrote it with the fresh shell's state every frame:

```rust
let (state, _automation) = craftkit_ui::frame(ctx, &mut self.session, &tokens);
self.state = state;
```

The field was kept. Its contents were replaced by the same emptiness, once a
frame, for as long as the app ran.

**Two text fields had the same disease and no field at all.** The "key at the
playhead" path and value were **locals**, rebuilt every frame, so every
keystroke was discarded — and the value field was drawn only on the frame the
path changed, which meant it was gone before anything could be typed into it.

### Why nothing noticed

**The headless path kept the shell.** `raster::render` draws both of its
frames against one shell, so the headless picture could never show a state
that did not survive. The bug was in the hosts, and the hosts were the two
things the tests did not use.

**Every test drew one frame.** The shell's tests draw a frame and look at the
registry. A single frame cannot show a thing that has to survive to the next
one — the defect is *invisible* to a one-frame observer by construction.

**The state was an output, and an output may be dropped.** `frame` returned
`(ShellState, Automation)`, so the signature *allowed* a host to throw the
state away, and the desktop app did exactly that. `let _ = …` is the loudest
way of saying "I do not want this", and the type system has nothing to say
about it: dropping a return value is not a mistake that can be seen.

### The fix

**The frame takes the state as well as giving it back.** The signature is now

```rust
pub fn frame(ctx: &Context, session: &mut Session, tokens: &Tokens, state: &mut ShellState) -> Automation
```

The state goes in and comes back through the same reference. A host that
forgets cannot compile, because there is no longer a return value to forget:
the only way to draw a frame is to say where the shell's memory lives. The
desktop app holds a `ShellState`; the web app hands its own field in.

**The two text fields are in the state**, and the value field is drawn whenever
there is a path to put a value on — not only on the frame the path changed.

### What now holds it

| Property | Held by |
|---|---|
| A selection made in one frame is there in the next | `a_selection_made_in_one_frame_is_there_in_the_next` |
| A path typed into the key field is still there in the next frame | `a_path_typed_into_the_key_field_is_still_there_in_the_next_frame` |

and four mutations in `scripts/mutation-test-gates.sh`: the frame forgets the
state it was given; the frame does not give the state back; the key path field
keeps its text in a local; the key value field is only there on the frame the
path changed.

The host half needs no mutation, and that is the point of the fix: **the
desktop app cannot drop the state, because there is nothing to drop.** The
compiler holds that half now, which is stronger than a test would have been.

### The lesson

**A frame's memory has to be carried across the frame boundary by something
that outlives it.** The shell is built for the frame and thrown away with it —
that is what the borrow checker is for. But the shell's *state* is not the
shell's; it is the app's, and the app has to hold it.

**An output that a caller may drop is a contract nobody signed.** The old
signature said "here is your state, keep it if you like", and the reader of
that signature could not tell that keeping it was the whole point. Making the
state an *input* turns the mistake from a silent omission into a compile
error — and a compile error is the only kind of gate that cannot be forgotten.

**A one-frame test cannot see a defect that is about the frame boundary.** Not
"the test was not thorough enough": the defect is *by construction* invisible
to it. The two tests above draw a frame, then draw another and look at what
survived.

---

## 106. The control channel drew the shell with nothing selected, so half the window was invisible

§99, §100 and §104 were about the **registry** — the names of the widgets and
where they were. §105 was about the shell's **memory**: the selection the
window threw away with the frame.

This is the other half of §105: the memory the window now keeps, the channel
could not see.

### The measurement

On the running app, over the control channel:

```
ui.widgets                 -> 34 names
  property editors: []
ui.click {"name": "node.1.prop.amount"}
  -> there is no widget named `node.1.prop.amount` on screen
ui.click {"name": "properties.key.path"}
  -> there is no widget named `properties.key.path` on screen
```

Thirty-four names: the toolbar's twenty-four buttons, three node rows, three
level bars, three mute buttons, three delete buttons and the playhead. **Not
one property editor** — and not the key fields either, which are the names
§100 invented, with a test of their own.

The reason is one line. `raster::render` built its shell with
`ShellState::default()`, and the default has no selection. With no selection
the properties panel draws *"Select a node."* and returns — so it publishes
nothing at all. The panel was not on the screen the channel described.

And that screen was not the window's screen either. Since §105 the window
keeps its selection, so the window can be showing four editors while the
channel says there are none.

### Why nothing noticed

**The names existed, and the names were the point.** §99 and §100 spent two
sections making sure every widget had a name that promised a widget that was
really there. That rule was checked in the shell's own frame — where the
properties panel is empty unless something is selected. The check passed, and
the channel never saw a frame with a selection in it.

**A render is a picture, and a picture of nothing looks right.** Thirty-four
names is a plausible list of widgets. Nothing said "this is the screen of an
application nobody has touched".

**§105 made the window right and the channel wrong.** Before §105 the window's
state was always the default, so the channel's empty screen and the window's
empty screen were the *same* screen and nobody could tell them apart. After
§105 the window keeps a selection and the two screens came apart. The
divergence was *caused* by the fix that came before it — which is why it
needed a measurement rather than a reading.

### The fix

**A render of the shell is a render of a shell *in a state*.** `raster::render`,
`click` and `click_name` take the shell's state, and it is not an internal
default any more: a caller that does not say which state it means is choosing
the empty one, and now has to say so. `Shell::with_state` is the way to say it.

**The channel and the window share one state.** The app holds it in an
`Arc<Mutex<ShellState>>`; the frame loop writes it back and the channel's
handler reads it. So the screen the channel describes is the screen the window
is showing. Both sides take the **session** before the **state**, so neither
can be waiting for the other.

**An agent can ask about a screen with a node selected.** The `ui` actions take
a `selected` node id; the reply says which node it drew, and a node that is
not in the document is refused rather than answered with an empty panel.

**Selecting a node is not a command.** It changes nothing in the document, so
there is nothing to undo and nothing to log — which is why it is a parameter of
a read rather than a new method. Whether it *should* be a command is the open
question in "Suggested order", item 19.

Measured on the running app, after the fix:

```
ui.widgets                                 -> 34 names, no property editor
ui.widgets {"selected": 1}                 -> 39 names
  property editors: node.1.prop.amount, node.1.prop.enabled,
                    node.1.prop.label, node.1.prop.tint
  key fields: properties.key.path
ui.click {"name": "node.1.prop.amount", "selected": 1}
  -> `node.1.prop.amount` is at that point
ui.widgets {"selected": 9999}               -> refused: there is no node with id 9999
```

The pictures are the channel's own render, before and after:
`/tmp/probe106-before.png` (the panel says *"Select a node."*) and
`/tmp/probe106-after.png` (the panel shows *"Level · source"*, the *enabled*
checkbox, the *amount* slider, the *label* field and the "key at the playhead"
field).

### What now holds it

| Property | Held by |
|---|---|
| The state decides what is on the screen | `the_state_decides_what_is_on_the_screen` |
| A property editor can be clicked by name when a node is selected | `a_property_editor_can_be_clicked_by_name_when_a_node_is_selected` |
| The property editors are on the screen when a node is selected | `the_property_editors_are_on_the_screen_when_a_node_is_selected` |
| The widgets reply is the screen the window is showing | `the_widgets_reply_is_the_screen_the_window_is_showing` |
| The channel reports the window's screen over the socket | `the_channel_reports_the_windows_screen_over_the_socket` |
| A selected node that is not there is refused | `a_selected_node_that_is_not_there_is_refused` |
| A `selected` that is not a node id is refused | `a_selected_that_is_not_a_node_id_is_refused` |

and five mutations in `scripts/mutation-test-gates.sh`: the render draws the
shell with nothing selected; a click by name looks for the widget on the empty
screen; the channel reports a screen of its own; the selected node is ignored;
a selected node that is not there is accepted.

The socket test is the one that covers the **wiring**: the others call the
handler directly, and the thing that can go wrong is a handler that is handed
a state of its own instead of the shared one.

### The lesson

**A tool that shows a screen must show the screen, not a screen it made up.**
The channel's answer to "what is on screen?" was a picture of an application
nobody had touched, and it was indistinguishable from a picture of the real
one — because the real one *was* that, until §105 made it something else.

**The state is the difference between a picture and a lie.** Thirty-four names
is not a wrong answer; it is the right answer to a question nobody asked.

**A fix can create the next gap.** §105 made the window right. The channel's
answer was wrong in a *new* way afterwards, and nothing in the tree would have
said so: both sides still agreed with themselves, and they had stopped
agreeing with each other.

---

## 107. One question, two answers: a greyed-out button was on the screen and could not be pressed

§104 made `ui.click` take a name *or* a point. It said: "a name is a name, a
point is a point, and the two spellings must not disagree". This is the
section where they did.

### The measurement

On the running app, over the control channel, with nothing to undo:

```
ui.click {"name": "toolbar.history.undo"}
  -> ok: false | `toolbar.history.undo` is on screen but disabled, so there is
                   nothing to click

ui.click {"x": 1015.1, "y": 32.0}          # the middle of that same button
  -> ok: true  | `toolbar.history.undo` is at that point; a click reports where
                   it landed and does not activate anything
```

**The same widget, the same moment, two answers.** Asked by name, the click was
refused: the button is on the screen and cannot be clicked. Asked by point —
at the button's own centre — the click was accepted and the button was
reported as the thing that was there.

And the reply that lists the screen did not say which of its widgets were
dead:

```
ui.widgets -> 34 names, 34 rectangles
ui.click {"name": "toolbar.history.undo"} -> refused
```

The Undo button had a rectangle in the map, a name in the list, and no way to
be pressed. The only way to find out was to try.

### Why nothing noticed

**The registry knew and the accessors dropped it.** `Automation` has carried
`enabled` on every entry since it was written, and `Widget::Found` carries it
too. But `Automation::at()` answered with the **name** and threw the flag
away, and `Automation::rects()` answered with the **rectangle** and threw it
away. Two of the three ways of asking about the registry could not say
whether a widget is live.

**The two clicks were written a week apart.** §104's by-name refusal was
written to answer "what if the caller named a disabled widget?"; the by-point
path answered "what is at this point?" and was written before the registry
knew about `enabled` at all. Neither was wrong where it stood; the wrongness
was in the *combination*, and nothing had ever asked the combination.

**A greyed-out button looks like a button.** In the picture, in the list of
names, in the map of rectangles — it is there, and it is the only kind of
button there is in the picture.

### The fix

**One rule, in one place.** The rule that a disabled widget cannot be clicked
now lives in the **click**, which is the thing that clicks. §104's separate
refusal in the by-name path is gone — not because it was wrong, but because
two spellings of one rule is how they come to disagree, and they did.

**The registry's answers keep the flag.** `Automation::at()` answers with a
`Hit` — the name *and* whether it can be clicked — and `Automation::disabled_names()`
lists the widgets that are on the screen and dead.

**The reply says which are dead.** `ui.widgets` and `ui.render` carry a
`disabled` list beside the names and the rectangles, so an agent does not have
to find out by trying.

**Both ways of asking refuse the same thing, in the same words.** A click by
name and a click by point on a disabled widget now give the same message —
and a widget that is *not there at all* is still a different refusal, because
"it is there but dead" and "it is not there" are different facts (§104).

Measured on the running app, after the fix:

```
ui.widgets -> disabled: ['toolbar.history.redo', 'toolbar.history.undo']
ui.click {"name": "toolbar.history.undo"} -> refused | `toolbar.history.undo`
    is on screen but disabled, so there is nothing to click
ui.click {"x": 1015.1, "y": 32.0}         -> refused | the same words
ui.click {"name": "nodes.row.1"}           -> ok | `nodes.row.1` is at that point
ui.click {"name": "nodes.row.9999"}        -> refused | there is no widget named
    `nodes.row.9999` on screen
```

### What now holds it

| Property | Held by |
|---|---|
| A click on a disabled widget is refused | `a_click_on_a_disabled_widget_is_refused` |
| A click on a disabled widget is refused however it is asked for | `a_click_on_a_disabled_widget_is_refused_how_it_is_asked_for` |
| The widgets reply says which widgets cannot be clicked | `the_widgets_reply_says_which_widgets_cannot_be_clicked` |
| The answer at a point says whether it can be clicked | `the_answer_at_a_point_says_whether_it_can_be_clicked` |
| Only the widgets that cannot be clicked are in the dead list | `only_the_widgets_that_cannot_be_clicked_are_in_the_dead_list` |
| A widget that is not there is refused differently from one that is dead | `a_disabled_widget_is_not_the_same_as_one_that_is_not_there` |

and four mutations in `scripts/mutation-test-gates.sh`: a click on a disabled
widget is not refused; the answer at a point does not say whether it can be
clicked; the dead list is the list of the live ones; a widget that is not
there is refused as though it were disabled.

### The lesson

**One question, one answer — however it is asked.** §104 made a name and a
point two spellings of the same act, and this is what that costs: two
spellings, two code paths, two answers, and a bug that only exists in the
gap between them.

**The rule was in the registry the whole time.** Not "the registry should
have known" — it knew, and three of its four accessors could not say so. A
fact that is stored but cannot be *asked for* is a fact that the tree has,
and its users do not.

**A disabled control is not a missing control, and both are not a control that
works.** Three different states, three different answers, and the tree has to
be able to tell them apart — which is why the refusal says *which* one it is.

**A restored file with an old timestamp is a stale build.** This section was
measured wrong once: a scratch script that restored a mutated file with
`cp -a` — which keeps the backup's old timestamp — left cargo believing the
crate was unchanged, so the next build kept the **mutated** code and the
measurement said the fix had inverted the list. The project's own mutation
script has known this for a while: its `restore` **touches** every file it
puts back, for exactly this reason. A measurement of a tree that was put back
by hand is a measurement of the wrong tree unless the timestamps say so too.

**The rule, measured.** Cargo rebuilds a file whose timestamp is **newer** than
the one it recorded, and it does **not** rebuild a file whose timestamp is
**older** — even when the content changed. Measured on `craftkit-core`: put
the file's timestamp a minute into the past, change its content, build, and
cargo says *"Finished in 0.05s"* and keeps the old code. So the danger is
not "the timestamp changed" but "the timestamp went **backwards**", which is
exactly what `cp -a` does.

**And the guess that seemed obvious was wrong.** The first guess was that
cargo compares timestamps at second granularity, so a file changed inside the
same second as the build would be missed. Measured: a file whose timestamp
moved forward by 0.4 s — the same second — **is** rebuilt. Cargo compares
with sub-second precision, and the rule is the *direction* of the timestamp,
not its resolution. A negative result, and the reason it is written down is
that it looked measured until it was measured.

---

## 108. The frame rate was on the screen and nowhere in the API — and the only way to ask was to change it

### The measurement

On the running app, over the control channel:

```
session.state -> {"at": 0, "can_redo": false, "can_undo": false, "dirty": false,
                  "name": "Sample plan", "nodes": [...], "plan": {...}}
```

**No frame rate. No range. No comment.** The state has the document's *name*
and not its other parameters.

Nor is there any other read:

```
plan.evaluate   -> {"at": 0, "diagnostics": {...}, "evaluated": 3, "reused": 0, "nodes": [...]}
session.stats    -> {"commands": 0, "failures": 0, "kernel_recoveries": 0, "nodes": 3, "undo_depth": 0}
diagnostics.list -> {"dropped": 0, "items": []}
doc.range {}     -> refused: give at least one of `start`, `end` or `fps`
```

And the window says, in its own header, *"Sample plan 24 fps · 5.00s"*.

So the rate was on the screen, in the model, and in the file — and in **no**
place an agent could ask. The only command that mentions it at all is
`doc.range`, which **sets** it. And setting it is not free:

```
doc.range {"fps": 30}   -> ok | range 0.000s..5.000s at 30 fps
session.state           -> dirty: true, can_undo: true
```

The agent asked for the rate and **rewrote the design**. The rate was not a
thing that could be read; it was a thing that could be *guessed*, and a wrong
guess cost the user their design's settings.

### Why nothing noticed

**Nothing in the tree reads it either.** The shell reads `meta.fps` for the
header and `meta.frame_rate()` for the timeline's frames (§115 changed the
second half of this sentence: the timeline had no scale, and the frame
number was on no screen at all); the CLI reads it for its own summary. Both
read the **model** directly. The API surfaces are the ones that answer
questions, and none of them was asked.

**`session.state` said "the whole state".** It was a summary, and the doc
sentence was the promise that made it look like one. This is the twenty-fifth
instance of the same shape, and the first one where the sentence was about the
*object* rather than about the code.

**Every parameter was write-only.** `doc.rename` sets the name and the state
carries it; `doc.range` sets the rate and the range and nothing carries them.
The pattern held for the one field that had been added to the state and not
for the three that had not.

### The fix

**The state carries the document's own parameters**: `fps`, `start` and `end`,
alongside the `name` it already had. The instants are raw ticks, the same
units the rest of the API speaks; `TICKS_PER_SECOND` is what turns them into
seconds, and it is documented in the README, the format document and the
time pattern.

**The state and the command agree.** `doc.range`'s reply carries `fps`,
`start` and `end`; the state carries the same three, from the same place, and
a test says they may not disagree.

**Reading costs nothing.** `session.state` is a read: it does not mark the
document dirty, it does not push an undo entry, and a test says so.

The document's **comment** is still not carried: nothing bounds its length,
and this reply is not capped, so a document with a very long comment would
make a very long reply. That is a limit, not an oversight, and it is the
comment's own gap to have.

Measured on the running app, after the fix:

```
session.state -> {"at": 0, "can_redo": false, "can_undo": false, "dirty": false,
                  "end": 1270080000000, "fps": 24, "name": "Sample plan",
                  "nodes": [...], "plan": {...}, "start": 0}
```

### What now holds it

| Property | Held by |
|---|---|
| The state carries the document's own parameters | `the_state_carries_the_document_s_own_parameters` |
| The state and the range command agree about the rate | `the_state_and_the_range_command_agree_about_the_rate` |
| Reading the rate does not change the document | `reading_the_rate_does_not_change_the_document` |

and three mutations in `scripts/mutation-test-gates.sh`: the state does not
carry the frame rate; the state does not carry the document's start; the state
does not carry the document's end.

The first of those three is the one that matters, and it is caught by the
*agreement* test rather than by the *contents* test: the contents test would
also be satisfied by a state that carried a constant, while the agreement test
compares the state against the command that writes the same fact.

### The lesson

**A parameter that can only be set cannot be read, and that is not the same as
a parameter nobody looks at.** The rate was not missing from the API; it was
present in the API *as a verb*, and an agent looking for a noun had to turn it
into one by writing to the design. The cost of the missing noun was paid in
the user's settings.

**The window is not a source of truth about the API.** The header said "24 fps"
and the API could not, and the header was right. Nothing about a screen
proves that the question behind it is answerable.

**"The whole state" is a promise, and it is the sort that is made by reading
the code rather than by counting the fields.** The state had nine fields and
the document had five parameters; four of them were in the file, in the model
and on the screen, and none of them in the state.

---

## 109. The session kept the last 256 runs and could only be asked how many had run

### The measurement

On the running app, over the control channel, after three commands — one that
added a node, one that renamed it, and one that was refused:

```
session.stats -> {"commands": 3, "failures": 1, "kernel_recoveries": 0,
                  "nodes": 4, "undo_depth": 2}
session.state -> {"at": 0, "can_redo": false, "can_undo": true, "dirty": true,
                  "end": ..., "fps": 24, "name": ..., "nodes": [...],
                  "plan": {...}, "start": 0}
```

**Three commands ran. One of them failed.** The session could tell an agent
that much — and not one of them:

- which commands ran,
- what they were called,
- what they said,
- and which of them had **failed**.

The `LogBook` was there the whole time: a bounded ring of 256 entries, each
with a sequence number, the command's id, whether it worked, and the message
the command produced. It is public (`Session::log`), it is what the crash
reporter replays, and it was not in the state, in `session.stats`, in the CLI,
or anywhere else an agent could ask.

### Why nothing noticed

**The count was there, and a count looks like an answer.** `session.stats`
reports "3 commands, 1 failure", and that is a *number about* the commands.
Nothing said "and here they are".

**The log's job had been decided for it.** §69 gave the book a bound and a
failure counter, and §47 gave the crash report a replay. Those are the two
things it was built for, and both of them are *exceptional* moments: a crash,
or a question asked afterwards. The ordinary case — an agent that has just run
four commands and wants to know what they were — was never a use case, so the
book was never asked.

**The shell shows the last message and the last message is not the log.** The
window's status line says what the last command said. That is one line, and
after the third command it says the third command's message. It is not a
history, and it was never mistaken for one.

### The fix

**The state carries the log**: the last 256 entries, each with its sequence
number, its command, whether it worked and its message. A refusal is in there
with `ok: false` and the reason it was refused — which is the entry an agent
most wants and could not have.

**The log and the counter agree.** A test says the state's log and
`session.stats`'s count may not disagree, because they are two answers about
one thing.

**Reading the log costs nothing.** It is a read: no dirty flag, no undo entry.

Measured on the running app, after the fix:

```
session.state -> {..., "log": [
  {"seq": 1, "command": "node.add",    "ok": true,  "message": "added source `In`"},
  {"seq": 2, "command": "node.rename", "ok": true,  "message": "renamed to `Renamed`"},
  {"seq": 3, "command": "node.add",    "ok": false, "message": "`node.add` rejected its
      parameters: `kind` `nonsense` is not a kind; expected one of source, filter,
      sink, cue, comment"}
]}
```

### What now holds it

| Property | Held by |
|---|---|
| The state carries the commands that ran | `the_state_carries_the_commands_that_ran` |
| The log and the command count agree | `the_log_and_the_command_count_agree` |

and three mutations in `scripts/mutation-test-gates.sh`: the state does not
carry the commands that ran; the log carries only the last command; the log
does not say whether the command worked.

### The lesson

**A count is not the thing it counts.** "3 commands, 1 failure" is a true
answer to a question nobody asked. The question was "what did this session
do?", and the answer was a number about it.

**The bound and the counter were the wrong two features.** §69 gave the book
its size and a count of the failures — the two things a *monitor* needs. The
book itself — the entries — were there and were never asked for, because
nothing asked for them.

**A record kept for the exceptional case is still a record.** The log was
built for the crash report, and it did that job well. The mistake was not
keeping it; it was not noticing that a thing kept for the crash is also the
answer to an ordinary question.

---

## 110. The design's note was in the file and in the model and nowhere else — and there was no way to write it

### The measurement

A design with a note in it — written by hand, with a `comment` in its meta:

```
$ craftkit-cli eval /tmp/commented.craftkit
left where it was
1 node(s), 12 frame(s) at 24 fps
{ "plan": {...}, "diagnostics": {...} }
```

The note is *in that file*. The file says:

> "This design came from a sketch: the taper is meant to be gentle."

and the evaluation of that file does not mention it. Nor did anything else:

| Where an agent or a person would look | What it said |
|---|---|
| `session.state` | `{at, can_redo, can_undo, dirty, end, fps, log, name, nodes, plan, start}` — no note |
| the window's header | `Commented  24 fps · 0.50s` — and nothing under it |
| `craftkit-cli eval` | the node count, the frame count and the frame rate — and nothing else |
| the command registry | 21 commands, none of which wrote a note |

`meta.comment` was in the model and in the file format, and the **only** place
in the whole tree that read it was `Document::estimated_bytes`, which counts
bytes. It was a field with a writer (the file reader) and no reader.

There was no way to *set* it either. `Document` had no setter for the note, no
command wrote one, and `Session` has no constructor that takes a document —
the app loads one with `Session::new` plus `load_document`, so a test could
only reach a document with a note by building one and loading it.

And the note had **no bound at all**. The model bounded the nodes, the
properties, the keys, the names, the paths and the whole document's bytes —
and the one field a person writes *prose* into was the one field with no limit
on it.

### Why nothing noticed

**A field nobody reads looks exactly like a field nobody needs.** `comment`
was in the file, and the file's schema is a list of fields the format has to
carry. Nothing in that list says the field is *used*.

**The name promised a comment node, not a comment.** `NodeKind::Comment`
exists — a node that says something in the timeline. So "comment" in this
project meant a *node*, and `meta.comment` was something else entirely: a
paragraph about the whole design, with no name of its own that anybody used.
The two things shared a word and nothing else, which is exactly the kind of
collision that makes a field look covered.

**Nothing in the window ever asked for it.** The window draws the design's
name, its frame rate and its range, because those are the three things a
person looks at when they open a design. The note is the fourth thing, and
nobody had ever opened a design *with* a note — because there was no way to
make one without editing the file by hand, and nobody edits a file by hand to
write a note they cannot read back.

**The bounds are written down, and the bounds are a list of counts.** Every
other limit is a *count* of things: nodes, properties, keys, names, path
segments. The note is not a count; it is a **size**, and the size was the one
thing the list of limits did not have.

### The fix

**The note has a bound: `MAX_COMMENT_BYTES` (4 096).** A bound is what makes
a note carryable — the state can hand it over whole, and a file with a longer
one is refused rather than half-kept. It is the model's bound, and the
validator enforces it, so a document that cannot be carried cannot be saved.

**The state carries the note**, whole:

```
session.state -> {..., "comment": "This design came from a sketch: the taper is
                            meant to be gentle.", ...}
```

**There is a command that writes it.** `doc.note` — the twenty-second
command — sets the note, and an empty text clears it:

```
engine.execute {"command": "doc.note", "params": {"text": "why the design is the way it is"}}
-> {"ok": true, "message": "note set (29 bytes)", "data": {"comment": "why the design is the way it is"}}
```

A command with **no** `text` is refused, because "write nothing" and "write
an empty note" are different things. A note that is already the note the
document has changes nothing, and says so (§93's rule), instead of tripping
§44's backstop.

**The window shows a glimpse of it.** The note is a paragraph and the panel
is not a page: the header shows the first 200 characters and marks the cut
with "…". The whole note is in the state.

**The CLI says it.** `craftkit-cli eval` prints the note, if the design has
one.

Measured on the running app, over the control channel, with the file above:

```
session.state -> {"comment": "This design came from a sketch: the taper is meant to be gentle.", ...}
engine.execute -> {"ok": true, "message": "note set (29 bytes)", "data": {"comment": "Set from the control channel."}}
session.state -> {"comment": "Set from the control channel.", "dirty": true, "can_undo": true, ...}
```

and in the window, under the design's name:

> **Commented**  24 fps · 0.50s
> This design came from a sketch: the taper is meant to be gentle.

### What now holds it

| Property | Held by |
|---|---|
| A note over the bound is refused | `a_note_over_the_byte_limit_is_refused` |
| A note command with no text is refused | `a_note_command_with_no_text_is_a_refusal` |
| A note command over the bound is refused | `a_note_over_the_limit_is_refused` |
| Writing the note the document already has changes nothing | `writing_the_same_note_changes_nothing` |
| The state carries the note | `the_state_carries_the_note` |
| The state and the document agree about the note | `the_state_and_the_document_agree_about_the_note` |
| The note is on the screen | `a_frame_with_a_note_is_not_the_frame_without_one` |
| The window's note is a glimpse, and says so | `the_note_preview_is_a_glimpse_of_the_note` |
| The CLI says what the design's note is | `eval_says_what_the_design_s_note_is` |

and eight mutations in `scripts/mutation-test-gates.sh`: the bound is one byte
too loose; the setter always claims it changed something; the state does not
carry the note; the command accepts a note over the limit; the command always
claims it changed something; the note is not drawn; the preview does not say
that it was cut; `eval` does not say what the design's note is.

### The lesson

**A field with a writer and no reader is a field nobody notices.** The file
format carried the note for as long as the format existed. Carrying it is
cheap and invisible; *reading* it is what nobody had to do.

**A bound is what makes a thing carryable.** The note could not be put in the
state until it had a size, because "whole" is not a thing you can promise
about an unbounded field. The bound came first, and the state followed.

**A comparison that differs for another reason proves nothing.** The first
version of the window's test compared a frame *with* a note against a frame
*without* one — and the mutation that stops the note being drawn was
**MISSED**: the two frames differed anyway, because the second was "unsaved"
in the status bar and the first was "saved". The test now compares two
frames whose notes have *different text* and nothing else, so the only thing
that can make them differ is the note. A comparison has to have exactly one
possible difference, or it is not a comparison.

**"Comment" was two things.** A node kind and a paragraph about the design,
sharing a word and nothing else. The node kind was the visible one, so the
paragraph looked covered.

---

## 111. The state could not say how deep the undo stack was, and four sentences said it could

### The measurement

On the running app, over the control channel, after three commands — two that
added a node and one that renamed one:

```
session.state -> {"at": 0, "can_redo": false, "can_undo": true, "comment": "",
                  "dirty": true, "end": 1270080000000, "fps": 24, "log": [...],
                  "name": "Sample plan", "nodes": [...], "plan": {...}, "start": 0}
session.stats -> {"commands": 3, "failures": 0, "kernel_recoveries": 0,
                  "nodes": 5, "undo_depth": 3}
```

**Two answers about one stack, in two objects, and the "whole state" had the
weaker one.** `session.state` — the object whose own doc comment calls it
*"the whole state, for an API surface that wants it in one object"* — said
`can_undo: true`. That is a boolean about a stack of **three**. The number
was in `session.stats`, which is about counters.

And the stack's **memory** was in neither: the undo stack has a 64 MiB
budget (§52), the tree measures its bytes on every push
(`History::bytes`), and no API surface could see them. A session at 63 MiB of
undo stack and a session at 6 KiB of undo stack answered the same questions
alike.

The documentation said otherwise, in four places:

| Where | What it said |
|---|---|
| `Session::snapshot_json`'s own doc | *"'The whole state' means the session's own: … the nodes, **the undo depth**, and the plan."* |
| the agent playbook | *"It answers with … the nodes, **the undo depth** and the plan."* |
| the checking skill | the same sentence |
| the MCP's `craftkit_status` description | *"The running session: … the playhead, **the undo depth**, and whether the document has unsaved changes."* |

Four sentences, one of them the function's own description of itself, and the
function never had the field. The sentence was written in §108 — the section
that added the frame rate and the range to the state and rewrote the
paragraph listing what is in it — and it was written from the *statistics*,
not from the state.

### Why nothing noticed

**A boolean about a stack and a number about a stack look like the same
answer.** `can_undo: true` says "there is something to undo", which is true,
and it is what the window needs to grey out a button. The depth is a
different question, and it was in the object about counters.

**The statistics were right, and being right is enough to stop the
question.** `session.stats` said `undo_depth: 3`, and that number was
correct. Nothing asked whether the *other* object agreed, because the other
object was not supposed to have the number — until four documents said it
did.

**Nobody read the state's list against the state.** The list in the doc
comment was written when the state was new and the writer had the
statistics open in another window. It is the same shape as §108's "the whole
state": a list of what an object carries, written from memory.

**The memory was nobody's business — until it was.** §52 gave the undo stack
a byte budget because the engine clones the whole document for every command
and keeps 128 of them. The budget is a promise to the *user* ("your
application will not grow without bound"), and the number in use is the
thing a person asks about when the application is slow. Nothing could ask.

### The fix

**The state carries the undo stack**: `undo_depth` (how many steps) and
`undo_bytes` (how much memory they hold), next to the `can_undo` and
`can_redo` booleans that answer the cheaper question.

**The two objects agree, and a test says so.** §109's rule — two answers
about one thing must not disagree — applied to the undo depth, in both
directions: the state and the statistics carry the same numbers, before a
command and after an undo.

**The four sentences are true now**, and the MCP's description says what the
state actually carries: the name, the note, the frame rate, the range, the
nodes, the playhead, the unsaved flag, the undo stack, the log and the plan.

Measured on the running app, after the fix, with the same three commands:

```
session.state -> {..., "can_redo": false, "can_undo": true,
                  "undo_bytes": 18285, "undo_depth": 3, ...}
session.stats -> {"commands": 3, "failures": 0, "kernel_recoveries": 0,
                  "nodes": 5, "undo_bytes": 18285, "undo_depth": 3}
```

### What now holds it

| Property | Held by |
|---|---|
| The state carries the undo stack's depth and its memory | `the_state_carries_the_undo_stack` |
| The state and the statistics agree about the undo stack | `the_state_and_the_statistics_agree_about_the_undo_stack` |

and three mutations in `scripts/mutation-test-gates.sh`: the state does not
carry the undo depth; the state does not carry the undo stack's memory; the
statistics and the state disagree about the undo depth.

### The lesson

**"The whole state" is a promise, and a list of its contents is where the
promise is kept.** §108 found the first half of this — the word "whole" over
an object that was not whole — and §111 found the rest: the object still was
not whole, and the list of what it was supposed to be was written from a
different object entirely. A list of fields is a claim about a struct, and
the only way to check it is to read the struct.

**Two answers about one thing are two things to keep in step.** The state and
the statistics now both carry the undo depth, and a test says they may not
disagree. Before §111 they did not disagree either — they were *different
questions*. A number in one place and a boolean in another is a shape that
looks fine until somebody asks the harder question.

**A budget nobody can see is a promise nobody can check.** §52 bounded the
undo stack's memory; §111 made the number in use visible, so the bound and
the number can be compared by the person the bound was made for.

---

## 112. The browser panel kept a flag for "a plan is saved here", and its error colour came from the wrong panel

### The measurement

The browser app is the only front end that can save a design: the desktop app
opens a file and cannot write one (§33), and the browser writes to the
origin's one plan slot. Its storage panel is the panel that says so.

Measured on the panel's own code:

| Thing | What it did |
|---|---|
| `Application::saved` | **written three times, read never** — set on save, cleared on "Forget", and initialised to `false`; the only read in the tree was a test asserting it was `false` |
| the line *"This browser holds one plan. Saving is explicit."* | **unreachable** — it was the `note.is_empty()` arm, and the note is never empty: `restore` always produces one |
| the note's colour | `self.state.status_is_error` — the **shell's** last command, not the panel's last action |

The flag's own doc comment said it existed *"so the panel can say 'saved'
rather than making the user wonder whether it has ever saved anything"*, and
the panel never said anything of the kind. The line the flag was for was the
one that could never be shown.

The colour was worse than dead: it was **live and wrong**. The panel drew its
note in the warning colour when the *shell's* last command had failed — a
command that has nothing to do with saving — and in the ordinary colour when
the browser itself had refused to save. A user whose save failed was told
nothing, and a user whose last command failed was told their save had.

Both pictures are the real panel, drawn headlessly by the app's own frame
(there is no browser here — §13 — so the app's egui context is rendered to a
PNG instead):

*Before* — one line, the note, and nothing about the slot:

> opened the sample plan; nothing was saved here yet

*After* — two lines: what this browser holds, and what just happened:

> This browser holds one plan, and it is saved.
> saved 5667 bytes under "craftkit/plan"

And after a refused save, the note is in the warning colour and the line
about the slot still says what is true:

> This browser holds one plan slot, and nothing is saved in it yet.
> **this build has nowhere to save; a browser build does**

### Why nothing noticed

**The panel is the one part of the app no test could read.** The shell
publishes every widget it draws, so a test can ask what is on the screen;
the storage panel drew plain egui widgets into a closure and threw the text
away. A line nobody can read is a line nobody can check, and this panel was
drawn — and wrong — for the whole life of the browser build.

**A flag that is written looks like a flag that is used.** `saved` was set in
three places, which reads as "the app tracks whether a plan is saved". It was
never read, and the only test that mentioned it asserted the *false* case.

**The unreachable line was unreachable for a reason nobody noticed.** The
panel's fallback text was the `note.is_empty()` arm, and `restore` always
returns a note — so the arm that described the browser's storage model had
been dead since the panel was written.

**The colour looked like it belonged to the note.** It was computed from the
shell's status flag, which is *the shell's* last command. The two panels sit
in the same window and the flag was right there to read, so nobody asked
whose error it was.

### The fix

**The panel says whether this browser holds a plan**, always, on every frame —
the line that was unreachable is now the first of two, and the note is the
second.

**The panel's outcome is the panel's own.** `Storage::failed` says whether
the panel's last action failed, and the note is drawn in the warning colour
when it did. The shell's status flag no longer has anything to do with it.

**The flag is set from the truth.** `restore` reports whether the browser
holds a plan, whether it loaded, and whether it failed; the panel's buttons
are now methods (`save`, `restore_from_browser`, `copy`, `forget`), so what
they do can be tested without a pointer.

**The panel can be read.** The application keeps what it last drew
(`said`), so a test can ask the panel what it said — the same thing the
shell's `ShellState.status` does for the shell.

### What now holds it

| Property | Held by |
|---|---|
| The panel says whether a plan is saved | `the_panel_says_whether_a_plan_is_saved` |
| A refused save is a warning and a successful one is not | `a_failed_save_is_drawn_as_a_warning_and_a_successful_one_is_not` |
| The panel's outcome is not the shell's outcome | `the_panels_outcome_is_not_the_shells_outcome` |
| The panel can be read | `the_panel_remembers_what_it_said` |
| A reopened plan is reported as saved | `a_stashed_plan_reopens_unchanged` |

and four mutations in `scripts/mutation-test-gates.sh`: the panel does not
say whether a plan is saved; a refused save is not drawn as a warning; a
reopened plan is not reported as saved; the panel ignores what it knows about
the stash.

### The lesson

**A panel that says something should be able to say what it said.** Every
other surface in the tree publishes what it draws — the shell's widgets, the
automation registry, the window's pixels. The storage panel drew text and
threw it away, so its two defects (§112) were invisible to every test in the
workspace.

**A flag that is only written is a promise to nobody.** `saved` was set in
three places, and the sentence that explained why said the panel would use
it. The use was never written, and the sentence was believed.

**A colour needs an owner.** The note was coloured by a flag that belonged
to a different panel. The rule the tree already has — *"the state is
everything a panel needs"* — is about panels, and this was a panel reaching
into another panel's state for a signal it did not own.

---

## 113. The crash reports were written for an agent to read, and the front end whose reader is an agent could not read them

### The measurement

The tree writes a crash report when the application panics: the design that
crashed, the commands that produced it, and the commands needed to replay it
(§47, §57, §58). It is a careful piece of work — a private folder, a UTC id
that sorts, a design file, a replay file, and a replay that says whether the
document came back the same.

Three of the four front ends could read one:

| Front end | How |
|---|---|
| the CLI | `craftkit-cli crashes list \| show <id> \| replay <id>` |
| the control channel | `diagnostics.crash_reports` with `action`: `list`, `show`, `replay` |
| the desktop app | it serves the control channel, so it answers too |
| **the MCP server** | **nothing** |

Measured on the MCP server, over its own stdio transport:

```
tools/list -> 24 tools: craftkit_status, craftkit_commands, and one per command
crash tools: []
```

**Twenty-four tools and not one of them about a crash.** And the MCP server is
the front end whose reader is an agent: *"An MCP server is a subprocess
speaking JSON-RPC, for an agent that has a tool-calling loop"* — the reader
most likely to be holding the tool *because* something crashed, and the one
surface that could not say what happened.

The control channel's `crashes` was also its own copy of the three branches: its
own wording for a missing id, its own wording for an unknown action, and its
own idea of what a replay answer looked like — and the MCP server had none of
it.

And the MCP crate's own module doc said:

> The important thing is that **it exposes no new behaviour**. Every tool maps
> onto a [`craftkit_engine`] command …

which was already half false (two of the twenty-four tools were not commands),
and the tools module's comment said:

> `session.state` and `diagnostics.list` are commands already, so they appear
> from the registry.

`diagnostics.list` is a command. **`session.state` is not** — it is the
control channel's method, and the registry has no such id — so it did not
appear from the registry, and the sentence was wrong about the one thing it
was explaining.

### Why nothing noticed

**A crash report is written by the app, and read by a person.** The desktop
app writes one when it panics, and the tree's own advice to a user is
`craftkit-cli crashes show <id>` — a command line. The MCP server is a
subprocess an agent starts; the agent has no reason to know that a crash
folder exists, and nothing in the tool list mentions one.

**The tool list is generated from the registry, and the crash reports are not
commands.** The MCP server's tools are one per command plus a handful of
built-ins, and the crash reports were never a command — so "the tools are
the commands" was true, and the *capability* was missing. Nothing generated
the tool because nothing was asked to.

**Two copies of three branches look like one rule.** The control channel's
`crashes` and the engine's `crash` module were written at different times for
different transports, and they agreed because they were short. Agreement by
coincidence is not agreement.

### The fix

**The crash reports are a tool.** `craftkit_crashes` takes an `action`
(`list`, `show`, `replay`) and an `id`, and answers with the same JSON the
control channel answers with. The MCP server now publishes 25 tools, and one
of them is about the crash reports.

**The three actions are one rule, in the engine.** `craftkit_engine::crash::action_in`
is the dispatcher: the action, the id, the design the caller is looking at, and
either the answer or a typed failure (`bad-parameter` for a refusal about the
request, `not-found` for a report that is not there). The control channel asks
it and the MCP server asks it, so the two cannot drift apart — and a test says
what each action means.

**The MCP crate's doc is true now.** Three tools are not commands, and the
doc says which three and why: they read what the engine already knows and
change nothing.

### What now holds it

| Property | Held by |
|---|---|
| The crash reports are a tool | `the_crash_reports_are_a_tool` |
| The tool can be called and answers | `the_crash_reports_are_a_tool_an_agent_can_call` |
| An action that does not exist is refused | `a_crash_action_that_does_not_exist_is_refused` |
| The three actions are one rule | `the_crash_actions_are_one_rule` |

and four mutations in `scripts/mutation-test-gates.sh`: the crash reports are
published under a name nobody calls; the tool ignores the action it is given;
a missing report id is not the refusal it should be; the control channel does
not ask the engine.

### The lesson

**The front end whose reader is an agent is the one that must be able to ask
what happened.** The crash pipeline is a whole chapter of the tree's work,
and the surface an agent holds in its hand could not see any of it.

**Two copies of three branches are two rules.** The dispatcher is in the
engine now, and the two front ends ask it the same question. The control
channel's `crashes` had its own wording for every failure; the MCP server
would have had to invent its own, and the two would have disagreed about what
"not found" means.

**"Every tool maps onto a command" was true, and the capability was missing
anyway.** The tools are generated from the registry, so a capability that is
not a command has to be built by hand — and nobody built it. The rule was
about the *shape* of a tool, and it said nothing about whether the tool list
covered what the tree can do.

---

## 114. The kernel described an engine that does not call it

### The measurement

Three sentences, in three files, about the same thing — the kernel's API and
the engine that was supposed to be using it:

| Where | What it says |
|---|---|
| `crates/kernel/src/measure.rs` | *"The kernel is the only layer that touches implementation detail, so it is the only place that can report **how much work happened**. **The engine uses that to decide whether an evaluation is worth caching.**"* |
| `crates/kernel/src/lib.rs`, on `guarded()` | *"**The engine uses this** to decide whether a recovered panic should be surfaced to the user as a warning or can be logged quietly: a kernel panic during ordinary work is a bug worth reporting, and one during a speculative background pass is noise."* |
| `docs/patterns/one-trust-boundary.md` | *"…and the engine reports the two differently. **`recovered_count()` and `guarded()` let it tell the two apart without parsing a log.**"* |

And what the tree actually does:

- **The engine's cache is content-addressed, not cost-based** (§80). An entry
  is reused when the node it was computed from still hashes the same, and the
  walk stops at the first entry that does not. A *cost* would not change that
  decision: the expensive node is the one whose content changed, whatever it
  cost to compute. `measure_run` is called by its own tests and by nothing
  else — a negative result §62 already recorded, in a different context.
- **`guarded()` is called by nothing outside the kernel crate.** The engine's
  rule for a recovered kernel panic is one rule: it is a bug, it is reported
  as `EngineError::Internal`, and the application's panic hook writes a crash
  report for it, because a panic that was recovered is still a panic. There
  is no quiet path, and there is no speculative background pass.
- **The engine tells "a bug" from "your input" by the variant it is holding.**
  `KernelError::Internal` against `Refused` and `Failed` — not by a counter.
  `recovered_count()` is what `session.stats` reports as `kernel_recoveries`:
  a diagnostic, not a decision.

### Why nothing noticed

**The kernel is the crate the tree trusts most, so its doc comments are the
ones nobody re-reads.** The kernel is the trust boundary (§78), the one place
a foreign implementation would be plugged in, and a reader who is checking the
*rule* does not stop to check the *caller*.

**The sentences were written for a design that was better and never arrived.**
A cost-based cache and a quiet path for speculative work are both reasonable
things to want. The tree grew a seam for each and then grew a different rule:
the cache became content-addressed, and the quiet path became unnecessary
because the only guarded operation the application calls — `apply_gain` — has
total arithmetic, so nothing is recovered to keep quiet about. The seams
stayed, and the sentences stayed with them.

**Nothing calls them, so nothing contradicts them.** A doc comment is not
checked against its callers. The tree's own gates check prose against
*constants*, *symbols* and *commands* (§41, §42, §77) — never against the
callers of a function.

### The fix

**The three sentences now say what is true**, and say it in a way a reader can
check:

- the measurement module says that **nothing in the tree asks** for a cost,
  and why a cost would not change the cache's decision;
- `guarded()` says that **nothing outside the crate calls it**, that it is
  the per-thread counterpart of `recovered_count()`, and that the tree has
  one rule for a recovered kernel panic — reported, never quiet;
- the pattern document says that **the variant** is what tells a bug from a
  bad input, and that the two counters are diagnostics rather than the
  decision.

**The facts the sentences now describe are pinned by the kernel's own tests**
— the depth balances across nesting and an early return, a guarded call on
one thread does not guard another, and a recovery on one thread is counted on
another — and one new mutation holds the boundary: *the guard depth never
unwinds*.

### What now holds it

| Property | Held by |
|---|---|
| The guard depth balances | `depth_is_balanced_across_nesting_and_early_return` |
| A guarded call on one thread does not guard another | `being_guarded_on_one_thread_does_not_guard_another` |
| A recovery on one thread is counted on another | `a_recovery_on_one_thread_is_counted_on_another` |

and one mutation in `scripts/mutation-test-gates.sh`: the guard depth never
unwinds.

### The lesson

**A library's API outlives the caller it was written for, and its doc
comment keeps that caller alive in the reader's mind.** Three sentences
described an engine that does not exist, in the crate the tree trusts most,
and every one of them was about a *caller*.

**A seam is not a rule.** The kernel has a measurement and a per-thread guard,
and the tree has neither a cost-based cache nor a quiet path. Both seams are
still the right place for those rules to live; neither rule exists, and a
sentence that says one does is worse than no sentence — because it is
*checkable-looking*.

**The gates check prose against constants, symbols and commands — never
against callers.** That is the hole this section fell into, and it is a
general one: a doc comment's most load-bearing claim is usually about
*who uses it*, and nothing in the tree checks that.

---

## 115. The timeline was a duration, and the playhead was a frame

### The measurement

A design from one second to five seconds, at 24 fps:

```text
design:  start frame 24, end frame 120, duration 96 frames
```

And the timeline, in `crates/ui/src/shell.rs`:

```rust
let total = meta.duration().frames_i64(rate).max(1);
let mut frame = self.session.at().frames_i64(rate);
egui::Slider::new(&mut frame, 0..=total)
```

**The slider's range was the duration and the value it held was an absolute
frame.** Two numbers that are not the same number: 96 is how long the design
is, 24..=120 is where it is. The slider said the design was frames 0 to 96,
and the handle it drew was placed by mapping the playhead's frame through
that range.

What a person got, measured by pressing the ends of the slider and reading
the playhead back:

| Press | Playhead | Should be |
|---|---|---|
| the far right | frame **96** | frame **120** — the design's last frame |
| the far left | frame **0** | frame **24** — the design's first frame |

The far right stopped **twenty-four frames short** of the end of the design:
the last second of a five-second animation could not be reached from the
timeline at all. And the handle was drawn in the wrong place — a quarter of
the way along the rail, because 24 of 96 is a quarter:

| | the handle |
|---|---|
| before | a quarter of the way along the rail |
| after | at the rail's start, where the design's first frame is |

Three more facts, all measured:

- **The slider's own formatter never ran.** `custom_formatter` was set —
  `format!("{:.2}s", f / fps)` — and `show_value(false)` was set beside it.
  egui calls the formatter **for the value it draws**, and the value was
  hidden: the closure was dead code. The timeline showed no time, and the
  frame number was shown nowhere at all.
- **The playhead could sit outside the design.** `doc.range {"start": 1.0}`
  moved the design's range and left the playhead at frame 0 — before the
  design's first frame, where there is nothing to see. Opening a design
  whose range starts one second in did the same thing: the playhead stayed
  where it was, and the plan was evaluated outside the design.
- **`playback.seek` clamped; nothing else did.** The engine's rule for the
  playhead has always been the closed range `[start, end]`. The two ways the
  range can move under the playhead — a new range, and a document that
  arrives from outside — did not apply it.

### Why nothing noticed

**The sample design starts at zero.** A design from 0 s to 5 s has frames
0..=120 and a duration of 120 frames, so the duration and the range are the
same range, the handle is in the right place, and the far right of the
slider is the design's last frame. Every design anybody opens by default
looks like that.

**The playhead's frame number was visible nowhere, so nobody could check the
handle.** The slider's value was hidden and its formatter never ran; the
plan's summary showed the tick count and the seconds. The one number the
handle's position is made of — the frame — was on no screen at all.

**§89 said the frame number was visible.** *"The playhead's frame number is
visible, so the drift is visible too."* It was not: the number existed in
the model, in the state and in the arithmetic, and a person could not see
it. The sentence was checked against the tree by reading the tree, and the
tree has the number in it.

**The range and the duration are both "the design's length".** A duration of
96 frames and a range of 96 frames look the same in a printout, and the
code that used the duration was *about* the design's length — so it read as
right.

### The fix

**The model owns the answer.** `DocumentMeta::frame_range()` returns the
design's frames — `start..=end`, at the design's own rate — and the two are
not the same number:

```rust
pub fn frame_range(&self) -> std::ops::RangeInclusive<i64> {
    let rate = self.frame_rate();
    let start = self.start.frames_i64(rate);
    // A range shorter than one frame has no last frame of its own; the
    // design still has a place to stand.
    let end = self.end.frames_i64(rate).max(start);
    start..=end
}
```

**The timeline uses it.** The slider's range is the design's range, and the
value is the playhead's absolute frame, clamped into it.

**The engine keeps the playhead in the design.** One rule, in one place:
`Session::clamp_playhead_into_range`, called from `Session::restore` — the
one place the document changes under the session — and from the range
command. `playback.seek` already clamped to the same range, so there is one
rule and not two.

**The command says when it moved the playhead.** A command that changed two
things and mentioned one lies by omission, and `doc.range` now says *"range
1.000s..5.000s at 24 fps; the playhead moved to 1.000s"* when it did.

**The timeline says where the playhead is.** The readout is the shell's own
label — the frame and the time, side by side — and the slider's formatter is
gone. egui's own value field was not used: it would be a second widget that
edits the playhead, and the registry would not name it (§100's rule). The
readout also makes §89's sentence true: with the frame number and the time
on the screen together, the drift between them is visible.

### What now holds it

| Property | Held by |
|---|---|
| The design's frames are its range, not its duration | `the_designs_frames_are_its_range_and_not_its_duration` |
| The timeline reaches both ends of the design | `the_timeline_can_reach_both_ends_of_the_design` |
| The timeline says where the playhead is | `the_timeline_says_where_the_playhead_is` |
| The readout says the frame and the time | `the_playhead_readout_says_the_frame_and_the_time` |
| A range that moves under the playhead takes it along | `a_range_that_moves_under_the_playhead_takes_the_playhead_with_it` |
| A document that arrives takes the playhead with it | `a_document_that_arrives_with_its_own_range_takes_the_playhead_with_it` |
| The range command says when it moved the playhead | `the_range_command_says_when_it_moved_the_playhead` |

and seven mutations in `scripts/mutation-test-gates.sh`: the timeline is the
duration again; the model's frames are the duration; the readout is not
drawn; the readout does not say the frame; a range that moves under the
playhead leaves it there; a document that arrives leaves the playhead outside
it; and the range command does not say that it moved the playhead.

### The lesson

**A duration and a range are both "how long".** The number that says how long
a design is and the number that says where it is are different numbers, and
a widget that holds one while meaning the other is wrong in a way that only
shows up on a design that does not start at zero.

**A widget that cannot be seen cannot be checked.** The handle's position is
made of the frame number, and the frame number was on no screen — so the one
thing a person would have noticed was the one thing they could not name.

**A formatter that is never called is a sentence about a thing that does not
happen.** egui calls the formatter for the value it draws. Hiding the value
silently turned the formatter into a comment, and the comment said the
timeline had a scale.

**A range that moves under the playhead has to take it along.** The engine
had one rule for the playhead — `[start, end]` — and applied it in one of the
three places the playhead can move. The rule was right; the coverage was
not.

---

## 116. The catalogue said one spelling of a parameter was required, and the type said the parameter took something the reader refuses

### The measurement

Thirteen commands take one parameter in **two spellings**:

| Spelling | The other | Who says so |
|---|---|---|
| `node` — an id | `node_name` — a name | the help: *"…or the node's name."* |
| `seconds` | `frame` | the help: *"…or a frame number at the plan's rate."* |

The tree says the same thing everywhere it refuses:

> pass `node` (an id) or `node_name` (a name)
>
> pass `seconds` or `frame`

And the catalogue said the opposite:

```json
{ "name": "node",      "required": true,  "type": "node-ref" }
{ "name": "node_name", "required": false, "type": "text" }
{ "name": "seconds",   "required": true,  "type": "time" }
{ "name": "frame",     "required": false, "type": "number" }
```

**`required: true` on the first spelling, in all thirteen commands.** And the
MCP builds its tool schemas from that catalogue, so the published schemas
carried it too — eleven tools with `"required": ["node"]` and one with
`"required": ["seconds"]`.

Four more measurements, all from the tree itself:

- **The MCP's own test says what that does.** *"An optional parameter must
  not be in `required`, or a client will refuse to call the tool without
  it."* The schema did exactly that, to a parameter that had an alternative.
- **The tree's own probe used the forbidden form.** The engine's read-only
  probe table calls `playback.seek` with `{"frame": 48}` — the spelling the
  published schema said it could not send.
- **The `node-ref` type said the reader took a name.** `Doc::NodeRef`'s doc
  said *"A node id, as a number or as the node's name"*, and the schema said
  `["number", "string"]` — and the reader refuses exactly that: *"`Level` is
  a name, not an id; names are resolved by `node_name`"*.
- **The published description had two full stops in it.** *"The node's id.. A
  whole number, or use `node_name`."* — the MCP appends a sentence to a help
  that already ends in one.

### Why nothing noticed

**The command works.** Every one of the thirteen commands accepts both
spellings, so every test that calls a command passes. The catalogue is
*describing* the command, and nothing checked the description against the
command — the tree's gates check prose against constants, symbols and
commands (§41, §42, §77), and a boolean in a JSON catalogue is none of those.

**The `?` in the CLI's listing looked like the answer.** The listing printed
`node` with no `?` and `node_name` with one, which reads as "the id is
needed, the name is a convenience". The convention was never written down
anywhere, so nothing said what it meant — and the one place it mattered, the
place where the listing is *about* parameters, was the place with no rule.

**The schema's `required` is a promise about what a client may send.** A
client that reads it and finds `node` compulsory will not send `node_name`
— not because the command refuses it, but because the client never tries.
The refusal never happens, so the tree never learns that the promise was
wrong.

**The id and the name are two parameters in the tree, and the pair was read
as two separate parameters.** `node` is an id and `node_name` is a name, and
the reader keeps them apart on purpose (*"names are resolved by
`node_name`"*). Nobody had said where the *pair* is written down, so it was
written down nowhere.

### The fix

**The catalogue learns the pair.** `ParamDoc` has one more field:

```rust
/// The parameter that may be given **instead of this one**.
pub alternative: Option<&'static str>,
```

and a constructor for it, `ParamDoc::either`, for a parameter the command
needs *unless the other one is given*. The pair is "one of the two": neither
is required on its own, and the command refuses when it has neither — which
it already did.

**The schema says "one of the two".** The MCP's schema builder turns each
alternative into an `anyOf`:

```json
"required": ["name"],
"allOf": [{ "anyOf": [{ "required": ["node"] }, { "required": ["node_name"] }] }]
```

The `allOf` is there for the general case: two pairs are two separate "one
of"s, and an `anyOf` over all four names would let a call through with
neither of one of them.

**The node parameter is a whole number again.** The type said
`["number", "string"]` and the reader refuses a string there; the name is
the parameter's alternative, and the schema says so beside it.

**The listing says it too.** The plain catalogue names the other spelling on
the line — `?node: node-ref — The node's id. (or give `node_name`)` — and
the CLI's help now explains the `?`:

> A parameter marked `?` is optional. A parameter with an alternative is
> marked `?` too, and its line names the other spelling: give one of the
> two.

### What now holds it

| Property | Held by |
|---|---|
| The catalogue says which parameter may be given instead | `the_catalogue_says_which_parameter_may_be_given_instead` |
| An alternative is a parameter of the same command | `an_alternative_is_a_parameter_of_the_same_command` |
| The schema says one of the two | `a_parameter_with_an_alternative_is_one_of_the_two` |
| The node parameter is a whole number | `the_node_parameter_is_a_whole_number` |
| A node may be addressed by its name | `a_node_may_be_addressed_by_its_name` |
| The seek may be given a frame | `the_seek_may_be_given_a_frame` |
| The listing names the alternative | `a_parameter_with_an_alternative_says_which_one` |

and five mutations in `scripts/mutation-test-gates.sh`: the catalogue does
not say which parameter may be given instead; the schema does not say that a
parameter has an alternative; the node parameter says it takes a name again; a
node cannot be addressed by its name; and the catalogue's listing does not
name the alternative.

### The lesson

**`required` in a schema is a promise about what a client may send.** It is
read by the client and obeyed by it, and a promise the command does not
keep is not a promise at all. The tree already knew this — its own MCP test
says so — and the schemas were built from a field nobody had checked.

**Two spellings of one parameter need one place to live.** The commands
already had the pair; the *catalogue* did not, so the catalogue described
half of it and called the whole of it required.

**A type is a promise about what the reader accepts.** `["number",
"string"]` said a name was welcome in the id parameter, and the reader's
own message said it was not — and the message was the one an agent would
have read, after the schema had already refused to let it send one.

---

## 117. The keyframes were in the document, in three commands, and in no reply and on no screen

### The measurement

Three commands address a keyframe **by its instant**:

| Command | What it does with the instant |
|---|---|
| `key.set` | writes the key at that instant |
| `key.remove` | removes the key at that instant |
| `key.ease` | changes how the key at that instant is left |

All three take a `time`. And the tree had **no way to ask when the keys
were**:

- **No command listed them.** The registry held twenty-two commands, and
  none of them answered *"when are this property's keyframes?"*.
- **`key.clear` reported the count, not the times.** *"cleared 3
  keyframe(s)"* — and the three it removed are gone.
- **The state carried the plan, and the plan carries the value and a flag.**
  `Resolved` has the path, the id, the value at the playhead and
  `animated: true` — and no times.
- **The window's properties panel said "3 keyframe(s)"** — a count — and
  **not when any of them was**.

So an agent asked to *"remove the keyframe"* had to guess an instant. The
guess was refused honestly — *"`amount` has no key there"* — but there was
nothing to find the right one with. And a person asked *"when does this
property change?"* had to open the `.craftkit` file.

### Why nothing noticed

**The commands that take an instant never needed one.** `key.set` writes a
key at the playhead; `key.remove` and `key.ease` take one; every use in the
tree, in the tests and in the window, was a use that *had* the instant
already. The parameter was never empty, so nobody asked what an empty one
means.

**The count is not the times.** "3 keyframe(s)" is true, and it is the
answer to a different question. §110's shape again: the design's note was
in the file and in the model and nowhere else — and this time the thing
that was nowhere else was the part of the design that *changes*.

**A tree built for agents had no agent's way in.** The whole point of the
command registry is that an agent reaches everything through it. The
keyframes were the one thing in the document that the registry could not
reach — and the reachability was not checked, because there was no list of
what the registry must reach.

### The fix

**A command that lists them.** `key.list` — the twenty-third — is read-only
and says when each keyframe is (in ticks and in seconds), what it holds and
how it is left:

```json
{ "node": 1, "path": "amount", "keyframes": [
  { "time": 254016000000, "seconds": 1.0, "value": { "t": "num", "v": 0.25 }, "ease": "linear" }
] }
```

**The message carries the times too**, because the message is what
`craftkit-cli run` prints and what the status bar shows — and the data is
for the machines. *"2 keyframe(s) on `amount`: 1.000s, 2.000s"*.

**The panel draws them.** The properties panel says the count *and* the
times, under the editor. The line is bounded and the cut is marked, the
same rule as the note's preview (§110).

**A new icon.** `keys` — three marks on a line: the keyframes of a
property, on the timeline they live on. The single key is a bow and a
shaft; a *list* of them is marks on a rail.

### What now holds it

| Property | Held by |
|---|---|
| The keyframes of a property can be listed | `the_keyframes_of_a_property_can_be_listed` |
| The listing says when the keyframes are | `the_listing_says_when_the_keyframes_are` |
| The listing refuses a property that is not there | `the_listing_refuses_a_property_that_is_not_there` |
| The panel says when the keyframes are | `the_panel_says_when_the_keyframes_are` |

and three mutations in `scripts/mutation-test-gates.sh`: the listing does
not say when the keyframes are; the listing's message does not say when the
keyframes are; and the panel does not say when the keyframes are.

### The lesson

**The address of a thing and the list of the things are the same
obligation.** Three commands took an instant as a parameter, and nothing
could produce one. A parameter that nothing can supply is a parameter the
caller has to guess.

**A count is not the thing it counts.** "3 keyframe(s)" was true and
useless: the question was *when*, and the answer was three times.

**Everything a command can change must be readable through a command.**
That is the tree's own rule ("everything is a command"), and the keyframes
were the one part of the document that broke it — not by refusing, but by
being unreachable.

---

## 118. The toolbar ran every command with no parameters, and seventeen of its twenty-one buttons could only be refused

### The measurement

The toolbar is the app's most visible surface, and it draws a button for every
command in the registry. It ran each of them like this:

```rust
self.run(spec.id, serde_json::json!({}));
```

**With no parameters at all.** Twenty-three commands, twenty-one buttons (the
two history buttons are disabled on a fresh session, so they publish no
rectangle), and this is what pressing each one said:

| Command | What the button said |
|---|---|
| `doc.rename` | *"`name` this parameter is required"* |
| `doc.note` | *"`text` give the note's text; an empty text clears the note"* |
| `doc.range` | *"`start` give at least one of `start`, `end` or `fps`"* |
| `node.remove`, `node.rename`, `node.mute`, `node.kind`, `node.move`, `node.duplicate` | *"pass `node` (an id) or `node_name` (a name)"* |
| `prop.set`, `prop.reset` | *"pass `node` (an id) or `node_name` (a name)"* |
| `key.set`, `key.remove`, `key.clear`, `key.ease`, `key.list` | *"pass `node` (an id) or `node_name` (a name)"* |
| `playback.seek` | *"pass `seconds` or `frame`"* |

**Seventeen of twenty-one buttons could only refuse.** The four that worked
were the four that take no parameters: `node.add` (*"added filter `filter
4`"*), `diagnostics.list` (*"0 problem(s)"*), `plan.evaluate` and
`session.stats`.

So the toolbar was a strip of twenty-one buttons of which **four did
something and seventeen said no**. And the *only* way to reach the other
eighteen commands was a script, the CLI, the control channel or the MCP
server — the interfaces for people and agents who are not looking at a
window.

### Why nothing noticed

**The refusal was honest.** Every one of those messages says exactly what was
missing and exactly how to give it. A defect that reports itself accurately
is very easy to mistake for a feature: the status bar was never *wrong*, only
unhelpful, and the status bar is the one thing on the screen that is
supposed to be right.

**The four that worked looked like the toolbar worked.** A test that pressed
*Add node* and saw a node appear would have concluded that the toolbar calls
the registry. It does — with an empty parameter object.

**No test pressed a toolbar button.** The UI tests pressed the *timeline*
(§115), read the *panel* (§117) and looked at the *nodes* (§99, §100) — and
not one of them pressed a button in the toolbar, because every button in it
was a button that needed something the test had no way to supply.

**The tree named the mechanism that was missing.** Four places said that
interactive work belongs to `ui.start`:

- `AGENTS.md`: *"Only a toolbar click (`ui.start`) may start an interactive
  tool."*
- `crates/engine/src/lib.rs`: *"Interactive work is a different entry point
  (`ui.start`), which the front ends own."*
- `crates/engine/src/session.rs`: *"Those are interactive concerns and belong
  to `ui.start`, which the front ends call instead."*
- `docs/patterns/everything-is-a-command.md`: *"Those are interactive
  concerns and belong to `ui.start`, which a front end calls instead."*

**There is no `ui.start` in the tree.** There was no interactive entry point at
all: the tree had a rule about who may ask a question and no way to ask one.

### The fix

**The registry says what a command asks a person for.** A new field on
`CommandSpec`:

```rust
/// The parameters a person is asked for when the command is started from
/// the interface.
pub asks: &'static [&'static str],
```

It is a separate field from `params` because the two differ on purpose:
`node.add` takes a name, a kind and an amount and is perfectly happy with
none of them, while `doc.range` takes three optional parameters and refuses
to run with none of them. Three gates hold it honest — every name in `asks` is
a parameter of that command, no command asks for the same parameter twice,
and every parameter a command cannot do without is one it asks for.

**The engine turns what a person typed into what the reader accepts.**
`craftkit_engine::params::text_to_param` is the one place that decides what a
number, a name or a time means — the readers are strictly typed, so a second
parser in the interface would be a second answer to the same question. A
node's name goes to `node_name` and a node's id to `node`, because a
parameter with an alternative is one parameter in two spellings (§116). A
time is left as text, because only the reader knows the document's frame rate
and so only the reader can say what `12f` means.

**The shell opens a form.** `Shell::start` is the interactive counterpart of
`Session::execute`, and it is the only path that asks a question:

- a command that asks for nothing runs at once;
- a command whose parameters the shell already knows — the node the person is
  looking at — runs at once;
- a command that needs anything else opens a **form**: one field per asked
  parameter, pre-filled with what the shell knows, and an OK that runs the
  command with what was typed.

The form's colours come from `theme::Tokens` like everything else the shell
draws, and its title is drawn by the shell rather than by `egui`, whose own
title is a second palette at a dimmer setting.

**The prose names the thing that exists.** The four `ui.start` mentions now
say what the mechanism is: the front end's own entry point, which asks for
the parameters and then calls `execute`.

### The evidence, in the real window

The real binary, a real display, a real mouse and a real keyboard:

1. Press **Rename plan** in the toolbar. The form opens in the middle of the
   window: the command's label, the field `name`, the parameter's own help
   line (*"The new name."*), an empty field, and OK and Cancel.
2. Click the field and type **Renamed in the live window**. The field takes
   the text.
3. Press **OK**. The form closes, the plan's title in the central panel
   reads **Renamed in the live window**, and the status bar says *"renamed to
   `Renamed in the live window`"* — and says **unsaved**, because the
   document changed.

Before the change, step 1 put *"renamed to …"* nowhere and left the status
bar saying *"`name` this parameter is required"*.

### What now holds it

| Property | Held by |
|---|---|
| No toolbar button is refused for wanting a parameter | `no_toolbar_button_is_refused_for_wanting_a_parameter` |
| A button that needs parameters opens a form | `a_button_that_needs_parameters_opens_a_form` |
| A button that needs a node asks for one | `a_button_that_needs_a_node_asks_for_one` |
| A button whose parameters the shell knows runs at once | `a_button_whose_parameters_the_shell_knows_runs_the_command` |
| The form runs the command when it is filled in | `the_form_runs_the_command_when_it_is_filled_in` |
| The form says what is wrong with what was typed | `the_form_says_what_is_wrong_with_what_was_typed` |
| The form is on the screen when it is open | `a_frame_with_a_form_is_not_the_frame_without_one` |
| What a person typed becomes what the reader accepts | `text_to_param`'s tests in `craftkit-engine` |
| Every name a command asks for is one of its parameters | `every_name_a_command_asks_for_is_a_parameter_of_that_command` |
| A command never asks for the same parameter twice | `a_command_never_asks_for_the_same_parameter_twice` |
| Every parameter a command needs is one it asks for | `every_parameter_a_command_needs_is_one_it_asks_for` |

and six mutations in `scripts/mutation-test-gates.sh`: the toolbar runs a
command with no parameters; the catalogue no longer says what a command asks
for; a command that needs a parameter does not ask for it; the form's OK
button does not run the command; a node's name no longer goes to the name
spelling; and the form is not on the screen.

### The lesson

**A button that is refused is a button that does nothing, and a refusal that
says exactly what is missing is very easy to mistake for a feature.** The
status bar was never wrong. It was the toolbar that was.

**What a command takes and what an interface can supply are the same
question.** Seventeen commands took parameters that no interface could
produce, and the only interface in the tree — the one with the buttons on it
— had no way to ask.

**A name in prose is not a mechanism.** Four places said `ui.start`; there
was no `ui.start`. That is the thirty-ninth instance of the shape item 8
describes, and the first one whose subject was a **piece of behaviour** rather
than a sentence about one.

---

## 119. A keyframe outside the design's range changed the design, and nothing said so

### The measurement

A design whose range is 0.000 s .. 5.000 s at 24 fps, and a property called
`amount`:

| What was done | What the design said |
|---|---|
| `key.set` at 30.000 s | *"keyed `amount` at 30.000s"* — accepted |
| `key.set` at -5.000 s | *"keyed `amount` at -5.000s"* — accepted |
| `diagnostics.list` | *"0 problem(s)"* |
| `key.list` | *"2 keyframe(s) on `amount`: -5.000s, 30.000s"* |
| 200 seeks from 0.0 s to 199.9 s | the playhead never left 0.000 s .. 5.000 s, and never reached either key |

So the two keys are **unreachable**: the playhead is clamped to the design's
range, and no instant of the design is either of them.

And then the measurement that matters. With **no** keys, `amount` at 0.000 s
is **1.0** — the property's own value. With a key at -5.000 s holding 0.25 and
a key at 30.000 s holding 0.75, `amount` at 0.000 s is:

```
0.25 + (0.75 - 0.25) x (0 - (-5)) / (30 - (-5))  =  0.3214285714285714
```

**A key past the end of the design changed every instant of the design.** The
value between two keys is interpolated, and the interpolation ran *across* a
key the playhead can never reach. At 4.999 s the value was 0.39284285714285716
— the same interpolation, a little further along, towards a value the design
never contains. Nothing anywhere said so: the command said only the time, the
diagnostics said there was nothing wrong, and the panel (§117) said the times
next to no range to compare them with.

### Why nothing noticed

**The model is right, and that is the point.** `Keyframes::sample` holds the
value before the first key and after the last — a rule that only matters for
keys the design does not contain. The model's own comment says the clamping
is what stops a caller having to *"special-case 'before the animation
starts'"*. So a key outside the range is **legal**, and refusing it would
throw away a rule the model deliberately has.

**Nothing compared the two ranges.** `Document::validate` checks the design's
range and the keyframes, and never checks one against the other. Two facts
about instants, each checked on its own, and the place where they meet was
nobody's job.

**§117 made the times visible, which made the gap harder to see.** The panel
now says *"2 keyframe(s) / -5.000s, 30.000s"* — and a time reads as an instant
of the design unless something says the design ends at 5.000 s. The listing is
*more* informative than it was and still says nothing that could be checked.

**The timeline showed the range and no keys.** The design's instants are the
timeline, and the keyframes were on no timeline, so there was no picture in
which the two could be seen to disagree. §121 has since put the keys on the
rail, and this sentence is the one that went stale when it did.

**A person cannot make one; a program can.** `key.set` asks a person for the
node, the path and the value, and the **time** defaults to the playhead —
which is inside the range by construction. So the form cannot key outside the
design, the properties panel's "key at the playhead" cannot either, and the
gap was one a **script, an agent or the CLI** could walk into. Which is why
the reporting had to be for everyone else and not only for the person who
pressed the key.

### The fix

**The command says it, at the moment it happens.** `key.set` at 30.000 s now
says:

> keyed `amount` at 30.000s — past the end of the design (5.000s), so the
> playhead can never reach it

and a key before the start says *"before the design starts at 0.000s"*. The
reply carries `"outside_range": true`, so an agent does not have to parse the
sentence.

**The diagnostics report it, for everyone else.** The evaluator knows the
document's range and every node's keyframes, so the plan's diagnostics now
carry:

> the keyframes of `amount` reach 30.000s, past the end of the design
> (5.000s)

and, at the other end, *"reach back to -5.000s, before the design starts at
0.000s"*. Both ends, not one: a property with a key at each end is outside
the range at both ends, and reporting only the first would hide the one that
changes every instant of the design.

The warning is a **warning**, not an error. The design is legal, and the
person who made the key may have meant it.

### The evidence, in the real window

A design keyed past its end (by the control channel, since the form cannot
key outside the range) and then the **Problems** button pressed in the real
window:

- the status bar says **1 problem(s)**;
- the plan panel says **Problems** and, under it, the warning the evaluator
  produced: *"warning: Level \"1\": the keyframes of `amount` reach 30.000s,
  past the end of the design (5.000s)"*;
- the plan's own footer says *"0t at 0.000s — 0 reused, 3 evaluated, 1
  problem(s)"*.

Before the change the same design said **0 problem(s)** and the panel said
nothing at all.

### What now holds it

| Property | Held by |
|---|---|
| A key outside the design says so | `a_key_outside_the_design_says_so` |
| A key before the design says so | `a_key_before_the_design_says_so` |
| A key inside the design does not say it is outside | `a_key_inside_the_design_does_not_say_it_is_outside` |
| The diagnostics report a key the playhead cannot reach | `the_diagnostics_report_a_key_the_playhead_cannot_reach` |
| A keyframe outside the design is a warning | `a_keyframe_outside_the_design_is_a_warning` |
| A keyframe outside the design is reported at both ends | `a_keyframe_outside_the_design_is_reported_at_both_ends` |
| A keyframe inside the design is not a warning | `a_keyframe_inside_the_design_is_not_a_warning` |

and three mutations in `scripts/mutation-test-gates.sh`: the command does not
say the key is outside the design; a key before the design is not noticed; and
a key past the end of the design is not a warning.

### The lesson

**A keyframe is not an instant of the design until the design contains it.**
Everything about a key assumes it is somewhere the playhead can be, and
nothing checked.

**"The value holds outside the keys" and "the design is a range" are two
rules, and the place where they meet was nobody's job.** The model was right
about each of them; the gap was the comparison between them.

**A value that is interpolated towards an instant nobody can reach is not
inert.** The key at 30 s did not wait, quietly, for a longer design. It was
already changing the design.

---

## 120. The catalogue said a parameter was optional, and the command could not run without it

### The measurement

Fourteen commands take a parameter in one of two spellings: thirteen address a
node by `node` or `node_name`, and `playback.seek` seeks by `seconds` or
`frame`. §116 made the pair **one parameter in two spellings** and stopped
marking the first one required — and the catalogue's own two faces still said
only that each half was **not required**.

The engine's catalogue — what `engine.describe` and `craftkit-cli commands
--json` return — said, for `node.mute`:

```json
{ "name": "node",      "type": "number", "required": false, "help": "The node's id.", "alternative": "node_name" },
{ "name": "node_name", "type": "string", "required": false, "help": "…or its name." }
```

Two entries, both `required: false`, and **nothing anywhere in that JSON**
saying that the command needs one of them. A reader of it is entitled to
believe that `node.mute` can be called with no parameters at all.

The plain listing — what a person runs first — said:

```text
      ?node: node-ref — The node's id. (or give `node_name`)
      ?node_name: text — …or its name.
```

Two `?`s. The listing's marker has exactly two states, *required* and *may be
left out*, and a pair is a third thing that neither of them can say: **two
lines that both say "may be left out" say that neither is needed.**

And the command says the opposite:

> pass `node` (an id) or `node_name` (a name)

So the catalogue's own JSON and its own listing said the command could run
with nothing, and the command said it could not. The **only** face of the
catalogue that had it right was the MCP's JSON Schema, which §116 gave an
`allOf` of `anyOf`s — and a JSON Schema is the one face a person reading
`craftkit-cli commands` never sees.

### Why nothing noticed

**§116's fix moved the truth and left the fact behind.** The old catalogue
marked the first spelling required, which was wrong in the other direction:
it told a client it could not use the name. The fix made both halves
"not required" ✓ and put *"one of the two"* into the schema ✓ — and the
pair's requiredness was never a fact the catalogue itself carried ✗.

**The commands are honest.** Every one of the fourteen refuses with a message
that names both spellings ✓, so anybody who *tried* it was told the truth at
once. Only somebody who *read* the catalogue was misled — and reading is
what an agent does before it calls.

**The refusal is the same shape as a typo's.** *"pass `node` (an id) or
`node_name` (a name)"* is what a command says when a parameter is missing,
so a reader who saw it could equally believe the catalogue's `?` and think
the pair was a nicety.

### The fix

**The catalogue carries the pairs.** Every command's schema now has:

```json
"one_of": [["node", "node_name"]]
```

— the groups of which one must be given. It is derived from the same
`ParamDoc::alternative` the MCP's schema builder reads ✓, so there is one
declaration ✓ and two renderings ✓.

**The plain listing prints a pair as one line.**

```text
      node|node_name: node-ref — The node's id, or its name. (one of the two is required)
```

Both spellings, one line, and the fact that the pair is needed. The second
spelling does not print a line of its own, because a line of its own is a
line that says "may be left out" ✗.

**The help says what the field takes.** The parameter's help went from *"The
node's id."* to *"The node's id, or its name."* ✓ — so the catalogue's JSON,
the listing and **the form** (§118) all say the same thing about what may be
typed. The form's label says *node or node_name* ✓.

### What now holds it

| Property | Held by |
|---|---|
| The catalogue says which pairs one of must be given | `the_catalogue_says_which_pairs_one_of_must_be_given` in `craftkit-engine` |
| A command that needs one of a pair refuses with neither | `a_command_that_needs_one_of_a_pair_refuses_with_neither` |
| A pair prints as one line that says one of the two is required | `a_pair_prints_as_one_line_that_says_one_of_the_two_is_required` |
| A pair prints as one line and the second spelling does not print its own | `a_pair_prints_as_one_line_and_the_second_spelling_does_not_print_its_own` |
| A plain parameter prints as before | `a_plain_parameter_prints_as_before` |
| An optional parameter is still marked with a `?` | `an_optional_parameter_is_marked_with_a_question_mark` |
| The catalogue's listing names the pairs | `the_catalogue_says_which_pairs_one_of_must_be_given` in `craftkit-cli` |

and three mutations in `scripts/mutation-test-gates.sh`: the catalogue no
longer says which pairs one of must be given; the listing does not say the
pair is required; and the listing prints the second spelling as its own line.

§116's own listing mutation was re-aimed, and the suite is what said so: it
dropped the *"(or give `node_name`)"* suffix, which this change made
unreachable, so the gate it pointed at could no longer fail — and a gate that
cannot fail is worse than no gate. The mutation now drops the second
spelling from the pair's line ✓, which is the same claim in the new
rendering ✓.

### The lesson

**"Not required" and "one of these is required" are different facts, and a
boolean can only hold the first.** The catalogue had one boolean per
parameter, and a pair of parameters is not a parameter.

**A pair needs a place of its own in the catalogue.** Otherwise each half is
read on its own — and read on its own, both halves are optional, and the
command is not.

---

---

## 121. The timeline was a hundred pixels wide and showed not one keyframe

### The measurement

A design's keyframes are the only part of it that moves. §117 put their times
in the properties panel — as text, for the selected node — and `key.list` put
them in a reply. The **timeline** showed the range and the playhead.

Two frames of the same design, differing in one thing only — a keyframe at
1.0 s against the same keyframe at 3.0 s — were **pixel-identical**:

| the two designs, rendered at 900 × 600 | pixels that differ (of 540 000) |
|---|---|
| before | **0** |
| after | **180**, every one of them on the rail |

Zero. The window could not have told a person which of the two designs it was
showing, and the two designs differ in exactly one thing: **where the key
is**. The value at every instant is the same in both — a single key's value
is held before it — so the difference between the two documents is a fact
about the *timeline*, and the timeline said nothing about it.

The rail itself was a stub. `egui`'s default slider is a hundred pixels wide,
and the timeline's rail was a hundred pixels wide: the playhead's published
rectangle was `[[216.3 123.0] - [316.3 141.0]]`, measured in a 900-pixel
frame. The same rail now spans the panel —
`[[216.3 123.0] - [692.0 141.0]]`, 476 pixels of it.

And the keyframes of the **other** nodes were nowhere at all: the properties
panel speaks for the selected node, so a design whose third node carried the
keyframes showed nothing, anywhere, about them.

### Why nothing noticed

**Nothing in the tree drew a keyframe.** The timeline drew the range, the
playhead and the readout; §117's panel drew the times as text. There was no
code path that *could* have drawn a mark — so there was no bug in one. There
was an absence.

**The engine and the panel both knew where the keys were.** `key.list`
answered with the times (§117), the panel printed them (§117), and the
evaluator sampled the curve through them. The *information* was in three
places; the place a person looks to find out **when** something happens was
not one of them.

**A design with no keyframes looks exactly like a design with keyframes.**
That is the shape of this defect: it is not wrong, it is missing — and a
missing thing is invisible to a reader who does not know it should be there.
§120's catalogue *said* something false; anybody who tried it was told the
truth at once. This one said nothing at all, and the only reader who could
have noticed was one who expected it.

### The fix

**The timeline is the design's, and the keyframes are on it.** Every node's
keyframes are marked on the rail — not only the selected node's, because the
timeline speaks for the design and the properties panel speaks for one node.

**The rail spans the panel.** `egui`'s slider is a hundred pixels wide by
default; the timeline's is the panel's width — 476 pixels in a 900-pixel
frame and 755 in a 1180-pixel one, measured in the running window.

**A mark is at the frame it names.** The marks are placed where the playhead's
handle travels, which is not the rail's whole width: `egui` keeps the handle
a radius inside each end. The rule is in one place (`handle_travel`), the
handle's shape is pinned so the two cannot drift apart, and the test
**clicks the mark** and checks that the playhead lands on that frame — so the
alignment is measured rather than asserted.

**A key outside the design's range is not drawn.** There is no frame to put it
at, and a mark at the end of the rail would say it was inside the design.
§119's diagnostics name such a key instead.

**The selected node's marks are the accent colour** and the rest are dim, so
the timeline and the properties panel agree about what is selected.

§117's icon — *"three marks on a line: the keyframes of a property, on the
timeline they live on"* — was a sentence about a thing that did not exist.
It is now literal.

### What now holds it

| Property | Held by |
|---|---|
| The timeline marks the keyframes | `the_timeline_marks_the_keyframes` in `craftkit-ui` |
| A keyframe's mark is where the playhead goes when it is clicked | `a_keyframe_mark_is_where_the_playhead_goes_when_it_is_clicked` |
| The rail follows the panel | `the_timeline_rail_follows_the_panel` |
| A key outside the design has no mark on the rail | `a_key_outside_the_design_has_no_mark_on_the_rail` |

and four mutations in `scripts/mutation-test-gates.sh`: the timeline marks no
keyframes; the rail is a stub again; a keyframe's mark is not at its frame;
and a key outside the design is drawn on the rail.

The fourth mutation is the interesting one, because **the first version of it
was missed**. It clamped the mark's place into the rail — and the gate did
not notice, because a key outside the design's range, placed without
clamping, lands *off* the end of the rail and is not drawn at all. The gate
was measuring the wrong thing: it could only catch the implementation that
clamped. The rule was moved into `mark_x` — which clamps, so it can never
answer with a place off the rail — and the mutation was re-aimed at the guard
that keeps such a key off the rail. **A gate that passes for the wrong reason
is a gate that has not been tested.**

### The lesson

**A timeline that does not show the keyframes is a slider.** The name says the
design's timeline; the widget was a value picker a hundred pixels wide.

**The place a person looks for *when* is a place of its own.** The times were
in the model, in a reply and in a panel — three copies of the fact, and none
of them where the question is asked.

**An absence is not a lie, and it is harder to catch.** The comparison that
found this one was the cheapest there is: two designs that differ in one
thing, drawn one after the other, and the pictures compared.

---

---

## 122. The model counted a node's properties the slow way, and every frame paid for it

### The measurement

A node's properties are listed **by path**: the listing is how a command, the
plan and the window all learn what a node holds. §85 made that listing
correct — every property named by exactly one path, and the path resolving
back to the property it names.

It was also, for a node with many properties, **quadratic**. The listing
counted a name's occurrences among its **siblings** by scanning the siblings
it had already seen, so listing *n* properties cost *n* scans of a growing
list. Measured on a document with one node and *n* properties, the same
machine, the same script, the two configurations back to back:

| *n* properties | the listing | the fingerprint | a cold evaluation | the shell draws in (the node selected) |
|---|---|---|---|---|
| 100 | 53.3 µs | 3.7 µs | 126 µs | 585 µs |
| 1 000 | 3.12 ms | 36.8 µs | 6.82 ms | 7.49 ms |
| 5 000 | **83.1 ms** | 194 µs | **171.7 ms** | **103.7 ms** |

Ten times the properties cost thirty-one times the time, and five times the
properties cost twenty-six times the time. That is the shape of a quadratic
thing, and the fingerprint beside it is the tell: the fingerprint hashes the
same tree in **194 µs** because it walks the tree *without building a path
for each property* — and the listing, which builds one, costs four hundred
times as much.

**The evaluation was the listing.** A cold evaluation of that document took
171.7 ms, and the listing inside it took 83.1 ms — twice, because the
evaluator resolves every property's value *and* checks every property's
keyframes for the ones outside the design's range (§119). So **every
command** on a document with a five-thousand-property node spent a third of
a second listing properties.

**And the window paid it on every frame.** The shell draws the properties
panel for the selected node on every frame, and — since §121 — the
timeline's marks for **every node** on every frame. The shell's own drawing
work — the panels, without the rasteriser — took 103.7 ms at five thousand
properties, and 585 µs at a hundred. `egui` repaints only when something
asks it to, so a cost like that is not a slow frame; it is a window that
takes a tenth of a second to answer a mouse.

The model's own limits allow **fifty thousand** properties in one node and
ten thousand nodes in a document. This is a document the model *accepts*.

### Why nothing noticed

**The listing was correct.** §85's tests are about *which* path names *which*
property, and every one of them still passes. A quadratic listing gives the
right answers; it just takes longer and longer to get them.

**The cost is only visible at a size nobody types.** A hand-made plan has a
handful of properties, and the listing is microseconds. The cost appears
where the model's limits are, and the limits are not a thing anybody draws.

**Nothing measured the shape that bites.** §103 measured what a *command*
costs — 23.8 ms on a document at the model's limits — and its document was
**one node with 32 properties and 10 000 keyframes on each**. The cost there
is in the *keyframes*: three hundred and twenty thousand of them, sampled on
every evaluation. The listing of thirty-two properties is nothing. A node
with five thousand *properties* is a different shape, and nobody had built
one.

### The fix

**The counts are kept in a map.** The listing counts a name's occurrences in
a map keyed by the ASCII-lowercased label, so counting a sibling is one
lookup instead of a scan of everything seen so far. The map is only ever
counted *in*, never read in order, so no path can depend on a hash order.

**The timeline's marks do not build a path at all.** The marks want the
*instants*, not the paths — and the model builds a path for every property
it lists, which is a copy of the tree. The window now walks the tree itself,
the way the evaluator's own `is_animated` walks it. On a five-thousand-
property node: **2.18 ms for the listing, 25 µs for the walk.**

After:

| *n* properties | the listing | a cold evaluation | the shell draws in |
|---|---|---|---|
| 100 | 35.6 µs | 91.6 µs | 541 µs |
| 1 000 | 386 µs | 1.02 ms | 4.83 ms |
| 5 000 | **2.18 ms** | **5.74 ms** | **22.7 ms** |

The listing is **linear** now — eleven times the time for ten times the
properties — and **thirty-eight times faster** at five thousand. The
evaluation is **thirty times faster**, and the shell's drawing work is four
and a half times faster.

**In the running application.** The same design — one node, five thousand
properties, eight hundred and ninety kilobytes — opened in the real window,
and ten commands sent over the control channel, each of which changes the
document and so re-evaluates the plan:

| the same ten commands, in the real window | fastest | median | slowest |
|---|---|---|---|
| before | **176 ms** | **189 ms** | 202 ms |
| after | **17 ms** | **20 ms** | 23 ms |

A tenth of a second per command became a twentieth of one, in the
application, on a document the model accepts.

### What now holds it

| Property | Held by |
|---|---|
| Listing a node's properties is not quadratic | `listing_a_node_does_not_cost_more_than_listing_a_smaller_one` in `craftkit-model` |
| A listing of many siblings numbers the duplicates | `a_listing_of_many_siblings_numbers_the_duplicates` |
| A keyframe inside a group is marked on the rail | `a_keyframe_inside_a_group_is_marked_on_the_rail` in `craftkit-ui` |

and three mutations in `scripts/mutation-test-gates.sh`: the listing counts
the siblings the slow way again; the listing forgets a sibling when counting;
and the marks skip the keyframes inside a group.

§85's own mutation — *"the listing counts names exactly while the resolver
counts them loosely"* — was **re-aimed**, because its anchor was the line
this change replaced. The first re-aim was wrong: it looked the label up in
the map without folding the case, which changes nothing for the test's
order (`Amount` first, `amount` second — the first one's case does not
matter). The mutation now folds the label *into* the key instead of folding
the key, which is the same defect in the new shape ✓.

### What is still open

**The properties panel still copies the selected node's properties on every
frame** — the paths *and* the properties, because the panel publishes a name
for each editor. That is linear now, and measured: the shell's drawing work
at five thousand properties is 22.7 ms. What is left in it — the copy, the
five thousand names it publishes, the tree — was not measured separately, and
a node with ten thousand keyframes carries ten thousand of them.

**The evaluator's out-of-range check still lists every property on every
plan**, because the warning names the property's path. Linear now — most of
the 5.74 ms plan at five thousand properties — but it is a path per property
on every plan.

### The lesson

**A correct listing can be quadratic.** §85 made the paths right, and every
test about them still passes; the cost was in *how* the occurrences were
counted, and nothing about the answers changed.

**Measure the shape, not the size.** §103's document had 32 properties and
320 000 keyframes; this one has 5 000 properties and 5 000 keyframes. Both
are at the model's limits, and the costs are in different places: the first
in the keys, the second in the names. A document at one limit is not a
document at another.

**A limit nobody draws is a limit nobody hits.** The model allows fifty
thousand properties in a node; the tree's own tests never built one, and
listing fifty thousand of them took eight seconds before the fix.

---

---


## 123. The properties panel copied the selected node's whole property tree, on every frame, to draw four things

### The measurement

The properties panel shows the selected node: one editor per property, the
number of keyframes the property has, and the times of the first eight of them
(§117). It is drawn on **every frame**.

It also **copied** the node's whole property tree on every frame. The copy took
every property's label, its value and **every keyframe's value** with it — and
the panel drew four things out of it: the path, the value, the count and the
first eight times.

Measured on the panel alone — its own layout, the node selected, the fastest of
nine rounds — with one property whose keyframes carry values:

| the panel alone | 1 keyframe | 100 | 1 000 | 10 000 |
|---|---|---|---|---|
| the values are numbers | 20 µs | 22 µs | 29 µs | 78 µs |
| the values are text | 20 µs | 28 µs | 71 µs | **484 µs** |

The panel's cost grew with the number of keyframes it does not draw: **twenty-
four times** from one keyframe to ten thousand, when the values were text. A
text value is the worst case, because every one of them is a separate
allocation; a number is the best case, because copying it copies nothing but
the bits.

The same defect inside a whole frame — the frame with the node selected minus
the same frame without it, which is the panel's own work. The keyframe values
are text here, which is the worst case for a copy:

| the panel's own cost in a frame | before | after |
|---|---|---|
| one property, 10 000 keyframes | 488 µs | **23 µs** |
| 200 properties, 1 000 keyframes each | 12.1 ms | **1.62 ms** |
| 100 properties, 10 000 keyframes each (a million keyframes) | **51.5 ms** | **0.87 ms** |
| 1 000 properties, one keyframe each | 7.15 ms | 6.87 ms |
| 5 000 properties, one keyframe each | 37.3 ms | 37.8 ms |

The last two rows did not move, and that is the point: the copy is paid in
**keyframes**, and a node with many properties and few keyframes pays it
almost nothing. A node with a million keyframes paid it fifty-one milliseconds
a frame.

### Why nothing noticed

**The copy was correct.** Everything the panel drew was exactly what it drew
before. Nothing about the *output* changed; only the cost of producing it.

**The cost is only visible at a shape nobody draws.** A hand-made plan has a
handful of properties and a handful of keyframes. The copy is paid in
keyframes, and the model's own limits allow ten thousand keyframes on a
property and fifty thousand properties on a node — a document the model
*accepts*.

**§122's own numbers had the answer in them and did not say so.** §122
measured "the shell draws in" at 103.7 ms for a node with five thousand
properties, and the listing inside it took 83.1 ms. The remaining twenty
milliseconds were never explained. They were the copy and the widgets
together, and the copy was the smaller half at that shape — because five
thousand properties with one keyframe each is not where the copy lives.

### The fix

**The panel borrows the tree.** The model's own listing hands out references
(`flatten` returns `Vec<(PropPath, &Property)>`), and the panel draws from the
borrow. The borrow of the session has to end before the panel can run a
command — `Shell::run` takes `&mut self` — and the panel collects its edits
and runs them **after** the loop that draws, so the borrow is over by then.

The panel still reads the keyframe **times** where they are and still draws
the count. It never copies a keyframe's value.

After, the panel alone:

| the panel alone | 1 keyframe | 100 | 1 000 | 10 000 |
|---|---|---|---|---|
| the values are numbers | 19 µs | 20 µs | 20 µs | 21 µs |
| the values are text | 19 µs | 20 µs | 20 µs | **20 µs** |

**Flat.** The panel's cost no longer depends on how many keyframes the node
has.

**In the running application.** A design with two nodes — one whose property
carries three hundred and fifty keyframes of 64 KiB of text each, one whose
property carries none — opened in the real app, and the two selections asked
for over the control channel, **paired back to back** so that everything they
share (the fresh font atlas, the raster, the machine's load at that moment)
cancels:

| the same paired frames, in the running app | the keyed node | the plain node | the panel's own cost |
|---|---|---|---|
| before | 129.6 ms | 112.8 ms | **+15.6 ms a frame** |
| after | 121.3 ms | 119.6 ms | −0.2 ms a frame |
| after, the run again | 120.8 ms | 121.1 ms | −0.5 ms a frame |

The panel's copy cost fifteen and a half milliseconds a frame in the real
application. After the fix there is nothing left to measure.

### What now holds it

| Property | Held by |
|---|---|
| The properties panel does not copy the keyframes it does not draw | `the_properties_panel_does_not_copy_the_keyframes_it_does_not_draw` in `craftkit-ui` |
| The properties panel does not copy the keyframe values | `the_properties_panel_does_not_copy_the_keyframe_values` |
| The panel publishes one editor for each property and no other | `the_panel_publishes_one_editor_for_each_property_and_no_other` |

and three mutations in `scripts/mutation-test-gates.sh`: the properties panel
copies the properties again; the properties panel copies the keyframes to draw
their times; and the panel publishes a property's label instead of its path.

The first two were measured against the gates that must catch them: the copy
restored makes the panel **25.2 times** as slow with ten thousand keyframes as
with one, and the keyframes copied to draw their times makes it **162.2 times**
as slow with 64 KiB values as with one byte. Both gates refuse anything above
eight times, so both are caught with room to spare.

### The lesson

**A copy that is never read is a copy that is always paid for.** The panel
copied every keyframe's value to draw a count and eight times.

**Measure the marginal cost.** The panel's own work is the difference between
a frame with the node selected and one without it; measuring a frame on its
own measures the whole window.

**A shape nobody draws is a shape nobody hits.** The model's limits allow a
node with a million keyframes on it, and no test — and no hand-made plan —
has ever made one.

Two things this section measured are **not** fixed and are written up as §125:
the panel lays out a widget for every property of the node, and the timeline
walks every keyframe of every node, on every frame.

---

## 124. A locked property was not locked: five of the six commands that write one ignored the lock

### The measurement

`Property::locked` says:

> Whether a user may change this. A locked property is greyed in the UI and
> refused by `property.set`, so the rule lives in the document rather than in
> every caller.

Six commands write a property: `prop.set`, `prop.reset`, `key.set`,
`key.remove`, `key.clear` and `key.ease`. **One** of them asked the lock.

Measured, on a node whose only property is locked and carries one keyframe,
each command asked to write it in turn — each on a fresh document, and at an
instant where the command **would** have worked:

| the command | before the fix | the property afterwards |
|---|---|---|
| `prop.set` | refused: "`amount` is locked" | unchanged |
| `key.set` | **ok: "keyed `amount` at 0.008s"** | **the value changed from 0.5 to 7.0**, and a second keyframe appeared |
| `key.remove` | **ok: "removed the key on `amount`"** | **the keyframe is gone** |
| `key.clear` | **ok: "cleared 1 keyframe(s)"** | **the keyframe is gone** |
| `key.ease` | **ok: "`amount` eased hold"** | **the easing changed** |
| `prop.reset` | **ok: "removed 1 keyframe(s) from `amount`"** | **the keyframe is gone** |

The lock was a word in a comment and a `bool` in the model. A property the
document says a person may not change could have its **value** changed — by
`key.set`, which sets the stored value as well as the key — and its animation
changed four ways. Every one of the five reported success.

And the first half of the sentence was false as well: `crates/ui` never read
`locked` at all, so the panel drew a locked property exactly like an ordinary
one — in full colour, with an editor that could be dragged, and published it to
the automation registry as an ordinary editor that could be clicked.

### Why nothing noticed

**Nothing could turn the lock on.** No command sets it. Only a file can carry
`"locked": true`, and only the model's own `Property::locked()` builder can
turn it on. So in practice a lock was something a file brought with it, and
nothing in the tree ever looked at it again.

**The one caller that did check was not tested either.** `prop.set` refused a
locked property and said so, and **no test anywhere** asserted it. The
model's own test — `a_locked_property_is_marked_so_the_ui_and_the_engine_agree`
— checked only that the flag is set. Its name states the *purpose* of the flag,
and the purpose was not real: the engine did not agree with itself, and the UI
was not in it at all.

**The rule was in every caller, and there was one caller.** The sentence says
the rule lives in the document "rather than in every caller" — and the code
had the check inline in one command, so a second command that wrote a property
had no reason to ask.

**The UI could not have shown it.** The panel had no way to know a property
was locked: it never read the flag. So a person looking at the screen had no
warning either, and an agent reading the registry was told the editor could
be clicked.

### The fix

**The rule lives in the model, and there is one door.** The model gained

```rust
pub fn find_writable(&mut self, path: &PropPath) -> Result<Option<&mut Property>, CoreError>
```

which resolves a path for writing and **refuses a locked property**. Every
command that writes a property comes through it — `prop.set`, `prop.reset`,
`key.set`, `key.remove`, `key.clear`, `key.ease` — so a command cannot change
a locked property by forgetting to check. The refusal names the property and
says it is locked, and `prop.set`'s message is **unchanged**:

> `prop.set` rejected its parameters: `path` `amount` is locked

**The panel greys a locked property.** Its editor is drawn disabled, and the
registry publishes it as disabled — so an agent asking "what is at this point?"
is told it cannot be clicked (§107's rule, applied to a lock). The "clear
keys" button is greyed as well, because `prop.reset` refuses a locked
property.

**The sentence is now true in full.** It said a locked property is *greyed in
the UI* and *refused by `property.set`*, and that the rule lives *in the
document rather than in every caller*. Both halves were false before: the UI
had never heard of the flag, and the rule was in one command's body. Both are
true now, and the last clause is true in the strictest reading — the rule is
in the model, and every command that writes a property comes through the one
door that enforces it.

After, all six commands refuse and the property is untouched:

| the command | after the fix | the property afterwards |
|---|---|---|
| `prop.set` | refused: "`amount` is locked" | unchanged |
| `key.set` | refused: "`amount` is locked" | unchanged |
| `key.remove` | refused: "`amount` is locked" | unchanged |
| `key.clear` | refused: "`amount` is locked" | unchanged |
| `key.ease` | refused: "`amount` is locked" | unchanged |
| `prop.reset` | refused: "`amount` is locked" | unchanged |

**In the running application.** A design with one node whose `amount` property
is locked, opened in the real window, and the node selected so the properties
panel was open. The control channel's own answer — the registry's list of the
widgets on the screen that **cannot** be clicked — before and after:

| the widgets the channel names as disabled | before | after |
|---|---|---|
| `node.1.prop.amount` | — | **`node.1.prop.amount`** |
| `node.1.keys.amount.clear` | — | **`node.1.keys.amount.clear`** |

Before, the only disabled widgets in the window were the toolbar's undo and
redo. A locked property's editor was published as an ordinary one that could
be clicked.

And on the screen, the slider for the locked `amount` and its "clear keys"
button are drawn **greyed**, while the ordinary `enabled` checkbox beside them
is not:

| the properties panel | before | after |
|---|---|---|
| the `amount` slider's handle | full colour | **greyed** |
| the "clear keys" button | full colour | **greyed** |
| the `enabled` checkbox | full colour | full colour — it is not locked |

### What now holds it

| Property | Held by |
|---|---|
| A locked property may not be written | `a_locked_property_may_not_be_written` in `craftkit-engine` |
| A locked property's editor is published as disabled | `a_locked_property_s_editor_is_published_as_disabled` in `craftkit-ui` |
| A locked property is drawn greyed | `a_locked_property_is_drawn_greyed` |

and three mutations in `scripts/mutation-test-gates.sh`: the model hands a
locked property out without asking; the panel publishes a locked property's
editor as enabled; and the panel draws a locked property's editor in full
colour.

The engine gate walks **all six** commands, each on a fresh document and at an
instant where it would have worked, so one mutation — the model forgetting the
lock — is caught by the one gate that covers all six writers. The two UI gates
are aimed apart: the registry's flag and the colour on the screen are two
different facts, and each mutation breaks one and not the other.

### What is still open

**Nothing can turn the lock on.** `Property::locked()` exists, a file can carry
`"locked": true`, and the flag is in the document's fingerprint — but no
command sets it. Whether a command should be able to lock a property, and
which ones, is a decision nobody has written down.

### The lesson

**A rule in every caller is a rule in no caller.** The sentence said the rule
lives in the document; the code had it in one command, and five other commands
that wrote the same thing had no reason to ask.

**A flag nobody reads is a flag that is not there.** `locked` was in the model,
in the file format and in the fingerprint — and the UI had never heard of it.

**The lock a property carries is the lock the document enforces.** `key.set`
changed a locked property's *value*, which is the one thing a lock is for, and
it did so while reporting success.

---

## 125. Two more gaps the properties panel was hiding — open

Fixing §123 measured two things that are **not** fixed. Each of them is in
the code this section touched, and each is recorded here rather than left in a
comment.

§126 came out of the same work and closed a **third** thing this section did
not mention: the panel had no scroll area at all, so the properties that did
not fit were not on the screen and could not be reached — and the "key at the
playhead" editor was drawn below the list, off the screen as well. What is
written below is what §126 did **not** close.

§127 came out of the same work too, and it is the **other panel**: the node
list asked the evaluator about every node, one node at a time, and the plan
answers by scanning — so the list's rows were collected quadratically, 43.7 ms
of every frame at the model's own limit of ten thousand nodes. The properties
panel and the node list are two halves of the same question — *what does this
panel cost as the document grows* — and only one of them was being measured.

### The properties panel drew every property of the selected node, on every frame

`egui` lays out a widget per property, and the panel asks for one per property
of the node. A node with five thousand properties therefore lays out five
thousand widgets a frame: **37.8 ms**, measured, and unchanged by §123
because the copy was not what that cost.

The fix is a virtualised list — draw the rows that are on screen and no more —
and it is a real piece of work: the panel's rows are the model's own paths
(§85), and a row's height depends on whether the property is animated and how
many widgets it has. A gate for it would have to measure the panel's cost
against the number of *properties*, the way §123's measures it against the
number of keyframes.

### The timeline walked every keyframe of every node, on every frame

The marks want the instants (§122), so the window walks the tree itself — and
it walks **every** key of **every** node, on every frame, and paints a mark
for each. A document with a million keyframes: **29 ms a frame**, measured as
the frame with nothing selected, so it is paid whether or not a node is
selected. The copy §123 removed was fifty-one milliseconds on top of that.

The marks are drawn on a rail a few hundred pixels wide, so a million of them
are a million of something nobody can see. The fix is to draw the marks that
are **near the playhead** and let the rest go — but "near" is a rule nobody
has written down yet, and it changes what the timeline means.

---

## 126. The properties panel had no scroll area: four thousand of five thousand properties were nowhere to be seen

### The measurement

A design with one node carrying five thousand properties, opened in the real
window with the node selected, so the properties panel was open on the screen.
The registry's own answer — the names and rectangles of the widgets the panel
published:

| what the registry said | before |
|---|---|
| property editors named | **5 002** |
| the first editor's top | y = 20 |
| the last editor's top | **y = 325 111** |
| editors below the window's bottom edge (y > 720) | **4 991** |

And the screen itself: the panel showed `p0`, `p1`, `p2` … `p8` and then ran
off the bottom of the window. There was no scroll bar on it, because there was
no scroll area in it. The **node list** has had a scroll area since the
beginning; the properties panel never got one.

Two things were unreachable, and the second one was not even on the screen:

- the four thousand nine hundred and ninety-one properties that did not fit;
- the **"key at the playhead" editor**, which was drawn *after* the list and
  was therefore laid out below the panel's bottom edge — off-screen, in every
  document with more properties than fit.

The registry did not know any of this. It named all five thousand and two
editors, every one of them with a rectangle, and an agent asking for
`node.1.prop.p4999` was answered with a rectangle at y = 325 111 — outside the
window, on a panel that cannot scroll.

### Why nothing noticed

**The panel was always given a node with a handful of properties.** The sample
design's nodes carry one or two, so the list fitted, and the missing scroll
area never had anything to hide.

**The registry counts names, and every name was there.** §123 made the panel
publish one editor per property, and §123's own gate checks exactly that — one
editor for each property, and no other. A panel that publishes five thousand
editors and shows nine of them passes that gate perfectly. The gate asked
"does the panel name every property?", and the answer was yes; nobody asked
"can anybody *reach* them?".

**The control channel draws its own frame.** `ui {"action":"widgets"}` renders
through `craftkit_ui::raster::render`, in a context of its own. So the
registry's rectangles are the ones a fresh frame would lay out — with the
scroll at the top, always — and they never showed the window's own scroll
position. Measuring the scroll through the registry was measuring a different
screen.

**A scroll area in a panel is not the same as a scroll area in a window.**
egui's scroll area answers the wheel only when the pointer is inside its
rectangle, and — this is the part that cost the most time to find — a scroll
area **shrinks to the width of its content** unless it is told not to. In a
headless frame, a scroll area whose content was 39.3 points wide had an outer
rectangle 39.3 points wide; a pointer at x = 150 was outside it, so the wheel
did nothing at all. With the pointer over the content, the same frame scrolled
(offset 15.9, then 42.7, then 76.9). Every measurement that said "the wheel
does nothing" was measuring a pointer that was not over the thing.

### The fix

**The rows live in a scroll area, and the scroll area fills the panel's
width.** `crates/ui/src/shell.rs` draws the property rows inside

```rust
ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| { … })
```

`auto_shrink([false, true])` says: do not shrink across, shrink down. The
scroll area is then as wide as the panel, its scroll bar is at the panel's
edge, and the wheel answers anywhere across the list — not only over the
editor of one property.

**The key editor moved above the list.** A scroll area takes the rest of the
panel, so a control drawn after it is laid out below the panel's bottom edge.
The "key at the playhead" editor is now drawn between the node's name and the
list, where it stays put while the list scrolls.

**The rows still borrow the tree** (§123): they are moved *into* the scroll
area's closure, so the borrow of the session ends with the closure, and the
commands the panel runs afterwards can still run.

### In the running application

The same design, the same window, the same node — with the wheel turned over
the properties panel in between the two pictures:

| the properties panel | before | after |
|---|---|---|
| the list shows | `p0` … `p8`, then off the bottom | `p15` … `p22` |
| a scroll bar on the panel | **none** | **one**, at the panel's edge |
| the "key at the playhead" editor | **not on the screen** | **at the top of the panel**, while the list scrolls |

The screenshots are the evidence: `/tmp/probe126-live-after-before.png` and
`/tmp/probe126-live-after-after.png`, cropped to the panel in
`/tmp/probe126-panel-before.png` and `/tmp/probe126-panel-after.png`.

### What now holds it

| Property | Held by |
|---|---|
| A property that does not fit can be brought into view | `the_properties_panel_scrolls_to_the_properties_that_do_not_fit` in `craftkit-ui` |
| The wheel scrolls the list from anywhere across the panel | `the_wheel_scrolls_the_list_from_across_the_panel` |
| The key editor stays on screen when the node has many properties | `the_key_editor_stays_on_screen_when_the_node_has_many_properties` |

and three mutations in `scripts/mutation-test-gates.sh`: the panel's rows do
not scroll; the panel's scroll area shrinks to its content; and the panel
draws the key editor after the list.

The first two are aimed at the two ways the wheel can fail to reach the list —
no scroll area at all, and a scroll area that is narrower than the panel — and
the third at the layout rule that a scroll area takes the panel.

### What is still open

**The list is still drawn in full, every frame.** A scroll area clips the
*painting*, not the *layout*: five thousand rows are still laid out, and the
panel still costs what §125 measured. This section made the properties
reachable; it did not make them cheap. §125's first item is still open.

**The registry still names the properties that are not on the screen.** An
agent asking for `node.1.prop.p4999` is answered with a rectangle outside the
window. The answer is not a lie — the widget is there, in the panel, and the
window's height is in the answer too — but the agent cannot scroll the panel,
because `ui.click` reports and does not act (§17). What an agent should be
able to do with a list that does not fit is a decision nobody has written
down.

### The lesson

**A list that does not fit is a list that is not there.** The panel published
five thousand and two editors and showed nine of them. Nothing in the code
was wrong except that nothing could move.

**A scroll area is not a frame of its own; it takes the panel.** The rule that
caught the key editor is not about scroll areas at all: it is that a control
drawn after one is laid out below the bottom edge. The panel that had the
scroll area was the panel that lost its key editor.

**A gate that counts names cannot see reachability.** "One editor for each
property, and no other" was true, and it was satisfied by a panel that showed
nine of five thousand. The gate that would have caught this asks a different
question: can the eleventh be *seen*.

**Measure the pointer, not the wheel.** Four hours went into "the wheel does
not scroll this" because the pointer was 100 points to the right of the thing
it was meant to be over, and the scroll area was as narrow as its content.
egui's own source says it in one line — `is_hovering_outer_rect` — and the
measurement had to find it.

---

## 127. The node list asked the plan about every node, one node at a time

### The measurement

The node list draws one row per node in the document. Each row shows the
node's **level** — the signal level leaving it — and that level is the
**evaluator's** answer: `Plan::state(id)` is where the plan holds it.

`Plan::state` is a `find`:

```rust
pub fn state(&self, id: NodeId) -> Option<&NodeState> {
    self.nodes.iter().find(|s| s.id == id)
}
```

So the list asked the plan about **every node in the document**, one at a
time, and each answer was a scan of the plan's own nodes. Collecting the
list's rows was therefore **quadratic in the number of nodes** — and the
model's own limit is ten thousand nodes.

Measured in the running process, the cheapest of nine rounds, at the sizes the
model allows:

| nodes | rows collected, the old way | rows collected, the new way |
|---|---|---|
| 100 | 14 µs | 11 µs |
| 1 000 | 304 µs | 112 µs |
| 5 000 | **9.19 ms** | 589 µs |
| 10 000 | **43.7 ms** | **1.21 ms** |

The old column multiplies by a hundred when the nodes multiply by ten; the
new one multiplies by ten. At the model's own limit the old way cost
**thirty-six times** what the new one costs.

The panel draws all of them, so the whole node list cost **95.1 ms** a frame
at ten thousand nodes, of which 43.7 ms was this and the rest was egui laying
out ten thousand rows (§125's first item, for the *other* panel).

### Why nothing noticed

**The sample document has three nodes.** Three nodes is a scan of three
entries, thirteen times over, and the difference between 14 µs and 11 µs is
not something a person can see.

**Nothing was wrong with the answer.** The level each row carried was always
the plan's level. The code was not slow because it was wrong; it was slow
because it was right in a way that cost a scan per row.

**The panel was measured, but the panel is not where the cost was.** The
node list's *drawing* — egui's layout of ten thousand rows — is the bigger
number, and it is linear. Measuring the panel and seeing "95 ms" says
nothing about which part of the panel that was; the finding only came from
splitting the panel's cost into **collecting** and **drawing** and timing
them apart.

**The plan's own API does not say what it costs.** `Plan::state` reads like a
lookup and is a scan. A caller that uses it in a loop — which is the obvious
way to use it — pays a scan per iteration, and the type says nothing about
that.

### The fix

**The levels are found once.** `crates/ui/src/shell.rs` now collects the
list's rows with a free function:

```rust
pub fn node_rows(
    document: &craftkit_model::Document,
    plan: Option<&craftkit_eval::Plan>,
) -> Vec<NodeRow>
```

The plan is walked **once**, into a map from node id to level, and every row
then costs one lookup. The rows still come from the **document**, not from the
plan: the list shows the document's nodes, and a plan that has fallen behind
the document must not hide a node or invent one.

The function is free rather than a method for the same reason `draw_level_bar`
is: it is a pure function of the document and the plan, and a pure function
can be **measured on its own**. That is how the cost was found — and it is
how the gate now holds it down.

### In the running application

The same design — ten thousand nodes — open in the real application, and the
application's own render measured over the control channel (`ui.render`, which
runs the same `craftkit_ui::frame` the window's own frame runs):

| the running application | before | after |
|---|---|---|
| `ui.render` with the **sample** document (3 nodes) | 11.9 ms | 12.5 ms |
| `ui.render` with **ten thousand** nodes | **341.7 ms** | **209.0 ms** |
| what the node list costs, the difference | **≈ 330 ms** | **≈ 197 ms** |

The small document is the control: it says the socket, the picture and the
rest of the shell cost about twelve milliseconds either way, so the whole
difference is the node list. Two runs of the "after" build gave 212.3 ms and
209.0 ms.

The difference is larger in the application than in the probe (130 ms against
43.7 ms), and the reason is the cache: in the old code the scan of the plan
was interleaved with cloning ten thousand names, so the plan was evicted
between scans and every scan went to memory. The probe collected the rows in
a tight loop, where the plan stayed in cache.

The screenshots are the evidence: `/tmp/probe127-live-big-before.png` and
`/tmp/probe127-live-big-after.png`. **They are the same picture** — and that
is the point. Nothing about the screen changed; what changed is how long it
takes to get there. A performance fix has no other visible sign.

### What now holds it

| Property | Held by |
|---|---|
| The node list carries the level the plan resolved | `the_node_list_carries_the_level_the_plan_resolved` in `craftkit-ui` |
| The node list does not scan the plan once per node | `the_node_list_does_not_scan_the_plan_once_per_node` |
| The node list draws the rows the collector found | `the_node_list_draws_the_rows_the_collector_found` |

and two mutations in `scripts/mutation-test-gates.sh`: the node list asks the
plan once per node again, and the node list shows every level as zero.

The first mutation is aimed at the cost gate and the second at the
correctness gate, and they are aimed apart on purpose: the scan-per-node
mutation still returns **the right levels**, so a gate that only checked the
levels would not notice it. The gate that notices it measures the
collection against the way it was done before, **in the same run, on the same
machine**: the fast way has to be at least four times cheaper than the scan,
and it is thirty-six times. §134 moved that threshold down, from eight:
eight was a number read off a quiet machine, and the gate failed on a busy
one.

### What is still open

**The list is still drawn in full, every frame.** The fix made the
collection cheap; the drawing is unchanged, and it is the larger half: at
ten thousand nodes the panel still lays out ten thousand rows. The fix took
the node list from 95 ms to 52 ms in the probe; the rest is egui's layout,
and only a virtualised list will remove it. §125's first item is the same
problem in the other panel.

**The plan's own API still costs a scan per question.** `Plan::state` is
still a `find`. This section fixed the one caller that asked it about every
node; the next caller will not know. The honest fix is a `HashMap` inside
the plan — but the plan is a **derived value** that is thrown away and built
again on every evaluation, and a map of ten thousand entries on that path is
a cost paid on every command to save one on every frame. That trade is a
decision nobody has written down.

**The level is not observable to an agent.** The level bar's **track** is
published (§99) and the track is always sixty-four points wide whatever the
level is, so the registry cannot say what level a node is at. An agent can
get it from `plan.evaluate`, which returns the whole plan — so nothing is
lost — but the screen and the registry disagree about how much is on the
screen.

### The lesson

**A lookup that reads like a lookup is not a lookup.** `Plan::state(id)` has
the shape of an indexed read and the cost of a scan. Nothing in the signature
says so, and the type that returns it cannot say so either.

**Quadratic costs hide behind linear ones.** The node list's cost was
dominated by egui's layout, which is linear and which everybody assumed was
"the cost of the list". The quadratic part was 46% of the total — enough to
matter, small enough to be lost in the noise of the bigger number. Splitting
a cost into its parts and timing them apart is what found it.

**Measure the whole of the thing, not the thing you can see.** The obvious
measurement — "how long does the panel take" — pointed at egui and found
nothing. The measurement that found it was of the panel's *input*, not its
output.

**A pure function is a gift to the next person.** Splitting the collection
out of the panel was not tidiness: it is what made the cost measurable, and
therefore what made it fixable, and therefore what made it holdable.

---

## 128. The node list's scroll area shrank to its content, and the wheel did nothing over the panel's margin

### The measurement

§126 found that egui's scroll area **shrinks to the width of its content**
unless it is told not to, and fixed the properties panel with
`auto_shrink([false, true])`. The node list has the same shape and never got
the same treatment.

Measured in a headless frame, with the node list drawn in a panel of a width
the test knows (240 points) and forty nodes in it:

| | |
|---|---|
| the panel's width | 240 points |
| the content reaches | x = 214.8 |
| the wheel at x = 234 — over the panel, over no row | **the list did not move** (y = 35.0 → 35.0) |
| the wheel at x = 40 — over the content | the list **scrolled** (y = 35.0 → −98.0) |

So the scroll area was narrower than the panel, and the strip between the
content and the panel's edge was dead to the wheel.

### Why nothing noticed

**The node rows are wide.** Every row carries a sixty-four-point level bar
and two buttons, so the content reaches 215 of the panel's 240 points. The
dead strip is about twenty-five points wide in that arrangement — and in the
**real window** it is two or three, because the panel is wider there and the
rows are as wide as they are. The wheel happens to work over almost the whole
panel, so a person never finds the strip.

**The properties panel's content was narrow.** A property editor is 39
points wide (§126), so the same bug left a 250-point strip of that panel
dead — and §126 found it there. The defect is the same; how much of the
panel it kills depends on how wide the content happens to be.

**The scroll bar is not where a scroll bar is.** Shrunk to its content, the
scroll area's scroll bar is drawn at the content's edge, a few points inside
the panel. Nothing in the screen says that is wrong.

### The fix

**`crates/ui/src/shell.rs` now says `auto_shrink([false, true])` for the node
list too** — do not shrink across, shrink down. The scroll area is then as
wide as the panel, its scroll bar is at the panel's edge, and the wheel
answers anywhere across the list.

### What now holds it

| Property | Held by |
|---|---|
| The wheel scrolls the node list from anywhere across the panel | `the_wheel_scrolls_the_node_list_from_across_the_panel` in `craftkit-ui` |

and one mutation in `scripts/mutation-test-gates.sh`: the node list's scroll
area shrinks to its content.

The mutation's anchor had to be made unique when the fix landed: §126's
mutation for the properties panel was written against a line that the node
list now shares, and `swap` replaces the **first** match — so §126's case
would have gone on mutating the wrong panel and stopped biting. Both anchors
carry the line above them now, and both cases are caught.

**The mutation suite is what noticed.** The first full run after the fix
reported:

```
1 of 281 gate(s) MISSED their mutation:
   • ui: the properties panel's rows do not scroll
```

That case swaps the scroll area for `ScrollArea::neither()` — and with the
node list's scroll area carrying the same text and coming first in the file,
the swap had landed on the **node list**. The properties panel was untouched,
its gate passed, and the case was silently testing nothing. The suite's
report is the only thing that said so: the gate itself cannot tell that it
is no longer being aimed at the panel it is about.

### What is still open

**The problems panel has the same defect, and it is the worst case of the
three.** The diagnostics are much narrower than the central panel, so a
scroll area that shrinks to them answers the wheel only over the text.

Measured in the real window, with a design of ten thousand nodes and the
wheel turned at a series of points across the central panel:

| the wheel at | the problems list |
|---|---|
| x = 300 | moved |
| x = 400 | moved |
| x = 500 | did not move |
| x = 600, 700, 800, 900 | **did not move at all** |

The central panel runs from 236 to 1010, and the diagnostics end at about
530. So the wheel was dead over **most of the panel** while a list of ten
thousand problems sat there.

**It is not fixed here, and the reason is the honest one.** The problems
list publishes nothing, so there is no name to look up and no rectangle to
compare; and the shape of a frame settles over several frames in ways that a
behavioural comparison cannot tell apart from a scroll — two attempts to
write a gate for it produced a gate that passed for the wrong reason (it was
matching the *status bar's* clip rectangle, which is wide for an unrelated
reason) and a gate that passed with the fix removed. A gate that cannot fail
is worse than no gate, and a change nobody can observe is a change nobody
can vouch for.

So the fix waits for the list to be **observable**: the problems panel would
have to publish its rows, which is a small change and one that also answers
what an agent may read off the screen.

### The lesson

**The same defect can be glaring in one panel and invisible in another.** The
properties panel's content was 39 points wide and the node list's is 215, so
the identical mistake cost a person 250 points of dead panel in one place and
two points in the other. "Nobody noticed" is not evidence that a defect is
small.

**A fix that changes the text of a line can change what a mutation bites.**
§126's mutation was aimed at a line the node list now shares, and the
mutation script replaces the first match — so the case would have gone on
mutating the wrong panel and stopped catching anything, and the run would
have said "caught" while testing nothing. The anchor is part of the case.

**A gate that cannot fail is worse than no gate.** Two attempts to write one
for the problems list passed while the fix was removed. The second one passed
because the frame's layout settles over several frames, and a comparison
between two consecutive frames cannot tell a settling layout from a scroll.

---


## 129. The evaluator's cache kept the nodes' states and dropped their problems, so the problems panel emptied itself the moment the playhead moved

### The measurement

The evaluator keeps the last evaluation and reuses the states of nodes that
have not changed (§53, §80). The **problems** a node reports were not part of
that cache: they were produced as a side effect of evaluating the node, and a
node that was reused was not evaluated, so it reported nothing.

In the evaluator itself, with five nodes that carry no `amount` property — a
warning each — the same document evaluated twice:

| | reused | evaluated | problems |
|---|---|---|---|
| the first evaluation | 0 | 5 | **5** |
| the second, at another instant | 5 | 0 | **0** |
| the third, at another instant | 5 | 0 | **0** |

And in the running application, with a design of ten thousand nodes, none of
which carries an `amount` property:

| | when the file opens | after `playback.seek` to 1.000 s |
|---|---|---|
| the status line | `0t at 0.000s — 0 reused, 10000 evaluated, 10000 problem(s)` | `254016000000t at 1.000s — 10000 reused, 0 evaluated, 0 problem(s)` |
| `diagnostics.list` over the control channel | 10 000 problems (256 listed, 9 744 dropped) | **0 problems** (0 listed, 0 dropped) |
| the problems area of the screen | 6 911 lit pixels | **0 lit pixels** |

The last row is the whole defect in one number: the panel does not empty, it
**disappears**. The Problems header and the list are drawn only when the plan
has diagnostics, and the plan had none.

With the fix in place, the same three runs give 10 000 → 10 000 problems and
6 911 → 6 911 lit pixels.

### Why nothing noticed

**The first evaluation always reports.** The cache is empty when a file opens,
so the problems are on the screen exactly when somebody opens the design and
looks at it.

**A document edit hides the defect, because the reuse is a prefix reuse.**
Muting the first node of the ten-thousand-node design re-evaluates everything
after it — the level entering a node comes from the node before it, so a change
anywhere in the prefix invalidates the rest. Measured: the problems area kept
**11 276 → 11 281** lit pixels. The obvious test — change the design, look at
the problems — finds nothing at all. The thing that has to happen is a change
that does **not** touch the document: the playhead moving.

**The playhead is what a person moves.** Scrubbing is the most ordinary thing
in the editor, and it is the one thing that produces a reuse.

**The status line reads like good news.** `10000 reused, 0 evaluated` is the
cache working; the `, 0 problem(s)` at the end of the line is the lie, and it
is at the end of a line nobody reads twice.

**The count on the screen does not change.** The plan keeps at most
`MAX_DIAGNOSTICS` = 256 diagnostics and counts the rest as dropped, so a
design with ten thousand problems and a design with two hundred and fifty-six
show the same eleven lines. Only the summary line carries the real number, and
§129's defect is in the number.

### The fix

**`crates/eval/src/evaluate.rs` — the cache keeps the problems with the
state.**

- `CacheEntry` carries a `diagnostics: Vec<Diagnostic>` beside its state.
- `eval_node` returns `(NodeState, Vec<Diagnostic>)` instead of pushing into a
  shared list, so a node's problems travel with its state.
- A reused node puts its cached problems back into the plan.
- The cache is rebuilt from the per-node lists, so a node that was evaluated
  keeps its problems for the next reuse.

The two documents that described the cache were rewritten with it, because the
rule in item 6 of "Suggested order" is *a change to a mechanism re-reads the
documents that describe it*: `CacheEntry`'s own doc comment said it was *"one
cached node state"*, and `docs/architecture.md` said an entry held *"the node's
fingerprint and whether it is animated"*. Both were true when they were written
and false the moment the entry gained a third field.

### What now holds it

| Property | Held by |
|---|---|
| A reused node still reports its problem | `a_reused_node_still_reports_its_problem` in `craftkit-eval` |

and one mutation in `scripts/mutation-test-gates.sh`: *eval: a reused node
repeats its problems*, which deletes the loop that puts the cached problems
back. It compiles, and the gate fails with *"the reused node did not report its
problem again, so the plan says the design is fine: []"*.

**What the fix costs.** The plan has to put ten thousand problems back
together on every warm evaluation. Measured on the ten-thousand-node design,
the best of thirty warm evaluations was **5.42 ms and 5.17 ms** with the fix
and **6.01 ms and 5.15 ms** without it — the difference is inside the
run-to-run spread, so the cost is not measurable against a warm evaluation that
already costs five milliseconds.

### What is still open

**The problems panel publishes nothing** (item 23): an agent cannot see the
problems on the screen, and `diagnostics.list` gives it at most 256 of them
and a count of the rest. An agent that asks "what is wrong with this design?"
is told the first 256 problems and that there are 9 744 more it cannot see.
Whether it should be able to page through them is not decided.

**The cache's key is the node's own content, so the problems have to fit
inside it.** A node's problems must depend on the node alone, because the
fingerprint covers the node alone. It is true today: `amount_of` reads the
node's own properties. It is not written down anywhere and nothing checks it,
and a diagnostic that depended on the level coming in, or on another node,
would be reported once and then reused — the same defect, one layer down.

**The prefix rule decides which defects a person ever sees.** The reuse stops
at the first node that changed, so a problem in a document that is edited near
its front is re-reported on every command, while a problem in a document that
is only scrubbed is reported once. That is a consequence of the rule in §53 and
is not wrong, but it means the *frequency* of a wrong answer depends on where
in the document the edit lands.

### The lesson

**A side effect is part of the answer.** The cache kept what the evaluation
*computed* and dropped what it *said*. The state of a node is a number; the
problems of a node are what the design is telling the person about itself, and
a design whose ten thousand warnings vanish the moment the playhead moves is
not a design with no warnings — it is a design the editor has stopped reading.

**The obvious test found nothing, and the obvious test was the wrong one.**
"Change the document and look at the problems" is what a person reaches for,
and the prefix reuse makes it useless. The defect needed an operation that
changes *nothing* — moving the playhead — and the operation that reveals
problems is not the one anybody would think to try.

**The number on the screen is not the number in the design.** The panel draws
at most 256 rows, so "10 000 problems" and "256 problems" look the same; only
the summary line carried the truth, and it was at the end of a line about the
cache.

---

## 130. The document every agent reads first named a module this tree has never had

### The measurement

`AGENTS.md` is the file every agent is told to read first, and it carried this
rule:

> **One set of camera controls.** Any new canvas calls
> `navigation::handle`-style handling with its own caps; never hand-roll camera
> input.

This tree has no canvas, no camera and no `navigation` module:

| | |
|---|---|
| `mod navigation` anywhere in the tree | none |
| a canvas or a camera in the tree | none — the window draws a node graph, a timeline and a properties panel |
| the modules the tree does declare | 50 |
| the crates the tree uses | 13 of its own, 19 dependencies |

So the rule pointed at a module that was never built here. The sentence had
travelled with the scaffold: the rule is true of a 3D application and false of
a node-graph editor, and the scaffold is the second.

Sweeping every document the tree points at code from — `docs/`, `skills/`,
`AGENTS.md`, `README.md`, `CONTRIBUTING.md` — for a backticked `a::b` whose
first part is module-shaped:

| | |
|---|---|
| names the sweep found | 6 |
| of those, the toolchain's own (`clippy::unwrap_used`) | 5 |
| **of those, a module this tree does not have** | **1** — `navigation::handle` |

One, and it was in the file every agent reads first.

### Why nothing noticed

**The two gates that check prose do not see this shape.**
`symbol_file_claims` needs the symbol to be followed by a `*.rs` path — "the
`x` test in `f.rs`" — and the document-links gate needs a `.md`. A bare
`a::b` is neither, so the one dangling module path in the tree sat in the one
document that is read first.

**The rule was about a feature the tree does not have.** Nobody looking for
camera controls in a tree with no camera would conclude that the document was
wrong; they would conclude that they were looking in the wrong place.

**A rule about a canvas that does not exist cannot fail.** There is nothing to
break, so there was nothing for a gate to notice.

### The fix

**The rule now says what is true of this tree.** `AGENTS.md` says that this
tree has no canvas and no camera, that if a canvas is added it must have
**one** set of camera controls — one module that owns them, with its own caps
— and that every canvas must go through that module rather than hand-rolling
its own input. The guidance survives; the name that was not there is gone.

**And the shape is gated.** `cargo xtask lists` now checks every backticked
module path in the prose: the first part must be a module the tree declares, a
crate the tree uses, the standard library, a tool, or a primitive type. A
recipe may propose a name that does not exist yet, and that exemption is keyed
to the **file** — the same shape as the command ids' exemption — so an
invented name anywhere else is still caught.

The gate found the defect on its first run, before the rule was rewritten:

```
error: 1 shared-list problem(s):
   • AGENTS.md: line 140 says `navigation::handle`, and `navigation` is not a module or a crate this tree has
```

### What now holds it

| Property | Held by |
|---|---|
| A module path in prose names a module this tree has | `a_module_path_in_prose_that_names_nothing_is_reported` in `xtask` |

and one mutation in `scripts/mutation-test-gates.sh`: *prose: a module path
that names a module this tree does not have*, which puts `navigation::handle`
back into `AGENTS.md`. Proven to fail without the fix — the gate reports the
line, the name and the reason:

```
AGENTS.md: line 142 says `navigation::handle`, and `navigation` is not a module or a crate this tree has
```

### What is still open

**The rule that the UI crate is swappable has nothing behind it.**
`AGENTS.md` says `crates/ui` "depends on `egui` and nothing else — no
windowing, no renderer — which is what lets the same panels run in a window,
headlessly, and in a browser". The layer gate checks that nothing **below** L6
names a UI toolkit, and `crates/ui` **is** L6 — so it may name `eframe`,
`winit` or `wgpu` without a word. The wasm build would notice some of those
and not others, and "the wasm build noticed" is not the rule. §131.

**A module path is the only shape this check reads.** A sentence that names a
*type* this tree does not have, or a *function*, is not caught: the check is
about the module-shaped head, because a module is a name the tree either has
or does not, and a type or a method is a claim about a shape as well as a
name. The gap is real and it is bounded — the one dangling name the sweep
found was a module — but it is a bound and not a proof.

### The lesson

**A rule carried in from another tree is a rule about another tree.** The
sentence was true where it came from and false where it arrived, and the
thing that carried it — a scaffold copied into an application — is a thing
that copies the wrong sentences too.

**A rule that names a module is a claim that the module exists.** The claim
was in the file every agent reads first, and it had been false since the rule
arrived. Nothing was going to notice: the rule was about a canvas, and there
is no canvas to break.

**The document that is read first is read by nobody.** This one was not read
by a person but by an agent, and an agent that follows it looks for
`navigation::handle`, does not find it, and learns nothing — which is the
worst outcome a rule can have, because it looks like the rule simply does not
apply.

**A gate checks the shapes it knows.** Three shapes of prose claim were
checked — a symbol beside a file, a document reference, a line number — and
the shape in this sentence was a fourth. The sweep that found it looked for
*any* backticked dotted name, and it found six; the gate that now holds it
down reads the same shape, and it reads one.

---

## 131. The rule that the shell may not own a window had nothing behind it

### The measurement

`AGENTS.md` says:

> **The UI is thin.** Panels read engine state and act through
> `session.execute`. A panel never writes to the document, never calls the
> evaluator, never caches a value the evaluator computes. `crates/ui` depends
> on `egui` and nothing else — no windowing, no renderer — which is what lets
> the same panels run in a window, headlessly, and in a browser.

`cargo xtask layers` is the gate that owns the rules about crates. It refuses
a **UI toolkit below L6**: `egui*`, `eframe`, `egui-wgpu`, `wgpu`, `winit`,
`rfd`, `accesskit*`.

`crates/ui` **is** L6. So the rule, as the gate read it, did not apply to the
crate the rule is about.

Measured, by adding `eframe` to `crates/ui`'s dependencies and running the
gate that owns the rule:

```
$ cargo xtask layers
── layers
   ok: every crate depends only on lower layers
```

The gate that owns the rule passed. The rule had nothing behind it: a window
in the shell was not a violation of anything, and the shell is the one crate
the rule is about.

### Why nothing noticed

**The rule was written as a description, and the gate that checks descriptions
is not the gate that checks the tree's shape.** "depends on `egui` and nothing
else" is true today, and it reads like a fact about the tree rather than a
promise about it. The promise is what needs holding down.

**The layer gate's own rule was about the other direction.** It exists so that
a *value* crate does not reach for egui's `Vec2` — that is what its message
says. The shell reaching for a window is the same shape of mistake one layer
up, and the gate stopped exactly where it was told to.

**Nothing else looks.** The wasm build may notice some windowing crates,
because they do not build for a browser — but that is the browser saying so,
not the rule, and it says nothing about a crate that does build for a browser.
A rule enforced by an accident is not a rule.

### The fix

**`cargo xtask layers` now refuses a window in the shell.** The UI crate may
name `egui` and nothing else from the UI list; every other crate keeps the
rule it had:

```rust
pub const UI_CRATE_ALLOWED: &[&str] = &["egui"];
```

The new violation says what the rule is *for*, because a violation that only
says "not allowed" teaches nothing:

```
craftkit-ui depends on `eframe`: the shell owns its panels, not its window.
     The shell may depend on egui and nothing else — no windowing, no renderer — which is what lets the same panels run in a window, headlessly, and in a browser.
```

The apps are exempt, and they were always going to be: they are the ones that
own the window.

**And the sentence in `AGENTS.md` is now true as written.** It said the shell
"depends on `egui` and nothing else", and the shell depends on nine things —
`serde`, `serde_json`, `log`, `egui`, `image` and four crates of this tree.
What the rule means is *no other UI toolkit*, and that is what it says now.

### What now holds it

| Property | Held by |
|---|---|
| The shell may not own a window | `the_shell_may_not_own_a_window` in `xtask` |

and one mutation in `scripts/mutation-test-gates.sh`: *layers: the shell takes
a window*, which adds `eframe` to `crates/ui`'s dependencies. Proven to fail
without the fix — the gate passed with `eframe` in the shell, and fails with
it.

### What is still open

**"A panel never calls the evaluator" has nothing behind it either.** The
shell depends on `craftkit-eval` and uses two of its *types* — `Plan` and
`Severity` — but nothing stops it from calling `evaluate_once` and evaluating
the document itself, which would be the evaluator's answer reaching the screen
by a second road. The tree does not do it, and the sentence is true; nothing
would say so if it did.

**The other two claims hold for structural reasons, which is better than a
gate.** "A panel never writes to the document" is enforced by the engine's
API: the only mutable accessor, `Session::doc_mut`, is `pub(crate)` to the
engine, so the shell cannot reach it. "Never caches a value the evaluator
computes" holds by what `ShellState` is — seven fields, and not one of them
an evaluator's answer — but that is an inspection, not a guarantee: nothing
stops a ninth field from being one.

### The lesson

**A rule about a crate and a rule about the tree are different rules.** The
layer gate checked the tree's *shape* — which crate depends on which — and the
rule was about one crate's *contents*. The gate did its job exactly as it was
written, and the rule it did not cover was the rule that mattered.

**The rule that is about the crate everyone looks at is the one that is
uncovered.** The shell is the crate the whole tree is built around: it is the
one the apps draw, the one the rasteriser renders, the one the automation
registry describes. A rule about *it* was the rule with the fewest checks.

**A description that is true today is a promise about tomorrow.** "depends on
`egui` and nothing else" was true when it was written and would have stayed
true through any number of changes, because nothing asked whether it still
was.

---

## 132. The rule that a panel never calls the evaluator had nothing behind it

### The measurement

`AGENTS.md` says, in the paragraph about the UI being thin:

> A panel never writes to the document, never calls the evaluator, never caches
> a value the evaluator computes.

Three claims. What holds them:

| the claim | what holds it |
|---|---|
| never writes to the document | **the engine's API** — the only mutable accessor, `Session::doc_mut`, is `pub(crate)` to the engine, so the shell cannot reach it |
| never calls the evaluator | **nothing** |
| never caches a value the evaluator computes | **an inspection** — `ShellState` has seven fields and not one of them is an evaluator's answer — and the inspection was already wrong: it has six. §138 |

The middle one had nothing at all. `crates/ui` depends on `craftkit-eval` — it
uses two of its *types*, `Plan` and `Severity` — so the shell holds both the
document and the evaluator's crate, and the two ways to evaluate a document
from outside `craftkit-eval` are one call away:

```rust
craftkit_eval::evaluate_once(document, at)   // the function
Evaluator::new().evaluate(document, at)       // the struct
```

Nothing in the tree called either. A shell that did would not look wrong: it
would draw the same panels, with the same levels, from the same document. It
would put a **second road to the answer** on the screen, and it would cost
every frame a fresh evaluation of the whole document — measured on the
ten-thousand-node design, **7.3 ms** at best and **13.3 ms** at worst (§129's
probe), against a frame that already costs about 52 ms for the node list
alone (§127).

### Why nothing noticed

**The rule is about a call, and the tree's gates are about shapes.** The layer
gate reads the crate graph; the fingerprint gate reads the model's fields;
the prose gates read documents. A rule about a *call* inside a function is the
one shape none of them looks at.

**The tree obeys it, and obeying it looks like holding it.** The shell has
never evaluated the document, so the rule has never been tested — and a rule
that has never been broken is a rule nobody has checked.

**The two roads would agree.** A shell that evaluated the document would draw
the same levels the plan holds, so the difference would not be *what* is on
the screen; it would be what the frame costs, and how many places the answer
can come from.

### The fix

**`cargo xtask lists` now checks that the shell does not evaluate the
document.** The check is on the **names**, and the names are exact: the two
ways to evaluate a document from outside `craftkit-eval` are
`evaluate_once` and `Evaluator`, and there is no third.

```rust
const EVALUATOR_NAMES: &[&str] = &["evaluate_once", "Evaluator"];
```

**Comments and strings do not count.** A doc comment that says the word is
saying the opposite, and a string that carries the word is a label. The
check therefore reads the file with its comments and its string contents
blanked out — and it keeps every line, because a check that names a line has
to name the **file's** line. (The helper that already existed for this,
`code_only`, removes a block comment whole, newlines and all, so the line
numbers of what is left are not the file's; this one keeps them.)

The tests are not the panels: a test may name the evaluator to prove that the
shell does not call it, so the check stops at the test module.

### What now holds it

| Property | Held by |
|---|---|
| The shell may read the plan and may not evaluate the document | `the_shell_may_read_the_plan_and_may_not_evaluate` in `xtask` |

and one mutation in `scripts/mutation-test-gates.sh`: *ui: the shell evaluates
the document*, which adds one line to `node_rows`:

```rust
let _ = craftkit_eval::evaluate_once(document, craftkit_core::Tick::ZERO);
```

It compiles, and the gate says:

```
crates/ui/src/shell.rs: line 841 names `evaluate_once`: the shell may read the plan, but it may not evaluate the document
```

### What is still open

**"Never caches a value the evaluator computes" is an inspection, not a
guarantee.** `ShellState` holds no evaluator's answer today, and nothing
stops a ninth field from being one. A gate for it would have to read the
struct's fields and know which types the evaluator produces — which is a
judgement about what a field *means*, and the tree's own rule is that such a
judgement is a heuristic that eventually flags something real (§29).

**The rule is checked in one direction.** The gate says the shell does not
evaluate the document; it does not say the shell does not *cache* an answer,
nor that the engine is the only place the plan is computed. The tree's
structure is what says the last one: `Session` owns the evaluator and
nothing else holds one.

### The lesson

**A rule about a call needs a gate that reads calls.** Every other rule in
this document is about a shape the tree has — a file, a crate, a field, a
document — and every one of those can be read. "Never calls the evaluator" is
about a *verb*, and the gates read nouns.

**Two roads to an answer are worse than one even when they agree.** The shell
evaluating the document would have drawn the same picture. The cost would
have been a frame that pays for the whole document to draw a panel, and an
answer that could come from two places and drift.

**A rule nobody has broken is a rule nobody has checked.** The tree obeyed
this one from the day it was written. That is not evidence that the rule
holds — it is evidence that the tree has never had the chance to break it.

---

## 133. A test that failed when the code did the right thing

### The measurement

`apps/craftkit-cli/tests/mcp_stdio.rs` ends with the test that holds the MCP
server's promise: a `shutdown` ends the server, and nothing is answered after
it.

The test wrote a `ping` **after** the shutdown and then read the answer:

```rust
s.send(r#"{"jsonrpc":"2.0","id":3,"method":"ping"}"#);
assert!(s.reply().is_none(), "the server answered after shutdown");
assert!(s.wait().is_some(), "the process did not exit after shutdown");
```

`send` writes to the server's stdin and **panics if the write fails**. And a
server that has done what it was told — exited — has closed that pipe: the
write fails with a broken pipe.

So the test failed when the server behaved correctly, and passed when the
server was still alive.

Measured, with the machine busy — twelve spinning loops on sixteen cores —
forty runs of the test:

| | failures |
|---|---|
| the test as it was | **32 of 40** |
| the test as it is now | **0 of 40** |

On a quiet machine it failed **once in forty**. That is why it survived: a
test that fails one time in forty on an idle machine and eighty times in
forty on a busy one is not a test anybody suspects — it looks like the
machine.

### Why nothing noticed

**The failure message pointed at the wrong thing.** The panic said *"the
server stopped listening (Broken pipe) — the server said nothing; its status
is Some(ExitStatus(unix_wait_status(0)))"*. Every part of that is the server
behaving correctly: it stopped listening, it said nothing more, and it exited
with success. The message was written for a server that *died*, and it was
being read as one.

**The question was right and the way of asking it was wrong.** "Does the
server answer after the shutdown?" is the right question. "Write to the
server and see whether the write works" is a different question, and the
server's answer to it is *"no — and that is what you asked for"*.

**A race with a process's exit is invisible when the machine is fast.** The
parent's write and the child's exit are two events on two cores, and on an
idle machine the parent wins.

### The fix

**The write after a shutdown is allowed to fail.** A server that has already
exited is behaving exactly as it was told, and the broken pipe is the proof
of it:

```rust
let ping = r#"{"jsonrpc":"2.0","id":3,"method":"ping"}"#;
let _ = writeln!(s.stdin, "{ping}");
let _ = s.stdin.flush();
```

**And the process's ending is waited for with the pipe into it still open.**
`Child::wait` closes that pipe, and a server that sees end-of-file on stdin
ends whether or not it was told to — so waiting that way would prove nothing
about the shutdown. The test polls `try_wait` instead:

```rust
fn wait_for_exit(&mut self, within: std::time::Duration) -> Option<std::process::ExitStatus> { … }
```

**And the wait is bounded.** A server that does not end is the failure this
test exists to catch, and a test that hangs is not a test that catches it.

### What now holds it

The mutation that proves the rule was already in the suite: *mcp: shutdown
says ok and then keeps serving*, which makes the server ignore the shutdown.
With the fixed test the server does not end, the bounded wait runs out, and
the test fails with *"the process did not exit after shutdown"* — verified,
and the whole `mcp_stdio` binary still fails under that mutation.

### What is still open

**The first symptom was a different flake.** The run that started this was a
`cargo xtask ci` that failed in `craftkit-ui --lib`. The flake found here is
in `craftkit-cli`, and the `craftkit-ui` one was caught next. Two gates
failing for the wrong reason in one afternoon is a habit, not a coincidence:
a gate that can fail for a reason other than the thing it watches will
eventually do so.

**A test that writes to a process it expects to die can fail the same way
again.** The rule the fix follows — *a write to a server that has already
exited is not a failure* — is now in this one place, and the other three
tests in the file write to a server that is supposed to stay up ✓. If
another test ever wants to write to a process it expects to end, it should
follow the same rule rather than rediscover it.

### The lesson

**A test that fails when the code is right is worse than no test.** It does
not sit there failing: it fails *sometimes*, on a busy machine, and it takes
the failure with it into CI, where nobody suspects the machine.

**The broken pipe was the evidence.** A pipe that breaks is a process that
has gone — which is exactly what the test wanted to see. The test read it as
a fault because the code around it only knew how to read a fault.

**Ask the question, not a proxy for it.** The question was "does the server
answer after the shutdown?"; the proxy was "does the write work?"; and the
proxy's answer changed meaning when the server did the right thing.

---

## 134. A gate that measured the machine instead of the code

### The measurement

§127's gate for the node list's collection is a timing gate: the production
way of collecting the rows has to be cheaper than the way it replaced, which
is the one that asked the plan about every node.

```rust
assert!(
    collected * 8.0 < scanned,
    "collecting the rows of ten thousand nodes took {collected:.6}s, and asking the plan \
     about every node took {scanned:.6}s: the rows are still collected a scan at a time"
);
```

The threshold was **eight**, and it was read off a quiet machine: 1.2 ms
against 43.7 ms, thirty-six times. A ratio of thirty-six is a number you
can put a threshold of eight next to and feel safe.

The fast way takes about a millisecond. The scan takes forty. So a moment
when the machine is busy — a scheduling quantum, a page fault storm, a
frequency step — is **all** of the fast measurement and a twentieth of the
slow one, and the ratio collapses.

Measured, on a busy machine:

```
collecting the rows of ten thousand nodes took 0.009469s, and asking the plan
about every node took 0.068891s
```

**7.3 times.** Under the threshold of eight, on a machine that was not
wrong, with the code thirty-six times faster than the thing it replaced.

### Why nothing noticed

**The threshold was set from the measurement it was supposed to be robust
against.** Thirty-six on a quiet machine, eight on the threshold — and the
number that matters is the *worst* ratio the machine can produce, not the
best one. Nobody measured the worst one, because the worst one only happens
when the machine is busy, and a busy machine is somebody else's problem.

**`fastest` takes the minimum of nine rounds, which helps and is not
enough.** The minimum rejects a round that was disturbed. It does not
reject a *sustained* disturbance, and a sustained one is what a busy machine
is.

**The failure message was about the code.** *"the rows are still collected a
scan at a time"* — said by a gate that was measuring how fast the machine
was.

### The fix

**The fast way is measured in bulk**, so that both sides of the comparison
span about the same half-second of the machine's time:

```rust
const CALLS: usize = 32;
let collected = fastest(9, || {
    for _ in 0..CALLS {
        std::hint::black_box(node_rows(session.document(), session.plan()));
    }
}) / CALLS as f64;
```

**And the threshold is four, not eight.** The scan this gate replaces is
*identical* to the fast way — a ratio of exactly one — so a threshold of
four is four times the margin over the thing the gate exists to catch. The
old threshold of eight was not closer to the mutation; it was closer to the
number on the quiet machine.

Measured after the change: **0 failures of 25** runs of the gate on a busy
machine.

### What now holds it

The mutation was already in the suite: *ui: the node list asks the plan once
per node again*, which puts the scan back. It still fails the gate, and now
it fails it by the right margin — under the mutation the two sides measure
the same thing, **0.052229s against 0.051710s**, a ratio of one.

### What is still open

**The threshold is still a number read off a machine.** Four is safer than
eight, and the measurement is now much steadier, but a gate that compares
two times will always be talking about the machine as well as the code. The
honest version of this gate would count the work instead of timing it —
count how many times the production code visits the plan's nodes — and that
needs the plan to say so. §127's fix made the count one pass; nothing
*reports* the count.

**Two of the tree's timing gates are like this.** The properties panel's two
(§123) compare the panel's cost with and without the copy it should not do,
and their thresholds are eight as well. They have not been seen failing, and
they have more room than this one did — measured at **1.54** and **0.94**
against a threshold of eight. The second of those measures a ratio of *less
than one*, which says the copy is not happening at all: the panel is not
slower with 64 KiB of keyframe values than with one byte. A gate whose
number is below one is a gate that has nothing to be nervous about.

### The lesson

**A threshold read off one machine is a threshold the machine sets.** The
number on the quiet machine was thirty-six; the number on the busy one was
7.3; and the gate was written for the first and failed on the second.

**The margin belongs to the mutation, not to the measurement.** The gate
catches a ratio of one. A threshold of four catches it with four times the
margin, and costs nothing — the extra margin was never buying anything,
because a ratio of eight was never the thing being looked for.

**A gate that can fail for a second reason will eventually do so.** Two in
one afternoon (§133 and this one), in two different crates, for two different
reasons, and neither of them had anything to do with the code it watches.

---

---

## 135. The rule that there is no second path had nothing behind it

### The measurement

`AGENTS.md`, under **Everything is a command**:

> User-visible behaviour = a `CommandSpec` in `crates/engine/src/commands.rs`.
> That one registry is what the toolbar, the dialogs, scripts, the CLI, the
> control channel and the MCP server all reach. There is no second path.

And `Session::execute`'s own doc comment:

> This is the only way anything changes.

Both claims are about the **front ends**, and the front ends were measured —
one by one, in production code only, with the test modules cut:

| Front end | reaches the registry | anything else that changes a design |
|---|---|---|
| `crates/ui/src/shell.rs` | `session.execute` — one call site | none |
| `apps/craftkit/src/main.rs` | through the shell | `load_document` — opening a file (§18) |
| `apps/craftkit-web/src/main.rs` | through the shell | `load_document` — the stash (§18) |
| `apps/craftkit-cli/src/cli.rs` | 5 × `session.execute` | `load_document` — opening a file (§18) |
| `crates/control/src/server.rs` | 2 × `session.execute` | none |
| `crates/mcp/src/server.rs` | `guard.execute(command, arguments)` | none |

The front ends honoured the rule. The **engine's own public API** did not.

`Session` had twenty-two public methods. Two are constructors, fifteen read,
one is the registry, two open a file (§18). The other two were roads around
the registry:

- **`pub fn recompute(&mut self)`** — re-evaluates the document at the
  playhead. No command, no trace step, no log entry, no undo. **It had no
  caller anywhere in the tree.** It was public by accident, and the doc
  comment above it said nothing about who was allowed to call it.
- **`pub fn seek_to(&mut self, at: i64)`** — moves the playhead. The registry
  has a command for exactly this, `playback.seek`, which validates its
  argument and **clamps it to the design's range**. `seek_to` did neither. It
  had one caller: the crash replay.

A front end was free to take either road. *"There is no second path"* was a
sentence about a door that was standing open.

### The difference, measured

A design whose range is one second to two, and a playhead asked to go to 999
seconds:

```
playback.seek 999s -> 2.000s data={"from":254016000000,"seconds":2.0,"to":508032000000}
seek_to(999s) -> at=999 (raw 253761984000000)
  inside the range? false
```

The registry clamps to the design's end. `seek_to` puts the playhead **999
seconds into a design that is one second long** — and the plan is evaluated
there, and the timeline draws a handle at a place the design does not reach.

§115 is the section that made the playhead a place **in** the design. It
clamped `playback.seek`, it took the playhead with a range that moved under
it, and it took the playhead with a document that arrived with its own range.
It did not touch the two roads that do not go through the registry.

### The consequence, on a road a user walks

The crash replay is not a hypothetical: `craftkit-cli crashes replay <id>`
and the control channel's `diagnostics.crash_reports` with `action: "replay"`
both run it.

A crash report carries **three** things (§47): the design as it was when the
crash happened, the design the trace started from, and the commands in
between. The playhead in the report is the one the session was standing at
**when it crashed**; the design in the report is the one the trace **started
from**. Those two are the same document only until a command changes the
range — and `doc.range` is a command.

So a report written across a `doc.range` command holds a playhead outside the
range of the design it is replayed onto. The replay loaded the base — which
clamped the playhead into the base's range — and then sought to the report's
tick, which put it straight back out:

```
the replay put the playhead at 1270080000000, outside the design's range 0..254016000000
```

Five seconds into a one-second design.

### The live evidence

The same thing, driven through the running application over the control
channel. A report is written whose base is a one-second design and whose
playhead is five seconds in — the shape a report written across a
`doc.range` command has — and the app is asked to replay it:

```text
diagnostics.crash_reports {"action": "replay", "id": "20250101-000000-0001"}
```

Before the fix:

```text
replay says: at=1270080000000 start=0 end=254016000000 inside_the_design=False
```

After the fix:

```text
replay says: at=254016000000 start=0 end=254016000000 inside_the_design=True
```

Same report, same app, same request; the only difference is the clamp.

### Why nothing noticed

**The rule was written about the front ends and was checked about the front
ends.** Every front end was inspected, at the start, and found clean. The
thing the rule was *about* — the engine's API — was never looked at, because
the rule did not say it was about the engine's API.

**`recompute` had no caller, and a public method with no caller is
invisible.** A dead private function is noise. A dead *public* function is an
API promise, and nobody reads an API promise that nothing calls.

**`seek_to`'s only caller was inside the crate.** `crates/engine/src/crash.rs`
called it, so a search for callers outside the crate found none — and the
caller was the crash reporter, a piece of the tree that reads like
infrastructure and is rarely looked at closely.

**The clamp was in the command, not in the session.** `playback.seek` clamped
its own argument. The invariant *"the playhead is a place in the design"* was
therefore a property of one command — and a command is the one place in this
tree where a property like that does not belong.

### The fix

**The invariant moved into the session**, where every road into the playhead
passes:

```rust
pub(crate) fn set_at(&mut self, at: Tick) {
    self.at = at;
    // The playhead is a place **in** the design (§115). …
    self.clamp_playhead_into_range();
    self.recompute();
}
```

**The two roads are gone from the public API.** `recompute` and `seek_to` are
`pub(crate)` now: `recompute` had no caller, and `seek_to` had one, inside the
crate.

**And the rule is enforced.** `cargo xtask lists` carries two new gates:

- `the_session_offers_no_second_path` — the public methods of `Session` are
  an **allow list**: the registry, the readers, and the two that let the host
  open a file. A new public method is refused until somebody says what it is
  and why it is not a second path. That is the only direction that fails
  safe.
- `the_shell_does_not_write_the_document` — the other half of *"a panel never
  writes to the document"*, which §132 called enforced by the engine's API. It
  was enforced for the **mutator** (`doc_mut` is `pub(crate)`) and not for
  the **writers** (`load_document` and `replace_document` are public, because
  the host opens designs with them). The shell never called either; now it
  may not.

The engine-side gate covers every front end at once, including the ones that
do not exist yet: a new front end cannot change a design without a command,
because the engine does not offer it a way to.

### What now holds it

Five mutations, in `scripts/mutation-test-gates.sh`:

| Mutation | Gate | Caught by |
|---|---|---|
| engine: the session offers a second path to the plan | `cargo xtask lists` | the allow list |
| engine: the session offers a second path to the playhead | `cargo xtask lists` | the allow list |
| engine: the playhead can leave the design | `craftkit-engine` test | `the_playhead_cannot_leave_the_design` |
| engine: a crash replay puts the playhead outside the design | `craftkit-engine` test | `a_crash_replay_does_not_put_the_playhead_outside_the_design` |
| ui: the shell puts a document into the session | `cargo xtask lists` | the shell gate |

### What is still open

**The allow list is a list of names.** It says which public methods exist, not
what they do. A public method that *reads* is on the list because it is on
the list; if one of them started writing, the gate would not notice. The
readers are safe today because `Session` has no interior mutability they
could reach — `history_mut` and `doc_mut` are `pub(crate)` — and that is an
argument, not a check.

**The front ends are not gated.** The engine's API is, so a front end cannot
change a design without a command; but nothing checks that a front end does
not *reimplement* a command's behaviour — a shell that drew a rectangle
itself rather than running `sketch.rectangle` would pass every gate in the
tree. §132's gate has the same shape and the same hole: it checks what the
shell *names*, not what the shell *does*.

**File I/O is still outside the registry**, by design (§18):
`load_document` and `replace_document` are public, and the CLI's `open` is a
subcommand. The owner question — *should `save`/`open` be commands?* — is
still open, and it is in the suggested order.

### The lesson

**A rule written about the callers is not a rule about the callee.** *"The
front ends reach the registry"* was true, and the thing it was supposed to
guarantee — that there is no other way in — was false, because the guarantee
lived in a type's visibility and the rule lived in a paragraph about front
ends.

**An API is a promise, and a promise nothing calls is invisible.** The dead
public `recompute` was the clearest thing in the tree and nobody saw it,
because *"does anything call this?"* is not a question anybody asks about a
function that is already public.

**The invariant belongs where every road passes.** The playhead is a place in
the design; that is a property of the *session*, not of the one command that
happened to implement it. §115 put the clamp in the command, and the command
was the only road that had it.

---

---

## 136. The rule that a parameter is never indexed was a convention

### The measurement

The tree says the same thing in four places:

- `AGENTS.md`: *"Read them through `craftkit_core::Params`' typed helpers;
  never index."*
- `crates/core/src/lib.rs`: *"[`Params`] — the only way to read a command
  parameter. Never index."*
- `crates/engine/src/lib.rs`: *"Never index a parameter object, anywhere,
  ever."*
- `crates/core/src/params.rs`: *"**Never index a parameter object.**
  `params["distance"]` is a panic waiting for an agent, a script, or a file
  from a future version."*

None of it was enforced. The typed helpers are typed, and a typed value cannot
be indexed — that half is structural. But there was a door out of it:

```rust
/// The raw JSON for a key, for the cases where a caller genuinely needs it.
pub fn raw(&self, key: &str) -> Option<&'a Json> {
    self.get(key).filter(|v| !v.is_null())
}
```

A `&Json` **is** indexable, and indexing it is either a panic or a silent
wrong answer: `json["key"]` panics on anything that is not an object, and
answers `null` for a key that is not there. A silent wrong answer is the one
thing the tree cannot catch.

There were **seven** call sites, and every one of them was careful — each
matched the JSON's shape with a `match` and never indexed:

| File | Call sites | What it read |
|---|---|---|
| `crates/core/src/params.rs` | 1 | `value_of` — the generic value path |
| `crates/engine/src/params.rs` | 2 | `node_ref`, `value_for` |
| `crates/engine/src/commands.rs` | 4 | presence, `opt_time`, `opt_kind`, `opt_easing` |

Seven careful call sites are not a rule. They are seven decisions to be
careful, made by whoever wrote them, and the eighth one would not have had to
be.

### The file that looked like it held the rule down

`clippy.toml` began:

```toml
# The one place that decides whether the "never crash" rule is enforced.
# Each is re-enabled for test code, because tests are allowed to panic.
allow-unwrap-in-tests = true
allow-expect-in-tests = true
allow-panic-in-tests = true
allow-indexing-slicing-in-tests = true
```

Three of those four settings re-enable a lint the crate roots deny. The
fourth re-enables **`clippy::indexing_slicing`**, which is denied in **no
crate** — the house deny list has six lints and it is not one of them. So the
setting allowed, in test code, something that was allowed everywhere: it
enforced nothing at all.

Measured, with the lint switched on by hand:

```text
 72 warning: indexing may panic
 16 warning: slicing may panic
```

**88 sites** in production code — 39 in `xtask`, 16 in `crates/model`, 11 in
`crates/ui`, 9 in `craftkit-cli`, 6 in the desktop app, 3 in `crates/core`,
2 in `crates/engine`, 2 in `crates/mcp`. All of them safe: `windows(2)`
yields slices of length two, `binary_search` returns an index that is in
range, and the reader's own slice ranges are bounded by construction. **The
tree indexes on purpose, and it is right to.** Forbidding *all* indexing would
be a style rule, and it would make twenty of those eighty-eight sites worse
rather than safer — `Ok(i) => { self.0[i].ease = ease; }` cannot become a
`get_mut` with an error branch, because the error branch cannot happen and
`unreachable!` is denied.

So the lint was the wrong tool, and the setting was not evidence of a rule
that existed. It was evidence of a rule that was *wanted*.

One measurement made the file's claim checkable: clippy **errors** on a
configuration key it does not know.

```text
error: error reading Clippy's configuration file: unknown field
`allow-definitely-not-a-real-key`, expected one of ...
```

So `allow-indexing-slicing-in-tests` was a key clippy accepts. It was
accepted, and it enforced nothing.

### Why nothing noticed

**The rule was true of every call site.** Seven of them, all careful, and the
carefulness looks exactly like the rule being enforced. Nothing distinguishes
"this code follows the rule" from "this code happens to be safe", and the
tests cannot: `hostile_params_never_panic` fires every command at every
documented parameter name with every hostile value, and it passed — because
the seven call sites were safe.

**The escape hatch had a plausible name.** `raw` reads as *"the raw value"*,
not as *"the door out of the typed readers"*. A reader of the code had no way
to know it was the one thing the rule was about.

**The lint was the wrong shape, so the lint could not be the answer.** The
rule is about *parameters*. `clippy::indexing_slicing` is about indexing. The
rule has no lint, and the lint has no rule, and the file that connected them
was a configuration key.

### The fix

**The escape hatch is named after what it is, and it has two homes.**
`Params::raw` is `Params::raw_json` now, and its doc says what it is:

> **This is the escape hatch** … a `&Json` *can* be indexed: `json["key"]`
> panics on anything that is not an object, and answers `null` for a key that
> is not there.

Two modules may call it, and both are *readers*:
`crates/core/src/params.rs`, which owns the parameters, and
`crates/engine/src/params.rs`, which is where a parameter becomes a typed
value. `cargo xtask lists` refuses the tree when a third file calls it.

**The commands no longer see the raw JSON at all.** The three readers that
have to know a parameter's shape — `opt_time`, `opt_kind`, `opt_easing` —
moved out of `crates/engine/src/commands.rs` and into
`crates/engine/src/params.rs`, which is the module whose own header says it
exists to be "the answer to that temptation". The one command that only had
to know whether a parameter was *given* now asks `Params::has`, which reads
no shape at all.

**The dead setting is gone, and the file says what it does.** `clippy.toml`
now says that the deny lists are in the crate roots and that it decides
where they do not apply, and it carries three settings instead of four. A
new gate checks that every setting in it names a lint the tree denies, so
the next dead key cannot be added quietly.

### What now holds it

Two mutations:

| Mutation | Gate | Caught by |
|---|---|---|
| core: a third file reads the raw parameter JSON | `cargo xtask lists` | the two-homes rule |
| clippy: a setting re-enables a lint nobody denies | `cargo xtask lists` | the settings rule |

And two tests: `the_raw_parameter_reader_has_no_third_home` and
`every_clippy_setting_names_a_denied_lint` in `xtask`, plus
`a_parameter_that_is_null_is_absent` and
`the_raw_reader_hands_back_the_json_itself` in `crates/core`.

### What is still open

**The rule is still a rule about *parameters*, not about indexing.** The tree
indexes eighty-eight times on purpose, and that is right. A second rule —
*never index a value that came from outside* — would be about the readers of
`.craftkit` files and of the control channel, and it is not written down
anywhere. The gate cannot express it: `indexing_slicing` is all-or-nothing,
and the tree's own indexing is safe.

**The escape hatch is still public.** `Params::raw_json` is on the public API
of `craftkit-core`, because the engine's reader needs it and the engine is
another crate. The gate holds it to two files; a *type* would hold it to two
files by construction, at the price of a newtype that every reader has to
unwrap. That trade is open.

**One skill cited the reader's lines.** `skills/craftkit-editing/SKILL.md`
had a table of readers cited by line number — `params.rs:108`, `params.rs:117`
— and the first edit to the file moved them. The four line citations are now
file paths, which is what the house style wants anyway: *"Prefer a symbol
over a line number when you write one."*

### The lesson

**A convention with seven careful examples is not a rule.** It is seven
decisions, and the eighth is not covered. The rule needed a shape that made
breaking it hard to write, not a paragraph that asked nicely.

**A configuration key is not evidence.** The file said it was "the one place
that decides whether the 'never crash' rule is enforced", and one of its four
keys enforced nothing — it re-enabled a lint that was denied nowhere. The
setting was not a rule; it was a wish written in TOML, and it looked exactly
like the opposite of one.

**The right tool for a narrow rule is a narrow tool.** `clippy::indexing_slicing`
would have forbidden eighty-eight safe indexings and made twenty of them
worse. The rule was about *parameters*, and the gate is about parameters.

---

---

## 137. A hostile file could make a design uneditable

### The measurement

`craftkit_core::IdCounter` hands out the ids a document's nodes get. It was
**saturating**:

```rust
pub fn next(&mut self) -> u64 {
    let cur = self.0;
    if self.0 < Self::MAX {
        self.0 += 1;
    }
    cur
}
```

At the ceiling it stops advancing, and so it hands out its last value **for
ever**. The comment above it said what that was for:

> Saturating stops the document changing at all, which is a far smaller lie.

It did not stop the document changing. It made the document **unbuildable**:
the second node it minted shared an id with the first, and two nodes that
share an id are a document the file format refuses (§92's "two objects share
an id").

And the test said so too, and said it was correct:

```rust
c.observe(IdCounter::MAX);
assert_eq!(c.next(), IdCounter::MAX);
assert_eq!(c.next(), IdCounter::MAX, "must saturate, never wrap");
```

The name was `ids_are_sequential_and_saturate` and the assertion was that the
same id comes out twice. **The defect was pinned by a test.** The intent
behind it — *never wrap* — was right; the implementation repeated instead of
stopping, and the test recorded the repeat as the promise.

### How a file reaches the ceiling

`IdCounter::MAX` is 2^52 − 1 = 4 503 599 627 370 495. That is inside the
range a JSON number carries exactly, so a file from another machine can hold
it.

`Document::reseat_id_counter` raises the counter above every id in a loaded
document, and the file reader calls it — that is the "single most important
line of defence against a corrupt or hostile file", its own comment says. It
is also what carried the ceiling into the document.

So a file with **one** node, whose id is the ceiling, leaves the counter
saturated — and `Document::add_node` then mints that same id again. The
document has two nodes with the same id, `Document::validate` refuses it, and
the engine's own check (§50 — *no command leaves a document the file would
refuse*) turns that into a refusal of the command.

### The live evidence

A script whose base is such a file, and one command:

```json
{ "name": "hostile ids",
  "base": { "meta": { "name": "Hostile", "fps": 24, "start": 0, "end": 127008000000 },
            "nodes": [ { "id": 4503599627370495, "name": "Only", "kind": "source",
                         "props": { "id": "props", "label": "Properties", "nodes": [] },
                         "muted": false } ] },
  "commands": [ { "command": "node.add", "params": { "name": "Second", "kind": "filter" } } ] }
```

```text
craftkit-cli: step 1/1: `node.add` failed unexpectedly (the command left the document in a
state the file format refuses: two objects share an id: two nodes share an id); your
document is unchanged
```

That is the tree's own vocabulary for *"worth a crash report and a bug"* — and
it is what a file from another machine produces. And it is not one failure:
the counter stays saturated, so **every** later `node.add` and every
`node.duplicate` fails the same way, for ever. The design is open, visible,
and uneditable.

### Why nothing noticed

**The ids are absurd.** Four and a half quadrillion, against a model that
allows ten thousand nodes. No honest document gets near the ceiling, so the
only way there is a file — and a file is *input*, the thing the tree's rules
are mostly about.

**The comment said the right thing.** "Saturating stops the document changing
at all" is the intention; the code did something else. A comment that is
right about the intent and wrong about the code is worse than no comment,
because it stops the reader from looking.

**The test asserted the defect.** `must saturate, never wrap` reads like a
rule. It was a test that the same id comes out twice, described as the
property the tree wanted. A test that pins a defect is not a missing test; it
is a test that says the defect is intended.

### The fix

**The counter stops one id short of its ceiling, and says so when it cannot
go on.** `next()` returns `Option<u64>` now, and `None` when it can no longer
promise an id that is not already in use:

```rust
pub fn next(&mut self) -> Option<u64> {
    if self.0 >= Self::MAX {
        return None;
    }
    let cur = self.0;
    self.0 += 1;
    Some(cur)
}
```

The refusal is the point: a counter that hands out its last id twice breaks
the one promise the type makes — *an id is unique across the whole document* —
and the document it breaks is the one the file format refuses, so the failure
lands on a command the user cannot fix.

**The document says why.** `Document::add_node` refuses with the reason
rather than minting a duplicate:

```text
craftkit-cli: step 1/1: this document's node ids have reached the ceiling of 4503599627370495,
so a new node cannot be given an id of its own
```

The document stays **valid** — the ids it has are unique — so it can still
be looked at, saved and replayed. Only the edit is refused, and the refusal
says what would have to change.

### What now holds it

Two mutations:

| Mutation | Gate | Caught by |
|---|---|---|
| model: the id counter hands out the same id twice | `craftkit-core` test | `the_counter_never_hands_out_the_same_id_twice` |
| model: the id counter ceiling is off by one | `craftkit-model` test | `a_document_whose_ids_reached_the_ceiling_refuses_a_new_node` |

And two tests: `ids_are_sequential_and_stop_at_the_ceiling` in
`craftkit-core`, and `a_document_whose_ids_reached_the_ceiling_refuses_a_new_node`
in `craftkit-model`.

### What is still open

**The reader accepts such a file, and that is a decision the tree made.** The
document is valid — its ids are unique — so refusing the file would lose a
design the user can still look at. The tree degrades gracefully: the design
opens, and the edit is refused with a reason that says what would have to
change. The other answer — refuse the file with *"its node ids are beyond
what this format can carry"* — is defensible, and a tree that valued a file
it cannot edit over a file it can would choose it.

**The ceiling is not in the model's limits table.** `IdCounter::MAX` lives in
`craftkit-core`, and the table in `crates/model/src/lib.rs` is the tree's
list of safety limits (§74). An id ceiling is a safety limit, and it is not
in the table — so nothing checks that the table and the code agree about it.

**A counter that stops is a counter that can be wrong.** If a document ever
needed more ids than the ceiling allows, the answer would be "no", and
nobody would be able to edit it. 2^52 is far beyond the model's ten thousand
nodes, so this is theoretical — and it is exactly the kind of theoretical
that the ceiling was chosen to make impossible.

### The lesson

**A test that pins a defect is worse than a missing test.** The test said
`must saturate, never wrap` and asserted the repeat. It read like a rule, so
everybody after it read the code as if the rule were being kept.

**"Saturating" is not "stopping".** The comment said saturating stops the
document changing; saturating makes the document *unbuildable*. The two
words are close enough that the code looked right.

**A hostile file is a file, and the model's limits are the honest place to
look for it.** Ten thousand nodes, fifty thousand properties, ten thousand
keyframes — every one of those is checked when a file is read. The one limit
that was not checked was the one a file could reach.

---

## 138. The rule that a panel caches no evaluated value was an inspection

### The measurement

`AGENTS.md` says:

> The UI is thin. Panels read engine state and act through `session.execute`.
> A panel never writes to the document, never calls the evaluator, never
> caches a value the evaluator computes. **All three are checked**: `cargo
> xtask lists` refuses a shell that names the evaluator, and one that puts a
> document into the session.

Two of the three were checked. The third was checked by somebody reading
`ShellState` once, and the reading was written down as a table row:

| never caches a value the evaluator computes | **an inspection** — `ShellState` has seven fields and not one of them is an evaluator's answer |

**And the inspection was already wrong.** `ShellState` has **six** fields:

```text
selected: Option<NodeId>
status: String
status_is_error: bool
key_path: String
key_value: String
prompt: Option<Prompt>
```

There is no seventh. The count has been six since §118 landed (commit
`8205259`, 2026-10-21), and the row that says "seven" was written **after**
that — in §132 (commit `0eee638`, the same day). So the snapshot was wrong
about the number it quoted from the moment it was written, and nothing
noticed, because nothing re-reads it.

That is the whole finding, in one sentence: **a hand-taken inspection of the
code is a snapshot with a timestamp on it, and the timestamp is the only part
that is ever checked.**

### What the inspection could not see

The shell's state is what the person is doing: a selection, a status line,
the text of the fields they are typing into. Everything else the shell shows
is read from the plan **every frame**, so it cannot go stale.

So the state may hold text, an id, and a flag. It may not hold a number.
Every number in a design is one of three things:

| what | type | where it comes from |
|---|---|---|
| a level or an amount | `f64` | `NodeState::level_in`, `NodeState::level_out`, `Value::Num` |
| a property's value | `craftkit_core::Value` | `Resolved::value` |
| an instant | `craftkit_core::Tick` | `Plan::at` |

A number kept between frames is a number the design said — and the design can
change. The copy goes stale, the screen shows the old number, and **nothing
says so**: the panel that draws it believes it.

There is a second, quieter shape of the same defect. The shell may *read* the
plan — that is allowed, and §132's gate is about it. What it may not do is
**keep** one. A `Plan` in the shell's state is a whole answer copied into a
frame, and it goes stale exactly the same way.

And the third shape is the one an inspection of `ShellState` alone could not
see at all: a struct that is not `ShellState` may hold the copy. `Shell`,
`Frame` and `Click` all carry the shell's state — `Frame` and `Click` are how
the headless renderer carries the screen's state out of a frame — and a `Plan`
in any of them is the same stale copy with more indirection.

### The fix

The scope is **found**, not listed. The state is `ShellState`; the structs it
holds are found by following the field types (`ShellState` holds `Prompt`,
`Prompt` holds `PromptField`); the structs that hold the state are found the
same way in the other direction (`Shell`, `Frame`, `Click`). A struct the
state starts holding is covered the day it is added, with no list to keep.

The rules are then two, with two different scopes:

- **The state holds no number and no value of the design's.** `f64`, `f32`,
  `craftkit_core::Value` and `craftkit_core::Tick` are refused in `ShellState`,
  `Prompt` and `PromptField`.
- **Nothing that carries the state holds the evaluator's answer.** `Plan`,
  `NodeState`, `Resolved`, `Diagnostic`, `Diagnostics` and `Severity` are
  refused in the state *and* in everything that carries it.

The two scopes differ because the two kinds of number are different kinds. A
frame's width, a click's screen coordinate — those describe **the screen**,
and they are allowed. A level is a number **the design said**, and it is not.

`serde_json::Value` is deliberately **not** refused: it is a JSON blob, a way
of carrying what the person typed, and not a property.

`NodeRow` shows why the scope had to be drawn where it was. `NodeRow` has a
`level: f64` in it — a number, from the design — and it is **correct**,
because a row is rebuilt every frame and thrown away. The line between a row
and the state is not "does it hold a number", it is "does it survive a frame".

The gate reports the file, the line, the struct, the field and the type:

```text
error: 1 shared-list problem(s):
   • crates/ui/src/shell.rs: line 47 gives `ShellState.cached_level` the type `f64`: that is a
     number the design said, and the design can change
     The plan is read every frame; the state holds no copy.
```

### What now holds it

Two mutations in `scripts/mutation-test-gates.sh`:

| Mutation | Gate | Caught by |
|---|---|---|
| ui: the shell caches a value the evaluator computes | `cargo xtask lists` | the number branch |
| ui: the shell keeps a copy of the plan | `cargo xtask lists` | the evaluator-type branch |

Both mutations **compile** — they are a change somebody could plausibly make
— and each is caught for a different reason, with a different message.

And one test, `the_shell_state_holds_nothing_the_evaluator_computes` in
`xtask`, which checks the scope as well as the verdict: that `Prompt` and
`PromptField` are in the state, that `NodeRow` is **not**, that `Shell` and
`Frame` carry it, and that a `serde_json::Value` is not a design's value.

### What is still open

**The gate reads the type as it is written.** A number behind an alias —
`type Level = f64;` — is not seen, because the text says `Level`. The same
hole is in the other direction: an **enum** variant carrying a `Plan` is not
seen either, because the gate reads structs. Both are narrow, and both are
the kind of thing a future change would have to notice rather than be
protected from.

**A number that is not the design's would be refused.** An animation phase or
a scroll offset kept in the state is not a number the design said — and the
gate would still refuse it. That is a deliberate trade: the state is *what the
person is doing*, and a number in it is a design number by default. The way
to keep one is to not keep it between frames.

**The state is the shell's — and today that covers both front ends.** The
desktop app's per-frame state *is* `ShellState`
(`SharedState = Arc<Mutex<craftkit_ui::shell::ShellState>>`), and so is the
web app's (`Application { state: craftkit_ui::ShellState, … }`). Neither keeps
a copy of its own. The CLI has no state between frames at all — it runs a
script and prints what came out — so there is nothing there to cover. A
**future** front end with a state of its own would need the rule restated for
it, and the gate would not notice it missing.

### The lesson

**"All three are checked" was a claim about two.** The sentence in `AGENTS.md`
counted three rules and named two gates, and the gap between the count and
the gates was not visible — the sentence read as though it were all right.

**An inspection is a snapshot, and snapshots go stale silently.** The §132 row
said "seven fields" and the code had six. It had had six since before the row
was written. A gate that re-reads the code cannot be wrong about the number
of fields; a note in a table can be, and will be, and nothing will say so.

**The scope of a rule is part of the rule.** "Never caches a value the
evaluator computes" says nothing about *where*, and the answer turned out to
be: the state, and the things that carry it, and not the things that are
rebuilt every frame. Writing the scope down is what made the gate possible —
without it the rule could only have been "look at `ShellState` and see".

---

## 139. The rule that every parameter you read is declared had nothing behind it

### The measurement

`docs/agent-playbook.md` has thirteen rules an agent is told to keep. Rule 4
is this one:

> ### 4. Every parameter you read is declared in `params`
>
> **Why:** `Session::execute` runs `p.deny_unknown(&spec.param_names())` before
> calling your `run`. A key you read but did not declare is **not ignored — it
> is a refusal**, and it takes the whole command down with it.
>
> **Check:** call your command with its own parameters in a test.
> `every_parameter_is_documented_and_uniquely_named` does not catch this.

The rule is **right about the consequence**, and it **names its own gap**: the
check is a thing a person does by hand, and the tree's test does not do it.
Nothing did it. `cargo xtask lists` had no check for it, and the tree's own
test — `every_parameter_is_documented_and_uniquely_named` — checks the help
text, that no parameter is declared twice, and that the first parameter is one
that is needed. It does not look at the command's body at all.

The scale of the thing nothing was checking: **23 commands, 56 declared
parameter entries, 19 distinct names**. Every one of them was a place where
the two lists could drift, and the two lists were kept in step by hand.

**Today the tree is consistent.** The gate reports **zero** mismatches, and an
independent probe of the same question — a different parser, reading the same
file — also reports zero. That is worth saying plainly: §139 did not find a
broken command. It found a rule with nothing behind it, in a tree where the
rule happened to be true.

### The two ways it goes wrong

**A parameter read but not declared makes the command unusable.** The engine
refuses an undeclared parameter, so the value the command needs can never
arrive. The command is not "slightly wrong": it cannot be called.

Here is `doc.rename` with the name it reads changed from `name` to `title` —
one word — and a script that does the obvious thing:

```text
craftkit-cli: step 1/1: `doc.rename` rejected its parameters: `title` this parameter is required
```

And the caller's second attempt, passing the parameter the error asked for:

```text
craftkit-cli: step 1/1: `doc.rename` rejected its parameters: `title` unknown parameter (expected one of: name)
```

The command asks for a parameter it is **refused**, and offers one it never
reads. There is no call that satisfies it. The error message names the
parameter the caller cannot supply — the worst kind of message, because it
sends the reader looking for something that does not exist.

**A parameter declared but not read is the silent one.** `node.mute` declares
`muted`, reads it, and — with one line changed — ignores it:

```text
1/1 node.mute: muted
```

The caller asked for `{"node": 1, "muted": false}` on an unmuted node. Before
the change the answer was `unmuted`; after it, the node is muted. The command
**succeeded**, the message is the message it always sends, and the value the
caller passed was thrown away. Nothing anywhere says so.

That is the direction that matters. The first one is loud: the command is
broken and says so. The second one is a value the caller believes in.

### The fix

The check is on the **source**, because that is where the truth is: the
command's `run` body says which names it reads, and its `params` list says
which names it declares. Both are in the same file, so they cannot drift
without the gate noticing.

**The readers are derived, not listed.** A reader is a function that takes a
`&Params` — the only way to read a parameter — or a method of `Params` itself.
The gate finds them by reading the tree, so a reader that is added is covered
the day it is added:

```rust
let is_method = has_word(&block, PARAMS_TYPE);
let takes_params = signature.contains(&format!("&{PARAMS_TYPE}"));
```

That is what makes the gate trustworthy. A list of readers written down in
the gate would be a second list to keep true, and a new helper would slip
past it silently — the same defect, one level down.

A reader's name argument is the one typed `key: &str`, and its **index**
matters: a method's receiver is not an argument, so `text(&self, key: &str,
max: usize)` has its name at index 0 and `node_ref(p: &Params<'_>, key:
&str)` has its name at index 1.

A reader with **no** name argument of its own reads whatever its own body
reads: `node_by_id_or_name` takes the document and the parameters and reads
`node` and `node_name` itself. The gate resolves that by reading its body,
one level down.

The two halves are told apart, and each has its own message:

```text
crates/engine/src/commands.rs: line 70 reads the parameter `note`, which `doc.note` does not
declare: the engine refuses an undeclared parameter, so the command cannot be given the value
it needs
```

```text
crates/engine/src/commands.rs: line 50 declares the parameter `text`, which `doc.note` never
reads: a caller that passes it is ignored
```

### What now holds it

Two mutations in `scripts/mutation-test-gates.sh`:

| Mutation | Gate | Caught by |
|---|---|---|
| engine: a command reads a parameter it does not declare | `cargo xtask lists` | the loud half |
| engine: a command declares a parameter it never reads | `cargo xtask lists` | the silent half |

And one test, `a_command_declares_every_parameter_it_reads` in `xtask`, which
checks the *scope* as well as the verdict: that the gate finds more than
twenty commands, that the nine `Params` methods and `node_by_id_or_name` are
among the readers it derived, that a reader with no name argument of its own
reads `node` and `node_name`, and that `doc.rename` declares `name` and reads
`name`.

### What is still open

**The gate reads the source as it is written.** A parameter read through a
macro, or a name built at run time, is not seen. There is no such reader in
the tree today, and the tree has no macros in command bodies, but the gate
knows about the readers it can see and not about the ones it cannot.

**The rule is about the registry.** The commands are read from
`crates/engine/src/commands.rs`, which is where the registry is; a command
defined somewhere else would not be covered. There is one registry, and it is
there.

**The engine's refusal is the backstop, and the gate is the reason it is
enough.** `deny_unknown` protects a *caller* from a command that lies about
what it accepts; it does not protect a *command* from a body that reads what
it did not declare. The gate is what covers that side.

### The lesson

**A rule that names its own gap is still a gap.** The playbook said the check
was manual and said the tree's test did not do it. The prose was honest and
the tree was unprotected, and the two facts sat next to each other for as long
as the rule existed.

**A derived list cannot go blind; a written-down list can.** The readers are
found from the tree, so a new helper is covered on the day it is written. The
gate would have been much shorter with a list of eleven names in it, and it
would have been wrong the day somebody wrote the twelfth.

**The two halves of a rule are not the same severity.** "Reads a parameter it
does not declare" breaks the command and says so. "Declares a parameter it
never reads" breaks the *caller*, quietly. A rule that only names the first
half names the half that is already loud.

---

## 140. The recipe for adding a command taught the two bugs the tree had already fixed

### The measurement

`docs/recipes/add-a-command.md` is the document an agent follows to add a
command to craftkit. It quotes a command to copy:

> `node.mute` (`NODE_MUTE` in `crates/engine/src/commands.rs`) is the closest
> template for anything that addresses a node and flips a boolean

The quote had drifted from the code in **three** places. Diffed against the
real `NODE_MUTE`:

```text
-        ParamDoc::req("node", Doc::NodeRef, "The node's id."),
+        ParamDoc::either("node", Doc::NodeRef, "The node's id, or its name.", "node_name"),
-        node_mut(s, id, NODE_MUTE.id)?.muted = muted;
+        let changed = node_mut(s, id, NODE_MUTE.id)?.set_muted(muted);
```

and one field missing altogether: `asks`.

**The quote does not compile.** Pasting it into the engine is the proof:

```text
error[E0063]: missing field `asks` in initializer of `CommandSpec`
```

So the document that tells you how to add a command gives you a command that
does not build. That is loud, at least — you find out in a minute.

The two edits are worse, because they compile and they are **the bugs the tree
had already fixed**:

- **`ParamDoc::req("node", …)` instead of `ParamDoc::either("node", …,
  "node_name")`** — the command it teaches cannot address a node by name. That
  is §116's defect, and §116 is the section that fixed it.
- **`node_mut(s, id, …)?.muted = muted;` instead of `set_muted(muted)`** — the
  model cannot report whether the value changed, so the command never says
  "changed nothing" and the engine's "declared that it mutates the document
  but changed nothing" check (§93, §44) never fires. That is §93's defect, and
  §93 is the section that fixed it.

**The recipe also named an API that does not exist.** Its rules list said to
read parameters through "`p.text`, `p.bool`, `p.int`, `p.num`, `p.num_in`,
`p.raw`" — and `p.raw` was renamed to `p.raw_json` in §136. So did the
playbook (§136 renamed the method and updated `AGENTS.md` and the skill, and
missed two documents).

And its step 5 said the catalogue was `static ALL: [&CommandSpec; 21]` and to
"bump the declared length from `21` to `22`". The catalogue has **23**
commands.

### Why nothing noticed

**The quote was not marked exact.** The tree has a gate for this: a block
marked ```` ```rust exact ```` is compared with the code it names, and the
gate says *"every exact block quotes the code it claims to"*. The recipe's
block was a plain ```` ```rust ```` block — a copy, and nothing said so.

The gate compares **functions and structs**. A command spec is neither: it is a
`pub static`, and a static that was never compared was a static nobody
checked.

**The other numbers were in the wrong shape.** The gate *"every number of
commands in prose is the registry's number"* looks for the phrase *"N
commands"*. The recipe says *"21"* — inside an array type. A number in a
sentence and a number in a type are the same number, and only one of them is
a claim the gate can read.

**A rename leaves prose behind.** §136 renamed a public method and updated
the two documents it knew about. There is no gate that says *"a method named
in prose still exists"*: the symbol gate checks claims that **name a file**,
and `p.raw` names none.

### The fix

**The quote is the code.** The recipe's block is the current `NODE_MUTE`, and
it says so:

````text
```rust exact
// crates/engine/src/commands.rs
pub static NODE_MUTE: CommandSpec = CommandSpec {
…
```
````

**The gate compares statics too.** `statics_in` and `static_in` read a
`pub static` out of a block and out of the source, and the comparison is
**line for line** — a command spec is quoted whole, because a command that is
one field out of date is a command that does not compile.

The drift is caught with both sides shown:

```text
crates/engine/src/commands.rs: the block at line 35 quotes `NODE_MUTE`, and it is not the one in
crates/engine/src/commands.rs
     the document says:
     …
     the file says:
     …
     Quote the code, or stop calling the block exact.
```

**The other numbers and names are right**: `p.raw_json`, `23` commands, and
the step that says what to bump.

### What now holds it

One mutation in `scripts/mutation-test-gates.sh` — *"prose: a quoted command
spec is not the one in the code"* — which makes the recipe's block stale again
and runs `cargo xtask lists`.

And the gate it uses is the old one, extended: *"every exact block quotes the
code it claims to"*, which had two mutations of its own (§40, §75) and now
covers a third shape of quoted code.

### What is still open

**A block that is not marked exact is not checked.** The recipe's other code
blocks — the `ALL` array, the `deny_unknown` snippet, the test example — are
still plain blocks. They are *illustrations* rather than quotes, and each one
is a copy that can drift. The gate can only check a copy that says it is one.

**The number of commands in prose is still only read in one shape.** *"21
commands"* would be caught; *"21"* inside `[&CommandSpec; 21]` is not. The
recipe's step 5 now says 23, and nothing checks it. A gate for "a number in
an array type in prose is the array's length" is possible, and it is not
written.

**A method named in prose is still not checked.** `p.raw_json` is right today
and `p.raw` was wrong yesterday, and the gate that noticed said nothing. The
narrow version — the tree's own `p.` convention for a `Params` — would be
checkable: a method the tree's source never calls is not a method the prose
should name. It is not written.

### The lesson

**A copy in a document is a copy of the code, and it is checked only if it
says so.** The recipe's block was a copy, the gate existed, and the block did
not claim to be exact — so the gate never looked at it. The marker was the
whole difference.

**The gate that compares code has to know every shape of code.** It compared
functions and structs and not statics, and the tree's central artefact — a
command — is a static. A gate that covers two of three shapes looks complete
and is not.

**A rename has a second half, and the second half is prose.** §136 renamed
`Params::raw` to `Params::raw_json` and updated two of the four documents
that named it. The two it missed were the two a person reads *before* writing
code.

## 141. The tree could hold a design it could not save

Two budgets, two currencies, and no stated conversion between them.

### The measurements

A design and the file that holds it are the same thing in two currencies: the
design's **memory** and the file's **text**. The model budgets the first
(`MAX_DOCUMENT_BYTES`, 32 MiB, §52) and the format budgets the second
(`MAX_FILE_BYTES`, 64 MiB, §49). The number that converts one into the other
was nowhere in the tree — and it was not a constant either.

The file was written pretty: one item per line, **indented by its depth**. The
depth is the design's own — a property tree may nest sixteen deep
(`MAX_PATH_SEGMENTS`), and a keyframe inside it is written nine levels down —
so the indentation grew with the design, and the file grew faster than the
design did.

Measured, over the shapes a design can take:

| the design | its memory | its file | the factor |
|---|---|---|---|
| 9 990 small nodes | 13.23 MiB | 11.43 MiB | **0.86** |
| 512 text values of 65 000 bytes | 31.88 MiB | 31.90 MiB | **1.00** |
| 32 properties × 1 000 keyframes | 2.94 MiB | 7.08 MiB | **2.48** |
| the same, in the deepest property tree | 2.95 MiB | 29.32 MiB | **9.95** |

The factor is a function of the *design*, and it ranges over a factor of
eleven. So the two budgets were not comparable numbers at all. §64's
reasoning — *"the document path is the one place in the tree where the bytes
of a file and the bytes of what it holds are two different numbers with the
tighter one on the memory"* — was true about memory and wrong about
everything else: the looser number was not loose enough.

**The tree could hold a design it could not save.** The largest design the
model accepts — 34 properties × 10 000 keyframes, in the deepest property tree
— is 31.15 MiB of memory, inside the model's 32 MiB budget, and **311.61 MiB**
of text, past the format's 64 MiB cap.

And the shape §51 measured, the one that section wrote down:

| | memory | text |
|---|---|---|
| 32 properties × 10 000 keyframes | **29.31 MiB** — inside the model's budget | **71.35 MiB** — past the format's cap |

The writer refused it:

```text
save_text -> the file would be more than 67108865 bytes, and the limit is 67108864
```

§52's budget was supposed to close §51's open item — *"the engine can still
hold such a document: nothing stops a command from growing the plan past what
can be written"* — and it did not, because the budget counts **memory** and
the refusal counts **text**, and the two are not the same thing.

**A test asserted the defect.** `a_document_too_large_to_read_back_is_not_written`
built that document and checked that the writer refused it. The rule it
enforced was *"a design the model accepts cannot be saved"*.

### Why nothing noticed

**The file's cap was a number somebody chose.** Its own doc comment said so:
*"The cap is generous for a document and small enough to bound the parse."*
That is a judgement, not a derivation, and §64's rule is that the limits are
*"derived from a memory budget rather than chosen on their own"*. The three raw
JSON readers had been re-derived; the design file had been left alone, and
§64's own body called it *"the exception"* without saying what the exception
was for.

**The tree's own documentation claimed the opposite.** `crates/core/src/lib.rs`
said of `MAX_JSON_TEXT_BYTES`:

> This is the number every reader of untrusted JSON in the tree uses: the
> control channel's line limit, the MCP server's message limit, and the CLI's
> script limit.

The design file is the fourth reader of untrusted JSON. It did not use that
number, and it could not: the number is a bound on a raw JSON tree, and a
design file is not one.

**Nothing compared the two currencies.** There was no number in the tree for
"how many bytes of text one byte of design takes", so the model's budget and
the format's cap could not be compared — and two limits that cannot be
compared are two limits that can drift apart silently.

### The fix

**The writer's indentation stops growing.** `crates/io/src/writer.rs` is a
formatter of the tree's own: serde_json's pretty formatter with one change,
that the indentation does not grow past [`MAX_INDENT_DEPTH`] (four levels).
Everything else — where the newlines go, how a string is escaped, how a
number is written — is serde_json's, so the file is the same JSON it always
was, and only the spaces at the start of a line differ.

The factor is now a constant of the format rather than a function of the
design:

| the design | before | after |
|---|---|---|
| 9 990 small nodes | 0.86 | **0.65** |
| 512 text values | 1.00 | **1.00** |
| keyframes, flat | 2.48 | **1.49** |
| keyframes, in the deepest tree | 9.95 | **1.48** |

**The file's cap is derived.** `MAX_FILE_BYTES` is now
`MAX_DOCUMENT_BYTES * FILE_EXPANSION` — the design's own budget times the
format's measured expansion. It is **64 MiB**, the number it has always been,
but it is a consequence rather than a choice, and it cannot drift away from
the budget it protects.

**The invariant is a test.** The document §51 measured is now written and read
back:

```text
32 properties x 10 000 keyframes: memory 29.31 MiB, text 42.05 MiB
save_text -> Ok, 44097000 bytes
```

and the largest design the model accepts is 46.97 MiB of text, inside the
64 MiB cap. The same design's file was 311.61 MiB before, and the reader
refused it.

**The core's claim is true now.** The comment says *raw* untrusted JSON, and
names the design file as the exception, with the reason.

### What now holds it

Three mutations, and three tests:

| Mutation | Caught by |
|---|---|
| the writer indents as deep as the design goes | `a_designs_file_is_bounded_by_the_design_it_holds` |
| the format's expansion is understated | `a_document_the_model_accepts_can_be_written_and_read_back` |
| the file's cap is a number somebody chose | `the_file_cap_is_derived_from_the_designs_budget` |

The third one is the interesting one: the mutation writes the cap as the
literal `64 * 1024 * 1024`, which is the *same number* the derivation gives.
The test in the io crate cannot see it — the two have the same value today —
and the gate can, because the gate reads the source. That is what a gate is
for, and it is the first time in this tree that a unit test and a gate cover
the same rule from two sides.

### What is still open

**The reader still pays for the text it refuses.** A 61 MiB file is refused
correctly and costs **119 MiB** of resident memory, because the reader holds
the text and then builds a document from it — and the document's budget is
checked *after* the parse, not during it. The cap is the same 64 MiB it has
always been, so nothing regressed, but the tree's promise for a hostile file
is still "the text plus the document", and the second half of that promise is
not bounded while the parse runs. Streaming the parse would fix it, and it is
a change to the reader's shape rather than to a number.

**The bound of four levels is a choice.** Below it the file looks as it always
has; below *that* the lines are flat. A design file that does not nest at all
— a property tree stored by path — would make the factor a property of the
data rather than of the depth, and the tree has machinery for a change of
shape (`format/pending`). It is a change of the file format, and the file
format is not changed for a number.

**The file's cap is still twice the design's budget.** That is the format's
expansion (1.51 measured, 2 allowed), and it is what it is: the file is
pretty, and a pretty file is bigger than the design it holds. The bound on
the reader's memory follows from it.

### The lesson

**Two budgets for one thing need a stated conversion, and the conversion has
to be measured.** The model and the format each had a number, the numbers
looked like they were about the same thing, and nothing in the tree said what
one was in units of the other. The conversion was a property of the *design* —
0.86 for one shape and 9.95 for another — and a property of the design is not
a number a limit can be written in.

**A limit checked in one currency cannot bound a limit in another.** The
model's budget was about memory and the format's cap was about text, and §52
believed the first closed the second. It did not, and the way to see that it
did not is to ask what the two numbers mean side by side: 29.31 MiB and
71.35 MiB, of the same design.

**A test that asserts a defect is a test that keeps the defect.** The test
that caught the writer refusing a design the model accepts was written to
prove §51's fix worked, and it did — §51's fix was *"refuse to write what the
reader would refuse"*, and the test enforced exactly that. What it could not
see was that the rule it enforced was the wrong rule: a design the model
accepts should be savable, and the writer should never have been asked.

---

## 142. The crash report wrote the design with a writer of its own

### The measurements

A crash report is **input** for whoever reads it: the recipe
(`docs/recipes/fix-a-crash.md`) asks for `report.json` from whoever had the
crash, so the file arrives from another machine. The reader has a limit for
that reason — `MAX_REPORT_BYTES`, 64 MiB (§72) — and the limit is applied
while reading ✓.

The writer had no limit at all, and it wrote the design with
`serde_json::to_string_pretty` — the tree's layout *without* the bound on the
indentation that §141 measured and bounded.

So a crash report of a design the model accepts:

| the design | its memory | `report.json` | `design.json` | the reader says |
|---|---|---|---|---|
| 32 properties × 10 000 keyframes | 29.31 MiB | **73.51 MiB** | 68.63 MiB | *refused* |
| the same, in the deepest property tree | 31.14 MiB | **311.62 MiB** | 306.42 MiB | *refused* |

```text
show_in -> could not read .../report.json: the input is 77085244 bytes; the limit is 67108864
```

**The app wrote crash reports it could not read back.** And the crash report
is the one thing a crash must produce: it is what carries the design, the
command and the trace, and what the person who reads it needs in order to
reproduce the crash at all.

### Why nothing noticed

**The writer was a copy.** The crash module lives in the engine, the engine
had no dependency on the io crate, and so the crash folder had its own
writer — a copy of the one the design file used, taken before §141 bounded
it. §136's lesson, in a place where nothing noticed: *a copy of a writer is a
copy that drifts*.

**The reader's limit was tested and the writer's output was not.** §72's
tests cover a report file over the limit ✓ — a file that is *too big* ✓ — and
nothing compared what the writer *produced* with what the reader would take.
§51's rule is that a limit belongs to both ends of a pipe, and the crash
folder had one end of it.

**Nothing compared the two designs.** The design on disk and the design in the
crash folder were written by two writers, and no test said they were the same
bytes — because there was nothing to say it.

### The fix

**The tree's JSON has one writer**, and it lives with the other half of the
byte boundary: `craftkit_core::writer`, the twin of the bounded readers. The
writer moved there from the io crate, so the io crate and the engine both use
it and **no new dependency edge was needed** — the engine already depends on
core, and the io crate already does too.

**The crash report's three files are written with it.** `report.json`,
`design.json` and `replay.json` are the tree's layout now, and the same
bytes as the design file's.

The measurements after:

| the design | `report.json` | the reader |
|---|---|---|
| 32 properties × 10 000 keyframes | **44.21 MiB** | reads back |
| the same, in the deepest property tree | **46.98 MiB** | reads back |

### What now holds it

Two mutations, and two tests:

| Mutation | Caught by |
|---|---|
| the crash report is written with an unbounded writer | `a_crash_report_of_a_big_design_can_be_read_back` |
| the crash report is written with a writer of its own | `the_tree_writes_its_json_with_one_writer` |

The second one is the interesting one, and it is the first time in this tree
that the two halves of a rule have been caught by two different things for a
reason that is worth spelling out. The mutation writes the report
**compactly** — which fits, which is why the crash report's own test passes
under it — and the design in the crash folder would no longer be the design
on disk. Only the gate sees that, because the gate reads the source and the
test reads the bytes.

### What is still open

**A session that opened a big design still writes a report the reader
refuses.** The report carries the design *and* the base the trace starts
from, so it carries two designs: **84.11 MiB**, and

```text
show_in -> could not read .../report.json: the input is 88195291 bytes; the limit is 67108864
```

That is the owner's item 13 — *how a big crash report's base is carried* — and
the app has no open path yet (§33), so the case is not reachable today. The
writer's side is fixed; the shape of the report is the decision.

**The crash report's cap is still a number somebody chose.** 64 MiB, written
out as a literal. Item 13 says the derivation is not the decision and the
value is, and this section did not take it.

**The tree still bounds its inputs and not its outputs.** The control
channel's reply for `engine.state` is **4.07 MiB** and for `plan.evaluate`
**3.50 MiB**, on a design of ten thousand nodes — and both are one line of
request. The crash folder is the sharp end of that, but the folder was the
one that could be fixed without deciding what an agent may ask for; the
control channel's replies are item 21 (*what an agent may do with a list that
does not fit*), and they are not small enough to ignore.

### The lesson

**A reader's limit and a writer's output are one contract.** §51 wrote that
down for the design file, and the crash folder had the same two halves with
one of them missing — so the reader's limit was a fact about a file nobody
could produce.

**A copy of a writer is a copy that drifts, and the drift is invisible from
one side.** The crash folder's writer was the design file's writer *before*
§141 bounded it; §141 made the two different, and neither file said so. The
writer moved to the layer that owns the tree's byte boundary so that the copy
is not there to make, and the gate says so for the copies that are made
anyway.

**One design, one writer — and the rule is checkable.** The gate reads the
source, and it is the only thing that can tell a bounded writer of the
tree's own from a bounded writer of its own.

## 143. The recipe for adding a control-channel method taught the rule the tree had already reversed

### The measurement

`docs/recipes/add-a-control-channel-method.md` is the document an agent
follows to add a method to the control channel. Its step 1 said:

> The worked example here is real work this tree needs: all three `ui.*` names
> parse to one variant, `Method::Ui`, and the action inside `params` is what
> distinguishes them — `"render"`, `"screenshot"`, `"click"` and `"widgets"`.

**That is the rule the tree had already reversed.** `Request::ui_action` says
the opposite: *"The **method name decides**, and `params.action` is only
consulted for the bare `ui`, which has no spelling of its own."* The reverse
order is what §32 replaced, because the host used to read the action out of
`params.action` and ignore the method the caller had typed — so
`{"method":"ui.click","params":{"x":400,"y":12}}` rendered a 1180×720 PNG and
answered `rendered 1180x720`, `ok: true`, and no click.

The recipe then taught the implementation that would bring it back. It told the
reader to add a variant:

```rust
pub enum Method {
    // …
    /// Render the current state off-screen, with no window.
    Render,
}
```

`Method` has **eight** variants and none of them is `Render`. The tree's
`Method` is `Execute`, `Describe`, `State`, `Diagnostics`, `Crashes`, `Ui`,
`Ping` and `Shutdown`, and `ui.render` is a **spelling of `Method::Ui`** —
deliberately, and by a test: `the_ui_aliases_all_reach_one_method` names all
four spellings and asserts they reach one method.

And the recipe told the reader to change that test:

> Removing `"ui.render"` from the alias group is the delicate part:
> `the_ui_aliases_all_reach_one_method` names all four strings, so that
> test has to be updated to the new truth in the same change.

### What the instruction does to the tree

So the recipe's six edits were applied to the tree, exactly as written: the
variant, the `as_str` arm, the `parse` arm moved out of the alias group, the
entry in `all()`, a dispatch arm, and the guard test edited to the "new
truth".

```text
test result: FAILED. 58 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
failures:
    server::tests::a_host_handler_answers_ui_instead_of_the_refusal
    server::tests::the_change_hook_fires_for_ui_requests
```

**The two failures are not the test the recipe names.** The recipe says the
delicate part is the alias group and the guard test; the recipe tells you to
edit the guard test, and when you do, the tree is still broken — somewhere the
recipe never mentions. `ui.render` stops reaching `Method::Ui`, so it stops
reaching the **host**, and the host is the only thing that draws the frame.
The reply becomes *"this build has no off-screen renderer"*, in an application
that has one.

And the application does not notice: `cargo test -p craftkit` is green,
**27 passed, 0 failed**, because nothing in the app sends a `ui.render` over
the control channel. Only two of the control crate's own tests send one, and
they are the two that fail.

The tree's own gate noticed something, and it was not the defect:

```text
error: 2 shared-list problem(s):
   • docs/recipes/add-a-control-channel-method.md: cites `crates/control/src/protocol.rs:103`, …
   • the control method `ui.render` is not in the table in docs/recipes/add-a-control-channel-method.md
```

The first of those is an artefact of the mutation shifting the file's lines. The
second is real: `control_methods_are_listed` compares the recipe's **table**
with the code's methods, so it noticed that `ui.render` had become a method of
its own. It said nothing about the variant the recipe invented, and nothing
about the sentence it taught.

### Why nothing noticed

**A symbol named in prose is not checked.** §130's gate checks `a::b` where
the head is a **module**, and says in so many words that a *type* path
(`Session::execute`) is *"not this check's business"*. §140's open item said
the same thing from the other end: *"A method named in prose is still not
checked."*

**The recipe contradicted itself, forty lines apart.** The note above the
worked example says *"A new spelling belongs in `UiAction::method` as well, or
the name is decoration — which is how `ui.click` came to render instead of
click"*, and *"Do not invent a ninth name where one of these fits"* — and then
invents a ninth. The tree's `Method::all` has eight entries, and the note
says so.

**The citations were wrong in the way citations rot.** `crates/control/src/protocol.rs:57`
was cited for `Method::parse`; line 57 is inside `as_str`, and `parse` is at
line 64. `crates/control/src/protocol.rs:103` was cited for `Request::token`;
line 103 is inside `UiAction`'s doc comment, and the field is at line 171.
Neither was caught, because `citation_drift` only reports a citation that
lands on **nothing** — a blank line or a lone brace — and both of these land
on real lines that say something else. That is the gate's stated limit: *"A
citation that lands on real code which says something else is not detectable
by any tool."*

**A limit named by a path is not a limit the limit-gate can see.** The gate
that checks *"a limit in prose is the constant's value"* reads a backticked
name that is **all capitals** — `MAX_ENTRIES` — and follows it with a number.
In a sentence the whole span is the name, and `History::MAX_ENTRIES` is not
all capitals: it is a *path*, so the gate skipped it and the value it claimed
was never compared with anything. The tree has two constants of that name,
128 in the engine's history and 256 in the engine's log, so the path was the
only thing that could have told them apart.

**A rule about a shape is not a rule about a symbol.** "The action inside
`params` is what distinguishes them" is a sentence, and no gate can judge a
sentence. The variant the sentence carried — `Method::Render` — was a symbol,
and that could have been checked.

### The fix

**The recipe teaches the tree's control channel.** Its step 1 is now "Which
shape is this?", and it says the two shapes out loud: a new **method** is a
new variant of `Method`, in three places (`as_str`, `parse`, `all()`); a new
**`ui.*` spelling** is *not* a new variant — it is an `UiAction`, in four
places (`UiAction::parse`, `UiAction::as_str`, `UiAction::method`, and
`Request::ui_action`). The rule is stated as the tree states it — the method
name decides — with §32's reason attached. The guard is stated as a guard:
**it grows, and it is never edited to let a spelling out.**

Its step 2 no longer invents a dispatch arm. The `ui.*` family is not
dispatched in the control crate at all; it is answered by the host, and the
recipe says so and shows the shape. The size of a frame is the host's bound —
`craftkit_ui::raster::MAX_SIZE` and `craftkit_ui::raster::MAX_PIXELS` — not a
range check invented in the wrong crate, and the recipe says *that* too.

**The citations are symbols now.** Three line numbers went; `Method::parse`,
`Request::token` and `MAX_LINE` are named, with the file beside them. That is
the tree's own convention: *"Prefer a symbol over a line number when you write
one. `wasm::run` still means something next year; `wasm.rs:41` does not."*

**And the other document that had the same defect.** `docs/architecture.md`
said *"`History::MAX_ENTRIES` is 128 entries"*. There is no `History::MAX_ENTRIES`:
the constant is module-level, `craftkit_engine::history::MAX_ENTRIES`, and the
engine's **log** has a `MAX_ENTRIES` of its own — 256, not 128. The number
was right and the name was not, and a reader who went looking for
`History::MAX_ENTRIES` would find nothing.

### What now holds it

A gate — `unknown_type_paths` — and two mutations.

The gate is the **twin of §130's `unknown_module_paths`**, and it is the thing
§130's own comment left undone. The rule: a `Type::member` in prose, where
`Type` is a type **this tree defines**, must name a member that type has — a
variant, a field, an item of an `impl` block, or a method a derived trait
provides.

```text
error: 5 shared-list problem(s):
   • docs/architecture.md: line 220 names `History::MAX_ENTRIES`, and History has no member of that name
   • docs/recipes/add-a-control-channel-method.md: line 124 names `Method::Render`, and Method has no member of that name
   • docs/recipes/add-a-control-channel-method.md: line 126 names `Method::Render`, and Method has no member of that name
   • docs/recipes/add-a-control-channel-method.md: line 143 names `Method::Render`, and Method has no member of that name
   • docs/recipes/add-a-control-channel-method.md: line 205 names `Method::Render`, and Method has no member of that name
```

| Mutation | Caught by |
|---|---|
| prose: the recipe names a method the tree does not have | `cargo xtask lists` |
| prose: a limit is named by a path the type does not have | `cargo xtask lists` |

The second one is the one that matters more, because it proves the check is
about the **tree's prose** and not about one document: it is the
`docs/architecture.md` defect, put back.

### What is still open

**The gate checks that a member exists, not that a reader can reach it.**
`docs/design/file-format.md` says *"`Document::next_id` is `#[serde(skip)]`"*.
The field is **private**, so no reader can write that path; the public names
are `Document::peek_next_id` and `Document::reseat_id_counter`. The gate
accepts it, on purpose: a document about the tree's own internals may name a
private field, and `Session::doc_mut` — which is `pub(crate)` — is named
correctly in two other documents. Reachability is a judgement, and this gate
does not make it.

**A rule about a shape is still not checked.** The recipe's sentence about
`params.action` was fixed by hand, because a sentence cannot be gated. What
*could* be gated is the symbol the sentence carried, and that is now.

**The exact-block gate still does not compare enums.** §140 left this open:
*"The gate that compares code has to know every shape of code."* It compares
functions, structs and statics; `Method` is an enum. The recipe no longer
quotes one, so nothing depends on it today.

### The lesson

**A recipe that teaches a rule the tree has already reversed is worse than no
recipe**, because it is read at the moment somebody is about to make the
change. The tree had the fixed rule in `Request::ui_action`, in `UiAction`'s
doc comment, and in a test named `the_ui_aliases_all_reach_one_method` — and
the recipe said the opposite, in the imperative mood.

**A gate that covers half of a path shape is a gate that looks complete.**
§130's gate checks `a::b` and says, in its own comment, that a type path is
"not this check's business". The type path was the business. The two halves
were written by two people and the second one was left out.

**A symbol in prose is checkable; a sentence is not.** The gate finds
`Method::Render` and cannot find *"the action inside `params` is what
distinguishes them"*. That is the right order to fix a document in — the
symbol first, because it is the part a machine can hold, and the sentence
second, because it is the part only a reader can judge.

**The two halves of the `ui.*` family are held by different things.** The
guard test holds the spellings together and the recipe held them apart, and
the recipe was the one that was wrong. A guard test nobody has to change is
worth more than a paragraph nobody has to read.

## 144. The limits table said a design was what the process uses, and the process holds five designs

### The measurement

`crates/model/src/lib.rs` is the tree's table of safety limits, and §74's gate
checks every constant in it is enforced somewhere in the code. Its entry for
`MAX_DOCUMENT_BYTES` said:

> The engine clones the whole document for every command and keeps 128 of them
> for undo, so this is the number that says what the process actually uses.
> §52.

Measured, with a session holding a design at the model's limit — 32
properties with 10 000 keyframes on each, 29.31 MiB estimated:

```text
the design's estimated bytes: 29.31 MiB   (the model's budget is 32.00 MiB)
rss after load_document:      41.74 MiB
   the session's undo depth:   0
rss after ten edits:         101.01 MiB
   the session's undo depth:   2
rss after two hundred edits:  102.06 MiB
   the session's undo depth:   2
peak rss / the model's budget:  3.19x
```

**The undo depth is 2, not 128.** And the documents the session holds, counted
exactly:

```text
   the document being edited:  29.61 MiB
   the trace's base:           29.31 MiB
   the undo and redo stacks:   59.20 MiB  (2.00 documents)
   together:                   118.12 MiB  (3.99 documents)
   the model's budget:         32.00 MiB
   the history's budget:       64.00 MiB  (2.16 documents)
```

Four documents between commands, and **five** while a command runs: the engine
takes its snapshot *before* the command edits anything, and the history is
trimmed when that snapshot is pushed — which is after the command has run. The
peak resident set was **102.69 MiB** (105 160 KiB), which is **3.19 times**
the number the table called "what the process actually uses".

So the sentence was wrong in three ways: the history keeps two snapshots and
not 128, the process holds four or five designs and not one, and the number
it names is a quarter of what the process spends.

### Why nothing noticed

**§52 fixed the file it was in and not the file that quoted it.** §52
replaced the history's *count* of steps with a byte budget — and the comment
in `crates/engine/src/history.rs` says so, at length, in the place where the
history lives:

> **That last paragraph was wrong, and §52 is why.** A count of entries is
> not a bound on memory […] The count is still here (a session can run for
> days), but the *budget* is what bounds the memory now.

The model's table quoted the old paragraph, and §52 did not come back for it.
A gate cannot see this: §74's gate checks that every constant in the table is
**enforced**, and `MAX_DOCUMENT_BYTES` was — perfectly enforced, 32 MiB, all
day. What was false was the sentence around it.

**The cap that decided it was a number somebody chose.** The history's budget
was `64 * 1024 * 1024`, written out as a literal — and §141 had just derived
the design file's cap from the model's budget, with the *same value*, because
the file's expansion is 2. Two caps, the same number, no relation between
them: raise the model's budget and the history's stays where it was.

**Nothing could ask.** §111 added `Session::history_bytes()`, because the
tree measured the bytes on every push and kept the measurement to itself. But
the measurement covered the **stacks** only. The document being edited and the
base the trace starts from were in memory, every second of every session, and
no number anywhere said so.

### The fix

**The table says what it bounds.** `MAX_DOCUMENT_BYTES` bounds *one
document*, and says so, and points at the number that says what the process
uses.

**The history's budget is derived.** `MAX_HISTORY_BYTES` is now
`craftkit_model::limits::MAX_DOCUMENT_BYTES * HISTORY_EXPANSION` — the same
64 MiB, and now it *means* something: two documents. The number did not
change; what it says did.

**The session has a budget of its own**, and it is the number that says what
the process uses:

```rust
pub const SESSION_DOCUMENTS: usize = SESSION_OWN_DOCUMENTS
    + SESSION_SNAPSHOT_DOCUMENTS
    + crate::history::HISTORY_EXPANSION;
pub const MAX_SESSION_BYTES: usize = craftkit_model::limits::MAX_DOCUMENT_BYTES * SESSION_DOCUMENTS;
```

Five documents: the one being edited, the base the trace starts from, the
snapshot a command takes, and the two the stacks hold. **160 MiB** — and the
measured 102.67 MiB is inside it, because the estimate counts the model's
representation and the allocator reuses what the trim frees.

**And a session can be asked.** `Session::document_bytes()` is the number the
table could not give, next to `history_bytes()`:

```rust
pub fn document_bytes(&self) -> usize {
    self.doc.estimated_bytes()
        + self.trace_base.estimated_bytes()
        + self.history.bytes()
}
```

§135's gate stopped it the moment it was added, because it is a new public
method on `Session` and that list is an allow list — a second path is refused
until somebody says what it is. It is a reader, and it is now on the list.

### What now holds it

A gate — `the_memory_budgets_are_derived_from_the_models` — and two mutations,
and three runtime tests.

The gate is the same shape as §141's: the runtime test cannot see a literal
that happens to have the right value today, so the **source** is checked.

| Mutation | Caught by |
|---|---|
| engine: the history's budget is a number somebody chose | `cargo xtask lists` |
| engine: the session's budget forgot the trace's base | `craftkit-engine` test |

The second one is caught by the **test** and not by the gate, and that is the
point of having both: the gate reads the source and sees that the name is
there; the test reads the numbers and sees that the base is not. A budget
that forgot a document has the right *names* in it and the wrong *value*.

```text
the session's budget is 100663296, and the stacks and the three documents need 134217728
```

### What is still open

**The crash report's cap is still a literal.** `MAX_REPORT_BYTES` is
`64 * 1024 * 1024`, and the reader's own budget is `report_memory_budget` —
the text plus twice a document — so reading a 64 MiB report costs 128 MiB of
memory, where every other reader of untrusted JSON in the tree allows itself
32 MiB. Item 13 says the *derivation* is not the owner's decision and the
*value* is; §144 did not take it, because the value depends on the **shape**
of the report — whether the base travels inside it — and that is item 13's
question, not this section's.

**The trace's 8 MiB is still a number somebody chose.** §70 derived
`MAX_TRACE_BYTES` from it — 8 MiB of memory divided by the JSON expansion —
but the 8 MiB itself is a literal, and nothing says what it should be
relative to.

**Nothing measures the peak.** `MAX_SESSION_BYTES` covers the snapshot a
command takes, and the measured 102.67 MiB is below it — but the number that
would *exceed* it was never reached, because the budget is a bound on what
the tree asks for and not on what the allocator does with it. The peak is an
argument, not a measurement.

### The lesson

**A table of limits has sentences in it, and the sentences rot faster than
the constants.** §74's gate checks that every constant in the model's table is
enforced. It cannot check that the *sentence* next to the constant is true,
and §52 made one of them false without noticing, because the file it was in
was not the file it was in.

**A budget that is a literal drifts from the budget it is about.** §141
derived the design file's cap from the model's budget and left the history's
alone — and the two happened to be the same number, so nothing looked wrong
and nothing would have until one of them moved.

**The number that says what the process uses has to exist.** Everything else
follows from it: the table's sentence, the history's expansion, the stacks'
share, and the peak. It did not exist, so four separate facts about memory
were each written down somewhere else, or nowhere.

**Ask the thing.** §111 added `history_bytes()` because the tree measured
the bytes and kept the measurement to itself. The same question about the
whole session — `document_bytes()` — was one method away, and the answer is
the number four documents of memory that no sentence in the tree mentioned.

---

---

---

## What enforces what

The point of this list is that a rule with nothing behind it is a wish. Every
row says who catches the violation, and the middle column is either a mutation
in `scripts/mutation-test-gates.sh` — the only kind of evidence that has ever
caught a gate quietly stopping — or the honest word *nothing*.

| Rule | Enforced by | Caught by |
|---|---|---|
| A crate depends only on strictly lower layers | `cargo xtask layers` | 3 mutations |
| No embedded image; every asset attributed | `cargo xtask assets` | 3 mutations |
| **No source outside Rust** | `cargo xtask assets` | 1 mutation — §6 |
| Crate roots carry the deny attribute and `forbid(unsafe_code)` | `cargo xtask lists` | 2 mutations |
| Documented layer table matches cargo metadata | `cargo xtask lists` | 2 mutations |
| Sorted workspace member list | `cargo xtask lists` | 1 mutation |
| **Prose cites code that still exists** | `cargo xtask lists` | 1 mutation — §7 |
| A format change on disk is registered and applied | `craftkit-io` tests | 1 mutation — §5 |
| **Control channel is loopback-only** | `craftkit-control` tests | 3 mutations — §4, §21 |
| `cargo xtask ci` runs what it reports running | `xtask` tests + the closing line | 2 mutations — §2 |
| The canvas, the bundle path and the worker's rules agree | `cargo xtask web` | 4 mutations |
| A browser build is at least attempted | `cargo xtask wasm` | 4 mutations, 1 un-catchable — §11, §13, §22 |
| **A green compile check says the bundle is not built** | `wasm::post_build_note` | 1 mutation — §13 |
| **An unverifiable bundler counts as blocked, not ready** | `doctor::bundler_status` | 1 mutation — §22 |
| **A mutation's target is snapshotted and restored** | `guarded` in the mutation script | probe mutation — §23 |
| **A mutation that does not apply stops the run** | `swap` in the mutation script | the self-test's second half — §46 |
| **A stop request ends the MCP server, not just says ok** | `craftkit-mcp` test + `mcp_stdio.rs` | 2 mutations — §24 |
| **The failure box names the command that builds the page** | `cargo xtask web` | 1 mutation — §25 |
| **A document never names a command the registry lacks** | `cargo xtask lists` | 1 mutation — §26 |
| **The prose walk descends into every subdirectory** | the fabricated-id case above | 1 mutation — §26 |
| **The crash folder can be relocated, so the pipeline is testable** | `crash_pipeline.rs` | 1 mutation — §27 |
| **The help shows the script format it requires** | `usage_tests` | 1 mutation — §28 |
| **The save note is a count, not the plan** | `stash_note_tests` | 1 mutation — §31 |
| **The ui method name decides what the request does** | `the_ui_action_comes_from_the_method_name` | 1 mutation — §32 |
| **A click's reply does not claim to have clicked** | `click_message` | 1 mutation — §32 |
| **A control-channel example in prose could be sent** | `cargo xtask lists` | 1 mutation — §32 |
| **A document reference in prose resolves** | `cargo xtask lists` | 1 mutation — §34 |
| **Every command's effect survives the file** | `command_effects_survive_the_file` | 1 mutation — §35 |
| **A citation names a symbol, not a bare line** | `cargo xtask lists` | 1 mutation — §36 |
| **The README's layer table matches the code** | `readme_layers` | 1 mutation — §37 |
| **The CLI's help lists every subcommand** | `the_help_lists_every_subcommand_the_parser_accepts` | 1 mutation — §38 |
| **A symbol claimed to be in a file is in it** | `symbol_file_claims` | 1 mutation — §39 |
| **An exact block quotes the code it claims to — functions, structs and statics** | `quoted_code_matches_the_source` | 4 mutations — §40, §75, §140 |
| **A line count in prose is the file's length** | `prose_line_counts_match_the_files` | 2 mutations — §41 |
| **A limit in prose is the constant's value** | `prose_constants_match_the_code` | 2 mutations — §41 |
| **Code prose attributes to a file is in that file** | `attributed_code_is_in_the_file` | 2 mutations — §42 |
| **A rollback restores the document *and the plan*** | `loading_a_document_re_evaluates_the_plan` | 1 mutation — §43 |
| **A rollback puts the dirty flag back** | `a_restore_puts_the_dirty_flag_back` | 1 mutation — §43 |
| **A command that changes nothing is refused** | `a_command_that_says_it_changes_the_document_and_does_not_is_an_error` | 1 mutation — §44 |
| **A refusal and a panic are both logged** | `a_refused_command_is_logged_as_a_failure` | 2 mutations — §44 |
| **The pending steps run in order, each seeing the last** | `the_steps_run_in_order_and_each_sees_the_previous_one` | 1 mutation — §45 |
| **A citation that names a test points at that test** | `citations_of_tests_point_at_the_test` | 1 mutation — §45 |
| **A restore cannot serve a state from the document it replaced** | `a_restore_cannot_serve_a_state_from_the_document_it_replaced` | 1 mutation — §40 |
| **A crash report replays onto the document its commands were relative to** | `a_session_that_opened_a_document_replays_onto_it` | 1 mutation — §47 |
| **A replay says whether it reproduced the design** | `a_replay_says_whether_it_reproduced_the_document` | 3 mutations — §47 |
| **The trace keeps the parameters exactly** | `the_trace_keeps_the_parameters_exactly` | 1 mutation — §47 |
| **A bounded trace says how many commands it dropped** | `a_long_session_reports_how_many_commands_it_dropped` | 1 mutation — §47 |
| **Opening a document clears the undo stack of the previous one** | `opening_a_document_clears_the_undo_stack` | 1 mutation — §47 |
| The control channel's crash replay says the same | `the_crash_replay_says_whether_it_reproduced_the_document` | 1 test — §47 |
| **A crash report file is read within a bound** | `a_report_larger_than_the_limit_is_refused` | 1 mutation — §48 |
| **A replay runs at most a bounded number of steps** | `a_replay_with_too_many_steps_is_refused_before_it_runs` | 1 mutation — §48 |
| **A report's base is validated like a design** | `a_report_with_a_corrupt_base_is_refused` | 1 mutation — §48 |
| **A limit is applied before the allocation** | `a_bounded_read_never_asks_for_more_than_the_limit` | 1 mutation — §49 |
| **The bytes a reader keeps never exceed the limit** | `the_bytes_kept_never_exceed_the_limit` | 1 mutation — §49 |
| **The writer refuses a document the reader would refuse** | `the_writer_refuses_a_document_the_reader_would_refuse` | 1 mutation — §50 |
| **A command may not leave a document the file format refuses** | `leaves_a_document_the_file_would_refuse` | 1 mutation — §50 |
| **A group id may repeat at a different depth** | `a_group_id_may_repeat_at_a_different_depth` | 1 mutation — §50 |
| **Two groups with one id among siblings are refused** | `two_siblings_with_the_same_id_are_refused` | 1 mutation — §50 |
| **A command's effect survives the file, at two depths** | `a_group_name_at_two_depths_survives_the_file` | 1 mutation — §50 |
| **The writer never writes a file the reader would refuse** | `a_document_too_large_to_read_back_is_not_written` | 1 mutation — §51 |
| **The writer's limit stops the text while it is written** | `the_writer_keeps_no_more_than_the_limit` | 1 mutation — §51 |
| **A document over the byte limit is refused** | `a_document_over_the_byte_limit_is_refused` | 1 mutation — §52 |
| **The size estimate counts the content** | `the_history_keeps_its_memory_within_a_budget` | 1 mutation — §52 |
| **The undo stack keeps its memory within a budget** | `the_history_keeps_its_memory_within_a_budget` | 1 mutation — §52 |
| **The fingerprint sees every part of the node** | `the_fingerprint_sees_every_part_of_the_node` | 1 mutation — §53 |
| **A changed keyframe invalidates the evaluator's cache** | `a_node_whose_keyframes_changed_is_not_served_from_the_cache` | 1 mutation — §53 |
| **A second instance does not take the first one's token file** | `a_second_instance_does_not_take_the_first_ones_token_file` | 1 mutation — §54 |
| Prose names real test symbols | *nothing* — §29 | a sweep found it; a gate would be a heuristic |
| **The page imports a bundle the build actually produced** | `cargo xtask web` + a CI step | 2 mutations — §11 |
| **Every command carries a label, a doc string and typed params** | a doc-test on `catalogue()` | doctest — §14 |
| The `.craftkit` shapes a hand-written file needs | doc-tests on `save_text`/`load_text` | doctests — §14 |
| MCP tools track the engine catalogue | `craftkit-mcp` test | assertion, not a mutation |
| **Every easing returns its documented value** | `craftkit-model` tests | 1 mutation — §8 |
| **A source sets the level; the chain starts at unity** | `craftkit-eval` tests | 2 mutations — §9 |
| Gain saturation and chain refusal stay deliberately different | `craftkit-kernel` tests | 1 test — §10 |
| A panicking command restores the document | `craftkit-engine` tests | 4 dedicated tests |
| A crash report is written before the process dies | `apps/craftkit` panic hook | 1 test |
| The control channel's token file is `0600` | `craftkit-control` test | — |
| **A directory the tool did not create keeps its permissions** | `craftkit-control` tests | 1 mutation — §55 |
| **The token file is private whatever the directory's mode** | `craftkit-control` tests | 2 mutations — §55 |
| **A token file that already exists is replaced, not kept** | `craftkit-control` tests | 1 mutation — §55 |
| **The live-window check measures this app's window** | `live-window-check.sh --self-test` | 1 mutation — §56 |
| **A crash report holds the design, and the design is private** | `craftkit-engine` tests | 3 mutations — §57 |
| **A crash report's id says when it happened, and sorts** | `craftkit-engine` tests | 2 mutations — §58 |
| **A second crash never destroys the first crash's report** | `craftkit-engine` tests | 1 mutation — §58 |
| **A frame's area is bounded, not just its sides** | `craftkit-ui` tests | 1 mutation — §59 |
| **A tick is never wider than a reply can carry** | `craftkit-core` + `craftkit-engine` tests | 1 mutation — §60 |
| **A file whose range no reply can carry is refused** | `craftkit-model` + `craftkit-io` tests | 2 mutations — §60 |
| **Saving the same plan twice gives the same file** | `craftkit-io` test | 1 test — §61 |
| **A recovery on one thread is counted on another** | `craftkit-kernel` test | 1 mutation — §62 |
| **The statistics count a recovery from another thread** | `craftkit-engine` test | 1 mutation — §62 |
| **A guarded call on another thread does not guard this one** | `craftkit-kernel` test | 1 test — §62 |
| **A token with padding on the end is not a token** | `craftkit-control` test | 1 mutation — §63 |
| **The channel refuses a padded token** | `craftkit-control` test | 1 mutation — §63 |
| **A text limit fits the memory budget once the parse has expanded it** | `craftkit-core` test | 2 mutations — §64 |
| **The expansion covers the real size of a parsed value** | `craftkit-core` test | 1 test — §64 |
| **The control channel's line limit is the tree's limit** | `craftkit-control` test | 1 mutation — §64 |
| **The MCP server's message limit is the tree's limit** | `craftkit-mcp` test | 1 mutation — §64 |
| **The CLI's script limit is the tree's limit** | `craftkit-cli` test | 1 mutation — §64 |
| **An overshooting easing stays inside the range a stored number may take** | `craftkit-model` test | 1 mutation — §65 |
| **An animated amount never takes the level out of range** | `craftkit-eval` test | 1 mutation — §65 |
| **A widget name owns its bytes** | `craftkit-ui` test | 1 test — §66 |
| **Building a widget name does not cost the process memory** | `craftkit-ui` test | 1 mutation — §66 |
| **The worker precaches the bundle the page imports** | `cargo xtask web` + `xtask` tests | 1 mutation — §67 |
| **The undo budget counts the memory the stacks hold** | `craftkit-engine` tests | 2 mutations — §68 |
| **A discarded redo branch gives its memory back** | `craftkit-engine` test | 1 mutation — §68 |
| **The failure count is every failure, not only the ones in the ring** | `craftkit-engine` test | 1 mutation — §69 |
| **The statistics count every command** | `craftkit-engine` test | 1 mutation — §69 |
| **A command too big for the trace is refused** | `craftkit-engine` test | 1 mutation — §70 |
| **The trace's budget is derived from the memory budget** | `craftkit-engine` test | 1 mutation — §70 |
| **The instrument's closing ratio is computed, not written by hand** | `cargo xtask lists` | the fabricated-ratio case — no mutation: the instrument cannot be mutated while it runs — §71 |
| **A report whose design is not a document is refused** | `craftkit-engine` test | 1 test — §72 |
| **A report the old tree wrote still reads** | `craftkit-engine` test | 1 test — §72 |
| **Reading a report costs no more than its text and one document** | `craftkit-engine` test | 1 mutation — §72 |
| **The size estimate counts a property's value** | `craftkit-model` test | 1 mutation — §73 |
| **A value that breaks an invariant is caught** | `craftkit-core` test | 1 test — §73 |
| **A document with a value the constructors would refuse is invalid** | `craftkit-model` test | 1 test — §73 |
| **A file whose value is out of range is refused** | `craftkit-io` test | 1 mutation — §73 |
| **Every limit in the model's table is enforced in code** | `cargo xtask lists` | 1 mutation — §74 |
| **A panic in the drawing code writes a report** | `craftkit` test | 1 mutation — §76 |
| **A number of commands in prose is the registry's number** | `cargo xtask lists` | 1 mutation — §77 |
| **Every boundary that catches a panic is declared** | `cargo xtask lists` | 1 mutation — §78 |
| **The format's example file is what the document says** | `craftkit-cli` test | 1 mutation — §79 |
| **A keyframe's time, value and easing are each seen on their own** | `craftkit-model` test | 1 mutation — §80 |
| **Every field the fingerprint walks is a field the hash reads** | `cargo xtask lists` | 1 mutation — §80 |
| **A changed easing is not served from the cache** | `craftkit-eval` test | 1 mutation — §80 |
| **A pending change's name is one word, and it is a module** | `every_pending_name_is_a_plausible_file_name` | 1 mutation — §81 |
| **Every listed change has a step, and is declared** | two `pending.rs` tests | 1 mutation — §81 |
| **A file is given only the changes it has not had** | `craftkit-io` test | 1 mutation — §81 |
| **The control channel's methods are the methods the prose lists** | `cargo xtask lists` | 1 mutation — §82 |
| **No two commands advertise the same chord** | `craftkit-engine` test | 1 mutation — §83 |
| **The history stays inside its memory budget, whoever moves a document** | `craftkit-engine` test | 1 mutation — §84 |
| **Every property the tree holds is named by a path the parser accepts** | `craftkit-model` test | 1 mutation — §85 |
| **A tree deeper than a path is refused, not truncated** | `craftkit-model` test | 1 mutation — §85 |
| **A path names the property the listing published it for** | `craftkit-model` test | 1 mutation — §85 |
| **The read side and the write side resolve an id the same way** | `craftkit-model` test | 1 mutation — §85 |
| **A path that names an id or a position cannot create anything** | model and engine tests | 2 mutations — §85 |
| **A property set by its id is the property the id names** | `craftkit-engine` test | 1 mutation — §85 |
| **A path too long to name is refused** | `craftkit-engine` test | 1 mutation — §85 |
| **A created property is readable by the path that created it** | model and engine tests | — |
| **The live-window check reads this app's token, not the last run's** | `live-window-check.sh --self-test` | 1 mutation — §86 |
| **The end-to-end MCP test says what the server said and how it exited** | `apps/craftkit-cli/tests/mcp_stdio.rs` | *nothing* — it only shows itself when the test fails — §87 |
| **A name the grammar cannot write is refused, and the reason comes from the grammar** | model and io tests | 3 mutations — §88 |
| **Every property is named by a path that names it** | `craftkit-model` test | 1 mutation — §88 |
| **The command can set any rate a document may carry** | `craftkit-engine` test | 2 mutations — §89 |
| **Whether a rate is exact is a fact the model can state** | `craftkit-core` test | 1 mutation — §90 |
| **A keyframe list is sorted, unique and within the limit** | `craftkit-model` test | 3 mutations — §91 |
| **A keyframe's time is one a reply could carry** | model and io tests | 1 mutation — §91 |
| **A keyframe's easing is one the model accepts** | model and io tests | 2 mutations — §91 |
| **A file carrying a broken keyframe list is refused** | `craftkit-io` test | 1 mutation — §91 |
| **A property with broken keyframes is refused** | `craftkit-model` test | 1 mutation — §91 |
| **An unsorted list samples to the wrong value** | `craftkit-model` test | 1 mutation — §91 |
| **A command cannot store an easing the model refuses** | `craftkit-model` test | 1 mutation — §91 |
| **A name is not empty, not blank and within the limit** | `craftkit-model` test | 2 mutations — §92 |
| **The constructor refuses a name the validator would refuse** | `craftkit-model` test | 1 mutation — §92 |
| **A file carrying an unusable node name is refused** | `craftkit-io` test | 1 mutation — §92 |
| **A user's bad parameter is a refusal, not an internal error** | `craftkit-engine` test | 2 mutations — §92 |
| **Every command that takes a name agrees about a name** | `craftkit-engine` test | 2 mutations — §92 |
| **A parameter that was given but is empty says so** | `craftkit-engine` test | 1 mutation — §92 |
| **Every documented parameter is fired with a hostile value** | `craftkit-engine` test | 1 mutation — §92 |
| **The engine's backstop still refuses an invalid document** | `craftkit-engine` test | 1 mutation — §92 |
| **A command asked to do what it has already done succeeds** | `craftkit-engine` test | 3 mutations — §93 |
| **Keying and easing what is already there succeeds** | `craftkit-engine` test | 1 mutation — §93 |
| **A command that claims to change something and does not is refused** | `craftkit-engine` test | 1 mutation — §93 |
| **A write that changes nothing says so** | model and engine tests | 2 mutations — §93 |
| **A command that does not mutate is never undoable** | `craftkit-engine` test | 1 mutation — §94 |
| **The changes a file has had that this build does not have are worked out** | `craftkit-io` test | 1 mutation — §95 |
| **A file at the current version naming an unknown change is refused** | `craftkit-io` test | 1 mutation — §95 |
| **A file naming a change this build has is read** | `craftkit-io` test | 1 mutation — §95 |
| **An earlier file may name a change that was folded** | `craftkit-io` test | 1 mutation — §95 |
| **A command that declares it does not mutate and writes is refused** | `craftkit-engine` test | 1 mutation — §96 |
| **A command that declares it does not mutate cannot leave an invalid document** | `craftkit-engine` test | 1 mutation — §96 |
| **A group id longer than a name is refused** | `craftkit-model` test | 1 mutation — §97 |
| **The longest path the grammar can express fits the path budget** | `craftkit-model` test | 1 mutation — §97 |
| **A name too long is refused in either spelling** | `craftkit-model` test | 1 mutation — §97 |
| **A path longer than a name is still carried** | `craftkit-engine` test | 1 mutation — §97 |
| **A file with a group id longer than a name is refused** | `craftkit-io` test | 1 mutation — §97 |
| **Publishing a widget does not cost more than the widgets before it** | `craftkit-ui` test | 1 mutation — §98 |
| **A level bar and a property never share a name** | `craftkit-ui` test | 1 mutation — §99 |
| **A name two widgets want is recorded as a collision** | `craftkit-ui` test | 1 mutation — §99 |
| **A frame publishes no name twice** | `craftkit-ui` test | 1 mutation — §99 |
| **Every property of the selected node has an editor** | `craftkit-ui` test | 1 mutation — §99 |
| **A property named after the level bar keeps its own editor** | `craftkit-ui` test | 1 mutation — §99 |
| **A level bar publishes a rectangle with an area** | `craftkit-ui` test | 1 mutation — §99 |
| **The registry holds the frame that was drawn** | `craftkit-ui` test | 1 mutation — §99 |
| **Every node row has its buttons named** | `craftkit-ui` test | 1 mutation — §100 |
| **The timeline has a name** | `craftkit-ui` test | 1 mutation — §100 |
| **The key fields have names** | `craftkit-ui` test | 1 mutation — §100 |
| **Every published name answers at its own centre** | `craftkit-ui` test | 1 mutation — §100 |
| **A name never promises a widget that is not drawn** | `craftkit-ui` test | 1 mutation — §100 |
| **A timing gate does not fail when the machine is busy** | `craftkit-ui` test, measured under load | the §98 mutation — §101 |
| **A mutation that does not compile is not a catch** | the mutation script | its third outcome, and the self-test's third half — §102 |
| **A skipped phase is not counted as one that ran** | `xtask` test | 1 mutation — §102 |
| **The phase list is the documented one** | `xtask` test | 1 mutation — §102 |
| **What a command costs is measured, and says which part of it is the restore** | `Session::execute`'s doc comment | — |
| **A click by name lands on that widget** | `craftkit-ui` test | 1 mutation — §104 |
| **A name that is not on screen is refused** | `craftkit-ui` test | 1 mutation — §104 |
| **A disabled widget is not the same as one that is not there** | `craftkit-ui` test | 1 mutation — §104 |
| **A click with neither a name nor a point is refused** | `craftkit` test | 1 mutation — §104 |
| **A click with both a name and a point is refused** | `craftkit` test | 1 mutation — §104 |
| **The widgets reply says where the widgets are** | `craftkit` test | 1 mutation — §104 |
| **A selection made in one frame is there in the next** | `craftkit-ui` test | 2 mutations — §105 |
| **A path typed into the key field is still there in the next frame** | `craftkit-ui` test | 2 mutations — §105 |
| **A host cannot forget the shell's state** | the compiler — the state is an *input* to `frame` | — |
| **The state decides what is on the screen** | `craftkit-ui` test | 1 mutation — §106 |
| **A property editor can be clicked by name when a node is selected** | `craftkit-ui` test | 1 mutation — §106 |
| **The property editors are on the screen when a node is selected** | `craftkit` test | 1 mutation — §106 |
| **The widgets reply is the screen the window is showing** | `craftkit` test | 1 mutation — §106 |
| **The channel reports the window's screen over the socket** | `craftkit` test | 1 mutation — §106 |
| **A selected node that is not there is refused** | `craftkit` test | 1 mutation — §106 |
| **A `selected` that is not a node id is refused** | `craftkit` test | — |
| **A click on a disabled widget is refused** | `craftkit-ui` test | 1 mutation — §107 |
| **A click on a disabled widget is refused however it is asked for** | `craftkit` test | — |
| **The widgets reply says which widgets cannot be clicked** | `craftkit` test | 1 mutation — §107 |
| **The answer at a point says whether it can be clicked** | `craftkit-ui` test | 1 mutation — §107 |
| **Only the widgets that cannot be clicked are in the dead list** | `craftkit-ui` test | — |
| **A widget that is not there is refused differently from one that is dead** | `craftkit-ui` test | 1 mutation — §107 |
| **The state carries the document's own parameters** | `craftkit-engine` test | 2 mutations — §108 |
| **The state and the range command agree about the rate** | `craftkit-engine` test | 1 mutation — §108 |
| **Reading the rate does not change the document** | `craftkit-engine` test | — |
| **The state carries the commands that ran** | `craftkit-engine` test | 2 mutations — §109 |
| **The log and the command count agree** | `craftkit-engine` test | 1 mutation — §109 |
| **A note over the bound is refused** | `craftkit-model` test | 1 mutation — §110 |
| **A note command with no text is refused** | `craftkit-engine` test | — |
| **A note command over the bound is refused** | `craftkit-engine` test | 1 mutation — §110 |
| **Writing the note the document already has changes nothing** | `craftkit-engine` test | 2 mutations — §110 |
| **The state carries the note** | `craftkit-engine` test | 1 mutation — §110 |
| **The state and the document agree about the note** | `craftkit-engine` test | — |
| **The note is on the screen** | `craftkit-ui` test | 1 mutation — §110 |
| **The window's note is a glimpse, and says so** | `craftkit-ui` test | 1 mutation — §110 |
| **The CLI says what the design's note is** | `craftkit-cli` test | 1 mutation — §110 |
| **The state carries the undo stack's depth and its memory** | `craftkit-engine` test | 2 mutations — §111 |
| **The state and the statistics agree about the undo stack** | `craftkit-engine` test | 1 mutation — §111 |
| **The browser panel says whether a plan is saved** | `craftkit-web` test | 1 mutation — §112 |
| **A refused save is a warning and a successful one is not** | `craftkit-web` test | 1 mutation — §112 |
| **The panel's outcome is not the shell's outcome** | `craftkit-web` test | 1 mutation — §112 |
| **The panel can be read** | `craftkit-web` test | 1 mutation — §112 |
| **The crash reports are a tool an agent can call** | `craftkit-mcp` test | 2 mutations — §113 |
| **An action that does not exist is refused** | `craftkit-mcp` test | 1 mutation — §113 |
| **The three crash actions are one rule** | `craftkit-engine` test | 1 mutation — §113 |
| **The guard depth balances** | `craftkit-kernel` test | 1 mutation — §114 |
| **A guarded call on one thread does not guard another** | `craftkit-kernel` test | 1 mutation — §62 |
| **A recovery on one thread is counted on another** | `craftkit-kernel` test | 1 mutation — §62 |
| **The design's frames are its range, not its duration** | `craftkit-model` test | 1 mutation — §115 |
| **The timeline reaches both ends of the design** | `craftkit-ui` test | 1 mutation — §115 |
| **The timeline says where the playhead is** | `craftkit-ui` test | 1 mutation — §115 |
| **The readout says the frame and the time** | `craftkit-ui` test | 1 mutation — §115 |
| **The playhead is inside the design** | `craftkit-engine` test | 2 mutations — §115 |
| **The range command says when it moved the playhead** | `craftkit-engine` test | 1 mutation — §115 |
| **The catalogue says which parameter may be given instead** | `craftkit-engine` test | 1 mutation — §116 |
| **An alternative is a parameter of the same command** | `craftkit-engine` test | — |
| **The schema says one of the two** | `craftkit-mcp` test | 1 mutation — §116 |
| **The node parameter is a whole number** | `craftkit-mcp` test | 1 mutation — §116 |
| **A node may be addressed by its name** | `craftkit-engine` test | 1 mutation — §116 |
| **The catalogue's listing names the alternative** | `craftkit-cli` test | 1 mutation — §116 |
| **The keyframes of a property can be listed** | `craftkit-engine` test | 1 mutation — §117 |
| **The listing says when the keyframes are** | `craftkit-engine` test | 1 mutation — §117 |
| **The listing refuses a property that is not there** | `craftkit-engine` test | — |
| **The panel says when the keyframes are** | `craftkit-ui` test | 1 mutation — §117 |
| **No toolbar button is refused for wanting a parameter** | `craftkit-ui` test | 1 mutation — §118 |
| **A button that needs parameters opens a form** | `craftkit-ui` test | 1 mutation — §118 |
| **A button that needs a node asks for one** | `craftkit-ui` test | — |
| **A button whose parameters the shell knows runs at once** | `craftkit-ui` test | — |
| **The form runs the command when it is filled in** | `craftkit-ui` test | 1 mutation — §118 |
| **The form says what is wrong with what was typed** | `craftkit-ui` test | — |
| **The form is on the screen when it is open** | `craftkit-ui` test | 1 mutation — §118 |
| **What a person typed becomes what the reader accepts** | `craftkit-engine` test | 1 mutation — §118 |
| **Every name a command asks for is one of its parameters** | `craftkit-engine` test | — |
| **A command never asks for the same parameter twice** | `craftkit-engine` test | — |
| **Every parameter a command needs is one it asks for** | `craftkit-engine` test | 1 mutation — §118 |
| **A key outside the design says so** | `craftkit-engine` test | 1 mutation — §119 |
| **A key before the design says so** | `craftkit-engine` test | 1 mutation — §119 |
| **A key inside the design does not say it is outside** | `craftkit-engine` test | — |
| **The diagnostics report a key the playhead cannot reach** | `craftkit-engine` test | — |
| **A keyframe outside the design is a warning** | `craftkit-eval` test | 1 mutation — §119 |
| **A keyframe outside the design is reported at both ends** | `craftkit-eval` test | — |
| **A keyframe inside the design is not a warning** | `craftkit-eval` test | — |
| **The catalogue says which pairs one of must be given** | `craftkit-engine` test | 1 mutation — §120 |
| **A command that needs one of a pair refuses with neither** | `craftkit-engine` test | — |
| **A pair prints as one line that says one of the two is required** | `craftkit-cli` test | 1 mutation — §120 |
| **A pair prints as one line and the second spelling does not print its own** | `craftkit-cli` test | 1 mutation — §120 |
| **The catalogue's listing names the pairs** | `craftkit-cli` test | — |
| **The timeline marks the keyframes** | `craftkit-ui` test | 1 mutation — §121 |
| **A keyframe's mark is where the playhead goes when it is clicked** | `craftkit-ui` test | 1 mutation — §121 |
| **The rail follows the panel** | `craftkit-ui` test | 1 mutation — §121 |
| **A key outside the design has no mark on the rail** | `craftkit-ui` test | 1 mutation — §121 |
| **Listing a node's properties is not quadratic** | `craftkit-model` test | 1 mutation — §122 |
| **A listing of many siblings numbers the duplicates** | `craftkit-model` test | 1 mutation — §122 |
| **A keyframe inside a group is marked on the rail** | `craftkit-ui` test | 1 mutation — §122 |
| **The properties panel does not copy the keyframes it does not draw** | `craftkit-ui` test | 1 mutation — §123 |
| **The properties panel does not copy the keyframe values** | `craftkit-ui` test | 1 mutation — §123 |
| **The panel publishes one editor for each property and no other** | `craftkit-ui` test | 1 mutation — §123 |
| **A locked property may not be written** | `craftkit-engine` test | 1 mutation — §124 |
| **A locked property's editor is published as disabled** | `craftkit-ui` test | 1 mutation — §124 |
| **A locked property is drawn greyed** | `craftkit-ui` test | 1 mutation — §124 |
| **A property that does not fit can be brought into view** | `craftkit-ui` test | 1 mutation — §126 |
| **The wheel scrolls the list from anywhere across the panel** | `craftkit-ui` test | 1 mutation — §126 |
| **The key editor stays on screen when the node has many properties** | `craftkit-ui` test | 1 mutation — §126 |
| **The node list carries the level the plan resolved** | `craftkit-ui` test | 1 mutation — §127 |
| **The node list does not scan the plan once per node** | `craftkit-ui` test | 1 mutation — §127 |
| **The node list draws the rows the collector found** | `craftkit-ui` test | 1 mutation — §127 |
| **The wheel scrolls the node list from anywhere across the panel** | `craftkit-ui` test | 1 mutation — §128 |
| **A reused node still reports its problem** | `craftkit-eval` test | 1 mutation — §129 |
| **A module path in prose names a module this tree has** | `cargo xtask lists` | 1 mutation — §130 |
| **The shell may not own a window** | `cargo xtask layers` | 1 mutation — §131 |
| **The shell may read the plan and may not evaluate the document** | `cargo xtask lists` | 1 mutation — §132 |
| **A server that has exited is not a fault** | `craftkit-cli` test | 1 mutation — §133 |
| **The rows are not collected a scan at a time, on a busy machine either** | `craftkit-ui` test | 1 mutation — §134 |
| **There is no second path into the design** | `cargo xtask lists` | 3 mutations — §135 |
| **A panel never puts a document into the session** | `cargo xtask lists` | 1 mutation — §135 |
| **The playhead is a place in the design** | `craftkit-engine` test | 2 mutations — §135 |
| **A parameter is never indexed** | `cargo xtask lists` | 1 mutation — §136 |
| **Every clippy setting names a denied lint** | `cargo xtask lists` | 1 mutation — §136 |
| **An id is unique across the whole document, even at the ceiling** | `craftkit-core` + `craftkit-model` tests | 2 mutations — §137 |
| **The shell's state holds no value the evaluator computes** | `cargo xtask lists` | 2 mutations — §138 |
| **A panel never keeps a copy of the plan** | `cargo xtask lists` | 1 mutation — §138 |
| **Every parameter a command reads is declared** | `cargo xtask lists` | 1 mutation — §139 |
| **Every parameter a command declares is read** | `cargo xtask lists` | 1 mutation — §139 |
| **A design the model accepts can always be saved** | `craftkit-io` test | 1 mutation — §141 |
| **A design's file is bounded by the design it holds** | `craftkit-io` test | 1 mutation — §141 |
| **The design file's cap is the design's budget, not a number of its own** | `cargo xtask lists` | 1 mutation — §141 |
| **A crash report of a design the model accepts can be read back** | `craftkit-engine` test | 1 mutation — §142 |
| **The tree's JSON has one writer, and the places that write a design use it** | `cargo xtask lists` | 1 mutation — §142 |
| **A type path in prose names a member the type has** | `cargo xtask lists` | 2 mutations — §143 |
| **A `ui.*` spelling reaches `Method::Ui`, and the guard grows** | `the_ui_aliases_all_reach_one_method` | — |
| **The recipe teaches the method name deciding, not `params.action`** | *nothing* | a sentence cannot be gated — §143 |
| **The history's budget is the model's budget times two** | `cargo xtask lists` | 1 mutation — §144 |
| **The session's budget covers the stacks and the three documents beside them** | `craftkit-engine` test | 1 mutation — §144 |
| **A session says what its documents cost** | `craftkit-engine` test | — |
| A wrong or missing token is refused | `craftkit-control` tests | — |
| **A command tells the window the document changed** | `craftkit-control` tests + `scripts/live-window-check.sh` | 1 mutation — §21 |
| Every UI icon is drawn in code | `cargo xtask assets` + `crates/ui` | icon table is data-driven |
| `ui.click` reports, it does not act | `click_message` | 1 mutation — §17, §32 |
| No commercial product's data, code or artwork | *nothing* | a gate cannot read provenance |
| Independent of any other project's source | *nothing* | a process, not a check |

The three `nothing` rows are not oversights, and there were four until §32. The
last two cannot be checked by
any tool: whether a measurement came from a commercial CAD package is a question
about where someone got it, and a rule that pretends otherwise is worse than one
that admits it. §17's `ui.click` was checkable in principle and documented
instead, on the grounds that the useful behaviour is the absence of a capability
and a test for "this does not happen" is a test that can be deleted by the person
it constrains. §32 found that the reasoning was right and the *sentence* was not:
the one thing the caller reads said `clicked`. That is now
`click_message`, mutation-tested.
Prose-cited test symbols (§29) could be gated, and deliberately are not: every
rule that separates `the_pending_list_is_sorted` from `catch_unwind` is a
heuristic that would eventually flag something real, and a gate that cries wolf
is this document's §1 in a new costume.

Where the middle column says *a test* rather than a mutation, the honest reading
is: the property is checked, but nothing yet proves the check still bites. That
is the same gap the mutation script closed for `layers`, `assets`, `lists`, `web`,
`ci`, `io` and `control`, and it is why adding a mutation is the first item in
`CONTRIBUTING.md`'s gate rule.

## Suggested order, if someone picks this up

1. ~~**Run `scripts/mutation-test-gates.sh` in CI.**~~ **Done.** The `mutation`
   job in `.github/workflows/ci.yml` breaks each rule and requires the gate to
   notice, and asserts `--self-test` fails before it trusts the full run. Every
   gap in §1 was found by this script, and a gate nobody has ever seen fail is a
   gate nobody can vouch for. §2 is the argument: the gate that runs all the
   other gates was the one nobody was checking, and §3 is the reason the job
   checks the instrument as well.
2. ~~**Get a real browser into the loop.**~~ **Done, and it paid.** §11 — the
   page was a 404 that four green checks sat in front of, because nothing
   produced the bundle and a gate's error message asserted a falsehood about
   what `wasm-bindgen` names its output. The page now builds, loads and renders
   in headless Chrome. §17's remaining limit is narrower: there is still no
   *interaction* test in a real browser.
3. ~~**Add a coverage gate.**~~ **Deliberately not.** §15 explains why: the rule
   it would enforce is already enforced, more strongly, by the mutation script —
   a mutation proves a line is reached *and* that reaching it matters, where a
   percentage proves only the first. `ci.rs`'s remaining 50% cannot be raised by
   a test at all, because those phases shell out to cargo.
4. ~~**Write doc-tests for the format and the command schemas.**~~ **Done.** §14 —
   four doc-tests now pin the tagged-value, externally-tagged-property and
   capitalised-hint shapes, a hand-written file round-trips, the bare-number
   error message the document quotes is asserted verbatim, and every command in
   the registry carries a label, a doc string and typed params.
5. ~~**Get a real display into the loop.**~~ **Done, and it paid more than the
   browser did.** §21 — the window never repainted after a command, so the engine
   said five nodes while the screen showed three, with `ok: true` on the reply
   and nothing anywhere reporting a problem. `scripts/live-window-check.sh` now
   drives the real binary on a real display and compares the window's own
   pixels; it exits 77 rather than 0 when there is no display, because a check
   that reports success where it could not run is the defect it was written to
   catch.
6. **Decide whether `save`/`open` belongs in the command registry.** §18 is a
   design question with a real cost either way, and it should be a decision
   rather than an accident. §33 sharpens what it costs: **nothing in the tree can
   write a design today.** The desktop app opens one and cannot save it, the
   only caller of `craftkit_io::save` is a test, and the sole signal that work is
   being lost is a line on stderr that a GUI launch shows nobody. The write path
   itself is finished and tested — this is a wire-up, not a feature — so the
   decision is about *where the call lives*, not about building anything.
   §47 took the sting out of the *open* half of it: opening a file used to
   leave the previous document's undo stack behind, so the first Ctrl+Z after
   opening a file replaced the file with an empty document. `Session::load_document`
   clears the undo stack, the log and the command trace now, and the crash
   report replays onto the document it was made from. §50 took the sting out
   of the *write* half: the writer now refuses a document the reader would
   refuse, and the engine refuses to leave such a document behind in the first
   place — so wiring `save` up is now safe rather than dangerous. §51 closed
   the size half: the writer will not write a file the reader would refuse,
   and it will not build the text to find out. The decision
   about where `open` and `save` live is still open — it is a decision, not a
   bug.
7. **Decide whether the engine's per-command clone is worth a redesign.**
   §52 measured it: 205 µs for a clone of a 19 MiB document, and about
   1.9 ms of engine overhead per command — the price of being able to put
   the design back when a command fails or panics. Removing the clone means
   copy-on-write — sharing the property tree between the snapshot and the
   live document and cloning only the branch a command writes — which is a
   change to the model's ownership rules and should be a decision rather than
   an accident. §53 is the arc that shows how to find such a cost: measure
   the thing you are optimising against the thing you are paying.
8. **Decide how prose is kept true.** `cargo xtask lists` checks that prose
   *points at* code that exists — a file, a line, a symbol, a test — and that
   is all it checks. §53 changed the evaluator's fingerprint and two documents
   went on describing the old one in detail, correctly, pointing at real
   code: *"It is FNV-1a over the document's serde form"*, and the argument
   for why. Nothing noticed. A gate that checks what prose *says* is a
   heuristic (§29 says so), so the cheap alternative is a rule: a change to a
   mechanism re-reads the documents that describe it. That is a process, not a
   check, and it is the same shape as the decision in item 6 — it should be a
   decision rather than an accident.
   §55 is the second instance, and it is the one that makes the case: §4 had
   already *checked* the token file's privacy — a test, a mode, a sentence
   saying the permissions were set *before* the content — and the sentence was
   wrong. A section that verifies a mechanism and then describes it is still a
   description, and §4's description was written by the same process that
   checked it, from the same reading of the code. Two instances in fifty-three
   sections is not a rate; it is a shape. The cheap rule would have caught
   both: **a change to a mechanism re-reads the documents that describe it.**
   §61 is the third — a field called `written_at` that is always zero, with a
   doc comment saying it is a time — and §62 is the fourth: a counter whose
   doc comment said *"since the process started"* while the code was a
   `thread_local!` cell, three lines away. §64 is the fifth: a doc comment
   saying that a read bounded the *memory*, when the read bounded the *text*
   and the parse was thirty-two times bigger. §65 is the sixth, and it is the
   worst of the six: the evaluator's comment said *"every level comes from
   `apply_gain`"*, three lines above the source node that takes its level from
   somewhere else, and the thing that sentence would have stopped somebody
   looking for was the bug. Six instances, every one of them a sentence written
   by someone reading the code rather than measuring it. The rule had six
   chances to have prevented something by then, and nobody has adopted it.
   §67 is the seventh, and it is the first one a *machine* said out loud: the
   web gate printed "the canvas, the bundle path and the worker's rules all
   agree" while the worker's copy of the bundle path disagreed with the page's
   — and the gate had no way to know, because it only ever looked at one of
   the two copies.
   §69 is the eighth: a doc comment that said *"how many commands failed
   since the session opened"* over a ring of the last 256 commands, so the
   number was frozen at 255 and the sentence was the only thing wrong.
   §71 is the ninth, and it is this file: §1's own "28 of 29 caught" was
   ninety cases out of date, and the counters in these very sentences went
   stale every time a section was added. Nine instances, and the ninth is the
   list of them.
   §73 is the tenth, and it is the worst of them because it was a **budget**:
   four sentences in four files said what the tree *did* rather than what it
   *checked* — the estimate's own "never under-estimates", §52's "the count
   that matters is the keyframes'", the evaluator's "every number the tree
   stores is", and the interpolator's "the keys are numbers the tree checked"
   — and the budget they described was not binding: a document was estimated
   at two hundred and seventeen times less than it was, and the undo stack
   grew to eight hundred and thirty megabytes against a sixty-four-megabyte
   budget. Ten instances, and the tenth is the one where the sentences were
   about the very thing that was broken.
   §74 is the eleventh: a limit in the table of limits that bounded "the
   reverse index" — and there is no reverse index. §75 is the twelfth, and it
   is the one this tree caused itself: §72 changed the crash report's shape
   and the recipe that quotes it — the document a person reads when their
   application crashed — went on quoting the old one, in the same stretch, by
   the same hand, while this very item was being written down for the tenth
   time. The rule was never wrong; it was never adopted.
   §76 is the thirteenth: three sentences promised a crash report "on every
   unclean exit", and the tree has no unclean-exit path at all — the only
   writer is the panic hook.
   §78 is the fourteenth, and it is the second one this tree caused itself:
   the pattern document said "**two boundaries, with distinct jobs, and no
   more**" while the tree had three, and §76's own fix added the fourth. A
   rule with a number in it is a rule that rots, and this one rotted twice
   without anybody re-reading it. Fourteen instances, and the last two are
   the tree contradicting itself in the act of documenting it.
   §80 is the fifteenth, and it is the one where the sentence named the
   guard that did not work: the fingerprint's file said a test would fail "if
   a part of the node stops being part of the hash", and the test it named
   would not — removing the keyframe's easing from the hash left **every test
   in the workspace green**. Fifteen instances, and the fifteenth is a
   sentence about the very check that was supposed to catch it.
   §81 is the sixteenth, and it is three sentences in one document about the
   format change: the recipe's worked example did not compile (`PropPath::parse`
   returns an `Option`, the recipe said a `Result`), the pattern document's
   example used a name that is not a Rust identifier, and the recipe's naming
   rule contradicted the test that enforced it. Sixteen instances, and the
   sixteenth is the only one where **no working change could be written at
   all**.
   §82 is the seventeenth, and it is a list: `AGENTS.md` said "Its methods are
   …" and named five of the eight the code has. Seventeen instances, and the
   seventeenth is the one an agent reads first.
   §83 is the eighteenth, and it is a claim about a parser: the shortcut
   field's doc said "in the form the UI's shortcut parser understands", and
   the tree has no shortcut parser and binds no keys at all. Eighteen
   instances, and the eighteenth is the only one whose sentence promised
   something *elsewhere in the tree* that was never built.
   §99 is the nineteenth, and it is the same shape as the eighteenth: the
   `Automation` struct's doc said *"Rebuilt every frame and dropped after, so
   a widget that disappears does not linger in the registry and let a click
   land on stale coordinates"* — and the shell never cleared it, so a widget
   that disappeared lingered, with the rectangle it had when it was first
   drawn. The sentence was in the place a reader would look, and the code
   said something else.
   §100 is the twentieth, and it is a sentence that named a widget the tree
   does not draw: the name constructor for the "clear keys" button said
   *"the name for the keyframe row of `path`"*, and there is no keyframe row.
   Twenty instances, and the last two are the tree contradicting itself in the
   act of documenting it — a promise about the frame and a promise about a
   widget, both made by the same code that broke them.
   §101 is the twenty-first, and it is a sentence about a *test*: the timing
   test's comment said *"the test is relative, so it does not depend on how
   fast the machine is"*, and it did not depend on the machine's **speed** — it
   depended on the machine's **load**, and failed four times in ten when the
   machine was busy. Twenty-one instances, and the twenty-first is the first
   one that was a promise about the *checking* rather than about the tree.
   §103 is the twenty-second, and it is a sentence about a *cost*: §52 said
   the engine's overhead per command was 1.9 ms and that "that is the price of
   the restore guarantee", and the restore guarantee is 205 µs of it — the
   other three quarters were three more walks over the document, added after
   §52 was written and never measured. Twenty-two instances, and the
   twenty-second is the only one where the number was right and the
   attribution was not.
   §105 is the twenty-third, and it is a sentence about a *contract*: the
   `frame` function's doc said *"The host keeps the `ShellState`"*, and the
   desktop host did not keep it — it dropped the state on the floor every
   frame, and the web host's field said *"Kept between frames by the host"*
   while it was overwritten with the same emptiness once a frame. Twenty-three
   instances, and the twenty-third is the one where the sentence described a
   promise the code was making *to the reader of the sentence* rather than to
   the compiler.
   §106 is the twenty-fourth, and it is a sentence about a *struct*: the
   raster's `Frame` said its state was *"selection and the status line"*, and
   §105 had already added two fields to it — the path and the value of the
   "key at the playhead" row. Twenty-four instances, and the twenty-fourth is
   the one where the sentence was half right, which is harder to notice than
   being half wrong.
   §108 is the twenty-fifth, and it is a sentence about a *whole*: the engine's
   `snapshot_json` said *"The whole state, for an API surface that wants it in
   one object"*, and the object had the document's name and not its frame rate,
   its range or its comment. Twenty-five instances, and the twenty-fifth is
   the one where the word "whole" was the whole problem: the sentence was
   true of every field it had and false of the object it described.
   §110 is the twenty-sixth, and it is a sentence that contradicted the
   sentence below it: the same doc comment said the document's comment was
   *not* in the state — *"nothing bounds its length, and this reply is not
   capped"* — and four lines later said the note was there, whole, with a
   bound. Two paragraphs, written for two different sections, and the one
   between them was not touched. Twenty-six instances, and the twenty-sixth
   is the only one where the tree contradicted itself **inside one doc
   comment**.
   §111 is the twenty-seventh, and it is the *same paragraph*: the sentence
   that listed what the state carries said *the undo depth*, and the state
   never carried it — the depth was in `session.stats` and the memory was in
   nothing at all. It was written in §108, which rewrote that very list, and
   it was written from the statistics rather than from the state.
   Twenty-seven instances, and the twenty-seventh is a list of fields written
   from a different object than the one it describes.
   §112 is the twenty-eighth, and it is a field's doc comment about what
   *else* would use it: the browser app's `saved` flag said it was there *"so
   the panel can say 'saved' rather than making the user wonder whether it
   has ever saved anything"*, and the panel said nothing of the kind — the
   flag was written three times and read never. Twenty-eight instances, and
   the twenty-eighth is the only one where the sentence was about a **use**
   rather than about the thing it described.
   §113 is the twenty-ninth and the thirtieth, and they are the same kind of
   sentence in the same crate. The MCP server's module doc said *"Every tool
   maps onto a [`craftkit_engine`] command"* — and two of the twenty-four
   tools did not. The tools module's comment said *"`session.state` and
   `diagnostics.list` are commands already, so they appear from the
   registry"* — and `session.state` is the control channel's method, not a
   command, and did not appear from the registry. Thirty instances, and the
   twenty-ninth and the thirtieth are the only ones where the sentence was
   about the *shape* of a surface rather than about the surface itself.
   §114 is the thirty-first, the thirty-second and the thirty-third, and they
   are all about a **caller**: the kernel's measurement module said *"The
   engine uses that to decide whether an evaluation is worth caching"*; the
   kernel's `guarded()` said *"The engine uses this to decide whether a
   recovered panic should be surfaced to the user as a warning or can be
   logged quietly"*; and the trust-boundary pattern said *"`recovered_count()`
   and `guarded()` let it tell the two apart without parsing a log"*. The
   engine calls none of them that way, and never did. Thirty-three instances,
   and the thirty-first to the thirty-third are the only ones whose subject is
   a **caller** rather than the thing itself — which is the one kind of claim
   the tree's gates do not check at all.
   §115 is the thirty-fourth, the thirty-fifth and the thirty-sixth, and they
   are all about **a number nobody could see**. §89 said *"The playhead's
   frame number is visible, so the drift is visible too"* — the frame number
   was in the model, in the state and in the arithmetic, and on no screen at
   all: the slider's value was hidden and its formatter, the only thing that
   would have written it down, never ran. The shell's own comment said *"a
   zero here would divide by zero, so the panel does not trust it"* — and the
   panel divided by the frame rate twice, once for the duration and once for
   the playhead's frame, while the rate it *did* trust (`f64::from(meta.fps).
   max(1.0)`) was used by the formatter that never ran. And §108 said *"The
   shell reads `meta.fps` for the header and the timeline's scale"* — the
   timeline had no scale. Thirty-six instances, and the thirty-fourth to the
   thirty-sixth are the only ones whose subject is a thing that was **never
   drawn** — a sentence about a widget, checked by reading the widget's
   neighbours.
   §116 is the thirty-seventh and the thirty-eighth, and they are both about
   a **type**. `Doc::NodeRef` said *"A node id, as a number or as the node's
   name"*, and the reader refuses a name in that parameter (*"`Level` is a
   name, not an id; names are resolved by `node_name`"*); and the catalogue
   marked the id as **required** in thirteen commands whose own refusal says
   *"pass `node` (an id) or `node_name` (a name)"*. Thirty-eight instances,
   and the thirty-seventh and the thirty-eighth are the only ones whose
   subject is the **shape of a value** — a type and a flag that say what a
   caller may send, checked by reading the code that reads them.
   §118 is the thirty-ninth, and it is the first one whose subject is a **piece
   of behaviour** rather than a sentence about one. Four places said that
   interactive work belongs to `ui.start` — in `AGENTS.md`, in the engine's own
   module doc, in `Session::execute`'s comment and in the pattern document —
   and the tree had no `ui.start`, no interactive entry point and no way to
   ask a question. The prose was not describing something that had drifted; it
   was describing something that had **never been built**. Thirty-nine
   instances, and the thirty-ninth is the only one where the sentence pointed
   at a *missing mechanism* rather than at a mechanism that had changed.
   §121 is the fortieth, and it is the **first one this tree caused itself
   in the act of fixing a gap**: §119's *"Why nothing noticed"* said *"The
   timeline shows the range and no keys"* — and §121 put the keys on the rail,
   which made the sentence false. The rule — *a change to a mechanism re-reads
   the documents that describe it* — was not wrong, and the sentence was not
   rewritten, because nothing asked. Forty instances, and the fortieth is the
   one where the sentence described the very thing that was fixed.
   §124 is the forty-first, and the first one a **doc comment on a field**:
   `Property::locked` says *"A locked property is greyed in the UI and refused
   by `property.set`"*, and only the second half is true — `crates/ui` never
   reads `locked`. A comment on a field is read by the next person to change
   the field, which makes it a document; a document that is wrong is worse than
   no document, because it is believed.
   §126 is the forty-second, and it is a sentence about **what the registry
   says it holds**: `Automation::rects` said *"The names say what is on
   screen"*, and the properties panel published four thousand nine hundred and
   ninety-one editors that were not on the screen — laid out below the panel's
   bottom edge and never painted. The sentence is now true of what the
   registry holds, and the comment says so; the deeper question, of whether a
   row that is scrolled out of view should be published at all, is open in
   §126.
   §127 is the forty-third, and it is the first one whose subject is a
   **number**: the doc comment on `node_rows` said the collection took
   *"49.8 ms"* at ten thousand nodes and *"0.9 ms"* once the levels were found
   once — the numbers from the first probe. The final measurement, with the
   code as it was committed, said **43.7 ms** and **1.2 ms**. The two probes
   differed only in the shape of the fix, and the comment was written from the
   earlier one and never re-read. A sentence about behaviour is checked by
   reading the behaviour; a **number** has to be checked by repeating the
   measurement, and nothing in a build repeats it. Forty-three instances, and
   the forty-third is the only one where the fix was measured **twice** and
   only the first measurement was written down.
9. **Decide how long a crash report lives.** §58 left one thing open, and it
   is the only unbounded writer left in the tree: the undo stack has a byte
   budget (§52), the trace has one, the command log has a count, and the
   crash folder has neither. A report of the sample document is 16 309 bytes,
   so an application that panics on launch and is relaunched by a supervisor
   writes a full copy of the design every time, and nothing prunes it. The
   obvious fix is to keep the newest *n* reports and drop the rest, and the
   reason it has not been done is not difficulty: **deleting somebody's
   evidence is a decision, not a default.** What *n* should be, and whether
   the folder should have a byte budget like the undo stack rather than a
   count, are choices with a cost either way. §57 made the folder private and
   §58 made it complete; neither of those is the same as making it finite.
10. **Decide what "private" means for the files this program writes.** §55
    and §57 found the same thing twice: a file this program writes holds
    something its owner did not choose to publish — a launch token, a copy of
    the design — and the permissions were whatever the umask said. A rule
    that appears once is a coincidence; a rule that appears twice is a
    convention, and the convention is now: **what this program writes is
    private, and a directory the program did not create is not the program's
    to change.** The next file this program writes should follow it without
    anyone having to measure the mode again. The one file that does not
    follow it is the design the user saves where they choose — and *that* one
    is deliberately left alone, because a document the user put somewhere is
    the user's business, not the program's.
11. **Decide what may go into a `json!`.** §60 found that `serde_json::json!`
    hides an `unwrap` inside the macro, where the house deny cannot see it,
    and that seven places in the tree put a tick into one — relying on the
    model's tick range being exactly the range a JSON number holds. That is
    true today and is true for a reason nobody wrote down at the site. A
    helper that carried a tick safely would put the rule in one place and
    would make the seven sites independent of the bound. The same question
    applies to every `json!` in the tree that interpolates a number: a `u64`
    above `i64::MAX` fails the same way, and there are more of those than
    there are ticks. The related open question is the `time` parameter's
    range: it accepts 10^9 seconds where a tick carries 72 623.7, so a
    hostile time saturates — which is the documented contract — and the
    message the user gets is about the document not changing, not about the
    time.
12. **Decide what a comparison has to be.** §63 found that the constant-time
    rewrite of the token comparison silently traded away exactness: the
    length difference was folded into a `u8` accumulator, every difference
    that was a multiple of 256 was lost, and a peer that presented the token
    followed by 256 NUL bytes was authenticated by the real control channel.
    The rewrite's own justification — *"the alternative is one `==` that
    someone eventually copies into a path where it matters"* — is a
    comparison that was **right**, so the rewrite was a trade nobody wrote
    down. The cheap rule would be: **a rewrite for a non-functional property
    must keep the functional one**, and the property to test is *"this and
    nothing else"*, because a test that checks one wrong token is refused
    has not checked the space of tokens that are accepted. The same question
    applies to every other comparison in the tree that was made constant-time
    or made tolerant for a reason, and there is no standing rule saying which
    of them are allowed to be inexact.
13. **Decide how a big crash report's base is carried.** §64 brought the
    CLI's script limit down from 32 MiB to 1 MiB, because the parse of a
    script costs up to thirty-two times its size and the old limit bought a
    gigabyte of memory. A script is one file, so it carries two very
    different things — a list of command invocations, which are small, and a
    base document, which can be thirty-two megabytes of memory — under one
    limit, and the two need limits an order of magnitude apart. The current
    answer is that the base is in the script if it fits and the tool says so
    if it does not, pointing at `craftkit-cli crashes replay <id>`, which
    replays it directly. The other answer is that the base travels beside
    the script rather than inside it, and that is a change to what
    `crashes replay --script` prints and what `craftkit-cli run` reads — a
    decision about a tool's output shape, not a bug. §72 took the sting out of
    one half of it: the crash reader now reads a report as a **document** and
    refuses one whose design is not a document, so a hostile report costs its
    text rather than thirty-two times its text. What is left of the question
    is the *shape* of the output, and one number the tree still does not
    derive: the report's own text cap is 64 MiB, and with the reader costing
    about four times its text that is a 256 MiB budget — above the 32 MiB a
    document may hold, and above the 32 MiB every other reader of untrusted
    JSON in the tree allows itself. Whether a crash report deserves its own,
    larger budget is a decision; that the cap should be *derived* from a
    memory budget rather than written by hand is not. §144 left it alone,
    because the value depends on the **shape** of the report — whether the
    base travels inside it — and the shape is the question above.
14. **Decide whether the running app should report its own memory.** §66 found
    a leak that cost 221 MB an hour, and nobody could see it, because egui
    only redraws on demand: a leak in the drawing path is *invisible* in a
    running application — not slow, not subtle, invisible — and the raster
    path that would have shown it is driven by an agent that closes the
    connection when it is done. The gate now measures the process's resident
    pages in a child process, which is honest and narrow: it covers one
    function, on Linux, and it says nothing about the window's own draw loop,
    or about what a long session does to the undo stack, the log or the
    command trace. The cheap alternative is a rule rather than a check — *a
    per-frame path allocates and frees, and a per-frame path that allocates a
    `Box::leak` is a bug* — but the same shape as item 8, it has to be adopted
    rather than merely stated. The bigger question is whether the app should
    *say* how big it has got: a status line with the process's resident size,
    or a warning once it passes a threshold, would turn "invisible" into
    "visible" for every future leak and not just this one. §144 made half of
    it askable — `Session::document_bytes()` is what the *documents* a session
    holds take, and `MAX_SESSION_BYTES` is what they are allowed to take — so
    the remaining half is the process's resident size, which is a fact about
    the operating system rather than about the design.
15. **Decide what a folded format change is.** §81 fixed the half that could
    be fixed — the naming, the guards, the reader — and left the release
    half open, because it is a decision about the release and the release is
    the owner's. The script folds a change into
    `crates/io/src/format/pending/v<N>/`, empties the table and puts `apply`
    back to "the loop with no arms"; **nothing in the tree then reads a
    `v<N>/` folder**, and no folded step is compiled or called. The recipe
    describes the other half: `crates/io/src/format/v2/` and a reader that
    dispatches on the version the file claims. So the choice is between
    *history* — the folded files are a record of what changed, and the reader
    is edited by hand to understand the older shape — and *mechanism* — the
    folded files stay callable, the reader dispatches on the version, and an
    old design keeps loading through its rewrite. The second is what the
    one-file-per-change rule is for; the first is what the script does today.
    Either way the answer belongs in `docs/patterns/format-changes.md`, and
    until it is there, a release silently drops the rewrite.
16. **Decide whether the shortcuts are bound or dropped.** §83 found seven
    chords in the registry — `Ctrl+N`, `Del`, `F2`, `M`, `Ctrl+D`, `Ctrl+Z`,
    `Ctrl+Shift+Z` — and **nothing in the tree binds a key**: no shortcut
    parser, no key handling in the UI, no key press that does anything. The
    chords were advertised to agents through the MCP tool descriptions as
    "Keyboard: Ctrl+Z", which is the tree sending a person to a key that does
    nothing. The documentation now says what is true — a chord is the chord a
    command is *advertised* with, and the tree binds no keys — and the chords
    are checked for uniqueness and shape, so the specification is one somebody
    could implement. What is left is the decision: **bind them, or drop the
    field.** Binding is a product question: which commands get a chord, what
    a chord does about a command that needs parameters (`Ctrl+N` is
    `node.add`, which needs a name and a kind), and what a key press means
    beside the rule that only a toolbar click starts an interactive tool.
17. **Decide the frame rate's ceiling, and whether an inexact rate is allowed.**
    §89 found the ceiling written in five places with two values — the reader
    accepted 1000 fps and the command 240 — and made it one number
    (`craftkit_model::limits::MAX_FRAME_RATE`, now 1000 so that no file which
    opens today stops opening). The *value* is the owner's: 240 is the
    sensible ceiling for a plan, 1000 is the one the reader had. §90 found the
    second half: a rate that does not divide the tick rate (11, 13, 17) has a
    truncated number of ticks per frame, and the error **grows** — 363 ticks
    after a thousand frames, 56 ms after an hour at 11 fps. The tick is exact
    either way, and the promise in the code has been corrected to say so; what
    is open is whether a document may carry a rate whose frame boundaries are
    not exact. If not, the rule is already derived — `FrameRate::is_exact` —
    and the reader and the command can refuse those rates with it.
18. **Decide what a command may cost.** §103 measured what a command costs on
    a document at the model's limits: the engine walks the whole document four
    times — the snapshot, the comparison, the validation and the size estimate
    — and a mutating command costs **23.8 ms**, which is more than a frame at
    sixty frames a second. The snapshot is item 7's question (copy-on-write).
    The comparison and the validation are *safety*: the comparison catches a
    command that lies about what it did (§93, §96) and the validation catches
    one that leaves a document the file reader would refuse (§50), and between
    them they cost 7.2 ms of the 23.8. The estimate is the undo stack's byte
    accounting (§52), and it is what keeps that budget honest. The question is
    not whether to keep them — they are the reason the tree can promise that a
    command cannot leave a broken design — but **what a document may cost to
    edit**. `MAX_DOCUMENT_BYTES` is a *storage* limit: it says a design may be
    32 MiB, and says nothing about what that design costs to drive. A second
    limit, on the cost rather than the size, would be a new number in a new
    place, and the tree has no way to measure the cost of a command without
    building a document at the limit to measure it on — which is what §103
    did, and why the numbers are here and not in a gate.
19. **Decide whether selecting a node belongs in the document.** §105 put the
    selection where the shell could keep it, and §106 shared it between the
    window and the control channel. It is the one piece of the interface that
    is **not** a command and not in the document: it changes nothing that can
    be undone, and nothing that is saved with the design. That is a defensible
    choice — a selection is a *view*, and undoing a selection would be
    strange — but it has a cost: a design reopened tomorrow opens with nothing
    selected, an agent and a person at the same desk can disagree about what
    is selected without either of them being wrong, and the CLI's `snapshot`
    can only ever draw the empty panel (§106's parameter is the control
    channel's way around it). The other choice is to put the selection in the
    document: it would then be saved, undoable, and shared by every surface —
    at the price of the file format, of the model's idea of what a design is,
    and of an undo stack that records "the person looked at something else".
    Neither is obvious, and the choice should not be an accident of which
    struct the field landed in.
20. **Decide what a design's `end` means.** The model says *"One past the last
    instant"* (`crates/model/src/document.rs`, the `end` field) and the
    catalogue says the same of `doc.range`'s `end` — and the code says the
    opposite. The frame range is `start..=end`, the playhead clamps to
    `[start, end]`, and the model's own test calls the end's frame *"the
    design's last frame"*: so a five-second design at 24 fps has frames
    0..=120, which is **121** of them, and the end's frame is one past the
    last instant the code believes in. The format's own specification
    (`docs/design/file-format.md`) reads the exclusive way — the end *"is
    0.5 s at 24 fps — 12 frames"* — so the tree has two readings of one
    field, and they differ by a frame. This is a **decision, not a bug**:
    either the two doc comments and the format spec's wording move (and every
    design that means *"my clip is five seconds long"* keeps 121 frames), or
    the code does — and that is a change to what a saved design means, which
    is what the pending-format scheme exists for (§95). The cost of leaving
    it is a person who follows the documentation setting an end one frame too
    late and never finding out.
21. **Decide what an agent may do with a list that does not fit.** §126 made
    the properties panel scroll, and the registry still publishes every row of
    it — the ones scrolled out of view included, with rectangles outside the
    window. An agent that asks for `node.1.prop.p4999` is told where it is,
    and it cannot get there: `ui.click` reports and does not act (§17), and
    nothing in the control channel scrolls a panel. The choices are: publish
    only the rows on the screen, so the registry says what the screen holds
    (§99's rule, read strictly); publish all of them and add a way to scroll
    the panel, so an agent can walk the list; or publish the scroll offset, so
    an agent can tell that the row is out of view and how far away it is.
    Which one is right depends on what an agent is for, and nobody has written
    that down.
22. **Decide what the plan should cost to ask it a question.** `Plan::state(id)`
    is a scan, and §127 fixed the one caller that asked it about every node in
    the document. The next caller will not know, because the signature reads
    like an indexed read. The obvious fix is a map inside the plan — but the
    plan is a **derived** value, rebuilt on every evaluation, so a map of ten
    thousand entries is paid on every *command* to save a scan on every
    *frame*. The alternatives are a map that is built lazily, a plan that
    keeps its nodes sorted so a binary search answers, or a rule that says a
    plan is only ever asked about a few nodes and every caller must collect
    first. Which one is right depends on how often the plan is rebuilt and
    how often it is asked, and nobody has measured the second against the
    first.
23. **Decide what an agent may read off the screen.** §128 measured that the
    problems panel publishes **nothing**: its rows are drawn and no name
    carries them, so an agent that asks "what does the screen say?" is told
    about the toolbar, the node rows and the property editors, and not about
    the problems. An agent can get the diagnostics from `plan.evaluate` — so
    nothing is lost that cannot be got another way — but the screen and the
    registry disagree about what is on the screen, which is the same drift
    §126 recorded for the properties panel. Making the list observable would
    also make it possible to gate: the problems panel's scroll area shrinks
    to its content, so the wheel does nothing over most of the central panel
    while a list of ten thousand problems sits there, and there is no gate
    for that today. The choice is: publish the rows as a name per diagnostic,
    or leave the panel as the one part of the screen an agent cannot see.
24. **Decide what a node's problems may depend on.** §129 made the cache keep
    the problems of the nodes it reuses. That is only sound while a node's
    problems depend on the node alone, because the fingerprint covers the node
    alone — and today that is true only because `amount_of` happens to read the
    node's own properties. It is written down nowhere and nothing checks it. A
    diagnostic that read the level coming in, or another node's state, would be
    reported once and then reused for every later evaluation, which is §129's
    defect with the fingerprint as the reason. The choices are: put the rule
    next to the fingerprint and gate it, widen the fingerprint to cover what a
    diagnostic may read, or make a diagnostic that depends on more than its own
    node a separate kind of thing that is never cached.
25. **Decide what a front end may do that is not a command.** §135 closed the
    second path *through the engine*: the session's public methods are an
    allow list, and a panel may not put a document into the session. What is
    still open is the other direction — a front end that *reimplements* a
    command's behaviour instead of running it. A shell that drew a rectangle
    itself rather than running `sketch.rectangle` would pass every gate in
    the tree, and §132's gate has the same hole: it checks what the shell
    *names*, not what the shell *does*. The choices are: compare the shell's
    effects against the registry's (hard, because the shell draws and the
    engine does not), name the commands a front end is allowed to reimplement
    (a list that has to be kept true), or accept that a front end's behaviour
    is checked by its own tests and that the gates police only the *names*.
26. **Decide what a design file may cost, and what the reader may spend on one.**
    §141 bound the writer's indentation, so that a file is at most twice the
    design it holds, and derived the file's cap from the model's budget. The
    two numbers are still the tree's own choices: the expansion is the
    format's (1.51 measured, 2 allowed) and the bound on the indentation
    (four levels) is a taste. And the reader still pays for the text it
    refuses — a 61 MiB file costs 119 MiB of resident memory, because the
    document's budget is checked after the parse rather than during it. The
    choices are: flatten the property tree in the file, so that a file's size
    is a property of the data and not of how deep the design nests (a change
    of the file format, with the `format/pending` machinery and the
    compatibility work that goes with it); or keep the nesting and stream the
    reader's parse, so that a hostile file costs a document and not a document
    plus its text; or accept both numbers as they are and write them down as
    the promise they are.
27. **Decide how large a reply may be.** The tree bounds every input it
    reads — the control channel's line, the MCP server's message, the CLI's
    script, the design file — and bounds no output it writes. Measured: a
    one-line request for `engine.state` on a design of ten thousand nodes
    answers with **4.07 MiB**, and `plan.evaluate` with **3.50 MiB**. The
    crash folder was the sharp end of that (§142 fixed the writer there, and
    the report of a design the model accepts now reads back), but the control
    channel's replies are still of whatever size the design makes them. The
    choices are: bound the reply like the request and refuse the big ones,
    which is the honest thing but leaves an agent unable to ask a question
    about a big design; page the reply, which changes the protocol; or accept
    the size and say what it costs, which is item 21's question about a list
    that does not fit, asked of the whole reply rather than of one list.
28. **Teach the exact-block gate about enums.** The gate that compares a
    quoted block with the code it names reads **functions**, **structs** and
    **statics** — §140 added the last — and nothing else. An enum is a fourth
    shape, and the control channel's whole vocabulary is one: `Method` is an
    enum, and §143's defect was a variant of it. A block marked `exact` that
    quotes an enum is reported as *"quotes no code"*, so nothing may quote
    one. The work is small — the variant names, in order, the way a struct's
    fields are compared — and §140's lesson applies directly: *"The gate that
    compares code has to know every shape of code."*
