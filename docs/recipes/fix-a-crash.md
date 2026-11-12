# Recipe: fix a crash

A crash is a panic. In this tree a panic is not a stack trace on somebody's
terminal: it is a **failed command**, and the document is put back. The rule
that makes that true is the rule this recipe is about:

> **Every crash fix lands with a regression test.**

A fix with no test is a hope.

---

## 1. Know the two boundaries

There are two, and they are in this order.

1. **`craftkit_sdk::guard`** (`crates/craftkit-sdk/src/guard.rs`) — turns a
   panic inside code craftkit does not own into an error. `guard::guard` runs
   the closure and returns the panic's **message**; `guard::message_of` names
   a panic that carried no message.
2. **`Session::execute`** (`crates/craftkit-sdk/src/session.rs`) — clones the
   document, runs the command under `catch_unwind` (`run_guarded`), and on a
   panic **restores the pre-command document** and reports
   `EngineError::Internal { .. }` with the message *"the command panicked; the
   document was put back"*.

So the promise is: **a command that panics is a command that did not happen.**
The document is as it was before it ran.

---

## 2. Reproduce it in a test first

A regression test that passes before the fix is not a regression test. Write
the test so that it **fails now**.

The tree has no test-only `panic` command, so the panic is provoked in the
test's own registry:

```rust
use craftkit_sdk::command::CommandRun;
use craftkit_sdk::serde_json::json;
use craftkit_sdk::{CommandSpec, EngineError, Registry, Session};

/// A command that panics, on purpose.
fn a_panicking_command() -> CommandSpec {
    CommandSpec {
        id: "test.panic",
        label: "Panic",
        area: "test",
        capability: "command",
        layer: "core",
        icon: "",
        doc: "a command that panics, so that the guard can be tested",
        shortcut: None,
        menu: &[],
        params: &[],
        mutates: true,
        undoable: true,
        journal: true,
        enabled: |_| true,
        run: CommandRun::Static(|_session, _params| panic!("the test's own panic")),
    }
}

#[test]
fn a_command_that_panics_leaves_the_document_as_it_was() {
    let mut commands = Registry::new();
    commands.register(a_panicking_command()).expect("the command is registered");
    let mut session = Session::new(commands, Registry::new());

    let error = session.execute("test.panic", &json!({}));
    assert!(matches!(error, Err(EngineError::Internal { .. })), "{error:?}");
    // The document is as it was: the command did not happen.
    assert_eq!(session.document().nodes.len(), 0);
}
```

Run it and **watch it fail** before you fix anything:

```bash
cargo test -p craftkit-sdk
```

---

## 3. Fix the cause, not the symptom

The fix is in the code that panicked. The possible answers, in the order this
tree prefers them:

- **The input was hostile.** Read it through typed helpers, check finiteness,
  bound the size, cap the loop count, and **refuse** what is out of range —
  do not clamp it.
- **The state was impossible.** A node that is not there, a relation that
  points at nothing, a kind that was not linked in: say so, in an error.
- **The feature is not there yet.** Return an error that says
  *"not supported yet: …"*. An unfinished feature is an error, never a panic
  and never a silent nothing.
- **The code does not own the panic.** Use `craftkit_sdk::guard::guard` and
  turn the panic into a message.

---

## 4. Keep the house rules

The crate you touch must still carry the house deny attribute, in this order,
and `#![forbid(unsafe_code)]`:

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

- No `unwrap()`, no `expect()`, no `panic!`, no `todo!`, no `unimplemented!`,
  no `unreachable!` in non-test code.
- `clippy.toml` relaxes the unwrap lints **in tests only**: a test may panic,
  nothing else may.
- The ten crates that may hold `unsafe` are named in
  `crates/craftkit-alloc/tests/gate.rs`, with one marked place each.

---

## 5. Prove it

```bash
cargo test -p <the crate you fixed>
cargo test -p craftkit-sdk
cargo clippy --workspace -- -D warnings
cargo fmt --check
cargo test -p craftkit-gates
```

Then **read the test once more** and ask: does it fail if the fix is undone?
If not, it is not a regression test.

---

## 6. Say it in the prose

- The root `README.md` is the design: if the crash was a **rule** being
  broken, the rule's section says what the rule is now.
- `KNOWN-GAPS.md` gets an entry if the crash was a hole in a gate — and the
  entry says what the gate still cannot see.

---

## 7. Commit

```text
fix(cad): the arc that panicked — §NNN
docs: the arc that panicked — §NNN
```
