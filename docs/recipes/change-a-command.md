# Recipe: change a command

Changing a command's parameters is not only a change to a struct. It is a
change to **what the kit promises** — to the schema an agent reads, to the
files on disk, and to the prose. Do it in that order, and say which of the
three you are changing.

---

## 1. Say what kind of change it is

| the change | what else it touches |
|---|---|
| a **new optional** parameter | the struct, the prose |
| a **new required** parameter | the struct, the prose, **and every caller** — including the tools and panels that name the command, and any script that runs it |
| a **renamed** parameter | the struct, the prose, and a **migration** if the name is in a file |
| a **changed default** | the struct, the prose, and a **migration** if the value is in a file |
| a **changed meaning** of an existing parameter | the struct, the prose, and a **migration** — and possibly a new format version |
| a **new kind** of parameter (`Text` → `Number`) | all of the above, and the schema changes shape |

**"Refuse, do not clamp"** applies here too: if a parameter's range changes,
a value that was legal yesterday and is illegal today is **refused**, not
quietly brought into range.

---

## 2. Change the declaration

In `crates/craftkit-plugin-<x>/src/<module>.rs` — a **row** of the battery's
table:

```rust
cad.entity.arc.add {
    area: "cad",
    layer: "cad",
    icon: "arc",
    mutates,
    undoable,
    params: {
        /// The block the arc goes in.
        block: NodeRef,
        /// The name of the arc.
        name: Text,
        /// The radius, in millimetres.
        ///
        /// A **number with a unit** is a different kind from a bare number, and
        /// the unit is part of the parameter's name in the schema.
        radius: Number,
    },
    run: { … }
}
```

A command that is **not** part of a battery is a struct, and the change is
the same one in the other spelling:

```rust
#[derive(Command)]
#[command(id = "cad.entity.arc.add", area = "cad", layer = "cad", icon = "arc", mutates, undoable)]
pub struct AddArc {
    /// The block the arc goes in.
    pub block: NodeRef,
    /// The name of the arc.
    pub name: Text,
    /// The radius, in millimetres.
    pub radius: Number,
}
```

The doc comment on the parameter is the parameter's help, and it goes into the
schema. **Write the help for the caller**, not for the author of the code.

If the parameter has two spellings, say so:

```rust
#[param(or = "other_name")]
name: Text,
```

**The row's id is the command's id, and the struct's name is the id's shape.**
A command that changes its id changes its name with it, and the battery's
anchor names the new one — so an id is not a thing to change casually.

---

## 3. The schema is written, not maintained

The JSON schema an agent reads is written **by the macro from the types** of
the fields. So there is nothing to update by hand: change the type and the
schema changes with it. That is the point of the derivation, and it is why
the schema cannot disagree with the code that reads the parameters.

What you **do** have to check by hand:

- every **tool** that names the command (`#[tool(..., command = "…")]`) still
  makes sense — a tool has no parameters of its own, so a command that now
  needs one cannot be run by that gesture;
- every **panel** that names the command;
- every **script** or **agent** that runs the command with the old
  parameters.

---

## 4. If the change is in a file, it is a format change

A parameter that is stored in the document is in the **file format**, and the
format is a set of contributions:

- a **format version** says what a document of that version looks like, and
  how to read and write it;
- a **migration** says how to move a document from one version to the next.

The versions and the steps live in `crates/craftkit-sdk/src/format.rs`:

```rust
/// The sixth version of the document format.
pub static FORMAT_V6: FormatSpec =
    FormatSpec { tag: "craftkit/6", version: 6, layer: "core", read: Document::from_json, write: write_v6 };

/// The step from the fifth version to the sixth.
#[derive(crate::Migration)]
#[migration(id = "craftkit.5to6", family = "craftkit", from = 5, to = 6, layer = "core")]
pub struct V5ToV6;

impl V5ToV6 {
    /// The step itself.
    fn apply(value: &mut Json) -> Result<(), String> {
        // the old shape becomes the new one
        Ok(())
    }
}
```

The rules:

- **A file is read with the version it was written with, and then migrated
  forward.** Never with the current version.
- **A step is one version to the next.** The chain has no hole: every version
  between the oldest and the newest has a step, and two steps from the same
  version would make the answer a matter of order. The ecosystem checks this
  when it is built, and refuses.
- **A new version is a new `FormatSpec`** with its own reader and writer, and
  the reader reads the version it is for.

---

## 5. Say it in the prose

- The root `README.md` is the design: the command's section says what the
  parameter means, what it refuses, and what its default is.
- If the change introduces a **public bound** (`MAX_*`), the bound must be
  **named in the README with the same number the code has** — the gate's rule
  "the numbers agree with the prose".
- `docs/architecture.md` only needs an edit if the change alters a seam.

---

## 6. Test it

- A test for the **new** behaviour.
- A test for the **old** behaviour where it still holds, and a test that says
  what happens where it does not: a file written by the old version is read
  by the new one, and the step is what carries it.
- A test that the command **refuses** an out-of-range value, rather than
  clamping it.

---

## 7. Run it

```bash
cargo test -p craftkit-plugin-<x>
cargo test -p craftkit-sdk
cargo clippy --workspace -- -D warnings
cargo fmt --check
cargo test -p craftkit-gates
```

**A battery's registry is pinned.** `apps/craftkit-studio/tests/<battery>_registry.rs`
holds every command of the battery as it was, and a change to the **registry**
— an id, a label, a parameter, a flag, a menu path — is a change to that
file:

```bash
cargo test -p craftkit-studio --features <battery> --test <battery>_registry
```

The test says what changed, and the file is the record of what the tree was.

---

## 8. Commit

One topic per commit, and the prose in its own:

```text
feat(cad): the arc's radius — §NNN
docs: the arc's radius — §NNN
```
