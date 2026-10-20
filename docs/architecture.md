# craftkit architecture

## What craftkit is

craftkit is a Rust scaffold for applications whose central artefact is a
**parametric plan**: a `.craftkit` document holding an ordered list of nodes —
`source`, `filter`, `sink`, `cue`, `comment` (`craftkit_model::NodeKind`) — each
with its own property tree (`craftkit_model::PropGroup`), keyframed against an
exact `Tick` timebase, and evaluated to a `Plan` at any instant. The problem it
models is the one every audio tool, video tool and effects chain has: a document
that must stay editable after it is evaluated, must re-evaluate cheaply as the
playhead moves, must never lose a person's work to a crash, and must be
driveable by an agent through the same API a person uses. craftkit is a scaffold
— the crate boundaries, the contracts and the tests are the deliverable; the
domain terms (level, gain, chain) are placeholders you replace.

## Layers

A crate may depend only on crates in **strictly** lower layers. "Same or lower"
would be the wrong rule: an equal-layer dependency is how two crates quietly
become one crate wearing two names. `cargo xtask layers` checks every crate's
`Cargo.toml`, through `cargo metadata`, against the table in
`xtask/src/layers.rs`; `cargo xtask lists` then checks that this table and
`xtask/src/layers.rs` agree, so the documentation cannot drift away from what is
enforced.

| Layer | Crate | Depends on |
|---|---|---|
| L0 | `craftkit-core` | nothing (no workspace deps) |
| L1 | `craftkit-model` | core |
| L2 | `craftkit-kernel` | core |
| L3 | `craftkit-eval` | core, model, kernel |
| L3 | `craftkit-io` | core, model |
| L4 | `craftkit-engine` | core, model, eval, kernel |
| L5 | `craftkit-control` | core, engine |
| L5 | `craftkit-mcp` | core, model, engine |
| L6 | `craftkit-ui` | core, model, eval, engine |
| exempt | `apps/craftkit`, `apps/craftkit-cli`, `apps/craftkit-web`, `xtask` | all of the above |

The third column lists workspace dependencies only; `cargo xtask lists` compares
it against `cargo metadata`, so adding a dependency without updating this row is
a failing check rather than a stale document.

`craftkit-ui` also depends on `egui`, and it is the only library crate that is
allowed to: no UI toolkit may appear below L6. The three applications and
`xtask` are `Class::Exempt` in `xtask/src/layers.rs` — they legitimately depend
on everything, and a rule about them would only be noise.

A crate with **no row at all** is not exempt. It is reported as
`Violation::Unregistered`, because a crate with no layer has no constraints: it
could reach `craftkit-ui` and `egui` from anywhere, which would put a window
dependency inside L0.

Two rows share L3 because neither is above the other: `eval` reads the model and
calls the kernel, `io` reads the model and knows nothing about evaluation. An
exempt crate — an application or `xtask` — may use anything; a layered crate may
not name an exempt one, which is reported as an upward dependency.

### The rule, and why it exists

**Nothing below L6 names a UI toolkit.** The list is `UI_CRATES` in
`xtask/src/layers.rs`: `egui*`, `eframe`, `egui-wgpu`, `wgpu`, `winit`, `rfd`,
`accesskit*`. Entries ending in `*` are prefixes, so `egui` and `egui_extras`
are both caught. `image` is deliberately *not* on the list: it is a PNG encoder,
not a toolkit, and `craftkit-ui` uses it.

The UI crate is swappable. That is not an aesthetic claim — it is the reason
`craftkit-model` can be tested in a plain `cargo test`, the reason
`craftkit-io` can round-trip a file with no window and no GPU, and the reason
`craftkit-engine` can be driven from a socket by an agent that will never open
the app. A layer that cannot be reused without a UI is a layer that will grow a
dependency it should not have, and the moment it does, the headless front ends
stop working. The cost of the rule is a few explicit conversions at the boundary
(`Session::snapshot_json`, `plan.evaluate`'s JSON outcome); the alternative is a
design tool that only exists as a desktop application.

## Data flow

One line per hop, in the order a command travels:

1. **JSON parameters** arrive as `serde_json::Value` from the UI, the CLI, the
   control channel or the MCP server.
2. **`craftkit_core::Params`** wraps them. Every read is a method that returns
   `Option` or `Result`; unknown keys are refused by `Params::deny_unknown`
   against the command's declared names.
3. **`Session::execute`** looks the id up in the registry, checks `enabled`,
   takes a snapshot, and is the single place wrapped in `catch_unwind`.
4. **`CommandSpec::run`** mutates `craftkit_model::Document` through
   `Session::doc_mut`. It cannot open a dialog and cannot touch the clipboard.
5. **`craftkit_eval::Evaluator::evaluate(doc, tick)`** returns a `Plan`, reusing
   the unchanged prefix.
6. **The `Plan`** reaches the UI as state and the caller as `Outcome` (message
   plus JSON `data`).

Nothing else mutates the document. The UI toolbar, a CLI script, the control
channel and the MCP tool all land on step 3, so a guard added to a command
cannot be bypassed by picking a different front end. A keyboard shortcut would
land there too — the registry advertises chords — but the tree binds none yet,
so a chord is a specification and not a binding (§83).

## The panic boundary: three, with different jobs

craftkit has exactly three `catch_unwind` sites, and they are not interchangeable.

| Boundary | Catches | Restores | Answers |
|---|---|---|---|
| `craftkit_kernel::guard` | panics in code craftkit does not own | nothing (calls take immutable input and return a new value) | `KernelError::Internal` |
| `Session::execute` | everything else, including a panic in a future command of ours | the pre-command document snapshot | `EngineError::Internal` |
| `dispatch_with`, around the `ui.*` handler | a panic in the *host's* window handler, which is code this crate does not own and cannot test | nothing — the handler has not touched the document | `handler-panicked`, and the connection stays good |

**`guard` sits around the risky operation, not around the program.** The kernel
is the boundary where a foreign or future implementation would live, so the
guard is placed at the arithmetic the evaluator performs — `apply_gain`, once per
node per frame — and at `measure_run`, which routes through the same `guard`.
There the closure "does not mutate anything the caller will read afterwards".
That discipline is what makes `AssertUnwindSafe` a reviewed decision rather than
a shrug, and it is why `guard` does not need to roll anything back.
`KernelError::Internal` is a distinct variant precisely because "a bug, not your
input" must be reported differently from an ordinary refusal.

**`Session::execute` is the one boundary in one place.** It catches our own
future bugs, restores the snapshot, invalidates the evaluator cache,
re-evaluates, restores the dirty flag, logs, and returns an error. Scattering
`catch_unwind` across call sites is worse than useless: each site needs its own
restore logic, they nest, and the one that is missing is the one that bites. One
boundary in one place means the restore path is a single unconditional
assignment on every failure branch, and "is this path covered?" is a question
about one function.

**The control channel's third boundary guards the code it does not own.**
`craftkit-control` is layer 5 and `craftkit-ui` is layer 6, so the channel
defines the extension point (`ServerOptions::with_ui`) and the *application*
fills it in. A panic inside that callback would otherwise drop the connection
with no reply at all, and an agent would be left guessing whether its request
had arrived. It is caught in `dispatch_with` rather than in `answer` so the
session guard is never dropped during unwinding and the lock is not poisoned:
the client gets a named failure and the next request still works.

`craftkit_kernel::guarded()` and `recovered_count()` let the engine tell a
recovered kernel panic from an ordinary error without parsing a log. The two
have deliberately different scopes, and the difference is the whole point:
`guarded()` asks "is *this* thread inside a guarded call?", which is a question
about one thread's stack, while `recovered_count()` asks "has the kernel
recovered from a panic anywhere in this process?", which is a question about
the whole process. The statistic is read from the control channel's thread
while the document is evaluated on the UI thread, so a per-thread count would
have been structurally incapable of reporting the thing it exists to report.
§62.

## Undo: clone before edit

`craftkit_engine::History` stores whole `Document` clones, not inverse commands.
The reason is stated in `crates/engine/src/history.rs`: an inverse that is not
*exactly* the inverse is how an undo stack starts losing work, and the bug only
surfaces after a crash.

```rust
// crates/engine/src/session.rs — one unconditional snapshot per command.
let snapshot = self.doc.clone();
let before_dirty = self.dirty;
// …
match catch_unwind(AssertUnwindSafe(|| (spec.run)(self, &p))) {
    Ok(Ok(outcome)) => { /* … */ if spec.undoable { self.history.push(snapshot, spec.label); } }
    Ok(Err(e)) => { self.restore(snapshot, before_dirty); /* … */ }
    Err(payload) => { self.restore(snapshot, before_dirty); /* … */ }
}
```

The restore is one function, `Session::restore`, and both failure paths call
it — so the two cannot drift apart. The pattern that says so, and the
measurement behind it, is in [`patterns/clone-before-edit.md`](patterns/clone-before-edit.md).

The same snapshot does three jobs: it is the undo entry, it is the error
rollback, and it is the "did this command actually change anything?" check.

That check used to turn *any* command declaring `mutates: true` and writing
nothing into an `EngineError::Internal`. The comparison cannot tell "the
command forgot to write" from "the command wrote the value a field already
held" — and the second is ordinary. Measured: eight of the nine commands that
could be asked to do what they had already done were refused as *"failed
unexpectedly"*. §93.

So the **command** says now: `Outcome::changed` is true unless the command
says otherwise, and the check fires only when the command claims it wrote and
the document says otherwise. A command that may legitimately change nothing
takes the answer from the model's write — `Node::set_muted`,
`Property::set_value`, `Document::set_range` and their siblings return
whether they changed anything — and passes it on with
`Outcome::unchanged_if(!changed)`. A no-op is then a success: no undo entry,
no dirty flag, and the document left as it was.

`history.undo` and `history.redo` are the two commands that change the
document and are **not** undoable: they change it *by* restoring a snapshot,
and pushing that as a new step would push an entry pointing at the state just
left behind.

A command that does **not** change the document cannot be undoable at all: the
push lives inside the `mutates` branch, so there is nothing to push. Four
commands — `playback.seek`, `plan.evaluate`, `diagnostics.list` and
`session.stats` — declared `undoable: true` and never could, and the flag is
published to agents in the command schema. They are `false` now, and
`no_command_that_does_not_mutate_is_undoable` keeps them that way. §94.

And the other side of the flag is a rule too. A command that declares that it
does **not** change the document and changes it is refused and rolled back: the
write would not be validated, would not be undoable, and would not mark the
project dirty — so the design would change and the app would not know that it
had, and closing the file would lose the change without anything having said
the file was unsaved. The flag is published to agents as *"safe to use as a
probe"*, so it has to mean it. §96.

The cost is bounded and measured rather than guessed: `History::MAX_ENTRIES` is
128 entries, and each entry is a document of at most
`craftkit_model::limits::MAX_NODES` (10 000) nodes. If that ever shows up in a
profile, the fix is to raise the entry count and add a size cap — not to start
inverting commands by hand.

## Incremental evaluation

`craftkit_eval::Evaluator` holds the previous evaluation as a `Vec<CacheEntry>`
— one per node, each with the node's fingerprint and whether it is animated.

The reuse rule, from `Evaluator::evaluate`'s doc comment: walk the node list
from the front; entry `i` is reused when **both** (1) the fingerprint of
`doc.nodes[i]` equals the one recorded with its cached state, and (2) either the
instant is unchanged, or that node has no animated properties. The first entry
that fails stops the walk.

Stopping at the first mismatch is sound because evaluation is sequential: the
level entering node `i+1` comes from node `i`, so a change anywhere in the
prefix can change every later state. Reusing a later entry after skipping an
earlier one would be unsound; evaluating later entries afresh is merely
redundant. For the edits people actually make — change one value, scrub the
playhead — stopping is also the faster option, and it is cheaper to reason
about than a sound-but-fiddly per-node cache.

The fingerprint is `Node::fingerprint`, in `craftkit-model`, and it is a
**structural** hash: it walks the node's fields rather than its `serde_json`
form. It used to hash the serialisation, because a hand-written hash has to be
updated by hand every time the model grows a field and a forgotten update is a
**silent wrong answer** — while hashing the serialisation cannot drift. That
was the right trade until the profile showed up: the serialised form of a node
with 200 000 keyframes is 12 MiB of JSON, hashing it cost 52 ms where the
evaluation it was meant to save cost 7 µs, and a fully reused evaluation cost
exactly as much as a full one. The structural hash is 5 ms, and the risk it
carries is held down by two things: a test next to it that changes each part of
a node **on its own**, and a gate that reads the tree — every field of every
type the hash walks is checked against the hash's own source, so a field the
model grows and the hash forgets is a gate failure rather than a silent wrong
answer. `KNOWN-GAPS.md` §53 has the measurements, and §80 has the story of the
guard that used to be a list of the same fields and drifted with them.

`Evaluator::invalidate` drops the cache after an undo, a redo or a load. That is
belt and braces: the cache is content-addressed, so a restored document cannot
be served a state belonging to a node it no longer has. `KNOWN-GAPS.md` §40 has
the measurement and the test.

## The never-crash contract

Nothing in a production path panics. This is enforced, not requested:

| Mechanism | Where |
|---|---|
| the six-lint deny (below) | every crate root: the nine `crates/*/src/lib.rs`, the three `apps/*/src/main.rs`, and `xtask/src/lib.rs` — thirteen in all |
| `#![forbid(unsafe_code)]` | the same thirteen roots, plus `[workspace.lints.rust] unsafe_code = "forbid"` so it is also true of anything added later |
| `clippy.toml` `allow-unwrap-in-tests`, `allow-expect-in-tests`, `allow-panic-in-tests`, `allow-indexing-slicing-in-tests` | tests may panic; nothing else may |
| Checked readers | `Params::num` / `num_in` / `int` / `text` / `color` / `num_list`, and `Value`'s checked constructors — hostile input is rejected at the read |
| Finite and range checks | `craftkit_core::finite`, `in_range`, `Tick`'s saturating arithmetic, `FrameRate::new` refusing 0 and > 1000 |
| Hard limits | `craftkit_model::limits`: `MAX_NODES` 10 000, `MAX_DEPTH` 32, `MAX_PROPERTIES` 50 000, `MAX_KEYS` 10 000, `MAX_PATH_SEGMENTS` 16, `MAX_NAME` 255, `MAX_DOCUMENT_BYTES` `32 * 1024 * 1024` — with a test asserting a "smaller" limit is never the larger one, and a gate asserting every one of them is enforced in code (§74) |
| Bounded file reads | `craftkit_io::format::File::from_text` refuses input over 64 MiB before parsing, then `Document::validate`, then `reseat_id_counter` |
| Bounded kernel walks | `craftkit_kernel::MAX_STEPS` (64), refused up front rather than detected by a counter deep in a loop |
| The last resort | `Session::execute`'s `catch_unwind`, described above |
| The gate | `cargo xtask ci` — fmt, assets, lists, layers, web, clippy, tests, wasm |

`MAX_DEPTH` bounds the **node graph**; the **property tree** is bounded by
`MAX_PATH_SEGMENTS`, because a property is named by a path and a path is the
shorter limit. A property tree deeper than a path can name holds properties
nothing can reach — the file keeps them, the listing drops them, the evaluator
never reads them — so such a tree is refused rather than quietly truncated.
`KNOWN-GAPS.md` §85 has the measurement.

### The deny attribute, exactly

Copy this into a new crate root. It is written across several lines because the
one-line form is longer than `rustfmt.toml`'s `max_width = 110`, so `cargo fmt`
leaves it like this:

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

`cargo xtask lists` normalises each crate root before comparing it with the
block above: it strips whitespace and the `clippy::` prefix, so the one-line and
multi-line forms are both accepted. **Order still matters.** The six lints must
appear in the order shown; a different set *or the same set in a different
order* is reported, because a reordered deny block is a rule nobody recognises at
a glance. A root with no deny at all is reported as such, with the block to paste
in the message.

Errors are values: `CoreError`, `KernelError`, `EvalError`, `EngineError`,
`FormatError`, `SaveError` are all `thiserror` enums that carry the thing that
failed and the number that was wrong. An unfinished feature returns
`Err(...)`, not a `todo!()`.

Two habits follow from the contract and are worth keeping:

- **Diagnostics, not early return, inside evaluation.** A node whose `amount` is
  the wrong type produces a `Diagnostic` and the plan carries on. One broken
  node must not hide the eleven behind it, and it must not stop a render. Only a
  failure that makes the *whole* evaluation impossible — a document over
  `MAX_NODES`, a kernel panic — is an `Err`.
- **Hostile input has one owner.** Validation lives in `Params` and `Value`, not
  in every command. `craftkit-eval` re-checks nothing on purpose: re-checking is
  not defence in depth, it is a second, drifting copy of the rules. A command
  that finds a bad value anyway reports it; it does not invent a second rule.

## Where things live

| Concern | File |
|---|---|
| Exact time, values, ids, parameter readers | `crates/core/src/{tick,value,ids,params}.rs` |
| Document, property tree, keyframes, paths, limits | `crates/model/src/{document,property,keyframe,path}.rs` |
| The guarded domain operation | `crates/kernel/src/{lib,ops,measure}.rs` |
| `(document, instant)` → `Plan`, prefix reuse, diagnostics | `crates/eval/src/{evaluate,plan}.rs` |
| Commands, session, undo, crash reports, log | `crates/engine/src/{command,commands,session,history,crash,log}.rs` |
| `.craftkit` read/write and pending format changes | `crates/io/src/{format,store}.rs`, `crates/io/src/format/pending.rs` |
| Control channel: protocol, socket, token | `crates/control/src/{lib,protocol,server,token}.rs` |
| MCP tools over stdio | `crates/mcp/src/{lib,rpc,server,tools}.rs` |
| The shell, its widgets, its icons and its software rasteriser | `crates/ui/src/{lib,shell,theme,icons,automation,raster}.rs` |
| The browser build | `apps/craftkit-web/src/{main,stash}.rs`, `apps/craftkit-web/index.html`, `apps/craftkit-web/sw.js` |
| The build tool every gate above lives in | `xtask/src/{lib,ci,layers,lists,assets,web,wasm,doctor}.rs` |

## See also

- [`docs/README.md`](README.md) — index.
- [`patterns/`](patterns/) — the recurring rules, one file each.
- [`recipes/`](recipes/) — worked end-to-end tasks.
- `design/` — **empty.** It is the intended home of the `.craftkit` format
  description and the plan model, and nothing has been written into it yet. The
  format is documented today by `crates/io/src/format.rs`'s module docs and by
  [patterns/format-changes.md](patterns/format-changes.md); do not expect a
  document here.
