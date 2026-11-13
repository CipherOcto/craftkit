# Recipe: add a command to a battery

A command's parameters are its fields. The macro writes the declaration, the
reader and the registration; the registry is a value and the battery never
names the application.

**There are two spellings, and the tree uses both.** A **single** command is
a struct with `#[derive(Command)]`. A **battery** is a **table** — one
`commands! { … }` per file, one row per command — and every battery of the
tree is written that way. A row's id is the command's id, and the compiler
writes the same struct, reader, registration and spec for it that the derive
writes for a struct.

This is the whole procedure. It takes about fifteen minutes and one
`cargo test`.

---

## 0. What you need

- the battery you are adding to: `crates/craftkit-plugin-<x>/`
- an id: `<area>.<thing>`, dotted, lower case, no underscores — for example
  `cad.entity.arc.add`
- the **area** the command belongs under, which is the area the battery's
  other commands use

Check that the id is free — the tree's own tool answers it:

```bash
cargo xtask find cad.entity.arc.add
```

A grep finds it too, in either spelling: the attribute says
`id = "cad.entity.arc.add"` and the row says `cad.entity.arc.add {`.

---

## 1. Write the row

In `crates/craftkit-plugin-<x>/src/<module>.rs` — a new module if the battery
has one per kind of thing, otherwise the battery's `lib.rs`. The file already
holds a **table**; the new command is a **row** in it:

```rust
use craftkit_derive::commands;
use craftkit_sdk::{Bool, EngineError, Outcome, Text};

commands! {
    /// Say hello to somebody.
    ///
    /// The doc comment is the command's own line about itself: it is what the
    /// catalogue shows and what the schema says.
    hello.greet {
        area: "hello",
        icon: "hello",
        params: {
            /// Who to greet.
            ///
            /// The doc comment on a **parameter** is that parameter's help.
            name: Text,
            /// Whether to shout.
            loudly: Option<Bool>,
        },
        run: {
            let name = &self.name.0;
            let greeting = match self.loudly {
                Some(Bool(true)) => format!("HELLO, {name}!"),
                _ => format!("Hello, {name}."),
            };
            Ok(Outcome::new(greeting))
        }
    }
}
```

### The rules of the declaration

- The row's **id** is the command's id: dotted, lower case, and in the
  battery's area. The compiler writes the struct, and **its name is the id's
  own shape** — `hello.greet` is `HelloGreet`.
- The row may carry: `label`, `area`, `layer`, `icon`, `shortcut`,
  `capability`, `menu("…", "…")`, `enabled`, and the flags `mutates`,
  `undoable`, `no_journal`, `no_prefix`, `example`. **No `label`** means the
  label is taken from the id.
- `mutates` says the command changes the document; the session **checks** it,
  and puts the document back if a command that did not promise to change it
  did. `undoable` puts the command in the undo history.
- The **parameters** are the fields of `params`, and their **types** are the
  kinds: `Text`, `Number`, `Bool`, `NodeRef`, `Option<T>` for an optional
  parameter, and `Either<A, B>` for a parameter with two spellings.
- A parameter's doc comment is that parameter's help. A parameter with **two**
  spellings says so: `#[param(or = "other_name")]`.
- `run` is the command's body; it reads `self`, and it returns
  `Result<Outcome, EngineError>`. **Never a panic**, never an `unwrap`.
- A command may have **helpers** of its own: a `methods: { … }` block beside
  `run`, and the compiler writes them into the command's own `impl`.

### The other spelling: one command on its own

A command that is **not** part of a battery is a struct:

```rust
use craftkit_derive::Command;
use craftkit_sdk::{Bool, EngineError, Outcome, Session, Text};

/// Say hello to somebody.
#[derive(Command)]
#[command(id = "hello.greet", area = "hello", icon = "hello")]
pub struct HelloGreet {
    /// Who to greet.
    pub name: Text,
    /// Whether to shout.
    pub loudly: Option<Bool>,
}

impl HelloGreet {
    fn run(&self, _session: &mut Session) -> Result<Outcome, EngineError> {
        Ok(Outcome::new(format!("Hello, {}.", self.name.0)))
    }
}
```

The attribute carries the same vocabulary — `id`, `label`, `area`, `icon`,
`shortcut`, `capability`, `layer`, `enabled`, `menu("…", "…")`, `mutates`,
`undoable`, `no_journal`, `no_prefix` — and the fields are the parameters,
and the field's own attributes (`#[param(or = "…")]`) are the same.

---

## 2. Name it in the anchor

At the bottom of the battery's `lib.rs`:

```rust
craftkit_sdk::plugin_anchor!(HelloGreet);
```

The anchor is the one name the application gives the crate, and the
relocation it carries is what keeps the command in the program — including on
`wasm32-unknown-unknown`, where a crate nothing names is dropped whole. A
command that is not in the anchor is not in the registry, and nothing says so.

The name in the anchor is the **id's shape** — `hello.greet` is
`HelloGreet`, `cad.group.move` is `CadGroupMove` — because the table writes
the struct's name from the id. The anchor and the row must agree, and the
compiler says so if they do not.

---

## 3. Write the test next to it

Tests go **next to the code**: a `#[cfg(test)] mod tests` at the end of the
module, or a file in `crates/craftkit-plugin-<x>/tests/`. A test that runs a
command needs a session:

```rust
use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

fn session() -> Session {
    let ecosystem = Ecosystem::from_inventory().expect("the ecosystem is built");
    Session::with_formats(
        ecosystem.commands,
        ecosystem.kinds,
        ecosystem.formats,
        ecosystem.migrations,
        ecosystem.evaluators,
    )
}

#[test]
fn the_greeting_says_the_name() {
    let mut session = session();
    let outcome = session
        .execute("hello.greet", &json!({ "name": "world" }))
        .expect("the greeting runs");
    assert_eq!(outcome.message, "Hello, world.");
}
```

A test may panic: `clippy.toml` re-enables the unwrap lints **in tests
only**. Nothing else may.

---

## 4. Run it

```bash
cargo test -p craftkit-plugin-hello
cargo clippy -p craftkit-plugin-hello -- -D warnings
cargo fmt --check
cargo test -p craftkit-gates
```

The last one is the tree's own gate: it checks the house deny attribute, the
layering, that a plugin depends on no other plugin, and that the front end is
thin.

**A battery's registry is pinned.** `apps/craftkit-studio/tests/<battery>_registry.rs`
holds every command of the battery as it was — its id, label, area,
capability, layer, icon, documentation, chord, menu path, parameters and
flags — and the test says whether the registry still says it. A command that
changes the registry changes that file, and the file is the proof:

```bash
cargo test -p craftkit-studio --features <battery> --test <battery>_registry
```

---

## 5. Say it in the README

The tree's README is the design, and a command is part of it. Add a line to
the battery's section saying what the command does and what it refuses. If the
command introduced a **public bound** — a `MAX_*` constant — the bound must be
named in the README **and the number in the README must equal the number in
the code**: that is the gate's rule "the numbers agree with the prose".

---

## 6. Commit

One topic per commit:

```text
feat(hello): the greeting — §NNN
docs: the greeting — §NNN
```

`CONTRIBUTING.md` has the form and the rules.
