# Clone before edit

## The problem

Undo is usually implemented by recording the inverse of what was just done:
`move_node(3 → 1)` is undone by `move_node(1 → 3)`, `set_value(0.5)` by
`set_value(<the old value>)`. It is cheaper than storing state, and it is how
most editors do it.

It is also the thing that goes wrong. An inverse that is not *exactly* the
inverse loses work in a way that only shows up after a crash — the worst possible
moment to find it. The states that drift are the boring ones: a command that
touched two nodes and only recorded one, a "delete" that also has to restore the
selection, a normalization step that ran during the move but not during the undo.
None of them are wrong on their own. Together they are an undo stack that
quietly corrupts a document.

The same mechanism is wanted for two more jobs, which a hand-written inverse
cannot do at all: rolling back a command that failed halfway, and restoring the
document when a command panicked.

## The rule

**Take a clone of the document before a command runs. Restore the whole document
on any failure. Push the same clone as the undo entry on success.**

Not "undo by inverse", not "rollback by hand". One clone, three uses:

| Use | Where |
|---|---|
| undo / redo entries | `History::push(snapshot, spec.label)`, `pop_undo(current)` |
| error rollback | `Session::restore(snapshot, before_dirty)` |
| panic rollback | the same call on the `Err(payload)` branch |
| "did this change anything?" | `if self.doc == snapshot` — a command declaring `mutates: true` that writes nothing is an `EngineError::Internal` |

## In the code

```rust
// crates/engine/src/session.rs
// Clone before edit. Taken for *every* command, mutating or not, so the
// restore path below is a single unconditional assignment.
let snapshot = self.doc.clone();
let before_dirty = self.dirty;
// …
let result = catch_unwind(AssertUnwindSafe(|| (spec.run)(self, &p)));

match result {
    Ok(Ok(outcome)) => {
        if spec.mutates {
            if self.doc == snapshot {
                self.dirty = before_dirty;
                let err = EngineError::Internal { /* … */ };
                self.log.record(id, false, &err.to_string());
                return Err(err);
            }
            if spec.undoable {
                self.history.push(snapshot, spec.label);
            }
            self.dirty = true;
            self.recompute();
        }
        Ok(outcome)
    }
    Ok(Err(e)) => {
        // The command refused. Put the document back exactly as it was:
        // a half-applied edit is worse than a failed one.
        self.restore(snapshot, before_dirty);
        self.log.record(id, false, &e.to_string());
        Err(e)
    }
    Err(payload) => { /* the same restore, plus a log entry */ }
}
```

The rollback itself is one function, so the two branches cannot drift apart:

```rust exact
// crates/engine/src/session.rs
fn restore(&mut self, doc: craftkit_model::Document, dirty: bool) {
    self.doc = doc;
    self.evaluator.invalidate();
    self.clamp_playhead_into_range();
    self.recompute();
    self.dirty = dirty;
}
```

Four details that are the whole pattern:

- **The snapshot is taken for every command, including read-only ones.** One
  unconditional line beats a flag a command has to remember to set.
- **`before_dirty` is captured alongside it.** A failed command must not leave
  the project dirty, and a rollback must not leave a save prompt for changes
  that were rolled back. Today no command sets the flag — the engine sets it
  only after a command succeeds — so this is a precaution against the next one
  that does, and `a_restore_puts_the_dirty_flag_back` is what keeps it from
  quietly becoming a comment.
- **The playhead follows the document in.** The playhead is a place *in* the
  design, and a restore puts a different design in place — so the playhead is
  brought back inside the new design's range. §115: without this line, opening
  a design that starts one second in left the playhead at zero, before the
  design's first frame, and the timeline drew a handle at a place the playhead
  was not. The line is here rather than at each call site because `restore` is
  the one place the document changes under the session.
- **`evaluator.invalidate()` accompanies every restore.** The cache belongs to the
  document that produced it, and a restore puts a different document in place.
  This one is belt and braces rather than a correctness requirement, and the
  tree measures that: with the `invalidate()` call in the engine removed,
  every test in the workspace still passes. The reason is that the cache is
  *content-addressed* —
  each entry holds the hash of the node it was computed from, and an entry is
  reused only when that hash still matches — so a restored document cannot be
  served a state that belongs to a node it no longer has. The call is kept
  because it is cheap and because clearing is cheaper than relying on a
  property; the property itself is tested, in
  `a_restore_cannot_serve_a_state_from_the_document_it_replaced`, and
  `KNOWN-GAPS.md` §40 has the measurement and the test.

**The other two statements, and why they are checked.** `self.recompute()` and
`self.dirty = dirty;` were, until §43, pinned by no test at all: deleting
either left the whole workspace green. The first is the one that matters most
— the plan is what the window draws — and it is now pinned by
`an_undo_re_evaluates_the_plan_and_not_only_the_document`. The second is a
precaution, and it is pinned by `a_restore_puts_the_dirty_flag_back`.

## The cost bound

A snapshot cannot be subtly wrong, and it is not free, so it is bounded rather
than reasoned about:

| Bound | Value | Where |
|---|---|---|
| Undo entries kept | 128 | `craftkit_engine::history::MAX_ENTRIES` |
| Undo and redo memory | two documents' worth | `craftkit_engine::history::MAX_HISTORY_BYTES` |
| Nodes per document | 10 000 | `craftkit_model::limits::MAX_NODES` |
| Clones per command | 1 | unconditional, above |

A session can run for days, so the history is capped in **steps**; a document
is capped in **bytes**, so each entry is bounded; and the stacks have a budget
of their own — `craftkit_engine::history::MAX_HISTORY_BYTES` is
`craftkit_model::limits::MAX_DOCUMENT_BYTES * HISTORY_EXPANSION`, which is
64 MiB, two documents' worth of memory.

**The budget binds before the count does**, for anything but a small document:
half a megabyte of design fills 64 MiB with a hundred and twenty-eight
snapshots, and a design at the model's limit keeps **two** of them. If it ever
appears in a profile, the fix is to raise
`craftkit_engine::history::HISTORY_EXPANSION` — not to raise `MAX_ENTRIES`,
which is not what binds, and not to start inverting commands by hand. That
sentence is the point of the pattern: it names the *next* thing a maintainer
will be tempted to do, and says it is the wrong one.

## When NOT to use it

- **For a state too large to clone per command** — a multi-gigabyte media cache.
  That belongs outside the document: reference it by id and keep the history over
  the document alone.
- **Inside a command body.** A command mutates the session's document in place;
  it does not take its own snapshot. One boundary takes one clone.
- **For undo granularity finer than "one command".** Coalescing a drag into one
  undo step is done by choosing when to call `execute`, not by snapshotting
  inside a command.
- **As a reason to skip the dirty flag.** The clone is about restoring state;
  `before_dirty` is a separate fact that has to be captured too.

## See also

- [`one-trust-boundary.md`](one-trust-boundary.md) — the `catch_unwind` this
  snapshot exists to be the safety net for.
- [`everything-is-a-command.md`](everything-is-a-command.md) — `mutates` and
  `undoable`, which decide whether the clone is kept or discarded.
- [`../architecture.md`](../architecture.md#undo-clone-before-edit)
