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

## 2. The seams

A tree that has commands, node kinds, tools, panels and layers has five
registries. Written by hand, that is five copies of the same four things: a
vector, a lookup, a duplicate check and a malformed-id check. Five copies is
five places for the checks to drift.

So there is **one** registry, generic over what it holds, and every
contribution is the same shape:

| | |
|---|---|
| **id** | unique, and well formed |
| **capability** | what sort of thing it is — `command`, `node kind`, `tool`, `panel` |
| **layer** | which group of contributions it belongs to |

```rust
pub trait Contribution: Clone + Send + Sync + 'static {
    fn id(&self) -> &'static str;
    fn capability(&self) -> &'static str;
    fn layer(&self) -> &'static str { "core" }
    fn malformed_id(id: &str) -> Option<String> { malformed_dotted_id(id) }
}
```

and the registry is generic:

```rust
pub struct Registry<T: Contribution> { items: Vec<T> }
```

A tree that has seven kinds of extension has seven registries. Written by
hand, that is seven copies of the same four things — a vector, a lookup, a
duplicate check, a malformed-id check. Seven copies is seven places for the
checks to drift, and the drift is invisible until two of them disagree.

### The five seams

| Seam | What it is | What the ecosystem checks |
|---|---|---|
| **command** | something a person or an agent can do | its id, and its layer |
| **node kind** | a kind of thing in the document | that the layer exists |
| **tool** | a gesture that runs a command | that the command exists |
| **panel** | a window's worth of commands | that the commands exist |
| **layer** | a group of contributions that depend on each other | that the graph is acyclic and complete |

The **node kind** is the seam that changes the shape of a tree. A node names
a kind, and the kind knows how it is painted and what is inside it. The core
never matches on a kind: it asks the registry. So a plugin can add a kind of
thing to a document that was written before the plugin existed.

```rust
/// A rectangle.
#[derive(NodeKind)]
#[node_kind(kind = "geometry.rect", label = "Rectangle", layer = "geometry")]
pub struct RectKind;

impl RectKind {
    fn paint(node: &Node, canvas: &mut dyn Canvas) { … }
    fn hit(node: &Node, point: Point) -> bool { … }
}
```

The **layer** is the answer to a tree whose layers are a `const` in a build
tool. A `const` can say what the tree knows at compile time, and it cannot
say anything about a crate that was linked in later. A layer is a
contribution too: it names what it needs, and the ecosystem checks the graph
when the program starts.

---

## 3. The ecosystem: one check, not five

Each registry checks its own ids. But some mistakes are only visible when the
registries are read **together**:

- a tool names a command that is not there;
- a panel names a command that is not there;
- a contribution is in a layer that was not linked in;
- a layer requires a layer that is not there, or requires itself, or the
  requirements have a cycle.

None of these belongs to one registry, so none of them can be checked by one
registry. They are checked by the **ecosystem**, when it is built — which is
before anybody has drawn anything. And it does not fix anything: a missing
command is a refusal, not a warning. A program that starts with a tool that
cannot run is a program that will fail later, in front of a person, instead
of now, in front of a log.

---

## 4. The projection: one source for every front end

A menu, a toolbar, a schema, a chord map and a document are five views of the
same registry. The projection is where they are made, so a command added by a
plugin appears in all five without anybody writing it down five times.

```rust
let projection = ecosystem.projection();
projection.menu(&["command", "effect"]);   // grouped by area
projection.schema("node.mute");            // what an agent is told
projection.manifest();                     // one line per contribution
```

The **capability** is what makes this work. A toolbar wants `tool` and
`command`; a file menu wants `exporter` and `importer`; an agent wants
everything with a schema. There is no list of "the commands in the toolbar"
to keep in step — the list is *derived* from the registry.

---

## 5. The session judges

The old tree asked the **command** whether it had changed anything, and the
command answered honestly if it felt like it. v2 compares the document
before and after, so the answer is a fact:

- a command that says `mutates: false` and changes the document is **refused
  and rolled back** — the promise was a promise;
- an undo entry is pushed when the command mutates, the command is
  undoable, **and** the document really changed. A command that sets a value
  to what it already was leaves no history;
- a command that **panics** leaves the document as it was and is reported as
  an internal error, because a crash is not a result;
- a **batch** is one undo step, so three commands that belong together can
  be undone together.

---

## 6. A plugin

`crates/craftkit-plugin-shapes` depends on the SDK and the macro. **That is
its whole dependency list**, and it does not depend on the application.

It contributes a **layer**, a **node kind**, a **tool**, a **panel** and a
**command** — and the application, which names no symbol of it, gets all
five.

The wiring is generated from the dependency list:

```rust
// generated by build.rs, from the manifest
use craftkit_plugin_hello as _;
use craftkit_plugin_shapes as _;
```

The reference is not decoration. Rust drops a dependency that nothing refers
to, and a dropped dependency takes its registrations with it: before the
build script existed, the plugin's commands were **not in the catalogue**,
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
| **the linked plugin** | a crate that is a dependency; its contributions are there from the first instruction | **built** |
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

## 7. The raw door

Every parameter in v2 is a **type**, and a type is checked. One is not: a
value of any shape, for the contributions that cannot offer a type — a
command from a subprocess, a command from a script.

It is named in the declaration (`kind: json`), so a caller knows it is
holding something unchecked, and it is not the default. A door that is
closed by default and open on purpose is a door; a door that is open is a
hole.

---

## 8. What is here, and what is not

### Built

- `crates/craftkit-sdk` — the whole surface a plugin needs: the typed
  readers, the one registry, the command, the five seams, the ecosystem, the
  projection, the session and the document.
- `crates/craftkit-derive` — five derives, with fifteen tests on the rules
  they hold.
- `crates/craftkit-plugin-hello` — a plugin that adds commands.
- `crates/craftkit-plugin-shapes` — a plugin that adds a **layer, a kind of
  node, a tool, a panel and a command**.
- `apps/craftkit-studio` — an application whose source names no plugin, and
  whose build script generates the wiring.

**57 tests.**

### Not built

- **The evaluator.** v2's document has nodes and a rectangle; it has no
  geometry, no plan and no kernel. The old tree's `craftkit-eval` and
  `craftkit-kernel` are untouched, and the redesign says nothing about them
  yet.
- **The file format.** The old tree's versioned `.craftkit` format, with its
  one-file-per-change discipline, is untouched. A `#[derive(Format)]` and a
  chain of `#[derive(Migration)]` would make the format's versions a fact
  about the types rather than a number somebody remembers to bump — that is
  the next seam, and it is the one the old tree's four-place ceremony points
  at.
- **The front ends.** No toolbar, no MCP server, no control channel. v2 has
  one door — `Session::execute` — and the projection that every front end
  would read.
- **The loaded plugin** (the third door above).

So v2 is a spike on **where behaviour comes from**, not a replacement for
the tree beside it. What it demonstrates is the part that was hardest to
change: the registry, the parameters, the extension seam and the seams
around it.

---

## 9. What the redesign would do to the rest of the tree

The rules of §2 reach further than the command registry, and this is the
part that is a plan rather than a build.

**The model.** The old tree's `Document`, `Node` and `Property` are
hand-written structs with hand-written readers, and the format's version
number is a constant that a file records in its header. A `#[derive(Format)]`
would write the reader and the writer from the struct, and a
`#[derive(Migration)]` would make the version chain a list of contributions
rather than a ceremony wired in four places. The old tree's pending-change
discipline exists because the format is hand-written; a derived format has a
different problem, and probably a smaller one.

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

## 10. Running it

```bash
cd v2
cargo test          # 57 tests
cargo run -p craftkit-studio
```

The application prints the ecosystem (the layers, kinds, tools and panels),
the manifest an agent would be told, the menu by capability, then runs a
command from a plugin, adds a **kind of node** from another plugin, draws the
document **through the kinds**, hit-tests it **through the kinds**, and
finally makes a command at run time and runs that too.

## 11. Honest notes

- **The linker is part of the design.** The build script's generated
  reference is what keeps a plugin in the binary. Without it, the plugin's
  contributions are missing and nothing says so.
- **A run-time command is written out by hand.** A command made while the
  program runs has no struct to derive from, so it reads its own parameter.
  That is the price of the third door.
- **The `Either` is a compromise.** One parameter with two spellings of
  different kinds cannot be a single typed value, so the command is handed
  whichever half arrived and decides what to do with it. The old tree had
  the same pair and the same awkwardness; here at least the *declaration*
  and the *reading* are written once.
- **The document model is small and local.** The node names a kind and
  carries a rectangle. A tree that wants a real document needs a real
  document, and that is design work, not a seam.
- **The derive's grammar is in one place** — the macro — and the gates do
  not read the source at all, so there is no second reader to keep in step.
  This is the problem phase one had, and it is *gone* in v2: the compiler is
  the only reader.
