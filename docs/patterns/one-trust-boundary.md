# One trust boundary

## The problem

craftkit's kernel is the seam where someone else's arithmetic will eventually
run: today it is a crate in this workspace, tomorrow it may be a WASM module, a
C library, or somebody's careful port of something. Code you do not own can
panic. Code you *do* own can also panic — but your own bugs are better handled
by a test than by a boundary.

The temptation is to wrap suspicious calls as they appear. The result is a
`catch_unwind` in a command, one in a panel, one in the socket handler, each
with its own idea of what to restore afterwards, nested in combinations nobody
tested.

## The rule

**A boundary per job, and no more of them than there are jobs.**

| Boundary | Catches | Restores | Answers with |
|---|---|---|---|
| `craftkit_kernel::guard`, around the arithmetic the evaluator calls per frame | panics in code craftkit does not own | nothing — kernel calls take immutable input and return a new value | `KernelError::Internal` |
| `Session::execute` | everything else, including a panic in our own future command | the pre-command document snapshot | `EngineError::Internal` |
| the control channel's `ui.*` handler (`crates/control/src/server.rs`) | a panic in the host's window handler, which runs with the session locked | nothing — the document is not touched, and catching here keeps the lock from being poisoned | a `handler-panicked` error, and the connection stays good |
| the render path (`apps/craftkit/src/main.rs`) | a panic in the drawing code | nothing — the document is not touched | a crash report, written from the only place that can still see the session |

The last two are here because of a **lock**, and neither of them is a general
safety net:

- the control channel answers `ui.*` by calling *host* code with the session
  locked. Without a boundary a panic in it would drop the connection with no
  reply, and an agent would be left guessing whether its request had arrived.
  Catching there — rather than in `answer` — means the session guard is never
  dropped during unwinding, so the lock is not poisoned.
- the render path holds the session's lock for the length of a frame, so the
  panic hook cannot write a report for a panic in a widget: the hook runs on
  the panicking thread, and `try_lock` fails. The render path catches it
  instead, and writes the report while the session is in hand (§76).

The kernel rule is precise: **if a call can panic and its code is not in this
workspace, it goes through `guard`.** No exceptions, no "this one is obviously
fine". The guarded sites today are `apply_gain` — the one multiply the
evaluator performs per node, per frame — `measure_run`, which goes through the
same `guard`, and `evaluate_chain`, which walks a whole chain: it validates
its input up front and returns an ordinary `KernelError::Failed` for anything
that leaves the finite world, and the guard is around the walk itself, so
that a future foreign implementation is swapped *inside* the boundary rather
than one frame out from it.

`cargo xtask lists` holds the tree to this table: every file with a
`catch_unwind` in it must be a declared boundary, and every declared boundary
must be named here. A new boundary has to be written down before it is
written (§78).

```rust
// crates/kernel/src/ops.rs
pub fn apply_gain(level: f64, gain: f64) -> Result<f64, KernelError> {
    guard("applying a gain", || {
        if !level.is_finite() || !gain.is_finite() { return Err(/* … */); }
        let v = level * gain;
        if v.is_nan() { return Err(/* … */); }
        Ok(v.clamp(-craftkit_core::MAX_MAGNITUDE, craftkit_core::MAX_MAGNITUDE))
    })
}
```

## Why the kernel boundary does not roll back

```rust
// crates/kernel/src/lib.rs
/// `AssertUnwindSafe` is a deliberate, reviewed decision: the closure may touch
/// shared state, and the safety of continuing after a caught panic rests on the
/// caller's discipline — which is why the rule is "the closure does not mutate
/// anything the caller will read afterwards". Every call site in this crate
/// obeys it, because they all build a new value from immutable input.
pub fn guard<T>(what: &'static str, f: impl FnOnce() -> Result<T, KernelError>) -> Result<T, KernelError> {
    let _depth = Depth::enter();
    match catch_unwind(AssertUnwindSafe(f)) { /* … */ }
}
```

`AssertUnwindSafe` is only defensible because the discipline exists. The kernel
operations take a `ChainInput`, validate it up front (`ChainInput::validated`,
`MAX_STEPS`), and return a new `Vec<ChainStep>`-derived value; nothing the
caller reads afterwards was mutated. A caller that mutates in place under
`guard` has a bug the boundary cannot see, which is why the guard is placed
around the *operation*, not around arbitrary code.

The `what` argument is `&'static str` so the message names the call site without
allocating in the failure path: a report reads *"evaluating the chain: the
implementation panicked …"*, not *"error"*.

`KernelError::Internal` is a distinct variant from `Refused` and `Failed`
because it means "a bug, not your input", and the engine reports the two
differently. **The variant is what tells them apart** — the engine reads the
error it is holding, and does not parse a log to find out what happened.
`recovered_count()` and `guarded()` are diagnostics, not the decision: the
first is what `session.stats` reports as `kernel_recoveries`, and the second
answers a question about one thread's stack that **nothing outside the kernel
asks** (§114).

The two counters have different scopes on purpose, and swapping them would
break the engine in opposite ways. `guarded()` is **per-thread**: it answers
"is this thread inside a guarded call?", a question about one thread's stack.
`recovered_count()` is **process-wide**: it answers "has the kernel recovered
from a panic anywhere?", which is the question `session.stats` reports — and
that question is asked from the control channel's thread, while the document is
evaluated on the UI thread. A per-thread recovery count would therefore always
have read zero in the desktop app, no matter how many panics the kernel
recovered. §62.

## Why `Session::execute` is the only other one

```rust
// crates/engine/src/session.rs — "One catch_unwind, in one place"
let result = catch_unwind(AssertUnwindSafe(|| (spec.run)(self, &p)));

match result {
    Ok(Ok(outcome)) => { /* … */ }
    Ok(Err(e)) => { self.restore(snapshot, before_dirty); Err(e) }
    Err(payload) => {
        self.restore(snapshot, before_dirty);
        log::error!("command `{id}` panicked and was recovered: {detail}");
        Err(EngineError::Internal { id: id.to_string(), detail })
    }
}
```

The restore both branches call is one function, so the two cannot drift apart:

```rust exact
// crates/engine/src/session.rs
fn restore(&mut self, doc: craftkit_model::Document, dirty: bool) {
    self.doc = doc;
    self.evaluator.invalidate();
    self.recompute();
    self.dirty = dirty;
}
```

Scattering boundaries is worse than having none:

- Each site needs its own restore logic, and the site that forgets one is the
  site that corrupts state.
- They nest, so a panic recovered by an inner boundary is invisible to the outer
  one and its restore never runs.
- Coverage becomes unanswerable. With one boundary, "is every failure path
  restored?" is a question about one function, and the restore is a single
  unconditional assignment on every branch.

This boundary also catches the failure nobody predicted — the bug in the command
written next month — which is exactly why it belongs above the registry rather
than inside each command.

## When NOT to use it

- **Around code you own, in a place with no restore to perform.** A pure helper
  that panics in a test is a failing test, not an incident. If a `catch_unwind`
  appears in production code outside the boundaries above, it is a bug in the
  design, not a safety net.
- **To turn an error into a `Result` and carry on as if nothing happened.** A
  recovered kernel panic is `Internal`, it is logged, and it becomes a crash
  record — because it is the thing worth reproducing, not noise.
- **To guard a whole subsystem "while we are here".** Guard the operation, at
  the kernel entry, where the closure's no-mutation discipline is checkable.
- **When the input is the problem.** A hostile parameter is rejected by
  [`never-index-a-parameter.md`](never-index-a-parameter.md), long before it
  reaches a boundary. A boundary is not an input validator.

## See also

- [`clone-before-edit.md`](clone-before-edit.md) — the snapshot that makes the
  engine boundary's restore a single assignment.
- [`../architecture.md`](../architecture.md#the-panic-boundary-two-with-different-jobs)
