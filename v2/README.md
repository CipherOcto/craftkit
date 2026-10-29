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

#[used]
pub static CRAFTKIT_PLUGIN_ANCHOR_CRAFTKIT_PLUGIN_HELLO: &[fn() -> usize] =
    craftkit_plugin_hello::CRAFTKIT_PLUGIN_ANCHOR;
#[used]
pub static CRAFTKIT_PLUGIN_ANCHOR_CRAFTKIT_PLUGIN_SHAPES: &[fn() -> usize] =
    craftkit_plugin_shapes::CRAFTKIT_PLUGIN_ANCHOR;
```

The reference is not decoration. Rust drops a dependency that nothing refers
to, and a dropped dependency takes its registrations with it: before the
build script existed, the plugin's commands were **not in the catalogue**,
and the failure was silent. That is worth writing down, because it is the
sort of thing that looks like a bug in the plugin and is really a fact about
the linker.

The **anchor** is the same fact, one step further on. The `use` keeps the
crate in the program on the native targets; on
`wasm32-unknown-unknown` it does not, and the plugin's commands are missing
there — silently, with an empty registry (§11). So each plugin writes one
anchor of its own, naming its items:

```rust
// in the plugin crate, once
craftkit_sdk::plugin_anchor!(HelloGreet, HelloCount);
```

and the generated file names that anchor. A name is a relocation, and a
relocation is something a linker cannot drop.

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

## 9. The second front end

A front end is a **view**, and the registry is what it views. The studio has
a window; `apps/craftkit-bridge` has none — it is a **server** that speaks
JSON-RPC over its standard input and output, and an agent drives it.

Both read the **same projection** and call the **same door**:

| | the studio | the bridge |
|---|---|---|
| the commands | `projection.menu(&[…])` | `projection.schema(id)` |
| running one | `Session::execute` | `Session::execute` |
| the plan | `session.evaluate()` | `session.evaluate()` |

So a command that a plugin adds is in the studio's menu **and** in the
bridge's tool list, without anybody writing it down twice — because it is in
the **registry**, and both front ends are views of the registry.

The bridge's methods are `initialize`, `tools/list`, `tools/call`,
`resources/list`, `plan` and `shutdown`. An unknown method says which
methods there are, because an agent that asks for the wrong thing should be
told what the right things are.

This is the cheapest of the four additions, and that is the point: a second
front end costs **one file** and no changes anywhere else.

---

## 10. The raw door

Every parameter in v2 is a **type**, and a type is checked. One is not: a
value of any shape, for the contributions that cannot offer a type — a
command from a subprocess, a command from a script.

It is named in the declaration (`kind: json`), so a caller knows it is
holding something unchecked, and it is not the default. A door that is
closed by default and open on purpose is a door; a door that is open is a
hole.

---

## 11. The browser: what the wasm target changes

The tree this replaces has a web build, so the question belongs here: **what
does `wasm32-unknown-unknown` do to this design?**

Three answers, all **measured** on this machine rather than reasoned about.

### There is no `dlopen`, and there is no third door

`dlopen` loads a shared object while the program runs. WebAssembly has no
such thing: a wasm module is linked at build time, and the only dynamic
thing in a browser is a **JavaScript import**. So a "loaded plugin" on the
web is not a shared library — it is a wasm module the page imports, and its
contributions can only reach the program through the **JavaScript
boundary**, never through a Rust registry.

That is a real constraint on this design and not a small one: the registry
is filled by the **linker**, and the linker's door does not exist in a
browser. A plugin that arrives while the page runs cannot register a Rust
`CommandSpec`; it can only hand the program **data** — a catalogue of
commands, in JSON.

Which is the **third door in a different shape**: the wire is already JSON
(§6), and in a browser it would be a function call and a promise instead of
a pipe. The **subprocess** door cannot exist there at all:

| thing | `wasm32-unknown-unknown` | measured |
|---|---|---|
| `std::process::Command::spawn` | "operation not supported on this platform" | yes |
| `std::net::TcpListener::bind` | "operation not supported on this platform" | yes |
| `dlopen` | no analogue at all | — |

So the honest doors on the web are the **linked plugin** (compiled into the
wasm module) and a **catalogue across the JavaScript boundary**. There is
no subprocess and there is no shared library.

### The linker drops a plugin that nothing names — and it does not say so

This is the one that bit, so it is written down carefully.

A plugin registers itself by putting a `#[used]` static in its **own**
object code, and the **linker** decides whether that object code is in the
program. On the native targets it keeps it. On `wasm32-unknown-unknown` it
does not, and it does not say so.

Measured, with the real v2 crates, on this machine:

| what the application names | commands in the registry |
|---|---|
| nothing (`use plugin as _;` only) | **0** |
| one item of the plugin | 3 |

A program that starts, reports no error, and has an **empty registry** is
the worst kind of bug, and on that target it was the default.

The fix is the **anchor**. Every plugin crate writes one public static that
names its items, and the application names that static. A name is a
relocation, and a relocation is something a linker cannot drop.

```rust
// in the plugin crate, once
craftkit_sdk::plugin_anchor!(HelloGreet, HelloCount);

// in the file the build script generates
#[used]
pub static CRAFTKIT_PLUGIN_ANCHOR_CRAFTKIT_PLUGIN_HELLO: &[fn() -> usize] =
    craftkit_plugin_hello::CRAFTKIT_PLUGIN_ANCHOR;
```

Re-measured with the anchor in place: **3 commands** on
`wasm32-unknown-unknown`, from the same wiring, and the whole v2 workspace
builds for that target. The anchor is what makes that build mean
something.

### What this does not claim

- The wasm build is **not run** by anything in this tree. It compiles, and
  the anchor was measured by instantiating the module under Node and asking
  the ecosystem how many commands it had.
- The browser bundle is **not built here**: the old tree's `cargo xtask
  wasm` is a compile check, and it says so (`KNOWN-GAPS.md` §13).
- The old tree is **not** affected by the drop: it does not use
  `inventory`, and its commands are a static array
  (`crates/engine/src/command.rs`).

---

## 12. The batteries: a real application, ported

A spike proves that a seam *can* hold. It does not prove that it holds a
**real application**. So the vector battery is here: the shape of a vector
illustration program — a path in the document, shapes that make paths, a
colour on a path, a plan that says what they add up to — written as v2
plugins.

`crates/craftkit-plugin-vector` is that port. It depends on the SDK and the
macro, and on nothing else, and it adds a **layer**, a **node kind**, three
**commands**, a **tool**, a **panel** and an **evaluator**. It is
**optional**: the studio and the bridge have a `vector` feature, and with it
off the crate is not there at all.

```bash
# the battery is in
cargo run -p craftkit-bridge --features vector --bin craftkit-bridge

# the battery is not
cargo run -p craftkit-bridge --bin craftkit-bridge
```

### What the port changed in the core

Three things, and each of them was a wall:

1. **The node carries a payload.** A node was a name, a kind and a
   rectangle, and a path is none of those. The node now carries the kind's
   own data as JSON: the core carries it, does not read it, and the file
   carries it too. A plugin's body type is the plugin's own.
2. **The canvas draws a path.** `Canvas` had a rectangle and a line of
   text. It now has a path — lines and cubic curves — with a fill and a
   stroke.
3. **The linker knows about optional plugins.** The wiring was generated
   from the dependency list, so an optional dependency was wired in whether
   or not its feature was on: a compile error. The linker now reads the
   features and wires in what is there.

### What the port found, and did not fix

The point of a port is the list of what it hits.

- **A payload is untyped.** The core carries JSON and the plugin parses it.
  The plugin's own type is checked by the plugin's own code, and the core
  cannot say whether a payload is well formed. The other answer is a
  typed body — the vector PoC's `NodeBody` trait object, which the kind
  parses from JSON and the core holds opaquely — and that is **not** in
  v2. It is the shape of the decision, not the decision.
- **The drawing vocabulary is the SDK's.** `Canvas` is a trait, so any
  backend can implement it: open for **backends**. It is closed for
  **kinds**: the set of things a kind may ask for is what the trait says,
  and every addition is a change to the SDK. A vector document needs
  strokes, gradients, clips, transforms and images. Each one is a fifth
  argument somewhere.
- **There is nowhere to put geometry.** The battery carries its own path
  building, its own flattening and its own area. Two plugins that both
  drew paths would each carry their own. And path **operations** — union,
  offset, simplify — have no home at all: the SDK's `Path` is a *shape*,
  not an operation. The application this was ported from has a geometry
  crate and a path-operations crate for exactly this; v2 has no place for
  them.
- **Colour is not a value type.** v2's parameters are numbers, texts,
  booleans, documents and node references. The battery reads a colour from
  a text (`#rrggbb`) and parses it itself, which is what a plugin should
  do — but a colour *model*, with conversions and a colour space, has no
  seam: `Colour` is an RGBA tuple the SDK owns.
- **A path with a hole is two paths.** The SDK's path is flat and has no
  fill rule, so a shape with a hole is two paths and a rule that is not
  there.
- **Hit testing is the plugin's, and it is approximate.** A curve is hit
  by its samples: right for a polygon, wrong near an edge.
- **The document is still small.** No layers, no groups, no selection, no
  appearance stack, no text, no images. The battery puts paths in a
  document that is a list of nodes.

### What the port did *not* need

Worth as much as the list above: **the registry did not change**. The
command registry, the projection, the typed parameters, the session's
judging, the file format's versions and migrations, the ecosystem's
start-up checks, the panel, tool and evaluator seams, the subprocess door
and the second front end — all of it took a real plugin unchanged. The
port needed the **document** and the **canvas**, and nothing else. That is
the shape of the thesis, tested against something real.

And the plugin's own type system stayed in the plugin: the payload type,
the colour parsing, the geometry and the flattening are all the battery's,
and none of them is in the core.

---

## 13. What is here, and what is not

### Built

- `crates/craftkit-sdk` — the whole surface a plugin needs: the typed
  readers, the one registry, the command, the seams, the ecosystem, the
  projection, the session, the document and the file format.
- `crates/craftkit-derive` — **eight** derives (`Command`, `NodeKind`,
  `Tool`, `Panel`, `Layer`, `Evaluator`, `Format`, `Migration`), with tests
  on the rules they hold.
- `crates/craftkit-link` — the wiring between an application and its plugins:
  the dependency list is the plugin list, and each plugin is named by its
  **anchor** so that a linker cannot drop it (§11).
- `apps/craftkit-hello-server` — a plugin that is a **separate program**.
- `apps/craftkit-bridge` — a **second front end**: the same registry, a
  JSON-RPC door.
- `crates/craftkit-plugin-hello` — a plugin that adds commands.
- `crates/craftkit-plugin-shapes` — a plugin that adds a **layer, a kind of
  node, a tool, a panel, a command and an evaluator**.
- `crates/craftkit-plugin-vector` — the **vector battery**: a port of a real
  application's shape into v2's seams, and the list of what it found (§12).
  It is optional: a program that wants paths says so.
- `apps/craftkit-studio` — an application whose source names no plugin, and
  whose build script generates the wiring.

**104 tests.**

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
  channel. What v2 has is the **projection** every front end would read, and
  one front end built on it as a proof.
- **A real document model.** The vector battery (§12) put paths, curves and
  colours in a document that is still a list of nodes with rectangles. What
  the port showed is missing: a typed body, a layer and group hierarchy, a
  selection, an appearance stack, text, images, and **geometry** — a place
  for path operations, which the SDK's `Path` is not.

So v2 is a spike on **where behaviour comes from**, not a replacement for
the tree beside it. What it demonstrates is the part that was hardest to
change: the registry, the parameters, the extension seam and the seams
around it.

The four gaps this README listed as "not built" — the file format, the
evaluator, the loaded plugin and a second front end — are now built. The
sections above are what each one is, and the honest notes below are what
each one is not.

---

## 14. What the redesign would do to the rest of the tree

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

## 15. Running it

```bash
cd v2
cargo test          # 91 tests
cargo run -p craftkit-studio
```

The application prints the ecosystem (the layers, kinds, tools, panels,
formats and evaluators), the manifest an agent would be told, the menu by
capability, then runs a command from a plugin, adds a **kind of node** from
another plugin, draws the document **through the kinds**, hit-tests it
**through the kinds**, prints the **plan**, and finally makes a command at
run time and runs that too.

## 16. Honest notes

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
- **The bridge is a proof, not a product.** It speaks JSON-RPC over stdio
  because that is the cheapest way to show a second front end over the
  same projection. The old tree's MCP server has a schema, a transport and
  an authorisation story, and none of that is here.
- **The plan is a value, not a drawing.** Nothing renders it, and nothing
  checks it against a rule. The old tree's `craftkit-eval` had a plan with
  a shape and a kernel behind it; what is here is the **place a plan comes
  from**, which is the part the enum made hard.
- **The third door does not exist on the web.** A subprocess plugin needs a
  process, and `wasm32-unknown-unknown` has none: `spawn` and `bind` both
  answer "operation not supported on this platform". On that target the
  doors are the linked plugin and a catalogue across the JavaScript
  boundary (§11).
- **The document model is still small.** Two versions of the format differ
  by one flag, because the document has one kind of node and a rectangle.
  The *seam* is the thing being demonstrated: a version is a contribution,
  a step is a contribution, and the chain is checked. A document worth
  versioning is a different piece of work.
