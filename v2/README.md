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
- a **state** is pushed when the command mutates, the command is undoable,
  **and** the document really changed. A command that sets a value to what it
  already was leaves no state;
- a command that **panics** leaves the document as it was and is reported as
  an internal error, because a crash is not a result;
- a **batch** is one undo step, so three commands that belong together can
  be undone together.

The history is a **tree** of those states, not a line of them: undo walks to
the parent, redo walks to a child, and a change made after an undo starts a
branch instead of throwing the old work away (§24).

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
- `crates/craftkit-plugin-pdf` — the **PDF battery**: the object model of
  a PDF document — indirect objects, dictionaries, streams and
  references — ported into v2's seams, and the three things it found
  (§27). It is optional, like the others.
- `crates/craftkit-plugin-film` — the **film battery**: the document model
  of a video editor — bins, items, tracks and clips — ported into v2's
  seams, and the three things it found (§30). It is optional, like the
  other three.
- `crates/craftkit-plugin-fx` — the **compositing battery**: the document
  model of a compositing application — comps, layers, property trees and
  the paths that address them — ported into v2's seams, and the three
  things it found (§31). It is optional, like the others.
- `crates/craftkit-plugin-art` — the **artist's battery**: the document model
  of an artist's IDE — projects, scenes, tracks, elements, an opaque 3D stage
  and remote generations — ported into v2's seams, and the three things it
  found (§33). It is optional, like the others.
- `crates/craftkit-plugin-grid` — the **spreadsheet battery**: the document
  model of a spreadsheet — workbooks, sheets, cells in a payload, and the
  formulas and the graph of areas between them — ported into v2's seams,
  and the four things it found (§42). It is optional, like the others.
- `crates/craftkit-plugin-cad` — the **CAD battery**: the document model
  of a CAD drawing — layers, blocks, entities and the edges between them
  — ported into v2's seams, and the three things it found (§44). It is
  optional, like the others.
- `crates/craftkit-alloc` — the **counting allocator**: the first of the
  crates that may hold `unsafe`, and what turns the lane's "must not
  allocate" rule into a measurement (§32). It is a **dev** dependency of
  the studio and nothing else.
- `crates/craftkit-ffi-abi` — **the contract** between a host and a plugin
  that is not part of the program: a versioned table of function
  pointers. No dependencies at all (§34).
- `crates/craftkit-ffi` — **the door**: opens a shared library, finds the
  entry point by name, checks the version and calls the table (§34). The
  second crate that may hold `unsafe`.
- `crates/craftkit-plugin-shared` — **a plugin that is not part of the
  program**: a shared library with an exported entry point, and the proof
  that the door works on something the host has never seen (§34).
- `apps/craftkit-studio` — an application whose source names no plugin, and
  whose build script generates the wiring.

**432 tests.** The MCP server links **all eight batteries** in, so the
workspace's count is the same with or without the studio's features
(`craftkit-studio/vector`, `/sound`, `/pdf`, `/film`, `/fx`, `/art`,
`/grid`, `/cad`) — and the studio on its own, with no features, carries
341 of them.

### Not built

- **The kernel.** v2's document has nodes, and a node may have a place; it
  has no **numeric core**: there is no type for a length in millimetres or
  an angle in radians, and every number in a document is a JSON number.
  The old tree's `craftkit-kernel` is untouched. What v2 has is the
  **seam** the kernel would plug into: a kind carries its own evaluation,
  and the plan is a value. The **panic guard** is built (§38, and the
  command path before it), and the **geometry** is a library of its own
  (§13) — so what is missing is the typed value, not the machinery
  around it.
- **The old tree's format.** The old tree's versioned `.craftkit` format,
  with its one-file-per-change discipline, is untouched. What v2 has is a
  format that is a **contribution**, with migrations that are contributions,
  and a version chain the ecosystem checks.
- **The old tree's front ends.** No toolbar, no control channel. The **MCP
  server is built** (§41: one tool per command, generated from the
  registry), and what v2 has besides that is the **projection** every front
  end would read, and one front end built on it as a proof.
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
cargo test          # 432 tests
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

**This list is history.** It is what the survey found on the day it was
written, and every item on it has an answer now — the table at the end
of §37 says which. The list is left as it was, because what it cost is
part of the record:

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
operating system for a scheduling class. And the **allocation rule is now
a measurement** — it was a rule and not a measurement until §32, which
built the counting global allocator the rule needed and proved the lane's
work allocates nothing.

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
- **The measurement of the allocation rule** — **this one is built now**
  (§32): a counting global allocator, the first of the three crates on
  **the list** (§34), and a test that proves the lane's work allocates
  zero times.
- **A bundle that the application writes** — **this one is built now**
  (§25.1): the studio writes one and reads one back. And a bundle that a
  *plugin* fills — **that one is built now too** (§25's stream, used by
  the art battery in §33): the files come from the document's assets and
  from the nodes' streams, and a plugin's own file is a node's stream, so
  the plugin says which file it is without asking for a new seam.
- **The rest of the §20 list**: a selection of spans rather than of ids
  (still not built), the view state (the **camera** is built, §29; what
  is not built is a view state that outlives the session), and the three
  plugin formats — **the door they come through is built** (§34: a
  shared library, an entry point by name, a versioned table), and the
  formats themselves are not, because each is a large ABI in another
  language and each would be a shim over that door rather than a change
  to it.

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

Two of the three were built after this survey was written, and §24 says how
and what they cost. The third is still open.

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

---

## 24. The two gaps closed: a history that branches, and a reader that keeps

§23 ended with three things missing. Two of them were about the kit being
right for the *next* program rather than this one, and both are now built.

### The history is a tree

The history was a **stack**: a line of documents, walked backwards one at a
time. That is fine until a person undoes three steps, changes something
else, and wants the three steps to still be there — one undo away, on the
other side of the change they just made. A line cannot hold that; a **tree**
can.

The history is now a tree of states. A state is a whole session state: the
document, the selection, and what was in the hand. Undo walks to the parent,
redo walks to a child, and a change made after an undo adds a child to the
state that was undone to. The old work is not thrown away; it is on its own
branch.

| | |
|---|---|
| `history.undo` | walk to the parent of the current state |
| `history.redo` | walk to a child — the newest one, or the one that is named |
| `history.go` | go to any state in the tree, and the work after it is kept |
| `history.tree` | the tree: the states, the current one, the branches, the path |

The tree is **bounded** at 256 states: the oldest one goes when it is full,
and what came from it takes its place. The cost is that the history forgets
how far back it goes, which is what a bound is.

Two bugs came out of the change, and both were found by the tests rather than
by reading:

- a **batch** only pushed an undo step when the history was empty, so a
  batch after any other command was not undoable at all;
- a command that ran a batch pushed a **second** step on top of the batch's
  own, so a journal replay was two undo steps rather than one.

The survey that found the gap is the compositing one, whose history is a tree
of snapshots. It is the only gap in §23 that was about the *session* and not
about the document, and it is the one that would have bitten the first real
person to use the thing.

### The reader keeps what it does not understand

A file written by a newer program used to come back **smaller**. The reader
read the fields it knew and dropped the rest, so the next program to open
the file would find the work gone.

Now the fields the reader does not understand are **kept**, and written back
as they were:

- the **document** and the **node** each carry the fields the reader did not
  understand, and the writers merge them back into the file, so the file
  keeps the shape it had;
- the derive macro learned a **merging field**: written by merging, read as
  what is left over;
- a file from a **newer** program is read rather than refused. It is read
  with the newest reader this program has, what that reader does not
  understand is kept, and the reader **says** that the file was newer, so
  that the program says so out loud: *"opened `song.craftkit` (written by a
  newer program, version 9; the parts this program does not understand were
  kept)"*;
- a file of a **family** this program does not know is still refused: there
  is nothing there it could keep.

The rule that makes the kept fields safe is that **a step must not overwrite
a field that is already there**. The steps use `or_insert` and never
`insert`, and there is a test for it: a file of the fifth version that
already has the assets the sixth version writes keeps them.

### What this cost, said plainly

- A program that reads a file from the future **cannot know** that a field it
  *does* know has not changed its meaning. It reads the fields whose meaning
  has not changed, and it keeps the rest. That is the price of reading
  forward, and it is not a small one.
- The file is written back in **this program's own version**, so the version
  number of the file is lost. The data is not. A program that wants to keep
  the version has to keep it in the payload.
- The kept fields are a **liability as well as an asset**: a document that
  went through a program that did not understand it carries fields that no
  program in the tree has ever looked at.

### What is still missing

One of the three is still there, and it is the smallest:

- **The second payload field** for a document that carries a lot of bytes: a
  dictionary *and* a stream, which is exactly a PDF object's shape. The
  answer for now is base64 in the payload (§23), and the answer costs a
  third more space and a format version when it arrives.

And the things that were never in the list, still not in it: the transport
(which belongs to the lane), and the CLAP, VST3 and AU plugin formats (they
are FFI, and FFI is unsafe). The allocation measurement of the lane is no
longer in that list: it is built (§32), and it cost one piece of `unsafe`
in a crate of its own. The **door** for the plugin formats is built too
(§34): the formats themselves are not, and they are a much larger piece
of work than the door they would come through.

### What this says about the kit

The two gaps were the same shape, and that is the finding. Both were about a
program meeting a program it did not know: a person who undoes and does
differently, and a file written by a newer build. In both cases the answer
was the same — **do not throw the old thing away**. The history keeps the
branch; the document keeps the fields.

That is the third time a port has found the same kind of gap, and the three
were all found by pointing the kit at somebody else's application:

| the port | what it found |
|---|---|
| the vector application | a node needs a **payload** |
| the sound application | a node may have **no place**, and the document needs **relations** |
| the compositing application | the history needs to **branch** |

None of them asked for a bigger core. Each asked for the core to stop
discarding something, and each of those three somethings turned out to be a
seam that the next domain needed as well.

---

## 25. The second payload field: a node has a dictionary and a stream

§23 named three gaps. §24 closed two of them. The third was **the second
payload field**: a document that carries a lot of bytes.

A node has a **dictionary** — its payload — and it may also have a
**stream**: bytes that belong to the node and are not JSON. That is the
shape of a PDF object, and of most of the formats that grew out of
PostScript.

### Where the bytes go

They do **not** go in the document. JSON cannot hold arbitrary bytes, and
the answer that was there before — base64 in the payload — works and costs
a **third more space**: a document that carries fifty megabytes of streams
carries sixty-seven megabytes of text, and every reader has to parse the
text.

So the bytes travel **beside** the document, in the bundle, and the node
says which file is its stream:

```json
{
  "id": 1,
  "name": "page one",
  "kind": "pdf.page",
  "data": { "media_box": [0, 0, 595, 842] },
  "stream": "streams/page-1.bin"
}
```

The document stays small, the bytes come back **byte for byte**, and a
program can say what is missing before it starts editing: a bundle whose
document names a file it does not carry says so, streams included.

### What it cost

Less than the survey expected. §23 said the bigger answer "would cost a
field, a format version and base64 in the file anyway". It cost a field
and a format version — **and no base64 at all**, because the bundle was
already carrying files. The base64 door stays for the small things.

| | |
|---|---|
| the field | `Node.stream`: the file a node's bytes are in |
| the version | `craftkit/7`, and the step that gives every node the field |
| the commands | `node.stream.set`, `node.stream.clear`, `node.stream` |
| the bundle | says what is missing, streams included |
| the old writers | take the field off again: a file of the sixth version must not carry a field the sixth version does not know |

Eight tests, including one that says the document does not grow by the
size of its bytes, and one that says the bytes come back byte for byte.

---

## 26. A fourth survey: artcraft, the IDE for artists

The three surveys of §23 were of Rust applications. This one is not: it is
a **Tauri shell over a TypeScript and React monorepo** — a video editor, a
3D scene, a 2D canvas, and a feed of generations — with a Rust side of 67
Tauri commands, provider API clients and a SQLite task schema.

That makes it the most different domain yet, and it is worth asking what
it would take.

### What mapped, and what did not

| artcraft | the seam | verdict |
|---|---|---|
| scenes → tracks → elements | the tree of nodes | fits |
| the element variants as a closed union | the **open kind** | v2 wins: the closed union is the thing the open kind exists to avoid |
| `mediaId`, `objectUuid`, a clip bound to an object | the **relations** | fits exactly: these are edges, not containment |
| params, effects, masks, keyframes and easing | the node and document **payloads** | fits |
| the timeline range, the trim, the ripple | the **span selection** | fits |
| move, rotate, scale, the pose mode | the **tool in the hand** | fits |
| the linear undo of the 2D canvas | the **branching history** | satisfied, not strained |

So the document-shaped half — the video editor and the 2D canvas — maps
onto the tree, the relations, the payload and the span selection **without
a change**, and the 3D timeline's keyframes with their cubic-bezier easing
are a payload like any other.

### The three things that do not fit

**This list is history.** It is what the survey found on the day it was
written, and §33 is the port that came after the seams it asked for. Each
of the three is answered below, and the answers are in §33.

- **The 3D scene is a document that refuses to be one.** The engine owns
  the scene and saves it as **one opaque JSON string**, and the front end
  keeps a *view* of it. Making that a v2 document means re-expressing the
  engine's scene graph as nodes with contributed kinds, and giving the
  engine a way to build itself *from* the document instead of from its own
  JSON. That is a rewrite of the 3D half, not a seam change, and it is
  the largest single piece of work in the list. — **Answered in §33**: the
  kit does not have to model it. The scene is a node's **stream**, the
  bytes travel beside the document, and the engine takes them back.
- **A job that lives outside the process.** A generation is enqueued, gets
  a provider id, is polled, is written to SQLite so that it survives the
  app being closed, and is finished by an **event that arrives from
  outside**. The job pool is in-process: a queue, workers, a cancellation
  flag, results the caller collects. There is no durable handle and no
  completion from outside, and **that is a new seam, not a domain thing**.
  — **Built in §28**: a task with a durable handle, and §33 added the
  lookup by that handle.
- **Assets that are pointers, not files.** The media are cloud tokens and
  CDN URLs, resolved lazily and cached. A list of URLs in the asset list
  would be a small change, but the **bundle is simply wrong here**:
  bundling that media means downloading the library. Those assets belong in
  a payload under a namespaced key, not in the document's asset list. —
  **Answered in §33**: the tokens are in the payload and the bundle never
  sees them.

And one thing that is **not** a gap in the kit: the language. v2 is Rust
and wasm, and artcraft is a TypeScript application. That is a gap between
the kit and the port, and it is better said plainly than discovered later.

### What artcraft would not need

The file format machinery (it persists to a server, so there is no
versioned local file to bite on yet), the bundle, the real-time lane (its
must-finish work is a browser render loop), the branching history (its
histories are linear), the journal (it has no agent or macro surface), and
base64 in the payload (the bytes it has are media it does not own).

### The verdict, and the one thing to learn

The kit survives this domain **for the document-shaped half**, and the
closed unions in artcraft's timeline are an argument *for* the open kind.
It does not survive artcraft **as it is**, because artcraft's centre of
gravity is a cloud media library and a remote generation queue rather than
a document.

The one thing the kit would have to learn is that **a job may live outside
the process**: enqueued, persisted, polled, and finished by an event hours
later. If v2 grows a seam for that — a durable handle, a provider id, a
completion that arrives from outside — the rest of artcraft's shape
follows. Without it, artcraft is a consumer of v2's *front end* — the
registry, the commands, the plan — and not a document that lives in one.

**And then it grew it** (§28), and §33 is the port this section was
waiting for: the stream carried the opaque 3D scene, the task carried the
remote job, and the payload carried the media tokens. The verdict above
was true when it was written and it is **history**, not a rule.

### 25.1 The bundle the application writes

A stream needs a **producer**: without one, the name of a file is a name
and nothing else. The SDK could carry the bytes and the document could
name them, but no command wrote a bundle.

There are two commands now:

| | |
|---|---|
| `doc.save.bundle` | the document and the files it needs — the assets **and** the nodes' streams — in one file. A file that is not there is refused **by name**. |
| `doc.open.bundle` | reads one back, writes the files into the directory they are relative to, and puts the document in the session |

Three tests: the bytes come back byte for byte, a missing file is refused
by name, and the assets travel as well as the streams.

What is still not there is a bundle that a **plugin** fills with files
the document does not name. And the answer turned out to be a
**convention**, not a seam: the document's **asset list** is how a
document says what files it needs, so a plugin that points at a file
adds an asset. The sound battery does exactly that — `sound.source.add`
puts the file in the asset list as well as in the payload — and the
bundle then carries it.

That is worth saying plainly, because the first answer to this question
was "the kit needs a way for a plugin to declare its files", and the
honest answer is that the kit already had one: the asset list, and the
rule that a plugin that points at a file adds an asset. A seam that
would have said the same thing twice is worse than a convention that
says it once.

---

## 27. The third battery: the object model of a PDF document

Two ports so far: a vector application (§12) and a sound application
(§19). Both were *document* ports. This one is a **graph** port, and it
is the first whose document is not a tree at all.

### What was ported

A PDF file is not a tree. It is an **object graph**: a set of indirect
objects, each one a **dictionary** of key and value pairs, some of which
are **streams** of bytes, and some of which are **references** to other
objects. Two objects may refer to the same object, and an object may
refer to one that is defined *later* in the file.

What is ported is the **object model** (ISO 32000-2 §7.3) and not the
**file**: the cross-reference table, the filters, the incremental save
and the parser are the file, and a v2 document is not a PDF file.

| the object model | v2 |
|---|---|
| an indirect object | a **node**, of the kind `pdf.object` or `pdf.stream` |
| its dictionary | the node's **payload** |
| its stream | the node's **stream** — the bytes, beside the document (§25) |
| a reference to another object | a **relation** of the kind `ref` |
| the object number | the payload's `object`, because the ids are the core's |

### The three things the port found

- **An edge has to be able to say something.** The kind of a relation is
  the *name* of the edge, and two edges of the same kind say different
  things: a PDF's `/Resources 12 0 R` says which **key** the reference
  is under and which **generation** of the object it names. There was
  nowhere to put that — a plugin had to squeeze it into the kind, which
  made the kind mean two things at once. So the relation grew a
  **payload** (§339), and the reference puts the key and the generation
  in it.
- **A dictionary is ordered, and JSON is not.** A JSON object does not
  keep the order of its keys, and a PDF dictionary does: the port that
  reads and writes a PDF keeps the order so that a rewritten file is
  byte-stable and its diff is readable. So the dictionary is a **list of
  pairs** — `[["Type", "Page"], ["MediaBox", [0, 0, 595, 842]]]` — and
  the pairs are in the order the file had them. This one needed no core
  change: it is the payload's shape, and the payload is the plugin's.
- **A reference may point forward.** A relation names two nodes that are
  *in the document*, and a PDF may refer to an object that is defined
  later. So the graph is built in **two passes**: the objects first, and
  the references after. That is the price of the core's rule, and it is
  a plugin's business.

And one that is a fact rather than a finding: a PDF **name** and a PDF
**string** are *bytes*, and the payload is JSON. A name that is not text
is base64 in the payload — the door is the SDK's (§22) — and the port
says so rather than pretending otherwise.

### What it says about the kit

The port cost **one core change** — the relation's payload — and the
other two findings were answered by the seams that were already there:
the payload for the dictionary's order, and the two passes for the
forward reference.

That is a better result than the sound port had, and it is the fourth
port to say the same thing: the seams carry a *document*, and what each
port finds is a seam that says too little rather than a seam that is
missing.

| the port | what it found | what it cost |
|---|---|---|
| the vector application | a node needs a **payload** | a field |
| the sound application | a node may have **no place**; the document needs **relations** | two fields, a version |
| the compositing application | the history needs to **branch** | a tree instead of a stack |
| the PDF application | an edge has to be able to **say** something | a field, a version |

The file format is now eight versions long, and every one of them is a
field that some port needed.

---

## 28. A task may outlive the process

The job pool (§21.1) is for work that runs **here**: a queue, workers, a
cancellation flag, and results the caller collects when the work is
done. Everything about it is in-process, and the work is over when the
worker returns.

Not every job is like that. A generation is enqueued with a provider,
gets the provider's own id, is polled, is written to a database so that
it survives the application being closed, and is finished by an **event
that arrives from outside** — hours later, in a process that has nothing
to do with the one that asked for it.

There was no way to say that with a pool: the pool owns its work.

### What a task is

A **task** is work with a **durable handle**:

| | |
|---|---|
| it is registered here | and it may be registered by a process that is **not** the one that finishes it |
| it has a state | pending, running, done, failed, cancelled — and the state moves in one direction |
| it may carry a handle | the other end's own id: the provider's job id, the thing to ask about it with |
| it is durable | the registry is written down and read back, so the list survives the process that made it |

The rule that makes it worth having: **whoever gets there first finishes
it**. This process may not have started the task at all.

### The commands

`task.add`, `task.list`, `task.start`, `task.attach`, `task.progress`,
`task.complete`, `task.fail`, `task.cancel`, `task.save`, `task.load`.

The registry is the session's own state, the way the selection and the
tool are: a task is something the program is doing, not something the
document holds. A host that wants the tasks to travel with the document
writes them into the payload — the registry is JSON, and reading it back
is a command.

### The test that matters

A task is started, written down, and the process ends. Another program
reads the list back, and **finishes the task from outside**. That is
the whole point, and it is a test rather than a claim.

### What this says about the kit

The survey of the artist's IDE called this "a new seam, and not a domain
thing", and it was right. The seam is not about generations or about
providers: it is about **work whose lifetime is not the program's**.

It also says something about the job pool: the two are not rivals. A
pool is for work that finishes here, and a task is for work that may not.
A host that wants both has both, and the rule that keeps them apart is
the same rule the lane and the pool have always had: the lane is for
work that must finish, the pool is for work that may take a while, and a
task is for work that may not finish at all.

---

## 29. One camera, with its own caps

Every application with a canvas has a camera: a zoom, a scroll, and the
rules that keep them sane. Without one, each canvas invents its own, and
two canvases in the same program scroll differently, zoom differently,
and fight over the wheel.

The tree beside this one has a rule about it — **one set of camera
controls**, one module that owns them, with its own caps, and every
canvas goes through that module — and v2 had no camera at all, so the
rule had nothing to hold. This is the seam that gives the rule
something to hold.

### What a camera is

A **zoom** and an **offset**, and two ways of saying where a place is:

```rust
let screen = camera.to_screen(world_point);   // where to draw it
let world  = camera.to_world(screen_point);   // where the click was
let visible = camera.visible(half_width, half_height);
```

That is all. What is selected, what is hovered and what the panels show
are the front end's business.

### The rules

- The zoom is **capped** — between a hundredth and a hundred times — and
  a zoom outside that is **clamped**, not refused. A wheel event can
  send anything, and a camera that refuses to move feels broken.
- The offset is **not** capped: a document may be scrolled anywhere, and
  the cap that matters is the zoom's.
- The camera is **not** part of what the undo restores. Scrolling is not
  an edit, and an undo that scrolls the view is a surprise. It is the
  session's state, the way the selection and the tool are — just not
  the undoable part of it.

### What it needed next to it

`Rect::intersects`, which the camera wants and every front end that
draws a thousand nodes wants: the ones that are off screen are not
drawn. Touching is not overlapping — two rectangles that share an edge
have shared no area, and a front end that drew both of them has drawn a
seam rather than an overlap.

### What this says about the kit

This is the smallest seam in the tree and the one every domain with a
picture needs, and it took nothing from the core: a camera is a value,
and the SDK hands it to whoever draws. The tree's rule — one set of
camera controls — is now a rule the code can keep, and not only a rule
the README can state.

---

## 30. The fourth battery: the document model of a video editor

Three ports so far, and each one has found the same kind of thing. This
one is a **video editor**, and its project is three things at once.

### What was ported

| | |
|---|---|
| a tree of **bins** | the folders the editor shows, each holding items and other bins |
| a flat **registry of items** | the media, the sequences, the graphics — each with its own id, and each placed in a bin |
| a **timeline** | sequences hold tracks, tracks hold clips, and a clip says which item it plays, where it starts, how long it is, and where in the media it begins |

That last part is the **two clocks**: a clip has a place on the timeline
and a place in the media, and they are not the same place.

| the editor | v2 |
|---|---|
| a bin | a **node** of the kind `film.bin` — a group |
| an item | a **node** of the kind `film.item`, in a bin |
| a track | a **node** of the kind `film.track`, in a sequence |
| a clip | a **node** of the kind `film.clip`, in a track |
| the item a clip plays | a **relation** of the kind `clip` |
| the item's own id | the payload's `item`, because the ids are the core's |

### The three things the port found

- **The tree gives a node one parent, and an editor's bins can hold the
  same item twice.** The item lives in one bin, and a second bin that
  holds it points at it with a relation. The core's rule — a node has
  one parent — is the right one, and the editor's flat registry is what
  has to give way. This is the same answer the PDF port gave for its
  object graph, and it is now the second port to answer it the same way.
- **The order of a timeline is not the order of the tree.** The children
  of a track are in the order they were made, and the timeline is in the
  order they *start*. So the order that matters is **derived**, and the
  evaluator derives it. The tree keeps the order of history; the
  timeline is the order of time.
- **A sequence's duration is not stored.** It is the end of the last
  clip on its tracks, and it changes when a clip moves. So it is derived
  too, and nobody writes it down.

### The table so far

| the port | what it found | what it cost |
|---|---|---|
| the vector application | a node needs a **payload** | a field |
| the sound application | a node may have **no place**; the document needs **relations** | two fields, a version |
| the compositing application | the history needs to **branch** | a tree instead of a stack |
| the PDF application | an edge has to be able to **say** something | a field, a version |
| the video editor | the order and the length of a timeline are **derived** | nothing |

Five ports. Four of them cost a field, a version, or a shape — and the
fifth cost **nothing at all**, because the seams that the first four had
already taught the kit were enough. The cost is falling, and that is the
finding of the fifth port: the kit is not a list of missing features any
more, it is a set of seams that a new domain arrives at and finds
already there.

---

## 31. The fifth battery: a compositing application

Four ports, and each one has found the same kind of thing. This one is a
**compositing application**, and its document is the strangest of the
five.

### What was ported

A composition is a stack of **layers**, and every layer has a **tree of
properties**: a group holds groups and properties, a property has a
value, and the value may be **animated** over time or computed by an
**expression**.

What is ported is that model, and the **paths** that address into it:

```text
transform/position
effects/#1/blurriness
contents/rect#2/size
```

| the application | v2 |
|---|---|
| a composition | a **node** of the kind `fx.comp` |
| a layer | a **node** of the kind `fx.layer`, in the comp |
| a layer's property tree | the node's **payload** |
| a property's value, its keyframes, its expression | the **payload** |
| the path that addresses a property | the **plugin's own** |

### The three things the port found

- **A path is not a path of node ids.** The core addresses a node by its
  id, and an application addresses a property by *name and order*: "the
  third `effects` group, its `blurriness` property". That path survives
  a rename and a reorder, which is why it is the way an application
  addresses its own properties. And it is **the plugin's**: the core
  does not know what a match id is, and a path type in the core would
  have to know. So the path type is in the plugin, with the rest of it.
- **A property tree is not a tree of nodes.** A layer has fifty
  properties and a comp has thirty layers: a thousand nodes for one
  picture, and the node graph would be unreadable. So the tree goes in
  the **payload**, and the paths address into it. This is what the
  payload is for, and the fifth port is the second one to say so.
- **The value at a time is derived.** A property's value at frame
  forty-eight is the interpolation of its keyframes, and it changes when
  a keyframe moves. So the **evaluator** says it, and nobody writes it
  down.

### The table, with the fifth row

| the port | what it found | what it cost |
|---|---|---|
| the vector application | a node needs a **payload** | a field |
| the sound application | a node may have **no place**; the document needs **relations** | two fields, a version |
| the compositing application | the history needs to **branch** | a tree instead of a stack |
| the PDF application | an edge has to be able to **say** something | a field, a version |
| the video editor | the order and the length of a timeline are **derived** | nothing |
| the compositing application (again) | a **path** is not a path of ids; a property tree is not a tree of nodes | nothing |

Five ports, and **two of them cost nothing at all**. That is the
finding of the fifth: the kit is not a list of missing features any more.
It is a set of seams that a new domain arrives at and finds already
there — the payload for what is data, the tree for what contains what,
the relations for what is attached to what, the evaluator for what is
derived, and the core's silence about what a domain calls its things.

What the sixth port would have to do to cost nothing is the question
that is left.

---

## 32. One piece of `unsafe`, and the lane's rule measured

**And then there were three** (§34): the loader and a shared library's
entry point joined the allocator on **the list**. The rule below did not
survive as "one piece of `unsafe`"; it survived as **the shape** — a
list, a reason beside each name, and a gate that reads the tree.

The lane has one rule: **its work must not allocate**. `run` does bounded
work in bounded time, and an allocation inside it is a lock inside it — a
lock on the lane, from the thread that is supposed to never wait. The rule
was written down early, and for a long time it was a rule and not a
measurement, because measuring it needs a **counting global allocator**, and
a global allocator is `unsafe` by definition. v2 had no `unsafe` at all.

That was a good default and it cost a measurement. This section is the
trade, made once, on purpose.

### What was allowed, exactly

One new crate, `crates/craftkit-alloc`, and it holds **one** `unsafe impl`:

```rust
#![deny(unsafe_code)]   // **not** `forbid`

#[allow(unsafe_code)]
unsafe impl GlobalAlloc for Counting { /* counts, then defers to System */ }
```

The four rules that hold it there:

- **one crate** does this, and the crate does one thing;
- the crate root says **`deny`**, not `forbid` — because `forbid` cannot be
  relaxed by an `allow`, so the exception could not live in the crate at
  all. `deny` is the backstop; every other crate in the tree still says
  `forbid(unsafe_code)`;
- the counts are **per thread**, in a thread-local with a `const`
  initialiser, so reading them inside the allocator cannot itself
  allocate — which would be a very funny way to count allocations;
- a **gate test** in that crate reads the whole workspace and refuses: a
  second `unsafe` of any kind, a second `allow(unsafe_code)`, an `unsafe`
  block outside the allocator, or a crate root that swapped `forbid` for
  `deny` in the wrong place. The exception is therefore not a promise in
  prose; it is a **mechanical** fact that a test re-checks.

This is the same shape as the outer tree's own one allowed exception, and
it was made because the owner asked for exactly this: *if v2 forbids it,
make it allow it in a very constrained way so that it is manageable*.

### What the measurement says

The studio's test binary installs the allocator and runs a **real** lane
job on a **real** lane thread. The job counts its own allocations before
and after its work, and the test asserts the difference is **zero**.

So the claim is no longer "the lane does not allocate", it is "the lane's
work allocated zero times, measured on the thread that ran it". The
control thread's allocations are not counted against it, because they are
not the lane's — which is the whole reason the counters are per thread.

### What this says about the kit

A rule that cannot be measured is a rule that is only believed. The tree
had one, and the price of keeping the tree `unsafe`-free was that the rule
stayed unmeasured.

The way out was not to argue about `unsafe`, but to make the exception
**small, named, and mechanically checked**: one crate, one impl, one
`allow`, and a test that reads the tree and refuses a second one. That is
the same discipline the outer tree uses for its own single allowed
`unsafe`, and it is now available here for the next thing that needs it —
the CLAP, VST3 and AU plugin formats, which are FFI, and FFI is unsafe.
When that port comes, it should land in its own crate under the same gate,
not in the middle of the SDK.

---

## 33. The sixth battery: an artist's IDE, and the port that was waiting

§26 surveyed the last tree in the family and gave it a verdict: the kit
survives this domain **for the document-shaped half**, and it does not
survive artcraft **as it is**, because artcraft's centre of gravity is a
cloud media library and a remote generation queue rather than a document.

That verdict was written before three things existed. The **stream**
(§25) carries bytes beside the document, the **task** (§28) is work with
a durable handle, and the **payload** (§25) holds what a domain calls
its things. So §26's three objections have answers now, and this section
is the sixth battery: `craftkit-plugin-art`.

The port is of artcraft's own three models — a **video editor**, a **3D
stage** and a **generation queue** — and it is honest about which half of
each one is in.

### What was ported

| artcraft | v2 |
|---|---|
| a project | a **node** of the kind `art.project` |
| a scene | a **node** of the kind `art.scene` |
| a track: `video`, `text`, `audio`, `graphic`, `effect` | **five kinds**, `art.track.video` and its four siblings |
| an element: `video`, `image`, `audio`, `text`, `sticker`, `graphic`, `effect` | **seven kinds**, `art.element.video` and its six siblings |
| the media an element plays | the element's **payload**: a **token**, not a file |
| the params, the effects, the masks, the keyframes | the element's **payload** |
| the 3D scene, one opaque JSON string | a **stream**: one file beside the document |
| a generation, with a provider's job id | a **task**, and the job id is its **handle** |

### The three things the port found

- **A closed union becomes kinds, and the core learns nothing.** The
  application has five kinds of track and seven kinds of element inside
  TypeScript unions that a new kind would have to edit. Here each one is
  a **kind**, and v2's kinds are open strings: a new kind of element is
  another plugin's kind, and nothing in this crate and nothing in the
  core changes. The price was **twelve commands** instead of one, and
  the twelve are twelve lines of macro each.
- **A document may carry a thing it does not model.** The 3D scene is the
  engine's own JSON, and modelling it would be a rewrite of the engine's
  half — the objection §26 raised. The kit does not have to model it: a
  node may have a **stream**, the bytes travel beside the document in the
  bundle, and the editor hands them back to the engine untouched. So the
  largest single piece of work in the family turned out to be **a file
  name**.
- **A job that lives outside the process is a task, and the outside
  world's name for it is a handle.** §26 said this was *the one thing the
  kit would have to learn*, and §28 built it. What §26 did not say is
  that the provider's job id comes back **by name** when the job is
  done, so the task has to be findable by that name — and the registry
  only found tasks by the registry's own number. That was **one method**
  in the SDK (`find_by_handle`, and `complete_by_handle` and
  `fail_by_handle` beside it), and it is the first thing a port has cost
  the core.

### And the one that is not the port's

The **media** are the interesting non-finding. The application holds
media as **tokens** — `m_1`, `jinf_7` — and resolves them lazily against
a server. So they are **not** assets: the asset list is for files the
document owns, and a token is a pointer into somebody else's library.
A bundle that tried to collect the media would download the library. So
the tokens live in the payload, under a name of the application's own,
and the bundle never sees them. That was §26's second objection, and it
cost nothing — but it is a **rule**, and the rule is worth writing down:
*the asset list is for bytes the document owns; a pointer is a pointer.*

### The table, with the sixth row

| the port | what it found | what it cost |
|---|---|---|
| the vector application | a node needs a **payload** | a field |
| the sound application | a node may have **no place**; the document needs **relations** | two fields, a version |
| the compositing application | the history needs to **branch** | a tree instead of a stack |
| the PDF application | an edge has to be able to **say** something | a field, a version |
| the video editor | the order and the length of a timeline are **derived** | nothing |
| the compositing application (again) | a **path** is not a path of ids; a property tree is not a tree of nodes | nothing |
| the artist's IDE | a closed union is **kinds**; a document may carry a thing it does not model; a task must be findable by its **handle** | one method |

Six ports. Two of them cost nothing at all, and the sixth cost **one
method in the SDK** — the smallest thing a port has ever cost, and the
first time one of them touched the core.

§31 asked what a sixth port would have to do to cost nothing. The answer
turns out to be: arrive with a **closed union** and leave with kinds,
arrive with an **opaque blob** and leave with a stream, and arrive with a
**job id from outside** and find that the kit can already hold it. The
kit did not grow a seam for this domain. It grew one method, in the
place where the outside world's name for a piece of work had to be
understood.

---

## 34. The door: a plugin that is not part of the program

§32 made a rule and broke it on purpose: one crate, one `unsafe impl`,
one `allow`, and a gate that holds the tree to it. That was the **first**
exception. This section is the **second** and the **third**, and it is
what a real plugin format needs.

### What the formats are, and what the door is

CLAP, VST3 and AU are not document models. They are **binary
contracts**: a C or C++ API with a fixed layout, a factory function, and
a set of calls the host makes. Porting one of them is not a port; it is
an implementation of somebody else's ABI, in a language this kit does
not speak, and it is large.

What was built is the **door** the formats would come through:

- `crates/craftkit-ffi-abi` — **the contract**: a table of function
  pointers with a version at the front. No dependencies at all, because a
  contract that depends on nothing is a contract that cannot move.
- `crates/craftkit-ffi` — **the host**: opens a shared library with
  `dlopen`, finds the entry point **by name** with `dlsym`, checks the
  version, and calls the table.
- `crates/craftkit-plugin-shared` — **a plugin that is not part of the
  program**: a `cdylib` that exports `craftkit_plugin_vtable` and does
  not link into the host at all.

A CLAP plugin would arrive as a **shim**: a small piece of C++ that
exposes this table. Nothing above the door would know the difference, and
the door is the same one the outer tree's Android package uses — a shared
library, an entry point, looked up by name.

### What the port found

- **The entry point cannot be the table.** The host does not know where
  in the library the table is, so the thing it finds by name has to be a
  **function** that gives the address. The first attempt made the entry
  point the table itself, and the host read a version of 822447432 out
  of a function pointer: the test caught it in one run.
- **A version is the whole of the safety here.** The host cannot know
  what is in a library it has not read, so the table says which contract
  it speaks, and the host **refuses** a table it does not understand
  rather than calling it and hoping. A plugin built against another
  version is not called; the numbers would mean different things.
- **The unsafe is one module.** Every foreign call in the loader —
  `dlopen`, `dlsym`, `dlclose`, `dlerror`, and reading a C string the
  plugin gave back — lives in **one module** with **one**
  `allow(unsafe_code)`. The rest of the crate is safe Rust that calls
  into it. The plugin's side is one `#[unsafe(no_mangle)]`, because a
  name that is found by name cannot be mangled.

### The list, and what it cost

The gate in `craftkit-alloc` used to hold the tree to **one** exception.
It is now a **list**, and the list is the whole of the exceptions:

| the crate | why it may hold `unsafe` |
|---|---|
| `craftkit-alloc` | a counting global allocator is unsafe by definition |
| `craftkit-ffi` | opening a library and calling into it |
| `craftkit-plugin-shared` | a shared library's entry point, found by name |

And the rules around it are mechanical: `unsafe` appears **only** in
those three crates; each of them says `deny(unsafe_code)` and **not**
`forbid`; each has **exactly one** `allow(unsafe_code)`, so an exception
is one marked place and not a scattering; the only `unsafe impl` in the
tree is the allocator's; and every other crate still forbids unsafe
outright.

So §32's rule did not survive as "one piece of `unsafe`". It survived as
**the shape**: a list, a reason beside each name, and a gate that reads
the whole tree. That is the outer tree's own pattern — one list of
allowed exceptions, and a gate that holds the tree to it — and it is now
in v2 as well.

### What is still not built

The **formats** are not built. (The door is a **command** now: §35.) There is no CLAP, no VST3 and no AU here:
each is a large ABI in another language, and each would be a **shim**
over this door, not a change to it. What is built is the door, the
contract, the version check, and the proof that a plugin in another
shared library can be reached from a program that has never seen it.

---

## 35. The door is a command

§34 built the door as a **library**: a host that opens a shared library
and calls the table in it. A library that only a Rust program can reach
is not a door; it is a function. The tree's rule is that everything a
person, an agent or a script can do is a **command**, and the door was
not one, so it became one.

### What was added

Three commands, in the SDK, next to the task commands:

- `plugin.library.open` — open a shared library, and give back its
  number;
- `plugin.library.list` — the plugins in the open libraries, with their
  names and their parameters;
- `plugin.library.close` — close one.

And the **session** holds the open libraries, the way it holds the
tasks and the camera: they are **session state**, not document state. A
library is a program the session has open, not a file the document owns,
so the undo does not close it and the bundle does not carry it.

### The three things this found

- **A handle that is a pointer makes the session un-`Send`.** The first
  version of the loader kept the operating system's handle as a raw
  pointer, and `Arc<Library>` was neither `Send` nor `Sync` — a warning
  in the SDK, and a session that could not cross a thread. The fix was
  not an `unsafe impl Send` (that would be a second exception in a
  crate that has one) but to keep the handle as an **address**: the
  loader's handle is opaque, and a number is as good a name for it as a
  pointer. The library became ordinary session state.
- **A library that is not a plugin fails at the right place.** Opening
  `libm.so.6` succeeds — it is a shared object — and the failure comes
  when the host looks for the entry point and does not find it. The
  refusal names the symbol it wanted, which is the thing the caller can
  act on.
- **The undo has nothing to undo, and that is correct.** Opening a
  library does not change the document, so there is no state to go back
  to, and the undo says so instead of pretending. A test says it: the
  library is still open after the undo refused.

### What this is not

The door is **query-only**: a plugin says what it is and what its
parameters are, and the host reads them. A format's ABI has more in it
than that — it has a way to create, activate and process — and a real
plugin would need the table to grow. That is a change to the **contract**
(`craftkit-ffi-abi`), which is exactly what a version number is for: the
host refuses a table it does not understand, so a table can grow and the
old host will not call the new one.

---

## 36. The editing half: a trim, a retime and a ripple

§33 ported the artist's IDE as a document. A document you cannot edit is
a picture of an editor, so this section ports the **editing**: the three
operations that are about a **range** and not about a node.

| the operation | what it does |
|---|---|
| `art.element.trim` | the element keeps its place and plays **less of its media** |
| `art.element.retime` | the media plays **faster**, and the element takes less room |
| `art.ripple` | the space one element took is given to the ones **after it** |

### The three things this found

- **The length of an element is derived, and the application stores it.**
  The application keeps three numbers — the length on the timeline, the
  span of the media, and the rate — and the first follows from the other
  two. It stores it anyway, because it is an editor and a stale number
  is one keystroke away. The kit **derives** it: the payload holds the
  span and the rate, and the evaluator works out the length. So a trim
  or a retime cannot leave a number behind that says something else.
  This is the sixth time a port has found a stored value that should
  have been derived, and the sixth time the kit's answer was the same.
- **The ripple does not cross into another track, and that is the
  application's rule.** A ripple takes the space out of one element and
  gives it to the ones after it **on that track**: the other tracks do
  not move, and the rule is not a default that could be changed — it is
  the only behaviour. So the command takes a **track**, and the core
  knows nothing about ripples.
- **The rate is clamped, not refused.** The application's bounds are a
  hundredth and five. A rate outside them is **clamped**, because the
  application wants the edit to happen and the bound to be respected;
  the kit's rule is the opposite — a hostile number is refused — so the
  bounds are **constants of the plugin** and the clamping happens
  there, where the domain's rules live.

### And the one about ranges

A trim is about a **range**, and the kit has a span selection (§22). But
the art battery's trim does **not** use it: the command names the
element **and** the range, in one call. The sound battery's trim works
on the *selection* instead. Two domains, two shapes, and the kit is
fine with both — because a command's parameters are typed, and what a
range means is the domain's business. The core's span selection is
there for the commands that act on "whatever is picked out", and a trim
that names its target does not need one.

---

## 37. The view state, and the end of §20's list

§20 listed six things the sound tree had and v2 did not. Five of them
were answered by later sections: the real-time door (§21.1), the media
that travel (§21.2), the span selection (§22), the plugin formats
(§34's door), and the menu (the registry generates it, so there is no
catalog to drift). The sixth was the **view state**, and this section is
it.

### What was built

Two commands:

- `view.save` — write the camera into the document, under the SDK's own
  key, `craftkit/view`;
- `view.restore` — read it back and move the camera.

### The rules, and why they are two commands

- **The camera is the session's state.** It is where the person is
  looking, it has its own caps, and the undo does not move it.
- **The document may remember the view.** The artist's IDE keeps the
  viewport in the document (§33), and it is right to: a board is a thing
  you come back to.
- **Saving the view does not move the camera, and restoring it does not
  change the document.** They are two commands because they are two
  different things: one writes a fact down, the other moves the eye.
  The undo takes the **saved** view back — saving changes the document —
  and it does not take the **camera** back, because the camera is not
  the document's.
- **The key is the SDK's own.** The document's payload is shared, and two
  plugins must not collide by accident, so the SDK writes under one
  namespaced name and a plugin that keeps a view of its own writes under
  a name of its own.

### §20's list, closed

| the stressor | answered |
|---|---|
| real time | §21.1 — the lane, its own rules, its own tests |
| media that travel | §21.2 — the assets and the bundle |
| a selection of spans | §22 — the selection holds parts |
| view state | **this section** — the camera, and the view written down |
| the plugin formats | §34 — the door; the formats themselves are not built |
| the menu catalog | §20.3 — the registry generates the menu, so there is no catalog |

Six stressors, six answers, and the kit grew a seam for each one rather
than a feature for each one. That is the whole claim of this tree, and
after six ports and six batteries it is a claim with a number against
it: the ports cost **a field**, **two fields and a version**, **a tree
instead of a stack**, **a field and a version**, **nothing**, **nothing**
and **one method**.

---

## 38. The guard: a contribution that panics does not take the program with it

§16 listed the **kernel** as the thing v2 did not have, and part of that
list was wrong: v2 had a **panic guard** for commands and none for
**evaluation**. That is the gap this section closes, and it is the same
gap the outer tree's `craftkit_kernel::guard` exists for.

### What was there and what was not

- A **command** that panics was already handled: `Session::execute`
  clones the document, runs the command under the guard, and on a panic
  **restores the pre-command document**. A command that panics is a
  command that did not happen.
- A **kind** or an **evaluator** that panics was **not** handled: the
  panic unwound through `Session::evaluate` and into whatever was
  drawing.

And a kind and an evaluator are **contributions**: code this tree does
not own. A plugin's kind that panics is a plugin's bug, and it must not
be the program's crash.

### What is there now

Every evaluation is under the guard, and the **plan says what it could
not say**:

```text
Plan { items: [...], failures: [PlanFailure { who, message }] }
```

- a kind that panics is **skipped**, and the plan carries a failure
  naming the kind and what it said;
- an evaluator that panics is **skipped**, and the plan carries a
  failure naming it;
- the failures are in the plan's **JSON** as well, so a front end can
  show them rather than draw a document that is quietly missing a piece.

A plan with a failure in it is a plan that is **short**, and it says so.
That is the difference between a guard and a `catch_unwind` that throws
the panic away: the program keeps running, the plan says what it could
not say, and the failure is **visible** instead of silent.

### What is still not built

The **numeric core**: there is no type for a length in millimetres or an
angle in radians, and every number in a document is a JSON number. The
guard is the machinery around that gap, and the gap is still there.

---

## 39. The kernel: what six ports say about the typed value

§16's first item was the **kernel**, and the honest reading of it is: no
**numeric core**. There is no type for a length in millimetres or an
angle in radians, and every number in a document is a JSON number.

Six ports have now gone through the kit, and they are the evidence.

| the port | the number it needed | where it lives |
|---|---|---|
| the video editor | a length in **frames** | the clip's payload |
| the sound editor | a length in **samples** | the clip's payload |
| the artist's IDE | a length in **ticks**, and a tick rate | the element's payload, and the project's |
| the compositing application | a path of **names and orders** | the layer's payload |
| the PDF application | an **object number** | the reference's payload |
| the vector application | a **point** in the world's units | the node's rectangle and payload |

Five numeric domains, and the core has **no opinion** about any of them:
the number is in the payload, the unit is the kind's, and the core
carries it without reading it. That is not an accident — it is the same
bargain the payload makes, one level down, and it is why six ports cost
six small things rather than six redesigns.

### So what would a numeric core buy, and what would it cost?

**Buy:** a number that cannot be confused with a number of another kind.
A length cannot be added to an angle, and a millimetre cannot be read as
a pixel. That is real, and it is why the old tree has a kernel.

**Cost:** every number in the **format** would have to carry its unit,
every plugin's payload would have to be typed, and every JSON value in
the plan would have to become a typed value. The `.craftkit` file, the
payload, the journal and the plan are all JSON today, and they are JSON
because they are **portable** — a document is a text file that anybody
can read.

### And the conclusion is not "no"

The conclusion is that a numeric core is **a plugin**, not a change to
the core. The seam is already there and it is exactly the right shape:

- a **kind** carries its own evaluation and returns a value;
- a **value** is JSON, so the format and the plan do not move;
- the **guard** (§38) is around every evaluation, so a numeric core
  that panics on a bad input is caught like any other contribution.

So a kernel would arrive as `craftkit-plugin-geometry` or
`craftkit-plugin-units`: kinds whose values carry units, a `Length` and
an `Angle` in the payload, and the core none the wiser. The kit would
not need a **seam** for it, because the seam is the seam a kind already
uses.

That is the last of §16's list answered — not by building the kernel,
but by showing where it goes when it is built, and what it costs to put
it anywhere else.

---

## 40. Four more trees: the family built the kit three times

Four more trees arrived, and they are the **same family** as the ones
already surveyed: DesignCraft (page layout and publishing), GridCraft
(spreadsheets), WordCraft (a word processor), and **craft-libs**, the
shared library battery all three of them depend on.

They are **Rust**, which artcraft was not, and that matters: this is
the first time the family could have built craftkit's architecture and
did not.

A fifth tree arrived in the middle of the session — **CADCraft**, the
CAD application — and it is surveyed in §43, because it is the same
domain this kit was written for and it deserves its own section.

### craft-libs: a battery, and not a kit

Twenty-six crates, all named `craft-<name>`, and one rule: *"No app
code. A crate here must be useful to more than one app and must not
depend on any app."*

What is in it: colour management (ICC profiles, transforms, gamut),
text shaping (hyphenation in forty-eight languages, UAX #14 line
breaks, UAX #9 bidi), crash-safe file writing, path safety for paths
stored in a document, fonts end to end, MP4/MOV/Matroska demuxing, a
clean-room JPEG decoder, PDF read and write, prepress and ink plates,
the OS shell (windows, menus, dialogs, printing, updates) and an egui
widget library with a docking workspace.

What is **not** in it: no command registry, no document model, no
evaluator, no plugin loader, no MCP layer, no `unsafe` at all (the
workspace forbids it, and every FFI call is delegated to a safe wrapper
in a third-party crate).

The one thing that touches a kit's ideas is `craft-keymap`'s
`CommandRef { id, params }`: a command's identity is a dotted id and a
blob of JSON, and the key is canonicalised. That is craftkit's
convention, arrived at in a crate that was not written for a kit.

### The three applications

Each one has the same crates in the same places: `doc` (the document),
`engine` (the commands), `mcp` (the agent door), `ui-egui`, a CLI and a
web build. And each one built the same architecture.

| | DesignCraft | GridCraft | WordCraft | CADCraft | craftkit-v2 |
|---|---|---|---|---|---|
| the registry | `CommandSpec` + a `cmd!` macro | `CommandSpec` + a `cmd!` macro | `CommandSpec::new` + a builder | `CommandSpec` + const builders | `CommandSpec` + `#[derive(Command)]` |
| how many commands | 542 | 363 | 632 | ~440 | 25, plus the batteries |
| the one door | `Session::execute(id, json)` | `Session::execute(id, json)` | `Session::run(id, &json)` | `session.execute(id, params)` |
| the panic guard | yes, and it restores the document | yes | yes | yes (§38 covers the evaluation too) |
| the parameters | a **doc string**, parsed by hand | a doc string | a JSON-schema-ish string | **typed** `ParamDoc`s |
| the MCP surface | 28 hand-written tools | 22 hand-written tools | ~17 hand-written tools | **one tool per command** |
| a plugin seam | **none** | **none** | **none** | link-time registration |
| the document's version | a ZIP with `meta.json` | **none at all** | **none at all** | `craftkit/N` + pending migrations |
| the document | spreads, items, stories, typed id fields | a **grid** of cells | a flat list of blocks and a side-vector | a tree of nodes, relations, payloads |
| a document-scoped plan | **none** | the calc graph | the layout cache | **the plan** |

DesignCraft says the rest of it in its own gap list, in one line:
*"Missing: any plug-in model… Owner decision; not estimated."*
WordCraft says it in a different way: `Block` and `InlineObject` are
**closed enums**, matched exhaustively in the document, the layout, the
docx reader and writer, the format converters and the UI — so a new
kind of content is a change in six crates at once.

### What the survey found

- **The architecture is the family's, and it was written four times.**
  A command registry, one `Session::execute`, JSON parameters, a panic
  guard that restores the document, an undo snapshot, a journal, and an
  MCP façade over the registry. Three trees, independently, with the same
  names for the same things. craftkit-v2 is not proposing something
  new: it is proposing to **share** something three times copied.
- **What the three trees lack is exactly what this kit has.** No plugin
  seam in any of them; no versioned document in two of them; typed
  relations replaced by hand-written id fields (`Item.story`,
  `Page.parent`, `Story.frames`) or by closed enums; no document-scoped
  plan, so every consumer of the document — `render`, `pdf`, `epub`,
  `pptx` in DesignCraft — walks it again with its own rules and can
  disagree with the others.
- **The parameters are a string in all three.** DesignCraft parses the
  `{…}` object out of the human-readable `params` text to reject
  misspelt keys for out-of-app callers; the other two do the same. This
  kit's parameters are **typed** (`ParamDoc { name, kind, required,
  help }`), so an agent that reads `describe` gets the types, and a
  hostile caller gets a typed failure instead of a string mismatch.
- **The MCP surface is a second vocabulary.** Seventeen to twenty-eight
  hand-written tools over three hundred and sixty-three to six hundred
  and thirty-two commands means an agent must learn the façade *and* the
  ids. This kit emits **one tool per command**, so the registry **is**
  the protocol and there is nothing to learn twice.
- **Where the derived value lives, three answers.** GridCraft writes a
  formula's result back **into the cell** — a stored derived value, one
  keystroke away from being wrong. WordCraft computes the layout and
  never stores it, caching it by a revision counter. DesignCraft
  composes per story, cached by the story's revision and the identity
  of the styles it used. None of them is "the plan", and the plan is
  what would let a plugin's evaluator and an application's renderer
  agree about what a document means.
- **Copy-on-write is how all three make undo cheap.** They share
  immutable `Arc` subtrees and snapshot the document by pointer. This
  kit's history **clones** the document, which is simple and correct and
  more expensive than it needs to be.
- **The spreadsheet is the hardest port yet.** A grid is not a tree at
  all, and its dependency graph is between **areas** — a cell depends on
  a *range* — which is the first relation kind this kit has not had.
  Its answers are worth having before the seventh battery is built.

### What the survey does not change

The seams, the six batteries, the list of allowed exceptions and the
rules of the tree are as they were. What changed is the **evidence**:
this kit is not a theory about how an application might be put together.
It is the third copy of a thing the family already builds, and the
question it answers is no longer "would this work?" but "what is the
cheapest way for four applications to stop copying it?".

And the answer is visible in the table: **the plugin seam**. It is the
one thing all three trees cannot do without a fork of their engine
crate, it is the one thing this kit has, and it is the only thing in the
list that is not a *shape* but a *door*.

---

## 41. The MCP door: one tool per command

§40 surveyed three applications in the family that all put an **MCP
façade** in front of their command registries: a handful of tools that
take a command id and a bag of parameters, seventeen to twenty-eight of
them over three hundred and sixty-three to six hundred and thirty-two
commands.

An agent that drives one of those servers has to learn **two** things: the
façade's vocabulary, and the command ids the façade forwards to. This
kit does not have a façade. It has the **registry**, and the registry is
the protocol.

### What was built

- `crates/craftkit-mcp` — the server: it takes a session and answers
  JSON-RPC 2.0 over newline-delimited stdio, which is what an MCP client
  expects and what the family's servers speak;
- `apps/craftkit-mcp-server` — the program: the studio's ecosystem with
  **all six batteries linked in**, one session, and the protocol on its
  standard output. Nothing else is ever printed there, because a log
  line on the protocol stream breaks the client.

### One tool per command

- the tool's **name** is the command's id, with the dots turned into
  underscores and a prefix: `doc.open` is `craftkit_doc_open`;
- the tool's **schema** is built from the command's **typed** parameters:
  `{"type": "string"}` for a text, `{"type": "integer", "minimum": 0}`
  for a node reference, and the command's own help as the description;
- the tool's **description** is the command's label and its own words;
- **calling the tool runs the command**, in the session, and returns what
  it said: the message, and the data as JSON.

So there is nothing to learn twice, and a command that is added is a tool
that is added. With the six batteries linked in, that is **106 tools** —
and a test says so, by counting the registry and the tool list and
checking that they are the same size.

### What the survey said about this, and what it cost

The survey's finding was that the three trees hide their registries
behind a second vocabulary, and that their parameters are a **human-
readable string** parsed by hand — DesignCraft parses the `{…}` object
out of the `params` text so that a misspelt key from an out-of-app caller
can be refused.

This kit's parameters were already typed (§ the SDK's `Params`), and the
schema above is that typing, written out. So the cost of the whole door
is **one crate and one app**, and the difference it makes is that an
agent reads the tool list and knows the kit's whole surface: what it can
do, what each thing takes, and what each thing is for.

### What is still not built

The **control channel** — the loopback socket the outer tree drives the
window with — is not here, and neither is a **toolbar**. The MCP server
is the door an agent uses; the other two are doors a person uses, and
they are the same kind of thing: a front end that reads the registry and
calls the commands.

---

## 42. The seventh battery: a spreadsheet, and a graph of areas

§40 said the spreadsheet is the hardest port in the family, because it is
not a tree and its graph is not a graph of things. So it is the seventh
battery: `craftkit-plugin-grid`.

### What was ported

- a **workbook** of **sheets** — nodes, `grid.workbook` and `grid.sheet`;
- the **cells** — the sheet's payload, a sparse map of addresses;
- the **formula** — a small language: numbers, text, booleans, cell
  references (`B3`), **ranges** (`A1:B4`), the arithmetic, the
  comparisons, and `SUM`, `AVERAGE`, `MIN`, `MAX`, `COUNT`;
- the **dependency graph** — the areas a formula reads, and the cells
  whose formulas cover a given cell;
- the **values** — derived, by the evaluator, every time.

### The four things the port found

- **A grid is not a tree, and the cells are in the payload.** A cell has
  no children and no relations, and a sheet has far too many of them to
  be nodes: a million cells is not a tree with a million nodes. So the
  sheet carries them, and the **payload is where the size is**. That is
  a thing a kit must be able to say out loud, because a document with a
  huge payload is a different kind of document from one with a huge tree,
  and a kit that cannot say which is which cannot plan for either.
- **A precedent is an area, not a thing.** `SUM(A1:B4)` reads sixteen
  cells and one range, and "what depends on this cell" is a question
  about *areas*. A kit whose relations are between **nodes** cannot hold
  that graph in its relations — so the graph lives in the payload and the
  plugin owns it, the same bargain the compositing battery made with its
  paths. The graph of a spreadsheet is not the graph of a document.
- **The value of a formula is derived, and the application stores it.**
  GridCraft's cell carries **both** the formula and the cached result,
  because a spreadsheet shows numbers and must show them at once. Here
  the cell carries **only** the formula, and the evaluator computes the
  value every time: a cell's value cannot be wrong, because it is never
  written down. The price is that every value is computed every time,
  and the price is paid knowingly.
- **A cell's address is a name, and the kit's ids are numbers.** `B3` is
  not an id: it is a *name*, and a name in a sheet is a name in a
  **range** — `A1:B4` is sixteen names at once. So the naming is the
  plugin's, exactly as the property paths are the compositing battery's,
  and this is the second time a path has turned out not to be a path of
  ids.

### And the one that is better here

**A cycle is refused when the formula is set.** `=A1+1` in `A1` is
refused at the moment it is typed, and so is a cycle of two, and the
refusal says which cell reads itself. The application finds a cycle
later, in the middle of a recalculation, which is a bad afternoon; the
kit finds it here, which is a keystroke. The check is possible because
the graph is in the payload and the payload is the plugin's.

### The table, with the seventh row

| the port | what it found | what it cost |
|---|---|---|
| the vector application | a node needs a **payload** | a field |
| the sound application | a node may have **no place**; the document needs **relations** | two fields, a version |
| the compositing application | the history needs to **branch** | a tree instead of a stack |
| the PDF application | an edge has to be able to **say** something | a field, a version |
| the video editor | the order and the length of a timeline are **derived** | nothing |
| the compositing application (again) | a **path** is not a path of ids | nothing |
| the artist's IDE | a closed union is **kinds**; a document may carry a thing it does not model | one method |
| the **spreadsheet** | a grid is a **payload**; a precedent is an **area**; a value is **derived** | nothing |

Seven ports. Four of them cost nothing, and the seventh — the one the
survey called the hardest — cost nothing either. What it cost was **the
kit learning to say that a document's size can be in its payload**, and
that a graph of areas is not a graph of nodes.

---

## 43. CADCraft: the tree that is this kit's own subject

CADCraft appeared in the middle of the session: a **2D and 3D CAD and
drafting application in pure Rust**, by the same team, with the same
shape as the others — `doc`, `engine`, `mcp`, `ui-egui`, `geom`,
`io`, and three apps.

It matters more than the other four, because it is the **same domain**
the tree this kit replaces was written for. So it is the closest thing
to a port that already exists, and it is the hardest thing to read.

### What it is

- a **document** that is a **table-heavy database**, not a tree: a
  header, the layers, the linetypes, the text and dimension styles, the
  blocks, the layouts, the views, the constraints, and a flat **entity
  store**;
- an **entity** that is a **closed enum of thirty kinds** — line, point,
  circle, arc, ellipse, polyline, spline, helix, text, mtext, an
  attribute, an insert, a dimension, a leader, a hatch, a solid, a
  region, an image, a table — each with its own struct, its own DXF
  mapping, its own transform, its own grips and its own decomposition
  into primitives;
- an **engine** with the family's command registry: `CommandSpec { id,
  label, menu, shortcut, aliases, params, enabled, run, undoable,
  interactive, transparent }`, about **440 commands**, one registry, one
  `Session::execute(id, &Value)`;
- a **geometry crate** with no commands and no document knowledge:
  `Vec2`, `Vec3`, `Mat3`, splines, intersections, fillets, regions;
- a **constraint solver**: a clean-room parametric solver — residuals,
  damped Gauss–Newton, a degrees-of-freedom analysis — that **rewrites
  the document** with the solved geometry;
- a **format** that is **DXF**, with no native versioned file of its own:
  the version is `$ACADVER`, there is no migration chain, and the
  application's own data rides along in `xdata` under the registered name
  `CADCRAFT`.

### The five things the port found

- **A CAD drawing is tables and a flat store, and the tree is not
  there.** Structure is expressed by *reference by name* (a layer, a
  block, a linetype) and by *handle* (groups, constraints, a dimension
  that is attached to the edges it measures). A kit whose tree is
  containment has to decide what a layer is — and the answer is that a
  layer is a **table**, a drawing's layers are **nodes**, and an
  entity's layer is an **edge that says something** rather than
  containment. That is the same answer the PDF port gave, and it is the
  second time the answer is an edge and not a parent.
- **A closed enum of thirty kinds is exactly what the open kind is
  for.** Every variant needs its own struct, its own DXF mapping, its
  own transform and its own decomposition, and a new kind of entity is a
  change in four places at once. With kinds as open strings, a new
  entity is a new kind, and nothing else moves.
- **The numeric core is a library below the document, and this kit's
  §39 said it would be.** CADCraft's geometry crate holds the numbers
  and knows nothing about drawings; the solver is a *service* that
  rewrites the document. So the core does not have a unit, does not have
  a solver, and does not need one: the numbers live in the library, the
  library is a plugin, and the kit is unchanged. §39's answer was not a
  guess — it was the shape the family had already chosen.
- **The prompt is a door, and a kit whose commands never ask cannot be
  driven through the commands that ask.** CADCraft's registry has
  `interactive: Option<Factory>`: some commands *ask*. Its MCP server
  does not hide them behind a façade of JSON parameters — one of its
  fifteen tools is `command_line {text}`, which types at the prompt
  exactly as a person would, and another is `script {text}`, which runs
  a `.scr` file. So an agent can drive the interactive commands *by
  answering the prompt*. This kit's rule is the opposite: a command
  takes its parameters and never asks, and only a front end's own entry
  point may put a question to a person. The finding is that the rule is
  **a choice, not a law**, and the family's choice buys something real.
- **The extension story is a script language, not a plugin.** CADCraft
  has a full **AutoLISP** interpreter — `defun c:NAME` makes a LISP
  function into a *command*, and `entget`/`entmake`/`entmod` reach the
  entities — plus `.scr` scripts and `.pgp` aliases, all routed through
  the same registry. The plugin system is in the gap list, not in the
  tree: *"E4 | Plug-in API (owner decision: Rust/WASM plug-ins rather
  than ObjectARX/.NET) | 30–50"*. So the family's answer to "how does
  somebody add to this?" is **scripting**, and this kit's answer is
  **linking** — and both are answers to the same question.

### And the three rules the tree already keeps

Three things in CADCraft are the same rules this kit keeps, arrived at
independently, and they are worth naming because they are evidence:

- **Round-trip by preservation.** What the application does not model is
  kept as raw file groups — `Foreign` for the drawing, `xdata` for the
  entity, `EntityKind::Unknown` for the object — and written back
  unchanged. That is §333's "the reader keeps what it does not
  understand", and it is the rule the artist's port needed for its opaque
  3D scene. Three trees, three implementations, one rule.
- **Hostile input is refused, not clamped.** Every parser has a limit
  with a number (`MAX_TAGS`, `MAX_OBJECTS`, a capped reader, a bounded
  MCP reply), and a bad JSON count is **refused** rather than clamped —
  the last two commits are exactly that. This kit's `Params` refuses a
  hostile number for the same reason.
- **Copy-on-write, so the undo is cheap.** The entity store is chunked
  and shared: an undo snapshot copies pointers, not entities. This kit's
  history **clones** the document, which is simple and correct and more
  expensive than it needs to be. This is the one place where the
  family's tree is plainly ahead, and it is a change to the kit's
  history, not to its seams.

### What CADCraft would get from this kit

The survey's own list, and every item is a seam this kit has and that
tree does not:

1. a **node-kind and payload model**, replacing thirty hand-written enum
   variants and their four places each;
2. a **real extension registry** — the registry is a fixed list of
   fifty-one `specs()` calls today, and the formats are a fixed enum;
3. **typed parameter schemas** — `params` is a free-text string that is
   parsed by hand;
4. a **versioned native format with pending migrations**, instead of
   `xdata` and raw-group preservation as the way to keep what the
   format cannot say;
5. **relations as data** — groups, dimension associativity and
   constraints each invented their own reference mechanism;
6. a **plan**, so that rendering, export and an agent's query all read
   the same derived thing;
7. **plugin-contributed commands, kinds, evaluators, panels and layers**;
8. and **one id namespace**, separate from the compatibility label —
   CADCraft's ids are AutoCAD command names, which is a parity metric
   and not a stable name for a kit.

### The table, with the eighth row

| the port | what it found | what it cost |
|---|---|---|
| the vector application | a node needs a **payload** | a field |
| the sound application | a node may have **no place**; the document needs **relations** | two fields, a version |
| the compositing application | the history needs to **branch** | a tree instead of a stack |
| the PDF application | an edge has to be able to **say** something | a field, a version |
| the video editor | the order and the length of a timeline are **derived** | nothing |
| the compositing application (again) | a **path** is not a path of ids | nothing |
| the artist's IDE | a closed union is **kinds**; a document may carry a thing it does not model | one method |
| the **spreadsheet** | a grid is a **payload**; a precedent is an **area** | nothing |
| the **CAD application** | a drawing is **tables and a flat store**; a layer is an **edge**; the prompt is a **door** | nothing yet |

Eight ports. Five of them cost nothing, and the two hardest — the
spreadsheet and the CAD application — cost nothing either. What the
survey of the family changed is not the kit's shape: it is the
**confidence** in the shape, which is now backed by five independent
implementations and a survey of every one of them.

---

## 44. The eighth battery: a CAD drawing, tables and a flat store

§43 surveyed CADCraft, the family's CAD application. A drawing is the
domain this kit was written for, so it is the eighth battery:
`craftkit-plugin-cad`.

### What was ported

| the drawing | v2 |
|---|---|
| the drawing | a **node** of the kind `cad.drawing` |
| a layer | a **node** of the kind `cad.layer` — a table row |
| a block | a **node** of the kind `cad.block` — a table row that **contains** entities |
| an entity | a **node** of an **open kind**: `cad.entity.line`, `.circle`, `.arc`, `.text`, `.insert` |
| an entity's layer | a **relation**: an edge that says something |
| a block insert | a **node** that **references** a block, by an edge |
| the geometry | the entity's **payload**, in the world's units |
| the extents | **derived**, by the evaluator, every time |

### The three things the port found

- **A drawing is tables and a flat store, and the tree is not there.**
  Structure in a drawing is *reference by name* — a layer, a block, a
  linetype — and by *handle*: a group, a constraint, a dimension that is
  attached to the edges it measures. So a layer is a **table**, and an
  entity's layer is an **edge that says something** rather than a
  parent: the entity is in a block *and* it is on a layer, and those
  are two different facts. This is the **second** time a port has
  answered "what is this relationship?" with an edge — the PDF port
  said it about a reference — and the answer is now a pattern rather
  than a coincidence.
- **A closed union of thirty kinds is exactly what the open kind is
  for.** In the application every entity kind is a variant with its own
  struct, its own file mapping, its own transform, its own grips and
  its own decomposition into primitives, and a new kind of entity is a
  change in four places at once. Here a kind is an **open string**: a
  new entity is a new kind, and nothing else moves. The third port to
  find this, after the artist's unions.
- **The numbers belong to a library, and the core has no unit.** The
  geometry is in the payload, in the world's own units, and the
  evaluation that does arithmetic reaches for a **library** — not for a
  unit in the core, and not for a solver in the core. That is §39's
  answer confirmed by the tree it was a prediction about: the numeric
  core arrives as a plugin, and the document does not change.

### And the one the port does not settle

The **prompt**. CADCraft's commands may *ask*: its registry has
`interactive: Option<Factory>`, and its MCP server exposes the prompt
itself as a tool, so an agent answers the questions an interactive
command asks. This kit's rule is the opposite — a command takes its
parameters and never asks, and only a front end's own entry point may
put a question to a person.

The rule is a **choice**, and the survey is evidence that it costs
something: a kit whose commands never ask cannot be driven through the
commands that ask. What it would cost to change is a real question,
and it is **not** answered here. The shape of the change would be: an
outcome that says "I need this", and a session that can be handed the
answer — which is the shape CADCraft's `command_line` tool has, and the
shape the outer tree's shell has (§118). Whether the kit should have
one is a decision for the owner, and it is written down here rather
than decided here.

### The table, with the eighth row

| the port | what it found | what it cost |
|---|---|---|
| the vector application | a node needs a **payload** | a field |
| the sound application | a node may have **no place**; the document needs **relations** | two fields, a version |
| the compositing application | the history needs to **branch** | a tree instead of a stack |
| the PDF application | an edge has to be able to **say** something | a field, a version |
| the video editor | the order and the length of a timeline are **derived** | nothing |
| the compositing application (again) | a **path** is not a path of ids | nothing |
| the artist's IDE | a closed union is **kinds**; a document may carry a thing it does not model | one method |
| the **spreadsheet** | a grid is a **payload**; a precedent is an **area** | nothing |
| the **CAD application** | a drawing is **tables and a flat store**; a layer is an **edge**; the prompt is a **door** | nothing |

Eight ports. Six of them cost nothing. The two that did — the artist's
unions and the branches — bought the kit's two strangest features, and
everything the family has taught it since has cost **nothing**.
