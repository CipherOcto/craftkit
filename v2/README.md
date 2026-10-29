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
| **capability** | what sort of thing it is — `command`, `node kind`, `tool`, `panel`, `evaluator`, `format` |
| **layer** | which group of contributions it belongs to |

```rust
pub trait Contribution: Clone + Send + Sync + 'static {
    fn id(&self) -> &str;
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

### The seams

| Seam | What it is | What the ecosystem checks |
|---|---|---|
| **command** | something a person or an agent can do | its id, and its layer |
| **node kind** | a kind of thing in the document | that the layer exists |
| **tool** | a gesture that runs a command | that the command exists |
| **panel** | a window's worth of commands | that the commands exist |
| **layer** | a group of contributions that depend on each other | that the graph is acyclic and complete |
| **evaluator** | what a whole document means | that the layer exists |
| **format** | one version of the file format | that the layer exists |
| **migration** | one step from one version to the next | that it is a step, and that the chain has no hole |

Eight seams, **one** registry, **one** check on each, **one** manifest, **one**
projection. A tree that has eight kinds of extension has eight registries
written by hand — eight copies of the same four things, and the drift is
invisible until two of them disagree.

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
| **the loaded plugin** | a **separate program**, started while the program runs; its commands arrive over a wire | **built** |

The third door is a **subprocess**, and the wire is newline-delimited JSON.
It has three messages:

```text
plugin → host   {"kind":"catalogue","commands":[ … ]}
host → plugin   {"kind":"run","id":"hello.greet","params":{ … },"document":{ … }}
plugin → host   {"kind":"result","ok":true,"message":"…","document":{ … }}
```

The **catalogue** is the plugin's commands, with their ids, their labels and
their parameters. The host turns each one into a command in its **own**
registry, so a command from a subprocess is the same thing as a command from
a linked plugin: the same door, the same metadata, the same refusal of an
unknown parameter.

The **document travels with the request**, and the answer may bring a new one
back. That is what lets a plugin in another process change the document: the
host hands it the document, the plugin hands back the document it would
have. The host's session judges the change exactly as it judges a linked
plugin's — by comparing before and after.

Why the document travels, rather than being shared: shared memory between
two processes that do not know each other is a protocol with no rules. A
document that is written down and read back has one rule — it is the same
shape on both sides, and the shape is the file format.

`apps/craftkit-hello-server` is the plugin program. It is a **full craftkit
program**: it has its own ecosystem, its own commands and its own session,
and it serves them. The host does not link it.

What this door does **not** buy: a subprocess plugin is **not checked by the
compiler**. Its catalogue is data, and a catalogue with a mistake in it is
refused when the plugin is loaded, not when it is compiled. That is the
price of the door, and it is why the other two exist.

---

## 7. The file format: versions are contributions

The old tree changes its format by adding **one file per change**, and the
change has to be wired in **four places**: the file, the table of pending
changes, the `mod` declaration, and the match that says which step applies.
The ceremony is deliberate — it is what lets two people change the format at
once without a merge conflict — and it exists because the format is
**hand-written**.

Here the format is contributions:

- a **format** says what a document of that version looks like, and how to
  read and write it;
- a **migration** says how to move a document from one version to the next.

So a change is one file and one derive, and the wiring is the list of
contributions — which the link fills.

```rust
/// The step from the first version to the second.
#[derive(Migration)]
#[migration(id = "craftkit.1to2", family = "craftkit", from = 1, to = 2, layer = "core")]
pub struct V1ToV2;

impl V1ToV2 {
    fn apply(value: &mut Json) -> Result<(), String> { … }
}
```

The rule that matters:

> **A file is read with the version it was written with, and then migrated
> forward.**

Never with the current version: a file written by an old program has to be
readable by a new one, and the only way to know that is to carry the version
and to have the steps that bridge it.

The ecosystem checks the chain at start-up, and refuses:

- a step that is not one version forward;
- two steps out of the same version (the answer would be a matter of order);
- a step past the newest format;
- **a version with no step out of it** — a file of that version would have
  nowhere to go.

There are two versions of the format here, and they differ by one thing: the
second writes a `muted` flag on every node and the first does not. The step
adds the flag to a file that does not have it. A file written by the current
program round-trips to the same document; a file written by an older one is
migrated and read.

---

## 8. The evaluator: a kind knows what its nodes mean

The old tree's `NodeKind` is an enum, and every place that matches on it has
to be updated when it grows. v2's node names a **kind**, and the kind is
contributed — and the kind carries its **own evaluation**:

```rust
#[derive(NodeKind)]
#[node_kind(kind = "geometry.rect", label = "Rectangle", layer = "geometry", evaluates)]
pub struct RectKind;

impl RectKind {
    fn paint(node: &Node, canvas: &mut dyn Canvas) { … }
    fn hit(node: &Node, point: Point) -> bool { … }
    fn evaluate(node: &Node, document: &Document) -> Option<Json> { … }
}
```

So "add a kind" is "write the kind", not "edit an enum and six matches".

There is a second kind of evaluator, because a kind can only speak about
**its own nodes** and sometimes the question is about the whole document. An
**evaluator** is a contribution like every other — an id, a layer, and the
ecosystem checks the layer is there:

```rust
#[derive(Evaluator)]
#[evaluator(id = "geometry.totalarea", label = "Total area", layer = "geometry")]
pub struct AreaEvaluator;

impl AreaEvaluator {
    fn evaluate(document: &Document) -> Plan { … }
}
```

The **plan** is the sum: what the kinds say about their nodes, then what the
evaluators say about the document. It is a **value**, not a drawing — what to
do with it is the front end's business.

What this is **not**: a kernel. The old tree's `craftkit-kernel` has a panic
guard, a numeric core and a plan of its own shape, and none of that is
rebuilt here. What is rebuilt is the **seam**: where a kind's evaluation
comes from, and the fact that the core never matches on a kind.

---

## 9. The raw door

Every parameter in v2 is a **type**, and a type is checked. One is not: a
value of any shape, for the contributions that cannot offer a type — a
command from a subprocess, a command from a script.

It is named in the declaration (`kind: json`), so a caller knows it is
holding something unchecked, and it is not the default. A door that is
closed by default and open on purpose is a door; a door that is open is a
hole.

---

## 10. What is here, and what is not

### Built

- `crates/craftkit-sdk` — the whole surface a plugin needs: the typed
  readers, the one registry, the command, the seams, the ecosystem, the
  projection, the session, the document and the file format.
- `crates/craftkit-derive` — eight derives, with tests on the rules they
  hold.
- `apps/craftkit-hello-server` — a plugin that is a **separate program**.
- `crates/craftkit-plugin-hello` — a plugin that adds commands.
- `crates/craftkit-plugin-shapes` — a plugin that adds a **layer, a kind of
  node, a tool, a panel, a command and an evaluator**.
- `apps/craftkit-studio` — an application whose source names no plugin, and
  whose build script generates the wiring.

**79 tests.**

### Not built

- **The kernel.** v2's document has nodes and a rectangle; it has no numeric
  core, no panic guard and no geometry engine. The old tree's
  `craftkit-kernel` is untouched. What v2 has is the **seam** the kernel
  would plug into: a kind carries its own evaluation, and the plan is a
  value.
- **The old tree's format.** The old tree's versioned `.craftkit` format,
  with its one-file-per-change discipline, is untouched. What v2 has is a
  format that is a **contribution**, with migrations that are contributions,
  and a version chain the ecosystem checks.
- **The old tree's front ends.** No toolbar, no MCP server, no control
  channel. v2 has one door — `Session::execute` — and the projection that
  every front end would read.

So v2 is a spike on **where behaviour comes from**, not a replacement for
the tree beside it. What it demonstrates is the part that was hardest to
change: the registry, the parameters, the extension seam and the seams
around it.

---

## 11. What the redesign would do to the rest of the tree

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

## 12. Running it

```bash
cd v2
cargo test          # 79 tests
cargo run -p craftkit-studio
```

The application prints the ecosystem (the layers, kinds, tools, panels,
formats and evaluators), the manifest an agent would be told, the menu by
capability, then runs a command from a plugin, adds a **kind of node** from
another plugin, draws the document **through the kinds**, hit-tests it
**through the kinds**, prints the **plan**, and finally makes a command at
run time and runs that too.

## 13. Honest notes

- **The linker is part of the design.** The build script's generated
  reference is what keeps a plugin in the binary. Without it, the plugin's
  contributions are missing and nothing says so.
- **A run-time command is written out by hand.** A command made while the
  program runs has no struct to derive from, so it reads its own parameter.
  That is the price of the run-time door.
- **The `Either` is a compromise.** One parameter with two spellings of
  different kinds cannot be a single typed value, so the command is handed
  whichever half arrived and decides what to do with it. The old tree had
  the same pair and the same awkwardness; here at least the *declaration*
  and the *reading* are written once.
- **The derive's grammar is in one place** — the macro — and the gates do
  not read the source at all, so there is no second reader to keep in step.
  This is the problem phase one had, and it is *gone* in v2: the compiler is
  the only reader.
- **A subprocess plugin's metadata is leaked, once.** A command's labels are
  `&'static str`, and a subprocess plugin's labels arrive at run time. They
  are leaked when the plugin is loaded — bounded by the number of commands
  the plugin has, and once each. The alternative is a `Cow`/`factory`
  refactor of every command's metadata, which is a wider change than this
  spike should make. It is a real leak, and it is written down here rather
  than left to be discovered.
- **The wire protocol is a decision, not a derivation.** Newline-delimited
  JSON was chosen because it is trivial to write in any language and easy to
  read in a log. A different wire would fit the same shape.
- **A subprocess plugin is not checked by the compiler.** Its catalogue is
  data, and a mistake in it is refused when the plugin is loaded.
- **The plan is a value, not a drawing.** Nothing renders it, and nothing
  checks it against a rule. The old tree's `craftkit-eval` had a plan with
  a shape and a kernel behind it; what is here is the **place a plan comes
  from**, which is the part the enum made hard.
- **The document model is still small.** Two versions of the format differ
  by one flag, because the document has one kind of node and a rectangle.
  The *seam* is the thing being demonstrated: a version is a contribution,
  a step is a contribution, and the chain is checked. A document worth
  versioning is a different piece of work.
