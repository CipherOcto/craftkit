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

There are **six** versions of the format here, and each step adds one
thing: the second writes a `muted` flag on every node, the third lets a node
hold other nodes (§14), the fourth lets a node have **no place** and gives
the document a payload of its own (§19), the fifth gives the document
the **relations** between its nodes (§20.1), and the sixth gives it the
**files it needs** (§21.2). Each step adds what the new
version writes to a file that does not have it. A file written by the current
program round-trips to the same document; a file written by an older one is
migrated and read.

A field that may be **absent** is written as `#[format(default)]`, and the
derive reads a file that does not have it as the field's default rather than
refusing it. That is what makes a new field a version and a step rather
than a break in the chain.

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
- **There was nowhere to put geometry.** The battery carried its own path
  building, its own flattening and its own area, and two plugins that both
  drew paths would each carry their own. Path **operations** — union,
  offset, simplify — had no home at all: the SDK's `Path` is a *shape*,
  not an operation. **Fixed in the next slice** (§13): the operations have
  a home, and it is a library rather than a plugin.
- **Colour is not a value type.** v2's parameters are numbers, texts,
  booleans, documents and node references. The battery reads a colour from
  a text (`#rrggbb`) and parses it itself, which is what a plugin should
  do — but a colour *model*, with conversions and a colour space, has no
  seam: `Colour` is an RGBA tuple the SDK owns.
- **A path with a hole was two paths.** The SDK's path was flat and had no
  fill rule, so a shape with a hole was two paths and a rule that was not
  there. **Fixed in the next slice** (§13): the path has contours and a
  rule.
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

## 13. The path operations, and the home they needed

The last section left a list of what the port did not fix, and the first
item on it was: **there is nowhere to put geometry**. Path operations —
union, difference, intersection, offset, simplification — had no home,
because the SDK's `Path` is a *shape* and not an *operation*.

### The answer: a library, not a plugin

`crates/craftkit-vector-geom` is that home, and it is deliberately **not a
plugin**:

- it contributes **nothing** to the registry: no command, no kind, no
  tool, no panel;
- the linker does **not** wire it in — the wiring is for plugins, and this
  is not one;
- a plugin that wants it depends on it the way it depends on the SDK.

That changes one rule, and the change is worth making explicitly:

> A plugin depends on the SDK, the macro, and any library it needs. A
> **library** is not a plugin: it adds nothing, and the linker does not
> wire it in.

The rule it replaces was *a plugin depends on the SDK and the macro, and on
nothing else*. That rule was about **plugins** — the application's are
named by nobody and the application does not change when one is added — and
a library is not one of those. The battery depends on the SDK, the macro
and the geometry, and the geometry adds nothing to the ecosystem.

### What is in it

| Operation | What it is |
|---|---|
| `boolean` | union, intersection, difference and exclusive-or of two paths |
| `area` | the area a path covers, **from the curves** — the integrand is a polynomial and three-point Gauss-Legendre integrates it exactly |
| `bounds` | where a path is |
| `simplify` | the path without the points a flat line would serve |
| `offset` | the path pushed out, or in — **curve-preserving**: the curves are offset as curves |
| `flatten` | a contour as a polygon, for the things that need one |

The **types** are the SDK's. A file never carries a `kurbo` type, the SDK
never depends on `kurbo`, and the conversion happens at the edge of the
library — which is the point of the SDK owning the *shape* of a path and
the library owning the *operations* on it.

The geometry itself is two dependencies rather than copied code: `kurbo`
for the beziers and `linesweeper` for a robust sweep-line boolean
operation. Both are permissively licensed.

### What the operations changed in the core

One thing, and it was forced: **a boolean result can have a hole, and a
hole is a contour**.

The SDK's path was one contour: a start, some segments, and whether it
closed. That cannot hold the difference of a square and a smaller square
inside it — the result is a shape with a hole, and the hole is a contour
of its own. So the path grew:

```text
Path   { subpaths: Vec<SubPath>, rule: FillRule }
SubPath { start: Point, segments: Vec<Segment>, closed: bool }
```

and the rule — non-zero or even-odd — says which parts are inside. The
battery now draws a shape with a hole, and the file carries it.

### What the operations do not do

- **The offset joins corners with a straight line.** A square's offset is
  exact (144 for a 10×10 square offset by 1, and the test says so); a
  curve's corner is approximated, and a corner whose join would be longer
  than four times the distance is cut off rather than spiked out.
- **An offset can meet itself.** A shape with a narrow neck, offset far
  enough, overlaps its own outline. The operation offsets; it does not
  clean up.
- **Simplification keeps the sharp corners.** A polygon with 32 sides
  still has 32 sides: they are corners, and a corner is not a point a
  flat line would serve.
- **Hit testing is still the plugin's, and still by sampling.** The
  geometry library knows the area exactly; the hit test flattens.
- **One path, one fill rule.** A shape with two holes of different
  depths is two contours and one rule, and the even-odd rule handles it;
  the non-zero rule handles it only if the contours run in opposite
  directions.

---

## 14. The document grows a tree, and the session remembers the selection

Two things the port kept saying were missing, and they are the same two
things a person editing a document needs first: **a place to put things in
other things**, and **a way to say which things you mean**.

### The tree

A node has **children** now, and a node with children is a **group**. That
is structural rather than a kind: the core walks into a group and draws
what is in it, and the group's own kind says what the group *means*.

The core contributes one plain kind of group, `core.group`, because a
node's kind has to be in the registry — the document may only hold kinds
the program knows how to draw — and a plugin may add kinds of group: a
layer, a frame, a symbol.

The document's API grew with it: `walk` (every node, in the order they are
drawn), `roots`, `node` and `node_mut` (wherever the node is),
`add_node_under`, `take_node`, `remove_node`.

### The format grew a version

A node may hold other nodes, so what a file holds changed, so the format
has a **third version** — `craftkit/3` — and the step from the second:

```text
craftkit/1  →  craftkit/2  →  craftkit/3  →  craftkit/4  →  craftkit/5  →  craftkit/6
   muted        children      no place, a payload   relations   assets
```

The step gives every node the empty group it would have had. A file of the
second version is a list, and a list is a tree whose nodes have nothing in
them. That is the format seam doing its job: a change to what is saved is
a new version and a step, and the ecosystem checks the chain.

### The selection

The **session** holds what is selected: the ids of the nodes that are
picked out. It is the session's own state, the way the document is, and a
node does not know whether it is selected.

The commands that change it are in the **SDK** rather than in an
application, because the selection belongs to the session and every front
end should have the same commands: `select.all`, `select.none`,
`select.node`.

And a command that acts on "the selection" reads it — which is how the
battery's path operations work now:

```rust
// two ids, or two selected shapes: the same thing said two ways
session.execute("vector.path.unite", &json!({ "first": 1, "second": 2 }))?;
session.execute("select.all", &json!({}))?;
session.execute("vector.path.unite", &json!({}))?;
```

The selection is restored when a command that changed the document is
undone — the undo puts the document back, and the selection goes with it.
Selecting is **not** an undo step of its own: it does not change the
document, so it leaves no history.

### The finding: a plugin program is a whole program

The SDK's commands went into every plugin's catalogue, and the host
refused them: *"two contributions are called `select.none`"*.

That is not a bug in the refusal — it is the right refusal, for the
**linked** door, where two crates contributing one id is a mistake. But a
plugin program is a **whole craftkit program**: it has the core's commands
as well as its own, and so does the host. The two copies are the same
commands.

So the **host** changed: a catalogue command the host already has, with
the same capability, is **skipped** — the host keeps its own — and one
with a *different* capability is still refused, because that is two
different commands with one id.

### What this does not do

- **A layer is not a group.** A group is a container; a layer is a group
  with a meaning of its own (it holds other things and it can be locked
  and hidden), and that meaning is a **kind** a plugin contributes. The
  document does not know what a layer is.
- **There is still no appearance stack** — and that is not a gap in v2 but
  a decision the payload made: a fill and a stroke are the *kind's* data,
  so they live in the node's payload (§12) and the core never has an
  `Appearance` enum to close.
- **The selection is ids.** A person picks a shape by clicking it; the
  selection is still a list of ids underneath, and nothing here knows
  about a pointer.
- **No text, no images, no symbols.** The document is still nodes, a
  payload and a tree.

---

## 15. What is here, and what is not

### Built

- `crates/craftkit-sdk` — the whole surface a plugin needs: the typed
  readers, the one registry, the command, the seams, the ecosystem, the
  projection, the session, the document and the file format. The
  **relations** between the nodes (§20.1), the **journal** of what ran
  (§20.2), the **menu tree** (§20.3), the **real-time lane** (§21.1) and
  the **assets** and the **bundle** (§21.2) are in here too, because
  they belong to the document and the session and not to an
  application.
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
- `crates/craftkit-vector-geom` — the **path operations**: booleans, area,
  bounds, simplification and offset (§13). A library, not a plugin: it
  contributes nothing and the linker does not wire it in.
- `crates/craftkit-plugin-sound` — the **sound battery**: a port of a sound
  application's document into v2's seams, and the three things it found
  (§19), and the mixer that the relations made possible (§20). It is
  optional, like the vector one.
- `crates/craftkit-sound-time` — the **time model**: samples, ticks, a
  tempo map and the three timebases (§19). A library, not a plugin, for
  the same reason the geometry is.
- `apps/craftkit-studio` — an application whose source names no plugin, and
  whose build script generates the wiring.

**237 tests.**

### Not built

- **The kernel.** v2's document has nodes, and a node may have a place; it
  has no numeric core, no panic guard and no geometry engine. The old tree's
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
  the port showed is missing: a typed body, text, and images. The
  **geometry** has a home (§13), the document is a **tree** (§14), and the
  **selection** is the session's state (§14). What is still missing is the
  rest of a document model: layers with a meaning of their own, an
  appearance that is not a plugin's payload, text, and images.

So v2 is a spike on **where behaviour comes from**, not a replacement for
the tree beside it. What it demonstrates is the part that was hardest to
change: the registry, the parameters, the extension seam and the seams
around it.

The four gaps this README listed as "not built" — the file format, the
evaluator, the loaded plugin and a second front end — are now built. The
sections above are what each one is, and the honest notes below are what
each one is not.

---

## 16. What the redesign would do to the rest of the tree

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

## 17. Running it

```bash
cd v2
cargo test          # 237 tests
cargo run -p craftkit-studio
```

The application prints the ecosystem (the layers, kinds, tools, panels,
formats and evaluators), the manifest an agent would be told, the menu by
capability, then runs a command from a plugin, adds a **kind of node** from
another plugin, draws the document **through the kinds**, hit-tests it
**through the kinds**, prints the **plan**, and finally makes a command at
run time and runs that too.

## 18. Honest notes

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
- **The document model is still small.** There are six versions of the
  format, and they differ by a flag, a tree, a place that may be absent,
  a payload of the document's own, the relations between the nodes and
  the files the document needs. The *seam* is the thing being
  demonstrated: a version is a contribution, a step is a contribution, and
  the chain is checked. A document worth versioning is a different piece of
  work.

---

## 19. The sound port: a document with no place in it

The third port. The first was a vector editor (§12), the second was its
path operations (§13), and this one is a **sound editor**: tracks, clips
on them, a tempo map, markers and the media the clips point at.

The tree it comes from is the owner's own: a digital audio workstation
with **482 commands**, a mixer, three plugin hosts and a web target. It
was surveyed read-only before anything was written, and the survey is
what the design below answers to.

### What was ported

A **battery**, optional like the vector one:

- `crates/craftkit-plugin-sound` — seven kinds (`sound.track`,
  `sound.playlist`, `sound.clip`, `sound.tempo`, `sound.meter`,
  `sound.marker`, `sound.source`), seven commands, a tool, a panel and an
  evaluator.
- `crates/craftkit-sound-time` — the **time model**, and it is a
  **library** and not a plugin, for the same reason the geometry is
  (§13): it contributes nothing to the registry and the linker does not
  wire it in. A plugin may depend on a library; the host never names one.

### The mapping

| In the sound tree | In v2 |
|---|---|
| a session | the document, plus a **payload** for the session's own settings |
| a track | a node of kind `sound.track`, with **playlists** under it |
| a playlist (an alternative take) | a node of kind `sound.playlist`, with **clips** under it |
| a clip | a node of kind `sound.clip`, with its span and its content in the payload |
| a tempo or meter event | a node of kind `sound.tempo` or `sound.meter` |
| a marker | a node of kind `sound.marker` |
| a media file | a node of kind `sound.source` — a **reference**, never the audio |
| `TrackKind` (eight variants) | a **string** in the track's payload |
| `ClipContent` (three variants) | a **field** in the clip's payload |
| the tempo map | **derived** from the events, by the evaluator |
| the sample rate | the document's payload, under one name |

Two of those rows are the port's real findings, and they are the next two
paragraphs.

### The finding: a sound document has no place in it

Every node in v2 had a **rectangle**, and the field was not optional. A
host that added a node of a kind it did not know had to invent a box for
it — a tenth of a unit square at the origin — and that invention was in
the file.

A clip in a sound document is at a **time**, and a time is not a place.
So the place became **optional**: a node may have one, and a node with
none says so. The sound kinds have none, and the host no longer invents
one: `node.add` takes a place if the caller wants it, the kind's own
payload if it has one, and nothing at all otherwise.

The file format grew a version for it — `craftkit/4` — and the step from
the third version gives the document an empty payload and leaves the
places where they are. The derive learned a field that may be **absent**,
so a new field is a version and a step rather than a break in the chain.

### The finding: the session had nowhere to live

The **node** had a payload and the **document** did not. A sound session's
sample rate and its timecode belong to the session and not to any one
node, so there was nowhere to put them that was not some node's own data.

So the document grew a payload of its own, one level up from the node's,
and the sound battery keeps its settings under one name in it — so that
two plugins cannot collide by accident.

### The finding: a bug the port walked into

The next free id counted only the nodes at the **top level**. The document
was a list when that was written, and the list was one level deep, and the
count was right by accident.

A sound document is not one level deep: a track holds a playlist, a
playlist holds a clip, and the clip took the id the next top-level node
was going to have. The id walks the **whole tree** now.

### What the kinds of a sound document are

In the tree this ports, the kind of a track is a closed enum of eight
variants and the kind of a clip is a closed enum of three, and the mixer
matches on both of them: adding a kind is an edit in the model, in the
mixer and in the user interface.

Here a track's kind is a **string in the track's payload** and a clip's
content is a **field in the clip's payload**. A third party can add a
kind of track without touching the core — and the plugin's own test says
so, by putting `quantum` in a track and reading it back.

That is the whole of the open-kind seam, arrived at by a port rather than
by an argument, and it is the third time this tree has been changed by
one: the payload (§12), the tree (§14) and now the place.

### The time model

The sound tree's positions are absolute **sample indices**, and their
musical meaning comes from a tempo map that is anchored in **ticks**
rather than in samples, and a track may measure its clips in ticks rather
than in samples.

So the time model is a library of its own: `Samples`, a `TempoMap` with
tempo and meter events, the conversions between ticks and samples (exact
inverses at tick boundaries), bars and beats, and the three timebases a
sound document is written in — bars and beats, minutes and seconds, and
samples.

The tempo map is **derived**: the document holds the *events*, and the
map is what they mean. That is the evaluator's job and not the document's,
and it is the same rule the vector battery follows.

### The blockers, and what each one costs

- **There is no timeline and no camera.** The sound kinds **draw
  nothing**, and that is written in the plugin rather than hidden. A sound
  document is not a picture, and a plugin that drew one would be the
  second set of camera controls in the tree. The camera is a question for
  the tree, not for a plugin: one module owns it, with its own caps, and
  every canvas goes through it.
- **There is no audio thread.** A command in v2 runs on the caller's
  thread, which is the wrong thread for a block of audio: a real-time
  door needs its own seam, its own rules about locking and allocation, and
  its own tests. The sound tree has that seam already, and it is the
  reason its plugin trait splits the work into `process` (audio thread),
  `PluginEditor` (control thread) and `PluginStateReader` (control
  thread). v2 has none of it, and this port does not pretend otherwise.
- **The plugin formats are not ported.** The sound tree hosts CLAP, VST3
  and AU binaries: a user installs a plugin and it is in the session, with
  no source change. That is genuinely open there and it needs **three**
  new crates here (or one with three modules), a format to host them in, and
  the decision of what a plugin's state looks like in a document whose
  payload is JSON.
- **The mixer is not ported.** Ten insert slots and ten send slots per
  strip, a routing graph over buses, and per-track automation are a
  **model**, and the model is where the sound tree's closed enums live
  (`Route`, `AutoParam`, `ChannelFormat`). In v2 they would all be
  payload, which is possible and is not yet written.
- **The menus are not ported.** The sound tree's menu bar is generated
  from a **catalog file** of 512 leaf paths, and a command whose path is
  not in the catalog never appears in the bar. v2's projection is a view
  of the registry, and there is no catalog; a menu tree is a question for
  the tree.

### What this does not claim

- **No sound was made.** Nothing here decodes audio, mixes it or plays
  it. A source is a path and a length.
- **The battery is not the sound tree.** It is a handful of kinds and
  commands that show where a sound document fits in v2's seams. The tree
  it comes from has 482 commands; this has seven.
- **The three findings are not all of the findings.** The survey turned up
  more — the closed enums, the menu catalog, the real-time rules, the
  plugin hosts — and they are listed above as blockers rather than
  answered.

---

## 20. The stressors: what the sound tree has and v2 did not

The sound tree was surveyed before anything was written, and the survey
listed what it has. Most of it is a thing v2 already had. Some of it was
a thing v2 had **no equivalent for**, and those are the interesting
ones: they are the places where a document model built for pictures
breaks, and where a *universal* kit has to be built differently.

### What was answered

| Stressor | What the sound tree has | What v2 does now |
|---|---|---|
| the mixer is a **graph** | tracks feed buses, sends go to buses, a track is under a controller — a graph of typed ids | the document has **relations**: typed edges between nodes, and the kind of an edge is a string (§20.1) |
| a **journal** of what ran | every command and its values, for macro recording | the session keeps a **journal** and can **replay** it, as one undo step (§20.2) |
| a **menu** is a tree | five hundred and twelve leaf paths, from a catalog file | a command says where it goes, and the projection builds the **tree** (§20.3) |
| **automation** | lanes of breakpoints on a mixer parameter | a lane is a node and the parameter is a **string** (§20.4) |
| **inserts** | ten plugin slots per strip, a plugin id and a parameter map | an insert is a node and the plugin is a **string** (§20.4) |
| the kinds of a thing are **closed** | `TrackKind`, `ClipContent`, `AutoParam`, `Route`, `ChannelFormat` — all enums, all matched in the mixer | every one of them is a **string in a payload**, and the core matches on none of them (§19) |

### 20.1 Relations: the document is a graph

The document was a **tree**: a node holds other nodes, and that is
containment. A tree says what holds what. It cannot say what is
*attached* to what — and a mixer is attached, not contained: a clip plays
a source and is not in it, a track feeds a bus and is not in it, a track
is under a controller and is not in it either.

So the document grew **relations**:

```rust
pub struct Relation { pub from: u64, pub to: u64, pub kind: String }
```

The kind is a **string** — `source`, `route`, `send`, `vca`, `loop`,
`asset`, `symbol` — because the kinds of relation are as open as the
kinds of node. The relations are in the **document** and not in the node,
because a relation is between two nodes and a node does not know what it
is attached to.

The commands are in the SDK, because they belong to the document and
every front end should have them: `link.add`, `link.remove` and
`link.list`. The file format grew a version for them (`craftkit/5`), and
the step from the fourth gives a document that has none the none it has.

**What it cost:** a field on the document, a version, a step, and three
commands. **What it bought:** the mixer became possible, and with it the
idea that a document may hold a graph as well as a tree.

### 20.2 The journal: what the session was asked to do

The session had a **log** of strings for a person to read and a
**history** of documents for a person to undo. Neither can be run again.

It now has a **journal**: every command that ran, with the values it was
given, exactly as they were given. It is a program:

```text
sound.track.add   { "name": "drums" }
sound.clip.add    { "track": 3, "start": 0, "length": 48000 }
```

and `journal.replay` runs it again, as **one undo step**, with the same
promise every command makes: a step that fails puts the document back the
way it was.

A command may say `no_journal` — the commands that are *about* the
journal say it, because a journal that lists the command that cleared it
is a journal that was not cleared. The sound tree has the same flag on
its command spec, for the same reason.

**What it cost:** a vector, a limit of a thousand entries, three
commands, and a flag. **What it bought:** macro recording, a script an
agent can hand back to the program, and a way to reproduce a failure.

### 20.3 The menu is a tree

A command had an **area**, and the projection made a flat list of them.
One level is not enough: the sound tree's menu bar is five hundred and
twelve leaf paths, and a bar that can only say `area` cannot draw them.

So a command may say where it goes:

```rust
#[command(id = "sound.track.add", menu("Track", "New"), …)]
```

and the projection builds the **tree**. The path names the parts, and
the command sits in the last one; a command that names no path sits
under its area, which is where it sat before. A part with nothing in it
is dropped: a battery that is not linked in leaves a hole, and a hole is
not a menu.

**What it cost:** a field, an attribute, and a tree in the projection.
**What it bought:** a menu bar that can be drawn, and a front end that
does not have to invent its own hierarchy.

### 20.4 The mixer, in payloads

The second slice of the sound battery is the mixer, and it is where the
relations earn their keep:

- a **bus** is a node, and the tracks that feed it say so with a
  relation of the kind `route`;
- a **send** is a node under a track, and the bus it goes to is a
  relation of the kind `send` — the node says *what the send is* and the
  relation says *where it goes*;
- an **insert** is a node under a track, and the plugin is a **string**:
  the sound tree has thirty built-in plugins in a static table and a
  hard-coded match on the id, so a plugin that is not in the table
  cannot be created at all;
- an **automation lane** is a node under a track, and it says which
  parameter it moves — `volume`, `pan`, `mute`, or anything a payload
  names. The sound tree has a closed enum of mixer parameters with one
  escape for plugin parameters, so a lane can automate only what the
  enum knows.

And the **routing** is derived: the evaluator reads the relations and
works out the order the buses are fed in. The document holds the edges;
the evaluator works out what they mean.

### What was not answered

These are the stressors that are still open, and each one says what it
would cost:

- **Real time.** The sound tree's plugin trait splits the work into
  `process` (the audio thread), `PluginEditor` and `PluginStateReader`
  (the control thread), and the audio callback never blocks and never
  allocates. v2 has **no real-time door**: a command runs on the caller's
  thread. A real-time seam needs its own rules, its own guards and its
  own tests, and it is a piece of work, not a flag.
- **Media that travels.** A source is a path, and the path is relative
  to the document. The sound tree saves a **bundle**: the document and
  the audio files in one archive. v2 has no assets and no bundle, and a
  document that references a file it cannot carry is a document that
  breaks when it moves.
- **A selection of spans.** The selection is **ids**. The sound tree
  also has an edit *range* — a span of time on a track — and half of
  its editing commands are about a range and not about a node. A
  universal selection would carry spans as well as ids, and a span needs
  a domain: a time range, a rectangle, a range of characters.
- **View state.** Zoom, scroll and the tool in the hand are the
  **session's** state in the sound tree and not the document's, and v2
  keeps the selection that way but has nowhere for the rest.
- **The plugin formats.** CLAP, VST3 and AU binaries are genuinely open
  in the sound tree: a user installs one and it is in the session, with
  no source change. Porting that needs a host crate per format, a
  format to host them in, and a decision about what a plugin's state is
  in a document whose payload is JSON.
- **The menu catalog.** The sound tree's menu is generated from a
  **file** of five hundred and twelve paths, and a command whose path is
  not in the file never appears in the bar. v2's menu is generated from
  the registry, which is better, and it has no catalog — and that is the
  point: a catalog is a second source, and the tree this ports has two
  sources and the drift between them.

### What a universal kit is, after three ports

Every port asked for the same things, and they are the list:

- **open data** — the payload, so that a kind can say what it is
  without the core knowing (§12);
- **containment** — the tree, so that a document can hold a group, a
  layer, a track's playlist (§14);
- **attachment** — the relations, so that a document can hold a graph
  and not only a tree (§20.1);
- **no assumption of space** — a node may have no place, so a document
  that is not a picture is a document (§19);
- **derived state** — the evaluator, so that what a document *means* is
  worked out and not stored (§8, §20.4);
- **a machine record** — the journal, so that what happened is a
  program and not a string (§20.2);
- **one projection** — the menu tree, the schema and the manifest, all
  made of the registry (§4, §20.3).

What is still missing from that list is the one thing no port has needed
yet: **a real-time door**, and **assets that travel with the document**.
Both are named above, with what they would cost, and both are the next
thing to build.

---

## 21. The two doors that were missing: real time, and files

The list at the end of §20 named two things a universal kit needs and
that no port had asked for: a **real-time door** and **files that travel
with the document**. Both are here now, and both are seams rather than
implementations — which is what this tree is for.

### 21.1 The real-time lane

The sound tree's plugin trait is the rules of a real-time lane written
down:

| Rule | Where it comes from |
|---|---|
| `process` never allocates and never panics | the trait's contract |
| the audio callback never blocks; it only ever **tries** a lock | the playback crate |
| a panicking block **silences the output** instead of killing the app | the guarded block |
| a plugin that outputs a NaN is taken out of the signal path | the failure containment |
| instances are created on the control thread and **handed over** | the instance handover |

None of those rules is about audio. They are about **work that must
finish**, and a kit that is to be universal needs them for audio, for
video, and for a solver's iteration.

So there is a **lane**:

```rust
pub trait LaneJob: Send {
    fn prepare(&mut self, block: usize) -> Result<(), String>;  // the control thread
    fn run(&mut self, frames: usize) -> Block;                   // the lane
}
```

- **prepare** runs where everything expensive may happen: buffers are
  made here, state is read here, a plugin is created here. A job that
  cannot be prepared is an error and there is no lane;
- **run** runs on the lane and must do bounded work. A panic in it is
  caught, the block is **silenced**, and the lane carries on — the
  program still holds the user's document, because a crash in the
  real-time work is not a crash of the program.

Three details are the whole of the design:

- the **counters are atomics**, and not a lock: a lock between the lane
  and the control thread would be a lock *on the lane*;
- the **mailbox is taken with `try_lock`** and never waited for: a
  mailbox that made the lane wait would break the promise, so the newest
  message wins and the lane moves on;
- the **work is handed back** when the lane stops, so that it is freed
  on the control thread and not on the lane.

And `on_lane()` answers "am I on a lane right now?", so that work which
must not be on one — anything that allocates, locks or reads a file —
can say so instead of finding out the hard way.

**What this does not do, and says so:** there is **no device**, so
nothing here plays a sound, and a real host drives the lane from its
device's callback instead. There is **no priority**: nothing asks the
operating system for a scheduling class. And the **allocation rule is a
rule and not a measurement** — the lane's own path allocates nothing per
block, and the job is told not to allocate, but this tree does not claim
a measurement it has not made. The measurement would take a counting
global allocator, which is `unsafe` by definition, and v2 has no
`unsafe` at all.

### 21.2 The files a document needs

A document that points at a file is **not finished**: it is a set of
instructions that only work on the machine that has the files. Send it
to somebody else and it opens with holes in it.

So the document **says what it needs**. Its **assets** are the
references — a path, a kind, and the length if anybody knows it:

```rust
pub struct Asset { pub path: String, pub kind: String, pub length: Option<u64> }
```

The bytes are **not** in the document, because a document that holds its
media is a document that cannot be edited. And a **bundle** carries them:
the document and the files it names, in one file, with a checksum, so
that a bundle that was damaged on the way is **refused** rather than
half-read. The format is small and written out in the SDK rather than
pulled in, because a container that a document format depends on is
worth owning.

The commands are in the SDK, because the list belongs to the document:
`doc.asset.add`, `doc.asset.remove` and `doc.assets`.

The sound battery is the first user, and it is what makes the list worth
having: a **source is a file**, and adding a source now says so in the
document's list. The document says what it needs in one place, instead
of in every payload that happens to hold a path.

The file format grew a version — `craftkit/6` — and the step from the
fifth gives a document that names no files the none it has.

### What is still missing

- **A device.** The lane has no sound and no clock. A host that wants
  one drives the lane from the device's callback, and that is the
  host's business.
- **The measurement of the allocation rule.** It wants a counting
  global allocator, which is `unsafe` by definition. v2 has no
  `unsafe` at all, and that is worth more than the measurement until
  somebody decides otherwise.
- **A bundle that the application writes.** The container is here; the
  studio has no media to put in one, so the command that writes a
  bundle is not written.
- **The rest of the §20 list**: a selection of spans rather than of
  ids, the view state, and the three plugin formats.

---

## 22. The selection holds parts, and there is a tool in the hand

The last two items of the §20 list that are about the **session's** own
state: a selection that can hold a *range*, and the tool the user is
holding.

### The selection was a list of ids, and a range is not a node

A selection of ids can say "this clip". It cannot say "**the first
second of** this clip", and half of the sound tree's editing commands
are exactly that: a trim, a split, a nudge, a razor. They act on a
**range**, and the range is not a node.

So a selected thing is now a node **and the part of it that is picked
out**:

```rust
pub struct Selected {
    pub node: u64,
    pub from: Option<f64>,   // in the node's own units
    pub to: Option<f64>,
}
```

The range is in the node's **own units** — samples for a clip, whatever
a shape's axis is for a shape — because there is no universal axis. The
core carries the numbers and the **kind** reads them: the same bargain
the payload makes, one level down. A kit that pretended there was a
universal axis would be right in exactly one domain.

`select.span` selects a part; `select.node` selects a whole one; and the
commands that want whole things ask for `selected_whole_ids()`, because a
boolean of half a path is not a thing the vector battery does.

### The first user: a trim

The sound battery's `sound.clip.trim` takes its range from the caller
**or from the selection**, and it is where the design is proved: a trim
shortens the clip, moves its start, and — the part that is easy to get
wrong — **moves the content with it**, so that a clip trimmed at its head
starts later in the source.

### The tool in the hand

A **tool** is a contribution — a gesture that runs a command — but a
tree of tools is not a person: a person holds **one** of them at a time.
Which one is the **session's** state and not the document's, because two
people editing the same document may hold different tools and neither of
them has changed the document.

So the session holds it (`tool.hold`, `tool.release`, `tool.held`), it
comes back with an undo, and it leaves no history: holding a tool does
not change the document.

### What is still not here

- **The zoom and the scroll.** They are view state, and a view needs a
  **camera**, and this tree has none: the window draws a node graph, a
  timeline and a properties panel. The tree's rule is that a canvas must
  have **one** set of camera controls, one module that owns them, with
  its own caps — so a zoom without a camera would be a number with
  nothing to zoom, and a second place to grow a camera in.
- **The transport.** Where the playhead is, and whether it is playing,
  is **real-time** state, and it belongs to the lane (§21.1) and not to
  the session's document state.
- **The plugin formats.** CLAP, VST3 and AU are still not ported, and
  they want a host crate each, a format to host them in, and a decision
  about what a plugin's state is in a document whose payload is JSON.

---

## 23. Three more surveys: does what is here survive other domains?

The sound port was one application. The question a universal kit has to
answer is whether the seams survive **other** applications, and the only
honest way to ask is to point them at three more and see what breaks.

Three read-only surveys, of the owner's own projects:

| Project | What it is | What it is to a kit |
|---|---|---|
| **pdfcraft** | a PDF editor: an object graph, a rasteriser, forms, annotations | a document whose nodes are **bytes** as much as JSON |
| **filmcraft** | a video editor: a tree of bins, a flat registry of items, sequences, tracks | a document with **two clocks** per clip, and a mixer over the tree |
| **effectcraft** | a compositing app: comps, layers, a property tree, keyframes | a document addressed by **property paths** and evaluated with **expressions** |

### What was already served

This is the interesting part, and it is the point of the exercise:

- **A selection of parts** (§22) turned out to cover PDF's **glyph
  ranges** and filmcraft's **gap selections** — a gap is a range on a
  track with nothing in it, and a glyph range is a range on a page. Both
  are a span of a node, which is what the span selection is.
- **Relations** (§20.1) cover PDF's **shared and cyclic object graph**
  and filmcraft's **flat registry** referenced from the tree: an object
  that several places point at is a node with several relations to it.
- **The payload** covers filmcraft's **two clocks per clip**, effectcraft's
  **property trees** and **keyframes**, and PDF's **dictionaries**.
- **Derived state** (§8) covers PDF's **object views**, which are computed
  from the graph and never stored twice.
- **Kinds as contributions** and **the command registry** cover PDF's
  two-hundred-variant edit enum and its command list.

So the span selection, the relations and the payload — three things that
were built for the sound port — carried two of the three new domains
without being changed.

### What was not, and what changed

- **A payload that cannot carry bytes.** A PDF stream is arbitrary binary
  and must round-trip byte for byte, and JSON cannot hold it. So the SDK
  has **base64** in the payload (`craftkit_sdk::bytes`), written out
  because a plugin should not need a crate to put a picture in a
  payload. The cost is written down: base64 is a third bigger, and a
  document that carries fifty megabytes of streams carries sixty-seven
  megabytes of text. The bigger answer — a node with a dictionary *and* a
  stream, which is exactly a PDF object's shape — would cost a field, a
  format version and base64 in the file anyway, and it is named as the
  next thing to build if a document ever needs it.
- **No background work that may take a while.** The lane is for work
  that must **finish**; a page of a document takes far too long to draw
  in the middle of a frame, and an export takes minutes. So there is a
  **job pool**: a queue with priorities, workers, **cancellation** (a job
  that has not started does not run at all, and a running job finds out
  through its flag) and **progress** (a job says how far it has got and
  the caller reads it). All three of the surveys asked for it — a render
  pool, a background job list with progress, an export job.

### What is still missing, and it is three things

- **Branching undo.** effectcraft's history is a **tree** of snapshots,
  not a line: an edit made in the past can be taken back and the work
  after it kept. v2's history is a stack. This is the one gap that is
  about the *session* and not the document, and it is the next thing to
  build.
- **Forward compatibility.** A file written by a newer program keeps the
  fields this one does not know. v2's reader drops them, so a document
  written by a program with a newer kind comes back **smaller**. The
  payload survives because the core does not read it, and that is the
  half that works; the half that does not is a reader that keeps what it
  does not understand.
- **The second payload field** for a document that carries a lot of
  bytes, as above.

Everything else the three surveys named is a **domain** thing and lives
in a payload: the proxy and the offline flag of a media item, the
environment-resolved font, the render queue, the output templates, the
property paths, the keyframes with their eases, the cache on the disk
beside the document. None of them wants a core that knows what it is.

### What this says about the kit

Three ports and three more surveys, and the shape is holding: the
**payload** carries what is data, the **tree** carries what contains
what, the **relations** carry what is attached to what, the **span
selection** carries what part of what, the **evaluator** carries what
is derived, the **lane** and the **job pool** carry the two kinds of
work, and the **bundle** carries the files.

What is missing is not a domain thing. It is a **history that branches**
and a **reader that keeps what it does not understand** — and both are
about the kit being right for the *next* program, and not only for this
one.
