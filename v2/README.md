# v2 — a metaprogramming-centric craftkit

A spike: the same idea as the tree beside it, built the other way round.

The tree beside it believes in **gates**: a program reads the source and
checks it. v2 believes in **the compiler**: the declaration is written once,
in the place where the decision is made, and everything else is derived from
it. Where the old tree needed a gate to notice that a command declared a
parameter it never read, v2 makes that mistake impossible to write.

Nothing here is a proposal for the tree beside it. It is a **different
answer** to the same question, built far enough to be measured.

---

## 1. The thesis

> **A command is a struct whose fields are its parameters. The compiler
> writes the declaration, the reader and the registration. The registry is a
> value, and any crate that is linked in can put a command in it.**

Three consequences fall out of that sentence, and they are the whole of v2.

**1. The gates that cost the most are the ones that disappear.**
The old tree spent two of its gates on one question — *does a command's
declaration match what its body does?* — and both read the source as text.
In v2 the question has no answer to be wrong about: a command's body is
handed a **struct**, not a bag of values, so it cannot read a parameter it
did not declare; and the reader is generated from the fields, so it cannot
declare a parameter it does not read.

**2. A plugin is a dependency, not a fork.**
The old tree's registry was `static ALL: [&CommandSpec; 25]` — one list, in
one crate, with a hand-written count. Adding a command meant editing that
crate, so every application that wanted the command had to take the edit. In
v2 the registry is a value built at run time from whatever the binary
contains, and a plugin crate contributes its commands by being **linked in**.

**3. A fact is written once.**
The old tree wrote a command's id in the registry, in the docs, in the MCP
schema and in the prose, and a gate compared them. v2 writes the id once, in
the attribute, and the catalogue, the schema and the documentation are
*derived*.

---

## 2. The ten rules

| | Rule | Where it is held |
|---|---|---|
| 1 | A command is a struct whose fields are its parameters. | the type |
| 2 | The parameter's **kind** is its type. | `ParamKind` |
| 3 | A parameter is **required** when its type is not an `Option`. | `Option<T>: ParamKind` |
| 4 | A parameter with two spellings is an `Either`. | `Either<A, B>` |
| 5 | A command's body never sees a parameter object. | the generated `run` |
| 6 | A command's documentation is its doc comment. | the derive macro |
| 7 | The registry is a **value**, built at run time. | `Registry` |
| 8 | A crate that is linked in contributes its commands. | `inventory` + the derive macro |
| 9 | The areas are **derived** from the commands, not listed. | `Registry::areas` |
| 10 | One way to run a command: `Session::execute`. | the SDK |

Rules 2 to 4 are the ones that do the work. They turn a **table** into a
**type**, and a table is something a person can get wrong.

---

## 3. What a command looks like

```rust
/// Mute or unmute a node.
///
/// The node may be given as an **id** or as a **name**: one parameter, two
/// spellings, and the type says so.
#[derive(Command)]
#[command(
    id = "node.mute",
    area = "node",
    icon = "mute",
    mutates,
    undoable,
    enabled = has_nodes
)]
pub struct NodeMute {
    /// The node's id, or its name.
    #[param(or = "node_name")]
    pub node: Either<NodeRef, Text>,
    /// True to mute, false to unmute. The opposite of the node's own state
    /// when it is not given.
    pub muted: Option<Bool>,
}

impl NodeMute {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        // …
    }
}
```

The macro writes:

- the **declaration** — the id, the area, the label (from the id), the
  documentation (from the doc comment) and the parameter table (from the
  fields and their types);
- the **reader** — `NodeMute::read(&params)`, which reads each field by its
  own name and with its own kind;
- the **entry point** — a function that reads the parameters and then calls
  `run`;
- the **registration** — a link-time registration, so the linker puts the
  command in the registry.

The body gets `&self` and `&mut Session`. It does not get a `Params`, so it
cannot read an undeclared parameter, cannot index one, and cannot reach the
raw JSON. **That is the whole of rule 5**, and it is a compile-time fact
rather than a rule somebody has to remember.

### What the compiler knows now

| Fact | In the old tree | In v2 |
|---|---|---|
| the parameter's name | a table | the field's name |
| the parameter's kind | a table | the field's type |
| whether it is required | a table | whether the type is an `Option` |
| the other spelling | a table | `Either` + `#[param(or = …)]` |
| what it does | a hand-written sentence | the doc comment |
| what the button says | a hand-written string | the id, capitalised |
| the id and the area agree | a gate reading the source | a compile error |
| `undoable` without `mutates` | a gate reading the source | a compile error |
| two parameters with one name | a gate reading the source | a compile error |
| a parameter read but not declared | a gate reading the source | **impossible** |
| a parameter declared but not read | a gate reading the source | **impossible** |

The last two are the expensive ones. The old tree needed a *text scanner*
to compare a declaration with a body, and the scanner had five documented
defects, each of them a false green. v2 has nothing to compare.

---

## 4. A plugin

`crates/craftkit-plugin-hello` depends on the SDK and the macro. **That is
its whole dependency list**, and it does not depend on the application.

```rust
/// Say hello to somebody.
#[derive(Command)]
#[command(id = "hello.greet", area = "hello", icon = "hello")]
pub struct HelloGreet {
    /// Who to greet.
    pub name: Text,
    /// Whether to shout.
    pub loudly: Option<Bool>,
}
```

No registration code. No list to add to. No call into the application.

The application lists the plugin in `Cargo.toml` — that is the whole of the
wiring — and a **build script** turns the dependency list into the
references:

```rust
// generated by build.rs, from the manifest
use craftkit_plugin_hello as _;
```

The reference is not decoration. Rust drops a dependency that nothing
refers to, and a dropped dependency takes its registrations with it: before
the build script existed, the plugin's commands were **not in the catalogue**,
and the failure was silent. That is worth writing down, because it is the
sort of thing that looks like a bug in the plugin and is really a fact about
the linker.

So the rule is:

> **The dependency list is the plugin list, and the wiring is generated from
> it.**

Adding a plugin is one line in `Cargo.toml` and nothing anywhere else. A
test says so: `the_application_names_no_plugin` reads the application's
source and refuses to find the word `craftkit_plugin` in it.

### The three doors

| Door | What it is | State |
|---|---|---|
| **the linked plugin** | a crate that is a dependency; its commands are in the registry from the first instruction | **built** |
| **the run-time command** | a `CommandSpec` written by hand and registered while the program runs | **built** |
| **the loaded plugin** | a `.so` loaded with `dlopen` while the program runs | **not built** |

The third door is the one that needs the most thought, and v2 has not
thought about it yet. A dynamically loaded plugin has to hand its commands
over explicitly, because a plugin loaded with `dlopen` has its own copy of
the SDK unless the SDK is a shared library — and if it is, then the registry
is shared state that two copies of the program could disagree about. The
honest shape is probably a **hand-off**: the plugin exports a function that
fills a registry the host owns, and the host calls it after loading. That is
what the run-time door already does, one step further on.

---

## 5. The registry is a value

```rust
let registry = Registry::from_inventory()?;   // every command that was linked in
registry.areas();                              // derived: doc, hello, node
registry.ids();                                // derived
```

The old tree's registry was a `static` with a hand-written length, and the
length was a number that a gate compared with the prose. v2 has no length to
write down: the catalogue is whatever was registered, and the areas are
computed from it — which is why the plugin's own area, `hello`, is in the
list without anybody having said so.

Registration is **open at run time**:

```rust
registry.register(runtime_echo())?;   // a command that exists only now
```

and the registry **refuses** a duplicate id, which is the one thing that has
to be checked at run time rather than by the compiler.

---

## 6. What is here, and what is not

### Built

- `crates/craftkit-sdk` — the whole surface a plugin needs: the typed
  readers, the command shape, the registry, the session and the document.
  **2307 lines** of Rust in the whole of v2, 39 tests.
- `crates/craftkit-derive` — the macro, with eleven tests on the rules it
  holds.
- `crates/craftkit-plugin-hello` — a plugin that depends on the SDK and the
  macro, and on nothing else.
- `apps/craftkit-studio` — an application whose source names no plugin, and
  whose build script generates the wiring.

### Not built

- **The evaluator.** v2's document has nodes and names; it has no geometry,
  no plan and no kernel. The old tree's `craftkit-eval` and `craftkit-kernel`
  are untouched, and the redesign says nothing about them yet.
- **The file format.** The old tree's versioned `.craftkit` format, with
  its one-file-per-change discipline, is untouched.
- **The front ends.** No toolbar, no MCP server, no control channel. v2 has
  one door — `Session::execute` — and nothing else to reach it.
- **The loaded plugin** (the third door above).

So v2 is a spike on **where behaviour comes from**, not a replacement for
the tree beside it. What it demonstrates is the part that was hardest to
change: the registry, the parameters and the extension seam.

---

## 7. What the redesign would do to the rest of the tree

The rules of §2 reach further than the command registry, and this is the
part that is a plan rather than a build.

**The model.** The old tree's `Document`, `Node` and `Property` are
hand-written structs with hand-written readers, and the format's version
number is a constant that a file records in its header. A `#[derive(Format)]`
would write the reader and the writer from the struct, and a
`#[derive(Version)]` would make the version a fact about the types rather
than a number somebody remembers to bump. The old tree's pending-change
ceremony — one file per change, wired in four places — exists because the
format is hand-written; a derived format has a different problem, and
probably a smaller one.

**The evaluator.** The old tree's `NodeKind` is an enum, and every place
that matches on it has to be updated when it grows. A kind that carries its
own evaluation — a trait object per kind, registered the way commands are —
turns "add a kind" from "edit the enum and six matches" into "write the
kind". The kernel's panic guard and the plan's shape stay as they are.

**The front ends.** The toolbar, the MCP server, the CLI and the control
channel all read the registry, so they all get the new commands for free.
What they lose is the compile-time knowledge that the registry is a closed
set — and what they gain is that a plugin does not have to be in the
application's crate to be in its toolbar.

**The gates.** The old tree's gates are a program reading text, and they are
good at it: the pattern document's five measured false greens are the price
of not parsing. With v2's declarations, the checks move to the compiler and
the gates are left with the questions that are genuinely about the tree as a
whole — the layering, the assets, the clean room, the format's versions.
**A gate that reads text is a gate that cannot see a macro**, and that is
the sentence the whole of phase one was about.

---

## 8. Running it

```bash
cd v2
cargo test          # 39 tests
cargo run -p craftkit-studio
```

The application prints the catalogue — the application's own commands and
the plugin's — runs one of each, and then makes a command at run time and
runs that too.

## 9. Honest notes

- **The linker is part of the design.** The build script's generated
  reference is what keeps a plugin in the binary. Without it, the plugin's
  commands are missing and nothing says so.
- **The run-time command is written out by hand.** A command made while the
  program runs has no struct to derive from, so it reads its own parameter.
  That is the price of the third door.
- **The `Either` is a compromise.** One parameter with two spellings of
  different kinds cannot be a single typed value, so the command is handed
  whichever half arrived and decides what to do with it. The old tree had
  the same pair and the same awkwardness; here at least the *declaration*
  and the *reading* are written once.
- **The plugin's crate is linked, not loaded.** A plugin that is a
  dependency is checked by the compiler and is there from the first
  instruction; a plugin that is loaded can arrive at any moment, and needs
  the hand-off that v2 has not built.
- **The derive's grammar is in one place** — the macro — and the gates do
  not read the source at all, so there is no second reader to keep in step.
  This is the problem phase one had, and it is *gone* in v2: the compiler is
  the only reader.
